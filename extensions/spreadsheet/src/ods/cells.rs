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
//! ODF collapses whitespace. A note and an image go first inside the cell, and a link wraps the
//! cell's text in one `text:a`, its lines split by `text:line-break` ([`Inside`]).

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

/// Appends `text` with a `text:line-break` between its lines, for one paragraph.
pub fn lines(out: &mut String, text: &str) {
    for (at, line) in text.split('\n').enumerate() {
        if at > 0 {
            out.push_str("<text:line-break/>");
        }
        spaced(out, line.strip_suffix('\r').unwrap_or(line));
    }
}

/// What a cell has besides its value: `lead`, written first inside the cell, such as a note or
/// an image, and `link`, the opening `text:a` tag that wraps the cell's text.
#[derive(Debug, Clone, Copy, Default)]
pub struct Inside<'a> {
    pub lead: &'a str,
    pub link: Option<&'a str>,
}

impl Inside<'_> {
    /// Ends the cell's start tag and writes the lead.
    fn open(&self, out: &mut String) {
        out.push('>');
        out.push_str(self.lead);
    }

    /// Appends one `text:p`, its content wrapped in the link when there is one.
    fn paragraph(&self, out: &mut String, body: impl FnOnce(&mut String)) {
        out.push_str("<text:p>");
        if let Some(link) = self.link {
            out.push_str(link);
        }
        body(out);
        if self.link.is_some() {
            out.push_str("</text:a>");
        }
        out.push_str("</text:p>");
    }

    /// Appends `text`: one `text:p` per line, or one with line breaks inside a link.
    fn text(&self, out: &mut String, text: &str) {
        if self.link.is_some() {
            self.paragraph(out, |out| lines(out, text));
        } else {
            paragraphs(out, text);
        }
    }
}

/// Appends the attributes and content of a cell holding `value`, after `<table:table-cell` and
/// its style. `run` gives the text style name of a rich run's layer, and `inside` what the cell
/// has besides its value.
pub fn value(
    out: &mut String,
    value: &Value,
    run: &mut dyn FnMut(u64) -> String,
    inside: Inside<'_>,
) {
    let float = |out: &mut String, shown: &str| {
        let _ = write!(out, " office:value-type=\"float\" office:value=\"{shown}\"");
        inside.open(out);
        inside.paragraph(out, |out| out.push_str(shown));
    };
    let plain = |out: &mut String, attrs: &str, shown: &str| {
        out.push_str(attrs);
        inside.open(out);
        inside.paragraph(out, |out| out.push_str(shown));
    };
    match value {
        Value::Text(text) => {
            out.push_str(" office:value-type=\"string\"");
            inside.open(out);
            inside.text(out, text);
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
                plain(out, " office:value-type=\"string\"", &shown);
            }
        }
        Value::Decimal(text) => {
            if fits(text) {
                float(out, text);
            } else {
                plain(out, " office:value-type=\"string\"", text);
            }
        }
        Value::Boolean(flag) => {
            let (attr, shown) = if *flag {
                ("true", "TRUE")
            } else {
                ("false", "FALSE")
            };
            let attrs = format!(" office:value-type=\"boolean\" office:boolean-value=\"{attr}\"");
            plain(out, &attrs, shown);
        }
        Value::Date(date) => {
            let text = date_text(date);
            let attrs = format!(" office:value-type=\"date\" office:date-value=\"{text}\"");
            plain(out, &attrs, &text);
        }
        Value::Clock(clock) => {
            let text = moment_text(clock);
            let attrs = format!(" office:value-type=\"date\" office:date-value=\"{text}\"");
            plain(out, &attrs, &text.replace('T', " "));
        }
        Value::Time(time) => {
            let text = time_text(time);
            let attrs = format!(
                " office:value-type=\"time\" office:time-value=\"PT{}\"",
                clock_text(
                    time.hour,
                    time.minute,
                    time.second,
                    time.nanos,
                    ["H", "M", "S"]
                )
            );
            plain(out, &attrs, &text);
        }
        Value::Duration(nanos) => {
            let attrs = format!(
                " office:value-type=\"time\" office:time-value=\"{}\"",
                duration_value(*nanos)
            );
            plain(out, &attrs, &duration_text(*nanos));
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
                out.push('"');
                inside.open(out);
                inside.text(out, text);
            } else if let Some(number) = cached_number {
                let mut shown = String::new();
                cells::number(&mut shown, *number);
                float(out, &shown);
            } else {
                inside.open(out);
                if inside.link.is_some() {
                    inside.paragraph(out, |_| {});
                }
            }
        }
        Value::Rich(runs) => {
            out.push_str(" office:value-type=\"string\"");
            inside.open(out);
            out.push_str("<text:p>");
            if let Some(link) = inside.link {
                out.push_str(link);
            }
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
            if inside.link.is_some() {
                out.push_str("</text:a>");
            }
            out.push_str("</text:p>");
        }
    }
}
