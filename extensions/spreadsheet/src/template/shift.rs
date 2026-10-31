//! The shifter: the A1 references in a formula's text, found and moved, and every other character
//! left as it was (`rule:core-classes/spreadsheet-template-fill`).
//!
//! [`rewrite`] walks the text and calls its map for each row a reference names: a cell (`C5`,
//! `$C$5`), either end of a range (`C5:D9`), either end of a whole-row range (`5:9`), each with
//! an optional sheet (`Items!C5`, `'Q1 sales'!C5`). A whole-column range (`C:D`) names no row
//! and is left alone. A string literal, a structured reference's brackets and a name or function
//! that only looks like a cell (`LOG10(`, `TAX1_RATE`) are skipped. A reference with no sheet is
//! on `home`, the sheet the text belongs to. A 3D reference (`Jan:Mar!C5`) is left alone.
//!
//! [`Shift`] is one repeated block's move: rows below it move by `delta`, a range whose end is
//! in or below it grows, and, inside a copy of the block, a relative row in the block moves with
//! the copy. [`apply`] runs a list of them in order, each in the rows the ones before it left.

use std::fmt::Write as _;

/// Which part of a reference a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Single,
    Start,
    End,
}

/// One row a reference names: its sheet, its number from 1, whether it has a `$`, and which part
/// of the reference it is.
#[derive(Debug, Clone, Copy)]
pub struct RowRef<'a> {
    pub sheet: &'a str,
    pub row: u32,
    pub absolute: bool,
    pub end: End,
}

/// The most rows a sheet has.
pub(crate) const ROWS: u32 = 1_048_576;

pub(crate) fn ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'\\' || b >= 0x80
}

/// A cell part at `at`: `$`, letters, `$`, digits. Returns its column end, its row, whether the
/// row is absolute and where it ends, or `None`.
fn cell(text: &[u8], at: usize) -> Option<(u32, bool, usize)> {
    let mut i = at;
    if text.get(i) == Some(&b'$') {
        i += 1;
    }
    let letters = i;
    while text.get(i).is_some_and(u8::is_ascii_uppercase) {
        i += 1;
    }
    if i == letters || i - letters > 3 {
        return None;
    }
    let absolute = text.get(i) == Some(&b'$');
    if absolute {
        i += 1;
    }
    row_digits(text, i).map(|(row, end)| (row, absolute, end))
}

pub(crate) fn row_digits(text: &[u8], at: usize) -> Option<(u32, usize)> {
    let mut i = at;
    while text.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == at || text[at] == b'0' || i - at > 7 {
        return None;
    }
    let row: u32 = std::str::from_utf8(&text[at..i]).ok()?.parse().ok()?;
    (row <= ROWS).then_some((row, i))
}

/// A whole-row part at `at`: `$`, digits.
pub(crate) fn row_only(text: &[u8], at: usize) -> Option<(u32, bool, usize)> {
    let absolute = text.get(at) == Some(&b'$');
    let from = at + usize::from(absolute);
    row_digits(text, from).map(|(row, end)| (row, absolute, end))
}

/// Whether a reference ending at `end` really ends there: not inside a longer name, and not a
/// function's name.
pub(crate) fn ends(text: &[u8], end: usize) -> bool {
    !text
        .get(end)
        .is_some_and(|&b| ident(b) || b == b'(' || b == b'!')
}

/// A sheet prefix at `at`: its name and where the reference after `!` begins.
pub(crate) fn sheet(text: &str, at: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    if bytes[at] == b'\'' {
        let mut name = String::new();
        let mut i = at + 1;
        loop {
            let quote = i + text[i..].find('\'')?;
            name.push_str(&text[i..quote]);
            if bytes.get(quote + 1) == Some(&b'\'') {
                name.push('\'');
                i = quote + 2;
            } else {
                return (bytes.get(quote + 1) == Some(&b'!')).then_some((name, quote + 2));
            }
        }
    }
    let mut i = at;
    while bytes.get(i).is_some_and(|&b| ident(b)) {
        i += 1;
    }
    (i > at && bytes.get(i) == Some(&b'!')).then(|| (text[at..i].to_owned(), i + 1))
}

/// Calls `map` for each row a reference in `text` names, and returns the text with each row
/// replaced by what `map` returned. A row `map` returns unchanged keeps its characters.
pub fn rewrite(text: &str, home: &str, map: &mut dyn FnMut(RowRef<'_>) -> u32) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut at = 0;
    while at < bytes.len() {
        let b = bytes[at];
        if b == b'"' {
            at += 1;
            while at < bytes.len() {
                if bytes[at] == b'"' {
                    if bytes.get(at + 1) == Some(&b'"') {
                        at += 2;
                        continue;
                    }
                    break;
                }
                at += 1;
            }
            at += 1;
            continue;
        }
        if b == b'[' {
            let mut depth = 0;
            while at < bytes.len() {
                match bytes[at] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                at += 1;
            }
            at += 1;
            continue;
        }
        let starts =
            at == 0 || !ident(bytes[at - 1]) && !matches!(bytes[at - 1], b'\'' | b':' | b'!');
        if !starts || !(b == b'$' || b == b'\'' || ident(b)) {
            at += 1;
            continue;
        }
        let (name, begin) = match sheet(text, at) {
            Some((name, begin)) => (Some(name), begin),
            None => (None, at),
        };
        let on = name.as_deref().unwrap_or(home);
        // Each part found: where its row digits are, the row, `$`, and which end it is.
        let mut parts: Vec<(usize, usize, u32, bool, End)> = Vec::new();
        let mut end = begin;
        if let Some((row, absolute, stop)) = cell(bytes, begin) {
            let digits = stop - row_len(row);
            if bytes.get(stop) == Some(&b':')
                && let Some((row2, absolute2, stop2)) = cell(bytes, stop + 1)
                && ends(bytes, stop2)
            {
                parts.push((digits, stop, row, absolute, End::Start));
                parts.push((stop2 - row_len(row2), stop2, row2, absolute2, End::End));
                end = stop2;
            } else if ends(bytes, stop) && bytes.get(stop) != Some(&b':') {
                parts.push((digits, stop, row, absolute, End::Single));
                end = stop;
            }
        } else if let Some((row, absolute, stop)) = row_only(bytes, begin)
            && bytes.get(stop) == Some(&b':')
            && let Some((row2, absolute2, stop2)) = row_only(bytes, stop + 1)
            && ends(bytes, stop2)
        {
            parts.push((stop - row_len(row), stop, row, absolute, End::Start));
            parts.push((stop2 - row_len(row2), stop2, row2, absolute2, End::End));
            end = stop2;
        }
        if parts.is_empty() {
            // Not a reference: skip the whole name, so its tail is not read as one.
            at = begin.max(at + 1);
            while at < bytes.len() && (ident(bytes[at]) || bytes[at] == b'$') {
                at += 1;
            }
            continue;
        }
        for (digits, stop, row, absolute, which) in parts {
            let moved = map(RowRef {
                sheet: on,
                row,
                absolute,
                end: which,
            });
            if moved != row {
                out.push_str(&text[copied..digits]);
                let _ = write!(out, "{moved}");
                copied = stop;
            }
        }
        at = end;
    }
    out.push_str(&text[copied..]);
    out
}

fn row_len(row: u32) -> usize {
    itoa::Buffer::new().format(row).len()
}

/// One repeated block's move, on `sheet`: the block's rows `top` to `bottom` as the earlier
/// shifts left them, the rows its copies added (`delta`, below zero when a list is empty), and
/// `copy`, the rows from the block's first copy to the one the text is in.
#[derive(Debug, Clone)]
pub struct Shift {
    pub sheet: String,
    pub top: u32,
    pub bottom: u32,
    pub delta: i64,
    pub copy: Option<u32>,
}

impl Shift {
    /// Where the row `at` goes.
    pub fn row(&self, at: RowRef<'_>) -> u32 {
        if !at.sheet.eq_ignore_ascii_case(&self.sheet) {
            return at.row;
        }
        let inside = at.row >= self.top && at.row <= self.bottom;
        if let Some(copy) = self.copy
            && inside
        {
            return if at.absolute { at.row } else { at.row + copy };
        }
        let moves = match at.end {
            End::End => at.row >= self.top,
            End::Single | End::Start => at.row > self.bottom,
        };
        if !moves {
            return at.row;
        }
        let moved = (i64::from(at.row) + self.delta).clamp(1, i64::from(ROWS)) as u32;
        // A range over an empty list keeps at least its first row.
        if at.end == End::End && moved < self.top {
            self.top
        } else {
            moved
        }
    }
}

/// `text` with every shift in `shifts` applied, in order.
pub fn apply(text: &str, home: &str, shifts: &[Shift]) -> String {
    if shifts.is_empty() {
        return text.to_owned();
    }
    let mut map = |at: RowRef<'_>| {
        shifts
            .iter()
            .fold(at.row, |row, shift| shift.row(RowRef { row, ..at }))
    };
    rewrite(text, home, &mut map)
}

/// A space-separated list of references, as `sqref` holds, with every shift applied.
pub fn apply_list(text: &str, home: &str, shifts: &[Shift]) -> String {
    text.split(' ')
        .map(|one| apply(one, home, shifts))
        .collect::<Vec<_>>()
        .join(" ")
}
