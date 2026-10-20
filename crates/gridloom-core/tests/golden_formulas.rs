use gridloom_core::ast::StructuredRef;
use gridloom_core::error::CellError;
use gridloom_core::functions::{call_function, EvaluationContext};
use gridloom_core::value::{CellCoord, CellRange, Value};
use rust_decimal::Decimal;
use std::collections::HashMap;

struct DummyContext {
    cells: HashMap<CellCoord, Value>,
}

impl EvaluationContext for DummyContext {
    fn get_cell_value(&self, coord: CellCoord) -> Result<Value, CellError> {
        Ok(self.cells.get(&coord).cloned().unwrap_or(Value::Blank))
    }

    fn get_range_values(&self, range: CellRange) -> Result<Value, CellError> {
        let mut rows = Vec::new();
        for r in range.start_row..=range.end_row {
            let mut row = Vec::new();
            for c in range.start_col..=range.end_col {
                row.push(self.get_cell_value(CellCoord::new(r, c))?);
            }
            rows.push(row);
        }
        Ok(Value::Array(rows))
    }

    fn get_table_column(&self, _sref: &StructuredRef) -> Result<Value, CellError> {
        Err(CellError::Ref)
    }

    fn execute_sql_value(&self, _conn: &str, _sql: &str, _params: &[Value]) -> Result<Value, CellError> {
        Err(CellError::Sql("Not implemented".to_string()))
    }

    fn execute_sql_table(&self, _conn: &str, _sql: &str, _params: &[Value]) -> Result<Value, CellError> {
        Err(CellError::Sql("Not implemented".to_string()))
    }
}

fn eval(name: &str, args: &[Value]) -> Result<Value, CellError> {
    let ctx = DummyContext {
        cells: HashMap::new(),
    };
    call_function(name, args, &ctx)
}

fn dec(s: &str) -> Value {
    Value::Decimal(Decimal::from_str_exact(s).unwrap())
}

#[test]
fn test_golden_arithmetic() {
    // SUM
    assert_eq!(eval("SUM", &[dec("10"), dec("20.5"), dec("5")]).unwrap(), dec("35.5"));
    // PRODUCT
    assert_eq!(eval("PRODUCT", &[dec("2"), dec("3"), dec("4")]).unwrap(), dec("24"));
    // ABS
    assert_eq!(eval("ABS", &[dec("-42.5")]).unwrap(), dec("42.5"));
    // SIGN
    assert_eq!(eval("SIGN", &[dec("-100")]).unwrap(), Value::Integer(-1));
    assert_eq!(eval("SIGN", &[dec("0")]).unwrap(), Value::Integer(0));
    assert_eq!(eval("SIGN", &[dec("50")]).unwrap(), Value::Integer(1));
    // MOD
    assert_eq!(eval("MOD", &[dec("10"), dec("3")]).unwrap(), dec("1"));
    assert_eq!(eval("MOD", &[dec("10"), dec("0")]).unwrap_err(), CellError::Div0);
    // POWER
    assert_eq!(eval("POWER", &[dec("2"), dec("3")]).unwrap(), dec("8"));
    // SQRT
    assert_eq!(eval("SQRT", &[dec("16")]).unwrap(), dec("4"));
    assert_eq!(eval("SQRT", &[dec("-1")]).unwrap_err(), CellError::Num);
    // ROUND, ROUNDUP, ROUNDDOWN
    assert_eq!(eval("ROUND", &[dec("12.3456"), Value::Integer(2)]).unwrap(), dec("12.35"));
    assert_eq!(eval("ROUNDUP", &[dec("12.341"), Value::Integer(2)]).unwrap(), dec("12.35"));
    assert_eq!(eval("ROUNDDOWN", &[dec("12.349"), Value::Integer(2)]).unwrap(), dec("12.34"));
    // INT & TRUNC
    assert_eq!(eval("INT", &[dec("12.99")]).unwrap(), dec("12"));
    assert_eq!(eval("INT", &[dec("-12.01")]).unwrap(), dec("-13"));
    assert_eq!(eval("TRUNC", &[dec("12.99")]).unwrap(), dec("12"));
    assert_eq!(eval("TRUNC", &[dec("-12.99")]).unwrap(), dec("-12"));
    // CEILING.MATH & FLOOR.MATH
    assert_eq!(eval("CEILING.MATH", &[dec("24.3"), dec("5")]).unwrap(), dec("25"));
    assert_eq!(eval("FLOOR.MATH", &[dec("24.3"), dec("5")]).unwrap(), dec("20"));
    // SUMPRODUCT
    let a1 = Value::Array(vec![vec![dec("1"), dec("2")], vec![dec("3"), dec("4")]]);
    let a2 = Value::Array(vec![vec![dec("10"), dec("20")], vec![dec("30"), dec("40")]]);
    assert_eq!(eval("SUMPRODUCT", &[a1, a2]).unwrap(), dec("300")); // 10 + 40 + 90 + 160 = 300
    // SUMIF & SUMIFS
    let range = Value::Array(vec![vec![dec("10")], vec![dec("20")], vec![dec("30")]]);
    assert_eq!(eval("SUMIF", &[range.clone(), Value::Text(">15".to_string())]).unwrap(), dec("50"));
    assert_eq!(eval("SUMIFS", &[range.clone(), range.clone(), Value::Text(">=20".to_string())]).unwrap(), dec("50"));
}

#[test]
fn test_golden_statistics() {
    let arr = Value::Array(vec![vec![dec("10")], vec![dec("20")], vec![dec("30")], vec![dec("40")]]);
    // AVERAGE
    assert_eq!(eval("AVERAGE", &[arr.clone()]).unwrap(), dec("25"));
    // COUNT & COUNTA & COUNTBLANK
    let mixed = Value::Array(vec![vec![dec("10")], vec![Value::Text("test".to_string())], vec![Value::Blank]]);
    assert_eq!(eval("COUNT", &[mixed.clone()]).unwrap(), Value::Integer(1));
    assert_eq!(eval("COUNTA", &[mixed.clone()]).unwrap(), Value::Integer(2));
    assert_eq!(eval("COUNTBLANK", &[mixed.clone()]).unwrap(), Value::Integer(1));
    // MIN & MAX & MEDIAN
    assert_eq!(eval("MIN", &[arr.clone()]).unwrap(), dec("10"));
    assert_eq!(eval("MAX", &[arr.clone()]).unwrap(), dec("40"));
    assert_eq!(eval("MEDIAN", &[arr.clone()]).unwrap(), dec("25"));
    // STDEV.S & STDEV.P & VAR.S & VAR.P
    let data = Value::Array(vec![vec![dec("2")], vec![dec("4")], vec![dec("4")], vec![dec("4")], vec![dec("5")], vec![dec("5")], vec![dec("7")], vec![dec("9")]]);
    let var_p = eval("VAR.P", &[data.clone()]).unwrap();
    assert_eq!(var_p, dec("4"));
    let stdev_p = eval("STDEV.P", &[data.clone()]).unwrap();
    assert_eq!(stdev_p, dec("2"));
}

#[test]
fn test_golden_logic() {
    // IF
    assert_eq!(eval("IF", &[Value::Boolean(true), Value::Text("YES".to_string()), Value::Text("NO".to_string())]).unwrap(), Value::Text("YES".to_string()));
    assert_eq!(eval("IF", &[Value::Boolean(false), Value::Text("YES".to_string()), Value::Text("NO".to_string())]).unwrap(), Value::Text("NO".to_string()));
    // IFS
    assert_eq!(eval("IFS", &[Value::Boolean(false), dec("1"), Value::Boolean(true), dec("2")]).unwrap(), dec("2"));
    // AND, OR, NOT
    assert_eq!(eval("AND", &[Value::Boolean(true), Value::Boolean(true)]).unwrap(), Value::Boolean(true));
    assert_eq!(eval("AND", &[Value::Boolean(true), Value::Boolean(false)]).unwrap(), Value::Boolean(false));
    assert_eq!(eval("OR", &[Value::Boolean(false), Value::Boolean(true)]).unwrap(), Value::Boolean(true));
    assert_eq!(eval("NOT", &[Value::Boolean(true)]).unwrap(), Value::Boolean(false));
    // IFERROR & IFNA
    assert_eq!(eval("IFERROR", &[Value::Error(CellError::Div0), dec("99")]).unwrap(), dec("99"));
    assert_eq!(eval("IFERROR", &[dec("42"), dec("99")]).unwrap(), dec("42"));
    assert_eq!(eval("IFNA", &[Value::Error(CellError::NA), dec("99")]).unwrap(), dec("99"));
    assert_eq!(eval("IFNA", &[Value::Error(CellError::Value), dec("99")]).unwrap(), Value::Error(CellError::Value));
    // Inspection
    assert_eq!(eval("ISBLANK", &[Value::Blank]).unwrap(), Value::Boolean(true));
    assert_eq!(eval("ISBLANK", &[Value::Null]).unwrap(), Value::Boolean(false)); // Null distinct from blank!
    assert_eq!(eval("ISNUMBER", &[dec("100")]).unwrap(), Value::Boolean(true));
    assert_eq!(eval("ISTEXT", &[Value::Text("abc".to_string())]).unwrap(), Value::Boolean(true));
    assert_eq!(eval("ISERROR", &[Value::Error(CellError::Ref)]).unwrap(), Value::Boolean(true));
    assert_eq!(eval("ISNA", &[Value::Error(CellError::NA)]).unwrap(), Value::Boolean(true));
    assert_eq!(eval("ISNA", &[Value::Error(CellError::Div0)]).unwrap(), Value::Boolean(false));
}

#[test]
fn test_golden_text() {
    // CONCAT
    assert_eq!(eval("CONCAT", &[Value::Text("Hello ".to_string()), Value::Text("World".to_string())]).unwrap(), Value::Text("Hello World".to_string()));
    // TEXTJOIN
    assert_eq!(
        eval("TEXTJOIN", &[
            Value::Text(", ".to_string()),
            Value::Boolean(true),
            Value::Text("A".to_string()),
            Value::Text("".to_string()),
            Value::Text("B".to_string())
        ]).unwrap(),
        Value::Text("A, B".to_string())
    );
    // LEFT, RIGHT, MID, LEN
    assert_eq!(eval("LEFT", &[Value::Text("GRIDLOOM".to_string()), Value::Integer(4)]).unwrap(), Value::Text("GRID".to_string()));
    assert_eq!(eval("RIGHT", &[Value::Text("GRIDLOOM".to_string()), Value::Integer(4)]).unwrap(), Value::Text("LOOM".to_string()));
    assert_eq!(eval("MID", &[Value::Text("GRIDLOOM".to_string()), Value::Integer(5), Value::Integer(4)]).unwrap(), Value::Text("LOOM".to_string()));
    assert_eq!(eval("LEN", &[Value::Text("GRIDLOOM".to_string())]).unwrap(), Value::Integer(8));
    // TRIM, CLEAN, UPPER, LOWER
    assert_eq!(eval("TRIM", &[Value::Text("  Grid   Loom  ".to_string())]).unwrap(), Value::Text("Grid Loom".to_string()));
    assert_eq!(eval("UPPER", &[Value::Text("gridloom".to_string())]).unwrap(), Value::Text("GRIDLOOM".to_string()));
    assert_eq!(eval("LOWER", &[Value::Text("GRIDLOOM".to_string())]).unwrap(), Value::Text("gridloom".to_string()));
    // SUBSTITUTE & REPLACE
    assert_eq!(eval("SUBSTITUTE", &[Value::Text("sales 2025".to_string()), Value::Text("2025".to_string()), Value::Text("2026".to_string())]).unwrap(), Value::Text("sales 2026".to_string()));
    assert_eq!(eval("REPLACE", &[Value::Text("ABCDEF".to_string()), Value::Integer(3), Value::Integer(2), Value::Text("*".to_string())]).unwrap(), Value::Text("AB*EF".to_string()));
    // FIND & SEARCH
    assert_eq!(eval("FIND", &[Value::Text("L".to_string()), Value::Text("GRIDLOOM".to_string())]).unwrap(), Value::Integer(5));
    assert_eq!(eval("SEARCH", &[Value::Text("loom".to_string()), Value::Text("GRIDLOOM".to_string())]).unwrap(), Value::Integer(5));
    // VALUE & TEXT
    assert_eq!(eval("VALUE", &[Value::Text("$1,234.50".to_string())]).unwrap(), dec("1234.50"));
    assert_eq!(eval("TEXT", &[dec("0.75"), Value::Text("0%".to_string())]).unwrap(), Value::Text("75%".to_string()));
}

#[test]
fn test_golden_dates() {
    let date_val = eval("DATE", &[Value::Integer(2026), Value::Integer(10), Value::Integer(19)]).unwrap();
    assert_eq!(eval("YEAR", &[date_val.clone()]).unwrap(), Value::Integer(2026));
    assert_eq!(eval("MONTH", &[date_val.clone()]).unwrap(), Value::Integer(10));
    assert_eq!(eval("DAY", &[date_val.clone()]).unwrap(), Value::Integer(19));

    // EDATE & EOMONTH
    let edate = eval("EDATE", &[date_val.clone(), Value::Integer(2)]).unwrap();
    assert_eq!(eval("MONTH", &[edate]).unwrap(), Value::Integer(12));
    let eom = eval("EOMONTH", &[date_val.clone(), Value::Integer(0)]).unwrap();
    assert_eq!(eval("DAY", &[eom]).unwrap(), Value::Integer(31));

    // DAYS
    let d1 = eval("DATE", &[Value::Integer(2026), Value::Integer(10), Value::Integer(29)]).unwrap();
    assert_eq!(eval("DAYS", &[d1, date_val.clone()]).unwrap(), Value::Integer(10));

    // WEEKDAY (2026-10-19 is Monday -> weekday 2 with type 1, weekday 1 with type 2)
    assert_eq!(eval("WEEKDAY", &[date_val.clone(), Value::Integer(2)]).unwrap(), Value::Integer(1));

    // WORKDAY & NETWORKDAYS
    let next_workday = eval("WORKDAY", &[date_val.clone(), Value::Integer(5)]).unwrap();
    assert_eq!(eval("DAYS", &[next_workday.clone(), date_val.clone()]).unwrap(), Value::Integer(7)); // 5 workdays = 7 calendar days (Mon -> next Mon)
    assert_eq!(eval("NETWORKDAYS", &[date_val.clone(), next_workday]).unwrap(), Value::Integer(6));
}

#[test]
fn test_golden_lookup_and_arrays() {
    // SEQUENCE
    let seq = eval("SEQUENCE", &[Value::Integer(2), Value::Integer(2), dec("10"), dec("5")]).unwrap();
    assert_eq!(
        seq,
        Value::Array(vec![
            vec![dec("10"), dec("15")],
            vec![dec("20"), dec("25")]
        ])
    );

    // TRANSPOSE
    let tr = eval("TRANSPOSE", &[seq.clone()]).unwrap();
    assert_eq!(
        tr,
        Value::Array(vec![
            vec![dec("10"), dec("20")],
            vec![dec("15"), dec("25")]
        ])
    );

    // INDEX & MATCH
    assert_eq!(eval("INDEX", &[seq.clone(), Value::Integer(1), Value::Integer(2)]).unwrap(), dec("15"));
    let list = Value::Array(vec![vec![Value::Text("Apple".to_string())], vec![Value::Text("Banana".to_string())], vec![Value::Text("Cherry".to_string())]]);
    assert_eq!(eval("MATCH", &[Value::Text("Banana".to_string()), list.clone(), Value::Integer(0)]).unwrap(), Value::Integer(2));

    // VLOOKUP
    let table = Value::Array(vec![
        vec![dec("101"), Value::Text("Alice".to_string()), dec("5000")],
        vec![dec("102"), Value::Text("Bob".to_string()), dec("6000")],
    ]);
    assert_eq!(
        eval("VLOOKUP", &[dec("102"), table.clone(), Value::Integer(2), Value::Boolean(false)]).unwrap(),
        Value::Text("Bob".to_string())
    );

    // XLOOKUP
    let ids = Value::Array(vec![vec![dec("101")], vec![dec("102")], vec![dec("103")]]);
    let names = Value::Array(vec![vec![Value::Text("Alice".to_string())], vec![Value::Text("Bob".to_string())], vec![Value::Text("Charlie".to_string())]]);
    assert_eq!(
        eval("XLOOKUP", &[dec("103"), ids.clone(), names.clone()]).unwrap(),
        Value::Text("Charlie".to_string())
    );
    assert_eq!(
        eval("XLOOKUP", &[dec("999"), ids.clone(), names.clone(), Value::Text("NOT_FOUND".to_string())]).unwrap(),
        Value::Text("NOT_FOUND".to_string())
    );

    // FILTER
    let numbers = Value::Array(vec![vec![dec("10")], vec![dec("20")], vec![dec("30")]]);
    let include = Value::Array(vec![vec![Value::Boolean(true)], vec![Value::Boolean(false)], vec![Value::Boolean(true)]]);
    let filtered = eval("FILTER", &[numbers, include]).unwrap();
    assert_eq!(filtered, Value::Array(vec![vec![dec("10")], vec![dec("30")]]));

    // UNIQUE
    let duplicates = Value::Array(vec![vec![dec("1")], vec![dec("2")], vec![dec("1")], vec![dec("3")]]);
    assert_eq!(eval("UNIQUE", &[duplicates]).unwrap(), Value::Array(vec![vec![dec("1")], vec![dec("2")], vec![dec("3")]]));

    // SORT
    let unsorted = Value::Array(vec![vec![dec("30")], vec![dec("10")], vec![dec("20")]]);
    assert_eq!(eval("SORT", &[unsorted, Value::Integer(1), Value::Integer(1)]).unwrap(), Value::Array(vec![vec![dec("10")], vec![dec("20")], vec![dec("30")]]));
}

#[test]
fn test_golden_financial() {
    // PMT: rate = 8%/12, nper = 10*12, pv = 10000
    // Monthly payment on a $10,000 loan at 8% for 10 years is approx -121.33
    let pmt_val = eval("PMT", &[dec("0.0066666667"), Value::Integer(120), dec("10000")]).unwrap();
    let rounded_pmt = eval("ROUND", &[pmt_val, Value::Integer(2)]).unwrap();
    assert_eq!(rounded_pmt, dec("-121.33"));

    // NPV
    let npv_val = eval("NPV", &[dec("0.10"), dec("-10000"), dec("3000"), dec("4200"), dec("6800")]).unwrap();
    let rounded_npv = eval("ROUND", &[npv_val, Value::Integer(2)]).unwrap();
    assert_eq!(rounded_npv, dec("1188.44"));

    // IRR
    let cash_flows = Value::Array(vec![vec![dec("-10000")], vec![dec("3000")], vec![dec("4200")], vec![dec("6800")]]);
    let irr_val = eval("IRR", &[cash_flows]).unwrap();
    let rounded_irr = eval("ROUND", &[irr_val, Value::Integer(4)]).unwrap();
    assert_eq!(rounded_irr, dec("0.1634"));
}
