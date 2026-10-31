//! The spreadsheet component: `nvs:spreadsheet/engine` from `wit/spreadsheet.wit`, built into
//! every `nvs` binary (`rule:packaging/the-first-party-components-are-built-in`,
//! `rule:core-classes/spreadsheet-roster`).
//!
//! The crate has two halves, as the image component's does. The core is plain Rust that builds
//! for the host too, so its tests run under an ordinary `cargo test` in this directory. The WIT
//! glue in `guest` is compiled only for a wasm target, and maps each export onto the core and
//! the WIT types onto the core's.
//!
//! **Writing is a stream** (`rule:core-classes/spreadsheet-writes-as-a-stream`). A `writer`
//! resource is a [`Writer`]: `writer-sheet` and `writer-rows` take a batch of rows, write its
//! sheet XML as text, deflate it, and return the compressed bytes ready to leave, and
//! `writer-finish` writes the rest of the workbook and the zip directory. The writer is ours and
//! frames the zip by hand (`zip`'s module doc), so the guest never knows an entry's size in
//! advance, never seeks, and keeps no row of an earlier batch. Strings are inline, so no
//! shared-string table grows. Each cell is written by the write table (`cells`' module doc), and
//! its format is its layers and its value's format combined (`styles`' module doc). The sheet
//! order, the parts and the fixed dates are `writer`'s module doc. The component performs no
//! I/O and is granted nothing (`rule:core-classes/spreadsheet-has-no-io`): the Novis half sends
//! the bytes on.
//!
//! **A workbook never finished is logged.** The guest's `writer` is dropped when the request
//! ends, and one dropped before `writer-finish` writes an error to the request's log naming the
//! workbook's `target` ([`writer::unfinished_message`]).
//!
//! **Reading** is `read`'s module doc: every `reader-` export reads the workbook through the
//! `range` of the source its call is lent.
//!
//! **Template fill** is `template`'s module doc. Its `filling` resource is not named
//! `template`, because a manifest's resource is a class in `Novis\Spreadsheet`, where `Template`
//! is the Novis class over it.
//!
//! Packed as a `.nvsx` and embedded in every `nvs` binary, the component is 0.96 MiB, built for
//! release as `crates/nvs-ext/build.rs` always builds it. That is the binary's growth for
//! spreadsheets, and `crates/nvs-ext/tests/builtin.rs` fails when this figure drifts past 5% of
//! the bytes the binary carries.
//!
//! **Not done yet.** Writing ods and an evaluate call are named in their rules'
//! fragments. These are the rest, none of them is checked against Excel, and they are
//! gap:nvs-spreadsheet/spreadsheet-reader-and-writer-leftovers:
//!
//! - Number formats (`format`): built-in id 14 renders as `m/d/yyyy` and id 22 as
//!   `m/d/yyyy h:mm`, as Excel shows them in en-US, where SSF uses `m/d/yy`.
//! - Merges (`read::merge`): a merged fill copies no `display`.
//! - Extras (`read::extras`): `notes` returns the `_x000D_` and `_x0000_` escapes undecoded.
//! - Rows (`read::sheet`): `Reader::rows` skips a row whose cells are all formulas with no cached
//!   value, so a totals row of formulas this writer wrote does not read back. `rows` accepts an
//!   unknown locale.
//! - Sheets (`read`, `nvs/Reader.nvs`): `Reader::sheets` gives `rows: 0, columns: 0` for a file
//!   [`Writer`] wrote, because the writer writes no sheet dimension, and it reads every sheet on
//!   each call. A sheet name from `sheets()` is a `tainted string`, and the other methods take
//!   `string|int`. Bytes that are not a spreadsheet open as a one-sheet CSV.
//! - The decompression-bound errors (`read::zip`) name a rule id, which a message a user reads
//!   should not.
//! - Charts (`chart`): a series range may point into another workbook or a URL
//!   (`[other.xlsx]Shop!B2:B3`), a sheet name in it may have 10,000 characters or control
//!   characters, an unclosed quote is read as part of the name, and `Shops!Shops!B2:B3` reads as
//!   the sheet `Shops!Shops`. The sheet name is escaped as XML text. `Sheet::chart` accepts a
//!   title of 100,000 characters.
//! - Sheet-end parts (`drawing`, `tail`, `conditional`, `nvs/Sheet.nvs`): `Sheet::image` accepts a
//!   PNG cut to 30 bytes and a width of 10,000,000 pixels. `Sheet::validate` writes text limits
//!   for a `whole` rule. `Sheet::conditional` takes a 10,000-character formula, where Excel allows
//!   8,192. A second `Sheet::print` replaces the first one's settings.
//! - Addresses (`nvs/Sheet.nvs`): `Sheet::cell` after the sheet is finished throws "cannot add
//!   parts to it". `Sheet::column($key)` before any data row returns the header cell, and the
//!   `writerColumn` export on a sheet with no data row returns a one-cell range. `cell('qty', 2)`
//!   before the first keyed row reads the key as column letters. The `writerCell` export names a
//!   column cut to 32 bits in its message.
//! - Names and values (`writer`, `cells`, `nvs/Workbook.nvs`): `Workbook::define` writes a
//!   DDE-like reference such as `cmd|'/c calc'!A0`, and takes `R1C1` and a 100,000-character
//!   name. `Workbook::sheet` takes `History` and a name with NUL. `Formula::of` takes an empty text
//!   or 1,000,000 characters, where Excel allows 8,192. A sheet with
//!   `{header: true, autofilter: true}` gets no filter name, where `fromRows` writes one. A NaN
//!   throws for xlsx and is written for CSV.
//! - `Export` (`nvs/Export.nvs`): an `Export` over a generator gives no rows the second time.
//! - Engine exports (`guest`): the reader exports do not check that `source` is the file
//!   `readerOpen` read. `readerRows` with `max` 0 returns `done: false`. Each `readerProperties`
//!   call counts against the reading's byte bound.
//! - Cost: `Template::open` costs about 2.6 ms and 1.1 MB on a tiny template, and one `set` about
//!   1.3 ms and 600 KB. 20,000 merges take about 15 s in a debug build. A request keeps every
//!   extension resource until it ends (`crates/nvs-ext/src/call.rs`), so some hundreds of
//!   unfinished writers in one request reach the extension's memory limit. That is O(in-flight)
//!   and under the request's cap.

pub mod book;
pub mod cells;
pub mod chart;
pub mod conditional;
pub mod drawing;
pub mod eval;
pub mod format;
pub mod ods;
pub mod openformula;
pub mod read;
pub mod styles;
pub mod table;
pub mod tail;
pub mod template;
pub mod width;
pub mod writer;
pub mod xml;
pub mod zip;

pub use book::Book;
pub use writer::Writer;

/// The WIT `error`: `Invalid`, `Parse` and `Runtime` throw `LogicError`, `ParseError` and
/// `RuntimeError` (`rule:packaging/a-guest-crash-throws`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Invalid(String),
    Parse(String),
    Runtime(String),
}

/// A calendar date, `month` and `day` from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

/// A time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub hour: i64,
    pub minute: i64,
    pub second: i64,
    pub nanos: i64,
}

/// A clock time in no zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub day: Date,
    pub clock: TimeOfDay,
}

/// One run of rich text, with an optional style layer.
#[derive(Debug, Clone, PartialEq)]
pub struct RichRun {
    pub text: String,
    pub style: Option<u64>,
}

/// One cell's value, by the write table.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Number(f64),
    Integer(i64),
    Decimal(String),
    Boolean(bool),
    Date(Date),
    Time(TimeOfDay),
    /// A duration in nanoseconds.
    Duration(i64),
    Clock(Clock),
    /// A formula's text with no leading `=`, and its cached result. A cached text wins over a
    /// cached number.
    Formula {
        formula: String,
        cached_text: Option<String>,
        cached_number: Option<f64>,
    },
    Rich(Vec<RichRun>),
}

/// One cell: its value, and its own style layer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cell {
    pub value: Option<Value>,
    pub style: Option<u64>,
}

/// One row and what xlsx writes with it. A `header` row is the sheet's header row, which only the
/// sheet's first row can be (`writer`'s module doc).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub style: Option<u64>,
    pub height: Option<f64>,
    pub hidden: bool,
    pub level: u64,
    pub header: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Thin,
    Medium,
    Thick,
    Dashed,
    Dotted,
    Double,
}

/// The four sides of a border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Border {
    pub top: Option<Line>,
    pub right: Option<Line>,
    pub bottom: Option<Line>,
    pub left: Option<Line>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Underline {
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
    Fill,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Valign {
    Top,
    Center,
    Bottom,
}

/// One style layer (`rule:core-classes/spreadsheet-styles-and-defaults`). `format` is a format
/// code or one of the shortcuts, which [`styles::Styles::layer`] turns into its code.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    pub font: Option<String>,
    pub size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub strike: Option<bool>,
    pub underline: Option<Underline>,
    pub color: Option<String>,
    pub fill: Option<String>,
    pub border: Option<Border>,
    pub border_color: Option<String>,
    pub align: Option<Align>,
    pub valign: Option<Valign>,
    pub wrap: Option<bool>,
    pub rotate: Option<i64>,
    pub indent: Option<u64>,
    pub format: Option<String>,
    pub locked: Option<bool>,
    pub hide_formula: Option<bool>,
}

impl Style {
    /// This layer over `base`: each field this layer sets replaces the base's, and a border
    /// side this layer sets replaces that side.
    pub fn over(&self, base: &Style) -> Style {
        let border = match (self.border, base.border) {
            (Some(top), Some(under)) => Some(Border {
                top: top.top.or(under.top),
                right: top.right.or(under.right),
                bottom: top.bottom.or(under.bottom),
                left: top.left.or(under.left),
            }),
            (top, under) => top.or(under),
        };
        Style {
            font: self.font.clone().or_else(|| base.font.clone()),
            size: self.size.or(base.size),
            bold: self.bold.or(base.bold),
            italic: self.italic.or(base.italic),
            strike: self.strike.or(base.strike),
            underline: self.underline.or(base.underline),
            color: self.color.clone().or_else(|| base.color.clone()),
            fill: self.fill.clone().or_else(|| base.fill.clone()),
            border,
            border_color: self
                .border_color
                .clone()
                .or_else(|| base.border_color.clone()),
            align: self.align.or(base.align),
            valign: self.valign.or(base.valign),
            wrap: self.wrap.or(base.wrap),
            rotate: self.rotate.or(base.rotate),
            indent: self.indent.or(base.indent),
            format: self.format.clone().or_else(|| base.format.clone()),
            locked: self.locked.or(base.locked),
            hide_formula: self.hide_formula.or(base.hide_formula),
        }
    }
}

/// The document properties `docProps/` carries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Properties {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub author: Option<String>,
    pub company: Option<String>,
    pub keywords: Option<String>,
    pub created: Option<Clock>,
}

/// What `writer-open` takes. `target` names where the bytes go, for the log line of a workbook
/// never finished. `ods` writes an ods ([`ods::Writer`]) in place of an xlsx.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkbookOptions {
    pub properties: Properties,
    pub target: String,
    pub ods: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
    VeryHidden,
}

/// A column, in header order. `format` is a format code or a shortcut over the column's `style`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Column {
    pub width: Option<f64>,
    pub auto: bool,
    pub style: Option<u64>,
    pub format: Option<String>,
    pub hidden: bool,
}

/// What a sheet's header row gets, when its first row is one. `default_style` is bold with a thin
/// bottom border, and `style` is a layer over it, or the whole style when `default_style` is
/// off. `freeze` freezes the rows above the second, and `filter` puts an autofilter over the
/// header and every row below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub default_style: bool,
    pub style: Option<u64>,
    pub freeze: bool,
    pub filter: bool,
}

impl Default for Header {
    /// The defaults of a sheet with a header row: every one on.
    fn default() -> Self {
        Self {
            default_style: true,
            style: None,
            freeze: true,
            filter: true,
        }
    }
}

/// What `writer-sheet` takes beside the first rows. A column past `columns` has auto width when
/// the sheet has a header row, and no width otherwise. `new_sheet_on_overflow` continues a sheet
/// that reaches the last row of the grid on a new sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetOptions {
    pub name: String,
    pub position: Option<u64>,
    pub visibility: Visibility,
    pub columns: Vec<Column>,
    pub freeze_rows: u64,
    pub freeze_columns: u64,
    pub autofilter: Option<String>,
    pub tab_color: Option<String>,
    pub zoom: Option<u64>,
    pub gridlines: bool,
    pub right_to_left: bool,
    pub header: Header,
    pub new_sheet_on_overflow: bool,
}

impl SheetOptions {
    /// A visible sheet named `name`, with gridlines, the header defaults, and nothing else set.
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            position: None,
            visibility: Visibility::Visible,
            columns: Vec::new(),
            freeze_rows: 0,
            freeze_columns: 0,
            autofilter: None,
            tab_color: None,
            zoom: None,
            gridlines: true,
            right_to_left: false,
            header: Header::default(),
            new_sheet_on_overflow: false,
        }
    }
}

/// One sheet's row count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetRows {
    pub name: String,
    pub rows: u64,
}

/// What `writer-finish` returns: the last bytes, the rows of each sheet in the order they were
/// written, and the size of every byte the workbook returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub data: Vec<u8>,
    pub rows: Vec<SheetRows>,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Cell {
        Cell {
            value: Some(Value::Text(value.to_owned())),
            style: None,
        }
    }

    fn write() -> Vec<u8> {
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        let bold = writer
            .style(Style {
                bold: Some(true),
                ..Style::default()
            })
            .unwrap();
        let header = Row {
            cells: vec![text("Name"), text("Price")],
            style: Some(bold),
            ..Row::default()
        };
        let mut out = writer
            .sheet(SheetOptions::named("Shop"), &[header])
            .unwrap();
        let row = Row {
            cells: vec![
                text(" Tea "),
                Cell {
                    value: Some(Value::Number(3.5)),
                    style: None,
                },
            ],
            ..Row::default()
        };
        out.extend(writer.rows(&[row]).unwrap());
        let finished = writer.finish().unwrap();
        out.extend(&finished.data);
        assert_eq!(finished.size, out.len() as u64);
        assert_eq!(
            finished.rows,
            [SheetRows {
                name: "Shop".to_owned(),
                rows: 2
            }]
        );
        out
    }

    #[test]
    fn two_writes_of_equal_input_are_byte_equal() {
        assert_eq!(write(), write());
    }

    #[test]
    fn a_nan_is_refused_naming_its_cell_and_the_sheet_goes_on() {
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        writer.sheet(SheetOptions::named("Blog"), &[]).unwrap();
        let row = Row {
            cells: vec![
                text("a"),
                Cell {
                    value: Some(Value::Number(f64::NAN)),
                    style: None,
                },
            ],
            ..Row::default()
        };
        let err = writer.rows(&[row]).unwrap_err();
        assert!(
            matches!(&err, Error::Invalid(m) if m.contains("`B1`")),
            "{err:?}"
        );
        writer.rows(&[Row::default()]).unwrap();
        assert_eq!(writer.finish().unwrap().rows[0].rows, 1);
    }

    #[test]
    fn a_bad_sheet_name_is_refused_naming_the_rule() {
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        let err = writer.sheet(SheetOptions::named("a/b"), &[]).unwrap_err();
        assert!(
            matches!(&err, Error::Invalid(m) if m.contains("`/`")),
            "{err:?}"
        );
    }
}

#[cfg(target_family = "wasm")]
mod guest {
    wit_bindgen::generate!({
        path: ["../../wit/nvs-ext", "../../wit/spreadsheet.wit"],
        world: "nvs:spreadsheet/spreadsheet",
        generate_all,
    });

    use std::cell::RefCell;

    use crate::tail;
    use exports::nvs::spreadsheet::engine as wit;
    use exports::nvs::spreadsheet::engine::{Guest, GuestFilling, GuestReading, GuestWriter};
    use nvs::ext::log;
    use nvs::ext::types::Value as Source;

    struct Component;

    /// A workbook being written, kept by the request that opened it.
    pub struct Book {
        writer: RefCell<crate::Book>,
    }

    impl GuestWriter for Book {}

    impl Drop for Book {
        fn drop(&mut self) {
            let writer = self.writer.get_mut();
            if !writer.is_finished() {
                log::write(
                    log::Level::Error,
                    &crate::writer::unfinished_message(writer.target()),
                );
            }
        }
    }

    /// A workbook being read, kept by the request that opened it.
    pub struct Reader {
        reader: RefCell<crate::read::Reader>,
    }

    impl GuestReading for Reader {}

    /// The source a reader call was lent, `bytes` or an open file, read through the host's
    /// `range`. The host gives at most one block per `range`, so a longer read asks again from
    /// where the last block ended, until it has `length` bytes or the source ends.
    struct Lent<'a>(&'a Source);

    fn not_a_source() -> crate::Error {
        crate::Error::Invalid(
            "A workbook is read from `bytes` or an open `Core\\IO\\File`.".to_owned(),
        )
    }

    impl crate::read::Source for Lent<'_> {
        fn size(&self) -> Result<u64, crate::Error> {
            self.0.size().ok_or_else(not_a_source)
        }

        fn range(&self, offset: u64, length: u64) -> Result<Vec<u8>, crate::Error> {
            let mut read = self.0.range(offset, length).ok_or_else(not_a_source)?;
            while (read.len() as u64) < length {
                let got = read.len() as u64;
                let block = self
                    .0
                    .range(offset.saturating_add(got), length - got)
                    .ok_or_else(not_a_source)?;
                if block.is_empty() {
                    break;
                }
                read.extend_from_slice(&block);
            }
            Ok(read)
        }
    }

    /// The bound the host clamped, each half missing only where no host did, and then the
    /// shipped default.
    fn bound_of(bound: wit::DecompressionBound) -> crate::read::zip::Bound {
        let shipped = crate::read::zip::Bound::default();
        crate::read::zip::Bound {
            bytes: bound.max_bytes.unwrap_or(shipped.bytes),
            ratio: bound.max_ratio.unwrap_or(shipped.ratio),
        }
    }

    fn read_options(options: wit::ReadOptions) -> crate::read::ReadOptions {
        crate::read::ReadOptions {
            sheet: options.sheet,
            header_row: options.header_row,
            from: options.from,
            limit: options.limit,
            columns: options.columns,
            text: options.text,
            text_all: options.text_all,
            formulas: options.formulas,
            keep_empty_rows: options.keep_empty_rows,
            fill_merged: options.fill_merged,
            skip_hidden: options.skip_hidden,
            locale: options.locale,
        }
    }

    fn sheet_info(sheet: crate::read::SheetInfo) -> wit::SheetInfo {
        wit::SheetInfo {
            name: sheet.name,
            position: sheet.position,
            visibility: match sheet.visibility {
                crate::Visibility::Visible => wit::Visibility::Visible,
                crate::Visibility::Hidden => wit::Visibility::Hidden,
                crate::Visibility::VeryHidden => wit::Visibility::VeryHidden,
            },
            rows: sheet.rows,
            columns: sheet.columns,
            kind: match sheet.kind {
                crate::read::SheetKind::Worksheet => wit::SheetKind::Worksheet,
                crate::read::SheetKind::Chartsheet => wit::SheetKind::Chartsheet,
            },
        }
    }

    fn wit_date(date: crate::Date) -> wit::Date {
        wit::Date {
            year: date.year,
            month: date.month,
            day: date.day,
        }
    }

    fn wit_time(time: crate::TimeOfDay) -> wit::TimeOfDay {
        wit::TimeOfDay {
            hour: time.hour,
            minute: time.minute,
            second: time.second,
            nanos: time.nanos,
        }
    }

    fn read_value(value: crate::read::Read) -> wit::ReadValue {
        use crate::read::{CellError, Read};
        match value {
            Read::Text(text) => wit::ReadValue::Text(wit::TextCell { text }),
            Read::Integer(integer) => wit::ReadValue::Integer(wit::IntegerCell { integer }),
            Read::Number(number) => wit::ReadValue::Number(wit::NumberCell { number }),
            Read::Boolean(boolean) => wit::ReadValue::Boolean(wit::BoolCell { boolean }),
            Read::Clock(clock) => wit::ReadValue::Clock(wit::ClockCell {
                day: wit_date(clock.day),
                clock: wit_time(clock.clock),
            }),
            Read::Date(date) => wit::ReadValue::Date(wit::DateCell {
                date: wit_date(date),
            }),
            Read::Time(time) => wit::ReadValue::Time(wit::TimeCell {
                time: wit_time(time),
            }),
            Read::Duration(nanos) => wit::ReadValue::Duration(wit::DurationCell {
                duration: wit::Duration { nanos },
            }),
            Read::Error(error) => wit::ReadValue::Error(wit::ErrorCell {
                error: match error {
                    CellError::Null => wit::CellError::Null,
                    CellError::DivZero => wit::CellError::DivZero,
                    CellError::Value => wit::CellError::Value,
                    CellError::Ref => wit::CellError::Ref,
                    CellError::Name => wit::CellError::Name,
                    CellError::Num => wit::CellError::Num,
                    CellError::NotAvailable => wit::CellError::NotAvailable,
                    CellError::GettingData => wit::CellError::GettingData,
                    CellError::Spill => wit::CellError::Spill,
                    CellError::Calc => wit::CellError::Calc,
                },
            }),
        }
    }

    fn read_row(row: crate::read::ReadRow) -> wit::ReadRow {
        wit::ReadRow {
            number: row.number,
            cells: row
                .cells
                .into_iter()
                .map(|cell| wit::ReadCell {
                    column: cell.column,
                    value: cell.value.map(read_value),
                    formula: cell.formula,
                    display: cell.display,
                })
                .collect(),
        }
    }

    /// A template being filled, kept by the request that opened it.
    pub struct Filling {
        template: RefCell<crate::template::Template>,
    }

    impl GuestFilling for Filling {}

    fn placeholders(values: Vec<wit::Placeholder>) -> Vec<crate::template::Placeholder> {
        values
            .into_iter()
            .map(|placeholder| (placeholder.path, placeholder.value.map(value)))
            .collect()
    }

    fn finished(finished: crate::Finished) -> wit::Finished {
        wit::Finished {
            data: finished.data,
            rows: finished
                .rows
                .into_iter()
                .map(|sheet| wit::SheetRows {
                    name: sheet.name,
                    rows: sheet.rows,
                })
                .collect(),
            size: finished.size,
        }
    }

    fn error(err: crate::Error) -> wit::Error {
        match err {
            crate::Error::Invalid(message) => wit::Error::Invalid(message),
            crate::Error::Parse(message) => wit::Error::Parse(message),
            crate::Error::Runtime(message) => wit::Error::Runtime(message),
        }
    }

    fn date(date: wit::Date) -> crate::Date {
        crate::Date {
            year: date.year,
            month: date.month,
            day: date.day,
        }
    }

    fn time(time: wit::TimeOfDay) -> crate::TimeOfDay {
        crate::TimeOfDay {
            hour: time.hour,
            minute: time.minute,
            second: time.second,
            nanos: time.nanos,
        }
    }

    fn clock(clock: wit::ClockCell) -> crate::Clock {
        crate::Clock {
            day: date(clock.day),
            clock: time(clock.clock),
        }
    }

    fn value(value: wit::CellValue) -> crate::Value {
        use crate::Value;
        match value {
            wit::CellValue::Text(cell) => Value::Text(cell.text),
            wit::CellValue::Number(cell) => Value::Number(cell.number),
            wit::CellValue::Integer(cell) => Value::Integer(cell.integer),
            wit::CellValue::Decimal(cell) => Value::Decimal(cell.decimal),
            wit::CellValue::Boolean(cell) => Value::Boolean(cell.boolean),
            wit::CellValue::Date(cell) => Value::Date(date(cell.date)),
            wit::CellValue::Time(cell) => Value::Time(time(cell.time)),
            wit::CellValue::Duration(cell) => Value::Duration(cell.duration.nanos),
            wit::CellValue::Clock(cell) => Value::Clock(clock(cell)),
            wit::CellValue::Formula(cell) => Value::Formula {
                formula: cell.formula,
                cached_text: cell.cached_text,
                cached_number: cell.cached_number,
            },
            wit::CellValue::Rich(cell) => Value::Rich(
                cell.runs
                    .into_iter()
                    .map(|run| crate::RichRun {
                        text: run.text,
                        style: run.style,
                    })
                    .collect(),
            ),
        }
    }

    fn rows(rows: Vec<wit::Row>) -> Vec<crate::Row> {
        rows.into_iter()
            .map(|row| crate::Row {
                cells: row
                    .cells
                    .into_iter()
                    .map(|cell| crate::Cell {
                        value: cell.value.map(value),
                        style: cell.style,
                    })
                    .collect(),
                style: row.style,
                height: row.height,
                hidden: row.hidden,
                level: row.level,
                header: row.header,
            })
            .collect()
    }

    fn line(line: wit::Line) -> crate::Line {
        match line {
            wit::Line::Thin => crate::Line::Thin,
            wit::Line::Medium => crate::Line::Medium,
            wit::Line::Thick => crate::Line::Thick,
            wit::Line::Dashed => crate::Line::Dashed,
            wit::Line::Dotted => crate::Line::Dotted,
            wit::Line::Double => crate::Line::Double,
        }
    }

    fn style(style: wit::Style) -> crate::Style {
        crate::Style {
            font: style.font,
            size: style.size,
            bold: style.bold,
            italic: style.italic,
            strike: style.strike,
            underline: style.underline.map(|underline| match underline {
                wit::Underline::Single => crate::Underline::Single,
                wit::Underline::Double => crate::Underline::Double,
            }),
            color: style.color,
            fill: style.fill,
            border: style.border.map(|border| crate::Border {
                top: border.top.map(line),
                right: border.right.map(line),
                bottom: border.bottom.map(line),
                left: border.left.map(line),
            }),
            border_color: style.border_color,
            align: style.align.map(|align| match align {
                wit::Align::Left => crate::Align::Left,
                wit::Align::Center => crate::Align::Center,
                wit::Align::Right => crate::Align::Right,
                wit::Align::Fill => crate::Align::Fill,
                wit::Align::Justify => crate::Align::Justify,
            }),
            valign: style.valign.map(|valign| match valign {
                wit::Valign::Top => crate::Valign::Top,
                wit::Valign::Center => crate::Valign::Center,
                wit::Valign::Bottom => crate::Valign::Bottom,
            }),
            wrap: style.wrap,
            rotate: style.rotate,
            indent: style.indent,
            format: style.format,
            locked: style.locked,
            hide_formula: style.hide_formula,
        }
    }

    fn sheet_options(options: wit::SheetOptions) -> crate::SheetOptions {
        crate::SheetOptions {
            name: options.name,
            position: options.position,
            visibility: match options.visibility {
                wit::Visibility::Visible => crate::Visibility::Visible,
                wit::Visibility::Hidden => crate::Visibility::Hidden,
                wit::Visibility::VeryHidden => crate::Visibility::VeryHidden,
            },
            columns: options
                .columns
                .into_iter()
                .map(|column| crate::Column {
                    width: column.width,
                    auto: column.auto,
                    style: column.style,
                    format: column.format,
                    hidden: column.hidden,
                })
                .collect(),
            freeze_rows: options.freeze_rows,
            freeze_columns: options.freeze_columns,
            autofilter: options.autofilter,
            tab_color: options.tab_color,
            zoom: options.zoom,
            gridlines: options.gridlines,
            right_to_left: options.right_to_left,
            header: crate::Header {
                default_style: options.header.default_style,
                style: options.header.style,
                freeze: options.header.freeze,
                filter: options.header.filter,
            },
            new_sheet_on_overflow: options.new_sheet_on_overflow,
        }
    }

    fn operator(operator: wit::Operator) -> tail::Operator {
        match operator {
            wit::Operator::Between => tail::Operator::Between,
            wit::Operator::NotBetween => tail::Operator::NotBetween,
            wit::Operator::Equal => tail::Operator::Equal,
            wit::Operator::NotEqual => tail::Operator::NotEqual,
            wit::Operator::Greater => tail::Operator::Greater,
            wit::Operator::GreaterOrEqual => tail::Operator::GreaterOrEqual,
            wit::Operator::Less => tail::Operator::Less,
            wit::Operator::LessOrEqual => tail::Operator::LessOrEqual,
        }
    }

    /// The core's part for `part`. Every kind of `wit::SheetPart` converts, so it never
    /// returns an error.
    fn sheet_part(part: wit::SheetPart) -> Result<tail::Part, wit::Error> {
        use tail::Part;
        Ok(match part {
            wit::SheetPart::Merge(merge) => Part::Merge(merge.merge),
            wit::SheetPart::Link(link) => Part::Link(tail::Link {
                anchor: link.anchor,
                target: link.target,
                text: link.text,
                tooltip: link.tooltip,
            }),
            wit::SheetPart::Note(note) => Part::Note(tail::Note {
                noted: note.noted,
                note: note.note,
                author: note.author,
            }),
            wit::SheetPart::Validation(rule) => Part::Validation(tail::Validation {
                validated: rule.validated,
                kind: match rule.kind {
                    wit::RuleKind::List => tail::RuleKind::List,
                    wit::RuleKind::Whole => tail::RuleKind::Whole,
                    wit::RuleKind::Decimal => tail::RuleKind::Decimal,
                    wit::RuleKind::Date => tail::RuleKind::Date,
                    wit::RuleKind::Length => tail::RuleKind::Length,
                    wit::RuleKind::Formula => tail::RuleKind::Formula,
                },
                operator: rule.operator.map(operator),
                values: rule.values,
                input_title: rule.input_title,
                input_message: rule.input_message,
                error_title: rule.error_title,
                error_message: rule.error_message,
                allow_empty: rule.allow_empty,
            }),
            wit::SheetPart::Group(group) => Part::Group(tail::Group {
                grouped: group.grouped,
                collapsed: group.collapsed,
            }),
            wit::SheetPart::Print(print) => Part::Print(tail::Print {
                orientation: print.orientation.map(|orientation| match orientation {
                    wit::Orientation::Portrait => tail::Orientation::Portrait,
                    wit::Orientation::Landscape => tail::Orientation::Landscape,
                }),
                fit_width: print.fit_width,
                fit_height: print.fit_height,
                margins: print.margins.map(|margins| tail::Margins {
                    top: margins.top,
                    right: margins.right,
                    bottom: margins.bottom,
                    left: margins.left,
                }),
                header: print.header,
                footer: print.footer,
                repeat_rows: print.repeat_rows,
                area: print.area,
            }),
            wit::SheetPart::Protect(protect) => Part::Protect(tail::Protect {
                password: protect.password,
                allow: protect.allow,
            }),
            wit::SheetPart::Image(image) => Part::Image(crate::drawing::Image {
                at: image.at,
                image: image.image,
                width: image.width,
                height: image.height,
                offset_x: image.offset_x,
                offset_y: image.offset_y,
            }),
            wit::SheetPart::Table(table) => Part::Table(crate::table::Table {
                range: table.table,
                name: table.name,
                style: table.style,
                banded: table.banded,
                filter: table.filter,
                totals: table
                    .totals
                    .into_iter()
                    .map(|total| crate::table::Total {
                        column: total.column,
                        function: total.function.map(|function| match function {
                            wit::TotalFunction::Sum => crate::table::Function::Sum,
                            wit::TotalFunction::Average => crate::table::Function::Average,
                            wit::TotalFunction::Count => crate::table::Function::Count,
                            wit::TotalFunction::Min => crate::table::Function::Min,
                            wit::TotalFunction::Max => crate::table::Function::Max,
                        }),
                        formula: total.formula,
                    })
                    .collect(),
            }),
            wit::SheetPart::Conditional(rule) => {
                Part::Conditional(crate::conditional::Conditional {
                    formatted: rule.formatted,
                    kind: match rule.kind {
                        wit::ConditionalKind::Cell => crate::conditional::Kind::Cell,
                        wit::ConditionalKind::Formula => crate::conditional::Kind::Formula,
                        wit::ConditionalKind::ColorScale => crate::conditional::Kind::ColorScale,
                        wit::ConditionalKind::DataBar => crate::conditional::Kind::DataBar,
                        wit::ConditionalKind::IconSet => crate::conditional::Kind::IconSet,
                        wit::ConditionalKind::Top => crate::conditional::Kind::Top,
                        wit::ConditionalKind::Bottom => crate::conditional::Kind::Bottom,
                        wit::ConditionalKind::Duplicates => crate::conditional::Kind::Duplicates,
                        wit::ConditionalKind::Unique => crate::conditional::Kind::Unique,
                    },
                    operator: rule.operator.map(operator),
                    values: rule.values,
                    colors: rule.colors,
                    icons: rule.icons,
                    rank: rule.rank,
                    percent: rule.percent,
                    style: rule.style.map(style),
                })
            }
            wit::SheetPart::Chart(chart) => Part::Chart(crate::chart::Chart {
                placed: chart.placed,
                layers: chart
                    .layers
                    .into_iter()
                    .map(|layer| crate::chart::Layer {
                        kind: match layer.kind {
                            wit::ChartKind::Column => crate::chart::Kind::Column,
                            wit::ChartKind::Bar => crate::chart::Kind::Bar,
                            wit::ChartKind::Line => crate::chart::Kind::Line,
                            wit::ChartKind::Area => crate::chart::Kind::Area,
                            wit::ChartKind::Pie => crate::chart::Kind::Pie,
                            wit::ChartKind::Doughnut => crate::chart::Kind::Doughnut,
                            wit::ChartKind::Scatter => crate::chart::Kind::Scatter,
                        },
                        series: layer
                            .series
                            .into_iter()
                            .map(|series| crate::chart::Series {
                                values: series.values,
                                categories: series.categories,
                                name: series.name,
                                color: series.color,
                            })
                            .collect(),
                    })
                    .collect(),
                title: chart.title,
                x_title: chart.x_title,
                y_title: chart.y_title,
                legend: match chart.legend {
                    wit::Legend::Hidden => crate::chart::Legend::Hidden,
                    wit::Legend::Top => crate::chart::Legend::Top,
                    wit::Legend::Right => crate::chart::Legend::Right,
                    wit::Legend::Bottom => crate::chart::Legend::Bottom,
                    wit::Legend::Left => crate::chart::Legend::Left,
                },
                labels: chart.labels,
                width: chart.width,
                height: chart.height,
            }),
        })
    }

    fn with<T>(
        writer: wit::WriterBorrow<'_>,
        call: impl FnOnce(&mut crate::Book) -> Result<T, crate::Error>,
    ) -> Result<T, wit::Error> {
        call(&mut writer.get::<Book>().writer.borrow_mut()).map_err(error)
    }

    impl Guest for Component {
        type Writer = Book;
        type Reading = Reader;
        type Filling = Filling;

        fn writer_open(options: wit::WorkbookOptions) -> Result<wit::Writer, wit::Error> {
            let properties = options.properties;
            let writer = crate::Book::open(crate::WorkbookOptions {
                properties: crate::Properties {
                    title: properties.title,
                    subject: properties.subject,
                    author: properties.author,
                    company: properties.company,
                    keywords: properties.keywords,
                    created: properties.created.map(clock),
                },
                target: options.target,
                ods: options.ods,
            })
            .map_err(error)?;
            Ok(wit::Writer::new(Book {
                writer: RefCell::new(writer),
            }))
        }

        fn writer_style(
            writer: wit::WriterBorrow<'_>,
            layer: wit::Style,
        ) -> Result<u64, wit::Error> {
            with(writer, |writer| writer.style(style(layer)))
        }

        fn writer_sheet(
            writer: wit::WriterBorrow<'_>,
            options: wit::SheetOptions,
            first: Vec<wit::Row>,
        ) -> Result<Vec<u8>, wit::Error> {
            let first = rows(first);
            with(writer, |writer| {
                writer.sheet(sheet_options(options), &first)
            })
        }

        fn writer_rows(
            writer: wit::WriterBorrow<'_>,
            batch: Vec<wit::Row>,
        ) -> Result<Vec<u8>, wit::Error> {
            let batch = rows(batch);
            with(writer, |writer| writer.rows(&batch))
        }

        fn writer_part(
            writer: wit::WriterBorrow<'_>,
            part: wit::SheetPart,
        ) -> Result<Vec<u8>, wit::Error> {
            let part = sheet_part(part)?;
            with(writer, |writer| writer.part(part))
        }

        fn writer_define(
            writer: wit::WriterBorrow<'_>,
            name: String,
            reference: String,
        ) -> Result<(), wit::Error> {
            with(writer, |writer| writer.define(&name, &reference))
        }

        fn writer_cell(
            writer: wit::WriterBorrow<'_>,
            column: u64,
            row: u64,
        ) -> Result<String, wit::Error> {
            let column = u32::try_from(column).unwrap_or(u32::MAX);
            let row = u32::try_from(row).unwrap_or(u32::MAX);
            with(writer, |writer| writer.cell(column, row))
        }

        fn writer_column(writer: wit::WriterBorrow<'_>, column: u64) -> Result<String, wit::Error> {
            let column = u32::try_from(column).unwrap_or(u32::MAX);
            with(writer, |writer| writer.column(column))
        }

        fn writer_finish(writer: wit::WriterBorrow<'_>) -> Result<wit::Finished, wit::Error> {
            with(writer, crate::Book::finish).map(finished)
        }

        fn reader_open(
            source: &Source,
            bound: wit::DecompressionBound,
        ) -> Result<wit::Reading, wit::Error> {
            let reader =
                crate::read::Reader::open(&Lent(source), bound_of(bound)).map_err(error)?;
            Ok(wit::Reading::new(Reader {
                reader: RefCell::new(reader),
            }))
        }

        fn reader_sheets(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
        ) -> Result<Vec<wit::SheetInfo>, wit::Error> {
            let sheets = reader
                .get::<Reader>()
                .reader
                .borrow()
                .sheets(&Lent(source))
                .map_err(error)?;
            Ok(sheets.into_iter().map(sheet_info).collect())
        }

        fn reader_header(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
            options: wit::ReadOptions,
        ) -> Result<Vec<String>, wit::Error> {
            reader
                .get::<Reader>()
                .reader
                .borrow_mut()
                .header(&Lent(source), &read_options(options))
                .map_err(error)
        }

        fn reader_rows(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
            options: wit::ReadOptions,
            max: u64,
        ) -> Result<wit::ReadBatch, wit::Error> {
            let batch = reader
                .get::<Reader>()
                .reader
                .borrow_mut()
                .rows(&Lent(source), &read_options(options), max)
                .map_err(error)?;
            Ok(wit::ReadBatch {
                rows: batch.rows.into_iter().map(read_row).collect(),
                done: batch.done,
            })
        }

        fn reader_merged(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
            sheet: String,
        ) -> Result<Vec<String>, wit::Error> {
            let ranges = reader
                .get::<Reader>()
                .reader
                .borrow()
                .merged(&Lent(source), &sheet)
                .map_err(error)?;
            Ok(ranges.iter().map(crate::read::merge::Merge::text).collect())
        }

        fn reader_links(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
            sheet: String,
        ) -> Result<Vec<wit::LinkInfo>, wit::Error> {
            let links = reader
                .get::<Reader>()
                .reader
                .borrow()
                .links(&Lent(source), &sheet)
                .map_err(error)?;
            Ok(links
                .into_iter()
                .map(|link| wit::LinkInfo {
                    anchor: link.anchor,
                    target: link.target,
                    text: link.text,
                    tooltip: link.tooltip,
                    external: link.external,
                })
                .collect())
        }

        fn reader_notes(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
            sheet: String,
        ) -> Result<Vec<wit::NoteInfo>, wit::Error> {
            let notes = reader
                .get::<Reader>()
                .reader
                .borrow()
                .notes(&Lent(source), &sheet)
                .map_err(error)?;
            Ok(notes
                .into_iter()
                .map(|note| wit::NoteInfo {
                    noted: note.noted,
                    note: note.note,
                    author: note.author,
                })
                .collect())
        }

        fn reader_names(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
        ) -> Result<Vec<wit::NameInfo>, wit::Error> {
            let names = reader
                .get::<Reader>()
                .reader
                .borrow()
                .names(&Lent(source))
                .map_err(error)?;
            Ok(names
                .into_iter()
                .map(|name| wit::NameInfo {
                    name: name.name,
                    reference: name.reference,
                    sheet: name.sheet,
                })
                .collect())
        }

        fn reader_tables(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
            sheet: String,
        ) -> Result<Vec<wit::TableInfo>, wit::Error> {
            let tables = reader
                .get::<Reader>()
                .reader
                .borrow()
                .tables(&Lent(source), &sheet)
                .map_err(error)?;
            Ok(tables
                .into_iter()
                .map(|table| wit::TableInfo {
                    table: table.table,
                    name: table.name,
                    sheet: table.sheet,
                })
                .collect())
        }

        fn reader_images(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
            sheet: String,
        ) -> Result<Vec<wit::ImageInfo>, wit::Error> {
            let images = reader
                .get::<Reader>()
                .reader
                .borrow()
                .images(&Lent(source), &sheet)
                .map_err(error)?;
            Ok(images
                .into_iter()
                .map(|image| wit::ImageInfo {
                    at: image.at,
                    image: image.image,
                    format: image.format,
                    link: image.link,
                })
                .collect())
        }

        fn reader_evaluate(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
        ) -> Result<Vec<wit::MissedInfo>, wit::Error> {
            let missed = reader
                .get::<Reader>()
                .reader
                .borrow_mut()
                .evaluate(&Lent(source))
                .map_err(error)?;
            Ok(missed
                .into_iter()
                .map(|missed| wit::MissedInfo {
                    cell: missed.cell,
                    kind: missed.kind.as_str().to_owned(),
                    text: missed.text,
                })
                .collect())
        }

        fn reader_properties(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
        ) -> Result<wit::Properties, wit::Error> {
            let properties = reader
                .get::<Reader>()
                .reader
                .borrow()
                .properties(&Lent(source))
                .map_err(error)?;
            Ok(wit::Properties {
                title: properties.title,
                subject: properties.subject,
                author: properties.author,
                company: properties.company,
                keywords: properties.keywords,
                created: properties.created.map(|clock| wit::ClockCell {
                    day: wit_date(clock.day),
                    clock: wit_time(clock.clock),
                }),
            })
        }

        fn reader_macros(
            reader: wit::ReadingBorrow<'_>,
            source: &Source,
        ) -> Result<Vec<String>, wit::Error> {
            reader
                .get::<Reader>()
                .reader
                .borrow()
                .macros(&Lent(source))
                .map_err(error)
        }

        fn source_bytes(
            source: &Source,
            offset: u64,
            length: Option<u64>,
        ) -> Result<Vec<u8>, wit::Error> {
            use crate::read::Source as _;
            let lent = Lent(source);
            let left = lent.size().map_err(error)?.saturating_sub(offset);
            lent.range(offset, length.map_or(left, |length| length.min(left)))
                .map_err(error)
        }

        fn template_open(
            source: &Source,
            bound: wit::DecompressionBound,
        ) -> Result<wit::Filling, wit::Error> {
            let template =
                crate::template::Template::open(&Lent(source), bound_of(bound)).map_err(error)?;
            Ok(wit::Filling::new(Filling {
                template: RefCell::new(template),
            }))
        }

        fn template_set(
            template: wit::FillingBorrow<'_>,
            address: String,
            cell: Option<wit::CellValue>,
        ) -> Result<(), wit::Error> {
            template
                .get::<Filling>()
                .template
                .borrow_mut()
                .set(&address, cell.map(value))
                .map_err(error)
        }

        fn template_fill(
            template: wit::FillingBorrow<'_>,
            values: Vec<wit::Placeholder>,
            lists: Vec<String>,
        ) -> Result<wit::Filled, wit::Error> {
            let (data, lists) = template
                .get::<Filling>()
                .template
                .borrow_mut()
                .fill(placeholders(values), &lists)
                .map_err(error)?;
            Ok(wit::Filled { data, lists })
        }

        fn template_items(
            template: wit::FillingBorrow<'_>,
            batch: wit::ItemBatch,
        ) -> Result<Vec<u8>, wit::Error> {
            let items = batch.items.into_iter().map(placeholders).collect();
            template
                .get::<Filling>()
                .template
                .borrow_mut()
                .items(&batch.name, items)
                .map_err(error)
        }

        fn template_finish(
            template: wit::FillingBorrow<'_>,
            source: &Source,
        ) -> Result<wit::Finished, wit::Error> {
            template
                .get::<Filling>()
                .template
                .borrow_mut()
                .finish(&Lent(source))
                .map(finished)
                .map_err(error)
        }
    }

    export!(Component);
}
