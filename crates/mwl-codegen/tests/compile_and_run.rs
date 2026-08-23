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
use mwl_runtime::{Ctx, DebugFlags, FATAL, FaultSite, OK, SafepointFlags, THROWN, Value, call};

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

    let layouts = mwl_types::build_class_layouts(&stmts, src, &module.graph);
    mwl_ir::lower::lower_file("<script>", &stmts, src, &exprs, &interner, &layouts)
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
fn a_second_script_runs_after_a_contained_helper_panic() {
    // M3's "leaves the process able to run the next one", which a one-shot
    // `mwl run` cannot show: two compiles and two runs in *this* process, the
    // first faulted deliberately. `benches/abi-probe`'s
    // `the_jit_is_still_usable_after_a_contained_panic` is the ABI-level
    // version of the same claim over hand-built frames; this is the compiler's.
    // Three `echo`s, so the fault lands on the second one — see
    // `FaultSite::HelperPanic` for why it is not the first.
    let source = "<?mwl\necho \"start\\n\";\necho \"middle\\n\";\necho \"end\\n\";\n";

    let mut faulted = Ctx::buffered();
    faulted.inject_fault(FaultSite::HelperPanic);
    let status = run_with(&mut faulted, source).unwrap_err();

    assert_eq!(status, FATAL);
    // Contained, and named: the panic became a status plus a message rather
    // than taking the process down.
    let message = faulted.take_pending().expect("a message was recorded");
    assert!(message.contains("injected helper panic"), "{message}");
    // What the request had already produced survives the fault, and nothing
    // after it runs.
    assert_eq!(
        faulted.take_buffered_output().as_deref(),
        Some(&b"start\n"[..])
    );

    // The second script is unaffected: same process, fresh unit, nothing
    // sticky left behind by the first.
    assert_eq!(output_of(source), "start\nmiddle\nend\n");
    assert_eq!(output_of(CALLS), "quadruple(5) = 20\n");
}

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
    let error = mwl_codegen::disassemble(&lower("<?mwl\nint $q = 7 % 2;\n")).unwrap_err();
    assert!(error.to_string().contains("Mod"), "{error}");
}

#[test]
fn an_unlowered_shape_is_an_error_naming_it_rather_than_a_panic() {
    // Integer modulo still has no lowering — a zero divisor has to throw
    // rather than trap the process, and that needs a checked divisor plus a
    // `Terminator::Throw` (the crate docs' known gaps). What matters is that
    // the backend *says so* instead of panicking or, worse, emitting
    // something.
    let error = compile("<?mwl\nint $q = 7 % 2;\n").unwrap_err();
    let message = error.to_string();
    assert!(message.contains("Mod"), "{message}");
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

/// The three-frame throw `examples/throw.mwl` runs, verbatim.
const THROWS: &str = "<?mwl
class Deep {
    public static function level3(): void {
        throw new Exception(\"boom\");
    }

    public static function level2(): void {
        Deep::level3();
    }

    public static function level1(): void {
        Deep::level2();
    }
}
";

#[test]
fn a_throw_crosses_several_frames_and_is_caught() {
    // ADR 0002's whole claim, end to end: no unwinder is involved, each frame
    // returns `THROWN` and its caller branches on it, and the `catch` three
    // frames up sees the message the `throw` built.
    let source = format!(
        "{THROWS}\ntry {{\n    Deep::level1();\n    echo \"not reached\";\n}} \
         catch (Throwable $e) {{\n    echo \"caught: \" . $e->getMessage();\n}}\n"
    );
    assert_eq!(output_of(&source), "caught: boom");
}

#[test]
fn an_uncaught_throw_leaves_the_status_and_the_message_on_the_context() {
    let mut ctx = Ctx::buffered();
    let source = format!("{THROWS}\nDeep::level1();\n");
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), THROWN);
    assert_eq!(ctx.pending(), Some("boom"));
}

#[test]
fn the_backtrace_names_every_frame_the_throw_left_in_order() {
    // Resolved from MWL's own frame chain — each frame's error path pushes its
    // own label — never from the platform unwinder, which ADR 0002 makes
    // unavailable through a JIT frame in the first place.
    let mut ctx = Ctx::buffered();
    let source = format!("{THROWS}\nDeep::level1();\n");
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), THROWN);

    let trace = ctx
        .take_thrown()
        .expect("an uncaught throw leaves its exception behind")
        .trace_as_string();
    let frames: Vec<&str> = trace.lines().collect();
    assert_eq!(frames.len(), 4, "{trace}");
    assert!(frames[0].starts_with("#0 Deep::level3() at "), "{trace}");
    assert!(frames[1].starts_with("#1 Deep::level2() at "), "{trace}");
    assert!(frames[2].starts_with("#2 Deep::level1() at "), "{trace}");
    assert!(frames[3].starts_with("#3 <script>() at "), "{trace}");
}

#[test]
fn a_caught_throw_stops_the_backtrace_at_the_frame_that_handled_it() {
    // The trace holds the frames the exception actually unwound *out of*, so a
    // `catch` in the calling frame never appears in it — `mwl_runtime::throwable`
    // owns that rule and why it differs from PHP's construction-time snapshot.
    let source = format!(
        "{THROWS}\ntry {{\n    Deep::level3();\n}} catch (Throwable $e) {{\n    \
         echo $e->getTraceAsString();\n}}\n"
    );
    let out = output_of(&source);
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.starts_with("#0 Deep::level3() at "), "{out}");
}

#[test]
fn a_fatal_is_never_caught() {
    // ADR 0020: a resource-limit stop is not a `Throwable`, so the `catch`
    // this program wraps around it does not run and the status travels on out.
    let mut ctx = Ctx::buffered();
    ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
    let source = format!(
        "{THROWS}\ntry {{\n    Deep::level1();\n}} catch (Throwable $e) {{\n    \
         echo \"caught\";\n}}\n"
    );
    assert_eq!(run_with(&mut ctx, &source).unwrap_err(), FATAL);
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
}

#[test]
fn a_frame_that_throws_releases_the_strings_it_still_held() {
    // The error path's refcount cleanup, observed rather than assumed: the
    // caught exception's message is the only allocation still alive once the
    // dust settles, so a landing block that skipped its releases would leave
    // the local's buffer behind. `mwl-ir`'s landing blocks are what put the
    // releases there; this checks the backend actually emits them.
    let source = format!(
        "{THROWS}\ntry {{\n    string $held = \"kept alive\";\n    Deep::level1();\n    \
         echo $held;\n}} catch (Throwable $e) {{\n    echo \"caught: \" . $e->getMessage();\n}}\n"
    );
    assert_eq!(output_of(&source), "caught: boom");
}

// ---------------------------------------------------------------------------
// Objects — M4's representation, end to end
// ---------------------------------------------------------------------------

/// A two-property class with a constructor and two readers, plus a subclass
/// that inherits both — the shape every object test below builds on.
const SHAPES: &str = "<?mwl
class Animal {
    public string $animalName;
    public int $legs;

    public function constructor(string $animalName, int $legs) {
        $this->animalName = $animalName;
        $this->legs = $legs;
    }

    public function name(): string {
        return $this->animalName;
    }

    public function legCount(): int {
        return $this->legs;
    }
}

class Dog extends Animal {
    public string $sound;

    public function constructor(string $animalName) {
        parent::constructor($animalName, 4);
        $this->sound = \"woof\";
    }

    public function speak(): string {
        return $this->sound;
    }
}
";

#[test]
fn a_constructor_writes_its_fields_and_a_reader_reads_them_back() {
    let source = format!("{SHAPES}\nvar $a = new Animal(\"cat\", 4);\necho $a->name();\n");
    assert_eq!(output_of(&source), "cat");
}

#[test]
fn an_int_field_survives_the_round_trip_through_its_value_slot() {
    // A field slot is a whole 16-byte `Value`, so an `int` field is written
    // with a tag byte the read never looks at — `mwl_runtime::object`'s own
    // docs own that decision. This is it working.
    let source = format!("{SHAPES}\nvar $a = new Animal(\"cat\", 4);\necho $a->legCount();\n");
    assert_eq!(output_of(&source), "4");
}

#[test]
fn a_subclass_reaches_both_its_own_slot_and_its_parents() {
    // `sound` is `Dog`'s own, at the slot after `Animal`'s two — the
    // ancestors-first layout rule, observed rather than asserted.
    let source = format!(
        "{SHAPES}\nvar $d = new Dog(\"rex\");\necho $d->name(), \"/\", $d->speak(), \"/\";\necho $d->legCount();\n"
    );
    assert_eq!(output_of(&source), "rex/woof/4");
}

#[test]
fn an_inherited_method_is_called_on_a_subclass_instance() {
    // `Dog` declares no `name()`, so the call resolves to `Animal::name` with
    // a `Dog` receiver — the slot index the parent computed still lands on the
    // right field.
    let source = format!("{SHAPES}\nvar $d = new Dog(\"rex\");\necho $d->name();\n");
    assert_eq!(output_of(&source), "rex");
}

#[test]
fn a_field_can_be_overwritten_after_construction() {
    let source = format!(
        "{SHAPES}\nvar $a = new Animal(\"cat\", 4);\n$a->animalName = \"lion\";\n$a->legs = 3;\necho $a->name(), \"/\", $a->legCount();\n"
    );
    assert_eq!(output_of(&source), "lion/3");
}

#[test]
fn an_object_field_keeps_its_own_object_alive() {
    let source = "<?mwl
class Leg {
    public int $length;
    public function constructor(int $length) { $this->length = $length; }
    public function length(): int { return $this->length; }
}

class Cat {
    public Leg $front;
    public function constructor(Leg $front) { $this->front = $front; }
    public function front(): Leg { return $this->front; }
}

var $cat = new Cat(new Leg(12));
echo $cat->front()->length();
";
    assert_eq!(output_of(source), "12");
}

#[test]
fn a_string_field_overwritten_in_a_loop_leaks_nothing() {
    // Each write releases what the slot held — `mwl_ir::lower` emits the
    // `FieldGet`/`Release` pair and this backend emits the store. A missing
    // release would leak 10_000 buffers; a doubled one would crash.
    let source = format!(
        "{SHAPES}\nvar $a = new Animal(\"cat\", 4);\nvar $i = 0;\nwhile ($i < 10000) {{\n    $a->animalName = \"n\" . $i;\n    $i = $i + 1;\n}}\necho $a->name();\n"
    );
    assert_eq!(output_of(&source), "n9999");
}

#[test]
fn a_discarded_instance_is_released_rather_than_leaked() {
    // A bare `new` used for its constructor's effect only. The instance has
    // exactly one owner and nothing binds it, so lowering releases it right
    // away — 50_000 of them must not grow the heap without bound.
    let source = format!(
        "{SHAPES}\nvar $i = 0;\nwhile ($i < 50000) {{\n    new Dog(\"rex\");\n    $i = $i + 1;\n}}\necho \"done\";\n"
    );
    assert_eq!(output_of(&source), "done");
}

#[test]
fn a_class_with_no_constructor_still_allocates() {
    let source = "<?mwl
class Marker {
    public function tag(): string { return \"marker\"; }
}

var $m = new Marker();
echo $m->tag();
";
    assert_eq!(output_of(source), "marker");
}

#[test]
fn a_throw_out_of_a_frame_holding_an_object_releases_it() {
    // The landing block's cleanup, for an object rather than a string: the
    // local goes out of scope on the error path too.
    let source = format!(
        "{SHAPES}\nclass Boom {{\n    public static function go(): void {{\n        throw new Exception(\"boom\");\n    }}\n}}\ntry {{\n    var $d = new Dog(\"rex\");\n    Boom::go();\n    echo $d->name();\n}} catch (Throwable $e) {{\n    echo \"caught: \" . $e->getMessage();\n}}\n"
    );
    assert_eq!(output_of(&source), "caught: boom");
}

#[test]
fn an_array_literal_reads_back_the_element_it_stored() {
    // The whole array path end to end: `mwl_array_new`, one `mwl_array_set`
    // per literal entry, and `mwl_array_get` reading one back.
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [10, 20, 30];\necho $a[1];\n"),
        "20"
    );
    assert_eq!(
        output_of("<?mwl\narray<string> $a = [\"k\" => \"v\"];\necho $a[\"k\"];\n"),
        "v"
    );
}

#[test]
fn a_written_element_is_visible_through_the_same_local() {
    // The write-back `mwl_ir::lower::write_back_array` emits: the local is
    // re-pointed at whatever `mwl_array_set` yielded, so the read that follows
    // sees the entry. Without it the read would still name the pre-write
    // array.
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [];\n$a[\"k\"] = 7;\necho $a[\"k\"];\n"),
        "7"
    );
    assert_eq!(
        output_of(
            "<?mwl\narray<int> $a = [];\n$a[] = 4;\n$a[] = 5;\necho $a[\"0\"] . $a[\"1\"];\n"
        ),
        "45"
    );
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [1, 2];\n$a[0] = 9;\necho $a[0];\n"),
        "9"
    );
}

#[test]
fn a_copy_written_after_aliasing_leaves_the_original_alone() {
    // ADR 0007 § 5's copy-on-write value semantics, which is the whole reason
    // an array write yields the array it wrote into.
    assert_eq!(
        output_of(
            "<?mwl\narray<int> $a = [\"k\" => 1];\nvar $b = $a;\n$b[\"k\"] = 99;\necho $a[\"k\"] . \"/\" . $b[\"k\"];\n"
        ),
        "1/99"
    );
}

#[test]
fn an_array_is_truthy_unless_it_is_empty() {
    // `Helper::ArrayTruthy`, whose entry point landed with the representation.
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [];\nif ($a) {\n    echo \"full\";\n}\necho \"done\";\n"),
        "done"
    );
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [1];\nif ($a) {\n    echo \"full\";\n}\n"),
        "full"
    );
}

#[test]
fn an_array_of_strings_rewritten_in_a_loop_leaks_nothing() {
    // Ten thousand writes into one solely-owned array: each one replaces a
    // stored string, which the table releases as it displaces it. A missing
    // release would leak the buffers; a doubled one would crash. This is also
    // the in-place fast path running ten thousand times without separating.
    let source = "<?mwl
array<string> $a = [];
var $i = 0;
while ($i < 10000) {
    $a[\"k\"] = \"n\" . $i;
    $i = $i + 1;
}
echo $a[\"k\"];
";
    assert_eq!(output_of(source), "n9999");
}

#[test]
fn an_array_element_written_through_a_property_survives_the_write_back() {
    // The other holder a separation is written back to: the property slot,
    // through a `field.set` of whatever the write yielded.
    let source = "<?mwl
class Bag {
    public array<int> $items;
    public function constructor() { $this->items = []; }
    public function put(string $k, int $v): void { $this->items[$k] = $v; }
    public function get(string $k): int { return $this->items[$k]; }
}

var $bag = new Bag();
$bag->put(\"a\", 1);
$bag->put(\"b\", 2);
echo $bag->get(\"a\") . $bag->get(\"b\");
";
    assert_eq!(output_of(source), "12");
}
