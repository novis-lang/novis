//! The spreadsheet component as `nvs-ext` sees it (`rule:packaging/the-first-party-components-are-built-in`,
//! `rule:core-classes/spreadsheet-writes-as-a-stream`, `rule:core-classes/spreadsheet-output-is-inert`):
//! `Novis\Spreadsheet\Engine` is in the binary and writes a workbook with no `[[extension]]`
//! entry and no grant. The chunks every call returns are one valid zip archive whose sheet has
//! its cells inline, two writes of equal input are byte-equal, and a writer the request ends
//! before `writerFinish` logs an error naming its target.

use std::future::Future;
use std::io::Read as _;
use std::pin::pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::SPREADSHEET_CLASS;
use nvs_ext::call::{Budget, Host, Level, Meter, Request};
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

/// A [`Meter`] that keeps every message written to its log.
struct Logged {
    meter: Meter,
    log: Mutex<Vec<(Level, String)>>,
}

impl Budget for Logged {
    fn cpu_spent(&self) -> bool {
        self.meter.cpu_spent()
    }

    fn charge(&self, bytes: i64) -> bool {
        self.meter.charge(bytes)
    }

    fn log(&self, level: Level, _channel: &str, message: &str) {
        self.log
            .lock()
            .expect("the log locks")
            .push((level, message.to_owned()));
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
}

fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(name, value)| (Key::String(name.to_owned()), value))
            .collect(),
    )
}

fn list(values: Vec<Value>) -> Value {
    Value::Array(
        values
            .into_iter()
            .enumerate()
            .map(|(at, value)| (Key::Int(i64::try_from(at).unwrap()), value))
            .collect(),
    )
}

/// A cell holding the `cell-value` case `case` with `value`.
fn cell(case: &str, value: Value) -> Value {
    shape(vec![
        ("value", shape(vec![(case, value)])),
        ("style", Value::Null),
    ])
}

fn row(cells: Vec<Value>) -> Value {
    shape(vec![
        ("cells", list(cells)),
        ("style", Value::Null),
        ("height", Value::Null),
        ("hidden", Value::Bool(false)),
        ("level", Value::Uint(0)),
    ])
}

fn options(target: &str) -> Value {
    shape(vec![
        (
            "properties",
            shape(vec![
                ("title", Value::Null),
                ("subject", Value::Null),
                ("author", Value::Null),
                ("company", Value::Null),
                ("keywords", Value::Null),
                ("created", Value::Null),
            ]),
        ),
        ("target", Value::String(target.to_owned())),
    ])
}

fn sheet(name: &str) -> Value {
    shape(vec![
        ("name", Value::String(name.to_owned())),
        ("position", Value::Null),
        ("visibility", Value::Case("Visible".to_owned())),
        ("columns", list(vec![])),
        ("freezeRows", Value::Uint(1)),
        ("freezeColumns", Value::Uint(0)),
        ("autofilter", Value::Null),
        ("tabColor", Value::Null),
        ("zoom", Value::Null),
        ("gridlines", Value::Bool(true)),
        ("rightToLeft", Value::Bool(false)),
    ])
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

fn call(request: &Request, extension: &Extension, method: &str, args: Vec<Value>) -> Value {
    block_on(request.call_values(extension, method, args))
        .unwrap_or_else(|err| panic!("`{method}` answers: {err}"))
        .unwrap_or(Value::Null)
}

fn bytes(value: Value) -> Vec<u8> {
    match value {
        Value::Bytes(bytes) => bytes,
        other => panic!("a write returns bytes, not {other:?}"),
    }
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

/// Writes a two-sheet workbook and returns every chunk, the last one from `writerFinish`, and
/// what `writerFinish` returned.
fn write(request: &Request, extension: &Extension) -> (Vec<Vec<u8>>, Value) {
    let writer = call(
        request,
        extension,
        "writerOpen",
        vec![options("report.xlsx")],
    );
    let header = row(vec![
        cell("text", Value::String("Name".to_owned())),
        cell("text", Value::String("Price".to_owned())),
        cell("text", Value::String("Since".to_owned())),
    ]);
    let mut chunks = vec![bytes(call(
        request,
        extension,
        "writerSheet",
        vec![writer.clone(), sheet("Shop"), list(vec![header])],
    ))];
    let rows = (0..200)
        .map(|at| {
            row(vec![
                cell("text", Value::String(format!("Tea & <Coffee> {at}"))),
                cell("number", Value::Float(f64::from(at) + 0.5)),
                cell(
                    "date",
                    Value::Core {
                        class: "Core\\Time\\Date".to_owned(),
                        fields: vec![
                            ("year".to_owned(), Value::Int(2024)),
                            ("month".to_owned(), Value::Int(1)),
                            ("day".to_owned(), Value::Int(15)),
                        ],
                    },
                ),
            ])
        })
        .collect();
    chunks.push(bytes(call(
        request,
        extension,
        "writerRows",
        vec![writer.clone(), list(rows)],
    )));
    chunks.push(bytes(call(
        request,
        extension,
        "writerSheet",
        vec![
            writer.clone(),
            sheet("Blog"),
            list(vec![row(vec![cell("boolean", Value::Bool(true))])]),
        ],
    )));
    let finished = call(request, extension, "writerFinish", vec![writer]);
    let Value::Bytes(last) = field(&finished, "data").clone() else {
        panic!("`data` is bytes");
    };
    chunks.push(last);
    (chunks, finished)
}

/// The entries of the zip archive `archive`, each name with its inflated content, read through
/// the central directory and checked against each entry's CRC-32.
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
        let crc = u32_at(at + 16);
        let packed = u32_at(at + 20) as usize;
        let name_len = u16_at(at + 28);
        let local = u32_at(at + 42) as usize;
        let name = String::from_utf8(archive[at + 46..at + 46 + name_len].to_vec()).unwrap();
        assert_eq!(u32_at(local), 0x0403_4b50, "{name} has a local header");
        let data = local + 30 + u16_at(local + 26) + u16_at(local + 28);
        let mut inflated = Vec::new();
        flate2::read::DeflateDecoder::new(&archive[data..data + packed])
            .read_to_end(&mut inflated)
            .unwrap_or_else(|err| panic!("{name} inflates: {err}"));
        let mut sum = flate2::Crc::new();
        sum.update(&inflated);
        assert_eq!(sum.sum(), crc, "{name}'s CRC-32");
        assert_eq!(
            u32_at(data + packed),
            0x0807_4b50,
            "{name} has a data descriptor"
        );
        out.push((name, inflated));
        at += 46 + name_len + u16_at(at + 30) + u16_at(at + 32);
    }
    out
}

#[test]
fn the_spreadsheet_component_is_built_in_and_answers_with_no_configuration() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let (chunks, finished) = write(&request, &extension);
    // The compressor keeps a small batch until its block fills, so only the first chunk, which
    // has the sheet's zip header, and the last are sure to hold bytes.
    assert!(!chunks[0].is_empty() && !chunks[3].is_empty());
    let size: usize = chunks.iter().map(Vec::len).sum();
    assert_eq!(field(&finished, "size"), &Value::Uint(size as u64));
    let rows = field(&finished, "rows");
    let Value::Array(sheets) = rows else {
        panic!("`rows` is a list");
    };
    assert_eq!(sheets.len(), 2);
    assert_eq!(
        field(&sheets[0].1, "name"),
        &Value::String("Shop".to_owned())
    );
    assert_eq!(field(&sheets[0].1, "rows"), &Value::Uint(201));
    assert_eq!(field(&sheets[1].1, "rows"), &Value::Uint(1));
    block_on(request.end()).expect("the request ends");
}

#[test]
fn the_chunks_of_a_finished_workbook_are_one_valid_xlsx_archive() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let (chunks, _) = write(&request, &extension);
    block_on(request.end()).expect("the request ends");
    let archive = chunks.concat();
    let entries = entries(&archive);
    let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "xl/worksheets/sheet1.xml",
            "xl/worksheets/sheet2.xml",
            "xl/styles.xml",
            "xl/workbook.xml",
            "xl/_rels/workbook.xml.rels",
            "docProps/core.xml",
            "docProps/app.xml",
            "_rels/.rels",
            "[Content_Types].xml",
        ]
    );
    let sheet = String::from_utf8(entries[0].1.clone()).unwrap();
    assert!(sheet.contains(
        "<c r=\"A2\" t=\"inlineStr\"><is><t>Tea &amp; &lt;Coffee&gt; 0</t></is></c><c r=\"B2\"><v>0.5</v></c><c r=\"C2\" s=\"1\"><v>45306</v></c>"
    ), "{sheet}");
    assert!(
        sheet.contains("<pane ySplit=\"1\" topLeftCell=\"A2\""),
        "{sheet}"
    );
    assert!(sheet.ends_with("</sheetData></worksheet>"), "{sheet}");
    let styles = String::from_utf8(entries[2].1.clone()).unwrap();
    assert!(
        styles.contains("<numFmt numFmtId=\"164\" formatCode=\"yyyy-mm-dd\"/>"),
        "{styles}"
    );
    let workbook = String::from_utf8(entries[3].1.clone()).unwrap();
    assert!(
        workbook.contains("<sheet name=\"Shop\" sheetId=\"1\" r:id=\"rId1\"/><sheet name=\"Blog\""),
        "{workbook}"
    );
    assert!(!names.contains(&"xl/sharedStrings.xml"));
}

#[test]
fn two_writes_of_equal_input_are_byte_equal() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let (first, _) = write(&request, &extension);
    let (second, _) = write(&request, &extension);
    block_on(request.end()).expect("the request ends");
    assert_eq!(first.concat(), second.concat());
}

#[test]
fn a_workbook_never_finished_is_logged_naming_it() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let budget = Arc::new(Logged {
        meter: Meter::new(Duration::from_secs(30), None),
        log: Mutex::default(),
    });
    let request = host.request(Arc::clone(&budget) as Arc<dyn Budget>);
    let (_, _) = write(&request, &extension);
    let writer = call(
        &request,
        &extension,
        "writerOpen",
        vec![options("export.xlsx")],
    );
    call(
        &request,
        &extension,
        "writerSheet",
        vec![writer, sheet("Shop"), list(vec![])],
    );
    block_on(request.end()).expect("the request ends");
    let log = budget.log.lock().expect("the log locks");
    assert_eq!(
        *log,
        [(
            Level::Error,
            "The workbook `export.xlsx` was never finished. Call `finish()` after the last row."
                .to_owned()
        )]
    );
}
