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

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn an_all_bits_off_debug_probe_stays_in_the_safepoint_cost_class() {
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
         ({sites} sites, {t_probed:.1} ns vs {t_plain:.1} ns)"
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
fn a_probe_that_is_switched_on_mid_flight_actually_fires() {
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

// ---------------------------------------------------------------------------
// Isolation boundaries
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn an_os_process_costs_orders_of_magnitude_more_than_a_task() {
    // ADR 0006 gives MWL an in-process script isolate because PHP's only way to
    // run a script under its own heap, globals and limits is another process.
    // The whole argument is this ratio, so it is measured rather than asserted.
    //
    // Both sides are floors: the process side is the cheapest do-nothing image
    // the platform can start (a real child would also boot an interpreter — PHP
    // 8.5.8 on this machine takes 35.9 ms to start and exit, 6x the floor), and
    // the task side is a bare coroutine (an isolate also builds an arena and a
    // set of globals). Measured on x86_64-pc-windows-msvc: 5.95 ms vs 4.29 us,
    // a ratio of ~1390x. `CreateProcess` is dearer than `fork`+`exec`, so a
    // Linux runner will report a smaller ratio; the 20x guard is set low enough
    // to hold everywhere and fires only on a change of kind — a task acquiring a
    // syscall, or committing its stack eagerly.
    const MIN_RATIO: f64 = 20.0;

    let task_ns = ns_per_op(50_000, 5, || {
        let run = mwl_abi_probe::in_coroutine(Ctx::new(), |_ctx| black_box(7i64));
        black_box(run.value);
    });

    // A spawn is milliseconds, so batches of one, and the minimum across them.
    let mut best_process = Duration::MAX;
    mwl_abi_probe::process::spawn_noop(); // warm the image cache
    for _ in 0..25 {
        let start = Instant::now();
        mwl_abi_probe::process::spawn_noop();
        best_process = best_process.min(start.elapsed());
    }
    let process_ns = best_process.as_secs_f64() * 1e9;

    let ratio = process_ns / task_ns;
    println!(
        "isolation boundary: os process {:.0} us vs task {task_ns:.2} us, ratio {ratio:.0}x",
        process_ns / 1000.0,
        task_ns = task_ns / 1000.0
    );

    assert!(
        ratio > MIN_RATIO,
        "an OS process now costs only {ratio:.1}x a task ({process_ns:.0} ns vs {task_ns:.0} ns), \
         under the {MIN_RATIO}x guard. ADR 0006 justifies in-process script isolates on that gap; \
         if the gap has really closed, the ADR needs revisiting."
    );
}

// ---------------------------------------------------------------------------
// ADR 0007's claim, tested rather than asserted
// ---------------------------------------------------------------------------
//
// "Mandatory types pay for themselves on the request path" is a claim about
// what the *compiler emits*, so both guards below compile the frozen
// `examples/arith.mwl` fixture through the real pipeline rather than
// approximating it with hand-built IR. The first is structural and the honest
// form of the claim; the second is a timing, and self-relative per ADR 0026.

/// Compiles `examples/arith.mwl` and returns its lowered program alongside the
/// compiled unit.
fn compile_arith() -> (mwl_ir::Program, mwl_codegen::Unit) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/arith.mwl");
    let mut map = mwl_diagnostics::SourceMap::new();
    let id = map
        .load(std::path::Path::new(path))
        .expect("the frozen acceptance fixture is readable");
    let src = map.file(id);

    let mut diags = mwl_diagnostics::Diagnostics::new();
    let stmts = mwl_syntax::parse_file(src, &mut diags);
    let module = mwl_hir::resolve_file(&stmts, src, &mut diags);
    let mut interner = mwl_types::TypeInterner::new();
    let mut exprs = mwl_types::ExprTypeTable::new();
    mwl_types::check_program(&stmts, src, &module, &mut interner, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "the fixture stopped type-checking");

    let program = mwl_ir::lower::lower_file("<script>", &stmts, src, &exprs, &interner);
    let unit = mwl_codegen::compile(&program).expect("the fixture compiles");
    (program, unit)
}

/// The lowered `Bench::sum`, whose body is the fixture's `while` loop.
fn bench_sum(program: &mwl_ir::Program) -> &mwl_ir::Function {
    program
        .functions
        .iter()
        .find(|f| f.name == "Bench::sum")
        .expect("the fixture declares Bench::sum")
}

#[test]
fn a_typed_arithmetic_loop_contains_no_call() {
    use mwl_ir::ir::InstKind;

    let (program, _unit) = compile_arith();
    let sum = bench_sum(&program);

    // Half one: the loop's arithmetic reached no helper and no MWL function.
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
    // safepoint poll or an ADR 0018 probe — both out-of-line, both emitted
    // unconditionally, and both guarded for cost separately above. Tying the
    // machine-code count to the IR site count is what makes half one a claim
    // about the emitted code rather than only about the IR.
    let sites = insts()
        .filter(|i| matches!(i.kind, InstKind::Safepoint | InstKind::StmtMarker(_)))
        .count();
    // Scanned line by line rather than by splitting on the section marker:
    // Cranelift renders a two-way branch as `jnz label3; j label2`, so a
    // `"; "` split would cut the section short at the first branch.
    let asm = mwl_codegen::disassemble(&program).expect("the fixture compiles");
    let mut in_section = false;
    let mut emitted = 0;
    for line in asm.lines() {
        if let Some(name) = line.strip_prefix("; ") {
            in_section = name == "Bench::sum";
        } else if in_section && line.trim_start().starts_with("call ") {
            emitted += 1;
        }
    }
    println!("Bench::sum: {sites} probe/safepoint sites, {emitted} emitted calls");

    assert_eq!(
        emitted, sites,
        "Bench::sum emits {emitted} call(s) for {sites} probe/safepoint site(s). \
         Every call in a typed arithmetic loop should be one of those two \
         out-of-line slow paths and nothing else."
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "baselines are release-mode figures")]
fn a_typed_arithmetic_loop_stays_in_the_native_cost_class() {
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
    let sum = unit
        .function("Bench::sum")
        .expect("the fixture declares Bench::sum");
    let mut ctx = mwl_runtime::Ctx::new(mwl_runtime::OutputSink::Sink);
    // Argument slot 0 is the implicit receiver every lowered method carries;
    // `Bench::sum` is static, so it is `null` — see `emit_call`'s own docs.
    let args = [
        mwl_runtime::Value::null(),
        mwl_runtime::Value::int(ITERATIONS),
    ];
    let per_call = ns_per_op(2_000, 5, || {
        black_box(mwl_runtime::call(sum, &mut ctx, &args)).expect("the loop ran");
    });
    let per_iteration = per_call / ITERATIONS as f64;

    let ratio = per_iteration / per_frame;
    println!(
        "typed arithmetic loop: {per_iteration:.2} ns/iteration against \
         {per_frame:.2} ns/frame, ratio {ratio:.1}x"
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
