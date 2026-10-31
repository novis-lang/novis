//! The PDF component's standards as `nvs-ext` sees them (`rule:core-classes/pdf-one-engine`): every
//! file is tagged from its HTML, with headings, paragraphs, lists, tables with header cells,
//! figures with their `alt`, and the document's language. PDF/A-2b, PDF/A-3b and PDF/UA-1 are
//! `krilla`'s validators, and a file that breaks its standard throws naming what broke. A
//! Factur-X invoice XML is embedded under PDF/A-3b alone, with the standard's XMP fields, and an
//! XML that is not well-formed throws. A password encrypts the file with AES-256, checked by
//! opening it with `hayro-syntax` and with `lopdf`, whose user-password check and file key check
//! `/U` and `/Perms`, and a password under PDF/A throws.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::PDF_CLASS;
use nvs_ext::call::{Error, Failure, Host, Meter};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::Loader;

/// A 1×1 PNG.
const PIXEL_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

/// `future` polled on this thread until it finishes.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}

fn shape(fields: Vec<(&str, Value)>) -> Value {
    Value::Array(
        fields
            .into_iter()
            .map(|(name, value)| (Key::String(name.to_owned()), value))
            .collect(),
    )
}

/// The file `Engine::render` returns for `html` with no option set.
fn render(html: &str) -> Result<Vec<u8>, Failure> {
    render_under(html, None, None)
}

/// The file `Engine::render` returns for `html` under the `Standard` case `standard`, created at
/// `created` seconds since the epoch.
fn render_under(
    html: &str,
    standard: Option<&str>,
    created: Option<i64>,
) -> Result<Vec<u8>, Failure> {
    render_invoice(html, standard, created, None)
}

/// The file `Engine::render` returns as [`render_under`] does, with `invoice` as its invoice XML.
fn render_invoice(
    html: &str,
    standard: Option<&str>,
    created: Option<i64>,
    invoice: Option<&str>,
) -> Result<Vec<u8>, Failure> {
    render_locked(html, standard, created, invoice, None)
}

/// The file `Engine::render` returns as [`render_invoice`] does, with `password` as its password.
fn render_locked(
    html: &str,
    standard: Option<&str>,
    created: Option<i64>,
    invoice: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<u8>, Failure> {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let extension = builtins
        .iter()
        .find(|builtin| builtin.manifest().class == PDF_CLASS)
        .expect("the PDF component is built in")
        .extension()
        .expect("the PDF component compiles and checks");
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let pass = shape(vec![
        ("html", Value::String(html.to_owned())),
        ("assets", Value::Array(Vec::new())),
        (
            "options",
            shape(vec![
                (
                    "standard",
                    standard.map_or(Value::Null, |case| Value::Case(case.to_owned())),
                ),
                (
                    "password",
                    password.map_or(Value::Null, |text| Value::String(text.to_owned())),
                ),
                (
                    "invoiceXml",
                    invoice.map_or(Value::Null, |xml| Value::String(xml.to_owned())),
                ),
                (
                    "created",
                    created.map_or(Value::Null, |seconds| Value::Core {
                        class: "Core\\Time\\Instant".to_owned(),
                        fields: vec![
                            ("seconds".to_owned(), Value::Int(seconds)),
                            ("nanos".to_owned(), Value::Int(0)),
                        ],
                    }),
                ),
            ]),
        ),
    ]);
    let result = block_on(request.call_values(extension, "render", vec![pass]));
    block_on(request.end()).expect("the request ends");
    let Value::Array(fields) = result?.expect("`render` returns a value") else {
        panic!("the result is not a shape");
    };
    let data = fields
        .into_iter()
        .find(|(key, _)| matches!(key, Key::String(key) if key == "data"))
        .map(|(_, value)| value);
    let Some(Value::Bytes(data)) = data else {
        panic!("`data` is not bytes");
    };
    Ok(data)
}

/// `needle` occurs in `haystack`.
fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

#[test]
fn output_is_tagged_by_default() {
    let html = format!(
        "<html lang=\"de\"><body><h1>Shop</h1><p>Tea and books.</p>\
         <ul><li>Tea</li><li>Books</li></ul>\
         <table><thead><tr><th>Item</th><th>Price</th></tr></thead>\
         <tbody><tr><td>Tea</td><td>1.50</td></tr></tbody></table>\
         <img alt=\"The shop's logo\" src=\"data:image/png;base64,{PIXEL_PNG}\">\
         <p>Thank you.</p></body></html>"
    );
    let data = render(&html).expect("it renders");
    for needle in [
        "/StructTreeRoot",
        "/Marked true",
        "/H1",
        "/P",
        "/L",
        "/LI",
        "/Table",
        "/TR",
        "/TH",
        "/TD",
        "/Figure",
        "(The shop's logo)",
        "/Lang",
    ] {
        assert!(contains(&data, needle), "the file has no `{needle}`");
    }
    assert_eq!(
        render(&html).expect("it renders again"),
        data,
        "two tagged renders differ"
    );
}

/// The message of the `Invalid` error `failure` is.
fn invalid(failure: Result<Vec<u8>, Failure>) -> String {
    match failure {
        Err(Failure::Error(Error::Invalid(message))) => message,
        Err(other) => panic!("not an invalid error: {other:?}"),
        Ok(_) => panic!("it rendered"),
    }
}

/// A shop's page with a title, a language, a heading, a table and an image with its `alt`.
fn shop_page() -> String {
    format!(
        "<html lang=\"en\"><head><title>Shop</title></head><body><h1>Shop</h1>\
         <p>Tea and books.</p><table><tr><th>Item</th></tr><tr><td>Tea</td></tr></table>\
         <img alt=\"The shop's logo\" src=\"data:image/png;base64,{PIXEL_PNG}\">\
         <p><a href=\"https://example.com/\">example.com</a></p></body></html>"
    )
}

/// 2026-01-01 at midnight, UTC.
const NEW_YEAR: i64 = 1_767_225_600;

// covers: Novis\Pdf\Document::standard
// covers: Novis\Pdf\Document::created
#[test]
fn pdf_a_2b_and_3b_validate() {
    let html = shop_page();
    for (case, part) in [("PdfA2b", "2"), ("PdfA3b", "3")] {
        let data = render_under(&html, Some(case), Some(NEW_YEAR)).expect("it renders");
        for needle in [
            "/GTS_PDFA1".to_owned(),
            format!("<pdfaid:part>{part}</pdfaid:part>"),
            "<pdfaid:conformance>B</pdfaid:conformance>".to_owned(),
            "/StructTreeRoot".to_owned(),
        ] {
            assert!(contains(&data, &needle), "{case} has no `{needle}`");
        }
        assert_eq!(
            render_under(&html, Some(case), Some(NEW_YEAR)).expect("it renders again"),
            data,
            "two {case} renders differ"
        );
        // PDF/A needs the creation time, and nothing in the component reads a clock.
        let message = invalid(render_under(&html, Some(case), None));
        assert!(message.contains("creation time"), "{message}");
    }
}

#[test]
fn pdf_ua_1_throws_for_an_image_without_alt() {
    let data = render_under(&shop_page(), Some("PdfUa1"), None).expect("it renders");
    assert!(contains(&data, "pdfuaid"), "the file does not claim PDF/UA");
    let html = format!(
        "<html lang=\"en\"><head><title>Shop</title></head><body><h1>Shop</h1>\
         <img src=\"data:image/png;base64,{PIXEL_PNG}\"></body></html>"
    );
    let message = invalid(render_under(&html, Some("PdfUa1"), None));
    assert!(
        message.contains("PDF/UA-1") && message.contains("an image has no `alt` text"),
        "{message}"
    );
    // Without a standard, the same document renders.
    render(&html).expect("it renders with no standard");
}

/// A Factur-X invoice of the EN 16931 profile from a shop, cut down to its context and number.
const INVOICE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<rsm:CrossIndustryInvoice \
xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\" \
xmlns:ram=\"urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100\">\
<rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter>\
<ram:ID>urn:cen.eu:en16931:2017</ram:ID>\
</ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext>\
<rsm:ExchangedDocument><ram:ID>Shop-1</ram:ID><ram:TypeCode>380</ram:TypeCode>\
</rsm:ExchangedDocument></rsm:CrossIndustryInvoice>";

/// How often `needle` occurs in `haystack`.
fn count(haystack: &[u8], needle: &str) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle.as_bytes())
        .count()
}

/// Every in-use entry of `file`'s cross-reference table points at its object's header.
fn assert_xref_points_at_objects(file: &[u8]) {
    let start = file
        .windows(10)
        .rposition(|window| window == b"startxref\n")
        .expect("the file has a startxref");
    // The table and the trailer after it are ASCII.
    let tail = std::str::from_utf8(&file[start + 10..]).expect("the trailer is ASCII");
    let xref: usize = tail
        .lines()
        .next()
        .and_then(|line| line.parse().ok())
        .expect("startxref names an offset");
    let table = std::str::from_utf8(&file[xref..start]).expect("the table is ASCII");
    assert!(table.starts_with("xref\n0 "), "startxref misses the table");
    let mut lines = table.lines().skip(1);
    let total: usize = lines
        .next()
        .and_then(|line| line.strip_prefix("0 "))
        .and_then(|count| count.parse().ok())
        .expect("the table has a count");
    for (id, entry) in lines.take(total).enumerate() {
        if !entry.trim_end().ends_with('n') {
            continue;
        }
        let offset: usize = entry[..10].parse().expect("an entry has an offset");
        assert!(
            file[offset..].starts_with(format!("{id} 0 obj").as_bytes()),
            "object {id} is not at {offset}"
        );
    }
}

// covers: Novis\Pdf\Document::invoiceXml
#[test]
fn factur_x_embeds_one_xml_under_pdf_a_3b() {
    let html = shop_page();
    let data =
        render_invoice(&html, Some("PdfA3b"), Some(NEW_YEAR), Some(INVOICE)).expect("it renders");
    for needle in [
        "(factur-x.xml)",
        "/EmbeddedFiles",
        "/AF",
        "/Alternative",
        "/text#2Fxml",
        "<pdfaid:part>3</pdfaid:part>",
        "<pdfaSchema:prefix>fx</pdfaSchema:prefix>",
        "<fx:DocumentType>INVOICE</fx:DocumentType>",
        "<fx:DocumentFileName>factur-x.xml</fx:DocumentFileName>",
        "<fx:Version>1.0</fx:Version>",
        "<fx:ConformanceLevel>EN 16931</fx:ConformanceLevel>",
    ] {
        assert!(contains(&data, needle), "the file has no `{needle}`");
    }
    assert_eq!(
        count(&data, "/EmbeddedFile"),
        2,
        "one file stream and the name tree"
    );
    assert_xref_points_at_objects(&data);
    assert_eq!(
        render_invoice(&html, Some("PdfA3b"), Some(NEW_YEAR), Some(INVOICE))
            .expect("it renders again"),
        data,
        "two invoice renders differ"
    );
    // The profile comes from the invoice's guideline id.
    let basic = INVOICE.replace(
        "urn:cen.eu:en16931:2017<",
        "urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:basic<",
    );
    let data =
        render_invoice(&html, Some("PdfA3b"), Some(NEW_YEAR), Some(&basic)).expect("it renders");
    assert!(contains(
        &data,
        "<fx:ConformanceLevel>BASIC</fx:ConformanceLevel>"
    ));
    assert_xref_points_at_objects(&data);
}

#[test]
fn an_invoice_xml_outside_pdf_a_3b_throws() {
    for standard in [None, Some("PdfA2b"), Some("PdfUa1")] {
        let message = invalid(render_invoice(
            &shop_page(),
            standard,
            Some(NEW_YEAR),
            Some(INVOICE),
        ));
        assert!(message.contains("PDF/A-3b"), "{standard:?}: {message}");
    }
}

// covers: Novis\Pdf\Document::password
#[test]
fn a_password_encrypts_with_aes_256_and_stays_byte_equal() {
    let html = shop_page();
    let data =
        render_locked(&html, None, Some(NEW_YEAR), None, Some("Shop-secret")).expect("it renders");
    for needle in [
        "/Filter /Standard",
        "/V 5",
        "/R 6",
        "/CFM /AESV3",
        "/Encrypt ",
    ] {
        assert!(contains(&data, needle), "the file has no `{needle}`");
    }
    // No string or stream is readable without the key.
    let plain = render_under(&html, None, Some(NEW_YEAR)).expect("it renders unlocked");
    for needle in ["(Shop)", "<x:xmpmeta"] {
        assert!(
            contains(&plain, needle),
            "the unlocked file has no `{needle}`"
        );
        assert!(!contains(&data, needle), "`{needle}` is readable");
    }
    assert_xref_points_at_objects(&data);
    assert_eq!(
        render_locked(&html, None, Some(NEW_YEAR), None, Some("Shop-secret"))
            .expect("it renders again"),
        data,
        "two encrypted renders differ"
    );
    assert_ne!(
        render_locked(&html, None, Some(NEW_YEAR), None, Some("Blog-secret"))
            .expect("it renders under another password"),
        data,
        "two passwords give one file"
    );

    // A reader that is not the writer opens it with the password and with nothing else.
    for wrong in ["", "Shop-Secret"] {
        match hayro_syntax::Pdf::new_with_password(data.clone(), wrong) {
            Err(hayro_syntax::LoadPdfError::Decryption(
                hayro_syntax::DecryptionError::PasswordProtected,
            )) => {}
            Err(other) => panic!("`{wrong}`: not a password error: {other:?}"),
            Ok(_) => panic!("`{wrong}` opens the file"),
        }
    }
    let pdf = hayro_syntax::Pdf::new_with_password(data, "Shop-secret").expect("it opens");
    assert_eq!(pdf.metadata().title.as_deref(), Some(&b"Shop"[..]));
    let pages = pdf.pages();
    assert_eq!(pages.len(), 1);
    let content = pages[0].page_stream().expect("the page has content");
    assert!(
        contains(content, " Tf"),
        "the page's content does not decrypt to text"
    );
}

/// The password of every encrypted render below.
const PASSWORD: &str = "Shop-secret";

/// `shop_page` rendered with [`PASSWORD`].
fn shop_page_locked() -> Vec<u8> {
    render_locked(&shop_page(), None, Some(NEW_YEAR), None, Some(PASSWORD)).expect("it renders")
}

// covers: Novis\Pdf\Document::password
#[test]
fn a_second_reader_opens_the_encrypted_file_with_the_user_password() {
    let data = shop_page_locked();
    // Loaded without a password, the file stays encrypted, and only the password is its user
    // password (ISO 32000-2's Algorithm 11).
    let locked = lopdf::Document::load_mem(&data).expect("the encrypted file parses");
    assert!(locked.is_encrypted(), "the file is not encrypted");
    locked
        .authenticate_user_password(PASSWORD)
        .expect("the password is the user password");
    for wrong in ["", "Shop-Secret"] {
        assert!(
            locked.authenticate_user_password(wrong).is_err(),
            "`{wrong}` is a user password"
        );
    }

    let pdf =
        lopdf::Document::load_mem_with_options(&data, lopdf::LoadOptions::with_password(PASSWORD))
            .expect("it opens with the password");
    assert!(pdf.was_encrypted());
    let pages = pdf.get_pages();
    assert_eq!(pages.len(), 1);
    let content = pdf.get_page_content(pages[&1]);
    assert!(
        contains(&content, " Tf"),
        "the page's content does not decrypt to text"
    );
    let info = pdf
        .trailer
        .get(b"Info")
        .and_then(lopdf::Object::as_reference)
        .and_then(|id| pdf.get_dictionary(id))
        .expect("the file has an information dictionary");
    let title = info
        .get(b"Title")
        .and_then(lopdf::decode_text_string)
        .expect("the file has a title");
    assert_eq!(title, "Shop");
}

// covers: Novis\Pdf\Document::password
#[test]
fn a_second_reader_checks_the_perms_entry() {
    use aes::cipher::{BlockCipherDecrypt as _, KeyInit as _};

    let data = shop_page_locked();
    let locked = lopdf::Document::load_mem(&data).expect("the encrypted file parses");
    let encrypt = locked
        .get_encrypted()
        .expect("the file has an `/Encrypt` dictionary");
    let p = encrypt
        .get(b"P")
        .and_then(lopdf::Object::as_i64)
        .expect("`/Encrypt` has `/P`");
    let perms: [u8; 16] = encrypt
        .get(b"Perms")
        .and_then(lopdf::Object::as_str)
        .expect("`/Encrypt` has `/Perms`")
        .try_into()
        .expect("`/Perms` is 16 bytes");

    // ISO 32000-2's Algorithm 13: `/Perms` decrypts with the file key, AES-256 in ECB mode.
    let pdf =
        lopdf::Document::load_mem_with_options(&data, lopdf::LoadOptions::with_password(PASSWORD))
            .expect("it opens with the password");
    let key = pdf
        .encryption_state
        .as_ref()
        .expect("the file was encrypted")
        .file_encryption_key();
    let cipher = aes::Aes256::new_from_slice(key).expect("the file key is 32 bytes");
    let mut block = aes::Block::from(perms);
    cipher.decrypt_block(&mut block);

    assert_eq!(&block[9..12], b"adb", "`/Perms` does not decrypt");
    let flags = i32::from_le_bytes(block[..4].try_into().expect("four bytes"));
    assert_eq!(i64::from(flags), p, "`/Perms` and `/P` differ");
    assert_eq!(
        block[8], b'T',
        "`/Perms` does not say the metadata is encrypted"
    );
}

#[test]
fn a_password_with_a_pdf_a_standard_throws() {
    for case in ["PdfA2b", "PdfA3b"] {
        let message = invalid(render_locked(
            &shop_page(),
            Some(case),
            Some(NEW_YEAR),
            None,
            Some("Shop-secret"),
        ));
        assert!(message.contains("password"), "{case}: {message}");
    }
    // PDF/UA allows encryption.
    render_locked(
        &shop_page(),
        Some("PdfUa1"),
        None,
        None,
        Some("Shop-secret"),
    )
    .expect("PDF/UA-1 renders with a password");
}

#[test]
fn a_malformed_invoice_xml_throws() {
    let entity = "<?xml version=\"1.0\"?><!DOCTYPE i [<!ENTITY a \"aaaa\">]><i>&a;&a;</i>";
    for xml in ["<rsm:CrossIndustryInvoice><unclosed>", "not XML", entity] {
        match render_invoice(&shop_page(), Some("PdfA3b"), Some(NEW_YEAR), Some(xml)) {
            Err(Failure::Error(Error::Parse(message))) => {
                assert!(message.contains("invoice XML"), "{message}");
            }
            other => panic!("{xml}: not a parse error: {other:?}"),
        }
    }
}
