use crate::{model::*, DatabaseStatus, ProbeDatabase, ProbeEntry, ProbeInput};
use rusqlite::{
    backup::{Backup, StepResult},
    Connection, OpenFlags, TransactionBehavior,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const SCHEMA_VERSION: i32 = 5;
const APP_ID: i32 = 0x41585030;
pub const LEGACY_SCHEMA: &str = "CREATE TABLE probe_entries(id INTEGER PRIMARY KEY,code TEXT NOT NULL CHECK(length(code) BETWEEN 1 AND 128),note TEXT NOT NULL CHECK(length(note)<=240),created_at TEXT NOT NULL DEFAULT(strftime('%Y-%m-%dT%H:%M:%fZ','now'))) STRICT; PRAGMA application_id=1096306736; PRAGMA user_version=1;";
const MIGRATIONS: [&str; 4] = [
    include_str!("../migrations/002_base.sql"),
    include_str!("../migrations/003_xml.sql"),
    include_str!("../migrations/004_lots.sql"),
    include_str!("../migrations/005_inventory.sql"),
];

pub struct Store {
    pub(crate) db: ProbeDatabase,
    path: PathBuf,
    backup_dir: PathBuf,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if rusqlite::version_number() < 3_054_000 {
            return Err(CoreError::new(
                "SQLITE_VERSION",
                "SQLite sem a versão mínima corrigida.",
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| CoreError::invalid("Caminho do banco inválido."))?;
        fs::create_dir_all(parent)?;
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        let version = identity(&connection)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        integrity(&connection)?;
        let backup_dir = parent.join("backups");
        if version < SCHEMA_VERSION {
            if version > 0 {
                fs::create_dir_all(&backup_dir)?;
                let backup = unique_file(&backup_dir, &format!("antes-schema-{version}"))?;
                backup_to(&connection, &backup)?;
                // Ensaio nas mesmas migrações, antes de tocar no esquema principal.
                let rehearsal = unique_file(&backup_dir, "ensaio-migracao")?;
                backup_to(&connection, &rehearsal)?;
                let rehearsal_result = (|| {
                    let mut copy = Connection::open(&rehearsal)?;
                    copy.pragma_update(None, "foreign_keys", true)?;
                    apply_migrations(&mut copy)?;
                    integrity(&copy)
                })();
                // Somente o arquivo temporário que acabamos de criar é removido.
                let _ = fs::remove_file(&rehearsal);
                rehearsal_result?;
            }
            apply_migrations(&mut connection)?;
        }
        let mode: String = connection.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        if mode != "wal" {
            return Err(CoreError::new("WAL", "O banco local não habilitou WAL."));
        }
        integrity(&connection)?;
        Ok(Self {
            db: ProbeDatabase { connection },
            path: path.into(),
            backup_dir,
        })
    }
    pub fn schema_version(&self) -> i32 {
        SCHEMA_VERSION
    }
    pub fn database_path(&self) -> &Path {
        &self.path
    }
    pub fn probe_status(&self) -> Result<DatabaseStatus> {
        self.db
            .status()
            .map_err(|e| CoreError::new("DATABASE", e.to_string()))
    }
    pub fn probe_entries(&self) -> Result<Vec<ProbeEntry>> {
        self.db
            .recent_entries()
            .map_err(|e| CoreError::new("DATABASE", e.to_string()))
    }
    pub fn record_probe(&mut self, input: ProbeInput) -> Result<ProbeEntry> {
        self.db
            .record(input)
            .map_err(|e| CoreError::invalid(e.to_string()))
    }
    pub fn create_backup(&self) -> Result<String> {
        fs::create_dir_all(&self.backup_dir)?;
        let path = unique_file(&self.backup_dir, "manual")?;
        backup_to(&self.db.connection, &path)?;
        Ok(path.to_string_lossy().into_owned())
    }
    pub fn integrity_check(&self) -> Result<()> {
        integrity(&self.db.connection)
    }
    pub fn settings(&self) -> Result<Settings> {
        Ok(self.db.connection.query_row(
            "SELECT short_days,long_days FROM app_settings WHERE id=1",
            [],
            |r| {
                Ok(Settings {
                    short_days: r.get(0)?,
                    long_days: r.get(1)?,
                })
            },
        )?)
    }
    pub fn save_settings(&mut self, input: Settings) -> Result<Settings> {
        if input.short_days < 0 || input.long_days < input.short_days || input.long_days > 3650 {
            return Err(CoreError::invalid("Configure alertas entre 0 e 3650 dias; o segundo prazo deve ser maior ou igual ao primeiro."));
        }
        self.db.connection.execute(
            "UPDATE app_settings SET short_days=?1,long_days=?2 WHERE id=1",
            [input.short_days, input.long_days],
        )?;
        self.settings()
    }
    pub fn workspace(&self) -> Result<Workspace> {
        let today = local_date();
        let snapshot = self.db.connection.unchecked_transaction()?;
        let workspace = Workspace {
            skus: self.skus()?,
            suppliers: self.suppliers()?,
            receipts: self.receipts()?,
            lots: self.lots(&today)?,
            inventories: self.inventories()?,
            settings: self.settings()?,
            local_date: today,
            schema_version: SCHEMA_VERSION,
            sqlite_version: rusqlite::version().into(),
        };
        snapshot.commit()?;
        Ok(workspace)
    }
}

fn identity(c: &Connection) -> Result<i32> {
    let version: i32 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let app: i32 = c.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let objects: i64 = c.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
        [],
        |r| r.get(0),
    )?;
    if app == 0 && version == 0 && objects == 0 {
        return Ok(0);
    }
    if app != APP_ID || !(1..=SCHEMA_VERSION).contains(&version) {
        return Err(CoreError::new(
            "INCOMPATIBLE_DATABASE",
            "Banco incompatível ou criado por uma versão mais recente. Não foi alterado.",
        ));
    }
    Ok(version)
}
pub fn integrity(c: &Connection) -> Result<()> {
    let mut q = c.prepare("PRAGMA integrity_check")?;
    let rows = q
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if rows != ["ok"] {
        return Err(CoreError::new(
            "INTEGRITY",
            "A verificação de integridade do banco falhou. Preserve o arquivo e o backup.",
        ));
    }
    let mut fk = c.prepare("PRAGMA foreign_key_check")?;
    if fk.query([])?.next()?.is_some() {
        return Err(CoreError::new(
            "INTEGRITY",
            "O banco contém referências inconsistentes.",
        ));
    }
    Ok(())
}
fn apply_migrations(c: &mut Connection) -> Result<()> {
    let version = identity(c)?;
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if version == 0 {
        tx.execute_batch(LEGACY_SCHEMA)?;
    }
    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let target = index as i32 + 2;
        if target > version {
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", target)?;
        }
    }
    integrity(&tx)?;
    tx.commit()?;
    Ok(())
}
fn unique_file(dir: &Path, label: &str) -> Result<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CoreError::invalid("Relógio do Windows inválido."))?
        .as_nanos();
    let path = dir.join(format!("{label}-{stamp}.db"));
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    Ok(path)
}
pub fn backup_to(source: &Connection, path: &Path) -> Result<()> {
    let mut destination = Connection::open(path)?;
    {
        let backup = Backup::new(source, &mut destination)?;
        let start = Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(120) {
                return Err(CoreError::new(
                    "BACKUP",
                    "Tempo limite ao criar backup. Nenhuma migração foi executada.",
                ));
            }
            match backup.step(128)? {
                StepResult::Done => break,
                StepResult::Busy | StepResult::Locked => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => {}
            }
        }
    }
    destination.pragma_update(None, "journal_mode", "DELETE")?;
    integrity(&destination)?;
    drop(destination);
    let verified = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    integrity(&verified)
}
