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
//! Nine come straight from `.phpt` and mean what they mean there:
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
//!
//! Six are Novis's own. Two of those are the differential pair
//! `docs/agent/loop-goal.md` names:
//!
//! | section | meaning |
//! |---|---|
//! | `--ORACLE--` | a PHP twin whose standard output this case's must equal |
//! | `--ORACLE-DIVERGES--` | the one-line reason there is deliberately no twin |
//! | `--EXPECT-ERROR--` | expected standard error, compared literally |
//! | `--EXPECTF-ERROR--` | expected standard error, with `%` escapes |
//! | `--FILE <relative/path>--` | another file, written beside `--FILE--`; repeatable |
//! | `--RUN--` | `run` (the default), `test`, `test --format=json`, `test --format=junit` or `config dump --origin`: the command line `--FILE--` goes through |
//!
//! ## Which subcommand a case is run through
//!
//! `--RUN--` is how a case reaches
//! [ADR 0079](/docs/adr/0079-testing-is-a-language-feature.md)'s other
//! runner. `--RUN--\ntest` runs `nvs test case.nvs`, so the program declares
//! `#[Test]` classes and what the case pins is the *report* of running them —
//! the only way a `.nvst` can observe the `#[Test]` table at all, since a row
//! and its order are visible nowhere else. § 22's two machine formats are two
//! more spellings of the same thing — `test --format=json` and
//! `test --format=junit` — because a report is observable only by being read,
//! and a format nothing pins is a format that can drift. The roster is closed
//! to those four spellings, so a misspelling is a parse error rather than a
//! case quietly run the other way, and the section applies to `--FILE--`
//! alone: `--SKIPIF--` and `--CLEAN--` are the runner's own scaffolding and
//! are always `nvs run`.
//!
//! § 22's report carries a per-test duration, so such a case wants
//! `--EXPECTF--`'s `%f` rather than `--EXPECT--`.
//!
//! `--RUN--\nconfig dump --origin` is the one spelling that runs no program.
//! It names no file on the command line, so `nvs config dump` resolves
//! [ADR 0103](/docs/adr/0103-configuration-is-a-tree-of-files.md)
//! § 1 step 2's `./nvs.toml` out of the case's own working directory — which
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
//! [ADR 0061](/docs/adr/0061-compile-time-autoload-and-program-discovery.md)
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
//! or `..` segment or name one of the four files the runner writes itself
//! (`case.nvs`, `skipif.nvs`, `clean.nvs`, `oracle.php`) — so a case cannot
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
//! oracle, not a failing comparison, and reporting sixty identical failures
//! would bury the one line that says PHP is missing. The count is what
//! catches it: `docs/agent/loop-goal.toml` sets a floor on *passing*
//! differential cases, so a leg that silently lost its oracle fails there.
//! A development machine carries PHP 8.5 on `PATH` on both sides of a Windows
//! setup — Windows and the WSL distro, at the same version — so the Linux leg
//! runs this suite rather than skipping it
//! ([docs/setup.md](/docs/setup.md)).
//!
//! ## What is parsed but not yet honoured
//!
//! `--INI--` parses — that is what keeps the M11 importer mechanical — but
//! nothing can act on it yet: `nvs.toml` is not read until M6
//! ([ADR 0064](/docs/adr/0064-configuration-file-format.md)). A case
//! that uses it is reported as a **failure** naming the milestone, never
//! run-and-half-ignored.
//!
//! `--ENV--` **is** honoured, since `Core\Env` landed at M8 and a program can
//! now read back what the section set. Its pairs are added to the environment
//! the runner already holds rather than replacing it, and both halves of a
//! differential case get them — [`case::Case::env`] owns both rules.
//!
//! `--ARGS--` **is** honoured, since
//! [ADR 0086](/docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6's
//! `Core\Command::run` gave `nvs run` a command line to pass on. Its lines are
//! appended past the case file, so they are the program's arguments and never
//! the runner's, and each line is one argument with no splitting and no
//! quoting — [`case::Case::args`] owns why.
//!
//! ## Known gaps
//!
//! 1. Cases run one at a time. The suites are small enough that ordering the
//!    output deterministically is worth more than the wall-clock; revisit
//!    when a suite crosses the point where it is felt.
//! 2. There is no `--EXPECTREGEX--`. `--EXPECTF--` covers what the corpus
//!    needs so far, and a second pattern language is a second thing to learn.

pub mod case;
pub mod expect;
pub mod run;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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
/// `tools/loop.py` reads to decide whether a suite check held.
///
/// # Errors
///
/// Fails on a discovery error, or when `out` cannot be written to. A failing
/// *case* is not an error: it is counted in the returned [`Summary`].
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
    for (index, (label, case)) in parsed.iter().enumerate() {
        let workdir = root.join(index.to_string());
        fs::create_dir_all(&workdir)?;
        let outcome = match case {
            Ok(case) => run::run_case(case, opts, &workdir, php),
            Err(error) => Outcome::Fail(vec![error.clone()]),
        };
        let _ = fs::remove_dir_all(&workdir);

        match outcome {
            Outcome::Pass => summary.passed += 1,
            Outcome::Skip(reason) => {
                summary.skipped += 1;
                writeln!(out, "SKIP {label} — {reason}")?;
            }
            Outcome::Fail(report) => {
                summary.failed += 1;
                writeln!(out, "FAIL {label}")?;
                for line in report {
                    writeln!(out, "{line}")?;
                }
                writeln!(out)?;
            }
        }
    }
    let _ = fs::remove_dir_all(&root);

    writeln!(
        out,
        "{} passed, {} failed, {} skipped",
        summary.passed, summary.failed, summary.skipped
    )?;
    Ok(summary)
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
