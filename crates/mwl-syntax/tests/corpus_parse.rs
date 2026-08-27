//! Corpus-parse smoke test: M1's plan requires parsing a full real-world PHP
//! corpus without the parser crashing (it is not required to accept every
//! construct cleanly — that is M2's job, and `mwl-syntax`'s module docs track
//! the known gaps that will surface here first).
//!
//! The corpus itself is not part of this repo (see `.gitignore`'s `/php-src`
//! entry) — point `MWL_PHP_CORPUS` at a directory of `.php` files, or drop one
//! at `<workspace-root>/php-src`. Absent either, the test is skipped rather
//! than failed, since CI has no corpus checked in.
//!
//! **What this test no longer measures, and why.** It used to report how many
//! corpus files parsed with *zero* diagnostics. That number is meaningless
//! now and is not tracked: every corpus file opens with `<?php`, which
//! [ADR 0049](../../../docs/adr/0049-single-open-tag-and-single-exit-keyword.md)
//! rejects in favour of `<?mwl`, so every one of them trips `E0229`;
//! [ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
//! made keyword matching exact, so a corpus file's mixed-case `IF`/`TRUE`
//! now lex as ordinary identifiers; and
//! [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
//! § 1 makes every `===`/`!==` in the corpus an `E0232`. All three widenings
//! are deliberate — the bar
//! this test holds is "the parser does not panic," which is what M1's plan
//! actually asked for.

#![allow(
    clippy::print_stderr,
    reason = "this test's whole job is reporting a human-readable summary"
)]

use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use mwl_diagnostics::{Diagnostics, SourceMap};
use mwl_syntax::parse_file;

fn corpus_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("MWL_PHP_CORPUS") {
        return Some(PathBuf::from(dir));
    }
    let default = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../php-src");
    default.is_dir().then_some(default)
}

fn collect_php_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_php_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("php"))
        {
            out.push(path);
        }
    }
}

/// Not `#[test]`-gated on the corpus existing (`cargo test` would then just
/// silently do nothing useful), but does nothing *but* print a skip notice
/// when it's absent, since the corpus is a local, gitignored fixture.
#[test]
fn corpus_parses_without_panicking() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "skipping corpus-parse test: no corpus at $MWL_PHP_CORPUS or <workspace-root>/php-src"
        );
        return;
    };

    let mut files = Vec::new();
    collect_php_files(&dir, &mut files);
    assert!(
        !files.is_empty(),
        "corpus at {} has no .php files",
        dir.display()
    );

    let mut panicked = Vec::new();
    // A corpus file `SourceMap` cannot read at all, which is a property of the corpus rather
    // than of the parser -- see the loop below for why it is counted instead of fatal.
    let mut unreadable = Vec::new();
    let mut clean = 0usize;
    let mut with_diagnostics = 0usize;
    // Which diagnostic code the corpus tripped, how often, and one example
    // message — the fastest way to tell "one known gap hit 300 times" from
    // "300 distinct problems" without reading every file by hand.
    let mut by_code: HashMap<String, (usize, String)> = HashMap::new();

    // The default hook would print a full backtrace-style panic message for
    // every corpus file that trips one, before this test gets to report its
    // own summary. Silence it for the duration and report just the payloads.
    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    for path in &files {
        let mut map = SourceMap::new();
        // Real-world PHP is not always UTF-8 -- Symfony alone ships a class named with the
        // latin-1 byte 0xA9 and a deliberately binary string fixture -- while `SourceMap::load`
        // reads UTF-8 only. A file that cannot be read is not a parser panic, which is the one
        // thing this test exists to catch, so count it and carry on. Panicking here instead
        // failed the whole run on the first such file, and the hook silenced just above ate the
        // message: a bare `FAILED` with no summary and no filename to chase.
        let id = match map.load(path) {
            Ok(id) => id,
            Err(err) => {
                unreadable.push(format!("{}: {err}", path.display()));
                continue;
            }
        };
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut diags = Diagnostics::new();
            let _ = parse_file(map.file(id), &mut diags);
            let entries: Vec<(String, String)> = diags
                .iter()
                .map(|d| {
                    let code = d
                        .code
                        .map_or_else(|| "<no code>".to_string(), |c| c.to_string());
                    (code, d.message.clone())
                })
                .collect();
            entries
        }));
        match result {
            Ok(entries) if entries.is_empty() => clean += 1,
            Ok(entries) => {
                with_diagnostics += 1;
                for (code, message) in entries {
                    let slot = by_code.entry(code).or_insert((0, message));
                    slot.0 += 1;
                }
            }
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "<non-string panic payload>".to_string());
                panicked.push(format!("{}: {msg}", path.display()));
            }
        }
    }
    panic::set_hook(prev_hook);

    eprintln!(
        "corpus-parse: {} files, {clean} parsed clean, {with_diagnostics} produced diagnostics, \
         {} unreadable, {} panicked",
        files.len(),
        unreadable.len(),
        panicked.len()
    );
    for entry in &unreadable {
        eprintln!("  unreadable: {entry}");
    }
    let mut by_code: Vec<(String, usize, String)> = by_code
        .into_iter()
        .map(|(code, (count, message))| (code, count, message))
        .collect();
    by_code.sort_by_key(|(_, count, _)| std::cmp::Reverse(*count));
    for (code, count, message) in &by_code {
        eprintln!("  {count:>4}x {code}: {message}");
    }

    assert!(
        panicked.is_empty(),
        "parser panicked on {} of {} corpus files:\n{}",
        panicked.len(),
        files.len(),
        panicked.join("\n")
    );
}
