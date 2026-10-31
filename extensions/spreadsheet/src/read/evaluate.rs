//! A workbook's formulas computed on request (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! [`Reader::evaluate`] reads every worksheet whole into a [`Book`]: each cell's value, each
//! formula cell's A1 text and the value the file stored for it, and each hidden row. A formula
//! cell with empty text is one the reader could not decode, and goes to the book with no text,
//! so the evaluator lists it. The workbook's defined names and every sheet's tables go with
//! them. The book is then computed once and kept on the reader.
//!
//! [`Reader::overlay`] then gives each formula cell a later read reaches its computed result,
//! written as the cell the file would have stored, so the read table and the cell's number
//! format apply to it as to any other. A cell the evaluator did not compute keeps the value
//! the file stored.
//!
//! What it spends, per request, until the reader is freed: one value for each non-empty cell
//! of every worksheet, the formulas' text and parsed trees, and the dependency order. Nothing is
//! read until the program calls `evaluate`.

use super::sheet::RawRow;
use super::{Error, Read, Reader, SheetKind, Source};
use crate::eval::parse::{self, Rect};
use crate::eval::{Book, Missed, Name, Table, Value};

impl Reader {
    /// Computes every formula of the workbook, and returns the cells it could not compute.
    /// Each later read of a sheet gives a computed cell's result.
    pub fn evaluate(&mut self, source: &dyn Source) -> Result<Vec<Missed>, Error> {
        let mut book = Book::new();
        if self.date1904 {
            book.use_1904();
        }
        let sheets: Vec<(String, SheetKind)> = self
            .sheets
            .iter()
            .map(|s| (s.name.clone(), s.kind))
            .collect();
        for (name, kind) in &sheets {
            let at = book.sheet(name);
            if *kind == SheetKind::Chartsheet {
                continue;
            }
            let mut cursor = self.cursor(source, name, true)?;
            while let Some(raw) = cursor.next(source)? {
                self.feed(source, &mut book, at, &raw)?;
            }
        }
        for name in self.names(source)? {
            book.define(Name {
                name: name.name,
                sheet: name.sheet,
                text: name.reference,
            });
        }
        for (name, kind) in &sheets {
            if *kind == SheetKind::Chartsheet {
                continue;
            }
            for (table, header, totals) in self.table_rows(source, name)? {
                let Some(rect) = range(&table.table) else {
                    continue;
                };
                let at = book.sheet(name);
                let columns = (rect.left..=rect.right)
                    .map(|col| match book.value(at, rect.top, col) {
                        Value::Text(text) if header => text.clone(),
                        _ => String::new(),
                    })
                    .collect();
                book.table(Table {
                    name: table.name,
                    sheet: name.clone(),
                    rect,
                    header,
                    totals,
                    columns,
                });
            }
        }
        let missed = book.evaluate();
        self.evaluated = Some(Box::new(book));
        // A read already open gave stored values, so the next read starts again.
        self.open = None;
        Ok(missed)
    }

    /// Puts the cells of `raw` into `book`'s sheet `at`.
    fn feed(
        &mut self,
        source: &dyn Source,
        book: &mut Book,
        at: usize,
        raw: &RawRow,
    ) -> Result<(), Error> {
        let Ok(row) = u32::try_from(raw.number) else {
            return Ok(());
        };
        if raw.hidden {
            book.hide_row(at, row);
        }
        if raw.cells.iter().any(|cell| cell.kind == b"s") {
            self.strings(source)?;
        }
        for cell in &raw.cells {
            let Ok(col) = u32::try_from(cell.column) else {
                continue;
            };
            let value = match self.value(cell)? {
                None => Value::Blank,
                Some(Read::Text(text)) => Value::Text(text),
                Some(Read::Boolean(b)) => Value::Bool(b),
                Some(Read::Error(e)) => Value::Error(e),
                // Every number and time value is the serial the file stores.
                Some(_) => match cell.value.as_deref().map(str::trim).map(str::parse) {
                    Some(Ok(n)) => Value::Number(n),
                    _ => Value::Blank,
                },
            };
            match &cell.formula {
                Some(text) => {
                    let text = (!text.is_empty()).then_some(text.as_str());
                    book.formula(at, row, col, text, value);
                }
                None => book.set(at, row, col, value),
            }
        }
        Ok(())
    }

    /// Gives each formula cell of `raw`, a row of the sheet `sheet`, its computed result,
    /// when the workbook was evaluated.
    pub(super) fn overlay(&self, sheet: &str, raw: &mut RawRow) {
        let Some(book) = &self.evaluated else {
            return;
        };
        let (Some(at), Ok(row)) = (book.find(sheet), u32::try_from(raw.number)) else {
            return;
        };
        for cell in &mut raw.cells {
            let Some(value) = u32::try_from(cell.column)
                .ok()
                .and_then(|col| book.computed(at, row, col))
            else {
                continue;
            };
            cell.inline = None;
            let (kind, text): (&[u8], Option<String>) = match value {
                Value::Blank => (b"n", Some("0".to_owned())),
                Value::Number(n) => (b"n", Some(ryu::Buffer::new().format(*n).to_owned())),
                Value::Text(text) => (b"text", Some(text.clone())),
                Value::Bool(b) => (b"b", Some(if *b { "1" } else { "0" }.to_owned())),
                Value::Error(e) => (b"e", Some(e.text().to_owned())),
            };
            cell.kind = kind.to_vec();
            cell.value = text;
        }
    }
}

/// The cells of a table's range, `A1:C9`.
fn range(text: &str) -> Option<Rect> {
    let text = text.replace('$', "");
    let (first, last) = text.split_once(':').unwrap_or((&text, &text));
    let (top, left) = parse::address(first)?;
    let (bottom, right) = parse::address(last)?;
    Some(Rect {
        top,
        left,
        bottom,
        right,
    })
}
