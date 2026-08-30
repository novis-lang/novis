//! Corpus-parse smoke test: M1's plan requires parsing a full real-world PHP
//! corpus without the parser crashing (it is not required to accept every
//! construct cleanly — that is M2's job, and `nvs-syntax`'s module docs track
//! the known gaps that will surface here first).
//!
//! The corpus itself is not part of this repo (see `.gitignore`'s `/php-src`
//! entry) — point `NVS_PHP_CORPUS` at a directory of `.php` files, or drop one
//! at `<workspace-root>/php-src`. Absent either, the test is skipped rather
//! than failed, since CI has no corpus checked in.
//!
//! **What this test no longer measures, and why.** It used to report how many
//! corpus files parsed with *zero* diagnostics. That number is meaningless
//! now and is not tracked: every corpus file opens with `<?php`, which
//! [ADR 0049](../../../docs/adr/0049-single-open-tag-and-single-exit-keyword.md)
//! rejects in favour of `<?nvs`, so every one of them trips `E0229`;
//! [ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
//! made keyword matching exact, so a corpus file's mixed-case `IF`/`TRUE`
//! now lex as ordinary identifiers; and
//! [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
//! § 1 makes every `===`/`!==` in the corpus an `E0232`. All three widenings
//! are deliberate — the bar
//! this test holds is "the parser does not panic," which is what M1's plan
//! actually asked for.
//!
//! **Why the files are parsed on every core.** This is one `#[test]`, so
//! libtest's own parallelism does nothing for it, and serially it was 20 s on
//! a 16-thread box — the single slowest test binary in the workspace, paid by
//! every `verify.py` and by every acceptance check the loop runs. The parser
//! holds no global state and a `SourceMap` is per file, so the workers share
//! nothing but the read-only file list and the counter that hands out the
//! next index; each keeps its own [`Tally`] and the test merges them. The
//! report is the one the serial loop printed, in the same order: the file
//! list is sorted first, so neither `read_dir`'s order nor which worker got
//! which file can change a line of it.

#![allow(
    clippy::print_stderr,
    reason = "this test's whole job is reporting a human-readable summary"
)]

use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_syntax::parse_file;

fn corpus_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("NVS_PHP_CORPUS") {
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

/// One worker's count of what it parsed. Each worker keeps its own and the
/// test merges them once every worker has joined.
#[derive(Default)]
struct Tally {
    clean: usize,
    with_diagnostics: usize,
    unreadable: Vec<String>,
    panicked: Vec<String>,
    /// Which diagnostic code the corpus tripped, how often, one example
    /// message — the fastest way to tell "one known gap hit 300 times" from
    /// "300 distinct problems" without reading every file by hand — and the
    /// index of the file the example came from. The index is what keeps the
    /// example the same from run to run: which worker saw a code first is a
    /// scheduling accident, so the merge keeps the earliest file's.
    by_code: HashMap<String, (usize, String, usize)>,
}

impl Tally {
    /// Parse one corpus file and count what happened. The panic hook is
    /// already silenced by the caller, for every worker at once.
    fn parse(&mut self, index: usize, path: &Path) {
        let mut map = SourceMap::new();
        // Real-world PHP is not always UTF-8 -- Symfony alone ships a class named with the
        // latin-1 byte 0xA9 and a deliberately binary string fixture -- while `SourceMap::load`
        // reads UTF-8 only. A file that cannot be read is not a parser panic, which is the one
        // thing this test exists to catch, so count it and carry on. Panicking here instead
        // failed the whole run on the first such file, and the hook silenced by the caller ate
        // the message: a bare `FAILED` with no summary and no filename to chase.
        let id = match map.load(path) {
            Ok(id) => id,
            Err(err) => {
                self.unreadable.push(format!("{}: {err}", path.display()));
                return;
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
            Ok(entries) if entries.is_empty() => self.clean += 1,
            Ok(entries) => {
                self.with_diagnostics += 1;
                for (code, message) in entries {
                    let slot = self.by_code.entry(code).or_insert((0, message, index));
                    slot.0 += 1;
                }
            }
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "<non-string panic payload>".to_string());
                self.panicked.push(format!("{}: {msg}", path.display()));
            }
        }
    }

    /// Fold another worker's tally into this one.
    fn absorb(&mut self, other: Tally) {
        self.clean += other.clean;
        self.with_diagnostics += other.with_diagnostics;
        self.unreadable.extend(other.unreadable);
        self.panicked.extend(other.panicked);
        for (code, (count, message, at)) in other.by_code {
            match self.by_code.get_mut(&code) {
                Some(slot) => {
                    slot.0 += count;
                    if at < slot.2 {
                        slot.1 = message;
                        slot.2 = at;
                    }
                }
                None => {
                    self.by_code.insert(code, (count, message, at));
                }
            }
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
            "skipping corpus-parse test: no corpus at $NVS_PHP_CORPUS or <workspace-root>/php-src"
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
    // `read_dir` hands files back in the filesystem's order. Sorted, the report below and the
    // example message kept per code read the same on every run and on every machine.
    files.sort();

    // The default hook would print a full backtrace-style panic message for
    // every corpus file that trips one, before this test gets to report its
    // own summary. Silence it for the duration and report just the payloads.
    // The hook is process-wide, so one silence covers every worker.
    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let workers = thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(files.len());
    let next = AtomicUsize::new(0);
    let mut tally = thread::scope(|scope| {
        // Collected before any is joined: a lazy `map` here would spawn and join one worker at
        // a time, which is the serial loop again with extra steps.
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Tally::default();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = files.get(index) else {
                            break;
                        };
                        mine.parse(index, path);
                    }
                    mine
                })
            })
            .collect();
        let mut all = Tally::default();
        for handle in handles {
            all.absorb(
                handle
                    .join()
                    .expect("a corpus worker catches every panic per file"),
            );
        }
        all
    });
    panic::set_hook(prev_hook);
    tally.unreadable.sort();
    tally.panicked.sort();

    eprintln!(
        "corpus-parse: {} files on {workers} thread(s), {} parsed clean, {} produced diagnostics, \
         {} unreadable, {} panicked",
        files.len(),
        tally.clean,
        tally.with_diagnostics,
        tally.unreadable.len(),
        tally.panicked.len()
    );
    for entry in &tally.unreadable {
        eprintln!("  unreadable: {entry}");
    }
    let mut by_code: Vec<(String, usize, String)> = tally
        .by_code
        .into_iter()
        .map(|(code, (count, message, _))| (code, count, message))
        .collect();
    by_code.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (code, count, message) in &by_code {
        eprintln!("  {count:>4}x {code}: {message}");
    }

    assert!(
        tally.panicked.is_empty(),
        "parser panicked on {} of {} corpus files:\n{}",
        tally.panicked.len(),
        files.len(),
        tally.panicked.join("\n")
    );
}
