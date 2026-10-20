use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::error::CellError;
use crate::functions::{call_function, EvaluationContext};
use crate::value::Value;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

pub fn evaluate_expr<C: EvaluationContext + ?Sized>(
    expr: &Expr,
    ctx: &C,
) -> Result<Value, CellError> {
    match expr {
        Expr::Literal(val) => Ok(val.clone()),
        Expr::CellRef(coord) => ctx.get_cell_value(*coord),
        Expr::RangeRef(range) => ctx.get_range_values(*range),
        Expr::SpillRef(coord) => {
            let anchor_val = ctx.get_cell_value(*coord)?;
            match anchor_val {
                Value::Array(_) => Ok(anchor_val),
                _ => Err(CellError::Ref),
            }
        }
        Expr::StructuredRef(sref) => ctx.get_table_column(sref),
        Expr::Unary { op, expr } => {
            let val = evaluate_expr(expr, ctx)?;
            if val.is_error() {
                return Ok(val);
            }
            match op {
                UnaryOp::Plus => {
                    let d = val.to_decimal()?;
                    Ok(Value::Decimal(d))
                }
                UnaryOp::Minus => {
                    let d = val.to_decimal()?;
                    Ok(Value::Decimal(-d))
                }
                UnaryOp::Percent => {
                    let d = val.to_decimal()?;
                    Ok(Value::Decimal(d / Decimal::from(100)))
                }
            }
        }
        Expr::Binary { op, left, right } => {
            let left_val = evaluate_expr(left, ctx)?;
            if left_val.is_error() {
                return Ok(left_val);
            }
            let right_val = evaluate_expr(right, ctx)?;
            if right_val.is_error() {
                return Ok(right_val);
            }

            match op {
                BinaryOp::Add => {
                    let a = left_val.to_decimal()?;
                    let b = right_val.to_decimal()?;
                    Ok(Value::Decimal(a + b))
                }
                BinaryOp::Sub => {
                    let a = left_val.to_decimal()?;
                    let b = right_val.to_decimal()?;
                    Ok(Value::Decimal(a - b))
                }
                BinaryOp::Mul => {
                    let a = left_val.to_decimal()?;
                    let b = right_val.to_decimal()?;
                    Ok(Value::Decimal(a * b))
                }
                BinaryOp::Div => {
                    let a = left_val.to_decimal()?;
                    let b = right_val.to_decimal()?;
                    if b.is_zero() {
                        return Err(CellError::Div0);
                    }
                    Ok(Value::Decimal(a / b))
                }
                BinaryOp::Power => {
                    let a = left_val.to_f64()?;
                    let b = right_val.to_f64()?;
                    let res = a.powf(b);
                    if res.is_nan() || res.is_infinite() {
                        return Err(CellError::Num);
                    }
                    if let Some(d) = Decimal::from_f64(res) {
                        Ok(Value::Decimal(d))
                    } else {
                        Ok(Value::Float(res))
                    }
                }
                BinaryOp::Concat => {
                    let a = left_val.to_string_repr();
                    let b = right_val.to_string_repr();
                    Ok(Value::Text(format!("{}{}", a, b)))
                }
                BinaryOp::Eq => Ok(Value::Boolean(left_val == right_val)),
                BinaryOp::NotEq => Ok(Value::Boolean(left_val != right_val)),
                BinaryOp::Lt => Ok(Value::Boolean(left_val < right_val)),
                BinaryOp::LtEq => Ok(Value::Boolean(left_val <= right_val)),
                BinaryOp::Gt => Ok(Value::Boolean(left_val > right_val)),
                BinaryOp::GtEq => Ok(Value::Boolean(left_val >= right_val)),
            }
        }
        Expr::FunctionCall { name, args } => {
            let mut evaluated_args = Vec::new();
            for arg_expr in args {
                evaluated_args.push(evaluate_expr(arg_expr, ctx)?);
            }
            call_function(name, &evaluated_args, ctx)
        }
        Expr::ArrayLiteral(rows) => {
            let mut res_rows = Vec::new();
            for row in rows {
                let mut res_row = Vec::new();
                for cell_expr in row {
                    res_row.push(evaluate_expr(cell_expr, ctx)?);
                }
                res_rows.push(res_row);
            }
            Ok(Value::Array(res_rows))
        }
    }
}
