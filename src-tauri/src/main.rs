#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use axis_estoque_core::*;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Manager, State};

enum DatabaseState {
    Starting,
    Ready(Box<Store>),
    Failed(CoreError),
}
struct AppState {
    database: Arc<Mutex<DatabaseState>>,
    path: String,
    profile: Option<String>,
}
async fn database<T: Send + 'static>(
    state: Arc<Mutex<DatabaseState>>,
    action: impl FnOnce(&mut Store) -> Result<T> + Send + 'static,
) -> Result<T> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = state.lock().map_err(|_| {
            CoreError::new("UNAVAILABLE", "Banco indisponível. Reinicie o aplicativo.")
        })?;
        match &mut *guard {
            DatabaseState::Starting => Err(CoreError::new(
                "STARTING",
                "Preparando banco e verificando migrações…",
            )),
            DatabaseState::Ready(store) => action(store),
            DatabaseState::Failed(error) => Err(CoreError::new(error.code, error.message.clone())),
        }
    })
    .await
    .map_err(|_| CoreError::new("UNAVAILABLE", "Não foi possível executar a operação local."))?
}
macro_rules! input_command {
    ($name:ident,$input:ty,$output:ty) => {
        #[tauri::command]
        async fn $name(input: $input, state: State<'_, AppState>) -> Result<$output> {
            database(state.database.clone(), move |db| db.$name(input)).await
        }
    };
}
input_command!(save_product, ProductInput, Sku);
input_command!(save_supplier, SupplierInput, Supplier);
input_command!(create_receipt, ReceiptCreate, ReceiptDetail);
input_command!(save_receipt_item, ReceiptItemInput, ReceiptDetail);
input_command!(delete_receipt_item, DeleteReceiptItem, ReceiptDetail);
input_command!(confirm_receipt, ReceiptAction, Confirmation);
input_command!(cancel_receipt, ReceiptAction, ());
input_command!(import_xml, XmlImportInput, XmlImportResult);
input_command!(plan_issue, IssuePlanInput, IssuePlan);
input_command!(issue_stock, IssueInput, ());
input_command!(adjust_stock, AdjustmentInput, ());
input_command!(regularize_lot, RegularizeInput, ());
input_command!(create_inventory, InventoryCreate, InventoryDetail);
input_command!(inventory_scan, InventoryScan, ScanResult);
input_command!(correct_count, CountCorrection, InventoryDetail);
input_command!(finish_inventory, InventoryFinish, Confirmation);
input_command!(cancel_inventory, InventoryCancel, ());
input_command!(save_settings, Settings, Settings);
input_command!(record_probe, ProbeInput, ProbeEntry);
#[tauri::command]
async fn get_workspace(state: State<'_, AppState>) -> Result<WorkspaceResponse> {
    let path = state.path.clone();
    let profile = state.profile.clone();
    database(state.database.clone(), move |db| {
        Ok(WorkspaceResponse {
            workspace: db.workspace()?,
            database_path: path,
            data_profile: profile,
        })
    })
    .await
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceResponse {
    #[serde(flatten)]
    workspace: Workspace,
    database_path: String,
    data_profile: Option<String>,
}
#[tauri::command]
async fn get_receipt(id: i64, state: State<'_, AppState>) -> Result<ReceiptDetail> {
    database(state.database.clone(), move |db| db.receipt(id)).await
}
#[tauri::command]
async fn get_inventory(id: i64, state: State<'_, AppState>) -> Result<InventoryDetail> {
    database(state.database.clone(), move |db| db.inventory(id)).await
}
#[tauri::command]
async fn get_inventory_events(
    id: i64,
    page: i64,
    state: State<'_, AppState>,
) -> Result<CountEventPage> {
    database(state.database.clone(), move |db| {
        db.inventory_events(id, page)
    })
    .await
}
#[tauri::command]
async fn get_movements(input: MovementQuery, state: State<'_, AppState>) -> Result<MovementPage> {
    database(state.database.clone(), move |db| db.movements(input)).await
}
#[tauri::command]
async fn create_backup(state: State<'_, AppState>) -> Result<String> {
    database(state.database.clone(), |db| db.create_backup()).await
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbeState {
    app_version: &'static str,
    database_path: String,
    database: DatabaseStatus,
    entries: Vec<ProbeEntry>,
}
#[tauri::command]
async fn get_probe_state(state: State<'_, AppState>) -> Result<ProbeState> {
    let path = state.path.clone();
    database(state.database.clone(), move |db| {
        Ok(ProbeState {
            app_version: env!("CARGO_PKG_VERSION"),
            database_path: path,
            database: db.probe_status()?,
            entries: db.probe_entries()?,
        })
    })
    .await
}
fn database_location() -> Result<(PathBuf, Option<String>)> {
    #[cfg(debug_assertions)]
    if let Ok(profile) = std::env::var("AXIS_ESTOQUE_TEST_PROFILE") {
        if profile.is_empty()
            || profile.len() > 40
            || !profile
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(CoreError::new("PATH", "Perfil de homologação inválido."));
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.cache/test-data");
        return Ok((root.join(&profile).join("estoque.db"), Some(profile)));
    }
    let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        CoreError::new(
            "PATH",
            "LOCALAPPDATA indisponível. Ambiente Windows necessário.",
        )
    })?;
    let root = PathBuf::from(local);
    if !root.is_absolute() {
        return Err(CoreError::new("PATH", "LOCALAPPDATA deve ser absoluto."));
    }
    Ok((
        root.join("AxisEstoque").join("phase0").join("prova.db"),
        None,
    ))
}
fn main() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let (path, profile) = database_location()?;
            let shared = Arc::new(Mutex::new(DatabaseState::Starting));
            app.manage(AppState {
                database: shared.clone(),
                path: path.to_string_lossy().into_owned(),
                profile,
            });
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(|| Store::open(&path)).unwrap_or_else(|_| {
                    Err(CoreError::new(
                        "DATABASE_INIT",
                        "Falha ao preparar o banco. Preserve os arquivos e verifique o backup.",
                    ))
                });
                if let Ok(mut state) = shared.lock() {
                    *state = match result {
                        Ok(store) => DatabaseState::Ready(Box::new(store)),
                        Err(error) => DatabaseState::Failed(error),
                    };
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_workspace,
            get_receipt,
            get_inventory,
            get_inventory_events,
            get_movements,
            create_backup,
            save_product,
            save_supplier,
            create_receipt,
            save_receipt_item,
            delete_receipt_item,
            confirm_receipt,
            cancel_receipt,
            import_xml,
            plan_issue,
            issue_stock,
            adjust_stock,
            regularize_lot,
            create_inventory,
            inventory_scan,
            correct_count,
            finish_inventory,
            cancel_inventory,
            save_settings,
            get_probe_state,
            record_probe
        ])
        .run(tauri::generate_context!());
    if result.is_err() {
        eprintln!("AXIS: falha ao iniciar o aplicativo local.");
        std::process::exit(1);
    }
}
