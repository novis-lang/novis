//! The one thing `rule:ide/one-grammar-one-tree` traded away, measured:
//! a full re-analysis of a ~1,000-line document stays under a named bound.
//!
//! There is no incremental reparse, so every keystroke that survives
//! cancellation runs `nvs_lsp::analyse_current` over the whole document —
//! lex, parse, `check_declarations`, resolve and type-check. This file is what
//! says that trade still pays, in the shape `benches/abi-probe/tests/perf_guards.rs`
//! uses: the minimum across several runs, a ceiling far above the measured
//! figure, and the headroom printed beside it so a later pass can tighten the
//! ceiling from logs rather than from one developer's box.
//!
//! Here rather than in a unit test because the subject is the whole front end
//! as an editor drives it, which is what `tests/coverage.rs` is also for: the
//! document is generated rather than kept under `tests/lsp/`, since its only
//! property is its size and a fixture nobody reads is a file that rots.
//!
//! ## Decision: the bound is a wall-clock ceiling per analysis, and there are two
//!
//! The bound is **25 ms in a `--release` build and 200 ms in a `debug` one**,
//! per full analysis of a ~1,000-line document. Two numbers rather than one
//! because this guard must be *both* honest and awake. The honest one is the
//! release figure: that is the build an editor runs, and the claim is about
//! what a person typing waits for. The debug one exists because
//! `cargo test -p nvs-lsp` — `verify.py`'s run, and the loop's acceptance
//! check — builds without optimisations, and a guard `#[ignore]`d there would
//! never run in this repository at all.
//!
//! Each is roughly 10x the figure measured where this landed — 2.7 ms release,
//! 22.3 ms debug, over the 997-line document [`document`] generates — which is
//! `perf_guards.rs`'s ratio and for its reason: this is an order-of-magnitude
//! guard against a change that makes the front end re-read, re-allocate or
//! re-resolve per node, not a detector of a 20% regression. A ceiling tightened
//! from one developer's box is a flaky gate rather than a stricter one, so the
//! headroom `under` prints beside every run is what a later pass would tighten
//! it from.
//!
//! ## If it goes red
//!
//! The answer is item-level caching over `nvs_lsp::analyse`'s parse — keyed on
//! the `SyntaxIndex` node an edit fell inside — and then a decision to revisit
//! the trade with a number in hand. It is never a larger number here.
//! `rule:ide/a-full-reanalysis-stays-under-a-bound` owns that ordering.

// A test binary is not the server. `rule:ide/stdout-belongs-to-the-protocol`
// gives *`nvs lsp`'s* stdout to the JSON-RPC stream, and `tests/stdout_policy.rs`
// is what holds that over every crate the server links; here the harness owns
// stdout, and a figure a guard measured is worth nothing if it is never printed.
#![allow(clippy::print_stdout)]

use std::time::{Duration, Instant};

use nvs_lsp::{Documents, analyse_current, uri_of};

/// The ceiling one full analysis must stay under.
///
/// Split by build profile for the reason the module doc gives; a `debug` build
/// of the front end is the one the acceptance check measures.
#[cfg(debug_assertions)]
const CEILING_MS: f64 = 200.0;
#[cfg(not(debug_assertions))]
const CEILING_MS: f64 = 25.0;

/// How many classes [`document`] writes to reach ~1,000 lines.
///
/// Each is 11 lines, over a 5-line header and a 2-line script body: 997.
const CLASSES: usize = 90;

/// How far a measurement sits from its ceiling, as `perf_guards.rs` prints it.
///
/// This changes no threshold and fails nothing. A figure printed alone reads
/// the same whether it had ten times the room it needed or none, and the ratio
/// is the number a later pass would tighten the ceiling from.
fn under(measured: f64, ceiling: f64) -> String {
    format!(
        " [ceiling {ceiling} ms, {:.1}x headroom]",
        ceiling / measured
    )
}

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

/// The fastest of `runs` analyses, in milliseconds.
///
/// The **minimum** rather than the mean, for `perf_guards.rs`'s reason: this
/// runs on shared machines, and a scheduler stealing the CPU mid-run moves the
/// mean while leaving the minimum where it was. A regression moves the minimum
/// too, so nothing is given up.
fn best_ms(runs: u32, mut op: impl FnMut()) -> f64 {
    op(); // Warm up: the first analysis pays for page faults nothing else does.
    let mut best = Duration::MAX;
    for _ in 0..runs {
        let start = Instant::now();
        op();
        best = best.min(start.elapsed());
    }
    best.as_secs_f64() * 1e3
}

/// One full re-analysis of a ~1,000-line document stays under [`CEILING_MS`].
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

    let ms = best_ms(5, || {
        let done = analyse_current(&documents, &uri, 1);
        assert!(done.is_some(), "the document stays open across the run");
    });

    println!(
        "a full re-analysis of {lines} lines: {ms:.1} ms{}",
        under(ms, CEILING_MS),
    );
    assert!(
        ms < CEILING_MS,
        "a full re-analysis of {lines} lines took {ms:.1} ms, over the {CEILING_MS} ms bound \
         `rule:ide/a-full-reanalysis-stays-under-a-bound` names; the answer is item-level \
         caching over the parse, never a larger number here",
    );
}
