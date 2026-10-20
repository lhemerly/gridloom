pub mod arithmetic;
pub mod date;
pub mod financial;
pub mod logic;
pub mod lookup;
pub mod statistics;
pub mod text;

use crate::error::CellError;
use crate::value::Value;

pub trait EvaluationContext {
    fn get_cell_value(&self, coord: crate::value::CellCoord) -> Result<Value, CellError>;
    fn get_range_values(&self, range: crate::value::CellRange) -> Result<Value, CellError>;
    fn get_table_column(&self, sref: &crate::ast::StructuredRef) -> Result<Value, CellError>;
    fn execute_sql_value(&self, connection_name: &str, sql: &str, params: &[Value]) -> Result<Value, CellError>;
    fn execute_sql_table(&self, connection_name: &str, sql: &str, params: &[Value]) -> Result<Value, CellError>;
}

pub fn call_function<C: EvaluationContext + ?Sized>(
    name: &str,
    args: &[Value],
    ctx: &C,
) -> Result<Value, CellError> {
    let upper = name.to_uppercase();
    match upper.as_str() {
        // Arithmetic
        "SUM" => arithmetic::sum(args),
        "SUMIF" => arithmetic::sumif(args),
        "SUMIFS" => arithmetic::sumifs(args),
        "SUMPRODUCT" => arithmetic::sumproduct(args),
        "PRODUCT" => arithmetic::product(args),
        "ABS" => arithmetic::abs(args),
        "SIGN" => arithmetic::sign(args),
        "MOD" => arithmetic::mod_fn(args),
        "POWER" => arithmetic::power(args),
        "SQRT" => arithmetic::sqrt(args),
        "ROUND" => arithmetic::round(args),
        "ROUNDUP" => arithmetic::roundup(args),
        "ROUNDDOWN" => arithmetic::rounddown(args),
        "INT" => arithmetic::int_fn(args),
        "TRUNC" => arithmetic::trunc(args),
        "CEILING.MATH" | "CEILING" => arithmetic::ceiling_math(args),
        "FLOOR.MATH" | "FLOOR" => arithmetic::floor_math(args),

        // Statistics
        "AVERAGE" => statistics::average(args),
        "AVERAGEIF" => statistics::averageif(args),
        "AVERAGEIFS" => statistics::averageifs(args),
        "COUNT" => statistics::count(args),
        "COUNTA" => statistics::counta(args),
        "COUNTBLANK" => statistics::countblank(args),
        "COUNTIF" => statistics::countif(args),
        "COUNTIFS" => statistics::countifs(args),
        "MIN" => statistics::min(args),
        "MAX" => statistics::max(args),
        "MEDIAN" => statistics::median(args),
        "STDEV.S" | "STDEV" => statistics::stdev_s(args),
        "STDEV.P" | "STDEVP" => statistics::stdev_p(args),
        "VAR.S" | "VAR" => statistics::var_s(args),
        "VAR.P" | "VARP" => statistics::var_p(args),

        // Logic & Inspection
        "IF" => logic::if_fn(args),
        "IFS" => logic::ifs_fn(args),
        "AND" => logic::and_fn(args),
        "OR" => logic::or_fn(args),
        "NOT" => logic::not_fn(args),
        "IFERROR" => logic::iferror(args),
        "IFNA" => logic::ifna(args),
        "ISBLANK" => logic::isblank(args),
        "ISNUMBER" => logic::isnumber(args),
        "ISTEXT" => logic::istext(args),
        "ISERROR" => logic::iserror(args),
        "ISNA" => logic::isna(args),

        // Text
        "CONCAT" | "CONCATENATE" => text::concat(args),
        "TEXTJOIN" => text::textjoin(args),
        "LEFT" => text::left(args),
        "RIGHT" => text::right(args),
        "MID" => text::mid(args),
        "LEN" => text::len(args),
        "TRIM" => text::trim(args),
        "CLEAN" => text::clean(args),
        "UPPER" => text::upper(args),
        "LOWER" => text::lower(args),
        "SUBSTITUTE" => text::substitute(args),
        "REPLACE" => text::replace(args),
        "FIND" => text::find(args),
        "SEARCH" => text::search(args),
        "VALUE" => text::value_fn(args),
        "TEXT" => text::text_fn(args),

        // Dates
        "DATE" => date::date_fn(args),
        "YEAR" => date::year_fn(args),
        "MONTH" => date::month_fn(args),
        "DAY" => date::day_fn(args),
        "TODAY" => date::today_fn(args),
        "NOW" => date::now_fn(args),
        "EDATE" => date::edate(args),
        "EOMONTH" => date::eomonth(args),
        "DAYS" => date::days(args),
        "WEEKDAY" => date::weekday(args),
        "NETWORKDAYS" => date::networkdays(args),
        "WORKDAY" => date::workday(args),

        // Worksheet Lookup & Arrays
        "XLOOKUP" => lookup::xlookup(args),
        "VLOOKUP" => lookup::vlookup(args),
        "INDEX" => lookup::index_fn(args),
        "MATCH" => lookup::match_fn(args),
        "FILTER" => lookup::filter_fn(args),
        "SORT" => lookup::sort_fn(args),
        "UNIQUE" => lookup::unique_fn(args),
        "SEQUENCE" => lookup::sequence_fn(args),
        "TRANSPOSE" => lookup::transpose_fn(args),

        // Financial
        "PMT" => financial::pmt(args),
        "PV" => financial::pv(args),
        "FV" => financial::fv(args),
        "NPV" => financial::npv(args),
        "IRR" => financial::irr(args),

        // SQL First-Class Formulas
        "SQLVALUE" => {
            if args.len() < 2 {
                return Err(CellError::Value);
            }
            let conn_name = args[0].to_string_repr();
            let sql = args[1].to_string_repr();
            let params = &args[2..];
            ctx.execute_sql_value(&conn_name, &sql, params)
        }
        "SQLTABLE" => {
            if args.len() < 2 {
                return Err(CellError::Value);
            }
            let conn_name = args[0].to_string_repr();
            let sql = args[1].to_string_repr();
            let params = &args[2..];
            ctx.execute_sql_table(&conn_name, &sql, params)
        }

        _ => Err(CellError::Name),
    }
}
