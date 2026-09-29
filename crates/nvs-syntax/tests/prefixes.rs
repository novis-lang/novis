//! Every prefix of every example is a document somebody is in the middle of
//! typing, and the parser owes it a tree.
//!
//! `rule:ide/the-tree-survives-a-syntax-error` is what an editor rests on: a
//! half-written file still parses, still answers where the caret is, and still
//! says what is missing. A file that only ever arrives whole cannot show that,
//! so this cuts `examples/*.nvs` at every token boundary — the states a typist
//! actually passes through — and asks all three questions of each cut.
//!
//! **The cut is a token boundary, not a byte**, because a byte inside a string
//! or an identifier is a state no editor buffer is in for longer than a
//! keystroke, and it would make the sweep quadratic in bytes rather than in
//! tokens. Lexing is deterministic over a prefix that ends where a token ends,
//! so the prefix's own tokens are the whole file's up to that point — which is
//! what lets [`boundaries`] carry the bracket depth out of one lex.

#![allow(
    clippy::print_stderr,
    reason = "the sweep's sizes are this test's report, as in `lossless.rs`"
)]

use std::path::PathBuf;

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_syntax::{Lexer, Parsed, TokenKind, parse, walk};

/// Every `examples/*.nvs`, sorted, so a failure names the same file on every
/// machine. The directory itself, not its subdirectories: M4B's acceptance
/// paragraph names that glob, and the nested directories are multi-file
/// programs whose parts are not documents on their own.
fn examples() -> Vec<(PathBuf, String)> {
    let dir = nvs_repo::path("examples");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("examples/ is in the repository")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("nvs"))
        .collect();
    paths.sort();
    assert!(
        paths.len() > 50,
        "the example corpus shrank to {} files — this sweep is measuring nothing",
        paths.len()
    );
    paths
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("an example is UTF-8");
            (path, text)
        })
        .collect()
}

/// Every token boundary of `text`, each with the bracket nesting depth a prefix
/// ending there is left inside.
///
/// The depth counts `(`, `[` and `{` written as code: a `{$…}` interpolation
/// site is its own token pair (`ComplexInterpOpen`), so a string never opens a
/// frame here. A prefix at a depth above zero is one this sweep calls
/// *genuinely incomplete* — no file ends inside a bracket.
fn boundaries(text: &str) -> Vec<(u32, u32)> {
    let mut map = SourceMap::new();
    let id = map.add("<prefix sweep>", text.to_string());
    let mut diags = Diagnostics::new();
    let mut lexer = Lexer::new(map.file(id));
    let mut out = Vec::new();
    let mut depth = 0_u32;
    loop {
        let token = lexer.next_token(&mut diags);
        if token.kind == TokenKind::Eof {
            break;
        }
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
        out.push((token.span.end, depth));
    }
    out
}

/// Parses `text` as a file of its own through the resilient entry point, and
/// returns the tree beside what it reported.
fn parse_prefix(name: &str, text: &str) -> (Parsed, Diagnostics) {
    let mut map = SourceMap::new();
    let id = map.add(name.to_string(), text.to_string());
    let mut diags = Diagnostics::new();
    let parsed = parse(map.file(id), &mut diags);
    (parsed, diags)
}

/// The first node in `nodes` whose span leaves the `len` bytes it was parsed
/// from, as a message naming it.
fn span_outside(nodes: &[walk::Node], len: u32) -> Option<String> {
    for node in nodes {
        if node.span.start > len || node.span.end > len {
            return Some(format!(
                "`{}` spans {}..{} of a {len}-byte prefix",
                node.kind, node.span.start, node.span.end
            ));
        }
        if let Some(found) = span_outside(&node.children, len) {
            return Some(found);
        }
    }
    None
}

#[test]
#[cfg_attr(
    miri,
    ignore = "only slow: the parser has no unsafe code, its unit tests run under Miri, and \
              parsing every prefix of every example takes Miri hours"
)]
fn every_prefix_of_every_example_parses_without_panicking() {
    let mut prefixes = 0_usize;
    let mut escaped = Vec::new();

    for (path, text) in examples() {
        for (end, _) in boundaries(&text) {
            let (parsed, _) = parse_prefix(&path.display().to_string(), &text[..end as usize]);
            prefixes += 1;
            // Not returning is most of the claim — a panic here fails the test
            // by itself. The rest is that recovery invented no position: a
            // `MemberName::Missing` or an `ExprKind::Error` carries the span of
            // what it stood in for (`rule:ide/recovery-is-explicit`), and what
            // it stood in for is inside the text the parser was handed.
            if let Some(found) = span_outside(&walk::of_stmts(&parsed.stmts), end) {
                escaped.push(format!("{}: {found}", path.display()));
            }
        }
    }

    assert!(
        escaped.is_empty(),
        "{} prefix(es) produced a node outside their own text:\n{}",
        escaped.len(),
        escaped.join("\n")
    );
    eprintln!("prefix sweep: {prefixes} prefix(es) parsed");
}

#[test]
#[cfg_attr(
    miri,
    ignore = "only slow: the parser has no unsafe code, its unit tests run under Miri, and \
              parsing every prefix of every example takes Miri hours"
)]
fn every_prefix_answers_a_syntax_index_lookup_at_its_end() {
    let mut answered = 0_usize;
    let mut broken = Vec::new();

    for (path, text) in examples() {
        // A file whose whole parse has no nodes at all has nothing for a lookup
        // to find in any prefix of it either, and the per-file assertion below
        // would be false for the wrong reason.
        let (whole, _) = parse_prefix(&path.display().to_string(), &text);
        if whole.index.is_empty() {
            continue;
        }

        let mut found_here = 0_usize;
        for (end, _) in boundaries(&text) {
            let (parsed, _) = parse_prefix(&path.display().to_string(), &text[..end as usize]);
            // The last byte the prefix contains. `at` is half-open, so `end`
            // itself is past every span by construction and would make this
            // vacuous; `end - 1` is the byte the caret sits after.
            let offset = end - 1;
            let path_at = parsed.index.at(offset);
            for (inner, outer) in path_at.nodes().iter().zip(path_at.nodes().iter().skip(1)) {
                if outer.span.start > inner.span.start || outer.span.end < inner.span.end {
                    broken.push(format!(
                        "{}: at {offset} of a {end}-byte prefix, `{}` {}..{} is not inside `{}` {}..{}",
                        path.display(),
                        inner.kind,
                        inner.span.start,
                        inner.span.end,
                        outer.kind,
                        outer.span.start,
                        outer.span.end
                    ));
                }
            }
            if let Some(node) = path_at.innermost() {
                if node.span.start > offset || node.span.end <= offset {
                    broken.push(format!(
                        "{}: at {offset}, `{}` {}..{} does not contain the offset",
                        path.display(),
                        node.kind,
                        node.span.start,
                        node.span.end
                    ));
                }
                found_here += 1;
            }
        }

        assert!(
            found_here > 0,
            "{}: no prefix of a file with {} node(s) answered a lookup at its end",
            path.display(),
            whole.index.len()
        );
        answered += found_here;
    }

    assert!(
        broken.is_empty(),
        "{} lookup(s) answered a path that is not a containment chain:\n{}",
        broken.len(),
        broken.join("\n")
    );
    eprintln!("prefix sweep: {answered} lookup(s) landed on a node");
}

#[test]
#[cfg_attr(
    miri,
    ignore = "only slow: the parser has no unsafe code, its unit tests run under Miri, and \
              parsing every prefix of every example takes Miri hours"
)]
fn an_incomplete_prefix_still_reports_a_diagnostic() {
    let mut incomplete = 0_usize;
    let mut silent = Vec::new();

    for (path, text) in examples() {
        for (end, depth) in boundaries(&text) {
            if depth == 0 {
                // Cutting at a closed bracket can leave a file that is simply
                // shorter and entirely valid, so there is nothing to demand.
                continue;
            }
            let (_, diags) = parse_prefix(&path.display().to_string(), &text[..end as usize]);
            incomplete += 1;
            if diags.is_empty() {
                silent.push(format!(
                    "{}: {end} bytes, inside {depth} bracket(s), reported nothing",
                    path.display()
                ));
            }
        }
    }

    assert!(
        silent.is_empty(),
        "{} prefix(es) ended inside a bracket and reported no diagnostic:\n{}",
        silent.len(),
        silent.join("\n")
    );
    assert!(
        incomplete > 100,
        "only {incomplete} prefix(es) were genuinely incomplete — the sweep is not exercising recovery"
    );
    eprintln!("prefix sweep: {incomplete} incomplete prefix(es) each reported a diagnostic");
}
