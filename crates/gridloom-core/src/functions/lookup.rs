use crate::error::CellError;
use crate::value::Value;
use rust_decimal::Decimal;
use std::collections::HashSet;

fn to_matrix(val: &Value) -> Vec<Vec<Value>> {
    match val {
        Value::Array(rows) => rows.clone(),
        other => vec![vec![other.clone()]],
    }
}

pub fn index_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let matrix = to_matrix(&args[0]);
    let row_num = args[1].to_i64()?; // 1-based in Excel
    let col_num = if args.len() > 2 {
        args[2].to_i64()?
    } else {
        1
    };

    if row_num < 0 || col_num < 0 {
        return Err(CellError::Ref);
    }

    if row_num == 0 && col_num == 0 {
        return Ok(Value::Array(matrix));
    }

    if row_num == 0 {
        // Return entire column
        let c_idx = (col_num - 1) as usize;
        let mut col_vals = Vec::new();
        for r in &matrix {
            let cell = r.get(c_idx).cloned().unwrap_or(Value::Blank);
            col_vals.push(vec![cell]);
        }
        return Ok(Value::Array(col_vals));
    }

    if col_num == 0 {
        // Return entire row
        let r_idx = (row_num - 1) as usize;
        if let Some(r) = matrix.get(r_idx) {
            return Ok(Value::Array(vec![r.clone()]));
        } else {
            return Err(CellError::Ref);
        }
    }

    let r_idx = (row_num - 1) as usize;
    let c_idx = (col_num - 1) as usize;

    if let Some(r) = matrix.get(r_idx) {
        if let Some(cell) = r.get(c_idx) {
            Ok(cell.clone())
        } else {
            Err(CellError::Ref)
        }
    } else {
        Err(CellError::Ref)
    }
}

pub fn match_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let lookup_value = &args[0];
    let matrix = to_matrix(&args[1]);
    let match_type = if args.len() > 2 {
        args[2].to_i64()?
    } else {
        1
    };

    let flat: Vec<Value> = matrix.into_iter().flatten().collect();

    if match_type == 0 {
        // Exact match
        for (i, item) in flat.iter().enumerate() {
            if item == lookup_value {
                return Ok(Value::Integer((i + 1) as i64));
            }
        }
        Err(CellError::NA)
    } else if match_type > 0 {
        // Less than or equal to, assuming sorted ascending
        let mut last_idx = None;
        for (i, item) in flat.iter().enumerate() {
            if item <= lookup_value {
                last_idx = Some(i + 1);
            } else {
                break;
            }
        }
        last_idx.map(|idx| Value::Integer(idx as i64)).ok_or(CellError::NA)
    } else {
        // Greater than or equal to, assuming sorted descending
        let mut last_idx = None;
        for (i, item) in flat.iter().enumerate() {
            if item >= lookup_value {
                last_idx = Some(i + 1);
            } else {
                break;
            }
        }
        last_idx.map(|idx| Value::Integer(idx as i64)).ok_or(CellError::NA)
    }
}

pub fn vlookup(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let lookup_val = &args[0];
    let table = to_matrix(&args[1]);
    let col_index = args[2].to_i64()?; // 1-based
    let range_lookup = if args.len() > 3 {
        args[3].to_bool()?
    } else {
        true
    };

    if col_index < 1 {
        return Err(CellError::Value);
    }
    let c_idx = (col_index - 1) as usize;

    if !range_lookup {
        // Exact match
        for row in &table {
            if let Some(first) = row.first() {
                if first == lookup_val {
                    return Ok(row.get(c_idx).cloned().unwrap_or(Value::Blank));
                }
            }
        }
        Err(CellError::NA)
    } else {
        // Approximate match (largest value less than or equal)
        let mut best_row = None;
        for row in &table {
            if let Some(first) = row.first() {
                if first <= lookup_val {
                    best_row = Some(row);
                } else {
                    break;
                }
            }
        }
        if let Some(row) = best_row {
            Ok(row.get(c_idx).cloned().unwrap_or(Value::Blank))
        } else {
            Err(CellError::NA)
        }
    }
}

pub fn xlookup(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let lookup_val = &args[0];
    let lookup_array = to_matrix(&args[1]);
    let return_array = to_matrix(&args[2]);
    let if_not_found = if args.len() > 3 {
        Some(&args[3])
    } else {
        None
    };
    let match_mode = if args.len() > 4 {
        args[4].to_i64()?
    } else {
        0
    };
    let search_mode = if args.len() > 5 {
        args[5].to_i64()?
    } else {
        1
    };

    let flat_lookup: Vec<Value> = lookup_array.into_iter().flatten().collect();

    let indices: Vec<usize> = if search_mode == -1 {
        (0..flat_lookup.len()).rev().collect()
    } else {
        (0..flat_lookup.len()).collect()
    };

    let mut found_idx = None;
    for idx in indices {
        let item = &flat_lookup[idx];
        let matched = match match_mode {
            0 => item == lookup_val,
            -1 => item <= lookup_val,
            1 => item >= lookup_val,
            _ => item == lookup_val,
        };
        if matched {
            found_idx = Some(idx);
            break;
        }
    }

    if let Some(idx) = found_idx {
        // Return element or row/col from return_array
        if return_array.len() == 1 {
            // 1D row
            Ok(return_array[0].get(idx).cloned().unwrap_or(Value::Blank))
        } else if return_array.first().map_or(0, |r| r.len()) == 1 {
            // 1D column
            Ok(return_array.get(idx).and_then(|r| r.first()).cloned().unwrap_or(Value::Blank))
        } else {
            // Multi-column or 2D
            if let Some(r) = return_array.get(idx) {
                Ok(Value::Array(vec![r.clone()]))
            } else {
                Err(CellError::NA)
            }
        }
    } else if let Some(fallback) = if_not_found {
        Ok(fallback.clone())
    } else {
        Err(CellError::NA)
    }
}

pub fn filter_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let array = to_matrix(&args[0]);
    let include_matrix = to_matrix(&args[1]);
    let include_flat: Vec<bool> = include_matrix
        .into_iter()
        .flatten()
        .map(|v| v.to_bool().unwrap_or(false))
        .collect();

    if include_flat.len() != array.len() {
        return Err(CellError::Value);
    }

    let mut result_rows = Vec::new();
    for (i, row) in array.into_iter().enumerate() {
        if include_flat[i] {
            result_rows.push(row);
        }
    }

    if result_rows.is_empty() {
        if args.len() > 2 {
            Ok(args[2].clone())
        } else {
            Err(CellError::NA)
        }
    } else {
        Ok(Value::Array(result_rows))
    }
}

pub fn sort_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let mut matrix = to_matrix(&args[0]);
    let sort_index = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        1
    };
    let sort_order = if args.len() > 2 {
        args[2].to_i64()?
    } else {
        1
    }; // 1 = asc, -1 = desc

    if sort_index < 1 {
        return Err(CellError::Value);
    }
    let col_idx = (sort_index - 1) as usize;

    matrix.sort_by(|a, b| {
        let val_a = a.get(col_idx).unwrap_or(&Value::Blank);
        let val_b = b.get(col_idx).unwrap_or(&Value::Blank);
        if sort_order >= 0 {
            val_a.partial_cmp(val_b).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            val_b.partial_cmp(val_a).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    Ok(Value::Array(matrix))
}

pub fn unique_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let matrix = to_matrix(&args[0]);
    let mut seen = HashSet::new();
    let mut out_rows = Vec::new();

    for row in matrix {
        let key: Vec<String> = row.iter().map(|v| v.to_string_repr()).collect();
        if seen.insert(key) {
            out_rows.push(row);
        }
    }

    Ok(Value::Array(out_rows))
}

pub fn sequence_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let rows = args[0].to_i64()?;
    let cols = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        1
    };
    let start = if args.len() > 2 {
        args[2].to_decimal()?
    } else {
        Decimal::ONE
    };
    let step = if args.len() > 3 {
        args[3].to_decimal()?
    } else {
        Decimal::ONE
    };

    if rows < 1 || cols < 1 {
        return Err(CellError::Value);
    }

    let mut current = start;
    let mut matrix = Vec::new();
    for _ in 0..rows {
        let mut row_vals = Vec::new();
        for _ in 0..cols {
            row_vals.push(Value::Decimal(current));
            current += step;
        }
        matrix.push(row_vals);
    }

    Ok(Value::Array(matrix))
}

pub fn transpose_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let matrix = to_matrix(&args[0]);
    if matrix.is_empty() {
        return Ok(Value::Array(vec![]));
    }
    let row_count = matrix.len();
    let col_count = matrix[0].len();

    let mut transposed = vec![vec![Value::Blank; row_count]; col_count];
    for r in 0..row_count {
        for c in 0..col_count {
            transposed[c][r] = matrix[r][c].clone();
        }
    }

    Ok(Value::Array(transposed))
}
