//! The behavioural invariants Novis's execution model depends on.
//!
//! If any of these fail after a dependency bump, the plan for M3 (backend) or
//! M5 (concurrency) is broken and needs revisiting before more code is written
//! on top of it. See `docs/adr/0002-error-propagation.md`.

use nvs_abi_probe::{Ctx, FATAL, Helper, NvsFn, OK, Probe, THROWN, Value, call, in_coroutine};
use nvs_runtime::{TaskRoot, run_task};

// ---------------------------------------------------------------------------
// The checked-return ABI
// ---------------------------------------------------------------------------

#[test]
fn a_result_travels_up_through_many_frames() {
    let mut probe = Probe::new();
    let chain = probe.compile_chain(3, Helper::Double);
    let mut ctx = Ctx::new();

    let (status, out) = call(chain, &mut ctx, Value::int(5));

    assert_eq!(status, OK);
    // Only the innermost frame calls the helper; the outer two forward the
    // argument down and copy the 16-byte result back up.
    assert_eq!(out.as_int(), 10);
    assert_eq!(ctx.helper_calls, 1);
    assert_eq!(ctx.pending, None);
}

#[test]
fn frame_count_does_not_change_the_result() {
    let mut probe = Probe::new();
    let mut ctx = Ctx::new();
    for depth in [1, 2, 5, 16] {
        let chain = probe.compile_chain(depth, Helper::Double);
        ctx.reset();
        let (status, out) = call(chain, &mut ctx, Value::int(7));
        assert_eq!(status, OK, "depth {depth}");
        assert_eq!(out.as_int(), 14, "depth {depth}");
    }
}

#[test]
fn a_throw_propagates_through_every_frame() {
    let mut probe = Probe::new();
    let mut ctx = Ctx::new();

    for depth in [1, 3, 16] {
        let chain = probe.compile_chain(depth, Helper::Double);
        ctx.reset();
        let (status, _) = call(chain, &mut ctx, Value::int(42));

        assert_eq!(
            status, THROWN,
            "depth {depth}: status must survive the climb"
        );
        assert!(
            ctx.pending
                .as_deref()
                .unwrap_or_default()
                .contains("refused 42"),
            "depth {depth}: the pending message must reach the top intact"
        );
    }
}

#[test]
fn a_runtime_panic_is_contained_and_reported() {
    // The invariant that makes a single-process server defensible: a bug in a
    // runtime helper must kill one request, not the process. If unwind
    // containment ever regresses, this test does not fail — the test *binary*
    // dies, which is an even louder signal.
    let mut probe = Probe::new();
    let chain = probe.compile_chain(3, Helper::Double);
    let mut ctx = Ctx::new();

    let (status, _) = call(chain, &mut ctx, Value::int(99));

    assert_eq!(status, FATAL, "a panic must surface as FATAL, not unwind");
    assert!(
        ctx.pending
            .as_deref()
            .unwrap_or_default()
            .contains("internal error"),
        "the panic message must be captured, got {:?}",
        ctx.pending
    );
}

#[test]
fn the_jit_is_still_usable_after_a_contained_panic() {
    let mut probe = Probe::new();
    let chain = probe.compile_chain(2, Helper::Double);
    let mut ctx = Ctx::new();

    let (fatal, _) = call(chain, &mut ctx, Value::int(99));
    assert_eq!(fatal, FATAL);

    ctx.reset();
    let (status, out) = call(chain, &mut ctx, Value::int(4));
    assert_eq!(
        (status, out.as_int()),
        (OK, 8),
        "compiled code must survive"
    );
}

#[test]
fn a_successful_call_leaves_no_pending_error() {
    let mut probe = Probe::new();
    let chain = probe.compile_chain(4, Helper::Double);
    let mut ctx = Ctx::new();

    // A stale message from an earlier throw must not be mistaken for a new one.
    call(chain, &mut ctx, Value::int(42));
    assert!(ctx.pending.is_some());
    ctx.reset();

    let (status, _) = call(chain, &mut ctx, Value::int(1));
    assert_eq!(status, OK);
    assert_eq!(ctx.pending, None);
}

// ---------------------------------------------------------------------------
// Stackful coroutines under JIT frames
// ---------------------------------------------------------------------------

#[test]
fn a_helper_can_suspend_with_jit_frames_live_above_it() {
    let mut probe = Probe::new();
    let chain: NvsFn = probe.compile_chain(2, Helper::Suspend);

    let run = in_coroutine(Ctx::new(), move |ctx| {
        let (status, out) = call(chain, ctx, Value::int(21));
        assert_eq!(status, OK);
        out.as_int()
    });

    assert_eq!(
        run.suspends, 1,
        "the helper must have suspended exactly once"
    );
    assert_eq!(
        run.value, 42,
        "and execution must resume into the JIT frames"
    );
    assert_eq!(run.ctx.suspends, 1);
}

#[test]
fn repeated_suspends_leave_the_frames_intact() {
    let mut probe = Probe::new();
    let chain: NvsFn = probe.compile_chain(3, Helper::Suspend);

    let run = in_coroutine(Ctx::new(), move |ctx| {
        let mut total = 0i64;
        for i in 1..=5i64 {
            let (status, out) = call(chain, ctx, Value::int(i));
            assert_eq!(status, OK, "call {i}");
            total += out.as_int();
        }
        total
    });

    assert_eq!(run.suspends, 5);
    assert_eq!(run.value, 2 + 4 + 6 + 8 + 10);
}

#[test]
fn a_throw_still_propagates_inside_a_coroutine() {
    let mut probe = Probe::new();
    let chain: NvsFn = probe.compile_chain(3, Helper::Suspend);

    let run = in_coroutine(Ctx::new(), move |ctx| {
        let (status, _) = call(chain, ctx, Value::int(42));
        status
    });

    assert_eq!(
        run.suspends, 0,
        "the throw happens before the suspend point"
    );
    assert_eq!(run.value, THROWN);
    assert!(
        run.ctx
            .pending
            .as_deref()
            .unwrap_or_default()
            .contains("refused 42"),
        "got {:?}",
        run.ctx.pending
    );
}

#[test]
fn a_runtime_panic_is_contained_inside_a_coroutine_too() {
    // Containment must hold on a coroutine stack as well as the main stack,
    // since in production every request runs on one.
    let mut probe = Probe::new();
    let chain: NvsFn = probe.compile_chain(3, Helper::Suspend);

    let run = in_coroutine(Ctx::new(), move |ctx| {
        let (status, _) = call(chain, ctx, Value::int(99));
        status
    });

    assert_eq!(run.value, FATAL);
    assert!(
        run.ctx
            .pending
            .as_deref()
            .unwrap_or_default()
            .contains("internal error"),
        "got {:?}",
        run.ctx.pending
    );
}

#[test]
fn many_coroutines_can_be_created_and_driven() {
    // A weak proxy for the concurrency ceiling: Novis needs tens of thousands of
    // in-flight tasks per process, so creating a coroutine must be cheap and
    // leak nothing.
    let mut probe = Probe::new();
    let chain: NvsFn = probe.compile_chain(2, Helper::Suspend);

    // Starts at 100 to stay clear of the helper's sentinels: 42 throws and 99
    // panics, both of which are exercised deliberately by other tests here.
    for round in 100..2_100i64 {
        let run = in_coroutine(Ctx::new(), move |ctx| {
            let (status, out) = call(chain, ctx, Value::int(round));
            assert_eq!(status, OK, "round {round}");
            out.as_int()
        });
        assert_eq!(run.suspends, 1, "round {round}");
        assert_eq!(run.value, round * 2, "round {round}");
    }
}

// ---------------------------------------------------------------------------
// The outer containment boundary — ADR 0106 § 2
// ---------------------------------------------------------------------------

#[test]
fn a_panic_in_a_worker_task_is_contained_at_the_task() {
    // The helper wrapper contains what a *helper* raises. This is the fault it
    // cannot see: the task's own code, running on a coroutine stack with the
    // JIT frames already returned. Containment has to survive the stack switch,
    // because a coroutine stack is where `nvs-host` will apply it.
    let mut probe = Probe::new();
    let chain: NvsFn = probe.compile_chain(2, Helper::Suspend);

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let fault = run_task(TaskRoot::Worker, move || {
        in_coroutine(Ctx::new(), move |ctx| {
            let (status, out) = call(chain, ctx, Value::int(21));
            assert_eq!(status, OK);
            assert_eq!(out.as_int(), 42);
            panic!("the worker's own code faulted");
        })
    })
    .expect_err("a panic on a coroutine stack must not escape the task root");
    std::panic::set_hook(previous);

    assert_eq!(fault.message(), "the worker's own code faulted");
    assert!(
        fault.retires_worker(),
        "no request owns it, so ADR 0106 § 2 retires the worker rather than \
         charging it to anyone"
    );

    // And the thread that contained it still creates and drives coroutines,
    // which is what "the worker survives" means before the drain exists.
    let run = in_coroutine(Ctx::new(), move |ctx| {
        let (status, out) = call(chain, ctx, Value::int(5));
        assert_eq!(status, OK);
        out.as_int()
    });
    assert_eq!(run.value, 10);
    assert_eq!(run.suspends, 1);
}

#[test]
fn a_request_owned_panic_leaves_the_worker_running() {
    // The other half of § 2's split, on the same stack shape: a fault a request
    // owns fails that request and nothing else, so the worker is not retired.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let fault = run_task(TaskRoot::Request, || {
        in_coroutine(Ctx::new(), |_ctx| {
            panic!("a request-owned bug");
        })
    })
    .expect_err("the panic must be contained at the task root");
    std::panic::set_hook(previous);

    assert!(
        !fault.retires_worker(),
        "the request owns the fault, so the worker keeps its other requests"
    );
}

#[test]
fn a_teardown_path_that_panics_does_not_recurse() {
    // ADR 0106 § 3's two bullets, which only bite together: a value graph is
    // released *while a panic is unwinding*, which is the one place where a
    // recursive teardown or a fallible one stops being containable. A stack
    // overflow or a second panic there aborts the process before any
    // `catch_unwind` — this test's own included — is ever consulted, so what
    // it pins is that `run_task` still gets to answer.
    //
    // `nvs-runtime`'s `array::tests::a_deeply_nested_array_releases_without_recursing`
    // owns the depth; this is the same depth reached down the unwinding path
    // rather than the ordinary one.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let fault = run_task(TaskRoot::Request, || {
        let mut nest = nvs_runtime::NvsArray::new();
        for _ in 0..200_000 {
            let mut outer = nvs_runtime::NvsArray::new();
            outer.append(nvs_runtime::Value::array(nest));
            nest = outer;
        }
        // Live across the panic, so its `Drop` runs as the frame unwinds and
        // the worklist in `nvs_runtime::release` drains from there.
        assert_eq!(nest.count(), 1);
        panic!("the request faulted with a deep graph still live");
    })
    .expect_err("the panic must be contained at the task root");
    std::panic::set_hook(previous);

    assert_eq!(
        fault.message(),
        "the request faulted with a deep graph still live"
    );
    assert!(
        !fault.retires_worker(),
        "a request-owned fault does not retire the worker, however deep the \
         graph it was holding"
    );
}

// ---------------------------------------------------------------------------
// Depth on the engine's own stack — ADR 0106 § 4
// ---------------------------------------------------------------------------

#[test]
fn engine_recursion_is_bounded_before_the_stack_is() {
    // ADR 0020 § 1 bounds recursion through Novis frames. A decoder's descent
    // over request data has no Novis frames in it, so its own counter is the
    // only bound — and the stack that bound has to stay under is not this
    // thread's. Every request runs on a coroutine stack, which is where the
    // margin is smallest, so both halves are asked for there: the deepest
    // document the default `maxDepth` accepts still decodes, and one four
    // orders of magnitude past it comes back as an ordinary catchable failure
    // rather than as a fault or as an overflow.
    //
    // Counted PHP's way, so a scalar is depth 1 and `[1]` is depth 2: 511
    // brackets is the last document a `maxDepth` of 512 accepts, and
    // `nvs-stdlib`'s `json` tests own that arithmetic.
    let accepted = format!("{}1{}", "[".repeat(511), "]".repeat(511));
    let refused = format!("{}1{}", "[".repeat(100_000), "]".repeat(100_000));

    let run = in_coroutine(Ctx::new(), move |_| {
        (
            decode_on_this_stack(&accepted),
            decode_on_this_stack(&refused),
        )
    });

    let (accepted, refused) = run.value;
    assert_eq!(
        accepted,
        Ok(()),
        "the deepest document the limit admits still fits the coroutine stack \
         it is decoded on — a bound above the stack is not a bound"
    );
    assert_eq!(
        refused,
        Err(nvs_runtime::THROWN),
        "and one past the limit is refused by the counter, not by the stack"
    );
}

/// Decodes `document` at the default `maxDepth`, keeping only the status.
///
/// The message is not read back: `Ctx::take_pending` renders a throw through
/// its exception object, and no exception class is installed in a context this
/// bare — what is being asked here is which of ADR 0002's three statuses the
/// decoder chose, which is the half that distinguishes a refusal from a fault.
fn decode_on_this_stack(document: &str) -> Result<(), i32> {
    let mut ctx = nvs_runtime::Ctx::buffered();
    let text = nvs_runtime::Value::str(nvs_runtime::NvsStr::new(document.as_bytes()));
    let answer = nvs_runtime::call(
        nvs_stdlib::json::nvs_core_json_decode,
        &mut ctx,
        // The default `maxDepth`; a compiled call site that names no option
        // passes the bag's constant, which is this same number.
        &[
            text,
            nvs_runtime::Value::uint(nvs_stdlib::json::DEFAULT_MAX_DEPTH),
        ],
    );
    #[expect(
        unsafe_code,
        reason = "the caller owns both references: a helper borrows its \
                  parameters and releases none of them, and a decoded document \
                  is handed back with its reference transferred"
    )]
    unsafe {
        text.release();
        if let Ok(value) = answer {
            value.release();
        }
    }
    answer.map(|_| ())
}
