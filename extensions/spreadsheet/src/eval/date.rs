//! The date and time functions (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! A date is a serial number of days and a time its fraction. In the 1900 system Excel counts a
//! 1900-02-29 that never was, as serial 60: serials 1 to 59 are one day off a real calendar, every
//! later serial counts days from 1899-12-30, and serial 0 reads as 1900-01-00. In the 1904 system
//! serial 0 is 1904-01-01. The last date is 9999-12-31 in both, and a date past it or below 0 is
//! `#NUM!`. `HOUR`, `MINUTE` and `SECOND` round to the nearest second first.
//!
//! A date argument converts as arithmetic does, so an ISO date text is a date. A boolean argument
//! is listed, since Excel's date functions differ in whether they take one. `DATEDIF`, `EDATE`
//! and `EOMONTH` list a date in January or February 1900, whose calendar Excel's functions do not
//! agree on. `DATEDIF` lists `YD` when a leap year is involved, and `MD` when the start day is
//! past the end day and past the length of the end month or the month before it: the inputs where
//! Excel's own answers are known to be off.

use super::functions::{arity, error, listed, number};
use super::parse::Expr;
use super::value::Value;
use super::{Ctx, Miss, Operand};
use crate::read::CellError;

/// The value of a `Result<_, CellError>`, or its error as the function's result.
macro_rules! take {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return error(e),
        }
    };
}

/// The serial of 9999-12-31 in the 1900 system and in the 1904 system.
const LAST_1900: i64 = 2_958_465;
const LAST_1904: i64 = 2_957_003;

fn last(ctx: &Ctx<'_>) -> i64 {
    if ctx.date1904() { LAST_1904 } else { LAST_1900 }
}

/// Days from 1970-01-01 to a Gregorian date.
fn civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The Gregorian date `days` after 1970-01-01.
fn from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in(year: i64, month: i64) -> i64 {
    match month {
        2 if leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The serial of a real date. In the 1900 system a date before March 1900 is one less, and
/// 1899-12-31 is 0.
fn serial(year: i64, month: i64, day: i64, date1904: bool) -> i64 {
    if date1904 {
        return civil(year, month, day) - civil(1904, 1, 1);
    }
    let s = civil(year, month, day) - civil(1899, 12, 30);
    if s < 61 { s - 1 } else { s }
}

/// The year, month and day Excel reads in a whole serial from 0 to the last date.
fn ymd(serial: i64, date1904: bool) -> (i64, i64, i64) {
    if date1904 {
        return from_days(civil(1904, 1, 1) + serial);
    }
    match serial {
        0 => (1900, 1, 0),
        60 => (1900, 2, 29),
        1..=59 => from_days(civil(1899, 12, 31) + serial),
        _ => from_days(civil(1899, 12, 30) + serial),
    }
}

/// A number argument. A boolean is listed as `name`.
fn arg(ctx: &mut Ctx<'_>, name: &str, expr: &Expr) -> Result<Result<f64, CellError>, Miss> {
    match ctx.scalar(expr)? {
        Value::Bool(_) => Err(listed(name)),
        v => Ok(v.number()),
    }
}

/// A date argument as a whole serial, `#NUM!` below 0 or past the last date.
fn date_arg(ctx: &mut Ctx<'_>, name: &str, expr: &Expr) -> Result<Result<i64, CellError>, Miss> {
    let n = match arg(ctx, name, expr)? {
        Ok(n) => n.floor(),
        Err(e) => return Ok(Err(e)),
    };
    Ok(if (0.0..=last(ctx) as f64).contains(&n) {
        Ok(n as i64)
    } else {
        Err(CellError::Num)
    })
}

/// `DATE`: a year from 0 to 1899 adds 1900, and a month or day past its range carries into the
/// next, or back into the one before when below 1.
pub(super) fn date(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("DATE", args, 3, 3)?;
    let year = take!(arg(ctx, "DATE", &args[0])?).trunc();
    let month = take!(arg(ctx, "DATE", &args[1])?).trunc();
    let d = take!(arg(ctx, "DATE", &args[2])?).trunc();
    let date1904 = ctx.date1904();
    if !(0.0..10_000.0).contains(&year) {
        return error(CellError::Num);
    }
    if month.abs() > 1e9 || d.abs() > 1e9 || (date1904 && year < 1900.0) {
        return Err(listed("DATE"));
    }
    let year = (if year < 1900.0 { year + 1900.0 } else { year }) as i64;
    let months = year * 12 + month as i64 - 1;
    let (y, m) = (months.div_euclid(12), months.rem_euclid(12) + 1);
    let s = serial(y, m, 1, date1904) + d as i64 - 1;
    if !(0..=last(ctx)).contains(&s) {
        return error(CellError::Num);
    }
    let first = if date1904 { 1904 } else { 1900 };
    if !(first..=9999).contains(&y) {
        return Err(listed("DATE"));
    }
    number(s as f64)
}

/// `TIME`: the hours, minutes and seconds as a fraction of a day, past 24 hours wrapped. An
/// argument past 32767 either way is listed.
pub(super) fn time(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("TIME", args, 3, 3)?;
    let h = take!(arg(ctx, "TIME", &args[0])?).trunc();
    let m = take!(arg(ctx, "TIME", &args[1])?).trunc();
    let s = take!(arg(ctx, "TIME", &args[2])?).trunc();
    if [h, m, s].iter().any(|v| v.abs() > 32_767.0) {
        return Err(listed("TIME"));
    }
    let total = h * 3600.0 + m * 60.0 + s;
    if total < 0.0 {
        return error(CellError::Num);
    }
    number(total.rem_euclid(86_400.0) / 86_400.0)
}

fn part(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
    pick: fn((i64, i64, i64)) -> i64,
) -> Result<Operand, Miss> {
    arity(name, args, 1, 1)?;
    let serial = take!(date_arg(ctx, name, &args[0])?);
    number(pick(ymd(serial, ctx.date1904())) as f64)
}

pub(super) fn year(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    part(ctx, "YEAR", args, |(y, _, _)| y)
}

pub(super) fn month(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    part(ctx, "MONTH", args, |(_, m, _)| m)
}

pub(super) fn day(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    part(ctx, "DAY", args, |(_, _, d)| d)
}

/// `HOUR`, `MINUTE` and `SECOND`: the second of the day, rounded, given to `pick`. A serial past
/// the last date is listed.
fn clock(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
    pick: fn(i64) -> i64,
) -> Result<Operand, Miss> {
    arity(name, args, 1, 1)?;
    let n = take!(arg(ctx, name, &args[0])?);
    if n < 0.0 {
        return error(CellError::Num);
    }
    if n.floor() > last(ctx) as f64 {
        return Err(listed(name));
    }
    let second = ((n - n.floor()) * 86_400.0).round() as i64 % 86_400;
    number(pick(second) as f64)
}

pub(super) fn hour(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    clock(ctx, "HOUR", args, |s| s / 3600)
}

pub(super) fn minute(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    clock(ctx, "MINUTE", args, |s| s / 60 % 60)
}

pub(super) fn second(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    clock(ctx, "SECOND", args, |s| s % 60)
}

/// `DAYS`: the end date less the start date, each without its time.
pub(super) fn days(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("DAYS", args, 2, 2)?;
    let end = take!(date_arg(ctx, "DAYS", &args[0])?);
    let start = take!(date_arg(ctx, "DAYS", &args[1])?);
    number((end - start) as f64)
}

/// `WEEKDAY`: return type 1 counts Sunday as 1, 2 Monday as 1, 3 Monday as 0, and 11 to 17
/// Monday to Sunday as 1. Any other type is `#NUM!`, and an empty one is listed. Serial 1 is a
/// Sunday, as Excel counts it.
pub(super) fn weekday(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("WEEKDAY", args, 1, 2)?;
    let serial = take!(date_arg(ctx, "WEEKDAY", &args[0])?);
    let kind = match args.get(1) {
        None => 1.0,
        Some(Expr::Missing) => return Err(listed("WEEKDAY")),
        Some(arg_) => take!(arg(ctx, "WEEKDAY", arg_)?).trunc(),
    };
    // The 1904 system's serial 0 is the 1900 system's 1462.
    let serial = if ctx.date1904() {
        serial + 1462
    } else {
        serial
    };
    let monday = (serial + 5).rem_euclid(7);
    let result = match kind as i64 {
        1 => (monday + 1) % 7 + 1,
        2 => monday + 1,
        3 => monday,
        k @ 11..=17 => (monday - (k - 11)).rem_euclid(7) + 1,
        _ => return error(CellError::Num),
    };
    number(result as f64)
}

pub(super) fn edate(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    shift(ctx, "EDATE", args, false)
}

pub(super) fn eomonth(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    shift(ctx, "EOMONTH", args, true)
}

/// `EDATE` and, with `end`, `EOMONTH`: the date a whole number of months away, its day kept or
/// cut to the month's last, or the last day of that month.
fn shift(ctx: &mut Ctx<'_>, name: &str, args: &[Expr], end: bool) -> Result<Operand, Miss> {
    arity(name, args, 2, 2)?;
    let start = take!(date_arg(ctx, name, &args[0])?);
    let months = take!(arg(ctx, name, &args[1])?).trunc();
    let date1904 = ctx.date1904();
    if !date1904 && start < 61 {
        return Err(listed(name));
    }
    if months.abs() > 1e6 {
        return error(CellError::Num);
    }
    let (y, m, d) = ymd(start, date1904);
    let total = y * 12 + m - 1 + months as i64;
    let (y, m) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    let first = if date1904 { 1904 } else { 1900 };
    if !(first..=9999).contains(&y) {
        return error(CellError::Num);
    }
    if !date1904 && y == 1900 && m <= 2 {
        return Err(listed(name));
    }
    let d = if end {
        days_in(y, m)
    } else {
        d.min(days_in(y, m))
    };
    number(serial(y, m, d, date1904) as f64)
}

/// `DATEDIF`: the whole years (`Y`), months (`M`) or days (`D`) from start to end, the months
/// past the whole years (`YM`), the days past the whole years (`YD`), or the days past the whole
/// months (`MD`). A start after the end, and any other unit, is `#NUM!`.
pub(super) fn datedif(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("DATEDIF", args, 3, 3)?;
    let start = take!(date_arg(ctx, "DATEDIF", &args[0])?);
    let end = take!(date_arg(ctx, "DATEDIF", &args[1])?);
    let unit = take!(ctx.scalar(&args[2])?.text()).to_ascii_uppercase();
    if start > end {
        return error(CellError::Num);
    }
    let date1904 = ctx.date1904();
    if !date1904 && start < 61 {
        return Err(listed("DATEDIF"));
    }
    let (y1, m1, d1) = ymd(start, date1904);
    let (y2, m2, d2) = ymd(end, date1904);
    let mut months = (y2 - y1) * 12 + m2 - m1;
    if d2 < d1 {
        months -= 1;
    }
    let result = match unit.as_str() {
        "D" => end - start,
        "M" => months,
        "Y" => months / 12,
        "YM" => months % 12,
        "YD" => {
            if [y1, y2 - 1, y2].into_iter().any(leap) {
                return Err(listed("DATEDIF"));
            }
            let year = if (m2, d2) >= (m1, d1) { y2 } else { y2 - 1 };
            end - serial(year, m1, d1, date1904)
        }
        "MD" if d2 >= d1 => d2 - d1,
        "MD" => {
            let (py, pm) = if m2 == 1 { (y2 - 1, 12) } else { (y2, m2 - 1) };
            let before = days_in(py, pm);
            if d1 > before || d1 > days_in(y2, m2) {
                return Err(listed("DATEDIF"));
            }
            before - d1 + d2
        }
        _ => return error(CellError::Num),
    };
    number(result as f64)
}
