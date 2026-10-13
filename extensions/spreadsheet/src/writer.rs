//! The workbook writer: sheets one after another, rows in batches, and the parts `finish` adds
//! (`rule:core-classes/spreadsheet-writes-as-a-stream`).
//!
//! [`Writer::sheet`] ends the open sheet and starts the next one's zip entry with what xlsx writes
//! before the rows: the tab color, the sheet view with its frozen pane, gridlines, direction and
//! zoom, and the columns. [`Writer::rows`] writes a batch into `<sheetData>`. A batch is built as
//! text first and goes into the compressor only once every cell of it has been written, so a batch
//! that fails leaves the sheet as it was and the guest keeps no row of it. [`Writer::finish`] ends
//! the last sheet with its autofilter and writes `styles.xml`, `workbook.xml` with the sheets in
//! tab order, the relationships, the document properties, the content types and the zip
//! directory. Each call returns the archive's bytes produced so far, so the writer holds nothing
//! it has already given out.
//!
//! The tab order is the sheets with a `position` by that position, then the others in the order
//! they were written. A sheet's part is named by the order it was written in, whatever its tab.
//!
//! **The document dates are fixed.** `created` and `modified` are 1980-01-01T00:00:00Z, the
//! archive's own time, unless the properties set `created`, so equal input writes equal bytes
//! (`rule:core-classes/spreadsheet-output-is-inert`).
//!
//! A call that fails in the archive (a part past 4 GB) stops the writer, and every later call
//! returns an error. A call that fails on its input (a bad sheet name, a NaN) changes nothing,
//! and the program may go on.
//!
//! **Not done yet.** Auto width writes no width, the sheet-end parts (`writer-part`) return an
//! error, and a sheet with a header row gets no default style: Stages 4 and 5 of goal
//! `spreadsheet`.

use std::fmt::Write as _;

use crate::cells::{self, Layers};
use crate::styles::{Styles, argb};
use crate::zip::Archive;
use crate::{
    Clock, Error, Finished, Row, SheetOptions, SheetRows, Style, Visibility, WorkbookOptions, xml,
};

/// The declaration every part begins with.
pub const XML_HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

/// Excel's grid.
const MAX_ROWS: u32 = 1_048_576;
const MAX_COLUMNS: usize = 16_384;

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// A sheet already started.
struct Sheet {
    name: String,
    position: Option<u64>,
    visibility: Visibility,
    rows: u64,
}

/// The sheet whose rows are being written.
struct OpenSheet {
    /// The sheet's index in [`Writer::sheets`].
    index: usize,
    columns: Vec<Option<u64>>,
    autofilter: Option<String>,
    /// The number the next row gets, from 1.
    next: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Writing,
    Finished,
    Stopped,
}

/// A workbook being written.
pub struct Writer {
    options: WorkbookOptions,
    archive: Archive,
    styles: Styles,
    sheets: Vec<Sheet>,
    open: Option<OpenSheet>,
    names: Vec<(String, String)>,
    formulas: bool,
    state: State,
}

/// The error a writer dropped before [`Writer::finish`] logs.
pub fn unfinished_message(target: &str) -> String {
    if target.is_empty() {
        "A workbook was never finished. Call `finish()` after the last row.".to_owned()
    } else {
        format!("The workbook `{target}` was never finished. Call `finish()` after the last row.")
    }
}

fn check_sheet_name(name: &str, sheets: &[Sheet]) -> Result<(), Error> {
    let bad = |rule: &str| Err(Error::Invalid(format!("The sheet name `{name}` {rule}.")));
    if name.is_empty() {
        return Err(Error::Invalid("A sheet name cannot be empty.".to_owned()));
    }
    if name.chars().count() > 31 {
        return bad("is too long. A sheet name has at most 31 characters");
    }
    if let Some(c) = name.chars().find(|c| ":\\/?*[]".contains(*c)) {
        return bad(&format!(
            "has the character `{c}`. A sheet name cannot have `: \\ / ? * [ ]`"
        ));
    }
    if name.starts_with('\'') || name.ends_with('\'') {
        return bad("starts or ends with `'`. That is not allowed");
    }
    let lower = name.to_lowercase();
    if sheets
        .iter()
        .any(|sheet| sheet.name.to_lowercase() == lower)
    {
        return bad("is used twice. Each sheet needs its own name");
    }
    Ok(())
}

/// Whether `text` is a cell or a range of cells such as `A1:D10`, with `$` allowed.
fn is_range(text: &str) -> bool {
    let cell = |part: &str| {
        let part = part.strip_prefix('$').unwrap_or(part);
        let letters = part.bytes().take_while(u8::is_ascii_uppercase).count();
        let rest = &part[letters..];
        let rest = rest.strip_prefix('$').unwrap_or(rest);
        (1..=3).contains(&letters)
            && !rest.is_empty()
            && !rest.starts_with('0')
            && rest.bytes().all(|b| b.is_ascii_digit())
    };
    match text.split_once(':') {
        Some((from, to)) => cell(from) && cell(to),
        None => cell(text),
    }
}

fn clock_text(clock: &Clock) -> Result<String, Error> {
    if !cells::valid_date(&clock.day) || !cells::valid_time(&clock.clock) {
        return Err(Error::Invalid(
            "The workbook's `created` date is not a real date and time.".to_owned(),
        ));
    }
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        clock.day.year,
        clock.day.month,
        clock.day.day,
        clock.clock.hour,
        clock.clock.minute,
        clock.clock.second
    ))
}

impl Writer {
    /// A workbook with nothing written yet.
    pub fn open(options: WorkbookOptions) -> Result<Self, Error> {
        if let Some(created) = &options.properties.created {
            clock_text(created)?;
        }
        Ok(Self {
            options,
            archive: Archive::new(),
            styles: Styles::default(),
            sheets: Vec::new(),
            open: None,
            names: Vec::new(),
            formulas: false,
            state: State::Writing,
        })
    }

    /// Where the bytes go, as the program named it.
    pub fn target(&self) -> &str {
        &self.options.target
    }

    /// Whether [`Writer::finish`] has written the whole workbook.
    pub fn is_finished(&self) -> bool {
        self.state == State::Finished
    }

    fn writing(&self) -> Result<(), Error> {
        match self.state {
            State::Writing => Ok(()),
            State::Finished => Err(Error::Invalid(
                "This workbook is finished. Start a new workbook to write more.".to_owned(),
            )),
            State::Stopped => Err(Error::Invalid(
                "This workbook stopped at an earlier error. Start a new workbook.".to_owned(),
            )),
        }
    }

    /// Runs an archive step, and stops the writer when it fails.
    fn archived<T>(
        &mut self,
        step: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let out = step(self);
        if out.is_err() {
            self.state = State::Stopped;
        }
        out
    }

    /// The id of a style layer, the same id for an equal layer.
    pub fn style(&mut self, style: Style) -> Result<u64, Error> {
        self.writing()?;
        self.styles.layer(style)
    }

    /// A defined name over a reference.
    pub fn define(&mut self, name: &str, reference: &str) -> Result<(), Error> {
        self.writing()?;
        let mut chars = name.chars();
        let first_ok = chars
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '\\');
        if !first_ok || !chars.all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return Err(Error::Invalid(format!(
                "`{name}` is not a name Excel allows. A name starts with a letter or `_`, and has only letters, digits, `_` and `.`."
            )));
        }
        if is_range(&name.to_ascii_uppercase()) {
            return Err(Error::Invalid(format!(
                "`{name}` is a cell reference, so it cannot be a name."
            )));
        }
        if reference.is_empty() {
            return Err(Error::Invalid(format!(
                "The name `{name}` needs a reference, such as `Sheet1!$A$1:$B$10`."
            )));
        }
        let lower = name.to_lowercase();
        if self.names.iter().any(|(n, _)| n.to_lowercase() == lower) {
            return Err(Error::Invalid(format!(
                "The name `{name}` is defined twice."
            )));
        }
        self.names.push((name.to_owned(), reference.to_owned()));
        Ok(())
    }

    /// Ends the open sheet, if any, starts the next one and writes its first rows. Returns the
    /// bytes ready to leave.
    pub fn sheet(&mut self, options: SheetOptions, first: &[Row]) -> Result<Vec<u8>, Error> {
        self.writing()?;
        check_sheet_name(&options.name, &self.sheets)?;
        if options.columns.len() > MAX_COLUMNS {
            return Err(Error::Invalid(format!(
                "The sheet `{}` has {} columns. A sheet has at most 16,384 columns.",
                options.name,
                options.columns.len()
            )));
        }
        if options.freeze_rows >= u64::from(MAX_ROWS)
            || options.freeze_columns >= MAX_COLUMNS as u64
        {
            return Err(Error::Invalid(format!(
                "The sheet `{}` freezes more rows or columns than it has.",
                options.name
            )));
        }
        if let Some(zoom) = options.zoom
            && !(10..=400).contains(&zoom)
        {
            return Err(Error::Invalid(format!(
                "The zoom {zoom} is not allowed. Zoom is from 10 to 400."
            )));
        }
        if let Some(range) = &options.autofilter
            && !is_range(range)
        {
            return Err(Error::Invalid(format!(
                "The autofilter range `{range}` is not a range of cells, such as `A1:D1`."
            )));
        }
        let mut head = String::new();
        self.sheet_head(&options, &mut head)?;
        let mut open = OpenSheet {
            index: self.sheets.len(),
            columns: options.columns.iter().map(|column| column.style).collect(),
            autofilter: options.autofilter.clone(),
            next: 1,
        };
        let mut body = String::new();
        let mut formulas = false;
        let next = self.batch(&options.name, &open, first, &mut body, &mut formulas)?;
        open.next = next;
        let part = format!("xl/worksheets/sheet{}.xml", open.index + 1);
        self.archived(|this| {
            this.end_sheet()?;
            this.archive.begin(&part)?;
            this.archive.write(head.as_bytes())?;
            this.archive.write(body.as_bytes())
        })?;
        self.formulas |= formulas;
        self.sheets.push(Sheet {
            name: options.name,
            position: options.position,
            visibility: options.visibility,
            rows: first.len() as u64,
        });
        self.open = Some(open);
        Ok(self.archive.take())
    }

    /// Writes a batch of rows to the open sheet. Returns the bytes ready to leave.
    pub fn rows(&mut self, rows: &[Row]) -> Result<Vec<u8>, Error> {
        self.writing()?;
        let Some(mut open) = self.open.take() else {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you add rows.".to_owned(),
            ));
        };
        let index = open.index;
        let name = self.sheets[index].name.clone();
        let mut body = String::new();
        let mut formulas = false;
        let written = self.batch(&name, &open, rows, &mut body, &mut formulas);
        let out = match written {
            Ok(next) => {
                open.next = next;
                self.archived(|this| this.archive.write(body.as_bytes()))
            }
            Err(err) => Err(err),
        };
        self.open = Some(open);
        out?;
        self.formulas |= formulas;
        self.sheets[index].rows += rows.len() as u64;
        Ok(self.archive.take())
    }

    /// Writes `rows` of the sheet `name` as `<row>` elements into `out`, and returns the number
    /// the next row gets.
    fn batch(
        &mut self,
        name: &str,
        open: &OpenSheet,
        rows: &[Row],
        out: &mut String,
        formulas: &mut bool,
    ) -> Result<u32, Error> {
        let mut number = open.next;
        for row in rows {
            if number > MAX_ROWS {
                return Err(Error::Invalid(format!(
                    "The sheet `{name}` is full. A sheet has at most 1,048,576 rows."
                )));
            }
            if row.cells.len() > MAX_COLUMNS {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` has {} cells. A row has at most 16,384 cells.",
                    row.cells.len()
                )));
            }
            let _ = write!(out, "<row r=\"{number}\"");
            if let Some(style) = row.style {
                let xf = self
                    .styles
                    .xf(None, Some(style), None, crate::styles::Implicit::None)?;
                if xf != 0 {
                    let _ = write!(out, " s=\"{xf}\" customFormat=\"1\"");
                }
            }
            if let Some(height) = row.height {
                if !(0.0..=409.0).contains(&height) {
                    return Err(Error::Invalid(format!(
                        "Row {number} of the sheet `{name}` has the height {height}. A row height is from 0 to 409 points."
                    )));
                }
                out.push_str(" ht=\"");
                cells::number(out, height);
                out.push_str("\" customHeight=\"1\"");
            }
            if row.hidden {
                out.push_str(" hidden=\"1\"");
            }
            if row.level > 7 {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` has the outline level {}. The level is from 0 to 7.",
                    row.level
                )));
            }
            if row.level > 0 {
                let _ = write!(out, " outlineLevel=\"{}\"", row.level);
            }
            out.push('>');
            for (column, cell) in row.cells.iter().enumerate() {
                let layers = Layers {
                    column: open.columns.get(column).copied().flatten(),
                    row: row.style,
                };
                *formulas |=
                    cells::cell(out, &mut self.styles, &layers, column as u32, number, cell)?;
            }
            out.push_str("</row>");
            number += 1;
        }
        Ok(number)
    }

    fn sheet_head(&mut self, options: &SheetOptions, out: &mut String) -> Result<(), Error> {
        out.push_str(XML_HEAD);
        let _ = write!(out, "<worksheet xmlns=\"{MAIN}\" xmlns:r=\"{RELS}\">");
        if let Some(color) = &options.tab_color {
            let _ = write!(
                out,
                "<sheetPr><tabColor rgb=\"{}\"/></sheetPr>",
                argb(color)?
            );
        }
        out.push_str("<sheetViews><sheetView");
        if !options.gridlines {
            out.push_str(" showGridLines=\"0\"");
        }
        if options.right_to_left {
            out.push_str(" rightToLeft=\"1\"");
        }
        if let Some(zoom) = options.zoom {
            let _ = write!(out, " zoomScale=\"{zoom}\" zoomScaleNormal=\"{zoom}\"");
        }
        out.push_str(" workbookViewId=\"0\"");
        let (rows, columns) = (options.freeze_rows, options.freeze_columns);
        if rows == 0 && columns == 0 {
            out.push_str("/>");
        } else {
            let top_left = cells::reference(columns as u32, rows as u32 + 1);
            let pane = match (rows > 0, columns > 0) {
                (true, true) => "bottomRight",
                (true, false) => "bottomLeft",
                _ => "topRight",
            };
            out.push_str("><pane");
            if columns > 0 {
                let _ = write!(out, " xSplit=\"{columns}\"");
            }
            if rows > 0 {
                let _ = write!(out, " ySplit=\"{rows}\"");
            }
            let _ = write!(
                out,
                " topLeftCell=\"{top_left}\" activePane=\"{pane}\" state=\"frozen\"/>\
                 <selection pane=\"{pane}\" activeCell=\"{top_left}\" sqref=\"{top_left}\"/></sheetView>"
            );
        }
        out.push_str("</sheetViews><sheetFormatPr defaultRowHeight=\"15\"/>");
        let mut cols = String::new();
        for (at, column) in options.columns.iter().enumerate() {
            let xf = match column.style {
                Some(style) => {
                    self.styles
                        .xf(Some(style), None, None, crate::styles::Implicit::None)?
                }
                None => 0,
            };
            if column.width.is_none() && xf == 0 && !column.hidden {
                continue;
            }
            let _ = write!(cols, "<col min=\"{0}\" max=\"{0}\"", at + 1);
            if let Some(width) = column.width {
                if !(0.0..=255.0).contains(&width) {
                    return Err(Error::Invalid(format!(
                        "Column {} of the sheet `{}` has the width {width}. A column width is from 0 to 255.",
                        at + 1,
                        options.name
                    )));
                }
                cols.push_str(" width=\"");
                cells::number(&mut cols, width);
                cols.push_str("\" customWidth=\"1\"");
            }
            if xf != 0 {
                let _ = write!(cols, " style=\"{xf}\"");
            }
            if column.hidden {
                cols.push_str(" hidden=\"1\"");
            }
            cols.push_str("/>");
        }
        if !cols.is_empty() {
            let _ = write!(out, "<cols>{cols}</cols>");
        }
        out.push_str("<sheetData>");
        Ok(())
    }

    /// Ends the open sheet's part.
    fn end_sheet(&mut self) -> Result<(), Error> {
        let Some(open) = self.open.take() else {
            return Ok(());
        };
        let mut tail = String::from("</sheetData>");
        if let Some(range) = &open.autofilter {
            let _ = write!(tail, "<autoFilter ref=\"{range}\"/>");
        }
        tail.push_str("</worksheet>");
        self.archive.write(tail.as_bytes())?;
        self.archive.end()
    }

    /// The sheets' indexes in tab order.
    fn tab_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.sheets.len()).collect();
        order.sort_by_key(|&at| (self.sheets[at].position.unwrap_or(u64::MAX), at));
        order
    }

    /// Ends the open sheet and the workbook. Returns the last bytes, the rows of each sheet and
    /// the size of the whole file.
    pub fn finish(&mut self) -> Result<Finished, Error> {
        self.writing()?;
        if self.sheets.is_empty() {
            return Err(Error::Invalid(
                "A workbook needs at least one sheet. Call `sheet()` before `finish()`.".to_owned(),
            ));
        }
        let order = self.tab_order();
        let Some(active) = order
            .iter()
            .position(|&at| self.sheets[at].visibility == Visibility::Visible)
        else {
            return Err(Error::Invalid(
                "Every sheet of the workbook is hidden. At least one sheet must be visible."
                    .to_owned(),
            ));
        };
        let parts = self.parts(&order, active)?;
        self.archived(|this| {
            this.end_sheet()?;
            for (name, text) in &parts {
                this.archive.begin(name)?;
                this.archive.write(text.as_bytes())?;
            }
            this.archive.close()
        })?;
        self.state = State::Finished;
        let data = self.archive.take();
        Ok(Finished {
            data,
            rows: self
                .sheets
                .iter()
                .map(|sheet| SheetRows {
                    name: sheet.name.clone(),
                    rows: sheet.rows,
                })
                .collect(),
            size: self.archive.size(),
        })
    }

    /// Every part after the sheets, by name, in the order they are written.
    fn parts(&self, order: &[usize], active: usize) -> Result<Vec<(String, String)>, Error> {
        let count = self.sheets.len();
        let mut styles = String::new();
        self.styles.write(&mut styles)?;

        let mut workbook = String::from(XML_HEAD);
        let _ = write!(
            workbook,
            "<workbook xmlns=\"{MAIN}\" xmlns:r=\"{RELS}\"><workbookPr/><bookViews><workbookView activeTab=\"{active}\"/></bookViews><sheets>"
        );
        for &at in order {
            let sheet = &self.sheets[at];
            workbook.push_str("<sheet name=\"");
            xml::attr(&mut workbook, &sheet.name);
            let _ = write!(workbook, "\" sheetId=\"{0}\"", at + 1);
            match sheet.visibility {
                Visibility::Visible => {}
                Visibility::Hidden => workbook.push_str(" state=\"hidden\""),
                Visibility::VeryHidden => workbook.push_str(" state=\"veryHidden\""),
            }
            let _ = write!(workbook, " r:id=\"rId{}\"/>", at + 1);
        }
        workbook.push_str("</sheets>");
        if !self.names.is_empty() {
            workbook.push_str("<definedNames>");
            for (name, reference) in &self.names {
                workbook.push_str("<definedName name=\"");
                xml::attr(&mut workbook, name);
                workbook.push_str("\">");
                xml::text(&mut workbook, reference);
                workbook.push_str("</definedName>");
            }
            workbook.push_str("</definedNames>");
        }
        if self.formulas {
            workbook.push_str("<calcPr fullCalcOnLoad=\"1\"/>");
        }
        workbook.push_str("</workbook>");

        let mut workbook_rels = String::from(XML_HEAD);
        let _ = write!(workbook_rels, "<Relationships xmlns=\"{PACKAGE_RELS}\">");
        for at in 0..count {
            let _ = write!(
                workbook_rels,
                "<Relationship Id=\"rId{0}\" Type=\"{RELS}/worksheet\" Target=\"worksheets/sheet{0}.xml\"/>",
                at + 1
            );
        }
        let _ = write!(
            workbook_rels,
            "<Relationship Id=\"rId{}\" Type=\"{RELS}/styles\" Target=\"styles.xml\"/></Relationships>",
            count + 1
        );

        let properties = &self.options.properties;
        let created = match &properties.created {
            Some(clock) => clock_text(clock)?,
            None => "1980-01-01T00:00:00Z".to_owned(),
        };
        let mut core = String::from(XML_HEAD);
        core.push_str(
            "<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
             xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" \
             xmlns:dcmitype=\"http://purl.org/dc/dcmitype/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">",
        );
        for (tag, value) in [
            ("dc:title", &properties.title),
            ("dc:subject", &properties.subject),
            ("dc:creator", &properties.author),
            ("cp:keywords", &properties.keywords),
            ("cp:lastModifiedBy", &properties.author),
        ] {
            if let Some(value) = value {
                let _ = write!(core, "<{tag}>");
                xml::text(&mut core, value);
                let _ = write!(core, "</{tag}>");
            }
        }
        let _ = write!(
            core,
            "<dcterms:created xsi:type=\"dcterms:W3CDTF\">{created}</dcterms:created>\
             <dcterms:modified xsi:type=\"dcterms:W3CDTF\">{created}</dcterms:modified></cp:coreProperties>"
        );

        let mut app = String::from(XML_HEAD);
        app.push_str(
            "<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\" \
             xmlns:vt=\"http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes\">\
             <Application>Novis</Application>",
        );
        if let Some(company) = &properties.company {
            app.push_str("<Company>");
            xml::text(&mut app, company);
            app.push_str("</Company>");
        }
        app.push_str("</Properties>");

        let mut root_rels = String::from(XML_HEAD);
        let _ = write!(
            root_rels,
            "<Relationships xmlns=\"{PACKAGE_RELS}\">\
             <Relationship Id=\"rId1\" Type=\"{RELS}/officeDocument\" Target=\"xl/workbook.xml\"/>\
             <Relationship Id=\"rId2\" Type=\"{PACKAGE_RELS}/metadata/core-properties\" Target=\"docProps/core.xml\"/>\
             <Relationship Id=\"rId3\" Type=\"{RELS}/extended-properties\" Target=\"docProps/app.xml\"/>\
             </Relationships>"
        );

        let mut types = String::from(XML_HEAD);
        types.push_str(
            "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
             <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
             <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
             <Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>",
        );
        for at in 0..count {
            let _ = write!(
                types,
                "<Override PartName=\"/xl/worksheets/sheet{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>",
                at + 1
            );
        }
        types.push_str(
            "<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>\
             <Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
             <Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/>\
             </Types>",
        );

        Ok(vec![
            ("xl/styles.xml".to_owned(), styles),
            ("xl/workbook.xml".to_owned(), workbook),
            ("xl/_rels/workbook.xml.rels".to_owned(), workbook_rels),
            ("docProps/core.xml".to_owned(), core),
            ("docProps/app.xml".to_owned(), app),
            ("_rels/.rels".to_owned(), root_rels),
            ("[Content_Types].xml".to_owned(), types),
        ])
    }
}
