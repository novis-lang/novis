//! Tables across pages on the host: the same columns on every page, repeated header and footer
//! rows, rows kept whole, borders closed at each break and the caption once
//! (`rule:core-classes/pdf-tables-across-pages`).

use nvs_pdf::layout::{self, Item, Laid, Paint, bounds};
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

/// The top and bottom of each solid edge on `page` in the colour `rgb` that is wider than it is
/// tall, top to bottom.
fn edges(laid: &Laid, page: usize, rgb: (u8, u8, u8)) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = laid.pages[page]
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Path {
                path,
                paint: Paint::Color(c),
                stroke: None,
            } if (c.r, c.g, c.b) == rgb => {
                let (left, top, right, bottom) = bounds(path);
                (right - left > bottom - top).then_some((top, bottom))
            }
            _ => None,
        })
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
    (ys.iter().copied().fold(f32::MAX, f32::min), ys.iter().copied().fold(f32::MIN, f32::max))
}

const RED: (u8, u8, u8) = (255, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

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
fn borders_are_closed_at_every_break() {
    // The table's own border: a top and a bottom edge on every page, around its rows.
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td>"));
    let laid = lay(&format!(
        "{PAGE}<table style='border: 2pt solid red'>{rows}</table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    for p in 0..pages {
        let on = edges(&laid, p, RED);
        let (high, low) = span(&laid, p, "Tea");
        assert_eq!(on.len(), 2, "page {p}: {on:?}");
        assert!(on[0].1 < high - 6.0 && on[1].0 > low, "page {p}: {on:?} {high} {low}");
        assert!((on[0].1 - on[0].0 - 2.0).abs() < 0.01, "page {p}: {on:?}");
    }
    // Every row is still drawn once.
    for n in 1..=60 {
        assert_eq!(pages_of(&laid, &format!("Tea{n}")).len(), 1, "Tea{n}");
    }

    // Collapsed cell borders: rows that meet share one edge, and the first and last row on
    // each page have theirs.
    let laid = lay(&format!(
        "{PAGE}<style>td {{ border: 1pt solid blue }}</style>
        <table style='border-collapse: collapse; border: 1pt solid red'>{rows}</table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    for p in 0..pages {
        let count = texts(&laid, p)
            .iter()
            .filter(|(t, ..)| t.starts_with("Tea"))
            .count();
        let mut lines: Vec<f32> = edges(&laid, p, BLUE).iter().map(|e| e.0).collect();
        lines.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        assert_eq!(lines.len(), count + 1, "page {p}: {lines:?}");
        let (high, low) = span(&laid, p, "Tea");
        assert!(lines[0] < high - 6.0 && lines[count] > low, "page {p}: {lines:?}");
        // The table's edges lie on the outer rows' edges.
        let red = edges(&laid, p, RED);
        assert_eq!(red.len(), 2, "page {p}: {red:?}");
        assert!((red[0].0 - lines[0]).abs() < 0.01, "page {p}: {red:?} {lines:?}");
        assert!((red[1].0 - lines[count]).abs() < 0.01, "page {p}: {red:?} {lines:?}");
    }

    // A cell of a row taller than a page is closed on every page it is on.
    let lines = (1..=80)
        .map(|n| format!("Shop{n}"))
        .collect::<Vec<_>>()
        .join("<br>");
    let laid = lay(&format!(
        "{PAGE}<style>td {{ border: 1pt solid blue }}</style>
        <table><tr><td>{lines}</td></tr></table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 2, "{pages} pages");
    for p in 0..pages {
        let on = edges(&laid, p, BLUE);
        let (high, low) = span(&laid, p, "Shop");
        assert_eq!(on.len(), 2, "page {p}: {on:?}");
        assert!(on[0].1 < high - 6.0 && on[1].0 > low, "page {p}: {on:?}");
    }
}

#[test]
fn slice_restores_the_open_edge() {
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td>"));
    let laid = lay(&format!(
        "{PAGE}<table style='border: 2pt solid red; box-decoration-break: slice'>{rows}</table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    // The top edge is on the first page only and the bottom edge on the last.
    let (high, _) = span(&laid, 0, "Tea");
    let first = edges(&laid, 0, RED);
    assert_eq!(first.len(), 1, "{first:?}");
    assert!(first[0].1 < high, "{first:?}");
    for p in 1..pages - 1 {
        assert_eq!(edges(&laid, p, RED), vec![], "page {p}");
    }
    let (_, low) = span(&laid, pages - 1, "Tea");
    let last = edges(&laid, pages - 1, RED);
    assert_eq!(last.len(), 1, "{last:?}");
    assert!(last[0].0 > low, "{last:?}");
    // A sliced table takes no more pages than a closed one.
    let closed = lay(&format!(
        "{PAGE}<table style='border: 2pt solid red'>{rows}</table>"
    ));
    assert!(pages <= closed.pages.len());
}

#[test]
fn a_slot_in_a_continued_row_is_filled_without_moving_anything() {
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td><td>4.00</td>"));
    let html = format!(
        "{PAGE}<table><thead><tr><th>Item</th><th>Price</th></tr></thead><tbody>{rows}</tbody>
        <tfoot><tr data-repeat='continued'><td>Carried</td><td><slot name='carried'></slot></td></tr>
        <tr data-repeat='last'><td>Total</td><td><slot name='total'></slot></td></tr></tfoot></table>"
    );
    let document = nvs_pdf::Document {
        html,
        ..Default::default()
    };
    let (kept, _) = nvs_pdf::first(&document, &nvs_pdf::Heights::default()).unwrap();
    let pages = kept.laid().pages.len();
    assert!(pages >= 3, "{pages} pages");
    // Each slot is on every page its row is on.
    for p in 0..pages {
        let names: Vec<&str> = kept.laid().pages[p]
            .slots
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        let want = if p + 1 == pages { "total" } else { "carried" };
        assert_eq!(names, vec![want], "page {p}");
    }
    let before: Vec<_> = (0..pages).map(|p| texts(kept.laid(), p)).collect();
    let fill = |text: &str| format!("<style>body {{ font-size: 10pt }}</style>{text}");
    let long = "A total far too long for its cell, so it wraps onto more lines than the row has";
    let markup: Vec<nvs_pdf::Markup> = (0..pages)
        .map(|p| nvs_pdf::Markup {
            slots: vec![
                ("carried".to_owned(), fill(&format!("{}.00", (p + 1) * 100))),
                ("total".to_owned(), fill(long)),
            ],
            ..Default::default()
        })
        .collect();
    let (kept, overflows) = nvs_pdf::draw(kept, &markup).unwrap();
    let laid = kept.laid();
    for (p, before) in before.iter().enumerate() {
        let after = texts(laid, p);
        // Everything drawn before is drawn where it was.
        for run in before {
            assert!(after.contains(run), "page {p}: {run:?} moved");
        }
        if p + 1 < pages {
            let at = |text: &str| after.iter().find(|(t, ..)| t == text).cloned();
            let (_, x, y) = at(&format!("{}.00", (p + 1) * 100))
                .unwrap_or_else(|| panic!("page {p}: no fill in {after:?}"));
            let (_, label, line) = at("Carried").expect("the carried row");
            assert!((y - line).abs() < 1.0 && x > label, "page {p}: {after:?}");
        }
    }
    // The long total is drawn in its cell and listed as an overflow.
    assert_eq!(overflows.len(), 1, "{overflows:?}");
    assert_eq!(overflows[0].area, "total");
    assert_eq!(overflows[0].page, pages as u64);
}

/// One row with a `note` slot in a table as wide as the page, laid out by the first pass, and
/// the slot's area on the page.
fn one_slot() -> (nvs_pdf::Kept, [f32; 4]) {
    let document = nvs_pdf::Document {
        html: "<table style='width: 100%'><tbody><tr><td>Delivery</td><td><slot name='note'></slot></td></tr></tbody></table>"
            .to_owned(),
        ..Default::default()
    };
    let (kept, _) = nvs_pdf::first(&document, &nvs_pdf::Heights::default()).unwrap();
    let area = kept.laid().pages[0].slots[0].1;
    (kept, area)
}

/// `kept` with `fill` drawn in its `note` slot on its one page.
fn fill_note(kept: nvs_pdf::Kept, fill: &str) -> (nvs_pdf::Kept, Vec<nvs_pdf::Overflow>) {
    let markup = vec![nvs_pdf::Markup {
        slots: vec![("note".to_owned(), fill.to_owned())],
        ..Default::default()
    }];
    nvs_pdf::draw(kept, &markup).unwrap()
}

#[test]
fn a_plain_fill_starts_at_the_top_left_of_its_cell() {
    let (kept, area) = one_slot();
    let (kept, overflows) = fill_note(kept, "42");
    let after = texts(kept.laid(), 0);
    let at = |text: &str| after.iter().find(|(t, ..)| t == text).cloned();
    let (_, x, y) = at("42").unwrap_or_else(|| panic!("no fill in {after:?}"));
    let (_, _, line) = at("Delivery").expect("the row's label");
    // The fill's body has no margin: its text starts at the content box's left edge, on the
    // same baseline as the label beside it, and fits the box.
    assert!((x - area[0]).abs() < 0.5, "{x} against {area:?}");
    assert!((y - line).abs() < 0.5, "{y} against {line}");
    assert!(overflows.is_empty(), "{overflows:?}");
}

#[test]
fn a_fill_keeps_the_margin_its_own_style_gives_its_body() {
    let (kept, area) = one_slot();
    let (kept, _) = fill_note(kept, "<style>body { margin-left: 5pt }</style>42");
    let after = texts(kept.laid(), 0);
    let (_, x, _) = after.iter().find(|(t, ..)| t == "42").cloned().unwrap();
    assert!((x - area[0] - 5.0).abs() < 0.5, "{x} against {area:?}");
}

#[test]
fn a_single_line_taller_than_its_cell_is_listed_as_an_overflow() {
    let (kept, _) = one_slot();
    let (kept, overflows) = fill_note(kept, "<span style='font-size: 40pt'>42</span>");
    // The one line cannot go on to a second page, so it stays and is cut off at the cell.
    assert!(texts(kept.laid(), 0).iter().any(|(t, ..)| t == "42"));
    assert_eq!(overflows.len(), 1, "{overflows:?}");
    assert_eq!(overflows[0].area, "note");
    assert_eq!(overflows[0].page, 1);
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

/// The left, top, right and bottom of each filled shape on `page` in the colour `rgb`, and its
/// index in the page's items, in the order they are drawn.
fn shapes(laid: &Laid, page: usize, rgb: (u8, u8, u8)) -> Vec<(usize, [f32; 4])> {
    laid.pages[page]
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match item {
            Item::Path {
                path,
                paint: Paint::Color(c),
                stroke: None,
            } if (c.r, c.g, c.b) == rgb => {
                let (left, top, right, bottom) = bounds(path);
                Some((i, [left, top, right, bottom]))
            }
            _ => None,
        })
        .collect()
}

/// The index in `page`'s items of the first text run starting with `prefix`.
fn text_index(laid: &Laid, page: usize, prefix: &str) -> usize {
    laid.pages[page]
        .items
        .iter()
        .position(|item| matches!(item, Item::Text { text, .. } if text.trim().starts_with(prefix)))
        .unwrap_or_else(|| panic!("no {prefix} on page {page}"))
}

/// The pen start and the baseline of the run whose text is `text` on `page`.
fn at(laid: &Laid, page: usize, text: &str) -> (f32, f32) {
    let runs = texts(laid, page);
    let run = runs.iter().find(|(t, ..)| t == text);
    let (_, x, y) = run.unwrap_or_else(|| panic!("no {text} on page {page}: {runs:?}"));
    (*x, *y)
}

#[test]
fn a_rowspan_piece_taller_than_a_page_splits_between_lines() {
    let lines = (1..=80)
        .map(|n| format!("Shop{n}"))
        .collect::<Vec<_>>()
        .join("<br>");
    let laid = lay(&format!(
        "{PAGE}<table><tr><td>Before</td><td>One</td></tr>
        <tr><td rowspan='2'>{lines}</td><td>Two</td></tr>
        <tr><td>Three</td></tr>
        <tr><td>After</td><td>End</td></tr></table>"
    ));
    let pages = laid.pages.len();
    assert!(pages >= 3, "{pages} pages");
    // The piece starts under the row before it, and every line is drawn once, in order.
    assert_eq!(pages_of(&laid, "Shop1"), vec![0]);
    let mut previous = (0, f32::MIN);
    for n in 1..=80 {
        let on = pages_of(&laid, &format!("Shop{n}"));
        assert_eq!(on.len(), 1, "Shop{n}: {on:?}");
        let (_, y) = at(&laid, on[0], &format!("Shop{n}"));
        assert!((on[0], y) > previous, "Shop{n} on {} at {y}", on[0]);
        previous = (on[0], y);
    }
    // The rows beside the tall cell keep their places, and the next row follows it.
    let (_, one) = at(&laid, 0, "One");
    let (_, two) = at(&laid, 0, "Two");
    let (_, three) = at(&laid, 0, "Three");
    assert!(one < two && two < three, "{one} {two} {three}");
    assert_eq!(pages_of(&laid, "After"), vec![pages - 1]);
    let (_, after) = at(&laid, pages - 1, "After");
    assert!(after > previous.1, "{after} against {previous:?}");
}

#[test]
fn a_table_without_a_width_is_as_wide_as_its_columns() {
    let width = |table: &str, cells: &str| {
        let laid = lay(&format!(
            "{PAGE}<table style='border: 1pt solid red; {table}'><tr>{cells}</tr></table>"
        ));
        let top = shapes(&laid, 0, RED)
            .into_iter()
            .map(|(_, [l, t, r, b])| (r - l, b - t))
            .find(|(w, h)| w > h)
            .expect("a top edge");
        top.0
    };
    // The page's content box is 85mm, about 241pt, wide.
    let short = width("", "<td>Tea</td><td>4.00</td>");
    assert!(short > 20.0 && short < 80.0, "{short}");
    let full = width("width: 100%", "<td>Tea</td><td>4.00</td>");
    assert!((full - 243.0).abs() < 1.0, "{full}");
    // Content wider than the page makes the table as wide as its containing block.
    let long = "A sentence that is much longer than the page is wide, so it has to wrap ".repeat(3);
    let wide = width("", &format!("<td>{long}</td><td>4.00</td>"));
    assert!((wide - 241.0).abs() < 1.0, "{wide}");
    // A fixed layout needs a width, so with none it is laid out as an automatic one.
    let fixed = width("table-layout: fixed", "<td>Tea</td><td>4.00</td>");
    assert!((fixed - short).abs() < 0.01, "{fixed} against {short}");
}

#[test]
fn a_cells_vertical_align_places_its_content() {
    let row = |align: &str| {
        lay(&format!(
            "{PAGE}<table><tr><td>Line1<br>Line2<br>Line3<br>Line4</td>
            <td style='vertical-align: {align}'>Mark</td></tr></table>"
        ))
    };
    let lines = |laid: &Laid| [1, 2, 3, 4].map(|n| at(laid, 0, &format!("Line{n}")).1);
    let laid = row("top");
    assert!((at(&laid, 0, "Mark").1 - lines(&laid)[0]).abs() < 0.01);
    let laid = row("bottom");
    assert!((at(&laid, 0, "Mark").1 - lines(&laid)[3]).abs() < 0.01);
    let laid = row("middle");
    let [_, two, three, _] = lines(&laid);
    let mark = at(&laid, 0, "Mark").1;
    // The lines' own heights differ by a fraction of a point.
    assert!((mark - (two + three) / 2.0).abs() < 0.5, "{mark} {two} {three}");
    // With no `vertical-align`, the content is at the top.
    let laid = row("inherit");
    assert!((at(&laid, 0, "Mark").1 - lines(&laid)[0]).abs() < 0.01);
    // `baseline` puts the first lines of the row's cells on one baseline.
    let laid = lay(&format!(
        "{PAGE}<table><tr><td style='vertical-align: baseline; font-size: 24pt'>Big</td>
        <td style='vertical-align: baseline'>Small<br>Next</td></tr></table>"
    ));
    let (big, small) = (at(&laid, 0, "Big").1, at(&laid, 0, "Small").1);
    assert!((big - small).abs() < 0.01, "{big} {small}");
}

#[test]
fn a_rows_and_a_groups_background_is_drawn() {
    let laid = lay(&format!(
        "{PAGE}<table><tbody style='background-color: blue'>
        <tr style='background-color: red'><td>Tea</td><td>4.00</td></tr>
        <tr><td>Two</td><td>5.00</td></tr></tbody></table>"
    ));
    let (_, tea) = at(&laid, 0, "Tea");
    let (_, two) = at(&laid, 0, "Two");
    let red = shapes(&laid, 0, RED);
    assert_eq!(red.len(), 1, "{red:?}");
    let (row, [_, top, _, bottom]) = red[0];
    assert!(top < tea && tea < bottom && bottom < two, "{red:?} {tea} {two}");
    let blue = shapes(&laid, 0, BLUE);
    assert_eq!(blue.len(), 1, "{blue:?}");
    let (group, [left, top, right, bottom]) = blue[0];
    assert!(top < tea && two < bottom, "{blue:?} {tea} {two}");
    // The group's box spans both columns, and is drawn under the row's, which is under the text.
    let (price, _) = at(&laid, 0, "4.00");
    assert!(left < price && price < right, "{blue:?} {price}");
    assert!(group < row && row < text_index(&laid, 0, "Tea"), "{group} {row}");

    // On each page a group is on, its box is drawn once, under its rows there.
    let rows = body_rows(60, |n| format!("<td>Tea{n}</td>"));
    let laid = lay(&format!(
        "{PAGE}<table><tbody style='background-color: blue'>{rows}</tbody></table>"
    ));
    assert!(laid.pages.len() >= 2, "{} pages", laid.pages.len());
    for p in 0..laid.pages.len() {
        let blue = shapes(&laid, p, BLUE);
        assert_eq!(blue.len(), 1, "page {p}: {blue:?}");
        let (high, low) = span(&laid, p, "Tea");
        assert!(blue[0].1[1] < high && low < blue[0].1[3], "page {p}: {blue:?}");
    }
}

#[test]
fn collapsed_row_and_group_borders_are_drawn() {
    let table = |collapse: &str| {
        lay(&format!(
            "{PAGE}<table style='border-collapse: {collapse}'>
            <tbody style='border: 1pt solid blue'>
            <tr style='border-bottom: 1pt solid red'><td>Tea</td></tr>
            <tr><td>Two</td></tr></tbody></table>"
        ))
    };
    let laid = table("collapse");
    let (_, tea) = at(&laid, 0, "Tea");
    let (_, two) = at(&laid, 0, "Two");
    // The row's bottom border is between its text and the next row's.
    let red = edges(&laid, 0, RED);
    assert_eq!(red.len(), 1, "{red:?}");
    assert!(tea < red[0].0 && red[0].1 < two, "{red:?} {tea} {two}");
    // The group's border is above its first row and under its last.
    let blue = edges(&laid, 0, BLUE);
    assert_eq!(blue.len(), 2, "{blue:?}");
    assert!(blue[0].1 < tea && two < blue[1].0, "{blue:?} {tea} {two}");
    // Separate borders are the cells' alone, as CSS 2.1 says.
    let laid = table("separate");
    assert_eq!(edges(&laid, 0, RED), vec![]);
    assert_eq!(edges(&laid, 0, BLUE), vec![]);
}

#[test]
fn a_table_directly_in_a_row_is_wrapped_in_a_cell() {
    let laid = lay(&format!(
        "{PAGE}<div style='display: table'><div style='display: table-row'>
        <div style='display: table-cell'>Left</div>
        <div style='display: table'><div style='display: table-row'>
        <div style='display: table-cell'>Inner1</div>
        <div style='display: table-cell'>Inner2</div></div></div>
        </div></div>"
    ));
    let (left, line) = at(&laid, 0, "Left");
    let (one, first) = at(&laid, 0, "Inner1");
    let (two, second) = at(&laid, 0, "Inner2");
    // The inner table is a cell beside `Left`, and its two cells are in one row.
    assert!(left < one && one < two, "{left} {one} {two}");
    assert!((first - second).abs() < 0.01 && (first - line).abs() < 0.01, "{line} {first} {second}");
}
