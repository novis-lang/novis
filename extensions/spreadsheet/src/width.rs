//! Auto width: a column's width from the text of its cells
//! (`rule:core-classes/spreadsheet-styles-and-defaults`).
//!
//! A cell's width is the number of characters its value shows, a wide East Asian character
//! counted as two, and the longest line of a text with line breaks. A column's width is its
//! widest cell plus one, capped at [`MAX_AUTO`]. There are no font metrics.
//!
//! **Not done yet.** A number is measured as the text the writer stores, a date or a time as the
//! length of its default format, so a column's own format code is not applied: Stage 8 of goal
//! `spreadsheet` measures through the number-format renderer.

use crate::{RichRun, Value};

/// The widest a measured column is, in characters.
pub const MAX_AUTO: f64 = 60.0;

/// Whether `c` takes two columns of a fixed-width grid: the wide and full-width East Asian
/// blocks, and the emoji blocks.
fn wide(c: char) -> bool {
    matches!(
        u32::from(c),
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F900..=0x1F9FF
            | 0x20000..=0x3FFFD
    )
}

/// The width of `text`: its longest line, a wide character counted as two.
pub fn text(text: &str) -> usize {
    text.lines()
        .map(|line| line.chars().map(|c| if wide(c) { 2 } else { 1 }).sum())
        .max()
        .unwrap_or(0)
}

fn number(value: f64) -> usize {
    if !value.is_finite() {
        return 0;
    }
    let mut out = String::new();
    crate::cells::number(&mut out, value);
    out.len()
}

fn rich(runs: &[RichRun]) -> usize {
    let whole: String = runs.iter().map(|run| run.text.as_str()).collect();
    text(&whole)
}

/// The width of a cell holding `value`.
pub fn value(value: &Value) -> usize {
    match value {
        Value::Text(value) => text(value),
        Value::Number(value) => number(*value),
        Value::Integer(value) => itoa::Buffer::new().format(*value).len(),
        Value::Decimal(value) => value.len(),
        Value::Boolean(true) => 4,
        Value::Boolean(false) => 5,
        Value::Date(_) => 10,
        Value::Time(_) => 8,
        Value::Duration(nanos) => {
            let hours = nanos.unsigned_abs() / 3_600_000_000_000;
            let sign = usize::from(*nanos < 0);
            itoa::Buffer::new().format(hours).len().max(1) + 6 + sign
        }
        Value::Clock(_) => 16,
        Value::Formula {
            cached_text,
            cached_number,
            ..
        } => match (cached_text, cached_number) {
            (Some(cached), _) => text(cached),
            (None, Some(cached)) => number(*cached),
            (None, None) => 0,
        },
        Value::Rich(runs) => rich(runs),
    }
}

/// The width of a column whose widest cell is `widest` characters wide, or `None` for a column
/// with no text to measure.
pub fn column(widest: usize) -> Option<f64> {
    if widest == 0 {
        return None;
    }
    Some(((widest + 1) as f64).min(MAX_AUTO))
}
