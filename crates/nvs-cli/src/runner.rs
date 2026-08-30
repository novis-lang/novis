//! [ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md)'s
//! runner: the `#[Test]` table a compile already built (§ 1), constructed and
//! called (§ 20), judged off § 5's ledger — [`run_case`] owns why that and not
//! the exception state — and reported in one of § 22's three formats.
//!
//! # One verdict, three renderings
//!
//! Every format renders the same [`Case`] list, and a verdict is decided once
//! — [`Outcome::verdict`] — rather than per format, so the plaintext mark, the
//! JSON string and the JUnit element cannot disagree about how a test came
//! out. The plaintext one is the default and is written as the run goes, which
//! is what keeps a long suite legible; a machine format is one document
//! written at the end, because neither JSON nor XML has a prefix worth
//! streaming.
//!
//! **Under a machine format the program's own output goes to stderr.** § 22
//! writes `nvs test --format=json > results.json`, so stdout has to be the
//! document and nothing else — a test's `echo` interleaved into it would make
//! the redirect produce something no parser accepts. It is not captured into
//! the document either: what a test printed is diagnostic output a person
//! reads, and buffering a whole suite's worth of it to quote back is a cost
//! every run would pay for the few that are being debugged.
//!
//! # Why the runner is here
//!
//! It is the one place that can reach both halves at once. The roster is
//! `nvs_types::ExprTypeTable::tests`, which is a *front-end* table, and the
//! instance it constructs is a compiled unit's, reached through a `Ctx` — and
//! `nvs-cli` is the only crate that holds a checked program and a runtime
//! context in one scope. Neither half is discovered at startup: § 1's table
//! was settled while the program was compiled, so there is no directory scan
//! and no reflection here, only a walk of what the compile handed over.
//!
//! # The entry file's own statements do not run
//!
//! `nvs run` calls the script frame; this does not. A `#[Test]` table is
//! discovered at compile time and a program's top-level statements are the
//! *program* — running them would run the thing under test as a side effect of
//! testing it, once per suite and in whatever order the entry point happened
//! to be written. Every file the `require`/`autoload` graph reached is still
//! compiled, because that is what declares the classes and what the checker
//! judged; only the execution of a file body is left out. (A `require`d file's
//! body therefore does not run either, its one caller being the frame that is
//! not called.)
//!
//! # Every test is its own isolate
//!
//! § 2's rule is [`run_in_isolate`]'s one call into `nvs_host::Isolate`: a
//! test's body runs on a context of its own, over the *same* compiled unit, so
//! a static one test wrote reads its declared initial value in the next and
//! there is no flag to change it (ADR 0116 § 4's fresh statics base, armed by
//! the `install_in` inside the child's own program). The isolate is per
//! **test** and not per attempt, which is § 20's retry left usable:
//! [`run_with_retries`] runs inside it.
//!
//! Two things cross that boundary and both are copies. § 8's fixtures are
//! built **once per class** in this process's own context and copied into each
//! test (`nvs_runtime::CrossedFixtures`), which is the whole of what § 8 asks
//! for: the expensive setup runs once, and what a test can reach is its own
//! graph. The verdict crosses the other way as data — [`Outcome`] is decided
//! on the child's stack, off the child's own ledger, and filed into a cell the
//! child's program captured, because ADR 0006's `Completion` carries a value,
//! bytes and a failure and none of those is a ledger.
//!
//! # The runner owns the test's task tree
//!
//! § 16's first half is [`run_suite_in_a_task`]: the whole suite runs inside
//! one task, so a test's isolate is a *child* of it and a `Core\Task::all`
//! written in a test has a calling task to put its own children under. That
//! function owns why it is one task for the suite rather than one per test,
//! and what has to be moved into it to make the scheduler's signature work.
//!
//! The second half is read off that tree at the one point it means anything —
//! the moment the test's body returns, on the test's own stack, in
//! [`run_in_isolate`]'s program closure. **A task still running then is a
//! failure named as such** ([`Outcome::with_tasks_left_running`]), because the
//! alternative is what the scheduler does on its own: cancel the leftovers as
//! the task retires and report a green test that never waited for its work.
//! ADR 0072 § 4 makes that bound reachable rather than aspirational — `::all`
//! and `::map` return with nothing still running — so the only way to fail
//! this is a `spawn script` the test never awaited.
//!
//! # What is owed
//!
//! § 2's parallelism is not built: the isolates are made and joined one at a
//! time, which is a scheduling question rather than an isolation one. A
//! constructor that declares parameters is
//! reported as that test failing rather than pretended past
//! (`nvs_runtime::construct_and_call`): § 7 makes the constructor `setUp` and
//! §§ 8-9 fill the test method's own parameters, which [`build_fixtures`]
//! builds once per class and [`invocations`] materializes per row. Class order is the roster's own sorted
//! order rather than § 20's declaration order: `ExprTypeTable::tests` is keyed
//! by class label and records no sequence, so declaration order across a
//! program's files is a fact only that table can grow. Within a class the
//! order *is* declaration order, which is what a reader of one file sees.

use std::cell::RefCell;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::{Duration, Instant};

use nvs_types::defaults::ConstArg;

/// § 22's three report formats.
///
/// The roster lives here rather than in `main`, because what a format *is* is
/// this module's question and `main` only has to name one on the command line.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Format {
    /// § 22's default: one line per test as the run goes, then the summary.
    Human,
    /// § 22's versioned JSON document, written to stdout at the end.
    Json,
    /// § 22's JUnit XML, the shape every CI system already ingests.
    Junit,
}

/// How one test came out.
enum Outcome {
    Passed,
    /// § 20's "a skip states a reason", carrying it.
    Skipped(String),
    /// Why it failed — every failed assertion the ledger recorded, or the one
    /// message a throw or an empty ledger left. A list rather than the first
    /// one, because a test that caught its own failures and carried on has
    /// several, and reporting one of them would hide exactly what § 5's ledger
    /// exists to keep.
    Failed(Vec<String>),
    /// § 20's `retries:`: the test failed and then passed within its
    /// allowance, carrying how many attempts it took and what the last failed
    /// one said. It is neither passed nor failed, which is the whole of that
    /// bullet — see [`run_with_retries`].
    Flaky {
        attempts: usize,
        failures: Vec<String>,
    },
    /// The test called `exit(n)`, which ends the whole run.
    Exited(i64),
}

impl Outcome {
    /// The one word every format names this outcome by.
    ///
    /// Each rendering reads its own mark, string or element off *this* rather
    /// than matching the outcome a second time, which is what makes "the two
    /// cannot disagree about a verdict" a property of the code rather than of
    /// a review.
    fn verdict(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Skipped(_) => "skipped",
            Self::Failed(_) => "failed",
            Self::Flaky { .. } => "flaky",
            Self::Exited(_) => "exited",
        }
    }

    /// The plaintext mark, beside the verdict it renders so that a variant
    /// this enum grows cannot take one without taking the other.
    fn mark(&self) -> &'static str {
        match self {
            Self::Passed => "\u{2713}",
            Self::Skipped(_) => "-",
            // Its own mark rather than the passing one: § 20 reports a retried
            // test as flaky and never as green, and a reader scanning the
            // marks is the first place that has to be true.
            Self::Flaky { .. } => "!",
            Self::Failed(_) | Self::Exited(_) => "\u{2717}",
        }
    }

    /// § 16's second half folded in: `running` children the test left behind
    /// when it returned turn any verdict a report could call green into a
    /// failure naming them.
    ///
    /// A skip never ran and an `exit(n)` ended the whole program rather than
    /// the test, so neither is an outcome this can say anything about; every
    /// other one gains the line, a flaky test included — a test that leaks a
    /// task is not one retrying made honest.
    fn with_tasks_left_running(self, running: usize) -> Self {
        if running == 0 {
            return self;
        }
        let named = format!(
            "it left {running} task(s) still running when it returned: ADR 0079 § 16 \
             fails a test whose task tree outlives it — `Core\\Task::all` and `::map` \
             return with nothing still running, and a `spawn script` is finished by `await`"
        );
        match self {
            Self::Passed => Self::Failed(vec![named]),
            Self::Failed(mut failures) | Self::Flaky { mut failures, .. } => {
                failures.push(named);
                Self::Failed(failures)
            }
            settled @ (Self::Skipped(_) | Self::Exited(_)) => settled,
        }
    }

    /// What went wrong, in the one line a machine format's `message` carries —
    /// `None` for a test that did not fail. Several failed assertions read as
    /// the first, the whole list staying available in the document's own
    /// `failures` array.
    fn message(&self) -> Option<String> {
        match self {
            Self::Passed => None,
            Self::Skipped(reason) => Some(reason.clone()),
            Self::Failed(reasons)
            | Self::Flaky {
                failures: reasons, ..
            } => Some(
                reasons
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "it failed".to_owned()),
            ),
            Self::Exited(code) => Some(format!("the test called exit({code})")),
        }
    }
}

/// One test's place in the run: what was called, how it came out, how long it
/// took. The machine formats render a list of these at the end; the plaintext
/// one renders each as it arrives.
struct Case {
    class: String,
    method: String,
    outcome: Outcome,
    elapsed: Duration,
}

/// Compiles `checked` and runs every `#[Test]` it declares, reporting in
/// `format`.
pub(crate) fn run(checked: crate::Checked, format: Format) -> ExitCode {
    let unit = match compile(&checked) {
        Ok(unit) => unit,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    // The module doc owns why a machine format sends the program's own output
    // to stderr: stdout is the document, and nothing else may be written to it.
    let mut ctx = match format {
        Format::Human => nvs_runtime::Ctx::stdout(),
        Format::Json | Format::Junit => nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Stderr),
    };
    unit.install_in(&mut ctx);

    let started = Instant::now();
    let (suite, mut ctx) = match run_suite_in_a_task(&unit, ctx, checked, format) {
        Ok(both) => both,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let Suite {
        cases,
        counts,
        exited,
    } = suite;
    // Flushed before the summary: a test's own `echo` is buffered on the
    // context, and printing the counts over the top of it would report a run
    // whose output had not been written yet.
    if let Err(error) = ctx.flush_output() {
        eprintln!("error: could not flush output: {error}");
        return ExitCode::FAILURE;
    }
    match format {
        Format::Human => println!(
            "\n  {} failed, {} passed, {} skipped, {} flaky in {:.0} ms",
            counts.failed,
            counts.passed,
            counts.skipped,
            counts.flaky,
            started.elapsed().as_secs_f64() * 1000.0
        ),
        Format::Json => print!("{}", json_document(&cases, counts, started.elapsed())),
        Format::Junit => print!("{}", junit_document(&cases, counts, started.elapsed())),
    }

    if let Some(code) = exited {
        // `exit(n)` ended the program the way `nvs run` reports one, and the
        // suite is over whatever the counts say.
        return ExitCode::from(u8::try_from(code & 0xFF).unwrap_or(0));
    }
    if counts.failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Runs the whole suite **inside one task**, on a [`nvs_host::Scheduler`] of
/// this run's own with a reactor and an isolate resolver installed over it —
/// ADR 0079 § 16's "the runner owns the test's task tree", and the same three
/// installations `nvs run` makes (`crate::main`, § *What `run` executes*).
///
/// Without it there is no task beneath a test at all: `nvs_host::Wake::current`
/// answers `None`, so a test's own isolate runs on this stack rather than as a
/// child (`nvs_host::isolate::Isolate::start`), a `Core\Task::all` in a test
/// has no calling task to put children under (ADR 0072 § 1), and § 16 has no
/// tree to read a leftover off. One task and not one per test: the tests are a
/// *suite*, and each test's own isolate is a child of it.
///
/// `checked` and the unit are moved in rather than borrowed, because a task's
/// body outlives the call that spawns it as far as the scheduler's signature
/// is concerned. What comes back out comes back by the two routes `nvs run`
/// already uses: the [`Suite`] through a cell the body captured, the `Ctx`
/// through the `Finished` the scheduler hands over, since it was moved into
/// the task rather than lent to it.
///
/// `Err` is the message to render — an internal failure of the run itself,
/// never a verdict about a test.
fn run_suite_in_a_task(
    unit: &Rc<nvs_codegen::Unit>,
    ctx: nvs_runtime::Ctx,
    checked: crate::Checked,
    format: Format,
) -> Result<(Suite, nvs_runtime::Ctx), String> {
    let mut sched = nvs_host::Scheduler::new();
    let filed: Rc<RefCell<Option<Suite>>> = Rc::new(RefCell::new(None));
    let collected = Rc::clone(&filed);
    let suite_unit = Rc::clone(unit);
    let root = sched.spawn(ctx, nvs_runtime::TaskRoot::Request, move |ctx| {
        *collected.borrow_mut() = Some(run_suite(&suite_unit, ctx, &checked, format));
    });

    // The reactor is what a parked task is woken by, and a test that sleeps or
    // reads a socket parks exactly as any other program does — `run_until_idle`
    // refuses a scheduler with no reactor on its thread.
    let reactor = nvs_host::Reactor::new()
        .map_err(|error| format!("could not start the reactor: {error}"))?;
    let installed = nvs_host::reactor::install(reactor);
    // A test's own isolate is built by this crate directly, but a `spawn script`
    // *inside* a test goes through the seam and needs the same resolver
    // `nvs run` installs — one per run, so two tests spawning one path share
    // the compiled unit (`crate::script`).
    let compiler = crate::script::Compiler::default();
    let ran = nvs_runtime::script::scoped(&compiler, || nvs_host::run_until_idle(&mut sched));
    drop(installed);
    ran.map_err(|error| format!("the scheduler stopped: {error}"))?;

    let Some(finished) = sched
        .take_finished()
        .into_iter()
        .find(|finished| finished.id == root)
    else {
        // Nothing can cancel the suite's task: it is the root, and the run is
        // over by the time this is read.
        return Err("internal error: the suite's task did not finish".to_owned());
    };
    if let Err(panic) = finished.outcome {
        // ADR 0106 § 2's boundary contained a panic under the task root. For a
        // test run that is this process's failure and not a verdict — the
        // runner is what broke, so there is no suite to report.
        return Err(format!("the test run panicked: {}", panic.message()));
    }
    let suite = filed
        .borrow_mut()
        .take()
        .ok_or_else(|| "internal error: the suite's task ran nothing".to_owned())?;
    Ok((suite, finished.ctx))
}

/// Lowers and compiles `checked` into the **one** unit every test isolate of
/// this run shares — ADR 0079 § 2's "shares compiled code with its siblings"
/// is this `Rc` and the program closure each child holds a clone of.
///
/// `Err` is the message to render; a compile that fails ends the run rather
/// than any one test.
fn compile(checked: &crate::Checked) -> Result<Rc<nvs_codegen::Unit>, String> {
    let program = nvs_ir::lower::lower_program(
        crate::SCRIPT,
        &checked.program_files(),
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    );
    nvs_codegen::compile(&program)
        .map(Rc::new)
        .map_err(|error| error.to_string())
}

/// A whole run's verdict, before any of § 22's three renderings has been
/// chosen — which is also the shape this module's own tests read, a rendering
/// being the one thing they are not about.
struct Suite {
    cases: Vec<Case>,
    counts: Counts,
    /// The code an `exit(n)` in a test or a fixture ended the run with.
    exited: Option<i64>,
}

/// Runs every `#[Test]` `checked` declares, in § 20's order, reporting each as
/// it arrives under [`Format::Human`] and only collecting under the other two.
fn run_suite(
    unit: &Rc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    checked: &crate::Checked,
    format: Format,
) -> Suite {
    let (mut passed, mut failed, mut skipped, mut flaky) = (0_usize, 0_usize, 0_usize, 0_usize);
    let mut exited = None;
    let mut cases = Vec::new();
    for class in checked.exprs.test_classes() {
        if format == Format::Human {
            println!("  {class}");
        }
        let tests = checked.exprs.tests(class).unwrap_or_default();
        // § 8's "built once, in the parent" is this call: one set per class,
        // built before its first test and dropped after its last, so a fixture
        // is not rebuilt per test and not kept past the class that declared it.
        let mut fixtures = nvs_runtime::Fixtures::new();
        let unbuilt = build_fixtures(unit, ctx, class, checked, tests, &mut fixtures);
        for call in tests.iter().flat_map(invocations) {
            let (case, label, row) = (call.case, call.label, call.row);
            let began = Instant::now();
            let outcome = match &unbuilt {
                // A fixture that would not build is reported against every
                // test that asked for one, rather than against the class: a
                // test is what a report has a line for, and a suite that lost
                // a whole class silently is what § 20 is written against.
                Some(FixtureFailure::Exited(code)) => Outcome::Exited(*code),
                Some(FixtureFailure::Threw(message)) if !fixtures_needed(case).is_empty() => {
                    Outcome::Failed(vec![message.clone()])
                }
                _ => run_in_isolate(unit, ctx, class, case, row, &fixtures),
            };
            let elapsed = began.elapsed();
            match &outcome {
                Outcome::Passed => passed += 1,
                Outcome::Skipped(_) => skipped += 1,
                Outcome::Flaky { .. } => flaky += 1,
                Outcome::Failed(_) | Outcome::Exited(_) => failed += 1,
            }
            if format == Format::Human {
                report(&label, &outcome, elapsed);
            }
            if let Outcome::Exited(code) = outcome {
                exited = Some(code);
            }
            cases.push(Case {
                class: class.to_owned(),
                method: label,
                outcome,
                elapsed,
            });
            if exited.is_some() {
                break;
            }
        }
        if exited.is_some() {
            break;
        }
    }
    Suite {
        cases,
        counts: Counts {
            passed,
            failed,
            skipped,
            flaky,
        },
        exited,
    }
}

/// One `#[Test]` method, run as many times as § 20's `retries:` allows.
///
/// **A retry is visible or it is worthless.** That bullet's whole point is
/// that a suite which retries stays honest about it, so a test that failed and
/// then passed within its allowance is [`Outcome::Flaky`] — a fifth verdict of
/// its own — rather than the passing one it would be if the last attempt were
/// simply the answer. What it carries is the *last failed* attempt's own
/// failures, because those are the run that says something: the attempt that
/// passed produced nothing to report.
///
/// Only a failure is retried. A skip never ran, and an `exit(n)` ended the
/// whole program rather than the test (ADR 0020), so neither is an outcome a
/// second attempt could improve on. Each attempt is a fresh instance with the
/// ledger and any pending exception taken between them ([`run_case`] takes
/// both on every path), which is what keeps one attempt's failures from being
/// reported against the next.
///
/// **Every attempt is inside the one isolate**, which is why this loop runs on
/// the child's stack rather than around it. § 2 gives an isolate to a *test*,
/// and the attempts are that test: a retry that could not see what the attempt
/// before it wrote to class storage would make the whole option unusable for
/// the thing it exists for — a test that succeeds on a second try because the
/// first left something behind.
///
/// **A flaky test does not fail the run.** Its exit code is the passing one,
/// because retries exist precisely so that a suite can pass in spite of one —
/// what § 20 takes away is the *silence*, not the green build.
fn run_with_retries(
    unit: &Rc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    method: &str,
    args: &[nvs_runtime::Value],
    allowance: usize,
) -> Outcome {
    let mut failures = match run_case(unit, ctx, class, method, args) {
        Outcome::Failed(failures) => failures,
        settled => return settled,
    };
    for retry in 0..allowance {
        match run_case(unit, ctx, class, method, args) {
            Outcome::Passed => {
                return Outcome::Flaky {
                    attempts: retry + 2,
                    failures,
                };
            }
            Outcome::Failed(again) => failures = again,
            settled => return settled,
        }
    }
    Outcome::Failed(failures)
}

/// Why a class's fixtures could not be built — reported against the tests
/// that asked for one.
enum FixtureFailure {
    /// The fixture body threw, carrying its message.
    Threw(String),
    /// The fixture body called `exit(n)`, which ends the whole run exactly as
    /// a test's own `exit` does.
    Exited(i64),
}

/// ADR 0079 § 8's fixtures for one class, built **once** and in dependency
/// order, into `fixtures`.
///
/// Only what a test that is going to run actually asks for is built: a fixture
/// nothing names would be setup nobody wanted, and § 20's `skip:` means the
/// test never runs, so building its fixture would run code the author asked to
/// skip.
///
/// The order is a post-order walk of the roster's own resolution — each row's
/// [`nvs_types::testing::Fixture::fixtures`] names what it needs, and
/// `nvs_types::testing` refused a cycle among them
/// (`E_FIXTURE_CYCLE`), which is what makes this walk terminate and what makes
/// every value a call needs already built when it is made.
fn build_fixtures(
    unit: &nvs_codegen::Unit,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    checked: &crate::Checked,
    tests: &[nvs_types::testing::TestCase],
    fixtures: &mut nvs_runtime::Fixtures,
) -> Option<FixtureFailure> {
    let roster = checked.exprs.fixtures(class).unwrap_or_default();
    if roster.is_empty() {
        return None;
    }
    let mut order: Vec<&str> = Vec::new();
    for case in tests {
        if skip_reason(case).is_some() {
            continue;
        }
        for needed in fixtures_needed(case) {
            collect_fixture(needed, roster, &mut order);
        }
    }
    for name in order {
        let Some(row) = roster.iter().find(|row| row.method == name) else {
            continue;
        };
        match unit.build_fixture(ctx, fixtures, class, &row.method, &row.fixtures) {
            Some(Ok(())) => {}
            Some(Err(status)) if status == nvs_runtime::EXITED => {
                return Some(FixtureFailure::Exited(ctx.exit_code()));
            }
            Some(Err(_)) => {
                return Some(FixtureFailure::Threw(
                    ctx.take_thrown().message().to_owned(),
                ));
            }
            None => {
                return Some(FixtureFailure::Threw(format!(
                    "internal error: the compiled unit declares no `{class}::{name}`"
                )));
            }
        }
    }
    None
}

/// Appends `name` to `order` behind everything it needs, once.
fn collect_fixture<'a>(
    name: &'a str,
    roster: &'a [nvs_types::testing::Fixture],
    order: &mut Vec<&'a str>,
) {
    if order.contains(&name) {
        return;
    }
    let Some(row) = roster.iter().find(|row| row.method == name) else {
        return;
    };
    for needed in &row.fixtures {
        collect_fixture(needed, roster, order);
    }
    order.push(name);
}

/// The `retries:` option's allowance — how many *further* attempts a failed
/// test is given — or `0` for a test that is run once.
///
/// Read back the way [`skip_reason`] reads its own, and for that function's
/// reason: `nvs_types::testing` has already refused a `retries` that folded to
/// anything but an `int`, so answering "absent" for one here would be a second
/// answer to a settled question. A negative allowance is a run of one, `-1`
/// attempts being nothing this can do.
fn retry_allowance(case: &nvs_types::testing::TestCase) -> usize {
    case.options
        .iter()
        .find_map(|(name, value)| match value {
            ConstArg::Int(allowance) if name == "retries" => Some(*allowance),
            _ => None,
        })
        .and_then(|allowance| usize::try_from(allowance).ok())
        .unwrap_or(0)
}

/// ADR 0079 § 12's `at:` option — the instant this test's isolate reads its
/// wall clock at, in nanoseconds since the Unix epoch — or `None` for a test
/// that reads the host's clock.
///
/// Read back the way [`skip_reason`] and [`retry_allowance`] read theirs, with
/// the one difference this option has: `nvs_types::testing` admitted it at type
/// `string` and made no claim about what the text *says*, so unlike those two
/// there is still a question left to answer here. It is answered by
/// `nvs_stdlib::time`, which owns every conversion between an instant and its
/// text, so the grammar a test writes is the grammar `Instant::toIso` emits and
/// nothing keeps two readings of RFC 3339 in step.
///
/// # Errors
///
/// The offending text, for an `at:` that is not an RFC 3339 timestamp.
fn fixed_clock(case: &nvs_types::testing::TestCase) -> Result<Option<i128>, &str> {
    let Some(at) = case.options.iter().find_map(|(name, value)| match value {
        ConstArg::Str(at) if name == "at" => Some(at.as_str()),
        _ => None,
    }) else {
        return Ok(None);
    };
    nvs_stdlib::time::fixed_clock_nanos(at).map(Some).ok_or(at)
}

/// ADR 0079 § 12's `seed:` option — the seed this test's isolate puts its
/// `Core\Random` draws on — or `None` for a test that draws from the operating
/// system.
///
/// Read back the way [`retry_allowance`] reads its own, and with the same
/// reasoning: `nvs_types::testing` has already refused a `seed` that folded to
/// anything but an `int`, so answering "absent" for one here would be a second
/// answer to a settled question. A **negative** seed is a seed like any other —
/// the generator's state is 64 bits and every one of them is a starting point,
/// so the two's-complement bits are taken as they lie rather than a sign being
/// treated as an error the checker did not treat as one.
fn random_seed(case: &nvs_types::testing::TestCase) -> Option<u64> {
    case.options.iter().find_map(|(name, value)| match value {
        ConstArg::Int(seed) if name == "seed" => Some(seed.cast_unsigned()),
        _ => None,
    })
}

/// One `#[Test]` method: skipped for its stated reason, or a fresh instance of
/// its class with the method called on it.
///
/// **The verdict is read off ADR 0079 § 5's ledger, not off the exception
/// state**, and that is the whole of what makes a test hard to pass by
/// accident. A body that caught its own `Core\Test\Failure` returns normally
/// and has still failed; a body that asserted nothing at all returns normally
/// too and fails for a second reason (§ 20). A throw is therefore the *last*
/// thing consulted rather than the first, and it only ever answers for a test
/// whose ledger is clean — an exception raised by the code under test rather
/// than by an assertion about it.
///
/// **The instance is constructed inside the isolate, not before it.** § 2's
/// boundary is around the whole test, so everything the test can observe —
/// its receiver, its statics, its ledger, its `echo` — is built on the child's
/// own context and released with it. What this function does on the parent's
/// side is only what has to happen before the child exists: § 9's row, § 8's
/// copies, and the closure the two are moved into.
fn run_in_isolate(
    unit: &Rc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    case: &nvs_types::testing::TestCase,
    row: Option<&[Option<ConstArg>]>,
    fixtures: &nvs_runtime::Fixtures,
) -> Outcome {
    if let Some(reason) = skip_reason(case) {
        return Outcome::Skipped(reason);
    }
    // §§ 8-9's injection: one value per declared parameter, in the order the
    // checker resolved them (`nvs_types::testing::TestCase::params`), so
    // nothing here re-derives which fixture answers which parameter or which
    // field answered which name. A fixture's value is this test's own copy and
    // a row's is materialized just below — the call retains its own either
    // way, exactly as any other call on a value this frame holds does.
    let needed: Vec<String> = fixtures_needed(case)
        .into_iter()
        .map(str::to_owned)
        .collect();
    // § 8's crossing, made here and not in the child: the source is the
    // parent's set and the walk consumes a reference to each of its values, so
    // it runs where that set is (`nvs_runtime::CrossedFixtures`).
    let crossed = match nvs_runtime::CrossedFixtures::copy(fixtures, &needed) {
        Some(Ok(crossed)) => crossed,
        // A fixture the parent built that has no meaning inside an isolate —
        // a closure, an object over a host handle. It is reported against the
        // test that asked for it, which is where the name in the message
        // means something, and nothing has run.
        Some(Err(refused)) => {
            return Outcome::Failed(vec![format!(
                "`{class}::{}` asks for a `#[Fixture]` that cannot cross into its isolate: \
                 {refused}",
                case.method
            )]);
        }
        None => {
            return Outcome::Failed(vec![format!(
                "internal error: `{class}::{}` asks for a `#[Fixture]` that was not built",
                case.method
            )]);
        }
    };
    // Dropped when this call is over and not before: it holds the one
    // reference each materialized value carries, and the call only borrows
    // them (`nvs_runtime::RowValues`).
    let mut materialized = nvs_runtime::RowValues::new();
    let mut built = crossed.values().iter().copied();
    let mut args = Vec::with_capacity(case.params.len());
    for (position, source) in case.params.iter().enumerate() {
        let value = match source {
            nvs_types::testing::Injection::Fixture(_) => built.next(),
            nvs_types::testing::Injection::Row => row
                .and_then(|row| row.get(position))
                .and_then(Option::as_ref)
                .and_then(|constant| materialize(&mut materialized, constant)),
        };
        let Some(value) = value else {
            // An internal inconsistency either way: the checker filled every
            // parameter from one roster or the other and refused the
            // declaration outright where it could not (`E0736`), so a hole
            // here is this runner disagreeing with the table it was handed.
            return Outcome::Failed(vec![format!(
                "internal error: `{class}::{}` has no value for parameter {position}",
                case.method
            )]);
        };
        args.push(value);
    }
    // The verdict is decided on the child's own stack and filed into a cell
    // its program captured, because a `Completion` carries a value, bytes and
    // a failure and none of those is § 5's ledger. Nothing about that cell
    // crosses the heap boundary: an [`Outcome`] is `String`s the runner owns,
    // built while the child's context is still alive and read after it is
    // gone.
    let filed: Rc<RefCell<Option<Outcome>>> = Rc::new(RefCell::new(None));
    let verdict = Rc::clone(&filed);
    let allowance = retry_allowance(case);
    // § 12's clock, resolved on the parent's side so that a malformed `at:` is
    // this test's own reported failure rather than something the child has to
    // carry back across the boundary — nothing has run at this point, which is
    // what makes the message the whole of what happened.
    let clock = match fixed_clock(case) {
        Ok(clock) => clock,
        Err(at) => {
            return Outcome::Failed(vec![format!(
                "`{class}::{}` declares `at: \"{at}\"`, which is not an RFC 3339 timestamp such \
                 as `2026-01-01T00:00:00Z`",
                case.method
            )]);
        }
    };
    let seed = random_seed(case);
    let child_unit = Rc::clone(unit);
    let class_name = class.to_owned();
    let method = case.method.clone();
    let program: nvs_runtime::script::Program = Box::new(move |child, argument| {
        // ADR 0116 § 4's fresh statics base, armed with *this* unit's tables:
        // the isolate's context deliberately arrives with none, so this call
        // is the whole of why one test does not read back another's static.
        child_unit.install_in(child);
        // § 12's fixed clock, armed before a line of the test runs and onto the
        // *child's* context, which is the only context a test can observe: a
        // sibling declaring no `at:` still reads the host's clock, because § 2
        // gave it a context of its own. `Core\Test::advance` moves this same
        // field and nothing else does (`nvs_runtime::Ctx::fixed_clock`).
        if let Some(nanos) = clock {
            child.set_fixed_clock(nanos);
        }
        // § 12's other half, and armed the same way and in the same place: the
        // generator's state lives on the child's own context, so a sibling that
        // declared no `seed:` still draws from the CSPRNG and one test's draws
        // cannot move another's (`nvs_stdlib::random::draw`).
        if let Some(seed) = seed {
            child.set_random_state(seed);
        }
        // Null here, and discharged anyway: the seam's contract is that
        // whatever crossed becomes the isolate's own root's, and a test's
        // values crossed as § 8's copies instead.
        child.set_isolate_argument(argument);
        // Both owners go down with the isolate, after the call that borrowed
        // their values has returned.
        let (_crossed, _materialized) = (crossed, materialized);
        let outcome = run_with_retries(&child_unit, child, &class_name, &method, &args, allowance);
        // § 16, read off the tree from inside the test's own task and nowhere
        // else: a child spawned here is a child of *this* task, and once this
        // closure returns the scheduler cancels whatever is left rather than
        // reporting it. `nvs_host::children_still_running` owns why an awaited
        // child is already gone from the count.
        *verdict.borrow_mut() =
            Some(outcome.with_tasks_left_running(nvs_host::children_still_running()));
        nvs_runtime::Value::null()
    });
    // `Output::Inherit`: what a test echoed is appended to the runner's own
    // stream at the join, which is where the ordering against the run's own
    // report is a fact rather than a race (`nvs_runtime::host::Output`).
    let completion = match nvs_host::Isolate::new(
        program,
        nvs_runtime::Value::null(),
        nvs_host::Output::Inherit,
    )
    .run(ctx)
    {
        Ok(completion) => completion,
        // Not reachable: the only `Err` this boundary has is about the
        // argument, and the argument is `null`. Reported rather than
        // unwrapped, because a panic in the runner is the one thing a program
        // under test may not be able to cause.
        Err(refused) => {
            return Outcome::Failed(vec![format!(
                "internal error: the test's isolate refused its argument: {refused}"
            )]);
        }
    };
    let taken = filed.borrow_mut().take();
    taken.unwrap_or_else(|| {
        // The child never reached its own verdict — it was cancelled, or a
        // panic inside it was contained (`nvs_host::isolate`'s module doc), so
        // what the boundary carries is all there is to report.
        Outcome::Failed(vec![completion.error.map_or_else(
            || "the test's isolate ended without reaching a verdict".to_owned(),
            |failure| format!("{}: {}", failure.class, failure.message),
        )])
    })
}

/// One attempt at one test: a fresh instance of `class`, its `method` called
/// on that instance with `args`, and § 5's verdict read off the ledger.
///
/// **Every line of it runs on the isolate's own stack** — the instance, the
/// ledger and the pending exception are all the child's, and reading any of
/// them after that context's wholesale release would be reading a context that
/// is gone. [`run_in_isolate`] is the half that runs before the child exists.
fn run_case(
    unit: &Rc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    method: &str,
    args: &[nvs_runtime::Value],
) -> Outcome {
    let Some(outcome) = unit.call_on_new_instance(ctx, class, method, args) else {
        return Outcome::Failed(vec![format!(
            "internal error: the compiled unit declares no class `{class}`"
        )]);
    };
    let thrown = match outcome {
        Ok(()) => None,
        // `exit` is not a verdict about the test at all, and the pending state
        // it leaves is a status rather than an exception — read before either
        // of the two below, for the reason `nvs run` reads it first.
        Err(status) if status == nvs_runtime::EXITED => return Outcome::Exited(ctx.exit_code()),
        // Taken whether or not it is what the verdict ends up being: a pending
        // exception left on the context would be the *next* test's to find.
        Err(_) => Some(ctx.take_thrown().message().to_owned()),
    };
    // Taken on every path, and for the same reason: one test's ledger is that
    // test's (`nvs_runtime::Ctx::take_assertions`), so consuming it here is
    // what keeps an earlier failure from being reported against a later test.
    let ledger = ctx.take_assertions();
    let failures: Vec<String> = ledger
        .iter()
        .filter_map(|entry| entry.failure.clone())
        .collect();
    if !failures.is_empty() {
        return Outcome::Failed(failures);
    }
    if let Some(message) = thrown {
        return Outcome::Failed(vec![message]);
    }
    if ledger.is_empty() {
        // § 20's first bullet, with the way out that section names: a test
        // whose whole claim is that nothing threw writes
        // `Core\Test::assertDoesNotThrow(callable)`, which records an entry
        // like any other assertion.
        return Outcome::Failed(vec![
            "it asserted nothing: ADR 0079 § 20 fails a test whose ledger is empty — write \
             `Core\\Test::assertDoesNotThrow` where the claim is that a call completes"
                .to_owned(),
        ]);
    }
    Outcome::Passed
}

/// The calls one `#[Test]` method makes — ADR 0079 § 9's "each row is its own
/// reported case".
///
/// A method with no `#[TestWith]` row is § 1's single call under its own name.
/// A method with rows is one call per row, in the source order the checker
/// recorded them, each labelled `method#N` — a **label** rather than a field
/// beside the name, so the plaintext mark, the JSON object and the JUnit
/// element tell two rows apart without any of the three growing a rendering of
/// its own, exactly as they read one verdict rather than deciding one each.
///
/// A `skip:`ped method is expanded too and reports one skipped case per row:
/// what § 20 skips is a *test*, and each row is one.
fn invocations(case: &nvs_types::testing::TestCase) -> Vec<Invocation<'_>> {
    if case.rows.is_empty() {
        return vec![Invocation {
            case,
            label: case.method.clone(),
            row: None,
        }];
    }
    case.rows
        .iter()
        .enumerate()
        .map(|(index, row)| Invocation {
            case,
            label: format!("{}#{index}", case.method),
            row: Some(row.as_slice()),
        })
        .collect()
}

/// One call [`invocations`] hands back: the method to run, the label it is
/// reported under, and the row it is filled from — `None` for the § 1 method
/// that has none.
struct Invocation<'a> {
    case: &'a nvs_types::testing::TestCase,
    label: String,
    row: Option<&'a [Option<ConstArg>]>,
}

/// One folded row field as the value the call takes, owned by `row`.
///
/// The roster is exactly what `nvs_types::defaults::literal_default` folds a
/// row field to — the four scalars and a `string` — because § 9 matches a
/// field against its parameter's *declared* type, and that is the closed list
/// of literals such a parameter can be declared at. Anything else answers
/// `None` and is reported as the internal inconsistency it would be, rather
/// than materialized as something plausible.
fn materialize(
    row: &mut nvs_runtime::RowValues,
    constant: &ConstArg,
) -> Option<nvs_runtime::Value> {
    Some(match constant {
        ConstArg::Bool(value) => row.bool(*value),
        ConstArg::Int(value) => row.int(*value),
        ConstArg::Uint(value) => row.uint(*value),
        ConstArg::Float(value) => row.float(*value),
        ConstArg::Str(text) => row.str(text),
        _ => return None,
    })
}

/// The `#[Fixture]` methods `case` asks for, in parameter order.
///
/// A §§ 8-9 parameter list is a list of *sources*
/// (`nvs_types::testing::Injection`), and only a fixture's has a value to be
/// built ahead of the call: a row's is folded into the case itself. So this is
/// the fixture positions alone, which is what both the build order and
/// `nvs_runtime::Fixtures::values` are keyed by.
fn fixtures_needed(case: &nvs_types::testing::TestCase) -> Vec<&str> {
    case.params
        .iter()
        .filter_map(|source| match source {
            nvs_types::testing::Injection::Fixture(name) => Some(name.as_str()),
            nvs_types::testing::Injection::Row => None,
        })
        .collect()
}

/// The `skip:` option's reason, or `None` for a test that runs.
///
/// § 20 makes the reason the whole point of the option — `#[Test(skip: true)]`
/// does not compile — so a `skip` that folded to anything but a string is one
/// `nvs_types::testing` has already refused, and reading it back as absent
/// here would be a second answer to a settled question.
fn skip_reason(case: &nvs_types::testing::TestCase) -> Option<String> {
    case.options.iter().find_map(|(name, value)| match value {
        ConstArg::Str(reason) if name == "skip" => Some(reason.clone()),
        _ => None,
    })
}

/// How the run came out, in the three counts every format's summary carries.
#[derive(Clone, Copy)]
struct Counts {
    passed: usize,
    failed: usize,
    skipped: usize,
    /// § 20's own section: a test that passed within its retry allowance is
    /// counted here and in neither of the two above it.
    flaky: usize,
}

impl Counts {
    fn total(self) -> usize {
        self.passed + self.failed + self.skipped + self.flaky
    }
}

/// § 22's one line per test: the mark, the method's own name, and how long it
/// took.
fn report(method: &str, outcome: &Outcome, elapsed: Duration) {
    let mark = outcome.mark();
    println!(
        "    {mark} {method:<32} {:.1} ms",
        elapsed.as_secs_f64() * 1000.0
    );
    match outcome {
        Outcome::Passed => {}
        Outcome::Skipped(reason) => println!("      skipped: {reason}"),
        Outcome::Failed(reasons) => {
            for reason in reasons {
                println!("      {reason}");
            }
        }
        Outcome::Flaky { attempts, failures } => {
            println!("      flaky: it passed on attempt {attempts}, having failed with:");
            for reason in failures {
                println!("      {reason}");
            }
        }
        Outcome::Exited(code) => println!("      the test called exit({code})"),
    }
}

/// § 22's versioned JSON: one object per run, over the very [`Case`] list the
/// plaintext report renders a line at a time.
///
/// The version is at the root because that section promises a *versioned*
/// schema, and a consumer that reads it knows what the rest of the keys mean.
/// What § 22 additionally names — a structured diff, `#[Bench]`'s counters, a
/// shrunk property counterexample, per-data-row results and per-test coverage
/// — is absent because nothing produces any of it yet; each arrives as a new
/// key beside these, which is the whole of what the version number buys.
fn json_document(cases: &[Case], counts: Counts, total: Duration) -> String {
    let mut out = String::from("{\n  \"schemaVersion\": 1,\n  \"summary\": {");
    out.push_str(&format!(
        "\"total\": {}, \"passed\": {}, \"failed\": {}, \"skipped\": {}, \"flaky\": {}, \"durationMs\": {:.3}}},\n  \"tests\": [",
        counts.total(),
        counts.passed,
        counts.failed,
        counts.skipped,
        counts.flaky,
        millis(total)
    ));
    for (at, case) in cases.iter().enumerate() {
        out.push_str(if at == 0 { "\n    {" } else { ",\n    {" });
        out.push_str("\"class\": ");
        json_string(&case.class, &mut out);
        out.push_str(", \"method\": ");
        json_string(&case.method, &mut out);
        out.push_str(", \"verdict\": ");
        json_string(case.outcome.verdict(), &mut out);
        out.push_str(&format!(", \"durationMs\": {:.3}", millis(case.elapsed)));
        match &case.outcome {
            Outcome::Passed => {}
            Outcome::Skipped(reason) => {
                out.push_str(", \"reason\": ");
                json_string(reason, &mut out);
            }
            // A flaky test carries the same `failures` array a failed one
            // does — the last failed attempt's — plus the count that says it
            // was retried at all. § 20's "never as green" is the `verdict`
            // above; this is what a reader does about it.
            Outcome::Failed(reasons)
            | Outcome::Flaky {
                failures: reasons, ..
            } => {
                if let Outcome::Flaky { attempts, .. } = &case.outcome {
                    out.push_str(&format!(", \"attempts\": {attempts}"));
                }
                out.push_str(", \"failures\": [");
                for (nth, reason) in reasons.iter().enumerate() {
                    if nth > 0 {
                        out.push_str(", ");
                    }
                    json_string(reason, &mut out);
                }
                out.push(']');
            }
            Outcome::Exited(code) => out.push_str(&format!(", \"exitCode\": {code}")),
        }
        out.push('}');
    }
    out.push_str(if cases.is_empty() {
        "]\n}\n"
    } else {
        "\n  ]\n}\n"
    });
    out
}

/// § 22's JUnit XML: the shape every CI system already ingests, over the same
/// [`Case`] list.
///
/// One `<testsuite>` per class, in the order the run took them, which is the
/// grouping every reader of this format expects and the one the run already
/// has — the roster is walked class by class, so a class's cases are
/// consecutive and no second pass is needed to collect them. A `skipped` test
/// is not a failure here either, matching both the exit code and § 20.
fn junit_document(cases: &[Case], counts: Counts, total: Duration) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<testsuites tests=\"{}\" failures=\"{}\" skipped=\"{}\" time=\"{:.6}\">\n",
        counts.total(),
        counts.failed,
        counts.skipped,
        total.as_secs_f64()
    ));
    let mut at = 0;
    while at < cases.len() {
        let class = &cases[at].class;
        let end = cases[at..]
            .iter()
            .position(|case| &case.class != class)
            .map_or(cases.len(), |offset| at + offset);
        let suite = &cases[at..end];
        let failures = suite
            .iter()
            .filter(|case| matches!(case.outcome, Outcome::Failed(_) | Outcome::Exited(_)))
            .count();
        let skipped = suite
            .iter()
            .filter(|case| matches!(case.outcome, Outcome::Skipped(_)))
            .count();
        let elapsed: Duration = suite.iter().map(|case| case.elapsed).sum();
        out.push_str("  <testsuite name=\"");
        xml_text(class, &mut out);
        out.push_str(&format!(
            "\" tests=\"{}\" failures=\"{failures}\" skipped=\"{skipped}\" time=\"{:.6}\">\n",
            suite.len(),
            elapsed.as_secs_f64()
        ));
        for case in suite {
            out.push_str("    <testcase name=\"");
            xml_text(&case.method, &mut out);
            out.push_str("\" classname=\"");
            xml_text(class, &mut out);
            out.push_str(&format!("\" time=\"{:.6}\"", case.elapsed.as_secs_f64()));
            match &case.outcome {
                Outcome::Passed => out.push_str("/>\n"),
                Outcome::Skipped(_) => {
                    out.push_str(">\n      <skipped message=\"");
                    xml_text(&case.outcome.message().unwrap_or_default(), &mut out);
                    out.push_str("\"/>\n    </testcase>\n");
                }
                // Jenkins' own spelling for exactly this: a `<flakyFailure>`
                // reports the attempt that failed while leaving the test
                // itself passing, so the suite's `failures` attribute does not
                // count it and no CI system reads the run as red. There is no
                // standard *attribute* for a flaky count, which is why the
                // element is where this shows up at all.
                Outcome::Flaky { attempts, failures } => {
                    out.push_str(">\n      <flakyFailure message=\"");
                    xml_text(&case.outcome.message().unwrap_or_default(), &mut out);
                    out.push_str(&format!("\" attempts=\"{attempts}\">"));
                    for reason in failures {
                        xml_text(reason, &mut out);
                        out.push('\n');
                    }
                    out.push_str("</flakyFailure>\n    </testcase>\n");
                }
                Outcome::Failed(_) | Outcome::Exited(_) => {
                    out.push_str(">\n      <failure message=\"");
                    xml_text(&case.outcome.message().unwrap_or_default(), &mut out);
                    out.push_str("\">");
                    // The body carries every failed assertion where the
                    // attribute carries the first: § 5's ledger keeps them all,
                    // and a format that reported one would hide what it exists
                    // to keep.
                    if let Outcome::Failed(reasons) = &case.outcome {
                        for reason in reasons {
                            xml_text(reason, &mut out);
                            out.push('\n');
                        }
                    }
                    out.push_str("</failure>\n    </testcase>\n");
                }
            }
        }
        out.push_str("  </testsuite>\n");
        at = end;
    }
    out.push_str("</testsuites>\n");
    out
}

/// A duration as the milliseconds every report quotes.
fn millis(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}

/// `text` as a JSON string literal, quotes included.
///
/// Written here rather than taken from a crate: this is the only JSON the CLI
/// produces, and `Core\Json` is a *runtime* member over runtime values, which
/// a `&str` held by the runner is not.
fn json_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Every other control character is spelled out; the rest, non-ASCII
            // included, is written as itself, the document being UTF-8.
            ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

/// `text` where XML admits character data, escaped for an attribute value and
/// an element body alike so one function serves both.
///
/// A control character other than tab, newline and carriage return is not
/// *representable* in XML 1.0 at all — not even as a numeric reference — so it
/// is replaced rather than escaped, which is the only lossless-looking option
/// this format leaves.
fn xml_text(text: &str, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(ch),
            ch if (ch as u32) < 0x20 => out.push('\u{fffd}'),
            ch => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Format, Outcome, compile, run_suite_in_a_task};

    /// A program under `tests/fixtures/runner/`, which `cargo test` does not
    /// run in — hence the manifest directory.
    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("runner")
            .join(name)
    }

    /// Runs one fixture's whole suite and answers with a `(method, verdict)`
    /// pair per reported case, plus whatever failed.
    ///
    /// [`Format::Json`] rather than the plaintext one because a test of the
    /// runner is not a test of a rendering: it collects rather than printing
    /// as it goes, and the program's own `echo` lands in a buffer nothing
    /// reads.
    fn verdicts(name: &str) -> Vec<(String, &'static str, Vec<String>)> {
        let checked = crate::front_end(&fixture(name)).expect("the fixture is a program");
        let unit = compile(&checked).expect("the fixture compiles");
        // Granting, because one fixture below spawns and ADR 0118 § 1 denies by
        // default — `crate::script::granting_ctx` owns why that helper exists.
        let mut ctx = crate::script::granting_ctx();
        unit.install_in(&mut ctx);
        // Through the same entry `run` takes, scheduler and all: a suite run
        // off a bare stack would be a different runner from the one shipped,
        // and § 16 is a claim about the one with a task tree under it.
        let (suite, _ctx) = run_suite_in_a_task(&unit, ctx, checked, Format::Json)
            .expect("the suite's own task runs");
        assert_eq!(
            std::rc::Rc::strong_count(&unit),
            1,
            "every isolate's program is dropped with it, so the code it shared \
             is left with one owner"
        );
        suite
            .cases
            .into_iter()
            .map(|case| {
                let failures = match &case.outcome {
                    Outcome::Failed(failures) => failures.clone(),
                    _ => Vec::new(),
                };
                (case.method, case.outcome.verdict(), failures)
            })
            .collect()
    }

    #[test]
    fn each_test_runs_in_its_own_isolate_sharing_only_compiled_code() {
        // ADR 0079 § 2 and ADR 0116 § 4: the two tests share the one compiled
        // unit and nothing else, so the second reads its static's *declared*
        // initial value however hard the first wrote to it. Asserted as a
        // verdict rather than as a number, because the fixture's own
        // assertions are what a failing runner would report — and the failure
        // messages come back with it, so a broken boundary says which side it
        // broke on.
        let verdicts = verdicts("statics-are-fresh.nvs");
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itWritesAStaticItsSiblingWillNotSee", "passed"),
                ("itReadsBackTheDeclaredInitialValue", "passed"),
            ],
            "failures: {:?}",
            verdicts
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_test_at_a_fixed_clock_reads_that_clock() {
        // ADR 0079 § 12. Three cases in one fixture because the claim is about
        // the isolate and not about the reading: the clock a test declared is
        // what `Core\Time::now` answers inside it, `Core\Test::advance` moves
        // that same reading, and the sibling that declared no `at:` still reads
        // the host's. A clock fixed anywhere above the isolate — on the
        // runner's own context, say — passes the first two and fails the third.
        let verdicts = verdicts("fixed-clock.nvs");
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itReadsTheClockItDeclared", "passed"),
                ("itReadsTheClockItAdvanced", "passed"),
                ("itReadsTheHostClockWithoutAnAt", "passed"),
            ],
            "failures: {:?}",
            verdicts
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_test_with_a_seed_draws_the_same_sequence_twice() {
        // ADR 0079 § 12's other half. Two tests declaring the same seed draw
        // the same three numbers, which is the promise; a third declaring a
        // different seed does not, so the assertion is about the seed and not
        // about a generator that answers one constant; and a fourth declaring
        // none still draws from the CSPRNG, which is what makes the seed the
        // *isolate's* rather than the process's. The fixture freezes seed 42's
        // answer, so changing `nvs_stdlib::random`'s step is meant to fail
        // here rather than silently rewrite what every seeded test expects.
        let verdicts = verdicts("seeded.nvs");
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itDrawsTheSequenceItsSeedNames", "passed"),
                ("itDrawsThatSameSequenceAgain", "passed"),
                ("itDrawsADifferentSequenceUnderADifferentSeed", "passed"),
                ("itDrawsFromTheCsprngWithoutASeed", "passed"),
            ],
            "failures: {:?}",
            verdicts
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_runner_owns_the_tests_task_tree() {
        // ADR 0079 § 16, both halves, from the language surface. The first
        // test's `Core\Task::all` has children only because the suite is
        // itself a task (ADR 0072 § 1's "a child of the calling task"), so a
        // runner that ran off a bare stack fails it rather than passing it
        // differently. The second leaves a `spawn script` unawaited: § 16 says
        // that is a failure and says it by name, where the scheduler on its own
        // would cancel the child at the retire and report green.
        let verdicts = verdicts("task-tree.nvs");
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itRunsInsideATaskOfTheRunners", "passed"),
                ("itLeavesAnUnawaitedChildRunning", "failed"),
            ],
            "failures: {:?}",
            verdicts
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
        let (_, _, left) = &verdicts[1];
        assert_eq!(
            left.len(),
            1,
            "the leftover task is the only thing reported: {left:?}"
        );
        assert!(
            left[0].starts_with("it left 1 task(s) still running when it returned"),
            "the failure names the tree it left behind: {left:?}"
        );
    }
}
