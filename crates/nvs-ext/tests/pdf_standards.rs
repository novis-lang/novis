//! The PDF component's standards as `nvs-ext` sees them (`rule:core-classes/pdf-one-engine`): every
//! file is tagged from its HTML, with headings, paragraphs, lists, tables with header cells,
//! figures with their `alt`, and the document's language.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::PDF_CLASS;
use nvs_ext::call::{Failure, Host, Meter};
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
                ("standard", Value::Null),
                ("password", Value::Null),
                ("invoiceXml", Value::Null),
                ("created", Value::Null),
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
