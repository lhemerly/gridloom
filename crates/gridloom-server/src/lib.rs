pub mod benchmark;
pub mod routes;
pub mod sample_data;
pub mod state;

use gridloom_core::workbook::Workbook;
use sample_data::ensure_sample_databases;
use state::AppState;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

pub async fn build_app(open_samples: bool) -> anyhow::Result<axum::Router> {
    let mut workbook = Workbook::new();

    if open_samples {
        if let Ok((sqlite_p, duckdb_p)) = ensure_sample_databases() {
            let _ = workbook.add_sqlite_connection("SalesDB", &sqlite_p);
            let _ = workbook.add_duckdb_connection("AnalyticsDB", &duckdb_p);

            // Pre-seed sample tables and queries
            let _ = workbook.create_named_source(
                "Invoices",
                "SalesDB",
                "SELECT id, customer_name, issued_on, amount, status FROM invoices",
            );

            let _ = workbook.insert_table(
                "Sheet1",
                "Invoices",
                "Invoices",
                gridloom_core::value::CellCoord::new(1, 1), // B2
            );
        }
    }

    let state = AppState::new(workbook);
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut router = routes::create_router(state).layer(cors);

    let dist_path = std::path::Path::new("frontend/dist");
    if dist_path.exists() {
        router = router.fallback_service(tower_http::services::ServeDir::new(dist_path));
    }

    Ok(router)
}

pub async fn start_server(port: u16, open_samples: bool) -> anyhow::Result<(u16, tokio::task::JoinHandle<()>)> {
    let app = build_app(open_samples).await?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_port = listener.local_addr()?.port();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Ok((actual_port, handle))
}
