//! Pagination on the host: the `@page` rules, forced breaks and margin boxes
//! (`rule:core-classes/pdf-css-subset-table`).

use nvs_pdf::layout::{self, Item, Laid};
use nvs_pdf::{Document, assets, css, fonts, parse, render};

fn lay(html: &str) -> Laid {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(&[]);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    layout::layout(&dom, &styles, &assets, fonts).unwrap()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

fn mm(v: f32) -> f32 {
    v * 72.0 / 25.4
}

/// Each text run on `page`: its text, its pen start and its baseline.
fn texts(laid: &Laid, page: usize) -> Vec<(String, f32, f32)> {
    laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, x, y, .. } => Some((text.trim().to_owned(), *x, *y)),
            _ => None,
        })
        .filter(|(text, ..)| !text.is_empty())
        .collect()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn page_size_margins_and_first_left_right_blank() {
    let laid = lay("<style>
        @page { size: A5 landscape; margin: 20mm }
        @page :first { margin-top: 40mm }
        @page :left { margin-left: 30mm; margin-right: 10mm }
        @page :right { margin-left: 10mm; margin-right: 30mm }
        @page :blank { size: A4 }
        body, p { margin: 0 }
        </style>
        <p>Shop</p><p style='break-before: page'>Blog</p><p style='break-before: left'>News</p>");
    assert_eq!(laid.pages.len(), 4);
    let [first, left, blank, last] = [0, 1, 2, 3].map(|at| laid.pages[at].geometry);

    // The first page is a right page, A5 turned on its side, with the `:first` top margin.
    assert!(
        close(first.width, mm(210.0)) && close(first.height, mm(148.0)),
        "{first:?}"
    );
    let want = [mm(40.0), mm(30.0), mm(20.0), mm(10.0)];
    assert!(
        first.margin.iter().zip(want).all(|(a, b)| close(*a, b)),
        "{first:?}"
    );
    let [(text, x, y)] = &texts(&laid, 0)[..] else {
        panic!("{:?}", texts(&laid, 0));
    };
    assert_eq!(text, "Shop");
    assert!(
        close(*x, mm(10.0)) && *y > mm(40.0) && *y < mm(40.0) + 20.0,
        "{x} {y}"
    );

    // The second page is a left one, with the plain top margin.
    assert!(
        close(left.margin[3], mm(30.0)) && close(left.margin[0], mm(20.0)),
        "{left:?}"
    );
    let [(text, x, _)] = &texts(&laid, 1)[..] else {
        panic!("{:?}", texts(&laid, 1));
    };
    assert_eq!(text, "Blog");
    assert!(close(*x, mm(30.0)), "{x}");

    // A break to a left page after a left page leaves a blank right page between them.
    assert!(laid.pages[2].blank && laid.pages[2].items.is_empty());
    assert!(close(blank.width, layout::PAGE_WIDTH) && close(blank.height, layout::PAGE_HEIGHT));
    assert!(!last.margin.is_empty() && !laid.pages[3].blank);
    assert_eq!(texts(&laid, 3)[0].0, "News");
    assert!(close(texts(&laid, 3)[0].1, mm(30.0)));

    // A custom size takes its width first, and a margin's percentage counts against the side.
    let custom = lay("<style>@page { size: 100mm 50mm; margin: 10% }</style><p>Shop</p>");
    let page = custom.pages[0].geometry;
    assert!(
        close(page.width, mm(100.0)) && close(page.height, mm(50.0)),
        "{page:?}"
    );
    assert!(
        close(page.margin[0], mm(5.0)) && close(page.margin[3], mm(10.0)),
        "{page:?}"
    );

    // A size that does not read is A4, and the file's pages carry their own sizes.
    let unread = lay("<style>@page { size: A4 tall }</style><p>Shop</p>");
    assert!(close(unread.pages[0].geometry.height, layout::PAGE_HEIGHT));
    let rendered = render(&Document {
        html: "<style>@page { size: 300pt 200pt }</style><p>Shop</p>".to_owned(),
        ..Document::default()
    })
    .unwrap();
    assert!(contains(&rendered.data, b"/MediaBox[0 0 300 200]"));
}

/// The text runs on `page` whose baseline is between `top` and `bottom`, joined in order, and
/// the left and right end of the first and last.
fn band(laid: &Laid, page: usize, top: f32, bottom: f32) -> (String, f32, f32) {
    let mut text = String::new();
    let (mut left, mut right) = (f32::MAX, f32::MIN);
    for item in &laid.pages[page].items {
        if let Item::Text {
            text: run,
            x,
            y,
            glyphs,
            ..
        } = item
            && *y > top
            && *y < bottom
        {
            text.push_str(run);
            left = left.min(*x);
            let inked = glyphs
                .iter()
                .rev()
                .skip_while(|g| run[g.text.clone()].trim().is_empty());
            right = right.max(x + inked.map(|g| g.advance).sum::<f32>());
        }
    }
    (text.trim().to_owned(), left, right)
}

#[test]
fn margin_boxes_print_page_x_of_y() {
    let laid = lay("<style>
        @page {
            size: 400pt 300pt;
            margin: 50pt;
            @bottom-right { content: 'Page ' counter(page) ' of ' counter(pages) }
            @top-center { content: 'Shop'; font-size: 10pt }
            @right-middle { content: counter(page, upper-roman) }
        }
        @page :first { @bottom-right { content: none } }
        body, p { margin: 0 }
        p { break-after: page }
        </style><p>One</p><p>Two</p><p>Three</p>");
    assert_eq!(laid.pages.len(), 3);
    // The first page's own rule removes its footer.
    assert_eq!(band(&laid, 0, 250.0, 300.0).0, "");
    for page in 1..3 {
        let (footer, _, right) = band(&laid, page, 250.0, 300.0);
        assert_eq!(footer, format!("Page {} of 3", page + 1));
        // The bottom-right box is the whole side when it is alone there, and its text ends at
        // the content box's right edge.
        assert!(close(right, 350.0), "{right}");
    }
    for page in 0..3 {
        // The top box is alone on its side, so its text is centred on the page.
        let (header, left, right) = band(&laid, page, 0.0, 50.0);
        assert_eq!(header, "Shop");
        assert!(((left + right) / 2.0 - 200.0).abs() < 0.5, "{left} {right}");
        // The right-side box is centred in the right margin, half way down.
        let roman = ["I", "II", "III"][page];
        let side = laid.pages[page].items.iter().find_map(|item| match item {
            Item::Text { text, x, y, .. } if text.trim() == roman && *x > 350.0 => Some(*y),
            _ => None,
        });
        assert!(side.is_some_and(|y| y > 120.0 && y < 180.0), "{side:?}");
    }
}

#[test]
fn a_running_element_repeats_on_every_page() {
    let laid = lay("<style>
        @page {
            size: 400pt 300pt;
            margin: 50pt;
            @top-center { content: element(head) }
            @bottom-left { content: element(foot, last) }
        }
        body, p, div { margin: 0 }
        .head { position: running(head); font-size: 10pt }
        .foot { position: running(foot) }
        p { break-after: page }
        </style>
        <div class=head>Shop</div><p>One</p><p>Two</p>
        <div class=head>News</div><div class=foot>Blog</div><div class=foot>Team</div>
        <p>Three</p>");
    assert_eq!(laid.pages.len(), 3);
    // A running element takes no room, so the body starts at the top of the content box.
    let (body, ..) = band(&laid, 0, 50.0, 250.0);
    assert_eq!(body, "One");
    let one = texts(&laid, 0)
        .into_iter()
        .find(|(t, ..)| t == "One")
        .unwrap();
    assert!(one.2 > 50.0 && one.2 < 50.0 + 20.0, "{one:?}");
    for (page, want) in [(0, "Shop"), (1, "Shop"), (2, "News")] {
        // The page's first running element, or the last of an earlier page, is the header,
        // centred on the page because its box is alone on the top side.
        let (header, left, right) = band(&laid, page, 0.0, 50.0);
        assert_eq!(header, want, "page {page}");
        assert!(((left + right) / 2.0 - 200.0).abs() < 0.5, "{left} {right}");
    }
    // `last` takes the last element on its page, and no page before the first has the box.
    assert_eq!(band(&laid, 0, 250.0, 300.0).0, "");
    assert_eq!(band(&laid, 1, 250.0, 300.0).0, "");
    let (footer, left, _) = band(&laid, 2, 250.0, 300.0);
    assert_eq!(footer, "Team");
    assert!(close(left, 50.0), "{left}");
}
