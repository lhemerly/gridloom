pub mod ast;
pub mod chart;
pub mod db;
pub mod depgraph;
pub mod error;
pub mod evaluator;
pub mod functions;
pub mod lineage;
pub mod parser;
pub mod pivot;
pub mod spill;
pub mod table;
pub mod value;
pub mod virtual_view;
pub mod workbook;
pub mod writeback;

pub use error::CellError;
pub use value::{CellCoord, CellRange, Value};
pub use workbook::Workbook;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
