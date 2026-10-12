//! Tables across pages on the host: the same columns on every page, repeated header and footer
//! rows, rows kept whole and the caption once (`rule:core-classes/pdf-tables-across-pages`).

use nvs_pdf::layout::{self, Item, Laid};
use nvs_pdf::{assets, css, fonts, parse};

fn lay(html: &str) -> Laid {
    let dom = parse::parse(html);
    let assets = assets::Assets::new(&[]);
    let styles = css::cascade(&dom, html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets).unwrap();
    layout::layout(&dom, &styles, &assets, fonts).unwrap()
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

const PAGE: &str = "<style>
    @page { size: 105mm 148mm; margin: 10mm }
    body { margin: 0; font-size: 10pt }
    td, th { padding: 2pt; text-align: left }
    </style>";

fn body_rows(count: usize, cells: impl Fn(usize) -> String) -> String {
    (1..=count)
        .map(|n| format!("<tr>{}</tr>", cells(n)))
        .collect()
}

#[test]
fn column_widths_are_the_same_on_every_page() {
    // One long name on the last page widens the first column on every page.
    let rows = body_rows(60, |n| {
        let name = if n == 60 {
            "Supercalifragilisticexpialidocious".to_owned()
        } else {
            format!("Tea{n}")
        };
        format!("<td>{name}</td><td>P{n}</td>")
    });
    let laid = lay(&format!("{PAGE}<table>{rows}</table>"));
    assert!(laid.pages.len() >= 3, "{} pages", laid.pages.len());
    let price_x: Vec<f32> = (0..laid.pages.len())
        .map(|p| {
            let runs = texts(&laid, p);
            let (_, x, _) = runs
                .iter()
                .find(|(t, ..)| t.starts_with('P'))
                .unwrap_or_else(|| panic!("no price on page {p}: {runs:?}"));
            *x
        })
        .collect();
    assert!(
        price_x.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01),
        "{price_x:?}"
    );
    let last = texts(&laid, laid.pages.len() - 1);
    let (_, name_x, _) = last
        .iter()
        .find(|(t, ..)| t.starts_with("Super"))
        .expect("the long name");
    // The price column starts right of the long name, which is about 160pt wide at 10pt.
    assert!(price_x[0] > name_x + 120.0, "{price_x:?} {name_x}");
}

#[test]
fn thead_and_tfoot_repeat_on_every_page() {
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td><td>4.00</td>"));
    let laid = lay(&format!(
        "{PAGE}<table><tfoot><tr><td>Subtotal</td><td>0</td></tr></tfoot>
        <thead><tr><th>Item</th><th>Price</th></tr></thead><tbody>{rows}</tbody></table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    assert_eq!(pages_of(&laid, "Item"), (0..pages).collect::<Vec<_>>());
    assert_eq!(pages_of(&laid, "Subtotal"), (0..pages).collect::<Vec<_>>());
    for p in 0..pages {
        let runs = texts(&laid, p);
        assert_eq!(runs[0].0, "Item", "page {p}: {runs:?}");
        assert_eq!(runs[runs.len() - 2].0, "Subtotal", "page {p}: {runs:?}");
        // The footer is under the last body row on the page.
        let body = runs.iter().filter(|(t, ..)| t.starts_with("Tea"));
        let lowest = body.map(|r| r.2).fold(0.0, f32::max);
        assert!(runs[runs.len() - 2].2 > lowest, "page {p}: {runs:?}");
    }
    // Every body row is drawn once.
    for n in 1..=60 {
        assert_eq!(pages_of(&laid, &format!("Tea{n}")).len(), 1, "Tea{n}");
    }
}

#[test]
fn data_repeat_first_last_and_continued_rows() {
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td><td>4.00</td>"));
    let laid = lay(&format!(
        "{PAGE}<table>
        <thead><tr data-repeat='first'><th>Invoice</th></tr>
        <tr data-repeat='continued'><th>Continued</th></tr>
        <tr><th>Item</th></tr></thead>
        <tbody>{rows}</tbody>
        <tfoot><tr data-repeat='continued'><td>Carried</td></tr>
        <tr data-repeat='last'><td>Total</td></tr></tfoot></table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    assert_eq!(pages_of(&laid, "Invoice"), vec![0]);
    assert_eq!(pages_of(&laid, "Continued"), (1..pages).collect::<Vec<_>>());
    assert_eq!(pages_of(&laid, "Item"), (0..pages).collect::<Vec<_>>());
    assert_eq!(
        pages_of(&laid, "Carried"),
        (0..pages - 1).collect::<Vec<_>>()
    );
    assert_eq!(pages_of(&laid, "Total"), vec![pages - 1]);
}

#[test]
fn a_row_that_fits_is_never_split() {
    let rows = body_rows(30, |n| {
        format!("<td>Shop{n}<br>Blog{n}<br>News{n}</td><td>One{n}</td>")
    });
    let laid = lay(&format!("{PAGE}<table>{rows}</table>"));
    assert!(laid.pages.len() >= 2, "{} pages", laid.pages.len());
    for n in 1..=30 {
        let on: Vec<Vec<usize>> = ["Shop", "Blog", "News", "One"]
            .iter()
            .map(|w| pages_of(&laid, &format!("{w}{n}")))
            .collect();
        assert!(
            on.iter().all(|p| p.len() == 1 && p == &on[0]),
            "row {n}: {on:?}"
        );
    }
}

#[test]
fn the_caption_is_on_the_first_page_only() {
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td>"));
    let laid = lay(&format!(
        "{PAGE}<table><caption>Prices</caption><thead><tr><th>Item</th></tr></thead>{rows}</table>"
    ));
    assert!(laid.pages.len() >= 2, "{} pages", laid.pages.len());
    assert_eq!(pages_of(&laid, "Prices"), vec![0]);
    let first = texts(&laid, 0);
    assert_eq!(first[0].0, "Prices", "{first:?}");
    assert_eq!(first[1].0, "Item", "{first:?}");
}

#[test]
fn a_header_with_no_room_for_a_row_starts_on_the_next_page() {
    let rows = body_rows(3, |n| format!("<td>Tea{n}</td>"));
    // The block above leaves room for the header row and not for the first body row.
    let laid = lay(&format!(
        "{PAGE}<div style='height: 334pt'></div>
        <table><thead><tr><th>Item</th></tr></thead>{rows}</table>"
    ));
    assert_eq!(pages_of(&laid, "Item"), vec![1]);
    assert_eq!(pages_of(&laid, "Tea1"), vec![1]);
}
