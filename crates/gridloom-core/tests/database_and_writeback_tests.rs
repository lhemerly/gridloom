use gridloom_core::db::EngineKind;
use gridloom_core::error::CellError;
use gridloom_core::value::{CellCoord, Value};
use gridloom_core::workbook::Workbook;
use rusqlite::Connection as RusqliteConn;
use rust_decimal::Decimal;
use tempfile::NamedTempFile;

#[test]
fn test_sqlite_and_duckdb_connections() {
    let mut wb = Workbook::new();
    wb.add_sqlite_memory("sqlite_db").unwrap();
    wb.add_duckdb_memory("duck_db").unwrap();

    assert!(wb.connections.contains_key("sqlite_db"));
    assert!(wb.connections.contains_key("duck_db"));

    let sqlite_conn = wb.connections.get("sqlite_db").unwrap();
    assert_eq!(sqlite_conn.kind(), EngineKind::Sqlite);

    let duck_conn = wb.connections.get("duck_db").unwrap();
    assert_eq!(duck_conn.kind(), EngineKind::DuckDb);
}

#[test]
fn test_security_read_only_boundary() {
    let mut wb = Workbook::new();
    wb.add_sqlite_memory("local_db").unwrap();

    // SELECT is allowed
    let res = wb.connections.get("local_db").unwrap().execute_read_query("SELECT 1 AS num", &[]);
    assert!(res.is_ok());

    // Mutations and attacks are rejected
    let del = wb.connections.get("local_db").unwrap().execute_read_query("DELETE FROM invoices", &[]);
    assert!(del.is_err());

    let drop = wb.connections.get("local_db").unwrap().execute_read_query("DROP TABLE invoices", &[]);
    assert!(drop.is_err());

    let attach = wb.connections.get("local_db").unwrap().execute_read_query("ATTACH DATABASE 'evil.db' AS evil", &[]);
    assert!(attach.is_err());
}

#[test]
fn test_staged_writeback_and_concurrency_conflict() {
    // 1. Create a SQLite database file with an invoices table
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap().to_string();

    {
        let conn = RusqliteConn::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE invoices (id INTEGER PRIMARY KEY, customer TEXT, amount REAL, status TEXT)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO invoices (id, customer, amount, status) VALUES (1, 'Acme Corp', 1500.0, 'Pending')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO invoices (id, customer, amount, status) VALUES (2, 'Globex Inc', 3200.0, 'Approved')",
            [],
        )
        .unwrap();
    }

    let mut wb = Workbook::new();
    wb.add_sqlite_connection("MainDB", &db_path).unwrap();

    // 2. Create named source from query
    let source = wb.create_named_source("Invoices", "MainDB", "SELECT id, customer, amount, status FROM invoices").unwrap();
    assert!(source.lineage.as_ref().unwrap().is_editable);

    // 3. Stage an edit: change Acme Corp amount from 1500 to 2000
    let _patch_id = wb.staged.stage_edit(
        "MainDB".to_string(),
        "invoices".to_string(),
        vec!["id".to_string()],
        vec![Value::Integer(1)],
        "amount".to_string(),
        Value::Float(1500.0),
        Value::Float(2000.0),
    );

    assert_eq!(wb.staged.patches.len(), 1);

    // 4. Simulate external concurrent edit: someone changes Acme Corp amount to 1800 behind our back!
    {
        let conn = RusqliteConn::open(&db_path).unwrap();
        conn.execute("UPDATE invoices SET amount = 1800.0 WHERE id = 1", []).unwrap();
    }

    // 5. Try applying writeback -> MUST FAIL and roll back with concurrency conflict!
    let conn = wb.connections.get("MainDB").unwrap();
    let apply_err = wb.staged.apply_writeback(conn, "MainDB");
    assert!(apply_err.is_err(), "Writeback must detect external modification conflict!");
    // Staged patch remains preserved for review, not discarded!
    assert_eq!(wb.staged.patches.len(), 1);
    assert_eq!(wb.staged.audit_log.len(), 1);

    // 6. Fix conflict by rebasing original value to 1800 and changing to 2500
    wb.staged.stage_edit(
        "MainDB".to_string(),
        "invoices".to_string(),
        vec!["id".to_string()],
        vec![Value::Integer(1)],
        "amount".to_string(),
        Value::Float(1800.0),
        Value::Float(2500.0),
    );

    // 7. Re-apply writeback -> MUST SUCCEED and update base table!
    let rec = wb.staged.apply_writeback(conn, "MainDB").unwrap();
    assert_eq!(rec.patch_count, 1);
    assert_eq!(wb.staged.patches.len(), 0);

    // 8. Verify base database row has updated value
    {
        let check_conn = RusqliteConn::open(&db_path).unwrap();
        let updated_amt: f64 = check_conn
            .query_row("SELECT amount FROM invoices WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(updated_amt, 2500.0);
    }
}

#[test]
fn test_sqlvalue_and_sqltable_formulas() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap().to_string();

    {
        let conn = RusqliteConn::open(&db_path).unwrap();
        conn.execute("CREATE TABLE metrics (name TEXT PRIMARY KEY, score INTEGER)", []).unwrap();
        conn.execute("INSERT INTO metrics VALUES ('Alpha', 95)", []).unwrap();
        conn.execute("INSERT INTO metrics VALUES ('Beta', 88)", []).unwrap();
    }

    let mut wb = Workbook::new();
    wb.add_sqlite_connection("LocalDB", &db_path).unwrap();

    // SQLVALUE formula
    wb.set_cell("Sheet1", CellCoord::new(0, 0), "=SQLVALUE(\"LocalDB\", \"SELECT score FROM metrics WHERE name = 'Alpha'\")".to_string()).unwrap();
    let cell_a1 = wb.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(0, 0)).unwrap();
    assert_eq!(cell_a1.evaluated_value, Value::Integer(95));

    // SQLVALUE with 0 rows returns explicit NULL
    wb.set_cell("Sheet1", CellCoord::new(1, 0), "=SQLVALUE(\"LocalDB\", \"SELECT score FROM metrics WHERE name = 'Missing'\")".to_string()).unwrap();
    let cell_a2 = wb.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(1, 0)).unwrap();
    assert_eq!(cell_a2.evaluated_value, Value::Null);

    // SQLVALUE with multi-row returns shape error (#VALUE!)
    wb.set_cell("Sheet1", CellCoord::new(2, 0), "=SQLVALUE(\"LocalDB\", \"SELECT score FROM metrics\")".to_string()).unwrap();
    let cell_a3 = wb.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(2, 0)).unwrap();
    assert_eq!(cell_a3.evaluated_value, Value::Error(CellError::Value));

    // SQLTABLE spilling formula
    wb.set_cell("Sheet1", CellCoord::new(3, 0), "=SQLTABLE(\"LocalDB\", \"SELECT name, score FROM metrics ORDER BY name\")".to_string()).unwrap();
    let cell_a4 = wb.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(3, 0)).unwrap();
    match &cell_a4.evaluated_value {
        Value::Array(rows) => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][0], Value::Text("Alpha".to_string()));
            assert_eq!(rows[0][1], Value::Integer(95));
            assert_eq!(rows[1][0], Value::Text("Beta".to_string()));
            assert_eq!(rows[1][1], Value::Integer(88));
        }
        _ => panic!("Expected dynamic array from SQLTABLE"),
    }
}

#[test]
fn test_spill_collision_prevention() {
    let mut wb = Workbook::new();

    // Occupy cell B2
    wb.set_cell("Sheet1", CellCoord::new(1, 1), "Blocked".to_string()).unwrap();

    // Set dynamic array formula in A1 that would spill 2x2 (covering A1, A2, B1, B2)
    wb.set_cell("Sheet1", CellCoord::new(0, 0), "=SEQUENCE(2, 2)".to_string()).unwrap();

    let cell_a1 = wb.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(0, 0)).unwrap();
    // Must produce #SPILL! error because B2 is occupied, without overwriting B2!
    assert_eq!(cell_a1.evaluated_value, Value::Error(CellError::Spill));

    let cell_b2 = wb.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(1, 1)).unwrap();
    assert_eq!(cell_b2.evaluated_value, Value::Text("Blocked".to_string()));

    // Now clear cell B2 -> recalculate -> spill succeeds!
    wb.set_cell("Sheet1", CellCoord::new(1, 1), "".to_string()).unwrap();
    wb.set_cell("Sheet1", CellCoord::new(0, 0), "=SEQUENCE(2, 2)".to_string()).unwrap();

    let cell_a1_cleared = wb.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(0, 0)).unwrap();
    match &cell_a1_cleared.evaluated_value {
        Value::Array(rows) => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 2);
        }
        _ => panic!("Expected successful spill after clearing block"),
    }
}

#[test]
fn test_workbook_save_and_reopen_persistence() {
    let temp_save = NamedTempFile::new().unwrap();
    let save_path = temp_save.path().to_str().unwrap().to_string();

    let mut wb = Workbook::new();
    wb.set_cell("Sheet1", CellCoord::new(0, 0), "100".to_string()).unwrap();
    wb.set_cell("Sheet1", CellCoord::new(1, 0), "=A1 * 2".to_string()).unwrap();

    // Add staged edit
    wb.staged.stage_edit(
        "Conn1".to_string(),
        "sales".to_string(),
        vec!["id".to_string()],
        vec![Value::Integer(10)],
        "price".to_string(),
        Value::Decimal(Decimal::from(50)),
        Value::Decimal(Decimal::from(75)),
    );

    // Save atomically
    wb.save_to_file(&save_path).unwrap();

    // Reopen from file
    let loaded = Workbook::open_from_file(&save_path).unwrap();

    let a1 = loaded.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(0, 0)).unwrap();
    assert_eq!(a1.evaluated_value, Value::Integer(100));

    let a2 = loaded.sheets.get("Sheet1").unwrap().get_cell(&CellCoord::new(1, 0)).unwrap();
    assert_eq!(a2.formula, Some("=A1 * 2".to_string()));
    assert_eq!(a2.evaluated_value, Value::Decimal(Decimal::from(200)));

    assert_eq!(loaded.staged.patches.len(), 1);
    let patch = loaded.staged.list_patches().pop().unwrap();
    assert_eq!(patch.table_name, "sales");
    assert_eq!(patch.proposed_value, Value::Decimal(Decimal::from(75)));
}
