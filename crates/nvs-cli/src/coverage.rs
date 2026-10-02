//! `nvs test --coverage-lcov <FILE>` and `--coverage-clover <FILE>`: line
//! coverage of a `#[Test]` run, written in the two formats CI services read.
//!
//! # Where the counts come from
//!
//! `rule:testing/debug-probes`'s statement probe is compiled into every unit.
//! The runner gives the suite's context one `nvs_runtime::StmtHits` table and
//! turns `DebugFlags::COVERAGE` on, and every context the run makes from it —
//! each test's isolate, a fixture, a request a `server: true` test sends —
//! counts into that same table. The probe passes a program-wide statement
//! number, and `nvs_ir::Program::stmt_spans` is the table that says where each
//! number is written. [`Sites::of`] turns those spans into file lines before
//! the suite runs, because the source map moves into the suite's task.
//!
//! # From statements to lines
//!
//! Both formats count lines, and one line can hold several statements: a
//! `while` and the block it opens start on the same line. A line's count is
//! the **largest** count of a statement starting on it, so a line reads as run
//! when any statement on it ran, and a loop header is not counted twice. Every
//! function in the program is compiled, so a statement that never ran is in
//! the report with `0`. A file with no statement in it is not in the report,
//! and neither is one the program did not read from disk: the entry file a
//! directory run generates has no file a CI service could show.
//!
//! # How a file is named
//!
//! Relative to the directory `nvs test` runs in when the file is under it,
//! and absolute otherwise, with `/` between the parts on every platform. A CI
//! service matches the names against the repository, so a run from the
//! repository's root gives names it finds. Windows' `\\?\` prefix, which a
//! required file's resolved path carries, is removed.
//!
//! # What it spends
//!
//! Nothing for a run that does not ask: no table is made and no span is read.
//! A run that asks holds one `u64` per statement in the program, plus one file
//! and line pair per statement for the report.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The coverage files a `nvs test` run was asked to write.
#[derive(Default)]
pub(crate) struct Requested {
    /// `--coverage-lcov <FILE>`.
    pub(crate) lcov: Option<PathBuf>,
    /// `--coverage-clover <FILE>`.
    pub(crate) clover: Option<PathBuf>,
}

impl Requested {
    /// Whether any coverage file was asked for, which is what turns counting on.
    pub(crate) fn any(&self) -> bool {
        self.lcov.is_some() || self.clover.is_some()
    }
}

/// Where each probe number is written: a file and a one-based line.
pub(crate) struct Sites {
    /// Each file's name, as [`shown`] writes it, and its line count. Sorted by
    /// name, so a report lists files in the same order on every run.
    files: Vec<(String, usize)>,
    /// One entry per probe number: the index into `files` and the line, or
    /// `None` for a statement in a file the report leaves out.
    at: Vec<Option<(usize, usize)>>,
}

impl Sites {
    /// Resolves `spans` — `nvs_ir::Program::stmt_spans` of the program — against
    /// the source map the program was read into.
    pub(crate) fn of(spans: &[nvs_diagnostics::Span], map: &nvs_diagnostics::SourceMap) -> Self {
        let here = std::env::current_dir().ok().map(|dir| plain(&dir));
        // Each file once: its name in the report, or `None` for a file that is
        // not on disk.
        let mut named: BTreeMap<nvs_diagnostics::SourceId, Option<String>> = BTreeMap::new();
        let mut sizes: BTreeMap<String, usize> = BTreeMap::new();
        for span in spans {
            named.entry(span.file).or_insert_with(|| {
                let file = map.get(span.file)?;
                let path = file.path().filter(|path| path.is_file())?;
                let name = shown(path, here.as_deref());
                sizes.insert(name.clone(), file.line_count());
                Some(name)
            });
        }
        let files: Vec<(String, usize)> = sizes.into_iter().collect();
        let at = spans
            .iter()
            .map(|span| {
                let name = named.get(&span.file)?.as_ref()?;
                let index = files.binary_search_by(|(known, _)| known.cmp(name)).ok()?;
                let line = map.get(span.file)?.line_col(span.start).0 + 1;
                Some((index, line))
            })
            .collect();
        Self { files, at }
    }

    /// How many probe numbers there are, which is the size the run's
    /// `StmtHits` table needs.
    pub(crate) fn len(&self) -> usize {
        self.at.len()
    }

    /// Each file that has a statement, with its lines and their counts.
    fn lines(&self, counts: &[u64]) -> Vec<FileLines<'_>> {
        let mut per_file: Vec<BTreeMap<usize, u64>> = vec![BTreeMap::new(); self.files.len()];
        for (site, &count) in self.at.iter().zip(counts) {
            if let Some((file, line)) = *site {
                let slot = per_file[file].entry(line).or_insert(0);
                *slot = (*slot).max(count);
            }
        }
        self.files
            .iter()
            .zip(per_file)
            .filter(|(_, lines)| !lines.is_empty())
            .map(|((name, line_count), lines)| FileLines {
                name,
                line_count: *line_count,
                lines,
            })
            .collect()
    }
}

/// How a report names the file at `path`: see the module docs. `here` is the
/// current directory, already passed through [`plain`].
fn shown(path: &Path, here: Option<&Path>) -> String {
    let path = plain(path);
    let relative = here
        .and_then(|here| path.strip_prefix(here).ok())
        .unwrap_or(&path);
    relative.to_string_lossy().replace('\\', "/")
}

/// `path` without Windows' `\\?\` prefix. A `\\?\UNC\` path is kept as it
/// is, because removing only the prefix would not give a network path.
fn plain(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// One file's part of a report.
struct FileLines<'a> {
    name: &'a str,
    line_count: usize,
    /// Line number to count, for every line a statement starts on.
    lines: BTreeMap<usize, u64>,
}

impl FileLines<'_> {
    fn hit(&self) -> usize {
        self.lines.values().filter(|&&count| count > 0).count()
    }
}

/// How many lines a run's report covers, and how many of them ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Summary {
    pub(crate) lines: usize,
    pub(crate) hit: usize,
}

/// Writes every file `requested` names from `counts`, the run's
/// `StmtHits::counts`, and returns what the report covers.
///
/// # Errors
///
/// The message to print when a file could not be written.
pub(crate) fn write(
    requested: &Requested,
    sites: &Sites,
    counts: &[u64],
) -> Result<Summary, String> {
    let files = sites.lines(counts);
    if let Some(path) = &requested.lcov {
        std::fs::write(path, lcov(&files))
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    if let Some(path) = &requested.clover {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        std::fs::write(path, clover(&files, now))
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    Ok(Summary {
        lines: files.iter().map(|file| file.lines.len()).sum(),
        hit: files.iter().map(FileLines::hit).sum(),
    })
}

/// The lcov tracefile: one `SF` record per file, a `DA` line per line a
/// statement starts on, and the `LF`/`LH` totals.
fn lcov(files: &[FileLines<'_>]) -> String {
    let mut out = String::from("TN:\n");
    for file in files {
        out.push_str(&format!("SF:{}\n", file.name));
        for (line, count) in &file.lines {
            out.push_str(&format!("DA:{line},{count}\n"));
        }
        out.push_str(&format!(
            "LF:{}\nLH:{}\nend_of_record\n",
            file.lines.len(),
            file.hit()
        ));
    }
    out
}

/// The Clover XML document: one `<file>` per file with a `<line type="stmt">`
/// per line and its `<metrics>`, then the project's `<metrics>`. `now` is the
/// Unix time the `generated` and `timestamp` attributes carry.
fn clover(files: &[FileLines<'_>], now: u64) -> String {
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<coverage generated=\"{now}\">\n  <project timestamp=\"{now}\">\n"
    );
    let (mut loc, mut statements, mut covered) = (0, 0, 0);
    for file in files {
        out.push_str("    <file name=\"");
        crate::runner::xml_text(file.name, &mut out);
        out.push_str("\">\n");
        for (line, count) in &file.lines {
            out.push_str(&format!(
                "      <line num=\"{line}\" type=\"stmt\" count=\"{count}\"/>\n"
            ));
        }
        out.push_str("      ");
        metrics(
            &mut out,
            None,
            file.line_count,
            file.lines.len(),
            file.hit(),
        );
        out.push_str("    </file>\n");
        loc += file.line_count;
        statements += file.lines.len();
        covered += file.hit();
    }
    out.push_str("    ");
    metrics(&mut out, Some(files.len()), loc, statements, covered);
    out.push_str("  </project>\n</coverage>\n");
    out
}

/// One Clover `<metrics>` element. Methods and conditionals are `0`, because a
/// line report counts neither.
fn metrics(out: &mut String, files: Option<usize>, loc: usize, statements: usize, covered: usize) {
    let files = files.map_or(String::new(), |count| format!("files=\"{count}\" "));
    out.push_str(&format!(
        "<metrics {files}loc=\"{loc}\" ncloc=\"{loc}\" classes=\"0\" methods=\"0\" coveredmethods=\"0\" \
         conditionals=\"0\" coveredconditionals=\"0\" statements=\"{statements}\" \
         coveredstatements=\"{covered}\" elements=\"{statements}\" coveredelements=\"{covered}\"/>\n"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two files out of order, a line holding two statements, and a statement
    /// that never ran.
    fn sample() -> (Sites, Vec<u64>) {
        let sites = Sites {
            files: vec![("a.nvs".to_owned(), 10), ("b.nvs".to_owned(), 4)],
            at: vec![Some((1, 2)), Some((0, 3)), Some((0, 3)), Some((0, 5)), None],
        };
        (sites, vec![1, 1, 2, 0, 7])
    }

    #[test]
    fn a_line_counts_its_busiest_statement_and_a_line_that_never_ran_reads_zero() {
        let (sites, counts) = sample();
        let files = sites.lines(&counts);
        assert_eq!(
            lcov(&files),
            "TN:\nSF:a.nvs\nDA:3,2\nDA:5,0\nLF:2\nLH:1\nend_of_record\n\
             SF:b.nvs\nDA:2,1\nLF:1\nLH:1\nend_of_record\n"
        );
    }

    #[test]
    fn the_clover_document_carries_a_line_per_statement_line_and_the_totals() {
        let (sites, counts) = sample();
        let files = sites.lines(&counts);
        let document = clover(&files, 1_700_000_000);
        assert!(document.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<coverage generated=\"1700000000\">\n"
        ));
        assert!(document.contains("<file name=\"a.nvs\">\n      <line num=\"3\" type=\"stmt\" count=\"2\"/>\n      <line num=\"5\" type=\"stmt\" count=\"0\"/>\n"));
        assert!(document.contains(
            "<metrics files=\"2\" loc=\"14\" ncloc=\"14\" classes=\"0\" methods=\"0\" coveredmethods=\"0\" \
             conditionals=\"0\" coveredconditionals=\"0\" statements=\"3\" coveredstatements=\"2\" \
             elements=\"3\" coveredelements=\"2\"/>"
        ));
    }

    #[test]
    #[cfg(unix)]
    fn a_file_is_named_relative_to_the_current_directory() {
        let here = Path::new("/work/app");
        assert_eq!(
            shown(Path::new("/work/app/src/Cart.nvs"), Some(here)),
            "src/Cart.nvs"
        );
        assert_eq!(
            shown(Path::new("tests/CartTest.nvs"), Some(here)),
            "tests/CartTest.nvs"
        );
        assert_eq!(
            shown(Path::new("/shared/Lib.nvs"), Some(here)),
            "/shared/Lib.nvs"
        );
    }

    #[test]
    #[cfg(windows)]
    fn a_file_is_named_relative_to_the_current_directory_with_forward_slashes() {
        let here = Path::new(r"D:\work\app");
        assert_eq!(
            shown(Path::new(r"\\?\D:\work\app\src\Cart.nvs"), Some(here)),
            "src/Cart.nvs"
        );
        assert_eq!(
            shown(Path::new(r"tests\CartTest.nvs"), Some(here)),
            "tests/CartTest.nvs"
        );
        assert_eq!(
            shown(Path::new(r"\\?\E:\shared\Lib.nvs"), Some(here)),
            "E:/shared/Lib.nvs"
        );
    }

    #[test]
    fn a_file_name_is_escaped_in_the_clover_document() {
        let sites = Sites {
            files: vec![("a&b.nvs".to_owned(), 1)],
            at: vec![Some((0, 1))],
        };
        assert!(clover(&sites.lines(&[1]), 0).contains("<file name=\"a&amp;b.nvs\">"));
    }
}
