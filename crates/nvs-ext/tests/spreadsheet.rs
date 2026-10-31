//! The spreadsheet component as `nvs-ext` sees it (`rule:packaging/the-first-party-components-are-built-in`,
//! `rule:core-classes/spreadsheet-writes-as-a-stream`, `rule:core-classes/spreadsheet-output-is-inert`):
//! `rule:core-classes/spreadsheet-value-types`): `Novis\Spreadsheet\Engine` is in the binary and
//! writes a workbook with no `[[extension]]` entry and no grant. Each batch of rows returns the
//! deflated bytes it made, the chunks of every call are one valid zip archive, and the guest's
//! memory stays flat over many batches. Strings are inline and no `xl/sharedStrings.xml` is
//! written. Every case of `cell-value` writes as its cell type, with the implicit formats of a
//! date, a time, a duration and a clock time; a whole number past 15 significant digits is text.
//! `writerFinish` returns each sheet's row count and the archive's size, two writes of equal
//! input are byte-equal, and a writer the request ends before `writerFinish` logs an error
//! naming its target.
//!
//! **The big file** (`rule:testing/feature-proofs`' growth): a workbook of about 100 MB in 20
//! columns is generated under `.agent-tmp/`, written to a file batch by batch and read back
//! through ranges of it, and the guest's memory is the same after the last batch as after the
//! sixteenth, writing and reading. A workbook of shared strings, laid out as another tool writes
//! one, costs the reader its strings and no more than one batch beside them.

use std::fs::{self, File};
use std::future::Future;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::SPREADSHEET_CLASS;
use nvs_ext::call::{Budget, Host, Lending, Level, Meter, Request};
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
    cell_of(vec![(case, value)])
}

/// A cell holding the `cell-value` case whose fields are `fields`.
fn cell_of(fields: Vec<(&str, Value)>) -> Value {
    shape(vec![("value", shape(fields)), ("style", Value::Null)])
}

/// The `Core` value of class `class` with `fields`.
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

fn time_of_day(hour: i64, minute: i64, second: i64) -> Value {
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

fn row(cells: Vec<Value>) -> Value {
    shape(vec![
        ("cells", list(cells)),
        ("style", Value::Null),
        ("height", Value::Null),
        ("hidden", Value::Bool(false)),
        ("level", Value::Uint(0)),
        ("header", Value::Bool(false)),
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
        ("ods", Value::Bool(false)),
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
        (
            "header",
            shape(vec![
                ("defaultStyle", Value::Bool(true)),
                ("style", Value::Null),
                ("freeze", Value::Bool(true)),
                ("filter", Value::Bool(true)),
            ]),
        ),
        ("newSheetOnOverflow", Value::Bool(false)),
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
                cell("date", date(2024, 1, 15)),
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

/// The text of the entry `name` in `entries`.
fn part(entries: &[(String, Vec<u8>)], name: &str) -> String {
    let (_, data) = entries
        .iter()
        .find(|(entry, _)| entry == name)
        .unwrap_or_else(|| panic!("the archive has `{name}`"));
    String::from_utf8(data.clone()).expect("an XML part is UTF-8")
}

/// The entries of a workbook whose one sheet, `Orders`, has `rows`, written in a request of
/// its own.
fn one_sheet(rows: Vec<Value>) -> Vec<(String, Vec<u8>)> {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let writer = call(
        &request,
        &extension,
        "writerOpen",
        vec![options("orders.xlsx")],
    );
    let mut chunks = vec![bytes(call(
        &request,
        &extension,
        "writerSheet",
        vec![writer.clone(), sheet("Orders"), list(rows)],
    ))];
    let finished = call(&request, &extension, "writerFinish", vec![writer]);
    let Value::Bytes(last) = field(&finished, "data").clone() else {
        panic!("`data` is bytes");
    };
    chunks.push(last);
    block_on(request.end()).expect("the request ends");
    entries(&chunks.concat())
}

/// A row of `Orders`: its number as text, a price, a count and a date.
fn order(at: u32) -> Value {
    row(vec![
        cell("text", Value::String(format!("Order {at}"))),
        cell("number", Value::Float(f64::from(at) * 1.25)),
        cell("integer", Value::Int(i64::from(at) * 7)),
        cell(
            "date",
            date(2024, 1 + i64::from(at % 12), 1 + i64::from(at % 28)),
        ),
    ])
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

#[test]
fn a_write_needs_no_capability_grant() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    assert_eq!(
        extension.grants,
        nvs_config::extension::Granted::default(),
        "the spreadsheet component is granted nothing"
    );
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let (chunks, _) = write(&request, &extension);
    block_on(request.end()).expect("the request ends");
    assert_eq!(entries(&chunks.concat()).len(), 9);
}

/// Rows in one `writerRows` batch of [`a_batch_of_rows_returns_the_compressed_bytes_ready_to_leave`]
/// and [`the_guest_holds_no_row_of_an_earlier_batch`].
const BATCH: u32 = 2000;

/// The rows of batch `batch`, numbered on from the header row.
fn batch(batch: u32) -> Value {
    list((0..BATCH).map(|at| order(batch * BATCH + at)).collect())
}

#[test]
fn a_batch_of_rows_returns_the_compressed_bytes_ready_to_leave() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let writer = call(
        &request,
        &extension,
        "writerOpen",
        vec![options("orders.xlsx")],
    );
    let header = row(vec![cell("text", Value::String("Order".to_owned()))]);
    let mut chunks = vec![bytes(call(
        &request,
        &extension,
        "writerSheet",
        vec![writer.clone(), sheet("Orders"), list(vec![header])],
    ))];
    for at in 0..10 {
        let chunk = bytes(call(
            &request,
            &extension,
            "writerRows",
            vec![writer.clone(), batch(at)],
        ));
        // A batch of this size is far past the compressor's block, so each one returns bytes.
        assert!(
            chunk.len() > 1000,
            "batch {at} returned {} bytes",
            chunk.len()
        );
        chunks.push(chunk);
    }
    let finished = call(&request, &extension, "writerFinish", vec![writer]);
    block_on(request.end()).expect("the request ends");
    let Value::Bytes(last) = field(&finished, "data").clone() else {
        panic!("`data` is bytes");
    };
    chunks.push(last);
    let archive = chunks.concat();
    assert_eq!(field(&finished, "size"), &Value::Uint(archive.len() as u64));
    let entries = entries(&archive);
    let sheet = part(&entries, "xl/worksheets/sheet1.xml");
    assert_eq!(sheet.matches("<row ").count(), 1 + 10 * BATCH as usize);
    assert!(
        sheet.contains("<c r=\"A2\" t=\"inlineStr\"><is><t>Order 0</t></is></c>"),
        "{sheet}"
    );
    assert!(
        sheet.contains(
            "<c r=\"A20001\" t=\"inlineStr\"><is><t>Order 19999</t></is></c><c r=\"B20001\"><v>24998.75</v></c><c r=\"C20001\"><v>139993</v></c>"
        ),
        "the last row is in the archive"
    );
}

#[test]
fn the_guest_holds_no_row_of_an_earlier_batch() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    // A `Meter`'s CPU budget is wall-clock time, and this test measures memory, not time. An hour
    // keeps 40 debug-built batches on a loaded machine clear of the deadline.
    let meter = Arc::new(Meter::new(Duration::from_secs(3600), None));
    let request = host.request(Arc::clone(&meter) as Arc<dyn Budget>);
    let writer = call(
        &request,
        &extension,
        "writerOpen",
        vec![options("orders.xlsx")],
    );
    call(
        &request,
        &extension,
        "writerSheet",
        vec![writer.clone(), sheet("Orders"), list(vec![])],
    );
    // The first two batches let the guest's memory grow to what one batch needs.
    for at in 0..2 {
        call(
            &request,
            &extension,
            "writerRows",
            vec![writer.clone(), batch(at)],
        );
    }
    let warm = meter.charged();
    assert!(warm > 0, "the guest's memory is charged to the request");
    for at in 2..40 {
        call(
            &request,
            &extension,
            "writerRows",
            vec![writer.clone(), batch(at)],
        );
    }
    let after = meter.charged();
    let finished = call(&request, &extension, "writerFinish", vec![writer]);
    block_on(request.end()).expect("the request ends");
    let Value::Array(sheets) = field(&finished, "rows") else {
        panic!("`rows` is a list");
    };
    assert_eq!(
        field(&sheets[0].1, "rows"),
        &Value::Uint(40 * u64::from(BATCH))
    );
    // Many times the rows went in, and the guest's memory is the same.
    assert_eq!(after, warm, "the guest grew past one batch's memory");
}

/// A folder of its own under `.agent-tmp/`, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(case: &str) -> Self {
        let root = nvs_repo::path(".agent-tmp")
            .join(format!("ext-spreadsheet-{case}-{}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).expect("the scratch folder is made");
        Self(root)
    }

    fn at(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

/// xorshift64*, seeded per row, so every row of a generated workbook is made again to check
/// what was read back.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// `length` letters and digits, which deflate barely shrinks.
    fn word(&mut self, length: usize) -> String {
        const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        (0..length)
            .map(|_| {
                let at = usize::try_from(self.next() % LETTERS.len() as u64).unwrap();
                char::from(LETTERS[at])
            })
            .collect()
    }
}

/// The 20 cell values of row `number` of a wide workbook: the number, seven fractions, four dates
/// and eight words. Each is the `cell-value` a write takes and the value a read gives.
fn wide_values(number: u32) -> Vec<Value> {
    let mut noise = Noise(u64::from(number).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut out = vec![shape(vec![("integer", Value::Int(i64::from(number)))])];
    for _ in 0..7 {
        // An odd count of 1024ths below 2^42 is exact and never whole, so it reads as a number.
        let fraction = ((noise.next() >> 22) | 1) as f64 / 1024.0;
        out.push(shape(vec![("number", Value::Float(fraction))]));
    }
    for _ in 0..4 {
        let n = i64::try_from(noise.next() >> 1).unwrap();
        let day = date(2000 + n % 30, 1 + n / 30 % 12, 1 + n / 360 % 28);
        out.push(shape(vec![("date", day)]));
    }
    for _ in 0..8 {
        out.push(shape(vec![("text", Value::String(noise.word(24)))]));
    }
    out
}

/// Rows in one batch of a wide workbook, written and read.
const WIDE_BATCH: u32 = 2000;

/// Batches of a wide workbook after which the guest's memory stays the same. Batches differ in
/// size, so the guest grows over the first few of them before it reaches what one batch needs.
const WARM_BATCHES: u32 = 16;

/// What the guest's memory was charged in one request over a wide workbook.
struct Peak {
    rows: u32,
    /// After the first [`WARM_BATCHES`] batches, once the guest has grown to what one batch needs.
    warm: u64,
    /// When the last batch is done. A guest's memory never shrinks, so this is its peak.
    last: u64,
}

/// Writes a wide workbook to `path` a batch at a time until `done(rows, bytes)` holds, and
/// returns the guest's memory over it.
fn write_wide(path: &Path, done: impl Fn(u32, u64) -> bool) -> Peak {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    // A `Meter`'s CPU budget is wall-clock time, and this measures memory, not time.
    let meter = Arc::new(Meter::new(Duration::from_secs(3600), None));
    let request = host.request(Arc::clone(&meter) as Arc<dyn Budget>);
    let mut file = File::create(path).expect("the workbook's file is made");
    let mut size = 0u64;
    let mut put = |chunk: Vec<u8>| {
        file.write_all(&chunk).expect("the chunk is written");
        size += chunk.len() as u64;
        size
    };
    let writer = call(
        &request,
        &extension,
        "writerOpen",
        vec![options("orders.xlsx")],
    );
    put(bytes(call(
        &request,
        &extension,
        "writerSheet",
        vec![writer.clone(), sheet("Orders"), list(vec![])],
    )));
    let mut rows = 0u32;
    let mut warm = 0;
    let mut written = 0;
    while !done(rows, written) {
        let batch = (rows + 1..=rows + WIDE_BATCH)
            .map(|number| {
                row(wide_values(number)
                    .into_iter()
                    .map(|value| shape(vec![("value", value), ("style", Value::Null)]))
                    .collect())
            })
            .collect();
        written = put(bytes(call(
            &request,
            &extension,
            "writerRows",
            vec![writer.clone(), list(batch)],
        )));
        rows += WIDE_BATCH;
        if rows == WARM_BATCHES * WIDE_BATCH {
            warm = meter.charged();
        }
    }
    let last = meter.charged();
    let finished = call(&request, &extension, "writerFinish", vec![writer]);
    let Value::Bytes(tail) = field(&finished, "data").clone() else {
        panic!("`data` is bytes");
    };
    put(tail);
    block_on(request.end()).expect("the request ends");
    Peak { rows, warm, last }
}

/// A budget that lends one file, as the host lends a file the program opened.
struct Opened {
    meter: Meter,
    file: Mutex<File>,
}

impl Budget for Opened {
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
        assert_eq!(id, 0, "the guest reads the one file it was lent");
        let mut file = self.file.lock().expect("the file locks");
        let mut out = Vec::new();
        file.seek(SeekFrom::Start(offset)).expect("the file seeks");
        (&mut *file)
            .take(length)
            .read_to_end(&mut out)
            .expect("the file reads");
        Lending::ready(Ok(out))
    }
}

/// `readerOpen`'s bound with neither half set, which the guest reads as the shipped default.
fn shipped() -> Value {
    shape(vec![("maxBytes", Value::Null), ("maxRatio", Value::Null)])
}

/// `readerRows`'s options for the sheet `name` from the row `from`, every other one its default.
fn read_options(name: &str, from: Option<u64>) -> Value {
    shape(vec![
        ("sheet", Value::String(name.to_owned())),
        ("headerRow", Value::Null),
        ("from", from.map_or(Value::Null, Value::Uint)),
        ("limit", Value::Null),
        ("columns", list(vec![])),
        ("text", list(vec![])),
        ("textAll", Value::Bool(false)),
        ("formulas", Value::Bool(false)),
        ("keepEmptyRows", Value::Bool(false)),
        ("fillMerged", Value::Bool(false)),
        ("skipHidden", Value::Bool(false)),
        ("locale", Value::String("en".to_owned())),
    ])
}

/// Reads every row of the sheet `name` from `source`, a batch of `max` at a time, hands each to
/// `check` as its number and its cells' values in column order, and returns the guest's memory
/// over it.
fn read_all(
    request: &Request,
    extension: &Extension,
    meter: &Meter,
    source: Value,
    name: &str,
    max: u32,
    mut check: impl FnMut(u64, Vec<Value>),
) -> Peak {
    let reader = call(
        request,
        extension,
        "readerOpen",
        vec![source.clone(), shipped()],
    );
    let mut from = None;
    let mut rows = 0u32;
    let mut batches = 0;
    let mut warm = 0;
    loop {
        let args = vec![
            reader.clone(),
            source.clone(),
            read_options(name, from),
            Value::Uint(u64::from(max)),
        ];
        let batch = call(request, extension, "readerRows", args);
        let Value::Array(read) = field(&batch, "rows") else {
            panic!("`rows` is a list");
        };
        for (_, read) in read {
            let Value::Uint(number) = field(read, "number") else {
                panic!("a row number is a uint");
            };
            let Value::Array(cells) = field(read, "cells") else {
                panic!("`cells` is a list");
            };
            let values = cells
                .iter()
                .enumerate()
                .map(|(at, (_, cell))| {
                    assert_eq!(
                        field(cell, "column"),
                        &Value::Uint(at as u64 + 1),
                        "row {number}"
                    );
                    field(cell, "value").clone()
                })
                .collect();
            check(*number, values);
            from = Some(number + 1);
            rows += 1;
        }
        batches += 1;
        if batches == WARM_BATCHES {
            warm = meter.charged();
        }
        if field(&batch, "done") == &Value::Bool(true) {
            return Peak {
                rows,
                warm,
                last: meter.charged(),
            };
        }
    }
}

/// Reads the wide workbook at `path` through ranges of the file, checks every row against
/// [`wide_values`], and returns the guest's memory over it.
fn read_wide(path: &Path) -> Peak {
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let size = fs::metadata(path).expect("the workbook is on disk").len();
    let opened = Arc::new(Opened {
        meter: Meter::new(Duration::from_secs(3600), None),
        file: Mutex::new(File::open(path).expect("the workbook opens")),
    });
    let request = host.request(Arc::clone(&opened) as Arc<dyn Budget>);
    let peak = read_all(
        &request,
        &extension,
        &opened.meter,
        Value::Lent { id: 0, size },
        "Orders",
        WIDE_BATCH,
        |number, values| {
            assert_eq!(
                values,
                wide_values(u32::try_from(number).unwrap()),
                "row {number} reads back as it was written"
            );
        },
    );
    block_on(request.end()).expect("the request ends");
    peak
}

/// About 100 MB: the size of the big workbook.
const BIG: u64 = 100_000_000;

#[test]
fn a_100_mb_workbook_writes_and_reads_back() {
    let scratch = Scratch::new("big");
    let path = scratch.at("orders.xlsx");
    let written = write_wide(&path, |_, bytes| bytes >= BIG);
    let size = fs::metadata(&path).expect("the workbook is on disk").len();
    assert!(size >= BIG, "the workbook is {size} bytes");
    let read = read_wide(&path);
    assert_eq!(read.rows, written.rows, "every row reads back");
    assert_eq!(
        written.last, written.warm,
        "the writer grew past one batch's memory"
    );
    assert_eq!(
        read.last, read.warm,
        "the reader grew past one batch's memory"
    );
}

#[test]
fn the_guest_peak_stays_flat_as_rows_grow() {
    let scratch = Scratch::new("flat");
    let mut peaks = Vec::new();
    for rows in [WARM_BATCHES * WIDE_BATCH, 4 * WARM_BATCHES * WIDE_BATCH] {
        let path = scratch.at(&format!("orders-{rows}.xlsx"));
        let written = write_wide(&path, |done, _| done >= rows);
        let read = read_wide(&path);
        assert_eq!(read.rows, rows);
        peaks.push((written.last, read.last));
    }
    // Four times the rows, and the guest's peak is the same writing and reading.
    assert_eq!(peaks[0], peaks[1], "the peak grew with the rows");
}

/// A zip archive of `parts`, each stored, as another tool may lay one out.
fn stored_zip(parts: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in parts {
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let mut header = Vec::new();
        header.extend_from_slice(&20u16.to_le_bytes());
        // The flags and the method: stored.
        header.extend_from_slice(&[0; 4]);
        // The time and the date.
        header.extend_from_slice(&[0; 4]);
        header.extend_from_slice(&crc.sum().to_le_bytes());
        let length = u32::try_from(data.len()).unwrap().to_le_bytes();
        header.extend_from_slice(&length);
        header.extend_from_slice(&length);
        header.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        let offset = u32::try_from(out.len()).unwrap();
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&header);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&header);
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

/// An xlsx whose one sheet, `Notes`, has `sheet_data` inside its `<sheetData>`, and whose shared
/// strings are `shared`.
fn shared_xlsx(sheet_data: &str, shared: &[String]) -> Vec<u8> {
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let package = "http://schemas.openxmlformats.org/package/2006/relationships";
    let mut sst = format!(r#"<?xml version="1.0"?><sst xmlns="{main}">"#);
    for text in shared {
        sst.push_str(&format!("<si><t>{text}</t></si>"));
    }
    sst.push_str("</sst>");
    stored_zip(&[
        ("[Content_Types].xml", b"<Types/>".to_vec()),
        (
            "_rels/.rels",
            format!(r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#).into_bytes(),
        ),
        (
            "xl/workbook.xml",
            format!(r#"<?xml version="1.0"?><workbook xmlns="{main}" xmlns:r="{rel}"><sheets><sheet name="Notes" sheetId="1" r:id="rId1"/></sheets></workbook>"#).into_bytes(),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            format!(r#"<?xml version="1.0"?><Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rIdT" Type="{rel}/sharedStrings" Target="sharedStrings.xml"/></Relationships>"#).into_bytes(),
        ),
        ("xl/sharedStrings.xml", sst.into_bytes()),
        (
            "xl/worksheets/sheet1.xml",
            format!(r#"<?xml version="1.0"?><worksheet xmlns="{main}"><sheetData>{sheet_data}</sheetData></worksheet>"#).into_bytes(),
        ),
    ])
}

/// Reads every row of `bytes`'s sheet `Notes`, checks each cell is the text `texts` gives it,
/// and returns the guest's memory at the end.
fn read_notes(bytes: Vec<u8>, texts: &[String], columns: usize) -> u64 {
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let meter = Arc::new(Meter::new(Duration::from_secs(3600), None));
    let request = host.request(Arc::clone(&meter) as Arc<dyn Budget>);
    let peak = read_all(
        &request,
        &extension,
        &meter,
        Value::Bytes(bytes),
        "Notes",
        WIDE_BATCH,
        |number, values| {
            let first = (usize::try_from(number).unwrap() - 1) * columns;
            let expected: Vec<Value> = texts[first..first + columns]
                .iter()
                .map(|text| shape(vec![("text", Value::String(text.clone()))]))
                .collect();
            assert_eq!(values, expected, "row {number}");
        },
    );
    block_on(request.end()).expect("the request ends");
    peak.last
}

#[test]
fn a_text_heavy_workbook_peaks_at_its_strings_plus_a_batch() {
    const ROWS: usize = 20_000;
    const COLUMNS: usize = 10;
    let mut noise = Noise(0x5eed);
    let texts: Vec<String> = (0..ROWS * COLUMNS).map(|_| noise.word(40)).collect();
    let column = |at: usize| char::from(b'A' + u8::try_from(at).unwrap());
    let mut shared = String::new();
    let mut inline = String::new();
    for number in 1..=ROWS {
        shared.push_str(&format!(r#"<row r="{number}">"#));
        inline.push_str(&format!(r#"<row r="{number}">"#));
        for at in 0..COLUMNS {
            let index = (number - 1) * COLUMNS + at;
            let cell = column(at);
            shared.push_str(&format!(
                r#"<c r="{cell}{number}" t="s"><v>{index}</v></c>"#
            ));
            inline.push_str(&format!(
                r#"<c r="{cell}{number}" t="inlineStr"><is><t>{}</t></is></c>"#,
                texts[index]
            ));
        }
        shared.push_str("</row>");
        inline.push_str("</row>");
    }
    // The same rows twice: once with every string in the file's string table, once with each
    // string in its cell. What the first costs past the second is what the table costs.
    let with_table = read_notes(shared_xlsx(&shared, &texts), &texts, COLUMNS);
    let batch_alone = read_notes(shared_xlsx(&inline, &[]), &texts, COLUMNS);
    let strings: usize = texts.iter().map(String::len).sum();
    let table = with_table - batch_alone;
    assert!(
        table >= strings as u64,
        "the table cost {table} bytes, less than its {strings} bytes of text"
    );
    // Each string's text and a four-byte end, with room for the arena to double.
    let most = 2 * (strings + 4 * texts.len()) as u64;
    assert!(
        table <= most,
        "the table cost {table} bytes, past {most} for {strings} bytes of text"
    );
}

#[test]
fn strings_are_written_inline_and_no_string_table_grows() {
    let rows = (0..300)
        .map(|at| {
            row(vec![
                cell("text", Value::String("Shop".to_owned())),
                cell("text", Value::String(format!("Order {at}"))),
                cell("integer", Value::Int(at)),
            ])
        })
        .collect();
    let entries = one_sheet(rows);
    let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
    assert!(!names.contains(&"xl/sharedStrings.xml"), "{names:?}");
    let sheet = part(&entries, "xl/worksheets/sheet1.xml");
    assert_eq!(sheet.matches(" t=\"inlineStr\"").count(), 600);
    assert_eq!(sheet.matches("<is><t>Shop</t></is>").count(), 300);
    assert!(!sheet.contains(" t=\"s\""), "{sheet}");
    assert!(
        sheet.contains(
            "<c r=\"A300\" t=\"inlineStr\"><is><t>Shop</t></is></c><c r=\"B300\" t=\"inlineStr\"><is><t>Order 299</t></is></c><c r=\"C300\"><v>299</v></c>"
        ),
        "{sheet}"
    );
    for name in ["[Content_Types].xml", "xl/_rels/workbook.xml.rels"] {
        let text = part(&entries, name);
        assert!(!text.contains("sharedStrings"), "{name}: {text}");
    }
}

#[test]
fn every_novis_type_writes_as_its_cell_type() {
    let runs = list(vec![
        shape(vec![
            ("text", Value::String("Tea ".to_owned())),
            ("style", Value::Null),
        ]),
        shape(vec![
            ("text", Value::String("time".to_owned())),
            ("style", Value::Null),
        ]),
    ]);
    let entries = one_sheet(vec![row(vec![
        cell("text", Value::String("Shop".to_owned())),
        cell("number", Value::Float(2.25)),
        cell("integer", Value::Int(42)),
        cell("decimal", Value::String("19.99".to_owned())),
        cell("boolean", Value::Bool(true)),
        cell("date", date(2024, 1, 15)),
        cell("time", time_of_day(6, 0, 0)),
        cell(
            "duration",
            core("Core\\Time\\Duration", &[("nanos", 36 * 3_600_000_000_000)]),
        ),
        cell_of(vec![
            ("day", date(2024, 1, 15)),
            ("clock", time_of_day(18, 0, 0)),
        ]),
        cell_of(vec![
            ("formula", Value::String("B1+C1".to_owned())),
            ("cachedText", Value::Null),
            ("cachedNumber", Value::Float(44.25)),
        ]),
        cell_of(vec![
            ("formula", Value::String("UPPER(A1)".to_owned())),
            ("cachedText", Value::String("SHOP".to_owned())),
            ("cachedNumber", Value::Null),
        ]),
        cell("runs", runs),
    ])]);
    let sheet = part(&entries, "xl/worksheets/sheet1.xml");
    for expected in [
        "<c r=\"A1\" t=\"inlineStr\"><is><t>Shop</t></is></c>",
        "<c r=\"B1\"><v>2.25</v></c>",
        "<c r=\"C1\"><v>42</v></c>",
        "<c r=\"D1\"><v>19.99</v></c>",
        "<c r=\"E1\" t=\"b\"><v>1</v></c>",
        "<c r=\"F1\" s=\"1\"><v>45306</v></c>",
        "<c r=\"G1\" s=\"2\"><v>0.25</v></c>",
        "<c r=\"H1\" s=\"3\"><v>1.5</v></c>",
        "<c r=\"I1\" s=\"4\"><v>45306.75</v></c>",
        "<c r=\"J1\"><f>B1+C1</f><v>44.25</v></c>",
        "<c r=\"K1\" t=\"str\"><f>UPPER(A1)</f><v>SHOP</v></c>",
        "<c r=\"L1\" t=\"inlineStr\"><is><r><t xml:space=\"preserve\">Tea </t></r><r><t>time</t></r></is></c>",
    ] {
        assert!(sheet.contains(expected), "{expected} is not in {sheet}");
    }
    let styles = part(&entries, "xl/styles.xml");
    assert!(
        styles.contains(
            "<numFmts count=\"4\"><numFmt numFmtId=\"164\" formatCode=\"yyyy-mm-dd\"/><numFmt numFmtId=\"165\" formatCode=\"hh:mm:ss\"/><numFmt numFmtId=\"166\" formatCode=\"[h]:mm:ss\"/><numFmt numFmtId=\"167\" formatCode=\"yyyy-mm-dd hh:mm\"/></numFmts>"
        ),
        "{styles}"
    );
    for (xf, format) in [(1, 164), (2, 165), (3, 166), (4, 167)] {
        let entry = styles
            .split("<cellXfs")
            .nth(1)
            .and_then(|xfs| xfs.split("<xf ").nth(xf + 1))
            .unwrap_or_else(|| panic!("styles.xml has the format {xf}: {styles}"));
        assert!(
            entry.starts_with(&format!("numFmtId=\"{format}\"")),
            "the format {xf} is {entry}"
        );
    }
}

#[test]
fn a_number_past_fifteen_significant_digits_is_a_text_cell() {
    let entries = one_sheet(vec![row(vec![
        cell("integer", Value::Int(1_234_567_890_123_456_789)),
        cell("integer", Value::Int(123_456_789_012_345)),
        cell("integer", Value::Int(-9_000_000_000_000_000_000)),
        cell("decimal", Value::String("12345678901234.567".to_owned())),
    ])]);
    let sheet = part(&entries, "xl/worksheets/sheet1.xml");
    for expected in [
        "<c r=\"A1\" t=\"inlineStr\"><is><t>1234567890123456789</t></is></c>",
        "<c r=\"B1\"><v>123456789012345</v></c>",
        "<c r=\"C1\"><v>-9000000000000000000</v></c>",
        "<c r=\"D1\" t=\"inlineStr\"><is><t>12345678901234.567</t></is></c>",
    ] {
        assert!(sheet.contains(expected), "{expected} is not in {sheet}");
    }
}

#[test]
fn a_clock_time_is_its_serial_with_the_date_time_format() {
    let entries = one_sheet(vec![row(vec![cell_of(vec![
        ("day", date(2024, 3, 1)),
        ("clock", time_of_day(12, 30, 0)),
    ])])]);
    // 2024-03-01 is day 45352 of the 1900 date system, and 12:30 is that fraction of a day.
    let serial = 45_352.0 + f64::from(12 * 60 + 30) * 60.0 * 1e9 / 86_400_000_000_000.0;
    let sheet = part(&entries, "xl/worksheets/sheet1.xml");
    let expected = format!("<c r=\"A1\" s=\"1\"><v>{serial}</v></c>");
    assert_eq!(serial.to_string(), "45352.520833333336");
    assert!(sheet.contains(&expected), "{expected} is not in {sheet}");
    let styles = part(&entries, "xl/styles.xml");
    assert!(
        styles.contains("<numFmt numFmtId=\"164\" formatCode=\"yyyy-mm-dd hh:mm\"/>"),
        "{styles}"
    );
    assert!(
        styles.contains("<cellXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"164\""),
        "{styles}"
    );
}

#[test]
fn finish_returns_the_row_count_per_sheet_and_the_size() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let writer = call(
        &request,
        &extension,
        "writerOpen",
        vec![options("shop.xlsx")],
    );
    let mut chunks = vec![bytes(call(
        &request,
        &extension,
        "writerSheet",
        vec![
            writer.clone(),
            sheet("Shop"),
            list((0..3).map(order).collect()),
        ],
    ))];
    chunks.push(bytes(call(
        &request,
        &extension,
        "writerRows",
        vec![writer.clone(), list((3..7).map(order).collect())],
    )));
    chunks.push(bytes(call(
        &request,
        &extension,
        "writerSheet",
        vec![
            writer.clone(),
            sheet("Orders"),
            list((0..2).map(order).collect()),
        ],
    )));
    let finished = call(&request, &extension, "writerFinish", vec![writer]);
    block_on(request.end()).expect("the request ends");
    let Value::Bytes(last) = field(&finished, "data").clone() else {
        panic!("`data` is bytes");
    };
    chunks.push(last);
    let size: usize = chunks.iter().map(Vec::len).sum();
    assert_eq!(field(&finished, "size"), &Value::Uint(size as u64));
    assert_eq!(
        field(&finished, "rows"),
        &list(vec![
            shape(vec![
                ("name", Value::String("Shop".to_owned())),
                ("rows", Value::Uint(7)),
            ]),
            shape(vec![
                ("name", Value::String("Orders".to_owned())),
                ("rows", Value::Uint(2)),
            ]),
        ])
    );
    let entries = entries(&chunks.concat());
    assert_eq!(
        part(&entries, "xl/worksheets/sheet1.xml")
            .matches("<row ")
            .count(),
        7
    );
    assert_eq!(
        part(&entries, "xl/worksheets/sheet2.xml")
            .matches("<row ")
            .count(),
        2
    );
}
