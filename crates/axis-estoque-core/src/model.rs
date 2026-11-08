use serde::{Deserialize, Serialize};
use std::fmt;

pub type Result<T, E = CoreError> = std::result::Result<T, E>;
pub const MAX_QUANTITY: i64 = 1_000_000_000;
pub const MAX_CENTS: i64 = 9_000_000_000_000;

#[derive(Debug, Serialize)]
pub struct CoreError {
    pub code: &'static str,
    pub message: String,
}
impl CoreError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("VALIDATION", message)
    }
}
impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CoreError {}
impl From<rusqlite::Error> for CoreError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(_, Some(ref message)) = e {
            if message.contains("skus.internal_code") {
                return Self::invalid("O código interno do SKU já está em uso.");
            }
            if message.contains("suppliers.tax_id") {
                return Self::invalid("Já existe um fornecedor com esse documento.");
            }
            if message.starts_with("AXIS:") {
                return Self::invalid(message.trim_start_matches("AXIS:"));
            }
        }
        Self::new("DATABASE", "Não foi possível concluir a operação no banco local. Nenhuma alteração parcial foi confirmada.")
    }
}
impl From<std::io::Error> for CoreError {
    fn from(_: std::io::Error) -> Self {
        Self::new(
            "FILESYSTEM",
            "Não foi possível acessar o arquivo local. Verifique espaço e permissões.",
        )
    }
}
impl From<serde_json::Error> for CoreError {
    fn from(_: serde_json::Error) -> Self {
        Self::invalid("Dados locais em formato inválido.")
    }
}

macro_rules! data {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}
data!(ProductInput { product_id: Option<i64>, sku_id: Option<i64>, name: String, description: String, brand: String, category: String, internal_code: String, presentation: String, purchase_cents: i64, sale_cents: i64, default_pack: i64, expiry_control: bool, active: bool, product_active: bool, barcodes: Vec<String> });
data!(Sku { id: i64, product_id: i64, name: String, description: String, brand: String, category: String, internal_code: String, presentation: String, purchase_cents: i64, sale_cents: i64, default_pack: i64, expiry_control: bool, active: bool, product_active: bool, barcodes: Vec<String>, balance: i64, unidentified_balance: i64 });
data!(SupplierInput { id: Option<i64>, name: String, tax_id: String, contact: String, active: bool });
data!(Supplier {
    id: i64,
    name: String,
    tax_id: String,
    contact: String,
    active: bool
});
data!(ReceiptCreate { supplier_id: Option<i64>, notes: String });
data!(LotDraft { label: String, manufactured_on: Option<String>, expires_on: Option<String>, vintage: Option<i32>, quantity: i64 });
data!(XmlLot { label: String, manufactured_on: Option<String>, expires_on: Option<String>, quantity: String });
data!(ReceiptItemInput { receipt_id: i64, expected_revision: i64, item_id: Option<i64>, sku_id: Option<i64>, quantity: String, units_per_pack: i64, commercial_unit: String, unit_price: String, total_cents: i64, lots: Vec<LotDraft>, remember_mapping: bool });
data!(ReceiptItem { id: i64, sku_id: Option<i64>, supplier_code: Option<String>, description: String, barcode: Option<String>, tax_barcode: Option<String>, quantity: String, units_per_pack: i64, units: Option<i64>, commercial_unit: String, unit_price: String, total_cents: i64, unit_cost_cents: Option<i64>, rounding_difference_cents: Option<i64>, tax_quantity: Option<String>, tax_unit: Option<String>, source_quantity: Option<String>, source_unit_price: Option<String>, source_total_cents: Option<i64>, source_adjustment_cents: Option<i64>, source_lots: Vec<XmlLot>, lots: Vec<LotDraft>, candidates: Vec<i64> });
data!(ReceiptSummary { id: i64, supplier_id: Option<i64>, supplier_name: String, status: String, source: String, invoice_number: Option<String>, invoice_series: Option<String>, issued_at: Option<String>, access_key: Option<String>, notes: String, revision: i64, created_at: String, total_cents: i64 });
data!(ReceiptDetail { receipt: ReceiptSummary, items: Vec<ReceiptItem> });
data!(ReceiptAction {
    id: i64,
    expected_revision: i64,
    confirmed: bool
});
data!(DeleteReceiptItem {
    receipt_id: i64,
    item_id: i64,
    expected_revision: i64
});
data!(Confirmation {
    id: i64,
    already_completed: bool
});
data!(XmlImportInput {
    content: String,
    file_name: String
});
data!(XmlImportResult { detail: ReceiptDetail, reused: bool, warnings: Vec<String> });
data!(Lot { id: i64, sku_id: i64, sku_name: String, internal_code: String, category: String, label: String, manufactured_on: Option<String>, expires_on: Option<String>, vintage: Option<i32>, supplier_id: Option<i64>, supplier_name: String, receipt_id: Option<i64>, received_quantity: i64, balance: i64, unit_cost_cents: i64, alert: String });
data!(LotQuantity { lot_id: Option<i64>, quantity: i64 });
data!(IssuePlanInput {
    sku_id: i64,
    quantity: i64,
    include_undated: bool,
    allow_expired: bool
});
data!(IssuePlan { sku_id: i64, quantity: i64, allocations: Vec<LotQuantity>, warnings: Vec<String> });
data!(IssueInput { request_key: String, sku_id: i64, allocations: Vec<LotQuantity>, reason: String, allow_expired: bool, confirmed: bool });
data!(AdjustmentInput { request_key: String, sku_id: i64, lot_id: Option<i64>, quantity: i64, reason: String, reference: String, confirmed: bool });
data!(RegularizeInput {
    request_key: String,
    sku_id: i64,
    lot: LotDraft,
    reason: String,
    confirmed: bool
});
data!(Movement { id: i64, sku_id: i64, internal_code: String, sku_name: String, lot_id: Option<i64>, lot_label: String, quantity: i64, kind: String, origin_type: String, origin_id: String, reason: String, unit_cost_cents: i64, created_at: String });
data!(MovementQuery { search: String, sku_id: Option<i64>, from_date: Option<String>, to_date: Option<String>, page: i64 });
data!(MovementPage { items: Vec<Movement>, total: i64, page: i64 });
data!(Settings {
    short_days: i64,
    long_days: i64
});
data!(InventoryCreate { name: String, sku_ids: Option<Vec<i64>> });
data!(InventorySummary { id: i64, name: String, status: String, scope: String, revision: i64, created_at: String, closed_at: Option<String>, uncounted_decision: Option<String>, sku_count: i64 });
data!(InventoryLine { id: i64, sku_id: i64, sku_name: String, internal_code: String, lot_id: Option<i64>, lot_label: String, expires_on: Option<String>, reference: i64, counted: Option<i64>, difference: Option<i64> });
data!(CountEvent { id: i64, code: String, sku_id: Option<i64>, lot_id: Option<i64>, quantity: i64, mode: String, outcome: String, created_at: String, previous_count: Option<i64>, new_count: Option<i64> });
data!(CountEventPage { items: Vec<CountEvent>, total: i64, page: i64 });
data!(InventoryDetail { inventory: InventorySummary, lines: Vec<InventoryLine>, events: Vec<CountEvent>, events_total: i64 });
data!(InventoryScan { inventory_id: i64, request_key: String, code: String, quantity: i64, sku_id: Option<i64>, lot_id: Option<i64>, lot_selected: bool });
data!(ScanResult { outcome: String, message: String, candidates: Vec<i64>, lot_choices: Vec<InventoryLine>, detail: InventoryDetail });
data!(CountCorrection {
    inventory_id: i64,
    line_id: i64,
    request_key: String,
    quantity: i64,
    expected_revision: i64
});
data!(InventoryFinish { id: i64, expected_revision: i64, uncounted_decision: Option<String>, confirmed: bool });
data!(InventoryCancel {
    id: i64,
    confirmed: bool
});
data!(Workspace { skus: Vec<Sku>, suppliers: Vec<Supplier>, receipts: Vec<ReceiptSummary>, lots: Vec<Lot>, inventories: Vec<InventorySummary>, settings: Settings, local_date: String, schema_version: i32, sqlite_version: String });

pub(crate) fn text(value: &str, name: &str, max: usize, required: bool) -> Result<String> {
    let v = value.trim();
    if (required && v.is_empty()) || v.chars().count() > max || v.chars().any(char::is_control) {
        return Err(CoreError::invalid(format!(
            "{name}: informe até {max} caracteres válidos."
        )));
    }
    Ok(v.into())
}
pub(crate) fn quantity(q: i64, positive: bool) -> Result<()> {
    if q.abs_diff(0) > MAX_QUANTITY as u64 || (positive && q <= 0) {
        return Err(CoreError::invalid("Quantidade fora do limite permitido."));
    }
    Ok(())
}
pub(crate) fn cents(c: i64) -> Result<()> {
    if !(0..=MAX_CENTS).contains(&c) {
        return Err(CoreError::invalid(
            "Valor monetário fora do limite permitido.",
        ));
    }
    Ok(())
}
pub(crate) fn local_date() -> String {
    chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}
pub(crate) fn date(value: &str) -> Result<chrono::NaiveDate> {
    let d = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| CoreError::invalid("Data inválida; use AAAA-MM-DD."))?;
    if d.format("%Y-%m-%d").to_string() != value {
        return Err(CoreError::invalid("Data inválida."));
    }
    Ok(d)
}
