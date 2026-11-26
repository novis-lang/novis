//! The parse: headers, encoded words, the two bodies, attachments, names and bounds.

use super::*;

/// `lines` joined with CRLF, as a message is written on the wire.
fn crlf(lines: &[&str]) -> Vec<u8> {
    lines.join("\r\n").into_bytes()
}

/// A message with every header the readers read, and an encoded subject and name.
// covers: Core\Mail\Message::parse, Core\Mail\Message::messageId, Core\Mail\Message::from
// covers: Core\Mail\Message::to, Core\Mail\Message::cc, Core\Mail\Message::replyTo
// covers: Core\Mail\Message::subject, Core\Mail\Message::date, Core\Mail\Message::header
// covers: Core\Mail\Message::inReplyTo, Core\Mail\Message::references
// covers: Core\Mail\Address::address, Core\Mail\Address::name
#[test]
fn parse_reads_the_headers_and_decodes_encoded_words() {
    let raw = crlf(&[
        "Message-ID: <2b@example.com>",
        "From: =?UTF-8?Q?J=C3=B6rg_Shop?= <shop@example.com>",
        "To: ada@example.com, \"Bo B\" <bo@example.com>",
        "Cc: desk@example.com",
        "Reply-To: help@example.com",
        "Subject: =?UTF-8?B?UmU6IEdyw7zDn2U=?=",
        "Date: Tue, 3 Nov 2026 14:05:09 +0100",
        "In-Reply-To: <1a@example.com>",
        "References: <09@example.com> <1a@example.com>",
        "X-Note: one",
        "x-note: =?UTF-8?Q?tw=C3=B6?=",
        "",
        "Hello.",
    ]);
    let parsed = parse(&raw);
    assert_eq!(parsed.message_id.as_deref(), Some("2b@example.com"));
    assert_eq!(
        parsed.from,
        Some(Mailbox {
            address: "shop@example.com".to_owned(),
            name: "Jörg Shop".to_owned(),
        })
    );
    let addresses = |list: &[Mailbox]| list.iter().map(|m| m.address.clone()).collect::<Vec<_>>();
    assert_eq!(addresses(&parsed.to), ["ada@example.com", "bo@example.com"]);
    assert_eq!(parsed.to[1].name, "Bo B");
    assert_eq!(addresses(&parsed.cc), ["desk@example.com"]);
    assert_eq!(addresses(&parsed.reply_to), ["help@example.com"]);
    assert_eq!(parsed.subject, "Re: Grüße");
    let (at, offset) = parsed.date.expect("the date is read");
    assert_eq!(
        (at.year, at.month, at.day, at.hour, at.minute, at.second),
        (2026, 11, 3, 14, 5, 9)
    );
    assert_eq!(offset, 3600);
    assert_eq!(parsed.in_reply_to.as_deref(), Some("1a@example.com"));
    assert_eq!(parsed.references, ["09@example.com", "1a@example.com"]);
    let notes: Vec<&str> = parsed
        .headers
        .iter()
        .filter(|(name, _)| name == "x-note")
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(notes, ["one", "twö"], "every value, decoded, in order");
}

/// A `multipart/alternative` gives its text part to `text` and its HTML part to `html`, each
/// decoded from its transfer encoding.
// covers: Core\Mail\Message::text, Core\Mail\Message::html
#[test]
fn parse_reads_the_text_and_the_html_body() {
    let raw = crlf(&[
        "From: shop@example.com",
        "Subject: Both",
        "MIME-Version: 1.0",
        "Content-Type: multipart/alternative; boundary=\"b1\"",
        "",
        "--b1",
        "Content-Type: text/plain; charset=utf-8",
        "Content-Transfer-Encoding: quoted-printable",
        "",
        "Gr=C3=BC=C3=9Fe",
        "--b1",
        "Content-Type: text/html; charset=utf-8",
        "Content-Transfer-Encoding: base64",
        "",
        "PHA+SGk8L3A+",
        "--b1--",
        "",
    ]);
    let parsed = parse(&raw);
    assert_eq!(parsed.text.as_deref().map(str::trim_end), Some("Grüße"));
    assert_eq!(parsed.html.as_deref(), Some("<p>Hi</p>"));
    assert!(
        parsed.attachments.is_empty(),
        "a body part is not an attachment"
    );

    let html_only = crlf(&["Content-Type: text/html", "", "<b>x</b>"]);
    let parsed = parse(&html_only);
    assert_eq!(parsed.text, None, "no text/plain part gives no text");
    assert_eq!(parsed.html.as_deref().map(str::trim_end), Some("<b>x</b>"));
}

/// A file part and an inline image are both attachments, with their name, type, bytes and
/// Content-ID.
// covers: Core\Mail\Message::attachments
#[test]
fn parse_reads_attachments_and_inline_parts() {
    let raw = crlf(&[
        "Content-Type: multipart/mixed; boundary=\"m\"",
        "",
        "--m",
        "Content-Type: multipart/related; boundary=\"r\"",
        "",
        "--r",
        "Content-Type: text/html",
        "",
        "<img src=\"cid:logo@example.com\">",
        "--r",
        "Content-Type: image/png",
        "Content-ID: <logo@example.com>",
        "Content-Disposition: inline; filename=\"logo.png\"",
        "Content-Transfer-Encoding: base64",
        "",
        "iVBORw==",
        "--r--",
        "--m",
        "Content-Type: application/pdf; name=\"=?UTF-8?Q?Rechnung_M=C3=A4rz.pdf?=\"",
        "Content-Disposition: attachment",
        "Content-Transfer-Encoding: base64",
        "",
        "JVBERg==",
        "--m--",
        "",
    ]);
    let parsed = parse(&raw);
    assert_eq!(parsed.attachments.len(), 2, "{:?}", parsed.attachments);
    let logo = &parsed.attachments[0];
    assert_eq!(logo.name.as_deref(), Some("logo.png"));
    assert_eq!(logo.media_type, "image/png");
    assert_eq!(logo.data, [0x89, b'P', b'N', b'G']);
    assert_eq!(logo.content_id.as_deref(), Some("logo@example.com"));
    let pdf = &parsed.attachments[1];
    assert_eq!(pdf.name.as_deref(), Some("Rechnung März.pdf"));
    assert_eq!(pdf.media_type, "application/pdf");
    assert_eq!(pdf.data, b"%PDF");
    assert_eq!(pdf.content_id, None);
}

/// A name is cut to its last segment after either separator, and a control byte is removed.
#[test]
fn an_attachment_name_is_its_last_path_segment() {
    assert_eq!(file_name("../../etc/passwd").as_deref(), Some("passwd"));
    assert_eq!(
        file_name("C:\\Users\\x\\report.pdf").as_deref(),
        Some("report.pdf")
    );
    assert_eq!(file_name("/abs/a.txt").as_deref(), Some("a.txt"));
    assert_eq!(
        file_name("in\u{0}voice\r\n.pdf").as_deref(),
        Some("invoice.pdf")
    );
    assert_eq!(file_name("plain.txt").as_deref(), Some("plain.txt"));

    let raw = crlf(&[
        "Content-Type: multipart/mixed; boundary=\"m\"",
        "",
        "--m",
        "Content-Type: text/plain",
        "",
        "body",
        "--m",
        "Content-Type: text/plain",
        "Content-Disposition: attachment; filename=\"../../secret/notes.txt\"",
        "",
        "notes",
        "--m--",
        "",
    ]);
    let parsed = parse(&raw);
    assert_eq!(parsed.attachments.len(), 1, "{:?}", parsed.attachments);
    assert_eq!(parsed.attachments[0].name.as_deref(), Some("notes.txt"));
}

/// A name that is empty, only separators, only control bytes, `.` or `..` is `None`.
#[test]
fn a_name_with_nothing_left_is_null() {
    for written in ["", "/", "dir/", "..\\", "\u{1}\u{7f}", "a/..", "x/.", "  "] {
        assert_eq!(file_name(written), None, "{written:?}");
    }
}

/// `depth` multiparts, one inside the other, with a text part at the bottom.
fn nested(depth: usize) -> Vec<u8> {
    let mut lines = vec!["Subject: deep".to_owned()];
    for level in 0..depth {
        lines.push(format!(
            "Content-Type: multipart/mixed; boundary=\"n{level}\""
        ));
        lines.push(String::new());
        lines.push(format!("--n{level}"));
    }
    lines.push("Content-Type: application/octet-stream".to_owned());
    lines.push("Content-Disposition: attachment; filename=\"deep.bin\"".to_owned());
    lines.push(String::new());
    lines.push("deep".to_owned());
    for level in (0..depth).rev() {
        lines.push(format!("--n{level}--"));
    }
    lines.push(String::new());
    lines.join("\r\n").into_bytes()
}

/// A part nested deeper than the bound is skipped, and the headers above it are still read.
#[test]
fn parse_stops_at_the_nesting_bound() {
    let shallow = parse(&nested(4));
    assert_eq!(shallow.attachments.len(), 1, "a shallow part is read");
    assert_eq!(shallow.attachments[0].name.as_deref(), Some("deep.bin"));

    let deep = parse(&nested(MAX_DEPTH * 2));
    assert_eq!(deep.subject, "deep");
    assert!(
        deep.attachments
            .iter()
            .all(|part| part.name.as_deref() != Some("deep.bin")),
        "the part past the bound is skipped: {:?}",
        deep.attachments
    );
}

/// A message with more parts than the bound keeps the parts before it and no more.
#[test]
fn parse_stops_at_the_part_count_bound() {
    let mut lines = vec![
        "Subject: many".to_owned(),
        "Content-Type: multipart/mixed; boundary=\"p\"".to_owned(),
        String::new(),
    ];
    for index in 0..MAX_PARTS + 100 {
        lines.push("--p".to_owned());
        lines.push(format!(
            "Content-Disposition: attachment; filename=\"f{index}.bin\""
        ));
        lines.push("Content-Type: application/octet-stream".to_owned());
        lines.push(String::new());
        lines.push(format!("{index}"));
    }
    lines.push("--p--".to_owned());
    let parsed = parse(lines.join("\r\n").as_bytes());
    assert_eq!(parsed.subject, "many");
    let count = parsed.attachments.len();
    assert!(count > 0 && count < MAX_PARTS, "{count} parts were read");
    assert_eq!(parsed.attachments[0].name.as_deref(), Some("f0.bin"));
}

/// Bytes with no header block are an empty message, and so is no input at all.
// covers: Core\Mail\Message::raw
#[test]
fn a_message_with_no_headers_is_an_empty_message() {
    for raw in [&b""[..], b"no header here", b"\x00\xff\xfe"] {
        let parsed = parse(raw);
        assert_eq!(parsed.message_id, None, "{raw:?}");
        assert_eq!(parsed.from, None);
        assert!(parsed.to.is_empty() && parsed.references.is_empty());
        assert_eq!(parsed.subject, "");
        assert_eq!(parsed.date, None);
        assert!(parsed.attachments.is_empty());
        let message = message_of(raw);
        let object = crate::instance::receiver(message, &MESSAGE, "test").expect("a message");
        let kept = crate::instance::slot(object, slot_of("raw"));
        assert_eq!(kept.as_bytes(), Some(raw), "raw() returns the bytes");
        #[expect(
            unsafe_code,
            reason = "the message was built here and nothing else holds it"
        )]
        // SAFETY: `message_of` returns one reference, owned by this test.
        unsafe {
            message.release();
        }
    }
}

/// Only the first [`MAX_HEADERS`] header lines of a part are read.
#[test]
fn parse_stops_at_the_header_line_bound() {
    let mut lines: Vec<String> = (0..MAX_HEADERS).map(|i| format!("X-Fill: {i}")).collect();
    lines.push("Subject: past the bound".to_owned());
    lines.push(String::new());
    lines.push("body".to_owned());
    let parsed = parse(lines.join("\r\n").as_bytes());
    assert_eq!(parsed.headers.len(), MAX_HEADERS);
    assert_eq!(parsed.subject, "", "a header past the bound is not read");
}

/// Every seed under `fuzz/seeds/mail_parse/` parses without a panic, and its instance's slots
/// fit their types.
#[test]
fn parse_never_panics_on_the_fuzz_corpus() {
    let dir = nvs_repo::path("fuzz/seeds/mail_parse");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).expect("the seed corpus is committed") {
        let path = entry.expect("a directory entry").path();
        let raw = std::fs::read(&path).expect("a seed reads");
        let message = crate::fuzz::mail_parse(&raw);
        crate::fuzz::check_fields(message.get())
            .unwrap_or_else(|why| panic!("{}: {why}", path.display()));
        seen += 1;
    }
    assert!(seen > 0, "no seed was found under {}", dir.display());
}
