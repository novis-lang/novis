//! The parts a sheet writes after its rows, formulas and rich text, read back from the written
//! archive (`rule:core-classes/spreadsheet-sheet-end-parts`,
//! `rule:core-classes/spreadsheet-formula-is-a-value`).

use miniz_oxide::inflate::decompress_to_vec;
use nvs_spreadsheet::conditional::{Conditional, Kind};
use nvs_spreadsheet::drawing::Image;
use nvs_spreadsheet::table::{Function, Table, Total};
use nvs_spreadsheet::tail::{
    Group, Link, Margins, Note, Operator, Orientation, Part, Print, Protect, RuleKind, Validation,
};
use nvs_spreadsheet::{
    Cell, Clock, Column, Date, Error, Properties, RichRun, Row, SheetOptions, Style, TimeOfDay,
    Value, Visibility, WorkbookOptions, Writer,
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

/// `count` rows of a name and a quantity.
fn orders(count: usize) -> Vec<Row> {
    (0..count)
        .map(|at| row(vec![text(&format!("Order {at}")), number(at as f64)]))
        .collect()
}

fn writer() -> Writer {
    Writer::open(WorkbookOptions::default()).unwrap()
}

/// A finished workbook: every part by name.
struct Book {
    parts: Vec<(String, Vec<u8>)>,
}

impl Book {
    fn bytes(&self, name: &str) -> &[u8] {
        self.parts
            .iter()
            .find(|(part, _)| part == name)
            .map(|(_, bytes)| bytes.as_slice())
            .unwrap_or_else(|| panic!("the archive has {name}"))
    }

    fn part(&self, name: &str) -> &str {
        std::str::from_utf8(self.bytes(name)).unwrap()
    }

    fn has(&self, name: &str) -> bool {
        self.parts.iter().any(|(part, _)| part == name)
    }

    fn sheet(&self, at: usize) -> &str {
        self.part(&format!("xl/worksheets/sheet{at}.xml"))
    }

    fn workbook(&self) -> &str {
        self.part("xl/workbook.xml")
    }
}

/// Finishes `writer` and reads back what it wrote, `out` being every byte returned before.
fn finish(mut writer: Writer, mut out: Vec<u8>) -> Book {
    out.extend(writer.finish().unwrap().data);
    Book {
        parts: entries(&out),
    }
}

/// The entries of the zip archive `archive`, each name with its inflated text, read through the
/// central directory.
fn entries(archive: &[u8]) -> Vec<(String, Vec<u8>)> {
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
        out.push((
            name,
            decompress_to_vec(&archive[data..data + packed]).unwrap(),
        ));
        at += 46 + name_len + u16_at(at + 30) + u16_at(at + 32);
    }
    out
}

/// The text of `xml` from `from` up to the end of the first `to` after it.
fn between<'a>(xml: &'a str, from: &str, to: &str) -> &'a str {
    let start = xml
        .find(from)
        .unwrap_or_else(|| panic!("{from} is in {xml}"));
    let end = start + xml[start..].find(to).expect("the end tag") + to.len();
    &xml[start..end]
}

fn invalid(result: Result<Vec<u8>, Error>) -> String {
    match result {
        Err(Error::Invalid(message)) => message,
        other => panic!("an invalid part, not {other:?}"),
    }
}

#[test]
fn a_conditional_formula_past_8192_characters_throws() {
    let red = Style {
        fill: Some("#FFC7CE".to_owned()),
        ..Style::default()
    };
    let rule = |kind: Kind, values: Vec<String>| {
        let mut rule = conditional("B1:B9", kind);
        rule.style = Some(red.clone());
        rule.operator = (kind == Kind::Cell).then_some(Operator::Greater);
        rule.values = values;
        rule
    };
    let mut writer = writer();
    writer
        .sheet(SheetOptions::named("Shop"), &orders(9))
        .unwrap();
    let long = format!("={}1", "1+".repeat(4_096));
    for kind in [Kind::Formula, Kind::Cell] {
        let message = invalid(writer.part(Part::Conditional(rule(kind, vec![long.clone()]))));
        assert!(
            message.contains("`B1:B9`") && message.contains("at most 8,192"),
            "{message}"
        );
        let message = invalid(writer.part(Part::Conditional(rule(kind, vec!["=".to_owned()]))));
        assert!(message.contains("empty"), "{message}");
    }
    let fits = format!("={}1", "1+".repeat(4_095));
    assert!(
        writer
            .part(Part::Conditional(rule(Kind::Formula, vec![fits])))
            .is_ok()
    );
}

fn validation(range: &str, kind: RuleKind, values: &[&str]) -> Validation {
    Validation {
        validated: range.to_owned(),
        kind,
        operator: None,
        values: values.iter().map(|&value| value.to_owned()).collect(),
        input_title: None,
        input_message: None,
        error_title: None,
        error_message: None,
        allow_empty: false,
    }
}

#[test]
fn merges_links_and_notes_point_at_earlier_rows() {
    let mut writer = writer();
    let mut out = writer
        .sheet(SheetOptions::named("Shop"), &orders(3))
        .unwrap();
    out.extend(writer.rows(&orders(500)).unwrap());
    for part in [
        Part::Merge("A1:B1".to_owned()),
        Part::Merge("$A$2:$A$4".to_owned()),
        Part::Link(Link {
            anchor: "A2".to_owned(),
            target: "https://example.com/orders?id=1&page=2".to_owned(),
            text: Some("Order 1".to_owned()),
            tooltip: Some("Open the order".to_owned()),
        }),
        Part::Link(Link {
            anchor: "A5".to_owned(),
            target: "mailto:shop@example.com".to_owned(),
            text: None,
            tooltip: None,
        }),
        Part::Link(Link {
            anchor: "A6".to_owned(),
            target: "'Shop totals'!B2".to_owned(),
            text: None,
            tooltip: None,
        }),
        Part::Note(Note {
            noted: "B3".to_owned(),
            note: "Checked by hand".to_owned(),
            author: Some("Blog".to_owned()),
        }),
        Part::Note(Note {
            noted: "B7".to_owned(),
            note: "A <second> note".to_owned(),
            author: None,
        }),
    ] {
        assert!(writer.part(part).unwrap().is_empty());
    }

    // A merge that overlaps one already added, a second link on a cell, and a link to another
    // scheme are errors, and the sheet goes on.
    assert!(
        invalid(writer.part(Part::Merge("B1:C2".to_owned())))
            .contains("overlaps another merged range")
    );
    assert!(invalid(writer.part(Part::Merge("A9".to_owned()))).contains("is one cell"));
    let link = |target: &str| {
        Part::Link(Link {
            anchor: "C9".to_owned(),
            target: target.to_owned(),
            text: None,
            tooltip: None,
        })
    };
    for target in ["file:///etc/passwd", "javascript:alert(1)", "example.com"] {
        assert!(
            invalid(writer.part(link(target))).contains("is not allowed"),
            "{target}"
        );
    }
    assert!(
        invalid(writer.part(Part::Link(Link {
            anchor: "A2".to_owned(),
            target: "https://example.com".to_owned(),
            text: None,
            tooltip: None,
        })))
        .contains("has a link already")
    );
    let book = finish(writer, out);

    let sheet = book.sheet(1);
    let tail = &sheet[sheet.find("</sheetData>").unwrap()..];
    assert_eq!(
        between(tail, "<mergeCells", "</mergeCells>"),
        "<mergeCells count=\"2\"><mergeCell ref=\"A1:B1\"/><mergeCell ref=\"A2:A4\"/></mergeCells>"
    );
    assert_eq!(
        between(tail, "<hyperlinks>", "</hyperlinks>"),
        "<hyperlinks><hyperlink ref=\"A2\" r:id=\"rId1\" tooltip=\"Open the order\" display=\"Order 1\"/>\
         <hyperlink ref=\"A5\" r:id=\"rId2\"/>\
         <hyperlink ref=\"A6\" location=\"'Shop totals'!B2\"/></hyperlinks>"
    );
    assert!(tail.ends_with("<legacyDrawing r:id=\"rId4\"/></worksheet>"));
    assert!(tail.find("<mergeCells").unwrap() < tail.find("<hyperlinks>").unwrap());

    let rels = book.part("xl/worksheets/_rels/sheet1.xml.rels");
    assert!(rels.contains(
        "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" \
         Target=\"https://example.com/orders?id=1&amp;page=2\" TargetMode=\"External\"/>"
    ));
    assert!(rels.contains("Id=\"rId2\"") && rels.contains("Target=\"mailto:shop@example.com\""));
    assert!(rels.contains("Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments\" Target=\"../comments1.xml\""));
    assert!(rels.contains("Id=\"rId4\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing\" Target=\"../drawings/vmlDrawing1.vml\""));

    let comments = book.part("xl/comments1.xml");
    assert!(comments.contains("<authors><author>Blog</author><author></author></authors>"));
    assert!(comments.contains(
        "<comment ref=\"B3\" authorId=\"0\"><text><r><t>Checked by hand</t></r></text></comment>"
    ));
    assert!(comments.contains(
        "<comment ref=\"B7\" authorId=\"1\"><text><r><t>A &lt;second&gt; note</t></r></text></comment>"
    ));
    let vml = book.part("xl/drawings/vmlDrawing1.vml");
    assert_eq!(vml.matches("<x:ClientData ObjectType=\"Note\">").count(), 2);
    assert!(vml.contains("<x:Row>2</x:Row><x:Column>1</x:Column>"));
    assert!(vml.contains("<x:Row>6</x:Row><x:Column>1</x:Column>"));

    let types = book.part("[Content_Types].xml");
    assert!(types.contains("<Default Extension=\"vml\""));
    assert!(types.contains("<Override PartName=\"/xl/comments1.xml\""));
}

#[test]
fn data_validation_writes_every_rule_kind_and_its_messages() {
    let mut writer = writer();
    let out = writer
        .sheet(SheetOptions::named("Shop"), &orders(9))
        .unwrap();
    let mut list = validation("A2:A9", RuleKind::List, &["new", "paid", "sent"]);
    list.input_title = Some("Status".to_owned());
    list.input_message = Some("Pick a status".to_owned());
    list.error_title = Some("Wrong status".to_owned());
    list.error_message = Some("Use a status from the list".to_owned());
    list.allow_empty = true;
    let mut whole = validation("B2:B9", RuleKind::Whole, &["1", "100"]);
    whole.operator = Some(Operator::NotBetween);
    let mut decimal = validation("C2:C9", RuleKind::Decimal, &["0.5"]);
    decimal.operator = Some(Operator::GreaterOrEqual);
    let date = validation("D2:D9", RuleKind::Date, &["2024-01-01", "2024-12-31"]);
    let mut length = validation("E2:E9", RuleKind::Length, &["20"]);
    length.operator = Some(Operator::LessOrEqual);
    for part in [
        list,
        validation("F2:F9", RuleKind::List, &["=$H$1:$H$4"]),
        whole,
        decimal,
        date,
        length,
        validation("G2:G9", RuleKind::Formula, &["=ISNUMBER(G2)"]),
    ] {
        writer.part(Part::Validation(part)).unwrap();
    }

    let message = invalid(writer.part(Part::Validation(validation(
        "A2:A9",
        RuleKind::List,
        &["a,b"],
    ))));
    assert!(message.contains("has a `,`"), "{message}");
    let message = invalid(writer.part(Part::Validation(validation(
        "B2:B9",
        RuleKind::Whole,
        &["1"],
    ))));
    assert!(message.contains("needs 2 value(s) and has 1"), "{message}");
    let long = "x".repeat(200);
    let message = invalid(writer.part(Part::Validation(validation(
        "A2:A9",
        RuleKind::List,
        &[&long, &long],
    ))));
    assert!(message.contains("at most 255"), "{message}");
    let book = finish(writer, out);

    let rules = between(book.sheet(1), "<dataValidations", "</dataValidations>");
    assert!(rules.starts_with("<dataValidations count=\"7\">"));
    for expected in [
        "<dataValidation type=\"list\" allowBlank=\"1\" showInputMessage=\"1\" showErrorMessage=\"1\" \
         errorTitle=\"Wrong status\" error=\"Use a status from the list\" promptTitle=\"Status\" \
         prompt=\"Pick a status\" sqref=\"A2:A9\"><formula1>\"new,paid,sent\"</formula1></dataValidation>",
        "<dataValidation type=\"list\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"F2:F9\"><formula1>$H$1:$H$4</formula1></dataValidation>",
        "<dataValidation type=\"whole\" operator=\"notBetween\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"B2:B9\"><formula1>1</formula1><formula2>100</formula2></dataValidation>",
        "<dataValidation type=\"decimal\" operator=\"greaterThanOrEqual\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"C2:C9\"><formula1>0.5</formula1></dataValidation>",
        "<dataValidation type=\"date\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"D2:D9\"><formula1>45292</formula1><formula2>45657</formula2></dataValidation>",
        "<dataValidation type=\"textLength\" operator=\"lessThanOrEqual\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"E2:E9\"><formula1>20</formula1></dataValidation>",
        "<dataValidation type=\"custom\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"G2:G9\"><formula1>ISNUMBER(G2)</formula1></dataValidation>",
    ] {
        assert!(rules.contains(expected), "{expected}\nin {rules}");
    }
}

#[test]
fn hidden_rows_columns_and_sheets_and_outline_groups_are_written() {
    let mut writer = writer();
    let options = SheetOptions {
        columns: vec![
            Column::default(),
            Column {
                hidden: true,
                ..Column::default()
            },
        ],
        ..SheetOptions::named("Shop")
    };
    let mut out = writer.sheet(options, &[]).unwrap();
    for grouped in ["2:5", "3:4", "C:D"] {
        writer
            .part(Part::Group(Group {
                grouped: grouped.to_owned(),
                collapsed: grouped == "3:4",
            }))
            .unwrap();
    }
    let mut rows = orders(7);
    rows[5].hidden = true;
    out.extend(writer.rows(&rows).unwrap());

    // A group over a written row, a column group after the first row, and a group that is not
    // one are errors.
    let group = |grouped: &str| {
        Part::Group(Group {
            grouped: grouped.to_owned(),
            collapsed: false,
        })
    };
    assert!(invalid(writer.part(group("6:9"))).contains("already written"));
    assert!(invalid(writer.part(group("E:F"))).contains("after its first row"));
    assert!(invalid(writer.part(group("A1:B2"))).contains("not a range of rows or columns"));
    writer.part(group("20:30")).unwrap();

    out.extend(
        writer
            .sheet(
                SheetOptions {
                    visibility: Visibility::Hidden,
                    ..SheetOptions::named("Shop data")
                },
                &orders(1),
            )
            .unwrap(),
    );
    let book = finish(writer, out);

    let sheet = book.sheet(1);
    assert!(sheet.contains(
        "<sheetFormatPr defaultRowHeight=\"15\" outlineLevelRow=\"2\" outlineLevelCol=\"1\"/>"
    ));
    assert!(sheet.contains(
        "<cols><col min=\"2\" max=\"2\" hidden=\"1\"/><col min=\"3\" max=\"3\" outlineLevel=\"1\"/><col min=\"4\" max=\"4\" outlineLevel=\"1\"/></cols>"
    ));
    assert!(sheet.contains("<row r=\"1\">"));
    assert!(sheet.contains("<row r=\"2\" outlineLevel=\"1\">"));
    assert!(sheet.contains("<row r=\"3\" hidden=\"1\" outlineLevel=\"2\">"));
    assert!(sheet.contains("<row r=\"4\" hidden=\"1\" outlineLevel=\"2\">"));
    assert!(sheet.contains("<row r=\"5\" outlineLevel=\"1\" collapsed=\"1\">"));
    assert!(sheet.contains("<row r=\"6\" hidden=\"1\">"));
    assert!(sheet.contains("<row r=\"7\">"));
    assert!(
        book.workbook()
            .contains("<sheet name=\"Shop data\" sheetId=\"2\" state=\"hidden\" r:id=\"rId2\"/>")
    );
}

#[test]
fn page_setup_print_titles_and_print_area_are_written() {
    let mut writer = writer();
    let mut out = writer
        .sheet(SheetOptions::named("Shop orders"), &[])
        .unwrap();
    writer
        .part(Part::Print(Print {
            orientation: Some(Orientation::Landscape),
            fit_width: Some(1),
            fit_height: None,
            margins: Some(Margins {
                top: 1.0,
                right: 0.5,
                bottom: 1.0,
                left: 0.5,
            }),
            header: Some("&CShop orders".to_owned()),
            footer: Some("&RPage &P of &N".to_owned()),
            repeat_rows: Some("1:2".to_owned()),
            area: Some("A1:B9".to_owned()),
        }))
        .unwrap();
    out.extend(writer.rows(&orders(9)).unwrap());
    let fit = Part::Print(Print {
        fit_height: Some(2),
        ..Print::default()
    });
    assert!(invalid(writer.part(fit)).contains("after its first row"));
    let bad = Part::Print(Print {
        repeat_rows: Some("A:B".to_owned()),
        ..Print::default()
    });
    assert!(invalid(writer.part(bad)).contains("not a range of rows"));
    let book = finish(writer, out);

    let sheet = book.sheet(1);
    assert!(sheet.contains("<sheetPr><pageSetUpPr fitToPage=\"1\"/></sheetPr>"));
    assert!(sheet.ends_with(
        "</sheetData><pageMargins left=\"0.5\" right=\"0.5\" top=\"1\" bottom=\"1\" header=\"0.3\" footer=\"0.3\"/>\
         <pageSetup orientation=\"landscape\" fitToWidth=\"1\" fitToHeight=\"0\"/>\
         <headerFooter><oddHeader>&amp;CShop orders</oddHeader><oddFooter>&amp;RPage &amp;P of &amp;N</oddFooter></headerFooter></worksheet>"
    ));
    let names = between(book.workbook(), "<definedNames>", "</definedNames>");
    assert_eq!(
        names,
        "<definedNames><definedName name=\"_xlnm.Print_Area\" localSheetId=\"0\">'Shop orders'!$A$1:$B$9</definedName>\
         <definedName name=\"_xlnm.Print_Titles\" localSheetId=\"0\">'Shop orders'!$1:$2</definedName></definedNames>"
    );
}

#[test]
fn protection_defined_names_and_document_properties_are_written() {
    let mut writer = Writer::open(WorkbookOptions {
        properties: Properties {
            title: Some("Orders".to_owned()),
            subject: Some("Shop orders".to_owned()),
            author: Some("Shop".to_owned()),
            company: Some("Example & Co".to_owned()),
            keywords: Some("orders, shop".to_owned()),
            created: Some(Clock {
                day: Date {
                    year: 2024,
                    month: 3,
                    day: 1,
                },
                clock: TimeOfDay {
                    hour: 9,
                    minute: 30,
                    second: 0,
                    nanos: 0,
                },
            }),
        },
        target: "orders.xlsx".to_owned(),
        ods: false,
    })
    .unwrap();
    let out = writer
        .sheet(SheetOptions::named("Shop"), &orders(3))
        .unwrap();
    writer
        .part(Part::Protect(Protect {
            password: Some("password".to_owned()),
            allow: vec![
                "formatColumns".to_owned(),
                "sort".to_owned(),
                "objects".to_owned(),
            ],
        }))
        .unwrap();
    let message = invalid(writer.part(Part::Protect(Protect {
        password: None,
        allow: vec!["everything".to_owned()],
    })));
    assert!(
        message.contains("`everything` is not something"),
        "{message}"
    );
    writer.define("Quantities", "Shop!$B$1:$B$3").unwrap();
    let book = finish(writer, out);

    let sheet = book.sheet(1);
    assert!(sheet.contains(
        "</sheetData><sheetProtection password=\"83AF\" sheet=\"1\" scenarios=\"1\" formatColumns=\"0\" sort=\"0\"/></worksheet>"
    ));
    assert!(book.workbook().contains(
        "<definedNames><definedName name=\"Quantities\">Shop!$B$1:$B$3</definedName></definedNames>"
    ));
    let core = book.part("docProps/core.xml");
    for expected in [
        "<dc:title>Orders</dc:title>",
        "<dc:subject>Shop orders</dc:subject>",
        "<dc:creator>Shop</dc:creator>",
        "<cp:keywords>orders, shop</cp:keywords>",
        "<dcterms:created xsi:type=\"dcterms:W3CDTF\">2024-03-01T09:30:00Z</dcterms:created>",
    ] {
        assert!(core.contains(expected), "{expected}");
    }
    assert!(
        book.part("docProps/app.xml")
            .contains("<Company>Example &amp; Co</Company>")
    );
    assert!(!book.has("xl/worksheets/_rels/sheet1.xml.rels"));
}

#[test]
fn rich_text_in_one_cell_is_written() {
    let mut writer = writer();
    let bold = writer
        .style(Style {
            bold: Some(true),
            ..Style::default()
        })
        .unwrap();
    let red = writer
        .style(Style {
            color: Some("#FF0000".to_owned()),
            italic: Some(true),
            ..Style::default()
        })
        .unwrap();
    let rich = Cell {
        value: Some(Value::Rich(vec![
            RichRun {
                text: "Total: ".to_owned(),
                style: Some(bold),
            },
            RichRun {
                text: "late".to_owned(),
                style: Some(red),
            },
            RichRun {
                text: " & paid".to_owned(),
                style: None,
            },
        ])),
        style: None,
    };
    let out = writer
        .sheet(SheetOptions::named("Shop"), &[row(vec![rich])])
        .unwrap();
    let book = finish(writer, out);

    let cell = between(book.sheet(1), "<c r=\"A1\"", "</c>");
    assert_eq!(
        cell,
        "<c r=\"A1\" t=\"inlineStr\"><is>\
         <r><rPr><b/></rPr><t xml:space=\"preserve\">Total: </t></r>\
         <r><rPr><i/><color rgb=\"FFFF0000\"/></rPr><t>late</t></r>\
         <r><t xml:space=\"preserve\"> &amp; paid</t></r></is></c>"
    );
}

#[test]
fn a_formula_carries_its_cached_value_and_the_workbook_recalculates_on_open() {
    let formula = |text: &str, cached_text: Option<&str>, cached_number: Option<f64>| Cell {
        value: Some(Value::Formula {
            formula: text.to_owned(),
            cached_text: cached_text.map(str::to_owned),
            cached_number,
        }),
        style: None,
    };
    // Without a formula, a workbook is not flagged.
    let mut plain = writer();
    let out = plain
        .sheet(SheetOptions::named("Shop"), &orders(2))
        .unwrap();
    assert!(!finish(plain, out).workbook().contains("<calcPr"));

    let mut writer = writer();
    let mut out = writer
        .sheet(SheetOptions::named("Shop"), &orders(3))
        .unwrap();
    out.extend(
        writer
            .rows(&[row(vec![
                text("=SUM(B1:B3)"),
                formula("SUM(B1:B3)", None, Some(3.0)),
                formula("IF(B4>2,\"many\",\"few\")", Some("many"), None),
                formula("NOW()", None, None),
            ])])
            .unwrap(),
    );
    let book = finish(writer, out);

    let sheet = book.sheet(1);
    let row = between(sheet, "<row r=\"4\">", "</row>");
    assert_eq!(
        row,
        "<row r=\"4\"><c r=\"A4\" t=\"inlineStr\"><is><t>=SUM(B1:B3)</t></is></c>\
         <c r=\"B4\"><f>SUM(B1:B3)</f><v>3</v></c>\
         <c r=\"C4\" t=\"str\"><f>IF(B4&gt;2,\"many\",\"few\")</f><v>many</v></c>\
         <c r=\"D4\"><f>NOW()</f></c></row>"
    );
    assert!(book.workbook().contains("<calcPr fullCalcOnLoad=\"1\"/>"));
}

#[test]
fn cell_and_column_give_a1_text_over_the_rows_written_so_far() {
    let mut writer = writer();
    assert!(writer.column(0).is_err(), "no sheet is open");
    let mut options = SheetOptions::named("Shop");
    options.header.filter = false;
    let header = Row {
        cells: vec![text("Item"), text("Qty")],
        header: true,
        ..Row::default()
    };
    let mut out = writer.sheet(options, &[header]).unwrap();
    assert_eq!(writer.cell(2, 7).unwrap(), "C7");
    assert_eq!(writer.cell(16_383, 1_048_576).unwrap(), "XFD1048576");
    assert!(writer.cell(16_384, 1).is_err());
    assert!(writer.cell(0, 0).is_err());
    // Before the first data row, a column has no cells.
    let Err(Error::Invalid(early)) = writer.column(1) else {
        panic!("a column before the first data row is an error");
    };
    assert!(early.contains("`Shop` has no data row yet"), "{early}");
    out.extend(writer.rows(&orders(1042)).unwrap());
    assert_eq!(writer.column(1).unwrap(), "B2:B1043");
    // A totals row written after the data covers every row of the stream.
    let total = format!("SUM({})", writer.column(1).unwrap());
    out.extend(
        writer
            .rows(&[row(vec![
                text("Total"),
                Cell {
                    value: Some(Value::Formula {
                        formula: total,
                        cached_text: None,
                        cached_number: None,
                    }),
                    style: None,
                },
            ])])
            .unwrap(),
    );
    let book = finish(writer, out);
    assert!(
        book.sheet(1)
            .contains("<c r=\"B1044\"><f>SUM(B2:B1043)</f></c>"),
        "the totals row sums every data row"
    );

    // A sheet with no header row starts its data at row 1.
    let mut plain = self::writer();
    plain
        .sheet(SheetOptions::named("Blog"), &orders(3))
        .unwrap();
    assert_eq!(plain.column(0).unwrap(), "A1:A3");
}

fn formula(text: &str) -> Cell {
    Cell {
        value: Some(Value::Formula {
            formula: text.to_owned(),
            cached_text: None,
            cached_number: None,
        }),
        style: None,
    }
}

fn header(names: &[&str]) -> Row {
    Row {
        cells: names.iter().map(|name| text(name)).collect(),
        header: true,
        ..Row::default()
    }
}

fn table(range: &str, totals: Vec<Total>) -> Table {
    Table {
        range: range.to_owned(),
        name: None,
        style: None,
        banded: true,
        filter: true,
        totals,
    }
}

fn total(column: &str, function: Option<Function>, formula: Option<&str>) -> Total {
    Total {
        column: column.to_owned(),
        function,
        formula: formula.map(str::to_owned),
    }
}

#[test]
fn a_table_with_a_totals_row_and_structured_references_is_written() {
    // Added before its rows: the header row names the columns, the rows' formulas are qualified,
    // and the totals row goes right after the last row, inside the batch.
    let mut writer = writer();
    let mut out = writer.sheet(SheetOptions::named("Shop"), &[]).unwrap();
    let mut shop = table(
        "A1:C4",
        vec![
            total("qty", Some(Function::Sum), None),
            total("total", None, Some("=SUM([total])/2")),
        ],
    );
    shop.name = Some("Orders".to_owned());
    out.extend(writer.part(Part::Table(shop)).unwrap());
    let line = |item: &str, qty: f64| row(vec![text(item), number(qty), formula("[@qty]*2")]);
    out.extend(
        writer
            .rows(&[
                header(&["item", "qty", "total"]),
                line("Pen", 1.0),
                line("Ink", 2.0),
                line("Pad", 3.0),
                row(vec![text("After")]),
            ])
            .unwrap(),
    );
    let message = invalid(writer.part(Part::Table(table("C3:D9", Vec::new()))));
    assert!(message.contains("overlaps the table `Orders`"), "{message}");
    match writer.define("orders", "Shop!$A$1") {
        Err(Error::Invalid(message)) => assert!(message.contains("table's name"), "{message}"),
        other => panic!("a name equal to a table's, not {other:?}"),
    }

    // Added after its rows: it starts at the header row, and its totals row is written at once.
    let mut options = SheetOptions::named("Blog");
    options.header.filter = true;
    out.extend(
        writer
            .sheet(
                options,
                &[
                    header(&["post", "views"]),
                    row(vec![text("Hello"), number(10.0)]),
                    row(vec![text("Again"), number(20.0)]),
                ],
            )
            .unwrap(),
    );
    out.extend(
        writer
            .part(Part::Note(Note {
                noted: "A1".to_owned(),
                note: "Every post".to_owned(),
                author: None,
            }))
            .unwrap(),
    );
    let message = invalid(writer.part(Part::Table(table(
        "A1:B2",
        vec![total("views", Some(Function::Max), None)],
    ))));
    assert!(message.contains("row 3 is written already"), "{message}");
    let mut posts = table("A1:B3", vec![total("views", Some(Function::Average), None)]);
    posts.style = Some("tablestylelight1".to_owned());
    posts.banded = false;
    posts.filter = false;
    out.extend(writer.part(Part::Table(posts)).unwrap());
    let book = finish(writer, out);

    let shop = book.sheet(1);
    assert!(shop.contains("<c r=\"C2\"><f>Orders[[#This Row],[qty]]*2</f></c>"));
    assert_eq!(
        between(shop, "<row r=\"5\"", "</row>"),
        "<row r=\"5\"><c r=\"A5\" t=\"inlineStr\"><is><t>Total</t></is></c>\
         <c r=\"B5\"><f>SUBTOTAL(109,Orders[qty])</f></c>\
         <c r=\"C5\"><f>SUM(Orders[total])/2</f></c></row>"
    );
    assert!(shop.contains("<row r=\"6\"><c r=\"A6\" t=\"inlineStr\"><is><t>After</t>"));
    assert!(
        !shop.contains("<autoFilter"),
        "the default autofilter overlaps the table, so only the table filters"
    );
    assert!(
        shop.ends_with(
            "<tableParts count=\"1\"><tablePart r:id=\"rId1\"/></tableParts></worksheet>"
        )
    );
    assert_eq!(
        book.part("xl/tables/table1.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <table xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" id=\"1\" name=\"Orders\" displayName=\"Orders\" ref=\"A1:C5\" totalsRowCount=\"1\">\
         <autoFilter ref=\"A1:C4\"/><tableColumns count=\"3\">\
         <tableColumn id=\"1\" name=\"item\" totalsRowLabel=\"Total\"/>\
         <tableColumn id=\"2\" name=\"qty\" totalsRowFunction=\"sum\"/>\
         <tableColumn id=\"3\" name=\"total\" totalsRowFunction=\"custom\"><totalsRowFormula>SUM([total])/2</totalsRowFormula></tableColumn>\
         </tableColumns><tableStyleInfo name=\"TableStyleMedium9\" showFirstColumn=\"0\" showLastColumn=\"0\" showRowStripes=\"1\" showColumnStripes=\"0\"/></table>"
    );
    assert!(
        book.part("xl/worksheets/_rels/sheet1.xml.rels")
            .contains("<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/table\" Target=\"../tables/table1.xml\"/>")
    );
    assert!(!book.workbook().contains("_xlnm._FilterDatabase"));

    let blog = book.sheet(2);
    assert!(blog.contains(
        "<row r=\"4\"><c r=\"A4\" t=\"inlineStr\"><is><t>Total</t></is></c><c r=\"B4\"><f>SUBTOTAL(101,Table2[views])</f></c></row>"
    ));
    assert!(blog.ends_with(
        "<legacyDrawing r:id=\"rId2\"/><tableParts count=\"1\"><tablePart r:id=\"rId3\"/></tableParts></worksheet>"
    ));
    let posts = book.part("xl/tables/table2.xml");
    assert!(posts.contains(
        "name=\"Table2\" displayName=\"Table2\" ref=\"A1:B4\" totalsRowCount=\"1\"><tableColumns"
    ));
    assert!(posts.contains("<tableStyleInfo name=\"TableStyleLight1\" showFirstColumn=\"0\" showLastColumn=\"0\" showRowStripes=\"0\""));
    assert!(
        book.part("xl/worksheets/_rels/sheet2.xml.rels")
            .contains("Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/table\" Target=\"../tables/table2.xml\"")
    );
    let types = book.part("[Content_Types].xml");
    for id in [1, 2] {
        assert!(types.contains(&format!(
            "<Override PartName=\"/xl/tables/table{id}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml\"/>"
        )));
    }

    // A header cell with no name, a bad style and a sheet that ends inside a table throw.
    let mut writer = self::writer();
    writer.sheet(SheetOptions::named("Shop"), &[]).unwrap();
    writer
        .part(Part::Table(table(
            "A1:B3",
            vec![total("qty", Some(Function::Sum), None)],
        )))
        .unwrap();
    let message = invalid(writer.rows(&[header(&["item"])]));
    assert!(message.contains("header cell `B1`"), "{message}");
    let mut styled = table("D1:E3", Vec::new());
    styled.style = Some("Shiny".to_owned());
    assert!(invalid(writer.part(Part::Table(styled))).contains("not a table style"));
    writer.rows(&[header(&["item", "qty"])]).unwrap();
    match writer.finish() {
        Err(Error::Invalid(message)) => {
            assert!(message.contains("covers rows 1 to 3"), "{message}")
        }
        other => panic!(
            "a sheet ending before a totals row, not {:?}",
            other.map(|done| done.size)
        ),
    }
    writer.rows(&orders(2)).unwrap();
    writer.finish().unwrap();
}

fn conditional(range: &str, kind: Kind) -> Conditional {
    Conditional {
        formatted: range.to_owned(),
        kind,
        operator: None,
        values: Vec::new(),
        colors: Vec::new(),
        icons: None,
        rank: None,
        percent: false,
        style: None,
    }
}

#[test]
fn every_conditional_format_kind_is_written_with_its_style() {
    let red = Style {
        color: Some("#9C0006".to_owned()),
        fill: Some("#FFC7CE".to_owned()),
        ..Style::default()
    };
    let bold = Style {
        bold: Some(true),
        format: Some("money".to_owned()),
        border: Some(nvs_spreadsheet::Border {
            bottom: Some(nvs_spreadsheet::Line::Thin),
            ..Default::default()
        }),
        ..Style::default()
    };
    let styled = |mut rule: Conditional, style: &Style| {
        rule.style = Some(style.clone());
        rule
    };
    let mut cell = styled(conditional("B1:B9", Kind::Cell), &red);
    cell.operator = Some(Operator::Between);
    cell.values = vec!["1".to_owned(), "=$D$1".to_owned()];
    let mut expression = styled(conditional("A1:B9", Kind::Formula), &bold);
    expression.values = vec!["=$B1>5".to_owned()];
    let mut scale = conditional("B1:B9", Kind::ColorScale);
    scale.colors = vec!["#FFFFFF".to_owned(), "#00FF00".to_owned()];
    let mut bar = conditional("B1:B9", Kind::DataBar);
    bar.colors = vec!["#0000FF".to_owned()];
    let mut icons = conditional("B1:B9", Kind::IconSet);
    icons.icons = Some("4Rating".to_owned());
    let mut top = styled(conditional("B1:B9", Kind::Top), &red);
    top.rank = Some(3);
    let mut bottom = styled(conditional("B1:B9", Kind::Bottom), &red);
    bottom.percent = true;
    let rules = [
        cell,
        expression,
        scale,
        conditional("B1:B9", Kind::ColorScale),
        bar,
        icons,
        top,
        bottom,
        styled(conditional("A1:A9 C1:C9", Kind::Duplicates), &bold),
        styled(conditional("A1:A9", Kind::Unique), &red),
    ];

    let mut writer = writer();
    let mut out = writer
        .sheet(SheetOptions::named("Shop"), &orders(9))
        .unwrap();
    out.extend(writer.part(Part::Merge("D1:E1".to_owned())).unwrap());
    for rule in rules {
        out.extend(writer.part(Part::Conditional(rule)).unwrap());
    }

    // Each kind that needs something throws without it, and adds no rule.
    let mut no_operator = styled(conditional("B1:B9", Kind::Cell), &red);
    no_operator.values = vec!["1".to_owned()];
    let mut one_bound = no_operator.clone();
    one_bound.operator = Some(Operator::Between);
    let mut bad_icons = conditional("B1:B9", Kind::IconSet);
    bad_icons.icons = Some("3Stars".to_owned());
    let mut bad_rank = styled(conditional("B1:B9", Kind::Top), &red);
    bad_rank.rank = Some(101);
    bad_rank.percent = true;
    let mut bad_color = conditional("B1:B9", Kind::DataBar);
    bad_color.colors = vec!["blue".to_owned()];
    for (rule, says) in [
        (conditional("B1:B9", Kind::Unique), "needs a `style`"),
        (
            styled(conditional("B1:B9", Kind::DataBar), &red),
            "no `style`",
        ),
        (
            styled(conditional("B0", Kind::Unique), &red),
            "range of cells",
        ),
        (no_operator, "needs an operator"),
        (one_bound, "has 1 values. The operator `between` needs 2."),
        (bad_icons, "4Rating"),
        (bad_rank, "from 1 to 100"),
        (bad_color, "is not a color"),
    ] {
        let message = invalid(writer.part(Part::Conditional(rule)));
        assert!(message.contains(says), "{message}");
    }
    let book = finish(writer, out);

    let sheet = book.sheet(1);
    let rules = between(sheet, "<conditionalFormatting", "</worksheet>");
    let rules = &rules[..rules.rfind("</conditionalFormatting>").unwrap() + 24];
    assert_eq!(
        rules,
        "<conditionalFormatting sqref=\"B1:B9\"><cfRule type=\"cellIs\" dxfId=\"0\" priority=\"1\" operator=\"between\"><formula>1</formula><formula>$D$1</formula></cfRule></conditionalFormatting>\
         <conditionalFormatting sqref=\"A1:B9\"><cfRule type=\"expression\" dxfId=\"1\" priority=\"2\"><formula>$B1&gt;5</formula></cfRule></conditionalFormatting>\
         <conditionalFormatting sqref=\"B1:B9\"><cfRule type=\"colorScale\" priority=\"3\"><colorScale><cfvo type=\"min\"/><cfvo type=\"max\"/><color rgb=\"FFFFFFFF\"/><color rgb=\"FF00FF00\"/></colorScale></cfRule></conditionalFormatting>\
         <conditionalFormatting sqref=\"B1:B9\"><cfRule type=\"colorScale\" priority=\"4\"><colorScale><cfvo type=\"min\"/><cfvo type=\"percentile\" val=\"50\"/><cfvo type=\"max\"/><color rgb=\"FFF8696B\"/><color rgb=\"FFFFEB84\"/><color rgb=\"FF63BE7B\"/></colorScale></cfRule></conditionalFormatting>\
         <conditionalFormatting sqref=\"B1:B9\"><cfRule type=\"dataBar\" priority=\"5\"><dataBar><cfvo type=\"min\"/><cfvo type=\"max\"/><color rgb=\"FF0000FF\"/></dataBar></cfRule></conditionalFormatting>\
         <conditionalFormatting sqref=\"B1:B9\"><cfRule type=\"iconSet\" priority=\"6\"><iconSet iconSet=\"4Rating\"><cfvo type=\"percent\" val=\"0\"/><cfvo type=\"percent\" val=\"25\"/><cfvo type=\"percent\" val=\"50\"/><cfvo type=\"percent\" val=\"75\"/></iconSet></cfRule></conditionalFormatting>\
         <conditionalFormatting sqref=\"B1:B9\"><cfRule type=\"top10\" dxfId=\"0\" priority=\"7\" rank=\"3\"/></conditionalFormatting>\
         <conditionalFormatting sqref=\"B1:B9\"><cfRule type=\"top10\" dxfId=\"0\" priority=\"8\" rank=\"10\" percent=\"1\" bottom=\"1\"/></conditionalFormatting>\
         <conditionalFormatting sqref=\"A1:A9 C1:C9\"><cfRule type=\"duplicateValues\" dxfId=\"1\" priority=\"9\"/></conditionalFormatting>\
         <conditionalFormatting sqref=\"A1:A9\"><cfRule type=\"uniqueValues\" dxfId=\"0\" priority=\"10\"/></conditionalFormatting>"
    );
    assert!(
        sheet.contains("</mergeCells><conditionalFormatting"),
        "the rules come after the merged cells"
    );
    let styles = book.part("xl/styles.xml");
    assert!(styles.contains(
        "<dxfs count=\"2\">\
         <dxf><font><color rgb=\"FF9C0006\"/></font><fill><patternFill><bgColor rgb=\"FFFFC7CE\"/></patternFill></fill></dxf>\
         <dxf><font><b/></font><numFmt numFmtId=\"4\" formatCode=\"#,##0.00\"/>\
         <border><left/><right/><top/><bottom style=\"thin\"><color auto=\"1\"/></bottom><vertical/><horizontal/></border></dxf>\
         </dxfs>"
    ), "{styles}");
}

/// The chunks of a PNG `width` by `height` pixels: its signature, `IHDR`, one `IDAT` and `IEND`.
/// The CRCs are zero, because the writer does not compute them.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
    bytes.extend(width.to_be_bytes());
    bytes.extend(height.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0]);
    bytes.extend([0; 4]);
    bytes.extend(b"\x00\x00\x00\x02IDAT\x78\x9c");
    bytes.extend([0; 4]);
    bytes.extend(b"\x00\x00\x00\x00IEND");
    bytes.extend([0; 4]);
    bytes
}

fn image(at: &str, bytes: Vec<u8>) -> Image {
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
fn an_image_is_its_own_entry_anchored_to_its_cell() {
    let mut writer = writer();
    let mut out = writer.sheet(SheetOptions::named("Shop"), &[]).unwrap();
    let logo = png(2, 1);
    let written = writer.part(Part::Image(image("A1", logo.clone()))).unwrap();
    assert!(
        !written.is_empty(),
        "an image added before the first row is written at once"
    );
    out.extend(written);
    out.extend(writer.rows(&orders(10)).unwrap());
    let gif = b"GIF89a\x04\x00\x03\x00\x00\x00\x00;".to_vec();
    let mut product = image("B5", gif.clone());
    product.width = Some(40.0);
    product.offset_x = Some(3.0);
    product.offset_y = Some(2.0);
    assert!(
        writer.part(Part::Image(product)).unwrap().is_empty(),
        "an image added while the sheet's entry is open waits for it to end"
    );
    let book = finish(writer, out);

    assert_eq!(book.bytes("xl/media/image1.png"), logo.as_slice());
    assert_eq!(book.bytes("xl/media/image2.gif"), gif.as_slice());
    let sheet = book.sheet(1);
    assert!(sheet.contains("<drawing r:id=\"rId1\"/></worksheet>"));
    let rels = book.part("xl/worksheets/_rels/sheet1.xml.rels");
    assert!(rels.contains("Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing\" Target=\"../drawings/drawing1.xml\""));

    let drawing = book.part("xl/drawings/drawing1.xml");
    assert_eq!(
        drawing
            .matches("<xdr:twoCellAnchor editAs=\"oneCell\">")
            .count(),
        2
    );
    assert!(drawing.contains(
        "<xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"
    ), "the logo is at A1: {drawing}");
    assert!(
        drawing.contains("<a:off x=\"0\" y=\"0\"/><a:ext cx=\"19050\" cy=\"9525\"/>"),
        "the logo is its own size: {drawing}"
    );
    assert!(drawing.contains(
        "<xdr:from><xdr:col>1</xdr:col><xdr:colOff>28575</xdr:colOff><xdr:row>4</xdr:row><xdr:rowOff>19050</xdr:rowOff></xdr:from>"
    ), "an offset is kept inside the cell: {drawing}");
    assert!(
        drawing.contains("<a:ext cx=\"381000\" cy=\"285750\"/>"),
        "a width alone scales the height by the image's ratio"
    );
    let drawing_rels = book.part("xl/drawings/_rels/drawing1.xml.rels");
    assert!(drawing_rels.contains("Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"../media/image1.png\""));
    assert!(drawing_rels.contains("Id=\"rId2\""));
    assert!(drawing_rels.contains("Target=\"../media/image2.gif\""));

    let types = book.part("[Content_Types].xml");
    assert!(types.contains("<Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"gif\" ContentType=\"image/gif\"/>"));
    assert!(types.contains("<Override PartName=\"/xl/drawings/drawing1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawing+xml\"/>"));
}

#[test]
fn an_image_that_is_not_png_jpeg_or_gif_throws() {
    let mut writer = writer();
    let mut out = writer
        .sheet(SheetOptions::named("Shop"), &orders(2))
        .unwrap();
    for bytes in [
        b"BM\x36\x00\x00\x00".to_vec(),
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec(),
        Vec::new(),
    ] {
        let message = invalid(writer.part(Part::Image(image("C3", bytes))));
        assert!(
            message.contains("`C3`") && message.contains("PNG, JPEG or GIF"),
            "{message}"
        );
    }
    let message = invalid(writer.part(Part::Image(image("C0", png(2, 1)))));
    assert!(message.contains("`C0`"), "{message}");
    let mut huge = image("C3", png(2, 1));
    huge.width = Some(f64::NAN);
    assert!(invalid(writer.part(Part::Image(huge))).contains("width"));

    out.extend(writer.part(Part::Image(image("C3", png(2, 1)))).unwrap());
    let book = finish(writer, out);
    assert!(
        book.has("xl/media/image1.png"),
        "an image that throws takes no number"
    );
}

#[test]
fn an_image_cut_short_throws() {
    let mut writer = writer();
    let mut out = writer.sheet(SheetOptions::named("Shop"), &[]).unwrap();
    let whole = png(2, 1);
    let mut no_end = whole.clone();
    no_end.truncate(whole.len() - 12);
    let mut no_data = whole[..33].to_vec();
    no_data.extend(&whole[whole.len() - 12..]);
    let mut too_long = whole.clone();
    too_long[33..37].copy_from_slice(&u32::MAX.to_be_bytes());
    for bytes in [
        whole[..8].to_vec(),
        whole[..30].to_vec(),
        no_end,
        no_data,
        too_long,
        png(0, 1),
    ] {
        let message = invalid(writer.part(Part::Image(image("C3", bytes))));
        assert!(
            message.contains("`C3`") && message.contains("cut short or damaged"),
            "{message}"
        );
    }
    let mut trailing = whole.clone();
    trailing.extend(b"more bytes after the end");
    out.extend(
        writer
            .part(Part::Image(image("C3", trailing.clone())))
            .unwrap(),
    );
    let book = finish(writer, out);
    assert_eq!(
        book.bytes("xl/media/image1.png"),
        trailing.as_slice(),
        "bytes after `IEND` are kept"
    );
}

#[test]
fn an_image_that_ends_past_the_grid_throws() {
    for (cell, width, height) in [
        ("A1", Some(10_000_000.0), None),
        ("XFC1", Some(1_000.0), Some(10.0)),
        ("A1048570", Some(10.0), Some(1_000.0)),
    ] {
        let mut writer = writer();
        writer
            .sheet(SheetOptions::named("Shop"), &orders(2))
            .unwrap();
        let mut wide = image(cell, png(2, 1));
        wide.width = width;
        wide.height = height;
        writer.part(Part::Image(wide)).unwrap();
        let message = match writer.finish() {
            Err(Error::Invalid(message)) => message,
            other => panic!("an image past the grid, not {other:?}"),
        };
        assert!(
            message.contains(&format!("The image at `{cell}` ends at the cell `"))
                && message.contains("outside the sheet"),
            "{message}"
        );
    }
    let mut writer = writer();
    let out = writer
        .sheet(SheetOptions::named("Shop"), &orders(2))
        .unwrap();
    let mut edge = image("XFD1", png(2, 1));
    edge.width = Some(64.0);
    writer.part(Part::Image(edge)).unwrap();
    let book = finish(writer, out);
    assert!(
        book.part("xl/drawings/drawing1.xml")
            .contains("<xdr:to><xdr:col>16383</xdr:col>"),
        "an image that fills the last column is inside the grid"
    );
}

#[test]
fn a_whole_number_rule_writes_no_text_limits() {
    let mut writer = writer();
    let out = writer
        .sheet(SheetOptions::named("Shop"), &orders(9))
        .unwrap();
    writer
        .part(Part::Validation(validation(
            "B2:B9",
            RuleKind::Whole,
            &["1", "100"],
        )))
        .unwrap();
    let mut at_least = validation("C2:C9", RuleKind::Whole, &["=$F$1"]);
    at_least.operator = Some(Operator::GreaterOrEqual);
    writer.part(Part::Validation(at_least)).unwrap();
    for bound in ["1.5", "ten", "inf"] {
        let mut rule = validation("D2:D9", RuleKind::Whole, &[bound]);
        rule.operator = Some(Operator::LessOrEqual);
        let message = invalid(writer.part(Part::Validation(rule)));
        assert!(
            message.contains(&format!("has the bound `{bound}`"))
                && message.contains("whole number"),
            "{message}"
        );
    }
    let book = finish(writer, out);
    let rules = between(book.sheet(1), "<dataValidations", "</dataValidations>");
    assert!(
        rules.starts_with("<dataValidations count=\"2\">"),
        "{rules}"
    );
    assert!(!rules.contains("textLength"), "{rules}");
    assert!(rules.contains(
        "<dataValidation type=\"whole\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"B2:B9\"><formula1>1</formula1><formula2>100</formula2></dataValidation>"
    ), "{rules}");
    assert!(rules.contains(
        "<dataValidation type=\"whole\" operator=\"greaterThanOrEqual\" showInputMessage=\"1\" showErrorMessage=\"1\" sqref=\"C2:C9\"><formula1>$F$1</formula1></dataValidation>"
    ), "{rules}");
}

#[test]
fn a_second_print_on_a_sheet_throws() {
    let mut writer = writer();
    let mut out = writer.sheet(SheetOptions::named("Shop"), &[]).unwrap();
    writer
        .part(Part::Print(Print {
            orientation: Some(Orientation::Landscape),
            fit_width: Some(1),
            ..Print::default()
        }))
        .unwrap();
    let message = invalid(writer.part(Part::Print(Print {
        header: Some("&CShop".to_owned()),
        ..Print::default()
    })));
    assert!(
        message.contains("The sheet `Shop` has its page setup already"),
        "{message}"
    );
    out.extend(writer.rows(&orders(2)).unwrap());
    let book = finish(writer, out);
    let sheet = book.sheet(1);
    assert!(
        sheet.contains("<pageSetup orientation=\"landscape\" fitToWidth=\"1\" fitToHeight=\"0\"/>"),
        "the first page setup is kept: {sheet}"
    );
    assert!(!sheet.contains("<headerFooter>"), "{sheet}");
}
