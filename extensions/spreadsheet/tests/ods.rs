//! The ods writer: the package, each value type, formulas, styles, columns and the header
//! defaults, read back from the written archive by our reader and by calamine
//! (`rule:core-classes/spreadsheet-evaluation-and-roster`,
//! `rule:core-classes/spreadsheet-writes-as-a-stream`).

use std::io::Cursor;

use calamine::{Data, Ods, Reader as _, open_workbook_from_rs};
use miniz_oxide::inflate::decompress_to_vec;
use nvs_spreadsheet::ods::Writer;
use nvs_spreadsheet::read::zip::Bound;
use nvs_spreadsheet::read::{Read, ReadCell, ReadOptions, ReadRow, Reader, Source};
use nvs_spreadsheet::{
    Cell, Clock, Column, Date, Error, Header, RichRun, Row, SheetOptions, Style, TimeOfDay, Value,
    WorkbookOptions,
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
