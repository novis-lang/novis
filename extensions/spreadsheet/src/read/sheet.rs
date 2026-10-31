//! One worksheet read as a stream of rows.
//!
//! A [`Cursor`] inflates the sheet's part a block at a time into a buffer and parses one `<row>`
//! at a time out of it. A row cut by the end of the buffer is parsed again once more bytes are
//! in, and the bytes before the row are dropped, so the buffer holds one block and one row. The
//! cursor reads every row in order. It never seeks back, and starting a sheet again opens a new
//! cursor.
//!
//! [`RawCell`] is a cell as the file writes it: its column, its type attribute, its style index,
//! its `<v>`, its `<f>`, and the text of an inline string. `super::value` turns it into a read
//! value.
//!
//! [`merged`] reads a sheet's merged ranges, which come after its rows, in a pass of its own.

use super::merge::Merge;
use super::xml::{self, Token};
use super::zip::{Entry, Part};
use super::{MOST_COLUMNS, MOST_ROWS, Source, letters};
use crate::Error;

/// A cell as the file writes it.
#[derive(Debug, Default, Clone)]
pub struct RawCell {
    /// From 1.
    pub column: u64,
    pub kind: Vec<u8>,
    pub style: u32,
    pub value: Option<String>,
    pub formula: Option<String>,
    pub inline: Option<String>,
}

/// A row as the file writes it.
#[derive(Debug, Default, Clone)]
pub struct RawRow {
    /// From 1.
    pub number: u64,
    pub hidden: bool,
    pub cells: Vec<RawCell>,
}

/// A worksheet being read.
#[derive(Debug)]
pub struct Cursor {
    part: Part,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    in_data: bool,
    over: bool,
    last: u64,
}

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file is not a valid workbook: {what}."))
}

/// The column of an `A1` reference, from 1, or `None` for text that does not start with letters.
pub fn column_of(reference: &[u8]) -> Option<u64> {
    let letters = reference
        .iter()
        .take_while(|b| b.is_ascii_alphabetic())
        .count();
    if letters == 0 || letters > 3 {
        return None;
    }
    Some(reference[..letters].iter().fold(0, |column, b| {
        column * 26 + u64::from(b.to_ascii_uppercase() - b'A' + 1)
    }))
}

/// The row of an `A1` reference.
pub fn row_of(reference: &[u8]) -> Option<u64> {
    let digits = &reference[reference
        .iter()
        .position(u8::is_ascii_digit)
        .unwrap_or(reference.len())..];
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// The rows and columns a sheet's `<dimension ref>` declares, as `A1:C10` or `A1`.
pub fn dimension(reference: &[u8]) -> (u64, u64) {
    let last = reference
        .split(|&b| b == b':')
        .next_back()
        .unwrap_or(reference);
    (row_of(last).unwrap_or(0), column_of(last).unwrap_or(0))
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
        })
    }

    /// The number of the last row read, 0 before the first.
    pub fn last(&self) -> u64 {
        self.last
    }

    /// Reads more of the part into the buffer, first dropping what was parsed.
    fn more(&mut self, source: &dyn Source) -> Result<(), Error> {
        if self.eof {
            return Err(broken("a worksheet ends inside an element"));
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
        Ok(())
    }

    /// The raw attributes of the sheet's `<dimension>`, read before its first row, or `None`
    /// when the sheet has none.
    pub fn dimension(&mut self, source: &dyn Source) -> Result<Option<(u64, u64)>, Error> {
        loop {
            match xml::next(&self.buf[self.pos..])? {
                None if self.eof => return Ok(None),
                None => self.more(source)?,
                Some((Token::Open { name, attrs, .. }, _)) if xml::local(name) == b"dimension" => {
                    return Ok(xml::attr(attrs, b"ref").map(dimension));
                }
                Some((Token::Open { name, .. }, _)) if xml::local(name) == b"sheetData" => {
                    return Ok(None);
                }
                Some((_, length)) => self.pos += length,
            }
        }
    }

    /// The next row of the sheet, or `None` after its last.
    pub fn next(&mut self, source: &dyn Source) -> Result<Option<RawRow>, Error> {
        loop {
            if self.over {
                return Ok(None);
            }
            let token = xml::next(&self.buf[self.pos..])?;
            match token {
                None if self.eof && self.pos == self.buf.len() => return Ok(None),
                None => self.more(source)?,
                Some((Token::Open { name, attrs, empty }, length)) => {
                    let name = xml::local(name);
                    if name == b"sheetData" {
                        self.in_data = !empty;
                        self.over = empty;
                        self.pos += length;
                    } else if name == b"row" && self.in_data {
                        match parse_row(&self.buf[self.pos..], attrs, empty, length, self.last)? {
                            Some((row, consumed)) => {
                                self.pos += consumed;
                                if row.number <= self.last {
                                    return Err(broken("a worksheet's rows are out of order"));
                                }
                                self.last = row.number;
                                return Ok(Some(row));
                            }
                            None => self.more(source)?,
                        }
                    } else {
                        self.pos += length;
                    }
                }
                Some((Token::Close(name), length)) => {
                    if xml::local(name) == b"sheetData" {
                        self.over = true;
                    }
                    self.pos += length;
                }
                Some((_, length)) => self.pos += length,
            }
        }
    }
}

/// The merged ranges of the sheet whose part is `entry`, read a block at a time to the part's end.
pub fn merged(source: &dyn Source, entry: &Entry) -> Result<Vec<Merge>, Error> {
    let mut ranges = Vec::new();
    scan(source, entry, |name, attrs| {
        if name == b"mergeCell"
            && let Some(reference) = xml::attr(attrs, b"ref")
        {
            ranges.push(Merge::parse(reference)?);
        }
        Ok(())
    })?;
    Ok(ranges)
}

/// Calls `each` with the local name and the attributes of every start tag of the part `entry`,
/// read a block at a time to the part's end, so the part is never held whole.
pub fn scan(
    source: &dyn Source,
    entry: &Entry,
    mut each: impl FnMut(&[u8], &[u8]) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut part = Part::open(source, entry)?;
    let mut buf = Vec::new();
    let mut at = 0;
    let mut eof = false;
    loop {
        let Some((token, size)) = xml::next(&buf[at..])? else {
            if eof {
                return Ok(());
            }
            buf.drain(..at);
            at = 0;
            eof = !part.fill(source, &mut buf)?;
            continue;
        };
        at += size;
        if let Token::Open { name, attrs, .. } = token {
            each(xml::local(name), attrs)?;
        }
    }
}

/// Where the text of a row's element goes.
#[derive(Clone, Copy, PartialEq)]
enum In {
    Nothing,
    Value,
    Formula,
    Inline,
    Phonetic,
}

/// `row`, or [`Error::Parse`] naming it when it is past the last row of a sheet. A cell of such
/// a row is refused earlier, naming the cell.
fn within(row: RawRow) -> Result<RawRow, Error> {
    if row.number > MOST_ROWS {
        return Err(past_row(row.number));
    }
    Ok(row)
}

/// The error for the row `number`, past the last row of a sheet. xlsb's cursor throws it too.
pub(super) fn past_row(number: u64) -> Error {
    broken(&format!(
        "the row {number} is past the last row of a sheet, {MOST_ROWS}"
    ))
}

/// The error for the cell in `column` of the row `row`, past the last cell of a sheet. xlsb's
/// cursor throws it too.
pub(super) fn past_cell(column: u64, row: u64) -> Error {
    broken(&format!(
        "the cell `{}{row}` is past the last cell of a sheet, `XFD1048576`",
        letters(column)
    ))
}

/// Parses the `<row>` that starts `buf`, whose start tag is `length` bytes and has `attrs`.
/// Returns `None` when `buf` ends inside the row.
fn parse_row(
    buf: &[u8],
    attrs: &[u8],
    empty: bool,
    length: usize,
    last: u64,
) -> Result<Option<(RawRow, usize)>, Error> {
    let mut row = RawRow {
        number: match xml::attr(attrs, b"r") {
            Some(r) => std::str::from_utf8(r)
                .ok()
                .and_then(|r| r.parse().ok())
                .filter(|&r| r > 0)
                .ok_or_else(|| broken("a row has no valid number"))?,
            None => last + 1,
        },
        hidden: matches!(xml::attr(attrs, b"hidden"), Some(b"1" | b"true")),
        cells: Vec::new(),
    };
    if empty {
        return within(row).map(|row| Some((row, length)));
    }
    let mut at = length;
    let mut into = In::Nothing;
    let mut text = Vec::new();
    loop {
        let Some((token, size)) = xml::next(&buf[at..])? else {
            return Ok(None);
        };
        at += size;
        match token {
            Token::Open { name, attrs, empty } => match xml::local(name) {
                b"c" => {
                    let column = match xml::attr(attrs, b"r") {
                        Some(r) => {
                            column_of(r).ok_or_else(|| broken("a cell has no valid reference"))?
                        }
                        None => row.cells.last().map_or(1, |cell| cell.column + 1),
                    };
                    if column > MOST_COLUMNS || row.number > MOST_ROWS {
                        return Err(past_cell(column, row.number));
                    }
                    let style = xml::attr(attrs, b"s")
                        .and_then(|s| std::str::from_utf8(s).ok()?.parse().ok())
                        .unwrap_or(0);
                    row.cells.push(RawCell {
                        column,
                        kind: xml::attr(attrs, b"t").unwrap_or(b"n").to_vec(),
                        style,
                        ..RawCell::default()
                    });
                }
                b"v" if !empty => into = In::Value,
                b"f" if !empty => into = In::Formula,
                b"is" => {
                    if let Some(cell) = row.cells.last_mut() {
                        cell.inline.get_or_insert_with(String::new);
                    }
                }
                b"t" if into != In::Phonetic && !empty => into = In::Inline,
                b"rPh" if !empty => into = In::Phonetic,
                _ => {}
            },
            Token::Text(raw) | Token::Raw(raw) if into != In::Nothing && into != In::Phonetic => {
                if matches!(token, Token::Raw(_)) {
                    text.extend_from_slice(raw);
                } else {
                    let mut decoded = String::new();
                    xml::decode(raw, &mut decoded)?;
                    text.extend_from_slice(decoded.as_bytes());
                }
            }
            Token::Close(name) => match xml::local(name) {
                b"row" => return within(row).map(|row| Some((row, at))),
                b"v" | b"f" | b"t" if into != In::Phonetic => {
                    let done = String::from_utf8_lossy(&text).into_owned();
                    text.clear();
                    if let Some(cell) = row.cells.last_mut() {
                        match into {
                            In::Value => cell.value = Some(done),
                            In::Formula => cell.formula = Some(done),
                            In::Inline => {
                                cell.inline.get_or_insert_with(String::new).push_str(&done)
                            }
                            In::Nothing | In::Phonetic => {}
                        }
                    }
                    into = In::Nothing;
                }
                b"rPh" => into = In::Nothing,
                _ => {}
            },
            _ => {}
        }
    }
}
