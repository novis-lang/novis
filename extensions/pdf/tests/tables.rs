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
fn a_row_taller_than_a_page_splits_between_lines() {
    let lines = |word: &str, count: usize| {
        (1..=count)
            .map(|n| format!("{word}{n}"))
            .collect::<Vec<_>>()
            .join("<br>")
    };
    let laid = lay(&format!(
        "{PAGE}<table><thead><tr><th>Item</th><th>Note</th></tr></thead>
        <tfoot><tr><td>Subtotal</td><td>0</td></tr></tfoot>
        <tr><td>Short</td><td>Before</td></tr>
        <tr><td>{}</td><td>{}</td></tr>
        <tr><td>After</td><td>End</td></tr></table>",
        lines("Shop", 80),
        lines("Blog", 40)
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    // Every line is drawn once, whole, and in order.
    let mut previous = (0, f32::MIN);
    for n in 1..=80 {
        let on = pages_of(&laid, &format!("Shop{n}"));
        assert_eq!(on.len(), 1, "Shop{n}: {on:?}");
        let y = texts(&laid, on[0])
            .into_iter()
            .find(|(t, ..)| *t == format!("Shop{n}"))
            .unwrap()
            .2;
        assert!((on[0], y) > previous, "Shop{n} on {} at {y}", on[0]);
        previous = (on[0], y);
    }
    for n in 1..=40 {
        assert_eq!(pages_of(&laid, &format!("Blog{n}")).len(), 1, "Blog{n}");
    }
    // The row starts on the first page, under the row before it.
    assert_eq!(pages_of(&laid, "Shop1"), vec![0]);
    assert_eq!(pages_of(&laid, "Blog1"), vec![0]);
    // Each page has its header row above the lines and its footer row under them.
    for p in 0..pages {
        let runs = texts(&laid, p);
        let at = |text: &str| runs.iter().find(|(t, ..)| t == text).map(|r| r.2);
        let (item, subtotal) = (at("Item"), at("Subtotal"));
        let (Some(item), Some(subtotal)) = (item, subtotal) else {
            panic!("page {p}: {runs:?}");
        };
        for (text, _, y) in &runs {
            if text.starts_with("Shop") || text.starts_with("Blog") {
                assert!(item < *y && *y < subtotal, "page {p}: {text} at {y}");
            }
        }
    }
    assert_eq!(pages_of(&laid, "After"), vec![pages - 1]);
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
fn a_header_is_never_alone_and_rows_obey_orphans_and_widows() {
    let rows = body_rows(3, |n| format!("<td>Tea{n}</td>"));
    let after = |space: &str, style: &str| {
        lay(&format!(
            "{PAGE}<div style='height: {space}'></div>
            <table style='{style}'><thead><tr><th>Item</th></tr></thead>{rows}</table>"
        ))
    };
    // The block above leaves room for the header row and not for the first body row.
    let laid = after("334pt", "");
    assert_eq!(pages_of(&laid, "Item"), vec![1]);
    assert_eq!(pages_of(&laid, "Tea1"), vec![1]);
    // Here it leaves room for one body row, and `orphans` is 2.
    let laid = after("317pt", "");
    assert_eq!(pages_of(&laid, "Tea1"), vec![1]);
    let laid = after("317pt", "orphans: 1");
    assert_eq!(pages_of(&laid, "Tea1"), vec![0]);
    assert_eq!(pages_of(&laid, "Tea2"), vec![1]);

    // The body rows on each page.
    let counts = |laid: &Laid| -> Vec<usize> {
        (0..laid.pages.len())
            .map(|p| {
                texts(laid, p)
                    .iter()
                    .filter(|(t, ..)| t.starts_with("Tea"))
                    .count()
            })
            .collect()
    };
    let table = |count: usize, style: &str| {
        let rows = body_rows(count, |n| format!("<td>Tea{n}</td>"));
        lay(&format!(
            "{PAGE}<table style='{style}'><thead><tr><th>Item</th></tr></thead>{rows}</table>"
        ))
    };
    // The first count of rows that leaves one row alone on the last page under `widows: 1`.
    let count = (10..80)
        .find(|n| counts(&table(*n, "widows: 1")).last() == Some(&1))
        .expect("a count that leaves one row on the last page");
    let on = counts(&table(count, ""));
    assert!(on.len() >= 2, "{on:?}");
    assert!(on.iter().all(|n| *n >= 2), "{count} rows: {on:?}");
    assert_eq!(on.iter().sum::<usize>(), count);
}

#[test]
fn a_tbody_and_a_rowspan_group_stay_together() {
    let page_of = |laid: &Laid, text: &str| {
        let on = pages_of(laid, text);
        assert_eq!(on.len(), 1, "{text}: {on:?}");
        on[0]
    };
    for before in 10..30 {
        // Every third row of the first group starts a cell over three rows.
        let first = body_rows(before * 3, |n| {
            if n % 3 == 1 {
                format!("<td rowspan='3'>Span{n}</td><td>Tea{n}</td>")
            } else {
                format!("<td>Tea{n}</td>")
            }
        });
        let second = body_rows(6, |n| format!("<td>Blog{n}</td><td>4.00</td>"));
        let laid = lay(&format!(
            "{PAGE}<table><tbody>{first}</tbody><tbody>{second}</tbody></table>"
        ));
        let group = page_of(&laid, "Blog1");
        for n in 2..=6 {
            assert_eq!(page_of(&laid, &format!("Blog{n}")), group, "{before}: Blog{n}");
        }
        for n in (1..=before * 3).step_by(3) {
            let span = page_of(&laid, &format!("Span{n}"));
            for m in n..n + 3 {
                assert_eq!(page_of(&laid, &format!("Tea{m}")), span, "{before}: Tea{m}");
            }
        }
    }
    // A group taller than a page splits between its rows.
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td>"));
    let laid = lay(&format!("{PAGE}<table><tbody>{rows}</tbody></table>"));
    assert!(laid.pages.len() >= 3, "{} pages", laid.pages.len());
}
