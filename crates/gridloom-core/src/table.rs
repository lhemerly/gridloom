use crate::ast::StructuredRef;
use crate::db::sqlite::QueryResult;
use crate::error::CellError;
use crate::lineage::LineageReport;
use crate::value::{CellCoord, CellRange, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceParam {
    pub name: String,
    pub cell_binding: Option<CellCoord>,
    pub constant_value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedSource {
    pub name: String,
    pub connection_name: String,
    pub sql: String,
    pub parameters: Vec<SourceParam>,
    pub last_result: Option<QueryResult>,
    pub lineage: Option<LineageReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorksheetTable {
    pub name: String,
    pub source_name: String,
    pub anchor: CellCoord,
    pub range: CellRange,
    pub columns: Vec<String>,
    pub has_headers: bool,
    pub has_totals: bool,
}

impl WorksheetTable {
    pub fn new(
        name: String,
        source_name: String,
        anchor: CellCoord,
        columns: Vec<String>,
        row_count: u32,
    ) -> Self {
        let col_count = columns.len() as u32;
        // 1 row for header + row_count for data
        let total_rows = 1 + row_count.max(1);
        let range = CellRange::new(
            anchor.row,
            anchor.col,
            anchor.row + total_rows - 1,
            anchor.col + col_count.saturating_sub(1),
        );
        Self {
            name,
            source_name,
            anchor,
            range,
            columns,
            has_headers: true,
            has_totals: false,
        }
    }

    /// Resolve a structured reference such as Invoices[Amount] into a CellRange
    pub fn resolve_structured_ref(
        &self,
        sref: &StructuredRef,
        eval_coord: Option<CellCoord>,
    ) -> Result<CellRange, CellError> {
        let col_idx = self
            .columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(&sref.column_name))
            .ok_or(CellError::Ref)?;

        let target_col = self.anchor.col + col_idx as u32;

        if sref.is_row_relative {
            // @Column - evaluates for the current row
            let cur = eval_coord.ok_or(CellError::Value)?;
            if cur.row <= self.anchor.row || cur.row > self.range.end_row {
                return Err(CellError::Value);
            }
            Ok(CellRange::single(CellCoord::new(cur.row, target_col)))
        } else {
            let start_row = if sref.include_headers {
                self.anchor.row
            } else {
                self.anchor.row + 1
            };
            let end_row = self.range.end_row;
            Ok(CellRange::new(start_row, target_col, end_row, target_col))
        }
    }
}
