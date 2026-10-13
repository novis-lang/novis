//! The spreadsheet component: `nvs:spreadsheet/engine` from `wit/spreadsheet.wit`, built into
//! every `nvs` binary (`rule:packaging/the-first-party-components-are-built-in`,
//! `rule:core-classes/spreadsheet-roster`).
//!
//! The crate has two halves, as the image component's does. The core is plain Rust that builds
//! for the host too, so its tests run under an ordinary `cargo test` in this directory. The WIT
//! glue in `guest` is compiled only for a wasm target, and maps each export onto the core and
//! the WIT types onto the core's.
//!
//! **Writing is a stream** (`rule:core-classes/spreadsheet-writes-as-a-stream`). A `writer`
//! resource is a [`Writer`]: `writer-sheet` and `writer-rows` take a batch of rows, write its
//! sheet XML as text, deflate it, and return the compressed bytes ready to leave, and
//! `writer-finish` writes the rest of the workbook and the zip directory. The writer is ours and
//! frames the zip by hand (`zip`'s module doc), so the guest never knows an entry's size in
//! advance, never seeks, and keeps no row of an earlier batch. Strings are inline, so no
//! shared-string table grows. Each cell is written by the write table (`cells`' module doc), and
//! its format is its layers and its value's format combined (`styles`' module doc). The sheet
//! order, the parts and the fixed dates are `writer`'s module doc. The component performs no
//! I/O and is granted nothing (`rule:core-classes/spreadsheet-has-no-io`): the Novis half sends
//! the bytes on.
//!
//! **A workbook never finished is logged.** The guest's `writer` is dropped when the request
//! ends, and one dropped before `writer-finish` writes an error to the request's log naming the
//! workbook's `target` ([`writer::unfinished_message`]).
//!
//! **Not done yet.** Of `writer-part`'s cases, tables, conditional formats, images and charts
//! return `runtime`: Stages 5 and 6 of goal `spreadsheet`. The reader and the template exports
//! return `runtime` too, and are not in `manifest.json`: Stages 7 and 10.
//!
//! Packed as a `.nvsx` and embedded in every `nvs` binary, the component is 0.41 MiB, built for
//! release as `crates/nvs-ext/build.rs` always builds it. That is the binary's growth for
//! spreadsheets, and `crates/nvs-ext/tests/builtin.rs` fails when this figure drifts past 5% of
//! the bytes the binary carries.

pub mod cells;
pub mod conditional;
pub mod drawing;
pub mod styles;
pub mod table;
pub mod tail;
pub mod width;
pub mod writer;
pub mod xml;
pub mod zip;

pub use writer::Writer;

/// The WIT `error`: `Invalid`, `Parse` and `Runtime` throw `LogicError`, `ParseError` and
/// `RuntimeError` (`rule:packaging/a-guest-crash-throws`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Invalid(String),
    Parse(String),
    Runtime(String),
}

/// A calendar date, `month` and `day` from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

/// A time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub hour: i64,
    pub minute: i64,
    pub second: i64,
    pub nanos: i64,
}

/// A clock time in no zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub day: Date,
    pub clock: TimeOfDay,
}

/// One run of rich text, with an optional style layer.
#[derive(Debug, Clone, PartialEq)]
pub struct RichRun {
    pub text: String,
    pub style: Option<u64>,
}

/// One cell's value, by the write table.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Number(f64),
    Integer(i64),
    Decimal(String),
    Boolean(bool),
    Date(Date),
    Time(TimeOfDay),
    /// A duration in nanoseconds.
    Duration(i64),
    Clock(Clock),
    /// A formula's text with no leading `=`, and its cached result. A cached text wins over a
    /// cached number.
    Formula {
        formula: String,
        cached_text: Option<String>,
        cached_number: Option<f64>,
    },
    Rich(Vec<RichRun>),
}

/// One cell: its value, and its own style layer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cell {
    pub value: Option<Value>,
    pub style: Option<u64>,
}

/// One row and what xlsx writes with it. A `header` row is the sheet's header row, which only the
/// sheet's first row can be (`writer`'s module doc).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub style: Option<u64>,
    pub height: Option<f64>,
    pub hidden: bool,
    pub level: u64,
    pub header: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Thin,
    Medium,
    Thick,
    Dashed,
    Dotted,
    Double,
}

/// The four sides of a border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Border {
    pub top: Option<Line>,
    pub right: Option<Line>,
    pub bottom: Option<Line>,
    pub left: Option<Line>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Underline {
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
    Fill,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Valign {
    Top,
    Center,
    Bottom,
}

/// One style layer (`rule:core-classes/spreadsheet-styles-and-defaults`). `format` is a format
/// code or one of the shortcuts, which [`styles::Styles::layer`] turns into its code.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    pub font: Option<String>,
    pub size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub strike: Option<bool>,
    pub underline: Option<Underline>,
    pub color: Option<String>,
    pub fill: Option<String>,
    pub border: Option<Border>,
    pub border_color: Option<String>,
    pub align: Option<Align>,
    pub valign: Option<Valign>,
    pub wrap: Option<bool>,
    pub rotate: Option<i64>,
    pub indent: Option<u64>,
    pub format: Option<String>,
    pub locked: Option<bool>,
    pub hide_formula: Option<bool>,
}

impl Style {
    /// This layer over `base`: each field this layer sets replaces the base's, and a border
    /// side this layer sets replaces that side.
    pub fn over(&self, base: &Style) -> Style {
        let border = match (self.border, base.border) {
            (Some(top), Some(under)) => Some(Border {
                top: top.top.or(under.top),
                right: top.right.or(under.right),
                bottom: top.bottom.or(under.bottom),
                left: top.left.or(under.left),
            }),
            (top, under) => top.or(under),
        };
        Style {
            font: self.font.clone().or_else(|| base.font.clone()),
            size: self.size.or(base.size),
            bold: self.bold.or(base.bold),
            italic: self.italic.or(base.italic),
            strike: self.strike.or(base.strike),
            underline: self.underline.or(base.underline),
            color: self.color.clone().or_else(|| base.color.clone()),
            fill: self.fill.clone().or_else(|| base.fill.clone()),
            border,
            border_color: self
                .border_color
                .clone()
                .or_else(|| base.border_color.clone()),
            align: self.align.or(base.align),
            valign: self.valign.or(base.valign),
            wrap: self.wrap.or(base.wrap),
            rotate: self.rotate.or(base.rotate),
            indent: self.indent.or(base.indent),
            format: self.format.clone().or_else(|| base.format.clone()),
            locked: self.locked.or(base.locked),
            hide_formula: self.hide_formula.or(base.hide_formula),
        }
    }
}

/// The document properties `docProps/` carries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Properties {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub author: Option<String>,
    pub company: Option<String>,
    pub keywords: Option<String>,
    pub created: Option<Clock>,
}

/// What `writer-open` takes. `target` names where the bytes go, for the log line of a workbook
/// never finished.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkbookOptions {
    pub properties: Properties,
    pub target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
    VeryHidden,
}

/// A column, in header order. `format` is a format code or a shortcut over the column's `style`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Column {
    pub width: Option<f64>,
    pub auto: bool,
    pub style: Option<u64>,
    pub format: Option<String>,
    pub hidden: bool,
}

/// What a sheet's header row gets, when its first row is one. `default_style` is bold with a thin
/// bottom border, and `style` is a layer over it, or the whole style when `default_style` is
/// off. `freeze` freezes the rows above the second, and `filter` puts an autofilter over the
/// header and every row below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub default_style: bool,
    pub style: Option<u64>,
    pub freeze: bool,
    pub filter: bool,
}

impl Default for Header {
    /// The defaults of a sheet with a header row: every one on.
    fn default() -> Self {
        Self {
            default_style: true,
            style: None,
            freeze: true,
            filter: true,
        }
    }
}

/// What `writer-sheet` takes beside the first rows. A column past `columns` has auto width when
/// the sheet has a header row, and no width otherwise. `new_sheet_on_overflow` continues a sheet
/// that reaches the last row of the grid on a new sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetOptions {
    pub name: String,
    pub position: Option<u64>,
    pub visibility: Visibility,
    pub columns: Vec<Column>,
    pub freeze_rows: u64,
    pub freeze_columns: u64,
    pub autofilter: Option<String>,
    pub tab_color: Option<String>,
    pub zoom: Option<u64>,
    pub gridlines: bool,
    pub right_to_left: bool,
    pub header: Header,
    pub new_sheet_on_overflow: bool,
}

impl SheetOptions {
    /// A visible sheet named `name`, with gridlines, the header defaults, and nothing else set.
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            position: None,
            visibility: Visibility::Visible,
            columns: Vec::new(),
            freeze_rows: 0,
            freeze_columns: 0,
            autofilter: None,
            tab_color: None,
            zoom: None,
            gridlines: true,
            right_to_left: false,
            header: Header::default(),
            new_sheet_on_overflow: false,
        }
    }
}

/// One sheet's row count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetRows {
    pub name: String,
    pub rows: u64,
}

/// What `writer-finish` returns: the last bytes, the rows of each sheet in the order they were
/// written, and the size of every byte the workbook returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub data: Vec<u8>,
    pub rows: Vec<SheetRows>,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Cell {
        Cell {
            value: Some(Value::Text(value.to_owned())),
            style: None,
        }
    }

    fn write() -> Vec<u8> {
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        let bold = writer
            .style(Style {
                bold: Some(true),
                ..Style::default()
            })
            .unwrap();
        let header = Row {
            cells: vec![text("Name"), text("Price")],
            style: Some(bold),
            ..Row::default()
        };
        let mut out = writer
            .sheet(SheetOptions::named("Shop"), &[header])
            .unwrap();
        let row = Row {
            cells: vec![
                text(" Tea "),
                Cell {
                    value: Some(Value::Number(3.5)),
                    style: None,
                },
            ],
            ..Row::default()
        };
        out.extend(writer.rows(&[row]).unwrap());
        let finished = writer.finish().unwrap();
        out.extend(&finished.data);
        assert_eq!(finished.size, out.len() as u64);
        assert_eq!(
            finished.rows,
            [SheetRows {
                name: "Shop".to_owned(),
                rows: 2
            }]
        );
        out
    }

    #[test]
    fn two_writes_of_equal_input_are_byte_equal() {
        assert_eq!(write(), write());
    }

    #[test]
    fn a_nan_is_refused_naming_its_cell_and_the_sheet_goes_on() {
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        writer.sheet(SheetOptions::named("Blog"), &[]).unwrap();
        let row = Row {
            cells: vec![
                text("a"),
                Cell {
                    value: Some(Value::Number(f64::NAN)),
                    style: None,
                },
            ],
            ..Row::default()
        };
        let err = writer.rows(&[row]).unwrap_err();
        assert!(
            matches!(&err, Error::Invalid(m) if m.contains("`B1`")),
            "{err:?}"
        );
        writer.rows(&[Row::default()]).unwrap();
        assert_eq!(writer.finish().unwrap().rows[0].rows, 1);
    }

    #[test]
    fn a_bad_sheet_name_is_refused_naming_the_rule() {
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        let err = writer.sheet(SheetOptions::named("a/b"), &[]).unwrap_err();
        assert!(
            matches!(&err, Error::Invalid(m) if m.contains("`/`")),
            "{err:?}"
        );
    }
}

#[cfg(target_family = "wasm")]
mod guest {
    wit_bindgen::generate!({
        path: ["../../wit/nvs-ext", "../../wit/spreadsheet.wit"],
        world: "nvs:spreadsheet/spreadsheet",
        generate_all,
    });

    use std::cell::RefCell;

    use crate::tail;
    use exports::nvs::spreadsheet::engine as wit;
    use exports::nvs::spreadsheet::engine::{Guest, GuestReader, GuestTemplate, GuestWriter};
    use nvs::ext::log;
    use nvs::ext::types::Value as Source;

    struct Component;

    /// A workbook being written, kept by the request that opened it.
    pub struct Book {
        writer: RefCell<crate::Writer>,
    }

    impl GuestWriter for Book {}

    impl Drop for Book {
        fn drop(&mut self) {
            let writer = self.writer.get_mut();
            if !writer.is_finished() {
                log::write(
                    log::Level::Error,
                    &crate::writer::unfinished_message(writer.target()),
                );
            }
        }
    }

    /// The reader, which Stage 7 of goal `spreadsheet` writes.
    pub struct Reader;

    impl GuestReader for Reader {}

    /// Template fill, which Stage 10 of goal `spreadsheet` writes.
    pub struct Template;

    impl GuestTemplate for Template {}

    fn reading() -> wit::Error {
        wit::Error::Runtime("This version of Novis cannot read workbooks yet.".to_owned())
    }

    fn filling() -> wit::Error {
        wit::Error::Runtime("This version of Novis cannot fill templates yet.".to_owned())
    }

    fn error(err: crate::Error) -> wit::Error {
        match err {
            crate::Error::Invalid(message) => wit::Error::Invalid(message),
            crate::Error::Parse(message) => wit::Error::Parse(message),
            crate::Error::Runtime(message) => wit::Error::Runtime(message),
        }
    }

    fn date(date: wit::Date) -> crate::Date {
        crate::Date {
            year: date.year,
            month: date.month,
            day: date.day,
        }
    }

    fn time(time: wit::TimeOfDay) -> crate::TimeOfDay {
        crate::TimeOfDay {
            hour: time.hour,
            minute: time.minute,
            second: time.second,
            nanos: time.nanos,
        }
    }

    fn clock(clock: wit::ClockCell) -> crate::Clock {
        crate::Clock {
            day: date(clock.day),
            clock: time(clock.clock),
        }
    }

    fn value(value: wit::CellValue) -> crate::Value {
        use crate::Value;
        match value {
            wit::CellValue::Text(cell) => Value::Text(cell.text),
            wit::CellValue::Number(cell) => Value::Number(cell.number),
            wit::CellValue::Integer(cell) => Value::Integer(cell.integer),
            wit::CellValue::Decimal(cell) => Value::Decimal(cell.decimal),
            wit::CellValue::Boolean(cell) => Value::Boolean(cell.boolean),
            wit::CellValue::Date(cell) => Value::Date(date(cell.date)),
            wit::CellValue::Time(cell) => Value::Time(time(cell.time)),
            wit::CellValue::Duration(cell) => Value::Duration(cell.duration.nanos),
            wit::CellValue::Clock(cell) => Value::Clock(clock(cell)),
            wit::CellValue::Formula(cell) => Value::Formula {
                formula: cell.formula,
                cached_text: cell.cached_text,
                cached_number: cell.cached_number,
            },
            wit::CellValue::Rich(cell) => Value::Rich(
                cell.runs
                    .into_iter()
                    .map(|run| crate::RichRun {
                        text: run.text,
                        style: run.style,
                    })
                    .collect(),
            ),
        }
    }

    fn rows(rows: Vec<wit::Row>) -> Vec<crate::Row> {
        rows.into_iter()
            .map(|row| crate::Row {
                cells: row
                    .cells
                    .into_iter()
                    .map(|cell| crate::Cell {
                        value: cell.value.map(value),
                        style: cell.style,
                    })
                    .collect(),
                style: row.style,
                height: row.height,
                hidden: row.hidden,
                level: row.level,
                header: row.header,
            })
            .collect()
    }

    fn line(line: wit::Line) -> crate::Line {
        match line {
            wit::Line::Thin => crate::Line::Thin,
            wit::Line::Medium => crate::Line::Medium,
            wit::Line::Thick => crate::Line::Thick,
            wit::Line::Dashed => crate::Line::Dashed,
            wit::Line::Dotted => crate::Line::Dotted,
            wit::Line::Double => crate::Line::Double,
        }
    }

    fn style(style: wit::Style) -> crate::Style {
        crate::Style {
            font: style.font,
            size: style.size,
            bold: style.bold,
            italic: style.italic,
            strike: style.strike,
            underline: style.underline.map(|underline| match underline {
                wit::Underline::Single => crate::Underline::Single,
                wit::Underline::Double => crate::Underline::Double,
            }),
            color: style.color,
            fill: style.fill,
            border: style.border.map(|border| crate::Border {
                top: border.top.map(line),
                right: border.right.map(line),
                bottom: border.bottom.map(line),
                left: border.left.map(line),
            }),
            border_color: style.border_color,
            align: style.align.map(|align| match align {
                wit::Align::Left => crate::Align::Left,
                wit::Align::Center => crate::Align::Center,
                wit::Align::Right => crate::Align::Right,
                wit::Align::Fill => crate::Align::Fill,
                wit::Align::Justify => crate::Align::Justify,
            }),
            valign: style.valign.map(|valign| match valign {
                wit::Valign::Top => crate::Valign::Top,
                wit::Valign::Center => crate::Valign::Center,
                wit::Valign::Bottom => crate::Valign::Bottom,
            }),
            wrap: style.wrap,
            rotate: style.rotate,
            indent: style.indent,
            format: style.format,
            locked: style.locked,
            hide_formula: style.hide_formula,
        }
    }

    fn sheet_options(options: wit::SheetOptions) -> crate::SheetOptions {
        crate::SheetOptions {
            name: options.name,
            position: options.position,
            visibility: match options.visibility {
                wit::Visibility::Visible => crate::Visibility::Visible,
                wit::Visibility::Hidden => crate::Visibility::Hidden,
                wit::Visibility::VeryHidden => crate::Visibility::VeryHidden,
            },
            columns: options
                .columns
                .into_iter()
                .map(|column| crate::Column {
                    width: column.width,
                    auto: column.auto,
                    style: column.style,
                    format: column.format,
                    hidden: column.hidden,
                })
                .collect(),
            freeze_rows: options.freeze_rows,
            freeze_columns: options.freeze_columns,
            autofilter: options.autofilter,
            tab_color: options.tab_color,
            zoom: options.zoom,
            gridlines: options.gridlines,
            right_to_left: options.right_to_left,
            header: crate::Header {
                default_style: options.header.default_style,
                style: options.header.style,
                freeze: options.header.freeze,
                filter: options.header.filter,
            },
            new_sheet_on_overflow: options.new_sheet_on_overflow,
        }
    }

    fn operator(operator: wit::Operator) -> tail::Operator {
        match operator {
            wit::Operator::Between => tail::Operator::Between,
            wit::Operator::NotBetween => tail::Operator::NotBetween,
            wit::Operator::Equal => tail::Operator::Equal,
            wit::Operator::NotEqual => tail::Operator::NotEqual,
            wit::Operator::Greater => tail::Operator::Greater,
            wit::Operator::GreaterOrEqual => tail::Operator::GreaterOrEqual,
            wit::Operator::Less => tail::Operator::Less,
            wit::Operator::LessOrEqual => tail::Operator::LessOrEqual,
        }
    }

    /// The part `part` is, or `runtime` for a kind this version does not write yet.
    fn sheet_part(part: wit::SheetPart) -> Result<tail::Part, wit::Error> {
        use tail::Part;
        Ok(match part {
            wit::SheetPart::Merge(merge) => Part::Merge(merge.merge),
            wit::SheetPart::Link(link) => Part::Link(tail::Link {
                anchor: link.anchor,
                target: link.target,
                text: link.text,
                tooltip: link.tooltip,
            }),
            wit::SheetPart::Note(note) => Part::Note(tail::Note {
                noted: note.noted,
                note: note.note,
                author: note.author,
            }),
            wit::SheetPart::Validation(rule) => Part::Validation(tail::Validation {
                validated: rule.validated,
                kind: match rule.kind {
                    wit::RuleKind::List => tail::RuleKind::List,
                    wit::RuleKind::Whole => tail::RuleKind::Whole,
                    wit::RuleKind::Decimal => tail::RuleKind::Decimal,
                    wit::RuleKind::Date => tail::RuleKind::Date,
                    wit::RuleKind::Length => tail::RuleKind::Length,
                    wit::RuleKind::Formula => tail::RuleKind::Formula,
                },
                operator: rule.operator.map(operator),
                values: rule.values,
                input_title: rule.input_title,
                input_message: rule.input_message,
                error_title: rule.error_title,
                error_message: rule.error_message,
                allow_empty: rule.allow_empty,
            }),
            wit::SheetPart::Group(group) => Part::Group(tail::Group {
                grouped: group.grouped,
                collapsed: group.collapsed,
            }),
            wit::SheetPart::Print(print) => Part::Print(tail::Print {
                orientation: print.orientation.map(|orientation| match orientation {
                    wit::Orientation::Portrait => tail::Orientation::Portrait,
                    wit::Orientation::Landscape => tail::Orientation::Landscape,
                }),
                fit_width: print.fit_width,
                fit_height: print.fit_height,
                margins: print.margins.map(|margins| tail::Margins {
                    top: margins.top,
                    right: margins.right,
                    bottom: margins.bottom,
                    left: margins.left,
                }),
                header: print.header,
                footer: print.footer,
                repeat_rows: print.repeat_rows,
                area: print.area,
            }),
            wit::SheetPart::Protect(protect) => Part::Protect(tail::Protect {
                password: protect.password,
                allow: protect.allow,
            }),
            wit::SheetPart::Image(image) => Part::Image(crate::drawing::Image {
                at: image.at,
                image: image.image,
                width: image.width,
                height: image.height,
                offset_x: image.offset_x,
                offset_y: image.offset_y,
            }),
            wit::SheetPart::Table(table) => Part::Table(crate::table::Table {
                range: table.table,
                name: table.name,
                style: table.style,
                banded: table.banded,
                filter: table.filter,
                totals: table
                    .totals
                    .into_iter()
                    .map(|total| crate::table::Total {
                        column: total.column,
                        function: total.function.map(|function| match function {
                            wit::TotalFunction::Sum => crate::table::Function::Sum,
                            wit::TotalFunction::Average => crate::table::Function::Average,
                            wit::TotalFunction::Count => crate::table::Function::Count,
                            wit::TotalFunction::Min => crate::table::Function::Min,
                            wit::TotalFunction::Max => crate::table::Function::Max,
                        }),
                        formula: total.formula,
                    })
                    .collect(),
            }),
            wit::SheetPart::Conditional(rule) => {
                Part::Conditional(crate::conditional::Conditional {
                    formatted: rule.formatted,
                    kind: match rule.kind {
                        wit::ConditionalKind::Cell => crate::conditional::Kind::Cell,
                        wit::ConditionalKind::Formula => crate::conditional::Kind::Formula,
                        wit::ConditionalKind::ColorScale => crate::conditional::Kind::ColorScale,
                        wit::ConditionalKind::DataBar => crate::conditional::Kind::DataBar,
                        wit::ConditionalKind::IconSet => crate::conditional::Kind::IconSet,
                        wit::ConditionalKind::Top => crate::conditional::Kind::Top,
                        wit::ConditionalKind::Bottom => crate::conditional::Kind::Bottom,
                        wit::ConditionalKind::Duplicates => crate::conditional::Kind::Duplicates,
                        wit::ConditionalKind::Unique => crate::conditional::Kind::Unique,
                    },
                    operator: rule.operator.map(operator),
                    values: rule.values,
                    colors: rule.colors,
                    icons: rule.icons,
                    rank: rule.rank,
                    percent: rule.percent,
                    style: rule.style.map(style),
                })
            }
            wit::SheetPart::Chart(_) => {
                return Err(wit::Error::Runtime(
                    "This version of Novis cannot add charts to a sheet yet.".to_owned(),
                ));
            }
        })
    }

    fn with<T>(
        writer: wit::WriterBorrow<'_>,
        call: impl FnOnce(&mut crate::Writer) -> Result<T, crate::Error>,
    ) -> Result<T, wit::Error> {
        call(&mut writer.get::<Book>().writer.borrow_mut()).map_err(error)
    }

    impl Guest for Component {
        type Writer = Book;
        type Reader = Reader;
        type Template = Template;

        fn writer_open(options: wit::WorkbookOptions) -> Result<wit::Writer, wit::Error> {
            let properties = options.properties;
            let writer = crate::Writer::open(crate::WorkbookOptions {
                properties: crate::Properties {
                    title: properties.title,
                    subject: properties.subject,
                    author: properties.author,
                    company: properties.company,
                    keywords: properties.keywords,
                    created: properties.created.map(clock),
                },
                target: options.target,
            })
            .map_err(error)?;
            Ok(wit::Writer::new(Book {
                writer: RefCell::new(writer),
            }))
        }

        fn writer_style(
            writer: wit::WriterBorrow<'_>,
            layer: wit::Style,
        ) -> Result<u64, wit::Error> {
            with(writer, |writer| writer.style(style(layer)))
        }

        fn writer_sheet(
            writer: wit::WriterBorrow<'_>,
            options: wit::SheetOptions,
            first: Vec<wit::Row>,
        ) -> Result<Vec<u8>, wit::Error> {
            let first = rows(first);
            with(writer, |writer| {
                writer.sheet(sheet_options(options), &first)
            })
        }

        fn writer_rows(
            writer: wit::WriterBorrow<'_>,
            batch: Vec<wit::Row>,
        ) -> Result<Vec<u8>, wit::Error> {
            let batch = rows(batch);
            with(writer, |writer| writer.rows(&batch))
        }

        fn writer_part(
            writer: wit::WriterBorrow<'_>,
            part: wit::SheetPart,
        ) -> Result<Vec<u8>, wit::Error> {
            let part = sheet_part(part)?;
            with(writer, |writer| writer.part(part))
        }

        fn writer_define(
            writer: wit::WriterBorrow<'_>,
            name: String,
            reference: String,
        ) -> Result<(), wit::Error> {
            with(writer, |writer| writer.define(&name, &reference))
        }

        fn writer_finish(writer: wit::WriterBorrow<'_>) -> Result<wit::Finished, wit::Error> {
            let finished = with(writer, crate::Writer::finish)?;
            Ok(wit::Finished {
                data: finished.data,
                rows: finished
                    .rows
                    .into_iter()
                    .map(|sheet| wit::SheetRows {
                        name: sheet.name,
                        rows: sheet.rows,
                    })
                    .collect(),
                size: finished.size,
            })
        }

        fn reader_open(_source: &Source) -> Result<wit::Reader, wit::Error> {
            Err(reading())
        }

        fn reader_sheets(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
        ) -> Result<Vec<wit::SheetInfo>, wit::Error> {
            Err(reading())
        }

        fn reader_header(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
            _options: wit::ReadOptions,
        ) -> Result<Vec<String>, wit::Error> {
            Err(reading())
        }

        fn reader_rows(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
            _options: wit::ReadOptions,
            _max: u64,
        ) -> Result<wit::ReadBatch, wit::Error> {
            Err(reading())
        }

        fn reader_merged(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
            _sheet: String,
        ) -> Result<Vec<String>, wit::Error> {
            Err(reading())
        }

        fn reader_links(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
            _sheet: String,
        ) -> Result<Vec<wit::LinkInfo>, wit::Error> {
            Err(reading())
        }

        fn reader_notes(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
            _sheet: String,
        ) -> Result<Vec<wit::NoteInfo>, wit::Error> {
            Err(reading())
        }

        fn reader_names(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
        ) -> Result<Vec<wit::NameInfo>, wit::Error> {
            Err(reading())
        }

        fn reader_tables(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
            _sheet: String,
        ) -> Result<Vec<wit::TableInfo>, wit::Error> {
            Err(reading())
        }

        fn reader_images(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
            _sheet: String,
        ) -> Result<Vec<wit::ImageInfo>, wit::Error> {
            Err(reading())
        }

        fn reader_properties(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
        ) -> Result<wit::Properties, wit::Error> {
            Err(reading())
        }

        fn reader_macros(
            _reader: wit::ReaderBorrow<'_>,
            _source: &Source,
        ) -> Result<Vec<String>, wit::Error> {
            Err(reading())
        }

        fn template_open(_source: &Source) -> Result<wit::Template, wit::Error> {
            Err(filling())
        }

        fn template_set(
            _template: wit::TemplateBorrow<'_>,
            _address: String,
            _value: wit::Cell,
        ) -> Result<(), wit::Error> {
            Err(filling())
        }

        fn template_fill(
            _template: wit::TemplateBorrow<'_>,
            _source: &Source,
            _values: Vec<wit::Placeholder>,
        ) -> Result<Vec<u8>, wit::Error> {
            Err(filling())
        }

        fn template_items(
            _template: wit::TemplateBorrow<'_>,
            _source: &Source,
            _batch: wit::ItemBatch,
        ) -> Result<Vec<u8>, wit::Error> {
            Err(filling())
        }

        fn template_finish(
            _template: wit::TemplateBorrow<'_>,
            _source: &Source,
        ) -> Result<wit::Finished, wit::Error> {
            Err(filling())
        }
    }

    export!(Component);
}
