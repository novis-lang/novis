use super::Case;
use nvs_spreadsheet::eval::Value;
use nvs_spreadsheet::read::CellError;

fn n(v: f64) -> Value {
    Value::Number(v)
}

fn t(v: &str) -> Value {
    Value::Text(v.to_owned())
}

fn e(v: CellError) -> Value {
    Value::Error(v)
}

/// Fruit names in `A1:A3` and prices in `B1:B3`, a sorted score column in `D1:D3` with grades in
/// `E1:E3`, and a descending column in `G1:G3`.
fn table() -> Vec<(&'static str, Value)> {
    vec![
        ("A1", t("Apple")),
        ("A2", t("Banana")),
        ("A3", t("Cherry")),
        ("B1", n(10.0)),
        ("B2", n(20.0)),
        ("B3", n(30.0)),
        ("D1", n(0.0)),
        ("D2", n(10.0)),
        ("D3", n(20.0)),
        ("E1", t("F")),
        ("E2", t("C")),
        ("E3", t("A")),
        ("G1", n(30.0)),
        ("G2", n(20.0)),
        ("G3", n(10.0)),
    ]
}

fn case(formula: &'static str, want: Value) -> Case {
    Case {
        cells: table(),
        formula,
        want,
    }
}

fn plain(formula: &'static str, want: Value) -> Case {
    Case {
        cells: Vec::new(),
        formula,
        want,
    }
}

pub fn cases() -> Vec<Case> {
    vec![
        // VLOOKUP and HLOOKUP
        case("=VLOOKUP(\"banana\",A1:B3,2,FALSE)", n(20.0)),
        case("=VLOOKUP(\"B*\",A1:B3,2,FALSE)", n(20.0)),
        case(
            "=VLOOKUP(\"Kiwi\",A1:B3,2,FALSE)",
            e(CellError::NotAvailable),
        ),
        case("=VLOOKUP(10,A1:B3,2,FALSE)", e(CellError::NotAvailable)),
        case("=VLOOKUP(\"Apple\",A1:B3,3,FALSE)", e(CellError::Ref)),
        case("=VLOOKUP(\"Apple\",A1:B3,0,FALSE)", e(CellError::Value)),
        case("=VLOOKUP(15,D1:E3,2,TRUE)", t("C")),
        case("=VLOOKUP(25,D1:E3,2)", t("A")),
        case("=VLOOKUP(10,D1:E3,2,TRUE)", t("C")),
        case("=VLOOKUP(-1,D1:E3,2,TRUE)", e(CellError::NotAvailable)),
        case("=VLOOKUP(15,D:E,2,TRUE)", t("C")),
        Case {
            cells: vec![
                ("A1", t("x")),
                ("B1", t("y")),
                ("C1", t("z")),
                ("A2", n(1.0)),
                ("B2", n(2.0)),
                ("C2", n(3.0)),
            ],
            formula: "=HLOOKUP(\"Y\",A1:C2,2,FALSE)",
            want: n(2.0),
        },
        // MATCH
        case("=MATCH(\"ch*\",A1:A3,0)", n(3.0)),
        case("=MATCH(\"?anana\",A1:A3,0)", n(2.0)),
        case("=MATCH(10,A1:A3,0)", e(CellError::NotAvailable)),
        case("=MATCH(20,D1:D3,1)", n(3.0)),
        case("=MATCH(15,D1:D3)", n(2.0)),
        case("=MATCH(15,G1:G3,-1)", n(2.0)),
        case("=MATCH(20,A1:B3,0)", e(CellError::NotAvailable)),
        // INDEX
        case("=INDEX(A1:B3,2,2)", n(20.0)),
        case("=INDEX(B1:B3,3)", n(30.0)),
        case("=INDEX(A1:B1,2)", n(10.0)),
        case("=INDEX(A1:B3,4,1)", e(CellError::Ref)),
        case("=SUM(INDEX(A1:B3,0,2))", n(60.0)),
        case("=SUM(INDEX(A1:B3,2,0))", n(20.0)),
        case("=SUM(INDEX(A1:B3,3))", n(30.0)),
        // CHOOSE
        plain("=CHOOSE(2,\"a\",\"b\",\"c\")", t("b")),
        plain("=CHOOSE(4,\"a\",\"b\")", e(CellError::Value)),
        case("=SUM(CHOOSE(1,B1:B3,A1))", n(60.0)),
        // XLOOKUP
        case("=XLOOKUP(\"cherry\",A1:A3,B1:B3)", n(30.0)),
        case("=XLOOKUP(\"Kiwi\",A1:A3,B1:B3,\"none\")", t("none")),
        case("=XLOOKUP(\"Kiwi\",A1:A3,B1:B3)", e(CellError::NotAvailable)),
        case("=XLOOKUP(\"B*\",A1:A3,B1:B3)", e(CellError::NotAvailable)),
        case("=XLOOKUP(\"?anana\",A1:A3,B1:B3,,2)", n(20.0)),
        case("=SUM(XLOOKUP(\"Banana\",A1:A3,A1:B3))", n(20.0)),
        case("=XLOOKUP(15,D1:D3,E1:E3,,-1)", t("C")),
        case("=XLOOKUP(15,D1:D3,E1:E3,,1)", t("A")),
        case("=XLOOKUP(10,D1:D3,E1:E3,,0,2)", t("C")),
        case("=XLOOKUP(15,G1:G3,D1:D3,,1,-2)", n(10.0)),
        case("=XLOOKUP(10,B1:B3,A1:A3,,0,-1)", t("Apple")),
        case("=XLOOKUP(10,B1:B3,A1:A2)", e(CellError::Value)),
        // ROW, COLUMN, ROWS and COLUMNS
        plain("=ROW(B3)", n(3.0)),
        plain("=COLUMN(C1)", n(3.0)),
        plain("=ROWS(A1:B3)", n(3.0)),
        plain("=COLUMNS(A1:B3)", n(2.0)),
        case("=ROW(INDEX(A1:B3,2,1))", n(2.0)),
        // DATE and its parts
        plain("=DATE(2024,1,15)", n(45306.0)),
        plain("=DATE(2024,14,1)", n(45689.0)),
        plain("=DATE(124,1,15)", n(45306.0)),
        plain("=DATE(2024,1,0)", n(45291.0)),
        plain("=DATE(1900,2,29)", n(60.0)),
        plain("=DATE(-1,1,1)", e(CellError::Num)),
        plain("=YEAR(45306)", n(2024.0)),
        plain("=MONTH(45306)", n(1.0)),
        plain("=DAY(45306.75)", n(15.0)),
        plain("=DAY(\"2024-01-15\")", n(15.0)),
        plain("=DAY(60)", n(29.0)),
        plain("=MONTH(60)", n(2.0)),
        plain("=DAY(0)", n(0.0)),
        plain("=MONTH(0)", n(1.0)),
        plain("=YEAR(0)", n(1900.0)),
        plain("=YEAR(-1)", e(CellError::Num)),
        plain("=YEAR(2958466)", e(CellError::Num)),
        // DAYS, WEEKDAY, EDATE and EOMONTH
        plain("=DAYS(45351,45306)", n(45.0)),
        plain("=WEEKDAY(45306)", n(2.0)),
        plain("=WEEKDAY(45306,2)", n(1.0)),
        plain("=WEEKDAY(45306,3)", n(0.0)),
        plain("=WEEKDAY(45306,17)", n(2.0)),
        plain("=WEEKDAY(45306,4)", e(CellError::Num)),
        plain("=WEEKDAY(1)", n(1.0)),
        plain("=WEEKDAY(0)", n(7.0)),
        plain("=EOMONTH(45306,1)", n(45351.0)),
        plain("=EOMONTH(45306,-1)", n(45291.0)),
        plain("=EDATE(45322,1)", n(45351.0)),
        plain("=EDATE(45306,12)", n(45672.0)),
        // DATEDIF
        plain("=DATEDIF(DATE(2020,5,15),DATE(2024,5,14),\"Y\")", n(3.0)),
        plain("=DATEDIF(DATE(2020,5,15),DATE(2024,5,14),\"M\")", n(47.0)),
        plain("=DATEDIF(45306,45351,\"D\")", n(45.0)),
        plain("=DATEDIF(DATE(2020,5,15),DATE(2024,8,10),\"YM\")", n(2.0)),
        plain("=DATEDIF(DATE(2024,1,15),DATE(2024,3,10),\"MD\")", n(24.0)),
        plain("=DATEDIF(45351,45306,\"D\")", e(CellError::Num)),
        // TIME and its parts
        plain("=TIME(25,0,0)", n(1.0 / 24.0)),
        plain("=TIME(12,30,0)", n(45_000.0 / 86_400.0)),
        plain("=TIME(0,-1,0)", e(CellError::Num)),
        plain("=HOUR(0.75)", n(18.0)),
        plain("=HOUR(\"13:30\")", n(13.0)),
        plain("=MINUTE(TIME(10,45,30))", n(45.0)),
        plain("=SECOND(TIME(10,45,30))", n(30.0)),
        plain("=SECOND(TIME(0,0,58)+0.6/86400)", n(59.0)),
    ]
}
