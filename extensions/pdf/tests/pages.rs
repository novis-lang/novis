//! The page-level rules: `counter-reset: page` restarts the numbers, an `@page` selector and an
//! `@media` feature the engine does not read are reported, a sheet that is not UTF-8 is
//! reported, a margin box taller than its margin is listed in the overflows, and the corner
//! boxes align toward the page (`rule:core-classes/pdf-css-subset`).

use nvs_pdf::layout::{self, Item, Laid};
use nvs_pdf::{Asset, Document, assets, css, fonts, parse, render};

/// The document `html` with `files` as its assets: its styles and its layout.
fn lay_with(html: &str, files: &[Asset]) -> (css::Styles, Laid, parse::Dom) {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(files);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    let laid = layout::layout(&dom, &styles, &assets, fonts).unwrap();
    (styles, laid, dom)
}

fn lay(html: &str) -> Laid {
    lay_with(html, &[]).1
}

/// Each dropped entry as its property, value and selector.
fn dropped(styles: &css::Styles) -> Vec<(String, String, String)> {
    styles
        .dropped
        .iter()
        .map(|d| (d.property.clone(), d.value.clone(), d.selector.clone()))
        .collect()
}

fn entry(property: &str, value: &str, selector: &str) -> (String, String, String) {
    (property.to_owned(), value.to_owned(), selector.to_owned())
}

/// The text runs on `page` whose baseline is between `top` and `bottom`, joined in order.
fn band(laid: &Laid, page: usize, top: f32, bottom: f32) -> String {
    let mut text = String::new();
    for item in &laid.pages[page].items {
        if let Item::Text { text: run, y, .. } = item
            && *y > top
            && *y < bottom
        {
            text.push_str(run);
        }
    }
    text.trim().to_owned()
}

/// The left and right end of the first run on page 0 whose text is `text`.
fn ends(laid: &Laid, text: &str) -> (f32, f32) {
    laid.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Item::Text {
                text: run,
                x,
                glyphs,
                ..
            } if run.trim() == text => {
                Some((*x, x + glyphs.iter().map(|g| g.advance).sum::<f32>()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no run {text}"))
}

/// The element whose `class` is `class`.
fn by_class(dom: &parse::Dom, class: &str) -> usize {
    dom.nodes
        .iter()
        .position(|node| node.attr("class") == Some(class))
        .unwrap()
}

#[test]
fn counter_reset_page_restarts_the_page_number() {
    let laid = lay("<style>
        @page {
            size: 400pt 300pt;
            margin: 50pt;
            @bottom-center { content: counter(page) ' / ' counter(pages) }
        }
        body, p { margin: 0 }
        p + p { break-before: page }
        .main { counter-reset: page }
        .appendix { counter-reset: page 10 }
        </style>
        <p>Cover</p><p>Contents</p><p class=main>Lorem</p><p>Ipsum</p>
        <p class=appendix>Shop</p>");
    let footers: Vec<String> = (0..laid.pages.len())
        .map(|page| band(&laid, page, 250.0, 300.0))
        .collect();
    // `counter(pages)` stays the number of pages in the file.
    assert_eq!(footers, ["1 / 5", "2 / 5", "1 / 5", "2 / 5", "10 / 5"]);
}

#[test]
fn an_unknown_page_pseudo_class_is_reported_as_dropped() {
    let (styles, laid, _) = lay_with(
        "<style>
        @page { size: 400pt 300pt; margin: 50pt }
        @page :nth(2) { margin: 0 }
        @page :first:hover { margin: 0 }
        @page :first { margin-top: 20pt }
        </style><p>Lorem</p>",
        &[],
    );
    assert_eq!(
        dropped(&styles),
        [
            entry("", "", "@page :nth(2)"),
            entry("", "", "@page :first:hover")
        ]
    );
    assert_eq!(styles.pages.len(), 2);
    // The rules that read still apply.
    assert_eq!(laid.pages[0].geometry.margin, [20.0, 50.0, 50.0, 50.0]);
}

#[test]
fn a_media_query_is_applied_by_type_and_page_size_and_an_unknown_feature_is_reported() {
    let html = "<style>
        @page { size: A5 landscape }
        @media print { .print { padding-left: 1pt } }
        @media screen { .screen { padding-left: 1pt } }
        @media not screen { .not-screen { padding-left: 1pt } }
        @media print and (orientation: landscape) { .landscape { padding-left: 1pt } }
        @media (orientation: portrait) { .portrait { padding-left: 1pt } }
        @media (min-width: 200mm) and (max-height: 150mm) { .wide { padding-left: 1pt } }
        @media (max-width: 150mm) { .narrow { padding-left: 1pt } }
        @media screen, (width: 210mm) { .exact { padding-left: 1pt } }
        @media (color) { .color { padding-left: 1pt } }
        @media print and (hover: hover) { .hover { padding-left: 1pt } }
        </style>
        <style media=\"screen\">.attribute { padding-left: 1pt }</style>
        <p class=print>a</p><p class=screen>b</p><p class=not-screen>c</p>
        <p class=landscape>d</p><p class=portrait>e</p><p class=wide>f</p>
        <p class=narrow>g</p><p class=exact>h</p><p class=color>i</p><p class=hover>j</p>
        <p class=attribute>k</p>";
    let (styles, laid, dom) = lay_with(html, &[]);
    // A5 landscape is 210 mm wide and 148 mm high, and the queries test that size.
    assert!((laid.pages[0].geometry.width - 210.0 * 72.0 / 25.4).abs() < 0.01);
    let applied = |class: &str| {
        let style = styles.of(by_class(&dom, class)).unwrap();
        style.get("padding-left").is_some()
    };
    for class in ["print", "not-screen", "landscape", "wide", "exact"] {
        assert!(applied(class), "{class} is not applied");
    }
    for class in [
        "screen",
        "portrait",
        "narrow",
        "color",
        "hover",
        "attribute",
    ] {
        assert!(!applied(class), "{class} is applied");
    }
    assert_eq!(
        dropped(&styles),
        [
            entry("@media", "(color)", ""),
            entry("@media", "(hover: hover)", "")
        ]
    );
}

#[test]
fn a_stylesheet_that_is_not_utf8_is_reported_as_dropped() {
    let files = [
        Asset {
            name: "latin.css".to_owned(),
            data: b"/* caf\xe9 */ .latin { padding-left: 1pt }".to_vec(),
        },
        Asset {
            name: "utf8.css".to_owned(),
            data: "\u{feff}.utf8 { padding-left: 1pt }".as_bytes().to_vec(),
        },
    ];
    let (styles, _, dom) = lay_with(
        "<link rel=stylesheet href=latin.css><link rel=stylesheet href=utf8.css>
        <p class=latin>Lorem</p><p class=utf8>Ipsum</p>",
        &files,
    );
    assert_eq!(dropped(&styles), [entry("", "latin.css", "<link>")]);
    let applied = |class: &str| {
        let style = styles.of(by_class(&dom, class)).unwrap();
        style.get("padding-left").is_some()
    };
    assert!(!applied("latin"));
    // A sheet with a byte order mark is UTF-8, and its first rule applies.
    assert!(applied("utf8"));
}

#[test]
fn a_margin_box_taller_than_its_margin_is_listed_in_overflows() {
    let rendered = render(&Document {
        html: "<style>
            @page {
                size: 400pt 300pt;
                margin: 20pt 50pt;
                @top-center { content: 'Shop'; font-size: 40pt }
                @top-right { content: 'fits'; font-size: 8pt }
                @bottom-center { content: element(foot) }
            }
            .foot { position: running(foot); height: 60pt }
            </style>
            <div class=foot>Lorem</div><p>Ipsum</p>"
            .to_owned(),
        ..Document::default()
    })
    .unwrap();
    let overflows: Vec<(u64, &str, &str)> = rendered
        .overflows
        .iter()
        .map(|o| (o.page, o.area.as_str(), o.text.as_str()))
        .collect();
    assert_eq!(
        overflows,
        [
            (1, "@top-center", "Shop"),
            (1, "@bottom-center", "element(foot)")
        ]
    );
}

#[test]
fn corner_margin_boxes_align_toward_the_page() {
    let laid = lay("<style>
        @page {
            size: 400pt 300pt;
            margin: 60pt;
            @top-left-corner { content: 'A' }
            @top-right-corner { content: 'B' }
            @bottom-left-corner { content: 'C' }
            @bottom-right-corner { content: 'D' }
        }
        </style><p>Lorem</p>");
    // A left corner box is 60 pt wide and its text ends at its right edge.
    for left in ["A", "C"] {
        let (_, right) = ends(&laid, left);
        assert!((right - 60.0).abs() < 0.5, "{left} ends at {right}");
    }
    // A right corner box starts at 340 pt and its text starts at its left edge.
    for right in ["B", "D"] {
        let (start, _) = ends(&laid, right);
        assert!((start - 340.0).abs() < 0.5, "{right} starts at {start}");
    }
}
