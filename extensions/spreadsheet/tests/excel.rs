//! What opens in Excel (`rule:core-classes/spreadsheet-output-is-inert`): every file the writer
//! and the template filler make, read back by two readers that share no code with the writer.
//!
//! **The round trip.** A workbook written here, and a template filled here, are read back twice:
//! by the component's own reader ([`nvs_spreadsheet::read::Reader`]) and by calamine, a reader
//! written apart from this crate. Each value must read back as the type and the value it was
//! written as, by `rule:core-classes/spreadsheet-value-types`. calamine reads values, formulas,
//! merged ranges and hyperlinks. It does not read notes or data validation, so those are checked
//! through our reader and through the sheet's own XML.
//!
//! **Against Excel's own files.** Each file under `tests/fixtures/excel/` was made by Excel (its
//! README names where they come from). The writer makes the same workbook, and the parts an area
//! is about are compared element by element, split the way the suite those files come from
//! splits them: `xl/styles.xml` for cell formats, the table's part for tables, the sheet's
//! `<conditionalFormatting>` and `<dataValidations>` with the `dxf` styles they use, the drawing
//! part for images, and the chart part and the drawing for charts. A difference is fixed in the
//! writer. A fixture is never changed.
//!
//! **What the chart comparison removes**, all of it in [`excel_chart`]: the `numCache` and
//! `strCache` Excel writes beside each reference, which
//! `rule:core-classes/spreadsheet-sheet-end-parts` declines, and the axis ids, which Excel picks
//! at random and which are renumbered in both files by order of appearance, so each axis still
//! names its partner.

use std::io::Cursor;

use calamine::{Data, Dimensions, Reader as _, Xlsx, open_workbook_from_rs};
use nvs_spreadsheet::cells::reference;
use nvs_spreadsheet::chart::{self, Chart, Layer, Legend, Series};
use nvs_spreadsheet::conditional::{Conditional, Kind};
use nvs_spreadsheet::drawing::Image;
use nvs_spreadsheet::read::merge::Merge;
use nvs_spreadsheet::read::zip::{Bound, Directory};
use nvs_spreadsheet::read::{Read, ReadCell, ReadOptions, ReadRow, Reader, Source};
use nvs_spreadsheet::table::{Function, Table, Total};
use nvs_spreadsheet::tail::{Link, Note, Operator, Part, RuleKind, Validation, cell_at};
use nvs_spreadsheet::template::{Placeholder, Template};
use nvs_spreadsheet::{
    Align, Border, Cell, Clock, Date, Error, Header, Line, RichRun, Row, SheetOptions, Style,
    TimeOfDay, Valign, Value, WorkbookOptions, Writer,
};

// ---------------------------------------------------------------------------------------------
// The two readers.
// ---------------------------------------------------------------------------------------------

/// A whole file as a [`Source`].
struct Bytes(Vec<u8>);

impl Source for Bytes {
    fn size(&self) -> Result<u64, Error> {
        self.0.as_slice().size()
    }

    fn range(&self, offset: u64, length: u64) -> Result<Vec<u8>, Error> {
        self.0.as_slice().range(offset, length)
    }
}

/// The file `bytes` as our reader and calamine each open it.
struct Opened {
    source: Bytes,
    ours: Reader,
    theirs: Xlsx<Cursor<Vec<u8>>>,
}

impl Opened {
    fn new(bytes: Vec<u8>) -> Self {
        let source = Bytes(bytes);
        let ours = Reader::open(&source, Bound::default()).unwrap();
        let theirs = open_workbook_from_rs(Cursor::new(source.0.clone())).unwrap();
        Self {
            source,
            ours,
            theirs,
        }
    }

    /// Every row of the sheet `name` by our reader, formulas included.
    fn rows(&mut self, name: &str) -> Vec<ReadRow> {
        let options = ReadOptions {
            sheet: name.to_owned(),
            formulas: true,
            ..ReadOptions::default()
        };
        let batch = self.ours.rows(&self.source, &options, 10_000).unwrap();
        assert!(batch.done, "one batch reads the whole sheet");
        batch.rows
    }

    /// The sheet `name`'s part, as text.
    fn part(&self, name: &str) -> String {
        let directory = Directory::read(&self.source, Bound::default()).unwrap();
        let entry = directory
            .find(name)
            .unwrap_or_else(|| panic!("the file has {name}"));
        String::from_utf8(directory.whole(&self.source, entry).unwrap()).unwrap()
    }
}

/// The cell at `row` and `column`, both from 1, in rows our reader gave.
fn ours_at(rows: &[ReadRow], row: u64, column: u64) -> Option<&ReadCell> {
    rows.iter()
        .find(|read| read.number == row)?
        .cells
        .iter()
        .find(|cell| cell.column == column)
}

/// A merged range by its corners, both from 1, as our reader gives it.
fn merge(first_row: u64, first_column: u64, last_row: u64, last_column: u64) -> Merge {
    Merge {
        first_row,
        last_row,
        first_column,
        last_column,
    }
}

/// A merged range by its corners, both from 1, as calamine gives it.
fn dimensions(first_row: u32, first_column: u32, last_row: u32, last_column: u32) -> Dimensions {
    Dimensions::new(
        (first_row - 1, first_column - 1),
        (last_row - 1, last_column - 1),
    )
}

/// What calamine reads for one cell.
#[derive(Debug)]
enum Theirs<'a> {
    Text(&'a str),
    Number(f64),
    Bool(bool),
    /// A serial with a date or time format.
    DateTime(f64),
    /// A serial with an elapsed-time format.
    Duration(f64),
    Empty,
}

impl Theirs<'_> {
    fn check(&self, data: &Data, at: &str) {
        match (self, data) {
            (Theirs::Text(want), Data::String(got)) => assert_eq!(got, want, "{at}"),
            (Theirs::Number(want), Data::Float(got)) => assert_eq!(got, want, "{at}"),
            (Theirs::Number(want), Data::Int(got)) => assert_eq!(*got as f64, *want, "{at}"),
            (Theirs::Bool(want), Data::Bool(got)) => assert_eq!(got, want, "{at}"),
            (Theirs::DateTime(want), Data::DateTime(got)) => {
                assert!(got.is_datetime(), "{at} is a date or time: {got:?}");
                assert!((got.as_f64() - want).abs() < 1e-9, "{at}: {got:?}");
            }
            (Theirs::Duration(want), Data::DateTime(got)) => {
                assert!(got.is_duration(), "{at} is a duration: {got:?}");
                assert!((got.as_f64() - want).abs() < 1e-9, "{at}: {got:?}");
            }
            (Theirs::Empty, Data::Empty) => {}
            _ => panic!("{at}: calamine reads {data:?}, and the case wants {self:?}"),
        }
    }
}

/// Checks the cell `at` (such as `B4`) through both readers: our value and formula, and
/// calamine's value and formula.
fn check_cell(
    opened: &mut Opened,
    rows: &[ReadRow],
    sheet: &str,
    at: (u32, u32),
    ours: Option<Read>,
    formula: Option<&str>,
    theirs: &Theirs<'_>,
) {
    let (row, column) = at;
    let label = format!("{sheet}!R{row}C{column}");
    let read = ours_at(rows, u64::from(row), u64::from(column));
    assert_eq!(
        read.and_then(|cell| cell.value.clone()),
        ours,
        "{label} by our reader"
    );
    assert_eq!(
        read.and_then(|cell| cell.formula.as_deref()),
        formula,
        "{label}'s formula by our reader"
    );
    let range = opened.theirs.worksheet_range(sheet).unwrap();
    let data = range
        .get_value((row - 1, column - 1))
        .cloned()
        .unwrap_or(Data::Empty);
    theirs.check(&data, &label);
    let formulas = opened.theirs.worksheet_formula(sheet).unwrap();
    let written = formulas
        .get_value((row - 1, column - 1))
        .filter(|text| !text.is_empty())
        .map(String::as_str);
    assert_eq!(written, formula, "{label}'s formula by calamine");
}

// ---------------------------------------------------------------------------------------------
// Building the files.
// ---------------------------------------------------------------------------------------------

fn cell(value: Option<Value>) -> Cell {
    Cell { value, style: None }
}

fn text(value: &str) -> Cell {
    cell(Some(Value::Text(value.to_owned())))
}

fn row(cells: Vec<Cell>) -> Row {
    Row {
        cells,
        ..Row::default()
    }
}

fn date(year: i64, month: i64, day: i64) -> Date {
    Date { year, month, day }
}

fn time(hour: i64, minute: i64, second: i64) -> TimeOfDay {
    TimeOfDay {
        hour,
        minute,
        second,
        nanos: 0,
    }
}

fn formula(text: &str, cached_text: Option<&str>, cached_number: Option<f64>) -> Value {
    Value::Formula {
        formula: text.to_owned(),
        cached_text: cached_text.map(str::to_owned),
        cached_number,
    }
}

/// Serial 45352 is 2024-03-01 in the 1900 system.
const MARCH_FIRST: f64 = 45352.0;
const SECOND_NANOS: i64 = 1_000_000_000;

/// One written cell: what goes in, and what each reader must give back.
struct Case {
    name: &'static str,
    written: Option<Value>,
    ours: Option<Read>,
    formula: Option<&'static str>,
    theirs: Theirs<'static>,
}

/// Every value type of the write table, with the read each gives. Case `i` is on row `i + 2`,
/// its name in column A and its value in column B.
fn cases(bold: u64) -> Vec<Case> {
    let case = |name, written, ours, theirs| Case {
        name,
        written,
        ours,
        formula: None,
        theirs,
    };
    let duration = (26 * 3600 + 30 * 60 + 15) * SECOND_NANOS;
    vec![
        case(
            "text",
            Some(Value::Text("Tea & <cake>".to_owned())),
            Some(Read::Text("Tea & <cake>".to_owned())),
            Theirs::Text("Tea & <cake>"),
        ),
        case(
            "text with spaces",
            Some(Value::Text("  two spaces  ".to_owned())),
            Some(Read::Text("  two spaces  ".to_owned())),
            Theirs::Text("  two spaces  "),
        ),
        case(
            "text in many scripts",
            Some(Value::Text("Café 東京 🎉".to_owned())),
            Some(Read::Text("Café 東京 🎉".to_owned())),
            Theirs::Text("Café 東京 🎉"),
        ),
        case(
            "text that looks like a formula",
            Some(Value::Text("=1+1".to_owned())),
            Some(Read::Text("=1+1".to_owned())),
            Theirs::Text("=1+1"),
        ),
        case(
            "rich text",
            Some(Value::Rich(vec![
                RichRun {
                    text: "Hot ".to_owned(),
                    style: None,
                },
                RichRun {
                    text: "tea".to_owned(),
                    style: Some(bold),
                },
            ])),
            Some(Read::Text("Hot tea".to_owned())),
            Theirs::Text("Hot tea"),
        ),
        case(
            "whole number",
            Some(Value::Integer(42)),
            Some(Read::Integer(42)),
            Theirs::Number(42.0),
        ),
        case(
            "negative float",
            Some(Value::Number(-2.5)),
            Some(Read::Number(-2.5)),
            Theirs::Number(-2.5),
        ),
        case(
            "float with no fraction",
            Some(Value::Number(3.0)),
            Some(Read::Integer(3)),
            Theirs::Number(3.0),
        ),
        case(
            "float that is not exact in binary",
            Some(Value::Number(0.1)),
            Some(Read::Number(0.1)),
            Theirs::Number(0.1),
        ),
        case(
            "decimal",
            Some(Value::Decimal("12.50".to_owned())),
            Some(Read::Number(12.5)),
            Theirs::Number(12.5),
        ),
        case(
            "integer past 15 digits",
            Some(Value::Integer(1_234_567_890_123_456_789)),
            Some(Read::Text("1234567890123456789".to_owned())),
            Theirs::Text("1234567890123456789"),
        ),
        case(
            "true",
            Some(Value::Boolean(true)),
            Some(Read::Boolean(true)),
            Theirs::Bool(true),
        ),
        case(
            "false",
            Some(Value::Boolean(false)),
            Some(Read::Boolean(false)),
            Theirs::Bool(false),
        ),
        case(
            "date",
            Some(Value::Date(date(2024, 3, 1))),
            Some(Read::Date(date(2024, 3, 1))),
            Theirs::DateTime(MARCH_FIRST),
        ),
        case(
            "time of day",
            Some(Value::Time(time(13, 45, 30))),
            Some(Read::Time(time(13, 45, 30))),
            Theirs::DateTime(f64::from(13 * 3600 + 45 * 60 + 30) / 86_400.0),
        ),
        case(
            "clock time",
            Some(Value::Clock(Clock {
                day: date(2024, 3, 1),
                clock: time(13, 45, 0),
            })),
            Some(Read::Clock(Clock {
                day: date(2024, 3, 1),
                clock: time(13, 45, 0),
            })),
            Theirs::DateTime(MARCH_FIRST + f64::from(13 * 3600 + 45 * 60) / 86_400.0),
        ),
        case(
            "duration",
            Some(Value::Duration(duration)),
            Some(Read::Duration(duration)),
            Theirs::Duration(duration as f64 / (86_400.0 * SECOND_NANOS as f64)),
        ),
        case("empty", None, None, Theirs::Empty),
        Case {
            name: "formula with a cached number",
            written: Some(formula("B7*2", None, Some(84.0))),
            ours: Some(Read::Integer(84)),
            formula: Some("B7*2"),
            theirs: Theirs::Number(84.0),
        },
        Case {
            name: "formula with a cached text",
            written: Some(formula("\"Sh\"&\"op\"", Some("Shop"), None)),
            ours: Some(Read::Text("Shop".to_owned())),
            formula: Some("\"Sh\"&\"op\""),
            theirs: Theirs::Text("Shop"),
        },
        Case {
            name: "formula with no cached result",
            written: Some(formula("SUM(B7:B11)", None, None)),
            ours: None,
            formula: Some("SUM(B7:B11)"),
            theirs: Theirs::Empty,
        },
    ]
}

fn validation(validated: &str, kind: RuleKind, values: &[&str]) -> Validation {
    Validation {
        validated: validated.to_owned(),
        kind,
        operator: None,
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        input_title: None,
        input_message: None,
        error_title: None,
        error_message: None,
        allow_empty: false,
    }
}

// ---------------------------------------------------------------------------------------------
// The round trip.
// ---------------------------------------------------------------------------------------------

#[test]
fn every_write_fixture_reads_back_through_our_reader_and_calamine() {
    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    let bold = writer
        .style(Style {
            bold: Some(true),
            ..Style::default()
        })
        .unwrap();
    let cases = cases(bold);
    let mut rows = vec![row(vec![text("Case"), text("Value")])];
    rows.extend(
        cases
            .iter()
            .map(|case| row(vec![text(case.name), cell(case.written.clone())])),
    );
    // Below the cases: a merged title, three links and a cell with a note.
    let first = u32::try_from(rows.len()).unwrap() + 1;
    rows.push(row(vec![text("Shop report")]));
    rows.push(row(vec![text("Shop page")]));
    rows.push(row(vec![text("Write to the shop")]));
    rows.push(row(vec![text("See the posts")]));
    let mut out = writer.sheet(SheetOptions::named("Shop"), &rows).unwrap();
    let parts = [
        Part::Merge(format!("A{first}:C{first}")),
        Part::Merge("C2:D3".to_owned()),
        Part::Link(Link {
            anchor: format!("A{}", first + 1),
            target: "https://example.com/shop?id=1&page=2".to_owned(),
            text: Some("Shop page".to_owned()),
            tooltip: Some("Open the shop".to_owned()),
        }),
        Part::Link(Link {
            anchor: format!("A{}", first + 2),
            target: "mailto:shop@example.com".to_owned(),
            text: None,
            tooltip: None,
        }),
        Part::Link(Link {
            anchor: format!("A{}", first + 3),
            target: "'Blog posts'!B2".to_owned(),
            text: None,
            tooltip: None,
        }),
        Part::Note(Note {
            noted: "B2".to_owned(),
            note: "Checked by hand".to_owned(),
            author: Some("Blog".to_owned()),
        }),
        Part::Note(Note {
            noted: "B7".to_owned(),
            note: "A <second> note".to_owned(),
            author: None,
        }),
        Part::Validation({
            let mut list = validation("E2:E9", RuleKind::List, &["new", "paid", "sent"]);
            list.input_title = Some("Status".to_owned());
            list.allow_empty = true;
            list
        }),
        Part::Validation({
            let mut whole = validation("F2:F9", RuleKind::Whole, &["1", "100"]);
            whole.operator = Some(Operator::Between);
            whole
        }),
    ];
    for part in parts {
        out.extend(writer.part(part).unwrap());
    }
    out.extend(
        writer
            .sheet(
                SheetOptions::named("Blog posts"),
                &[row(vec![text("Title"), text("Views")])],
            )
            .unwrap(),
    );
    out.extend(writer.finish().unwrap().data);

    let mut opened = Opened::new(out);
    assert_eq!(opened.theirs.sheet_names(), ["Shop", "Blog posts"]);
    let names: Vec<String> = opened
        .ours
        .sheets(&opened.source)
        .unwrap()
        .into_iter()
        .map(|sheet| sheet.name)
        .collect();
    assert_eq!(names, ["Shop", "Blog posts"]);

    // Every value, its type and its formula.
    let read = opened.rows("Shop");
    for (at, case) in cases.iter().enumerate() {
        let number = u32::try_from(at).unwrap() + 2;
        check_cell(
            &mut opened,
            &read,
            "Shop",
            (number, 1),
            Some(Read::Text(case.name.to_owned())),
            None,
            &Theirs::Text(case.name),
        );
        check_cell(
            &mut opened,
            &read,
            "Shop",
            (number, 2),
            case.ours.clone(),
            case.formula,
            &case.theirs,
        );
    }

    // The merged ranges.
    assert_eq!(
        opened.ours.merged(&opened.source, "Shop").unwrap(),
        [
            merge(u64::from(first), 1, u64::from(first), 3),
            merge(2, 3, 3, 4)
        ]
    );
    assert_eq!(
        opened.theirs.merge_cells_by_sheet_name("Shop").unwrap(),
        [dimensions(first, 1, first, 3), dimensions(2, 3, 3, 4)]
    );

    // The links: an external URL, a `mailto:` address and a place in the workbook.
    let links = opened.ours.links(&opened.source, "Shop").unwrap();
    let ours: Vec<(&str, &str, bool)> = links
        .iter()
        .map(|link| (link.anchor.as_str(), link.target.as_str(), link.external))
        .collect();
    let (shop, mail, posts) = (
        format!("A{}", first + 1),
        format!("A{}", first + 2),
        format!("A{}", first + 3),
    );
    assert_eq!(
        ours,
        [
            (shop.as_str(), "https://example.com/shop?id=1&page=2", true),
            (mail.as_str(), "mailto:shop@example.com", true),
            (posts.as_str(), "'Blog posts'!B2", false),
        ]
    );
    assert_eq!(links[0].text.as_deref(), Some("Shop page"));
    assert_eq!(links[0].tooltip.as_deref(), Some("Open the shop"));
    let theirs: Vec<(Dimensions, Option<String>, Option<String>)> = opened
        .theirs
        .hyperlinks_by_sheet_name("Shop")
        .unwrap()
        .into_iter()
        // calamine gives a relationship's `Target` as the attribute's raw text, with its XML
        // escapes still in it, so the `&amp;` the archive holds is decoded here.
        .map(|link| {
            let target = link.target.map(|target| target.replace("&amp;", "&"));
            (link.range, target, link.location)
        })
        .collect();
    assert_eq!(
        theirs,
        [
            (
                dimensions(first + 1, 1, first + 1, 1),
                Some("https://example.com/shop?id=1&page=2".to_owned()),
                None
            ),
            (
                dimensions(first + 2, 1, first + 2, 1),
                Some("mailto:shop@example.com".to_owned()),
                None
            ),
            (
                dimensions(first + 3, 1, first + 3, 1),
                None,
                Some("'Blog posts'!B2".to_owned())
            ),
        ]
    );

    // The notes. calamine does not read notes, so only our reader checks them.
    let notes: Vec<(String, String, Option<String>)> = opened
        .ours
        .notes(&opened.source, "Shop")
        .unwrap()
        .into_iter()
        .map(|note| (note.noted, note.note, note.author))
        .collect();
    assert_eq!(
        notes,
        [
            (
                "B2".to_owned(),
                "Checked by hand".to_owned(),
                Some("Blog".to_owned())
            ),
            ("B7".to_owned(), "A <second> note".to_owned(), None),
        ]
    );

    // The data validation. Neither reader reads it, so the sheet's XML is checked.
    let sheet = opened.part("xl/worksheets/sheet1.xml");
    assert!(
        sheet.contains(
            "<dataValidation type=\"list\" allowBlank=\"1\" showInputMessage=\"1\" \
             showErrorMessage=\"1\" promptTitle=\"Status\" sqref=\"E2:E9\">\
             <formula1>\"new,paid,sent\"</formula1></dataValidation>"
        ),
        "{sheet}"
    );
    // `between` is the operator a rule has when it names none, so it is not written.
    assert!(
        sheet.contains(
            "<dataValidation type=\"whole\" showInputMessage=\"1\" \
             showErrorMessage=\"1\" sqref=\"F2:F9\"><formula1>1</formula1>\
             <formula2>100</formula2></dataValidation>"
        ),
        "{sheet}"
    );

    // The second sheet.
    let posts = opened.rows("Blog posts");
    check_cell(
        &mut opened,
        &posts,
        "Blog posts",
        (1, 2),
        Some(Read::Text("Views".to_owned())),
        None,
        &Theirs::Text("Views"),
    );
}

/// The template: a title, a header, one item row with a formula, a sum over the item rows, a
/// merged footer, a link and a validated cell below them.
fn invoice_template() -> Vec<u8> {
    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    let rows = [
        row(vec![text("Invoice {{number}}"), text("{{customer}}")]),
        row(vec![text("Item"), text("Qty"), text("Total")]),
        row(vec![
            text("{{items.name}}"),
            text("{{items.qty}}"),
            cell(Some(formula("B3*2", None, Some(0.0)))),
        ]),
        row(vec![
            text("Sum"),
            cell(None),
            cell(Some(formula("SUM(C3:C3)", None, Some(0.0)))),
        ]),
        row(vec![text("Thank you")]),
        row(vec![text("Shop page")]),
        row(vec![text("Rating"), cell(Some(Value::Integer(5)))]),
    ];
    let mut out = writer.sheet(SheetOptions::named("Invoice"), &rows).unwrap();
    for part in [
        Part::Merge("A5:C5".to_owned()),
        Part::Link(Link {
            anchor: "A6".to_owned(),
            target: "https://example.com/shop".to_owned(),
            text: None,
            tooltip: None,
        }),
        Part::Validation(validation("B7", RuleKind::Whole, &["1", "5"])),
    ] {
        out.extend(writer.part(part).unwrap());
    }
    out.extend(writer.finish().unwrap().data);
    out
}

fn placeholder(path: &str, value: Value) -> Placeholder {
    (path.to_owned(), Some(value))
}

#[test]
fn a_filled_template_reads_back_with_its_values_and_shifted_references() {
    let template = Bytes(invoice_template());
    let mut filling = Template::open(&template, Bound::default()).unwrap();
    let (mut out, order) = filling
        .fill(
            vec![
                placeholder("number", Value::Integer(42)),
                placeholder("customer", Value::Text("Shop".to_owned())),
            ],
            &["items".to_owned()],
        )
        .unwrap();
    assert_eq!(order, ["items"]);
    let items = (1..=3)
        .map(|at| {
            vec![
                placeholder("name", Value::Text(format!("Tea {at}"))),
                placeholder("qty", Value::Integer(at)),
            ]
        })
        .collect();
    out.extend(filling.items("items", items).unwrap());
    out.extend(filling.finish(&template).unwrap().data);

    let mut opened = Opened::new(out);
    let rows = opened.rows("Invoice");

    // The placeholders: text around one makes text, and a lone one keeps its value's type.
    check_cell(
        &mut opened,
        &rows,
        "Invoice",
        (1, 1),
        Some(Read::Text("Invoice 42".to_owned())),
        None,
        &Theirs::Text("Invoice 42"),
    );
    check_cell(
        &mut opened,
        &rows,
        "Invoice",
        (1, 2),
        Some(Read::Text("Shop".to_owned())),
        None,
        &Theirs::Text("Shop"),
    );

    // The item row is on rows 3 to 5, and each copy's formula names its own row.
    for at in 1..=3u32 {
        let number = at + 2;
        check_cell(
            &mut opened,
            &rows,
            "Invoice",
            (number, 1),
            Some(Read::Text(format!("Tea {at}"))),
            None,
            &Theirs::Text(&format!("Tea {at}")),
        );
        check_cell(
            &mut opened,
            &rows,
            "Invoice",
            (number, 2),
            Some(Read::Integer(i64::from(at))),
            None,
            &Theirs::Number(f64::from(at)),
        );
        let total = format!("B{number}*2");
        let read = ours_at(&rows, u64::from(number), 3).unwrap();
        assert_eq!(read.formula.as_deref(), Some(total.as_str()));
        let formulas = opened.theirs.worksheet_formula("Invoice").unwrap();
        assert_eq!(formulas.get_value((number - 1, 2)), Some(&total));
    }

    // Every row below the block moved down by two, and the sum grew over the three items.
    let sum = ours_at(&rows, 6, 3).unwrap();
    assert_eq!(sum.formula.as_deref(), Some("SUM(C3:C5)"));
    let formulas = opened.theirs.worksheet_formula("Invoice").unwrap();
    assert_eq!(
        formulas.get_value((5, 2)).map(String::as_str),
        Some("SUM(C3:C5)")
    );
    check_cell(
        &mut opened,
        &rows,
        "Invoice",
        (6, 1),
        Some(Read::Text("Sum".to_owned())),
        None,
        &Theirs::Text("Sum"),
    );
    check_cell(
        &mut opened,
        &rows,
        "Invoice",
        (7, 1),
        Some(Read::Text("Thank you".to_owned())),
        None,
        &Theirs::Text("Thank you"),
    );
    check_cell(
        &mut opened,
        &rows,
        "Invoice",
        (9, 2),
        Some(Read::Integer(5)),
        None,
        &Theirs::Number(5.0),
    );
    assert_eq!(
        opened.ours.merged(&opened.source, "Invoice").unwrap(),
        [merge(7, 1, 7, 3)]
    );
    assert_eq!(
        opened.theirs.merge_cells_by_sheet_name("Invoice").unwrap(),
        [dimensions(7, 1, 7, 3)]
    );
    let links = opened.ours.links(&opened.source, "Invoice").unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(
        (links[0].anchor.as_str(), links[0].target.as_str()),
        ("A8", "https://example.com/shop")
    );
    let theirs = opened.theirs.hyperlinks_by_sheet_name("Invoice").unwrap();
    assert_eq!(theirs.len(), 1);
    assert_eq!(theirs[0].range, dimensions(8, 1, 8, 1));
    // Neither reader reads data validation, so the sheet's XML is checked.
    let sheet = opened.part("xl/worksheets/sheet1.xml");
    assert!(sheet.contains("sqref=\"B9\""), "{sheet}");
    assert!(!sheet.contains("sqref=\"B7\""), "{sheet}");
}

// ---------------------------------------------------------------------------------------------
// Against Excel's own files.
// ---------------------------------------------------------------------------------------------

/// The Excel-made file `name` under `tests/fixtures/excel/`.
fn excel_file(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/excel/{name}.xlsx",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
}

/// The entry `name` of the archive `archive`.
fn entry_bytes(archive: &[u8], name: &str) -> Vec<u8> {
    let source = Bytes(archive.to_vec());
    let directory = Directory::read(&source, Bound::default()).unwrap();
    let found = directory
        .find(name)
        .unwrap_or_else(|| panic!("the archive has {name}"));
    directory.whole(&source, found).unwrap()
}

/// The entry `name` of the archive `archive`, as text.
fn entry(archive: &[u8], name: &str) -> String {
    String::from_utf8(entry_bytes(archive, name)).unwrap()
}

/// The text of `xml` from the first `<tag` to the last `</tag>`, both included.
fn region<'a>(xml: &'a str, tag: &str) -> &'a str {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let from = xml
        .find(&open)
        .unwrap_or_else(|| panic!("the part has {open}: {xml}"));
    let to = xml
        .rfind(&close)
        .unwrap_or_else(|| panic!("the part has {close}: {xml}"));
    &xml[from..to + close.len()]
}

/// `xml` split into its elements the way rust_xlsxwriter's own comparison splits it: at each
/// `>` followed by `<`, with the whitespace between them dropped. An element that contains one of
/// `ignore` is left out, which is how that suite leaves out an element Excel writes and it does
/// not, such as the `condense` and `extend` of a conditional format's font.
fn elements(xml: &str, ignore: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    for (at, c) in xml.char_indices() {
        if c == '>' && xml[at + 1..].trim_start().starts_with('<') {
            out.push(&xml[start..at]);
            start = at + 1 + (xml[at + 1..].len() - xml[at + 1..].trim_start().len());
        }
    }
    out.push(&xml[start..]);
    out.into_iter()
        .map(|token| {
            let token = token.trim().replace('\r', "");
            let token = if token.starts_with('<') {
                token
            } else {
                format!("<{token}")
            };
            if token.ends_with('>') {
                token
            } else {
                format!("{token}>")
            }
        })
        .filter(|element| !ignore.iter().any(|word| element.contains(word)))
        .collect()
}

/// Checks that `ours` and Excel's `excel` have the same elements, in the same order.
fn same_part(fixture: &str, part: &str, ours: &str, excel: &str, ignore: &[&str]) {
    assert_eq!(
        elements(ours, ignore),
        elements(excel, ignore),
        "{part} differs from the one Excel wrote in {fixture}.xlsx"
    );
}

/// A sheet `Sheet1` with no header defaults, its `parts` added before its `rows`, and the
/// archive `writer` makes of it.
fn excel_book(mut writer: Writer, parts: Vec<Part>, rows: Vec<Row>) -> Vec<u8> {
    let options = SheetOptions {
        header: Header {
            default_style: false,
            style: None,
            freeze: false,
            filter: false,
        },
        ..SheetOptions::named("Sheet1")
    };
    let mut out = writer.sheet(options, &[]).unwrap();
    for part in parts {
        out.extend(writer.part(part).unwrap());
    }
    out.extend(writer.rows(&rows).unwrap());
    out.extend(writer.finish().unwrap().data);
    out
}

/// The rows from 1 to `last`, or further down to the last row of `cells`, with `cells` placed by
/// their references, such as `B2`.
fn grid<S: AsRef<str>>(cells: Vec<(S, Cell)>, last: u32) -> Vec<Row> {
    let mut rows = vec![Row::default(); last as usize];
    for (at, placed) in cells {
        let (column, number) = cell_at(at.as_ref()).unwrap();
        if rows.len() < number as usize {
            rows.resize(number as usize, Row::default());
        }
        let row = &mut rows[number as usize - 1];
        if row.cells.len() <= column as usize {
            row.cells.resize(column as usize + 1, Cell::default());
        }
        row.cells[column as usize] = placed;
    }
    rows
}

fn styled(value: Value, style: u64) -> Cell {
    Cell {
        value: Some(value),
        style: Some(style),
    }
}

fn number(value: f64) -> Cell {
    cell(Some(Value::Number(value)))
}

/// The numbers 10, 20, 30 and 40 down `A1:A4`, the cells most of the conditional format files
/// hold.
fn tens() -> Vec<(&'static str, Cell)> {
    vec![
        ("A1", number(10.0)),
        ("A2", number(20.0)),
        ("A3", number(30.0)),
        ("A4", number(40.0)),
    ]
}

fn thin_box() -> Border {
    Border {
        top: Some(Line::Thin),
        right: Some(Line::Thin),
        bottom: Some(Line::Thin),
        left: Some(Line::Thin),
    }
}

#[test]
fn styles_match_the_excel_made_fixtures() {
    let bold = || Style {
        bold: Some(true),
        ..Style::default()
    };
    // Each case is the fixture's name, and the writer with its styles and the cells that use
    // them, in the order the file uses them.
    type Case = (
        &'static str,
        Box<dyn Fn(&mut Writer) -> Vec<(&'static str, Cell)>>,
    );
    let cases: Vec<Case> = vec![
        (
            "format02",
            Box::new(move |writer| {
                let arial = Style {
                    font: Some("Arial".to_owned()),
                    bold: Some(true),
                    valign: Some(Valign::Bottom),
                    ..Style::default()
                };
                let left = writer
                    .style(Style {
                        align: Some(Align::Left),
                        rotate: Some(0),
                        ..arial.clone()
                    })
                    .unwrap();
                let turned = writer
                    .style(Style {
                        align: Some(Align::Center),
                        rotate: Some(90),
                        ..arial
                    })
                    .unwrap();
                vec![
                    ("A1", styled(Value::Text("Foo".to_owned()), left)),
                    ("B1", styled(Value::Text("Bar".to_owned()), turned)),
                ]
            }),
        ),
        (
            "format03",
            Box::new(move |writer| {
                let red = Style {
                    fill: Some("#FF0000".to_owned()),
                    ..bold()
                };
                let first = writer.style(red.clone()).unwrap();
                let second = writer
                    .style(Style {
                        italic: Some(true),
                        ..red
                    })
                    .unwrap();
                vec![
                    ("A1", styled(Value::Text("Foo".to_owned()), first)),
                    ("A2", styled(Value::Text("Bar".to_owned()), second)),
                ]
            }),
        ),
        (
            "format05",
            Box::new(|writer| {
                let wrap = writer
                    .style(Style {
                        wrap: Some(true),
                        ..Style::default()
                    })
                    .unwrap();
                vec![("A1", styled(Value::Text("Foo\nBar".to_owned()), wrap))]
            }),
        ),
        (
            "format11",
            Box::new(|writer| {
                let centered = writer
                    .style(Style {
                        align: Some(Align::Center),
                        valign: Some(Valign::Center),
                        ..Style::default()
                    })
                    .unwrap();
                vec![("B2", styled(Value::Text("Foo".to_owned()), centered))]
            }),
        ),
        (
            "format12",
            Box::new(|writer| {
                let mut lines = |left: bool, bottom: bool| {
                    let border = Border {
                        top: Some(Line::Thin),
                        left: left.then_some(Line::Thin),
                        bottom: bottom.then_some(Line::Thin),
                        right: None,
                    };
                    writer
                        .style(Style {
                            border: Some(border),
                            ..Style::default()
                        })
                        .unwrap()
                };
                let top_left_bottom = lines(true, true);
                let top_left = lines(true, false);
                let top_bottom = lines(false, true);
                let test = |style| styled(Value::Text("test".to_owned()), style);
                vec![
                    ("B2", test(top_left_bottom)),
                    ("D2", test(top_left)),
                    ("F2", test(top_bottom)),
                ]
            }),
        ),
        (
            "format15",
            Box::new(move |writer| {
                let plain = writer.style(bold()).unwrap();
                let whole = writer
                    .style(Style {
                        format: Some("0".to_owned()),
                        ..bold()
                    })
                    .unwrap();
                vec![
                    ("A1", styled(Value::Number(1.0), plain)),
                    ("A2", styled(Value::Number(2.0), whole)),
                ]
            }),
        ),
        (
            "format19",
            Box::new(|writer| {
                let clock = Style {
                    format: Some("hh:mm;@".to_owned()),
                    ..Style::default()
                };
                let plain = writer.style(clock.clone()).unwrap();
                let yellow = writer
                    .style(Style {
                        fill: Some("#FFFF00".to_owned()),
                        ..clock
                    })
                    .unwrap();
                vec![
                    ("A1", styled(Value::Number(1.0), plain)),
                    ("A2", styled(Value::Number(2.0), yellow)),
                ]
            }),
        ),
    ];
    for (fixture, build) in cases {
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        let cells = build(&mut writer);
        let ours = excel_book(writer, Vec::new(), grid(cells, 0));
        let excel = excel_file(fixture);
        let part = "xl/styles.xml";
        same_part(
            fixture,
            part,
            &entry(&ours, part),
            &entry(&excel, part),
            &[],
        );
    }
}

/// A table over `range` with Excel's defaults: no name, the default style, banded rows and a
/// filter.
fn excel_table(range: &str, totals: Vec<Total>) -> Table {
    Table {
        range: range.to_owned(),
        name: None,
        style: None,
        banded: true,
        filter: true,
        totals,
    }
}

fn excel_total(column: &str, function: Option<Function>, formula: Option<&str>) -> Total {
    Total {
        column: column.to_owned(),
        function,
        formula: formula.map(str::to_owned),
    }
}

#[test]
fn tables_match_the_excel_made_fixtures() {
    // The header cells `names`, from the cell `first` to the right.
    let columns = |first: &str, names: &[&str]| -> Vec<(String, Cell)> {
        let (left, row) = cell_at(first).unwrap();
        names
            .iter()
            .enumerate()
            .map(|(at, name)| (reference(left + at as u32, row), text(name)))
            .collect()
    };
    let four = ["Column1", "Column2", "Column3", "Column4"];
    let mut medium10 = excel_table("C3:F13", Vec::new());
    medium10.style = Some("TableStyleMedium10".to_owned());
    let mut unfiltered = excel_table("C3:F13", Vec::new());
    unfiltered.filter = false;
    // Each case is the fixture's name, the table, its header cells and the sheet's last row.
    type Case = (&'static str, Table, Vec<(String, Cell)>, u32);
    let cases: Vec<Case> = vec![
        (
            "table01",
            excel_table("C3:F13", Vec::new()),
            columns("C3", &four),
            13,
        ),
        (
            "table08",
            excel_table(
                "C3:F13",
                vec![excel_total("Column4", Some(Function::Count), None)],
            ),
            columns("C3", &four),
            13,
        ),
        (
            "table23",
            excel_table(
                "B3:F8",
                ["Column'", "Column#", "Column[", "Column]"]
                    .into_iter()
                    .map(|name| excel_total(name, Some(Function::Sum), None))
                    .collect(),
            ),
            columns(
                "B3",
                &["Column1", "Column'", "Column#", "Column[", "Column]"],
            ),
            8,
        ),
        ("table24", medium10, columns("C3", &four), 13),
        ("table28", unfiltered, columns("C3", &four), 13),
        (
            "table32",
            excel_table(
                "C3:F13",
                vec![
                    excel_total("Column2", None, Some("D5+D9")),
                    excel_total("Column3", None, Some("SUM([Column3])")),
                    excel_total("Column4", Some(Function::Count), None),
                ],
            ),
            columns("C3", &four),
            13,
        ),
    ];
    for (fixture, table, header, last) in cases {
        let writer = Writer::open(WorkbookOptions::default()).unwrap();
        let ours = excel_book(writer, vec![Part::Table(table)], grid(header, last));
        let excel = excel_file(fixture);
        let part = "xl/tables/table1.xml";
        same_part(
            fixture,
            part,
            &entry(&ours, part),
            &entry(&excel, part),
            &[],
        );
    }
}

/// The formula text of the cell `at` in the sheet XML `sheet`.
fn formula_at<'a>(sheet: &'a str, at: &str) -> &'a str {
    let cell = &sheet[sheet
        .find(&format!("<c r=\"{at}\""))
        .unwrap_or_else(|| panic!("the sheet has {at}"))..];
    let from = cell.find("<f>").expect("the cell has a formula") + 3;
    &cell[from..from + cell[from..].find("</f>").unwrap()]
}

#[test]
fn a_calculated_column_is_written_in_the_form_excel_stores() {
    // table10 has a table at B3:K6 whose last column is `SUM([@[Column1]:[Column3]])`. Excel
    // stores `[@x]` as `Table1[[#This Row],[x]]` in each data cell.
    let excel = entry(&excel_file("table10"), "xl/worksheets/sheet1.xml");
    let header: Vec<(String, Cell)> = (1..=10)
        .map(|at| (reference(at, 3), text(&format!("Column{at}"))))
        .collect();
    for given in [
        "SUM([@[Column1]:[Column3]])",
        "SUM(Table1[@[Column1]:[Column3]])",
    ] {
        let mut cells = header.clone();
        for row in [4, 5] {
            cells.push((
                reference(10, row),
                cell(Some(formula(given, None, Some(0.0)))),
            ));
        }
        // The table is added before its header row, row 3.
        let mut rows = grid(cells, 5);
        let data = rows.split_off(2);
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        let mut ours = writer.sheet(SheetOptions::named("Sheet1"), &rows).unwrap();
        ours.extend(
            writer
                .part(Part::Table(excel_table("B3:K5", Vec::new())))
                .unwrap(),
        );
        ours.extend(writer.rows(&data).unwrap());
        ours.extend(writer.finish().unwrap().data);
        let sheet = entry(&ours, "xl/worksheets/sheet1.xml");
        for at in ["K4", "K5"] {
            assert_eq!(
                formula_at(&sheet, at),
                formula_at(&excel, at),
                "{given} at {at}"
            );
        }
    }
}

/// A `cellIs` rule over `formatted` that compares with `value` and formats with `style`.
fn excel_rule(formatted: &str, operator: Operator, value: &str, style: Style) -> Conditional {
    Conditional {
        formatted: formatted.to_owned(),
        kind: Kind::Cell,
        operator: Some(operator),
        values: vec![value.to_owned()],
        colors: Vec::new(),
        icons: None,
        rank: None,
        percent: false,
        style: Some(style),
    }
}

#[test]
fn conditional_formats_match_the_excel_made_fixtures() {
    let dark_on_light = |color: &str, fill: &str| Style {
        color: Some(color.to_owned()),
        fill: Some(fill.to_owned()),
        ..Style::default()
    };
    let bold_italic = Style {
        bold: Some(true),
        italic: Some(true),
        ..Style::default()
    };
    // Each case is the fixture's name, its rule, the cell format it uses besides, and its cells.
    // Excel's preset styles give a font `condense` and `extend`, which rust_xlsxwriter does not
    // write and its own comparison leaves out for these files.
    let cases: Vec<(&str, Conditional, Option<Style>, &[&str])> = vec![
        (
            "cond_format01",
            excel_rule(
                "A1",
                Operator::Greater,
                "5",
                dark_on_light("#9C0006", "#FFC7CE"),
            ),
            None,
            &["condense", "extend"],
        ),
        (
            "cond_format05",
            excel_rule(
                "A1",
                Operator::Equal,
                "7",
                Style {
                    border: Some(thin_box()),
                    ..Style::default()
                },
            ),
            None,
            &[],
        ),
        (
            "cond_format08",
            excel_rule(
                "A1",
                Operator::Greater,
                "5",
                dark_on_light("#9C6500", "#FFEB9C"),
            ),
            None,
            &["condense", "extend"],
        ),
        (
            "cond_format10",
            excel_rule("B3:B6", Operator::Greater, "20", bold_italic.clone()),
            Some(bold_italic),
            &[],
        ),
        (
            "cond_format19",
            excel_rule(
                "A1",
                Operator::Equal,
                "\"X\"",
                dark_on_light("#9C0006", "#FFC7CE"),
            ),
            None,
            &["condense", "extend"],
        ),
    ];
    for (fixture, rule, cell_style, ignore) in cases {
        let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
        let cells = match cell_style {
            // This file writes `Hello` in its rule's style and the numbers in `B3:B6`.
            Some(style) => {
                let style = writer.style(style).unwrap();
                vec![
                    ("A1", styled(Value::Text("Hello".to_owned()), style)),
                    ("B3", number(10.0)),
                    ("B4", number(20.0)),
                    ("B5", number(30.0)),
                    ("B6", number(40.0)),
                ]
            }
            None => tens(),
        };
        let ours = excel_book(writer, vec![Part::Conditional(rule)], grid(cells, 0));
        let excel = excel_file(fixture);
        let sheet = "xl/worksheets/sheet1.xml";
        let (our_sheet, excel_sheet) = (entry(&ours, sheet), entry(&excel, sheet));
        same_part(
            fixture,
            "The sheet's conditional formats",
            region(&our_sheet, "conditionalFormatting"),
            region(&excel_sheet, "conditionalFormatting"),
            &[],
        );
        let styles = "xl/styles.xml";
        same_part(
            fixture,
            styles,
            &entry(&ours, styles),
            &entry(&excel, styles),
            ignore,
        );
    }
}

#[test]
fn data_validation_matches_the_excel_made_fixtures() {
    let list = |validated: &str, values: &[&str], prompt: Option<(&str, &str)>| Validation {
        allow_empty: true,
        input_title: prompt.map(|(title, _)| title.to_owned()),
        input_message: prompt.map(|(_, message)| message.to_owned()),
        ..validation(validated, RuleKind::List, values)
    };
    let titled = Some(("This is the input title", "This is the input message"));
    let longest_message = format!("This is the longest input message {}", "a".repeat(221));
    let mut long_list: Vec<String> = Vec::new();
    for second in ['a', 'b'] {
        for third in 'a'..='z' {
            let name = format!("Foob{second}{third}");
            if name.as_str() >= "Foobar" {
                long_list.push(name);
            }
        }
    }
    long_list.push("Foobca".to_owned());
    long_list.push("End".to_owned());
    let long_list: Vec<&str> = long_list.iter().map(String::as_str).collect();
    let foo = ["Foo", "Bar", "Baz"];
    let cases: Vec<(&str, Vec<Validation>)> = vec![
        ("data_validation01", vec![list("C2", &foo, None)]),
        ("data_validation02", vec![list("C2", &foo, titled)]),
        (
            "data_validation03",
            vec![
                list("C2", &foo, titled),
                list(
                    "D6",
                    &long_list,
                    Some(("This is the longest input title1", &longest_message)),
                ),
            ],
        ),
    ];
    for (fixture, rules) in cases {
        let writer = Writer::open(WorkbookOptions::default()).unwrap();
        let parts = rules.into_iter().map(Part::Validation).collect();
        let ours = excel_book(writer, parts, Vec::new());
        let excel = excel_file(fixture);
        let sheet = "xl/worksheets/sheet1.xml";
        same_part(
            fixture,
            "The sheet's data validation",
            region(&entry(&ours, sheet), "dataValidations"),
            region(&entry(&excel, sheet), "dataValidations"),
            &[],
        );
    }
}

/// The rows of `columns`, each a list of numbers down its column from `A1`.
fn number_columns(columns: &[&[f64]]) -> Vec<Row> {
    let mut cells = Vec::new();
    for (at, values) in columns.iter().enumerate() {
        for (row, value) in values.iter().enumerate() {
            cells.push((reference(at as u32, row as u32 + 1), number(*value)));
        }
    }
    grid(cells, 0)
}

/// An image the file Excel made holds, at `at` and its own size.
fn excel_image(at: &str, bytes: Vec<u8>) -> Image {
    Image {
        at: at.to_owned(),
        image: bytes,
        width: None,
        height: None,
        offset_x: None,
        offset_y: None,
    }
}

#[test]
fn images_match_the_excel_made_fixtures() {
    let excel = excel_file("image53");
    let red = entry_bytes(&excel, "xl/media/image1.png");
    let writer = Writer::open(WorkbookOptions::default()).unwrap();
    let parts = vec![Part::Image(excel_image("E9", red.clone()))];
    let ours = excel_book(writer, parts, Vec::new());
    assert_eq!(entry_bytes(&ours, "xl/media/image1.png"), red);
    for part in [
        "xl/drawings/drawing1.xml",
        "xl/drawings/_rels/drawing1.xml.rels",
    ] {
        same_part(
            "image53",
            part,
            &entry(&ours, part),
            &entry(&excel, part),
            &[],
        );
    }
}

/// A chart at `E9` of the default size, as each chart file was made, with one layer per
/// `(kind, series)` and its series as `(categories, values)`.
fn excel_chart(layers: Vec<(chart::Kind, Vec<(Option<&str>, &str)>)>) -> Chart {
    Chart {
        placed: "E9".to_owned(),
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

/// A chart part made comparable: Excel's `numCache` and `strCache` removed, which
/// `rule:core-classes/spreadsheet-sheet-end-parts` declines, each axis id renumbered from 1 by
/// order of appearance, because Excel picks them at random, and the rounding noise of a decimal
/// such as `0.75000000000000011` cut at its run of twelve zeros, as the suite the files come from
/// cuts it. Both files go through it.
fn comparable_chart(xml: &str) -> String {
    let mut out = xml.to_owned();
    while let Some(from) = out.find("000000000000") {
        let digits = out[from..]
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(out.len() - from);
        out.replace_range(from..from + digits, "");
    }
    for cache in ["c:numCache", "c:strCache"] {
        let open = format!("<{cache}>");
        let close = format!("</{cache}>");
        while let Some(from) = out.find(&open) {
            let to = out[from..].find(&close).unwrap() + from + close.len();
            out.replace_range(from..to, "");
        }
    }
    let mut ids: Vec<String> = Vec::new();
    let mut renumbered = String::new();
    let mut rest = out.as_str();
    while let Some(at) = ["<c:axId val=\"", "<c:crossAx val=\""]
        .iter()
        .filter_map(|tag| rest.find(tag).map(|at| at + tag.len()))
        .min()
    {
        let end = at + rest[at..].find('"').unwrap();
        let id = &rest[at..end];
        let number = match ids.iter().position(|seen| seen == id) {
            Some(number) => number + 1,
            None => {
                ids.push(id.to_owned());
                ids.len()
            }
        };
        renumbered.push_str(&rest[..at]);
        renumbered.push_str(&number.to_string());
        rest = &rest[end..];
    }
    renumbered.push_str(rest);
    renumbered
}

#[test]
fn every_chart_type_in_the_subset_matches_its_excel_made_fixture() {
    use chart::Kind::{Area, Bar, Column, Doughnut, Line, Pie, Scatter};
    let fifteen: [&[f64]; 3] = [
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        &[2.0, 4.0, 6.0, 8.0, 10.0],
        &[3.0, 6.0, 9.0, 12.0, 15.0],
    ];
    let area: [&[f64]; 3] = [
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        &[8.0, 7.0, 6.0, 8.0, 10.0],
        &[3.0, 6.0, 9.0, 12.0, 15.0],
    ];
    let round: [&[f64]; 2] = [&[2.0, 4.0, 6.0], &[60.0, 30.0, 10.0]];
    let combined: [&[f64]; 2] = [&[2.0, 7.0, 3.0, 6.0, 2.0], &[20.0, 25.0, 10.0, 10.0, 20.0]];
    let three = vec![(None, "A1:A5"), (None, "B1:B5"), (None, "C1:C5")];
    let two = vec![(Some("A1:A5"), "B1:B5"), (Some("A1:A5"), "C1:C5")];
    let one = vec![(Some("A1:A3"), "B1:B3")];
    let cases: Vec<(&str, &[&[f64]], Chart)> = vec![
        (
            "chart_column01",
            &fifteen,
            excel_chart(vec![(Column, three.clone())]),
        ),
        (
            "chart_bar01",
            &fifteen,
            excel_chart(vec![(Bar, two.clone())]),
        ),
        ("chart_line01", &fifteen, excel_chart(vec![(Line, three)])),
        (
            "chart_area01",
            &area,
            excel_chart(vec![(Area, two.clone())]),
        ),
        ("chart_pie01", &round, excel_chart(vec![(Pie, one.clone())])),
        (
            "chart_doughnut01",
            &round,
            excel_chart(vec![(Doughnut, one)]),
        ),
        (
            "chart_scatter01",
            &fifteen,
            excel_chart(vec![(Scatter, two)]),
        ),
        (
            "chart_combined02",
            &combined,
            excel_chart(vec![
                (Column, vec![(None, "A1:A5")]),
                (Line, vec![(None, "B1:B5")]),
            ]),
        ),
    ];
    for (fixture, columns, chart) in cases {
        let writer = Writer::open(WorkbookOptions::default()).unwrap();
        let ours = excel_book(writer, vec![Part::Chart(chart)], number_columns(columns));
        let excel = excel_file(fixture);
        // The program sets the empty-cell display of this one chart to gaps, and Excel writes
        // the setting. Gaps are also what Excel shows when a chart does not set it, so the
        // writer never writes it.
        let ignore: &[&str] = if fixture == "chart_combined02" {
            &["c:dispBlanksAs"]
        } else {
            &[]
        };
        let part = "xl/charts/chart1.xml";
        same_part(
            fixture,
            part,
            &comparable_chart(&entry(&ours, part)),
            &comparable_chart(&entry(&excel, part)),
            ignore,
        );
        let drawing = "xl/drawings/drawing1.xml";
        same_part(
            fixture,
            drawing,
            &entry(&ours, drawing),
            &entry(&excel, drawing),
            &[],
        );
    }
}
