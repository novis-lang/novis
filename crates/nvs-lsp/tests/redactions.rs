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

use nvs_diagnostics::PositionEncoding;
use nvs_lsp::{Documents, Response, analyse, redactions, uri_of};

/// What the server would answer for `source`, rendered as a case freezes it.
///
/// UTF-8 columns, which is what a `.lspt` case is read in: the encoding
/// negotiation is `rule:ide/positions-have-one-home`'s business and has nothing
/// to say about which ranges are answered.
fn concealed(source: &str) -> String {
    let dir = nvs_repo::scratch("lsp-redactions");
    let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    Response::Redactions(redactions::for_document(&analysed, PositionEncoding::Utf8)).render()
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

/// ADR 0101 § 1's named fail direction: the initializer cannot be typed — it
/// calls a free function `rule:classes/no-free-functions-or-constants` refuses
/// — and the literal is concealed anyway, because the declaration is what says
/// `secret` and it is stable while the value beside it is being written.
#[test]
fn an_untypable_expression_whose_binding_is_secret_is_answered_anyway() {
    assert_eq!(
        concealed("<?nvs\nsecret string $key = \"sk-live-\" . nope();\n"),
        "2:22-2:32 secretLiteral\n"
    );
}
