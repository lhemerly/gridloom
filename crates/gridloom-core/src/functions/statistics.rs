use crate::error::CellError;
use crate::functions::arithmetic::{flatten_values, matches_criteria};
use crate::value::Value;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

pub fn average(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut sum = Decimal::ZERO;
    let mut count = 0;
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        if val.is_number() {
            if let Ok(d) = val.to_decimal() {
                sum += d;
                count += 1;
            }
        }
    }
    if count == 0 {
        return Err(CellError::Div0);
    }
    Ok(Value::Decimal(sum / Decimal::from(count)))
}

pub fn averageif(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let range = match &args[0] {
        Value::Array(rows) => rows,
        _ => return Err(CellError::Value),
    };
    let criteria = &args[1];
    let avg_range = if args.len() > 2 {
        match &args[2] {
            Value::Array(rows) => Some(rows),
            _ => return Err(CellError::Value),
        }
    } else {
        None
    };

    let mut sum = Decimal::ZERO;
    let mut count = 0;
    for (r_idx, row) in range.iter().enumerate() {
        for (c_idx, cell) in row.iter().enumerate() {
            if matches_criteria(cell, criteria) {
                let add_val = if let Some(ar) = avg_range {
                    ar.get(r_idx)
                        .and_then(|r| r.get(c_idx))
                        .unwrap_or(&Value::Blank)
                } else {
                    cell
                };
                if let Ok(d) = add_val.to_decimal() {
                    sum += d;
                    count += 1;
                }
            }
        }
    }
    if count == 0 {
        return Err(CellError::Div0);
    }
    Ok(Value::Decimal(sum / Decimal::from(count)))
}

pub fn averageifs(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 || args.len() % 2 == 0 {
        return Err(CellError::Value);
    }
    let avg_range = match &args[0] {
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

    let mut sum = Decimal::ZERO;
    let mut count = 0;
    for (r_idx, row) in avg_range.iter().enumerate() {
        for (c_idx, avg_cell) in row.iter().enumerate() {
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
                if let Ok(d) = avg_cell.to_decimal() {
                    sum += d;
                    count += 1;
                }
            }
        }
    }
    if count == 0 {
        return Err(CellError::Div0);
    }
    Ok(Value::Decimal(sum / Decimal::from(count)))
}

pub fn count(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut count = 0;
    for val in flat {
        if val.is_number() || matches!(val, Value::Date(_) | Value::Timestamp(_)) {
            count += 1;
        }
    }
    Ok(Value::Integer(count))
}

pub fn counta(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut count = 0;
    for val in flat {
        if !val.is_blank() {
            count += 1;
        }
    }
    Ok(Value::Integer(count))
}

pub fn countblank(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut count = 0;
    for val in flat {
        if val.is_blank() || val.is_empty_text() {
            count += 1;
        }
    }
    Ok(Value::Integer(count))
}

pub fn countif(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let flat = flatten_values(&args[0..1]);
    let criteria = &args[1];
    let mut c = 0;
    for cell in flat {
        if matches_criteria(&cell, criteria) {
            c += 1;
        }
    }
    Ok(Value::Integer(c))
}

pub fn countifs(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 || args.len() % 2 != 0 {
        return Err(CellError::Value);
    }
    let mut criteria_pairs = Vec::new();
    let mut idx = 0;
    while idx < args.len() {
        let range = match &args[idx] {
            Value::Array(rows) => rows,
            _ => return Err(CellError::Value),
        };
        let crit = &args[idx + 1];
        criteria_pairs.push((range, crit));
        idx += 2;
    }

    let first_range = criteria_pairs[0].0;
    let mut total = 0;
    for (r_idx, row) in first_range.iter().enumerate() {
        for (c_idx, _) in row.iter().enumerate() {
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
                total += 1;
            }
        }
    }
    Ok(Value::Integer(total))
}

pub fn min(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut min_val: Option<Decimal> = None;
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        if val.is_number() {
            if let Ok(d) = val.to_decimal() {
                min_val = Some(match min_val {
                    Some(m) => m.min(d),
                    None => d,
                });
            }
        }
    }
    match min_val {
        Some(m) => Ok(Value::Decimal(m)),
        None => Ok(Value::Decimal(Decimal::ZERO)),
    }
}

pub fn max(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut max_val: Option<Decimal> = None;
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        if val.is_number() {
            if let Ok(d) = val.to_decimal() {
                max_val = Some(match max_val {
                    Some(m) => m.max(d),
                    None => d,
                });
            }
        }
    }
    match max_val {
        Some(m) => Ok(Value::Decimal(m)),
        None => Ok(Value::Decimal(Decimal::ZERO)),
    }
}

pub fn median(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut nums: Vec<Decimal> = Vec::new();
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        if val.is_number() {
            if let Ok(d) = val.to_decimal() {
                nums.push(d);
            }
        }
    }
    if nums.is_empty() {
        return Err(CellError::Num);
    }
    nums.sort();
    let n = nums.len();
    if n % 2 == 1 {
        Ok(Value::Decimal(nums[n / 2]))
    } else {
        let m = (nums[n / 2 - 1] + nums[n / 2]) / Decimal::from(2);
        Ok(Value::Decimal(m))
    }
}

pub fn var_s(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut nums: Vec<f64> = Vec::new();
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        if val.is_number() {
            if let Ok(f) = val.to_f64() {
                nums.push(f);
            }
        }
    }
    if nums.len() < 2 {
        return Err(CellError::Div0);
    }
    let mean = nums.iter().sum::<f64>() / (nums.len() as f64);
    let var = nums
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / ((nums.len() - 1) as f64);
    Decimal::from_f64(var)
        .map(Value::Decimal)
        .ok_or(CellError::Num)
}

pub fn var_p(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut nums: Vec<f64> = Vec::new();
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        if val.is_number() {
            if let Ok(f) = val.to_f64() {
                nums.push(f);
            }
        }
    }
    if nums.is_empty() {
        return Err(CellError::Div0);
    }
    let mean = nums.iter().sum::<f64>() / (nums.len() as f64);
    let var = nums
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / (nums.len() as f64);
    Decimal::from_f64(var)
        .map(Value::Decimal)
        .ok_or(CellError::Num)
}

pub fn stdev_s(args: &[Value]) -> Result<Value, CellError> {
    let var_val = var_s(args)?;
    let f = var_val.to_f64()?.sqrt();
    Decimal::from_f64(f)
        .map(Value::Decimal)
        .ok_or(CellError::Num)
}

pub fn stdev_p(args: &[Value]) -> Result<Value, CellError> {
    let var_val = var_p(args)?;
    let f = var_val.to_f64()?.sqrt();
    Decimal::from_f64(f)
        .map(Value::Decimal)
        .ok_or(CellError::Num)
}
