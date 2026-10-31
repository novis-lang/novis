//! The spreadsheet component's `nvs:spreadsheet/engine`, called typed: `wit/spreadsheet.wit`'s types
//! as `bindgen!` writes them, and the conversion of each to and from its Novis value. Every export
//! takes a resource or a lent `mixed` source, so each of those parameters and results is a
//! [`ResourceAny`] here.

use wasmtime::component::ResourceAny;

use self::bindings::exports::nvs::spreadsheet::engine::{
    Align, BoolCell, Border, Cell, CellError, CellValue, ChartKind, ChartLayer, ChartPart,
    ChartSeries, ClockCell, Column, ConditionalKind, ConditionalPart, DateCell, DecimalCell,
    DecompressionBound, DurationCell, ErrorCell, Filled, Finished, FormulaCell, GroupPart,
    HeaderOptions, ImageInfo, ImagePart, IntegerCell, ItemBatch, Legend, Line, LinkInfo, LinkPart,
    Margins, MergePart, NameInfo, NoteInfo, NotePart, NumberCell, Operator, Orientation,
    Placeholder, PrintPart, Properties, ProtectPart, ReadBatch, ReadCell, ReadOptions, ReadRow,
    ReadValue, RichCell, RichRun, Row, RuleKind, SheetInfo, SheetKind, SheetOptions, SheetPart,
    SheetRows, Style, TableInfo, TablePart, TableTotal, TextCell, TimeCell, TotalFunction,
    Underline, ValidationPart, Valign, Visibility, WorkbookOptions,
};
use self::bindings::nvs::ext::types::{Date, Duration, Error as WitError, TimeOfDay};
use crate::call::Error;

/// The types `bindgen!` writes for the world that exports `nvs:spreadsheet/engine` alone. Its
/// calls are made through [`crate::typed::call`], so only the types are used.
#[allow(
    missing_docs,
    unreachable_pub,
    dead_code,
    clippy::all,
    reason = "generated code, of which only the types are named"
)]
mod bindings {
    wasmtime::component::bindgen!({
        path: ["../../wit/nvs-ext", "../../wit/spreadsheet.wit"],
        inline: "
            package nvs:typed;

            world spreadsheet {
                export nvs:spreadsheet/engine@1.0.0;
            }
        ",
        world: "nvs:typed/spreadsheet",
    });
}

impl From<WitError> for Error {
    fn from(err: WitError) -> Self {
        match err {
            WitError::Invalid(message) => Self::Invalid(message),
            WitError::Parse(message) => Self::Parse(message),
            WitError::Runtime(message) => Self::Runtime(message),
        }
    }
}

world! {
    error: WitError;
    writer_open(WorkbookOptions) -> ResourceAny;
    writer_style(ResourceAny, Style) -> u64;
    writer_sheet(ResourceAny, SheetOptions, Vec<Row>) -> Vec<u8>;
    writer_rows(ResourceAny, Vec<Row>) -> Vec<u8>;
    writer_part(ResourceAny, SheetPart) -> Vec<u8>;
    writer_define(ResourceAny, String, String) -> ();
    writer_cell(ResourceAny, u64, u64) -> String;
    writer_column(ResourceAny, u64) -> String;
    writer_finish(ResourceAny) -> Finished;
    reader_open(ResourceAny, DecompressionBound) -> ResourceAny;
    reader_sheets(ResourceAny, ResourceAny) -> Vec<SheetInfo>;
    reader_header(ResourceAny, ResourceAny, ReadOptions) -> Vec<String>;
    reader_rows(ResourceAny, ResourceAny, ReadOptions, u64) -> ReadBatch;
    reader_merged(ResourceAny, ResourceAny, String) -> Vec<String>;
    reader_links(ResourceAny, ResourceAny, String) -> Vec<LinkInfo>;
    reader_notes(ResourceAny, ResourceAny, String) -> Vec<NoteInfo>;
    reader_names(ResourceAny, ResourceAny) -> Vec<NameInfo>;
    reader_tables(ResourceAny, ResourceAny, String) -> Vec<TableInfo>;
    reader_images(ResourceAny, ResourceAny, String) -> Vec<ImageInfo>;
    reader_properties(ResourceAny, ResourceAny) -> Properties;
    reader_macros(ResourceAny, ResourceAny) -> Vec<String>;
    source_bytes(ResourceAny, u64, Option<u64>) -> Vec<u8>;
    template_open(ResourceAny, DecompressionBound) -> ResourceAny;
    template_set(ResourceAny, String, Option<CellValue>) -> ();
    template_fill(ResourceAny, Vec<Placeholder>, Vec<String>) -> Filled;
    template_items(ResourceAny, ItemBatch) -> Vec<u8>;
    template_finish(ResourceAny, ResourceAny) -> Finished;
}

records! {
    TextCell { text }
    NumberCell { number }
    IntegerCell { integer }
    DecimalCell { decimal }
    BoolCell { boolean }
    DateCell { date }
    TimeCell { time }
    DurationCell { duration }
    ClockCell { day, clock }
    FormulaCell { formula, cached_text, cached_number }
    RichRun { text, style }
    RichCell { runs }
    Cell { value, style }
    Row { cells, style, height, hidden, level, header }
    Border { top, right, bottom, left }
    Style {
        font, size, bold, italic, strike, underline, color, fill, border, border_color, align,
        valign, wrap, rotate, indent, format, locked, hide_formula,
    }
    Properties { title, subject, author, company, keywords, created }
    WorkbookOptions { properties, target }
    Column { width, auto, style, format, hidden }
    HeaderOptions { default_style, style, freeze, filter }
    SheetOptions {
        name, position, visibility, columns, freeze_rows, freeze_columns, autofilter, tab_color,
        zoom, gridlines, right_to_left, header, new_sheet_on_overflow,
    }
    MergePart { merge }
    LinkPart { anchor, target, text, tooltip }
    NotePart { noted, note, author }
    ValidationPart {
        validated, kind, operator, values, input_title, input_message, error_title,
        error_message, allow_empty,
    }
    GroupPart { grouped, collapsed }
    TableTotal { column, function, formula }
    TablePart { table, name, style, banded, filter, totals }
    ConditionalPart { formatted, kind, operator, values, colors, icons, rank, percent, style }
    ImagePart { at, image, width, height, offset_x, offset_y }
    ChartSeries { values, categories, name, color }
    ChartLayer { kind, series }
    ChartPart { placed, layers, title, x_title, y_title, legend, labels, width, height }
    Margins { top, right, bottom, left }
    PrintPart {
        orientation, fit_width, fit_height, margins, header, footer, repeat_rows, area,
    }
    ProtectPart { password, allow }
    SheetRows { name, rows }
    Finished { data, rows, size }
    SheetInfo { name, position, visibility, rows, columns, kind }
    ReadOptions {
        sheet, header_row, from, limit, columns, text, text_all, formulas, keep_empty_rows,
        fill_merged, skip_hidden, locale,
    }
    ErrorCell { error }
    ReadCell { column, value, formula, display }
    ReadRow { number, cells }
    ReadBatch { rows, done }
    LinkInfo { anchor, target, text, tooltip, external }
    NoteInfo { noted, note, author }
    NameInfo { name, reference, sheet }
    TableInfo { table, name, sheet }
    ImageInfo { at, image, format, link }
    DecompressionBound { max_bytes, max_ratio }
    Placeholder { path, value }
    ItemBatch { name, items }
    Filled { data, lists }
}

enums! {
    Line [Thin, Medium, Thick, Dashed, Dotted, Double]
    Underline [Single, Double]
    Align [Left, Center, Right, Fill, Justify]
    Valign [Top, Center, Bottom]
    Visibility [Visible, Hidden, VeryHidden]
    RuleKind [List, Whole, Decimal, Date, Length, Formula]
    Operator [
        Between, NotBetween, Equal, NotEqual, Greater, GreaterOrEqual, Less, LessOrEqual,
    ]
    TotalFunction [Sum, Average, Count, Min, Max]
    ConditionalKind [
        Cell, Formula, ColorScale, DataBar, IconSet, Top, Bottom, Duplicates, Unique,
    ]
    ChartKind [Column, Bar, Line, Area, Pie, Doughnut, Scatter]
    Legend [Hidden, Top, Right, Bottom, Left]
    Orientation [Portrait, Landscape]
    SheetKind [Worksheet, Chartsheet]
    CellError [Null, DivZero, Value, Ref, Name, Num, NotAvailable, GettingData, Spill, Calc]
}

variants! {
    CellValue [
        Text, Number, Integer, Decimal, Boolean, Date, Time, Duration, Clock, Formula, Rich,
    ]
    SheetPart [
        Merge, Link, Note, Validation, Group, Table, Conditional, Image, Chart, Print, Protect,
    ]
    ReadValue [Text, Integer, Number, Boolean, Clock, Date, Time, Duration, Error]
}

cores! {
    Date { year, month, day }
    TimeOfDay { hour, minute, second, nanos }
    Duration { nanos }
}
