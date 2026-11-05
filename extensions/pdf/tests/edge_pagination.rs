//! Pagination edge cases: adjacent and trailing forced breaks, breaks in fixed boxes and through
//! wrappers, margins at unforced breaks, kept runs, orphans and widows, split lists and counters,
//! fixed content on every page and the size of a blank page (`rule:core-classes/pdf-css-subset`).

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

/// Each text run on `page` that is not only spaces: its text and its baseline.
fn runs(laid: &Laid, page: usize) -> Vec<(String, f32)> {
    laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, y, .. } => Some((text.trim().to_owned(), *y)),
            _ => None,
        })
        .filter(|(text, _)| !text.is_empty())
        .collect()
}

/// The text of each run on each page.
fn lines(laid: &Laid) -> Vec<Vec<String>> {
    (0..laid.pages.len())
        .map(|page| runs(laid, page).into_iter().map(|(t, _)| t).collect())
        .collect()
}

/// The baseline of the first run on `page` whose text is `text`.
fn baseline(laid: &Laid, page: usize, text: &str) -> f32 {
    runs(laid, page)
        .into_iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("no {text:?} on page {page}: {:?}", lines(laid)))
        .1
}

/// A page 200pt tall inside its margins, with lines 20pt tall, so ten lines fill it.
const TEN_LINES: &str = "<style>
    @page { size: 400pt 300pt; margin: 50pt }
    body, p, div, ol, ul, li { margin: 0; font-size: 10pt; line-height: 20pt }
    </style>";

#[test]
fn two_forced_breaks_in_a_row_make_one_break() {
    let laid = lay(&format!(
        "{TEN_LINES}<p style='break-after: page'>Shop</p><p style='break-before: page'>Blog</p>"
    ));
    assert_eq!(lines(&laid), [["Shop"], ["Blog"]]);

    // Breaks after a box, after its wrapper and before the next box are still one break.
    let laid = lay(&format!(
        "{TEN_LINES}<div style='break-after: page'><p style='break-after: page'>Shop</p></div>
        <div style='break-before: page'><p style='break-before: page'>Blog</p></div>"
    ));
    assert_eq!(lines(&laid), [["Shop"], ["Blog"]]);
}

#[test]
fn a_trailing_break_after_adds_no_page() {
    let laid = lay(&format!(
        "{TEN_LINES}<p>Shop</p><p style='break-after: page'>Blog</p>"
    ));
    assert_eq!(lines(&laid), [["Shop", "Blog"]]);

    // The same break on the last box inside a wrapper adds no page either.
    let laid = lay(&format!(
        "{TEN_LINES}<p>Shop</p><div style='break-after: page'><p style='break-after: page'>Blog</p></div>"
    ));
    assert_eq!(lines(&laid), [["Shop", "Blog"]]);
}

#[test]
fn a_forced_break_at_the_top_of_a_page_adds_no_page() {
    let laid = lay(&format!(
        "{TEN_LINES}<p style='break-before: page'>Shop</p><p>Blog</p>"
    ));
    assert_eq!(lines(&laid), [["Shop", "Blog"]]);

    // The first box of the first box breaks before nothing too.
    let laid = lay(&format!(
        "{TEN_LINES}<div style='break-before: page'><p style='break-before: page'>Shop</p></div>"
    ));
    assert_eq!(lines(&laid), [["Shop"]]);

    // A break before the first box on a page started by a break adds no second page.
    let laid = lay(&format!(
        "{TEN_LINES}<p>Shop</p><div style='break-before: page'><p style='break-before: page'>Blog</p></div>"
    ));
    assert_eq!(lines(&laid), [["Shop"], ["Blog"]]);
}

#[test]
fn a_break_inside_a_fixed_element_is_ignored() {
    let laid = lay(&format!(
        "{TEN_LINES}<p>Shop</p>
        <div style='position: fixed; top: 0; left: 0; width: 100pt'>
            <p>Blog</p><p style='break-before: page'>News</p><p style='break-after: page'>Team</p>
        </div>
        <p>Cart</p>"
    ));
    assert_eq!(laid.pages.len(), 1, "{:?}", lines(&laid));
    let page = &lines(&laid)[0];
    for text in ["Shop", "Blog", "News", "Team", "Cart"] {
        assert!(page.iter().any(|t| t == text), "{text} {page:?}");
    }
}

#[test]
fn a_break_after_passes_up_through_wrapper_elements() {
    let laid = lay(&format!(
        "{TEN_LINES}<div style='padding-bottom: 10pt'><div><div>
            <p>Shop</p><p style='break-after: page'>Blog</p>
        </div></div></div><p>News</p>"
    ));
    assert_eq!(lines(&laid), [vec!["Shop", "Blog"], vec!["News"]]);
    // The wrappers' bottom padding stays on the first page, so `News` starts the second page
    // where `Shop` starts the first.
    assert!(close(
        baseline(&laid, 1, "News"),
        baseline(&laid, 0, "Shop")
    ));
}

#[test]
fn an_unforced_break_drops_the_adjoining_margin_inside_a_flow_root() {
    let filler = "<p>Line</p>".repeat(10);
    for root in ["", "display: flow-root", "overflow: hidden"] {
        let laid = lay(&format!(
            "{TEN_LINES}<div style='{root}'>{filler}<p style='margin-top: 30pt'>Shop</p></div>"
        ));
        let pages = lines(&laid);
        assert_eq!(pages.len(), 2, "{root}: {pages:?}");
        assert_eq!(pages[1], ["Shop"], "{root}: {pages:?}");
        // `Shop` starts the page with no margin above it, as the first line of the first page.
        assert!(
            close(baseline(&laid, 1, "Shop"), baseline(&laid, 0, "Line")),
            "{root}: {} {}",
            baseline(&laid, 1, "Shop"),
            baseline(&laid, 0, "Line")
        );
    }
}

#[test]
fn a_chain_of_three_avoid_boxes_moves_together() {
    let filler = "<p>Line</p>".repeat(8);
    let laid = lay(&format!(
        "{TEN_LINES}{filler}<p style='break-after: avoid'>Shop</p>
        <p style='break-after: avoid'>Blog</p><p>News</p>"
    ));
    let pages = lines(&laid);
    assert_eq!(pages.len(), 2, "{pages:?}");
    assert_eq!(pages[0], vec!["Line"; 8], "{pages:?}");
    assert_eq!(pages[1], ["Shop", "Blog", "News"], "{pages:?}");

    // Without the chain the first two boxes fill the first page.
    let laid = lay(&format!(
        "{TEN_LINES}{filler}<p>Shop</p><p>Blog</p><p>News</p>"
    ));
    assert_eq!(lines(&laid)[1], ["News"]);
}

#[test]
fn orphans_and_widows_split_seven_lines_as_css_says() {
    let words = ["Lorem", "ipsum", "dolor", "sit", "amet", "elit", "sed"];
    let paragraph = words.join("<br>");
    // Six lines leave room for four lines of the paragraph on the first page.
    let filler = "<p>Line</p>".repeat(6);
    for (style, first) in [
        ("orphans: 2; widows: 2", 4),
        ("orphans: 5", 0),
        ("widows: 4", 3),
        ("orphans: 4; widows: 4", 0),
    ] {
        let laid = lay(&format!(
            "{TEN_LINES}{filler}<p style='{style}'>{paragraph}</p>"
        ));
        let pages = lines(&laid);
        assert_eq!(pages.len(), 2, "{style}: {pages:?}");
        assert_eq!(pages[0][..6], vec!["Line"; 6], "{style}: {pages:?}");
        assert_eq!(pages[0][6..], words[..first], "{style}: {pages:?}");
        assert_eq!(pages[1], words[first..], "{style}: {pages:?}");
    }
}

#[test]
fn a_paragraph_split_across_pages_keeps_every_character_once() {
    let source = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor \
        incididunt ut labore et dolore magna aliqua. Shop and Blog at example.com. "
        .repeat(12);
    let laid = lay(&format!("{TEN_LINES}<p>{source}</p>"));
    assert!(laid.pages.len() >= 2, "{}", laid.pages.len());
    let drawn: String = (0..laid.pages.len())
        .flat_map(|page| runs(&laid, page))
        .map(|(text, _)| text)
        .collect::<String>()
        .split_whitespace()
        .collect();
    let want: String = source.split_whitespace().collect();
    assert_eq!(drawn, want);
}

#[test]
fn a_split_list_item_draws_its_marker_on_the_first_page_only() {
    let filler = "<p>Line</p>".repeat(6);
    let laid = lay(&format!(
        "{TEN_LINES}{filler}<ol><li>Lorem<br>ipsum<br>dolor<br>sit<br>amet<br>elit<br>sed</li></ol>"
    ));
    let pages = lines(&laid);
    assert_eq!(pages.len(), 2, "{pages:?}");
    // The marker is drawn once, beside the item's first line.
    assert_eq!(
        pages[0].iter().filter(|t| *t == "1.").count(),
        1,
        "{pages:?}"
    );
    assert!(close(baseline(&laid, 0, "1."), baseline(&laid, 0, "Lorem")));
    // Four of the seven lines fit under the six, and the last three go on.
    assert_eq!(pages[1], ["amet", "elit", "sed"], "{pages:?}");
}

#[test]
fn list_numbers_and_counters_continue_across_pages() {
    let items = "<li>Shop</li>".repeat(12);
    let laid = lay(&format!("{TEN_LINES}<ol>{items}</ol>"));
    let pages = lines(&laid);
    assert_eq!(pages.len(), 2, "{pages:?}");
    assert!(pages[0].iter().any(|t| t == "10."), "{pages:?}");
    assert!(pages[1].iter().any(|t| t == "11."), "{pages:?}");
    assert!(pages[1].iter().any(|t| t == "12."), "{pages:?}");
    assert!(!pages[1].iter().any(|t| t == "1."), "{pages:?}");

    // A counter shown by `::before` goes on counting on the next page.
    let blocks = "<div class=c>Blog</div>".repeat(12);
    let laid = lay(&format!(
        "{TEN_LINES}<style>body {{ counter-reset: x }} .c {{ counter-increment: x }}
        .c::before {{ content: 'N' counter(x) ' ' }}</style>{blocks}"
    ));
    let joined: Vec<String> = lines(&laid).iter().map(|p| p.concat()).collect();
    assert_eq!(joined.len(), 2, "{joined:?}");
    assert!(joined[0].contains("N10"), "{joined:?}");
    assert!(joined[1].starts_with("N11"), "{joined:?}");
    assert!(joined[1].contains("N12"), "{joined:?}");
}

#[test]
fn fixed_content_repeats_on_pages_made_only_by_forced_breaks() {
    let laid = lay(&format!(
        "{TEN_LINES}<div style='position: fixed; top: 0; left: 0'>Mark</div>
        <p>Shop</p><p style='break-before: page'>Blog</p><p style='break-before: page'>News</p>"
    ));
    let pages = lines(&laid);
    assert_eq!(pages.len(), 3, "{pages:?}");
    for (page, text) in ["Shop", "Blog", "News"].into_iter().enumerate() {
        let drawn = &pages[page];
        assert_eq!(
            drawn.iter().filter(|t| *t == "Mark").count(),
            1,
            "page {page}: {pages:?}"
        );
        assert!(drawn.iter().any(|t| t == text), "page {page}: {pages:?}");
    }
}

#[test]
fn a_blank_page_between_named_groups_has_the_outer_size() {
    let laid = lay(&format!(
        "{TEN_LINES}<style>
        @page a {{ size: 300pt 300pt }}
        @page b {{ size: 500pt 300pt }}
        </style>
        <div style='page: a'>Shop</div><div style='page: b; break-before: right'>Blog</div>"
    ));
    // The first page is a right page, so a blank left page comes before `Blog`.
    let pages = lines(&laid);
    assert_eq!(pages.len(), 3, "{pages:?}");
    assert_eq!(pages[0], ["Shop"]);
    assert!(pages[1].is_empty(), "{pages:?}");
    assert_eq!(pages[2], ["Blog"]);
    assert!(laid.pages[1].blank);
    let names: Vec<Option<&str>> = laid.pages.iter().map(|p| p.name.as_deref()).collect();
    assert_eq!(names, [Some("a"), None, Some("b")]);
    // The blank page takes the size of the unnamed `@page`.
    let widths: Vec<f32> = laid.pages.iter().map(|p| p.geometry.width).collect();
    for (width, want) in widths.iter().zip([300.0, 400.0, 500.0]) {
        assert!(close(*width, want), "{widths:?}");
    }
}
