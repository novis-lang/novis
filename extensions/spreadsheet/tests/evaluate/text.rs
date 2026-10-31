//! The cases of the text functions.

use super::Case;
use nvs_spreadsheet::eval::Value;
use nvs_spreadsheet::read::CellError;

fn t(s: &str) -> Value {
    Value::Text(s.to_owned())
}

fn n(v: f64) -> Value {
    Value::Number(v)
}

fn wrong() -> Value {
    Value::Error(CellError::Value)
}

fn case(formula: &'static str, want: Value) -> Case {
    Case {
        cells: Vec::new(),
        formula,
        want,
    }
}

pub fn cases() -> Vec<Case> {
    vec![
        case("=CHAR(65)", t("A")),
        case("=CHAR(128)", t("\u{20AC}")),
        case("=CHAR(150.9)", t("\u{2013}")),
        case("=CHAR(0)", wrong()),
        case("=CHAR(256)", wrong()),
        case("=CODE(\"A\")", n(65.0)),
        case("=CODE(\"\u{20AC}uro\")", n(128.0)),
        case("=CODE(\"\")", wrong()),
        case("=CLEAN(\"a\"&CHAR(9)&\"b\"&CHAR(10))", t("ab")),
        Case {
            cells: vec![("A1", t("a")), ("B1", t("b")), ("A2", n(1.0))],
            formula: "=CONCAT(A1:B2,\"!\")",
            want: t("ab1!"),
        },
        case("=CONCAT(\"x\",1.5,TRUE)", t("x1.5TRUE")),
        case("=CONCATENATE(\"a\",\"b\",1)", t("ab1")),
        case(
            "=CONCATENATE(\"a\",#N/A)",
            Value::Error(CellError::NotAvailable),
        ),
        case("=EXACT(\"abc\",\"abc\")", Value::Bool(true)),
        case("=EXACT(\"a\",\"A\")", Value::Bool(false)),
        case("=FIND(\"b\",\"abcb\",3)", n(4.0)),
        case("=FIND(\"B\",\"abc\")", wrong()),
        case("=FIND(\"\",\"abc\",2)", n(2.0)),
        case("=FIND(\"a\",\"abc\",0)", wrong()),
        case("=LEFT(\"abc\",2)", t("ab")),
        case("=LEFT(\"abc\")", t("a")),
        case("=LEFT(\"abc\",9)", t("abc")),
        case("=LEFT(\"abc\",-1)", wrong()),
        case("=LEN(\"abc\")", n(3.0)),
        case("=LEN(\"\u{1F600}\")", n(2.0)),
        case("=LEN(123.5)", n(5.0)),
        case("=LOWER(\"AbC D\")", t("abc d")),
        case("=MID(\"abcdef\",2,3)", t("bcd")),
        case("=MID(\"abc\",5,2)", t("")),
        case("=MID(\"abc\",0,2)", wrong()),
        case("=PROPER(\"2nd hello-world\")", t("2Nd Hello-World")),
        case("=PROPER(\"hELLO wORLD\")", t("Hello World")),
        case("=REPLACE(\"abcdef\",2,3,\"X\")", t("aXef")),
        case("=REPLACE(\"abc\",9,1,\"X\")", t("abcX")),
        case("=REPT(\"ab\",3)", t("ababab")),
        case("=REPT(\"a\",40000)", wrong()),
        case("=RIGHT(\"abc\",2)", t("bc")),
        case("=RIGHT(\"abc\")", t("c")),
        case("=SEARCH(\"B\",\"abcb\")", n(2.0)),
        case("=SEARCH(\"b?d\",\"abcd\")", n(2.0)),
        case("=SEARCH(\"c*e\",\"abcde\")", n(3.0)),
        case("=SEARCH(\"~*\",\"a*b\")", n(2.0)),
        case("=SEARCH(\"x\",\"abc\")", wrong()),
        case("=SUBSTITUTE(\"a-b-c\",\"-\",\"+\",2)", t("a-b+c")),
        case("=SUBSTITUTE(\"a-b-c\",\"-\",\"\")", t("abc")),
        case("=SUBSTITUTE(\"a-b-c\",\"-\",\"+\",3)", t("a-b-c")),
        case("=TEXT(1234.5,\"#,##0.00\")", t("1,234.50")),
        case("=TEXT(45306,\"yyyy-mm-dd\")", t("2024-01-15")),
        case("=TEXT(\"12\",\"0.0\")", t("12.0")),
        case("=TEXT(0.5,\"0%\")", t("50%")),
        Case {
            cells: vec![("A1", t("a")), ("A3", t("c"))],
            formula: "=TEXTJOIN(\",\",TRUE,A1:A3)",
            want: t("a,c"),
        },
        Case {
            cells: vec![("A1", t("a")), ("A3", t("c"))],
            formula: "=TEXTJOIN(\"-\",FALSE,A1:A3)",
            want: t("a--c"),
        },
        case("=TEXTJOIN(\" \",TRUE,\"a\",\"\",\"b\")", t("a b")),
        case("=TRIM(\"  a   b  \")", t("a b")),
        case("=UPPER(\"abc\")", t("ABC")),
        case("=VALUE(\"12\")", n(12.0)),
        case("=VALUE(\"50%\")", n(0.5)),
        case("=VALUE(\"abc\")", wrong()),
    ]
}
