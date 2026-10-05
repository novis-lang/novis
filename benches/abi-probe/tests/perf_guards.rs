//! Regression guards on the costs the ADRs quote: timings with two bounds each,
//! and counts with one.
//!
//! The criterion benches in `benches/` track these costs precisely but cannot
//! fail a build. These tests can, and a timing that fails a build has to hold on
//! every machine it runs on. A timing figure is comparable only with another
//! taken on the same machine, and even there load moves it: a shared CI runner,
//! or a developer's box building something else, reads the same code as several
//! times slower than an idle one does.
//!
//! So every timing guard has two bounds, and [`judge`] applies both:
//!
//! - The **soft bound** is where the figure stays on an ordinary machine.
//!   Missing it fails nothing: it prints a warning that reaches the CI log and
//!   the step summary although the test passes.
//! - The **hard bound** sits an order of magnitude past the soft one, or just
//!   short of the figure a change of kind reaches where ten times would hide it.
//!   It is not there to catch a 20% drift. It catches a dependency starting to
//!   allocate or to make a syscall per call, or the pooling allocator silently
//!   not being used. A figure past it is measured again, up to [`ATTEMPTS`]
//!   times, and only the last attempt fails the test.
//!
//! Each attempt runs between two readings of a fixed [`Yardstick`] loop. When
//! the two differ by more than [`LOAD_TOLERANCE`], the machine was busy during
//! the attempt and its printed figure says `noisy`. Every printed time also
//! gives its ratio to the yardstick, written `yd`.
//!
//! The counts are the durable signal: allocations, emitted calls, cache blocks,
//! an order of events. A count reads the same on any machine however loaded, so
//! it has a single bound and fails the moment it is missed.
//!
//! Every test is skipped unless built with optimisations, because the bounds
//! are release-mode figures. Run them with:
//!
//! ```text
//! cargo test --release -p nvs-abi-probe --test perf_guards
//! ```

#![expect(
    clippy::print_stdout,
    reason = "the measured figure is this guard's output. A timing guard that passed silently \
              would report only that a cost is inside its hard bound, which is the one thing a \
              reader already knows when it is green; the printed figures are what makes a CI log \
              answer how far inside, and `docs/perf/` quotes them"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use nvs_abi_probe::{Ctx, Helper, Probe, Value, call};

// The same source `benches/isolation.rs` compiles for its `isolate/` arm, so the
// guard below and the bench cannot drift apart; that file's own doc owns why it
// lives outside `src/`.
#[path = "../shared/isolate.rs"]
mod isolate;

// The same table `benches/routing.rs` walks, for the same reason.
#[path = "../shared/routes.rs"]
mod routes;

/// Holds this binary's guards apart, so each one measures a machine that is
/// otherwise idle.
///
/// libtest runs a test binary's tests on as many threads as the host has cores,
/// and every test below is a *timing*. A guard whose figure is a ratio across
/// worker cores then measures its neighbours as much as the code under it —
/// they are on the cores it is fanning out onto, so it answers that the fan-out
/// lost to a single core, which is a red build that says nothing about the
/// tree. The thresholds here are baselines taken with nothing else running in
/// the process, and this is what keeps the run they are compared against the
/// same kind of run they came from.
///
/// The lock is poison-tolerant on purpose: a guard that fails panics while
/// holding it, and a poisoned lock would turn one honest red into a spurious
/// one in every guard after it, hiding the one that matters.
fn serialised() -> MutexGuard<'static, ()> {
    static QUIET: Mutex<()> = Mutex::new(());
    QUIET.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Every guard in this file takes [`serialised`] before it measures anything.
///
/// The lock quietens a run only if all of them take it: one guard that skips
/// the line is one that runs on the cores the guard beside it is timing. That
/// is a convention a reader has to remember, so this counts it instead.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "it counts the guards, which only run in release"
)]
fn every_guard_in_this_binary_takes_the_lock() {
    let _quiet = serialised();

    let source = include_str!("perf_guards.rs");
    // Spelled in halves so the needle does not match its own literal here.
    let needle = concat!("let _quiet = ", "serialised();");
    let holding = source.matches(needle).count();
    let guards = source.matches("\n#[test]\n").count();
    assert_eq!(
        holding, guards,
        "{guards} guards in this file and {holding} of them take `serialised()`. One that does \
         not runs on the cores the guard beside it is timing, which is the failure this lock \
         exists to stop."
    );
}

/// How many times a timing guard measures before its hard bound fails it.
///
/// A figure past the hard bound is measured again, and only the last attempt is
/// judged. A burst of load that lands on one attempt passes on the next one; a
/// change of kind is past the bound on every attempt, so it still fails.
const ATTEMPTS: usize = 3;

/// How far the two [`Yardstick`] readings around one attempt may differ before
/// that attempt is labelled noisy, as a fraction of the faster reading.
const LOAD_TOLERANCE: f64 = 0.25;

/// A fixed scalar loop, timed before and after every attempt of every timing
/// guard, in nanoseconds per run of the loop.
///
/// The loop does the same work on every run, so its two readings differ only
/// when the machine does: another process on the core, or a clock that dropped
/// under heat. It is timed with [`ns_per_op`], like every figure here, so load
/// that moves a figure moves the yardstick the same way. A time divided by it
/// is what two runs on one machine can compare when a bare nanosecond count
/// has moved with the load.
#[derive(Clone, Copy)]
struct Yardstick(f64);

impl Yardstick {
    /// The dependent steps in one run of the loop.
    const STEPS: u32 = 256;

    fn measure() -> Self {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        Self(ns_per_op(2_000, 5, || {
            for _ in 0..Self::STEPS {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state = black_box(state);
            }
        }))
    }

    /// `ns` as a time, with its ratio to this reading.
    fn time(self, ns: f64) -> String {
        format!("{} = {} yd", duration(ns), sig(ns / self.0))
    }
}

/// `value` to three significant figures.
fn sig(value: f64) -> String {
    if !value.is_normal() {
        return format!("{value}");
    }
    let decimals = (2 - value.abs().log10().floor() as i32).clamp(0, 6) as usize;
    format!("{value:.decimals$}")
}

/// `ns` nanoseconds in the unit that keeps the figure short.
fn duration(ns: f64) -> String {
    match ns.abs() {
        magnitude if magnitude >= 1e6 => format!("{} ms", sig(ns / 1e6)),
        magnitude if magnitude >= 1e3 => format!("{} us", sig(ns / 1e3)),
        _ => format!("{} ns", sig(ns)),
    }
}

/// Which side of its bounds a figure stays on.
#[derive(Clone, Copy)]
enum Side {
    Under,
    Over,
}

/// What a figure measures: nanoseconds, or a ratio of two timings.
#[derive(Clone, Copy)]
enum Unit {
    Time,
    Ratio,
}

/// A timing guard's two bounds, as the module doc describes them.
#[derive(Clone, Copy)]
struct Bound {
    side: Side,
    unit: Unit,
    soft: f64,
    hard: f64,
}

impl Bound {
    /// A time in nanoseconds that stays under `soft`, and fails over `hard`.
    const fn time_under(soft: f64, hard: f64) -> Self {
        Self {
            side: Side::Under,
            unit: Unit::Time,
            soft,
            hard,
        }
    }

    /// A ratio that stays under `soft`, and fails over `hard`.
    const fn ratio_under(soft: f64, hard: f64) -> Self {
        Self {
            side: Side::Under,
            unit: Unit::Ratio,
            soft,
            hard,
        }
    }

    /// A ratio that stays over `soft`, and fails under `hard`.
    const fn ratio_over(soft: f64, hard: f64) -> Self {
        Self {
            side: Side::Over,
            unit: Unit::Ratio,
            soft,
            hard,
        }
    }

    /// Whether `value` is on the right side of `limit`. A NaN is on neither.
    fn inside(self, value: f64, limit: f64) -> bool {
        match self.side {
            Side::Under => value < limit,
            Side::Over => value > limit,
        }
    }

    fn show(self, value: f64) -> String {
        match self.unit {
            Unit::Time => duration(value),
            Unit::Ratio => format!("{}x", sig(value)),
        }
    }

    /// How many times over the figure could move before it reaches `soft`.
    fn headroom(self, value: f64) -> f64 {
        match self.side {
            Side::Under => self.soft / value,
            Side::Over => value / self.soft,
        }
    }
}

/// One attempt's figure, and the words printed after it.
struct Reading {
    value: f64,
    detail: String,
}

/// Measures a timing guard's figure and judges it against `bound`, as the
/// module doc describes: the soft bound warns, the hard bound fails, and a hard
/// miss is measured again up to [`ATTEMPTS`] times before the last one fails.
///
/// `measure` is one attempt. It gets the [`Yardstick`] reading taken just
/// before it, for the times it prints in its detail, and returns its figure.
/// `why` is the sentence a hard failure adds after the figure: what the change
/// of kind means, and which decision it reopens. Returns the judged figure.
fn judge(
    what: &str,
    bound: Bound,
    why: &str,
    mut measure: impl FnMut(Yardstick) -> Reading,
) -> f64 {
    judge_where_measurable(what, bound, why, |yard| Ok(measure(yard)))
        .expect("an attempt that always returns a figure is always judged")
}

/// [`judge`] for a figure this machine may be unable to give at all.
///
/// `measure` returns `Err` with the reason when it cannot give the figure, and
/// that attempt counts as a failed one. Returns `None` when the last attempt
/// was one of those, and the guard reports itself not measured.
fn judge_where_measurable(
    what: &str,
    bound: Bound,
    why: &str,
    mut measure: impl FnMut(Yardstick) -> Result<Reading, String>,
) -> Option<f64> {
    let soft = bound.show(bound.soft);
    let hard = bound.show(bound.hard);
    for attempt in 1..=ATTEMPTS {
        let before = Yardstick::measure();
        let reading = measure(before);
        let after = Yardstick::measure();
        let drift = before.0.max(after.0) / before.0.min(after.0) - 1.0;
        let load = if drift > LOAD_TOLERANCE {
            "noisy"
        } else {
            "steady"
        };
        let marker = format!(
            "attempt {attempt} of {ATTEMPTS}, {load}: yardstick {} then {}",
            duration(before.0),
            duration(after.0)
        );
        let reading = match reading {
            Ok(reading) => reading,
            Err(reason) => {
                println!("{what}: not measured, {reason} [{marker}]");
                continue;
            }
        };
        let figure = match bound.unit {
            Unit::Time => before.time(reading.value),
            Unit::Ratio => bound.show(reading.value),
        };
        println!(
            "{what}: {figure}{} [soft {soft}, hard {hard}, {:.1}x headroom to soft; {marker}]",
            reading.detail,
            bound.headroom(reading.value)
        );
        if bound.inside(reading.value, bound.hard) {
            if !bound.inside(reading.value, bound.soft) {
                warn(
                    what,
                    &format!(
                        "{figure} is past the soft bound of {soft} on a {load} run. The hard \
                         bound of {hard} holds, so the test passes."
                    ),
                );
            }
            return Some(reading.value);
        }
        assert!(
            attempt < ATTEMPTS,
            "{what}: {figure} on the last of {ATTEMPTS} attempts, a {load} run, is past the hard \
             bound of {hard}. {why}"
        );
    }
    None
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
        let title = format!("perf guard: {what}");
        format!(
            "::warning title={}::{}",
            workflow_escape(&title, true),
            workflow_escape(message, false)
        )
    } else {
        format!("warning: perf guard: {what}: {message}")
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
            .and_then(|mut file| writeln!(file, "- **perf guard: {what}**: {message}"));
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

/// Times `op` and returns nanoseconds per iteration.
///
/// Takes the **minimum** across several batches rather than the mean: CI runners
/// are shared and noisy, and the minimum is far more stable against a scheduler
/// stealing the CPU mid-batch. A regression shows up in the minimum too, so
/// nothing is lost.
fn ns_per_op(iters: u64, batches: u32, mut op: impl FnMut()) -> f64 {
    // Warm up: first-call trampolines, branch predictors, page faults.
    for _ in 0..iters.min(10_000) {
        op();
    }

    let mut best = Duration::MAX;
    for _ in 0..batches {
        let start = Instant::now();
        for _ in 0..iters {
            op();
        }
        best = best.min(start.elapsed());
    }
    best.as_secs_f64() * 1e9 / iters as f64
}

/// How many allocation requests this thread makes while `op` runs.
///
/// Read from the count the registered allocator keeps for every thread
/// (`nvs_runtime::budget::allocations`), so a guard adds nothing to the code it
/// counts. A request that is freed again before `op` returns is still counted.
fn allocations_during(op: impl FnOnce()) -> usize {
    let before = nvs_runtime::budget::allocations();
    op();
    nvs_runtime::budget::allocations() - before
}

/// Runs `op` with `depth` more frames of this thread's own stack above the
/// caller, each one a real call: the work after the inner call keeps the
/// optimizer from turning the recursion into a loop.
#[inline(never)]
fn under_frames(depth: u32, op: &mut dyn FnMut()) {
    if black_box(depth) == 0 {
        op();
    } else {
        under_frames(depth - 1, op);
        black_box(depth);
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_checked_return_frame_stays_cheap() {
    let _quiet = serialised();

    // The cost of one frame, taken as the slope between two depths so that the
    // harness's own call-out cancels.
    const PER_FRAME: Bound = Bound::time_under(15.0, 150.0);

    let mut probe = Probe::new();
    let shallow = probe.compile_chain(2, Helper::Double);
    let deep = probe.compile_chain(18, Helper::Double);
    let mut ctx = Ctx::new();
    let arg = Value::int(3);

    judge(
        "checked-return frame",
        PER_FRAME,
        "ADR 0002 justifies the checked-return convention on a frame costing about a \
         nanosecond; at this cost that argument needs revisiting.",
        |yard| {
            let t_shallow = ns_per_op(200_000, 5, || {
                black_box(call(shallow, &mut ctx, arg));
            });
            let t_deep = ns_per_op(200_000, 5, || {
                black_box(call(deep, &mut ctx, arg));
            });
            Reading {
                value: (t_deep - t_shallow) / 16.0,
                detail: format!(
                    " per frame (2 frames {}, 18 frames {})",
                    yard.time(t_shallow),
                    yard.time(t_deep)
                ),
            }
        },
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn throwing_costs_about_the_same_as_returning() {
    let _quiet = serialised();

    // ADR 0002 claims a throw costs roughly what a return does, which is what
    // lets PHP code that uses exceptions for control flow keep working. A large
    // ratio here means the error path has acquired real work — an allocation
    // for the message is on its own enough to put a throw at several times a
    // return. See `Ctx::pending`.
    //
    // What this prices is the *propagation*: the probe's helper leaves a
    // message on the context, the way a real helper's `Fault` does, and no site
    // is rendered anywhere. `rule:errors/throw-is-not-slower`'s other half — the
    // label a raise carrying a site renders once for its own frame — is
    // `a_raise_renders_one_frame_label_and_nothing_larger` below.
    //
    // The proof is counted, so it holds on any machine however loaded. The
    // helper's message is borrowed, and a status climbing the frames builds
    // nothing, so a throw makes no allocation at all. A return makes none
    // either, which is what leaves the two the same cost. The timing beside the
    // count is a sanity bound on the work that allocates nothing.
    const THROW_PER_RETURN: Bound = Bound::ratio_under(2.0, 20.0);
    const COUNTED: usize = 1_000;

    let mut probe = Probe::new();
    let chain = probe.compile_chain(8, Helper::Double);
    let mut ctx = Ctx::new();

    let returned = allocations_during(|| {
        for _ in 0..COUNTED {
            let (status, _) = call(chain, &mut ctx, Value::int(3));
            assert_eq!(status, nvs_abi_probe::OK, "3 is an argument that returns");
        }
    });
    // 42 makes the innermost helper throw, which then climbs all 8 frames.
    let thrown = allocations_during(|| {
        for _ in 0..COUNTED {
            let (status, _) = call(chain, &mut ctx, Value::int(42));
            assert_eq!(
                status,
                nvs_abi_probe::THROWN,
                "42 is the argument that throws"
            );
            ctx.pending = None;
        }
    });
    println!(
        "{COUNTED} throws through 8 frames: {thrown} allocations, and {returned} in {COUNTED} \
         returns [both exactly 0]"
    );
    assert_eq!(
        (thrown, returned),
        (0, 0),
        "{COUNTED} throws through 8 frames made {thrown} allocations and {COUNTED} returns made \
         {returned}. A throw that costs about what a return does allocates nothing, which is \
         ADR 0002's claim and the reason `Ctx::pending` borrows its message."
    );

    judge(
        "throw/return ratio at depth 8",
        THROW_PER_RETURN,
        "The error path has acquired real work, which ADR 0002's claim that a throw costs \
         about what a return does rules out.",
        |yard| {
            let ok = ns_per_op(200_000, 5, || {
                black_box(call(chain, &mut ctx, Value::int(3)));
            });
            let thrown = ns_per_op(200_000, 5, || {
                black_box(call(chain, &mut ctx, Value::int(42)));
                ctx.pending = None;
            });
            Reading {
                value: thrown / ok,
                detail: format!(
                    " ({} thrown vs {} returned)",
                    yard.time(thrown),
                    yard.time(ok)
                ),
            }
        },
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_raise_renders_one_frame_label_and_nothing_larger() {
    let _quiet = serialised();

    // `rule:errors/throw-is-not-slower` now says a raise allocates: it renders
    // the frame it happened in, from the site the `throw` was compiled with, so
    // a `catch` beside the `throw` has a backtrace at all. What must stay true
    // is the *shape* of that cost — one decode and one label, paid once. The
    // answer this guard rules out is the one the decision behind
    // `nvs_runtime`'s gap 6 priced and rejected: walking a stack, or asking the
    // OS for a backtrace.
    //
    // The proof is counted, not timed, so it holds on any machine however
    // loaded. The allocator counts every allocation a raise makes and every
    // byte it asks for, and the rendering is the difference between raising
    // with a site and raising with the zero word, so the object both arms
    // allocate cancels. Two counts pin the shape. The rendering's allocations
    // and bytes stay under a small fixed bound, which is room for one decode
    // and one label and not for a list of frames. And both are identical when
    // the raise happens `DEEP` frames further down this thread's stack, which
    // a walk of that stack, or an OS backtrace of it, cannot be.
    //
    // The timing beside it is a sanity bound, not the proof. The rendering is
    // made of allocations, so its time follows the platform's allocation path:
    // on macOS every allocation's counter update is a call through the
    // thread-local descriptor, and the same allocations cost more there. The
    // bound is therefore relative to a raise without a site, which pays for
    // allocations on the same path: a label costs less than ten raises, and a
    // hundred is a label doing work that allocates nothing.
    const DEEP: u32 = 64;
    const MAX_ALLOCATIONS: usize = 16;
    const MAX_BYTES: usize = 1024;
    const LABEL_PER_RAISE: Bound = Bound::ratio_under(10.0, 100.0);

    let mut table = nvs_runtime::ClassTable::new();
    let slots = ["message", "previous", "backtrace", "location"];
    let class = table.define("LogicError", &slots, &[]);
    let class = table.desc(class);
    let blob = nvs_runtime::source::encode(&nvs_render::Source {
        file: "app/Http/Handler.nvs".to_owned(),
        line: 118,
        member: Some("Handler::respond".to_owned()),
    });
    let mut ctx = nvs_runtime::Ctx::buffered();

    #[expect(
        unsafe_code,
        reason = "the raise primitive is the thing being measured, and the \
                  class table, the exception's reference and the blob are all \
                  this frame's own"
    )]
    // SAFETY: the table outlives every instance made from it, each raise is
    // handed one reference it takes over, and the blob is `encode`'s bytes.
    let mut raise = |site: *const u8| unsafe {
        let thrown = nvs_runtime::Thrown::new(class, "boom");
        nvs_runtime::nvs_raise(&raw mut ctx, thrown.into_raw(), site);
        drop(black_box(nvs_runtime::Ctx::take_thrown(&mut ctx)));
    };
    let mut counted = |site: *const u8, depth: u32| -> (usize, usize) {
        let calls = nvs_runtime::budget::allocations();
        let bytes = nvs_runtime::budget::allocated_bytes();
        under_frames(depth, &mut || raise(site));
        (
            nvs_runtime::budget::allocations() - calls,
            nvs_runtime::budget::allocated_bytes() - bytes,
        )
    };
    // A first raise of each kind, so nothing a first call sets up is counted.
    counted(std::ptr::null(), 0);
    counted(blob.as_ptr(), 0);
    let mut rendered = |depth: u32| {
        let (bare_calls, bare_bytes) = counted(std::ptr::null(), depth);
        let (sited_calls, sited_bytes) = counted(blob.as_ptr(), depth);
        (
            sited_calls.saturating_sub(bare_calls),
            sited_bytes.saturating_sub(bare_bytes),
        )
    };
    let (calls, bytes) = rendered(0);
    let (deep_calls, deep_bytes) = rendered(DEEP);
    println!(
        "a raise's own frame label: {calls} allocations and {bytes} bytes, \
         {deep_calls} and {deep_bytes} {DEEP} frames deeper \
         [ceiling {MAX_ALLOCATIONS} allocations and {MAX_BYTES} bytes]"
    );
    assert!(
        calls <= MAX_ALLOCATIONS && bytes <= MAX_BYTES,
        "rendering a raise's own frame now makes {calls} allocations of {bytes} bytes, over the \
         guard's {MAX_ALLOCATIONS} and {MAX_BYTES}. A throw is meant to pay for one decode and \
         one label, not for a list of frames."
    );
    assert!(
        (deep_calls, deep_bytes) == (calls, bytes),
        "rendering a raise's own frame made {calls} allocations of {bytes} bytes, and \
         {deep_calls} of {deep_bytes} bytes {DEEP} frames further down the stack. A cost that \
         grows with the stack is a walk of it, which a throw is never meant to pay for."
    );

    judge(
        "a raise's own frame label, against a raise without a site",
        LABEL_PER_RAISE,
        "The allocation counts above are in bounds, so the time is going into work that \
         allocates nothing.",
        |yard| {
            let bare = ns_per_op(100_000, 5, || raise(std::ptr::null()));
            let sited = ns_per_op(100_000, 5, || raise(blob.as_ptr()));
            let rendering = sited - bare;
            Reading {
                value: rendering / bare,
                detail: format!(
                    " ({} rendering: {} sited vs {} bare)",
                    yard.time(rendering),
                    yard.time(sited),
                    yard.time(bare)
                ),
            }
        },
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_coroutine_round_trip_stays_cheap() {
    let _quiet = serialised();

    // A suspend and resume through JIT frames, which ADR 0003's preamble and
    // docs/adr/README.md price as the cost of "no async colouring".
    //
    // The proof is counted, so it holds on any machine however loaded. A
    // suspend and the resume after it switch between two stacks that both
    // exist already, so a round trip inside a running coroutine makes no
    // allocation. The count is taken inside the coroutine and covers both
    // sides of every switch, because the side that resumes runs on this thread
    // too. The timing beside it is a sanity bound on the switch itself.
    const PER_TRIP: Bound = Bound::time_under(400.0, 4_000.0);
    const COUNTED: u64 = 1_000;

    let mut probe = Probe::new();
    let chain = probe.compile_chain(2, Helper::Suspend);

    let counted = nvs_abi_probe::in_coroutine(Ctx::new(), move |ctx| {
        allocations_during(|| {
            for _ in 0..COUNTED {
                black_box(call(chain, ctx, Value::int(1)));
            }
        })
    });
    assert_eq!(
        counted.suspends as u64, COUNTED,
        "every counted call must have suspended"
    );
    println!(
        "{COUNTED} coroutine round trips through 2 JIT frames: {} allocations [exactly 0]",
        counted.value
    );
    assert_eq!(
        counted.value, 0,
        "{COUNTED} suspend and resume round trips made {} allocations. A round trip switches \
         between two stacks that already exist and allocates nothing; one that allocates is no \
         longer the cheap switch ADR 0003's \"no async colouring\" rests on.",
        counted.value
    );

    // One coroutine per batch, suspending `iters` times inside it, so creation
    // cost is amortised to nothing and the figure is the round trip itself.
    let batch = |iters: u64| -> Duration {
        let start = Instant::now();
        let run = nvs_abi_probe::in_coroutine(Ctx::new(), move |ctx| {
            for _ in 0..iters {
                black_box(call(chain, ctx, Value::int(1)));
            }
        });
        let elapsed = start.elapsed();
        assert_eq!(run.suspends as u64, iters, "every call must have suspended");
        elapsed
    };

    judge(
        "coroutine suspend/resume through 2 JIT frames",
        PER_TRIP,
        "A round trip in this class is no longer the cheap switch ADR 0003's \"no async \
         colouring\" rests on.",
        |_yard| {
            batch(10_000); // warm up
            let iters = 200_000u64;
            let mut best = Duration::MAX;
            for _ in 0..5 {
                best = best.min(batch(iters));
            }
            Reading {
                value: best.as_secs_f64() * 1e9 / iters as f64,
                detail: String::new(),
            }
        },
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn an_all_bits_off_debug_probe_stays_in_the_safepoint_cost_class() {
    let _quiet = serialised();

    // ADR 0018 § 1's whole argument for a runtime-checked flag over a second
    // compiled tier rests on this: the check is emitted at every statement
    // boundary of every compiled unit, whether or not any request ever sets a
    // bit, so its all-bits-off cost has to be the same cost class already
    // accepted for the safepoint poll — one cached load and one
    // predicted-not-taken branch. If it is not, that ADR's *Revisiting*
    // section says the fix is coarsening the probe site to one per basic
    // block, not loosening this number.
    //
    // The soft bound sits well under `a_checked_return_frame_stays_cheap`'s own:
    // a probe site does strictly less than a frame (no call, no 16-byte result
    // copy, no status check), so a site that costs what a frame may cost has
    // acquired real work.
    const PER_SITE: Bound = Bound::time_under(5.0, 50.0);
    const DEPTH: usize = 8;
    const STMTS_PER_FRAME: usize = 16;

    let mut probe = Probe::new();
    // Same depth, same statement count, same stores — the two chains differ
    // by exactly the flag checks. See `compile_probe_chain`'s own docs for
    // why the statements store something rather than being empty.
    let plain = probe.compile_probe_chain(DEPTH, Helper::Double, STMTS_PER_FRAME, false);
    let probed = probe.compile_probe_chain(DEPTH, Helper::Double, STMTS_PER_FRAME, true);
    let mut ctx = Ctx::new();
    let arg = Value::int(3);
    let sites = DEPTH * STMTS_PER_FRAME;

    judge(
        "all-bits-off debug probe",
        PER_SITE,
        "That check is emitted at every statement boundary of every compiled unit; if this is \
         real, ADR 0018 § Revisiting says to coarsen the probe site, not to raise this bound.",
        |yard| {
            let t_plain = ns_per_op(200_000, 5, || {
                black_box(call(plain, &mut ctx, arg));
            });
            let t_probed = ns_per_op(200_000, 5, || {
                black_box(call(probed, &mut ctx, arg));
            });
            Reading {
                value: (t_probed - t_plain) / sites as f64,
                detail: format!(
                    " per site ({sites} sites, {} probed vs {} plain)",
                    yard.time(t_probed),
                    yard.time(t_plain)
                ),
            }
        },
    );

    // Nothing set a bit, so no site may have reached its slow path.
    assert_eq!(
        ctx.probe_hits, 0,
        "a probe fired with every debug flag off, which would make the \
         measurement meaningless"
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn an_amortised_deadline_poll_costs_less_than_the_check_it_rides_beside() {
    let _quiet = serialised();

    // ADR 0106 § 5 owns exactly one number and this is it: a helper whose
    // runtime scales with its input polls the deadline through
    // `nvs_runtime::bounded_loop`, the poll is amortised over
    // `DEADLINE_POLL_BATCH` iterations, and what makes that batch the right
    // size is that the amortised cost stays **under the stack check's own
    // per-call cost**.
    //
    // Both sides are measured here rather than one of them being a constant.
    // A ceiling copied from one developer's box is what makes a guard like this
    // flaky, and the two costs move together anyway: they are the same shape.
    // The right-hand side is the cost class the test above establishes — one
    // load from `Ctx`'s hot line and one branch predicted not taken — which is
    // exactly what the stack check compiles to, at the same emit site the
    // safepoint poll uses (`nvs-runtime`'s `ctx` module doc). Measuring the
    // emitted check itself would need the probe JIT to grow a stack-check site;
    // measuring its cost class needs nothing new and answers the same question.
    //
    // The figure is the poll's cost as a multiple of the check's. The soft
    // bound is the ADR's own, a poll cheaper than the check; the hard one is a
    // poll ten checks dear, which is a batch that stopped amortising. Both
    // sides are differences of two sub-nanosecond timings, so a burst of load
    // on one batch moves a side by a whole nanosecond, and that is what the
    // re-measuring in `judge` absorbs.
    //
    // The count beside the timing is the part no machine moves: where the
    // polls are. The body expires the deadline at each position in turn, and
    // the loop has to stop at the next multiple of `DEADLINE_POLL_BATCH`. A
    // loop that stops sooner reads the flag more than once in a batch, which is
    // a poll that is not amortised. A loop that stops later has left a batch
    // with no poll in it.
    const POLL_PER_CHECK: Bound = Bound::ratio_under(1.0, 10.0);
    const DEPTH: usize = 8;
    const STMTS_PER_FRAME: usize = 16;
    const ITEMS: usize = nvs_runtime::DEADLINE_POLL_BATCH * 64;
    const BATCH: usize = nvs_runtime::DEADLINE_POLL_BATCH;
    const COUNTED_BATCHES: usize = 3;

    let mut stops = std::collections::BTreeSet::new();
    for expires_at in 1..COUNTED_BATCHES * BATCH {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        let mut ran = 0usize;
        let stopped = nvs_runtime::bounded_loop(
            &mut ctx,
            "Core\\Probe::sweep",
            0..(COUNTED_BATCHES + 1) * BATCH,
            |ctx, _item| {
                ran += 1;
                if ran == expires_at {
                    ctx.expire_deadline();
                }
                Ok(())
            },
        );
        assert!(
            stopped.is_err(),
            "a deadline expired at iteration {expires_at} never stopped the loop"
        );
        // The poll at a boundary runs before that iteration's body.
        let boundary = (expires_at / BATCH + 1) * BATCH;
        assert_eq!(
            ran,
            boundary - 1,
            "a deadline expired at iteration {expires_at} stopped the loop after {ran} \
             iterations. The only poll after it is at iteration {boundary}, because \
             `bounded_loop` reads the flag once in every {BATCH} iterations; ADR 0106 § 5 rests \
             on that batch."
        );
        stops.insert(ran);
    }
    // Every stop above was at a boundary, so the distinct stops are the polls.
    println!(
        "deadline polls in {COUNTED_BATCHES} batches of {BATCH} iterations: {} [exactly \
         {COUNTED_BATCHES}, one at each batch boundary]",
        stops.len()
    );

    let mut probe = Probe::new();
    let plain_chain = probe.compile_probe_chain(DEPTH, Helper::Double, STMTS_PER_FRAME, false);
    let checked_chain = probe.compile_probe_chain(DEPTH, Helper::Double, STMTS_PER_FRAME, true);
    let mut probe_ctx = Ctx::new();
    let arg = Value::int(3);

    // The two loops differ by exactly what the combinator adds: a register
    // countdown and a branch predicted not taken, plus one flag load per batch.
    // `black_box` on the accumulator keeps both scalar, so neither side wins by
    // being vectorized rather than by being cheaper.
    let mut rt = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
    let mut sum = 0usize;

    judge(
        "amortised deadline poll, against a hot-line check",
        POLL_PER_CHECK,
        "ADR 0106 § 5 bounds the poll by the check, and the fix is the batch in \
         `nvs_runtime::DEADLINE_POLL_BATCH` or the shape of `bounded_loop`'s countdown — not \
         this comparison, which is what the ADR says.",
        |yard| {
            let t_plain_chain = ns_per_op(200_000, 5, || {
                black_box(call(plain_chain, &mut probe_ctx, arg));
            });
            let t_checked_chain = ns_per_op(200_000, 5, || {
                black_box(call(checked_chain, &mut probe_ctx, arg));
            });
            let per_check = (t_checked_chain - t_plain_chain) / (DEPTH * STMTS_PER_FRAME) as f64;

            let t_plain_loop = ns_per_op(2_000, 5, || {
                for item in 0..ITEMS {
                    sum = black_box(sum.wrapping_add(item));
                }
            });
            let t_polled_loop = ns_per_op(2_000, 5, || {
                nvs_runtime::bounded_loop(&mut rt, "Core\\Probe::sweep", 0..ITEMS, |_ctx, item| {
                    sum = black_box(sum.wrapping_add(item));
                    Ok(())
                })
                .expect("nothing set a deadline on this context");
            });
            let per_poll = (t_polled_loop - t_plain_loop) / ITEMS as f64;
            // A check that measured as free leaves no ratio to take, and that
            // attempt is a miss the next one measures again.
            let value = if per_check > 0.0 {
                per_poll / per_check
            } else {
                f64::INFINITY
            };
            Reading {
                value,
                detail: format!(
                    " ({} per iteration over {ITEMS} iterations in batches of {}, polled loop {} \
                     vs plain {}; {} per check)",
                    yard.time(per_poll),
                    nvs_runtime::DEADLINE_POLL_BATCH,
                    yard.time(t_polled_loop),
                    yard.time(t_plain_loop),
                    yard.time(per_check)
                ),
            }
        },
    );
    black_box(sum);
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_probe_that_is_switched_on_mid_flight_actually_fires() {
    let _quiet = serialised();

    // The other half of the same claim, and the reason the cost above is
    // worth paying: setting the word on a context is the entire mechanism, so
    // already-compiled code has to start reporting with no recompilation.
    const DEPTH: usize = 2;
    const STMTS_PER_FRAME: usize = 4;

    let mut probe = Probe::new();
    let probed = probe.compile_probe_chain(DEPTH, Helper::Double, STMTS_PER_FRAME, true);
    let mut ctx = Ctx::new();

    call(probed, &mut ctx, Value::int(3));
    assert_eq!(ctx.probe_hits, 0);

    ctx.debug_flags = 1;
    call(probed, &mut ctx, Value::int(3));
    assert_eq!(ctx.probe_hits, (DEPTH * STMTS_PER_FRAME) as u64);
}

// ---------------------------------------------------------------------------
// Extension sandbox
// ---------------------------------------------------------------------------

#[cfg(feature = "wasm-probe")]
mod wasm_guards {
    use super::{Bound, Instant, Reading, black_box, judge, ns_per_op};
    use nvs_abi_probe::wasm::WasmProbe;

    const NO_DEADLINE: u64 = u64::MAX;

    #[test]
    #[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
    fn a_host_to_guest_call_stays_cheap() {
        // ADR 0003's whole tier argument rests on this call costing tens of
        // nanoseconds rather than a microsecond, which is where the hard bound
        // sits.
        const PER_CALL: Bound = Bound::time_under(200.0, 2_000.0);

        let probe = WasmProbe::new().expect("wasm probe");
        let (mut store, instance) = probe.instantiate(0, NO_DEADLINE).expect("instantiate");
        let add = instance
            .get_typed_func::<(i64, i64), i64>(&mut store, "add")
            .expect("add export");

        judge(
            "wasm host->guest call",
            PER_CALL,
            "ADR 0003's tier argument assumes a call in the tens of nanoseconds.",
            |_yard| Reading {
                value: ns_per_op(200_000, 5, || {
                    black_box(add.call(&mut store, (1, 2)).expect("call"));
                }),
                detail: String::new(),
            },
        );
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
    fn per_request_instantiation_stays_affordable() {
        // What makes a fresh instance per request — and therefore per-request
        // isolation for extensions — affordable (ADR 0003). A figure past the
        // hard bound most likely means the pooling allocator is no longer in
        // use.
        const PER_INSTANCE: Bound = Bound::time_under(100_000.0, 1_000_000.0);

        let probe = WasmProbe::pooled(1000).expect("pooled probe");

        let run = || {
            let (mut store, instance) = probe.instantiate(0, NO_DEADLINE).expect("instantiate");
            let add = instance
                .get_typed_func::<(i64, i64), i64>(&mut store, "add")
                .expect("add export");
            black_box(add.call(&mut store, (1, 2)).expect("call"));
        };

        judge(
            "wasm pooled instantiate + 1 call",
            PER_INSTANCE,
            "Check that the pooling allocator is still in use.",
            |_yard| {
                for _ in 0..1_000 {
                    run();
                }
                let iters = 20_000u64;
                let mut best = f64::MAX;
                for _ in 0..3 {
                    let start = Instant::now();
                    for _ in 0..iters {
                        run();
                    }
                    best = best.min(start.elapsed().as_secs_f64() * 1e9 / iters as f64);
                }
                Reading {
                    value: best,
                    detail: String::new(),
                }
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Isolation boundaries
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn an_os_process_costs_orders_of_magnitude_more_than_a_task() {
    let _quiet = serialised();

    // ADR 0006 gives Novis an in-process script isolate because PHP's only way to
    // run a script under its own heap, globals and limits is another process.
    // The whole argument is this ratio, so it is measured rather than asserted.
    //
    // Both sides are floors: the process side is the cheapest do-nothing image
    // the platform can start (a real child would also boot an interpreter, and
    // the PHP binary on this machine takes several times the floor just to
    // start and exit), and the task side is a bare coroutine (an isolate also
    // builds an arena and a set of globals). `CreateProcess` is dearer than
    // `fork`+`exec`, so the ratio differs by platform. The soft bound of 20x
    // holds everywhere; the hard bound of 2x is a task that costs what a process
    // does, the change of kind — a task acquiring a syscall, or committing its
    // stack eagerly.
    const PROCESS_PER_TASK: Bound = Bound::ratio_over(20.0, 2.0);

    judge(
        "isolation boundary, an os process against a task",
        PROCESS_PER_TASK,
        "ADR 0006 justifies in-process script isolates on that gap; if the gap has really \
         closed, the ADR needs revisiting.",
        |yard| {
            let task_ns = ns_per_op(50_000, 5, || {
                let run = nvs_abi_probe::in_coroutine(Ctx::new(), |_ctx| black_box(7i64));
                black_box(run.value);
            });

            // A spawn is milliseconds, so batches of one, and the minimum across them.
            let mut best_process = Duration::MAX;
            nvs_abi_probe::process::spawn_noop(); // warm the image cache
            for _ in 0..25 {
                let start = Instant::now();
                nvs_abi_probe::process::spawn_noop();
                best_process = best_process.min(start.elapsed());
            }
            let process_ns = best_process.as_secs_f64() * 1e9;
            Reading {
                value: process_ns / task_ns,
                detail: format!(
                    " (os process {} vs task {})",
                    yard.time(process_ns),
                    yard.time(task_ns)
                ),
            }
        },
    );
}

/// Runs `round_trips` spawn-to-result round trips one after another inside one
/// task, and answers with how many tasks ran to their end and how many stacks
/// the scheduler has pooled once the run queue is empty.
///
/// The round trip is [`isolate::spawn_to_result`], the one
/// [`isolate::spawn_to_result_batch`] times. The scheduler is built here
/// because the pool is read from it after the task has ended.
fn stacks_pooled_after(round_trips: usize) -> (usize, usize) {
    let mut sched = nvs_host::Scheduler::new();
    let parent = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
    sched.spawn(parent, nvs_runtime::TaskRoot::Request, move |ctx| {
        for _ in 0..round_trips {
            black_box(isolate::spawn_to_result(ctx));
        }
    });
    let report = sched.run();
    (report.finished, sched.pooled_stacks())
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_spawn_to_result_round_trip_stays_in_the_microsecond_class() {
    let _quiet = serialised();

    // The figure M5's acceptance asks for, and the one the test above only
    // approximates: that one prices a bare coroutine, this one prices ADR 0006's
    // boundary itself — the argument's graph copy in, the child's own ownership
    // root and context, the child task, the answer's copy out, the release.
    //
    // The count beside the timing is the part no machine moves: how many stacks
    // the scheduler has pooled once a task of round trips has ended. A child's
    // task ends before its parent is resumed, and the scheduler takes its stack
    // back into the pool the next spawn takes one from. Round trips made one
    // after another therefore leave two stacks pooled however many there were:
    // the parent's own, and the one every child ran on in turn. A spawn that
    // reserves a stack of its own leaves one for every child, a child that runs
    // inline on its parent's stack leaves one, and a stack handed back to the OS
    // leaves none.
    //
    // The soft bound is M5's own number rather than a multiple of a baseline:
    // at ten microseconds "single-digit microseconds" stops being true, and the
    // run says so. The hard bound, a hundred microseconds, is a boundary that
    // has acquired a syscall or a child doing something a child should not.
    const PER_ROUND_TRIP: Bound = Bound::time_under(10_000.0, 100_000.0);
    const ITERS: u64 = 20_000;
    const COUNTED: usize = 1_000;
    const POOLED: usize = 2;

    let (finished, pooled) = stacks_pooled_after(COUNTED);
    assert_eq!(
        finished,
        COUNTED + 1,
        "every counted round trip must have run a child task to its end, beside their parent"
    );
    println!(
        "{COUNTED} spawn-to-result round trips: {pooled} stacks pooled afterwards [exactly \
         {POOLED}, the parent's and the one every child reused]"
    );
    assert_eq!(
        pooled, POOLED,
        "{COUNTED} spawn-to-result round trips made one after another left {pooled} stacks in \
         the scheduler's pool. A child's stack goes back to the pool when its task ends and the \
         next spawn takes it from there (`nvs_host::stack`), so the pool ends with the parent's \
         stack and one more; any other count is a spawn that reserved a stack instead of reusing \
         one, or a child that did not run as a task."
    );

    judge(
        "isolate spawn to result",
        PER_ROUND_TRIP,
        "ADR 0006 replaces a child process with this boundary and M5's acceptance puts it in \
         the single-digit microseconds; at this figure the child is doing something a child \
         should not, or the boundary has acquired a syscall.",
        |_yard| {
            // Each batch builds its own scheduler and task and warms inside
            // them; the shared module owns why the loop cannot simply live here.
            let mut best = f64::MAX;
            for _ in 0..5 {
                let took = isolate::spawn_to_result_batch(ITERS);
                best = best.min(took.as_secs_f64() * 1e9 / ITERS as f64);
            }
            Reading {
                value: best,
                detail: String::new(),
            }
        },
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn concurrent_spawns_leave_the_scheduler_serving_other_tasks() {
    let _quiet = serialised();

    // M8's list for `rule:core-classes/process-is-argv-only`: children in
    // flight do not stop the core they were started from serving anything else.
    // What is driven is the handoff the member performs — `blocking::run`
    // around a real child — and not the member, which this crate cannot reach;
    // `nvs-stdlib`'s `a_spawned_childs_reads_suspend_the_coroutine_and_free_the_core`
    // owns the correctness half over `Core\Process::spawn` itself, and the
    // property measured here is the scheduler's either way.
    //
    // The bound is an order of events, not a ratio of times, so it holds on any
    // machine however loaded. Each child's job starts its process and then
    // waits, with the process still unreaped, until the neighbour has run. A
    // core that hands itself back reaches the neighbour and opens that gate; a
    // core held by the wait never reaches it, and the gate's deadline is what
    // ends the run as a failure. A ratio of elapsed times could not tell the
    // two apart on a shared host: a child started and reaped in half a
    // millisecond leaves no room between them for a scheduler to show it was
    // not blocked.
    const CHILDREN: usize = 4;
    // Only the failing run ever waits this long.
    const GATE_DEADLINE: Duration = Duration::from_secs(20);

    let _installed =
        nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
    let mut sched = nvs_host::Scheduler::new();

    let gate: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));
    let started = Instant::now();
    let deadline = started + GATE_DEADLINE;
    let behind_the_neighbour: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    for _ in 0..CHILDREN {
        let ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        let gate = Arc::clone(&gate);
        let behind = Rc::clone(&behind_the_neighbour);
        sched.spawn(ctx, nvs_runtime::TaskRoot::Worker, move |_ctx| {
            let (status, neighbour_ran) = nvs_host::blocking::run(move || {
                let mut child = nvs_abi_probe::process::noop_command()
                    .spawn()
                    .expect("the host must be able to start a do-nothing process");
                let (open, opened) = &*gate;
                let open = open.lock().unwrap_or_else(PoisonError::into_inner);
                let wait = deadline.saturating_duration_since(Instant::now());
                let (open, _) = opened
                    .wait_timeout_while(open, wait, |open| !*open)
                    .unwrap_or_else(PoisonError::into_inner);
                let neighbour_ran = *open;
                drop(open);
                let status = child.wait().expect("a started child is one to be reaped");
                (status, neighbour_ran)
            });
            assert!(status.success(), "a do-nothing process must exit cleanly");
            if neighbour_ran {
                behind.set(behind.get() + 1);
            }
        });
    }
    let served: Rc<Cell<Option<Duration>>> = Rc::new(Cell::new(None));
    let neighbour = Rc::clone(&served);
    let opens = Arc::clone(&gate);
    sched.spawn(
        nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink),
        nvs_runtime::TaskRoot::Worker,
        move |_ctx| {
            neighbour.set(Some(started.elapsed()));
            let (open, opened) = &*opens;
            *open.lock().unwrap_or_else(PoisonError::into_inner) = true;
            opened.notify_all();
        },
    );

    let report = nvs_host::run_until_idle(&mut sched).expect("the loop failed");
    let total = started.elapsed();
    assert_eq!(
        report.finished,
        CHILDREN + 1,
        "a task never came back off the blocking pool"
    );
    assert!(
        nvs_host::blocking::pool_size().0 >= 1,
        "the children were waited for on the core: this thread's pool never started a thread"
    );

    let serve = served.get().expect("the neighbour task never ran");
    let behind = behind_the_neighbour.get();
    println!(
        "{CHILDREN} concurrent spawns: neighbour served at {:.2} ms of {:.2} ms, \
         {behind} of {CHILDREN} children held open until it ran",
        serve.as_secs_f64() * 1e3,
        total.as_secs_f64() * 1e3,
    );

    assert_eq!(
        behind,
        CHILDREN,
        "{} of {CHILDREN} children gave up waiting for the neighbour task, which ran only at \
         {:.2} ms. A child is waited for off the core through `nvs_host::blocking::run`; a \
         child that cannot finish until the neighbour runs, and a neighbour that does not run, \
         mean the wait is holding the core and every other request on it is behind a process.",
        CHILDREN - behind,
        serve.as_secs_f64() * 1e3
    );
}

/// How much of the fan-out this machine can actually run at once: the same total
/// work [`isolate::one_core_batch`] does, spread over [`isolate::WIDTH`] threads
/// instead of over a placement.
///
/// No scheduler, no reactor and no placement under them, so the only thing left
/// for the figure to be about is how many of the cores are free — which is the
/// one question the guard below cannot answer from the code it is measuring.
/// Divided into the same denominator, it is a speedup of the same shape.
///
/// Thread `n` pins itself to `nvs_host::cpus()[n]`, the CPU the worker set's
/// `n`th core is started on (`nvs_host::worker`'s `start_one`), so the figure is
/// about the CPUs the placement runs on and not about whichever ones are free.
/// A worker core cannot move off a busy CPU and an unpinned thread can: under a
/// loaded sweep, unpinned threads found idle CPUs elsewhere, read the machine as
/// free, and the guard below blamed the placement for load on the CPUs it is
/// pinned to.
///
/// Each thread is started once and runs the whole batch. A thread started per
/// fan-out prices thread creation instead, which is a large enough share of one
/// child's work to read *below* the placement this is meant to bound — and a
/// probe shaped that way reports cores as busy when they were only expensive to
/// acquire, skipping a guard that could have run.
fn plain_thread_fan_out(iters: u64) -> Duration {
    let cpus = nvs_host::cpus();
    let start = Instant::now();
    thread::scope(|scope| {
        let running: Vec<_> = (0..isolate::WIDTH)
            .map(|child| {
                let cpu = cpus.get(child).copied();
                scope.spawn(move || {
                    // A refused pin leaves the thread where the OS put it, which
                    // is where a worker core whose pin was refused runs too.
                    if let Some(cpu) = cpu {
                        nvs_host::pin_current_thread(cpu);
                    }
                    // Chained, like every other loop in this file: the same seed
                    // twice is a call an optimiser is free to make once.
                    let mut acc = child as u64;
                    for _ in 0..iters {
                        acc = isolate::burn(black_box(acc));
                    }
                    acc
                })
            })
            .collect();
        for child in running {
            black_box(
                child
                    .join()
                    .expect("a thread runs its children to their end"),
            );
        }
    });
    start.elapsed()
}

/// Where an overlapping fan-out ran: for each of [`isolate::WIDTH`] children
/// posted from one task, the thread it ran on and whether it saw all of them
/// running at once.
///
/// Each child keeps its core until every child has started, or until
/// `patience` runs out. `nvs_host::worker` gives a placement a core that has no
/// other placement on it, and starts another core while the CPU count allows
/// one, so no two of these children share a core. A child that was queued
/// behind another on one core starts only after that one stopped waiting, and
/// the one that stopped waiting answers that it did not see them all.
fn overlapping_fan_out(patience: Duration) -> Vec<(thread::ThreadId, bool)> {
    let mut sched = nvs_host::Scheduler::new();
    let answers: Rc<Cell<Vec<(thread::ThreadId, bool)>>> = Rc::new(Cell::new(Vec::new()));
    let collected = Rc::clone(&answers);
    let running: Arc<(Mutex<usize>, Condvar)> = Arc::new((Mutex::new(0), Condvar::new()));

    let parent = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
    sched.spawn(parent, nvs_runtime::TaskRoot::Request, move |_ctx| {
        // Every child is posted before any is collected, as
        // `isolate::worker_fan_out_batch` posts them.
        let posted: Vec<_> = (0..isolate::WIDTH)
            .map(|child| {
                let running = Arc::clone(&running);
                nvs_host::worker::post(move || {
                    let (started, changed) = &*running;
                    let mut started = started.lock().unwrap_or_else(PoisonError::into_inner);
                    *started += 1;
                    changed.notify_all();
                    let (started, _) = changed
                        .wait_timeout_while(started, patience, |started| *started < isolate::WIDTH)
                        .unwrap_or_else(PoisonError::into_inner);
                    (thread::current().id(), *started >= isolate::WIDTH)
                })
                .unwrap_or_else(|_| panic!("a core takes child {child}"))
            })
            .collect();
        collected.set(
            posted
                .into_iter()
                .map(|child| match child.collect() {
                    nvs_host::worker::Answer::Value(answer) => answer,
                    other => panic!("a child came back as {other:?} rather than with its answer"),
                })
                .collect(),
        );
    });
    // A placed child answers through a wake from another thread, so the parent
    // parks on a reactor, and a post with no reactor installed is refused.
    let installed = nvs_host::reactor::install(nvs_host::Reactor::new().expect("a reactor starts"));
    nvs_host::run_until_idle(&mut sched).expect("the scheduler finishes");
    drop(installed);
    answers.take()
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_cpu_bound_fan_out_across_four_worker_cores_is_near_linear_by_the_margin_this_test_names() {
    let _quiet = serialised();

    // The second figure M5's acceptance asks for, and the one that is a ratio:
    // CPU-bound children placed `on: "worker"` against the same children run one
    // after another on the core that asked for them. The construct is the
    // placement rather than `Core\Task::map`, which is concurrency on one core —
    // `rule:concurrency/on-worker-runs-the-child-on-another-core` is where that
    // is decided, and the shared module owns what the two batches include.
    //
    // The soft floor is the margin this test's name promises, and it is well
    // under the ideal 4x deliberately: a ratio is bounded by whatever else the
    // machine is running. The hard floor sits just over 1x, because the
    // failure worth a red build is a fan-out that stopped fanning out — a
    // refused placement, a picker handing every child to one core, a join that
    // serialises the children — all of which land at or under 1x. Ten times
    // under the soft floor would be under that line and catch none of them.
    //
    // The count beside the speedup is the part no machine moves: how many
    // cores the children ran on. `isolate::WIDTH` children are posted that each
    // keep their core until all of them are running, so they overlap however
    // loaded the machine is. They have to run on `isolate::WIDTH` different
    // threads, and none of those is the thread that posted them. A picker that
    // gives two children one core fails that count, and so does a placement
    // that runs a child where it was posted.
    const SPEEDUP: Bound = Bound::ratio_over(2.0, 1.2);
    const ITERS: u64 = 5;
    const ROUNDS: usize = 5;
    // Only the failing run ever waits this long.
    const PATIENCE: Duration = Duration::from_secs(20);

    let cpus = nvs_host::cpus().len();
    if cpus < isolate::WIDTH {
        println!(
            "fan-out across worker cores: skipped, {cpus} CPU(s) is fewer than the \
             {} a fan-out places",
            isolate::WIDTH
        );
        return;
    }

    let placing = thread::current().id();
    let ran = overlapping_fan_out(PATIENCE);
    let together = ran.iter().filter(|(_, together)| *together).count();
    let threads: std::collections::HashSet<_> = ran.iter().map(|(thread, _)| *thread).collect();
    let on_the_placing_thread = usize::from(threads.contains(&placing));
    println!(
        "{} overlapping children: {together} running at once, on {} threads, \
         {on_the_placing_thread} of them the placing thread [exactly {}, {} and 0]",
        isolate::WIDTH,
        threads.len(),
        isolate::WIDTH,
        isolate::WIDTH
    );
    assert_eq!(
        (together, threads.len(), on_the_placing_thread),
        (isolate::WIDTH, isolate::WIDTH, 0),
        "of {} children posted together, {together} saw all of them running at once, and they \
         ran on {} threads, {on_the_placing_thread} of them the thread that posted them. A \
         fan-out runs each child on a core of its own that is not its parent's \
         (`rule:concurrency/on-worker-runs-the-child-on-another-core`); fewer threads than \
         children is `nvs_host::worker`'s picker giving two children one core.",
        isolate::WIDTH,
        threads.len()
    );

    // Interleaved, so that a machine warming up or throttling mid-test moves
    // both halves rather than one; the minimum of each, for the reason
    // `ns_per_op` above gives. The third batch is the machine's own ceiling, and
    // it is interleaved with them for that same reason.
    //
    // The CPU count above answers a machine too small to fan out at all. It
    // cannot answer a machine whose four cores are all busy with something else,
    // and that machine reads exactly like a picker handing every child to one
    // core: both land under the floor. The third batch tells the two apart,
    // because threads pinned to the CPUs the cores run on are held to the same
    // hard floor, and load on those CPUs fails it too. Under it, the ratio was
    // never there to be measured, so that attempt reports the machine rather
    // than the tree, and the guard reports itself not measured when the last
    // attempt does — the same answer the CPU count gets, for the same reason.
    let measured = judge_where_measurable(
        "fan-out across worker cores",
        SPEEDUP,
        "M5's acceptance claims near-linear speedup across cores for exactly this shape of work; \
         at this ratio the children are sharing a core rather than spreading over them.",
        |yard| {
            let mut placed = f64::MAX;
            let mut one_core = f64::MAX;
            let mut threads = f64::MAX;
            for _ in 0..ROUNDS {
                placed = placed.min(isolate::worker_fan_out_batch(ITERS).as_secs_f64());
                one_core = one_core.min(isolate::one_core_batch(ITERS).as_secs_f64());
                threads = threads.min(plain_thread_fan_out(ITERS).as_secs_f64());
            }
            let speedup = one_core / placed;
            let available = one_core / threads;
            if !SPEEDUP.inside(available, SPEEDUP.hard) {
                return Err(format!(
                    "{} plain threads reach only {available:.2}x on this machine, under the \
                     {}x hard floor, so the {speedup:.2}x the placement reached is a figure \
                     about the machine",
                    isolate::WIDTH,
                    SPEEDUP.hard
                ));
            }
            let per_batch = |seconds: f64| yard.time(seconds * 1e9 / ITERS as f64);
            Ok(Reading {
                value: speedup,
                detail: format!(
                    " ({} children in {} placed vs {} on one core, of the {available:.2}x plain \
                     threads reach here)",
                    isolate::WIDTH,
                    per_batch(placed),
                    per_batch(one_core)
                ),
            })
        },
    );
    if measured.is_none() {
        println!("fan-out across worker cores: skipped, the last attempt was not measured");
    }
}

// ---------------------------------------------------------------------------
// `rule:routing/path-grammar`'s match, priced per row of the table it is asked of
// ---------------------------------------------------------------------------

/// The length of a route table does not show in a request matched against it.
///
/// `nvs_runtime::routes` descends a trie over the rows' fixed prefixes and
/// compares the request only against the rows sharing its own, so the slope
/// between two table sizes is close to nothing. The bound is the one the
/// linear walk the trie replaced was held to, so a figure past it means the
/// match visits rows outside the request's prefix again, or a row began to
/// allocate or convert before it has matched.
///
/// The slope between two table sizes is what is measured, so the split of the
/// path and the capture the answer carries — both per request and neither per
/// row — cancel out. `benches/routing.rs` is the same measurement with
/// criterion's precision; this one can fail a build.
#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_route_table_walk_costs_a_fraction_of_the_request_it_rides_in() {
    let _quiet = serialised();

    // A figure past the hard bound is the table's length showing in a request.
    const PER_ROW: Bound = Bound::time_under(30.0, 300.0);

    let (method, path) = routes::REQUEST;
    let small = routes::table(8);
    let large = routes::table(512);

    judge(
        "route walk",
        PER_ROW,
        "The match should visit only the rows sharing the request's fixed prefix; a table's \
         length showing in a request means the prefix trie in nvs_runtime::routes is bypassed.",
        |yard| {
            let t_small = ns_per_op(200_000, 5, || {
                black_box(small.match_request(black_box(method), black_box(path)));
            });
            let t_large = ns_per_op(20_000, 5, || {
                black_box(large.match_request(black_box(method), black_box(path)));
            });
            Reading {
                value: (t_large - t_small) / 504.0,
                detail: format!(
                    " per row (8 rows {}, 512 rows {})",
                    yard.time(t_small),
                    yard.time(t_large)
                ),
            }
        },
    );
}

// ---------------------------------------------------------------------------
// ADR 0007's claim, tested rather than asserted
// ---------------------------------------------------------------------------
//
// "Mandatory types pay for themselves on the request path" is a claim about
// what the *compiler emits*, so both guards below compile the frozen
// `examples/arith.nvs` fixture through the real pipeline rather than
// approximating it with hand-built IR. The first is structural and the honest
// form of the claim; the second is a timing, and self-relative per ADR 0026.

/// Compiles `examples/arith.nvs` and returns its lowered program alongside the
/// compiled unit.
fn compile_arith() -> (nvs_ir::Program, nvs_codegen::Unit) {
    let path = nvs_repo::path("examples/arith.nvs");
    let text = std::fs::read_to_string(path).expect("the frozen acceptance fixture is readable");
    compile_source("arith.nvs", &text)
}

/// Compiles one source text through the whole real pipeline — parse, resolve,
/// check, lower, codegen — and returns its lowered program alongside the
/// compiled unit.
///
/// Every guard below that makes a claim about emitted code goes through this
/// rather than hand-building IR: the claims are about what the *compiler*
/// does, so an approximation of its output would not test them.
fn compile_source(name: &str, text: &str) -> (nvs_ir::Program, nvs_codegen::Unit) {
    let mut map = nvs_diagnostics::SourceMap::new();
    let id = map.add(name, text);
    let src = map.file(id);

    let mut diags = nvs_diagnostics::Diagnostics::new();
    let stmts = nvs_syntax::parse_file(src, &mut diags);
    let module = nvs_hir::resolve_file(&stmts, src, &mut diags);
    let mut interner = nvs_types::TypeInterner::new();
    let mut exprs = nvs_types::ExprTypeTable::new();
    let files = [nvs_types::ProgramFile { src, stmts: &stmts }];
    let enums = nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "{name} stopped type-checking");

    let layouts = nvs_types::build_class_layouts(&files, &module.graph);
    let program = nvs_ir::lower::lower_file(
        nvs_ir::lower::ENTRY_SCRIPT_LABEL,
        &stmts,
        src,
        &exprs,
        &interner,
        &enums,
        &layouts,
    );
    let unit = nvs_codegen::compile(&program).expect("the fixture compiles");
    (program, unit)
}

/// The lowered `Bench::sum`, whose body is the fixture's `while` loop.
fn bench_sum(program: &nvs_ir::Program) -> &nvs_ir::Function {
    program
        .functions
        .iter()
        .find(|f| f.name == "Bench::sum")
        .expect("the fixture declares Bench::sum")
}

#[test]
fn a_typed_arithmetic_loop_contains_no_call() {
    use nvs_ir::ir::InstKind;

    let _quiet = serialised();

    let (program, _unit) = compile_arith();
    let sum = bench_sum(&program);

    // Half one: the loop's arithmetic reached no helper and no Novis function.
    // `$total + $i * 2 - 1` and `$i < $n` are native instructions because
    // ADR 0007 settled both operands' types before lowering; an untyped
    // language has to call something here.
    let insts = || sum.blocks.iter().flat_map(|b| b.insts.iter());
    let calls: Vec<&InstKind> = insts()
        .map(|i| &i.kind)
        .filter(|k| matches!(k, InstKind::Call { .. } | InstKind::HelperCall { .. }))
        .collect();
    assert!(
        calls.is_empty(),
        "Bench::sum now contains {} call instruction(s): {calls:?}. ADR 0007 justifies \
         mandatory types on typed arithmetic lowering to native instructions; if a call \
         genuinely belongs here now, that argument needs revisiting.",
        calls.len()
    );

    // Half two: every `call` the *machine code* contains is accounted for by a
    // safepoint poll, an ADR 0018 probe or an overflow raise — all out-of-line,
    // the first two emitted unconditionally and guarded for cost separately
    // above. Tying the machine-code count to the IR site count is what makes
    // half one a claim about the emitted code rather than only about the IR.
    let sites = insts()
        .filter(|i| matches!(i.kind, InstKind::Safepoint | InstKind::StmtMarker(_)))
        .count();
    // The remaining accounted categories, both owed to ADR 0007 § 4's overflow
    // throw. Each checked integer arithmetic instruction owns a cold block
    // calling `nvs_runtime::nvs_raise_new`, and the ADR 0002 error edge it
    // takes ends in a landing block whose `Propagate` calls `nvs_trace_push`.
    // Both are out-of-line and neither is reached while the arithmetic fits —
    // the hot path is still one machine instruction plus a predicted not-taken
    // branch — so they are accounted for here rather than read as calls the
    // loop pays for.
    let raises = insts()
        .filter(|i| {
            i.on_error.is_some() && matches!(i.kind, InstKind::BinOp { .. } | InstKind::UnOp { .. })
        })
        .count();
    let landings = sum
        .blocks
        .iter()
        .filter(|b| matches!(b.term, nvs_ir::ir::Terminator::Propagate { .. }))
        .count();
    // Scanned line by line rather than by splitting on the section marker:
    // Cranelift renders a two-way branch as `jnz label3; j label2`, so a
    // `"; "` split would cut the section short at the first branch.
    //
    // The mnemonic is the backend's, and there is more than one spelling of
    // it: x86_64 emits `call`, aarch64 `bl` for a direct call and `blr`
    // through a register. Counting `call` alone reads an aarch64 build as a
    // loop containing no calls at all, comparing 0 against a site count only
    // x86_64 can match. Half two claims something about the emitted code, and
    // that claim is per backend or it is nothing.
    let is_call = |line: &str| {
        ["call ", "bl ", "blr "]
            .iter()
            .any(|op| line.starts_with(op))
    };
    let asm = nvs_codegen::disassemble(&program).expect("the fixture compiles");
    let mut in_section = false;
    let mut emitted = 0;
    for line in asm.lines() {
        if let Some(name) = line.strip_prefix("; ") {
            in_section = name == "Bench::sum";
        } else if in_section && is_call(line.trim_start()) {
            emitted += 1;
        }
    }
    println!(
        "Bench::sum: {sites} probe/safepoint sites, {raises} overflow raises, \
         {landings} landing blocks, {emitted} emitted calls"
    );

    assert_eq!(
        emitted,
        sites + raises + landings,
        "Bench::sum emits {emitted} call(s) for {sites} probe/safepoint site(s), \
         {raises} overflow raise(s) and {landings} landing block(s). Every call in a \
         typed arithmetic loop should be one of those out-of-line slow paths and \
         nothing else."
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_typed_arithmetic_loop_stays_in_the_native_cost_class() {
    let _quiet = serialised();

    // Self-relative, per ADR 0026: the bound is a multiple of the
    // checked-return frame cost this file already measures on *this* machine,
    // never an absolute figure quoted from another one.
    //
    // An iteration does a compare, three arithmetic operations, a safepoint
    // poll and six ADR 0018 probe checks — and still costs less than one
    // cross-frame call, which is the whole of ADR 0007's claim. The soft bound
    // is set well above that. Arithmetic going through a runtime helper instead
    // of a native instruction puts four calls in each iteration, and
    // `a_typed_arithmetic_loop_contains_no_call` above is what fails on it,
    // by count; this timing holds the cost class around it.
    const ITERATION_PER_FRAME: Bound = Bound::ratio_under(6.0, 60.0);
    const ITERATIONS: i64 = 1_000;

    let mut probe = Probe::new();
    let shallow = probe.compile_chain(2, Helper::Double);
    let deep = probe.compile_chain(18, Helper::Double);
    let mut probe_ctx = Ctx::new();
    let arg = Value::int(3);

    let (_program, unit) = compile_arith();
    // `raw_function` and not `Unit::call_static`, which is the entry point a
    // hand caller otherwise wants: that one builds the slot array per call and
    // looks a descriptor up, and both are measurable against a loop iteration.
    // The array below is built once, outside the timing.
    let sum = unit
        .raw_function("Bench::sum")
        .expect("the fixture declares Bench::sum");
    let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
    // Argument slot 0 is the implicit receiver every lowered method carries;
    // `Bench::sum` is static, so it is `null` — see `emit_call`'s own docs. A
    // `null` there is honest only because this fixture uses no late static
    // binding; `Unit::call_static` passes the real called-class descriptor, and
    // anything that reads `static::` needs that and not this.
    let args = [
        nvs_runtime::Value::null(),
        nvs_runtime::Value::int(ITERATIONS),
    ];
    judge(
        "typed arithmetic loop iteration, against a checked-return frame",
        ITERATION_PER_FRAME,
        "ADR 0007 justifies mandatory types on typed arithmetic staying in the native cost \
         class; if this is a real regression that argument needs revisiting.",
        |yard| {
            let t_shallow = ns_per_op(200_000, 5, || {
                black_box(call(shallow, &mut probe_ctx, arg));
            });
            let t_deep = ns_per_op(200_000, 5, || {
                black_box(call(deep, &mut probe_ctx, arg));
            });
            let per_frame = (t_deep - t_shallow) / 16.0;
            let per_call = ns_per_op(2_000, 5, || {
                black_box(nvs_runtime::call(sum, &mut ctx, &args)).expect("the loop ran");
            });
            let per_iteration = per_call / ITERATIONS as f64;
            Reading {
                value: per_iteration / per_frame,
                detail: format!(
                    " ({} per iteration, {} per frame)",
                    yard.time(per_iteration),
                    yard.time(per_frame)
                ),
            }
        },
    );
}

// ---------------------------------------------------------------------------
// ADR 0054's claim, measured rather than asserted
// ---------------------------------------------------------------------------
//
// `decimal` is the arithmetic type whose operators are out-of-line helper calls
// over a 96-bit mantissa rather than one native instruction, so the guard above
// says nothing about it: a `decimal` loop is *expected* to carry calls. What it
// owes instead is a cost class — several helper calls per iteration, not a
// heap allocation or an arbitrary-precision path per operation.

/// Compiles the `decimal` arithmetic loop `benches/members/` measures as this
/// feature's figure, and returns the compiled unit.
///
/// The guard below and the ledger figure read the **same program**, for the
/// reason `shared/isolate.rs` is shared above: two loops written to be alike
/// drift apart, and then the number in `docs/perf/members.ndjson` is no longer
/// the number this test protects.
fn compile_decimal_arith() -> nvs_codegen::Unit {
    let path = nvs_repo::path("benches/members/lang/types/numbers-bool-int-uint-float-decimal.nvs");
    let text = std::fs::read_to_string(path).expect("the bench program is readable");
    compile_source("numbers-bool-int-uint-float-decimal.nvs", &text).1
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_typed_decimal_arithmetic_loop_stays_in_its_cost_class() {
    let _quiet = serialised();

    // Self-relative, per ADR 0026: the bound is a multiple of the
    // checked-return frame cost this file already measures on *this* machine,
    // never an absolute figure quoted from another one.
    //
    // An iteration is three `decimal` operations — a multiply, an add and a
    // subtract — plus the `int` compare and increment the `int` loop above pays
    // too. Each of the three is an out-of-line helper call over a 96-bit
    // mantissa, so this loop sits tens of frames up rather than under one.
    //
    // The soft bound sits a few times over that, closer than the usual ten,
    // and the closeness is the point. What it warns about is a change of
    // *kind* — a `decimal` that starts allocating, or an operator that falls
    // back to an arbitrary-precision path, against
    // `crates/nvs-runtime/src/decimal.rs`'s "nothing is allocated and nothing
    // is refcounted" — and that costs tens of nanoseconds per operator, which
    // can stay inside the hard bound, ten times the soft one. A warning from
    // this guard is therefore worth reading even though the test passed.
    const ITERATION_PER_FRAME: Bound = Bound::ratio_under(120.0, 1_200.0);
    const ITERATIONS: i64 = 1_000;

    let mut probe = Probe::new();
    let shallow = probe.compile_chain(2, Helper::Double);
    let deep = probe.compile_chain(18, Helper::Double);
    let mut probe_ctx = Ctx::new();
    let arg = Value::int(3);

    let unit = compile_decimal_arith();
    // `raw_function` rather than `Unit::call_static`, and argument slot 0 the
    // implicit `null` receiver, both for the reasons the `int` guard above
    // gives at the same call.
    let run = unit
        .raw_function("Bench::run")
        .expect("the bench program declares Bench::run");
    let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
    let args = [
        nvs_runtime::Value::null(),
        nvs_runtime::Value::int(ITERATIONS),
    ];
    judge(
        "typed decimal arithmetic loop iteration, against a checked-return frame",
        ITERATION_PER_FRAME,
        "ADR 0054 § *Consequences* claims a helper call per operator and no allocation; at \
         this ratio one of those two has stopped being true.",
        |yard| {
            let t_shallow = ns_per_op(200_000, 5, || {
                black_box(call(shallow, &mut probe_ctx, arg));
            });
            let t_deep = ns_per_op(200_000, 5, || {
                black_box(call(deep, &mut probe_ctx, arg));
            });
            let per_frame = (t_deep - t_shallow) / 16.0;
            let per_call = ns_per_op(2_000, 5, || {
                black_box(nvs_runtime::call(run, &mut ctx, &args)).expect("the loop ran");
            });
            let per_iteration = per_call / ITERATIONS as f64;
            Reading {
                value: per_iteration / per_frame,
                detail: format!(
                    " ({} per iteration, {} per frame)",
                    yard.time(per_iteration),
                    yard.time(per_frame)
                ),
            }
        },
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_refcount_one_array_member_mutates_in_place() {
    let _quiet = serialised();

    // Self-relative, per ADR 0026: the bound compares two writes measured on
    // *this* machine, never an absolute figure quoted from another one.
    //
    // ADR 0063 R3 says no `Core` member mutates its subject — every one
    // returns a fresh value. That is only affordable because a write into an
    // array nothing else holds is done in place, with no copy at all: the
    // `refcount == 1` fast path in `nvs_runtime::array`. This guard is what
    // makes the claim checkable rather than remembered. It measures one write
    // into a solely-owned thousand-entry array against the same write into an
    // aliased one, which has to separate first and is therefore O(entries).
    //
    // The in-place write is orders of magnitude cheaper, and the soft bound
    // sits far above it. The hard bound sits at half a separating write, just
    // short of the change of *kind* it exists to catch: the fast path being
    // lost puts the two within a small constant factor of each other, which
    // ten times the soft bound would not reach.
    //
    // The proof is counted, so it holds on any machine however loaded. A write
    // to a key the solely-owned array already has replaces one value where it
    // is, and makes no allocation. The same write through an aliased handle
    // copies the entries first, and a copy allocates: that second count is
    // what shows the first one can see a separation at all. The timing beside
    // them is a sanity bound on the work that allocates nothing.
    const IN_PLACE_PER_SEPARATING: Bound = Bound::ratio_under(0.05, 0.5);
    const ENTRIES: i64 = 1_000;
    const COUNTED: i64 = 1_000;

    let mut array = nvs_runtime::NvsArray::new();
    for index in 0..ENTRIES {
        array.set(
            nvs_runtime::NvsStr::new(index.to_string().as_bytes()),
            nvs_runtime::Value::int(index),
        );
    }
    let key = nvs_runtime::NvsStr::new(b"probe");

    // This write adds the key. Every write after it replaces the key's value.
    array.set(key.clone(), nvs_runtime::Value::int(0));
    let in_place = allocations_during(|| {
        for round in 0..COUNTED {
            array.set(key.clone(), nvs_runtime::Value::int(round));
        }
    });
    let separating = allocations_during(|| {
        let mut aliased = array.clone();
        aliased.set(key.clone(), nvs_runtime::Value::int(1));
        black_box(aliased.count());
    });
    println!(
        "{COUNTED} array writes in place: {in_place} allocations [exactly 0]; one separating \
         write of {ENTRIES} entries: {separating} [more than 0]"
    );
    assert_eq!(
        in_place, 0,
        "{COUNTED} writes to a key of a solely-owned array made {in_place} allocations. A write \
         in place replaces one value and allocates nothing; an allocation there is the \
         `refcount == 1` fast path in `nvs_runtime::array` copying or growing the entries, and \
         ADR 0063 R3's immutable `Core` API rests on it doing neither."
    );
    assert!(
        separating > 0,
        "a write through an aliased handle made no allocation, so this guard is no longer \
         telling a write in place from one that copies the entries."
    );

    judge(
        "array write in place, against a separating one",
        IN_PLACE_PER_SEPARATING,
        "ADR 0063 R3's immutable `Core` API rests on that write being in place; if this is a \
         real regression that argument needs revisiting.",
        |yard| {
            let in_place = ns_per_op(200_000, 5, || {
                array.set(key.clone(), nvs_runtime::Value::int(1));
            });
            assert_eq!(array.refcount(), 1, "the in-place write never separated");

            let separating = ns_per_op(300, 5, || {
                let mut aliased = array.clone();
                aliased.set(key.clone(), nvs_runtime::Value::int(1));
                black_box(aliased.count());
            });
            Reading {
                value: in_place / separating,
                detail: format!(
                    " ({} in place vs {} separating {ENTRIES} entries)",
                    yard.time(in_place),
                    yard.time(separating)
                ),
            }
        },
    );
}

/// The two bodies of text the granularity decision turns on, each repeated
/// until it is a few kilobytes: the ASCII one every request path is actually
/// full of, and a mixed one carrying accented Latin built from combining marks
/// and emoji clusters that span several code points each.
///
/// Both, not one. A grapheme segmenter has a fast path for an ASCII run, so an
/// ASCII-only corpus would measure that fast path and report that granularity
/// is free; a corpus that is *all* emoji would report the opposite. The
/// decision needs the common case and the bad case side by side, so this guard
/// prints and bounds both.
///
/// Each corpus comes with the bounds on its grapheme count, in UTF-8
/// validations of the same buffer. Each soft bound is an order of magnitude
/// above that corpus's usual figure. The `mixed` hard bound is ten times its
/// soft one. The `ascii` hard bound is closer, because the change of kind it
/// exists to catch is `one_byte_per_cluster`'s fast path being lost, which
/// puts the `ascii` leg in the hundreds.
fn corpora() -> [(&'static str, String, Bound); 2] {
    let repeat = |unit: &str| unit.repeat(64);
    [
        (
            "ascii",
            repeat("the quick brown fox jumps over the lazy dog, "),
            Bound::ratio_under(30.0, 100.0),
        ),
        (
            "mixed",
            repeat(concat!(
                "the quick brown fox jumps over the lazy dog, ",
                "na\u{131}\u{308}ve cafe\u{301} re\u{301}sume\u{301}, ",
                "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} \u{1f1e6}\u{1f1f9} ",
                "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f} ",
            )),
            Bound::ratio_under(250.0, 2_500.0),
        ),
    ]
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_grapheme_index_costs_more_than_a_code_point_index() {
    let _quiet = serialised();

    // ADR 0009 § 2's granularity, decided by this measurement, which its
    // *Revisiting* section asked for by name.
    //
    // The reference cost that ADR names is UTF-8 validation: `bytes as string`
    // already pays one O(n) pass over the buffer at every such conversion, and
    // that cost is already accepted. So the question is how many of those
    // passes a grapheme count is worth, and the answer splits by corpus —
    // which is why the bound travels with the corpus rather than being one
    // constant here. The name is the finding on text that is not plain ASCII:
    // a grapheme index is the dearer of the two seams, by orders of magnitude.
    //
    // Off the fast path a grapheme count also costs more than a code-point
    // count. The soft floor is that claim exactly. The hard floor is a grapheme
    // count at half a code-point count's cost, which a segmenter that still
    // segments cannot reach; ten times under the soft floor would pass one that
    // had become free.
    const GRAPHEME_PER_CODE_POINT: Bound = Bound::ratio_over(1.0, 0.5);

    for (name, text, per_validation) in corpora() {
        let bytes = text.as_bytes();
        let graphemes = nvs_stdlib::granularity::Unit::Grapheme.length(&text);
        let code_points = nvs_stdlib::granularity::Unit::CodePoint.length(&text);

        // Per byte, so the three legs are comparable and the corpus size is
        // not baked into the bounds: UTF-8 validation, a code-point count and
        // a grapheme count, in that order.
        let legs = || {
            let per_byte = |ns: f64| ns / bytes.len() as f64;
            (
                per_byte(ns_per_op(2_000, 5, || {
                    black_box(std::str::from_utf8(black_box(bytes)).is_ok());
                })),
                per_byte(ns_per_op(2_000, 5, || {
                    black_box(nvs_stdlib::granularity::Unit::CodePoint.length(black_box(&text)));
                })),
                per_byte(ns_per_op(2_000, 5, || {
                    black_box(nvs_stdlib::granularity::Unit::Grapheme.length(black_box(&text)));
                })),
            )
        };
        let detail = |yard: Yardstick, (validate, code_point, grapheme): (f64, f64, f64)| {
            format!(
                " ({} bytes, {graphemes} graphemes, {code_points} code points; per byte: \
                 validate {}, code point {}, grapheme {})",
                bytes.len(),
                yard.time(validate),
                yard.time(code_point),
                yard.time(grapheme)
            )
        };

        judge(
            &format!("{name}: a grapheme count, in UTF-8 validations of the same buffer"),
            per_validation,
            "ADR 0009 § 2 makes grapheme clusters `string`'s default unit on the strength of a \
             measured figure well under that; if this is a real regression, that decision needs \
             revisiting rather than this bound.",
            |yard| {
                let figures = legs();
                Reading {
                    value: figures.2 / figures.0,
                    detail: detail(yard, figures),
                }
            },
        );

        // Only off the fast path. On plain ASCII a grapheme count *is* the
        // byte length behind one vectorized scan, so it is legitimately in the
        // same class as a code-point count there and the comparison says
        // nothing; judging it on both legs would be a coin flip.
        if !text.is_ascii() {
            judge(
                &format!("{name}: a grapheme count, against a code-point count"),
                GRAPHEME_PER_CODE_POINT,
                "The two are meant to be distinct seams over the same buffer; if segmentation \
                 has become free, `nvs_stdlib::granularity` is no longer measuring what it \
                 claims to.",
                |yard| {
                    let figures = legs();
                    Reading {
                        value: figures.2 / figures.1,
                        detail: detail(yard, figures),
                    }
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ADR 0056's claim, measured rather than asserted
// ---------------------------------------------------------------------------
//
// `rule:core-classes/regex-two-tiers` routes a pattern to the linear engine
// unless that engine cannot express it, and a developer has no way to ask for
// the other one. That is defensible only if the tier nobody can choose is not
// the slow one, so the two are measured against each other here rather than
// asserted in prose.

/// The corpus both tiers scan, once, repeated into a few kilobytes.
const CORPUS_UNIT: &str = "the quick brown fox jumps over the lazy dog, ";

/// How many copies of [`CORPUS_UNIT`] the scanned subject holds.
const CORPUS_REPEATS: usize = 64;

/// The pattern the linear engine expresses.
const LINEAR_PATTERN: &str = r"\b[a-z]+@[a-z]+\.[a-z]{2,4}\b";

/// The same pattern behind a lookahead that is satisfied wherever the pattern
/// could start, so it matches exactly what [`LINEAR_PATTERN`] matches and
/// differs only in the engine it can run on — a lookaround is the construct
/// `rule:core-classes/regex-two-tiers` names as reaching the second tier.
///
/// A twin rather than one pattern run twice, because there is no second pattern
/// here: the tier is a property of the text, so the only way to put the same
/// work on both engines is to write the same search two ways and check, with
/// `validate`, that each landed where it was meant to.
const BACKTRACKING_PATTERN: &str = r"(?=[a-z])\b[a-z]+@[a-z]+\.[a-z]{2,4}\b";

/// Compiles the program that scans [`CORPUS_UNIT`]'s corpus with a pattern,
/// with both patterns substituted into their own entry point.
///
/// Through the real pipeline and `Core\Regex::matches`, not against the two
/// engine crates directly: what the claim is about is the throughput a *Novis
/// program* sees, which includes the cache lookup and the argument decoding
/// each call pays. Both tiers pay them identically, so they cancel in the
/// ratio and leave the scan itself.
fn compile_regex_scan() -> nvs_codegen::Unit {
    let source = r"<?nvs
class Bench {
    public static function linear(int $rounds): int {
        return Bench::scan($rounds, '@LINEAR@');
    }

    public static function backtracking(int $rounds): int {
        return Bench::scan($rounds, '@BACKTRACKING@');
    }

    public static function scan(int $rounds, string $pattern): int {
        var $subject = Core\Str::repeat('@UNIT@', @REPEATS@);
        var $found = 0;
        var $i = 0;
        while ($i < $rounds) {
            if (Core\Regex::matches($subject, $pattern)) {
                $found = $found + 1;
            }
            $i = $i + 1;
        }
        return $found;
    }
}
"
    .replace("@LINEAR@", LINEAR_PATTERN)
    .replace("@BACKTRACKING@", BACKTRACKING_PATTERN)
    .replace("@UNIT@", CORPUS_UNIT)
    .replace("@REPEATS@", &CORPUS_REPEATS.to_string());
    compile_source("regex-tiers.nvs", &source).1
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn the_linear_regex_tier_keeps_pace_with_the_backtracking_tier_on_one_corpus() {
    let _quiet = serialised();

    // Self-relative, per ADR 0026: the two tiers are measured against each
    // other in the same run, never against a figure quoted from another
    // machine. Neither pattern matches anywhere in the corpus, so each leg is
    // a full scan of the same bytes rather than a race to an early hit.
    //
    // The soft floor is 1.0 — the linear tier must simply be the faster of the
    // two — because that is the claim exactly: a developer who cannot choose a
    // tier is not being handed the slower one. The margin is usually orders of
    // magnitude, since the literal `@` gives the linear engine a prefilter that
    // rejects the whole corpus in one pass and the lookahead denies the
    // backtracker the same trick. The hard floor is a linear tier ten times the
    // slower, which no timing noise explains.
    const SPEEDUP: Bound = Bound::ratio_over(1.0, 0.1);
    const ROUNDS: i64 = 50;

    assert_eq!(
        nvs_stdlib::regex::validate(LINEAR_PATTERN),
        Ok(nvs_stdlib::regex::Tier::Linear),
        "the linear leg's pattern no longer routes to the linear engine, so this \
         guard would be comparing one tier with itself"
    );
    assert_eq!(
        nvs_stdlib::regex::validate(BACKTRACKING_PATTERN),
        Ok(nvs_stdlib::regex::Tier::Backtracking),
        "the backtracking leg's pattern no longer routes to the backtracking \
         engine, so this guard would be comparing one tier with itself"
    );

    let unit = compile_regex_scan();
    let bytes = (CORPUS_UNIT.len() * CORPUS_REPEATS) as f64;
    let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
    let args = [nvs_runtime::Value::null(), nvs_runtime::Value::int(ROUNDS)];
    // Nanoseconds per byte scanned: the corpus size then sits in neither the
    // printed figure nor the threshold.
    let mut per_byte = |entry: &str| {
        let scan = unit
            .raw_function(entry)
            .unwrap_or_else(|| panic!("the fixture declares Bench::{entry}"));
        let per_call = ns_per_op(20, 5, || {
            black_box(nvs_runtime::call(scan, &mut ctx, &args)).expect("the scan ran");
        });
        per_call / ROUNDS as f64 / bytes
    };
    judge(
        "regex tiers, the linear tier's speedup over the backtracking one",
        SPEEDUP,
        "`rule:core-classes/regex-two-tiers` gives a developer no way to ask for the second \
         tier, which rests on the first not being the slower one; if this is a real regression \
         that argument needs revisiting.",
        |yard| {
            let linear = per_byte("Bench::linear");
            let backtracking = per_byte("Bench::backtracking");
            Reading {
                value: backtracking / linear,
                detail: format!(
                    " over {bytes:.0} bytes (per byte: linear {}, backtracking {})",
                    yard.time(linear),
                    yard.time(backtracking)
                ),
            }
        },
    );
}

// ---------------------------------------------------------------------------
// ADR 0014 § 4's claim, tested rather than asserted
// ---------------------------------------------------------------------------

/// The fixture the ADR 0014 guard compiles, holding both sides of that ADR's
/// § 4 claim in one class so they share a layout, a header and an allocation.
///
/// `$n` is an ordinary property on a class that declares neither a hook for it
/// nor `PropertyObserver`; `$m` is the same `int` behind the cheapest possible
/// opt-in, a pass-through `get`/`set` pair that does nothing but name its own
/// backing slot. `Cell::touch` exists for the structural half and takes its
/// receiver as a parameter, so that the one `Call` a `new` would contribute
/// does not have to be excused; the four `plain*`/`hooked*` methods exist for
/// the timing half and differ from each other by exactly three access pairs
/// per iteration, which is what makes the slope between them the cost of an
/// access rather than of a call frame.
const OBSERVER_FIXTURE: &str = r#"<?nvs
class Cell {
    public int $n;

    public int $m {
        get => $this->m;
        set => $value;
    }

    public function constructor(int $n) {
        $this->n = $n;
        $this->m = $n;
    }

    public static function touch(Cell $c, int $rounds): int {
        var $i = 0;
        while ($i < $rounds) {
            $c->n = $c->n + 1;
            $i = $i + 1;
        }
        return $c->n;
    }

    public static function plainOne(int $rounds): int {
        var $c = new Cell(0);
        var $i = 0;
        while ($i < $rounds) {
            $c->n = $c->n + 1;
            $i = $i + 1;
        }
        return $c->n;
    }

    public static function plainFour(int $rounds): int {
        var $c = new Cell(0);
        var $i = 0;
        while ($i < $rounds) {
            $c->n = $c->n + 1;
            $c->n = $c->n + 1;
            $c->n = $c->n + 1;
            $c->n = $c->n + 1;
            $i = $i + 1;
        }
        return $c->n;
    }

    public static function hookedOne(int $rounds): int {
        var $c = new Cell(0);
        var $i = 0;
        while ($i < $rounds) {
            $c->m = $c->m + 1;
            $i = $i + 1;
        }
        return $c->m;
    }

    public static function hookedFour(int $rounds): int {
        var $c = new Cell(0);
        var $i = 0;
        while ($i < $rounds) {
            $c->m = $c->m + 1;
            $c->m = $c->m + 1;
            $c->m = $c->m + 1;
            $c->m = $c->m + 1;
            $i = $i + 1;
        }
        return $c->m;
    }
}
"#;

/// The `call` instructions the emitted machine code for `name` contains **on
/// the path a straight-line execution takes** — returned rather than counted
/// so a failure can name them.
///
/// Scanned line by line rather than by splitting on the section marker, for
/// the reason `a_typed_arithmetic_loop_contains_no_call` gives: Cranelift
/// renders a two-way branch as `jnz label3; j label2`, so a `"; "` split would
/// cut the section short at the first branch.
///
/// The whole function is the wrong denominator for a per-access cost: every
/// status-returning instruction owns a landing block, so each added
/// `$c->n = $c->n + 1` brings an overflow raise, a `Release` of the receiver
/// and a `Propagate` with it — three machine calls that no run reaches unless
/// the addition has already overflowed. Counting those reads an access as a
/// dispatch it never performs, which is exactly what ADR 0014 § 4 claims it is
/// not. So the
/// blocks are walked from the entry, and the **taken** edge of a
/// `test`-then-`jnz` pair is not followed: that pair is how codegen renders
/// every check of a status word — a call's error return, an overflow's `seto`,
/// the safepoint's pending-exception field, the ADR 0018 probe's null table
/// pointer — so its target is by construction a block a run that throws
/// nothing never enters. Every other edge is followed, the `cmpq`-driven
/// branches a real condition compiles to included, so the loop's own two arms
/// both count.
///
/// **x86_64 only, and the one caller is gated to match.** The walk reads
/// `jmp`, the `jnz`/`test` pair and the `; j ` two-way form, which are that
/// backend's own spellings; aarch64 writes `b`, `cbz`/`b.ne` and puts the
/// second branch on its own line, so the walk would leave every block without
/// a successor and report the empty path. A guard that passes because it
/// looked at nothing is worse than one that says it did not run.
fn path_calls(program: &nvs_ir::Program, name: &str) -> Vec<String> {
    let asm = nvs_codegen::disassemble(program).expect("the fixture compiles");

    // The section's `blockN:` bodies, in emitted order. The prologue ahead of
    // the first label holds no call and belongs to no block, so it is dropped.
    let mut blocks: Vec<(usize, Vec<String>)> = Vec::new();
    let mut in_section = false;
    for line in asm.lines() {
        if let Some(section) = line.strip_prefix("; ") {
            in_section = section == name;
        } else if in_section {
            let line = line.trim();
            if let Some(index) = line
                .strip_prefix("block")
                .and_then(|rest| rest.strip_suffix(':'))
                .and_then(|index| index.parse().ok())
            {
                blocks.push((index, Vec::new()));
            } else if let Some((_, body)) = blocks.last_mut() {
                body.push(line.to_owned());
            }
        }
    }

    let label = |operand: &str| -> Option<usize> {
        operand
            .rsplit_once("label")
            .and_then(|(_, index)| index.trim().parse().ok())
    };
    let successors = |body: &[String]| -> Vec<usize> {
        let mut out = Vec::new();
        for (i, inst) in body.iter().enumerate() {
            if inst.starts_with("jmp ") {
                out.extend(label(inst));
            } else if let Some((taken, fallthrough)) = inst.split_once("; j ") {
                let status = taken.starts_with("jnz")
                    && i.checked_sub(1)
                        .and_then(|previous| body.get(previous))
                        .is_some_and(|previous| previous.starts_with("test"));
                if !status {
                    out.extend(label(taken));
                }
                out.extend(label(fallthrough));
            }
        }
        out
    };

    let mut queue: Vec<usize> = blocks
        .first()
        .map(|(index, _)| *index)
        .into_iter()
        .collect();
    let bodies: std::collections::BTreeMap<usize, Vec<String>> = blocks.into_iter().collect();
    let mut walked: std::collections::BTreeSet<usize> = Default::default();
    while let Some(block) = queue.pop() {
        if !walked.insert(block) {
            continue;
        }
        if let Some(body) = bodies.get(&block) {
            queue.extend(successors(body));
        }
    }

    walked
        .iter()
        .filter_map(|block| bodies.get(block))
        .flatten()
        .filter(|line| line.starts_with("call "))
        .cloned()
        .collect()
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
#[cfg_attr(
    not(target_arch = "x86_64"),
    ignore = "`path_calls` walks x86_64 branch mnemonics"
)]
fn a_class_without_a_property_observer_costs_nothing_extra() {
    use nvs_ir::ir::InstKind;

    let _quiet = serialised();

    // ADR 0014 § 4: a property with no hook, on a class that does not
    // implement `PropertyObserver`, compiles to a direct field load or store —
    // "no branch, no virtual call, nothing paid by a class that never asked
    // for either mechanism". That sentence is what buys the whole ADR: an
    // observer is a language-wide mechanism, so it is only affordable if the
    // classes that ignore it pay for it exactly nothing.
    //
    // `PropertyObserver` itself has no implementation yet — it is deliberately
    // absent from `nvs_hir::interfaces::RESERVED`, which says so — so the
    // reference this measures against is the ADR's *other* opt-in, a
    // per-property hook. That is the stronger of the two references anyway: a
    // hook is the cheapest thing an access can become once it stops being a
    // slot touch, one ordinary call each way, and an observer would add a
    // second on top of it. So if an unhooked access ever acquired a dispatch,
    // its cost would climb into the hooked one's class and this ratio would
    // approach 1.
    //
    // Self-relative, per ADR 0026: both legs are measured on this machine, in
    // the same run, over the same class.
    //
    // The unhooked pair costs a small fraction of the hooked one, and the soft
    // bound sits an order of magnitude above that fraction. The hard bound sits
    // just short of the ~1x an unhooked access would reach the moment it
    // acquired a dispatch of its own — the change of kind this exists to
    // catch, and one that ten times the soft bound would pass.
    const PLAIN_PER_HOOKED: Bound = Bound::ratio_under(0.2, 0.8);
    const ROUNDS: i64 = 1_000;

    let (program, unit) = compile_source("observer.nvs", OBSERVER_FIXTURE);

    // Half one, and the honest form of the claim: the accesses reached no
    // helper and no Novis function. `Cell::touch`'s body is nothing but a loop
    // reading and writing `$n`, so an empty call list here *is* "no virtual
    // call" — and the same body over the hooked `$m` would not have one, which
    // `each_property_hook_is_compiled_under_its_own_label` in `nvs-codegen`
    // holds from the other side.
    let touch = program
        .functions
        .iter()
        .find(|f| f.name == "Cell::touch")
        .expect("the fixture declares Cell::touch");
    let insts = || touch.blocks.iter().flat_map(|b| b.insts.iter());
    let calls: Vec<&InstKind> = insts()
        .map(|i| &i.kind)
        .filter(|k| matches!(k, InstKind::Call { .. } | InstKind::HelperCall { .. }))
        .collect();
    assert!(
        calls.is_empty(),
        "an unhooked property access on an observer-free class now lowers to \
         {} call instruction(s): {calls:?}. ADR 0014 § 4 makes that access a \
         direct field load or store; if a call genuinely belongs here now, \
         that ADR's zero-cost claim needs revisiting.",
        calls.len()
    );

    // Not vacuous: the accesses are actually there, and they are field
    // instructions rather than having been folded away.
    let fields = insts()
        .filter(|i| {
            matches!(
                i.kind,
                InstKind::FieldGet { .. } | InstKind::FieldSet { .. }
            )
        })
        .count();
    println!("Cell::touch: {fields} field instructions, no call instructions");
    assert!(
        fields >= 2,
        "Cell::touch lowered {fields} field instruction(s); its loop reads and \
         writes $n, so this guard is measuring something other than it thinks."
    );

    // And the same tie to the *emitted* code the arithmetic guard makes,
    // which is what turns the check above from a claim about IR into a claim
    // about machine code. Taken as a slope between two bodies rather than as
    // an absolute count, for the same reason the timing below is: a frame
    // pays for things that are not this ADR's — the allocation, the loop, the
    // one refcount call an object-typed local costs at its scope's end — and
    // all of those are per-frame, so they cancel. What is left is exactly what
    // three more access pairs added.
    //
    // No probe/safepoint correction is needed: `path_calls` walks the path a
    // run that throws nothing takes, and an ADR 0018 probe and a safepoint
    // poll each sit behind a status test, so neither is on it. A count over
    // the whole function would need them subtracted and would still hold the
    // landing blocks every added access brings, which is enough to read calls
    // here where this ADR claims none.
    let slope = |one: &str, four: &str| -> isize {
        path_calls(&program, four).len() as isize - path_calls(&program, one).len() as isize
    };
    let plain_extra = slope("Cell::plainOne", "Cell::plainFour");
    let hooked_extra = slope("Cell::hookedOne", "Cell::hookedFour");
    println!(
        "three more access pairs: {plain_extra} more calls on the path that \
         runs, unhooked, against {hooked_extra} hooked"
    );

    // WHICH instruction crept into a plain field store, not just how many: the
    // emitted calls are all indirect (`call *%rax`), so the asm cannot name the
    // helper and a bare count costs whoever reads this a release build to find
    // out. The IR slope can name it, and that is the question a failure asks.
    let kinds = |name: &str| {
        let mut seen: std::collections::BTreeMap<String, isize> = Default::default();
        for inst in program
            .functions
            .iter()
            .find(|f| f.name == name)
            .expect("the fixture declares it")
            .blocks
            .iter()
            .flat_map(|b| b.insts.iter())
        {
            let rendered = format!("{:?}", inst.kind);
            let variant = rendered
                .split_once(['(', '{', ' '])
                .map_or(rendered.as_str(), |(head, _)| head)
                .to_owned();
            *seen.entry(variant).or_default() += 1;
        }
        seen
    };
    let (one, four) = (kinds("Cell::plainOne"), kinds("Cell::plainFour"));
    let plain_slope: Vec<String> = four
        .iter()
        .filter_map(|(variant, count)| {
            let grew = count - one.get(variant).copied().unwrap_or(0);
            (grew != 0).then(|| format!("{variant} +{grew}"))
        })
        .collect();
    assert_eq!(
        plain_extra, 0,
        "three more unhooked property accesses now emit {plain_extra} call(s) \
         on the path a run that throws nothing takes. ADR 0014 § 4 makes an \
         access on an observer-free class a direct field load or store; \
         anything else there is a dispatch it did not ask for. \
         Three more access pairs grew the IR by: {plain_slope:?}"
    );
    // Not vacuous: the same slope over the hooked property does show the calls
    // an opt-in costs, so a measurement that found none anywhere would fail
    // here instead of passing quietly.
    assert!(
        hooked_extra > 0,
        "three more accesses through a property hook emitted no extra call at \
         all, so this guard is no longer distinguishing an opted-in access \
         from an unhooked one."
    );

    // Half two: what an access costs, taken as the slope between two bodies
    // that differ by exactly three access pairs per iteration, so that the
    // allocation, the loop and the call-out all cancel. The hooked pair is the
    // identical measurement over `$m`.
    let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
    // Argument slot 0 is the implicit receiver every lowered method carries;
    // these are static, so it is `null` — see `emit_call`'s own docs, and the
    // note above `Bench::sum` on why the raw path rather than
    // `Unit::call_static` is what a measurement at this resolution wants.
    let args = [nvs_runtime::Value::null(), nvs_runtime::Value::int(ROUNDS)];
    let mut per_pair = |one: &str, four: &str| -> f64 {
        let one = unit.raw_function(one).expect("the fixture declares it");
        let four = unit.raw_function(four).expect("the fixture declares it");
        let t_one = ns_per_op(2_000, 5, || {
            black_box(nvs_runtime::call(one, &mut ctx, &args)).expect("the loop ran");
        });
        let t_four = ns_per_op(2_000, 5, || {
            black_box(nvs_runtime::call(four, &mut ctx, &args)).expect("the loop ran");
        });
        (t_four - t_one) / (3.0 * ROUNDS as f64)
    };
    judge(
        "property read+write unhooked, against a pass-through hook pair",
        PLAIN_PER_HOOKED,
        "ADR 0014 § 4 rests on the unhooked access paying nothing for a mechanism it never \
         asked for; if this is real, that claim needs revisiting rather than this bound.",
        |yard| {
            let plain = per_pair("Cell::plainOne", "Cell::plainFour");
            let hooked = per_pair("Cell::hookedOne", "Cell::hookedFour");
            Reading {
                value: plain / hooked,
                detail: format!(
                    " ({} unhooked vs {} hooked)",
                    yard.time(plain),
                    yard.time(hooked)
                ),
            }
        },
    );
}

/// One `alloc`/`dealloc` round trip through whatever allocator this process
/// registered — `nvs_runtime`'s own in an optimized build.
#[expect(
    unsafe_code,
    reason = "measuring an allocator means calling it; the block is freed with \
              the layout it was allocated with, and a null return is refused \
              rather than freed"
)]
fn global_round_trip(layout: Layout) {
    unsafe {
        let block = std::alloc::alloc(layout);
        assert!(!block.is_null(), "the global allocator returned null");
        std::alloc::dealloc(black_box(block), layout);
    }
}

/// The registered allocator's cache count for `layout`, read three times: after
/// `N` blocks are allocated and freed, after `N` more are allocated, and after
/// those are freed again.
#[expect(
    unsafe_code,
    reason = "measuring an allocator means calling it; every block is freed \
              with the layout it was allocated with, and a null return is \
              refused rather than freed"
)]
fn cache_counts<const N: usize>(layout: Layout) -> (u32, u32, u32) {
    let mut blocks = [std::ptr::null_mut::<u8>(); N];
    let fill = |blocks: &mut [*mut u8; N]| {
        for block in blocks.iter_mut() {
            *block = unsafe { std::alloc::alloc(layout) };
            assert!(!block.is_null(), "the global allocator returned null");
        }
    };
    let empty = |blocks: &[*mut u8; N]| {
        for &block in blocks {
            unsafe { std::alloc::dealloc(black_box(block), layout) };
        }
    };
    fill(&mut blocks);
    empty(&blocks);
    let filled = nvs_runtime::budget::pooled_blocks(layout);
    fill(&mut blocks);
    let drained = nvs_runtime::budget::pooled_blocks(layout);
    empty(&blocks);
    let refilled = nvs_runtime::budget::pooled_blocks(layout);
    (filled, drained, refilled)
}

/// The same round trip, taken straight to the platform heap.
#[expect(
    unsafe_code,
    reason = "as `global_round_trip`, against `System` rather than whatever is \
              registered"
)]
fn platform_round_trip(layout: Layout) {
    unsafe {
        let block = System.alloc(layout);
        assert!(!block.is_null(), "`System` returned null");
        System.dealloc(black_box(block), layout);
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn an_allocation_round_trip_stays_in_the_pooled_cost_class() {
    let _quiet = serialised();

    // What this guard holds is that Novis serves a small allocation from its
    // own cache, and the failure this file's preamble names is the pooling
    // allocator silently not being registered. That is proved by counting: the
    // registered allocator is handed `BLOCKS` requests, and the cache it keeps
    // for this size has to give up exactly that many blocks and take them all
    // back when they are freed. A count holds on any machine however loaded,
    // and it cannot be met by an allocator that sends the request to the
    // platform heap, whose cache count never moves.
    //
    // The timing beside it is a sanity bound, not the proof. A ratio against
    // the platform heap measures two allocators at once, and how close they
    // come is a property of the platform: every round trip here also keeps
    // `nvs_runtime::budget`'s per-thread counters, each its own thread-local
    // cell, and on macOS each access to one is a call through the thread-local
    // descriptor where Linux and Windows read one register. The soft bound is
    // the one line no platform moves: a round trip through the cache costs
    // less than the platform heap it sits in front of, or the cache is
    // spending time and saving none. The hard bound is a cache twice the
    // heap's cost, just past that line, because ten times the heap would hide
    // a cache that had become pure overhead.
    const BLOCKS: u32 = 16;
    const POOLED_PER_PLATFORM: Bound = Bound::ratio_under(1.0, 2.0);

    // 32 bytes with 8-byte alignment: `NvsStr`'s header plus a short string,
    // and squarely inside the second size class.
    let layout = Layout::from_size_align(32, 8).expect("a valid layout");

    let (filled, drained, refilled) = cache_counts::<{ BLOCKS as usize }>(layout);
    println!(
        "32-byte allocation cache: {filled} blocks after {BLOCKS} frees, {drained} after \
         {BLOCKS} allocations, {refilled} after {BLOCKS} frees again"
    );
    assert!(
        filled.checked_sub(drained) == Some(BLOCKS) && refilled == filled,
        "the registered allocator's cache for a 32-byte request held {filled} blocks after \
         {BLOCKS} frees, {drained} after {BLOCKS} allocations and {refilled} after {BLOCKS} \
         frees again; a cache that serves every one of them falls by exactly {BLOCKS} and \
         comes back. Either `nvs_runtime::alloc` is no longer the `#[global_allocator]` for \
         this build, or its cache is no longer serving a request this size; \
         docs/perf/userland-gap.md § A is the measurement that made Novis own its allocator."
    );

    judge(
        "32-byte allocation round trip through the registered allocator, against the platform \
         heap",
        POOLED_PER_PLATFORM,
        "The cache is no faster than the heap it fronts. The cache count above says it is \
         serving the request, so what grew is the work around it: `nvs_runtime::budget`'s \
         counters or the free list itself.",
        |yard| {
            let pooled = ns_per_op(500_000, 5, || global_round_trip(layout));
            let platform = ns_per_op(500_000, 5, || platform_round_trip(layout));
            Reading {
                value: pooled / platform,
                detail: format!(
                    " ({} pooled vs {} platform)",
                    yard.time(pooled),
                    yard.time(platform)
                ),
            }
        },
    );
}

/// One allocation's and one release's worth of `nvs_runtime::budget`'s balance
/// update, with no allocation behind it: the live balance, the high-water mark
/// and the ceiling check, exactly as the registered allocator runs them.
fn balance_round_trip(bytes: isize) {
    nvs_runtime::budget::carry(black_box(bytes));
    nvs_runtime::budget::carry(black_box(-bytes));
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn the_memory_mark_stays_small_beside_the_allocation_it_fronts() {
    let _quiet = serialised();

    // `nvs_runtime::budget`'s module doc argues that keeping the high-water mark
    // on every allocation is a few instructions in front of an allocation that
    // costs far more. This measures the whole balance update the mark is part
    // of — the live balance, the mark's compare and the ceiling check, on the
    // allocation and on the release — against a round trip to the platform
    // heap, which is an allocation with no counting in front of it. The mark is
    // inside the measured figure, so a bound on the whole is a bound on the
    // mark. The soft bound is a quarter: the update is meant to be a small part
    // of what it fronts. The hard bound is the heap's own cost, because an
    // update as dear as the allocation is no longer "a few instructions" on any
    // platform, and the argument in that module doc is reopened.
    const UPDATE_PER_PLATFORM: Bound = Bound::ratio_under(0.25, 1.0);

    let layout = Layout::from_size_align(32, 8).expect("a valid layout");
    let bytes = isize::try_from(layout.size()).expect("a 32-byte layout fits an isize");

    judge(
        "memory balance and high-water mark update, against a platform heap round trip",
        UPDATE_PER_PLATFORM,
        "Keeping the memory mark and balance now costs as much as the allocation it fronts. \
         `nvs_runtime::budget`'s § What it spends argues it is a few instructions, and that \
         argument is reopened.",
        |yard| {
            let update = ns_per_op(500_000, 5, || balance_round_trip(bytes));
            let platform = ns_per_op(500_000, 5, || platform_round_trip(layout));
            Reading {
                value: update / platform,
                detail: format!(
                    " ({} update vs {} platform)",
                    yard.time(update),
                    yard.time(platform)
                ),
            }
        },
    );
}
