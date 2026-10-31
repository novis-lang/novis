//! The one reader of a reference the program writes as a place in the workbook: a defined name's
//! and a chart series' (`rule:core-classes/spreadsheet-output-is-inert`).
//!
//! A reference is an optional sheet name and `!`, then a cell, a range of cells, a range of whole
//! columns (`$A:$C`) or a range of whole rows (`$1:$3`), each with or without `$`. A sheet name
//! is quoted as Excel quotes one, `'Shop orders'!A1`, when it is not plain, and it follows
//! [`check_sheet_name`]. Nothing else is a reference: another workbook (`[other.xlsx]Shop!A1`), a
//! DDE call (`cmd|'/c calc'!A0`), a URL, a function call, an R1C1 reference and a second sheet
//! name all fail, so no reference a program writes reaches outside the workbook.

use crate::Error;
use crate::chart::quoted;
use crate::tail::{Area, area};
use crate::writer::check_sheet_name;

/// The cells a reference covers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Cells {
    /// A cell or a range of cells.
    Area(Area),
    /// Whole columns, from 0, the first and the last.
    Columns(u32, u32),
    /// Whole rows, from 1, the first and the last.
    Rows(u32, u32),
}

/// A reference read: its sheet name, with quotes and doubled `'` undone, and its cells.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reference {
    pub(crate) sheet: Option<String>,
    pub(crate) cells: Cells,
}

/// Reads `text` as a reference. `whole` allows whole columns and whole rows. A text that is no
/// reference returns `bad()`, and a sheet name Excel does not allow returns its own error.
pub(crate) fn parse(text: &str, whole: bool, bad: impl Fn() -> Error) -> Result<Reference, Error> {
    let (sheet, cells) = match text.strip_prefix('\'') {
        Some(after) => {
            let (name, rest) = closing_quote(after).ok_or_else(&bad)?;
            let cells = rest.strip_prefix('!').ok_or_else(&bad)?;
            (Some(name), cells)
        }
        None => match text.split_once('!') {
            Some((name, cells)) => {
                if name.is_empty() || quoted(name) != name {
                    return Err(bad());
                }
                (Some(name.to_owned()), cells)
            }
            None => (None, text),
        },
    };
    if let Some(name) = &sheet {
        check_sheet_name(name, std::iter::empty())?;
    }
    let cells = read_cells(&cells.to_ascii_uppercase(), whole).ok_or_else(bad)?;
    Ok(Reference { sheet, cells })
}

/// The name up to the `'` that closes it, with each `''` read as one `'`, and the text after it.
fn closing_quote(text: &str) -> Option<(String, &str)> {
    let mut name = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if c != '\'' {
            name.push(c);
        } else if chars.peek().is_some_and(|&(_, next)| next == '\'') {
            name.push('\'');
            chars.next();
        } else {
            return Some((name, &text[at + 1..]));
        }
    }
    None
}

fn read_cells(text: &str, whole: bool) -> Option<Cells> {
    if let Some(at) = area(text) {
        return Some(Cells::Area(at));
    }
    if !whole {
        return None;
    }
    let (from, to) = text.split_once(':')?;
    let part = |side: &str| side.strip_prefix('$').unwrap_or(side).to_owned();
    let (from, to) = (part(from), part(to));
    if let (Some(first), Some(last)) = (column(&from), column(&to)) {
        return (first <= last).then_some(Cells::Columns(first, last));
    }
    let (first, last) = (row(&from)?, row(&to)?);
    (first <= last).then_some(Cells::Rows(first, last))
}

fn column(letters: &str) -> Option<u32> {
    let (at, _) = crate::tail::cell_at(&format!("{letters}1"))?;
    letters
        .bytes()
        .all(|b| b.is_ascii_uppercase())
        .then_some(at)
}

fn row(digits: &str) -> Option<u32> {
    let (_, at) = crate::tail::cell_at(&format!("A{digits}"))?;
    digits.bytes().all(|b| b.is_ascii_digit()).then_some(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str, whole: bool) -> Option<Reference> {
        parse(text, whole, || Error::Invalid(String::new())).ok()
    }

    #[test]
    fn a_range_of_this_workbook_reads() {
        for text in [
            "A1",
            "$A$1:$B$10",
            "Shop!B2:B9",
            "'Shop orders'!$A$1",
            "'It''s'!A1",
            "Shop!$A:$C",
            "Shop!1:3",
            "shop!b2",
        ] {
            assert!(read(text, true).is_some(), "{text}");
        }
        let it = read("'It''s'!A1", true).unwrap();
        assert_eq!(it.sheet.as_deref(), Some("It's"));
        assert_eq!(read("$B:$D", true).unwrap().cells, Cells::Columns(1, 3));
        assert_eq!(read("$2:$4", true).unwrap().cells, Cells::Rows(2, 4));
    }

    #[test]
    fn nothing_else_reads() {
        for text in [
            "",
            "cmd|'/c calc'!A0",
            "[other.xlsx]Shop!B2:B3",
            "'[other.xlsx]Shop'!B2:B3",
            "https://example.com/a!A1",
            "SUM(A1:A3)",
            "R1C1",
            "Shops!Shops!B2:B3",
            "'Shop!B2:B3",
            "Shop orders!A1",
            "Shop!A0",
            "Shop!XFE1",
            "!A1",
            "'History'!A1",
            "$C:$A",
        ] {
            assert!(read(text, true).is_none(), "{text}");
        }
        assert!(read("Shop!$A:$C", false).is_none());
    }
}
