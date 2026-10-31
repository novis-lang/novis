//! A template's worksheet: its XML kept as text, and the rows and cells in it found once, so a
//! fill copies every byte it does not change (`rule:core-classes/spreadsheet-template-fill`).
//!
//! [`Sheet::parse`] finds `<sheetData>`, and in it each row and cell with the byte ranges of
//! their `r` attributes and of a formula's text, `ref` and shared group. A cell's text, inline or from the
//! shared strings, is kept only when it holds `{{`. A row or a cell `Template::set` adds has no
//! range, and a cell it fills has its value.

use std::ops::Range;

use crate::Error;
use crate::Value;
use crate::read::xml::{self, Token};

/// One cell of the template.
#[derive(Debug, Clone)]
pub struct Cell {
    pub column: u32,
    pub raw: Option<Range<usize>>,
    pub r: Option<Range<usize>>,
    pub style: Option<String>,
    /// The cell's text, when it holds a placeholder.
    pub text: Option<String>,
    /// The formula's text, as written, and its range.
    pub formula: Option<(Range<usize>, String)>,
    /// The range of a formula's `ref` attribute.
    pub formula_ref: Option<Range<usize>>,
    /// A shared formula's group: the range of its `si` attribute and its number. The cell with
    /// the formula's text is the group's first cell, and the others have no text.
    pub shared: Option<(Range<usize>, u32)>,
    /// A value `set` gave the cell.
    pub set: Option<Option<Value>>,
}

/// One row of the template.
#[derive(Debug, Clone)]
pub struct Row {
    pub number: u32,
    /// The whole row, its start tag and whether it closes itself.
    pub raw: Option<Range<usize>>,
    pub open: Range<usize>,
    pub empty: bool,
    pub r: Option<Range<usize>>,
    pub cells: Vec<Cell>,
}

/// One worksheet of the template.
#[derive(Debug)]
pub struct Sheet {
    pub name: String,
    pub path: String,
    pub xml: String,
    /// Where the rows begin, and where `</sheetData>` begins.
    pub rows_at: usize,
    pub tail_at: usize,
    /// Whether the sheet's data was `<sheetData/>`, which a fill writes as an open and a close.
    pub self_closed: bool,
    pub rows: Vec<Row>,
    /// One past the highest shared formula group, the first number a new group may take.
    pub next_group: u32,
}

fn broken(path: &str) -> Error {
    Error::Parse(format!(
        "The template's part `{path}` is not a valid worksheet."
    ))
}

fn offset(base: &[u8], part: &[u8]) -> usize {
    part.as_ptr() as usize - base.as_ptr() as usize
}

fn span(base: &[u8], attrs: &[u8], name: &[u8]) -> Option<Range<usize>> {
    xml::attr(attrs, name).map(|value| {
        let at = offset(base, value);
        at..at + value.len()
    })
}

fn decoded(raw: &[u8]) -> Result<String, Error> {
    let mut out = String::new();
    xml::decode(raw, &mut out)?;
    Ok(out)
}

/// The column, from 0, and the row of a reference such as `C5`.
pub fn address(text: &str) -> Option<(u32, u32)> {
    let letters = text.bytes().take_while(u8::is_ascii_alphabetic).count();
    if letters == 0 || letters > 3 {
        return None;
    }
    let column = text[..letters].bytes().fold(0u32, |n, b| {
        n * 26 + u32::from(b.to_ascii_uppercase() - b'A') + 1
    });
    let row: u32 = text[letters..].parse().ok()?;
    ((1..=1_048_576).contains(&row) && column <= 16_384).then_some((column - 1, row))
}

impl Sheet {
    /// Parses the worksheet `xml`, with `strings` the workbook's shared strings.
    pub fn parse(
        name: String,
        path: String,
        xml: String,
        strings: &[String],
    ) -> Result<Self, Error> {
        let bytes = xml.as_bytes();
        let mut at = 0;
        let mut rows = Vec::new();
        let mut rows_at = None;
        let mut tail_at = None;
        let mut self_closed = false;
        let mut row: Option<Row> = None;
        let mut cell: Option<(Cell, Option<String>, String)> = None;
        // The element whose text is being read: `v`, `t` or `f`.
        let mut reading: Option<&[u8]> = None;
        let mut in_phonetic = false;
        let mut next_group = 0;
        while let Some((token, length)) = xml::next(&bytes[at..])? {
            let start = at;
            at += length;
            match token {
                Token::Open { name, attrs, empty } => match xml::local(name) {
                    b"sheetData" if rows_at.is_none() => {
                        rows_at = Some(at);
                        if empty {
                            self_closed = true;
                            tail_at = Some(start);
                            rows_at = Some(start);
                        }
                    }
                    b"row" if rows_at.is_some() && tail_at.is_none() => {
                        let number = match xml::attr(attrs, b"r") {
                            Some(r) => std::str::from_utf8(r)
                                .ok()
                                .and_then(|r| r.parse().ok())
                                .ok_or_else(|| broken(&path))?,
                            None => rows.last().map_or(1, |row: &Row| row.number + 1),
                        };
                        let made = Row {
                            number,
                            raw: Some(start..at),
                            open: start..at,
                            empty,
                            r: span(bytes, attrs, b"r"),
                            cells: Vec::new(),
                        };
                        if empty {
                            rows.push(made);
                        } else {
                            row = Some(made);
                        }
                    }
                    b"c" if row.is_some() => {
                        let current = row.as_ref().expect("checked");
                        let column = match xml::attr_text(attrs, b"r")? {
                            Some(r) => address(&r).ok_or_else(|| broken(&path))?.0,
                            None => current.cells.last().map_or(0, |cell| cell.column + 1),
                        };
                        let made = Cell {
                            column,
                            raw: Some(start..at),
                            r: span(bytes, attrs, b"r"),
                            style: xml::attr_text(attrs, b"s")?,
                            text: None,
                            formula: None,
                            formula_ref: None,
                            shared: None,
                            set: None,
                        };
                        if empty {
                            row.as_mut().expect("checked").cells.push(made);
                        } else {
                            let kind = xml::attr_text(attrs, b"t")?;
                            cell = Some((made, kind, String::new()));
                        }
                    }
                    b"f" if cell.is_some() => {
                        let (made, _, _) = cell.as_mut().expect("checked");
                        made.formula_ref = span(bytes, attrs, b"ref");
                        if xml::attr(attrs, b"t") == Some(b"shared")
                            && let Some(at) = span(bytes, attrs, b"si")
                        {
                            let group: u32 = xml[at.clone()].parse().map_err(|_| broken(&path))?;
                            next_group = next_group.max(group.saturating_add(1));
                            made.shared = Some((at, group));
                        }
                        if !empty {
                            reading = Some(b"f");
                        }
                    }
                    b"rPh" => in_phonetic = !empty,
                    b"v" | b"t" if cell.is_some() && !empty && !in_phonetic => {
                        reading = Some(xml::local(name));
                    }
                    _ => {}
                },
                Token::Text(raw) | Token::Raw(raw) => {
                    if let (Some(what), Some((made, kind, text))) = (reading, cell.as_mut()) {
                        let value = if matches!(token, Token::Raw(_)) {
                            String::from_utf8_lossy(raw).into_owned()
                        } else {
                            decoded(raw)?
                        };
                        match what {
                            b"f" => {
                                let from = offset(bytes, raw);
                                made.formula = Some((from..from + raw.len(), value));
                            }
                            b"v" if kind.as_deref() == Some("s") => {
                                let index: usize =
                                    value.trim().parse().map_err(|_| broken(&path))?;
                                text.push_str(strings.get(index).map_or("", String::as_str));
                            }
                            b"v" if kind.as_deref() == Some("str") => {}
                            b"t" => text.push_str(&value),
                            _ => {}
                        }
                    }
                }
                Token::Close(name) => match xml::local(name) {
                    b"f" | b"v" | b"t" => reading = None,
                    b"rPh" => in_phonetic = false,
                    b"c" => {
                        if let (Some((mut made, _, text)), Some(current)) =
                            (cell.take(), row.as_mut())
                        {
                            if let Some(raw) = &mut made.raw {
                                raw.end = at;
                            }
                            if made.formula.is_none() && text.contains("{{") {
                                made.text = Some(xml::unescape(text));
                            }
                            current.cells.push(made);
                        }
                    }
                    b"row" => {
                        if let Some(mut done) = row.take() {
                            if let Some(raw) = &mut done.raw {
                                raw.end = at;
                            }
                            rows.push(done);
                        }
                    }
                    b"sheetData" if tail_at.is_none() => tail_at = Some(start),
                    _ => {}
                },
                Token::Skip => {}
            }
        }
        let (Some(rows_at), Some(tail_at)) = (rows_at, tail_at) else {
            return Err(broken(&path));
        };
        Ok(Self {
            name,
            path,
            xml,
            rows_at,
            tail_at,
            self_closed,
            rows,
            next_group,
        })
    }

    /// The row numbered `number`, made with no cells when the template has none there.
    pub fn row_mut(&mut self, number: u32) -> &mut Row {
        let at = match self.rows.binary_search_by_key(&number, |row| row.number) {
            Ok(at) => at,
            Err(at) => {
                self.rows.insert(
                    at,
                    Row {
                        number,
                        raw: None,
                        open: 0..0,
                        empty: false,
                        r: None,
                        cells: Vec::new(),
                    },
                );
                at
            }
        };
        &mut self.rows[at]
    }
}

impl Row {
    /// The cell at `column`, made with no XML when the row has none there.
    pub fn cell_mut(&mut self, column: u32) -> &mut Cell {
        let at = match self.cells.binary_search_by_key(&column, |cell| cell.column) {
            Ok(at) => at,
            Err(at) => {
                self.cells.insert(
                    at,
                    Cell {
                        column,
                        raw: None,
                        r: None,
                        style: None,
                        text: None,
                        formula: None,
                        formula_ref: None,
                        shared: None,
                        set: None,
                    },
                );
                at
            }
        };
        &mut self.cells[at]
    }
}

/// The placeholders in `text`: each `{{path}}`'s byte range and its path, trimmed.
pub fn placeholders(text: &str) -> Vec<(Range<usize>, &str)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find("{{") {
        let open = from + open;
        let Some(close) = text[open + 2..].find("}}") else {
            break;
        };
        let close = open + 2 + close;
        let path = text[open + 2..close].trim();
        if !path.is_empty()
            && path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
        {
            found.push((open..close + 2, path));
        }
        from = close + 2;
    }
    found
}
