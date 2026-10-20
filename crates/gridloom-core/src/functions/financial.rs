use crate::error::CellError;
use crate::functions::arithmetic::flatten_values;
use crate::value::Value;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

pub fn pmt(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let rate = args[0].to_f64()?;
    let nper = args[1].to_f64()?;
    let pv = args[2].to_f64()?;
    let fv = if args.len() > 3 {
        args[3].to_f64()?
    } else {
        0.0
    };
    let p_type = if args.len() > 4 {
        args[4].to_i64()?
    } else {
        0
    }; // 0 = end of period, 1 = beginning

    if nper == 0.0 {
        return Err(CellError::Div0);
    }

    let pmt_val = if rate == 0.0 {
        -(pv + fv) / nper
    } else {
        let pvif = (1.0 + rate).powf(nper);
        let num = rate * (pv * pvif + fv);
        let denom = (1.0 + rate * (p_type as f64)) * (pvif - 1.0);
        -num / denom
    };

    if let Some(d) = Decimal::from_f64(pmt_val) {
        Ok(Value::Decimal(d))
    } else {
        Ok(Value::Float(pmt_val))
    }
}

pub fn pv(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let rate = args[0].to_f64()?;
    let nper = args[1].to_f64()?;
    let pmt = args[2].to_f64()?;
    let fv = if args.len() > 3 {
        args[3].to_f64()?
    } else {
        0.0
    };
    let p_type = if args.len() > 4 {
        args[4].to_i64()?
    } else {
        0
    };

    let pv_val = if rate == 0.0 {
        -(pmt * nper + fv)
    } else {
        let pvif = (1.0 + rate).powf(nper);
        let denom = pvif;
        let pmt_factor = (1.0 + rate * (p_type as f64)) * (pvif - 1.0) / rate;
        -(fv + pmt * pmt_factor) / denom
    };

    if let Some(d) = Decimal::from_f64(pv_val) {
        Ok(Value::Decimal(d))
    } else {
        Ok(Value::Float(pv_val))
    }
}

pub fn fv(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let rate = args[0].to_f64()?;
    let nper = args[1].to_f64()?;
    let pmt = args[2].to_f64()?;
    let pv = if args.len() > 3 {
        args[3].to_f64()?
    } else {
        0.0
    };
    let p_type = if args.len() > 4 {
        args[4].to_i64()?
    } else {
        0
    };

    let fv_val = if rate == 0.0 {
        -(pv + pmt * nper)
    } else {
        let pvif = (1.0 + rate).powf(nper);
        let pmt_factor = (1.0 + rate * (p_type as f64)) * (pvif - 1.0) / rate;
        -(pv * pvif + pmt * pmt_factor)
    };

    if let Some(d) = Decimal::from_f64(fv_val) {
        Ok(Value::Decimal(d))
    } else {
        Ok(Value::Float(fv_val))
    }
}

pub fn npv(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let rate = args[0].to_f64()?;
    if rate <= -1.0 {
        return Err(CellError::Num);
    }

    let values = flatten_values(&args[1..]);
    let mut npv_val = 0.0;
    for (i, val) in values.into_iter().enumerate() {
        let cf = val.to_f64()?;
        let t = (i + 1) as f64;
        npv_val += cf / (1.0 + rate).powf(t);
    }

    if let Some(d) = Decimal::from_f64(npv_val) {
        Ok(Value::Decimal(d))
    } else {
        Ok(Value::Float(npv_val))
    }
}

pub fn irr(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let values_flat = flatten_values(&args[0..1]);
    let mut cfs = Vec::new();
    for v in values_flat {
        cfs.push(v.to_f64()?);
    }

    if cfs.len() < 2 {
        return Err(CellError::Num);
    }

    let guess = if args.len() > 1 {
        args[1].to_f64()?
    } else {
        0.1
    };

    // Newton-Raphson method to solve NPV(rate) = 0
    let mut rate = guess;
    let max_iter = 100;
    let tol = 1e-7;

    for _ in 0..max_iter {
        let mut npv = 0.0;
        let mut d_npv = 0.0;

        for (t, &cf) in cfs.iter().enumerate() {
            let denom = (1.0 + rate).powi(t as i32);
            npv += cf / denom;
            if t > 0 {
                d_npv -= (t as f64) * cf / (1.0 + rate).powi((t + 1) as i32);
            }
        }

        if d_npv.abs() < 1e-12 {
            return Err(CellError::Num);
        }

        let new_rate = rate - npv / d_npv;
        if (new_rate - rate).abs() < tol {
            return if let Some(d) = Decimal::from_f64(new_rate) {
                Ok(Value::Decimal(d))
            } else {
                Ok(Value::Float(new_rate))
            };
        }
        rate = new_rate;
    }

    Err(CellError::Num)
}
