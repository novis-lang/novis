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
//! # Branches
//!
//! A branch is one `Terminator::Branch`, and its two edges are its true and
//! its false side. The runner also turns `DebugFlags::BRANCH` on, and the
//! edge probe counts each side into the table's edge half. A branch is
//! written on the line where the earlier of its two edges' spans starts, which
//! is its condition's. A `Terminator::Switch` numbers its edges too but has no
//! probe, so [`Probes::of`] picks only the branches' edges and a switch is in
//! no report.
//!
//! # Per test
//!
//! Under a coverage flag, `--format json` writes `schemaVersion: 3` and each
//! test's record lists the lines that test reached. The suite runs one test at
//! a time, so the runner reads the table before and after each test, and the
//! statements whose count grew ([`grew`]) are that test's: every attempt a
//! retry made and every request a `server: true` test sent. [`Sites::reached`]
//! turns them into lines. A fixture is built before its class's first test,
//! so its lines are in no test's list. A runner that runs tests in parallel
//! cannot read one shared table this way: it gives each test a table of its
//! own and adds them into the run's.
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
//! A run that asks holds one `u64` per statement and per edge in the program,
//! plus one file and line pair per statement and per branch and one name per
//! function for the report. Per test it reads the statement table twice and
//! keeps the numbers of the statements that test reached until the run ends.

use std::collections::{BTreeMap, BTreeSet};
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
    /// `nvs_ir::Program::edge_spans`: where each edge number is written.
    edges: Vec<nvs_diagnostics::Span>,
    /// Each `Terminator::Branch`: the program-wide numbers of its true and its
    /// false edge. A `Terminator::Switch` has edge numbers too, but no probe,
    /// so it is not here.
    branches: Vec<(usize, usize)>,
}

impl Probes {
    pub(crate) fn of(program: &nvs_ir::Program) -> Self {
        let mut functions = Vec::new();
        let mut branches = Vec::new();
        let mut base = 0;
        let mut edge_base = 0;
        for function in &program.functions {
            if !function.stmt_spans.is_empty() {
                functions.push((function.name.clone(), base));
            }
            for block in &function.blocks {
                if let nvs_ir::ir::Terminator::Branch {
                    then_edge,
                    else_edge,
                    ..
                } = &block.term
                {
                    branches.push((
                        edge_base + then_edge.index() as usize,
                        edge_base + else_edge.index() as usize,
                    ));
                }
            }
            base += function.stmt_spans.len();
            edge_base += function.edge_spans.len();
        }
        Self {
            stmts: program.stmt_spans(),
            functions,
            edges: program.edge_spans(),
            branches,
        }
    }
}

/// The statement numbers whose count is larger in `after` than in `before`:
/// the statements that ran between two reads of the run's `StmtHits`.
pub(crate) fn grew(before: &[u64], after: &[u64]) -> Vec<usize> {
    before
        .iter()
        .zip(after)
        .enumerate()
        .filter(|(_, (was, is))| is > was)
        .map(|(stmt, _)| stmt)
        .collect()
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
    /// How many edge numbers there are.
    edges: usize,
    /// One entry per [`Probes::branches`] in a file the report keeps: the
    /// index into `files`, the line, and the true and false edge numbers.
    branches: Vec<(usize, usize, usize, usize)>,
}

impl Sites {
    /// Resolves `probes` against the source map the program was read into.
    pub(crate) fn of(probes: Probes, map: &nvs_diagnostics::SourceMap) -> Self {
        let Probes {
            stmts: spans,
            functions,
            edges,
            branches,
        } = probes;
        let here = std::env::current_dir().ok().map(|dir| plain(&dir));
        // Each file once: its name in the report, or `None` for a file that is
        // not on disk.
        let mut named: BTreeMap<nvs_diagnostics::SourceId, Option<String>> = BTreeMap::new();
        let mut sizes: BTreeMap<String, usize> = BTreeMap::new();
        for span in spans.iter().chain(&edges) {
            named.entry(span.file).or_insert_with(|| {
                let file = map.get(span.file)?;
                let path = file.path().filter(|path| path.is_file())?;
                let name = shown(path, here.as_deref());
                sizes.insert(name.clone(), file.line_count());
                Some(name)
            });
        }
        let files: Vec<(String, usize)> = sizes.into_iter().collect();
        let locate = |span: &nvs_diagnostics::Span| {
            let name = named.get(&span.file)?.as_ref()?;
            let index = files.binary_search_by(|(known, _)| known.cmp(name)).ok()?;
            let line = map.get(span.file)?.line_col(span.start).0 + 1;
            Some((index, line))
        };
        let at = spans.iter().map(locate).collect();
        // A branch is written where its earlier edge's span starts. One edge
        // is the condition's and the other the body's or the rest's, and the
        // condition always comes first in the source.
        let branches = branches
            .iter()
            .filter_map(|&(then, otherwise)| {
                let span = [edges.get(then)?, edges.get(otherwise)?]
                    .into_iter()
                    .min_by_key(|span| span.start)?;
                let (file, line) = locate(span)?;
                Some((file, line, then, otherwise))
            })
            .collect();
        Self {
            files,
            at,
            functions,
            edges: edges.len(),
            branches,
        }
    }

    /// How many statement numbers there are, which is the size of the run's
    /// `StmtHits` statement table.
    pub(crate) fn len(&self) -> usize {
        self.at.len()
    }

    /// How many edge numbers there are, which is the size of the run's
    /// `StmtHits` edge table.
    pub(crate) fn edges(&self) -> usize {
        self.edges
    }

    /// The lines the statements numbered in `reached` start on, for one
    /// test's record in the JSON report. Files are in name order, and each
    /// file's lines are sorted and listed once. A statement in a file the
    /// report leaves out is not listed.
    pub(crate) fn reached(&self, reached: &[usize]) -> Vec<(&str, Vec<usize>)> {
        let mut per_file: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for stmt in reached {
            if let Some(&Some((file, line))) = self.at.get(*stmt) {
                per_file.entry(file).or_default().insert(line);
            }
        }
        per_file
            .into_iter()
            .map(|(file, lines)| (self.files[file].0.as_str(), lines.into_iter().collect()))
            .collect()
    }

    /// Each file that has a statement, with its lines, its functions, its
    /// branches and their counts. `edges` is the run's `StmtHits::edge_counts`.
    fn lines(&self, counts: &[u64], edges: &[u64]) -> Vec<FileLines<'_>> {
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
        let mut branches: Vec<Vec<Branch>> = vec![Vec::new(); self.files.len()];
        for &(file, line, then, otherwise) in &self.branches {
            let count = |edge: usize| edges.get(edge).copied().unwrap_or(0);
            branches[file].push(Branch {
                line,
                taken: [count(then), count(otherwise)],
            });
        }
        for list in &mut branches {
            // A stable sort keeps the program's order among branches on one line.
            list.sort_by_key(|branch| branch.line);
        }
        self.files
            .iter()
            .zip(per_file)
            .zip(functions)
            .zip(branches)
            .filter(|(((_, lines), _), _)| !lines.is_empty())
            .map(
                |((((name, line_count), lines), functions), branches)| FileLines {
                    name,
                    line_count: *line_count,
                    lines,
                    functions,
                    branches,
                },
            )
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
    /// The branches in this file, by line.
    branches: Vec<Branch>,
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

    /// How many sides the file's branches have: two each.
    fn sides(&self) -> usize {
        self.branches.len() * 2
    }

    /// How many of those sides ran at least once.
    fn sides_hit(&self) -> usize {
        self.branches.iter().map(Branch::sides_hit).sum()
    }
}

/// One `Terminator::Branch` in a report: the line it is written on, and how
/// often its true side and its false side ran.
#[derive(Clone, Copy)]
struct Branch {
    line: usize,
    taken: [u64; 2],
}

impl Branch {
    fn sides_hit(&self) -> usize {
        self.taken.iter().filter(|&&count| count > 0).count()
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

/// Writes every file `requested` names from `counts` and `edges`, the run's
/// `StmtHits::counts` and `StmtHits::edge_counts`, and returns what the
/// report covers.
///
/// # Errors
///
/// The message to print when a file could not be written.
pub(crate) fn write(
    requested: &Requested,
    sites: &Sites,
    counts: &[u64],
    edges: &[u64],
) -> Result<Summary, String> {
    let files = sites.lines(counts, edges);
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
/// line per function and the `FNF`/`FNH` totals, a `BRDA` line per side of
/// each branch and the `BRF`/`BRH` totals, then a `DA` line per line a
/// statement starts on and the `LF`/`LH` totals.
///
/// A `BRDA` line is `<line>,<block>,<side>,<count>`. The block numbers the
/// branches on one line from `0`, and the side is `0` for true and `1` for
/// false. The count is `-` for both sides of a branch that never ran.
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
        let mut block = 0;
        let mut previous = None;
        for branch in &file.branches {
            block = if previous == Some(branch.line) {
                block + 1
            } else {
                0
            };
            previous = Some(branch.line);
            let ran = branch.taken.iter().any(|&count| count > 0);
            for (side, count) in branch.taken.iter().enumerate() {
                let count = if ran {
                    count.to_string()
                } else {
                    "-".to_owned()
                };
                out.push_str(&format!("BRDA:{},{block},{side},{count}\n", branch.line));
            }
        }
        out.push_str(&format!("BRF:{}\nBRH:{}\n", file.sides(), file.sides_hit()));
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
/// per function, a `<line type="stmt">` per line and a `<line type="cond">`
/// per branch, in line order, and its `<metrics>`, then the project's
/// `<metrics>`. On one line a method comes first and a branch last. `now` is
/// the Unix time the `generated` and `timestamp` attributes carry.
fn clover(files: &[FileLines<'_>], now: u64) -> String {
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<coverage generated=\"{now}\">\n  <project timestamp=\"{now}\">\n"
    );
    let mut total = Metrics::default();
    for file in files {
        out.push_str("    <file name=\"");
        crate::runner::xml_text(file.name, &mut out);
        out.push_str("\">\n");
        let mut rows: Vec<(usize, u8, String)> = Vec::new();
        for function in &file.functions {
            let mut row = format!(
                "      <line num=\"{}\" type=\"method\" name=\"",
                function.line
            );
            crate::runner::xml_text(function.name, &mut row);
            row.push_str(&format!("\" count=\"{}\"/>\n", function.count));
            rows.push((function.line, 0, row));
        }
        for (&line, count) in &file.lines {
            rows.push((
                line,
                1,
                format!("      <line num=\"{line}\" type=\"stmt\" count=\"{count}\"/>\n"),
            ));
        }
        for branch in &file.branches {
            let [truecount, falsecount] = branch.taken;
            rows.push((
                branch.line,
                2,
                format!(
                    "      <line num=\"{}\" type=\"cond\" truecount=\"{truecount}\" falsecount=\"{falsecount}\"/>\n",
                    branch.line
                ),
            ));
        }
        // A stable sort keeps each kind's own order on one line.
        rows.sort_by_key(|&(line, kind, _)| (line, kind));
        for (_, _, row) in rows {
            out.push_str(&row);
        }
        let own = Metrics {
            loc: file.line_count,
            statements: file.lines.len(),
            covered: file.hit(),
            methods: file.functions.len(),
            covered_methods: file.functions_hit(),
            conditionals: file.sides(),
            covered_conditionals: file.sides_hit(),
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
    /// Branch sides, two per branch.
    conditionals: usize,
    covered_conditionals: usize,
}

impl Metrics {
    fn add(&mut self, other: &Self) {
        self.loc += other.loc;
        self.statements += other.statements;
        self.covered += other.covered;
        self.methods += other.methods;
        self.covered_methods += other.covered_methods;
        self.conditionals += other.conditionals;
        self.covered_conditionals += other.covered_conditionals;
    }

    /// Writes the element, with a `files` count for the project's. Classes are
    /// `0`, because the report does not count them. A method and a branch side
    /// are each an element, as a statement is.
    fn write(&self, out: &mut String, files: Option<usize>) {
        let files = files.map_or(String::new(), |count| format!("files=\"{count}\" "));
        let Self {
            loc,
            statements,
            covered,
            methods,
            covered_methods,
            conditionals,
            covered_conditionals,
        } = self;
        let elements = statements + methods + conditionals;
        let covered_elements = covered + covered_methods + covered_conditionals;
        out.push_str(&format!(
            "<metrics {files}loc=\"{loc}\" ncloc=\"{loc}\" classes=\"0\" methods=\"{methods}\" \
             coveredmethods=\"{covered_methods}\" conditionals=\"{conditionals}\" \
             coveredconditionals=\"{covered_conditionals}\" \
             statements=\"{statements}\" coveredstatements=\"{covered}\" elements=\"{elements}\" \
             coveredelements=\"{covered_elements}\"/>\n"
        ));
    }
}

/// The Cobertura XML document: one `<package>` per directory and one `<class>`
/// per file in it, each with a `<line number hits>` per line and its
/// `line-rate` and `branch-rate`. A line with a branch on it also has
/// `branch="true"` and a `condition-coverage` of the sides that ran. A branch
/// on a line no statement starts on is listed with the number of times its
/// condition ran as `hits`, and is not counted in `line-rate`. A file at the
/// top of the run's directory is in the package `.`. `now` is the Unix time,
/// which Cobertura writes in milliseconds.
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
    let sides: usize = files.iter().map(FileLines::sides).sum();
    let sides_hit: usize = files.iter().map(FileLines::sides_hit).sum();
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE coverage SYSTEM \"http://cobertura.sourceforge.net/xml/coverage-04.dtd\">\n\
         <coverage line-rate=\"{}\" branch-rate=\"{}\" lines-covered=\"{hit}\" lines-valid=\"{lines}\" \
         branches-covered=\"{sides_hit}\" branches-valid=\"{sides}\" complexity=\"0\" version=\"0\" timestamp=\"{}\">\n\
         \x20 <sources>\n    <source>.</source>\n  </sources>\n  <packages>\n",
        rate(hit, lines),
        rate(sides_hit, sides),
        now.saturating_mul(1000)
    );
    for (directory, members) in packages {
        let lines: usize = members.iter().map(|file| file.lines.len()).sum();
        let hit: usize = members.iter().map(|file| file.hit()).sum();
        let sides: usize = members.iter().map(|file| file.sides()).sum();
        let sides_hit: usize = members.iter().map(|file| file.sides_hit()).sum();
        out.push_str("    <package name=\"");
        crate::runner::xml_text(directory, &mut out);
        out.push_str(&format!(
            "\" line-rate=\"{}\" branch-rate=\"{}\" complexity=\"0\">\n      <classes>\n",
            rate(hit, lines),
            rate(sides_hit, sides)
        ));
        for file in members {
            out.push_str("        <class name=\"");
            crate::runner::xml_text(file.name, &mut out);
            out.push_str("\" filename=\"");
            crate::runner::xml_text(file.name, &mut out);
            out.push_str(&format!(
                "\" line-rate=\"{}\" branch-rate=\"{}\" complexity=\"0\">\n          <methods/>\n          <lines>\n",
                rate(file.hit(), file.lines.len()),
                rate(file.sides_hit(), file.sides())
            ));
            // Each line: its count, or `None` for a line only a branch is on,
            // and its branch sides and how many of them ran.
            let mut rows: BTreeMap<usize, (Option<u64>, usize, usize)> = file
                .lines
                .iter()
                .map(|(&line, &count)| (line, (Some(count), 0, 0)))
                .collect();
            let mut evaluated: BTreeMap<usize, u64> = BTreeMap::new();
            for branch in &file.branches {
                let row = rows.entry(branch.line).or_insert((None, 0, 0));
                row.1 += 2;
                row.2 += branch.sides_hit();
                *evaluated.entry(branch.line).or_insert(0) += branch.taken[0] + branch.taken[1];
            }
            for (line, (count, sides, sides_hit)) in rows {
                let count = count.unwrap_or_else(|| evaluated.get(&line).copied().unwrap_or(0));
                out.push_str(&format!(
                    "            <line number=\"{line}\" hits=\"{count}\""
                ));
                if let Some(percent) = (sides_hit * 100).checked_div(sides) {
                    out.push_str(&format!(
                        " branch=\"true\" condition-coverage=\"{percent}% ({sides_hit}/{sides})\""
                    ));
                }
                out.push_str("/>\n");
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
        reason = "a line or side count is far below 2^52, and the rate is written to four places"
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
            edges: 6,
            // Two branches on line 3 and one on line 5 that never ran, out of
            // line order.
            branches: vec![(0, 5, 4, 5), (0, 3, 0, 1), (0, 3, 2, 3)],
        };
        (sites, vec![1, 1, 2, 0, 7])
    }

    const SAMPLE_EDGES: [u64; 6] = [2, 0, 1, 1, 0, 0];

    #[test]
    fn a_line_counts_its_busiest_statement_and_a_line_that_never_ran_reads_zero() {
        let (sites, counts) = sample();
        let files = sites.lines(&counts, &SAMPLE_EDGES);
        assert_eq!(
            lcov(&files),
            "TN:\nSF:a.nvs\nFN:3,A::early\nFN:5,A::late\nFNDA:1,A::early\nFNDA:0,A::late\nFNF:2\nFNH:1\n\
             BRDA:3,0,0,2\nBRDA:3,0,1,0\nBRDA:3,1,0,1\nBRDA:3,1,1,1\nBRDA:5,0,0,-\nBRDA:5,0,1,-\nBRF:6\nBRH:3\n\
             DA:3,2\nDA:5,0\nLF:2\nLH:1\nend_of_record\n\
             SF:b.nvs\nFN:2,B::only\nFNDA:1,B::only\nFNF:1\nFNH:1\nBRF:0\nBRH:0\nDA:2,1\nLF:1\nLH:1\nend_of_record\n"
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
            edges: 4,
            // One branch on a statement's line, and one on line 3, which no
            // statement starts on.
            branches: vec![(0, 2, 0, 1), (0, 3, 2, 3)],
        };
        let document = cobertura(&sites.lines(&[3, 0, 0, 1], &[3, 0, 0, 0]), 1_700_000_000);
        assert!(document.contains(
            "<coverage line-rate=\"0.5\" branch-rate=\"0.25\" lines-covered=\"2\" lines-valid=\"4\" \
             branches-covered=\"1\" branches-valid=\"4\" "
        ));
        assert!(document.contains("timestamp=\"1700000000000\""));
        assert!(
            document.contains(
                "<package name=\".\" line-rate=\"1\" branch-rate=\"1\" complexity=\"0\">"
            )
        );
        assert!(document.contains(
            "<package name=\"src\" line-rate=\"0.3333\" branch-rate=\"0.25\" complexity=\"0\">"
        ));
        assert!(document.contains(
            "<class name=\"src/Cart.nvs\" filename=\"src/Cart.nvs\" line-rate=\"0.5\" branch-rate=\"0.25\" complexity=\"0\">\n          <methods/>\n          <lines>\n            \
             <line number=\"2\" hits=\"3\" branch=\"true\" condition-coverage=\"50% (1/2)\"/>\n            \
             <line number=\"3\" hits=\"0\" branch=\"true\" condition-coverage=\"0% (0/2)\"/>\n            \
             <line number=\"4\" hits=\"0\"/>\n          </lines>\n"
        ));
        // Packages are sorted by name, so `.` comes before `src`.
        assert!(document.find("name=\".\"") < document.find("name=\"src\""));
    }

    #[test]
    fn the_clover_document_carries_a_line_per_statement_line_and_the_totals() {
        let (sites, counts) = sample();
        let files = sites.lines(&counts, &SAMPLE_EDGES);
        let document = clover(&files, 1_700_000_000);
        assert!(document.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<coverage generated=\"1700000000\">\n"
        ));
        assert!(document.contains(
            "<file name=\"a.nvs\">\n      <line num=\"3\" type=\"method\" name=\"A::early\" count=\"1\"/>\n      \
             <line num=\"3\" type=\"stmt\" count=\"2\"/>\n      \
             <line num=\"3\" type=\"cond\" truecount=\"2\" falsecount=\"0\"/>\n      \
             <line num=\"3\" type=\"cond\" truecount=\"1\" falsecount=\"1\"/>\n      \
             <line num=\"5\" type=\"method\" name=\"A::late\" count=\"0\"/>\n      \
             <line num=\"5\" type=\"stmt\" count=\"0\"/>\n      \
             <line num=\"5\" type=\"cond\" truecount=\"0\" falsecount=\"0\"/>\n"
        ));
        assert!(document.contains(
            "<metrics files=\"2\" loc=\"14\" ncloc=\"14\" classes=\"0\" methods=\"3\" coveredmethods=\"2\" \
             conditionals=\"6\" coveredconditionals=\"3\" statements=\"3\" coveredstatements=\"2\" \
             elements=\"12\" coveredelements=\"7\"/>"
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
            edges: 0,
            branches: Vec::new(),
        };
        assert!(clover(&sites.lines(&[1], &[]), 0).contains("<file name=\"a&amp;b.nvs\">"));
    }
}
