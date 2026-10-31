//! The `writer` resource's workbook: an xlsx [`Writer`] or an ods [`ods::Writer`], by the
//! options' `ods`, behind one set of calls (`rule:core-classes/spreadsheet-writes-as-a-stream`).
//!
//! The two take the same calls and return the same results, so the Novis half writes either
//! from the same code. An xlsx returns its bytes as each call writes them. An ods returns no
//! byte before [`Book::finish`], because it keeps the workbook until then (`ods`' module doc).

use crate::tail::Part;
use crate::{Error, Finished, Row, SheetOptions, Style, WorkbookOptions, Writer, ods};

/// A workbook being written, in one of the two formats. The xlsx writer is boxed, because it
/// is much larger than the ods one.
pub enum Book {
    Xlsx(Box<Writer>),
    Ods(ods::Writer),
}

impl Book {
    /// A workbook with nothing written yet, an ods when `options.ods` is set.
    pub fn open(options: WorkbookOptions) -> Result<Self, Error> {
        if options.ods {
            ods::Writer::open(options).map(Book::Ods)
        } else {
            Writer::open(options).map(|writer| Book::Xlsx(Box::new(writer)))
        }
    }

    /// Where the bytes go, as the program named it.
    pub fn target(&self) -> &str {
        match self {
            Book::Xlsx(writer) => writer.target(),
            Book::Ods(writer) => writer.target(),
        }
    }

    /// Whether [`Book::finish`] has written the whole workbook.
    pub fn is_finished(&self) -> bool {
        match self {
            Book::Xlsx(writer) => writer.is_finished(),
            Book::Ods(writer) => writer.is_finished(),
        }
    }

    /// The id of a style layer, the same id for an equal layer.
    pub fn style(&mut self, style: Style) -> Result<u64, Error> {
        match self {
            Book::Xlsx(writer) => writer.style(style),
            Book::Ods(writer) => writer.style(style),
        }
    }

    /// A defined name over a reference.
    pub fn define(&mut self, name: &str, reference: &str) -> Result<(), Error> {
        match self {
            Book::Xlsx(writer) => writer.define(name, reference),
            Book::Ods(writer) => writer.define(name, reference),
        }
    }

    /// Ends the open sheet, if any, opens the next one and writes its first rows. Returns the
    /// bytes ready to leave.
    pub fn sheet(&mut self, options: SheetOptions, first: &[Row]) -> Result<Vec<u8>, Error> {
        match self {
            Book::Xlsx(writer) => writer.sheet(options, first),
            Book::Ods(writer) => writer.sheet(options, first).map(|()| Vec::new()),
        }
    }

    /// Writes a batch of rows to the open sheet. Returns the bytes ready to leave.
    pub fn rows(&mut self, rows: &[Row]) -> Result<Vec<u8>, Error> {
        match self {
            Book::Xlsx(writer) => writer.rows(rows),
            Book::Ods(writer) => writer.rows(rows).map(|()| Vec::new()),
        }
    }

    /// Adds a sheet-end part to the open sheet. Returns the bytes ready to leave.
    pub fn part(&mut self, part: Part) -> Result<Vec<u8>, Error> {
        match self {
            Book::Xlsx(writer) => writer.part(part),
            Book::Ods(writer) => writer.part(part).map(|()| Vec::new()),
        }
    }

    /// The A1 address of the column at `column`, from 0, in the row `row`.
    pub fn cell(&self, column: u32, row: u32) -> Result<String, Error> {
        crate::writer::address(column, row)
    }

    /// The open sheet's data range in the column at `column`, from 0.
    pub fn column(&self, column: u32) -> Result<String, Error> {
        match self {
            Book::Xlsx(writer) => writer.column(column),
            Book::Ods(writer) => writer.column(column),
        }
    }

    /// Ends the open sheet and the workbook.
    pub fn finish(&mut self) -> Result<Finished, Error> {
        match self {
            Book::Xlsx(writer) => writer.finish(),
            Book::Ods(writer) => writer.finish(),
        }
    }
}
