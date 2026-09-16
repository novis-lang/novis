//! `rule:testing/debug-probes`'s flag-gated probes and the safepoint poll they share: coverage counts, and what a pending request stops.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

/// A script whose body is one loop, so its Cranelift IR holds a function-entry
/// poll and a back-edge poll and nothing else that reads the context.
const A_LOOP: &str = "<?nvs\nint $i = 0;\nwhile ($i < 3) {\n    $i = $i + 1;\n}\necho $i;\n";

/// One function's Cranelift IR out of the text `nvs_codegen::clif` renders,
/// which heads each function with `; <name>`.
fn section<'a>(text: &'a str, name: &str) -> &'a str {
    let head = format!("; {name}\n");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("the unit defines no `{name}`:\n{text}"))
        + head.len();
    let body = &text[start..];
    match body.find("\n; ") {
        Some(end) => &body[..end],
        None => body,
    }
}

/// Every line of `clif` that loads at offset zero of the ABI's first parameter
/// — `Ctx`'s hot slot, which holds the address of the safepoint word
/// (`nvs_runtime::SAFEPOINT_OFFSET`). Every other inline read of the context is
/// at a non-zero offset, so this names the handle load and nothing else.
fn handle_loads(clif: &str) -> Vec<&str> {
    clif.lines()
        .map(str::trim)
        .filter(|line| line.contains("load") && (line.ends_with("v0") || line.ends_with("v0+0")))
        .collect()
}

/// The instruction lines of `clif`'s ABI entry block, which is its first.
fn entry_block(clif: &str) -> Vec<&str> {
    clif.lines()
        .map(str::trim)
        .skip_while(|line| !line.starts_with("block0("))
        .skip(1)
        .take_while(|line| !line.starts_with("block"))
        .collect()
}

#[test]
fn the_safepoint_handle_is_loaded_in_the_abi_entry_block() {
    // The word a poll reads lives outside `Ctx` so that a thread which does not
    // own the request can write it, and the hot slot holds its address — so the
    // poll is a pointer hop. This is what makes the hop cost one per *call*
    // rather than one per poll: the entry block dominates every block below it,
    // so the address is bound there and nothing below reloads it. Every
    // function, not just this script's own: `nvs_ir::lower` gives each one an
    // entry poll, so a handle loaded anywhere else would be one loaded twice.
    let text = nvs_codegen::clif(&lower(A_LOOP)).expect("the unit compiled");
    for clif in text.split("\n; ") {
        let loads = handle_loads(clif);
        assert_eq!(loads.len(), 1, "the handle is not loaded once:\n{clif}");
        assert!(
            entry_block(clif).contains(&loads[0]),
            "the handle is loaded outside the ABI entry block:\n{clif}"
        );
    }
}

#[test]
fn a_loop_back_edge_polls_with_one_load() {
    // What the entry block's binding buys, asserted where it is spent: a poll
    // on a back edge reads the word through the value bound at entry, so going
    // round the loop costs the load of the word, the test and the branch — and
    // not a second load to find the word first.
    let text = nvs_codegen::clif(&lower(A_LOOP)).expect("the unit compiled");
    let clif = section(&text, "<script>");
    let loads = handle_loads(clif);
    assert_eq!(loads.len(), 1, "the handle is not loaded once:\n{clif}");

    let (handle, _) = loads[0]
        .split_once(" = ")
        .expect("a load names the value it defines");
    let polls = clif
        .lines()
        .map(str::trim)
        .filter(|line| line.contains("load") && line.ends_with(handle))
        .count();
    assert!(
        polls >= 2,
        "this script has a function-entry poll and a back-edge poll, and both \
         read the word through the entry block's value; found {polls}:\n{clif}"
    );
}

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
    // `rule:testing/debug-probes`'s check is emitted unconditionally at every statement
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

/// A retrieval the checker answers whole — `rule:attributes/retrieval-folds-while-checking`'s
/// fold — written as one statement's entire expression, with an ordinary statement on either side
/// of it.
const A_FOLDED_CALL: &str = "<?nvs\nclass Row {\n    #[{column: \"name\"}]\n    public string $name = \"\";\n}\necho \"a\";\n?{column: string} $found = Core\\Attributes::get<{column: string}>(Row::constructor(...), \"name\");\necho \"b\";\n";

#[test]
fn a_statement_holding_a_folded_intrinsic_call_is_still_reported_covered() {
    // `rule:expressions/preparation-preserves-behaviour`'s last paragraph names one observable
    // difference and bounds it: a fully folded call is no longer a call, so no per-call probe
    // fires for it — and the statement it was written in is still a statement, so
    // `rule:testing/debug-probes`' coverage reports it like any other. The failure this pins is a
    // line vanishing from a coverage report for having been answered early, which would make
    // coverage a claim about the optimiser rather than about the program.
    //
    // That the call really folded is what the run itself says: both rows of `Core\Attributes`
    // name a body that aborts the process on entry (`nvs_stdlib::attributes`'s
    // `folded_at_compile_time`), so a script that returns at all is one whose retrieval never
    // became a call.
    let mut ctx = Ctx::buffered();
    ctx.set_debug_flags(DebugFlags::COVERAGE);
    run_with(&mut ctx, A_FOLDED_CALL).expect("the script ran");
    assert_eq!(ctx.stmt_hits(), [1, 1, 1]);
}

/// A statement path and a call path in one script, so `rule:testing/debug-probes`'s probe units
/// are present for the assertion below to count.
const MEASURED: &str = "<?nvs\nclass Math {\n    public static function double(int $n): int {\n        return $n + $n;\n    }\n}\nint $n = Math::double(2);\necho $n;\n";

#[test]
fn no_probe_is_added_to_the_measured_path() {
    // `rule:observability/default-series`: every default series is read from instrumentation that
    // already exists, so its export "adds no probe site to the
    // per-statement/per-call path" `rule:testing/debug-probes` measures. A probe site is emitted
    // code, which is why the claim is this crate's — a server sees a header
    // and a counter, never a site.
    //
    // The case that would break it is a request already carrying `rule:observability/a-trace-id-exists-for-every-request`
    // 's trace identity, which is what an exporter keys its spans off: a
    // site emitted to feed a series, or an identity that switches `rule:testing/debug-probes`'s
    // own sites on, both move the counts. They are asserted against the same
    // script with no identity, so either failure shows up here while both runs
    // still print `4`.
    let sites = |identity: bool, flags: DebugFlags| {
        let mut ctx = Ctx::buffered();
        if identity {
            let mut inbound = Inbound::new("GET", "/", "");
            inbound.set_trace_context(TraceContext::continuing(
                Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
                0.0,
            ));
            ctx.set_inbound(inbound);
        }
        ctx.set_debug_flags(flags);
        run_with(&mut ctx, MEASURED).expect("the script ran");
        (ctx.stmt_hits().to_vec(), ctx.trace().len())
    };

    // With `rule:testing/debug-probes`'s bits off, the identity is on the context and reaches
    // neither path.
    assert_eq!(sites(true, DebugFlags::empty()), (Vec::new(), 0));

    // With them on, what fires is exactly `rule:testing/debug-probes`'s own set, identity or not.
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
