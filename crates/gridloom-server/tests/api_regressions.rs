use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use gridloom_core::{
    value::{CellCoord, Value},
    Workbook,
};
use gridloom_server::{routes::create_router, state::AppState};
use serde_json::{json, Value as Json};
use tower::ServiceExt;

async fn call(app: &Router, method: &str, path: &str, body: Json) -> (StatusCode, Json) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    assert!(
        status.is_success(),
        "{status}: {}",
        String::from_utf8_lossy(&bytes)
    );
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn metadata_edit_review_refresh_and_pivot_chart_contracts() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let external = rusqlite::Connection::open(file.path()).unwrap();
    external.execute_batch("CREATE TABLE items(id TEXT PRIMARY KEY, amount INTEGER); INSERT INTO items VALUES('A',10),('a',20)").unwrap();
    let mut wb = Workbook::new();
    wb.add_sqlite_connection("db", file.path()).unwrap();
    wb.create_named_source("source", "db", "SELECT * FROM items ORDER BY id")
        .unwrap();
    wb.insert_table("Sheet1", "DisplayName", "source", CellCoord::new(0, 0))
        .unwrap();
    wb.set_cell(
        "Sheet1",
        CellCoord::new(0, 3),
        "=SUM(DisplayName[amount])".into(),
    )
    .unwrap();
    let app = create_router(AppState::new(wb));
    let (_, summary) = call(&app, "GET", "/api/workbook", json!(null)).await;
    let sheet = summary["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == summary["active_sheet"])
        .unwrap();
    assert_eq!(sheet["tables"][0]["name"], "DisplayName");
    let source = sheet["tables"][0]["source_name"].as_str().unwrap();
    call(&app, "POST", "/api/staged", json!({"connection_name":"db", "table_name":"items", "pk_columns":["id"],
        "pk_values":[{"type":"Text","value":"A"}], "column_name":"amount", "original_value":{"type":"Integer","value":10},
        "proposed_value":{"type":"Decimal","value":"15"}})).await;
    let (_, items) = call(&app, "GET", "/api/writeback/review", json!(null)).await;
    assert_eq!(items[0]["can_apply"], true);
    let (_, viewport) = call(&app, "POST", "/api/cells/viewport", json!({"sheet_name":"Sheet1","start_row":0,"end_row":5,"start_col":0,"end_col":5,"overscan":0})).await;
    assert_eq!(viewport["cells"]["B2"]["display_text"], "15");
    assert_eq!(viewport["cells"]["B3"]["display_text"], "20");
    call(
        &app,
        "POST",
        "/api/writeback/apply",
        json!({"connection_name":"db"}),
    )
    .await;
    external
        .execute("INSERT INTO items VALUES ('b',30)", [])
        .unwrap();
    call(&app, "POST", "/api/sources/source/refresh", json!(null)).await;
    let (_, vp) = call(&app, "POST", "/api/cells/viewport", json!({"sheet_name":"Sheet1","start_row":0,"end_row":5,"start_col":0,"end_col":5,"overscan":0})).await;
    assert_eq!(vp["cells"]["D1"]["display_text"], "65");
    assert_eq!(vp["cells"]["B4"]["display_text"], "30");
    call(&app, "POST", "/api/pivots", json!({"name":"p", "source_table":source, "row_fields":["id"], "col_fields":[],"value_field":"amount","aggregation":"Sum"})).await;
    let (_, pivot) = call(&app, "GET", "/api/pivots/p/data", json!(null)).await;
    assert!(!pivot["rows"].as_array().unwrap().is_empty());
    call(&app, "POST", "/api/charts", json!({"id":"c","title":"chart","chart_type":"Bar","source_table":source,"category_column":"id","series":[{"name":"amount","value_column":"amount"}]})).await;
    let (_, chart) = call(&app, "GET", "/api/charts/c/data", json!(null)).await;
    assert_eq!(chart["series"][0]["values"], json!([15.0, 20.0, 30.0]));
}
#[tokio::test]
async fn review_uses_each_patches_own_connection() {
    let first = tempfile::NamedTempFile::new().unwrap();
    let second = tempfile::NamedTempFile::new().unwrap();
    let mut wb = Workbook::new();
    for (name, file, value) in [("one", &first, 10), ("two", &second, 20)] {
        rusqlite::Connection::open(file.path()).unwrap().execute_batch(&format!("CREATE TABLE items(id INTEGER PRIMARY KEY, amount INTEGER); INSERT INTO items VALUES(1,{value})")).unwrap();
        wb.add_sqlite_connection(name, file.path()).unwrap();
        wb.staged.stage_edit(
            name.into(),
            "items".into(),
            vec!["id".into()],
            vec![Value::Integer(1)],
            "amount".into(),
            Value::Integer(value),
            Value::Integer(30),
        );
    }
    let app = create_router(AppState::new(wb));
    let (_, items) = call(&app, "GET", "/api/writeback/review", json!(null)).await;
    assert_eq!(items.as_array().unwrap().len(), 2);
    assert!(items
        .as_array()
        .unwrap()
        .iter()
        .all(|i| i["can_apply"] == true));
    call(
        &app,
        "POST",
        "/api/writeback/apply",
        json!({"connection_name":"one"}),
    )
    .await;
    let (_, remaining) = call(&app, "GET", "/api/staged", json!(null)).await;
    assert_eq!(remaining.as_array().unwrap().len(), 1);
    assert_eq!(remaining[0]["connection_name"], "two");
}
