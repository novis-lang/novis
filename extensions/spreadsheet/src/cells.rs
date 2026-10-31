//! One cell's XML, by `rule:core-classes/spreadsheet-value-types`' write table.
//!
//! [`cell`] writes a cell at its reference: text and rich text inline (`t="inlineStr"`), so no
//! shared-string table grows; a number as the shortest text that reads back as the same `f64`;
//! a whole number or a decimal with more than 15 significant digits as text, so no digit is lost;
//! a boolean as `t="b"`; a date, a time of day, a duration and a clock time as a serial of the
//! 1900 date system, which [`Implicit`] gives its format; and a formula as `<f>` with its cached
//! result. A cell with no value is written only when it has a style of its own.
//!
//! **The 1900 date system** counts days from 1899-12-30, and Excel counts 1900-02-29, a day that
//! does not exist, so a date before 1900-03-01 is one day less. A date before 1900-01-01 or after
//! 9999-12-31 has no serial and is a [`Error::Invalid`] naming the cell, and so is a number that
//! is NaN or infinite, and a text past Excel's 32,767 characters.

use std::fmt::Write as _;

use crate::styles::{Implicit, Styles};
use crate::{Cell, Clock, Date, Error, TimeOfDay, Value, xml};

/// Nanoseconds in a day.
const DAY_NANOS: f64 = 86_400_000_000_000.0;

/// The most characters Excel keeps in one cell.
const MAX_TEXT: usize = 32_767;

/// Appends the letters of the column at `index`, counted from 0.
pub fn column_letters(out: &mut String, index: u32) {
    let mut letters = [0u8; 4];
    let mut len = 0;
    let mut n = index + 1;
    while n > 0 {
        n -= 1;
        letters[len] = b'A' + (n % 26) as u8;
        len += 1;
        n /= 26;
    }
    for &letter in letters[..len].iter().rev() {
        out.push(char::from(letter));
    }
}

/// The reference of the cell at `column`, counted from 0, in `row`, counted from 1.
pub fn reference(column: u32, row: u32) -> String {
    let mut out = String::new();
    column_letters(&mut out, column);
    out.push_str(itoa::Buffer::new().format(row));
    out
}

/// Appends `value`, which is finite, as the shortest text that reads back as it.
pub fn number(out: &mut String, value: f64) {
    if value == 0.0 {
        out.push('0');
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        out.push_str(itoa::Buffer::new().format(value as i64));
    } else {
        out.push_str(ryu::Buffer::new().format_finite(value));
    }
}

/// The significant digits of a whole number or a decimal's digits, leading and trailing zeros
/// not counted.
fn significant(digits: &str) -> usize {
    let digits: String = digits.chars().filter(char::is_ascii_digit).collect();
    digits.trim_start_matches('0').trim_end_matches('0').len()
}

/// Whether `text` is a decimal: an optional `-`, digits, and an optional `.` with digits.
fn is_decimal(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (unsigned, None),
    };
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    digits(whole) && fraction.is_none_or(digits)
}

/// Days from 1970-01-01 to a valid civil date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Whether `date` is a real calendar date.
pub fn valid_date(date: &Date) -> bool {
    (1..=12).contains(&date.month)
        && date.day >= 1
        && date.day <= days_in_month(date.year, date.month)
}

/// Whether `time` is a real time of day.
pub fn valid_time(time: &TimeOfDay) -> bool {
    (0..24).contains(&time.hour)
        && (0..60).contains(&time.minute)
        && (0..60).contains(&time.second)
        && (0..1_000_000_000).contains(&time.nanos)
}

/// The 1900-system serial of `date`, or `None` outside 1900-01-01 to 9999-12-31.
pub fn date_serial(date: &Date) -> Option<i64> {
    if !valid_date(date) || !(1900..=9999).contains(&date.year) {
        return None;
    }
    let serial = days_from_civil(date.year, date.month, date.day) - days_from_civil(1899, 12, 30);
    Some(if serial < 61 { serial - 1 } else { serial })
}

/// The fraction of a day `time` is.
pub fn day_fraction(time: &TimeOfDay) -> f64 {
    let nanos =
        ((time.hour * 60 + time.minute) * 60 + time.second) as f64 * 1e9 + time.nanos as f64;
    nanos / DAY_NANOS
}

fn date_error(at: &str, date: &Date) -> Error {
    if valid_date(date) {
        Error::Invalid(format!(
            "The cell `{at}` has the date {:04}-{:02}-{:02}. Excel shows only dates from 1900-01-01 to 9999-12-31.",
            date.year, date.month, date.day
        ))
    } else {
        Error::Invalid(format!(
            "The cell `{at}` has the date {}-{}-{}, which is not a real date.",
            date.year, date.month, date.day
        ))
    }
}

fn time_error(at: &str, time: &TimeOfDay) -> Error {
    Error::Invalid(format!(
        "The cell `{at}` has the time {}:{}:{}, which is not a real time of day.",
        time.hour, time.minute, time.second
    ))
}

fn finite(at: &str, value: f64) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::Invalid(format!(
            "The cell `{at}` has the number {value}. A cell can hold only a finite number."
        )))
    }
}

/// `text`, when its `count` characters fit in one cell.
fn length<'a>(at: &str, text: &'a str, count: usize) -> Result<&'a str, Error> {
    if count > MAX_TEXT {
        return Err(Error::Invalid(format!(
            "The cell `{at}` has {count} characters. A cell has at most 32,767 characters."
        )));
    }
    Ok(text)
}

/// The most characters Excel allows in a formula, without its leading `=`.
pub const MAX_FORMULA: usize = 8_192;

/// Checks a formula the writer writes, in a cell, a total, a conditional format or a validation:
/// from 1 to 8,192 characters once a leading `=` is dropped. `place` names where it is, such as
/// ``the cell `B2` ``, and is built only for the error.
pub fn check_formula(text: &str, place: impl FnOnce() -> String) -> Result<(), Error> {
    let text = text.strip_prefix('=').unwrap_or(text);
    if text.is_empty() {
        return Err(Error::Invalid(format!(
            "The formula in {} is empty.",
            place()
        )));
    }
    if text.len() > MAX_FORMULA {
        let count = text.chars().count();
        if count > MAX_FORMULA {
            return Err(Error::Invalid(format!(
                "The formula in {} has {count} characters. A formula has at most 8,192.",
                place()
            )));
        }
    }
    Ok(())
}

/// The number a cell holding `value` stores, and the format the write table gives it, or `None`
/// for a value stored as text, a boolean, a formula or nothing. A value [`cell`] refuses is `None`.
pub fn stored_number(value: &Value) -> Option<(f64, Implicit)> {
    match value {
        Value::Number(value) => Some((*value, Implicit::None)).filter(|(v, _)| v.is_finite()),
        Value::Integer(value) => {
            let mut integer = itoa::Buffer::new();
            (significant(integer.format(*value)) <= 15).then_some((*value as f64, Implicit::None))
        }
        Value::Decimal(text) if is_decimal(text) && significant(text) <= 15 => {
            Some((text.parse().ok()?, Implicit::None))
        }
        Value::Date(date) => Some((date_serial(date)? as f64, Implicit::Date)),
        Value::Time(time) => valid_time(time).then(|| (day_fraction(time), Implicit::Time)),
        Value::Duration(nanos) => Some((*nanos as f64 / DAY_NANOS, Implicit::Duration)),
        Value::Clock(clock) => Some((clock_serial("", clock).ok()?, Implicit::Clock)),
        _ => None,
    }
}

fn clock_serial(at: &str, clock: &Clock) -> Result<f64, Error> {
    let day = date_serial(&clock.day).ok_or_else(|| date_error(at, &clock.day))?;
    if !valid_time(&clock.clock) {
        return Err(time_error(at, &clock.clock));
    }
    Ok(day as f64 + day_fraction(&clock.clock))
}

/// What a value writes as: its `t` attribute, the format it implies, and its content.
enum Written<'a> {
    Number(f64, Implicit),
    Raw(&'a str),
    Text(&'a str),
    Boolean(bool),
    Formula(&'a str, Option<&'a str>, Option<f64>),
    Rich(&'a [crate::RichRun]),
}

/// The layers a cell's format combines: its column's and its row's, beside its own.
pub struct Layers {
    pub column: Option<u64>,
    pub row: Option<u64>,
}

/// Checks `value`, the value of the cell at `at`, by the write table, and returns what it writes
/// as.
fn written<'a>(
    at: &str,
    value: &'a Value,
    integer: &'a mut itoa::Buffer,
) -> Result<Written<'a>, Error> {
    Ok(match value {
        Value::Text(text) => Written::Text(length(at, text, text.chars().count())?),
        Value::Number(value) => Written::Number(finite(at, *value)?, Implicit::None),
        Value::Integer(value) => {
            let text = integer.format(*value);
            if significant(text) > 15 {
                Written::Text(text)
            } else {
                Written::Raw(text)
            }
        }
        Value::Decimal(text) => {
            if !is_decimal(text) {
                return Err(Error::Invalid(format!(
                    "The cell `{at}` has the decimal `{text}`, which is not a number."
                )));
            }
            if significant(text) > 15 {
                Written::Text(text)
            } else {
                Written::Raw(text)
            }
        }
        Value::Boolean(value) => Written::Boolean(*value),
        Value::Date(date) => {
            let serial = date_serial(date).ok_or_else(|| date_error(at, date))?;
            Written::Number(serial as f64, Implicit::Date)
        }
        Value::Time(time) => {
            if !valid_time(time) {
                return Err(time_error(at, time));
            }
            Written::Number(day_fraction(time), Implicit::Time)
        }
        Value::Duration(nanos) => Written::Number(*nanos as f64 / DAY_NANOS, Implicit::Duration),
        Value::Clock(clock) => Written::Number(clock_serial(at, clock)?, Implicit::Clock),
        Value::Formula {
            formula,
            cached_text,
            cached_number,
        } => {
            check_formula(formula, || format!("the cell `{at}`"))?;
            let number = cached_number.map(|n| finite(at, n)).transpose()?;
            Written::Formula(formula, cached_text.as_deref(), number)
        }
        Value::Rich(runs) => {
            let count = runs.iter().map(|run| run.text.chars().count()).sum();
            length(at, "", count)?;
            Written::Rich(runs)
        }
    })
}

/// Checks `value`, the value of the cell at column `column` of row `row`, by the write table,
/// and returns the format the table gives it. An ods cell is checked here, so a value that
/// xlsx does not allow is not allowed in ods either.
pub fn check(column: u32, row: u32, value: &Value) -> Result<Implicit, Error> {
    let mut integer = itoa::Buffer::new();
    Ok(
        match written(&reference(column, row), value, &mut integer)? {
            Written::Number(_, implicit) => implicit,
            _ => Implicit::None,
        },
    )
}

/// Appends the cell `cell` at column `column` of row `row`, and returns whether it holds a
/// formula.
pub fn cell(
    out: &mut String,
    styles: &mut Styles,
    layers: &Layers,
    column: u32,
    row: u32,
    cell: &Cell,
) -> Result<bool, Error> {
    let at = reference(column, row);
    let mut integer = itoa::Buffer::new();
    let Some(value) = &cell.value else {
        if cell.style.is_none() {
            return Ok(false);
        }
        let xf = styles.xf(layers.column, layers.row, cell.style, Implicit::None)?;
        let _ = write!(out, "<c r=\"{at}\"");
        if xf != 0 {
            let _ = write!(out, " s=\"{xf}\"");
        }
        out.push_str("/>");
        return Ok(false);
    };
    let written = written(&at, value, &mut integer)?;
    let implicit = match written {
        Written::Number(_, implicit) => implicit,
        _ => Implicit::None,
    };
    let xf = styles.xf(layers.column, layers.row, cell.style, implicit)?;
    let _ = write!(out, "<c r=\"{at}\"");
    if xf != 0 {
        let _ = write!(out, " s=\"{xf}\"");
    }
    let mut formula = false;
    match written {
        Written::Number(value, _) => {
            out.push_str("><v>");
            number(out, value);
            out.push_str("</v></c>");
        }
        Written::Raw(text) => {
            let _ = write!(out, "><v>{text}</v></c>");
        }
        Written::Boolean(value) => {
            let _ = write!(out, " t=\"b\"><v>{}</v></c>", u8::from(value));
        }
        Written::Text(text) => {
            out.push_str(" t=\"inlineStr\"><is>");
            run_text(out, text);
            out.push_str("</is></c>");
        }
        Written::Rich(runs) => {
            out.push_str(" t=\"inlineStr\"><is>");
            for run in runs {
                out.push_str("<r>");
                if let Some(style) = run.style {
                    styles.run_properties(out, style)?;
                }
                run_text(out, &run.text);
                out.push_str("</r>");
            }
            out.push_str("</is></c>");
        }
        Written::Formula(text, cached_text, cached_number) => {
            formula = true;
            if cached_text.is_some() {
                out.push_str(" t=\"str\"");
            }
            out.push_str("><f>");
            xml::text(out, text);
            out.push_str("</f>");
            if let Some(cached) = cached_text {
                out.push_str("<v>");
                xml::text(out, cached);
                out.push_str("</v>");
            } else if let Some(cached) = cached_number {
                out.push_str("<v>");
                number(out, cached);
                out.push_str("</v>");
            }
            out.push_str("</c>");
        }
    }
    Ok(formula)
}

/// Appends `<t>` with `text`, keeping its whitespace at either end.
pub fn run_text(out: &mut String, text: &str) {
    if xml::needs_preserve(text) {
        out.push_str("<t xml:space=\"preserve\">");
    } else {
        out.push_str("<t>");
    }
    xml::text(out, text);
    out.push_str("</t>");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i64, month: i64, day: i64) -> Date {
        Date { year, month, day }
    }

    #[test]
    fn columns_are_lettered_as_excel_letters_them() {
        assert_eq!(reference(0, 1), "A1");
        assert_eq!(reference(25, 2), "Z2");
        assert_eq!(reference(26, 3), "AA3");
        assert_eq!(reference(16_383, 9), "XFD9");
    }

    #[test]
    fn serials_count_the_1900_leap_day() {
        assert_eq!(date_serial(&date(1900, 1, 1)), Some(1));
        assert_eq!(date_serial(&date(1900, 2, 28)), Some(59));
        assert_eq!(date_serial(&date(1900, 3, 1)), Some(61));
        assert_eq!(date_serial(&date(2024, 1, 15)), Some(45_306));
        assert_eq!(date_serial(&date(1899, 12, 31)), None);
        assert_eq!(date_serial(&date(2023, 2, 29)), None);
    }

    #[test]
    fn numbers_are_written_in_their_shortest_form() {
        let text = |value| {
            let mut out = String::new();
            number(&mut out, value);
            out
        };
        assert_eq!(text(3.0), "3");
        assert_eq!(text(-0.0), "0");
        assert_eq!(text(0.1), "0.1");
        assert_eq!(text(1e20), "1e20");
    }

    #[test]
    fn digits_past_fifteen_make_a_text_cell() {
        assert_eq!(significant("1234567890123456"), 16);
        assert_eq!(significant("-1000000000000000000"), 1);
        assert_eq!(significant("0.000123"), 3);
        assert!(is_decimal("-12.50"));
        assert!(!is_decimal("1e5"));
        assert!(!is_decimal("1."));
    }
}
