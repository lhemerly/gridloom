use crate::db::sqlite::TableSchema;
use serde::{Deserialize, Serialize};
use sqlparser::ast::{
    Expr, SelectItem, SetExpr, Statement, TableFactor,
};
use sqlparser::dialect::{DuckDbDialect, GenericDialect, SQLiteDialect};
use sqlparser::parser::Parser;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageReport {
    pub is_editable: bool,
    pub base_table: Option<String>,
    pub primary_keys: Vec<String>,
    pub editable_columns: Vec<String>,
    pub read_only_columns: Vec<String>,
    pub read_only_reasons: Vec<String>,
}

pub fn analyze_lineage<F>(sql: &str, dialect_name: &str, schema_lookup: F) -> LineageReport
where
    F: Fn(&str) -> Option<TableSchema>,
{
    let mut reasons = Vec::new();
    let dialect: Box<dyn sqlparser::dialect::Dialect> = match dialect_name.to_lowercase().as_str() {
        "sqlite" => Box::new(SQLiteDialect {}),
        "duckdb" => Box::new(DuckDbDialect {}),
        _ => Box::new(GenericDialect {}),
    };

    let ast = match Parser::parse_sql(dialect.as_ref(), sql) {
        Ok(statements) => statements,
        Err(e) => {
            reasons.push(format!("Failed to parse SQL: {}", e));
            return non_editable(reasons);
        }
    };

    if ast.len() != 1 {
        reasons.push("Only single SELECT queries can have editable lineage".to_string());
        return non_editable(reasons);
    }

    let query = match &ast[0] {
        Statement::Query(q) => q,
        _ => {
            reasons.push("Statement is not a query".to_string());
            return non_editable(reasons);
        }
    };

    let select = match query.body.as_ref() {
        SetExpr::Select(s) => s,
        SetExpr::SetOperation { .. } => {
            reasons.push("UNION, INTERSECT, and EXCEPT queries are read-only".to_string());
            return non_editable(reasons);
        }
        _ => {
            reasons.push("Complex query structure is read-only".to_string());
            return non_editable(reasons);
        }
    };

    // Check for DISTINCT
    if select.distinct.is_some() {
        reasons.push("DISTINCT queries cannot be safely mapped to unique base rows".to_string());
    }

    // Check for GROUP BY / HAVING
    let has_group_by = match &select.group_by {
        sqlparser::ast::GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
        sqlparser::ast::GroupByExpr::All(_) => true,
    };
    if has_group_by || select.having.is_some() {
        reasons.push("Aggregations and GROUP BY queries are read-only".to_string());
    }

    // Check FROM table
    if select.from.is_empty() {
        reasons.push("Query has no FROM table".to_string());
        return non_editable(reasons);
    }
    if select.from.len() > 1 {
        reasons.push("Multi-table queries (joins or cartesian products) are read-only".to_string());
        return non_editable(reasons);
    }

    let table_ref = &select.from[0];
    if !table_ref.joins.is_empty() {
        reasons.push("Queries with JOINs are read-only in MVP".to_string());
        return non_editable(reasons);
    }

    let base_table_name = match &table_ref.relation {
        TableFactor::Table { name, .. } => name.to_string(),
        _ => {
            reasons.push("Subqueries or table functions in FROM clause are read-only".to_string());
            return non_editable(reasons);
        }
    };

    // Lookup base table schema
    let schema = match schema_lookup(&base_table_name) {
        Some(s) => s,
        None => {
            reasons.push(format!("Base table '{}' schema could not be verified", base_table_name));
            return non_editable(reasons);
        }
    };

    if schema.primary_keys.is_empty() {
        reasons.push(format!(
            "Base table '{}' has no declared primary key for row identity",
            base_table_name
        ));
    }

    // Analyze projections (columns)
    let mut editable_columns = Vec::new();
    let mut read_only_columns = Vec::new();
    let mut selected_pk_count = 0;

    let base_col_set: HashMap<String, bool> = schema
        .columns
        .iter()
        .map(|c| (c.name.to_lowercase(), c.is_primary_key))
        .collect();

    for item in &select.projection {
        match item {
            SelectItem::Wildcard(_) => {
                // All columns of base table
                for col in &schema.columns {
                    editable_columns.push(col.name.clone());
                    if col.is_primary_key {
                        selected_pk_count += 1;
                    }
                }
            }
            SelectItem::UnnamedExpr(expr) => match expr {
                Expr::Identifier(ident) => {
                    let col_name = ident.value.clone();
                    if let Some(&is_pk) = base_col_set.get(&col_name.to_lowercase()) {
                        editable_columns.push(col_name);
                        if is_pk {
                            selected_pk_count += 1;
                        }
                    } else {
                        read_only_columns.push(col_name);
                    }
                }
                Expr::CompoundIdentifier(parts) => {
                    if parts.len() == 2 {
                        let col_name = parts[1].value.clone();
                        if let Some(&is_pk) = base_col_set.get(&col_name.to_lowercase()) {
                            editable_columns.push(col_name);
                            if is_pk {
                                selected_pk_count += 1;
                            }
                        } else {
                            read_only_columns.push(col_name);
                        }
                    } else {
                        read_only_columns.push(parts.iter().map(|p| p.value.as_str()).collect::<Vec<_>>().join("."));
                    }
                }
                _ => {
                    read_only_columns.push(expr.to_string());
                }
            },
            SelectItem::ExprWithAlias { expr, alias } => {
                let alias_name = alias.value.clone();
                match expr {
                    Expr::Identifier(ident) => {
                        let col_name = ident.value.clone();
                        if let Some(&is_pk) = base_col_set.get(&col_name.to_lowercase()) {
                            editable_columns.push(alias_name);
                            if is_pk {
                                selected_pk_count += 1;
                            }
                        } else {
                            read_only_columns.push(alias_name);
                        }
                    }
                    _ => {
                        read_only_columns.push(alias_name);
                    }
                }
            }
            _ => {}
        }
    }

    if !schema.primary_keys.is_empty() && selected_pk_count < schema.primary_keys.len() {
        // Missing PK in projection - note that in Gridloom, PK can be retrieved as hidden identity metadata if needed
    }

    let is_editable = reasons.is_empty() && !schema.primary_keys.is_empty() && !editable_columns.is_empty();

    LineageReport {
        is_editable,
        base_table: Some(base_table_name),
        primary_keys: schema.primary_keys,
        editable_columns,
        read_only_columns,
        read_only_reasons: reasons,
    }
}

fn non_editable(reasons: Vec<String>) -> LineageReport {
    LineageReport {
        is_editable: false,
        base_table: None,
        primary_keys: Vec::new(),
        editable_columns: Vec::new(),
        read_only_columns: Vec::new(),
        read_only_reasons: reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sqlite::ColumnMeta;

    fn mock_schema(name: &str) -> Option<TableSchema> {
        if name.eq_ignore_ascii_case("invoices") {
            Some(TableSchema {
                table_name: "invoices".to_string(),
                columns: vec![
                    ColumnMeta {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        is_primary_key: true,
                        is_nullable: false,
                    },
                    ColumnMeta {
                        name: "amount".to_string(),
                        data_type: "DECIMAL".to_string(),
                        is_primary_key: false,
                        is_nullable: false,
                    },
                    ColumnMeta {
                        name: "status".to_string(),
                        data_type: "TEXT".to_string(),
                        is_primary_key: false,
                        is_nullable: true,
                    },
                ],
                primary_keys: vec!["id".to_string()],
            })
        } else {
            None
        }
    }

    #[test]
    fn test_editable_query() {
        let rep = analyze_lineage("SELECT id, amount, status FROM invoices", "sqlite", mock_schema);
        assert!(rep.is_editable);
        assert_eq!(rep.base_table, Some("invoices".to_string()));
        assert_eq!(rep.primary_keys, vec!["id"]);
        assert_eq!(rep.editable_columns.len(), 3);
    }

    #[test]
    fn test_non_editable_aggregate() {
        let rep = analyze_lineage("SELECT status, SUM(amount) FROM invoices GROUP BY status", "sqlite", mock_schema);
        assert!(!rep.is_editable);
        assert!(rep.read_only_reasons.iter().any(|r| r.contains("GROUP BY")));
    }

    #[test]
    fn test_non_editable_join() {
        let rep = analyze_lineage(
            "SELECT i.id, c.name FROM invoices i JOIN customers c ON i.customer_id = c.id",
            "sqlite",
            mock_schema,
        );
        assert!(!rep.is_editable);
        assert!(rep.read_only_reasons.iter().any(|r| r.contains("JOIN")));
    }
}
