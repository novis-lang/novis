//! `nvs fmt` — the formatter as a command: which files it reads, and where the
//! text it produces goes.
//!
//! Every layout decision belongs to [`nvs_fmt`] and none of them is here. This
//! module resolves the paths, calls [`nvs_fmt::format`] once per file, and puts
//! the answer where the I/O mode says to put it.
//! `rule:tooling/fmt-check-writes-nothing` is that set of modes — in place by
//! default, `--check` naming each file that would change, `--diff` showing the
//! change itself, `--stdin` formatting a buffer an editor holds — and
//! `rule:tooling/fmt-is-one-canonical-style` is why they are the only flags
//! there are: not one of them changes a byte of what the formatter produces.
//!
//! **A directory is walked, a named file is taken as named.** Inside a
//! directory only `.nvs` files are collected, in sorted order so two runs over
//! one tree report in the same order; a path given on the command line is
//! formatted whatever it is called, because naming it is already the choice the
//! extension filter exists to make for a walk. The walk is
//! [`nvs_hir::lenders::sources`], the one every command that reads a project
//! uses: it skips what a `.gitignore` or `.ignore` names, and every directory
//! named `vendor` or `target` and every dot-directory under the path it starts
//! from. Naming one of those on the command line formats it.
//!
//! **One file's refusal never stops the walk.** Each file is its own parse
//! ([`nvs_fmt`]'s module doc), so a file that does not parse is named on
//! standard error with its diagnostics rendered under it and the files after it
//! are still formatted.
//!
//! **The exit code says what the run found** ([`Found`]):
//! `rule:tooling/fmt-exit-status-says-what-it-found`. `1` is a file that would
//! change, which running `nvs fmt` fixes, and `2` is a file refused or not read
//! or written, which it does not; `2` wins when one run finds both.
//!
//! **Nothing is written when the bytes already match.** A file in its canonical
//! layout keeps its modification time, which is what an editor watching the
//! tree and a build system reading timestamps both go by.
//!
//! # The diff
//!
//! `--diff` prints a unified diff with three lines of context, built from the
//! longest common subsequence of the two line vectors after their common prefix
//! and suffix are trimmed. Lines are split on `\n` rather than taken from
//! [`str::lines`], so the vectors are equal exactly when the texts are: a file
//! whose only difference is the newline it ends with is one this shows rather
//! than one it silently calls unchanged.
//!
//! The table that subsequence needs is quadratic in the trimmed middle, so past
//! [`CELLS`] the middle is printed as one replacement instead of being diffed.
//! A formatter's diff is a convenience over a file a person is about to read,
//! and not worth an unbounded allocation for the pathological case where a
//! rewrite touches every line.

use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_diagnostics::SourceMap;


/// What a buffer read from standard input is called in a diagnostic.
const STDIN_NAME: &str = "<stdin>";

/// How many unchanged lines a hunk carries on each side of a change.
const CONTEXT: usize = 3;

/// The largest subsequence table `--diff` allocates, in cells.
const CELLS: usize = 1 << 20;


/// What one file, or one run, found: the worst wins, and it is the exit code.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Found {
    /// Already canonical, or rewritten in place.
    Settled,
    /// A mode that writes nothing found a file that would change. Exit `1`.
    Changed,
    /// Refused, or not read or written. Exit `2`.
    Refused,
}

impl From<Found> for ExitCode {
    fn from(found: Found) -> Self {
        match found {
            Found::Settled => ExitCode::SUCCESS,
            Found::Changed => ExitCode::from(1),
            Found::Refused => ExitCode::from(2),
        }
    }
}

/// Where a formatted file's text goes.
///
/// The three modes over the named paths; `--stdin` is [`stdin`] rather than a
/// fourth variant, because it resolves no path at all, and it takes
/// [`Mode::Write`] or [`Mode::Check`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    /// Rewrite each file that would change. The default.
    Write,
    /// Write nothing, and name each file that would change.
    Check,
    /// Write nothing, and print a unified diff per file that would change.
    Diff,
}

/// Rewrite Novis files into the one canonical layout.
///
/// Takes no configuration and has no style flag: the output is a pure
/// function of the input bytes, so two projects run through this tool
/// cannot disagree about what formatted means. The flags below are I/O
/// modes, and each one decides only where the text goes.
///
/// A file whose parse reports an error is refused rather than rewritten,
/// named on standard error, and left exactly as it was.
// `rule:tooling/fmt-is-one-canonical-style` and
// `rule:tooling/fmt-check-writes-nothing`.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The files and directories to format. A directory is walked for the
    /// `.nvs` files under it.
    #[arg(required_unless_present = "stdin")]
    paths: Vec<PathBuf>,
    /// Write nothing and print the name of each file that would change.
    /// The exit code is 1 if a file would change, and 2 if a file has an
    /// error. With `--stdin`, the name printed is `<stdin>`.
    #[arg(long, alias = "dry-run", conflicts_with = "diff")]
    check: bool,
    /// Write nothing and print a unified diff for each file that would
    /// change. The exit code is 1 if a file would change, and 2 if a file
    /// has an error.
    // Refused beside `--check` for the reason `nvs check`'s `--json` is
    // refused beside `--autoload-map`: both render the same list onto the
    // same standard output, and one printed after the other is neither.
    #[arg(long, conflicts_with = "stdin")]
    diff: bool,
    /// Format what standard input holds onto standard output, naming no
    /// file. A refusal writes nothing at all to standard output.
    #[arg(long, conflicts_with = "paths")]
    stdin: bool,
}

impl Args {
    /// Runs `nvs fmt` with the arguments clap parsed.
    pub(crate) fn run(self) -> ExitCode {
        let mode = if self.check {
            Mode::Check
        } else if self.diff {
            Mode::Diff
        } else {
            Mode::Write
        };
        if self.stdin {
            stdin(mode).into()
        } else {
            run(&self.paths, mode).into()
        }
    }
}

/// Formats every file `paths` names, and answers the worst any of them found.
pub(crate) fn run(paths: &[PathBuf], mode: Mode) -> Found {
    let mut files = Vec::new();
    for path in paths {
        if let Err(err) = collect(path, &mut files) {
            eprintln!("error: could not read {}: {err}", path.display());
            return Found::Refused;
        }
    }

    files
        .iter()
        .map(|file| one(file, mode))
        .max()
        .unwrap_or(Found::Settled)
}

/// Formats what standard input holds: onto standard output under
/// [`Mode::Write`], and under [`Mode::Check`] only as [`STDIN_NAME`] when it
/// would change.
///
/// A refusal writes nothing at all to standard output — the buffer an editor
/// sent is the buffer it keeps, rather than one this replaced with a partial
/// rendering of a file its author has not finished typing.
pub(crate) fn stdin(mode: Mode) -> Found {
    let mut source = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut source) {
        eprintln!("error: could not read standard input: {err}");
        return Found::Refused;
    }

    let mut map = SourceMap::new();
    let id = map.add(STDIN_NAME, source);
    match nvs_fmt::format(map.file(id)) {
        Ok(formatted) if mode == Mode::Check => {
            if formatted == map.file(id).text() {
                Found::Settled
            } else {
                println!("{STDIN_NAME}");
                Found::Changed
            }
        }
        Ok(formatted) => {
            print!("{formatted}");
            Found::Settled
        }
        Err(refusal) => {
            eprintln!("error: {refusal}");
            crate::render_diagnostics(&mut refusal.into_diagnostics(), &map);
            Found::Refused
        }
    }
}

/// Adds `path` to `into`: itself, or every `.nvs` file the project walk
/// [`nvs_hir::lenders::sources`] finds under it, in sorted order.
///
/// A directory that cannot be listed is an error here, because it is the one
/// the command line named. A directory under it that cannot be listed is
/// skipped, as the walk skips it for every other command.
fn collect(path: &Path, into: &mut Vec<PathBuf>) -> io::Result<()> {
    if !path.is_dir() {
        into.push(path.to_path_buf());
        return Ok(());
    }

    fs::read_dir(path)?;
    let mut found = nvs_hir::lenders::sources(path);
    found.sort();
    into.extend(found);
    Ok(())
}

/// One file, through the mode in force.
///
/// [`Found::Refused`] once the file was refused or could not be read or
/// written, and [`Found::Changed`] once a mode that writes nothing found that
/// it would change.
fn one(path: &Path, mode: Mode) -> Found {
    let mut map = SourceMap::new();
    let id = match map.load(path) {
        Ok(id) => id,
        Err(err) => {
            crate::report_unreadable(path, &err, crate::Sink::Text);
            return Found::Refused;
        }
    };

    let before = map.file(id).text().to_owned();
    let after = match nvs_fmt::format(map.file(id)) {
        Ok(text) => text,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            crate::render_diagnostics(&mut refusal.into_diagnostics(), &map);
            return Found::Refused;
        }
    };
    if after == before {
        return Found::Settled;
    }

    match mode {
        Mode::Write => match fs::write(path, &after) {
            Ok(()) => Found::Settled,
            Err(err) => {
                eprintln!("error: could not write {}: {err}", path.display());
                Found::Refused
            }
        },
        Mode::Check => {
            println!("{}", path.display());
            Found::Changed
        }
        Mode::Diff => {
            print!("{}", unified(&path.display().to_string(), &before, &after));
            Found::Changed
        }
    }
}

/// One line of a diff, as what happened to it.
enum Op<'a> {
    /// In both texts.
    Keep(&'a str),
    /// In the file as it stands, and not in its canonical layout.
    Del(&'a str),
    /// In the canonical layout, and not in the file as it stands.
    Ins(&'a str),
}

/// `before` against `after`, as a unified diff naming `name` on both sides.
fn unified(name: &str, before: &str, after: &str) -> String {
    let old: Vec<&str> = before.split('\n').collect();
    let new: Vec<&str> = after.split('\n').collect();
    render(name, &script(&old, &new))
}

/// The edit script from `old` to `new`, one [`Op`] per line of either.
fn script<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Op<'a>> {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (opening, middle) = old.split_at(prefix);
    let inserted = &new[prefix..new.len() - suffix];
    let (middle, closing) = middle.split_at(middle.len() - suffix);

    let mut ops: Vec<Op<'a>> = opening.iter().copied().map(Op::Keep).collect();
    if middle.len().saturating_mul(inserted.len()) > CELLS {
        ops.extend(middle.iter().copied().map(Op::Del));
        ops.extend(inserted.iter().copied().map(Op::Ins));
    } else {
        ops.extend(subsequence(middle, inserted));
    }
    ops.extend(closing.iter().copied().map(Op::Keep));
    ops
}

/// The edit script the longest common subsequence of `a` and `b` describes.
fn subsequence<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<Op<'a>> {
    let width = b.len() + 1;
    let mut table = vec![0usize; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i * width + j] = if a[i] == b[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }

    let mut ops = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            ops.push(Op::Keep(a[i]));
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            ops.push(Op::Del(a[i]));
            i += 1;
        } else {
            ops.push(Op::Ins(b[j]));
            j += 1;
        }
    }
    ops.extend(a[i..].iter().copied().map(Op::Del));
    ops.extend(b[j..].iter().copied().map(Op::Ins));
    ops
}

/// `ops` as the text of a unified diff, empty when nothing changed.
fn render(name: &str, ops: &[Op<'_>]) -> String {
    let mut position = Vec::with_capacity(ops.len() + 1);
    let (mut old_no, mut new_no) = (1usize, 1usize);
    for op in ops {
        position.push((old_no, new_no));
        match op {
            Op::Keep(_) => {
                old_no += 1;
                new_no += 1;
            }
            Op::Del(_) => old_no += 1,
            Op::Ins(_) => new_no += 1,
        }
    }
    position.push((old_no, new_no));

    let changed: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, op)| !matches!(op, Op::Keep(_)))
        .map(|(index, _)| index)
        .collect();
    if changed.is_empty() {
        return String::new();
    }

    let mut out = format!("--- {name}\n+++ {name}\n");
    let mut at = 0;
    while at < changed.len() {
        // One hunk runs to the last change whose own context still touches the
        // one before it; two hunks that would print the same line between them
        // are one hunk.
        let first = changed[at];
        let mut last = first;
        while at + 1 < changed.len() && changed[at + 1] <= last + 2 * CONTEXT + 1 {
            at += 1;
            last = changed[at];
        }
        at += 1;

        let start = first.saturating_sub(CONTEXT);
        let end = (last + 1 + CONTEXT).min(ops.len());
        let lines = &ops[start..end];
        let old_count = lines.iter().filter(|op| !matches!(op, Op::Ins(_))).count();
        let new_count = lines.iter().filter(|op| !matches!(op, Op::Del(_))).count();
        let (old_start, new_start) = position[start];
        out.push_str(&format!(
            "@@ -{},{old_count} +{},{new_count} @@\n",
            head(old_start, old_count),
            head(new_start, new_count),
        ));
        for op in lines {
            let (marker, line) = match op {
                Op::Keep(line) => (' ', line),
                Op::Del(line) => ('-', line),
                Op::Ins(line) => ('+', line),
            };
            out.push(marker);
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// A hunk header's start line: the line before an empty range, which is what
/// the unified format says an insertion into nothing sits after.
fn head(start: usize, count: usize) -> usize {
    if count == 0 { start - 1 } else { start }
}
