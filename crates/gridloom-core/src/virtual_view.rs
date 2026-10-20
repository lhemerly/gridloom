use crate::value::{CellCoord, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportRequest {
    pub sheet_name: String,
    pub start_row: u32,
    pub start_col: u32,
    pub end_row: u32,
    pub end_col: u32,
    pub overscan: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellRenderData {
    pub coord: CellCoord,
    pub display_text: String,
    pub raw_value: Value,
    pub formula: Option<String>,
    pub number_format: Option<String>,
    pub is_staged: bool,
    pub is_spill: bool,
    pub is_error: bool,
    pub table_name: Option<String>,
    pub is_table_header: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportResponse {
    pub sheet_name: String,
    pub start_row: u32,
    pub start_col: u32,
    pub end_row: u32,
    pub end_col: u32,
    pub cells: HashMap<String, CellRenderData>, // Key is A1 notation
    pub total_rows: u32,
    pub total_cols: u32,
    pub generation: u64,
}
