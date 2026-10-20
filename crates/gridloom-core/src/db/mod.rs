pub mod duckdb;
pub mod security;
pub mod sqlite;

use crate::error::CellError;
use crate::value::Value;
pub use duckdb::DuckDbEngine;
use serde::{Deserialize, Serialize};
pub use sqlite::{ColumnMeta, QueryResult, SqliteEngine, TableSchema};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineKind {
    Sqlite,
    DuckDb,
}

pub enum DbConnection {
    Sqlite(SqliteEngine),
    DuckDb(DuckDbEngine),
}

impl DbConnection {
    pub fn open_sqlite<P: AsRef<Path>>(path: P) -> Result<Self, CellError> {
        Ok(DbConnection::Sqlite(SqliteEngine::open(path)?))
    }

    pub fn open_sqlite_memory() -> Result<Self, CellError> {
        Ok(DbConnection::Sqlite(SqliteEngine::open_memory()?))
    }

    pub fn open_duckdb<P: AsRef<Path>>(path: P) -> Result<Self, CellError> {
        Ok(DbConnection::DuckDb(DuckDbEngine::open(path)?))
    }

    pub fn open_duckdb_memory() -> Result<Self, CellError> {
        Ok(DbConnection::DuckDb(DuckDbEngine::open_memory()?))
    }

    pub fn kind(&self) -> EngineKind {
        match self {
            DbConnection::Sqlite(_) => EngineKind::Sqlite,
            DbConnection::DuckDb(_) => EngineKind::DuckDb,
        }
    }

    pub fn list_tables(&self) -> Result<Vec<String>, CellError> {
        match self {
            DbConnection::Sqlite(eng) => eng.list_tables(),
            DbConnection::DuckDb(eng) => eng.list_tables(),
        }
    }

    pub fn get_table_schema(&self, table_name: &str) -> Result<TableSchema, CellError> {
        match self {
            DbConnection::Sqlite(eng) => eng.get_table_schema(table_name),
            DbConnection::DuckDb(eng) => eng.get_table_schema(table_name),
        }
    }

    pub fn execute_read_query(
        &self,
        sql: &str,
        params: &[Value],
    ) -> Result<QueryResult, CellError> {
        match self {
            DbConnection::Sqlite(eng) => eng.execute_read_query(sql, params),
            DbConnection::DuckDb(eng) => eng.execute_read_query(sql, params),
        }
    }

    pub fn update_rows(&self, patches: &[crate::writeback::StagedPatch]) -> Result<(), CellError> {
        match self {
            Self::Sqlite(engine) => engine.update_rows(patches),
            Self::DuckDb(engine) => engine.update_rows(patches),
        }
    }

    pub fn update_row(
        &self,
        table: &str,
        pk_cols: &[String],
        pk_vals: &[Value],
        col_name: &str,
        original_val: &Value,
        proposed_val: &Value,
    ) -> Result<(), CellError> {
        match self {
            DbConnection::Sqlite(eng) => eng.update_row(
                table,
                pk_cols,
                pk_vals,
                col_name,
                original_val,
                proposed_val,
            ),
            DbConnection::DuckDb(eng) => eng.update_row(
                table,
                pk_cols,
                pk_vals,
                col_name,
                original_val,
                proposed_val,
            ),
        }
    }
}
