//! `rule:testing/mutation-testing`'s run: each mutant of the program under
//! test runs the tests that reached the statement it changes, and comes out
//! killed, survived, timed out or not reached.
//!
//! # Which tests a mutant runs
//!
//! The coverage run records the statements each test reached
//! ([`Case::reached`]). A mutant runs the tests that reached the innermost
//! statement whose span holds its changed expression. A change in a condition
//! is held by its `if` or loop statement, which runs whenever the condition is
//! read, so no test that reads the change is left out. A change no statement
//! holds, such as a property's default, is not reached.
//!
//! # One mutant at a time
//!
//! Each mutant is lowered and compiled whole, with no artifact cache: the
//! cache key is the unchanged program's. Its tests run through the same suite
//! as the coverage run, with the report collected and never printed,
//! `--fail-fast` on, and a wall-time limit for every test that has none
//! ([`Shape::wall`]). A run that panics or whose scheduler stops is a killed
//! mutant: no test passed under it.
//!
//! # What it spends
//!
//! One compiled unit and one suite's tests in flight at a time, so
//! O(`--jobs`), plus one [`Scored`] per mutant until the report is written
//! and one test index per statement and test pair the coverage run reached.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use nvs_diagnostics::Span;
use nvs_ir::lower::Mutant;

use super::{
    Case, Format, Outcome, PAST_WALL_TIME, Selection, Shape, message_of, percent,
    run_suite_in_a_task,
};

/// How one mutant came out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    /// A test failed, called `exit`, or the run stopped.
    Killed,
    /// Every test the mutant ran passed.
    Survived,
    /// No test failed, and one ran past its wall-time limit.
    TimedOut,
    /// No selected test reached its statement, so nothing ran.
    NotReached,
}

impl Verdict {
    /// The name every report gives the outcome.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Killed => "killed",
            Self::Survived => "survived",
            Self::TimedOut => "timed out",
            Self::NotReached => "not reached",
        }
    }
}

/// One mutant as the report names it.
pub(super) struct Scored {
    /// The file, named as a coverage report names it.
    pub(super) file: String,
    pub(super) line: usize,
    /// [`Mutant::change`].
    pub(super) change: String,
    pub(super) verdict: Verdict,
    /// How many tests reached the mutant's line, and so ran against it. `0`
    /// for a mutant that is `not reached`.
    pub(super) tests: usize,
}

/// What every mutant's run shares.
pub(super) struct Runs<'a> {
    /// A new suite context, configured as the coverage run's was. The
    /// mutant's unit is installed in it here.
    pub(super) ctx: &'a dyn Fn() -> nvs_runtime::Ctx,
    pub(super) config: &'a nvs_config::Config,
    pub(super) extensions: Option<Arc<nvs_ext::load::Set>>,
    pub(super) jobs: usize,
    /// The run's selection. A mutant narrows it to the tests that reach it.
    pub(super) select: &'a Selection,
}

/// Every mutant of `checked`, in the order its sites are lowered.
pub(super) fn list(checked: &crate::Checked) -> Vec<Mutant> {
    let files = checked.program_files();
    nvs_ir::lower::mutants(
        nvs_ir::lower::ENTRY_SCRIPT_LABEL,
        &files,
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    )
}

/// Runs each of `mutants` against the tests of `baseline` that reached it.
/// `stmts` are the coverage run's statement spans, by statement number.
///
/// `Err` is a mutant that did not compile, which is an internal error: every
/// mutant keeps its operands' and its result's types.
pub(super) fn score(
    checked: &Rc<crate::Checked>,
    mutants: &[Mutant],
    baseline: &[Case],
    stmts: &[Span],
    runs: &Runs<'_>,
) -> Result<Vec<Scored>, String> {
    let covering = covering_tests(baseline);
    let wall = limit(baseline);
    let files = checked.program_files();
    let digest = crate::cache::test_program_digest(&files);
    let mut scored = Vec::with_capacity(mutants.len());
    for mutant in mutants {
        let tests: BTreeSet<String> = innermost(stmts, mutant.span)
            .and_then(|stmt| covering.get(&stmt))
            .map(|cases| {
                cases
                    .iter()
                    .map(|&at| format!("{}::{}", baseline[at].class, baseline[at].method))
                    .collect()
            })
            .unwrap_or_default();
        let file = crate::coverage::file_name(&checked.map, mutant.file);
        let reached = tests.len();
        let verdict = if tests.is_empty() {
            Verdict::NotReached
        } else {
            let program = nvs_ir::lower::lower_mutant(
                nvs_ir::lower::ENTRY_SCRIPT_LABEL,
                &files,
                &checked.exprs,
                &checked.interner,
                &checked.enums,
                &checked.layouts,
                mutant,
            );
            let unit = crate::cache::unit_for(&program, digest, None)
                .map(|(unit, _)| Arc::new(unit))
                .map_err(|error| {
                    format!(
                        "internal error: the mutant at {file}:{} ({}) did not compile: {error}",
                        mutant.line,
                        mutant.change()
                    )
                })?;
            drop(program);
            run_one(&unit, checked, tests, wall, runs)
        };
        scored.push(Scored {
            file,
            line: mutant.line,
            change: mutant.change(),
            verdict,
            tests: reached,
        });
    }
    Ok(scored)
}

/// Runs `tests` over one mutant's `unit`.
fn run_one(
    unit: &Arc<nvs_codegen::Unit>,
    checked: &Rc<crate::Checked>,
    tests: BTreeSet<String>,
    wall: Duration,
    runs: &Runs<'_>,
) -> Verdict {
    let mut ctx = (runs.ctx)();
    unit.install_in(&mut ctx);
    let select = Selection {
        only: Some(tests),
        ..runs.select.clone()
    };
    let shape = Shape {
        format: Format::Json,
        jobs: runs.jobs,
        show_output: false,
        fail_fast: true,
        shuffle: None,
        wall: Some(wall),
    };
    match run_suite_in_a_task(
        unit,
        ctx,
        Rc::clone(checked),
        runs.config,
        runs.extensions.clone(),
        shape,
        &select,
        None,
    ) {
        Ok((suite, _)) => verdict_of(
            &suite.cases,
            suite.exited.is_some() || !suite.class_failures.is_empty(),
        ),
        Err(_) => Verdict::Killed,
    }
}

/// The verdict of one mutant's run. `stopped` is an `exit` or an
/// `#[AfterClass]` that threw.
fn verdict_of(cases: &[Case], stopped: bool) -> Verdict {
    let mut timed_out = false;
    for case in cases {
        match &case.outcome {
            Outcome::Failed(failures)
                if failures
                    .iter()
                    .any(|record| message_of(record).starts_with(PAST_WALL_TIME)) =>
            {
                timed_out = true;
            }
            Outcome::Failed(_) | Outcome::Exited(_) => return Verdict::Killed,
            Outcome::Passed | Outcome::Skipped(_) | Outcome::Flaky { .. } => {}
        }
    }
    match (stopped, timed_out) {
        (true, _) => Verdict::Killed,
        (false, true) => Verdict::TimedOut,
        (false, false) => Verdict::Survived,
    }
}

/// Each statement number the coverage run reached, with the index into
/// `baseline` of every test that reached it.
fn covering_tests(baseline: &[Case]) -> BTreeMap<usize, Vec<usize>> {
    let mut covering: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (at, case) in baseline.iter().enumerate() {
        for &stmt in case.reached.as_deref().unwrap_or_default() {
            covering.entry(stmt).or_default().push(at);
        }
    }
    covering
}

/// The number of the smallest statement whose span holds `changed`.
fn innermost(stmts: &[Span], changed: Span) -> Option<usize> {
    stmts
        .iter()
        .enumerate()
        .filter(|(_, stmt)| {
            stmt.file == changed.file && stmt.start <= changed.start && changed.end <= stmt.end
        })
        .min_by_key(|(_, stmt)| stmt.len())
        .map(|(at, _)| at)
}

/// The wall-time limit of a mutant's test that has none: ten times the
/// slowest test of the coverage run, plus one second.
fn limit(baseline: &[Case]) -> Duration {
    let slowest = baseline
        .iter()
        .map(|case| case.elapsed)
        .max()
        .unwrap_or_default();
    slowest.saturating_mul(10) + Duration::from_secs(1)
}

/// The human report's lines: the four counts, the score, then each survived
/// mutant.
pub(super) fn report(scored: &[Scored]) -> String {
    let count = |verdict: Verdict| scored.iter().filter(|s| s.verdict == verdict).count();
    let (killed, survived, timed_out, unreached) = (
        count(Verdict::Killed),
        count(Verdict::Survived),
        count(Verdict::TimedOut),
        count(Verdict::NotReached),
    );
    let mut out = format!(
        "\n  mutants: {killed} killed, {survived} survived, {timed_out} timed out, {unreached} not reached\n"
    );
    let reached = scored.len() - unreached;
    if reached > 0 {
        out.push_str(&format!(
            "  mutation score: {:.1}% ({} of {reached} reached mutants made a test fail or time out)\n",
            percent(killed + timed_out, reached),
            killed + timed_out
        ));
    }
    for survivor in scored.iter().filter(|s| s.verdict == Verdict::Survived) {
        out.push_str(&format!(
            "  survived  {}:{}  {}\n",
            survivor.file, survivor.line, survivor.change
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::{Runs, Scored, Verdict, list, score};
    use crate::runner::{Format, Selection, Shape, compile, json_document, run_suite_in_a_task};

    /// The fixture's coverage run and every mutant's verdict.
    fn scored() -> (Vec<Scored>, String) {
        let path = nvs_repo::path("crates/nvs-cli/tests/fixtures/runner/mutate.nvs");
        let source = std::fs::read_to_string(&path).expect("the fixture is on disk");
        let checked = Rc::new(crate::front_end(&path).expect("the fixture is a program"));
        let (unit, probes, ..) =
            compile(&checked, None, true, false).expect("the fixture compiles");
        let stmts = probes.stmt_spans().to_vec();
        let sites = crate::coverage::Sites::of(probes, &checked.map);
        let mut ctx = crate::script::granting_ctx();
        unit.install_in(&mut ctx);
        ctx.set_stmt_hits(Some(std::sync::Arc::new(
            nvs_runtime::StmtHits::with_edges(sites.len(), sites.edges()),
        )));
        ctx.set_debug_flags(
            ctx.debug_flags() | nvs_runtime::DebugFlags::COVERAGE | nvs_runtime::DebugFlags::BRANCH,
        );
        let shape = Shape {
            format: Format::Json,
            jobs: crate::runner::default_jobs(),
            show_output: false,
            fail_fast: false,
            shuffle: None,
            wall: None,
        };
        let config = nvs_config::Config::default();
        let select = Selection::default();
        let (baseline, _) = run_suite_in_a_task(
            &unit,
            ctx,
            Rc::clone(&checked),
            &config,
            None,
            shape,
            &select,
            None,
        )
        .expect("the suite's own task runs");
        assert!(
            baseline
                .cases
                .iter()
                .all(|case| case.outcome.verdict() == "passed"),
            "every test passes before any change is made"
        );
        let listed = list(&checked);
        // Every mutant compiles, the unreached ones included, which `score` never compiles.
        let files = checked.program_files();
        for mutant in &listed {
            let program = nvs_ir::lower::lower_mutant(
                nvs_ir::lower::ENTRY_SCRIPT_LABEL,
                &files,
                &checked.exprs,
                &checked.interner,
                &checked.enums,
                &checked.layouts,
                mutant,
            );
            if let Err(error) = nvs_codegen::compile(&program) {
                panic!(
                    "the mutant at line {} ({}) compiles: {error}",
                    mutant.line,
                    mutant.change()
                );
            }
        }
        let runs = Runs {
            ctx: &crate::script::granting_ctx,
            config: &config,
            extensions: None,
            jobs: crate::runner::default_jobs(),
            select: &select,
        };
        let scored = score(&checked, &listed, &baseline.cases, &stmts, &runs)
            .expect("every mutant compiles");
        (scored, source)
    }

    /// `rule:testing/mutation-testing`: one killed, one survived, one timed-out and one unreached
    /// mutant, each found by its line and its change.
    #[test]
    fn a_fixture_reports_each_of_the_four_outcomes() {
        let (scored, source) = scored();
        let line_of = |marker: &str| {
            source
                .lines()
                .position(|line| line.contains(marker))
                .map(|index| index + 1)
                .expect("the marker is in the fixture")
        };
        let verdict = |marker: &str, change: &str| {
            let line = line_of(marker);
            scored
                .iter()
                .find(|mutant| mutant.line == line && mutant.change == change)
                .map(|mutant| mutant.verdict)
        };
        assert_eq!(
            verdict("// survives", "`>=` to `>`"),
            Some(Verdict::Survived)
        );
        assert_eq!(verdict("// killed", "`+` to `-`"), Some(Verdict::Killed));
        assert_eq!(
            verdict("// times out", "`+` to `-`"),
            Some(Verdict::TimedOut)
        );
        assert_eq!(
            verdict("// not reached", "`-` to `+`"),
            Some(Verdict::NotReached)
        );
        assert!(
            scored
                .iter()
                .all(|mutant| mutant.file.ends_with("mutate.nvs")),
            "every mutant is in the program's one file"
        );
        let report = super::report(&scored);
        assert!(
            report.contains("  survived  ") && report.contains("`>=` to `>`"),
            "the human report lists the survived mutant: {report}"
        );
    }

    /// `rule:testing/report-formats`: version 4's `mutants` array reads back as the records written.
    #[test]
    fn the_json_mutants_array_round_trips() {
        let scored = vec![
            Scored {
                file: "src/Shop.nvs".to_owned(),
                line: 12,
                change: "`<` to `<=`".to_owned(),
                verdict: Verdict::Survived,
                tests: 2,
            },
            Scored {
                file: "src/Shop.nvs".to_owned(),
                line: 20,
                change: "`true` to `false`".to_owned(),
                verdict: Verdict::TimedOut,
                tests: 1,
            },
        ];
        let document = json_document(
            &[],
            crate::runner::Counts::default(),
            std::time::Duration::ZERO,
            None,
            Some(&scored),
        );
        let read: serde_json::Value =
            serde_json::from_str(&document).expect("the report is a JSON document");
        assert_eq!(read["schemaVersion"], 4);
        let mutants = read["mutants"].as_array().expect("`mutants` is an array");
        let back: Vec<(String, u64, String, String, u64)> = mutants
            .iter()
            .map(|record| {
                (
                    record["file"].as_str().unwrap_or_default().to_owned(),
                    record["line"].as_u64().unwrap_or_default(),
                    record["change"].as_str().unwrap_or_default().to_owned(),
                    record["outcome"].as_str().unwrap_or_default().to_owned(),
                    record["tests"].as_u64().unwrap_or_default(),
                )
            })
            .collect();
        let written: Vec<(String, u64, String, String, u64)> = scored
            .iter()
            .map(|mutant| {
                (
                    mutant.file.clone(),
                    mutant.line as u64,
                    mutant.change.clone(),
                    mutant.verdict.name().to_owned(),
                    mutant.tests as u64,
                )
            })
            .collect();
        assert_eq!(back, written);
    }
}
