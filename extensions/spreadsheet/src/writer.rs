//! The workbook writer: sheets one after another, rows in batches, and the parts `finish` adds
//! (`rule:core-classes/spreadsheet-writes-as-a-stream`).
//!
//! [`Writer::sheet`] checks the sheet's options, ends the open sheet and opens the next one. The
//! new sheet's zip entry starts with its first batch of rows, or when it ends with none, with
//! what xlsx writes before the rows: the tab color, the sheet view with its frozen pane,
//! gridlines, direction and zoom, and the columns. Waiting for the first batch lets auto width
//! measure it. [`Writer::rows`] writes a batch into `<sheetData>`. A batch is built as text first
//! and goes into the compressor only once every cell of it has been written, so a batch that
//! fails leaves the sheet as it was and the guest keeps no row of it. [`Writer::finish`] ends the
//! last sheet with its autofilter and writes `styles.xml`, `workbook.xml` with the sheets in tab
//! order, the relationships, the document properties, the content types and the zip directory.
//! Each call returns the archive's bytes produced so far, so the writer holds nothing it has
//! already given out.
//!
//! **The header row** (`rule:core-classes/spreadsheet-styles-and-defaults`) is a first row whose
//! `header` is set, and only the first row can be one. It gets the [`Header`] layer as its row
//! style, the rows above the second are frozen, an autofilter covers the header and every row
//! below it with the hidden `_xlnm._FilterDatabase` name Excel writes beside one, and every column
//! the options do not list has auto width. It is not counted in the sheet's rows. **Auto width**
//! measures the first batch, header included (`width`'s module doc): the Novis half sends 1,000
//! rows in it, or every row of an array.
//!
//! **The grid is Excel's.** A row past 1,048,576 is an error naming the limit, unless
//! `new_sheet_on_overflow` is set: then the rows that fit end the sheet, and the rest go on a
//! sheet with the same options named `<name> (2)`, then `(3)`, its header row repeated.
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
//! and the program may go on. The one exception is [`Writer::sheet`] given first rows that fail:
//! the sheet is open by then, and stays open with no rows.
//!
//! **The sheet-end parts** ([`Writer::part`]) are kept by the open sheet until it ends, and then
//! written after its rows, with the entries of its own they need (`tail`'s module doc). A part
//! goes to the sheet that is open when it is added, so a part added after an overflow goes to the
//! sheet the overflow opened. The print area and the rows to repeat are names `workbook.xml`
//! defines for the sheet, beside its autofilter's.
//!
//! **An image is its own entry, `xl/media/image<n>.<ext>`**, numbered across the workbook. Added
//! before the sheet's first row, it is written by the call that adds it. Once the sheet's own
//! entry has started, the zip entry is still open, and a zip entry cannot be split, so the image's
//! bytes wait in [`OpenSheet`] until the sheet ends. Memory: per writing request, the images added
//! to the open sheet after its first row, until the sheet ends.
//!
//! **A chart is its own part, `xl/charts/chart<n>.xml`**, numbered across the workbook and written
//! when its sheet ends, so a range that grows with the sheet ends at the sheet's last row
//! (`chart`'s module doc).
//!
//! **A table** (`table`'s module doc) is numbered across the workbook, and its name is unique
//! among the tables and the defined names. Its header row names its columns when it is written,
//! and its totals row is written right after its last row, so [`Writer::rows`] splits a batch
//! there. A formula written in a table's rows has its structured references qualified. The
//! sheet's default autofilter is left out when it overlaps a table, and a sheet does not end
//! while a table's header row or totals row is still to write.

use std::fmt::Write as _;

use crate::cells::{self, Layers};
use crate::chart::is_r1c1;
use crate::conditional;
use crate::drawing::{Format, Grid};
use crate::reference::{self, Reference};
use crate::styles::{Implicit, Styles, argb};
use crate::table::{Placed, Table, expand};
use crate::tail::{Groups, Part, Tail, area};
use crate::zip::Archive;
use crate::{
    Cell, Clock, Error, Finished, Row, SheetOptions, SheetRows, Style, Value, Visibility,
    WorkbookOptions, width, xml,
};

/// The declaration every part begins with.
pub const XML_HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

/// Excel's grid.
pub(crate) const MAX_ROWS: u32 = 1_048_576;
pub(crate) const MAX_COLUMNS: usize = 16_384;
/// The width Excel writes for a column of its default 8.43 characters.
const DEFAULT_WIDTH: f64 = 9.140625;

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// A sheet already started. `names` are the names `workbook.xml` defines for it, each with its
/// reference in the sheet, `notes` whether it has a comments part and `drawing` whether it has
/// images; all three are set when it ends.
struct Sheet {
    name: String,
    position: Option<u64>,
    visibility: Visibility,
    rows: u64,
    names: Vec<(&'static str, String)>,
    notes: bool,
    drawing: bool,
}

/// The sheet whose rows are being written.
struct OpenSheet {
    /// The sheet's index in [`Writer::sheets`].
    index: usize,
    options: SheetOptions,
    /// The layer of its header row.
    header_style: Option<u64>,
    /// Whether its zip entry has started.
    started: bool,
    columns: Vec<Option<u64>>,
    /// Its header row, kept to repeat on the next sheet when it overflows.
    header: Option<Row>,
    /// The name the overflow sheets count from, and the number of this one, from 1.
    base: String,
    part: u32,
    /// The number the next row gets, from 1.
    next: u32,
    /// Its sheet-end parts, its outline groups, and whether its head fits it to pages.
    tail: Tail,
    groups: Groups,
    fit: bool,
    /// The media entries added after its own entry started, by name, until it ends.
    waiting: Vec<(String, Vec<u8>)>,
    /// Its column widths, row heights and number columns, for its anchors and charts.
    grid: Grid,
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
    /// The media entries written so far, and their formats in order, for the content types.
    media: usize,
    formats: Vec<Format>,
    /// The names of the tables added so far, the one numbered `n` at `n - 1`.
    tables: Vec<String>,
    /// The charts added so far, each numbered across the workbook.
    charts: usize,
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

/// Checks `name` against Excel's rules for a sheet name and the names in `taken`.
pub(crate) fn check_sheet_name<'a>(
    name: &str,
    mut taken: impl Iterator<Item = &'a str>,
) -> Result<(), Error> {
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
    if name.chars().any(char::is_control) {
        return bad("has a control character. A sheet name cannot have one");
    }
    if name.starts_with('\'') || name.ends_with('\'') {
        return bad("starts or ends with `'`. That is not allowed");
    }
    let lower = name.to_lowercase();
    if lower == "history" {
        return bad("is not allowed. Excel keeps the name `History` for itself");
    }
    if taken.any(|sheet| sheet.to_lowercase() == lower) {
        return bad("is used twice. Each sheet needs its own name");
    }
    Ok(())
}

/// Turns each column's format into a layer over its style, so a cell layers one column style.
pub(crate) fn fold_column_formats(
    styles: &mut Styles,
    options: &mut SheetOptions,
) -> Result<(), Error> {
    for column in &mut options.columns {
        if let Some(format) = column.format.take() {
            let base = match column.style {
                Some(id) => styles.get(id)?.clone(),
                None => Style::default(),
            };
            let style = Style {
                format: Some(format),
                ..base
            };
            column.style = Some(styles.layer(style)?);
        }
    }
    Ok(())
}

/// The layer of a header row under `options`, or `None` for an unstyled one.
pub(crate) fn header_layer(
    styles: &mut Styles,
    options: &SheetOptions,
) -> Result<Option<u64>, Error> {
    let header = options.header;
    if !header.default_style {
        if let Some(id) = header.style {
            styles.get(id)?;
        }
        return Ok(header.style);
    }
    let base = crate::styles::header();
    let style = match header.style {
        Some(id) => styles.get(id)?.over(&base),
        None => base,
    };
    styles.layer(style).map(Some)
}

/// Checks a sheet's options against Excel's rules, the sheet names in `taken` and the layers
/// of `styles`.
pub(crate) fn check_options<'a>(
    options: &SheetOptions,
    taken: impl Iterator<Item = &'a str>,
    styles: &Styles,
) -> Result<(), Error> {
    check_sheet_name(&options.name, taken)?;
    if options.columns.len() > MAX_COLUMNS {
        return Err(Error::Invalid(format!(
            "The sheet `{}` has {} columns. A sheet has at most 16,384 columns.",
            options.name,
            options.columns.len()
        )));
    }
    if options.freeze_rows >= u64::from(MAX_ROWS) || options.freeze_columns >= MAX_COLUMNS as u64 {
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
    if let Some(color) = &options.tab_color {
        argb(color)?;
    }
    for (at, column) in options.columns.iter().enumerate() {
        if let Some(width) = column.width
            && !(0.0..=255.0).contains(&width)
        {
            return Err(Error::Invalid(format!(
                "Column {} of the sheet `{}` has the width {width}. A column width is from 0 to 255.",
                at + 1,
                options.name
            )));
        }
        if let Some(style) = column.style {
            styles.get(style)?;
        }
    }
    Ok(())
}

/// Checks a defined name against Excel's rules and the names already taken, by `defined` and
/// by `tables`.
pub(crate) fn check_name(
    name: &str,
    reference: &str,
    defined: &[(String, String)],
    tables: &[String],
) -> Result<(), Error> {
    let mut chars = name.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '\\');
    if !first_ok || !chars.all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
        return Err(Error::Invalid(format!(
            "`{name}` is not a name Excel allows. A name starts with a letter or `_`, and has only letters, digits, `_` and `.`."
        )));
    }
    if name.chars().count() > 255 {
        return Err(Error::Invalid(format!(
            "The name `{name}` is too long. A name has at most 255 characters."
        )));
    }
    if is_range(&name.to_ascii_uppercase()) || is_r1c1(name) {
        return Err(Error::Invalid(format!(
            "`{name}` is a cell reference, so it cannot be a name."
        )));
    }
    reference::parse(reference, true, || {
        Error::Invalid(format!(
            "The reference `{reference}` of the name `{name}` is not a range of this workbook. Write a sheet and a range, such as `'Sheet 1'!$A$1:$B$10`."
        ))
    })?;
    let lower = name.to_lowercase();
    if defined.iter().any(|(n, _)| n.to_lowercase() == lower) {
        return Err(Error::Invalid(format!(
            "The name `{name}` is defined twice."
        )));
    }
    if tables.iter().any(|n| n.to_lowercase() == lower) {
        return Err(Error::Invalid(format!(
            "The name `{name}` is a table's name already."
        )));
    }
    Ok(())
}

/// Checks that the sheet of each defined name in `names` is one of `sheets`, once the workbook
/// has every sheet.
pub(crate) fn check_name_sheets<'a>(
    names: &[(String, String)],
    sheets: impl Iterator<Item = &'a str> + Clone,
) -> Result<(), Error> {
    for (name, text) in names {
        let Ok(Reference {
            sheet: Some(sheet), ..
        }) = reference::parse(text, true, || Error::Invalid(String::new()))
        else {
            continue;
        };
        let lower = sheet.to_lowercase();
        if !sheets.clone().any(|known| known.to_lowercase() == lower) {
            return Err(Error::Invalid(format!(
                "The name `{name}` refers to the sheet `{sheet}`, which the workbook does not have. Add the sheet, or change the reference."
            )));
        }
    }
    Ok(())
}

/// The A1 address of the column at `column`, from 0, in the row `row`, such as `C7`, when it is
/// inside the grid.
pub(crate) fn address(column: u32, row: u32) -> Result<String, Error> {
    if column as usize >= MAX_COLUMNS || !(1..=MAX_ROWS).contains(&row) {
        return Err(Error::Invalid(format!(
            "Column {} of row {row} is outside the sheet. A sheet has 16,384 columns and 1,048,576 rows.",
            u64::from(column) + 1
        )));
    }
    Ok(cells::reference(column, row))
}

/// Whether `text` is a cell or a range of cells such as `A1:D10`, with `$` allowed.
pub(crate) fn is_range(text: &str) -> bool {
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

/// `range` with every row and column fixed, as `$A$1:$C$10`.
pub(crate) fn absolute(range: &str) -> String {
    let fixed = |cell: &str| {
        let cell = cell.replace('$', "");
        let letters = cell.bytes().take_while(u8::is_ascii_alphabetic).count();
        format!("${}${}", &cell[..letters], &cell[letters..])
    };
    match range.split_once(':') {
        Some((from, to)) => format!("{}:{}", fixed(from), fixed(to)),
        None => fixed(range),
    }
}

/// The name of the sheet number `part` of an overflow from `base`, as `Orders (2)`, with `base`
/// shortened to keep it within 31 characters.
pub(crate) fn continued(base: &str, part: u32) -> String {
    let suffix = format!(" ({part})");
    let keep = 31 - suffix.chars().count();
    let mut name: String = base.chars().take(keep).collect();
    name.push_str(&suffix);
    name
}

/// `name` quoted for a reference, as `'Shop orders'`.
pub(crate) fn quoted(name: &str) -> String {
    format!("'{}'", name.replace('\'', "''"))
}

pub(crate) fn clock_text(clock: &Clock) -> Result<String, Error> {
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
            media: 0,
            formats: Vec::new(),
            tables: Vec::new(),
            charts: 0,
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
        check_name(name, reference, &self.names, &self.tables)?;
        self.names.push((name.to_owned(), reference.to_owned()));
        Ok(())
    }

    /// Ends the open sheet, if any, opens the next one and writes its first rows. Returns the
    /// bytes ready to leave.
    pub fn sheet(&mut self, mut options: SheetOptions, first: &[Row]) -> Result<Vec<u8>, Error> {
        self.writing()?;
        self.check_options(&options)?;
        fold_column_formats(&mut self.styles, &mut options)?;
        let header_style = header_layer(&mut self.styles, &options)?;
        self.ready_to_end()?;
        self.archived(|this| this.end_sheet())?;
        let base = options.name.clone();
        self.open_sheet(options, header_style, base, 1);
        if first.is_empty() {
            return Ok(self.archive.take());
        }
        self.rows(first)
    }

    fn open_sheet(
        &mut self,
        options: SheetOptions,
        header_style: Option<u64>,
        base: String,
        part: u32,
    ) {
        let index = self.sheets.len();
        self.sheets.push(Sheet {
            name: options.name.clone(),
            position: options.position,
            visibility: options.visibility,
            rows: 0,
            names: Vec::new(),
            notes: false,
            drawing: false,
        });
        self.open = Some(OpenSheet {
            index,
            options,
            header_style,
            started: false,
            columns: Vec::new(),
            header: None,
            base,
            part,
            next: 1,
            tail: Tail::default(),
            groups: Groups::default(),
            fit: false,
            waiting: Vec::new(),
            grid: Grid::default(),
        });
    }

    /// Adds a sheet-end part to the open sheet. Returns the bytes ready to leave.
    pub fn part(&mut self, part: Part) -> Result<Vec<u8>, Error> {
        self.writing()?;
        let Some(open) = self.open.as_mut() else {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you add to it.".to_owned(),
            ));
        };
        let sheet = open.options.name.as_str();
        match part {
            Part::Group(group) => open.groups.add(&group, sheet, open.next, open.started)?,
            Part::Print(print) => {
                if print.fits() && open.started {
                    return Err(Error::Invalid(format!(
                        "The sheet `{sheet}` cannot be fitted to pages after its first row. Set the fit before you add rows."
                    )));
                }
                open.fit |= print.fits();
                open.tail.add(Part::Print(print), sheet)?;
            }
            Part::Image(image) => {
                let format = open.tail.image(&image, self.media + 1)?;
                self.media += 1;
                if !self.formats.contains(&format) {
                    self.formats.push(format);
                    self.formats.sort();
                }
                let name = format!("xl/media/image{}.{}", self.media, format.extension());
                if open.started {
                    open.waiting.push((name, image.image));
                } else {
                    return self.archived(|writer| {
                        writer.archive.begin(&name)?;
                        writer.archive.write(&image.image)?;
                        writer.archive.end()?;
                        Ok(writer.archive.take())
                    });
                }
            }
            Part::Chart(chart) => {
                let first = if open.header.is_some() { 2 } else { 1 };
                let reach = open.next.saturating_sub(1).max(first);
                open.tail.chart(&chart, self.charts + 1, sheet, reach)?;
                self.charts += 1;
            }
            Part::Table(table) => return self.table(table),
            Part::Conditional(rule) => {
                conditional::check(&rule)?;
                let dxf = rule
                    .style
                    .clone()
                    .map(|style| self.styles.dxf(style))
                    .transpose()?;
                let written = conditional::write(&rule, dxf, open.tail.next_priority());
                open.tail.conditional(written);
            }
            part => open.tail.add(part, sheet)?,
        }
        Ok(self.archive.take())
    }

    /// Adds `table` to the open sheet, and writes its totals row when it has totals (`table`'s
    /// module doc). Returns the bytes ready to leave.
    fn table(&mut self, table: Table) -> Result<Vec<u8>, Error> {
        let id = self.tables.len() + 1;
        let Some(open) = self.open.as_mut() else {
            unreachable!("`part` checked that a sheet is open");
        };
        let sheet = open.options.name.as_str();
        let mut placed = Placed::new(table, id, sheet)?;
        let lower = placed.name.to_lowercase();
        if self.tables.iter().any(|n| n.to_lowercase() == lower)
            || self.names.iter().any(|(n, _)| n.to_lowercase() == lower)
        {
            return Err(Error::Invalid(format!(
                "The name `{}` is used twice. A table's name is different from every other table and name of the workbook.",
                placed.name
            )));
        }
        if let Some(range) = &open.options.autofilter
            && area(range).is_some_and(|filter| filter.overlaps(&placed.area))
        {
            return Err(Error::Invalid(format!(
                "The table `{}` overlaps the autofilter `{range}` of the sheet `{sheet}`. A table has its own filter.",
                placed.name
            )));
        }
        open.tail.check_table(&placed, sheet)?;
        if placed.area.top < open.next {
            match &open.header {
                Some(header) if placed.area.top == 1 => placed.header(header)?,
                _ => {
                    return Err(Error::Invalid(format!(
                        "The header row {} of the table `{}` is written already. Add a table before its header row, or start it at the sheet's header row.",
                        placed.area.top, placed.name
                    )));
                }
            }
        }
        if placed.has_totals() {
            if placed.area.bottom + 1 < open.next {
                return Err(Error::Invalid(format!(
                    "The table `{}` has a totals row after its last row {}, and row {} is written already. Add the table before that row.",
                    placed.name,
                    placed.area.bottom,
                    placed.area.bottom + 1
                )));
            }
            if placed.area.bottom >= MAX_ROWS {
                return Err(Error::Invalid(format!(
                    "The table `{}` ends at the sheet's last row, so it has no room for its totals row.",
                    placed.name
                )));
            }
            if placed.area.bottom + 1 == open.next {
                // The header is named, so this checks each total names a column.
                placed.totals_row()?;
            }
        }
        let name = placed.name.clone();
        open.tail.table(placed, sheet)?;
        if let Err(error) = self.append(&[]) {
            if let Some(open) = self.open.as_mut() {
                open.tail.forget_table(id);
            }
            return Err(error);
        }
        self.tables.push(name);
        Ok(self.archive.take())
    }

    /// Checks that the open sheet can end: every table's header row and totals row is written.
    fn ready_to_end(&self) -> Result<(), Error> {
        if let Some(open) = &self.open
            && let Some(table) = open.tail.unfinished_table()
        {
            return Err(Error::Invalid(format!(
                "The table `{}` covers rows {} to {}, and the sheet `{}` ends at row {}. Write the table's rows before the sheet ends.",
                table.name,
                table.area.top,
                table.area.bottom,
                open.options.name,
                open.next - 1
            )));
        }
        Ok(())
    }

    /// The A1 address of the column at `column`, from 0, in the row `row`, such as `C7`
    /// (`rule:core-classes/spreadsheet-formula-is-a-value`).
    pub fn cell(&self, column: u32, row: u32) -> Result<String, Error> {
        address(column, row)
    }

    /// The open sheet's data range in the column at `column`, from 0, over the rows written so
    /// far, such as `C2:C1043`. The header row is not in it. Before the first data row, the range
    /// is that row alone.
    pub fn column(&self, column: u32) -> Result<String, Error> {
        let Some(open) = &self.open else {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you ask for a column.".to_owned(),
            ));
        };
        let first = if open.header.is_some() { 2 } else { 1 };
        let last = open.next.saturating_sub(1).max(first);
        Ok(format!(
            "{}:{}",
            self.cell(column, first)?,
            self.cell(column, last)?
        ))
    }

    /// Checks a sheet's options against Excel's rules and the sheets already started.
    fn check_options(&self, options: &SheetOptions) -> Result<(), Error> {
        let taken = self.sheets.iter().map(|sheet| sheet.name.as_str());
        check_options(options, taken, &self.styles)
    }

    /// Writes a batch of rows to the open sheet. Returns the bytes ready to leave.
    pub fn rows(&mut self, rows: &[Row]) -> Result<Vec<u8>, Error> {
        if self.state == State::Finished
            && let Some(last) = self.sheets.last()
        {
            return Err(Error::Invalid(format!(
                "The sheet `{}` is finished, because the workbook is finished. `finish()` is the last call on a workbook.",
                last.name
            )));
        }
        self.writing()?;
        // A part that fails after another was written leaves its bytes in the archive, and they
        // leave with the next call that succeeds.
        self.append(rows)?;
        Ok(self.archive.take())
    }

    /// Writes `rows` to the open sheet, and goes on to the next sheet at the end of the grid when
    /// the sheet's options say so.
    fn append(&mut self, rows: &[Row]) -> Result<(), Error> {
        let Some(open) = &self.open else {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you add rows.".to_owned(),
            ));
        };
        // A table's totals row goes right after its last row.
        if let Some((id, totals)) = open.tail.totals(open.next) {
            let totals = totals?;
            let mut open = self.open.take().expect("a sheet is open");
            let out = self.write_batch(&mut open, &[totals]);
            if out.is_ok() {
                open.tail.totaled(id);
            }
            self.open = Some(open);
            out?;
            return self.append(rows);
        }
        if let Some(cut) = open.tail.totals_due(open.next, rows.len())
            && cut < rows.len()
        {
            self.append(&rows[..cut])?;
            return self.append(&rows[cut..]);
        }
        let room = (MAX_ROWS + 1).saturating_sub(open.next) as usize;
        if rows.len() > room && open.options.new_sheet_on_overflow {
            self.append(&rows[..room])?;
            self.overflow()?;
            return self.append(&rows[room..]);
        }
        let mut open = self.open.take().expect("a sheet is open");
        let out = self.write_batch(&mut open, rows);
        let due = open.tail.totals(open.next).is_some();
        self.open = Some(open);
        out?;
        if due && !rows.is_empty() {
            return self.append(&[]);
        }
        Ok(())
    }

    /// Ends the open sheet, which is full, and opens the next sheet of its overflow with the same
    /// options and its header row.
    fn overflow(&mut self) -> Result<(), Error> {
        let Some(open) = &self.open else {
            return Ok(());
        };
        let mut options = open.options.clone();
        let (base, header, header_style) =
            (open.base.clone(), open.header.clone(), open.header_style);
        let mut part = open.part + 1;
        options.name = continued(&base, part);
        while check_sheet_name(&options.name, self.sheets.iter().map(|s| s.name.as_str())).is_err()
        {
            part += 1;
            options.name = continued(&base, part);
        }
        options.position = None;
        self.archived(|this| this.end_sheet())?;
        self.open_sheet(options, header_style, base, part);
        match header {
            Some(header) => self.append(&[header]),
            None => Ok(()),
        }
    }

    /// Writes `rows` to `open`, starting its zip entry with the first of them.
    fn write_batch(&mut self, open: &mut OpenSheet, rows: &[Row]) -> Result<(), Error> {
        let Some(first) = rows.first() else {
            return Ok(());
        };
        let header = (open.next == 1 && first.header).then(|| first.clone());
        let mut head = String::new();
        let mut columns = if open.started {
            None
        } else {
            Some(self.sheet_head(open, header.is_some(), rows, &mut head)?)
        };
        let mut body = String::new();
        let mut formulas = false;
        let layers = columns
            .as_ref()
            .map_or(open.columns.as_slice(), |(layers, _)| layers);
        let next = self.batch(open, layers, rows, &mut body, &mut formulas)?;
        open.tail.headers(rows, open.next)?;
        if let Some((_, grid)) = &mut columns {
            open.grid = std::mem::take(grid);
        }
        for (at, row) in rows.iter().enumerate() {
            let hidden = row.hidden || open.groups.row(open.next + at as u32).hidden;
            open.grid.row(row, hidden);
        }
        let part = format!("xl/worksheets/sheet{}.xml", open.index + 1);
        let started = open.started;
        self.archived(|this| {
            if !started {
                this.archive.begin(&part)?;
                this.archive.write(head.as_bytes())?;
            }
            this.archive.write(body.as_bytes())
        })?;
        if let Some((columns, _)) = columns {
            open.columns = columns;
            open.started = true;
        }
        let counted = rows.len() - usize::from(header.is_some());
        if header.is_some() {
            open.header = header;
        }
        open.next = next;
        self.formulas |= formulas;
        self.sheets[open.index].rows += counted as u64;
        Ok(())
    }

    /// Writes `rows` of the sheet `open` as `<row>` elements into `out`, each cell over its
    /// column's layer in `columns`, and returns the number the next row gets.
    fn batch(
        &mut self,
        open: &OpenSheet,
        columns: &[Option<u64>],
        rows: &[Row],
        out: &mut String,
        formulas: &mut bool,
    ) -> Result<u32, Error> {
        let name = open.options.name.as_str();
        let mut number = open.next;
        for row in rows {
            if number > MAX_ROWS {
                return Err(Error::Invalid(format!(
                    "The sheet `{name}` is full. A sheet has at most 1,048,576 rows. To go on, set the sheet option `overflow: 'newSheet'`."
                )));
            }
            if row.header && number != 1 {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` is a header row. Only the first row of a sheet can be its header row."
                )));
            }
            let style = if row.header {
                open.header_style.or(row.style)
            } else {
                row.style
            };
            let outline = open.groups.row(number);
            if row.cells.len() > MAX_COLUMNS {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` has {} cells. A row has at most 16,384 cells.",
                    row.cells.len()
                )));
            }
            let _ = write!(out, "<row r=\"{number}\"");
            if let Some(style) = style {
                let xf = self.styles.xf(None, Some(style), None, Implicit::None)?;
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
            if row.hidden || outline.hidden {
                out.push_str(" hidden=\"1\"");
            }
            let level = row.level + outline.level;
            if level > 7 {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` has the outline level {level}, counting its groups. The level is from 0 to 7."
                )));
            }
            if level > 0 {
                let _ = write!(out, " outlineLevel=\"{level}\"");
            }
            if outline.collapsed {
                out.push_str(" collapsed=\"1\"");
            }
            out.push('>');
            for (column, cell) in row.cells.iter().enumerate() {
                let layers = Layers {
                    column: columns.get(column).copied().flatten(),
                    row: style,
                };
                // A formula in a table's rows has its structured references qualified.
                let qualified = match &cell.value {
                    Some(Value::Formula {
                        formula,
                        cached_text,
                        cached_number,
                    }) if formula.contains('[') => {
                        open.tail.table_at(column as u32, number).map(|table| Cell {
                            value: Some(Value::Formula {
                                formula: expand(formula, table),
                                cached_text: cached_text.clone(),
                                cached_number: *cached_number,
                            }),
                            style: cell.style,
                        })
                    }
                    _ => None,
                };
                let cell = qualified.as_ref().unwrap_or(cell);
                *formulas |=
                    cells::cell(out, &mut self.styles, &layers, column as u32, number, cell)?;
            }
            out.push_str("</row>");
            number += 1;
        }
        Ok(number)
    }

    /// Writes what the sheet `open` has before its rows into `out`, measuring `first` for auto
    /// width, and returns the columns' layers and a grid with the columns' widths. `header` says
    /// whether `first` starts with the header row.
    fn sheet_head(
        &mut self,
        open: &OpenSheet,
        header: bool,
        first: &[Row],
        out: &mut String,
    ) -> Result<(Vec<Option<u64>>, Grid), Error> {
        let options = &open.options;
        let mut grid = Grid::default();
        out.push_str(XML_HEAD);
        let _ = write!(out, "<worksheet xmlns=\"{MAIN}\" xmlns:r=\"{RELS}\">");
        if options.tab_color.is_some() || open.fit {
            out.push_str("<sheetPr>");
            if let Some(color) = &options.tab_color {
                let _ = write!(out, "<tabColor rgb=\"{}\"/>", argb(color)?);
            }
            if open.fit {
                out.push_str("<pageSetUpPr fitToPage=\"1\"/>");
            }
            out.push_str("</sheetPr>");
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
        let rows = if header && options.header.freeze {
            options.freeze_rows.max(1)
        } else {
            options.freeze_rows
        };
        let columns = options.freeze_columns;
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
        out.push_str("</sheetViews><sheetFormatPr defaultRowHeight=\"15\"");
        let (row_depth, column_depth) = open.groups.depth();
        let first_depth = first.iter().map(|row| row.level).max().unwrap_or(0);
        let row_depth = (row_depth + first_depth).min(7);
        if row_depth > 0 {
            let _ = write!(out, " outlineLevelRow=\"{row_depth}\"");
        }
        if column_depth > 0 {
            let _ = write!(out, " outlineLevelCol=\"{}\"", column_depth.min(7));
        }
        out.push_str("/>");
        // A column the options do not list has auto width under a header row.
        let count = if header {
            let widest = first.iter().map(|row| row.cells.len()).max().unwrap_or(0);
            widest.max(options.columns.len())
        } else {
            options.columns.len()
        };
        let count = count.max(open.groups.columns_reached());
        let mut cols = String::new();
        for at in 0..count {
            let column = options.columns.get(at);
            let xf = match column.and_then(|column| column.style) {
                Some(style) => self.styles.xf(Some(style), None, None, Implicit::None)?,
                None => 0,
            };
            let outline = open.groups.column(at as u32);
            let hidden = column.is_some_and(|column| column.hidden) || outline.hidden;
            let width = match column.and_then(|column| column.width) {
                Some(width) => Some(width),
                None if column.map_or(header, |column| column.auto) => {
                    let layer = column.and_then(|column| column.style);
                    let styles = &self.styles;
                    let widest = first
                        .iter()
                        .filter_map(|row| {
                            let cell = row.cells.get(at)?;
                            let style = if row.header {
                                open.header_style.or(row.style)
                            } else {
                                row.style
                            };
                            Some(width::value(cell.value.as_ref()?, |implicit| {
                                styles.code(layer, style, cell.style, implicit).ok()?
                            }))
                        })
                        .max()
                        .unwrap_or(0);
                    width::column(widest)
                }
                None => None,
            };
            if width.is_some() || hidden {
                grid.width(at as u32, width.unwrap_or(DEFAULT_WIDTH), hidden);
            }
            if width.is_none() && xf == 0 && !hidden && outline == Default::default() {
                continue;
            }
            let _ = write!(cols, "<col min=\"{0}\" max=\"{0}\"", at + 1);
            if let Some(width) = width {
                cols.push_str(" width=\"");
                cells::number(&mut cols, width);
                cols.push_str("\" customWidth=\"1\"");
            }
            if xf != 0 {
                let _ = write!(cols, " style=\"{xf}\"");
            }
            if hidden {
                cols.push_str(" hidden=\"1\"");
            }
            if outline.level > 0 {
                let _ = write!(cols, " outlineLevel=\"{}\"", outline.level.min(7));
            }
            if outline.collapsed {
                cols.push_str(" collapsed=\"1\"");
            }
            cols.push_str("/>");
        }
        if !cols.is_empty() {
            let _ = write!(out, "<cols>{cols}</cols>");
        }
        out.push_str("<sheetData>");
        Ok((
            options.columns.iter().map(|column| column.style).collect(),
            grid,
        ))
    }

    /// Ends the open sheet's part, starting it first when no row was written to it, and writes
    /// the entries its sheet-end parts need.
    fn end_sheet(&mut self) -> Result<(), Error> {
        let Some(mut open) = self.open.take() else {
            return Ok(());
        };
        if !open.started {
            let mut head = String::new();
            open.grid = self.sheet_head(&open, false, &[], &mut head)?.1;
            self.archive
                .begin(&format!("xl/worksheets/sheet{}.xml", open.index + 1))?;
            self.archive.write(head.as_bytes())?;
        }
        let mut tail = String::from("</sheetData>");
        let filter = match (&open.options.autofilter, &open.header) {
            (Some(range), _) => Some(range.clone()),
            (None, Some(header)) if open.options.header.filter => {
                let columns = header.cells.len().max(open.options.columns.len()).max(1);
                let last = open.next.saturating_sub(1).max(1);
                let range = format!("A1:{}", cells::reference(columns as u32 - 1, last));
                // A table has its own filter, and Excel allows no other over it.
                area(&range)
                    .is_some_and(|at| !open.tail.overlaps_table(&at))
                    .then_some(range)
            }
            _ => None,
        };
        open.tail.write(&mut tail, filter.as_deref());
        tail.push_str("</worksheet>");
        self.archive.write(tail.as_bytes())?;
        self.archive.end()?;
        for (name, bytes) in &open.waiting {
            self.archive.begin(name)?;
            self.archive.write(bytes)?;
            self.archive.end()?;
        }
        let number = open.index + 1;
        let mut entries = Vec::new();
        if let Some(rels) = open.tail.rels(number) {
            entries.push((format!("xl/worksheets/_rels/sheet{number}.xml.rels"), rels));
        }
        entries.extend(open.tail.notes(number));
        entries.extend(
            open.tail
                .drawing(number, open.next.saturating_sub(1), &open.grid),
        );
        entries.extend(open.tail.tables());
        for (name, text) in &entries {
            self.archive.begin(name)?;
            self.archive.write(text.as_bytes())?;
            self.archive.end()?;
        }
        let sheet = &mut self.sheets[open.index];
        if let Some(range) = &filter {
            sheet.names.push(("_xlnm._FilterDatabase", absolute(range)));
        }
        sheet.names.extend(open.tail.print_names());
        sheet.notes = open.tail.has_notes();
        sheet.drawing = open.tail.has_drawing();
        Ok(())
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
        check_name_sheets(
            &self.names,
            self.sheets.iter().map(|sheet| sheet.name.as_str()),
        )?;
        // The last sheet ends first, because its autofilter is a defined name in `workbook.xml`.
        self.ready_to_end()?;
        self.archived(|this| this.end_sheet())?;
        let parts = self.parts(&order, active)?;
        self.archived(|this| {
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
        let local: Vec<(usize, &str, String)> = order
            .iter()
            .enumerate()
            .flat_map(|(tab, &at)| {
                let sheet = &self.sheets[at];
                sheet.names.iter().map(move |(name, range)| {
                    (tab, *name, format!("{}!{range}", quoted(&sheet.name)))
                })
            })
            .collect();
        if !self.names.is_empty() || !local.is_empty() {
            workbook.push_str("<definedNames>");
            for (tab, name, reference) in &local {
                let hidden = if *name == "_xlnm._FilterDatabase" {
                    " hidden=\"1\""
                } else {
                    ""
                };
                let _ = write!(
                    workbook,
                    "<definedName name=\"{name}\" localSheetId=\"{tab}\"{hidden}>"
                );
                xml::text(&mut workbook, reference);
                workbook.push_str("</definedName>");
            }
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
             <Default Extension=\"xml\" ContentType=\"application/xml\"/>",
        );
        if self.sheets.iter().any(|sheet| sheet.notes) {
            types.push_str(
                "<Default Extension=\"vml\" ContentType=\"application/vnd.openxmlformats-officedocument.vmlDrawing\"/>",
            );
        }
        for format in &self.formats {
            let _ = write!(
                types,
                "<Default Extension=\"{}\" ContentType=\"{}\"/>",
                format.extension(),
                format.content_type()
            );
        }
        types.push_str(
            "<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>",
        );
        for at in 0..count {
            let _ = write!(
                types,
                "<Override PartName=\"/xl/worksheets/sheet{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>",
                at + 1
            );
            if self.sheets[at].notes {
                let _ = write!(
                    types,
                    "<Override PartName=\"/xl/comments{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml\"/>",
                    at + 1
                );
            }
            if self.sheets[at].drawing {
                let _ = write!(
                    types,
                    "<Override PartName=\"/xl/drawings/drawing{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawing+xml\"/>",
                    at + 1
                );
            }
        }
        for id in 1..=self.tables.len() {
            let _ = write!(
                types,
                "<Override PartName=\"/xl/tables/table{id}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml\"/>"
            );
        }
        for id in 1..=self.charts {
            let _ = write!(
                types,
                "<Override PartName=\"/xl/charts/chart{id}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/>"
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
