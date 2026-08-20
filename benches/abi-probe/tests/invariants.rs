//! The behavioural invariants MWL's execution model depends on.
//!
//! If any of these fail after a dependency bump, the plan for M3 (backend) or
//! M5 (concurrency) is broken and needs revisiting before more code is written
//! on top of it. See `docs/adr/0002-error-propagation.md`.

use mwl_abi_probe::{Ctx, FATAL, Helper, MwlFn, OK, Probe, THROWN, Value, call, in_coroutine};

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
    let chain: MwlFn = probe.compile_chain(2, Helper::Suspend);

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
    let chain: MwlFn = probe.compile_chain(3, Helper::Suspend);

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
    let chain: MwlFn = probe.compile_chain(3, Helper::Suspend);

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
    let chain: MwlFn = probe.compile_chain(3, Helper::Suspend);

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
    // A weak proxy for the concurrency ceiling: MWL needs tens of thousands of
    // in-flight tasks per process, so creating a coroutine must be cheap and
    // leak nothing.
    let mut probe = Probe::new();
    let chain: MwlFn = probe.compile_chain(2, Helper::Suspend);

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
