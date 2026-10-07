//! The image component's pipeline, called through the host as `Novis\Image` calls it
//! (`rule:core-classes/image-pipeline`, `rule:core-classes/image-correct-by-default`): a JPEG
//! with an EXIF orientation opens upright unless `autoOrient` is off, a re-encoded JPEG carries
//! no EXIF block unless the plan keeps it, and every format the roster encodes writes a file the
//! component reopens (`rule:core-classes/image-format-roster`). `variants` decodes once and costs
//! one host-to-guest call whatever its number of plans, as `encode` costs one.
//!
//! The tagged inputs are built here from `extensions/image/fixtures/gradient.jpg`, by inserting
//! an APP1 segment whose EXIF block this file writes, so no fixture carries real metadata.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::call::{Failure, Host, Meter, Request};
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

/// An encoded source of `data`, opened with `autoOrient` set to `auto_orient`.
fn encoded(data: Vec<u8>, auto_orient: bool) -> Value {
    opened(data, auto_orient, true)
}

/// An encoded source of `data`, opened with `autoOrient` and `toSrgb` as given.
fn opened(data: Vec<u8>, auto_orient: bool, to_srgb: bool) -> Value {
    shape(vec![
        ("data", Value::Bytes(data)),
        ("autoOrient", Value::Bool(auto_orient)),
        ("toSrgb", Value::Bool(to_srgb)),
    ])
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

/// The step `format(Format::<format>, {...})`, with `quality` and `lossless` when they are given.
fn format_step(format: &str, quality: Option<u64>, lossless: Option<bool>) -> Value {
    let mut options = vec![("format", Value::Case(format.to_owned()))];
    if let Some(quality) = quality {
        options.push(("quality", Value::Uint(quality)));
    }
    if let Some(lossless) = lossless {
        options.push(("lossless", Value::Bool(lossless)));
    }
    shape(vec![("format", shape(options))])
}

/// The `Format` case `file` is in, read from its signature.
fn format_of(file: &[u8]) -> &'static str {
    match file {
        [0xff, 0xd8, 0xff, ..] => "Jpeg",
        [0x89, b'P', b'N', b'G', ..] => "Png",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => "Webp",
        [b'G', b'I', b'F', b'8', ..] => "Gif",
        [
            _,
            _,
            _,
            _,
            b'f',
            b't',
            b'y',
            b'p',
            b'a',
            b'v',
            b'i',
            b'f',
            ..,
        ] => "Avif",
        _ => panic!("no known signature: {:?}", &file[..file.len().min(12)]),
    }
}

/// The chunks of the WebP `file`, each as its four-byte name and its payload.
fn webp_chunks(file: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    assert_eq!(&file[..4], b"RIFF", "the output is not a RIFF file");
    assert_eq!(&file[8..12], b"WEBP", "the output is not a WebP file");
    let riff = u32::from_le_bytes(file[4..8].try_into().unwrap());
    assert_eq!(usize::try_from(riff).unwrap() + 8, file.len());
    let mut chunks = Vec::new();
    let mut at = 12;
    while at + 8 <= file.len() {
        let name: [u8; 4] = file[at..at + 4].try_into().unwrap();
        let size =
            usize::try_from(u32::from_le_bytes(file[at + 4..at + 8].try_into().unwrap())).unwrap();
        chunks.push((name, file[at + 8..at + 8 + size].to_vec()));
        at += 8 + size + size % 2;
    }
    assert_eq!(at, file.len(), "a chunk runs past the end of the file");
    chunks
}

/// The step `metadata({keep: keep})`.
fn metadata(keep: bool) -> Value {
    shape(vec![("metadata", shape(vec![("keep", Value::Bool(keep))]))])
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

/// The field `name` of the header `info` reads from `data`.
fn info_field(request: &Request, extension: &Extension, data: Vec<u8>, name: &str) -> Value {
    let info = block_on(request.call_values(extension, "info", vec![Value::Bytes(data)]))
        .expect("`info` answers")
        .expect("`info` returns a value");
    let Value::Array(fields) = info else {
        panic!("a shape was expected")
    };
    fields
        .into_iter()
        .find(|(key, _)| *key == Key::String(name.to_owned()))
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("`info` has no `{name}`"))
}

/// The width and height in the 16-byte header `raw` and `size` begin with.
fn size_of(out: &[u8]) -> (u64, u64) {
    let word = |at: usize| u64::from_be_bytes(out[at..at + 8].try_into().unwrap());
    (word(0), word(8))
}

/// The RGBA8 pixel at `x`, `y` of a `raw` output.
fn pixel(raw: &[u8], x: u64, y: u64) -> [u8; 4] {
    let (width, _) = size_of(raw);
    let at = 16 + usize::try_from((y * width + x) * 4).unwrap();
    raw[at..at + 4].try_into().unwrap()
}

fn is_red(rgba: [u8; 4]) -> bool {
    rgba[0] > 200 && rgba[2] < 60
}

fn is_blue(rgba: [u8; 4]) -> bool {
    rgba[2] > 200 && rgba[0] < 60
}

/// Whether every colour channel of `rgba` is within 10 of `want`.
fn near(rgba: [u8; 4], want: [u8; 3]) -> bool {
    rgba.iter()
        .zip(want)
        .all(|(got, want)| got.abs_diff(want) <= 10)
}

/// The sRGB colour the test profile prints a corner of the CMYK cube as: the naive conversion,
/// where each ink takes away its complement and black takes away everything, squeezed into
/// 32..=223 so it can never be mistaken for the naive result itself.
fn press(c: u8, m: u8, y: u8, k: u8) -> [u8; 3] {
    let channel = |ink: u8| u8::try_from(32 + 191 * u32::from((1 - ink) * (1 - k))).unwrap();
    [channel(c), channel(m), channel(y)]
}

/// The CIELAB value of the sRGB colour `rgb` under D50, the ICC profile connection space.
fn lab(rgb: [u8; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(|value| {
        let value = f64::from(value) / 255.0;
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    let x = (0.436_074_7 * r + 0.385_064_9 * g + 0.143_080_4 * b) / 0.9642;
    let y = 0.222_504_5 * r + 0.716_878_6 * g + 0.060_616_9 * b;
    let z = (0.013_932_2 * r + 0.097_104_5 * g + 0.714_173_3 * b) / 0.8249;
    let f = |t: f64| {
        if t > (6.0f64 / 29.0).powi(3) {
            t.cbrt()
        } else {
            t / (3.0 * (6.0f64 / 29.0).powi(2)) + 4.0 / 29.0
        }
    };
    [
        116.0 * f(y) - 16.0,
        500.0 * (f(x) - f(y)),
        200.0 * (f(y) - f(z)),
    ]
}

/// A version 2 CMYK printer profile whose one tag, `A2B0`, is a lut16 table with two grid points
/// per ink: each corner of the CMYK cube is `press`'s colour, as CIELAB in the version 2
/// encoding, and the colours between are interpolated. The test writes it so that no fixture
/// carries a profile somebody else made.
fn cmyk_profile() -> Vec<u8> {
    let mut lut = b"mft2\0\0\0\0".to_vec();
    lut.extend_from_slice(&[4, 3, 2, 0]);
    for row in 0..3 {
        for column in 0..3 {
            let one: u32 = if row == column { 0x1_0000 } else { 0 };
            lut.extend_from_slice(&one.to_be_bytes());
        }
    }
    lut.extend_from_slice(&2u16.to_be_bytes());
    lut.extend_from_slice(&2u16.to_be_bytes());
    for _ in 0..4 {
        lut.extend_from_slice(&[0, 0, 0xff, 0xff]);
    }
    for corner in 0u8..16 {
        let bit = |at: u8| (corner >> at) & 1;
        let [l, a, b] = lab(press(bit(3), bit(2), bit(1), bit(0)));
        for value in [l * 652.8, (a + 128.0) * 256.0, (b + 128.0) * 256.0] {
            #[expect(
                clippy::cast_sign_loss,
                clippy::cast_possible_truncation,
                reason = "the value is rounded and clamped to the range of a `u16` first"
            )]
            let value = value.round().clamp(0.0, 65535.0) as u16;
            lut.extend_from_slice(&value.to_be_bytes());
        }
    }
    for _ in 0..3 {
        lut.extend_from_slice(&[0, 0, 0xff, 0xff]);
    }
    let lut_at = 128 + 4 + 12;
    let size = u32::try_from(lut_at + lut.len()).unwrap();
    let mut out = Vec::with_capacity(lut_at + lut.len());
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&0x0210_0000u32.to_be_bytes());
    out.extend_from_slice(b"prtrCMYKLab ");
    out.extend_from_slice(&[0; 12]);
    out.extend_from_slice(b"acsp");
    out.extend_from_slice(&[0; 28]);
    for illuminant in [0x0000_f6d6u32, 0x0001_0000, 0x0000_d32d] {
        out.extend_from_slice(&illuminant.to_be_bytes());
    }
    out.resize(128, 0);
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(b"A2B0");
    out.extend_from_slice(&u32::try_from(lut_at).unwrap().to_be_bytes());
    out.extend_from_slice(&u32::try_from(lut.len()).unwrap().to_be_bytes());
    out.extend_from_slice(&lut);
    out
}

/// `jpeg` with an APP2 segment carrying the ICC profile `profile` inserted after its start marker.
fn with_icc(jpeg: &[u8], profile: &[u8]) -> Vec<u8> {
    let mut payload = b"ICC_PROFILE\0\x01\x01".to_vec();
    payload.extend_from_slice(profile);
    let mut out = jpeg[..2].to_vec();
    out.extend_from_slice(&[0xff, 0xe2]);
    out.extend_from_slice(&u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
    out.extend_from_slice(&payload);
    out.extend_from_slice(&jpeg[2..]);
    out
}

/// One 12-byte IFD entry, big-endian, whose value or offset is `value`.
fn entry(out: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: [u8; 4]) {
    out.extend_from_slice(&tag.to_be_bytes());
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(&count.to_be_bytes());
    out.extend_from_slice(&value);
}

/// A big-endian EXIF block: IFD0 holds `orientation` when it is given, and with `gps` it points
/// at a GPS IFD holding a latitude of 48° 12' 30" N.
fn exif_block(orientation: Option<u16>, gps: bool) -> Vec<u8> {
    let count = usize::from(orientation.is_some()) + usize::from(gps);
    let gps_at = u32::try_from(8 + 2 + 12 * count + 4).unwrap();
    let rationals_at = gps_at + 2 + 12 * 2 + 4;
    let mut out = b"MM\0\x2a".to_vec();
    out.extend_from_slice(&8u32.to_be_bytes());
    out.extend_from_slice(&u16::try_from(count).unwrap().to_be_bytes());
    if let Some(orientation) = orientation {
        let [high, low] = orientation.to_be_bytes();
        entry(&mut out, 0x0112, 3, 1, [high, low, 0, 0]);
    }
    if gps {
        entry(&mut out, 0x8825, 4, 1, gps_at.to_be_bytes());
    }
    out.extend_from_slice(&0u32.to_be_bytes());
    if gps {
        out.extend_from_slice(&2u16.to_be_bytes());
        entry(&mut out, 0x0001, 2, 2, *b"N\0\0\0");
        entry(&mut out, 0x0002, 5, 3, rationals_at.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes());
        for (numerator, denominator) in [(48u32, 1u32), (12, 1), (30, 1)] {
            out.extend_from_slice(&numerator.to_be_bytes());
            out.extend_from_slice(&denominator.to_be_bytes());
        }
    }
    out
}

/// `jpeg` with an APP1 segment carrying `block` inserted after its start marker.
fn with_exif(jpeg: &[u8], block: &[u8]) -> Vec<u8> {
    let mut payload = b"Exif\0\0".to_vec();
    payload.extend_from_slice(block);
    let mut out = jpeg[..2].to_vec();
    out.extend_from_slice(&[0xff, 0xe1]);
    out.extend_from_slice(&u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
    out.extend_from_slice(&payload);
    out.extend_from_slice(&jpeg[2..]);
    out
}

/// The EXIF block in `jpeg`'s APP1 segment, found by walking the segments up to the scan.
fn exif_in(jpeg: &[u8]) -> Option<Vec<u8>> {
    assert_eq!(&jpeg[..2], &[0xff, 0xd8], "the output is not a JPEG");
    let mut at = 2;
    while at + 4 <= jpeg.len() && jpeg[at] == 0xff && jpeg[at + 1] != 0xda {
        let length = usize::from(u16::from_be_bytes([jpeg[at + 2], jpeg[at + 3]]));
        let body = &jpeg[at + 4..at + 2 + length];
        if jpeg[at + 1] == 0xe1 && body.starts_with(b"Exif\0\0") {
            return Some(body[6..].to_vec());
        }
        at += 2 + length;
    }
    None
}

#[test]
fn a_jpeg_with_orientation_6_opens_upright() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    // Orientation 6 says the stored frame turns 90 degrees clockwise to be upright, so the
    // stored 16 by 12 gradient, red at the top, opens 12 by 16 with red on the right.
    let tagged = with_exif(&fixture("gradient.jpg"), &exif_block(Some(6), false));
    let raw = run(
        &request,
        &extension,
        encoded(tagged, true),
        plan(Vec::new(), "Raw"),
    )
    .expect("the tagged JPEG decodes");
    assert_eq!(size_of(&raw), (12, 16));
    assert!(is_red(pixel(&raw, 11, 0)), "{:?}", pixel(&raw, 11, 0));
    assert!(is_blue(pixel(&raw, 0, 15)), "{:?}", pixel(&raw, 0, 15));
    block_on(request.end()).expect("the request ends");
}

#[test]
fn auto_orient_false_keeps_the_stored_orientation() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let tagged = with_exif(&fixture("gradient.jpg"), &exif_block(Some(6), false));
    let raw = run(
        &request,
        &extension,
        encoded(tagged, false),
        plan(Vec::new(), "Raw"),
    )
    .expect("the tagged JPEG decodes");
    assert_eq!(size_of(&raw), (16, 12));
    assert!(is_red(pixel(&raw, 0, 0)), "{:?}", pixel(&raw, 0, 0));
    assert!(is_blue(pixel(&raw, 15, 11)), "{:?}", pixel(&raw, 15, 11));
    block_on(request.end()).expect("the request ends");
}

/// `cmyk.jpg`, red ink over blue ink stored as Adobe stores CMYK, with `cmyk_profile` embedded.
fn cmyk_with_profile() -> Vec<u8> {
    with_icc(&fixture("cmyk.jpg"), &cmyk_profile())
}

#[test]
fn a_cmyk_jpeg_with_a_profile_resizes_to_the_reference_colours_not_inverted() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    // The top half is magenta and yellow ink, the bottom half cyan and magenta. Through the
    // profile they print as `press`'s red and blue. Read without Adobe's inversion, the top half
    // would be cyan and black ink and print near black.
    let raw = run(
        &request,
        &extension,
        opened(cmyk_with_profile(), true, true),
        plan(Vec::new(), "Raw"),
    )
    .expect("the CMYK JPEG decodes");
    assert_eq!(size_of(&raw), (16, 16));
    for (x, y) in [(0, 0), (15, 7)] {
        let got = pixel(&raw, x, y);
        assert!(near(got, press(0, 1, 1, 0)), "({x}, {y}) is {got:?}");
    }
    for (x, y) in [(0, 8), (15, 15)] {
        let got = pixel(&raw, x, y);
        assert!(near(got, press(1, 1, 0, 0)), "({x}, {y}) is {got:?}");
    }
    block_on(request.end()).expect("the request ends");
}

#[test]
fn to_srgb_false_keeps_the_raw_channels() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    // Without the conversion the profile is not read, and the ink amounts give pure red and
    // pure blue.
    let raw = run(
        &request,
        &extension,
        opened(cmyk_with_profile(), true, false),
        plan(Vec::new(), "Raw"),
    )
    .expect("the CMYK JPEG decodes");
    assert_eq!(size_of(&raw), (16, 16));
    assert!(
        near(pixel(&raw, 0, 0), [255, 0, 0]),
        "{:?}",
        pixel(&raw, 0, 0)
    );
    assert!(
        near(pixel(&raw, 15, 15), [0, 0, 255]),
        "{:?}",
        pixel(&raw, 15, 15)
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn a_jpeg_with_gps_tags_re_encodes_without_them() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let tagged = with_exif(&fixture("gradient.jpg"), &exif_block(None, true));
    assert!(exif_in(&tagged).is_some());
    let out = run(
        &request,
        &extension,
        encoded(tagged.clone(), true),
        plan(Vec::new(), "Encoded"),
    )
    .expect("the tagged JPEG re-encodes");
    assert_eq!(exif_in(&out), None);
    // `metadata({keep: false})` is the same as no `metadata` step.
    let out = run(
        &request,
        &extension,
        encoded(tagged, true),
        plan(vec![metadata(false)], "Encoded"),
    )
    .expect("the tagged JPEG re-encodes");
    assert_eq!(exif_in(&out), None);
    let size = run(
        &request,
        &extension,
        encoded(out, true),
        plan(Vec::new(), "Size"),
    )
    .expect("the output reopens");
    assert_eq!(size_of(&size), (16, 12));
    block_on(request.end()).expect("the request ends");
}

#[test]
fn metadata_keep_true_keeps_them() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let block = exif_block(None, true);
    let out = run(
        &request,
        &extension,
        encoded(with_exif(&fixture("gradient.jpg"), &block), true),
        plan(vec![metadata(true)], "Encoded"),
    )
    .expect("the tagged JPEG re-encodes");
    assert_eq!(exif_in(&out), Some(block));
    // A kept block says orientation 1 once `open` has turned the pixels upright, so a viewer
    // does not turn them a second time.
    let out = run(
        &request,
        &extension,
        encoded(
            with_exif(&fixture("gradient.jpg"), &exif_block(Some(6), true)),
            true,
        ),
        plan(vec![metadata(true)], "Encoded"),
    )
    .expect("the tagged JPEG re-encodes");
    assert_eq!(exif_in(&out), Some(exif_block(Some(1), true)));
    assert_eq!(
        info_field(&request, &extension, out.clone(), "width"),
        Value::Uint(12)
    );
    assert_eq!(
        info_field(&request, &extension, out, "orientation"),
        Value::Uint(1)
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn every_encoder_writes_its_format_and_the_result_reopens() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    // WebP appears twice: lossless through `image`, and lossy through libwebp.
    for (format, lossless) in [
        ("Jpeg", None),
        ("Png", None),
        ("Webp", Some(true)),
        ("Webp", None),
        ("Gif", None),
        ("Avif", None),
    ] {
        let out = run(
            &request,
            &extension,
            encoded(fixture("gradient.jpg"), true),
            plan(vec![format_step(format, None, lossless)], "Encoded"),
        )
        .unwrap_or_else(|err| panic!("{format} {lossless:?} does not encode: {err:?}"));
        assert_eq!(format_of(&out), format, "{lossless:?}");
        let raw = run(
            &request,
            &extension,
            encoded(out, true),
            plan(Vec::new(), "Raw"),
        )
        .unwrap_or_else(|err| panic!("{format} {lossless:?} does not reopen: {err:?}"));
        assert_eq!(size_of(&raw), (16, 12), "{format} {lossless:?}");
        assert!(
            is_red(pixel(&raw, 0, 0)),
            "{format} {:?}",
            pixel(&raw, 0, 0)
        );
        assert!(
            is_blue(pixel(&raw, 15, 11)),
            "{format} {:?}",
            pixel(&raw, 15, 11)
        );
    }
    block_on(request.end()).expect("the request ends");
}

#[test]
fn lossy_webp_is_encoded_by_libwebp() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let encode = |source: Vec<u8>, steps: Vec<Value>| {
        run(
            &request,
            &extension,
            encoded(source, true),
            plan(steps, "Encoded"),
        )
        .expect("the WebP encodes")
    };
    // A lossy WebP holds a `VP8 ` bitstream, which only libwebp writes here. A lossless one
    // holds `VP8L`.
    let lossy = encode(
        fixture("gradient.jpg"),
        vec![format_step("Webp", None, None)],
    );
    let names: Vec<[u8; 4]> = webp_chunks(&lossy)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, vec![*b"VP8 "]);
    let lossless = encode(
        fixture("gradient.jpg"),
        vec![format_step("Webp", None, Some(true))],
    );
    assert_eq!(webp_chunks(&lossless)[0].0, *b"VP8L");
    // `quality` reaches libwebp: a lower quality writes a smaller file.
    let low = encode(
        fixture("gradient.jpg"),
        vec![format_step("Webp", Some(5), None)],
    );
    let high = encode(
        fixture("gradient.jpg"),
        vec![format_step("Webp", Some(100), None)],
    );
    assert!(low.len() < high.len(), "{} >= {}", low.len(), high.len());
    // A kept EXIF block goes in an `EXIF` chunk after the bitstream, flagged in a `VP8X` header.
    let block = exif_block(None, true);
    let kept = encode(
        with_exif(&fixture("gradient.jpg"), &block),
        vec![format_step("Webp", None, None), metadata(true)],
    );
    let chunks = webp_chunks(&kept);
    let names: Vec<[u8; 4]> = chunks.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, vec![*b"VP8X", *b"VP8 ", *b"EXIF"]);
    assert_eq!(
        chunks[0].1[0] & 0x08,
        0x08,
        "the VP8X header has no EXIF flag"
    );
    assert_eq!(chunks[2].1, block);
    assert_eq!(
        info_field(&request, &extension, kept, "width"),
        Value::Uint(16)
    );
    block_on(request.end()).expect("the request ends");
}

#[test]
fn variants_with_three_entries_costs_one_crossing() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let plans = Value::Array(
        [
            plan(vec![format_step("Png", None, None)], "Encoded"),
            plan(vec![format_step("Webp", Some(70), None)], "Encoded"),
            plan(Vec::new(), "Raw"),
        ]
        .into_iter()
        .enumerate()
        .map(|(at, plan)| (Key::Int(i64::try_from(at).unwrap()), plan))
        .collect(),
    );
    let out = block_on(request.call_values(
        &extension,
        "variants",
        vec![encoded(fixture("gradient.jpg"), true), plans],
    ))
    .expect("`variants` answers");
    assert_eq!(request.crossings(), 1);
    let Some(Value::Array(outs)) = out else {
        panic!("`variants` returned {out:?}")
    };
    let outs: Vec<Vec<u8>> = outs
        .into_iter()
        .map(|(_, out)| match out {
            Value::Bytes(bytes) => bytes,
            other => panic!("a variant is {other:?}"),
        })
        .collect();
    assert_eq!(outs.len(), 3);
    assert_eq!(format_of(&outs[0]), "Png");
    assert_eq!(format_of(&outs[1]), "Webp");
    assert_eq!(size_of(&outs[2]), (16, 12));
    assert!(is_red(pixel(&outs[2], 0, 0)), "{:?}", pixel(&outs[2], 0, 0));
    block_on(request.end()).expect("the request ends");
}

#[test]
fn encode_costs_one_crossing() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let out = run(
        &request,
        &extension,
        encoded(fixture("gradient.jpg"), true),
        plan(
            vec![format_step("Png", None, None), metadata(false)],
            "Encoded",
        ),
    )
    .expect("the JPEG re-encodes");
    assert_eq!(format_of(&out), "Png");
    assert_eq!(request.crossings(), 1);
    block_on(request.end()).expect("the request ends");
}
