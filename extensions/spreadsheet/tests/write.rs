//! Columns, styles, the header defaults and sheets, read back from the written archive
//! (`rule:core-classes/spreadsheet-styles-and-defaults`,
//! `rule:core-classes/spreadsheet-writes-as-a-stream`).

use miniz_oxide::inflate::decompress_to_vec;
use nvs_spreadsheet::styles::{Implicit, Styles};
use nvs_spreadsheet::{
    Border, Cell, Column, Error, Header, Line, Row, SheetOptions, Style, Value, WorkbookOptions,
    Writer,
};

fn text(value: &str) -> Cell {
    Cell {
        value: Some(Value::Text(value.to_owned())),
        style: None,
    }
}

fn number(value: f64) -> Cell {
    Cell {
        value: Some(Value::Number(value)),
        style: None,
    }
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
    Writer::open(WorkbookOptions::default()).unwrap()
}

/// A finished workbook: every part by name, and the rows of each sheet.
struct Book {
    parts: Vec<(String, String)>,
    rows: Vec<(String, u64)>,
}

impl Book {
    fn part(&self, name: &str) -> &str {
        self.parts
            .iter()
            .find(|(part, _)| part == name)
            .map(|(_, text)| text.as_str())
            .unwrap_or_else(|| panic!("the archive has {name}"))
    }

    fn sheet(&self, at: usize) -> &str {
        self.part(&format!("xl/worksheets/sheet{at}.xml"))
    }

    fn styles(&self) -> &str {
        self.part("xl/styles.xml")
    }
}

/// Finishes `writer` and reads back what it wrote, `out` being every byte returned before.
fn finish(mut writer: Writer, mut out: Vec<u8>) -> Book {
    let finished = writer.finish().unwrap();
    out.extend(&finished.data);
    Book {
        parts: entries(&out),
        rows: finished
            .rows
            .into_iter()
            .map(|sheet| (sheet.name, sheet.rows))
            .collect(),
    }
}

/// The entries of the zip archive `archive`, each name with its inflated text, read through the
/// central directory.
fn entries(archive: &[u8]) -> Vec<(String, String)> {
    let u16_at = |at: usize| usize::from(u16::from_le_bytes([archive[at], archive[at + 1]]));
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
        let packed = u32_at(at + 20) as usize;
        let name_len = u16_at(at + 28);
        let local = u32_at(at + 42) as usize;
        let name = String::from_utf8(archive[at + 46..at + 46 + name_len].to_vec()).unwrap();
        let data = local + 30 + u16_at(local + 26) + u16_at(local + 28);
        let inflated = decompress_to_vec(&archive[data..data + packed]).unwrap();
        out.push((name, String::from_utf8(inflated).unwrap()));
        at += 46 + name_len + u16_at(at + 30) + u16_at(at + 32);
    }
    out
}

fn invalid(err: Error) -> String {
    match err {
        Error::Invalid(message) => message,
        other => panic!("expected an invalid-input error, got {other:?}"),
    }
}

fn no_header() -> Header {
    Header {
        default_style: false,
        style: None,
        freeze: false,
        filter: false,
    }
}

#[test]
fn columns_take_header_width_format_style_hidden_and_formula() {
    let mut writer = writer();
    let bold = writer
        .style(Style {
            bold: Some(true),
            ..Style::default()
        })
        .unwrap();
    let mut options = SheetOptions::named("Shop");
    options.columns = vec![
        Column {
            width: Some(12.0),
            ..Column::default()
        },
        Column {
            auto: true,
            style: Some(bold),
            format: Some("money".to_owned()),
            ..Column::default()
        },
        Column {
            hidden: true,
            ..Column::default()
        },
    ];
    let total = Cell {
        value: Some(Value::Formula {
            formula: "B2*2".to_owned(),
            cached_text: None,
            cached_number: Some(7.0),
        }),
        style: None,
    };
    let out = writer
        .sheet(
            options,
            &[
                header(&["Name", "Price", "Code", "Total"]),
                row(vec![text("Tea"), number(3.5), text("T1"), total]),
            ],
        )
        .unwrap();
    let book = finish(writer, out);
    let sheet = book.sheet(1);
    assert!(sheet.contains("t=\"inlineStr\"><is><t>Name</t>"), "{sheet}");
    assert!(
        sheet.contains("<col min=\"1\" max=\"1\" width=\"12\" customWidth=\"1\"/>"),
        "{sheet}"
    );
    assert!(
        sheet.contains("<col min=\"2\" max=\"2\" width=\"6\" customWidth=\"1\" style=\""),
        "{sheet}"
    );
    // A listed column has auto width only when it says so.
    assert!(
        sheet.contains("<col min=\"3\" max=\"3\" hidden=\"1\"/>"),
        "{sheet}"
    );
    // The fourth column is not listed, so a header row gives it auto width.
    assert!(
        sheet.contains("<col min=\"4\" max=\"4\" width=\"6\" customWidth=\"1\"/>"),
        "{sheet}"
    );
    assert!(sheet.contains("<f>B2*2</f><v>7</v>"), "{sheet}");
    // The price is bold from the column's style, and money from its format.
    assert!(
        book.styles()
            .contains("<xf numFmtId=\"4\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\" applyFont=\"1\"/>"),
        "{}",
        book.styles()
    );
    assert!(
        book.part("xl/workbook.xml")
            .contains("fullCalcOnLoad=\"1\"")
    );
    assert_eq!(book.rows, [("Shop".to_owned(), 1)]);
}

#[test]
fn a_list_of_plain_lists_has_no_header() {
    let mut writer = writer();
    let out = writer
        .sheet(
            SheetOptions::named("Blog"),
            &[
                row(vec![text("Hello"), number(1.0)]),
                row(vec![text("World")]),
            ],
        )
        .unwrap();
    let book = finish(writer, out);
    let sheet = book.sheet(1);
    assert!(
        sheet.contains("<row r=\"1\"><c r=\"A1\" t=\"inlineStr\">"),
        "{sheet}"
    );
    assert!(!sheet.contains("<pane"), "{sheet}");
    assert!(!sheet.contains("<autoFilter"), "{sheet}");
    assert!(!sheet.contains("<cols>"), "{sheet}");
    assert!(!book.part("xl/workbook.xml").contains("_FilterDatabase"));
    assert_eq!(book.rows, [("Blog".to_owned(), 2)]);
}

#[test]
fn each_style_field_comes_from_the_most_specific_layer() {
    let mut writer = writer();
    let column = writer
        .style(Style {
            bold: Some(true),
            color: Some("#FF0000".to_owned()),
            fill: Some("#FFFF00".to_owned()),
            ..Style::default()
        })
        .unwrap();
    let row_layer = writer
        .style(Style {
            color: Some("#0000FF".to_owned()),
            align: Some(nvs_spreadsheet::Align::Center),
            ..Style::default()
        })
        .unwrap();
    let cell = writer
        .style(Style {
            italic: Some(true),
            color: Some("#00FF00".to_owned()),
            ..Style::default()
        })
        .unwrap();
    let mut options = SheetOptions::named("Shop");
    options.columns = vec![Column {
        style: Some(column),
        ..Column::default()
    }];
    let styled = Row {
        cells: vec![Cell {
            value: Some(Value::Text("Tea".to_owned())),
            style: Some(cell),
        }],
        style: Some(row_layer),
        ..Row::default()
    };
    let out = writer.sheet(options, &[styled]).unwrap();
    let book = finish(writer, out);
    let styles = book.styles();
    // Bold and the fill come from the column, the alignment from the row, and italic and the
    // color from the cell.
    assert!(
        styles.contains("<font><b/><i/><sz val=\"11\"/><color rgb=\"FF00FF00\"/>"),
        "{styles}"
    );
    assert!(styles.contains("<fgColor rgb=\"FFFFFF00\"/>"), "{styles}");
    assert!(
        styles.contains("<alignment horizontal=\"center\"/>"),
        "{styles}"
    );
}

#[test]
fn equal_style_combinations_share_one_styles_xml_entry() {
    let mut writer = writer();
    let bold = writer
        .style(Style {
            bold: Some(true),
            ..Style::default()
        })
        .unwrap();
    assert_eq!(
        writer
            .style(Style {
                bold: Some(true),
                ..Style::default()
            })
            .unwrap(),
        bold
    );
    let by_cell = row(vec![Cell {
        value: Some(Value::Text("a".to_owned())),
        style: Some(bold),
    }]);
    let by_row = Row {
        cells: vec![text("b"), text("c")],
        style: Some(bold),
        ..Row::default()
    };
    let out = writer
        .sheet(SheetOptions::named("Shop"), &[by_cell, by_row])
        .unwrap();
    let book = finish(writer, out);
    assert!(
        book.styles().contains("<cellXfs count=\"2\">"),
        "{}",
        book.styles()
    );
    let sheet = book.sheet(1);
    for cell in ["A1", "A2", "B2"] {
        assert!(
            sheet.contains(&format!("<c r=\"{cell}\" s=\"1\"")),
            "{cell} in {sheet}"
        );
    }
}

#[test]
fn every_format_shortcut_writes_its_format_code() {
    let builtin = [
        ("integer", 1),
        ("decimal", 2),
        ("money", 4),
        ("percent", 9),
        ("text", 49),
    ];
    let custom = [
        ("date", "yyyy-mm-dd"),
        ("datetime", "yyyy-mm-dd hh:mm"),
        ("time", "hh:mm:ss"),
        ("duration", "[h]:mm:ss"),
    ];
    let shortcuts = builtin
        .iter()
        .map(|(name, _)| *name)
        .chain(custom.iter().map(|(name, _)| *name));
    for shortcut in shortcuts {
        let mut writer = writer();
        let layer = writer
            .style(Style {
                format: Some(shortcut.to_owned()),
                ..Style::default()
            })
            .unwrap();
        let cell = Cell {
            value: Some(Value::Number(1.5)),
            style: Some(layer),
        };
        let out = writer
            .sheet(SheetOptions::named("Shop"), &[row(vec![cell])])
            .unwrap();
        let book = finish(writer, out);
        let styles = book.styles();
        if let Some((_, id)) = builtin.iter().find(|(name, _)| *name == shortcut) {
            assert!(
                styles.contains(&format!("<xf numFmtId=\"{id}\" ")),
                "{shortcut}: {styles}"
            );
        } else {
            let (_, code) = custom.iter().find(|(name, _)| *name == shortcut).unwrap();
            assert!(
                styles.contains(&format!("formatCode=\"{code}\"")),
                "{shortcut}: {styles}"
            );
        }
    }
}

#[test]
fn past_the_format_limit_throws_naming_it() {
    let mut styles = Styles::default();
    // Entry 0 is the default format, so 65,489 more fill Excel's 65,490.
    for at in 0..65_489_u32 {
        let layer = styles
            .layer(Style {
                size: Some(1.0 + f64::from(at) * 0.001),
                ..Style::default()
            })
            .unwrap();
        styles.xf(None, None, Some(layer), Implicit::None).unwrap();
    }
    let last = styles
        .layer(Style {
            size: Some(300.0),
            ..Style::default()
        })
        .unwrap();
    let message = invalid(
        styles
            .xf(None, None, Some(last), Implicit::None)
            .unwrap_err(),
    );
    assert!(message.contains("65,490"), "{message}");
}

#[test]
fn a_header_row_is_bold_bordered_frozen_and_filtered_by_default() {
    let mut writer = writer();
    let out = writer
        .sheet(
            SheetOptions::named("Orders"),
            &[
                header(&["Name", "Price"]),
                row(vec![text("Tea"), number(3.5)]),
                row(vec![text("Cake"), number(2.0)]),
            ],
        )
        .unwrap();
    let book = finish(writer, out);
    let sheet = book.sheet(1);
    assert!(
        sheet.contains(
            "<pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/>"
        ),
        "{sheet}"
    );
    assert!(
        sheet.contains("<row r=\"1\" s=\"1\" customFormat=\"1\">"),
        "{sheet}"
    );
    assert!(sheet.contains("<autoFilter ref=\"A1:B3\"/>"), "{sheet}");
    let styles = book.styles();
    assert!(styles.contains("<font><b/>"), "{styles}");
    assert!(
        styles.contains("<bottom style=\"thin\"><color auto=\"1\"/></bottom>"),
        "{styles}"
    );
    assert!(
        book.part("xl/workbook.xml").contains(
            "<definedName name=\"_xlnm._FilterDatabase\" localSheetId=\"0\" hidden=\"1\">'Orders'!$A$1:$B$3</definedName>"
        ),
        "{}",
        book.part("xl/workbook.xml")
    );
    // The header row is not counted.
    assert_eq!(book.rows, [("Orders".to_owned(), 2)]);
}

#[test]
fn each_header_default_turns_off() {
    let off = [
        (
            Header {
                freeze: false,
                ..Header::default()
            },
            "<pane",
        ),
        (
            Header {
                filter: false,
                ..Header::default()
            },
            "<autoFilter",
        ),
        (
            Header {
                default_style: false,
                ..Header::default()
            },
            "<row r=\"1\" s=",
        ),
    ];
    for (header_options, absent) in off {
        let mut writer = writer();
        let mut options = SheetOptions::named("Orders");
        options.header = header_options;
        let out = writer
            .sheet(options, &[header(&["Name"]), row(vec![text("Tea")])])
            .unwrap();
        let book = finish(writer, out);
        let sheet = book.sheet(1);
        assert!(!sheet.contains(absent), "{absent} in {sheet}");
    }

    // A column's own width replaces auto width, and a header with no defaults is a plain row.
    let mut writer = writer();
    let mut options = SheetOptions::named("Orders");
    options.header = no_header();
    options.columns = vec![Column {
        width: Some(30.0),
        ..Column::default()
    }];
    let out = writer
        .sheet(options, &[header(&["Name"]), row(vec![text("Tea")])])
        .unwrap();
    let book = finish(writer, out);
    let sheet = book.sheet(1);
    assert!(
        sheet.contains("<row r=\"1\"><c r=\"A1\" t=\"inlineStr\">"),
        "{sheet}"
    );
    assert!(sheet.contains("width=\"30\""), "{sheet}");
    assert!(
        !sheet.contains("<pane") && !sheet.contains("<autoFilter"),
        "{sheet}"
    );
}

#[test]
fn auto_width_measures_the_header_and_the_first_thousand_rows() {
    let mut writer = writer();
    let mut first = vec![header(&["Name", "A long header text"])];
    for at in 0..1_000 {
        let name = if at == 500 {
            "x".repeat(20)
        } else {
            "Tea".to_owned()
        };
        first.push(row(vec![text(&name), number(1.0)]));
    }
    let mut out = writer.sheet(SheetOptions::named("Orders"), &[]).unwrap();
    out.extend(writer.rows(&first).unwrap());
    // A longer value after the first batch does not change the width.
    out.extend(
        writer
            .rows(&[row(vec![text(&"y".repeat(50)), number(1.0)])])
            .unwrap(),
    );
    let book = finish(writer, out);
    let sheet = book.sheet(1);
    assert!(
        sheet.contains("<col min=\"1\" max=\"1\" width=\"21\" customWidth=\"1\"/>"),
        "{sheet}"
    );
    assert!(
        sheet.contains("<col min=\"2\" max=\"2\" width=\"19\" customWidth=\"1\"/>"),
        "{sheet}"
    );
}

#[test]
fn auto_width_counts_a_wide_east_asian_character_as_two_and_caps_at_sixty() {
    let mut writer = writer();
    let out = writer
        .sheet(
            SheetOptions::named("Orders"),
            &[
                header(&["Ort", "Note"]),
                row(vec![text("東京都"), text(&"a".repeat(100))]),
            ],
        )
        .unwrap();
    let book = finish(writer, out);
    let sheet = book.sheet(1);
    assert!(
        sheet.contains("<col min=\"1\" max=\"1\" width=\"7\" customWidth=\"1\"/>"),
        "{sheet}"
    );
    assert!(
        sheet.contains("<col min=\"2\" max=\"2\" width=\"60\" customWidth=\"1\"/>"),
        "{sheet}"
    );
}

#[test]
fn auto_width_measures_the_text_the_cell_format_shows() {
    let mut writer = writer();
    let mut options = SheetOptions::named("Orders");
    options.columns = vec![
        Column {
            auto: true,
            format: Some("#,##0.00".to_owned()),
            ..Column::default()
        },
        Column {
            auto: true,
            format: Some("d mmmm yyyy".to_owned()),
            ..Column::default()
        },
        Column {
            auto: true,
            ..Column::default()
        },
    ];
    let day = || Cell {
        value: Some(Value::Date(nvs_spreadsheet::Date {
            year: 2026,
            month: 10,
            day: 14,
        })),
        style: None,
    };
    // The cells show `1,234,567.50`, `14 October 2026` and `2026-10-14`.
    let out = writer
        .sheet(
            options,
            &[
                header(&["Total", "Day", "Plain"]),
                row(vec![number(1_234_567.5), day(), day()]),
            ],
        )
        .unwrap();
    let book = finish(writer, out);
    let sheet = book.sheet(1);
    for (column, width) in [(1, 13), (2, 16), (3, 11)] {
        let col = format!("<col min=\"{column}\" max=\"{column}\" width=\"{width}\" ");
        assert!(sheet.contains(&col), "{col} in {sheet}");
    }
}

#[test]
fn writing_to_a_finished_sheet_throws_naming_it() {
    let mut writer = writer();
    writer
        .sheet(SheetOptions::named("A"), &[row(vec![text("a")])])
        .unwrap();
    writer
        .sheet(SheetOptions::named("B"), &[row(vec![text("b")])])
        .unwrap();
    // Rows after `sheet("B")` go to `B`, so sheet `A` keeps its one row.
    writer.rows(&[row(vec![text("c")])]).unwrap();
    writer.finish().unwrap();
    let message = invalid(writer.rows(&[row(vec![text("d")])]).unwrap_err());
    assert!(message.contains("`B` is finished"), "{message}");
}

#[test]
fn position_sets_the_tab_order() {
    let mut writer = writer();
    let mut out = writer
        .sheet(SheetOptions::named("Orders"), &[row(vec![text("a")])])
        .unwrap();
    let mut summary = SheetOptions::named("Summary");
    summary.position = Some(0);
    out.extend(writer.sheet(summary, &[row(vec![text("b")])]).unwrap());
    let book = finish(writer, out);
    let workbook = book.part("xl/workbook.xml");
    let summary = workbook.find("name=\"Summary\"").unwrap();
    let orders = workbook.find("name=\"Orders\"").unwrap();
    assert!(summary < orders, "{workbook}");
    // Each part keeps the order it was written in.
    assert!(book.sheet(1).contains(">a<") && book.sheet(2).contains(">b<"));
}

#[test]
fn a_bad_sheet_name_throws_naming_the_rule() {
    let cases = [
        ("", "cannot be empty"),
        ("A name longer than thirty-one ch", "at most 31 characters"),
        ("Q1/Q2", "`/`"),
        ("[Orders]", "`[`"),
        ("'Orders", "starts or ends with `'`"),
        ("orders", "used twice"),
    ];
    for (name, rule) in cases {
        let mut writer = writer();
        writer.sheet(SheetOptions::named("Orders"), &[]).unwrap();
        let message = invalid(writer.sheet(SheetOptions::named(name), &[]).unwrap_err());
        assert!(message.contains(rule), "`{name}`: {message}");
    }
}

/// Writes `count` empty rows to the open sheet in batches.
fn fill(writer: &mut Writer, out: &mut Vec<u8>, count: usize) {
    let batch = vec![Row::default(); 65_536];
    let mut left = count;
    while left > 0 {
        let take = left.min(batch.len());
        out.extend(writer.rows(&batch[..take]).unwrap());
        left -= take;
    }
}

#[test]
fn the_row_past_the_limit_throws_naming_it() {
    let mut writer = writer();
    let mut out = writer.sheet(SheetOptions::named("Orders"), &[]).unwrap();
    fill(&mut writer, &mut out, 1_048_576);
    let message = invalid(writer.rows(&[row(vec![text("one more")])]).unwrap_err());
    assert!(message.contains("`Orders` is full"), "{message}");
    assert!(message.contains("1,048,576"), "{message}");
    assert!(message.contains("overflow: 'newSheet'"), "{message}");
}

#[test]
fn overflow_new_sheet_continues_with_the_header_repeated() {
    let mut writer = writer();
    let mut options = SheetOptions::named("Orders");
    options.new_sheet_on_overflow = true;
    let mut out = writer.sheet(options, &[header(&["Name"])]).unwrap();
    fill(&mut writer, &mut out, 1_048_574);
    // One row fits, and the second goes on the next sheet.
    out.extend(
        writer
            .rows(&[row(vec![text("last")]), row(vec![text("next")])])
            .unwrap(),
    );
    let book = finish(writer, out);
    assert_eq!(
        book.rows,
        [
            ("Orders".to_owned(), 1_048_575),
            ("Orders (2)".to_owned(), 1)
        ]
    );
    let second = book.sheet(2);
    assert!(
        second.contains("<row r=\"1\" s=\"1\" customFormat=\"1\"><c r=\"A1\" s=\"1\" t=\"inlineStr\"><is><t>Name</t>"),
        "{second}"
    );
    assert!(
        second.contains("<c r=\"A2\" t=\"inlineStr\"><is><t>next</t>"),
        "{second}"
    );
    assert!(second.contains("state=\"frozen\""), "{second}");
    assert!(second.contains("<autoFilter ref=\"A1:A2\"/>"), "{second}");
    assert!(book.sheet(1).contains("<autoFilter ref=\"A1:A1048576\"/>"));
}

#[test]
fn a_value_past_the_cell_length_throws_naming_the_cell() {
    let mut writer = writer();
    writer.sheet(SheetOptions::named("Blog"), &[]).unwrap();
    let message = invalid(
        writer
            .rows(&[row(vec![text("ok"), text(&"a".repeat(32_768))])])
            .unwrap_err(),
    );
    assert!(message.contains("`B1`"), "{message}");
    assert!(message.contains("32,767"), "{message}");
    // The longest text a cell takes is written.
    writer
        .rows(&[row(vec![text(&"a".repeat(32_767))])])
        .unwrap();
}

#[test]
fn a_border_layer_replaces_only_the_sides_it_sets() {
    let over = Style {
        border: Some(Border {
            top: Some(Line::Thick),
            ..Border::default()
        }),
        ..Style::default()
    };
    let merged = over.over(&nvs_spreadsheet::styles::header());
    let border = merged.border.unwrap();
    assert_eq!(border.top, Some(Line::Thick));
    assert_eq!(border.bottom, Some(Line::Thin));
}

/// The message `define` throws for `reference`, or `None` when the name is defined.
fn define(name: &str, reference: &str) -> Option<String> {
    let mut writer = writer();
    writer.define(name, reference).err().map(invalid)
}

#[test]
fn define_takes_only_a_range_of_this_workbook() {
    for reference in [
        "Shop!$A$1",
        "Shop!$A$1:$B$10",
        "Shop!A1:B10",
        "'Shop orders'!$C$2:$C$9",
        "'It''s'!A1",
        "Shop!$A:$C",
        "Shop!$1:$3",
        "$A$1:$B$2",
    ] {
        assert_eq!(define("Picked", reference), None, "{reference}");
    }
    let mut writer = writer();
    writer.define("Picked", "'Shop orders'!$A$1:$B$2").unwrap();
    let out = writer
        .sheet(
            SheetOptions::named("Shop orders"),
            &[row(vec![number(1.0)])],
        )
        .unwrap();
    let book = finish(writer, out);
    assert!(
        book.part("xl/workbook.xml")
            .contains("<definedName name=\"Picked\">'Shop orders'!$A$1:$B$2</definedName>")
    );
}

#[test]
fn define_throws_for_a_dde_or_external_reference() {
    for reference in [
        "",
        "cmd|'/c calc'!A0",
        "[other.xlsx]Shop!B2:B3",
        "'[other.xlsx]Shop'!B2:B3",
        "https://example.com/book.xlsx!A1",
        "SUM(Shop!A1:A3)",
        "Shop!R1C1",
        "Shop!Shop!A1",
        "'Shop!A1",
        "Shop!A0",
        "1+1",
    ] {
        let message =
            define("Picked", reference).unwrap_or_else(|| panic!("`{reference}` is defined"));
        assert!(
            message.contains("not a range of this workbook") || message.contains("The sheet name"),
            "`{reference}`: {message}"
        );
    }
}

#[test]
fn a_defined_name_past_255_characters_or_in_r1c1_form_throws() {
    let long = "a".repeat(256);
    assert!(
        define(&long, "Shop!A1")
            .unwrap()
            .contains("at most 255 characters")
    );
    assert_eq!(define(&"a".repeat(255), "Shop!A1"), None);
    for name in ["R1C1", "r2c3", "R", "C", "RC", "rc2", "R7"] {
        let message = define(name, "Shop!A1").unwrap_or_else(|| panic!("`{name}` is defined"));
        assert!(message.contains("cell reference"), "`{name}`: {message}");
    }
    assert_eq!(define("Rate", "Shop!A1"), None);
}

#[test]
fn a_defined_names_sheet_that_never_comes_throws_at_finish() {
    let mut writer = writer();
    writer.define("Picked", "Shops!$A$1").unwrap();
    writer
        .sheet(SheetOptions::named("Shop"), &[row(vec![number(1.0)])])
        .unwrap();
    let message = invalid(writer.finish().unwrap_err());
    assert!(
        message.contains("`Picked`") && message.contains("`Shops`"),
        "{message}"
    );

    let mut writer = self::writer();
    writer.define("Picked", "shop!$A$1").unwrap();
    writer
        .sheet(SheetOptions::named("Shop"), &[row(vec![number(1.0)])])
        .unwrap();
    assert!(writer.finish().is_ok());
}

#[test]
fn a_sheet_name_with_a_control_character_or_named_history_throws() {
    let cases = [
        ("Shop\0", "control character"),
        ("Shop\u{1}", "control character"),
        ("Line\nbreak", "control character"),
        ("History", "`History`"),
        ("history", "`History`"),
    ];
    for (name, rule) in cases {
        let mut writer = writer();
        let message = invalid(writer.sheet(SheetOptions::named(name), &[]).unwrap_err());
        assert!(message.contains(rule), "`{name:?}`: {message}");
    }
    let mut writer = writer();
    assert!(writer.sheet(SheetOptions::named("History 2"), &[]).is_ok());
    let message = define("Picked", "'Shop\u{7}'!A1").unwrap();
    assert!(message.contains("control character"), "{message}");
}

#[test]
fn a_formula_that_is_empty_or_past_8192_characters_throws() {
    let formula = |text: String| Cell {
        value: Some(Value::Formula {
            formula: text,
            cached_text: None,
            cached_number: None,
        }),
        style: None,
    };
    let write = |text: String| {
        let mut writer = writer();
        writer
            .sheet(SheetOptions::named("Shop"), &[row(vec![formula(text)])])
            .err()
            .map(invalid)
    };
    let message = write(String::new()).unwrap();
    assert!(
        message.contains("`A1`") && message.contains("empty"),
        "{message}"
    );
    assert!(write("=".to_owned()).unwrap().contains("empty"));
    let long = format!("{}1", "1+".repeat(4_096));
    let message = write(long).unwrap();
    assert!(
        message.contains("8193 characters") && message.contains("8,192"),
        "{message}"
    );
    assert_eq!(write(format!("{}1", "1+".repeat(4_095))), None);
    assert_eq!(write(format!("={}1", "1+".repeat(4_095))), None);
}

#[test]
fn a_header_with_autofilter_writes_the_filter_name() {
    let mut writer = writer();
    // The sheet starts with no rows, and its header row comes in the first batch, as
    // `Workbook::sheet` sends them.
    let mut out = writer.sheet(SheetOptions::named("Orders"), &[]).unwrap();
    out.extend(
        writer
            .rows(&[
                header(&["Name", "Price"]),
                row(vec![text("Tea"), number(3.5)]),
            ])
            .unwrap(),
    );
    out.extend(
        writer
            .rows(&[row(vec![text("Cake"), number(2.0)])])
            .unwrap(),
    );
    out.extend(writer.sheet(SheetOptions::named("Empty"), &[]).unwrap());
    out.extend(writer.rows(&[header(&["Name"])]).unwrap());
    let book = finish(writer, out);
    assert!(
        book.sheet(1).contains("<autoFilter ref=\"A1:B3\"/>"),
        "{}",
        book.sheet(1)
    );
    let workbook = book.part("xl/workbook.xml");
    for name in [
        "<definedName name=\"_xlnm._FilterDatabase\" localSheetId=\"0\" hidden=\"1\">'Orders'!$A$1:$B$3</definedName>",
        "<definedName name=\"_xlnm._FilterDatabase\" localSheetId=\"1\" hidden=\"1\">'Empty'!$A$1:$A$1</definedName>",
    ] {
        assert!(workbook.contains(name), "{name}\nin {workbook}");
    }
}

#[test]
fn a_column_past_32_bits_is_named_in_full() {
    let mut writer = writer();
    let out = writer
        .sheet(
            SheetOptions::named("Orders"),
            &[header(&["Name"]), row(vec![text("Tea")])],
        )
        .unwrap();
    let past = 1u64 << 32;
    for message in [
        invalid(writer.cell(past, 1).unwrap_err()),
        invalid(writer.column(past).unwrap_err()),
    ] {
        assert!(
            message.contains("Column 4294967297 is outside the sheet"),
            "{message}"
        );
    }
    let message = invalid(writer.cell(0, 1u64 << 32).unwrap_err());
    assert!(message.contains("Row 4294967296"), "{message}");
    assert_eq!(writer.cell(16_383, 1_048_576).unwrap(), "XFD1048576");
    assert_eq!(writer.column(0).unwrap(), "A2:A2");
    finish(writer, out);
}
