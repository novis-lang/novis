//! The number-format renderer: a value as Excel displays it under its format code
//! (`rule:core-classes/spreadsheet-reads-as-a-stream`'s `text` option).

use nvs_spreadsheet::format::{
    self, Condition, Locale, Op, Options, Separators, Token, Value, locale,
};
use nvs_spreadsheet::openformula;
use nvs_spreadsheet::read::format::{builtin as builtin_kind, classify};

fn render(code: &str, value: f64) -> String {
    format::render(code, Value::Number(value), &Options::default())
}

fn in_1904(code: &str, value: f64) -> String {
    let options = Options {
        date1904: true,
        ..Options::default()
    };
    format::render(code, Value::Number(value), &options)
}

fn in_locale(code: &str, value: f64, tag: &str) -> String {
    let options = Options {
        locale: Locale::of_tag(tag),
        ..Options::default()
    };
    format::render(code, Value::Number(value), &options)
}

#[test]
fn every_row_of_the_reference_table_renders_as_excel_displays_it() {
    let table = include_str!("fixtures/number-formats.txt");
    let mut rows = 0;
    let mut wrong = Vec::new();
    for line in table.lines() {
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let (row, end) = line.rsplit_once('|').expect("a row ends with `|`");
        assert!(end.is_empty(), "{line}: text after the last `|`");
        let mut parts = row.splitn(3, '|');
        let (code, value, text) = (
            parts.next().unwrap(),
            parts.next().expect("a value"),
            parts.next().expect("a text"),
        );
        let value = if let Some(text) = value.strip_prefix("t:") {
            Value::Text(text)
        } else if let Some(flag) = value.strip_prefix("b:") {
            Value::Boolean(flag == "true")
        } else {
            Value::Number(value.parse().expect("a number"))
        };
        let shown = format::render(code, value, &Options::default());
        if shown != text {
            wrong.push(format!("{code} with {value:?}: `{shown}`, want `{text}`"));
        }
        rows += 1;
    }
    assert!(rows > 100, "the table has {rows} rows");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_built_in_format_ids_map_to_their_codes() {
    for id in 0..=49 {
        let code = format::builtin(id).unwrap_or_else(|| panic!("id {id} has no code"));
        if let Some(kind) = builtin_kind(id) {
            assert_eq!(classify(code), kind, "id {id}: `{code}`");
        }
    }
    assert_eq!(format::builtin(0), Some("General"));
    assert_eq!(format::builtin(14), Some("m/d/yyyy"));
    assert_eq!(format::builtin(46), Some("[h]:mm:ss"));
    assert_eq!(format::builtin(49), Some("@"));
    assert_eq!(format::builtin(164), None);
    assert_eq!(render(format::builtin(3).unwrap(), 1234.0), "1,234");
    assert_eq!(render(format::builtin(10).unwrap(), 0.5), "50.00%");
    assert_eq!(render(format::builtin(7).unwrap(), -2.5), "($2.50)");
    // The `*` fill has no cell to fill, so the accounting format shows its parts side by side.
    assert_eq!(render(format::builtin(44).unwrap(), 0.0), " $-   ");
}

#[test]
fn sections_conditions_and_colours_parse() {
    assert_eq!(
        format::split(r#"[Red]0;"a;b"0;\;0;@"#),
        vec!["[Red]0", r#""a;b"0"#, r"\;0", "@"]
    );
    let section = format::lex("[Red][>=100]#,##0.0\"kg\"");
    assert_eq!(
        section.condition,
        Some(Condition {
            op: Op::GreaterEqual,
            value: 100.0
        })
    );
    assert_eq!(
        section.tokens,
        vec![
            Token::Digit('#'),
            Token::Comma,
            Token::Digit('#'),
            Token::Digit('#'),
            Token::Digit('0'),
            Token::Point,
            Token::Digit('0'),
            Token::Literal("kg".to_owned()),
        ]
    );
    let section = format::lex("[$€-407]0");
    assert_eq!(section.locale, Some(0x407));
    assert_eq!(section.tokens[0], Token::Literal("€".to_owned()));
    let sections: Vec<_> = ["[<=-1]0", "[>=1]0", "\"between\""]
        .into_iter()
        .map(format::lex)
        .collect();
    assert_eq!(format::choose(&sections, -3.0), (0, false));
    assert_eq!(format::choose(&sections, 3.0), (1, false));
    assert_eq!(format::choose(&sections, 0.5), (2, false));
    assert_eq!(render("[Green]0.0;[Magenta]-0.0;[Color10]0", -1.25), "-1.3");
    assert_eq!(
        render("[<=-1]\"low\";[>=1]\"high\";\"between\"", 0.5),
        "between"
    );
    assert_eq!(render("_(* #,##0_)", 12.0), " 12 ");
}

#[test]
fn elapsed_time_and_both_epochs_render() {
    assert_eq!(render("[h]:mm:ss", 1.5), "36:00:00");
    assert_eq!(render("[h]:mm:ss", -0.25), "-6:00:00");
    assert_eq!(render("[mm]:ss", 1.0), "1440:00");
    assert_eq!(render("[ss].00", 0.5 / 86_400.0), "00.50");
    assert_eq!(render("yyyy-mm-dd", 45306.0), "2024-01-15");
    assert_eq!(in_1904("yyyy-mm-dd", 0.0), "1904-01-01");
    assert_eq!(in_1904("yyyy-mm-dd", 43844.0), "2024-01-15");
    assert_eq!(in_1904("dddd", 0.0), "Friday");
    assert_eq!(in_1904("dddd", 43844.0), "Monday");
    // A serial no date has shows in General.
    assert_eq!(render("yyyy-mm-dd", -1.0), "-1");
    assert_eq!(render("yyyy-mm-dd", 2_958_466.0), "2958466");
    assert_eq!(render("yyyy-mm-dd", 2_958_465.0), "9999-12-31");
}

#[test]
fn the_1900_leap_day_bug_is_kept() {
    assert_eq!(render("yyyy-mm-dd", 0.0), "1900-01-00");
    assert_eq!(render("yyyy-mm-dd", 1.0), "1900-01-01");
    assert_eq!(render("yyyy-mm-dd", 59.0), "1900-02-28");
    assert_eq!(render("yyyy-mm-dd", 60.0), "1900-02-29");
    assert_eq!(render("yyyy-mm-dd", 61.0), "1900-03-01");
    assert_eq!(render("dddd", 1.0), "Sunday");
    assert_eq!(render("dddd", 60.0), "Wednesday");
    assert_eq!(render("dddd", 61.0), "Thursday");
}

#[test]
fn separators_follow_the_locale_and_a_locale_prefix() {
    assert_eq!(in_locale("#,##0.00", 1234567.5, "en"), "1,234,567.50");
    assert_eq!(in_locale("#,##0.00", 1234567.5, "de"), "1.234.567,50");
    assert_eq!(in_locale("#,##0.00", 1234567.5, "de-CH"), "1'234'567.50");
    assert_eq!(
        in_locale("#,##0.00", 1234567.5, "fr"),
        "1\u{202f}234\u{202f}567,50"
    );
    assert_eq!(
        in_locale("#,##0.00", 1234567.5, "sv-SE"),
        "1\u{a0}234\u{a0}567,50"
    );
    assert_eq!(in_locale("#,##0.00", 1234567.5, "ja"), "1,234,567.50");
    assert_eq!(in_locale("#,##0.00", 1234567.5, "xx"), "1,234,567.50");
    assert_eq!(in_locale("General", 0.25, "de"), "0,25");
    assert_eq!(in_locale("0.0%", 0.125, "pl"), "12,5%");
    assert_eq!(render("[$€-407]#,##0.00", 1234.5), "€1.234,50");
    assert_eq!(render("[$-1010407]0.00", 1.5), "1,50");
    assert_eq!(in_locale("[$-409]#,##0.00", 1234.5, "de"), "1,234.50");
    assert_eq!(locale::of_tag("de_AT"), locale::of_tag("de-AT"));
    assert_eq!(locale::of_id(0xFFFF), None);
    assert_eq!(locale::of_tag("en-US"), Separators::EN);
}

/// 45306 is Monday 2024-01-15.
const MONDAY: f64 = 45306.0;

#[test]
fn month_and_day_names_follow_the_locale() {
    assert_eq!(
        in_locale("dddd, d. mmmm yyyy", MONDAY, "de"),
        "Montag, 15. Januar 2024"
    );
    assert_eq!(in_locale("ddd d mmm", MONDAY, "de-CH"), "Mo. 15 Jan.");
    assert_eq!(in_locale("dddd d mmmm", MONDAY, "fr"), "lundi 15 janvier");
    assert_eq!(in_locale("d mmmm", MONDAY, "ru"), "15 января");
    assert_eq!(in_locale("ddd", MONDAY, "pl"), "pon.");
    assert_eq!(in_locale("mmmm", 45366.0, "de"), "März");
    // `mmmmm` is the first letter of the full name, which may be more than one byte.
    assert_eq!(in_locale("mmmmm", MONDAY, "el"), "Ι");
    assert_eq!(in_locale("dddd", MONDAY, "no"), "mandag");
    // A `[$-407]` prefix picks German names whatever the read's locale is.
    assert_eq!(render("[$-407]dddd mmmm", MONDAY), "Montag Januar");
    assert_eq!(
        in_locale("[$-409]dddd mmmm", MONDAY, "de"),
        "Monday January"
    );
    assert_eq!(Locale::of_id(0x040C), Some(Locale::of_tag("fr")));
    assert_eq!(Locale::of_tag("de_AT").names, Locale::of_tag("de").names);
}

#[test]
fn a_locale_with_no_names_row_renders_english_names() {
    assert_eq!(
        in_locale("dddd mmmm mmm ddd", MONDAY, "en"),
        "Monday January Jan Mon"
    );
    assert_eq!(in_locale("dddd mmmm", MONDAY, "ja"), "Monday January");
    assert_eq!(in_locale("dddd mmmm", MONDAY, "xx"), "Monday January");
    assert_eq!(render("[$-411]dddd mmmm", MONDAY), "Monday January");
    assert_eq!(render("[$-FFFF]dddd", MONDAY), "Monday");
    assert_eq!(Locale::of_tag("ja"), Locale::EN);
}

#[test]
fn openformula_and_a1_text_translate_both_ways() {
    // Each pair reads both ways: OpenFormula as LibreOffice writes it, and the A1 text.
    let pairs = [
        ("of:=SUM([.A1:.B2])", "SUM(A1:B2)"),
        ("of:=[.$A$1]*2", "$A$1*2"),
        (
            "of:=[$'My sheet'.A1:.B2]+[$Shop.C3]",
            "'My sheet'!A1:B2+Shop!C3",
        ),
        ("of:=SUM([$Shop.A1:$Lists.B2])", "SUM(Shop:Lists!A1:B2)"),
        ("of:=[$'O''Brien'.A1]", "'O''Brien'!A1"),
        ("of:=['file:///a.ods'#$Shop.A1]", "'[file:///a.ods]Shop'!A1"),
        ("of:=SUM([.A:.A])+SUM([.1:.3])", "SUM(A:A)+SUM(1:3)"),
        ("of:=IF([.A1]>0;\"a;b\";TRUE())", "IF(A1>0,\"a;b\",TRUE)"),
        ("of:=SUM({1;2|3;4})", "SUM({1,2;3,4})"),
        ("of:=COM.MICROSOFT.IFS([.A1]=1;\"x\")", "IFS(A1=1,\"x\")"),
        ("of:=IFERROR(1/0;#DIV/0!)", "IFERROR(1/0,#DIV/0!)"),
        ("of:=SUM(([.A1]~[.B1]))", "SUM((A1,B1))"),
        ("of:=SUM([.A1:.B2]![.B1:.B3])", "SUM(A1:B2 B1:B3)"),
        ("of:=Total*1.5E+3+50%", "Total*1.5E+3+50%"),
        ("of:=\"a [.A1] b\"&[.A1]", "\"a [.A1] b\"&A1"),
    ];
    for (open, a1) in pairs {
        assert_eq!(openformula::to_a1(open).as_deref(), Some(a1), "{open}");
        assert_eq!(openformula::from_a1(a1), open, "{a1}");
        assert_eq!(openformula::from_a1(&format!("={a1}")), open, "={a1}");
    }
    // A reference to a deleted sheet or cell is `#REF!`, and a sheet's `$` is dropped.
    assert_eq!(
        openformula::to_a1("of:=[$#REF!.A1]+1").as_deref(),
        Some("#REF!+1")
    );
    assert_eq!(openformula::to_a1("of:=[.#REF!]").as_deref(), Some("#REF!"));
    assert_eq!(openformula::to_a1("=[Shop.A1]").as_deref(), Some("Shop!A1"));
    assert_eq!(
        openformula::to_a1("of:=[Sheet 1.A1]").as_deref(),
        Some("'Sheet 1'!A1")
    );
    // Text with no end cannot be read.
    assert_eq!(openformula::to_a1("of:=[.A1"), None);
    assert_eq!(openformula::to_a1("of:=\"open"), None);
}
