//! The trivia layer: what the lexer keeps when it is asked to keep it, and
//! which comment among those is documentation.
//!
//! `rule:ide/one-grammar-one-tree` puts a `Trivia` beside every token;
//! `rule:tooling/doc-comment-is-three-slashes` says which of them a stage past
//! the lexer reads. These are the cases for the second, and every one of them
//! is about the *opening run* — the only part of a comment its kind depends on.

use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostics, SourceMap, code};
use nvs_syntax::{Lexer, TokenKind, TriviaKind, parse_file};

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

/// Every file under `dir` whose extension is in `exts`, so a failure names the
/// same file on every machine.
fn collect(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, exts, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.contains(&e))
        {
            out.push(path);
        }
    }
}

/// The Novis source inside a corpus file: a `.nvs` file is source outright, and
/// a `.nvst` case is its `--FILE--` section, since every other section is the
/// harness's rather than the language's.
///
/// `None` for a case whose expectation is a compile error, because such a case
/// *states* a diagnostic rather than carrying one — a `///` that documents
/// nothing is the whole subject of two of them, and reading those as corpus
/// would make this file's sweep assert the opposite of what they pin.
fn source_of(path: &Path, text: &str) -> Option<String> {
    if path.extension().and_then(|e| e.to_str()) != Some("nvst") {
        return Some(text.to_string());
    }
    if text
        .lines()
        .any(|line| line == "--EXPECT-ERROR--" || line == "--EXPECTF-ERROR--")
    {
        return None;
    }
    let body = text.split_once("--FILE--\n")?.1;
    let mut source = String::new();
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed.len() > 4 && trimmed.starts_with("--") && trimmed.ends_with("--") {
            break;
        }
        source.push_str(line);
    }
    Some(source)
}

/// The corpus written before `///` meant anything, and the claim that giving it
/// a meaning cost no edit.
///
/// Reclassification is retroactive: the day the lexer began answering
/// [`TriviaKind::DocComment`], every `///` already on disk became documentation
/// and had to be in a position that documents something. So this walks the
/// whole corpus, keeps the files a `///` opens a comment in, and asserts that
/// none of them reports [`code::E_DOC_COMMENT_UNATTACHED`] or
/// [`code::E_DOC_COMMENT_UNKNOWN_TAG`] — that the run landed on a declaration,
/// and that whatever `@tag` it carried was already one of the two.
///
/// It is a sweep asserted by counting rather than a list of paths, so a case
/// added later is covered the day it lands; the floor keeps a corpus that
/// stopped carrying doc comments from passing silently. A case that expects a
/// compile error is not corpus for this purpose ([`source_of`]).
#[test]
fn the_existing_triple_slash_comments_attach_to_their_declarations() {
    let mut paths = Vec::new();
    collect(&nvs_repo::path("examples"), &["nvs"], &mut paths);
    collect(&nvs_repo::path("tests"), &["nvs", "nvst"], &mut paths);
    paths.sort();

    let mut documented = Vec::new();
    let mut loose = Vec::new();
    for path in &paths {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Some(source) = source_of(path, &text) else {
            continue;
        };
        if !source.contains("///") {
            continue;
        }

        let mut map = SourceMap::new();
        let id = map.add(path.display().to_string(), &source);

        // The lexer's half. A `///` inside a string literal or in the middle of
        // an ordinary comment is not an opening run, so the trivia layer is
        // what decides whether this file carries documentation at all.
        let mut diags = Diagnostics::new();
        let mut lexer = Lexer::with_trivia(map.file(id));
        while lexer.next_token(&mut diags).kind != TokenKind::Eof {}
        let docs = lexer
            .trivia()
            .iter()
            .filter(|t| t.kind == TriviaKind::DocComment)
            .count();
        if docs == 0 {
            continue;
        }
        documented.push(path.display().to_string());

        // The parser's half, over the same source and with no trivia asked for,
        // because attachment is the grammar's own.
        let mut diags = Diagnostics::new();
        let _stmts = parse_file(map.file(id), &mut diags);
        let reported = diags
            .iter()
            .filter(|d| {
                d.code == Some(code::E_DOC_COMMENT_UNATTACHED)
                    || d.code == Some(code::E_DOC_COMMENT_UNKNOWN_TAG)
            })
            .count();
        if reported > 0 {
            loose.push(format!("{}: {reported} of {docs}", path.display()));
        }
    }

    assert!(
        documented.len() >= 7,
        "only {} corpus file(s) carry a doc comment — this test is measuring nothing:\n{}",
        documented.len(),
        documented.join("\n")
    );
    assert!(
        loose.is_empty(),
        "{} corpus file(s) carry a `///` that documents nothing:\n{}",
        loose.len(),
        loose.join("\n")
    );
}
