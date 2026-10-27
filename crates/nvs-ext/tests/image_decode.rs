//! `rule:core-classes/image-format-roster` and `rule:core-classes/image-pixel-cap` inside the built-in
//! image component: `run` decodes each first-wave fixture under `extensions/image/fixtures/` in the
//! guest, JPEG XL decodes and does not encode, a header over the cap is refused before a buffer is
//! allocated, and bytes that are no image or carry a malformed IFD return `parse` rather than trap.
//! `info` returns the EXIF fields of each format that carries a block, and `null` for a file with
//! none.
//! An SVG draws with nothing outside it read, and a PDF page rasterises at its `dpi` under the cap
//! with `rule:core-classes/pdf-decode-refusals`' bomb corpus contained.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::call::{Error, Failure, Host, Limit, Meter, Request};
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
    run_args(request, extension, vec![source, plan])
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

/// The EXIF block of `exif.webp`, from its TIFF header on, as its `EXIF` chunk stores it.
fn exif_block() -> Vec<u8> {
    let webp = fixture("exif.webp");
    let mut at = 12;
    while at + 8 <= webp.len() {
        let size = u32::from_le_bytes(webp[at + 4..at + 8].try_into().unwrap()) as usize;
        if &webp[at..at + 4] == b"EXIF" {
            return webp[at + 8..at + 8 + size].to_vec();
        }
        at += 8 + size + size % 2;
    }
    panic!("`exif.webp` has no `EXIF` chunk")
}

/// `gradient.jpg` with `exif` written as an APP1 segment right after its SOI.
fn jpeg_with_exif(exif: &[u8]) -> Vec<u8> {
    let mut jpeg = fixture("gradient.jpg");
    let mut segment = vec![0xff, 0xe1];
    segment.extend_from_slice(&u16::try_from(exif.len() + 8).unwrap().to_be_bytes());
    segment.extend_from_slice(b"Exif\0\0");
    segment.extend_from_slice(exif);
    jpeg.splice(2..2, segment);
    jpeg
}

#[test]
fn info_returns_the_exif_fields_of_every_format_that_carries_them() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let exif_of = |data: Vec<u8>| {
        let info = block_on(request.call_values(&extension, "info", vec![Value::Bytes(data)]))
            .expect("`info` answers");
        let Value::Array(fields) = info.expect("`info` returns a value") else {
            panic!("a shape was expected")
        };
        fields
            .into_iter()
            .find(|(k, _)| *k == Key::String("exif".to_owned()))
            .map(|(_, v)| v)
            .expect("the shape has `exif`")
    };
    let tags = Value::Array(
        [
            ("Make", "Example"),
            ("Model", "Camera One"),
            ("Orientation", "row 0 at top and column 0 at left"),
            ("Software", "Novis fixtures"),
            ("DateTime", "2024:01:02 03:04:05"),
        ]
        .into_iter()
        .map(|(tag, value)| (Key::String(tag.to_owned()), Value::String(value.to_owned())))
        .collect(),
    );
    let block = exif_block();
    assert_eq!(exif_of(jpeg_with_exif(&block)), tags, "exif.jpg");
    for name in ["exif.png", "exif.webp", "exif.avif", "exif.jxl"] {
        assert_eq!(exif_of(fixture(name)), tags, "{name}");
    }
    assert_eq!(exif_of(fixture("gradient.png")), Value::Null);
    // The block's first IFD starts far past its end.
    let mut malformed = b"MM\0\x2a".to_vec();
    malformed.extend_from_slice(&0x7fff_0000u32.to_be_bytes());
    let refused = block_on(request.call_values(
        &extension,
        "info",
        vec![Value::Bytes(jpeg_with_exif(&malformed))],
    ));
    assert!(
        matches!(&refused, Err(Failure::Error(Error::Parse(m))) if m.starts_with("the EXIF block is malformed")),
        "{refused:?}"
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
        shape(vec![
            ("format", Value::Case("Jxl".to_owned())),
            ("quality", Value::Null),
            ("progressive", Value::Null),
            ("lossless", Value::Null),
            ("effort", Value::Null),
        ]),
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

/// `run`'s arguments as the host sends them under a request whose `[image] max_pixels` is
/// `in_force`.
fn capped(source: Value, plan: Value, in_force: u64) -> Vec<Value> {
    let mut args = vec![source, plan];
    nvs_ext::builtin::cap_pixels("Novis\\Image\\Codec", "run", &mut args, in_force);
    args
}

fn run_args(
    request: &Request,
    extension: &Extension,
    args: Vec<Value>,
) -> Result<Vec<u8>, Failure> {
    match block_on(request.call_values(extension, "run", args))? {
        Some(Value::Bytes(bytes)) => Ok(bytes),
        other => panic!("`run` returned {other:?}"),
    }
}

/// The `maxPixels` the encoded source in `args` crosses with.
fn cap_in(args: &[Value]) -> Option<Value> {
    let Value::Array(fields) = &args[0] else {
        panic!("a shape was expected")
    };
    fields
        .iter()
        .find(|(key, _)| *key == Key::String("maxPixels".to_owned()))
        .map(|(_, value)| value.clone())
}

#[test]
fn the_same_image_passes_when_the_cap_is_raised() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    // The fixture is 16 by 12, which is 192 pixels.
    let refused = expect_error(run_args(
        &request,
        &extension,
        capped(
            encoded(fixture("gradient.png"), None),
            plan(Vec::new(), "Size"),
            191,
        ),
    ));
    assert_eq!(
        refused,
        Error::Runtime(
            "the image is 16x12, which is 192 pixels, over the cap of 191 pixels".to_owned()
        )
    );
    let out = run_args(
        &request,
        &extension,
        capped(
            encoded(fixture("gradient.png"), None),
            plan(Vec::new(), "Size"),
            192,
        ),
    )
    .expect("the same bytes decode under a cap of 192");
    assert_eq!(size_of(&out), (16, 12));
    block_on(request.end()).expect("the request ends");
}

#[test]
fn a_call_may_lower_the_cap_and_never_raise_it() {
    let source = |cap| encoded(fixture("gradient.png"), cap);
    let size = || plan(Vec::new(), "Size");
    // A call that names no cap gets the one in force, and one that names a lower cap keeps it.
    assert_eq!(
        cap_in(&capped(source(None), size(), 1000)),
        Some(Value::Uint(1000))
    );
    assert_eq!(
        cap_in(&capped(source(Some(50)), size(), 1000)),
        Some(Value::Uint(50))
    );
    // A call that names a higher cap gets the one in force.
    assert_eq!(
        cap_in(&capped(source(Some(1 << 40)), size(), 1000)),
        Some(Value::Uint(1000))
    );
    // An overlay's source is held to the same cap.
    let overlay = shape(vec![
        ("source", source(Some(1 << 40))),
        ("steps", Value::Array(Vec::new())),
    ]);
    let mut args = capped(
        source(None),
        shape(vec![
            ("steps", Value::Array(Vec::new())),
            ("overlays", Value::Array(vec![(Key::Int(0), overlay)])),
            ("output", Value::Case("Size".to_owned())),
        ]),
        1000,
    );
    let Value::Array(plan_fields) = &mut args[1] else {
        panic!()
    };
    let Value::Array(overlays) = &plan_fields[1].1 else {
        panic!()
    };
    let Value::Array(overlay) = &overlays[0].1 else {
        panic!()
    };
    assert_eq!(cap_in(&[overlay[0].1.clone()]), Some(Value::Uint(1000)));
    // Another class's call is left alone.
    let mut other = vec![source(Some(1 << 40)), size()];
    nvs_ext::builtin::cap_pixels("Shop\\Codec", "run", &mut other, 1000);
    assert_eq!(cap_in(&other), Some(Value::Uint(1 << 40)));

    // In the guest: a call asking for more than the cap in force is refused at that cap.
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let refused = expect_error(run_args(
        &request,
        &extension,
        capped(source(Some(1 << 40)), size(), 100),
    ));
    assert_eq!(
        refused,
        Error::Runtime(
            "the image is 16x12, which is 192 pixels, over the cap of 100 pixels".to_owned()
        )
    );
    let refused = expect_error(run_args(
        &request,
        &extension,
        capped(source(Some(100)), size(), 1 << 30),
    ));
    assert!(
        matches!(&refused, Error::Runtime(m) if m.ends_with("over the cap of 100 pixels")),
        "{refused:?}"
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

#[test]
fn an_svg_decodes_and_an_external_href_throws() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let svg = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10">
  <rect width="10" height="10" fill="#ff0000"/>
  <rect x="10" width="10" height="10" fill="#0000ff"/>
</svg>"##;
    let out = run(
        &request,
        &extension,
        encoded(svg.as_bytes().to_vec(), None),
        plan(Vec::new(), "Raw"),
    )
    .expect("the SVG decodes");
    assert_eq!(size_of(&out), (20, 10));
    let pixel = |x: usize, y: usize| &out[16 + (y * 20 + x) * 4..16 + (y * 20 + x) * 4 + 4];
    assert_eq!(pixel(2, 5), &[255, 0, 0, 255]);
    assert_eq!(pixel(17, 5), &[0, 0, 255, 255]);
    let info = block_on(request.call_values(
        &extension,
        "info",
        vec![Value::Bytes(svg.as_bytes().to_vec())],
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
    assert_eq!(get("format"), Some(Value::Case("Svg".to_owned())));
    assert_eq!(get("width"), Some(Value::Uint(20)));
    // An SVG is one page.
    let second = shape(vec![
        ("data", Value::Bytes(svg.as_bytes().to_vec())),
        ("autoOrient", Value::Bool(true)),
        ("toSrgb", Value::Bool(true)),
        ("page", Value::Uint(2)),
    ]);
    assert_eq!(
        expect_error(run(&request, &extension, second, plan(Vec::new(), "Raw"))),
        Error::Runtime("the document has 1 page, so there is no page 2".to_owned())
    );
    // Neither a path nor an address is read. Each throws, naming the reference.
    for href in [
        "/etc/passwd",
        "file:///etc/passwd",
        "https://example.com/a.png",
        "logo.png",
    ] {
        let linked = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><image href="{href}" width="4" height="4"/></svg>"#
        );
        let refused = expect_error(run(
            &request,
            &extension,
            encoded(linked.into_bytes(), None),
            plan(Vec::new(), "Raw"),
        ));
        assert_eq!(
            refused,
            Error::Runtime(format!(
                "the SVG links to `{href}`. An SVG can only use images it contains as `data:` URIs"
            ))
        );
    }
    block_on(request.end()).expect("the request ends");
}

/// A PDF of `objects`, numbered from 1 with the catalog first, and a cross-reference table whose
/// offsets are the objects' own.
fn pdf_of(objects: &[String]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (at, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{body}\nendobj\n", at + 1).bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .bytes(),
    );
    out
}

fn stream(dict: &str, content: &str) -> String {
    format!(
        "<< {dict} /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    )
}

/// A document whose pages are `width` by `height` points, page `n` drawn by `contents[n - 1]`,
/// with `resources` on every page and `extra` objects numbered after the pages' own.
fn pages_pdf(
    width: u32,
    height: u32,
    contents: &[String],
    resources: &str,
    extra: &[String],
) -> Vec<u8> {
    let count = contents.len();
    let kids: Vec<String> = (0..count).map(|at| format!("{} 0 R", 3 + 2 * at)).collect();
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {count} >>",
            kids.join(" ")
        ),
    ];
    for (at, content) in contents.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << {resources} >> /Contents {} 0 R >>",
            4 + 2 * at
        ));
        objects.push(stream("", content));
    }
    objects.extend_from_slice(extra);
    pdf_of(&objects)
}

/// Three pages of 200 by 100 points, filled red, green and blue.
fn three_pages() -> Vec<u8> {
    let fill = |rgb: &str| format!("{rgb} rg 0 0 200 100 re f");
    pages_pdf(
        200,
        100,
        &[fill("1 0 0"), fill("0 1 0"), fill("0 0 1")],
        "",
        &[],
    )
}

/// An encoded source of `data` opening `page` at `dpi`, with `maxPixels` set when `cap` is.
fn page_of(data: Vec<u8>, page: u64, dpi: f64, cap: Option<u64>) -> Value {
    let mut fields = vec![
        ("data", Value::Bytes(data)),
        ("autoOrient", Value::Bool(true)),
        ("toSrgb", Value::Bool(true)),
        ("page", Value::Uint(page)),
        ("dpi", Value::Float(dpi)),
    ];
    if let Some(cap) = cap {
        fields.push(("maxPixels", Value::Uint(cap)));
    }
    shape(fields)
}

/// The RGBA of the pixel at `x`, `y` in a `raw` output `width` pixels wide.
fn pixel_at(out: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let at = 16 + (y * width + x) * 4;
    out[at..at + 4].try_into().unwrap()
}

#[test]
fn a_pdf_page_rasterises_at_the_requested_dpi() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    // 200 by 100 points at 144 dpi is 400 by 200 pixels, and page 2 is the green one.
    let out = run(
        &request,
        &extension,
        page_of(three_pages(), 2, 144.0, None),
        plan(Vec::new(), "Raw"),
    )
    .expect("page 2 rasterises");
    assert_eq!(size_of(&out), (400, 200));
    assert_eq!(pixel_at(&out, 400, 200, 100), [0, 255, 0, 255]);
    // With no `dpi`, a page is drawn at 150: each side is rounded up to a whole pixel.
    let out = run(
        &request,
        &extension,
        encoded(three_pages(), None),
        plan(Vec::new(), "Raw"),
    )
    .expect("page 1 rasterises");
    assert_eq!(size_of(&out), (417, 209));
    assert_eq!(pixel_at(&out, 417, 10, 10), [255, 0, 0, 255]);
    block_on(request.end()).expect("the request ends");
}

#[test]
fn info_counts_pdf_pages_and_allocates_no_pixel() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let meter = Arc::new(Meter::new(Duration::from_secs(30), None));
    let request = host.request(meter.clone());
    // A hundred pages of 14400 points square, the largest a page may be: one of them at 72 dpi
    // is more than 800 MiB of RGBA.
    let pages: Vec<String> = (0..100)
        .map(|_| "0 0 1 rg 0 0 10 10 re f".to_owned())
        .collect();
    let data = pages_pdf(14_400, 14_400, &pages, "", &[]);
    let info = block_on(request.call_values(&extension, "info", vec![Value::Bytes(data)]))
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
    assert_eq!(get("format"), Some(Value::Case("Pdf".to_owned())));
    assert_eq!(get("frames"), Some(Value::Uint(100)));
    assert_eq!(get("width"), Some(Value::Uint(14_400)));
    assert_eq!(get("height"), Some(Value::Uint(14_400)));
    assert!(
        meter.charged() < 64 << 20,
        "`info` charged {} bytes",
        meter.charged()
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn a_pdf_page_past_the_end_throws_naming_the_count() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    assert_eq!(
        expect_error(run(
            &request,
            &extension,
            page_of(three_pages(), 7, 72.0, None),
            plan(Vec::new(), "Raw"),
        )),
        Error::Runtime("the document has 3 pages, so there is no page 7".to_owned())
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn a_pdf_page_over_the_pixel_cap_is_refused_before_a_buffer() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let meter = Arc::new(Meter::new(Duration::from_secs(30), None));
    let request = host.request(meter.clone());
    // 14400 points square at 600 dpi is 120000 pixels a side: 14.4 billion pixels.
    let page = vec!["1 0 0 rg 0 0 14400 14400 re f".to_owned()];
    let data = pages_pdf(14_400, 14_400, &page, "", &[]);
    assert_eq!(
        expect_error(run(
            &request,
            &extension,
            page_of(data.clone(), 1, 600.0, None),
            plan(Vec::new(), "Raw"),
        )),
        Error::Runtime(
            "the image is 120000x120000, which is 14400000000 pixels, over the cap of 25165824 pixels"
                .to_owned()
        )
    );
    assert!(
        meter.charged() < 64 << 20,
        "the refusal charged {} bytes",
        meter.charged()
    );
    // The same page passes at a lower resolution: 14400 points at 7.2 dpi is 1440 pixels.
    let out = run(
        &request,
        &extension,
        page_of(data, 1, 7.2, None),
        plan(Vec::new(), "Size"),
    )
    .expect("the page passes at 7.2 dpi");
    assert_eq!(size_of(&out), (1440, 1440));
    block_on(request.end()).expect("the request ends");
}

#[test]
fn an_encrypted_pdf_throws_a_parse_error() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    // The fixture has no user password, so it would open with an empty one.
    let refused = expect_error(run(
        &request,
        &extension,
        encoded(fixture("encrypted.pdf"), None),
        plan(Vec::new(), "Raw"),
    ));
    let expected = Error::Parse(
        "the PDF is encrypted. Only a PDF without encryption can be opened as an image".to_owned(),
    );
    assert_eq!(refused, expected);
    match block_on(request.call_values(
        &extension,
        "info",
        vec![Value::Bytes(fixture("encrypted.pdf"))],
    )) {
        Err(Failure::Error(error)) => assert_eq!(error, expected),
        other => panic!("`info` should throw, not return {other:?}"),
    }
    block_on(request.end()).expect("the request ends");
}

#[test]
fn a_link_a_script_and_an_attachment_rasterise_inert() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    // The document runs a script on open, carries a file and links outside itself. The page is
    // drawn from its content alone, and the component has no import that could fetch or open.
    let data = pdf_of(&[
        "<< /Type /Catalog /Pages 2 0 R /OpenAction 5 0 R /Names << /EmbeddedFiles << /Names [(a.txt) 6 0 R] >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Contents 4 0 R /Annots [8 0 R] /AA << /O 9 0 R >> >>".to_owned(),
        stream("", "1 0 0 rg 0 0 200 100 re f"),
        "<< /S /JavaScript /JS (app.alert\\(1\\)) >>".to_owned(),
        "<< /Type /Filespec /F (a.txt) /EF << /F 7 0 R >> >>".to_owned(),
        stream("/Type /EmbeddedFile", "Shop"),
        "<< /Type /Annot /Subtype /Link /Rect [0 0 200 100] /A << /S /URI /URI (https://example.com/) >> >>".to_owned(),
        "<< /S /Launch /F (shop.exe) >>".to_owned(),
    ]);
    let out = run(
        &request,
        &extension,
        page_of(data, 1, 72.0, None),
        plan(Vec::new(), "Raw"),
    )
    .expect("the page rasterises");
    assert_eq!(size_of(&out), (200, 100));
    assert_eq!(pixel_at(&out, 200, 10, 10), [255, 0, 0, 255]);
    block_on(request.end()).expect("the request ends");
}

#[test]
fn an_image_that_does_not_decode_throws_naming_the_page() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let image = stream(
        "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode",
        "not a JPEG",
    );
    let data = pages_pdf(
        200,
        100,
        &["200 0 0 100 0 0 cm /Im Do".to_owned()],
        "/XObject << /Im 5 0 R >>",
        &[image],
    );
    let refused = expect_error(run(
        &request,
        &extension,
        page_of(data, 1, 72.0, None),
        plan(Vec::new(), "Raw"),
    ));
    assert_eq!(
        refused,
        Error::Runtime("page 1 contains an image that does not decode".to_owned())
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn an_unembedded_standard_font_renders_with_its_substitute() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let font = |name: &str| format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} >>");
    let text = |font: &str| vec![format!("BT /{font} 80 Tf 10 20 Td (Hi) Tj ET")];
    let extra = [font("Helvetica"), font("Courier"), font("Times-Roman")];
    let resources = "/Font << /Sans 5 0 R /Mono 6 0 R /Serif 7 0 R >>";
    let mut drawn = Vec::new();
    for name in ["Sans", "Mono", "Serif"] {
        let data = pages_pdf(200, 100, &text(name), resources, &extra);
        let out = run(
            &request,
            &extension,
            page_of(data, 1, 72.0, None),
            plan(Vec::new(), "Raw"),
        )
        .unwrap_or_else(|err| panic!("the {name} page rasterises: {err:?}"));
        let dark = out[16..]
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel[0] < 128)
            .count();
        assert!(dark > 500, "the {name} text drew {dark} dark pixels");
        drawn.push(out);
    }
    // Each standard font has its own substitute, so the three pages differ.
    assert_ne!(drawn[0], drawn[1]);
    assert_ne!(drawn[0], drawn[2]);
    assert_ne!(drawn[1], drawn[2]);
    block_on(request.end()).expect("the request ends");
}

#[test]
fn two_rasterisations_of_a_pdf_page_are_byte_equal() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let content = vec![
        "0.2 0.4 0.8 rg 10 10 120 60 re f 1 0 0 RG 4 w 20 90 m 180 15 l S BT /F 40 Tf 30 30 Td (Shop) Tj ET"
            .to_owned(),
    ];
    let extra = ["<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_owned()];
    let data = pages_pdf(200, 100, &content, "/Font << /F 5 0 R >>", &extra);
    let mut outs = Vec::new();
    for _ in 0..2 {
        let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
        outs.push(
            run(
                &request,
                &extension,
                page_of(data.clone(), 1, 150.0, None),
                plan(Vec::new(), "Raw"),
            )
            .expect("the page rasterises"),
        );
        block_on(request.end()).expect("the request ends");
    }
    assert_eq!(outs[0], outs[1]);
}

/// How a case of the bomb corpus ends: the pixel at 10, 10 of its drawn page, or the limit its
/// request reached.
type Ends = Result<[u8; 4], Limit>;

/// The outcome of rasterising `data` in a request of its own, with `seconds` of CPU and 256 MiB.
fn bomb(
    host: &Host,
    extension: &Extension,
    data: Vec<u8>,
    seconds: u64,
) -> Result<Vec<u8>, Failure> {
    let request = host.request(Arc::new(Meter::new(
        Duration::from_secs(seconds),
        Some(256 << 20),
    )));
    let out = run(
        &request,
        extension,
        page_of(data, 1, 72.0, None),
        plan(Vec::new(), "Raw"),
    );
    let _ = block_on(request.end());
    out
}

#[test]
fn the_pdf_bomb_corpus_ends_in_a_throw_or_a_contained_trap() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let form = |content: &str, resources: &str| {
        stream(
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 200 100] /Resources << {resources} >>"
            ),
            content,
        )
    };
    let draw = "/X Do ".repeat(16);
    // Each of ten forms draws the next one 16 times: 16 to the power 10 draws of the last.
    let mut nested: Vec<String> = (0..10)
        .map(|at| form(&draw, &format!("/XObject << /X {} 0 R >>", 6 + at)))
        .collect();
    nested.push(form("0 0 1 rg 0 0 1 1 re f", ""));
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    const RED: [u8; 4] = [255, 0, 0, 255];
    // Each case ends in a limit the request reached, or in its page drawn whole: `hayro` cuts a
    // form that draws itself and repairs a broken cross-reference table or stream length, and the
    // pixel named is the one a reader that is not fooled draws there.
    let mut corpus: Vec<(&str, Vec<u8>, Ends)> = vec![
        (
            "nested form XObjects",
            pages_pdf(
                200,
                100,
                &["/X Do".to_owned()],
                "/XObject << /X 5 0 R >>",
                &nested,
            ),
            Err(Limit::Cpu),
        ),
        (
            "a form XObject that draws itself",
            pages_pdf(
                200,
                100,
                &["/X Do".to_owned()],
                "/XObject << /X 5 0 R >>",
                &[form("/X Do /X Do", "/XObject << /X 5 0 R >>")],
            ),
            Ok(WHITE),
        ),
        (
            "a deflate bomb",
            fixture("deflate-bomb.pdf"),
            Err(Limit::Memory),
        ),
    ];
    // A cross-reference table whose `/Prev` is itself.
    let mut looped = three_pages();
    let start = String::from_utf8_lossy(&looped)
        .rsplit("startxref\n")
        .next()
        .and_then(|tail| tail.lines().next())
        .and_then(|offset| offset.parse::<usize>().ok())
        .expect("the document has a startxref");
    let text = String::from_utf8(looped.clone()).unwrap();
    looped = text
        .replace("/Root 1 0 R >>", &format!("/Root 1 0 R /Prev {start} >>"))
        .into_bytes();
    corpus.push((
        "a cross-reference table that is its own previous one",
        looped,
        Ok(RED),
    ));
    // A content stream whose `/Length` claims far more bytes than the file has.
    let lie = String::from_utf8(three_pages())
        .unwrap()
        .replacen("/Length 25 >>", "/Length 999999999 >>", 1)
        .into_bytes();
    corpus.push(("a stream length that is a lie", lie, Ok(RED)));
    for (name, data, ends) in corpus {
        // A bomb that ends at the memory cap gets a deadline that a loaded machine does not
        // reach first. Every other one ends inside two seconds.
        let seconds = if ends == Err(Limit::Memory) { 60 } else { 2 };
        match (bomb(&host, &extension, data, seconds), ends) {
            (Err(Failure::Limit(got)), Err(limit)) => assert_eq!(got, limit, "{name}"),
            (Ok(out), Ok(pixel)) => {
                assert_eq!(size_of(&out), (200, 100), "{name}");
                assert_eq!(pixel_at(&out, 200, 10, 10), pixel, "{name}");
            }
            (got, _) => panic!("{name}: {got:?}"),
        }
    }
    // The host is unharmed: the next request rasterises a page.
    let out = bomb(&host, &extension, three_pages(), 2).expect("a plain page after the corpus");
    assert_eq!(size_of(&out), (200, 100));
}
