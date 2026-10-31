//! Auto width: a column's width from the text of its cells
//! (`rule:core-classes/spreadsheet-styles-and-defaults`).
//!
//! A cell's width is the number of characters its value shows, a wide East Asian character
//! counted as two, and the longest line of a text with line breaks. A column's width is its
//! widest cell plus one, capped at [`MAX_AUTO`]. There are no font metrics.
//!
//! What a cell shows is the number-format renderer's text (`crate::format`) for the cell's format
//! code, the one its layers set or the write table's for its value, with English separators and
//! the 1900 epoch the writer always uses.

use crate::format::{self, Options};
use crate::{RichRun, Value, cells};

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

/// The width of `shown` rendered under `code`, General when there is none.
fn rendered(code: Option<&str>, shown: format::Value<'_>) -> usize {
    text(&format::render(
        code.unwrap_or("General"),
        shown,
        &Options::default(),
    ))
}

fn rich(runs: &[RichRun]) -> usize {
    let whole: String = runs.iter().map(|run| run.text.as_str()).collect();
    text(&whole)
}

/// The width of a cell holding `value`. `code` gives the cell's format code for the format the
/// write table gives its value, `None` for General.
pub fn value<'a>(
    value: &Value,
    code: impl FnOnce(crate::styles::Implicit) -> Option<&'a str>,
) -> usize {
    if let Some((number, implicit)) = cells::stored_number(value) {
        return rendered(code(implicit), format::Value::Number(number));
    }
    let code = code(crate::styles::Implicit::None);
    match value {
        // An integer or decimal past 15 significant digits is a text cell.
        Value::Text(value) | Value::Decimal(value) => rendered(code, format::Value::Text(value)),
        Value::Integer(value) => rendered(
            code,
            format::Value::Text(itoa::Buffer::new().format(*value)),
        ),
        Value::Boolean(value) => rendered(code, format::Value::Boolean(*value)),
        Value::Formula {
            cached_text,
            cached_number,
            ..
        } => match (cached_text, cached_number) {
            (Some(cached), _) => rendered(code, format::Value::Text(cached)),
            (None, Some(cached)) if cached.is_finite() => {
                rendered(code, format::Value::Number(*cached))
            }
            _ => 0,
        },
        Value::Rich(runs) => rich(runs),
        _ => 0,
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
