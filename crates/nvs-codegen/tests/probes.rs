//! ADR 0018's flag-gated probes and the safepoint poll they share: coverage counts, and what a pending request stops.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn a_pending_safepoint_stops_the_script_before_it_writes_anything() {
    // The poll `nvs-codegen` emits at function entry, doing its job: a request
    // whose CPU budget is already spent never reaches its first statement, and
    // the stop is FATAL rather than THROWN because `rule:errors/escalation-ladder` keeps a
    // resource-limit report out of `catch` entirely.
    let mut ctx = Ctx::buffered();
    ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
    let status = run_with(&mut ctx, "<?nvs\necho \"Hello, World!\";\n").unwrap_err();

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
    run_with(&mut ctx, "<?nvs\necho \"Hello, World!\";\n").expect("the script ran");

    assert_eq!(
        ctx.take_buffered_output().as_deref(),
        Some(&b"Hello, World!"[..])
    );
    assert!(ctx.safepoint_flags().is_empty());
}

#[test]
fn the_debug_probe_costs_nothing_observable_with_every_bit_off() {
    // ADR 0018's check is emitted unconditionally at every statement
    // boundary. With no bit set it must not reach `nvs_probe_stmt` at all.
    let mut ctx = Ctx::buffered();
    run_with(&mut ctx, "<?nvs\necho \"a\";\necho \"b\";\n").expect("the script ran");
    assert!(ctx.stmt_hits().is_empty());
}

#[test]
fn turning_coverage_on_records_one_hit_per_executed_statement() {
    let mut ctx = Ctx::buffered();
    ctx.set_debug_flags(DebugFlags::COVERAGE);
    run_with(&mut ctx, "<?nvs\necho \"a\";\necho \"b\";\necho \"c\";\n").expect("the script ran");
    assert_eq!(ctx.stmt_hits(), [1, 1, 1]);
}

/// A statement path and a call path in one script, so both of ADR 0018's probe
/// units are present for the assertion below to count.
const MEASURED: &str = "<?nvs\nclass Math {\n    public static function double(int $n): int {\n        return $n + $n;\n    }\n}\nint $n = Math::double(2);\necho $n;\n";

#[test]
fn no_probe_is_added_to_the_measured_path() {
    // ADR 0076 § 1: every default series is read from instrumentation that
    // already exists, so its export "adds no probe site to the
    // per-statement/per-call path" ADR 0018 measures. A probe site is emitted
    // code, which is why the claim is this crate's — a server sees a header
    // and a counter, never a site.
    //
    // The case that would break it is a request already carrying ADR 0076
    // § 2's trace identity, which is what an exporter keys its spans off: a
    // site emitted to feed a series, or an identity that switches ADR 0018's
    // own sites on, both move the counts. They are asserted against the same
    // script with no identity, so either failure shows up here while both runs
    // still print `4`.
    let sites = |identity: bool, flags: DebugFlags| {
        let mut ctx = Ctx::buffered();
        if identity {
            let mut inbound = Inbound::new("GET", "/", "");
            inbound.set_trace_context(TraceContext::continuing(Some(
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            )));
            ctx.set_inbound(inbound);
        }
        ctx.set_debug_flags(flags);
        run_with(&mut ctx, MEASURED).expect("the script ran");
        (ctx.stmt_hits().to_vec(), ctx.trace().len())
    };

    // With ADR 0018's bits off, the identity is on the context and reaches
    // neither path.
    assert_eq!(sites(true, DebugFlags::empty()), (Vec::new(), 0));

    // With them on, what fires is exactly ADR 0018's own set, identity or not.
    let on = DebugFlags::COVERAGE | DebugFlags::TRACE;
    assert_eq!(sites(true, on), sites(false, on));
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
            "<?nvs\nint $i = 0;\nwhile ($i < 3) {\n    echo \"x\";\n    $i = $i + 1;\n}\nif ($i > 2) {\n    echo \"!\";\n}\n"
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
        "<?nvs\nint $i = 0;\nwhile ($i < 2) {\n    $i = $i + 1;\n}\n",
    )
    .expect("the script ran");

    // s0 the declaration, s1 the `while` statement itself, s2 the body block,
    // s3 the body's one statement — the body's two run twice, the two before
    // the loop once each.
    assert_eq!(ctx.stmt_hits(), [1, 1, 2, 2]);
}
