//! Reading and merging existing PDFs on the host (`rule:core-classes/pdf-reader-and-merge`):
//! the page count, the metadata, the attachments and a page's text, merging with page lists, and
//! the refusals every job shares, and the decompression bound each job decodes under
//! (`rule:core-classes/decompression-bound`).

use nvs_pdf::merge::Part;
use nvs_pdf::read::{Bound, Info, Reading};
use nvs_pdf::{Asset, Document, Error, Instant, Options, Standard, render};

fn read(data: Vec<u8>) -> Result<Info, Error> {
    nvs_pdf::read::read(data, Bound::default()).map(|(_, info)| info)
}

fn open(data: Vec<u8>) -> Result<Reading, Error> {
    nvs_pdf::read::read(data, Bound::default()).map(|(file, _)| file)
}

/// The text of `pages` of `data`, through a reading opened for this call alone.
fn text(data: Vec<u8>, pages: &[u64]) -> Result<Vec<String>, Error> {
    nvs_pdf::read::text(&open(data)?, pages, Bound::default())
}

fn merge(parts: Vec<Part>) -> Result<Vec<u8>, Error> {
    nvs_pdf::merge::merge(parts, Bound::default())
}

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
fn text_reads_the_file_its_reading_holds() {
    // The source bytes move into the reading, so every later call reads the reading's copy.
    let shop = open(numbered(3)).unwrap();
    let blog = open(rendered("<p>Blog post</p>", Options::default())).unwrap();
    let page =
        |file: &Reading, page: u64| nvs_pdf::read::text(file, &[page], Bound::default()).unwrap();
    assert_eq!(page(&shop, 3), ["Shop page 3"]);
    assert_eq!(page(&blog, 1), ["Blog post"]);
    assert_eq!(page(&shop, 1), ["Shop page 1"]);
    assert_eq!(
        nvs_pdf::read::text(&shop, &[2, 2], Bound::default()).unwrap(),
        ["Shop page 2", "Shop page 2"]
    );
    let Err(Error::Invalid(message)) = nvs_pdf::read::text(&blog, &[2], Bound::default()) else {
        panic!("a page past the blog's one page was read");
    };
    assert!(message.contains("1 page"), "{message}");
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

/// A one-page PDF built by hand whose only font is a CID font with no `ToUnicode` map, encoded
/// with the predefined CMap `encoding` over the collection `ordering`, and whose page shows the
/// hexadecimal codes `codes`.
fn cid_pdf(encoding: &str, ordering: &str, codes: &str) -> Vec<u8> {
    let content = format!("BT /F1 24 Tf 72 700 Td <{codes}> Tj ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /ShopSans /Encoding /{encoding} \
             /DescendantFonts [6 0 R] >>"
        ),
        format!(
            "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /ShopSans \
             /CIDSystemInfo << /Registry (Adobe) /Ordering ({ordering}) /Supplement 0 >> \
             /FontDescriptor 7 0 R >>"
        ),
        "<< /Type /FontDescriptor /FontName /ShopSans /Flags 4 /FontBBox [0 0 1000 1000] \
         /ItalicAngle 0 /Ascent 880 /Descent -120 /CapHeight 700 /StemV 80 >>"
            .to_owned(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (at, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", at + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[test]
fn a_predefined_cmap_gives_unicode_text() {
    // `UniGB-UCS2-H` maps UCS-2 codes to Adobe-GB1 CIDs, and `Adobe-GB1-UCS2` maps them back.
    let chinese = cid_pdf("UniGB-UCS2-H", "GB1", "4E2D6587");
    assert_eq!(text(chinese, &[1]).unwrap(), ["\u{4e2d}\u{6587}"]);
    // `90ms-RKSJ-H` is Shift JIS: 0x82A0 is あ and 0x82A2 is い.
    let japanese = cid_pdf("90ms-RKSJ-H", "Japan1", "82A082A2");
    assert_eq!(text(japanese, &[1]).unwrap(), ["\u{3042}\u{3044}"]);
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

#[test]
fn a_stream_past_the_decompression_bound_throws() {
    let data = rendered(&"<p>Shop invoice line</p>".repeat(200), Options::default());
    let ceiling = Bound {
        max_bytes: 100,
        max_ratio: 1000,
    };
    let ratio = Bound {
        max_bytes: 64 << 20,
        max_ratio: 1,
    };
    for (bound, key) in [
        (ceiling, "`[limits] max_decompressed`"),
        (ratio, "`[limits] max_decompression_ratio`"),
    ] {
        let jobs = [
            nvs_pdf::read::read(data.clone(), bound)
                .and_then(|(file, _)| nvs_pdf::read::text(&file, &[1], bound))
                .map(drop),
            nvs_pdf::merge::merge(
                vec![Part {
                    data: data.clone(),
                    pages: None,
                }],
                bound,
            )
            .map(drop)
            .or_else(|err| match err {
                // A page is copied compressed, so a file with no object stream merges.
                Error::Parse(_) => Err(err),
                other => panic!("merge gave {other:?}"),
            }),
        ];
        let Err(Error::Parse(message)) = &jobs[0] else {
            panic!("text under {bound:?} gave {:?}", jobs[0]);
        };
        assert!(message.contains(key), "{message}");
        if let Err(Error::Parse(message)) = &jobs[1] {
            assert!(message.contains(key), "{message}");
        }
    }
    text(data, &[1]).unwrap();
}

#[test]
fn an_attachment_past_the_decompression_bound_throws() {
    let padding = format!("<!-- {} -->", "Shop ".repeat(20_000));
    let invoice = INVOICE.replacen(
        "<rsm:ExchangedDocument>",
        &(padding + "<rsm:ExchangedDocument>"),
        1,
    );
    let data = rendered(
        "<html lang=\"en\"><head><title>Shop</title></head><p>Invoice</p></html>",
        Options {
            standard: Some(Standard::PdfA3b),
            invoice_xml: Some(invoice.clone()),
            created: Some(Instant {
                seconds: 1_767_225_600,
                nanos: 0,
            }),
            ..Options::default()
        },
    );
    let bound = Bound {
        max_bytes: 50_000,
        max_ratio: 1000,
    };
    match nvs_pdf::read::read(data.clone(), bound) {
        Err(Error::Parse(message)) => {
            assert!(message.contains("50000 bytes"), "{message}");
            assert!(message.contains("`[limits] max_decompressed`"), "{message}");
        }
        other => panic!("an attachment past the bound gave {other:?}"),
    }
    assert_eq!(
        read(data).unwrap().invoice_xml.as_deref(),
        Some(invoice.as_str())
    );
}
