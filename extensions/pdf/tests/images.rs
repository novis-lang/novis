//! An `<img>` and an inline `<svg>` are replaced boxes: inline in their line, floated, or blocks
//! of their own, sized by their style within its least and greatest sizes, and turned by a
//! JPEG's EXIF orientation (`rule:core-classes/pdf-css-subset`).

use nvs_pdf::layout::{self, Item, Laid};
use nvs_pdf::{Asset, Document, assets, css, fonts, parse, render};

fn lay_with(html: &str, list: &[Asset]) -> Laid {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(list);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    layout::layout(&dom, &styles, &assets, fonts).unwrap()
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

fn tile() -> [Asset; 1] {
    [Asset {
        name: "tile.png".to_owned(),
        data: png(),
    }]
}

fn lay(html: &str) -> Laid {
    lay_with(html, &tile())
}

/// Each image on page `page`: its left, top, width and height.
fn images(laid: &Laid, page: usize) -> Vec<[f32; 4]> {
    laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Image {
                x,
                y,
                width,
                height,
                ..
            } => Some([*x, *y, *width, *height]),
            _ => None,
        })
        .collect()
}

/// Each word on page `page`: its text, its left end, its right end and its baseline. A word's
/// ends are where the pen is at its first glyph and after its last.
fn words(laid: &Laid, page: usize) -> Vec<(String, f32, f32, f32)> {
    let mut out = Vec::new();
    for item in &laid.pages[page].items {
        let Item::Text {
            x, y, glyphs, text, ..
        } = item
        else {
            continue;
        };
        let mut pen = *x;
        let mut current: Option<(String, f32, f32)> = None;
        // Glyphs of one cluster share its text, which the word takes once.
        let mut last = usize::MAX..usize::MAX;
        for g in glyphs {
            let piece = &text[g.text.clone()];
            if piece.trim().is_empty() {
                if let Some((word, left, right)) = current.take() {
                    out.push((word, left, right, *y));
                }
            } else {
                let (word, _, right) = current.get_or_insert((String::new(), pen, pen));
                if g.text != last {
                    word.push_str(piece);
                }
                *right = pen + g.advance;
            }
            last = g.text.clone();
            pen += g.advance;
        }
        if let Some((word, left, right)) = current {
            out.push((word, left, right, *y));
        }
    }
    out
}

fn word(laid: &Laid, page: usize, text: &str) -> (String, f32, f32, f32) {
    words(laid, page)
        .into_iter()
        .find(|w| w.0.contains(text))
        .unwrap_or_else(|| panic!("no `{text}` in {:?}", words(laid, page)))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

#[test]
fn an_img_in_a_paragraph_sits_on_the_line_between_its_words() {
    let laid = lay(
        "<p>Shop <img src='tile.png' style='width: 20pt; height: 10pt' alt='Logo'> \
         example</p>",
    );
    let [[x, y, width, height]] = images(&laid, 0)[..] else {
        panic!("{:?}", images(&laid, 0));
    };
    assert!(
        close(width, 20.0) && close(height, 10.0),
        "{width} {height}"
    );
    let shop = word(&laid, 0, "Shop");
    let example = word(&laid, 0, "example");
    // The three are on one line, in source order, and the picture's bottom is on the baseline.
    assert!(close(shop.3, example.3), "{shop:?} {example:?}");
    assert!(
        shop.2 <= x + 0.01 && x + width <= example.1 + 0.01,
        "{shop:?} {x} {example:?}"
    );
    assert!(close(y + height, shop.3), "{y} {height} {shop:?}");
    // The picture keeps its figure tag around it.
    let items = &laid.pages[0].items;
    let at = items
        .iter()
        .position(|i| matches!(i, Item::Image { .. }))
        .unwrap();
    assert!(
        items[..at].iter().rev().any(|i| matches!(
            i,
            Item::Tag(layout::Role::Figure(Some(alt))) if alt == "Logo"
        )),
        "{:?}",
        &items[..at]
    );
}

#[test]
fn vertical_align_middle_centres_an_img_on_the_line() {
    let at = |align: &str| {
        let laid = lay(&format!(
            "<p style='font-size: 10pt'>Shop <img src='tile.png' \
             style='width: 40pt; height: 40pt; vertical-align: {align}'> example</p>"
        ));
        let [[_, y, _, height]] = images(&laid, 0)[..] else {
            panic!("{:?}", images(&laid, 0));
        };
        (y, height, word(&laid, 0, "Shop").3)
    };
    let (y, height, baseline) = at("baseline");
    assert!(close(y + height, baseline), "{y} {height} {baseline}");
    // `middle` puts the picture's centre half the x-height, about 2.5pt, above the baseline.
    let (y, height, baseline) = at("middle");
    let centre = y + height / 2.0;
    assert!(
        centre < baseline && (baseline - centre - 2.5).abs() < 1.5,
        "{centre} {baseline}"
    );
}

#[test]
fn a_floated_img_inside_a_link_wraps_the_text_beside_it() {
    let text = "lorem ipsum dolor sit amet ".repeat(40);
    let laid = lay(&format!(
        "<p style='font-size: 10pt'><a href='https://example.com/'><img src='tile.png' \
         style='float: left; width: 60pt; height: 60pt'></a>{text}</p>"
    ));
    let [[x, y, width, height]] = images(&laid, 0)[..] else {
        panic!("{:?}", images(&laid, 0));
    };
    assert!(close(width, 60.0) && close(height, 60.0));
    let all = words(&laid, 0);
    // Lines beside the picture start to its right, and lines below it at its left edge.
    let beside: Vec<_> = all.iter().filter(|w| w.3 < y + height).collect();
    let below: Vec<_> = all.iter().filter(|w| w.3 > y + height + 12.0).collect();
    assert!(!beside.is_empty() && !below.is_empty(), "{all:?}");
    assert!(beside.iter().all(|w| w.1 >= x + width - 0.01), "{beside:?}");
    assert!(below.iter().any(|w| close(w.1, x)), "{below:?}");
}

#[test]
fn display_block_makes_an_img_its_own_block() {
    let laid = lay(
        "<p>Shop <img src='tile.png' style='display: block; width: 20pt; height: 10pt'> \
         example</p>",
    );
    let [[_, y, _, height]] = images(&laid, 0)[..] else {
        panic!("{:?}", images(&laid, 0));
    };
    // The text before the picture is above it, and the text after it is below it.
    assert!(word(&laid, 0, "Shop").3 < y, "{:?}", words(&laid, 0));
    assert!(
        word(&laid, 0, "example").3 > y + height,
        "{:?}",
        words(&laid, 0)
    );
}

#[test]
fn max_width_max_height_and_a_percent_height_limit_an_img() {
    let size = |css: &str| {
        let laid = lay(&format!(
            "<style>@page {{ size: 300pt 200pt; margin: 0 }}</style>\
             <body style='margin: 0'><img src='tile.png' style='{css}'></body>"
        ));
        let [[_, _, width, height]] = images(&laid, 0)[..] else {
            panic!("{css}: {:?}", images(&laid, 0));
        };
        (width, height)
    };
    let is = |css: &str, want: (f32, f32)| {
        let got = size(css);
        assert!(
            close(got.0, want.0) && close(got.1, want.1),
            "{css}: {got:?}"
        );
    };
    // The picture is square, and a limit on one side keeps it square.
    is("width: 200pt; max-width: 100pt", (100.0, 100.0));
    is("width: 80pt; max-height: 40pt", (40.0, 40.0));
    is("min-width: 30pt", (30.0, 30.0));
    is("min-height: 24pt; max-width: 12pt", (12.0, 24.0));
    // With both sizes given, each limit applies to its own side.
    is("width: 80pt; height: 20pt; max-width: 50pt", (50.0, 20.0));
    // A percent height counts against the page's content height of 200pt.
    is("height: 10%", (20.0, 20.0));
    is("height: 150pt; max-height: 25%", (50.0, 50.0));
}

#[test]
fn an_img_taller_than_the_page_moves_whole_to_the_next_page_and_shrinks_to_fit() {
    let laid = lay("<style>@page { size: 200pt 200pt; margin: 10pt }</style>\
         <body style='margin: 0'><p>Shop</p>\
         <p><img src='tile.png' style='width: 100pt; height: 400pt'></p></body>");
    assert!(laid.pages.len() >= 2, "{} pages", laid.pages.len());
    assert!(images(&laid, 0).is_empty(), "{:?}", images(&laid, 0));
    assert!(!words(&laid, 0).is_empty());
    let [[_, y, width, height]] = images(&laid, 1)[..] else {
        panic!("{:?}", images(&laid, 1));
    };
    // The picture keeps its aspect ratio at the page's content height of 180pt.
    assert!(
        close(height, 180.0) && close(width, 45.0),
        "{width} {height}"
    );
    assert!(y >= 10.0 - 0.01 && y + height <= 200.0 + 0.01, "{y}");
}

/// A grey JPEG `width` by `height` pixels, every pixel the same, with an EXIF segment that gives
/// `orientation` when there is one. Each 8 by 8 block is coded as a zero DC difference and an
/// end of block, each a one-bit Huffman code of `0`.
fn jpeg(width: u16, height: u16, orientation: Option<u16>) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8];
    let mut segment = |marker: u8, body: &[u8]| {
        out.extend([0xff, marker]);
        out.extend(u16::try_from(body.len() + 2).unwrap().to_be_bytes());
        out.extend(body);
    };
    if let Some(turn) = orientation {
        let mut exif = b"Exif\0\0MM\0\x2a\0\0\0\x08\0\x01".to_vec();
        exif.extend([0x01, 0x12, 0x00, 0x03, 0, 0, 0, 1]);
        exif.extend(turn.to_be_bytes());
        exif.extend([0, 0, 0, 0, 0, 0]);
        segment(0xe1, &exif);
    }
    let mut dqt = vec![0];
    dqt.extend([1; 64]);
    segment(0xdb, &dqt);
    let mut sof = vec![8];
    sof.extend(height.to_be_bytes());
    sof.extend(width.to_be_bytes());
    sof.extend([1, 1, 0x11, 0]);
    segment(0xc0, &sof);
    for class in [0x00, 0x10] {
        let mut dht = vec![class, 1];
        dht.extend([0; 15]);
        dht.push(0);
        segment(0xc4, &dht);
    }
    segment(0xda, &[1, 1, 0x00, 0, 63, 0]);
    let blocks = usize::from(width.div_ceil(8)) * usize::from(height.div_ceil(8));
    let bits = 2 * blocks;
    let mut data = vec![0u8; bits.div_ceil(8)];
    // The bits after the last block are padded with ones.
    if bits % 8 != 0 {
        *data.last_mut().unwrap() = 0xff >> (bits % 8);
    }
    out.extend(data);
    out.extend([0xff, 0xd9]);
    out
}

#[test]
fn a_jpeg_with_an_exif_orientation_is_drawn_turned() {
    let size = |orientation: Option<u16>| {
        let assets = [Asset {
            name: "photo.jpg".to_owned(),
            data: jpeg(16, 8, orientation),
        }];
        let html = "<img src='photo.jpg'>";
        let laid = lay_with(html, &assets);
        let [[_, _, width, height]] = images(&laid, 0)[..] else {
            panic!("{:?}", images(&laid, 0));
        };
        // The file writes, with its picture turned.
        render(&Document {
            html: html.to_owned(),
            assets: assets.to_vec(),
            ..Document::default()
        })
        .unwrap();
        (width, height)
    };
    // 16 by 8 pixels is 12pt by 6pt.
    assert_eq!(size(None), (12.0, 6.0));
    assert_eq!(size(Some(1)), (12.0, 6.0));
    assert_eq!(size(Some(3)), (12.0, 6.0));
    // Orientations 5 to 8 turn the picture a quarter turn, so its width and height swap.
    assert_eq!(size(Some(6)), (6.0, 12.0));
    assert_eq!(size(Some(8)), (6.0, 12.0));
}
