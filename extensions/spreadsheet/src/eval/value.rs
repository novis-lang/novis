//! A value the evaluator computes, and Excel's coercion between them
//! (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! - **To a number**, for arithmetic: a boolean is 1 or 0, a blank is 0, and a text is its
//!   number when it is one: a decimal or scientific number, a percentage (`50%`), an ISO date
//!   (`2024-01-15`), a clock time (`13:30`, `13:30:15`) or both. Other text is `#VALUE!`. A
//!   date in a locale's own order (`1/15/2024`) is other text here, where Excel reads it by the
//!   machine's locale.
//! - **To text**, for `&`: a number is written with up to 15 significant digits, in the
//!   scientific form past `1E+15` and below `1E-9`. A boolean is `TRUE` or `FALSE`, and a blank
//!   is `""`.
//! - **To a boolean**, for a condition: a number is `TRUE` unless it is 0, a text is `TRUE` or
//!   `FALSE` in any case and `#VALUE!` otherwise, and a blank is `FALSE`.
//! - **Comparison**: every number is below every text, and every text below every boolean. A
//!   blank compares as the other side's zero (0, `""` or `FALSE`). Text compares without case.
//!
//! An error is a value too, and each coercion returns the first error it meets, left first.

use std::cmp::Ordering;

use crate::read::CellError;

/// One computed value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Blank,
    Number(f64),
    Text(String),
    Bool(bool),
    Error(CellError),
}

impl Value {
    /// A number result: an infinite or NaN number is `#NUM!`.
    pub fn number_or_error(v: f64) -> Self {
        if v.is_finite() {
            Self::Number(v)
        } else {
            Self::Error(CellError::Num)
        }
    }

    /// The value as a number, for arithmetic.
    pub fn number(&self) -> Result<f64, CellError> {
        match self {
            Self::Blank => Ok(0.0),
            Self::Number(n) => Ok(*n),
            Self::Bool(b) => Ok(f64::from(u8::from(*b))),
            Self::Text(t) => number_of_text(t).ok_or(CellError::Value),
            Self::Error(e) => Err(*e),
        }
    }

    /// The value as text, for `&`.
    pub fn text(&self) -> Result<String, CellError> {
        match self {
            Self::Blank => Ok(String::new()),
            Self::Number(n) => Ok(number_text(*n)),
            Self::Bool(b) => Ok(if *b { "TRUE" } else { "FALSE" }.to_owned()),
            Self::Text(t) => Ok(t.clone()),
            Self::Error(e) => Err(*e),
        }
    }

    /// The value as a condition.
    pub fn boolean(&self) -> Result<bool, CellError> {
        match self {
            Self::Blank => Ok(false),
            Self::Number(n) => Ok(*n != 0.0),
            Self::Bool(b) => Ok(*b),
            Self::Text(t) if t.eq_ignore_ascii_case("TRUE") => Ok(true),
            Self::Text(t) if t.eq_ignore_ascii_case("FALSE") => Ok(false),
            Self::Text(_) => Err(CellError::Value),
            Self::Error(e) => Err(*e),
        }
    }

    /// How `self` compares with `other` by Excel's order.
    pub fn compare(&self, other: &Self) -> Result<Ordering, CellError> {
        if let Self::Error(e) = self {
            return Err(*e);
        }
        if let Self::Error(e) = other {
            return Err(*e);
        }
        let zero = |v: &Self| match v {
            Self::Text(_) => Self::Text(String::new()),
            Self::Bool(_) => Self::Bool(false),
            _ => Self::Number(0.0),
        };
        let (a, b) = match (self, other) {
            (Self::Blank, Self::Blank) => return Ok(Ordering::Equal),
            (Self::Blank, b) => (zero(b), b.clone()),
            (a, Self::Blank) => (a.clone(), zero(a)),
            (a, b) => (a.clone(), b.clone()),
        };
        let rank = |v: &Self| match v {
            Self::Text(_) => 1,
            Self::Bool(_) => 2,
            _ => 0,
        };
        Ok(match (&a, &b) {
            (Self::Number(x), Self::Number(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
            (Self::Text(x), Self::Text(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
            (Self::Bool(x), Self::Bool(y)) => x.cmp(y),
            _ => rank(&a).cmp(&rank(&b)),
        })
    }
}

/// `v` as `&` writes it: up to 15 significant digits.
pub fn number_text(v: f64) -> String {
    if v == 0.0 {
        return "0".to_owned();
    }
    let scientific = format!("{:.14e}", v.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let mut digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }
    let mut out = String::new();
    if v < 0.0 {
        out.push('-');
    }
    if (-9..=14).contains(&exponent) {
        if exponent >= 0 {
            let whole = exponent as usize + 1;
            if digits.len() <= whole {
                out.push_str(&digits);
                out.extend(std::iter::repeat_n('0', whole - digits.len()));
            } else {
                out.push_str(&digits[..whole]);
                out.push('.');
                out.push_str(&digits[whole..]);
            }
        } else {
            out.push_str("0.");
            out.extend(std::iter::repeat_n('0', (-exponent - 1) as usize));
            out.push_str(&digits);
        }
    } else {
        out.push_str(&digits[..1]);
        if digits.len() > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('E');
        out.push(if exponent < 0 { '-' } else { '+' });
        out.push_str(&format!("{:02}", exponent.abs()));
    }
    out
}

/// A text's number, by the rules in the module doc.
pub fn number_of_text(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(rest) = t.strip_suffix('%') {
        return plain(rest.trim_end()).map(|n| n / 100.0);
    }
    if let Some(n) = plain(t) {
        return Some(n);
    }
    let (day, clock) = match t.split_once(' ') {
        Some((day, clock)) => (Some(day), Some(clock.trim_start())),
        None if t.contains(':') => (None, Some(t)),
        None => (Some(t), None),
    };
    let mut serial = 0.0;
    if let Some(day) = day {
        serial += iso_day(day)?;
    }
    if let Some(clock) = clock {
        serial += clock_time(clock)?;
    }
    Some(serial)
}

/// A decimal or scientific number with an optional sign, nothing else.
fn plain(t: &str) -> Option<f64> {
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let first = body.bytes().next()?;
    if !(first.is_ascii_digit() || first == b'.') {
        return None;
    }
    if !body
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-'))
    {
        return None;
    }
    t.parse().ok().filter(|n: &f64| n.is_finite())
}

/// `yyyy-mm-dd` as a serial of the 1900 system.
fn iso_day(t: &str) -> Option<f64> {
    let mut parts = t.split('-');
    let year: i64 = digits(parts.next()?, 4)?;
    let month: i64 = digits(parts.next()?, 2)?;
    let day: i64 = digits(parts.next()?, 2)?;
    if parts.next().is_some() || !(1900..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day < 1 || day > days[(month - 1) as usize] {
        return None;
    }
    let serial = civil(year, month, day) - civil(1899, 12, 30);
    // Excel counts 1900-02-29, so a day before March 1900 is one less.
    let serial = if serial < 61 { serial - 1 } else { serial };
    Some(serial as f64)
}

/// `hh:mm` or `hh:mm:ss` as a fraction of a day.
fn clock_time(t: &str) -> Option<f64> {
    let mut parts = t.split(':');
    let hours: i64 = digits(parts.next()?, 2)?;
    let minutes: i64 = digits(parts.next()?, 2)?;
    let seconds: i64 = match parts.next() {
        Some(s) => digits(s, 2)?,
        None => 0,
    };
    if parts.next().is_some() || hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }
    Some((hours * 3600 + minutes * 60 + seconds) as f64 / 86_400.0)
}

fn digits(t: &str, most: usize) -> Option<i64> {
    (!t.is_empty() && t.len() <= most && t.bytes().all(|b| b.is_ascii_digit()))
        .then(|| t.parse().ok())
        .flatten()
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
