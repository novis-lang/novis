//! Template fill: a workbook designed in Excel, copied part by part with its placeholders filled
//! and the rows of each list repeated (`rule:core-classes/spreadsheet-template-fill`).
//!
//! [`Template::open`] reads the zip directory, the workbook's sheets and defined names, the
//! shared strings and every worksheet ([`sheet`]), and nothing else. [`Template::set`] fills one
//! cell by address. [`Template::fill`] takes every value that is not in a list and the names of
//! the lists, finds each list's block, and returns the lists in the order their blocks stand.
//! A block is the rows a defined name of the list's name covers, or else the one row whose
//! placeholders go through the list. A block inside another, or a row with two lists, is a list
//! inside a list and throws.
//!
//! **Each sheet with blocks streams, in workbook order.** The blocks are ordered by sheet and
//! then by row, and each sheet with blocks is a [`Page`]. `fill` writes the first page's sheet up
//! to its first block, each [`Template::items`] writes its batch's copies, and a batch for a
//! later page first ends the pages before it: the rest of each sheet, its rows below moved down.
//! [`Template::finish`] ends the last page and then writes every other part. A part `finish`
//! writes is copied as it is, or rewritten with each repeated block's [`shift::Shift`] applied
//! to its references: the worksheets, the workbook's defined names, tables, drawings' anchors and
//! charts. A sheet cannot be written before its lists are known when a formula above a block
//! names a row the block moves, or when a formula or part on it names a row a later page moves.
//! Such a page keeps its items, records its moves once the fill passes it, and is written at
//! `finish`: the memory of every item, still under the request's cap. An item whose copy would
//! end past row 1,048,576 throws naming its list, before any of its rows is written.
//!
//! **What changes besides the cells.** `calcChain.xml` names cells by position, so it is
//! dropped, and the workbook recalculates on open. A sheet with a block loses its `dimension`.
//! An xlsm template gives xlsx: its VBA parts and their relationships and content types are
//! dropped, and the workbook's content type is the xlsx one
//! (`rule:core-classes/spreadsheet-output-is-inert`).
//!
//! What it spends, per filling request: the zip directory, the shared strings, every worksheet's
//! XML, one batch of items (and every item of each sheet that is kept), and one part at a time
//! at `finish`.

pub mod sheet;
pub mod shift;

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::read::Source;
use crate::read::xml::{self, Token};
use crate::read::zip::{Bound, Directory, Part};
use crate::zip::Archive;
use crate::{Error, Finished, SheetRows, Value};
use sheet::{Cell, Row, Sheet, placeholders};
use shift::{RowRef, Shift};

/// One placeholder's path and its value, `None` for `null`.
pub type Placeholder = (String, Option<Value>);

/// A list's repeated rows.
#[derive(Debug, Clone)]
struct Block {
    list: String,
    top: u32,
    bottom: u32,
    items: u32,
    written: u32,
    held: Vec<Vec<Placeholder>>,
}

impl Block {
    fn height(&self) -> u32 {
        self.bottom - self.top + 1
    }
}

/// A sheet with blocks: its blocks are `first..end` of the fill's, and its shifts start at
/// `base` of the fill's once its first block closes.
#[derive(Debug, Clone, Copy)]
struct Page {
    sheet: usize,
    first: usize,
    end: usize,
    stream: bool,
    base: usize,
}

/// A fill under way.
struct Filling {
    values: BTreeMap<String, Option<Value>>,
    /// Every block, in sheet order and then row order, and the sheets they are on.
    blocks: Vec<Block>,
    pages: Vec<Page>,
    /// The page the fill is on, the block being written, and the next template row to write.
    page: usize,
    current: usize,
    next_row: usize,
    /// Every closed block's move, each page's in a run from its `base`.
    shifts: Vec<Shift>,
    /// The next shared formula group a copy of a block may take on the sheet being written.
    next_group: u32,
    /// The rows written to the sheet being written, and every sheet written so far.
    rows_out: u64,
    rows: Vec<SheetRows>,
    archive: Archive,
}

enum State {
    Open,
    Filling(Box<Filling>),
    Finished,
}

/// A designed workbook being filled.
pub struct Template {
    directory: Directory,
    sheets: Vec<Sheet>,
    names: Vec<(String, String)>,
    state: State,
}

impl std::fmt::Debug for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Template")
            .field("sheets", &self.sheets.len())
            .finish_non_exhaustive()
    }
}

fn invalid(message: String) -> Error {
    Error::Invalid(message)
}

fn broken(what: &str) -> Error {
    Error::Parse(format!(
        "The template is not a valid xlsx or xlsm file: {what}."
    ))
}

fn text_of(bytes: Vec<u8>, path: &str) -> Result<String, Error> {
    String::from_utf8(bytes).map_err(|_| broken(&format!("the part `{path}` is not UTF-8")))
}

fn read(directory: &Directory, source: &dyn Source, path: &str) -> Result<Option<String>, Error> {
    directory
        .find(path)
        .map(|entry| text_of(directory.whole(source, entry)?, path))
        .transpose()
}

/// The part a relationship's `target` names, from a part in `folder`.
fn resolve(folder: &str, target: &str) -> String {
    let mut parts: Vec<&str> = if let Some(rooted) = target.strip_prefix('/') {
        return rooted.to_owned();
    } else {
        folder.split('/').filter(|part| !part.is_empty()).collect()
    };
    for piece in target.split('/') {
        match piece {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            piece => parts.push(piece),
        }
    }
    parts.join("/")
}

fn folder(path: &str) -> &str {
    path.rfind('/').map_or("", |at| &path[..at])
}

fn rels_of(path: &str) -> String {
    let (folder, file) = path.rsplit_once('/').unwrap_or(("", path));
    format!("{folder}/_rels/{file}.rels")
}

/// Each relationship of `rels`: its id and the part it names.
fn relationships(rels: &str, from: &str) -> Result<Vec<(String, String)>, Error> {
    let mut found = Vec::new();
    let bytes = rels.as_bytes();
    let mut at = 0;
    while let Some((token, length)) = xml::next(&bytes[at..])? {
        at += length;
        if let Token::Open { name, attrs, .. } = token
            && xml::local(name) == b"Relationship"
            && xml::attr(attrs, b"TargetMode") != Some(b"External")
            && let (Some(id), Some(target)) = (
                xml::attr_text(attrs, b"Id")?,
                xml::attr_text(attrs, b"Target")?,
            )
        {
            found.push((id, resolve(folder(from), &target)));
        }
    }
    Ok(found)
}

/// A part the fill leaves out of what it writes.
fn dropped(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "xl/calcchain.xml" || lower.contains("vbaproject")
}

/// The text a value shows inside other text.
fn as_text(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Text(text) | Value::Decimal(text) => out.push_str(text),
        Value::Number(number) => crate::cells::number(&mut out, *number),
        Value::Integer(integer) => out.push_str(itoa::Buffer::new().format(*integer)),
        Value::Boolean(value) => out.push_str(if *value { "TRUE" } else { "FALSE" }),
        Value::Date(date) => {
            let _ = write!(out, "{:04}-{:02}-{:02}", date.year, date.month, date.day);
        }
        Value::Time(time) => {
            let _ = write!(
                out,
                "{:02}:{:02}:{:02}",
                time.hour, time.minute, time.second
            );
        }
        Value::Clock(clock) => {
            let (d, t) = (clock.day, clock.clock);
            let _ = write!(
                out,
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                d.year, d.month, d.day, t.hour, t.minute, t.second
            );
        }
        Value::Duration(nanos) => {
            let seconds = nanos / 1_000_000_000;
            let _ = write!(
                out,
                "{}:{:02}:{:02}",
                seconds / 3600,
                seconds % 3600 / 60,
                seconds % 60
            );
        }
        Value::Formula {
            cached_text,
            cached_number,
            ..
        } => match (cached_text, cached_number) {
            (Some(text), _) => out.push_str(text),
            (None, Some(number)) => crate::cells::number(&mut out, *number),
            (None, None) => {}
        },
        Value::Rich(runs) => runs.iter().for_each(|run| out.push_str(&run.text)),
    }
    out
}

/// Appends a cell at `reference` with the template's `style`, holding `value` by the write table.
fn value_cell(
    out: &mut String,
    reference: &str,
    style: Option<&str>,
    value: Option<&Value>,
) -> Result<(), Error> {
    let _ = write!(out, "<c r=\"{reference}\"");
    if let Some(style) = style {
        out.push_str(" s=\"");
        crate::xml::attr(out, style);
        out.push('"');
    }
    let text_cell = |out: &mut String, text: &str| {
        out.push_str(" t=\"inlineStr\"><is>");
        crate::cells::run_text(out, text);
        out.push_str("</is></c>");
    };
    let Some(value) = value else {
        out.push_str("/>");
        return Ok(());
    };
    match value {
        Value::Text(text) => text_cell(out, text),
        Value::Rich(_) => text_cell(out, &as_text(value)),
        Value::Boolean(value) => {
            let _ = write!(out, " t=\"b\"><v>{}</v></c>", u8::from(*value));
        }
        Value::Formula {
            formula,
            cached_text,
            cached_number,
        } => {
            if cached_text.is_some() {
                out.push_str(" t=\"str\"");
            }
            out.push_str("><f>");
            crate::xml::text(out, formula);
            out.push_str("</f>");
            if let Some(cached) = cached_text {
                out.push_str("<v>");
                crate::xml::text(out, cached);
                out.push_str("</v>");
            } else if let Some(cached) = cached_number.filter(|n| n.is_finite()) {
                out.push_str("<v>");
                crate::cells::number(out, cached);
                out.push_str("</v>");
            }
            out.push_str("</c>");
        }
        Value::Integer(_) | Value::Decimal(_) if crate::cells::stored_number(value).is_none() => {
            if let Value::Decimal(text) = value
                && text.parse::<f64>().is_err()
            {
                return Err(invalid(format!(
                    "The cell `{reference}` gets the decimal `{text}`, which is not a number."
                )));
            }
            text_cell(out, &as_text(value));
        }
        _ => {
            let (number, _) = crate::cells::stored_number(value).ok_or_else(|| {
                invalid(format!(
                    "The cell `{reference}` gets a value that has no place in a spreadsheet cell."
                ))
            })?;
            out.push_str("><v>");
            crate::cells::number(out, number);
            out.push_str("</v></c>");
        }
    }
    Ok(())
}

/// Every row `text` names on any sheet, with the sheet it is on.
fn rows_named(text: &str, home: &str) -> Vec<(String, u32)> {
    let mut found = Vec::new();
    shift::rewrite(text, home, &mut |at: RowRef<'_>| {
        found.push((at.sheet.to_owned(), at.row));
        at.row
    });
    found
}

/// Copies `xml[range]` to `out` with each edit's range replaced by its text. The edits are in
/// order and inside the range.
fn splice(
    out: &mut String,
    xml: &str,
    range: std::ops::Range<usize>,
    edits: &mut [(std::ops::Range<usize>, String)],
) {
    edits.sort_by_key(|(at, _)| at.start);
    let mut copied = range.start;
    for (at, text) in edits.iter() {
        out.push_str(&xml[copied..at.start]);
        out.push_str(text);
        copied = at.end;
    }
    out.push_str(&xml[copied..range.end]);
}

/// The attributes holding references, and the elements whose text is a formula.
const REF_ATTRS: [&[u8]; 4] = [b"ref", b"sqref", b"activeCell", b"topLeftCell"];
const FORMULA_ELEMENTS: [&[u8]; 6] = [
    b"f",
    b"formula",
    b"formula1",
    b"formula2",
    b"definedName",
    b"calculatedColumnFormula",
];

/// A part's XML with every reference in it moved by `shifts`, `home` the sheet its plain
/// references are on. A drawing's anchor rows move too.
fn rewrite_part(text: &str, home: &str, shifts: &[Shift]) -> Result<String, Error> {
    if shifts.is_empty() {
        return Ok(text.to_owned());
    }
    let bytes = text.as_bytes();
    let mut edits = Vec::new();
    let mut stack: Vec<&[u8]> = Vec::new();
    let mut at = 0;
    while let Some((token, length)) = xml::next(&bytes[at..])? {
        at += length;
        match token {
            Token::Open { name, attrs, empty } => {
                for attr in REF_ATTRS {
                    if let Some(value) = xml::attr(attrs, attr) {
                        let raw = String::from_utf8_lossy(value);
                        let moved = shift::apply_list(&raw, home, shifts);
                        if moved != raw {
                            let from = value.as_ptr() as usize - bytes.as_ptr() as usize;
                            edits.push((from..from + value.len(), moved));
                        }
                    }
                }
                if !empty {
                    stack.push(xml::local(name));
                }
            }
            Token::Close(_) => {
                stack.pop();
            }
            Token::Text(raw) => {
                let from = raw.as_ptr() as usize - bytes.as_ptr() as usize;
                let inner = stack.last().copied().unwrap_or_default();
                let parent = stack
                    .len()
                    .checked_sub(2)
                    .map(|at| stack[at])
                    .unwrap_or_default();
                if inner == b"sqref" {
                    // An `x14` conditional format's `xm:sqref` holds its ranges as text.
                    let raw = String::from_utf8_lossy(raw);
                    let moved = shift::apply_list(&raw, home, shifts);
                    if moved != raw {
                        edits.push((from..from + raw.len(), moved));
                    }
                } else if FORMULA_ELEMENTS.contains(&inner) {
                    let mut decoded = String::new();
                    xml::decode(raw, &mut decoded)?;
                    let moved = shift::apply(&decoded, home, shifts);
                    if moved != decoded {
                        let mut escaped = String::new();
                        crate::xml::text(&mut escaped, &moved);
                        edits.push((from..from + raw.len(), escaped));
                    }
                } else if inner == b"row"
                    && (parent == b"from" || parent == b"to")
                    && let Ok(row) = String::from_utf8_lossy(raw).trim().parse::<u32>()
                {
                    let end = if parent == b"from" {
                        shift::End::Single
                    } else {
                        shift::End::End
                    };
                    let moved = shifts.iter().fold(row + 1, |row, shift| {
                        shift.row(RowRef {
                            sheet: home,
                            row,
                            absolute: false,
                            end,
                        })
                    }) - 1;
                    if moved != row {
                        edits.push((from..from + raw.len(), moved.to_string()));
                    }
                }
            }
            Token::Raw(_) | Token::Skip => {}
        }
    }
    let mut out = String::with_capacity(text.len());
    splice(&mut out, text, 0..text.len(), &mut edits);
    Ok(out)
}

/// `text` without the self-closing elements whose attributes hold any of `needles`, and with
/// each `(from, to)` in `swaps` replaced.
fn without(text: &str, needles: &[&str], swaps: &[(&str, &str)]) -> Result<String, Error> {
    let bytes = text.as_bytes();
    let mut edits = Vec::new();
    let mut at = 0;
    while let Some((token, length)) = xml::next(&bytes[at..])? {
        let start = at;
        at += length;
        if let Token::Open {
            attrs, empty: true, ..
        } = token
        {
            let attrs = String::from_utf8_lossy(attrs).to_ascii_lowercase();
            if needles.iter().any(|needle| attrs.contains(needle)) {
                edits.push((start..at, String::new()));
            }
        }
    }
    let mut out = String::with_capacity(text.len());
    splice(&mut out, text, 0..text.len(), &mut edits);
    for (from, to) in swaps {
        out = out.replace(from, to);
    }
    Ok(out)
}

/// The workbook's XML set to recalculate on open.
fn recalculating(workbook: &str) -> String {
    if let Some(at) = workbook.find("<calcPr") {
        let close = at + workbook[at..].find('>').unwrap_or(0);
        if workbook[at..close].contains("fullCalcOnLoad") {
            return workbook.to_owned();
        }
        let insert = if workbook[..close].ends_with('/') {
            close - 1
        } else {
            close
        };
        return format!(
            "{} fullCalcOnLoad=\"1\"{}",
            &workbook[..insert],
            &workbook[insert..]
        );
    }
    for after in ["</definedNames>", "</sheets>"] {
        if let Some(at) = workbook.find(after) {
            let at = at + after.len();
            return format!(
                "{}<calcPr fullCalcOnLoad=\"1\"/>{}",
                &workbook[..at],
                &workbook[at..]
            );
        }
    }
    workbook.to_owned()
}

impl Template {
    /// Opens the template `source` is: an xlsx or xlsm file, its parts read under `bound`.
    pub fn open(source: &dyn Source, bound: Bound) -> Result<Self, Error> {
        let directory = Directory::read(source, bound)?;
        let workbook = read(&directory, source, "xl/workbook.xml")?
            .ok_or_else(|| broken("it has no workbook part"))?;
        let rels = read(&directory, source, "xl/_rels/workbook.xml.rels")?.unwrap_or_default();
        let targets = relationships(&rels, "xl/workbook.xml")?;
        let strings = match read(&directory, source, "xl/sharedStrings.xml")? {
            Some(text) => shared_strings(&text)?,
            None => Vec::new(),
        };
        let bytes = workbook.as_bytes();
        let mut sheets = Vec::new();
        let mut names = Vec::new();
        let mut at = 0;
        let mut naming: Option<String> = None;
        while let Some((token, length)) = xml::next(&bytes[at..])? {
            at += length;
            match token {
                Token::Open { name, attrs, .. } if xml::local(name) == b"sheet" => {
                    let (Some(sheet), Some(id)) = (
                        xml::attr_text(attrs, b"name")?,
                        xml::attr_text(attrs, b"id")?,
                    ) else {
                        continue;
                    };
                    let Some((_, path)) = targets.iter().find(|(rel, _)| *rel == id) else {
                        continue;
                    };
                    if !path.to_ascii_lowercase().starts_with("xl/worksheets/") {
                        continue;
                    }
                    let xml = read(&directory, source, path)?
                        .ok_or_else(|| broken(&format!("the sheet `{sheet}` has no part")))?;
                    sheets.push(Sheet::parse(sheet, path.clone(), xml, &strings)?);
                }
                Token::Open {
                    name,
                    attrs,
                    empty: false,
                } if xml::local(name) == b"definedName" => {
                    naming = xml::attr_text(attrs, b"name")?;
                }
                Token::Text(raw) => {
                    if let Some(name) = naming.take() {
                        let mut text = String::new();
                        xml::decode(raw, &mut text)?;
                        names.push((name, text));
                    }
                }
                Token::Close(_) => naming = None,
                _ => {}
            }
        }
        if sheets.is_empty() {
            return Err(broken("it has no worksheet"));
        }
        Ok(Self {
            directory,
            sheets,
            names,
            state: State::Open,
        })
    }

    fn sheet_named(&self, name: &str) -> Option<usize> {
        self.sheets
            .iter()
            .position(|sheet| sheet.name.eq_ignore_ascii_case(name))
    }

    /// Fills the cell at `address`, such as `Summary!B2` or `B2` on the first sheet, with
    /// `value`. The cell keeps its style.
    pub fn set(&mut self, address: &str, value: Option<Value>) -> Result<(), Error> {
        if !matches!(self.state, State::Open) {
            return Err(invalid(
                "`set` fills a cell before `fill`. This template is already filled.".to_owned(),
            ));
        }
        let (sheet, cell) = match address.rsplit_once('!') {
            Some((sheet, cell)) => (sheet.trim_matches('\''), cell),
            None => (self.sheets[0].name.as_str(), address),
        };
        let index = self.sheet_named(sheet).ok_or_else(|| {
            invalid(format!(
                "The template has no sheet `{sheet}`, which `{address}` names."
            ))
        })?;
        let (column, row) = sheet::address(&cell.replace('$', "")).ok_or_else(|| {
            invalid(format!(
                "`{address}` is not a cell address, such as `Summary!B2`."
            ))
        })?;
        self.sheets[index].row_mut(row).cell_mut(column).set = Some(value);
        Ok(())
    }

    /// Finds each list's block, and returns each with the sheet it is on, in sheet order and
    /// then row order.
    fn blocks(&self, lists: &[String]) -> Result<Vec<(usize, Block)>, Error> {
        let mut blocks: Vec<(usize, Block)> = Vec::new();
        for list in lists {
            let prefix = format!("{list}.");
            let named = self
                .names
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(list));
            let found = if let Some((_, reference)) = named {
                let rows = rows_named(reference, "");
                let Some((sheet, _)) = rows.first() else {
                    return Err(invalid(format!(
                        "The defined name `{list}` is `{reference}`, which names no rows."
                    )));
                };
                let index = self.sheet_named(sheet).ok_or_else(|| {
                    invalid(format!("The defined name `{list}` names the sheet `{sheet}`, which the template does not have."))
                })?;
                let top = rows.iter().map(|(_, row)| *row).min().unwrap_or(1);
                let bottom = rows.iter().map(|(_, row)| *row).max().unwrap_or(1);
                Some((index, top, bottom))
            } else {
                let mut found: Option<(usize, u32)> = None;
                for (index, sheet) in self.sheets.iter().enumerate() {
                    for row in &sheet.rows {
                        let uses = row
                            .cells
                            .iter()
                            .filter_map(|cell| cell.text.as_deref())
                            .any(|text| {
                                placeholders(text)
                                    .iter()
                                    .any(|(_, path)| path.starts_with(&prefix))
                            });
                        if !uses {
                            continue;
                        }
                        if let Some((other, at)) = found {
                            return Err(invalid(format!(
                                "The list `{list}` is on the rows `{}!{at}` and `{}!{}`. Give the rows a defined name `{list}` to repeat them as one block.",
                                self.sheets[other].name, sheet.name, row.number
                            )));
                        }
                        found = Some((index, row.number));
                    }
                }
                found.map(|(index, row)| (index, row, row))
            };
            if let Some((sheet, top, bottom)) = found {
                blocks.push((
                    sheet,
                    Block {
                        list: list.clone(),
                        top,
                        bottom,
                        items: 0,
                        written: 0,
                        held: Vec::new(),
                    },
                ));
            }
        }
        blocks.sort_by_key(|(sheet, block)| (*sheet, block.top));
        for pair in blocks.windows(2) {
            if pair[0].0 == pair[1].0 && pair[1].1.top <= pair[0].1.bottom {
                return Err(invalid(format!(
                    "The list `{}` is inside the list `{}`. A template cannot repeat a list inside a list.",
                    pair[1].1.list, pair[0].1.list
                )));
            }
        }
        for (index, sheet) in self.sheets.iter().enumerate() {
            for row in &sheet.rows {
                let Some((_, block)) = blocks.iter().find(|(on, block)| {
                    *on == index && (block.top..=block.bottom).contains(&row.number)
                }) else {
                    continue;
                };
                for cell in &row.cells {
                    if cell.set.is_some() {
                        return Err(invalid(format!(
                            "`set` fills a cell in the repeated rows of the list `{}`, on row {}.",
                            block.list, row.number
                        )));
                    }
                    for (_, path) in cell.text.as_deref().map(placeholders).unwrap_or_default() {
                        if let Some(other) = lists.iter().find(|other| {
                            **other != block.list && path.starts_with(&format!("{other}."))
                        }) {
                            return Err(invalid(format!(
                                "The list `{other}` is inside the list `{}`. A template cannot repeat a list inside a list.",
                                block.list
                            )));
                        }
                    }
                }
            }
        }
        Ok(blocks)
    }

    /// Whether a formula or a part of `sheet` names a row that a block of `later` moves, so the
    /// sheet waits until those blocks' items are known.
    fn refers_later(sheet: &Sheet, later: &[Shift]) -> Result<bool, Error> {
        if later.is_empty() {
            return Ok(false);
        }
        let formulas = sheet.rows.iter().flat_map(|row| &row.cells).any(|cell| {
            cell.formula
                .as_ref()
                .is_some_and(|(_, formula)| shift::apply(formula, &sheet.name, later) != *formula)
        });
        let rest = &sheet.xml[sheet.tail_at..];
        Ok(formulas || rewrite_part(rest, &sheet.name, later)? != rest)
    }

    /// Whether a formula above the last block names a row the first block moves, so the sheet
    /// waits for every item.
    fn must_hold(sheet: &Sheet, blocks: &[Block]) -> bool {
        let (Some(first), Some(last)) = (blocks.first(), blocks.last()) else {
            return false;
        };
        sheet
            .rows
            .iter()
            .filter(|row| row.number <= last.bottom)
            .any(|row| {
                let own = blocks
                    .iter()
                    .find(|block| (block.top..=block.bottom).contains(&row.number));
                row.cells
                    .iter()
                    .filter_map(|cell| cell.formula.as_ref())
                    .any(|(_, formula)| {
                        rows_named(formula, &sheet.name).iter().any(|(on, at)| {
                            on.eq_ignore_ascii_case(&sheet.name)
                                && *at >= first.top
                                && !own.is_some_and(|block| (block.top..=block.bottom).contains(at))
                        })
                    })
            })
    }

    /// Fills every placeholder that is not in a list with `values`, finds the blocks of
    /// `lists`, and returns the bytes ready to leave and the lists in their blocks' order.
    pub fn fill(
        &mut self,
        values: Vec<Placeholder>,
        lists: &[String],
    ) -> Result<(Vec<u8>, Vec<String>), Error> {
        if !matches!(self.state, State::Open) {
            return Err(invalid("This template is already filled.".to_owned()));
        }
        let placed = self.blocks(lists)?;
        let mut pages: Vec<Page> = Vec::new();
        for (at, (sheet, _)) in placed.iter().enumerate() {
            match pages.last_mut() {
                Some(page) if page.sheet == *sheet => page.end = at + 1,
                _ => pages.push(Page {
                    sheet: *sheet,
                    first: at,
                    end: at + 1,
                    stream: true,
                    base: 0,
                }),
            }
        }
        let blocks: Vec<Block> = placed.into_iter().map(|(_, block)| block).collect();
        // A probe move at each page's first block finds a sheet that names rows a later page
        // moves.
        let probes: Vec<Shift> = pages
            .iter()
            .map(|page| Shift {
                sheet: self.sheets[page.sheet].name.clone(),
                top: blocks[page.first].top,
                bottom: blocks[page.first].top,
                delta: 1,
                copy: None,
            })
            .collect();
        for (at, page) in pages.iter_mut().enumerate() {
            let sheet = &self.sheets[page.sheet];
            page.stream = !Self::must_hold(sheet, &blocks[page.first..page.end])
                && !Self::refers_later(sheet, &probes[at + 1..])?;
        }
        let order = blocks.iter().map(|block| block.list.clone()).collect();
        let mut filling = Box::new(Filling {
            values: values.into_iter().collect(),
            blocks,
            pages,
            page: 0,
            current: 0,
            next_row: 0,
            shifts: Vec::new(),
            next_group: 0,
            rows_out: 0,
            rows: Vec::new(),
            archive: Archive::new(),
        });
        if !filling.pages.is_empty() {
            enter(&self.sheets, &mut filling, 0)?;
        }
        let data = filling.archive.take();
        self.state = State::Filling(filling);
        Ok((data, order))
    }

    /// Writes a batch of the list `list`'s items, each its placeholders by their paths inside
    /// the item, and returns the bytes ready to leave.
    pub fn items(&mut self, list: &str, items: Vec<Vec<Placeholder>>) -> Result<Vec<u8>, Error> {
        let State::Filling(filling) = &mut self.state else {
            return Err(invalid(
                "A template's lists are written after `fill` and before `finish`.".to_owned(),
            ));
        };
        let Some(at) = filling.blocks.iter().position(|block| block.list == list) else {
            return Ok(Vec::new());
        };
        if at < filling.current {
            return Err(invalid(format!(
                "The list `{list}` is written after the list `{}`, which comes below it in the template.",
                filling.blocks[filling.current].list
            )));
        }
        let sheets = &self.sheets;
        while filling.pages[filling.page].end <= at {
            leave(sheets, filling)?;
            filling.page += 1;
            enter(sheets, filling, filling.page)?;
        }
        let page = filling.pages[filling.page];
        let sheet = &sheets[page.sheet];
        filling.blocks[at].items += items.len() as u32;
        if !page.stream {
            filling.current = at;
            filling.blocks[at].held.extend(items);
            return Ok(filling.archive.take());
        }
        let mut out = String::new();
        while filling.current < at {
            close_block(&mut out, sheet, filling, page)?;
        }
        for item in &items {
            item_rows(&mut out, sheet, filling, page, at, item)?;
        }
        filling.archive.write(out.as_bytes())?;
        Ok(filling.archive.take())
    }

    /// Writes the rest of every sheet with blocks and every other part, and returns the last
    /// bytes.
    pub fn finish(&mut self, source: &dyn Source) -> Result<Finished, Error> {
        let State::Filling(mut filling) = std::mem::replace(&mut self.state, State::Finished)
        else {
            return Err(invalid(
                "A template is finished once, after `fill`.".to_owned(),
            ));
        };
        if !filling.pages.is_empty() {
            loop {
                leave(&self.sheets, &mut filling)?;
                if filling.page + 1 == filling.pages.len() {
                    break;
                }
                filling.page += 1;
                let at = filling.page;
                enter(&self.sheets, &mut filling, at)?;
            }
        }
        for page in filling.pages.clone() {
            if !page.stream {
                write_held(&self.sheets, &mut filling, page)?;
            }
        }
        let owners = self.owners(source)?;
        let entries = self.directory.entries().to_vec();
        for entry in &entries {
            let name = entry.name.as_str();
            if dropped(name)
                || filling
                    .pages
                    .iter()
                    .any(|page| self.sheets[page.sheet].path.eq_ignore_ascii_case(name))
            {
                continue;
            }
            filling.archive.begin(name)?;
            if let Some(index) = self
                .sheets
                .iter()
                .position(|sheet| sheet.path.eq_ignore_ascii_case(name))
            {
                let sheet = &self.sheets[index];
                let mut out = String::new();
                filling.rows_out = 0;
                head(&mut out, sheet, false);
                filling.next_row = 0;
                rows_until(&mut out, sheet, &mut filling, u32::MAX)?;
                tail(&mut out, sheet, &filling.shifts)?;
                filling.archive.write(out.as_bytes())?;
                filling.rows.push(SheetRows {
                    name: sheet.name.clone(),
                    rows: filling.rows_out,
                });
            } else if let Some(text) = self.rewritten(source, entry, &owners, &filling.shifts)? {
                filling.archive.write(text.as_bytes())?;
            } else {
                let mut part = Part::open(source, entry)?;
                let mut chunk = Vec::new();
                while part.fill(source, &mut chunk)? {
                    filling.archive.write(&chunk)?;
                    chunk.clear();
                }
            }
            filling.archive.end()?;
        }
        filling.archive.close()?;
        let size = filling.archive.size();
        Ok(Finished {
            data: filling.archive.take(),
            rows: std::mem::take(&mut filling.rows),
            size,
        })
    }

    /// The sheet each table and drawing belongs to.
    fn owners(&self, source: &dyn Source) -> Result<Vec<(String, String)>, Error> {
        let mut owners = Vec::new();
        for sheet in &self.sheets {
            if let Some(rels) = read(&self.directory, source, &rels_of(&sheet.path))? {
                for (_, part) in relationships(&rels, &sheet.path)? {
                    owners.push((part.to_ascii_lowercase(), sheet.name.clone()));
                }
            }
        }
        Ok(owners)
    }

    /// The text of a part the fill changes, or `None` for one it copies as it is.
    fn rewritten(
        &self,
        source: &dyn Source,
        entry: &crate::read::zip::Entry,
        owners: &[(String, String)],
        shifts: &[Shift],
    ) -> Result<Option<String>, Error> {
        let lower = entry.name.to_ascii_lowercase();
        let read = || text_of(self.directory.whole(source, entry)?, &entry.name);
        Ok(Some(match lower.as_str() {
            "[content_types].xml" => without(
                &read()?,
                &["vbaproject", "calcchain"],
                &[(
                    "application/vnd.ms-excel.sheet.macroEnabled.main+xml",
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
                )],
            )?,
            "xl/_rels/workbook.xml.rels" => without(&read()?, &["vbaproject", "calcchain"], &[])?,
            "xl/workbook.xml" => recalculating(&rewrite_part(&read()?, "", shifts)?),
            _ if shifts.is_empty() || lower.ends_with(".rels") => return Ok(None),
            _ if lower.starts_with("xl/tables/") || lower.starts_with("xl/drawings/") => {
                let home = owners
                    .iter()
                    .find(|(part, _)| *part == lower)
                    .map_or("", |(_, sheet)| sheet.as_str());
                rewrite_part(&read()?, home, shifts)?
            }
            _ if lower.starts_with("xl/charts/") => rewrite_part(&read()?, "", shifts)?,
            _ => return Ok(None),
        }))
    }
}

/// The shared strings' texts, each its runs joined.
fn shared_strings(text: &str) -> Result<Vec<String>, Error> {
    let bytes = text.as_bytes();
    let mut strings = Vec::new();
    let mut at = 0;
    let mut reading = false;
    let mut phonetic = false;
    while let Some((token, length)) = xml::next(&bytes[at..])? {
        at += length;
        match token {
            Token::Open { name, empty, .. } => match xml::local(name) {
                b"si" => strings.push(String::new()),
                b"rPh" => phonetic = !empty,
                b"t" => reading = !empty && !phonetic,
                _ => {}
            },
            Token::Close(name) => match xml::local(name) {
                b"t" => reading = false,
                b"rPh" => phonetic = false,
                _ => {}
            },
            Token::Text(raw) if reading => {
                if let Some(last) = strings.last_mut() {
                    xml::decode(raw, last)?;
                }
            }
            Token::Raw(raw) if reading => {
                if let Some(last) = strings.last_mut() {
                    last.push_str(&String::from_utf8_lossy(raw));
                }
            }
            _ => {}
        }
    }
    Ok(strings)
}

/// Appends the sheet's XML before its rows, without its `dimension` when `blocks` is true.
fn head(out: &mut String, sheet: &Sheet, blocks: bool) {
    let head = &sheet.xml[..sheet.rows_at];
    let mut edits = Vec::new();
    if blocks && let Some(at) = head.find("<dimension") {
        let end = at + head[at..].find("/>").map_or(0, |end| end + 2);
        edits.push((at..end, String::new()));
    }
    splice(out, &sheet.xml, 0..sheet.rows_at, &mut edits);
    if sheet.self_closed {
        out.push_str("<sheetData>");
    }
}

/// Starts the page `at`: a page that streams writes its sheet up to its first block.
fn enter(sheets: &[Sheet], filling: &mut Filling, at: usize) -> Result<(), Error> {
    let page = filling.pages[at];
    filling.current = page.first;
    if !page.stream {
        return Ok(());
    }
    filling.pages[at].base = filling.shifts.len();
    let sheet = &sheets[page.sheet];
    filling.archive.begin(&sheet.path)?;
    filling.next_row = 0;
    filling.rows_out = 0;
    filling.next_group = sheet.next_group;
    let mut out = String::new();
    head(&mut out, sheet, true);
    let top = filling.blocks[page.first].top;
    rows_until(&mut out, sheet, filling, top)?;
    filling.archive.write(out.as_bytes())?;
    Ok(())
}

/// Ends the page the fill is on, once every item of its lists is known. A page that streams
/// writes the rest of its sheet. A page that waits records its blocks' moves, and is written at
/// `finish`.
fn leave(sheets: &[Sheet], filling: &mut Filling) -> Result<(), Error> {
    let page = filling.pages[filling.page];
    let sheet = &sheets[page.sheet];
    if !page.stream {
        filling.pages[filling.page].base = filling.shifts.len();
        let mut moved = 0i64;
        for at in page.first..page.end {
            let block = &filling.blocks[at];
            let delta = i64::from(block.items * block.height()) - i64::from(block.height());
            let shift = Shift {
                sheet: sheet.name.clone(),
                top: (i64::from(block.top) + moved) as u32,
                bottom: (i64::from(block.bottom) + moved) as u32,
                delta,
                copy: None,
            };
            filling.shifts.push(shift);
            moved += delta;
        }
        filling.current = page.end;
        return Ok(());
    }
    let mut out = String::new();
    while filling.current < page.end {
        close_block(&mut out, sheet, filling, page)?;
    }
    rows_until(&mut out, sheet, filling, u32::MAX)?;
    tail(&mut out, sheet, &filling.shifts)?;
    filling.archive.write(out.as_bytes())?;
    filling.archive.end()?;
    filling.rows.push(SheetRows {
        name: sheet.name.clone(),
        rows: filling.rows_out,
    });
    Ok(())
}

/// Writes a page that waited, with every block's move known.
fn write_held(sheets: &[Sheet], filling: &mut Filling, page: Page) -> Result<(), Error> {
    let sheet = &sheets[page.sheet];
    filling.archive.begin(&sheet.path)?;
    filling.next_row = 0;
    filling.rows_out = 0;
    filling.next_group = sheet.next_group;
    let mut out = String::new();
    head(&mut out, sheet, true);
    for at in page.first..page.end {
        let top = filling.blocks[at].top;
        rows_until(&mut out, sheet, filling, top)?;
        let held = std::mem::take(&mut filling.blocks[at].held);
        for item in &held {
            item_rows(&mut out, sheet, filling, page, at, item)?;
        }
        let bottom = filling.blocks[at].bottom;
        filling.next_row = sheet.rows.partition_point(|row| row.number <= bottom);
    }
    rows_until(&mut out, sheet, filling, u32::MAX)?;
    tail(&mut out, sheet, &filling.shifts)?;
    filling.archive.write(out.as_bytes())?;
    filling.archive.end()?;
    filling.rows.push(SheetRows {
        name: sheet.name.clone(),
        rows: filling.rows_out,
    });
    Ok(())
}

/// Appends `</sheetData>` and the rest of the sheet, moved by `shifts`.
fn tail(out: &mut String, sheet: &Sheet, shifts: &[Shift]) -> Result<(), Error> {
    let mut rest = &sheet.xml[sheet.tail_at..];
    if sheet.self_closed {
        out.push_str("</sheetData>");
        rest = &rest[rest.find("/>").map_or(0, |at| at + 2)..];
    }
    out.push_str(&rewrite_part(rest, &sheet.name, shifts)?);
    Ok(())
}

/// Appends the template rows from `filling.next_row` up to, not including, row `until`, of the
/// sheet `sheet`.
fn rows_until(
    out: &mut String,
    sheet: &Sheet,
    filling: &mut Filling,
    until: u32,
) -> Result<(), Error> {
    while let Some(row) = sheet.rows.get(filling.next_row) {
        if row.number >= until {
            break;
        }
        let number = filling.shifts.iter().fold(row.number, |at, shift| {
            shift.row(RowRef {
                sheet: &sheet.name,
                row: at,
                absolute: false,
                end: shift::End::Single,
            })
        });
        let values = &filling.values;
        let moves = Moves {
            shifts: &filling.shifts,
            groups: &[],
        };
        write_row(out, sheet, row, number, &moves, &|path| {
            values.get(path).map(Option::as_ref)
        })?;
        filling.rows_out += 1;
        filling.next_row += 1;
    }
    Ok(())
}

/// Ends the block being written: its move is known now, and the rows up to the next block are
/// written with it.
fn close_block(
    out: &mut String,
    sheet: &Sheet,
    filling: &mut Filling,
    page: Page,
) -> Result<(), Error> {
    let block = &filling.blocks[filling.current];
    let moved = filling.shifts[page.base..]
        .iter()
        .map(|shift| shift.delta)
        .sum::<i64>();
    let delta = i64::from(block.items * block.height()) - i64::from(block.height());
    filling.shifts.push(Shift {
        sheet: sheet.name.clone(),
        top: (i64::from(block.top) + moved) as u32,
        bottom: (i64::from(block.bottom) + moved) as u32,
        delta,
        copy: None,
    });
    let bottom = block.bottom;
    filling.next_row = sheet.rows.partition_point(|row| row.number <= bottom);
    filling.current += 1;
    let until = if filling.current < page.end {
        filling.blocks[filling.current].top
    } else {
        u32::MAX
    };
    if until != u32::MAX {
        rows_until(out, sheet, filling, until)?;
    }
    Ok(())
}

/// Appends one item's copy of the block `at`.
fn item_rows(
    out: &mut String,
    sheet: &Sheet,
    filling: &mut Filling,
    page: Page,
    at: usize,
    item: &[Placeholder],
) -> Result<(), Error> {
    let block = &filling.blocks[at];
    let height = block.height();
    let copy = block.written * height;
    // The block's own move is `filling.shifts[own]`, once the block is closed.
    let own = page.base + (at - page.first);
    let moved: i64 = filling.shifts[page.base..own.min(filling.shifts.len())]
        .iter()
        .map(|shift| shift.delta)
        .sum();
    let top = (i64::from(block.top) + moved) as u32;
    let last = u64::from(top) + u64::from(copy) + u64::from(height) - 1;
    if last > 1_048_576 {
        return Err(invalid(format!(
            "The list `{}` has too many items for one sheet. Item {} would end on row {last}, and a sheet has at most 1,048,576 rows.",
            block.list,
            block.written + 1
        )));
    }
    let mut shifts: Vec<Shift> = filling.shifts.iter().take(own).cloned().collect();
    let mut moving = filling.shifts.get(own).cloned().unwrap_or(Shift {
        sheet: sheet.name.clone(),
        top,
        bottom: top + height - 1,
        delta: 0,
        copy: None,
    });
    moving.copy = Some(copy);
    shifts.push(moving);
    shifts.extend(filling.shifts.iter().skip(own + 1).cloned());
    let prefix = format!("{}.", block.list);
    let fields: BTreeMap<&str, Option<&Value>> = item
        .iter()
        .map(|(path, value)| (path.as_str(), value.as_ref()))
        .collect();
    let (from, to) = (block.top, block.bottom);
    let written = block.written;
    let rows_of = || {
        sheet
            .rows
            .iter()
            .filter(move |row| (from..=to).contains(&row.number))
    };
    // A shared formula whose first cell is in the block gets a new group in every copy but the
    // first, numbered after the sheet's highest, so no two copies share one.
    let mut groups: Vec<(u32, u32)> = Vec::new();
    if written > 0 {
        for cell in rows_of().flat_map(|row| &row.cells) {
            if let (Some((_, group)), Some(_)) = (&cell.shared, &cell.formula)
                && !groups.iter().any(|(old, _)| old == group)
            {
                groups.push((*group, filling.next_group));
                filling.next_group += 1;
            }
        }
    }
    let values = &filling.values;
    let moves = Moves {
        shifts: &shifts,
        groups: &groups,
    };
    let mut rows = 0;
    for row in rows_of() {
        let number = top + copy + (row.number - from);
        write_row(
            out,
            sheet,
            row,
            number,
            &moves,
            &|path| match path.strip_prefix(&prefix) {
                Some(field) => fields.get(field).copied(),
                None => values.get(path).map(Option::as_ref),
            },
        )?;
        rows += 1;
    }
    filling.rows_out += rows;
    filling.blocks[at].written += 1;
    Ok(())
}

/// What a copied row changes in its formulas: the moves of its references, and each shared
/// formula group's new number, old first.
struct Moves<'s> {
    shifts: &'s [Shift],
    groups: &'s [(u32, u32)],
}

/// Appends `row` as row `number`, its formulas changed by `moves` and its placeholders filled
/// by `lookup`.
fn write_row<'v>(
    out: &mut String,
    sheet: &Sheet,
    row: &Row,
    number: u32,
    moves: &Moves<'_>,
    lookup: &dyn Fn(&str) -> Option<Option<&'v Value>>,
) -> Result<(), Error> {
    let xml = &sheet.xml;
    match (&row.raw, &row.r) {
        (Some(_), Some(r)) => {
            let mut edits = vec![(r.clone(), number.to_string())];
            splice(out, xml, row.open.clone(), &mut edits);
        }
        (Some(_), None) => out.push_str(&xml[row.open.clone()]),
        (None, _) => {
            let _ = write!(out, "<row r=\"{number}\">");
        }
    }
    if row.empty {
        return Ok(());
    }
    for cell in &row.cells {
        write_cell(out, sheet, row, cell, number, moves, lookup)?;
    }
    out.push_str("</row>");
    Ok(())
}

fn write_cell<'v>(
    out: &mut String,
    sheet: &Sheet,
    row: &Row,
    cell: &Cell,
    number: u32,
    moves: &Moves<'_>,
    lookup: &dyn Fn(&str) -> Option<Option<&'v Value>>,
) -> Result<(), Error> {
    let Moves { shifts, groups } = *moves;
    let reference = crate::cells::reference(cell.column, number);
    if let Some(value) = &cell.set {
        return value_cell(out, &reference, cell.style.as_deref(), value.as_ref());
    }
    if let Some(text) = &cell.text {
        let found = placeholders(text);
        if !found.is_empty() {
            let mut values = Vec::with_capacity(found.len());
            for (_, path) in &found {
                let value = lookup(path).ok_or_else(|| {
                    invalid(format!(
                        "The template's cell `{}!{}` has the placeholder `{{{{{path}}}}}`, which the data does not have.",
                        sheet.name,
                        crate::cells::reference(cell.column, row.number)
                    ))
                })?;
                values.push(value);
            }
            if found.len() == 1 && found[0].0 == (0..text.len()) {
                return value_cell(out, &reference, cell.style.as_deref(), values[0]);
            }
            let mut filled = String::with_capacity(text.len());
            let mut copied = 0;
            for ((at, _), value) in found.iter().zip(&values) {
                filled.push_str(&text[copied..at.start]);
                if let Some(value) = value {
                    filled.push_str(&as_text(value));
                }
                copied = at.end;
            }
            filled.push_str(&text[copied..]);
            return value_cell(
                out,
                &reference,
                cell.style.as_deref(),
                Some(&Value::Text(filled)),
            );
        }
    }
    let Some(raw) = &cell.raw else {
        return Ok(());
    };
    let mut edits = Vec::new();
    if let Some(r) = &cell.r {
        edits.push((r.clone(), reference));
    }
    if let Some((at, formula)) = &cell.formula {
        let moved = shift::apply(formula, &sheet.name, shifts);
        if moved != *formula {
            let mut escaped = String::new();
            crate::xml::text(&mut escaped, &moved);
            edits.push((at.clone(), escaped));
        }
    }
    if let Some(at) = &cell.formula_ref {
        let current = &sheet.xml[at.clone()];
        let moved = shift::apply_list(current, &sheet.name, shifts);
        if moved != current {
            edits.push((at.clone(), moved));
        }
    }
    if let Some((at, group)) = &cell.shared
        && let Some((_, new)) = groups.iter().find(|(old, _)| old == group)
    {
        edits.push((at.clone(), new.to_string()));
    }
    splice(out, &sheet.xml, raw.clone(), &mut edits);
    Ok(())
}
