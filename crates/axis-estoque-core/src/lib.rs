//! Núcleo integrado de estoque; mantém a API legada da Fase 0 para compatibilidade.
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{fmt, path::Path, time::Duration};

mod catalog;
pub mod decimal;
mod inventory;
mod lots;
pub mod model;
mod receipts;
pub mod security;
mod stock;
pub mod storage;
pub mod xml;
pub use model::*;
pub use storage::Store;

const APPLICATION_ID: i32 = 0x41585030; // AXP0: identidade preservada do banco original.
const SCHEMA_VERSION: i32 = 1;

#[derive(Debug)]
pub enum ProbeError {
    InvalidInput(&'static str),
    UnsafeSqlite,
    UnsupportedDatabase,
    Sqlite(rusqlite::Error),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => f.write_str(message),
            Self::UnsafeSqlite => f.write_str("SQLite sem a versão de integridade exigida."),
            Self::UnsupportedDatabase => f.write_str("Banco incompatível com a prova da Fase 0."),
            Self::Sqlite(_) => f.write_str("Não foi possível concluir a operação no banco local."),
        }
    }
}

impl std::error::Error for ProbeError {}

impl From<rusqlite::Error> for ProbeError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeInput {
    pub code: String,
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeEntry {
    pub id: i64,
    pub code: String,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub sqlite_version: String,
    pub schema_version: i32,
    pub journal_mode: String,
    pub foreign_keys: bool,
    pub synchronous: i32,
    pub busy_timeout_ms: i64,
    pub integrity: String,
    pub total_entries: i64,
}

pub struct ProbeDatabase {
    pub(crate) connection: Connection,
}

impl ProbeDatabase {
    pub fn open(path: &Path) -> Result<Self, ProbeError> {
        security::validate_path(path)
            .map_err(|_| ProbeError::InvalidInput("Caminho local inseguro."))?;
        // Exigir a linha incorporada 3.54+; nunca habilitar WAL em versões antigas.
        if rusqlite::version_number() < 3_054_000 {
            return Err(ProbeError::UnsafeSqlite);
        }
        let mut connection = Connection::open(path)?;
        security::configure_connection(&connection)
            .map_err(|_| ProbeError::InvalidInput("Não foi possível proteger a conexão."))?;
        connection.busy_timeout(Duration::from_secs(5))?;
        // Recusar bancos estranhos/mais novos antes de qualquer PRAGMA persistente.
        check_database_identity(&connection)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        let journal: String = connection.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        if !journal.eq_ignore_ascii_case("wal") {
            return Err(ProbeError::UnsupportedDatabase);
        }
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_database_identity(&tx)?;
        let version: i32 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version == 0 {
            // Inicialização atômica de banco vazio, sem migração de dados existentes.
            tx.execute_batch(
                "CREATE TABLE probe_entries (
                    id INTEGER PRIMARY KEY,
                    code TEXT NOT NULL CHECK(length(code) BETWEEN 1 AND 128),
                    note TEXT NOT NULL CHECK(length(note) <= 240),
                    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
                ) STRICT;",
            )?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)?;
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        tx.commit()?;
        Ok(Self { connection })
    }

    pub fn status(&self) -> Result<DatabaseStatus, ProbeError> {
        let c = &self.connection;
        Ok(DatabaseStatus {
            sqlite_version: rusqlite::version().to_owned(),
            schema_version: c.query_row("PRAGMA user_version", [], |r| r.get(0))?,
            journal_mode: c.query_row("PRAGMA journal_mode", [], |r| r.get(0))?,
            foreign_keys: c.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i32>(0))? == 1,
            synchronous: c.query_row("PRAGMA synchronous", [], |r| r.get(0))?,
            busy_timeout_ms: c.query_row("PRAGMA busy_timeout", [], |r| r.get(0))?,
            integrity: c.query_row("PRAGMA quick_check", [], |r| r.get(0))?,
            total_entries: c.query_row("SELECT COUNT(*) FROM probe_entries", [], |r| r.get(0))?,
        })
    }

    pub fn recent_entries(&self) -> Result<Vec<ProbeEntry>, ProbeError> {
        let mut statement = self.connection.prepare(
            "SELECT id, code, note, created_at FROM probe_entries ORDER BY id DESC LIMIT 20",
        )?;
        let entries = statement.query_map([], entry_from_row)?;
        Ok(entries.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn record(&mut self, input: ProbeInput) -> Result<ProbeEntry, ProbeError> {
        // Sem conversão numérica: códigos são texto, inclusive seus zeros iniciais.
        let code = input.code.trim();
        if code.is_empty() || code.chars().count() > 128 || code.chars().any(char::is_control) {
            return Err(ProbeError::InvalidInput(
                "Informe um código de 1 a 128 caracteres, sem caracteres de controle.",
            ));
        }
        let note = input.note.trim();
        if note.chars().count() > 240 || note.chars().any(char::is_control) {
            return Err(ProbeError::InvalidInput(
                "A observação aceita até 240 caracteres, sem caracteres de controle.",
            ));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO probe_entries (code, note) VALUES (?1, ?2)",
            params![code, note],
        )?;
        let entry = tx.query_row(
            "SELECT id, code, note, created_at FROM probe_entries WHERE id = ?1",
            [tx.last_insert_rowid()],
            entry_from_row,
        )?;
        // Nenhum sucesso é retornado antes do commit. Drop da transação faz rollback.
        tx.commit()?;
        Ok(entry)
    }
}

fn entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProbeEntry> {
    Ok(ProbeEntry {
        id: row.get(0)?,
        code: row.get(1)?,
        note: row.get(2)?,
        created_at: row.get(3)?,
    })
}

fn check_database_identity(connection: &Connection) -> Result<(), ProbeError> {
    let application: i32 = connection.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: i32 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if application == APPLICATION_ID && version == SCHEMA_VERSION {
        return Ok(());
    }
    let objects: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
        [],
        |r| r.get(0),
    )?;
    if application == 0 && version == 0 && objects == 0 {
        return Ok(());
    }
    Err(ProbeError::UnsupportedDatabase)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(code: &str) -> ProbeInput {
        ProbeInput {
            code: code.into(),
            note: "Teste técnico".into(),
        }
    }

    #[test]
    fn persists_text_and_configures_durability_on_every_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prova.db");
        {
            let mut db = ProbeDatabase::open(&path).unwrap();
            let entry = db.record(input("0012345678905")).unwrap();
            assert_eq!(entry.code, "0012345678905");
            assert!(entry.created_at.ends_with('Z'));
        }
        let db = ProbeDatabase::open(&path).unwrap();
        let status = db.status().unwrap();
        assert_eq!(status.journal_mode, "wal");
        assert!(status.foreign_keys);
        assert_eq!(status.synchronous, 2);
        assert_eq!(status.busy_timeout_ms, 5000);
        assert_eq!(status.integrity, "ok");
        assert_eq!(db.recent_entries().unwrap()[0].code, "0012345678905");
    }

    #[test]
    fn validates_input_and_uses_bound_parameters() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = ProbeDatabase::open(&dir.path().join("prova.db")).unwrap();
        for code in ["", "   ", "ab\ncd", "\0", &"x".repeat(129)] {
            assert!(matches!(
                db.record(input(code)),
                Err(ProbeError::InvalidInput(_))
            ));
        }
        let mut long_note = input("123");
        long_note.note = "á".repeat(241);
        assert!(matches!(
            db.record(long_note),
            Err(ProbeError::InvalidInput(_))
        ));
        assert_eq!(db.status().unwrap().total_entries, 0);
        let sql = "'; DROP TABLE probe_entries; --";
        assert_eq!(db.record(input(sql)).unwrap().code, sql);
        assert_eq!(db.status().unwrap().total_entries, 1);
    }

    #[test]
    fn rolls_back_insert_if_a_trigger_fails_after_writing() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = ProbeDatabase::open(&dir.path().join("prova.db")).unwrap();
        db.record(input("antes")).unwrap();
        db.connection
            .execute_batch(
                "CREATE TRIGGER simulate_failure AFTER INSERT ON probe_entries
             WHEN NEW.code = 'falhar' BEGIN SELECT RAISE(ABORT, 'falha injetada'); END;",
            )
            .unwrap();
        assert!(db.record(input("falhar")).is_err());
        assert_eq!(db.status().unwrap().total_entries, 1);
        assert_eq!(db.status().unwrap().integrity, "ok");
        assert_eq!(db.record(input("depois")).unwrap().code, "depois");
    }

    #[test]
    fn refuses_foreign_and_newer_databases_without_changing_journal() {
        for sql in [
            "CREATE TABLE foreign_data (id INTEGER);",
            "PRAGMA application_id = 1096306736; PRAGMA user_version = 99;",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("incompativel.db");
            let original = Connection::open(&path).unwrap();
            original.execute_batch(sql).unwrap();
            drop(original);
            assert!(matches!(
                ProbeDatabase::open(&path),
                Err(ProbeError::UnsupportedDatabase)
            ));
            let original = Connection::open(&path).unwrap();
            let mode: String = original
                .query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .unwrap();
            assert_eq!(mode, "delete");
        }
    }

    #[test]
    fn recent_entries_are_limited_and_allow_repeated_scans() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = ProbeDatabase::open(&dir.path().join("prova.db")).unwrap();
        for _ in 0..25 {
            db.record(input("00000123")).unwrap();
        }
        let entries = db.recent_entries().unwrap();
        assert_eq!(entries.len(), 20);
        assert_eq!(entries[0].id, 25);
        assert_eq!(db.status().unwrap().total_entries, 25);
    }
}
