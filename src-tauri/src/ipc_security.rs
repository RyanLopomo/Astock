use axis_estoque_core::{CoreError, Result};
use serde_json::Value;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

pub fn trusted_location(url: &tauri::Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    matches!(
        (url.scheme(), url.host_str(), url.port()),
        ("http", Some("tauri.localhost"), None) | ("tauri", Some("localhost"), None)
    ) || cfg!(debug_assertions)
        && matches!(
            (url.scheme(), url.host_str(), url.port()),
            ("http", Some("127.0.0.1"), Some(1420))
        )
}

fn budget(value: &Value, remaining: &mut usize, depth: usize) -> Result<()> {
    if depth > 12 {
        return Err(CoreError::invalid(
            "Estrutura IPC excede a profundidade permitida.",
        ));
    }
    let size = match value {
        Value::String(s) => s.len() + 8,
        _ => 8,
    };
    *remaining = remaining
        .checked_sub(size)
        .ok_or_else(|| CoreError::invalid("A solicitação excede o limite de tamanho."))?;
    match value {
        Value::Array(items) => {
            if items.len() > 10_000 {
                return Err(CoreError::invalid("Lista IPC muito extensa."));
            }
            for item in items {
                budget(item, remaining, depth + 1)?;
            }
        }
        Value::Object(fields) => {
            if fields.len() > 64 {
                return Err(CoreError::invalid("Campos IPC em excesso."));
            }
            for (key, item) in fields {
                *remaining = remaining
                    .checked_sub(key.len())
                    .ok_or_else(|| CoreError::invalid("Solicitação muito extensa."))?;
                budget(item, remaining, depth + 1)?;
            }
        }
        _ => (),
    }
    Ok(())
}

pub fn validate(label: &str, url: &tauri::Url, command: &str, body: &Value) -> Result<()> {
    if label != "main" || !trusted_location(url) || !crate::commands::COMMANDS.contains(&command) {
        return Err(CoreError::new(
            "IPC_DENIED",
            "Comando ou origem não autorizados.",
        ));
    }
    let fields = body
        .as_object()
        .ok_or_else(|| CoreError::invalid("Argumentos IPC devem ser um objeto."))?;
    let allowed: &[&str] = match command {
        "get_workspace" | "get_probe_state" | "create_backup" => &[],
        "get_receipt" | "get_inventory" => &["id"],
        "get_inventory_events" => &["id", "page"],
        _ => &["input"],
    };
    if fields.len() != allowed.len() || fields.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(CoreError::invalid(
            "Argumentos IPC ausentes ou não reconhecidos.",
        ));
    }
    let mut remaining = if command == "import_xml" {
        axis_estoque_core::xml::MAX_XML_BYTES + 16 * 1024
    } else {
        256 * 1024
    };
    budget(body, &mut remaining, 0)?;
    for key in ["id", "page"] {
        if let Some(value) = fields.get(key) {
            let n = value
                .as_i64()
                .ok_or_else(|| CoreError::invalid("Identificador ou página inválidos."))?;
            if (key == "id" && n <= 0) || (key == "page" && !(0..=1_000_000).contains(&n)) {
                return Err(CoreError::invalid(
                    "Identificador ou página fora do limite.",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
pub enum Work {
    Normal,
    Xml,
    Backup,
}
#[derive(Default)]
struct Counts {
    xml: bool,
    backup: bool,
    last_backup: Option<Instant>,
}
#[derive(Default)]
pub struct WorkGate {
    pending: AtomicUsize,
    counts: Mutex<Counts>,
}
pub struct Permit {
    gate: Arc<WorkGate>,
    kind: Work,
    success: bool,
}
impl WorkGate {
    pub fn enter(self: &Arc<Self>, kind: Work) -> Result<Permit> {
        let mut counts = self
            .counts
            .lock()
            .map_err(|_| CoreError::new("BUSY", "Controle de operações indisponível."))?;
        if kind == Work::Xml && counts.xml
            || kind == Work::Backup && counts.backup
            || self.pending.load(Ordering::SeqCst) >= 32
        {
            return Err(CoreError::new(
                "BUSY",
                "Há operações locais em andamento. Aguarde a conclusão e tente novamente.",
            ));
        }
        if kind == Work::Backup
            && counts
                .last_backup
                .is_some_and(|time| time.elapsed() < Duration::from_secs(60))
        {
            return Err(CoreError::new(
                "RATE_LIMITED",
                "Um backup foi criado há menos de um minuto. Aguarde antes de criar outro.",
            ));
        }
        self.pending.fetch_add(1, Ordering::SeqCst);
        if kind == Work::Xml {
            counts.xml = true;
        }
        if kind == Work::Backup {
            counts.backup = true;
        }
        Ok(Permit {
            gate: self.clone(),
            kind,
            success: false,
        })
    }
}
impl Permit {
    pub fn succeeded(&mut self) {
        self.success = true;
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        if let Ok(mut counts) = self.gate.counts.lock() {
            if self.kind == Work::Xml {
                counts.xml = false;
            }
            if self.kind == Work::Backup {
                counts.backup = false;
                if self.success {
                    counts.last_backup = Some(Instant::now());
                }
            }
            self.gate.pending.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_origins_commands_and_paths() {
        let url = tauri::Url::parse("http://tauri.localhost/").unwrap();
        for (label, command, body) in [
            ("other", "get_workspace", json!({})),
            ("main", "execute_sql", json!({"sql":"SELECT 1"})),
            ("main", "create_backup", json!({"path":"C:/outside.db"})),
            ("main", "get_receipt", json!({"id":-1})),
        ] {
            assert!(validate(label, &url, command, &body).is_err());
        }
        assert!(validate("main", &url, "get_workspace", &json!({})).is_ok());
        for u in [
            "https://evil.invalid/",
            "file:///C:/Windows/win.ini",
            "javascript:alert(1)",
            "http://tauri.localhost.evil.invalid/",
            "http://tauri.localhost:9999/",
            "http://user:pass@tauri.localhost/",
        ] {
            assert!(!trusted_location(&tauri::Url::parse(u).unwrap()));
        }
    }
    #[test]
    fn bounds_bodies() {
        let u = tauri::Url::parse("http://tauri.localhost/").unwrap();
        assert!(validate(
            "main",
            &u,
            "save_product",
            &json!({"input":{"description":"x".repeat(256*1024)}})
        )
        .is_err());
        assert!(validate(
            "main",
            &u,
            "create_inventory",
            &json!({"input":{"skuIds":vec![1;10_001]}})
        )
        .is_err());
        let mut deep = json!({});
        for _ in 0..14 {
            deep = json!({"x":deep});
        }
        assert!(validate("main", &u, "save_product", &json!({"input":deep})).is_err());
    }
    #[test]
    fn bounded_queue_and_heavy_operations() {
        let g = Arc::new(WorkGate::default());
        let permits: Vec<_> = (0..32).map(|_| g.enter(Work::Normal).unwrap()).collect();
        assert!(g.enter(Work::Normal).is_err());
        drop(permits);
        assert!(g.enter(Work::Normal).is_ok());
        let xml = g.enter(Work::Xml).unwrap();
        assert!(g.enter(Work::Xml).is_err());
        drop(xml);
        assert!(g.enter(Work::Xml).is_ok());
        let mut backup = g.enter(Work::Backup).unwrap();
        assert!(g.enter(Work::Backup).is_err());
        backup.succeeded();
        drop(backup);
        assert_eq!(g.enter(Work::Backup).err().unwrap().code, "RATE_LIMITED");
        assert!(g.enter(Work::Normal).is_ok());
    }
    #[test]
    fn failures_release_permits_without_backup_cooldown() {
        let g = Arc::new(WorkGate::default());
        drop(g.enter(Work::Backup).unwrap());
        assert!(g.enter(Work::Backup).is_ok());
    }
}
