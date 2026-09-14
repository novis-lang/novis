//! `rule:testing/test-attribute`'s
//! runner: the `#[Test]` table a compile already built (§ 1), constructed and
//! called (§ 20), judged off § 5's ledger — [`run_case`] owns why that and not
//! the exception state — and reported in one of § 22's formats.
//!
//! # One verdict, every rendering
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
//! there is no flag to change it (`rule:statements/an-isolate-has-its-own-statics`'s fresh statics base, armed by
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
//! child's program captured, because `rule:security/isolate-shares-nothing`'s `Completion` carries a value,
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
//! `rule:concurrency/nothing-is-still-running-when-a-call-returns` makes that bound reachable rather than aspirational — `::all`
//! and `::map` return with nothing still running — so the only way to fail
//! this is a `spawn script` the test never awaited.
//!
//! # Known gaps
//!
//! 1. **§ 2's parallelism is not built: the isolates are made and joined one
//!    at a time.** `docs/decisions/0079.md:158` makes an isolate per test
//!    *and* the suite parallel, and that record's milestone table carries
//!    both to M5 (`docs/decisions/0079.md:872`). Only the second half is
//!    open, and it is a scheduling question rather than an isolation one:
//!    every test already runs in an isolate of its own.
//!    — owner: unowned
//!
//! # What a constructor, a fixture and class order do
//!
//! A constructor that declares parameters is
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

/// § 22's report formats.
///
/// The roster lives here rather than in `main`, because what a format *is* is
/// this module's question and `main` only has to name one on the command line.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Format {
    /// The default: one line per test as the run goes, then the summary.
    Human,
    /// A versioned JSON document, written to stdout at the end.
    Json,
    /// JUnit XML, the shape every CI system already ingests.
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
    /// than matching the outcome a second time, which is what makes "no two of
    /// them disagree about a verdict" a property of the code rather than of a
    /// review.
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
    /// A flaky test gains the line like any other — a test that leaks a task is
    /// not one retrying made honest — and which outcomes are left alone is
    /// [`Self::and_failed`]'s.
    fn with_tasks_left_running(self, running: usize) -> Self {
        if running == 0 {
            return self;
        }
        let named = format!(
            "it left {running} task(s) still running when it returned: `rule:testing/task-tree-and-virtual-clock` \
             fails a test whose task tree outlives it — `Core\\Task::all` and `::map` \
             return with nothing still running, and a `spawn script` is finished by `await`"
        );
        self.and_failed(named)
    }

    /// This outcome with one more failure folded into it — what the runner
    /// itself found wrong *around* the test, rather than what the test's own
    /// ledger recorded.
    ///
    /// [`Self::with_tasks_left_running`] is § 16's caller and this is § 17's,
    /// which is why the fold is here once rather than at each: a verdict a
    /// report could call green becomes a failure, a failure gains a line, and a
    /// skip or an `exit(n)` is untouched — a test that never ran cannot have
    /// left a transaction open, and an `exit(n)` ended the program rather than
    /// the test.
    fn and_failed(self, named: String) -> Self {
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
    /// Where the method carrying `#[Test]` is declared, resolved while the run
    /// still holds the source map.
    ///
    /// It is resolved here rather than rendered from a [`Span`](nvs_diagnostics::Span)
    /// at the end, because `run_suite_in_a_task` **moves** the `Checked` into
    /// the suite's own task: by the time a document is written the map that
    /// turns a byte offset into a line is gone. `None` only for a span the map
    /// does not hold, which a test the compiler collected cannot have.
    declared: Option<Declared>,
    outcome: Outcome,
    elapsed: Duration,
    /// § 14's inline snapshots this test produced a different rendering for,
    /// carried out of the isolate beside the verdict because that is the only
    /// place they exist — the child's context is gone by the time `--update`
    /// runs. Always collected and never reported: what reads it is
    /// [`update_snapshots`], and only when the run was asked to.
    snapshots: Vec<nvs_runtime::SnapshotMismatch>,
}

/// Where one test is written: the file the `-->` header names, and a one-based
/// line and column into it.
///
/// The same three fields `nvs check --json` carries for a diagnostic's span
/// (`rule:ide/check-json-is-the-diagnostic-record-as-a-document`), spelled the
/// same way and counted the same way, so an editor that can open one location
/// can open the other. What points at the test is the method's **name**, which
/// is what every refusal about that test already points at.
struct Declared {
    file: String,
    line: usize,
    column: usize,
}

/// A `#[Test]` row's own span, as the location a report can print.
///
/// The column counts `char`s rather than bytes, which is
/// `nvs_diagnostics::SourceFile::line_col`'s decision and the one the text
/// renderer is already rendered from — two answers to "which column" is the
/// one thing a second location renderer must not introduce.
fn declared_at(span: nvs_diagnostics::Span, map: &nvs_diagnostics::SourceMap) -> Option<Declared> {
    let file = map.get(span.file)?;
    let (line, column) = file.line_col(span.start);
    Some(Declared {
        file: file.name().to_owned(),
        line: line + 1,
        column: column + 1,
    })
}

/// Compiles `checked` and runs every `#[Test]` it declares that `filter`
/// selects, reporting in `format`.
///
/// `update` is `rule:testing/inline-snapshots`'s `nvs test --update`: after the suite, and only
/// then, each failed snapshot is spliced back into the source that wrote it.
/// The verdicts are the same either way — a run that rewrote a snapshot still
/// reports the test that produced it as failed, because it did, and the
/// re-run is what says the new snapshot is the one the author meant.
pub(crate) fn run(
    checked: crate::Checked,
    snapshot: &nvs_config::Snapshot,
    format: Format,
    filter: Option<String>,
    update: bool,
    list: bool,
) -> ExitCode {
    // Answered before the compile below, rather than beside the run: what
    // discovery reads is the checked program's own test table, so a listing
    // costs no lowering and can say nothing about how a test would come out.
    if list {
        let listed = listing(&checked, filter.as_deref());
        return match format {
            Format::Human => {
                for test in &listed {
                    match &test.declared {
                        Some(declared) => println!(
                            "{}::{}  {}:{}",
                            test.class, test.method, declared.file, declared.line
                        ),
                        None => println!("{}::{}", test.class, test.method),
                    }
                }
                ExitCode::SUCCESS
            }
            Format::Json => {
                print!("{}", listing_document(&listed));
                ExitCode::SUCCESS
            }
            // Refused rather than ignored, for `--update`'s reason in `main`:
            // every element JUnit has is about how a test *came out*, so a
            // listing in that format would be a run nobody performed.
            Format::Junit => {
                eprintln!("error: `--list` has no JUnit document: JUnit reports a run's results");
                ExitCode::FAILURE
            }
        };
    }
    // `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` and `rule:config/opcache-file-cache-directives-are-system`: the suite's unit comes off disk when this
    // environment has an artifact for this program, and the key is the
    // program's, not the subcommand's — a `nvs run` and a `nvs test` of one
    // program lower the same `nvs_ir::Program` through the same entry label, so
    // they share an artifact and whichever ran first pays for it.
    let cache = crate::cache::from_config(&snapshot.config);
    let unit = match compile(&checked, cache.as_ref()) {
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

    // Read out before `checked` is moved into the suite's task, which is where
    // it is dropped: the rows are the compiler's and the run cannot produce
    // them. Nothing is read at all for a run that was not asked to update, so
    // an ordinary suite pays one `bool`.
    let sites = match update {
        true => snapshot_sites(&checked),
        false => Vec::new(),
    };
    let started = Instant::now();
    let (suite, mut ctx) = match run_suite_in_a_task(
        &unit,
        ctx,
        checked,
        &snapshot.config,
        format,
        filter.as_deref(),
    ) {
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
    // § 14's splice, after every test has run and before the summary is
    // printed, so a reader sees what was rewritten above the counts that made
    // it necessary. It is the one thing `nvs test` writes to a file, and it
    // does not touch the verdicts above.
    if update {
        update_snapshots(&sites, &cases);
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
/// `rule:testing/task-tree-and-virtual-clock`'s "the runner owns the test's task tree", and the same three
/// installations `nvs run` makes (`crate::main`, § *What `run` executes*).
///
/// Without it there is no task beneath a test at all: `nvs_host::Wake::current`
/// answers `None`, so a test's own isolate runs on this stack rather than as a
/// child (`nvs_host::isolate::Isolate::start`), a `Core\Task::all` in a test
/// has no calling task to put children under (`rule:concurrency/a-child-belongs-to-the-calling-task`), and § 16 has no
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
    config: &nvs_config::Config,
    format: Format,
    filter: Option<&str>,
) -> Result<(Suite, nvs_runtime::Ctx), String> {
    let mut sched = nvs_host::Scheduler::new();
    // Built before `checked` is moved into the task below, which is the only
    // reason it is here rather than beside the resolver: both are installed
    // over the same `run_until_idle`.
    let under_test = UnderTest::new(unit, &checked);
    let filed: Rc<RefCell<Option<Suite>>> = Rc::new(RefCell::new(None));
    let collected = Rc::clone(&filed);
    let suite_unit = Rc::clone(unit);
    // Owned for the same reason `checked` is: the body outlives this call as
    // far as the scheduler's signature is concerned.
    let filter = filter.map(ToOwned::to_owned);
    let root = sched.spawn(ctx, nvs_runtime::TaskRoot::Request, move |ctx| {
        *collected.borrow_mut() = Some(run_suite(
            &suite_unit,
            ctx,
            &checked,
            format,
            filter.as_deref(),
        ));
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
    // the compiled unit (`crate::script`). It reads the tree's own `[opcache]`,
    // because the snapshot is resolved above the suite's compile and
    // `rule:config/the-extension-set-is-in-every-unit-key`'s environment digest is half of every key this resolver
    // writes: a default one would file a unit under an environment this run is
    // not in. Nothing edits a file mid-suite, so the revalidation policy in it
    // chooses nothing.
    let compiler = crate::script::Compiler::new(config);
    // And `rule:testing/in-process-request`'s unit under test, over the same run: a `Core\Test`
    // member reaches it the way a `spawn script` reaches the resolver, so the
    // two guards nest rather than either one being a special case.
    let ran = nvs_runtime::script::scoped(&compiler, || {
        nvs_runtime::inproc::scoped(&under_test, || nvs_host::run_until_idle(&mut sched))
    });
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
        // `rule:http-server/containment-does-not-end-at-the-helper`'s boundary contained a panic under the task root. For a
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

/// The program `Core\Test::request` answers a synthetic request with — ADR
/// 0079 § 18, and the far side of [`nvs_runtime::inproc::Answering`].
///
/// It lives here rather than in `nvs-stdlib` for that seam's stated reason: a
/// `Core` member may not hold a compiled unit, `nvs-codegen` being above the
/// runtime, and this binary is the one place a checked program and a runtime
/// context are in the same scope. It is also why the *test runner* owns it
/// rather than `serve`: what a `#[Test]` method asks for is the program under
/// test, which is a unit this run compiled, where a served request's program is
/// whichever entry the mount table selected.
pub(crate) struct UnderTest {
    /// The unit the suite is running, whose script frame is the entry a
    /// synthetic request runs — the same frame a served request would run,
    /// which is what makes § 18's "the real chain" true rather than a mock.
    unit: Rc<nvs_codegen::Unit>,
    /// `rule:routing/matched-once-before-the-handler`'s table, installed on the child so that a handler reading
    /// its own route reads the same one the door matched against.
    routes: std::sync::Arc<nvs_runtime::routes::Routes>,
}

impl std::fmt::Debug for UnderTest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The unit has no rendering and would not be worth one here: what a
        // reader of a seam's `Debug` wants is which program is installed, and
        // the route count is the only thing about it that distinguishes two.
        formatter
            .debug_struct("UnderTest")
            .field("routes", &self.routes.rows().len())
            .finish_non_exhaustive()
    }
}

impl UnderTest {
    /// The unit under test for this run, with `rule:routing/matched-once-before-the-handler`'s table already
    /// crossed into the runtime's shape.
    pub(crate) fn new(unit: &Rc<nvs_codegen::Unit>, checked: &crate::Checked) -> Self {
        Self {
            unit: Rc::clone(unit),
            routes: std::sync::Arc::new(crate::runtime_routes(checked.exprs.routes())),
        }
    }
}

impl nvs_runtime::inproc::Answering for UnderTest {
    fn answer(
        &self,
        ctx: &mut nvs_runtime::Ctx,
        mut inbound: Box<nvs_runtime::Inbound>,
    ) -> Result<nvs_runtime::host::Completion, String> {
        // `rule:routing/matched-once-before-the-handler`'s one match, here because this is the side holding the
        // table and the last point before application code exists to have run.
        // A program with no `#[Route]` has an empty table and claims nothing,
        // which is `rule:routing/table-is-opt-in`'s opt-in rule and leaves the route `null`.
        if let Some(matched) = self.routes.match_request(inbound.method(), inbound.path()) {
            inbound.set_route(matched);
        }
        let unit = Rc::clone(&self.unit);
        let routes = std::sync::Arc::clone(&self.routes);
        // The same program `crate::script::program_over` builds for a `spawn
        // script`, over this run's own unit instead of a resolved one: the
        // child's statics and its error class are armed from inside, because
        // `rule:security/isolate-shares-nothing`'s isolate shares compiled code and nothing else.
        let program: nvs_runtime::script::Program =
            Box::new(move |ctx: &mut nvs_runtime::Ctx, _args| {
                unit.install_in(ctx);
                if !routes.rows().is_empty() {
                    ctx.set_routes(std::sync::Arc::clone(&routes));
                }
                let Some(entry) = unit.script() else {
                    // Not reachable for a unit that compiled, and a failure
                    // value rather than a panic for `program_over`'s reason: a
                    // child may not end its parent.
                    ctx.set_pending("the program under test has no script frame");
                    return nvs_runtime::Value::null();
                };
                entry
                    .call(ctx)
                    .unwrap_or_else(|_| nvs_runtime::Value::null())
            });
        // `Isolate` and not `Host::start_isolate`: the seam's operation takes no
        // request, and the request is exactly what decides the child's sink —
        // an isolate answering one writes to a response body under `rule:tooling/echo-always-has-a-sink`
        // , and a `Core` member reaching the host through the trait could
        // not have said so. `nvs_host::Isolate::answering` is the one spelling
        // of that, and it is this crate's to reach.
        let running = nvs_host::Isolate::new(
            program,
            nvs_runtime::Value::null(),
            nvs_runtime::host::Output::Capture,
        )
        .answering(*inbound)
        .start(ctx)
        .map_err(|error| error.to_string())?;
        Ok(running.join(ctx))
    }
}

/// Lowers and compiles `checked` into the **one** unit every test isolate of
/// this run shares — `rule:testing/isolate-per-test`'s "shares compiled code with its siblings"
/// is this `Rc` and the program closure each child holds a clone of.
///
/// `Err` is the message to render; a compile that fails ends the run rather
/// than any one test.
///
/// `cache` is `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact cache, or [`None`] for a run that consults
/// none — every way it can fail to answer is a cold compile and nothing a
/// verdict can see, which is why the [`crate::cache::Provenance`] this drops is
/// dropped rather than reported.
fn compile(
    checked: &crate::Checked,
    cache: Option<&crate::cache::Cache>,
) -> Result<Rc<nvs_codegen::Unit>, String> {
    let files = checked.program_files();
    let program = nvs_ir::lower::lower_program(
        nvs_ir::lower::ENTRY_SCRIPT_LABEL,
        &files,
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    );
    crate::cache::unit_for(&program, crate::cache::program_digest(&files), cache)
        .map(|(unit, _)| Rc::new(unit))
        .map_err(|error| error.to_string())
}

/// A whole run's verdict, before any of § 22's renderings has been chosen —
/// which is also the shape this module's own tests read, a rendering being the
/// one thing they are not about.
struct Suite {
    cases: Vec<Case>,
    counts: Counts,
    /// The code an `exit(n)` in a test or a fixture ended the run with.
    exited: Option<i64>,
}

/// Runs every `#[Test]` `checked` declares that `filter` selects, in § 20's
/// order, reporting each as it arrives under [`Format::Human`] and only
/// collecting under the other two.
fn run_suite(
    unit: &Rc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    checked: &crate::Checked,
    format: Format,
    filter: Option<&str>,
) -> Suite {
    let (mut passed, mut failed, mut skipped, mut flaky) = (0_usize, 0_usize, 0_usize, 0_usize);
    let mut exited = None;
    let mut cases = Vec::new();
    // `rule:routing/matched-once-before-the-handler`'s table, crossed once for the whole suite rather than per
    // test: it is a product of the compile every isolate already shares, and
    // [`TestServer`] is the only thing that reads it — a run whose tests declare
    // no `server:` pays one walk of the program's `#[Route]` rows and nothing
    // else. `UnderTest` builds its own for § 18's first mechanism, which is
    // installed a call above this one and holds it for the whole run.
    let routes = std::sync::Arc::new(crate::runtime_routes(checked.exprs.routes()));
    for class in checked.exprs.test_classes() {
        let tests = checked.exprs.tests(class).unwrap_or_default();
        // The selection is made **before** the class is announced or its
        // fixtures are built: a `--filter` that reaches none of a class's
        // tests must cost that class's `#[Fixture]` nothing, or filtering down
        // to one fast test still pays for every expensive setup in the
        // program. A class with nothing selected is not reported either — the
        // header would name a class the run said nothing about.
        let calls: Vec<Invocation<'_>> = tests
            .iter()
            .flat_map(invocations)
            .filter(|call| selected(filter, class, &call.label))
            .collect();
        if calls.is_empty() {
            continue;
        }
        if format == Format::Human {
            println!("  {class}");
        }
        // § 8's "built once, in the parent" is this call: one set per class,
        // built before its first test and dropped after its last, so a fixture
        // is not rebuilt per test and not kept past the class that declared it.
        let mut fixtures = nvs_runtime::Fixtures::new();
        let unbuilt = build_fixtures(unit, ctx, class, checked, tests, &mut fixtures);
        for call in calls {
            let (case, label, row) = (call.case, call.label, call.row);
            let began = Instant::now();
            let (outcome, snapshots) = match &unbuilt {
                // A fixture that would not build is reported against every
                // test that asked for one, rather than against the class: a
                // test is what a report has a line for, and a suite that lost
                // a whole class silently is what § 20 is written against.
                Some(FixtureFailure::Exited(code)) => (Outcome::Exited(*code), Vec::new()),
                Some(FixtureFailure::Threw(message)) if !fixtures_needed(case).is_empty() => {
                    (Outcome::Failed(vec![message.clone()]), Vec::new())
                }
                _ => run_in_isolate(unit, ctx, class, case, row, &fixtures, &routes),
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
                declared: declared_at(case.span, &checked.map),
                outcome,
                elapsed,
                snapshots,
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
/// whole program rather than the test (`rule:errors/escalation-ladder`), so neither is an outcome a
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

/// `rule:testing/fixtures`'s fixtures for one class, built **once** and in dependency
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

/// `rule:testing/determinism-declared-on-the-test`'s `at:` option — the instant this test's isolate reads its
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

/// `rule:testing/determinism-declared-on-the-test`'s `seed:` option — the seed this test's isolate puts its
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
/// **The verdict is read off `rule:testing/failure-ledger`'s ledger, not off the exception
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
    routes: &std::sync::Arc<nvs_runtime::routes::Routes>,
) -> (Outcome, Vec<nvs_runtime::SnapshotMismatch>) {
    // § 14's mismatches cross the boundary the verdict crosses, in a cell of
    // their own beside it: they are the child's context's and that context is
    // gone by the time this returns, and they are not part of the verdict —
    // nothing about a test's outcome depends on whether it was going to be
    // rewritten. The seam is one cell rather than a second field on [`Outcome`]
    // so that every early exit below, none of which ran a line of the test,
    // hands back an empty list without saying so.
    let snapshots: Rc<RefCell<Vec<nvs_runtime::SnapshotMismatch>>> =
        Rc::new(RefCell::new(Vec::new()));
    let outcome = run_the_test(unit, ctx, class, case, row, fixtures, routes, &snapshots);
    let taken = std::mem::take(&mut *snapshots.borrow_mut());
    (outcome, taken)
}

/// [`run_in_isolate`]'s body, with the cell it fills passed in — everything
/// this module's docs say about the isolate is about this function.
#[expect(
    clippy::too_many_arguments,
    reason = "the caller's seven, plus the one cell § 14's mismatches cross in"
)]
fn run_the_test(
    unit: &Rc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    case: &nvs_types::testing::TestCase,
    row: Option<&[Option<ConstArg>]>,
    fixtures: &nvs_runtime::Fixtures,
    routes: &std::sync::Arc<nvs_runtime::routes::Routes>,
    snapshots: &Rc<RefCell<Vec<nvs_runtime::SnapshotMismatch>>>,
) -> Outcome {
    if let Some(reason) = skip_reason(case) {
        return Outcome::Skipped(reason);
    }
    // § 18's second mechanism, bound **before** the child exists for the same
    // reason § 9's row and § 8's copies are: what the test observes of it is one
    // string on its own context, and a socket cannot be handed across the heap
    // boundary afterwards. A failure to bind is this test's own failure and
    // nothing has run.
    let served = match wants_server(case) {
        false => None,
        true => match TestServer::bind(unit, routes) {
            Ok(server) => Some(server),
            Err(refused) => {
                return Outcome::Failed(vec![format!(
                    "`{class}::{}` declares `server: true`, and no listener could be bound for \
                     it: {refused}",
                    case.method
                )]);
            }
        },
    };
    let listening = served.as_ref().map(|server| server.url.clone());
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
    let recorded = Rc::clone(snapshots);
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
    // § 17's block name, read on the parent's side with every other option and
    // owned because the closure outlives this frame — but *opened* inside the
    // child, which is the one decision this mechanism makes. A connection is
    // memoized on the context it was opened on, so a transaction begun out here
    // would be on a connection the test's own `Core\Db::connect` could never
    // reach, and § 17's "a transaction opened inside the test is a savepoint"
    // would be two transactions on two connections deadlocking on each other.
    // `nvs_stdlib::db::begin_test_transaction` is the one home of that reading.
    let transacted = wants_db(case).map(str::to_owned);
    let child_unit = Rc::clone(unit);
    let class_name = class.to_owned();
    let method = case.method.clone();
    let program: nvs_runtime::script::Program = Box::new(move |child, argument| {
        // `rule:statements/an-isolate-has-its-own-statics`'s fresh statics base, armed with *this* unit's tables:
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
        // § 18's listener, armed the same way and in the same place as the two
        // above: the address is the *child's* to read, so a sibling that
        // declared no `server:` answers `null` and cannot reach a port this
        // test bound (`nvs_runtime::Ctx::test_server`).
        if let Some(url) = listening {
            child.set_test_server(url);
        }
        // Null here, and discharged anyway: the seam's contract is that
        // whatever crossed becomes the isolate's own root's, and a test's
        // values crossed as § 8's copies instead.
        child.set_isolate_argument(argument);
        // Both owners go down with the isolate, after the call that borrowed
        // their values has returned.
        let (_crossed, _materialized) = (crossed, materialized);
        let outcome = match transacted.as_deref() {
            None => run_with_retries(&child_unit, child, &class_name, &method, &args, allowance),
            // § 17, around the whole allowance rather than around one attempt:
            // `retries:` exists for a test that is flaky against something
            // outside the database, and a retry that started from a pristine
            // database would be a second rule about what a retry re-runs —
            // § 20 says only that the method is called again. A test whose
            // second attempt has to unsee its first attempt's writes has
            // written a test the rollback cannot fix.
            Some(name) => match nvs_stdlib::db::begin_test_transaction(child, name) {
                Err(refused) => Outcome::Failed(vec![format!(
                    "`{class_name}::{method}` declares `db: \"{name}\"`, and no transaction \
                     could be opened on it: {refused}"
                )]),
                Ok(key) => {
                    let outcome = run_with_retries(
                        &child_unit,
                        child,
                        &class_name,
                        &method,
                        &args,
                        allowance,
                    );
                    match nvs_stdlib::db::roll_back_test_transaction(child, key) {
                        Ok(()) => outcome,
                        // Reported and never swallowed: a rollback that did not
                        // happen is a test that left rows behind, and the next
                        // test reading them would fail somewhere with nothing
                        // to say about why.
                        Err(refused) => outcome.and_failed(format!(
                            "`{class_name}::{method}` declares `db: \"{name}\"`, and its \
                             transaction could not be rolled back: {refused}"
                        )),
                    }
                }
            },
        };
        // § 16, read off the tree from inside the test's own task and nowhere
        // else: a child spawned here is a child of *this* task, and once this
        // closure returns the scheduler cancels whatever is left rather than
        // reporting it. `nvs_host::children_still_running` owns why an awaited
        // child is already gone from the count.
        // § 14's mismatches, taken on the child's own stack for the reason its
        // ledger is taken there: the record is this test's, and this is the
        // last frame that holds the context carrying it. Every attempt § 20
        // allowed is in the list, which is why [`update_snapshots`] reads only
        // a test that ended failed — a retried test that went on to pass has
        // the snapshot its last attempt matched.
        *recorded.borrow_mut() = child.take_snapshot_mismatches();
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
    // § 2's boundary is around the whole test, and a listener is the one thing
    // it owns that the child's context does not release for it: the socket was
    // bound out here, so it is retired out here, after the isolate that could
    // still have been talking to it has joined.
    if let Some(server) = served {
        server.stop();
    }
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

/// Whether this case asked for `rule:testing/in-process-request`'s second mechanism.
///
/// `server` is a `bool` in `nvs_types::testing`'s roster, so anything else has
/// already been refused while compiling and reading it back as absent here
/// would be a second answer to a settled question — the same reading
/// [`skip_reason`] takes one option over. A written `server: false` is the same
/// as an unwritten one: the option says whether a listener is wanted, and
/// nothing downstream distinguishes "no" from "did not ask".
fn wants_server(case: &nvs_types::testing::TestCase) -> bool {
    case.options
        .iter()
        .any(|(name, value)| name == "server" && matches!(value, ConstArg::Bool(true)))
}

/// The `[db.<name>]` block this case asked to run inside a transaction of —
/// `rule:testing/db-transaction` — or `None` for a test that names none.
///
/// `db` is a `string` in `nvs_types::testing`'s roster, so a non-text value has
/// already been refused while compiling and reading it back as absent here
/// would be a second answer to a settled question — [`wants_server`]'s reading
/// of its own option, over a different type. An empty name is *not* special
/// cased: `[db.]` is a block nobody can write, so it reaches the same "nothing
/// sets that up" refusal every other unwritten name does, worded about the name
/// the test actually declared.
fn wants_db(case: &nvs_types::testing::TestCase) -> Option<&str> {
    case.options.iter().find_map(|(name, value)| match value {
        ConstArg::Str(block) if name == "db" => Some(block.as_str()),
        _ => None,
    })
}

/// `rule:testing/in-process-request`'s second mechanism, bound: one ephemeral listener over the
/// program under test, and the task accepting on it.
///
/// **Why a second mechanism at all**, when § 18's first one already runs a
/// request through the same entry with no socket: what this one answers is
/// everything between the two — the `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` header set actually arriving on
/// the wire, chunking, keep-alive, a peer that hangs up mid-body. The in-process
/// path deliberately never builds a response head, so none of that is a question
/// it *can* be asked, which is the ADR's "measurably different questions".
///
/// **The port is the operating system's**, taken by binding `127.0.0.1:0` and
/// reading the address back: a fixed port would collide between two `cargo test`
/// processes on one machine and between two tests of one suite, and there is
/// nothing for a developer to configure because there is nothing for them to
/// choose. Loopback and not `0.0.0.0` for the reason that needs no measurement —
/// a test suite that binds a routable address is a test suite that serves a
/// program under test to the network.
///
/// **The handler holds a [`std::rc::Weak`] of the unit, never an [`Rc`].** The
/// accept loop parks on a socket nothing will connect to again once the test is
/// over, and a parked task's closure is dropped when the scheduler reaps it
/// rather than when [`Self::stop`] asks — so a strong handle here would keep the
/// whole compiled program alive on a schedule this function does not control.
/// A request arriving after the test that owns the program has ended finds no
/// program, which is the same nothing-claims-this a table's own `404` is.
///
struct TestServer {
    /// What the test reads through `Core\Test::serverUrl()` — `http://` and the
    /// bound address, with no trailing slash so a path appends directly.
    url: String,
    /// The accept loop's own task, which is a **sibling** of the test's isolate
    /// and a child of the suite's task. Sibling and not child on purpose: § 16
    /// reads `nvs_host::children_still_running` from inside the test's own task,
    /// so a server underneath it would be reported as work the test left behind.
    /// `None` where the scheduler refused the spawn, which leaves a bound socket
    /// nothing accepts on — a request against it then times out rather than
    /// silently passing, and [`Self::bind`] reports the refusal instead.
    task: Option<nvs_host::TaskId>,
}

impl TestServer {
    /// Binds the listener and starts accepting on it.
    ///
    /// Every policy this loop serves under is the **default** one rather than
    /// the tree's: a `#[Test(server: true)]` is a claim about what the runtime
    /// emits with nothing configured (`rule:http-server/secure-headers-with-nothing-written`'s set, `rule:http-server/cors-is-closed-until-origins-are-named`'s closed
    /// CORS, `rule:http-server/the-server-block-is-boot-class`'s waits), and reading a deployment's `nvs.toml` here
    /// would make the test's subject the deployment. `Trusted::of(&[])` is the
    /// same direction stated once more — no forwarded header is read, so the
    /// peer is the peer.
    ///
    /// # Errors
    ///
    /// The message to report against the test: the address could not be bound,
    /// its own name could not be read back, or there was no task to spawn the
    /// accept loop onto.
    fn bind(
        unit: &Rc<nvs_codegen::Unit>,
        routes: &std::sync::Arc<nvs_runtime::routes::Routes>,
    ) -> Result<Self, String> {
        let wanted = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0));
        let mut listener = nvs_host::NvsListener::bind(wanted)
            .map_err(|error| format!("could not listen on {wanted}: {error}"))?;
        let bound = listener
            .local_addr()
            .map_err(|error| format!("the listener bound no readable address: {error}"))?;
        let held = Rc::downgrade(unit);
        let carried = std::sync::Arc::clone(routes);
        let handler = Rc::new(
            move |request: nvs_server::Request<nvs_server::Incoming>,
                  origin: nvs_server::Origin| {
                answer_on_the_wire(&held, &carried, request, origin)
            },
        );
        let serving = nvs_server::Serving::new(
            std::sync::Arc::new(nvs_server::Admission::new(&nvs_server::Ceiling::of(
                &nvs_config::server::Capacity {
                    configured: u64::MAX,
                    per_request: None,
                    budget: None,
                },
            ))),
            std::sync::Arc::new(nvs_server::Secure::of(None)),
            std::sync::Arc::new(nvs_server::Trusted::of(&[]).0),
            std::sync::Arc::new(nvs_server::Cors::of(None)),
            // The tree, default for the reason every policy above it is: what a
            // `#[Test(server: true)]` asserts is what the runtime does with
            // nothing configured, and a request reading the deployment's
            // `nvs.toml` here would make that deployment the test's subject.
            std::sync::Arc::default(),
        );
        // A `Draining` of this server's own and deliberately not the process's:
        // `nvs test` is one process running many listeners one after another,
        // and a shared bit would be a `503` one test could leave armed for the
        // next.
        let draining = nvs_server::Draining::detached();
        let waits = nvs_config::server::Waits::default();
        let task = nvs_host::spawn_child(
            nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink),
            nvs_runtime::TaskRoot::Request,
            move |_ctx| {
                // `ControlFlow::Continue` forever, and the loop is ended by
                // `Self::stop` cancelling this task instead: the flag is only
                // read between connections, so a server whose test never
                // connected would never look at one.
                let served = nvs_server::serve_on_this_core(
                    &mut listener,
                    &handler,
                    waits,
                    &serving,
                    &draining,
                    // Swallowed rather than printed: a note about a connection
                    // is not a verdict about a test, and stdout under
                    // `Format::Json` is the document.
                    |_note| {},
                    || std::ops::ControlFlow::Continue(()),
                );
                drop(served);
            },
        );
        if task.is_none() {
            return Err(
                "there is no task to accept on — the runner's own scheduler is gone".into(),
            );
        }
        Ok(Self {
            url: format!("http://{bound}"),
            task,
        })
    }

    /// Retires the accept loop once the test that asked for it has joined.
    ///
    /// Cancellation and not a flag, for the reason [`Self::bind`] states: the
    /// loop is parked in `accept` and nothing will connect again, so the only
    /// thing that reaches it is the scheduler. What the cancel buys is the
    /// listener's file descriptor going back at the next scheduler pass rather
    /// than at the end of the run — a suite of a hundred `server: true` tests
    /// would otherwise hold a hundred sockets.
    fn stop(self) {
        if let Some(task) = self.task {
            nvs_host::cancel_task(task);
        }
    }
}

/// One request off the wire, answered by the program under test — the same
/// shape `UnderTest::answer` builds for § 18's first mechanism, over a
/// carrier `hyper` filled instead of one a `Core` member wrote.
///
/// It is a free function rather than a closure body so that the two halves read
/// side by side: everything about *which* program answers is the same, and
/// everything about *what arrived* is the door's.
fn answer_on_the_wire(
    held: &std::rc::Weak<nvs_codegen::Unit>,
    routes: &std::sync::Arc<nvs_runtime::routes::Routes>,
    request: nvs_server::Request<nvs_server::Incoming>,
    origin: nvs_server::Origin,
) -> nvs_server::Reply {
    // Ahead of everything, and asked of `nvs_stdlib::request` for `crate::serve`'s
    // reason: the roster of verbs has one home and the door does not keep a list.
    if !nvs_stdlib::request::is_known_verb(request.method().as_str()) {
        return nvs_server::Reply::not_implemented();
    }
    let Some(unit) = held.upgrade() else {
        // The test that owned this program has ended. There is no program to
        // run and so nothing claims the path — `Self::bind`'s doc owns why the
        // handle is weak in the first place.
        return nvs_server::Reply::not_found();
    };
    let mut inbound = nvs_runtime::Inbound::new(
        request.method().as_str(),
        request.uri().path(),
        request.uri().query().unwrap_or(""),
    );
    for (name, value) in request.headers() {
        inbound.push_header(name.as_str(), value.as_bytes());
    }
    inbound.set_peer(origin.client(), origin.scheme());
    nvs_server::trace::take(&mut inbound);
    // `rule:routing/matched-once-before-the-handler`'s one match, taken here for the reason `crate::serve` takes
    // it here: the request and the unit that will answer it are both in hand,
    // and no application code has run.
    nvs_server::route::take(routes, &mut inbound);
    let (head, incoming) = request.into_parts();
    let supply = match nvs_server::body::of(&head.headers, incoming) {
        nvs_server::Arrived::Absent => None,
        nvs_server::Arrived::TooLarge => return nvs_server::Reply::too_large(),
        nvs_server::Arrived::Streaming(supply, pull) => {
            inbound.set_body(pull);
            Some(supply)
        }
    };
    let carried = std::sync::Arc::clone(routes);
    let program: nvs_runtime::script::Program =
        Box::new(move |ctx: &mut nvs_runtime::Ctx, _args| {
            unit.install_in(ctx);
            if !carried.rows().is_empty() {
                ctx.set_routes(std::sync::Arc::clone(&carried));
            }
            let Some(entry) = unit.script() else {
                ctx.set_pending("the program under test has no script frame");
                return nvs_runtime::Value::null();
            };
            entry
                .call(ctx)
                .unwrap_or_else(|_| nvs_runtime::Value::null())
        });
    nvs_server::Reply::Run(
        nvs_host::Isolate::new(
            program,
            nvs_runtime::Value::null(),
            nvs_runtime::host::Output::Capture,
        )
        .answering(inbound),
        supply,
    )
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
            "it asserted nothing: `rule:testing/runner-is-strict` fails a test whose ledger is empty — write \
             `Core\\Test::assertDoesNotThrow` where the claim is that a call completes"
                .to_owned(),
        ]);
    }
    Outcome::Passed
}

/// The calls one `#[Test]` method makes — `rule:testing/data-rows`'s "each row is its own
/// reported case".
///
/// A method with no `#[TestWith]` row is § 1's single call under its own name.
/// A method with rows is one call per row, in the source order the checker
/// recorded them, each labelled `method#N` — a **label** rather than a field
/// beside the name, so the plaintext mark, the JSON object and the JUnit
/// element tell two rows apart without any of them growing a rendering of its
/// own, exactly as they read one verdict rather than deciding one each.
///
/// Whether `--filter` selects one test — **the same rule the `.nvst` tree's
/// own filter uses, asked of the other suite's names.**
///
/// `nvs_test::run` keeps a case whose label *contains* the filter, that label
/// being the case's path; a `#[Test]` method's label is `Class::method`
/// (`Class::method#row` for a data-provider row, which is the name the report
/// prints), so the same containment answers both. Case-sensitive, for the
/// reason it is case-sensitive over a path: one rule that holds for `nvs test
/// tests/` and `nvs test app.nvs` alike is worth more than a friendlier one
/// that holds for only half of the subcommand. No filter selects everything —
/// running the whole suite is what `nvs test` without the flag means.
///
/// Matching the class as well as the method is what makes `--filter Timing::`
/// a whole-class selector without a second flag for it.
fn selected(filter: Option<&str>, class: &str, label: &str) -> bool {
    filter.is_none_or(|filter| format!("{class}::{label}").contains(filter))
}

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
/// row field to — the scalars and a `string` — because § 9 matches a
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

/// How the run came out, in the counts every format's summary carries.
#[derive(Clone, Copy)]
struct Counts {
    passed: usize,
    failed: usize,
    skipped: usize,
    /// § 20's own section: a test that passed within its retry allowance is
    /// counted here and in none of the fields above it.
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
///
/// **Version 2 is where a record says where its test is written.** `file`,
/// `line` and `column` are what a Test Explorer needs to place a test in a
/// file, and nothing below the compiler can recover them: a class label and a
/// method name say nothing about which file declared the two. The version went
/// up rather than a second document being written beside this one, because
/// this report is read by CI as well, and one schema with a number on it is
/// what `rule:ide/ast-json-schema-is-frozen` already settled as the shape for
/// exactly this.
fn json_document(cases: &[Case], counts: Counts, total: Duration) -> String {
    let mut out = String::from("{\n  \"schemaVersion\": 2,\n  \"summary\": {");
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
        location_keys(case.declared.as_ref(), &mut out);
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

/// One `#[Test]` call discovery found: what the report would name it, and
/// where it is written.
///
/// It carries no verdict and no duration, because nothing ran — a record
/// holding either would be a run this listing did not do.
struct Listed {
    class: String,
    method: String,
    declared: Option<Declared>,
}

/// Every `#[Test]` call the program declares that `filter` selects, located.
///
/// **Discovery is the front end's answer, not the runner's.** The table is a
/// product of the compile that already happened, so nothing here lowers,
/// installs or calls anything: a listing of a program whose tests fail, hang
/// or `exit` is the same listing as a passing one's. One entry per *call*
/// rather than per method, so a `#[TestWith]` row is listed under the label the
/// report would give it and a consumer can match the two by `class` and
/// `method`.
fn listing(checked: &crate::Checked, filter: Option<&str>) -> Vec<Listed> {
    let mut listed = Vec::new();
    for class in checked.exprs.test_classes() {
        for case in checked.exprs.tests(class).unwrap_or_default() {
            for call in invocations(case) {
                if selected(filter, class, &call.label) {
                    listed.push(Listed {
                        class: class.to_owned(),
                        method: call.label,
                        declared: declared_at(call.case.span, &checked.map),
                    });
                }
            }
        }
    }
    listed
}

/// The discovery document: the report's own version, and a `listed` array.
///
/// **`listed` rather than `tests`**, and no `summary` at all. A summary of
/// zeros is a report of a run where nothing passed, which is not what happened,
/// and a `tests` array whose records are missing their verdict would make every
/// consumer of the run document branch on a key's absence to tell a listing
/// from a run. The key that carries the data is what says which document this
/// is. The version is the same number the run document carries, because these
/// are two readings of one schema and `rule:ide/ast-json-schema-is-frozen` puts
/// one version on one surface.
fn listing_document(listed: &[Listed]) -> String {
    let mut out = String::from("{\n  \"schemaVersion\": 2,\n  \"listed\": [");
    for (at, test) in listed.iter().enumerate() {
        out.push_str(if at == 0 { "\n    {" } else { ",\n    {" });
        out.push_str("\"class\": ");
        json_string(&test.class, &mut out);
        out.push_str(", \"method\": ");
        json_string(&test.method, &mut out);
        location_keys(test.declared.as_ref(), &mut out);
        out.push('}');
    }
    out.push_str(if listed.is_empty() {
        "]\n}\n"
    } else {
        "\n  ]\n}\n"
    });
    out
}

/// The three keys that say where a test is written, or the three nulls that
/// stand in for them.
///
/// They are written together and by one function, because the run document and
/// the listing document both carry them and two spellings of one location is
/// the thing a consumer cannot be asked to handle. `null` only where the map
/// did not hold the span's file, which a test the compiler collected cannot
/// have: a consumer reads three keys or three nulls, never a record silently
/// missing the location. Data rows share a declaration, so every row of one
/// method reports that method's own line.
fn location_keys(declared: Option<&Declared>, out: &mut String) {
    match declared {
        Some(declared) => {
            out.push_str(", \"file\": ");
            json_string(&declared.file, out);
            out.push_str(&format!(
                ", \"line\": {}, \"column\": {}",
                declared.line, declared.column
            ));
        }
        None => out.push_str(", \"file\": null, \"line\": null, \"column\": null"),
    }
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

/// One written `Core\Test::assertMatchesInline`, resolved to the file holding
/// it — [`nvs_types::testing::InlineSnapshot`] with its `SourceId` looked up.
///
/// A row whose file was not read from disk (a `--FILE--` section compiled from
/// text) simply produces no site, which is what makes "nothing else ever
/// writes to a test's own file" hold for a program that has no file.
struct SnapshotSite {
    path: std::path::PathBuf,
    span: nvs_diagnostics::Span,
    expected: String,
    owner: Option<String>,
}

/// Every § 14 row the compile recorded, with its file resolved.
///
/// Read before the suite's task takes ownership of `checked`, which is the
/// only constraint on where this is called.
fn snapshot_sites(checked: &crate::Checked) -> Vec<SnapshotSite> {
    checked
        .exprs
        .inline_snapshots()
        .iter()
        .filter_map(|row| {
            let path = checked.map.get(row.span.file)?.path()?.to_path_buf();
            Some(SnapshotSite {
                path,
                span: row.span,
                expected: row.expected.clone(),
                owner: row.owner.clone(),
            })
        })
        .collect()
}

/// `rule:testing/inline-snapshots`'s updater: splices each failed snapshot's produced rendering
/// into the literal the test wrote, and writes nothing else anywhere.
///
/// # The join, and what it refuses
///
/// A run knows a snapshot by its *text* — `nvs_stdlib::test`'s helper doc owns
/// why there is no span at run time — so a mismatch is matched to a site by
/// the expected text **within the test that produced it**. That second half is
/// what makes the workflow § 14 describes work at all: it starts every
/// snapshot at `""`, so a program with two of them has two sites sharing a
/// join key, and only the method tells them apart.
///
/// Where the join is still not one site — two snapshots with the same text in
/// one method, or a snapshot whose expectation was not a written literal — the
/// site is **left alone and named**. Rewriting one of two candidates would put
/// a rendering under a snapshot nobody asserted, which is worse than the
/// failing test it replaces.
///
/// Only a test that ended `Failed` is read. A `Flaky` one passed on its last
/// attempt, so its snapshot is the one that held, and the mismatch on the
/// record is an earlier attempt's.
fn update_snapshots(sites: &[SnapshotSite], cases: &[Case]) {
    // Keyed by the site, so that two mismatches reaching one literal are
    // recognised as the ambiguity they are rather than racing to write it.
    let mut edits: Vec<(usize, String)> = Vec::new();
    let mut refused: Vec<String> = Vec::new();
    for case in cases {
        if !matches!(case.outcome, Outcome::Failed(_)) {
            continue;
        }
        // A `#[TestWith]` row is reported as `method#N` and written as
        // `method`, the rows being one declaration — `invocations` owns the
        // label — so the label is cut back to the method the source has.
        let method = case.method.split('#').next().unwrap_or(&case.method);
        let owner = format!("{}::{}", case.class, method);
        for mismatch in &case.snapshots {
            let mut found = sites.iter().enumerate().filter(|(_, site)| {
                site.owner.as_deref() == Some(owner.as_str()) && site.expected == mismatch.expected
            });
            let (Some((index, _)), None) = (found.next(), found.next()) else {
                refused.push(format!(
                    "`{owner}`: its snapshot is written twice with the same text, or is not a \
                     written literal, so there is no one place to put the new rendering"
                ));
                continue;
            };
            match edits.iter().find(|(at, _)| *at == index) {
                Some((_, already)) if *already == mismatch.produced => {}
                Some(_) => refused.push(format!(
                    "`{owner}`: one snapshot literal produced two different renderings in this \
                     run, so neither was written"
                )),
                None => edits.push((index, mismatch.produced.clone())),
            }
        }
    }
    for line in &refused {
        eprintln!("  not updated: {line}");
    }
    for (path, written) in splice(sites, &edits) {
        match written {
            Ok(()) => eprintln!("  updated: {}", path.display()),
            Err(error) => eprintln!("  not updated: {}: {error}", path.display()),
        }
    }
}

/// Applies `edits` to the files their sites are in, one write per file.
///
/// Each file's spans are replaced **back to front**, so an earlier edit's
/// change in length cannot move a later one's offsets — the spans are the
/// compile's, and nothing re-reads the file between them. The whole of a
/// file's edits is one `write`: a partial rewrite of a source file is the one
/// outcome an updater must not have.
fn splice(
    sites: &[SnapshotSite],
    edits: &[(usize, String)],
) -> Vec<(std::path::PathBuf, std::io::Result<()>)> {
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for (index, _) in edits {
        let path = &sites[*index].path;
        if !files.contains(path) {
            files.push(path.clone());
        }
    }
    files
        .into_iter()
        .map(|path| {
            let written = splice_one(&path, sites, edits);
            (path, written)
        })
        .collect()
}

/// One file's edits, applied and written.
fn splice_one(
    path: &std::path::Path,
    sites: &[SnapshotSite],
    edits: &[(usize, String)],
) -> std::io::Result<()> {
    let mut text = std::fs::read_to_string(path)?;
    let mut mine: Vec<&(usize, String)> = edits
        .iter()
        .filter(|(index, _)| sites[*index].path == path)
        .collect();
    mine.sort_unstable_by_key(|(index, _)| std::cmp::Reverse(sites[*index].span.start));
    for (index, produced) in mine {
        let span = sites[*index].span;
        let (start, end) = (span.start as usize, span.end as usize);
        if end > text.len() || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            // The file changed under the run — the spans are from the compile
            // that started it. Refusing the whole file is the safe half of
            // "writes the literal and nothing else".
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the file changed since it was compiled",
            ));
        }
        text.replace_range(start..end, &novis_literal(produced));
    }
    std::fs::write(path, text)
}

/// `text` as a Novis string literal — **single-quoted**, so what a reviewer
/// reads in the diff is the rendering itself.
///
/// A single-quoted literal has exactly two escapes and no interpolation
/// (`nvs_syntax`'s lexer, `lex_single_quoted`), so a newline, a `$`, a
/// backslash run and a `{` all stand for themselves. That is the property
/// § 14 is written on: the snapshot's whole point is that the diff is read,
/// and a multi-line rendering folded onto one line behind `\n` escapes is a
/// diff nobody reads. The double-quoted form would also have to escape `$` and
/// `{`, either of which appears in an ordinary `Core\Debug::render` of an
/// object.
fn novis_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for ch in text.chars() {
        if ch == '\'' || ch == '\\' {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::{Format, Outcome, compile, json_document, junit_document, run_suite_in_a_task};

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
        verdicts_filtered(name, None)
    }

    /// [`verdicts`] with `--filter`'s own argument, which is what selects the
    /// tests that run at all.
    fn verdicts_filtered(
        name: &str,
        filter: Option<&str>,
    ) -> Vec<(String, &'static str, Vec<String>)> {
        // Granting, because one fixture below spawns and `rule:security/capability-question-is-grant-and-scope` denies by
        // default — `crate::script::granting_ctx` owns why that helper exists.
        verdicts_on(&fixture(name), crate::script::granting_ctx(), filter)
    }

    /// [`verdicts_filtered`] over a context the caller built, for the one
    /// fixture whose configuration is more than a grant: `rule:testing/db-transaction` needs a
    /// `[db.<name>]` block, and a fixture has no `nvs.toml` beside it to carry
    /// one.
    fn verdicts_on(
        path: &std::path::Path,
        mut ctx: nvs_runtime::Ctx,
        filter: Option<&str>,
    ) -> Vec<(String, &'static str, Vec<String>)> {
        let checked = crate::front_end(path).expect("the fixture is a program");
        // No cache: a fixture's verdicts are about the runner, and a suite that
        // read one would be asserting against whatever a previous test run left
        // in this account's cache directory.
        let unit = compile(&checked, None).expect("the fixture compiles");
        unit.install_in(&mut ctx);
        // Through the same entry `run` takes, scheduler and all: a suite run
        // off a bare stack would be a different runner from the one shipped,
        // and § 16 is a claim about the one with a task tree under it.
        let (suite, _ctx) = run_suite_in_a_task(
            &unit,
            ctx,
            checked,
            &nvs_config::Config::default(),
            Format::Json,
            filter,
        )
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

    /// One fixture's whole suite, collected rather than reported, for the tests
    /// that are about what a *document* says.
    fn ran(name: &str) -> super::Suite {
        let path = fixture(name);
        let checked = crate::front_end(&path).expect("the fixture is a program");
        let unit = compile(&checked, None).expect("the fixture compiles");
        let mut ctx = crate::script::granting_ctx();
        unit.install_in(&mut ctx);
        let (suite, _ctx) = run_suite_in_a_task(
            &unit,
            ctx,
            checked,
            &nvs_config::Config::default(),
            Format::Json,
            None,
        )
        .expect("the suite's own task runs");
        suite
    }

    /// [`ran`] as § 22's JSON, parsed back.
    ///
    /// The document rather than the [`super::Suite`] it is built from, because
    /// what a Test Explorer and a CI job read is this text: a field the run
    /// resolved and the writer forgot to print is exactly the failure the test
    /// below exists to catch.
    fn json_report(name: &str) -> serde_json::Value {
        let suite = ran(name);
        let document = json_document(&suite.cases, suite.counts, std::time::Duration::ZERO);
        serde_json::from_str(&document).expect("the report is a JSON document")
    }

    #[test]
    fn every_json_test_record_carries_its_declaring_file_and_line() {
        let source = std::fs::read_to_string(fixture("statics-are-fresh.nvs"))
            .expect("the fixture is on disk");
        let report = json_report("statics-are-fresh.nvs");
        assert_eq!(
            report["schemaVersion"], 2,
            "the located record is what version 2 adds"
        );
        let tests = report["tests"]
            .as_array()
            .expect("the report lists the tests it ran");
        assert_eq!(tests.len(), 2, "the fixture declares two tests");
        for case in tests {
            let method = case["method"].as_str().expect("a record names its method");
            let file = case["file"].as_str().expect("a record names its file");
            assert!(
                std::path::Path::new(file).ends_with("statics-are-fresh.nvs"),
                "`{method}` is located in the file that declares it, not in `{file}`"
            );
            let line = usize::try_from(case["line"].as_u64().expect("a record carries a line"))
                .expect("a line number fits a `usize`");
            let declaration = source
                .lines()
                .nth(line - 1)
                .expect("the line is one of the fixture's");
            // The line is asserted against the fixture's own text rather than
            // against a number written here, so moving a test inside the
            // fixture does not fail a test about locating one.
            assert!(
                declaration.contains(method),
                "line {line} is where `{method}` is declared, and it reads `{declaration}`"
            );
            assert!(
                case["column"].as_u64().is_some_and(|column| column > 0),
                "`{method}`'s column is one-based, like the `-->` header's"
            );
        }
    }

    /// Discovery answers from the checked program alone.
    ///
    /// **Nothing in this test compiles a unit**, which is the whole claim:
    /// `compile` is what produces something callable, and a listing taken
    /// without it cannot have run a test whatever the fixture's tests do. The
    /// records are asserted against the same fixture the run document's test
    /// uses, so a listing and a run name the same two tests.
    #[test]
    fn the_listing_mode_discovers_without_executing() {
        let path = fixture("statics-are-fresh.nvs");
        let checked = crate::front_end(&path).expect("the fixture is a program");
        let listed = super::listing(&checked, None);
        let names: Vec<&str> = listed.iter().map(|test| test.method.as_str()).collect();
        assert_eq!(
            names,
            [
                "itWritesAStaticItsSiblingWillNotSee",
                "itReadsBackTheDeclaredInitialValue"
            ],
            "every declared test is listed, in declaration order"
        );
        for test in &listed {
            assert_eq!(test.class, "StaticsTest");
            let declared = test.declared.as_ref().expect("a listed test is located");
            assert!(
                std::path::Path::new(&declared.file).ends_with("statics-are-fresh.nvs"),
                "`{}` is located in the file that declares it, not in `{}`",
                test.method,
                declared.file
            );
        }
        let document: serde_json::Value = serde_json::from_str(&super::listing_document(&listed))
            .expect("the listing is a JSON document");
        assert_eq!(document["schemaVersion"], 2);
        assert_eq!(
            document["listed"].as_array().map(Vec::len),
            Some(2),
            "the discovery document carries its records under `listed`"
        );
        assert!(
            document["tests"].is_null() && document["summary"].is_null(),
            "a listing is not a run: it has no `tests` array and no summary of zeros"
        );
        assert!(
            document["listed"][0]["verdict"].is_null(),
            "nothing ran, so no record carries a verdict"
        );
        // `--filter` means the same thing here as it does in a run, which is
        // what lets an editor list and then run the same selection.
        let one = super::listing(&checked, Some("StaticsTest::itReadsBack"));
        assert_eq!(
            one.len(),
            1,
            "a filter selects what it would select in a run"
        );
    }

    /// The location is the JSON document's alone: the other two formats are
    /// the bytes they were before it existed.
    ///
    /// § 22's JUnit XML is asserted here attribute by attribute, because "the
    /// same bytes as before" is only checkable against the shape the writer
    /// emits — a `file=` or `line=` attribute added to a `<testcase>` would be
    /// a schema change nobody versioned, this format having no version to
    /// raise. The plaintext report's bytes are pinned where it is rendered, by
    /// `tests/conformance/lang/a-test-attribute-builds-a-table-the-runner-reports.nvst`,
    /// which this slice left untouched.
    #[test]
    fn the_schema_version_is_two_and_junit_and_human_are_byte_identical() {
        let suite = ran("statics-are-fresh.nvs");
        let json = json_document(&suite.cases, suite.counts, std::time::Duration::ZERO);
        assert!(
            json.contains("\"schemaVersion\": 2,"),
            "the located record is version 2 of this document: {json}"
        );
        let junit = junit_document(&suite.cases, suite.counts, std::time::Duration::ZERO);
        for case in &suite.cases {
            assert!(
                junit.contains(&format!(
                    "<testcase name=\"{}\" classname=\"{}\" time=\"",
                    case.method, case.class
                )),
                "a `<testcase>` carries the three attributes it always did, in order: {junit}"
            );
        }
        for attribute in ["file=", "line=", "column="] {
            assert!(
                !junit.contains(attribute),
                "`{attribute}` is the JSON document's, and JUnit has no version to raise: {junit}"
            );
        }
    }

    #[test]
    fn test_request_dispatches_in_process_through_the_compiled_route_table() {
        // `rule:testing/in-process-request`'s first mechanism, asserted where it lives: `nvs-test`
        // holds the `.nvst` format and declares no dependencies at all, so the
        // crate that can build both halves of this — a checked program and a
        // runtime context in one scope — is this one.
        //
        // Two cases in one fixture, because the claim is about the *table* and
        // not about one row of it. The first asks a path the fixture's own
        // `#[Route]` declares and reads the match back inside the program that
        // answered, which is the whole of "through the compiled route table";
        // the second asks a path no route claims and still gets an answer,
        // which is `rule:routing/matched-once-before-the-handler`'s "nothing here dispatches" — a runner that
        // sent a `404` of its own would pass the first and fail the second.
        // Neither opens a socket: the fixture is answered by an isolate over
        // this run's own unit (`UnderTest`).
        let verdicts = verdicts("in-process-request.nvs");
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itReachesTheCompiledTableWithNoSocket", "passed"),
                ("itAnswersAPathTheTableDoesNotClaim", "passed"),
            ],
            "failures: {:?}",
            verdicts
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_test_with_server_true_gets_an_ephemeral_listener() {
        // `rule:testing/in-process-request`'s second mechanism, and the crate that can assert it is
        // this one for the first mechanism's reason: a listener needs a checked
        // program and a runtime context in one scope, and `nvs-test` declares no
        // dependencies at all.
        //
        // Two cases in one fixture, because the claim is that the listener is
        // the *test's* and not the process's. The first reads its own address,
        // fetches it over a real socket and asserts what the program wrote came
        // back with a `200`; the second declares no `server:` and reads `null`,
        // which a runner that armed the URL anywhere above the isolate — on the
        // suite's own context, say — would fail while passing the first.
        // Nothing here names a port: the operating system chose it, which is
        // what makes the case runnable twice at once.
        let verdicts = verdicts("ephemeral-listener.nvs");
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itReachesItsOwnListenerOverTheWire", "passed"),
                ("itHasNoListenerWithoutTheOption", "passed"),
            ],
            "failures: {:?}",
            verdicts
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
    }

    /// `[db.test]` over the compose PostgreSQL `nvs.toml`'s `[db.main]` names,
    /// or `None` where this machine has no such server.
    ///
    /// **The block is built here rather than read from a file**, which is the
    /// same call `crate::script::granting_ctx` makes for a capability: a fixture
    /// is one `.nvs` under `tests/fixtures/runner/` with no configuration tree
    /// beside it, and giving one a tree would make every fixture's ambient
    /// configuration a question. What that costs is that the boot-time
    /// resolution `nvs_config::db` performs is skipped, so the CA bundle is
    /// named absolutely here — a relative path would resolve against whatever
    /// directory `cargo test` chose.
    ///
    /// **`None` is a skip and not a failure.** The bundle is issued by
    /// `tests/db/compose.yaml`'s own `certs` service into a Docker volume and is
    /// not in git, so a checkout with no servers up cannot reach the database
    /// this asserts about at all. `tools/loop.py` brings both up for the
    /// acceptance sweep, which is where the assertion below actually runs.
    fn compose_postgres() -> Option<nvs_config::tree::Database> {
        let bundle = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("db")
            .join("ca.crt");
        if !bundle.is_file() {
            return None;
        }
        let address = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 15432));
        // A plain TCP probe and not a handshake: what it answers is *is there a
        // server here*, and every other question — TLS, the login, the schema —
        // is one this test is meant to fail on rather than skip over.
        std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_millis(500))
            .ok()?;
        Some(nvs_config::tree::Database {
            driver: Some("postgres".to_owned()),
            host: Some("127.0.0.1".to_owned()),
            port: Some(15432),
            user: Some("novis".to_owned()),
            password: Some("Novis-Test-Pw1".to_owned()),
            database: Some("novis_test".to_owned()),
            tls_ca_file: Some(bundle.to_string_lossy().into_owned()),
            ..nvs_config::tree::Database::default()
        })
    }

    #[test]
    fn a_test_with_db_runs_inside_a_transaction_that_is_rolled_back() {
        // `rule:testing/db-transaction`, and the crate that can assert it is this one for the
        // reason the two mechanisms above are asserted here: the transaction is
        // opened around a test's isolate, which needs a checked program and a
        // runtime context in one scope.
        //
        // Four cases in one fixture, in declaration order, because no single
        // one of them is the claim. The first commits a table because it names
        // no `db:`; the second writes a row inside the runner's transaction and
        // proves its own nested `transaction()` is a savepoint rather than a
        // second connection; the third is the assertion — the row is gone, with
        // no cleanup written anywhere — and the fourth drops the table again, so
        // a run leaves the database as it found it. A runner that opened a
        // transaction for every test would roll the *table* away and fail the
        // second; one that opened none would fail the third.
        let Some(block) = compose_postgres() else {
            // Printed rather than silent: a green line for a test that reached
            // no database is exactly the outcome that should be legible.
            eprintln!(
                "skipped: no PostgreSQL on 127.0.0.1:15432, or no `tests/db/ca.crt` — \
                 `docker compose -f tests/db/compose.yaml up -d --wait` is what this needs"
            );
            return;
        };
        let mut snapshot = nvs_config::Snapshot::default();
        // `rule:core-classes/db-capabilities`'s grant, for the one name the fixture opens. `net.*` is
        // deliberately not granted beside it: a `[db.<name>]` endpoint is
        // operator-written and so is pre-approved against `rule:http-server/allow-url-pins-the-address`'s denied
        // ranges (`nvs_config::tree::CapDb`), and a test that granted both could
        // not tell which of the two the connection went through.
        snapshot.config.capabilities = Some(nvs_config::tree::Capabilities {
            db: Some(nvs_config::tree::CapDb {
                connect: Some(nvs_config::tree::Setting::List(vec!["test".into()])),
                open: None,
                schema: None,
            }),
            ..nvs_config::tree::Capabilities::default()
        });
        snapshot.config.db.insert("test".to_owned(), block);
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
        ctx.set_config(std::sync::Arc::new(snapshot));
        let verdicts = verdicts_on(&fixture("db-transaction.nvs"), ctx, None);
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itPreparesATableOutsideAnyTransaction", "passed"),
                ("itWritesInsideTheRunnersTransaction", "passed"),
                ("itSeesNothingTheLastTestWrote", "passed"),
                ("itDropsTheTableOutsideAnyTransaction", "passed"),
            ],
            "failures: {:?}",
            verdicts
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn each_test_runs_in_its_own_isolate_sharing_only_compiled_code() {
        // `rule:testing/isolate-per-test` and `rule:statements/an-isolate-has-its-own-statics`: the two tests share the one compiled
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
        // `rule:testing/determinism-declared-on-the-test`. Three cases in one fixture because the claim is about
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

    /// `--filter` selects `#[Test]` methods, by the containment rule
    /// [`super::selected`] owns — the flag reaches a program's own tests and
    /// not only the `.nvst` tree.
    ///
    /// Four filters over the one fixture, because the claim is *selection*
    /// and a single filter cannot make it: the method name picks one of three,
    /// the shared substring picks all three, the class name picks the class
    /// without a flag of its own, and a name nothing carries picks nothing.
    /// A runner that ignored the filter passes the second of those alone.
    #[test]
    fn test_filter_selects_test_methods_by_name() {
        let selected = |filter: &str| {
            verdicts_filtered("fixed-clock.nvs", Some(filter))
                .into_iter()
                .map(|(method, _, _)| method)
                .collect::<Vec<_>>()
        };

        assert_eq!(selected("ItAdvanced"), vec!["itReadsTheClockItAdvanced"]);
        assert_eq!(
            selected("Clock"),
            vec![
                "itReadsTheClockItDeclared",
                "itReadsTheClockItAdvanced",
                "itReadsTheHostClockWithoutAnAt",
            ]
        );
        assert_eq!(selected("FixedClockTest::").len(), 3);
        assert!(selected("noSuchTest").is_empty());
    }

    #[test]
    fn a_test_with_a_seed_draws_the_same_sequence_twice() {
        // `rule:testing/determinism-declared-on-the-test`'s other half. Two tests declaring the same seed draw
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
        // `rule:testing/task-tree-and-virtual-clock`, both halves, from the language surface. The first
        // test's `Core\Task::all` has children only because the suite is
        // itself a task (`rule:concurrency/a-child-belongs-to-the-calling-task`'s "a child of the calling task"), so a
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

    /// `rule:testing/inline-snapshots`, both halves in one case because they are one claim: the
    /// updater writes the produced rendering into the `$expected` literal that
    /// asked for it, and nothing else in `nvs test` writes to a source file at
    /// all.
    ///
    /// Run over a **copy**, in a directory of this test's own, for the obvious
    /// reason — and the copy is where the "never otherwise" half is asserted,
    /// byte for byte, against the run that was not asked to update.
    ///
    /// Both snapshots in the fixture start empty, so the expected text alone
    /// joins a mismatch to two sites; that they still land in their own
    /// literals is what says the join is narrowed by the method the call is
    /// written in. The last assertion is the one that says the spliced text is
    /// a *literal* and not merely bytes: the same suite, recompiled from the
    /// rewritten file, passes.
    #[test]
    fn an_inline_snapshot_updates_its_own_source_when_asked_and_never_otherwise() {
        let dir = std::env::temp_dir().join(format!(
            "nvs-inline-snapshot-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("a directory of this test's own");
        let program = dir.join("inline-snapshot.nvs");
        let template = std::fs::read_to_string(fixture("inline-snapshot.nvs"))
            .expect("the fixture is on disk");
        std::fs::write(&program, &template).expect("the copy is written");

        // `file_cache = false` and not a default snapshot: this test is about
        // what the updater writes, and a run reading the account's own artifact
        // directory would put a test's units in it — `cache::from_config`
        // answers `None` for exactly this block.
        let uncached = nvs_config::Snapshot {
            config: toml::from_str("[opcache]\nfile_cache = false\n")
                .expect("a tree that turns the disk cache off"),
            ..Default::default()
        };

        // Never otherwise: the shipped entry, with the flag off, over a suite
        // whose every snapshot fails.
        let checked = crate::front_end(&program).expect("the copy is a program");
        let _ = super::run(checked, &uncached, Format::Json, None, false, false);
        assert_eq!(
            std::fs::read_to_string(&program).expect("the copy is still there"),
            template,
            "a run that was not asked to update writes nothing at all"
        );

        // When asked.
        let checked = crate::front_end(&program).expect("the copy is a program");
        let _ = super::run(checked, &uncached, Format::Json, None, true, false);
        let updated = std::fs::read_to_string(&program).expect("the copy is still there");
        assert_ne!(updated, template, "the update rewrote the source");
        assert!(
            updated.contains("Users: 3"),
            "the summary's own rendering is in the literal that asserted it: {updated}"
        );
        assert!(
            updated.contains("class Snapshots {"),
            "and nothing outside the two literals moved: {updated}"
        );

        let verdicts = verdicts_on(&program, crate::script::granting_ctx(), None);
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itRendersTheSummary", "passed"),
                ("itCountsTheActiveOnes", "passed"),
            ],
            "the rewritten snapshots are what the run produces: {verdicts:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
