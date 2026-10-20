use crate::error::CellError;
use crate::functions::arithmetic::flatten_values;
use crate::value::Value;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::prelude::ToPrimitive;

fn value_to_date(val: &Value) -> Result<NaiveDate, CellError> {
    match val {
        Value::Date(d) => Ok(*d),
        Value::Timestamp(ts) => Ok(ts.date_naive()),
        Value::Integer(i) => {
            // Excel serial date epoch: 1899-12-30
            let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            epoch.checked_add_signed(Duration::days(*i)).ok_or(CellError::Num)
        }
        Value::Decimal(d) => {
            let i = d.to_i64().ok_or(CellError::Num)?;
            let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            epoch.checked_add_signed(Duration::days(i)).ok_or(CellError::Num)
        }
        Value::Float(f) => {
            let i = *f as i64;
            let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            epoch.checked_add_signed(Duration::days(i)).ok_or(CellError::Num)
        }
        Value::Text(s) => {
            if let Ok(d) = NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d") {
                Ok(d)
            } else if let Ok(d) = NaiveDate::parse_from_str(s.trim(), "%m/%d/%Y") {
                Ok(d)
            } else if let Ok(d) = NaiveDate::parse_from_str(s.trim(), "%d/%m/%Y") {
                Ok(d)
            } else {
                Err(CellError::Value)
            }
        }
        _ => Err(CellError::Value),
    }
}

pub fn date_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 3 {
        return Err(CellError::Value);
    }
    let year = args[0].to_i64()? as i32;
    let month = args[1].to_i64()? as u32;
    let day = args[2].to_i64()? as u32;

    NaiveDate::from_ymd_opt(year, month, day)
        .map(Value::Date)
        .ok_or(CellError::Num)
}

pub fn year_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let date = value_to_date(&args[0])?;
    Ok(Value::Integer(date.year() as i64))
}

pub fn month_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let date = value_to_date(&args[0])?;
    Ok(Value::Integer(date.month() as i64))
}

pub fn day_fn(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let date = value_to_date(&args[0])?;
    Ok(Value::Integer(date.day() as i64))
}

pub fn today_fn(_args: &[Value]) -> Result<Value, CellError> {
    let now = Utc::now().date_naive();
    Ok(Value::Date(now))
}

pub fn now_fn(_args: &[Value]) -> Result<Value, CellError> {
    Ok(Value::Timestamp(Utc::now()))
}

pub fn edate(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let date = value_to_date(&args[0])?;
    let months = args[1].to_i64()?;

    let mut year = date.year();
    let mut month = date.month() as i64 + months;

    while month > 12 {
        year += 1;
        month -= 12;
    }
    while month < 1 {
        year -= 1;
        month += 12;
    }

    let mut day = date.day();
    // Clamp to end of month if necessary
    while day > 28 && NaiveDate::from_ymd_opt(year, month as u32, day).is_none() {
        day -= 1;
    }

    NaiveDate::from_ymd_opt(year, month as u32, day)
        .map(Value::Date)
        .ok_or(CellError::Num)
}

pub fn eomonth(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let date = value_to_date(&args[0])?;
    let months = args[1].to_i64()?;

    let mut year = date.year();
    let mut month = date.month() as i64 + months + 1;

    while month > 12 {
        year += 1;
        month -= 12;
    }
    while month < 1 {
        year -= 1;
        month += 12;
    }

    // 1st of next month minus 1 day
    let first_next = NaiveDate::from_ymd_opt(year, month as u32, 1).ok_or(CellError::Num)?;
    let last = first_next - Duration::days(1);
    Ok(Value::Date(last))
}

pub fn days(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let end_date = value_to_date(&args[0])?;
    let start_date = value_to_date(&args[1])?;
    Ok(Value::Integer((end_date - start_date).num_days()))
}

pub fn weekday(args: &[Value]) -> Result<Value, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let date = value_to_date(&args[0])?;
    let return_type = if args.len() > 1 {
        args[1].to_i64()?
    } else {
        1
    };

    // chrono weekday: Mon=0 .. Sun=6
    let d = date.weekday().num_days_from_sunday(); // Sun=0, Mon=1, Tue=2, Wed=3, Thu=4, Fri=5, Sat=6
    match return_type {
        1 => Ok(Value::Integer((d + 1) as i64)), // 1=Sun .. 7=Sat
        2 => {
            let m = date.weekday().num_days_from_monday();
            Ok(Value::Integer((m + 1) as i64)) // 1=Mon .. 7=Sun
        }
        3 => {
            let m = date.weekday().num_days_from_monday();
            Ok(Value::Integer(m as i64)) // 0=Mon .. 6=Sun
        }
        _ => Err(CellError::Num),
    }
}

pub fn networkdays(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let start = value_to_date(&args[0])?;
    let end = value_to_date(&args[1])?;

    let mut holidays = Vec::new();
    if args.len() > 2 {
        for h in flatten_values(&args[2..]) {
            if let Ok(d) = value_to_date(&h) {
                holidays.push(d);
            }
        }
    }

    if start > end {
        // Negative count
        let count = count_workdays(end, start, &holidays);
        Ok(Value::Integer(-count))
    } else {
        let count = count_workdays(start, end, &holidays);
        Ok(Value::Integer(count))
    }
}

fn count_workdays(start: NaiveDate, end: NaiveDate, holidays: &[NaiveDate]) -> i64 {
    let mut count = 0;
    let mut cur = start;
    while cur <= end {
        let wd = cur.weekday().num_days_from_monday(); // 0..=6 where 5=Sat, 6=Sun
        if wd < 5 && !holidays.contains(&cur) {
            count += 1;
        }
        cur += Duration::days(1);
    }
    count
}

pub fn workday(args: &[Value]) -> Result<Value, CellError> {
    if args.len() < 2 {
        return Err(CellError::Value);
    }
    let mut cur = value_to_date(&args[0])?;
    let days = args[1].to_i64()?;

    let mut holidays = Vec::new();
    if args.len() > 2 {
        for h in flatten_values(&args[2..]) {
            if let Ok(d) = value_to_date(&h) {
                holidays.push(d);
            }
        }
    }

    let step = if days >= 0 { 1 } else { -1 };
    let mut remaining = days.abs();

    while remaining > 0 {
        cur += Duration::days(step);
        let wd = cur.weekday().num_days_from_monday();
        if wd < 5 && !holidays.contains(&cur) {
            remaining -= 1;
        }
    }

    Ok(Value::Date(cur))
}
