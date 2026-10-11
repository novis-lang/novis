//! The image component's `barcode` export and `qr`'s SVG output, called through the host as
//! `Novis\Image\Barcode` and `Novis\Image\QrCode` call them (`rule:core-classes/image-barcode-roster`).
//!
//! Every 1D symbology and DataMatrix is drawn as PNG, and read back to the data written by
//! `rxing`, a reader that is a dependency of this test only. A QR code and a Code 128 drawn as SVG
//! are SVG documents, and the component's own SVG decoding turns them into pixels that read back
//! too. A wrong check digit returns `invalid` naming the digit it should be, and a character a
//! symbology cannot carry returns `invalid` naming it. The GiroCode payload is Novis source, and
//! `crates/nvs-cli/tests/image_barcode.rs` runs it.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::call::{Error, Failure, Host, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Extension, Loader};
use rxing::{BarcodeFormat, DecodeHints};

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

fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(key, value)| (Key::String(key.to_owned()), value))
            .collect(),
    )
}

/// A host and the image component, compiled on its engine, and a request to call it in.
fn setup() -> (Host, Extension, Request) {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let extension = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load")
        .into_iter()
        .find(|builtin| builtin.manifest().class == "Novis\\Image\\Codec")
        .expect("the image component is built in")
        .extension()
        .expect("the image component compiles")
        .clone();
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    (host, extension, request)
}

/// `barcode` of `data` as `symbology`, in `format` or PNG.
fn barcode(
    request: &Request,
    extension: &Extension,
    data: &str,
    symbology: &str,
    format: Option<&str>,
) -> Result<Vec<u8>, Failure> {
    let options = shape(vec![
        ("height", Value::Null),
        ("module", Value::Null),
        ("margin", Value::Null),
        ("text", Value::Null),
        (
            "format",
            format.map_or(Value::Null, |format| Value::Case(format.to_owned())),
        ),
    ]);
    let args = vec![
        Value::String(data.to_owned()),
        Value::Case(symbology.to_owned()),
        options,
    ];
    match block_on(request.call_values(extension, "barcode", args))? {
        Some(Value::Bytes(bytes)) => Ok(bytes),
        other => panic!("`barcode` returned {other:?}"),
    }
}

/// The grey pixels of the encoded image `data`, decoded by the component: width, height and one
/// byte a pixel.
fn grey(request: &Request, extension: &Extension, data: &[u8]) -> (u32, u32, Vec<u8>) {
    let source = shape(vec![
        ("data", Value::Bytes(data.to_vec())),
        ("autoOrient", Value::Bool(true)),
        ("toSrgb", Value::Bool(true)),
    ]);
    let plan = shape(vec![
        ("steps", Value::Array(Vec::new())),
        ("overlays", Value::Array(Vec::new())),
        ("output", Value::Case("Raw".to_owned())),
    ]);
    let out = match block_on(request.call_values(extension, "run", vec![source, plan])) {
        Ok(Some(Value::Bytes(bytes))) => bytes,
        other => panic!("`run` returned {other:?}"),
    };
    let word = |at: usize| u64::from_be_bytes(out[at..at + 8].try_into().unwrap());
    let (width, height) = (word(0), word(8));
    let luma = out[16..].chunks_exact(4).map(|px| px[0]).collect();
    (width as u32, height as u32, luma)
}

/// The text `rxing` reads from the encoded image `data` as a `format` code. A GS1-128 code reads
/// as `]C1` and its element strings, an FNC1 between two of them read as the GS character.
fn read(request: &Request, extension: &Extension, data: &[u8], format: BarcodeFormat) -> String {
    let (width, height, luma) = grey(request, extension, data);
    let mut hints = DecodeHints {
        AssumeGs1: Some(true),
        ..DecodeHints::default()
    };
    rxing::helpers::detect_in_luma_with_hints(luma, width, height, Some(format), &mut hints)
        .unwrap_or_else(|err| panic!("no {format:?} code is read back: {err:?}"))
        .getText()
        .to_owned()
}

// covers: Novis\Image\Barcode::render
#[test]
fn every_1d_symbology_reads_back_as_the_data_written() {
    let (_host, extension, request) = setup();
    for (symbology, data, format, read_back) in [
        (
            "Code128",
            "Shop-42 order",
            BarcodeFormat::CODE_128,
            "Shop-42 order",
        ),
        (
            "Gs1128",
            "(01)09501101530003(10)ABC",
            BarcodeFormat::CODE_128,
            "]C10109501101530003\u{1d}10ABC",
        ),
        (
            "Ean13",
            "4006381333931",
            BarcodeFormat::EAN_13,
            "4006381333931",
        ),
        ("Ean8", "96385074", BarcodeFormat::EAN_8, "96385074"),
        ("UpcA", "036000291452", BarcodeFormat::UPC_A, "036000291452"),
        ("Code39", "SHOP-42", BarcodeFormat::CODE_39, "SHOP-42"),
        (
            "Itf14",
            "15400141288763",
            BarcodeFormat::ITF,
            "15400141288763",
        ),
    ] {
        let png = barcode(&request, &extension, data, symbology, None)
            .unwrap_or_else(|failure| panic!("{symbology} of {data:?}: {failure:?}"));
        assert!(
            png.starts_with(b"\x89PNG"),
            "{symbology}: the default format is PNG"
        );
        assert_eq!(
            read(&request, &extension, &png, format),
            read_back,
            "{symbology} reads back"
        );
    }
}

// covers: Novis\Image\Codec::barcode
#[test]
fn a_datamatrix_reads_back_as_the_data_written() {
    let (_host, extension, request) = setup();
    let data = "https://shop.example.com/items/1234";
    let png = barcode(&request, &extension, data, "DataMatrix", None).expect("a DataMatrix draws");
    assert_eq!(
        read(&request, &extension, &png, BarcodeFormat::DATA_MATRIX),
        data
    );
}

// covers: Novis\Image\Barcode::svg
// covers: Novis\Image\QrCode::svg
#[test]
fn a_qr_code_and_a_barcode_render_as_svg() {
    let (_host, extension, request) = setup();
    let options = shape(vec![
        ("size", Value::Null),
        ("margin", Value::Null),
        ("level", Value::Null),
        ("format", Value::Case("Svg".to_owned())),
    ]);
    let data = "https://blog.example.com/";
    let args = vec![Value::String(data.to_owned()), options];
    let qr = match block_on(request.call_values(&extension, "qr", args)) {
        Ok(Some(Value::Bytes(bytes))) => bytes,
        other => panic!("`qr` returned {other:?}"),
    };
    let text = std::str::from_utf8(&qr).expect("an SVG document is UTF-8");
    assert!(
        text.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""),
        "{text}"
    );
    assert!(text.contains(r#"width="256" height="256""#), "{text}");
    let (width, height, luma) = grey(&request, &extension, &qr);
    let mut prepared =
        rqrr::PreparedImage::prepare_from_greyscale(width as usize, height as usize, |x, y| {
            luma[y * width as usize + x]
        });
    let grids = prepared.detect_grids();
    assert_eq!(grids.len(), 1, "one QR code is found in the SVG");
    assert_eq!(grids[0].decode().expect("the QR code decodes").1, data);

    let svg =
        barcode(&request, &extension, "Shop-42", "Code128", Some("Svg")).expect("Code 128 draws");
    assert!(
        svg.starts_with(b"<svg "),
        "{}",
        String::from_utf8_lossy(&svg)
    );
    assert_eq!(
        read(&request, &extension, &svg, BarcodeFormat::CODE_128),
        "Shop-42"
    );
}

#[test]
fn a_wrong_check_digit_throws_naming_it() {
    let (_host, extension, request) = setup();
    for (symbology, data, expected) in [
        ("Ean13", "4006381333932", "is 1, and the data ends in 2"),
        ("Ean8", "96385075", "is 4, and the data ends in 5"),
        ("UpcA", "036000291453", "is 2, and the data ends in 3"),
        ("Itf14", "15400141288764", "is 3, and the data ends in 4"),
        (
            "Gs1128",
            "(01)09501101530004",
            "is 3, and the data ends in 4",
        ),
    ] {
        match barcode(&request, &extension, data, symbology, None) {
            Err(Failure::Error(Error::Invalid(message))) => {
                assert!(message.contains(expected), "{symbology}: {message}");
            }
            other => panic!("{symbology} of {data:?} returned {other:?}"),
        }
    }
    match barcode(&request, &extension, "Shop", "Code39", None) {
        Err(Failure::Error(Error::Invalid(message))) => {
            assert!(message.contains("'h' at position 2"), "{message}");
        }
        other => panic!("Code 39 of lower case returned {other:?}"),
    }
}
