//! Losslessness: every token and every trivium, in offset order, are the file.
//!
//! `rule:ide/tokens-plus-trivia-reproduce-the-file` is a property of the whole
//! corpus rather than an assertion in the lexer, so this is where it is
//! checked: `examples/`, and every `.nvs` and `.nvst` case under `tests/`. A
//! formatter rests on this (`rule:tooling/fmt-quotes` promises comments
//! survive), and so does anything that reads a `///`.
//!
//! **The check is tiling, not concatenation, and they are the same claim.** The
//! spans are walked in order and each must begin exactly where the last one
//! ended: a gap is a byte the lexer dropped, an overlap is a byte it counted
//! twice, and either one makes the concatenation differ. Tiling names *where*,
//! which a string comparison over a 4,000-line file does not.

#![allow(
    clippy::print_stderr,
    reason = "the corpus sizes are this test's report"
)]

use std::panic::resume_unwind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use nvs_diagnostics::{Diagnostics, SourceFile, SourceMap};
use nvs_syntax::{Lexer, TokenKind};

/// Every file under `dir` whose extension is in `exts`, sorted, so a failure
/// names the same file on every machine.
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

/// The `--FILE--` section of a `.nvst` case, which is the only part of one that
/// is Novis source. `None` when the case has no such section.
fn case_source(text: &str) -> Option<String> {
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

/// Lexes `file` keeping its trivia and returns where the pieces stop tiling it,
/// as a message naming the offset and what sits at it.
fn first_hole(file: &SourceFile) -> Option<String> {
    let mut diags = Diagnostics::new();
    let mut lexer = Lexer::with_trivia(file);
    let mut pieces = Vec::new();
    loop {
        let token = lexer.next_token(&mut diags);
        pieces.push((
            token.span.start,
            token.span.end,
            format!("{:?}", token.kind),
        ));
        if token.kind == TokenKind::Eof {
            break;
        }
    }
    for trivium in lexer.trivia() {
        pieces.push((
            trivium.span.start,
            trivium.span.end,
            format!("{:?}", trivium.kind),
        ));
    }
    pieces.sort_by_key(|(start, end, _)| (*start, *end));

    let text = file.text();
    let mut cursor = 0_u32;
    for (start, end, what) in &pieces {
        if *start != cursor {
            let (kind, from, to) = if *start > cursor {
                ("dropped", cursor, *start)
            } else {
                ("counted twice", *start, cursor)
            };
            let bytes = &text[from as usize..to as usize];
            return Some(format!(
                "{kind} {} byte(s) at offset {from} ({bytes:?}), before the {what} at {start}..{end}",
                to - from
            ));
        }
        cursor = cursor.max(*end);
    }
    let len = u32::try_from(text.len()).expect("a source file is under 4 GiB");
    (cursor != len).then(|| format!("dropped the {} byte(s) after offset {cursor}", len - cursor))
}

/// The line naming where `path`'s pieces stop tiling it, or `None` when they tile
/// it or it holds no Novis source. `to_source` turns a file's bytes into the
/// Novis source inside it.
fn hole_in(path: &Path, to_source: &impl Fn(&str) -> Option<String>) -> Option<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        // A file that is not UTF-8 has no text for the lexer to tile.
        return None;
    };
    let source = to_source(&text)?;
    let mut map = SourceMap::new();
    let id = map.add(path.display().to_string(), &source);
    first_hole(map.file(id)).map(|hole| format!("{}: {hole}", path.display()))
}

/// Reports every file in `paths` whose pieces do not tile it, one line each, in
/// `paths`' order.
///
/// The files are shared out over every core, because libtest runs this test on
/// one thread. Each file is lexed into its own `SourceMap`, so the workers share
/// nothing but the index of the next file, and sorting by that index gives back
/// the order a single thread would report in.
fn holes_in(paths: &[PathBuf], to_source: impl Fn(&str) -> Option<String> + Sync) -> Vec<String> {
    let workers = thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(paths.len());
    let next = AtomicUsize::new(0);
    let mut found: Vec<(usize, String)> = thread::scope(|scope| {
        // Collected before any is joined, or each worker would be spawned and
        // joined in turn -- the serial loop again.
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(index) else {
                            break;
                        };
                        if let Some(hole) = hole_in(path, &to_source) {
                            mine.push((index, hole));
                        }
                    }
                    mine
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap_or_else(|panic| resume_unwind(panic)))
            .collect()
    });
    found.sort_unstable_by_key(|(index, _)| *index);
    found.into_iter().map(|(_, hole)| hole).collect()
}

#[test]
#[cfg_attr(
    miri,
    ignore = "only slow: the lexer has no unsafe code, its unit tests run under Miri, and \
              walking and lexing the whole corpus takes Miri hours"
)]
fn tokens_and_trivia_reproduce_every_corpus_file_byte_for_byte() {
    let examples = nvs_repo::path("examples");
    let tests = nvs_repo::path("tests");

    let mut novis = Vec::new();
    collect(&examples, &["nvs"], &mut novis);
    collect(&tests, &["nvs"], &mut novis);
    novis.sort();
    assert!(
        novis.len() > 50,
        "the Novis corpus shrank to {} files — this test is measuring nothing",
        novis.len()
    );

    let mut cases = Vec::new();
    collect(&tests, &["nvst"], &mut cases);
    cases.sort();
    assert!(
        cases.len() > 100,
        "the case corpus shrank to {} files",
        cases.len()
    );

    let mut holes = holes_in(&novis, |text| Some(text.to_string()));
    holes.extend(holes_in(&cases, case_source));

    assert!(
        holes.is_empty(),
        "{} corpus file(s) are not reproduced by their tokens and trivia:\n{}",
        holes.len(),
        holes.join("\n")
    );
    eprintln!(
        "losslessness: {} Novis file(s), {} case(s)",
        novis.len(),
        cases.len()
    );
}
