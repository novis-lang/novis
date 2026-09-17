//! Order-of-magnitude regression guards on the costs the ADRs quote.
//!
//! The criterion benches in `benches/` track these numbers precisely but cannot
//! fail a build. These tests can, so their thresholds are deliberately loose —
//! roughly 10× the measured baseline. They are not here to detect a 20%
//! regression; they are here to catch the kind of change that turns 0.85 ns into
//! 500 ns, such as a dependency starting to allocate or syscall per call, or the
//! pooling allocator silently not being used.
//!
//! Every test is skipped unless built with optimisations, because the baselines
//! are release-mode figures. Run them with:
//!
//! ```text
//! cargo test --release -p nvs-abi-probe --test perf_guards
//! ```

#![expect(
    clippy::print_stdout,
    reason = "the measured figure is this guard's output. A threshold test that passed silently \
              would report only that a cost is under 10x its baseline, which is the one thing a \
              reader already knows when it is green; the printed nanoseconds are what makes a \
              CI log answer how far under, and `docs/perf/` quotes them"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard, PoisonError};
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

/// How far a measurement sits from the threshold it must stay under, as the
/// phrase every guard with a numeric bound prints beside its own figure.
///
/// **This changes no threshold and fails nothing.** It exists because a guard
/// that prints `2.07 ns` against a ceiling of `15.0` reads exactly like a guard
/// with no room at all, and the module docs above say the ceilings are
/// deliberately ~10x the baseline — so the number that says whether that is
/// still true is the *ratio*. Logs of it from several platforms are what a
/// later pass sets a tighter ceiling from; a ceiling tightened from one
/// developer's box is a flaky gate, not a stricter one.
fn under(measured: f64, ceiling: f64) -> String {
    format!(" [ceiling {ceiling}, {:.1}x headroom]", ceiling / measured)
}

/// [`under`]'s twin for a guard whose bound is a *floor* — a ratio that must
/// stay large rather than small.
fn over(measured: f64, floor: f64) -> String {
    format!(" [floor {floor}, {:.1}x headroom]", measured / floor)
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

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_checked_return_frame_stays_cheap() {
    let _quiet = serialised();

    // Baseline: ~0.85 ns per frame (ADR 0002). Derived from the slope between
    // two depths so that harness call-out overhead cancels out.
    const MAX_NS_PER_FRAME: f64 = 15.0;

    let mut probe = Probe::new();
    let shallow = probe.compile_chain(2, Helper::Double);
    let deep = probe.compile_chain(18, Helper::Double);
    let mut ctx = Ctx::new();
    let arg = Value::int(3);

    let t_shallow = ns_per_op(200_000, 5, || {
        black_box(call(shallow, &mut ctx, arg));
    });
    let t_deep = ns_per_op(200_000, 5, || {
        black_box(call(deep, &mut ctx, arg));
    });

    let per_frame = (t_deep - t_shallow) / 16.0;
    println!(
        "checked-return frame: {per_frame:.2} ns (2 frames {t_shallow:.1} ns, 18 frames {t_deep:.1} ns){}",
        under(per_frame, MAX_NS_PER_FRAME)
    );

    assert!(
        per_frame < MAX_NS_PER_FRAME,
        "a frame now costs {per_frame:.2} ns, over the {MAX_NS_PER_FRAME} ns guard. \
         ADR 0002 justifies the checked-return convention on a ~0.85 ns baseline; \
         if this is a real regression that argument needs revisiting."
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
    const MAX_RATIO: f64 = 2.0;

    let mut probe = Probe::new();
    let chain = probe.compile_chain(8, Helper::Double);
    let mut ctx = Ctx::new();

    let ok = ns_per_op(200_000, 5, || {
        black_box(call(chain, &mut ctx, Value::int(3)));
    });
    // 42 makes the innermost helper throw, which then climbs all 8 frames.
    let thrown = ns_per_op(200_000, 5, || {
        black_box(call(chain, &mut ctx, Value::int(42)));
        ctx.pending = None;
    });

    let ratio = thrown / ok;
    println!(
        "throw/return ratio at depth 8: {ratio:.2} ({thrown:.1} ns vs {ok:.1} ns){}",
        under(ratio, MAX_RATIO)
    );

    assert!(
        ratio < MAX_RATIO,
        "a throw now costs {ratio:.1}x a normal return ({thrown:.1} ns vs {ok:.1} ns), \
         over the {MAX_RATIO}x guard"
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
    // answer this ceiling rules out is the one the decision behind
    // `nvs_runtime`'s gap 6 priced and rejected: walking a stack, or asking the
    // OS for a backtrace, lands two orders of magnitude above it.
    //
    // Measured as the difference between raising with a site and raising with
    // the zero word, so the object allocation both arms pay cancels.
    const MAX_NS: f64 = 500.0;

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
    let bare = ns_per_op(100_000, 5, || raise(std::ptr::null()));
    let sited = ns_per_op(100_000, 5, || raise(blob.as_ptr()));

    let rendering = sited - bare;
    println!(
        "a raise's own frame label: {rendering:.1} ns ({sited:.1} ns sited vs {bare:.1} ns bare){}",
        under(rendering, MAX_NS)
    );

    assert!(
        rendering < MAX_NS,
        "rendering a raise's own frame now costs {rendering:.1} ns, over the {MAX_NS} ns guard — \
         a throw is meant to pay for one label, not for a walk of anything"
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_coroutine_round_trip_stays_cheap() {
    let _quiet = serialised();

    // Baseline: ~25 ns per suspend/resume through JIT frames (ADR 0003 preamble
    // / docs/adr/README.md). This is the price of "no async colouring".
    const MAX_NS: f64 = 400.0;

    let mut probe = Probe::new();
    let chain = probe.compile_chain(2, Helper::Suspend);

    // One coroutine per batch, suspending `iters` times inside it, so creation
    // cost is amortised to nothing and the figure is the round trip itself.
    let measure = |iters: u64| -> Duration {
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

    measure(10_000); // warm up
    let iters = 200_000u64;
    let mut best = Duration::MAX;
    for _ in 0..5 {
        best = best.min(measure(iters));
    }
    let per_trip = best.as_secs_f64() * 1e9 / iters as f64;
    println!(
        "coroutine suspend/resume through 2 JIT frames: {per_trip:.1} ns{}",
        under(per_trip, MAX_NS)
    );

    assert!(
        per_trip < MAX_NS,
        "a suspend/resume round trip now costs {per_trip:.1} ns, over the {MAX_NS} ns guard"
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
    // The threshold is stated against `a_checked_return_frame_stays_cheap`'s
    // own guard: a probe site does strictly less than a frame (no call, no
    // 16-byte result copy, no status check), so anything at or above a
    // frame's guarded cost means the check has acquired real work.
    const MAX_NS_PER_PROBE: f64 = 5.0;
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

    let t_plain = ns_per_op(200_000, 5, || {
        black_box(call(plain, &mut ctx, arg));
    });
    let t_probed = ns_per_op(200_000, 5, || {
        black_box(call(probed, &mut ctx, arg));
    });

    // Nothing set a bit, so no site may have reached its slow path.
    assert_eq!(
        ctx.probe_hits, 0,
        "a probe fired with every debug flag off, which would make the \
         measurement meaningless"
    );

    let sites = (DEPTH * STMTS_PER_FRAME) as f64;
    let per_probe = (t_probed - t_plain) / sites;
    println!(
        "all-bits-off debug probe: {per_probe:.3} ns per site \
         ({sites} sites, {t_probed:.1} ns vs {t_plain:.1} ns){}",
        under(per_probe, MAX_NS_PER_PROBE)
    );

    assert!(
        per_probe < MAX_NS_PER_PROBE,
        "an all-bits-off ADR 0018 probe site now costs {per_probe:.3} ns, over \
         the {MAX_NS_PER_PROBE} ns guard. That check is emitted at every \
         statement boundary of every compiled unit; if this is real, ADR 0018 \
         § Revisiting says to coarsen the probe site, not to raise this bound."
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
    const DEPTH: usize = 8;
    const STMTS_PER_FRAME: usize = 16;
    const ITEMS: usize = nvs_runtime::DEADLINE_POLL_BATCH * 64;

    let mut probe = Probe::new();
    let plain_chain = probe.compile_probe_chain(DEPTH, Helper::Double, STMTS_PER_FRAME, false);
    let checked_chain = probe.compile_probe_chain(DEPTH, Helper::Double, STMTS_PER_FRAME, true);
    let mut probe_ctx = Ctx::new();
    let arg = Value::int(3);

    let t_plain_chain = ns_per_op(200_000, 5, || {
        black_box(call(plain_chain, &mut probe_ctx, arg));
    });
    let t_checked_chain = ns_per_op(200_000, 5, || {
        black_box(call(checked_chain, &mut probe_ctx, arg));
    });
    let per_check = (t_checked_chain - t_plain_chain) / (DEPTH * STMTS_PER_FRAME) as f64;

    // The two loops differ by exactly what the combinator adds: a register
    // countdown and a branch predicted not taken, plus one flag load per batch.
    // `black_box` on the accumulator keeps both scalar, so neither side wins by
    // being vectorized rather than by being cheaper.
    let mut rt = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
    let mut sum = 0usize;

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
    black_box(sum);

    let per_poll = (t_polled_loop - t_plain_loop) / ITEMS as f64;
    println!(
        "amortised deadline poll: {per_poll:.4} ns per iteration \
         ({ITEMS} iterations, batch {}, {t_polled_loop:.0} ns vs {t_plain_loop:.0} ns) \
         against a hot-line check at {per_check:.4} ns{}",
        nvs_runtime::DEADLINE_POLL_BATCH,
        under(per_poll.max(f64::MIN_POSITIVE), per_check)
    );

    assert!(
        per_poll < per_check,
        "an amortised deadline poll now costs {per_poll:.4} ns per iteration, \
         over the {per_check:.4} ns a hot-line check costs. That is ADR 0106 \
         § 5's own bound, and the fix is the batch in \
         `nvs_runtime::DEADLINE_POLL_BATCH` or the shape of `bounded_loop`'s \
         countdown — not this comparison, which is what the ADR says."
    );
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
    use super::{Instant, black_box, ns_per_op, under};
    use nvs_abi_probe::wasm::WasmProbe;

    const NO_DEADLINE: u64 = u64::MAX;

    #[test]
    #[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
    fn a_host_to_guest_call_stays_cheap() {
        // Baseline: 11.5 ns (ADR 0003). The whole tier argument rests on this
        // being roughly 10 ns rather than roughly 1 µs.
        const MAX_NS: f64 = 200.0;

        let probe = WasmProbe::new().expect("wasm probe");
        let (mut store, instance) = probe.instantiate(0, NO_DEADLINE).expect("instantiate");
        let add = instance
            .get_typed_func::<(i64, i64), i64>(&mut store, "add")
            .expect("add export");

        let ns = ns_per_op(200_000, 5, || {
            black_box(add.call(&mut store, (1, 2)).expect("call"));
        });
        println!("wasm host->guest call: {ns:.1} ns{}", under(ns, MAX_NS));

        assert!(
            ns < MAX_NS,
            "a host->guest call now costs {ns:.1} ns, over the {MAX_NS} ns guard. \
             ADR 0003's tier argument assumes ~11.5 ns."
        );
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
    fn per_request_instantiation_stays_affordable() {
        // Baseline: 7.57 µs pooled (ADR 0003). This is what makes a fresh
        // instance per request — and therefore per-request isolation for
        // extensions — affordable. A large regression here most likely means the
        // pooling allocator is no longer being used.
        const MAX_US: f64 = 100.0;

        let probe = WasmProbe::pooled(1000).expect("pooled probe");

        let run = || {
            let (mut store, instance) = probe.instantiate(0, NO_DEADLINE).expect("instantiate");
            let add = instance
                .get_typed_func::<(i64, i64), i64>(&mut store, "add")
                .expect("add export");
            black_box(add.call(&mut store, (1, 2)).expect("call"));
        };

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
            best = best.min(start.elapsed().as_secs_f64() * 1e6 / iters as f64);
        }
        println!(
            "wasm pooled instantiate + 1 call: {best:.2} us{}",
            under(best, MAX_US)
        );

        assert!(
            best < MAX_US,
            "per-request instantiation now costs {best:.2} us, over the {MAX_US} us guard. \
             Check that the pooling allocator is still in use."
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
    // builds an arena and a set of globals). Measured on
    // x86_64-pc-windows-msvc: 5.95 ms vs 4.29 us, a ratio of ~1390x.
    // `CreateProcess` is dearer than `fork`+`exec`, so a
    // Linux runner will report a smaller ratio; the 20x guard is set low enough
    // to hold everywhere and fires only on a change of kind — a task acquiring a
    // syscall, or committing its stack eagerly.
    const MIN_RATIO: f64 = 20.0;

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

    let ratio = process_ns / task_ns;
    println!(
        "isolation boundary: os process {:.0} us vs task {task_ns:.2} us, ratio {ratio:.0}x{}",
        process_ns / 1000.0,
        over(ratio, MIN_RATIO),
        task_ns = task_ns / 1000.0
    );

    assert!(
        ratio > MIN_RATIO,
        "an OS process now costs only {ratio:.1}x a task ({process_ns:.0} ns vs {task_ns:.0} ns), \
         under the {MIN_RATIO}x guard. ADR 0006 justifies in-process script isolates on that gap; \
         if the gap has really closed, the ADR needs revisiting."
    );
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
    // The ceiling is M5's own number rather than a multiple of the baseline: at
    // ten microseconds the guard fails exactly when "single-digit microseconds"
    // stops being true, which is the claim worth a red build. Measured on
    // x86_64-pc-windows-msvc: 0.44 us, so there is ~23x of headroom — more than
    // the module doc's usual ~10x, and deliberately, because a ceiling tightened
    // from one developer's box is a flaky gate rather than a stricter one. The
    // measured figure is printed either way, so drift inside the class is
    // visible without being fatal.
    const MAX_US: f64 = 10.0;
    const ITERS: u64 = 20_000;

    // Each batch builds its own scheduler and task and warms inside them; the
    // shared module owns why the loop cannot simply live here.
    let mut best = f64::MAX;
    for _ in 0..5 {
        let took = isolate::spawn_to_result_batch(ITERS);
        best = best.min(took.as_secs_f64() * 1e9 / ITERS as f64);
    }
    let us = best / 1000.0;
    println!("isolate spawn to result: {us:.2} us{}", under(us, MAX_US));

    assert!(
        us < MAX_US,
        "spawn-to-result now costs {us:.2} us, over the {MAX_US} us guard. ADR 0006 replaces a \
         child process with this boundary and M5's acceptance puts it in the single-digit \
         microseconds; at this figure the child is doing something a child should not, or the \
         boundary has acquired a syscall."
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
    // The bound is a ratio between two figures this one run takes, so it needs
    // no baseline and holds on any machine: a core that blocked on a child
    // would reach the neighbour only after the last of them and report about
    // 1x, where one that hands itself back reaches it in the microseconds
    // before the first child has started. Measured on x86_64-pc-windows-msvc:
    // a neighbour served 0.23 ms into a 10.7 ms run, a ratio of ~46x, where the
    // same four children run one after another take 24.9 ms.
    const MIN_RATIO: f64 = 10.0;
    const CHILDREN: usize = 4;

    let _installed =
        nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
    let mut sched = nvs_host::Scheduler::new();

    // The serial figure the concurrent run is printed against: one child, start
    // to reaped, off any core. It is printed and not asserted, because how far
    // the children overlap is the blocking pool's bound and the host's core
    // count, while what this guard fails on is the core being held at all.
    nvs_abi_probe::process::spawn_noop();
    let mut serial = Duration::MAX;
    for _ in 0..3 {
        let one = Instant::now();
        nvs_abi_probe::process::spawn_noop();
        serial = serial.min(one.elapsed());
    }

    let started = Instant::now();
    for _ in 0..CHILDREN {
        let ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        sched.spawn(ctx, nvs_runtime::TaskRoot::Worker, move |_ctx| {
            let status = nvs_host::blocking::run(|| {
                nvs_abi_probe::process::noop_command()
                    .spawn()
                    .expect("the host must be able to start a do-nothing process")
                    .wait()
                    .expect("a started child is one to be reaped")
            });
            assert!(status.success(), "a do-nothing process must exit cleanly");
        });
    }
    let served: Rc<Cell<Option<Duration>>> = Rc::new(Cell::new(None));
    let neighbour = Rc::clone(&served);
    sched.spawn(
        nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink),
        nvs_runtime::TaskRoot::Worker,
        move |_ctx| neighbour.set(Some(started.elapsed())),
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
    let ratio = total.as_secs_f64() / serve.as_secs_f64().max(1e-9);
    println!(
        "{CHILDREN} concurrent spawns: neighbour served at {:.2} ms of {:.2} ms, \
         serially {:.2} ms, ratio {ratio:.0}x{}",
        serve.as_secs_f64() * 1e3,
        total.as_secs_f64() * 1e3,
        serial.as_secs_f64() * 1e3 * CHILDREN as f64,
        over(ratio, MIN_RATIO)
    );

    assert!(
        ratio > MIN_RATIO,
        "a neighbour task waited {:.2} ms of the {:.2} ms {CHILDREN} children took, a ratio of \
         {ratio:.1}x and under the {MIN_RATIO}x guard. A child is waited for off the core \
         through `nvs_host::blocking::run`; at this ratio the wait is holding the core and \
         every other request on it is behind a process.",
        serve.as_secs_f64() * 1e3,
        total.as_secs_f64() * 1e3
    );
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
    // The floor is the margin this test's name promises, and it is well under
    // the ideal 4x deliberately. A ratio is bounded by whatever else the machine
    // is running, and the failure worth a red build is a fan-out that stopped
    // fanning out — a refused placement, a picker handing every child to one
    // core, a join that serialises the children — all of which land at or under
    // 1x. Halving the ideal leaves room for a busy machine and still cannot be
    // reached without three of the four children running somewhere else.
    const MIN_SPEEDUP: f64 = 2.0;
    const ITERS: u64 = 5;
    const ROUNDS: usize = 5;

    let cpus = nvs_host::cpus().len();
    if cpus < isolate::WIDTH {
        println!(
            "fan-out across worker cores: skipped, {cpus} CPU(s) is fewer than the \
             {} a fan-out places",
            isolate::WIDTH
        );
        return;
    }

    // Interleaved, so that a machine warming up or throttling mid-test moves
    // both halves rather than one; the minimum of each, for the reason
    // `ns_per_op` above gives.
    let mut placed = f64::MAX;
    let mut one_core = f64::MAX;
    for _ in 0..ROUNDS {
        placed = placed.min(isolate::worker_fan_out_batch(ITERS).as_secs_f64());
        one_core = one_core.min(isolate::one_core_batch(ITERS).as_secs_f64());
    }
    let speedup = one_core / placed;

    println!(
        "fan-out across worker cores: {} children in {:.1} ms placed vs {:.1} ms on one core, \
         {speedup:.2}x{}",
        isolate::WIDTH,
        placed * 1e3 / ITERS as f64,
        one_core * 1e3 / ITERS as f64,
        over(speedup, MIN_SPEEDUP)
    );

    assert!(
        speedup > MIN_SPEEDUP,
        "{} CPU-bound children placed on worker cores now run only {speedup:.2}x faster than the \
         same children one after another, under the {MIN_SPEEDUP}x guard. M5's acceptance claims \
         near-linear speedup across cores for exactly this shape of work; at this ratio the \
         children are sharing a core rather than spreading over them.",
        isolate::WIDTH
    );
}

// ---------------------------------------------------------------------------
// `rule:routing/path-grammar`'s trie, priced against the walk standing in for it
// ---------------------------------------------------------------------------

/// One row of a route table costs a small enough slice of a request that
/// replacing the walk with a trie would buy nothing measurable.
///
/// `nvs_runtime::routes` compares against every row of the right verb, so its
/// cost grows with the table rather than with the path. What decides whether
/// that matters is not the per-row figure on its own but the figure times the
/// rows an application declares, against what serving a request costs —
/// `benches/serve-proxied.json`'s `nvs-serve-direct` arm, which is where the
/// denominator lives and the only place it is written down.
///
/// The slope between two table sizes is what is measured, so the split of the
/// path and the capture the answer carries — both per request and neither per
/// row — cancel out. `benches/routing.rs` is the same measurement with
/// criterion's precision; this one can fail a build.
#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_route_table_walk_costs_a_fraction_of_the_request_it_rides_in() {
    let _quiet = serialised();

    // Roughly 10x the measured baseline, like every ceiling in this file. A
    // regression past it is a row that began to allocate or to convert before
    // it has matched, which is what would make the table's length start to
    // show in a request.
    const MAX_NS_PER_ROW: f64 = 30.0;

    let (method, path) = routes::REQUEST;
    let small = routes::table(8);
    let large = routes::table(512);

    let t_small = ns_per_op(200_000, 5, || {
        black_box(small.match_request(black_box(method), black_box(path)));
    });
    let t_large = ns_per_op(20_000, 5, || {
        black_box(large.match_request(black_box(method), black_box(path)));
    });

    let per_row = (t_large - t_small) / 504.0;
    println!(
        "route walk: {per_row:.2} ns per row (8 rows {t_small:.1} ns, 512 rows {t_large:.1} ns){}",
        under(per_row, MAX_NS_PER_ROW)
    );

    assert!(
        per_row < MAX_NS_PER_ROW,
        "a route table row now costs {per_row:.2} ns to walk past, over the {MAX_NS_PER_ROW} ns \
         guard. The linear walk is kept in place because the table's length does not show in a \
         request; if this is a real regression, that argument is the one to revisit."
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
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/arith.nvs");
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
    // Measured on x86_64-pc-windows-msvc: 1.13-1.28 ns per loop iteration
    // against 1.38-1.89 ns per checked-return frame, a ratio of 0.7-0.8x
    // across runs. An iteration does a compare, three arithmetic operations, a
    // safepoint poll and six ADR 0018 probe checks — and still costs less than
    // one cross-frame call, which is the whole of ADR 0007's claim.
    //
    // The guard is set well above that, because what it exists to catch is a
    // change of *kind*: arithmetic going through a runtime helper instead of a
    // native instruction would put four calls in each iteration and move the
    // ratio into the tens.
    const MAX_RATIO: f64 = 6.0;
    const ITERATIONS: i64 = 1_000;

    let mut probe = Probe::new();
    let shallow = probe.compile_chain(2, Helper::Double);
    let deep = probe.compile_chain(18, Helper::Double);
    let mut probe_ctx = Ctx::new();
    let arg = Value::int(3);
    let t_shallow = ns_per_op(200_000, 5, || {
        black_box(call(shallow, &mut probe_ctx, arg));
    });
    let t_deep = ns_per_op(200_000, 5, || {
        black_box(call(deep, &mut probe_ctx, arg));
    });
    let per_frame = (t_deep - t_shallow) / 16.0;

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
    let per_call = ns_per_op(2_000, 5, || {
        black_box(nvs_runtime::call(sum, &mut ctx, &args)).expect("the loop ran");
    });
    let per_iteration = per_call / ITERATIONS as f64;

    let ratio = per_iteration / per_frame;
    println!(
        "typed arithmetic loop: {per_iteration:.2} ns/iteration against \
         {per_frame:.2} ns/frame, ratio {ratio:.1}x{}",
        under(ratio, MAX_RATIO)
    );

    assert!(
        ratio < MAX_RATIO,
        "a loop iteration now costs {ratio:.1}x a checked-return frame \
         ({per_iteration:.2} ns vs {per_frame:.2} ns), over the {MAX_RATIO}x guard. \
         ADR 0007 justifies mandatory types on typed arithmetic staying in the \
         native cost class; if this is a real regression that argument needs \
         revisiting."
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
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../members/lang/types/numbers-bool-int-uint-float-decimal.nvs"
    );
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
    // mantissa, so this loop sits tens of frames up rather than under one:
    // measured on x86_64-pc-windows-msvc, 61 ns per iteration against 1.55 ns
    // per checked-return frame, a ratio of 39x steady across runs.
    //
    // The ceiling is about three times that rather than this file's usual ~10x,
    // and the tighter bound is the point. What it exists to catch is a change
    // of *kind* — a `decimal` that starts allocating, or an operator that falls
    // back to an arbitrary-precision path, against
    // `crates/nvs-runtime/src/decimal.rs`'s "nothing is allocated and nothing
    // is refcounted" — and that costs tens of nanoseconds per operator, which a
    // ceiling ten times a ratio already in the tens would sail straight past.
    // A ratio of two figures measured in the same run is also the portable
    // half of ADR 0026's finding, so it carries less machine-to-machine slack
    // than an absolute figure needs.
    const MAX_RATIO: f64 = 120.0;
    const ITERATIONS: i64 = 1_000;

    let mut probe = Probe::new();
    let shallow = probe.compile_chain(2, Helper::Double);
    let deep = probe.compile_chain(18, Helper::Double);
    let mut probe_ctx = Ctx::new();
    let arg = Value::int(3);
    let t_shallow = ns_per_op(200_000, 5, || {
        black_box(call(shallow, &mut probe_ctx, arg));
    });
    let t_deep = ns_per_op(200_000, 5, || {
        black_box(call(deep, &mut probe_ctx, arg));
    });
    let per_frame = (t_deep - t_shallow) / 16.0;

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
    let per_call = ns_per_op(2_000, 5, || {
        black_box(nvs_runtime::call(run, &mut ctx, &args)).expect("the loop ran");
    });
    let per_iteration = per_call / ITERATIONS as f64;

    let ratio = per_iteration / per_frame;
    println!(
        "typed decimal arithmetic loop: {per_iteration:.2} ns/iteration against \
         {per_frame:.2} ns/frame, ratio {ratio:.1}x{}",
        under(ratio, MAX_RATIO)
    );

    assert!(
        ratio < MAX_RATIO,
        "a decimal loop iteration now costs {ratio:.1}x a checked-return frame \
         ({per_iteration:.2} ns vs {per_frame:.2} ns), over the {MAX_RATIO}x \
         guard. ADR 0054 § *Consequences* claims a helper call per operator and \
         no allocation; at this ratio one of those two has stopped being true."
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
    // Measured on x86_64-pc-windows-msvc: ~25 ns for the in-place write
    // against ~34 us for the separating one, a ratio around 0.0007x. The
    // guard sits two orders of magnitude above that, because what it exists to
    // catch is a change of *kind* — the fast path being lost, which would put
    // the two within a small constant factor of each other.
    const MAX_RATIO: f64 = 0.05;
    const ENTRIES: i64 = 1_000;

    let mut array = nvs_runtime::NvsArray::new();
    for index in 0..ENTRIES {
        array.set(
            nvs_runtime::NvsStr::new(index.to_string().as_bytes()),
            nvs_runtime::Value::int(index),
        );
    }
    let key = nvs_runtime::NvsStr::new(b"probe");

    let in_place = ns_per_op(200_000, 5, || {
        array.set(key.clone(), nvs_runtime::Value::int(1));
    });
    assert_eq!(array.refcount(), 1, "the in-place write never separated");

    let separating = ns_per_op(300, 5, || {
        let mut aliased = array.clone();
        aliased.set(key.clone(), nvs_runtime::Value::int(1));
        black_box(aliased.count());
    });

    let ratio = in_place / separating;
    println!(
        "array write: {in_place:.0} ns in place against {separating:.0} ns \
         separating {ENTRIES} entries, ratio {ratio:.4}x{}",
        under(ratio, MAX_RATIO)
    );

    assert!(
        ratio < MAX_RATIO,
        "a write into a solely-owned array now costs {ratio:.4}x one that has \
         to separate ({in_place:.0} ns vs {separating:.0} ns), over the \
         {MAX_RATIO}x guard. ADR 0063 R3's immutable `Core` API rests on that \
         write being in place; if this is a real regression that argument \
         needs revisiting."
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
/// Each corpus with the bound its grapheme count must stay under, in UTF-8
/// validations of the same buffer.
///
/// The baselines behind the bounds: 3.0 validations for `ascii`, 24.7 for
/// `mixed`. Each bound is this file's usual order of magnitude above its own
/// baseline, which is loose enough not to track a machine and tight enough to
/// catch what actually matters — without `one_byte_per_cluster`'s fast path
/// the `ascii` leg climbs into the hundreds.
fn corpora() -> [(&'static str, String, f64); 2] {
    let repeat = |unit: &str| unit.repeat(64);
    [
        (
            "ascii",
            repeat("the quick brown fox jumps over the lazy dog, "),
            30.0,
        ),
        (
            "mixed",
            repeat(concat!(
                "the quick brown fox jumps over the lazy dog, ",
                "na\u{131}\u{308}ve cafe\u{301} re\u{301}sume\u{301}, ",
                "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} \u{1f1e6}\u{1f1f9} ",
                "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f} ",
            )),
            250.0,
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

    for (name, text, max_validations) in corpora() {
        let bytes = text.as_bytes();
        let graphemes = nvs_stdlib::granularity::Unit::Grapheme.length(&text);
        let code_points = nvs_stdlib::granularity::Unit::CodePoint.length(&text);

        // Per byte, so the three legs are comparable and the corpus size is
        // not baked into the threshold.
        let per_byte = |ns: f64| ns / bytes.len() as f64;

        let validate = per_byte(ns_per_op(2_000, 5, || {
            black_box(std::str::from_utf8(black_box(bytes)).is_ok());
        }));
        let code_point = per_byte(ns_per_op(2_000, 5, || {
            black_box(nvs_stdlib::granularity::Unit::CodePoint.length(black_box(&text)));
        }));
        let grapheme = per_byte(ns_per_op(2_000, 5, || {
            black_box(nvs_stdlib::granularity::Unit::Grapheme.length(black_box(&text)));
        }));

        let validations = grapheme / validate;
        println!(
            "{name}: {} bytes, {graphemes} graphemes, {code_points} code points — \
             validate {validate:.3} ns/byte, code point {code_point:.3} ns/byte, \
             grapheme {grapheme:.3} ns/byte; a grapheme count is {validations:.1} UTF-8 \
             validations and {:.1}x a code-point count",
            bytes.len(),
            grapheme / code_point
        );

        // Only off the fast path. On plain ASCII a grapheme count *is* the
        // byte length behind one vectorized scan, so it is legitimately in the
        // same class as a code-point count there and the comparison says
        // nothing; asserting it on both legs would be a coin flip.
        assert!(
            text.is_ascii() || grapheme >= code_point,
            "over {name}, a grapheme count now costs {grapheme:.3} ns/byte against a \
             code-point count's {code_point:.3} ns/byte. The two are meant to be distinct \
             seams over the same buffer; if segmentation has become free, \
             `nvs_stdlib::granularity` is no longer measuring what it claims to."
        );

        assert!(
            validations < max_validations,
            "over {name}, a grapheme count now costs {validations:.1} UTF-8 validations of \
             the same buffer ({grapheme:.3} ns/byte against {validate:.3} ns/byte), over \
             the {max_validations} guard. ADR 0009 § 2 makes grapheme clusters `string`'s \
             default unit on the strength of a measured figure well under that; if this is \
             a real regression, that decision needs revisiting rather than this threshold."
        );
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
    // The bound is a floor of 1.0 — the linear tier must simply be the faster
    // of the two — because that is the claim exactly, and nothing more than it
    // is safe to fix: a developer who cannot choose a tier is not being handed
    // the slower one. Measured on x86_64-pc-windows-msvc the margin is three
    // orders of magnitude, since the literal `@` gives the linear engine a
    // prefilter that rejects the whole corpus in one pass and the lookahead
    // denies the backtracker the same trick; the printed figure is where a
    // later pass would tighten this from, across several machines.
    const MIN_SPEEDUP: f64 = 1.0;
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
    let linear = per_byte("Bench::linear");
    let backtracking = per_byte("Bench::backtracking");

    let speedup = backtracking / linear;
    println!(
        "regex tiers over {bytes:.0} bytes: linear {linear:.3} ns/byte against \
         backtracking {backtracking:.3} ns/byte, {speedup:.1}x{}",
        over(speedup, MIN_SPEEDUP)
    );

    assert!(
        speedup > MIN_SPEEDUP,
        "the linear tier now runs at {speedup:.2}x the backtracking tier over the \
         same corpus ({linear:.3} ns/byte against {backtracking:.3} ns/byte), under \
         the {MIN_SPEEDUP}x floor. `rule:core-classes/regex-two-tiers` gives a \
         developer no way to ask for the second tier, which rests on the first not \
         being the slower one; if this is a real regression that argument needs \
         revisiting."
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
    // Measured on x86_64-pc-windows-msvc: 0.29 ns for the unhooked read+write
    // against 14.5 ns for the hooked one, a ratio of 0.02x, and three more
    // unhooked accesses emit 0 machine-code calls against the hooked pair's
    // 36. The guard sits this file's usual order of magnitude above that,
    // which is still far below the ~1x an unhooked access would reach the
    // moment it acquired a dispatch of its own — the change of kind this
    // exists to catch.
    const MAX_RATIO: f64 = 0.2;
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
    let plain = per_pair("Cell::plainOne", "Cell::plainFour");
    let hooked = per_pair("Cell::hookedOne", "Cell::hookedFour");

    let ratio = plain / hooked;
    println!(
        "property read+write: {plain:.2} ns unhooked against {hooked:.2} ns \
         through a pass-through hook pair, ratio {ratio:.3}x{}",
        under(ratio, MAX_RATIO)
    );

    assert!(
        ratio < MAX_RATIO,
        "a read+write of an unhooked property on an observer-free class now \
         costs {ratio:.3}x the same pair behind a hook ({plain:.2} ns vs \
         {hooked:.2} ns), over the {MAX_RATIO}x guard. ADR 0014 § 4 rests on \
         the unhooked access paying nothing for a mechanism it never asked \
         for; if this is real, that claim needs revisiting rather than this \
         threshold."
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

    // Self-relative, per ADR 0026: the bound is this machine's own platform
    // heap, measured in the same loop rather than quoted. A 32-byte round trip
    // through `System` costs tens of nanoseconds
    // (docs/perf/userland-gap.md § A) while a free-list pop and push is a
    // small multiple of a load and a store, so the real ratio is an order of
    // magnitude under this bound. What the guard holds is the cost
    // *class*: Novis either serves a small allocation from its own cache or it
    // does not, and the failure this file's preamble names — the pooling
    // allocator silently not being registered — lands exactly here, because
    // then the two sides are the same code and the ratio is 1.
    const MAX_RATIO: f64 = 0.5;

    // 32 bytes with 8-byte alignment: `NvsStr`'s header plus a short string,
    // and squarely inside the second size class.
    let layout = Layout::from_size_align(32, 8).expect("a valid layout");

    let pooled = ns_per_op(500_000, 5, || global_round_trip(layout));
    let platform = ns_per_op(500_000, 5, || platform_round_trip(layout));

    let ratio = pooled / platform;
    println!(
        "32-byte allocation round trip: {pooled:.2} ns through the registered \
         allocator against {platform:.2} ns through the platform heap, ratio \
         {ratio:.3}x{}",
        under(ratio, MAX_RATIO)
    );

    assert!(
        ratio < MAX_RATIO,
        "a 32-byte round trip now costs {ratio:.3}x the platform heap's own \
         ({pooled:.2} ns vs {platform:.2} ns), over the {MAX_RATIO}x guard. \
         Either `nvs_runtime::alloc` is no longer the `#[global_allocator]` \
         for this build, or its cache is no longer serving a request this \
         size; docs/perf/userland-gap.md § A is the measurement that made \
         Novis own its allocator."
    );
}
