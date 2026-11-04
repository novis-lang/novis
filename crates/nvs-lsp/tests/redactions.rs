//! What `nvs/redactions` conceals, over documents the checker was able to say
//! something about and documents it was not.
//!
//! Here rather than in a unit test because the answer is a *type* question:
//! `crate::redactions`' walk asks what a binding was declared as, and only a
//! full analysis — the graph `nvs check` resolves, the check it runs — answers
//! that. An open buffer stands in for the file under it
//! (`nvs_diagnostics::SourceMap::load`), so a case here needs no directory on
//! disk: nothing in it `require`s anything.
//!
//! The expectations are `nvs_lsp::render`'s spelling, not this file's
//! (`rule:ide/the-rendering-has-one-home`), so a `.lspt` case under
//! `tests/lsp/redactions/` and a test here freeze the same text.

use lsp_types::{ClientCapabilities, InitializeParams};
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::{Documents, Response, Settings, analyse, redactions, uri_of};

/// What the server would answer for `source`, rendered as a case freezes it.
///
/// UTF-8 columns, which is what a `.lspt` case is read in: the encoding
/// negotiation is `rule:ide/positions-have-one-home`'s business and has nothing
/// to say about which ranges are answered.
fn concealed(source: &str) -> String {
    answered(source, false)
}

/// [`concealed`], for a client that did or did not declare `docComment`.
fn answered(source: &str, doc_comments: bool) -> String {
    let dir = nvs_repo::scratch("lsp-redactions");
    let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    Response::Redactions(redactions::for_document(
        &analysed,
        PositionEncoding::Utf8,
        doc_comments,
    ))
    .render()
}

/// Whether a client whose `experimental` capability is `experimental` is
/// answered `docComment` ranges.
fn declared(experimental: serde_json::Value) -> bool {
    let capabilities: ClientCapabilities =
        serde_json::from_value(serde_json::json!({ "experimental": experimental }))
            .expect("the capabilities are the protocol's own shape");
    Settings::from_initialize(&InitializeParams {
        capabilities,
        ..InitializeParams::default()
    })
    .client
    .doc_comments
}

/// One range per `///` line, from the first slash to the last character of
/// the text. The trailing spaces and the `\r` of a CRLF line are not in it,
/// and an ordinary `//` comment has no range.
#[test]
fn a_doc_comment_line_is_answered_from_its_slashes_to_the_end_of_its_text() {
    assert_eq!(
        answered(
            "<?nvs\n/// Adds two numbers.   \r\n    /// Returns the sum.\n// not a doc comment\nclass Calc {}\n",
            true
        ),
        "2:1-2:22 docComment\n3:5-3:25 docComment\n"
    );
}

/// `rule:ide/redaction-ranges-come-from-the-server`: a client built before the
/// kind conceals every kind it does not know, so the kind is answered only to a
/// client that listed it in `experimental.redactionKinds`.
#[test]
fn a_doc_comment_is_answered_only_to_a_client_that_declared_the_kind() {
    let source = "<?nvs\n/// A calculator.\nclass Calc {}\n";
    assert_eq!(answered(source, false), "none\n");
    assert_eq!(answered(source, true), "2:1-2:18 docComment\n");
    assert!(declared(
        serde_json::json!({ "redactionKinds": [redactions::DOC_COMMENT] })
    ));
    assert!(!declared(
        serde_json::json!({ "redactionKinds": ["somethingElse"] })
    ));
    assert!(!declared(serde_json::json!({ "commands": [] })));
}

/// The ranges are the lexer's doc comments, so three slashes inside a string
/// or in inline HTML are text, and have no range.
#[test]
fn slashes_inside_a_string_or_inline_html_are_no_doc_comment() {
    assert_eq!(
        answered(
            "<?nvs\nvar $path = \"/// not a comment\nstill the string\";\necho $path;\n?>\n<p>/// plain text</p>\n",
            true
        ),
        "none\n"
    );
}

/// The two shapes `rule:ide/redaction-covers-bytes-only` names: a literal
/// token whose binding carries `secret`, and an interpolation slot whose
/// expression does — the second answered inside a statement that does not
/// compile, because `echo` of a `secret` is exactly where the value would
/// otherwise be on screen.
#[test]
fn a_secret_literal_and_a_secret_interpolation_slot_are_answered() {
    assert_eq!(
        concealed(
            "<?nvs\nsecret string $key = \"sk-live-abcdef\";\necho \"prefix $key suffix\";\n"
        ),
        "2:22-2:38 secretLiteral\n3:14-3:18 secretLiteral\n"
    );
}

/// Neither a plain literal nor a `tainted` one is concealed: `secret` alone is
/// a confidentiality claim, and `rule:security/tainted-qualifier`'s axis is
/// about trust — a value nobody may be shown is not the same as one nothing may
/// be told (`rule:security/secret-qualifier`).
///
/// The `tainted` declaration's *name* is answered, and that is the other kind:
/// a marker the client draws only where `nvs.taint.mark` asks for one, over the
/// six bytes of `$dirty` rather than the eleven of the literal beside it.
#[test]
fn a_plain_and_a_tainted_literal_are_not() {
    assert_eq!(
        concealed("<?nvs\nstring $plain = \"hello\";\ntainted string $dirty = \"untrusted\";\n"),
        "3:16-3:22 taintedDeclaration\n"
    );
}

/// Both kinds on one list, in document order
/// (`rule:ide/tainted-has-no-default-decoration` and
/// `rule:ide/redaction-ranges-come-from-the-server`).
///
/// The `secret` declaration's own name is answered for nothing — it carries no
/// `tainted` and a name is never concealed — so the two ranges here are the
/// marked name and the concealed literal below it, and nothing has to say which
/// list a client should have looked in.
#[test]
fn a_marked_declaration_and_a_concealed_literal_share_one_list() {
    assert_eq!(
        concealed(
            "<?nvs\ntainted string $dirty = \"untrusted\";\nsecret string $key = \"sk-live-abcdef\";\n"
        ),
        "2:16-2:22 taintedDeclaration\n3:22-3:38 secretLiteral\n"
    );
}

/// `rule:ide/redaction-ranges-come-from-the-server`'s named fail direction: the
/// initializer cannot be typed — it calls a free function `rule:classes/no-free-functions-or-constants` refuses
/// — and the literal is concealed anyway, because the declaration is what says
/// `secret` and it is stable while the value beside it is being written.
#[test]
fn an_untypable_expression_whose_binding_is_secret_is_answered_anyway() {
    assert_eq!(
        concealed("<?nvs\nsecret string $key = \"sk-live-\" . nope();\n"),
        "2:22-2:32 secretLiteral\n"
    );
}
