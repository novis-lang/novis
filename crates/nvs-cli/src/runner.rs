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
//! # What is owed
//!
//! § 2's isolate-per-test and parallelism are M5, so this runs every test in
//! one process on one `Ctx` — which is also what a retry re-enters, class
//! storage surviving between attempts where an isolate would not; §§ 8-9's
//! `#[Fixture]` injection is not built, and a constructor that declares
//! parameters is reported as that test failing rather than pretended past
//! (`nvs_runtime::construct_and_call`). Class order is the roster's own sorted
//! order rather than § 20's declaration order: `ExprTypeTable::tests` is keyed
//! by class label and records no sequence, so declaration order across a
//! program's files is a fact only that table can grow. Within a class the
//! order *is* declaration order, which is what a reader of one file sees.

use std::process::ExitCode;
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
pub(crate) fn run(checked: &crate::Checked, format: Format) -> ExitCode {
    let program = nvs_ir::lower::lower_program(
        crate::SCRIPT,
        &checked.program_files(),
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    );
    let unit = match nvs_codegen::compile(&program) {
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

    let (mut passed, mut failed, mut skipped, mut flaky) = (0_usize, 0_usize, 0_usize, 0_usize);
    let mut exited = None;
    let mut cases = Vec::new();
    let started = Instant::now();
    for class in checked.exprs.test_classes() {
        if format == Format::Human {
            println!("  {class}");
        }
        for case in checked.exprs.tests(class).unwrap_or_default() {
            let began = Instant::now();
            let outcome = run_with_retries(&unit, &mut ctx, class, case);
            let elapsed = began.elapsed();
            match &outcome {
                Outcome::Passed => passed += 1,
                Outcome::Skipped(_) => skipped += 1,
                Outcome::Flaky { .. } => flaky += 1,
                Outcome::Failed(_) | Outcome::Exited(_) => failed += 1,
            }
            if format == Format::Human {
                report(&case.method, &outcome, elapsed);
            }
            if let Outcome::Exited(code) = outcome {
                exited = Some(code);
            }
            cases.push(Case {
                class: class.to_owned(),
                method: case.method.clone(),
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
    // Flushed before the summary: a test's own `echo` is buffered on the
    // context, and printing the counts over the top of it would report a run
    // whose output had not been written yet.
    if let Err(error) = ctx.flush_output() {
        eprintln!("error: could not flush output: {error}");
        return ExitCode::FAILURE;
    }
    let counts = Counts {
        passed,
        failed,
        skipped,
        flaky,
    };
    match format {
        Format::Human => println!(
            "\n  {failed} failed, {passed} passed, {skipped} skipped, {flaky} flaky in {:.0} ms",
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
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
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
/// reported against the next; what does *not* reset is class storage, an
/// isolate per test being § 2's and waiting on M5.
///
/// **A flaky test does not fail the run.** Its exit code is the passing one,
/// because retries exist precisely so that a suite can pass in spite of one —
/// what § 20 takes away is the *silence*, not the green build.
fn run_with_retries(
    unit: &nvs_codegen::Unit,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    case: &nvs_types::testing::TestCase,
) -> Outcome {
    let mut failures = match run_case(unit, ctx, class, case) {
        Outcome::Failed(failures) => failures,
        settled => return settled,
    };
    for retry in 0..retry_allowance(case) {
        match run_case(unit, ctx, class, case) {
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
fn run_case(
    unit: &nvs_codegen::Unit,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    case: &nvs_types::testing::TestCase,
) -> Outcome {
    if let Some(reason) = skip_reason(case) {
        return Outcome::Skipped(reason);
    }
    let Some(outcome) = unit.call_on_new_instance(ctx, class, &case.method) else {
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
