use crate::error::CellError;
use crate::functions::arithmetic::flatten_values;
use crate::value::Value;

pub fn if_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let cond = args[0].to_bool()?;
    if cond {
        Ok(args[1].clone())
    } else if args.len() > 2 {
        Ok(args[2].clone())
    } else {
        Ok(Value::Boolean(false))
    }
}

pub fn ifs_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() || args.len() % 2 != 0 {
        return Err(CellError::Value);
    }
    let mut i = 0;
    while i < args.len() {
        let cond = args[i].to_bool()?;
        if cond {
            return Ok(args[i + 1].clone());
        }
        i += 2;
    }
    Err(CellError::NA)
}

pub fn and_fn(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    if flat.is_empty() {
        return Err(CellError::Value);
    }
    let mut has_bool = false;
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        if let Ok(b) = val.to_bool() {
            if !b {
                return Ok(Value::Boolean(false));
            }
            has_bool = true;
        }
    }
    if !has_bool {
        return Err(CellError::Value);
    }
    Ok(Value::Boolean(true))
}

pub fn or_fn(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    if flat.is_empty() {
        return Err(CellError::Value);
    }
    let mut has_bool = false;
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        if let Ok(b) = val.to_bool() {
            if b {
                return Ok(Value::Boolean(true));
            }
            has_bool = true;
        }
    }
    if !has_bool {
        return Err(CellError::Value);
    }
    Ok(Value::Boolean(false))
}

pub fn not_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let b = args[0].to_bool()?;
    Ok(Value::Boolean(!b))
}

pub fn iferror(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    if args[0].is_error() {
        Ok(args[1].clone())
    } else {
        Ok(args[0].clone())
    }
}

pub fn ifna(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    if let Value::Error(CellError::NA) = &args[0] {
        Ok(args[1].clone())
    } else {
        Ok(args[0].clone())
    }
}

pub fn isblank(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    Ok(Value::Boolean(args[0].is_blank()))
}

pub fn isnumber(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    Ok(Value::Boolean(args[0].is_number()))
}

pub fn istext(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    Ok(Value::Boolean(args[0].is_text()))
}

pub fn iserror(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    Ok(Value::Boolean(args[0].is_error()))
}

pub fn isna(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    Ok(Value::Boolean(matches!(args[0], Value::Error(CellError::NA))))
}
