use crate::error::CellError;
use crate::value::Value;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PivotAggregation {
    Count,
    Sum,
    Average,
    Min,
    Max,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotDefinition {
    pub name: String,
    pub source_table: String,
    pub row_fields: Vec<String>,
    pub col_fields: Vec<String>,
    pub value_field: String,
    pub aggregation: PivotAggregation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotResult {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

pub fn compute_pivot(
    def: &PivotDefinition,
    column_names: &[String],
    source_rows: &[Vec<Value>],
) -> Result<PivotResult, CellError> {
    let mut field_indices = HashMap::new();
    for (i, name) in column_names.iter().enumerate() {
        field_indices.insert(name.to_lowercase(), i);
    }

    let val_idx = *field_indices
        .get(&def.value_field.to_lowercase())
        .ok_or(CellError::Value)?;

    let row_indices: Vec<usize> = def
        .row_fields
        .iter()
        .map(|f| field_indices.get(&f.to_lowercase()).copied().ok_or(CellError::Value))
        .collect::<Result<_, _>>()?;

    // Map: GroupKey -> Vec<Value>
    let mut groups: BTreeMap<Vec<String>, Vec<Value>> = BTreeMap::new();

    for r in source_rows {
        let mut key = Vec::new();
        for &idx in &row_indices {
            let cell = r.get(idx).unwrap_or(&Value::Blank);
            key.push(cell.to_string_repr());
        }
        let val = r.get(val_idx).cloned().unwrap_or(Value::Blank);
        groups.entry(key).or_default().push(val);
    }

    let mut headers = def.row_fields.clone();
    headers.push(format!("{:?}({})", def.aggregation, def.value_field));

    let mut result_rows = Vec::new();

    for (key, vals) in groups {
        let agg_value = match def.aggregation {
            PivotAggregation::Count => {
                let count = vals.iter().filter(|v| !v.is_blank()).count();
                Value::Integer(count as i64)
            }
            PivotAggregation::Sum => {
                let mut sum = Decimal::ZERO;
                for v in &vals {
                    if let Ok(d) = v.to_decimal() {
                        sum += d;
                    }
                }
                Value::Decimal(sum)
            }
            PivotAggregation::Average => {
                let mut sum = Decimal::ZERO;
                let mut count = 0;
                for v in &vals {
                    if let Ok(d) = v.to_decimal() {
                        sum += d;
                        count += 1;
                    }
                }
                if count > 0 {
                    Value::Decimal(sum / Decimal::from(count))
                } else {
                    Value::Error(CellError::Div0)
                }
            }
            PivotAggregation::Min => {
                let mut min_val: Option<Decimal> = None;
                for v in &vals {
                    if let Ok(d) = v.to_decimal() {
                        min_val = Some(min_val.map_or(d, |m| m.min(d)));
                    }
                }
                min_val.map(Value::Decimal).unwrap_or(Value::Blank)
            }
            PivotAggregation::Max => {
                let mut max_val: Option<Decimal> = None;
                for v in &vals {
                    if let Ok(d) = v.to_decimal() {
                        max_val = Some(max_val.map_or(d, |m| m.max(d)));
                    }
                }
                max_val.map(Value::Decimal).unwrap_or(Value::Blank)
            }
        };

        let mut row = Vec::new();
        for k in key {
            row.push(Value::Text(k));
        }
        row.push(agg_value);
        result_rows.push(row);
    }

    Ok(PivotResult {
        headers,
        rows: result_rows,
    })
}
