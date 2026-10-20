use crate::error::CellError;
use chrono::{DateTime, NaiveDate, Timelike, Utc};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

/// Rich value model for Gridloom worksheets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    /// Blank unpopulated cell
    Blank,
    /// Explicit empty string ""
    EmptyText,
    /// Database SQL NULL - distinct from blank and empty text
    Null,
    /// Boolean true/false
    Boolean(bool),
    /// Exact 64-bit integer
    Integer(i64),
    /// Exact fixed-point decimal for accounting precision and DuckDB DECIMAL preservation
    Decimal(Decimal),
    /// IEEE 754 floating-point number
    Float(f64),
    /// String text
    Text(String),
    /// Date without time
    Date(NaiveDate),
    /// UTC Timestamp
    Timestamp(DateTime<Utc>),
    /// Dynamic array / matrix of values
    Array(Vec<Vec<Value>>),
    /// Calculation or runtime error
    Error(CellError),
}

impl Default for Value {
    fn default() -> Self {
        Value::Blank
    }
}

impl Value {
    pub fn is_blank(&self) -> bool {
        matches!(self, Value::Blank)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_empty_text(&self) -> bool {
        matches!(self, Value::EmptyText)
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Value::Blank | Value::EmptyText | Value::Null)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Integer(_) | Value::Decimal(_) | Value::Float(_))
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Value::Text(_) | Value::EmptyText)
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }

    pub fn as_error(&self) -> Option<&CellError> {
        match self {
            Value::Error(err) => Some(err),
            _ => None,
        }
    }

    /// Convert to exact Decimal for accounting calculations.
    /// Handles exact integers, decimals, floats without loss where possible,
    /// and parses numeric strings. Blanks coerce to 0.
    pub fn to_decimal(&self) -> Result<Decimal, CellError> {
        match self {
            Value::Blank => Ok(Decimal::ZERO),
            Value::Integer(i) => Ok(Decimal::from(*i)),
            Value::Decimal(d) => Ok(*d),
            Value::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    Err(CellError::Num)
                } else {
                    Decimal::from_f64(*f).ok_or(CellError::Num)
                }
            }
            Value::Boolean(b) => Ok(if *b { Decimal::ONE } else { Decimal::ZERO }),
            Value::Text(s) => Decimal::from_str_exact(s.trim()).map_err(|_| CellError::Value),
            Value::EmptyText => Err(CellError::Value),
            Value::Null => Err(CellError::Value),
            Value::Date(d) => {
                // Excel serial date representation: days since 1899-12-30
                let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
                let days = (*d - epoch).num_days();
                Ok(Decimal::from(days))
            }
            Value::Timestamp(ts) => {
                let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
                let days = (ts.date_naive() - epoch).num_days();
                let secs_in_day = ts.time().num_seconds_from_midnight() as f64 / 86400.0;
                let day_frac = Decimal::from_f64(secs_in_day).unwrap_or(Decimal::ZERO);
                Ok(Decimal::from(days) + day_frac)
            }
            Value::Array(_) => Err(CellError::Value),
            Value::Error(e) => Err(e.clone()),
        }
    }

    /// Convert to f64 for statistical or floating-point calculations
    pub fn to_f64(&self) -> Result<f64, CellError> {
        match self {
            Value::Blank => Ok(0.0),
            Value::Integer(i) => Ok(*i as f64),
            Value::Decimal(d) => d.to_f64().ok_or(CellError::Num),
            Value::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    Err(CellError::Num)
                } else {
                    Ok(*f)
                }
            }
            Value::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::Text(s) => s.trim().parse::<f64>().map_err(|_| CellError::Value),
            Value::EmptyText => Err(CellError::Value),
            Value::Null => Err(CellError::Value),
            Value::Date(d) => {
                let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
                Ok((*d - epoch).num_days() as f64)
            }
            Value::Timestamp(ts) => {
                let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
                let days = (ts.date_naive() - epoch).num_days() as f64;
                let frac = ts.time().num_seconds_from_midnight() as f64 / 86400.0;
                Ok(days + frac)
            }
            Value::Array(_) => Err(CellError::Value),
            Value::Error(e) => Err(e.clone()),
        }
    }

    pub fn to_i64(&self) -> Result<i64, CellError> {
        match self {
            Value::Blank => Ok(0),
            Value::Integer(i) => Ok(*i),
            Value::Decimal(d) => d.to_i64().ok_or(CellError::Num),
            Value::Float(f) => {
                if f.is_nan() || f.is_infinite() {
                    Err(CellError::Num)
                } else {
                    Ok(*f as i64)
                }
            }
            Value::Boolean(b) => Ok(if *b { 1 } else { 0 }),
            Value::Text(s) => s.trim().parse::<i64>().map_err(|_| CellError::Value),
            Value::EmptyText => Err(CellError::Value),
            Value::Null => Err(CellError::Value),
            Value::Date(d) => {
                let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
                Ok((*d - epoch).num_days())
            }
            Value::Timestamp(ts) => {
                let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
                Ok((ts.date_naive() - epoch).num_days())
            }
            Value::Array(_) => Err(CellError::Value),
            Value::Error(e) => Err(e.clone()),
        }
    }

    pub fn to_bool(&self) -> Result<bool, CellError> {
        match self {
            Value::Boolean(b) => Ok(*b),
            Value::Blank => Ok(false),
            Value::Integer(i) => Ok(*i != 0),
            Value::Decimal(d) => Ok(!d.is_zero()),
            Value::Float(f) => Ok(*f != 0.0),
            Value::Text(s) => match s.trim().to_uppercase().as_str() {
                "TRUE" => Ok(true),
                "FALSE" => Ok(false),
                _ => Err(CellError::Value),
            },
            Value::EmptyText => Err(CellError::Value),
            Value::Null => Err(CellError::Value),
            Value::Date(_) | Value::Timestamp(_) => Ok(true),
            Value::Array(_) => Err(CellError::Value),
            Value::Error(e) => Err(e.clone()),
        }
    }

    pub fn to_string_repr(&self) -> String {
        match self {
            Value::Blank => String::new(),
            Value::EmptyText => String::new(),
            Value::Null => "NULL".to_string(),
            Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Decimal(d) => d.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.0}", f)
                } else {
                    f.to_string()
                }
            }
            Value::Text(s) => s.clone(),
            Value::Date(d) => d.format("%Y-%m-%d").to_string(),
            Value::Timestamp(ts) => ts.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            Value::Array(rows) => {
                format!("Array({}x{})", rows.len(), rows.first().map_or(0, |r| r.len()))
            }
            Value::Error(e) => e.to_string(),
        }
    }

    /// Extract scalar if 1x1 array or return shape error if multi-row/col
    pub fn into_scalar(self) -> Result<Value, CellError> {
        match self {
            Value::Array(rows) => {
                if rows.is_empty() || rows[0].is_empty() {
                    Ok(Value::Null)
                } else if rows.len() == 1 && rows[0].len() == 1 {
                    Ok(rows.into_iter().next().unwrap().into_iter().next().unwrap())
                } else {
                    Err(CellError::Value)
                }
            }
            other => Ok(other),
        }
    }

    /// SQL NULL-safe equality:
    /// Returns true if both are Null, or if values are strictly equal.
    pub fn is_not_distinct_from(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Blank, Value::Blank) => true,
            (Value::EmptyText, Value::EmptyText) => true,
            (Value::Null, _) | (_, Value::Null) => false,
            (Value::Blank, Value::EmptyText) | (Value::EmptyText, Value::Blank) => false,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Decimal(a), Value::Decimal(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Timestamp(a), Value::Timestamp(b)) => a == b,
            (Value::Error(a), Value::Error(b)) => a == b,
            // Cross-numeric exact check
            (Value::Integer(a), Value::Decimal(b)) => Decimal::from(*a) == *b,
            (Value::Decimal(a), Value::Integer(b)) => *a == Decimal::from(*b),
            (Value::Integer(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Integer(b)) => *a == (*b as f64),
            (Value::Decimal(a), Value::Float(b)) => a.to_f64() == Some(*b),
            (Value::Float(a), Value::Decimal(b)) => Some(*a) == b.to_f64(),
            _ => false,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Blank, Value::Blank) => true,
            (Value::EmptyText, Value::EmptyText) => true,
            (Value::Null, Value::Null) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Decimal(a), Value::Decimal(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b || (a.is_nan() && b.is_nan()),
            (Value::Text(a), Value::Text(b)) => a.eq_ignore_ascii_case(b),
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Timestamp(a), Value::Timestamp(b)) => a == b,
            (Value::Error(a), Value::Error(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            // Cross-numeric
            (Value::Integer(a), Value::Decimal(b)) => Decimal::from(*a) == *b,
            (Value::Decimal(a), Value::Integer(b)) => *a == Decimal::from(*b),
            (Value::Integer(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Integer(b)) => *a == (*b as f64),
            (Value::Decimal(a), Value::Float(b)) => a.to_f64() == Some(*b),
            (Value::Float(a), Value::Decimal(b)) => Some(*a) == b.to_f64(),
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a.partial_cmp(b),
            (Value::Decimal(a), Value::Decimal(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Text(a), Value::Text(b)) => {
                a.to_lowercase().partial_cmp(&b.to_lowercase())
            }
            (Value::Date(a), Value::Date(b)) => a.partial_cmp(b),
            (Value::Timestamp(a), Value::Timestamp(b)) => a.partial_cmp(b),
            (Value::Boolean(a), Value::Boolean(b)) => a.partial_cmp(b),
            (Value::Integer(a), Value::Decimal(b)) => Decimal::from(*a).partial_cmp(b),
            (Value::Decimal(a), Value::Integer(b)) => a.partial_cmp(&Decimal::from(*b)),
            (Value::Integer(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Integer(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Decimal(a), Value::Float(b)) => a.to_f64()?.partial_cmp(b),
            (Value::Float(a), Value::Decimal(b)) => a.partial_cmp(&b.to_f64()?),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_repr())
    }
}

/// Zero-indexed row and column coordinate
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellCoord {
    pub row: u32,
    pub col: u32,
}

impl CellCoord {
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    /// Converts 0-indexed coord to A1 notation, e.g. (0, 0) -> "A1", (3, 1) -> "B4"
    pub fn to_a1(&self) -> String {
        format!("{}{}", col_to_letters(self.col), self.row + 1)
    }

    /// Parses A1 notation into CellCoord, e.g. "B4" -> (3, 1)
    pub fn from_a1(s: &str) -> Result<Self, CellError> {
        let trimmed = s.trim();
        let mut col_str = String::new();
        let mut row_str = String::new();

        for ch in trimmed.chars() {
            if ch == '$' {
                continue;
            }
            if ch.is_ascii_alphabetic() {
                if !row_str.is_empty() {
                    return Err(CellError::Ref);
                }
                col_str.push(ch.to_ascii_uppercase());
            } else if ch.is_ascii_digit() {
                row_str.push(ch);
            } else {
                return Err(CellError::Ref);
            }
        }

        if col_str.is_empty() || row_str.is_empty() {
            return Err(CellError::Ref);
        }

        let col = letters_to_col(&col_str)?;
        let row_num: u32 = row_str.parse().map_err(|_| CellError::Ref)?;
        if row_num == 0 {
            return Err(CellError::Ref);
        }
        let row = row_num - 1;

        Ok(CellCoord { row, col })
    }
}

/// Rectangular range of cells
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellRange {
    pub start_row: u32,
    pub start_col: u32,
    pub end_row: u32,
    pub end_col: u32,
}

impl CellRange {
    pub fn new(start_row: u32, start_col: u32, end_row: u32, end_col: u32) -> Self {
        Self {
            start_row: start_row.min(end_row),
            start_col: start_col.min(end_col),
            end_row: start_row.max(end_row),
            end_col: start_col.max(end_col),
        }
    }

    pub fn single(coord: CellCoord) -> Self {
        Self::new(coord.row, coord.col, coord.row, coord.col)
    }

    pub fn width(&self) -> u32 {
        self.end_col - self.start_col + 1
    }

    pub fn height(&self) -> u32 {
        self.end_row - self.start_row + 1
    }

    pub fn contains(&self, coord: CellCoord) -> bool {
        coord.row >= self.start_row
            && coord.row <= self.end_row
            && coord.col >= self.start_col
            && coord.col <= self.end_col
    }

    pub fn intersects(&self, other: &CellRange) -> bool {
        !(self.end_row < other.start_row
            || self.start_row > other.end_row
            || self.end_col < other.start_col
            || self.start_col > other.end_col)
    }

    pub fn to_a1(&self) -> String {
        let start = CellCoord::new(self.start_row, self.start_col).to_a1();
        let end = CellCoord::new(self.end_row, self.end_col).to_a1();
        if start == end {
            start
        } else {
            format!("{}:{}", start, end)
        }
    }

    pub fn from_a1(s: &str) -> Result<Self, CellError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 1 {
            let coord = CellCoord::from_a1(parts[0])?;
            Ok(CellRange::single(coord))
        } else if parts.len() == 2 {
            let start = CellCoord::from_a1(parts[0])?;
            let end = CellCoord::from_a1(parts[1])?;
            Ok(CellRange::new(start.row, start.col, end.row, end.col))
        } else {
            Err(CellError::Ref)
        }
    }
}

pub fn col_to_letters(mut col: u32) -> String {
    let mut letters = Vec::new();
    loop {
        let rem = (col % 26) as u8;
        letters.push((b'A' + rem) as char);
        if col < 26 {
            break;
        }
        col = (col / 26) - 1;
    }
    letters.reverse();
    letters.into_iter().collect()
}

pub fn letters_to_col(s: &str) -> Result<u32, CellError> {
    let mut col: u32 = 0;
    for ch in s.chars() {
        if !ch.is_ascii_uppercase() {
            return Err(CellError::Ref);
        }
        let val = (ch as u32) - ('A' as u32) + 1;
        col = col.checked_mul(26).ok_or(CellError::Ref)?;
        col = col.checked_add(val).ok_or(CellError::Ref)?;
    }
    if col == 0 {
        return Err(CellError::Ref);
    }
    Ok(col - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a1_conversion() {
        assert_eq!(CellCoord::new(0, 0).to_a1(), "A1");
        assert_eq!(CellCoord::new(3, 1).to_a1(), "B4");
        assert_eq!(CellCoord::new(99, 26).to_a1(), "AA100");

        assert_eq!(CellCoord::from_a1("A1").unwrap(), CellCoord::new(0, 0));
        assert_eq!(CellCoord::from_a1("B4").unwrap(), CellCoord::new(3, 1));
        assert_eq!(CellCoord::from_a1("$AA$100").unwrap(), CellCoord::new(99, 26));

        let range = CellRange::from_a1("A1:B10").unwrap();
        assert_eq!(range.width(), 2);
        assert_eq!(range.height(), 10);
        assert_eq!(range.to_a1(), "A1:B10");
    }

    #[test]
    fn test_decimal_precision() {
        let d = Decimal::from_str_exact("123456789.123456789").unwrap();
        let val = Value::Decimal(d);
        assert_eq!(val.to_decimal().unwrap(), d);
        assert_eq!(val.to_string(), "123456789.123456789");
    }

    #[test]
    fn test_null_distinctness() {
        assert!(Value::Null.is_null());
        assert!(!Value::Blank.is_null());
        assert!(!Value::EmptyText.is_null());
        assert!(Value::Null.is_not_distinct_from(&Value::Null));
        assert!(!Value::Null.is_not_distinct_from(&Value::Blank));
        assert!(!Value::Null.is_not_distinct_from(&Value::EmptyText));
    }
}
