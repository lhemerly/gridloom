use gridloom_core::writeback::{AuditStatus, ConflictStatus};
use gridloom_core::{
    value::{CellCoord, Value},
    virtual_view::ViewportRequest,
    workbook::{Sheet, Workbook},
};
use tempfile::{NamedTempFile, TempDir};

fn stage(wb: &mut Workbook, connection: &str, key: Value, original: Value, proposed: Value) {
    wb.staged.stage_edit(
        connection.into(),
        "items".into(),
        vec!["id".into()],
        vec![key],
        "label".into(),
        original,
        proposed,
    );
}
fn fixture() -> (NamedTempFile, Workbook) {
    let file = NamedTempFile::new().unwrap();
    rusqlite::Connection::open(file.path()).unwrap().execute_batch(
        "CREATE TABLE items(id TEXT PRIMARY KEY, label TEXT UNIQUE); INSERT INTO items VALUES ('A','first'),('a','second');"
    ).unwrap();
    let mut wb = Workbook::new();
    wb.add_sqlite_connection("db", file.path()).unwrap();
    (file, wb)
}
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn viewport(wb: &Workbook) -> gridloom_core::virtual_view::ViewportResponse {
    wb.get_viewport(&ViewportRequest {
        sheet_name: "Sheet1".into(),
        start_row: 0,
        end_row: 10,
        start_col: 0,
        end_col: 10,
        overscan: 0,
    })
    .unwrap()
}
#[test]
fn unique_failure_rolls_back_entire_sqlite_batch_and_preserves_staging() {
    let (_file, mut wb) = fixture();
    stage(&mut wb, "db", text("A"), text("first"), text("duplicate"));
    stage(&mut wb, "db", text("a"), text("second"), text("duplicate"));
    assert_eq!(
        wb.staged.patches.len(),
        2,
        "case-sensitive keys must remain separate"
    );
    assert!(wb.apply_writeback("db").is_err());
    let rows = wb.connections["db"]
        .execute_read_query("SELECT label FROM items ORDER BY id", &[])
        .unwrap()
        .rows;
    assert_eq!(rows, vec![vec![text("first")], vec![text("second")]]);
    assert_eq!(wb.staged.patches.len(), 2);
    assert!(matches!(
        wb.staged.audit_log[0].status,
        AuditStatus::RolledBackError
    ));
}
#[test]
fn exact_keys_update_distinct_rows_and_overlay_only_the_matching_row() {
    let (_file, mut wb) = fixture();
    wb.create_named_source(
        "DifferentSourceName",
        "db",
        "SELECT id, label FROM items ORDER BY id",
    )
    .unwrap();
    wb.insert_table(
        "Sheet1",
        "DifferentTableName",
        "DifferentSourceName",
        CellCoord::new(0, 0),
    )
    .unwrap();
    stage(&mut wb, "db", text("A"), text("first"), text("edited A"));
    let vp = viewport(&wb);
    assert_eq!(vp.cells["B2"].display_text, "edited A");
    assert!(vp.cells["B2"].is_staged);
    assert_eq!(vp.cells["B3"].display_text, "second");
    assert!(!vp.cells["B3"].is_staged);
    stage(&mut wb, "db", text("a"), text("second"), text("edited a"));
    wb.apply_writeback("db").unwrap();
    let rows = wb.connections["db"]
        .execute_read_query("SELECT label FROM items ORDER BY id", &[])
        .unwrap()
        .rows;
    // Spreadsheet equality folds case; inspect exact strings instead.
    assert_eq!(rows[0][0].to_string_repr(), "edited A");
    assert_eq!(rows[1][0].to_string_repr(), "edited a");
}
#[test]
fn duckdb_integer_and_date_reads_agree_with_writeback_and_batch_is_atomic() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("db.duckdb");
    {
        let conn = duckdb::Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE items(id INTEGER PRIMARY KEY, label DATE UNIQUE); INSERT INTO items VALUES (1, DATE '2026-01-01'),(2, DATE '2026-01-02');").unwrap();
    }
    let mut wb = Workbook::new();
    wb.add_duckdb_connection("db", &path).unwrap();
    let rows = wb.connections["db"]
        .execute_read_query("SELECT id, label FROM items ORDER BY id", &[])
        .unwrap()
        .rows;
    for row in &rows {
        stage(
            &mut wb,
            "db",
            row[0].clone(),
            row[1].clone(),
            Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
        );
    }
    assert!(wb
        .staged
        .review(&wb.connections["db"], "db")
        .iter()
        .all(|i| i.conflict_status == ConflictStatus::NoConflict));
    assert!(wb.apply_writeback("db").is_err());
    assert_eq!(
        wb.connections["db"]
            .execute_read_query("SELECT id, label FROM items ORDER BY id", &[])
            .unwrap()
            .rows,
        rows
    );
    wb.staged.discard_all();
    stage(
        &mut wb,
        "db",
        rows[0][0].clone(),
        rows[0][1].clone(),
        Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
    );
    wb.apply_writeback("db").unwrap();
    // INTEGER as the edited value used to decode to NULL inside the transaction.
    wb.staged.stage_edit(
        "db".into(),
        "items".into(),
        vec!["id".into()],
        vec![Value::Integer(1)],
        "id".into(),
        Value::Integer(1),
        Value::Integer(3),
    );
    wb.apply_writeback("db").unwrap();
    assert_eq!(
        wb.connections["db"]
            .execute_read_query("SELECT id FROM items ORDER BY id", &[])
            .unwrap()
            .rows,
        vec![vec![Value::Integer(2)], vec![Value::Integer(3)]]
    );
}
#[test]
fn formula_error_handlers_are_lazy_and_inspect_errors() {
    let mut wb = Workbook::new();
    for (formula, expected) in [
        ("=IFERROR(1/0,0)", Value::Integer(0)),
        ("=IF(TRUE,1,1/0)", Value::Integer(1)),
        ("=IF(FALSE,1/0,2)", Value::Integer(2)),
        ("=ISERROR(1/0)", Value::Boolean(true)),
        ("=IFERROR(5,1/0)", Value::Integer(5)),
        ("=IFS(TRUE,1,FALSE,1/0)", Value::Integer(1)),
        ("=IFNA(1,1/0)", Value::Integer(1)),
    ] {
        wb.set_cell("Sheet1", CellCoord::new(0, 0), formula.into())
            .unwrap();
        assert_eq!(
            wb.sheets["Sheet1"]
                .get_cell(&CellCoord::new(0, 0))
                .unwrap()
                .evaluated_value,
            expected,
            "{formula}"
        );
    }
}
#[test]
fn spills_round_trip_and_child_dependents_recalculate_in_order() {
    let mut wb = Workbook::new();
    // Consumer precedes producer, so first-time dependency discovery must work too.
    wb.set_cell("Sheet1", CellCoord::new(0, 2), "=A2+1".into())
        .unwrap();
    wb.set_cell("Sheet1", CellCoord::new(0, 0), "=SEQUENCE(2)".into())
        .unwrap();
    assert_eq!(viewport(&wb).cells["C1"].raw_value, Value::Integer(3));
    let file = NamedTempFile::new().unwrap();
    wb.save_to_file(file.path()).unwrap();
    let mut loaded = Workbook::open_from_file(file.path()).unwrap();
    assert_eq!(viewport(&loaded).cells["A2"].raw_value, Value::Integer(2));
    loaded
        .set_cell("Sheet1", CellCoord::new(0, 0), "=SEQUENCE(3,1,10)".into())
        .unwrap();
    assert_eq!(viewport(&loaded).cells["C1"].raw_value, Value::Integer(12));
    loaded
        .set_cell("Sheet1", CellCoord::new(0, 0), "5".into())
        .unwrap();
    assert!(!viewport(&loaded).cells.contains_key("A2"));
    assert_eq!(viewport(&loaded).cells["C1"].raw_value, Value::Integer(1));
}
#[test]
fn refresh_grows_shrinks_and_recalculates_tables_on_every_sheet() {
    let (file, mut wb) = fixture();
    wb.create_named_source(
        "src",
        "db",
        "SELECT id, length(label) AS length FROM items ORDER BY id",
    )
    .unwrap();
    wb.insert_table("Sheet1", "Items", "src", CellCoord::new(0, 0))
        .unwrap();
    wb.sheets.insert("Other".into(), Sheet::new("Other".into()));
    wb.insert_table("Other", "Items", "src", CellCoord::new(0, 0))
        .unwrap();
    for name in ["Sheet1", "Other"] {
        wb.set_cell(name, CellCoord::new(0, 3), "=SUM(Items[length])".into())
            .unwrap();
    }
    let external = rusqlite::Connection::open(file.path()).unwrap();
    external
        .execute("INSERT INTO items VALUES ('b','third')", [])
        .unwrap();
    wb.refresh_source("src").unwrap();
    assert_eq!(viewport(&wb).cells["B4"].raw_value, Value::Integer(5));
    for name in ["Sheet1", "Other"] {
        assert_eq!(wb.sheets[name].tables["Items"].range.end_row, 3);
        assert_eq!(
            wb.sheets[name]
                .get_cell(&CellCoord::new(0, 3))
                .unwrap()
                .evaluated_value,
            Value::Integer(16)
        );
    }
    external
        .execute("DELETE FROM items WHERE id != 'A'", [])
        .unwrap();
    wb.refresh_source("src").unwrap();
    assert_eq!(wb.sheets["Sheet1"].tables["Items"].range.end_row, 1);
    assert!(!viewport(&wb).cells.contains_key("B4"));
    assert_eq!(viewport(&wb).cells["D1"].raw_value, Value::Integer(5));
}
#[test]
fn refresh_collision_keeps_previous_result_and_table_range() {
    let (file, mut wb) = fixture();
    wb.create_named_source("src", "db", "SELECT * FROM items")
        .unwrap();
    wb.insert_table("Sheet1", "Items", "src", CellCoord::new(0, 0))
        .unwrap();
    wb.set_cell("Sheet1", CellCoord::new(3, 0), "keep me".into())
        .unwrap();
    rusqlite::Connection::open(file.path())
        .unwrap()
        .execute("INSERT INTO items VALUES ('b','third')", [])
        .unwrap();
    assert!(wb.refresh_source("src").is_err());
    assert_eq!(
        wb.named_sources["src"]
            .last_result
            .as_ref()
            .unwrap()
            .rows
            .len(),
        2
    );
    assert_eq!(wb.sheets["Sheet1"].tables["Items"].range.end_row, 2);
    assert_eq!(viewport(&wb).cells["A4"].display_text, "keep me");
}

#[test]
fn spill_ranges_use_each_displayed_scalar_and_hash_refs_keep_the_array() {
    let mut wb = Workbook::new();
    wb.set_cell("Sheet1", CellCoord::new(0, 0), "=SEQUENCE(2)".into())
        .unwrap();
    wb.set_cell("Sheet1", CellCoord::new(0, 2), "=SUM(A1:A2)".into())
        .unwrap();
    wb.set_cell("Sheet1", CellCoord::new(1, 2), "=SUM(A1#)".into())
        .unwrap();
    let vp = viewport(&wb);
    assert_eq!(vp.cells["A1"].display_text, "1");
    assert_eq!(vp.cells["C1"].display_text, "3");
    assert_eq!(vp.cells["C2"].display_text, "3");
}

#[test]
fn empty_duckdb_results_keep_column_metadata() {
    let mut wb = Workbook::new();
    wb.add_duckdb_memory("db").unwrap();
    let res = wb.connections["db"]
        .execute_read_query("SELECT 1 AS id, 2 AS amount WHERE false", &[])
        .unwrap();
    assert!(res.rows.is_empty());
    assert_eq!(
        res.columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        vec!["id", "amount"]
    );
}

#[test]
fn transactional_checks_roll_back_earlier_updates_on_a_late_conflict() {
    let (file, mut wb) = fixture();
    stage(&mut wb, "db", text("A"), text("first"), text("changed"));
    stage(&mut wb, "db", text("a"), text("second"), text("other"));
    rusqlite::Connection::open(file.path())
        .unwrap()
        .execute("UPDATE items SET label='external' WHERE id='a'", [])
        .unwrap();
    let patches = wb.staged.list_patches();
    // Call the engine batch directly: the transactional recheck must work even without preflight.
    assert!(wb.connections["db"].update_rows(&patches).is_err());
    let rows = wb.connections["db"]
        .execute_read_query("SELECT label FROM items ORDER BY id", &[])
        .unwrap()
        .rows;
    assert_eq!(rows, vec![vec![text("first")], vec![text("external")]]);
}
