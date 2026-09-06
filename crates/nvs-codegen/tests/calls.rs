//! A static call end to end — resolution, recursion, declared defaults, `rule:testing/debug-probes`'s call tracing, and `rule:errors/propagation`'s panic containment.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

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

/// Three parameters whose values are told apart from each other and from
/// whatever a slice's neighbour happens to hold, and one method per class that
/// answers through `static::` — so what reached the receiver slot is readable
/// from the outside.
///
/// `Renamed` re-declares `named` rather than inheriting it because
/// `Unit::call_static` names a *compiled function* and fills the slot with
/// that same class: an inherited body is compiled once, under the class that
/// declares it.
const SLOTS: &str = "<?nvs
class Slots {
    public static function first(int $a, int $b, int $c): int {
        return $a;
    }
    public static function second(int $a, int $b, int $c): int {
        return $b;
    }
    public static function third(int $a, int $b, int $c): int {
        return $c;
    }
    public static function tag(): string { return \"base\"; }
    public static function named(): string { return static::tag(); }
}
class Renamed extends Slots {
    public static function tag(): string { return \"derived\"; }
    public static function named(): string { return static::tag(); }
}
";

/// A hand-built call delivers each argument to the parameter it was written
/// for — the claim `nvs_runtime::NvsFn` makes about slot 0 being an implicit
/// receiver, checked from the only side that can get it wrong.
///
/// **Every value here is non-zero and every one is different.** The bug this
/// guards against shifts every parameter by one slot, so a caller reads its
/// neighbour and then one `Value` past the end of its own slice — which is
/// invisible against a `0` argument, and invisible against two arguments that
/// happen to be equal. It is a live out-of-bounds read either way: under ASAN
/// the trailing slot reads back as an int-tagged zero, and under an ordinary
/// build as whatever was next on the stack.
#[test]
fn a_hand_built_call_delivers_every_argument_to_its_own_parameter() {
    let unit = compile(SLOTS).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    let args = [Value::int(11), Value::int(22), Value::int(33)];

    for (method, expected) in [("first", 11), ("second", 22), ("third", 33)] {
        let answer = unit
            .call_static(&mut ctx, "Slots", method, &args)
            .expect("the method was compiled")
            .expect("the method ran");
        assert_eq!(
            answer.as_int(),
            Some(expected),
            "`{method}` read the wrong argument slot"
        );
    }
}

/// The receiver slot is not filler: a `static` method reached from Rust can
/// still answer `static::`, which is only true if slot 0 carries the *called*
/// class rather than a null.
///
/// Without this, filling slot 0 with `Value::null()` would pass every other
/// test in this file and hand late static binding a wild pointer.
#[test]
fn a_hand_built_static_call_carries_the_class_it_was_called_on() {
    let unit = compile(SLOTS).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);

    for (class, tag) in [("Slots", &b"base"[..]), ("Renamed", &b"derived"[..])] {
        let answer = unit
            .call_static(&mut ctx, class, "named", &[])
            .expect("the method was compiled")
            .expect("the method ran");
        assert_eq!(answer.as_str_bytes(), Some(tag));
    }
}

/// A short argument list is refused rather than read past, which is the other
/// half of the receiver-slot defect: `nvs_runtime::call` is handed a pointer
/// and never a length, so nothing below this can tell three slots from two.
///
/// The instance path refuses the same thing
/// (`nvs_runtime::construct_and_call`); this is the static one.
#[test]
#[should_panic(expected = "declares 3 parameter(s) and this call supplies 2")]
fn a_hand_built_call_short_of_an_argument_is_refused() {
    let unit = compile(SLOTS).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    let _ = unit.call_static(
        &mut ctx,
        "Slots",
        "first",
        &[Value::int(11), Value::int(22)],
    );
}

/// And a long one, which reads no further than the callee's own frame but
/// means the caller and the callee disagree about the signature.
#[test]
#[should_panic(expected = "declares 0 parameter(s) and this call supplies 1")]
fn a_hand_built_call_with_an_argument_too_many_is_refused() {
    let unit = compile(SLOTS).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    let _ = unit.call_static(&mut ctx, "Slots", "tag", &[Value::int(11)]);
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
    // `rule:testing/debug-probes`'s call-site pair. `quadruple` is entered first and left
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

// A *non-`OK`* traced exit has no fixture here, and deliberately not: the only
// stop reachable without a `throw` is a pending safepoint, which fires at the
// script frame's own entry poll before any call is reached. `nvs_probe_call_
// exit`'s own unit test covers that it records the status it is handed; that
// the probe is emitted *before* `rule:errors/propagation`'s compare-and-branch — so a thrown
// exit is traced rather than skipped along with the rest of the frame —
// belongs to an end-to-end fixture with `throw`.
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
    // `rule:errors/propagation`'s compare-and-branch doing its job across an Novis-level frame:
    // the callee's entry safepoint refuses, and the status travels up through
    // the caller unchanged rather than being swallowed at the call site.
    let mut ctx = Ctx::buffered();
    ctx.request_safepoint(SafepointFlags::CPU_LIMIT);
    let status = run_with(&mut ctx, CALLS).unwrap_err();

    assert_eq!(status, FATAL);
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
}
