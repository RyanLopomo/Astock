use axis_estoque_core::{
    decimal::Decimal,
    storage::{Store, LEGACY_SCHEMA},
    xml::{parse_purchase_xml, validate_access_key},
    *,
};
use rusqlite::Connection;
use std::path::PathBuf;

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    db: Store,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("estoque.db");
        let db = Store::open(&path).unwrap();
        Self {
            _dir: dir,
            path,
            db,
        }
    }
}
fn product(code: &str) -> ProductInput {
    ProductInput {
        product_id: None,
        sku_id: None,
        name: format!("Produto {code}"),
        description: "Teste sintético".into(),
        brand: "Teste".into(),
        category: "Bebidas".into(),
        internal_code: code.into(),
        presentation: "Unidade".into(),
        purchase_cents: 100,
        sale_cents: 250,
        default_pack: 1,
        expiry_control: false,
        active: true,
        product_active: true,
        barcodes: vec![format!("00{code}")],
    }
}
fn add(db: &mut Store, sku: i64, q: i64, key: &str) {
    db.adjust_stock(AdjustmentInput {
        request_key: key.into(),
        sku_id: sku,
        lot_id: None,
        quantity: q,
        reason: "Estoque de teste".into(),
        reference: String::new(),
        confirmed: true,
    })
    .unwrap();
}
fn receive(db: &mut Store, sku: i64, q: &str, pack: i64, lots: Vec<LotDraft>) -> i64 {
    let r = db
        .create_receipt(ReceiptCreate {
            supplier_id: None,
            notes: "Teste".into(),
        })
        .unwrap();
    let r = db
        .save_receipt_item(ReceiptItemInput {
            receipt_id: r.receipt.id,
            expected_revision: r.receipt.revision,
            item_id: None,
            sku_id: Some(sku),
            quantity: q.into(),
            units_per_pack: pack,
            commercial_unit: "CX".into(),
            unit_price: "1.00".into(),
            total_cents: 100,
            lots,
            remember_mapping: false,
        })
        .unwrap();
    db.confirm_receipt(ReceiptAction {
        id: r.receipt.id,
        expected_revision: r.receipt.revision,
        confirmed: true,
    })
    .unwrap();
    r.receipt.id
}
fn lot(label: &str, q: i64, expiry: Option<String>) -> LotDraft {
    LotDraft {
        label: label.into(),
        quantity: q,
        manufactured_on: None,
        expires_on: expiry,
        vintage: None,
    }
}
fn start(db: &mut Store, ids: Option<Vec<i64>>) -> InventoryDetail {
    db.create_inventory(InventoryCreate {
        name: "Contagem de teste".into(),
        sku_ids: ids,
    })
    .unwrap()
}
fn scan(db: &mut Store, inv: i64, code: &str, q: i64, key: &str) -> ScanResult {
    db.inventory_scan(InventoryScan {
        inventory_id: inv,
        request_key: key.into(),
        code: code.into(),
        quantity: q,
        sku_id: None,
        lot_id: None,
        lot_selected: false,
    })
    .unwrap()
}
fn finish(db: &mut Store, inv: i64, decision: Option<&str>) -> Result<Confirmation> {
    let d = db.inventory(inv)?;
    db.finish_inventory(InventoryFinish {
        id: inv,
        expected_revision: d.inventory.revision,
        uncounted_decision: decision.map(str::to_string),
        confirmed: true,
    })
}
fn key(number: u32) -> String {
    let base = format!("3526111234567800019555001{number:09}112345678");
    assert_eq!(base.len(), 43);
    let mut weight = 2;
    let mut sum = 0;
    for b in base.bytes().rev() {
        sum += (b - b'0') as i32 * weight;
        weight = if weight == 9 { 2 } else { weight + 1 };
    }
    let r = sum % 11;
    format!("{base}{}", if r < 2 { 0 } else { 11 - r })
}
fn xml(
    number: u32,
    code: &str,
    barcode: &str,
    q: &str,
    price: &str,
    total: &str,
    rastro: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><NFe xmlns="http://www.portalfiscal.inf.br/nfe"><infNFe Id="NFe{}" versao="4.00"><ide><cUF>35</cUF><mod>55</mod><serie>1</serie><nNF>{number}</nNF><dhEmi>2026-11-07T12:00:00-03:00</dhEmi></ide><emit><CNPJ>12345678000195</CNPJ><xNome>Fornecedor Sintético &amp; Teste</xNome></emit><det nItem="1"><prod><cProd>{code}</cProd><cEAN>{barcode}</cEAN><xProd>Produto Sintético</xProd><uCom>CX</uCom><qCom>{q}</qCom><vUnCom>{price}</vUnCom><vProd>{total}</vProd><cEANTrib>SEM GTIN</cEANTrib><uTrib>UN</uTrib><qTrib>999.000</qTrib>{rastro}</prod></det></infNFe></NFe>"#,
        key(number)
    )
}

#[test]
fn a_catalog_unique_codes_shared_barcodes_and_inactivation() {
    let mut f = Fixture::new();
    let mut a = product("A");
    a.barcodes = vec!["00123456".into()];
    let s = f.db.save_product(a.clone()).unwrap();
    assert!(f.db.save_product(a).is_err());
    let mut b = product("B");
    b.product_id = Some(s.product_id);
    b.barcodes = vec!["00123456".into()];
    f.db.save_product(b).unwrap();
    assert_eq!(f.db.resolve_code("00123456").unwrap().len(), 2);
    let mut edit = product("A");
    edit.product_id = Some(s.product_id);
    edit.sku_id = Some(s.id);
    edit.active = false;
    edit.barcodes = vec!["00123456".into()];
    f.db.save_product(edit).unwrap();
    assert_eq!(f.db.resolve_code("00123456").unwrap().len(), 1);
    assert_eq!(f.db.skus().unwrap().len(), 2);
    f.db.integrity_check().unwrap();
}
#[test]
fn a_manual_receipt_conversion_duplicate_and_sales_price() {
    let mut f = Fixture::new();
    let sku = f.db.save_product(product("A")).unwrap();
    let id = receive(&mut f.db, sku.id, "5", 12, vec![]);
    assert_eq!(f.db.skus().unwrap()[0].balance, 60);
    assert_eq!(f.db.skus().unwrap()[0].sale_cents, 250);
    assert!(
        f.db.confirm_receipt(ReceiptAction {
            id,
            expected_revision: 0,
            confirmed: true
        })
        .unwrap()
        .already_completed
    );
    assert_eq!(f.db.skus().unwrap()[0].balance, 60);
    assert!(f
        .db
        .save_receipt_item(ReceiptItemInput {
            receipt_id: id,
            expected_revision: 0,
            item_id: None,
            sku_id: Some(sku.id),
            quantity: "1".into(),
            units_per_pack: 1,
            commercial_unit: "UN".into(),
            unit_price: "1".into(),
            total_cents: 100,
            lots: vec![],
            remember_mapping: false
        })
        .is_err());
}
#[test]
fn a_negative_stock_immutable_movements_and_idempotent_adjustment() {
    let mut f = Fixture::new();
    let sku = f.db.save_product(product("A")).unwrap();
    add(&mut f.db, sku.id, 5, "initial");
    add(&mut f.db, sku.id, 5, "initial");
    assert_eq!(f.db.skus().unwrap()[0].balance, 5);
    assert!(f
        .db
        .adjust_stock(AdjustmentInput {
            request_key: "negative".into(),
            sku_id: sku.id,
            lot_id: None,
            quantity: -6,
            reason: "Teste".into(),
            reference: "".into(),
            confirmed: true
        })
        .is_err());
    let c = Connection::open(&f.path).unwrap();
    assert!(c
        .execute("UPDATE stock_movements SET quantity=99", [])
        .is_err());
    assert!(c.execute("DELETE FROM stock_movements", []).is_err());
}
#[test]
fn a_legacy_migration_preserves_probe_and_unidentified_stock_with_verified_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let c = Connection::open(&path).unwrap();
    c.pragma_update(None, "journal_mode", "WAL").unwrap();
    c.execute_batch(LEGACY_SCHEMA).unwrap();
    c.execute(
        "INSERT INTO probe_entries(code,note) VALUES('00001234','Preservar')",
        [],
    )
    .unwrap();
    c.execute_batch(include_str!("../migrations/002_base.sql"))
        .unwrap();
    c.execute_batch(include_str!("../migrations/003_xml.sql"))
        .unwrap();
    c.pragma_update(None, "user_version", 3).unwrap();
    c.execute_batch("INSERT INTO products(id,name) VALUES(1,'Anterior'); INSERT INTO skus(id,product_id,internal_code,purchase_cents,sale_cents) VALUES(1,1,'ANT',100,200); INSERT INTO stock_movements(sku_id,quantity,kind,origin_type,origin_id,origin_line,reason,unit_cost_cents) VALUES(1,7,'adjustment','adjustment','legacy',0,'Anterior',100);").unwrap();
    let db = Store::open(&path).unwrap();
    assert_eq!(db.probe_entries().unwrap()[0].code, "00001234");
    assert_eq!(db.skus().unwrap()[0].unidentified_balance, 7);
    assert!(db.lots("2026-11-07").unwrap().is_empty());
    let files = std::fs::read_dir(dir.path().join("backups"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1);
    let backup = Connection::open(&files[0]).unwrap();
    assert_eq!(
        backup
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        backup
            .query_row("SELECT code FROM probe_entries", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "00001234"
    );
}
#[test]
fn a_failed_migration_does_not_change_original() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.db");
    let c = Connection::open(&path).unwrap();
    c.execute_batch(LEGACY_SCHEMA).unwrap();
    c.execute_batch("CREATE TABLE products(incompatible TEXT); INSERT INTO probe_entries(code,note) VALUES('0001','keep');").unwrap();
    drop(c);
    assert!(Store::open(&path).is_err());
    let c = Connection::open(&path).unwrap();
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM probe_entries", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn a_future_schema_rejected_before_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    let c = Connection::open(&path).unwrap();
    c.execute_batch("PRAGMA application_id=1096306736; PRAGMA user_version=99;")
        .unwrap();
    drop(c);
    assert!(Store::open(&path).is_err());
    let c = Connection::open(&path).unwrap();
    assert_eq!(
        c.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "delete"
    );
}
#[test]
fn a_receipt_failure_rolls_back_all_items_and_lots() {
    let mut f = Fixture::new();
    let sku = f.db.save_product(product("A")).unwrap();
    let r =
        f.db.create_receipt(ReceiptCreate {
            supplier_id: None,
            notes: "".into(),
        })
        .unwrap();
    let mut r = r;
    for qty in [1, 2] {
        r =
            f.db.save_receipt_item(ReceiptItemInput {
                receipt_id: r.receipt.id,
                expected_revision: r.receipt.revision,
                item_id: None,
                sku_id: Some(sku.id),
                quantity: qty.to_string(),
                units_per_pack: 1,
                commercial_unit: "UN".into(),
                unit_price: "1".into(),
                total_cents: 100,
                lots: vec![lot(&format!("L{qty}"), qty, None)],
                remember_mapping: false,
            })
            .unwrap();
    }
    let c = Connection::open(&f.path).unwrap();
    c.execute_batch("CREATE TRIGGER inject_receipt_failure AFTER INSERT ON stock_movements WHEN NEW.quantity=2 BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(f
        .db
        .confirm_receipt(ReceiptAction {
            id: r.receipt.id,
            expected_revision: r.receipt.revision,
            confirmed: true
        })
        .is_err());
    assert_eq!(f.db.skus().unwrap()[0].balance, 0);
    assert!(f.db.lots("2026-11-07").unwrap().is_empty());
    assert_eq!(f.db.receipt(r.receipt.id).unwrap().receipt.status, "draft");
}
#[test]
fn a_backup_online_matches_live_wal() {
    let mut f = Fixture::new();
    let sku = f.db.save_product(product("A")).unwrap();
    add(&mut f.db, sku.id, 9, "b");
    let backup = f.db.create_backup().unwrap();
    let c = Connection::open(backup).unwrap();
    assert_eq!(
        c.query_row("SELECT sum(quantity) FROM stock_movements", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        9
    );
    assert_eq!(
        c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}
#[test]
fn b_xml_valid_wrappers_and_namespace_prefixes() {
    let content = xml(1, "001", "00123456", "5.0000", "12.1234567890", "60.62", "");
    let n = parse_purchase_xml(&content).unwrap();
    assert_eq!(n.supplier_name, "Fornecedor Sintético & Teste");
    assert_eq!(n.items[0].source_total_cents, 6062);
    assert_eq!(n.items[0].tax_quantity.as_deref(), Some("999.000"));
    let inner = content.split_once("?>").unwrap().1;
    let wrapper =
        format!("<nfeProc xmlns=\"http://www.portalfiscal.inf.br/nfe\">{inner}</nfeProc>");
    assert!(parse_purchase_xml(&wrapper).is_ok());
    assert!(parse_purchase_xml(&wrapper.replace("nfeProc", "procNFe")).is_ok());
    let prefixed = inner
        .replace("<NFe xmlns=", "<n:NFe xmlns:n=")
        .replace("</NFe>", "</n:NFe>")
        .replace("<infNFe ", "<n:infNFe ")
        .replace("</infNFe>", "</n:infNFe>");
    assert!(
        parse_purchase_xml(&prefixed).is_err(),
        "filhos sem namespace nao devem ser aceitos por nome local"
    );
}
#[test]
fn b_xml_invalid_unsupported_key_and_entities() {
    for content in [
        "<x>",
        "<root/>",
        "<NFe/>",
        "<!DOCTYPE a [<!ENTITY a SYSTEM 'file:///secret'>]><a>&a;</a>",
    ] {
        assert!(parse_purchase_xml(content).is_err());
    }
    assert!(validate_access_key("123").is_err());
    let mut wrong = key(1);
    let last = wrong.pop().unwrap();
    wrong.push(if last == '0' { '1' } else { '0' });
    assert!(validate_access_key(&wrong).is_err());
    assert!(parse_purchase_xml(
        &xml(1, "A", "SEM GTIN", "1", "1", "1", "").replace("4.00", "3.10")
    )
    .is_err());
    assert!(parse_purchase_xml(
        &xml(1, "A", "SEM GTIN", "1", "1", "1", "").replace("Produto Sintético", "&unknown;")
    )
    .is_err());
    assert!(parse_purchase_xml(&" ".repeat(axis_estoque_core::xml::MAX_XML_BYTES + 1)).is_err());
}
#[test]
fn b_xml_unknown_duplicate_and_persistent_supplier_mapping() {
    let mut f = Fixture::new();
    let sku = f.db.save_product(product("A")).unwrap();
    let input = XmlImportInput {
        content: xml(1, "001", "SEM GTIN", "5", "12", "60", ""),
        file_name: "teste.xml".into(),
    };
    let imported = f.db.import_xml(input.clone()).unwrap();
    assert!(imported.detail.items[0].sku_id.is_none());
    assert_eq!(f.db.skus().unwrap()[0].balance, 0);
    let id = imported.detail.receipt.id;
    assert!(f
        .db
        .confirm_receipt(ReceiptAction {
            id,
            expected_revision: 0,
            confirmed: true
        })
        .is_err());
    assert!(f.db.import_xml(input).unwrap().reused);
    assert_eq!(f.db.receipts().unwrap().len(), 1);
    let i = &imported.detail.items[0];
    let r =
        f.db.save_receipt_item(ReceiptItemInput {
            receipt_id: id,
            expected_revision: 0,
            item_id: Some(i.id),
            sku_id: Some(sku.id),
            quantity: i.quantity.clone(),
            units_per_pack: 12,
            commercial_unit: i.commercial_unit.clone(),
            unit_price: i.unit_price.clone(),
            total_cents: i.total_cents,
            lots: vec![],
            remember_mapping: true,
        })
        .unwrap();
    f.db.confirm_receipt(ReceiptAction {
        id,
        expected_revision: r.receipt.revision,
        confirmed: true,
    })
    .unwrap();
    drop(f.db);
    f.db = Store::open(&f.path).unwrap();
    let next =
        f.db.import_xml(XmlImportInput {
            content: xml(2, "001", "SEM GTIN", "1", "12", "12", ""),
            file_name: "outro.xml".into(),
        })
        .unwrap();
    assert_eq!(next.detail.items[0].sku_id, Some(sku.id));
    assert_eq!(next.detail.items[0].units, Some(12));
    assert_eq!(f.db.skus().unwrap()[0].balance, 60);
}
#[test]
fn b_decimal_rounding_and_fractional_conversion_are_exact() {
    assert_eq!(Decimal::parse("1.005").unwrap().cents().unwrap(), 101);
    assert_eq!(Decimal::parse("0.5").unwrap().units(12).unwrap(), 6);
    assert!(Decimal::parse("0.5").unwrap().units(1).is_err());
    assert!(Decimal::parse("1e3").is_err());
    assert_eq!(
        axis_estoque_core::decimal::unit_cost(100, 3).unwrap(),
        (33, -1)
    );
    assert!(Decimal::parse("99999999999999999999999999999999")
        .unwrap()
        .product_cents(Decimal::parse("99999999999999999999999999999999").unwrap())
        .is_err());
    let mut f = Fixture::new();
    let d =
        f.db.import_xml(XmlImportInput {
            content: xml(1, "F", "SEM GTIN", "0.5", "2", "1", ""),
            file_name: "fracao.xml".into(),
        })
        .unwrap();
    assert_eq!(d.detail.items[0].units, None);
    assert_eq!(d.detail.items[0].unit_cost_cents, None);
}
#[test]
fn c_lot_distribution_must_match_and_no_fake_dates() {
    let mut f = Fixture::new();
    let sku = f.db.save_product(product("A")).unwrap();
    let r =
        f.db.create_receipt(ReceiptCreate {
            supplier_id: None,
            notes: "".into(),
        })
        .unwrap();
    assert!(f
        .db
        .save_receipt_item(ReceiptItemInput {
            receipt_id: r.receipt.id,
            expected_revision: 0,
            item_id: None,
            sku_id: Some(sku.id),
            quantity: "5".into(),
            units_per_pack: 1,
            commercial_unit: "UN".into(),
            unit_price: "1".into(),
            total_cents: 100,
            lots: vec![lot("L", 4, None)],
            remember_mapping: false
        })
        .is_err());
    receive(
        &mut f.db,
        sku.id,
        "5",
        1,
        vec![lot("A", 2, None), lot("B", 3, None)],
    );
    let lots = f.db.lots("2026-11-07").unwrap();
    assert_eq!(lots.len(), 2);
    assert_eq!(lots.iter().map(|l| l.balance).sum::<i64>(), 5);
    assert!(lots
        .iter()
        .all(|l| l.expires_on.is_none() && l.alert == "undated"));
}
#[test]
fn c_fefo_and_explicit_undated_and_negative_lot_guards() {
    let mut f = Fixture::new();
    let mut p = product("A");
    p.expiry_control = true;
    let sku = f.db.save_product(p).unwrap();
    let today = chrono::Local::now().date_naive();
    receive(
        &mut f.db,
        sku.id,
        "7",
        1,
        vec![
            lot(
                "Later",
                3,
                Some((today + chrono::Days::new(30)).to_string()),
            ),
            lot("First", 2, Some((today + chrono::Days::new(7)).to_string())),
            lot("Unknown", 2, None),
        ],
    );
    let plan =
        f.db.plan_issue(IssuePlanInput {
            sku_id: sku.id,
            quantity: 4,
            include_undated: false,
            allow_expired: false,
        })
        .unwrap();
    let lots = f.db.lots(&today.to_string()).unwrap();
    let first = lots.iter().find(|l| l.label == "First").unwrap();
    assert_eq!(plan.allocations[0].lot_id, Some(first.id));
    assert_eq!(plan.allocations[0].quantity, 2);
    f.db.issue_stock(IssueInput {
        request_key: "out".into(),
        sku_id: sku.id,
        allocations: plan.allocations,
        reason: "Teste FEFO".into(),
        allow_expired: false,
        confirmed: true,
    })
    .unwrap();
    assert!(f
        .db
        .plan_issue(IssuePlanInput {
            sku_id: sku.id,
            quantity: 3,
            include_undated: false,
            allow_expired: false
        })
        .is_err());
    assert!(f
        .db
        .plan_issue(IssuePlanInput {
            sku_id: sku.id,
            quantity: 3,
            include_undated: true,
            allow_expired: false
        })
        .is_ok());
    assert!(f
        .db
        .issue_stock(IssueInput {
            request_key: "bad".into(),
            sku_id: sku.id,
            allocations: vec![LotQuantity {
                lot_id: Some(first.id),
                quantity: 1
            }],
            reason: "Teste".into(),
            allow_expired: true,
            confirmed: true
        })
        .is_err());
    assert_eq!(f.db.skus().unwrap()[0].balance, 3);
}
#[test]
fn c_alert_boundaries_and_regularization_preserve_total() {
    // A classificação usa datas civis, sem converter meia-noite para UTC.
    let settings = Settings {
        short_days: 7,
        long_days: 30,
    };
    for (d, expected) in [
        ("2026-11-06", "expired"),
        ("2026-11-07", "short"),
        ("2026-11-14", "short"),
        ("2026-11-15", "long"),
        ("2026-12-07", "long"),
        ("2026-12-08", "safe"),
    ] {
        let mut f = Fixture::new();
        let s = f.db.save_product(product("A")).unwrap();
        receive(&mut f.db, s.id, "1", 1, vec![lot("L", 1, Some(d.into()))]);
        assert_eq!(f.db.lots("2026-11-07").unwrap()[0].alert, expected);
    }
    assert_eq!(settings.short_days, 7);
    let mut f = Fixture::new();
    let s = f.db.save_product(product("A")).unwrap();
    add(&mut f.db, s.id, 10, "old");
    f.db.regularize_lot(RegularizeInput {
        request_key: "reg".into(),
        sku_id: s.id,
        lot: lot("Identificado", 4, None),
        reason: "Conferência física".into(),
        confirmed: true,
    })
    .unwrap();
    let s = &f.db.skus().unwrap()[0];
    assert_eq!((s.balance, s.unidentified_balance), (10, 6));
    assert_eq!(f.db.lots("2026-11-07").unwrap()[0].balance, 4);
}
#[test]
fn d_inventory_complete_partial_unknown_ambiguous_and_manual_quantity() {
    let mut f = Fixture::new();
    let a = f.db.save_product(product("A")).unwrap();
    let b = f.db.save_product(product("B")).unwrap();
    add(&mut f.db, a.id, 10, "a");
    add(&mut f.db, b.id, 10, "b");
    let inv = start(&mut f.db, Some(vec![a.id]));
    assert_eq!(inv.lines.len(), 1);
    assert!(f
        .db
        .create_inventory(InventoryCreate {
            name: "Outro".into(),
            sku_ids: None
        })
        .is_err());
    assert_eq!(
        scan(
            &mut f.db,
            inv.inventory.id,
            "https://exemplo.test/qr",
            1,
            "unknown"
        )
        .outcome,
        "unknown"
    );
    let result = scan(&mut f.db, inv.inventory.id, "00A", 1, "s1");
    assert_eq!(result.detail.lines[0].counted, Some(1));
    scan(&mut f.db, inv.inventory.id, "00A", 1, "s2");
    scan(&mut f.db, inv.inventory.id, "00A", 12, "s3");
    let repeat = scan(&mut f.db, inv.inventory.id, "00A", 12, "s3");
    assert_eq!(repeat.detail.lines[0].counted, Some(14));
    assert_eq!(
        f.db.skus()
            .unwrap()
            .iter()
            .find(|s| s.id == a.id)
            .unwrap()
            .balance,
        10
    );
    finish(&mut f.db, inv.inventory.id, None).unwrap();
    assert_eq!(
        f.db.skus()
            .unwrap()
            .iter()
            .find(|s| s.id == a.id)
            .unwrap()
            .balance,
        14
    );
    assert_eq!(
        f.db.skus()
            .unwrap()
            .iter()
            .find(|s| s.id == b.id)
            .unwrap()
            .balance,
        10
    );
    let full = start(&mut f.db, None);
    assert_eq!(full.inventory.sku_count, 2);
    f.db.cancel_inventory(InventoryCancel {
        id: full.inventory.id,
        confirmed: true,
    })
    .unwrap();
    let mut shared = product("C");
    shared.barcodes = vec!["00A".into()];
    let c = f.db.save_product(shared).unwrap();
    let i = start(&mut f.db, None);
    let r = scan(&mut f.db, i.inventory.id, "00A", 1, "amb");
    assert_eq!(r.outcome, "ambiguous");
    assert_eq!(r.candidates.len(), 2);
    assert_eq!(
        f.db.inventory_scan(InventoryScan {
            inventory_id: i.inventory.id,
            request_key: "choice".into(),
            code: "00A".into(),
            quantity: 1,
            sku_id: Some(c.id),
            lot_id: None,
            lot_selected: false
        })
        .unwrap()
        .outcome,
        "counted"
    );
}
#[test]
fn d_uncounted_is_not_zero_without_explicit_decision() {
    let mut f = Fixture::new();
    let a = f.db.save_product(product("A")).unwrap();
    add(&mut f.db, a.id, 8, "a");
    let i = start(&mut f.db, None);
    assert!(finish(&mut f.db, i.inventory.id, None).is_err());
    finish(&mut f.db, i.inventory.id, Some("skip")).unwrap();
    assert_eq!(f.db.skus().unwrap()[0].balance, 8);
    let i = start(&mut f.db, None);
    finish(&mut f.db, i.inventory.id, Some("zero")).unwrap();
    assert_eq!(f.db.skus().unwrap()[0].balance, 0);
    assert!(
        finish(&mut f.db, i.inventory.id, Some("zero"))
            .unwrap()
            .already_completed
    );
}
#[test]
fn d_blocked_movements_resume_after_cancel_and_survive_restart() {
    let mut f = Fixture::new();
    let s = f.db.save_product(product("A")).unwrap();
    add(&mut f.db, s.id, 5, "start");
    let i = start(&mut f.db, None);
    scan(&mut f.db, i.inventory.id, "00A", 2, "scan");
    let adjustment = AdjustmentInput {
        request_key: "blocked".into(),
        sku_id: s.id,
        lot_id: None,
        quantity: 1,
        reason: "Teste".into(),
        reference: "".into(),
        confirmed: true,
    };
    assert!(f.db.adjust_stock(adjustment.clone()).is_err());
    let c = Connection::open(&f.path).unwrap();
    assert!(c.execute("INSERT INTO stock_movements(sku_id,quantity,kind,origin_type,origin_id,origin_line,reason,unit_cost_cents) VALUES(?1,1,'adjustment','adjustment','bypass',0,'Teste',0)",[s.id]).is_err());
    drop(c);
    drop(f.db);
    f.db = Store::open(&f.path).unwrap();
    let d = f.db.inventory(i.inventory.id).unwrap();
    assert_eq!(d.inventory.status, "active");
    assert_eq!(d.lines[0].counted, Some(2));
    assert!(f.db.adjust_stock(adjustment.clone()).is_err());
    f.db.cancel_inventory(InventoryCancel {
        id: i.inventory.id,
        confirmed: true,
    })
    .unwrap();
    f.db.adjust_stock(adjustment).unwrap();
    assert_eq!(f.db.skus().unwrap()[0].balance, 6);
}
#[test]
fn d_finalize_failure_rollback_and_stale_revision() {
    let mut f = Fixture::new();
    let a = f.db.save_product(product("A")).unwrap();
    let b = f.db.save_product(product("B")).unwrap();
    add(&mut f.db, a.id, 5, "a");
    add(&mut f.db, b.id, 5, "b");
    let i = start(&mut f.db, None);
    scan(&mut f.db, i.inventory.id, "00A", 7, "a-count");
    scan(&mut f.db, i.inventory.id, "00B", 8, "b-count");
    assert!(f
        .db
        .finish_inventory(InventoryFinish {
            id: i.inventory.id,
            expected_revision: 0,
            uncounted_decision: None,
            confirmed: true
        })
        .is_err());
    let c = Connection::open(&f.path).unwrap();
    c.execute_batch(&format!("CREATE TRIGGER fail_close AFTER INSERT ON stock_movements WHEN NEW.kind='inventory' AND NEW.sku_id={} BEGIN SELECT RAISE(ABORT,'injected'); END;",b.id)).unwrap();
    assert!(finish(&mut f.db, i.inventory.id, None).is_err());
    assert!(f.db.skus().unwrap().iter().all(|s| s.balance == 5));
    assert_eq!(
        f.db.inventory(i.inventory.id).unwrap().inventory.status,
        "active"
    );
    c.execute_batch("DROP TRIGGER fail_close;").unwrap();
    finish(&mut f.db, i.inventory.id, None).unwrap();
    assert_eq!(
        f.db.skus().unwrap().iter().map(|s| s.balance).sum::<i64>(),
        15
    );
}
#[test]
fn d_lot_inventory_choice_correction_and_no_difference() {
    let mut f = Fixture::new();
    let mut p = product("A");
    p.expiry_control = true;
    let s = f.db.save_product(p).unwrap();
    receive(&mut f.db, s.id, "3", 1, vec![lot("L", 3, None)]);
    let inv = start(&mut f.db, None);
    let r = scan(&mut f.db, inv.inventory.id, "00A", 3, "requires-lot");
    assert_eq!(r.outcome, "lot_required");
    let lot_id = f.db.lots("2026-11-07").unwrap()[0].id;
    f.db.inventory_scan(InventoryScan {
        inventory_id: inv.inventory.id,
        request_key: "lot-count".into(),
        code: "00A".into(),
        quantity: 2,
        sku_id: None,
        lot_id: Some(lot_id),
        lot_selected: true,
    })
    .unwrap();
    let d = f.db.inventory(inv.inventory.id).unwrap();
    let line = d.lines.iter().find(|l| l.lot_id == Some(lot_id)).unwrap();
    let d =
        f.db.correct_count(CountCorrection {
            inventory_id: inv.inventory.id,
            line_id: line.id,
            request_key: "correct".into(),
            quantity: 3,
            expected_revision: d.inventory.revision,
        })
        .unwrap();
    assert_eq!(d.events[0].previous_count, Some(2));
    assert_eq!(d.events[0].new_count, Some(3));
    finish(&mut f.db, inv.inventory.id, Some("skip")).unwrap();
    assert_eq!(f.db.skus().unwrap()[0].balance, 3);
    assert_eq!(
        f.db.movements(MovementQuery {
            search: "".into(),
            sku_id: None,
            from_date: None,
            to_date: None,
            page: 0
        })
        .unwrap()
        .total,
        1
    );
}
#[test]
fn integrated_xml_lot_fefo_inventory_and_reopen() {
    let mut f = Fixture::new();
    let mut p = product("A");
    p.expiry_control = true;
    p.barcodes = vec!["00123456".into()];
    let s = f.db.save_product(p).unwrap();
    let import =
        f.db.import_xml(XmlImportInput {
            content: xml(10, "TEST", "00123456", "5", "12", "60", ""),
            file_name: "sintetico.xml".into(),
        })
        .unwrap();
    let item = &import.detail.items[0];
    let expiry = (chrono::Local::now().date_naive() + chrono::Days::new(30)).to_string();
    let d =
        f.db.save_receipt_item(ReceiptItemInput {
            receipt_id: import.detail.receipt.id,
            expected_revision: 0,
            item_id: Some(item.id),
            sku_id: Some(s.id),
            quantity: "5".into(),
            units_per_pack: 12,
            commercial_unit: "CX".into(),
            unit_price: "12".into(),
            total_cents: 6000,
            lots: vec![lot("L-XML", 60, Some(expiry))],
            remember_mapping: true,
        })
        .unwrap();
    f.db.confirm_receipt(ReceiptAction {
        id: d.receipt.id,
        expected_revision: d.receipt.revision,
        confirmed: true,
    })
    .unwrap();
    let plan =
        f.db.plan_issue(IssuePlanInput {
            sku_id: s.id,
            quantity: 10,
            include_undated: false,
            allow_expired: false,
        })
        .unwrap();
    let lot = plan.allocations[0].lot_id;
    f.db.issue_stock(IssueInput {
        request_key: "sale".into(),
        sku_id: s.id,
        allocations: plan.allocations,
        reason: "Saída física de teste".into(),
        allow_expired: false,
        confirmed: true,
    })
    .unwrap();
    let i = start(&mut f.db, None);
    f.db.inventory_scan(InventoryScan {
        inventory_id: i.inventory.id,
        request_key: "count".into(),
        code: "00123456".into(),
        quantity: 48,
        sku_id: None,
        lot_id: lot,
        lot_selected: true,
    })
    .unwrap();
    finish(&mut f.db, i.inventory.id, Some("skip")).unwrap();
    drop(f.db);
    f.db = Store::open(&f.path).unwrap();
    assert_eq!(f.db.skus().unwrap()[0].balance, 48);
    assert_eq!(f.db.lots("2026-11-07").unwrap()[0].balance, 48);
    let moves =
        f.db.movements(MovementQuery {
            search: "".into(),
            sku_id: Some(s.id),
            from_date: None,
            to_date: None,
            page: 0,
        })
        .unwrap();
    assert_eq!(moves.total, 3);
    assert_eq!(moves.items[0].kind, "inventory");
    f.db.integrity_check().unwrap();
}
#[test]
fn volume_1000_skus_and_5000_real_movements() {
    let mut f = Fixture::new();
    let start = std::time::Instant::now();
    for index in 0..1000 {
        let sku = f.db.save_product(product(&format!("M{index:04}"))).unwrap();
        for n in 0..5 {
            add(&mut f.db, sku.id, 1, &format!("mass-{index}-{n}"));
        }
    }
    let rows = f.db.skus().unwrap();
    assert_eq!(rows.len(), 1000);
    assert_eq!(rows.iter().map(|s| s.balance).sum::<i64>(), 5000);
    assert!(rows.iter().all(|s| s.balance == 5));
    assert_eq!(
        f.db.movements(MovementQuery {
            search: "".into(),
            sku_id: None,
            from_date: None,
            to_date: None,
            page: 99
        })
        .unwrap()
        .items
        .len(),
        50
    );
    let inv = start_inventory_for_mass(&mut f.db);
    assert_eq!(inv.inventory.sku_count, 1000);
    f.db.cancel_inventory(InventoryCancel {
        id: inv.inventory.id,
        confirmed: true,
    })
    .unwrap();
    f.db.integrity_check().unwrap();
    eprintln!(
        "Massa sintética: 1000 SKUs, 5000 movimentos: {:?}",
        start.elapsed()
    );
}
fn start_inventory_for_mass(db: &mut Store) -> InventoryDetail {
    start(db, None)
}

#[test]
fn b_xml_ambiguous_barcodes_do_not_choose_by_description() {
    let mut f = Fixture::new();
    for code in ["A", "B"] {
        let mut p = product(code);
        p.barcodes = vec!["00000001".into()];
        f.db.save_product(p).unwrap();
    }
    let result =
        f.db.import_xml(XmlImportInput {
            content: xml(41, "FORN", "00000001", "1", "1", "1", ""),
            file_name: "ambiguo.xml".into(),
        })
        .unwrap();
    assert_eq!(result.detail.items[0].sku_id, None);
    assert_eq!(result.detail.items[0].candidates.len(), 2);
}
#[test]
fn b_rastro_high_precision_adjustments_and_nested_xml_limits() {
    let content=xml(42,"L","SEM GTIN","3","0.335","1.01","<vDesc>0.01</vDesc><vFrete>0.02</vFrete><rastro><nLote>R-01</nLote><qLote>3</qLote><dFab>2026-10-01</dFab><dVal>2026-12-01</dVal></rastro>");
    let parsed = parse_purchase_xml(&content).unwrap();
    assert_eq!(parsed.items[0].total_cents, 102);
    assert_eq!(
        parsed.items[0].lots[0].expires_on.as_deref(),
        Some("2026-12-01")
    );
    let deep = format!("{}{}", "<a>".repeat(33), "</a>".repeat(33));
    assert!(parse_purchase_xml(&deep).is_err());
    assert!(parse_purchase_xml(&content.replace(
        "http://www.portalfiscal.inf.br/nfe",
        "https://malicioso.test/"
    ))
    .is_err());
}
#[test]
fn c_expired_lot_requires_explicit_decision_and_keeps_its_cost() {
    let mut f = Fixture::new();
    let mut p = product("A");
    p.expiry_control = true;
    let s = f.db.save_product(p).unwrap();
    let yesterday = (chrono::Local::now().date_naive() - chrono::Days::new(1)).to_string();
    receive(
        &mut f.db,
        s.id,
        "4",
        1,
        vec![lot("Velho", 4, Some(yesterday))],
    );
    let old = f.db.lots("2026-11-07").unwrap()[0].id;
    receive(&mut f.db, s.id, "5", 1, vec![lot("Novo", 5, None)]);
    assert!(f
        .db
        .plan_issue(IssuePlanInput {
            sku_id: s.id,
            quantity: 1,
            include_undated: false,
            allow_expired: false
        })
        .is_err());
    let plan =
        f.db.plan_issue(IssuePlanInput {
            sku_id: s.id,
            quantity: 1,
            include_undated: false,
            allow_expired: true,
        })
        .unwrap();
    assert_eq!(plan.allocations[0].lot_id, Some(old));
    f.db.issue_stock(IssueInput {
        request_key: "expired-explicit".into(),
        sku_id: s.id,
        allocations: plan.allocations,
        reason: "Descarte explícito".into(),
        allow_expired: true,
        confirmed: true,
    })
    .unwrap();
    let moves =
        f.db.movements(MovementQuery {
            search: "".into(),
            sku_id: Some(s.id),
            from_date: None,
            to_date: None,
            page: 0,
        })
        .unwrap();
    assert_eq!(moves.items[0].unit_cost_cents, 25);
    assert_eq!(f.db.skus().unwrap()[0].purchase_cents, 20);
}
#[test]
fn d_two_connections_enforce_one_inventory_and_finalized_counts_are_immutable() {
    let mut f = Fixture::new();
    let s = f.db.save_product(product("A")).unwrap();
    add(&mut f.db, s.id, 5, "stock");
    let i = start(&mut f.db, None);
    let mut other = Store::open(&f.path).unwrap();
    assert!(other
        .create_inventory(InventoryCreate {
            name: "Concorrente".into(),
            sku_ids: None
        })
        .is_err());
    assert!(other
        .adjust_stock(AdjustmentInput {
            request_key: "other-process".into(),
            sku_id: s.id,
            lot_id: None,
            quantity: 1,
            reason: "Outro processo".into(),
            reference: "".into(),
            confirmed: true
        })
        .is_err());
    finish(&mut f.db, i.inventory.id, Some("zero")).unwrap();
    let d = f.db.inventory(i.inventory.id).unwrap();
    assert_eq!(d.lines[0].counted, None);
    assert_eq!(d.lines[0].difference, Some(-5));
    let c = Connection::open(&f.path).unwrap();
    assert!(c
        .execute(
            "UPDATE inventory_lines SET counted=99 WHERE inventory_id=?1",
            [i.inventory.id]
        )
        .is_err());
    assert!(c
        .execute(
            "DELETE FROM inventory_lines WHERE inventory_id=?1",
            [i.inventory.id]
        )
        .is_err());
}

#[test]
fn b_shipped_xml_examples_include_default_and_prefixed_namespaces() {
    for content in [
        include_str!("fixtures/nfe-compra-exemplo.xml"),
        include_str!("fixtures/nfe-prefixado-exemplo.xml"),
    ] {
        let parsed = parse_purchase_xml(content).unwrap();
        assert_eq!(parsed.number, "123");
        assert_eq!(parsed.items[0].total_cents, 60000);
        assert_eq!(
            parsed.items[0].tax_barcode.as_deref(),
            Some("7890000000017")
        );
    }
}
