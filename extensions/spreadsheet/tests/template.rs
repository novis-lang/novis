//! Template fill (`rule:core-classes/spreadsheet-template-fill`): a template built here part by
//! part, filled, and each part of the result read back as text.

use std::collections::BTreeMap;

use nvs_spreadsheet::read::Source;
use nvs_spreadsheet::read::zip::{Bound, Directory};
use nvs_spreadsheet::template::shift::{self, Shift};
use nvs_spreadsheet::template::{Placeholder, Template};
use nvs_spreadsheet::zip::Archive;
use nvs_spreadsheet::{Date, Error, Value};

struct Bytes(Vec<u8>);

impl Source for Bytes {
    fn size(&self) -> Result<u64, Error> {
        self.0.as_slice().size()
    }

    fn range(&self, offset: u64, length: u64) -> Result<Vec<u8>, Error> {
        self.0.as_slice().range(offset, length)
    }
}

const STRINGS: [&str; 6] = [
    "Invoice {{number}}",
    "{{customer.name}}",
    "{{items.name}}",
    "{{items.qty}}",
    "Item",
    "Sum",
];

/// The invoice: a header with placeholders, the item row 5, and its sum on row 6.
const INVOICE: &str = concat!(
    r#"<row r="1" spans="1:4"><c r="A1" t="s"><v>0</v></c><c r="B1" s="1" t="s"><v>1</v></c></row>"#,
    r#"<row r="2"><c r="A2" s="3" t="inlineStr"><is><t>{{total}}</t></is></c><c r="B2" t="inlineStr"><is><t>{{note}}</t></is></c></row>"#,
    r#"<row r="4"><c r="A4" t="s"><v>4</v></c></row>"#,
    r#"<row r="5" ht="20" customHeight="1"><c r="A5" t="s"><v>2</v></c><c r="B5" s="1" t="s"><v>3</v></c><c r="D5" s="2"><f>B5*C5</f><v>0</v></c></row>"#,
    r#"<row r="6"><c r="A6" t="s"><v>5</v></c><c r="D6"><f>SUM(D5:D5)</f><v>0</v></c></row>"#,
    r#"<row r="7"><c r="A7"><f>D6+Summary!B2</f><v>0</v></c></row>"#,
);

/// A sheet with only fixed cells and one placeholder, for the parts copied as they are.
const PLAIN: &str = concat!(
    r#"<row r="1" spans="1:4"><c r="A1" t="s"><v>4</v></c><c r="B1" s="1" t="s"><v>1</v></c></row>"#,
    r#"<row r="5"><c r="D5" s="2"><f>B5*C5</f><v>0</v></c></row>"#,
    r#"<row r="6"><c r="D6"><f>SUM(D5:D5)</f><v>0</v></c></row>"#,
);

const SHEET_TAIL: &str = concat!(
    r#"<mergeCells count="1"><mergeCell ref="A6:C6"/></mergeCells>"#,
    r#"<conditionalFormatting sqref="D5:D6"><cfRule type="expression" dxfId="0" priority="1"><formula>D5&gt;100</formula></cfRule></conditionalFormatting>"#,
    r#"<dataValidations count="1"><dataValidation type="whole" sqref="B7"><formula1>0</formula1></dataValidation></dataValidations>"#,
    r#"<drawing r:id="rId1"/><tableParts count="1"><tablePart r:id="rId2"/></tableParts>"#,
    r#"<extLst><ext uri="{78C0D931-6437-407d-A8EE-F0AAD7539E65}" xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"><x14:conditionalFormattings><x14:conditionalFormatting xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"><x14:cfRule type="expression" priority="2" id="{00000000-0000-0000-0000-000000000001}"><xm:f>A7&gt;D6</xm:f></x14:cfRule><xm:sqref>D5:D6 A7</xm:sqref></x14:conditionalFormatting></x14:conditionalFormattings></ext></extLst></worksheet>"#,
);

fn sheet(rows: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><dimension ref="A1:D7"/><sheetData>{rows}</sheetData>{SHEET_TAIL}"#
    )
}

/// The sheet `Summary`'s rows: a formula over the invoice's sum, and a number.
const SUMMARY: &str = concat!(
    r#"<row r="1"><c r="B1"><f>Invoice!D6</f><v>0</v></c></row>"#,
    r#"<row r="2"><c r="B2"><v>10</v></c></row>"#,
);

/// A template with `rows` on the sheet `Invoice`, the defined names `names`, and a VBA project
/// when `macros` is true.
fn template(rows: &str, names: &str, macros: bool) -> Vec<u8> {
    template_with(rows, SUMMARY, names, macros)
}

/// A template as [`template`] makes it, with `summary` as the rows of the sheet `Summary`.
fn template_with(rows: &str, summary: &str, names: &str, macros: bool) -> Vec<u8> {
    let main = if macros {
        "application/vnd.ms-excel.sheet.macroEnabled.main+xml"
    } else {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
    };
    let vba_type = if macros {
        r#"<Default Extension="bin" ContentType="application/vnd.ms-office.vbaProject"/>"#
    } else {
        ""
    };
    let vba_rel = if macros {
        r#"<Relationship Id="rId9" Type="http://schemas.microsoft.com/office/2006/relationships/vbaProject" Target="vbaProject.bin"/>"#
    } else {
        ""
    };
    let strings: String = STRINGS
        .iter()
        .map(|s| format!("<si><t>{s}</t></si>"))
        .collect();
    let mut parts: Vec<(&str, String)> = vec![
        ("[Content_Types].xml", format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>{vba_type}<Override PartName="/xl/workbook.xml" ContentType="{main}"/><Override PartName="/xl/calcChain.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml"/></Types>"#
        )),
        ("_rels/.rels", r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_owned()),
        ("xl/workbook.xml", format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Invoice" sheetId="1" r:id="rId1"/><sheet name="Summary" sheetId="2" r:id="rId2"/></sheets><definedNames><definedName name="_xlnm.Print_Area" localSheetId="0">Invoice!$A$1:$D$8</definedName>{names}</definedNames><calcPr calcId="191029"/></workbook>"#
        )),
        ("xl/_rels/workbook.xml.rels", format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain" Target="calcChain.xml"/>{vba_rel}</Relationships>"#
        )),
        ("xl/worksheets/sheet1.xml", sheet(rows)),
        ("xl/worksheets/_rels/sheet1.xml.rels", r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/table" Target="../tables/table1.xml"/></Relationships>"#.to_owned()),
        ("xl/worksheets/sheet2.xml", format!(r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{summary}</sheetData></worksheet>"#)),
        ("xl/sharedStrings.xml", format!(r#"<?xml version="1.0" encoding="UTF-8"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="6" uniqueCount="6">{strings}</sst>"#)),
        ("xl/styles.xml", r#"<?xml version="1.0" encoding="UTF-8"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cellXfs count="4"><xf/><xf/><xf/><xf/></cellXfs></styleSheet>"#.to_owned()),
        ("xl/tables/table1.xml", r#"<?xml version="1.0" encoding="UTF-8"?><table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" id="1" name="Items" displayName="Items" ref="A4:D5"><autoFilter ref="A4:D5"/><tableColumns count="4"><tableColumn id="1" name="Item"/><tableColumn id="2" name="Qty"/><tableColumn id="3" name="Price"/><tableColumn id="4" name="Total"/></tableColumns></table>"#.to_owned()),
        ("xl/drawings/drawing1.xml", r#"<?xml version="1.0" encoding="UTF-8"?><xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"><xdr:twoCellAnchor><xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>7</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:to><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>12</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to></xdr:twoCellAnchor></xdr:wsDr>"#.to_owned()),
        ("xl/charts/chart1.xml", r#"<?xml version="1.0" encoding="UTF-8"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:barChart><c:ser><c:val><c:numRef><c:f>Invoice!$D$5:$D$5</c:f></c:numRef></c:val></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#.to_owned()),
        ("xl/calcChain.xml", r#"<?xml version="1.0" encoding="UTF-8"?><calcChain xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><c r="D5" i="1"/></calcChain>"#.to_owned()),
    ];
    if macros {
        parts.push(("xl/vbaProject.bin", "not really a VBA project".to_owned()));
    }
    let mut archive = Archive::new();
    for (name, text) in &parts {
        archive.begin(name).unwrap();
        archive.write(text.as_bytes()).unwrap();
        archive.end().unwrap();
    }
    archive.close().unwrap();
    archive.take()
}

/// Every part of the zip `bytes`, by name, as text.
fn parts(bytes: &[u8]) -> BTreeMap<String, String> {
    let source = Bytes(bytes.to_vec());
    let directory = Directory::read(&source, Bound::default()).unwrap();
    directory
        .entries()
        .iter()
        .map(|entry| {
            let text = String::from_utf8(directory.whole(&source, entry).unwrap()).unwrap();
            (entry.name.clone(), text)
        })
        .collect()
}

fn text(value: &str) -> Option<Value> {
    Some(Value::Text(value.to_owned()))
}

fn values(pairs: &[(&str, Option<Value>)]) -> Vec<Placeholder> {
    pairs
        .iter()
        .map(|(path, value)| ((*path).to_owned(), value.clone()))
        .collect()
}

fn header() -> Vec<Placeholder> {
    values(&[
        ("number", Some(Value::Integer(42))),
        ("customer.name", text("Shop")),
        ("total", Some(Value::Number(12.5))),
        ("note", text("=1+1")),
    ])
}

fn item(index: usize) -> Vec<Placeholder> {
    values(&[
        ("name", text(&format!("Item {index}"))),
        ("qty", Some(Value::Integer(index as i64))),
    ])
}

/// Fills `template` and returns the whole file: `lists` each with its items, in batches of 10.
fn fill(
    template: &[u8],
    sets: &[(&str, Option<Value>)],
    placeholders: Vec<Placeholder>,
    lists: &[(&str, Vec<Vec<Placeholder>>)],
) -> Result<Vec<u8>, Error> {
    let source = Bytes(template.to_vec());
    let mut filled = Template::open(&source, Bound::default())?;
    for (address, value) in sets {
        filled.set(address, value.clone())?;
    }
    let names: Vec<String> = lists.iter().map(|(name, _)| (*name).to_owned()).collect();
    let (mut out, order) = filled.fill(placeholders, &names)?;
    for name in order {
        let (_, items) = lists.iter().find(|(list, _)| *list == name).unwrap();
        for batch in items.chunks(10) {
            out.extend(filled.items(&name, batch.to_vec())?);
        }
    }
    let finished = filled.finish(&source)?;
    out.extend(finished.data);
    assert_eq!(out.len() as u64, finished.size);
    Ok(out)
}

fn invoice(items: usize) -> BTreeMap<String, String> {
    let items = (1..=items).map(item).collect();
    let out = fill(
        &template(INVOICE, "", false),
        &[],
        header(),
        &[("items", items)],
    )
    .unwrap();
    parts(&out)
}

#[test]
fn every_part_but_the_filled_cells_is_copied_unchanged() {
    let before = parts(&template(PLAIN, "", false));
    let out = fill(&template(PLAIN, "", false), &[], header(), &[]).unwrap();
    let after = parts(&out);
    assert!(!after.contains_key("xl/calcChain.xml"));
    for (name, text) in &before {
        match name.as_str() {
            "xl/calcChain.xml" => {}
            "[Content_Types].xml" | "xl/_rels/workbook.xml.rels" => {
                assert!(!after[name].contains("calcChain"), "{name}");
            }
            // The workbook recalculates on open, and is otherwise unchanged.
            "xl/workbook.xml" => {
                assert_eq!(
                    after[name],
                    text.replace(
                        "calcId=\"191029\"",
                        "calcId=\"191029\" fullCalcOnLoad=\"1\""
                    )
                );
            }
            "xl/worksheets/sheet1.xml" => {
                let filled = text.replace(
                    r#"<c r="B1" s="1" t="s"><v>1</v></c>"#,
                    r#"<c r="B1" s="1" t="inlineStr"><is><t>Shop</t></is></c>"#,
                );
                assert_eq!(after[name], filled);
            }
            _ => assert_eq!(&after[name], text, "{name}"),
        }
    }
}

#[test]
fn a_lone_placeholder_keeps_the_value_type_and_text_around_it_makes_text() {
    let sheet = &invoice(1)["xl/worksheets/sheet1.xml"];
    assert!(
        sheet.contains(r#"<c r="A1" t="inlineStr"><is><t>Invoice 42</t></is></c>"#),
        "{sheet}"
    );
    assert!(
        sheet.contains(r#"<c r="A2" s="3"><v>12.5</v></c>"#),
        "{sheet}"
    );
    assert!(sheet.contains(r#"<c r="B5" s="1"><v>1</v></c>"#), "{sheet}");

    let dated = fill(
        &template(PLAIN, "", false),
        &[],
        values(&[(
            "customer.name",
            Some(Value::Date(Date {
                year: 2024,
                month: 3,
                day: 1,
            })),
        )]),
        &[],
    )
    .unwrap();
    let sheet = &parts(&dated)["xl/worksheets/sheet1.xml"];
    assert!(
        sheet.contains(r#"<c r="B1" s="1"><v>45352</v></c>"#),
        "{sheet}"
    );
    let empty = fill(
        &template(PLAIN, "", false),
        &[],
        values(&[("customer.name", None)]),
        &[],
    )
    .unwrap();
    assert!(parts(&empty)["xl/worksheets/sheet1.xml"].contains(r#"<c r="B1" s="1"/>"#));
}

#[test]
fn a_value_beginning_with_equals_stays_text() {
    let sheet = &invoice(1)["xl/worksheets/sheet1.xml"];
    assert!(
        sheet.contains(r#"<c r="B2" t="inlineStr"><is><t>=1+1</t></is></c>"#),
        "{sheet}"
    );
}

#[test]
fn a_placeholder_the_data_lacks_throws_naming_it_and_its_cell() {
    let missing = values(&[("number", Some(Value::Integer(1)))]);
    let err = fill(
        &template(INVOICE, "", false),
        &[],
        missing,
        &[("items", vec![item(1)])],
    )
    .unwrap_err();
    let Error::Invalid(message) = err else {
        panic!("{err:?}")
    };
    assert!(
        message.contains("`{{customer.name}}`") && message.contains("`Invoice!B1`"),
        "{message}"
    );

    let err = fill(
        &template(INVOICE, "", false),
        &[],
        header(),
        &[("items", vec![values(&[("name", text("x"))])])],
    )
    .unwrap_err();
    let Error::Invalid(message) = err else {
        panic!("{err:?}")
    };
    assert!(
        message.contains("`{{items.qty}}`") && message.contains("`Invoice!B5`"),
        "{message}"
    );
}

#[test]
fn a_list_placeholder_repeats_its_row_with_its_style() {
    let sheet = &invoice(3)["xl/worksheets/sheet1.xml"];
    for row in 5..=7 {
        let index = row - 4;
        assert!(sheet.contains(&format!(r#"<row r="{row}" ht="20" customHeight="1"><c r="A{row}" t="inlineStr"><is><t>Item {index}</t></is></c><c r="B{row}" s="1"><v>{index}</v></c><c r="D{row}" s="2"><f>B{row}*C{row}</f><v>0</v></c></row>"#)), "{sheet}");
    }
    assert!(sheet.contains(r#"<row r="4"><c r="A4" t="s"><v>4</v></c></row>"#));
    assert!(
        sheet.contains(r#"<row r="8"><c r="A8" t="s"><v>5</v></c><c r="D8"><f>SUM(D5:D7)</f>"#),
        "{sheet}"
    );
    assert!(!sheet.contains("<dimension"));
}

#[test]
fn a_defined_name_repeats_a_block_of_rows() {
    let rows = concat!(
        r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Lines</t></is></c></row>"#,
        r#"<row r="5"><c r="A5" t="inlineStr"><is><t>{{lines.name}}</t></is></c></row>"#,
        r#"<row r="6"><c r="B6" t="inlineStr"><is><t>{{lines.note}}</t></is></c></row>"#,
        r#"<row r="7"><c r="A7"><f>COUNTA(A5:A6)</f></c></row>"#,
    );
    let names = r#"<definedName name="lines">Invoice!$A$5:$B$6</definedName>"#;
    let lines = (1..=2)
        .map(|i| {
            values(&[
                ("name", text(&format!("Line {i}"))),
                ("note", text(&format!("Note {i}"))),
            ])
        })
        .collect();
    let out = fill(
        &template(rows, names, false),
        &[],
        vec![],
        &[("lines", lines)],
    )
    .unwrap();
    let parts = parts(&out);
    let sheet = &parts["xl/worksheets/sheet1.xml"];
    assert!(
        sheet.contains(concat!(
            r#"<row r="5"><c r="A5" t="inlineStr"><is><t>Line 1</t></is></c></row>"#,
            r#"<row r="6"><c r="B6" t="inlineStr"><is><t>Note 1</t></is></c></row>"#,
            r#"<row r="7"><c r="A7" t="inlineStr"><is><t>Line 2</t></is></c></row>"#,
            r#"<row r="8"><c r="B8" t="inlineStr"><is><t>Note 2</t></is></c></row>"#,
            r#"<row r="9"><c r="A9"><f>COUNTA(A5:A8)</f></c></row>"#,
        )),
        "{sheet}"
    );
    assert!(
        parts["xl/workbook.xml"]
            .contains(r#"<definedName name="lines">Invoice!$A$5:$B$8</definedName>"#)
    );
}

#[test]
fn a_list_inside_a_list_throws() {
    let rows = r#"<row r="5"><c r="A5" t="inlineStr"><is><t>{{orders.id}}</t></is></c><c r="B5" t="inlineStr"><is><t>{{lines.id}}</t></is></c></row>"#;
    let err = fill(
        &template(rows, "", false),
        &[],
        vec![],
        &[("orders", vec![]), ("lines", vec![])],
    )
    .unwrap_err();
    let Error::Invalid(message) = err else {
        panic!("{err:?}")
    };
    assert!(
        message.contains("`lines`") && message.contains("`orders`") && message.contains("inside"),
        "{message}"
    );

    let names = r#"<definedName name="orders">Invoice!$A$4:$B$6</definedName>"#;
    let err = fill(
        &template(rows.replace("orders.id", "x").as_str(), names, false),
        &[],
        vec![],
        &[("orders", vec![]), ("lines", vec![])],
    )
    .unwrap_err();
    let Error::Invalid(message) = err else {
        panic!("{err:?}")
    };
    assert!(
        message.contains("`lines` is inside the list `orders`"),
        "{message}"
    );
}

#[test]
fn rows_below_a_block_move_with_every_part_that_points_at_them() {
    let parts = invoice(3);
    let sheet = &parts["xl/worksheets/sheet1.xml"];
    assert!(
        sheet.contains(r#"<row r="9"><c r="A9"><f>D8+Summary!B2</f>"#),
        "{sheet}"
    );
    assert!(sheet.contains(r#"<mergeCell ref="A8:C8"/>"#));
    assert!(sheet.contains(r#"<conditionalFormatting sqref="D5:D8"><cfRule type="expression" dxfId="0" priority="1"><formula>D5&gt;100</formula>"#), "{sheet}");
    assert!(sheet.contains(r#"<dataValidation type="whole" sqref="B9">"#));
    assert!(parts["xl/worksheets/sheet2.xml"].contains("<f>Invoice!D8</f>"));
    assert!(
        parts["xl/workbook.xml"].contains(r#"localSheetId="0">Invoice!$A$1:$D$10</definedName>"#)
    );
    assert!(parts["xl/workbook.xml"].contains(r#"fullCalcOnLoad="1""#));
    let drawing = &parts["xl/drawings/drawing1.xml"];
    assert!(
        drawing.contains("<xdr:row>9</xdr:row>") && drawing.contains("<xdr:row>14</xdr:row>"),
        "{drawing}"
    );
    assert!(!parts.contains_key("xl/calcChain.xml"));
}

#[test]
fn lists_on_two_sheets_each_repeat_and_shift_every_reference_to_their_sheet() {
    // `Summary` repeats row 3 for `lines`, below the formula over the invoice's sum.
    let summary = concat!(
        r#"<row r="1"><c r="B1"><f>Invoice!D6</f><v>0</v></c></row>"#,
        r#"<row r="2"><c r="B2"><v>10</v></c></row>"#,
        r#"<row r="3"><c r="A3" t="inlineStr"><is><t>{{lines.label}}</t></is></c><c r="B3" t="inlineStr"><is><t>{{lines.amount}}</t></is></c></row>"#,
        r#"<row r="4"><c r="B4"><f>SUM(B3:B3)</f><v>0</v></c></row>"#,
    );
    let line = |at: i64| {
        values(&[
            ("label", text(&format!("Line {at}"))),
            ("amount", Some(Value::Integer(at))),
        ])
    };
    // The first run has the invoice name only a `Summary` row above its block, so the invoice
    // streams. The second adds a row naming the sum below it, so the invoice waits for `lines`.
    let names_the_sum = r#"<row r="8"><c r="A8"><f>Summary!B4</f><v>0</v></c></row>"#;
    for invoice_rows in [INVOICE.to_owned(), format!("{INVOICE}{names_the_sum}")] {
        let source = template_with(&invoice_rows, summary, "", false);
        let template = Bytes(source.clone());
        let mut filled = Template::open(&template, Bound::default()).unwrap();
        let lists = ["lines".to_owned(), "items".to_owned()];
        let (_, order) = filled.fill(header(), &lists).unwrap();
        assert_eq!(order, ["items", "lines"]);
        let out = fill(
            &source,
            &[],
            header(),
            &[
                ("lines", (1..=2).map(line).collect()),
                ("items", (1..=3).map(item).collect()),
            ],
        )
        .unwrap();
        let parts = parts(&out);
        let invoice = &parts["xl/worksheets/sheet1.xml"];
        assert!(invoice.contains("<f>SUM(D5:D7)</f>"), "{invoice}");
        assert!(
            invoice.contains(r#"<c r="A9"><f>D8+Summary!B2</f>"#),
            "{invoice}"
        );
        if invoice_rows.len() > INVOICE.len() {
            assert!(
                invoice.contains(r#"<c r="A10"><f>Summary!B5</f>"#),
                "{invoice}"
            );
        }
        let summary = &parts["xl/worksheets/sheet2.xml"];
        assert!(summary.contains("<f>Invoice!D8</f>"), "{summary}");
        assert!(summary.contains(r#"<c r="A4" t="inlineStr">"#), "{summary}");
        assert!(summary.contains("Line 2"), "{summary}");
        assert!(
            summary.contains(r#"<c r="B5"><f>SUM(B3:B4)</f>"#),
            "{summary}"
        );
        assert!(
            parts["xl/workbook.xml"]
                .contains(r#"localSheetId="0">Invoice!$A$1:$D$10</definedName>"#)
        );
    }
}

#[test]
fn a_shared_formula_in_a_repeated_row_gets_its_own_group_per_copy() {
    // Row 5 shares `B5*C5` over D5:E5 as group 0, and row 7 shares `A6` over B7:C7 as group 3.
    let rows = INVOICE
        .replace(
            r#"<c r="D5" s="2"><f>B5*C5</f><v>0</v></c>"#,
            r#"<c r="D5" s="2"><f t="shared" ref="D5:E5" si="0">B5*C5</f><v>0</v></c><c r="E5"><f t="shared" si="0"/><v>0</v></c>"#,
        )
        .replace(
            r#"<c r="A7"><f>D6+Summary!B2</f><v>0</v></c>"#,
            r#"<c r="A7"><f>D6+Summary!B2</f><v>0</v></c><c r="B7"><f t="shared" ref="B7:C7" si="3">A6</f><v>0</v></c><c r="C7"><f t="shared" si="3"/><v>0</v></c>"#,
        );
    let items = (1..=3).map(item).collect();
    let out = fill(
        &template(&rows, "", false),
        &[],
        header(),
        &[("items", items)],
    )
    .unwrap();
    let sheet = &parts(&out)["xl/worksheets/sheet1.xml"];
    // The first copy keeps group 0, and each later copy takes the next free number.
    for (row, group) in [(5, 0), (6, 4), (7, 5)] {
        let master = format!(
            r#"<c r="D{row}" s="2"><f t="shared" ref="D{row}:E{row}" si="{group}">B{row}*C{row}</f>"#
        );
        let follower = format!(r#"<c r="E{row}"><f t="shared" si="{group}"/>"#);
        assert!(
            sheet.contains(&master) && sheet.contains(&follower),
            "{sheet}"
        );
    }
    // A group below the block moves with its rows and keeps its number.
    assert!(
        sheet.contains(r#"<c r="B9"><f t="shared" ref="B9:C9" si="3">A8</f>"#)
            && sheet.contains(r#"<c r="C9"><f t="shared" si="3"/>"#),
        "{sheet}"
    );
}

#[test]
fn an_x14_conditional_format_moves_with_its_rows() {
    let sheet = &invoice(3)["xl/worksheets/sheet1.xml"];
    assert!(sheet.contains("<xm:f>A9&gt;D8</xm:f>"), "{sheet}");
    assert!(sheet.contains("<xm:sqref>D5:D8 A9</xm:sqref>"), "{sheet}");
}

#[test]
fn a_nul_in_a_filled_value_is_escaped_as_every_written_cell_is() {
    // `note` fills B2 with a NUL, text that looks like an escape, and a carriage return.
    let data = header()
        .into_iter()
        .map(|(path, value)| match path.as_str() {
            "note" => (path, text("a\0b_x0041_\r")),
            _ => (path, value),
        })
        .collect();
    let out = fill(
        &template(INVOICE, "", false),
        &[],
        data,
        &[("items", vec![item(1)])],
    )
    .unwrap();
    let sheet = &parts(&out)["xl/worksheets/sheet1.xml"];
    assert!(
        sheet.contains(r#"<c r="B2" t="inlineStr"><is><t xml:space="preserve">a_x0000_b_x005F_x0041__x000D_</t></is></c>"#),
        "{sheet}"
    );
}

#[test]
fn a_range_over_the_template_row_grows_with_the_block() {
    let hundred = invoice(100);
    assert!(hundred["xl/worksheets/sheet1.xml"].contains("<f>SUM(D5:D104)</f>"));
    assert!(
        hundred["xl/tables/table1.xml"].contains(r#"ref="A4:D104"><autoFilter ref="A4:D104"/>"#)
    );
    assert!(hundred["xl/charts/chart1.xml"].contains("<c:f>Invoice!$D$5:$D$104</c:f>"));

    // A formula above the block that names its rows: the sheet waits for every item.
    let above = INVOICE.replace(
        r#"<row r="4">"#,
        r#"<row r="3"><c r="D3"><f>SUM(D5:D5)+D6</f></c></row><row r="4">"#,
    );
    let items = (1..=25).map(item).collect();
    let out = fill(
        &template(&above, "", false),
        &[],
        header(),
        &[("items", items)],
    )
    .unwrap();
    let sheet = &parts(&out)["xl/worksheets/sheet1.xml"];
    assert!(sheet.contains("<f>SUM(D5:D29)+D30</f>"), "{sheet}");
    assert!(sheet.contains(r#"<row r="29" ht="20""#) && sheet.contains("<f>SUM(D5:D29)</f>"));
}

#[test]
fn the_shifter_moves_every_a1_reference_form_and_nothing_else() {
    let shifts = [Shift {
        sheet: "Data".to_owned(),
        top: 5,
        bottom: 5,
        delta: 10,
        copy: None,
    }];
    let cases = [
        ("A1+A6", "A1+A16"),
        ("$A$6*$B6", "$A$16*$B16"),
        ("SUM(A5:B6)", "SUM(A5:B16)"),
        ("SUM(A5:A5)", "SUM(A5:A15)"),
        ("A5", "A5"),
        ("SUM(6:7)", "SUM(16:17)"),
        ("SUM($6:$6)", "SUM($16:$16)"),
        ("SUM(C:D)", "SUM(C:D)"),
        ("Data!A6", "Data!A16"),
        ("'Data'!A6+'It''s'!A6", "'Data'!A16+'It''s'!A6"),
        ("Other!A6", "Other!A6"),
        ("\"A6\"&A6", "\"A6\"&A16"),
        ("LOG10(A6)", "LOG10(A16)"),
        ("Items[A6]+TAX1_RATE+A6", "Items[A6]+TAX1_RATE+A16"),
        ("1.5*A7", "1.5*A17"),
        ("Jan:Mar!A6", "Jan:Mar!A6"),
    ];
    for (before, after) in cases {
        assert_eq!(shift::apply(before, "Data", &shifts), after, "{before}");
    }
    assert_eq!(shift::apply("A6+Data!A6", "Other", &shifts), "A6+Data!A16");
    let copy = [Shift {
        copy: Some(3),
        ..shifts[0].clone()
    }];
    assert_eq!(shift::apply("A5*$A$5+A9", "Data", &copy), "A8*$A$5+A19");
}

#[test]
fn set_writes_one_cell_by_address() {
    let sets = [
        ("Summary!B2", Some(Value::Integer(99))),
        ("C1", text("Paid")),
        ("Invoice!E9", Some(Value::Boolean(true))),
    ];
    let out = fill(&template(PLAIN, "", false), &sets, header(), &[]).unwrap();
    let parts = parts(&out);
    assert!(parts["xl/worksheets/sheet2.xml"].contains(r#"<c r="B2"><v>99</v></c>"#));
    let sheet = &parts["xl/worksheets/sheet1.xml"];
    assert!(
        sheet.contains(r#"<c r="C1" t="inlineStr"><is><t>Paid</t></is></c></row>"#),
        "{sheet}"
    );
    assert!(
        sheet.contains(r#"<row r="9"><c r="E9" t="b"><v>1</v></c></row></sheetData>"#),
        "{sheet}"
    );

    let source = Bytes(template(PLAIN, "", false));
    let mut filled = Template::open(&source, Bound::default()).unwrap();
    assert!(
        matches!(filled.set("Nowhere!A1", None), Err(Error::Invalid(m)) if m.contains("`Nowhere`"))
    );
}

#[test]
fn an_xlsm_template_gives_xlsx_without_its_macros() {
    let out = fill(&template(PLAIN, "", true), &[], header(), &[]).unwrap();
    let parts = parts(&out);
    assert!(!parts.keys().any(|name| name.contains("vbaProject")));
    let types = &parts["[Content_Types].xml"];
    assert!(
        !types.contains("vbaProject") && !types.contains("macroEnabled"),
        "{types}"
    );
    assert!(types.contains("spreadsheetml.sheet.main+xml"));
    assert!(!parts["xl/_rels/workbook.xml.rels"].contains("vbaProject"));
}

#[test]
fn two_fills_of_equal_input_are_byte_equal() {
    let once = fill(
        &template(INVOICE, "", false),
        &[],
        header(),
        &[("items", (1..=30).map(item).collect())],
    )
    .unwrap();
    let twice = fill(
        &template(INVOICE, "", false),
        &[],
        header(),
        &[("items", (1..=30).map(item).collect())],
    )
    .unwrap();
    assert_eq!(once, twice);
}
