//! Integer arithmetic whose edges diverge from a native instruction — `%`'s signs, and a zero divisor that throws rather than traps.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn integer_modulo_answers_php_s_value_for_every_sign() {
    // The sign rules are PHP's: the result takes the *dividend's* sign, which
    // is what `srem` already gives, so these are here to hold that rather than
    // to describe a conversion.
    assert_eq!(output_of("<?mwl\nint $q = 17 % 5;\necho $q;\n"), "2");
    assert_eq!(output_of("<?mwl\nint $q = -17 % 5;\necho $q;\n"), "-2");
    assert_eq!(output_of("<?mwl\nint $q = 17 % -5;\necho $q;\n"), "2");
    assert_eq!(
        output_of("<?mwl\nuint $a = 17;\nuint $b = 5;\necho $a % $b;\n"),
        "2"
    );
}

#[test]
fn a_modulo_by_minus_one_answers_zero_rather_than_trapping() {
    // `i64::MIN % -1` is the second input `srem` traps on, and a trap takes
    // the whole process down. It is *not* an overflow — `x % -1` is `0` for
    // every `x`, which is representable, and PHP 8 answers `0` — so
    // `Emitter::emit_int_mod` rewrites the divisor rather than throwing.
    // Reaching `i64::MIN` needs the negation, since the literal itself is out
    // of `int`'s range.
    assert_eq!(
        output_of("<?mwl\nint $a = -9223372036854775807 - 1;\nint $b = -1;\necho $a % $b;\n"),
        "0"
    );
}

#[test]
fn a_modulo_by_zero_throws_arithmetic_error_rather_than_trapping() {
    // The whole reason this operator waited for the throw path: `srem` on a
    // zero divisor traps, which is a request-isolation failure rather than a
    // wrong answer. The message is PHP's own, and the class is spec § 10's
    // `ArithmeticError` — which is why the raise names a descriptor instead of
    // going through a helper's `Fault`, whose bare message could only ever be
    // promoted to `RuntimeError`.
    assert_eq!(
        output_of(
            "<?mwl
int $b = 0;
try {
  echo 10 % $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Modulo by zero"
    );
}
