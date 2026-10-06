//! `nvs test --coverage-lcov <FILE>`, `--coverage-clover <FILE>` and
//! `--coverage-cobertura <FILE>`: line and function coverage of a `#[Test]`
//! run, written in the three formats CI services read (ADR 0273).
//!
//! # Where the counts come from
//!
//! `rule:testing/debug-probes`'s statement probe is compiled into every unit.
//! The runner gives the suite's context one `nvs_runtime::StmtHits` table and
//! turns `DebugFlags::COVERAGE` on, and every context the run makes from it —
//! each test's isolate, a fixture, a request a `server: true` test sends —
//! counts into that same table. The probe passes a program-wide statement
//! number, and [`Probes::of`] reads where each number is written, and which
//! function each starts, from the lowered program. [`Sites::of`] turns those
//! into file lines before the suite runs, because the source map moves into
//! the suite's task.
//!
//! # From statements to lines
//!
//! Every format counts lines, and one line can hold several statements: a
//! `while` and the block it opens start on the same line. A line's count is
//! the **largest** count of a statement starting on it, so a line reads as run
//! when any statement on it ran, and a loop header is not counted twice. Every
//! function in the program is compiled, so a statement that never ran is in
//! the report with `0`. A file with no statement in it is not in the report,
//! and neither is one the program did not read from disk: the entry file a
//! directory run generates has no file a CI service could show.
//!
//! # Functions
//!
//! A function is one `nvs_ir::Function`, under the name the IR gives it. It
//! is listed at the line of its first statement with that statement's count,
//! which is how many times it was called: the first statement runs on every
//! call. A function with no statement has no probe, so it is not listed.
//! lcov and Clover carry functions. Cobertura carries lines only.
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
//! and line pair per statement and one name per function for the report.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The coverage files a `nvs test` run was asked to write.
#[derive(Default)]
pub(crate) struct Requested {
    /// `--coverage-lcov <FILE>`.
    pub(crate) lcov: Option<PathBuf>,
    /// `--coverage-clover <FILE>`.
    pub(crate) clover: Option<PathBuf>,
    /// `--coverage-cobertura <FILE>`.
    pub(crate) cobertura: Option<PathBuf>,
}

impl Requested {
    /// Whether any coverage file was asked for, which is what turns counting on.
    pub(crate) fn any(&self) -> bool {
        self.lcov.is_some() || self.clover.is_some() || self.cobertura.is_some()
    }
}

/// A program's probes, read from the lowered program before it is dropped.
#[derive(Default)]
pub(crate) struct Probes {
    /// `nvs_ir::Program::stmt_spans`: where each statement number is written.
    stmts: Vec<nvs_diagnostics::Span>,
    /// Each function that has a statement: its name and the number of its
    /// first statement, from the same walk over `Program::functions` that
    /// gives each function its base.
    functions: Vec<(String, usize)>,
}

impl Probes {
    pub(crate) fn of(program: &nvs_ir::Program) -> Self {
        let mut functions = Vec::new();
        let mut base = 0;
        for function in &program.functions {
            if !function.stmt_spans.is_empty() {
                functions.push((function.name.clone(), base));
            }
            base += function.stmt_spans.len();
        }
        Self {
            stmts: program.stmt_spans(),
            functions,
        }
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
    /// [`Probes::functions`], unchanged.
    functions: Vec<(String, usize)>,
}

impl Sites {
    /// Resolves `probes` against the source map the program was read into.
    pub(crate) fn of(probes: Probes, map: &nvs_diagnostics::SourceMap) -> Self {
        let Probes {
            stmts: spans,
            functions,
        } = probes;
        let here = std::env::current_dir().ok().map(|dir| plain(&dir));
        // Each file once: its name in the report, or `None` for a file that is
        // not on disk.
        let mut named: BTreeMap<nvs_diagnostics::SourceId, Option<String>> = BTreeMap::new();
        let mut sizes: BTreeMap<String, usize> = BTreeMap::new();
        for span in &spans {
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
        Self {
            files,
            at,
            functions,
        }
    }

    /// How many probe numbers there are, which is the size the run's
    /// `StmtHits` table needs.
    pub(crate) fn len(&self) -> usize {
        self.at.len()
    }

    /// Each file that has a statement, with its lines, its functions and
    /// their counts.
    fn lines(&self, counts: &[u64]) -> Vec<FileLines<'_>> {
        let mut per_file: Vec<BTreeMap<usize, u64>> = vec![BTreeMap::new(); self.files.len()];
        for (site, &count) in self.at.iter().zip(counts) {
            if let Some((file, line)) = *site {
                let slot = per_file[file].entry(line).or_insert(0);
                *slot = (*slot).max(count);
            }
        }
        let mut functions: Vec<Vec<Function<'_>>> = vec![Vec::new(); self.files.len()];
        for (name, first) in &self.functions {
            if let Some(&Some((file, line))) = self.at.get(*first) {
                functions[file].push(Function {
                    name,
                    line,
                    count: counts.get(*first).copied().unwrap_or(0),
                });
            }
        }
        for list in &mut functions {
            list.sort_by(|a, b| (a.line, a.name).cmp(&(b.line, b.name)));
        }
        self.files
            .iter()
            .zip(per_file)
            .zip(functions)
            .filter(|((_, lines), _)| !lines.is_empty())
            .map(|(((name, line_count), lines), functions)| FileLines {
                name,
                line_count: *line_count,
                lines,
                functions,
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
    /// The functions whose first statement is in this file, by line.
    functions: Vec<Function<'a>>,
}

impl FileLines<'_> {
    fn hit(&self) -> usize {
        self.lines.values().filter(|&&count| count > 0).count()
    }

    fn functions_hit(&self) -> usize {
        self.functions
            .iter()
            .filter(|function| function.count > 0)
            .count()
    }
}

/// One function in a report: see the module docs.
#[derive(Clone)]
struct Function<'a> {
    name: &'a str,
    line: usize,
    count: u64,
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
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    if let Some(path) = &requested.clover {
        std::fs::write(path, clover(&files, now))
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    if let Some(path) = &requested.cobertura {
        std::fs::write(path, cobertura(&files, now))
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    }
    Ok(Summary {
        lines: files.iter().map(|file| file.lines.len()).sum(),
        hit: files.iter().map(FileLines::hit).sum(),
    })
}

/// The lcov tracefile: one `SF` record per file, with an `FN` and an `FNDA`
/// line per function and the `FNF`/`FNH` totals, then a `DA` line per line a
/// statement starts on and the `LF`/`LH` totals.
fn lcov(files: &[FileLines<'_>]) -> String {
    let mut out = String::from("TN:\n");
    for file in files {
        out.push_str(&format!("SF:{}\n", file.name));
        for function in &file.functions {
            out.push_str(&format!("FN:{},{}\n", function.line, function.name));
        }
        for function in &file.functions {
            out.push_str(&format!("FNDA:{},{}\n", function.count, function.name));
        }
        out.push_str(&format!(
            "FNF:{}\nFNH:{}\n",
            file.functions.len(),
            file.functions_hit()
        ));
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

/// The Clover XML document: one `<file>` per file with a `<line type="method">`
/// per function and a `<line type="stmt">` per line, in line order, and its
/// `<metrics>`, then the project's `<metrics>`. `now` is the Unix time the
/// `generated` and `timestamp` attributes carry.
fn clover(files: &[FileLines<'_>], now: u64) -> String {
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<coverage generated=\"{now}\">\n  <project timestamp=\"{now}\">\n"
    );
    let mut total = Metrics::default();
    for file in files {
        out.push_str("    <file name=\"");
        crate::runner::xml_text(file.name, &mut out);
        out.push_str("\">\n");
        let mut functions = file.functions.iter().peekable();
        for (&line, count) in &file.lines {
            while let Some(function) = functions.next_if(|function| function.line <= line) {
                out.push_str(&format!(
                    "      <line num=\"{}\" type=\"method\" name=\"",
                    function.line
                ));
                crate::runner::xml_text(function.name, &mut out);
                out.push_str(&format!("\" count=\"{}\"/>\n", function.count));
            }
            out.push_str(&format!(
                "      <line num=\"{line}\" type=\"stmt\" count=\"{count}\"/>\n"
            ));
        }
        let own = Metrics {
            loc: file.line_count,
            statements: file.lines.len(),
            covered: file.hit(),
            methods: file.functions.len(),
            covered_methods: file.functions_hit(),
        };
        out.push_str("      ");
        own.write(&mut out, None);
        out.push_str("    </file>\n");
        total.add(&own);
    }
    out.push_str("    ");
    total.write(&mut out, Some(files.len()));
    out.push_str("  </project>\n</coverage>\n");
    out
}

/// What one Clover `<metrics>` element counts.
#[derive(Default)]
struct Metrics {
    loc: usize,
    statements: usize,
    covered: usize,
    methods: usize,
    covered_methods: usize,
}

impl Metrics {
    fn add(&mut self, other: &Self) {
        self.loc += other.loc;
        self.statements += other.statements;
        self.covered += other.covered;
        self.methods += other.methods;
        self.covered_methods += other.covered_methods;
    }

    /// Writes the element, with a `files` count for the project's. Classes and
    /// conditionals are `0`, because the report counts neither. A method is an
    /// element as a statement is.
    fn write(&self, out: &mut String, files: Option<usize>) {
        let files = files.map_or(String::new(), |count| format!("files=\"{count}\" "));
        let Self {
            loc,
            statements,
            covered,
            methods,
            covered_methods,
        } = self;
        let elements = statements + methods;
        let covered_elements = covered + covered_methods;
        out.push_str(&format!(
            "<metrics {files}loc=\"{loc}\" ncloc=\"{loc}\" classes=\"0\" methods=\"{methods}\" \
             coveredmethods=\"{covered_methods}\" conditionals=\"0\" coveredconditionals=\"0\" \
             statements=\"{statements}\" coveredstatements=\"{covered}\" elements=\"{elements}\" \
             coveredelements=\"{covered_elements}\"/>\n"
        ));
    }
}

/// The Cobertura XML document: one `<package>` per directory and one `<class>`
/// per file in it, each with a `<line number hits>` per line and its
/// `line-rate`. A file at the top of the run's directory is in the package
/// `.`. `now` is the Unix time, which Cobertura writes in milliseconds.
fn cobertura(files: &[FileLines<'_>], now: u64) -> String {
    let mut packages: BTreeMap<&str, Vec<&FileLines<'_>>> = BTreeMap::new();
    for file in files {
        let directory = file.name.rsplit_once('/').map_or(".", |(directory, _)| {
            if directory.is_empty() { "/" } else { directory }
        });
        packages.entry(directory).or_default().push(file);
    }
    let lines: usize = files.iter().map(|file| file.lines.len()).sum();
    let hit: usize = files.iter().map(FileLines::hit).sum();
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE coverage SYSTEM \"http://cobertura.sourceforge.net/xml/coverage-04.dtd\">\n\
         <coverage line-rate=\"{}\" branch-rate=\"0\" lines-covered=\"{hit}\" lines-valid=\"{lines}\" \
         branches-covered=\"0\" branches-valid=\"0\" complexity=\"0\" version=\"0\" timestamp=\"{}\">\n\
         \x20 <sources>\n    <source>.</source>\n  </sources>\n  <packages>\n",
        rate(hit, lines),
        now.saturating_mul(1000)
    );
    for (directory, members) in packages {
        let lines: usize = members.iter().map(|file| file.lines.len()).sum();
        let hit: usize = members.iter().map(|file| file.hit()).sum();
        out.push_str("    <package name=\"");
        crate::runner::xml_text(directory, &mut out);
        out.push_str(&format!(
            "\" line-rate=\"{}\" branch-rate=\"0\" complexity=\"0\">\n      <classes>\n",
            rate(hit, lines)
        ));
        for file in members {
            out.push_str("        <class name=\"");
            crate::runner::xml_text(file.name, &mut out);
            out.push_str("\" filename=\"");
            crate::runner::xml_text(file.name, &mut out);
            out.push_str(&format!(
                "\" line-rate=\"{}\" branch-rate=\"0\" complexity=\"0\">\n          <methods/>\n          <lines>\n",
                rate(file.hit(), file.lines.len())
            ));
            for (line, count) in &file.lines {
                out.push_str(&format!(
                    "            <line number=\"{line}\" hits=\"{count}\"/>\n"
                ));
            }
            out.push_str("          </lines>\n        </class>\n");
        }
        out.push_str("      </classes>\n    </package>\n");
    }
    out.push_str("  </packages>\n</coverage>\n");
    out
}

/// `hit` out of `of` as a Cobertura rate between `0` and `1`, to four places,
/// and `1` when there is nothing to count.
fn rate(hit: usize, of: usize) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a line count is far below 2^52, and the rate is written to four places"
    )]
    match of {
        0 => 1.0,
        _ => (hit as f64 / of as f64 * 10_000.0).round() / 10_000.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two files out of order, a line holding two statements, a statement
    /// that never ran, and a function in each file, one of which never ran.
    fn sample() -> (Sites, Vec<u64>) {
        let sites = Sites {
            files: vec![("a.nvs".to_owned(), 10), ("b.nvs".to_owned(), 4)],
            at: vec![Some((1, 2)), Some((0, 3)), Some((0, 3)), Some((0, 5)), None],
            functions: vec![
                ("A::late".to_owned(), 3),
                ("B::only".to_owned(), 0),
                ("A::early".to_owned(), 1),
                ("Gone::f".to_owned(), 4),
            ],
        };
        (sites, vec![1, 1, 2, 0, 7])
    }

    #[test]
    fn a_line_counts_its_busiest_statement_and_a_line_that_never_ran_reads_zero() {
        let (sites, counts) = sample();
        let files = sites.lines(&counts);
        assert_eq!(
            lcov(&files),
            "TN:\nSF:a.nvs\nFN:3,A::early\nFN:5,A::late\nFNDA:1,A::early\nFNDA:0,A::late\nFNF:2\nFNH:1\n\
             DA:3,2\nDA:5,0\nLF:2\nLH:1\nend_of_record\n\
             SF:b.nvs\nFN:2,B::only\nFNDA:1,B::only\nFNF:1\nFNH:1\nDA:2,1\nLF:1\nLH:1\nend_of_record\n"
        );
    }

    #[test]
    fn the_cobertura_document_groups_files_by_directory_with_their_rates() {
        let sites = Sites {
            files: vec![
                ("src/Cart.nvs".to_owned(), 9),
                ("src/Price.nvs".to_owned(), 9),
                ("top.nvs".to_owned(), 3),
            ],
            at: vec![Some((0, 2)), Some((0, 4)), Some((1, 1)), Some((2, 1))],
            functions: Vec::new(),
        };
        let document = cobertura(&sites.lines(&[3, 0, 0, 1]), 1_700_000_000);
        assert!(document.contains(
            "<coverage line-rate=\"0.5\" branch-rate=\"0\" lines-covered=\"2\" lines-valid=\"4\" "
        ));
        assert!(document.contains("timestamp=\"1700000000000\""));
        assert!(
            document.contains(
                "<package name=\".\" line-rate=\"1\" branch-rate=\"0\" complexity=\"0\">"
            )
        );
        assert!(document.contains(
            "<package name=\"src\" line-rate=\"0.3333\" branch-rate=\"0\" complexity=\"0\">"
        ));
        assert!(document.contains(
            "<class name=\"src/Cart.nvs\" filename=\"src/Cart.nvs\" line-rate=\"0.5\" branch-rate=\"0\" complexity=\"0\">\n          <methods/>\n          <lines>\n            <line number=\"2\" hits=\"3\"/>\n            <line number=\"4\" hits=\"0\"/>\n          </lines>\n"
        ));
        // Packages are sorted by name, so `.` comes before `src`.
        assert!(document.find("name=\".\"") < document.find("name=\"src\""));
    }

    #[test]
    fn the_clover_document_carries_a_line_per_statement_line_and_the_totals() {
        let (sites, counts) = sample();
        let files = sites.lines(&counts);
        let document = clover(&files, 1_700_000_000);
        assert!(document.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<coverage generated=\"1700000000\">\n"
        ));
        assert!(document.contains(
            "<file name=\"a.nvs\">\n      <line num=\"3\" type=\"method\" name=\"A::early\" count=\"1\"/>\n      \
             <line num=\"3\" type=\"stmt\" count=\"2\"/>\n      <line num=\"5\" type=\"method\" name=\"A::late\" count=\"0\"/>\n      \
             <line num=\"5\" type=\"stmt\" count=\"0\"/>\n"
        ));
        assert!(document.contains(
            "<metrics files=\"2\" loc=\"14\" ncloc=\"14\" classes=\"0\" methods=\"3\" coveredmethods=\"2\" \
             conditionals=\"0\" coveredconditionals=\"0\" statements=\"3\" coveredstatements=\"2\" \
             elements=\"6\" coveredelements=\"4\"/>"
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
            functions: Vec::new(),
        };
        assert!(clover(&sites.lines(&[1]), 0).contains("<file name=\"a&amp;b.nvs\">"));
    }
}
