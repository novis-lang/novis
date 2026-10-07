//! `rule:core-classes/image-format-roster` and `rule:core-classes/image-pixel-cap` inside the built-in
//! image component: `run` decodes each first-wave fixture under `extensions/image/fixtures/` in the
//! guest, JPEG XL decodes and does not encode, a header over the cap is refused before a buffer is
//! allocated, and bytes that are no image or carry a malformed IFD return `parse` rather than trap.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::call::{Error, Failure, Host, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Extension, Loader};

/// `future` polled on this thread until it finishes.
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

/// The image component, compiled on `host`'s engine.
fn component(host: &Host) -> Extension {
    Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load")
        .into_iter()
        .find(|builtin| builtin.manifest().class == "Novis\\Image\\Codec")
        .expect("the image component is built in")
        .extension()
        .expect("the image component compiles")
        .clone()
}

fn fixture(name: &str) -> Vec<u8> {
    let path = nvs_repo::path("extensions/image/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(key, value)| (Key::String(key.to_owned()), value))
            .collect(),
    )
}

/// An encoded source of `data`, with `maxPixels` set when `cap` is.
fn encoded(data: Vec<u8>, cap: Option<u64>) -> Value {
    let mut fields = vec![
        ("data", Value::Bytes(data)),
        ("autoOrient", Value::Bool(true)),
        ("toSrgb", Value::Bool(true)),
    ];
    if let Some(cap) = cap {
        fields.push(("maxPixels", Value::Uint(cap)));
    }
    shape(fields)
}

/// A plan of `steps` with no overlay, returning `output`.
fn plan(steps: Vec<Value>, output: &str) -> Value {
    shape(vec![
        (
            "steps",
            Value::Array(
                steps
                    .into_iter()
                    .enumerate()
                    .map(|(at, step)| (Key::Int(i64::try_from(at).unwrap()), step))
                    .collect(),
            ),
        ),
        ("overlays", Value::Array(Vec::new())),
        ("output", Value::Case(output.to_owned())),
    ])
}

fn run(
    request: &Request,
    extension: &Extension,
    source: Value,
    plan: Value,
) -> Result<Vec<u8>, Failure> {
    match block_on(request.call_values(extension, "run", vec![source, plan]))? {
        Some(Value::Bytes(bytes)) => Ok(bytes),
        other => panic!("`run` returned {other:?}"),
    }
}

/// The width and height in the 16-byte header `raw` and `size` begin with.
fn size_of(out: &[u8]) -> (u64, u64) {
    let word = |at: usize| u64::from_be_bytes(out[at..at + 8].try_into().unwrap());
    (word(0), word(8))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A PNG whose header declares `width` by `height` RGBA pixels, with a few bytes of pixel data
/// that decode to nothing.
fn declared_png(width: u32, height: u32) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &[0x78, 0x01, 0x00, 0x00]);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn expect_error(result: Result<Vec<u8>, Failure>) -> Error {
    match result {
        Err(Failure::Error(error)) => error,
        other => panic!("an `err` was expected, not {other:?}"),
    }
}

#[test]
fn every_first_wave_format_decodes_its_fixture() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    for name in [
        "gradient.jpg",
        "gradient.png",
        "gradient.webp",
        "gradient.gif",
        "gradient.avif",
        "gradient.jxl",
    ] {
        let out = run(
            &request,
            &extension,
            encoded(fixture(name), None),
            plan(Vec::new(), "Raw"),
        )
        .unwrap_or_else(|err| panic!("{name} does not decode: {err}"));
        assert_eq!(size_of(&out), (16, 12), "{name}");
        assert_eq!(out.len(), 16 + 16 * 12 * 4, "{name}");
        // The fixture runs from red at the top to blue at the bottom.
        let (top, bottom) = (&out[16..20], &out[out.len() - 4..]);
        assert!(
            top[0] > 200 && top[2] < 60,
            "{name}: the top row is {top:?}"
        );
        assert!(
            bottom[2] > 200 && bottom[0] < 60,
            "{name}: the bottom row is {bottom:?}"
        );
        assert_eq!(top[3], 255, "{name}");
    }
    block_on(request.end()).expect("the request ends");
}

#[test]
fn info_reads_the_header_and_never_a_pixel() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let meter = Arc::new(Meter::new(Duration::from_secs(30), None));
    let request = host.request(meter.clone());
    // The header declares 268 million pixels, a gigabyte of RGBA, and the pixel data is four bytes.
    let info = block_on(request.call_values(
        &extension,
        "info",
        vec![Value::Bytes(declared_png(16_384, 16_384))],
    ))
    .expect("`info` answers")
    .expect("`info` returns a value");
    let Value::Array(fields) = info else {
        panic!("a shape was expected")
    };
    let get = |name: &str| {
        fields
            .iter()
            .find(|(k, _)| *k == Key::String(name.to_owned()))
            .map(|(_, v)| v.clone())
    };
    assert_eq!(get("width"), Some(Value::Uint(16_384)));
    assert_eq!(get("height"), Some(Value::Uint(16_384)));
    assert!(
        meter.charged() < 64 << 20,
        "`info` charged {} bytes",
        meter.charged()
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn jpeg_xl_decodes_and_does_not_encode() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let raw = run(
        &request,
        &extension,
        encoded(fixture("gradient.jxl"), None),
        plan(Vec::new(), "Raw"),
    )
    .expect("a JPEG XL decodes");
    assert_eq!(size_of(&raw), (16, 12));
    let refused = expect_error(run(
        &request,
        &extension,
        encoded(fixture("gradient.jxl"), None),
        plan(Vec::new(), "Encoded"),
    ));
    assert_eq!(
        refused,
        Error::Invalid(
            "Jxl is a format the image component decodes and does not encode".to_owned()
        )
    );
    let to_jxl = shape(vec![(
        "format",
        shape(vec![("format", Value::Case("Jxl".to_owned()))]),
    )]);
    let refused = expect_error(run(
        &request,
        &extension,
        encoded(fixture("gradient.png"), None),
        plan(vec![to_jxl], "Encoded"),
    ));
    assert!(matches!(refused, Error::Invalid(_)), "{refused:?}");
    block_on(request.end()).expect("the request ends");
}

#[test]
fn an_image_over_max_pixels_is_refused_before_a_buffer_is_allocated() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let meter = Arc::new(Meter::new(Duration::from_secs(30), None));
    let request = host.request(meter.clone());
    let refused = expect_error(run(
        &request,
        &extension,
        encoded(declared_png(16_384, 16_384), None),
        plan(Vec::new(), "Raw"),
    ));
    assert_eq!(
        refused,
        Error::Runtime(
            "the image is 16384x16384, which is 268435456 pixels, over the cap of 25165824 pixels"
                .to_owned()
        )
    );
    assert!(
        meter.charged() < 64 << 20,
        "the refusal charged {} bytes",
        meter.charged()
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn a_malformed_ifd_is_a_thrown_parse_error_not_a_trap() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    // An EXIF block whose first IFD starts far past its end, written right after the JPEG's SOI.
    let mut exif = b"Exif\0\0MM\0\x2a".to_vec();
    exif.extend_from_slice(&0x7fff_0000u32.to_be_bytes());
    let mut jpeg = fixture("gradient.jpg");
    let mut segment = vec![0xff, 0xe1];
    segment.extend_from_slice(&u16::try_from(exif.len() + 2).unwrap().to_be_bytes());
    segment.extend_from_slice(&exif);
    jpeg.splice(2..2, segment);
    let refused = expect_error(run(
        &request,
        &extension,
        encoded(jpeg, None),
        plan(Vec::new(), "Raw"),
    ));
    assert!(
        matches!(&refused, Error::Parse(m) if m.starts_with("the EXIF block is malformed")),
        "{refused:?}"
    );
    // The instance was kept, and the next call in the same request decodes.
    run(
        &request,
        &extension,
        encoded(fixture("gradient.jpg"), None),
        plan(Vec::new(), "Size"),
    )
    .expect("the next call decodes");
    block_on(request.end()).expect("the request ends");
}

#[test]
fn bytes_that_are_not_an_image_throw_parse_error() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let refused = expect_error(run(
        &request,
        &extension,
        encoded(b"not an image at all".to_vec(), None),
        plan(Vec::new(), "Raw"),
    ));
    assert_eq!(
        refused,
        Error::Parse("the data is not an image in a format the image component reads".to_owned())
    );
    block_on(request.end()).expect("the request ends");
}
