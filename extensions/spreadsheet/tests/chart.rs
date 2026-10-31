//! Charts of the subset, read back from the written archive
//! (`rule:core-classes/spreadsheet-sheet-end-parts`).

use miniz_oxide::inflate::decompress_to_vec;
use nvs_spreadsheet::chart::{Chart, Kind, Layer, Legend, Series};
use nvs_spreadsheet::tail::Part;
use nvs_spreadsheet::{Cell, Error, Row, SheetOptions, Value, WorkbookOptions, Writer};

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

fn header() -> Row {
    Row {
        cells: vec![text("month"), text("sales"), text("costs")],
        header: true,
        ..Row::default()
    }
}

/// `count` rows of a month, its sales and its costs, the first numbered `from`.
fn months(from: usize, count: usize) -> Vec<Row> {
    (from..from + count)
        .map(|at| Row {
            cells: vec![
                text(&format!("Month {at}")),
                number(at as f64 * 10.0),
                number(at as f64 * 4.0),
            ],
            ..Row::default()
        })
        .collect()
}

fn series(values: &str, categories: Option<&str>) -> Series {
    Series {
        values: values.to_owned(),
        categories: categories.map(str::to_owned),
        ..Series::default()
    }
}

fn chart(kind: Kind, series: Vec<Series>) -> Chart {
    Chart {
        placed: "E2".to_owned(),
        layers: vec![Layer { kind, series }],
        title: None,
        x_title: None,
        y_title: None,
        legend: Legend::Right,
        labels: false,
        width: None,
        height: None,
    }
}

/// A finished workbook: every part by name.
struct Book {
    parts: Vec<(String, String)>,
}

impl Book {
    fn part(&self, name: &str) -> &str {
        self.parts
            .iter()
            .find(|(part, _)| part == name)
            .map(|(_, text)| text.as_str())
            .unwrap_or_else(|| panic!("the archive has {name}"))
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
        let packed = u32_at(at + 20) as usize;
        let name_len = u16_at(at + 28);
        let local = u32_at(at + 42) as usize;
        let name = String::from_utf8(archive[at + 46..at + 46 + name_len].to_vec()).unwrap();
        let data = local + 30 + u16_at(local + 26) + u16_at(local + 28);
        let bytes = decompress_to_vec(&archive[data..data + packed]).unwrap();
        out.push((name, String::from_utf8_lossy(&bytes).into_owned()));
        at += 46 + name_len + u16_at(at + 30) + u16_at(at + 32);
    }
    out
}

/// The archive of a sheet `Sales` with a header and six months, and `charts` added after them.
fn written(charts: Vec<Chart>) -> Vec<u8> {
    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    let mut first = vec![header()];
    first.extend(months(1, 6));
    let mut out = writer.sheet(SheetOptions::named("Sales"), &first).unwrap();
    for chart in charts {
        out.extend(writer.part(Part::Chart(chart)).unwrap());
    }
    out.extend(writer.finish().unwrap().data);
    out
}

fn book(charts: Vec<Chart>) -> Book {
    Book {
        parts: entries(&written(charts)),
    }
}

fn invalid(result: Result<Vec<u8>, Error>) -> String {
    match result {
        Err(Error::Invalid(message)) => message,
        other => panic!("an invalid part, not {other:?}"),
    }
}

#[test]
fn every_chart_type_in_the_subset_is_written() {
    let kinds = [
        (
            Kind::Column,
            "<c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/>",
        ),
        (
            Kind::Bar,
            "<c:barChart><c:barDir val=\"bar\"/><c:grouping val=\"clustered\"/>",
        ),
        (Kind::Line, "<c:lineChart><c:grouping val=\"standard\"/>"),
        (Kind::Area, "<c:areaChart><c:grouping val=\"standard\"/>"),
        (Kind::Pie, "<c:pieChart><c:varyColors val=\"1\"/>"),
        (Kind::Doughnut, "<c:doughnutChart><c:varyColors val=\"1\"/>"),
        (
            Kind::Scatter,
            "<c:scatterChart><c:scatterStyle val=\"lineMarker\"/>",
        ),
    ];
    let charts = kinds
        .iter()
        .map(|&(kind, _)| chart(kind, vec![series("B2:B7", Some("A2:A7"))]))
        .collect();
    let book = book(charts);

    for (at, (kind, opening)) in kinds.iter().enumerate() {
        let part = book.part(&format!("xl/charts/chart{}.xml", at + 1));
        assert!(part.contains(opening), "{kind:?}: {part}");
        let axes = part.contains("<c:valAx>");
        assert_eq!(
            axes,
            !matches!(kind, Kind::Pie | Kind::Doughnut),
            "{kind:?} has axes unless it is round: {part}"
        );
        if *kind == Kind::Scatter {
            assert!(part.contains("<c:xVal><c:numRef><c:f>Sales!$A$2:$A$7</c:f></c:numRef></c:xVal><c:yVal><c:numRef><c:f>Sales!$B$2:$B$7</c:f></c:numRef></c:yVal>"), "{part}");
            assert!(!part.contains("<c:catAx>"), "a scatter has two value axes");
        } else {
            assert!(part.contains("<c:cat><c:strRef><c:f>Sales!$A$2:$A$7</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>Sales!$B$2:$B$7</c:f></c:numRef></c:val>"), "{part}");
        }
    }
    let bar = book.part("xl/charts/chart2.xml");
    assert!(
        bar.contains("<c:axPos val=\"l\"/>") && bar.contains("<c:catAx><c:axId val=\"50010001\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"l\"/>"),
        "a bar chart's categories are on the left: {bar}"
    );

    let drawing = book.part("xl/drawings/drawing1.xml");
    assert_eq!(drawing.matches("<xdr:graphicFrame").count(), 7);
    assert!(drawing.contains(
        "<xdr:twoCellAnchor><xdr:from><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"
    ), "a chart is anchored at its cell: {drawing}");
    assert!(
        drawing.contains("<xdr:row>15</xdr:row><xdr:rowOff>76200</xdr:rowOff></xdr:to>"),
        "a chart is 288 pixels high, 14.4 rows of 20 pixels: {drawing}"
    );
    assert!(drawing.contains("name=\"Chart 7\""));
    let rels = book.part("xl/drawings/_rels/drawing1.xml.rels");
    assert!(rels.contains("Id=\"rId7\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart\" Target=\"../charts/chart7.xml\""), "{rels}");
    let sheet_rels = book.part("xl/worksheets/_rels/sheet1.xml.rels");
    assert!(sheet_rels.contains("Target=\"../drawings/drawing1.xml\""));
    let types = book.part("[Content_Types].xml");
    assert!(types.contains("<Override PartName=\"/xl/charts/chart7.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/>"));
    assert!(types.contains("<Override PartName=\"/xl/drawings/drawing1.xml\""));
}

#[test]
fn a_combination_of_two_types_is_written() {
    let mut combined = chart(Kind::Column, vec![series("B2:B7", Some("A2:A7"))]);
    combined.layers.push(Layer {
        kind: Kind::Line,
        series: vec![series("C2:C7", Some("A2:A7"))],
    });
    let book = book(vec![combined]);
    let part = book.part("xl/charts/chart1.xml");
    let column = part.find("<c:barChart>").expect("the column layer");
    let line = part.find("<c:lineChart>").expect("the line layer");
    let axes = part.find("<c:catAx>").expect("the axes");
    assert!(
        column < line && line < axes,
        "both layers come before the one pair of axes: {part}"
    );
    assert_eq!(part.matches("<c:catAx>").count(), 1);
    assert_eq!(
        part.matches("<c:axId val=\"50010001\"/>").count(),
        3,
        "both layers share the axes"
    );
    assert!(part.contains("<c:lineChart><c:grouping val=\"standard\"/><c:ser><c:idx val=\"1\"/><c:order val=\"1\"/>"), "the second layer's series count on: {part}");

    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    writer
        .sheet(SheetOptions::named("Sales"), &months(1, 3))
        .unwrap();
    for (first, second) in [
        (Kind::Pie, Kind::Line),
        (Kind::Column, Kind::Column),
        (Kind::Bar, Kind::Line),
    ] {
        let mut bad = chart(first, vec![series("B1:B3", None)]);
        bad.layers.push(Layer {
            kind: second,
            series: vec![series("C1:C3", None)],
        });
        let message = invalid(writer.part(Part::Chart(bad)));
        assert!(
            message.contains("`E2`") && message.contains("column, line and area"),
            "{message}"
        );
    }
    let mut three = chart(Kind::Column, vec![series("B1:B3", None)]);
    for kind in [Kind::Line, Kind::Area] {
        three.layers.push(Layer {
            kind,
            series: vec![series("C1:C3", None)],
        });
    }
    assert!(invalid(writer.part(Part::Chart(three))).contains("combines column and line and area"));
    assert!(invalid(writer.part(Part::Chart(chart(Kind::Line, Vec::new())))).contains("no series"));
    assert!(
        invalid(writer.part(Part::Chart(chart(Kind::Line, vec![series("B0:B3", None)]))))
            .contains("`B0:B3`")
    );
}

#[test]
fn titles_axis_titles_legend_colours_and_data_labels_are_written() {
    let mut sales = series("B2:B7", Some("A2:A7"));
    sales.name = Some("Sales & costs".to_owned());
    sales.color = Some("#1f77b4".to_owned());
    let mut column = chart(Kind::Column, vec![sales.clone()]);
    column.title = Some("Sales by month".to_owned());
    column.x_title = Some("Month".to_owned());
    column.y_title = Some("Euro".to_owned());
    column.legend = Legend::Bottom;
    column.labels = true;
    column.width = Some(600.0);
    let mut line = chart(Kind::Line, vec![sales.clone()]);
    line.legend = Legend::Hidden;
    let scatter = chart(Kind::Scatter, vec![sales]);
    let book = book(vec![column, line, scatter]);

    let part = book.part("xl/charts/chart1.xml");
    assert!(part.contains("<c:chart><c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:t>Sales by month</a:t></a:r></a:p></c:rich></c:tx><c:overlay val=\"0\"/></c:title><c:plotArea>"), "{part}");
    assert!(part.contains("<c:axPos val=\"b\"/><c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:t>Month</a:t>"), "the category title is level: {part}");
    assert!(part.contains("<c:majorGridlines/><c:title><c:tx><c:rich><a:bodyPr rot=\"-5400000\" vert=\"horz\"/><a:lstStyle/><a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:t>Euro</a:t>"), "the value title reads upwards: {part}");
    assert!(part.contains("<c:legend><c:legendPos val=\"b\"/><c:layout/></c:legend>"));
    assert!(part.contains("<c:tx><c:v>Sales &amp; costs</c:v></c:tx><c:spPr><a:solidFill><a:srgbClr val=\"1F77B4\"/></a:solidFill></c:spPr><c:dLbls><c:showLegendKey val=\"0\"/><c:showVal val=\"1\"/>"), "{part}");
    let drawing = book.part("xl/drawings/drawing1.xml");
    assert!(
        drawing.contains("<xdr:row>15</xdr:row><xdr:rowOff>76200</xdr:rowOff></xdr:to>"),
        "a width alone keeps the default height: {drawing}"
    );

    let line = book.part("xl/charts/chart2.xml");
    assert!(!line.contains("<c:legend>"), "a hidden legend is left out");
    assert!(!line.contains("<c:dLbls>"));
    assert!(line.contains("<c:spPr><a:ln w=\"28575\" cap=\"rnd\"><a:solidFill><a:srgbClr val=\"1F77B4\"/></a:solidFill><a:round/></a:ln></c:spPr>"), "a line's colour is its line: {line}");
    let scatter = book.part("xl/charts/chart3.xml");
    assert!(scatter.contains("<c:marker><c:symbol val=\"circle\"/><c:size val=\"7\"/><c:spPr><a:solidFill><a:srgbClr val=\"1F77B4\"/></a:solidFill>"), "a scatter's colour is its markers: {scatter}");

    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    writer
        .sheet(SheetOptions::named("Sales"), &months(1, 3))
        .unwrap();
    let mut red = series("B1:B3", None);
    red.color = Some("red".to_owned());
    assert!(invalid(writer.part(Part::Chart(chart(Kind::Column, vec![red])))).contains("#RRGGBB"));
    let mut flat = chart(Kind::Column, vec![series("B1:B3", None)]);
    flat.height = Some(0.0);
    let message = invalid(writer.part(Part::Chart(flat)));
    assert!(
        message.contains("chart at `E2`") && message.contains("height"),
        "{message}"
    );
    let mut nowhere = chart(Kind::Column, vec![series("B1:B3", None)]);
    nowhere.placed = "E0".to_owned();
    assert!(invalid(writer.part(Part::Chart(nowhere))).contains("`E0`"));
}

#[test]
fn a_series_over_a_stream_covers_every_row_the_sheet_ends_with() {
    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    let mut out = writer
        .sheet(SheetOptions::named("Sales"), &[header()])
        .unwrap();
    assert!(
        writer.column(1).is_err(),
        "before the first data row, a column has no cells"
    );
    let mut early = chart(Kind::Line, vec![series("B2:B2", Some("A2:A2"))]);
    early.placed = "E2".to_owned();
    out.extend(writer.part(Part::Chart(early)).unwrap());
    out.extend(writer.rows(&months(1, 10)).unwrap());
    let sales = writer.column(1).unwrap();
    assert_eq!(sales, "B2:B11");
    let mut mid = chart(Kind::Column, vec![series(&sales, None)]);
    mid.layers[0].series.push(series("C2:C5", None));
    mid.layers[0].series.push(series("'Plan'!C2:C11", None));
    out.extend(writer.part(Part::Chart(mid)).unwrap());
    out.extend(writer.rows(&months(11, 20)).unwrap());
    out.extend(writer.finish().unwrap().data);
    let book = Book {
        parts: entries(&out),
    };

    let early = book.part("xl/charts/chart1.xml");
    assert!(
        early.contains("<c:f>Sales!$A$2:$A$31</c:f>")
            && early.contains("<c:f>Sales!$B$2:$B$31</c:f>"),
        "a chart added before the rows covers them all: {early}"
    );
    let mid = book.part("xl/charts/chart2.xml");
    assert!(
        mid.contains("<c:f>Sales!$B$2:$B$31</c:f>"),
        "a column range grows to the last row: {mid}"
    );
    assert!(
        mid.contains("<c:f>Sales!$C$2:$C$5</c:f>"),
        "a range ending elsewhere is kept as given"
    );
    assert!(
        mid.contains("<c:f>Plan!$C$2:$C$11</c:f>"),
        "a range on another sheet is kept as given"
    );
}

#[test]
fn a_sheet_name_in_a_range_is_escaped_as_element_text() {
    let book = book(vec![chart(
        Kind::Pie,
        vec![series("'<x a=\"1\">&amp;'!B2:B3", Some("'Sh''ops'!A2:A3"))],
    )]);
    let part = book.part("xl/charts/chart1.xml");
    assert!(
        part.contains("<c:f>'&lt;x a=\"1\"&gt;&amp;amp;'!$B$2:$B$3</c:f>"),
        "markup in a sheet name is text: {part}"
    );
    assert!(
        part.contains("<c:f>'Sh''ops'!$A$2:$A$3</c:f>"),
        "a quote in a sheet name is doubled: {part}"
    );
    assert!(
        !part.contains("<x a="),
        "no element of the name is written: {part}"
    );
}

#[test]
fn two_writes_of_equal_charts_are_byte_equal() {
    let charts = || {
        let mut combined = chart(Kind::Area, vec![series("B2:B7", Some("A2:A7"))]);
        combined.title = Some("Sales".to_owned());
        combined.labels = true;
        combined.layers.push(Layer {
            kind: Kind::Line,
            series: vec![series("C2:C7", Some("A2:A7"))],
        });
        vec![
            combined,
            chart(Kind::Doughnut, vec![series("B2:B7", Some("A2:A7"))]),
        ]
    };
    assert_eq!(written(charts()), written(charts()));
}

/// The message a pie chart over `values` throws on the sheet `Sales`.
fn refused(values: &str) -> String {
    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    writer
        .sheet(SheetOptions::named("Sales"), &[header()])
        .unwrap();
    invalid(writer.part(Part::Chart(chart(Kind::Pie, vec![series(values, None)]))))
}

#[test]
fn a_series_range_into_another_workbook_or_a_url_throws() {
    for range in [
        "[other.xlsx]Shop!B2:B3",
        "'[other.xlsx]Shop'!B2:B3",
        "https://example.com/book.xlsx!B2:B3",
        "cmd|'/c calc'!A1",
        "SUM(B2:B3)",
        "Sales!$B:$B",
    ] {
        let message = refused(range);
        assert!(
            message.contains(&format!("`{range}`")) || message.contains("The sheet name"),
            "`{range}`: {message}"
        );
    }
}

#[test]
fn a_series_range_sheet_name_follows_the_sheet_name_rule() {
    let long = format!("'{}'!B2:B3", "S".repeat(10_000));
    assert!(refused(&long).contains("at most 31 characters"));
    assert!(refused("'Sh\u{1}ops'!B2:B3").contains("control character"));
    assert!(refused("'Sh\0ops'!B2:B3").contains("control character"));
    assert!(refused("History!B2:B3").contains("`History`"));
    assert!(refused("'Q1/Q2'!B2:B3").contains("`/`"));
}

#[test]
fn a_series_range_with_an_unclosed_quote_or_two_sheets_throws() {
    for range in [
        "'Shops!B2:B3",
        "'Shops'B2:B3",
        "Shops!Shops!B2:B3",
        "'Shops'!'Shops'!B2:B3",
    ] {
        let message = refused(range);
        assert!(
            message.contains(&format!("`{range}`")),
            "`{range}`: {message}"
        );
    }
}

#[test]
fn a_chart_title_past_255_characters_throws() {
    let mut writer = Writer::open(WorkbookOptions::default()).unwrap();
    writer
        .sheet(SheetOptions::named("Sales"), &[header()])
        .unwrap();
    let mut titled = chart(Kind::Column, vec![series("B2:B3", None)]);
    titled.title = Some("t".repeat(255));
    titled.y_title = Some("y".repeat(255));
    assert!(writer.part(Part::Chart(titled.clone())).is_ok());
    titled.title = Some("t".repeat(256));
    assert!(invalid(writer.part(Part::Chart(titled.clone()))).contains("The title"));
    titled.title = None;
    titled.x_title = Some("x".repeat(100_000));
    let message = invalid(writer.part(Part::Chart(titled)));
    assert!(
        message.contains("x-axis title") && message.contains("at most 255"),
        "{message}"
    );
}
