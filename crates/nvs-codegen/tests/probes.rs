//! `rule:testing/debug-probes`'s flag-gated probes and the safepoint poll they share: coverage counts, and what a pending request stops.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

/// A script whose body is one loop, so its Cranelift IR holds a function-entry
/// poll and a back-edge poll and nothing else that reads the context.
const A_LOOP: &str = "<?nvs\nint $i = 0;\nwhile ($i < 3) {\n    $i = $i + 1;\n}\necho $i;\n";

/// One `(line, hits)` pair per counted site, sorted by line.
type Lines = Vec<(usize, u64)>;

/// Runs `source` with `flags` on and a hit table sized for its program's
/// statements and conditional edges. Returns one `(line, hits)` pair per
/// statement and one per edge, each list sorted by line. A statement or an
/// edge that never ran is in its list with `0`.
fn counted_with(ctx: &mut Ctx, flags: DebugFlags, source: &str) -> (Lines, Lines) {
    let program = lower(source);
    let stmts = program.stmt_spans();
    let edges = program.edge_spans();
    let hits = std::sync::Arc::new(nvs_runtime::StmtHits::with_edges(stmts.len(), edges.len()));
    ctx.set_stmt_hits(Some(std::sync::Arc::clone(&hits)));
    ctx.set_debug_flags(flags);
    let unit = nvs_codegen::compile(&program).expect("the fixture compiles");
    unit.install_in(ctx);
    unit.script()
        .expect("the script frame was compiled")
        .call(ctx)
        .expect("the script ran");
    // `lower` names its one file `test.nvs`, so the same map gives every span
    // the same file id.
    let mut map = nvs_diagnostics::SourceMap::new();
    map.add("test.nvs", source);
    let by_line = |spans: &[nvs_diagnostics::Span], counts: Vec<u64>| {
        let mut lines: Vec<(usize, u64)> = spans
            .iter()
            .zip(counts)
            .map(|(span, count)| (map.file(span.file).line_col(span.start).0 + 1, count))
            .collect();
        lines.sort_unstable();
        lines
    };
    (
        by_line(&stmts, hits.counts()),
        by_line(&edges, hits.edge_counts()),
    )
}

/// [`counted_with`]'s statement list alone.
fn covered_with(ctx: &mut Ctx, flags: DebugFlags, source: &str) -> Vec<(usize, u64)> {
    counted_with(ctx, flags, source).0
}

/// [`counted_with`]'s edge list alone, on a fresh context.
fn branched_with(flags: DebugFlags, source: &str) -> Vec<(usize, u64)> {
    counted_with(&mut Ctx::buffered(), flags, source).1
}

/// [`covered_with`] with coverage on, on a fresh context.
fn covered(source: &str) -> Vec<(usize, u64)> {
    covered_with(&mut Ctx::buffered(), DebugFlags::COVERAGE, source)
}

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
    let lines = covered_with(
        &mut Ctx::buffered(),
        DebugFlags::empty(),
        "<?nvs\necho \"a\";\necho \"b\";\n",
    );
    assert_eq!(lines, [(2, 0), (3, 0)]);
}

#[test]
fn turning_coverage_on_records_one_hit_per_executed_statement() {
    assert_eq!(
        covered("<?nvs\necho \"a\";\necho \"b\";\necho \"c\";\n"),
        [(2, 1), (3, 1), (4, 1)]
    );
}

/// Two functions each number their statements from zero. The probe adds each
/// function's base, so the method's line and the script's lines are counted
/// apart, and the method that never ran reads `0`.
const TWO_METHODS: &str = "<?nvs\nclass Math {\n    public static function double(int $n): int {\n        return $n + $n;\n    }\n    public static function never(): int {\n        return 0;\n    }\n}\nint $n = Math::double(2);\nint $m = Math::double($n);\necho $m;\n";

#[test]
fn every_function_counts_into_its_own_part_of_the_table() {
    assert_eq!(
        covered(TWO_METHODS),
        [(4, 2), (7, 0), (10, 1), (11, 1), (12, 1)]
    );
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
    assert_eq!(covered(A_FOLDED_CALL), [(6, 1), (7, 1), (8, 1)]);
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
        let lines = covered_with(&mut ctx, flags, MEASURED);
        let hits: u64 = lines.iter().map(|(_, count)| count).sum();
        (hits, ctx.trace().len())
    };

    // With `rule:testing/debug-probes`'s bits off, the identity is on the context and reaches
    // neither path.
    assert_eq!(sites(true, DebugFlags::empty()), (0, 0));

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

/// An `if` whose condition is true and one whose condition is false, then a
/// loop that runs its body twice.
const BRANCHES: &str = "<?nvs\nint $i = 0;\nif ($i == 0) {\n    echo \"a\";\n}\nif ($i == 1) {\n    echo \"b\";\n}\nwhile ($i < 2) {\n    $i = $i + 1;\n}\n";

#[test]
fn a_branch_probe_counts_each_edge_taken() {
    // Each `if` has two edges and one of them ran once. The loop's test ran
    // three times: twice into the body, and once out of the loop.
    assert_eq!(
        branched_with(DebugFlags::BRANCH, BRANCHES),
        [(3, 0), (3, 1), (6, 0), (6, 1), (9, 1), (9, 2)]
    );
}

/// Two functions, each with one `if`, so each numbers its edges from zero.
const TWO_BRANCHING_METHODS: &str = "<?nvs\nclass Sign {\n    public static function of(int $n): int {\n        if ($n < 0) {\n            return -1;\n        }\n        return 1;\n    }\n    public static function never(int $n): int {\n        if ($n < 0) {\n            return 0;\n        }\n        return 1;\n    }\n}\necho Sign::of(-5);\necho Sign::of(5);\necho Sign::of(7);\n";

#[test]
fn every_function_counts_its_edges_into_its_own_part_of_the_table() {
    // The probe adds each function's edge base, so the method that ran twice
    // past its `if` and once into it is counted apart from the one that never
    // ran.
    assert_eq!(
        branched_with(DebugFlags::BRANCH, TWO_BRANCHING_METHODS),
        [(4, 1), (4, 2), (10, 0), (10, 0)]
    );
}

#[test]
fn branch_coverage_and_line_coverage_are_separate_flags() {
    // Line coverage alone counts no edge, and branch coverage alone counts no
    // statement, though both read the same flag word at their sites.
    assert!(
        branched_with(DebugFlags::COVERAGE, BRANCHES)
            .iter()
            .all(|(_, count)| *count == 0)
    );
    let (lines, edges) = counted_with(&mut Ctx::buffered(), DebugFlags::BRANCH, BRANCHES);
    assert!(lines.iter().all(|(_, count)| *count == 0), "{lines:?}");
    assert!(edges.iter().any(|(_, count)| *count > 0), "{edges:?}");
}

#[test]
fn the_branch_probe_costs_nothing_observable_with_every_bit_off() {
    // The check is emitted at every two-way branch. With no bit set the run
    // never reaches `nvs_probe_edge`, and still takes the same edges.
    let edges = branched_with(DebugFlags::empty(), BRANCHES);
    assert_eq!(edges.len(), 6);
    assert!(edges.iter().all(|(_, count)| *count == 0), "{edges:?}");
    assert_eq!(
        output_of(
            "<?nvs\nint $i = 0;\nwhile ($i < 3) {\n    $i = $i + 1;\n}\nif ($i == 3) {\n    echo \"three\";\n} else {\n    echo \"other\";\n}\n"
        ),
        "three"
    );
}

#[test]
fn coverage_counts_a_looping_statement_once_per_iteration() {
    // The declaration on line 2 and the `while` statement on line 3 run once.
    // The body block also starts on line 3 and runs twice, and the body's one
    // statement on line 4 runs twice.
    assert_eq!(
        covered("<?nvs\nint $i = 0;\nwhile ($i < 2) {\n    $i = $i + 1;\n}\n"),
        [(2, 1), (3, 1), (3, 2), (4, 2)]
    );
}
