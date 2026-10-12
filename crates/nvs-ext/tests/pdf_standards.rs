//! The PDF component's standards as `nvs-ext` sees them (`rule:core-classes/pdf-one-engine`): every
//! file is tagged from its HTML, with headings, paragraphs, lists, tables with header cells,
//! figures with their `alt`, and the document's language. PDF/A-2b, PDF/A-3b and PDF/UA-1 are
//! `krilla`'s validators, and a file that breaks its standard throws naming what broke.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::PDF_CLASS;
use nvs_ext::call::{Error, Failure, Host, Meter};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::Loader;

/// A 1×1 PNG.
const PIXEL_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

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
            .map(|(name, value)| (Key::String(name.to_owned()), value))
            .collect(),
    )
}

/// The file `Engine::render` returns for `html` with no option set.
fn render(html: &str) -> Result<Vec<u8>, Failure> {
    render_under(html, None, None)
}

/// The file `Engine::render` returns for `html` under the `Standard` case `standard`, created at
/// `created` seconds since the epoch.
fn render_under(
    html: &str,
    standard: Option<&str>,
    created: Option<i64>,
) -> Result<Vec<u8>, Failure> {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let extension = builtins
        .iter()
        .find(|builtin| builtin.manifest().class == PDF_CLASS)
        .expect("the PDF component is built in")
        .extension()
        .expect("the PDF component compiles and checks");
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let pass = shape(vec![
        ("html", Value::String(html.to_owned())),
        ("assets", Value::Array(Vec::new())),
        (
            "options",
            shape(vec![
                (
                    "standard",
                    standard.map_or(Value::Null, |case| Value::Case(case.to_owned())),
                ),
                ("password", Value::Null),
                ("invoiceXml", Value::Null),
                (
                    "created",
                    created.map_or(Value::Null, |seconds| Value::Core {
                        class: "Core\\Time\\Instant".to_owned(),
                        fields: vec![
                            ("seconds".to_owned(), Value::Int(seconds)),
                            ("nanos".to_owned(), Value::Int(0)),
                        ],
                    }),
                ),
            ]),
        ),
    ]);
    let result = block_on(request.call_values(extension, "render", vec![pass]));
    block_on(request.end()).expect("the request ends");
    let Value::Array(fields) = result?.expect("`render` returns a value") else {
        panic!("the result is not a shape");
    };
    let data = fields
        .into_iter()
        .find(|(key, _)| matches!(key, Key::String(key) if key == "data"))
        .map(|(_, value)| value);
    let Some(Value::Bytes(data)) = data else {
        panic!("`data` is not bytes");
    };
    Ok(data)
}

/// `needle` occurs in `haystack`.
fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

#[test]
fn output_is_tagged_by_default() {
    let html = format!(
        "<html lang=\"de\"><body><h1>Shop</h1><p>Tea and books.</p>\
         <ul><li>Tea</li><li>Books</li></ul>\
         <table><thead><tr><th>Item</th><th>Price</th></tr></thead>\
         <tbody><tr><td>Tea</td><td>1.50</td></tr></tbody></table>\
         <img alt=\"The shop's logo\" src=\"data:image/png;base64,{PIXEL_PNG}\">\
         <p>Thank you.</p></body></html>"
    );
    let data = render(&html).expect("it renders");
    for needle in [
        "/StructTreeRoot",
        "/Marked true",
        "/H1",
        "/P",
        "/L",
        "/LI",
        "/Table",
        "/TR",
        "/TH",
        "/TD",
        "/Figure",
        "(The shop's logo)",
        "/Lang",
    ] {
        assert!(contains(&data, needle), "the file has no `{needle}`");
    }
    assert_eq!(
        render(&html).expect("it renders again"),
        data,
        "two tagged renders differ"
    );
}

/// The message of the `Invalid` error `failure` is.
fn invalid(failure: Result<Vec<u8>, Failure>) -> String {
    match failure {
        Err(Failure::Error(Error::Invalid(message))) => message,
        Err(other) => panic!("not an invalid error: {other:?}"),
        Ok(_) => panic!("it rendered"),
    }
}

/// A shop's page with a title, a language, a heading, a table and an image with its `alt`.
fn shop_page() -> String {
    format!(
        "<html lang=\"en\"><head><title>Shop</title></head><body><h1>Shop</h1>\
         <p>Tea and books.</p><table><tr><th>Item</th></tr><tr><td>Tea</td></tr></table>\
         <img alt=\"The shop's logo\" src=\"data:image/png;base64,{PIXEL_PNG}\">\
         <p><a href=\"https://example.com/\">example.com</a></p></body></html>"
    )
}

/// 2026-01-01 at midnight, UTC.
const NEW_YEAR: i64 = 1_767_225_600;

#[test]
fn pdf_a_2b_and_3b_validate() {
    let html = shop_page();
    for (case, part) in [("PdfA2b", "2"), ("PdfA3b", "3")] {
        let data = render_under(&html, Some(case), Some(NEW_YEAR)).expect("it renders");
        for needle in [
            "/GTS_PDFA1".to_owned(),
            format!("<pdfaid:part>{part}</pdfaid:part>"),
            "<pdfaid:conformance>B</pdfaid:conformance>".to_owned(),
            "/StructTreeRoot".to_owned(),
        ] {
            assert!(contains(&data, &needle), "{case} has no `{needle}`");
        }
        assert_eq!(
            render_under(&html, Some(case), Some(NEW_YEAR)).expect("it renders again"),
            data,
            "two {case} renders differ"
        );
        // PDF/A needs the creation time, and nothing in the component reads a clock.
        let message = invalid(render_under(&html, Some(case), None));
        assert!(message.contains("creation time"), "{message}");
    }
}

#[test]
fn pdf_ua_1_throws_for_an_image_without_alt() {
    let data = render_under(&shop_page(), Some("PdfUa1"), None).expect("it renders");
    assert!(contains(&data, "pdfuaid"), "the file does not claim PDF/UA");
    let html = format!(
        "<html lang=\"en\"><head><title>Shop</title></head><body><h1>Shop</h1>\
         <img src=\"data:image/png;base64,{PIXEL_PNG}\"></body></html>"
    );
    let message = invalid(render_under(&html, Some("PdfUa1"), None));
    assert!(
        message.contains("PDF/UA-1") && message.contains("an image has no `alt` text"),
        "{message}"
    );
    // Without a standard, the same document renders.
    render(&html).expect("it renders with no standard");
}
