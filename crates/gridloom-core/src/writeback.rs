use crate::db::DbConnection;
use crate::error::CellError;
use crate::value::Value;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedPatch {
    pub id: Uuid,
    pub connection_name: String,
    pub table_name: String,
    pub pk_columns: Vec<String>,
    pub pk_values: Vec<Value>,
    pub column_name: String,
    pub original_value: Value,
    pub proposed_value: Value,
    pub staged_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictStatus {
    NoConflict,
    RowDeletedExternally,
    CheckFailed { message: String },
    RowModifiedExternally { current_value: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritebackReviewItem {
    pub patch: StagedPatch,
    pub conflict_status: ConflictStatus,
    pub can_apply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditStatus {
    AppliedSuccessfully,
    RolledBackConflict,
    RolledBackError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: Uuid,
    pub connection_name: String,
    pub table_name: String,
    pub applied_at: DateTime<Utc>,
    pub patch_count: usize,
    pub status: AuditStatus,
    pub error_message: Option<String>,
    pub patches: Vec<StagedPatch>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StagedRepository {
    pub patches: HashMap<Uuid, StagedPatch>,
    pub audit_log: Vec<AuditRecord>,
}

impl StagedRepository {
    pub fn new() -> Self {
        Self {
            patches: HashMap::new(),
            audit_log: Vec::new(),
        }
    }

    pub fn stage_edit(
        &mut self,
        connection_name: String,
        table_name: String,
        pk_columns: Vec<String>,
        pk_values: Vec<Value>,
        column_name: String,
        original_value: Value,
        proposed_value: Value,
    ) -> Uuid {
        // If there's already an existing patch for this exact row and column, replace it
        let existing_id = self.patches.iter().find_map(|(id, p)| {
            if p.connection_name == connection_name
                && p.table_name == table_name
                && p.pk_columns == pk_columns
                && p.pk_values.len() == pk_values.len()
                && p.pk_values
                    .iter()
                    .zip(&pk_values)
                    .all(|(a, b)| a.is_not_distinct_from(b))
                && p.column_name == column_name
            {
                Some(*id)
            } else {
                None
            }
        });

        if let Some(id) = existing_id {
            if let Some(p) = self.patches.get_mut(&id) {
                p.original_value = original_value;
                p.proposed_value = proposed_value;
                p.staged_at = Utc::now();
                return id;
            }
        }

        let id = Uuid::new_v4();
        let patch = StagedPatch {
            id,
            connection_name,
            table_name,
            pk_columns,
            pk_values,
            column_name,
            original_value,
            proposed_value,
            staged_at: Utc::now(),
        };
        self.patches.insert(id, patch);
        id
    }

    pub fn discard_patch(&mut self, id: &Uuid) -> bool {
        self.patches.remove(id).is_some()
    }

    pub fn discard_all(&mut self) {
        self.patches.clear();
    }

    pub fn list_patches(&self) -> Vec<StagedPatch> {
        let mut list: Vec<StagedPatch> = self.patches.values().cloned().collect();
        list.sort_by_key(|p| p.staged_at);
        list
    }

    /// Check conflicts against the current database state
    pub fn review(&self, conn: &DbConnection, connection_name: &str) -> Vec<WritebackReviewItem> {
        let mut review_items = Vec::new();

        for patch in self
            .patches
            .values()
            .filter(|p| p.connection_name == connection_name)
        {
            let conflict_status = self.check_patch_conflict(conn, patch);
            let can_apply = conflict_status == ConflictStatus::NoConflict;
            review_items.push(WritebackReviewItem {
                patch: patch.clone(),
                conflict_status,
                can_apply,
            });
        }

        review_items.sort_by_key(|i| i.patch.staged_at);
        review_items
    }

    fn check_patch_conflict(&self, conn: &DbConnection, patch: &StagedPatch) -> ConflictStatus {
        if patch.pk_columns.is_empty() || patch.pk_columns.len() != patch.pk_values.len() {
            return ConflictStatus::CheckFailed {
                message: "A complete primary key is required".into(),
            };
        }
        let mut where_parts = Vec::new();
        let mut params = Vec::new();
        for (col, val) in patch.pk_columns.iter().zip(patch.pk_values.iter()) {
            where_parts.push(format!("\"{}\" = ?", col.replace('"', "\"\"")));
            params.push(val.clone());
        }

        let sql = format!(
            "SELECT \"{}\" FROM \"{}\" WHERE {}",
            patch.column_name.replace('"', "\"\""),
            patch.table_name.replace('"', "\"\""),
            where_parts.join(" AND ")
        );

        match conn.execute_read_query(&sql, &params) {
            Ok(res) => {
                if res.rows.is_empty() {
                    ConflictStatus::RowDeletedExternally
                } else {
                    let current = res.rows[0].get(0).cloned().unwrap_or(Value::Null);
                    if current.is_not_distinct_from(&patch.original_value) {
                        ConflictStatus::NoConflict
                    } else {
                        ConflictStatus::RowModifiedExternally {
                            current_value: current,
                        }
                    }
                }
            }
            Err(e) => ConflictStatus::CheckFailed {
                message: e.to_string(),
            },
        }
    }

    /// Atomically applies all staged patches targeting a connection in a single transaction.
    /// If any conflict or error occurs, rolls back completely.
    pub fn apply_writeback(
        &mut self,
        conn: &DbConnection,
        connection_name: &str,
    ) -> Result<AuditRecord, CellError> {
        let patches_to_apply: Vec<StagedPatch> = self
            .list_patches()
            .into_iter()
            .filter(|p| p.connection_name == connection_name)
            .collect();

        if patches_to_apply.is_empty() {
            return Err(CellError::Sql("No staged patches to apply".to_string()));
        }

        let first_table = patches_to_apply[0].table_name.clone();

        // 1. Pre-flight conflict check
        for patch in &patches_to_apply {
            let conflict = self.check_patch_conflict(conn, patch);
            if conflict != ConflictStatus::NoConflict {
                let rec = AuditRecord {
                    id: Uuid::new_v4(),
                    connection_name: connection_name.to_string(),
                    table_name: first_table.clone(),
                    applied_at: Utc::now(),
                    patch_count: patches_to_apply.len(),
                    status: if matches!(&conflict, ConflictStatus::CheckFailed { .. }) {
                        AuditStatus::RolledBackError
                    } else {
                        AuditStatus::RolledBackConflict
                    },
                    error_message: Some(format!(
                        "Pre-flight conflict check failed: {:?}",
                        conflict
                    )),
                    patches: patches_to_apply.clone(),
                };
                self.audit_log.push(rec.clone());
                return Err(CellError::Sql(format!(
                    "Writeback rejected due to conflict: {:?}",
                    conflict
                )));
            }
        }

        // The engines perform every optimistic check and update under the same transaction.
        if let Err(e) = conn.update_rows(&patches_to_apply) {
            self.audit_log.push(AuditRecord {
                id: Uuid::new_v4(),
                connection_name: connection_name.to_string(),
                table_name: first_table.clone(),
                applied_at: Utc::now(),
                patch_count: patches_to_apply.len(),
                status: AuditStatus::RolledBackError,
                error_message: Some(e.to_string()),
                patches: patches_to_apply.clone(),
            });
            return Err(CellError::Sql(format!(
                "Writeback failed and rolled back: {}",
                e
            )));
        }

        // 3. Remove applied patches from staged repository
        for patch in &patches_to_apply {
            self.patches.remove(&patch.id);
        }

        let record = AuditRecord {
            id: Uuid::new_v4(),
            connection_name: connection_name.to_string(),
            table_name: first_table,
            applied_at: Utc::now(),
            patch_count: patches_to_apply.len(),
            status: AuditStatus::AppliedSuccessfully,
            error_message: None,
            patches: patches_to_apply,
        };
        self.audit_log.push(record.clone());

        Ok(record)
    }
}
