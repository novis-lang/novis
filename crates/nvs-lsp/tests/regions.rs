//! What `nvs/regions` reports, and — the half a rule turns on — what it cannot
//! report.
//!
//! Here rather than in a unit test because the subject is the *answer*, not the
//! walk: the shape the client reads is the JSON `crate::regions::wire` builds,
//! and a field that reached an editor without passing through this file would
//! be a field nothing held to
//! `rule:ide/a-template-region-gets-services-but-no-second-formatter`. The
//! boundaries themselves are the lexer's and are tested where the lexer is.
//!
//! The expectations are `nvs_lsp::render`'s spelling
//! (`rule:ide/the-rendering-has-one-home`), so a `.lspt` case under
//! `tests/lsp/regions/` and a test here freeze the same text.

use nvs_diagnostics::{PositionEncoding, SourceMap};
use nvs_lsp::{Documents, Region, Response, regions, uri_of};

/// Every region of `source`, as one document's worth.
///
/// UTF-8 columns, which is what a `.lspt` case is read in: what a boundary is
/// has nothing to do with what units it is counted in
/// (`rule:ide/positions-have-one-home`).
fn reported(source: &str) -> Vec<Region> {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs", source);
    regions::for_source(map.file(id), PositionEncoding::Utf8)
}

/// The same list rendered as a case freezes it.
fn rendered(source: &str) -> String {
    Response::Regions(reported(source)).render()
}

/// What one request answers, rendered the same way.
///
/// The params are built as JSON and read back through
/// [`regions::Params::from_value`], so what these tests freeze is the object a
/// client sends rather than a struct assembled in Rust: a renamed field would
/// fail here exactly as it would fail on the wire.
fn asked(buffer: &str, text: Option<&str>) -> String {
    let uri = uri_of(&std::env::temp_dir().join("nvs-regions-request.nvs"))
        .expect("a temp path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, buffer.to_owned());

    let mut params = serde_json::json!({ "textDocument": { "uri": uri.as_str() } });
    if let Some(text) = text {
        params["text"] = serde_json::json!(text);
    }
    let params = regions::Params::from_value(params).expect("the params are readable");

    Response::Regions(regions::for_request(
        &documents,
        PositionEncoding::Utf8,
        &params,
    ))
    .render()
}

/// The answer is a span and a language, and there is nowhere in it for a
/// formatter to be named.
///
/// `rule:ide/a-template-region-gets-services-but-no-second-formatter` excludes
/// formatting from what an embedded service is registered for, and the client
/// is where that registration happens — but a *server* that shipped a third
/// field would be where the pressure to honour one came from. Two keys, checked
/// on the wire object rather than on the struct, because the client reads JSON.
#[test]
fn a_region_answer_carries_a_span_and_a_language_and_nothing_else() {
    let regions = reported("<?nvs\necho \"hi\";\n?>\n<p>hello</p>\n");
    let [region] = regions.as_slice() else {
        panic!("one `?>` opens one region, got {regions:?}");
    };

    let answer = regions::wire(region);
    let object = answer.as_object().expect("a region is a JSON object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["language", "range"],
        "a region carries a span and a language; anything else is a second \
         instruction to a client that already has one"
    );

    // And the two are what they claim to be: a range an editor can ask inside,
    // and a language it has a service for. A `range` that serialized as
    // something other than LSP's own shape would be a span no client could use.
    assert_eq!(object["language"], serde_json::json!(regions::HTML));
    let range = object["range"]
        .as_object()
        .expect("a range is LSP's `{start, end}`");
    assert!(range.contains_key("start") && range.contains_key("end"));
}

/// A document that is Novis from its first byte has no region, and that is an
/// answer rather than a silence.
///
/// Most `.nvs` files are this, and an empty list is what tells a client to stop
/// forwarding — one that was sent nothing would keep the services pointed at a
/// region the developer has since deleted.
#[test]
fn a_document_that_is_novis_throughout_reports_none() {
    assert_eq!(rendered("<?nvs\nvar $name = \"world\";\n"), "none\n");
}

/// Markup before the open tag is a region too.
///
/// The lexer starts a file in HTML mode (`crates/nvs-syntax/src/lexer.rs`'s
/// `Mode::Html`), so a template whose first bytes are a doctype is markup from
/// byte zero — there is no rule here saying a region follows a `?>`, only the
/// mode the lexer was in.
#[test]
fn markup_before_the_open_tag_is_a_region() {
    assert_eq!(
        rendered("<!doctype html>\n<?nvs\necho \"hi\";\n"),
        "1:1-2:1 html\n"
    );
}

/// A hole in the markup splits it, and both sides are reported.
///
/// Not one range spanning the `<?= … ?>`: the bytes between the two runs are
/// Novis, and an HTML service asked about them would be answering about a
/// variable. Joining them into one virtual document is the client's job,
/// because the client is what decides what goes in the gap.
#[test]
fn a_novis_hole_splits_one_paragraph_into_two_regions() {
    assert_eq!(
        rendered("<?nvs\nvar $name = \"world\";\n?>\n<p>hi <?= $name ?>!</p>\n"),
        "4:1-4:7 html\n4:19-5:1 html\n"
    );
}

/// A request carrying `text` is answered about that text, and the open buffer
/// is not read.
///
/// This is what the format pass asks (ADR 0173 § 4): the regions it needs are
/// the ones in `nvs fmt`'s output, which no buffer holds — the editor has not
/// applied the edit yet and may never, since a chunk whose formatting would
/// reach a hole is left as written. The buffer here is Novis throughout, so an
/// answer read off it would be `none` and the ranges below could not have come
/// from anywhere else.
#[test]
fn a_regions_request_carrying_text_answers_for_that_text_and_not_the_buffer() {
    let buffer = "<?nvs\nvar $name = \"world\";\n";
    assert_eq!(asked(buffer, None), "none\n");
    assert_eq!(
        asked(
            buffer,
            Some("<?nvs\nvar $name = \"world\";\n?>\n<p>hi</p>\n")
        ),
        "4:1-5:1 html\n"
    );
}

/// A request carrying no `text` is answered about the open document, as it was
/// before there was a `text` to carry.
///
/// The forwarding of hover, completion and the colour picker into a region asks
/// exactly this, on the heels of every edit, and it has no text to send: the
/// buffer is the document. A params object with no `text` key is what that
/// looks like on the wire, so the field's absence is read here rather than a
/// `null` standing in for it.
#[test]
fn a_regions_request_without_text_answers_for_the_open_document() {
    assert_eq!(
        asked("<?nvs\necho \"hi\";\n?>\n<p>hello</p>\n", None),
        "4:1-5:1 html\n"
    );
}
