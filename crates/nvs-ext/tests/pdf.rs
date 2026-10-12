//! The PDF component as `nvs-ext` sees it (`rule:packaging/the-first-party-components-are-built-in`,
//! `rule:core-classes/pdf-render-has-no-io`, `rule:core-classes/pdf-output-is-inert`):
//! `Novis\Pdf\Engine` is in the binary and renders with no `[[extension]]` entry and no grant. A
//! paragraph renders with the default fonts, a reference resolves against the asset map and `data:`
//! alone, and a `file:`, `http:` or unknown name throws naming it. Two renders of one document are
//! byte-equal, the output carries no action a reader runs, and HTML nested past the parser's
//! element cap flattens as `Core\Html`'s parse does.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::{PDF, PDF_CLASS, PDF_SHA256};
use nvs_ext::call::{Error, Failure, Host, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Builtin, Extension, Loader};

/// A 1×1 PNG.
const PIXEL_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

/// The PDF component among `builtins`.
fn pdf(builtins: &[Builtin]) -> &Builtin {
    builtins
        .iter()
        .find(|builtin| builtin.manifest().class == PDF_CLASS)
        .expect("the PDF component is built in")
}

/// `future` polled on this thread until it finishes. A guest call yields at every epoch tick, and
/// polling again resumes it.
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

/// The `whole` pass of `html` with `assets`, as `Novis\Pdf\Document::render` builds it.
fn whole(html: &str, assets: &[(&str, &[u8])]) -> Value {
    let assets = assets
        .iter()
        .enumerate()
        .map(|(at, (name, data))| {
            (
                Key::Int(i64::try_from(at).unwrap()),
                shape(vec![
                    ("name", Value::String((*name).to_owned())),
                    ("data", Value::Bytes(data.to_vec())),
                ]),
            )
        })
        .collect();
    shape(vec![
        ("html", Value::String(html.to_owned())),
        ("assets", Value::Array(assets)),
        (
            "options",
            shape(vec![
                ("standard", Value::Null),
                ("password", Value::Null),
                ("invoiceXml", Value::Null),
                ("created", Value::Null),
            ]),
        ),
    ])
}

fn field(value: &Value, name: &str) -> Value {
    let Value::Array(fields) = value else {
        panic!("a shape was expected, not {value:?}");
    };
    fields
        .iter()
        .find(|(key, _)| matches!(key, Key::String(key) if key == name))
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| panic!("the result has no `{name}`"))
}

/// The file and the page count `Engine::render` returns for `html` and `assets`.
fn render(
    request: &Request,
    extension: &Extension,
    html: &str,
    assets: &[(&str, &[u8])],
) -> Result<(Vec<u8>, u64), Failure> {
    let result = block_on(request.call_values(extension, "render", vec![whole(html, assets)]))?
        .expect("`render` returns a value");
    let Value::Bytes(data) = field(&result, "data") else {
        panic!("`data` is not bytes");
    };
    let Value::Uint(pages) = field(&result, "pageCount") else {
        panic!("`pageCount` is not a uint");
    };
    Ok((data, pages))
}

fn run<T>(test: impl FnOnce(&Request, &Extension) -> T) -> T {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let extension = pdf(&builtins)
        .extension()
        .expect("the PDF component compiles and checks");
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let out = test(&request, extension);
    block_on(request.end()).expect("the request ends");
    out
}

/// `needle` occurs in `haystack`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The message of the `invalid` error `failure` is, or a panic naming what it is instead.
fn invalid(failure: Result<(Vec<u8>, u64), Failure>) -> String {
    match failure {
        Err(Failure::Error(Error::Invalid(message))) => message,
        other => panic!("the render was not `invalid`: {other:?}"),
    }
}

#[test]
fn the_pdf_component_is_built_in_and_answers_with_no_configuration() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let builtin = pdf(&builtins);
    let digest: String = PDF_SHA256.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(builtin.sha256(), digest);
    assert_eq!(builtin.sha256(), nvs_ext::load::pin(PDF));
    assert!(!builtin.compiled(), "loading compiled the component");
    let extension = builtin
        .extension()
        .expect("the PDF component compiles and checks");
    assert_eq!(extension.memory, None);
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let (data, pages) = render(&request, extension, "<p>Shop</p>", &[]).expect("it renders");
    assert!(data.starts_with(b"%PDF-"), "the output is not a PDF file");
    assert_eq!(pages, 1);
    block_on(request.end()).expect("the request ends");
}

#[test]
fn a_render_needs_no_capability_grant() {
    run(|request, extension| {
        assert_eq!(
            extension.grants,
            nvs_config::extension::Granted::default(),
            "the PDF component is granted nothing"
        );
        render(request, extension, "<p>Blog</p>", &[]).expect("a render with no grant succeeds");
    });
}

#[test]
fn a_paragraph_renders_with_the_default_fonts() {
    run(|request, extension| {
        let html = "<h1>Shop</h1><p>Thank you for your order. It ships tomorrow.</p>";
        let (data, pages) = render(request, extension, html, &[]).expect("it renders");
        assert_eq!(pages, 1);
        assert!(
            contains(&data, b"LiberationSans"),
            "the paragraph is not set in Liberation Sans"
        );
        assert!(
            contains(&data, b"/FontFile2"),
            "the font is not embedded in the file"
        );
    });
}

#[test]
fn a_file_or_http_reference_throws_naming_it() {
    run(|request, extension| {
        for html in [
            "<img src=\"file:///etc/passwd\">",
            "<p style=\"background: url(file:///etc/passwd)\">x</p>",
            "<svg><image href=\"file:///etc/passwd\"/></svg>",
        ] {
            let message = invalid(render(request, extension, html, &[]));
            assert!(
                message.contains("`file:///etc/passwd`"),
                "{html}: {message}"
            );
        }
        for html in [
            "<img src=\"http://example.com/a.png\">",
            "<style>p { background: url('http://example.com/a.png') }</style><p>x</p>",
            "<svg><image href=\"http://example.com/a.png\"/></svg>",
        ] {
            let message = invalid(render(request, extension, html, &[]));
            assert!(
                message.contains("`http://example.com/a.png`"),
                "{html}: {message}"
            );
        }
    });
}

#[test]
fn a_name_missing_from_the_asset_map_throws() {
    run(|request, extension| {
        let png = base64(PIXEL_PNG);
        let message = invalid(render(
            request,
            extension,
            "<img src=\"logo.png\">",
            &[("other.png", &png)],
        ));
        assert!(message.contains("`logo.png`"), "{message}");
        render(
            request,
            extension,
            "<img src=\"logo.png\">",
            &[("logo.png", &png)],
        )
        .expect("the same name in the map renders");
    });
}

#[test]
fn a_data_uri_image_renders() {
    run(|request, extension| {
        let html = format!("<img src=\"data:image/png;base64,{PIXEL_PNG}\">");
        let (data, pages) = render(request, extension, &html, &[]).expect("it renders");
        assert_eq!(pages, 1);
        assert!(
            contains(&data, b"/Image"),
            "the `data:` image is not in the file"
        );
    });
}

#[test]
fn two_renders_of_equal_input_are_byte_equal() {
    run(|request, extension| {
        let html = format!(
            "<h1>Blog</h1><p>{}</p><img src=\"data:image/png;base64,{PIXEL_PNG}\">",
            "A paragraph of text that wraps. ".repeat(200)
        );
        let (first, pages) = render(request, extension, &html, &[]).expect("it renders");
        let (second, _) = render(request, extension, &html, &[]).expect("it renders");
        assert!(pages > 1, "the text did not reach a second page");
        assert!(first == second, "two renders of one document differ");
        assert!(
            !contains(&first, b"/CreationDate"),
            "a render with no `created` carries a creation time"
        );
    });
}

#[test]
fn the_output_has_no_javascript_launch_or_open_action() {
    run(|request, extension| {
        let html = "<script>app.alert(1)</script>\
                    <p><a href=\"https://example.com\">Shop</a> \
                    <a href=\"javascript:alert(1)\">Blog</a></p>\
                    <body onload=\"alert(1)\">";
        let (data, _) = render(request, extension, html, &[]).expect("it renders");
        for name in [
            &b"/JavaScript"[..],
            b"/JS",
            b"/Launch",
            b"/OpenAction",
            b"/AA",
            b"/EmbeddedFile",
        ] {
            assert!(
                !contains(&data, name),
                "the output carries `{}`",
                String::from_utf8_lossy(name)
            );
        }
        assert!(
            !contains(&data, b"app.alert"),
            "the script's text is in the output"
        );
    });
}

#[test]
fn html_nested_past_the_element_cap_flattens_as_core_html_does() {
    run(|request, extension| {
        let depth = 10_000;
        let html = format!(
            "{}<p>Shop</p>{}",
            "<div>".repeat(depth),
            "</div>".repeat(depth)
        );
        let (data, pages) =
            render(request, extension, &html, &[]).expect("nesting past the cap still renders");
        assert!(data.starts_with(b"%PDF-"));
        assert_eq!(pages, 1, "the flattened document is one paragraph");
    });
}

/// `text` decoded as standard base64.
fn base64(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in text.bytes().filter(|byte| *byte != b'=') {
        let value = ALPHABET
            .iter()
            .position(|letter| *letter == byte)
            .expect("standard base64");
        buffer = (buffer << 6) | u32::try_from(value).unwrap();
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).unwrap());
        }
    }
    out
}
