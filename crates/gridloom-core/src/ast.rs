use crate::value::{CellCoord, CellRange, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Power,
    Concat, // &
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Plus,
    Minus,
    Percent,
}

/// Structured reference to a positioned worksheet table
/// e.g. Invoices[Amount], Invoices[@Amount], Invoices[[#All], [Amount]]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructuredRef {
    pub table_name: String,
    pub column_name: String,
    pub is_row_relative: bool, // @Column
    pub include_headers: bool, // #All or #Headers
    pub include_totals: bool,  // #Totals
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Literal(Value),
    CellRef(CellCoord),
    RangeRef(CellRange),
    SpillRef(CellCoord), // A1#
    StructuredRef(StructuredRef),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },
    ArrayLiteral(Vec<Vec<Expr>>),
}
