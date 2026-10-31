//! The extras a read gives on request (`rule:core-classes/spreadsheet-reads-as-a-stream`): a
//! sheet's hyperlinks, notes, tables and images, and the workbook's defined names and document
//! properties. `Reader::merged` and `Reader::macros` are the other two, in the parent module.
//!
//! **Where each one is.** A sheet's hyperlinks are `<hyperlink>` elements after its rows, so
//! [`Reader::links`] reads the sheet's part to its end a block at a time, as `merged` does. Notes,
//! tables and images are parts of their own that the sheet's relationships name: a comments part,
//! one part per table, and a drawing whose relationships name each picture. The defined names are
//! in the workbook part, and the properties in `docProps/core.xml` and `docProps/app.xml`. Each of
//! those is a small part and is read whole. An ods's extras are read with its cells
//! (`super::ods`), and its properties are `meta.xml`'s.
//!
//! **Nothing a file names is followed** (`rule:core-classes/spreadsheet-has-no-io`). An external
//! hyperlink is its target's text with `external` set, a linked picture is its target's text in
//! `link` with no bytes, and a defined name over another workbook is its reference's text,
//! `[1]Shop!$A$1`. A data connection is never read, and an xlsb's external link part is read for
//! the other workbook's sheet names alone.
//!
//! **xlsb.** The sheet part and the parts it names are records ([`super::binary`]): a hyperlink is
//! a `BrtHLink` in the sheet part, a note is in a comments part of records, and a table is the
//! `BrtBeginList` of its part. A defined name is a `BrtName` of the workbook part, its formula
//! decoded by [`super::tokens`]. A name the decoder cannot read is listed with an empty
//! reference. The relationships and the drawing are XML, as in
//! an xlsx.
//!
//! **Formats.** Every extra is read from an xlsx, xlsm, xlsb, xls or ods, and gives the same
//! shape for each. An xls's extras are read with its cells (`super::xls_extras`), and BIFF8 has no
//! table part, so an xls has no tables.
//!
//! What it spends, per call: the part it reads whole, or one block of a sheet's part and the
//! inflater's window, and the list it returns. An image's bytes are held once, in the list.

use super::{Reader, Source, binary, civil, days_from_civil, letters, relationships, sheet, xml};
use crate::{Clock, Error, Properties, TimeOfDay};
use xml::Token;

/// One hyperlink: its cell or range, and its target. An external target is a URL or a file the
/// link names, and an internal one a place in the workbook such as `'Sheet 2'!A1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub anchor: String,
    pub target: String,
    pub text: Option<String>,
    pub tooltip: Option<String>,
    pub external: bool,
}

/// One note: its cell, its text and its author. An empty name in the authors list, which a note
/// written with no author is given, reads as no author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub noted: String,
    pub note: String,
    pub author: Option<String>,
}

/// One defined name: its reference as written, and the sheet it is local to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub name: String,
    pub reference: String,
    pub sheet: Option<String>,
}

/// One Excel table: its range, its name and its sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub table: String,
    pub name: String,
    pub sheet: String,
}

/// One picture: its top-left cell, its bytes and their format. A linked picture has no bytes and
/// its target in `link`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub at: String,
    pub image: Vec<u8>,
    pub format: String,
    pub link: Option<String>,
}

/// The decoded text of every text token in `raw`.
fn text(raw: &[u8], out: &mut String) -> Result<(), Error> {
    xml::decode(raw, out)
}

/// The format of a picture: its bytes' signature, and otherwise its part's extension.
fn format(bytes: &[u8], target: &str) -> String {
    if bytes.starts_with(b"\x89PNG") {
        return "png".to_owned();
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "jpeg".to_owned();
    }
    if bytes.starts_with(b"GIF8") {
        return "gif".to_owned();
    }
    target
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

/// A W3CDTF time, `2024-03-05T14:30:00Z` or with an offset, as its clock time in UTC. A date
/// alone is its midnight. A text in no such form is `None`, because a property is metadata and
/// not a cell.
fn instant(text: &str) -> Option<Clock> {
    let text = text.trim();
    let (date, rest) = text.split_once('T').unwrap_or((text, ""));
    let mut parts = date.splitn(3, '-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (time, offset) = if let Some(time) = rest.strip_suffix('Z') {
        (time, 0)
    } else if let Some(at) = rest.rfind(['+', '-']) {
        let (hours, minutes) = rest[at + 1..].split_once(':')?;
        let minutes = hours.parse::<i64>().ok()? * 60 + minutes.parse::<i64>().ok()?;
        let sign = if rest.as_bytes()[at] == b'-' { -1 } else { 1 };
        (&rest[..at], sign * minutes * 60)
    } else {
        (rest, 0)
    };
    let mut seconds = 0;
    let mut nanos = 0;
    if !time.is_empty() {
        let mut fields = time.splitn(3, ':');
        let hour: i64 = fields.next()?.parse().ok()?;
        let minute: i64 = fields.next()?.parse().ok()?;
        let second = fields.next().unwrap_or("0");
        let (whole, fraction) = second.split_once('.').unwrap_or((second, ""));
        let second: i64 = whole.parse().ok()?;
        seconds = hour * 3600 + minute * 60 + second;
        let digits: String = fraction.chars().take(9).collect();
        if !digits.is_empty() {
            nanos = format!("{digits:0<9}").parse().ok()?;
        }
    }
    let total = days_from_civil(year, month, day) * 86_400 + seconds - offset;
    let of_day = total.rem_euclid(86_400);
    Some(Clock {
        day: civil(total.div_euclid(86_400)),
        clock: TimeOfDay {
            hour: of_day / 3600,
            minute: of_day % 3600 / 60,
            second: of_day % 60,
            nanos,
        },
    })
}

impl Reader {
    /// The workbook part of an xlsx, xlsm or xlsb. Every other format's extras are read with its
    /// cells, so a reader with no workbook part is not a package.
    fn package_workbook(&self) -> Result<&str, Error> {
        self.workbook.as_deref().ok_or_else(|| {
            Error::Parse("The file is not a valid workbook: it has no workbook part.".to_owned())
        })
    }

    /// The extras of an xls, after a check that the sheet `name` exists, when the file is one.
    fn xls_of(&self, name: &str) -> Result<Option<&super::xls_extras::Extras>, Error> {
        match self.xls.as_deref() {
            Some(extras) => {
                self.listed(name)?;
                Ok(Some(extras))
            }
            None => Ok(None),
        }
    }

    /// The extras of an ods, after a check that the sheet `name` exists, when the file is one.
    fn ods_of(&self, name: &str) -> Result<Option<&super::ods::Extras>, Error> {
        match self.ods.as_deref() {
            Some(extras) => {
                self.listed(name)?;
                Ok(Some(extras))
            }
            None => Ok(None),
        }
    }

    /// The relationships of the sheet `name` of an xlsx, xlsm or xlsb. A sheet with no part gives
    /// `None`.
    fn sheet_rels(
        &self,
        source: &dyn Source,
        name: &str,
    ) -> Result<Option<Vec<(String, String, String, bool)>>, Error> {
        self.package_workbook()?;
        match &self.listed(name)?.part {
            Some(part) => relationships(source, &self.directory, part).map(Some),
            None => Ok(None),
        }
    }

    /// A part the package holds, read whole, or `None` when it is missing.
    fn whole_part(&self, source: &dyn Source, part: &str) -> Result<Option<Vec<u8>>, Error> {
        self.directory
            .find(part)
            .map(|entry| self.directory.whole(source, entry))
            .transpose()
    }

    /// The hyperlinks of the sheet `name`, in the order the file lists them.
    pub fn links(&self, source: &dyn Source, name: &str) -> Result<Vec<Link>, Error> {
        if let Some(extras) = self.ods_of(name)? {
            return Ok(of_sheet(&extras.links, name));
        }
        if let Some(extras) = self.xls_of(name)? {
            return Ok(of_sheet(&extras.links, name));
        }
        let Some(rels) = self.sheet_rels(source, name)? else {
            return Ok(Vec::new());
        };
        let entry = self.part(self.listed(name)?)?;
        let mut links = Vec::new();
        let target = |rel: Option<&str>, location: Option<String>| {
            let rel = rel.and_then(|id| rels.iter().find(|(rid, _, _, _)| rid == id));
            match (rel, location) {
                (Some((_, _, target, external)), Some(location)) => {
                    Some((format!("{target}#{location}"), *external))
                }
                (Some((_, _, target, external)), None) => Some((target.clone(), *external)),
                (None, Some(location)) => Some((location, false)),
                (None, None) => None,
            }
        };
        if self.binary {
            binary::scan(source, entry, |kind, body| {
                if kind != binary::HYPERLINK {
                    return Ok(());
                }
                let link = binary::hyperlink(body)?;
                if let Some((target, external)) = target(link.rel.as_deref(), link.location) {
                    links.push(Link {
                        anchor: link.anchor,
                        target,
                        text: link.display,
                        tooltip: link.tooltip,
                        external,
                    });
                }
                Ok(())
            })?;
            return Ok(links);
        }
        sheet::scan(source, entry, |tag, attrs| {
            if tag != b"hyperlink" {
                return Ok(());
            }
            let anchor = xml::attr_text(attrs, b"ref")?.unwrap_or_default();
            let location = xml::attr_text(attrs, b"location")?;
            let rel = xml::attr_text(attrs, b"id")?;
            let Some((target, external)) = target(rel.as_deref(), location) else {
                return Ok(());
            };
            links.push(Link {
                anchor,
                target,
                text: xml::attr_text(attrs, b"display")?,
                tooltip: xml::attr_text(attrs, b"tooltip")?,
                external,
            });
            Ok(())
        })?;
        Ok(links)
    }

    /// The notes of the sheet `name`, in the order its comments part lists them.
    pub fn notes(&self, source: &dyn Source, name: &str) -> Result<Vec<Note>, Error> {
        if let Some(extras) = self.ods_of(name)? {
            return Ok(of_sheet(&extras.notes, name));
        }
        if let Some(extras) = self.xls_of(name)? {
            return Ok(of_sheet(&extras.notes, name));
        }
        let Some(rels) = self.sheet_rels(source, name)? else {
            return Ok(Vec::new());
        };
        let Some(bytes) = rels
            .iter()
            .find(|(_, kind, _, external)| kind == "comments" && !external)
            .map(|(_, _, target, _)| self.whole_part(source, target))
            .transpose()?
            .flatten()
        else {
            return Ok(Vec::new());
        };
        if self.binary {
            return Ok(binary::comments(&bytes)?
                .into_iter()
                .map(|(noted, note, author)| Note {
                    noted,
                    note,
                    author,
                })
                .collect());
        }
        let mut authors = Vec::new();
        let mut notes = Vec::new();
        let mut author: Option<String> = None;
        let mut note: Option<(String, Option<usize>, String)> = None;
        let mut in_text = false;
        let mut at = 0;
        while let Some((token, size)) = xml::next(&bytes[at..])? {
            at += size;
            match token {
                Token::Open { name, attrs, empty } => match xml::local(name) {
                    b"author" if !empty => author = Some(String::new()),
                    b"author" => authors.push(String::new()),
                    b"comment" => {
                        let noted = xml::attr_text(attrs, b"ref")?.unwrap_or_default();
                        let by = xml::attr(attrs, b"authorId")
                            .and_then(|id| std::str::from_utf8(id).ok()?.parse().ok());
                        note = Some((noted, by, String::new()));
                    }
                    b"t" => in_text = !empty,
                    _ => {}
                },
                Token::Close(name) => match xml::local(name) {
                    b"author" => authors.extend(author.take()),
                    b"t" => in_text = false,
                    b"comment" => {
                        if let Some((noted, by, text)) = note.take() {
                            notes.push(Note {
                                noted,
                                note: text,
                                author: by
                                    .and_then(|by| authors.get(by).cloned())
                                    .filter(|author| !author.is_empty()),
                            });
                        }
                    }
                    _ => {}
                },
                Token::Text(raw) => {
                    if let Some(author) = author.as_mut() {
                        text(raw, author)?;
                    } else if in_text && let Some((_, _, out)) = note.as_mut() {
                        text(raw, out)?;
                    }
                }
                Token::Raw(raw) => {
                    if in_text && let Some((_, _, out)) = note.as_mut() {
                        out.push_str(&String::from_utf8_lossy(raw));
                    }
                }
                Token::Skip => {}
            }
        }
        Ok(notes)
    }

    /// The workbook's defined names, in the order the file lists them. A name local to a sheet
    /// names that sheet.
    pub fn names(&self, source: &dyn Source) -> Result<Vec<Name>, Error> {
        if let Some(extras) = &self.ods {
            return Ok(extras.names.clone());
        }
        if let Some(extras) = &self.xls {
            return Ok(extras.names.clone());
        }
        let part = self.package_workbook()?;
        let Some(bytes) = self.whole_part(source, part)? else {
            return Ok(Vec::new());
        };
        if self.binary {
            return Ok(binary::names(&bytes, &self.book)?
                .into_iter()
                .map(|(name, sheet, reference)| Name {
                    name,
                    reference: reference.unwrap_or_default(),
                    sheet: sheet
                        .and_then(|id| self.sheets.get(id))
                        .map(|listed| listed.name.clone()),
                })
                .collect());
        }
        let mut names = Vec::new();
        let mut open: Option<Name> = None;
        let mut at = 0;
        while let Some((token, size)) = xml::next(&bytes[at..])? {
            at += size;
            match token {
                Token::Open { name, attrs, empty } if xml::local(name) == b"definedName" => {
                    let sheet = xml::attr(attrs, b"localSheetId")
                        .and_then(|id| std::str::from_utf8(id).ok()?.parse::<usize>().ok())
                        .and_then(|id| self.sheets.get(id))
                        .map(|listed| listed.name.clone());
                    let name = Name {
                        name: xml::attr_text(attrs, b"name")?.unwrap_or_default(),
                        reference: String::new(),
                        sheet,
                    };
                    if empty {
                        names.push(name);
                    } else {
                        open = Some(name);
                    }
                }
                Token::Text(raw) => {
                    if let Some(name) = open.as_mut() {
                        text(raw, &mut name.reference)?;
                    }
                }
                Token::Close(name) if xml::local(name) == b"definedName" => {
                    names.extend(open.take());
                }
                _ => {}
            }
        }
        Ok(names)
    }

    /// The Excel tables of the sheet `name`, in the order its relationships list them.
    pub fn tables(&self, source: &dyn Source, name: &str) -> Result<Vec<Table>, Error> {
        Ok(self
            .table_rows(source, name)?
            .into_iter()
            .map(|(table, _, _)| table)
            .collect())
    }

    /// [`Self::tables`], each with whether it has a header row and a totals row, which a
    /// structured reference reads.
    pub fn table_rows(
        &self,
        source: &dyn Source,
        name: &str,
    ) -> Result<Vec<(Table, bool, bool)>, Error> {
        if let Some(extras) = self.ods_of(name)? {
            return Ok(extras
                .tables
                .iter()
                .filter(|table| table.sheet == name)
                .map(|table| (table.clone(), true, false))
                .collect());
        }
        // BIFF8 has no table part, so an xls has no tables.
        if self.xls_of(name)?.is_some() {
            return Ok(Vec::new());
        }
        let Some(rels) = self.sheet_rels(source, name)? else {
            return Ok(Vec::new());
        };
        let mut tables = Vec::new();
        for (_, kind, target, external) in &rels {
            if kind != "table" || *external {
                continue;
            }
            let Some(bytes) = self.whole_part(source, target)? else {
                continue;
            };
            if self.binary {
                if let Some((table, shown, header, totals)) = binary::table(&bytes)? {
                    let table = Table {
                        table,
                        name: shown,
                        sheet: name.to_owned(),
                    };
                    tables.push((table, header, totals));
                }
                continue;
            }
            let mut at = 0;
            while let Some((token, size)) = xml::next(&bytes[at..])? {
                at += size;
                if let Token::Open {
                    name: tag, attrs, ..
                } = token
                    && xml::local(tag) == b"table"
                {
                    let shown = xml::attr_text(attrs, b"displayName")?;
                    let count = |attr: &[u8], default: &str| -> Result<bool, Error> {
                        Ok(xml::attr_text(attrs, attr)?.as_deref().unwrap_or(default) != "0")
                    };
                    let table = Table {
                        table: xml::attr_text(attrs, b"ref")?.unwrap_or_default(),
                        name: match shown {
                            Some(shown) => shown,
                            None => xml::attr_text(attrs, b"name")?.unwrap_or_default(),
                        },
                        sheet: name.to_owned(),
                    };
                    let header = count(b"headerRowCount", "1")?;
                    tables.push((table, header, count(b"totalsRowCount", "0")?));
                    break;
                }
            }
        }
        Ok(tables)
    }

    /// The pictures of the sheet `name`, each at the top-left cell of its anchor, in the order
    /// its drawing lists them. A picture anchored to no cell is not listed.
    pub fn images(&self, source: &dyn Source, name: &str) -> Result<Vec<Image>, Error> {
        if let Some(extras) = self.ods_of(name)? {
            let mut images = Vec::new();
            for (sheet, at, href) in &extras.images {
                if sheet != name {
                    continue;
                }
                // A picture outside the package is its target's text, and is never read.
                if href.contains(':') || href.starts_with('/') || href.starts_with("../") {
                    images.push(Image {
                        at: at.clone(),
                        format: format(&[], href),
                        image: Vec::new(),
                        link: Some(href.clone()),
                    });
                    continue;
                }
                let part = href.strip_prefix("./").unwrap_or(href);
                let image = self.whole_part(source, part)?.unwrap_or_default();
                images.push(Image {
                    at: at.clone(),
                    format: format(&image, part),
                    image,
                    link: None,
                });
            }
            return Ok(images);
        }
        if let Some(extras) = self.xls_of(name)? {
            return Ok(of_sheet(&extras.images, name));
        }
        let Some(rels) = self.sheet_rels(source, name)? else {
            return Ok(Vec::new());
        };
        let mut images = Vec::new();
        for (_, kind, drawing, external) in &rels {
            if kind != "drawing" || *external {
                continue;
            }
            let Some(bytes) = self.whole_part(source, drawing)? else {
                continue;
            };
            let pictures = relationships(source, &self.directory, drawing)?;
            let mut from: Option<(u64, u64)> = None;
            let mut in_from = false;
            let mut field: Option<bool> = None;
            let mut number = String::new();
            let mut at = 0;
            while let Some((token, size)) = xml::next(&bytes[at..])? {
                at += size;
                match token {
                    Token::Open {
                        name: tag,
                        attrs,
                        empty,
                    } => match xml::local(tag) {
                        b"twoCellAnchor" | b"oneCellAnchor" | b"absoluteAnchor" => from = None,
                        b"from" => {
                            in_from = !empty;
                            from = Some((0, 0));
                        }
                        b"col" if in_from => {
                            field = Some(true);
                            number.clear();
                        }
                        b"row" if in_from => {
                            field = Some(false);
                            number.clear();
                        }
                        b"blip" => {
                            let Some((column, row)) = from else {
                                continue;
                            };
                            let at = format!("{}{}", letters(column + 1), row + 1);
                            let embed = xml::attr_text(attrs, b"embed")?;
                            let link = xml::attr_text(attrs, b"link")?;
                            let target = |id: &str| {
                                pictures
                                    .iter()
                                    .find(|(rid, _, _, _)| rid == id)
                                    .map(|(_, _, target, external)| (target.clone(), *external))
                            };
                            if let Some((target, false)) = embed.as_deref().and_then(target) {
                                let image = self.whole_part(source, &target)?.unwrap_or_default();
                                images.push(Image {
                                    at,
                                    format: format(&image, &target),
                                    image,
                                    link: None,
                                });
                            } else if let Some((target, _)) = link.as_deref().and_then(target) {
                                images.push(Image {
                                    at,
                                    format: format(&[], &target),
                                    image: Vec::new(),
                                    link: Some(target),
                                });
                            }
                        }
                        _ => {}
                    },
                    Token::Text(raw) if field.is_some() => text(raw, &mut number)?,
                    Token::Close(tag) => match xml::local(tag) {
                        b"from" => in_from = false,
                        b"col" | b"row" => {
                            if let (Some(is_column), Some((column, row))) = (field, from.as_mut()) {
                                let value = number.trim().parse().unwrap_or(0);
                                if is_column {
                                    *column = value;
                                } else {
                                    *row = value;
                                }
                            }
                            field = None;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        Ok(images)
    }

    /// The document properties of a zip package: the core part's title, subject, author,
    /// keywords and creation time, and the app part's company. An ods's are `meta.xml`'s, and an
    /// xls's its summary streams', read when it opened.
    pub fn properties(&self, source: &dyn Source) -> Result<Properties, Error> {
        if self.ods.is_some() {
            return self.meta_properties(source);
        }
        if let Some(extras) = &self.xls {
            return Ok(extras.properties.clone());
        }
        self.package_workbook()?;
        let rels = relationships(source, &self.directory, "")?;
        let part = |kind: &str, usual: &str| {
            rels.iter()
                .find(|(_, found, _, external)| found == kind && !external)
                .map_or_else(|| usual.to_owned(), |(_, _, target, _)| target.clone())
        };
        let mut properties = Properties::default();
        for part in [
            part("core-properties", "docProps/core.xml"),
            part("extended-properties", "docProps/app.xml"),
        ] {
            let Some(bytes) = self.whole_part(source, &part)? else {
                continue;
            };
            let mut open: Option<(&'static str, String)> = None;
            let mut at = 0;
            while let Some((token, size)) = xml::next(&bytes[at..])? {
                at += size;
                match token {
                    Token::Open {
                        name, empty: false, ..
                    } => {
                        let field = match xml::local(name) {
                            b"title" => "title",
                            b"subject" => "subject",
                            b"creator" => "author",
                            b"keywords" => "keywords",
                            b"created" => "created",
                            b"Company" => "company",
                            _ => continue,
                        };
                        open = Some((field, String::new()));
                    }
                    Token::Text(raw) => {
                        if let Some((_, out)) = open.as_mut() {
                            text(raw, out)?;
                        }
                    }
                    Token::Close(_) => {
                        let Some((field, value)) = open.take() else {
                            continue;
                        };
                        match field {
                            "title" => properties.title = Some(value),
                            "subject" => properties.subject = Some(value),
                            "author" => properties.author = Some(value),
                            "keywords" => properties.keywords = Some(value),
                            "company" => properties.company = Some(value),
                            _ => properties.created = instant(&value),
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(properties)
    }

    /// The document properties of an ods's `meta.xml`. The author is the initial creator, or
    /// the last one when the file names no initial creator. The keywords are joined with `, `,
    /// and the company is a user-defined field named `Company`.
    fn meta_properties(&self, source: &dyn Source) -> Result<Properties, Error> {
        let mut properties = Properties::default();
        let Some(bytes) = self.whole_part(source, "meta.xml")? else {
            return Ok(properties);
        };
        let (mut first, mut last) = (None, None);
        let mut keywords: Vec<String> = Vec::new();
        let mut open: Option<(&'static str, String)> = None;
        let mut at = 0;
        while let Some((token, size)) = xml::next(&bytes[at..])? {
            at += size;
            match token {
                Token::Open {
                    name,
                    attrs,
                    empty: false,
                } => {
                    let field = match xml::local(name) {
                        b"title" => "title",
                        b"subject" => "subject",
                        b"initial-creator" => "first",
                        b"creator" => "last",
                        b"keyword" => "keyword",
                        b"creation-date" => "created",
                        b"user-defined" if xml::attr(attrs, b"name") == Some(b"Company") => {
                            "company"
                        }
                        _ => continue,
                    };
                    open = Some((field, String::new()));
                }
                Token::Text(raw) => {
                    if let Some((_, out)) = open.as_mut() {
                        text(raw, out)?;
                    }
                }
                Token::Close(_) => {
                    let Some((field, value)) = open.take() else {
                        continue;
                    };
                    match field {
                        "title" => properties.title = Some(value),
                        "subject" => properties.subject = Some(value),
                        "first" => first = Some(value),
                        "last" => last = Some(value),
                        "keyword" => keywords.push(value),
                        "company" => properties.company = Some(value),
                        _ => properties.created = instant(&value),
                    }
                }
                _ => {}
            }
        }
        properties.author = first.or(last);
        properties.keywords = (!keywords.is_empty()).then(|| keywords.join(", "));
        Ok(properties)
    }
}

/// The entries of `list` on the sheet `name`, in the file's order.
fn of_sheet<T: Clone>(list: &[(String, T)], name: &str) -> Vec<T> {
    list.iter()
        .filter(|(sheet, _)| sheet == name)
        .map(|(_, entry)| entry.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(text: &str) -> Option<(i64, i64, i64, i64, i64, i64)> {
        instant(text).map(|clock| {
            (
                clock.day.year,
                clock.day.month,
                clock.day.day,
                clock.clock.hour,
                clock.clock.minute,
                clock.clock.second,
            )
        })
    }

    #[test]
    fn a_creation_time_reads_in_utc() {
        assert_eq!(clock("2024-03-05T14:30:09Z"), Some((2024, 3, 5, 14, 30, 9)));
        assert_eq!(
            clock("2024-03-05T01:30:00+02:00"),
            Some((2024, 3, 4, 23, 30, 0))
        );
        assert_eq!(
            clock("2024-12-31T23:00:00-01:30"),
            Some((2025, 1, 1, 0, 30, 0))
        );
        assert_eq!(clock("2024-03-05"), Some((2024, 3, 5, 0, 0, 0)));
        assert_eq!(clock("yesterday"), None);
        assert_eq!(
            instant("2024-03-05T14:30:09.25Z").map(|clock| clock.clock.nanos),
            Some(250_000_000)
        );
    }
}
