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
//! **A request with no arm in [`answer`] fails the case naming itself**, and
//! that is the whole of what an unimplemented request does here: a handler
//! lands one arm at a time and the cases that ask it land with it, because a
//! runner that scored an unanswered case as a pass would put a number in front
//! of the loop that means nothing.
//!
//! **A case is answered out of its own sections and nothing on the machine.**
//! The `--FILE--` sections are written into a directory of the run's own and
//! opened as the buffers over them ([`Materialised`]), so a `require` reaches
//! the case's aux file rather than whatever sits beside the case in
//! `tests/lsp/`, and the analysis is the server's own
//! (`rule:ide/an-open-document-is-its-own-entry-point`).
//!
//! **Columns are counted in UTF-8**, so a `--EXPECT--` line's `L:C` is the
//! compiler's own byte column and reads against the source without counting
//! code units. The encoding a *client* gets is negotiated at the handshake and
//! is not this runner's business; a case is read by a person.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use lsp_types::Uri;
use nvs_diagnostics::PositionEncoding;

use crate::case::{Case, MAIN_PATH, Request};
use crate::completion;
use crate::definition;
use crate::diagnostics::{Phases, for_document};
use crate::document::{Documents, analyse, uri_of};
use crate::folding;
use crate::hover;
use crate::links;
use crate::render::{Link, Place, Response};
use crate::selection;
use crate::semantic;
use crate::symbols;

/// The units a case's columns are counted in — see the module doc.
const COLUMNS: PositionEncoding = PositionEncoding::Utf8;

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
/// This is the seam every request slice landed in, one arm at a time, and the
/// roster is now whole: the match has no wildcard, so a request added to
/// [`Request`] is a build error here rather than a case that passes by being
/// skipped — which is `rule:ide/the-request-set-is-closed` enforced by the
/// compiler on the side the runner controls.
fn answer(case: &Case) -> Result<Response, String> {
    match case.request {
        Request::Diagnostics => diagnostics(case),
        Request::Hover => hover(case),
        Request::Definition => definition(case),
        Request::Completion => completion(case),
        Request::SemanticTokens => semantic_tokens(case),
        Request::DocumentSymbol => document_symbol(case),
        Request::SelectionRange => selection_range(case),
        Request::FoldingRange => folding_range(case),
        Request::DocumentLink => document_link(case),
        Request::Redactions => redactions(case),
    }
}

/// One case's `--FILE--` sections on disk, in a directory of this run's own.
///
/// **A case is materialised, not overlaid alone.** A `require` is resolved
/// against the filesystem before any source map is consulted — `nvs_hir`
/// canonicalizes the target and reports `E0311` when there is no file there —
/// so a buffer shadows a file and there has to be a file for it to shadow. The
/// directory is what gives `require 'lib.nvs'` something to resolve; the text
/// every phase actually reads is still the buffer's, which is what keeps the
/// runner on the same path as the server.
struct Materialised {
    /// Removed when the answer has been rendered. Numbered rather than named
    /// after the case, so two cases answered at once cannot clear each other's
    /// directory out from under them.
    dir: PathBuf,
}

impl Materialised {
    /// Writes `case`'s sections, entry first.
    fn write(case: &Case) -> io::Result<Self> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "nvs-lspt-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // A directory left behind by a run that died, under a process id the
        // host has since handed out again.
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;

        let materialised = Self { dir };
        materialised.file(MAIN_PATH, &case.document)?;
        for aux in &case.aux {
            materialised.file(&aux.path, &aux.body)?;
        }
        Ok(materialised)
    }

    /// How a case would have written `target`: its path relative to this
    /// directory, with `/` separators whatever the host uses.
    ///
    /// **Two prefixes are tried, because two kinds of path reach this.** A file
    /// the graph walk loaded is canonical — `nvs_hir` canonicalizes every
    /// `require` target, and a temporary directory is reached through a symlink
    /// on more than one platform — while the entry document's path is the one
    /// its URI spells, which on Windows lacks the verbatim prefix
    /// canonicalizing adds. Stripping one of the two would name the entry by
    /// its whole path and every file it required relatively.
    ///
    /// A target under neither keeps its whole path, which no `require` a case's
    /// own sections resolved can produce — and if one ever does, an absolute
    /// path in a frozen expectation is a failure a reader can see.
    fn spelling(&self, target: &Path) -> String {
        let canonical = fs::canonicalize(&self.dir).unwrap_or_else(|_| self.dir.clone());
        let relative = target
            .strip_prefix(&canonical)
            .or_else(|_| target.strip_prefix(&self.dir))
            .unwrap_or(target);
        relative.to_string_lossy().replace('\\', "/")
    }

    /// Writes one section, creating the directories its relative path names —
    /// `--FILE lib/user.nvs--` is a directory the case did not have to declare.
    fn file(&self, at: &str, body: &str) -> io::Result<()> {
        let path = self.dir.join(at);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, body)
    }
}

impl Drop for Materialised {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// The store one case is answered out of: its sections as open buffers over
/// the files [`Materialised`] wrote, and the entry document's URI.
///
/// The directory is handed back with them because it has to outlive the
/// answer: dropping it takes the files a `require` resolves against with it.
fn store(case: &Case) -> Result<(Materialised, Documents, Uri), String> {
    let files = Materialised::write(case)
        .map_err(|error| format!("the case's files could not be written: {error}"))?;
    let mut documents = Documents::new();
    let entry = open(&mut documents, &files.dir.join(MAIN_PATH), &case.document)?;
    for aux in &case.aux {
        open(&mut documents, &files.dir.join(&aux.path), &aux.body)?;
    }
    Ok((files, documents, entry))
}

/// Opens one materialised file as the buffer over it, and hands back its URI.
///
/// Version 1 and never another: a case is one document at one moment, so
/// `rule:ide/the-server-is-synchronous`'s cancellation has nothing to say here.
fn open(documents: &mut Documents, path: &Path, text: &str) -> Result<Uri, String> {
    let uri = uri_of(path).ok_or_else(|| format!("`{}` is not a UTF-8 path", path.display()))?;
    documents.open(uri.clone(), 1, text.to_owned());
    Ok(uri)
}

/// `textDocument/publishDiagnostics`, gated unless the case asked for
/// `phase=all`.
///
/// The same [`for_document`] call the server makes, so a case cannot pass over
/// a gate the client is not actually behind — and `phase=all` is the same walk
/// read with the filter switched off, which is what pins
/// `rule:ide/diagnostics-are-phase-gated` from the side that would otherwise be
/// invisible: what the gate holds back.
fn diagnostics(case: &Case) -> Result<Response, String> {
    // Held to the end of the answer: the files a `require` resolves against go
    // when it does.
    let (_files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    let phases = if case.args.phase_all {
        Phases::All
    } else {
        Phases::Gated
    };
    Ok(Response::Diagnostics(for_document(
        &analysed, phases, COLUMNS,
    )))
}

/// `textDocument/documentSymbol` — the entry document's outline.
///
/// No cursor and no arguments: the outline is a walk of the whole file, so a
/// case asks for it and nothing else. [`crate::symbols::for_document`] is the
/// same call the server makes, on an analysis produced the same way.
fn document_symbol(case: &Case) -> Result<Response, String> {
    // Held to the end of the answer, as in `diagnostics`.
    let (_files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    Ok(Response::DocumentSymbol(symbols::for_document(
        &analysed, COLUMNS,
    )))
}

/// `textDocument/semanticTokens/full` — every name the case's document writes.
///
/// No cursor: the answer is the whole document, so a case that wrote one would
/// be asking about a position nothing here reads.
/// [`crate::semantic::for_document`] is the same call the server makes, on an
/// analysis produced the same way.
fn semantic_tokens(case: &Case) -> Result<Response, String> {
    // Held to the end of the answer, as in `diagnostics`.
    let (_files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    Ok(Response::SemanticTokens(semantic::for_document(
        &analysed,
        COLUMNS,
        &case.args.types,
    )))
}

/// `textDocument/selectionRange` — the chain at the case's `<|>`.
///
/// The first request here asked *at* a position, and the cursor is already
/// guaranteed: [`Request::takes_cursor`] names this one, so a case that wrote
/// no `<|>` was refused when it was read rather than answered at offset 0.
///
/// The offset is into the case's document, which is byte-for-byte what was
/// materialised and then loaded, so it is the entry file's offset with no
/// conversion — the marker is taken out before either is written.
fn selection_range(case: &Case) -> Result<Response, String> {
    let cursor = case.cursor.expect("selectionRange is asked at a position");
    // Held to the end of the answer, as in `diagnostics`.
    let (_files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    Ok(Response::SelectionRange(selection::at(
        &analysed,
        u32::try_from(cursor).unwrap_or(u32::MAX),
        COLUMNS,
    )))
}

/// `textDocument/definition` — where the name under the cursor is declared.
///
/// The file is named the way the case wrote it, which is the resolution half
/// [`crate::definition::Declared`] leaves to the caller, on
/// [`document_link`]'s terms: the analysis answers an absolute path inside the
/// directory this case was materialised into. A declaration in a `--FILE
/// lib/user.nvs--` section is therefore spelled `lib/user.nvs`, which is what
/// makes a cross-file case readable.
fn definition(case: &Case) -> Result<Response, String> {
    let cursor = case.cursor.expect("definition is asked at a position");
    // Held to the end of the answer, as in `diagnostics` — and here it is also
    // what the declaring file is named against.
    let (files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    let found = definition::at(
        &analysed,
        u32::try_from(cursor).unwrap_or(u32::MAX),
        COLUMNS,
    )
    .map(|declared| Place {
        path: files.spelling(&declared.path),
        position: declared.range.start,
    });
    Ok(Response::Definition(found))
}

/// `textDocument/completion` — what may be written at the case's `<|>`.
///
/// The two arguments a completion case may write are applied here rather than
/// in [`crate::completion`], and in this order: the offered list is already
/// sorted by label, so `prefix=` narrows it and `limit=` then takes the first
/// few of a list whose order does not depend on either. Doing it the other way
/// round would make `limit=1` freeze whichever member the walk happened to find
/// first. `rule:ide/a-request-line-is-closed` is what keeps the pair closed;
/// the client applies neither, because an editor filters as the developer
/// types.
fn completion(case: &Case) -> Result<Response, String> {
    let cursor = case.cursor.expect("completion is asked at a position");
    // Held to the end of the answer, as in `diagnostics`.
    let (_files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    let mut items = completion::at(&analysed, u32::try_from(cursor).unwrap_or(u32::MAX));
    if let Some(prefix) = &case.args.prefix {
        items.retain(|item| item.label.starts_with(prefix));
    }
    if let Some(limit) = case.args.limit {
        items.truncate(limit);
    }
    Ok(Response::Completion(items))
}

/// `textDocument/hover` — what the declaration under the case's `<|>` says
/// about itself.
///
/// The rendering is the Markdown verbatim, so a case's `--EXPECT--` is the doc
/// comment's own prose with its markers off — which is the point of freezing it
/// here rather than in a Rust test: the run is read the way a reader will see
/// it. [`crate::hover::at`] is the same call the server makes.
fn hover(case: &Case) -> Result<Response, String> {
    let cursor = case.cursor.expect("hover is asked at a position");
    // Held to the end of the answer, as in `diagnostics`.
    let (_files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    Ok(Response::Hover(hover::at(
        &analysed,
        u32::try_from(cursor).unwrap_or(u32::MAX),
        COLUMNS,
    )))
}

/// `textDocument/foldingRange` — where the entry document collapses.
///
/// No cursor and no arguments, as the outline takes none:
/// [`crate::folding::for_document`] is the same call the server makes, on an
/// analysis produced the same way.
fn folding_range(case: &Case) -> Result<Response, String> {
    // Held to the end of the answer, as in `diagnostics`.
    let (_files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    Ok(Response::FoldingRange(folding::for_document(
        &analysed, COLUMNS,
    )))
}

/// `textDocument/documentLink` — every `require` the entry document writes.
///
/// The target is named the way the case wrote it, which is the resolution half
/// [`Link`]'s doc leaves to the runner: what the analysis answers is an
/// absolute path inside the directory this case was materialised into, and
/// nothing else knows what that directory is.
fn document_link(case: &Case) -> Result<Response, String> {
    // Held to the end of the answer, as in `diagnostics` — and here it is also
    // what a target is named against.
    let (files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    let links = links::for_document(&analysed, COLUMNS)
        .into_iter()
        .map(|link| Link {
            range: link.range,
            target: files.spelling(&link.target),
        })
        .collect();
    Ok(Response::DocumentLink(links))
}

/// `nvs/redactions` — which bytes of the entry document the client conceals.
///
/// No cursor and no arguments, on [`folding_range`]'s terms: the request is
/// asked of the whole document, and the same
/// [`crate::redactions::for_document`] call the server makes answers it.
fn redactions(case: &Case) -> Result<Response, String> {
    // Held to the end of the answer, as in `diagnostics`.
    let (_files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    Ok(Response::Redactions(crate::redactions::for_document(
        &analysed, COLUMNS,
    )))
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

    /// One case's document, asked about twice.
    fn asked(request: &str) -> String {
        let text = format!(
            "--TEST--\nthe gate, from the side that is otherwise invisible\n--FILE--\n\
             <?nvs\nvar $broken = ;\nvar $other = $nope;\n--REQUEST--\n{request}\n\
             --EXPECT--\n"
        );
        let case = Case::parse(Path::new("gate.lspt"), &text).expect("the case parses");
        answer(&case).expect("diagnostics are answered").render()
    }

    /// The codes in a rendering, which is `L:C-L:C severity CODE message`.
    fn codes(rendered: &str) -> Vec<&str> {
        rendered
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2))
            .collect()
    }

    /// `rule:ide/diagnostics-are-phase-gated` from the side no gated case can
    /// show: `phase=all` hands back what the gate held, so the suppression is
    /// pinned as a filter rather than as an analysis that never ran.
    ///
    /// The first assertion is what keeps that honest — without it this passes
    /// just as well over a front end that reports nothing below the parser at
    /// all, which is a state this crate has been in.
    #[test]
    fn phase_all_publishes_what_the_gate_suppressed() {
        let ungated = asked("diagnostics phase=all");
        let held = codes(&ungated);
        assert!(
            held.iter().any(|code| code.starts_with("E03")),
            "the walk reported nothing for the gate to hold back: {held:?}"
        );

        let gated = asked("diagnostics");
        let published = codes(&gated);
        assert!(
            published.iter().any(|code| code.starts_with("E01")),
            "the parse error itself was held back: {published:?}"
        );
        assert!(
            !published.iter().any(|code| code.starts_with("E03")
                || code.starts_with("E04")
                || code.starts_with("E07")
                || code.starts_with("E08")),
            "a diagnostic from below the broken parse reached the case: {published:?}"
        );

        // The two answers are the same walk read two ways, so everything the
        // gate published is in what it held back beside the rest.
        for line in gated.lines() {
            assert!(
                ungated.lines().any(|ungated| ungated == line),
                "`phase=all` dropped a diagnostic the gate published, so the two \
                 are not one walk: {line}"
            );
        }
    }

    #[test]
    fn a_tree_of_cases_reports_one_summary_line() {
        let dir = scratch("tree");
        write(
            &dir.join("mismatched.lspt"),
            "--TEST--\nwhat a class name completes to\n--FILE--\n<?nvs\nclass Us<|>er {}\n\
             --REQUEST--\ncompletion\n--EXPECT--\nname    property  string\n",
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

        // The two failures read differently: one file is not a case at all,
        // and the other is a case whose frozen expectation is not the answer.
        assert!(report.contains("no `--REQUEST--` section"), "{report}");
        assert!(
            report.contains("the completion answer is not what `--EXPECT--` freezes"),
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
