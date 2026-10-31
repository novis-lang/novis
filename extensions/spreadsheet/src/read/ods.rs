//! ods, the OpenDocument spreadsheet: a zip package whose `content.xml` holds every sheet.
//!
//! **An ods is read whole.** [`read`] reads `content.xml` once, and gives each sheet's rows as the
//! [`RawRow`]s the xlsx cursor gives, so `super::Reader::value` reads every format the same way.
//! A cell carries its type and its value in attributes: a date, a clock time, a time of day and
//! a duration are each given a serial and one of the kinds [`KINDS`] lists. A time value under 24
//! hours is a time of day, and a longer one a duration. A cell's text is rendered under the
//! file's own number style, which `super::ods_styles` reads from `styles.xml` and `content.xml`
//! as a format code, so a cell's style index names its kind and that code. A sheet is hidden when
//! its table style says `table:display="false"`, and a row when its `visibility` is not
//! `visible`.
//!
//! **The extras are read with the cells**, into [`Extras`]: a `text:a` in a cell is a link, an
//! `office:annotation` a note, and a `draw:frame` with a `draw:image` a picture at its cell. The
//! text of a note or a frame is not the cell's text. `table:named-range` and
//! `table:named-expression` are the names, local to a sheet when they stand inside its table,
//! and `table:database-range` is a table. The range LibreOffice keeps for a sheet's own filter,
//! `__Anonymous_Sheet_DB__0`, is not a table.
//!
//! A row or a cell written once with a repeat count is that many rows or cells. An empty one only
//! moves the count on, so the million empty rows a file ends with cost nothing. A repeated cell
//! with a value is held once per column it covers, up to the grid's 1,048,576 rows and 16,384
//! columns, and a repeat past the grid is [`Error::Parse`]. A formula is OpenFormula text, which
//! `crate::openformula` translates to A1 text, so a cell read with `formulas: true` gives a
//! formula as an xlsx cell does. A formula the translator cannot read gives none.
//! A cell that spans columns or rows is a merged range, and the cells it covers follow it as
//! `covered-table-cell`s.

use std::collections::HashMap;
use std::sync::Arc;

use super::extras::{Link, Name, Note, Table as DataRange};
use super::format::{Kind, Style};
use super::merge::Merge;
use super::sheet::{RawCell, RawRow};
use super::xml::{self, Token};
use super::{
    Listed, MOST_COLUMNS, MOST_ROWS, SheetKind, Source, days_from_civil, letters, ods_styles, zip,
};
use crate::{Error, Visibility};

/// The read types of an ods cell, by its style's index. The styles after these are a kind with
/// the code of a number style the file defines.
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
    /// The last column of each run of columns, and the cell style it gives a cell with none.
    columns: Vec<(u64, Option<String>)>,
}

/// What an ods has beside its cells, each with the name of its sheet where it has one.
#[derive(Debug, Default)]
pub struct Extras {
    pub links: Vec<(String, Link)>,
    pub notes: Vec<(String, Note)>,
    /// Each picture's sheet, its cell, and the package part or the URL it names.
    pub images: Vec<(String, String, String)>,
    pub names: Vec<Name>,
    pub tables: Vec<DataRange>,
}

/// An ods read whole: its sheets, the styles their cells' indexes name, and its extras.
pub struct Book {
    pub sheets: Vec<Listed>,
    pub styles: Vec<Style>,
    pub extras: Extras,
}

/// Text inside a cell that is not the cell's own: a note, with its author and its paragraphs,
/// or a frame.
enum Aside {
    Note {
        author: Option<String>,
        text: String,
        paragraphs: usize,
        /// Whether the text read now is the author's, or a date's, which is not kept.
        field: Option<bool>,
    },
    Frame,
}

/// The text and the paragraph count that text read now goes to: the note's, or the cell's.
fn target<'a>(
    cell: &'a mut Option<Open>,
    aside: &'a mut Option<Aside>,
) -> Option<(&'a mut String, &'a mut usize)> {
    match aside {
        Some(Aside::Note {
            text,
            paragraphs,
            field: None,
            ..
        }) => Some((text, paragraphs)),
        Some(_) => None,
        None => cell
            .as_mut()
            .map(|open| (&mut open.text, &mut open.paragraphs)),
    }
}

/// A sheet name as A1 text writes it: quoted when it is not one word.
fn sheet_ref(sheet: &str) -> String {
    let word = !sheet.is_empty()
        && !sheet.starts_with(|c: char| c.is_ascii_digit())
        && sheet.chars().all(|c| c.is_alphanumeric() || c == '_');
    if word {
        sheet.to_owned()
    } else {
        format!("'{}'", sheet.replace('\'', "''"))
    }
}

/// The sheet and the range of the ods address `text`, such as `$Shop.$A$1:.$B$3` or
/// `Shop.A1:Shop.B3`, with each cell's `$` kept when `dollars` is set. An address with no sheet
/// is `None`.
fn address(text: &str, dollars: bool) -> Option<(String, String)> {
    let mut parts = Vec::new();
    let mut quoted = false;
    let mut start = 0;
    for (at, c) in text.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            ':' if !quoted => {
                parts.push(&text[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    let mut sheet: Option<String> = None;
    let mut cells = Vec::new();
    for part in parts {
        let (name, cell) = part.trim().rsplit_once('.')?;
        let name = name.trim_start_matches('$');
        if !name.is_empty() && sheet.is_none() {
            sheet = Some(
                match name.strip_prefix('\'').and_then(|n| n.strip_suffix('\'')) {
                    Some(inner) => inner.replace("''", "'"),
                    None => name.to_owned(),
                },
            );
        }
        cells.push(if dollars {
            cell.to_owned()
        } else {
            cell.replace('$', "")
        });
    }
    Some((sheet?, cells.join(":")))
}

/// The name `attrs` describe, a `table:named-range` or a `table:named-expression`, local to
/// `sheet` when it stands inside one.
fn defined_name(attrs: &[u8], sheet: Option<&Table>) -> Result<Option<Name>, Error> {
    let Some(name) = xml::attr_text(attrs, b"name")? else {
        return Ok(None);
    };
    let reference = if let Some(range) = xml::attr_text(attrs, b"cell-range-address")? {
        match address(&range, true) {
            Some((sheet, range)) => format!("{}!{range}", sheet_ref(&sheet)),
            None => range,
        }
    } else {
        let expression = xml::attr_text(attrs, b"expression")?.unwrap_or_default();
        crate::openformula::to_a1(&expression).unwrap_or(expression)
    };
    Ok(Some(Name {
        name,
        reference,
        sheet: sheet.map(|table| table.name.clone()),
    }))
}

/// The table a `table:database-range` describes, or `None` for a sheet's own filter range.
fn database_range(attrs: &[u8]) -> Result<Option<DataRange>, Error> {
    let name = xml::attr_text(attrs, b"name")?.unwrap_or_default();
    if name.starts_with("__Anonymous_Sheet_DB__") {
        return Ok(None);
    }
    let range = xml::attr_text(attrs, b"target-range-address")?.unwrap_or_default();
    Ok(address(&range, false).map(|(sheet, table)| DataRange { table, name, sheet }))
}

/// The style index of a cell of kind `kind` whose number style has the code `code`: the kind's
/// own index with no code, and otherwise the one `styles` keeps for that pair.
fn style_index(
    kind: u32,
    code: Option<&Arc<str>>,
    styles: &mut Vec<Style>,
    interned: &mut HashMap<(u32, Arc<str>), u32>,
) -> u32 {
    let Some(code) = code else {
        return kind;
    };
    *interned.entry((kind, Arc::clone(code))).or_insert_with(|| {
        styles.push(Style {
            kind: KINDS[kind as usize],
            code: Arc::clone(code),
        });
        u32::try_from(styles.len() - 1).unwrap_or(u32::MAX)
    })
}

/// Reads every sheet of the ods whose zip directory is `directory`, with its styles and extras.
pub fn read(source: &dyn Source, directory: &zip::Directory) -> Result<Book, Error> {
    let entry = directory
        .find("content.xml")
        .ok_or_else(|| broken("it has no content.xml"))?;
    let content = directory.whole(source, entry)?;
    let mut found = ods_styles::Styles::default();
    if let Some(entry) = directory.find("styles.xml") {
        found.add(&directory.whole(source, entry)?)?;
    }
    found.add(&content)?;
    let codes = found.codes();
    let mut styles: Vec<Style> = KINDS.map(Style::of_kind).to_vec();
    let mut interned: HashMap<(u32, Arc<str>), u32> = HashMap::new();
    let mut extras = Extras::default();
    let mut hidden_styles: Vec<String> = Vec::new();
    let mut style: Option<String> = None;
    let mut tables: Vec<Table> = Vec::new();
    let mut table: Option<Table> = None;
    let mut row: Option<(RawRow, u64, u64)> = None;
    let mut row_style: Option<String> = None;
    let mut cell: Option<Open> = None;
    // The cell being read's address, a link in it with where its text starts, and a note or a
    // frame in it.
    let mut here = String::new();
    let mut link: Option<(Link, usize)> = None;
    let mut aside: Option<Aside> = None;
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
                        columns: Vec::new(),
                    };
                    if empty {
                        tables.push(next);
                    } else {
                        table = Some(next);
                    }
                }
                b"table-column" => {
                    if let Some(table) = table.as_mut() {
                        let count = repeat(attrs, b"number-columns-repeated")?;
                        let last = table.columns.last().map_or(0, |&(last, _)| last);
                        let style = xml::attr_text(attrs, b"default-cell-style-name")?;
                        table.columns.push((last.saturating_add(count), style));
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
                        row_style = xml::attr_text(attrs, b"default-cell-style-name")?;
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
                    let mut typed = typed_cell(attrs)?;
                    if let (Some(typed), Some(table), Some((_, _, column))) =
                        (typed.as_mut(), table.as_ref(), row.as_ref())
                    {
                        let named = match xml::attr_text(attrs, b"style-name")? {
                            Some(name) => Some(name),
                            None => row_style.clone().or_else(|| {
                                table
                                    .columns
                                    .iter()
                                    .find(|&&(last, _)| last >= *column)
                                    .and_then(|(_, name)| name.clone())
                            }),
                        };
                        let code = named.and_then(|name| codes.get(&name));
                        typed.style = style_index(typed.style, code, &mut styles, &mut interned);
                    }
                    let open = Open {
                        cell: typed,
                        repeat: repeat(attrs, b"number-columns-repeated")?,
                        ..Open::default()
                    };
                    if empty {
                        close_cell(&mut row, open)?;
                    } else {
                        if let (Some(table), Some((_, _, column))) = (table.as_ref(), row.as_ref())
                        {
                            here = format!("{}{}", letters(*column), table.next_row);
                        }
                        cell = Some(open);
                    }
                }
                b"annotation" if cell.is_some() && aside.is_none() && !empty => {
                    aside = Some(Aside::Note {
                        author: None,
                        text: String::new(),
                        paragraphs: 0,
                        field: None,
                    });
                }
                b"creator" | b"date" | b"date-string" if !empty => {
                    if let Some(Aside::Note { field, .. }) = aside.as_mut() {
                        *field = Some(xml::local(name) == b"creator");
                    }
                }
                b"frame" if cell.is_some() && aside.is_none() && !empty => {
                    aside = Some(Aside::Frame);
                }
                b"image" if matches!(aside, Some(Aside::Frame)) => {
                    if let (Some(href), Some(table)) =
                        (xml::attr_text(attrs, b"href")?, table.as_ref())
                    {
                        extras.images.push((table.name.clone(), here.clone(), href));
                    }
                }
                b"a" if aside.is_none() && !empty => {
                    if let (Some(open), Some(href)) =
                        (cell.as_ref(), xml::attr_text(attrs, b"href")?)
                    {
                        let (target, external) = match href.strip_prefix('#') {
                            Some(place) => (
                                address(place, true).map_or_else(
                                    || place.to_owned(),
                                    |(sheet, range)| format!("{}!{range}", sheet_ref(&sheet)),
                                ),
                                false,
                            ),
                            None => (href, true),
                        };
                        let found = Link {
                            anchor: here.clone(),
                            target,
                            text: None,
                            tooltip: xml::attr_text(attrs, b"title")?,
                            external,
                        };
                        link = Some((found, open.text.len()));
                    }
                }
                b"named-range" | b"named-expression" => {
                    extras.names.extend(defined_name(attrs, table.as_ref())?);
                }
                b"database-range" => extras.tables.extend(database_range(attrs)?),
                b"p" if !empty => {
                    if let Some((text, paragraphs)) = target(&mut cell, &mut aside) {
                        if *paragraphs > 0 {
                            text.push('\n');
                        }
                        *paragraphs += 1;
                    }
                }
                b"s" => {
                    if let Some((text, _)) = target(&mut cell, &mut aside) {
                        let count = repeat(attrs, b"c")?.min(1024);
                        text.extend(std::iter::repeat_n(' ', count as usize));
                    }
                }
                b"tab" => target(&mut cell, &mut aside)
                    .into_iter()
                    .for_each(|(text, _)| text.push('\t')),
                b"line-break" => target(&mut cell, &mut aside)
                    .into_iter()
                    .for_each(|(text, _)| text.push('\n')),
                _ => {}
            },
            Token::Text(raw) => match aside.as_mut() {
                Some(Aside::Note {
                    author,
                    field: Some(true),
                    ..
                }) => xml::decode(raw, author.get_or_insert_default())?,
                _ => {
                    if let Some((text, _)) = target(&mut cell, &mut aside) {
                        xml::decode(raw, text)?;
                    }
                }
            },
            Token::Raw(raw) => {
                if let Some((text, _)) = target(&mut cell, &mut aside) {
                    text.push_str(&String::from_utf8_lossy(raw));
                }
            }
            Token::Close(name) => match xml::local(name) {
                b"creator" | b"date" | b"date-string" => {
                    if let Some(Aside::Note { field, .. }) = aside.as_mut() {
                        *field = None;
                    }
                }
                b"annotation" => {
                    if let (Some(Aside::Note { author, text, .. }), Some(table)) =
                        (aside.take(), table.as_ref())
                    {
                        let note = Note {
                            noted: here.clone(),
                            note: text,
                            author: author.filter(|author| !author.is_empty()),
                        };
                        extras.notes.push((table.name.clone(), note));
                    }
                }
                b"frame" => aside = None,
                b"a" if aside.is_none() => {
                    if let (Some((mut found, start)), Some(open), Some(table)) =
                        (link.take(), cell.as_ref(), table.as_ref())
                    {
                        found.text = open
                            .text
                            .get(start..)
                            .filter(|text| !text.is_empty())
                            .map(str::to_owned);
                        extras.links.push((table.name.clone(), found));
                    }
                }
                b"table-cell" | b"covered-table-cell" => {
                    aside = None;
                    link = None;
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
    let sheets = tables
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
        .collect();
    Ok(Book {
        sheets,
        styles,
        extras,
    })
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
