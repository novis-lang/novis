//! End-to-end: MWL source in, native code out, run it, assert on what it
//! wrote.
//!
//! These go through the *real* pipeline — `mwl-syntax`, `mwl-hir`,
//! `mwl-types`, `mwl-ir`, `mwl-codegen` — rather than hand-building IR,
//! because the property worth guarding is that the five crates agree, and a
//! hand-built `Function` would let this crate's own assumptions about what
//! lowering produces go untested. That is also why the assertions are on
//! output bytes and statuses rather than on generated instructions: what
//! `mwl run` prints is the observable contract, and `benches/abi-probe` is
//! where instruction-level costs are held.

use mwl_diagnostics::{Diagnostics, SourceMap};
use mwl_runtime::{Ctx, DebugFlags, FATAL, OK, SafepointFlags, Value, call};

/// Compiles a whole file, returning the unit or the first thing that refused
/// it.
///
/// Front-end diagnostics are a panic rather than an error: every fixture below
/// is meant to type-check, so a diagnostic is a broken fixture, not an outcome
/// under test.
fn compile(source: &str) -> Result<mwl_codegen::Unit, mwl_codegen::CodegenError> {
    mwl_codegen::compile(&lower(source))
}

/// Runs the whole front end over `source` and lowers it — every class method
/// plus the script frame — without compiling it.
fn lower(source: &str) -> mwl_ir::Program {
    let mut map = SourceMap::new();
    let id = map.add("test.mwl", source);
    let src = map.file(id);

    let mut diags = Diagnostics::new();
    let stmts = mwl_syntax::parse_file(src, &mut diags);
    let module = mwl_hir::resolve_file(&stmts, src, &mut diags);
    let mut interner = mwl_types::TypeInterner::new();
    let mut exprs = mwl_types::ExprTypeTable::new();
    mwl_types::check_program(&stmts, src, &module, &mut interner, &mut exprs, &mut diags);
    assert!(
        !diags.has_errors(),
        "the fixture does not type-check: {:?}",
        diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );

    mwl_ir::lower::lower_file("<script>", &stmts, src, &exprs, &interner)
}

/// Compiles and runs `source` against `ctx`, returning the compiled status.
fn run_with(ctx: &mut Ctx, source: &str) -> Result<Value, i32> {
    let unit = compile(source).expect("the fixture compiles");
    let entry = unit
        .function("<script>")
        .expect("the script frame was compiled");
    call(entry, ctx, &[])
}

/// Compiles, runs, and returns whatever the script echoed.
fn output_of(source: &str) -> String {
    let mut ctx = Ctx::buffered();
    run_with(&mut ctx, source).expect("the script ran to completion");
    String::from_utf8(ctx.take_buffered_output().expect("a buffered context"))
        .expect("the script echoed UTF-8")
}

#[test]
fn the_acceptance_program_prints_hello_world() {
    assert_eq!(
        output_of("<?mwl\necho \"Hello, World!\";\n"),
        "Hello, World!"
    );
}

#[test]
fn echo_writes_its_operands_in_order_with_no_separator() {
    assert_eq!(output_of("<?mwl\necho \"a\", \"b\";\necho \"c\";\n"), "abc");
}

#[test]
fn a_scalar_operand_reaches_the_matching_conversion_helper() {
    // `echo` of a non-string goes through `Helper::IntToString` and friends,
    // which is `mwl-codegen`'s helper-symbol table under test as much as the
    // conversion itself.
    assert_eq!(output_of("<?mwl\necho 42;\n"), "42");
    assert_eq!(output_of("<?mwl\necho -7;\n"), "-7");
    assert_eq!(output_of("<?mwl\necho true;\n"), "1");
    assert_eq!(output_of("<?mwl\necho false;\n"), "");
    assert_eq!(output_of("<?mwl\necho 1.5;\n"), "1.5");
}

#[test]
fn an_escape_reaches_the_output_as_the_byte_it_names() {
    assert_eq!(output_of("<?mwl\necho \"a\\tb\\n\";\n"), "a\tb\n");
}

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

/// The `examples/calls.mwl` acceptance fixture, as an inline source string.
const CALLS: &str = "<?mwl
class Math {
    public static function double(int $n): int {
        return $n * 2;
    }

    public static function quadruple(int $n): int {
        return Math::double(Math::double($n));
    }
}

echo \"quadruple(5) = \" . Math::quadruple(5) . \"\\n\";
";

#[test]
fn a_static_call_reaches_its_callee_and_brings_a_value_back() {
    assert_eq!(output_of(CALLS), "quadruple(5) = 20\n");
}

#[test]
fn a_call_resolves_a_callee_declared_after_it() {
    // The declare-then-define pass is what makes this work: `first` names
    // `second`, which the unit only declares later in source order.
    assert_eq!(
        output_of(
            "<?mwl\nclass C {\n    public static function first(): int {\n        return C::second() + 1;\n    }\n    public static function second(): int {\n        return 41;\n    }\n}\necho C::first();\n"
        ),
        "42"
    );
}

#[test]
fn a_recursive_call_terminates_and_returns_the_right_value() {
    assert_eq!(
        output_of(
            "<?mwl\nclass F {\n    public static function fact(int $n): int {\n        if ($n < 2) {\n            return 1;\n        }\n        return $n * F::fact($n - 1);\n    }\n}\necho F::fact(10);\n"
        ),
        "3628800"
    );
}

#[test]
fn the_call_probe_costs_nothing_observable_with_every_bit_off() {
    let mut ctx = Ctx::buffered();
    run_with(&mut ctx, CALLS).expect("the script ran");
    assert!(ctx.trace().is_empty());
}

#[test]
fn turning_tracing_on_records_an_entry_and_an_exit_per_call() {
    // ADR 0018 § 1's call-site pair. `quadruple` is entered first and left
    // last; both `double` calls nest inside it, and every exit carries the
    // status the call site is about to branch on.
    let mut ctx = Ctx::buffered();
    ctx.set_debug_flags(DebugFlags::TRACE);
    run_with(&mut ctx, CALLS).expect("the script ran");

    let events: Vec<(&str, Option<i32>)> = ctx
        .trace()
        .iter()
        .map(|e| (e.callee.as_str(), e.status))
        .collect();
    assert_eq!(
        events,
        [
            ("Math::quadruple", None),
            ("Math::double", None),
            ("Math::double", Some(OK)),
            ("Math::double", None),
            ("Math::double", Some(OK)),
            ("Math::quadruple", Some(OK)),
        ]
    );
}

// A *non-`OK`* traced exit has no fixture here yet, and deliberately not: the
// only stop this slice can provoke is a pending safepoint, which fires at the
// script frame's own entry poll before any call is reached. `mwl_probe_call_
// exit`'s own unit test covers that it records the status it is handed; that
// the probe is emitted *before* ADR 0002's compare-and-branch — so a thrown
// exit is traced rather than skipped along with the rest of the frame — gets
// its end-to-end fixture with `throw`, which is the next slice.

#[test]
fn a_callee_that_stops_the_request_stops_its_caller_too() {
    // ADR 0002's compare-and-branch doing its job across an MWL-level frame:
    // the callee's entry safepoint refuses, and the status travels up through
    // the caller unchanged rather than being swallowed at the call site.
    let mut ctx = Ctx::buffered();
    ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
    let status = run_with(&mut ctx, CALLS).unwrap_err();

    assert_eq!(status, FATAL);
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
}

#[test]
fn every_lowered_method_is_compiled_under_its_class_qualified_name() {
    // The label a call's target is rendered from and the name its callee is
    // compiled under are the same string by construction — see
    // `mwl_types::expr_table::ExprTypeTable::method_label`. A namespace is
    // where the two would drift apart if they were spelled twice.
    let unit = compile(
        "<?mwl\nnamespace App;\nclass Math {\n    public static function id(int $n): int {\n        return $n;\n    }\n}\necho \\App\\Math::id(7);\n",
    )
    .expect("the fixture compiles");
    assert!(unit.function("App\\Math::id").is_some(), "{unit:?}");
    assert!(unit.function("<script>").is_some(), "{unit:?}");
}

#[test]
fn disassembling_names_each_frame_and_shows_the_code_that_would_have_run() {
    // What `mwl run --dump-asm` prints. The two structural claims worth
    // holding: every compiled frame gets a section headed by its MWL name, and
    // the section carries the probe sites this backend emits unconditionally —
    // so a disassembly cannot silently be of some differently-configured
    // second compile.
    let text = mwl_codegen::disassemble(&lower("<?mwl\necho \"Hello, World!\";\n"))
        .expect("the fixture compiles");

    assert!(text.starts_with("; <script>\n"), "{text}");
    assert!(text.contains("block0:"), "{text}");
    // Two `load_ext_name` sites at minimum: the safepoint slow path and the
    // statement probe, both out-of-line calls this backend always emits.
    assert!(text.matches("load_ext_name").count() >= 2, "{text}");
}

#[test]
fn disassembling_an_unlowered_shape_reports_it_rather_than_printing_half_a_unit() {
    let error = mwl_codegen::disassemble(&lower("<?mwl\narray<int> $a = [1, 2];\n")).unwrap_err();
    assert!(error.to_string().contains("array literal"), "{error}");
}

#[test]
fn an_unlowered_shape_is_an_error_naming_it_rather_than_a_panic() {
    // An array has no runtime representation yet — the crate docs' known gap
    // 1. What matters is that the backend *says so* instead of panicking or,
    // worse, emitting something.
    let error = compile("<?mwl\narray<int> $a = [1, 2];\n").unwrap_err();
    let message = error.to_string();
    assert!(message.contains("array literal"), "{message}");
}

#[test]
fn concatenation_joins_its_operands_and_converts_a_scalar_one_first() {
    // `.` over two strings is one `mwl_str_concat`; over a scalar it is the
    // matching `…ToString` helper first, which `mwl-ir` inserts. Both halves
    // are what every acceptance example past `hello.mwl` runs on.
    assert_eq!(output_of("<?mwl\necho \"a\" . \"b\";\n"), "ab");
    assert_eq!(output_of("<?mwl\necho \"\" . \"\";\n"), "");
    assert_eq!(output_of("<?mwl\necho \"n = \" . 42 . \"!\";\n"), "n = 42!");
    assert_eq!(output_of("<?mwl\necho \"f\" . 1.5 . true;\n"), "f1.51");
}

#[test]
fn a_concatenation_in_a_loop_keeps_producing_the_right_bytes() {
    // Each iteration's result is released once the local it was assigned to is
    // overwritten, so a botched refcount here shows up as freed bytes rather
    // than only as a leak. The leak half is what M4's Valgrind/ASAN run is for.
    assert_eq!(
        output_of(
            "<?mwl\nstring $s = \"\";\nint $i = 0;\nwhile ($i < 4) {\n    $s = $s . \"ab\";\n    $i = $i + 1;\n}\necho $s;\n"
        ),
        "abababab"
    );
}
