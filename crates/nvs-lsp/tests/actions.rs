//! Which fixes `textDocument/codeAction` offers, and which diagnostic each one
//! came from.
//!
//! Here rather than in a unit test for `tests/redactions.rs`' reason: the
//! answer is what a whole analysis reported, so a fixture would be asserting
//! that a `Suggestion` this file wrote survives being copied. Every expectation
//! below is `nvs_lsp::render`'s spelling
//! (`rule:ide/the-rendering-has-one-home`), so a `.lspt` case under
//! `tests/lsp/actions/` and a test here freeze the same text.
//!
//! The boundary these tests pin is
//! `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`: two
//! fixes ship because their replacement text is already sitting on a
//! diagnostic, and a fix the checker would have to compute is offered by
//! nothing at all.

use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{Analysed, Documents, Response, actions, analyse, uri_of};

/// `source` analysed as its own entry point, out of an open buffer alone —
/// nothing here `require`s anything, so no directory is needed
/// (`nvs_diagnostics::SourceMap::load`).
fn analysed(source: &str) -> Analysed {
    let uri =
        uri_of(&std::env::temp_dir().join("nvs-actions-case.nvs")).expect("a temp path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    analyse(&documents, &uri).expect("an open document analyses")
}

/// Where `needle` starts in `source`, as the cursor a client would send.
fn cursor(source: &str, needle: &str) -> u32 {
    let at = source
        .find(needle)
        .expect("the cursor's anchor is in the document");
    u32::try_from(at).expect("a test document is short")
}

/// What would be offered at `needle`, rendered as a case freezes it.
///
/// UTF-8 columns, which is what a `.lspt` case is read in: the encoding
/// negotiation is `rule:ide/positions-have-one-home`'s business and has nothing
/// to say about which fixes are offered.
fn offered(source: &str, needle: &str) -> String {
    let at = cursor(source, needle);
    let analysed = analysed(source);
    Response::CodeAction(actions::at(
        &analysed,
        &CompletionFiles::default(),
        at,
        at,
        actions::Kind::QuickFix,
        PositionEncoding::Utf8,
    ))
    .render()
}

/// Every suggestion the analysis of `source` reported, as `(title, text)`.
fn suggested(source: &str) -> Vec<(String, String)> {
    analysed(source)
        .diags
        .iter()
        .flat_map(|diagnostic| diagnostic.suggestions.iter())
        .map(|suggestion| (suggestion.message.clone(), suggestion.replacement.clone()))
        .collect()
}

/// The provenance claim itself: an action's title and its replacement are the
/// `Suggestion`'s own, not text this crate composed. Nothing here computes a
/// fix, which is why the boundary is enforceable at all — a fix with no
/// suggestion behind it has nothing to be translated from.
#[test]
fn a_code_action_comes_from_a_suggestion_the_diagnostic_already_carried() {
    let source = "<?nvs\nclass user_account {}\n";
    let carried = suggested(source);
    assert_eq!(
        carried,
        vec![(
            "rename to `UserAccount`".to_owned(),
            "UserAccount".to_owned()
        )]
    );

    let at = cursor(source, "user_account");
    let offered: Vec<(String, String)> = actions::at(
        &analysed(source),
        &CompletionFiles::default(),
        at,
        at,
        actions::Kind::QuickFix,
        PositionEncoding::Utf8,
    )
    .into_iter()
    .map(|action| (action.title, action.replacement))
    .collect();
    assert_eq!(offered, carried);
}

/// The first of the two that ship (`rule:core-api/identifier-casing`), frozen
/// whole — the range it edits is the name alone, so applying it renames and
/// touches nothing else.
///
/// A casing error is `E0110`, the parser's band, so it is published whatever
/// else the file has wrong with it and the fix is reachable on the file that
/// needs it (`rule:ide/diagnostics-are-phase-gated`).
#[test]
fn the_casing_diagnostic_carries_its_replacement() {
    assert_eq!(
        offered("<?nvs\nclass user_account {}\n", "user_account"),
        "2:7-2:19 quickfix rename to `UserAccount` -> \"UserAccount\"\n"
    );
}

/// The second (`rule:types/no-legacy-cast`): the whole `(int)$n` is replaced by
/// the one conversion spelling Novis has. The edit covers the cast as well as
/// its operand, because a rewrite that dropped `(int)` and left the operand
/// alone would silently change what the expression means.
#[test]
fn the_legacy_cast_diagnostic_carries_its_replacement() {
    assert_eq!(
        offered("<?nvs\nint $n = 3;\nint $m = (int)$n;\n", "(int)"),
        "3:10-3:17 quickfix rewrite as `$n as int` -> \"$n as int\"\n"
    );
}

/// `W1022` carries two groupings (`rule:expressions/misread-grouping-warns`).
/// A light bulb offers both, and a fix-all applies only the one that keeps
/// what the line does, because the other is an alternative a person chooses.
#[test]
fn a_fix_all_applies_the_grouping_that_keeps_the_meaning_and_never_the_alternative() {
    let source = "<?nvs\n?int $n = 5;\necho $n ?? 0 + 10;\n";
    let at = cursor(source, "0 + 10");
    let titles = |kind| -> Vec<String> {
        actions::at(
            &analysed(source),
            &CompletionFiles::default(),
            at,
            at,
            kind,
            PositionEncoding::Utf8,
        )
        .into_iter()
        .map(|action| action.title)
        .collect()
    };
    assert_eq!(
        titles(actions::Kind::QuickFix),
        [
            "apply `??` first: `($n ?? 0) + 10`",
            "keep the current meaning: `$n ?? (0 + 10)`"
        ]
    );
    assert_eq!(
        titles(actions::Kind::FixAll),
        ["keep the current meaning: `$n ?? (0 + 10)`"]
    );
}

/// The far side of the boundary, in both of its shapes.
///
/// `array<mixed>` narrowing (`rule:ide/narrow-an-annotation-to-its-literal`) is
/// the worked example: the type is derivable, the checker walks every element
/// already, and there is still nothing to offer here — no diagnostic carries
/// it, so this module has nothing to translate. A type error is the other
/// shape: it is published, the cursor is on it, and it names no replacement
/// because choosing one would be the checker computing a fix.
#[test]
fn a_fix_the_checker_would_have_to_compute_is_offered_by_nothing() {
    let narrowable = "<?nvs\narray<mixed> $rows = [1, 2, 3];\n";
    assert_eq!(suggested(narrowable), Vec::new());
    assert_eq!(offered(narrowable, "array<mixed>"), "none\n");

    let mistyped = "<?nvs\nint $n = \"text\";\n";
    assert!(
        !nvs_lsp::for_document(
            &analysed(mistyped),
            &CompletionFiles::default(),
            nvs_lsp::Phases::Gated,
            PositionEncoding::Utf8
        )
        .is_empty(),
        "the document has a published diagnostic for the cursor to be on"
    );
    assert_eq!(offered(mistyped, "\"text\""), "none\n");
}
