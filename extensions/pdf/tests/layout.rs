//! The block and inline layout on the host: the boxes the computed styles give, the text and its
//! font chain, and the elements the subset does not render
//! (`rule:core-classes/pdf-css-subset`, `rule:core-classes/pdf-default-fonts`).

use nvs_pdf::layout::{self, Item, Laid, MARGIN};
use nvs_pdf::{Asset, Document, assets, css, fonts, parse, render};

const SERIF: &[u8] = include_bytes!("../../fonts/LiberationSerif-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../image/fixtures/NotoSansArabic-Regular.ttf");

fn lay_with(html: &str, list: &[Asset]) -> Laid {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(list);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    layout::layout(&dom, &styles, &assets, fonts).unwrap()
}

fn lay(html: &str) -> Laid {
    lay_with(html, &[])
}

/// Each text item's face, pen start, baseline and right end, a line's last space not counted.
fn runs(laid: &Laid) -> Vec<(usize, f32, f32, f32)> {
    laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text {
                face,
                x,
                y,
                glyphs,
                text,
                ..
            } => {
                let inked = glyphs.len()
                    - glyphs
                        .iter()
                        .rev()
                        .take_while(|g| text[g.text.clone()].trim().is_empty())
                        .count();
                let end = x + glyphs[..inked].iter().map(|g| g.advance).sum::<f32>();
                Some((*face, *x, *y, end))
            }
            _ => None,
        })
        .collect()
}

/// Each line's left and right end, by baseline, top to bottom.
fn lines(laid: &Laid) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32, f32)> = Vec::new();
    for (_, x, y, end) in runs(laid) {
        match out.iter_mut().find(|(at, _, _)| (*at - y).abs() < 0.01) {
            Some((_, left, right)) => {
                *left = left.min(x);
                *right = right.max(end);
            }
            None => out.push((y, x, end)),
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.into_iter().map(|(_, l, r)| (l, r)).collect()
}

/// A rectangle's `x`, `y`, width, height and colour.
type Rect = (f32, f32, f32, f32, (u8, u8, u8));

fn rects(laid: &Laid) -> Vec<Rect> {
    laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Rect {
                x,
                y,
                width,
                height,
                color,
            } => Some((*x, *y, *width, *height, (color.r, color.g, color.b))),
            _ => None,
        })
        .collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

const WORDS: &str = "The quick brown fox jumps over the lazy dog, and the dog keeps on \
                     sleeping in the warm afternoon sun while the fox runs off.";

#[test]
fn text_wraps_and_justifies_inside_its_box() {
    let laid = lay(&format!(
        "<body style='margin: 0'><p style='width: 200pt; text-align: justify'>{WORDS}</p>"
    ));
    let lines = lines(&laid);
    assert!(lines.len() > 2, "{lines:?}");
    for (left, right) in &lines[..lines.len() - 1] {
        assert!(close(*left, MARGIN), "{lines:?}");
        assert!((right - (MARGIN + 200.0)).abs() < 0.5, "{lines:?}");
    }
    let (_, last) = lines[lines.len() - 1];
    assert!(last < MARGIN + 190.0, "{lines:?}");
}

#[test]
fn centred_and_right_aligned_lines_end_where_their_box_says() {
    let laid = lay("<body style='margin: 0'><p style='width: 300pt; text-align: right'>Total</p>");
    let (_, right) = lines(&laid)[0];
    assert!((right - (MARGIN + 300.0)).abs() < 0.5, "{right}");
    let laid = lay("<body style='margin: 0'><p style='width: 300pt; text-align: center'>Total</p>");
    let (left, right) = lines(&laid)[0];
    assert!(((left - MARGIN) - (MARGIN + 300.0 - right)).abs() < 0.5);
}

#[test]
fn cjk_breaks_between_ideographs() {
    let text = "一二三四五六七八九十".repeat(4);
    let laid = lay(&format!(
        "<body style='margin: 0'><p style='width: 60pt'>{text}</p>"
    ));
    let lines = lines(&laid);
    assert!(lines.len() > 3, "{lines:?}");
    for (_, right) in &lines {
        assert!(*right <= MARGIN + 60.5, "{lines:?}");
    }
}

/// Each text item's glyphs as the characters their clusters cover, left to right.
fn drawn(laid: &Laid) -> Vec<String> {
    laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { glyphs, text, .. } => {
                let mut clusters: Vec<_> = glyphs.iter().map(|g| g.text.clone()).collect();
                clusters.dedup();
                Some(clusters.into_iter().map(|r| &text[r]).collect())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn arabic_and_hebrew_lay_out_right_to_left() {
    // Hebrew is in Liberation: a right-to-left block starts at its right edge, and its first
    // letter is drawn rightmost.
    let laid = lay("<body style='margin: 0'><p dir='rtl' style='width: 300pt'>שלום</p>");
    assert!(laid.missing.is_empty(), "{:?}", laid.missing);
    let (left, right) = lines(&laid)[0];
    assert!((right - (MARGIN + 300.0)).abs() < 0.5, "{left} {right}");
    assert_eq!(drawn(&laid), ["םולש"]);

    // A Latin word in a right-to-left paragraph is its own left-to-right run. It comes first, so
    // it is drawn right of the Hebrew.
    let laid = lay("<body style='margin: 0'><p dir='rtl'>Shop שלום</p>");
    let texts = drawn(&laid);
    let runs = runs(&laid);
    let latin = texts.iter().position(|t| t.contains("Shop")).unwrap();
    let hebrew = texts.iter().position(|t| t.contains("םולש")).unwrap();
    assert!(runs[latin].1 > runs[hebrew].3, "{texts:?} {runs:?}");

    // An override draws Latin letters right to left, and an isolate keeps its own order.
    let laid = lay(
        "<p><bdo dir='rtl'>abc</bdo> <span style='unicode-bidi: isolate; direction: rtl'>xyz</span></p>",
    );
    let texts = drawn(&laid).join("");
    assert!(texts.starts_with("cba"), "{texts:?}");
    assert!(texts.ends_with("xyz"), "{texts:?}");

    // Arabic comes from an `@font-face`: its letters join, so a letter drawn inside a word is
    // another glyph from the same letter drawn alone.
    let arabic = [Asset {
        name: "arabic.ttf".to_owned(),
        data: ARABIC.to_vec(),
    }];
    let html = "<style>@font-face { font-family: Kufi; src: url(arabic.ttf) }\
                p { font-family: Kufi; direction: rtl; width: 300pt }</style>\
                <body style='margin: 0'><p>بحب</p><p>ح</p>";
    let laid = lay_with(html, &arabic);
    assert!(laid.missing.is_empty(), "{:?}", laid.missing);
    let glyphs: Vec<Vec<u32>> = laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { glyphs, .. } => Some(glyphs.iter().map(|g| g.id).collect()),
            _ => None,
        })
        .collect();
    assert_eq!(glyphs.len(), 2, "{glyphs:?}");
    assert!(!glyphs[0].contains(&glyphs[1][0]), "{glyphs:?}");
    assert_eq!(drawn(&laid)[0], "بحب");
    for (_, right) in lines(&laid) {
        assert!((right - (MARGIN + 300.0)).abs() < 0.5, "{right}");
    }
}

#[test]
fn a_missing_glyph_is_drawn_as_the_empty_box_and_reported_with_its_chain() {
    let laid = lay("<p style='font-family: Times, serif'>a 一二 b</p>");
    assert_eq!(laid.missing.len(), 1, "{:?}", laid.missing);
    assert_eq!(laid.missing[0].text, "一二");
    assert_eq!(laid.missing[0].fonts, ["Liberation Serif"]);
    assert_eq!(laid.missing[0].page, 1);
    let notdef = laid.pages[0].items.iter().any(|item| match item {
        Item::Text { glyphs, .. } => glyphs.iter().any(|g| g.id == 0),
        _ => false,
    });
    assert!(notdef);
    let clean = lay("<p>Nothing is missing here.</p>");
    assert!(clean.missing.is_empty());
}

#[test]
fn a_font_face_from_the_asset_map_is_used_and_subset() {
    let html = "<style>@font-face { font-family: Brand; src: url(brand.ttf) }\
                p { font-family: Brand, sans-serif }</style><p>Shop</p>";
    let brand = [Asset {
        name: "brand.ttf".to_owned(),
        data: SERIF.to_vec(),
    }];
    let laid = lay_with(html, &brand);
    assert_eq!(laid.faces.len(), 13);
    assert!(runs(&laid).iter().all(|(face, ..)| *face == 12));
    let rendered = render(&Document {
        html: html.to_owned(),
        assets: brand.to_vec(),
        ..Document::default()
    })
    .unwrap();
    assert!(rendered.missing_glyphs.is_empty());
    assert!(
        rendered.data.len() < SERIF.len() / 4,
        "{}",
        rendered.data.len()
    );
}

#[test]
fn a_font_face_that_is_not_a_font_throws_naming_it() {
    let html = "<style>@font-face { font-family: Brand; src: url(brand.ttf) }</style><p>x</p>";
    let err = render(&Document {
        html: html.to_owned(),
        assets: vec![Asset {
            name: "brand.ttf".to_owned(),
            data: b"not a font".to_vec(),
        }],
        ..Document::default()
    })
    .unwrap_err();
    assert!(format!("{err:?}").contains("brand.ttf"), "{err:?}");
}

#[test]
fn arial_helvetica_times_and_courier_map_to_liberation() {
    for (family, face) in [
        ("Arial", 0),
        ("Helvetica", 0),
        ("'Times New Roman'", 4),
        ("Times", 4),
        ("Courier", 8),
        ("Unknown, monospace", 8),
    ] {
        let laid = lay(&format!("<p style=\"font-family: {family}\">Blog</p>"));
        let faces: Vec<usize> = runs(&laid).iter().map(|r| r.0).collect();
        assert_eq!(faces, [face], "{family}");
    }
    let laid = lay("<p style='font-family: Courier; font-weight: bold; font-style: italic'>x</p>");
    assert_eq!(runs(&laid)[0].0, 11);
}

#[test]
fn block_boxes_take_margin_padding_border_and_width() {
    let laid = lay(
        "<body style='margin: 0'><div style='margin: 10pt; padding: 5pt; \
         border: 2pt solid #ff0000; width: 100pt; background-color: #00ff00'>Shop</div>",
    );
    let rects = rects(&laid);
    let background = rects
        .iter()
        .find(|r| r.4 == (0, 255, 0))
        .expect("a background");
    assert!(close(background.0, MARGIN + 10.0), "{background:?}");
    assert!(close(background.1, MARGIN + 10.0), "{background:?}");
    assert!(close(background.2, 114.0), "{background:?}");
    assert_eq!(rects.iter().filter(|r| r.4 == (255, 0, 0)).count(), 4);
    let (_, x, ..) = runs(&laid)[0];
    assert!(close(x, MARGIN + 17.0), "{x}");
}

#[test]
fn display_none_draws_nothing_and_a_user_agent_margin_applies() {
    let laid = lay("<p style='display: none'>hidden</p><div class='x'></div>");
    assert!(laid.pages[0].items.is_empty());
    let laid = lay("<body style='margin: 0'><p>one</p><p>two</p>");
    let runs = runs(&laid);
    // Two 12pt paragraphs with 1em between them.
    assert!(runs[1].2 - runs[0].2 > 12.0 + 11.0, "{runs:?}");
}

/// Each line's text items from left to right, joined by a space, top to bottom.
fn line_texts(laid: &Laid) -> Vec<String> {
    let mut items: Vec<(f32, f32, String)> = laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { x, y, text, .. } => Some((*y, *x, text.trim().to_owned())),
            _ => None,
        })
        .collect();
    items.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut out: Vec<(f32, Vec<String>)> = Vec::new();
    for (y, _, text) in items {
        match out.last_mut() {
            Some((at, texts)) if (*at - y).abs() < 0.01 => {
                if texts.last() != Some(&text) {
                    texts.push(text);
                }
            }
            _ => out.push((y, vec![text])),
        }
    }
    out.into_iter().map(|(_, t)| t.join(" ")).collect()
}

#[test]
fn lists_and_counters_number_their_items() {
    let laid = lay(
        "<ol start='3'><li>a</li><li>b</li></ol><ol style='list-style-type: upper-roman'><li>c</li><li value='9'>d</li></ol>",
    );
    assert_eq!(line_texts(&laid), ["3. a", "4. b", "I. c", "IX. d"]);
    // The marker is outside the content box: it ends where the item's text starts.
    let runs = runs(&laid);
    let (content, marker) = (runs[0], runs[1]);
    assert!(marker.3 <= content.1 + 0.01, "{runs:?}");
    assert!(close(marker.2, content.2), "{runs:?}");

    let laid = lay("<ul style='list-style-position: inside'><li>x</li></ul>");
    assert_eq!(line_texts(&laid), ["\u{2022} x"]);

    let laid = lay(
        "<style>body { counter-reset: section } h2 { counter-increment: section }\
         h2::before { content: counter(section, upper-alpha) '. ' }\
         h2::after { content: ' (' attr(data-id) ')' }</style>\
         <h2 data-id='x'>Shop</h2><h2 data-id='y'>Blog</h2>",
    );
    assert_eq!(line_texts(&laid), ["A. Shop (x)", "B. Blog (y)"]);

    let laid = lay(
        "<style>ol { counter-reset: item; list-style-type: none } li { counter-increment: item }\
         li::before { content: counters(item, '.') ' ' }</style>\
         <ol><li>a<ol><li>b</li><li>c</li></ol></li><li>d</li></ol>",
    );
    assert_eq!(line_texts(&laid), ["1 a", "1.1 b", "1.2 c", "2 d"]);
}

#[test]
fn iframe_video_canvas_and_form_fields_are_dropped_and_draw_nothing() {
    let html = "<p>Shop</p>\n<iframe src='https://example.com'></iframe>\n<video></video>\
                <canvas></canvas>\n<input value='x'>";
    let rendered = render(&Document {
        html: html.to_owned(),
        ..Document::default()
    })
    .unwrap();
    let dropped: Vec<(&str, u64)> = rendered
        .dropped
        .iter()
        .map(|d| (d.selector.as_str(), d.line))
        .collect();
    assert_eq!(
        dropped,
        [
            ("<iframe>", 2),
            ("<video>", 3),
            ("<canvas>", 3),
            ("<input>", 4)
        ]
    );
    assert!(rendered.dropped.iter().all(|d| d.property.is_empty()));
    let laid = lay("<p>Shop</p><canvas>fallback</canvas><button>Buy</button>");
    assert_eq!(runs(&laid).len(), 1);
}
