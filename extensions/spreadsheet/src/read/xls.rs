//! xls, the binary workbook of Excel 97 to 2003: a compound file ([`super::cfb`]) whose
//! `Workbook` stream is a run of BIFF8 records, each a type, a length and that many bytes.
//!
//! **An xls is read whole.** [`read`] reads the workbook globals — the date system, the number
//! formats, the cell styles, the sheets and the shared strings — and then every sheet's cells into
//! the [`RawRow`]s the xlsx cursor gives, so `super::Reader::value` reads every format the same
//! way. A sheet holds at most 65,536 rows. A text cell is the kind `text`, a number has its cell
//! style's index, and a formula gives its stored result, because its formula is stored as parsed
//! tokens. The streams of an older Excel, BIFF5 and before, are [`Error::Runtime`].
//!
//! A record longer than 8,224 bytes goes on in `CONTINUE` records. The shared strings are the one
//! table this reader needs that crosses them, and [`Continued`] reads across the seams, where a
//! string's characters start again with a byte that says whether they are one byte or two.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::binary::{error_text, f64_at, number, rk, typed, u8_at, u16_at, u32_at};
use super::format::{self, Kind};
use super::sheet::{RawCell, RawRow};
use super::{Listed, SheetKind, cfb};
use crate::{Error, Visibility};

const FORMULA: u16 = 0x0006;
const EOF: u16 = 0x000A;
const DATE_MODE: u16 = 0x0022;
const CONTINUE: u16 = 0x003C;
const SHEET: u16 = 0x0085;
const MULTIPLE_RK: u16 = 0x00BD;
const RICH_LABEL: u16 = 0x00D6;
const STYLE: u16 = 0x00E0;
const SHARED_STRINGS: u16 = 0x00FC;
const SHARED_LABEL: u16 = 0x00FD;
const NUMBER: u16 = 0x0203;
const LABEL: u16 = 0x0204;
const BOOL_ERROR: u16 = 0x0205;
const STRING: u16 = 0x0207;
const ROW: u16 = 0x0208;
const RK: u16 = 0x027E;
const FORMAT: u16 = 0x041E;
const BOF: u16 = 0x0809;
const BIFF8: u16 = 0x0600;
const MOST_ROWS: u32 = 65_536;
const MOST_COLUMNS: u16 = 256;

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file is not a valid xls file: {what}."))
}

/// The records of `stream` from `at`: each one's type, its body and the offset after it.
fn records(stream: &[u8], mut at: usize) -> impl Iterator<Item = Result<(u16, &[u8]), Error>> {
    std::iter::from_fn(move || {
        if at + 4 > stream.len() {
            return None;
        }
        let kind = u16::from_le_bytes([stream[at], stream[at + 1]]);
        let length = usize::from(u16::from_le_bytes([stream[at + 2], stream[at + 3]]));
        let Some(body) = stream.get(at + 4..at + 4 + length) else {
            at = stream.len();
            return Some(Err(broken("a record is longer than the stream")));
        };
        at += 4 + length;
        Some(Ok((kind, body)))
    })
}

/// The characters `bytes` are: one byte each, or two when `wide`.
fn characters(bytes: &[u8], wide: bool) -> String {
    if wide {
        let units = bytes
            .chunks_exact(2)
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]));
        char::decode_utf16(units)
            .map(|c| c.unwrap_or('\u{FFFD}'))
            .collect()
    } else {
        bytes.iter().map(|&b| char::from(b)).collect()
    }
}

/// A string inside one record at `at`, its count `count_bytes` wide: the count, the flags and the
/// characters, and the bytes it takes.
fn string_at(body: &[u8], at: usize, count_bytes: usize) -> Result<(String, usize), Error> {
    let count = if count_bytes == 1 {
        usize::from(u8_at(body, at)?)
    } else {
        usize::from(u16_at(body, at)?)
    };
    let flags = u8_at(body, at + count_bytes)?;
    let wide = flags & 1 != 0;
    let mut start = at + count_bytes + 1;
    let (mut runs, mut extra) = (0, 0);
    if flags & 0x08 != 0 {
        runs = usize::from(u16_at(body, start)?) * 4;
        start += 2;
    }
    if flags & 0x04 != 0 {
        extra = u32_at(body, start)? as usize;
        start += 4;
    }
    let length = count * if wide { 2 } else { 1 };
    let bytes = body
        .get(start..start + length)
        .ok_or_else(|| broken("a string is longer than its record"))?;
    Ok((characters(bytes, wide), start + length + runs + extra - at))
}

/// The shared strings record and its `CONTINUE` records, read as one run of bytes.
struct Continued<'a> {
    parts: Vec<&'a [u8]>,
    part: usize,
    at: usize,
}

impl Continued<'_> {
    fn take(&mut self, mut length: usize, out: &mut Vec<u8>) -> Result<(), Error> {
        while length > 0 {
            let part = self
                .parts
                .get(self.part)
                .ok_or_else(|| broken("the shared strings end early"))?;
            if self.at == part.len() {
                self.part += 1;
                self.at = 0;
                continue;
            }
            let step = length.min(part.len() - self.at);
            out.extend_from_slice(&part[self.at..self.at + step]);
            self.at += step;
            length -= step;
        }
        Ok(())
    }

    fn skip(&mut self, length: usize) -> Result<(), Error> {
        let mut scratch = Vec::new();
        self.take(length, &mut scratch)
    }

    fn uint(&mut self, bytes: usize) -> Result<u32, Error> {
        let mut out = Vec::with_capacity(4);
        self.take(bytes, &mut out)?;
        out.resize(4, 0);
        Ok(u32::from_le_bytes([out[0], out[1], out[2], out[3]]))
    }

    /// One string. Where its characters cross into the next record, they go on after a flags
    /// byte of their own.
    fn string(&mut self) -> Result<String, Error> {
        let count = self.uint(2)? as usize;
        let flags = self.uint(1)?;
        let mut wide = flags & 1 != 0;
        let runs = if flags & 0x08 != 0 {
            self.uint(2)? as usize * 4
        } else {
            0
        };
        let extra = if flags & 0x04 != 0 {
            self.uint(4)? as usize
        } else {
            0
        };
        let mut text = String::new();
        let mut left = count;
        while left > 0 {
            let part = self
                .parts
                .get(self.part)
                .ok_or_else(|| broken("the shared strings end early"))?;
            if self.at == part.len() {
                self.part += 1;
                self.at = 0;
                wide = self.uint(1)? & 1 != 0;
                continue;
            }
            let width = if wide { 2 } else { 1 };
            let fits = ((part.len() - self.at) / width).min(left);
            if fits == 0 {
                return Err(broken("a character is split across two records"));
            }
            let mut bytes = Vec::with_capacity(fits * width);
            self.take(fits * width, &mut bytes)?;
            text.push_str(&characters(&bytes, wide));
            left -= fits;
        }
        self.skip(runs + extra)?;
        Ok(text)
    }
}

/// The shared strings, from the record's body and its `CONTINUE` records.
fn shared(parts: Vec<&[u8]>) -> Result<Vec<String>, Error> {
    let mut run = Continued {
        parts,
        part: 0,
        at: 0,
    };
    run.skip(4)?;
    let unique = run.uint(4)? as usize;
    let mut out = Vec::with_capacity(unique.min(1 << 16));
    for _ in 0..unique {
        out.push(run.string()?);
    }
    Ok(out)
}

/// A cell record's cell: its column from 1 and its style index, with no value yet.
fn cell(body: &[u8]) -> Result<(u32, RawCell), Error> {
    let row = u32::from(u16_at(body, 0)?);
    Ok((
        row,
        RawCell {
            column: u64::from(u16_at(body, 2)?) + 1,
            style: u32::from(u16_at(body, 4)?),
            ..RawCell::default()
        },
    ))
}

/// What [`read`] gives `super::Reader::open`.
pub struct Book {
    pub date1904: bool,
    pub kinds: Vec<Kind>,
    pub sheets: Vec<Listed>,
}

/// Reads the xls `bytes` whole.
pub fn read(bytes: &[u8]) -> Result<Book, Error> {
    let file = cfb::File::open(bytes)?;
    let stream = match file.stream("Workbook")? {
        Some(stream) => stream,
        None if file.stream("Book")?.is_some() => {
            return Err(Error::Runtime(
                "This xls file was written by Excel 95 or earlier. Novis reads xls files from Excel 97 onwards.".to_owned(),
            ));
        }
        None => return Err(broken("it has no workbook stream")),
    };
    let mut date1904 = false;
    let mut formats: BTreeMap<u16, String> = BTreeMap::new();
    let mut styles: Vec<u16> = Vec::new();
    let mut listed: Vec<(u32, String, Visibility, u8)> = Vec::new();
    let mut strings_parts: Vec<&[u8]> = Vec::new();
    let mut in_strings = false;
    for record in records(&stream, 0) {
        let (kind, body) = record?;
        if kind != CONTINUE {
            in_strings = false;
        }
        match kind {
            BOF if u16_at(body, 0)? != BIFF8 => {
                return Err(Error::Runtime(
                    "This xls file was written by Excel 95 or earlier. Novis reads xls files from Excel 97 onwards.".to_owned(),
                ));
            }
            DATE_MODE => date1904 = u16_at(body, 0)? == 1,
            FORMAT => {
                let (code, _) = string_at(body, 2, 2)?;
                formats.insert(u16_at(body, 0)?, code);
            }
            STYLE => styles.push(u16_at(body, 2)?),
            SHEET => {
                let visibility = match u8_at(body, 4)? & 3 {
                    0 => Visibility::Visible,
                    1 => Visibility::Hidden,
                    _ => Visibility::VeryHidden,
                };
                let (name, _) = string_at(body, 6, 1)?;
                listed.push((u32_at(body, 0)?, name, visibility, u8_at(body, 5)?));
            }
            SHARED_STRINGS => {
                strings_parts.push(body);
                in_strings = true;
            }
            CONTINUE if in_strings => strings_parts.push(body),
            EOF => break,
            _ => {}
        }
    }
    let strings = if strings_parts.is_empty() {
        Vec::new()
    } else {
        shared(strings_parts)?
    };
    let kinds = styles
        .iter()
        .map(|&id| {
            format::builtin(u32::from(id))
                .or_else(|| formats.get(&id).map(|code| format::classify(code)))
                .unwrap_or(Kind::Plain)
        })
        .collect();
    let mut sheets = Vec::with_capacity(listed.len());
    for (offset, name, visibility, sheet_type) in listed {
        let kind = if sheet_type == 2 {
            SheetKind::Chartsheet
        } else {
            SheetKind::Worksheet
        };
        let rows = if kind == SheetKind::Worksheet {
            Some(Arc::new(sheet(&stream, offset as usize, &strings)?))
        } else {
            None
        };
        sheets.push(Listed {
            name,
            visibility,
            part: None,
            kind,
            rows,
        });
    }
    Ok(Book {
        date1904,
        kinds,
        sheets,
    })
}

/// The rows of the sheet whose `BOF` record is at `offset`, in order, each cell in order.
fn sheet(stream: &[u8], offset: usize, strings: &[String]) -> Result<Vec<RawRow>, Error> {
    let mut rows: BTreeMap<u32, RawRow> = BTreeMap::new();
    let mut pending: Option<(u32, RawCell)> = None;
    let put = |rows: &mut BTreeMap<u32, RawRow>, row: u32, cell: RawCell| {
        if row >= MOST_ROWS || cell.column > u64::from(MOST_COLUMNS) {
            return Err(broken("a cell is past the last row or column of the sheet"));
        }
        rows.entry(row)
            .or_insert_with(|| RawRow {
                number: u64::from(row) + 1,
                ..RawRow::default()
            })
            .cells
            .push(cell);
        Ok(())
    };
    let mut records = records(stream, offset);
    match records.next() {
        Some(Ok((BOF, _))) => {}
        _ => return Err(broken("a sheet does not start where the workbook says")),
    }
    for record in records {
        let (kind, body) = record?;
        match kind {
            EOF => break,
            ROW => {
                let row = u32::from(u16_at(body, 0)?);
                let hidden = u8_at(body, 12)? & 0x20 != 0;
                rows.entry(row)
                    .or_insert_with(|| RawRow {
                        number: u64::from(row) + 1,
                        ..RawRow::default()
                    })
                    .hidden = hidden;
            }
            NUMBER => {
                let (row, at) = cell(body)?;
                put(&mut rows, row, number(at, f64_at(body, 6)?))?;
            }
            RK => {
                let (row, at) = cell(body)?;
                put(&mut rows, row, number(at, rk(u32_at(body, 6)?)))?;
            }
            MULTIPLE_RK => {
                let row = u32::from(u16_at(body, 0)?);
                let first = u64::from(u16_at(body, 2)?) + 1;
                let count = body.len().saturating_sub(6) / 6;
                for n in 0..count {
                    let at = 4 + n * 6;
                    let raw = RawCell {
                        column: first + n as u64,
                        style: u32::from(u16_at(body, at)?),
                        ..RawCell::default()
                    };
                    put(&mut rows, row, number(raw, rk(u32_at(body, at + 2)?)))?;
                }
            }
            SHARED_LABEL => {
                let (row, at) = cell(body)?;
                let text = strings
                    .get(u32_at(body, 6)? as usize)
                    .ok_or_else(|| broken("a cell names a shared string the file does not have"))?;
                put(&mut rows, row, typed(at, b"text", Some(text.clone())))?;
            }
            LABEL | RICH_LABEL => {
                let (row, at) = cell(body)?;
                let (text, _) = string_at(body, 6, 2)?;
                put(&mut rows, row, typed(at, b"text", Some(text)))?;
            }
            BOOL_ERROR => {
                let (row, at) = cell(body)?;
                let value = u8_at(body, 6)?;
                let raw = if u8_at(body, 7)? == 0 {
                    typed(
                        at,
                        b"b",
                        Some(if value == 0 { "0" } else { "1" }.to_owned()),
                    )
                } else {
                    typed(at, b"e", error_text(value).map(str::to_owned))
                };
                put(&mut rows, row, raw)?;
            }
            FORMULA => {
                let (row, at) = cell(body)?;
                let result = body
                    .get(6..14)
                    .ok_or_else(|| broken("a record is shorter than its fields"))?;
                if result[6..8] != [0xFF, 0xFF] {
                    put(&mut rows, row, number(at, f64_at(body, 6)?))?;
                    continue;
                }
                match result[0] {
                    // The text result is the `STRING` record that follows.
                    0 => pending = Some((row, at)),
                    1 => put(
                        &mut rows,
                        row,
                        typed(
                            at,
                            b"b",
                            Some(if result[2] == 0 { "0" } else { "1" }.to_owned()),
                        ),
                    )?,
                    2 => put(
                        &mut rows,
                        row,
                        typed(at, b"e", error_text(result[2]).map(str::to_owned)),
                    )?,
                    _ => {}
                }
            }
            STRING => {
                if let Some((row, at)) = pending.take() {
                    let (text, _) = string_at(body, 0, 2)?;
                    put(&mut rows, row, typed(at, b"text", Some(text)))?;
                }
            }
            _ => {}
        }
    }
    Ok(rows
        .into_values()
        .map(|mut row| {
            row.cells.sort_by_key(|cell| cell.column);
            row.cells.dedup_by_key(|cell| cell.column);
            row
        })
        .collect())
}
