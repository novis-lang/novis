//! The image component's analysis exports, called through the host as `Novis\Image` calls them
//! (`rule:core-classes/image-one-entry-point-per-job`): `compare` of an image with itself is
//! `identical` with an SSIM of exactly `1.0`, and of two images differing in one pixel reports
//! one differing pixel, in one host-to-guest call. A size mismatch returns `invalid` naming both
//! sizes, `tolerance` counts only the pixels whose delta is above it, and `render` returns a PNG
//! of the frames' size with the differing pixel in red. The inputs are built here from
//! `extensions/image/fixtures/gradient.png`.

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
