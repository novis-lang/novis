//! No text a diagnostic prints names a rule id or a form of "refuse"
//! (AGENTS.md § *Text an end user reads*).
//!
//! A diagnostic is read by somebody who has never seen this repository, so a
//! `rule:` id is a reference they cannot follow, and "refused" is this
//! repository's word for "is not allowed" or "does not compile". Two places
//! hold that text, and this test reads both:
//!
//! - every card in [`code::ALL`], which `nvs agent show <code>` prints;
//! - every string literal in the source of the crates that build compile-time
//!   diagnostics: a message, a label, a help and a note are each one.
//!
//! A literal is skipped where it is not printed to a user: inside `assert!`,
//! `panic!`, `unreachable!`, `.expect(…)` or an `#[expect(…)]`, and in the
//! `#[cfg(test)]` tail of a file. Comments are not literals.

use std::fs;
use std::path::Path;

use nvs_diagnostics::code;

/// The crates whose source builds a compile-time diagnostic.
const DIAGNOSTIC_CRATES: &[&str] = &[
    "crates/nvs-syntax/src",
    "crates/nvs-hir/src",
    "crates/nvs-types/src",
    "crates/nvs-config/src",
    "crates/nvs-diagnostics/src",
];

/// Files in those crates that print no diagnostic. `audit.rs` renders
/// `nvs config dump`, which names the address-policy rule beside
/// `resolve = "proxy"` as `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`
/// says.
const NOT_DIAGNOSTICS: &[&str] = &["audit.rs"];

/// A call or macro whose string arguments are never printed to a user.
const NOT_PRINTED: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "expect",
    "expect_err",
    "allow",
];

/// Whether `text` names a rule id or a form of "refuse".
fn cites(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("refus")
        || lower
            .match_indices("rule:")
            .any(|(at, _)| lower[at + 5..].starts_with(|c: char| c.is_ascii_lowercase()))
}

/// Each string literal in `source` that is printed, as its line and its text.
///
/// A small lexer: it skips comments and character literals, reads plain and
/// raw strings, and keeps a stack of open brackets with the name written
/// before each, so it knows when a literal sits inside a call in
/// [`NOT_PRINTED`].
fn printed_literals(source: &str) -> Vec<(usize, String)> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut open: Vec<bool> = Vec::new();
    let mut last_word = String::new();
    let mut line = 1;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    line += usize::from(bytes[i] == b'\n');
                    i += 1;
                }
                i += 2;
            }
            b'\'' => {
                // A character literal is `'x'` or `'\…'`; a lifetime has no closing quote.
                if bytes.get(i + 1) == Some(&b'\\') {
                    i += 3;
                    while i < bytes.len() && bytes[i] != b'\'' {
                        i += 1;
                    }
                    i += 1;
                } else if bytes.get(i + 2) == Some(&b'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            b'"' | b'r' | b'b'
                if b == b'"'
                    || (b == b'r' && matches!(bytes.get(i + 1), Some(b'"' | b'#')))
                    || (b == b'b' && bytes.get(i + 1) == Some(&b'"')) =>
            {
                if b == b'r' || b == b'b' {
                    if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
                        last_word.push(char::from(b));
                        i += 1;
                        continue;
                    }
                    i += 1;
                }
                let mut hashes = 0;
                while bytes.get(i) == Some(&b'#') {
                    hashes += 1;
                    i += 1;
                }
                if bytes.get(i) != Some(&b'"') {
                    continue;
                }
                let raw = b == b'r';
                let start_line = line;
                i += 1;
                let begin = i;
                loop {
                    if i >= bytes.len() {
                        break;
                    }
                    if !raw && bytes[i] == b'\\' {
                        line += usize::from(bytes.get(i + 1) == Some(&b'\n'));
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"'
                        && bytes[i + 1..]
                            .iter()
                            .take(hashes)
                            .filter(|&&h| h == b'#')
                            .count()
                            == hashes
                    {
                        break;
                    }
                    line += usize::from(bytes[i] == b'\n');
                    i += 1;
                }
                let text = &source[begin..i.min(bytes.len())];
                i += 1 + hashes;
                if !open.iter().any(|&skipped| skipped) {
                    out.push((start_line, text.to_owned()));
                }
                last_word.clear();
            }
            b'(' | b'[' | b'{' => {
                let name = last_word.trim_end_matches('!');
                open.push(NOT_PRINTED.contains(&name));
                last_word.clear();
                i += 1;
            }
            b')' | b']' | b'}' => {
                open.pop();
                last_word.clear();
                i += 1;
            }
            _ if b.is_ascii_alphanumeric() || b == b'_' || b == b'!' => {
                if !(i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')) {
                    last_word.clear();
                }
                last_word.push(char::from(b));
                i += 1;
            }
            _ if b.is_ascii_whitespace() => i += 1,
            _ => {
                last_word.clear();
                i += 1;
            }
        }
    }
    out
}

/// The `.rs` files under `dir` that are not tests.
fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if entry.file_name() != "tests" {
                sources(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs")
            && !path.file_stem().is_some_and(|s| s == "tests")
            && !NOT_DIAGNOSTICS.contains(&entry.file_name().to_string_lossy().as_ref())
        {
            out.push(path);
        }
    }
}

#[test]
fn no_card_names_a_rule_or_a_refusal() {
    let wrong: Vec<String> = code::ALL
        .iter()
        .filter_map(|&c| {
            code::card(c)
                .filter(|card| cites(card))
                .map(|card| format!("{c}: {card}"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "these cards cite a rule or say \"refuse\":\n{}",
        wrong.join("\n")
    );
}

#[cfg_attr(
    miri,
    ignore = "reads the source of every crate that builds a diagnostic, which Miri interprets for longer than CI's miri job may run, and a text scan has no undefined behaviour of its own to find"
)]
#[test]
fn no_diagnostic_text_names_a_rule_or_a_refusal() {
    let mut files = Vec::new();
    for dir in DIAGNOSTIC_CRATES {
        sources(&nvs_repo::path(dir), &mut files);
    }
    assert!(
        files.len() > 50,
        "the walk found the crates' sources: {}",
        files.len()
    );
    let mut wrong = Vec::new();
    let mut read = 0;
    for path in files {
        let source = fs::read_to_string(&path).expect("a source file is text");
        let shipped = source.split("#[cfg(test)]").next().unwrap_or_default();
        let literals = printed_literals(shipped);
        read += literals.len();
        for (line, text) in literals {
            if cites(&text) {
                wrong.push(format!("{}:{line}: {text}", path.display()));
            }
        }
    }
    assert!(read > 5000, "the scan read every printed literal: {read}");
    assert!(
        wrong.is_empty(),
        "these texts cite a rule or say \"refuse\"; write what is not allowed instead:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn the_scan_finds_a_literal_and_skips_what_is_not_printed() {
    let source = "fn f() {\n    help(\"see `rule:types/declaration`\");\n    assert!(ok, \"refused\");\n    \
                  x.expect(\"refused\");\n    // \"refused\"\n    let c = '\\'';\n    note(r#\"is refused\"#);\n}\n";
    let found: Vec<(usize, bool)> = printed_literals(source)
        .into_iter()
        .map(|(line, text)| (line, cites(&text)))
        .collect();
    assert_eq!(found, vec![(2, true), (7, true)]);
}
