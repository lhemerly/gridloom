use crate::benchmark::run_performance_benchmarks;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use gridloom_core::chart::ChartDefinition;
use gridloom_core::db::EngineKind;
use gridloom_core::lineage::analyze_lineage;
use gridloom_core::pivot::PivotDefinition;
use gridloom_core::value::{CellCoord, Value};
use gridloom_core::virtual_view::ViewportRequest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/api/health", get(health_handler))
        // Workbook
        .route("/api/workbook", get(get_workbook_summary))
        .route("/api/workbook/new", post(new_workbook))
        .route("/api/workbook/open", post(open_workbook))
        .route("/api/workbook/save", post(save_workbook))
        // Database Connections & Schema
        .route("/api/connections", get(list_connections).post(connect_db))
        .route("/api/connections/{name}/tables", get(list_tables))
        .route(
            "/api/connections/{name}/tables/{table}/schema",
            get(get_table_schema),
        )
        // Query Execution & Lineage
        .route("/api/query/execute", post(execute_query))
        .route("/api/query/analyze", post(analyze_query))
        // Named Data Sources
        .route("/api/sources", get(list_sources).post(create_source))
        .route("/api/sources/{name}/refresh", post(refresh_source))
        // Worksheet Tables
        .route("/api/tables/insert", post(insert_table))
        // Cells & Virtual Viewport
        .route("/api/cells/set", post(set_cell))
        .route("/api/cells/viewport", post(get_viewport))
        // Staged Writeback & Review
        .route(
            "/api/staged",
            get(list_staged).post(stage_edit).delete(discard_all_staged),
        )
        .route("/api/staged/{id}", delete(discard_staged_item))
        .route("/api/writeback/review", get(review_writeback))
        .route("/api/writeback/apply", post(apply_writeback))
        .route("/api/audit", get(list_audit_log))
        // Pivots & Charts
        .route("/api/pivots", get(list_pivots).post(save_pivot))
        .route("/api/pivots/{name}/data", get(get_pivot_data))
        .route("/api/charts", get(list_charts).post(save_chart))
        .route("/api/charts/{id}/data", get(get_chart_data))
        // Performance Acceptance Benchmark Runner
        .route("/api/benchmark/run", post(run_benchmarks))
        .with_state(state)
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "app": "Gridloom",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[derive(Serialize)]
struct SheetSummary {
    name: String,
    tables: Vec<gridloom_core::table::WorksheetTable>,
}

#[derive(Serialize)]
struct WorkbookSummary {
    active_sheet: String,
    sheets: Vec<SheetSummary>,
    connections: Vec<String>,
    sources: Vec<String>,
    staged_patches_count: usize,
    file_path: Option<String>,
}

async fn get_workbook_summary(State(state): State<AppState>) -> impl IntoResponse {
    let wb = state.workbook.read().await;
    let summary = WorkbookSummary {
        active_sheet: wb.active_sheet.clone(),
        sheets: wb
            .sheets
            .values()
            .map(|s| SheetSummary {
                name: s.name.clone(),
                tables: s.tables.values().cloned().collect(),
            })
            .collect(),
        connections: wb.connections.keys().cloned().collect(),
        sources: wb.named_sources.keys().cloned().collect(),
        staged_patches_count: wb.staged.patches.len(),
        file_path: wb
            .file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
    };
    Json(summary)
}

async fn new_workbook(State(state): State<AppState>) -> impl IntoResponse {
    let mut wb = state.workbook.write().await;
    *wb = gridloom_core::Workbook::new();
    Json(serde_json::json!({ "status": "created" }))
}

#[derive(Deserialize)]
struct PathPayload {
    path: String,
}

async fn open_workbook(
    State(state): State<AppState>,
    Json(payload): Json<PathPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let loaded = gridloom_core::Workbook::open_from_file(&payload.path)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let mut wb = state.workbook.write().await;
    *wb = loaded;
    Ok(Json(
        serde_json::json!({ "status": "opened", "path": payload.path }),
    ))
}

async fn save_workbook(
    State(state): State<AppState>,
    Json(payload): Json<PathPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut wb = state.workbook.write().await;
    wb.save_to_file(&payload.path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "status": "saved", "path": payload.path }),
    ))
}

#[derive(Deserialize)]
struct ConnectDbRequest {
    name: String,
    kind: String, // "sqlite" or "duckdb"
    path: String,
}

async fn connect_db(
    State(state): State<AppState>,
    Json(req): Json<ConnectDbRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut wb = state.workbook.write().await;
    match req.kind.to_lowercase().as_str() {
        "sqlite" => {
            if req.path == ":memory:" {
                wb.add_sqlite_memory(&req.name)
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            } else {
                wb.add_sqlite_connection(&req.name, &req.path)
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            }
        }
        "duckdb" => {
            if req.path == ":memory:" {
                wb.add_duckdb_memory(&req.name)
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            } else {
                wb.add_duckdb_connection(&req.name, &req.path)
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            }
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid engine kind, use 'sqlite' or 'duckdb'".to_string(),
            ))
        }
    }
    Ok(Json(
        serde_json::json!({ "status": "connected", "name": req.name }),
    ))
}

async fn list_connections(State(state): State<AppState>) -> impl IntoResponse {
    let wb = state.workbook.read().await;
    let list: Vec<_> = wb.connection_configs.values().cloned().collect();
    Json(list)
}

async fn list_tables(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wb = state.workbook.read().await;
    let conn = wb.connections.get(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Connection '{}' not found", name),
        )
    })?;
    let tables = conn
        .list_tables()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(tables))
}

async fn get_table_schema(
    State(state): State<AppState>,
    Path((name, table)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wb = state.workbook.read().await;
    let conn = wb.connections.get(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Connection '{}' not found", name),
        )
    })?;
    let schema = conn
        .get_table_schema(&table)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(schema))
}

#[derive(Deserialize)]
struct QueryRequest {
    connection_name: String,
    sql: String,
}

async fn execute_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wb = state.workbook.read().await;
    let conn = wb.connections.get(&req.connection_name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Connection '{}' not found", req.connection_name),
        )
    })?;
    let res = conn
        .execute_read_query(&req.sql, &[])
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(res))
}

async fn analyze_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wb = state.workbook.read().await;
    let conn = wb.connections.get(&req.connection_name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Connection '{}' not found", req.connection_name),
        )
    })?;

    let dialect = match conn.kind() {
        EngineKind::Sqlite => "sqlite",
        EngineKind::DuckDb => "duckdb",
    };
    let schema_fn = |t: &str| conn.get_table_schema(t).ok();
    let report = analyze_lineage(&req.sql, dialect, schema_fn);
    Ok(Json(report))
}

#[derive(Deserialize)]
struct CreateSourceRequest {
    name: String,
    connection_name: String,
    sql: String,
}

async fn create_source(
    State(state): State<AppState>,
    Json(req): Json<CreateSourceRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut wb = state.workbook.write().await;
    let source = wb
        .create_named_source(&req.name, &req.connection_name, &req.sql)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(source))
}

async fn list_sources(State(state): State<AppState>) -> impl IntoResponse {
    let wb = state.workbook.read().await;
    let sources: Vec<_> = wb.named_sources.values().cloned().collect();
    Json(sources)
}

async fn refresh_source(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut wb = state.workbook.write().await;
    wb.refresh_source(&name)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "status": "refreshed", "source": name }),
    ))
}

#[derive(Deserialize)]
struct InsertTableRequest {
    sheet_name: String,
    table_name: String,
    source_name: String,
    row: u32,
    col: u32,
}

async fn insert_table(
    State(state): State<AppState>,
    Json(req): Json<InsertTableRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut wb = state.workbook.write().await;
    let table = wb
        .insert_table(
            &req.sheet_name,
            &req.table_name,
            &req.source_name,
            CellCoord::new(req.row, req.col),
        )
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(table))
}

#[derive(Deserialize)]
struct SetCellRequest {
    sheet_name: String,
    row: u32,
    col: u32,
    raw_input: String,
}

async fn set_cell(
    State(state): State<AppState>,
    Json(req): Json<SetCellRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut wb = state.workbook.write().await;
    wb.set_cell(
        &req.sheet_name,
        CellCoord::new(req.row, req.col),
        req.raw_input,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

async fn get_viewport(
    State(state): State<AppState>,
    Json(req): Json<ViewportRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wb = state.workbook.read().await;
    let vp = wb
        .get_viewport(&req)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(vp))
}

#[derive(Deserialize)]
struct StageEditRequest {
    connection_name: String,
    table_name: String,
    pk_columns: Vec<String>,
    pk_values: Vec<Value>,
    column_name: String,
    original_value: Value,
    proposed_value: Value,
}

async fn stage_edit(
    State(state): State<AppState>,
    Json(req): Json<StageEditRequest>,
) -> impl IntoResponse {
    let mut wb = state.workbook.write().await;
    let id = wb.staged.stage_edit(
        req.connection_name,
        req.table_name,
        req.pk_columns,
        req.pk_values,
        req.column_name,
        req.original_value,
        req.proposed_value,
    );
    Json(serde_json::json!({ "status": "staged", "id": id }))
}

async fn list_staged(State(state): State<AppState>) -> impl IntoResponse {
    let wb = state.workbook.read().await;
    let patches = wb.staged.list_patches();
    Json(patches)
}

async fn discard_staged_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let mut wb = state.workbook.write().await;
    let removed = wb.staged.discard_patch(&id);
    Json(serde_json::json!({ "status": "discarded", "removed": removed }))
}

async fn discard_all_staged(State(state): State<AppState>) -> impl IntoResponse {
    let mut wb = state.workbook.write().await;
    wb.staged.discard_all();
    Json(serde_json::json!({ "status": "all_discarded" }))
}

async fn review_writeback(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wb = state.workbook.read().await;
    let mut items = Vec::new();
    for name in wb
        .staged
        .patches
        .values()
        .map(|p| &p.connection_name)
        .collect::<std::collections::HashSet<_>>()
    {
        let conn = wb.connections.get(name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Target connection '{}' not found", name),
            )
        })?;
        items.extend(wb.staged.review(conn, name));
    }
    items.sort_by_key(|i| i.patch.staged_at);
    Ok(Json(items))
}

#[derive(Deserialize)]
struct ApplyWritebackRequest {
    connection_name: String,
}

async fn apply_writeback(
    State(state): State<AppState>,
    Json(req): Json<ApplyWritebackRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut wb = state.workbook.write().await;
    let rec = wb
        .apply_writeback(&req.connection_name)
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;

    Ok(Json(rec))
}

async fn list_audit_log(State(state): State<AppState>) -> impl IntoResponse {
    let wb = state.workbook.read().await;
    Json(wb.staged.audit_log.clone())
}

async fn list_pivots(State(state): State<AppState>) -> impl IntoResponse {
    let wb = state.workbook.read().await;
    let pivots: Vec<_> = wb.pivots.values().cloned().collect();
    Json(pivots)
}

async fn save_pivot(
    State(state): State<AppState>,
    Json(def): Json<PivotDefinition>,
) -> impl IntoResponse {
    let mut wb = state.workbook.write().await;
    wb.pivots.insert(def.name.clone(), def.clone());
    Json(serde_json::json!({ "status": "saved", "name": def.name }))
}

async fn get_pivot_data(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wb = state.workbook.read().await;
    let res = wb
        .compute_pivot_result(&name)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(res))
}

async fn list_charts(State(state): State<AppState>) -> impl IntoResponse {
    let wb = state.workbook.read().await;
    let charts: Vec<_> = wb.charts.values().cloned().collect();
    Json(charts)
}

async fn save_chart(
    State(state): State<AppState>,
    Json(def): Json<ChartDefinition>,
) -> impl IntoResponse {
    let mut wb = state.workbook.write().await;
    wb.charts.insert(def.id.clone(), def.clone());
    Json(serde_json::json!({ "status": "saved", "id": def.id }))
}

async fn get_chart_data(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let wb = state.workbook.read().await;
    let data = wb
        .compute_chart_data(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(data))
}

async fn run_benchmarks() -> impl IntoResponse {
    let report = run_performance_benchmarks();
    Json(report)
}
