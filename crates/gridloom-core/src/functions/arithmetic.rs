use crate::error::CellError;
use crate::value::Value;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

/// Helper to flatten nested values and arrays into a flat list of values
pub fn flatten_values(args: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for val in args {
        match val {
            Value::Array(rows) => {
                for row in rows {
                    for cell in row {
                        out.push(cell.clone());
                    }
                }
            }
            other => out.push(other.clone()),
        }
    }
    out
}

pub fn sum(args: &[Value]) -> Result<Value, CellError> {
    let mut total = Decimal::ZERO;
    let flat = flatten_values(args);
    for val in flat {
        match val {
            Value::Integer(i) => total += Decimal::from(i),
            Value::Decimal(d) => total += d,
            Value::Float(f) => {
                if let Some(d) = Decimal::from_f64(f) {
                    total += d;
                }
            }
            Value::Error(e) => return Err(e),
            // Text and blank cells in ranges are ignored in Excel SUM
            _ => {}
        }
    }
    Ok(Value::Decimal(total))
}

pub fn product(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut prod = Decimal::ONE;
    let mut count = 0;
    for val in flat {
        match val {
            Value::Integer(i) => {
                prod *= Decimal::from(i);
                count += 1;
            }
            Value::Decimal(d) => {
                prod *= d;
                count += 1;
            }
            Value::Float(f) => {
                if let Some(d) = Decimal::from_f64(f) {
                    prod *= d;
                    count += 1;
                }
            }
            Value::Error(e) => return Err(e),
            _ => {}
        }
    }
    if count == 0 {
        Ok(Value::Decimal(Decimal::ZERO))
    } else {
        Ok(Value::Decimal(prod))
    }
}

pub fn abs(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let d = args[0].to_decimal()?;
    Ok(Value::Decimal(d.abs()))
}

pub fn sign(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let d = args[0].to_decimal()?;
    if d > Decimal::ZERO {
        Ok(Value::Integer(1))
    } else if d < Decimal::ZERO {
        Ok(Value::Integer(-1))
    } else {
        Ok(Value::Integer(0))
    }
}

pub fn mod_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let n = args[0].to_decimal()?;
    let d = args[1].to_decimal()?;
    if d.is_zero() {
        return Err(CellError::Div0);
    }
    // Excel MOD(n, d) = n - d * INT(n / d)
    let div = n / d;
    let floor_div = div.floor();
    let res = n - d * floor_div;
    Ok(Value::Decimal(res))
}

pub fn power(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let base = args[0].to_f64()?;
    let exp = args[1].to_f64()?;
    let res = base.powf(exp);
    if res.is_nan() || res.is_infinite() {
        return Err(CellError::Num);
    }
    if let Some(d) = Decimal::from_f64(res) {
        Ok(Value::Decimal(d))
    } else {
        Ok(Value::Float(res))
    }
}

pub fn sqrt(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let d = args[0].to_decimal()?;
    if d < Decimal::ZERO {
        return Err(CellError::Num);
    }
    let f = d.to_f64().ok_or(CellError::Num)?;
    let res = f.sqrt();
    if let Some(dec) = Decimal::from_f64(res) {
        Ok(Value::Decimal(dec))
    } else {
        Ok(Value::Float(res))
    }
}

pub fn round(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let n = args[0].to_decimal()?;
    let digits = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        0
    };
    if digits >= 0 {
        let d = digits as u32;
        Ok(Value::Decimal(n.round_dp(d)))
    } else {
        let factor = Decimal::from(10i64.pow((-digits) as u32));
        let scaled = (n / factor).round();
        Ok(Value::Decimal(scaled * factor))
    }
}

pub fn roundup(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let n = args[0].to_decimal()?;
    let digits = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        0
    };
    if digits >= 0 {
        let factor = Decimal::from(10i64.pow(digits as u32));
        let sign = if n < Decimal::ZERO { -Decimal::ONE } else { Decimal::ONE };
        let abs_scaled = (n * factor).abs();
        let ceil = abs_scaled.ceil();
        Ok(Value::Decimal(ceil * sign / factor))
    } else {
        let factor = Decimal::from(10i64.pow((-digits) as u32));
        let sign = if n < Decimal::ZERO { -Decimal::ONE } else { Decimal::ONE };
        let abs_scaled = (n / factor).abs();
        let ceil = abs_scaled.ceil();
        Ok(Value::Decimal(ceil * sign * factor))
    }
}

pub fn rounddown(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let n = args[0].to_decimal()?;
    let digits = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        0
    };
    if digits >= 0 {
        let factor = Decimal::from(10i64.pow(digits as u32));
        let sign = if n < Decimal::ZERO { -Decimal::ONE } else { Decimal::ONE };
        let abs_scaled = (n * factor).abs();
        let floor = abs_scaled.floor();
        Ok(Value::Decimal(floor * sign / factor))
    } else {
        let factor = Decimal::from(10i64.pow((-digits) as u32));
        let sign = if n < Decimal::ZERO { -Decimal::ONE } else { Decimal::ONE };
        let abs_scaled = (n / factor).abs();
        let floor = abs_scaled.floor();
        Ok(Value::Decimal(floor * sign * factor))
    }
}

pub fn int_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let n = args[0].to_decimal()?;
    Ok(Value::Decimal(n.floor()))
}

pub fn trunc(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let n = args[0].to_decimal()?;
    let digits = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        0
    };
    if digits >= 0 {
        let factor = Decimal::from(10i64.pow(digits as u32));
        let truncated = (n * factor).trunc() / factor;
        Ok(Value::Decimal(truncated))
    } else {
        let factor = Decimal::from(10i64.pow((-digits) as u32));
        let truncated = (n / factor).trunc() * factor;
        Ok(Value::Decimal(truncated))
    }
}

pub fn ceiling_math(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let n = args[0].to_decimal()?;
    let sig = if args.len() > 1 && !args[1].is_blank() {
        args[1].to_decimal()?.abs()
    } else {
        Decimal::ONE
    };
    if sig.is_zero() {
        return Ok(Value::Decimal(Decimal::ZERO));
    }
    let mode = if args.len() > 2 {
        args[2].to_i64()?
    } else {
        0
    };
    let div = n / sig;
    let mult = if n < Decimal::ZERO && mode != 0 {
        div.floor() // towards zero
    } else {
        div.ceil()
    };
    Ok(Value::Decimal(mult * sig))
}

pub fn floor_math(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let n = args[0].to_decimal()?;
    let sig = if args.len() > 1 && !args[1].is_blank() {
        args[1].to_decimal()?.abs()
    } else {
        Decimal::ONE
    };
    if sig.is_zero() {
        return Ok(Value::Decimal(Decimal::ZERO));
    }
    let mode = if args.len() > 2 {
        args[2].to_i64()?
    } else {
        0
    };
    let div = n / sig;
    let mult = if n < Decimal::ZERO && mode != 0 {
        div.ceil() // towards zero
    } else {
        div.floor()
    };
    Ok(Value::Decimal(mult * sig))
}

pub fn sumproduct(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let mut flat_arrays: Vec<Vec<Decimal>> = Vec::new();
    for arg in args {
        let mut list = Vec::new();
        match arg {
            Value::Array(rows) => {
                for r in rows {
                    for cell in r {
                        list.push(cell.to_decimal().unwrap_or(Decimal::ZERO));
                    }
                }
            }
            other => {
                list.push(other.to_decimal().unwrap_or(Decimal::ZERO));
            }
        }
        flat_arrays.push(list);
    }

    let len = flat_arrays[0].len();
    for arr in &flat_arrays {
        if arr.len() != len {
            return Err(CellError::Value);
        }
    }

    let mut sum = Decimal::ZERO;
    for i in 0..len {
        let mut prod = Decimal::ONE;
        for arr in &flat_arrays {
            prod *= arr[i];
        }
        sum += prod;
    }
    Ok(Value::Decimal(sum))
}

pub fn matches_criteria(cell: &Value, criteria: &Value) -> bool {
    let crit_str = criteria.to_string_repr().trim().to_string();
    if crit_str.starts_with(">=") {
        let target = Decimal::from_str_exact(crit_str[2..].trim()).unwrap_or(Decimal::ZERO);
        cell.to_decimal().map_or(false, |val| val >= target)
    } else if crit_str.starts_with("<=") {
        let target = Decimal::from_str_exact(crit_str[2..].trim()).unwrap_or(Decimal::ZERO);
        cell.to_decimal().map_or(false, |val| val <= target)
    } else if crit_str.starts_with("<>") {
        let target = &crit_str[2..].trim();
        cell.to_string_repr() != *target
    } else if crit_str.starts_with('>') {
        let target = Decimal::from_str_exact(crit_str[1..].trim()).unwrap_or(Decimal::ZERO);
        cell.to_decimal().map_or(false, |val| val > target)
    } else if crit_str.starts_with('<') {
        let target = Decimal::from_str_exact(crit_str[1..].trim()).unwrap_or(Decimal::ZERO);
        cell.to_decimal().map_or(false, |val| val < target)
    } else if crit_str.starts_with('=') {
        let target = &crit_str[1..].trim();
        cell.to_string_repr().eq_ignore_ascii_case(target)
    } else {
        if let Ok(target_num) = Decimal::from_str_exact(&crit_str) {
            cell.to_decimal().map_or(false, |v| v == target_num)
        } else {
            cell.to_string_repr().eq_ignore_ascii_case(&crit_str)
        }
    }
}

pub fn sumif(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let range = match &args[0] {
        Value::Array(rows) => rows,
        _ => return Err(CellError::Value),
    };
    let criteria = &args[1];
    let sum_range = if args.len() > 2 {
        match &args[2] {
            Value::Array(rows) => Some(rows),
            _ => return Err(CellError::Value),
        }
    } else {
        None
    };

    let mut sum = Decimal::ZERO;
    for (r_idx, row) in range.iter().enumerate() {
        for (c_idx, cell) in row.iter().enumerate() {
            if matches_criteria(cell, criteria) {
                let add_val = if let Some(sr) = sum_range {
                    sr.get(r_idx)
                        .and_then(|r| r.get(c_idx))
                        .unwrap_or(&Value::Blank)
                } else {
                    cell
                };
                if let Ok(d) = add_val.to_decimal() {
                    sum += d;
                }
            }
        }
    }
    Ok(Value::Decimal(sum))
}

pub fn sumifs(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 || args.len() % 2 == 0 {
        return Err(CellError::Value);
    }
    let sum_range = match &args[0] {
        Value::Array(rows) => rows,
        _ => return Err(CellError::Value),
    };

    let mut criteria_pairs = Vec::new();
    let mut idx = 1;
    while idx < args.len() {
        let range = match &args[idx] {
            Value::Array(rows) => rows,
            _ => return Err(CellError::Value),
        };
        let crit = &args[idx + 1];
        criteria_pairs.push((range, crit));
        idx += 2;
    }

    let mut total = Decimal::ZERO;
    for (r_idx, row) in sum_range.iter().enumerate() {
        for (c_idx, sum_cell) in row.iter().enumerate() {
            let mut all_match = true;
            for (crange, crit) in &criteria_pairs {
                let cell = crange
                    .get(r_idx)
                    .and_then(|r| r.get(c_idx))
                    .unwrap_or(&Value::Blank);
                if !matches_criteria(cell, crit) {
                    all_match = false;
                    break;
                }
            }
            if all_match {
                if let Ok(d) = sum_cell.to_decimal() {
                    total += d;
                }
            }
        }
    }
    Ok(Value::Decimal(total))
}
