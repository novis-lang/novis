//! Hostile input a render meets: lengths that overflow to infinity, `var()` chains that double,
//! text of size 0, link and metadata text with PDF's own syntax in it, encoded schemes, and SVG
//! entities and `<use>` cycles. Each one renders, or returns an error, and never panics
//! (`rule:core-classes/pdf-css-subset`, `rule:core-classes/pdf-render-has-no-io`).

use std::collections::BTreeMap;

use hayro_interpret::hayro_syntax::object::{Array, Dict, String as PdfString};
use nvs_pdf::css::value::resolve_custom;
use nvs_pdf::read::Bound;
use nvs_pdf::{Asset, Document, Error, Instant, Options, Rendered, Standard, render};

fn rendered(html: &str) -> Rendered {
    render(&Document {
        html: html.to_owned(),
        ..Document::default()
    })
    .unwrap()
}

/// The properties `rendered` dropped, each with its value.
fn dropped(rendered: &Rendered) -> Vec<(String, String)> {
    rendered
        .dropped
        .iter()
        .map(|d| (d.property.clone(), d.value.clone()))
        .collect()
}

/// Whether the file's bytes contain `needle`.
fn contains(data: &[u8], needle: &str) -> bool {
    data.windows(needle.len()).any(|w| w == needle.as_bytes())
}

/// The width and height in points of the first page of the file `data`.
fn first_page_size(data: &[u8]) -> (f64, f64) {
    let pdf = nvs_pdf::read::open(data.to_vec()).unwrap();
    let rect = pdf.pages()[0].media_box();
    (rect.x1 - rect.x0, rect.y1 - rect.y0)
}

/// The URI of each link annotation of the file `data`, page by page.
fn link_uris(data: &[u8]) -> Vec<String> {
    let pdf = nvs_pdf::read::open(data.to_vec()).unwrap();
    let mut out = Vec::new();
    for page in pdf.pages().iter() {
        let Some(annots) = page.raw().get::<Array<'_>>(b"Annots") else {
            continue;
        };
        for annot in annots.iter::<Dict<'_>>() {
            if let Some(uri) = annot
                .get::<Dict<'_>>(b"A")
                .and_then(|action| action.get::<PdfString<'_>>(b"URI"))
            {
                out.push(String::from_utf8(uri.as_bytes().to_vec()).unwrap());
            }
        }
    }
    out
}

#[test]
fn a_length_that_overflows_to_infinity_is_dropped_and_reported() {
    let html = r#"<style>
        .wide { width: 1e39px }
        .calc { margin-left: calc(3e38pt * 10) }
        .huge { font-size: 3e38pt }
        .huge .huger { font-size: 100em }
        .tall { line-height: 1e40 }
    </style>
    <div class="wide">Shop</div>
    <div class="calc">Shop</div>
    <div class="huge"><span class="huger">Shop</span></div>
    <p class="tall">Shop</p>"#;
    let out = rendered(html);
    let dropped = dropped(&out);
    for property in ["width", "margin-left", "line-height"] {
        assert!(
            dropped.iter().any(|(p, _)| p == property),
            "{property} is not in {dropped:?}"
        );
    }
    // `3e38pt` is finite, and `100em` of it is not, so the inner `font-size` alone is dropped.
    assert!(dropped.contains(&("font-size".to_owned(), "100em".to_owned())));
    assert!(!dropped.contains(&("font-size".to_owned(), "3e38pt".to_owned())));
    assert!(!contains(&out.data, "NaN") && !contains(&out.data, "inf"));
}

#[test]
fn a_page_size_that_overflows_to_infinity_falls_back_to_a4_and_is_reported() {
    for size in ["1e39mm", "3e38in 1pt", "banana"] {
        let html = format!("<style>@page {{ size: {size} }}</style><p>Shop</p>");
        let out = rendered(&html);
        assert_eq!(
            dropped(&out),
            [("size".to_owned(), size.to_owned())],
            "size: {size}"
        );
        let (width, height) = first_page_size(&out.data);
        assert!(
            (width - 595.28).abs() < 0.01 && (height - 841.89).abs() < 0.01,
            "size: {size} is {width} by {height}"
        );
    }
    // A header height that overflows is not a margin.
    let header = nvs_pdf::first(
        &Document {
            html: "<p>Shop</p>".to_owned(),
            ..Document::default()
        },
        &nvs_pdf::Heights {
            header: Some("1e39mm".to_owned()),
            footer: None,
        },
    );
    assert!(matches!(header, Err(Error::Invalid(_))));
}

#[test]
fn a_var_chain_that_doubles_at_each_step_resolves_each_property_once() {
    // `--v20` names `--v19` twice, which names `--v18` twice, and so on down to `--v0`.
    let mut raw: BTreeMap<String, String> = BTreeMap::new();
    raw.insert("--v0".to_owned(), "x".to_owned());
    for n in 1..=20 {
        raw.insert(
            format!("--v{n}"),
            format!("var(--v{p}) var(--v{p})", p = n - 1),
        );
    }
    let names: Vec<&str> = raw.keys().map(String::as_str).collect();
    let mut reads: BTreeMap<String, usize> = BTreeMap::new();
    let resolved = resolve_custom(
        &names,
        &mut |name| {
            *reads.entry(name.to_owned()).or_default() += 1;
            raw.get(name).map(String::as_str)
        },
        &|_| None,
    );
    assert!(reads.values().all(|&n| n == 1), "{reads:?}");
    assert_eq!(reads.len(), 21);
    let top = resolved[names.iter().position(|n| *n == "--v20").unwrap()]
        .as_deref()
        .unwrap();
    // The text doubles at each step: 2^20 copies of `x`, with a space between each two.
    assert_eq!(top.len(), (1 << 21) - 1);

    // Thirty steps down to a property nobody sets: every step is invalid, and the render ends.
    let mut chain = String::from(":root { ");
    for n in 1..=30 {
        chain.push_str(&format!("--w{n}: var(--w{p}) var(--w{p}); ", p = n - 1));
    }
    chain.push('}');
    let html = format!("<style>{chain} p {{ width: var(--w30) }}</style><p>Shop</p>");
    let out = rendered(&html);
    assert_eq!(
        dropped(&out),
        [("width".to_owned(), "var(--w30)".to_owned())]
    );
}

#[test]
fn text_with_a_font_size_of_zero_draws_nothing_and_writes_no_nan() {
    let html = r#"<p>Shop <span style="font-size: 0">hidden</span> open</p>
        <p style="font-size: 0; letter-spacing: 1em">Lorem ipsum</p>
        <p style="font-size: calc(10pt - 10pt)">x</p>"#;
    let out = rendered(html);
    assert!(out.dropped.is_empty(), "{:?}", out.dropped);
    assert!(!contains(&out.data, "NaN"));
    let (reading, info) = nvs_pdf::read::read(out.data, Bound::default()).unwrap();
    assert_eq!(info.page_count, 1);
    let text = nvs_pdf::read::text(&reading, &[1], Bound::default()).unwrap();
    assert!(text[0].contains("Shop"), "{text:?}");
    assert!(
        !text[0].contains("hidden") && !text[0].contains("Lorem"),
        "{text:?}"
    );
}

#[test]
fn a_link_and_a_title_with_parentheses_backslashes_and_non_ascii_survive_a_round_trip() {
    let odd = r"Shop (é) \ % & < > ) (";
    let escaped = odd
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let uri = r"https://example.com/a(b)\c%28é?x=1&y=<2>";
    let html = format!(
        r#"<html lang="en"><head><title>{escaped}</title>
        <meta name="author" content="{escaped}">
        <meta name="description" content="{escaped}"></head>
        <body><h1>Shop</h1><p><a href="{href}">Lorem</a></p></body></html>"#,
        href = uri
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    );
    for standard in [None, Some(Standard::PdfA2b)] {
        let out = render(&Document {
            html: html.clone(),
            options: Options {
                standard,
                created: Some(Instant {
                    seconds: 1_767_225_600,
                    nanos: 0,
                }),
                ..Options::default()
            },
            ..Document::default()
        })
        .unwrap_or_else(|e| panic!("{standard:?}: {e:?}"));
        assert_eq!(link_uris(&out.data), [uri.to_owned()]);
        let (reading, info) = nvs_pdf::read::read(out.data, Bound::default()).unwrap();
        assert_eq!(info.metadata.title.as_deref(), Some(odd));
        assert_eq!(info.metadata.author.as_deref(), Some(odd));
        assert_eq!(info.metadata.subject.as_deref(), Some(odd));
        let text = nvs_pdf::read::text(&reading, &[1], Bound::default()).unwrap();
        assert!(text[0].contains("Lorem"), "{text:?}");
    }
}

#[test]
fn a_percent_encoded_scheme_is_a_missing_asset() {
    let assets = vec![Asset {
        name: "logo.png".to_owned(),
        data: vec![0; 8],
    }];
    for src in [
        "file%3A//x",
        "file%3a//etc/passwd",
        "HTTP://x",
        "Https://example.com/logo.png",
        "FILE:///logo.png",
        "%2Fetc%2Fpasswd",
        "logo%2Epng",
    ] {
        let result = render(&Document {
            html: format!("<img src=\"{src}\" alt=\"x\">"),
            assets: assets.clone(),
            ..Document::default()
        });
        match result {
            Err(Error::Invalid(message)) => assert!(message.contains(src), "{message}"),
            other => panic!("{src}: {:?}", other.map(|r| r.pages.len())),
        }
    }
}

#[test]
fn an_svg_with_entities_and_a_use_cycle_renders_or_throws() {
    let laughs = r#"<?xml version="1.0"?>
<!DOCTYPE svg [
  <!ENTITY a "lollollollollollollollollollol">
  <!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">
  <!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">
  <!ENTITY d "&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;">
  <!ENTITY e "&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;">
  <!ENTITY f "&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;">
  <!ENTITY g "&f;&f;&f;&f;&f;&f;&f;&f;&f;&f;">
  <!ENTITY h "&g;&g;&g;&g;&g;&g;&g;&g;&g;&g;">
]>
<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text>&h;</text></svg>"#;
    let external = r#"<?xml version="1.0"?>
<!DOCTYPE svg [ <!ENTITY secret SYSTEM "file:///etc/passwd"> ]>
<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text>&secret;</text></svg>"#;
    let cycle = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="10" height="10">
  <defs>
    <g id="a"><use xlink:href="#b"/><rect width="1" height="1"/></g>
    <g id="b"><use href="#a"/></g>
  </defs>
  <use href="#a"/><use href="#self" id="self"/>
</svg>"##;
    for (name, svg) in [
        ("laughs.svg", laughs),
        ("external.svg", external),
        ("cycle.svg", cycle),
    ] {
        let result = render(&Document {
            html: format!("<p>Shop</p><img src=\"{name}\" alt=\"x\">"),
            assets: vec![Asset {
                name: name.to_owned(),
                data: svg.as_bytes().to_vec(),
            }],
            ..Document::default()
        });
        match result {
            Ok(out) => assert!(!contains(&out.data, "root:"), "{name} read a file"),
            Err(Error::Parse(_)) => {}
            Err(other) => panic!("{name}: {other:?}"),
        }
    }
    // The same SVG inline in the HTML.
    let inline = cycle.replace("width=\"10\"", "width=\"10\" id=\"top\"");
    let result = render(&Document {
        html: format!("<p>Shop</p>{inline}"),
        ..Document::default()
    });
    match result {
        Ok(out) => assert_eq!(out.pages.len(), 1),
        Err(Error::Parse(_)) => {}
        Err(other) => panic!("inline: {other:?}"),
    }
}
