//! The parts xlsx writes after a sheet's rows or in an entry of their own
//! (`rule:core-classes/spreadsheet-sheet-end-parts`).
//!
//! [`Tail`] keeps each part the program adds to the open sheet, checked when it is added, until
//! the sheet ends. Then [`Tail::write`] writes them after `</sheetData>` in the order the xlsx
//! schema gives: protection, the autofilter, merged cells, conditional formats (`conditional`'s
//! module doc), data validation, hyperlinks, the page
//! margins, the page setup, the header and footer, the images' and the notes' drawings, and the
//! tables. A part may name any cell, written or not. A link to a URL or a `mailto:` address, the
//! notes, the images and the tables are relationships of the sheet, which [`Tail::rels`] writes
//! in that order, and the notes are a `comments<n>.xml` beside the VML drawing Excel still needs
//! to show them ([`Tail::notes`]). The images and charts are the sheet's `drawing<n>.xml`
//! (`drawing`'s module doc), whose `<drawing>` element comes before the notes'. Each table is its own
//! `xl/tables/table<n>.xml` (`table`'s module doc), and no table overlaps a merged range or
//! another table.
//!
//! **Two parts are read before the rows.** An outline group gives each row it covers one more
//! level, which xlsx writes with the row, and a column group the same for its columns, which xlsx
//! writes before the first row. The fit-to-page flag is in the sheet's head too. [`Groups`] keeps
//! the groups for the rows and columns still to come, and a group over a row already written, a
//! column group or a `fit` once the first row is written is an error that says so.
//!
//! **A link goes to `http:`, `https:`, `mailto:` or a cell of the workbook** (`'Sheet 2'!A1`).
//! Any other scheme, such as `file:` or `javascript:`, is an error, so a link built from user data
//! opens no local file and runs nothing.
//!
//! **Sheet protection locks the layout and is not security.** The password is written as Excel's
//! 16-bit hash, which any tool removes.
//!
//! Memory: every part of the open sheet until the sheet ends, as text no longer than the program
//! gave.

use std::fmt::Write as _;

use crate::chart::Chart;
use crate::conditional::Conditional;
use crate::drawing::{Drawing, Format, Grid, Image};
use crate::table::{Placed, Table};
use crate::{Error, Row, cells, xml};

/// The relationship types and the namespace every part of a sheet uses.
const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// Excel's limits on the parts.
const MAX_LINKS: usize = 65_530;
const MAX_URL: usize = 2_079;
const MAX_NOTE: usize = 32_767;
const MAX_LIST: usize = 255;
const MAX_TITLE: usize = 32;
const MAX_MESSAGE: usize = 255;
const MAX_HEADER: usize = 255;

/// A rectangle of cells: columns from 0, rows from 1, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Area {
    pub fn overlaps(&self, other: &Area) -> bool {
        self.left <= other.right
            && other.left <= self.right
            && self.top <= other.bottom
            && other.top <= self.bottom
    }
}

/// The column, from 0, of the letters `letters`, such as `AB`.
fn column_index(letters: &str) -> Option<u32> {
    if !(1..=3).contains(&letters.len()) || !letters.bytes().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    let column = letters
        .bytes()
        .fold(0u32, |sum, b| sum * 26 + u32::from(b - b'A' + 1));
    (column <= 16_384).then(|| column - 1)
}

/// The row of `digits`, from 1.
fn row_number(digits: &str) -> Option<u32> {
    if digits.is_empty()
        || digits.len() > 7
        || digits.starts_with('0')
        || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let row: u32 = digits.parse().ok()?;
    (row <= 1_048_576).then_some(row)
}

/// The column from 0 and the row from 1 of a cell such as `B7` or `$B$7`.
pub fn cell_at(text: &str) -> Option<(u32, u32)> {
    let text = text.strip_prefix('$').unwrap_or(text);
    let letters = text.bytes().take_while(u8::is_ascii_uppercase).count();
    let rest = &text[letters..];
    let rest = rest.strip_prefix('$').unwrap_or(rest);
    Some((column_index(&text[..letters])?, row_number(rest)?))
}

/// The corners of a range such as `A1:C4`, or of one cell.
pub fn area(text: &str) -> Option<Area> {
    let (from, to) = text.split_once(':').unwrap_or((text, text));
    let (left, top) = cell_at(from)?;
    let (right, bottom) = cell_at(to)?;
    (left <= right && top <= bottom).then_some(Area {
        left,
        top,
        right,
        bottom,
    })
}

fn bad_range(what: &str, range: &str) -> Error {
    Error::Invalid(format!(
        "The {what} range `{range}` is not a range of cells, such as `A1:D10`."
    ))
}

pub(crate) fn bad_cell(what: &str, cell: &str) -> Error {
    Error::Invalid(format!(
        "The {what} cell `{cell}` is not a cell reference, such as `B7`."
    ))
}

/// A hyperlink on `anchor`: a URL, a `mailto:` address or a cell such as `'Sheet 2'!A1`.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub anchor: String,
    pub target: String,
    pub text: Option<String>,
    pub tooltip: Option<String>,
}

/// A note on the cell `noted`.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub noted: String,
    pub note: String,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    List,
    Whole,
    Decimal,
    Date,
    Length,
    Formula,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Between,
    NotBetween,
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

impl Operator {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Operator::Between => "between",
            Operator::NotBetween => "notBetween",
            Operator::Equal => "equal",
            Operator::NotEqual => "notEqual",
            Operator::Greater => "greaterThan",
            Operator::GreaterOrEqual => "greaterThanOrEqual",
            Operator::Less => "lessThan",
            Operator::LessOrEqual => "lessThanOrEqual",
        }
    }

    pub(crate) fn operands(self) -> usize {
        match self {
            Operator::Between | Operator::NotBetween => 2,
            _ => 1,
        }
    }
}

/// Data validation over `validated`. `values` are the list's items, or the operator's bounds as
/// numbers, `yyyy-mm-dd` dates or formulas; a list of one value that begins with `=` is a
/// reference to the cells that hold the items.
#[derive(Debug, Clone, PartialEq)]
pub struct Validation {
    pub validated: String,
    pub kind: RuleKind,
    pub operator: Option<Operator>,
    pub values: Vec<String>,
    pub input_title: Option<String>,
    pub input_message: Option<String>,
    pub error_title: Option<String>,
    pub error_message: Option<String>,
    pub allow_empty: bool,
}

/// An outline group over rows such as `5:9` or columns such as `B:D`.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub grouped: String,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Page margins in inches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// The margins a page setup has when it gives none: Excel's.
pub const DEFAULT_MARGINS: Margins = Margins {
    top: 0.75,
    right: 0.7,
    bottom: 0.75,
    left: 0.7,
};

/// Page setup. `fit_width` and `fit_height` are the pages to fit the sheet on, and an absent one
/// of the two is as many as it needs. `repeat_rows` is rows such as `1:2`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Print {
    pub orientation: Option<Orientation>,
    pub fit_width: Option<u64>,
    pub fit_height: Option<u64>,
    pub margins: Option<Margins>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub repeat_rows: Option<String>,
    pub area: Option<String>,
}

impl Print {
    /// Whether it fits the sheet to pages, which the sheet's head says.
    pub fn fits(&self) -> bool {
        self.fit_width.is_some() || self.fit_height.is_some()
    }
}

/// Sheet protection. `allow` names what a person may still do, by the names
/// [`PROTECT_ALLOWS`] gives.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Protect {
    pub password: Option<String>,
    pub allow: Vec<String>,
}

/// What a protected sheet may allow, in the order xlsx writes them.
pub const PROTECT_ALLOWS: [&str; 15] = [
    "objects",
    "scenarios",
    "formatCells",
    "formatColumns",
    "formatRows",
    "insertColumns",
    "insertRows",
    "insertHyperlinks",
    "deleteColumns",
    "deleteRows",
    "selectLockedCells",
    "sort",
    "autoFilter",
    "pivotTables",
    "selectUnlockedCells",
];

/// One part of a sheet the writer writes after its rows.
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Merge(String),
    Link(Link),
    Note(Note),
    Validation(Validation),
    Group(Group),
    Print(Print),
    Protect(Protect),
    Image(Image),
    Chart(Chart),
    Table(Table),
    Conditional(Conditional),
}

/// Excel's 16-bit hash of a sheet's password, as it writes it.
pub fn password_hash(password: &str) -> u16 {
    if password.is_empty() {
        return 0;
    }
    let rotate = |hash: u16| ((hash >> 14) & 0x01) | ((hash << 1) & 0x7fff);
    let mut hash: u16 = 0;
    for &byte in password.as_bytes().iter().rev() {
        hash = rotate(hash) ^ u16::from(byte);
    }
    rotate(hash) ^ (password.len() as u16) ^ 0xCE4B
}

/// The rows such as `5:9` or columns such as `B:D` a group or a title names, from and to.
enum Span {
    Rows(u32, u32),
    Columns(u32, u32),
}

/// The first and last row of rows such as `1:2`, or `None` when `text` is not rows.
pub(crate) fn row_span(text: &str) -> Option<(u32, u32)> {
    match span(text)? {
        Span::Rows(from, to) => Some((from, to)),
        Span::Columns(..) => None,
    }
}

fn span(text: &str) -> Option<Span> {
    let text = text.replace('$', "");
    let (from, to) = text.split_once(':').unwrap_or((&text, &text));
    if let (Some(from), Some(to)) = (row_number(from), row_number(to)) {
        return (from <= to).then_some(Span::Rows(from, to));
    }
    let (from, to) = (column_index(from)?, column_index(to)?);
    (from <= to).then_some(Span::Columns(from, to))
}

/// The outline groups of the rows and columns still to be written. Each group a row or column is
/// in gives it one more level, and a collapsed group hides it and marks the row or column after
/// it.
#[derive(Debug, Default)]
pub struct Groups {
    rows: Vec<(u32, u32, bool)>,
    columns: Vec<(u32, u32, bool)>,
}

/// What the groups give one row or column: its added level, whether it is hidden, and whether it
/// follows a collapsed group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Outline {
    pub level: u64,
    pub hidden: bool,
    pub collapsed: bool,
}

fn outline(groups: &[(u32, u32, bool)], at: u32) -> Outline {
    let mut out = Outline::default();
    for &(from, to, collapsed) in groups {
        if (from..=to).contains(&at) {
            out.level += 1;
            out.hidden |= collapsed;
        }
        out.collapsed |= collapsed && to.checked_add(1) == Some(at);
    }
    out
}

impl Groups {
    /// Adds `group` to the sheet `sheet`, whose next row is `next` and whose head is written when
    /// `started`.
    pub fn add(
        &mut self,
        group: &Group,
        sheet: &str,
        next: u32,
        started: bool,
    ) -> Result<(), Error> {
        let range = &group.grouped;
        match span(range) {
            Some(Span::Rows(from, to)) => {
                if from < next {
                    return Err(Error::Invalid(format!(
                        "Rows {range} of the sheet `{sheet}` are already written. Group rows before you add them, or give each row its `level`."
                    )));
                }
                self.rows.push((from, to, group.collapsed));
            }
            Some(Span::Columns(from, to)) => {
                if started {
                    return Err(Error::Invalid(format!(
                        "The columns {range} of the sheet `{sheet}` cannot be grouped after its first row. Group columns before you add rows."
                    )));
                }
                self.columns.push((from, to, group.collapsed));
            }
            None => {
                return Err(Error::Invalid(format!(
                    "The group `{range}` is not a range of rows or columns, such as `5:9` or `B:D`."
                )));
            }
        }
        Ok(())
    }

    /// What the groups give the row `number`.
    pub fn row(&self, number: u32) -> Outline {
        outline(&self.rows, number)
    }

    /// What the groups give the column at `at`, from 0.
    pub fn column(&self, at: u32) -> Outline {
        outline(&self.columns, at)
    }

    /// The number of columns the groups reach, the one after a collapsed group included.
    pub fn columns_reached(&self) -> usize {
        self.columns
            .iter()
            .map(|&(_, to, collapsed)| to as usize + 1 + usize::from(collapsed))
            .max()
            .unwrap_or(0)
    }

    /// The deepest level the row groups can give, and the column groups'.
    pub fn depth(&self) -> (u64, u64) {
        let deepest = |groups: &[(u32, u32, bool)]| {
            groups
                .iter()
                .map(|&(from, ..)| outline(groups, from).level)
                .max()
                .unwrap_or(0)
        };
        (deepest(&self.rows), deepest(&self.columns))
    }
}

/// The parts of one sheet, until it ends.
#[derive(Debug, Default)]
pub struct Tail {
    merges: Vec<Area>,
    /// Each link's cell, and its `<hyperlink>` element.
    links: Vec<(Area, String)>,
    /// The targets of the links that leave the workbook, in the order of their `rId`.
    external: Vec<String>,
    notes: Vec<Note>,
    noted: Vec<Area>,
    validations: Vec<String>,
    print: Option<Print>,
    protect: Option<String>,
    drawing: Drawing,
    tables: Vec<Placed>,
    /// Each conditional format's `<conditionalFormatting>` element.
    conditionals: Vec<String>,
}

fn limited(what: &str, text: &Option<String>, max: usize) -> Result<(), Error> {
    match text {
        Some(text) if text.chars().count() > max => Err(Error::Invalid(format!(
            "The {what} has {} characters. Excel allows at most {max}.",
            text.chars().count()
        ))),
        _ => Ok(()),
    }
}

/// Appends ` name="value"` when `value` is set.
fn optional_attr(out: &mut String, name: &str, value: &Option<String>) {
    if let Some(value) = value {
        let _ = write!(out, " {name}=\"");
        xml::attr(out, value);
        out.push('"');
    }
}

/// A validation bound as formula text: a `yyyy-mm-dd` date for a date rule is its serial, and a
/// leading `=` is dropped.
pub(crate) fn bound(kind: RuleKind, value: &str) -> String {
    if kind == RuleKind::Date
        && let [year, month, day] = value.split('-').collect::<Vec<_>>()[..]
        && let (Ok(year), Ok(month), Ok(day)) = (year.parse(), month.parse(), day.parse())
        && let Some(serial) = cells::date_serial(&crate::Date { year, month, day })
    {
        return serial.to_string();
    }
    value.strip_prefix('=').unwrap_or(value).to_owned()
}

impl Tail {
    /// Checks `part` and keeps it for the sheet `sheet`. A group goes to [`Groups::add`].
    pub fn add(&mut self, part: Part, sheet: &str) -> Result<(), Error> {
        match part {
            Part::Merge(range) => self.merge(&range, sheet),
            Part::Link(link) => self.link(link),
            Part::Note(note) => self.note(note),
            Part::Validation(validation) => self.validation(&validation),
            Part::Print(print) => self.page(print),
            Part::Protect(protect) => self.protection(&protect),
            Part::Group(_) => Err(Error::Runtime(
                "An outline group is kept with the rows, not the sheet's tail.".to_owned(),
            )),
            Part::Image(_) => Err(Error::Runtime(
                "An image is numbered by the workbook. Add it with `Tail::image`.".to_owned(),
            )),
            Part::Chart(_) => Err(Error::Runtime(
                "A chart is numbered by the workbook. Add it with `Tail::chart`.".to_owned(),
            )),
            Part::Table(_) => Err(Error::Runtime(
                "A table is numbered by the workbook. Add it with `Tail::table`.".to_owned(),
            )),
            Part::Conditional(_) => Err(Error::Runtime(
                "A conditional format's style is the workbook's. Add it with `Tail::conditional`."
                    .to_owned(),
            )),
        }
    }

    /// The priority the next conditional format takes: the sheet's rules counted from 1.
    pub fn next_priority(&self) -> usize {
        self.conditionals.len() + 1
    }

    /// The merged ranges, in the order they were added.
    pub fn merges(&self) -> &[Area] {
        &self.merges
    }

    /// The tables, in the order they were added.
    pub fn placed(&self) -> &[Placed] {
        &self.tables
    }

    /// The page setup, if the sheet has one.
    pub fn print(&self) -> Option<&Print> {
        self.print.as_ref()
    }

    /// Keeps a conditional format written by `conditional::write`.
    pub fn conditional(&mut self, written: String) {
        self.conditionals.push(written);
    }

    fn merge(&mut self, range: &str, sheet: &str) -> Result<(), Error> {
        let merged = area(range).ok_or_else(|| bad_range("merged", range))?;
        if merged.left == merged.right && merged.top == merged.bottom {
            return Err(Error::Invalid(format!(
                "The merged range `{range}` is one cell. A merged range has at least two cells."
            )));
        }
        if self.merges.iter().any(|other| other.overlaps(&merged)) {
            return Err(Error::Invalid(format!(
                "The merged range `{range}` overlaps another merged range of the sheet `{sheet}`."
            )));
        }
        if let Some(table) = self.tables.iter().find(|t| t.whole().overlaps(&merged)) {
            return Err(Error::Invalid(format!(
                "The merged range `{range}` overlaps the table `{}`. A table has no merged cells.",
                table.name
            )));
        }
        self.merges.push(merged);
        Ok(())
    }

    fn link(&mut self, link: Link) -> Result<(), Error> {
        let at = area(&link.anchor).ok_or_else(|| bad_cell("link", &link.anchor))?;
        if self.links.len() >= MAX_LINKS {
            return Err(Error::Invalid(
                "A sheet has at most 65,530 links.".to_owned(),
            ));
        }
        if self.links.iter().any(|(other, _)| other.overlaps(&at)) {
            return Err(Error::Invalid(format!(
                "The cell `{}` has a link already. A cell has at most one link.",
                link.anchor
            )));
        }
        limited("link's text", &link.text, MAX_URL)?;
        limited("link's tooltip", &link.tooltip, MAX_MESSAGE)?;
        let target = link.target.as_str();
        let lower = target.to_ascii_lowercase();
        let external = ["http://", "https://", "mailto:"]
            .iter()
            .any(|scheme| lower.starts_with(scheme));
        let mut element = String::from("<hyperlink ref=\"");
        element.push_str(&link.anchor.replace('$', ""));
        element.push('"');
        if external {
            if target.chars().count() > MAX_URL {
                return Err(Error::Invalid(format!(
                    "The link target has {} characters. Excel allows at most 2,079.",
                    target.chars().count()
                )));
            }
            if target.chars().any(char::is_whitespace) {
                return Err(Error::Invalid(format!(
                    "The link target `{target}` has a space in it. Encode it as `%20`."
                )));
            }
            self.external.push(target.to_owned());
            let _ = write!(element, " r:id=\"rId{}\"", self.external.len());
        } else if let Some((sheet, cell)) = target.rsplit_once('!')
            && !sheet.is_empty()
            && area(cell).is_some()
        {
            element.push_str(" location=\"");
            xml::attr(&mut element, target);
            element.push('"');
        } else {
            return Err(Error::Invalid(format!(
                "The link target `{target}` is not allowed. A link goes to an `http:`, `https:` or `mailto:` address, or to a cell such as `'Sheet 2'!A1`."
            )));
        }
        optional_attr(&mut element, "tooltip", &link.tooltip);
        optional_attr(&mut element, "display", &link.text);
        element.push_str("/>");
        self.links.push((at, element));
        Ok(())
    }

    fn note(&mut self, note: Note) -> Result<(), Error> {
        let at = cell_at(&note.noted)
            .map(|(column, row)| Area {
                left: column,
                top: row,
                right: column,
                bottom: row,
            })
            .ok_or_else(|| bad_cell("note", &note.noted))?;
        if self.noted.contains(&at) {
            return Err(Error::Invalid(format!(
                "The cell `{}` has a note already. A cell has at most one note.",
                note.noted
            )));
        }
        limited("note", &Some(note.note.clone()), MAX_NOTE)?;
        limited("note's author", &note.author, MAX_MESSAGE)?;
        self.noted.push(at);
        self.notes.push(note);
        Ok(())
    }

    fn validation(&mut self, rule: &Validation) -> Result<(), Error> {
        let range = &rule.validated;
        area(range).ok_or_else(|| bad_range("validation", range))?;
        limited("input title", &rule.input_title, MAX_TITLE)?;
        limited("error title", &rule.error_title, MAX_TITLE)?;
        limited("input message", &rule.input_message, MAX_MESSAGE)?;
        limited("error message", &rule.error_message, MAX_MESSAGE)?;
        let (kind, operator) = match rule.kind {
            RuleKind::List => ("list", None),
            RuleKind::Formula => ("custom", None),
            RuleKind::Whole => ("whole", Some(rule.operator.unwrap_or(Operator::Between))),
            RuleKind::Decimal => ("decimal", Some(rule.operator.unwrap_or(Operator::Between))),
            RuleKind::Date => ("date", Some(rule.operator.unwrap_or(Operator::Between))),
            RuleKind::Length => (
                "textLength",
                Some(rule.operator.unwrap_or(Operator::Between)),
            ),
        };
        let wanted = operator.map_or(1, Operator::operands);
        if rule.kind != RuleKind::List && rule.values.len() != wanted {
            return Err(Error::Invalid(format!(
                "The validation of `{range}` needs {wanted} value(s) and has {}.",
                rule.values.len()
            )));
        }
        let formulas: Vec<String> = match rule.kind {
            RuleKind::List => match &rule.values[..] {
                [] => {
                    return Err(Error::Invalid(format!(
                        "The list validation of `{range}` has no values."
                    )));
                }
                [one] if one.starts_with('=') => vec![one[1..].to_owned()],
                values => {
                    if let Some(value) = values.iter().find(|v| v.contains([',', '"'])) {
                        return Err(Error::Invalid(format!(
                            "The list value `{value}` has a `,` or a `\"`. Put such values in cells, and give the list as a reference such as `=$F$1:$F$9`."
                        )));
                    }
                    let joined = values.join(",");
                    if joined.chars().count() > MAX_LIST {
                        return Err(Error::Invalid(format!(
                            "The list validation of `{range}` has {} characters. Excel allows at most 255. Put the values in cells, and give the list as a reference such as `=$F$1:$F$9`.",
                            joined.chars().count()
                        )));
                    }
                    vec![format!("\"{joined}\"")]
                }
            },
            kind => rule.values.iter().map(|value| bound(kind, value)).collect(),
        };
        let mut out = format!("<dataValidation type=\"{kind}\"");
        if let Some(operator) = operator
            && operator != Operator::Between
        {
            let _ = write!(out, " operator=\"{}\"", operator.name());
        }
        if rule.allow_empty {
            out.push_str(" allowBlank=\"1\"");
        }
        out.push_str(" showInputMessage=\"1\" showErrorMessage=\"1\"");
        optional_attr(&mut out, "errorTitle", &rule.error_title);
        optional_attr(&mut out, "error", &rule.error_message);
        optional_attr(&mut out, "promptTitle", &rule.input_title);
        optional_attr(&mut out, "prompt", &rule.input_message);
        let _ = write!(out, " sqref=\"{}\">", range.replace('$', ""));
        for (at, formula) in formulas.iter().enumerate() {
            let _ = write!(out, "<formula{}>", at + 1);
            xml::text(&mut out, formula);
            let _ = write!(out, "</formula{}>", at + 1);
        }
        out.push_str("</dataValidation>");
        self.validations.push(out);
        Ok(())
    }

    fn page(&mut self, print: Print) -> Result<(), Error> {
        limited("print header", &print.header, MAX_HEADER)?;
        limited("print footer", &print.footer, MAX_HEADER)?;
        for fit in [print.fit_width, print.fit_height].into_iter().flatten() {
            if fit > 32_767 {
                return Err(Error::Invalid(format!(
                    "Fit to {fit} pages is not allowed. The number of pages is from 0 to 32,767."
                )));
            }
        }
        if let Some(margins) = print.margins {
            for side in [margins.top, margins.right, margins.bottom, margins.left] {
                if !(0.0..=49.0).contains(&side) {
                    return Err(Error::Invalid(format!(
                        "The page margin {side} is not allowed. A margin is from 0 to 49 inches."
                    )));
                }
            }
        }
        if let Some(rows) = &print.repeat_rows
            && !matches!(span(rows), Some(Span::Rows(..)))
        {
            return Err(Error::Invalid(format!(
                "The rows to repeat `{rows}` are not a range of rows, such as `1:2`."
            )));
        }
        if let Some(range) = &print.area {
            area(range).ok_or_else(|| bad_range("print", range))?;
        }
        self.print = Some(print);
        Ok(())
    }

    fn protection(&mut self, protect: &Protect) -> Result<(), Error> {
        limited("password", &protect.password, 255)?;
        if let Some(name) = protect
            .allow
            .iter()
            .find(|name| !PROTECT_ALLOWS.contains(&name.as_str()))
        {
            return Err(Error::Invalid(format!(
                "`{name}` is not something a protected sheet can allow. The names are {}.",
                PROTECT_ALLOWS.join(", ")
            )));
        }
        let allows = |name: &str| protect.allow.iter().any(|allowed| allowed == name);
        let mut out = String::from("<sheetProtection");
        if let Some(password) = &protect.password
            && !password.is_empty()
        {
            let _ = write!(out, " password=\"{:04X}\"", password_hash(password));
        }
        out.push_str(" sheet=\"1\"");
        for name in ["objects", "scenarios"] {
            if !allows(name) {
                let _ = write!(out, " {name}=\"1\"");
            }
        }
        for name in &PROTECT_ALLOWS[2..] {
            if allows(name) && !name.starts_with("select") {
                let _ = write!(out, " {name}=\"0\"");
            }
        }
        out.push_str("/>");
        self.protect = Some(out);
        Ok(())
    }

    /// Checks `image` and keeps its anchor, `media` being the number its entry takes in the
    /// workbook. Returns its format.
    pub fn image(&mut self, image: &Image, media: usize) -> Result<Format, Error> {
        self.drawing.add(image, media)
    }

    /// Checks `chart` and keeps it with its anchor (`Drawing::chart`).
    pub fn chart(
        &mut self,
        chart: &Chart,
        number: usize,
        sheet: &str,
        reach: u32,
    ) -> Result<(), Error> {
        self.drawing.chart(chart, number, sheet, reach)
    }

    /// Whether the sheet has notes.
    pub fn has_notes(&self) -> bool {
        !self.notes.is_empty()
    }

    /// Whether the sheet has images.
    pub fn has_drawing(&self) -> bool {
        !self.drawing.is_empty()
    }

    /// The `rId` of the drawing: after the links that leave the workbook and the notes' two.
    fn drawing_id(&self) -> usize {
        self.external.len() + if self.has_notes() { 2 } else { 0 } + 1
    }

    /// The `rId` of the table at `at` in the sheet's order: after the drawing.
    fn table_id(&self, at: usize) -> usize {
        self.drawing_id() + usize::from(self.has_drawing()) + at
    }

    /// Keeps `table` when it overlaps no merged range and no other table of the sheet `sheet`.
    pub fn table(&mut self, table: Placed, sheet: &str) -> Result<(), Error> {
        self.check_table(&table, sheet)?;
        self.tables.push(table);
        Ok(())
    }

    /// Checks that `table` overlaps no merged range and no other table of the sheet `sheet`.
    pub fn check_table(&self, table: &Placed, sheet: &str) -> Result<(), Error> {
        let whole = table.whole();
        if self.merges.iter().any(|merged| merged.overlaps(&whole)) {
            return Err(Error::Invalid(format!(
                "The table `{}` overlaps a merged range of the sheet `{sheet}`. A table has no merged cells.",
                table.name
            )));
        }
        if let Some(other) = self.tables.iter().find(|t| t.whole().overlaps(&whole)) {
            return Err(Error::Invalid(format!(
                "The table `{}` overlaps the table `{}` of the sheet `{sheet}`.",
                table.name, other.name
            )));
        }
        Ok(())
    }

    /// Removes the table numbered `id`, whose totals row failed.
    pub fn forget_table(&mut self, id: usize) {
        self.tables.retain(|table| table.id != id);
    }

    /// How many of `count` rows numbered from `next` come before a table's totals row, when one
    /// falls among them.
    pub fn totals_due(&self, next: u32, count: usize) -> Option<usize> {
        self.tables
            .iter()
            .filter(|table| table.owes_totals() && table.area.bottom >= next)
            .map(|table| (table.area.bottom - next) as usize + 1)
            .filter(|&cut| cut < count + 1)
            .min()
    }

    /// The totals row of the table whose last row is the one before `next`, with the table's id.
    pub fn totals(&self, next: u32) -> Option<(usize, Result<Row, Error>)> {
        let table = self
            .tables
            .iter()
            .find(|table| table.owes_totals() && table.area.bottom + 1 == next)?;
        Some((table.id, table.totals_row()))
    }

    /// Records that the totals row of the table numbered `id` is written.
    pub fn totaled(&mut self, id: usize) {
        if let Some(table) = self.tables.iter_mut().find(|table| table.id == id) {
            table.totaled();
        }
    }

    /// Whether the area `at` overlaps a table.
    pub fn overlaps_table(&self, at: &Area) -> bool {
        self.tables.iter().any(|table| table.whole().overlaps(at))
    }

    /// The name of the table whose rows hold the cell at `column` and `row`, if any.
    pub fn table_at(&self, column: u32, row: u32) -> Option<&str> {
        self.tables
            .iter()
            .find(|table| table.covers(column, row))
            .map(|table| table.name.as_str())
    }

    /// Names the columns of each table whose header row is among `rows`, the first of them
    /// numbered `first`. A header that fails names nothing.
    pub fn headers(&mut self, rows: &[Row], first: u32) -> Result<(), Error> {
        let mut named = Vec::new();
        for (at, table) in self.tables.iter().enumerate() {
            let Some(offset) = table.area.top.checked_sub(first) else {
                continue;
            };
            if let Some(row) = rows.get(offset as usize)
                && !table.has_header()
            {
                let mut table = table.clone();
                table.header(row)?;
                named.push((at, table));
            }
        }
        for (at, table) in named {
            self.tables[at] = table;
        }
        Ok(())
    }

    /// A table whose header row or totals row is not written yet.
    pub fn unfinished_table(&self) -> Option<&Placed> {
        self.tables
            .iter()
            .find(|table| !table.has_header() || table.owes_totals())
    }

    /// The tables' entries, `xl/tables/table<id>.xml`, by name.
    pub fn tables(&self) -> Vec<(String, String)> {
        self.tables
            .iter()
            .map(|table| (format!("xl/tables/table{}.xml", table.id), table.write()))
            .collect()
    }

    /// The print area and the rows to repeat, as the names `workbook.xml` defines for the sheet,
    /// each with its reference in the sheet.
    pub fn print_names(&self) -> Vec<(&'static str, String)> {
        let Some(print) = &self.print else {
            return Vec::new();
        };
        let mut names = Vec::new();
        if let Some(range) = &print.area
            && let Some(at) = area(range)
        {
            names.push((
                "_xlnm.Print_Area",
                format!(
                    "${}${}:${}${}",
                    letters(at.left),
                    at.top,
                    letters(at.right),
                    at.bottom
                ),
            ));
        }
        if let Some(rows) = &print.repeat_rows
            && let Some(Span::Rows(from, to)) = span(rows)
        {
            names.push(("_xlnm.Print_Titles", format!("${from}:${to}")));
        }
        names
    }

    /// Writes the parts between `</sheetData>` and `</worksheet>` into `out`, `filter` being the
    /// sheet's autofilter.
    pub fn write(&self, out: &mut String, filter: Option<&str>) {
        if let Some(protect) = &self.protect {
            out.push_str(protect);
        }
        if let Some(range) = filter {
            let _ = write!(out, "<autoFilter ref=\"{range}\"/>");
        }
        if !self.merges.is_empty() {
            let _ = write!(out, "<mergeCells count=\"{}\">", self.merges.len());
            for merged in &self.merges {
                let _ = write!(
                    out,
                    "<mergeCell ref=\"{}:{}\"/>",
                    cells::reference(merged.left, merged.top),
                    cells::reference(merged.right, merged.bottom)
                );
            }
            out.push_str("</mergeCells>");
        }
        for conditional in &self.conditionals {
            out.push_str(conditional);
        }
        if !self.validations.is_empty() {
            let _ = write!(
                out,
                "<dataValidations count=\"{}\">",
                self.validations.len()
            );
            for validation in &self.validations {
                out.push_str(validation);
            }
            out.push_str("</dataValidations>");
        }
        if !self.links.is_empty() {
            out.push_str("<hyperlinks>");
            for (_, link) in &self.links {
                out.push_str(link);
            }
            out.push_str("</hyperlinks>");
        }
        if let Some(print) = &self.print {
            let margins = print.margins.unwrap_or(DEFAULT_MARGINS);
            out.push_str("<pageMargins");
            let mut side = |name: &str, value: f64| {
                let _ = write!(out, " {name}=\"");
                cells::number(out, value);
                out.push('"');
            };
            side("left", margins.left);
            side("right", margins.right);
            side("top", margins.top);
            side("bottom", margins.bottom);
            out.push_str(" header=\"0.3\" footer=\"0.3\"/>");
            if print.orientation.is_some() || print.fits() {
                out.push_str("<pageSetup");
                if let Some(orientation) = print.orientation {
                    let name = match orientation {
                        Orientation::Portrait => "portrait",
                        Orientation::Landscape => "landscape",
                    };
                    let _ = write!(out, " orientation=\"{name}\"");
                }
                if print.fits() {
                    let _ = write!(
                        out,
                        " fitToWidth=\"{}\" fitToHeight=\"{}\"",
                        print.fit_width.unwrap_or(0),
                        print.fit_height.unwrap_or(0)
                    );
                }
                out.push_str("/>");
            }
            if print.header.is_some() || print.footer.is_some() {
                out.push_str("<headerFooter>");
                for (tag, text) in [("oddHeader", &print.header), ("oddFooter", &print.footer)] {
                    if let Some(text) = text {
                        let _ = write!(out, "<{tag}>");
                        xml::text(out, text);
                        let _ = write!(out, "</{tag}>");
                    }
                }
                out.push_str("</headerFooter>");
            }
        }
        if self.has_drawing() {
            let _ = write!(out, "<drawing r:id=\"rId{}\"/>", self.drawing_id());
        }
        if self.has_notes() {
            let _ = write!(
                out,
                "<legacyDrawing r:id=\"rId{}\"/>",
                self.external.len() + 2
            );
        }
        if !self.tables.is_empty() {
            let _ = write!(out, "<tableParts count=\"{}\">", self.tables.len());
            for at in 0..self.tables.len() {
                let _ = write!(out, "<tablePart r:id=\"rId{}\"/>", self.table_id(at));
            }
            out.push_str("</tableParts>");
        }
    }

    /// The sheet's relationships part, when it has any: the links that leave the workbook, then
    /// the notes and their VML drawing, then the images' drawing. `sheet` is the sheet's number in
    /// its part names.
    pub fn rels(&self, sheet: usize) -> Option<String> {
        if self.external.is_empty()
            && !self.has_notes()
            && !self.has_drawing()
            && self.tables.is_empty()
        {
            return None;
        }
        let mut out = String::from(crate::writer::XML_HEAD);
        let _ = write!(out, "<Relationships xmlns=\"{PACKAGE_RELS}\">");
        for (at, target) in self.external.iter().enumerate() {
            let _ = write!(
                out,
                "<Relationship Id=\"rId{}\" Type=\"{RELS}/hyperlink\" Target=\"",
                at + 1
            );
            xml::attr(&mut out, target);
            out.push_str("\" TargetMode=\"External\"/>");
        }
        if self.has_notes() {
            let next = self.external.len() + 1;
            let _ = write!(
                out,
                "<Relationship Id=\"rId{next}\" Type=\"{RELS}/comments\" Target=\"../comments{sheet}.xml\"/>\
                 <Relationship Id=\"rId{}\" Type=\"{RELS}/vmlDrawing\" Target=\"../drawings/vmlDrawing{sheet}.vml\"/>",
                next + 1
            );
        }
        if self.has_drawing() {
            let _ = write!(
                out,
                "<Relationship Id=\"rId{}\" Type=\"{RELS}/drawing\" Target=\"../drawings/drawing{sheet}.xml\"/>",
                self.drawing_id()
            );
        }
        for (at, table) in self.tables.iter().enumerate() {
            let _ = write!(
                out,
                "<Relationship Id=\"rId{}\" Type=\"{RELS}/table\" Target=\"../tables/table{}.xml\"/>",
                self.table_id(at),
                table.id
            );
        }
        out.push_str("</Relationships>");
        Some(out)
    }

    /// The drawing's entries by name: `drawing<sheet>.xml`, its relationships and each chart
    /// part, whose growing ranges end at the row `last`, over the sheet's `grid`. Empty when the
    /// sheet has no images and no charts.
    pub fn drawing(&self, sheet: usize, last: u32, grid: &Grid) -> Vec<(String, String)> {
        if !self.has_drawing() {
            return Vec::new();
        }
        let mut entries = vec![
            (
                format!("xl/drawings/drawing{sheet}.xml"),
                self.drawing.write(grid),
            ),
            (
                format!("xl/drawings/_rels/drawing{sheet}.xml.rels"),
                self.drawing.rels(),
            ),
        ];
        entries.extend(self.drawing.charts(last, grid));
        entries
    }

    /// The notes' two entries, `comments<sheet>.xml` and the VML drawing, by name. Empty when the
    /// sheet has no notes.
    pub fn notes(&self, sheet: usize) -> Vec<(String, String)> {
        if !self.has_notes() {
            return Vec::new();
        }
        let mut authors: Vec<&str> = Vec::new();
        for note in &self.notes {
            let author = note.author.as_deref().unwrap_or("");
            if !authors.contains(&author) {
                authors.push(author);
            }
        }
        let mut comments = String::from(crate::writer::XML_HEAD);
        let _ = write!(comments, "<comments xmlns=\"{MAIN}\"><authors>");
        for author in &authors {
            comments.push_str("<author>");
            xml::text(&mut comments, author);
            comments.push_str("</author>");
        }
        comments.push_str("</authors><commentList>");
        let mut vml = String::from(
            "<xml xmlns:v=\"urn:schemas-microsoft-com:vml\" xmlns:o=\"urn:schemas-microsoft-com:office:office\" xmlns:x=\"urn:schemas-microsoft-com:office:excel\">",
        );
        let _ = write!(
            vml,
            "<o:shapelayout v:ext=\"edit\"><o:idmap v:ext=\"edit\" data=\"{sheet}\"/></o:shapelayout>\
             <v:shapetype id=\"_x0000_t202\" coordsize=\"21600,21600\" o:spt=\"202\" path=\"m,l,21600r21600,l21600,xe\">\
             <v:stroke joinstyle=\"miter\"/><v:path gradientshapeok=\"t\" o:connecttype=\"rect\"/></v:shapetype>"
        );
        for ((at, note), place) in self.notes.iter().enumerate().zip(&self.noted) {
            let author = note.author.as_deref().unwrap_or("");
            let id = authors.iter().position(|a| *a == author).unwrap_or(0);
            let cell = cells::reference(place.left, place.top);
            let _ = write!(
                comments,
                "<comment ref=\"{cell}\" authorId=\"{id}\"><text><r>"
            );
            if xml::needs_preserve(&note.note) {
                comments.push_str("<t xml:space=\"preserve\">");
            } else {
                comments.push_str("<t>");
            }
            xml::text(&mut comments, &note.note);
            comments.push_str("</t></r></text></comment>");
            let (column, row) = (place.left, place.top - 1);
            let top = row.saturating_sub(1);
            let _ = write!(
                vml,
                "<v:shape id=\"_x0000_s{}\" type=\"#_x0000_t202\" style=\"position:absolute;margin-left:59.25pt;margin-top:1.5pt;width:108pt;height:59.25pt;z-index:{};visibility:hidden\" fillcolor=\"#ffffe1\" o:insetmode=\"auto\">\
                 <v:fill color2=\"#ffffe1\"/><v:shadow on=\"t\" color=\"black\" obscured=\"t\"/><v:path o:connecttype=\"none\"/>\
                 <v:textbox style=\"mso-direction-alt:auto\"><div style=\"text-align:left\"></div></v:textbox>\
                 <x:ClientData ObjectType=\"Note\"><x:MoveWithCells/><x:SizeWithCells/>\
                 <x:Anchor>{}, 15, {top}, 10, {}, 15, {}, 4</x:Anchor><x:AutoFill>False</x:AutoFill>\
                 <x:Row>{row}</x:Row><x:Column>{column}</x:Column></x:ClientData></v:shape>",
                sheet * 1024 + 1 + at,
                at + 1,
                column + 1,
                column + 3,
                top + 4
            );
        }
        comments.push_str("</commentList></comments>");
        vml.push_str("</xml>");
        vec![
            (format!("xl/comments{sheet}.xml"), comments),
            (format!("xl/drawings/vmlDrawing{sheet}.vml"), vml),
        ]
    }
}

fn letters(column: u32) -> String {
    let mut out = String::new();
    cells::column_letters(&mut out, column);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_password_hash_is_excels() {
        assert_eq!(format!("{:04X}", password_hash("password")), "83AF");
    }

    #[test]
    fn ranges_read_as_their_corners() {
        assert_eq!(
            area("$B$2:D9"),
            Some(Area {
                left: 1,
                top: 2,
                right: 3,
                bottom: 9
            })
        );
        assert_eq!(area("D9:B2"), None);
        assert_eq!(area("A0"), None);
        assert_eq!(cell_at("XFD1048576"), Some((16_383, 1_048_576)));
        assert_eq!(cell_at("XFE1"), None);
    }
}
