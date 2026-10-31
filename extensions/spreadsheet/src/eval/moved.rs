//! A1 formula text moved to another cell, as Excel fills a shared formula into each cell it
//! covers (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! [`moved`] adds a row and a column offset to every relative part of every reference: a cell
//! (`B2`), a whole column (`B:C`) and a whole row (`2:3`). A part with `$` stays. Text in quotes,
//! a quoted sheet name and anything in brackets (a structured reference, another workbook) is
//! copied as it is, and so is a name followed by `(`, which is a function. A reference moved
//! off the grid makes the whole text `None`, which the evaluator lists as `token`.
//!
//! What it spends: the text it returns, per formula cell.

const COLUMNS: i64 = 16_384;
const ROWS: i64 = 1_048_576;

/// `text` with its relative references moved by `rows` and `columns`.
pub fn moved(text: &str, rows: i64, columns: i64) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + 8);
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        match bytes[at] {
            quote @ (b'"' | b'\'') => {
                at += 1;
                while at < bytes.len() {
                    if bytes[at] == quote {
                        if bytes.get(at + 1) == Some(&quote) {
                            at += 2;
                            continue;
                        }
                        at += 1;
                        break;
                    }
                    at += 1;
                }
                out.push_str(&text[start..at]);
            }
            b'[' => {
                let mut depth = 0usize;
                while at < bytes.len() {
                    match bytes[at] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                at += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    at += 1;
                }
                out.push_str(&text[start..at]);
            }
            b if word(b) && !(start > 0 && word(bytes[start - 1])) => {
                while at < bytes.len() && word(bytes[at]) {
                    at += 1;
                }
                let token = &text[start..at];
                let ranged =
                    bytes.get(at) == Some(&b':') || (start > 0 && bytes[start - 1] == b':');
                // A function, a table, or a sheet name before `!`.
                if matches!(bytes.get(at), Some(b'(' | b'[' | b'!')) {
                    out.push_str(token);
                } else {
                    out.push_str(&part(token, rows, columns, ranged)?);
                }
            }
            _ => {
                let c = text[at..].chars().next().unwrap_or_default();
                at += c.len_utf8();
                out.push(c);
            }
        }
    }
    Some(out)
}

fn word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'$' | b'\\') || b >= 0x80
}

/// One word of formula text: a cell, a column or a row of a range moved, and anything else as
/// it is.
fn part(token: &str, rows: i64, columns: i64, ranged: bool) -> Option<String> {
    let (first_fixed, rest) = dollar(token.as_bytes());
    let letters = rest.iter().take_while(|c| c.is_ascii_alphabetic()).count();
    let after = &rest[letters..];
    let (row_fixed, digits) = dollar(after);
    let number = (!digits.is_empty() && digits.iter().all(u8::is_ascii_digit))
        .then(|| std::str::from_utf8(digits).ok()?.parse::<i64>().ok())
        .flatten();
    let column = rest[..letters].iter().fold(0i64, |n, c| {
        n * 26 + i64::from(c.to_ascii_uppercase() - b'A' + 1)
    });
    let shift = |value: i64, by: i64, fixed: bool, most: i64| {
        let moved = if fixed { value } else { value + by };
        (1..=most).contains(&moved).then_some(moved)
    };
    let fixed = |on: bool| if on { "$" } else { "" };
    let is_column = (1..=3).contains(&letters) && column <= COLUMNS;
    match number {
        // A cell: letters, then digits.
        Some(row) if is_column && (1..=ROWS).contains(&row) => {
            let col = shift(column, columns, first_fixed, COLUMNS)?;
            let row = shift(row, rows, row_fixed, ROWS)?;
            Some(format!(
                "{}{}{}{row}",
                fixed(first_fixed),
                name(col),
                fixed(row_fixed)
            ))
        }
        // A whole row of a range: digits alone.
        Some(row) if letters == 0 && !row_fixed && ranged => {
            let row = shift(row, rows, first_fixed, ROWS)?;
            Some(format!("{}{row}", fixed(first_fixed)))
        }
        // A whole column of a range: letters alone.
        None if is_column && after.is_empty() && ranged => {
            let col = shift(column, columns, first_fixed, COLUMNS)?;
            Some(format!("{}{}", fixed(first_fixed), name(col)))
        }
        _ => Some(token.to_owned()),
    }
}

/// Whether `b` starts with `$`, and the rest.
fn dollar(b: &[u8]) -> (bool, &[u8]) {
    match b.split_first() {
        Some((b'$', rest)) => (true, rest),
        _ => (false, b),
    }
}

fn name(column: i64) -> String {
    let mut out = Vec::new();
    let mut n = column;
    while n > 0 {
        let rem = (n - 1) % 26;
        out.push(b'A' + rem as u8);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}
