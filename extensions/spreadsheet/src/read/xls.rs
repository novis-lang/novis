//! xls, the binary workbook of Excel 97 to 2003: a compound file ([`super::cfb`]) whose
//! `Workbook` stream is a run of BIFF8 records, each a type, a length and that many bytes.
//!
//! **An xls is read whole.** [`read`] reads the workbook globals — the date system, the number
//! formats, the cell styles, the sheets and the shared strings — and then every sheet's cells into
//! the [`RawRow`]s the xlsx cursor gives, so `super::Reader::value` reads every format the same
//! way. A sheet holds at most 65,536 rows. A text cell is the kind `text`, and a number has its
//! cell style's index. The streams of an older Excel, BIFF5 and before, are [`Error::Runtime`].
//! A sheet's merged ranges are its `MERGEDCELLS` records, read with its cells, and so are its
//! hyperlinks, notes and pictures and the workbook's names and properties ([`xls_extras`]).
//!
//! **Formulas.** A formula cell gives its stored result, and its parsed tokens are turned into A1
//! text by [`super::tokens`] when the file opens, through the sheets, the `XTI` list, the add-in
//! names and the defined names of the globals. So a read with `formulas: true` gives a formula as
//! an xlsx cell does. Each cell of a shared formula holds a `ptgExp` naming the formula's first
//! cell, and its `SHRFMLA` follows the first of those cells. So the cells are given their text
//! once the sheet's records are read. An array formula's text is given in its first cell. What
//! it spends: the text of each formula, held with the rows.
//!
//! A record longer than 8,224 bytes goes on in `CONTINUE` records. The shared strings are the one
//! table this reader needs that crosses them, and [`Continued`] reads across the seams, where a
//! string's characters start again with a byte that says whether they are one byte or two.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::binary::{error_text, f64_at, number, rk, typed, u8_at, u16_at, u32_at};
use super::extras::{Link, Name, Note};
use super::format::Style;
use super::merge::Merge;
use super::sheet::{RawCell, RawRow};
use super::tokens::{self, Biff, Linked, Place};
use super::{Listed, SheetKind, cfb, xls_extras};
use crate::{Error, Visibility};

const FORMULA: u16 = 0x0006;
const EOF: u16 = 0x000A;
const EXTERN_SHEET: u16 = 0x0017;
const NAME: u16 = 0x0018;
const EXTERN_NAME: u16 = 0x0023;
const NOTE: u16 = 0x001C;
const DATE_MODE: u16 = 0x0022;
const OBJ: u16 = 0x005D;
const MSO_DRAWING_GROUP: u16 = 0x00EB;
const MSO_DRAWING: u16 = 0x00EC;
const TXO: u16 = 0x01B6;
const HLINK: u16 = 0x01B8;
const HLINK_TOOLTIP: u16 = 0x0800;
/// The type an `OBJ` record gives a note's object.
const NOTE_OBJECT: u16 = 0x19;
const CONTINUE: u16 = 0x003C;
const SHEET: u16 = 0x0085;
const MULTIPLE_RK: u16 = 0x00BD;
const SUP_BOOK: u16 = 0x01AE;
const RICH_LABEL: u16 = 0x00D6;
const STYLE: u16 = 0x00E0;
const MERGED_CELLS: u16 = 0x00E5;
const SHARED_STRINGS: u16 = 0x00FC;
const SHARED_LABEL: u16 = 0x00FD;
const NUMBER: u16 = 0x0203;
const LABEL: u16 = 0x0204;
const BOOL_ERROR: u16 = 0x0205;
const STRING: u16 = 0x0207;
const ROW: u16 = 0x0208;
const ARRAY: u16 = 0x0221;
const RK: u16 = 0x027E;
const FORMAT: u16 = 0x041E;
const SHARED_FORMULA: u16 = 0x04BC;
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

/// The built-in defined names by their code, as an xlsx file names them after `_xlnm.`.
const BUILT_IN_NAMES: [&str; 14] = [
    "Consolidate_Area",
    "Auto_Open",
    "Auto_Close",
    "Extract",
    "Database",
    "Criteria",
    "Print_Area",
    "Print_Titles",
    "Recorder",
    "Data_Form",
    "Auto_Activate",
    "Auto_Deactivate",
    "Sheet_Title",
    "_FilterDatabase",
];

/// The name a `NAME` record defines: its flags, its shortcut key, its length, the sizes and the
/// sheet, and at 14 the flags byte of its characters. A built-in name is one character, its code.
fn defined_name(body: &[u8]) -> Option<String> {
    let built_in = u16_at(body, 0).ok()? & 0x0020 != 0;
    let count = usize::from(u8_at(body, 3).ok()?);
    let wide = u8_at(body, 14).ok()? & 1 != 0;
    let bytes = body.get(15..15 + count * if wide { 2 } else { 1 })?;
    let name = characters(bytes, wide);
    if !built_in {
        return Some(name);
    }
    let code = name.chars().next()? as usize;
    Some(format!("_xlnm.{}", BUILT_IN_NAMES.get(code)?))
}

/// A record's parsed formula at `at`: a two-byte count, that many bytes of tokens, and the rest of
/// the record, its extra data. `None` when the tokens are longer than the record.
fn parsed_at(body: &[u8], at: usize) -> Option<(&[u8], &[u8])> {
    let count = usize::from(u16_at(body, at).ok()?);
    let rest = body.get(at + 2..)?;
    (count <= rest.len()).then(|| rest.split_at(count))
}

/// A shared or array formula of a sheet: its first and last row and column, from 0, and its
/// tokens and extra data.
struct Shared<'a> {
    rows: (u32, u32),
    columns: (u32, u32),
    tokens: &'a [u8],
    extra: &'a [u8],
    array: bool,
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

/// A `NAME` record's formula, after its name: the token count at 4, and the name's length at 3
/// and the flags byte of its characters at 14. Gives the tokens and the extra data.
fn name_formula(body: &[u8]) -> Option<(&[u8], &[u8])> {
    let tokens = usize::from(u16_at(body, 4).ok()?);
    let count = usize::from(u8_at(body, 3).ok()?);
    let wide = u8_at(body, 14).ok()? & 1 != 0;
    let start = 15 + count * if wide { 2 } else { 1 };
    let rest = body.get(start..)?;
    (tokens <= rest.len()).then(|| rest.split_at(tokens))
}

/// What [`read`] gives `super::Reader::open`.
pub struct Book {
    pub date1904: bool,
    pub styles: Vec<Style>,
    pub sheets: Vec<Listed>,
    pub extras: xls_extras::Extras,
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
    let mut group: Vec<u8> = Vec::new();
    let mut name_records: Vec<&[u8]> = Vec::new();
    let mut last = 0;
    let mut book = tokens::Book::default();
    for record in records(&stream, 0) {
        let (kind, body) = record?;
        let in_strings = last == SHARED_STRINGS;
        let in_group = last == MSO_DRAWING_GROUP;
        if kind != CONTINUE {
            last = kind;
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
            SHARED_STRINGS => strings_parts.push(body),
            CONTINUE if in_strings => strings_parts.push(body),
            MSO_DRAWING_GROUP => group.extend_from_slice(body),
            CONTINUE if in_group => group.extend_from_slice(body),
            // The sheet count, then a code that says which workbook this is.
            // Another workbook's code is the length of its path, and its sheet names follow the
            // path. A name that does not read ends the list, and a reference past it is `#REF!`.
            SUP_BOOK => {
                let linked = match u16_at(body, 2)? {
                    0x0401 => Linked::Own,
                    0x3A01 => Linked::AddIn(Vec::new()),
                    _ => {
                        let mut other = Linked::other(&book.linked);
                        if let (Linked::Other { sheets, .. }, Ok((_, used))) =
                            (&mut other, string_at(body, 2, 2))
                        {
                            let mut at = 2 + used;
                            for _ in 0..u16_at(body, 0)? {
                                let Ok((name, used)) = string_at(body, at, 2) else {
                                    break;
                                };
                                sheets.push(name);
                                at += used;
                            }
                        }
                        other
                    }
                };
                book.linked.push(linked);
            }
            // The names of an add-in or another workbook follow its `SUPBOOK`, each after six
            // bytes of flags. A name that does not read is kept empty, so the names after it keep
            // their numbers.
            EXTERN_NAME => {
                if let Some(Linked::AddIn(names) | Linked::Other { names, .. }) =
                    book.linked.last_mut()
                {
                    names.push(
                        string_at(body, 6, 1)
                            .map(|(name, _)| name)
                            .unwrap_or_default(),
                    );
                }
            }
            // A count, then each `XTI`: its `SUPBOOK` and its first and last sheet.
            EXTERN_SHEET => {
                let count = usize::from(u16_at(body, 0)?);
                for xti in body
                    .get(2..)
                    .unwrap_or_default()
                    .chunks_exact(6)
                    .take(count)
                {
                    let sheet = |n: usize| i32::from(i16::from_le_bytes([xti[n], xti[n + 1]]));
                    let linked = u32::from(u16::from_le_bytes([xti[0], xti[1]]));
                    book.xti.push((linked, sheet(2), sheet(4)));
                }
            }
            // A name that does not read is kept empty, so the names after it keep their numbers.
            NAME => {
                book.names.push(defined_name(body).unwrap_or_default());
                name_records.push(body);
            }
            EOF => break,
            _ => {}
        }
    }
    let strings = if strings_parts.is_empty() {
        Vec::new()
    } else {
        shared(strings_parts)?
    };
    // An xls writes a FORMAT record for each built-in id it shows in its own locale, so the
    // record's code is the one a cell displays.
    let formats: BTreeMap<u16, std::sync::Arc<str>> = formats
        .into_iter()
        .map(|(id, code)| (id, code.into()))
        .collect();
    let styles = styles
        .iter()
        .map(|&id| Style::of(u32::from(id), formats.get(&id)))
        .collect();
    book.sheets = listed.iter().map(|(_, name, ..)| name.clone()).collect();
    let mut extras = xls_extras::Extras {
        properties: xls_extras::properties(&file)?,
        ..xls_extras::Extras::default()
    };
    // A name's sheet is its `itab` at 8, counted from 1, and 0 for the workbook.
    for (body, name) in name_records.iter().zip(&book.names) {
        let place = Place {
            base: (0, 0),
            shared: true,
        };
        let reference = name_formula(body)
            .and_then(|(tokens, extra)| tokens::decode(Biff::Eight, tokens, extra, place, &book));
        let sheet = u16_at(body, 8)
            .ok()
            .and_then(|itab| usize::from(itab).checked_sub(1))
            .and_then(|index| book.sheets.get(index))
            .cloned();
        extras.names.push(Name {
            name: name.clone(),
            reference: reference.unwrap_or_default(),
            sheet,
        });
    }
    let store = if group.is_empty() {
        Vec::new()
    } else {
        xls_extras::store(&group)
    };
    let mut sheets = Vec::with_capacity(listed.len());
    for (offset, name, visibility, sheet_type) in listed {
        let kind = if sheet_type == 2 {
            SheetKind::Chartsheet
        } else {
            SheetKind::Worksheet
        };
        let (rows, merged) = if kind == SheetKind::Worksheet {
            let read = sheet(&stream, offset as usize, &strings, &book)?;
            let on = |entry| (name.clone(), entry);
            extras.links.extend(read.links.into_iter().map(on));
            let on = |entry| (name.clone(), entry);
            extras.notes.extend(read.notes.into_iter().map(on));
            let on = |entry| (name.clone(), entry);
            let images = xls_extras::placed(&read.drawing)
                .into_iter()
                .filter_map(|(index, column, row)| xls_extras::image(&store, index, column, row));
            extras.images.extend(images.map(on));
            (Some(Arc::new(read.rows)), read.merged)
        } else {
            (None, Vec::new())
        };
        sheets.push(Listed {
            name,
            visibility,
            part: None,
            kind,
            rows,
            merged,
        });
    }
    Ok(Book {
        date1904,
        styles,
        sheets,
        extras,
    })
}

/// What [`sheet`] reads of one sheet.
struct Read {
    rows: Vec<RawRow>,
    merged: Vec<Merge>,
    links: Vec<Link>,
    notes: Vec<Note>,
    /// The sheet's `MSODRAWING` records and their `CONTINUE`s, as one run of bytes.
    drawing: Vec<u8>,
}

/// The rows of the sheet whose `BOF` record is at `offset`, in order, each cell in order with
/// its formula's text, its merged ranges, its hyperlinks, its notes and its drawing.
fn sheet(
    stream: &[u8],
    offset: usize,
    strings: &[String],
    book: &tokens::Book,
) -> Result<Read, Error> {
    let mut rows: BTreeMap<u32, RawRow> = BTreeMap::new();
    let mut merged = Vec::new();
    let mut links: Vec<Link> = Vec::new();
    let mut drawing = Vec::new();
    // The id of the last note's `OBJ`, the text of the `TXO` being read with the characters it
    // still needs, each note's text by its id, and each `NOTE` with its id and author.
    let mut object = None;
    let mut text: Option<(u16, usize, String)> = None;
    let mut texts: Vec<(u16, String)> = Vec::new();
    let mut noted: Vec<(String, u16, String)> = Vec::new();
    let mut last = 0;
    let mut pending: Option<(u32, RawCell)> = None;
    let mut shared: Vec<Shared<'_>> = Vec::new();
    // Each cell of a shared or array formula: its row and column from 0, and the first cell of
    // the formula it uses.
    let mut uses: Vec<((u32, u32), (u32, u32))> = Vec::new();
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
        let after = last;
        if kind != CONTINUE {
            last = kind;
        }
        match kind {
            EOF => break,
            HLINK => links.extend(xls_extras::hyperlink(body)?),
            HLINK_TOOLTIP if after == HLINK => {
                if let Some(link) = links.last_mut() {
                    link.tooltip = Some(xls_extras::tooltip(body));
                }
            }
            MSO_DRAWING => drawing.extend_from_slice(body),
            CONTINUE if after == MSO_DRAWING => drawing.extend_from_slice(body),
            OBJ => {
                let (kind, id) = xls_extras::object(body)?;
                object = (kind == NOTE_OBJECT).then_some(id);
            }
            TXO => {
                if let Some(id) = object.take() {
                    let length = xls_extras::text_length(body)?;
                    if length == 0 {
                        texts.push((id, String::new()));
                    } else {
                        text = Some((id, length, String::new()));
                    }
                }
            }
            CONTINUE if after == TXO => {
                if let Some((id, left, mut out)) = text.take() {
                    let left = left - xls_extras::text_part(body, left, &mut out)?;
                    if left == 0 {
                        texts.push((id, out));
                    } else {
                        text = Some((id, left, out));
                    }
                }
            }
            NOTE => {
                let (cell, id) = xls_extras::note(body)?;
                let (author, _) = string_at(body, 8, 2)?;
                noted.push((cell, id, author));
            }
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
            // The cell, its result, two bytes of flags, four bytes of cache, then the formula.
            FORMULA => {
                let (row, mut at) = cell(body)?;
                if let Some((tokens, extra)) = parsed_at(body, 20) {
                    let column = u32::try_from(at.column - 1).unwrap_or(u32::MAX);
                    match tokens::shared_at(Biff::Eight, tokens) {
                        Some((first, Some(first_column))) => {
                            uses.push(((row, column), (first, first_column)));
                        }
                        _ => {
                            let place = Place {
                                base: (row, column),
                                shared: false,
                            };
                            at.formula = Some(
                                tokens::decode(Biff::Eight, tokens, extra, place, book)
                                    .unwrap_or_default(),
                            );
                        }
                    }
                }
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
            // The range, as two rows of two bytes and two columns of one, then two bytes, then
            // the formula. An array formula has six bytes of flags where a shared one has two.
            SHARED_FORMULA | ARRAY => {
                let array = kind == ARRAY;
                if let Some((tokens, extra)) = parsed_at(body, if array { 12 } else { 8 }) {
                    shared.push(Shared {
                        rows: (u32::from(u16_at(body, 0)?), u32::from(u16_at(body, 2)?)),
                        columns: (u32::from(u8_at(body, 4)?), u32::from(u8_at(body, 5)?)),
                        tokens,
                        extra,
                        array,
                    });
                }
            }
            // A count, then each range as its first and last row and first and last column, each
            // from 0 in two bytes.
            MERGED_CELLS => {
                let count = usize::from(u16_at(body, 0)?);
                for n in 0..count {
                    let corner = |at| u16_at(body, 2 + n * 8 + at).map(|v| u64::from(v) + 1);
                    merged.push(Merge::new(
                        (corner(0)?, corner(2)?),
                        (corner(4)?, corner(6)?),
                    )?);
                }
            }
            _ => {}
        }
    }
    for ((row, column), first) in uses {
        let Some(formula) = shared
            .iter()
            .find(|formula| (formula.rows.0, formula.columns.0) == first)
        else {
            continue;
        };
        let covered = (formula.rows.0..=formula.rows.1).contains(&row)
            && (formula.columns.0..=formula.columns.1).contains(&column);
        if !covered || (formula.array && (row, column) != first) {
            continue;
        }
        let place = Place {
            base: (row, column),
            shared: !formula.array,
        };
        let text = tokens::decode(Biff::Eight, formula.tokens, formula.extra, place, book);
        let cell = rows.get_mut(&row).and_then(|cells| {
            cells
                .cells
                .iter_mut()
                .find(|cell| cell.column == u64::from(column) + 1)
        });
        if let Some(cell) = cell {
            cell.formula = Some(text.unwrap_or_default());
        }
    }
    let rows = rows
        .into_values()
        .map(|mut row| {
            row.cells.sort_by_key(|cell| cell.column);
            row.cells.dedup_by_key(|cell| cell.column);
            row
        })
        .collect();
    let notes = noted
        .into_iter()
        .map(|(cell, id, author)| Note {
            noted: cell,
            note: texts
                .iter()
                .find(|(of, _)| *of == id)
                .map(|(_, text)| text.clone())
                .unwrap_or_default(),
            author: (!author.is_empty()).then_some(author),
        })
        .collect();
    Ok(Read {
        rows,
        merged,
        links,
        notes,
        drawing,
    })
}
