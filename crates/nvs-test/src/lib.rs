//! The `.nvst` conformance-case format and the runner behind `nvs test`.
//!
//! A `.nvst` file is one case: a program, what it should print, and a title
//! saying what it is for. The expectation is frozen in the case itself, so a
//! case states what Novis prints and is judged against nothing else
//! (`rule:testing/nvst-is-separate`).
//!
//! ```text
//! --TEST--
//! an int converts to a string
//! --FILE--
//! <?nvs
//! echo 41 as string;
//! --EXPECT--
//! 41
//! ```
//!
//! ## The sections
//!
//! The program and what it prints, and the input it runs with:
//!
//! | section | meaning |
//! |---|---|
//! | `--TEST--` | the one-line title, required |
//! | `--FILE--` | the Novis program, required |
//! | `--EXPECT--` | expected standard output, compared literally |
//! | `--EXPECTF--` | expected standard output, with [`expect`]'s `%` escapes |
//! | `--SKIPIF--` | a program whose output starting `skip` skips the case; one that exits non-zero fails it |
//! | `--CLEAN--` | a program run afterwards, whose output is ignored |

//! | `--ARGS--` | the program's own arguments, one per line |
//! | `--ENV--` | environment variables for the run, one `NAME=value` per line |
//! | `--GET--` | the query string the case's request carries, on one line |
//! | `--POST--` | that request's body as urlencoded pairs, on one line |
//! | `--POST_RAW--` | that request's body verbatim |
//! | `--COOKIE--` | its cookies, one `NAME=value` per line |
//! | `--HEADERS--` | its header fields, one `Name: value` per line |
//!
//! The error pair, the peer, the extra files and the subcommand:
//!
//! | section | meaning |
//! |---|---|
//! | `--EXPECT-ERROR--` | expected standard error, compared literally |
//! | `--EXPECTF-ERROR--` | expected standard error, with `%` escapes |
//! | `--CLIENT_IP--` | the address the request's peer resolved to, on one line |
//! | `--SCHEME--` | `http` or `https`, the scheme that request arrived over |
//! | `--FILE <relative/path>--` | another file, written beside `--FILE--`; repeatable |
//! | `--RUN--` | `run` (the default), `test`, `test --format=json`, `test --format=junit`, `test --list --format=json` or `config dump --origin`: the command line `--FILE--` goes through |
//! | `--EXTENSION--` | fixture extensions to load, one name per line: each `<name>.nvsx` under the nearest `ext/fixtures/` above the case is copied beside `--FILE--`, and its pinned `[[extension]]` entry is added to `nvs.toml`, which is created when the case wrote none; a name with no fixture is a parse error |
//! | `--COPY--` | repository files to copy beside `--FILE--` before `--SKIPIF--` runs, one path per line relative to the case's directory, `..` allowed: each lands under its last segment, and a source that is not there skips the case naming it |
//!
//! ## Which subcommand a case is run through
//!
//! `--RUN--` is how a case reaches
//! `rule:testing/test-attribute`'s other
//! runner. `--RUN--\ntest` runs `nvs test case.nvs`, so the program declares
//! `#[Test]` classes and what the case pins is the *report* of running them —
//! the only way a `.nvst` can observe the `#[Test]` table at all, since a row
//! and its order are visible nowhere else. § 22's machine formats are further
//! spellings of the same thing — `test --format=json` and
//! `test --format=junit` — because a report is observable only by being read,
//! and a format nothing pins is a format that can drift.
//! `test --list --format=json` is the same argument about the document that
//! says what a program declares *without* running it: what it pins is that a
//! case whose tests would fail produces the same listing a passing one does.
//! The roster is closed
//! to those spellings, so a misspelling is a parse error rather than a
//! case quietly run the other way, and the section applies to `--FILE--`
//! alone: `--SKIPIF--` and `--CLEAN--` are the runner's own scaffolding and
//! are always `nvs run`.
//!
//! § 22's report carries a per-test duration, so such a case wants
//! `--EXPECTF--`'s `%f` rather than `--EXPECT--`.
//!
//! `--RUN--\nconfig dump --origin` is the one spelling that runs no program.
//! It names no file on the command line, so `nvs config dump` resolves
//! `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
//! step 2's `./nvs.toml` out of the case's own working directory — which
//! makes a tree written with `--FILE nvs.toml--` and `--FILE conf.d/…--` the
//! thing under test, and § 9's listing the expectation. That is where § 3's
//! obligation to record an override with **both** origins is observable at
//! all: no program can ask where a value was written. Such a case wants
//! `--EXPECTF--`, because the listing names absolute paths under a temporary
//! directory.
//!
//! ## More than one file
//!
//! `--FILE--` is the program that runs, and it is always written as
//! `case.nvs` in the working directory. `--FILE <relative/path>--` writes
//! *another* file into that same directory, at the path it names, creating
//! the directories along the way — so a case can hold a `require` target, an
//! autoload root and the class it declares, which is what
//! `rule:programs/no-runtime-autoload`
//! needs to be observable end to end at all:
//!
//! ```text
//! --FILE--
//! <?nvs
//! require './src/Bootstrap.nvs';
//! echo (new App\Greeter())->greet(), "\n";
//! --FILE src/Bootstrap.nvs--
//! <?nvs
//! autoload 'App' from './';
//! --FILE src/Greeter.nvs--
//! <?nvs
//! namespace App;
//! class Greeter { public function greet(): string { return "hi"; } }
//! --EXPECT--
//! hi
//! ```
//!
//! The path is relative, `/`-separated on both legs, and may not hold a `.`
//! or `..` segment or name one of the files the runner writes itself
//! (`case.nvs`, `skipif.nvs`, `clean.nvs`, `request.nvsr`) — so a case
//! cannot reach outside the temporary directory it is given, and needs no
//! sanitiser to say so. Repeating one path is a parse error, the way
//! repeating any other section is.
//!
//! ## Failing runs, and unknown sections
//!
//! The error pair exists because Novis writes a diagnostic and an uncaught
//! throw to **standard error**. Without it, no case could cover a compile
//! error at all. Their presence is also the one thing that says a case
//! expects the run to fail; every other case must exit zero, so nothing can
//! pass by printing the right prefix on its way to a crash.
//!
//! A section name outside the tables above is a parse error naming the
//! offender and its line — `--INI--` and `--ORACLE--` among them — so a case
//! never runs with part of what it wrote silently ignored.
//!
//! ## Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-test/src/lib.rs` lists them.
//!
//! ## What the request sections change
//!
//! The seven request sections **are** honoured, and they are the one thing in
//! this format that changes what the program under test *is*: a case writing
//! `--GET--`, `--POST--`, `--POST_RAW--`, `--COOKIE--`, `--HEADERS--`,
//! `--CLIENT_IP--` or `--SCHEME--` is answering a request, so `Core\Request`'s
//! members read it back rather than throwing
//! (`rule:security/request-state-throws-in-an-isolate`). The runner freezes the
//! description beside the program and points `nvs run --request` at it —
//! [`request`] owns that file, and the three facts a case does not write.
//!
//! The last two are the peer's, and are Novis's own because `.phpt` has no way
//! to say either: `--CLIENT_IP--` is the address the request resolved to and
//! `--SCHEME--` is `http` or `https`. Both are *answers* rather than the
//! headers an answer is walked out of, which is why a case states them instead
//! of writing `x-forwarded-for`
//! (`rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`), and
//! both are read where they are written: an address that is not one, or a third
//! scheme, is a parse error naming the line.
//!
//! Two refusals come with them. `--POST--` and `--POST_RAW--` together are
//! refused rather than merged, since two spellings of one body cannot both be
//! it; and describing a request at all rules out every `--RUN--` but the
//! default, `nvs run` being the only subcommand that takes one.
//!
//! `--ENV--` **is** honoured: `Core\Env` is how a program reads back what the
//! section set. Its pairs are added to the environment the runner already
//! holds rather than replacing it — [`case::Case::env`] owns that rule.
//!
//! `--ARGS--` **is** honoured, since
//! `rule:tooling/commands-are-compiled`'s
//! `Core\Command::run` gives `nvs run` a command line to pass on. Its lines are
//! appended past the case file, so they are the program's arguments and never
//! the runner's, and each line is one argument with no splitting and no
//! quoting — [`case::Case::args`] owns why.
//!
//! ## The section that is deliberately absent
//!
//! There is no `--EXPECTREGEX--`, and it is not merely unwritten:
//! `--EXPECTF--`'s placeholders cover what a case actually varies over, and
//! a second pattern language is a second thing every case author has to
//! learn before writing the first one.

pub mod case;
pub mod expect;
pub mod request;
pub mod run;
pub mod section;

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

pub use case::{Case, Expectation, ParseError, Subcommand};
pub use run::{Options, Outcome, record_name, recording};

/// The extension a case file carries.
pub const EXTENSION: &str = "nvst";

/// What a whole run came to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Cases whose every expectation held.
    pub passed: usize,
    /// Cases that failed, including ones that could not be parsed or run.
    pub failed: usize,
    /// Cases `--SKIPIF--` declined.
    pub skipped: usize,
}

impl Summary {
    /// True when nothing failed.
    #[must_use]
    pub const fn is_success(self) -> bool {
        self.failed == 0
    }
}

/// Collects every `.nvst` file under `paths`, sorted, directories walked.
///
/// A path naming a file is taken as-is whatever its extension, so a single
/// case can be run by name.
///
/// # Errors
///
/// Fails when a path does not exist or a directory cannot be read.
pub fn discover(paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for path in paths {
        collect(path, &mut found)?;
    }
    found.sort();
    Ok(found)
}

fn collect(path: &Path, into: &mut Vec<PathBuf>) -> io::Result<()> {
    let meta = fs::metadata(path)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))?;
    if meta.is_file() {
        into.push(path.to_path_buf());
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<_>>()?;
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            collect(&entry, into)?;
        } else if entry.extension().is_some_and(|ext| ext == EXTENSION) {
            into.push(entry);
        }
    }
    Ok(())
}

/// The cases of `found` that `only` lists, in discovery order. A path is compared with either
/// slash and without a leading `./`, so a list written on one platform names the cases another
/// discovers.
///
/// # Errors
///
/// [`io::ErrorKind::NotFound`] naming the first listed path that is not one of `found`: a list
/// that names a case no tree holds was made against another tree, and running the rest would
/// report a pass for a case that never ran.
pub fn listed(found: Vec<PathBuf>, only: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    fn key(path: &Path) -> String {
        let text = path.to_string_lossy().replace('\\', "/");
        text.strip_prefix("./").unwrap_or(&text).to_owned()
    }
    let held: HashSet<String> = found.iter().map(|path| key(path)).collect();
    if let Some(missing) = only.iter().find(|path| !held.contains(&key(path))) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{}: not a case of the named trees", missing.display()),
        ));
    }
    let wanted: HashSet<String> = only.iter().map(|path| key(path)).collect();
    Ok(found
        .into_iter()
        .filter(|path| wanted.contains(&key(path)))
        .collect())
}

/// Runs every case in `paths`, writing a report to `out`.
///
/// The last line is always `N passed, M failed, K skipped` — the shape
/// `bun nv loop` reads to decide whether an `nvs-suite` check held.
///
/// Cases run [`Options::jobs`] at a time, which asks nothing of a case: each
/// runs in its own process with its own working directory, and the artifact
/// cache the processes share is written by atomic rename of content-addressed
/// files (`nvs-cli`'s `cache` module). The report is written in discovery
/// order — a worker's outcome waits until every case before it has been
/// reported — so two runs of the same tree print the same text whatever the
/// machine. What the pool buys is the difference between this tree dominating
/// every `nv verify` run, paid again by the loop's acceptance sweep, and
/// it costing a fraction of the build it rides on.
///
/// # Errors
///
/// With [`Options::only`], the cases are the listed ones of those `paths` hold,
/// judged and summed up the same way. With [`Options::record`], each case's
/// processes are recorded ([`recording`]).
///
/// Fails on a discovery error, a listed case no tree holds, or when `out`
/// cannot be written to. A failing *case* is not an error: it is counted in the
/// returned [`Summary`], and so
/// is a case whose process had to be killed for running past
/// [`run::CASE_TIMEOUT`]. A wedged case is one failure with a reason on it,
/// never a suite that stops reporting.
pub fn run(paths: &[PathBuf], opts: &Options, out: &mut dyn Write) -> io::Result<Summary> {
    // Everything is parsed before anything runs, so a malformed case is
    // reported without having spawned a compiler.
    let found = discover(paths)?;
    let found = match &opts.only {
        Some(only) => listed(found, only)?,
        None => found,
    };
    let parsed: Vec<(String, Result<Case, String>)> = found
        .into_iter()
        .map(|path| (path.display().to_string(), read(&path)))
        .filter(|(label, _)| {
            opts.filter
                .as_ref()
                .is_none_or(|filter| label.contains(filter.as_str()))
        })
        .collect();

    let root = &opts.root;
    fs::create_dir_all(root).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("could not create {}: {error}", root.display()),
        )
    })?;

    let mut summary = Summary::default();
    // A pool of workers each taking the next unstarted case off one counter, and one channel
    // back. The pool never exceeds the case count, so a tree of three cases spawns three
    // threads and not sixteen.
    let jobs = opts.jobs.clamp(1, parsed.len().max(1));
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel::<(usize, Outcome)>();
    thread::scope(|scope| {
        for _ in 0..jobs {
            let tx = tx.clone();
            let (parsed, next) = (&parsed, &next);
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((_, case)) = parsed.get(index) else {
                        break;
                    };
                    let workdir = root.join(index.to_string());
                    let outcome = match (fs::create_dir_all(&workdir), case) {
                        (Err(error), _) => Outcome::Fail(vec![format!(
                            "could not create its working directory: {error}"
                        )]),
                        (Ok(()), Err(error)) => Outcome::Fail(vec![error.clone()]),
                        (Ok(()), Ok(case)) => run::run_case(case, opts, &workdir),
                    };
                    let _ = fs::remove_dir_all(&workdir);
                    // The receiver is gone only when the report itself failed to write, and
                    // then there is nobody left to tell.
                    if tx.send((index, outcome)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);

        // Outcomes arrive in whatever order the workers finish; `held` keeps the early ones
        // until the case before them has reported, so the report reads in discovery order.
        let mut held = BTreeMap::new();
        let mut due = 0;
        for (index, outcome) in rx {
            held.insert(index, outcome);
            while let Some(outcome) = held.remove(&due) {
                report(out, &parsed[due].0, outcome, &mut summary)?;
                due += 1;
            }
        }
        Ok::<(), io::Error>(())
    })?;
    let _ = fs::remove_dir_all(root);

    writeln!(
        out,
        "{} passed, {} failed, {} skipped",
        summary.passed, summary.failed, summary.skipped
    )?;
    Ok(summary)
}

/// Counts one outcome into `summary`, and writes what a reader needs to see of it: nothing
/// for a pass, one line for a skip, the case's own report for a failure.
fn report(
    out: &mut dyn Write,
    label: &str,
    outcome: Outcome,
    summary: &mut Summary,
) -> io::Result<()> {
    match outcome {
        Outcome::Pass => summary.passed += 1,
        Outcome::Skip(reason) => {
            summary.skipped += 1;
            writeln!(out, "SKIP {label} — {reason}")?;
        }
        Outcome::Fail(lines) => {
            summary.failed += 1;
            writeln!(out, "FAIL {label}")?;
            for line in lines {
                writeln!(out, "{line}")?;
            }
            writeln!(out)?;
        }
    }
    Ok(())
}

/// Reads and parses the case at `path`, or the line to report instead.
fn read(path: &Path) -> Result<Case, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("could not read it: {error}"))?;
    case::parse(path, &text).map_err(|error| format!("not a valid case: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`Options::root`] is the only place a run writes: the root's missing
    /// parent is created beside the scratch guard, the root itself is gone when
    /// the run ends, and a root that cannot be created stops the run naming it.
    /// The case's `nvs` does not exist, so the case fails after its working
    /// directory was made.
    #[test]
    fn the_nvst_runner_writes_its_cases_under_the_root_it_is_given() {
        let scratch = nvs_repo::scratch("test-root");
        let tree = scratch.join("tree");
        fs::create_dir_all(&tree).expect("the tree's directory");
        fs::write(
            tree.join("one.nvst"),
            "--TEST--\none\n--FILE--\n<?nvs\necho 1;\n--EXPECT--\n1\n",
        )
        .expect("the case");

        let root = scratch.join("given").join("root");
        let mut opts = Options::from_current_exe(root.clone()).expect("this binary has a path");
        opts.nvs = scratch.join("no-such-nvs");
        let mut out = Vec::new();
        let summary = run(std::slice::from_ref(&tree), &opts, &mut out).expect("the run finishes");

        assert_eq!(summary.failed, 1, "{}", String::from_utf8_lossy(&out));
        assert!(
            scratch.join("given").is_dir(),
            "the root was made under the scratch guard"
        );
        assert!(!root.exists(), "the run deletes its root");

        fs::write(scratch.join("a-file"), b"").expect("a file in the root's way");
        opts.root = scratch.join("a-file").join("root");
        let error = run(std::slice::from_ref(&tree), &opts, &mut Vec::new())
            .expect_err("a root under a file cannot be created");
        assert!(error.to_string().contains("a-file"), "{error}");
    }

    #[test]
    fn a_summary_with_no_failures_is_a_success() {
        assert!(Summary::default().is_success());
        assert!(
            !Summary {
                passed: 3,
                failed: 1,
                skipped: 0
            }
            .is_success()
        );
    }

    #[test]
    fn a_case_list_keeps_the_listed_cases_in_discovery_order() {
        let found: Vec<PathBuf> = ["tree/a.nvst", r"tree\sub\b.nvst", "tree/c.nvst"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let only = [
            PathBuf::from("./tree/c.nvst"),
            PathBuf::from("tree/sub/b.nvst"),
        ];
        let kept = listed(found, &only).expect("both are in the tree");
        assert_eq!(
            kept,
            [
                PathBuf::from(r"tree\sub\b.nvst"),
                PathBuf::from("tree/c.nvst")
            ]
        );
    }

    #[test]
    fn a_case_list_naming_a_case_the_tree_lacks_is_refused() {
        let found = vec![PathBuf::from("tree/a.nvst")];
        let error =
            listed(found, &[PathBuf::from("tree/gone.nvst")]).expect_err("a case no tree holds");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("tree/gone.nvst"), "{error}");
    }

    #[test]
    fn discovery_names_the_path_it_could_not_read() {
        let error = discover(&[PathBuf::from("no/such/place")]).expect_err("a missing path fails");
        assert!(error.to_string().contains("no/such/place"), "{error}");
    }
}
