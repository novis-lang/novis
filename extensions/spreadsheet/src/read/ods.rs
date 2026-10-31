//! ods, the OpenDocument spreadsheet: a zip package whose `content.xml` holds every sheet.
//!
//! **An ods is read whole.** [`read`] reads `content.xml` once, and gives each sheet's rows as the
//! [`RawRow`]s the xlsx cursor gives, so `super::Reader::value` reads every format the same way.
//! A cell carries its type and its value in attributes, so no style part is read: a date, a clock
//! time, a time of day and a duration are each given a serial and one of the styles [`KINDS`]
//! lists. A time value under 24 hours is a time of day, and a longer one a duration. A sheet is
//! hidden when its table style says `table:display="false"`, and a row when its `visibility` is
//! not `visible`.
//!
//! A row or a cell written once with a repeat count is that many rows or cells. An empty one only
//! moves the count on, so the million empty rows a file ends with cost nothing. A repeated cell
//! with a value is held once per column it covers, up to the grid's 1,048,576 rows and 16,384
//! columns, and a repeat past the grid is [`Error::Parse`]. A formula is OpenFormula text, which
//! `crate::openformula` translates to A1 text, so a cell read with `formulas: true` gives a
//! formula as an xlsx cell does. A formula the translator cannot read gives none.
//! A cell that spans columns or rows is a merged range, and the cells it covers follow it as
//! `covered-table-cell`s.

use std::sync::Arc;

use super::format::Kind;
use super::merge::Merge;
use super::sheet::{RawCell, RawRow};
use super::xml::{self, Token};
use super::{Listed, MOST_COLUMNS, MOST_ROWS, SheetKind, Source, days_from_civil, zip};
use crate::{Error, Visibility};

/// The read types the styles of an ods cell stand for, by index.
pub const KINDS: [Kind; 5] = [
    Kind::Plain,
    Kind::Date,
    Kind::Time,
    Kind::Clock,
    Kind::Duration,
];
const DATE: u32 = 1;
const TIME: u32 = 2;
const CLOCK: u32 = 3;
const DURATION: u32 = 4;

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file is not a valid ods file: {what}."))
}

fn repeat(attrs: &[u8], name: &[u8]) -> Result<u64, Error> {
    match xml::attr(attrs, name) {
        None => Ok(1),
        Some(raw) => std::str::from_utf8(raw)
            .ok()
            .and_then(|text| text.trim().parse::<u64>().ok())
            .filter(|&n| n > 0)
            .ok_or_else(|| broken("a repeat count is not a number")),
    }
}

/// The serial of the ISO date or date and time `text`, and whether it has a time.
fn date_serial(text: &str) -> Option<(f64, bool)> {
    let (day, time) = text.split_once('T').unwrap_or((text, ""));
    let mut parts = day.splitn(3, '-').map(|part| part.parse::<i64>().ok());
    let (year, month, date) = (parts.next()??, parts.next()??, parts.next()??);
    let days = days_from_civil(year, month, date) - days_from_civil(1899, 12, 30);
    // Excel counts a 29 February 1900 that did not exist, so a date before 1 March 1900 is one
    // serial lower than its count of days.
    let days = if days < 61 { days - 1 } else { days };
    let seconds = if time.is_empty() {
        0.0
    } else {
        let time = time.trim_end_matches('Z');
        let mut parts = time.splitn(3, ':').map(|part| part.parse::<f64>().ok());
        parts.next()?? * 3600.0 + parts.next()?? * 60.0 + parts.next().flatten().unwrap_or(0.0)
    };
    Some((days as f64 + seconds / 86_400.0, !time.is_empty()))
}

/// The seconds of the ISO duration `text`, such as `PT10H30M00S`.
fn duration_seconds(text: &str) -> Option<f64> {
    let (negative, text) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let text = text.strip_prefix('P')?;
    let (days, time) = text.split_once('T').unwrap_or((text, ""));
    let mut seconds = 0.0;
    if let Some(days) = days.strip_suffix('D') {
        seconds += days.parse::<f64>().ok()? * 86_400.0;
    } else if !days.is_empty() {
        return None;
    }
    let mut number = String::new();
    for c in time.chars() {
        let unit = match c {
            'H' => 3600.0,
            'M' => 60.0,
            'S' => 1.0,
            _ => {
                number.push(c);
                continue;
            }
        };
        seconds += number.parse::<f64>().ok()? * unit;
        number.clear();
    }
    Some(if negative { -seconds } else { seconds })
}

/// A cell being read: its attributes' value, the text of its paragraphs, and its repeat count.
#[derive(Default)]
struct Open {
    cell: Option<RawCell>,
    text: String,
    paragraphs: usize,
    repeat: u64,
}

/// The cell `attrs` describe, with its formula as A1 text and no column yet, or `None` for a
/// cell with no value type.
fn typed_cell(attrs: &[u8]) -> Result<Option<RawCell>, Error> {
    let formula =
        xml::attr_text(attrs, b"formula")?.and_then(|text| crate::openformula::to_a1(&text));
    Ok(value_cell(attrs)?.map(|cell| RawCell { formula, ..cell }))
}

/// The cell `attrs` describe by their value type alone.
fn value_cell(attrs: &[u8]) -> Result<Option<RawCell>, Error> {
    let value = |name: &[u8]| xml::attr_text(attrs, name);
    let number = |text: String, style: u32| RawCell {
        style,
        value: Some(text),
        ..RawCell::default()
    };
    if attrs
        .windows(26)
        .any(|window| window == b"calcext:value-type=\"error\"")
    {
        return Ok(Some(RawCell {
            kind: b"e".to_vec(),
            ..RawCell::default()
        }));
    }
    let Some(kind) = value(b"value-type")? else {
        return Ok(None);
    };
    Ok(Some(match kind.as_str() {
        "float" | "percentage" | "currency" => match value(b"value")? {
            Some(text) => number(text, 0),
            None => return Ok(None),
        },
        "date" => {
            let text = value(b"date-value")?.ok_or_else(|| broken("a date cell has no date"))?;
            let (serial, timed) =
                date_serial(&text).ok_or_else(|| broken("a date cell's date is not a date"))?;
            number(serial.to_string(), if timed { CLOCK } else { DATE })
        }
        "time" => {
            let text = value(b"time-value")?.ok_or_else(|| broken("a time cell has no time"))?;
            let seconds = duration_seconds(&text)
                .ok_or_else(|| broken("a time cell's time is not a duration"))?;
            let style = if (0.0..86_400.0).contains(&seconds) {
                TIME
            } else {
                DURATION
            };
            number((seconds / 86_400.0).to_string(), style)
        }
        "boolean" => {
            let truth = matches!(value(b"boolean-value")?.as_deref(), Some("true" | "1"));
            RawCell {
                kind: b"b".to_vec(),
                value: Some(if truth { "1" } else { "0" }.to_owned()),
                ..RawCell::default()
            }
        }
        // A string's text is its paragraphs, which the caller adds when the cell closes.
        _ => RawCell {
            kind: b"text".to_vec(),
            ..RawCell::default()
        },
    }))
}

/// A sheet being read.
struct Table {
    name: String,
    style: Option<String>,
    rows: Vec<RawRow>,
    next_row: u64,
    merged: Vec<Merge>,
}

/// Reads every sheet of the ods whose zip directory is `directory`.
pub fn read(source: &dyn Source, directory: &zip::Directory) -> Result<Vec<Listed>, Error> {
    let entry = directory
        .find("content.xml")
        .ok_or_else(|| broken("it has no content.xml"))?;
    let content = directory.whole(source, entry)?;
    let mut hidden_styles: Vec<String> = Vec::new();
    let mut style: Option<String> = None;
    let mut tables: Vec<Table> = Vec::new();
    let mut table: Option<Table> = None;
    let mut row: Option<(RawRow, u64, u64)> = None;
    let mut cell: Option<Open> = None;
    let mut at = 0;
    while let Some((token, size)) = xml::next(&content[at..])? {
        at += size;
        match token {
            Token::Open { name, attrs, empty } => match xml::local(name) {
                b"style" => style = xml::attr_text(attrs, b"name")?,
                b"table-properties" => {
                    if xml::attr(attrs, b"display") == Some(b"false") {
                        hidden_styles.extend(style.clone());
                    }
                }
                b"table" if table.is_none() => {
                    let name = xml::attr_text(attrs, b"name")?
                        .ok_or_else(|| broken("a sheet has no name"))?;
                    let next = Table {
                        name,
                        style: xml::attr_text(attrs, b"style-name")?,
                        rows: Vec::new(),
                        next_row: 1,
                        merged: Vec::new(),
                    };
                    if empty {
                        tables.push(next);
                    } else {
                        table = Some(next);
                    }
                }
                b"table-row" if table.is_some() => {
                    let count = repeat(attrs, b"number-rows-repeated")?;
                    let hidden = xml::attr(attrs, b"visibility").is_some_and(|v| v != b"visible");
                    let raw = RawRow {
                        hidden,
                        ..RawRow::default()
                    };
                    if empty {
                        if let Some(table) = table.as_mut() {
                            table.next_row = table.next_row.saturating_add(count);
                        }
                    } else {
                        row = Some((raw, count, 1));
                    }
                }
                b"table-cell" | b"covered-table-cell" if row.is_some() => {
                    let columns = repeat(attrs, b"number-columns-spanned")?;
                    let rows = repeat(attrs, b"number-rows-spanned")?;
                    if (columns > 1 || rows > 1)
                        && let (Some(table), Some((_, _, column))) = (table.as_mut(), row.as_ref())
                    {
                        table.merged.push(Merge::new(
                            (table.next_row, table.next_row.saturating_add(rows - 1)),
                            (*column, column.saturating_add(columns - 1)),
                        )?);
                    }
                    let open = Open {
                        cell: typed_cell(attrs)?,
                        repeat: repeat(attrs, b"number-columns-repeated")?,
                        ..Open::default()
                    };
                    if empty {
                        close_cell(&mut row, open)?;
                    } else {
                        cell = Some(open);
                    }
                }
                b"p" if !empty => {
                    if let Some(open) = cell.as_mut() {
                        if open.paragraphs > 0 {
                            open.text.push('\n');
                        }
                        open.paragraphs += 1;
                    }
                }
                b"s" => {
                    if let Some(open) = cell.as_mut() {
                        let count = repeat(attrs, b"c")?.min(1024);
                        open.text.extend(std::iter::repeat_n(' ', count as usize));
                    }
                }
                b"tab" => cell.iter_mut().for_each(|open| open.text.push('\t')),
                b"line-break" => cell.iter_mut().for_each(|open| open.text.push('\n')),
                _ => {}
            },
            Token::Text(raw) => {
                if let Some(open) = cell.as_mut() {
                    xml::decode(raw, &mut open.text)?;
                }
            }
            Token::Raw(raw) => {
                if let Some(open) = cell.as_mut() {
                    open.text.push_str(&String::from_utf8_lossy(raw));
                }
            }
            Token::Close(name) => match xml::local(name) {
                b"table-cell" | b"covered-table-cell" => {
                    if let Some(open) = cell.take() {
                        close_cell(&mut row, open)?;
                    }
                }
                b"table-row" => {
                    if let (Some((raw, count, _)), Some(table)) = (row.take(), table.as_mut()) {
                        close_row(table, raw, count)?;
                    }
                }
                b"table" => tables.extend(table.take()),
                _ => {}
            },
            Token::Skip => {}
        }
    }
    Ok(tables
        .into_iter()
        .map(|table| Listed {
            visibility: if table
                .style
                .as_ref()
                .is_some_and(|style| hidden_styles.contains(style))
            {
                Visibility::Hidden
            } else {
                Visibility::Visible
            },
            name: table.name,
            part: None,
            kind: SheetKind::Worksheet,
            rows: Some(Arc::new(table.rows)),
            merged: table.merged,
        })
        .collect())
}

/// Adds the cell `open` to the row being read, once per column it repeats over.
fn close_cell(row: &mut Option<(RawRow, u64, u64)>, open: Open) -> Result<(), Error> {
    let Some((raw, _, column)) = row.as_mut() else {
        return Ok(());
    };
    let first = *column;
    *column = column.saturating_add(open.repeat);
    let Some(mut cell) = open.cell else {
        return Ok(());
    };
    if cell.kind == b"text" || cell.kind == b"e" {
        cell.value = Some(open.text);
    }
    if *column - 1 > MOST_COLUMNS {
        return Err(broken("a cell is past the last column of the sheet"));
    }
    for at in first..*column {
        raw.cells.push(RawCell {
            column: at,
            kind: cell.kind.clone(),
            style: cell.style,
            value: cell.value.clone(),
            formula: cell.formula.clone(),
            ..RawCell::default()
        });
    }
    Ok(())
}

/// Adds the row `raw` to `table`, once per row it repeats over. A row with no value only moves
/// the count on.
fn close_row(table: &mut Table, raw: RawRow, count: u64) -> Result<(), Error> {
    let first = table.next_row;
    table.next_row = table.next_row.saturating_add(count);
    if raw.cells.is_empty() {
        return Ok(());
    }
    if table.next_row - 1 > MOST_ROWS {
        return Err(broken("a row is past the last row of the sheet"));
    }
    for number in first..table.next_row {
        table.rows.push(RawRow {
            number,
            hidden: raw.hidden,
            cells: raw.cells.clone(),
        });
    }
    Ok(())
}
