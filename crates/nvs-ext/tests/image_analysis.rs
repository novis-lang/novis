//! The image component's analysis exports, called through the host as `Novis\Image` calls them
//! (`rule:core-classes/image-one-entry-point-per-job`): `compare` of an image with itself is
//! `identical` with an SSIM of exactly `1.0`, and of two images differing in one pixel reports
//! one differing pixel, in one host-to-guest call. A size mismatch returns `invalid` naming both
//! sizes, `tolerance` counts only the pixels whose delta is above it, and `render` returns a PNG
//! of the frames' size with the differing pixel in red. `hash` returns 8, 16 or 32 bytes for
//! `Perceptual`, `Difference` and `Average` whatever the image's size, and a copy at half the
//! size hashes within an eighth of the bits of its source. `hashDistance` is Novis source and
//! is pinned under `tests/conformance/novis/`. A `BlurHash` and a `ThumbHash` placeholder decode
//! to the image's average colour, in linear light and in sRGB. `palette` of two flat colours
//! returns those colours, most frequent first, and never more than `count`; a `count` above 256
//! returns `invalid`. A `qr` code reads back to its data through `rqrr` at every level, a higher
//! level needing a version at least as big, and is a `size`-pixel square with a light margin; a
//! `size` under the code's modules and data too long for its level return `invalid`. Text in
//! the font given as bytes draws dark pixels on a canvas in one crossing, the `text` source's
//! `size` output is exactly the box its `raw` output fills and a step's ink stays inside that box,
//! `maxWidth` puts each word of two on its own line and never breaks a word, and a font cut off
//! after its header returns `parse` from both the source and the step. The inputs are built here
//! or read from `extensions/image/fixtures/`.

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

fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(key, value)| (Key::String(key.to_owned()), value))
            .collect(),
    )
}

fn field(value: &Value, name: &str) -> Value {
    let Value::Array(fields) = value else {
        panic!("a shape was expected, not {value:?}")
    };
    fields
        .iter()
        .find(|(key, _)| *key == Key::String(name.to_owned()))
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| panic!("the shape has no `{name}`"))
}

/// A plan with no step and no overlay, returning `output`.
fn plan(output: &str) -> Value {
    shape(vec![
        ("steps", Value::Array(Vec::new())),
        ("overlays", Value::Array(Vec::new())),
        ("output", Value::Case(output.to_owned())),
    ])
}

fn run(request: &Request, extension: &Extension, source: Value, output: &str) -> Vec<u8> {
    match block_on(request.call_values(extension, "run", vec![source, plan(output)])) {
        Ok(Some(Value::Bytes(bytes))) => bytes,
        other => panic!("`run` returned {other:?}"),
    }
}

/// The width, the height and the RGBA8 rows of the encoded image `data`.
fn raw(request: &Request, extension: &Extension, data: &[u8]) -> (u64, u64, Vec<u8>) {
    let source = shape(vec![
        ("data", Value::Bytes(data.to_vec())),
        ("autoOrient", Value::Bool(true)),
        ("toSrgb", Value::Bool(true)),
    ]);
    let out = run(request, extension, source, "Raw");
    let word = |at: usize| u64::from_be_bytes(out[at..at + 8].try_into().unwrap());
    (word(0), word(8), out[16..].to_vec())
}

/// `pixels`, `width` by `height`, encoded as PNG by the component.
fn png(
    request: &Request,
    extension: &Extension,
    width: u64,
    height: u64,
    pixels: &[u8],
) -> Vec<u8> {
    let source = shape(vec![
        ("width", Value::Uint(width)),
        ("height", Value::Uint(height)),
        ("pixels", Value::Bytes(pixels.to_vec())),
    ]);
    run(request, extension, source, "Encoded")
}

fn compare(
    request: &Request,
    extension: &Extension,
    a: &[u8],
    b: &[u8],
    tolerance: Option<u64>,
    render: bool,
) -> Result<Value, Failure> {
    let options = shape(vec![
        ("tolerance", tolerance.map_or(Value::Null, Value::Uint)),
        ("render", Value::Bool(render)),
    ]);
    let args = vec![Value::Bytes(a.to_vec()), Value::Bytes(b.to_vec()), options];
    Ok(block_on(request.call_values(extension, "compare", args))?
        .expect("`compare` returns a value"))
}

fn setup() -> (Host, Extension) {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = component(&host);
    (host, extension)
}

fn request(host: &Host) -> Request {
    host.request(Arc::new(Meter::new(Duration::from_secs(60), None)))
}

fn gradient() -> Vec<u8> {
    let path = nvs_repo::path("extensions/image/fixtures/gradient.png");
    std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

// covers: Novis\Image\Image::compare
#[test]
fn an_image_compared_with_itself_is_identical_with_an_ssim_of_one_in_one_call() {
    let (host, extension) = setup();
    let request = request(&host);
    let data = gradient();
    let diff = compare(&request, &extension, &data, &data, None, false).expect("`compare` answers");
    assert_eq!(request.crossings(), 1);
    assert_eq!(field(&diff, "identical"), Value::Bool(true));
    assert_eq!(field(&diff, "differingPixels"), Value::Uint(0));
    assert_eq!(field(&diff, "maxDelta"), Value::Uint(0));
    assert_eq!(field(&diff, "ssim"), Value::Float(1.0));
    assert_eq!(field(&diff, "diff"), Value::Null);
}

// covers: Novis\Image\Codec::compare
#[test]
fn two_images_differing_in_one_pixel_report_one_differing_pixel() {
    let (host, extension) = setup();
    let request = request(&host);
    let (width, height, mut pixels) = raw(&request, &extension, &gradient());
    let a = png(&request, &extension, width, height, &pixels);
    pixels[0] ^= 0x80;
    let b = png(&request, &extension, width, height, &pixels);
    let diff = compare(&request, &extension, &a, &b, None, false).expect("`compare` answers");
    assert_eq!(field(&diff, "identical"), Value::Bool(false));
    assert_eq!(field(&diff, "differingPixels"), Value::Uint(1));
    assert_eq!(field(&diff, "maxDelta"), Value::Uint(128));
    let Value::Float(ssim) = field(&diff, "ssim") else {
        panic!("`ssim` is a float")
    };
    assert!(ssim < 1.0 && ssim > 0.9, "{ssim}");
}

#[test]
fn a_size_mismatch_returns_invalid_naming_both_sizes() {
    let (host, extension) = setup();
    let request = request(&host);
    let a = png(&request, &extension, 3, 2, &[7; 24]);
    let b = png(&request, &extension, 2, 3, &[7; 24]);
    match compare(&request, &extension, &a, &b, None, false) {
        Err(Failure::Error(Error::Invalid(message))) => {
            assert!(
                message.contains("3x2") && message.contains("2x3"),
                "{message}"
            );
        }
        other => panic!("a size mismatch returned {other:?}"),
    }
}

#[test]
fn a_delta_at_the_tolerance_is_not_counted_and_one_above_it_is() {
    let (host, extension) = setup();
    let request = request(&host);
    let a = png(
        &request,
        &extension,
        2,
        1,
        &[100, 100, 100, 255, 100, 100, 100, 255],
    );
    let b = png(
        &request,
        &extension,
        2,
        1,
        &[110, 100, 100, 255, 100, 111, 100, 255],
    );
    let counted = |tolerance| {
        let diff = compare(&request, &extension, &a, &b, Some(tolerance), false)
            .expect("`compare` answers");
        field(&diff, "differingPixels")
    };
    assert_eq!(counted(9), Value::Uint(2));
    assert_eq!(counted(10), Value::Uint(1));
    assert_eq!(counted(11), Value::Uint(0));
}

#[test]
fn render_returns_a_png_of_the_same_size_with_the_differing_pixel_in_red() {
    let (host, extension) = setup();
    let request = request(&host);
    let white = [255u8; 4 * 6];
    let mut marked = white;
    marked[4 * 4] = 0;
    let a = png(&request, &extension, 3, 2, &white);
    let b = png(&request, &extension, 3, 2, &marked);
    let diff = compare(&request, &extension, &a, &b, None, true).expect("`compare` answers");
    let Value::Bytes(rendered) = field(&diff, "diff") else {
        panic!("`render` returns the diff's bytes")
    };
    assert!(rendered.starts_with(b"\x89PNG"), "the diff is a PNG");
    let (width, height, pixels) = raw(&request, &extension, &rendered);
    assert_eq!((width, height), (3, 2));
    let red: Vec<usize> = pixels
        .chunks_exact(4)
        .enumerate()
        .filter(|(_, p)| *p == [255, 0, 0, 255])
        .map(|(at, _)| at)
        .collect();
    assert_eq!(red, [4]);
}

const HASH_KINDS: [(&str, usize); 3] = [("Perceptual", 8), ("Difference", 16), ("Average", 32)];

fn hash(request: &Request, extension: &Extension, data: &[u8], kind: &str) -> Vec<u8> {
    let args = vec![Value::Bytes(data.to_vec()), Value::Case(kind.to_owned())];
    match block_on(request.call_values(extension, "hash", args)) {
        Ok(Some(Value::Bytes(bytes))) => bytes,
        other => panic!("`hash` returned {other:?}"),
    }
}

/// How many bits differ between `a` and `b`, as `Image::hashDistance` counts them.
fn distance(a: &[u8], b: &[u8]) -> u32 {
    a.iter().zip(b).map(|(a, b)| (a ^ b).count_ones()).sum()
}

// covers: Novis\Image\Image::hash
#[test]
fn each_hash_kind_returns_a_hash_of_its_own_length() {
    let (host, extension) = setup();
    let request = request(&host);
    let tiny = png(&request, &extension, 3, 2, &[7; 24]);
    for data in [gradient(), tiny] {
        for (kind, bytes) in HASH_KINDS {
            assert_eq!(
                hash(&request, &extension, &data, kind).len(),
                bytes,
                "{kind}"
            );
        }
    }
}

// covers: Novis\Image\Codec::hash
#[test]
fn a_resized_copy_hashes_within_a_small_distance_of_its_source() {
    let (host, extension) = setup();
    let request = request(&host);
    // A picture big enough that half of it still fills every hash's grid: red rises to the
    // right, green rises downwards, and blue fills the top left and the bottom right.
    let (width, height) = (240u64, 180u64);
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let blue = if (x < width / 2) == (y < height * 3 / 10) {
                255
            } else {
                0
            };
            pixels.extend_from_slice(&[
                u8::try_from(x * 255 / width).unwrap(),
                u8::try_from(y * 255 / height).unwrap(),
                blue,
                255,
            ]);
        }
    }
    let (half_width, half_height) = (width / 2, height / 2);
    let mut half = Vec::new();
    for y in 0..half_height {
        for x in 0..half_width {
            for channel in 0..4 {
                let at = |dx: u64, dy: u64| {
                    let offset = ((y * 2 + dy) * width + x * 2 + dx) * 4 + channel;
                    u32::from(pixels[usize::try_from(offset).unwrap()])
                };
                let mean = (at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1)) / 4;
                half.push(u8::try_from(mean).unwrap());
            }
        }
    }
    let source = png(&request, &extension, width, height, &pixels);
    let copy = png(&request, &extension, half_width, half_height, &half);
    for (kind, bytes) in HASH_KINDS {
        let bits = u32::try_from(bytes * 8).unwrap();
        let near = distance(
            &hash(&request, &extension, &source, kind),
            &hash(&request, &extension, &copy, kind),
        );
        assert!(
            near <= bits / 8,
            "{kind}: the copy is {near} of {bits} bits away"
        );
    }
}

/// A `width` by `height` image whose left `left` columns are `a` and the rest `b`, as PNG.
fn two_colours(
    request: &Request,
    extension: &Extension,
    (width, height, left): (u64, u64, u64),
    a: [u8; 4],
    b: [u8; 4],
) -> Vec<u8> {
    let mut pixels = Vec::new();
    for _ in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(if x < left { &a } else { &b });
        }
    }
    png(request, extension, width, height, &pixels)
}

fn placeholder(request: &Request, extension: &Extension, data: &[u8], kind: &str) -> String {
    let args = vec![Value::Bytes(data.to_vec()), Value::Case(kind.to_owned())];
    match block_on(request.call_values(extension, "placeholder", args)) {
        Ok(Some(Value::String(text))) => text,
        other => panic!("`placeholder` returned {other:?}"),
    }
}

fn palette(
    request: &Request,
    extension: &Extension,
    data: &[u8],
    count: u64,
) -> Result<Vec<[f64; 4]>, Failure> {
    let args = vec![Value::Bytes(data.to_vec()), Value::Uint(count)];
    let Some(Value::Array(colours)) = block_on(request.call_values(extension, "palette", args))?
    else {
        panic!("`palette` returns an array")
    };
    Ok(colours
        .iter()
        .map(|(_, colour)| {
            ["r", "g", "b", "alpha"].map(|name| match field(colour, name) {
                Value::Uint(channel) => channel as f64,
                Value::Float(alpha) => alpha,
                other => panic!("`{name}` is {other:?}"),
            })
        })
        .collect())
}

/// The sRGB colour a BlurHash decodes to on average: its first component, four base-83 digits.
fn blurhash_average(hash: &str) -> [u64; 3] {
    const DIGITS: &str =
        "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz#$%*+,-.:;=?@[]^_{|}~";
    let n = hash[2..6]
        .chars()
        .fold(0u64, |n, c| n * 83 + DIGITS.find(c).unwrap() as u64);
    [n >> 16, (n >> 8) & 255, n & 255]
}

/// The sRGB colour a ThumbHash decodes to on average, from its header, each channel 0 to 255.
fn thumbhash_average(hash: &str) -> [f64; 3] {
    const DIGITS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let n = hash[..4].chars().fold(0u32, |n, c| {
        n << 6 | u32::try_from(DIGITS.find(c).unwrap()).unwrap()
    });
    let header = (n >> 16) | (n & 0xff00) | ((n & 0xff) << 16);
    let l = f64::from(header & 63) / 63.0;
    let p = f64::from((header >> 6) & 63) / 31.5 - 1.0;
    let q = f64::from((header >> 12) & 63) / 31.5 - 1.0;
    let b = l - 2.0 / 3.0 * p;
    let r = (3.0 * l - b + q) / 2.0;
    [r, r - q, b].map(|c| c.clamp(0.0, 1.0) * 255.0)
}

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

// covers: Novis\Image\Image::placeholder
#[test]
fn a_blurhash_placeholder_decodes_to_the_image_average_colour() {
    let (host, extension) = setup();
    let request = request(&host);
    let plain = two_colours(&request, &extension, (30, 20, 30), [40, 80, 120, 255], RED);
    let hash = placeholder(&request, &extension, &plain, "BlurHash");
    assert_eq!(hash.len(), 28, "{hash}");
    assert_eq!(blurhash_average(&hash), [40, 80, 120]);
    // BlurHash averages in linear light: half pure red and half pure blue is 188, not 128.
    let halves = two_colours(&request, &extension, (60, 40, 30), RED, BLUE);
    let [r, g, b] = blurhash_average(&placeholder(&request, &extension, &halves, "BlurHash"));
    assert!(
        r.abs_diff(188) <= 1 && g == 0 && b.abs_diff(188) <= 1,
        "{r}, {g}, {b}"
    );
}

// covers: Novis\Image\Codec::placeholder
#[test]
fn a_thumbhash_placeholder_decodes_to_the_image_average_colour() {
    let (host, extension) = setup();
    let request = request(&host);
    let colour = [40, 80, 120, 255];
    let plain = two_colours(&request, &extension, (30, 20, 30), colour, RED);
    let halves = two_colours(&request, &extension, (60, 40, 30), RED, BLUE);
    for (data, average) in [(plain, [40.0, 80.0, 120.0]), (halves, [127.5, 0.0, 127.5])] {
        let found = thumbhash_average(&placeholder(&request, &extension, &data, "ThumbHash"));
        assert!(
            found.iter().zip(average).all(|(f, a)| (f - a).abs() <= 3.0),
            "{found:?} is not {average:?}"
        );
    }
}

// covers: Novis\Image\Image::palette
#[test]
fn the_palette_of_a_two_colour_image_returns_those_two_colours() {
    let (host, extension) = setup();
    let request = request(&host);
    let data = two_colours(
        &request,
        &extension,
        (40, 30, 10),
        [220, 20, 60, 255],
        [30, 60, 200, 255],
    );
    let found = palette(&request, &extension, &data, 5).expect("`palette` succeeds");
    assert_eq!(
        found,
        [[30.0, 60.0, 200.0, 1.0], [220.0, 20.0, 60.0, 1.0]],
        "blue covers three quarters of the image, so it is first"
    );
}

// covers: Novis\Image\Codec::palette
#[test]
fn palette_returns_at_most_count_colours() {
    let (host, extension) = setup();
    let request = request(&host);
    let data = gradient();
    for count in [0, 1, 3, 5, 256] {
        let found = palette(&request, &extension, &data, count).expect("`palette` succeeds");
        assert!(found.len() as u64 <= count, "{count}: {}", found.len());
        assert_eq!(found.is_empty(), count == 0, "{count}");
    }
    match palette(&request, &extension, &data, 257) {
        Err(Failure::Error(Error::Invalid(message))) => {
            assert!(message.contains("257"), "{message}");
        }
        other => panic!("a count of 257 returned {other:?}"),
    }
}

/// `qr` of `data` with each option `None` left unset.
fn qr(
    request: &Request,
    extension: &Extension,
    data: &str,
    size: Option<u64>,
    margin: Option<u64>,
    level: Option<&str>,
) -> Result<Vec<u8>, Failure> {
    let options = shape(vec![
        ("size", size.map_or(Value::Null, Value::Uint)),
        ("margin", margin.map_or(Value::Null, Value::Uint)),
        (
            "level",
            level.map_or(Value::Null, |level| Value::Case(level.to_owned())),
        ),
        ("format", Value::Null),
    ]);
    let args = vec![Value::String(data.to_owned()), options];
    match block_on(request.call_values(extension, "qr", args))? {
        Some(Value::Bytes(bytes)) => Ok(bytes),
        other => panic!("`qr` returned {other:?}"),
    }
}

/// The data and the version of the one QR code in the encoded image `png`, read by `rqrr`.
fn read_qr(request: &Request, extension: &Extension, png: &[u8]) -> (String, usize) {
    let (width, height, rgba) = raw(request, extension, png);
    let width = usize::try_from(width).unwrap();
    let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(
        width,
        usize::try_from(height).unwrap(),
        |x, y| rgba[(y * width + x) * 4],
    );
    let grids = prepared.detect_grids();
    assert_eq!(grids.len(), 1, "one QR code is found");
    let (meta, content) = grids[0].decode().expect("the QR code decodes");
    (content, meta.version.0)
}

// covers: Novis\Image\QrCode::render
#[test]
fn a_qr_code_renders_and_decodes_back_to_its_data() {
    let (host, extension) = setup();
    let request = request(&host);
    let data = "https://shop.example.com/orders/1234?ref=Blog";
    let png = qr(&request, &extension, data, None, None, None).expect("`qr` succeeds");
    assert_eq!(request.crossings(), 1);
    assert!(png.starts_with(b"\x89PNG"), "the default format is PNG");
    let (width, height, _) = raw(&request, &extension, &png);
    assert_eq!((width, height), (256, 256), "the default size is 256");
    assert_eq!(read_qr(&request, &extension, &png).0, data);
}

// covers: Novis\Image\Codec::qr
#[test]
fn each_qr_level_renders_a_code_that_decodes() {
    let (host, extension) = setup();
    let request = request(&host);
    let data = "Shop order 1234, shipped to Blog Street 5";
    let mut versions = Vec::new();
    for level in ["Low", "Medium", "Quartile", "High"] {
        let png = qr(&request, &extension, data, Some(400), None, Some(level))
            .unwrap_or_else(|failure| panic!("{level}: {failure:?}"));
        let (content, version) = read_qr(&request, &extension, &png);
        assert_eq!(content, data, "{level}");
        versions.push(version);
    }
    assert!(
        versions.windows(2).all(|pair| pair[0] <= pair[1]) && versions[0] < versions[3],
        "a higher level needs a bigger code for the same data: {versions:?}"
    );
}

#[test]
fn qr_size_and_margin_set_the_image_size() {
    let (host, extension) = setup();
    for (size, margin) in [(25, 2), (29, 4), (100, 2), (333, 4), (512, 10)] {
        // A request of its own for each size, so the pixels read back stay under one request's copy limit.
        let request = self::request(&host);
        let png = qr(&request, &extension, "Shop", Some(size), Some(margin), None)
            .unwrap_or_else(|failure| panic!("{size}/{margin}: {failure:?}"));
        let (width, height, rgba) = raw(&request, &extension, &png);
        assert_eq!((width, height), (size, size), "{size}/{margin}");
        if margin > 0 {
            let side = usize::try_from(size).unwrap();
            let light = (0..side).all(|at| rgba[at * 4] == 255 && rgba[at * side * 4] == 255);
            assert!(light, "{size}/{margin}: the margin is light");
        }
        assert_eq!(
            read_qr(&request, &extension, &png).0,
            "Shop",
            "{size}/{margin}"
        );
    }
    // "Shop" is a version 1 code, 21 modules wide, so 4 modules of margin on each side make 29.
    let request = request(&host);
    match qr(&request, &extension, "Shop", Some(28), Some(4), None) {
        Err(Failure::Error(Error::Invalid(message))) => {
            assert!(
                message.contains("29") && message.contains("28"),
                "{message}"
            );
        }
        other => panic!("a size of 28 returned {other:?}"),
    }
}

#[test]
fn qr_data_too_long_for_its_level_is_a_logic_error() {
    let (host, extension) = setup();
    let request = request(&host);
    let data = "x".repeat(2_000);
    assert!(qr(&request, &extension, &data, None, None, Some("Low")).is_ok());
    match qr(&request, &extension, &data, None, None, Some("High")) {
        Err(Failure::Error(Error::Invalid(message))) => {
            assert!(message.contains("2000"), "{message}");
        }
        other => panic!("2000 bytes at `High` returned {other:?}"),
    }
}

fn font() -> Vec<u8> {
    let path = nvs_repo::path("extensions/image/fixtures/DancingScript-Regular.ttf");
    std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// `run` of `source` through `steps`, returning `output`.
fn run_steps(
    request: &Request,
    extension: &Extension,
    source: Value,
    steps: Vec<Value>,
    output: &str,
) -> Result<Vec<u8>, Failure> {
    let plan = shape(vec![
        (
            "steps",
            Value::Array(
                (0..)
                    .zip(steps)
                    .map(|(at, step)| (Key::Int(at), step))
                    .collect(),
            ),
        ),
        ("overlays", Value::Array(Vec::new())),
        ("output", Value::Case(output.to_owned())),
    ]);
    match block_on(request.call_values(extension, "run", vec![source, plan]))? {
        Some(Value::Bytes(bytes)) => Ok(bytes),
        other => panic!("`run` returned {other:?}"),
    }
}

/// The size header of a `size` or `raw` output, and the pixels behind it.
fn header(out: &[u8]) -> (u64, u64, &[u8]) {
    let word = |at: usize| u64::from_be_bytes(out[at..at + 8].try_into().unwrap());
    (word(0), word(8), &out[16..])
}

/// A `text` source of `text` at 32 pixels, wrapped at `max_width` when there is one.
fn text_source(text: &str, font: &[u8], max_width: Option<u64>) -> Value {
    shape(vec![
        ("text", Value::String(text.to_owned())),
        ("font", Value::Bytes(font.to_vec())),
        ("size", Value::Float(32.0)),
        ("maxWidth", max_width.map_or(Value::Null, Value::Uint)),
        ("align", Value::Null),
    ])
}

/// The width and height `measureText` reads: a `text` source run to its `size` output.
fn measure(
    request: &Request,
    extension: &Extension,
    text: &str,
    font: &[u8],
    max_width: Option<u64>,
) -> (u64, u64) {
    let out = run_steps(
        request,
        extension,
        text_source(text, font, max_width),
        Vec::new(),
        "Size",
    )
    .unwrap_or_else(|failure| panic!("measuring {text:?}: {failure:?}"));
    assert_eq!(out.len(), 16, "`size` is the header alone");
    let (width, height, _) = header(&out);
    (width, height)
}

/// A white canvas of `width` by `height` with `text` drawn on it in black at `x`, `y`, as raw pixels.
fn drawn(
    request: &Request,
    extension: &Extension,
    text: &str,
    font: &[u8],
    at: (i64, i64),
) -> Result<Vec<u8>, Failure> {
    let white = shape(vec![
        ("r", Value::Uint(255)),
        ("g", Value::Uint(255)),
        ("b", Value::Uint(255)),
        ("alpha", Value::Float(1.0)),
    ]);
    let canvas = shape(vec![
        ("width", Value::Uint(240)),
        ("height", Value::Uint(100)),
        ("fill", white),
    ]);
    let black = shape(vec![
        ("r", Value::Uint(0)),
        ("g", Value::Uint(0)),
        ("b", Value::Uint(0)),
        ("alpha", Value::Float(1.0)),
    ]);
    let step = shape(vec![(
        "text",
        shape(vec![
            ("text", Value::String(text.to_owned())),
            ("font", Value::Bytes(font.to_vec())),
            ("size", Value::Float(32.0)),
            ("color", black),
            ("gravity", Value::Null),
            ("x", Value::Int(at.0)),
            ("y", Value::Int(at.1)),
            ("maxWidth", Value::Null),
            ("align", Value::Null),
        ]),
    )]);
    run_steps(request, extension, canvas, vec![step], "Raw")
}

/// The smallest box holding every pixel of `rgba`, `width` wide, that `inked` picks.
fn ink_box(rgba: &[u8], width: u64, inked: impl Fn(&[u8]) -> bool) -> Option<(u64, u64, u64, u64)> {
    let mut found: Option<(u64, u64, u64, u64)> = None;
    for (at, px) in rgba.chunks_exact(4).enumerate() {
        if !inked(px) {
            continue;
        }
        let (x, y) = (at as u64 % width, at as u64 / width);
        found = Some(match found {
            None => (x, y, x, y),
            Some((left, top, right, bottom)) => {
                (left.min(x), top.min(y), right.max(x), bottom.max(y))
            }
        });
    }
    found
}

// covers: Novis\Image\Image::text
#[test]
fn text_renders_latin_with_the_font_given_as_bytes() {
    let (host, extension) = setup();
    let request = request(&host);
    let font = font();
    let out = drawn(&request, &extension, "Shop", &font, (10, 10)).expect("the `text` step runs");
    assert_eq!(
        request.crossings(),
        1,
        "the font and the text cross with the plan, in one call"
    );
    let (width, height, rgba) = header(&out);
    assert_eq!(
        (width, height),
        (240, 100),
        "the step keeps the frame's size"
    );
    let dark = rgba.chunks_exact(4).filter(|px| px[0] < 128).count();
    assert!(dark > 100, "the text is drawn in black: {dark} dark pixels");
    // The text source is the text alone, on a transparent canvas.
    let out = run_steps(
        &request,
        &extension,
        text_source("Blog", &font, None),
        Vec::new(),
        "Raw",
    )
    .expect("the `text` source runs");
    let (_, _, rgba) = header(&out);
    assert!(
        rgba.chunks_exact(4).any(|px| px[3] == 0),
        "the canvas is transparent"
    );
    assert!(
        rgba.chunks_exact(4)
            .any(|px| px[3] == 255 && px[..3] == [0, 0, 0]),
        "the text is opaque black"
    );
}

// covers: Novis\Image\Image::measureText
#[test]
fn measure_text_matches_the_box_text_draws() {
    let (host, extension) = setup();
    let request = request(&host);
    let font = font();
    let (width, height) = measure(&request, &extension, "Shop and Blog", &font, None);
    assert!(width > 0 && height >= 32, "{width}x{height}");
    let out = run_steps(
        &request,
        &extension,
        text_source("Shop and Blog", &font, None),
        Vec::new(),
        "Raw",
    )
    .unwrap();
    let (raw_w, raw_h, rgba) = header(&out);
    assert_eq!(
        (raw_w, raw_h),
        (width, height),
        "the source is exactly the measured box"
    );
    let (left, top, right, bottom) =
        ink_box(rgba, raw_w, |px| px[3] > 0).expect("the text has ink");
    assert!(
        left < width / 4 && right > width * 3 / 4,
        "the ink spans the box: {left}..{right} of {width}"
    );
    assert!(top < height && bottom < height);
    // Drawn as a step at 10, 10, every inked pixel lies inside the measured box placed there.
    let out = drawn(&request, &extension, "Shop and Blog", &font, (10, 10)).unwrap();
    let (canvas_w, _, rgba) = header(&out);
    let (left, top, right, bottom) =
        ink_box(rgba, canvas_w, |px| px[0] < 255).expect("the step draws");
    assert!(left >= 10 && top >= 10, "{left}, {top}");
    assert!(
        right < 10 + width && bottom < 10 + height,
        "{right}, {bottom} inside {width}x{height}"
    );
}

#[test]
fn max_width_wraps_at_a_word_boundary() {
    let (host, extension) = setup();
    let request = request(&host);
    let font = font();
    let (word, line) = measure(&request, &extension, "Shop", &font, None);
    let (both, _) = measure(&request, &extension, "Shop Shop", &font, None);
    assert!(
        both > word * 2,
        "two words on one line are wider than two words"
    );
    let (wrapped_w, wrapped_h) = measure(&request, &extension, "Shop Shop", &font, Some(word + 1));
    assert_eq!(wrapped_w, word, "each line is one word");
    assert!(
        wrapped_h > line * 3 / 2,
        "the text is two lines: {wrapped_h} against {line}"
    );
    // A word wider than `maxWidth` is a line of its own and is not broken.
    let (narrow_w, narrow_h) = measure(&request, &extension, "Shop", &font, Some(1));
    assert_eq!((narrow_w, narrow_h), (word, line));
}

#[test]
fn a_font_with_malformed_tables_throws_parse_error_at_the_terminal() {
    let (host, extension) = setup();
    let request = request(&host);
    // The header is a real TrueType header, and the tables it names are cut off.
    let truncated = font()[..200].to_vec();
    for (what, result) in [
        (
            "the source",
            run_steps(
                &request,
                &extension,
                text_source("Shop", &truncated, None),
                Vec::new(),
                "Size",
            ),
        ),
        (
            "the step",
            drawn(&request, &extension, "Shop", &truncated, (0, 0)),
        ),
    ] {
        match result {
            Err(Failure::Error(Error::Parse(message))) => {
                assert!(message.contains("200 bytes"), "{what}: {message}");
            }
            other => panic!("{what} with a truncated font returned {other:?}"),
        }
    }
}
