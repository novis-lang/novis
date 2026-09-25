//! The `.nvst` conformance-case format and the runner behind `nvs test`.
//!
//! A `.nvst` file is one case: a program, what it should print, and a title
//! saying what it is for. The format is deliberately a **superset of PHP's
//! `.phpt`**, so importing PHP's own test corpus at M11 is mechanical rather
//! than a rewrite — `docs/implementation-plan.md` § M4 is where that decision
//! lives.
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
//! These come straight from `.phpt` and mean what they mean there:
//!
//! | section | meaning |
//! |---|---|
//! | `--TEST--` | the one-line title, required |
//! | `--FILE--` | the Novis program, required |
//! | `--EXPECT--` | expected standard output, compared literally |
//! | `--EXPECTF--` | expected standard output, with [`expect`]'s `%` escapes |
//! | `--SKIPIF--` | a program whose output starting `skip` skips the case |
//! | `--CLEAN--` | a program run afterwards, whose output is ignored |
//! | `--INI--` | configuration for the run |
//! | `--ARGS--` | the program's own arguments, one per line |
//! | `--ENV--` | environment variables for the run, one `NAME=value` per line |
//! | `--GET--` | the query string the case's request carries, on one line |
//! | `--POST--` | that request's body as urlencoded pairs, on one line |
//! | `--POST_RAW--` | that request's body verbatim |
//! | `--COOKIE--` | its cookies, one `NAME=value` per line |
//! | `--HEADERS--` | its header fields, one `Name: value` per line |
//!
//! The rest are Novis's own, among them the differential pair:
//!
//! | section | meaning |
//! |---|---|
//! | `--ORACLE--` | a PHP twin whose standard output this case's must equal |
//! | `--ORACLE-DIVERGES--` | the one-line reason there is deliberately no twin |
//! | `--EXPECT-ERROR--` | expected standard error, compared literally |
//! | `--EXPECTF-ERROR--` | expected standard error, with `%` escapes |
//! | `--CLIENT_IP--` | the address the request's peer resolved to, on one line |
//! | `--SCHEME--` | `http` or `https`, the scheme that request arrived over |
//! | `--FILE <relative/path>--` | another file, written beside `--FILE--`; repeatable |
//! | `--RUN--` | `run` (the default), `test`, `test --format=json`, `test --format=junit`, `test --list --format=json` or `config dump --origin`: the command line `--FILE--` goes through |
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
//! (`case.nvs`, `skipif.nvs`, `clean.nvs`, `oracle.php`, `request.nvsr`) — so
//! a case cannot
//! reach outside the temporary directory it is given, and needs no sanitiser
//! to say so. Repeating one path is a parse error, the way repeating any
//! other section is.
//!
//! `--ORACLE--` is how the differential suite proves PHP compatibility
//! instead of freezing a belief about it: the expectation is not a string
//! someone typed, it is what PHP 8.5 does on the machine running the suite.
//! `--ORACLE-DIVERGES--` is the other half — where Novis differs from PHP on
//! purpose, the case states the reason and its own expectation, so a
//! divergence is a named, reviewable line rather than a comparison quietly
//! left out.
//!
//! The error pair exists because Novis writes a diagnostic and an uncaught
//! throw to **standard error**, where PHP writes both to standard output.
//! Without it, no case could cover a compile error at all. Their presence is
//! also the one thing that says a case expects the run to fail; every other
//! case must exit zero, so nothing can pass by printing the right prefix on
//! its way to a crash.
//!
//! A `--ORACLE--` case is **skipped**, once and with the reason named, on a
//! machine where the PHP binary cannot be run at all — that is an absent
//! oracle, not a failing comparison, and one identical failure per case would
//! bury the one line that says PHP is missing. The count is what
//! catches it: the differential suite's check in the goal records under
//! `data/goals/` sets a `minPassing` floor on *passing* cases, so a leg that
//! silently lost its oracle fails there.
//! A development machine carries PHP 8.5 on `PATH` on both sides of a Windows
//! setup — Windows and the WSL distro, at the same version — so the Linux leg
//! runs this suite rather than skipping it
//! ([docs/setup.md](/docs/setup.md)).
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
//! holds rather than replacing it, and both halves of a differential case get
//! them — [`case::Case::env`] owns both rules.
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

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

pub use case::{Case, Expectation, Oracle, ParseError, Subcommand};
pub use run::{Options, Outcome};

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
/// Fails on a discovery error, or when `out` cannot be written to. A failing
/// *case* is not an error: it is counted in the returned [`Summary`], and so
/// is a case whose process had to be killed for running past
/// [`run::CASE_TIMEOUT`]. A wedged case is one failure with a reason on it,
/// never a suite that stops reporting.
pub fn run(paths: &[PathBuf], opts: &Options, out: &mut dyn Write) -> io::Result<Summary> {
    // Everything is parsed before anything runs, for two reasons: a malformed
    // case is reported without having spawned a compiler, and the PHP probe
    // below only happens when some case actually wants an oracle.
    let parsed: Vec<(String, Result<Case, String>)> = discover(paths)?
        .into_iter()
        .map(|path| (path.display().to_string(), read(&path)))
        .filter(|(label, _)| {
            opts.filter
                .as_ref()
                .is_none_or(|filter| label.contains(filter.as_str()))
        })
        .collect();

    let wants_oracle = parsed
        .iter()
        .filter_map(|(_, case)| case.as_ref().ok())
        .any(|case| matches!(case.oracle, Some(Oracle::Php(_))));
    let php = !wants_oracle || run::php_available(opts);

    let root = std::env::temp_dir().join(format!("nvs-test-{}", std::process::id()));
    // A previous run that was killed before its cleanup leaves this behind.
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;

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
            let (parsed, next, root) = (&parsed, &next, &root);
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
                        (Ok(()), Ok(case)) => run::run_case(case, opts, &workdir, php),
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
    let _ = fs::remove_dir_all(&root);

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
    fn discovery_names_the_path_it_could_not_read() {
        let error = discover(&[PathBuf::from("no/such/place")]).expect_err("a missing path fails");
        assert!(error.to_string().contains("no/such/place"), "{error}");
    }
}
