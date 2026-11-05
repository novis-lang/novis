//! Edge cases of text and of the cascade on the host: the font chain for digits, CJK line
//! breaks, `overflow-wrap`, `<pre>`, an `inline-block` ending in a space, entities,
//! `!important` in a shorthand, `box-sizing` and percent margins, a float wider than its
//! container and list markers past their alphabet (`rule:core-classes/pdf-css-subset`,
//! `rule:core-classes/pdf-default-fonts`).

use nvs_pdf::layout::{self, Href, Item, Laid, MARGIN, Paint, bounds};
use nvs_pdf::{Asset, assets, css, fonts, parse};

const SERIF: &[u8] = include_bytes!("../../fonts/LiberationSerif-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../image/fixtures/NotoSansArabic-Regular.ttf");

fn lay_with(html: &str, files: &[Asset]) -> Laid {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(files);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    layout::layout(&dom, &styles, &assets, fonts).unwrap()
}

fn lay(html: &str) -> Laid {
    lay_with(html, &[])
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

/// A text run as drawn: its face, the text its glyphs draw, its pen start, its baseline and its
/// right end, the spaces at its end not counted.
struct Run {
    face: usize,
    text: String,
    x: f32,
    y: f32,
    right: f32,
}

/// Each text run on `page` that draws something.
fn runs(laid: &Laid, page: usize) -> Vec<Run> {
    laid.pages[page]
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
                let mut pen = *x;
                let mut right = *x;
                for g in glyphs {
                    pen += g.advance;
                    if !text[g.text.clone()].trim().is_empty() {
                        right = pen;
                    }
                }
                let mut clusters: Vec<_> = glyphs.iter().map(|g| g.text.clone()).collect();
                clusters.dedup();
                Some(Run {
                    face: *face,
                    text: clusters.into_iter().map(|r| &text[r]).collect(),
                    x: *x,
                    y: *y,
                    right,
                })
            }
            _ => None,
        })
        .filter(|run| !run.text.trim().is_empty())
        .collect()
}

/// The text of each line on `page`, top to bottom: the runs on one baseline, left to right.
fn lines(laid: &Laid, page: usize) -> Vec<String> {
    let mut all = runs(laid, page);
    all.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let mut out: Vec<(f32, String)> = Vec::new();
    for run in all {
        match out.last_mut() {
            Some((y, text)) if close(*y, run.y) => text.push_str(&run.text),
            _ => out.push((run.y, run.text)),
        }
    }
    out.into_iter()
        .map(|(_, text)| text.trim().to_owned())
        .collect()
}

/// The left, top, right and bottom around every filled shape on `page` in the colour `rgb`.
fn shape(laid: &Laid, page: usize, rgb: (u8, u8, u8)) -> [f32; 4] {
    let found: Vec<[f32; 4]> = laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Path {
                path,
                paint: Paint::Color(c),
                stroke: None,
            } if (c.r, c.g, c.b) == rgb => {
                let (left, top, right, bottom) = bounds(path);
                Some([left, top, right, bottom])
            }
            _ => None,
        })
        .collect();
    assert!(!found.is_empty(), "no shape in {rgb:?}");
    found.iter().fold(
        [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
        |[l, t, r, b], s| [l.min(s[0]), t.min(s[1]), r.max(s[2]), b.max(s[3])],
    )
}

const RED: (u8, u8, u8) = (255, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

/// A family listed first draws the digits it has. The next family in the list draws only what
/// the first one has no glyph for.
#[test]
fn a_family_listed_first_that_has_digits_draws_them() {
    let brand = [Asset {
        name: "brand.ttf".to_owned(),
        data: SERIF.to_vec(),
    }];
    let laid = lay_with(
        "<style>@font-face { font-family: Brand; src: url(brand.ttf) }\
         p { font-family: Brand, monospace }</style><p>Shop 2024</p>",
        &brand,
    );
    let all = runs(&laid, 0);
    assert!(!all.is_empty());
    assert!(
        all.iter().all(|r| r.face == 12),
        "{:?}",
        all.iter().map(|r| (&r.text, r.face)).collect::<Vec<_>>()
    );

    let kufi = [Asset {
        name: "kufi.ttf".to_owned(),
        data: ARABIC.to_vec(),
    }];
    let laid = lay_with(
        "<style>@font-face { font-family: Kufi; src: url(kufi.ttf) }\
         p { font-family: Kufi, serif }</style><p>2024</p>",
        &kufi,
    );
    assert!(laid.missing.is_empty(), "{:?}", laid.missing);
    let all = runs(&laid, 0);
    assert!(
        all.iter().all(|r| r.face == 12),
        "{:?}",
        all.iter().map(|r| (&r.text, r.face)).collect::<Vec<_>>()
    );
}

/// CJK text wrapped in a narrow box never starts a line with closing punctuation, at any
/// width.
#[test]
fn cjk_kinsoku_keeps_closing_punctuation_off_a_line_start() {
    let text = "東京は大きい。今日は、晴れです。「こんにちは」と言った。店は、近い。";
    for width in (30..=120).step_by(5) {
        let laid = lay(&format!(
            "<body style='margin: 0'><p style='width: {width}pt; font-size: 10pt'>{text}</p>"
        ));
        let all = lines(&laid, 0);
        assert!(all.len() > 1, "{width}: {all:?}");
        assert_eq!(all.concat(), text, "{width}");
        for line in &all {
            let first = line.chars().next().unwrap();
            assert!(!"。、」）．，".contains(first), "{width}: {all:?}");
        }
    }
}

/// `overflow-wrap: break-word` breaks a word too long for its line even when the word starts
/// inside an inline element, so no text leaves the box.
#[test]
fn break_word_breaks_a_long_word_that_starts_after_an_inline_element() {
    let word = "Shop".repeat(15);
    for (html, drawn) in [
        (
            format!("<p class='w'><b>x</b>{word}</p>"),
            format!("x{word}"),
        ),
        (
            format!("<p class='w'>lorem <b>x</b>{word}</p>"),
            format!("loremx{word}"),
        ),
        (
            format!("<p class='w'><b>x</b> {word}</p>"),
            format!("x{word}"),
        ),
    ] {
        let laid = lay(&format!(
            "<style>body {{ margin: 0 }} .w {{ width: 100pt; overflow-wrap: break-word }}</style>\
             {html}"
        ));
        let all = runs(&laid, 0);
        assert!(lines(&laid, 0).len() > 2, "{html}: {:?}", lines(&laid, 0));
        for run in &all {
            assert!(
                run.right <= MARGIN + 100.5,
                "{html}: {} ends at {}",
                run.text,
                run.right
            );
        }
        assert_eq!(lines(&laid, 0).concat().replace(' ', ""), drawn, "{html}");
    }
}

/// A `<pre>` drops the newline right after its start tag and keeps the spaces at the start of
/// the next line.
#[test]
fn pre_drops_the_first_newline_and_keeps_leading_spaces() {
    let style = "<style>body { margin: 0 } pre { font-size: 10pt }</style>";
    let plain = lay(&format!("{style}<pre>Shop</pre>"));
    let spaced = lay(&format!("{style}<pre>\n  Shop\nBlog</pre>"));
    let plain = runs(&plain, 0);
    let spaced = runs(&spaced, 0);
    let shop = |all: &[Run]| -> (f32, f32) {
        let run = all.iter().find(|r| r.text.contains("Shop")).unwrap();
        (run.right, run.y)
    };
    // The first line of text is on the same baseline: no empty line comes before it.
    assert!(
        close(shop(&plain).1, shop(&spaced).1),
        "{:?} {:?}",
        shop(&plain),
        shop(&spaced)
    );
    // `Shop` ends two monospace spaces further right: 2 × 0.6 × 10pt.
    let shift = shop(&spaced).0 - shop(&plain).0;
    assert!((shift - 2.0 * 0.6 * 10.0).abs() < 0.1, "{shift}");
    assert_eq!(
        spaced.len(),
        2,
        "{:?}",
        spaced.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
}

/// An `inline-block` whose text ends in a space is as wide as its words, so they stay on one
/// line.
#[test]
fn an_inline_block_ending_in_a_space_keeps_its_words_on_one_line() {
    for inner in ["Shop items ", "Shop items  ", "Shop <b>items</b> "] {
        let laid = lay(&format!(
            "<body style='margin: 0'><p style='width: 300pt'>\
             <span style='display: inline-block'>{inner}</span>lorem</p>"
        ));
        let all = lines(&laid, 0);
        assert!(all[0].starts_with("Shop items"), "{inner:?}: {all:?}");
    }
}

/// Named and numbered character references are decoded in text and in an attribute that
/// reaches the page.
#[test]
fn entities_are_decoded_in_text_and_attributes() {
    let laid = lay(
        "<style>body { margin: 0 } p.t::before { content: attr(title) }</style>\
         <p>Shop &amp; Blog &lt;b&gt; caf&#233; caf&eacute; caf&#xE9;</p>\
         <p class='t' title='Shop &amp; &lt;Blog&gt; &#233;'></p>\
         <p><a href='https://example.com/?a=1&amp;b=2&#38;c=3'>link</a></p>",
    );
    let all = lines(&laid, 0);
    assert_eq!(all[0], "Shop & Blog <b> café café café");
    assert_eq!(all[1], "Shop & <Blog> é");
    let links: Vec<&Href> = laid.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Link { href, .. } => Some(href),
            _ => None,
        })
        .collect();
    assert!(
        matches!(links[..], [Href::Uri(uri)] if uri == "https://example.com/?a=1&b=2&c=3"),
        "{links:?}"
    );
}

/// `!important` on a shorthand makes every longhand it sets important, so a later rule with a
/// more specific selector does not override one of them.
#[test]
fn important_in_a_shorthand_wins_for_every_longhand() {
    let laid = lay("<style>body { margin: 0 } div { height: 10pt }\
         .a { background-color: #0000ff }\
         .b { margin: 10pt !important; background-color: #ff0000 }\
         body div#x.b { margin-top: 50pt; margin-left: 40pt }</style>\
         <div class='a'></div><div id='x' class='b'></div>");
    let blue = shape(&laid, 0, BLUE);
    let red = shape(&laid, 0, RED);
    assert!(close(red[1] - blue[3], 10.0), "{blue:?} {red:?}");
    assert!(close(red[0] - blue[0], 10.0), "{blue:?} {red:?}");
    assert!(close(red[2], blue[2] - 10.0), "{blue:?} {red:?}");
}

/// `box-sizing: border-box` puts the padding and the border inside `width`, and a percent
/// `margin-top` is a share of the containing block's width, not of its height.
#[test]
fn border_box_and_percent_vertical_margins_follow_css() {
    let laid = lay("<style>body { margin: 0 }\
         .c { width: 200pt; height: 400pt; padding-top: 1pt; background-color: #0000ff }\
         .r { box-sizing: border-box; width: 100pt; height: 60pt; padding: 10pt;\
              border: 5pt solid #ff0000; background-color: #ff0000; margin-top: 10% }\
         </style><div class='c'><div class='r'>x</div></div>");
    let blue = shape(&laid, 0, BLUE);
    let red = shape(&laid, 0, RED);
    assert!(close(red[2] - red[0], 100.0), "{red:?}");
    assert!(close(red[3] - red[1], 60.0), "{red:?}");
    // 1pt of padding, then 10% of 200pt.
    assert!(close(red[1] - blue[1], 21.0), "{blue:?} {red:?}");
}

/// A float wider than its container has no room beside another float, so it goes below the
/// float before it, at the container's left edge. No line of text sits beside it: the text
/// fills the room beside the first float, then goes on below the wide one.
#[test]
fn a_float_wider_than_its_container_sits_alone_on_its_line() {
    let text = "lorem ".repeat(40);
    let laid = lay(&format!(
        "<style>body {{ margin: 0 }} .c {{ width: 200pt }}\
         .n {{ float: left; width: 50pt; height: 40pt; background-color: #0000ff }}\
         .w {{ float: left; width: 300pt; height: 60pt; background-color: #ff0000 }}\
         </style><div class='c'><div class='n'></div><div class='w'></div>{text}</div>"
    ));
    let blue = shape(&laid, 0, BLUE);
    let red = shape(&laid, 0, RED);
    assert!(close(red[0], MARGIN), "{red:?}");
    assert!(close(red[1], blue[3]), "{blue:?} {red:?}");
    let all = runs(&laid, 0);
    for run in &all {
        assert!(
            run.y < red[1] || run.y > red[3],
            "{} at {} {red:?}",
            run.text,
            run.y
        );
        if run.y < red[1] {
            assert!(run.x >= blue[2] - 0.5, "{} at {} {blue:?}", run.text, run.x);
        }
    }
    assert!(all.iter().any(|r| r.y < red[1]));
    assert!(all.iter().any(|r| r.y > red[3]));
}

/// `upper-alpha` goes on from `Z` to `AA` and from `ZZ` to `AAA`. `upper-roman` and
/// `lower-roman` write 1 to 3999, and any other number in decimal.
#[test]
fn list_markers_go_past_z_and_past_3999() {
    let markers = |kind: &str, start: i64, count: usize| -> Vec<String> {
        let items = "<li>lorem</li>".repeat(count);
        let laid = lay(&format!(
            "<ol style='list-style-type: {kind}' start='{start}'>{items}</ol>"
        ));
        lines(&laid, 0)
            .into_iter()
            .map(|line| line.trim_end_matches("lorem").trim().to_owned())
            .collect()
    };
    assert_eq!(markers("upper-alpha", 25, 3), ["Y.", "Z.", "AA."]);
    assert_eq!(markers("upper-alpha", 702, 2), ["ZZ.", "AAA."]);
    assert_eq!(markers("lower-alpha", 52, 2), ["az.", "ba."]);
    assert_eq!(
        markers("upper-roman", 3998, 3),
        ["MMMCMXCVIII.", "MMMCMXCIX.", "4000."]
    );
    assert_eq!(markers("lower-roman", -1, 3), ["-1.", "0.", "i."]);
}
