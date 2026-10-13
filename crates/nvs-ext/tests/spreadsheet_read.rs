//! The spreadsheet component's reader (`rule:core-classes/spreadsheet-reads-as-a-stream`), driven
//! through the built-in component as a request would call it: each workbook is made here as zip
//! bytes, lent as the `mixed` source of every call, and read back as typed rows.

#![allow(clippy::unwrap_used, reason = "a test")]

use std::future::Future;
use std::io::Write as _;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::SPREADSHEET_CLASS;
use nvs_ext::call::{Host, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Extension, Loader};

/// `future` polled on this thread until it finishes. A guest call yields at every epoch tick, and
/// polling again resumes it.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}

fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(name, value)| (Key::String(name.to_owned()), value))
            .collect(),
    )
}

fn strings(values: &[&str]) -> Value {
    Value::Array(
        values
            .iter()
            .enumerate()
            .map(|(at, value)| {
                (
                    Key::Int(i64::try_from(at).unwrap()),
                    Value::String((*value).to_owned()),
                )
            })
            .collect(),
    )
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    let Value::Array(pairs) = value else {
        panic!("{value:?} is not a shape");
    };
    pairs
        .iter()
        .find(|(k, _)| *k == Key::String(key.to_owned()))
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("{value:?} has no `{key}`"))
}

fn entries(value: &Value) -> Vec<&Value> {
    let Value::Array(pairs) = value else {
        panic!("{value:?} is not a list");
    };
    pairs.iter().map(|(_, value)| value).collect()
}

/// The spreadsheet component, compiled and checked.
fn spreadsheet(host: &Host) -> Extension {
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    builtins
        .iter()
        .find(|builtin| builtin.manifest().class == SPREADSHEET_CLASS)
        .expect("the spreadsheet component is built in")
        .extension()
        .expect("the spreadsheet component compiles and checks")
        .clone()
}

/// A zip archive of `parts`. A part of 1 MiB or more is stored, so the file is as large as the
/// text the reader parses, and every other part is deflated.
fn zip(parts: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in parts {
        let stored = data.len() >= 1 << 20;
        let packed = if stored {
            data.clone()
        } else {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(data).unwrap();
            encoder.finish().unwrap()
        };
        let method: u16 = if stored { 0 } else { 8 };
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let offset = u32::try_from(out.len()).unwrap();
        let name_len = u16::try_from(name.len()).unwrap();
        let fixed = |sig: u32, out: &mut Vec<u8>| {
            out.extend_from_slice(&sig.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
        };
        let sizes = |out: &mut Vec<u8>| {
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&method.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&crc.sum().to_le_bytes());
            out.extend_from_slice(&u32::try_from(packed.len()).unwrap().to_le_bytes());
            out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
            out.extend_from_slice(&name_len.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
        };
        fixed(0x0403_4b50, &mut out);
        sizes(&mut out);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);
        fixed(0x0201_4b50, &mut central);
        central.extend_from_slice(&20u16.to_le_bytes());
        sizes(&mut central);
        // The comment's length, the disk, the internal and the external attributes.
        central.extend_from_slice(&[0; 10]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let start = u32::try_from(out.len()).unwrap();
    let count = u16::try_from(parts.len()).unwrap();
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&u32::try_from(central.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// One sheet of a test workbook: its name, its `state`, and the XML inside its `<sheetData>`.
struct Sheet<'a> {
    name: &'a str,
    state: Option<&'a str>,
    dimension: Option<&'a str>,
    data: String,
}

fn sheet(name: &str, data: impl Into<String>) -> Sheet<'_> {
    Sheet {
        name,
        state: None,
        dimension: None,
        data: data.into(),
    }
}

/// The `cellXfs` the test workbooks share: 0 is plain, 1 a date, 2 a clock time, 3 a time,
/// 4 elapsed time, and 5 a custom date format.
const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<numFmts count="1"><numFmt numFmtId="164" formatCode="dd/mm/yyyy"/></numFmts>
<cellXfs count="6"><xf numFmtId="0"/><xf numFmtId="14" applyNumberFormat="1"/><xf numFmtId="22"/><xf numFmtId="21"/><xf numFmtId="46"/><xf numFmtId="164"/></cellXfs>
</styleSheet>"#;

/// An xlsx of `sheets` with the shared strings `shared`, laid out as Excel lays one out.
fn xlsx(sheets: &[Sheet<'_>], shared: &[&str]) -> Vec<u8> {
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let mut book =
        format!(r#"<?xml version="1.0"?><workbook xmlns="{main}" xmlns:r="{rel}"><sheets>"#);
    let mut rels = String::from(
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    let mut parts = Vec::new();
    for (at, sheet) in sheets.iter().enumerate() {
        let n = at + 1;
        let state = sheet
            .state
            .map(|state| format!(r#" state="{state}""#))
            .unwrap_or_default();
        book.push_str(&format!(
            r#"<sheet name="{}" sheetId="{n}"{state} r:id="rId{n}"/>"#,
            sheet.name
        ));
        rels.push_str(&format!(
            r#"<Relationship Id="rId{n}" Type="{rel}/worksheet" Target="worksheets/sheet{n}.xml"/>"#
        ));
        let dimension = sheet
            .dimension
            .map(|d| format!(r#"<dimension ref="{d}"/>"#))
            .unwrap_or_default();
        parts.push((
            format!("xl/worksheets/sheet{n}.xml"),
            format!(
                r#"<?xml version="1.0"?><worksheet xmlns="{main}">{dimension}<sheetData>{}</sheetData></worksheet>"#,
                sheet.data
            )
            .into_bytes(),
        ));
    }
    book.push_str("</sheets></workbook>");
    rels.push_str(&format!(
        r#"<Relationship Id="rIdS" Type="{rel}/styles" Target="styles.xml"/><Relationship Id="rIdT" Type="{rel}/sharedStrings" Target="/xl/sharedStrings.xml"/></Relationships>"#
    ));
    let mut sst = format!(r#"<?xml version="1.0"?><sst xmlns="{main}">"#);
    for text in shared {
        sst.push_str(&format!("<si><t>{text}</t></si>"));
    }
    sst.push_str("</sst>");
    let mut all = vec![
        ("[Content_Types].xml".to_owned(), b"<Types/>".to_vec()),
        (
            "_rels/.rels".to_owned(),
            format!(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#).into_bytes(),
        ),
        ("xl/workbook.xml".to_owned(), book.into_bytes()),
        ("xl/_rels/workbook.xml.rels".to_owned(), rels.into_bytes()),
        ("xl/styles.xml".to_owned(), STYLES.as_bytes().to_vec()),
        ("xl/sharedStrings.xml".to_owned(), sst.into_bytes()),
    ];
    all.extend(parts);
    let borrowed: Vec<(&str, Vec<u8>)> = all
        .iter()
        .map(|(name, data)| (name.as_str(), data.clone()))
        .collect();
    zip(&borrowed)
}

/// A request reading one workbook.
struct Read {
    request: Request,
    extension: Extension,
    reader: Value,
    source: Value,
    meter: Arc<Meter>,
    _host: Host,
}

impl Read {
    fn open(bytes: Vec<u8>) -> Self {
        let host = Host::new(2, |_| Ok(())).expect("the host starts");
        let extension = spreadsheet(&host);
        let meter = Arc::new(Meter::new(Duration::from_secs(60), None));
        let request = host.request(meter.clone());
        let source = Value::Bytes(bytes);
        let reader = block_on(request.call_values(&extension, "readerOpen", vec![source.clone()]))
            .unwrap_or_else(|err| panic!("`readerOpen` answers: {err}"))
            .unwrap();
        Self {
            request,
            extension,
            reader,
            source,
            meter,
            _host: host,
        }
    }

    fn call(&self, method: &str, mut args: Vec<Value>) -> Value {
        args.insert(0, self.source.clone());
        args.insert(0, self.reader.clone());
        block_on(self.request.call_values(&self.extension, method, args))
            .unwrap_or_else(|err| panic!("`{method}` answers: {err}"))
            .unwrap_or(Value::Null)
    }

    fn failure(&self, method: &str, mut args: Vec<Value>) -> String {
        args.insert(0, self.source.clone());
        args.insert(0, self.reader.clone());
        match block_on(self.request.call_values(&self.extension, method, args)) {
            Ok(value) => panic!("`{method}` answered {value:?}"),
            Err(err) => err.to_string(),
        }
    }

    /// One batch of rows of `sheet`.
    fn batch(&self, options: Value, max: u64) -> Value {
        self.call("readerRows", vec![options, Value::Uint(max)])
    }

    /// Every row of the read `options` names, each as its number and its cells, a batch of `max`
    /// at a time.
    fn all(&self, options: &Options<'_>, max: u64) -> Vec<(u64, Vec<(u64, Value)>)> {
        let mut out = Vec::new();
        let mut from = options.from;
        loop {
            let batch = self.batch(
                Options {
                    from,
                    ..options.clone()
                }
                .value(),
                max,
            );
            for row in entries(field(&batch, "rows")) {
                let Value::Uint(number) = field(row, "number") else {
                    panic!("a row number is a uint");
                };
                from = Some(number + 1);
                out.push((*number, cells(row)));
            }
            if field(&batch, "done") == &Value::Bool(true) {
                return out;
            }
        }
    }
}

/// The cells of a read row, each as its column and its value.
fn cells(row: &Value) -> Vec<(u64, Value)> {
    entries(field(row, "cells"))
        .into_iter()
        .map(|cell| {
            let Value::Uint(column) = field(cell, "column") else {
                panic!("a column is a uint");
            };
            (*column, field(cell, "value").clone())
        })
        .collect()
}

#[derive(Clone)]
struct Options<'a> {
    sheet: &'a str,
    header_row: Option<u64>,
    from: Option<u64>,
    limit: Option<u64>,
    columns: &'a [&'a str],
    formulas: bool,
    keep_empty_rows: bool,
    skip_hidden: bool,
}

fn options(sheet: &str) -> Options<'_> {
    Options {
        sheet,
        header_row: None,
        from: None,
        limit: None,
        columns: &[],
        formulas: false,
        keep_empty_rows: false,
        skip_hidden: false,
    }
}

impl Options<'_> {
    fn value(&self) -> Value {
        let uint = |n: Option<u64>| n.map_or(Value::Null, Value::Uint);
        shape(vec![
            ("sheet", Value::String(self.sheet.to_owned())),
            ("headerRow", uint(self.header_row)),
            ("from", uint(self.from)),
            ("limit", uint(self.limit)),
            ("columns", strings(self.columns)),
            ("text", strings(&[])),
            ("textAll", Value::Bool(false)),
            ("formulas", Value::Bool(self.formulas)),
            ("keepEmptyRows", Value::Bool(self.keep_empty_rows)),
            ("fillMerged", Value::Bool(false)),
            ("skipHidden", Value::Bool(self.skip_hidden)),
            ("locale", Value::String("en".to_owned())),
        ])
    }
}

fn text(text: &str) -> Value {
    shape(vec![("text", Value::String(text.to_owned()))])
}

fn integer(n: i64) -> Value {
    shape(vec![("integer", Value::Int(n))])
}

fn number(x: f64) -> Value {
    shape(vec![("number", Value::Float(x))])
}

fn core(class: &str, fields: &[(&str, i64)]) -> Value {
    Value::Core {
        class: class.to_owned(),
        fields: fields
            .iter()
            .map(|&(name, value)| (name.to_owned(), Value::Int(value)))
            .collect(),
    }
}

fn date(year: i64, month: i64, day: i64) -> Value {
    core(
        "Core\\Time\\Date",
        &[("year", year), ("month", month), ("day", day)],
    )
}

fn time(hour: i64, minute: i64, second: i64) -> Value {
    core(
        "Core\\Time\\TimeOfDay",
        &[
            ("hour", hour),
            ("minute", minute),
            ("second", second),
            ("nanos", 0),
        ],
    )
}

/// A row of numbered cells `r="A1"` onwards, each `(type, style, inner XML)`.
fn row(number: u64, cells: &[(&str, u32, &str)]) -> String {
    let mut out = format!(r#"<row r="{number}">"#);
    for (at, (kind, style, inner)) in cells.iter().enumerate() {
        let column = char::from(b'A' + u8::try_from(at).unwrap());
        let kind = if kind.is_empty() {
            String::new()
        } else {
            format!(r#" t="{kind}""#)
        };
        out.push_str(&format!(
            r#"<c r="{column}{number}" s="{style}"{kind}>{inner}</c>"#
        ));
    }
    out.push_str("</row>");
    out
}

#[test]
fn an_xlsx_is_read_through_byte_ranges_and_never_whole() {
    // The sheet is stored, so the file is as large as its XML, about 6 MB.
    let mut data = String::new();
    for n in 1..=60_000u64 {
        let x = n.wrapping_mul(2_654_435_761) % 1_000_003;
        data.push_str(&row(
            n,
            &[
                ("", 0, &format!("<v>{x}</v>")),
                ("", 0, &format!("<v>{}</v>", x * 7)),
            ],
        ));
    }
    let bytes = xlsx(&[sheet("Shop", data)], &[]);
    let size = bytes.len() as u64;
    let read = Read::open(bytes);
    let rows = read.all(&options("Shop"), 5_000);
    assert_eq!(rows.len(), 60_000);
    assert_eq!(rows[59_999].0, 60_000);
    let charged = read.meter.charged();
    assert!(
        charged < size,
        "the guest grew to {charged} bytes reading a file of {size}"
    );
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn shared_strings_are_kept_in_one_arena() {
    const COUNT: usize = 200_000;
    let shared: Vec<String> = (0..COUNT).map(|n| format!("{:04x}", n % 0xffff)).collect();
    let shared: Vec<&str> = shared.iter().map(String::as_str).collect();
    let data = row(
        1,
        &[
            ("s", 0, "<v>0</v>"),
            ("s", 0, &format!("<v>{}</v>", COUNT - 1)),
        ],
    );
    let read = Read::open(xlsx(&[sheet("Shop", data)], &shared));
    let before = read.meter.charged();
    let rows = read.all(&options("Shop"), 100);
    assert_eq!(
        rows[0].1,
        vec![
            (1, text("0000")),
            (2, text(&format!("{:04x}", (COUNT - 1) % 0xffff)))
        ]
    );
    // Four bytes of text and a four-byte end per string, with room for each to double: a
    // `String` per string would cost a 24-byte header and an allocation of its own.
    let grown = read.meter.charged() - before;
    assert!(
        grown < (COUNT * 28) as u64,
        "{COUNT} shared strings grew the guest by {grown} bytes"
    );
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn every_cell_reads_as_its_read_table_type() {
    let data = row(
        1,
        &[
            ("s", 0, "<v>1</v>"),
            (
                "inlineStr",
                0,
                "<is><r><t>Tea </t></r><r><t>&amp; cake</t></r></is>",
            ),
            ("str", 0, "<f>A1&amp;B1</f><v>formula text</v>"),
            ("b", 0, "<v>1</v>"),
            ("b", 0, "<v>0</v>"),
            ("", 0, "<v>42</v>"),
            ("", 0, "<v>-3.25</v>"),
            ("", 0, "<v>9007199254740994</v>"),
            ("", 0, "<v>1E+20</v>"),
            ("", 0, ""),
        ],
    );
    let read = Read::open(xlsx(&[sheet("Shop", data)], &["unused", "Coffee_x000D_"]));
    let rows = read.all(&options("Shop"), 10);
    let boolean = |b| shape(vec![("boolean", Value::Bool(b))]);
    assert_eq!(
        rows,
        vec![(
            1,
            vec![
                (1, text("Coffee\r")),
                (2, text("Tea & cake")),
                (3, text("formula text")),
                (4, boolean(true)),
                (5, boolean(false)),
                (6, integer(42)),
                (7, number(-3.25)),
                (8, number(9_007_199_254_740_994.0)),
                (9, number(1e20)),
            ]
        )]
    );
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn a_number_with_a_date_format_reads_as_its_time_type_in_the_zone() {
    // The guest gives a clock time in no zone. The Novis half places it in the reader's `zone`.
    let data = row(
        1,
        &[
            ("", 1, "<v>45306</v>"),
            ("", 2, "<v>45306.75</v>"),
            ("", 3, "<v>0.5208333333333334</v>"),
            ("", 5, "<v>45306</v>"),
            ("", 1, "<v>59</v>"),
            ("", 1, "<v>61</v>"),
        ],
    );
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let rows = read.all(&options("Shop"), 10);
    assert_eq!(
        rows[0].1,
        vec![
            (1, shape(vec![("date", date(2024, 1, 15))])),
            (
                2,
                shape(vec![("day", date(2024, 1, 15)), ("clock", time(18, 0, 0))])
            ),
            (3, shape(vec![("time", time(12, 30, 0))])),
            (4, shape(vec![("date", date(2024, 1, 15))])),
            (5, shape(vec![("date", date(1900, 2, 28))])),
            (6, shape(vec![("date", date(1900, 3, 1))])),
        ]
    );
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn an_elapsed_time_format_reads_as_a_duration() {
    let data = row(1, &[("", 4, "<v>1.5</v>"), ("", 4, "<v>-0.25</v>")]);
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let hours = |h: i64| {
        shape(vec![(
            "duration",
            core("Core\\Time\\Duration", &[("nanos", h * 3_600_000_000_000)]),
        )])
    };
    assert_eq!(
        read.all(&options("Shop"), 10)[0].1,
        vec![(1, hours(36)), (2, hours(-6))]
    );
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn an_error_cell_reads_as_its_cell_error_case() {
    let errors = [
        ("#NULL!", "Null"),
        ("#DIV/0!", "DivZero"),
        ("#VALUE!", "Value"),
        ("#REF!", "Ref"),
        ("#NAME?", "Name"),
        ("#NUM!", "Num"),
        ("#N/A", "NotAvailable"),
        ("#GETTING_DATA", "GettingData"),
        ("#SPILL!", "Spill"),
        ("#CALC!", "Calc"),
    ];
    let inner: Vec<String> = errors
        .iter()
        .map(|(text, _)| format!("<v>{text}</v>"))
        .collect();
    let cells: Vec<(&str, u32, &str)> =
        inner.iter().map(|inner| ("e", 0, inner.as_str())).collect();
    let read = Read::open(xlsx(&[sheet("Shop", row(1, &cells))], &[]));
    let read_back = read.all(&options("Shop"), 10);
    let expected: Vec<(u64, Value)> = errors
        .iter()
        .zip(1..)
        .map(|((_, case), column)| {
            (
                column,
                shape(vec![("error", Value::Case((*case).to_owned()))]),
            )
        })
        .collect();
    assert_eq!(read_back[0].1, expected);
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn formulas_true_gives_the_formula_with_its_stored_result() {
    let data = row(1, &[("", 0, "<v>2</v>"), ("", 0, "<f>A1*21</f><v>42</v>")]);
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let batch = read.batch(
        Options {
            formulas: true,
            ..options("Shop")
        }
        .value(),
        10,
    );
    let row = entries(field(&batch, "rows"))[0];
    let cells = entries(field(row, "cells"));
    assert_eq!(field(cells[0], "formula"), &Value::Null);
    assert_eq!(
        field(cells[1], "formula"),
        &Value::String("A1*21".to_owned())
    );
    assert_eq!(field(cells[1], "value"), &integer(42));
    let plain = read.batch(options("Shop").value(), 10);
    let row = entries(field(&plain, "rows"))[0];
    assert_eq!(
        field(entries(field(row, "cells"))[1], "formula"),
        &Value::Null
    );
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn a_duplicate_header_is_numbered_and_an_empty_one_is_its_column_letter() {
    let data = [
        row(
            2,
            &[
                ("inlineStr", 0, "<is><t>Name</t></is>"),
                ("", 0, ""),
                ("inlineStr", 0, "<is><t>Name</t></is>"),
                ("", 0, "<v>2024</v>"),
                ("inlineStr", 0, "<is><t>Name</t></is>"),
            ],
        ),
        row(3, &[("inlineStr", 0, "<is><t>Tea</t></is>")]),
    ]
    .concat();
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let header = read.call("readerHeader", vec![options("Shop").value()]);
    assert_eq!(
        header,
        strings(&["Name", "B", "Name (2)", "2024", "Name (3)"])
    );
    let named = read.call(
        "readerHeader",
        vec![
            Options {
                header_row: Some(3),
                ..options("Shop")
            }
            .value(),
        ],
    );
    assert_eq!(named, strings(&["Tea"]));
    let rows = read.all(
        &Options {
            header_row: Some(2),
            ..options("Shop")
        },
        10,
    );
    assert_eq!(rows, vec![(3, vec![(1, text("Tea"))])]);
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn the_foreach_key_is_the_row_number_in_the_file() {
    let data = [
        row(1, &[("", 0, "<v>1</v>")]),
        r#"<row r="2"/>"#.to_owned(),
        row(7, &[("", 0, "<v>7</v>")]),
        row(1_048_576, &[("", 0, "<v>9</v>")]),
    ]
    .concat();
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let numbers: Vec<u64> = read
        .all(&options("Shop"), 1)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(numbers, vec![1, 7, 1_048_576]);
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn from_limit_and_columns_select_the_range() {
    let data: String = (1..=10)
        .map(|n| {
            let v = format!("<v>{n}</v>");
            row(
                n,
                &[
                    ("", 0, &v),
                    ("", 0, &v),
                    ("", 0, &v),
                    ("", 0, &v),
                    ("", 0, &v),
                    ("", 0, &v),
                ],
            )
        })
        .collect();
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let batch = read.batch(
        Options {
            from: Some(4),
            limit: Some(3),
            columns: &["A:B", "F"],
            ..options("Shop")
        }
        .value(),
        100,
    );
    let rows: Vec<(u64, Vec<u64>)> = entries(field(&batch, "rows"))
        .into_iter()
        .map(|row| {
            let Value::Uint(n) = field(row, "number") else {
                panic!()
            };
            (
                *n,
                cells(row).into_iter().map(|(column, _)| column).collect(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![(4, vec![1, 2, 6]), (5, vec![1, 2, 6]), (6, vec![1, 2, 6])]
    );
    let bad = read.failure(
        "readerRows",
        vec![
            Options {
                columns: &["1:2"],
                ..options("Shop")
            }
            .value(),
            Value::Uint(1),
        ],
    );
    assert!(bad.contains("`1:2` is not a column"), "{bad}");
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn empty_rows_and_hidden_rows_follow_their_options() {
    let data = [
        row(1, &[("", 0, "<v>1</v>")]),
        r#"<row r="2"><c r="A2" s="0"/></row>"#.to_owned(),
        r#"<row r="4" hidden="1"><c r="A4"><v>4</v></c></row>"#.to_owned(),
        row(5, &[("", 0, "<v>5</v>")]),
    ]
    .concat();
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let numbers = |options: &Options<'_>| -> Vec<u64> {
        read.all(options, 2).into_iter().map(|(n, _)| n).collect()
    };
    assert_eq!(numbers(&options("Shop")), vec![1, 4, 5]);
    assert_eq!(
        numbers(&Options {
            keep_empty_rows: true,
            ..options("Shop")
        }),
        vec![1, 2, 3, 4, 5]
    );
    assert_eq!(
        numbers(&Options {
            skip_hidden: true,
            ..options("Shop")
        }),
        vec![1, 5]
    );
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn sheets_gives_name_position_visibility_size_and_kind() {
    let book = xlsx(
        &[
            Sheet {
                dimension: Some("A1:C10"),
                ..sheet("Shop", row(1, &[("", 0, "<v>1</v>")]))
            },
            Sheet {
                state: Some("hidden"),
                ..sheet("Blog", "")
            },
            Sheet {
                state: Some("veryHidden"),
                dimension: Some("A1"),
                ..sheet("Lists", "")
            },
        ],
        &[],
    );
    let read = Read::open(book);
    let sheets = read.call("readerSheets", vec![]);
    let info: Vec<(String, u64, String, u64, u64, String)> = entries(&sheets)
        .into_iter()
        .map(|sheet| {
            let (
                Value::String(name),
                Value::Uint(position),
                Value::Case(visibility),
                Value::Uint(rows),
                Value::Uint(columns),
                Value::Case(kind),
            ) = (
                field(sheet, "name"),
                field(sheet, "position"),
                field(sheet, "visibility"),
                field(sheet, "rows"),
                field(sheet, "columns"),
                field(sheet, "kind"),
            )
            else {
                panic!("{sheet:?} is not a sheet's shape");
            };
            (
                name.clone(),
                *position,
                visibility.clone(),
                *rows,
                *columns,
                kind.clone(),
            )
        })
        .collect();
    let worksheet = || "Worksheet".to_owned();
    assert_eq!(
        info,
        vec![
            (
                "Shop".to_owned(),
                0,
                "Visible".to_owned(),
                10,
                3,
                worksheet()
            ),
            ("Blog".to_owned(), 1, "Hidden".to_owned(), 0, 0, worksheet()),
            (
                "Lists".to_owned(),
                2,
                "VeryHidden".to_owned(),
                1,
                1,
                worksheet()
            ),
        ]
    );
    let missing = read.failure(
        "readerRows",
        vec![options("Orders").value(), Value::Uint(1)],
    );
    assert!(missing.contains("no sheet named `Orders`"), "{missing}");
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn a_document_type_declaration_is_never_read() {
    let parts = [(
        "_rels/.rels",
        br#"<!DOCTYPE r [<!ENTITY x SYSTEM "file:///etc/passwd">]><Relationships/>"#.to_vec(),
    )];
    let host = Host::new(1, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let err =
        block_on(request.call_values(&extension, "readerOpen", vec![Value::Bytes(zip(&parts))]))
            .expect_err("a DTD is refused");
    assert!(
        err.to_string().contains("document type declaration"),
        "{err}"
    );
    block_on(request.end()).expect("the request ends");
}
