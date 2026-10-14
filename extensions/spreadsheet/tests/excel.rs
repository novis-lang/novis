//! What opens in Excel (`rule:core-classes/spreadsheet-output-is-inert`): every file the writer
//! and the template filler make, read back by two readers that share no code with the writer.
//!
//! **The round trip.** A workbook written here, and a template filled here, are read back twice:
//! by the component's own reader ([`nvs_spreadsheet::read::Reader`]) and by calamine, a reader
//! written apart from this crate. Each value must read back as the type and the value it was
//! written as, by `rule:core-classes/spreadsheet-value-types`. calamine reads values, formulas,
//! merged ranges and hyperlinks. It does not read notes or data validation, so those are checked
//! through our reader and through the sheet's own XML.

use std::io::Cursor;

use calamine::{Data, Dimensions, Reader as _, Xlsx, open_workbook_from_rs};
use nvs_spreadsheet::read::merge::Merge;
use nvs_spreadsheet::read::zip::{Bound, Directory};
use nvs_spreadsheet::read::{Read, ReadCell, ReadOptions, ReadRow, Reader, Source};
use nvs_spreadsheet::tail::{Link, Note, Operator, Part, RuleKind, Validation};
use nvs_spreadsheet::template::{Placeholder, Template};
use nvs_spreadsheet::{
    Cell, Clock, Date, Error, RichRun, Row, SheetOptions, Style, TimeOfDay, Value, WorkbookOptions,
    Writer,
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
