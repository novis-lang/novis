//! The ods writer: the same calls as [`crate::Writer`], with the workbook held until
//! [`Writer::finish`] (`rule:core-classes/spreadsheet-writes-as-a-stream`).
//!
//! **The writer holds the workbook.** ods puts some parts before the first sheet and others
//! inside the cells they cover, so no part of `content.xml` can leave before the last row is
//! known. [`Writer::sheet`] and [`Writer::rows`] check each row as xlsx does and keep it, with
//! each cell's value and its cell format. [`Writer::finish`] writes the whole package and returns
//! every byte. Memory: every cell of the workbook, per writing request, freed at `finish`.
//!
//! **The package** is a zip with `mimetype` first and stored, as ODF requires, then
//! `content.xml`, `styles.xml`, `meta.xml`, `settings.xml` and `META-INF/manifest.xml`. The
//! sheets are written in tab order. Every style is an automatic style in `content.xml`:
//!
//! - **Cell styles** are `ce<n>`, one for each `cellXfs` entry [`crate::styles::Styles`] makes,
//!   so equal combinations of layers share one. `styles` writes them, and `numbers` writes each
//!   distinct format code as the number style `N<n>`.
//! - **Columns** are `co<n>` for each distinct width, written as Excel's character width turned
//!   into inches. A column's own style is its `table:default-cell-style-name`.
//! - **Rows** are `ro<n>` for each distinct height. A row's style is its default cell style.
//!   A row's outline level nests it in `table:table-row-group`s.
//! - **Sheets** are `ta<n>` for each distinct display, direction and tab colour. A very hidden
//!   sheet is hidden, because ods has no very hidden sheet.
//!
//! **The header row** gets what it gets in xlsx: its style, the rows above the second frozen,
//! and a filter over the header and every row below it. The filter is the database range
//! `__Anonymous_Sheet_DB__<tab>`, the name LibreOffice gives a sheet's own filter, so a reader
//! does not take it for a table. Auto width measures the first batch, as in xlsx.
//!
//! **`settings.xml`** has each sheet's frozen rows and columns, zoom and gridlines, in the view
//! settings LibreOffice reads. A defined name over a range is a `table:named-range`, and any
//! other is a `table:named-expression`. A formula is OpenFormula, with its cached result.
//!
//! **Two writes are byte-equal**: every part is written in the order of the calls, and the
//! document dates are `writer`'s fixed ones.
//!
//! The sheet-end parts are not written yet. [`Writer::part`] throws naming ods.

mod cells;
mod numbers;
mod styles;

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::styles::{Implicit, Styles};
use crate::tail::Part;
use crate::writer::{
    MAX_COLUMNS, MAX_ROWS, check_name, check_options, check_sheet_name, clock_text, continued,
    fold_column_formats, header_layer,
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
     xmlns:tableooo=\"http://openoffice.org/2009/table\"";

/// One kept cell: its value, and its `cellXfs` entry, 0 for none.
struct Held {
    value: Option<Value>,
    xf: u32,
}

/// One kept row.
struct HeldRow {
    cells: Vec<Held>,
    xf: u32,
    height: Option<f64>,
    hidden: bool,
    level: u64,
}

/// One column of a sheet, as its first batch set it.
#[derive(Clone, Copy, PartialEq)]
struct HeldColumn {
    width: Option<f64>,
    xf: u32,
    hidden: bool,
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
        }
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
        Some(format!(
            "A1:{}",
            crate::cells::reference(columns as u32 - 1, last)
        ))
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
        check_name(name, reference, &self.names, &[])?;
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
        let base = options.name.clone();
        self.sheets.push(Sheet::new(options, header_style, base, 1));
        self.open = true;
        self.rows(first)
    }

    /// Adds a sheet-end part to the open sheet. ods does not have them yet.
    pub fn part(&mut self, _part: Part) -> Result<(), Error> {
        self.writing()?;
        Err(Error::Invalid(
            "This part cannot be written to an ods file yet. Write the workbook as xlsx to use it."
                .to_owned(),
        ))
    }

    /// The open sheet's data range in the column at `column`, from 0, over the rows kept so
    /// far, such as `C2:C1043`. The header row is not in it.
    pub fn column(&self, column: u32) -> Result<String, Error> {
        let Some(sheet) = self.open_sheet() else {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you ask for a column.".to_owned(),
            ));
        };
        let first = if sheet.header.is_some() { 2 } else { 1 };
        let last = sheet.next.saturating_sub(1).max(first);
        Ok(format!(
            "{}:{}",
            crate::writer::address(column, first)?,
            crate::writer::address(column, last)?
        ))
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
        if !self.open {
            return Err(Error::Invalid(
                "No sheet is open. Call `sheet()` before you add rows.".to_owned(),
            ));
        }
        let Some(sheet) = self.sheets.last() else {
            unreachable!("an open sheet is the last one");
        };
        let room = (MAX_ROWS + 1).saturating_sub(sheet.next) as usize;
        if rows.len() > room && sheet.options.new_sheet_on_overflow {
            self.rows(&rows[..room])?;
            self.overflow()?;
            return self.rows(&rows[room..]);
        }
        self.keep(rows)
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
            Some(header) => self.keep(&[header]),
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
        let mut number = sheet.next;
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
            if row.level > 7 {
                return Err(Error::Invalid(format!(
                    "Row {number} of the sheet `{name}` has the outline level {}. The level is from 0 to 7.",
                    row.level
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
                cells.push(Held {
                    value: cell.value.clone(),
                    xf,
                });
            }
            kept.push(HeldRow {
                cells,
                xf: row_xf,
                height: row.height,
                hidden: row.hidden,
                level: row.level,
            });
            number += 1;
        }
        let Some(sheet) = self.sheets.last_mut() else {
            unreachable!("an open sheet is the last one");
        };
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
        });
    }
    Ok(out)
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
        let content = self.content(&order)?;
        let parts = [
            ("content.xml", content),
            ("styles.xml", self.styles_part()),
            ("meta.xml", self.meta()?),
            ("settings.xml", self.settings(&order, active)),
            ("META-INF/manifest.xml", manifest()),
        ];
        let mut archive = Archive::new();
        let written = (|| {
            archive.stored("mimetype", MIME.as_bytes())?;
            for (name, text) in &parts {
                archive.begin(name)?;
                archive.write(text.as_bytes())?;
            }
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
        // The workbook is written, so its cells go now, not when the request ends.
        self.sheets
            .iter_mut()
            .for_each(|sheet| sheet.rows = Vec::new());
        Ok(Finished {
            size: data.len() as u64,
            data,
            rows,
        })
    }

    /// `content.xml`: the automatic styles, the sheets in tab order, the names and the filters.
    fn content(&self, order: &[usize]) -> Result<String, Error> {
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
        let mut column_styles = Named::default();
        let mut row_styles = Named::default();
        let mut table_styles = Named::default();
        let mut runs: HashMap<u64, String> = HashMap::new();
        let mut run_order: Vec<u64> = Vec::new();
        let mut body = String::new();
        for &at in order {
            self.table(
                &mut body,
                &self.sheets[at],
                &mut column_styles,
                &mut row_styles,
                &mut table_styles,
                &mut |id| {
                    let next = runs.len() + 1;
                    runs.entry(id)
                        .or_insert_with(|| {
                            run_order.push(id);
                            format!("T{next}")
                        })
                        .clone()
                },
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
        for (at, props) in table_styles.order.iter().enumerate() {
            let _ = write!(
                out,
                "<style:style style:name=\"ta{}\" style:family=\"table\"><style:table-properties{props}/></style:style>",
                at + 1
            );
        }
        out.push_str(&cell_styles);
        for id in &run_order {
            styles::run(&mut out, &runs[id], self.styles.get(*id)?)?;
        }
        out.push_str("</office:automatic-styles><office:body><office:spreadsheet>");
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

    /// Appends one sheet as `table:table`.
    fn table(
        &self,
        out: &mut String,
        sheet: &Sheet,
        column_styles: &mut Named,
        row_styles: &mut Named,
        table_styles: &mut Named,
        run: &mut dyn FnMut(u64) -> String,
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
        let table_style = table_styles.id(props);
        out.push_str("<table:table table:name=\"");
        xml::odf(out, &options.name, true);
        let _ = write!(out, "\" table:style-name=\"ta{table_style}\">");
        let columns = sheet.columns.clone().unwrap_or_default();
        let widest = sheet
            .rows
            .iter()
            .map(|row| row.cells.len())
            .max()
            .unwrap_or(0)
            .max(columns.len())
            .max(1);
        let mut at = 0;
        while at < widest {
            let column = columns.get(at).copied().unwrap_or(HeldColumn {
                width: None,
                xf: 0,
                hidden: false,
            });
            let mut repeat = 1;
            while at + repeat < widest
                && columns.get(at + repeat).copied().unwrap_or(HeldColumn {
                    width: None,
                    xf: 0,
                    hidden: false,
                }) == column
            {
                repeat += 1;
            }
            out.push_str("<table:table-column");
            let width = match (column.width, column.hidden) {
                (Some(width), _) => Some(width),
                (None, true) => Some(DEFAULT_WIDTH),
                (None, false) => None,
            };
            if let Some(width) = width {
                let id = column_styles.id(inches(width));
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
        let mut depth = 0;
        for row in &sheet.rows {
            while depth < row.level {
                out.push_str("<table:table-row-group>");
                depth += 1;
            }
            while depth > row.level {
                out.push_str("</table:table-row-group>");
                depth -= 1;
            }
            out.push_str("<table:table-row");
            if let Some(height) = row.height {
                let mut text = String::new();
                crate::cells::number(&mut text, height);
                let id = row_styles.id(text);
                let _ = write!(out, " table:style-name=\"ro{id}\"");
            }
            if row.hidden {
                out.push_str(" table:visibility=\"collapse\"");
            }
            if row.xf != 0 {
                let _ = write!(out, " table:default-cell-style-name=\"ce{}\"", row.xf);
            }
            out.push('>');
            let last = row
                .cells
                .iter()
                .rposition(|cell| cell.value.is_some() || cell.xf != 0);
            let cells = last.map_or(&row.cells[..0], |last| &row.cells[..=last]);
            let mut empty = 0;
            for cell in cells {
                if cell.value.is_none() && cell.xf == 0 {
                    empty += 1;
                    continue;
                }
                empty_cells(out, &mut empty);
                out.push_str("<table:table-cell");
                if cell.xf != 0 {
                    let _ = write!(out, " table:style-name=\"ce{}\"", cell.xf);
                }
                match &cell.value {
                    Some(value) => {
                        cells::value(out, value, run);
                        out.push_str("</table:table-cell>");
                    }
                    None => out.push_str("/>"),
                }
            }
            if cells.is_empty() {
                out.push_str("<table:table-cell/>");
            }
            out.push_str("</table:table-row>");
        }
        while depth > 0 {
            out.push_str("</table:table-row-group>");
            depth -= 1;
        }
        if sheet.rows.is_empty() {
            out.push_str("<table:table-row><table:table-cell/></table:table-row>");
        }
        out.push_str("</table:table>");
    }

    /// `styles.xml`: the default cell style, Excel's default font, and nothing else.
    fn styles_part(&self) -> String {
        let mut out = String::from(HEAD);
        let _ = write!(
            out,
            "<office:document-styles {NAMESPACES} office:version=\"1.3\"><office:styles>\
             <style:default-style style:family=\"table-cell\"><style:text-properties fo:font-family=\"Calibri\" fo:font-size=\"11pt\"/></style:default-style>\
             <style:style style:name=\"Default\" style:family=\"table-cell\"/>\
             </office:styles></office:document-styles>"
        );
        out
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

/// `META-INF/manifest.xml`: the package and each of its parts.
fn manifest() -> String {
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
    out.push_str("</manifest:manifest>");
    out
}
