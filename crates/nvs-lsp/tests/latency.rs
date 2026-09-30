//! The one thing `rule:ide/one-grammar-one-tree` traded away, measured:
//! a full re-analysis of a ~1,000-line document stays under a named bound.
//!
//! There is no incremental reparse, so every keystroke that survives
//! cancellation runs `nvs_lsp::analyse_current` over the whole document —
//! lex, parse, `check_declarations`, resolve and type-check. This file is what
//! says that trade still pays, in the shape `benches/abi-probe/tests/perf_guards.rs`
//! uses for a timing: the minimum across several runs, judged against a soft
//! and a hard bound, with the headroom printed beside it so a later pass can
//! tighten the bounds from logs rather than from one developer's box.
//!
//! Here rather than in a unit test because the subject is the whole front end
//! as an editor drives it, which is what `tests/coverage.rs` is also for: the
//! document is generated rather than kept under `tests/lsp/`, since its only
//! property is its size and a fixture nobody reads is a file that rots.
//!
//! ## Decision: the bound is a wall-clock time per analysis, soft and hard, in two profiles
//!
//! A timing is comparable only with another taken on the same machine, and
//! load on that machine still moves it. So [`judge`] holds each figure against
//! the two bounds `rule:testing/perf-two-mechanisms` gives a timing:
//!
//! - The **soft bound** is 25 ms in a `--release` build and 200 ms in a `debug`
//!   one, per full analysis of a ~1,000-line document. A figure past it fails
//!   nothing: [`warn`] prints a warning that reaches the CI log and the step
//!   summary although the test passes.
//! - The **hard bound** is ten times the soft one. A figure past it is measured
//!   again, up to [`ATTEMPTS`] times, and only the last attempt fails the test.
//!
//! Two profiles rather than one because this guard must be *both* honest and
//! awake. The honest one is the release figure: that is the build an editor
//! runs, and the claim is about what a person typing waits for. The debug one
//! exists because `cargo test` — `nv verify`'s run, CI's, and the loop's
//! acceptance check — builds without optimisations, and a guard `#[ignore]`d
//! there would never run in this repository at all.
//!
//! The soft bound sits about an order of magnitude over what an idle machine
//! measures for the document [`document`] generates, which is `perf_guards.rs`'s
//! ratio and for its reason: this is an order-of-magnitude guard against a
//! change that makes the front end re-read, re-allocate or re-resolve per node,
//! not a detector of a 20% regression. That change of kind is far past ten
//! times the soft bound too, so the hard bound still fails on it. A bound
//! tightened from one developer's box is a flaky gate rather than a stricter
//! one, so the headroom [`judge`] prints beside every attempt is what a later
//! pass would tighten it from.
//!
//! Each attempt runs between two readings of a fixed scalar loop,
//! [`yardstick_ns`]. When the two differ by more than [`LOAD_TOLERANCE`], the
//! machine was busy during the attempt and its printed figure says `noisy`.
//! Every printed time also gives its ratio to the yardstick, written `yd`.
//!
//! ## If it warns, and if it goes red
//!
//! A warning says the figure left the soft bound on that machine in that run.
//! The comparison that answers it is a run of the commit before on the same
//! machine, because no other comparison of two timings says anything.
//!
//! Red is the hard bound on the last attempt. The answer is item-level caching
//! over `nvs_lsp::analyse`'s parse — keyed on the `SyntaxIndex` node an edit
//! fell inside — and then a decision to revisit the trade with a number in
//! hand. It is never a larger number here.
//! `rule:ide/a-full-reanalysis-stays-under-a-bound` owns that ordering.

// A test binary is not the server. `rule:ide/stdout-belongs-to-the-protocol`
// gives *`nvs lsp`'s* stdout to the JSON-RPC stream, and `tests/stdout_policy.rs`
// is what holds that over every crate the server links; here the harness owns
// stdout, and a figure a guard measured is worth nothing if it is never printed.
#![allow(clippy::print_stdout)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use nvs_lsp::{CheckScope, Documents, SymbolIndex, analyse_current, path_of, uri_of};

/// The soft bound on one full analysis: a figure past it warns and fails
/// nothing.
///
/// Split by build profile for the reason the module doc gives; a `debug` build
/// of the front end is the one the acceptance check measures.
#[cfg(debug_assertions)]
const SOFT_MS: f64 = 200.0;
#[cfg(not(debug_assertions))]
const SOFT_MS: f64 = 25.0;

/// The hard bound on one full analysis: a figure past it on the last of
/// [`ATTEMPTS`] attempts fails the test.
const HARD_MS: f64 = SOFT_MS * 10.0;

/// How many times [`judge`] measures before the hard bound fails the test.
///
/// A figure past the hard bound is measured again, and only the last attempt is
/// judged. A burst of load that lands on one attempt passes on the next one; a
/// change of kind is past the bound on every attempt, so it still fails.
const ATTEMPTS: u32 = 3;

/// How far the two [`yardstick_ns`] readings around one attempt may differ
/// before that attempt is labelled noisy, as a fraction of the faster reading.
const LOAD_TOLERANCE: f64 = 0.25;

/// How many classes [`document`] writes to reach ~1,000 lines.
///
/// Each is 11 lines, over a 5-line header and a 2-line script body: 997.
const CLASSES: usize = 90;

/// A ~1,000-line document that resolves and type-checks with nothing to report.
///
/// Deliberately ordinary rather than pathological: repeated declarations of the
/// shape a real file is mostly made of, each with a property read, a method
/// call target, a local and an arithmetic expression, so every phase does the
/// work it would do on a file someone wrote. A document built to be slow would
/// measure the worst case, and the claim being guarded is about the ordinary
/// one.
fn document(classes: usize) -> String {
    let mut text = String::from(
        "<?nvs\nnamespace App;\ninterface Greets {\n    public function greet(): string;\n}\n",
    );
    for i in 0..classes {
        text.push_str(&format!(
            "class User{i} implements Greets {{\n\
             \x20   public string $name = \"\";\n\
             \x20   public int $count = 0;\n\
             \x20   public function greet(): string {{\n\
             \x20       return \"hi, \" . $this->name;\n\
             \x20   }}\n\
             \x20   public function bump(int $by): int {{\n\
             \x20       var $next = $this->count + $by;\n\
             \x20       return $next;\n\
             \x20   }}\n\
             }}\n"
        ));
    }
    text.push_str("var $u = new User0();\necho $u->greet();\n");
    text
}

/// How many runs [`best_ms`] always takes, so the figure one attempt is judged
/// on is a minimum over several whatever the first run measured.
const RUNS: u32 = 5;

/// How many runs [`best_ms`] may take while its minimum sits between the two
/// bounds. A machine busy with other work lands one run under the soft bound
/// sooner or later, and a figure that stays past it through all of them is
/// what the warning reports.
const MAX_RUNS: u32 = 100;

/// The fastest of the analyses run in one attempt, in milliseconds.
///
/// The **minimum** rather than the mean, for `perf_guards.rs`'s reason: this
/// runs on shared machines, and a scheduler stealing the CPU mid-run moves the
/// mean while leaving the minimum where it was. A regression moves the minimum
/// too, so nothing is given up.
///
/// It takes [`RUNS`] runs. While the minimum then sits between [`SOFT_MS`] and
/// [`HARD_MS`] it keeps going, until the minimum is under the soft bound or
/// [`MAX_RUNS`] are spent, so a test binary sharing the machine with others
/// waits for a quiet moment before it warns. A minimum past the hard bound
/// after [`RUNS`] ends the attempt at once: measuring that again is [`judge`]'s
/// next attempt, between fresh yardstick readings.
fn best_ms(mut op: impl FnMut()) -> f64 {
    op(); // Warm up: the first analysis pays for page faults nothing else does.
    let mut best = Duration::MAX;
    for run in 0..MAX_RUNS {
        if run >= RUNS && !(SOFT_MS..HARD_MS).contains(&(best.as_secs_f64() * 1e3)) {
            break;
        }
        let start = Instant::now();
        op();
        best = best.min(start.elapsed());
    }
    best.as_secs_f64() * 1e3
}

/// A fixed scalar loop, timed: nanoseconds per run of the loop, the minimum
/// across several batches.
///
/// The loop does the same work on every run, so two readings differ only when
/// the machine does: another process on the core, or a clock that dropped under
/// heat. [`judge`] reads it before and after every attempt. A time divided by
/// it is what two runs on one machine can compare when a bare millisecond
/// count has moved with the load.
fn yardstick_ns() -> f64 {
    /// The dependent steps in one run of the loop.
    const STEPS: u32 = 256;
    /// The runs of the loop in one timed batch.
    const ITERS: u32 = 2_000;
    /// The timed batches the minimum is taken across.
    const BATCHES: u32 = 5;

    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut batch = || {
        let start = Instant::now();
        for _ in 0..ITERS * STEPS {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state = black_box(state);
        }
        start.elapsed()
    };
    batch(); // Warm up: the first batch pays for the branch predictor.
    let best = (0..BATCHES).fold(Duration::MAX, |best, _| best.min(batch()));
    best.as_secs_f64() * 1e9 / f64::from(ITERS)
}

/// Measures `op` and judges it against [`SOFT_MS`] and [`HARD_MS`], as the
/// module doc describes: the soft bound warns, the hard bound fails, and a hard
/// miss is measured again up to [`ATTEMPTS`] times before the last one fails.
///
/// One attempt is one [`best_ms`] between two [`yardstick_ns`] readings. `why`
/// is the sentence a hard failure adds after the figure: which rule names the
/// bound, and what the answer is.
fn judge(what: &str, why: &str, mut op: impl FnMut()) {
    let mut attempt = 1;
    loop {
        let before = yardstick_ns();
        let ms = best_ms(&mut op);
        let after = yardstick_ns();
        let drift = before.max(after) / before.min(after) - 1.0;
        let load = if drift > LOAD_TOLERANCE {
            "noisy"
        } else {
            "steady"
        };
        let figure = format!("{ms:.1} ms = {:.0} yd", ms * 1e6 / before);
        println!(
            "{what}: {figure} [soft {SOFT_MS} ms, hard {HARD_MS} ms, {:.1}x headroom to soft; \
             attempt {attempt} of {ATTEMPTS}, {load}: yardstick {before:.0} ns then {after:.0} ns]",
            SOFT_MS / ms,
        );
        if ms < HARD_MS {
            if ms >= SOFT_MS {
                warn(
                    what,
                    &format!(
                        "{figure} is past the soft bound of {SOFT_MS} ms on a {load} run. The \
                         hard bound of {HARD_MS} ms holds, so the test passes."
                    ),
                );
            }
            return;
        }
        assert!(
            attempt < ATTEMPTS,
            "{what}: {figure} on the last of {ATTEMPTS} attempts, a {load} run, is past the hard \
             bound of {HARD_MS} ms. {why}"
        );
        attempt += 1;
    }
}

/// Reports a soft-bound miss where a reader of the run sees it, although the
/// test passes.
///
/// libtest captures what a passing test prints through `println!`, and only
/// that: the process's standard output handle, written directly, reaches the
/// console and the CI log. On GitHub Actions the line is a `::warning`
/// workflow command, which lists it among the run's annotations, and a line is
/// appended to the step summary too. Elsewhere it is a plain line.
fn warn(what: &str, message: &str) {
    use std::io::Write as _;

    let line = if std::env::var_os("GITHUB_ACTIONS").is_some() {
        let title = format!("latency guard: {what}");
        format!(
            "::warning title={}::{}",
            workflow_escape(&title, true),
            workflow_escape(message, false)
        )
    } else {
        format!("warning: latency guard: {what}: {message}")
    };
    // Its own line: libtest may have printed a test's name without a newline.
    // A warning that cannot be written must not fail a guard that passed.
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "\n{line}").and_then(|()| out.flush());
    if let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary)
            .and_then(|mut file| writeln!(file, "- **latency guard: {what}**: {message}"));
    }
}

/// `text` escaped for a GitHub Actions workflow command: its message, or one
/// of its `key=value` properties, which also escape `:` and `,`.
fn workflow_escape(text: &str, property: bool) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '%' => escaped.push_str("%25"),
            '\r' => escaped.push_str("%0D"),
            '\n' => escaped.push_str("%0A"),
            ':' if property => escaped.push_str("%3A"),
            ',' if property => escaped.push_str("%2C"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// One full re-analysis of a ~1,000-line document stays under the bounds
/// [`judge`] applies.
#[test]
fn a_full_reanalysis_of_a_thousand_lines_stays_under_the_bound() {
    let text = document(CLASSES);
    let lines = text.lines().count();
    assert!(
        (900..1_100).contains(&lines),
        "the document under measurement is ~1,000 lines, not {lines}",
    );

    // No file on disk: `Documents::open` registers the buffer as an overlay and
    // the analysis reads it from there. The path is named only because a `file:`
    // URI is what `analyse` resolves a graph against.
    let uri =
        uri_of(&std::env::temp_dir().join("nvs-latency/main.nvs")).expect("a temp path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, text);

    // Measuring an analysis that stopped early would measure less than the
    // guard claims, and a syntax slip in the generator above is exactly how
    // that happens.
    let analysed = analyse_current(&documents, &uri, 1).expect("the open document analyses");
    assert!(
        analysed.diags.is_empty(),
        "the generated document is clean, so the measurement is of the whole walk: {} reported",
        analysed.diags.len(),
    );

    judge(
        &format!("a full re-analysis of {lines} lines"),
        "`rule:ide/a-full-reanalysis-stays-under-a-bound` names the bound; the answer is \
         item-level caching over the parse, never a larger number here.",
        || {
            let done = analyse_current(&documents, &uri, 1);
            assert!(done.is_some(), "the document stays open across the run");
        },
    );
}

/// A keystroke against a **warm** workspace index stays under the same bounds.
///
/// The bounds above are on what one analysis costs, and
/// `rule:ide/five-features-are-one-reference-index`'s index is a thing that
/// analyses files — so the question this answers is whether having one turns
/// every keystroke into a walk of the workspace. It does not:
/// `nvs_lsp::SymbolIndex::refresh` re-indexes the file that changed and the
/// files whose analysis read it, which for a document requiring nothing is one
/// re-analysis and the map write that follows it.
///
/// Measured through the same [`judge`], against the same document, so the two
/// figures printed by this file are directly comparable: the headroom between
/// them is what the index itself costs.
#[test]
fn a_warm_index_answers_within_the_reanalysis_bound() {
    let text = document(CLASSES);
    let uri = uri_of(&std::env::temp_dir().join("nvs-latency-index/main.nvs"))
        .expect("a temp path is UTF-8");
    let path = path_of(&uri).expect("a file URI names a file");
    let mut documents = Documents::new();
    documents.open(uri, 1, text);

    let mut index = SymbolIndex::build(&documents, CheckScope::Open, None);
    assert_eq!(index.len(), 1, "the open document is the whole tree here");
    assert!(
        index.declaration("App\\User0").is_some(),
        "the index is warm before it is measured, and holds what the document \
         declares",
    );

    judge(
        "a warm index after one change",
        "`rule:ide/a-full-reanalysis-stays-under-a-bound` names the bound; an index that costs \
         more than the analysis under it is one that rebuilds more than what changed.",
        || {
            let dropped = index.refresh(&documents, &path);
            assert_eq!(dropped.len(), 1, "one file changed and nothing reads it");
        },
    );
}
