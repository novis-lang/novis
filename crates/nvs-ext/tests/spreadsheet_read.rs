//! The spreadsheet component's reader (`rule:core-classes/spreadsheet-reads-as-a-stream`), driven
//! through the built-in component as a request would call it: each workbook is made here as zip
//! bytes, lent as the `mixed` source of every call, and read back as typed rows.

#![allow(clippy::unwrap_used, reason = "a test")]

use std::future::Future;
use std::io::Write as _;
use std::pin::pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::SPREADSHEET_CLASS;
use nvs_ext::call::{Budget, Host, Lending, Level, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::handle::RANGE_MOST;
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

/// `readerOpen`'s bound with neither half set, which the guest reads as the shipped default.
fn shipped() -> Value {
    shape(vec![("maxBytes", Value::Null), ("maxRatio", Value::Null)])
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
    /// The XML after `</sheetData>`.
    tail: String,
}

fn sheet(name: &str, data: impl Into<String>) -> Sheet<'_> {
    Sheet {
        name,
        state: None,
        dimension: None,
        data: data.into(),
        tail: String::new(),
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
    package(sheets, shared, &[])
}

/// The package [`xlsx`] makes, with the parts `extra` added.
fn package(sheets: &[Sheet<'_>], shared: &[&str], extra: &[(&str, Vec<u8>)]) -> Vec<u8> {
    package_with(sheets, shared, extra, "")
}

/// The package [`package`] makes, with `book_tail` written in the workbook part after its sheets.
fn package_with(
    sheets: &[Sheet<'_>],
    shared: &[&str],
    extra: &[(&str, Vec<u8>)],
    book_tail: &str,
) -> Vec<u8> {
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
                r#"<?xml version="1.0"?><worksheet xmlns="{main}">{dimension}<sheetData>{}</sheetData>{}</worksheet>"#,
                sheet.data, sheet.tail
            )
            .into_bytes(),
        ));
    }
    book.push_str("</sheets>");
    book.push_str(book_tail);
    book.push_str("</workbook>");
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
    all.extend(
        extra
            .iter()
            .map(|(name, data)| ((*name).to_owned(), data.clone())),
    );
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
        let reader = block_on(request.call_values(
            &extension,
            "readerOpen",
            vec![source.clone(), shipped()],
        ))
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
    fill_merged: bool,
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
        fill_merged: false,
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
            ("fillMerged", Value::Bool(self.fill_merged)),
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

/// A budget that lends one source, as the host lends a `bytes` argument or an open file, and
/// keeps the offset and length of every read the guest makes of it.
struct Lends {
    meter: Meter,
    source: Vec<u8>,
    reads: Mutex<Vec<(u64, u64)>>,
}

impl Budget for Lends {
    fn cpu_spent(&self) -> bool {
        self.meter.cpu_spent()
    }

    fn charge(&self, bytes: i64) -> bool {
        self.meter.charge(bytes)
    }

    fn log(&self, level: Level, channel: &str, message: &str) {
        self.meter.log(level, channel, message);
    }

    fn wall_clock(&self) -> Duration {
        self.meter.wall_clock()
    }

    fn monotonic_clock(&self) -> u64 {
        self.meter.monotonic_clock()
    }

    fn random(&self, out: &mut [u8]) {
        self.meter.random(out);
    }

    fn setting(&self, block: &str, key: &str) -> Option<serde_json::Value> {
        self.meter.setting(block, key)
    }

    fn read(&self, id: u64, offset: u64, length: u64) -> Lending {
        assert_eq!(id, 0, "the guest reads the one source it was lent");
        self.reads.lock().unwrap().push((offset, length));
        let start = usize::try_from(offset).unwrap().min(self.source.len());
        let end = start
            .saturating_add(usize::try_from(length).unwrap())
            .min(self.source.len());
        Lending::ready(Ok(self.source[start..end].to_vec()))
    }
}

#[test]
fn a_lent_source_is_read_through_the_budget_a_block_at_a_time() {
    // The sheet is stored, so the file is about 2 MB, more than one block of the host's.
    let mut data = String::new();
    for n in 1..=20_000u64 {
        data.push_str(&row(n, &[("", 0, &format!("<v>{n}</v>"))]));
    }
    let bytes = xlsx(&[sheet("Shop", data)], &[]);
    let size = bytes.len() as u64;
    assert!(size > RANGE_MOST as u64);
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let lends = Arc::new(Lends {
        meter: Meter::new(Duration::from_secs(60), None),
        source: bytes,
        reads: Mutex::new(Vec::new()),
    });
    let request = host.request(lends.clone());
    let source = Value::Lent { id: 0, size };
    let reader =
        block_on(request.call_values(&extension, "readerOpen", vec![source.clone(), shipped()]))
            .unwrap_or_else(|err| panic!("`readerOpen` answers: {err}"))
            .unwrap();
    let mut from = None;
    let mut count = 0u64;
    loop {
        let asked = Options {
            from,
            ..options("Shop")
        };
        let args = vec![
            reader.clone(),
            source.clone(),
            asked.value(),
            Value::Uint(5_000),
        ];
        let batch = block_on(request.call_values(&extension, "readerRows", args))
            .unwrap_or_else(|err| panic!("`readerRows` answers: {err}"))
            .unwrap();
        for read in entries(field(&batch, "rows")) {
            let Value::Uint(number) = field(read, "number") else {
                panic!("a row number is a uint");
            };
            count += 1;
            assert_eq!(*number, count);
            from = Some(number + 1);
        }
        if field(&batch, "done") == &Value::Bool(true) {
            break;
        }
    }
    assert_eq!(count, 20_000);
    let reads = lends.reads.lock().unwrap().clone();
    assert!(reads.len() > 2, "the guest read {} ranges", reads.len());
    assert!(
        reads.iter().all(|(_, length)| *length <= RANGE_MOST as u64),
        "no read is longer than one block: {reads:?}"
    );
    block_on(request.end()).expect("the request ends");
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

// covers: Novis\Spreadsheet\CellError
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
fn a_header_of_every_column_equal_is_numbered_in_linear_time() {
    let mut data = String::from(r#"<row r="1">"#);
    for at in 0..16_384_u32 {
        let mut column = String::new();
        let mut rest = at + 1;
        while rest > 0 {
            column.insert(0, char::from(b'A' + u8::try_from((rest - 1) % 26).unwrap()));
            rest = (rest - 1) / 26;
        }
        let text = if at == 1 { "Name (2)" } else { "Name" };
        data.push_str(&format!(
            r#"<c r="{column}1" t="inlineStr"><is><t>{text}</t></is></c>"#
        ));
    }
    data.push_str("</row>");
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let started = std::time::Instant::now();
    let header = read.call("readerHeader", vec![options("Shop").value()]);
    let elapsed = started.elapsed();
    let mut want = vec!["Name".to_owned(), "Name (2)".to_owned()];
    want.extend((3..=16_384).map(|count| format!("Name ({count})")));
    let want: Vec<&str> = want.iter().map(String::as_str).collect();
    assert_eq!(
        header,
        strings(&want),
        "a name given in the file is skipped"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "a full row of equal headers is read in {elapsed:?}"
    );
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
fn empty_rows_merged_cells_and_hidden_rows_follow_their_options() {
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
    // The empty row 3 is kept though the row after it is hidden and skipped.
    assert_eq!(
        numbers(&Options {
            keep_empty_rows: true,
            skip_hidden: true,
            ..options("Shop")
        }),
        vec![1, 2, 3, 5]
    );
    block_on(read.request.end()).expect("the request ends");

    // `A1:B2` and `C4:C6` are merged, and the rows 2 and 5 are not in the file.
    let data = [
        r#"<row r="1"><c r="A1"><v>1</v></c><c r="C1"><v>3</v></c></row>"#.to_owned(),
        row(3, &[("", 0, "<v>9</v>")]),
        r#"<row r="4"><c r="C4"><v>4</v></c></row>"#.to_owned(),
        row(6, &[("", 0, "<v>6</v>")]),
    ]
    .concat();
    let read = Read::open(xlsx(
        &[Sheet {
            tail: r#"<mergeCells count="2"><mergeCell ref="A1:B2"/><mergeCell ref="C4:C6"/></mergeCells>"#.to_owned(),
            ..sheet("Shop", data)
        }],
        &[],
    ));
    assert_eq!(
        read.call("readerMerged", vec![Value::String("Shop".to_owned())]),
        strings(&["A1:B2", "C4:C6"])
    );
    let one = integer(1);
    let four = integer(4);
    assert_eq!(
        read.all(&options("Shop"), 2),
        vec![
            (1, vec![(1, one.clone()), (3, integer(3))]),
            (3, vec![(1, integer(9))]),
            (4, vec![(3, four.clone())]),
            (6, vec![(1, integer(6))]),
        ]
    );
    let fill = Options {
        fill_merged: true,
        ..options("Shop")
    };
    assert_eq!(
        read.all(&fill, 2),
        vec![
            (1, vec![(1, one.clone()), (2, one.clone()), (3, integer(3))]),
            (2, vec![(1, one.clone()), (2, one.clone())]),
            (3, vec![(1, integer(9))]),
            (4, vec![(3, four.clone())]),
            (5, vec![(3, four.clone())]),
            (6, vec![(1, integer(6)), (3, four.clone())]),
        ]
    );
    // A range whose top-left cell is above `from` still fills the rows below it.
    assert_eq!(
        read.all(
            &Options {
                from: Some(2),
                limit: Some(1),
                ..fill.clone()
            },
            5
        )[0],
        (2, vec![(1, one.clone()), (2, one.clone())])
    );
    // The cells outside the column `B` are dropped after the fill.
    assert_eq!(
        read.all(
            &Options {
                columns: &["B"],
                ..fill.clone()
            },
            5
        ),
        vec![(1, vec![(2, one.clone())]), (2, vec![(2, one)])]
    );
    let bad = read.failure("readerMerged", vec![Value::String("Blog".to_owned())]);
    assert!(bad.contains("no sheet named `Blog`"), "{bad}");
    block_on(read.request.end()).expect("the request ends");

    // xlsb lists its ranges as records, xls as `MERGEDCELLS` and ods as spanned cells.
    let corners: Vec<u8> = [1u32, 2, 0, 1]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    let data = [
        binary_row(1, false),
        binary_cell(2, 0, 0, &((7 << 2) | 2u32).to_le_bytes()),
        record(176, &corners),
    ]
    .concat();
    for (bytes, ranges) in [
        (xlsb(&data, &[]), ["A2:B3"]),
        (xls(), ["A2:B2"]),
        (ods(), ["A4:B5"]),
    ] {
        let read = Read::open(bytes);
        assert_eq!(
            read.call("readerMerged", vec![Value::String("Shop".to_owned())]),
            strings(&ranges)
        );
        block_on(read.request.end()).expect("the request ends");
    }
}

// covers: Novis\Spreadsheet\Reader::Visibility
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

// covers: Novis\Spreadsheet\Reader::SheetKind
#[test]
fn a_chartsheet_is_listed_as_one_and_has_no_rows() {
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let relationships = "http://schemas.openxmlformats.org/package/2006/relationships";
    let book = zip(&[
        ("[Content_Types].xml", b"<Types/>".to_vec()),
        (
            "_rels/.rels",
            format!(r#"<?xml version="1.0"?><Relationships xmlns="{relationships}"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#).into_bytes(),
        ),
        (
            "xl/workbook.xml",
            format!(r#"<?xml version="1.0"?><workbook xmlns="{main}" xmlns:r="{rel}"><sheets><sheet name="Shop" sheetId="1" r:id="rId1"/><sheet name="Sales chart" sheetId="2" r:id="rId2"/></sheets></workbook>"#).into_bytes(),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            format!(r#"<?xml version="1.0"?><Relationships xmlns="{relationships}"><Relationship Id="rId1" Type="{rel}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{rel}/chartsheet" Target="chartsheets/sheet1.xml"/><Relationship Id="rIdS" Type="{rel}/styles" Target="styles.xml"/></Relationships>"#).into_bytes(),
        ),
        ("xl/styles.xml", STYLES.as_bytes().to_vec()),
        (
            "xl/worksheets/sheet1.xml",
            format!(r#"<?xml version="1.0"?><worksheet xmlns="{main}"><sheetData>{}</sheetData></worksheet>"#, row(1, &[("", 0, "<v>1</v>")])).into_bytes(),
        ),
        (
            "xl/chartsheets/sheet1.xml",
            format!(r#"<?xml version="1.0"?><chartsheet xmlns="{main}"/>"#).into_bytes(),
        ),
    ]);
    let read = Read::open(book);
    let sheets = read.call("readerSheets", vec![]);
    let kinds: Vec<(String, String)> = entries(&sheets)
        .into_iter()
        .map(|sheet| {
            let (Value::String(name), Value::Case(kind)) =
                (field(sheet, "name"), field(sheet, "kind"))
            else {
                panic!("{sheet:?} is not a sheet's shape");
            };
            (name.clone(), kind.clone())
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("Shop".to_owned(), "Worksheet".to_owned()),
            ("Sales chart".to_owned(), "Chartsheet".to_owned()),
        ]
    );
    let chart = read.failure(
        "readerRows",
        vec![options("Sales chart").value(), Value::Uint(1)],
    );
    assert!(chart.contains("is a chart. It has no rows"), "{chart}");
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
    let err = block_on(request.call_values(
        &extension,
        "readerOpen",
        vec![Value::Bytes(zip(&parts)), shipped()],
    ))
    .expect_err("a DTD is refused");
    assert!(
        err.to_string().contains("document type declaration"),
        "{err}"
    );
    block_on(request.end()).expect("the request ends");
}

/// One xlsb record: its type and its length, each in 7-bit groups, and then `body`.
fn record(kind: u16, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut kind = kind;
    loop {
        let low = u8::try_from(kind & 0x7F).unwrap();
        kind >>= 7;
        out.push(if kind == 0 { low } else { low | 0x80 });
        if kind == 0 {
            break;
        }
    }
    let mut length = body.len();
    loop {
        let low = u8::try_from(length & 0x7F).unwrap();
        length >>= 7;
        out.push(if length == 0 { low } else { low | 0x80 });
        if length == 0 {
            break;
        }
    }
    out.extend_from_slice(body);
    out
}

/// An xlsb string: its count of UTF-16 units and the units.
fn wide(text: &str) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut out = u32::try_from(units.len()).unwrap().to_le_bytes().to_vec();
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

/// An xlsb cell record of `kind` in `column`, from 0, with `style` and the value bytes `value`.
fn binary_cell(kind: u16, column: u32, style: u32, value: &[u8]) -> Vec<u8> {
    let mut body = column.to_le_bytes().to_vec();
    body.extend_from_slice(&style.to_le_bytes());
    body.extend_from_slice(value);
    record(kind, &body)
}

/// An xlsb row header for the row `number`, from 1.
fn binary_row(number: u32, hidden: bool) -> Vec<u8> {
    let mut body = (number - 1).to_le_bytes().to_vec();
    body.extend_from_slice(&[0; 6]);
    body.push(0);
    body.push(if hidden { 0x10 } else { 0 });
    body.extend_from_slice(&[0; 5]);
    record(0, &body)
}

/// An xlsb with the sheets `Shop`, whose dimension is `A1:F60003` and whose rows are the records
/// `data`, and `Lists`, hidden and empty. Style 1
/// is the custom date format `dd/mm/yyyy`, and the shared strings are `shared`.
fn xlsb(data: &[u8], shared: &[&str]) -> Vec<u8> {
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let package = "http://schemas.openxmlformats.org/package/2006/relationships";
    let mut book = record(153, &[0; 12]);
    for (n, (name, state)) in [("Shop", 0u32), ("Lists", 1)].iter().enumerate() {
        let mut body = state.to_le_bytes().to_vec();
        body.extend_from_slice(&u32::try_from(n + 1).unwrap().to_le_bytes());
        body.extend(wide(&format!("rId{}", n + 1)));
        body.extend(wide(name));
        book.extend(record(156, &body));
    }
    let mut styles = record(
        44,
        &[&164u16.to_le_bytes()[..], &wide("dd/mm/yyyy")].concat(),
    );
    styles.extend(record(617, &[0; 4]));
    for format in [0u16, 164] {
        let mut body = 0u16.to_le_bytes().to_vec();
        body.extend_from_slice(&format.to_le_bytes());
        body.extend_from_slice(&[0; 12]);
        styles.extend(record(47, &body));
    }
    styles.extend(record(618, &[]));
    let mut sst = record(159, &[0; 8]);
    for text in shared {
        sst.extend(record(19, &[&[0u8][..], &wide(text)].concat()));
    }
    // The dimension: the first and last row and the first and last column, each from 0.
    let dimension: Vec<u8> = [0u32, 60_002, 0, 5]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    let mut sheet = record(148, &dimension);
    sheet.extend(record(145, &[]));
    sheet.extend_from_slice(data);
    sheet.extend(record(146, &[]));
    let lists = [record(145, &[]), record(146, &[])].concat();
    zip(&[
        ("[Content_Types].xml", b"<Types/>".to_vec()),
        (
            "_rels/.rels",
            format!(r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.bin"/></Relationships>"#).into_bytes(),
        ),
        ("xl/workbook.bin", book),
        (
            "xl/_rels/workbook.bin.rels",
            format!(r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/worksheet" Target="worksheets/sheet1.bin"/><Relationship Id="rId2" Type="{rel}/worksheet" Target="worksheets/sheet2.bin"/><Relationship Id="rIdS" Type="{rel}/styles" Target="styles.bin"/><Relationship Id="rIdT" Type="{rel}/sharedStrings" Target="sharedStrings.bin"/></Relationships>"#).into_bytes(),
        ),
        ("xl/styles.bin", styles),
        ("xl/sharedStrings.bin", sst),
        ("xl/worksheets/sheet1.bin", sheet),
        ("xl/worksheets/sheet2.bin", lists),
    ])
}

#[test]
fn an_xlsb_is_read_through_byte_ranges() {
    let rk_int = |n: i32| ((n << 2) | 2).to_le_bytes();
    let mut data = binary_row(1, false);
    data.extend(binary_cell(7, 0, 0, &0u32.to_le_bytes()));
    data.extend(binary_cell(6, 1, 0, &wide("Price")));
    data.extend(binary_cell(4, 2, 0, &[1]));
    data.extend(binary_cell(3, 3, 0, &[0x07]));
    data.extend(binary_row(2, false));
    data.extend(binary_cell(2, 0, 0, &rk_int(42)));
    data.extend(binary_cell(2, 1, 0, &((150u32 << 2) | 3).to_le_bytes()));
    data.extend(binary_cell(5, 2, 1, &45_000f64.to_le_bytes()));
    data.extend(binary_cell(
        9,
        3,
        0,
        &[&7f64.to_le_bytes()[..], &[0; 6]].concat(),
    ));
    data.extend(binary_cell(7, 4, 0, &1u32.to_le_bytes()));
    data.extend(binary_cell(1, 5, 0, &[]));
    data.extend(binary_row(3, true));
    data.extend(binary_cell(2, 0, 0, &rk_int(-3)));
    // The sheet is stored, so the file is as large as its records, about 3 MB.
    for n in 4..=60_003u32 {
        data.extend(binary_row(n, false));
        data.extend(binary_cell(2, 0, 0, &rk_int(i32::try_from(n).unwrap())));
        data.extend(binary_cell(5, 1, 0, &(f64::from(n) * 0.5).to_le_bytes()));
    }
    let bytes = xlsb(&data, &["Shop", "Tea &amp; cake"]);
    let size = bytes.len() as u64;
    let read = Read::open(bytes);
    let rows = read.all(&options("Shop"), 5_000);
    assert_eq!(rows.len(), 60_003);
    let boolean = shape(vec![("boolean", Value::Bool(true))]);
    let error = shape(vec![("error", Value::Case("DivZero".to_owned()))]);
    assert_eq!(
        rows[0],
        (
            1,
            vec![
                (1, text("Shop")),
                (2, text("Price")),
                (3, boolean),
                (4, error)
            ]
        )
    );
    assert_eq!(
        rows[1],
        (
            2,
            vec![
                (1, integer(42)),
                (2, number(1.5)),
                (3, shape(vec![("date", date(2023, 3, 15))])),
                (4, integer(7)),
                (5, text("Tea &amp; cake")),
            ]
        )
    );
    assert_eq!(rows[2], (3, vec![(1, integer(-3))]));
    assert_eq!(
        rows[60_002],
        (60_003, vec![(1, integer(60_003)), (2, number(30_001.5))])
    );
    let charged = read.meter.charged();
    assert!(
        charged < size,
        "the guest grew to {charged} bytes reading a file of {size}"
    );
    let shown = read.all(
        &Options {
            skip_hidden: true,
            ..options("Shop")
        },
        5_000,
    );
    assert_eq!(shown.len(), 60_002);
    let sheets = read.call("readerSheets", vec![]);
    let listed: Vec<(&Value, &Value, &Value, &Value)> = entries(&sheets)
        .into_iter()
        .map(|sheet| {
            (
                field(sheet, "name"),
                field(sheet, "visibility"),
                field(sheet, "rows"),
                field(sheet, "columns"),
            )
        })
        .collect();
    assert_eq!(
        listed,
        vec![
            (
                &Value::String("Shop".to_owned()),
                &Value::Case("Visible".to_owned()),
                &Value::Uint(60_003),
                &Value::Uint(6),
            ),
            (
                &Value::String("Lists".to_owned()),
                &Value::Case("Hidden".to_owned()),
                &Value::Uint(0),
                &Value::Uint(0),
            ),
        ]
    );
    block_on(read.request.end()).expect("the request ends");
}

/// A compound file of 512-byte sectors holding `streams`. Each stream is padded with zeros to the
/// 4,096 bytes below which it would be kept in the mini stream.
fn compound(streams: &[(&str, Vec<u8>)]) -> Vec<u8> {
    compound_with(streams, false)
}

/// A compound file of 512-byte sectors holding `streams`, each named by its path: `VBA/dir` is
/// the stream `dir` in the storage `VBA`. With `mini`, a stream under 4,096 bytes is kept in the
/// mini stream, as Excel keeps a VBA project's. Every other stream is padded with zeros to 4,096
/// bytes. A storage's children are chained as right siblings.
fn compound_with(streams: &[(&str, Vec<u8>)], mini: bool) -> Vec<u8> {
    const END: u32 = 0xFFFF_FFFE;
    const NONE: u32 = 0xFFFF_FFFF;
    let small = |data: &Vec<u8>| mini && data.len() < 4096;
    let chain_of = |table: &mut Vec<u32>, count: usize| {
        let start = table.len();
        for n in 0..count {
            table.push(if n + 1 == count {
                END
            } else {
                u32::try_from(start + n + 1).unwrap()
            });
        }
        u32::try_from(start).unwrap()
    };
    let mut mini_table = Vec::new();
    let mut mini_stream = Vec::new();
    let mut mini_starts = Vec::new();
    for (_, data) in streams {
        if small(data) {
            let count = data.len().div_ceil(64).max(1);
            mini_starts.push(chain_of(&mut mini_table, count));
            let mut data = data.clone();
            data.resize(count * 64, 0);
            mini_stream.extend(data);
        } else {
            mini_starts.push(END);
        }
    }
    let mut mini_bytes: Vec<u8> = mini_table.iter().flat_map(|id| id.to_le_bytes()).collect();
    mini_bytes.resize(mini_bytes.len().div_ceil(512) * 512, 0xFF);
    let mini_size = mini_stream.len();
    mini_stream.resize(mini_size.div_ceil(512) * 512, 0);
    let padded: Vec<Vec<u8>> = streams
        .iter()
        .map(|(_, data)| {
            if small(data) {
                return Vec::new();
            }
            let mut data = data.clone();
            data.resize(data.len().max(4096).div_ceil(512) * 512, 0);
            data
        })
        .collect();
    let mut storages: Vec<&str> = Vec::new();
    for (path, _) in streams {
        if let Some((storage, _)) = path.rsplit_once('/') {
            assert!(!storage.contains('/'), "one level of storages");
            if !storages.contains(&storage) {
                storages.push(storage);
            }
        }
    }
    let entries = 1 + storages.len() + streams.len();
    let dir_sectors = entries.div_ceil(4);
    let data_sectors: usize = padded.iter().map(|data| data.len() / 512).sum::<usize>()
        + (mini_bytes.len() + mini_stream.len()) / 512;
    let mut fat_sectors = 1;
    while fat_sectors * 128 < fat_sectors + dir_sectors + data_sectors {
        fat_sectors += 1;
    }
    assert!(
        fat_sectors <= 109,
        "the header lists the whole sector table"
    );
    let mut fat = vec![0xFFFF_FFFDu32; fat_sectors];
    let dir_start = chain_of(&mut fat, dir_sectors);
    let (mini_table_start, mini_start) = if mini_size == 0 {
        (END, END)
    } else {
        (
            chain_of(&mut fat, mini_bytes.len() / 512),
            chain_of(&mut fat, mini_stream.len() / 512),
        )
    };
    let starts: Vec<u32> = padded
        .iter()
        .zip(&mini_starts)
        .map(|(data, mini)| {
            if data.is_empty() {
                *mini
            } else {
                chain_of(&mut fat, data.len() / 512)
            }
        })
        .collect();
    fat.resize(fat_sectors * 128, NONE);
    // Each entry's parent: the root is entry 0, and storage `n` is entry `n + 1`.
    let parent_of = |path: &str| {
        path.rsplit_once('/').map_or(0, |(storage, _)| {
            1 + storages.iter().position(|s| *s == storage).unwrap()
        })
    };
    let parents: Vec<usize> = storages
        .iter()
        .map(|_| 0)
        .chain(streams.iter().map(|(path, _)| parent_of(path)))
        .collect();
    let next_of = |at: usize| {
        (at + 1..parents.len())
            .find(|&later| parents[later] == parents[at])
            .map_or(NONE, |later| u32::try_from(later + 1).unwrap())
    };
    let child_of = |parent: usize| {
        parents
            .iter()
            .position(|&p| p == parent)
            .map_or(NONE, |at| u32::try_from(at + 1).unwrap())
    };
    let entry = |name: &str, kind: u8, start: u32, size: usize, right: u32, child: u32| {
        let mut out = vec![0u8; 128];
        let units: Vec<u16> = name.encode_utf16().chain([0]).collect();
        for (at, unit) in units.iter().enumerate() {
            out[at * 2..at * 2 + 2].copy_from_slice(&unit.to_le_bytes());
        }
        out[64..66].copy_from_slice(&u16::try_from(units.len() * 2).unwrap().to_le_bytes());
        out[66] = kind;
        out[68..72].fill(0xFF);
        out[72..76].copy_from_slice(&right.to_le_bytes());
        out[76..80].copy_from_slice(&child.to_le_bytes());
        out[116..120].copy_from_slice(&start.to_le_bytes());
        out[120..124].copy_from_slice(&u32::try_from(size).unwrap().to_le_bytes());
        out
    };
    let mut directory = entry("Root Entry", 5, mini_start, mini_size, NONE, child_of(0));
    for (at, storage) in storages.iter().enumerate() {
        directory.extend(entry(storage, 1, 0, 0, next_of(at), child_of(at + 1)));
    }
    for (at, ((path, data), start)) in streams.iter().zip(&starts).enumerate() {
        let name = path.rsplit_once('/').map_or(*path, |(_, name)| name);
        let size = if small(data) {
            data.len()
        } else {
            data.len().max(4096)
        };
        let at = storages.len() + at;
        directory.extend(entry(name, 2, *start, size, next_of(at), NONE));
    }
    directory.resize(dir_sectors * 512, 0);
    let mut out = vec![0u8; 512];
    out[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    let fields: [(usize, u32); 7] = [
        (44, u32::try_from(fat_sectors).unwrap()),
        (48, dir_start),
        (56, 4096),
        (60, mini_table_start),
        (64, u32::try_from(mini_bytes.len() / 512).unwrap()),
        (68, END),
        (72, 0),
    ];
    for (at, value) in fields {
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    out[24..26].copy_from_slice(&0x3Eu16.to_le_bytes());
    out[26..28].copy_from_slice(&3u16.to_le_bytes());
    out[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
    out[30..32].copy_from_slice(&9u16.to_le_bytes());
    out[32..34].copy_from_slice(&6u16.to_le_bytes());
    out[76..].fill(0xFF);
    for n in 0..fat_sectors {
        let id = u32::try_from(n).unwrap();
        out[76 + n * 4..80 + n * 4].copy_from_slice(&id.to_le_bytes());
    }
    out.extend(fat.iter().flat_map(|id| id.to_le_bytes()));
    out.extend(directory);
    out.extend(mini_bytes);
    out.extend(mini_stream);
    for data in padded {
        out.extend(data);
    }
    out
}

/// One BIFF8 record: its type, its length and `body`.
fn biff(kind: u16, body: &[u8]) -> Vec<u8> {
    let mut out = kind.to_le_bytes().to_vec();
    out.extend_from_slice(&u16::try_from(body.len()).unwrap().to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// A BIFF8 string with a two-byte count, in two-byte characters.
fn biff_text(text: &str) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut out = u16::try_from(units.len()).unwrap().to_le_bytes().to_vec();
    out.push(1);
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

/// An xls with the sheets `Shop` and `Lists`, hidden and empty. The cell styles are 0 plain,
/// 1 a date, 2 the custom date format `dd/mm/yyyy`, 3 a clock time and 4 elapsed time.
fn xls() -> Vec<u8> {
    let bof = |kind: u16| {
        biff(
            0x0809,
            &[&0x0600u16.to_le_bytes()[..], &kind.to_le_bytes(), &[0; 12]].concat(),
        )
    };
    let mut globals = bof(0x0005);
    globals.extend(biff(
        0x041E,
        &[&164u16.to_le_bytes()[..], &biff_text("dd/mm/yyyy")].concat(),
    ));
    for format in [0u16, 14, 164, 22, 46] {
        globals.extend(biff(
            0x00E0,
            &[&[0u8, 0][..], &format.to_le_bytes(), &[0; 16]].concat(),
        ));
    }
    // `Pastries` starts in one-byte characters and goes on in the `CONTINUE` record in
    // two-byte ones, and `Crème` is wholly inside it.
    let mut sst = [3u32.to_le_bytes(), 3u32.to_le_bytes()].concat();
    sst.extend([6u8, 0, 0]);
    sst.extend(b"Coffee");
    sst.extend([8u8, 0, 0]);
    sst.extend(b"Past");
    globals.extend(biff(0x00FC, &sst));
    let mut more = vec![1u8];
    for unit in "ries".encode_utf16() {
        more.extend(unit.to_le_bytes());
    }
    more.extend(biff_text("Crème"));
    globals.extend(biff(0x003C, &more));
    let rk_int = |n: i32| ((n << 2) | 2).to_le_bytes();
    let cell = |kind: u16, row: u16, column: u16, style: u16, value: &[u8]| {
        biff(
            kind,
            &[
                &row.to_le_bytes()[..],
                &column.to_le_bytes(),
                &style.to_le_bytes(),
                value,
            ]
            .concat(),
        )
    };
    let mut shop = bof(0x0010);
    // The third row is hidden.
    let mut hidden = 2u16.to_le_bytes().to_vec();
    hidden.extend([0; 10]);
    hidden.extend([0x20, 0x01, 0x0F, 0x00]);
    shop.extend(biff(0x0208, &hidden));
    shop.extend(cell(0x00FD, 0, 0, 0, &0u32.to_le_bytes()));
    shop.extend(cell(0x00FD, 0, 1, 0, &1u32.to_le_bytes()));
    shop.extend(cell(0x00FD, 0, 2, 0, &2u32.to_le_bytes()));
    shop.extend(cell(0x0205, 0, 3, 0, &[1, 0]));
    shop.extend(cell(0x0205, 0, 4, 0, &[0x07, 1]));
    shop.extend(cell(0x0203, 1, 0, 0, &2.5f64.to_le_bytes()));
    shop.extend(cell(0x027E, 1, 1, 0, &rk_int(42)));
    let mut several = [1u16.to_le_bytes(), 2u16.to_le_bytes()].concat();
    several.extend(1u16.to_le_bytes());
    several.extend(rk_int(45_000));
    several.extend(0u16.to_le_bytes());
    several.extend(rk_int(7));
    several.extend(3u16.to_le_bytes());
    shop.extend(biff(0x00BD, &several));
    shop.extend(cell(
        0x0006,
        1,
        4,
        0,
        &[&10f64.to_le_bytes()[..], &[0; 8]].concat(),
    ));
    shop.extend(cell(
        0x0006,
        1,
        5,
        0,
        &[&[0u8, 0, 0, 0, 0, 0, 0xFF, 0xFF][..], &[0; 8]].concat(),
    ));
    shop.extend(biff(0x0207, &biff_text("total")));
    shop.extend(cell(0x0203, 2, 0, 3, &45_000.5f64.to_le_bytes()));
    shop.extend(cell(0x0203, 2, 1, 4, &1.5f64.to_le_bytes()));
    shop.extend(cell(0x0204, 2, 2, 2, &biff_text("Tea & cake")));
    // `A2:B2` is merged.
    shop.extend(biff(
        0x00E5,
        &[1u16, 1, 1, 0, 1].map(u16::to_le_bytes).concat(),
    ));
    shop.extend(biff(0x000A, &[]));
    let lists = [bof(0x0010), biff(0x000A, &[])].concat();
    let listed = |at: usize, state: u8, name: &str| {
        let at = u32::try_from(at).unwrap();
        let length = u8::try_from(name.len()).unwrap();
        biff(
            0x0085,
            &[
                &at.to_le_bytes()[..],
                &[state, 0, length, 0],
                name.as_bytes(),
            ]
            .concat(),
        )
    };
    let start = globals.len() + listed(0, 0, "Shop").len() + listed(0, 1, "Lists").len() + 4;
    globals.extend(listed(start, 0, "Shop"));
    globals.extend(listed(start + shop.len(), 1, "Lists"));
    globals.extend(biff(0x000A, &[]));
    globals.extend(shop);
    globals.extend(lists);
    compound(&[("Workbook", globals)])
}

/// An ods of the sheets `Shop` and `Lists`, hidden, with the cells of `content.xml` as
/// LibreOffice writes them, a million empty rows at the end included.
fn ods() -> Vec<u8> {
    let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:calcext="urn:org:documentfoundation:names:experimental:calc:xmlns:calcext:1.0">
<office:automatic-styles><style:style style:name="ta1" style:family="table"><style:table-properties table:display="true"/></style:style><style:style style:name="ta2" style:family="table"><style:table-properties table:display="false"/></style:style></office:automatic-styles>
<office:body><office:spreadsheet>
<table:table table:name="Shop" table:style-name="ta1"><table:table-column table:number-columns-repeated="3"/>
<table:table-row><table:table-cell office:value-type="string" calcext:value-type="string"><text:p>Tea &amp; cake</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="42"><text:p>42</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="2.5"/><table:table-cell office:value-type="boolean" office:boolean-value="true"/><table:table-cell table:formula="of:=1/0" office:value-type="string" office:string-value="" calcext:value-type="error"><text:p>#DIV/0!</text:p></table:table-cell></table:table-row>
<table:table-row table:number-rows-repeated="2"><table:table-cell/></table:table-row>
<table:table-row table:visibility="collapse"><table:table-cell table:number-columns-spanned="2" table:number-rows-spanned="2" office:value-type="date" office:date-value="2024-01-15"/><table:table-cell office:value-type="date" office:date-value="2024-01-15T18:00:00"/><table:table-cell office:value-type="time" office:time-value="PT12H30M00S"/><table:table-cell office:value-type="time" office:time-value="PT36H00M00S"/></table:table-row>
<table:table-row table:number-rows-repeated="2"><table:table-cell table:number-columns-repeated="2" office:value-type="string"><text:p>a</text:p><text:p>b<text:s text:c="2"/>c</text:p></table:table-cell><table:table-cell table:number-columns-repeated="1000"/><table:table-cell office:value-type="percentage" office:value="0.25"/></table:table-row>
<table:table-row table:number-rows-repeated="1048570"><table:table-cell table:number-columns-repeated="1024"/></table:table-row>
</table:table>
<table:table table:name="Lists" table:style-name="ta2"><table:table-row><table:table-cell office:value-type="string"><text:p>x</text:p></table:table-cell></table:table-row></table:table>
</office:spreadsheet></office:body></office:document-content>"#;
    zip(&[
        (
            "mimetype",
            b"application/vnd.oasis.opendocument.spreadsheet".to_vec(),
        ),
        ("META-INF/manifest.xml", b"<manifest:manifest/>".to_vec()),
        ("content.xml", content.as_bytes().to_vec()),
    ])
}

/// Each sheet's name, visibility, rows and columns, as `readerSheets` gives them.
fn listed(read: &Read) -> Vec<(String, String, u64, u64)> {
    entries(&read.call("readerSheets", vec![]))
        .into_iter()
        .map(|sheet| {
            let (
                Value::String(name),
                Value::Case(visibility),
                Value::Uint(rows),
                Value::Uint(columns),
            ) = (
                field(sheet, "name"),
                field(sheet, "visibility"),
                field(sheet, "rows"),
                field(sheet, "columns"),
            )
            else {
                panic!("{sheet:?} is not a sheet's shape");
            };
            (name.clone(), visibility.clone(), *rows, *columns)
        })
        .collect()
}

#[test]
fn xlsm_xls_and_ods_are_read_whole() {
    let boolean = shape(vec![("boolean", Value::Bool(true))]);
    let error = shape(vec![("error", Value::Case("DivZero".to_owned()))]);
    let hours = |h: i64| {
        shape(vec![(
            "duration",
            core("Core\\Time\\Duration", &[("nanos", h * 3_600_000_000_000)]),
        )])
    };
    let clock = |hour| {
        shape(vec![
            ("day", date(2023, 3, 15)),
            ("clock", time(hour, 0, 0)),
        ])
    };

    // An xlsm is an xlsx with a VBA part, and reads as one.
    let vba = compound(&[(
        "PROJECT",
        b"ID=\"{00000000-0000-0000-0000-000000000000}\"".to_vec(),
    )]);
    let xlsm = package(
        &[sheet(
            "Shop",
            row(1, &[("s", 0, "<v>0</v>"), ("", 0, "<v>3</v>")]),
        )],
        &["Coffee"],
        &[("xl/vbaProject.bin", vba)],
    );
    let read = Read::open(xlsm);
    assert_eq!(
        read.all(&options("Shop"), 10),
        vec![(1, vec![(1, text("Coffee")), (2, integer(3))])]
    );
    block_on(read.request.end()).expect("the request ends");

    // An xls: shared strings across a `CONTINUE` record, every cell record, a formula's number
    // and text results, the cell styles' formats and a hidden row.
    let read = Read::open(xls());
    let rows = read.all(&options("Shop"), 2);
    assert_eq!(
        rows,
        vec![
            (
                1,
                vec![
                    (1, text("Coffee")),
                    (2, text("Pastries")),
                    (3, text("Crème")),
                    (4, boolean.clone()),
                    (5, error.clone()),
                ]
            ),
            (
                2,
                vec![
                    (1, number(2.5)),
                    (2, integer(42)),
                    (3, shape(vec![("date", date(2023, 3, 15))])),
                    (4, integer(7)),
                    (5, integer(10)),
                    (6, text("total")),
                ]
            ),
            (
                3,
                vec![(1, clock(12)), (2, hours(36)), (3, text("Tea & cake"))]
            ),
        ]
    );
    let shown = read.all(
        &Options {
            skip_hidden: true,
            ..options("Shop")
        },
        10,
    );
    assert_eq!(shown.len(), 2);
    assert_eq!(
        listed(&read),
        vec![
            ("Shop".to_owned(), "Visible".to_owned(), 3, 6),
            ("Lists".to_owned(), "Hidden".to_owned(), 0, 0),
        ]
    );
    block_on(read.request.end()).expect("the request ends");

    // An xls from Excel 95 or earlier names what it is.
    let old = compound(&[("Book", biff(0x0809, &[0; 16]))]);
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let refused =
        block_on(request.call_values(&extension, "readerOpen", vec![Value::Bytes(old), shipped()]))
            .expect_err("an Excel 95 file is not read");
    assert!(refused.to_string().contains("Excel 95"), "{refused}");
    block_on(request.end()).expect("the request ends");

    // An ods: every value type, repeated rows and cells, a hidden row and a hidden sheet.
    let read = Read::open(ods());
    let both = vec![
        (1, text("a\nb  c")),
        (2, text("a\nb  c")),
        (1003, number(0.25)),
    ];
    assert_eq!(
        read.all(&options("Shop"), 2),
        vec![
            (
                1,
                vec![
                    (1, text("Tea & cake")),
                    (2, integer(42)),
                    (3, number(2.5)),
                    (4, boolean),
                    (5, error),
                ]
            ),
            (
                4,
                vec![
                    (1, shape(vec![("date", date(2024, 1, 15))])),
                    (
                        2,
                        shape(vec![("day", date(2024, 1, 15)), ("clock", time(18, 0, 0))])
                    ),
                    (3, shape(vec![("time", time(12, 30, 0))])),
                    (4, hours(36)),
                ]
            ),
            (5, both.clone()),
            (6, both),
        ]
    );
    let shown = read.all(
        &Options {
            skip_hidden: true,
            ..options("Shop")
        },
        10,
    );
    assert_eq!(shown.len(), 3);
    assert_eq!(
        listed(&read),
        vec![
            ("Shop".to_owned(), "Visible".to_owned(), 6, 1003),
            ("Lists".to_owned(), "Hidden".to_owned(), 1, 1),
        ]
    );
    block_on(read.request.end()).expect("the request ends");
}

/// `data` compressed as MS-OVBA 2.4.1 compresses a VBA project's `dir` stream: each chunk of
/// 4,096 bytes is literal bytes and copies of the longest earlier match in the chunk.
fn ovba(data: &[u8]) -> Vec<u8> {
    let mut out = vec![1u8];
    for chunk in data.chunks(4096) {
        let mut body = Vec::new();
        let mut at = 0;
        while at < chunk.len() {
            let flags = body.len();
            body.push(0u8);
            for bit in 0..8 {
                if at >= chunk.len() {
                    break;
                }
                let bits = if at == 0 {
                    4
                } else {
                    (usize::BITS - (at - 1).leading_zeros()).clamp(4, 12)
                };
                let longest = (0xFFFFusize >> bits) + 3;
                let farthest = 1usize << (16 - bits);
                let mut best = (0, 0);
                for offset in 1..=at.min(farthest) {
                    let mut length = 0;
                    while length < longest
                        && at + length < chunk.len()
                        && chunk[at + length - offset] == chunk[at + length]
                    {
                        length += 1;
                    }
                    if length > best.1 {
                        best = (offset, length);
                    }
                }
                if best.1 >= 3 {
                    let token = ((best.0 - 1) << (16 - bits)) | (best.1 - 3);
                    body.extend(u16::try_from(token).unwrap().to_le_bytes());
                    body[flags] |= 1 << bit;
                    at += best.1;
                } else {
                    body.push(chunk[at]);
                    at += 1;
                }
            }
        }
        assert!(body.len() <= 4096, "the chunk compresses");
        let header = 0xB000 | u16::try_from(body.len() - 1).unwrap();
        out.extend(header.to_le_bytes());
        out.extend(body);
    }
    out
}

/// One record of a VBA project's `dir` stream: its type, its length and `body`.
fn dir_record(kind: u16, body: &[u8]) -> Vec<u8> {
    let mut out = kind.to_le_bytes().to_vec();
    out.extend(u32::try_from(body.len()).unwrap().to_le_bytes());
    out.extend(body);
    out
}

/// A VBA project whose `dir` stream lists `modules`, each its single-byte name and its UTF-16
/// name. Each module's own stream is bytes that are not VBA, so reading one would fail.
fn vba_project(modules: &[(&[u8], &str)]) -> Vec<u8> {
    let mut dir = [
        dir_record(0x0001, &3u32.to_le_bytes()),
        // The version record's length field is 4, and six bytes follow it.
        [
            &0x0009u16.to_le_bytes()[..],
            &4u32.to_le_bytes(),
            &[0x5E, 0, 0, 0, 3, 0],
        ]
        .concat(),
        dir_record(0x0003, &1252u16.to_le_bytes()),
        dir_record(0x0004, b"VBAProject"),
        dir_record(0x000F, &u16::try_from(modules.len()).unwrap().to_le_bytes()),
        dir_record(0x0013, &0xFFFFu16.to_le_bytes()),
    ]
    .concat();
    let mut streams = vec![(
        "PROJECT".to_owned(),
        b"ID=\"{00000000-0000-0000-0000-000000000000}\"".to_vec(),
    )];
    for (name, unicode) in modules {
        let wide: Vec<u8> = unicode.encode_utf16().flat_map(u16::to_le_bytes).collect();
        dir.extend(dir_record(0x0019, name));
        dir.extend(dir_record(0x0047, &wide));
        dir.extend(dir_record(0x001A, name));
        dir.extend(dir_record(0x0032, &wide));
        dir.extend(dir_record(0x0031, &0u32.to_le_bytes()));
        dir.extend(dir_record(0x0021, &[]));
        dir.extend(dir_record(0x002B, &[]));
        streams.push((format!("VBA/{unicode}"), b"MsgBox \"never run\"".to_vec()));
    }
    dir.extend(dir_record(0x0010, &[]));
    streams.push(("VBA/dir".to_owned(), ovba(&dir)));
    let streams: Vec<(&str, Vec<u8>)> = streams
        .iter()
        .map(|(path, data)| (path.as_str(), data.clone()))
        .collect();
    compound_with(&streams, true)
}

#[test]
fn xlsm_macros_are_listed_and_never_run() {
    let shop = || sheet("Shop", row(1, &[("s", 0, "<v>0</v>"), ("", 0, "<v>3</v>")]));
    // The `dir` stream and every module are in the mini stream, as Excel keeps them.
    let vba = vba_project(&[
        (b"ThisWorkbook", "ThisWorkbook"),
        (b"Module1", "Module1"),
        (b"Module2", "Module2"),
        (b"Mod\xFCl3", "Modül3"),
    ]);
    let read = Read::open(package(
        &[shop()],
        &["Coffee"],
        &[("xl/vbaProject.bin", vba)],
    ));
    assert_eq!(
        read.call("readerMacros", vec![]),
        strings(&["ThisWorkbook", "Module1", "Module2", "Modül3"])
    );
    assert_eq!(
        read.all(&options("Shop"), 10),
        vec![(1, vec![(1, text("Coffee")), (2, integer(3))])]
    );
    block_on(read.request.end()).expect("the request ends");

    // A workbook with no VBA project has no macros.
    let read = Read::open(xlsx(&[shop()], &["Coffee"]));
    assert_eq!(read.call("readerMacros", vec![]), strings(&[]));
    block_on(read.request.end()).expect("the request ends");

    // A `dir` stream that is not compressed as MS-OVBA throws, naming the VBA project.
    let broken = compound_with(&[("VBA/dir", b"\x00 not compressed".to_vec())], true);
    let read = Read::open(package(
        &[shop()],
        &["Coffee"],
        &[("xl/vbaProject.bin", broken)],
    ));
    let refused = read.failure("readerMacros", vec![]);
    assert!(refused.contains("VBA project"), "{refused}");
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn the_header_names_every_key_and_a_missing_cell_is_null() {
    // The header has an empty cell in B and nothing in D, so B and D are named by their letters.
    let data = [
        row(1, &[("inlineStr", 0, "<is><t>Item</t></is>")]),
        r#"<row r="2"><c r="A2" t="inlineStr"><is><t>Name</t></is></c><c r="B2"/><c r="C2" t="inlineStr"><is><t>Price</t></is></c><c r="E2" t="inlineStr"><is><t>Notes</t></is></c></row>"#.to_owned(),
        r#"<row r="3"><c r="A3" t="inlineStr"><is><t>Tea</t></is></c><c r="F3"><v>9</v></c></row>"#.to_owned(),
    ]
    .concat();
    let read = Read::open(xlsx(&[sheet("Shop", data)], &[]));
    let header = Options {
        header_row: Some(2),
        ..options("Shop")
    };
    assert_eq!(
        read.call("readerHeader", vec![header.value()]),
        strings(&["Name", "B", "Price", "D", "Notes"])
    );
    // The row gives only the cells it has. The Novis half gives every header key, with `null`
    // for a key whose cell is missing, and names column F by its letter.
    assert_eq!(
        read.all(&header, 10),
        vec![(3, vec![(1, text("Tea")), (6, integer(9))])]
    );
    // With no header row given, the header is the first row with a value in it.
    assert_eq!(
        read.call("readerHeader", vec![options("Shop").value()]),
        strings(&["Item"])
    );
    block_on(read.request.end()).expect("the request ends");
}

fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

/// A relationships part of `(id, type, target, external)` entries.
fn rels(entries: &[(&str, &str, &str, bool)]) -> Vec<u8> {
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let mut out = String::from(
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (id, kind, target, external) in entries {
        let mode = if *external {
            r#" TargetMode="External""#
        } else {
            ""
        };
        out.push_str(&format!(
            r#"<Relationship Id="{id}" Type="{rel}/{kind}" Target="{target}"{mode}/>"#
        ));
    }
    out.push_str("</Relationships>");
    out.into_bytes()
}

/// One picture of a drawing part: its anchor element, the `(column, row)` it starts at, and its
/// blip's attributes.
type Anchor<'a> = (&'a str, Option<(u32, u32)>, &'a str);

/// A drawing part with one anchor per [`Anchor`]; an `absoluteAnchor` has no `from`.
fn drawing(anchors: &[Anchor<'_>]) -> Vec<u8> {
    let mut out = String::from(
        r#"<?xml version="1.0"?><xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    );
    for (anchor, from, blip) in anchors {
        out.push_str(&format!("<xdr:{anchor}>"));
        if let Some((column, row)) = from {
            out.push_str(&format!(
                "<xdr:from><xdr:col>{column}</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>{row}</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"
            ));
        }
        out.push_str(&format!(
            r#"<xdr:pic><xdr:blipFill><a:blip {blip}/></xdr:blipFill></xdr:pic><xdr:clientData/></xdr:{anchor}>"#
        ));
    }
    out.push_str("</xdr:wsDr>");
    out.into_bytes()
}

/// The bytes of a PNG file's signature and the start of its header: what an image part holds.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR";

#[test]
fn the_extras_read_on_request() {
    let shop = Sheet {
        tail: r#"<hyperlinks><hyperlink ref="A1" r:id="rIdL" display="Shop site" tooltip="Open the shop"/><hyperlink ref="B1:B2" location="'Blog'!A1"/></hyperlinks><drawing r:id="rIdD"/>"#.to_owned(),
        ..sheet("Shop", row(1, &[("s", 0, "<v>0</v>"), ("", 0, "<v>3</v>")]))
    };
    let blog = sheet("Blog", row(1, &[("", 0, "<v>1</v>")]));
    let comments = r#"<?xml version="1.0"?><comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><authors><author>Ann</author><author>Bo &amp; Co</author></authors><commentList><comment ref="A1" authorId="1"><text><r><t>Bo:</t></r><r><t xml:space="preserve"> check the price</t></r></text></comment><comment ref="B2" authorId="0"><text><t>Plain</t></text></comment></commentList></comments>"#;
    let table = r#"<?xml version="1.0"?><table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" id="1" name="Table1" displayName="Prices" ref="A1:B3"><autoFilter ref="A1:B3"/></table>"#;
    let core = r#"<?xml version="1.0"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>Prices</dc:title><dc:subject>Coffee &amp; tea</dc:subject><dc:creator>Shop</dc:creator><cp:keywords>prices, shop</cp:keywords><dcterms:created xsi:type="dcterms:W3CDTF">2024-03-05T15:30:09+01:00</dcterms:created></cp:coreProperties>"#;
    let app = r#"<?xml version="1.0"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Microsoft Excel</Application><Company>Example</Company></Properties>"#;
    let names = r#"<definedNames><definedName name="Prices">Shop!$A$1:$B$3</definedName><definedName name="_xlnm.Print_Area" localSheetId="1">Blog!$A$1:$C$9</definedName></definedNames>"#;
    let read = Read::open(package_with(
        &[shop, blog],
        &["Coffee"],
        &[
            (
                "xl/worksheets/_rels/sheet1.xml.rels",
                rels(&[
                    ("rIdL", "hyperlink", "https://example.com/shop", true),
                    ("rIdC", "comments", "../comments1.xml", false),
                    ("rIdT", "table", "../tables/table1.xml", false),
                    ("rIdD", "drawing", "../drawings/drawing1.xml", false),
                ]),
            ),
            ("xl/comments1.xml", comments.as_bytes().to_vec()),
            ("xl/tables/table1.xml", table.as_bytes().to_vec()),
            (
                "xl/drawings/drawing1.xml",
                drawing(&[
                    ("twoCellAnchor", Some((2, 4)), r#"r:embed="rIdP""#),
                    ("absoluteAnchor", None, r#"r:embed="rIdP""#),
                ]),
            ),
            (
                "xl/drawings/_rels/drawing1.xml.rels",
                rels(&[("rIdP", "image", "../media/image1.png", false)]),
            ),
            ("xl/media/image1.png", PNG.to_vec()),
            ("docProps/core.xml", core.as_bytes().to_vec()),
            ("docProps/app.xml", app.as_bytes().to_vec()),
        ],
        names,
    ));

    let links = read.call("readerLinks", vec![string("Shop")]);
    let links = entries(&links);
    assert_eq!(links.len(), 2);
    assert_eq!(field(links[0], "anchor"), &string("A1"));
    assert_eq!(
        field(links[0], "target"),
        &string("https://example.com/shop")
    );
    assert_eq!(field(links[0], "text"), &string("Shop site"));
    assert_eq!(field(links[0], "tooltip"), &string("Open the shop"));
    assert_eq!(field(links[0], "external"), &Value::Bool(true));
    // A link to a place in the workbook is its location, and is not external.
    assert_eq!(field(links[1], "anchor"), &string("B1:B2"));
    assert_eq!(field(links[1], "target"), &string("'Blog'!A1"));
    assert_eq!(field(links[1], "text"), &Value::Null);
    assert_eq!(field(links[1], "external"), &Value::Bool(false));

    // A note's text is every run of it, and its author is named by the authors list.
    let notes = read.call("readerNotes", vec![string("Shop")]);
    let notes = entries(&notes);
    assert_eq!(notes.len(), 2);
    assert_eq!(field(notes[0], "noted"), &string("A1"));
    assert_eq!(field(notes[0], "note"), &string("Bo: check the price"));
    assert_eq!(field(notes[0], "author"), &string("Bo & Co"));
    assert_eq!(field(notes[1], "noted"), &string("B2"));
    assert_eq!(field(notes[1], "author"), &string("Ann"));

    let names = read.call("readerNames", vec![]);
    let names = entries(&names);
    assert_eq!(names.len(), 2);
    assert_eq!(field(names[0], "name"), &string("Prices"));
    assert_eq!(field(names[0], "reference"), &string("Shop!$A$1:$B$3"));
    assert_eq!(field(names[0], "sheet"), &Value::Null);
    assert_eq!(field(names[1], "name"), &string("_xlnm.Print_Area"));
    assert_eq!(field(names[1], "sheet"), &string("Blog"));

    let tables = read.call("readerTables", vec![string("Shop")]);
    let tables = entries(&tables);
    assert_eq!(tables.len(), 1);
    assert_eq!(field(tables[0], "table"), &string("A1:B3"));
    assert_eq!(field(tables[0], "name"), &string("Prices"));
    assert_eq!(field(tables[0], "sheet"), &string("Shop"));

    // The picture is at the cell its anchor starts in, C5, and one anchored to no cell is not
    // listed.
    let images = read.call("readerImages", vec![string("Shop")]);
    let images = entries(&images);
    assert_eq!(images.len(), 1);
    assert_eq!(field(images[0], "at"), &string("C5"));
    assert_eq!(field(images[0], "image"), &Value::Bytes(PNG.to_vec()));
    assert_eq!(field(images[0], "format"), &string("png"));
    assert_eq!(field(images[0], "link"), &Value::Null);

    // The creation time is in UTC.
    let properties = read.call("readerProperties", vec![]);
    assert_eq!(field(&properties, "title"), &string("Prices"));
    assert_eq!(field(&properties, "subject"), &string("Coffee & tea"));
    assert_eq!(field(&properties, "author"), &string("Shop"));
    assert_eq!(field(&properties, "company"), &string("Example"));
    assert_eq!(field(&properties, "keywords"), &string("prices, shop"));
    let created = field(&properties, "created");
    assert_eq!(field(created, "day"), &date(2024, 3, 5));
    assert_eq!(field(created, "clock"), &time(14, 30, 9));

    // A sheet with none of them gives empty lists, and a sheet the workbook lacks throws.
    for method in ["readerLinks", "readerNotes", "readerTables", "readerImages"] {
        assert_eq!(
            read.call(method, vec![string("Blog")]),
            strings(&[]),
            "{method}"
        );
    }
    let missing = read.failure("readerLinks", vec![string("Prices")]);
    assert!(missing.contains("no sheet named `Prices`"), "{missing}");
    block_on(read.request.end()).expect("the request ends");

    // A workbook with no properties part gives `null` for each.
    let read = Read::open(xlsx(&[sheet("Shop", String::new())], &[]));
    let properties = read.call("readerProperties", vec![]);
    assert_eq!(field(&properties, "title"), &Value::Null);
    assert_eq!(field(&properties, "created"), &Value::Null);
    assert_eq!(read.call("readerNames", vec![]), strings(&[]));
    block_on(read.request.end()).expect("the request ends");

    // An ods file throws naming its format, so an empty list always means the file has none.
    let read = Read::open(ods());
    let refused = read.failure("readerNotes", vec![string("Shop")]);
    assert!(refused.contains("an ods file"), "{refused}");
    block_on(read.request.end()).expect("the request ends");
}

#[test]
fn an_external_link_or_connection_reads_as_data_and_is_never_followed() {
    // Every target names a file that exists on this machine. Following one would put its bytes
    // in the result.
    let manifest = nvs_repo::path("Cargo.toml");
    let file = format!(
        "file:///{}",
        manifest.display().to_string().replace('\\', "/")
    );
    let file = file.as_str();
    let shop = Sheet {
        tail: r#"<hyperlinks><hyperlink ref="A1" r:id="rIdL"/></hyperlinks><drawing r:id="rIdD"/>"#
            .to_owned(),
        ..sheet(
            "Shop",
            r#"<row r="1"><c r="A1"><f>[1]Prices!A1</f><v>42</v></c></row>"#,
        )
    };
    let link = r#"<?xml version="1.0"?><externalLink xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><externalBook r:id="rId1"><sheetNames><sheetName val="Prices"/></sheetNames></externalBook></externalLink>"#;
    let connections = format!(
        r#"<?xml version="1.0"?><connections xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><connection id="1" name="Shop" type="4" refreshOnLoad="1"><webPr url="{file}"/></connection></connections>"#
    );
    let read = Read::open(package_with(
        &[shop],
        &[],
        &[
            (
                "xl/worksheets/_rels/sheet1.xml.rels",
                rels(&[
                    ("rIdL", "hyperlink", file, true),
                    ("rIdD", "drawing", "../drawings/drawing1.xml", false),
                ]),
            ),
            (
                "xl/drawings/drawing1.xml",
                drawing(&[("oneCellAnchor", Some((0, 0)), r#"r:link="rIdP""#)]),
            ),
            (
                "xl/drawings/_rels/drawing1.xml.rels",
                rels(&[("rIdP", "image", file, true)]),
            ),
            (
                "xl/externalLinks/externalLink1.xml",
                link.as_bytes().to_vec(),
            ),
            (
                "xl/externalLinks/_rels/externalLink1.xml.rels",
                rels(&[("rId1", "externalLinkPath", file, true)]),
            ),
            ("xl/connections.xml", connections.into_bytes()),
        ],
        r#"<definedNames><definedName name="Remote">[1]Prices!$A$1</definedName></definedNames>"#,
    ));

    // The hyperlink is its target's text.
    let links = read.call("readerLinks", vec![string("Shop")]);
    let links = entries(&links);
    assert_eq!(field(links[0], "target"), &string(file));
    assert_eq!(field(links[0], "external"), &Value::Bool(true));

    // The linked picture is its target's text, with no bytes.
    let images = read.call("readerImages", vec![string("Shop")]);
    let images = entries(&images);
    assert_eq!(images.len(), 1);
    assert_eq!(field(images[0], "at"), &string("A1"));
    assert_eq!(field(images[0], "image"), &Value::Bytes(Vec::new()));
    assert_eq!(field(images[0], "link"), &string(file));

    // A formula over another workbook reads as its stored result, and its text is data.
    let rows = read.all(
        &Options {
            formulas: true,
            ..options("Shop")
        },
        10,
    );
    assert_eq!(rows, vec![(1, vec![(1, integer(42))])]);
    let batch = read.batch(
        Options {
            formulas: true,
            ..options("Shop")
        }
        .value(),
        10,
    );
    let row = entries(field(&batch, "rows"))[0];
    assert_eq!(
        field(entries(field(row, "cells"))[0], "formula"),
        &string("[1]Prices!A1")
    );

    // A defined name over another workbook is its reference's text.
    let names = read.call("readerNames", vec![]);
    assert_eq!(
        field(entries(&names)[0], "reference"),
        &string("[1]Prices!$A$1")
    );
    block_on(read.request.end()).expect("the request ends");
}
