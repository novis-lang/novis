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
//! that reads back as the same `f64`. A formula is stored as parsed tokens, not as text, so a cell
//! read with `formulas: true` gives its stored result and no formula.

use std::sync::Arc;

use super::format::Style;
use super::merge::Merge;
use super::sheet::{self, RawCell, RawRow};
use super::zip::{Entry, Part};
use super::{MOST_COLUMNS, MOST_ROWS, Source, Strings};
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
const FORMAT: u16 = 44;
const STYLE: u16 = 47;
const CELL_RICH: u16 = 62;
const BEGIN_SHEET_DATA: u16 = 145;
const END_SHEET_DATA: u16 = 146;
const DIMENSION: u16 = 148;
const WORKBOOK_PROPERTIES: u16 = 153;
const SHEET: u16 = 156;
const MERGE_CELL: u16 = 176;
const BEGIN_CELL_STYLES: u16 = 617;
const END_CELL_STYLES: u16 = 618;

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
        .chunks_exact(2)
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
}

/// Reads `xl/workbook.bin`.
pub fn workbook(part: &[u8]) -> Result<Workbook, Error> {
    let mut book = Workbook {
        date1904: false,
        sheets: Vec::new(),
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
                book.sheets.push((name, visibility, id.unwrap_or_default()));
            }
            _ => {}
        }
    }
    Ok(book)
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
    let mut part = Part::open(source, entry)?;
    let mut buf = Vec::new();
    let mut at = 0;
    let mut ranges = Vec::new();
    loop {
        match record(&buf[at..])? {
            Some((kind, body, length)) => {
                if kind == MERGE_CELL {
                    let corner = |at| u32_at(body, at).map(|n| u64::from(n) + 1);
                    ranges.push(Merge::new(
                        (corner(0)?, corner(4)?),
                        (corner(8)?, corner(12)?),
                    )?);
                }
                at += length;
            }
            None => {
                buf.drain(..at);
                at = 0;
                if !part.fill(source, &mut buf)? {
                    if buf.is_empty() {
                        return Ok(ranges);
                    }
                    return Err(broken("a worksheet ends inside a record"));
                }
            }
        }
    }
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
}

impl Cursor {
    /// Starts reading the sheet whose part is `entry`.
    pub fn open(source: &dyn Source, entry: &Entry) -> Result<Self, Error> {
        Ok(Self {
            part: Part::open(source, entry)?,
            buf: Vec::new(),
            pos: 0,
            eof: false,
            in_data: false,
            over: false,
            last: 0,
            row: None,
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
                    let hidden = body.get(11).is_some_and(|flags| flags & 0x10 != 0);
                    done = self.row.replace(RawRow {
                        number,
                        hidden,
                        cells: Vec::new(),
                    });
                }
                CELL_BLANK..=FORMULA_ERROR | CELL_RICH if self.in_data => {
                    let cell = cell_of(kind, body)?;
                    let row = self
                        .row
                        .as_mut()
                        .ok_or_else(|| broken("a cell comes before its row"))?;
                    if cell.column > MOST_COLUMNS {
                        return Err(sheet::past_cell(cell.column, row.number));
                    }
                    row.cells.push(cell);
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
