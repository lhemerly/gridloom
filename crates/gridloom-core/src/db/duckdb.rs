use crate::db::security::validate_read_only_sql;
use crate::db::sqlite::{ColumnMeta, QueryResult, TableSchema};
use crate::error::CellError;
use crate::value::Value;
use chrono::{DateTime, NaiveDate, Utc};
use duckdb::types::{ToSqlOutput, ValueRef};
use duckdb::{params_from_iter, Connection, ToSql};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct DuckDbEngine {
    conn: Mutex<Connection>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl DuckDbEngine {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, CellError> {
        let p = path.as_ref().to_path_buf();
        let conn = Connection::open(&p)
            .map_err(|e| CellError::Sql(format!("Failed to open DuckDB file: {}", e)))?;

        Ok(Self {
            conn: Mutex::new(conn),
            path: p,
        })
    }

    pub fn open_memory() -> Result<Self, CellError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| CellError::Sql(format!("Failed to open in-memory DuckDB: {}", e)))?;

        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        })
    }

    pub fn list_tables(&self) -> Result<Vec<String>, CellError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT table_name FROM information_schema.tables WHERE table_schema='main' ORDER BY table_name")
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
        let pragma_sql = format!("PRAGMA table_info('{}')", table_name.replace('\'', "''"));
        let mut stmt = conn
            .prepare(&pragma_sql)
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let data_type: String = row.get(2)?;
                let not_null: bool = row.get(3)?;
                let pk: bool = row.get(5)?;
                Ok(ColumnMeta {
                    name,
                    data_type,
                    is_primary_key: pk,
                    is_nullable: !not_null,
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
        validate_read_only_sql(sql, "duckdb")?;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let duck_params: Vec<DuckDbParam> = params.iter().map(|p| DuckDbParam(p.clone())).collect();
        let mut rows_iter = stmt
            .query(params_from_iter(duck_params.iter()))
            .map_err(|e| CellError::Sql(e.to_string()))?;

        let mut columns = Vec::new();
        let mut rows = Vec::new();

        {
            let st = rows_iter
                .as_ref()
                .ok_or_else(|| CellError::Sql("Missing query metadata".into()))?;
            let col_count = st.column_count();
            for idx in 0..col_count {
                let col_name = st
                    .column_name(idx)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|_| format!("col_{}", idx));
                columns.push(ColumnMeta {
                    name: col_name,
                    data_type: "DUCKDB_ANY".to_string(),
                    is_primary_key: false,
                    is_nullable: true,
                });
            }
        }

        while let Some(row) = rows_iter
            .next()
            .map_err(|e| CellError::Sql(e.to_string()))?
        {
            let mut row_vals = Vec::new();
            for idx in 0..columns.len() {
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

    /// Transactional update with optimistic concurrency check
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
            .map_err(|e| CellError::Sql(format!("Failed to start DuckDB transaction: {}", e)))?;

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
            .map_err(|e| CellError::Sql(format!("Failed to commit DuckDB transaction: {}", e)))?;

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
        tx: &duckdb::Connection,
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
        let mut where_clauses = Vec::new();
        let mut check_params: Vec<DuckDbParam> = Vec::new();

        for (col, val) in pk_cols.iter().zip(pk_vals.iter()) {
            where_clauses.push(format!("\"{}\" = ?", col.replace('"', "\"\"")));
            check_params.push(DuckDbParam(val.clone()));
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
                    "Concurrency conflict: row was deleted in DuckDB".to_string(),
                ));
            }
        };

        if !current.is_not_distinct_from(original_val) {
            return Err(CellError::Sql(format!(
                "Concurrency conflict: column '{}' was modified externally (expected '{}', found '{}')",
                col_name, original_val, current
            )));
        }

        let update_sql = format!(
            "UPDATE \"{}\" SET \"{}\" = ? WHERE {}",
            table.replace('"', "\"\""),
            col_name.replace('"', "\"\""),
            where_clauses.join(" AND ")
        );

        let mut update_params = Vec::new();
        update_params.push(DuckDbParam(proposed_val.clone()));
        update_params.extend(check_params);

        let updated_rows = tx
            .execute(&update_sql, params_from_iter(update_params.iter()))
            .map_err(|e| CellError::Sql(format!("DuckDB update failed: {}", e)))?;

        if updated_rows != 1 {
            return Err(CellError::Sql(
                "Update must affect exactly one row".to_string(),
            ));
        }

        Ok(())
    }
}

pub struct DuckDbParam(pub Value);

impl ToSql for DuckDbParam {
    fn to_sql(&self) -> duckdb::Result<ToSqlOutput<'_>> {
        match &self.0 {
            Value::Null => Ok(ToSqlOutput::from(duckdb::types::Null)),
            Value::Blank => Ok(ToSqlOutput::from(duckdb::types::Null)),
            Value::EmptyText => Ok(ToSqlOutput::from("")),
            Value::Boolean(b) => Ok(ToSqlOutput::from(*b)),
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
        ValueRef::Boolean(b) => Value::Boolean(b),
        ValueRef::TinyInt(i) => Value::Integer(i as i64),
        ValueRef::SmallInt(i) => Value::Integer(i as i64),
        ValueRef::Int(i) => Value::Integer(i as i64),
        ValueRef::BigInt(i) => Value::Integer(i),
        ValueRef::HugeInt(h) => {
            let s = h.to_string();
            if let Ok(d) = Decimal::from_str_exact(&s) {
                Value::Decimal(d)
            } else {
                Value::Text(s)
            }
        }
        ValueRef::Float(f) => {
            if let Some(d) = Decimal::from_f64(f as f64) {
                Value::Decimal(d)
            } else {
                Value::Float(f as f64)
            }
        }
        ValueRef::Double(d) => {
            if let Some(dec) = Decimal::from_f64(d) {
                Value::Decimal(dec)
            } else {
                Value::Float(d)
            }
        }
        ValueRef::Decimal(d) => {
            let s = d.to_string();
            if let Ok(dec) = Decimal::from_str_exact(&s) {
                Value::Decimal(dec)
            } else {
                Value::Text(s)
            }
        }
        ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes).to_string();
            if let Ok(date) = NaiveDate::parse_from_str(&text, "%Y-%m-%d") {
                Value::Date(date)
            } else if let Ok(ts) = DateTime::parse_from_rfc3339(&text) {
                Value::Timestamp(ts.with_timezone(&Utc))
            } else {
                Value::Text(text)
            }
        }
        ValueRef::Date32(days) => {
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let d = epoch + chrono::Duration::days(days as i64);
            Value::Date(d)
        }
        ValueRef::Time64(_unit, _micros) => Value::Text("<Time>".to_string()),
        ValueRef::Timestamp(_unit, micros) => {
            let epoch =
                DateTime::from_timestamp(micros / 1_000_000, ((micros % 1_000_000) * 1000) as u32);
            if let Some(ts) = epoch {
                Value::Timestamp(ts)
            } else {
                Value::Text(format!("<Timestamp {}>", micros))
            }
        }
        ValueRef::Blob(bytes) => Value::Text(format!("<Blob {} bytes>", bytes.len())),
        _ => Value::Null,
    }
}
