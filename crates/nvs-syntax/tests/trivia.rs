//! The trivia layer: what the lexer keeps when it is asked to keep it, and
//! which comment among those is documentation.
//!
//! `rule:ide/one-grammar-one-tree` puts a `Trivia` beside every token;
//! `rule:tooling/doc-comment-is-three-slashes` says which of them a stage past
//! the lexer reads. These are the cases for the second, and every one of them
//! is about the *opening run* — the only part of a comment its kind depends on.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_syntax::{Lexer, TokenKind, TriviaKind};

/// Lexes `src` to end of input and returns every trivium it kept, as its kind
/// and the exact bytes it covers. A diagnostic here means the case is wrong,
/// not the layer, so it fails loudly rather than being returned.
fn trivia_of(src: &str) -> Vec<(TriviaKind, String)> {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let mut lexer = Lexer::with_trivia(map.file(id));
    while lexer.next_token(&mut diags).kind != TokenKind::Eof {}
    assert!(
        !diags.has_errors(),
        "unexpected diagnostics for {src:?}: {diags:?}"
    );
    lexer
        .take_trivia()
        .iter()
        .map(|t| {
            let text = map
                .file(id)
                .span_text(t.span)
                .expect("a trivium's span is inside the file it came from");
            (t.kind, text.to_string())
        })
        .collect()
}

/// The comments alone, in source order. Whitespace is trivia too and is what
/// makes the stream lossless, but it says nothing about a comment's kind.
fn comments_of(src: &str) -> Vec<(TriviaKind, String)> {
    trivia_of(src)
        .into_iter()
        .filter(|(kind, _)| *kind != TriviaKind::Whitespace)
        .collect()
}

#[test]
fn three_slashes_lex_as_a_doc_comment() {
    let comments = comments_of("<?nvs\n/// The price in cents.\nclass A {}\n");
    assert_eq!(
        comments,
        vec![(
            TriviaKind::DocComment,
            "/// The price in cents.".to_string()
        )]
    );
}

#[test]
fn two_slashes_lex_as_a_line_comment() {
    // The second line is the encoding case's own shape: a `////` inside the
    // text of an ordinary comment, which the opening run has already decided.
    let comments = comments_of("<?nvs\n// ordinary\n// `ffffff` is `////`\n");
    assert_eq!(comments.len(), 2);
    assert!(
        comments
            .iter()
            .all(|(kind, _)| *kind == TriviaKind::LineComment),
        "{comments:?}"
    );
}

#[test]
fn four_or_more_slashes_lex_as_a_line_comment() {
    // A sweep, asserted by counting: a lexer that reads three slashes and stops
    // looking answers plausibly on any single row of this.
    let mut kinds = Vec::new();
    for slashes in 4..=8 {
        let src = format!("<?nvs\n{} a divider\n", "/".repeat(slashes));
        kinds.extend(comments_of(&src).into_iter().map(|(kind, _)| kind));
    }
    assert_eq!(kinds.len(), 5);
    assert!(
        kinds.iter().all(|kind| *kind == TriviaKind::LineComment),
        "{kinds:?}"
    );

    // The bound from below, named beside the refusals: one slash fewer is the
    // one length that documents anything.
    assert_eq!(
        comments_of("<?nvs\n/// a doc comment\n")[0].0,
        TriviaKind::DocComment
    );
}

#[test]
fn a_hash_comment_is_never_a_doc_comment() {
    for src in ["<?nvs\n# one\n", "<?nvs\n## two\n", "<?nvs\n### three\n"] {
        let comments = comments_of(src);
        assert_eq!(comments.len(), 1, "{src:?}");
        assert_eq!(comments[0].0, TriviaKind::LineComment, "{src:?}");
    }

    // A shebang line is lexed as the `#` comment it already is
    // (`rule:tooling/shebang-opens-code-mode`), so it is ordinary at the one
    // offset where a run length could plausibly have meant something else.
    assert_eq!(
        comments_of("#!/usr/bin/env nvs\nclass A {}\n")[0],
        (TriviaKind::LineComment, "#!/usr/bin/env nvs".to_string())
    );
}

#[test]
fn a_block_comment_keeps_its_own_kind() {
    // PHPDoc's `/**` is the trap: it opens a block comment, and no run of stars
    // makes one documentation — `///` is the only spelling that does.
    let comments = comments_of("<?nvs\n/* plain */\n/** phpdoc-shaped */\n");
    assert_eq!(comments.len(), 2);
    assert!(
        comments
            .iter()
            .all(|(kind, _)| *kind == TriviaKind::BlockComment),
        "{comments:?}"
    );
    assert_eq!(comments[1].1, "/** phpdoc-shaped */");
}

/// The corpus case the run-length rule was written against. Its `////` is in
/// two places and neither is a comment opening: inside the text of a `//`
/// comment, and in the expected output of the base64url row. So the case
/// carries no documentation at all, and reclassification cannot reach it.
#[test]
fn the_four_slash_divider_in_the_encoding_case_stays_ordinary() {
    const CASE: &str = include_str!(
        "../../../tests/conformance/core/encoding-every-encoder-agrees-with-its-own-decoder-over-a-table.nvst"
    );
    let source = CASE
        .split_once("--FILE--\n")
        .expect("the case names a --FILE-- section")
        .1
        .split_once("\n--EXPECT--")
        .expect("the case names an --EXPECT-- section")
        .0;

    let comments = comments_of(source);
    assert!(
        comments.iter().any(|(_, text)| text.contains("////")),
        "the case no longer carries the `////` this test is about"
    );
    assert!(
        comments
            .iter()
            .all(|(kind, _)| *kind == TriviaKind::LineComment),
        "{comments:?}"
    );
}
