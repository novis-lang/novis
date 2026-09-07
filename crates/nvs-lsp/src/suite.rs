//! Running a tree of `.lspt` cases, and the summary line the loop reads.
//!
//! `nvs lsp-test <paths>` is this module with a `main` around it: it walks the
//! paths for `*.lspt`, reads each with [`Case::parse`](crate::Case::parse),
//! asks the question the case asks, renders the answer through
//! [`crate::render`] and compares it to `--EXPECT--` byte for byte
//! (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`).
//!
//! The last line is always `N passed, M failed` — the shape `tools/loop.py`'s
//! `nvs-suite` check parses, which is how an editor-behaviour gate reaches the
//! loop with no change to the driver. **It is `.lspt`'s own count and shares
//! nothing with `nvs test`'s**: two suites answering two questions, and a
//! summary that meant both would mean neither.
//!
//! A case runs in this process rather than in one of its own. `.nvst` spawns
//! per case because it runs a *program* whose stdout is the expectation;
//! nothing here runs a program at all, so a case is a parse, a question and a
//! string comparison, and a process would buy isolation from nothing.
//!
//! **No request is answered yet**, so every case fails naming the request it
//! asked — [`answer`] is where a handler lands, one arm per request slice, and
//! the case corpus arrives with the handler that makes it answerable. A runner
//! that scored an unanswered case as a pass would put a number in front of the
//! loop that means nothing.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::case::Case;
use crate::render::Response;

/// The extension a case file carries.
pub const EXTENSION: &str = "lspt";

/// What a whole run came to.
///
/// No `skipped`: `.lspt` has no `--SKIPIF--` and will not grow one — a case
/// asks a question of a document, so there is no host condition it could
/// decline for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Cases whose rendering equalled `--EXPECT--`.
    pub passed: usize,
    /// Cases that failed, including ones that could not be read at all.
    pub failed: usize,
}

impl Summary {
    /// True when nothing failed.
    #[must_use]
    pub const fn is_success(self) -> bool {
        self.failed == 0
    }
}

/// Collects every `.lspt` file under `paths`, sorted, directories walked.
///
/// Sorted so two runs of one tree report in the same order, on any host: a
/// directory listing is not ordered, and a suite whose report shuffles cannot
/// be diffed against its last run.
///
/// # Errors
///
/// Fails when a path cannot be read, including one that does not exist — a
/// mistyped directory is the failure it looks like, never an empty suite that
/// passes.
pub fn collect(paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for path in paths {
        gather(path, &mut found)?;
    }
    found.sort();
    Ok(found)
}

/// Adds `path` — or every case under it — to `into`.
fn gather(path: &Path, into: &mut Vec<PathBuf>) -> io::Result<()> {
    if !path.is_dir() {
        // A file named on the command line is run whatever it is called; the
        // extension is how a *directory* is walked, not what a case has to be.
        if path.exists() {
            into.push(path.to_path_buf());
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} does not exist", path.display()),
        ));
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?.path();
        if entry.is_dir() {
            gather(&entry, into)?;
        } else if entry.extension().is_some_and(|ext| ext == EXTENSION) {
            into.push(entry);
        }
    }
    Ok(())
}

/// Runs every case in `paths`, writing a report to `out`.
///
/// # Errors
///
/// Fails on a discovery error, or when `out` cannot be written to. A failing
/// *case* is not an error: it is counted in the returned [`Summary`], and so is
/// a case that could not be parsed, because a file that is not a case is a
/// failure of that file rather than of the run.
pub fn run(paths: &[PathBuf], out: &mut dyn Write) -> io::Result<Summary> {
    let mut summary = Summary::default();
    for path in collect(paths)? {
        match check(&path) {
            Ok(()) => summary.passed += 1,
            Err(lines) => {
                summary.failed += 1;
                writeln!(out, "FAIL {}", path.display())?;
                for line in lines {
                    writeln!(out, "  {line}")?;
                }
            }
        }
    }
    writeln!(out, "{} passed, {} failed", summary.passed, summary.failed)?;
    Ok(summary)
}

/// Reads one case, asks its question and compares the rendering.
fn check(path: &Path) -> Result<(), Vec<String>> {
    let text = fs::read_to_string(path).map_err(|error| vec![format!("{error}")])?;
    let case = Case::parse(path, &text).map_err(|error| vec![format!("{error}")])?;
    let answer = answer(&case).map_err(|why| vec![why])?;
    let rendered = answer.render();
    if rendered == case.expect {
        return Ok(());
    }
    // Both halves in full, and never a "did not match": an expectation is
    // frozen, so what a reader has to decide is whether the *answer* changed
    // for a reason — and that decision needs the two texts side by side.
    let mut lines = vec![format!(
        "the {} answer is not what `--EXPECT--` freezes",
        case.request
    )];
    lines.push("--- expected".to_owned());
    lines.extend(case.expect.lines().map(str::to_owned));
    lines.push("--- answered".to_owned());
    lines.extend(rendered.lines().map(str::to_owned));
    Err(lines)
}

/// Answers the question `case` asks of its document.
///
/// This is the seam every request slice lands in, one arm at a time. Until one
/// does, a case says which request it is waiting for rather than passing: the
/// number in front of `min_passing` counts answers, and an unanswered case is
/// not one.
fn answer(case: &Case) -> Result<Response, String> {
    Err(format!(
        "`{}` is not answered yet; the handler and this case's own slice land together",
        case.request
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of this run's own, removed by the test that made it.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nvs-lspt-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("nested")).expect("a scratch directory");
        dir
    }

    fn write(path: &Path, text: &str) {
        fs::write(path, text).expect("a case file");
    }

    #[test]
    fn a_tree_of_cases_reports_one_summary_line() {
        let dir = scratch("tree");
        write(
            &dir.join("unanswered.lspt"),
            "--TEST--\nthe outline of a class\n--FILE--\n<?nvs\nclass User {}\n\
             --REQUEST--\ndocumentSymbol\n--EXPECT--\nUser class\n",
        );
        write(
            &dir.join("nested").join("malformed.lspt"),
            "--TEST--\nno request at all\n--FILE--\n<?nvs\n--EXPECT--\nnone\n",
        );
        // Walked for the extension, so a file that is not a case is not one.
        write(&dir.join("README.md"), "what this tree is for\n");

        let mut report = Vec::new();
        let summary = run(std::slice::from_ref(&dir), &mut report).expect("the tree is readable");
        let report = String::from_utf8(report).expect("the report is text");

        assert_eq!(
            summary,
            Summary {
                passed: 0,
                failed: 2
            },
            "{report}"
        );
        assert!(!summary.is_success());
        assert!(report.ends_with("0 passed, 2 failed\n"), "{report}");

        // The two failures read differently: one file is not a case, and the
        // other is a case nothing answers yet.
        assert!(report.contains("no `--REQUEST--` section"), "{report}");
        assert!(
            report.contains("`documentSymbol` is not answered yet"),
            "{report}"
        );
        assert!(!report.contains("README"), "{report}");

        // A path that does not exist is the mistake it looks like, rather than
        // an empty suite that passes.
        let missing = dir.join("nowhere");
        let error = run(&[missing], &mut Vec::new()).expect_err("a mistyped path is an error");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);

        let _ = fs::remove_dir_all(&dir);
    }
}
