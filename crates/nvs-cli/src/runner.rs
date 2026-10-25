//! `rule:testing/test-attribute`'s
//! runner: the `#[Test]` table a compile already built (`rule:testing/test-attribute`), constructed and
//! called (`rule:testing/runner-is-strict`), judged off `rule:testing/failure-ledger`'s ledger — [`run_case`] owns why that and not
//! the exception state — and reported in one of `rule:testing/report-formats`'s formats.
//!
//! # One verdict, every rendering
//!
//! Every format renders the same [`Case`] list, and a verdict is decided once
//! — [`Outcome::verdict`] — rather than per format, so the plaintext mark, the
//! JSON string and the JUnit element cannot disagree about how a test came
//! out. Each failure inside a verdict is `rule:errors/record-producers`'
//! record ([`failure`], [`assertion_record`]), and every format prints the
//! record's message. The plaintext one is the default and is written in declaration order as
//! each test is reported, which is what keeps a long suite legible; a machine
//! format is one document written at the end, because neither JSON nor XML has
//! a prefix worth streaming. Every line the plaintext report writes goes
//! through the suite's own context, so a test of this module reads the report
//! back out of a buffer.
//!
//! **A test's output is captured** (`rule:testing/tests-run-across-every-core`):
//! each test runs under a parent context of its own whose sink is a buffer, and
//! the child isolate's `Output::Inherit` lands its `echo` there. A passing
//! test's buffer is dropped at its report; a failing one's is written under its
//! failure, in the report's place. What a class's `#[BeforeClass]` and fixtures
//! print is captured on the suite's context and written under the class header,
//! and `#[AfterClass]` runs at its place in the report, so the report reads the
//! same on one core as on many. `--show-output` gives each test's context the
//! terminal instead and captures nothing. **Under a machine format the suite's
//! context writes to stderr**: `rule:testing/report-formats` writes `nvs test
//! --format=json > results.json`, so stdout is the document and nothing else,
//! and a failing test's output is not quoted into the document either.
//!
//! # Why the runner is here
//!
//! It is the one place that can reach both halves at once. The roster is
//! `nvs_types::ExprTypeTable::tests`, which is a *front-end* table, and the
//! instance it constructs is a compiled unit's, reached through a `Ctx` — and
//! `nvs-cli` is the only crate that holds a checked program and a runtime
//! context in one scope. Neither half is discovered at startup: `rule:testing/test-attribute`'s table
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
//! `rule:testing/isolate-per-test`'s rule is [`run_in_isolate`]'s one call into `nvs_host::Isolate`: a
//! test's body runs on a context of its own, over the *same* compiled unit, so
//! a static one test wrote reads its declared initial value in the next and
//! there is no flag to change it (`rule:statements/an-isolate-has-its-own-statics`'s fresh statics base, armed by
//! the `install_in` inside the child's own program). The isolate is per
//! **test** and not per attempt, which is `rule:testing/runner-is-strict`'s retry left usable:
//! [`run_with_retries`] runs inside it.
//!
//! # Tests run across every core
//!
//! `rule:testing/tests-run-across-every-core` is [`run_suite`]'s queue. Each
//! test is a [`Job`], posted to another core with `nvs_host::worker::post`, as
//! many at once as `--jobs` allows; it builds its own parent context there and
//! runs [`run_in_isolate`] under it. `--jobs 1`, and a core that cannot be had,
//! run the job on the suite's own thread. The class hooks and the fixtures run
//! on the suite's context, and the queue keeps `rule:testing/class-hooks`'s
//! order and the report's declaration order ([`Queue`]).
//!
//! Two things cross the isolate boundary and both are copies. `rule:testing/fixtures`'s fixtures are
//! built **once per class** in the suite's own context, encoded there and decoded
//! on the test's core (`nvs_runtime::CrossedFixtures`), which is the whole of what `rule:testing/fixtures` asks
//! for: the expensive setup runs once, and what a test can reach is its own
//! graph. The verdict crosses the other way as data — [`Outcome`] is decided
//! on the child's stack, off the child's own ledger, and filed into a cell the
//! child's program captured, because `rule:security/isolate-shares-nothing`'s `Completion` carries a value,
//! bytes and a failure and none of those is a ledger.
//!
//! # The runner owns the test's task tree
//!
//! `rule:testing/task-tree-and-virtual-clock`'s first half is [`run_suite_in_a_task`]: the whole suite runs inside
//! one task, so a test's isolate is a *child* of it and a `Core\Task::all`
//! written in a test has a calling task to put its own children under. That
//! function owns why it is one task for the suite rather than one per test,
//! and what has to be moved into it to make the scheduler's signature work.
//!
//! The second half is read off that tree at the one point it means anything —
//! the moment the test's body returns, on the test's own stack, in
//! [`run_in_isolate`]'s program, a Rust closure. **A task still running then is a
//! failure named as such** ([`Outcome::with_tasks_left_running`]), because the
//! alternative is what the scheduler does on its own: cancel the leftovers as
//! the task retires and report a green test that never waited for its work.
//! `rule:concurrency/nothing-is-still-running-when-a-call-returns` makes that bound reachable rather than aspirational — `::all`
//! and `::map` return with nothing still running — so the only way to fail
//! this is a `spawn script` the test never awaited.
//!
//! # What a constructor, a fixture and class order do
//!
//! A constructor that declares parameters is
//! reported as that test failing rather than pretended past
//! (`nvs_runtime::construct_and_call`): `rule:testing/constructor-is-setup` makes the constructor `setUp` and
//! `rule:testing/fixtures` and `rule:testing/data-rows` fill the test method's own parameters, which [`build_fixtures`]
//! builds once per class and [`invocations`] materializes per row. Class order is the roster's own sorted
//! order rather than `rule:testing/runner-is-strict`'s declaration order: `ExprTypeTable::tests` is keyed
//! by class label and records no sequence, so declaration order across a
//! program's files is a fact only that table can grow. Within a class the
//! order *is* declaration order, which is what a reader of one file sees.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nvs_render::{Level, Node, Record, Rendered, Scalar};
use nvs_types::defaults::ConstArg;
use rand::RngExt;

mod mutation;

/// Run a program's `#[Test]` methods, or a tree of `.nvst` conformance
/// cases.
///
/// The path decides which one runs. A `.nvs` file is a program, and its
/// `#[Test]` methods run. A folder with `.nvs` files is one program made of
/// every `.nvs` file in it and in its subfolders. Run `nvs test .` in your
/// project folder to run every test in the project. The entry file is in
/// that folder, so its `autoload` line loads your classes for every test.
/// The tests can be under `src/`, or in a `tests/` folder with one
/// `bootstrap.nvs` that requires the entry file. A file that another file
/// in the folder requires is loaded once, through that file.
/// Any other path is a `.nvst` case file or a folder of `.nvst` files. A
/// folder with both kinds of file is an error, because the two kinds have
/// different reports.
///
/// Exits non-zero if any case or any test failed; a skipped one is not a
/// failure.
// `rule:testing/nvst-is-separate` § 1, and
// `rule:testing/a-directory-of-programs-is-one-test-program` for the directory.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The program or directory of programs to run, or the case files and
    /// directories.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
    /// Run only the tests whose name contains this text — a `.nvst` case's
    /// path, or a `#[Test]` method's `Class::method`.
    #[arg(long, value_name = "TEXT")]
    filter: Option<String>,
    /// Run only the tests that have this tag.
    ///
    /// A test gets tags with `#[Test(tags: ["slow", "db"])]`. You can give
    /// `--tag` more than once. A test then runs when it has at least one of
    /// the tags.
    // `rule:testing/tags-and-shards-select-what-runs`.
    #[arg(long, value_name = "TAG")]
    tag: Vec<String>,
    /// Do not run the tests that have this tag.
    ///
    /// You can give `--skip-tag` more than once. A test with any of these
    /// tags does not run, even when it also has a `--tag` tag.
    #[arg(long, value_name = "TAG")]
    skip_tag: Vec<String>,
    /// Run one part of the tests, such as `--shard 2/4` for the second of
    /// four parts.
    ///
    /// Each test is in exactly one part. A test stays in the same part when
    /// other tests are added or removed. Four machines that each run one of
    /// `1/4` to `4/4` run every test once. A `.nvst` case is placed by its
    /// path.
    #[arg(long, value_name = "N/M")]
    shard: Option<nvs_test::Shard>,
    /// Stop starting tests after the first test fails.
    ///
    /// Tests that are already running finish, and they are reported. The
    /// summary counts the tests that did not start as not run. A flaky test
    /// does not stop the run.
    // `rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle`, as are the two below.
    #[arg(long, conflicts_with = "list")]
    fail_fast: bool,
    /// Run only the tests that failed in the last run of this program.
    ///
    /// Every run writes the names of its failed tests to a file in the data
    /// folder. When the last run had no failed test, or no run is recorded,
    /// every selected test runs. `--filter`, `--tag` and `--skip-tag` still
    /// select from these tests.
    #[arg(long, conflicts_with = "list")]
    rerun_failed: bool,
    /// Start the tests in a random order, and print the seed in the summary.
    ///
    /// `--shuffle=<seed>` starts the tests in the same order as the run that
    /// printed that seed. The tests of each class start in a random order.
    /// The report still lists the tests in the order they are written.
    #[arg(
        long,
        value_name = "SEED",
        num_args = 0..=1,
        require_equals = true,
        conflicts_with = "list"
    )]
    shuffle: Option<Option<u64>>,
    /// Run the tests, then run them again each time a source file of the
    /// program changes. Press Ctrl+C to stop.
    ///
    /// The files checked are the files the program reads: the file you name
    /// and every file it requires or autoloads. A change while the tests
    /// run starts the next run after this one ends. When the program does
    /// not compile, the errors are printed, and the tests run again at the
    /// next change. `--watch` needs the human format, and does not run
    /// `.nvst` cases.
    // `rule:testing/watch-reruns-on-an-edit`.
    #[arg(long, conflicts_with_all = ["list", "update"])]
    watch: bool,
    /// How many tests or `.nvst` cases run at once.
    ///
    /// The default is the number of hardware threads on this machine.
    /// `--jobs 1` runs one test at a time.
    // `rule:testing/tests-run-across-every-core`.
    #[arg(long, value_name = "N")]
    jobs: Option<std::num::NonZeroUsize>,
    /// Print each test's output as it runs.
    ///
    /// Without this flag, the output of a test that passes is not shown.
    /// The output of a test that fails is printed under its failure. With
    /// this flag, lines from tests that run at the same time may mix.
    // `rule:testing/tests-run-across-every-core`.
    #[arg(long)]
    show_output: bool,
    /// Also run each `.nvst` case on one shared worker, after its own
    /// run has passed.
    ///
    /// The worker runs every case's program in turn, in the same process,
    /// as a server runs requests. A case fails when its output there
    /// differs from its own run. Then the case read something that an
    /// earlier case left behind. A case that needs a request, arguments,
    /// environment variables or other files runs only on its own.
    // `rule:security/isolate-shares-nothing`.
    #[arg(long, conflicts_with_all = ["list", "update"])]
    shared_worker: bool,
    /// How a program's `#[Test]` run is reported.
    ///
    /// The default is the human format, and it is what a `.nvst` tree is
    /// always reported in: a conformance run's report is a summary rather
    /// than a suite of test methods, and the two share nothing — so naming
    /// a machine format beside a case tree is refused rather than silently
    /// ignored.
    // `rule:testing/report-formats`, and § 23 is what keeps the two reports
    // from sharing a shape.
    #[arg(long, value_name = "FORMAT", default_value = "human")]
    format: Format,
    /// Rewrite each failed `Core\Test::assertMatchesInline` snapshot in
    /// the source that wrote it.
    ///
    /// This is the only spelling under which `nvs test` writes to a source
    /// file, and what it writes is the `$expected` literal and nothing
    /// else: a run without it never touches the program, and a run with it
    /// never touches a passing snapshot. A `.nvst` tree has no snapshot to
    /// update, so naming it there is refused rather than ignored.
    // `rule:testing/inline-snapshots`.
    #[arg(long)]
    update: bool,
    /// Write the run's coverage to this file in the lcov format.
    ///
    /// Every line a statement starts on is listed with the number of
    /// times it ran, `0` for a line no test reached. Every branch is
    /// listed with the number of times its true side and its false side
    /// ran. Every function is listed with the number of times it was
    /// called. A `.nvst` tree has
    /// no coverage to report, so naming it there is an error.
    // `rule:testing/debug-probes`; `crate::coverage` owns how the counts are taken.
    #[arg(long, value_name = "FILE", conflicts_with = "list")]
    coverage_lcov: Option<PathBuf>,
    /// Write the run's coverage to this file as Clover XML.
    ///
    /// The same lines, branches, functions and counts as
    /// `--coverage-lcov`, in the Clover format that many CI services
    /// read.
    #[arg(long, value_name = "FILE", conflicts_with = "list")]
    coverage_clover: Option<PathBuf>,
    /// Write the run's coverage to this file as Cobertura XML.
    ///
    /// The same lines, branches and counts as `--coverage-lcov`, in the format
    /// GitLab and Azure DevOps read. One run can write all three files.
    #[arg(long, value_name = "FILE", conflicts_with = "list")]
    coverage_cobertura: Option<PathBuf>,
    /// Time every call and write a profile file when the tests end.
    ///
    /// `callgrind` writes the time spent in each function, for KCachegrind
    /// or Webgrind. `speedscope` writes every call on a timeline, with one
    /// timeline for each test. The file is `callgrind.out.<pid>` or
    /// `profile.<pid>.speedscope.json` in the current folder. The timing
    /// adds a small cost to every call, so code that makes many small calls
    /// looks slower than it is. A `.nvst` tree has no profile.
    #[arg(long, value_name = "FORMAT", conflicts_with = "list")]
    profile: Option<crate::profile::Format>,
    /// Write the profile to this file instead.
    #[arg(long, value_name = "FILE", requires = "profile")]
    profile_out: Option<PathBuf>,
    /// Check whether the tests notice small changes to the program.
    ///
    /// The tests run once. Then the program is changed in one place, such
    /// as `<` to `<=`, and the tests that run that line run again. A test
    /// should now fail. When every test still passes, the change survived,
    /// and the report lists it with its file and line. Each change is
    /// killed, survived, timed out or not reached. The score is the share
    /// of reached changes that the tests noticed. Test code is never
    /// changed. A survived change does not make the run fail.
    // `rule:testing/mutation-testing`.
    #[arg(long, conflicts_with_all = ["list", "update", "watch"])]
    mutate: bool,
    /// Run the benchmarks instead of the tests.
    ///
    /// A benchmark is a method with `#[Bench]`. Each one runs once, and the
    /// report prints how many statements, calls, allocations, bytes and
    /// garbage collections it used. These numbers are the same on every run
    /// and on every machine, so CI can compare them. The report also prints
    /// the time each benchmark took. The time changes from run to run, so the
    /// report marks it as advisory. The benchmarks run one at a time.
    // `rule:testing/bench-counters`.
    #[arg(long, conflicts_with_all = ["mutate", "jobs"])]
    bench: bool,
    /// Print each test and the file it is in, without running any test.
    ///
    /// For a program, each `#[Test]` method is printed with its line. The
    /// program is compiled, but no test runs. A test that fails or hangs
    /// is listed like a test that passes. For `.nvst` cases, each case
    /// file is printed. A folder with any `.nvst` file in it lists its
    /// cases. `--filter`, `--shard` and `--cases` select what is listed.
    /// Add `--format json` to get one JSON document.
    // `rule:testing/report-formats`, and `rule:ide/every-feature-is-staged-behind-its-dependency`
    // is why the CLI surface lands before the client that reads it.
    #[arg(long, conflicts_with = "update")]
    list: bool,
    /// Run only the `.nvst` case files this file lists, one path per
    /// line, out of the trees named.
    ///
    /// A case is judged and counted exactly as a whole-tree run judges it,
    /// and the summary counts the listed cases. A listed path that no named
    /// tree holds is refused before anything runs.
    #[arg(long, value_name = "FILE")]
    cases: Option<PathBuf>,
    /// Record each `.nvst` case into this directory, which is created
    /// when it is not there.
    ///
    /// Every `nvs` process a case starts writes its coverage counters to
    /// `<name>-<pid>.profraw` when the binary is built with coverage, and
    /// the classes and files it used to `<name>.log`, and compiles with
    /// no artifact cache. `<name>` is the case's path with `/` and `\`
    /// as `~` and every other character that is not a letter, a digit,
    /// `.`, `_` or `-` as `@` and two hex digits.
    #[arg(long, value_name = "DIR", conflicts_with = "list")]
    record: Option<PathBuf>,
}

impl Args {
    /// Runs `nvs test` with the arguments clap parsed, the global `--config`
    /// files and the project's init gate.
    pub(crate) fn run(self, config: &[PathBuf], init: crate::config::Init) -> ExitCode {
        crate::run_test(
            &self.paths,
            Selection {
                filter: self.filter,
                tags: self.tag,
                skip_tags: self.skip_tag,
                shard: self.shard,
                only: None,
                bench: self.bench,
            },
            self.jobs,
            self.format,
            Flags {
                update: self.update,
                list: self.list,
                // One bench at a time, so no other bench shares its core or its wall time.
                jobs: match self.bench {
                    true => 1,
                    false => self
                        .jobs
                        .map_or_else(default_jobs, std::num::NonZeroUsize::get),
                },
                show_output: self.show_output,
                shared_worker: self.shared_worker,
                fail_fast: self.fail_fast,
                rerun_failed: self.rerun_failed,
                shuffle: self
                    .shuffle
                    .map(|seed| seed.unwrap_or_else(|| rand::rng().random::<u64>())),
                record_failed: None,
                mutate: self.mutate,
                profile: self.profile.map(|format| (format, self.profile_out)),
            },
            &crate::coverage::Requested {
                lcov: self.coverage_lcov,
                clover: self.coverage_clover,
                cobertura: self.coverage_cobertura,
            },
            self.cases,
            self.record,
            self.watch,
            config,
            init,
        )
    }
}

/// `rule:testing/report-formats`'s report formats.
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
    /// `rule:testing/runner-is-strict`'s "a skip states a reason", carrying it.
    Skipped(String),
    /// Why it failed — one [`failure`] record per failed assertion the ledger
    /// recorded, or one for the message a throw or an empty ledger left. A
    /// list rather than the first one, because a test that caught its own
    /// failures and carried on has several, and reporting one of them would
    /// hide exactly what `rule:testing/failure-ledger`'s ledger exists to keep.
    Failed(Vec<Record>),
    /// `rule:testing/runner-is-strict`'s `retries:`: the test failed and then passed within its
    /// allowance, carrying how many attempts it took and what the last failed
    /// one said. It is neither passed nor failed, which is the whole of that
    /// bullet — see [`run_with_retries`].
    Flaky {
        attempts: usize,
        failures: Vec<Record>,
    },
    /// The test called `exit(n)`, which ends the whole run.
    Exited(i64),
}

impl Outcome {
    /// A failure with one record, carrying `message` alone.
    fn failed(message: impl AsRef<str>) -> Self {
        Self::Failed(vec![failure(message.as_ref())])
    }

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
            // Its own mark rather than the passing one: `rule:testing/runner-is-strict` reports a retried
            // test as flaky and never as green, and a reader scanning the
            // marks is the first place that has to be true.
            Self::Flaky { .. } => "!",
            Self::Failed(_) | Self::Exited(_) => "\u{2717}",
        }
    }

    /// `rule:testing/task-tree-and-virtual-clock`'s second half folded in: `running` children the test left behind
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
            "it left {running} task(s) still running when it returned. A test fails when one \
             of its tasks is still running at its end. `Core\\Task::all` and `::map` return with \
             nothing still running, and `await` finishes a `spawn script`"
        );
        self.and_failed(named)
    }

    /// This outcome with one more failure folded into it — what the runner
    /// itself found wrong *around* the test, rather than what the test's own
    /// ledger recorded.
    ///
    /// [`Self::with_tasks_left_running`] is `rule:testing/task-tree-and-virtual-clock`'s caller and this is `rule:testing/db-transaction`'s,
    /// which is why the fold is here once rather than at each: a verdict a
    /// report could call green becomes a failure, a failure gains a line, and a
    /// skip or an `exit(n)` is untouched — a test that never ran cannot have
    /// left a transaction open, and an `exit(n)` ended the program rather than
    /// the test.
    fn and_failed(self, named: String) -> Self {
        self.and_record(failure(&named))
    }

    /// [`Self::and_failed`] with a record already built, for a failure that carries fields
    /// beside its line.
    fn and_record(self, record: Record) -> Self {
        match self {
            Self::Passed => Self::Failed(vec![record]),
            Self::Failed(mut failures) | Self::Flaky { mut failures, .. } => {
                failures.push(record);
                Self::Failed(failures)
            }
            settled @ (Self::Skipped(_) | Self::Exited(_)) => settled,
        }
    }

    /// `rule:testing/a-test-has-a-wall-time-limit`'s failure. The runtime's own line for the stop
    /// names a request, so it is replaced by one that names the test's limit and where it was
    /// written. A test stopped while it was being retried is a failure, never flaky.
    fn past_wall_time(self, limit: &str) -> Self {
        let named = format!("{PAST_WALL_TIME} of {limit}");
        match self {
            Self::Failed(failures) | Self::Flaky { failures, .. } => {
                let mut kept: Vec<Record> = failures
                    .into_iter()
                    .filter(|record| {
                        !message_of(record).starts_with(nvs_runtime::Ctx::WALL_TIME_EXCEEDED)
                    })
                    .collect();
                kept.push(failure(&named));
                Self::Failed(kept)
            }
            other => other.and_failed(named),
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
            } => Some(reasons.first().map_or("it failed", message_of).to_owned()),
            Self::Exited(code) => Some(format!("the test called exit({code})")),
        }
    }
}

/// `rule:errors/record-producers`' test-result producer: one failure of a
/// test as `rule:errors/diagnostic-record`'s record, carrying `message`
/// alone.
///
/// The record is at `Error`, and its message is the failure's line. Every
/// report prints that line from the record, so a control byte in a message a
/// test wrote is substituted once, here, for all three formats.
fn failure(message: &str) -> Record {
    let mut record = Record::at(Level::Error);
    record.envelope.message = Some(Rendered::new(message));
    record
}

/// The record of one failed ledger entry, or `None` for an assertion that
/// held: no report shows a passing assertion, so it builds nothing.
///
/// [`ASSERTION`] names the `Core\Test` method. A failed comparison's two
/// values are the record's nodes, actual first and expected second, in the
/// order the method takes them.
fn assertion_record(entry: nvs_runtime::AssertionOutcome) -> Option<Record> {
    let mut record = failure(entry.failure.as_deref()?);
    record.envelope.fields.push((
        ASSERTION.to_owned(),
        Node::Scalar(Scalar::Str {
            text: Rendered::new(entry.member),
            bytes: entry.member.len(),
        }),
    ));
    if let Some(compared) = entry.compared {
        let nvs_runtime::Compared { actual, expected } = *compared;
        record.nodes = vec![actual, expected];
    }
    Some(record)
}

/// How [`Outcome::past_wall_time`]'s failure starts, which is how a mutant's run tells a test
/// stopped at its limit from one that failed ([`mutation`]).
const PAST_WALL_TIME: &str = "the test ran past its wall-time limit";

/// The field naming the `Core\Test` method a failed assertion called.
const ASSERTION: &str = "assertion";

/// The line a failure record prints in every report.
fn message_of(record: &Record) -> &str {
    record
        .envelope
        .message
        .as_ref()
        .map_or("it failed", Rendered::as_str)
}

/// One test's place in the run: what was called, how it came out, how long it
/// took. The machine formats render a list of these at the end; the plaintext
/// one renders each as it is reported.
struct Case {
    class: String,
    method: String,
    /// Where the method carrying `#[Test]` is declared, resolved while the run
    /// still holds the source map.
    ///
    /// It is resolved here rather than rendered from a [`Span`](nvs_diagnostics::Span)
    /// at the end, so every format renders a location it already has, and none
    /// of them needs the source map. `None` only for a span the map does not
    /// hold, which a test the compiler collected cannot have.
    declared: Option<Declared>,
    outcome: Outcome,
    elapsed: Duration,
    /// `rule:testing/inline-snapshots`'s inline snapshots this test produced a different rendering for,
    /// carried out of the isolate beside the verdict because that is the only
    /// place they exist — the child's context is gone by the time `--update`
    /// runs. Always collected and never reported: what reads it is
    /// [`update_snapshots`], and only when the run was asked to.
    snapshots: Vec<nvs_runtime::SnapshotMismatch>,
    /// The statement numbers this test reached, in any attempt, or `None`
    /// when the run has no coverage flag. [`json_document`] turns them into
    /// file lines.
    reached: Option<Vec<usize>>,
    /// A `#[Bench]`'s rows, or `None` for a test and for a bench that did not run.
    rows: Option<Rows>,
}

/// `rule:testing/bench-counters`' rows for one `#[Bench]`: the work its call did, and how long
/// it took. Every count is the same on every run of the same program, and `wall` is not.
///
/// The counts are taken on the bench's own isolate and around the call alone, so the isolate's
/// own setup is in no bench's rows. A task the bench spawns counts its statements and calls on a
/// context of its own, which these do not include, as `nvs run --count` does not. **What it
/// spends:** these six words per bench, in the report.
#[derive(Clone, Copy, Debug)]
struct Rows {
    statements: u64,
    calls: u64,
    allocations: usize,
    bytes: usize,
    gc: u64,
    /// Advisory: it changes from run to run, so nothing may gate on it.
    wall: Duration,
}

impl Rows {
    /// The five counts, in the order every report prints them.
    fn counts(self) -> (u64, u64, usize, usize, u64) {
        (
            self.statements,
            self.calls,
            self.allocations,
            self.bytes,
            self.gc,
        )
    }

    /// The counts as `nvs run --count` prints them, then the advisory wall time.
    fn line(self) -> String {
        format!(
            "count: statements={} calls={} allocations={} bytes={} gc={}, wall {:.3} ms (advisory)",
            self.statements,
            self.calls,
            self.allocations,
            self.bytes,
            self.gc,
            millis(self.wall)
        )
    }
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
///
/// `coverage` names the line-coverage files to write after the suite
/// (`crate::coverage` owns how the counts are taken). Asking for none leaves
/// the run exactly as it is without the flags.
///
/// `extensions` is the set the suite's thread installed its host over, and
/// each of the run's cores installs a host of its own over it.
pub(crate) fn run(
    checked: crate::Checked,
    snapshot: &nvs_config::Snapshot,
    extensions: Option<Arc<nvs_ext::load::Set>>,
    format: Format,
    select: &Selection,
    flags: Flags,
    coverage: &crate::coverage::Requested,
) -> ExitCode {
    let Flags {
        update,
        list,
        jobs,
        show_output,
        shared_worker: _,
        fail_fast,
        rerun_failed: _,
        shuffle,
        record_failed,
        mutate,
        profile,
    } = flags;
    if mutate && format == Format::Junit {
        // `rule:testing/mutation-testing`: JUnit has an element for a test and none for a mutant.
        eprintln!(
            "error: `--mutate` has no JUnit document. Use `--format json` or the default format"
        );
        return ExitCode::FAILURE;
    }
    // Answered before the compile below, rather than beside the run: what
    // discovery reads is the checked program's own test table, so a listing
    // costs no lowering and can say nothing about how a test would come out.
    if list {
        let listed = listing(&checked, select);
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
    // `--mutate` runs the suite with coverage first, to find the tests each mutant needs.
    let measured = coverage.any() || mutate;
    let (unit, probes) = match compile(&checked, cache.as_ref(), measured) {
        Ok(both) => both,
        Err(error) => {
            crate::report_internal(error);
            return ExitCode::FAILURE;
        }
    };
    let stmts = match mutate {
        true => probes.stmt_spans().to_vec(),
        false => Vec::new(),
    };
    // Resolved to file lines once, before the suite runs.
    let sites = crate::coverage::Sites::of(probes, &checked.map);

    // The module doc owns why a machine format sends the suite's own output
    // to stderr: stdout is the document, and nothing else may be written to it.
    let mut ctx = match format {
        Format::Human => nvs_runtime::Ctx::stdout(),
        Format::Json | Format::Junit => nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Stderr),
    };
    unit.install_in(&mut ctx);
    // The run's table. The class hooks and fixtures count into it directly, and
    // each test's own table is added to it when the test is reported
    // ([`Queue`]), so it holds the whole run when the suite returns.
    let hits = measured.then(|| {
        let hits = std::sync::Arc::new(sites.table());
        ctx.set_stmt_hits(Some(std::sync::Arc::clone(&hits)));
        ctx.set_debug_flags(
            ctx.debug_flags() | nvs_runtime::DebugFlags::COVERAGE | nvs_runtime::DebugFlags::BRANCH,
        );
        hits
    });
    // `--profile`: the suite's context and every test's time their calls into one file.
    let profiling = match profile {
        Some((wanted, out)) => {
            let name = checked
                .map
                .files()
                .next()
                .map_or_else(String::new, |file| file.name().to_owned());
            match crate::profile::install(&mut ctx, wanted, out.as_deref(), &name) {
                Ok(running) => Some(running),
                Err(line) => {
                    eprintln!("{line}");
                    return ExitCode::FAILURE;
                }
            }
        }
        None => None,
    };
    // The tree's snapshot goes on the suite's context before anything runs, as
    // `nvs run` puts it on the script's: every test's own parent context is
    // given this snapshot ([`Job::run`]), and its isolate asks that context for
    // its grants, so a `[capabilities]` block reaches a
    // test exactly as it reaches the program. A context with no snapshot denies
    // every grant, which would leave a `#[Test(server: true)]` unable to connect
    // to its own listener. It spends one copy of the snapshot per run.
    ctx.set_config(std::sync::Arc::new(snapshot.clone()));
    // The run's app folder is the tested file's, or the `root` of an
    // `[[app]]` block over it, and every test's context takes it.
    if let Some(folder) = snapshot.folder() {
        ctx.set_app_folder(folder.into());
    }

    // Read out before the suite runs: the rows are the compiler's and the run
    // cannot produce them. Nothing is read at all for a run that was not asked
    // to update, so an ordinary suite pays one `bool`.
    let snapshot_rows = match update {
        true => snapshot_sites(&checked),
        false => Vec::new(),
    };
    let checked = Rc::new(checked);
    let started = Instant::now();
    let (suite, mut ctx) = match run_suite_in_a_task(
        &unit,
        ctx,
        Rc::clone(&checked),
        &snapshot.config,
        extensions.clone(),
        Shape {
            format,
            jobs,
            show_output,
            fail_fast,
            shuffle,
            wall: None,
        },
        select,
    ) {
        Ok(both) => both,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let took = started.elapsed();
    let Suite {
        cases,
        counts,
        left_out,
        not_run,
        exited,
        class_failures,
    } = suite;
    if let Some(file) = &record_failed {
        record_failed_tests(file, &cases);
    }
    // Flushed before the summary: the report and what the class hooks printed
    // are written through the context, and printing the counts over the top of
    // them would report a run whose output had not been written yet.
    if let Err(error) = ctx.flush_output() {
        eprintln!("error: could not flush output: {error}");
        return ExitCode::FAILURE;
    }
    if let Some(running) = profiling
        && !running.finish(&mut ctx)
    {
        return ExitCode::FAILURE;
    }
    // `rule:testing/inline-snapshots`'s splice, after every test has run and before the summary is
    // printed, so a reader sees what was rewritten above the counts that made
    // it necessary. It is the one thing `nvs test` writes to a file, and it
    // does not touch the verdicts above.
    if update {
        update_snapshots(&snapshot_rows, &cases);
    }
    match format {
        Format::Human => println!(
            "\n  {} failed, {} passed, {} skipped, {} flaky{}{} in {:.0} ms",
            counts.failed,
            counts.passed,
            counts.skipped,
            counts.flaky,
            // `rule:testing/tags-and-shards-select-what-runs`: a selection that left every test out
            // is visible as one.
            match left_out {
                0 => String::new(),
                n => format!(", {n} left out"),
            },
            match not_run {
                0 => String::new(),
                n => format!(", {n} not run"),
            },
            took.as_secs_f64() * 1000.0
        ),
        // Written below, once a `--mutate` run's mutants are in.
        Format::Json => {}
        Format::Junit => print!("{}", junit_document(&cases, counts, took)),
    }
    // A machine format's documents keep their schema, so what they have no key for is said on
    // stderr, beside the rest of the suite's own output.
    if format != Format::Human && not_run > 0 {
        eprintln!("  {not_run} not run: `--fail-fast` stopped the run after a test failed");
    }
    if let Some(seed) = shuffle {
        let line = format!("  started in random order: --shuffle={seed} repeats this order");
        match format {
            Format::Human => println!("{line}"),
            Format::Json | Format::Junit => eprintln!("{line}"),
        }
    }
    if let Some(hits) = hits {
        match crate::coverage::write(coverage, &sites, &hits.counts(), &hits.edge_counts()) {
            // The summary line is the human format's. A machine format's
            // stdout is its document, and the files say the same thing.
            Ok(summary) if format == Format::Human => println!(
                "  {} of {} lines run ({:.1}%)",
                summary.hit,
                summary.lines,
                percent(summary.hit, summary.lines)
            ),
            Ok(_) => {}
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    let passed = exited.is_none() && counts.failed == 0 && class_failures.is_empty();
    let mutants = match (mutate, passed) {
        (false, _) => None,
        (true, false) => {
            eprintln!("  no mutant ran: a test failed before any change was made to the program");
            Some(Vec::new())
        }
        (true, true) => {
            let listed = mutation::list(&checked);
            let note = match listed.len() {
                1 => "\n  1 mutant: it runs the tests that reach its line".to_owned(),
                n => format!("\n  {n} mutants: each one runs the tests that reach its line"),
            };
            match format {
                Format::Human => println!("{note}"),
                Format::Json | Format::Junit => eprintln!("{note}"),
            }
            let new_ctx = || {
                let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
                ctx.set_config(std::sync::Arc::new(snapshot.clone()));
                if let Some(folder) = snapshot.folder() {
                    ctx.set_app_folder(folder.into());
                }
                ctx
            };
            let runs = mutation::Runs {
                ctx: &new_ctx,
                config: &snapshot.config,
                extensions,
                jobs,
                select,
            };
            match mutation::score(&checked, &listed, &cases, &stmts, &runs) {
                Ok(scored) => Some(scored),
                Err(error) => {
                    crate::report_internal(error);
                    return ExitCode::FAILURE;
                }
            }
        }
    };
    match format {
        Format::Human => {
            if let Some(scored) = &mutants {
                print!("{}", mutation::report(scored));
            }
        }
        Format::Json => print!(
            "{}",
            json_document(
                &cases,
                counts,
                took,
                measured.then_some(&sites),
                mutants.as_deref()
            )
        ),
        Format::Junit => {}
    }

    if let Some(code) = exited {
        // `exit(n)` ended the program the way `nvs run` reports one, and the
        // suite is over whatever the counts say.
        return ExitCode::from(u8::try_from(code & 0xFF).unwrap_or(0));
    }
    if counts.failed == 0 && class_failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The file in the data folder that keeps the tests the last run of the program at `entry`
/// failed, or `None` when there is no data folder to write in.
///
/// `rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle` keys it by the entry path: its
/// name is a hash of the canonical path, so two programs never share one and a path's own
/// characters never reach a file name.
pub(crate) fn failed_tests_file(entry: &std::path::Path) -> Option<PathBuf> {
    let folder = nvs_config::data::current()?;
    let canonical = std::fs::canonicalize(entry).unwrap_or_else(|_| entry.to_path_buf());
    let hash = blake3::hash(canonical.to_string_lossy().as_bytes()).to_hex();
    Some(
        folder
            .failed_tests()
            .join(format!("{}.txt", hash.get(..32).unwrap_or(&hash))),
    )
}

/// The tests `file` names, one `Class::label` per line, or `None` when no run wrote it or the
/// last run failed nothing.
pub(crate) fn read_failed_tests(file: &std::path::Path) -> Option<BTreeSet<String>> {
    let text = std::fs::read_to_string(file).ok()?;
    let names: BTreeSet<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    (!names.is_empty()).then_some(names)
}

/// Writes the run's failed tests to `file`, replacing what the last run wrote. A run that failed
/// nothing writes an empty file. A write that fails is a warning, never a verdict.
fn record_failed_tests(file: &std::path::Path, cases: &[Case]) {
    let mut text = String::new();
    for case in cases {
        if matches!(case.outcome, Outcome::Failed(_) | Outcome::Exited(_)) {
            text.push_str(&format!("{}::{}\n", case.class, case.method));
        }
    }
    let written = file
        .parent()
        .map_or(Ok(()), nvs_config::data::create_private_root)
        .and_then(|()| std::fs::write(file, text));
    if let Err(error) = written {
        eprintln!(
            "warning: could not record the failed tests in {}: {error}",
            file.display()
        );
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
/// has no calling task to put children under (`rule:concurrency/a-child-belongs-to-the-calling-task`), and `rule:testing/task-tree-and-virtual-clock` has no
/// tree to read a leftover off. One task and not one per test: the tests are a
/// *suite*, and each test's own isolate is a child of it.
///
/// `checked` and the unit are shared rather than borrowed, because a task's
/// body outlives the call that spawns it as far as the scheduler's signature
/// is concerned. A `--mutate` run keeps its own reference, and runs a suite per
/// mutant over the same checked program. What comes back out comes back by the two routes `nvs run`
/// already uses: the [`Suite`] through a cell the body captured, the `Ctx`
/// through the `Finished` the scheduler hands over, since it was moved into
/// the task rather than lent to it.
///
/// `Err` is the message to render — an internal failure of the run itself,
/// never a verdict about a test.
fn run_suite_in_a_task(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: nvs_runtime::Ctx,
    checked: Rc<crate::Checked>,
    config: &nvs_config::Config,
    extensions: Option<Arc<nvs_ext::load::Set>>,
    shape: Shape,
    select: &Selection,
) -> Result<(Suite, nvs_runtime::Ctx), String> {
    let mut sched = nvs_host::Scheduler::new();
    // `rule:testing/in-process-request`'s unit under test, built before
    // `checked` is moved into the task below. A `Core\Test` member reaches it
    // the way a `spawn script` reaches the resolver, so the two guards nest
    // rather than either one being a special case; each of the run's cores
    // installs the same two for its whole life ([`Seams`]).
    let under_test = Arc::new(UnderTest::new(unit, &checked));
    // A test's own isolate is built by this crate directly, but a `spawn script`
    // *inside* a test goes through the seam and needs the same resolver
    // `nvs run` installs — one per run, so two tests spawning one path share
    // the compiled unit (`crate::script`). It reads the tree's own `[opcache]`,
    // because the snapshot is resolved above the suite's compile and
    // `rule:config/the-extension-set-is-in-every-unit-key`'s environment digest is half of every key this resolver
    // writes: a default one would file a unit under an environment this run is
    // not in. Nothing edits a file mid-suite, so the revalidation policy in it
    // chooses nothing.
    let compiler = Arc::new(crate::script::Compiler::new(config));
    // The cores the tests run on, started at the first test posted and joined
    // when the last owner below drops them. Each runs inside the same two
    // seams as this thread, so a test reads this run's unit and compiler on
    // whichever core it lands. Each also installs an extension host of its
    // own over the run's loaded set, since a host is its thread's alone: the
    // first-party components are always loaded, so any test may call one.
    // A test's calls run in its own tree, so its budget is its own.
    let cores = {
        let (under_test, compiler) = (Arc::clone(&under_test), Arc::clone(&compiler));
        Arc::new(nvs_host::worker::Cores::new(
            nvs_host::worker::bound(),
            move |core: &mut dyn FnMut()| {
                let calling = extensions.as_ref().map(|set| {
                    nvs_runtime::extension::install(Rc::new(crate::extensions::Tested(Arc::clone(
                        set,
                    ))))
                });
                nvs_runtime::script::scoped(&*compiler, || {
                    nvs_runtime::inproc::scoped(&*under_test, core);
                });
                drop(calling);
            },
        ))
    };
    let filed: Rc<RefCell<Option<Suite>>> = Rc::new(RefCell::new(None));
    let collected = Rc::clone(&filed);
    let suite_unit = Arc::clone(unit);
    let seams = Seams {
        shape,
        cores: Arc::clone(&cores),
    };
    // Owned for the same reason `checked` is: the body outlives this call as
    // far as the scheduler's signature is concerned.
    let select = select.clone();
    let root = sched.spawn(ctx, nvs_runtime::TaskRoot::Request, move |ctx| {
        *collected.borrow_mut() = Some(run_suite(&suite_unit, ctx, &checked, seams, &select));
    });

    // The reactor is what a parked task is woken by, and a test that sleeps or
    // reads a socket parks exactly as any other program does — `run_until_idle`
    // refuses a scheduler with no reactor on its thread.
    let reactor = nvs_host::Reactor::new()
        .map_err(|error| format!("could not start the reactor: {error}"))?;
    let installed = nvs_host::reactor::install(reactor);
    let ran = nvs_runtime::script::scoped(&*compiler, || {
        nvs_runtime::inproc::scoped(&*under_test, || nvs_host::run_until_idle(&mut sched))
    });
    drop(installed);
    // Joins the run's cores. Every test has been reported unless the scheduler
    // stopped, and a test still running on a core is cancelled first.
    drop(cores);
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

/// The program `Core\Test::request` answers a synthetic request with —
/// `rule:testing/in-process-request`, and the far side of [`nvs_runtime::inproc::Answering`].
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
    /// which is what makes `rule:testing/in-process-request`'s "the real chain" true rather than a mock.
    /// Shared atomically, because every core a test runs on reaches this seam.
    unit: Arc<nvs_codegen::Unit>,
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
    pub(crate) fn new(unit: &Arc<nvs_codegen::Unit>, checked: &crate::Checked) -> Self {
        Self {
            unit: Arc::clone(unit),
            routes: std::sync::Arc::new(crate::runtime_routes(&checked.exprs)),
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
        // `rule:routing/a-gate-answers-around-every-handler`'s refusal mode, for
        // the one door refusal a synthetic request can meet: its verb is a
        // `Core\Http\Method` case and its body is no stream, so only the CSRF
        // check is taken, with [`Door`]'s inputs and as `answer_on_the_wire`
        // takes it. A unit that does not dispatch is answered as before.
        refuse_a_forged_write(ctx, &self.routes, &mut inbound);
        let unit = Arc::clone(&self.unit);
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
/// is this `Arc` and the program's Rust closure each child holds a clone of, on
/// whichever core the child runs.
///
/// `Err` is the message [`crate::report_internal`] renders; a compile that
/// fails ends the run rather than any one test.
///
/// `cache` is `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact cache, or [`None`] for a run that consults
/// none — every way it can fail to answer is a cold compile and nothing a
/// verdict can see, which is why the [`crate::cache::Provenance`] this drops is
/// dropped rather than reported.
///
/// With `coverage`, it also returns the program's [`crate::coverage::Probes`],
/// which a unit loaded from the cache numbers the same way; without it, they
/// are empty and nothing is read for them.
fn compile(
    checked: &crate::Checked,
    cache: Option<&crate::cache::Cache>,
    coverage: bool,
) -> Result<(Arc<nvs_codegen::Unit>, crate::coverage::Probes), String> {
    let files = checked.program_files();
    let program = nvs_ir::lower::lower_program_with_tests(
        nvs_ir::lower::ENTRY_SCRIPT_LABEL,
        &files,
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    );
    let probes = match coverage {
        true => crate::coverage::Probes::of(&program),
        false => crate::coverage::Probes::default(),
    };
    crate::cache::unit_for(&program, crate::cache::test_program_digest(&files), cache)
        .map(|(unit, _)| (Arc::new(unit), probes))
        .map_err(|error| error.to_string())
}

/// `hit` as a percentage of `of`, and `100` for a report with no lines.
fn percent(hit: usize, of: usize) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a line count is far below 2^52, and the figure is printed to one decimal"
    )]
    match of {
        0 => 100.0,
        _ => hit as f64 * 100.0 / of as f64,
    }
}

/// What `--filter`, `--tag`, `--skip-tag` and `--shard` select of a program's tests
/// (`rule:testing/tags-and-shards-select-what-runs`). A test runs when every one of them selects it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Selection {
    pub(crate) filter: Option<String>,
    /// `--tag`: a test runs when it has one of these, or when this is empty.
    pub(crate) tags: Vec<String>,
    /// `--skip-tag`: a test with any of these does not run.
    pub(crate) skip_tags: Vec<String>,
    pub(crate) shard: Option<nvs_test::Shard>,
    /// `--rerun-failed`: the `Class::label` of each test the last run failed, or `None` to run
    /// every test the other four select.
    pub(crate) only: Option<BTreeSet<String>>,
    /// `--bench`: the `#[Bench]` methods run and no `#[Test]` does. Without it, no bench runs
    /// (`rule:testing/bench-counters`).
    pub(crate) bench: bool,
}

impl Selection {
    /// The selection `--filter` alone makes.
    #[cfg(test)]
    fn by_name(filter: Option<&str>) -> Self {
        Self {
            filter: filter.map(ToOwned::to_owned),
            ..Self::default()
        }
    }

    /// Whether `call`, of `class`, runs. Its shard is its method's, so every data row of a test
    /// is in the part the test is in.
    fn selects(&self, class: &str, call: &Invocation<'_>) -> bool {
        let tags = tags_of(call.case);
        self.runs_its_kind(call)
            && selected(self.filter.as_deref(), class, &call.label)
            && (self.tags.is_empty() || self.tags.iter().any(|tag| tags.contains(&tag.as_str())))
            && !self
                .skip_tags
                .iter()
                .any(|tag| tags.contains(&tag.as_str()))
            && self
                .shard
                .is_none_or(|shard| shard.holds(&format!("{class}::{}", call.case.method)))
            && self
                .only
                .as_ref()
                .is_none_or(|only| only.contains(&format!("{class}::{}", call.label)))
    }

    /// Whether `call` is a bench under `--bench`, or a test without it. A call of the other
    /// kind is not counted as left out, because no selection could have run it.
    fn runs_its_kind(&self, call: &Invocation<'_>) -> bool {
        call.case.bench == self.bench
    }
}

/// The switches of [`run`] that change what a run does rather than which
/// format it reports in: `--update`, `--list`, `--jobs`, `--show-output` and
/// `rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle`'s three.
#[derive(Clone)]
pub(crate) struct Flags {
    pub(crate) update: bool,
    pub(crate) list: bool,
    /// How many tests run at once (`rule:testing/tests-run-across-every-core`).
    /// `1` runs every test on the suite's own thread, which is what a program
    /// that loads an extension needs: the extension host is that thread's.
    pub(crate) jobs: usize,
    /// `--show-output`: each test writes to the terminal as it runs, and
    /// nothing is captured.
    pub(crate) show_output: bool,
    /// `--shared-worker`: each `.nvst` case also runs on one shared worker
    /// ([`crate::shared`]). A program's `#[Test]` run refuses it.
    pub(crate) shared_worker: bool,
    /// `--fail-fast`: no test starts once one has been reported failed.
    pub(crate) fail_fast: bool,
    /// `--rerun-failed`, which `crate::run_program_tests` turns into [`Selection::only`].
    pub(crate) rerun_failed: bool,
    /// `--shuffle`'s seed, drawn already when the flag named none.
    pub(crate) shuffle: Option<u64>,
    /// The file the run writes its failed tests to ([`failed_tests_file`]), or `None` to write
    /// nothing.
    pub(crate) record_failed: Option<PathBuf>,
    /// `--mutate` (`rule:testing/mutation-testing`): after the run, each mutant runs the tests
    /// that reach it ([`mutation`]).
    pub(crate) mutate: bool,
    /// `--profile` and `--profile-out` ([`crate::profile`]).
    pub(crate) profile: Option<(crate::profile::Format, Option<PathBuf>)>,
}

/// `--jobs`'s default: the machine's hardware threads, as `nvs_test` counts
/// them for `.nvst` cases.
pub(crate) fn default_jobs() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

/// How a run reports and spreads its tests: the part of [`run`]'s arguments
/// that crosses into the suite's task.
#[derive(Clone, Copy)]
struct Shape {
    format: Format,
    jobs: usize,
    show_output: bool,
    /// `--fail-fast` ([`Flags::fail_fast`]).
    fail_fast: bool,
    /// `--shuffle`'s seed ([`Flags::shuffle`]).
    shuffle: Option<u64>,
    /// The wall-time limit a test with none of its own runs under, or `None` for no limit. Only
    /// a mutant's run sets it ([`mutation`]).
    wall: Option<Duration>,
}

/// What [`run_suite`] needs beside [`Shape`]: the cores it posts each test to.
///
/// They are this run's own ([`nvs_host::worker::Cores`]), and each runs its
/// whole life under the two seams `run_suite_in_a_task` installs on the suite's
/// thread. A test parks while its isolate runs, so a seam installed per test
/// would be read by every other test on that core, including another run's in
/// the same process.
struct Seams {
    shape: Shape,
    cores: Arc<nvs_host::worker::Cores>,
}

/// A whole run's verdict, before any of `rule:testing/report-formats`'s renderings has been chosen —
/// which is also the shape this module's own tests read, a rendering being the
/// one thing they are not about.
struct Suite {
    cases: Vec<Case>,
    counts: Counts,
    /// How many tests the selection left out, which no field of [`Self::counts`] counts.
    left_out: usize,
    /// How many selected tests did not start because `--fail-fast` stopped the run.
    not_run: usize,
    /// The code an `exit(n)` in a test or a fixture ended the run with.
    exited: Option<i64>,
    /// Each `#[AfterClass]` that threw, as its class and the message. One
    /// fails the run, and is reported against the class rather than a test
    /// (`rule:testing/class-hooks`).
    class_failures: Vec<(String, String)>,
}

/// One test as each of its attempts runs it, owned so that it can move into
/// the isolate's program.
#[derive(Clone)]
struct Attempted {
    class: String,
    method: String,
    /// The name the report prints, with `#N` for a data row.
    label: String,
    /// `rule:testing/after-each-test`'s hook, called after every attempt.
    after_each: Option<nvs_types::test_hooks::HookMethod>,
    /// The generators of a `#[Property]` method, which [`run_property`] calls once per
    /// generated input set. `None` for a `#[Test]` method.
    property: Option<Arc<nvs_types::property::PropertyCase>>,
    /// Whether the method is a `#[Bench]`, which [`judge`] passes with no assertion.
    bench: bool,
}

/// Runs every `#[Test]` and `#[Property]` `checked` declares that `filter` selects, in `rule:testing/runner-is-strict`'s
/// order, as many at once as `--jobs` allows, reporting each in declaration
/// order under [`Format::Human`] and only collecting under the other two.
///
/// The class loop runs on the suite's context: `#[BeforeClass]`, the fixtures,
/// then one [`Job`] per selected test, posted to another core. What it cannot
/// post — a skip, a failed setup, a fixture that cannot cross — enters the
/// [`Queue`] with its outcome already decided. The queue is what reports, and
/// what runs `#[AfterClass]` once the class's last test has been reported.
fn run_suite(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    checked: &crate::Checked,
    seams: Seams,
    select: &Selection,
) -> Suite {
    let Seams { shape, cores } = seams;
    // Counted before anything runs, so an `exit(n)` that ends the run early does not change it.
    let left_out = checked
        .exprs
        .test_classes()
        .into_iter()
        .map(|class| {
            let tests = checked.exprs.tests(class).unwrap_or_default();
            let properties = checked.exprs.properties(class).unwrap_or_default();
            let stand_ins: Vec<_> = properties.iter().map(stand_in).collect();
            class_calls(tests, properties, &stand_ins)
                .iter()
                .filter(|call| select.runs_its_kind(call) && !select.selects(class, call))
                .count()
        })
        .sum();
    // The run's coverage table, or `None` when no coverage flag was given. A
    // test counts into a table of its own, sized from this one, and the queue
    // adds it here when the test is reported. A fixture is built before its
    // class's first test, so it is no test's. It is also the run table, which
    // the isolates a hook or a fixture spawns are added into at their join.
    let hits = ctx.stmt_hits().cloned();
    ctx.set_run_hits(hits.clone());
    let shared = Arc::new(Shared {
        // `rule:routing/matched-once-before-the-handler`'s table, crossed once for the whole suite rather than per
        // test: it is a product of the compile every isolate already shares, and
        // [`TestServer`] is the only thing that reads it — a run whose tests declare
        // no `server:` pays one walk of the program's `#[Route]` rows and nothing
        // else. `UnderTest` builds its own for `rule:testing/in-process-request`'s first mechanism.
        routes: Arc::new(crate::runtime_routes(&checked.exprs)),
        config: ctx.config().map(|config| Arc::clone(config.snapshot())),
        folder: ctx
            .config()
            .and_then(nvs_config::Request::folder)
            .map(Arc::clone),
        coverage: hits.as_ref().map(|hits| {
            (
                hits.counts().len(),
                hits.edge_counts().len(),
                hits.lines().map(Arc::clone),
            )
        }),
        profile: ctx.run_profile().cloned(),
        format: shape.format,
        show_output: shape.show_output,
        wall: shape.wall,
        watchdog: nvs_host::ThreadClock::current()
            .is_some()
            .then(nvs_host::Watchdog::new),
    });
    let mut queue = Queue::new(unit, ctx, shape, hits, cores);
    let mut order = shape.shuffle.map(Order);
    let mut not_run = 0;
    for class in checked.exprs.test_classes() {
        let tests = checked.exprs.tests(class).unwrap_or_default();
        let properties = checked.exprs.properties(class).unwrap_or_default();
        let stand_ins: Vec<_> = properties.iter().map(stand_in).collect();
        // The selection is made **before** the class is announced or its
        // fixtures are built: a `--filter` that reaches none of a class's
        // tests must cost that class's `#[Fixture]` nothing, or filtering down
        // to one fast test still pays for every expensive setup in the
        // program. A class with nothing selected is not reported either — the
        // header would name a class the run said nothing about.
        let calls: Vec<Invocation<'_>> = class_calls(tests, properties, &stand_ins)
            .into_iter()
            .filter(|call| select.selects(class, call))
            .collect();
        if calls.is_empty() {
            continue;
        }
        // `--fail-fast` after a failure: the class's setup does not run either.
        if queue.stopped() {
            not_run += calls.len();
            continue;
        }
        // `rule:testing/class-hooks`'s order: `#[BeforeClass]`, the fixtures,
        // the tests, then `#[AfterClass]`, the hooks and fixtures in the
        // parent. A setup that threw runs no test and builds no fixture, and
        // `#[AfterClass]` still runs, because the setup may have left something
        // to remove. The header enters the queue first; what the two print is
        // captured and enters behind it, because an earlier class's tests may
        // still be reporting. Under `--show-output` it streams instead.
        if queue.push(Entry::Class(class.to_owned())) {
            break;
        }
        let capture = !shape.show_output;
        if capture {
            queue.ctx.begin_capture();
        }
        let hooks = checked.exprs.test_hooks(class);
        let setup = hooks
            .and_then(|hooks| hooks.before_class.as_ref())
            .and_then(|hook| run_class_hook(unit, queue.ctx, class, hook, "#[BeforeClass]", None));
        // `rule:testing/fixtures`'s "built once, in the parent" is this call: one set per class,
        // built before its first test and dropped after its last, so a fixture
        // is not rebuilt per test and not kept past the class that declared it.
        let mut fixtures = nvs_runtime::Fixtures::new();
        let unbuilt = match setup {
            Some(_) => None,
            None => build_fixtures(unit, queue.ctx, class, checked, tests, &mut fixtures),
        };
        if capture {
            let printed = queue.ctx.end_capture().unwrap_or_default();
            if queue.push(Entry::Printed(printed)) {
                break;
            }
        }
        if let Some(FixtureFailure::Exited(code)) = setup {
            // Every earlier class finishes and is reported first: the queue
            // reaches this entry in declaration order.
            queue.push(Entry::Exit(code));
            break;
        }
        // `--shuffle` changes the order the tests start in. [`Held`] puts each one back in
        // declaration order before it enters the queue, so the report's order does not change.
        let mut starts: Vec<usize> = (0..calls.len()).collect();
        if let Some(order) = &mut order {
            order.shuffle(&mut starts);
        }
        let mut held = Held::new(calls.len());
        let mut calls: Vec<Option<Invocation<'_>>> = calls.into_iter().map(Some).collect();
        for at in starts {
            let Some(call) = calls.get_mut(at).and_then(Option::take) else {
                continue;
            };
            if queue.stopped() {
                not_run += 1;
                held.put(at, None);
                if held.flush(&mut queue) {
                    break;
                }
                continue;
            }
            let (case, label, row) = (call.case, call.label, call.row);
            let test = Attempted {
                class: class.to_owned(),
                method: case.method.clone(),
                label: label.clone(),
                after_each: hooks.and_then(|hooks| hooks.after_each_test.clone()),
                property: call.property.map(|property| Arc::new(property.clone())),
                bench: case.bench,
            };
            let decided = match setup.as_ref().or(unbuilt.as_ref()) {
                Some(FixtureFailure::Threw(message)) if setup.is_some() => {
                    Some(Outcome::failed(message))
                }
                // A fixture that would not build is reported against every
                // test that asked for one, rather than against the class: a
                // test is what a report has a line for, and a suite that lost
                // a whole class silently is what `rule:testing/runner-is-strict` is written against.
                Some(FixtureFailure::Exited(code)) => Some(Outcome::Exited(*code)),
                Some(FixtureFailure::Threw(message)) if !fixtures_needed(case).is_empty() => {
                    Some(Outcome::failed(message))
                }
                _ => None,
            };
            let state = match decided {
                Some(outcome) => State::Decided(outcome),
                None => match Job::prepare(unit, test, case, row, &fixtures, &shared) {
                    Ok(job) => {
                        held.make_room(queue.running, shape.jobs);
                        queue.start(job, shape.jobs)
                    }
                    Err(outcome) => State::Decided(outcome),
                },
            };
            held.put(
                at,
                Some(Box::new(Pending {
                    class: class.to_owned(),
                    label,
                    declared: declared_at(case.span, &checked.map),
                    state,
                })),
            );
            if held.flush(&mut queue) {
                break;
            }
        }
        drop(held);
        if queue.ended() {
            break;
        }
        if queue.push(Entry::ClassEnd {
            name: class.to_owned(),
            fixtures,
            after: hooks.and_then(|hooks| hooks.after_class.clone()),
        }) {
            break;
        }
    }
    Suite {
        left_out,
        not_run,
        ..queue.finish()
    }
}

/// `--shuffle`'s start order: a SplitMix64 stream from the run's seed, so one seed gives one
/// order on every machine and in every version that keeps this function.
struct Order(u64);

impl Order {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Shuffles `items` in place, Fisher–Yates.
    fn shuffle(&mut self, items: &mut [usize]) {
        for at in (1..items.len()).rev() {
            let bound = u64::try_from(at + 1).unwrap_or(u64::MAX);
            let other = usize::try_from(self.next() % bound).unwrap_or(at);
            items.swap(at, other);
        }
    }
}

/// One class's tests between the moment each one starts and the moment it enters the
/// [`Queue`]. `--shuffle` starts them out of order, and the queue takes them in declaration
/// order, so a test that started early waits here for the ones written above it.
///
/// **What it spends** (`rule:programs/memory-priority`): one slot per selected test of the
/// class, and the results of finished tests waiting behind one that has not started. Without
/// `--shuffle` every test enters the queue as it starts, and nothing waits here.
struct Held {
    slots: Vec<Slot>,
    /// The first slot not yet in the queue.
    next: usize,
}

enum Slot {
    /// Not started yet.
    Waiting,
    /// `--fail-fast` stopped the run before it started.
    NotRun,
    Started(Box<Pending>),
}

impl Held {
    fn new(tests: usize) -> Self {
        Self {
            slots: (0..tests).map(|_| Slot::Waiting).collect(),
            next: 0,
        }
    }

    /// Files the test at `at`, or `None` for one that will not run.
    fn put(&mut self, at: usize, pending: Option<Box<Pending>>) {
        if let Some(slot) = self.slots.get_mut(at) {
            *slot = pending.map_or(Slot::NotRun, Slot::Started);
        }
    }

    /// Moves every test at the front that has started into `queue`, in declaration order.
    /// `true` once an `exit(n)` has ended the run.
    fn flush(&mut self, queue: &mut Queue<'_>) -> bool {
        while let Some(slot) = self.slots.get_mut(self.next) {
            match std::mem::replace(slot, Slot::NotRun) {
                Slot::Waiting => {
                    *slot = Slot::Waiting;
                    return false;
                }
                Slot::NotRun => {}
                Slot::Started(pending) => {
                    if queue.push(Entry::Test(pending)) {
                        self.next += 1;
                        return true;
                    }
                }
            }
            self.next += 1;
        }
        false
    }

    /// Waits for tests held here until fewer than `jobs` run at once, beside the `running`
    /// ones the queue counts. Only a shuffled run holds a running test.
    fn make_room(&mut self, running: usize, jobs: usize) {
        if jobs <= 1 {
            return;
        }
        let mut held = self
            .slots
            .iter()
            .filter(|slot| matches!(slot, Slot::Started(pending) if matches!(pending.state, State::Running(_))))
            .count();
        for slot in &mut self.slots {
            if running + held < jobs {
                break;
            }
            if let Slot::Started(pending) = slot {
                let state = std::mem::replace(&mut pending.state, State::Decided(Outcome::Passed));
                pending.state = match state {
                    State::Running(posted) => {
                        held -= 1;
                        State::Ran(collect(posted))
                    }
                    other => other,
                };
            }
        }
    }
}

impl Drop for Held {
    /// An `exit(n)` ended the run, and the tests still held are abandoned.
    fn drop(&mut self) {
        for slot in std::mem::take(&mut self.slots) {
            if let Slot::Started(pending) = slot {
                abandon(Entry::Test(pending));
            }
        }
    }
}

/// What every test of one run shares on whichever core it runs: read-only, and
/// built once per run.
struct Shared {
    routes: Arc<nvs_runtime::routes::Routes>,
    /// The suite's configuration snapshot, which each test's own parent
    /// context is given, or `None` for a suite nobody configured.
    config: Option<Arc<nvs_config::Snapshot>>,
    /// The suite's app folder, which each test's context is given beside
    /// [`Self::config`].
    folder: Option<Arc<std::path::Path>>,
    /// The sizes of the run's coverage table, statements then edges, and where
    /// each statement is written, or `None` for a run with no coverage flag.
    /// Each test's own table is located by the same lines.
    coverage: Option<(usize, usize, Option<Arc<nvs_runtime::StmtLines>>)>,
    /// The run's profiler under `--profile`, which each test's context times its calls into.
    profile: Option<Arc<nvs_runtime::Profiler>>,
    format: Format,
    show_output: bool,
    /// [`Shape::wall`].
    wall: Option<Duration>,
    /// The thread that charges each test's tree for the CPU time it runs and raises its
    /// `[limits] cpu_time` ceiling, as it does for `nvs run`. `None` on a platform with no
    /// per-thread clock, where nothing could be charged. Each [`Job`] registers the thread it
    /// runs on and publishes its own tree, so a test's CPU time, its extension calls' included,
    /// is its own and never the test's before it.
    ///
    /// **What it spends:** one thread for the run, which starts at the first registration, and
    /// one entry in its watched set per test in flight.
    watchdog: Option<nvs_host::Watchdog>,
}

/// One test on its way to the core that runs it: everything [`run_in_isolate`]
/// reads, owned, and each fixture it asks for as graph bytes.
///
/// **What it spends** (`rule:programs/memory-priority`): per test in flight, a
/// parent context of its own beside its isolate, its fixtures as bytes and then
/// as decoded copies, its captured output and, under a coverage flag, a
/// coverage table the size of the run's. All of it is released when the test
/// is reported, and a passing test's output is never kept past that. That is
/// O(`--jobs`), plus the results of finished tests waiting behind an earlier
/// one that is still running.
struct Job {
    unit: Arc<nvs_codegen::Unit>,
    test: Attempted,
    case: nvs_types::testing::TestCase,
    row: Option<Vec<Option<ConstArg>>>,
    fixtures: Vec<Vec<u8>>,
    shared: Arc<Shared>,
}

/// What a [`Job`] sends back to the suite's core.
struct Ran {
    outcome: Outcome,
    snapshots: Vec<nvs_runtime::SnapshotMismatch>,
    elapsed: Duration,
    /// What the test, its `#[AfterEachTest]` hook and its isolate printed, or
    /// nothing under `--show-output`.
    output: Vec<u8>,
    /// The test's own statement and edge counts, under a coverage flag.
    coverage: Option<(Vec<u64>, Vec<u64>)>,
    /// A bench's rows ([`Case::rows`]).
    rows: Option<Rows>,
}

impl Ran {
    /// A test that never ran a line.
    fn decided(outcome: Outcome) -> Self {
        Self {
            outcome,
            snapshots: Vec::new(),
            elapsed: Duration::ZERO,
            output: Vec::new(),
            coverage: None,
            rows: None,
        }
    }
}

impl Job {
    /// The job for one test, or the outcome it has without running: a skip,
    /// or a fixture that cannot cross. The fixtures are encoded here, on the
    /// parent's core, because the set that owns them is the parent's.
    fn prepare(
        unit: &Arc<nvs_codegen::Unit>,
        test: Attempted,
        case: &nvs_types::testing::TestCase,
        row: Option<&[Option<ConstArg>]>,
        fixtures: &nvs_runtime::Fixtures,
        shared: &Arc<Shared>,
    ) -> Result<Self, Outcome> {
        if let Some(reason) = skip_reason(case) {
            return Err(Outcome::Skipped(reason));
        }
        let needed: Vec<String> = fixtures_needed(case)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let encoded = match nvs_runtime::CrossedFixtures::encode(fixtures, &needed) {
            Some(Ok(encoded)) => encoded,
            // A fixture the parent built that has no meaning inside an isolate —
            // a callable, an object over a host handle. It is reported against the
            // test that asked for it, which is where the name in the message
            // means something, and nothing has run.
            Some(Err(refused)) => {
                return Err(Outcome::failed(format!(
                    "`{}::{}` asks for a `#[Fixture]` that cannot cross into its isolate: \
                     {refused}",
                    test.class, case.method
                )));
            }
            None => {
                return Err(Outcome::failed(format!(
                    "internal error: `{}::{}` asks for a `#[Fixture]` that was not built",
                    test.class, case.method
                )));
            }
        };
        Ok(Self {
            unit: Arc::clone(unit),
            test,
            case: case.clone(),
            row: row.map(<[Option<ConstArg>]>::to_vec),
            fixtures: encoded,
            shared: Arc::clone(shared),
        })
    }

    /// Runs the test on this thread, under a parent context built for it.
    ///
    /// The context is what the suite's would have been for this one test: the
    /// unit installed, the suite's configuration and a coverage table of its
    /// own. Its sink is a buffer, so what the child isolate prints through
    /// `Output::Inherit` is the test's captured output.
    ///
    /// The two seams are the thread's, not this call's: one of the run's own
    /// cores, or the suite's thread under `--jobs 1`, installs them for its
    /// whole life ([`Seams`]).
    fn run(self) -> Ran {
        let began = Instant::now();
        let Self {
            unit,
            test,
            case,
            row,
            fixtures,
            shared,
        } = self;
        let sink = match (shared.show_output, shared.format) {
            (false, _) => nvs_runtime::OutputSink::Buffer(Vec::new()),
            (true, Format::Human) => nvs_runtime::OutputSink::Stdout,
            (true, Format::Json | Format::Junit) => nvs_runtime::OutputSink::Stderr,
        };
        let mut ctx = nvs_runtime::Ctx::new(sink);
        unit.install_in(&mut ctx);
        if let Some(config) = &shared.config {
            ctx.set_config(Arc::clone(config));
        }
        if let Some(folder) = &shared.folder {
            ctx.set_app_folder(Arc::clone(folder));
        }
        // Armed here, on the context the test's class is constructed under, so the test's own
        // fixtures and hooks count toward it and the tests before it do not.
        let limit = match wall_limit(&case, &mut ctx, shared.wall) {
            Ok(limit) => limit,
            Err(outcome) => return Ran::decided(outcome),
        };
        let clock = crate::wall::start(ctx.safepoint_view());
        // `[limits] cpu_time`'s half, charged against this thread's clock from here until the
        // test is judged. A registration belongs to the thread that made it, so it is made here,
        // on the core the test runs on.
        let watched = shared.watchdog.as_ref().map(|watchdog| {
            let watched = watchdog.register_requests();
            watched.publish(ctx.safepoint_view());
            watched
        });
        let hits = shared.coverage.as_ref().map(|(stmts, edges, lines)| {
            let table = nvs_runtime::StmtHits::with_edges(*stmts, *edges);
            let hits = Arc::new(match lines {
                Some(lines) => table.located(Arc::clone(lines)),
                None => table,
            });
            // The test runs in an isolate of this context, which counts into a table of its own,
            // so this table is also the run table its join adds that one into.
            ctx.set_stmt_hits(Some(Arc::clone(&hits)));
            ctx.set_run_hits(Some(Arc::clone(&hits)));
            ctx.set_debug_flags(
                ctx.debug_flags()
                    | nvs_runtime::DebugFlags::COVERAGE
                    | nvs_runtime::DebugFlags::BRANCH,
            );
            hits
        });
        // Under `--profile` the test's calls are a track of their own in the run's one file,
        // named for the test.
        if let Some(profiler) = &shared.profile {
            ctx.set_run_profile(Some(Arc::clone(profiler)));
            ctx.set_profile_sink(Some(
                profiler.track(&format!("{}::{}", test.class, case.method)),
            ));
            ctx.set_debug_flags(ctx.debug_flags() | nvs_runtime::DebugFlags::PROFILE);
        }
        let receiving = ctx.class_table();
        let (outcome, snapshots, rows) =
            match nvs_runtime::CrossedFixtures::decode(&fixtures, receiving.as_ref()) {
                Ok(crossed) => run_in_isolate(
                    &unit,
                    &mut ctx,
                    &test,
                    &case,
                    row.as_deref(),
                    crossed,
                    &shared.routes,
                ),
                // Not reachable for bytes encoded against the same unit's
                // table, and reported rather than unwrapped: nothing has run.
                Err(refused) => (
                    Outcome::failed(format!(
                        "internal error: `{}::{}`'s `#[Fixture]` could not be read on the core \
                         the test runs on: {refused}",
                        test.class, case.method
                    )),
                    Vec::new(),
                    None,
                ),
            };
        drop(watched);
        drop(clock);
        let outcome = match limit {
            Some(limit)
                if ctx.safepoint_view().deadline_expired()
                    || ctx.safepoint_view().wall_time_stopped() =>
            {
                outcome.past_wall_time(&limit)
            }
            _ => outcome,
        };
        // A sink that is the terminal is flushed here, on the thread whose
        // writes it holds, and a failure to flush is nothing a verdict can see.
        drop(ctx.flush_output());
        Ran {
            outcome,
            snapshots,
            elapsed: began.elapsed(),
            output: ctx.take_buffered_output().unwrap_or_default(),
            coverage: hits.map(|hits| (hits.counts(), hits.edge_counts())),
            rows,
        }
    }
}

/// One entry of the [`Queue`], in the order the report prints them.
enum Entry {
    /// A class's header, before its `#[BeforeClass]` runs.
    Class(String),
    /// What a class's `#[BeforeClass]` and fixtures printed, captured.
    Printed(Vec<u8>),
    Test(Box<Pending>),
    /// After a class's last test: its fixtures, released here, then its
    /// `#[AfterClass]`.
    ClassEnd {
        name: String,
        fixtures: nvs_runtime::Fixtures,
        after: Option<nvs_types::test_hooks::HookMethod>,
    },
    /// An `exit(n)` in a class's `#[BeforeClass]`, which ends the run here.
    Exit(i64),
}

/// One test's place in the queue.
struct Pending {
    class: String,
    label: String,
    declared: Option<Declared>,
    state: State,
}

/// Where one test is.
enum State {
    /// Its outcome was known without running it.
    Decided(Outcome),
    /// Running on another core.
    Running(nvs_host::worker::Posted<Ran>),
    /// Run on the suite's own thread.
    Ran(Ran),
}

/// `rule:testing/tests-run-across-every-core`'s order, kept while tests run at
/// once: every class header, test, `#[AfterClass]` and setup `exit` enters in
/// declaration order, and leaves in that order to be reported.
///
/// An entry leaves as soon as everything ahead of it has, and it is not a test
/// still running; [`Self::start`] waits at the front while `--jobs` tests are
/// running, and [`Self::finish`] waits for all of them. So a later class's
/// `#[BeforeClass]` may run while an earlier class's tests still are, and
/// `#[AfterClass]` starts after its class's last test has been reported. An
/// `exit(n)` reported here abandons every test still running behind it.
struct Queue<'a> {
    unit: &'a nvs_codegen::Unit,
    ctx: &'a mut nvs_runtime::Ctx,
    format: Format,
    hits: Option<Arc<nvs_runtime::StmtHits>>,
    entries: VecDeque<Entry>,
    running: usize,
    cases: Vec<Case>,
    counts: Counts,
    /// The class being reported, its counts and the labels of its failed
    /// tests: `#[AfterClass]`'s `Core\Test\Summary`.
    class: nvs_stdlib::TestCounts,
    failed_tests: Vec<String>,
    exited: Option<i64>,
    class_failures: Vec<(String, String)>,
    /// The run's own cores, which [`Self::start`] posts each test to.
    cores: Arc<nvs_host::worker::Cores>,
    /// `--fail-fast`, which [`Self::stopped`] reads.
    fail_fast: bool,
}

impl<'a> Queue<'a> {
    fn new(
        unit: &'a nvs_codegen::Unit,
        ctx: &'a mut nvs_runtime::Ctx,
        shape: Shape,
        hits: Option<Arc<nvs_runtime::StmtHits>>,
        cores: Arc<nvs_host::worker::Cores>,
    ) -> Self {
        Self {
            unit,
            ctx,
            format: shape.format,
            fail_fast: shape.fail_fast,
            hits,
            cores,
            entries: VecDeque::new(),
            running: 0,
            cases: Vec::new(),
            counts: Counts {
                passed: 0,
                failed: 0,
                skipped: 0,
                flaky: 0,
            },
            class: nvs_stdlib::TestCounts::default(),
            failed_tests: Vec::new(),
            exited: None,
            class_failures: Vec::new(),
        }
    }

    /// Whether an `exit(n)` has ended the run.
    fn ended(&self) -> bool {
        self.exited.is_some()
    }

    /// Whether `--fail-fast` starts no more tests: a failed test has been reported. A failure
    /// still running or waiting behind an earlier test does not count yet, so under `--jobs 1`
    /// the run stops right after the first failure, and with more jobs a few tests later at most.
    fn stopped(&self) -> bool {
        self.fail_fast && self.counts.failed > 0
    }

    /// Starts `job`: posted to one of the run's cores once fewer than `jobs`
    /// tests are running, or run here under `--jobs 1` and when no core can be
    /// had.
    fn start(&mut self, job: Job, jobs: usize) -> State {
        if jobs <= 1 {
            return State::Ran(job.run());
        }
        while self.running >= jobs && !self.ended() {
            self.report_front();
        }
        match self.cores.post(move || job.run()) {
            Ok(posted) => State::Running(posted),
            Err(job) => State::Ran(job()),
        }
    }

    /// Adds `entry` behind the others and reports every entry at the front
    /// that is ready. `true` once an `exit(n)` has ended the run.
    fn push(&mut self, entry: Entry) -> bool {
        if self.ended() {
            abandon(entry);
            return true;
        }
        if let Entry::Test(pending) = &entry
            && matches!(pending.state, State::Running(_))
        {
            self.running += 1;
        }
        self.entries.push_back(entry);
        while self.entries.front().is_some_and(ready) && !self.ended() {
            self.report_front();
        }
        self.ended()
    }

    /// Reports every entry left, waiting for each test still running, and
    /// hands back the whole run.
    fn finish(mut self) -> Suite {
        while !self.entries.is_empty() && !self.ended() {
            self.report_front();
        }
        Suite {
            cases: self.cases,
            counts: self.counts,
            left_out: 0,
            not_run: 0,
            exited: self.exited,
            class_failures: self.class_failures,
        }
    }

    /// Takes the front entry out and reports it, waiting for it if it is a
    /// test still running. An `exit(n)` abandons everything behind it.
    fn report_front(&mut self) {
        let Some(entry) = self.entries.pop_front() else {
            return;
        };
        match entry {
            Entry::Class(name) => {
                if self.format == Format::Human {
                    say(self.ctx, &format!("  {name}\n"));
                }
                self.class = nvs_stdlib::TestCounts::default();
                self.failed_tests.clear();
            }
            Entry::Printed(printed) => drop(self.ctx.write_output(&printed)),
            Entry::Test(pending) => self.report_test(*pending),
            Entry::ClassEnd {
                name,
                fixtures,
                after,
            } => {
                // Released before `#[AfterClass]`, which may remove what a
                // fixture still has open.
                drop(fixtures);
                if let Some(hook) = after {
                    self.after_class(&name, &hook);
                }
            }
            Entry::Exit(code) => self.exited = Some(code),
        }
        if self.ended() {
            for entry in std::mem::take(&mut self.entries) {
                abandon(entry);
            }
            self.running = 0;
        }
    }

    /// Reports one test: its counts, its line, its output under a failure,
    /// and its coverage added to the run's.
    fn report_test(&mut self, pending: Pending) {
        let Pending {
            class,
            label,
            declared,
            state,
        } = pending;
        let ran = match state {
            State::Decided(outcome) => Ran::decided(outcome),
            State::Ran(ran) => ran,
            State::Running(posted) => {
                self.running -= 1;
                collect(posted)
            }
        };
        let Ran {
            outcome,
            snapshots,
            elapsed,
            output,
            coverage,
            rows,
        } = ran;
        let reached = self.hits.as_ref().map(|hits| match coverage {
            Some((counts, edges)) => {
                hits.add(&counts, &edges);
                crate::coverage::ran(&counts)
            }
            None => Vec::new(),
        });
        let failed = matches!(outcome, Outcome::Failed(_) | Outcome::Exited(_));
        match &outcome {
            Outcome::Passed => self.class.passed += 1,
            Outcome::Skipped(_) => self.class.skipped += 1,
            Outcome::Flaky { .. } => self.class.flaky += 1,
            Outcome::Failed(_) | Outcome::Exited(_) => {
                self.class.failed += 1;
                self.failed_tests.push(label.clone());
            }
        }
        match &outcome {
            Outcome::Passed => self.counts.passed += 1,
            Outcome::Skipped(_) => self.counts.skipped += 1,
            Outcome::Flaky { .. } => self.counts.flaky += 1,
            Outcome::Failed(_) | Outcome::Exited(_) => self.counts.failed += 1,
        }
        if self.format == Format::Human {
            report(self.ctx, &label, &outcome, elapsed, rows);
        }
        // A passing test's output is dropped with `output`. A failing one's
        // is written in the report's place for that test, and under a machine
        // format, where there is no line to put it under, it names the test.
        if failed && !output.is_empty() {
            let text = String::from_utf8_lossy(&output);
            let (heading, indent) = match self.format {
                Format::Human => ("      output:\n".to_owned(), "        "),
                Format::Json | Format::Junit => (format!("{class}::{label} output:\n"), "  "),
            };
            say(self.ctx, &heading);
            for line in text.lines() {
                say(self.ctx, &format!("{indent}{line}\n"));
            }
        }
        if let Outcome::Exited(code) = outcome {
            self.exited = Some(code);
        }
        self.cases.push(Case {
            class,
            method: label,
            declared,
            outcome,
            elapsed,
            snapshots,
            reached,
            rows,
        });
    }

    /// `rule:testing/class-hooks`'s `#[AfterClass]`, with its `Summary`.
    fn after_class(&mut self, class: &str, hook: &nvs_types::test_hooks::HookMethod) {
        let summary = hook
            .takes_value
            .then(|| nvs_stdlib::test_summary(class, self.class, &self.failed_tests));
        match run_class_hook(self.unit, self.ctx, class, hook, "#[AfterClass]", summary) {
            None => {}
            Some(FixtureFailure::Exited(code)) => self.exited = Some(code),
            Some(FixtureFailure::Threw(message)) => {
                if self.format == Format::Human {
                    say(
                        self.ctx,
                        &format!("    ✗ {}\n      {message}\n", hook.method),
                    );
                } else {
                    eprintln!("error: {class}: {message}");
                }
                self.class_failures.push((class.to_owned(), message));
            }
        }
    }
}

/// What a test posted to another core came out with, waiting for it to finish.
fn collect(posted: nvs_host::worker::Posted<Ran>) -> Ran {
    match posted.collect() {
        nvs_host::worker::Answer::Value(ran) => ran,
        // Contained at the far core's task root, and this test's
        // failure rather than the run's.
        nvs_host::worker::Answer::Panicked(panic) => Ran::decided(Outcome::failed(format!(
            "the test's isolate panicked: {panic}"
        ))),
        nvs_host::worker::Answer::Stopped => Ran::decided(Outcome::failed(
            "the test stopped without reaching a verdict",
        )),
    }
}

/// Whether `entry` can be reported without waiting.
fn ready(entry: &Entry) -> bool {
    match entry {
        Entry::Test(pending) => match &pending.state {
            State::Running(posted) => posted.finished(),
            State::Decided(_) | State::Ran(_) => true,
        },
        Entry::Class(_) | Entry::Printed(_) | Entry::ClassEnd { .. } | Entry::Exit(_) => true,
    }
}

/// Drops an entry an `exit(n)` left behind, ending its test first if it is
/// still running. Its class's `#[AfterClass]` does not run.
fn abandon(entry: Entry) {
    if let Entry::Test(pending) = entry
        && let State::Running(posted) = pending.state
    {
        posted.abandon();
    }
}

/// Writes `text` to the suite's own output. A write that fails is nothing a
/// verdict can see.
fn say(ctx: &mut nvs_runtime::Ctx, text: &str) {
    drop(ctx.write_output(text.as_bytes()));
}

/// Calls one of `rule:testing/class-hooks`'s two hooks of `class`, in the
/// parent, with `argument` as its one parameter where it declares one.
///
/// The failure is a [`FixtureFailure`] because it is reported the way a
/// fixture's is: a throw names the hook, and an `exit(n)` ends the run.
fn run_class_hook(
    unit: &nvs_codegen::Unit,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    hook: &nvs_types::test_hooks::HookMethod,
    kind: &str,
    argument: Option<nvs_runtime::Value>,
) -> Option<FixtureFailure> {
    let args: Vec<nvs_runtime::Value> = argument.into_iter().collect();
    let failure = match unit.call_static_as(ctx, &hook.declared_in, class, &hook.method, &args) {
        Some(Ok(_)) => None,
        Some(Err(status)) if status == nvs_runtime::EXITED => {
            Some(FixtureFailure::Exited(ctx.exit_code()))
        }
        Some(Err(_)) => Some(FixtureFailure::Threw(format!(
            "the `{kind}` method `{}` threw: {}",
            hook.method,
            ctx.take_thrown().message()
        ))),
        None => Some(FixtureFailure::Threw(format!(
            "internal error: the compiled unit declares no `{class}::{}`",
            hook.method
        ))),
    };
    // A hook's assertions belong to no test, and the next test's ledger is
    // its own isolate's, so what the hook recorded is dropped here.
    drop(ctx.take_assertions());
    failure
}

/// One `#[Test]` method, run as many times as `rule:testing/runner-is-strict`'s `retries:` allows.
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
/// the child's stack rather than around it. `rule:testing/isolate-per-test` gives an isolate to a *test*,
/// and the attempts are that test: a retry that could not see what the attempt
/// before it wrote to class storage would make the whole option unusable for
/// the thing it exists for — a test that succeeds on a second try because the
/// first left something behind.
///
/// **A flaky test does not fail the run.** Its exit code is the passing one,
/// because retries exist precisely so that a suite can pass in spite of one —
/// what `rule:testing/runner-is-strict` takes away is the *silence*, not the green build.
fn run_with_retries(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    test: &Attempted,
    args: &[nvs_runtime::Value],
    allowance: usize,
) -> Outcome {
    let mut failures = match run_case(unit, ctx, test, args, 1) {
        Outcome::Failed(failures) => failures,
        settled => return settled,
    };
    for retry in 0..allowance {
        match run_case(unit, ctx, test, args, retry + 2) {
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

/// One test's calls on the isolate's stack: a `#[Test]` method's attempts
/// ([`run_with_retries`]), or a `#[Property]` method's generated calls ([`run_property`]),
/// which `property_seed` is the seed of.
fn run_attempts(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    test: &Attempted,
    args: &[nvs_runtime::Value],
    allowance: usize,
    property_seed: Option<u64>,
) -> Outcome {
    match (&test.property, property_seed) {
        (Some(property), Some(seed)) => run_property(unit, ctx, test, property, seed),
        _ => run_with_retries(unit, ctx, test, args, allowance),
    }
}

/// `rule:testing/property-testing`'s run of one `#[Property]` method: `runs` calls, each on a
/// fresh instance with inputs generated from `seed` and the call's index.
///
/// The first call that does not pass ends the run. A failing call's inputs are shrunk
/// ([`shrink_failure`]), and the outcome is the last failing call's. It gets one more record
/// naming the call, the shrunk and the generated inputs and the seed, so `#[Property(seed: n)]`
/// runs the same inputs again ([`counterexample_record`]). There is no retry: `retries:` is not
/// a property's option.
///
/// **What it spends:** one call's inputs at a time, as generated values and then as runtime
/// values, both freed before the next call is generated. A failure also keeps its generated
/// inputs while they shrink.
fn run_property(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    test: &Attempted,
    property: &nvs_types::property::PropertyCase,
    seed: u64,
) -> Outcome {
    // `seed:` is an `int`, and `random_seed` takes its bits as they lie, so the seed is
    // reported as the `int` with the same bits.
    let written = seed.cast_signed();
    for index in 0..property.runs {
        let call = index + 1;
        let Ok(inputs) = nvs_types::property::generate(property, seed, index) else {
            return Outcome::failed(format!(
                "call {call} of `{}`: the generated inputs are larger than one call may use \
                 ({} elements, bytes and characters). Use `gen:` to make a length range \
                 smaller.",
                test.method,
                nvs_types::property::generate::MAX_INPUT_UNITS
            ));
        };
        let found = Counterexample {
            call,
            runs: property.runs,
            seed: written,
            generated: inputs.iter().map(render_input).collect(),
            shrunk: None,
        };
        let outcome = match call_property(unit, ctx, test, property, &inputs) {
            Ok(Outcome::Passed) => continue,
            Ok(failed @ (Outcome::Failed(_) | Outcome::Flaky { .. })) => failed,
            Ok(settled @ (Outcome::Skipped(_) | Outcome::Exited(_))) => return settled,
            // An input that could not be built never reached the method, so there is no
            // failing call to shrink.
            Err(outcome) => return outcome.and_record(counterexample_record(&found)),
        };
        let (shrunk, outcome) = shrink_failure(unit, ctx, test, property, inputs, outcome);
        let found = Counterexample {
            shrunk: shrunk.map(|inputs| inputs.iter().map(render_input).collect()),
            ..found
        };
        return outcome.and_record(counterexample_record(&found));
    }
    Outcome::Passed
}

/// One call of a `#[Property]` method with `inputs`, on a fresh instance.
///
/// # Errors
///
/// The outcome of an input that could not be built: [`build_input`]'s.
fn call_property(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    test: &Attempted,
    property: &nvs_types::property::PropertyCase,
    inputs: &[nvs_types::property::Generated],
) -> Result<Outcome, Outcome> {
    // Dropped after the call has returned: it has the one reference each input carries, and
    // the call only borrows them.
    let mut values = nvs_runtime::InputValues::new();
    let mut args = Vec::with_capacity(inputs.len());
    for (generator, input) in property.params.iter().zip(inputs) {
        let built = build_input(ctx, &mut values, property, generator, input)?;
        args.push(values.get(built));
    }
    Ok(run_case(unit, ctx, test, &args, 1))
}

/// `inputs`, which failed with `failed`, shrunk by `nvs_types::property::shrink` in the
/// test's own isolate: each candidate is built and the method called once on a fresh
/// instance, at most [`MAX_SHRINK_CALLS`](nvs_types::property::shrink::MAX_SHRINK_CALLS)
/// times. Returns the shrunk inputs, or `None` when no simpler input failed, and the outcome
/// of the last call that failed.
///
/// **Only the last failing call's records are kept.** [`judge`] takes the ledger at every
/// call, so a passing candidate leaves nothing behind, and each failing one replaces the
/// outcome before it. A candidate that could not be built, or that passed, is not a failure.
///
/// **A stop ends the shrinking.** Once the test is past its wall-time limit, or a candidate
/// called `exit()`, no other candidate runs, and the last real failure is reported. A call the
/// limit stopped would otherwise count as a failure of every candidate after it.
fn shrink_failure(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    test: &Attempted,
    property: &nvs_types::property::PropertyCase,
    inputs: Vec<nvs_types::property::Generated>,
    failed: Outcome,
) -> (Option<Vec<nvs_types::property::Generated>>, Outcome) {
    use nvs_types::property::shrink;
    let mut last = failed;
    let mut shrank = false;
    let mut stopped = false;
    let (shrunk, _calls) = shrink::shrink(
        inputs,
        &property.params,
        &property.classes,
        |candidate| {
            if stopped || ctx.safepoint_view().deadline_expired() {
                stopped = true;
                return false;
            }
            let outcome = call_property(unit, ctx, test, property, candidate);
            if ctx.safepoint_view().deadline_expired() {
                stopped = true;
                return false;
            }
            match outcome {
                Ok(failed @ (Outcome::Failed(_) | Outcome::Flaky { .. })) => {
                    last = failed;
                    shrank = true;
                    true
                }
                Ok(Outcome::Exited(_)) | Err(Outcome::Exited(_)) => {
                    stopped = true;
                    false
                }
                Ok(_) | Err(_) => false,
            }
        },
        shrink::MAX_SHRINK_CALLS,
    );
    (shrank.then_some(shrunk), last)
}

/// A property's failing call, as a report prints it: which call of how many, the seed, and
/// its inputs as the generator made them and as they shrank.
struct Counterexample {
    call: u32,
    runs: u32,
    seed: i64,
    generated: Vec<String>,
    /// `None` when no simpler input failed.
    shrunk: Option<Vec<String>>,
}

/// The field of the record that names a property's failing call
/// (`rule:testing/report-formats`'s counterexample). [`json_document`] writes it as the
/// test's `counterexample` key.
const COUNTEREXAMPLE: &str = "counterexample";

/// The failure record of a property's failing call: its line, and the [`COUNTEREXAMPLE`]
/// field the JSON report reads.
fn counterexample_record(found: &Counterexample) -> Record {
    let Counterexample {
        call,
        runs,
        seed,
        generated,
        shrunk,
    } = found;
    let generated_line = generated.join(", ");
    let line = match shrunk {
        Some(shrunk) => format!(
            "call {call} of {runs} failed. The smallest inputs that still fail are ({}). The \
             generated inputs were ({generated_line}). Write `#[Property(seed: {seed})]` to run \
             the same inputs again.",
            shrunk.join(", ")
        ),
        None => format!(
            "call {call} of {runs} failed with the inputs ({generated_line}). Write \
             `#[Property(seed: {seed})]` to run the same inputs again."
        ),
    };
    let texts = |items: &[String]| {
        Node::Sequence(
            items
                .iter()
                .map(|text| {
                    Node::Scalar(Scalar::Str {
                        text: Rendered::new(text),
                        bytes: text.len(),
                    })
                })
                .collect(),
        )
    };
    let mut record = failure(&line);
    record.envelope.fields.push((
        COUNTEREXAMPLE.to_owned(),
        Node::Map(vec![
            (Rendered::new("seed"), Node::Scalar(Scalar::Int(*seed))),
            (
                Rendered::new("call"),
                Node::Scalar(Scalar::Uint(u64::from(*call))),
            ),
            (
                Rendered::new("inputs"),
                texts(shrunk.as_deref().unwrap_or(generated)),
            ),
            (Rendered::new("generatedInputs"), texts(generated)),
        ]),
    ));
    record
}

/// `input` built as the value a call takes, owned by `values`. `generator` is what generated
/// it: it names the class of a shape, whose optional fields `input` may leave out, and the arm
/// of a union a value came from.
///
/// # Errors
///
/// The outcome of a constructor that threw or called `exit()`, or an internal error for a
/// value the compiled unit has no class for.
fn build_input(
    ctx: &mut nvs_runtime::Ctx,
    values: &mut nvs_runtime::InputValues,
    property: &nvs_types::property::PropertyCase,
    generator: &nvs_types::property::Gen,
    input: &nvs_types::property::Generated,
) -> Result<nvs_runtime::Input, Outcome> {
    use nvs_types::property::{Gen, Generated};
    let internal = |what: String| Outcome::failed(format!("internal error: {what}"));
    Ok(match (generator, input) {
        (Gen::Nullable(inner), _) if *input != Generated::Null => {
            build_input(ctx, values, property, inner, input)?
        }
        (Gen::OneOf(arms), _) => {
            let arm = arms
                .iter()
                .find(|arm| input_fits(arm, input))
                .ok_or_else(|| internal(format!("no type of the union generated {input:?}")))?;
            build_input(ctx, values, property, arm, input)?
        }
        (Gen::List { element, .. }, Generated::List(items)) => {
            let mut built = Vec::with_capacity(items.len());
            for item in items {
                built.push(build_input(ctx, values, property, element, item)?);
            }
            values.list(&built)
        }
        (Gen::Shape(fields), Generated::Shape(written)) => {
            let mut names: Vec<String> = fields.iter().map(|field| field.name.clone()).collect();
            names.sort_unstable();
            let label = nvs_types::derive::shape_class_label(&names);
            let mut built = Vec::with_capacity(written.len());
            for (name, value) in written {
                let field = fields
                    .iter()
                    .find(|field| field.name == *name)
                    .ok_or_else(|| {
                        internal(format!("the shape `{label}` has no field `{name}`"))
                    })?;
                built.push((
                    name.as_str(),
                    build_input(ctx, values, property, &field.generator, value)?,
                ));
            }
            values
                .shape(ctx, &label, &built)
                .ok_or_else(|| internal(format!("the compiled unit has no class `{label}`")))?
        }
        (Gen::Class(_), Generated::Object { class, args }) => {
            let generators = property
                .class(class)
                .map(|row| row.args.as_slice())
                .unwrap_or_default();
            let mut built = Vec::with_capacity(args.len());
            for (generator, arg) in generators.iter().zip(args) {
                built.push(build_input(ctx, values, property, generator, arg)?);
            }
            match values.object(ctx, class, &built) {
                Ok(object) => object,
                Err(None) => {
                    return Err(internal(format!(
                        "the compiled unit declares no class `{class}`"
                    )));
                }
                Err(Some(status)) if status == nvs_runtime::EXITED => {
                    return Err(Outcome::Exited(ctx.exit_code()));
                }
                Err(Some(_)) => {
                    return Err(Outcome::failed(format!(
                        "`{}` threw while creating a generated input: {}",
                        render_input(input),
                        ctx.take_thrown().message()
                    )));
                }
            }
        }
        (_, Generated::Null) => values.null(),
        (_, Generated::Bool(value)) => values.bool(*value),
        (_, Generated::Int(value)) => values.int(*value),
        (_, Generated::Uint(value)) => values.uint(*value),
        (_, Generated::Float(value)) => values.float(*value),
        (_, Generated::Decimal(digits)) => values
            .decimal(digits)
            .ok_or_else(|| internal(format!("`{digits}` is not a decimal")))?,
        (_, Generated::Str(text)) => values.str(text),
        (_, Generated::Bytes(raw)) => values.bytes(raw),
        (_, Generated::Enum { value, uint: true }) => values.enum_uint(
            u64::try_from(*value).map_err(|_| internal(format!("{value} is not a `uint`")))?,
        ),
        (_, Generated::Enum { value, uint: false }) => values.enum_int(
            i64::try_from(*value).map_err(|_| internal(format!("{value} is not an `int`")))?,
        ),
        (_, Generated::List(_) | Generated::Shape(_) | Generated::Object { .. }) => {
            return Err(internal(format!(
                "{input:?} does not match the generator {generator:?}"
            )));
        }
    })
}

/// Whether `generator` could have generated `input`: the test [`build_input`] uses to find the
/// arm of a union a value came from.
fn input_fits(
    generator: &nvs_types::property::Gen,
    input: &nvs_types::property::Generated,
) -> bool {
    use nvs_types::property::{Gen, Generated};
    match (generator, input) {
        (Gen::Literal(value), _) => value == input,
        (Gen::Nullable(inner), _) => *input == Generated::Null || input_fits(inner, input),
        (Gen::OneOf(arms), _) => arms.iter().any(|arm| input_fits(arm, input)),
        (Gen::Bool, Generated::Bool(_))
        | (Gen::Float { .. }, Generated::Float(_))
        | (Gen::Decimal { .. }, Generated::Decimal(_))
        | (Gen::Str { .. }, Generated::Str(_))
        | (Gen::Bytes { .. }, Generated::Bytes(_)) => true,
        (Gen::Int { min, max }, Generated::Int(value)) => (min..=max).contains(&value),
        (Gen::Uint { min, max }, Generated::Uint(value)) => (min..=max).contains(&value),
        (
            Gen::Enum { values, uint },
            Generated::Enum {
                value,
                uint: backed,
            },
        ) => uint == backed && values.contains(value),
        (Gen::List { element, .. }, Generated::List(items)) => {
            items.iter().all(|item| input_fits(element, item))
        }
        (Gen::Shape(fields), Generated::Shape(written)) => {
            fields
                .iter()
                .all(|field| !field.required || written.iter().any(|(name, _)| *name == field.name))
                && written.iter().all(|(name, value)| {
                    fields
                        .iter()
                        .any(|field| field.name == *name && input_fits(&field.generator, value))
                })
        }
        (Gen::Class(label), Generated::Object { class, .. }) => label == class,
        _ => false,
    }
}

/// One generated input written the way a program writes the value: `[1, 2]`, `{a: 1}`,
/// `new Point(1, 2)`, and a string in double quotes with its escapes. An enum case is written as
/// its backing value.
fn render_input(input: &nvs_types::property::Generated) -> String {
    use nvs_types::property::Generated;
    let list = |items: &[Generated]| {
        items
            .iter()
            .map(render_input)
            .collect::<Vec<_>>()
            .join(", ")
    };
    match input {
        Generated::Null => "null".to_owned(),
        Generated::Bool(value) => value.to_string(),
        Generated::Int(value) => value.to_string(),
        Generated::Uint(value) => value.to_string(),
        Generated::Float(value) => {
            let text = value.to_string();
            if text.contains(['.', 'e', 'i', 'N']) {
                text
            } else {
                format!("{text}.0")
            }
        }
        Generated::Decimal(digits) => format!("\"{digits}\" as decimal"),
        Generated::Str(text) => {
            let mut out = String::from("\"");
            for ch in text.chars() {
                match ch {
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    '$' => out.push_str("\\$"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    ch if ch.is_control() => out.push_str(&format!("\\u{{{:X}}}", u32::from(ch))),
                    ch => out.push(ch),
                }
            }
            out.push('"');
            out
        }
        Generated::Bytes(raw) => {
            let mut out = String::from("\"");
            for byte in raw {
                match byte {
                    b'\\' => out.push_str("\\\\"),
                    b'"' => out.push_str("\\\""),
                    b'$' => out.push_str("\\$"),
                    0x20..=0x7E => out.push(char::from(*byte)),
                    _ => out.push_str(&format!("\\x{byte:02X}")),
                }
            }
            out.push_str("\" as bytes");
            out
        }
        Generated::Enum { value, .. } => value.to_string(),
        Generated::List(items) => format!("[{}]", list(items)),
        Generated::Shape(fields) => format!(
            "{{{}}}",
            fields
                .iter()
                .map(|(name, value)| format!("{name}: {}", render_input(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Generated::Object { class, args } => format!("new {class}({})", list(args)),
    }
}

/// Why a class's fixtures could not be built — reported against the tests
/// that asked for one — or why one of its class hooks failed.
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
/// nothing names would be setup nobody wanted, and `rule:testing/runner-is-strict`'s `skip:` means the
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
        match unit.build_fixture(
            ctx,
            fixtures,
            &row.declared_in,
            class,
            &row.method,
            &row.fixtures,
        ) {
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
/// too and fails for a second reason (`rule:testing/runner-is-strict`). A throw is therefore the *last*
/// thing consulted rather than the first, and it only ever answers for a test
/// whose ledger is clean — an exception raised by the code under test rather
/// than by an assertion about it.
///
/// **The instance is constructed inside the isolate, not before it.** `rule:testing/isolate-per-test`'s
/// boundary is around the whole test, so everything the test can observe —
/// its receiver, its statics, its ledger, its `echo` — is built on the child's
/// own context and released with it. What this function does on the parent's
/// side is only what has to happen before the child exists: `rule:testing/data-rows`'s row,
/// and the Rust closure it and `rule:testing/fixtures`'s copies, decoded by [`Job::run`], are moved into.
fn run_in_isolate(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    test: &Attempted,
    case: &nvs_types::testing::TestCase,
    row: Option<&[Option<ConstArg>]>,
    crossed: nvs_runtime::CrossedFixtures,
    routes: &std::sync::Arc<nvs_runtime::routes::Routes>,
) -> (Outcome, Vec<nvs_runtime::SnapshotMismatch>, Option<Rows>) {
    // `rule:testing/inline-snapshots`'s mismatches cross the boundary the verdict crosses, in a cell of
    // their own beside it: they are the child's context's and that context is
    // gone by the time this returns, and they are not part of the verdict —
    // nothing about a test's outcome depends on whether it was going to be
    // rewritten. The seam is one cell rather than a second field on [`Outcome`]
    // so that every early exit below, none of which ran a line of the test,
    // hands back an empty list without saying so.
    let snapshots: Rc<RefCell<Vec<nvs_runtime::SnapshotMismatch>>> =
        Rc::new(RefCell::new(Vec::new()));
    // A bench's rows cross the same way, in a cell of their own.
    let rows: Rc<Cell<Option<Rows>>> = Rc::new(Cell::new(None));
    let outcome = run_the_test(unit, ctx, test, case, row, crossed, routes, &snapshots, &rows);
    let taken = std::mem::take(&mut *snapshots.borrow_mut());
    (outcome, taken, rows.get())
}

/// [`run_in_isolate`]'s body, with the cell it fills passed in — everything
/// this module's docs say about the isolate is about this function.
#[expect(
    clippy::too_many_arguments,
    reason = "the caller's seven, plus the cells `rule:testing/inline-snapshots`'s mismatches and a bench's rows cross in"
)]
fn run_the_test(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    test: &Attempted,
    case: &nvs_types::testing::TestCase,
    row: Option<&[Option<ConstArg>]>,
    crossed: nvs_runtime::CrossedFixtures,
    routes: &std::sync::Arc<nvs_runtime::routes::Routes>,
    snapshots: &Rc<RefCell<Vec<nvs_runtime::SnapshotMismatch>>>,
    rows: &Rc<Cell<Option<Rows>>>,
) -> Outcome {
    let class = test.class.as_str();
    if let Some(reason) = skip_reason(case) {
        return Outcome::Skipped(reason);
    }
    // `rule:testing/in-process-request`'s second mechanism, bound **before** the child exists for the same
    // reason `rule:testing/data-rows`'s row and `rule:testing/fixtures`'s copies are: what the test observes of it is one
    // string on its own context, and a socket cannot be handed across the heap
    // boundary afterwards. A failure to bind is this test's own failure and
    // nothing has run.
    let served = match wants_server(case) {
        false => None,
        true => match TestServer::bind(unit, routes, Door::of(ctx), ctx.stmt_hits().cloned()) {
            Ok(server) => Some(server),
            Err(refused) => {
                return Outcome::failed(format!(
                    "`{class}::{}` declares `server: true`, and no listener could be bound for \
                     it: {refused}",
                    case.method
                ));
            }
        },
    };
    let listening = served.as_ref().map(|server| server.url.clone());
    // `rule:testing/fixtures` and `rule:testing/data-rows`'s injection: one value per declared parameter, in the order the
    // checker resolved them (`nvs_types::testing::TestCase::params`), so
    // nothing here re-derives which fixture answers which parameter or which
    // field answered which name. A fixture's value is this test's own copy and
    // a row's is materialized just below — the call retains its own either
    // way, exactly as any other call on a value this frame holds does.
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
            return Outcome::failed(format!(
                "internal error: `{class}::{}` has no value for parameter {position}",
                case.method
            ));
        };
        args.push(value);
    }
    // The verdict is decided on the child's own stack and filed into a cell
    // its program captured, because a `Completion` carries a value, bytes and
    // a failure and none of those is `rule:testing/failure-ledger`'s ledger. Nothing about that cell
    // crosses the heap boundary: an [`Outcome`] is records the runner owns,
    // built while the child's context is still alive and read after it is
    // gone.
    let filed: Rc<RefCell<Option<Outcome>>> = Rc::new(RefCell::new(None));
    let verdict = Rc::clone(&filed);
    let recorded = Rc::clone(snapshots);
    let counted = case.bench.then(|| Rc::clone(rows));
    let allowance = retry_allowance(case);
    // `rule:testing/determinism-declared-on-the-test`'s clock, resolved on the parent's side so that a malformed `at:` is
    // this test's own reported failure rather than something the child has to
    // carry back across the boundary — nothing has run at this point, which is
    // what makes the message the whole of what happened.
    let clock = match fixed_clock(case) {
        Ok(clock) => clock,
        Err(at) => {
            return Outcome::failed(format!(
                "`{class}::{}` declares `at: \"{at}\"`, which is not an RFC 3339 timestamp such \
                 as `2026-01-01T00:00:00Z` from `-9999-01-02T01:59:59Z` to `9999-12-30T22:00:00Z`",
                case.method
            ));
        }
    };
    let seed = random_seed(case);
    // `rule:testing/property-testing`'s seed, drawn here on the parent's side when `seed:` names
    // none, so the failure the child reports can name the seed that reproduces it. It seeds the
    // isolate's `Core\Random` too, as a written `seed:` does.
    let property_seed = test
        .property
        .is_some()
        .then(|| seed.unwrap_or_else(|| rand::rng().random::<u64>()));
    let seed = seed.or(property_seed);
    // `rule:testing/db-transaction`'s block name, read on the parent's side with every other option and
    // owned because the Rust closure outlives this frame — but *opened* inside the
    // child, which is the one decision this mechanism makes. A connection is
    // memoized on the context it was opened on, so a transaction begun out here
    // would be on a connection the test's own `Core\Db::connect` could never
    // reach, and `rule:testing/db-transaction`'s "a transaction opened inside the test is a savepoint"
    // would be two transactions on two connections deadlocking on each other.
    // `nvs_stdlib::db::begin_test_transaction` is the one home of that reading.
    let transacted = wants_db(case).map(str::to_owned);
    let child_unit = Arc::clone(unit);
    let class_name = class.to_owned();
    let method = case.method.clone();
    let attempted = test.clone();
    let program: nvs_runtime::script::Program = Box::new(move |child, argument| {
        // `rule:statements/an-isolate-has-its-own-statics`'s fresh statics base, armed with *this* unit's tables:
        // the isolate's context deliberately arrives with none, so this call
        // is the whole of why one test does not read back another's static.
        child_unit.install_in(child);
        // A failed comparison keeps its two values only on a test's own
        // context (`nvs_runtime::Ctx::owned_by_test`).
        child.set_owned_by_test();
        // `rule:testing/determinism-declared-on-the-test`'s fixed clock, armed before a line of the test runs and onto the
        // *child's* context, which is the only context a test can observe: a
        // sibling declaring no `at:` still reads the host's clock, because `rule:testing/isolate-per-test`
        // gave it a context of its own. `Core\Test::advance` moves this same
        // field and nothing else does (`nvs_runtime::Ctx::fixed_clock`).
        if let Some(nanos) = clock {
            child.set_fixed_clock(nanos);
        }
        // `rule:testing/determinism-declared-on-the-test`'s other half, and armed the same way and in the same place: the
        // generator's state lives on the child's own context, so a sibling that
        // declared no `seed:` still draws from the CSPRNG and one test's draws
        // cannot move another's (`nvs_stdlib::random::draw`).
        if let Some(seed) = seed {
            child.set_random_state(seed);
        }
        // `rule:testing/in-process-request`'s listener, armed the same way and in the same place as the two
        // above: the address is the *child's* to read, so a sibling that
        // declared no `server:` answers `null` and cannot reach a port this
        // test bound (`nvs_runtime::Ctx::test_server`).
        if let Some(url) = listening {
            child.set_test_server(url);
        }
        // Null here, and discharged anyway: the seam's contract is that
        // whatever crossed becomes the isolate's own root's, and a test's
        // values crossed as `rule:testing/fixtures`'s copies instead.
        child.set_isolate_argument(argument);
        // Both owners go down with the isolate, after the call that borrowed
        // their values has returned.
        let (_crossed, _materialized) = (crossed, materialized);
        // `rule:testing/bench-counters`' counting mode, on the child's own context and around the
        // call alone. The allocation counters are this thread's, and the child runs on it. The
        // thread's empty array is made before the window, so a bench's rows do not depend on
        // whether a bench before it on this thread wrote the first `[]`.
        let before = counted.as_ref().map(|_| {
            nvs_runtime::prime_empty_array();
            child.set_debug_flags(child.debug_flags() | nvs_runtime::DebugFlags::COUNT);
            (
                nvs_runtime::budget::allocations(),
                nvs_runtime::budget::allocated_bytes(),
                Instant::now(),
            )
        });
        let outcome = match transacted.as_deref() {
            None => run_attempts(
                &child_unit,
                child,
                &attempted,
                &args,
                allowance,
                property_seed,
            ),
            // `rule:testing/db-transaction`, around the whole allowance rather than around one attempt:
            // `retries:` exists for a test that is flaky against something
            // outside the database, and a retry that started from a pristine
            // database would be a second rule about what a retry re-runs —
            // `rule:testing/runner-is-strict` says only that the method is called again. A test whose
            // second attempt has to unsee its first attempt's writes has
            // written a test the rollback cannot fix.
            Some(name) => match nvs_stdlib::db::begin_test_transaction(child, name) {
                Err(refused) => Outcome::failed(format!(
                    "`{class_name}::{method}` declares `db: \"{name}\"`, and no transaction \
                     could be opened on it: {refused}"
                )),
                Ok(key) => {
                    let outcome = run_attempts(
                        &child_unit,
                        child,
                        &attempted,
                        &args,
                        allowance,
                        property_seed,
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
        if let (Some(cell), Some((allocations, bytes, began))) = (&counted, before) {
            let wall = began.elapsed();
            let done = child.counted();
            cell.set(Some(Rows {
                statements: done.statements,
                calls: done.calls,
                allocations: nvs_runtime::budget::allocations().wrapping_sub(allocations),
                bytes: nvs_runtime::budget::allocated_bytes().wrapping_sub(bytes),
                gc: done.gc_cycles,
                wall,
            }));
        }
        // `rule:testing/task-tree-and-virtual-clock`, read off the tree from inside the test's own task and nowhere
        // else: a child spawned here is a child of *this* task, and once this
        // Rust closure returns the scheduler cancels whatever is left rather than
        // reporting it. `nvs_host::children_still_running` owns why an awaited
        // child is already gone from the count.
        // `rule:testing/inline-snapshots`'s mismatches, taken on the child's own stack for the reason its
        // ledger is taken there: the record is this test's, and this is the
        // last frame that holds the context carrying it. Every attempt `rule:testing/runner-is-strict`
        // allowed is in the list, which is why [`update_snapshots`] reads only
        // a test that ended failed — a retried test that went on to pass has
        // the snapshot its last attempt matched.
        *recorded.borrow_mut() = child.take_snapshot_mismatches();
        *verdict.borrow_mut() =
            Some(outcome.with_tasks_left_running(nvs_host::children_still_running()));
        nvs_runtime::Value::null()
    });
    // `Output::Inherit`: what a test echoed is appended to `ctx`'s stream at
    // the join, and `ctx` is the test's own parent context, whose buffer is
    // the test's captured output ([`Job::run`]).
    let completion = match nvs_host::Isolate::new(
        program,
        nvs_runtime::Value::null(),
        nvs_host::Output::Inherit,
    )
    .running_as(nvs_runtime::RunKind::Test)
    .run(ctx)
    {
        Ok(completion) => completion,
        // Not reachable: the only `Err` this boundary has is about the
        // argument, and the argument is `null`. Reported rather than
        // unwrapped, because a panic in the runner is the one thing a program
        // under test may not be able to cause.
        Err(refused) => {
            return Outcome::failed(format!(
                "internal error: the test's isolate refused its argument: {refused}"
            ));
        }
    };
    // `rule:testing/isolate-per-test`'s boundary is around the whole test, and a listener is the one thing
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
        Outcome::failed(completion.error.map_or_else(
            || "the test's isolate ended without reaching a verdict".to_owned(),
            |failure| format!("{}: {}", failure.class, failure.message),
        ))
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
/// **Why a second mechanism at all**, when `rule:testing/in-process-request`'s first one already runs a
/// request through the same entry with no socket: what this one answers is
/// everything between the two — the `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` header set actually arriving on
/// the wire, chunking, keep-alive, a peer that hangs up mid-body. The in-process
/// path deliberately never builds a response head, so none of that is a question
/// it *can* be asked, which is why `rule:testing/in-process-request` keeps a second mechanism.
///
/// **The port is the operating system's**, taken by binding `127.0.0.1:0` and
/// reading the address back: a fixed port would collide between two `cargo test`
/// processes on one machine and between two tests of one suite, and there is
/// nothing for a developer to configure because there is nothing for them to
/// choose. Loopback and not `0.0.0.0` for the reason that needs no measurement —
/// a test suite that binds a routable address is a test suite that serves a
/// program under test to the network.
///
/// **The handler holds a [`std::sync::Weak`] of the unit, never an [`Arc`].** The
/// accept loop parks on a socket nothing will connect to again once the test is
/// over, and a parked task's Rust closure is dropped when the scheduler reaps it
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
    /// and a child of the suite's task. Sibling and not child on purpose: `rule:testing/task-tree-and-virtual-clock`
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
    ///
    /// `hits` is the run's coverage table, or `None` when the run does not
    /// measure coverage. Each request's own context gets it in
    /// [`answer_on_the_wire`], because the server starts every connection
    /// from a fresh context that copies nothing from this one.
    fn bind(
        unit: &Arc<nvs_codegen::Unit>,
        routes: &std::sync::Arc<nvs_runtime::routes::Routes>,
        door: Door,
        hits: Option<std::sync::Arc<nvs_runtime::StmtHits>>,
    ) -> Result<Self, String> {
        let wanted = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0));
        let mut listener = nvs_host::NvsListener::bind(wanted)
            .map_err(|error| format!("could not listen on {wanted}: {error}"))?;
        let bound = listener
            .local_addr()
            .map_err(|error| format!("the listener bound no readable address: {error}"))?;
        let held = Arc::downgrade(unit);
        let carried = std::sync::Arc::clone(routes);
        let handler = Rc::new(
            move |request: nvs_server::Request<nvs_server::Incoming>,
                  origin: nvs_server::Origin| {
                answer_on_the_wire(&held, &carried, &door, hits.as_ref(), request, origin)
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
        // `nvs test` is one process running many listeners, at once and one
        // after another, and a shared bit would be a `503` one test could leave
        // armed for another.
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

/// `rule:security/csrf-is-on-by-default`'s refusal over a request already in
/// memory, in `rule:routing/a-gate-answers-around-every-handler`'s refusal
/// mode: a forged write is marked refused and its body cleared, and the gate
/// answers it. `UnderTest::answer` and `nvs run --request` both take it, so a
/// request read off a file meets the door a test's request meets. A unit that
/// does not dispatch is left as it was.
///
/// A synthetic body is already in memory, so its form field is read here,
/// where `answer_on_the_wire` leaves it to the request's task. `ctx` is read
/// for the tree, so it must already carry the run's configuration.
pub(crate) fn refuse_a_forged_write(
    ctx: &nvs_runtime::Ctx,
    routes: &nvs_runtime::routes::Routes,
    inbound: &mut nvs_runtime::Inbound,
) {
    if !routes.dispatches() {
        return;
    }
    let door = Door::of(ctx);
    let verdict = match nvs_server::route::csrf(inbound, door.key.as_deref(), &door.cookie) {
        nvs_server::route::Csrf::InBody => nvs_server::route::csrf_in_body(
            inbound,
            door.key.as_deref(),
            &door.cookie,
            ctx.request_body_limit(),
        ),
        verdict => verdict,
    };
    if verdict == nvs_server::route::Csrf::Refused {
        inbound.set_refusal(nvs_runtime::routes::Refusal::Csrf);
        inbound.clear_body();
    }
}

/// What `rule:security/csrf-is-on-by-default`'s refusal is decided from, read
/// off the tree once at bind time.
///
/// `rule:testing/in-process-request` is why it exists at all: a request a test
/// sends crosses the same door as one off the wire, so the check has to be taken
/// here too. It is read once rather than per request because the listener
/// outlives no reload — a test's tree is the one the case started under — and
/// because the Rust closure that answers is `'static` and a borrow of the context's
/// snapshot is not.
struct Door {
    /// `[http] csrf_key` decoded, or `None` where the tree names none.
    key: Option<Vec<u8>>,
    /// The cookie the session identifier rides under.
    cookie: String,
    /// `[limits] upload_total`, which the door applies to a declared length.
    upload_total: u64,
}

impl Door {
    /// What the tree this case is running under says, or what a case with no
    /// configuration at all gets: no key, the shipped cookie name and the
    /// shipped `upload_total`.
    fn of(ctx: &nvs_runtime::Ctx) -> Self {
        let Some(tree) = ctx.config().map(|request| &request.snapshot().config) else {
            let tree = nvs_config::Config::default();
            return Self {
                key: None,
                cookie: nvs_stdlib::session::cookie_in(&tree).to_owned(),
                upload_total: nvs_config::app::upload_total(&tree),
            };
        };
        Self {
            key: nvs_config::http::csrf_key(tree),
            cookie: nvs_stdlib::session::cookie_in(tree).to_owned(),
            upload_total: nvs_config::app::upload_total(tree),
        }
    }
}

/// One request off the wire, answered by the program under test — the same
/// shape `UnderTest::answer` builds for `rule:testing/in-process-request`'s first mechanism, over a
/// carrier `hyper` filled instead of one a `Core` member wrote.
///
/// It is a free function rather than a Rust closure's body so that the two halves read
/// side by side: everything about *which* program answers is the same, and
/// everything about *what arrived* is the door's.
fn answer_on_the_wire(
    held: &std::sync::Weak<nvs_codegen::Unit>,
    routes: &std::sync::Arc<nvs_runtime::routes::Routes>,
    door: &Door,
    hits: Option<&std::sync::Arc<nvs_runtime::StmtHits>>,
    request: nvs_server::Request<nvs_server::Incoming>,
    origin: nvs_server::Origin,
) -> nvs_server::Reply {
    // Asked of `nvs_stdlib::request` for `crate::serve`'s reason: the roster of
    // verbs has one home and the door does not keep a list. A program that calls
    // `Core\Router::dispatch` answers it in refusal mode, as `crate::serve`'s
    // door does, and every other program gets the door's `501`.
    let unknown_verb = !nvs_stdlib::request::is_known_verb(request.method().as_str());
    let refusal_mode = routes.dispatches();
    if unknown_verb && !refusal_mode {
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
    // A rate of zero, which is not a stand-in for a tree this path does not
    // hold: `nvs test`'s served request has no exporter to push a recorded trace
    // to, so what head sampling would decide here is a decision with no consumer.
    // An arrived sampled header is still continued, because that half is the
    // root's and not this hop's (`rule:observability/sampling-is-head-based`).
    nvs_server::trace::take(&mut inbound, None);
    // `rule:routing/matched-once-before-the-handler`'s one match, taken here for the reason `crate::serve` takes
    // it here: the request and the unit that will answer it are both in hand,
    // and no application code has run.
    if unknown_verb {
        inbound.set_refusal(nvs_runtime::routes::Refusal::UnknownMethod);
    } else {
        nvs_server::route::take(routes, &mut inbound);
    }
    // And the refusal over it, for `crate::serve`'s reason and in the same place:
    // `rule:testing/in-process-request` has a test's request cross this door the
    // way one off the wire does, so a case that asserts a `403` on a forged write
    // is asserting the server's own answer.
    let mut in_body = false;
    if inbound.refusal().is_none() {
        match nvs_server::route::csrf(&inbound, door.key.as_deref(), &door.cookie) {
            nvs_server::route::Csrf::Refused if !refusal_mode => {
                return nvs_server::Reply::forbidden();
            }
            nvs_server::route::Csrf::Refused => {
                inbound.set_refusal(nvs_runtime::routes::Refusal::Csrf);
            }
            nvs_server::route::Csrf::InBody => in_body = true,
            nvs_server::route::Csrf::Passed => {}
        }
    }
    let (head, incoming) = request.into_parts();
    let supply = match nvs_server::body::of(&head.headers, incoming, door.upload_total) {
        _ if inbound.refusal().is_some() => None,
        nvs_server::Arrived::Absent => None,
        nvs_server::Arrived::TooLarge if refusal_mode => {
            inbound.set_refusal(nvs_runtime::routes::Refusal::BodyTooLarge);
            None
        }
        nvs_server::Arrived::TooLarge => return nvs_server::Reply::too_large(),
        nvs_server::Arrived::Streaming(supply, pull) => {
            inbound.set_body(pull);
            Some(supply)
        }
    };
    let carried = std::sync::Arc::clone(routes);
    let hits = hits.cloned();
    let program: nvs_runtime::script::Program =
        Box::new(move |ctx: &mut nvs_runtime::Ctx, _args| {
            unit.install_in(ctx);
            if !carried.rows().is_empty() {
                ctx.set_routes(std::sync::Arc::clone(&carried));
            }
            if let Some(hits) = hits {
                ctx.set_stmt_hits(Some(hits));
                ctx.set_debug_flags(
                    ctx.debug_flags()
                        | nvs_runtime::DebugFlags::COVERAGE
                        | nvs_runtime::DebugFlags::BRANCH,
                );
            }
            let Some(entry) = unit.script() else {
                ctx.set_pending("the program under test has no script frame");
                return nvs_runtime::Value::null();
            };
            entry
                .call(ctx)
                .unwrap_or_else(|_| nvs_runtime::Value::null())
        });
    // `crate::serve`'s reason: the door cannot pull the body, so a form token
    // is read by the request's own task before the program runs.
    let program = match (in_body, &door.key) {
        (true, Some(key)) => nvs_server::route::reading_the_form(
            program,
            key.clone(),
            door.cookie.clone(),
            refusal_mode,
        ),
        _ => program,
    };
    nvs_server::Reply::Run(
        nvs_host::Isolate::new(
            program,
            nvs_runtime::Value::null(),
            nvs_runtime::host::Output::Capture,
        )
        .answering(inbound),
        supply,
        None,
    )
}

/// One attempt at one test: a fresh instance of the test's class, its method
/// called on that instance with `args`, `rule:testing/failure-ledger`'s verdict
/// read off the ledger, and then `rule:testing/after-each-test`'s hook on the
/// same instance. `attempt` is `1` for the first.
///
/// **Every line of it runs on the isolate's own stack** — the instance, the
/// ledger and the pending exception are all the child's, and reading any of
/// them after that context's wholesale release would be reading a context that
/// is gone. [`run_in_isolate`] is the half that runs before the child exists.
fn run_case(
    unit: &Arc<nvs_codegen::Unit>,
    ctx: &mut nvs_runtime::Ctx,
    test: &Attempted,
    args: &[nvs_runtime::Value],
    attempt: usize,
) -> Outcome {
    let mut verdict = None;
    let mut after = |ctx: &mut nvs_runtime::Ctx, receiver, status| {
        let outcome = judge(ctx, status, test.bench);
        verdict = Some(match &test.after_each {
            Some(hook) => after_each_test(ctx, receiver, test, hook, attempt, outcome),
            None => outcome,
        });
    };
    let Some(status) = unit.call_on_new_instance(ctx, &test.class, &test.method, args, &mut after)
    else {
        return Outcome::failed(format!(
            "internal error: the compiled unit declares no class `{}`",
            test.class
        ));
    };
    // No verdict means the method was never called: the constructor threw,
    // so there is no object for the hook to run on.
    verdict.unwrap_or_else(|| judge(ctx, status, test.bench))
}

/// `rule:testing/after-each-test`'s call: `hook` on the instance the attempt
/// ran on, with a `Core\Test\Outcome` where it declares one. What the hook
/// threw or failed is added to the attempt's own failures. A test that called
/// `exit()` is not followed by the hook, since nothing after an `exit()` runs.
fn after_each_test(
    ctx: &mut nvs_runtime::Ctx,
    receiver: nvs_runtime::Value,
    test: &Attempted,
    hook: &nvs_types::test_hooks::HookMethod,
    attempt: usize,
    outcome: Outcome,
) -> Outcome {
    let failures: Vec<String> = match &outcome {
        Outcome::Failed(failures) => failures
            .iter()
            .map(|record| message_of(record).to_owned())
            .collect(),
        Outcome::Passed => Vec::new(),
        _ => return outcome,
    };
    let argument = hook
        .takes_value
        .then(|| nvs_stdlib::test_outcome(&test.class, &test.label, attempt, &failures));
    let thrown = match nvs_runtime::call_hook(ctx, receiver, &hook.method, argument) {
        Ok(()) => None,
        Err(status) if status == nvs_runtime::EXITED => return Outcome::Exited(ctx.exit_code()),
        Err(_) => Some(ctx.take_thrown().message().to_owned()),
    };
    let mut reasons: Vec<String> = ctx
        .take_assertions()
        .iter()
        .filter_map(|entry| entry.failure.clone())
        .collect();
    if reasons.is_empty() {
        reasons.extend(thrown);
    }
    reasons.into_iter().fold(outcome, |outcome, reason| {
        outcome.and_failed(format!(
            "the `#[AfterEachTest]` method `{}` failed: {reason}",
            hook.method
        ))
    })
}

/// The verdict of one call to a test method, read off its status and the
/// ledger, both of which it takes.
///
/// A `bench` passes with an empty ledger: what it claims is its rows, and a failed assertion
/// in it still fails it (`rule:testing/bench-counters`).
fn judge(ctx: &mut nvs_runtime::Ctx, outcome: Result<(), i32>, bench: bool) -> Outcome {
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
    let asserted = !ledger.is_empty();
    let failures: Vec<Record> = ledger.into_iter().filter_map(assertion_record).collect();
    if !failures.is_empty() {
        return Outcome::Failed(failures);
    }
    if let Some(message) = thrown {
        return Outcome::failed(message);
    }
    if !asserted && !bench {
        // `rule:testing/runner-is-strict`'s first bullet, with the way out that section names: a test
        // whose whole claim is that nothing threw writes
        // `Core\Test::assertDoesNotThrow(callable)`, which records an entry
        // like any other assertion.
        return Outcome::failed(
            "it asserted nothing: `rule:testing/runner-is-strict` fails a test whose ledger is empty — write \
             `Core\\Test::assertDoesNotThrow` where the claim is that a call completes",
        );
    }
    Outcome::Passed
}

/// The calls one `#[Test]` method makes — `rule:testing/data-rows`'s "each row is its own
/// reported case".
///
/// A method with no `#[TestWith]` row is `rule:testing/test-attribute`'s single call under its own name.
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
/// what `rule:testing/runner-is-strict` skips is a *test*, and each row is one.
fn invocations(case: &nvs_types::testing::TestCase) -> Vec<Invocation<'_>> {
    if case.rows.is_empty() {
        return vec![Invocation {
            case,
            label: case.method.clone(),
            row: None,
            property: None,
        }];
    }
    case.rows
        .iter()
        .enumerate()
        .map(|(index, row)| Invocation {
            case,
            label: format!("{}#{index}", case.method),
            row: Some(row.as_slice()),
            property: None,
        })
        .collect()
}

/// The `#[Test]` row a `#[Property]` method is run under: its name, span and options, and no
/// injected parameter or data row. Every option reader below reads it as it reads a test's,
/// which is what keeps `skip:`, `timeout:`, `tags:`, `server:`, `db:`, `at:` and `seed:` one
/// reading each. `gen:` and `runs:` stay on the [`nvs_types::property::PropertyCase`].
fn stand_in(property: &nvs_types::property::PropertyCase) -> nvs_types::testing::TestCase {
    nvs_types::testing::TestCase {
        method: property.method.clone(),
        span: property.span,
        options: property.options.clone(),
        params: Vec::new(),
        rows: Vec::new(),
        bench: false,
    }
}

/// Every call one class makes: each `#[Test]` method's [`invocations`], then one call per
/// `#[Property]` method under its own name (`rule:testing/property-testing`: a property is one
/// test). `stand_ins` is [`stand_in`] of each of `properties`, in the same order.
fn class_calls<'a>(
    tests: &'a [nvs_types::testing::TestCase],
    properties: &'a [nvs_types::property::PropertyCase],
    stand_ins: &'a [nvs_types::testing::TestCase],
) -> Vec<Invocation<'a>> {
    let mut calls: Vec<Invocation<'a>> = tests.iter().flat_map(invocations).collect();
    calls.extend(
        properties
            .iter()
            .zip(stand_ins)
            .map(|(property, case)| Invocation {
                case,
                label: property.method.clone(),
                row: None,
                property: Some(property),
            }),
    );
    calls
}

/// One call [`invocations`] hands back: the method to run, the label it is
/// reported under, and the row it is filled from — `None` for the `rule:testing/test-attribute` method
/// that has none.
struct Invocation<'a> {
    case: &'a nvs_types::testing::TestCase,
    label: String,
    row: Option<&'a [Option<ConstArg>]>,
    /// The generators of a `#[Property]` method, whose `case` is its [`stand_in`].
    property: Option<&'a nvs_types::property::PropertyCase>,
}

/// One folded row field as the value the call takes, owned by `row`.
///
/// The roster is exactly what `nvs_types::defaults::literal_default` folds a
/// row field to — the scalars and a `string` — because `rule:testing/data-rows` matches a
/// field against its parameter's *declared* type, and that is the closed list
/// of literals such a parameter can be declared at, plus `null` for a `?T`
/// one. Anything else answers
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
        // A `null` row field, which the checker admits only at a parameter
        // whose type has `null` in it. It carries no reference.
        ConstArg::Null => nvs_runtime::Value::null(),
        _ => return None,
    })
}

/// The `#[Fixture]` methods `case` asks for, in parameter order.
///
/// A `rule:testing/fixtures` and `rule:testing/data-rows` parameter list is a list of *sources*
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
/// `rule:testing/runner-is-strict` makes the reason the whole point of the option — `#[Test(skip: true)]`
/// does not compile — so a `skip` that folded to anything but a string is one
/// `nvs_types::testing` has already refused, and reading it back as absent
/// here would be a second answer to a settled question.
/// `rule:testing/a-test-has-a-wall-time-limit`'s limit for one test, written into `ctx`'s
/// configuration, and the words a failure names it by. `Ok(None)` for a test under no limit.
///
/// A `timeout:` is set the way `Core\Config::set("wall_time", …)` sets it, so `[limits.hard]`
/// bounds it as it bounds a request's own setting. `Err` is the failure of a test whose `timeout:`
/// is above that bound, and nothing of it has run.
///
/// **What it spends:** one deadline per test in flight, the one a request carries, and a thread
/// of [`crate::wall`]'s for each test that has a limit.
///
/// `fallback` is the limit of a test that has neither, which only a mutant's run gives
/// (`rule:testing/mutation-testing`).
fn wall_limit(
    case: &nvs_types::testing::TestCase,
    ctx: &mut nvs_runtime::Ctx,
    fallback: Option<Duration>,
) -> Result<Option<String>, Outcome> {
    let timeout = case.options.iter().find_map(|(name, value)| match value {
        ConstArg::Str(text) if name == "timeout" => Some(text.as_str()),
        _ => None,
    });
    if let Some(text) = timeout {
        let accepted = ctx
            .config_mut()
            .is_some_and(|config| config.set("wall_time", text));
        if !accepted {
            return Err(Outcome::failed(format!(
                "`timeout: \"{text}\"` is longer than `[limits.hard] wall_time` allows, so the \
                 test did not run"
            )));
        }
        ctx.refresh_limits();
        return Ok(Some(format!(
            "{text}, set by `#[Test(timeout: \"{text}\")]`"
        )));
    }
    let nanos = ctx.safepoint_view().wall_time();
    if nanos != 0 {
        return Ok(Some(format!(
            "{:?}, set by `[limits] wall_time`",
            Duration::from_nanos(nanos)
        )));
    }
    let Some(fallback) = fallback else {
        return Ok(None);
    };
    let text = format!("{}ms", fallback.as_millis().max(1));
    let accepted = ctx
        .config_mut()
        .is_some_and(|config| config.set("wall_time", &text));
    if !accepted {
        return Ok(None);
    }
    ctx.refresh_limits();
    Ok(Some(format!("{fallback:?}, set by `--mutate`")))
}

/// `rule:testing/tags-and-shards-select-what-runs`'s tags of one test, as written.
fn tags_of(case: &nvs_types::testing::TestCase) -> Vec<&str> {
    case.options
        .iter()
        .find_map(|(name, value)| match value {
            ConstArg::Array(entries) if name == "tags" => Some(
                entries
                    .iter()
                    .filter_map(|(_, tag)| match tag {
                        ConstArg::Str(tag) => Some(tag.as_str()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn skip_reason(case: &nvs_types::testing::TestCase) -> Option<String> {
    case.options.iter().find_map(|(name, value)| match value {
        ConstArg::Str(reason) if name == "skip" => Some(reason.clone()),
        _ => None,
    })
}

/// How the run came out, in the counts every format's summary carries.
#[derive(Clone, Copy, Default)]
struct Counts {
    passed: usize,
    failed: usize,
    skipped: usize,
    /// `rule:testing/runner-is-strict`'s own section: a test that passed within its retry allowance is
    /// counted here and in none of the fields above it.
    flaky: usize,
}

impl Counts {
    fn total(self) -> usize {
        self.passed + self.failed + self.skipped + self.flaky
    }
}

/// `rule:testing/report-formats`'s one line per test: the mark, the method's own name, and how long it
/// took, written to the suite's own output. A bench's rows follow on a line of their own.
fn report(
    ctx: &mut nvs_runtime::Ctx,
    method: &str,
    outcome: &Outcome,
    elapsed: Duration,
    rows: Option<Rows>,
) {
    let mark = outcome.mark();
    let mut text = format!(
        "    {mark} {method:<32} {:.1} ms\n",
        elapsed.as_secs_f64() * 1000.0
    );
    let mut line = |line: &str| {
        text.push_str("      ");
        text.push_str(line);
        text.push('\n');
    };
    if let Some(rows) = rows {
        line(&rows.line());
    }
    match outcome {
        Outcome::Passed => {}
        Outcome::Skipped(reason) => line(&format!("skipped: {reason}")),
        Outcome::Failed(reasons) => reasons.iter().for_each(|reason| line(message_of(reason))),
        Outcome::Flaky { attempts, failures } => {
            line(&format!(
                "flaky: it passed on attempt {attempts}, having failed with:"
            ));
            failures.iter().for_each(|reason| line(message_of(reason)));
        }
        Outcome::Exited(code) => line(&format!("the test called exit({code})")),
    }
    say(ctx, &text);
}

/// `rule:testing/report-formats`'s versioned JSON: one object per run, over the very [`Case`] list the
/// plaintext report renders a line at a time.
///
/// The version is at the root because that section promises a *versioned*
/// schema, and a consumer that reads it knows what the rest of the keys mean.
/// What `rule:testing/report-formats` additionally names — a structured diff and per-data-row
/// results — is absent because nothing produces any of it yet; each arrives as a new key beside
/// these, which is the whole of what the version number buys.
///
/// **A `#[Bench]` has `counts` and `advisoryWallMs`** (`rule:testing/bench-counters`):
/// `{statements, calls, allocations, bytes, gc}`, then the wall time in milliseconds, named
/// advisory because it changes from run to run. They are new keys, not a new version.
///
/// **A failed `#[Property]` has `counterexample`**: `{seed, call, inputs, generatedInputs}`,
/// each input as the report prints it, `inputs` shrunk. It is a new key, not a new version: a
/// consumer that does not read it reads the rest unchanged.
///
/// **Version 2 is where a record says where its test is written.** `file`,
/// `line` and `column` are what a Test Explorer needs to place a test in a
/// file, and nothing below the compiler can recover them: a class label and a
/// method name say nothing about which file declared the two. The version went
/// up rather than a second document being written beside this one, because
/// this report is read by CI as well, and one schema with a number on it is
/// what `rule:ide/ast-json-schema-is-frozen` already settled as the shape for
/// exactly this.
///
/// **Version 3 is where a record says which lines its test reached**, and it
/// is written only under a coverage flag: `sites` is `Some` then, and every
/// record gains `coverage`, each file name mapped to the sorted lines the
/// test reached in any attempt (`crate::coverage::Sites::reached`). A run
/// without the flag writes version 2 unchanged.
///
/// **Version 4 adds `mutants`**, written only under `--mutate`
/// (`rule:testing/mutation-testing`): one `{file, line, change, outcome,
/// tests}` record per mutant, after `tests`. A record's `tests` is how many
/// tests reached the mutant's line.
fn json_document(
    cases: &[Case],
    counts: Counts,
    total: Duration,
    sites: Option<&crate::coverage::Sites>,
    mutants: Option<&[mutation::Scored]>,
) -> String {
    let version = match (mutants.is_some(), sites.is_some()) {
        (true, _) => 4,
        (false, true) => 3,
        (false, false) => 2,
    };
    let mut out = format!("{{\n  \"schemaVersion\": {version},\n  \"summary\": {{");
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
            // was retried at all. `rule:testing/runner-is-strict`'s "never as green" is the `verdict`
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
                    json_string(message_of(reason), &mut out);
                }
                out.push(']');
                let found = reasons.iter().rev().find_map(|reason| {
                    reason
                        .envelope
                        .fields
                        .iter()
                        .find(|(name, _)| name == COUNTEREXAMPLE)
                });
                if let Some((_, found)) = found {
                    out.push_str(", \"counterexample\": ");
                    json_node(found, &mut out);
                }
            }
            Outcome::Exited(code) => out.push_str(&format!(", \"exitCode\": {code}")),
        }
        if let Some(rows) = case.rows {
            let (statements, calls, allocations, bytes, gc) = rows.counts();
            out.push_str(&format!(
                ", \"counts\": {{\"statements\": {statements}, \"calls\": {calls}, \
                 \"allocations\": {allocations}, \"bytes\": {bytes}, \"gc\": {gc}}}, \
                 \"advisoryWallMs\": {:.3}",
                millis(rows.wall)
            ));
        }
        if let Some(sites) = sites {
            out.push_str(", \"coverage\": {");
            let reached = sites.reached(case.reached.as_deref().unwrap_or_default());
            for (nth, (file, lines)) in reached.iter().enumerate() {
                if nth > 0 {
                    out.push_str(", ");
                }
                json_string(file, &mut out);
                out.push_str(": [");
                let lines: Vec<String> = lines.iter().map(usize::to_string).collect();
                out.push_str(&lines.join(", "));
                out.push(']');
            }
            out.push('}');
        }
        out.push('}');
    }
    out.push_str(if cases.is_empty() { "]" } else { "\n  ]" });
    if let Some(mutants) = mutants {
        out.push_str(",\n  \"mutants\": [");
        for (at, mutant) in mutants.iter().enumerate() {
            out.push_str(if at == 0 { "\n    {" } else { ",\n    {" });
            out.push_str("\"file\": ");
            json_string(&mutant.file, &mut out);
            out.push_str(&format!(", \"line\": {}, \"change\": ", mutant.line));
            json_string(&mutant.change, &mut out);
            out.push_str(", \"outcome\": ");
            json_string(mutant.verdict.name(), &mut out);
            out.push_str(&format!(", \"tests\": {}}}", mutant.tests));
        }
        out.push_str(if mutants.is_empty() { "]" } else { "\n  ]" });
    }
    out.push_str("\n}\n");
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
fn listing(checked: &crate::Checked, select: &Selection) -> Vec<Listed> {
    let mut listed = Vec::new();
    for class in checked.exprs.test_classes() {
        let tests = checked.exprs.tests(class).unwrap_or_default();
        let properties = checked.exprs.properties(class).unwrap_or_default();
        let stand_ins: Vec<_> = properties.iter().map(stand_in).collect();
        for call in class_calls(tests, properties, &stand_ins) {
            if select.selects(class, &call) {
                listed.push(Listed {
                    class: class.to_owned(),
                    method: call.label,
                    declared: declared_at(call.case.span, &checked.map),
                });
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

/// `nvs test --list` over `.nvst` trees: each case `select` picks, by the
/// path the run's report names it by.
///
/// The selection is `nvs_test::selected`, the one a run of the same trees
/// makes, so a listing never names a case the run would leave out. Only the
/// directories are walked: a case that does not parse is still listed, and a
/// run reports it as a failure.
pub(crate) fn list_cases(
    paths: &[PathBuf],
    only: Option<&[PathBuf]>,
    select: &Selection,
    format: Format,
) -> ExitCode {
    let chosen = nvs_test::selected(paths, only, select.filter.as_deref(), select.shard);
    let labels: Vec<String> = match chosen {
        Ok((_, chosen)) => chosen.into_iter().map(|(label, _)| label).collect(),
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    match format {
        Format::Json => print!("{}", case_listing_document(&labels)),
        _ => {
            for label in &labels {
                println!("{label}");
            }
        }
    }
    ExitCode::SUCCESS
}

/// The discovery document of a case tree, in [`listing_document`]'s schema.
///
/// **`case` in place of `class` and `method`**, because a case is a file and
/// has neither: a consumer tells the two kinds of record apart by the key
/// that names the test. The location is the file's first line, so an editor
/// opens a case where it opens any other file.
fn case_listing_document(labels: &[String]) -> String {
    let mut out = String::from("{\n  \"schemaVersion\": 2,\n  \"listed\": [");
    for (at, label) in labels.iter().enumerate() {
        out.push_str(if at == 0 { "\n    {" } else { ",\n    {" });
        out.push_str("\"case\": ");
        json_string(label, &mut out);
        let declared = Declared {
            file: label.clone(),
            line: 1,
            column: 1,
        };
        location_keys(Some(&declared), &mut out);
        out.push('}');
    }
    out.push_str(if labels.is_empty() {
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

/// `rule:testing/report-formats`'s JUnit XML: the shape every CI system already ingests, over the same
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
                        xml_text(message_of(reason), &mut out);
                        out.push('\n');
                    }
                    out.push_str("</flakyFailure>\n    </testcase>\n");
                }
                Outcome::Failed(_) | Outcome::Exited(_) => {
                    out.push_str(">\n      <failure message=\"");
                    xml_text(&case.outcome.message().unwrap_or_default(), &mut out);
                    out.push_str("\">");
                    // The body carries every failed assertion where the
                    // attribute carries the first: `rule:testing/failure-ledger`'s ledger keeps them all,
                    // and a format that reported one would hide what it exists
                    // to keep.
                    if let Outcome::Failed(reasons) = &case.outcome {
                        for reason in reasons {
                            xml_text(message_of(reason), &mut out);
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

/// `node` as JSON, for the nodes [`counterexample_record`] builds: a map, a sequence, a string
/// and a number. Any other node is `null`.
fn json_node(node: &Node, out: &mut String) {
    match node {
        Node::Map(entries) => {
            out.push('{');
            for (nth, (key, value)) in entries.iter().enumerate() {
                if nth > 0 {
                    out.push_str(", ");
                }
                json_string(key.as_str(), out);
                out.push_str(": ");
                json_node(value, out);
            }
            out.push('}');
        }
        Node::Sequence(items) => {
            out.push('[');
            for (nth, item) in items.iter().enumerate() {
                if nth > 0 {
                    out.push_str(", ");
                }
                json_node(item, out);
            }
            out.push(']');
        }
        Node::Scalar(Scalar::Str { text, .. }) => json_string(text.as_str(), out),
        Node::Scalar(Scalar::Int(number)) => out.push_str(&number.to_string()),
        Node::Scalar(Scalar::Uint(number)) => out.push_str(&number.to_string()),
        _ => out.push_str("null"),
    }
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
pub(crate) fn xml_text(text: &str, out: &mut String) {
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

/// Every `rule:testing/inline-snapshots` row the compile recorded, with its file resolved.
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
/// what makes the workflow `rule:testing/inline-snapshots` describes work at all: it starts every
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
/// `rule:testing/inline-snapshots` is written on: the snapshot's whole point is that the diff is read,
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
    use super::{
        Format, Outcome, Selection, Shape, compile, json_document, junit_document,
        run_suite_in_a_task,
    };

    /// The shape every test here runs under: collected rather than printed, as
    /// many tests at once as `nvs test` runs by default.
    fn collected() -> Shape {
        Shape {
            format: Format::Json,
            jobs: super::default_jobs(),
            show_output: false,
            fail_fast: false,
            shuffle: None,
            wall: None,
        }
    }

    /// A program under `crates/nvs-cli/tests/fixtures/runner/`, reached
    /// through `nvs_repo` so `cargo test`'s working directory does not matter.
    fn fixture(name: &str) -> std::path::PathBuf {
        nvs_repo::path("crates/nvs-cli/tests/fixtures/runner").join(name)
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
        verdicts_on(
            &fixture(name),
            crate::script::granting_ctx(),
            filter,
            collected(),
        )
    }

    /// [`verdicts_filtered`] over a context the caller built, for the one
    /// fixture whose configuration is more than a grant: `rule:testing/db-transaction` needs a
    /// `[db.<name>]` block, and a fixture has no `nvs.toml` beside it to carry
    /// one.
    fn verdicts_on(
        path: &std::path::Path,
        mut ctx: nvs_runtime::Ctx,
        filter: Option<&str>,
        shape: Shape,
    ) -> Vec<(String, &'static str, Vec<String>)> {
        let checked = crate::front_end(path).expect("the fixture is a program");
        // No cache: a fixture's verdicts are about the runner, and a suite that
        // read one would be asserting against whatever a previous test run left
        // in this account's cache directory.
        let (unit, _) = compile(&checked, None, false).expect("the fixture compiles");
        unit.install_in(&mut ctx);
        // Through the same entry `run` takes, scheduler and all: a suite run
        // off a bare stack would be a different runner from the one shipped,
        // and `rule:testing/task-tree-and-virtual-clock` is a claim about the one with a task tree under it.
        let (suite, _ctx) = run_suite_in_a_task(
            &unit,
            ctx,
            std::rc::Rc::new(checked),
            &nvs_config::Config::default(),
            None,
            shape,
            &Selection::by_name(filter),
        )
        .expect("the suite's own task runs");
        assert_eq!(
            std::sync::Arc::strong_count(&unit),
            1,
            "every isolate's program is dropped with it, so the code it shared \
             is left with one owner"
        );
        suite
            .cases
            .into_iter()
            .map(|case| {
                let failures = match &case.outcome {
                    Outcome::Failed(failures) => failures
                        .iter()
                        .map(|record| super::message_of(record).to_owned())
                        .collect(),
                    _ => Vec::new(),
                };
                (case.method, case.outcome.verdict(), failures)
            })
            .collect()
    }

    /// One fixture's whole suite, collected rather than reported, for the tests
    /// that are about what a *document* says.
    fn ran(name: &str) -> super::Suite {
        ran_selected(name, &Selection::default())
    }

    /// [`ran`] under a selection of the caller's.
    fn ran_selected(name: &str, select: &Selection) -> super::Suite {
        ran_shaped(name, select, collected())
    }

    /// [`ran_selected`] under a shape of the caller's.
    fn ran_shaped(name: &str, select: &Selection, shape: Shape) -> super::Suite {
        let path = fixture(name);
        let checked = crate::front_end(&path).expect("the fixture is a program");
        let (unit, _) = compile(&checked, None, false).expect("the fixture compiles");
        let mut ctx = crate::script::granting_ctx();
        unit.install_in(&mut ctx);
        let (suite, _ctx) = run_suite_in_a_task(
            &unit,
            ctx,
            std::rc::Rc::new(checked),
            &nvs_config::Config::default(),
            None,
            shape,
            select,
        )
        .expect("the suite's own task runs");
        suite
    }

    /// `rule:testing/bench-counters`: `--bench` runs the benches and no test, a plain run runs
    /// the test and leaves no bench out, and two runs of one bench report the same rows.
    #[test]
    fn a_bench_reports_the_same_rows_on_every_run() {
        type Counted = (u64, u64, usize, usize, u64);
        let bench = Selection {
            bench: true,
            ..Selection::default()
        };
        // `--bench` runs one bench at a time, as `Args::run` sets it.
        let shape = Shape {
            jobs: 1,
            ..collected()
        };
        let rows = |suite: super::Suite| -> Vec<(String, Option<Counted>)> {
            suite
                .cases
                .into_iter()
                .map(|case| (case.method, case.rows.map(super::Rows::counts)))
                .collect()
        };
        let first = rows(ran_shaped("bench.nvs", &bench, shape));
        let second = rows(ran_shaped("bench.nvs", &bench, shape));
        let names: Vec<&str> = first.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["buildsAList", "skipped"]);
        let Some((_, Some((statements, calls, allocations, bytes, gc)))) = first.first() else {
            panic!("the bench that ran has rows: {first:?}");
        };
        // 20 calls of `of`, each making 100 calls of `square`.
        assert_eq!(*calls, 2020);
        assert!(
            *statements > 0 && *allocations > 0 && *bytes > 0 && *gc == 0,
            "{first:?}"
        );
        assert_eq!(first.get(1).map(|(_, rows)| *rows), Some(None));
        assert_eq!(first, second, "a bench's rows are the same on every run");

        let plain = ran_shaped("bench.nvs", &Selection::default(), shape);
        assert_eq!(plain.left_out, 0, "a plain run leaves no bench out");
        let names: Vec<&str> = plain.cases.iter().map(|case| case.method.as_str()).collect();
        assert_eq!(names, ["aTest"]);
        assert!(plain.cases.iter().all(|case| case.rows.is_none()));
    }

    /// `rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle`: after the first failure
    /// no test starts, not even in a later class, and the suite counts every one of them.
    #[test]
    fn fail_fast_starts_no_test_after_the_first_failure() {
        let shape = Shape {
            jobs: 1,
            fail_fast: true,
            ..collected()
        };
        let suite = ran_shaped("fail-fast.nvs", &Selection::default(), shape);
        assert_eq!(
            names(&suite),
            ["AlphaTest::itPassesFirst", "AlphaTest::itFails"]
        );
        assert_eq!(suite.counts.failed, 1);
        assert_eq!(
            suite.not_run, 5,
            "two rows and one test of AlphaTest, and BetaTest's two"
        );

        // Without the flag every test runs, and the run fails twice.
        let whole = ran_shaped(
            "fail-fast.nvs",
            &Selection::default(),
            Shape {
                jobs: 1,
                ..collected()
            },
        );
        assert_eq!(whole.cases.len(), 7);
        assert_eq!((whole.counts.failed, whole.not_run), (2, 0));
    }

    /// `rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle`: a shuffled run reports in
    /// declaration order with the same verdicts, on one core and on many.
    #[test]
    fn a_shuffled_run_reports_what_a_plain_run_reports() {
        let report = |suite: &super::Suite| -> Vec<(String, &'static str)> {
            suite
                .cases
                .iter()
                .map(|case| {
                    (
                        format!("{}::{}", case.class, case.method),
                        case.outcome.verdict(),
                    )
                })
                .collect()
        };
        let plain = report(&ran("fail-fast.nvs"));
        for jobs in [1, 4] {
            for seed in [1, 2, 3] {
                let shape = Shape {
                    jobs,
                    shuffle: Some(seed),
                    ..collected()
                };
                let shuffled = ran_shaped("fail-fast.nvs", &Selection::default(), shape);
                assert_eq!(report(&shuffled), plain, "--jobs {jobs} --shuffle={seed}");
            }
        }
    }

    /// One seed gives one order, every order is a permutation, and the seeds do not all give
    /// the order the tests are written in.
    #[test]
    fn a_shuffle_seed_repeats_its_order() {
        let order = |seed| {
            let mut items: Vec<usize> = (0..8).collect();
            super::Order(seed).shuffle(&mut items);
            items
        };
        assert_eq!(order(42), order(42));
        let mut sorted = order(42);
        sorted.sort_unstable();
        assert_eq!(sorted, (0..8).collect::<Vec<_>>());
        let moved = (0..10).filter(|&seed| order(seed) != (0..8).collect::<Vec<_>>());
        assert!(
            moved.count() >= 9,
            "nearly every seed changes the order of eight tests"
        );
    }

    /// `rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle`: a run's failed tests are
    /// recorded, the record selects exactly them and their data rows, and a run that failed
    /// nothing records nothing to rerun.
    #[test]
    fn rerun_failed_runs_the_tests_the_last_run_failed() {
        let scratch = nvs_repo::scratch("runner-rerun-failed");
        let file = scratch.join("failed-tests").join("program.txt");
        let first = ran("fail-fast.nvs");
        super::record_failed_tests(&file, &first.cases);
        let only = super::read_failed_tests(&file).expect("two tests failed");
        assert_eq!(only.len(), 2, "{only:?}");
        assert!(only.contains("AlphaTest::itFails"), "{only:?}");

        let select = Selection {
            only: Some(only.clone()),
            ..Selection::default()
        };
        let rerun = ran_selected("fail-fast.nvs", &select);
        let mut again = names(&rerun);
        again.sort();
        assert_eq!(again, only.into_iter().collect::<Vec<_>>());
        assert_eq!(rerun.left_out, 5);

        let passing = ran("tags-and-shards.nvs");
        super::record_failed_tests(&file, &passing.cases);
        assert_eq!(super::read_failed_tests(&file), None, "nothing failed");
    }

    /// Each reported case of `suite` as `Class::label`.
    fn names(suite: &super::Suite) -> Vec<String> {
        suite
            .cases
            .iter()
            .map(|case| format!("{}::{}", case.class, case.method))
            .collect()
    }

    /// `rule:testing/tags-and-shards-select-what-runs`: a test runs when it has one `--tag` tag
    /// and no `--skip-tag` tag, a data row goes with its test, and the suite counts every test
    /// the selection left out.
    #[test]
    fn tags_select_what_runs_and_the_suite_counts_what_they_left_out() {
        let select = Selection {
            tags: vec!["db".to_owned(), "mail".to_owned()],
            skip_tags: vec!["slow".to_owned()],
            ..Selection::default()
        };
        let suite = ran_selected("tags-and-shards.nvs", &select);
        let ran = names(&suite);
        assert_eq!(ran.len(), 4, "{ran:?}");
        for name in ["ShopTest::itSavesAnOrder", "ShopTest::itSendsAReceipt"] {
            assert!(
                ran.iter().any(|ran| ran == name),
                "{name} did not run: {ran:?}"
            );
        }
        let rows = ran
            .iter()
            .filter(|name| name.starts_with("BlogTest::itNamesALength"));
        assert_eq!(rows.count(), 2, "{ran:?}");
        assert_eq!(suite.left_out, 4, "two of ShopTest and two of BlogTest");
    }

    /// `rule:testing/tags-and-shards-select-what-runs`: the parts of a run are disjoint, together
    /// they are the whole run, and every data row of a test is in the test's part.
    #[test]
    fn the_shards_of_a_run_are_disjoint_and_together_are_the_whole_run() {
        let whole = names(&ran("tags-and-shards.nvs"));
        let mut parts = Vec::new();
        for n in 1..=3 {
            let select = Selection {
                shard: Some(format!("{n}/3").parse().expect("a well-formed shard")),
                ..Selection::default()
            };
            let suite = ran_selected("tags-and-shards.nvs", &select);
            let part = names(&suite);
            assert_eq!(part.len() + suite.left_out, whole.len(), "part {n}/3");
            let rows = part
                .iter()
                .filter(|name| name.contains("itNamesALength"))
                .count();
            assert!(
                rows == 0 || rows == 2,
                "part {n}/3 split the rows of one test: {part:?}"
            );
            parts.extend(part);
        }
        let mut whole = whole;
        whole.sort();
        parts.sort();
        assert_eq!(parts, whole);
    }

    /// [`ran`] as `rule:testing/report-formats`'s JSON, parsed back.
    ///
    /// The document rather than the [`super::Suite`] it is built from, because
    /// what a Test Explorer and a CI job read is this text: a field the run
    /// resolved and the writer forgot to print is exactly the failure the test
    /// below exists to catch.
    fn json_report(name: &str) -> serde_json::Value {
        let suite = ran(name);
        let document = json_document(
            &suite.cases,
            suite.counts,
            std::time::Duration::ZERO,
            None,
            None,
        );
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

    /// `--list` over a case tree names each case by the path a run reports it
    /// by, in the program listing's document. A folder with programs and
    /// cases in it lists its cases, and a cache folder in the tree is skipped.
    #[test]
    fn the_listing_mode_lists_a_case_tree() {
        let scratch = nvs_repo::scratch("runner-list-cases");
        let tree = scratch.join("tree");
        let write = |at: &str, text: &str| {
            let path = tree.join(at);
            std::fs::create_dir_all(path.parent().expect("a file has a folder"))
                .expect("the folder is made");
            std::fs::write(&path, text).expect("the file is written");
        };
        let case = "--FILE--\necho 1;\n--EXPECT--\n1\n";
        write("first.nvst", case);
        write("more/second.nvst", case);
        write("app.nvs", "echo 1;\n");
        write("cache/copied.nvst", case);
        write(
            "cache/CACHEDIR.TAG",
            "Signature: 8a477f597d28d172789f06886806bc55\n",
        );
        assert!(
            crate::holds_cases(&tree),
            "a folder with a case in it is a case tree under `--list`"
        );

        let paths = [tree.clone()];
        let (_, chosen) = nvs_test::selected(&paths, None, None, None).expect("the tree is walked");
        let labels: Vec<String> = chosen.into_iter().map(|(label, _)| label).collect();
        assert_eq!(
            labels.len(),
            2,
            "the cache folder's case is skipped: {labels:?}"
        );
        assert!(labels[0].ends_with("first.nvst") && labels[1].ends_with("second.nvst"));

        let document: serde_json::Value =
            serde_json::from_str(&super::case_listing_document(&labels))
                .expect("the listing is a JSON document");
        assert_eq!(document["schemaVersion"], 2);
        let listed = document["listed"]
            .as_array()
            .expect("records under `listed`");
        for (record, label) in listed.iter().zip(&labels) {
            assert_eq!(record["case"], label.as_str());
            assert_eq!(record["file"], label.as_str());
            assert_eq!(
                (record["line"].as_u64(), record["column"].as_u64()),
                (Some(1), Some(1))
            );
            assert!(record["class"].is_null() && record["verdict"].is_null());
        }

        // `--filter` selects the same cases a run of the tree would.
        let (_, one) =
            nvs_test::selected(&paths, None, Some("second"), None).expect("the tree is walked");
        assert_eq!(one.len(), 1);
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
        let listed = super::listing(&checked, &Selection::default());
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
        let one = super::listing(
            &checked,
            &Selection::by_name(Some("StaticsTest::itReadsBack")),
        );
        assert_eq!(
            one.len(),
            1,
            "a filter selects what it would select in a run"
        );
    }

    /// The location is the JSON document's alone: the other two formats are
    /// the bytes they were before it existed.
    ///
    /// `rule:testing/report-formats`'s JUnit XML is asserted here attribute by attribute, because "the
    /// same bytes as before" is only checkable against the shape the writer
    /// emits — a `file=` or `line=` attribute added to a `<testcase>` would be
    /// a schema change nobody versioned, this format having no version to
    /// raise. The plaintext report's bytes are pinned where it is rendered, by
    /// `tests/conformance/lang/a-test-attribute-builds-a-table-the-runner-reports.nvst`,
    /// which this slice left untouched.
    #[test]
    fn the_schema_version_is_two_and_junit_and_human_are_byte_identical() {
        let suite = ran("statics-are-fresh.nvs");
        let json = json_document(
            &suite.cases,
            suite.counts,
            std::time::Duration::ZERO,
            None,
            None,
        );
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

    /// `rule:errors/record-producers`' test-result producer: a failed
    /// `assertSame` is a record whose message is the line every report
    /// prints, and whose two nodes are the actual and expected values.
    #[test]
    fn a_failed_comparison_is_a_record_with_actual_and_expected_as_nodes() {
        use nvs_render::{Node, Scalar};

        let suite = ran("parallel-output.nvs");
        let Outcome::Failed(failures) = &suite.cases[1].outcome else {
            panic!("`itFails` failed");
        };
        let [record] = failures.as_slice() else {
            panic!("one failed assertion is one record: {failures:?}");
        };
        assert!(
            super::message_of(record).starts_with("Core\\Test::assertSame failed"),
            "the message is the failure's line: {record:?}"
        );
        assert!(
            record.envelope.fields.iter().any(|(name, node)| name == super::ASSERTION
                && matches!(node, Node::Scalar(Scalar::Str { text, .. }) if text.as_str() == "assertSame")),
            "the record names the method: {record:?}"
        );
        assert_eq!(
            record.nodes,
            vec![Node::Scalar(Scalar::Int(1)), Node::Scalar(Scalar::Int(2))],
            "actual first, expected second"
        );
        let json = json_document(
            &suite.cases,
            suite.counts,
            std::time::Duration::ZERO,
            None,
            None,
        );
        assert!(
            json.contains(&format!(
                "\"failures\": [\"{}",
                "Core\\\\Test::assertSame failed"
            )),
            "the JSON report prints the record's message: {json}"
        );
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
    /// this asserts about at all. `docker compose -f tests/db/compose.yaml up`
    /// brings both up, and then the assertion below runs.
    fn compose_postgres() -> Option<nvs_config::tree::Database> {
        let bundle = nvs_repo::path("tests/db/ca.crt");
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

    /// A context whose configuration is `block` as `[db.test]`, and the grant
    /// to connect to it.
    ///
    /// `rule:core-classes/db-capabilities`'s grant, for the one name the
    /// fixtures open. `net.*` is not granted beside it: a `[db.<name>]`
    /// endpoint is operator-written and so is pre-approved against
    /// `rule:http-server/allow-url-pins-the-address`'s denied ranges
    /// (`nvs_config::tree::CapDb`), and a test that granted both could not tell
    /// which of the two the connection went through.
    fn connecting_to(block: nvs_config::tree::Database) -> nvs_runtime::Ctx {
        let mut snapshot = nvs_config::Snapshot::default();
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
        ctx
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
        // second; one that opened none would fail the third. The four share one
        // table, so they run one at a time: `rule:testing/tests-run-across-every-core`
        // leaves what tests share outside their isolates to the program.
        let Some(block) = compose_postgres() else {
            // Printed rather than silent: a green line for a test that reached
            // no database is exactly the outcome that should be legible.
            eprintln!(
                "skipped: no PostgreSQL on 127.0.0.1:15432, or no `tests/db/ca.crt` — \
                 `docker compose -f tests/db/compose.yaml up -d --wait` is what this needs"
            );
            return;
        };
        let one_at_a_time = Shape {
            jobs: 1,
            ..collected()
        };
        let verdicts = verdicts_on(
            &fixture("db-transaction.nvs"),
            connecting_to(block),
            None,
            one_at_a_time,
        );
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
    fn a_hook_seeds_a_table_before_the_tests_and_drops_it_after_them() {
        // `rule:testing/class-hooks` over the compose PostgreSQL: the setup's
        // effect lives outside every isolate, which is what the hooks exist for.
        // Two runs of one fixture. The first is the class: `#[BeforeClass]`
        // creates and seeds the table, both tests read the seeded rows (one
        // after writing a row its transaction rolls back), and `#[AfterClass]`
        // drops it. The second asks the database whether the table is still
        // there. A hook that never ran fails the first run, and an
        // `#[AfterClass]` that never ran fails the second.
        let Some(block) = compose_postgres() else {
            eprintln!(
                "skipped: no PostgreSQL on 127.0.0.1:15432, or no `tests/db/ca.crt` — \
                 `docker compose -f tests/db/compose.yaml up -d --wait` is what this needs"
            );
            return;
        };
        let path = fixture("class-hooks-seed-and-drop-a-table.nvs");
        let class = verdicts_on(
            &path,
            connecting_to(block.clone()),
            Some("SeededTableTest::"),
            collected(),
        );
        assert_eq!(
            class
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![
                ("itReadsTheSeededRows", "passed"),
                ("itWritesARowThatIsRolledBack", "passed"),
            ],
            "failures: {:?}",
            class
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
        let after = verdicts_on(
            &path,
            connecting_to(block),
            Some("TableIsGoneTest::"),
            collected(),
        );
        assert_eq!(
            after
                .iter()
                .map(|(method, verdict, _)| (method.as_str(), *verdict))
                .collect::<Vec<_>>(),
            vec![("itFindsNoTable", "passed")],
            "`#[AfterClass]` dropped the table: {:?}",
            after
                .iter()
                .flat_map(|(_, _, failures)| failures.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_hook_removes_the_file_each_test_wrote() {
        // `rule:testing/after-each-test` over a real file. Each test writes one
        // into a folder of this case's own and asserts it is there; the hook
        // deletes it after the test, and one of the two tests fails on purpose,
        // because the hook runs after a failed attempt as well. The folder is
        // empty afterwards only if the hook ran after both.
        //
        // The fixture has no `nvs.toml`, so the configuration is written here:
        // `fs.*` for the folder, under the program's grants as the rule says,
        // and `io.temp_root` naming it, which is the key the fixture reads.
        let scratch = nvs_repo::scratch("runner-after-each-test");
        // Canonical, because a grant's roots are compared against the
        // canonical path a door is asked about, and a resolve would have
        // written them that way.
        let folder = nvs_config::trust::canonical(&scratch).expect("the folder is there");
        let folder = folder.to_string_lossy();
        let written = format!(
            "[capabilities.fs]\nread = ['{folder}']\nwrite = ['{folder}']\n\n\
             [io]\ntemp_root = '{folder}'\n"
        );
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        let snapshot = nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..Default::default()
        };
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
        ctx.set_config(std::sync::Arc::new(snapshot));
        let verdicts = verdicts_on(
            &fixture("after-each-test-removes-a-file.nvs"),
            ctx,
            None,
            collected(),
        );
        assert_eq!(
            verdicts
                .iter()
                .map(|(method, verdict, failures)| (method.as_str(), *verdict, failures.len()))
                .collect::<Vec<_>>(),
            vec![
                ("itWritesAFile", "passed", 0),
                ("itWritesAFileAndFails", "failed", 1),
            ],
            "the failing test fails its own assertion and nothing else: {verdicts:?}"
        );
        let left: Vec<_> = std::fs::read_dir(&*scratch)
            .expect("the folder is still there")
            .map(|entry| entry.expect("the folder can be listed").file_name())
            .collect();
        assert!(
            left.is_empty(),
            "`#[AfterEachTest]` removed the file each test wrote, and {left:?} is left"
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
    fn a_property_runs_as_one_test_over_its_generated_inputs() {
        // `rule:testing/property-testing`: a property that holds for every input passes, one
        // that does not fails, shrinks to the smallest input that still fails, and is reported
        // with that input, the generated one and the seed that reproduces them. One whose
        // inputs are a list of shapes, a class built through its constructor, a `?decimal` and
        // `bytes` is called with each of them, and a subclass runs the property it inherits
        // (`rule:testing/hook-inheritance`). `--list` lists each property under its own name.
        let verdicts = verdicts("property.nvs");
        let named: Vec<(&str, &str)> = verdicts
            .iter()
            .map(|(method, verdict, _)| (method.as_str(), *verdict))
            .collect();
        assert_eq!(
            named,
            vec![
                ("itAddsZero", "passed"),
                ("itAddsInEitherOrder", "passed"),
                ("itFailsForANumberAboveTen", "failed"),
                ("itBuildsEveryKindOfInput", "passed"),
            ],
            "{verdicts:?}"
        );
        let failures = &verdicts[2].2;
        assert!(
            failures.iter().any(|failure| failure
                .contains("The smallest inputs that still fail are (11).")
                && failure.contains("#[Property(seed: ")),
            "the failure names the shrunk input and the seed: {failures:?}"
        );
        // Only the last failing call's assertion is reported, beside the counterexample.
        assert_eq!(failures.len(), 2, "{failures:?}");
        let report = json_report("property.nvs");
        let failed = report["tests"]
            .as_array()
            .expect("the report lists its tests")
            .iter()
            .find(|test| test["method"] == "itFailsForANumberAboveTen")
            .expect("the failing property is reported");
        let found = &failed["counterexample"];
        assert_eq!(found["inputs"], serde_json::json!(["11"]), "{failed}");
        assert!(found["seed"].is_i64(), "{failed}");
        assert!(found["call"].is_u64(), "{failed}");
        assert_eq!(
            found["generatedInputs"].as_array().map(Vec::len),
            Some(1),
            "{failed}"
        );
        let checked = crate::front_end(&fixture("property.nvs")).expect("the fixture is a program");
        let listed: Vec<String> = super::listing(&checked, &Selection::default())
            .into_iter()
            .map(|test| test.method)
            .collect();
        assert_eq!(
            listed,
            [
                "itAddsZero",
                "itAddsInEitherOrder",
                "itFailsForANumberAboveTen",
                "itBuildsEveryKindOfInput"
            ]
        );
    }

    #[test]
    fn the_runner_owns_the_tests_task_tree() {
        // `rule:testing/task-tree-and-virtual-clock`, both halves, from the language surface. The first
        // test's `Core\Task::all` has children only because the suite is
        // itself a task (`rule:concurrency/a-child-belongs-to-the-calling-task`'s "a child of the calling task"), so a
        // runner that ran off a bare stack fails it rather than passing it
        // differently. The second leaves a `spawn script` unawaited: `rule:testing/task-tree-and-virtual-clock` says
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

    /// `rule:testing/task-tree-and-virtual-clock`'s budget, accepted side.
    ///
    /// The clock only exists inside a `#[Test(at: ...)]`, so this is the one
    /// place `Core\Test::assertCompletes` can be asked anything but its
    /// refusal — `tests/conformance/core/assert-completes-*` is the other side,
    /// and the split is `Core\Test::advance`'s.
    ///
    /// The advance is asserted with the pass rather than beside it, because a
    /// member that ran the body and answered `void` without moving the clock is
    /// exactly the plausible wrong implementation: it passes on every body that
    /// was going to finish anyway, and buys nothing for the retry and backoff
    /// code the member exists for.
    #[test]
    fn assert_completes_passes_a_body_that_finishes_under_the_clock() {
        let verdicts = verdicts("assert-completes.nvs");
        let (method, verdict, failures) = &verdicts[0];
        assert_eq!(
            (method.as_str(), *verdict),
            ("itPassesABodyThatFinishes", "passed"),
            "failures: {failures:?}"
        );
    }

    /// `rule:testing/task-tree-and-virtual-clock`'s other half at the same boundary: a body that returned with work
    /// of its own still going is the failure, and the message carries the
    /// budget it overran.
    ///
    /// Two failures are reported and that is the point — the member's own, and
    /// the runner's for the tree the test as a whole left behind
    /// ([`Outcome::with_tasks_left_running`]). A member that merely threw
    /// without naming the duration leaves the second line looking identical,
    /// which is why the assertion is on the text and not on the count.
    #[test]
    fn assert_completes_fails_a_body_still_running_naming_the_duration() {
        let verdicts = verdicts("assert-completes.nvs");
        let (method, verdict, failures) = &verdicts[1];
        assert_eq!(
            (method.as_str(), *verdict),
            ("itFailsABodyThatLeavesWorkRunning", "failed"),
            "the body left a child running, so the budget was overrun"
        );
        assert!(
            failures.iter().any(|one| one
                == "Core\\Test::assertCompletes failed: `$body` returned with 1 task(s) of its \
                    own still running, after the 50ms `within` named"),
            "the failure names the budget it overran: {failures:?}"
        );
    }

    /// `rule:observability/speedscope-timeline-export`: `--profile` writes each export, and each
    /// passes its own format's checks. The speedscope file has what the published schema
    /// requires, with open and close events that balance and times that never go back. The
    /// Callgrind file has its header, `fn=` blocks with `calls=` lines, and self costs that sum
    /// to the total.
    // covers: tools:cli/profiling
    #[cfg_attr(
        miri,
        ignore = "compiles and runs a program, which Miri does not emulate"
    )]
    #[test]
    fn a_profile_of_a_suite_passes_each_formats_checks() {
        let dir = nvs_repo::scratch("runner-profile");
        let uncached = nvs_config::Snapshot {
            config: toml::from_str("[opcache]\nfile_cache = false\n")
                .expect("a tree that turns the disk cache off"),
            ..Default::default()
        };
        let profile = |format: crate::profile::Format, name: &str| {
            let out = dir.join(name);
            let checked = crate::front_end(&fixture("profile.nvs")).expect("the fixture compiles");
            let code = super::run(
                checked,
                &uncached,
                None,
                Format::Json,
                &Selection::default(),
                super::Flags {
                    update: false,
                    list: false,
                    jobs: super::default_jobs(),
                    show_output: false,
                    shared_worker: false,
                    fail_fast: false,
                    rerun_failed: false,
                    shuffle: None,
                    record_failed: None,
                    mutate: false,
                    profile: Some((format, Some(out.clone()))),
                },
                &crate::coverage::Requested::default(),
            );
            assert_eq!(code, std::process::ExitCode::SUCCESS, "both tests pass");
            std::fs::read_to_string(&out).expect("the profile was written")
        };

        let speedscope: serde_json::Value =
            serde_json::from_str(&profile(crate::profile::Format::Speedscope, "run.json"))
                .expect("the speedscope file is JSON");
        let frames = check_speedscope(&speedscope);
        for function in ["Steps::double", "Steps::triple"] {
            assert!(
                frames.iter().any(|frame| frame == function),
                "{function} is a frame: {frames:?}"
            );
        }
        let names: Vec<&str> = speedscope["profiles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|profile| profile["name"].as_str())
            .collect();
        for test in ["ProfileTest::itDoubles", "ProfileTest::itTriples"] {
            assert!(
                names.iter().any(|name| name.starts_with(test)),
                "{test} has a timeline: {names:?}"
            );
        }

        let callgrind = profile(crate::profile::Format::Callgrind, "callgrind.out");
        check_callgrind(&callgrind);
        assert!(
            callgrind.contains("fn=Steps::triple\n0 ")
                && callgrind.contains("cfn=Steps::double\ncalls=1 0\n"),
            "`Steps::triple` calls `Steps::double` once: {callgrind}"
        );
        let left: Vec<_> = std::fs::read_dir(&dir)
            .expect("the scratch directory exists")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().ends_with(".part"))
            .collect();
        assert!(left.is_empty(), "every part file is deleted: {left:?}");
    }

    /// What speedscope's `file-format-schema.json` requires of an evented file, and that its
    /// events nest: answers the frame names.
    fn check_speedscope(file: &serde_json::Value) -> Vec<String> {
        assert_eq!(
            file["$schema"],
            nvs_runtime::SPEEDSCOPE_SCHEMA,
            "the schema is named"
        );
        let frames: Vec<String> = file["shared"]["frames"]
            .as_array()
            .expect("`shared.frames` is an array")
            .iter()
            .map(|frame| {
                frame["name"]
                    .as_str()
                    .expect("a frame has a name")
                    .to_owned()
            })
            .collect();
        let profiles = file["profiles"].as_array().expect("`profiles` is an array");
        assert!(
            !profiles.is_empty(),
            "a run that called something has a profile"
        );
        for profile in profiles {
            assert_eq!(profile["type"], "evented");
            assert_eq!(profile["unit"], "nanoseconds");
            assert!(profile["name"].is_string(), "a profile has a name");
            let start = profile["startValue"]
                .as_u64()
                .expect("`startValue` is a number");
            let end = profile["endValue"]
                .as_u64()
                .expect("`endValue` is a number");
            let mut open = Vec::new();
            let mut last = start;
            for event in profile["events"].as_array().expect("`events` is an array") {
                let frame = event["frame"].as_u64().expect("an event names a frame");
                let at = event["at"].as_u64().expect("an event has a time");
                assert!(
                    usize::try_from(frame).is_ok_and(|frame| frame < frames.len()),
                    "{event}"
                );
                assert!(
                    at >= last && at <= end,
                    "`at` never goes back and ends by `endValue`: {event}"
                );
                last = at;
                match event["type"].as_str() {
                    Some("O") => open.push(frame),
                    Some("C") => {
                        assert_eq!(open.pop(), Some(frame), "a close ends the frame on top")
                    }
                    other => panic!("an event is `O` or `C`, not {other:?}"),
                }
            }
            assert!(open.is_empty(), "every open has its close: {profile}");
        }
        frames
    }

    /// Callgrind's header, its `fn=` blocks with their `cfn=` and `calls=` lines, and self costs
    /// that sum to the `summary:` and the `totals:`.
    fn check_callgrind(file: &str) {
        let lines: Vec<&str> = file.lines().collect();
        assert_eq!(lines.first(), Some(&"# callgrind format"));
        assert!(
            lines.iter().any(|line| line.starts_with("events: ")),
            "the header names its events"
        );
        let number = |prefix: &str| -> u64 {
            lines
                .iter()
                .find_map(|line| line.strip_prefix(prefix))
                .and_then(|n| n.trim().parse().ok())
                .unwrap_or_else(|| panic!("`{prefix}` is in the file"))
        };
        let mut selves = 0;
        let mut functions = 0;
        for (at, line) in lines.iter().enumerate() {
            if line.starts_with("fn=") {
                functions += 1;
                let cost = lines.get(at + 1).and_then(|cost| cost.strip_prefix("0 "));
                selves += cost
                    .and_then(|n| n.parse::<u64>().ok())
                    .expect("a `fn=` line has its self cost");
            }
            if line.starts_with("cfn=") {
                assert!(
                    lines
                        .get(at + 1)
                        .is_some_and(|calls| calls.starts_with("calls=")),
                    "a `cfn=` line is followed by its `calls=` line"
                );
                assert!(
                    lines.get(at + 2).is_some_and(|cost| cost.starts_with("0 ")),
                    "a `calls=` line is followed by its inclusive cost"
                );
            }
        }
        assert!(functions > 0, "a run that called something has a function");
        assert_eq!(
            selves,
            number("summary:"),
            "the self costs sum to the summary"
        );
        assert_eq!(
            selves,
            number("totals:"),
            "the self costs sum to the totals"
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
        let dir = nvs_repo::scratch("inline-snapshot");
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
        let _ = super::run(
            checked,
            &uncached,
            None,
            Format::Json,
            &Selection::default(),
            super::Flags {
                update: false,
                list: false,
                jobs: super::default_jobs(),
                show_output: false,
                shared_worker: false,
                fail_fast: false,
                rerun_failed: false,
                shuffle: None,
                record_failed: None,
                mutate: false,
                profile: None,
            },
            &crate::coverage::Requested::default(),
        );
        assert_eq!(
            std::fs::read_to_string(&program).expect("the copy is still there"),
            template,
            "a run that was not asked to update writes nothing at all"
        );

        // When asked.
        let checked = crate::front_end(&program).expect("the copy is a program");
        let _ = super::run(
            checked,
            &uncached,
            None,
            Format::Json,
            &Selection::default(),
            super::Flags {
                update: true,
                list: false,
                jobs: super::default_jobs(),
                show_output: false,
                shared_worker: false,
                fail_fast: false,
                rerun_failed: false,
                shuffle: None,
                record_failed: None,
                mutate: false,
                profile: None,
            },
            &crate::coverage::Requested::default(),
        );
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

        let verdicts = verdicts_on(&program, crate::script::granting_ctx(), None, collected());
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
    }

    /// `rule:testing/tests-run-across-every-core`, one test per claim it makes.
    mod parallel {
        use super::super::{
            Format, Outcome, Selection, Shape, Suite, compile, run_suite_in_a_task,
        };
        use super::fixture;

        /// Runs one fixture's suite under `shape` and hands back the suite,
        /// what the suite's own context printed, and how long the run took.
        fn run(
            name: &str,
            shape: Shape,
            coverage: Option<&mut Option<crate::coverage::Sites>>,
        ) -> (Suite, String, std::time::Duration) {
            let checked = crate::front_end(&fixture(name)).expect("the fixture is a program");
            let (unit, probes) =
                compile(&checked, None, coverage.is_some()).expect("the fixture compiles");
            let mut ctx = crate::script::granting_ctx();
            unit.install_in(&mut ctx);
            if let Some(sites) = coverage {
                let resolved = crate::coverage::Sites::of(probes, &checked.map);
                ctx.set_stmt_hits(Some(std::sync::Arc::new(
                    nvs_runtime::StmtHits::with_edges(resolved.len(), resolved.edges()),
                )));
                ctx.set_debug_flags(
                    ctx.debug_flags()
                        | nvs_runtime::DebugFlags::COVERAGE
                        | nvs_runtime::DebugFlags::BRANCH,
                );
                *sites = Some(resolved);
            }
            let began = std::time::Instant::now();
            let (suite, mut ctx) = run_suite_in_a_task(
                &unit,
                ctx,
                std::rc::Rc::new(checked),
                &nvs_config::Config::default(),
                None,
                shape,
                &Selection::default(),
            )
            .expect("the suite's own task runs");
            let took = began.elapsed();
            let printed = ctx.take_buffered_output().unwrap_or_default();
            (suite, String::from_utf8_lossy(&printed).into_owned(), took)
        }

        /// At least four at once, whatever this machine's thread count, so a
        /// test here asks about overlap and not about the machine.
        fn spread(format: Format) -> Shape {
            Shape {
                format,
                jobs: super::super::default_jobs().max(4),
                show_output: false,
                fail_fast: false,
                shuffle: None,
                wall: None,
            }
        }

        /// Every case's `(method, verdict)`, in report order.
        fn verdicts(suite: &Suite) -> Vec<(&str, &'static str)> {
            suite
                .cases
                .iter()
                .map(|case| (case.method.as_str(), case.outcome.verdict()))
                .collect()
        }

        /// The instants printed after `marker`, in the order they were printed.
        fn instants(printed: &str, marker: &str) -> Vec<i64> {
            printed
                .lines()
                .filter_map(|line| line.trim().strip_prefix(marker))
                .map(|rest| rest.trim().parse().expect("an instant in microseconds"))
                .collect()
        }

        #[test]
        fn parallel_tests_overlap_on_more_than_one_core() {
            // Four tests of 400 ms each, read from the instants they print
            // rather than from the run's wall time, which a loaded machine
            // stretches. Sorted, the starts and the ends of tests run one after
            // another alternate: each test starts after the one before it ended.
            // A start before the previous end is two tests running at once.
            let (suite, printed, _) = run("parallel-overlap.nvs", spread(Format::Json), None);
            assert!(
                verdicts(&suite)
                    .iter()
                    .all(|(_, verdict)| *verdict == "failed"),
                "every test fails on purpose, so its output is printed: {:?}",
                verdicts(&suite)
            );
            assert_eq!(suite.cases.len(), 4);
            let mut started = instants(&printed, "test-started ");
            let mut ended = instants(&printed, "test-ended ");
            assert_eq!(
                (started.len(), ended.len()),
                (4, 4),
                "every test printed its instants: {printed}"
            );
            started.sort_unstable();
            ended.sort_unstable();
            assert!(
                started
                    .iter()
                    .skip(1)
                    .zip(&ended)
                    .any(|(next, end)| next < end),
                "the four tests ran one after another: {printed}"
            );
        }

        #[test]
        fn parallel_class_hooks_wrap_every_test_of_their_class() {
            let (suite, printed, _) = run("parallel-class-hooks.nvs", spread(Format::Json), None);
            assert_eq!(
                verdicts(&suite),
                vec![
                    ("itRunsBesideItsSibling", "failed"),
                    ("itRunsBesideItsSiblingToo", "failed"),
                ],
                "both tests fail on purpose, so their output is printed: {printed}"
            );
            let before = instants(&printed, "before-class-returned ");
            let started = instants(&printed, "test-started ");
            let ended = instants(&printed, "test-ended ");
            let after = instants(&printed, "after-class-started ");
            assert_eq!(
                (before.len(), started.len(), ended.len(), after.len()),
                (1, 2, 2, 1),
                "every hook and test printed its instants: {printed}"
            );
            assert!(
                started.iter().all(|start| *start >= before[0]),
                "`#[BeforeClass]` returned before either test started: {printed}"
            );
            assert!(
                ended.iter().all(|end| *end <= after[0]),
                "`#[AfterClass]` started after both tests ended: {printed}"
            );
            assert!(
                started.iter().max() < ended.iter().min(),
                "the two tests ran at the same time: {printed}"
            );
        }

        #[test]
        fn parallel_report_is_in_declaration_order() {
            let (suite, _, _) = run("parallel-report-order.nvs", spread(Format::Json), None);
            assert_eq!(
                verdicts(&suite),
                vec![
                    ("itFinishesLast", "passed"),
                    ("itFinishesThird", "passed"),
                    ("itFinishesSecond", "passed"),
                    ("itFinishesFirst", "passed"),
                ],
                "the report is in declaration order, not the order the tests finished in"
            );
            assert!(
                suite.cases[0].elapsed > suite.cases[3].elapsed,
                "the first test declared is the last to finish"
            );
        }

        #[test]
        fn parallel_coverage_is_each_tests_own() {
            let source = std::fs::read_to_string(fixture("parallel-coverage.nvs"))
                .expect("the fixture is on disk");
            let line_of = |marker: &str| {
                source
                    .lines()
                    .position(|line| line.contains(marker))
                    .map(|index| index + 1)
                    .expect("the marker is in the fixture")
            };
            let (first, second) = (
                line_of("// only the first test"),
                line_of("// only the second test"),
            );
            let mut sites = None;
            let (suite, _, _) = run(
                "parallel-coverage.nvs",
                spread(Format::Json),
                Some(&mut sites),
            );
            let sites = sites.expect("the run measured coverage");
            let lines = |index: usize| -> Vec<usize> {
                let reached = suite.cases[index]
                    .reached
                    .as_ref()
                    .expect("a run with coverage records what each test reached");
                sites
                    .reached(reached)
                    .into_iter()
                    .flat_map(|(_, lines)| lines)
                    .collect()
            };
            assert_eq!(
                verdicts(&suite),
                vec![
                    ("itRunsTheFirstLine", "passed"),
                    ("itRunsTheSecondLine", "passed"),
                ]
            );
            let (one, two) = (lines(0), lines(1));
            assert!(
                one.contains(&first) && !one.contains(&second),
                "the first test reached line {first} and not line {second}: {one:?}"
            );
            assert!(
                two.contains(&second) && !two.contains(&first),
                "the second test reached line {second} and not line {first}: {two:?}"
            );
        }

        #[test]
        fn a_failed_tests_output_is_printed_under_its_failure() {
            let (suite, printed, _) = run("parallel-output.nvs", spread(Format::Human), None);
            assert_eq!(
                verdicts(&suite),
                vec![("itPasses", "passed"), ("itFails", "failed")]
            );
            let at = |text: &str| {
                printed
                    .find(text)
                    .unwrap_or_else(|| panic!("`{text}` is in the report: {printed}"))
            };
            let line = at("\u{2717} itFails");
            let failure = at("Core\\Test::assertSame failed");
            let output = at("      output:\n        printed by the failing test\n");
            assert!(
                line < failure && failure < output,
                "the output is under the failure, in the test's place: {printed}"
            );
            assert!(
                matches!(&suite.cases[1].outcome, Outcome::Failed(_)),
                "the test that printed it failed"
            );
        }

        #[test]
        fn a_passed_tests_output_is_discarded() {
            for format in [Format::Human, Format::Json] {
                let (_, printed, _) = run("parallel-output.nvs", spread(format), None);
                assert!(
                    !printed.contains("printed by the passing test"),
                    "a passing test's output is not printed: {printed}"
                );
                assert!(
                    printed.contains("printed by the failing test"),
                    "the failing test's output is printed in every format: {printed}"
                );
            }
        }
    }
}
