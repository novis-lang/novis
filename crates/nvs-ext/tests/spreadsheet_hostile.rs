//! Hostile spreadsheets against the built-in spreadsheet component: each one ends in a throw or a
//! contained trap (`rule:core-classes/decompression-bound`,
//! `rule:core-classes/spreadsheet-reads-as-a-stream`).

use std::future::Future;
use std::io::Write as _;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::{Decompression, SPREADSHEET_CLASS, cap_decompression};
use nvs_ext::call::{Budget, Host, Lending, Level, Meter, Outbound, Outgoing};
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
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in parts {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(data).unwrap();
        let packed = encoder.finish().unwrap();
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let offset = u32::try_from(out.len()).unwrap();
        let sizes = |out: &mut Vec<u8>| {
            out.extend_from_slice(&[0, 0, 8, 0, 0, 0, 0, 0]);
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
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let package = "http://schemas.openxmlformats.org/package/2006/relationships";
    let root = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
    );
    let book = format!(
        r#"<workbook xmlns="{main}" xmlns:r="{rel}"><sheets><sheet name="Orders" sheetId="1" r:id="rId1"/></sheets></workbook>"#
    );
    let rels = format!(
        r#"<Relationships xmlns="{package}"><Relationship Id="rId1" Type="{rel}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{rel}/sharedStrings" Target="sharedStrings.xml"/></Relationships>"#
    );
    zip(&[
        ("[Content_Types].xml", b"<Types/>"),
        ("_rels/.rels", root.as_bytes()),
        ("xl/workbook.xml", book.as_bytes()),
        ("xl/_rels/workbook.xml.rels", rels.as_bytes()),
        ("xl/sharedStrings.xml", shared),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
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
    let host = Host::new(2, |_| Ok(())).expect("the host starts");
    let extension = spreadsheet(&host);
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
            let options = Value::Array(
                [
                    ("sheet", Value::String("Orders".to_owned())),
                    ("headerRow", Value::Null),
                    ("from", from.clone()),
                    ("limit", Value::Null),
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
            );
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
    drop(block_on(request.end()));
    (failure, budget.sent.load(Ordering::Acquire))
}

/// A failure that is the reader's throw or the request's memory limit, never a crash.
fn contained(failure: Option<String>, what: &str) -> String {
    let failure = failure.unwrap_or_else(|| panic!("{what} reads as if it were a workbook"));
    assert!(
        failure.contains("rule:core-classes/decompression-bound") || failure.contains("memory"),
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
