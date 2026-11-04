//! Page breaks fall where CSS Fragmentation puts them: page sides follow the document's
//! direction, a forced break keeps the next box's top margin, a box's bottom edge never makes a
//! page of its own, fit tests forgive rounding, and a float after a break goes with it
//! (`rule:core-classes/pdf-css-subset`).

use nvs_pdf::layout::{self, Item, Laid};
use nvs_pdf::{assets, css, fonts, parse};

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

/// The text of each text run on page `page` that is not only spaces.
fn texts(laid: &Laid, page: usize) -> Vec<String> {
    laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } => Some(text.trim().to_owned()),
            _ => None,
        })
        .filter(|text| !text.is_empty())
        .collect()
}

/// A left page has a 30 pt left margin and a right page a 10 pt one.
const SIDES: &str = "@page :left { margin-left: 30pt } @page :right { margin-left: 10pt }
    body, p { margin: 0 }";

#[test]
fn in_a_right_to_left_document_the_first_page_is_a_left_page() {
    let laid = lay(&format!(
        "<html style='direction: rtl'><style>{SIDES}</style>
        <p>Shop</p><p style='break-before: page'>Blog</p></html>"
    ));
    assert_eq!(laid.pages.len(), 2);
    let left = laid
        .pages
        .iter()
        .map(|p| p.geometry.margin[3])
        .collect::<Vec<_>>();
    assert!(close(left[0], 30.0) && close(left[1], 10.0), "{left:?}");

    // The same document running left to right starts on a right page.
    let laid = lay(&format!(
        "<style>{SIDES}</style><p>Shop</p><p style='break-before: page'>Blog</p>"
    ));
    let left = laid
        .pages
        .iter()
        .map(|p| p.geometry.margin[3])
        .collect::<Vec<_>>();
    assert!(close(left[0], 10.0) && close(left[1], 30.0), "{left:?}");
}

#[test]
fn recto_and_verso_follow_the_document_direction() {
    let body = "<p>Shop</p><p style='break-before: recto'>Blog</p>
        <p style='break-before: verso'>News</p><p style='break-before: left'>Cart</p>";
    // The first page is recto, so the break to a recto leaves a blank verso before `Blog`,
    // and `News` is on the verso after it. A verso is a left page left to right, so the next
    // left page comes after a blank one. Right to left, a verso is a right page, so it does not.
    for (dir, blank) in [
        ("ltr", vec![false, true, false, false, true, false]),
        ("rtl", vec![false, true, false, false, false]),
    ] {
        let laid = lay(&format!(
            "<html style='direction: {dir}'><style>body, p {{ margin: 0 }}</style>{body}</html>"
        ));
        let got = laid.pages.iter().map(|p| p.blank).collect::<Vec<_>>();
        assert_eq!(got, blank, "{dir}");
        let page = |word: &str| {
            (0..laid.pages.len())
                .find(|&p| texts(&laid, p).iter().any(|t| t == word))
                .unwrap()
        };
        assert_eq!(
            [page("Shop"), page("Blog"), page("News"), page("Cart")],
            [0, 2, 3, blank.len() - 1],
            "{dir}"
        );
    }
}

#[test]
fn a_forced_break_keeps_the_top_margin_of_the_next_box() {
    let laid = lay("<style>body, p { margin: 0 }</style>
        <p style='margin-bottom: 50pt'>Shop</p>
        <p id='blog' style='break-before: page; margin-top: 30pt'>Blog</p>
        <p id='news' style='margin-top: 30pt'>News</p>");
    assert_eq!(laid.pages.len(), 2);
    // The margin after the break is kept, and the one before it is dropped. A top is measured
    // from the page's top edge, so it has the page's top margin in it.
    let page_top = laid.pages[1].geometry.margin[0];
    let (page, top) = laid.anchors["blog"];
    assert!(page == 1 && close(top, page_top + 30.0), "{page} {top}");
    // The margin of a box placed under another one is unchanged.
    let (page, top) = laid.anchors["news"];
    assert!(page == 1 && top > page_top + 30.0 + 30.0, "{page} {top}");
}

#[test]
fn a_box_whose_content_fills_the_page_adds_no_page_for_its_bottom_padding() {
    let laid = lay(
        "<style>@page { size: 200pt 300pt; margin: 0 } body { margin: 0 }</style>
        <div style='padding-bottom: 20pt; border-bottom: 2pt solid black'>
        <div style='height: 300pt'></div></div>",
    );
    assert_eq!(laid.pages.len(), 1);
    assert!(laid.pages.iter().all(|p| !p.blank));
}

#[test]
fn heights_that_sum_to_the_page_in_another_order_still_fit() {
    // Three heights that add up to 300 in decimals, but to more than 300 when `f32` adds them
    // in this order.
    let tenths = |k: u32| (k as f32) / 10.0;
    let heights = (1001..1500)
        .flat_map(|a| (501..1500).map(move |b| (a, b)))
        .filter(|&(a, b)| a + b < 3000)
        .map(|(a, b)| [tenths(a), tenths(b), tenths(3000 - a - b)])
        .find(|h| h[0] + h[1] + h[2] > 300.0)
        .expect("a sum that rounds up");
    let boxes: String = heights
        .iter()
        .map(|h| format!("<div style='height: {h}pt'></div>"))
        .collect();
    let laid = lay(&format!(
        "<style>@page {{ size: 200pt 300pt; margin: 0 }} body {{ margin: 0 }}</style>{boxes}"
    ));
    assert_eq!(laid.pages.len(), 1, "{heights:?}");
}

#[test]
fn a_float_after_a_forced_break_lands_on_the_new_page() {
    let laid = lay("<style>body, p { margin: 0 }</style>
        <p style='break-after: page'>Shop</p>
        <div style='float: left; width: 50pt'>Cart</div>
        <p>Blog</p>");
    assert_eq!(laid.pages.len(), 2);
    assert_eq!(texts(&laid, 0), ["Shop"]);
    let second = texts(&laid, 1);
    assert!(second.iter().any(|t| t == "Cart"), "{second:?}");
}
