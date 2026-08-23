//! ADR 0018's flag-gated probes and the safepoint poll they share: coverage counts, and what a pending request stops.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn a_pending_safepoint_stops_the_script_before_it_writes_anything() {
    // The poll `mwl-codegen` emits at function entry, doing its job: a request
    // whose CPU budget is already spent never reaches its first statement, and
    // the stop is FATAL rather than THROWN because ADR 0020 keeps a
    // resource-limit report out of `catch` entirely.
    let mut ctx = Ctx::buffered();
    ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
    let status = run_with(&mut ctx, "<?mwl\necho \"Hello, World!\";\n").unwrap_err();

    assert_eq!(status, FATAL);
    assert_eq!(
        ctx.take_pending().as_deref(),
        Some("the request exceeded its CPU-time limit")
    );
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
}

#[test]
fn a_cleared_safepoint_request_lets_the_script_continue() {
    // COLLECT and DEBUG_BREAK reach the slow path and are cleared there, so
    // the poll's OK return has to actually resume the frame rather than being
    // treated as a stop.
    let mut ctx = Ctx::buffered();
    ctx.request_safepoint(SafepointFlags::COLLECT);
    run_with(&mut ctx, "<?mwl\necho \"Hello, World!\";\n").expect("the script ran");

    assert_eq!(
        ctx.take_buffered_output().as_deref(),
        Some(&b"Hello, World!"[..])
    );
    assert!(ctx.safepoint_flags().is_empty());
}

#[test]
fn the_debug_probe_costs_nothing_observable_with_every_bit_off() {
    // ADR 0018's check is emitted unconditionally at every statement
    // boundary. With no bit set it must not reach `mwl_probe_stmt` at all.
    let mut ctx = Ctx::buffered();
    run_with(&mut ctx, "<?mwl\necho \"a\";\necho \"b\";\n").expect("the script ran");
    assert!(ctx.stmt_hits().is_empty());
}

#[test]
fn turning_coverage_on_records_one_hit_per_executed_statement() {
    let mut ctx = Ctx::buffered();
    ctx.set_debug_flags(DebugFlags::COVERAGE);
    run_with(&mut ctx, "<?mwl\necho \"a\";\necho \"b\";\necho \"c\";\n").expect("the script ran");
    assert_eq!(ctx.stmt_hits(), [1, 1, 1]);
}

#[test]
fn a_loop_and_a_branch_run_through_their_phis() {
    // The one shape that exercises everything structural at once: a header
    // phi patched from a back edge, a conditional branch on a comparison, a
    // back-edge safepoint poll splitting the block its jump is emitted from,
    // and a merge that falls through. That last one is why `phi_args` keys
    // off the *IR* block rather than the Cranelift block a terminator lands
    // in — the safepoint splits `bb2` in two, and the header's phi still
    // names `bb2`.
    assert_eq!(
        output_of(
            "<?mwl\nint $i = 0;\nwhile ($i < 3) {\n    echo \"x\";\n    $i = $i + 1;\n}\nif ($i > 2) {\n    echo \"!\";\n}\n"
        ),
        "xxx!"
    );
}

#[test]
fn coverage_counts_a_looping_statement_once_per_iteration() {
    let mut ctx = Ctx::buffered();
    ctx.set_debug_flags(DebugFlags::COVERAGE);
    run_with(
        &mut ctx,
        "<?mwl\nint $i = 0;\nwhile ($i < 2) {\n    $i = $i + 1;\n}\n",
    )
    .expect("the script ran");

    // s0 the declaration, s1 the `while` statement itself, s2 the body block,
    // s3 the body's one statement — the body's two run twice, the two before
    // the loop once each.
    assert_eq!(ctx.stmt_hits(), [1, 1, 2, 2]);
}
