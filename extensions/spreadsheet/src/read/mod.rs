//! The reader: a workbook read through byte ranges of a source the program holds
//! (`rule:core-classes/spreadsheet-reads-as-a-stream`).
//!
//! **This is our own thin reader, not calamine.** calamine keeps one `String` per shared
//! string, exposes no cell style, and reads a sheet from a `Read + Seek` it owns, where a
//! reader resource must keep its place between guest calls. So the zip directory and the parts
//! are read here ([`zip`]), the XML is tokenized here ([`xml`]), and a sheet is a [`sheet::Cursor`]
//! that survives from one call to the next. xlsb is the same package with binary parts, read
//! the same way by [`binary`]. xlsm is an xlsx with a VBA part, and reads as one.
//! [`Reader::macros`] lists the VBA project's module names ([`vba`]), and nothing runs them.
//!
//! **xls and ods are read whole**, at [`Reader::open`]: [`xls`] over the compound file [`cfb`],
//! and [`ods`]. Each sheet's rows are kept in its [`Listed`], and a cursor walks them. calamine
//! does not read them here either. Its release with a sound `quick-xml` needs a `zip` that does
//! not build for `wasm32-wasip2`, and its release that builds carries `quick-xml`'s two open
//! advisories, which `deny.toml` refuses.
//!
//! [`Reader::open`] reads the zip directory, the package's relationships, the workbook part, its
//! relationships and the styles part, and nothing of a sheet. The shared strings are read at the
//! first call that reads cells, into one arena with `u32` ends ([`Strings`]). Each cell becomes
//! its read type by `value`: the read table of `rule:core-classes/spreadsheet-value-types`, with
//! each cell's style index giving the number format that makes a number a date, a time, a clock
//! time or a duration ([`format`]).
//!
//! **One cursor at a time.** [`Reader::rows`] continues the open cursor when it is on the same
//! sheet and `from` is past the last row it read, and opens a new one otherwise. So the Novis
//! half asks for its next batch with `from` one past the last row it was given, and a new loop
//! starts the sheet again. `limit` is the most rows this call may give.
//!
//! **Merged ranges.** [`Reader::merged`] lists a sheet's ranges. A read with `merged: 'fill'`
//! reads them when it opens its cursor, at one more pass over an xlsx or xlsb sheet's part, and
//! gives each cell of a range the range's top-left value ([`merge`]). A read without it gives the
//! value in the top-left cell only.
//!
//! **The other extras**, a sheet's hyperlinks, notes, tables and images and the workbook's names
//! and properties, are [`extras`]' module doc.
//!
//! **Time values.** A serial is rounded to the millisecond, the precision Excel shows. In the 1900
//! system a serial below 61 is one day later than its count, because Excel counts the day
//! 1900-02-29, which did not exist. Serial 60 reads as 1900-03-01.
//!
//! What it spends, per reading request: the zip directory, the styles' kinds, the shared strings
//! in one arena, one block of compressed bytes and the inflater's window, one block of sheet XML
//! and the row being parsed, and one batch of rows. A fill adds 32 bytes per merged range of the
//! sheet, and the top-left value of each range the read is inside.

pub mod binary;
mod cfb;
pub mod extras;
pub mod format;
pub mod merge;
mod ods;
mod ods_styles;
pub mod sheet;
pub mod tokens;
mod vba;
mod xls;
mod xls_extras;
pub mod xml;
pub mod zip;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::format::{Locale, Options, Value};
use crate::{Clock, Date, Error, TimeOfDay, Visibility};
use format::{Kind, Style};
use merge::{Fill, Merge};
use xml::Token;

/// A source the reader reads byte ranges of.
pub trait Source {
    /// The source's length in bytes.
    fn size(&self) -> Result<u64, Error>;
    /// At most `length` bytes from `offset`, fewer at the end.
    fn range(&self, offset: u64, length: u64) -> Result<Vec<u8>, Error>;
}

impl Source for [u8] {
    fn size(&self) -> Result<u64, Error> {
        Ok(self.len() as u64)
    }

    fn range(&self, offset: u64, length: u64) -> Result<Vec<u8>, Error> {
        let start = usize::try_from(offset).map_or(self.len(), |at| at.min(self.len()));
        let take = usize::try_from(length).map_or(usize::MAX, |n| n);
        Ok(self[start..start + take.min(self.len() - start)].to_vec())
    }
}

/// An error a cell holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellError {
    Null,
    DivZero,
    Value,
    Ref,
    Name,
    Num,
    NotAvailable,
    GettingData,
    Spill,
    Calc,
}

impl CellError {
    fn of(text: &str) -> Option<Self> {
        Some(match text {
            "#NULL!" => Self::Null,
            "#DIV/0!" => Self::DivZero,
            "#VALUE!" => Self::Value,
            "#REF!" => Self::Ref,
            "#NAME?" => Self::Name,
            "#NUM!" => Self::Num,
            "#N/A" => Self::NotAvailable,
            "#GETTING_DATA" => Self::GettingData,
            "#SPILL!" => Self::Spill,
            "#CALC!" => Self::Calc,
            _ => return None,
        })
    }

    /// The text Excel shows for the error.
    pub fn text(self) -> &'static str {
        match self {
            Self::Null => "#NULL!",
            Self::DivZero => "#DIV/0!",
            Self::Value => "#VALUE!",
            Self::Ref => "#REF!",
            Self::Name => "#NAME?",
            Self::Num => "#NUM!",
            Self::NotAvailable => "#N/A",
            Self::GettingData => "#GETTING_DATA",
            Self::Spill => "#SPILL!",
            Self::Calc => "#CALC!",
        }
    }
}

/// One read cell's value, by the read table.
#[derive(Debug, Clone, PartialEq)]
pub enum Read {
    Text(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    Clock(Clock),
    Date(Date),
    Time(TimeOfDay),
    /// Nanoseconds.
    Duration(i64),
    Error(CellError),
}

/// One read cell. `column` counts from 1.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadCell {
    pub column: u64,
    pub value: Option<Read>,
    pub formula: Option<String>,
    pub display: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadRow {
    pub number: u64,
    pub cells: Vec<ReadCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub rows: Vec<ReadRow>,
    pub done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetKind {
    Worksheet,
    Chartsheet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SheetInfo {
    pub name: String,
    pub position: u64,
    pub visibility: Visibility,
    pub rows: u64,
    pub columns: u64,
    pub kind: SheetKind,
}

/// The selection and the defaults of a read (`wit/spreadsheet.wit`'s `read-options`).
#[derive(Debug, Clone, Default)]
pub struct ReadOptions {
    pub sheet: String,
    pub header_row: Option<u64>,
    pub from: Option<u64>,
    pub limit: Option<u64>,
    pub columns: Vec<String>,
    pub text: Vec<String>,
    pub text_all: bool,
    pub formulas: bool,
    pub keep_empty_rows: bool,
    pub fill_merged: bool,
    pub skip_hidden: bool,
    pub locale: String,
}

/// The shared strings, in one arena: string `i` is `text[ends[i - 1]..ends[i]]`.
#[derive(Debug, Default)]
pub struct Strings {
    text: String,
    ends: Vec<u32>,
}

impl Strings {
    /// The number of strings.
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    /// The arena's length in bytes, which is every string's bytes and nothing else.
    pub fn arena(&self) -> usize {
        self.text.len()
    }

    pub fn get(&self, at: usize) -> Option<&str> {
        let end = *self.ends.get(at)? as usize;
        let start = at
            .checked_sub(1)
            .map_or(0, |before| self.ends[before] as usize);
        self.text.get(start..end)
    }

    fn push(&mut self, text: &str) -> Result<(), Error> {
        self.push_text(&xml::unescape(text.to_owned()))
    }

    /// Adds `text` as it is, with no XML escapes to undo.
    fn push_text(&mut self, text: &str) -> Result<(), Error> {
        self.text.push_str(text);
        let end = u32::try_from(self.text.len()).map_err(|_| {
            Error::Parse("The workbook's shared strings are larger than 4 GiB.".to_owned())
        })?;
        self.ends.push(end);
        Ok(())
    }

    /// Reads the shared strings part `entry` a block at a time, so only the arena is held.
    fn parse(source: &dyn Source, entry: &zip::Entry) -> Result<Self, Error> {
        let mut part = zip::Part::open(source, entry)?;
        let mut buf = Vec::new();
        let mut eof = false;
        let mut strings = Self::default();
        let mut at = 0;
        let (mut item, mut inside_t, mut phonetic) = (String::new(), false, false);
        loop {
            let Some((token, size)) = xml::next(&buf[at..])? else {
                if eof {
                    break;
                }
                buf.drain(..at);
                at = 0;
                eof = !part.fill(source, &mut buf)?;
                continue;
            };
            at += size;
            match token {
                Token::Open { name, empty, .. } => match xml::local(name) {
                    b"si" if empty => strings.push("")?,
                    b"si" => item.clear(),
                    b"t" if !empty && !phonetic => inside_t = true,
                    b"rPh" if !empty => phonetic = true,
                    _ => {}
                },
                Token::Text(raw) if inside_t => xml::decode(raw, &mut item)?,
                Token::Raw(raw) if inside_t => item.push_str(&String::from_utf8_lossy(raw)),
                Token::Close(name) => match xml::local(name) {
                    b"si" => strings.push(&item)?,
                    b"t" => inside_t = false,
                    b"rPh" => phonetic = false,
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(strings)
    }
}

/// A sheet as the workbook lists it. A sheet of a format read whole has its rows in `rows` and
/// its merged ranges in `merged`.
#[derive(Debug, Clone)]
struct Listed {
    name: String,
    visibility: Visibility,
    part: Option<String>,
    kind: SheetKind,
    rows: Option<Arc<Vec<sheet::RawRow>>>,
    merged: Vec<Merge>,
}

/// A workbook being read.
#[derive(Debug)]
pub struct Reader {
    directory: zip::Directory,
    sheets: Vec<Listed>,
    /// The workbook part of a zip package: an xlsx, xlsm or xlsb.
    workbook: Option<String>,
    strings_part: Option<String>,
    strings: Option<Strings>,
    /// The number format of each cell style index: the read type it gives a number, and its
    /// code for the renderer.
    styles: Vec<Style>,
    date1904: bool,
    /// Whether the parts are xlsb's records.
    binary: bool,
    /// Whether the file is a compound file, an xls.
    compound: bool,
    /// The VBA project's part, when the package has one.
    vba_part: Option<String>,
    /// The links, notes, pictures, names and tables of an ods, read with its cells.
    ods: Option<Box<ods::Extras>>,
    /// The links, notes, pictures, names and properties of an xls, read with its cells.
    xls: Option<Box<xls_extras::Extras>>,
    /// What an xlsb's formulas refer to by number.
    book: Arc<tokens::Book>,
    open: Option<Open>,
}

/// The sheet [`Reader::rows`] is reading: its cursor, a row read and held back while the empty
/// rows before it are given, the number of the next row a call continues from, and the merged
/// ranges of a read that fills them.
#[derive(Debug)]
struct Open {
    sheet: String,
    cursor: Cursor,
    held: Option<sheet::RawRow>,
    next: u64,
    fill: Option<Fill>,
    /// The `text` and `text_all` the cursor was opened for, and the column ranges they name.
    text: (Vec<String>, bool),
    shown: Vec<(u64, u64)>,
    /// Whether the cursor gives each formula's text.
    formulas: bool,
}

/// A sheet's cursor, over XML, over records, or over the rows of a sheet read whole.
#[derive(Debug)]
enum Cursor {
    Xml(sheet::Cursor),
    Binary(binary::Cursor),
    Whole {
        rows: Arc<Vec<sheet::RawRow>>,
        at: usize,
    },
}

impl Cursor {
    fn dimension(&mut self, source: &dyn Source) -> Result<Option<(u64, u64)>, Error> {
        match self {
            Self::Xml(cursor) => cursor.dimension(source),
            Self::Binary(cursor) => cursor.dimension(source),
            Self::Whole { rows, .. } => Ok(rows.last().map(|last| {
                let columns = rows
                    .iter()
                    .flat_map(|row| row.cells.last())
                    .map(|cell| cell.column)
                    .max()
                    .unwrap_or(0);
                (last.number, columns)
            })),
        }
    }

    fn next(&mut self, source: &dyn Source) -> Result<Option<sheet::RawRow>, Error> {
        match self {
            Self::Xml(cursor) => cursor.next(source),
            Self::Binary(cursor) => cursor.next(source),
            Self::Whole { rows, at } => {
                let row = rows.get(*at).cloned();
                *at += 1;
                Ok(row)
            }
        }
    }
}

fn broken(what: &str) -> Error {
    Error::Parse(format!("The file is not a valid workbook: {what}."))
}

/// `target` resolved against the folder of the part `base`, as a relationship's target is.
fn resolve(base: &str, target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.to_owned();
    }
    let mut parts: Vec<&str> = base.split('/').collect();
    parts.pop();
    for segment in target.split('/') {
        match segment {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            segment => parts.push(segment),
        }
    }
    parts.join("/")
}

/// The relationships part of `part`: each id, its type's last segment, its target and whether it
/// is external. An external target is kept as text and never followed.
fn relationships(
    source: &dyn Source,
    directory: &zip::Directory,
    part: &str,
) -> Result<Vec<(String, String, String, bool)>, Error> {
    let (folder, file) = part.rsplit_once('/').unwrap_or(("", part));
    let rels = if folder.is_empty() {
        format!("_rels/{file}.rels")
    } else {
        format!("{folder}/_rels/{file}.rels")
    };
    let Some(entry) = directory.find(&rels) else {
        return Ok(Vec::new());
    };
    let bytes = directory.whole(source, entry)?;
    let mut out = Vec::new();
    let mut at = 0;
    while let Some((token, size)) = xml::next(&bytes[at..])? {
        at += size;
        if let Token::Open { name, attrs, .. } = token
            && xml::local(name) == b"Relationship"
        {
            let id = xml::attr_text(attrs, b"Id")?.unwrap_or_default();
            let kind = xml::attr_text(attrs, b"Type")?.unwrap_or_default();
            let kind = kind.rsplit('/').next().unwrap_or_default().to_owned();
            let target = xml::attr_text(attrs, b"Target")?.unwrap_or_default();
            let external = xml::attr(attrs, b"TargetMode") == Some(b"External");
            let target = if external {
                target
            } else {
                resolve(part, &target)
            };
            out.push((id, kind, target, external));
        }
    }
    Ok(out)
}

/// The number format of each cell style index in `styles.xml`.
fn styles(part: &[u8]) -> Result<Vec<Style>, Error> {
    let mut codes: Vec<(u32, Arc<str>)> = Vec::new();
    let mut styles = Vec::new();
    let mut in_xfs = false;
    let mut at = 0;
    while let Some((token, size)) = xml::next(&part[at..])? {
        at += size;
        match token {
            Token::Open { name, attrs, empty } => match xml::local(name) {
                b"numFmt" => {
                    let id = xml::attr(attrs, b"numFmtId")
                        .and_then(|id| std::str::from_utf8(id).ok()?.parse().ok());
                    if let (Some(id), Some(code)) = (id, xml::attr_text(attrs, b"formatCode")?) {
                        codes.push((id, Arc::from(code)));
                    }
                }
                b"cellXfs" => in_xfs = !empty,
                b"xf" if in_xfs => {
                    let id: u32 = xml::attr(attrs, b"numFmtId")
                        .and_then(|id| std::str::from_utf8(id).ok()?.parse().ok())
                        .unwrap_or(0);
                    let custom = codes.iter().find(|(code, _)| *code == id);
                    styles.push(Style::of(id, custom.map(|(_, code)| code)));
                }
                _ => {}
            },
            Token::Close(name) if xml::local(name) == b"cellXfs" => in_xfs = false,
            _ => {}
        }
    }
    Ok(styles)
}

impl Reader {
    /// Opens the workbook `source` is: its directory and its workbook part. A zip archive's
    /// parts are read under `bound`.
    pub fn open(source: &dyn Source, bound: zip::Bound) -> Result<Self, Error> {
        let head = source.range(0, 8)?;
        if cfb::is_compound(&head) {
            let book = xls::read(&source.range(0, source.size()?)?)?;
            return Ok(Self {
                compound: true,
                xls: Some(Box::new(book.extras)),
                ..Self::whole(
                    zip::Directory::default(),
                    book.sheets,
                    book.styles,
                    book.date1904,
                )
            });
        }
        if !head.starts_with(b"PK") {
            return Err(Error::Parse(
                "The file is not a workbook. Novis reads xlsx, xlsm, xlsb, xls and ods files."
                    .to_owned(),
            ));
        }
        let directory = zip::Directory::read(source, bound)?;
        let workbook = relationships(source, &directory, "")?
            .into_iter()
            .find(|(_, kind, _, _)| kind == "officeDocument")
            .map_or_else(|| "xl/workbook.xml".to_owned(), |(_, _, target, _)| target);
        let binary = workbook.ends_with(".bin");
        let Some(entry) = directory.find(&workbook) else {
            if directory.find("content.xml").is_none() {
                return Err(broken("it has no workbook part"));
            }
            let book = ods::read(source, &directory)?;
            return Ok(Self {
                ods: Some(Box::new(book.extras)),
                ..Self::whole(directory, book.sheets, book.styles, false)
            });
        };
        let book = directory.whole(source, entry)?;
        let rels = relationships(source, &directory, &workbook)?;
        let (date1904, listed, formulas) = if binary {
            let book = binary::workbook(&book)?;
            (book.date1904, book.sheets, book.book)
        } else {
            let mut listed = Vec::new();
            let mut date1904 = false;
            let mut at = 0;
            while let Some((token, size)) = xml::next(&book[at..])? {
                at += size;
                let Token::Open { name, attrs, .. } = token else {
                    continue;
                };
                match xml::local(name) {
                    b"workbookPr" => {
                        date1904 = matches!(xml::attr(attrs, b"date1904"), Some(b"1" | b"true"));
                    }
                    b"sheet" => {
                        let name = xml::attr_text(attrs, b"name")?
                            .ok_or_else(|| broken("a sheet has no name"))?;
                        let visibility = match xml::attr(attrs, b"state") {
                            Some(b"hidden") => Visibility::Hidden,
                            Some(b"veryHidden") => Visibility::VeryHidden,
                            _ => Visibility::Visible,
                        };
                        let id = xml::attr_text(attrs, b"id")?.unwrap_or_default();
                        listed.push((name, visibility, id));
                    }
                    _ => {}
                }
            }
            (date1904, listed, tokens::Book::default())
        };
        let sheets = listed
            .into_iter()
            .map(|(name, visibility, id)| {
                let rel = rels.iter().find(|(rid, _, _, _)| *rid == id);
                Listed {
                    name,
                    visibility,
                    part: rel.filter(|rel| !rel.3).map(|rel| rel.2.clone()),
                    kind: match rel {
                        Some((_, kind, _, _)) if kind == "chartsheet" => SheetKind::Chartsheet,
                        _ => SheetKind::Worksheet,
                    },
                    rows: None,
                    merged: Vec::new(),
                }
            })
            .collect();
        let part_of = |wanted: &str| {
            rels.iter()
                .find(|(_, kind, _, external)| kind == wanted && !external)
                .map(|(_, _, target, _)| target.clone())
        };
        let styles = match part_of("styles").and_then(|part| directory.find(&part).cloned()) {
            Some(entry) if binary => binary::styles(&directory.whole(source, &entry)?)?,
            Some(entry) => styles(&directory.whole(source, &entry)?)?,
            None => Vec::new(),
        };
        // Excel always names the VBA part in the workbook's relationships. A part at the usual
        // path with no relationship is listed too, so a file never hides a macro.
        let vba_part = part_of("vbaProject")
            .or_else(|| Some("xl/vbaProject.bin".to_owned()))
            .filter(|part| directory.find(part).is_some());
        Ok(Self {
            strings_part: part_of("sharedStrings"),
            vba_part,
            workbook: Some(workbook),
            directory,
            sheets,
            strings: None,
            styles,
            date1904,
            binary,
            compound: false,
            ods: None,
            xls: None,
            book: Arc::new(formulas),
            open: None,
        })
    }

    /// A workbook whose sheets were read whole, with their rows in `sheets`.
    fn whole(
        directory: zip::Directory,
        sheets: Vec<Listed>,
        styles: Vec<Style>,
        date1904: bool,
    ) -> Self {
        Self {
            directory,
            sheets,
            workbook: None,
            strings_part: None,
            strings: None,
            styles,
            date1904,
            binary: false,
            compound: false,
            vba_part: None,
            ods: None,
            xls: None,
            book: Arc::default(),
            open: None,
        }
    }

    /// The names of the workbook's VBA modules, in the project's order, and none when it has no
    /// VBA project. An xlsm's project is its VBA part, and an xls's is its `_VBA_PROJECT_CUR`
    /// storage. Nothing runs them.
    pub fn macros(&self, source: &dyn Source) -> Result<Vec<String>, Error> {
        if self.compound {
            let bytes = source.range(0, source.size()?)?;
            return vba::modules(&cfb::File::open(&bytes)?, &["_VBA_PROJECT_CUR", "VBA"]);
        }
        let Some(entry) = self
            .vba_part
            .as_deref()
            .and_then(|part| self.directory.find(part))
        else {
            return Ok(Vec::new());
        };
        let bytes = self.directory.whole(source, entry)?;
        vba::modules(&cfb::File::open(&bytes)?, &["VBA"])
    }

    /// The shared strings, read at the first call that needs them.
    pub fn strings(&mut self, source: &dyn Source) -> Result<&Strings, Error> {
        if self.strings.is_none() {
            let strings = match self
                .strings_part
                .as_deref()
                .and_then(|part| self.directory.find(part))
            {
                Some(entry) if self.binary => binary::strings(source, entry)?,
                Some(entry) => Strings::parse(source, entry)?,
                None => Strings::default(),
            };
            self.strings = Some(strings);
        }
        Ok(self.strings.get_or_insert_default())
    }

    fn listed(&self, name: &str) -> Result<&Listed, Error> {
        self.sheets
            .iter()
            .find(|sheet| sheet.name == name)
            .ok_or_else(|| Error::Invalid(format!("The workbook has no sheet named `{name}`.")))
    }

    /// The cursor of the sheet `name`. With `formulas`, an xlsb cursor gives each formula's
    /// text. Every other format has it in its rows already.
    fn cursor(&self, source: &dyn Source, name: &str, formulas: bool) -> Result<Cursor, Error> {
        let listed = self.listed(name)?;
        if listed.kind == SheetKind::Chartsheet {
            return Err(Error::Invalid(format!(
                "The sheet `{name}` is a chart. It has no rows to read."
            )));
        }
        if let Some(rows) = &listed.rows {
            return Ok(Cursor::Whole {
                rows: rows.clone(),
                at: 0,
            });
        }
        let entry = self.part(listed)?;
        Ok(if self.binary {
            let book = formulas.then(|| self.book.clone());
            Cursor::Binary(binary::Cursor::open(source, entry, book)?)
        } else {
            Cursor::Xml(sheet::Cursor::open(source, entry)?)
        })
    }

    /// The zip entry of the sheet `listed`, a sheet of a package.
    fn part(&self, listed: &Listed) -> Result<&zip::Entry, Error> {
        listed
            .part
            .as_deref()
            .and_then(|part| self.directory.find(part))
            .ok_or_else(|| broken(&format!("the sheet `{}` has no part", listed.name)))
    }

    /// The merged ranges of the sheet `name`, in the order the file lists them. A chart sheet
    /// has none.
    pub fn merged(&self, source: &dyn Source, name: &str) -> Result<Vec<Merge>, Error> {
        let listed = self.listed(name)?;
        if listed.rows.is_some() || listed.kind == SheetKind::Chartsheet {
            return Ok(listed.merged.clone());
        }
        let entry = self.part(listed)?;
        if self.binary {
            binary::merged(source, entry)
        } else {
            sheet::merged(source, entry)
        }
    }

    /// Every sheet, in tab order.
    pub fn sheets(&self, source: &dyn Source) -> Result<Vec<SheetInfo>, Error> {
        let mut out = Vec::with_capacity(self.sheets.len());
        for (position, listed) in self.sheets.iter().enumerate() {
            let (rows, columns) = if listed.kind == SheetKind::Worksheet {
                self.cursor(source, &listed.name, false)?
                    .dimension(source)?
                    .unwrap_or((0, 0))
            } else {
                (0, 0)
            };
            out.push(SheetInfo {
                name: listed.name.clone(),
                position: position as u64,
                visibility: listed.visibility,
                rows,
                columns,
                kind: listed.kind,
            });
        }
        Ok(out)
    }

    /// The header's keys: each header cell's text, a duplicate numbered, and an empty one its
    /// column letter. The header row is `header_row`, or the first row at or after `from` with a
    /// value in it.
    pub fn header(
        &mut self,
        source: &dyn Source,
        options: &ReadOptions,
    ) -> Result<Vec<String>, Error> {
        let mut cursor = self.cursor(source, &options.sheet, false)?;
        let from = options.from.unwrap_or(1);
        let row = loop {
            let Some(raw) = cursor.next(source)? else {
                return Ok(Vec::new());
            };
            let wanted = match options.header_row {
                Some(header) if raw.number > header => return Ok(Vec::new()),
                Some(header) => raw.number == header,
                None => raw.number >= from,
            };
            if wanted {
                let row = self.row(source, raw, options, &[], Locale::EN)?;
                if options.header_row.is_some() || row.cells.iter().any(|cell| cell.value.is_some())
                {
                    break row;
                }
            }
        };
        let last = row.cells.iter().map(|cell| cell.column).max().unwrap_or(0);
        let values: HashMap<u64, &Read> = row
            .cells
            .iter()
            .filter_map(|cell| Some((cell.column, cell.value.as_ref()?)))
            .collect();
        // A header row of many equal cells is an upload's to send, so a name is found in linear
        // time: `taken` has every name given, and `counts` the last number tried for each base,
        // which every earlier number of that base is already taken past.
        let mut names: Vec<String> = Vec::with_capacity(row.cells.len());
        let mut taken: HashSet<String> = HashSet::with_capacity(row.cells.len());
        let mut counts: HashMap<String, u64> = HashMap::new();
        for column in 1..=last {
            let text = values
                .get(&column)
                .map(|value| header_text(value))
                .filter(|text| !text.is_empty());
            let base = text.unwrap_or_else(|| letters(column));
            let mut name = base.clone();
            if taken.contains(&name) {
                let count = counts.entry(base.clone()).or_insert(1);
                while taken.contains(&name) {
                    *count += 1;
                    name = format!("{base} ({count})");
                }
            }
            taken.insert(name.clone());
            names.push(name);
        }
        Ok(names)
    }

    /// The next batch of at most `max` rows, by the rules in the module doc.
    pub fn rows(
        &mut self,
        source: &dyn Source,
        options: &ReadOptions,
        max: u64,
    ) -> Result<Batch, Error> {
        let from = options
            .from
            .unwrap_or(1)
            .max(options.header_row.map_or(1, |header| header + 1));
        let resume = matches!(&self.open, Some(open) if open.sheet == options.sheet
            && open.next <= from
            && open.fill.is_some() == options.fill_merged
            && open.text.0 == options.text
            && open.text.1 == options.text_all
            && open.formulas == options.formulas);
        if !resume {
            let shown = self.shown(source, options)?;
            let cursor = self.cursor(source, &options.sheet, options.formulas)?;
            let fill = if options.fill_merged {
                Some(Fill::new(self.merged(source, &options.sheet)?))
            } else {
                None
            };
            self.open = Some(Open {
                sheet: options.sheet.clone(),
                cursor,
                held: None,
                next: 1,
                fill,
                text: (options.text.clone(), options.text_all),
                shown,
                formulas: options.formulas,
            });
        }
        let locale = Locale::of_tag(&options.locale);
        let shown = self
            .open
            .as_ref()
            .map(|open| open.shown.clone())
            .unwrap_or_default();
        let selected = selection(&options.columns)?;
        let most = max.min(options.limit.unwrap_or(u64::MAX));
        let mut rows = Vec::new();
        while (rows.len() as u64) < most {
            let Some(open) = self.open.as_mut() else {
                break;
            };
            let raw = match open.held.take() {
                Some(raw) => raw,
                None => match open.cursor.next(source)? {
                    Some(raw) => raw,
                    None => return Ok(Batch { rows, done: true }),
                },
            };
            // A row the file does not write is given when empty rows are kept, or when a fill
            // has a value for it.
            let expected = open.next.max(from);
            let gap = if expected >= raw.number {
                None
            } else if options.keep_empty_rows {
                Some(expected)
            } else {
                open.fill
                    .as_ref()
                    .and_then(|fill| fill.covered(expected, raw.number))
            };
            let raw = match gap {
                Some(number) => {
                    open.held = Some(raw);
                    sheet::RawRow {
                        number,
                        ..sheet::RawRow::default()
                    }
                }
                None => raw,
            };
            open.next = raw.number + 1;
            self.reach(source, &raw)?;
            if raw.number < from || (options.skip_hidden && raw.hidden) {
                continue;
            }
            let mut row = self.row(source, raw, options, &shown, locale)?;
            if let Some(fill) = self.open.as_ref().and_then(|open| open.fill.as_ref()) {
                fill.fill(&mut row);
            }
            row.cells.retain(|cell| {
                selected.is_empty()
                    || selected
                        .iter()
                        .any(|&(first, last)| (first..=last).contains(&cell.column))
            });
            if row.cells.iter().all(|cell| cell.value.is_none()) && !options.keep_empty_rows {
                continue;
            }
            rows.push(row);
        }
        Ok(Batch { rows, done: false })
    }

    /// Moves a fill to the row `raw`, and keeps the top-left value of each range that starts in
    /// it. Does nothing for a read that does not fill.
    fn reach(&mut self, source: &dyn Source, raw: &sheet::RawRow) -> Result<(), Error> {
        let Some(fill) = self.open.as_mut().and_then(|open| open.fill.as_mut()) else {
            return Ok(());
        };
        let starting = fill.reach(raw.number);
        if starting.is_empty() {
            return Ok(());
        }
        if raw.cells.iter().any(|cell| cell.kind == b"s") {
            self.strings(source)?;
        }
        let mut values = Vec::with_capacity(starting.len());
        for (at, column) in starting {
            let value = match raw.cells.iter().find(|cell| cell.column == column) {
                Some(cell) => self.value(cell)?,
                None => None,
            };
            values.push((at, value));
        }
        if let Some(fill) = self.open.as_mut().and_then(|open| open.fill.as_mut()) {
            for (at, value) in values {
                fill.set(at, value);
            }
        }
        Ok(())
    }

    /// The column ranges the read's `text` names: each name a header key when the read has a
    /// header row, and otherwise a column or a range of columns. `text_all` names every column.
    fn shown(
        &mut self,
        source: &dyn Source,
        options: &ReadOptions,
    ) -> Result<Vec<(u64, u64)>, Error> {
        if options.text_all {
            return Ok(vec![(1, u64::MAX)]);
        }
        if options.text.is_empty() {
            return Ok(Vec::new());
        }
        let keys = if options.header_row.is_some() {
            self.header(source, options)?
        } else {
            Vec::new()
        };
        options
            .text
            .iter()
            .map(|name| {
                if let Some(at) = keys.iter().position(|key| key == name) {
                    let column = at as u64 + 1;
                    return Ok((column, column));
                }
                selection(std::slice::from_ref(name))
                    .map(|ranges| ranges[0])
                    .map_err(|_| {
                        Error::Invalid(format!(
                            "The option `text` names `{name}`, which is not a header of the sheet \
                             or a column such as `B` or `A:C`."
                        ))
                    })
            })
            .collect()
    }

    /// `raw` as read cells, every empty cell dropped. A cell in a column of `shown` carries
    /// its displayed text, with the separators and names of `locale`.
    fn row(
        &mut self,
        source: &dyn Source,
        raw: sheet::RawRow,
        options: &ReadOptions,
        shown: &[(u64, u64)],
        locale: Locale,
    ) -> Result<ReadRow, Error> {
        let needs_strings = raw.cells.iter().any(|cell| cell.kind == b"s");
        if needs_strings {
            self.strings(source)?;
        }
        let mut cells = Vec::with_capacity(raw.cells.len());
        for cell in raw.cells {
            let value = self.value(&cell)?;
            let display = match &value {
                Some(value)
                    if shown
                        .iter()
                        .any(|&(first, last)| (first..=last).contains(&cell.column)) =>
                {
                    Some(self.display(&cell, value, locale))
                }
                _ => None,
            };
            let formula = if options.formulas { cell.formula } else { None };
            if value.is_none() && formula.is_none() {
                continue;
            }
            cells.push(ReadCell {
                column: cell.column,
                value,
                formula,
                display,
            });
        }
        Ok(ReadRow {
            number: raw.number,
            cells,
        })
    }

    /// The read value of `cell`, by the read table.
    fn value(&self, cell: &sheet::RawCell) -> Result<Option<Read>, Error> {
        let text = || cell.value.clone().unwrap_or_default();
        Ok(Some(match cell.kind.as_slice() {
            b"s" => {
                let Some(index) = cell
                    .value
                    .as_deref()
                    .and_then(|v| v.trim().parse::<usize>().ok())
                else {
                    return Ok(None);
                };
                let strings = self
                    .strings
                    .as_ref()
                    .ok_or_else(|| broken("no shared strings"))?;
                let text = strings
                    .get(index)
                    .ok_or_else(|| broken("a cell names a shared string the file does not have"))?;
                Read::Text(text.to_owned())
            }
            b"inlineStr" => match &cell.inline {
                Some(text) => Read::Text(xml::unescape(text.clone())),
                None => return Ok(None),
            },
            b"str" => match &cell.value {
                Some(text) => Read::Text(xml::unescape(text.clone())),
                None => return Ok(None),
            },
            b"text" => match &cell.value {
                Some(text) => Read::Text(text.clone()),
                None => return Ok(None),
            },
            b"b" => match cell.value.as_deref().map(str::trim) {
                Some("1" | "true") => Read::Boolean(true),
                Some(_) => Read::Boolean(false),
                None => return Ok(None),
            },
            b"e" => match cell.value.as_deref().and_then(|v| CellError::of(v.trim())) {
                Some(error) => Read::Error(error),
                None => return Ok(None),
            },
            b"d" => Read::Text(text()),
            _ => {
                let Some(raw) = cell.value.as_deref().map(str::trim) else {
                    return Ok(None);
                };
                let number: f64 = raw
                    .parse()
                    .map_err(|_| broken(&format!("a number cell holds `{raw}`")))?;
                let kind = self
                    .styles
                    .get(cell.style as usize)
                    .map_or(Kind::Plain, |style| style.kind);
                timed(number, kind, self.date1904).unwrap_or_else(|| plain(number))
            }
        }))
    }

    /// The text `cell`, read as `value`, displays under its style's number format. A number is
    /// rendered from the serial the file stores, so a date shows as its format writes it.
    fn display(&self, cell: &sheet::RawCell, value: &Read, locale: Locale) -> String {
        let code = self
            .styles
            .get(cell.style as usize)
            .map_or("General", |style| &style.code);
        let options = Options {
            date1904: self.date1904,
            locale,
        };
        let shown = match value {
            Read::Text(text) => Value::Text(text),
            Read::Boolean(flag) => Value::Boolean(*flag),
            Read::Error(error) => Value::Error(error.text()),
            _ => match cell.value.as_deref().map(str::trim).map(str::parse::<f64>) {
                Some(Ok(number)) => Value::Number(number),
                _ => return String::new(),
            },
        };
        crate::format::render(code, shown, &options)
    }
}

/// A number as `integer` when it is whole and within ±2^53, and `number` otherwise.
fn plain(number: f64) -> Read {
    const EXACT: f64 = 9_007_199_254_740_992.0;
    if number.fract() == 0.0 && number.abs() <= EXACT {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "whole and within ±2^53, so exact"
        )]
        Read::Integer(number as i64)
    } else {
        Read::Number(number)
    }
}

const DAY_MS: i64 = 86_400_000;

/// The time value a serial `number` is under a format of `kind`, or `None` for a plain number or a
/// serial no date can have.
fn timed(number: f64, kind: Kind, date1904: bool) -> Option<Read> {
    if kind == Kind::Plain || !number.is_finite() {
        return None;
    }
    let ms = (number * DAY_MS as f64).round();
    if ms.abs() > 3e15 {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "bounded by the check above"
    )]
    let ms = ms as i64;
    if kind == Kind::Duration {
        return Some(Read::Duration(ms * 1_000_000));
    }
    if ms < 0 {
        return None;
    }
    let (days, of_day) = (ms / DAY_MS, ms % DAY_MS);
    let time = TimeOfDay {
        hour: of_day / 3_600_000,
        minute: of_day / 60_000 % 60,
        second: of_day / 1000 % 60,
        nanos: of_day % 1000 * 1_000_000,
    };
    let days = if date1904 {
        days + 1462
    } else if days < 61 {
        days + 1
    } else {
        days
    };
    let day = civil(days + days_from_civil(1899, 12, 30));
    if day.year > 9999 {
        return None;
    }
    Some(match kind {
        Kind::Date => Read::Date(day),
        Kind::Time => Read::Time(time),
        _ => Read::Clock(Clock { day, clock: time }),
    })
}

pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// The civil date `days` after 1970-01-01.
pub(crate) fn civil(days: i64) -> Date {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    Date {
        year: yoe + era * 400 + i64::from(month <= 2),
        month,
        day,
    }
}

/// The most rows and columns a sheet has.
const MOST_ROWS: u64 = 1_048_576;
const MOST_COLUMNS: u64 = 16_384;

/// The column letters of `column`, from 1.
fn letters(column: u64) -> String {
    let mut out = String::new();
    crate::cells::column_letters(&mut out, u32::try_from(column - 1).unwrap_or(u32::MAX));
    out
}

/// The text a header cell gives its key.
fn header_text(value: &Read) -> String {
    match value {
        Read::Text(text) => text.trim().to_owned(),
        Read::Integer(n) => n.to_string(),
        Read::Number(x) => x.to_string(),
        Read::Boolean(b) => (if *b { "TRUE" } else { "FALSE" }).to_owned(),
        Read::Date(d) => format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
        Read::Time(t) => format!("{:02}:{:02}:{:02}", t.hour, t.minute, t.second),
        Read::Clock(c) => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            c.day.year, c.day.month, c.day.day, c.clock.hour, c.clock.minute
        ),
        Read::Duration(nanos) => format!("{}s", nanos / 1_000_000_000),
        Read::Error(error) => error.text().to_owned(),
    }
}

/// The column ranges `columns` names, such as `A:C` and `F`.
fn selection(columns: &[String]) -> Result<Vec<(u64, u64)>, Error> {
    columns
        .iter()
        .map(|range| {
            let bad = || {
                Error::Invalid(format!(
                    "`{range}` is not a column or a range of columns such as `A:C`."
                ))
            };
            let (first, last) = range.split_once(':').unwrap_or((range, range));
            let first = sheet::column_of(first.trim().as_bytes()).ok_or_else(bad)?;
            let last = sheet::column_of(last.trim().as_bytes()).ok_or_else(bad)?;
            Ok((first.min(last), first.max(last)))
        })
        .collect()
}
