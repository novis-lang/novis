//! The PDF component as `nvs-ext` sees it (`rule:packaging/the-first-party-components-are-built-in`,
//! `rule:core-classes/pdf-render-has-no-io`, `rule:core-classes/pdf-output-is-inert`):
//! `Novis\Pdf\Engine` is in the binary and renders with no `[[extension]]` entry and no grant. A
//! paragraph renders with the default fonts, a reference resolves against the asset map and `data:`
//! alone, and a `file:`, `http:` or unknown name throws naming it. Two renders of one document are
//! byte-equal, the output carries no action a reader runs, and HTML nested past the parser's
//! element cap flattens as `Core\Html`'s parse does.
//!
//! A render with a header or footer function is the `renderFirst` and `renderSecond` passes
//! (`rule:core-classes/pdf-header-is-a-page-function`): two crossings, each page's facts and
//! marks and a kept body from the first, and the second drawing markup over that body's margins
//! without moving it. Two first passes keep two bodies, so a render inside a header function
//! leaves the outer one's as it was.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::{PDF, PDF_CLASS, PDF_SHA256};
use nvs_ext::call::{Error, Failure, Host, Limit, Meter, Request};
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

/// The document of `html` with `assets`, as `Novis\Pdf\Document::render` builds it.
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

/// The first pass of `html`, with the header height `height` when there is one.
fn first(html: &str, height: Option<&str>) -> Value {
    shape(vec![
        ("document", whole(html, &[])),
        (
            "headerHeight",
            height.map_or(Value::Null, |h| Value::String(h.to_owned())),
        ),
        ("footerHeight", Value::Null),
    ])
}

/// The second pass's margins, with one header and footer for each page.
fn second(pages: &[(Option<&str>, Option<&str>)]) -> Value {
    let text = |part: Option<&str>| part.map_or(Value::Null, |p| Value::String(p.to_owned()));
    let pages = pages
        .iter()
        .enumerate()
        .map(|(at, (header, footer))| {
            (
                Key::Int(i64::try_from(at).unwrap()),
                shape(vec![
                    ("header", text(*header)),
                    ("footer", text(*footer)),
                    ("slots", Value::Array(Vec::new())),
                ]),
            )
        })
        .collect();
    shape(vec![("pages", Value::Array(pages))])
}

/// The body `Engine::renderFirst` keeps for `pass`, and the pages' facts.
fn lay_out(
    request: &Request,
    extension: &Extension,
    pass: Value,
) -> Result<(Value, Value), Failure> {
    let result = block_on(request.call_values(extension, "renderFirst", vec![pass]))?
        .expect("`renderFirst` returns a value");
    Ok((field(&result, "body"), field(&result, "facts")))
}

/// The whole result of `Engine::renderSecond` for `body` and `margins`.
fn write(
    request: &Request,
    extension: &Extension,
    body: &Value,
    margins: Value,
) -> Result<Value, Failure> {
    Ok(
        block_on(request.call_values(extension, "renderSecond", vec![body.clone(), margins]))?
            .expect("`renderSecond` returns a value"),
    )
}

/// The values of the list `value`, in order.
fn items(value: &Value) -> Vec<Value> {
    let Value::Array(pairs) = value else {
        panic!("a list was expected, not {value:?}");
    };
    pairs.iter().map(|(_, value)| value.clone()).collect()
}

fn text(value: &Value) -> String {
    let Value::String(text) = value else {
        panic!("a string was expected, not {value:?}");
    };
    text.clone()
}

/// Each `{name, value}` of the list `value`.
fn pairs(value: &Value) -> Vec<(String, String)> {
    items(value)
        .iter()
        .map(|pair| (text(&field(pair, "name")), text(&field(pair, "value"))))
        .collect()
}

fn bytes(value: &Value) -> Vec<u8> {
    let Value::Bytes(data) = field(value, "data") else {
        panic!("`data` is not bytes");
    };
    data
}

/// A document of three pages, each `<h2>` starting a page and setting the `chapter` string,
/// with the marks `amount` on each row.
const THREE_PAGES: &str = "<style>h2 { break-before: page; string-set: chapter content() }</style>\
    <h1>Shop</h1><p data-mark=\"amount:1.50\">Tea</p>\
    <h2>Books</h2><p data-mark=\"amount:12.00;count:1\">A book</p>\
    <p data-mark=\"amount:3.25\">A map</p>\
    <h2>Toys</h2><p data-mark=\"amount:0.75\">A ball</p>";

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

// covers: Novis\Pdf\Engine::render
// covers: Novis\Pdf\Document::fromHtml
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

// covers: Novis\Pdf\Document::asset
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

// covers: Novis\Pdf\Document::render
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

// covers: Novis\Pdf\Page::constructor
// covers: Novis\Pdf\Page::isFirst
// covers: Novis\Pdf\Page::isLast
// covers: Novis\Pdf\Page::string
#[test]
fn a_header_function_runs_once_per_page_with_its_number_and_count() {
    run(|request, extension| {
        let (body, facts) =
            lay_out(request, extension, first(THREE_PAGES, None)).expect("it lays out");
        assert!(bytes(&facts).is_empty(), "the first pass wrote a file");
        assert_eq!(field(&facts, "pageCount"), Value::Uint(3));
        let pages = items(&field(&facts, "pages"));
        let numbers: Vec<Value> = pages.iter().map(|page| field(page, "number")).collect();
        assert_eq!(numbers, [Value::Uint(1), Value::Uint(2), Value::Uint(3)]);
        let chapters: Vec<Vec<(String, String)>> = pages
            .iter()
            .map(|page| pairs(&field(page, "strings")))
            .collect();
        assert_eq!(chapters[0], []);
        assert_eq!(chapters[1], [("chapter".to_owned(), "Books".to_owned())]);
        assert_eq!(chapters[2], [("chapter".to_owned(), "Toys".to_owned())]);
        let header = |n: u64| format!("<p>Shop, page {n} of 3</p>");
        let headers: Vec<String> = (1..=3).map(header).collect();
        let markup: Vec<_> = headers.iter().map(|h| (Some(h.as_str()), None)).collect();
        let rendered = write(request, extension, &body, second(&markup)).expect("it renders");
        assert!(bytes(&rendered).starts_with(b"%PDF-"));
        assert_eq!(field(&rendered, "pageCount"), Value::Uint(3));
        assert_eq!(items(&field(&rendered, "overflows")), []);
    });
}

// covers: Novis\Pdf\Page::marks
// covers: Novis\Pdf\Page::marksUpTo
#[test]
fn marks_on_a_page_and_up_to_it_reach_the_function() {
    run(|request, extension| {
        let (_, facts) =
            lay_out(request, extension, first(THREE_PAGES, None)).expect("it lays out");
        let marks: Vec<Vec<(String, String)>> = items(&field(&facts, "pages"))
            .iter()
            .map(|page| pairs(&field(page, "marks")))
            .collect();
        let pair = |name: &str, value: &str| (name.to_owned(), value.to_owned());
        assert_eq!(marks[0], [pair("amount", "1.50")]);
        assert_eq!(
            marks[1],
            [
                pair("amount", "12.00"),
                pair("count", "1"),
                pair("amount", "3.25")
            ]
        );
        assert_eq!(marks[2], [pair("amount", "0.75")]);
    });
}

#[test]
fn a_header_function_costs_two_crossings() {
    run(|request, extension| {
        let before = request.crossings();
        let (body, _) = lay_out(request, extension, first(THREE_PAGES, None)).expect("it lays out");
        let markup = [(Some("<p>Shop</p>"), Some("<p>Blog</p>")); 3];
        let rendered = write(request, extension, &body, second(&markup)).expect("it renders");
        assert!(bytes(&rendered).starts_with(b"%PDF-"));
        assert_eq!(request.crossings() - before, 2);
        let message = match write(request, extension, &body, second(&markup)) {
            Err(Failure::Error(Error::Invalid(message))) => message,
            other => panic!("a body written twice was not `invalid`: {other:?}"),
        };
        assert!(message.contains("written once"), "{message}");
    });
}

#[test]
fn a_first_pass_inside_another_keeps_its_own_body() {
    run(|request, extension| {
        let outer = format!("<p>{}</p>", "Thank you for your order. ".repeat(400));
        let (whole, count) = render(request, extension, &outer, &[]).expect("it renders");
        let pages = usize::try_from(count).unwrap();
        let (outer_body, _) =
            lay_out(request, extension, first(&outer, None)).expect("it lays out");
        let (inner_body, inner) =
            lay_out(request, extension, first(THREE_PAGES, None)).expect("it lays out");
        let markup = [(Some("<p>Shop</p>"), None); 3];
        let drawn =
            write(request, extension, &inner_body, second(&markup)).expect("the inner renders");
        assert_eq!(field(&drawn, "pageCount"), field(&inner, "pageCount"));
        let empty = write(
            request,
            extension,
            &outer_body,
            second(&vec![(None, None); pages]),
        )
        .expect("the outer renders after the inner");
        assert!(
            bytes(&empty) == whole,
            "the outer body changed when another render ran inside it"
        );
    });
}

// covers: Novis\Pdf\Document::header
#[test]
fn a_header_never_moves_the_body() {
    run(|request, extension| {
        let html = format!("<p>{}</p>", "Thank you for your order. ".repeat(400));
        let (whole, count) = render(request, extension, &html, &[]).expect("it renders");
        let (body, facts) = lay_out(request, extension, first(&html, None)).expect("it lays out");
        assert_eq!(field(&facts, "pageCount"), Value::Uint(count));
        let pages = usize::try_from(count).unwrap();
        let empty = write(
            request,
            extension,
            &body,
            second(&vec![(None, None); pages]),
        )
        .expect("it renders");
        assert!(
            bytes(&empty) == whole,
            "a second pass with no markup differs from a whole render"
        );
        let (body, _) = lay_out(request, extension, first(&html, None)).expect("it lays out");
        let markup = vec![(Some("<h1>Shop</h1>"), Some("<p>Blog</p>")); pages];
        let drawn = write(request, extension, &body, second(&markup)).expect("it renders");
        assert_eq!(field(&drawn, "pageCount"), Value::Uint(count));
        assert!(bytes(&drawn) != whole, "the header was not drawn");
        let (_, taller) =
            lay_out(request, extension, first(&html, Some("10cm"))).expect("it lays out");
        let Value::Uint(more) = field(&taller, "pageCount") else {
            panic!("`pageCount` is not a uint");
        };
        assert!(
            more > count,
            "a taller header margin did not shorten each page"
        );
        let message = match lay_out(request, extension, first(&html, Some("tall"))) {
            Err(Failure::Error(Error::Invalid(message))) => message,
            other => panic!("a height that is not a length was not `invalid`: {other:?}"),
        };
        assert!(message.contains("`tall`"), "{message}");
    });
}

// covers: Novis\Pdf\Rendered::constructor
#[test]
fn a_header_taller_than_its_margin_is_reported() {
    run(|request, extension| {
        let html = "<style>@page { margin-top: 1cm }</style><p>Shop</p>";
        let (body, _) = lay_out(request, extension, first(html, None)).expect("it lays out");
        let tall = "<p>Shop</p><p>Blog</p><p>Shop</p><p>Blog</p>";
        let rendered = write(
            request,
            extension,
            &body,
            second(&[(Some(tall), Some("<p>Blog</p>"))]),
        )
        .expect("it renders");
        let overflows = items(&field(&rendered, "overflows"));
        assert_eq!(overflows.len(), 1, "{overflows:?}");
        assert_eq!(field(&overflows[0], "page"), Value::Uint(1));
        assert_eq!(text(&field(&overflows[0], "area")), "header");
        assert_eq!(text(&field(&overflows[0], "text")), tall);
    });
}

// covers: Novis\Pdf\Document::footer
#[test]
fn a_function_header_wins_over_a_margin_box() {
    run(|request, extension| {
        let body = "<p>Shop</p>";
        let boxed =
            "<style>@page { @top-center { content: 'Blog Zebra Quilt' } }</style><p>Shop</p>";
        let (plain, _) = render(request, extension, body, &[]).expect("it renders");
        let (with_box, _) = render(request, extension, boxed, &[]).expect("it renders");
        assert!(plain != with_box, "the margin box was not drawn");
        let header = [(Some("<p>Shop</p>"), None)];
        let over = |html: &str, markup: &[(Option<&str>, Option<&str>)]| {
            let (kept, _) = lay_out(request, extension, first(html, None)).expect("it lays out");
            bytes(&write(request, extension, &kept, second(markup)).expect("it renders"))
        };
        let over_plain = over(body, &header);
        let over_box = over(boxed, &header);
        assert!(
            over_plain == over_box,
            "the margin box was drawn beside the function's header"
        );
        let footer = [(None, Some("<p>Shop</p>"))];
        let kept = over(boxed, &footer);
        let without = over(body, &footer);
        assert!(kept != without, "a footer took off a box in the top margin");
    });
}

/// The second pass's margins for a one-page document with no header or footer, and `fills` as
/// its slots.
fn slotted(fills: &[(&str, &str)]) -> Value {
    let slots = fills
        .iter()
        .enumerate()
        .map(|(at, (name, value))| {
            (
                Key::Int(i64::try_from(at).unwrap()),
                shape(vec![
                    ("name", Value::String((*name).to_owned())),
                    ("value", Value::String((*value).to_owned())),
                ]),
            )
        })
        .collect();
    let page = shape(vec![
        ("header", Value::Null),
        ("footer", Value::Null),
        ("slots", Value::Array(slots)),
    ]);
    shape(vec![("pages", Value::Array(vec![(Key::Int(0), page)]))])
}

// covers: Novis\Pdf\Document::slots
#[test]
fn a_slot_fill_is_drawn_in_its_cell_and_a_tall_one_is_reported() {
    run(|request, extension| {
        let html = "<table style=\"width: 100%\"><tbody><tr><td>Total</td><td><slot name=\"total\"></slot></td></tr></tbody></table>";
        let (whole, count) = render(request, extension, html, &[]).expect("it renders");
        assert_eq!(count, 1);
        let (body, _) = lay_out(request, extension, first(html, None)).expect("it lays out");
        let empty = bytes(&write(request, extension, &body, slotted(&[])).expect("it renders"));
        assert!(
            empty == whole,
            "a second pass with no fill differs from a whole render"
        );
        let (body, _) = lay_out(request, extension, first(html, None)).expect("it lays out");
        let drawn = write(
            request,
            extension,
            &body,
            slotted(&[("total", "<b>12.50</b>")]),
        )
        .expect("it renders");
        assert_eq!(field(&drawn, "pageCount"), Value::Uint(1));
        assert!(bytes(&drawn) != whole, "the slot was not filled");
        assert!(items(&field(&drawn, "overflows")).is_empty());
        let (body, _) = lay_out(request, extension, first(html, None)).expect("it lays out");
        let tall = "<p>Shop</p><p>Blog</p><p>Shop</p><p>Blog</p>";
        let over = write(
            request,
            extension,
            &body,
            slotted(&[("total", tall), ("missing", "<p>Blog</p>")]),
        )
        .expect("it renders");
        let overflows = items(&field(&over, "overflows"));
        assert_eq!(overflows.len(), 1, "{overflows:?}");
        assert_eq!(field(&overflows[0], "page"), Value::Uint(1));
        assert_eq!(text(&field(&overflows[0], "area")), "total");
        assert_eq!(text(&field(&overflows[0], "text")), tall);
    });
}

/// A report with one table of `rows` orders, as `benches/members/novis/Pdf-Document` renders it.
fn report(rows: usize) -> String {
    let rows: String = (0..rows)
        .map(|at| format!("<tr><td>Order {at}</td><td>Shop</td><td>12.50 EUR</td></tr>"))
        .collect();
    format!(
        "<h1>Orders</h1><table><thead><tr><th>Order</th><th>Customer</th><th>Total</th></tr>\
         </thead><tbody>{rows}</tbody></table>"
    )
}

/// The render of `report(8400)` in a request of `memory` bytes, or of no limit for `None`, and the
/// bytes its request is charged once the call has returned. Its PDF is about 4 MiB, and its guest
/// grows to about 150 MiB.
fn render_report(memory: Option<u64>) -> (Result<(Vec<u8>, u64), Failure>, u64) {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let extension = pdf(&builtins)
        .extension()
        .expect("the PDF component compiles and checks");
    let meter = Arc::new(Meter::new(Duration::from_secs(120), memory));
    let request = host.request(meter.clone());
    let out = render(&request, extension, &report(8400), &[]);
    (out, meter.charged())
}

#[test]
fn a_200_page_report_crosses_under_the_requests_memory() {
    let (data, pages) = render_report(Some(1 << 30)).0.expect("it renders");
    assert!(pages >= 200, "the report has {pages} pages");
    assert!(data.len() > 3 << 20, "the PDF is {} bytes", data.len());
}

#[test]
fn a_result_the_request_cannot_afford_reaches_the_memory_limit() {
    // The guest and the HTML fit in this budget, and the result's 4 MiB do not.
    let (out, guest) = render_report(None);
    let (data, _) = out.expect("it renders");
    let budget = guest + report(8400).len() as u64 + data.len() as u64 / 2;
    match render_report(Some(budget)).0 {
        Err(Failure::Limit(Limit::Memory)) => {}
        other => panic!("the render did not reach the memory limit: {other:?}"),
    }
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
