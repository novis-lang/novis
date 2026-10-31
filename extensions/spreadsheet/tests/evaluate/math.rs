//! The cases of the math, statistics, logic and information functions.

use super::Case;
use nvs_spreadsheet::eval::Value;
use nvs_spreadsheet::read::CellError;

fn n(v: f64) -> Value {
    Value::Number(v)
}

fn t(s: &str) -> Value {
    Value::Text(s.into())
}

fn b(v: bool) -> Value {
    Value::Bool(v)
}

fn e(err: CellError) -> Value {
    Value::Error(err)
}

fn case(cells: Vec<(&'static str, Value)>, formula: &'static str, want: Value) -> Case {
    Case {
        cells,
        formula,
        want,
    }
}

/// A column of mixed values: 1, 2, a text, a boolean and 4, with `A6` blank.
fn mixed() -> Vec<(&'static str, Value)> {
    vec![
        ("A1", n(1.0)),
        ("A2", n(2.0)),
        ("A3", t("x")),
        ("A4", b(true)),
        ("A5", n(4.0)),
    ]
}

/// Keys in `A1:A3` and amounts in `B1:B3`.
fn keyed() -> Vec<(&'static str, Value)> {
    vec![
        ("A1", t("x")),
        ("A2", t("y")),
        ("A3", t("x")),
        ("B1", n(1.0)),
        ("B2", n(2.0)),
        ("B3", n(3.0)),
    ]
}

/// Words and numbers for criteria, with an empty text in `A7` and `A8` blank.
fn words() -> Vec<(&'static str, Value)> {
    vec![
        ("A1", t("apple")),
        ("A2", t("Apricot")),
        ("A3", t("banana")),
        ("A4", n(5.0)),
        ("A5", n(10.0)),
        ("A6", t("5")),
        ("A7", t("")),
    ]
}

pub fn cases() -> Vec<Case> {
    use CellError::{DivZero, NotAvailable, Num};
    let pi = std::f64::consts::PI;
    vec![
        // Aggregates.
        case(mixed(), "=SUM(A1:A6)", n(7.0)),
        case(vec![], "=SUM(1,\"3\",TRUE)", n(5.0)),
        case(
            vec![("A1", n(1.0)), ("A2", e(DivZero))],
            "=SUM(A1:A2)",
            e(DivZero),
        ),
        case(mixed(), "=AVERAGE(A1:A6)", n(7.0 / 3.0)),
        case(mixed(), "=AVERAGE(A3)", e(DivZero)),
        case(mixed(), "=COUNT(A1:A6)", n(3.0)),
        case(vec![], "=COUNT(1,\"2\",\"x\",TRUE)", n(3.0)),
        case(mixed(), "=COUNTA(A1:A6)", n(5.0)),
        case(mixed(), "=COUNTBLANK(A1:A6)", n(1.0)),
        case(
            vec![("A1", t("")), ("A2", n(1.0))],
            "=COUNTBLANK(A1:A3)",
            n(2.0),
        ),
        case(mixed(), "=MAX(A1:A6)", n(4.0)),
        case(mixed(), "=MAX(A3)", n(0.0)),
        case(vec![], "=MAX(1,1/0)", e(DivZero)),
        case(mixed(), "=MIN(A1:A6)", n(1.0)),
        case(mixed(), "=MEDIAN(A1:A6)", n(2.0)),
        case(vec![], "=MEDIAN(1,2,3,4)", n(2.5)),
        case(mixed(), "=PRODUCT(A1:A6)", n(8.0)),
        case(vec![], "=PRODUCT(2,3)", n(6.0)),
        // Math.
        case(vec![], "=ABS(-2)", n(2.0)),
        case(vec![], "=EXP(1)", n(std::f64::consts::E)),
        case(vec![], "=LN(EXP(2))", n(2.0)),
        case(vec![], "=LN(0)", e(Num)),
        case(vec![], "=LOG(8,2)", n(3.0)),
        case(vec![], "=LOG(100)", n(2.0)),
        case(vec![], "=LOG10(1000)", n(3.0)),
        case(vec![], "=INT(-2.5)", n(-3.0)),
        case(vec![], "=MOD(-3,2)", n(1.0)),
        case(vec![], "=MOD(3,-2)", n(-1.0)),
        case(vec![], "=MOD(5,0)", e(DivZero)),
        case(vec![], "=PI()", n(pi)),
        case(vec![], "=POWER(2,10)", n(1024.0)),
        case(vec![], "=SQRT(16)", n(4.0)),
        case(vec![], "=SQRT(-1)", e(Num)),
        case(vec![], "=SIGN(-5)", n(-1.0)),
        case(vec![], "=ROUND(2.675,2)", n(2.68)),
        case(vec![], "=ROUND(-2.5,0)", n(-3.0)),
        case(vec![], "=ROUND(1234.5,-2)", n(1200.0)),
        case(vec![], "=ROUNDUP(3.14159,2)", n(3.15)),
        case(vec![], "=ROUNDUP(-3.14159,1)", n(-3.2)),
        case(vec![], "=ROUNDDOWN(3.999,1)", n(3.9)),
        case(vec![], "=TRUNC(-8.9)", n(-8.0)),
        case(vec![], "=TRUNC(8.97,1)", n(8.9)),
        case(vec![], "=CEILING(2.5,1)", n(3.0)),
        case(vec![], "=CEILING(-2.5,2)", n(-2.0)),
        case(vec![], "=CEILING(-2.5,-2)", n(-4.0)),
        case(vec![], "=CEILING(1.5,0.1)", n(1.5)),
        case(vec![], "=CEILING(0.234,0.01)", n(0.24)),
        case(vec![], "=CEILING(2.5,-2)", e(Num)),
        case(vec![], "=FLOOR(3.7,2)", n(2.0)),
        case(vec![], "=FLOOR(-2.5,-2)", n(-2.0)),
        case(vec![], "=FLOOR(1.58,0.1)", n(1.5)),
        case(vec![], "=FLOOR(0.234,0.01)", n(0.23)),
        case(vec![], "=FLOOR(2.5,-2)", e(Num)),
        case(vec![], "=CEILING.MATH(24.3,5)", n(25.0)),
        case(vec![], "=CEILING.MATH(6.7)", n(7.0)),
        case(vec![], "=CEILING.MATH(-8.1,2)", n(-8.0)),
        case(vec![], "=CEILING.MATH(-5.5,2,-1)", n(-6.0)),
        case(vec![], "=FLOOR.MATH(24.3,5)", n(20.0)),
        case(vec![], "=FLOOR.MATH(6.7)", n(6.0)),
        case(vec![], "=FLOOR.MATH(-8.1,2)", n(-10.0)),
        case(vec![], "=FLOOR.MATH(-5.5,2,-1)", n(-4.0)),
        case(keyed(), "=SUMPRODUCT(B1:B3,B1:B3)", n(14.0)),
        case(keyed(), "=SUMPRODUCT(A1:A3,B1:B3)", n(0.0)),
        case(keyed(), "=SUMPRODUCT(B1:B3,B1:B2)", e(CellError::Value)),
        // SUBTOTAL.
        case(keyed(), "=SUBTOTAL(9,B1:B3)", n(6.0)),
        case(keyed(), "=SUBTOTAL(101,B1:B3)", n(2.0)),
        case(keyed(), "=SUBTOTAL(2,A1:B3)", n(3.0)),
        case(keyed(), "=SUBTOTAL(3,A1:B3)", n(6.0)),
        case(keyed(), "=SUBTOTAL(7,B1:B3)", n(1.0)),
        case(keyed(), "=SUBTOTAL(110,B1:B3)", n(1.0)),
        case(keyed(), "=SUBTOTAL(8,B1:B3)", n((2.0f64 / 3.0).sqrt())),
        case(keyed(), "=SUBTOTAL(12,B1:B3)", e(CellError::Value)),
        // Criteria.
        case(words(), "=COUNTIF(A1:A7,\"ap*\")", n(2.0)),
        case(words(), "=COUNTIF(A1:A7,\"?ANANA\")", n(1.0)),
        case(words(), "=COUNTIF(A1:A7,\"=apple\")", n(1.0)),
        case(words(), "=COUNTIF(A1:A7,\">5\")", n(1.0)),
        case(words(), "=COUNTIF(A1:A7,\">=5\")", n(2.0)),
        case(words(), "=COUNTIF(A1:A5,\"<>apple\")", n(4.0)),
        case(words(), "=COUNTIF(A1:A8,\"\")", n(2.0)),
        case(words(), "=COUNTIF(A:A,\"<>apple\")", n(1_048_575.0)),
        case(
            vec![("A1", t("a*b")), ("A2", t("axb"))],
            "=COUNTIF(A1:A2,\"a~*b\")",
            n(1.0),
        ),
        case(keyed(), "=COUNTIFS(A1:A3,\"x\",B1:B3,\">1\")", n(1.0)),
        case(
            keyed(),
            "=COUNTIFS(A1:A3,\"x\",B1:B2,\">1\")",
            e(CellError::Value),
        ),
        case(keyed(), "=SUMIF(A1:A3,\"x\",B1:B3)", n(4.0)),
        case(keyed(), "=SUMIF(A1:A3,\"x\",B1)", n(4.0)),
        case(keyed(), "=SUMIF(B1:B3,\">1\")", n(5.0)),
        case(
            vec![
                ("A1", n(1.0)),
                ("A2", n(2.0)),
                ("A3", n(3.0)),
                ("B1", n(10.0)),
                ("B2", n(20.0)),
                ("B3", n(30.0)),
            ],
            "=SUMIF(A:A,\">1\",B:B)",
            n(50.0),
        ),
        case(keyed(), "=SUMIFS(B1:B3,A1:A3,\"x\",B1:B3,\"<3\")", n(1.0)),
        case(keyed(), "=AVERAGEIF(A1:A3,\"x\",B1:B3)", n(2.0)),
        case(keyed(), "=AVERAGEIF(A1:A3,\"z\",B1:B3)", e(DivZero)),
        case(keyed(), "=AVERAGEIFS(B1:B3,A1:A3,\"x\")", n(2.0)),
        case(keyed(), "=MAXIFS(B1:B3,A1:A3,\"x\")", n(3.0)),
        case(keyed(), "=MAXIFS(B1:B3,A1:A3,\"z\")", n(0.0)),
        case(keyed(), "=MINIFS(B1:B3,A1:A3,\"x\")", n(1.0)),
        // Logic.
        case(vec![], "=IF(1>0,\"yes\",\"no\")", t("yes")),
        case(vec![], "=IF(FALSE,1)", b(false)),
        case(vec![], "=AND(TRUE,1)", b(true)),
        case(vec![], "=AND(TRUE,0)", b(false)),
        case(vec![("A1", t("x"))], "=AND(A1)", e(CellError::Value)),
        case(vec![], "=OR(FALSE,0)", b(false)),
        case(
            vec![("A1", b(false)), ("A2", b(true))],
            "=OR(A1:A2)",
            b(true),
        ),
        case(vec![], "=XOR(TRUE,TRUE)", b(false)),
        case(vec![], "=XOR(TRUE,FALSE,FALSE)", b(true)),
        case(vec![], "=NOT(FALSE)", b(true)),
        case(vec![], "=TRUE()", b(true)),
        case(vec![], "=FALSE()", b(false)),
        case(vec![], "=IFERROR(1/0,\"bad\")", t("bad")),
        case(vec![], "=IFERROR(5,1/0)", n(5.0)),
        case(vec![("A1", e(NotAvailable))], "=IFERROR(A1,0)", n(0.0)),
        case(vec![], "=IFNA(NA(),1)", n(1.0)),
        case(vec![], "=IFNA(1/0,1)", e(DivZero)),
        case(vec![], "=IFS(1>2,\"a\",2>1,\"b\")", t("b")),
        case(vec![], "=IFS(FALSE,1)", e(NotAvailable)),
        case(vec![], "=SWITCH(2,1,\"one\",2,\"two\")", t("two")),
        case(vec![], "=SWITCH(3,1,\"one\",\"other\")", t("other")),
        case(vec![], "=SWITCH(3,1,\"one\")", e(NotAvailable)),
        // Information.
        case(vec![], "=ISBLANK(A1)", b(true)),
        case(vec![], "=ISBLANK(\"\")", b(false)),
        case(vec![], "=ISERR(NA())", b(false)),
        case(vec![], "=ISERR(1/0)", b(true)),
        case(vec![], "=ISERROR(NA())", b(true)),
        case(vec![], "=ISNA(NA())", b(true)),
        case(vec![], "=ISNA(1/0)", b(false)),
        case(vec![], "=ISEVEN(2.5)", b(true)),
        case(vec![], "=ISODD(-3)", b(true)),
        case(vec![], "=ISLOGICAL(TRUE)", b(true)),
        case(vec![], "=ISNUMBER(\"1\")", b(false)),
        case(vec![("A1", n(1.0))], "=ISNUMBER(A1)", b(true)),
        case(vec![], "=ISTEXT(\"a\")", b(true)),
        case(vec![], "=NA()", e(NotAvailable)),
    ]
}
