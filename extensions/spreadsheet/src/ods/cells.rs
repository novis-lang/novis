//! One ods cell, `table:table-cell`, by the write table (`rule:core-classes/spreadsheet-value-types`).
//!
//! Each value is the `office:value-type` that holds it:
//!
//! | Value | ods |
//! |---|---|
//! | text | `string`, one `text:p` for each line |
//! | rich text | `string`, one `text:p` with a `text:span` for each styled run |
//! | number, a whole number or a decimal of up to 15 digits | `float`, with `office:value` |
//! | a longer whole number or decimal | `string`, so no digit is lost |
//! | boolean | `boolean` |
//! | date, clock time | `date`, with `office:date-value` |
//! | time of day, duration | `time`, with `office:time-value` as `PT10H30M00S` |
//! | formula | `table:formula` in OpenFormula (`openformula`), with its cached result |
//!
//! The values are checked by `crate::cells::check` when they are added, so a value that xlsx
//! does not allow is not allowed here either. A cell's `text:p` is the value's plain text, such
//! as `2024-01-15` for a date. An application shows the value under the cell's number style.
//! Spaces past the first, a space at the start and a tab are `text:s` and `text:tab`, because
//! ODF collapses whitespace.

use std::fmt::Write as _;

use crate::{Clock, Date, TimeOfDay, Value, cells, openformula, xml};

/// Nanoseconds in a second.
const SECOND: i64 = 1_000_000_000;

/// Appends `text` as the content of a `text:p` or a `text:span`, with its whitespace kept.
pub fn spaced(out: &mut String, text: &str) {
    let mut spaces = 0usize;
    let mut start = true;
    let flush = |out: &mut String, spaces: &mut usize, start: bool| {
        if *spaces == 0 {
            return;
        }
        // The first space of a run inside the text is kept as it is. Every other is `text:s`.
        let mut rest = *spaces;
        if !start {
            out.push(' ');
            rest -= 1;
        }
        match rest {
            0 => {}
            1 => out.push_str("<text:s/>"),
            n => {
                let _ = write!(out, "<text:s text:c=\"{n}\"/>");
            }
        }
        *spaces = 0;
    };
    let mut plain = String::new();
    for c in text.chars() {
        match c {
            ' ' => {
                xml::odf(out, &plain, false);
                plain.clear();
                spaces += 1;
            }
            '\t' => {
                xml::odf(out, &plain, false);
                plain.clear();
                flush(out, &mut spaces, start);
                out.push_str("<text:tab/>");
                start = false;
            }
            c => {
                if spaces > 0 {
                    flush(out, &mut spaces, start);
                }
                plain.push(c);
                start = false;
            }
        }
    }
    xml::odf(out, &plain, false);
    // Spaces at the end are kept the way spaces at the start are.
    flush(out, &mut spaces, true);
}

/// Appends `text` as one `text:p` for each of its lines.
pub fn paragraphs(out: &mut String, text: &str) {
    for line in text.split('\n') {
        out.push_str("<text:p>");
        spaced(out, line.strip_suffix('\r').unwrap_or(line));
        out.push_str("</text:p>");
    }
}

fn date_text(date: &Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

/// `hh:mm:ss` with the fraction of a second the nanoseconds give, if any.
fn clock_text(hour: i64, minute: i64, second: i64, nanos: i64, separator: [&str; 3]) -> String {
    let mut out = format!(
        "{hour:02}{}{minute:02}{}{second:02}",
        separator[0], separator[1]
    );
    if nanos > 0 {
        let fraction = format!("{nanos:09}");
        let _ = write!(out, ".{}", fraction.trim_end_matches('0'));
    }
    out.push_str(separator[2]);
    out
}

fn time_text(time: &TimeOfDay) -> String {
    clock_text(
        time.hour,
        time.minute,
        time.second,
        time.nanos,
        [":", ":", ""],
    )
}

fn moment_text(clock: &Clock) -> String {
    format!("{}T{}", date_text(&clock.day), time_text(&clock.clock))
}

/// A duration of `nanos` as `office:time-value`, such as `PT26H00M00S` or `-PT1H30M00S`.
fn duration_value(nanos: i64) -> String {
    let sign = if nanos < 0 { "-" } else { "" };
    let total = nanos.unsigned_abs();
    let seconds = (total / SECOND as u64) as i64;
    let rest = (total % SECOND as u64) as i64;
    format!(
        "{sign}PT{}",
        clock_text(
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60,
            rest,
            ["H", "M", "S"]
        )
    )
}

/// The plain text of a duration of `nanos`, such as `26:00:00`.
fn duration_text(nanos: i64) -> String {
    let sign = if nanos < 0 { "-" } else { "" };
    let total = nanos.unsigned_abs();
    let seconds = (total / SECOND as u64) as i64;
    let rest = (total % SECOND as u64) as i64;
    format!(
        "{sign}{}",
        clock_text(
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60,
            rest,
            [":", ":", ""]
        )
    )
}

/// Whether a whole number's or a decimal's digits fit an `f64` exactly: at most 15 significant.
fn fits(digits: &str) -> bool {
    let digits: String = digits.chars().filter(char::is_ascii_digit).collect();
    digits.trim_start_matches('0').trim_end_matches('0').len() <= 15
}

/// Appends the attributes and content of a cell holding `value`, after `<table:table-cell` and
/// its style. `run` gives the text style name of a rich run's layer.
pub fn value(out: &mut String, value: &Value, run: &mut dyn FnMut(u64) -> String) {
    let float = |out: &mut String, shown: &str| {
        let _ = write!(
            out,
            " office:value-type=\"float\" office:value=\"{shown}\"><text:p>{shown}</text:p>"
        );
    };
    match value {
        Value::Text(text) => {
            out.push_str(" office:value-type=\"string\">");
            paragraphs(out, text);
        }
        Value::Number(number) => {
            let mut shown = String::new();
            cells::number(&mut shown, *number);
            float(out, &shown);
        }
        Value::Integer(integer) => {
            let shown = itoa::Buffer::new().format(*integer).to_owned();
            if fits(&shown) {
                float(out, &shown);
            } else {
                let _ = write!(
                    out,
                    " office:value-type=\"string\"><text:p>{shown}</text:p>"
                );
            }
        }
        Value::Decimal(text) => {
            if fits(text) {
                float(out, text);
            } else {
                let _ = write!(out, " office:value-type=\"string\"><text:p>{text}</text:p>");
            }
        }
        Value::Boolean(flag) => {
            let (attr, shown) = if *flag {
                ("true", "TRUE")
            } else {
                ("false", "FALSE")
            };
            let _ = write!(
                out,
                " office:value-type=\"boolean\" office:boolean-value=\"{attr}\"><text:p>{shown}</text:p>"
            );
        }
        Value::Date(date) => {
            let text = date_text(date);
            let _ = write!(
                out,
                " office:value-type=\"date\" office:date-value=\"{text}\"><text:p>{text}</text:p>"
            );
        }
        Value::Clock(clock) => {
            let text = moment_text(clock);
            let _ = write!(
                out,
                " office:value-type=\"date\" office:date-value=\"{text}\"><text:p>{}</text:p>",
                text.replace('T', " ")
            );
        }
        Value::Time(time) => {
            let text = time_text(time);
            let _ = write!(
                out,
                " office:value-type=\"time\" office:time-value=\"PT{}\"><text:p>{text}</text:p>",
                clock_text(
                    time.hour,
                    time.minute,
                    time.second,
                    time.nanos,
                    ["H", "M", "S"]
                )
            );
        }
        Value::Duration(nanos) => {
            let _ = write!(
                out,
                " office:value-type=\"time\" office:time-value=\"{}\"><text:p>{}</text:p>",
                duration_value(*nanos),
                duration_text(*nanos)
            );
        }
        Value::Formula {
            formula,
            cached_text,
            cached_number,
        } => {
            out.push_str(" table:formula=\"");
            xml::odf(out, &openformula::from_a1(formula), true);
            out.push('"');
            if let Some(text) = cached_text {
                out.push_str(" office:value-type=\"string\" office:string-value=\"");
                xml::odf(out, text, true);
                out.push_str("\">");
                paragraphs(out, text);
            } else if let Some(number) = cached_number {
                let mut shown = String::new();
                cells::number(&mut shown, *number);
                float(out, &shown);
            } else {
                out.push('>');
            }
        }
        Value::Rich(runs) => {
            out.push_str(" office:value-type=\"string\"><text:p>");
            for piece in runs {
                let mut lines = piece.text.split('\n');
                let mut first = true;
                let style = piece.style.map(&mut *run);
                for line in lines.by_ref() {
                    if !first {
                        out.push_str("<text:line-break/>");
                    }
                    first = false;
                    if line.is_empty() {
                        continue;
                    }
                    match &style {
                        Some(name) => {
                            let _ = write!(out, "<text:span text:style-name=\"{name}\">");
                            spaced(out, line);
                            out.push_str("</text:span>");
                        }
                        None => spaced(out, line),
                    }
                }
            }
            out.push_str("</text:p>");
        }
    }
}
