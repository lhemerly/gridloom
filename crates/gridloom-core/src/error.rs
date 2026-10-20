use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellError {
    /// #NULL! - Intersection of two ranges that do not intersect
    Null,
    /// #DIV/0! - Attempt to divide by zero
    Div0,
    /// #VALUE! - Wrong type of argument or operand
    Value,
    /// #REF! - Invalid cell reference
    Ref,
    /// #NAME? - Unrecognized formula name or label
    Name,
    /// #NUM! - Invalid numeric value or calculation overflow
    Num,
    /// #N/A - Value not available
    NA,
    /// #SPILL! - Spilled dynamic array overlaps with non-empty cells
    Spill,
    /// #SQL! - SQL database query error or constraint failure
    Sql(String),
    /// #CYCLE! - Circular dependency detected in calculation graph
    Cycle,
    /// #STALE! - Query failed or cancelled, retaining last known value
    Stale,
}

impl CellError {
    pub fn code(&self) -> &'static str {
        match self {
            CellError::Null => "#NULL!",
            CellError::Div0 => "#DIV/0!",
            CellError::Value => "#VALUE!",
            CellError::Ref => "#REF!",
            CellError::Name => "#NAME?",
            CellError::Num => "#NUM!",
            CellError::NA => "#N/A",
            CellError::Spill => "#SPILL!",
            CellError::Sql(_) => "#SQL!",
            CellError::Cycle => "#CYCLE!",
            CellError::Stale => "#STALE!",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            CellError::Null => "Intersection of two ranges does not intersect",
            CellError::Div0 => "Division by zero",
            CellError::Value => "A value used in the formula is of the wrong data type",
            CellError::Ref => "Invalid cell reference",
            CellError::Name => "Unrecognized formula name or syntax",
            CellError::Num => "Number calculation error or overflow",
            CellError::NA => "Value not available",
            CellError::Spill => "Spill range is blocked by existing cells",
            CellError::Sql(msg) => msg.as_str(),
            CellError::Cycle => "Circular reference detected",
            CellError::Stale => "Calculation stale due to error or cancellation",
        }
    }
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl std::error::Error for CellError {}
