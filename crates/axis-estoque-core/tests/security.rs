use axis_estoque_core::*;
use rusqlite::{config::DbConfig, limits::Limit, Connection};
use std::{fs, path::Path};

fn product(code: &str) -> ProductInput {
    ProductInput {
        product_id: None,
        sku_id: None,
        name: "Produto sintético".into(),
        description: String::new(),
        brand: String::new(),
        category: String::new(),
        internal_code: code.into(),
        presentation: String::new(),
        purchase_cents: 100,
        sale_cents: 200,
        default_pack: 1,
        expiry_control: false,
        active: true,
        product_active: true,
        barcodes: vec![code.into()],
    }
}
fn add(db: &mut Store, id: i64, key: &str) {
    db.adjust_stock(AdjustmentInput {
        request_key: key.into(),
        sku_id: id,
        lot_id: None,
        quantity: 10,
        reason: "Teste sintético".into(),
        reference: String::new(),
        confirmed: true,
    })
    .unwrap();
}

#[test]
fn sql_injection_remains_data_in_catalog_scan_and_search() {
    let temp = tempfile::tempdir().unwrap();
    let mut db = Store::open(&temp.path().join("data.db")).unwrap();
    let payload = "x'); DROP TABLE skus; --";
    let sku = db.save_product(product(payload)).unwrap();
    add(&mut db, sku.id, "sql-test");
    assert_eq!(db.workspace().unwrap().skus[0].internal_code, payload);
    let movement = db
        .movements(MovementQuery {
            search: payload.into(),
            sku_id: None,
            from_date: None,
            to_date: None,
            page: 0,
        })
        .unwrap();
    assert_eq!(movement.total, 1);
    let session = db
        .create_inventory(InventoryCreate {
            name: payload.into(),
            sku_ids: Some(vec![sku.id]),
        })
        .unwrap();
    let counted = db
        .inventory_scan(InventoryScan {
            inventory_id: session.inventory.id,
            request_key: "sql-scan".into(),
            code: payload.into(),
            quantity: 1,
            sku_id: None,
            lot_id: None,
            lot_selected: false,
        })
        .unwrap();
    assert_eq!(counted.outcome, "counted");
    db.integrity_check().unwrap();
}

#[test]
fn defensive_sqlite_configuration_restricts_schema_extensions_and_limits() {
    let db = Connection::open_in_memory().unwrap();
    security::configure_connection(&db).unwrap();
    assert!(!db
        .db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA)
        .unwrap());
    assert!(db.db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE).unwrap());
    assert_eq!(
        db.limit(Limit::SQLITE_LIMIT_LENGTH).unwrap(),
        16 * 1024 * 1024
    );
    assert!(db
        .execute_batch("ATTACH DATABASE ':memory:' AS external")
        .is_err());
    assert!(db
        .query_row("SELECT load_extension('untrusted.dll')", [], |r| r
            .get::<_, String>(0))
        .is_err());
    db.execute_batch("PRAGMA writable_schema=ON").unwrap();
    assert!(db.execute("DELETE FROM sqlite_schema", []).is_err());
}

#[test]
fn backup_restore_live_wal_and_refuse_existing_target() {
    let temp = tempfile::tempdir().unwrap();
    let mut db = Store::open(&temp.path().join("data.db")).unwrap();
    let sku = db.save_product(product("RESTORE")).unwrap();
    add(&mut db, sku.id, "initial");
    let backup = db.create_backup().unwrap();
    let before = fs::read(&backup).unwrap();
    let source = Connection::open_with_flags(
        db.database_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        storage::backup_to(&source, Path::new(&backup))
            .unwrap_err()
            .code,
        "BACKUP_EXISTS"
    );
    assert_eq!(fs::read(&backup).unwrap(), before);
    fs::copy(&backup, temp.path().join("restored.db")).unwrap();
    let restored = Store::open(&temp.path().join("restored.db")).unwrap();
    assert_eq!(restored.workspace().unwrap().skus[0].balance, 10);
    restored.integrity_check().unwrap();
}

#[test]
fn hard_links_cannot_redirect_database_or_backup() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("original.txt");
    fs::write(&original, b"dados sinteticos preservados").unwrap();
    let linked = temp.path().join("linked.db");
    fs::hard_link(&original, &linked).unwrap();
    assert_eq!(Store::open(&linked).err().unwrap().code, "UNSAFE_PATH");
    assert_eq!(
        fs::read(&original).unwrap(),
        b"dados sinteticos preservados"
    );
    let db = Connection::open_in_memory().unwrap();
    assert_eq!(
        storage::backup_to(&db, &linked).unwrap_err().code,
        "UNSAFE_PATH"
    );
    let safe = temp.path().join("safe.db");
    drop(Store::open(&safe).unwrap());
    let journal = temp.path().join("safe.db-journal");
    fs::hard_link(&original, &journal).unwrap();
    assert_eq!(Store::open(&safe).err().unwrap().code, "UNSAFE_PATH");
    assert_eq!(
        fs::read(&original).unwrap(),
        b"dados sinteticos preservados"
    );
}

#[test]
fn relative_parent_and_network_paths_are_rejected() {
    for p in ["../outside.db", "relative.db"] {
        assert!(security::validate_path(Path::new(p)).is_err());
    }
    #[cfg(windows)]
    for p in [
        r"\\server\share\data.db",
        r"\\?\UNC\server\share\data.db",
        r"\\.\GLOBALROOT\data.db",
        r"C:\data\..\outside.db",
        r"C:\data\CON\test.db",
        r"C:\data\prova.db:alternate",
        r"C:\data\bad.\test.db",
    ] {
        assert!(security::validate_path(Path::new(p)).is_err());
    }
}

#[cfg(windows)]
#[test]
fn directory_junction_cannot_redirect_storage() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    fs::create_dir(&target).unwrap();
    let link = temp.path().join("junction");
    let result=std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command","$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:ASTOCK_TEST_JUNCTION -Target $env:ASTOCK_TEST_TARGET | Out-Null"]).env("ASTOCK_TEST_JUNCTION",&link).env("ASTOCK_TEST_TARGET",&target).output().unwrap();
    assert!(
        result.status.success(),
        "Não foi possível criar a junção sintética para o teste"
    );
    assert_eq!(
        Store::open(&link.join("data.db")).err().unwrap().code,
        "UNSAFE_PATH"
    );
    assert!(!target.join("data.db").exists());
    fs::remove_dir(&link).unwrap();
}

#[test]
fn invalid_ipc_structs_and_oversized_inventory_are_rejected_without_mutation() {
    assert!(serde_json::from_str::<AdjustmentInput>(r#"{"requestKey":"x","skuId":1,"lotId":null,"quantity":9223372036854775808,"reason":"x","reference":"","confirmed":true}"#).is_err());
    assert!(serde_json::from_str::<XmlImportInput>(
        r#"{"content":"x","fileName":"x","path":"C:/outside"}"#
    )
    .is_err());
    let temp = tempfile::tempdir().unwrap();
    let mut db = Store::open(&temp.path().join("data.db")).unwrap();
    assert!(db
        .create_inventory(InventoryCreate {
            name: "Mass request".into(),
            sku_ids: Some(vec![1; 10_001])
        })
        .is_err());
    assert!(db.inventories().unwrap().is_empty());
}

#[test]
fn xml_entities_and_html_are_inert_or_rejected() {
    let fixture = include_str!("fixtures/nfe-compra-exemplo.xml");
    let start = fixture.find("<xProd>").unwrap() + 7;
    let end = start + fixture[start..].find("</xProd>").unwrap();
    let encoded = format!(
        "{}&lt;img src=x onerror=alert(1)&gt;{}",
        &fixture[..start],
        &fixture[end..]
    );
    let parsed = xml::parse_purchase_xml(&encoded).unwrap();
    assert_eq!(parsed.items[0].description, "<img src=x onerror=alert(1)>");
    for data in [
        "<!DOCTYPE NFe [<!ENTITY x SYSTEM 'file:///C:/Windows/win.ini'>]><NFe>&x;</NFe>",
        "<!DOCTYPE x [<!ENTITY a 'aaaaaaaa'><!ENTITY b '&a;&a;&a;'>]><x>&b;</x>",
        "<x>\u{0}</x>",
    ] {
        assert!(xml::parse_purchase_xml(data).is_err());
    }
}

#[test]
fn process_exit_rolls_back_uncommitted_changes_and_preserves_commits() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("crash.db");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_fixture_child", "--ignored", "--nocapture"])
        .env("ASTOCK_CRASH_FIXTURE", &path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(91));
    let restored = Store::open(&path).unwrap();
    let state = restored.workspace().unwrap();
    assert_eq!(state.skus[0].balance, 10);
    assert_eq!(state.skus[0].name, "Produto sintético");
    restored.integrity_check().unwrap();
}

#[test]
#[ignore = "Executado somente como subprocesso pelo teste de recuperação"]
fn crash_fixture_child() {
    let path = std::env::var_os("ASTOCK_CRASH_FIXTURE").expect("Fixture de subprocesso ausente");
    let mut db = Store::open(Path::new(&path)).unwrap();
    let sku = db.save_product(product("CRASH")).unwrap();
    add(&mut db, sku.id, "committed-before-crash");
    let direct = Connection::open(&path).unwrap();
    direct
        .execute_batch("BEGIN IMMEDIATE; UPDATE products SET name='UNCOMMITTED';")
        .unwrap();
    std::process::exit(91);
}
