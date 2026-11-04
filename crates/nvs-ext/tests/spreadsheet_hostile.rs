//! Hostile spreadsheets against the built-in spreadsheet component: each one ends in a throw or a
//! contained trap (`rule:core-classes/decompression-bound`,
//! `rule:core-classes/spreadsheet-reads-as-a-stream`).

use std::future::Future;
use std::io::{Read as _, Write as _};
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::{Decompression, SPREADSHEET_CLASS, cap_decompression};
use nvs_ext::call::{
    Budget, Failure, Host, Lending, Level, Limit, Meter, Outbound, Outgoing, Request,
};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Extension, Loader};

fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
    }
}

/// The spreadsheet component, compiled and checked.
fn spreadsheet(host: &Host) -> Extension {
    Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load")
        .iter()
        .find(|builtin| builtin.manifest().class == SPREADSHEET_CLASS)
        .expect("the spreadsheet component is built in")
        .extension()
        .expect("the spreadsheet component compiles and checks")
        .clone()
}

/// A zip archive of `parts`, each deflated.
fn zip(parts: &[(&str, &[u8])]) -> Vec<u8> {
    zip_as(parts, true)
}

/// A zip archive of `parts`, each deflated when `deflate` is set and stored when it is not.
fn zip_as(parts: &[(&str, &[u8])], deflate: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    let method = if deflate { 8 } else { 0 };
    for (name, data) in parts {
        let packed = if deflate {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
            encoder.write_all(data).unwrap();
            encoder.finish().unwrap()
        } else {
            data.to_vec()
        };
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let offset = u32::try_from(out.len()).unwrap();
        let sizes = |out: &mut Vec<u8>| {
            out.extend_from_slice(&[0, 0, method, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&crc.sum().to_le_bytes());
            out.extend_from_slice(&u32::try_from(packed.len()).unwrap().to_le_bytes());
            out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
            out.extend_from_slice(&u16::try_from(name.len()).unwrap().to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
        };
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        sizes(&mut out);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&[20, 0, 20, 0]);
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

/// A `readerOpen` bound asking for `bytes` and `ratio`, `None` asking for nothing.
fn bound(bytes: Option<u64>, ratio: Option<u64>) -> Value {
    let half = |value: Option<u64>| value.map_or(Value::Null, Value::Uint);
    Value::Array(vec![
        (Key::String("maxBytes".to_owned()), half(bytes)),
        (Key::String("maxRatio".to_owned()), half(ratio)),
    ])
}

/// `readerOpen` over `file` with `asked`, clamped as the host clamps a call to `in_force`: the
/// error's text, or `None` when the workbook opens.
fn open(file: &[u8], asked: Value, in_force: Decompression) -> Option<String> {
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let mut args = vec![Value::Bytes(file.to_vec()), asked];
    cap_decompression(SPREADSHEET_CLASS, "readerOpen", &mut args, in_force);
    block_on(request.call_values(&extension, "readerOpen", args))
        .err()
        .map(|err| err.to_string())
}

/// The workbook part, padded to 200,000 bytes that deflate to a few hundred, is read whole when
/// the reader opens. Each half of the server's bound stops it, a call cannot raise either, and a
/// bound above it opens the workbook.
#[test]
fn the_configured_decompression_bound_reaches_the_guest() {
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let mut book = format!(r#"<?xml version="1.0"?><workbook xmlns="{main}"><sheets/>"#);
    book.push_str(&" ".repeat(200_000 - book.len() - "</workbook>".len()));
    book.push_str("</workbook>");
    let file = zip(&[("xl/workbook.xml", book.as_bytes())]);
    let roomy = Decompression {
        bytes: 1 << 20,
        ratio: 1 << 20,
    };

    let err = open(
        &file,
        bound(None, None),
        Decompression {
            bytes: 199_999,
            ..roomy
        },
    )
    .expect("one byte under the held part is refused");
    assert!(err.contains("max_decompressed"), "{err}");
    assert!(err.contains("xl/workbook.xml"), "{err}");

    let err = open(
        &file,
        bound(None, None),
        Decompression { ratio: 10, ..roomy },
    )
    .expect("a ratio of 10 is refused");
    assert!(err.contains("max_decompression_ratio"), "{err}");

    let err = open(
        &file,
        bound(Some(u64::MAX), Some(u64::MAX)),
        Decompression {
            bytes: 199_999,
            ..roomy
        },
    )
    .expect("a call asking for more gets the server's bound");
    assert!(err.contains("max_decompressed"), "{err}");

    let err = open(&file, bound(Some(1000), None), roomy).expect("a call may lower the bound");
    assert!(err.contains("max_decompressed"), "{err}");

    assert_eq!(
        open(
            &file,
            bound(None, None),
            Decompression {
                bytes: 200_000,
                ..roomy
            }
        ),
        None
    );
}

/// An xlsx of one sheet, `Orders`, whose part is `sheet` and whose shared strings are `shared`,
/// each a whole XML part.
fn xlsx(sheet: &[u8], shared: &[u8]) -> Vec<u8> {
    xlsx_named(sheet, shared, "")
}

/// [`xlsx`] whose workbook has the `<definedName>` elements `names`.
fn xlsx_named(sheet: &[u8], shared: &[u8], names: &str) -> Vec<u8> {
    xlsx_with(sheet, shared, names, None)
}

/// [`xlsx_named`] with the styles part `styles`, when there is one.
fn xlsx_with(sheet: &[u8], shared: &[u8], names: &str, styles: Option<&[u8]>) -> Vec<u8> {
    xlsx_as(sheet, shared, names, styles, true)
}

/// [`xlsx_with`] whose parts are deflated when `deflate` is set and stored when it is not.
fn xlsx_as(
    sheet: &[u8],
    shared: &[u8],
    names: &str,
    styles: Option<&[u8]>,
    deflate: bool,
) -> Vec<u8> {
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let package = "http://schemas.openxmlformats.org/package/2006/relationships";
    let root = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
    );
    let book = format!(
        r#"<workbook xmlns="{main}" xmlns:r="{rel}"><sheets><sheet name="Orders" sheetId="1" r:id="rId1"/></sheets><definedNames>{names}</definedNames></workbook>"#
    );
    let styled = if styles.is_some() {
        format!(r#"<Relationship Id="rId3" Type="{rel}/styles" Target="styles.xml"/>"#)
    } else {
        String::new()
    };
    let rels = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{rel}/sharedStrings" Target="sharedStrings.xml"/>{styled}</Relationships>"#
    );
    let mut parts: Vec<(&str, &[u8])> = vec![
        ("[Content_Types].xml", b"<Types/>"),
        ("_rels/.rels", root.as_bytes()),
        ("xl/workbook.xml", book.as_bytes()),
        ("xl/_rels/workbook.xml.rels", rels.as_bytes()),
        ("xl/sharedStrings.xml", shared),
        ("xl/worksheets/sheet1.xml", sheet),
    ];
    if let Some(styles) = styles {
        parts.push(("xl/styles.xml", styles));
    }
    zip_as(&parts, deflate)
}

/// A styles part whose custom number formats are `codes`, from id 164 up, each the format of
/// one cell style in the same order.
fn styles(codes: &[String]) -> Vec<u8> {
    let (mut formats, mut xfs) = (String::new(), String::new());
    for (at, code) in codes.iter().enumerate() {
        let id = 164 + at;
        formats.push_str(&format!(r#"<numFmt numFmtId="{id}" formatCode="{code}"/>"#));
        xfs.push_str(&format!(r#"<xf numFmtId="{id}"/>"#));
    }
    format!(
        r#"<?xml version="1.0"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="{}">{formats}</numFmts><cellXfs count="{}">{xfs}</cellXfs></styleSheet>"#,
        codes.len(),
        codes.len()
    )
    .into_bytes()
}

/// A sheet part whose `<sheetData>` is `data`.
fn sheet(data: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{data}</sheetData></worksheet>"#
    )
    .into_bytes()
}

/// A shared-strings part whose strings are `items`, each the XML inside its `<si>`.
fn shared(items: &[&str]) -> Vec<u8> {
    let mut sst = String::from(
        r#"<?xml version="1.0"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );
    for item in items {
        sst.push_str(&format!("<si>{item}</si>"));
    }
    sst.push_str("</sst>");
    sst.into_bytes()
}

/// `part` with `dtd` after its XML declaration, where a document type declaration stands.
fn declared(dtd: &str, part: &[u8]) -> Vec<u8> {
    let end = part
        .windows(2)
        .position(|pair| pair == b"?>")
        .expect("the part has an XML declaration")
        + 2;
    [&part[..end], dtd.as_bytes(), &part[end..]].concat()
}

/// A request's budget under a memory cap, which counts every outbound HTTP request a guest sends
/// and sends none of them.
struct Watched {
    meter: Meter,
    sent: AtomicUsize,
}

impl Budget for Watched {
    fn cpu_spent(&self) -> bool {
        self.meter.cpu_spent()
    }

    fn charge(&self, bytes: i64) -> bool {
        self.meter.charge(bytes)
    }

    fn left(&self) -> Option<u64> {
        self.meter.left()
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

    fn send(&self, request: Outgoing) -> Outbound {
        self.sent.fetch_add(1, Ordering::AcqRel);
        self.meter.send(request)
    }

    fn read(&self, id: u64, offset: u64, length: u64) -> Lending {
        self.meter.read(id, offset, length)
    }
}

/// The memory cap every hostile read runs under, so a bound that failed would end in a trap.
const CAP: u64 = 256 << 20;

/// `file` opened with the shipped bound and every row of `Orders` read, as a request under
/// [`CAP`] reads it: the first failure's text, or `None` when the whole sheet reads, and the
/// outbound requests the guest sent.
fn read(file: &[u8]) -> (Option<String>, usize) {
    read_filling(file, false)
}

/// [`read`] with `fillMerged` set to `fill`.
fn read_filling(file: &[u8], fill: bool) -> (Option<String>, usize) {
    let (failure, sent, _) = read_in(file, fill);
    (failure, sent)
}

/// The time [`read`] takes over `file`, from the open to the last batch. The host and the
/// component are made before the clock starts, so the time is the reader's alone.
fn read_time(file: &[u8]) -> Duration {
    let (failure, _, took) = read_in(file, false);
    assert_eq!(failure, None, "the workbook reads");
    took
}

/// Asserts that reading `large`, eight times `small`, costs at most four times as much per unit:
/// the fastest of three reads of each, taken in turn so both see the same load
/// (`rule:testing/a-test-holds-at-any-speed`, a bound measured in the same run).
fn linear(small: &[u8], large: &[u8], what: &str) {
    linear_by(small, large, what, read_time);
}

/// [`linear`], with the time of one run of `small` or `large` taken by `time`.
fn linear_by(small: &[u8], large: &[u8], what: &str, time: fn(&[u8]) -> Duration) {
    let (mut fastest_small, mut fastest_large) = (Duration::MAX, Duration::MAX);
    for _ in 0..3 {
        fastest_small = fastest_small.min(time(small));
        fastest_large = fastest_large.min(time(large));
    }
    assert!(
        fastest_large < fastest_small * 32,
        "{what}: N took {fastest_small:?} and 8N {fastest_large:?}"
    );
}

/// [`read_filling`], with the time the reads took.
fn read_in(file: &[u8], fill: bool) -> (Option<String>, usize, Duration) {
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let started = std::time::Instant::now();
    let budget = Arc::new(Watched {
        meter: Meter::new(Duration::from_secs(120), Some(CAP)),
        sent: AtomicUsize::new(0),
    });
    let request = host.request(budget.clone());
    let source = Value::Bytes(file.to_vec());
    let failure = (|| {
        let reader = block_on(request.call_values(
            &extension,
            "readerOpen",
            vec![source.clone(), bound(None, None)],
        ))
        .map_err(|err| err.to_string())?
        .expect("`readerOpen` returns a reader");
        let mut from = Value::Null;
        loop {
            let options = rows_options(from.clone(), Value::Null, fill);
            let batch = block_on(request.call_values(
                &extension,
                "readerRows",
                vec![reader.clone(), source.clone(), options, Value::Uint(1000)],
            ))
            .map_err(|err| err.to_string())?
            .expect("`readerRows` returns a batch");
            let Value::Array(fields) = &batch else {
                panic!("{batch:?} is not a batch");
            };
            let field = |name: &str| {
                fields
                    .iter()
                    .find(|(key, _)| *key == Key::String(name.to_owned()))
                    .map(|(_, value)| value.clone())
                    .expect("a batch has its fields")
            };
            if field("done") == Value::Bool(true) {
                return Ok(());
            }
            let Value::Array(rows) = field("rows") else {
                panic!("a batch's rows are a list");
            };
            let Some((_, Value::Array(last))) = rows.last() else {
                panic!("a batch that is not done has a row");
            };
            let number = last
                .iter()
                .find(|(key, _)| *key == Key::String("number".to_owned()))
                .map(|(_, value)| value.clone());
            let Some(Value::Uint(number)) = number else {
                panic!("a row has its number");
            };
            from = Value::Uint(number + 1);
        }
    })()
    .err();
    let took = started.elapsed();
    drop(block_on(request.end()));
    (failure, budget.sent.load(Ordering::Acquire), took)
}

/// The `readerRows` options that read `Orders` from the row `from`, at most `limit` rows, with
/// `fillMerged` set to `fill`.
fn rows_options(from: Value, limit: Value, fill: bool) -> Value {
    Value::Array(
        [
            ("sheet", Value::String("Orders".to_owned())),
            ("headerRow", Value::Null),
            ("from", from),
            ("limit", limit),
            ("columns", Value::Array(Vec::new())),
            ("text", Value::Array(Vec::new())),
            ("textAll", Value::Bool(false)),
            ("formulas", Value::Bool(false)),
            ("keepEmptyRows", Value::Bool(false)),
            ("fillMerged", Value::Bool(fill)),
            ("skipHidden", Value::Bool(false)),
            ("locale", Value::String("en".to_owned())),
        ]
        .into_iter()
        .map(|(key, value)| (Key::String(key.to_owned()), value))
        .collect(),
    )
}

/// `file` opened under [`CAP`] and the first `limit` rows of `Orders` read: each row's number and
/// count of cells, or the first failure's text.
fn first_rows(file: &[u8], limit: u64) -> Result<Vec<(u64, usize)>, String> {
    first_rows_under(file, limit, SHIPPED)
}

/// The shipped `[limits] max_decompressed` and `max_decompression_ratio`.
const SHIPPED: Decompression = Decompression {
    bytes: 64 << 20,
    ratio: 1000,
};

/// `bytes` letters that deflate poorly, so a part of them stays under any decompression ratio.
fn noise(bytes: usize, seed: u32) -> String {
    let mut state = seed | 1;
    (0..bytes)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            char::from(b'a' + u8::try_from(state % 26).expect("under 26"))
        })
        .collect()
}

/// [`first_rows`] with the server's bound `in_force`.
fn first_rows_under(
    file: &[u8],
    limit: u64,
    in_force: Decompression,
) -> Result<Vec<(u64, usize)>, String> {
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(120), Some(CAP))));
    let source = Value::Bytes(file.to_vec());
    let read = (|| {
        let mut args = vec![source.clone(), bound(None, None)];
        cap_decompression(SPREADSHEET_CLASS, "readerOpen", &mut args, in_force);
        let reader = block_on(request.call_values(&extension, "readerOpen", args))
            .map_err(|err| err.to_string())?
            .expect("`readerOpen` returns a reader");
        let options = rows_options(Value::Null, Value::Uint(limit), false);
        let batch = block_on(request.call_values(
            &extension,
            "readerRows",
            vec![reader, source.clone(), options, Value::Uint(limit)],
        ))
        .map_err(|err| err.to_string())?
        .expect("`readerRows` returns a batch");
        let field = |of: &Value, name: &str| {
            let Value::Array(fields) = of else {
                panic!("{of:?} is not a shape");
            };
            fields
                .iter()
                .find(|(key, _)| *key == Key::String(name.to_owned()))
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| panic!("{of:?} has no `{name}`"))
        };
        let Value::Array(rows) = field(&batch, "rows") else {
            panic!("a batch's rows are a list");
        };
        Ok(rows
            .iter()
            .map(|(_, row)| {
                let Value::Uint(number) = field(row, "number") else {
                    panic!("a row has its number");
                };
                let Value::Array(cells) = field(row, "cells") else {
                    panic!("a row has its cells");
                };
                (number, cells.len())
            })
            .collect())
    })();
    drop(block_on(request.end()));
    read
}

/// A failure that is the reader's throw or the request's memory limit, never a crash.
fn contained(failure: Option<String>, what: &str) -> String {
    let failure = failure.unwrap_or_else(|| panic!("{what} reads as if it were a workbook"));
    assert!(
        // Both decompression bounds name the setting that raises them, `[limits]
        // max_decompression_ratio` or `max_decompressed`.
        failure.contains("[limits] max_decompress") || failure.contains("memory"),
        "{what}: {failure}"
    );
    failure
}

/// 30 MB of one byte deflate past the shipped ratio of 1000. A sheet part is streamed and a
/// shared-strings part is held, and each stops at the ratio, naming its part, long before the
/// request's memory cap.
#[test]
fn the_spreadsheet_bomb_corpus_ends_in_a_throw_or_a_contained_trap() {
    let bomb = " ".repeat(30_000_000);
    let sheet_bomb = xlsx(
        &sheet(&format!(r#"<row r="1"><c r="A1"><v>1</v></c></row>{bomb}"#)),
        &shared(&[]),
    );
    assert!(sheet_bomb.len() < 100_000, "{}", sheet_bomb.len());
    let failure = contained(read(&sheet_bomb).0, "a bomb in a sheet");
    assert!(failure.contains("xl/worksheets/sheet1.xml"), "{failure}");
    assert!(failure.contains("max_decompression_ratio"), "{failure}");

    let strings_bomb = xlsx(
        &sheet(r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#),
        &shared(&[&format!("<t>{}</t>", "a".repeat(30_000_000))]),
    );
    let failure = contained(read(&strings_bomb).0, "a bomb in the shared strings");
    assert!(failure.contains("xl/sharedStrings.xml"), "{failure}");
    assert!(failure.contains("max_decompression_ratio"), "{failure}");

    let plain = xlsx(
        &sheet(r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#),
        &shared(&["<t>Shop</t>"]),
    );
    assert_eq!(read(&plain).0, None, "the same workbook with no bomb reads");
}

/// A billion-laughs DTD in a sheet and in the shared strings stops at its `<!DOCTYPE`, before
/// a single entity is declared. An entity used with no DTD at all throws naming it, so no text
/// is ever the expansion of anything but the five XML predefines.
#[test]
fn an_entity_is_never_expanded_and_no_dtd_is_loaded() {
    let mut laughs = String::from(r#"<!DOCTYPE lolz [<!ENTITY lol0 "lol">"#);
    for n in 1..10 {
        let previous = format!("&lol{};", n - 1);
        laughs.push_str(&format!(r#"<!ENTITY lol{n} "{}">"#, previous.repeat(10)));
    }
    laughs.push(']');
    laughs.push('>');
    let row = r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#;
    let in_sheet = declared(
        &laughs,
        &sheet(r#"<row r="1"><c r="A1" t="inlineStr"><is><t>&lol9;</t></is></c></row>"#),
    );
    let in_strings = declared(&laughs, &shared(&["<t>&lol9;</t>"]));
    for (what, file) in [
        ("a sheet", xlsx(&in_sheet, &shared(&[]))),
        ("the shared strings", xlsx(&sheet(row), &in_strings)),
    ] {
        let failure = read(&file)
            .0
            .unwrap_or_else(|| panic!("a DTD in {what} reads"));
        assert!(
            failure.contains("document type declaration"),
            "{what}: {failure}"
        );
    }

    for (what, file) in [
        (
            "a sheet",
            xlsx(
                &sheet(r#"<row r="1"><c r="A1" t="inlineStr"><is><t>&lol9;</t></is></c></row>"#),
                &shared(&[]),
            ),
        ),
        (
            "the shared strings",
            xlsx(&sheet(row), &shared(&["<t>&lol9;</t>"])),
        ),
    ] {
        let failure = read(&file)
            .0
            .unwrap_or_else(|| panic!("an entity in {what} reads"));
        assert!(failure.contains("the entity `lol9`"), "{what}: {failure}");
    }
}

/// An external entity naming a local file or the cloud metadata address is refused at its
/// `<!DOCTYPE`, and its reference alone throws naming it. The guest sends no request and the
/// failure carries nothing the file or the URL holds.
#[test]
fn an_external_entity_naming_a_file_or_a_url_reads_nothing() {
    for target in [
        "file:///etc/passwd",
        "http://169.254.169.254/latest/meta-data/",
    ] {
        let dtd = format!(r#"<!DOCTYPE r [<!ENTITY xxe SYSTEM "{target}">]>"#);
        let cell = r#"<row r="1"><c r="A1" t="inlineStr"><is><t>&xxe;</t></is></c></row>"#;
        let row = r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#;
        for (what, file) in [
            ("a sheet", xlsx(&declared(&dtd, &sheet(cell)), &shared(&[]))),
            (
                "the shared strings",
                xlsx(&sheet(row), &declared(&dtd, &shared(&["<t>&xxe;</t>"]))),
            ),
        ] {
            let (failure, sent) = read(&file);
            let failure = failure.unwrap_or_else(|| panic!("{target} in {what} reads"));
            assert!(
                failure.contains("document type declaration"),
                "{target} in {what}: {failure}"
            );
            assert!(!failure.contains("root:"), "{failure}");
            assert_eq!(sent, 0, "{target} in {what} sent a request");
        }

        let (failure, sent) = read(&xlsx(&sheet(cell), &shared(&[])));
        let failure = failure.unwrap_or_else(|| panic!("`&xxe;` reads"));
        assert!(failure.contains("the entity `xxe`"), "{failure}");
        assert_eq!(sent, 0);
    }
}

/// A cell past `XFD1048576` throws naming it, a row past 1,048,576 with no cell throws naming
/// the row, and the last cell of the grid reads.
#[test]
fn a_row_index_past_the_grid_throws_naming_the_cell() {
    let past = |data: &str| {
        read(&xlsx(&sheet(data), &shared(&[])))
            .0
            .unwrap_or_else(|| panic!("{data} reads"))
    };
    let failure = past(r#"<row r="2147483648"><c r="A2147483648"><v>1</v></c></row>"#);
    assert!(failure.contains("`A2147483648`"), "{failure}");
    let failure = past(r#"<row r="1048577"><c><v>1</v></c></row>"#);
    assert!(failure.contains("`A1048577`"), "{failure}");
    let failure = past(r#"<row r="1"><c r="XFE1"><v>1</v></c></row>"#);
    assert!(failure.contains("`XFE1`"), "{failure}");
    let failure = past(r#"<row r="2147483648"/>"#);
    assert!(failure.contains("the row 2147483648"), "{failure}");

    let last = xlsx(
        &sheet(r#"<row r="1048576"><c r="XFD1048576"><v>1</v></c></row>"#),
        &shared(&[]),
    );
    assert_eq!(read(&last).0, None, "the last cell of the grid reads");

    let past = |data: Vec<u8>| read(&xlsb(&data)).0.expect("an xlsb past the grid reads");
    let failure = past(binary_row(u32::MAX));
    assert!(failure.contains("the row 4294967296"), "{failure}");
    let failure = past([binary_row(0), binary_real(16_384)].concat());
    assert!(failure.contains("`XFE1`"), "{failure}");
    let last = [binary_row(1_048_575), binary_real(16_383)].concat();
    assert_eq!(read(&xlsb(&last)).0, None, "the last cell of an xlsb reads");
}

/// `n` in 7-bit groups, lowest first, as an xlsb record's type and length are written.
fn groups(mut n: usize) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let low = u8::try_from(n & 0x7F).unwrap();
        n >>= 7;
        out.push(if n == 0 { low } else { low | 0x80 });
        if n == 0 {
            return out;
        }
    }
}

/// One xlsb record of `kind` whose length is the true length of `body`.
fn record(kind: u16, body: &[u8]) -> Vec<u8> {
    [groups(usize::from(kind)), groups(body.len()), body.to_vec()].concat()
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

/// An xlsb row header for the row `index`, from 0.
fn binary_row(index: u32) -> Vec<u8> {
    record(0, &[&index.to_le_bytes()[..], &[0; 13]].concat())
}

/// An xlsb number cell holding 1 in `column`, from 0.
fn binary_real(column: u32) -> Vec<u8> {
    let body = [&column.to_le_bytes()[..], &[0; 4], &1f64.to_le_bytes()].concat();
    record(5, &body)
}

/// An xlsb of one sheet, `Orders`, whose sheet data is the bytes `data`.
fn xlsb(data: &[u8]) -> Vec<u8> {
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let package = "http://schemas.openxmlformats.org/package/2006/relationships";
    let root = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.bin"/></Relationships>"#
    );
    let rels = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/worksheet" Target="worksheets/sheet1.bin"/></Relationships>"#
    );
    let book = record(
        156,
        &[
            &[0u8; 4][..],
            &1u32.to_le_bytes(),
            &wide("rId1"),
            &wide("Orders"),
        ]
        .concat(),
    );
    let part = [record(145, &[]), data.to_vec(), record(146, &[])].concat();
    zip(&[
        ("[Content_Types].xml", b"<Types/>"),
        ("_rels/.rels", root.as_bytes()),
        ("xl/workbook.bin", &book),
        ("xl/_rels/workbook.bin.rels", rels.as_bytes()),
        ("xl/worksheets/sheet1.bin", &part),
    ])
}

/// A record whose length runs past the end of its part, one whose length is shorter than its
/// fields, and one whose length takes five bytes each throw naming the fault. The longest length
/// a record can claim reads no further than the part's end.
#[test]
fn an_xlsb_record_whose_length_lies_throws() {
    let plain = [binary_row(0), binary_real(0)].concat();
    assert_eq!(
        read(&xlsb(&plain)).0,
        None,
        "the same sheet with no lie reads"
    );

    let past_the_end = [binary_row(0), vec![5, 0xFF, 0xFF, 0xFF, 0x7F], vec![0; 16]].concat();
    let short = [binary_row(0), record(5, &[0; 4])].concat();
    let five_bytes = [binary_row(0), vec![5, 0x80, 0x80, 0x80, 0x80, 0x01]].concat();
    for (what, data, fault) in [
        ("past the end", past_the_end, "ends inside a record"),
        ("short", short, "shorter than its fields"),
        ("five bytes long", five_bytes, "longer than four bytes"),
    ] {
        let failure = read(&xlsb(&data))
            .0
            .unwrap_or_else(|| panic!("a length {what} reads"));
        assert!(failure.contains(fault), "{what}: {failure}");
    }
}

/// A compound file of 512-byte sectors: the header, the sector table `table` in sector 0, the
/// directory in sector 1, and the stream `Workbook` of 8,192 bytes starting at sector 2.
fn compound(table: &[u32]) -> Vec<u8> {
    compound_of(table, "Workbook", &[])
}

/// [`compound`] whose stream is named `name` and starts with `content`, the rest zeros.
fn compound_of(table: &[u32], name: &str, content: &[u8]) -> Vec<u8> {
    const NONE: u32 = 0xFFFF_FFFF;
    const END: u32 = 0xFFFF_FFFE;
    let mut header = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    header.resize(512, 0);
    let mut put = |at: usize, value: u32| header[at..at + 4].copy_from_slice(&value.to_le_bytes());
    // The version 3 and byte order, then the sector size 2^9 and the mini sector size 2^6.
    put(24, 0x0003_003E);
    put(28, 0x0009_FFFE);
    put(32, 6);
    put(44, 1);
    put(48, 1);
    put(56, 4096);
    put(60, END);
    put(68, END);
    put(76, 0);
    for at in 1..109 {
        put(76 + at * 4, NONE);
    }
    let mut sectors: Vec<u32> = table.to_vec();
    sectors.resize(128, NONE);
    let fat: Vec<u8> = sectors.iter().flat_map(|id| id.to_le_bytes()).collect();
    let entry = |name: &str, kind: u8, child: u32, start: u32, size: u32| {
        let mut out = vec![0u8; 128];
        let units: Vec<u8> = name
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        out[..units.len()].copy_from_slice(&units);
        out[64..66].copy_from_slice(&u16::try_from(units.len()).unwrap().to_le_bytes());
        out[66] = kind;
        for (at, value) in [
            (68, NONE),
            (72, NONE),
            (76, child),
            (116, start),
            (120, size),
        ] {
            out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        out
    };
    let mut directory = entry("Root Entry", 5, 1, END, 0);
    directory.extend(entry(name, 2, NONE, 2, 8192));
    directory.resize(512, 0);
    let mut stream = content.to_vec();
    stream.resize(512 * 16, 0);
    [header, fat, directory, stream].concat()
}

/// A sector chain that comes back to a sector it already read is refused at the repeat, in a
/// stream and in the directory. The same file with an honest chain gets past the compound file,
/// and throws only because its stream is not a workbook.
#[test]
fn an_xls_sector_chain_that_loops_is_refused() {
    const END: u32 = 0xFFFF_FFFE;
    const TABLE: u32 = 0xFFFF_FFFD;
    let honest: Vec<u32> = [TABLE, END].into_iter().chain(3..18).chain([END]).collect();
    let failure = read(&compound(&honest))
        .0
        .expect("zeros read as a workbook");
    assert!(!failure.contains("sector"), "{failure}");

    let mut in_stream = honest.clone();
    in_stream[3] = 2;
    let mut in_directory = honest;
    in_directory[1] = 1;
    for (what, table) in [("a stream", in_stream), ("the directory", in_directory)] {
        let failure = read(&compound(&table))
            .0
            .unwrap_or_else(|| panic!("a loop in {what} reads"));
        assert!(
            failure.contains("a sector chain loops"),
            "{what}: {failure}"
        );
    }
}

/// A merged range with a corner past the grid throws when its ranges are listed and when a
/// read fills merged cells.
#[test]
fn a_merged_range_past_the_grid_throws() {
    for range in ["A1:A2147483648", "A1:XFE1", "A2147483648"] {
        let part = format!(
            r#"<?xml version="1.0"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData><mergeCells><mergeCell ref="{range}"/></mergeCells></worksheet>"#
        );
        let file = xlsx(part.as_bytes(), &shared(&[]));
        assert_eq!(
            read(&file).0,
            None,
            "{range}: a read that fills nothing reads"
        );
        let failure = read_filling(&file, true)
            .0
            .unwrap_or_else(|| panic!("{range} fills"));
        assert!(
            failure.contains("merged range is past"),
            "{range}: {failure}"
        );

        let host = Host::new(2, |_| Ok(())).expect("the host starts");
        let extension = spreadsheet(&host);
        let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), Some(CAP))));
        let source = Value::Bytes(file);
        let reader = block_on(request.call_values(
            &extension,
            "readerOpen",
            vec![source.clone(), bound(None, None)],
        ))
        .expect("the workbook opens")
        .expect("`readerOpen` returns a reader");
        let failure = block_on(request.call_values(
            &extension,
            "readerMerged",
            vec![reader, source, Value::String("Orders".to_owned())],
        ))
        .expect_err("the range is listed")
        .to_string();
        assert!(
            failure.contains("merged range is past"),
            "{range}: {failure}"
        );
        drop(block_on(request.end()));
    }
}

/// A shape of `fields`, as a Novis shape crosses.
fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(key, value)| (Key::String(key.to_owned()), value))
            .collect(),
    )
}

/// A list of `values`, keyed from 0.
fn list(values: Vec<Value>) -> Value {
    Value::Array(
        values
            .into_iter()
            .enumerate()
            .map(|(at, value)| (Key::Int(i64::try_from(at).unwrap()), value))
            .collect(),
    )
}

/// A text cell's value.
fn text(text: &str) -> Value {
    shape(vec![("text", Value::String(text.to_owned()))])
}

/// A batch of `count` items of the list `items`, each filling `name`.
fn items(from: usize, count: usize) -> Value {
    let items = (from..from + count)
        .map(|n| {
            list(vec![shape(vec![
                ("path", Value::String("name".to_owned())),
                ("value", text(&format!("Shop item number {n}"))),
            ])])
        })
        .collect();
    shape(vec![
        ("name", Value::String("items".to_owned())),
        ("items", list(items)),
    ])
}

/// A template whose row 2 repeats for the list `items`, below the rows `above`, with the
/// defined names `names`.
fn template(above: &str, names: &str) -> Vec<u8> {
    let part = sheet(&format!(
        r#"{above}<row r="2"><c r="A2" t="inlineStr"><is><t>{{{{items.name}}}}</t></is></c></row>"#
    ));
    xlsx_named(&part, &shared(&[]), names)
}

/// `template` opened and filled with no values under `request`, ready for its items.
fn filling(request: &Request, extension: &Extension, template: &[u8]) -> Value {
    let source = Value::Bytes(template.to_vec());
    let filling =
        block_on(request.call_values(extension, "templateOpen", vec![source, bound(None, None)]))
            .expect("the template opens")
            .expect("`templateOpen` returns a filling");
    block_on(request.call_values(
        extension,
        "templateFill",
        vec![
            filling.clone(),
            list(Vec::new()),
            list(vec![Value::String("items".to_owned())]),
        ],
    ))
    .expect("the template fills");
    filling
}

/// A formula above the block names its row, so the template keeps every item until `finish`.
/// Ten million items reach the request's memory cap long before the last one, the request ends
/// with its memory given back, and the next request on the same host fills the same template.
#[test]
fn a_template_list_past_the_memory_cap_is_a_contained_trap() {
    const SMALL: u64 = 48 << 20;
    const BATCH: usize = 10_000;
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let held = template(
        r#"<row r="1"><c r="A1"><f>SUM(A2:A2)</f><v>0</v></c></row>"#,
        "",
    );
    let budget = Arc::new(Meter::new(Duration::from_secs(120), Some(SMALL)));
    let request = host.request(budget.clone());
    let filled = filling(&request, &extension, &held);
    let mut sent = 0;
    let failure = loop {
        assert!(sent < 10_000_000, "ten million items fit under the cap");
        match block_on(request.call_values(
            &extension,
            "templateItems",
            vec![filled.clone(), items(sent, BATCH)],
        )) {
            Ok(_) => sent += BATCH,
            Err(failure) => break failure,
        }
    };
    assert_eq!(failure, Failure::Limit(Limit::Memory), "after {sent} items");
    drop(block_on(request.end()));
    assert_eq!(
        budget.charged(),
        0,
        "the trapped request gives its memory back"
    );

    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), Some(SMALL))));
    let filled = filling(&request, &extension, &held);
    block_on(request.call_values(
        &extension,
        "templateItems",
        vec![filled.clone(), items(0, 3)],
    ))
    .expect("three items fit");
    block_on(request.call_values(
        &extension,
        "templateFinish",
        vec![filled, Value::Bytes(held)],
    ))
    .expect("the next request finishes the same template");
    drop(block_on(request.end()));
}

/// A block of 1,000 rows repeated past row 1,048,576 throws naming the list, before the item
/// that would cross the last row is written.
#[test]
fn a_template_list_past_the_last_row_throws_naming_it() {
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let tall = template(
        "",
        r#"<definedName name="items">Orders!$A$2:$A$1001</definedName>"#,
    );
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(120), Some(CAP))));
    let filled = filling(&request, &extension, &tall);
    let failure =
        block_on(request.call_values(&extension, "templateItems", vec![filled, items(0, 1_100)]))
            .expect_err("1,100 copies of 1,000 rows fit on one sheet")
            .to_string();
    assert!(failure.contains("The list `items`"), "{failure}");
    assert!(
        failure.contains("Item 1049 would end on row 1049001"),
        "{failure}"
    );
    drop(block_on(request.end()));
}

/// The text of the entry `name` in the zip archive `archive`, found through its central
/// directory and inflated.
fn entry(archive: &[u8], name: &str) -> String {
    let u16_at = |at: usize| usize::from(u16::from_le_bytes([archive[at], archive[at + 1]]));
    let u32_at = |at: usize| u32::from_le_bytes(archive[at..at + 4].try_into().unwrap()) as usize;
    let end = archive.len() - 22;
    let mut at = u32_at(end + 16);
    for _ in 0..u16_at(end + 10) {
        let length = u16_at(at + 28);
        let local = u32_at(at + 42);
        if &archive[at + 46..at + 46 + length] == name.as_bytes() {
            let data = local + 30 + u16_at(local + 26) + u16_at(local + 28);
            let mut text = String::new();
            flate2::read::DeflateDecoder::new(&archive[data..data + u32_at(at + 20)])
                .read_to_string(&mut text)
                .expect("the entry inflates");
            return text;
        }
        at += 46 + length + u16_at(at + 30) + u16_at(at + 32);
    }
    panic!("the archive has no `{name}`");
}

/// A string that begins as a formula would is written as a text cell holding that string, so
/// a spreadsheet program shows it and never runs it.
#[test]
fn an_export_string_beginning_with_equals_is_a_text_cell() {
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), Some(CAP))));
    let call = |method: &str, args: Vec<Value>| {
        block_on(request.call_values(&extension, method, args))
            .unwrap_or_else(|err| panic!("`{method}`: {err}"))
            .unwrap_or(Value::Null)
    };
    let properties = [
        "title", "subject", "author", "company", "keywords", "created",
    ]
    .map(|key| (key, Value::Null))
    .to_vec();
    let writer = call(
        "writerOpen",
        vec![shape(vec![
            ("properties", shape(properties)),
            ("target", Value::String("orders.xlsx".to_owned())),
            ("ods", Value::Bool(false)),
        ])],
    );
    let hostile = [
        r#"=HYPERLINK("http://example.com/","Open")"#,
        "+1+1",
        "-1+1",
        "@SUM(1)",
    ];
    let cells = hostile
        .iter()
        .map(|value| shape(vec![("value", text(value)), ("style", Value::Null)]))
        .collect();
    let row = shape(vec![
        ("cells", list(cells)),
        ("style", Value::Null),
        ("height", Value::Null),
        ("hidden", Value::Bool(false)),
        ("level", Value::Uint(0)),
        ("header", Value::Bool(false)),
    ]);
    let options = shape(vec![
        ("name", Value::String("Orders".to_owned())),
        ("position", Value::Null),
        ("visibility", Value::Case("Visible".to_owned())),
        ("columns", list(Vec::new())),
        ("freezeRows", Value::Uint(0)),
        ("freezeColumns", Value::Uint(0)),
        ("autofilter", Value::Null),
        ("tabColor", Value::Null),
        ("zoom", Value::Null),
        ("gridlines", Value::Bool(true)),
        ("rightToLeft", Value::Bool(false)),
        (
            "header",
            shape(vec![
                ("defaultStyle", Value::Bool(false)),
                ("style", Value::Null),
                ("freeze", Value::Bool(false)),
                ("filter", Value::Bool(false)),
            ]),
        ),
        ("newSheetOnOverflow", Value::Bool(false)),
    ]);
    let Value::Bytes(mut archive) = call(
        "writerSheet",
        vec![writer.clone(), options, list(vec![row])],
    ) else {
        panic!("a sheet's first chunk is bytes");
    };
    let finished = call("writerFinish", vec![writer]);
    let Value::Array(fields) = finished else {
        panic!("`writerFinish` returns a shape");
    };
    let Some((_, Value::Bytes(last))) = fields
        .into_iter()
        .find(|(key, _)| *key == Key::String("data".to_owned()))
    else {
        panic!("`writerFinish` returns its last bytes");
    };
    archive.extend(last);
    drop(block_on(request.end()));

    let part = entry(&archive, "xl/worksheets/sheet1.xml");
    assert!(!part.contains("<f>") && !part.contains("<f "), "{part}");
    assert_eq!(
        part.matches(r#"t="inlineStr""#).count(),
        hostile.len(),
        "{part}"
    );
    assert!(part.contains("HYPERLINK("), "{part}");
}

/// One valued cell repeated over all 16,384 columns, in a row repeated to the grid's last row,
/// is 2^34 cells in a few hundred bytes. Kept once and given again by the cursor, it opens and
/// reads under [`CAP`]. Cloned into a stored row per repeat, it ran out of memory at the open.
#[test]
fn an_ods_row_repeated_to_the_last_row_costs_one_row_to_read() {
    let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:body><office:spreadsheet><table:table table:name="Orders">
<table:table-row table:number-rows-repeated="1048575"><table:table-cell office:value-type="float" office:value="1" table:number-columns-repeated="16384"/></table:table-row>
<table:table-row><table:table-cell office:value-type="float" office:value="2"/></table:table-row>
</table:table></office:spreadsheet></office:body></office:document-content>"#;
    let file = zip(&[
        (
            "mimetype",
            b"application/vnd.oasis.opendocument.spreadsheet".as_slice(),
        ),
        ("content.xml", content.as_bytes()),
    ]);
    assert_eq!(
        first_rows(&file, 3),
        Ok(vec![(1, 16_384), (2, 16_384), (3, 16_384)])
    );
}

/// The letters of the column `column`, from 1.
fn letters(mut column: u32) -> String {
    let mut out = Vec::new();
    while column > 0 {
        column -= 1;
        out.push(b'A' + u8::try_from(column % 26).expect("under 26"));
        column /= 26;
    }
    out.reverse();
    String::from_utf8(out).expect("letters are ASCII")
}

/// A text of 2 MiB and one of 16 MiB, each one token across many 64 KiB read blocks, read at most
/// four times as slowly per byte. Rescanned from its start at every refill, the larger one took
/// about 64 times as long.
#[test]
fn one_token_longer_than_many_read_blocks_is_scanned_once() {
    let book = |bytes: usize| {
        let text = noise(bytes, 7);
        xlsx(
            &sheet(&format!(
                r#"<row r="1"><c r="A1" t="inlineStr"><is><t>{text}</t></is></c></row>"#
            )),
            &shared(&[]),
        )
    };
    linear(&book(2 << 20), &book(16 << 20), "one long text");
}

/// A shared-strings table of 2 MB, read under a server bound of 1 MiB, throws naming the part
/// and the setting, as a part read whole does. Under 4 MiB, it reads.
#[test]
fn the_shared_string_table_counts_against_the_archive_ceiling() {
    let items: Vec<String> = (0..2000)
        .map(|at| format!("<t>{}</t>", noise(1000, at + 1)))
        .collect();
    let items: Vec<&str> = items.iter().map(String::as_str).collect();
    let file = xlsx(
        &sheet(r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#),
        &shared(&items),
    );
    let under = |bytes: u64| first_rows_under(&file, 1, Decompression { bytes, ..SHIPPED });
    let failure = under(1 << 20).expect_err("2 MB of strings is past a 1 MiB ceiling");
    assert!(failure.contains("xl/sharedStrings.xml"), "{failure}");
    assert!(failure.contains("max_decompressed"), "{failure}");
    assert_eq!(under(4 << 20), Ok(vec![(1, 1)]));
}

/// A row of 2,048 and one of 16,384 shared-formula masters, each its own group, with a row of
/// their followers below, read at most four times as slowly per cell. Found by a search of the
/// row per group, the larger one took about 64 times as long.
#[test]
fn many_shared_formula_groups_in_a_row_read_in_linear_time() {
    let book = |columns: u32| {
        let (mut first, mut second) = (String::new(), String::new());
        for at in 1..=columns {
            let column = letters(at);
            let si = at - 1;
            first.push_str(&format!(
                r#"<c r="{column}1"><f t="shared" ref="{column}1:{column}2" si="{si}">{column}3+1</f><v>1</v></c>"#
            ));
            second.push_str(&format!(
                r#"<c r="{column}2"><f t="shared" si="{si}"/><v>1</v></c>"#
            ));
        }
        xlsx(
            &sheet(&format!(
                r#"<row r="1">{first}</row><row r="2">{second}</row>"#
            )),
            &shared(&[]),
        )
    };
    linear(&book(2048), &book(16_384), "shared-formula groups");
}

/// A styles part of 2,048 and one of 16,384 custom number formats, each the format of one cell
/// style, opened and read at most four times as slowly per format. Found by a search of every
/// code per style, the work grew with the square of the size.
#[test]
fn many_custom_number_formats_read_in_linear_time() {
    let book = |count: usize| {
        let codes = vec!["0.00".to_owned(); count];
        let last = count - 1;
        xlsx_with(
            &sheet(&format!(
                r#"<row r="1"><c r="A1" s="{last}"><v>1.5</v></c></row>"#
            )),
            &shared(&[]),
            "",
            Some(&styles(&codes)),
        )
    };
    linear(&book(2048), &book(16_384), "custom number formats");
}

/// A format code of 32 KiB and one of 256 KiB, made of the letters `g` and `a`, read at most
/// four times as slowly per byte. The format is read when the file opens. Copied from each
/// letter to the end of the code, the work grew with the square of the size.
#[test]
fn a_long_format_code_with_many_g_and_a_letters_renders_in_linear_time() {
    let book = |bytes: usize| {
        let code = "ga".repeat(bytes / 2);
        xlsx_with(
            &sheet(r#"<row r="1"><c r="A1" s="0"><v>1.5</v></c></row>"#),
            &shared(&[]),
            "",
            Some(&styles(&[code])),
        )
    };
    linear(&book(32 << 10), &book(256 << 10), "a long format code");
}

/// The time `readerEvaluate` takes over `file`, from the open to its result, under [`CAP`].
/// Every formula is computed.
fn evaluate_time(file: &[u8]) -> Duration {
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let started = std::time::Instant::now();
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(120), Some(CAP))));
    let source = Value::Bytes(file.to_vec());
    let missed = (|| {
        let reader = block_on(request.call_values(
            &extension,
            "readerOpen",
            vec![source.clone(), bound(None, None)],
        ))
        .map_err(|err| err.to_string())?
        .expect("`readerOpen` returns a reader");
        block_on(request.call_values(&extension, "readerEvaluate", vec![reader, source.clone()]))
            .map_err(|err| err.to_string())
    })();
    let took = started.elapsed();
    drop(block_on(request.end()));
    match missed {
        Ok(Some(Value::Array(missed))) => assert!(missed.is_empty(), "missed {missed:?}"),
        other => panic!("the workbook evaluates: {other:?}"),
    }
    took
}

/// 1,000 and 8,000 rows, each a value in `B` and a formula in `C` that adds up column `A`, which
/// has one cell. Evaluated at most four times as slowly per row. When each `A:A` walked every
/// stored cell of the sheet, the work grew with the square of the size.
#[test]
fn a_whole_column_reference_visits_only_that_column() {
    let book = |rows: u32| {
        let mut data = String::from(
            r#"<row r="1"><c r="A1"><v>1</v></c><c r="B1"><v>1</v></c><c r="C1"><f>SUM(A:A)</f><v>0</v></c></row>"#,
        );
        for row in 2..=rows {
            data.push_str(&format!(
                r#"<row r="{row}"><c r="B{row}"><v>1</v></c><c r="C{row}"><f>SUM(A:A)</f><v>0</v></c></row>"#
            ));
        }
        xlsx(&sheet(&data), &shared(&[]))
    };
    linear_by(
        &book(1000),
        &book(8000),
        "whole-column references",
        evaluate_time,
    );
}

/// An xlsx saved with a password is a compound file with an `EncryptedPackage` stream. An xls
/// saved with a password has a `FILEPASS` record after its first `BOF`, and every record after it
/// is cipher text. Both throw that the file is encrypted, and neither is read as a workbook.
#[test]
fn an_encrypted_xlsx_and_an_xls_with_a_password_throw_that_encryption_is_not_supported() {
    const END: u32 = 0xFFFF_FFFE;
    const TABLE: u32 = 0xFFFF_FFFD;
    let table: Vec<u32> = [TABLE, END].into_iter().chain(3..18).chain([END]).collect();
    let encrypted_xlsx = compound_of(&table, "EncryptedPackage", &[0x5A; 64]);
    let mut stream = Vec::new();
    let mut record = |kind: u16, body: &[u8]| {
        stream.extend(kind.to_le_bytes());
        stream.extend(
            u16::try_from(body.len())
                .expect("a short record")
                .to_le_bytes(),
        );
        stream.extend(body);
    };
    let mut bof = vec![0u8; 16];
    bof[..2].copy_from_slice(&0x0600u16.to_le_bytes());
    bof[2..4].copy_from_slice(&0x0005u16.to_le_bytes());
    record(0x0809, &bof);
    // FILEPASS: RC4 encryption, then its salt and verifier.
    let mut pass = vec![0x01, 0x00, 0x01, 0x00, 0x01, 0x00];
    pass.extend([0x5A; 48]);
    record(0x002F, &pass);
    record(0x0085, &[0x5A; 16]);
    let protected_xls = compound_of(&table, "Workbook", &stream);
    for (what, file) in [("xlsx", encrypted_xlsx), ("xls", protected_xls)] {
        let failure = read(&file).0.unwrap_or_else(|| panic!("the {what} reads"));
        assert!(
            failure.contains(
                "This file is encrypted with a password. Encrypted files are not supported."
            ),
            "{what}: {failure}"
        );
    }
}

/// The bytes in the field `data` of the record `value`.
fn data_of(value: Value) -> Vec<u8> {
    let Value::Array(fields) = value else {
        panic!("a record is a shape");
    };
    match fields
        .into_iter()
        .find(|(key, _)| *key == Key::String("data".to_owned()))
    {
        Some((_, Value::Bytes(data))) => data,
        _ => panic!("the record has its bytes in `data`"),
    }
}

/// The names of the entries in the zip archive `archive`, from its central directory.
fn entry_names(archive: &[u8]) -> Vec<String> {
    let u16_at = |at: usize| usize::from(u16::from_le_bytes([archive[at], archive[at + 1]]));
    let end = archive.len() - 22;
    let mut at = u32::from_le_bytes(archive[end + 16..end + 20].try_into().unwrap()) as usize;
    let mut names = Vec::new();
    for _ in 0..u16_at(end + 10) {
        let length = u16_at(at + 28);
        names.push(String::from_utf8_lossy(&archive[at + 46..at + 46 + length]).into_owned());
        at += 46 + length + u16_at(at + 30) + u16_at(at + 32);
    }
    names
}

/// A template with an XLM macro sheet, VBA data, an ActiveX control, a ribbon and an OLE object
/// is filled. None of them is in the result, and nothing names them: not the content types, the
/// relationships, the workbook or the sheet. The embedded xlsx and the image are kept, and the
/// defined names and the view move to the sheets that are left.
#[test]
fn a_filled_template_drops_macro_sheets_activex_ribbons_and_ole_objects_and_keeps_images() {
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let package = "http://schemas.openxmlformats.org/package/2006/relationships";
    let mc = "http://schemas.openxmlformats.org/markup-compatibility/2006";
    let types = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="png" ContentType="image/png"/><Default Extension="xlsx" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"/><Default Extension="bin" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.printerSettings"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.ms-excel.sheet.macroEnabled.main+xml"/><Override PartName="/xl/macrosheets/sheet1.xml" ContentType="application/vnd.ms-excel.macrosheet+xml"/><Override PartName="/xl/vbaData.xml" ContentType="application/vnd.ms-word.vbaData+xml"/><Override PartName="/xl/activeX/activeX1.xml" ContentType="application/vnd.ms-office.activeX+xml"/><Override PartName="/xl/activeX/activeX1.bin" ContentType="application/vnd.ms-office.activeX"/><Override PartName="/xl/embeddings/oleObject1.bin" ContentType="application/vnd.openxmlformats-officedocument.oleObject"/><Override PartName="/customUI/customUI14.xml" ContentType="application/xml"/></Types>"#;
    let root = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/><Relationship Id="rId2" Type="http://schemas.microsoft.com/office/2007/relationships/ui/extensibility" Target="customUI/customUI14.xml"/></Relationships>"#
    );
    let book = format!(
        r#"<workbook xmlns="{main}" xmlns:r="{rel}"><bookViews><workbookView firstSheet="2" activeTab="2"/></bookViews><sheets><sheet name="Orders" sheetId="1" r:id="rId1"/><sheet name="Macro1" sheetId="2" r:id="rId5"/><sheet name="Summary" sheetId="3" r:id="rId6"/></sheets><definedNames><definedName name="_xlnm.Auto_Open">Macro1!$A$1</definedName><definedName name="start" localSheetId="1">Macro1!$B$1</definedName><definedName name="total" localSheetId="2">Summary!$A$1</definedName></definedNames></workbook>"#
    );
    let book_rels = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{rel}/sharedStrings" Target="sharedStrings.xml"/><Relationship Id="rId5" Type="http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet" Target="macrosheets/sheet1.xml"/><Relationship Id="rId6" Type="{rel}/worksheet" Target="worksheets/sheet2.xml"/></Relationships>"#
    );
    let orders = format!(
        r#"<worksheet xmlns="{main}" xmlns:r="{rel}" xmlns:mc="{mc}"><sheetData><row r="2"><c r="A2" t="inlineStr"><is><t>{{{{items.name}}}}</t></is></c></row></sheetData><drawing r:id="rId1"/><oleObjects><mc:AlternateContent><mc:Choice Requires="x14"><oleObject progId="Package" shapeId="1026" r:id="rId4"><objectPr r:id="rId6"/></oleObject></mc:Choice><mc:Fallback><oleObject progId="Package" shapeId="1026" r:id="rId4"/></mc:Fallback></mc:AlternateContent><oleObject progId="Excel.Sheet.12" shapeId="1027" r:id="rId5"/></oleObjects><mc:AlternateContent><mc:Choice Requires="x14"><controls><mc:AlternateContent><mc:Choice Requires="x14"><control shapeId="1025" r:id="rId3" name="Order"><controlPr/></control></mc:Choice><mc:Fallback><control shapeId="1025" r:id="rId3" name="Order"/></mc:Fallback></mc:AlternateContent></controls></mc:Choice></mc:AlternateContent></worksheet>"#
    );
    let orders_rels = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/drawing" Target="../drawings/drawing1.xml"/><Relationship Id="rId3" Type="{rel}/control" Target="../activeX/activeX1.xml"/><Relationship Id="rId4" Type="{rel}/oleObject" Target="../embeddings/oleObject1.bin"/><Relationship Id="rId5" Type="{rel}/package" Target="../embeddings/Shop.xlsx"/><Relationship Id="rId6" Type="{rel}/image" Target="../media/image1.png"/></Relationships>"#
    );
    let summary = sheet(r#"<row r="1"><c r="A1"><v>1</v></c></row>"#);
    let macro_sheet = format!(r#"<xm:macrosheet xmlns:xm="{main}"><sheetData/></xm:macrosheet>"#);
    let strings = shared(&[]);
    let template = zip(&[
        ("[Content_Types].xml", types.as_bytes()),
        ("_rels/.rels", root.as_bytes()),
        ("customUI/customUI14.xml", b"<customUI/>"),
        ("xl/workbook.xml", book.as_bytes()),
        ("xl/_rels/workbook.xml.rels", book_rels.as_bytes()),
        ("xl/sharedStrings.xml", &strings),
        ("xl/worksheets/sheet1.xml", orders.as_bytes()),
        ("xl/worksheets/_rels/sheet1.xml.rels", orders_rels.as_bytes()),
        ("xl/worksheets/sheet2.xml", &summary),
        ("xl/macrosheets/sheet1.xml", macro_sheet.as_bytes()),
        ("xl/vbaData.xml", b"<wne:vbaSuppData/>"),
        ("xl/activeX/activeX1.xml", b"<ax:ocx/>"),
        ("xl/activeX/_rels/activeX1.xml.rels", b"<Relationships/>"),
        ("xl/activeX/activeX1.bin", b"control"),
        ("xl/embeddings/oleObject1.bin", b"ole object"),
        ("xl/embeddings/Shop.xlsx", b"PK embedded"),
        ("xl/media/image1.png", b"\x89PNG\r\n\x1a\n"),
        ("xl/drawings/drawing1.xml", b"<xdr:wsDr/>"),
    ]);

    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), Some(CAP))));
    let call = |method: &str, args: Vec<Value>| {
        block_on(request.call_values(&extension, method, args))
            .unwrap_or_else(|err| panic!("`{method}`: {err}"))
            .unwrap_or(Value::Null)
    };
    let filled = call(
        "templateOpen",
        vec![Value::Bytes(template.clone()), bound(None, None)],
    );
    let mut archive = data_of(call(
        "templateFill",
        vec![
            filled.clone(),
            list(Vec::new()),
            list(vec![Value::String("items".to_owned())]),
        ],
    ));
    let Value::Bytes(batch) = call("templateItems", vec![filled.clone(), items(0, 2)]) else {
        panic!("`templateItems` returns bytes");
    };
    archive.extend(batch);
    archive.extend(data_of(call(
        "templateFinish",
        vec![filled, Value::Bytes(template)],
    )));
    drop(block_on(request.end()));

    let names = entry_names(&archive);
    for gone in [
        "customUI/customUI14.xml",
        "xl/macrosheets/sheet1.xml",
        "xl/vbaData.xml",
        "xl/activeX/activeX1.xml",
        "xl/activeX/_rels/activeX1.xml.rels",
        "xl/activeX/activeX1.bin",
        "xl/embeddings/oleObject1.bin",
    ] {
        assert!(!names.iter().any(|name| name == gone), "{gone}: {names:?}");
    }
    for kept in [
        "xl/embeddings/Shop.xlsx",
        "xl/media/image1.png",
        "xl/drawings/drawing1.xml",
        "xl/worksheets/sheet2.xml",
    ] {
        assert!(names.iter().any(|name| name == kept), "{kept}: {names:?}");
    }

    let types = entry(&archive, "[Content_Types].xml");
    for gone in [
        "macrosheet",
        "vbaData",
        "activeX1",
        "oleObject1",
        "customUI",
        "macroEnabled",
    ] {
        assert!(!types.contains(gone), "{gone}: {types}");
    }
    assert!(types.contains(r#"Extension="bin""#), "{types}");
    let root = entry(&archive, "_rels/.rels");
    assert!(
        !root.contains("customUI") && root.contains("officeDocument"),
        "{root}"
    );
    let book_rels = entry(&archive, "xl/_rels/workbook.xml.rels");
    assert!(!book_rels.contains("macrosheets/"), "{book_rels}");
    assert!(book_rels.contains("worksheets/sheet2.xml"), "{book_rels}");
    let orders_rels = entry(&archive, "xl/worksheets/_rels/sheet1.xml.rels");
    assert!(
        !orders_rels.contains("activeX") && !orders_rels.contains("oleObject1"),
        "{orders_rels}"
    );
    assert!(
        orders_rels.contains("Shop.xlsx") && orders_rels.contains("image1.png"),
        "{orders_rels}"
    );

    let book = entry(&archive, "xl/workbook.xml");
    assert!(!book.contains("Macro1"), "{book}");
    assert!(book.contains(r#"<sheet name="Summary""#), "{book}");
    assert!(
        book.contains(r#"firstSheet="1""#) && book.contains(r#"activeTab="1""#),
        "{book}"
    );
    assert!(
        book.contains(r#"<definedName name="total" localSheetId="1">"#),
        "{book}"
    );

    let orders = entry(&archive, "xl/worksheets/sheet1.xml");
    assert!(
        !orders.contains("<control") && !orders.contains(r#"r:id="rId4""#),
        "{orders}"
    );
    assert!(
        orders.contains(r#"<oleObject progId="Excel.Sheet.12""#),
        "{orders}"
    );
    assert!(orders.contains(r#"<drawing r:id="rId1"/>"#), "{orders}");
    assert_eq!(orders.matches("Shop item number").count(), 2, "{orders}");
}

/// A sheet part in UTF-16 or UTF-32, with a byte order mark or without one, and a part whose
/// declaration names another encoding each throw naming the part. A declaration of UTF-8 or
/// US-ASCII, and a UTF-8 byte order mark, read.
#[test]
fn a_part_in_utf16_or_another_declared_encoding_throws_naming_the_part() {
    let utf8 = sheet(r#"<row r="1"><c r="A1"><v>1</v></c></row>"#);
    let text = String::from_utf8(utf8.clone()).unwrap();
    let utf16 = |bom: &[u8], big: bool| {
        let mut out = bom.to_vec();
        for unit in text.encode_utf16() {
            out.extend(if big {
                unit.to_be_bytes()
            } else {
                unit.to_le_bytes()
            });
        }
        out
    };
    let mut utf32 = vec![0xFF, 0xFE, 0, 0];
    for c in text.chars() {
        utf32.extend(u32::from(c).to_le_bytes());
    }
    let declared = |encoding: &str| {
        text.replacen(
            r#"<?xml version="1.0"?>"#,
            &format!(r#"<?xml version="1.0" encoding="{encoding}" standalone="yes"?>"#),
            1,
        )
        .into_bytes()
    };
    let part = "`xl/worksheets/sheet1.xml`";
    for (what, data, says) in [
        ("UTF-16LE", utf16(&[0xFF, 0xFE], false), "is in UTF-16."),
        ("UTF-16BE", utf16(&[0xFE, 0xFF], true), "is in UTF-16."),
        ("UTF-16 without a mark", utf16(&[], false), "is in UTF-16 or UTF-32."),
        ("UTF-32LE", utf32, "is in UTF-32."),
        (
            "a declared Latin-1",
            declared("ISO-8859-1"),
            "declares the encoding `ISO-8859-1`.",
        ),
    ] {
        // A stored part shorter than the bytes the check judges ends in the same block it
        // starts in, and is judged there too.
        for deflate in [true, false] {
            let file = xlsx_as(&data, &shared(&[]), "", None, deflate);
            let failure = read(&file)
                .0
                .unwrap_or_else(|| panic!("{what} reads (deflated: {deflate})"));
            assert!(
                failure.contains(&format!("{part} {says} Only UTF-8 is supported.")),
                "{what} (deflated: {deflate}): {failure}"
            );
        }
    }
    let mut marked = vec![0xEF, 0xBB, 0xBF];
    marked.extend(&utf8);
    for (what, data) in [
        ("UTF-8", declared("UTF-8")),
        ("US-ASCII", declared("US-ASCII")),
        ("a UTF-8 mark", marked),
    ] {
        let file = xlsx_named(&data, &shared(&[]), "");
        assert_eq!(read(&file).0, None, "{what}");
    }
}
