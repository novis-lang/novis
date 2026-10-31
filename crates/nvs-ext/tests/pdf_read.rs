//! Existing PDFs as `nvs-ext` sees the PDF component (`rule:core-classes/pdf-reader-and-merge`): a
//! page of one is placed in an `<img>` and under every page as vector content, `merge` combines,
//! selects, repeats and reorders pages, `read` returns the page count, the metadata and the
//! attachments with the Factur-X XML and the file it opened, `read-text` returns a page's text of
//! that file in the same request, and an encrypted source
//! throws a parse error in each of them. Every PDF here is one the component rendered, and each
//! result is checked by reading it back through the component.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::PDF_CLASS;
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

fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(name, value)| (Key::String(name.to_owned()), value))
            .collect(),
    )
}

fn list(values: Vec<Value>) -> Value {
    Value::Array(
        values
            .into_iter()
            .enumerate()
            .map(|(at, value)| (Key::Int(i64::try_from(at).unwrap()), value))
            .collect(),
    )
}

/// The value under `name` in the shape `value`.
fn field(value: &Value, name: &str) -> Value {
    let Value::Array(fields) = value else {
        panic!("{value:?} is not a shape");
    };
    fields
        .iter()
        .find(|(key, _)| matches!(key, Key::String(key) if key == name))
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| panic!("the shape has no `{name}`"))
}

/// What the PDF component's `method` returns for `args`, in a request of its own.
fn call(method: &str, args: Vec<Value>) -> Result<Value, Failure> {
    in_request(|request, extension| {
        block_on(request.call_values(extension, method, args))
            .map(|value| value.expect("the export returns a value"))
    })
}

/// What `calls` returns, given a request of its own and the PDF component.
fn in_request<T>(
    calls: impl FnOnce(&Request, &Extension) -> Result<T, Failure>,
) -> Result<T, Failure> {
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
    let result = calls(&request, extension);
    block_on(request.end()).expect("the request ends");
    result
}

/// The file `Engine::render` returns for `html` with `assets`, and `options` over no option.
fn render_with(
    html: &str,
    assets: &[(&str, &[u8])],
    options: Vec<(&str, Value)>,
) -> Result<Vec<u8>, Failure> {
    let assets = assets
        .iter()
        .map(|(name, data)| {
            shape(vec![
                ("name", Value::String((*name).to_owned())),
                ("data", Value::Bytes(data.to_vec())),
            ])
        })
        .collect();
    let mut all = vec![
        ("standard", Value::Null),
        ("password", Value::Null),
        ("invoiceXml", Value::Null),
        ("created", Value::Null),
    ];
    for (name, value) in options {
        all.iter_mut().find(|(each, _)| *each == name).unwrap().1 = value;
    }
    let pass = shape(vec![
        ("html", Value::String(html.to_owned())),
        ("assets", list(assets)),
        ("options", shape(all)),
    ]);
    match field(&call("render", vec![pass])?, "data") {
        Value::Bytes(data) => Ok(data),
        other => panic!("`data` is {other:?}"),
    }
}

fn render(html: &str) -> Vec<u8> {
    render_with(html, &[], Vec::new()).expect("it renders")
}

/// A document of `pages` pages, each with one heading naming its number.
fn numbered(pages: u64) -> Vec<u8> {
    let body: String = (1..=pages)
        .map(|n| format!("<h1 style=\"break-before: page\">Shop page {n}</h1>"))
        .collect();
    render(&body)
}

fn merge(parts: Vec<(Vec<u8>, Option<Vec<u64>>)>) -> Result<Vec<u8>, Failure> {
    let parts = parts
        .into_iter()
        .map(|(data, pages)| {
            shape(vec![
                ("data", Value::Bytes(data)),
                (
                    "pages",
                    pages.map_or(Value::Null, |pages| {
                        list(pages.into_iter().map(Value::Uint).collect())
                    }),
                ),
            ])
        })
        .collect();
    match call("merge", vec![list(parts), unbounded()])? {
        Value::Bytes(data) => Ok(data),
        other => panic!("`merge` returned {other:?}"),
    }
}

/// A `decompression-bound` with neither half set, which the component reads as the shipped
/// default.
fn unbounded() -> Value {
    shape(vec![("maxBytes", Value::Null), ("maxRatio", Value::Null)])
}

fn read(data: Vec<u8>) -> Result<Value, Failure> {
    call("read", vec![Value::Bytes(data), unbounded()])
}

/// The text of `pages` of `data`: `read` opens the file, and `readText` reads the file `read`
/// returned, in the same request.
fn text(data: Vec<u8>, pages: &[u64]) -> Result<Vec<String>, Failure> {
    let pages = list(pages.iter().copied().map(Value::Uint).collect());
    let texts = in_request(|request, extension| {
        let info = block_on(request.call_values(
            extension,
            "read",
            vec![Value::Bytes(data), unbounded()],
        ))?
        .expect("`read` returns a value");
        let args = vec![field(&info, "file"), unbounded(), pages];
        Ok(block_on(request.call_values(extension, "readText", args))?
            .expect("`readText` returns a value"))
    })?;
    let Value::Array(texts) = texts else {
        panic!("`readText` returned no list");
    };
    Ok(texts
        .into_iter()
        .map(|(_, value)| match value {
            Value::String(text) => text,
            other => panic!("a page's text is {other:?}"),
        })
        .collect())
}

fn page_count(info: &Value) -> u64 {
    match field(info, "pageCount") {
        Value::Uint(count) => count,
        Value::Int(count) => u64::try_from(count).unwrap(),
        other => panic!("`pageCount` is {other:?}"),
    }
}

/// How often `needle` occurs in `haystack`.
fn count(haystack: &[u8], needle: &str) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle.as_bytes())
        .count()
}

#[test]
fn a_pdf_page_in_an_img_is_placed_as_vector_content() {
    let terms = numbered(3);
    let data = render_with(
        "<p>Before</p><img src=\"terms.pdf#page=2\" style=\"width: 100pt\"><p>After</p>",
        &[("terms.pdf", &terms)],
        Vec::new(),
    )
    .expect("it renders");
    // The page is a form XObject, so its text is still text and no pixel is drawn.
    assert_eq!(count(&data, "/Subtype/Form"), 1);
    assert_eq!(count(&data, "/Subtype/Image"), 0);
    assert_eq!(
        text(data, &[1]).expect("it reads"),
        ["Before\nShop page 2\nAfter"]
    );
    let refused = render_with(
        "<img src=\"terms.pdf#page=4\">",
        &[("terms.pdf", &terms)],
        Vec::new(),
    );
    let Err(Failure::Error(Error::Invalid(message))) = refused else {
        panic!("page 4 of 3 gave {refused:?}");
    };
    assert!(
        message.contains("terms.pdf") && message.contains("3 pages"),
        "{message}"
    );
}

#[test]
fn a_pdf_letterhead_is_the_page_background() {
    let letterhead = render("<p>Shop letterhead</p>");
    let data = render_with(
        "<style>@page { background-image: url(letterhead.pdf) }</style>\
         <p>One</p><p style=\"break-before: page\">Two</p>",
        &[("letterhead.pdf", &letterhead)],
        Vec::new(),
    )
    .expect("it renders");
    assert_eq!(
        count(&data, "/Subtype/Form"),
        1,
        "one XObject for both pages"
    );
    assert_eq!(
        text(data, &[1, 2]).expect("it reads"),
        ["Shop letterhead\nOne", "Shop letterhead\nTwo"]
    );
}

// covers: Novis\Pdf\Pages::of
// covers: Novis\Pdf\Pages::merge
// covers: Novis\Pdf\Engine::merge
#[test]
fn merge_combines_selects_and_reorders_pages() {
    let parts = || {
        vec![
            (numbered(3), Some(vec![3, 1])),
            (numbered(2), None),
            (numbered(1), Some(vec![1, 1])),
        ]
    };
    let merged = merge(parts()).expect("it merges");
    assert_eq!(page_count(&read(merged.clone()).expect("it reads")), 6);
    assert_eq!(
        text(merged.clone(), &[1, 2, 3, 4, 5, 6]).expect("it reads"),
        [
            "Shop page 3",
            "Shop page 1",
            "Shop page 1",
            "Shop page 2",
            "Shop page 1",
            "Shop page 1"
        ]
    );
    assert_eq!(merge(parts()).expect("it merges"), merged, "byte-equal");
    let refused = merge(vec![(numbered(2), Some(vec![3]))]);
    let Err(Failure::Error(Error::Invalid(message))) = refused else {
        panic!("page 3 of 2 gave {refused:?}");
    };
    assert!(message.contains("2 pages"), "{message}");
}

// covers: Novis\Pdf\Reader::open
// covers: Novis\Pdf\Engine::read
#[test]
fn reader_counts_pages_and_reads_metadata() {
    let data = render(
        "<html><head><title>Shop invoice</title><meta name=\"author\" content=\"Shop\">\
         <meta name=\"keywords\" content=\"invoice, Shop\"></head>\
         <body><p>One</p><p style=\"break-before: page\">Two</p></body></html>",
    );
    let info = read(data).expect("it reads");
    assert_eq!(page_count(&info), 2);
    let metadata = field(&info, "metadata");
    assert_eq!(
        field(&metadata, "title"),
        Value::String("Shop invoice".to_owned())
    );
    assert_eq!(field(&metadata, "author"), Value::String("Shop".to_owned()));
    assert_eq!(field(&metadata, "subject"), Value::Null);
    assert_eq!(field(&info, "invoiceXml"), Value::Null);
}

const INVOICE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<rsm:CrossIndustryInvoice \
xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\" \
xmlns:ram=\"urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100\">\
<rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter>\
<ram:ID>urn:cen.eu:en16931:2017</ram:ID>\
</ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext>\
<rsm:ExchangedDocument><ram:ID>Shop-1</ram:ID><ram:TypeCode>380</ram:TypeCode>\
</rsm:ExchangedDocument></rsm:CrossIndustryInvoice>";

// covers: Novis\Pdf\Reader::attachments
// covers: Novis\Pdf\Reader::invoiceXml
#[test]
fn reader_returns_the_factur_x_attachment() {
    let created = Value::Core {
        class: "Core\\Time\\Instant".to_owned(),
        fields: vec![
            ("seconds".to_owned(), Value::Int(1_767_225_600)),
            ("nanos".to_owned(), Value::Int(0)),
        ],
    };
    let data = render_with(
        "<html lang=\"en\"><head><title>Shop</title></head><body><p>Invoice</p></body></html>",
        &[],
        vec![
            ("standard", Value::Case("PdfA3b".to_owned())),
            ("invoiceXml", Value::String(INVOICE.to_owned())),
            ("created", created),
        ],
    )
    .expect("it renders");
    let info = read(data).expect("it reads");
    assert_eq!(
        field(&info, "invoiceXml"),
        Value::String(INVOICE.to_owned())
    );
    let Value::Array(attachments) = field(&info, "attachments") else {
        panic!("`attachments` is not a list");
    };
    assert_eq!(attachments.len(), 1);
    let one = &attachments[0].1;
    assert_eq!(field(one, "name"), Value::String("factur-x.xml".to_owned()));
    assert_eq!(
        field(one, "data"),
        Value::Bytes(INVOICE.as_bytes().to_vec())
    );
}

// covers: Novis\Pdf\Reader::text
// covers: Novis\Pdf\Engine::readText
#[test]
fn reader_extracts_the_text_of_a_page() {
    let data = numbered(3);
    assert_eq!(
        text(data.clone(), &[3, 1]).expect("it reads"),
        ["Shop page 3", "Shop page 1"]
    );
    let refused = text(data, &[4]);
    let Err(Failure::Error(Error::Invalid(message))) = refused else {
        panic!("page 4 of 3 gave {refused:?}");
    };
    assert!(message.contains("3 pages"), "{message}");
}

#[test]
fn the_text_of_our_own_render_reads_back() {
    let data = render(
        "<h1>Invoice from Shop</h1><p>Total: 42.00 €</p>\
         <p style=\"font-family: serif\">Grüße aus Köln, ½ Preis</p>",
    );
    assert_eq!(
        text(data, &[1]).expect("it reads"),
        ["Invoice from Shop\nTotal: 42.00 €\nGrüße aus Köln, ½ Preis"]
    );
}

#[test]
fn an_encrypted_source_throws_in_merge_reader_and_placement() {
    let locked = render_with(
        "<p>Shop</p>",
        &[],
        vec![("password", Value::String("Shop-secret".to_owned()))],
    )
    .expect("it renders");
    let refused = |result: Result<(), Failure>| match result {
        Err(Failure::Error(Error::Parse(message))) => {
            assert!(message.contains("encrypted"), "{message}");
        }
        other => panic!("an encrypted source gave {other:?}"),
    };
    refused(read(locked.clone()).map(drop));
    refused(text(locked.clone(), &[1]).map(drop));
    refused(merge(vec![(locked.clone(), None)]).map(drop));
    refused(
        render_with(
            "<img src=\"locked.pdf\">",
            &[("locked.pdf", &locked)],
            Vec::new(),
        )
        .map(drop),
    );
}
