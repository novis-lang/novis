//! A static call end to end — resolution, recursion, declared defaults, ADR 0018's call tracing, and ADR 0002's panic containment.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

/// The `examples/calls.nvs` acceptance fixture, as an inline source string.
const CALLS: &str = "<?nvs
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
            "<?nvs\nclass C {\n    public static function first(): int {\n        return C::second() + 1;\n    }\n    public static function second(): int {\n        return 41;\n    }\n}\necho C::first();\n"
        ),
        "42"
    );
}

#[test]
fn a_recursive_call_terminates_and_returns_the_right_value() {
    assert_eq!(
        output_of(
            "<?nvs\nclass F {\n    public static function fact(int $n): int {\n        if ($n < 2) {\n            return 1;\n        }\n        return $n * F::fact($n - 1);\n    }\n}\necho F::fact(10);\n"
        ),
        "3628800"
    );
}

/// A parameter default is materialized *at the call site* — the callee keeps
/// exactly one arity, so nothing in this crate knows defaults exist. See
/// `nvs_types::defaults` for why the caller does the work.
#[test]
fn a_call_that_omits_a_defaulted_parameter_passes_the_declared_default() {
    assert_eq!(
        output_of(
            "<?nvs\nclass Box {\n    public static function scale(int $n, int $by = 3): int {\n        return $n * $by;\n    }\n}\necho Box::scale(5), \",\", Box::scale(5, 2);\n"
        ),
        "15,10"
    );
}

/// The refcounted case: a defaulted `string` is a fresh `ConstStr` the callee
/// owns and releases at its own exit, exactly like a written literal argument.
#[test]
fn a_defaulted_string_parameter_reaches_the_callee_as_a_real_string() {
    assert_eq!(
        output_of(
            "<?nvs\nclass Greeter {\n    public static function greet(string $name, string $sep = \": \"): string {\n        return $name . $sep . \"hi\";\n    }\n}\necho Greeter::greet(\"ana\"), \"|\", Greeter::greet(\"bo\", \"-\");\n"
        ),
        "ana: hi|bo-hi"
    );
}

/// Two defaults in a row, and the middle one supplied — the arrangement that
/// would break if the materialized constants were appended in the wrong order
/// or aligned from the wrong end.
#[test]
fn a_call_may_supply_some_defaulted_parameters_and_omit_the_rest() {
    assert_eq!(
        output_of(
            "<?nvs\nclass Sum {\n    public static function of(int $a, int $b = 20, int $c = 300): int {\n        return $a + $b + $c;\n    }\n}\necho Sum::of(1), \",\", Sum::of(1, 2), \",\", Sum::of(1, 2, 3);\n"
        ),
        "321,303,6"
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
// script frame's own entry poll before any call is reached. `nvs_probe_call_
// exit`'s own unit test covers that it records the status it is handed; that
// the probe is emitted *before* ADR 0002's compare-and-branch — so a thrown
// exit is traced rather than skipped along with the rest of the frame — gets
// its end-to-end fixture with `throw`, which is the next slice.
#[test]
fn a_second_script_runs_after_a_contained_helper_panic() {
    // M3's "leaves the process able to run the next one", which a one-shot
    // `nvs run` cannot show: two compiles and two runs in *this* process, the
    // first faulted deliberately. `benches/abi-probe`'s
    // `the_jit_is_still_usable_after_a_contained_panic` is the ABI-level
    // version of the same claim over hand-built frames; this is the compiler's.
    // Three `echo`s, so the fault lands on the second one — see
    // `FaultSite::HelperPanic` for why it is not the first.
    let source = "<?nvs\necho \"start\\n\";\necho \"middle\\n\";\necho \"end\\n\";\n";

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
    // ADR 0002's compare-and-branch doing its job across an Novis-level frame:
    // the callee's entry safepoint refuses, and the status travels up through
    // the caller unchanged rather than being swallowed at the call site.
    let mut ctx = Ctx::buffered();
    ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
    let status = run_with(&mut ctx, CALLS).unwrap_err();

    assert_eq!(status, FATAL);
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
}
