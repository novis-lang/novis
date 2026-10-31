//! The `whole` pass on the host: a render's file, its pages, its determinism, and the asset
//! map's refusals (`rule:core-classes/pdf-render-has-no-io`, `rule:core-classes/pdf-output-is-inert`).

use base64::Engine as _;
use nvs_pdf::{Asset, Document, Error, Instant, Options, Standard, render};

fn html(html: &str) -> Document {
    Document {
        html: html.to_owned(),
        ..Document::default()
    }
}

/// A 4 by 3 PNG, opaque red.
fn png() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 4, 3);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[255, 0, 0, 255].repeat(12))
            .unwrap();
    }
    out
}

fn data_png() -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png())
    )
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn a_paragraph_is_one_page() {
    let rendered = render(&html("<p>Hello, world.</p>")).unwrap();
    assert!(rendered.data.starts_with(b"%PDF-"));
    assert_eq!(rendered.pages.len(), 1);
    assert_eq!(rendered.pages[0].number, 1);
    assert!(contains(&rendered.data, b"/FontFile2"));
}

#[test]
fn a_script_and_the_head_draw_nothing() {
    let document = html(
        "<head><title>t</title><style>p { color: red }</style></head>\
         <body><script>document.write('x')</script><noscript>n</noscript></body>",
    );
    let rendered = render(&document).unwrap();
    assert_eq!(rendered.pages.len(), 1);
    assert!(!contains(&rendered.data, b"/FontFile2"));
}

#[test]
fn an_empty_document_is_one_page() {
    let rendered = render(&html("")).unwrap();
    assert_eq!(rendered.pages.len(), 1);
}

#[test]
fn a_long_text_makes_more_than_one_page() {
    let paragraph =
        "<p>".to_owned() + &"Lorem ipsum dolor sit amet, consectetur. ".repeat(40) + "</p>";
    let rendered = render(&html(&paragraph.repeat(20))).unwrap();
    // About 1,700 characters a paragraph at about 90 a line and 50 lines a page.
    assert!(
        (5..=12).contains(&rendered.pages.len()),
        "{} pages",
        rendered.pages.len()
    );
    let numbers: Vec<u64> = rendered.pages.iter().map(|p| p.number).collect();
    assert_eq!(
        numbers,
        (1..=rendered.pages.len() as u64).collect::<Vec<_>>()
    );
}

#[test]
fn two_renders_of_one_document_are_byte_equal() {
    let mut document = html(&format!(
        "<h1>Invoice</h1><p>One <b>bold</b> and <i>italic</i> word.</p>\
         <ul><li>first</li><li>second</li></ul><pre>a  b\nc</pre><img src=\"{}\">",
        data_png()
    ));
    assert_eq!(render(&document).unwrap(), render(&document).unwrap());
    document.options.created = Some(Instant {
        seconds: 1_700_000_000,
        nanos: 0,
    });
    let dated = render(&document).unwrap();
    assert_eq!(dated, render(&document).unwrap());
    assert!(contains(&dated.data, b"D:20231114"));
}

#[test]
fn a_document_has_no_date_unless_given_one() {
    let rendered = render(&html("<p>x</p>")).unwrap();
    assert!(!contains(&rendered.data, b"/CreationDate"));
}

#[test]
fn a_data_png_renders() {
    let rendered = render(&html(&format!("<p><img src=\"{}\"></p>", data_png()))).unwrap();
    assert_eq!(rendered.pages.len(), 1);
    assert!(contains(&rendered.data, b"/Image"));
}

#[test]
fn a_named_png_renders() {
    let mut document = html("<img src=\"logo.png\">");
    document.assets.push(Asset {
        name: "logo.png".to_owned(),
        data: png(),
    });
    assert!(contains(&render(&document).unwrap().data, b"/Image"));
}

#[test]
fn bytes_that_are_not_an_image_return_parse_naming_the_asset() {
    let mut document = html("<img src=\"logo.png\">");
    document.assets.push(Asset {
        name: "logo.png".to_owned(),
        data: b"not an image".to_vec(),
    });
    match render(&document) {
        Err(Error::Parse(message)) => assert!(message.contains("`logo.png`"), "{message}"),
        other => panic!("{other:?}"),
    }
}

const REFUSED: [&str; 7] = [
    "file:///etc/passwd",
    "http://example.com/x.png",
    "https://example.com/x.png",
    "/etc/passwd",
    "C:\\x.png",
    "\\\\host\\share\\x.png",
    "missing.png",
];

fn assert_refused(document: &str, reference: &str) {
    match render(&html(document)) {
        Err(Error::Invalid(message)) => assert_eq!(
            message,
            format!(
                "`{reference}` is not in the asset map: a render reads only its assets and `data:` URIs"
            ),
        ),
        other => panic!("{document}: {other:?}"),
    }
}

#[test]
fn an_img_outside_the_map_is_refused() {
    for reference in REFUSED {
        assert_refused(&format!("<img src='{reference}'>"), reference);
    }
}

#[test]
fn a_css_url_outside_the_map_is_refused() {
    for reference in REFUSED {
        // A backslash starts an escape in CSS, so the stylesheet writes each one twice.
        let css = reference.replace('\\', "\\\\");
        assert_refused(
            &format!("<style>p {{ background: url('{css}') }}</style><p>x</p>"),
            reference,
        );
        assert_refused(
            &format!("<p style=\"background-image: url({css})\">x</p>"),
            reference,
        );
        assert_refused(&format!("<style>@import '{css}';</style>"), reference);
    }
}

#[test]
fn a_stylesheet_link_outside_the_map_is_refused_and_its_urls_are_checked() {
    for reference in REFUSED {
        assert_refused(
            &format!("<link rel=stylesheet href='{reference}'><p>x</p>"),
            reference,
        );
    }
    let mut document = html("<link rel='alternate Stylesheet' href='a.css'><p>x</p>");
    document.assets.push(Asset {
        name: "a.css".to_owned(),
        data: b"p { background: url(http://example.com/x.png) }".to_vec(),
    });
    match render(&document) {
        Err(Error::Invalid(message)) => assert!(message.contains("`http://example.com/x.png`")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_svg_image_or_use_outside_the_map_is_refused() {
    for reference in REFUSED {
        assert_refused(
            &format!("<svg><image href='{reference}' width=10 height=10/></svg>"),
            reference,
        );
        assert_refused(
            &format!("<svg><image xlink:href='{reference}'/></svg>"),
            reference,
        );
        assert_refused(&format!("<svg><use href='{reference}'/></svg>"), reference);
    }
}

#[test]
fn a_fragment_a_link_and_a_data_uri_are_not_refused() {
    let document = format!(
        "<svg><defs><circle id=c r=1 /></defs><use href='#c'/></svg>\
         <a href='http://example.com/'>link</a>\
         <style>p {{ background: url('{0}') }}</style><svg><image href='{0}'/></svg>",
        data_png()
    );
    render(&html(&document)).unwrap();
}

#[test]
fn the_output_has_no_script_launch_or_open_action() {
    let document = html(
        "<script>app.alert(1)</script><p onclick='x()'>Hi</p>\
         <a href='javascript:alert(1)'>a</a>",
    );
    let data = render(&document).unwrap().data;
    for needle in [
        &b"/JavaScript"[..],
        b"/JS",
        b"/Launch",
        b"/OpenAction",
        b"/AA",
    ] {
        assert!(
            !contains(&data, needle),
            "{}",
            String::from_utf8_lossy(needle)
        );
    }
}

#[test]
fn ten_thousand_nested_divs_render() {
    let document = "<div>".repeat(10_000) + "deep" + &"</div>".repeat(10_000);
    let rendered = render(&html(&document)).unwrap();
    assert_eq!(rendered.pages.len(), 1);
}

#[test]
fn an_inline_block_alone_on_its_line_renders() {
    let rendered = render(&html("<div style=\"display:inline-block\">deep")).unwrap();
    assert_eq!(rendered.pages.len(), 1);
}

#[test]
fn layout_nested_past_the_depth_bound_throws() {
    // A debug build's frames are many times a release build's, so the test runs on a stack of
    // its own.
    let run = || {
        for open in [
            "<table><tr><td>",
            "<div style=\"display:flex\">",
            "<div style=\"display:grid\">",
        ] {
            let within = render(&html(&(open.repeat(20) + "deep")));
            assert_eq!(within.map(|r| r.pages.len()), Ok(1), "{open}");
            let Err(Error::Invalid(message)) = render(&html(&(open.repeat(40) + "deep"))) else {
                panic!("{open} nested 40 deep did not return Invalid");
            };
            assert!(message.contains("more than 32 levels"), "{message}");
        }
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn an_invoice_xml_nested_past_the_depth_bound_throws() {
    let nested = |depth: usize| {
        let mut document = html("<p>x</p>");
        document.options = Options {
            standard: Some(Standard::PdfA3b),
            invoice_xml: Some("<a>".repeat(depth) + &"</a>".repeat(depth)),
            created: Some(Instant {
                seconds: 1_767_225_600,
                nanos: 0,
            }),
            ..Options::default()
        };
        render(&document)
    };
    nested(256).unwrap();
    for depth in [257, 100_000] {
        match nested(depth) {
            Err(Error::Parse(message)) => assert!(message.contains("256 levels"), "{message}"),
            other => panic!("{depth} levels: {other:?}"),
        }
    }
}

#[test]
fn an_invoice_needs_pdf_a3b_and_pdf_a_has_no_password() {
    let mut document = html("<p>x</p>");
    document.options = Options {
        invoice_xml: Some("<x/>".to_owned()),
        created: Some(Instant {
            seconds: 1_767_225_600,
            nanos: 0,
        }),
        ..Options::default()
    };
    assert!(matches!(render(&document), Err(Error::Invalid(_))));
    document.options.standard = Some(Standard::PdfA3b);
    render(&document).unwrap();
    document.options.password = Some("secret".to_owned());
    assert!(matches!(render(&document), Err(Error::Invalid(_))));
    document.html = "<html lang=\"en\"><head><title>Shop</title></head><p>x</p></html>".to_owned();
    document.options = Options {
        standard: Some(Standard::PdfUa1),
        password: Some("secret".to_owned()),
        ..Options::default()
    };
    render(&document).unwrap();
}
