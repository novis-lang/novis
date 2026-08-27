//! Integer arithmetic whose edges diverge from a native instruction — `%`'s signs, a divisor that throws rather than traps, and ADR 0007 § 4's overflow throw.
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

#[test]
fn an_integer_division_emits_both_of_its_representations() {
    // ADR 0007 § 4 types `int / int` as `int|float`, PHP-exact, so which of the
    // two a given pair produces is a *runtime* question and
    // `Emitter::emit_int_div` answers it with a branch rather than with a type.
    // Both arms are asked, because either alone is satisfied by an
    // implementation that always tags its result the same way.
    assert_eq!(
        output_of(
            "<?mwl
var $q = 7 / 2;
echo $q;
"
        ),
        "3.5"
    );
    assert_eq!(
        output_of(
            "<?mwl
var $q = 6 / 3;
echo $q;
"
        ),
        "2"
    );
    assert_eq!(
        output_of(
            "<?mwl
uint $a = 7;
uint $b = 2;
var $q = $a / $b;
echo $q;
"
        ),
        "3.5"
    );
    // `i64::MIN / -1` is the signed overflow `sdiv` traps on, and it takes the
    // *float* arm rather than becoming a second throw: the quotient is not an
    // `int` at all, and PHP answers this value for it.
    assert_eq!(
        output_of(
            "<?mwl
int $a = -9223372036854775807 - 1;
int $b = -1;
var $q = $a / $b;
echo $q;
"
        ),
        "9.2233720368548E+18"
    );
}

#[test]
fn an_integer_addition_traps_on_overflow() {
    // ADR 0007 § 4's overflow row, at the bound and one step inside it. The check is
    // the machine's own overflow flag (`Emitter::emit_checked_int_arith`), so
    // the not-taken side is a predicted branch and the taken one raises spec
    // § 10's `ArithmeticError` from a baked-in descriptor, exactly as the zero
    // divisor above does.
    assert_eq!(
        output_of(
            "<?mwl
int $a = 9223372036854775806;
int $b = 1;
echo $a + $b;
"
        ),
        "9223372036854775807"
    );
    assert_eq!(
        output_of(
            "<?mwl
int $a = 9223372036854775807;
int $b = 1;
try {
  echo $a + $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer addition overflowed"
    );
    // The unsigned bound is its own, and it is a carry out of bit 63 rather
    // than a sign flip: `uadd_overflow`, not `sadd_overflow` read differently.
    assert_eq!(
        output_of(
            "<?mwl
uint $a = 18446744073709551615;
uint $b = 1;
try {
  echo $a + $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer addition overflowed"
    );
}

#[test]
fn an_integer_subtraction_and_multiplication_trap_on_overflow() {
    // The other two binary rows and the unary one, each named at the value
    // that has no answer. `-i64::MIN` is the whole reason unary `-` joined the
    // checked set: it is the one `int` whose negation is not an `int`.
    assert_eq!(
        output_of(
            "<?mwl
int $a = -9223372036854775807 - 1;
int $b = 1;
try {
  echo $a - $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer subtraction overflowed"
    );
    assert_eq!(
        output_of(
            "<?mwl
int $a = 4611686018427387904;
int $b = 2;
try {
  echo $a * $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer multiplication overflowed"
    );
    assert_eq!(
        output_of(
            "<?mwl
int $a = -9223372036854775807 - 1;
try {
  echo -$a;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer negation overflowed"
    );
    // A `uint` has no negative side at all, so every non-zero one negates to
    // nothing while `0` still answers itself.
    assert_eq!(
        output_of(
            "<?mwl
uint $a = 0;
echo -$a;
"
        ),
        "0"
    );
    assert_eq!(
        output_of(
            "<?mwl
uint $a = 1;
try {
  echo -$a;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer negation overflowed"
    );
    // One step inside each bound still answers, so "refuse everything" is not
    // a way to pass the three assertions above.
    assert_eq!(
        output_of(
            "<?mwl
int $a = -9223372036854775807;
int $b = 1;
echo $a - $b;
"
        ),
        "-9223372036854775808"
    );
    assert_eq!(
        output_of(
            "<?mwl
int $a = 4611686018427387903;
int $b = 2;
echo $a * $b;
"
        ),
        "9223372036854775806"
    );
}
