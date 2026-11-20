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
//! # Every mutant has a key
//!
//! A mutant's key is `{function}#{n}: {change}`, such as
//! ``Shop\Cart::total#2: `<` to `<=` ``: its function's label, then which of
//! that function's mutants with the same change it is, counted from 1 in
//! source order. An edit outside the function leaves the key alone, so a
//! report and a person can name one mutant across runs. [`list`] numbers
//! every mutant of the program before [`Aim`] keeps any, so aiming a run never
//! changes a key.
//!
//! # A run is aimed
//!
//! `--mutate=<target>` keeps the mutants in a folder or file, in a class's
//! methods, or in one method ([`Aim`]). A target is judged once the program is
//! checked, before any test runs, and one that names nothing in the program is
//! an error. `#[NoMutate]` code makes no mutant at all: the lowering's survey
//! leaves it out.
//!
//! `--since <ref>` keeps a mutant when any line of its span changed since the
//! ref ([`super::since`]). With a target as well, both must hold.
//!
//! # A mutant that compiles to known code runs no test
//!
//! Before the program is compiled with its mutants, [`Code`] lowers each
//! reached mutant with its one change made, compiles the one function the
//! change is in, and hashes it. A hash equal to the program's own
//! function is `equivalent`, and one equal to an earlier mutant's in that
//! function is a `duplicate`. Neither runs a test or counts in the score.
//!
//! # Judged survivors, and the report grouped by function
//!
//! [`Judgments`] reads `mutants.toml` from the program's folder before any
//! test runs. A survivor whose key has an entry is judged: the report counts
//! it and does not list it, and an `equivalent` one leaves the score. An
//! entry no mutant of the program has is a warning. [`report`] lists the
//! other survivors by function, most first, one line per source line, each
//! ending with its key as a line to paste.
//!
//! # One compile, then one mutant at a time
//!
//! Once every reached mutant is judged, the ones whose tests run are lowered
//! into one program with each change behind the mutant switch
//! ([`nvs_ir::lower::lower_schema`]), and that program is compiled once, with
//! no artifact cache. Before each mutant's tests the run sets its number as
//! the unit's active mutant ([`nvs_codegen::Unit::set_active_mutant`]), so only
//! its own site takes the changed form. Its tests run through the same suite
//! as the coverage run, with the report collected and never printed,
//! `--fail-fast` on, and a wall-time limit for every test that has none
//! ([`Shape::wall`]). A run that panics or whose scheduler stops is a killed
//! mutant: no test passed under it.
//!
//! # What it spends
//!
//! One compiled unit for the run, bigger than the program's by each armed
//! site's second form and its test, and one suite's tests in flight at a
//! time, so O(`--jobs`). Judging lowers the program once more per reached
//! mutant and holds one hash each. Plus one [`Scored`] per mutant until the
//! report is written and one test index per statement and test pair the
//! coverage run reached.

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
    /// Its changed function compiles to the program's own code, so no test
    /// can tell it apart. Nothing ran.
    Equivalent,
    /// Its changed function compiles to an earlier mutant's code
    /// ([`Scored::same_as`]). Nothing ran.
    Duplicate,
}

impl Verdict {
    /// The name every report gives the outcome.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Killed => "killed",
            Self::Survived => "survived",
            Self::TimedOut => "timed out",
            Self::NotReached => "not reached",
            Self::Equivalent => "equivalent",
            Self::Duplicate => "duplicate",
        }
    }
}

/// One mutant as the report names it.
pub(super) struct Scored {
    /// The file, named as a coverage report names it.
    pub(super) file: String,
    pub(super) line: usize,
    /// [`Mutant::function`].
    pub(super) function: String,
    /// The mutant's key ([`Listed::key`]).
    pub(super) key: String,
    /// [`Mutant::change`].
    pub(super) change: String,
    pub(super) verdict: Verdict,
    /// How many tests reached the mutant's line, and so ran against it. `0`
    /// for a mutant that is `not reached`, `equivalent` or `duplicate`.
    pub(super) tests: usize,
    /// For a `duplicate`, the key of the first mutant that compiled alike.
    pub(super) same_as: Option<String>,
    /// For a survivor, its `mutants.toml` entry when it has one.
    pub(super) judged: Option<Judgment>,
}

/// The file of judged survivors, in the program's folder.
pub(super) const JUDGED_FILE: &str = "mutants.toml";

/// One `[[judged]]` entry of [`JUDGED_FILE`]: a person read the survivor and
/// wrote why it may stay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Judgment {
    pub(super) reason: String,
    /// No test can tell the mutant from the program, so the score leaves it
    /// out the way it leaves out an `equivalent` mutant.
    pub(super) equivalent: bool,
}

/// [`JUDGED_FILE`]'s entries, by mutant key. Read once per `--mutate` run.
///
/// What it spends: one key and one reason per entry, for the run.
#[derive(Default)]
pub(super) struct Judgments(BTreeMap<String, Judgment>);

impl Judgments {
    /// Reads [`JUDGED_FILE`] from `folder`. No file is no entry. `Err` says
    /// what is wrong with the file, and the run exits with status 2.
    pub(super) fn read(folder: &std::path::Path) -> Result<Self, String> {
        let path = folder.join(JUDGED_FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(format!("{} cannot be read: {error}", path.display())),
        };
        Self::parse(&text).map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Parses the text of [`JUDGED_FILE`]. Every entry needs a `mutant` key
    /// and a non-empty `reason`, and no other key but `equivalent` is allowed.
    fn parse(text: &str) -> Result<Self, String> {
        let mut table: toml::Table = text.parse().map_err(|error| format!("{error}"))?;
        let entries = match table.remove("judged") {
            None => Vec::new(),
            Some(toml::Value::Array(entries)) => entries,
            Some(_) => return Err("`judged` must be an array of tables: `[[judged]]`".to_owned()),
        };
        if let Some(other) = table.keys().next() {
            return Err(format!(
                "`{other}` is not a setting of this file; only `[[judged]]` is"
            ));
        }
        let mut judgments = BTreeMap::new();
        for (at, entry) in entries.into_iter().enumerate() {
            let nth = at + 1;
            let toml::Value::Table(mut entry) = entry else {
                return Err(format!("entry {nth} of `judged` is not a table"));
            };
            let text = |entry: &mut toml::Table, name: &str| match entry.remove(name) {
                Some(toml::Value::String(value)) if !value.trim().is_empty() => Ok(value),
                Some(toml::Value::String(_)) | None => Err(format!(
                    "entry {nth} of `judged` needs a non-empty `{name}`"
                )),
                Some(_) => Err(format!(
                    "`{name}` in entry {nth} of `judged` must be a string"
                )),
            };
            let mutant = text(&mut entry, "mutant")?;
            let reason = text(&mut entry, "reason")?;
            let equivalent = match entry.remove("equivalent") {
                None => false,
                Some(toml::Value::Boolean(value)) => value,
                Some(_) => {
                    return Err(format!(
                        "`equivalent` in entry {nth} of `judged` must be `true` or `false`"
                    ));
                }
            };
            if let Some(other) = entry.keys().next() {
                return Err(format!(
                    "`{other}` in entry {nth} of `judged` is not allowed; use `mutant`, `reason` and `equivalent`"
                ));
            }
            if judgments
                .insert(mutant.clone(), Judgment { reason, equivalent })
                .is_some()
            {
                return Err(format!("`{mutant}` is judged twice"));
            }
        }
        Ok(Self(judgments))
    }

    /// Marks each survivor in `scored` whose key has an entry. Gives each
    /// entry's key that none of `keys`, every mutant of the program, has.
    pub(super) fn apply(&self, scored: &mut [Scored], keys: &BTreeSet<String>) -> Vec<String> {
        for mutant in scored.iter_mut() {
            if mutant.verdict == Verdict::Survived {
                mutant.judged = self.0.get(&mutant.key).cloned();
            }
        }
        self.0
            .keys()
            .filter(|key| !keys.contains(*key))
            .cloned()
            .collect()
    }
}

/// What every mutant's run shares.
pub(super) struct Runs<'a> {
    /// A new suite context, configured as the coverage run's was. The
    /// mutant's unit is installed in it here.
    pub(super) ctx: &'a dyn Fn() -> nvs_runtime::Ctx,
    pub(super) config: &'a nvs_config::Config,
    /// Which file wrote each key of `config`.
    pub(super) origins: &'a std::collections::BTreeMap<String, nvs_config::Origin>,
    pub(super) extensions: Option<Arc<nvs_ext::load::Set>>,
    pub(super) jobs: usize,
    /// The run's selection. A mutant narrows it to the tests that reach it.
    pub(super) select: &'a Selection,
}

/// One listed mutant and its key.
pub(super) struct Listed {
    pub(super) mutant: Mutant,
    /// `{function}#{n}: {change}`: see the module doc.
    pub(super) key: String,
}

/// Every mutant of `checked` that `aim` keeps, in the order its sites are
/// lowered, each with its key; and the key of every mutant of the program,
/// kept or not.
pub(super) fn list(checked: &crate::Checked, aim: &Aim) -> (Vec<Listed>, BTreeSet<String>) {
    let files = checked.program_files();
    let mutants = nvs_ir::lower::mutants(
        nvs_ir::lower::ENTRY_SCRIPT_LABEL,
        &files,
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    );
    let keys = keys(&mutants);
    let every = keys.iter().cloned().collect();
    let listed = mutants
        .into_iter()
        .zip(keys)
        .filter(|(mutant, _)| aim.keeps(checked, mutant))
        .map(|(mutant, key)| Listed { mutant, key })
        .collect();
    (listed, every)
}

/// Each mutant's key, in `mutants`' order. The count runs over the mutants of
/// one function with one change, ordered by where each is written.
fn keys(mutants: &[Mutant]) -> Vec<String> {
    let mut order: Vec<usize> = (0..mutants.len()).collect();
    order.sort_by_key(|&at| {
        let m = &mutants[at];
        (m.span.file, m.span.start, m.span.end)
    });
    let mut seen: BTreeMap<(&str, String), usize> = BTreeMap::new();
    let mut keys = vec![String::new(); mutants.len()];
    for at in order {
        let mutant = &mutants[at];
        let change = mutant.change();
        let n = seen
            .entry((mutant.function.as_str(), change.clone()))
            .or_default();
        *n += 1;
        keys[at] = format!("{}#{n}: {change}", mutant.function);
    }
    keys
}

/// What `--mutate=<target>` and `--since` keep. With neither, every mutant.
#[derive(Default)]
pub(super) struct Aim {
    /// Folders and files, canonical.
    paths: Vec<std::path::PathBuf>,
    /// Class and method labels, with no leading `\`.
    names: Vec<String>,
    /// `--since`'s changed lines.
    since: Option<super::since::Changed>,
}

impl Aim {
    /// Reads each target as a folder or file relative to the current folder
    /// when one exists there, and otherwise as a class or a `Class::method`
    /// label. `Err` names the first target that matches nothing in `checked`.
    pub(super) fn resolve(checked: &crate::Checked, targets: &[String]) -> Result<Self, String> {
        let mut aim = Self::default();
        let files: Vec<std::path::PathBuf> = checked
            .program_files()
            .iter()
            .filter_map(|file| file.src.path())
            .filter_map(|path| std::fs::canonicalize(path).ok())
            .collect();
        for target in targets {
            let target = target.trim();
            if let Ok(path) = std::fs::canonicalize(target) {
                if !files.iter().any(|file| file.starts_with(&path)) {
                    return Err(format!(
                        "`--mutate={target}` names a path with no file of this program in it"
                    ));
                }
                aim.paths.push(path);
                continue;
            }
            let name = target.trim_start_matches('\\');
            let declared = !name.is_empty()
                && checked.exprs.method_labels().any(|label| {
                    label == name
                        || label
                            .strip_prefix(name)
                            .is_some_and(|rest| rest.starts_with("::"))
                });
            if !declared {
                return Err(format!(
                    "`--mutate={target}` matches no folder, file, class or method of this program"
                ));
            }
            aim.names.push(name.to_owned());
        }
        Ok(aim)
    }

    /// Keeps only the mutants on a line in `changed`, besides any target.
    pub(super) fn since(&mut self, changed: super::since::Changed) {
        self.since = Some(changed);
    }

    /// Whether `mutant` is kept: its targets keep it, and under `--since` a
    /// line of its span changed.
    fn keeps(&self, checked: &crate::Checked, mutant: &Mutant) -> bool {
        self.targets_keep(checked, mutant)
            && self.since.as_ref().is_none_or(|changed| {
                let Some(source) = checked.map.get(mutant.span.file) else {
                    return false;
                };
                let first = source.line_col(mutant.span.start).0 + 1;
                let last = source.line_col(mutant.span.end).0 + 1;
                source
                    .path()
                    .and_then(|path| std::fs::canonicalize(path).ok())
                    .is_some_and(|path| changed.touches(&path, first, last))
            })
    }

    /// Whether the targets keep `mutant`: no target was given, its file is
    /// under a path target, or its function is a named method or in a named
    /// class.
    fn targets_keep(&self, checked: &crate::Checked, mutant: &Mutant) -> bool {
        if self.paths.is_empty() && self.names.is_empty() {
            return true;
        }
        let in_names = self.names.iter().any(|name| {
            mutant.function == *name
                || mutant
                    .function
                    .strip_prefix(name.as_str())
                    .is_some_and(|rest| rest.starts_with("::"))
        });
        in_names
            || (!self.paths.is_empty()
                && checked
                    .map
                    .get(mutant.file)
                    .and_then(|file| file.path())
                    .and_then(|path| std::fs::canonicalize(path).ok())
                    .is_some_and(|file| self.paths.iter().any(|path| file.starts_with(path))))
    }
}

/// What [`score`] found: one record per mutant, and how many times the
/// program was compiled to run them.
pub(super) struct Scoring {
    /// One record per mutant, in the order they were listed.
    pub(super) scored: Vec<Scored>,
    /// 1 when any mutant ran tests, and 0 when none did.
    pub(super) compiles: usize,
    /// How many mutants ran tests.
    pub(super) ran: usize,
}

/// Runs each of `mutants` against the tests of `baseline` that reached it.
/// `stmts` are the coverage run's statement spans, by statement number.
///
/// Every reached mutant is judged first ([`Code`]). Then the ones whose tests
/// run are lowered into one schema ([`nvs_ir::lower::lower_schema`]) and
/// compiled once, and each runs its tests with its number set as the unit's
/// active mutant.
///
/// `Err` is a program that did not compile, which is an internal error: every
/// mutant keeps its operands' and its result's types.
pub(super) fn score(
    checked: &Rc<crate::Checked>,
    mutants: &[Listed],
    baseline: &[Case],
    stmts: &[Span],
    runs: &Runs<'_>,
) -> Result<Scoring, String> {
    let covering = covering_tests(baseline);
    let wall = limit(baseline);
    let files = checked.program_files();
    let digest = crate::cache::test_program_digest(&files);
    let mut scored = Vec::with_capacity(mutants.len());
    let mut code = Code::default();
    // The mutants whose tests run: each one's index in `scored`, and its tests.
    let mut pending: Vec<(usize, &Mutant, BTreeSet<String>)> = Vec::new();
    for Listed { mutant, key } in mutants {
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
        let mut reached = tests.len();
        let mut same_as = None;
        let verdict = if tests.is_empty() {
            Verdict::NotReached
        } else {
            let internal = |error: &dyn std::fmt::Display| {
                format!(
                    "internal error: the mutant at {file}:{} ({}) did not compile: {error}",
                    mutant.line,
                    mutant.change()
                )
            };
            let mutated = nvs_ir::lower::lower_mutant(
                nvs_ir::lower::ENTRY_SCRIPT_LABEL,
                &files,
                &checked.exprs,
                &checked.interner,
                &checked.enums,
                &checked.layouts,
                mutant,
            );
            match code
                .judge(checked, &mutated, key)
                .map_err(|error| internal(&error))?
            {
                Judged::Equivalent => {
                    reached = 0;
                    Verdict::Equivalent
                }
                Judged::Duplicate(first) => {
                    reached = 0;
                    same_as = Some(first);
                    Verdict::Duplicate
                }
                Judged::New => {
                    pending.push((scored.len(), mutant, tests));
                    // Replaced once the mutant's tests have run.
                    Verdict::NotReached
                }
            }
        };
        scored.push(Scored {
            file,
            line: mutant.line,
            function: mutant.function.clone(),
            key: key.clone(),
            change: mutant.change(),
            verdict,
            tests: reached,
            same_as,
            judged: None,
        });
    }
    let ran = pending.len();
    if pending.is_empty() {
        return Ok(Scoring {
            scored,
            compiles: 0,
            ran,
        });
    }
    let armed: Vec<&Mutant> = pending.iter().map(|(_, mutant, _)| *mutant).collect();
    let schema = nvs_ir::lower::lower_schema(
        nvs_ir::lower::ENTRY_SCRIPT_LABEL,
        &files,
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
        &armed,
    );
    let unit = crate::cache::unit_for(&schema, digest, None)
        .map(|(unit, _)| Arc::new(unit))
        .map_err(|error| {
            format!("internal error: the program with its {ran} mutants did not compile: {error}")
        })?;
    drop(schema);
    for (number, (at, _, tests)) in (1..).zip(pending) {
        unit.set_active_mutant(number);
        scored[at].verdict = run_one(&unit, checked, tests, wall, runs);
    }
    unit.set_active_mutant(0);
    Ok(Scoring {
        scored,
        compiles: 1,
        ran,
    })
}

/// What [`Code::judge`] found for one mutant.
enum Judged {
    /// Its changed function compiles to the program's own code.
    Equivalent,
    /// Its changed function compiles to an earlier mutant's code, whose key
    /// this is.
    Duplicate(String),
    /// Neither: its tests run.
    New,
}

/// The code hashes a run has seen: Trivial Compiler Equivalence.
///
/// A mutant's changed function is compiled alone
/// ([`nvs_codegen::function_code_hash`]) and its hash compared with the
/// program's own function and with every earlier mutant's in that function.
///
/// What it spends: the program lowered once more without a change, held for
/// the run, and one hash per original function and per reached mutant.
#[derive(Default)]
struct Code {
    /// The program with no change, lowered on the first reached mutant.
    original: Option<nvs_ir::Program>,
    /// Each function's original hash, by its name. `None` when the
    /// original program has no function by that name.
    originals: BTreeMap<String, Option<[u8; 32]>>,
    /// Each changed function's mutant hashes, with the key of the first
    /// mutant that compiled to each.
    seen: BTreeMap<String, BTreeMap<[u8; 32], String>>,
}

impl Code {
    fn judge(
        &mut self,
        checked: &crate::Checked,
        mutated: &nvs_ir::lower::Mutated,
        key: &str,
    ) -> Result<Judged, nvs_codegen::CodegenError> {
        let Some(hash) = nvs_codegen::function_code_hash(&mutated.program, &mutated.changed)?
        else {
            return Ok(Judged::New);
        };
        let original = self.original.get_or_insert_with(|| {
            let files = checked.program_files();
            nvs_ir::lower::lower_program_with_tests(
                nvs_ir::lower::ENTRY_SCRIPT_LABEL,
                &files,
                &checked.exprs,
                &checked.interner,
                &checked.enums,
                &checked.layouts,
            )
        });
        let before = match self.originals.get(&mutated.changed) {
            Some(before) => *before,
            None => {
                let before = nvs_codegen::function_code_hash(original, &mutated.changed)?;
                self.originals.insert(mutated.changed.clone(), before);
                before
            }
        };
        if before == Some(hash) {
            return Ok(Judged::Equivalent);
        }
        let seen = self.seen.entry(mutated.changed.clone()).or_default();
        if let Some(first) = seen.get(&hash) {
            return Ok(Judged::Duplicate(first.clone()));
        }
        seen.insert(hash, key.to_owned());
        Ok(Judged::New)
    }
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
        runs.origins,
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

/// The human report's lines: the four counts, the skipped and judged counts,
/// the score, then the survivors no `mutants.toml` entry judged, grouped by
/// function ([`survivors`]).
pub(super) fn report(scored: &[Scored]) -> String {
    let count = |verdict: Verdict| scored.iter().filter(|s| s.verdict == verdict).count();
    let (killed, survived, timed_out, unreached) = (
        count(Verdict::Killed),
        count(Verdict::Survived),
        count(Verdict::TimedOut),
        count(Verdict::NotReached),
    );
    let (equivalent, duplicate) = (count(Verdict::Equivalent), count(Verdict::Duplicate));
    let mut out = format!(
        "\n  mutants: {killed} killed, {survived} survived, {timed_out} timed out, {unreached} not reached\n"
    );
    if equivalent + duplicate > 0 {
        out.push_str(&format!(
            "  skipped: {equivalent} equivalent, {duplicate} duplicate (compiled to the same code, so no test ran)\n"
        ));
    }
    let judged = scored.iter().filter(|s| s.judged.is_some()).count();
    let judged_equivalent = scored
        .iter()
        .filter(|s| s.judged.as_ref().is_some_and(|j| j.equivalent))
        .count();
    if judged > 0 {
        out.push_str(&format!(
            "  judged: {judged} survived (in {JUDGED_FILE}, so they are not listed below)\n"
        ));
    }
    let reached = scored.len() - unreached - equivalent - duplicate - judged_equivalent;
    if reached > 0 {
        out.push_str(&format!(
            "  mutation score: {:.1}% ({} of {reached} reached mutants made a test fail or time out)\n",
            percent(killed + timed_out, reached),
            killed + timed_out
        ));
    }
    out.push_str(&survivors(scored));
    out
}

/// The survivors no `mutants.toml` entry judged, grouped by function: the
/// function with the most first, then by name. Each source line is one line:
/// its first survivor's change, how many more are on that line, and the
/// survivor's key as a `mutants.toml` line. A `duplicate` of such a survivor
/// is listed with it, since it would have come out the same.
fn survivors(scored: &[Scored]) -> String {
    let by_key: BTreeMap<&str, &Scored> = scored.iter().map(|s| (s.key.as_str(), s)).collect();
    let open = |s: &Scored| s.verdict == Verdict::Survived && s.judged.is_none();
    let mut functions: BTreeMap<&str, Vec<&Scored>> = BTreeMap::new();
    for mutant in scored {
        let listed = open(mutant)
            || (mutant.verdict == Verdict::Duplicate
                && mutant
                    .same_as
                    .as_deref()
                    .and_then(|first| by_key.get(first))
                    .is_some_and(|first| open(first)));
        if listed {
            functions
                .entry(mutant.function.as_str())
                .or_default()
                .push(mutant);
        }
    }
    let mut functions: Vec<(&str, Vec<&Scored>)> = functions.into_iter().collect();
    functions.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let mut out = String::new();
    for (function, mutants) in functions {
        out.push_str(&format!("  survived in {function} ({}):\n", mutants.len()));
        let mut lines: BTreeMap<(&str, usize), Vec<&Scored>> = BTreeMap::new();
        for mutant in mutants {
            lines
                .entry((mutant.file.as_str(), mutant.line))
                .or_default()
                .push(mutant);
        }
        for ((file, line), mut on) in lines {
            // A real survivor names the line before a duplicate of one.
            on.sort_by_key(|mutant| mutant.verdict == Verdict::Duplicate);
            let first = on[0];
            let more = match on.len() {
                1 => String::new(),
                n => format!(" (+{} more on this line)", n - 1),
            };
            let key = first.same_as.as_deref().unwrap_or(&first.key);
            out.push_str(&format!(
                "    {file}:{line}  {}{more}  mutant = \"{}\"\n",
                first.change,
                toml_escaped(key)
            ));
        }
    }
    out
}

/// `text` as the inside of a TOML basic string.
fn toml_escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::{Aim, Runs, Scored, Verdict, list, score};
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
        let origins = std::collections::BTreeMap::new();
        let select = Selection::default();
        let (baseline, _) = run_suite_in_a_task(
            &unit,
            ctx,
            Rc::clone(&checked),
            &config,
            &origins,
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
        let (listed, _) = list(&checked, &Aim::default());
        // Every mutant compiles, the unreached ones included, which `score` never compiles.
        let files = checked.program_files();
        for super::Listed { mutant, .. } in &listed {
            let program = nvs_ir::lower::lower_mutant(
                nvs_ir::lower::ENTRY_SCRIPT_LABEL,
                &files,
                &checked.exprs,
                &checked.interner,
                &checked.enums,
                &checked.layouts,
                mutant,
            )
            .program;
            if let Err(error) = nvs_codegen::compile(&program) {
                panic!(
                    "the mutant at line {} ({}) compiles: {error}",
                    mutant.line,
                    mutant.change()
                );
            }
        }
        // And every mutant compiles behind the switch, all of them in one program.
        let armed: Vec<&nvs_ir::lower::Mutant> = listed.iter().map(|listed| &listed.mutant).collect();
        let schema = nvs_ir::lower::lower_schema(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &files,
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
            &armed,
        );
        if let Err(error) = nvs_codegen::compile(&schema) {
            panic!("the program with every mutant behind the switch compiles: {error}");
        }
        let runs = Runs {
            ctx: &crate::script::granting_ctx,
            config: &config,
            origins: &origins,
            extensions: None,
            jobs: crate::runner::default_jobs(),
            select: &select,
        };
        let scored = score(&checked, &listed, &baseline.cases, &stmts, &runs)
            .expect("every mutant compiles")
            .scored;
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
            report.contains("  survived in ") && report.contains("`>=` to `>`"),
            "the human report lists the survived mutant: {report}"
        );
    }

    /// `rule:testing/report-formats`: version 5's `mutants` array reads back as the records written.
    #[test]
    fn the_json_mutants_array_round_trips() {
        let scored = vec![
            Scored {
                file: "src/Shop.nvs".to_owned(),
                line: 12,
                function: r"Shop\Cart::total".to_owned(),
                key: r"Shop\Cart::total#1: `<` to `<=`".to_owned(),
                change: "`<` to `<=`".to_owned(),
                verdict: Verdict::Survived,
                tests: 2,
                same_as: None,
                judged: Some(super::Judgment {
                    reason: "the loop ends on the same item".to_owned(),
                    equivalent: true,
                }),
            },
            Scored {
                file: "src/Shop.nvs".to_owned(),
                line: 20,
                function: r"Shop\Cart::empty".to_owned(),
                key: r"Shop\Cart::empty#1: `true` to `false`".to_owned(),
                change: "`true` to `false`".to_owned(),
                verdict: Verdict::TimedOut,
                tests: 1,
                same_as: None,
                judged: None,
            },
            Scored {
                file: "src/Shop.nvs".to_owned(),
                line: 21,
                function: r"Shop\Cart::empty".to_owned(),
                key: r"Shop\Cart::empty#1: `condition` to `!condition`".to_owned(),
                change: "`condition` to `!condition`".to_owned(),
                verdict: Verdict::Duplicate,
                tests: 0,
                same_as: Some(r"Shop\Cart::empty#1: `==` to `!=`".to_owned()),
                judged: None,
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
        assert_eq!(read["schemaVersion"], 5);
        let mutants = read["mutants"].as_array().expect("`mutants` is an array");
        let back: Vec<(String, u64, String, String, String, String, u64)> = mutants
            .iter()
            .map(|record| {
                (
                    record["file"].as_str().unwrap_or_default().to_owned(),
                    record["line"].as_u64().unwrap_or_default(),
                    record["function"].as_str().unwrap_or_default().to_owned(),
                    record["key"].as_str().unwrap_or_default().to_owned(),
                    record["change"].as_str().unwrap_or_default().to_owned(),
                    record["outcome"].as_str().unwrap_or_default().to_owned(),
                    record["tests"].as_u64().unwrap_or_default(),
                )
            })
            .collect();
        let written: Vec<(String, u64, String, String, String, String, u64)> = scored
            .iter()
            .map(|mutant| {
                (
                    mutant.file.clone(),
                    mutant.line as u64,
                    mutant.function.clone(),
                    mutant.key.clone(),
                    mutant.change.clone(),
                    mutant.verdict.name().to_owned(),
                    mutant.tests as u64,
                )
            })
            .collect();
        assert_eq!(back, written);
        let same_as: Vec<Option<&str>> = mutants
            .iter()
            .map(|record| record["sameAs"].as_str())
            .collect();
        assert_eq!(
            same_as,
            [None, None, Some(r"Shop\Cart::empty#1: `==` to `!=`")]
        );
        let judged: Vec<(Option<bool>, Option<&str>, Option<bool>)> = mutants
            .iter()
            .map(|record| {
                (
                    record["judged"].as_bool(),
                    record["reason"].as_str(),
                    record["equivalent"].as_bool(),
                )
            })
            .collect();
        assert_eq!(
            judged,
            [
                (
                    Some(true),
                    Some("the loop ends on the same item"),
                    Some(true)
                ),
                (None, None, None),
                (None, None, None),
            ]
        );
    }

    /// One survivor for `function` at `line`, with the `n`th key of `change`.
    fn survivor(function: &str, line: usize, change: &str, n: usize) -> Scored {
        Scored {
            file: "shop.nvs".to_owned(),
            line,
            function: function.to_owned(),
            key: format!("{function}#{n}: {change}"),
            change: change.to_owned(),
            verdict: Verdict::Survived,
            tests: 1,
            same_as: None,
            judged: None,
        }
    }

    /// `rule:testing/mutation-testing`: survivors are grouped by function,
    /// most first, with one line per source line and the key to paste.
    #[test]
    fn the_report_groups_survivors_by_function() {
        let mut duplicate = survivor(r"Shop\Cart::total", 7, "`condition` to `!condition`", 1);
        duplicate.verdict = Verdict::Duplicate;
        duplicate.same_as = Some(r"Shop\Cart::total#1: `<` to `<=`".to_owned());
        let scored = vec![
            survivor(r"Shop\Cart::empty", 20, "`true` to `false`", 1),
            survivor(r"Shop\Cart::total", 7, "`<` to `<=`", 1),
            duplicate,
            survivor(r"Shop\Cart::total", 9, "`+` to `-`", 1),
        ];
        let report = super::report(&scored);
        let listed: Vec<&str> = report
            .lines()
            .skip_while(|line| !line.starts_with("  survived in"))
            .collect();
        assert_eq!(
            listed,
            [
                r"  survived in Shop\Cart::total (3):",
                r#"    shop.nvs:7  `<` to `<=` (+1 more on this line)  mutant = "Shop\\Cart::total#1: `<` to `<=`""#,
                r#"    shop.nvs:9  `+` to `-`  mutant = "Shop\\Cart::total#1: `+` to `-`""#,
                r"  survived in Shop\Cart::empty (1):",
                r#"    shop.nvs:20  `true` to `false`  mutant = "Shop\\Cart::empty#1: `true` to `false`""#,
            ],
        );
    }

    /// `mutants.toml` needs a key and a reason in each entry, and allows no
    /// other setting.
    #[test]
    fn a_judged_file_is_read_strictly() {
        let read = super::Judgments::parse(
            "[[judged]]\nmutant = \"Cart::total#1: `<` to `<=`\"\nreason = \"same item\"\nequivalent = true\n",
        )
        .expect("a well-formed file");
        assert_eq!(
            read.0.get("Cart::total#1: `<` to `<=`"),
            Some(&super::Judgment {
                reason: "same item".to_owned(),
                equivalent: true,
            })
        );
        for (text, error) in [
            ("[[judged]]\nmutant = \"a\"\n", "needs a non-empty `reason`"),
            (
                "[[judged]]\nmutant = \"a\"\nreason = \" \"\n",
                "needs a non-empty `reason`",
            ),
            ("[[judged]]\nreason = \"r\"\n", "needs a non-empty `mutant`"),
            (
                "[[judged]]\nmutant = \"a\"\nreason = \"r\"\nwhy = 1\n",
                "`why`",
            ),
            ("judged = 1\n", "array of tables"),
            ("[other]\n", "`other`"),
            (
                "[[judged]]\nmutant = \"a\"\nreason = \"r\"\n[[judged]]\nmutant = \"a\"\nreason = \"s\"\n",
                "judged twice",
            ),
        ] {
            match super::Judgments::parse(text) {
                Ok(_) => panic!("{text:?} was read"),
                Err(message) => assert!(message.contains(error), "{text:?}: {message}"),
            }
        }
    }
}
