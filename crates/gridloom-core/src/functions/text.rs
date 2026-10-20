use crate::error::CellError;
use crate::functions::arithmetic::flatten_values;
use crate::value::Value;
use rust_decimal::Decimal;

pub fn concat(args: &[Value]) -> Result<Value, CellError> {
    let flat = flatten_values(args);
    let mut out = String::new();
    for val in flat {
        if val.is_error() {
            return Err(val.as_error().unwrap().clone());
        }
        out.push_str(&val.to_string_repr());
    }
    Ok(Value::Text(out))
}

pub fn textjoin(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let delimiter = args[0].to_string_repr();
    let ignore_empty = args[1].to_bool()?;
    let text_items = flatten_values(&args[2..]);

    let mut parts = Vec::new();
    for item in text_items {
        if item.is_error() {
            return Err(item.as_error().unwrap().clone());
        }
        let s = item.to_string_repr();
        if ignore_empty && s.is_empty() {
            continue;
        }
        parts.push(s);
    }
    Ok(Value::Text(parts.join(&delimiter)))
}

pub fn left(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    let num_chars = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        1
    };
    if num_chars < 0 {
        return Err(CellError::Value);
    }
    let chars: String = text.chars().take(num_chars as usize).collect();
    Ok(Value::Text(chars))
}

pub fn right(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    let num_chars = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        1
    };
    if num_chars < 0 {
        return Err(CellError::Value);
    }
    let total_chars = text.chars().count();
    let skip = if (num_chars as usize) >= total_chars {
        0
    } else {
        total_chars - (num_chars as usize)
    };
    let chars: String = text.chars().skip(skip).collect();
    Ok(Value::Text(chars))
}

pub fn mid(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    let start_num = args[1].to_i64()?; // 1-based in Excel
    let num_chars = args[2].to_i64()?;
    if start_num < 1 || num_chars < 0 {
        return Err(CellError::Value);
    }
    let skip = (start_num - 1) as usize;
    let chars: String = text.chars().skip(skip).take(num_chars as usize).collect();
    Ok(Value::Text(chars))
}

pub fn len(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    Ok(Value::Integer(text.chars().count() as i64))
}

pub fn trim(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    // Excel TRIM removes leading, trailing, and reduces consecutive spaces to single space
    let words: Vec<&str> = text.split_whitespace().collect();
    Ok(Value::Text(words.join(" ")))
}

pub fn clean(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    let cleaned: String = text.chars().filter(|c| !c.is_control()).collect();
    Ok(Value::Text(cleaned))
}

pub fn upper(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    Ok(Value::Text(text.to_uppercase()))
}

pub fn lower(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    Ok(Value::Text(text.to_lowercase()))
}

pub fn substitute(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    let old_text = args[1].to_string_repr();
    let new_text = args[2].to_string_repr();

    if args.len() > 3 {
        let instance = args[3].to_i64()?;
        if instance < 1 {
            return Err(CellError::Value);
        }
        let mut occurrences = 0;
        let mut out = String::new();
        let mut remainder = text.as_str();

        while let Some(pos) = remainder.find(&old_text) {
            occurrences += 1;
            out.push_str(&remainder[..pos]);
            if occurrences == instance {
                out.push_str(&new_text);
                out.push_str(&remainder[pos + old_text.len()..]);
                return Ok(Value::Text(out));
            } else {
                out.push_str(&old_text);
            }
            remainder = &remainder[pos + old_text.len()..];
        }
        out.push_str(remainder);
        Ok(Value::Text(out))
    } else {
        Ok(Value::Text(text.replace(&old_text, &new_text)))
    }
}

pub fn replace(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 4 {
        return Err(CellError::Value);
    }
    let text = args[0].to_string_repr();
    let start_num = args[1].to_i64()?; // 1-based
    let num_chars = args[2].to_i64()?;
    let new_text = args[3].to_string_repr();

    if start_num < 1 || num_chars < 0 {
        return Err(CellError::Value);
    }
    let start_idx = (start_num - 1) as usize;
    let count = num_chars as usize;

    let chars: Vec<char> = text.chars().collect();
    if start_idx > chars.len() {
        return Err(CellError::Value);
    }
    let end_idx = (start_idx + count).min(chars.len());

    let prefix: String = chars[..start_idx].iter().collect();
    let suffix: String = chars[end_idx..].iter().collect();

    Ok(Value::Text(format!("{}{}{}", prefix, new_text, suffix)))
}

pub fn find(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let find_text = args[0].to_string_repr();
    let within_text = args[1].to_string_repr();
    let start_num = if args.len() > 2 {
        args[2].to_i64()?
    } else {
        1
    };
    if start_num < 1 {
        return Err(CellError::Value);
    }
    let skip = (start_num - 1) as usize;
    let chars: Vec<char> = within_text.chars().collect();
    if skip > chars.len() {
        return Err(CellError::Value);
    }
    let substr: String = chars[skip..].iter().collect();

    match substr.find(&find_text) {
        Some(byte_pos) => {
            let char_offset = substr[..byte_pos].chars().count();
            Ok(Value::Integer((skip + char_offset + 1) as i64))
        }
        None => Err(CellError::Value),
    }
}

pub fn search(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let find_text = args[0].to_string_repr().to_lowercase();
    let within_text = args[1].to_string_repr().to_lowercase();
    let start_num = if args.len() > 2 {
        args[2].to_i64()?
    } else {
        1
    };
    if start_num < 1 {
        return Err(CellError::Value);
    }
    let skip = (start_num - 1) as usize;
    let chars: Vec<char> = within_text.chars().collect();
    if skip > chars.len() {
        return Err(CellError::Value);
    }
    let substr: String = chars[skip..].iter().collect();

    match substr.find(&find_text) {
        Some(byte_pos) => {
            let char_offset = substr[..byte_pos].chars().count();
            Ok(Value::Integer((skip + char_offset + 1) as i64))
        }
        None => Err(CellError::Value),
    }
}

pub fn value_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let s = args[0].to_string_repr();
    let trimmed = s.trim().trim_start_matches('$').replace(',', "");
    if let Ok(d) = Decimal::from_str_exact(&trimmed) {
        Ok(Value::Decimal(d))
    } else if let Ok(f) = trimmed.parse::<f64>() {
        Ok(Value::Float(f))
    } else {
        Err(CellError::Value)
    }
}

pub fn text_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let val = &args[0];
    let format_str = args[1].to_string_repr();

    // Standard number format handling: "0.00", "0%", "$#,##0.00", "yyyy-mm-dd"
    if format_str.contains("yyyy") || format_str.contains("mm") || format_str.contains("dd") {
        match val {
            Value::Date(d) => Ok(Value::Text(d.format("%Y-%m-%d").to_string())),
            Value::Timestamp(ts) => Ok(Value::Text(ts.format("%Y-%m-%d").to_string())),
            _ => Ok(Value::Text(val.to_string_repr())),
        }
    } else if format_str.ends_with('%') {
        let d = val.to_decimal()? * Decimal::from(100);
        let decimals = format_str.chars().filter(|&c| c == '0').count().saturating_sub(1);
        Ok(Value::Text(format!("{:.decimals$}%", d, decimals = decimals)))
    } else if format_str.contains('.') {
        let d = val.to_decimal()?;
        let decimals = format_str.split('.').nth(1).map_or(0, |s| s.len());
        Ok(Value::Text(format!("{:.decimals$}", d, decimals = decimals)))
    } else {
        Ok(Value::Text(val.to_string_repr()))
    }
}
