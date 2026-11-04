//! No part of a render grows faster than the document: each test lays out a document and one
//! ten times its size, and compares the work each did, never the time
//! (`rule:core-classes/pdf-tables-across-pages`, `rule:core-classes/pdf-css-subset`).

use nvs_pdf::layout::{self, Item, Laid};
use nvs_pdf::{Asset, assets, css, fonts, parse};

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

/// Whether `big`, the work of a document ten times the size of the one that took `small`, is no
/// more than ten times `small` and a quarter more. The smaller document's last page is a larger
/// part of it, so its work per unit is a little lower. Work that grows with the square of the
/// document is a hundred times `small`.
fn linear(small: usize, big: usize) -> bool {
    small > 0 && big * 10 <= small * 125
}

#[test]
fn a_table_ten_times_longer_costs_ten_times_the_work() {
    let table = |rows: usize| {
        let body: String = (0..rows)
            .map(|n| format!("<tr><td>Shop{n}</td><td>4.00</td></tr>"))
            .collect();
        lay(&format!(
            "<style>@page {{ size: 105mm 148mm; margin: 10mm }} body {{ font-size: 10pt }}</style>\
             <table><thead><tr><th>Item</th><th>Price</th></tr></thead><tbody>{body}</tbody>\
             <tfoot><tr><td>Total</td><td>-</td></tr></tfoot></table>"
        ))
    };
    let (small, big) = (table(100), table(1000));
    // Both tables run over several pages, so the work of paging them is counted.
    let pages = (small.pages.len(), big.pages.len());
    assert!(pages.0 > 2 && pages.1 > 8 * pages.0, "{pages:?}");
    let (small, big) = (small.work.table_reads, big.work.table_reads);
    assert!(
        linear(small, big),
        "{small} reads for 100 rows, {big} for 1000"
    );
}

#[test]
fn a_string_set_on_every_page_costs_the_same_per_page_at_ten_times_the_pages() {
    let pages = |count: usize| {
        let body: String = (0..count)
            .map(|n| format!("<h2>Blog{n}</h2><p data-mark=\"post\">Lorem ipsum.</p>"))
            .collect();
        lay(&format!(
            "<style>@page {{ size: A6; margin: 20mm;
               @top-center {{ content: string(title) }}
               @top-right {{ content: element(head) }}
               @bottom-center {{ content: counter(section) }} }}
             h2 {{ break-before: page; string-set: title content(); counter-increment: section }}
             .head {{ position: running(head) }}</style>\
             <div class=\"head\">Shop</div>{body}"
        ))
    };
    let (small, big) = (pages(20), pages(200));
    assert!(small.pages.len() >= 20 && big.pages.len() >= 200);
    assert_eq!(
        big.pages[150].strings,
        [("title".to_owned(), "Blog150".to_owned())]
    );
    let (small, big) = (small.work.margin_reads, big.work.margin_reads);
    assert!(
        linear(small, big),
        "{small} reads for 20 pages, {big} for 200"
    );
}

#[test]
fn one_image_used_a_thousand_times_is_resolved_and_decoded_once() {
    let logo = [Asset {
        name: "logo.png".to_owned(),
        data: png(),
    }];
    let html = "<img src=\"logo.png\" width=\"10\"> \
                <div style=\"background-image: url(logo.png); height: 4pt\"></div>"
        .repeat(500);
    let laid = lay_with(&html, &logo);
    let drawn = laid
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .filter(|item| matches!(item, Item::Image { .. }))
        .count();
    assert_eq!(drawn, 500);
    assert_eq!(laid.images.len(), 1);
    assert_eq!((laid.work.image_resolves, laid.work.image_decodes), (1, 1));
}
