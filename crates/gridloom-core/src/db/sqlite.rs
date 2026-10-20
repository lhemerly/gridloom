use crate::db::security::validate_read_only_sql;
use crate::error::CellError;
use crate::value::Value;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::types::{ToSql, ToSqlOutput, ValueRef};
use rusqlite::{params_from_iter, Connection, OpenFlags};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
    pub is_nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub table_name: String,
    pub columns: Vec<ColumnMeta>,
    pub primary_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<Value>>,
    pub total_rows: usize,
}

pub struct SqliteEngine {
    conn: Mutex<Connection>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl SqliteEngine {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, CellError> {
        let p = path.as_ref().to_path_buf();
        let conn = Connection::open_with_flags(
            &p,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .map_err(|e| CellError::Sql(format!("Failed to open SQLite database: {}", e)))?;

        Ok(Self {
            conn: Mutex::new(conn),
            path: p,
        })
    }

    pub fn open_memory() -> Result<Self, CellError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| CellError::Sql(format!("Failed to open in-memory SQLite: {}", e)))?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        })
    }

    pub fn list_tables(&self) -> Result<Vec<String>, CellError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let mut tables = Vec::new();
        for r in rows {
            if let Ok(name) = r {
                tables.push(name);
            }
        }
        Ok(tables)
    }

    pub fn get_table_schema(&self, table_name: &str) -> Result<TableSchema, CellError> {
        let conn = self.conn.lock().unwrap();
        let pragma_sql = format!("PRAGMA table_info(\"{}\")", table_name.replace('"', "\"\""));
        let mut stmt = conn
            .prepare(&pragma_sql)
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let data_type: String = row.get(2)?;
                let not_null: i64 = row.get(3)?;
                let pk: i64 = row.get(5)?;
                Ok(ColumnMeta {
                    name,
                    data_type,
                    is_primary_key: pk > 0,
                    is_nullable: not_null == 0,
                })
            })
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();
        for r in rows {
            let col = r.map_err(|e| CellError::Sql(e.to_string()))?;
            if col.is_primary_key {
                primary_keys.push(col.name.clone());
            }
            columns.push(col);
        }

        Ok(TableSchema {
            table_name: table_name.to_string(),
            columns,
            primary_keys,
        })
    }

    pub fn execute_read_query(
        &self,
        sql: &str,
        params: &[Value],
    ) -> Result<QueryResult, CellError> {
        validate_read_only_sql(sql, "sqlite")?;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let col_count = stmt.column_count();
        let mut columns = Vec::new();
        for idx in 0..col_count {
            let col_name = stmt.column_name(idx).unwrap_or("?").to_string();
            columns.push(ColumnMeta {
                name: col_name,
                data_type: "SQLITE_ANY".to_string(),
                is_primary_key: false,
                is_nullable: true,
            });
        }

        let sqlite_params: Vec<SqliteParam> =
            params.iter().map(|p| SqliteParam(p.clone())).collect();
        let mut rows_iter = stmt
            .query(params_from_iter(sqlite_params.iter()))
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let mut rows = Vec::new();
        while let Some(row) = rows_iter
            .next()
            .map_err(|e| CellError::Sql(e.to_string()))?
        {
            let mut row_vals = Vec::new();
            for idx in 0..col_count {
                let val_ref = row
                    .get_ref(idx)
                    .map_err(|e| CellError::Sql(e.to_string()))?;
                let val = decode_value(val_ref);
                row_vals.push(val);
            }
            rows.push(row_vals);
        }

        let total_rows = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            total_rows,
        })
    }

    /// Transactional writeback for qualified row update with optimistic check
    pub fn update_row(
        &self,
        table: &str,
        pk_cols: &[String],
        pk_vals: &[Value],
        col_name: &str,
        original_val: &Value,
        proposed_val: &Value,
    ) -> Result<(), CellError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| CellError::Sql(format!("Failed to start transaction: {}", e)))?;

        Self::update_in_transaction(
            &tx,
            table,
            pk_cols,
            pk_vals,
            col_name,
            original_val,
            proposed_val,
        )?;

        tx.commit()
            .map_err(|e| CellError::Sql(format!("Failed to commit update transaction: {}", e)))?;

        Ok(())
    }
    /// Holds the connection lock and one transaction for the entire changeset.
    pub fn update_rows(&self, patches: &[crate::writeback::StagedPatch]) -> Result<(), CellError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| CellError::Sql(e.to_string()))?;
        for patch in patches {
            Self::update_in_transaction(
                &tx,
                &patch.table_name,
                &patch.pk_columns,
                &patch.pk_values,
                &patch.column_name,
                &patch.original_value,
                &patch.proposed_value,
            )?;
        }
        tx.commit().map_err(|e| CellError::Sql(e.to_string()))
    }

    fn update_in_transaction(
        tx: &rusqlite::Connection,
        table: &str,
        pk_cols: &[String],
        pk_vals: &[Value],
        col_name: &str,
        original_val: &Value,
        proposed_val: &Value,
    ) -> Result<(), CellError> {
        if pk_cols.is_empty()
            || pk_cols.len() != pk_vals.len()
            || pk_vals.iter().any(|v| v.is_null())
        {
            return Err(CellError::Sql(
                "A complete non-NULL primary key is required".into(),
            ));
        }
        // 1. Verify row still exists and original value matches (NULL-safe optimistic concurrency)
        let mut where_clauses = Vec::new();
        let mut check_params: Vec<SqliteParam> = Vec::new();

        for (col, val) in pk_cols.iter().zip(pk_vals.iter()) {
            where_clauses.push(format!("\"{}\" = ?", col.replace('"', "\"\"")));
            check_params.push(SqliteParam(val.clone()));
        }

        let check_sql = format!(
            "SELECT \"{}\" FROM \"{}\" WHERE {}",
            col_name.replace('"', "\"\""),
            table.replace('"', "\"\""),
            where_clauses.join(" AND ")
        );

        let current_val: Option<Value> = {
            let mut stmt = tx
                .prepare(&check_sql)
                .map_err(|e| CellError::Sql(e.to_string()))?;
            let mut rows = stmt
                .query(params_from_iter(check_params.iter()))
                .map_err(|e| CellError::Sql(e.to_string()))?;

            if let Some(r) = rows.next().map_err(|e| CellError::Sql(e.to_string()))? {
                let vref = r.get_ref(0).map_err(|e| CellError::Sql(e.to_string()))?;
                Some(decode_value(vref))
            } else {
                None
            }
        };

        let current = match current_val {
            Some(c) => c,
            None => {
                return Err(CellError::Sql(
                    "Concurrency conflict: row was deleted in database".to_string(),
                ));
            }
        };

        if !current.is_not_distinct_from(original_val) {
            return Err(CellError::Sql(format!(
                "Concurrency conflict: column '{}' was modified externally (expected '{}', found '{}')",
                col_name, original_val, current
            )));
        }

        // 2. Perform update
        let update_sql = format!(
            "UPDATE \"{}\" SET \"{}\" = ? WHERE {}",
            table.replace('"', "\"\""),
            col_name.replace('"', "\"\""),
            where_clauses.join(" AND ")
        );

        let mut update_params = Vec::new();
        update_params.push(SqliteParam(proposed_val.clone()));
        update_params.extend(check_params);

        let updated_rows = tx
            .execute(&update_sql, params_from_iter(update_params.iter()))
            .map_err(|e| CellError::Sql(format!("Update failed: {}", e)))?;

        if updated_rows != 1 {
            return Err(CellError::Sql(
                "Update must affect exactly one row".to_string(),
            ));
        }

        Ok(())
    }
}

pub struct SqliteParam(pub Value);

impl ToSql for SqliteParam {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match &self.0 {
            Value::Null => Ok(ToSqlOutput::from(rusqlite::types::Null)),
            Value::Blank => Ok(ToSqlOutput::from(rusqlite::types::Null)),
            Value::EmptyText => Ok(ToSqlOutput::from("")),
            Value::Boolean(b) => Ok(ToSqlOutput::from(*b as i64)),
            Value::Integer(i) => Ok(ToSqlOutput::from(*i)),
            Value::Decimal(d) => {
                if let Some(f) = d.to_f64() {
                    Ok(ToSqlOutput::from(f))
                } else {
                    Ok(ToSqlOutput::from(d.to_string()))
                }
            }
            Value::Float(f) => Ok(ToSqlOutput::from(*f)),
            Value::Text(s) => Ok(ToSqlOutput::from(s.as_str())),
            Value::Date(d) => Ok(ToSqlOutput::from(d.format("%Y-%m-%d").to_string())),
            Value::Timestamp(ts) => Ok(ToSqlOutput::from(ts.to_rfc3339())),
            Value::Error(e) => Ok(ToSqlOutput::from(e.to_string())),
            Value::Array(_) => Ok(ToSqlOutput::from(self.0.to_string_repr())),
        }
    }
}

fn decode_value(val_ref: ValueRef<'_>) -> Value {
    match val_ref {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::Integer(i),
        ValueRef::Real(r) => {
            if let Some(d) = Decimal::from_f64(r) {
                Value::Decimal(d)
            } else {
                Value::Float(r)
            }
        }
        ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes).to_string();
            // Check if ISO date
            if let Ok(d) = NaiveDate::parse_from_str(&text, "%Y-%m-%d") {
                Value::Date(d)
            } else if let Ok(ts) = DateTime::parse_from_rfc3339(&text) {
                Value::Timestamp(ts.with_timezone(&Utc))
            } else {
                Value::Text(text)
            }
        }
        ValueRef::Blob(bytes) => Value::Text(format!("<Blob {} bytes>", bytes.len())),
    }
}
