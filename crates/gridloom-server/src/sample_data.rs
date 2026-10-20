use rusqlite::Connection as RusqliteConn;
use std::fs;
use std::path::{Path, PathBuf};

pub fn ensure_sample_databases() -> anyhow::Result<(PathBuf, PathBuf)> {
    let fixtures_dir = Path::new("fixtures");
    fs::create_dir_all(fixtures_dir)?;

    let sqlite_path = fixtures_dir.join("sample_sales.sqlite");
    if !sqlite_path.exists() {
        create_sample_sqlite(&sqlite_path)?;
    }

    let duckdb_path = fixtures_dir.join("sample_analytics.duckdb");
    if !duckdb_path.exists() {
        create_sample_duckdb(&duckdb_path)?;
    }

    Ok((sqlite_path, duckdb_path))
}

fn create_sample_sqlite(path: &Path) -> anyhow::Result<()> {
    let conn = RusqliteConn::open(path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS invoices (
            id INTEGER PRIMARY KEY,
            customer_name TEXT NOT NULL,
            issued_on TEXT NOT NULL,
            amount REAL NOT NULL,
            status TEXT NOT NULL
        )",
        [],
    )?;

    let count: i64 = conn.query_row("SELECT count(*) FROM invoices", [], |r| r.get(0))?;
    if count == 0 {
        let mut stmt = conn.prepare(
            "INSERT INTO invoices (id, customer_name, issued_on, amount, status) VALUES (?, ?, ?, ?, ?)",
        )?;
        let sample_records = [
            (1, "Acme Logistics", "2026-09-01", 14500.00, "Paid"),
            (2, "Stark Industries", "2026-09-05", 89400.50, "Approved"),
            (3, "Wayne Enterprises", "2026-09-12", 45200.00, "Pending"),
            (4, "Cyberdyne Systems", "2026-09-18", 12300.75, "Pending"),
            (5, "Umbrella Corp", "2026-09-22", 67800.00, "Paid"),
            (6, "Initech", "2026-09-28", 3200.00, "Paid"),
            (7, "Hooli Corp", "2026-10-02", 98500.20, "Approved"),
            (8, "Pied Piper", "2026-10-08", 24100.00, "Pending"),
            (9, "Massive Dynamic", "2026-10-14", 51200.00, "Paid"),
            (10, "Tyrell Corp", "2026-10-19", 112000.00, "Pending"),
        ];

        for r in sample_records {
            stmt.execute(rusqlite::params![r.0, r.1, r.2, r.3, r.4])?;
        }
    }
    Ok(())
}

fn create_sample_duckdb(path: &Path) -> anyhow::Result<()> {
    let conn = duckdb::Connection::open(path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS server_metrics (
            id BIGINT PRIMARY KEY,
            region VARCHAR NOT NULL,
            recorded_at VARCHAR NOT NULL,
            requests_per_sec DOUBLE NOT NULL,
            p99_latency_ms DOUBLE NOT NULL,
            error_rate DOUBLE NOT NULL
        )",
        [],
    )?;

    let count: i64 = conn.query_row("SELECT count(*) FROM server_metrics", [], |r| r.get(0))?;
    if count == 0 {
        let mut stmt = conn.prepare(
            "INSERT INTO server_metrics VALUES (?, ?, ?, ?, ?, ?)",
        )?;
        let sample_records = [
            (101, "us-east-1", "2026-10-19 18:00:00", 12500.0, 4.2, 0.001),
            (102, "us-west-2", "2026-10-19 18:00:00", 8900.0, 3.8, 0.0005),
            (103, "eu-central-1", "2026-10-19 18:00:00", 14200.0, 5.1, 0.002),
            (104, "ap-northeast-1", "2026-10-19 18:00:00", 6300.0, 6.4, 0.0015),
            (105, "sa-east-1", "2026-10-19 18:00:00", 4100.0, 7.2, 0.003),
            (106, "us-east-1", "2026-10-19 19:00:00", 13100.0, 4.5, 0.0012),
            (107, "us-west-2", "2026-10-19 19:00:00", 9400.0, 3.9, 0.0004),
            (108, "eu-central-1", "2026-10-19 19:00:00", 15800.0, 5.3, 0.0018),
            (109, "ap-northeast-1", "2026-10-19 19:00:00", 6700.0, 6.1, 0.0011),
            (110, "sa-east-1", "2026-10-19 19:00:00", 4400.0, 7.0, 0.0028),
        ];

        for r in sample_records {
            stmt.execute(duckdb::params![r.0, r.1, r.2, r.3, r.4, r.5])?;
        }
    }
    Ok(())
}
