//! The ods writer: the package, each value type, formulas, styles, columns and the header
//! defaults, read back from the written archive by our reader and by calamine
//! (`rule:core-classes/spreadsheet-evaluation-and-roster`,
//! `rule:core-classes/spreadsheet-writes-as-a-stream`).

use std::io::Cursor;

use calamine::{Data, Ods, Reader as _, open_workbook_from_rs};
use miniz_oxide::inflate::decompress_to_vec;
use nvs_spreadsheet::chart::{self, Chart, Layer, Legend, Series};
use nvs_spreadsheet::conditional::{Conditional, Kind};
use nvs_spreadsheet::drawing::Image;
use nvs_spreadsheet::ods::Writer;
use nvs_spreadsheet::read::merge::Merge;
use nvs_spreadsheet::read::zip::Bound;
use nvs_spreadsheet::read::{Read, ReadCell, ReadOptions, ReadRow, Reader, Source};
use nvs_spreadsheet::table::{Function, Table, Total};
use nvs_spreadsheet::tail::{
    Group, Link, Margins, Note, Operator, Orientation, Part, Print, Protect, RuleKind, Validation,
};
use nvs_spreadsheet::{
    Cell, Clock, Column, Date, Error, Header, RichRun, Row, SheetOptions, Style, TimeOfDay, Value,
    Visibility, WorkbookOptions,
};

const MIME: &str = "application/vnd.oasis.opendocument.spreadsheet";

/// A whole file as a [`Source`].
struct Bytes<'a>(&'a [u8]);

impl Source for Bytes<'_> {
    fn size(&self) -> Result<u64, Error> {
        self.0.size()
    }

    fn range(&self, offset: u64, length: u64) -> Result<Vec<u8>, Error> {
        self.0.range(offset, length)
    }
}

fn cell(value: Value) -> Cell {
    Cell {
        value: Some(value),
        style: None,
    }
}

fn styled(value: Value, style: u64) -> Cell {
    Cell {
        value: Some(value),
        style: Some(style),
    }
}

fn text(value: &str) -> Cell {
    cell(Value::Text(value.to_owned()))
}

fn number(value: f64) -> Cell {
    cell(Value::Number(value))
}

fn row(cells: Vec<Cell>) -> Row {
    Row {
        cells,
        ..Row::default()
    }
}

fn header(names: &[&str]) -> Row {
    Row {
        cells: names.iter().map(|name| text(name)).collect(),
        header: true,
        ..Row::default()
    }
}

fn writer() -> Writer {
    Writer::open(WorkbookOptions {
        target: "report.ods".to_owned(),
        ods: true,
        ..WorkbookOptions::default()
    })
    .unwrap()
}

fn time(hour: i64, minute: i64, second: i64) -> TimeOfDay {
    TimeOfDay {
        hour,
        minute,
        second,
        nanos: 0,
    }
}

fn date(year: i64, month: i64, day: i64) -> Date {
    Date { year, month, day }
}

/// The whole file `writer` writes for the sheet `name` and its rows.
fn written(mut writer: Writer, name: &str, rows: &[Row]) -> Vec<u8> {
    writer.sheet(SheetOptions::named(name), rows).unwrap();
    writer.finish().unwrap().data
}

/// One entry of the archive: its name, its method, and its bytes as stored.
struct Entry {
    name: String,
    method: u16,
    local: usize,
    data: Vec<u8>,
}

/// The entries of the zip archive `archive`, in the order of its central directory.
fn entries(archive: &[u8]) -> Vec<Entry> {
    let u16_at = |at: usize| u16::from_le_bytes([archive[at], archive[at + 1]]);
    let u32_at = |at: usize| u32::from_le_bytes(archive[at..at + 4].try_into().unwrap());
    let end = archive.len() - 22;
    assert_eq!(
        u32_at(end),
        0x0605_4b50,
        "the archive ends in its end record"
    );
    let count = u16_at(end + 10);
    let mut at = u32_at(end + 16) as usize;
    let mut out = Vec::new();
    for _ in 0..count {
        assert_eq!(u32_at(at), 0x0201_4b50, "a directory entry");
        let method = u16_at(at + 10);
        let packed = u32_at(at + 20) as usize;
        let name_len = usize::from(u16_at(at + 28));
        let local = u32_at(at + 42) as usize;
        let name = String::from_utf8(archive[at + 46..at + 46 + name_len].to_vec()).unwrap();
        let data = local + 30 + usize::from(u16_at(local + 26)) + usize::from(u16_at(local + 28));
        out.push(Entry {
            name,
            method,
            local,
            data: archive[data..data + packed].to_vec(),
        });
        at += 46 + name_len + usize::from(u16_at(at + 30)) + usize::from(u16_at(at + 32));
    }
    out
}

/// The part `name` of the archive, inflated, as text.
fn part(archive: &[u8], name: &str) -> String {
    let entry = entries(archive)
        .into_iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("the archive has {name}"));
    let bytes = match entry.method {
        0 => entry.data,
        _ => decompress_to_vec(&entry.data).unwrap(),
    };
    String::from_utf8(bytes).unwrap()
}

/// Every row of the sheet `name`, as our reader reads it, formulas included.
fn read(archive: &[u8], name: &str) -> Vec<ReadRow> {
    let source = Bytes(archive);
    let mut reader = Reader::open(&source, Bound::default()).unwrap();
    let options = ReadOptions {
        sheet: name.to_owned(),
        formulas: true,
        ..ReadOptions::default()
    };
    let batch = reader.rows(&source, &options, 10_000).unwrap();
    assert!(batch.done, "one batch reads the whole sheet");
    batch.rows
}

/// The cell at `row` and `column`, both from 1.
fn at(rows: &[ReadRow], row: u64, column: u64) -> &ReadCell {
    rows.iter()
        .find(|read| read.number == row)
        .and_then(|read| read.cells.iter().find(|cell| cell.column == column))
        .unwrap_or_else(|| panic!("row {row} has a cell in column {column}"))
}

/// How many times `needle` is in `haystack`.
fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn an_ods_is_a_package_with_its_mimetype_first_and_stored() {
    let archive = written(writer(), "Data", &[row(vec![text("a")])]);
    assert_eq!(
        &archive[..4],
        b"PK\x03\x04",
        "the archive starts with a local header"
    );
    assert_eq!(
        &archive[30..38],
        b"mimetype",
        "the first entry is the mimetype"
    );
    assert_eq!(&archive[38..38 + MIME.len()], MIME.as_bytes());
    let found = entries(&archive);
    let names: Vec<&str> = found.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "mimetype",
            "content.xml",
            "styles.xml",
            "meta.xml",
            "settings.xml",
            "META-INF/manifest.xml"
        ]
    );
    assert_eq!(found[0].method, 0, "the mimetype is stored");
    assert_eq!(found[0].local, 0);
    assert_eq!(found[0].data, MIME.as_bytes());
    assert!(found[1..].iter().all(|entry| entry.method == 8));
    let manifest = part(&archive, "META-INF/manifest.xml");
    for name in ["content.xml", "styles.xml", "meta.xml", "settings.xml"] {
        assert!(
            manifest.contains(&format!("manifest:full-path=\"{name}\"")),
            "the manifest lists {name}: {manifest}"
        );
    }
    assert!(manifest.contains(&format!("manifest:media-type=\"{MIME}\"")));
    let source = Bytes(&archive);
    let reader = Reader::open(&source, Bound::default()).unwrap();
    let sheets = reader.sheets(&source).unwrap();
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0].name, "Data");
}

#[test]
fn every_value_type_writes_as_its_ods_type() {
    let values = vec![
        text("two  spaces"),
        number(1.5),
        cell(Value::Integer(42)),
        cell(Value::Integer(1_234_567_890_123_456_789)),
        cell(Value::Decimal("12.25".to_owned())),
        cell(Value::Boolean(true)),
        cell(Value::Date(date(2024, 1, 15))),
        cell(Value::Time(time(10, 30, 0))),
        cell(Value::Duration(26 * 3_600 * 1_000_000_000)),
        cell(Value::Clock(Clock {
            day: date(2024, 1, 15),
            clock: time(9, 5, 0),
        })),
        cell(Value::Rich(vec![
            RichRun {
                text: "plain ".to_owned(),
                style: None,
            },
            RichRun {
                text: "bold".to_owned(),
                style: Some(0),
            },
        ])),
    ];
    let mut book = writer();
    let bold = book
        .style(Style {
            bold: Some(true),
            ..Style::default()
        })
        .unwrap();
    let mut values = values;
    if let Some(Value::Rich(runs)) = &mut values[10].value {
        runs[1].style = Some(bold);
    }
    let archive = written(book, "Data", &[row(values)]);
    let content = part(&archive, "content.xml");
    for expected in [
        "office:value-type=\"string\"><text:p>two <text:s/>spaces</text:p>",
        "office:value-type=\"float\" office:value=\"1.5\"",
        "office:value-type=\"float\" office:value=\"42\"",
        "office:value-type=\"string\"><text:p>1234567890123456789</text:p>",
        "office:value-type=\"float\" office:value=\"12.25\"",
        "office:value-type=\"boolean\" office:boolean-value=\"true\"",
        "office:value-type=\"date\" office:date-value=\"2024-01-15\"",
        "office:value-type=\"time\" office:time-value=\"PT10H30M00S\"",
        "office:value-type=\"time\" office:time-value=\"PT26H00M00S\"",
        "office:value-type=\"date\" office:date-value=\"2024-01-15T09:05:00\"",
        "<text:p>plain<text:s/><text:span text:style-name=\"T1\">bold</text:span></text:p>",
    ] {
        assert!(
            content.contains(expected),
            "content.xml has {expected}: {content}"
        );
    }
    assert!(content.contains("<style:style style:name=\"T1\" style:family=\"text\">"));

    let rows = read(&archive, "Data");
    let values: Vec<Option<Read>> = (1..=11)
        .map(|column| at(&rows, 1, column).value.clone())
        .collect();
    assert_eq!(
        values,
        [
            Some(Read::Text("two  spaces".to_owned())),
            Some(Read::Number(1.5)),
            Some(Read::Integer(42)),
            Some(Read::Text("1234567890123456789".to_owned())),
            Some(Read::Number(12.25)),
            Some(Read::Boolean(true)),
            Some(Read::Date(date(2024, 1, 15))),
            Some(Read::Time(time(10, 30, 0))),
            Some(Read::Duration(26 * 3_600 * 1_000_000_000)),
            Some(Read::Clock(Clock {
                day: date(2024, 1, 15),
                clock: time(9, 5, 0),
            })),
            Some(Read::Text("plain bold".to_owned())),
        ]
    );
}

#[test]
fn a_formula_is_written_as_openformula_with_its_cached_value() {
    let rows = [
        row(vec![number(1.0), number(2.0)]),
        row(vec![
            cell(Value::Formula {
                formula: "SUM(A1:B1)".to_owned(),
                cached_text: None,
                cached_number: Some(3.0),
            }),
            cell(Value::Formula {
                formula: "IF(A1>0,\"yes\",\"no\")".to_owned(),
                cached_text: Some("yes".to_owned()),
                cached_number: None,
            }),
            cell(Value::Formula {
                formula: "Data!A1*2".to_owned(),
                cached_text: None,
                cached_number: None,
            }),
        ]),
    ];
    let archive = written(writer(), "Data", &rows);
    let content = part(&archive, "content.xml");
    for expected in [
        "table:formula=\"of:=SUM([.A1:.B1])\" office:value-type=\"float\" office:value=\"3\"><text:p>3</text:p>",
        "table:formula=\"of:=IF([.A1]&gt;0;&quot;yes&quot;;&quot;no&quot;)\" office:value-type=\"string\" office:string-value=\"yes\"><text:p>yes</text:p>",
        "table:formula=\"of:=[$Data.A1]*2\">",
    ] {
        assert!(
            content.contains(expected),
            "content.xml has {expected}: {content}"
        );
    }

    let read = read(&archive, "Data");
    let sum = at(&read, 2, 1);
    assert_eq!(sum.formula.as_deref(), Some("SUM(A1:B1)"));
    assert_eq!(sum.value, Some(Read::Integer(3)));
    let choice = at(&read, 2, 2);
    assert_eq!(choice.formula.as_deref(), Some("IF(A1>0,\"yes\",\"no\")"));
    assert_eq!(choice.value, Some(Read::Text("yes".to_owned())));
    assert_eq!(at(&read, 2, 3).formula.as_deref(), Some("Data!A1*2"));
}

#[test]
fn equal_style_combinations_share_one_ods_style() {
    let mut book = writer();
    let bold = book
        .style(Style {
            bold: Some(true),
            ..Style::default()
        })
        .unwrap();
    let money = book
        .style(Style {
            format: Some("#,##0.00".to_owned()),
            ..Style::default()
        })
        .unwrap();
    let bold_again = book
        .style(Style {
            bold: Some(true),
            ..Style::default()
        })
        .unwrap();
    assert_eq!(bold, bold_again, "an equal layer has the same id");
    let rows = [
        row(vec![
            styled(Value::Text("a".to_owned()), bold),
            styled(Value::Number(1.0), money),
        ]),
        row(vec![
            styled(Value::Text("b".to_owned()), bold),
            styled(Value::Number(2.0), money),
        ]),
        row(vec![
            styled(Value::Text("c".to_owned()), bold_again),
            styled(Value::Number(3.0), money),
        ]),
    ];
    let archive = written(book, "Data", &rows);
    let content = part(&archive, "content.xml");
    assert_eq!(
        count(&content, "style:family=\"table-cell\""),
        2,
        "two combinations, two cell styles: {content}"
    );
    assert_eq!(
        count(&content, "<number:number-style "),
        1,
        "one number style: {content}"
    );
    assert_eq!(count(&content, "table:style-name=\"ce1\""), 3);
    assert_eq!(count(&content, "table:style-name=\"ce2\""), 3);
    assert!(content.contains("fo:font-weight=\"bold\""));
    assert!(content.contains("style:data-style-name=\"N1\""));
    assert!(content.contains("number:grouping=\"true\""));

    let read = read(&archive, "Data");
    assert_eq!(at(&read, 3, 2).value, Some(Read::Integer(3)));
}

#[test]
fn columns_header_defaults_and_freeze_are_written_to_ods() {
    let mut book = writer();
    let options = SheetOptions {
        columns: vec![
            Column {
                width: Some(20.0),
                ..Column::default()
            },
            Column {
                hidden: true,
                ..Column::default()
            },
        ],
        freeze_columns: 1,
        zoom: Some(150),
        gridlines: false,
        header: Header::default(),
        ..SheetOptions::named("Report")
    };
    let rows = [
        header(&["Name", "Qty"]),
        row(vec![text("a"), number(1.0)]),
        row(vec![text("b"), number(2.0)]),
    ];
    book.sheet(options, &rows).unwrap();
    let finished = book.finish().unwrap();
    assert_eq!(finished.rows[0].rows, 2, "the header row is not counted");
    let archive = finished.data;
    let content = part(&archive, "content.xml");
    for expected in [
        "style:name=\"co1\" style:family=\"table-column\"><style:table-column-properties fo:break-before=\"auto\" style:column-width=\"1.5104in\"/>",
        "<table:table-column table:style-name=\"co1\"/>",
        "table:visibility=\"collapse\"/>",
        "fo:font-weight=\"bold\"",
        "fo:border-bottom=\"",
        "<table:database-range table:name=\"__Anonymous_Sheet_DB__0\" table:target-range-address=\"Report.A1:Report.B3\" table:display-filter-buttons=\"true\"/>",
    ] {
        assert!(
            content.contains(expected),
            "content.xml has {expected}: {content}"
        );
    }
    let settings = part(&archive, "settings.xml");
    for expected in [
        "<config:config-item config:name=\"VerticalSplitPosition\" config:type=\"int\">1</config:config-item>",
        "<config:config-item config:name=\"HorizontalSplitPosition\" config:type=\"int\">1</config:config-item>",
        "<config:config-item config:name=\"ZoomValue\" config:type=\"int\">150</config:config-item>",
        "<config:config-item config:name=\"ShowGrid\" config:type=\"boolean\">false</config:config-item>",
        "<config:config-item config:name=\"ActiveTable\" config:type=\"string\">Report</config:config-item>",
    ] {
        assert!(
            settings.contains(expected),
            "settings.xml has {expected}: {settings}"
        );
    }

    let read = read(&archive, "Report");
    assert_eq!(at(&read, 1, 1).value, Some(Read::Text("Name".to_owned())));
    assert_eq!(at(&read, 3, 2).value, Some(Read::Integer(2)));
    let source = Bytes(&archive);
    let reader = Reader::open(&source, Bound::default()).unwrap();
    let names: Vec<String> = reader
        .tables(&source, "Report")
        .unwrap()
        .into_iter()
        .map(|table| table.name)
        .collect();
    assert!(
        names.is_empty(),
        "the sheet's own filter is not a table: {names:?}"
    );
}

#[test]
fn two_ods_writes_of_equal_input_are_byte_equal() {
    let make = || {
        let mut book = writer();
        let bold = book
            .style(Style {
                bold: Some(true),
                ..Style::default()
            })
            .unwrap();
        book.define("Total", "Data!$B$2").unwrap();
        book.sheet(
            SheetOptions::named("Data"),
            &[
                header(&["Name", "Qty"]),
                row(vec![styled(Value::Text("a".to_owned()), bold), number(1.5)]),
            ],
        )
        .unwrap();
        book.sheet(
            SheetOptions::named("Other"),
            &[row(vec![cell(Value::Date(date(2024, 1, 15)))])],
        )
        .unwrap();
        book.finish().unwrap().data
    };
    let first = make();
    assert_eq!(first, make());
    assert!(
        part(&first, "meta.xml")
            .contains("<meta:creation-date>1980-01-01T00:00:00Z</meta:creation-date>")
    );
    assert!(part(&first, "content.xml").contains("table:name=\"Total\""));
}

#[test]
fn calamine_reads_back_what_the_ods_writer_wrote() {
    let mut book = writer();
    book.sheet(
        SheetOptions::named("Data"),
        &[
            header(&["Name", "Qty", "Paid", "Day", "Took"]),
            row(vec![
                text("a  b"),
                number(1.5),
                cell(Value::Boolean(true)),
                cell(Value::Date(date(2024, 1, 15))),
                cell(Value::Duration(90 * 60 * 1_000_000_000)),
            ]),
            row(vec![
                text("line one\nline two"),
                cell(Value::Formula {
                    formula: "B2*2".to_owned(),
                    cached_text: None,
                    cached_number: Some(3.0),
                }),
            ]),
        ],
    )
    .unwrap();
    book.sheet(SheetOptions::named("My sheet"), &[row(vec![number(7.0)])])
        .unwrap();
    let archive = book.finish().unwrap().data;

    let mut theirs: Ods<_> = open_workbook_from_rs(Cursor::new(archive)).unwrap();
    assert_eq!(theirs.sheet_names(), ["Data", "My sheet"]);
    let range = theirs.worksheet_range("Data").unwrap();
    assert_eq!(
        range.get_value((0, 1)),
        Some(&Data::String("Qty".to_owned()))
    );
    assert_eq!(
        range.get_value((1, 0)),
        Some(&Data::String("a  b".to_owned()))
    );
    assert_eq!(range.get_value((1, 1)), Some(&Data::Float(1.5)));
    assert_eq!(range.get_value((1, 2)), Some(&Data::Bool(true)));
    assert_eq!(
        range.get_value((1, 3)),
        Some(&Data::DateTimeIso("2024-01-15".to_owned()))
    );
    assert_eq!(
        range.get_value((1, 4)),
        Some(&Data::DurationIso("PT01H30M00S".to_owned()))
    );
    assert_eq!(
        range.get_value((2, 0)),
        Some(&Data::String("line one\nline two".to_owned()))
    );
    assert_eq!(range.get_value((2, 1)), Some(&Data::Float(3.0)));
    let formulas = theirs.worksheet_formula("Data").unwrap();
    assert_eq!(
        formulas.get_value((2, 1)).map(String::as_str),
        Some("of:=[.B2]*2")
    );
    let other = theirs.worksheet_range("My sheet").unwrap();
    assert_eq!(other.get_value((0, 0)), Some(&Data::Float(7.0)));
}

/// Asserts that `haystack`, the part `name`, has each of `needles`.
fn has_all(haystack: &str, name: &str, needles: &[&str]) {
    for needle in needles {
        assert!(haystack.contains(needle), "{name} has {needle}: {haystack}");
    }
}

/// The header of a PNG `width` by `height` pixels: its signature and `IHDR` chunk.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
    bytes.extend(width.to_be_bytes());
    bytes.extend(height.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0]);
    bytes
}

fn link(anchor: &str, target: &str, text: Option<&str>, tooltip: Option<&str>) -> Part {
    Part::Link(Link {
        anchor: anchor.to_owned(),
        target: target.to_owned(),
        text: text.map(str::to_owned),
        tooltip: tooltip.map(str::to_owned),
    })
}

fn note(noted: &str, text: &str, author: Option<&str>) -> Part {
    Part::Note(Note {
        noted: noted.to_owned(),
        note: text.to_owned(),
        author: author.map(str::to_owned),
    })
}

fn rule(validated: &str, kind: RuleKind, operator: Option<Operator>, values: &[&str]) -> Part {
    Part::Validation(Validation {
        validated: validated.to_owned(),
        kind,
        operator,
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        input_title: None,
        input_message: None,
        error_title: None,
        error_message: None,
        allow_empty: true,
    })
}

fn format_rule(formatted: &str, kind: Kind, style: Option<Style>) -> Conditional {
    Conditional {
        formatted: formatted.to_owned(),
        kind,
        operator: None,
        values: Vec::new(),
        colors: Vec::new(),
        icons: None,
        rank: None,
        percent: false,
        style,
    }
}

#[test]
fn ods_merges_links_and_notes_are_written_in_their_cells() {
    let mut book = writer();
    book.sheet(
        SheetOptions::named("Data"),
        &[
            row(vec![text("Title"), text("covered"), number(3.0)]),
            row(vec![text("site"), text("jump")]),
        ],
    )
    .unwrap();
    book.part(Part::Merge("A1:B1".to_owned())).unwrap();
    book.part(Part::Merge("D5:E6".to_owned())).unwrap();
    book.part(link(
        "A2",
        "https://example.com/docs",
        None,
        Some("Open the docs"),
    ))
    .unwrap();
    book.part(link("B2", "'Sheet 2'!A1", None, None)).unwrap();
    book.part(link("C8", "https://example.com", Some("Home"), None))
        .unwrap();
    book.part(note("C1", "Checked\nby hand", Some("Ann")))
        .unwrap();
    book.part(note("F9", "Empty cell", None)).unwrap();
    book.sheet(SheetOptions::named("Sheet 2"), &[row(vec![text("target")])])
        .unwrap();
    let archive = book.finish().unwrap().data;
    let content = part(&archive, "content.xml");
    has_all(
        &content,
        "content.xml",
        &[
            "table:number-columns-spanned=\"2\" table:number-rows-spanned=\"1\"",
            "<table:covered-table-cell office:value-type=\"string\"><text:p>covered</text:p></table:covered-table-cell>",
            "table:number-columns-spanned=\"2\" table:number-rows-spanned=\"2\"",
            "<text:a xlink:href=\"https://example.com/docs\" xlink:type=\"simple\" office:title=\"Open the docs\">site</text:a>",
            "<text:a xlink:href=\"#'Sheet 2'.A1\" xlink:type=\"simple\">jump</text:a>",
            "office:value-type=\"string\"><text:p><text:a xlink:href=\"https://example.com\" xlink:type=\"simple\">Home</text:a></text:p>",
            "<office:annotation><dc:creator>Ann</dc:creator><text:p>Checked</text:p><text:p>by hand</text:p></office:annotation>",
            "<table:table-cell><office:annotation><text:p>Empty cell</text:p></office:annotation></table:table-cell>",
        ],
    );

    let source = Bytes(&archive);
    let reader = Reader::open(&source, Bound::default()).unwrap();
    let merged = reader.merged(&source, "Data").unwrap();
    assert_eq!(
        merged,
        [
            Merge::parse(b"A1:B1").unwrap(),
            Merge::parse(b"D5:E6").unwrap()
        ]
    );
    let links = reader.links(&source, "Data").unwrap();
    let found: Vec<(&str, &str, bool, Option<&str>)> = links
        .iter()
        .map(|link| {
            (
                link.anchor.as_str(),
                link.target.as_str(),
                link.external,
                link.tooltip.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (
                "A2",
                "https://example.com/docs",
                true,
                Some("Open the docs")
            ),
            ("B2", "'Sheet 2'!A1", false, None),
            ("C8", "https://example.com", true, None),
        ]
    );
    assert_eq!(links[2].text.as_deref(), Some("Home"));
    let notes = reader.notes(&source, "Data").unwrap();
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].noted, "C1");
    assert_eq!(notes[0].note, "Checked\nby hand");
    assert_eq!(notes[0].author.as_deref(), Some("Ann"));
    assert_eq!(notes[1].noted, "F9");
    assert_eq!(notes[1].author, None);
    let rows = read(&archive, "Data");
    assert_eq!(
        at(&rows, 1, 2).value,
        Some(Read::Text("covered".to_owned()))
    );
    assert_eq!(at(&rows, 2, 1).value, Some(Read::Text("site".to_owned())));
    assert_eq!(at(&rows, 8, 3).value, Some(Read::Text("Home".to_owned())));

    let mut theirs: Ods<_> = open_workbook_from_rs(Cursor::new(archive)).unwrap();
    let range = theirs.worksheet_range("Data").unwrap();
    assert_eq!(
        range.get_value((0, 0)),
        Some(&Data::String("Title".to_owned()))
    );
    assert_eq!(
        range.get_value((1, 0)),
        Some(&Data::String("site".to_owned()))
    );
    assert_eq!(range.get_value((0, 2)), Some(&Data::Float(3.0)));
}

#[test]
fn ods_validation_is_written_before_the_first_sheet() {
    let mut book = writer();
    book.sheet(
        SheetOptions::named("Data"),
        &[header(&["Paid", "Qty", "Size"])],
    )
    .unwrap();
    book.part(rule("A2:A1048576", RuleKind::List, None, &["Yes", "No"]))
        .unwrap();
    book.part(rule(
        "B2:B10",
        RuleKind::Whole,
        Some(Operator::Between),
        &["1", "10"],
    ))
    .unwrap();
    book.part(rule("C2:C5", RuleKind::List, None, &["=$F$1:$F$3"]))
        .unwrap();
    book.rows(&[row(vec![text("Yes"), number(2.0), text("S")])])
        .unwrap();
    let archive = book.finish().unwrap().data;
    let content = part(&archive, "content.xml");
    let validations = content
        .find("<table:content-validations>")
        .expect("content.xml has the validations");
    let first_sheet = content
        .find("<table:table ")
        .expect("content.xml has a sheet");
    assert!(
        validations < first_sheet,
        "the validations come before the first sheet: {content}"
    );
    has_all(
        &content,
        "content.xml",
        &[
            "<table:content-validation table:name=\"val1\" table:condition=\"of:cell-content-is-in-list(&quot;Yes&quot;;&quot;No&quot;)\" table:allow-empty-cell=\"true\" table:base-cell-address=\"Data.A2\" table:display-list=\"unsorted\">",
            "table:condition=\"of:cell-content-is-whole-number() and cell-content-is-between(1;10)\"",
            "table:condition=\"of:cell-content-is-in-list([.$F$1:.$F$3])\"",
            "table:content-validation-name=\"val1\"",
            "table:content-validation-name=\"val2\"",
            "table:content-validation-name=\"val3\"",
            "table:number-rows-repeated=",
        ],
    );
    assert!(
        content.len() < 20_000,
        "a validation down a whole column is a few elements: {} bytes",
        content.len()
    );
    let rows = read(&archive, "Data");
    assert_eq!(at(&rows, 2, 1).value, Some(Read::Text("Yes".to_owned())));
}

#[test]
fn ods_hidden_rows_columns_sheets_and_groups_are_written() {
    let mut book = writer();
    book.sheet(SheetOptions::named("Data"), &[]).unwrap();
    book.part(Part::Group(Group {
        grouped: "3:4".to_owned(),
        collapsed: true,
    }))
    .unwrap();
    book.part(Part::Group(Group {
        grouped: "B:C".to_owned(),
        collapsed: false,
    }))
    .unwrap();
    book.rows(&[
        row(vec![text("one")]),
        Row {
            cells: vec![text("two")],
            hidden: true,
            ..Row::default()
        },
        row(vec![text("three")]),
        row(vec![text("four")]),
        row(vec![text("five")]),
    ])
    .unwrap();
    let error = book
        .part(Part::Group(Group {
            grouped: "2:3".to_owned(),
            collapsed: false,
        }))
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("Group rows before you add them"),
        "{error:?}"
    );
    let error = book
        .part(Part::Group(Group {
            grouped: "E:F".to_owned(),
            collapsed: false,
        }))
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("Group columns before you add rows"),
        "{error:?}"
    );
    book.sheet(
        SheetOptions {
            visibility: Visibility::Hidden,
            columns: vec![Column {
                hidden: true,
                ..Column::default()
            }],
            ..SheetOptions::named("Secret")
        },
        &[row(vec![text("hidden")])],
    )
    .unwrap();
    let archive = book.finish().unwrap().data;
    let content = part(&archive, "content.xml");
    has_all(
        &content,
        "content.xml",
        &[
            "<table:table-row-group table:display=\"false\"><table:table-row table:visibility=\"collapse\">",
            "<table:table-column-group><table:table-column table:number-columns-repeated=\"2\"/></table:table-column-group>",
            "<table:table-row table:visibility=\"collapse\"><table:table-cell office:value-type=\"string\"><text:p>two</text:p>",
            "<table:table-column table:style-name=\"co1\" table:visibility=\"collapse\"/>",
            "<style:table-properties table:display=\"false\"",
        ],
    );

    let source = Bytes(&archive);
    let reader = Reader::open(&source, Bound::default()).unwrap();
    let sheets = reader.sheets(&source).unwrap();
    assert_eq!(sheets[0].visibility, Visibility::Visible);
    assert_eq!(sheets[1].name, "Secret");
    assert_eq!(sheets[1].visibility, Visibility::Hidden);
    let mut reader = Reader::open(&source, Bound::default()).unwrap();
    let options = ReadOptions {
        sheet: "Data".to_owned(),
        skip_hidden: true,
        ..ReadOptions::default()
    };
    let shown: Vec<u64> = reader
        .rows(&source, &options, 100)
        .unwrap()
        .rows
        .iter()
        .map(|row| row.number)
        .collect();
    assert_eq!(shown, [1, 5], "rows 2 to 4 are hidden");
}

#[test]
fn ods_tables_are_database_ranges_with_their_filter() {
    let mut book = writer();
    book.sheet(SheetOptions::named("Data"), &[]).unwrap();
    book.part(Part::Table(Table {
        range: "A1:D4".to_owned(),
        name: Some("Shop".to_owned()),
        style: None,
        banded: true,
        filter: true,
        totals: vec![Total {
            column: "qty".to_owned(),
            function: Some(Function::Sum),
            formula: None,
        }],
    }))
    .unwrap();
    let line = |at: u32| {
        cell(Value::Formula {
            formula: "[@qty]*[@price]".to_owned(),
            cached_text: None,
            cached_number: Some(f64::from(at)),
        })
    };
    book.rows(&[
        Row {
            cells: vec![text("item"), text("qty"), text("price"), text("line")],
            header: true,
            ..Row::default()
        },
        row(vec![text("pen"), number(1.0), number(2.0), line(2)]),
        row(vec![text("ink"), number(2.0), number(3.0), line(6)]),
        row(vec![text("pad"), number(3.0), number(1.0), line(3)]),
        row(vec![text("after")]),
    ])
    .unwrap();
    let error = book
        .define("shop", "Data!$A$1")
        .expect_err("a name a table has is taken");
    assert!(format!("{error:?}").contains("shop"), "{error:?}");
    let archive = book.finish().unwrap().data;
    let content = part(&archive, "content.xml");
    has_all(
        &content,
        "content.xml",
        &[
            "<table:database-range table:name=\"Shop\" table:target-range-address=\"Data.A1:Data.D4\" table:contains-header=\"true\" table:display-filter-buttons=\"true\"/>",
            "table:formula=\"of:=[.B2]*[.C2]\"",
            "table:formula=\"of:=SUBTOTAL(109;[.$B$2:.$B$4])\"",
        ],
    );
    assert!(!content.contains("#This Row"), "{content}");
    assert!(
        !content.contains("__Anonymous_Sheet_DB__"),
        "the table drops the sheet's own filter: {content}"
    );

    let source = Bytes(&archive);
    let reader = Reader::open(&source, Bound::default()).unwrap();
    let tables = reader.tables(&source, "Data").unwrap();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].name, "Shop");
    assert_eq!(tables[0].table, "A1:D4");
    let rows = read(&archive, "Data");
    assert_eq!(at(&rows, 5, 1).value, Some(Read::Text("Total".to_owned())));
    assert_eq!(
        at(&rows, 5, 2).formula.as_deref(),
        Some("SUBTOTAL(109,$B$2:$B$4)")
    );
    assert_eq!(at(&rows, 3, 4).formula.as_deref(), Some("B3*C3"));
    assert_eq!(at(&rows, 6, 1).value, Some(Read::Text("after".to_owned())));
}

#[test]
fn ods_conditional_formats_are_written() {
    let make = || {
        let mut book = writer();
        book.sheet(
            SheetOptions::named("Data"),
            &[row(vec![number(4.0), number(1.0)]), row(vec![number(9.0)])],
        )
        .unwrap();
        let fill = Style {
            fill: Some("#FFC7CE".to_owned()),
            ..Style::default()
        };
        let bold = Style {
            bold: Some(true),
            ..Style::default()
        };
        book.part(Part::Conditional(Conditional {
            operator: Some(Operator::Greater),
            values: vec!["5".to_owned()],
            ..format_rule("A1:A10", Kind::Cell, Some(fill.clone()))
        }))
        .unwrap();
        book.part(Part::Conditional(Conditional {
            values: vec!["=$B1>0".to_owned()],
            ..format_rule("A1:A10 C1:C10", Kind::Formula, Some(bold))
        }))
        .unwrap();
        book.part(Part::Conditional(format_rule(
            "B1:B10",
            Kind::ColorScale,
            None,
        )))
        .unwrap();
        book.part(Part::Conditional(format_rule(
            "B1:B10",
            Kind::DataBar,
            None,
        )))
        .unwrap();
        book.part(Part::Conditional(Conditional {
            icons: Some("3Arrows".to_owned()),
            ..format_rule("B1:B10", Kind::IconSet, None)
        }))
        .unwrap();
        book.part(Part::Conditional(format_rule(
            "A1:A10",
            Kind::Top,
            Some(fill),
        )))
        .unwrap();
        book.finish().unwrap().data
    };
    let archive = make();
    assert_eq!(archive, make(), "two writes are byte-equal");
    let content = part(&archive, "content.xml");
    has_all(
        &content,
        "content.xml",
        &[
            "<calcext:conditional-formats><calcext:conditional-format calcext:target-range-address=\"Data.A1:Data.A10\"><calcext:condition calcext:apply-style-name=\"Cond1\" calcext:value=\"&gt;5\" calcext:base-cell-address=\"Data.A1\"/>",
            "calcext:target-range-address=\"Data.A1:Data.A10 Data.C1:Data.C10\"><calcext:condition calcext:apply-style-name=\"Cond2\" calcext:value=\"formula-is([.$B1]&gt;0)\"",
            "<calcext:color-scale><calcext:color-scale-entry calcext:value=\"0\" calcext:type=\"minimum\" calcext:color=\"#F8696B\"/><calcext:color-scale-entry calcext:value=\"50\" calcext:type=\"percentile\" calcext:color=\"#FFEB84\"/><calcext:color-scale-entry calcext:value=\"0\" calcext:type=\"maximum\" calcext:color=\"#63BE7B\"/></calcext:color-scale>",
            "<calcext:data-bar calcext:positive-color=\"#638EC6\"><calcext:formatting-entry calcext:value=\"0\" calcext:type=\"auto-minimum\"/>",
            "<calcext:icon-set calcext:icon-set-type=\"3Arrows\"><calcext:formatting-entry calcext:value=\"0\" calcext:type=\"percent\"/><calcext:formatting-entry calcext:value=\"33\" calcext:type=\"percent\"/><calcext:formatting-entry calcext:value=\"67\" calcext:type=\"percent\"/></calcext:icon-set>",
            "calcext:apply-style-name=\"Cond1\" calcext:value=\"top-elements(10)\"",
        ],
    );
    let styles = part(&archive, "styles.xml");
    has_all(
        &styles,
        "styles.xml",
        &[
            "<style:style style:name=\"Cond1\" style:family=\"table-cell\" style:parent-style-name=\"Default\"><style:table-cell-properties fo:background-color=\"#FFC7CE\"/>",
            "<style:style style:name=\"Cond2\" style:family=\"table-cell\" style:parent-style-name=\"Default\"><style:text-properties fo:font-weight=\"bold\"/>",
        ],
    );
    let mut theirs: Ods<_> = open_workbook_from_rs(Cursor::new(archive)).unwrap();
    let range = theirs.worksheet_range("Data").unwrap();
    assert_eq!(range.get_value((1, 0)), Some(&Data::Float(9.0)));
}

#[test]
fn ods_images_print_setup_and_protection_are_written() {
    let mut book = writer();
    book.sheet(SheetOptions::named("Data"), &[]).unwrap();
    book.part(Part::Print(Print {
        orientation: Some(Orientation::Landscape),
        fit_width: Some(1),
        fit_height: None,
        margins: Some(Margins {
            top: 1.0,
            right: 0.5,
            bottom: 1.0,
            left: 0.5,
        }),
        header: Some("Monthly report".to_owned()),
        footer: None,
        repeat_rows: Some("1:1".to_owned()),
        area: Some("A1:C20".to_owned()),
    }))
    .unwrap();
    book.rows(&[header(&["Name", "Qty"]), row(vec![text("a"), number(1.0)])])
        .unwrap();
    let image = png(20, 10);
    book.part(Part::Image(Image {
        at: "B3".to_owned(),
        image: image.clone(),
        width: Some(40.0),
        height: None,
        offset_x: Some(4.0),
        offset_y: None,
    }))
    .unwrap();
    book.part(Part::Protect(Protect {
        password: Some("password".to_owned()),
        allow: vec!["sort".to_owned()],
    }))
    .unwrap();
    let error = book
        .part(Part::Print(Print {
            fit_height: Some(2),
            ..Print::default()
        }))
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("after its first row"),
        "{error:?}"
    );
    let archive = book.finish().unwrap().data;
    let names: Vec<String> = entries(&archive)
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert!(names.contains(&"Pictures/1.png".to_owned()), "{names:?}");
    let manifest = part(&archive, "META-INF/manifest.xml");
    assert!(manifest.contains(
        "<manifest:file-entry manifest:full-path=\"Pictures/1.png\" manifest:media-type=\"image/png\"/>"
    ));
    let content = part(&archive, "content.xml");
    has_all(
        &content,
        "content.xml",
        &[
            "<table:table-cell><draw:frame draw:z-index=\"0\" draw:name=\"Image 1\" svg:width=\"0.4167in\" svg:height=\"0.2083in\" svg:x=\"0.0417in\" svg:y=\"0.0000in\"><draw:image xlink:href=\"Pictures/1.png\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/></draw:frame></table:table-cell>",
            "style:master-page-name=\"mp1\"",
            "table:protected=\"true\" table:protection-key=\"g68=\" table:protection-key-digest-algorithm=\"http://docs.oasis-open.org/office/ns/table/legacy-hash-excel\"",
            "table:print-ranges=\"Data.A1:Data.C20\"",
            "<table:table-header-rows><table:table-row table:default-cell-style-name=\"ce1\"><table:table-cell table:style-name=\"ce1\" office:value-type=\"string\"><text:p>Name</text:p>",
            "</table:table-row></table:table-header-rows><table:table-row>",
        ],
    );
    let styles = part(&archive, "styles.xml");
    has_all(
        &styles,
        "styles.xml",
        &[
            "<style:page-layout style:name=\"pm1\"><style:page-layout-properties fo:page-width=\"11in\" fo:page-height=\"8.5in\" style:print-orientation=\"landscape\" fo:margin-top=\"1in\" fo:margin-bottom=\"1in\" fo:margin-left=\"0.5in\" fo:margin-right=\"0.5in\" style:scale-to-X=\"1\" style:scale-to-Y=\"0\"/>",
            "<style:master-page style:name=\"mp1\" style:page-layout-name=\"pm1\"><style:header><text:p>Monthly report</text:p></style:header></style:master-page>",
        ],
    );

    let source = Bytes(&archive);
    let reader = Reader::open(&source, Bound::default()).unwrap();
    let images = reader.images(&source, "Data").unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].at, "B3");
    assert_eq!(images[0].image, image);
    assert_eq!(images[0].link, None);
}

#[test]
fn ods_number_formats_with_sections_conditions_and_fractions_are_written() {
    let mut book = writer();
    let mut cells = Vec::new();
    for (format, value) in [
        ("#,##0.00;(#,##0.00)", -3.5),
        ("[>=100]0;0.0", 250.0),
        ("# ?/?", 1.5),
    ] {
        let id = book
            .style(Style {
                format: Some(format.to_owned()),
                ..Style::default()
            })
            .unwrap();
        cells.push(styled(Value::Number(value), id));
    }
    let archive = written(book, "Data", &[row(cells)]);
    let content = part(&archive, "content.xml");
    has_all(
        &content,
        "content.xml",
        &[
            "style:condition=\"value()&gt;=0\" style:apply-style-name=\"N1P0\"",
            "style:condition=\"value()&gt;=100\" style:apply-style-name=\"N2P0\"",
            "<number:fraction ",
        ],
    );
    let rows = read(&archive, "Data");
    assert_eq!(at(&rows, 1, 3).value, Some(Read::Number(1.5)));
}

/// One chart layer's series, each as `(categories, values)`.
type Ranges<'a> = Vec<(Option<&'a str>, &'a str)>;

/// A chart at `placed`, one layer per `(kind, series)`.
fn ods_chart(placed: &str, layers: Vec<(chart::Kind, Ranges)>) -> Chart {
    Chart {
        placed: placed.to_owned(),
        layers: layers
            .into_iter()
            .map(|(kind, series)| Layer {
                kind,
                series: series
                    .into_iter()
                    .map(|(categories, values)| Series {
                        values: values.to_owned(),
                        categories: categories.map(str::to_owned),
                        name: None,
                        color: None,
                    })
                    .collect(),
            })
            .collect(),
        title: None,
        x_title: None,
        y_title: None,
        legend: Legend::Right,
        labels: false,
        width: None,
        height: None,
    }
}

/// The sheet `Data`: a header row and the rows `(name, qty)`.
fn chart_rows(book: &mut Writer, rows: &[(&str, f64)]) {
    let mut kept = vec![header(&["Name", "Qty"])];
    kept.extend(
        rows.iter()
            .map(|&(name, qty)| row(vec![text(name), number(qty)])),
    );
    book.sheet(SheetOptions::named("Data"), &kept).unwrap();
}

#[test]
fn every_chart_type_of_the_subset_is_written_to_ods() {
    use chart::Kind::{Area, Bar, Column, Doughnut, Line, Pie, Scatter};
    let mut book = writer();
    chart_rows(&mut book, &[("a", 1.0), ("b", 2.0)]);
    let reach = book.column(1).unwrap();
    assert_eq!(reach, "B2:B3");
    let cases = [
        (Column, "chart:bar"),
        (Bar, "chart:bar"),
        (Line, "chart:line"),
        (Area, "chart:area"),
        (Pie, "chart:circle"),
        (Doughnut, "chart:ring"),
        (Scatter, "chart:scatter"),
    ];
    for (at, &(kind, _)) in cases.iter().enumerate() {
        let mut one = ods_chart(
            &format!("D{}", at * 20 + 2),
            vec![(kind, vec![(Some("A2:A3"), reach.as_str())])],
        );
        if kind == Column {
            one.title = Some("Sales & costs".to_owned());
            one.x_title = Some("Month".to_owned());
            one.y_title = Some("Euro".to_owned());
            one.legend = Legend::Bottom;
            one.labels = true;
            one.width = Some(960.0);
            one.layers[0].series[0].name = Some("Qty \"all\"".to_owned());
            one.layers[0].series[0].color = Some("#FF0000".to_owned());
        }
        if kind == Line {
            one.legend = Legend::Hidden;
            one.layers[0].series[0].color = Some("#00FF00".to_owned());
        }
        book.part(Part::Chart(one)).unwrap();
    }
    // The rows after the charts grow the ranges that end at the column's last row.
    book.rows(&[row(vec![text("c"), number(3.0)])]).unwrap();
    let error = book
        .part(Part::Chart(ods_chart("D200", vec![(Pie, vec![])])))
        .unwrap_err();
    assert!(format!("{error:?}").contains("has no series"), "{error:?}");
    let archive = book.finish().unwrap().data;
    let names: Vec<String> = entries(&archive)
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    let manifest = part(&archive, "META-INF/manifest.xml");
    for (at, &(kind, class)) in cases.iter().enumerate() {
        let number = at + 1;
        let entry = format!("Object {number}/content.xml");
        assert!(names.contains(&entry), "{names:?}");
        has_all(
            &manifest,
            "META-INF/manifest.xml",
            &[
                &format!(
                    "manifest:full-path=\"Object {number}/\" manifest:version=\"1.3\" manifest:media-type=\"application/vnd.oasis.opendocument.chart\""
                ),
                &format!(
                    "manifest:full-path=\"Object {number}/content.xml\" manifest:media-type=\"text/xml\""
                ),
            ],
        );
        let object = part(&archive, &entry);
        has_all(
            &object,
            &entry,
            &[
                "<office:chart><chart:chart ",
                &format!("chart:class=\"{class}\">"),
                &format!("chart:series chart:style-name=\"s0\" chart:class=\"{class}\""),
                "chart:values-cell-range-address=\"Data.$B$2:Data.$B$4\"",
            ],
        );
        if kind == Scatter {
            has_all(
                &object,
                &entry,
                &[
                    "<chart:domain table:cell-range-address=\"Data.$A$2:Data.$A$4\"/>",
                    "draw:stroke=\"none\"",
                ],
            );
            assert!(!object.contains("chart:categories"), "{object}");
        } else {
            has_all(
                &object,
                &entry,
                &["<chart:categories table:cell-range-address=\"Data.$A$2:Data.$A$4\"/>"],
            );
        }
        let axes = !matches!(kind, Pie | Doughnut);
        assert_eq!(object.contains("chart:dimension=\"y\""), axes, "{object}");
        assert_eq!(object.contains("chart:vertical=\"true\""), kind == Bar);
        assert_eq!(object.contains("<chart:legend "), kind != Line, "{object}");
    }
    let column = part(&archive, "Object 1/content.xml");
    has_all(
        &column,
        "Object 1/content.xml",
        &[
            "svg:width=\"10.0000in\" svg:height=\"3.0000in\"",
            "<chart:title><text:p>Sales &amp; costs</text:p></chart:title>",
            "<chart:legend chart:legend-position=\"bottom\"/>",
            "<chart:axis chart:dimension=\"x\" chart:name=\"primary-x\"><chart:title><text:p>Month</text:p></chart:title>",
            "<chart:title><text:p>Euro</text:p></chart:title><chart:grid chart:class=\"major\"/>",
            "loext:label-string=\"Qty &quot;all&quot;\"",
            "chart:data-label-number=\"value\"",
            "draw:fill-color=\"#FF0000\"",
        ],
    );
    let line = part(&archive, "Object 3/content.xml");
    has_all(
        &line,
        "Object 3/content.xml",
        &["chart:symbol-type=\"none\"", "svg:stroke-color=\"#00FF00\""],
    );
    let content = part(&archive, "content.xml");
    has_all(
        &content,
        "content.xml",
        &[
            "<draw:frame draw:z-index=\"0\" draw:name=\"Chart 1\" svg:width=\"10.0000in\" svg:height=\"3.0000in\" svg:x=\"0in\" svg:y=\"0in\">\
             <draw:object draw:notify-on-update-of-ranges=\"Data.$A$2:Data.$A$4 Data.$B$2:Data.$B$4\" xlink:href=\"./Object 1\"",
            "draw:name=\"Chart 7\"",
        ],
    );
    assert_eq!(count(&content, "<draw:object "), 7);
    let rows = read(&archive, "Data");
    assert_eq!(at(&rows, 4, 2).value, Some(Read::Integer(3)));
    let mut calamine: Ods<_> = open_workbook_from_rs(Cursor::new(archive)).unwrap();
    let range = calamine.worksheet_range("Data").unwrap();
    assert_eq!(range.get_value((3, 1)), Some(&Data::Float(3.0)));
}

#[test]
fn a_combination_of_two_types_is_written_to_ods() {
    use chart::Kind::{Column, Line, Pie};
    let mut book = writer();
    chart_rows(&mut book, &[("a", 1.0), ("b", 2.0), ("c", 3.0)]);
    book.part(Part::Image(Image {
        at: "D2".to_owned(),
        image: png(20, 10),
        width: None,
        height: None,
        offset_x: None,
        offset_y: None,
    }))
    .unwrap();
    book.part(Part::Chart(ods_chart(
        "D2",
        vec![
            (Column, vec![(Some("A2:A4"), "B2:B4")]),
            (Line, vec![(None, "'Other sheet'!C2:C4")]),
        ],
    )))
    .unwrap();
    let error = book
        .part(Part::Chart(ods_chart(
            "D30",
            vec![(Pie, vec![(None, "B2:B4")]), (Line, vec![(None, "B2:B4")])],
        )))
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("combines pie and line"),
        "{error:?}"
    );
    let archive = book.finish().unwrap().data;
    let object = part(&archive, "Object 1/content.xml");
    has_all(
        &object,
        "Object 1/content.xml",
        &[
            "<chart:chart svg:width=\"5.0000in\" svg:height=\"3.0000in\" chart:class=\"chart:bar\">",
            "<chart:series chart:style-name=\"s0\" chart:class=\"chart:bar\" chart:values-cell-range-address=\"Data.$B$2:Data.$B$4\">",
            "<chart:series chart:style-name=\"s1\" chart:class=\"chart:line\" chart:values-cell-range-address=\"'Other sheet'.$C$2:'Other sheet'.$C$4\">",
            "<chart:categories table:cell-range-address=\"Data.$A$2:Data.$A$4\"/>",
        ],
    );
    assert_eq!(count(&object, "<chart:axis "), 2, "{object}");
    assert_eq!(count(&object, "<chart:plot-area "), 1, "{object}");
    // The image and the chart share the cell, in the order they were added.
    let content = part(&archive, "content.xml");
    let image = content
        .find("draw:z-index=\"0\" draw:name=\"Image 1\"")
        .unwrap();
    let chart = content
        .find("draw:z-index=\"1\" draw:name=\"Chart 1\"")
        .unwrap();
    assert!(image < chart, "{content}");
}
