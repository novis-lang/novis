//! `rule:errors/on-limit`'s call-stack limit, riding the safepoint's emit site.
//!
//! The bounds are *armed* rather than discovered here — `Ctx::arm_stack_limit`
//! with a base this frame measures — for the reason `nvs_runtime::ctx`'s own
//! module doc gives: what is under test is that the compare is emitted, that a
//! leaf skips it, and that a runaway reaches a report, none of which should
//! depend on how much stack `cargo test` happened to hand this thread.
//!
//! **The runaway's base must be an address on the real stack**, which is why
//! CI runs this crate under ASAN with `detect_stack_use_after_return` off and
//! the rest of the workspace with it on. That option moves an address-taken
//! local into a heap fake frame, so `&ctx` would arm a window megabytes from
//! the stack the JIT'd frames walk down — and a JIT frame is not instrumented,
//! so there is no spelling of the measurement that survives it.

mod common;

use common::*;

/// A class whose two static methods differ in exactly one thing: `deeper`
/// calls something and `twice` does not, so one carries the check and the
/// other is elided.
const TWO_METHODS: &str = "<?nvs
class Depth {
    public static function twice(int $n): int {
        return $n * 2;
    }
    public static function deeper(int $n): int {
        return Depth::twice($n) + 1;
    }
}
";

#[test]
fn a_function_entry_checks_the_stack_limit() {
    // Bounds no stack pointer can satisfy: every address is below them, so the
    // compare at entry has to fail on the first frame entered. The script
    // frame `echo`es, which is a call, so it is not a leaf and carries one.
    let mut ctx = Ctx::buffered();
    ctx.arm_stack_limit(usize::MAX, 0);
    let status = run_with(&mut ctx, "<?nvs\necho \"never\";\n").unwrap_err();

    // Below the *floor*, so this is the tier no `catch` sees.
    assert_eq!(status, FATAL);
    assert_eq!(
        ctx.take_pending().as_deref(),
        Some("the request exceeded its call-stack limit")
    );
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));

    // The same impossible bounds against a method that calls one thing: the
    // check is at the entry of any non-leaf function, not only the script's.
    let unit = compile(TWO_METHODS).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    ctx.arm_stack_limit(usize::MAX, 0);
    assert_eq!(
        unit.call_static(&mut ctx, "Depth", "deeper", &[Value::int(21)])
            .expect("`deeper` was compiled")
            .err(),
        Some(FATAL)
    );
}

#[test]
fn a_leaf_function_under_the_slack_emits_no_stack_check() {
    // Bounds nothing can satisfy, and a function that calls nothing. It runs,
    // which is only possible if no compare was emitted at all — its caller
    // passed one with `STACK_RESERVE` still underneath, which is the whole of
    // the elision's justification.
    let unit = compile(TWO_METHODS).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    ctx.arm_stack_limit(usize::MAX, 0);

    // The `21` is load-bearing twice over: it is what proves the body ran at
    // all, and — because this is one of the few places a compiled method is
    // reached from Rust — that the argument arrived in the slot the callee
    // reads. `crates/nvs-codegen/tests/calls.rs` holds the dedicated guard.
    let answer = unit
        .call_static(&mut ctx, "Depth", "twice", &[Value::int(21)])
        .expect("`twice` was compiled")
        .expect("the leaf ran");
    assert_eq!(answer.as_int(), Some(42));
    assert!(ctx.pending().is_none());
}

#[test]
fn a_runaway_recursion_reports_a_limit_rather_than_faulting() {
    let mut ctx = Ctx::buffered();
    // A ceiling of twice the reserve puts the soft address one reserve below
    // this frame, so the recursion trips after ~256 KiB of real stack and
    // never approaches whatever this thread actually has.
    let here = std::ptr::from_ref(&ctx) as usize;
    ctx.arm_stack_limit(here, 2 * STACK_RESERVE);

    let status = run_with(
        &mut ctx,
        "<?nvs
class Runaway {
    public static function down(int $n): int {
        return Runaway::down($n + 1);
    }
}
echo Runaway::down(0);
",
    )
    .unwrap_err();

    // The *soft* tier: still above the floor, so it is a throw a program could
    // have caught rather than the end of the request.
    assert_eq!(status, THROWN);
    assert_eq!(
        ctx.take_pending().as_deref(),
        Some("the call stack is too deep")
    );
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b""[..]));
}
