//! The ods writer: the same calls as [`crate::Writer`], with the workbook held until
//! [`Writer::finish`] (`rule:core-classes/spreadsheet-writes-as-a-stream`).
//!
//! **The writer holds the workbook.** ods puts some parts before the first sheet and others
//! inside the cells they cover, so no part of `content.xml` can leave before the last row is
//! known. [`Writer::sheet`] and [`Writer::rows`] check each row as xlsx does and keep it, with
//! each cell's value and its cell format. [`Writer::finish`] writes the whole package and returns
//! every byte. Memory: every cell and every sheet-end part of the workbook, the image bytes
//! included, per writing request, freed at `finish`.
//!
//! **The package** is a zip with `mimetype` first and stored, as ODF requires, then
//! `content.xml`, `styles.xml`, `meta.xml`, `settings.xml`, each image as `Pictures/<n>.<ext>`,
//! each chart as `Object <n>/content.xml` and `META-INF/manifest.xml`. The sheets are written in
//! tab order. The styles of the cells are
//! automatic styles in `content.xml`:
//!
//! - **Cell styles** are `ce<n>`, one for each `cellXfs` entry [`crate::styles::Styles`] makes,
//!   so equal combinations of layers share one. `styles` writes them, and `numbers` writes each
//!   distinct format code as the number style `N<n>`.
//! - **Columns** are `co<n>` for each distinct width, written as Excel's character width turned
//!   into inches. A column's own style is its `table:default-cell-style-name`.
//! - **Rows** are `ro<n>` for each distinct height. A row's style is its default cell style.
//!   A row's outline level nests it in `table:table-row-group`s.
//! - **Sheets** are `ta<n>` for each distinct display, direction, tab colour and master page. A
//!   very hidden sheet is hidden, because ods has no very hidden sheet.
//!
//! **The header row** gets what it gets in xlsx: its style, the rows above the second frozen,
//! and a filter over the header and every row below it. The filter is the database range
//! `__Anonymous_Sheet_DB__<tab>`, the name LibreOffice gives a sheet's own filter, so a reader
//! does not take it for a table. A table over the filter's range drops the filter, as in xlsx.
//! Auto width measures the first batch, as in xlsx.
//!
//! **`settings.xml`** has each sheet's frozen panes, zoom and gridlines, in the view
//! settings LibreOffice reads. A defined name over a range is a `table:named-range`, and any
//! other is a `table:named-expression`. A formula is OpenFormula, with its cached result.
//!
//! **The sheet-end parts** are checked by the code xlsx checks them with, the sheet's
//! `crate::tail::Tail` and `crate::tail::Groups`, so each error is the one xlsx gives. `parts`
//! writes the parts that are not cells:
//!
//! - **A merged range** spans from its top-left cell, and every other cell of it is a
//!   `table:covered-table-cell`, which keeps its value if it has one.
//! - **A link** wraps the cell's text in `text:a`. A cell with no value shows the link's text, or
//!   its target. A cell link is `#Sheet.A1`.
//! - **A note** is an `office:annotation` with its author, first in its cell. **An image** is a
//!   `draw:frame` in its cell, after the note, in inches at 96 pixels an inch. **A chart** is a
//!   `draw:frame` with a `draw:object` in its cell, among the images in the order they were
//!   added, and its object is `chart`'s module doc.
//! - **A validation** is a `table:content-validation` before the first sheet, named `val<n>`
//!   across the workbook, and each cell it covers names it.
//! - **A group** adds to each row's and column's outline level as in xlsx, and nests it in a
//!   `table:table-row-group` or a `table:table-column-group`. A collapsed group's rows and
//!   columns are hidden.
//! - **The page setup** is a page layout and a master page per sheet in `styles.xml`, with its
//!   header and footer. The print area is `table:print-ranges`, and the rows to repeat are
//!   `table:table-header-rows`.
//! - **Protection** is `table:protected`, with the password as Excel's 16-bit hash. ods has no
//!   form for what a protected sheet allows, so `allow` is not written.
//! - **A table** is a `table:database-range`. Its totals row is a row of the sheet, written
//!   where xlsx writes it. Its style and banding have no ods form. A structured reference in any
//!   formula becomes the A1 range it covers when the workbook is finished.
//! - **A conditional format** is a `calcext:conditional-format` after the sheet's rows, whose
//!   style is the named cell style `Cond<n>` in `styles.xml`.
//!
//! A part may name a cell no row reached, so the rows go on to the last row a part needs. A run
//! of equal rows is one row with `table:number-rows-repeated`, and a run of equal cells one cell
//! with `table:number-columns-repeated`, so a validation down a whole column costs a few
//! elements.
//!
//! **Two writes are byte-equal**: every part is written in the order of the calls, and the
//! document dates are `writer`'s fixed ones.

mod cells;
mod chart;
mod numbers;
mod parts;
mod styles;

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::chart::Planned;
use crate::conditional::{self, Conditional};
use crate::drawing::{self, ChartPlacement, Placement};
use crate::styles::{Implicit, Styles};
use crate::table::{Placed, Table, expand};
use crate::tail::{
    Area, Groups, Link, Note, Part, Protect, Tail, Validation, area, cell_at, row_span,
};
use crate::writer::{
    MAX_COLUMNS, MAX_ROWS, check_name, check_name_sheets, check_options, check_sheet_name,
    clock_text, continued, fold_column_formats, header_layer,
};
use crate::zip::Archive;
use crate::{
    Error, Finished, Row, SheetOptions, SheetRows, Style, Value, Visibility, WorkbookOptions,
    openformula, width, xml,
};

const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
const MIME: &str = "application/vnd.oasis.opendocument.spreadsheet";
const NAMESPACES: &str = "xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
     xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" \
     xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" \
     xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" \
     xmlns:number=\"urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0\" \
     xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" \
     xmlns:of=\"urn:oasis:names:tc:opendocument:xmlns:of:1.2\" \
     xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
     xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
     xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\" \
     xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\" \
     xmlns:calcext=\"urn:org:documentfoundation:names:experimental:calc:xmlns:calcext:1.0\" \
     xmlns:tableooo=\"http://openoffice.org/2009/table\"";

/// One kept cell: its value, and its `cellXfs` entry, 0 for none.
struct Held {
    value: Option<Value>,
    xf: u32,
}

impl Held {
    /// Whether the cell is written: it has a value or a format.
    fn written(&self) -> bool {
        self.value.is_some() || self.xf != 0
    }
}

/// One kept row. Its level and its hidden flag count its groups.
struct HeldRow {
    cells: Vec<Held>,
    xf: u32,
    height: Option<f64>,
    hidden: bool,
    level: u64,
}

/// One column of a sheet, as its first batch and its groups set it.
#[derive(Clone, Copy, PartialEq)]
struct HeldColumn {
    width: Option<f64>,
    xf: u32,
    hidden: bool,
    level: u64,
}

/// A column with nothing of its own.
const PLAIN_COLUMN: HeldColumn = HeldColumn {
    width: None,
    xf: 0,
    hidden: false,
    level: 0,
};

/// One image: where it goes, its number in the workbook, its place in the sheet's drawing order
/// and its bytes.
struct HeldImage {
    placed: Placement,
    number: usize,
    z: usize,
    bytes: Vec<u8>,
}

/// One chart: where it goes, its object's number in the workbook, its place in the sheet's
/// drawing order and the chart checked.
struct HeldChart {
    placed: ChartPlacement,
    number: usize,
    z: usize,
    planned: Planned,
}

impl HeldChart {
    /// Its object's folder in the package, such as `Object 1`.
    fn folder(&self) -> String {
        format!("Object {}", self.number)
    }
}

impl HeldImage {
    /// Its entry in the package, such as `Pictures/1.png`.
    fn entry(&self) -> String {
        format!(
            "Pictures/{}.{}",
            self.number,
            self.placed.format.extension()
        )
    }
}

/// One sheet, open or ended.
struct Sheet {
    options: SheetOptions,
    header_style: Option<u64>,
    /// The column layers, by column.
    layers: Vec<Option<u64>>,
    /// The columns, set by the first batch.
    columns: Option<Vec<HeldColumn>>,
    rows: Vec<HeldRow>,
    /// Its header row, kept to repeat on the next sheet when it overflows.
    header: Option<Row>,
    base: String,
    part: u32,
    /// The number the next row gets, from 1.
    next: u32,
    /// Its data rows, the header row not counted.
    counted: u64,
    /// The checked parts, as xlsx keeps them: the merges, the tables and the page setup.
    tail: Tail,
    groups: Groups,
    /// Each link and note with its cell's column and row.
    links: Vec<(u32, u32, Link)>,
    notes: Vec<(u32, u32, Note)>,
    /// Each validation with its range and its number in the workbook.
    checks: Vec<(Area, usize, Validation)>,
    protect: Option<Protect>,
    images: Vec<HeldImage>,
    charts: Vec<HeldChart>,
    /// Each conditional format with its `dxfs` index, in priority order.
    conditionals: Vec<(Conditional, Option<u32>)>,
}

impl Sheet {
    fn new(options: SheetOptions, header_style: Option<u64>, base: String, part: u32) -> Self {
        let layers = options.columns.iter().map(|column| column.style).collect();
        Self {
            options,
            header_style,
            layers,
            columns: None,
            rows: Vec::new(),
            header: None,
            base,
            part,
            next: 1,
            counted: 0,
            tail: Tail::default(),
            groups: Groups::default(),
            links: Vec::new(),
            notes: Vec::new(),
            checks: Vec::new(),
            protect: None,
            images: Vec::new(),
            charts: Vec::new(),
            conditionals: Vec::new(),
        }
    }

    /// Whether its first rows are kept, which fixes its columns.
    fn started(&self) -> bool {
        self.columns.is_some()
    }

    /// The rows frozen at the top: at least the header row when the header freezes.
    fn frozen_rows(&self) -> u64 {
        if self.header.is_some() && self.options.header.freeze {
            self.options.freeze_rows.max(1)
        } else {
            self.options.freeze_rows
        }
    }

    /// The range its filter covers, if it has one, as `A1:C10`.
    fn filter(&self) -> Option<String> {
        if let Some(range) = &self.options.autofilter {
            return Some(range.replace('$', ""));
        }
        let header = self.header.as_ref()?;
        if !self.options.header.filter {
            return None;
        }
        let columns = header.cells.len().max(self.options.columns.len()).max(1);
        let last = self.next.saturating_sub(1).max(1);
        let range = format!("A1:{}", crate::cells::reference(columns as u32 - 1, last));
        // A table has its own filter, and xlsx writes no other over it.
        area(&range)
            .is_some_and(|at| !self.tail.overlaps_table(&at))
            .then_some(range)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Writing,
    Finished,
    Stopped,
}

/// An ods workbook being written.
pub struct Writer {
    options: WorkbookOptions,
    styles: Styles,
    sheets: Vec<Sheet>,
    /// Whether the last sheet is still open.
    open: bool,
    names: Vec<(String, String)>,
    /// The tables' names, in the order they were added.
    tables: Vec<String>,
    /// The images, the charts and the validations added so far.
    media: usize,
    objects: usize,
    checks: usize,
    state: State,
}

/// `name` as the sheet of an ods address, quoted when it is not one word.
fn sheet_ref(name: &str) -> String {
    let plain = name.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !name.starts_with(|c: char| c.is_ascii_digit());
    if plain {
        name.to_owned()
    } else {
        format!("'{}'", name.replace('\'', "''"))
    }
}

/// A width of `chars` characters, Excel's unit, in inches: 7 pixels a character and 5 of
/// padding, at 96 pixels an inch.
fn inches(chars: f64) -> String {
    format!("{:.4}in", (chars * 7.0 + 5.0) / 96.0)
}

/// `pixels` at 96 an inch, in inches.
fn pixel_inches(pixels: f64) -> String {
    format!("{:.4}in", pixels / 96.0)
}

/// The width Excel gives a column of its default 8.43 characters.
const DEFAULT_WIDTH: f64 = 8.43;

/// Names automatic styles by their written properties, in the order of first use.
#[derive(Default)]
struct Named {
    order: Vec<String>,
    ids: HashMap<String, usize>,
}

impl Named {
    /// The number of `key`, from 1.
    fn id(&mut self, key: String) -> usize {
        if let Some(&id) = self.ids.get(&key) {
            return id;
        }
        self.order.push(key.clone());
        let id = self.order.len();
        self.ids.insert(key, id);
        id
    }
}

/// The page layouts in `styles.xml`, and each sheet with a page setup by its index, with its
/// layout's number. The sheet's master page is `mp<n>`, `n` counting this list from 1.
#[derive(Default)]
struct Pages {
    layouts: Named,
    masters: Vec<(usize, usize)>,
}

impl Pages {
    /// The master page of the sheet at `at`, if it has a page setup.
    fn master(&self, at: usize) -> Option<String> {
        self.masters
            .iter()
            .position(|&(sheet, _)| sheet == at)
            .map(|place| format!("mp{}", place + 1))
    }
}

impl Writer {
    /// A workbook with nothing written yet.
    pub fn open(options: WorkbookOptions) -> Result<Self, Error> {
        if let Some(created) = &options.properties.created {
            clock_text(created)?;
        }
        Ok(Self {
            options,
            styles: Styles::default(),
            sheets: Vec::new(),
            open: false,
            names: Vec::new(),
            tables: Vec::new(),
            media: 0,
            objects: 0,
            checks: 0,
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

    /// Ends the open sheet, if any, opens the next one and keeps its first rows.
    pub fn sheet(&mut self, mut options: SheetOptions, first: &[Row]) -> Result<(), Error> {
        self.writing()?;
        let taken = self.sheets.iter().map(|sheet| sheet.options.name.as_str());
        check_options(&options, taken, &self.styles)?;
        fold_column_formats(&mut self.styles, &mut options)?;
        let header_style = header_layer(&mut self.styles, &options)?;
        self.ready_to_end()?;
        let base = options.name.clone();
        self.sheets.push(Sheet::new(options, header_style, base, 1));
        self.open = true;
        self.rows(first)
    }

    /// Adds a sheet-end part to the open sheet, checked as xlsx checks it.
    pub fn part(&mut self, part: Part) -> Result<(), Error> {
        self.writing()?;
        if !self.open {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you add to it.".to_owned(),
            ));
        }
        let dxf = match &part {
            Part::Table(_) => {
                let Part::Table(table) = part else {
                    unreachable!("the part is a table");
                };
                return self.table(table);
            }
            Part::Conditional(rule) => {
                conditional::check(rule)?;
                rule.style
                    .clone()
                    .map(|style| self.styles.dxf(style))
                    .transpose()?
            }
            _ => None,
        };
        let (media, object, check) = (self.media + 1, self.objects + 1, self.checks + 1);
        let Some(sheet) = self.sheets.last_mut() else {
            unreachable!("an open sheet is the last one");
        };
        let name = sheet.options.name.clone();
        match part {
            Part::Group(group) => {
                sheet
                    .groups
                    .add(&group, &name, sheet.next, sheet.columns.is_some())?;
            }
            Part::Print(print) => {
                if print.fits() && sheet.started() {
                    return Err(Error::Invalid(format!(
                        "The sheet `{name}` cannot be fitted to pages after its first row. Set the fit before you add rows."
                    )));
                }
                sheet.tail.add(Part::Print(print), &name)?;
            }
            Part::Image(image) => {
                let placed = drawing::place(&image)?;
                let z = sheet.images.len() + sheet.charts.len();
                sheet.images.push(HeldImage {
                    placed,
                    number: media,
                    z,
                    bytes: image.image,
                });
                self.media = media;
            }
            Part::Chart(chart) => {
                let first = if sheet.header.is_some() { 2 } else { 1 };
                let reach = sheet.next.saturating_sub(1).max(first);
                let (placed, planned) = drawing::place_chart(&chart, &name, reach)?;
                let z = sheet.images.len() + sheet.charts.len();
                sheet.charts.push(HeldChart {
                    placed,
                    number: object,
                    z,
                    planned,
                });
                self.objects = object;
            }
            Part::Conditional(rule) => sheet.conditionals.push((rule, dxf)),
            Part::Link(link) => {
                sheet.tail.add(Part::Link(link.clone()), &name)?;
                if let Some(at) = area(&link.anchor) {
                    sheet.links.push((at.left, at.top, link));
                }
            }
            Part::Note(note) => {
                sheet.tail.add(Part::Note(note.clone()), &name)?;
                if let Some((column, row)) = cell_at(&note.noted) {
                    sheet.notes.push((column, row, note));
                }
            }
            Part::Validation(rule) => {
                sheet.tail.add(Part::Validation(rule.clone()), &name)?;
                if let Some(at) = area(&rule.validated) {
                    sheet.checks.push((at, check, rule));
                    self.checks = check;
                }
            }
            Part::Protect(protect) => {
                sheet.tail.add(Part::Protect(protect.clone()), &name)?;
                sheet.protect = Some(protect);
            }
            part => sheet.tail.add(part, &name)?,
        }
        Ok(())
    }

    /// Adds `table` to the open sheet, and keeps its totals row when it has totals, as xlsx
    /// does (`table`'s module doc).
    fn table(&mut self, table: Table) -> Result<(), Error> {
        let id = self.tables.len() + 1;
        let Some(sheet) = self.sheets.last() else {
            unreachable!("an open sheet is the last one");
        };
        let name = sheet.options.name.as_str();
        let mut placed = Placed::new(table, id, name)?;
        let lower = placed.name.to_lowercase();
        if self.tables.iter().any(|n| n.to_lowercase() == lower)
            || self.names.iter().any(|(n, _)| n.to_lowercase() == lower)
        {
            return Err(Error::Invalid(format!(
                "The name `{}` is used twice. A table's name is different from every other table and name of the workbook.",
                placed.name
            )));
        }
        if let Some(range) = &sheet.options.autofilter
            && area(range).is_some_and(|filter| filter.overlaps(&placed.area))
        {
            return Err(Error::Invalid(format!(
                "The table `{}` overlaps the autofilter `{range}` of the sheet `{name}`. A table has its own filter.",
                placed.name
            )));
        }
        sheet.tail.check_table(&placed, name)?;
        if placed.area.top < sheet.next {
            match &sheet.header {
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
            if placed.area.bottom + 1 < sheet.next {
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
            if placed.area.bottom + 1 == sheet.next {
                // The header is named, so this checks each total names a column.
                placed.totals_row()?;
            }
        }
        let table_name = placed.name.clone();
        let name = name.to_owned();
        let Some(sheet) = self.sheets.last_mut() else {
            unreachable!("an open sheet is the last one");
        };
        sheet.tail.table(placed, &name)?;
        if let Err(error) = self.append(&[]) {
            if let Some(sheet) = self.sheets.last_mut() {
                sheet.tail.forget_table(id);
            }
            return Err(error);
        }
        self.tables.push(table_name);
        Ok(())
    }

    /// Checks that the open sheet can end: every table's header row and totals row is kept.
    fn ready_to_end(&self) -> Result<(), Error> {
        if let Some(sheet) = self.open_sheet()
            && let Some(table) = sheet.tail.unfinished_table()
        {
            return Err(Error::Invalid(format!(
                "The table `{}` covers rows {} to {}, and the sheet `{}` ends at row {}. Write the table's rows before the sheet ends.",
                table.name,
                table.area.top,
                table.area.bottom,
                sheet.options.name,
                sheet.next - 1
            )));
        }
        Ok(())
    }

    /// The open sheet's data range in the column at `column`, from 0, over the rows kept so
    /// far, such as `C2:C1043`. The header row is not in it, and a sheet with no data row yet is
    /// an error (`writer::data_range`).
    pub fn column(&self, column: u64) -> Result<String, Error> {
        let Some(sheet) = self.open_sheet() else {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you ask for a column.".to_owned(),
            ));
        };
        crate::writer::data_range(
            &sheet.options.name,
            sheet.header.is_some(),
            sheet.next,
            column,
        )
    }

    fn open_sheet(&self) -> Option<&Sheet> {
        if self.open { self.sheets.last() } else { None }
    }

    /// Keeps a batch of rows in the open sheet.
    pub fn rows(&mut self, rows: &[Row]) -> Result<(), Error> {
        if self.state == State::Finished
            && let Some(last) = self.sheets.last()
        {
            return Err(Error::Invalid(format!(
                "The sheet `{}` is finished, because the workbook is finished. `finish()` is the last call on a workbook.",
                last.options.name
            )));
        }
        self.writing()?;
        self.append(rows)
    }

    /// Keeps `rows` in the open sheet, with a table's totals row after the table's last row, and
    /// goes on to the next sheet at the end of the grid when the sheet's options say so.
    fn append(&mut self, rows: &[Row]) -> Result<(), Error> {
        let Some(sheet) = self.open_sheet() else {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you add rows.".to_owned(),
            ));
        };
        // A table's totals row goes right after its last row.
        if let Some((id, totals)) = sheet.tail.totals(sheet.next) {
            self.keep(&[totals?])?;
            if let Some(sheet) = self.sheets.last_mut() {
                sheet.tail.totaled(id);
            }
            return self.append(rows);
        }
        if let Some(cut) = sheet.tail.totals_due(sheet.next, rows.len())
            && cut < rows.len()
        {
            self.append(&rows[..cut])?;
            return self.append(&rows[cut..]);
        }
        let room = (MAX_ROWS + 1).saturating_sub(sheet.next) as usize;
        if rows.len() > room && sheet.options.new_sheet_on_overflow {
            self.append(&rows[..room])?;
            self.overflow()?;
            return self.append(&rows[room..]);
        }
        self.keep(rows)?;
        let due = self
            .open_sheet()
            .is_some_and(|sheet| sheet.tail.totals(sheet.next).is_some());
        if due && !rows.is_empty() {
            return self.append(&[]);
        }
        Ok(())
    }

    /// Ends the open sheet, which is full, and opens the next sheet of its overflow with the same
    /// options and its header row.
    fn overflow(&mut self) -> Result<(), Error> {
        let Some(sheet) = self.sheets.last() else {
            return Ok(());
        };
        let mut options = sheet.options.clone();
        let (base, header, header_style) =
            (sheet.base.clone(), sheet.header.clone(), sheet.header_style);
        let mut part = sheet.part + 1;
        options.name = continued(&base, part);
        let taken = |sheets: &[Sheet], name: &str| {
            check_sheet_name(name, sheets.iter().map(|sheet| sheet.options.name.as_str())).is_err()
        };
        while taken(&self.sheets, &options.name) {
            part += 1;
            options.name = continued(&base, part);
        }
        options.position = None;
        self.sheets
            .push(Sheet::new(options, header_style, base, part));
        match header {
            Some(header) => self.append(&[header]),
            None => Ok(()),
        }
    }

    /// Checks `rows` and keeps them in the open sheet, all of them or none.
    fn keep(&mut self, rows: &[Row]) -> Result<(), Error> {
        let Some(first) = rows.first() else {
            return Ok(());
        };
        let Some(sheet) = self.sheets.last() else {
            unreachable!("an open sheet is the last one");
        };
        let header = (sheet.next == 1 && first.header).then(|| first.clone());
        let columns = match &sheet.columns {
            Some(_) => None,
            None => Some(columns(&mut self.styles, sheet, header.is_some(), rows)?),
        };
        let styles = &mut self.styles;
        let name = sheet.options.name.as_str();
        let start = sheet.next;
        let mut number = start;
        let mut kept = Vec::with_capacity(rows.len());
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
            if row.cells.len() > MAX_COLUMNS {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` has {} cells. A row has at most 16,384 cells.",
                    row.cells.len()
                )));
            }
            if let Some(height) = row.height
                && !(0.0..=409.0).contains(&height)
            {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` has the height {height}. A row height is from 0 to 409 points."
                )));
            }
            let outline = sheet.groups.row(number);
            let level = row.level + outline.level;
            if level > 7 {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` has the outline level {level}, counting its groups. The level is from 0 to 7."
                )));
            }
            let style = if row.header {
                sheet.header_style.or(row.style)
            } else {
                row.style
            };
            let row_xf = match style {
                Some(style) => styles.xf(None, Some(style), None, Implicit::None)?,
                None => 0,
            };
            let mut cells = Vec::with_capacity(row.cells.len());
            for (column, cell) in row.cells.iter().enumerate() {
                let implicit = match &cell.value {
                    Some(value) => crate::cells::check(column as u32, number, value)?,
                    None => Implicit::None,
                };
                if let Some(Value::Rich(runs)) = &cell.value {
                    for run in runs {
                        if let Some(id) = run.style {
                            styles.get(id)?;
                        }
                    }
                }
                let layer = sheet.layers.get(column).copied().flatten();
                let xf = if cell.value.is_none() && cell.style.is_none() {
                    0
                } else {
                    styles.xf(layer, style, cell.style, implicit)?
                };
                // A formula in a table's rows has its structured references qualified, as
                // xlsx writes it.
                let value = match &cell.value {
                    Some(Value::Formula {
                        formula,
                        cached_text,
                        cached_number,
                    }) if formula.contains('[') => match sheet.tail.table_at(column as u32, number)
                    {
                        Some(table) => Some(Value::Formula {
                            formula: expand(formula, table),
                            cached_text: cached_text.clone(),
                            cached_number: *cached_number,
                        }),
                        None => cell.value.clone(),
                    },
                    value => value.clone(),
                };
                cells.push(Held { value, xf });
            }
            kept.push(HeldRow {
                cells,
                xf: row_xf,
                height: row.height,
                hidden: row.hidden || outline.hidden,
                level,
            });
            number += 1;
        }
        let Some(sheet) = self.sheets.last_mut() else {
            unreachable!("an open sheet is the last one");
        };
        sheet.tail.headers(rows, start)?;
        if let Some(columns) = columns {
            sheet.columns = Some(columns);
        }
        let counted = rows.len() - usize::from(header.is_some());
        if header.is_some() {
            sheet.header = header;
        }
        sheet.rows.extend(kept);
        sheet.next = number;
        sheet.counted += counted as u64;
        Ok(())
    }
}

/// The columns of `sheet`, measuring `first` for auto width. `header` says whether `first`
/// starts with the header row.
fn columns(
    styles: &mut Styles,
    sheet: &Sheet,
    header: bool,
    first: &[Row],
) -> Result<Vec<HeldColumn>, Error> {
    let options = &sheet.options;
    let count = if header {
        let widest = first.iter().map(|row| row.cells.len()).max().unwrap_or(0);
        widest.max(options.columns.len())
    } else {
        options.columns.len()
    };
    let mut out = Vec::with_capacity(count);
    for at in 0..count {
        let column = options.columns.get(at);
        let layer = column.and_then(|column| column.style);
        let xf = match layer {
            Some(style) => styles.xf(Some(style), None, None, Implicit::None)?,
            None => 0,
        };
        let width = match column.and_then(|column| column.width) {
            Some(width) => Some(width),
            None if column.map_or(header, |column| column.auto) => {
                let styles = &*styles;
                let widest = first
                    .iter()
                    .filter_map(|row| {
                        let cell = row.cells.get(at)?;
                        let style = if row.header {
                            sheet.header_style.or(row.style)
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
        out.push(HeldColumn {
            width,
            xf,
            hidden: column.is_some_and(|column| column.hidden),
            level: 0,
        });
    }
    Ok(out)
}

/// What the parts put in one cell: a link, a note and images, each image with its place in the
/// sheet's drawing order.
struct Spot<'a> {
    row: u32,
    column: u32,
    link: Option<&'a Link>,
    note: Option<&'a Note>,
    images: Vec<&'a HeldImage>,
    charts: Vec<&'a HeldChart>,
    /// The row each growing chart range ends at: the sheet's last row.
    last: u32,
}

impl Spot<'_> {
    /// What the cell has before its text: the note, then each image's and chart's frame, in
    /// the order they were added.
    fn lead(&self) -> String {
        let mut out = String::new();
        if let Some(note) = self.note {
            out.push_str("<office:annotation>");
            if let Some(author) = &note.author {
                out.push_str("<dc:creator>");
                xml::odf(&mut out, author, false);
                out.push_str("</dc:creator>");
            }
            cells::paragraphs(&mut out, &note.note);
            out.push_str("</office:annotation>");
        }
        let mut frames: Vec<(usize, String)> = Vec::new();
        for image in &self.images {
            let placed = &image.placed;
            frames.push((
                image.z,
                format!(
                    "<draw:frame draw:z-index=\"{}\" draw:name=\"Image {}\" svg:width=\"{}\" svg:height=\"{}\" svg:x=\"{}\" svg:y=\"{}\">\
                     <draw:image xlink:href=\"{}\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/></draw:frame>",
                    image.z,
                    image.number,
                    pixel_inches(placed.width),
                    pixel_inches(placed.height),
                    pixel_inches(placed.offset_x),
                    pixel_inches(placed.offset_y),
                    image.entry()
                ),
            ));
        }
        for held in &self.charts {
            let placed = &held.placed;
            let mut frame = format!(
                "<draw:frame draw:z-index=\"{}\" draw:name=\"Chart {}\" svg:width=\"{}\" svg:height=\"{}\" svg:x=\"0in\" svg:y=\"0in\">\
                 <draw:object draw:notify-on-update-of-ranges=\"",
                held.z,
                held.number,
                pixel_inches(placed.width),
                pixel_inches(placed.height),
            );
            xml::odf(
                &mut frame,
                &chart::ranges(&held.planned, self.last).join(" "),
                true,
            );
            frame.push_str("\" xlink:href=\"./");
            xml::odf(&mut frame, &held.folder(), true);
            frame.push_str(
                "\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/></draw:frame>",
            );
            frames.push((held.z, frame));
        }
        frames.sort_by_key(|(z, _)| *z);
        for (_, frame) in frames {
            out.push_str(&frame);
        }
        out
    }
}

/// The opening `text:a` of `link`: a URL as it is, and a cell such as `'Sheet 2'!A1` as
/// `#'Sheet 2'.A1`.
fn link_open(link: &Link) -> String {
    let target = link.target.as_str();
    let lower = target.to_ascii_lowercase();
    let external = ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme));
    let href = match target.rsplit_once('!') {
        Some((sheet, cell)) if !external => format!("#{sheet}.{cell}"),
        _ => target.to_owned(),
    };
    let mut out = String::from("<text:a xlink:href=\"");
    xml::odf(&mut out, &href, true);
    out.push_str("\" xlink:type=\"simple\"");
    if let Some(tooltip) = &link.tooltip {
        out.push_str(" office:title=\"");
        xml::odf(&mut out, tooltip, true);
        out.push('"');
    }
    out.push('>');
    out
}

/// What one sheet's rows are written with: the workbook's tables, the cell styles of rich runs,
/// and the automatic styles the rows name.
struct Rows<'a, 'b> {
    tables: &'a [(&'a str, &'a Placed)],
    row_styles: &'b mut Named,
    run: &'b mut dyn FnMut(u64) -> String,
}

impl Writer {
    /// The sheets' indexes in tab order.
    fn tab_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.sheets.len()).collect();
        order.sort_by_key(|&at| (self.sheets[at].options.position.unwrap_or(u64::MAX), at));
        order
    }

    /// Ends the open sheet and the workbook, and writes the whole package. Returns its bytes,
    /// the rows of each sheet and the size of the file.
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
            .copied()
            .find(|&at| self.sheets[at].options.visibility == Visibility::Visible)
        else {
            return Err(Error::Invalid(
                "Every sheet of the workbook is hidden. At least one sheet must be visible."
                    .to_owned(),
            ));
        };
        check_name_sheets(
            &self.names,
            self.sheets.iter().map(|sheet| sheet.options.name.as_str()),
        )?;
        self.ready_to_end()?;
        let pages = self.pages(&order);
        let content = self.content(&order, &pages)?;
        let parts = [
            ("content.xml", content),
            ("styles.xml", self.styles_part(&pages)?),
            ("meta.xml", self.meta()?),
            ("settings.xml", self.settings(&order, active)),
        ];
        let images: Vec<&HeldImage> = self
            .sheets
            .iter()
            .flat_map(|sheet| sheet.images.iter())
            .collect();
        let objects: Vec<(String, String)> = self
            .sheets
            .iter()
            .flat_map(|sheet| {
                let last = sheet.next.saturating_sub(1);
                sheet.charts.iter().map(move |held| {
                    let placed = &held.placed;
                    (
                        held.folder(),
                        chart::object(&held.planned, placed.width, placed.height, last),
                    )
                })
            })
            .collect();
        let manifest = manifest(&images, &objects);
        let mut archive = Archive::new();
        let written = (|| {
            archive.stored("mimetype", MIME.as_bytes())?;
            for (name, text) in &parts {
                archive.begin(name)?;
                archive.write(text.as_bytes())?;
            }
            for image in &images {
                archive.begin(&image.entry())?;
                archive.write(&image.bytes)?;
            }
            for (folder, content) in &objects {
                archive.begin(&format!("{folder}/content.xml"))?;
                archive.write(content.as_bytes())?;
            }
            archive.begin("META-INF/manifest.xml")?;
            archive.write(manifest.as_bytes())?;
            archive.close()
        })();
        if let Err(error) = written {
            self.state = State::Stopped;
            return Err(error);
        }
        self.state = State::Finished;
        self.open = false;
        let data = archive.take();
        let rows = self
            .sheets
            .iter()
            .map(|sheet| SheetRows {
                name: sheet.options.name.clone(),
                rows: sheet.counted,
            })
            .collect();
        // The workbook is written, so its cells and images go now, not when the request ends.
        self.sheets.iter_mut().for_each(|sheet| {
            sheet.rows = Vec::new();
            sheet.images = Vec::new();
            sheet.charts = Vec::new();
        });
        Ok(Finished {
            size: data.len() as u64,
            data,
            rows,
        })
    }

    /// The page layouts and master pages of the sheets with a page setup, in tab order.
    fn pages(&self, order: &[usize]) -> Pages {
        let mut pages = Pages::default();
        for &at in order {
            if let Some(print) = self.sheets[at].tail.print() {
                let layout = pages.layouts.id(parts::page_layout(print));
                pages.masters.push((at, layout));
            }
        }
        pages
    }

    /// `content.xml`: the automatic styles, the validations, the sheets in tab order, the names,
    /// the filters and the tables.
    fn content(&self, order: &[usize], pages: &Pages) -> Result<String, Error> {
        let mut numbers = Named::default();
        let mut cell_styles = String::new();
        for xf in 1..=self.styles.xf_count() {
            let style = self.styles.xf_style(xf);
            let data = style
                .format
                .as_deref()
                .map(|code| format!("N{}", numbers.id(code.to_owned())));
            styles::cell(
                &mut cell_styles,
                &format!("ce{xf}"),
                &style,
                data.as_deref(),
            )?;
        }
        let tables: Vec<(&str, &Placed)> = self
            .sheets
            .iter()
            .flat_map(|sheet| {
                sheet
                    .tail
                    .placed()
                    .iter()
                    .map(move |table| (sheet.options.name.as_str(), table))
            })
            .collect();
        let mut column_styles = Named::default();
        let mut row_styles = Named::default();
        let mut table_styles = Named::default();
        let mut runs: HashMap<u64, String> = HashMap::new();
        let mut run_order: Vec<u64> = Vec::new();
        let mut body = String::new();
        let mut run = |id| {
            let next = runs.len() + 1;
            runs.entry(id)
                .or_insert_with(|| {
                    run_order.push(id);
                    format!("T{next}")
                })
                .clone()
        };
        for &at in order {
            let mut rows = Rows {
                tables: &tables,
                row_styles: &mut row_styles,
                run: &mut run,
            };
            self.table_xml(
                &mut body,
                &self.sheets[at],
                &mut column_styles,
                &mut table_styles,
                pages.master(at),
                &mut rows,
            );
        }
        let mut out = String::from(HEAD);
        let _ = write!(
            out,
            "<office:document-content {NAMESPACES} office:version=\"1.3\"><office:automatic-styles>"
        );
        for (at, code) in numbers.order.iter().enumerate() {
            numbers::write(&mut out, &format!("N{}", at + 1), code);
        }
        for (at, width) in column_styles.order.iter().enumerate() {
            let _ = write!(
                out,
                "<style:style style:name=\"co{}\" style:family=\"table-column\"><style:table-column-properties fo:break-before=\"auto\" style:column-width=\"{width}\"/></style:style>",
                at + 1
            );
        }
        for (at, height) in row_styles.order.iter().enumerate() {
            let _ = write!(
                out,
                "<style:style style:name=\"ro{}\" style:family=\"table-row\"><style:table-row-properties style:row-height=\"{height}pt\" style:use-optimal-row-height=\"false\"/></style:style>",
                at + 1
            );
        }
        for (at, key) in table_styles.order.iter().enumerate() {
            let (master, props) = key.split_once('|').unwrap_or(("", key));
            let _ = write!(
                out,
                "<style:style style:name=\"ta{}\" style:family=\"table\"{master}><style:table-properties{props}/></style:style>",
                at + 1
            );
        }
        out.push_str(&cell_styles);
        for id in &run_order {
            styles::run(&mut out, &runs[id], self.styles.get(*id)?)?;
        }
        out.push_str("</office:automatic-styles><office:body><office:spreadsheet>");
        let mut checks = String::new();
        for &at in order {
            let sheet = &self.sheets[at];
            for (range, id, rule) in &sheet.checks {
                parts::validation(
                    &mut checks,
                    &format!("val{id}"),
                    &sheet.options.name,
                    range,
                    rule,
                );
            }
        }
        if !checks.is_empty() {
            let _ = write!(
                out,
                "<table:content-validations>{checks}</table:content-validations>"
            );
        }
        out.push_str(&body);
        self.named(&mut out, order);
        let mut ranges = String::new();
        for (tab, &at) in order.iter().enumerate() {
            let sheet = &self.sheets[at];
            if let Some(range) = sheet.filter() {
                let name = sheet_ref(&sheet.options.name);
                let (from, to) = range.split_once(':').unwrap_or((&range, &range));
                let _ = write!(
                    ranges,
                    "<table:database-range table:name=\"__Anonymous_Sheet_DB__{tab}\" table:target-range-address=\""
                );
                xml::odf(&mut ranges, &format!("{name}.{from}:{name}.{to}"), true);
                ranges.push_str("\" table:display-filter-buttons=\"true\"/>");
            }
            for table in sheet.tail.placed() {
                ranges.push_str("<table:database-range table:name=\"");
                xml::odf(&mut ranges, &table.name, true);
                ranges.push_str("\" table:target-range-address=\"");
                xml::odf(
                    &mut ranges,
                    &parts::range(&sheet.options.name, &table.area),
                    true,
                );
                let _ = write!(
                    ranges,
                    "\" table:contains-header=\"true\" table:display-filter-buttons=\"{}\"/>",
                    table.filter()
                );
            }
        }
        if !ranges.is_empty() {
            let _ = write!(
                out,
                "<table:database-ranges>{ranges}</table:database-ranges>"
            );
        }
        out.push_str("</office:spreadsheet></office:body></office:document-content>");
        Ok(out)
    }

    /// Appends `table:named-expressions` for the defined names, if there are any.
    fn named(&self, out: &mut String, order: &[usize]) {
        if self.names.is_empty() {
            return;
        }
        let base = format!("${}.$A$1", sheet_ref(&self.sheets[order[0]].options.name));
        out.push_str("<table:named-expressions>");
        for (name, reference) in &self.names {
            let formula = openformula::from_a1(reference);
            let range = formula
                .strip_prefix("of:=[")
                .and_then(|rest| rest.strip_suffix(']'))
                .filter(|inner| !inner.contains(['[', ']', ';', '(', '~']));
            out.push_str("<table:");
            match range {
                Some(range) => {
                    out.push_str("named-range table:name=\"");
                    xml::odf(out, name, true);
                    out.push_str("\" table:base-cell-address=\"");
                    xml::odf(out, &base, true);
                    out.push_str("\" table:cell-range-address=\"");
                    xml::odf(out, range, true);
                }
                None => {
                    out.push_str("named-expression table:name=\"");
                    xml::odf(out, name, true);
                    out.push_str("\" table:base-cell-address=\"");
                    xml::odf(out, &base, true);
                    out.push_str("\" table:expression=\"");
                    xml::odf(out, &formula, true);
                }
            }
            out.push_str("\"/>");
        }
        out.push_str("</table:named-expressions>");
    }

    /// Appends one sheet as `table:table`, its page setup on the master page `master`.
    fn table_xml(
        &self,
        out: &mut String,
        sheet: &Sheet,
        column_styles: &mut Named,
        table_styles: &mut Named,
        master: Option<String>,
        rows: &mut Rows<'_, '_>,
    ) {
        let options = &sheet.options;
        let mut props = String::new();
        let shown = options.visibility == Visibility::Visible;
        let _ = write!(props, " table:display=\"{shown}\"");
        let mode = if options.right_to_left {
            "rl-tb"
        } else {
            "lr-tb"
        };
        let _ = write!(props, " style:writing-mode=\"{mode}\"");
        if let Some(color) = &options.tab_color {
            let _ = write!(
                props,
                " tableooo:tab-color=\"{}\"",
                color.to_ascii_lowercase()
            );
        }
        let master = master
            .map(|name| format!(" style:master-page-name=\"{name}\""))
            .unwrap_or_default();
        let table_style = table_styles.id(format!("{master}|{props}"));
        out.push_str("<table:table table:name=\"");
        xml::odf(out, &options.name, true);
        let _ = write!(out, "\" table:style-name=\"ta{table_style}\"");
        if let Some(protect) = &sheet.protect {
            out.push_str(" table:protected=\"true\"");
            if let Some(password) = protect.password.as_deref().filter(|p| !p.is_empty()) {
                let _ = write!(
                    out,
                    " table:protection-key=\"{}\" table:protection-key-digest-algorithm=\"{}\"",
                    parts::protection_key(password),
                    parts::LEGACY_HASH
                );
            }
        }
        if let Some(range) = sheet
            .tail
            .print()
            .and_then(|print| print.area.as_deref())
            .and_then(area)
        {
            out.push_str(" table:print-ranges=\"");
            xml::odf(out, &parts::range(&options.name, &range), true);
            out.push('"');
        }
        out.push('>');
        let spots = spots(sheet);
        let reach = sheet
            .tail
            .merges()
            .iter()
            .map(|at| at.right as usize + 1)
            .chain(sheet.checks.iter().map(|(at, ..)| at.right as usize + 1))
            .chain(spots.iter().map(|spot| spot.column as usize + 1))
            .max()
            .unwrap_or(0);
        let widest = sheet
            .rows
            .iter()
            .map(|row| row.cells.len())
            .max()
            .unwrap_or(0)
            .max(sheet.columns.as_ref().map_or(0, Vec::len))
            .max(sheet.groups.columns_reached())
            .max(reach)
            .clamp(1, MAX_COLUMNS);
        write_columns(out, sheet, widest, column_styles);
        write_rows(out, sheet, &spots, rows);
        if !sheet.conditionals.is_empty() {
            out.push_str("<calcext:conditional-formats>");
            for (rule, dxf) in &sheet.conditionals {
                let style = dxf.map(|dxf| format!("Cond{}", dxf + 1));
                parts::conditional(out, &options.name, rule, style.as_deref());
            }
            out.push_str("</calcext:conditional-formats>");
        }
        out.push_str("</table:table>");
    }

    /// `styles.xml`: the default cell style, Excel's default font, the conditional formats'
    /// named styles, and the page layouts and master pages of `pages`.
    fn styles_part(&self, pages: &Pages) -> Result<String, Error> {
        let mut numbers = Named::default();
        let mut named = String::new();
        for dxf in 0..self.styles.dxf_count() {
            let Some(style) = self.styles.dxf_style(dxf) else {
                continue;
            };
            let data = style
                .format
                .as_deref()
                .map(|code| format!("CN{}", numbers.id(code.to_owned())));
            styles::cell(
                &mut named,
                &format!("Cond{}", dxf + 1),
                style,
                data.as_deref(),
            )?;
        }
        let mut out = String::from(HEAD);
        let _ = write!(
            out,
            "<office:document-styles {NAMESPACES} office:version=\"1.3\"><office:styles>\
             <style:default-style style:family=\"table-cell\"><style:text-properties fo:font-family=\"Calibri\" fo:font-size=\"11pt\"/></style:default-style>\
             <style:style style:name=\"Default\" style:family=\"table-cell\"/>"
        );
        for (at, code) in numbers.order.iter().enumerate() {
            numbers::write(&mut out, &format!("CN{}", at + 1), code);
        }
        out.push_str(&named);
        out.push_str("</office:styles>");
        if !pages.masters.is_empty() {
            out.push_str("<office:automatic-styles>");
            for (at, props) in pages.layouts.order.iter().enumerate() {
                let _ = write!(
                    out,
                    "<style:page-layout style:name=\"pm{}\"><style:page-layout-properties{props}/></style:page-layout>",
                    at + 1
                );
            }
            out.push_str("</office:automatic-styles><office:master-styles>");
            for (place, &(at, layout)) in pages.masters.iter().enumerate() {
                if let Some(print) = self.sheets[at].tail.print() {
                    parts::master_page(&mut out, &format!("mp{}", place + 1), layout, print);
                }
            }
            out.push_str("</office:master-styles>");
        }
        out.push_str("</office:document-styles>");
        Ok(out)
    }

    /// `meta.xml`: the document properties, with `writer`'s fixed date when none is set.
    fn meta(&self) -> Result<String, Error> {
        let properties = &self.options.properties;
        let created = match &properties.created {
            Some(clock) => clock_text(clock)?,
            None => "1980-01-01T00:00:00Z".to_owned(),
        };
        let mut out = String::from(HEAD);
        out.push_str(
            "<office:document-meta xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
             xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\" \
             xmlns:dc=\"http://purl.org/dc/elements/1.1/\" office:version=\"1.3\">\
             <office:meta><meta:generator>Novis</meta:generator>",
        );
        for (tag, value) in [
            ("dc:title", &properties.title),
            ("dc:subject", &properties.subject),
            ("meta:keyword", &properties.keywords),
            ("meta:initial-creator", &properties.author),
            ("dc:creator", &properties.author),
        ] {
            if let Some(value) = value {
                let _ = write!(out, "<{tag}>");
                xml::odf(&mut out, value, false);
                let _ = write!(out, "</{tag}>");
            }
        }
        let _ = write!(
            out,
            "<meta:creation-date>{created}</meta:creation-date><dc:date>{created}</dc:date>"
        );
        if let Some(company) = &properties.company {
            out.push_str("<meta:user-defined meta:name=\"Company\">");
            xml::odf(&mut out, company, false);
            out.push_str("</meta:user-defined>");
        }
        out.push_str("</office:meta></office:document-meta>");
        Ok(out)
    }

    /// `settings.xml`: each sheet's frozen panes, zoom and gridlines, and the sheet shown first.
    fn settings(&self, order: &[usize], active: usize) -> String {
        let item = |out: &mut String, name: &str, kind: &str, value: &str| {
            let _ = write!(
                out,
                "<config:config-item config:name=\"{name}\" config:type=\"{kind}\">{value}</config:config-item>"
            );
        };
        let mut tables = String::new();
        for &at in order {
            let sheet = &self.sheets[at];
            let options = &sheet.options;
            tables.push_str("<config:config-item-map-entry config:name=\"");
            xml::odf(&mut tables, &options.name, true);
            tables.push_str("\">");
            let rows = sheet.frozen_rows().to_string();
            let columns = options.freeze_columns.to_string();
            let split = |n: &str| if n == "0" { "0" } else { "2" };
            let range = match (rows != "0", columns != "0") {
                (true, true) => "3",
                (true, false) => "2",
                (false, true) => "1",
                (false, false) => "2",
            };
            item(&mut tables, "HorizontalSplitMode", "short", split(&columns));
            item(&mut tables, "VerticalSplitMode", "short", split(&rows));
            item(&mut tables, "HorizontalSplitPosition", "int", &columns);
            item(&mut tables, "VerticalSplitPosition", "int", &rows);
            item(&mut tables, "ActiveSplitRange", "short", range);
            item(&mut tables, "PositionLeft", "int", "0");
            item(&mut tables, "PositionRight", "int", &columns);
            item(&mut tables, "PositionTop", "int", "0");
            item(&mut tables, "PositionBottom", "int", &rows);
            item(&mut tables, "ZoomType", "short", "0");
            let zoom = options.zoom.unwrap_or(100).to_string();
            item(&mut tables, "ZoomValue", "int", &zoom);
            let grid = if options.gridlines { "true" } else { "false" };
            item(&mut tables, "ShowGrid", "boolean", grid);
            tables.push_str("</config:config-item-map-entry>");
        }
        let mut out = String::from(HEAD);
        out.push_str(
            "<office:document-settings xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
             xmlns:config=\"urn:oasis:names:tc:opendocument:xmlns:config:1.0\" office:version=\"1.3\">\
             <office:settings><config:config-item-set config:name=\"ooo:view-settings\">\
             <config:config-item-map-indexed config:name=\"Views\"><config:config-item-map-entry>",
        );
        item(&mut out, "ViewId", "string", "view1");
        let _ = write!(
            out,
            "<config:config-item-map-named config:name=\"Tables\">{tables}</config:config-item-map-named>"
        );
        let mut name = String::new();
        xml::odf(&mut name, &self.sheets[active].options.name, false);
        item(&mut out, "ActiveTable", "string", &name);
        out.push_str(
            "</config:config-item-map-entry></config:config-item-map-indexed>\
             </config:config-item-set></office:settings></office:document-settings>",
        );
        out
    }
}

/// The cells of `sheet` that a link, a note, an image or a chart is in, by row and then by
/// column.
fn spots(sheet: &Sheet) -> Vec<Spot<'_>> {
    let mut spots: Vec<Spot<'_>> = Vec::new();
    let mut index: HashMap<(u32, u32), usize> = HashMap::new();
    let mut spot = |row: u32, column: u32| -> usize {
        *index.entry((row, column)).or_insert_with(|| {
            spots.push(Spot {
                row,
                column,
                link: None,
                note: None,
                images: Vec::new(),
                charts: Vec::new(),
                last: sheet.next.saturating_sub(1),
            });
            spots.len() - 1
        })
    };
    let mut places = Vec::new();
    for (column, row, link) in &sheet.links {
        places.push((spot(*row, *column), Some(link), None, None, None));
    }
    for (column, row, note) in &sheet.notes {
        places.push((spot(*row, *column), None, Some(note), None, None));
    }
    for image in &sheet.images {
        let at = spot(image.placed.row + 1, image.placed.column);
        places.push((at, None, None, Some(image), None));
    }
    for held in &sheet.charts {
        let at = spot(held.placed.row + 1, held.placed.column);
        places.push((at, None, None, None, Some(held)));
    }
    for (at, link, note, image, held) in places {
        let spot = &mut spots[at];
        spot.link = spot.link.or(link);
        spot.note = spot.note.or(note);
        spot.images.extend(image);
        spot.charts.extend(held);
    }
    spots.sort_by_key(|spot| (spot.row, spot.column));
    spots
}

/// Appends the `table:table-column`s of `sheet`, `widest` of them, each run of equal columns as
/// one, nested in a `table:table-column-group` per outline level.
fn write_columns(out: &mut String, sheet: &Sheet, widest: usize, styles: &mut Named) {
    let kept = sheet.columns.as_deref().unwrap_or_default();
    let column_at = |at: usize| {
        let column = kept.get(at).copied().unwrap_or(PLAIN_COLUMN);
        let outline = sheet.groups.column(at as u32);
        HeldColumn {
            hidden: column.hidden || outline.hidden,
            level: outline.level.min(7),
            ..column
        }
    };
    let mut depth = 0;
    let mut at = 0;
    while at < widest {
        let column = column_at(at);
        let mut repeat = 1;
        while at + repeat < widest && column_at(at + repeat) == column {
            repeat += 1;
        }
        while depth > column.level {
            out.push_str("</table:table-column-group>");
            depth -= 1;
        }
        while depth < column.level {
            depth += 1;
            out.push_str("<table:table-column-group");
            if column.hidden && depth == column.level {
                out.push_str(" table:display=\"false\"");
            }
            out.push('>');
        }
        out.push_str("<table:table-column");
        let width = match (column.width, column.hidden) {
            (Some(width), _) => Some(width),
            (None, true) => Some(DEFAULT_WIDTH),
            (None, false) => None,
        };
        if let Some(width) = width {
            let id = styles.id(inches(width));
            let _ = write!(out, " table:style-name=\"co{id}\"");
        }
        if repeat > 1 {
            let _ = write!(out, " table:number-columns-repeated=\"{repeat}\"");
        }
        if column.hidden {
            out.push_str(" table:visibility=\"collapse\"");
        }
        if column.xf != 0 {
            let _ = write!(out, " table:default-cell-style-name=\"ce{}\"", column.xf);
        }
        out.push_str("/>");
        at += repeat;
    }
    while depth > 0 {
        out.push_str("</table:table-column-group>");
        depth -= 1;
    }
}

/// Appends the rows of `sheet`: each kept row, then the empty rows up to the last row a part
/// needs, a run of equal empty rows as one.
fn write_rows(out: &mut String, sheet: &Sheet, spots: &[Spot<'_>], rows: &mut Rows<'_, '_>) {
    let mut merges: Vec<Area> = sheet.tail.merges().to_vec();
    merges.sort_by_key(|at| (at.top, at.left));
    let mut checks: Vec<(Area, usize)> =
        sheet.checks.iter().map(|(at, id, _)| (*at, *id)).collect();
    checks.sort_by_key(|(at, id)| (at.top, *id));
    let header_rows = sheet
        .tail
        .print()
        .and_then(|print| print.repeat_rows.as_deref())
        .and_then(row_span);
    let held = sheet.rows.len() as u32;
    // The rows where an empty row may differ from the one above it.
    let mut breaks: Vec<u32> = Vec::new();
    let mut last = held;
    for at in &merges {
        breaks.extend([at.top, at.top + 1, at.bottom + 1]);
        last = last.max(at.bottom);
    }
    for (at, _) in &checks {
        breaks.extend([at.top, at.bottom + 1]);
        last = last.max(at.bottom);
    }
    for spot in spots {
        breaks.extend([spot.row, spot.row + 1]);
        last = last.max(spot.row);
    }
    if let Some((from, to)) = header_rows {
        breaks.extend([from, to + 1]);
        last = last.max(to);
    }
    breaks.push(last + 1);
    breaks.sort_unstable();
    breaks.dedup();
    let (mut merge_next, mut check_next, mut spot_next) = (0, 0, 0);
    let mut active_merges: Vec<Area> = Vec::new();
    let mut active_checks: Vec<(Area, usize)> = Vec::new();
    let mut depth = 0;
    let mut in_header = false;
    let mut row = 1;
    while row <= last {
        let data = sheet.rows.get(row as usize - 1);
        let span = match data {
            Some(_) => 1,
            None => breaks[breaks.partition_point(|&at| at <= row)] - row,
        };
        while merge_next < merges.len() && merges[merge_next].top <= row {
            active_merges.push(merges[merge_next]);
            merge_next += 1;
        }
        active_merges.retain(|at| at.bottom >= row);
        while check_next < checks.len() && checks[check_next].0.top <= row {
            active_checks.push(checks[check_next]);
            check_next += 1;
        }
        active_checks.retain(|(at, _)| at.bottom >= row);
        let first_spot = spot_next;
        while spot_next < spots.len() && spots[spot_next].row == row {
            spot_next += 1;
        }
        let level = data.map_or(0, |data| data.level);
        let hidden = data.is_some_and(|data| data.hidden);
        if in_header && depth != level {
            // Header rows stay inside one group.
            out.push_str("</table:table-header-rows>");
            in_header = false;
        }
        while depth > level {
            out.push_str("</table:table-row-group>");
            depth -= 1;
        }
        while depth < level {
            depth += 1;
            out.push_str("<table:table-row-group");
            if hidden && depth == level {
                out.push_str(" table:display=\"false\"");
            }
            out.push('>');
        }
        if header_rows.is_some_and(|(from, _)| from == row) {
            out.push_str("<table:table-header-rows>");
            in_header = true;
        }
        out.push_str("<table:table-row");
        if let Some(height) = data.and_then(|data| data.height) {
            let mut text = String::new();
            crate::cells::number(&mut text, height);
            let id = rows.row_styles.id(text);
            let _ = write!(out, " table:style-name=\"ro{id}\"");
        }
        if span > 1 {
            let _ = write!(out, " table:number-rows-repeated=\"{span}\"");
        }
        if hidden {
            out.push_str(" table:visibility=\"collapse\"");
        }
        if let Some(data) = data
            && data.xf != 0
        {
            let _ = write!(out, " table:default-cell-style-name=\"ce{}\"", data.xf);
        }
        out.push('>');
        write_cells(
            out,
            &sheet.options.name,
            row,
            data,
            &active_merges,
            &active_checks,
            &spots[first_spot..spot_next],
            rows,
        );
        out.push_str("</table:table-row>");
        row += span;
        if in_header && header_rows.is_some_and(|(_, to)| row > to) {
            out.push_str("</table:table-header-rows>");
            in_header = false;
        }
    }
    if in_header {
        out.push_str("</table:table-header-rows>");
    }
    while depth > 0 {
        out.push_str("</table:table-row-group>");
        depth -= 1;
    }
    if last == 0 {
        out.push_str("<table:table-row><table:table-cell/></table:table-row>");
    }
}

/// Appends the cells of the row `row` of the sheet `sheet`: its kept cells `data`, and what the
/// merges, the validations and the spots active on it put in each cell. A run of equal cells is
/// one cell, and the empty cells after the last one are not written.
#[allow(clippy::too_many_arguments)]
fn write_cells(
    out: &mut String,
    sheet: &str,
    row: u32,
    data: Option<&HeldRow>,
    merges: &[Area],
    checks: &[(Area, usize)],
    spots: &[Spot<'_>],
    rows: &mut Rows<'_, '_>,
) {
    let mut cuts: Vec<u32> = vec![0];
    if let Some(data) = data {
        for (column, cell) in data.cells.iter().enumerate() {
            if cell.written() {
                cuts.extend([column as u32, column as u32 + 1]);
            }
        }
    }
    for at in merges {
        cuts.extend([at.left, at.right + 1]);
        if at.top == row {
            cuts.push(at.left + 1);
        }
    }
    for (at, _) in checks {
        cuts.extend([at.left, at.right + 1]);
    }
    for spot in spots {
        cuts.extend([spot.column, spot.column + 1]);
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut empty = 0;
    let mut wrote = false;
    for pair in cuts.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let held = data
            .and_then(|data| data.cells.get(from as usize))
            .filter(|cell| cell.written());
        let merge = merges.iter().find(|at| at.left <= from && from <= at.right);
        let start = merge.filter(|at| at.top == row && at.left == from);
        let check = checks
            .iter()
            .filter(|(at, _)| at.left <= from && from <= at.right)
            .map(|&(_, id)| id)
            .min();
        let spot = spots.iter().find(|spot| spot.column == from);
        if held.is_none() && merge.is_none() && check.is_none() && spot.is_none() {
            empty += (to - from) as usize;
            continue;
        }
        empty_cells(out, &mut empty);
        wrote = true;
        let tag = if merge.is_some() && start.is_none() {
            "table:covered-table-cell"
        } else {
            "table:table-cell"
        };
        let _ = write!(out, "<{tag}");
        if let Some(cell) = held
            && cell.xf != 0
        {
            let _ = write!(out, " table:style-name=\"ce{}\"", cell.xf);
        }
        if let Some(id) = check {
            let _ = write!(out, " table:content-validation-name=\"val{id}\"");
        }
        if let Some(at) = start {
            let _ = write!(
                out,
                " table:number-columns-spanned=\"{}\" table:number-rows-spanned=\"{}\"",
                at.right - at.left + 1,
                at.bottom - at.top + 1
            );
        }
        if to - from > 1 {
            let _ = write!(out, " table:number-columns-repeated=\"{}\"", to - from);
        }
        let lead = spot.map(Spot::lead).unwrap_or_default();
        let link = spot.and_then(|spot| spot.link);
        let opened = link.map(link_open);
        let inside = cells::Inside {
            lead: &lead,
            link: opened.as_deref(),
        };
        let value = held.and_then(|cell| cell.value.as_ref());
        // A structured reference becomes the A1 range it covers.
        let resolved = match value {
            Some(Value::Formula {
                formula,
                cached_text,
                cached_number,
            }) if formula.contains('[') => Some(Value::Formula {
                formula: parts::resolve(formula, row, sheet, rows.tables),
                cached_text: cached_text.clone(),
                cached_number: *cached_number,
            }),
            _ => None,
        };
        // A link in a cell with no value shows its text, or its target.
        let shown = match (value, link) {
            (None, Some(link)) => Some(Value::Text(
                link.text.clone().unwrap_or_else(|| link.target.clone()),
            )),
            _ => None,
        };
        match resolved.as_ref().or(value).or(shown.as_ref()) {
            Some(value) => {
                cells::value(out, value, rows.run, inside);
                let _ = write!(out, "</{tag}>");
            }
            None if !lead.is_empty() => {
                out.push('>');
                out.push_str(&lead);
                let _ = write!(out, "</{tag}>");
            }
            None => out.push_str("/>"),
        }
    }
    if !wrote {
        out.push_str("<table:table-cell/>");
    }
}

/// Appends `count` empty cells as one repeated cell, and sets `count` to 0.
fn empty_cells(out: &mut String, count: &mut usize) {
    match *count {
        0 => {}
        1 => out.push_str("<table:table-cell/>"),
        n => {
            let _ = write!(
                out,
                "<table:table-cell table:number-columns-repeated=\"{n}\"/>"
            );
        }
    }
    *count = 0;
}

/// `META-INF/manifest.xml`: the package, each of its parts, each image and each chart object
/// by its folder.
fn manifest(images: &[&HeldImage], objects: &[(String, String)]) -> String {
    let mut out = String::from(HEAD);
    let _ = write!(
        out,
        "<manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.3\">\
         <manifest:file-entry manifest:full-path=\"/\" manifest:version=\"1.3\" manifest:media-type=\"{MIME}\"/>"
    );
    for part in ["content.xml", "styles.xml", "meta.xml", "settings.xml"] {
        let _ = write!(
            out,
            "<manifest:file-entry manifest:full-path=\"{part}\" manifest:media-type=\"text/xml\"/>"
        );
    }
    for image in images {
        let _ = write!(
            out,
            "<manifest:file-entry manifest:full-path=\"{}\" manifest:media-type=\"{}\"/>",
            image.entry(),
            image.placed.format.content_type()
        );
    }
    for (folder, _) in objects {
        out.push_str("<manifest:file-entry manifest:full-path=\"");
        xml::odf(&mut out, folder, true);
        out.push_str("/\" manifest:version=\"1.3\" manifest:media-type=\"application/vnd.oasis.opendocument.chart\"/>");
        out.push_str("<manifest:file-entry manifest:full-path=\"");
        xml::odf(&mut out, folder, true);
        out.push_str("/content.xml\" manifest:media-type=\"text/xml\"/>");
    }
    out.push_str("</manifest:manifest>");
    out
}
