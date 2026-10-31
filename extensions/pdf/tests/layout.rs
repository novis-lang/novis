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
    assert!(
        matches!(items[0], Item::Begin { opacity, clip: None, transform: None } if opacity == 0.5)
    );
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

    // `background-size` sizes the tile and `background-position` places it in the padding box.
    let placed = |style: &str| {
        let laid = lay_with(
            &boxed(&format!("background: url(tile.png) no-repeat; {style}")),
            &assets,
        );
        laid.pages[0]
            .items
            .iter()
            .find_map(|item| match *item {
                Item::Image {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => Some((x - MARGIN, y - MARGIN, width, height)),
                _ => None,
            })
            .unwrap()
    };
    let at = |got: (f32, f32, f32, f32), want: (f32, f32, f32, f32)| {
        assert!(
            close(got.0, want.0)
                && close(got.1, want.1)
                && close(got.2, want.2)
                && close(got.3, want.3),
            "{got:?} is not {want:?}"
        );
    };
    at(
        placed("background-position: right bottom"),
        (98.5, 48.5, 1.5, 1.5),
    );
    at(
        placed("background-position: bottom right"),
        (98.5, 48.5, 1.5, 1.5),
    );
    at(
        placed("background-size: cover; background-position: center"),
        (0.0, -25.0, 100.0, 100.0),
    );
    at(
        placed("background-size: contain; background-position: center"),
        (25.0, 0.0, 50.0, 50.0),
    );
    at(
        placed("background-size: 20pt; background-position: 10pt 25%"),
        (10.0, 7.5, 20.0, 20.0),
    );
    at(placed("background-size: 50% 10pt"), (0.0, 0.0, 50.0, 10.0));
    let laid = lay_with(
        &boxed("background: url(tile.png) center / 10pt 5pt"),
        &assets,
    );
    assert!(
        matches!(paths(&laid)[0].1, Paint::Tiles { x, y, width, height, .. } if close(x, MARGIN + 45.0) && close(y, MARGIN + 22.5) && close(width, 10.0) && close(height, 5.0)),
        "{:?}",
        paths(&laid)
    );

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

    // A `::before` or `::after` with `display: block` is a box of its own above or below the
    // element's text, with its own padding.
    let laid = lay("<style>body { margin: 0 } p { margin: 0 }\
         p::before { content: 'Shop'; display: block; padding-left: 20pt }\
         p::after { content: 'Blog'; display: block }</style><p>Text</p>");
    assert_eq!(line_texts(&laid), ["Shop", "Text", "Blog"]);
    let starts: Vec<f32> = lines(&laid).iter().map(|(left, _)| left - MARGIN).collect();
    assert!(
        close(starts[0], 20.0) && close(starts[1], 0.0) && close(starts[2], 0.0),
        "{starts:?}"
    );
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
fn an_svg_text_is_drawn_with_the_default_fonts() {
    // An SVG's `<text>` lays out over the twelve default faces and is drawn as glyphs, so the
    // output embeds a font and its text reads back. A family the database lacks is Serif.
    let svg = |family: &str| {
        format!(
            "<svg width='200' height='40'><text x='4' y='24' font-family='{family}' \
             font-size='16'>Shop total</text></svg>"
        )
    };
    let has = |data: &[u8], what: &[u8]| data.windows(what.len()).any(|w| w == what);
    for (family, face) in [
        ("monospace", &b"LiberationMono"[..]),
        ("sans-serif", b"LiberationSans"),
        ("Unknown", b"LiberationSerif"),
    ] {
        let rendered = render(&Document {
            html: svg(family),
            ..Document::default()
        })
        .unwrap();
        assert!(has(&rendered.data, b"/FontFile2"), "{family}");
        assert!(has(&rendered.data, face), "{family}");
        let bound = nvs_pdf::read::Bound::default();
        let (file, _) = nvs_pdf::read::read(rendered.data, bound).unwrap();
        let text = nvs_pdf::read::text(&file, &[1], bound).unwrap();
        assert!(text[0].contains("Shop total"), "{family}: {text:?}");
    }

    // An `<img>` of an SVG file draws its `<text>` too.
    let assets = [Asset {
        name: "label.svg".to_owned(),
        data: svg("serif")
            .replace("<svg ", "<svg xmlns='http://www.w3.org/2000/svg' ")
            .into_bytes(),
    }];
    let rendered = render(&Document {
        html: "<img src='label.svg'>".to_owned(),
        assets: assets.to_vec(),
        ..Document::default()
    })
    .unwrap();
    assert!(has(&rendered.data, b"LiberationSerif"));
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
        let drawn = items.iter().find(|item| !matches!(item, Item::Tag(_)));
        let clipped = matches!(drawn, Some(Item::Begin { clip: Some(_), .. }));
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
    // `text-top` puts the top of a span's line box, half its leading above its letters, at the
    // top of the paragraph font's letters, and `text-bottom` does the same at the bottom.
    let small = |css: &str| format!("<span style='vertical-align: {css}; font-size: 6pt'>s</span>");
    let edges = baselines(&format!(
        "Shop{}{}{}{}",
        span("text-top", "a"),
        span("text-bottom", "b"),
        small("text-top"),
        small("text-bottom")
    ));
    assert_eq!(edges.len(), 5, "{edges:?}");
    // Liberation Sans' ascent and descent, as fractions of the font size.
    let (ascent, descent) = (1854.0 / 2048.0, 434.0 / 2048.0);
    let half_leading = |size: f32| (20.0 - size * (ascent + descent)) / 2.0;
    let down = |at: usize| edges[at] - edges[0];
    assert!(close(down(1), half_leading(12.0)), "{edges:?}");
    assert!(close(down(2), -half_leading(12.0)), "{edges:?}");
    let top = 6.0 * ascent + half_leading(6.0) - 12.0 * ascent;
    let bottom = 6.0 * descent + half_leading(6.0) - 12.0 * descent;
    assert!(close(down(3), top), "{edges:?}");
    assert!(close(down(4), -bottom), "{edges:?}");
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

/// The pen start and right end of each text item drawn as `…`.
fn ellipses(laid: &Laid) -> Vec<(f32, f32)> {
    laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text {
                x, glyphs, text, ..
            } if text == "\u{2026}" => Some((*x, x + glyphs[0].advance)),
            _ => None,
        })
        .collect()
}

#[test]
fn text_overflow_ellipsis_ends_a_cut_line_inside_its_box() {
    let body = |css: &str, html: &str| {
        lay(&format!(
            "<body style='margin: 0'><div style='width: 100pt; white-space: nowrap; {css}'>{html}</div>"
        ))
    };
    let cut = "overflow: hidden; text-overflow: ellipsis";
    // A line wider than its box keeps the words that fit and ends in `…` at the right edge.
    let laid = body(cut, WORDS);
    let line = lines(&laid);
    assert_eq!(line.len(), 1, "{line:?}");
    let dots = ellipses(&laid);
    assert_eq!(dots.len(), 1, "{dots:?}");
    assert!(close(line[0].1, dots[0].1), "{line:?} {dots:?}");
    assert!(dots[0].1 <= MARGIN + 100.0 + 0.01, "{dots:?}");
    // `visible` overflow and `clip` draw the whole line and no ellipsis.
    for css in [
        "text-overflow: ellipsis",
        "overflow: hidden; text-overflow: clip",
    ] {
        let laid = body(css, WORDS);
        assert!(ellipses(&laid).is_empty(), "{css}");
        assert!(lines(&laid)[0].1 > MARGIN + 100.0, "{css}");
    }
    // A line that fits is not cut.
    assert!(ellipses(&body(cut, "Shop")).is_empty());
    // In a right-to-left block the text is cut at its left end, and `…` is drawn there.
    let hebrew = "שלום עולם ".repeat(8);
    let laid = body(&format!("{cut}; direction: rtl"), &hebrew);
    let dots = ellipses(&laid);
    assert_eq!(dots.len(), 1, "{dots:?}");
    assert!(dots[0].0 >= MARGIN - 0.01, "{dots:?}");
    let (left, right) = lines(&laid)[0];
    assert!(
        close(left, dots[0].0) && right <= MARGIN + 100.0 + 0.01,
        "{left} {right}"
    );
}

#[test]
fn a_right_to_left_list_item_has_its_marker_at_the_right() {
    let laid = lay(
        "<body style='margin: 0'><ol dir='rtl' style='margin: 0; padding: 0 30pt'><li>שלום</li></ol>",
    );
    let runs = runs(&laid);
    assert!(runs.len() >= 2, "{runs:?}");
    // The marker starts at the content box's right edge, level with the item's first line.
    let content = runs[0];
    let right = layout::PAGE_WIDTH - MARGIN - 30.0;
    assert!(close(content.3, right), "{runs:?}");
    let marker = runs[1..].iter().map(|r| r.1).fold(f32::MAX, f32::min);
    assert!(close(marker, right), "{runs:?}");
    assert!(runs[1..].iter().all(|r| close(r.2, content.2)), "{runs:?}");
}

#[test]
fn an_inline_box_draws_its_border_padding_and_background_around_its_text() {
    const YELLOW: (u8, u8, u8) = (255, 255, 0);
    let page = |span: &str, width: &str| {
        lay(&format!(
            "<body style='margin: 0'><p style='margin: 0; {width}'>Shop <span style='{span}'>Blog Blog Blog Blog Blog</span> end</p>"
        ))
    };

    // On one line, the padding and the border at each end push the text beside the span apart.
    let plain = page("", "");
    let laid = page(
        "background-color: #ffff00; padding: 2pt 4pt; border: 1pt solid #ff0000",
        "",
    );
    let grown = lines(&laid)[0].1 - lines(&plain)[0].1;
    assert!(close(grown, 10.0), "{grown}");
    let drawn = runs(&laid);
    assert_eq!(drawn.len(), 3, "{drawn:?}");
    let blog = drawn[1];
    let yellow: Vec<Rect> = rects(&laid).into_iter().filter(|r| r.4 == YELLOW).collect();
    assert_eq!(yellow.len(), 1, "{yellow:?}");
    let (x, y, w, h, _) = yellow[0];
    assert!(
        close(x, blog.1 - 5.0) && close(x + w, blog.3 + 5.0),
        "{yellow:?} {drawn:?}"
    );
    // It reaches above the text and below its baseline by the padding and the border.
    assert!(
        y < blog.2 - 12.0 && y + h > blog.2 + 3.0,
        "{yellow:?} {drawn:?}"
    );
    assert!(rects(&laid).iter().any(|r| r.4 == (255, 0, 0)));

    // Across lines, each line draws its slice: the left padding on the first line only and the
    // right padding on the last line only.
    let laid = page("background-color: #ffff00; padding: 0 6pt", "width: 60pt");
    let mut yellow: Vec<Rect> = rects(&laid).into_iter().filter(|r| r.4 == YELLOW).collect();
    yellow.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert!(yellow.len() >= 3, "{yellow:?}");
    let runs = runs(&laid);
    let first_blog = runs[1];
    assert!(
        close(yellow[0].0, first_blog.1 - 6.0),
        "{yellow:?} {runs:?}"
    );
    assert!(close(yellow[1].0, MARGIN), "{yellow:?}");
    let last = yellow[yellow.len() - 1];
    let end = runs[runs.len() - 1];
    let last_blog = runs[runs.len() - 2];
    assert!(
        close(last.0 + last.2, last_blog.3 + 6.0),
        "{yellow:?} {runs:?}"
    );
    assert!(end.1 >= last.0 + last.2, "{yellow:?} {runs:?}");

    // It renders, and twice to the same bytes.
    let document = Document {
        html: "<p>Shop <span style='border: 1pt dashed red; border-radius: 3pt; background: linear-gradient(white, gray)'>Blog</span></p>".to_owned(),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert_eq!(first.data, render(&document).unwrap().data);
}

#[test]
fn an_inline_block_is_laid_out_as_a_block_and_placed_in_its_line() {
    const YELLOW: (u8, u8, u8) = (255, 255, 0);
    const BOX: &str =
        "display: inline-block; padding: 4pt; border: 1pt solid #ff0000; background-color: #ffff00";
    let page = |inside: &str, style: &str| {
        lay(&format!(
            "<body style='margin: 0'><p style='margin: 0'>Shop <span style='{BOX}; {style}'>{inside}</span> end</p><p style='margin: 0'>Next</p>"
        ))
    };
    let yellow =
        |laid: &Laid| -> Vec<Rect> { rects(laid).into_iter().filter(|r| r.4 == YELLOW).collect() };

    // With no width, the box is as wide as its text and its padding and border, and sits
    // between the words on their baseline.
    let laid = page("Blog", "");
    let mut drawn = runs(&laid);
    let first = drawn.iter().map(|r| r.2).fold(f32::MAX, f32::min);
    drawn.retain(|r| close(r.2, first));
    drawn.sort_by(|a, b| a.1.total_cmp(&b.1));
    let (shop, blog, end) = (drawn[0], drawn[1], drawn[2]);
    let boxes = yellow(&laid);
    assert_eq!(boxes.len(), 1, "{boxes:?}");
    let (x, y, w, h, _) = boxes[0];
    assert!(close(blog.1, x + 5.0), "{boxes:?} {drawn:?}");
    assert!(close(blog.3, x + w - 5.0), "{boxes:?} {drawn:?}");
    assert!(x > shop.3 && end.1 > x + w - 0.05, "{boxes:?} {drawn:?}");
    assert!(close(blog.2, shop.2) && close(end.2, shop.2), "{drawn:?}");
    assert!(
        y < blog.2 - 12.0 && y + h > blog.2 + 3.0,
        "{boxes:?} {drawn:?}"
    );
    assert!(rects(&laid).iter().any(|r| r.4 == (255, 0, 0)));

    // A width of its own is kept, and a taller box makes its line taller.
    let plain = page("Blog", "");
    let tall = page("Blog", "width: 80pt; height: 60pt");
    let boxes = yellow(&tall);
    assert!(
        close(boxes[0].2, 90.0) && close(boxes[0].3, 70.0),
        "{boxes:?}"
    );
    let next = |laid: &Laid| lines(laid).len();
    assert_eq!(next(&tall), next(&plain));
    let baseline = |laid: &Laid, text: &str| {
        laid.pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                Item::Text { y, text: t, .. } if t.starts_with(text) => Some(*y),
                _ => None,
            })
            .unwrap()
    };
    let grown = baseline(&tall, "Next") - baseline(&plain, "Next");
    assert!(grown > 40.0, "{grown}");
    // The box's one line is at its top and on the line's baseline, so the box reaches down
    // from it.
    let shop_line = baseline(&tall, "Shop");
    assert!(
        boxes[0].1 < shop_line && boxes[0].1 + boxes[0].3 > shop_line + 40.0,
        "{boxes:?} {shop_line}"
    );

    // Of two lines in the box, the last one is on the baseline of the words beside it.
    let laid = page("Blog<br>Shop", "");
    let drawn = runs(&laid);
    let mut words: Vec<f32> = drawn.iter().map(|r| r.2).collect();
    words.sort_by(f32::total_cmp);
    // The last baseline is the next paragraph's.
    words.pop();
    assert_eq!(words.len(), 4, "{drawn:?}");
    assert!(words[0] < words[1] - 5.0, "{drawn:?}");
    assert!(
        close(words[2], words[1]) && close(words[3], words[1]),
        "{drawn:?}"
    );

    // An inline-block inside another is drawn inside it.
    let laid = page(
        "Blog <span style='display: inline-block; background-color: #0000ff'>Shop</span>",
        "",
    );
    let outer = yellow(&laid)[0];
    let inner = rects(&laid)
        .into_iter()
        .find(|r| r.4 == (0, 0, 255))
        .unwrap();
    assert!(
        inner.0 > outer.0 && inner.0 + inner.2 < outer.0 + outer.2,
        "{outer:?} {inner:?}"
    );
    assert!(
        inner.1 >= outer.1 && inner.1 + inner.3 <= outer.1 + outer.3,
        "{outer:?} {inner:?}"
    );

    // It renders, and twice to the same bytes.
    let document = Document {
        html: format!("<p>Shop <span style='{BOX}'>Blog</span> end</p>"),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert_eq!(first.data, render(&document).unwrap().data);
}

const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 255, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

/// The filled rectangle of each colour, in the order given.
fn boxes_of(laid: &Laid, colours: &[(u8, u8, u8)]) -> Vec<Rect> {
    colours
        .iter()
        .map(|c| {
            rects(laid)
                .into_iter()
                .find(|r| r.4 == *c)
                .unwrap_or_else(|| panic!("no box of {c:?}: {:?}", rects(laid)))
        })
        .collect()
}

/// A flex container with `style` around three items, red, green and blue, with their own
/// styles and texts.
fn flex(style: &str, items: [(&str, &str); 3]) -> Laid {
    let colours = ["#ff0000", "#00ff00", "#0000ff"];
    let inner: String = items
        .iter()
        .zip(colours)
        .map(|((s, text), c)| format!("<div style='background-color: {c}; {s}'>{text}</div>"))
        .collect();
    lay(&format!(
        "<body style='margin: 0'><div style='display: flex; {style}'>{inner}</div><p style='margin: 0'>Next</p>"
    ))
}

#[test]
fn flex_lays_out_rows_and_columns() {
    let width = 595.275_6 - 2.0 * MARGIN;
    let all = [RED, GREEN, BLUE];

    // In a row, an item with no width is as wide as its text, a growing item takes the free
    // space, and an item with a width keeps it. Every item is as tall as the line.
    let laid = flex(
        "",
        [
            ("padding: 4pt", "Shop"),
            ("flex-grow: 1", "Blog"),
            ("width: 60pt", "End"),
        ],
    );
    let [red, green, blue] = boxes_of(&laid, &all)[..] else {
        unreachable!()
    };
    assert!(close(red.0, MARGIN), "{red:?}");
    assert!(red.2 > 20.0 && red.2 < 60.0, "{red:?}");
    assert!(close(green.0, red.0 + red.2), "{red:?} {green:?}");
    assert!(close(blue.0, green.0 + green.2), "{green:?} {blue:?}");
    assert!(close(blue.2, 60.0), "{blue:?}");
    assert!(close(blue.0 + blue.2, MARGIN + width), "{blue:?}");
    for b in [green, blue] {
        assert!(close(b.1, red.1) && close(b.3, red.3), "{red:?} {b:?}");
    }
    // The padding moves the first text down, the other two share a baseline, and the next
    // paragraph is under the row.
    let lines_drawn = lines(&laid);
    assert_eq!(lines_drawn.len(), 3, "{lines_drawn:?}");
    assert!(
        close(lines_drawn[0].0, green.0) && lines_drawn[0].1 > blue.0,
        "{lines_drawn:?}"
    );
    let next = runs(&laid).last().unwrap().2;
    assert!(next > red.1 + red.3, "{next} {red:?}");

    // In a column with a gap, each item is as wide as the container and under the one before.
    let laid = flex(
        "flex-direction: column; row-gap: 10pt",
        [("", "Shop"), ("", "Blog"), ("", "End")],
    );
    let [red, green, blue] = boxes_of(&laid, &all)[..] else {
        unreachable!()
    };
    for b in [red, green, blue] {
        assert!(close(b.0, MARGIN) && close(b.2, width), "{b:?}");
    }
    assert!(close(green.1, red.1 + red.3 + 10.0), "{red:?} {green:?}");
    assert!(
        close(blue.1, green.1 + green.3 + 10.0),
        "{green:?} {blue:?}"
    );

    // `justify-content: center` shares the free space on both sides, `align-items: center`
    // centres a short item against a tall one, and `order` moves an item first.
    let laid = flex(
        "justify-content: center; align-items: center",
        [
            ("width: 100pt; height: 40pt", "Shop"),
            ("width: 100pt; height: 20pt", "Blog"),
            ("width: 100pt; height: 40pt; order: -1", "End"),
        ],
    );
    let [red, green, blue] = boxes_of(&laid, &all)[..] else {
        unreachable!()
    };
    assert!(close(blue.0, MARGIN + (width - 300.0) / 2.0), "{blue:?}");
    assert!(close(red.0, blue.0 + 100.0) && close(green.0, red.0 + 100.0));
    assert!(
        close(green.3, 20.0) && close(green.1, red.1 + 10.0),
        "{red:?} {green:?}"
    );

    // Under `flex-wrap: wrap`, an item that does not fit starts a new line.
    let laid = flex(
        "flex-wrap: wrap",
        [
            ("width: 200pt", "Shop"),
            ("width: 200pt", "Blog"),
            ("width: 200pt", "End"),
        ],
    );
    let [red, green, blue] = boxes_of(&laid, &all)[..] else {
        unreachable!()
    };
    assert!(
        close(green.1, red.1) && close(green.0, red.0 + 200.0),
        "{green:?}"
    );
    assert!(
        close(blue.0, MARGIN) && close(blue.1, red.1 + red.3),
        "{blue:?}"
    );

    // Text directly in the container is an item of its own.
    let laid = lay(
        "<body style='margin: 0'><div style='display: flex'>Shop <span style='background-color: #ff0000'>Blog</span></div>",
    );
    let drawn = runs(&laid);
    assert_eq!(drawn.len(), 2, "{drawn:?}");
    let red = boxes_of(&laid, &[RED])[0];
    assert!(
        close(red.0, drawn[0].3) || red.0 > drawn[0].3,
        "{red:?} {drawn:?}"
    );
    assert!(close(drawn[0].2, drawn[1].2), "{drawn:?}");

    // It renders, and twice to the same bytes.
    let document = Document {
        html: "<div style='display: flex; gap: 8pt'><p style='flex: 1'>Shop</p><p>Blog</p></div>"
            .into(),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert_eq!(first.data, render(&document).unwrap().data);
}

/// The top of each box on page `page` that is filled with one of `colours`, with its colour.
fn tops_on(laid: &Laid, page: usize, colours: &[(u8, u8, u8)]) -> Vec<((u8, u8, u8), f32)> {
    laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Path {
                path,
                paint: Paint::Color(c),
                stroke: None,
            } if colours.contains(&(c.r, c.g, c.b)) => {
                Some(((c.r, c.g, c.b), layout::bounds(path).1))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn flex_and_grid_break_only_between_items_and_rows() {
    let all = [RED, GREEN, BLUE];
    let item = |c: &str| format!("<div style='background-color: {c}; height: 60pt'>Shop</div>");
    let items: String = ["#ff0000", "#00ff00", "#0000ff"].map(item).concat();

    // A flex line that does not fit under the block above it goes to the next page whole.
    let laid = lay(&format!(
        "<div style='height: 700pt'></div><div style='display: flex'>{items}</div>"
    ));
    assert_eq!(laid.pages.len(), 2, "{:?}", rects(&laid));
    assert!(
        tops_on(&laid, 0, &all).is_empty(),
        "{:?}",
        tops_on(&laid, 0, &all)
    );
    let moved = tops_on(&laid, 1, &all);
    assert_eq!(moved.len(), 3, "{moved:?}");
    assert!(
        moved.iter().all(|(_, top)| close(*top, MARGIN)),
        "{moved:?}"
    );

    // A grid breaks between its rows: the first row stays and the second starts the next page.
    let laid = lay(&format!(
        "<div style='height: 620pt'></div>\
         <div style='display: grid; grid-template-columns: 1fr 1fr; row-gap: 10pt'>{items}</div>"
    ));
    assert_eq!(laid.pages.len(), 2);
    let stayed = tops_on(&laid, 0, &all);
    assert_eq!(stayed.len(), 2, "{stayed:?}");
    assert!(close(stayed[0].1, stayed[1].1), "{stayed:?}");
    assert_eq!(tops_on(&laid, 1, &all), vec![(BLUE, MARGIN)]);
}

#[test]
fn grid_places_items_in_tracks() {
    let width = 595.275_6 - 2.0 * MARGIN;
    let all = [RED, GREEN, BLUE];

    // A fixed column, then two `fr` columns that share what is left after the gaps.
    let laid = flex(
        "display: grid; grid-template-columns: 100pt 1fr 2fr; column-gap: 10pt",
        [("", "Shop"), ("", "Blog"), ("", "End")],
    );
    let [red, green, blue] = boxes_of(&laid, &all)[..] else {
        unreachable!()
    };
    let share = (width - 120.0) / 3.0;
    assert!(close(red.0, MARGIN) && close(red.2, 100.0), "{red:?}");
    assert!(
        close(green.0, MARGIN + 110.0) && close(green.2, share),
        "{green:?}"
    );
    assert!(
        close(blue.0, green.0 + share + 10.0) && close(blue.2, 2.0 * share),
        "{blue:?}"
    );
    for b in [green, blue] {
        assert!(close(b.1, red.1) && close(b.3, red.3), "{red:?} {b:?}");
    }
    let next = runs(&laid).last().unwrap().2;
    assert!(next > red.1 + red.3, "{next} {red:?}");

    // Lines and rows: the first item spans both columns, the second is placed in the second
    // row's second column, and the third fills the first free cell.
    let laid = flex(
        "display: grid; grid-template-columns: 1fr 1fr; grid-template-rows: 30pt 50pt",
        [
            ("grid-column: 1 / 3", "Shop"),
            ("grid-row: 2; grid-column: 2", "Blog"),
            ("", "End"),
        ],
    );
    let [red, green, blue] = boxes_of(&laid, &all)[..] else {
        unreachable!()
    };
    assert!(
        close(red.0, MARGIN) && close(red.2, width) && close(red.3, 30.0),
        "{red:?}"
    );
    assert!(
        close(green.0, MARGIN + width / 2.0) && close(green.1, red.1 + 30.0),
        "{green:?}"
    );
    assert!(close(green.3, 50.0), "{green:?}");
    assert!(close(blue.0, MARGIN) && close(blue.1, green.1), "{blue:?}");

    // Named areas.
    let laid = flex(
        "display: grid; grid-template-columns: 80pt 1fr; grid-template-areas: \"head head\" \"side main\"",
        [
            ("grid-area: main", "Shop"),
            ("grid-area: head", "Blog"),
            ("grid-area: side", "End"),
        ],
    );
    let [red, green, blue] = boxes_of(&laid, &all)[..] else {
        unreachable!()
    };
    assert!(close(green.0, MARGIN) && close(green.2, width), "{green:?}");
    assert!(close(blue.0, MARGIN) && close(blue.2, 80.0), "{blue:?}");
    assert!(close(blue.1, green.1 + green.3), "{green:?} {blue:?}");
    assert!(
        close(red.0, MARGIN + 80.0) && close(red.1, blue.1),
        "{red:?} {blue:?}"
    );

    // `repeat()` and `span`: an item two columns wide, then the next row once the first is
    // full. Under `justify-items: start`, an item is as wide as its text.
    let laid = flex(
        "display: grid; grid-template-columns: repeat(3, 50pt); justify-items: start",
        [
            ("grid-column: span 2; justify-self: stretch", "Shop"),
            ("", "Blog"),
            ("grid-column: span 2", "End"),
        ],
    );
    let [red, green, blue] = boxes_of(&laid, &all)[..] else {
        unreachable!()
    };
    assert!(close(red.0, MARGIN) && close(red.2, 100.0), "{red:?}");
    assert!(
        close(green.0, MARGIN + 100.0) && green.2 < 50.0,
        "{green:?}"
    );
    assert!(
        close(blue.0, MARGIN) && close(blue.1, red.1 + red.3),
        "{blue:?}"
    );

    // It renders, and twice to the same bytes.
    let document = Document {
        html: "<div style='display: grid; grid-template-columns: 1fr 1fr'><p>Shop</p><p>Blog</p></div>"
            .into(),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert_eq!(first.data, render(&document).unwrap().data);
}

/// The baseline of the first text item that starts with `text`.
fn baseline_of(laid: &Laid, text: &str) -> f32 {
    laid.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Item::Text { y, text: t, .. } if t.starts_with(text) => Some(*y),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text {text}"))
}

#[test]
fn a_float_right_wraps_the_text_beside_it() {
    const FLOAT: &str = "width: 100pt; height: 60pt; background-color: #0000ff";
    let right = layout::PAGE_WIDTH - MARGIN;

    // A right float is at the right of its containing block, and the lines beside it end
    // before it. The lines below it are as wide as the box again.
    let laid = lay(&format!(
        "<body style='margin: 0'><div style='float: right; {FLOAT}'></div><p style='margin: 0'>{WORDS} {WORDS} {WORDS} {WORDS} {WORDS}</p>"
    ));
    let [float] = boxes_of(&laid, &[BLUE])[..] else {
        unreachable!()
    };
    assert!(
        close(float.0 + float.2, right) && close(float.1, MARGIN),
        "{float:?}"
    );
    assert!(close(float.2, 100.0) && close(float.3, 60.0), "{float:?}");
    let drawn = lines(&laid);
    let baselines: Vec<f32> = {
        let mut b: Vec<f32> = runs(&laid).iter().map(|r| r.2).collect();
        b.sort_by(f32::total_cmp);
        b.dedup_by(|a, b| close(*a, *b));
        b
    };
    let beside: Vec<_> = drawn
        .iter()
        .zip(&baselines)
        .filter(|(_, y)| **y < MARGIN + 60.0)
        .collect();
    let below: Vec<_> = drawn
        .iter()
        .zip(&baselines)
        .filter(|(_, y)| **y > MARGIN + 80.0)
        .collect();
    assert!(beside.len() >= 3 && below.len() >= 2, "{drawn:?}");
    for ((l, r), _) in &beside {
        assert!(close(*l, MARGIN) && *r <= float.0 + 0.05, "{drawn:?}");
    }
    assert!(
        below.iter().any(|((_, r), _)| *r > float.0 + 20.0),
        "{drawn:?}"
    );

    // A left float moves the lines beside it to the right, and an inline element can float.
    let laid = lay(&format!(
        "<body style='margin: 0'><p style='margin: 0'><span style='float: left; {FLOAT}'></span>{WORDS}</p>"
    ));
    let [float] = boxes_of(&laid, &[BLUE])[..] else {
        unreachable!()
    };
    assert!(
        close(float.0, MARGIN) && close(float.1, MARGIN),
        "{float:?}"
    );
    let first = lines(&laid)[0];
    assert!(close(first.0, MARGIN + 100.0), "{first:?}");

    // `clear` moves a block below the float. Without it, a short paragraph sits beside it.
    let short = |clear: &str| {
        lay(&format!(
            "<body style='margin: 0'><div style='float: right; {FLOAT}'></div><p style='margin: 0'>Shop</p><p style='margin: 0; {clear}'>Blog</p>"
        ))
    };
    let beside = baseline_of(&short(""), "Blog");
    let cleared = baseline_of(&short("clear: right"), "Blog");
    assert!(beside < MARGIN + 60.0, "{beside}");
    assert!(cleared > MARGIN + 60.0, "{cleared}");
    assert!(baseline_of(&short("clear: left"), "Blog") < MARGIN + 60.0);
    assert!(baseline_of(&short("clear: both"), "Blog") > MARGIN + 60.0);

    // Two right floats sit side by side, the second to the left of the first, and a float
    // with no width is as wide as its content.
    let laid = lay(&format!(
        "<body style='margin: 0'><div style='float: right; {FLOAT}'></div><div style='float: right; height: 20pt; background-color: #ff0000'>Shop</div><p style='margin: 0'>Blog</p>"
    ));
    let [blue, red] = boxes_of(&laid, &[BLUE, RED])[..] else {
        unreachable!()
    };
    assert!(
        close(red.0 + red.2, blue.0) && close(red.1, blue.1),
        "{blue:?} {red:?}"
    );
    assert!(red.2 > 10.0 && red.2 < 60.0, "{red:?}");

    // It renders, and twice to the same bytes.
    let document = Document {
        html: format!("<div style='float: right; {FLOAT}'></div><p>{WORDS}</p>"),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert_eq!(first.data, render(&document).unwrap().data);
}

/// The index on the first page of the first box filled with `colour`.
fn index_of(laid: &Laid, colour: (u8, u8, u8)) -> usize {
    laid.pages[0]
        .items
        .iter()
        .position(|item| {
            matches!(item, Item::Path { paint: Paint::Color(c), stroke: None, .. }
                if (c.r, c.g, c.b) == colour)
        })
        .unwrap_or_else(|| panic!("no box of {colour:?}"))
}

#[test]
fn absolute_positions_against_its_containing_block() {
    const BOX: &str = "position: relative; margin-left: 50pt; width: 300pt; height: 200pt; \
                       border: 10pt solid #000000; padding: 5pt; background-color: #ff0000";
    const BLUE_BOX: &str =
        "position: absolute; width: 40pt; height: 50pt; background-color: #0000ff";

    // An absolute box is placed against the padding box of the positioned box around it, and
    // takes no room: the text after it is where it would be without it.
    let laid = lay(&format!(
        "<body style='margin: 0'><p style='margin: 0'>Shop</p><div style='{BOX}'><div style='{BLUE_BOX}; top: 20pt; right: 30pt'></div><p style='margin: 0'>Blog</p></div><p style='margin: 0'>Next</p>"
    ));
    let [red, blue] = boxes_of(&laid, &[RED, BLUE])[..] else {
        unreachable!()
    };
    assert!(
        close(red.0, MARGIN + 50.0) && close(red.2, 330.0),
        "{red:?}"
    );
    assert!(
        close(blue.0, red.0 + 10.0 + 310.0 - 30.0 - 40.0) && close(blue.1, red.1 + 10.0 + 20.0),
        "{red:?} {blue:?}"
    );
    assert!(close(blue.2, 40.0) && close(blue.3, 50.0), "{blue:?}");
    let blog = baseline_of(&laid, "Blog");
    assert!(blog > red.1 + 15.0 && blog < red.1 + 40.0, "{blog} {red:?}");
    assert!(baseline_of(&laid, "Next") > red.1 + red.3, "{red:?}");

    // `bottom` and `left` place it against the bottom-left corner of that padding box.
    let laid = lay(&format!(
        "<body style='margin: 0'><div style='{BOX}'><div style='{BLUE_BOX}; bottom: 0; left: 0'></div></div>"
    ));
    let [red, blue] = boxes_of(&laid, &[RED, BLUE])[..] else {
        unreachable!()
    };
    assert!(
        close(blue.0, red.0 + 10.0) && close(blue.1 + blue.3, red.1 + red.3 - 10.0),
        "{red:?} {blue:?}"
    );

    // With no positioned box around it, the page's content box is its containing block.
    let laid = lay(&format!(
        "<body style='margin: 0'><div style='margin-left: 50pt'><div style='{BLUE_BOX}; bottom: 0; right: 0'></div></div><p>Shop</p>"
    ));
    let [blue] = boxes_of(&laid, &[BLUE])[..] else {
        unreachable!()
    };
    assert!(
        close(blue.0 + blue.2, layout::PAGE_WIDTH - MARGIN)
            && close(blue.1 + blue.3, layout::PAGE_HEIGHT - MARGIN),
        "{blue:?}"
    );

    // A relative box is drawn moved by its offsets, and the block after it keeps its place.
    let relative = |offset: &str| {
        lay(&format!(
            "<body style='margin: 0'><div style='height: 40pt; background-color: #00ff00; {offset}'></div><p style='margin: 0'>Next</p>"
        ))
    };
    let still = relative("");
    let moved = relative("position: relative; top: 10pt; left: 20pt");
    let ([before], [after]) = (
        &boxes_of(&still, &[GREEN])[..],
        &boxes_of(&moved, &[GREEN])[..],
    ) else {
        unreachable!()
    };
    assert!(
        close(after.0, before.0 + 20.0) && close(after.1, before.1 + 10.0),
        "{before:?} {after:?}"
    );
    assert!(close(
        baseline_of(&still, "Next"),
        baseline_of(&moved, "Next")
    ));

    // Positioned boxes are drawn over the others by `z-index`, then in document order, and a
    // box below 0 is drawn under the boxes in the flow.
    let laid = lay(&format!(
        "<body style='margin: 0'><div style='{BLUE_BOX}; top: 0; left: 0; z-index: 2'></div><div style='{BLUE_BOX}; top: 0; left: 0; z-index: 1; background-color: #00ff00'></div><div style='{BLUE_BOX}; top: 0; left: 0; z-index: -1; background-color: #ff0000'></div><div style='height: 60pt; background-color: #ffff00'></div>"
    ));
    let order = [RED, (255, 255, 0), GREEN, BLUE].map(|c| index_of(&laid, c));
    assert!(order.is_sorted(), "{order:?}");

    // A box with a `z-index` draws the boxes inside it with it: a child below 0 is still over
    // its parent's background, and the parent stays under a later box with a higher `z-index`.
    let laid = lay(&format!(
        "<body style='margin: 0'><div style='{BOX}; z-index: 1'><div style='{BLUE_BOX}; top: 0; left: 0; z-index: -1'></div></div><div style='{BLUE_BOX}; top: 0; left: 0; z-index: 2; background-color: #00ff00'></div>"
    ));
    let order = [RED, BLUE, GREEN].map(|c| index_of(&laid, c));
    assert!(order.is_sorted(), "{order:?}");

    // It renders, and twice to the same bytes.
    let document = Document {
        html: format!(
            "<div style='{BOX}'><div style='{BLUE_BOX}; top: 20pt; right: 30pt'></div>{WORDS}</div>"
        ),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert_eq!(first.data, render(&document).unwrap().data);
}

/// The corners of each box filled with `colour` on `page`, left, top, right and bottom, and its
/// place among the page's items.
fn on_page(laid: &Laid, page: usize, colour: (u8, u8, u8)) -> Vec<(usize, (f32, f32, f32, f32))> {
    laid.pages[page]
        .items
        .iter()
        .enumerate()
        .filter_map(|(at, item)| match item {
            Item::Path {
                path,
                paint: Paint::Color(c),
                stroke: None,
            } if (c.r, c.g, c.b) == colour => Some((at, layout::bounds(path))),
            _ => None,
        })
        .collect()
}

#[test]
fn fixed_repeats_on_every_page() {
    const BLUE_FIXED: &str =
        "position: fixed; width: 40pt; height: 50pt; background-color: #0000ff";
    const TALL: &str = "<div style='height: 700pt; background-color: #ff0000'></div>";

    // A `fixed` box is drawn on every page, at the same place against the page's content box,
    // even inside a positioned box, and takes no room in the flow.
    let laid = lay(&format!(
        "<body style='margin: 0'><div style='position: relative; margin-left: 50pt'><div style='{BLUE_FIXED}; top: 100pt; right: 0'></div></div>{TALL}{TALL}{TALL}"
    ));
    assert_eq!(laid.pages.len(), 3);
    for page in 0..3 {
        let [(at, blue)] = on_page(&laid, page, BLUE)[..] else {
            panic!("page {page}: {:?}", on_page(&laid, page, BLUE))
        };
        assert!(
            close(blue.2, layout::PAGE_WIDTH - MARGIN)
                && close(blue.1, MARGIN + 100.0)
                && close(blue.2 - blue.0, 40.0)
                && close(blue.3 - blue.1, 50.0),
            "page {page}: {blue:?}"
        );
        // The red boxes start at the top of every page, under the blue one.
        let reds = on_page(&laid, page, RED);
        assert!(close(reds[0].1.1, MARGIN), "page {page}: {reds:?}");
        assert!(reds.iter().all(|r| r.0 < at), "page {page}: {reds:?} {at}");
    }

    // A watermark with a `z-index` below 0 is drawn under the page's other items.
    let laid = lay(&format!(
        "<body style='margin: 0'><div style='{BLUE_FIXED}; bottom: 0; left: 0; z-index: -1'></div>{TALL}{TALL}"
    ));
    for page in 0..2 {
        let [(at, blue)] = on_page(&laid, page, BLUE)[..] else {
            unreachable!()
        };
        assert!(
            close(blue.0, MARGIN) && close(blue.3, layout::PAGE_HEIGHT - MARGIN),
            "page {page}: {blue:?}"
        );
        let reds = on_page(&laid, page, RED);
        assert!(
            !reds.is_empty() && reds.iter().all(|r| at < r.0),
            "page {page}: {at} {reds:?}"
        );
    }

    // It renders, and twice to the same bytes.
    let document = Document {
        html: format!("<div style='{BLUE_FIXED}; top: 0; left: 0'></div>{TALL}{TALL}"),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert_eq!(first.data, render(&document).unwrap().data);
}

/// The matrix of each group on the first page that has one, and the items inside it.
fn transformed(laid: &Laid) -> Vec<(layout::Matrix, Vec<Item>)> {
    let items = &laid.pages[0].items;
    items
        .iter()
        .enumerate()
        .filter_map(|(at, item)| match item {
            Item::Begin {
                transform: Some(m), ..
            } => {
                let mut depth = 0;
                let inside = items[at + 1..]
                    .iter()
                    .take_while(|i| {
                        match i {
                            Item::Begin { .. } => depth += 1,
                            Item::End if depth == 0 => return false,
                            Item::End => depth -= 1,
                            _ => {}
                        }
                        true
                    })
                    .cloned()
                    .collect();
                Some((*m, inside))
            }
            _ => None,
        })
        .collect()
}

fn apply(m: layout::Matrix, x: f32, y: f32) -> (f32, f32) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

#[test]
fn a_rotated_transform_is_drawn_rotated() {
    let boxed = |style: &str| {
        lay(&format!(
            "<body style='margin: 0'><div style='width: 100pt; height: 50pt; background-color: #00ff00; {style}'>Shop</div><p>Blog</p>"
        ))
    };

    // A rotation turns the box and its text about the centre of its border box, and the
    // paragraph after it is not in the group.
    let laid = boxed("transform: rotate(90deg)");
    let [(m, inside)] = &transformed(&laid)[..] else {
        panic!("{:?}", transformed(&laid))
    };
    let (cx, cy) = (MARGIN + 50.0, MARGIN + 25.0);
    let centre = apply(*m, cx, cy);
    assert!(close(centre.0, cx) && close(centre.1, cy), "{m:?}");
    let corner = apply(*m, MARGIN, MARGIN);
    assert!(
        close(corner.0, cx + 25.0) && close(corner.1, cy - 50.0),
        "{m:?} {corner:?}"
    );
    assert!(inside.iter().any(|i| matches!(i, Item::Path { .. })));
    assert!(inside.iter().any(|i| matches!(i, Item::Text { .. })));
    assert_eq!(drawn(&laid).concat().matches("Blog").count(), 1);
    let texts = inside
        .iter()
        .filter(|i| matches!(i, Item::Text { .. }))
        .count();
    assert_eq!(texts, 1, "{inside:?}");

    // The functions apply left to right about `transform-origin`, and a percentage in
    // `translate` counts against the border box.
    let laid = boxed("transform-origin: left top; transform: translate(10pt, 50%) scale(2)");
    let [(m, _)] = &transformed(&laid)[..] else {
        unreachable!()
    };
    let corner = apply(*m, MARGIN + 100.0, MARGIN + 50.0);
    assert!(
        close(corner.0, MARGIN + 210.0) && close(corner.1, MARGIN + 125.0),
        "{m:?} {corner:?}"
    );

    // An absolute box turns about its own centre where it is placed.
    let laid = lay(
        "<body style='margin: 0'><div style='position: absolute; top: 100pt; left: 100pt; width: 40pt; height: 40pt; background-color: #0000ff; transform: rotate(45deg)'></div>",
    );
    let [(m, _)] = &transformed(&laid)[..] else {
        unreachable!()
    };
    let (cx, cy) = (MARGIN + 120.0, MARGIN + 120.0);
    let centre = apply(*m, cx, cy);
    assert!(close(centre.0, cx) && close(centre.1, cy), "{m:?}");
    let corner = apply(*m, MARGIN + 100.0, MARGIN + 100.0);
    assert!(
        close(corner.0, cx) && close(corner.1, cy - 20.0 * 2.0_f32.sqrt()),
        "{m:?} {corner:?}"
    );

    // `none`, and a function with an argument it does not take, draw the box as it is.
    for style in ["transform: none", "transform: rotate(10pt)"] {
        assert!(transformed(&boxed(style)).is_empty(), "{style}");
    }

    // It renders, and twice to the same bytes.
    let document = Document {
        html: "<div style='width: 100pt; transform: rotate(-30deg) skewX(10deg)'>Shop</div>"
            .to_owned(),
        ..Document::default()
    };
    let first = render(&document).unwrap();
    assert_eq!(first.data, render(&document).unwrap().data);
}
