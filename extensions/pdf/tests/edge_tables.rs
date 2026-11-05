//! Edge cases of tables on the host: a header of several rows, empty and zero-width tables,
//! fixed and over-full column widths, `rowspan` alignment, nested tables, floats in cells and
//! links in repeated rows (`rule:core-classes/pdf-tables-across-pages`).

use nvs_pdf::layout::{self, Href, Item, Laid, Paint, bounds};
use nvs_pdf::{Document, assets, css, fonts, parse, render};

fn lay(html: &str) -> Laid {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(&[]);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    layout::layout(&dom, &styles, &assets, fonts).unwrap()
}

fn rendered(html: &str) -> Vec<u8> {
    render(&Document {
        html: html.to_owned(),
        ..Document::default()
    })
    .unwrap()
    .data
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

/// Each text run on `page`, top to bottom: its text, its pen start and its baseline.
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

/// The page of each run whose text is `text`.
fn pages_of(laid: &Laid, text: &str) -> Vec<usize> {
    (0..laid.pages.len())
        .flat_map(|p| {
            texts(laid, p)
                .into_iter()
                .filter(|(t, ..)| t == text)
                .map(move |_| p)
        })
        .collect()
}

/// The pen start and the baseline of the run whose text is `text` on `page`.
fn at(laid: &Laid, page: usize, text: &str) -> (f32, f32) {
    let runs = texts(laid, page);
    let run = runs.iter().find(|(t, ..)| t == text);
    let (_, x, y) = run.unwrap_or_else(|| panic!("no {text} on page {page}: {runs:?}"));
    (*x, *y)
}

/// The left, top, right and bottom of each filled shape on `page` in the colour `rgb`.
fn shapes(laid: &Laid, page: usize, rgb: (u8, u8, u8)) -> Vec<[f32; 4]> {
    laid.pages[page]
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
        .collect()
}

/// The top and bottom of each filled shape on `page` in the colour `rgb` that is wider than it
/// is tall, top to bottom.
fn edges(laid: &Laid, page: usize, rgb: (u8, u8, u8)) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = shapes(laid, page, rgb)
        .into_iter()
        .filter(|[l, t, r, b]| r - l > b - t)
        .map(|[_, t, _, b]| (t, b))
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The highest and the lowest baseline on `page` of a run starting with `prefix`.
fn span(laid: &Laid, page: usize, prefix: &str) -> (f32, f32) {
    let ys: Vec<f32> = texts(laid, page)
        .into_iter()
        .filter(|(t, ..)| t.starts_with(prefix))
        .map(|r| r.2)
        .collect();
    assert!(!ys.is_empty(), "no {prefix} on page {page}");
    (
        ys.iter().copied().fold(f32::MAX, f32::min),
        ys.iter().copied().fold(f32::MIN, f32::max),
    )
}

fn body_rows(count: usize, cells: impl Fn(usize) -> String) -> String {
    (1..=count)
        .map(|n| format!("<tr>{}</tr>", cells(n)))
        .collect()
}

/// An A6 page with a 10mm margin, so the content box is 85mm, about 241pt, wide.
const PAGE: &str = "<style>
    @page { size: 105mm 148mm; margin: 10mm }
    body { margin: 0; font-size: 10pt; line-height: 12pt }
    td, th { padding: 2pt; text-align: left }
    </style>";

const RED: (u8, u8, u8) = (255, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

#[test]
fn a_multi_row_thead_repeats_whole() {
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td><td>4.00</td>"));
    let laid = lay(&format!(
        "{PAGE}<table><thead><tr><th>Shop</th><th>Blog</th></tr>
        <tr><th>Item</th><th>Price</th></tr></thead><tbody>{rows}</tbody></table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    let every: Vec<usize> = (0..pages).collect();
    for word in ["Shop", "Blog", "Item", "Price"] {
        assert_eq!(pages_of(&laid, word), every, "{word}");
    }
    // On each page the two header rows are on top, in source order, above every body row.
    for p in 0..pages {
        let runs = texts(&laid, p);
        let first: Vec<&str> = runs[..4].iter().map(|r| r.0.as_str()).collect();
        assert_eq!(first, ["Shop", "Blog", "Item", "Price"], "page {p}");
        let (high, _) = span(&laid, p, "Tea");
        assert!(
            runs[0].2 < runs[2].2 && runs[2].2 < high,
            "page {p}: {runs:?}"
        );
    }
    for n in 1..=60 {
        assert_eq!(pages_of(&laid, &format!("Tea{n}")).len(), 1, "Tea{n}");
    }
}

#[test]
fn degenerate_tables_render_without_a_panic() {
    let cases = [
        "<table></table>",
        "<table><tr></tr></table>",
        "<table><tbody><tr></tr><tr></tr></tbody></table>",
        "<table style='width: 0'><tr><td>Shop</td><td>Blog</td></tr></table>",
        "<table style='width: 0; table-layout: fixed'><tr><td>Shop</td></tr></table>",
        "<table><thead><tr></tr></thead><tfoot></tfoot></table>",
    ];
    for case in cases {
        let html = format!("{PAGE}<p>Before</p>{case}<p>After</p>");
        let laid = lay(&html);
        // The paragraphs around the table are drawn on the first page, in order.
        assert_eq!(laid.pages.len(), 1, "{case}");
        let (_, before) = at(&laid, 0, "Before");
        let (_, after) = at(&laid, 0, "After");
        assert!(before < after, "{case}");
        let data = rendered(&html);
        assert!(data.starts_with(b"%PDF-"), "{case}");
    }
    // The text of a zero-width table's cells is still drawn.
    let laid = lay(&format!(
        "{PAGE}<table style='width: 0'><tr><td>Shop</td><td>Blog</td></tr></table>"
    ));
    assert_eq!(pages_of(&laid, "Shop"), vec![0]);
    assert_eq!(pages_of(&laid, "Blog"), vec![0]);
}

#[test]
fn fixed_table_layout_and_percentages_over_a_hundred_share_the_width() {
    // Under `table-layout: fixed`, the first row sets the first column to 50pt, and the other
    // two share the 150pt left. The long word in the second row does not widen its column.
    let laid = lay(&format!(
        "{PAGE}<style>td {{ padding: 0 }}</style>
        <table style='table-layout: fixed; width: 200pt; border-spacing: 0'>
        <tr><td style='width: 50pt'>A</td><td>B</td><td>C</td></tr>
        <tr><td>D</td><td>Supercalifragilisticexpialidocious</td><td>F</td></tr></table>"
    ));
    let (a, _) = at(&laid, 0, "A");
    let (b, _) = at(&laid, 0, "B");
    let (c, _) = at(&laid, 0, "C");
    assert!((b - a - 50.0).abs() < 0.01, "{a} {b}");
    assert!((c - b - 75.0).abs() < 0.01, "{b} {c}");
    let (f, _) = at(&laid, 0, "F");
    assert!((f - c).abs() < 0.01, "{c} {f}");

    // Two cells of 60% each ask for more than the table has, and they share its width.
    let laid = lay(&format!(
        "{PAGE}<table style='width: 200pt; border-spacing: 0; border: 1pt solid red'>
        <tr><td style='width: 60%; background-color: blue'>Shop</td>
        <td style='width: 60%; background-color: blue'>Blog</td></tr></table>"
    ));
    let table = shapes(&laid, 0, RED);
    let left = table.iter().map(|s| s[0]).fold(f32::MAX, f32::min);
    let right = table.iter().map(|s| s[2]).fold(f32::MIN, f32::max);
    assert!((right - left - 202.0).abs() < 0.01, "{table:?}");
    let mut cells = shapes(&laid, 0, BLUE);
    cells.sort_by(|a, b| a[0].total_cmp(&b[0]));
    assert_eq!(cells.len(), 2, "{cells:?}");
    // The cells lie inside the table's border, side by side and the same width.
    assert!(cells[0][0] >= left + 1.0 - 0.01, "{cells:?} {left}");
    assert!(cells[1][2] <= right - 1.0 + 0.01, "{cells:?} {right}");
    assert!(cells[0][2] <= cells[1][0] + 0.01, "{cells:?}");
    // Each column shrinks from its 120pt toward its text's width, so they are nearly equal.
    let (one, two) = (cells[0][2] - cells[0][0], cells[1][2] - cells[1][0]);
    assert!(
        one < 120.0 && two < 120.0 && (one - two).abs() < 2.0,
        "{cells:?}"
    );
    assert!((one + two - 200.0).abs() < 0.5, "{cells:?}");
}

#[test]
fn a_rowspan_cell_aligns_its_content_by_vertical_align() {
    let table = |align: &str| {
        lay(&format!(
            "{PAGE}<style>td {{ padding: 0 }}</style><table style='border-spacing: 0'>
            <tr><td>Line1<br>Line2</td><td rowspan='2' style='vertical-align: {align}'>Mark</td></tr>
            <tr><td>Line3<br>Line4</td></tr></table>"
        ))
    };
    let lines = |laid: &Laid| [1, 2, 3, 4].map(|n| at(laid, 0, &format!("Line{n}")).1);
    let laid = table("top");
    assert!((at(&laid, 0, "Mark").1 - lines(&laid)[0]).abs() < 0.01);
    // The two rows are 48pt together, and the middle of that is between `Line2` and `Line3`.
    let laid = table("middle");
    let [_, two, three, _] = lines(&laid);
    let mark = at(&laid, 0, "Mark").1;
    assert!(
        (mark - (two + three) / 2.0).abs() < 0.5,
        "{mark} {two} {three}"
    );
    let laid = table("bottom");
    let mark = at(&laid, 0, "Mark").1;
    assert!(
        (mark - lines(&laid)[3]).abs() < 0.01,
        "{mark} {:?}",
        lines(&laid)
    );
}

#[test]
fn a_nested_table_splits_across_pages_with_its_own_borders() {
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td>"));
    let laid = lay(&format!(
        "{PAGE}<table><tr><td>Shop</td>
        <td><table style='border: 2pt solid red'>{rows}</table></td></tr></table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages: {:?}", texts(&laid, 0));
    // Every inner row is drawn once, in order, from the first page on.
    assert_eq!(pages_of(&laid, "Tea1"), vec![0]);
    let mut previous = (0, f32::MIN);
    for n in 1..=60 {
        let on = pages_of(&laid, &format!("Tea{n}"));
        assert_eq!(on.len(), 1, "Tea{n}: {on:?}");
        let (_, y) = at(&laid, on[0], &format!("Tea{n}"));
        assert!((on[0], y) > previous, "Tea{n} on {} at {y}", on[0]);
        previous = (on[0], y);
    }
    // The inner table's border has a top and a bottom edge on every page, around its rows.
    for p in 0..pages {
        let on = edges(&laid, p, RED);
        let (high, low) = span(&laid, p, "Tea");
        assert_eq!(on.len(), 2, "page {p}: {on:?}");
        assert!(
            on[0].1 < high - 6.0 && on[1].0 > low,
            "page {p}: {on:?} {high} {low}"
        );
    }
}

#[test]
fn a_float_inside_a_cell_stays_inside_the_cell() {
    let laid = lay(&format!(
        "{PAGE}<table><tr><td>Shop</td>
        <td style='background-color: blue'><div style='float: left; width: 30pt; height: 40pt;
        background-color: red'></div>Blog</td></tr>
        <tr><td>Next</td><td>Row</td></tr></table>"
    ));
    let cell = shapes(&laid, 0, BLUE);
    let float = shapes(&laid, 0, RED);
    assert_eq!((cell.len(), float.len()), (1, 1), "{cell:?} {float:?}");
    let ([cl, ct, cr, cb], [fl, ft, fr, fb]) = (cell[0], float[0]);
    assert!(
        cl <= fl + 0.01 && fr <= cr + 0.01 && ct <= ft + 0.01 && fb <= cb + 0.01,
        "{cell:?} {float:?}"
    );
    // The float is 40pt tall, so the cell grows to hold it, and the next row starts under it.
    assert!((fb - ft - 40.0).abs() < 0.01, "{float:?}");
    let (blog, _) = at(&laid, 0, "Blog");
    assert!(blog >= fr - 0.01, "{blog} {float:?}");
    let (_, next) = at(&laid, 0, "Next");
    assert!(next > cb, "{next} {cell:?}");
}

#[test]
fn a_link_in_a_repeated_header_is_a_link_on_every_page() {
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td>"));
    let html = format!(
        "{PAGE}<table><thead><tr><th><a href='https://example.com/'>Shop</a></th></tr></thead>
        <tbody>{rows}</tbody></table>"
    );
    let laid = lay(&html);
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    let want = Href::Uri("https://example.com/".to_owned());
    for p in 0..pages {
        let links: Vec<(f32, f32)> = laid.pages[p]
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Link {
                    href, y, height, ..
                } if *href == want => Some((*y, *height)),
                _ => None,
            })
            .collect();
        assert_eq!(links.len(), 1, "page {p}: {links:?}");
        // The link box covers the header's text on that page.
        let (_, baseline) = at(&laid, p, "Shop");
        let (y, height) = links[0];
        assert!(
            y < baseline && baseline < y + height,
            "page {p}: {links:?} {baseline}"
        );
    }
    let data = rendered(&html);
    assert_eq!(count(&data, b"/Subtype/Link"), pages);
    assert_eq!(count(&data, b"/URI(https://example.com/)"), pages);
}
