//! xlsb, the binary workbook: the same zip package as xlsx, with each part a run of records
//! (MS-XLSB 2.1.4) in place of XML. The relationships parts are still XML, and `super` reads them.
//!
//! A record is its type, in one or two bytes of 7 bits each, its length, in up to four, and then
//! that many bytes. [`Cursor`] reads a sheet's part a block at a time, as `super::sheet::Cursor`
//! does, and gives each row as the [`RawRow`] the xlsx cursor gives, so `super::Reader::value`
//! reads both. A row ends where the next row's header starts or the sheet's data ends, so the
//! cursor holds the row being read until then. A row header past row 1,048,576 throws naming the
//! row, and a cell past column `XFD` naming the cell, with the errors the xlsx cursor throws.
//!
//! A cell's text is the kind `text`, which `super::Reader::value` reads as it is, because an
//! xlsb string has no XML escapes. A number is written into the raw cell as the shortest text
//! that reads back as the same `f64`.
//!
//! **Formulas.** A formula is stored as parsed tokens, which [`super::tokens`] turns into A1
//! text when a read asks for `formulas: true`, through the sheets, the `XTI` list and the names
//! [`workbook`] reads. Each cell of a shared formula holds a `ptgExp` naming the formula's first
//! row, and the `BrtShrFmla` with its tokens follows the first of those cells. So a cell that
//! comes before its shared formula waits for it until its row ends. The cursor keeps each shared
//! or array formula until it has passed its last row. An array formula's text is given in its
//! first cell, as an xlsx file writes it.

use std::sync::Arc;

use super::format::Style;
use super::merge::Merge;
use super::sheet::{self, RawCell, RawRow};
use super::tokens::{self, Biff, Linked, Place};
use super::zip::{Entry, Part};
use super::{MOST_COLUMNS, MOST_ROWS, Source, Strings};
use crate::cells::column_letters;
use crate::{Error, Visibility};

const ROW_HEADER: u16 = 0;
const CELL_BLANK: u16 = 1;
const CELL_RK: u16 = 2;
const CELL_ERROR: u16 = 3;
const CELL_BOOL: u16 = 4;
const CELL_REAL: u16 = 5;
const CELL_TEXT: u16 = 6;
const CELL_SHARED: u16 = 7;
const FORMULA_TEXT: u16 = 8;
const FORMULA_NUMBER: u16 = 9;
const FORMULA_BOOL: u16 = 10;
const FORMULA_ERROR: u16 = 11;
const SHARED_ITEM: u16 = 19;
const NAME: u16 = 39;
const FORMAT: u16 = 44;
const STYLE: u16 = 47;
const CELL_RICH: u16 = 62;
const BEGIN_SHEET_DATA: u16 = 145;
const END_SHEET_DATA: u16 = 146;
const DIMENSION: u16 = 148;
const WORKBOOK_PROPERTIES: u16 = 153;
const SHEET: u16 = 156;
const MERGE_CELL: u16 = 176;
const SUP_BOOK_SOURCE: u16 = 355;
const SUP_SELF: u16 = 357;
const SUP_SAME: u16 = 358;
const SUP_TABS: u16 = 359;
const EXTERN_SHEET: u16 = 362;
const ARRAY_FORMULA: u16 = 426;
const SHARED_FORMULA: u16 = 427;
const BEGIN_CELL_STYLES: u16 = 617;
const END_CELL_STYLES: u16 = 618;
const SUP_ADDIN: u16 = 667;

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file is not a valid workbook: {what}."))
}

/// A record's type, its body and its whole length.
type Record<'a> = (u16, &'a [u8], usize);

/// The record that starts `buf`, or `None` when `buf` ends inside it.
fn record(buf: &[u8]) -> Result<Option<Record<'_>>, Error> {
    let mut at = 0;
    let mut kind = 0u16;
    for group in 0..2 {
        let Some(&byte) = buf.get(at) else {
            return Ok(None);
        };
        at += 1;
        kind |= u16::from(byte & 0x7F) << (7 * group);
        if byte & 0x80 == 0 {
            break;
        }
        if group == 1 {
            return Err(broken("a record's type is longer than two bytes"));
        }
    }
    let mut length = 0usize;
    for group in 0..4 {
        let Some(&byte) = buf.get(at) else {
            return Ok(None);
        };
        at += 1;
        length |= usize::from(byte & 0x7F) << (7 * group);
        if byte & 0x80 == 0 {
            break;
        }
        if group == 3 {
            return Err(broken("a record's length is longer than four bytes"));
        }
    }
    Ok(buf
        .get(at..at + length)
        .map(|body| (kind, body, at + length)))
}

/// Every record of a part held whole, in order.
fn records(part: &[u8]) -> impl Iterator<Item = Result<(u16, &[u8]), Error>> {
    let mut at = 0;
    std::iter::from_fn(move || {
        if at == part.len() {
            return None;
        }
        Some(match record(&part[at..]) {
            Ok(Some((kind, body, length))) => {
                at += length;
                Ok((kind, body))
            }
            Ok(None) => {
                at = part.len();
                Err(broken("a part ends inside a record"))
            }
            Err(err) => {
                at = part.len();
                Err(err)
            }
        })
    })
}

pub(super) fn u32_at(body: &[u8], at: usize) -> Result<u32, Error> {
    body.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| broken("a record is shorter than its fields"))
}

pub(super) fn u16_at(body: &[u8], at: usize) -> Result<u16, Error> {
    body.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| broken("a record is shorter than its fields"))
}

pub(super) fn u8_at(body: &[u8], at: usize) -> Result<u8, Error> {
    body.get(at)
        .copied()
        .ok_or_else(|| broken("a record is shorter than its fields"))
}

pub(super) fn f64_at(body: &[u8], at: usize) -> Result<f64, Error> {
    body.get(at..at + 8)
        .map(|b| f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
        .ok_or_else(|| broken("a record is shorter than its fields"))
}

/// The string at `at`: a count of UTF-16 units and the units. Returns `None` for the count
/// `0xFFFFFFFF`, which a nullable string uses for no string, and the bytes the string takes.
fn wide_at(body: &[u8], at: usize) -> Result<(Option<String>, usize), Error> {
    let count = u32_at(body, at)?;
    if count == u32::MAX {
        return Ok((None, 4));
    }
    let bytes = usize::try_from(count)
        .ok()
        .and_then(|count| count.checked_mul(2))
        .and_then(|length| body.get(at + 4..at + 4 + length))
        .ok_or_else(|| broken("a string is longer than its record"))?;
    let units = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|unit| u16::from_le_bytes([unit[0], unit[1]]));
    Ok((
        Some(
            char::decode_utf16(units)
                .map(|c| c.unwrap_or('\u{FFFD}'))
                .collect(),
        ),
        4 + bytes.len(),
    ))
}

/// The text Excel shows for an error code.
pub(super) fn error_text(code: u8) -> Option<&'static str> {
    Some(match code {
        0x00 => "#NULL!",
        0x07 => "#DIV/0!",
        0x0F => "#VALUE!",
        0x17 => "#REF!",
        0x1D => "#NAME?",
        0x24 => "#NUM!",
        0x2A => "#N/A",
        0x2B => "#GETTING_DATA",
        _ => return None,
    })
}

/// An RK number: a 30-bit whole number or the top 30 bits of an `f64`, either divided by 100
/// when its lowest bit is set.
pub(super) fn rk(value: u32) -> f64 {
    #[allow(
        clippy::cast_possible_wrap,
        reason = "the RK bits are a signed whole number"
    )]
    let number = if value & 2 == 0 {
        f64::from_bits(u64::from(value & !3) << 32)
    } else {
        f64::from((value as i32) >> 2)
    };
    if value & 1 == 0 {
        number
    } else {
        number / 100.0
    }
}

/// A workbook part as `super::Reader::open` needs it.
pub struct Workbook {
    pub date1904: bool,
    /// Each sheet's name, visibility and relationship id, in tab order.
    pub sheets: Vec<(String, Visibility, String)>,
    /// What the workbook's formulas refer to by number.
    pub book: tokens::Book,
    /// Each other workbook: its index in `book.linked`, and the relationship id of the external
    /// link part that lists its sheets.
    pub externals: Vec<(usize, String)>,
}

/// Reads `xl/workbook.bin`.
pub fn workbook(part: &[u8]) -> Result<Workbook, Error> {
    let mut book = Workbook {
        date1904: false,
        sheets: Vec::new(),
        book: tokens::Book::default(),
        externals: Vec::new(),
    };
    for item in records(part) {
        let (kind, body) = item?;
        match kind {
            WORKBOOK_PROPERTIES => book.date1904 = u32_at(body, 0)? & 1 == 1,
            SHEET => {
                let visibility = match u32_at(body, 0)? {
                    1 => Visibility::Hidden,
                    2 => Visibility::VeryHidden,
                    _ => Visibility::Visible,
                };
                let (id, used) = wide_at(body, 8)?;
                let (name, _) = wide_at(body, 8 + used)?;
                let name = name.ok_or_else(|| broken("a sheet has no name"))?;
                book.book.sheets.push(name.clone());
                book.sheets.push((name, visibility, id.unwrap_or_default()));
            }
            SUP_SELF | SUP_SAME => book.book.linked.push(Linked::Own),
            SUP_ADDIN => book.book.linked.push(Linked::AddIn(Vec::new())),
            SUP_BOOK_SOURCE => {
                if let (Some(id), _) = wide_at(body, 0)? {
                    book.externals.push((book.book.linked.len(), id));
                }
                book.book.linked.push(Linked::other(&book.book.linked));
            }
            // A count, then each `XTI`: its `SupBook` and its first and last sheet.
            EXTERN_SHEET => {
                let count = u32_at(body, 0)? as usize;
                for xti in body
                    .get(4..)
                    .unwrap_or_default()
                    .as_chunks::<12>()
                    .0
                    .iter()
                    .take(count)
                {
                    let at = |n: usize| [xti[n], xti[n + 1], xti[n + 2], xti[n + 3]];
                    book.book.xti.push((
                        u32::from_le_bytes(at(0)),
                        i32::from_le_bytes(at(4)),
                        i32::from_le_bytes(at(8)),
                    ));
                }
            }
            // The flags, the shortcut key and the sheet, and then the name. A name that does not
            // read is kept empty, so the names after it keep their numbers.
            NAME => {
                let name = wide_at(body, 9).ok().and_then(|(name, _)| name);
                book.book.names.push(name.unwrap_or_default());
            }
            _ => {}
        }
    }
    Ok(book)
}

/// The sheet names of an external link part, from its `BrtSupTabs`: a count and each name.
pub fn external_sheets(part: &[u8]) -> Result<Vec<String>, Error> {
    for item in records(part) {
        let (kind, body) = item?;
        if kind == SUP_TABS {
            let mut sheets = Vec::new();
            let mut at = 4;
            for _ in 0..u32_at(body, 0)? {
                let (name, used) = wide_at(body, at)?;
                sheets.push(name.unwrap_or_default());
                at += used;
            }
            return Ok(sheets);
        }
    }
    Ok(Vec::new())
}

/// The number format of each cell style index in `xl/styles.bin`.
pub fn styles(part: &[u8]) -> Result<Vec<Style>, Error> {
    let mut codes: Vec<(u16, Arc<str>)> = Vec::new();
    let mut styles = Vec::new();
    let mut in_cell_styles = false;
    for item in records(part) {
        let (kind, body) = item?;
        match kind {
            FORMAT => {
                let id = u16_at(body, 0)?;
                if let (Some(code), _) = wide_at(body, 2)? {
                    codes.push((id, Arc::from(code)));
                }
            }
            BEGIN_CELL_STYLES => in_cell_styles = true,
            END_CELL_STYLES => in_cell_styles = false,
            STYLE if in_cell_styles => {
                let id = u16_at(body, 2)?;
                let custom = codes.iter().find(|(code, _)| *code == id);
                styles.push(Style::of(u32::from(id), custom.map(|(_, code)| code)));
            }
            _ => {}
        }
    }
    Ok(styles)
}

/// Reads `xl/sharedStrings.bin` a block at a time, so only the arena is held.
pub fn strings(source: &dyn Source, entry: &Entry) -> Result<Strings, Error> {
    let mut part = Part::open(source, entry)?;
    let mut buf = Vec::new();
    let mut at = 0;
    let mut strings = Strings::default();
    loop {
        match record(&buf[at..])? {
            Some((kind, body, length)) => {
                if kind == SHARED_ITEM {
                    let (text, _) = wide_at(body, 1)?;
                    strings.push_text(&text.unwrap_or_default())?;
                }
                at += length;
            }
            None => {
                buf.drain(..at);
                at = 0;
                if !part.fill(source, &mut buf)? {
                    if buf.is_empty() {
                        return Ok(strings);
                    }
                    return Err(broken("the shared strings end inside a record"));
                }
            }
        }
    }
}

/// The merged ranges of the sheet whose part is `entry`, read a block at a time to the part's end.
/// Each is a `BrtMergeCell` record: its first and last row and its first and last column, each
/// from 0.
pub fn merged(source: &dyn Source, entry: &Entry) -> Result<Vec<Merge>, Error> {
    let mut ranges = Vec::new();
    scan(source, entry, |kind, body| {
        if kind == MERGE_CELL {
            let corner = |at| u32_at(body, at).map(|n| u64::from(n) + 1);
            ranges.push(Merge::new(
                (corner(0)?, corner(4)?),
                (corner(8)?, corner(12)?),
            )?);
        }
        Ok(())
    })?;
    Ok(ranges)
}

/// Calls `each` with every record of the sheet part `entry`, read a block at a time to its end.
pub fn scan(
    source: &dyn Source,
    entry: &Entry,
    mut each: impl FnMut(u16, &[u8]) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut part = Part::open(source, entry)?;
    let mut buf = Vec::new();
    let mut at = 0;
    loop {
        match record(&buf[at..])? {
            Some((kind, body, length)) => {
                each(kind, body)?;
                at += length;
            }
            None => {
                buf.drain(..at);
                at = 0;
                if !part.fill(source, &mut buf)? {
                    if buf.is_empty() {
                        return Ok(());
                    }
                    return Err(broken("a worksheet ends inside a record"));
                }
            }
        }
    }
}

/// The cell or range of the `RfX` at `at`: its first and last row and its first and last
/// column, each from 0. One cell is `A1`, and more are `A1:B2`.
fn range_at(body: &[u8], at: usize) -> Result<String, Error> {
    let cell = |row: u32, column: u32| {
        let mut out = String::new();
        column_letters(&mut out, column);
        out.push_str(itoa::Buffer::new().format(u64::from(row) + 1));
        out
    };
    let (first, last) = (u32_at(body, at)?, u32_at(body, at + 4)?);
    let (left, right) = (u32_at(body, at + 8)?, u32_at(body, at + 12)?);
    let mut out = cell(first, left);
    if (first, left) != (last, right) {
        out.push(':');
        out.push_str(&cell(last, right));
    }
    Ok(out)
}

/// A string at `at` that is `None` when it is missing or empty, and the bytes it takes.
fn some_at(body: &[u8], at: usize) -> Result<(Option<String>, usize), Error> {
    let (text, used) = wide_at(body, at)?;
    Ok((text.filter(|text| !text.is_empty()), used))
}

/// A sheet's `BrtHLink`, as [`hyperlink`] reads it.
pub const HYPERLINK: u16 = 494;

/// One hyperlink of a sheet part: its cell or range, the relationship id of its target, the
/// place in the workbook it goes to, its tooltip and its text.
pub struct Hyperlink {
    pub anchor: String,
    pub rel: Option<String>,
    pub location: Option<String>,
    pub tooltip: Option<String>,
    pub display: Option<String>,
}

/// A `BrtHLink` record's body: its `RfX`, and then its relationship id, location, tooltip and
/// display text.
pub fn hyperlink(body: &[u8]) -> Result<Hyperlink, Error> {
    let anchor = range_at(body, 0)?;
    let (rel, used) = some_at(body, 16)?;
    let mut at = 16 + used;
    let (location, used) = some_at(body, at)?;
    at += used;
    let (tooltip, used) = some_at(body, at)?;
    at += used;
    let (display, _) = some_at(body, at)?;
    Ok(Hyperlink {
        anchor,
        rel,
        location,
        tooltip,
        display,
    })
}

const COMMENT_AUTHOR: u16 = 632;
const BEGIN_COMMENT: u16 = 635;
const END_COMMENT: u16 = 636;
const COMMENT_TEXT: u16 = 637;

/// The notes of a comments part, `xl/comments1.bin`: each one's cell, text and author, in order.
/// A `BrtCommentAuthor` is one name of the authors list, and a `BrtBeginComment` names its
/// author by index and its cell. Its `BrtCommentText` is a rich string, a byte of flags and then
/// the text, whose runs are not read.
pub fn comments(part: &[u8]) -> Result<Vec<(String, String, Option<String>)>, Error> {
    let mut authors = Vec::new();
    let mut notes = Vec::new();
    let mut open: Option<(String, Option<usize>, String)> = None;
    for item in records(part) {
        let (kind, body) = item?;
        match kind {
            COMMENT_AUTHOR => authors.push(wide_at(body, 0)?.0.unwrap_or_default()),
            BEGIN_COMMENT => {
                let by = usize::try_from(u32_at(body, 0)?).ok();
                let noted = range_at(body, 4)?;
                let noted = noted.split(':').next().unwrap_or_default().to_owned();
                open = Some((noted, by, String::new()));
            }
            COMMENT_TEXT => {
                if let Some((_, _, text)) = open.as_mut() {
                    *text = wide_at(body, 1)?.0.unwrap_or_default();
                }
            }
            END_COMMENT => {
                if let Some((noted, by, text)) = open.take() {
                    let author = by
                        .and_then(|by| authors.get(by).cloned())
                        .filter(|author: &String| !author.is_empty());
                    notes.push((noted, text, author));
                }
            }
            _ => {}
        }
    }
    Ok(notes)
}

/// A defined name: its name, the index of the sheet it is local to, and its formula's text.
pub type DefinedName = (String, Option<usize>, Option<String>);

/// The defined names of `xl/workbook.bin`, each its name, the index of the sheet it is local to
/// and its formula's text. A `BrtName` is its flags, a shortcut key, the sheet's index or
/// `0xFFFFFFFF` for the workbook, its name and its parsed formula. A formula the decoder cannot
/// read has no text.
pub fn names(part: &[u8], book: &tokens::Book) -> Result<Vec<DefinedName>, Error> {
    let mut names = Vec::new();
    for item in records(part) {
        let (kind, body) = item?;
        if kind != NAME {
            continue;
        }
        let sheet = u32_at(body, 5)?;
        let (name, used) = wide_at(body, 9)?;
        let reference = parsed_at(body, 9 + used).and_then(|(tokens, extra)| {
            let place = Place {
                base: (0, 0),
                shared: true,
            };
            tokens::decode(Biff::Twelve, tokens, extra, place, book)
        });
        let sheet = (sheet != u32::MAX)
            .then(|| usize::try_from(sheet).ok())
            .flatten();
        names.push((name.unwrap_or_default(), sheet, reference));
    }
    Ok(names)
}

const BEGIN_LIST: u16 = 343;

/// The range, the name, and whether it has a header and a totals row, of the table part `part`,
/// `xl/tables/table1.bin`, from its
/// `BrtBeginList`: its `RfX`, twelve numbers of four bytes, and its name and its display name.
/// The display name is the one a formula uses, and the name is used when it is missing.
pub fn table(part: &[u8]) -> Result<Option<(String, String, bool, bool)>, Error> {
    for item in records(part) {
        let (kind, body) = item?;
        if kind == BEGIN_LIST {
            let range = range_at(body, 0)?;
            let (name, used) = some_at(body, 64)?;
            let (shown, _) = some_at(body, 64 + used)?;
            // `crwHeader` and `crwTotals`, after the range, the type and the id.
            let header = u32_at(body, 24)? != 0;
            let totals = u32_at(body, 28)? != 0;
            return Ok(Some((
                range,
                shown.or(name).unwrap_or_default(),
                header,
                totals,
            )));
        }
    }
    Ok(None)
}

/// A cell record's cell: its column from 1 and its style index, with no value yet.
fn cell(body: &[u8]) -> Result<RawCell, Error> {
    Ok(RawCell {
        column: u64::from(u32_at(body, 0)?) + 1,
        style: u32_at(body, 4)? & 0x00FF_FFFF,
        ..RawCell::default()
    })
}

pub(super) fn number(mut cell: RawCell, value: f64) -> RawCell {
    cell.value = Some(ryu::Buffer::new().format(value).to_owned());
    cell
}

pub(super) fn typed(mut cell: RawCell, kind: &[u8], value: Option<String>) -> RawCell {
    cell.kind = kind.to_vec();
    cell.value = value;
    cell
}

/// A parsed formula at `at` of `body`: its tokens and its extra data, each after a count of four
/// bytes. A record that ends before the extra data has none. `None` when the tokens are longer
/// than the record.
fn parsed_at(body: &[u8], at: usize) -> Option<(&[u8], &[u8])> {
    let count = u32_at(body, at).ok()? as usize;
    let tokens = body.get(at + 4..)?.get(..count)?;
    let after = at + 4 + count;
    let extra = match u32_at(body, after) {
        Ok(length) => body.get(after + 4..)?.get(..length as usize)?,
        Err(_) => &[],
    };
    Some((tokens, extra))
}

/// The tokens and extra data of the formula cell record `kind`, after its cell, its value and
/// its two bytes of flags. `None` for a cell that is not a formula.
fn formula_of(kind: u16, body: &[u8]) -> Option<(&[u8], &[u8])> {
    let value = match kind {
        FORMULA_TEXT => wide_at(body, 8).ok()?.1,
        FORMULA_NUMBER => 8,
        FORMULA_BOOL | FORMULA_ERROR => 1,
        _ => return None,
    };
    parsed_at(body, 8 + value + 2)
}

/// A shared or array formula the cursor keeps until it passes its last row.
#[derive(Debug)]
struct Shared {
    /// The first and last row, and the first and last column, each from 0.
    rows: (u32, u32),
    columns: (u32, u32),
    tokens: Vec<u8>,
    extra: Vec<u8>,
    array: bool,
}

impl Shared {
    fn covers(&self, (row, column): (u32, u32)) -> bool {
        (self.rows.0..=self.rows.1).contains(&row)
            && (self.columns.0..=self.columns.1).contains(&column)
    }

    /// The formula's text for the cell `at`. An array formula has text in its first cell only.
    fn text(&self, at: (u32, u32), book: &tokens::Book) -> Option<String> {
        if !self.covers(at) || (self.array && at != (self.rows.0, self.columns.0)) {
            return None;
        }
        let place = Place {
            base: at,
            shared: !self.array,
        };
        tokens::decode(Biff::Twelve, &self.tokens, &self.extra, place, book)
    }
}

/// The cell `cell` of the row `number`, from 1, as the zero-based pair a formula counts from.
fn place_of(number: u64, cell: &RawCell) -> (u32, u32) {
    let from_zero = |n: u64| u32::try_from(n.saturating_sub(1)).unwrap_or(u32::MAX);
    (from_zero(number), from_zero(cell.column))
}

/// The cell record `kind` with `body`, as the raw cell the xlsx cursor would give.
fn cell_of(kind: u16, body: &[u8]) -> Result<RawCell, Error> {
    let at = cell(body)?;
    Ok(match kind {
        CELL_RK => number(at, rk(u32_at(body, 8)?)),
        CELL_REAL | FORMULA_NUMBER => number(at, f64_at(body, 8)?),
        CELL_BOOL | FORMULA_BOOL => {
            let value = if u8_at(body, 8)? == 0 { "0" } else { "1" };
            typed(at, b"b", Some(value.to_owned()))
        }
        CELL_ERROR | FORMULA_ERROR => {
            let text = error_text(u8_at(body, 8)?).map(str::to_owned);
            typed(at, b"e", text)
        }
        CELL_TEXT | FORMULA_TEXT => typed(at, b"text", wide_at(body, 8)?.0),
        CELL_RICH => typed(at, b"text", wide_at(body, 9)?.0),
        CELL_SHARED => typed(at, b"s", Some(u32_at(body, 8)?.to_string())),
        _ => at,
    })
}

/// A worksheet's part being read as a stream of rows.
#[derive(Debug)]
pub struct Cursor {
    part: Part,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    in_data: bool,
    over: bool,
    last: u64,
    row: Option<RawRow>,
    /// What formulas refer to, when the read gives each formula's text.
    formulas: Option<Arc<tokens::Book>>,
    shared: Vec<Shared>,
    /// The columns of the row being read whose cell waits for the shared formula after it.
    waiting: Vec<u64>,
}

impl Cursor {
    /// Starts reading the sheet whose part is `entry`. With `formulas`, each formula cell gets
    /// its text.
    pub fn open(
        source: &dyn Source,
        entry: &Entry,
        formulas: Option<Arc<tokens::Book>>,
    ) -> Result<Self, Error> {
        Ok(Self {
            part: Part::open(source, entry)?,
            buf: Vec::new(),
            pos: 0,
            eof: false,
            in_data: false,
            over: false,
            last: 0,
            row: None,
            formulas,
            shared: Vec::new(),
            waiting: Vec::new(),
        })
    }

    /// Reads more of the part into the buffer, first dropping what was read. Returns `false` at
    /// the end of the part.
    fn more(&mut self, source: &dyn Source) -> Result<bool, Error> {
        if self.eof {
            if self.pos == self.buf.len() {
                return Ok(false);
            }
            return Err(broken("a worksheet ends inside a record"));
        }
        self.buf.drain(..self.pos);
        self.pos = 0;
        let before = self.buf.len();
        while self.buf.len() == before {
            if !self.part.fill(source, &mut self.buf)? {
                self.eof = true;
                break;
            }
        }
        Ok(true)
    }

    /// The rows and columns the sheet's dimension record declares, read before its first row,
    /// or `None` when the sheet has none.
    pub fn dimension(&mut self, source: &dyn Source) -> Result<Option<(u64, u64)>, Error> {
        loop {
            match record(&self.buf[self.pos..])? {
                None => {
                    if !self.more(source)? {
                        return Ok(None);
                    }
                }
                Some((DIMENSION, body, _)) => {
                    let rows = u64::from(u32_at(body, 4)?) + 1;
                    let columns = u64::from(u32_at(body, 12)?) + 1;
                    return Ok(Some((rows, columns)));
                }
                Some((BEGIN_SHEET_DATA, ..)) => return Ok(None),
                Some((_, _, length)) => self.pos += length,
            }
        }
    }

    /// The next row of the sheet, or `None` after its last.
    pub fn next(&mut self, source: &dyn Source) -> Result<Option<RawRow>, Error> {
        loop {
            if self.over {
                return Ok(self.row.take());
            }
            let Some((kind, body, length)) = record(&self.buf[self.pos..])? else {
                if !self.more(source)? {
                    self.over = true;
                }
                continue;
            };
            let mut done = None;
            match kind {
                BEGIN_SHEET_DATA => self.in_data = true,
                END_SHEET_DATA => self.over = true,
                ROW_HEADER if self.in_data => {
                    let number = u64::from(u32_at(body, 0)?) + 1;
                    if number <= self.last {
                        return Err(broken("a worksheet's rows are out of order"));
                    }
                    if number > MOST_ROWS {
                        return Err(sheet::past_row(number));
                    }
                    self.last = number;
                    self.waiting.clear();
                    self.shared
                        .retain(|shared| u64::from(shared.rows.1) + 1 >= number);
                    let hidden = body.get(11).is_some_and(|flags| flags & 0x10 != 0);
                    done = self.row.replace(RawRow {
                        number,
                        hidden,
                        cells: Vec::new(),
                    });
                }
                CELL_BLANK..=FORMULA_ERROR | CELL_RICH if self.in_data => {
                    let mut cell = cell_of(kind, body)?;
                    let row = self
                        .row
                        .as_mut()
                        .ok_or_else(|| broken("a cell comes before its row"))?;
                    if cell.column > MOST_COLUMNS {
                        return Err(sheet::past_cell(cell.column, row.number));
                    }
                    if let (Some(book), Some((tokens, extra))) =
                        (&self.formulas, formula_of(kind, body))
                    {
                        let at = place_of(row.number, &cell);
                        match tokens::shared_at(Biff::Twelve, tokens) {
                            Some((first, _)) => match self
                                .shared
                                .iter()
                                .find(|shared| shared.rows.0 == first && shared.covers(at))
                            {
                                Some(shared) => cell.formula = shared.text(at, book),
                                None => self.waiting.push(cell.column),
                            },
                            None => {
                                let place = Place {
                                    base: at,
                                    shared: false,
                                };
                                cell.formula = Some(
                                    tokens::decode(Biff::Twelve, tokens, extra, place, book)
                                        .unwrap_or_default(),
                                );
                            }
                        }
                    }
                    row.cells.push(cell);
                }
                // The range, then for an array formula one byte of flags, then the formula.
                SHARED_FORMULA | ARRAY_FORMULA if self.in_data && self.formulas.is_some() => {
                    let array = kind == ARRAY_FORMULA;
                    if let Some((tokens, extra)) = parsed_at(body, if array { 17 } else { 16 }) {
                        let shared = Shared {
                            rows: (u32_at(body, 0)?, u32_at(body, 4)?),
                            columns: (u32_at(body, 8)?, u32_at(body, 12)?),
                            tokens: tokens.to_vec(),
                            extra: extra.to_vec(),
                            array,
                        };
                        if let (Some(row), Some(book)) = (self.row.as_mut(), &self.formulas) {
                            self.waiting.retain(|&column| {
                                let Some(cell) = row.cells.iter_mut().find(|c| c.column == column)
                                else {
                                    return false;
                                };
                                let at = place_of(row.number, cell);
                                if !shared.covers(at) {
                                    return true;
                                }
                                cell.formula = shared.text(at, book);
                                false
                            });
                        }
                        self.shared.push(shared);
                    }
                }
                _ => {}
            }
            self.pos += length;
            if done.is_some() {
                return Ok(done);
            }
        }
    }
}
