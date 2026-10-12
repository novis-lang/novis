//! The block and inline layout on the host: the boxes the computed styles give, the text and its
//! font chain, and the elements the subset does not render
//! (`rule:core-classes/pdf-css-subset`, `rule:core-classes/pdf-default-fonts`).

use nvs_pdf::layout::{self, Item, Laid, MARGIN, Paint, Seg, Stroke};
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

/// Each filled path's bounds and its colour, when it is filled with one.
fn rects(laid: &Laid) -> Vec<Rect> {
    paths(laid)
        .into_iter()
        .filter_map(|(bounds, paint, stroke)| match (paint, stroke) {
            (Paint::Color(c), None) => {
                Some((bounds.0, bounds.1, bounds.2, bounds.3, (c.r, c.g, c.b)))
            }
            _ => None,
        })
        .collect()
}

/// A path's bounds as `x`, `y`, width and height, its paint and its stroke.
type Drawn = ((f32, f32, f32, f32), Paint, Option<Stroke>);

fn paths(laid: &Laid) -> Vec<Drawn> {
    laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Path {
                path,
                paint,
                stroke,
            } => {
                let (left, top, right, bottom) = layout::bounds(path);
                Some((
                    (left, top, right - left, bottom - top),
                    paint.clone(),
                    stroke.clone(),
                ))
            }
            _ => None,
        })
        .collect()
}

/// The segments of each path, by kind: moves and cubic curves.
fn contours_and_curves(laid: &Laid) -> Vec<(usize, usize)> {
    laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Path { path, .. } => Some((
                path.iter().filter(|s| matches!(s, Seg::Move(..))).count(),
                path.iter().filter(|s| matches!(s, Seg::Cubic(..))).count(),
            )),
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

/// Each line that ends in a drawn hyphen: its own text and the hyphen's right end.
fn hyphenated_lines(laid: &Laid) -> Vec<(String, f32)> {
    let items = &laid.pages[0].items;
    items
        .iter()
        .filter_map(|item| match item {
            Item::Text {
                x, y, glyphs, text, ..
            } if text == "-" => {
                let line = items.iter().find_map(|other| match other {
                    Item::Text { y: at, text, .. } if text != "-" && (at - y).abs() < 0.01 => {
                        Some(text.clone())
                    }
                    _ => None,
                })?;
                Some((line, x + glyphs[0].advance))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn hyphens_auto_breaks_by_the_language() {
    let words = "internationalization responsibilities characteristically";
    let para = |lang: &str, hyphens: &str| {
        format!(
            "<html lang='{lang}'><body style='margin: 0'>\
             <p style='width: 80pt; hyphens: {hyphens}; text-align: justify'>{words}</p>"
        )
    };
    let laid = lay(&para("en-GB", "auto"));
    let hyphenated = hyphenated_lines(&laid);
    assert!(hyphenated.len() >= 2, "{hyphenated:?}");
    for (text, right) in &hyphenated {
        // The line ends at a break point of the English patterns, and the hyphen is in the box.
        assert!(text.ends_with('\u{ad}'), "{text:?}");
        assert!(*right <= MARGIN + 80.0 + 0.01, "{hyphenated:?}");
    }
    for (_, right) in lines(&laid) {
        assert!(right <= MARGIN + 80.0 + 0.01, "{right}");
    }

    // German breaks a German compound by its own patterns.
    let laid = lay("<html lang='de'><body style='margin: 0'>\
         <p style='width: 60pt; hyphens: auto'>Donaudampfschifffahrtsgesellschaft</p>");
    assert!(!hyphenated_lines(&laid).is_empty());

    // No hyphen is drawn under `hyphens: manual`, or for a language with no patterns.
    assert!(hyphenated_lines(&lay(&para("en", "manual"))).is_empty());
    assert!(hyphenated_lines(&lay(&para("ja", "auto"))).is_empty());
    // An author's soft hyphen breaks under `manual` too.
    let laid = lay(
        "<body style='margin: 0'><p style='width: 60pt'>Shop\u{ad}keeper\u{ad}assistant\u{ad}ship</p>",
    );
    assert!(!hyphenated_lines(&laid).is_empty());
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

/// A 2 by 2 PNG, opaque blue.
fn png() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[0, 0, 255, 255].repeat(4))
            .unwrap();
    }
    out
}

#[test]
fn borders_radius_backgrounds_and_gradients_lay_out() {
    let boxed = |style: &str| {
        format!(
            "<body style='margin: 0'><div style='width: 100pt; height: 50pt; {style}'>Shop</div>"
        )
    };
    let lay_box = |style: &str| lay(&boxed(style));

    // Sides that differ are drawn one by one: a dashed side is a stroke along its middle.
    let laid = lay_box("border: 2pt dashed #ff0000; border-left-style: solid");
    let strokes: Vec<Stroke> = paths(&laid).into_iter().filter_map(|p| p.2).collect();
    assert_eq!(strokes.len(), 3, "{strokes:?}");
    assert!(
        strokes
            .iter()
            .all(|s| s.dash == [4.0, 4.0] && !s.round && s.width == 2.0)
    );
    assert_eq!(rects(&laid).len(), 1);

    // Four dotted sides alike are one stroked rectangle along the border's middle.
    let laid = lay_box("border: 2pt dotted #ff0000");
    let all = paths(&laid);
    assert_eq!(all.len(), 1, "{all:?}");
    let ((x, y, w, h), _, stroke) = &all[0];
    let stroke = stroke.as_ref().unwrap();
    assert!(stroke.round && stroke.dash == [0.0, 4.0], "{stroke:?}");
    assert!(
        close(*x, MARGIN + 1.0) && close(*y, MARGIN + 1.0),
        "{all:?}"
    );
    assert!(close(*w, 102.0) && close(*h, 52.0), "{all:?}");

    // A double border is two rings, each a third of its width.
    let laid = lay_box("border: 3pt double #ff0000");
    let rings = contours_and_curves(&laid);
    assert_eq!(rings, [(2, 0), (2, 0)], "{rings:?}");
    let both = rects(&laid);
    assert!(
        close(both[0].2, 106.0) && close(both[1].2, 102.0),
        "{both:?}"
    );

    // A radius rounds the background, and a percentage is of the width across and the height
    // down.
    let laid = lay_box("border-radius: 10pt; background-color: #00ff00");
    assert_eq!(contours_and_curves(&laid), [(1, 4)]);
    let background = rects(&laid)[0];
    assert!(
        close(background.0, MARGIN) && close(background.2, 100.0),
        "{background:?}"
    );
    let laid = lay_box("border-radius: 50%; background-color: #00ff00");
    let Item::Path { path, .. } = &laid.pages[0].items[0] else {
        panic!("{:?}", laid.pages[0].items[0]);
    };
    assert!(matches!(path[0], Seg::Move(x, y) if close(x, MARGIN + 50.0) && close(y, MARGIN)));
    assert!(
        path.iter()
            .any(|s| matches!(s, Seg::Cubic(.., x, y) if close(*x, MARGIN + 100.0) && close(*y, MARGIN + 25.0)))
    );
    // A rounded solid border is one ring that follows the corners.
    let laid = lay_box("border: 2pt solid #ff0000; border-radius: 8pt");
    assert_eq!(contours_and_curves(&laid), [(2, 8)]);

    // A linear gradient runs along its direction over the padding box.
    let laid = lay_box("background-image: linear-gradient(to right, red, blue)");
    let all = paths(&laid);
    let Paint::Linear {
        x1,
        y1,
        x2,
        y2,
        stops,
    } = &all[0].1
    else {
        panic!("{all:?}");
    };
    assert!(close(*x1, MARGIN) && close(*x2, MARGIN + 100.0) && close(*y1, *y2));
    assert_eq!(stops.len(), 2);
    assert_eq!((stops[0].offset, stops[0].color.r), (0.0, 255));
    assert_eq!((stops[1].offset, stops[1].color.b), (1.0, 255));
    let laid = lay_box("background: linear-gradient(red, lime 20%, blue)");
    let all = paths(&laid);
    let Paint::Linear {
        x1, y1, y2, stops, ..
    } = &all[0].1
    else {
        panic!("{all:?}");
    };
    assert!(close(*y1, MARGIN) && close(*y2, MARGIN + 50.0) && close(*x1, MARGIN + 50.0));
    let offsets: Vec<f32> = stops.iter().map(|s| s.offset).collect();
    assert!(close(offsets[1], 0.2) && offsets[2] == 1.0, "{offsets:?}");

    // A radial gradient is centred, and `circle` reaches the farthest corner.
    let laid = lay_box("background-image: radial-gradient(circle, white, black)");
    let all = paths(&laid);
    let Paint::Radial {
        cx, cy, r, scale_y, ..
    } = &all[0].1
    else {
        panic!("{all:?}");
    };
    assert!(
        close(*cx, MARGIN + 50.0) && close(*cy, MARGIN + 25.0),
        "{all:?}"
    );
    assert!(
        close(*r, 50.0_f32.hypot(25.0)) && *scale_y == 1.0,
        "{all:?}"
    );

    // An outline is drawn outside the border box, `outline-offset` away from it.
    let laid = lay_box("outline: 1pt solid #0000ff; outline-offset: 2pt");
    let top = rects(&laid)
        .into_iter()
        .find(|r| r.4 == (0, 0, 255) && r.3 < 2.0)
        .expect("a top edge");
    assert!(
        close(top.0, MARGIN - 3.0) && close(top.1, MARGIN - 3.0),
        "{top:?}"
    );
    assert!(close(top.2, 106.0), "{top:?}");

    // `opacity` puts the box and its text in one group.
    let laid = lay_box("opacity: 0.5; background-color: #00ff00");
    let items = &laid.pages[0].items;
    assert!(matches!(items[0], Item::Begin { opacity, clip: None } if opacity == 0.5));
    assert!(matches!(items[items.len() - 1], Item::End));
    assert!(items.iter().any(|i| matches!(i, Item::Text { .. })));

    // An image `url()` is laid in tiles from the padding box, or once under `no-repeat`.
    let assets = [Asset {
        name: "tile.png".to_owned(),
        data: png(),
    }];
    let laid = lay_with(
        &boxed("border: 4pt solid #ff0000; background-image: url(tile.png)"),
        &assets,
    );
    let all = paths(&laid);
    assert!(
        matches!(all[0].1, Paint::Tiles { x, y, width, .. } if close(x, MARGIN + 4.0) && close(y, MARGIN + 4.0) && close(width, 1.5)),
        "{all:?}"
    );
    let laid = lay_with(&boxed("background: url(tile.png) no-repeat"), &assets);
    let items = &laid.pages[0].items;
    assert!(
        matches!(items[0], Item::Begin { clip: Some(_), .. }),
        "{items:?}"
    );
    assert!(matches!(items[1], Item::Image { width, .. } if close(width, 1.5)));
    assert!(matches!(items[2], Item::End));

    // All of them render, and twice to the same bytes.
    let html: String = [
        "border: 2pt dashed red; border-left-style: dotted",
        "border: 3pt double red; border-radius: 6pt",
        "border-radius: 50%; background: radial-gradient(white, black)",
        "background-image: linear-gradient(45deg, red, blue 80%)",
        "outline: 1pt dotted blue; opacity: 0.4",
        "background: url(tile.png) repeat-x",
        "background-image: url(tile.png)",
    ]
    .iter()
    .map(|style| boxed(style).replace("<body style='margin: 0'>", ""))
    .collect();
    let document = Document {
        html,
        assets: assets.to_vec(),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert!(first.data.starts_with(b"%PDF"));
    assert_eq!(first.data, render(&document).unwrap().data);
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

/// Each image item's width and height, and whether its picture is an SVG.
fn pictures(laid: &Laid) -> Vec<(f32, f32, bool)> {
    laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Image {
                image,
                width,
                height,
                ..
            } => Some((
                *width,
                *height,
                matches!(laid.images[*image].0, layout::Picture::Vector(_)),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn an_inline_svg_is_vector_content_in_the_output() {
    const LOGO: &str = "<svg width='40' height='20' viewBox='0 0 4 2'>\
                        <rect width='4' height='2' fill='#ff0000'/>\
                        <circle cx='1' cy='1' r='1' style='fill: blue'/></svg>";

    // An inline `<svg>` is one picture at its own size, a pixel being three quarters of a point.
    // A CSS `width` replaces the `width` attribute, and the `height` attribute still applies.
    let laid = lay(&format!("<body style='margin: 0'><p>Shop</p>{LOGO}"));
    assert_eq!(pictures(&laid), [(30.0, 15.0, true)]);
    let laid = lay(&format!(
        "<body>{}</body>",
        LOGO.replace("<svg ", "<svg style='width: 60pt' ")
    ));
    assert_eq!(pictures(&laid), [(60.0, 15.0, true)]);
    let laid = lay(&format!(
        "<body>{}</body>",
        LOGO.replace("<svg ", "<svg style='display: none' ")
    ));
    assert!(pictures(&laid).is_empty());

    // An SVG `<img>` is a picture too, and an `<image>` inside it loads from the asset map.
    let file = "<svg xmlns='http://www.w3.org/2000/svg' width='8' height='8'>\
                <image href='tile.png' width='8' height='8'/></svg>";
    let assets = [
        Asset {
            name: "logo.svg".to_owned(),
            data: file.as_bytes().to_vec(),
        },
        Asset {
            name: "tile.png".to_owned(),
            data: png(),
        },
    ];
    let laid = lay_with("<img src='logo.svg'>", &assets);
    assert_eq!(pictures(&laid), [(6.0, 6.0, true)]);
    let err = render(&Document {
        html: "<img src='logo.svg'>".to_owned(),
        assets: assets[..1].to_vec(),
        ..Document::default()
    })
    .unwrap_err();
    assert!(
        matches!(&err, nvs_pdf::Error::Invalid(why) if why.contains("tile.png")),
        "{err:?}"
    );

    // The output draws paths and holds no raster image, and two renders are byte-equal.
    let document = Document {
        html: format!("<p>Shop</p>{LOGO}"),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    let raster = render(&Document {
        html: "<img src='tile.png'>".to_owned(),
        assets: assets[1..].to_vec(),
        ..Document::default()
    })
    .unwrap();
    let has = |data: &[u8], what: &[u8]| data.windows(what.len()).any(|w| w == what);
    assert!(has(&raster.data, b"/Subtype/Image"));
    assert!(!has(&first.data, b"/Subtype/Image"));
    assert!(
        first.data.len()
            > render(&Document {
                html: "<p>Shop</p>".to_owned(),
                ..Document::default()
            })
            .unwrap()
            .data
            .len()
    );
    assert_eq!(first.data, render(&document).unwrap().data);
}

#[test]
fn object_fit_and_object_position_place_the_picture_in_its_box() {
    // The PNG is 1.5pt square, and each `<img>` box is 30pt by 15pt at the page margin.
    let assets = [Asset {
        name: "tile.png".to_owned(),
        data: png(),
    }];
    // Each image's left and top from the box's, its width and height, and whether it is clipped.
    let placed = |css: &str| {
        let laid = lay_with(
            &format!(
                "<body style='margin: 0'><img src='tile.png' \
                 style='width: 30pt; height: 15pt; {css}'></body>"
            ),
            &assets,
        );
        let items = &laid.pages[0].items;
        let clipped = matches!(items.first(), Some(Item::Begin { clip: Some(_), .. }));
        let image = items.iter().find_map(|item| match item {
            Item::Image {
                x,
                y,
                width,
                height,
                ..
            } => Some([*x - MARGIN, *y - MARGIN, *width, *height]),
            _ => None,
        });
        (image.unwrap(), clipped)
    };
    let near = |(got, clipped): ([f32; 4], bool), want: [f32; 4], clip: bool| {
        assert!(
            got.iter().zip(want).all(|(g, w)| close(*g, w)) && clipped == clip,
            "{got:?} {clipped} against {want:?} {clip}"
        );
    };
    near(placed(""), [0.0, 0.0, 30.0, 15.0], false);
    near(placed("object-fit: fill"), [0.0, 0.0, 30.0, 15.0], false);
    near(placed("object-fit: contain"), [7.5, 0.0, 15.0, 15.0], false);
    near(placed("object-fit: cover"), [0.0, -7.5, 30.0, 30.0], true);
    near(placed("object-fit: none"), [14.25, 6.75, 1.5, 1.5], false);
    near(
        placed("object-fit: scale-down"),
        [14.25, 6.75, 1.5, 1.5],
        false,
    );
    near(
        placed("object-fit: contain; object-position: right"),
        [15.0, 0.0, 15.0, 15.0],
        false,
    );
    near(
        placed("object-fit: contain; object-position: bottom right"),
        [15.0, 0.0, 15.0, 15.0],
        false,
    );
    near(
        placed("object-fit: cover; object-position: 0 100%"),
        [0.0, -15.0, 30.0, 30.0],
        true,
    );
    near(
        placed("object-fit: none; object-position: 10pt 50%"),
        [10.0, 6.75, 1.5, 1.5],
        false,
    );

    // A clipped picture renders, and the same input gives the same bytes.
    let document = Document {
        html: "<img src='tile.png' style='width: 30pt; height: 15pt; object-fit: cover'>"
            .to_owned(),
        assets: assets.to_vec(),
        ..Document::default()
    };
    assert_eq!(
        render(&document).unwrap().data,
        render(&document).unwrap().data
    );
}

#[test]
fn text_indent_moves_the_first_line_of_each_block() {
    let body = |css: &str, html: &str| {
        lines(&lay(&format!(
            "<body style='margin: 0'><div style='width: 200pt; {css}'>{html}</div>"
        )))
    };
    // The first line starts 20pt in and is 20pt narrower, and the lines after it are not moved.
    let indented = body("text-indent: 20pt", WORDS);
    assert!(indented.len() > 2, "{indented:?}");
    assert!(close(indented[0].0, MARGIN + 20.0), "{indented:?}");
    assert!(indented[0].1 <= MARGIN + 200.0 + 0.05, "{indented:?}");
    assert!(close(indented[1].0, MARGIN), "{indented:?}");
    // A percentage is of the content box's width, and a negative indent hangs out to the left.
    assert!(close(body("text-indent: 10%", WORDS)[0].0, MARGIN + 20.0));
    assert!(close(body("text-indent: -10pt", WORDS)[0].0, MARGIN - 10.0));
    // The property is inherited, so each paragraph's first line is indented. Text after a
    // paragraph inside the same block is not its first line.
    let blocks = body("text-indent: 20pt", "Shop<p style='margin: 0'>Blog</p>More");
    let starts: Vec<f32> = blocks.iter().map(|(left, _)| left - MARGIN).collect();
    assert_eq!(starts.len(), 3, "{blocks:?}");
    assert!(
        close(starts[0], 20.0) && close(starts[1], 20.0),
        "{starts:?}"
    );
    assert!(close(starts[2], 0.0), "{starts:?}");
    // In a right-to-left block the indent is at the right edge, so the first line ends 20pt
    // before the line under it.
    let rtl = body("text-indent: 20pt; direction: rtl", WORDS);
    assert!(rtl.len() > 2, "{rtl:?}");
    assert!(close(rtl[1].1 - rtl[0].1, 20.0), "{rtl:?}");
}

#[test]
fn vertical_align_raises_and_lowers_a_span_from_its_baseline() {
    // The baseline of each text item on the first page, in order.
    let baselines = |html: &str| -> Vec<f32> {
        lay(&format!(
            "<body style='margin: 0; font-size: 12pt; line-height: 20pt'><p>{html}</p>"
        ))
        .pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Text { y, .. } => Some(*y),
                _ => None,
            })
            .collect()
    };
    let span = |css: &str, text: &str| format!("<span style='vertical-align: {css}'>{text}</span>");
    let got = baselines(&format!(
        "Shop{}{}{}{}",
        span("super", "a"),
        span("sub", "b"),
        span("6pt", "c"),
        span("50%", "d")
    ));
    assert_eq!(got.len(), 5, "{got:?}");
    let base = got[0];
    // `super` raises by a third of the parent's 12pt and `sub` lowers by a fifth. A length
    // raises by itself, and a percentage is of the line height.
    for (at, rise) in [(1, 4.0), (2, -2.4), (3, 6.0), (4, 10.0)] {
        assert!(close(base - got[at], rise), "{at}: {got:?}");
    }
    // A raised span inside a raised span adds its own shift to its parent's.
    let nested = baselines(&format!(
        "Shop{}",
        span("super", &format!("a{}", span("super", "b")))
    ));
    assert!(close(nested[0] - nested[2], 8.0), "{nested:?}");
    // A span raised past the line's own height makes the line taller.
    let plain = lay("<p style='margin: 0'>Shop</p><p style='margin: 0'>Blog</p>");
    let raised = lay(&format!(
        "<p style='margin: 0'>Shop{}</p><p style='margin: 0'>Blog</p>",
        span("20pt", "a")
    ));
    let last_baseline = |laid: &Laid| {
        laid.pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Text { y, .. } => Some(*y),
                _ => None,
            })
            .fold(0.0, f32::max)
    };
    assert!(last_baseline(&raised) > last_baseline(&plain) + 10.0);
}
