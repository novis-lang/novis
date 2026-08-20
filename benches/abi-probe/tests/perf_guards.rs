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
//! cargo test --release -p mwl-abi-probe --test perf_guards
//! ```

use std::hint::black_box;
use std::time::{Duration, Instant};

use mwl_abi_probe::{Ctx, Helper, Probe, Value, call};

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
        "checked-return frame: {per_frame:.2} ns (2 frames {t_shallow:.1} ns, 18 frames {t_deep:.1} ns)"
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
    // ADR 0002 claims a throw costs roughly what a return does, which is what
    // lets PHP code that uses exceptions for control flow keep working. A large
    // ratio here means the error path has acquired real work — when this probe
    // was written the message was a `String`, and that allocation alone made a
    // throw 2.8x a return. See `Ctx::pending`.
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
    println!("throw/return ratio at depth 8: {ratio:.2} ({thrown:.1} ns vs {ok:.1} ns)");

    assert!(
        ratio < MAX_RATIO,
        "a throw now costs {ratio:.1}x a normal return ({thrown:.1} ns vs {ok:.1} ns), \
         over the {MAX_RATIO}x guard"
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_coroutine_round_trip_stays_cheap() {
    // Baseline: ~25 ns per suspend/resume through JIT frames (ADR 0003 preamble
    // / docs/adr/README.md). This is the price of "no async colouring".
    const MAX_NS: f64 = 400.0;

    let mut probe = Probe::new();
    let chain = probe.compile_chain(2, Helper::Suspend);

    // One coroutine per batch, suspending `iters` times inside it, so creation
    // cost is amortised to nothing and the figure is the round trip itself.
    let measure = |iters: u64| -> Duration {
        let start = Instant::now();
        let run = mwl_abi_probe::in_coroutine(Ctx::new(), move |ctx| {
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
    println!("coroutine suspend/resume through 2 JIT frames: {per_trip:.1} ns");

    assert!(
        per_trip < MAX_NS,
        "a suspend/resume round trip now costs {per_trip:.1} ns, over the {MAX_NS} ns guard"
    );
}

// ---------------------------------------------------------------------------
// Extension sandbox
// ---------------------------------------------------------------------------

#[cfg(feature = "wasm-probe")]
mod wasm_guards {
    use super::{Instant, black_box, ns_per_op};
    use mwl_abi_probe::wasm::WasmProbe;

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
        println!("wasm host->guest call: {ns:.1} ns");

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
        println!("wasm pooled instantiate + 1 call: {best:.2} us");

        assert!(
            best < MAX_US,
            "per-request instantiation now costs {best:.2} us, over the {MAX_US} us guard. \
             Check that the pooling allocator is still in use."
        );
    }
}
