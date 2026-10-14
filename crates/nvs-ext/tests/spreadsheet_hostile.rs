//! Hostile spreadsheets against the built-in spreadsheet component: each one ends in a throw or a
//! contained trap (`rule:core-classes/decompression-bound`,
//! `rule:core-classes/spreadsheet-reads-as-a-stream`).

use std::future::Future;
use std::io::Write as _;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::{Decompression, SPREADSHEET_CLASS, cap_decompression};
use nvs_ext::call::{Host, Meter};
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
