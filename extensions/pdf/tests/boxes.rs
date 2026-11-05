//! Every box and text property the subset accepts is applied: `overflow` cuts content off,
//! `visibility` hides it, margins collapse with negative ones, a span's `white-space` and
//! `line-height` and a block's `text-align-last` are used, and `<ol type>` and `reversed` set
//! the markers (`rule:core-classes/pdf-css-subset`).

use nvs_pdf::layout::{self, Item, Laid, Paint, bounds};
use nvs_pdf::{Asset, assets, css, fonts, parse};

/// The layout of `html` with a 2 by 2 blue PNG named `tile.png`, and the declarations the
/// cascade dropped, as `property: value`.
fn lay_with(html: &str) -> (Laid, Vec<String>) {
    let dom = parse::parse(html);
    let mut data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut data, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[0, 0, 255, 255].repeat(4))
            .unwrap();
    }
    let tile = [Asset {
        name: "tile.png".to_owned(),
        data,
    }];
    let assets = assets::Assets::new(&tile);
    let styles = css::cascade(&dom, html, &assets);
    let dropped = styles
        .dropped
        .iter()
        .map(|d| format!("{}: {}", d.property, d.value))
        .collect();
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    (
        layout::layout(&dom, &styles, &assets, fonts).unwrap(),
        dropped,
    )
}

fn lay(html: &str) -> Laid {
    lay_with(html).0
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

const PAGE: &str = "<style>
    @page { size: 105mm 148mm; margin: 10mm }
    body, p, div { margin: 0; font-size: 10pt; line-height: 12pt }
    </style>";

/// Each text run on `page`, top to bottom: its line's text, its pen start and its baseline.
fn texts(laid: &Laid, page: usize) -> Vec<(String, f32, f32)> {
    let mut out: Vec<(String, f32, f32)> = laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, x, y, .. } => Some((text.trim().to_owned(), *x, *y)),
            _ => None,
        })
        .filter(|(text, ..)| !text.is_empty())
        .collect();
    out.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.1.total_cmp(&b.1)));
    out
}

/// The pen start and the baseline of the first run on page 0 whose line's text is `text`.
fn at(laid: &Laid, text: &str) -> (f32, f32) {
    let runs = texts(laid, 0);
    let run = runs.iter().find(|(t, ..)| t == text);
    let (_, x, y) = run.unwrap_or_else(|| panic!("no {text}: {runs:?}"));
    (*x, *y)
}

/// The bounds of each filled shape on `page` in the colour `rgb`.
fn fills(laid: &Laid, page: usize, rgb: (u8, u8, u8)) -> Vec<(f32, f32, f32, f32)> {
    laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Path {
                path,
                paint: Paint::Color(c),
                stroke: None,
            } if (c.r, c.g, c.b) == rgb => Some(bounds(path)),
            _ => None,
        })
        .collect()
}

fn lorem(count: usize) -> String {
    (1..=count).map(|n| format!("<p>Lorem {n}</p>")).collect()
}

#[test]
fn overflow_hidden_clips_content_and_adds_no_page_for_it() {
    // The same content in a box that does not cut it off runs over several pages.
    let open = lay(&format!(
        "{PAGE}<div style='height: 50pt'>{}</div><p>Shop</p>",
        lorem(80)
    ));
    assert!(open.pages.len() > 2, "{}", open.pages.len());

    for overflow in [
        "overflow: hidden",
        "overflow-y: hidden",
        "overflow-x: hidden",
    ] {
        let laid = lay(&format!(
            "{PAGE}<p>Blog</p><div style='height: 50pt; {overflow}'>{}</div><p>Shop</p>",
            lorem(80)
        ));
        assert_eq!(laid.pages.len(), 1, "{overflow}");
        // The content is drawn inside a group cut off at the 50 pt padding box.
        let clip = laid.pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                Item::Begin {
                    clip: Some(path), ..
                } => Some(bounds(path)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no clip for {overflow}"));
        let (_, top, _, bottom) = clip;
        assert!(close(bottom - top, 50.0), "{overflow}: {clip:?}");
        // `Shop` is on the line under the box.
        let (_, blog) = at(&laid, "Blog");
        let (_, shop) = at(&laid, "Shop");
        assert!(close(shop - blog, 62.0), "{overflow}: {blog} {shop}");
    }
}

#[test]
fn visibility_hidden_keeps_the_space_and_draws_nothing() {
    let doc = |style: &str| {
        lay(&format!(
            "{PAGE}<p>Shop</p><p style='background: red; text-decoration: underline; {style}'>
            Hidden</p><p>Blog</p>"
        ))
    };
    let shown = doc("");
    let hidden = doc("visibility: hidden");
    assert!(texts(&hidden, 0).iter().all(|(t, ..)| t != "Hidden"));
    assert!(fills(&hidden, 0, (255, 0, 0)).is_empty());
    assert!(!fills(&shown, 0, (255, 0, 0)).is_empty());
    assert!(close(at(&shown, "Blog").1, at(&hidden, "Blog").1));

    // A hidden image keeps its space and is not drawn.
    let picture = |style: &str| {
        lay(&format!(
            "{PAGE}<img src='tile.png' style='display: block; height: 30pt; {style}'><p>Blog</p>"
        ))
    };
    let image = |laid: &Laid| {
        laid.pages[0]
            .items
            .iter()
            .any(|item| matches!(item, Item::Image { .. }))
    };
    let (shown, hidden) = (picture(""), picture("visibility: hidden"));
    assert!(image(&shown) && !image(&hidden));
    assert!(close(at(&shown, "Blog").1, at(&hidden, "Blog").1));

    // A table row with `visibility: collapse` takes no space.
    let table = |middle: &str| {
        lay(&format!(
            "{PAGE}<table style='border-spacing: 0'><tr><td>One</td></tr>{middle}
            <tr><td>Three</td></tr></table>"
        ))
    };
    let without = table("");
    let collapsed = table("<tr style='visibility: collapse'><td>Two</td></tr>");
    assert!(texts(&collapsed, 0).iter().all(|(t, ..)| t != "Two"));
    assert!(close(at(&without, "Three").1, at(&collapsed, "Three").1));
}

#[test]
fn a_negative_margin_pulls_the_box_up() {
    let doc = |margin: &str| {
        lay(&format!(
            "{PAGE}<p>Shop</p><p style='margin-top: {margin}'>Blog</p>"
        ))
    };
    let (_, flat) = at(&doc("0"), "Blog");
    let (_, pulled) = at(&doc("-10pt"), "Blog");
    assert!(close(flat - pulled, 10.0), "{flat} {pulled}");
}

#[test]
fn collapsing_margins_add_the_largest_positive_and_the_most_negative() {
    let doc = |after: &str, before: &str, inner: &str| {
        lay(&format!(
            "{PAGE}<p style='margin-bottom: {after}'>Shop</p>
            <div style='margin-top: {before}'><p style='margin-top: {inner}'>Blog</p></div>"
        ))
    };
    let (_, flat) = at(&doc("0", "0", "0"), "Blog");
    // 20 and 10 collapse to 20, and -8 is the most negative: 20 - 8.
    let (_, mixed) = at(&doc("20pt", "10pt", "-8pt"), "Blog");
    assert!(close(mixed - flat, 12.0), "{flat} {mixed}");
    // Negative margins alone collapse to the most negative one.
    let (_, negative) = at(&doc("-3pt", "-6pt", "-2pt"), "Blog");
    assert!(close(flat - negative, 6.0), "{flat} {negative}");
}

#[test]
fn white_space_nowrap_on_a_span_keeps_its_words_on_one_line() {
    let doc = |style: &str| {
        lay(&format!(
            "{PAGE}<p style='width: 80pt'>Shop <span style='{style}'>dolor sit amet
            consectetur adipiscing</span> elit</p>"
        ))
    };
    let words = "dolor sit amet consectetur adipiscing";
    let has = |laid: &Laid| texts(laid, 0).iter().any(|(t, ..)| t.contains(words));
    assert!(!has(&doc("")));
    assert!(has(&doc("white-space: nowrap")));
}

#[test]
fn line_height_on_a_span_grows_its_line() {
    let doc = |style: &str| {
        lay(&format!(
            "{PAGE}<p>Shop <span style='{style}'>Blog</span></p><p>Cart</p>"
        ))
    };
    let (_, normal) = at(&doc(""), "Cart");
    let (_, tall) = at(&doc("line-height: 40pt"), "Cart");
    // The line is 40 pt tall, not 12 pt.
    assert!(close(tall - normal, 28.0), "{normal} {tall}");
}

#[test]
fn text_align_last_aligns_the_last_line_or_is_reported() {
    let doc = |style: &str| {
        lay_with(&format!(
            "{PAGE}<p style='width: 120pt; {style}'>Lorem ipsum dolor sit amet Shop</p>"
        ))
    };
    let last = |laid: &Laid| {
        let runs = texts(laid, 0);
        let (_, x, _) = runs.last().expect("a line").clone();
        x
    };
    let (start, _) = doc("");
    let (right, dropped) = doc("text-align-last: right");
    assert!(dropped.is_empty(), "{dropped:?}");
    assert!(last(&right) > last(&start) + 20.0);
    // `justify` with `text-align-last: center` centres the last line.
    let (centre, _) = doc("text-align: justify; text-align-last: center");
    assert!(last(&centre) > last(&start) + 10.0 && last(&centre) < last(&right) - 10.0);
}

#[test]
fn ol_type_and_reversed_set_the_markers() {
    let markers = |html: &str| -> Vec<String> {
        texts(&lay(&format!("{PAGE}{html}")), 0)
            .into_iter()
            .map(|(t, ..)| t)
            .filter(|t| t.ends_with('.'))
            .collect()
    };
    assert_eq!(markers("<ol type='A'><li>Shop<li>Blog</ol>"), ["A.", "B."]);
    assert_eq!(markers("<ol type='i'><li>Shop<li>Blog</ol>"), ["i.", "ii."]);
    assert_eq!(
        markers("<ol reversed><li>Shop<li>Blog<li>Cart</ol>"),
        ["3.", "2.", "1."]
    );
    assert_eq!(
        markers("<ol reversed start='10'><li>Shop<li>Blog</ol>"),
        ["10.", "9."]
    );
    // A rule in the document's sheet wins over the attribute.
    assert_eq!(
        markers("<style>ol { list-style-type: lower-alpha }</style><ol type='I'><li>Shop</ol>"),
        ["a."]
    );
}
