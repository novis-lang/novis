//! The evaluator (`rule:core-classes/spreadsheet-evaluation-and-roster`): formula text parsed,
//! and a workbook built here cell by cell, computed, and read back.

use nvs_spreadsheet::eval::parse::{self, Area, Binary, Expr, Item, Rect, Structured, Unary};
use nvs_spreadsheet::eval::{Book, Kind, Missed, Name, Table, Value};
use nvs_spreadsheet::read::zip::Bound;
use nvs_spreadsheet::read::{CellError, Read, ReadOptions, ReadRow, Reader, Source};
use nvs_spreadsheet::{
    Cell, Column, Date, Error, Row, SheetOptions, Value as Written, WorkbookOptions, Writer,
};

fn n(v: f64) -> Value {
    Value::Number(v)
}

fn text(v: &str) -> Value {
    Value::Text(v.to_owned())
}

/// A book with one sheet per name, and each `(sheet, cell, formula)` set with a stored 0.
fn book(sheets: &[&str], formulas: &[(&str, &str, &str)]) -> Book {
    let mut book = Book::new();
    for sheet in sheets {
        book.sheet(sheet);
    }
    for (sheet, cell, formula) in formulas {
        put_formula(&mut book, sheet, cell, formula, n(0.0));
    }
    book
}

fn put(book: &mut Book, sheet: &str, cell: &str, value: Value) {
    let at = book.sheet(sheet);
    let (row, col) = parse::address(cell).unwrap();
    book.set(at, row, col, value);
}

fn put_formula(book: &mut Book, sheet: &str, cell: &str, formula: &str, stored: Value) {
    let at = book.sheet(sheet);
    let (row, col) = parse::address(cell).unwrap();
    book.formula(at, row, col, Some(formula), stored);
}

fn get(book: &Book, reference: &str) -> Value {
    book.get(reference).cloned().unwrap()
}

fn missed(cell: &str, kind: Kind, text: &str) -> Missed {
    Missed {
        cell: cell.to_owned(),
        kind,
        text: text.to_owned(),
    }
}

fn area(sheet: Option<&str>, top: u32, left: u32, bottom: u32, right: u32) -> Expr {
    Expr::Area(Area {
        sheet: sheet.map(str::to_owned),
        rect: Rect {
            top,
            left,
            bottom,
            right,
        },
    })
}

fn boxed(e: Expr) -> Box<Expr> {
    Box::new(e)
}

#[test]
fn formula_text_parses_into_references_operators_and_calls() {
    let p = |t: &str| parse::parse(t).unwrap();
    assert_eq!(
        p("=SUM(A1:B2, 'Q1 sales'!$C$3)"),
        Expr::Call(
            "SUM".into(),
            vec![area(None, 1, 1, 2, 2), area(Some("Q1 sales"), 3, 3, 3, 3)]
        )
    );
    assert_eq!(p("=a:c"), area(None, 1, 1, 1_048_576, 3));
    assert_eq!(p("=Data!3:5"), area(Some("Data"), 3, 1, 5, 16_384));
    // `-` binds tighter than `^`, and `*` tighter than `+`.
    assert_eq!(
        p("=-2^2"),
        Expr::Binary(
            Binary::Power,
            boxed(Expr::Unary(Unary::Minus, boxed(Expr::Number(2.0)))),
            boxed(Expr::Number(2.0))
        )
    );
    assert_eq!(
        p("=1+2*3%"),
        Expr::Binary(
            Binary::Plus,
            boxed(Expr::Number(1.0)),
            boxed(Expr::Binary(
                Binary::Times,
                boxed(Expr::Number(2.0)),
                boxed(Expr::Unary(Unary::Percent, boxed(Expr::Number(3.0))))
            ))
        )
    );
    assert_eq!(
        p("=A1&\"x\"<=B1"),
        Expr::Binary(
            Binary::Le,
            boxed(Expr::Binary(
                Binary::Concat,
                boxed(area(None, 1, 1, 1, 1)),
                boxed(Expr::Text("x".into()))
            )),
            boxed(area(None, 1, 2, 1, 2))
        )
    );
    assert_eq!(p("=\"a\"\"b\""), Expr::Text("a\"b".into()));
    assert_eq!(p("=true"), Expr::Bool(true));
    assert_eq!(p("=#N/A"), Expr::Error(CellError::NotAvailable));
    assert_eq!(p("=TAX1_RATE"), Expr::Name(None, "TAX1_RATE".into()));
    assert_eq!(
        p("=Data!Rate"),
        Expr::Name(Some("Data".into()), "Rate".into())
    );
    assert_eq!(
        p("=log10(A1)"),
        Expr::Call("LOG10".into(), vec![area(None, 1, 1, 1, 1)])
    );
    assert_eq!(
        p("=_xlfn.CONCAT(1)"),
        Expr::Call("CONCAT".into(), vec![Expr::Number(1.0)])
    );
    assert_eq!(
        p("=IF(A1,,2)"),
        Expr::Call(
            "IF".into(),
            vec![area(None, 1, 1, 1, 1), Expr::Missing, Expr::Number(2.0)]
        )
    );
    let structured = |table: Option<&str>, items: Vec<Item>, columns: Option<(&str, &str)>| {
        Expr::Structured(Structured {
            table: table.map(str::to_owned),
            items,
            columns: columns.map(|(a, b)| (a.to_owned(), b.to_owned())),
        })
    };
    assert_eq!(
        p("=Sales[Unit price]"),
        structured(Some("Sales"), vec![], Some(("Unit price", "Unit price")))
    );
    assert_eq!(
        p("=[@Qty]"),
        structured(None, vec![Item::ThisRow], Some(("Qty", "Qty")))
    );
    assert_eq!(
        p("=Sales[[#This Row],[Qty]]"),
        structured(Some("Sales"), vec![Item::ThisRow], Some(("Qty", "Qty")))
    );
    assert_eq!(
        p("=Sales[[#Headers],[#Data],[Qty]:[Price]]"),
        structured(
            Some("Sales"),
            vec![Item::Headers, Item::Data],
            Some(("Qty", "Price"))
        )
    );
    assert_eq!(
        p("=Sales[#Totals]"),
        structured(Some("Sales"), vec![Item::Totals], None)
    );
    assert_eq!(
        p("=Sales[[a']b]]"),
        structured(Some("Sales"), vec![], Some(("a]b", "a]b")))
    );
    // The parts the evaluator lists keep the text the formula wrote.
    assert_eq!(p("=Jan:Mar!A1"), Expr::Listed("Jan:Mar!A1".into()));
    assert_eq!(p("=[1]Shop!A1"), Expr::Listed("[1]Shop!A1".into()));
    assert_eq!(
        p("='[Book.xlsx]Shop'!A1"),
        Expr::Listed("'[Book.xlsx]Shop'!A1".into())
    );
    assert_eq!(
        p("=SUM((A1,B1))"),
        Expr::Call("SUM".into(), vec![Expr::Listed("(A1,B1)".into())])
    );
    assert_eq!(
        p("=SUM(A1:B2 B1:C3)"),
        Expr::Call("SUM".into(), vec![Expr::Listed("A1:B2 B1:C3".into())])
    );
    assert_eq!(p("={1,2;3,4}"), Expr::Listed("{1,2;3,4}".into()));
    for bad in ["=1+", "=SUM(1", "=\"open", "=Sales[#Nothing]", "=)", "=$"] {
        assert!(parse::parse(bad).is_err(), "{bad}");
    }
    let deep = format!("={}1{}", "(".repeat(1_000), ")".repeat(1_000));
    assert!(parse::parse(&deep).is_err());
    let long = format!("=1{}", "+1".repeat(10_000));
    assert!(parse::parse(&long).is_err());
}

#[test]
fn operators_follow_excels_precedence_and_coercion() {
    let cases: &[(&str, Value)] = &[
        ("=-2^2", n(4.0)),
        ("=2^3^2", n(64.0)),
        ("=1+2*3", n(7.0)),
        ("=(1+2)*3", n(9.0)),
        ("=50%*10", n(5.0)),
        ("=\"3\"+1", n(4.0)),
        ("=\" 2.5e1 \"*2", n(50.0)),
        ("=\"50%\"*2", n(1.0)),
        ("=\"2024-01-15\"+0", n(45_306.0)),
        ("=\"12:00\"*2", n(1.0)),
        ("=TRUE+1", n(2.0)),
        ("=Z9+1", n(1.0)),
        ("=Z9", n(0.0)),
        ("=\"a\"&1.5&TRUE&Z9", text("a1.5TRUE")),
        ("=(0.1+0.2)&\"\"", text("0.3")),
        ("=1/3&\"\"", text("0.333333333333333")),
        ("=10^15&\"\"", text("1E+15")),
        ("=123456789012345&\"\"", text("123456789012345")),
        ("=1/0", Value::Error(CellError::DivZero)),
        ("=0^0", Value::Error(CellError::Num)),
        ("=(-8)^0.5", Value::Error(CellError::Num)),
        ("=\"x\"+1", Value::Error(CellError::Value)),
        ("=\"1/15/2024\"+0", Value::Error(CellError::Value)),
        ("=#N/A+1/0", Value::Error(CellError::NotAvailable)),
        ("=1<\"a\"", Value::Bool(true)),
        ("=\"z\"<TRUE", Value::Bool(true)),
        ("=\"a\"=\"A\"", Value::Bool(true)),
        ("=Z9=0", Value::Bool(true)),
        ("=Z9=\"\"", Value::Bool(true)),
        ("=2<>2", Value::Bool(false)),
        ("=+\"kept\"", text("kept")),
    ];
    let mut book = Book::new();
    book.sheet("Sheet1");
    for (row, (formula, _)) in cases.iter().enumerate() {
        put_formula(
            &mut book,
            "Sheet1",
            &format!("A{}", row + 1),
            formula,
            n(-1.0),
        );
    }
    assert_eq!(book.evaluate(), []);
    for (row, (formula, want)) in cases.iter().enumerate() {
        assert_eq!(&get(&book, &format!("A{}", row + 1)), want, "{formula}");
    }

    // A long sum and deep parentheses within the bounds compute.
    let mut book = Book::new();
    let sheet = book.sheet("Sheet1");
    let long = format!("=1{}", "+1".repeat(399));
    let deep = format!("={}2{}", "(".repeat(60), ")".repeat(60));
    book.formula(sheet, 1, 1, Some(&long), n(0.0));
    book.formula(sheet, 2, 1, Some(&deep), n(0.0));
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "A1"), n(400.0));
    assert_eq!(get(&book, "A2"), n(2.0));
}

#[test]
fn cells_compute_in_dependency_order_across_sheets() {
    // Each formula is added before the cells it reads.
    let mut book = book(
        &["Summary", "Data"],
        &[
            ("Summary", "A1", "=SUM(Data!B1:B3)"),
            ("Summary", "A2", "=SUM(Data!B:B)+Data!B3"),
            ("Data", "B3", "=B2+1"),
            ("Data", "B2", "=B1+1"),
            ("Summary", "A3", "=IF(Data!B3>11,\"big\",\"small\")"),
            ("Summary", "A4", "=IF(FALSE,1)"),
            ("Summary", "A5", "=ROW()*100+COLUMN()"),
            (
                "Summary",
                "A6",
                "=ROW(Data!C7)+COLUMN(Data!C7)+ROWS(A1:A5)+COLUMNS(A:C)",
            ),
            ("Summary", "A7", "=ROWS(A7:A9)"),
        ],
    );
    put(&mut book, "Data", "B1", n(10.0));
    put(&mut book, "Data", "C1", text("not counted"));
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "Data!B2"), n(11.0));
    assert_eq!(get(&book, "Data!B3"), n(12.0));
    assert_eq!(get(&book, "Summary!A1"), n(33.0));
    assert_eq!(get(&book, "Summary!A2"), n(45.0));
    assert_eq!(get(&book, "Summary!A3"), text("big"));
    assert_eq!(get(&book, "Summary!A4"), Value::Bool(false));
    assert_eq!(get(&book, "Summary!A5"), n(501.0));
    assert_eq!(get(&book, "Summary!A6"), n(7.0 + 3.0 + 5.0 + 3.0));
    // `ROWS` reads where its reference is, so a cell may name itself in it.
    assert_eq!(get(&book, "Summary!A7"), n(3.0));

    // A chain of 20,000 cells, each reading the one above, added from the bottom.
    let mut chain = Book::new();
    let sheet = chain.sheet("Chain");
    chain.set(sheet, 1, 1, n(1.0));
    for row in (2..=20_000).rev() {
        let formula = format!("=A{}+1", row - 1);
        chain.formula(sheet, row, 1, Some(&formula), n(0.0));
    }
    assert_eq!(chain.evaluate(), []);
    assert_eq!(chain.value(sheet, 20_000, 1), &n(20_000.0));
}

#[test]
fn defined_names_and_structured_references_resolve() {
    let mut book = book(&["Summary", "Data"], &[]);
    put(&mut book, "Data", "F1", n(0.5));
    book.define(Name {
        name: "Rate".into(),
        sheet: None,
        text: "Data!$F$1".into(),
    });
    book.define(Name {
        name: "Twice".into(),
        sheet: None,
        text: "Rate*2".into(),
    });
    book.define(Name {
        name: "Rate".into(),
        sheet: Some("Summary".into()),
        text: "0.25".into(),
    });
    // A table over A3:D6: a header row, two data rows and a totals row.
    book.table(Table {
        name: "Sales".into(),
        sheet: "Data".into(),
        rect: Rect {
            top: 3,
            left: 1,
            bottom: 6,
            right: 4,
        },
        header: true,
        totals: true,
        columns: vec!["Item".into(), "Qty".into(), "Price".into(), "Total".into()],
    });
    for (cell, value) in [
        ("A3", text("Item")),
        ("B3", text("Qty")),
        ("C3", text("Price")),
        ("D3", text("Total")),
        ("A4", text("Pen")),
        ("B4", n(2.0)),
        ("C4", n(1.5)),
        ("A5", text("Ink")),
        ("B5", n(3.0)),
        ("C5", n(4.0)),
    ] {
        put(&mut book, "Data", cell, value);
    }
    for (sheet, cell, formula) in [
        ("Summary", "A1", "=Rate"),
        ("Data", "E1", "=Rate"),
        ("Data", "E2", "=Twice"),
        ("Data", "E3", "=Summary!Rate"),
        ("Summary", "A4", "=SUM(Sales[Total])"),
        ("Summary", "A5", "=ROWS(Sales[#Data])+ROWS(Sales[#All])"),
        ("Summary", "A6", "=SUM(Sales[[#Data],[Qty]:[Price]])"),
        ("Summary", "A7", "=Sales[[#Headers],[Price]]"),
        ("Summary", "A8", "=Nowhere"),
        ("Summary", "A9", "=Sales[Missing]"),
        ("Data", "D4", "=[@Qty]*[@Price]"),
        (
            "Data",
            "D5",
            "=Sales[[#This Row],[Qty]]*Sales[[#This Row],[Price]]",
        ),
        ("Data", "D6", "=SUM(Sales[Total])"),
        ("Data", "E4", "=Sales[@Item]"),
    ] {
        put_formula(&mut book, sheet, cell, formula, n(0.0));
    }
    assert_eq!(book.evaluate(), []);
    // A name local to a sheet comes first on that sheet.
    assert_eq!(get(&book, "Summary!A1"), n(0.25));
    assert_eq!(get(&book, "Data!E1"), n(0.5));
    assert_eq!(get(&book, "Data!E2"), n(1.0));
    assert_eq!(get(&book, "Data!E3"), n(0.25));
    assert_eq!(get(&book, "Data!D4"), n(3.0));
    assert_eq!(get(&book, "Data!D5"), n(12.0));
    assert_eq!(get(&book, "Data!D6"), n(15.0));
    assert_eq!(get(&book, "Data!E4"), text("Pen"));
    assert_eq!(get(&book, "Summary!A4"), n(15.0));
    assert_eq!(get(&book, "Summary!A5"), n(6.0));
    assert_eq!(get(&book, "Summary!A6"), n(10.5));
    assert_eq!(get(&book, "Summary!A7"), text("Price"));
    assert_eq!(get(&book, "Summary!A8"), Value::Error(CellError::Name));
    assert_eq!(get(&book, "Summary!A9"), Value::Error(CellError::Ref));
}

#[test]
fn a_circular_reference_is_listed_and_not_computed() {
    let mut book = Book::new();
    for (cell, formula, stored) in [
        ("A1", "=B1+1", 5.0),
        ("B1", "=A1+1", 6.0),
        ("C1", "=A1*2", 0.0),
        ("D1", "=D1", 9.0),
        ("E1", "=SUM(E:E)", 4.0),
    ] {
        put_formula(&mut book, "Sheet1", cell, formula, n(stored));
    }
    assert_eq!(
        book.evaluate(),
        [
            missed("Sheet1!A1", Kind::Circular, "=B1+1"),
            missed("Sheet1!B1", Kind::Circular, "=A1+1"),
            missed("Sheet1!D1", Kind::Circular, "=D1"),
            missed("Sheet1!E1", Kind::Circular, "=SUM(E:E)"),
        ]
    );
    assert_eq!(get(&book, "A1"), n(5.0));
    assert_eq!(get(&book, "B1"), n(6.0));
    assert_eq!(get(&book, "D1"), n(9.0));
    // A cell that reads a circular cell reads its stored value.
    assert_eq!(get(&book, "C1"), n(10.0));
}

#[test]
fn an_external_reference_is_listed_and_never_followed() {
    let mut book = book(&["Shop"], &[]);
    for (cell, formula) in [
        ("A1", "=[1]Shop!A1"),
        ("A2", "='[Book.xlsx]Shop'!A1+1"),
        ("A3", "=SUM(Jan:Mar!B2)"),
        ("A4", "=Missing!A1"),
        ("A5", "='Q1 sales'!B2:B3"),
        ("A6", "=B1:B3"),
    ] {
        put_formula(&mut book, "Shop", cell, formula, n(7.0));
    }
    assert_eq!(
        book.evaluate(),
        [
            missed("Shop!A1", Kind::Reference, "[1]Shop!A1"),
            missed("Shop!A2", Kind::Reference, "'[Book.xlsx]Shop'!A1"),
            missed("Shop!A3", Kind::Reference, "Jan:Mar!B2"),
            missed("Shop!A4", Kind::Reference, "Missing!A1"),
            missed("Shop!A5", Kind::Reference, "'Q1 sales'!B2:B3"),
            missed("Shop!A6", Kind::Reference, "B1:B3"),
        ]
    );
    for row in 1..=6 {
        assert_eq!(get(&book, &format!("A{row}")), n(7.0));
    }
}

#[test]
fn a_function_outside_the_set_is_listed_and_its_cell_keeps_its_stored_result() {
    let mut book = Book::new();
    let sheet = book.sheet("Sheet1");
    put_formula(&mut book, "Sheet1", "A1", "=NPV(0.1,C:D)", n(7.0));
    put_formula(&mut book, "Sheet1", "A2", "=A1+1", n(0.0));
    // `IF` computes only the branch it takes.
    put_formula(&mut book, "Sheet1", "A3", "=IF(TRUE,1,NOPE())", n(0.0));
    put_formula(&mut book, "Sheet1", "A4", "=1+", n(3.0));
    put_formula(&mut book, "Sheet1", "A5", "=_xludf.MINE(1)", n(2.0));
    put_formula(&mut book, "Sheet1", "A6", "=IF(1)", n(1.0));
    book.formula(sheet, 7, 1, None, text("kept"));
    assert_eq!(
        book.evaluate(),
        [
            missed("Sheet1!A1", Kind::Function, "NPV"),
            missed("Sheet1!A4", Kind::Token, "=1+"),
            missed("Sheet1!A5", Kind::Function, "_XLUDF.MINE"),
            missed("Sheet1!A6", Kind::Function, "IF"),
            missed("Sheet1!A7", Kind::Token, ""),
        ]
    );
    assert_eq!(get(&book, "A1"), n(7.0));
    assert_eq!(get(&book, "A2"), n(8.0));
    assert_eq!(get(&book, "A3"), n(1.0));
    assert_eq!(get(&book, "A4"), n(3.0));
    assert_eq!(get(&book, "A7"), text("kept"));
}

/// The cases of each batch of the function set, in `tests/evaluate/`.
#[path = "evaluate/lookup.rs"]
mod lookup;
#[path = "evaluate/math.rs"]
mod math;
#[path = "evaluate/text.rs"]
mod text;

/// One function case: the cells of `Sheet1` it reads, a formula, and the value it gives. The
/// formula is put in `Sheet1!ZZ1`, which no case's cells use.
pub struct Case {
    pub cells: Vec<(&'static str, Value)>,
    pub formula: &'static str,
    pub want: Value,
}

fn same(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => (a - b).abs() <= 1e-9 * b.abs().max(1.0),
        _ => got == want,
    }
}

#[test]
fn every_function_of_the_set_has_a_case() {
    let cases: Vec<Case> = [math::cases(), text::cases(), lookup::cases()]
        .into_iter()
        .flatten()
        .collect();
    let mut wrong = Vec::new();
    for case in &cases {
        let mut book = Book::new();
        book.sheet("Sheet1");
        for (cell, value) in &case.cells {
            put(&mut book, "Sheet1", cell, value.clone());
        }
        put_formula(&mut book, "Sheet1", "ZZ1", case.formula, text("stored"));
        let missed = book.evaluate();
        let got = get(&book, "ZZ1");
        if !missed.is_empty() || !same(&got, &case.want) {
            wrong.push(format!(
                "{}: got {got:?}, want {:?}, missed {missed:?}",
                case.formula, case.want
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let named = |name: &str| {
        cases.iter().any(|case| {
            let upper = case.formula.to_ascii_uppercase();
            upper.match_indices(name).any(|(at, _)| {
                let before = upper[..at].chars().next_back();
                upper[at + name.len()..].starts_with('(')
                    && !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '.')
            })
        })
    };
    let owed: Vec<&str> = nvs_spreadsheet::eval::functions::FUNCTIONS
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !named(name))
        .collect();
    assert!(owed.is_empty(), "no case calls {owed:?}");
    assert_eq!(nvs_spreadsheet::eval::functions::FUNCTIONS.len(), 104);
}

#[test]
fn a_volatile_or_dynamic_function_is_listed() {
    let mut book = Book::new();
    book.sheet("Sheet1");
    let formulas = [
        "=TODAY()",
        "=NOW()",
        "=RAND()",
        "=RANDBETWEEN(1,6)",
        "=INDIRECT(\"B1\")",
        "=OFFSET(B1,1,0)",
    ];
    for (at, formula) in formulas.iter().enumerate() {
        put_formula(
            &mut book,
            "Sheet1",
            &format!("A{}", at + 1),
            formula,
            n(5.0),
        );
    }
    assert_eq!(
        book.evaluate(),
        [
            missed("Sheet1!A1", Kind::Function, "TODAY"),
            missed("Sheet1!A2", Kind::Function, "NOW"),
            missed("Sheet1!A3", Kind::Function, "RAND"),
            missed("Sheet1!A4", Kind::Function, "RANDBETWEEN"),
            missed("Sheet1!A5", Kind::Function, "INDIRECT"),
            missed("Sheet1!A6", Kind::Function, "OFFSET"),
        ]
    );
    for row in 1..=6 {
        assert_eq!(get(&book, &format!("A{row}")), n(5.0));
    }
}

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

fn formula(text: &str) -> Cell {
    Cell {
        value: Some(Written::Formula {
            formula: text.to_owned(),
            cached_text: None,
            cached_number: None,
        }),
        style: None,
    }
}

#[test]
fn a_computed_result_reads_by_the_read_table() {
    // `B1` is a formula under a date format, and row 2 is formulas alone, none with a cached
    // value: the writer wrote them, and only `evaluate` gives them a value.
    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    let mut options = SheetOptions::named("Sheet1");
    options.columns = vec![
        Column::default(),
        Column {
            format: Some("yyyy-mm-dd".to_owned()),
            ..Column::default()
        },
    ];
    let year = Cell {
        value: Some(Written::Number(2024.0)),
        style: None,
    };
    let rows = [
        Row {
            cells: vec![year, formula("DATE(A1,1,15)")],
            ..Row::default()
        },
        Row {
            cells: vec![formula("A1+1"), formula("B1+1")],
            ..Row::default()
        },
    ];
    let mut bytes = writer.sheet(options, &rows).unwrap();
    bytes.extend(writer.finish().unwrap().data);
    let source = Bytes(bytes);
    let mut reader = Reader::open(&source, Bound::default()).unwrap();
    assert_eq!(reader.evaluate(&source).unwrap(), []);
    let read = ReadOptions {
        sheet: "Sheet1".to_owned(),
        formulas: true,
        ..ReadOptions::default()
    };
    let batch = reader.rows(&source, &read, 100).unwrap();
    let values = |rows: &[ReadRow], number: u64| -> Vec<Option<Read>> {
        rows.iter()
            .find(|row| row.number == number)
            .unwrap_or_else(|| panic!("row {number} reads"))
            .cells
            .iter()
            .map(|cell| cell.value.clone())
            .collect()
    };
    assert_eq!(
        values(&batch.rows, 1),
        [
            Some(Read::Integer(2024)),
            Some(Read::Date(Date {
                year: 2024,
                month: 1,
                day: 15,
            })),
        ]
    );
    assert_eq!(
        values(&batch.rows, 2),
        [
            Some(Read::Integer(2025)),
            Some(Read::Date(Date {
                year: 2024,
                month: 1,
                day: 16,
            })),
        ]
    );
    let cached: Vec<Option<String>> = batch.rows[1]
        .cells
        .iter()
        .map(|cell| cell.formula.clone())
        .collect();
    assert_eq!(cached, [Some("A1+1".to_owned()), Some("B1+1".to_owned())]);
}

#[test]
fn a_table_column_named_with_a_hash_is_found_by_its_escaped_name() {
    let mut book = book(&["Shop"], &[]);
    book.table(Table {
        name: "Sales".into(),
        sheet: "Shop".into(),
        rect: Rect {
            top: 1,
            left: 1,
            bottom: 3,
            right: 2,
        },
        header: true,
        totals: false,
        columns: vec!["Item".into(), "#Orders".into()],
    });
    for (cell, value) in [
        ("A1", text("Item")),
        ("B1", text("#Orders")),
        ("A2", text("Pen")),
        ("B2", n(2.0)),
        ("A3", text("Ink")),
        ("B3", n(5.0)),
    ] {
        put(&mut book, "Shop", cell, value);
    }
    // `'#` is the escaped column name, and `#Data` with no `'` is an item.
    for (cell, formula) in [
        ("D1", "=SUM(Sales['#Orders])"),
        ("D2", "=SUM(Sales[[#Data],['#Orders]])"),
        ("D3", "=Sales[[#Headers],['#Orders]]"),
    ] {
        put_formula(&mut book, "Shop", cell, formula, n(0.0));
    }
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "Shop!D1"), n(7.0));
    assert_eq!(get(&book, "Shop!D2"), n(7.0));
    assert_eq!(get(&book, "Shop!D3"), text("#Orders"));
}

#[test]
fn a_date_text_in_a_1904_workbook_counts_from_1904() {
    let formulas = [
        ("Shop", "A1", "=\"2024-01-15\"+0"),
        ("Shop", "A2", "=\"1904-01-02 12:00\"*1"),
        ("Shop", "A3", "=-\"2024-01-15\""),
    ];
    let mut old = book(&["Shop"], &formulas);
    assert_eq!(old.evaluate(), []);
    assert_eq!(get(&old, "Shop!A1"), n(45306.0));
    let mut book = book(&["Shop"], &formulas);
    book.use_1904();
    assert_eq!(book.evaluate(), []);
    // 2024-01-15 is 1,462 days fewer from 1904-01-01 than from the 1900 system's start.
    assert_eq!(get(&book, "Shop!A1"), n(43844.0));
    assert_eq!(get(&book, "Shop!A2"), n(1.5));
    assert_eq!(get(&book, "Shop!A3"), n(-43844.0));
}

#[test]
fn text_that_needs_a_locale_to_be_a_number_is_listed_and_keeps_its_stored_value() {
    let mut book = book(&["Shop"], &[]);
    for (cell, value) in [
        ("A1", text("$1")),
        ("A2", text("1,000")),
        ("A3", text("1/15/2024")),
        ("A4", text("2")),
    ] {
        put(&mut book, "Shop", cell, value);
    }
    for (cell, formula, stored) in [
        ("B1", "=A1+1", n(7.0)),
        ("B2", "=-A2", n(8.0)),
        ("B3", "=A3*2", n(9.0)),
        ("B4", "=COUNTIF(A1:A3,1000)", n(1.0)),
        ("B5", "=COUNTIF(C1:C2,\"1,000\")", n(2.0)),
        ("B6", "=A4+1", n(0.0)),
        ("B7", "=B1&\"\"", n(0.0)),
    ] {
        put_formula(&mut book, "Shop", cell, formula, stored);
    }
    assert_eq!(
        book.evaluate(),
        [
            missed("Shop!B1", Kind::Coercion, "$1"),
            missed("Shop!B2", Kind::Coercion, "1,000"),
            missed("Shop!B3", Kind::Coercion, "1/15/2024"),
            missed("Shop!B4", Kind::Coercion, "$1"),
            missed("Shop!B5", Kind::Coercion, "1,000"),
        ]
    );
    // Each missed cell keeps its stored value, and a cell that reads one reads that value.
    assert_eq!(get(&book, "Shop!B1"), n(7.0));
    assert_eq!(get(&book, "Shop!B3"), n(9.0));
    assert_eq!(get(&book, "Shop!B6"), n(3.0));
    assert_eq!(get(&book, "Shop!B7"), text("7"));
    assert_eq!(Kind::Coercion.as_str(), "coercion");
}

#[test]
fn sheet_names_with_apostrophes_bangs_and_non_latin_letters_parse() {
    let p = |t: &str| parse::parse(t).unwrap();
    assert_eq!(p("='Shop''s'!A1"), area(Some("Shop's"), 1, 1, 1, 1));
    assert_eq!(p("='Q1!Sales'!A1"), area(Some("Q1!Sales"), 1, 1, 1, 1));
    assert_eq!(p("='Продажи'!A1"), area(Some("Продажи"), 1, 1, 1, 1));
    assert_eq!(p("=Продажи!B1"), area(Some("Продажи"), 1, 2, 1, 2));
    let mut book = book(
        &["Summary", "Shop's", "Q1!Sales", "Продажи"],
        &[
            ("Summary", "A1", "='Shop''s'!A1"),
            ("Summary", "A2", "='Q1!Sales'!A1"),
            ("Summary", "A3", "='Продажи'!A1"),
            ("Summary", "A4", "=Продажи!B1"),
        ],
    );
    put(&mut book, "Shop's", "A1", n(1.0));
    put(&mut book, "Q1!Sales", "A1", n(2.0));
    put(&mut book, "Продажи", "A1", n(3.0));
    put(&mut book, "Продажи", "B1", text("четыре"));
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "Summary!A1"), n(1.0));
    assert_eq!(get(&book, "Summary!A2"), n(2.0));
    assert_eq!(get(&book, "Summary!A3"), n(3.0));
    assert_eq!(get(&book, "Summary!A4"), text("четыре"));
}

#[test]
fn whole_column_ranges_and_chained_ranges_parse() {
    let p = |t: &str| parse::parse(t).unwrap();
    assert_eq!(
        p("=SUM(Sheet2!$B:$B)"),
        Expr::Call("SUM".into(), vec![area(Some("Sheet2"), 1, 2, 1_048_576, 2)])
    );
    assert_eq!(
        p("=A1:B2:C3"),
        Expr::Binary(
            Binary::Range,
            boxed(area(None, 1, 1, 2, 2)),
            boxed(area(None, 3, 3, 3, 3))
        )
    );
    let mut book = book(
        &["Sheet1", "Sheet2"],
        &[
            ("Sheet1", "E1", "=SUM(Sheet2!$B:$B)"),
            ("Sheet1", "E2", "=SUM(A1:B2:C3)"),
        ],
    );
    put(&mut book, "Sheet2", "B1", n(1.0));
    put(&mut book, "Sheet2", "B1000000", n(2.0));
    put(&mut book, "Sheet2", "C1", n(100.0));
    // `C1` is outside `A1:B2` and outside `C3`, but inside the box `A1:C3`.
    for (cell, value) in [
        ("A1", 1.0),
        ("B2", 2.0),
        ("C1", 4.0),
        ("C3", 8.0),
        ("D4", 16.0),
    ] {
        put(&mut book, "Sheet1", cell, n(value));
    }
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "Sheet1!E1"), n(3.0));
    assert_eq!(get(&book, "Sheet1!E2"), n(15.0));
}

#[test]
fn an_empty_argument_is_evaluated_as_excel_does() {
    let mut book = book(
        &["Sheet1"],
        &[
            ("Sheet1", "A1", "=SUM(1,)"),
            ("Sheet1", "A2", "=IF(FALSE,1,)"),
            ("Sheet1", "A3", "=IF(TRUE,,1)"),
        ],
    );
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "A1"), n(1.0));
    assert_eq!(get(&book, "A2"), n(0.0));
    assert_eq!(get(&book, "A3"), n(0.0));
}

#[test]
fn a_string_that_looks_like_a_reference_stays_a_string() {
    assert_eq!(parse::parse("=\"A1\"").unwrap(), Expr::Text("A1".into()));
    let mut book = book(
        &["Sheet1"],
        &[
            ("Sheet1", "C1", "=\"A1\""),
            ("Sheet1", "C2", "=LEN(\"B2:C3\")"),
        ],
    );
    put(&mut book, "Sheet1", "A1", n(42.0));
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "C1"), text("A1"));
    assert_eq!(get(&book, "C2"), n(5.0));
}

#[test]
fn names_that_look_like_cells_or_have_dots_or_non_ascii_letters_parse() {
    let p = |t: &str| parse::parse(t).unwrap();
    for name in ["A1B", "R1C", "x.y", "Größe"] {
        assert_eq!(
            p(&format!("={name}")),
            Expr::Name(None, name.into()),
            "{name}"
        );
    }
    let mut book = book(&["Sheet1"], &[]);
    for (name, value) in [("A1B", "1"), ("R1C", "2"), ("x.y", "4"), ("Größe", "8")] {
        book.define(Name {
            name: name.into(),
            sheet: None,
            text: value.into(),
        });
    }
    put_formula(&mut book, "Sheet1", "D1", "=A1B+R1C+x.y+Größe", n(0.0));
    put_formula(&mut book, "Sheet1", "D2", "=Größe", n(0.0));
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "D1"), n(15.0));
    assert_eq!(get(&book, "D2"), n(8.0));
}

#[test]
fn implicit_intersection_and_spill_references_are_listed() {
    // Neither form is computed: each cell is listed and keeps the value the file stored.
    let mut book = book(&["Sheet1"], &[]);
    for (cell, value) in [("A1", 1.0), ("A2", 2.0), ("A3", 3.0)] {
        put(&mut book, "Sheet1", cell, n(value));
    }
    for (cell, formula) in [
        ("B1", "=@A1:A3"),
        ("B2", "=A1#"),
        ("B3", "=_xlfn.SINGLE(A1:A3)"),
        ("B4", "=_xlfn.ANCHORARRAY(A1)"),
    ] {
        put_formula(&mut book, "Sheet1", cell, formula, n(9.0));
    }
    assert_eq!(
        book.evaluate(),
        [
            missed("Sheet1!B1", Kind::Token, "=@A1:A3"),
            missed("Sheet1!B2", Kind::Token, "=A1#"),
            missed("Sheet1!B3", Kind::Function, "SINGLE"),
            missed("Sheet1!B4", Kind::Function, "ANCHORARRAY"),
        ]
    );
    for row in 1..=4 {
        assert_eq!(get(&book, &format!("B{row}")), n(9.0));
    }
}

#[test]
fn chained_comparisons_evaluate_left_to_right() {
    assert_eq!(
        parse::parse("=1<2<3").unwrap(),
        Expr::Binary(
            Binary::Lt,
            boxed(Expr::Binary(
                Binary::Lt,
                boxed(Expr::Number(1.0)),
                boxed(Expr::Number(2.0))
            )),
            boxed(Expr::Number(3.0))
        )
    );
    // `1<2` is TRUE, and a boolean compares greater than any number, so `TRUE<3` is FALSE.
    let mut book = book(
        &["Sheet1"],
        &[
            ("Sheet1", "A1", "=1<2<3"),
            ("Sheet1", "A2", "=3>2>1"),
            ("Sheet1", "A3", "=1=1=TRUE"),
        ],
    );
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "A1"), Value::Bool(false));
    // `3>2` is TRUE, and `TRUE>1` is TRUE for the same reason.
    assert_eq!(get(&book, "A2"), Value::Bool(true));
    assert_eq!(get(&book, "A3"), Value::Bool(true));
}

#[test]
fn an_empty_string_is_not_equal_to_zero() {
    // Overlaps `operators_follow_excels_precedence_and_coercion` for an empty cell.
    let mut book = book(
        &["Sheet1"],
        &[
            ("Sheet1", "A1", "=\"\"=0"),
            ("Sheet1", "A2", "=0=\"\""),
            ("Sheet1", "A3", "=Z9=0"),
            ("Sheet1", "A4", "=Z9=\"\""),
            ("Sheet1", "A5", "=Z9=FALSE"),
        ],
    );
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "A1"), Value::Bool(false));
    assert_eq!(get(&book, "A2"), Value::Bool(false));
    assert_eq!(get(&book, "A3"), Value::Bool(true));
    assert_eq!(get(&book, "A4"), Value::Bool(true));
    assert_eq!(get(&book, "A5"), Value::Bool(true));
}

#[test]
fn an_error_inside_a_range_gives_that_error() {
    let mut book = book(
        &["Sheet1"],
        &[
            ("Sheet1", "B2", "=#REF!"),
            ("Sheet1", "C2", "=1/0"),
            ("Sheet1", "D1", "=SUM(A1:A3)"),
            ("Sheet1", "D2", "=SUM(B1:B3)"),
            ("Sheet1", "D3", "=SUM(C1:C3)"),
        ],
    );
    for col in ["A", "B", "C"] {
        put(&mut book, "Sheet1", &format!("{col}1"), n(1.0));
        put(&mut book, "Sheet1", &format!("{col}3"), n(2.0));
    }
    put(&mut book, "Sheet1", "A2", Value::Error(CellError::Ref));
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "D1"), Value::Error(CellError::Ref));
    assert_eq!(get(&book, "D2"), Value::Error(CellError::Ref));
    assert_eq!(get(&book, "D3"), Value::Error(CellError::DivZero));
}

#[test]
fn ifs_switch_and_choose_evaluate_only_the_branch_taken() {
    // `NOPE` is no function of the set: computing it would list the cell, so an empty list
    // shows the branch was never computed.
    let mut book = book(
        &["Sheet1"],
        &[
            ("Sheet1", "A1", "=IFS(TRUE,1,1/0,2)"),
            ("Sheet1", "A2", "=IFS(TRUE,1,NOPE(),NOPE())"),
            ("Sheet1", "A3", "=SWITCH(1,1,\"a\",2,1/0)"),
            ("Sheet1", "A4", "=SWITCH(1,1,\"a\",NOPE(),NOPE(),NOPE())"),
            ("Sheet1", "A5", "=CHOOSE(1,\"x\",1/0)"),
            ("Sheet1", "A6", "=CHOOSE(1,\"x\",NOPE())"),
        ],
    );
    assert_eq!(book.evaluate(), []);
    assert_eq!(get(&book, "A1"), n(1.0));
    assert_eq!(get(&book, "A2"), n(1.0));
    assert_eq!(get(&book, "A3"), text("a"));
    assert_eq!(get(&book, "A4"), text("a"));
    assert_eq!(get(&book, "A5"), text("x"));
    assert_eq!(get(&book, "A6"), text("x"));
}

#[test]
fn rounddown_and_roundup_correct_binary_noise() {
    let cases: &[(&str, f64)] = &[
        ("=ROUNDDOWN(249.9975,0)", 249.0),
        ("=ROUNDUP(280.000000000001,0)", 281.0),
        ("=ROUNDUP(2.2*3,1)", 6.6),
        ("=ROUNDDOWN(0.3-0.1,1)", 0.2),
        ("=ROUNDUP(0.1+0.2,1)", 0.3),
        ("=ROUNDDOWN(1.1*3,1)", 3.3),
    ];
    let mut book = Book::new();
    book.sheet("Sheet1");
    for (row, (formula, _)) in cases.iter().enumerate() {
        put_formula(
            &mut book,
            "Sheet1",
            &format!("A{}", row + 1),
            formula,
            n(0.0),
        );
    }
    assert_eq!(book.evaluate(), []);
    for (row, (formula, want)) in cases.iter().enumerate() {
        assert_eq!(get(&book, &format!("A{}", row + 1)), n(*want), "{formula}");
    }
}

#[test]
fn subtotal_skips_hidden_rows_and_nested_subtotals() {
    // Overlaps the SUBTOTAL cases in `evaluate/math.rs`, which have no nested or hidden rows.
    let mut book = book(
        &["Shown", "Hidden"],
        &[
            ("Shown", "A3", "=SUBTOTAL(9,A1:A2)"),
            ("Shown", "A5", "=SUBTOTAL(9,A1:A4)"),
            ("Shown", "A6", "=SUM(A1:A4)"),
            ("Hidden", "A4", "=SUBTOTAL(109,A1:A3)"),
            ("Hidden", "A5", "=SUBTOTAL(109,A1:A4)"),
            ("Hidden", "A6", "=SUBTOTAL(9,A1:A3)"),
        ],
    );
    for (cell, value) in [("A1", 1.0), ("A2", 2.0), ("A4", 4.0)] {
        put(&mut book, "Shown", cell, n(value));
    }
    for (cell, value) in [("A1", 10.0), ("A2", 20.0), ("A3", 30.0)] {
        put(&mut book, "Hidden", cell, n(value));
    }
    let hidden = book.sheet("Hidden");
    book.hide_row(hidden, 2);
    // Code 9 over a hidden row is listed: the file does not say whether a filter hid it.
    assert_eq!(
        book.evaluate(),
        [missed("Hidden!A6", Kind::Function, "SUBTOTAL")]
    );
    assert_eq!(get(&book, "Shown!A3"), n(3.0));
    assert_eq!(get(&book, "Shown!A5"), n(7.0));
    assert_eq!(get(&book, "Shown!A6"), n(10.0));
    assert_eq!(get(&book, "Hidden!A4"), n(40.0));
    assert_eq!(get(&book, "Hidden!A5"), n(40.0));
    assert_eq!(get(&book, "Hidden!A6"), n(0.0));
}
