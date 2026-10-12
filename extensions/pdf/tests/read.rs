//! Reading and merging existing PDFs on the host (`rule:core-classes/pdf-reader-and-merge`):
//! the page count, the metadata, the attachments and a page's text, merging with page lists, and
//! the refusals every job shares.

use nvs_pdf::merge::{Part, merge};
use nvs_pdf::read::{read, text};
use nvs_pdf::{Asset, Document, Error, Instant, Options, Standard, render};

fn rendered(html: &str, options: Options) -> Vec<u8> {
    render(&Document {
        html: html.to_owned(),
        options,
        ..Document::default()
    })
    .unwrap()
    .data
}

/// A document of `pages` pages, each with one heading naming its number.
fn numbered(pages: u64) -> Vec<u8> {
    let body: String = (1..=pages)
        .map(|n| format!("<h1 style=\"break-before: page\">Shop page {n}</h1>"))
        .collect();
    rendered(&body, Options::default())
}

const INVOICE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<rsm:CrossIndustryInvoice \
xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\" \
xmlns:ram=\"urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100\">\
<rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter>\
<ram:ID>urn:cen.eu:en16931:2017</ram:ID>\
</ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext>\
<rsm:ExchangedDocument><ram:ID>Shop-1</ram:ID><ram:TypeCode>380</ram:TypeCode>\
</rsm:ExchangedDocument></rsm:CrossIndustryInvoice>";

#[test]
fn the_text_of_a_render_reads_back_in_its_lines() {
    let data = rendered(
        "<p>Invoice from Shop</p><p>Total: 42.00 EUR</p>",
        Options::default(),
    );
    let pages = text(data, &[1]).unwrap();
    assert_eq!(pages, ["Invoice from Shop\nTotal: 42.00 EUR"]);
}

#[test]
fn read_counts_pages_and_returns_metadata() {
    let data = rendered(
        "<html><head><title>Shop invoice</title><meta name=\"author\" content=\"Shop\">\
         </head><body><p>One</p><p style=\"break-before: page\">Two</p></body></html>",
        Options::default(),
    );
    let info = read(data).unwrap();
    assert_eq!(info.page_count, 2);
    assert_eq!(info.metadata.title.as_deref(), Some("Shop invoice"));
    assert_eq!(info.metadata.author.as_deref(), Some("Shop"));
    assert!(info.attachments.is_empty());
    assert_eq!(info.invoice_xml, None);
}

#[test]
fn read_returns_the_factur_x_xml() {
    let data = rendered(
        "<html lang=\"en\"><head><title>Shop</title></head><body><p>Invoice</p></body></html>",
        Options {
            standard: Some(Standard::PdfA3b),
            invoice_xml: Some(INVOICE.to_owned()),
            created: Some(Instant {
                seconds: 1_767_225_600,
                nanos: 0,
            }),
            ..Options::default()
        },
    );
    let info = read(data).unwrap();
    assert_eq!(info.attachments.len(), 1);
    assert_eq!(info.attachments[0].0, "factur-x.xml");
    assert_eq!(info.invoice_xml.as_deref(), Some(INVOICE));
}

#[test]
fn merge_selects_repeats_and_reorders_pages() {
    let merged = merge(vec![
        Part {
            data: numbered(3),
            pages: Some(vec![3, 1]),
        },
        Part {
            data: numbered(2),
            pages: None,
        },
        Part {
            data: numbered(1),
            pages: Some(vec![1, 1]),
        },
    ])
    .unwrap();
    assert_eq!(read(merged.clone()).unwrap().page_count, 6);
    let pages = text(merged.clone(), &[1, 2, 3, 4, 5, 6]).unwrap();
    assert_eq!(
        pages,
        [
            "Shop page 3",
            "Shop page 1",
            "Shop page 1",
            "Shop page 2",
            "Shop page 1",
            "Shop page 1"
        ]
    );
    let again = merge(vec![
        Part {
            data: numbered(3),
            pages: Some(vec![3, 1]),
        },
        Part {
            data: numbered(2),
            pages: None,
        },
        Part {
            data: numbered(1),
            pages: Some(vec![1, 1]),
        },
    ])
    .unwrap();
    assert_eq!(merged, again, "two merges of equal parts are byte-equal");
}

#[test]
fn a_page_outside_its_document_names_the_count() {
    let Err(Error::Invalid(message)) = merge(vec![Part {
        data: numbered(2),
        pages: Some(vec![3]),
    }]) else {
        panic!("page 3 of 2 merged");
    };
    assert!(message.contains("2 pages"), "{message}");
    let Err(Error::Invalid(message)) = text(numbered(1), &[0]) else {
        panic!("page 0 was read");
    };
    assert!(message.contains("1 page"), "{message}");
}

fn with_assets(html: &str, assets: Vec<(&str, Vec<u8>)>) -> Result<Vec<u8>, Error> {
    render(&Document {
        html: html.to_owned(),
        assets: assets
            .into_iter()
            .map(|(name, data)| Asset {
                name: name.to_owned(),
                data,
            })
            .collect(),
        ..Document::default()
    })
    .map(|rendered| rendered.data)
}

#[test]
fn a_pdf_page_in_an_img_is_drawn_as_its_own_content() {
    let data = with_assets(
        "<p>Before</p><img src=\"terms.pdf#page=2\" style=\"width: 100pt\"><p>After</p>",
        vec![("terms.pdf", numbered(3))],
    )
    .unwrap();
    let pages = text(data.clone(), &[1]).unwrap();
    assert_eq!(pages, ["Before\nShop page 2\nAfter"]);
    let Err(Error::Invalid(message)) = with_assets(
        "<img src=\"terms.pdf#page=4\">",
        vec![("terms.pdf", numbered(3))],
    ) else {
        panic!("page 4 of 3 was placed");
    };
    assert!(
        message.contains("terms.pdf") && message.contains("3 pages"),
        "{message}"
    );
}

#[test]
fn a_pdf_letterhead_is_under_every_page() {
    let letterhead = rendered("<p>Shop letterhead</p>", Options::default());
    let data = with_assets(
        "<style>@page { background-image: url(letterhead.pdf) }</style>\
         <p>One</p><p style=\"break-before: page\">Two</p>",
        vec![("letterhead.pdf", letterhead)],
    )
    .unwrap();
    let pages = text(data.clone(), &[1, 2]).unwrap();
    assert_eq!(pages, ["Shop letterhead\nOne", "Shop letterhead\nTwo"]);
    let forms = data
        .windows(b"/Subtype/Form".len())
        .filter(|w| *w == b"/Subtype/Form")
        .count();
    assert_eq!(forms, 1, "the letterhead is one form XObject");
}

#[test]
fn an_encrypted_source_throws_in_merge_read_and_text() {
    let locked = rendered(
        "<p>Shop</p>",
        Options {
            password: Some("Shop-secret".to_owned()),
            ..Options::default()
        },
    );
    let refused = |result: Result<(), Error>| match result {
        Err(Error::Parse(message)) => assert!(message.contains("encrypted"), "{message}"),
        other => panic!("an encrypted source gave {other:?}"),
    };
    refused(read(locked.clone()).map(drop));
    refused(text(locked.clone(), &[1]).map(drop));
    refused(
        merge(vec![Part {
            data: locked.clone(),
            pages: None,
        }])
        .map(drop),
    );
    refused(with_assets("<img src=\"locked.pdf\">", vec![("locked.pdf", locked)]).map(drop));
    let Err(Error::Parse(_)) = read(b"not a PDF".to_vec()) else {
        panic!("bytes that are not a PDF were read");
    };
}
