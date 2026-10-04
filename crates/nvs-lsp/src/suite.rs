//! Running a tree of `.lspt` cases, and the summary line the loop reads.
//!
//! `nvs lsp-test <paths>` is this module with a `main` around it: it walks the
//! paths for `*.lspt`, reads each with [`Case::parse`](crate::Case::parse),
//! asks the question the case asks, renders the answer through
//! [`crate::render`] and compares it to `--EXPECT--` byte for byte
//! (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`).
//!
//! The last line is always `N passed, M failed` — the shape `bun nv loop`'s
//! `nvs-suite` check parses, which is how an editor-behaviour gate reaches the
//! loop with no change to the driver. **It is `.lspt`'s own count and shares
//! nothing with `nvs test`'s**: two suites answering two questions, and a
//! summary that meant both would mean neither.
//!
//! `--coverage` prints [`crate::coverage`]'s matrix **above** that line and
//! never in place of it: a run whose report ended in a matrix would be a run
//! with no verdict in it, and what a coverage run is gated on is still the
//! corpus passing. The matrix comes out of the same pass — a case is answered
//! once and its constructs are read off the analysis that answered it — which
//! is why [`Outcome`] carries both.
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
use nvs_diagnostics::{BytePos, PositionEncoding};

use crate::actions;
use crate::case::{Case, MAIN_PATH, Request};

/// Where a case's stub tree is written, under the case's own directory, and
/// so how a jump into it is spelled: `stubs/Core/Str.nvs`.
const STUBS_PATH: &str = "stubs";
use crate::coverage::{self, Matrix};
use crate::definition;
use crate::diagnostics::{Phases, for_document};
use crate::document::{Analysed, Documents, analyse, path_of, uri_of};
use crate::folding;
use crate::hover;
use crate::links;
use crate::render::{Link, Place, Response};
use crate::selection;
use crate::semantic;
use crate::stubs::{self, Stubs};
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

/// Everything one run produced: what it came to, and what it covered.
///
/// The matrix travels with the count because it is derived from the same pass
/// and cannot be rebuilt without repeating it — a corpus is answered once, and
/// a caller that wanted both would otherwise run every case twice.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Cases passed and failed.
    pub summary: Summary,
    /// What the passing cases covered
    /// (`rule:ide/lspt-coverage-is-inferred`).
    pub matrix: Matrix,
}

/// What a run prints when it is done.
///
/// Whether the matrix comes first, never whether the verdict comes at all:
/// `bun nv loop`'s `nvs-suite` check reads the last line as `N passed, M
/// failed`, so the matrix goes above it, where it is a report for a person and
/// the line under it is still the one the driver finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    /// The verdict alone.
    Summary,
    /// The request × construct matrix, and the verdict under it.
    Coverage,
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
pub fn run(paths: &[PathBuf], out: &mut dyn Write, report: Report) -> io::Result<Outcome> {
    let mut outcome = Outcome {
        summary: Summary::default(),
        matrix: Matrix::new(),
    };
    for path in collect(paths)? {
        match check(&path) {
            Ok((request, covered)) => {
                outcome.summary.passed += 1;
                outcome.matrix.record(&path, request, &covered);
            }
            Err(lines) => {
                outcome.summary.failed += 1;
                writeln!(out, "FAIL {}", path.display())?;
                for line in lines {
                    writeln!(out, "  {line}")?;
                }
            }
        }
    }
    if report == Report::Coverage {
        write!(out, "{}", outcome.matrix.render())?;
    }
    writeln!(
        out,
        "{} passed, {} failed",
        outcome.summary.passed, outcome.summary.failed
    )?;
    Ok(outcome)
}

/// Reads one case, asks its question and compares the rendering.
///
/// The constructs come back with the request that reached them, and only from
/// here: a case whose rendering is not what it froze covers nothing, because
/// coverage is a claim a frozen expectation makes
/// (`rule:ide/lspt-coverage-is-inferred`).
fn check(path: &Path) -> Result<(Request, Vec<&'static str>), Vec<String>> {
    let text = fs::read_to_string(path).map_err(|error| vec![format!("{error}")])?;
    let case = Case::parse(path, &text).map_err(|error| vec![format!("{error}")])?;
    let answered = answer(&case).map_err(|why| vec![why])?;
    let rendered = answered.response.render();
    if rendered == case.expect {
        return Ok((case.request, answered.covered));
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

/// What one case's question came to.
pub(crate) struct Answered {
    /// The answer, in the shape the request answers.
    pub response: Response,
    /// Every construct the question landed on, sorted — see [`crate::coverage`].
    pub covered: Vec<&'static str>,
}

/// Answers the question `case` asks of its document.
///
/// This is the seam every request slice landed in, one arm at a time, and the
/// roster is now whole: the match has no wildcard, so a request added to
/// [`Request`] is a build error here rather than a case that passes by being
/// skipped — which is `rule:ide/the-request-set-is-closed` enforced by the
/// compiler on the side the runner controls.
///
/// **The analysis is made once, here, and every arm is handed it.** That is
/// what lets coverage be inferred rather than declared: the construct a case
/// reached is read off the same [`Analysed`] that answered it, and an arm has
/// no opportunity to report a construct its own answer did not touch
/// (`rule:ide/lspt-coverage-is-inferred`).
///
/// The materialised files are held until the answer is rendered, because the
/// files a `require` resolves against go when they do.
pub(crate) fn answer(case: &Case) -> Result<Answered, String> {
    let (files, documents, entry) = store(case)?;
    let analysed = analyse(&documents, &entry)
        .ok_or_else(|| "the case's document could not be analysed".to_owned())?;
    let response = match case.request {
        Request::Diagnostics => {
            diagnostics(&analysed, &documents, &files, &entry, case.args.phase_all)
        }
        Request::Hover => hover(&analysed, at(case)),
        Request::Definition => definition(&analysed, &files, at(case)),
        Request::Completion => completion(&analysed, &documents, &files, case, at(case)),
        Request::SemanticTokens => semantic_tokens(&analysed, &case.args.types),
        Request::DocumentSymbol => document_symbol(&analysed),
        Request::SelectionRange => selection_range(&analysed, at(case)),
        Request::FoldingRange => folding_range(&analysed),
        Request::DocumentLink => document_link(&analysed, &files),
        Request::CodeAction => code_action(&analysed, at(case)),
        Request::CodeLens => code_lens(&documents, &files, &entry),
        Request::References => references(&analysed, &documents, &files, at(case)),
        Request::DocumentHighlight => {
            document_highlight(&analysed, &documents, &files, &entry, at(case))
        }
        Request::InlayHint => inlay_hints(&analysed),
        Request::Redactions => redactions(&analysed),
        Request::Regions => regions(&analysed),
    };
    let covered = coverage::of(&analysed, cursor(case), &response, COLUMNS);
    Ok(Answered { response, covered })
}

/// The case's cursor as a byte offset, if it wrote one.
fn cursor(case: &Case) -> Option<BytePos> {
    case.cursor
        .map(|at| BytePos::try_from(at).unwrap_or(BytePos::MAX))
}

/// The case's cursor, for a request that is asked at one.
///
/// [`Request::takes_cursor`] names those requests and [`Case::parse`] refuses
/// one written without a `<|>`, so the marker is guaranteed by the time an arm
/// is picked rather than defaulted to offset 0.
fn at(case: &Case) -> BytePos {
    cursor(case).expect("a request asked at a position was written a cursor")
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
    ///
    /// **Canonical, without Windows' verbatim `\\?\` prefix**, which is the
    /// spelling [`crate::document::uri_of`] gives a canonical path. The server
    /// names every file it answers by its canonical path, so a directory
    /// spelled any other way would never be a prefix of an answered location.
    /// The platform temporary root is not canonical everywhere: some Windows
    /// machines name it by an 8.3 alias (`C:\Users\RUNNER~1`) or in a case the
    /// disk does not use, and macOS keeps it under the `/var` symlink.
    dir: PathBuf,
    /// The same directory as [`fs::canonicalize`] spells it, verbatim prefix
    /// and all: the spelling of a path the graph walk loaded.
    canonical: PathBuf,
}

impl Materialised {
    /// Writes `case`'s sections, entry first.
    fn write(case: &Case) -> io::Result<Self> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let created = std::env::temp_dir().join(format!(
            "nvs-lspt-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // A directory left behind by a run that died, under a process id the
        // host has since handed out again.
        let _ = fs::remove_dir_all(&created);
        fs::create_dir_all(&created)?;
        let canonical = match fs::canonicalize(&created) {
            Ok(canonical) => canonical,
            Err(error) => {
                let _ = fs::remove_dir_all(&created);
                return Err(error);
            }
        };
        let dir = canonical
            .to_str()
            .and_then(|text| text.strip_prefix(r"\\?\"))
            .map_or_else(|| canonical.clone(), PathBuf::from);

        let materialised = Self { dir, canonical };
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
    /// the graph walk loaded is canonical, because `nvs_hir` canonicalizes
    /// every `require` target, and on Windows that spelling carries the
    /// verbatim prefix. A location the server answered is spelled by its URI,
    /// which never carries that prefix. The directory is held in both
    /// spellings, and stripping only one would name some of a case's files by
    /// their whole path.
    ///
    /// A target under neither keeps its whole path, which no `require` a case's
    /// own sections resolved can produce — and if one ever does, an absolute
    /// path in a frozen expectation is a failure a reader can see.
    fn spelling(&self, target: &Path) -> String {
        let relative = target
            .strip_prefix(&self.canonical)
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
        // A case that jumped to a `Core` name wrote the stub tree here, and
        // every file of it read-only.
        stubs::unlock(&self.dir);
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
    // The case's own stub tree, under its directory, so a jump to a `Core`
    // name is spelled `stubs/Core/Str.nvs` the way a `--FILE lib/user.nvs--`
    // is spelled — and written only by a case that makes such a jump.
    documents.set_stubs(Some(Stubs::at(files.dir.join(STUBS_PATH))));
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
///
/// The dimming the server publishes beside them is appended the same way
/// ([`crate::server::dimming_of_case`]), so an unused import or private member
/// is frozen in the case that writes one.
fn diagnostics(
    analysed: &Analysed,
    documents: &Documents,
    files: &Materialised,
    entry: &Uri,
    phase_all: bool,
) -> Response {
    let phases = if phase_all {
        Phases::All
    } else {
        Phases::Gated
    };
    let mut published = for_document(
        analysed,
        &crate::completion_files::CompletionFiles::default(),
        phases,
        COLUMNS,
    );
    published.extend(crate::server::dimming_of_case(
        documents, analysed, &files.dir, entry, COLUMNS,
    ));
    Response::Diagnostics(published)
}

/// `textDocument/documentSymbol` — the entry document's outline.
///
/// No cursor and no arguments: the outline is a walk of the whole file, so a
/// case asks for it and nothing else. [`crate::symbols::for_document`] is the
/// same call the server makes, on an analysis produced the same way.
fn document_symbol(analysed: &Analysed) -> Response {
    Response::DocumentSymbol(symbols::for_document(analysed, COLUMNS))
}

/// `textDocument/semanticTokens/full` — every name the case's document writes.
///
/// No cursor: the answer is the whole document, so a case that wrote one would
/// be asking about a position nothing here reads.
/// [`crate::semantic::for_document`] is the same call the server makes, on an
/// analysis produced the same way.
fn semantic_tokens(analysed: &Analysed, types: &[String]) -> Response {
    Response::SemanticTokens(semantic::for_document(analysed, COLUMNS, types))
}

/// `textDocument/selectionRange` — the chain at the case's `<|>`.
///
/// The offset is into the case's document, which is byte-for-byte what was
/// materialised and then loaded, so it is the entry file's offset with no
/// conversion — the marker is taken out before either is written.
fn selection_range(analysed: &Analysed, offset: BytePos) -> Response {
    Response::SelectionRange(selection::at(analysed, offset, COLUMNS))
}

/// `textDocument/definition` — where the name under the cursor is declared.
///
/// The file is named the way the case wrote it, which is the resolution half
/// [`crate::definition::Declared`] leaves to the caller, on
/// [`document_link`]'s terms: the analysis answers an absolute path inside the
/// directory this case was materialised into. A declaration in a `--FILE
/// lib/user.nvs--` section is therefore spelled `lib/user.nvs`, which is what
/// makes a cross-file case readable.
fn definition(analysed: &Analysed, files: &Materialised, offset: BytePos) -> Response {
    let found = definition::at(
        analysed,
        &crate::completion_files::CompletionFiles::default(),
        offset,
        COLUMNS,
    )
    .map(|declared| Place {
        path: files.spelling(&declared.path),
        position: declared.range.start,
    });
    Response::Definition(found)
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
fn completion(
    analysed: &Analysed,
    documents: &Documents,
    files: &Materialised,
    case: &Case,
    offset: BytePos,
) -> Response {
    let mut items = crate::server::items_of_case(documents, analysed, &files.dir, offset);
    if let Some(prefix) = &case.args.prefix {
        items.retain(|item| item.label.starts_with(prefix));
    }
    if let Some(limit) = case.args.limit {
        items.truncate(limit);
    }
    Response::Completion(items)
}

/// `textDocument/hover` — what the declaration under the case's `<|>` says
/// about itself.
///
/// The rendering is the Markdown verbatim, so a case's `--EXPECT--` is the doc
/// comment's own prose with its markers off — which is the point of freezing it
/// here rather than in a Rust test: the run is read the way a reader will see
/// it. [`crate::hover::at`] is the same call the server makes.
fn hover(analysed: &Analysed, offset: BytePos) -> Response {
    Response::Hover(hover::at(
        analysed,
        &crate::completion_files::CompletionFiles::default(),
        offset,
        COLUMNS,
    ))
}

/// `textDocument/foldingRange` — where the entry document collapses.
///
/// No cursor and no arguments, as the outline takes none:
/// [`crate::folding::for_document`] is the same call the server makes, on an
/// analysis produced the same way.
fn folding_range(analysed: &Analysed) -> Response {
    Response::FoldingRange(folding::for_document(analysed, COLUMNS))
}

/// `textDocument/documentLink` — every `require` and `autoload` written path
/// the entry document writes that names something on disk.
///
/// The target is named the way the case wrote it, which is the resolution half
/// [`Link`]'s doc leaves to the runner: what the analysis answers is an
/// absolute path inside the directory this case was materialised into, and
/// nothing else knows what that directory is. A directory ends in `/`, as the
/// URI the server sends for one does.
fn document_link(analysed: &Analysed, files: &Materialised) -> Response {
    let links = links::for_document(analysed, COLUMNS)
        .into_iter()
        .map(|link| {
            let mut target = files.spelling(&link.target);
            if link.directory {
                target.push('/');
            }
            Link {
                range: link.range,
                target,
            }
        })
        .collect();
    Response::DocumentLink(links)
}

/// `textDocument/codeAction` — what may be fixed where the cursor is.
///
/// The request is asked over a range and a case writes a cursor, so the range
/// is the empty one at it: a light bulb with nothing selected, which is how
/// both of the fixes that ship are reached
/// (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
/// [`crate::actions::Kind::QuickFix`] is what a person's client asks for, so it
/// is the kind a case freezes; the other kind is a client's own request and is
/// answered from the same translation.
fn code_action(analysed: &Analysed, offset: BytePos) -> Response {
    Response::CodeAction(actions::at(
        analysed,
        &crate::completion_files::CompletionFiles::default(),
        offset,
        offset,
        actions::Kind::QuickFix,
        COLUMNS,
    ))
}

/// `nvs/redactions` — which bytes of the entry document the client conceals.
///
/// No cursor and no arguments, on [`folding_range`]'s terms: the request is
/// asked of the whole document, and the same
/// [`crate::redactions::for_document`] call the server makes answers it.
fn redactions(analysed: &Analysed) -> Response {
    Response::Redactions(crate::redactions::for_document(analysed, COLUMNS))
}

/// `nvs/regions` — where the entry document stops being Novis.
///
/// No cursor and no arguments, on [`folding_range`]'s terms. Through
/// [`crate::regions::for_document`] rather than the lex the server runs over a
/// buffer, because that is the analysis this suite already has in hand; both
/// reach the same walk, so a case and an editor cannot disagree about a
/// boundary.
fn regions(analysed: &Analysed) -> Response {
    Response::Regions(crate::regions::for_document(analysed, COLUMNS))
}

/// `textDocument/inlayHint` — what the type phase recorded at the two places
/// the entry document left unwritten.
///
/// No cursor and no arguments, on [`folding_range`]'s terms, and the same
/// [`crate::hints::for_document`] call the server makes. Every hint the
/// document carries: the server filters that answer down to the range a client
/// asked about, and a viewport is not a thing a case's sections can write.
fn inlay_hints(analysed: &Analysed) -> Response {
    Response::Hints(crate::hints::for_document(analysed, COLUMNS))
}

/// `textDocument/codeLens` — what the index counts above every declaration the
/// entry document makes.
///
/// The one arm that is not answered from the analysis: a lens is a count of
/// uses across the workspace, so it comes from the index `crate::server`'s
/// `lenses_of_case` builds over this case and its `--FILE--` sections. No
/// cursor and no arguments, on [`folding_range`]'s terms.
///
/// The runner asks with `nvs.codeLens.enable` at the roster's default, which
/// is on, so the `None` that setting buys is never what a case freezes — a
/// case writes a document and a question, never a setting, and the refusal has
/// a `-p nvs-lsp` test of its own.
fn code_lens(documents: &Documents, files: &Materialised, entry: &Uri) -> Response {
    Response::CodeLens(
        crate::server::lenses_of_case(documents, &files.dir, entry, COLUMNS).unwrap_or_default(),
    )
}

/// `textDocument/references` — every use of the name under the case's cursor,
/// with the declaration among them.
///
/// A hit is spelled the way [`definition`]'s place is: the server answers a
/// `Location` naming the directory the case was materialised into, and turning
/// that back into `case.nvs` or `lib/user.nvs` is this runner's job rather than
/// [`crate::render`]'s. A URI that is no path is dropped, which is unreachable
/// from a case's own sections and would be an absolute path in a frozen
/// expectation if it were not.
fn references(
    analysed: &Analysed,
    documents: &Documents,
    files: &Materialised,
    offset: BytePos,
) -> Response {
    let found = crate::server::references_of_case(documents, analysed, &files.dir, offset, COLUMNS)
        .unwrap_or_default();
    Response::References(
        found
            .iter()
            .filter_map(|at| {
                Some(Place {
                    path: files.spelling(&path_of(&at.uri)?),
                    position: at.range.start,
                })
            })
            .collect(),
    )
}

/// `textDocument/documentHighlight` — every use of the name under the case's
/// cursor that was written in the case's own `--FILE--`.
///
/// [`references`]' spelling over [`references`]' query narrowed to one file, so
/// a highlight case and a reference case at the same cursor differ only by what
/// this one drops: a use in an aux file, and the declaration.
///
/// A case's entry document is always a path, being a section this runner wrote
/// itself, so the `None` arm is unreachable and answers an empty list rather
/// than an error nothing could produce.
fn document_highlight(
    analysed: &Analysed,
    documents: &Documents,
    files: &Materialised,
    entry: &Uri,
    offset: BytePos,
) -> Response {
    let found = path_of(entry)
        .and_then(|here| {
            crate::server::highlights_of_case(
                documents, analysed, &files.dir, &here, offset, COLUMNS,
            )
        })
        .unwrap_or_default();
    Response::DocumentHighlight(
        found
            .iter()
            .filter_map(|at| {
                Some(Place {
                    path: files.spelling(&path_of(&at.uri)?),
                    position: at.range.start,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of this run's own with `nested/` in it, deleted when the
    /// guard drops.
    fn scratch(name: &str) -> nvs_repo::Scratch {
        let dir = nvs_repo::scratch(&format!("lspt-{name}"));
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
        answer(&case)
            .expect("diagnostics are answered")
            .response
            .render()
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

    // covers: tools:editor/nvs-lsp-test
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
        let outcome =
            run(&[dir.to_path_buf()], &mut report, Report::Summary).expect("the tree is readable");
        let report = String::from_utf8(report).expect("the report is text");

        assert_eq!(
            outcome.summary,
            Summary {
                passed: 0,
                failed: 2
            },
            "{report}"
        );
        assert!(!outcome.summary.is_success());
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
        let error = run(&[missing], &mut Vec::new(), Report::Summary)
            .expect_err("a mistyped path is an error");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
