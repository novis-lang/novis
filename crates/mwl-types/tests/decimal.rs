//! `decimal` — ADR 0054 §§ 1-3, the checker half: literal placement, the
//! arithmetic table, and the layout bound a placed literal must fit.
//!
//! The runtime half (i128 arithmetic with scale reconciliation, `ArithmeticError`
//! on overflow, half-even division) is M4's and has no test here yet — see that
//! ADR's *Verification* section for what it owes.

mod common;

use common::*;
use mwl_diagnostics::code;

/// ADR 0054 § 2: a fractional literal is untyped until placed, and takes
/// `decimal` or `float` from the position it lands in. Both spellings of the
/// same digits are legal, and neither needs a suffix.
#[test]
fn a_fractional_literal_takes_decimal_or_float_from_its_target() {
    let diags = check_in_method("decimal $price = 19.99;\nfloat $ratio = 19.99;\n");
    assert!(!diags.has_errors(), "{diags:?}");

    // And the placement is real, not a formality: once `as decimal` has
    // placed it, the value is a `decimal` and a `float` target refuses it.
    let diags = check_in_method("float $f = 19.99 as decimal;\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// ADR 0054 § 2's two positions that have no target: `var` infers `float`,
/// and `as decimal` supplies one.
#[test]
fn var_infers_float_without_a_target_and_decimal_under_a_conversion() {
    let diags = check_in_method("var $x = 19.99;\nfloat $f = $x;\n");
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_in_method("var $y = 19.99 as decimal;\ndecimal $d = $y;\n");
    assert!(!diags.has_errors(), "{diags:?}");

    // The same `var $y` is *not* a `float`, which is what makes the inference
    // above load-bearing rather than incidental.
    let diags = check_in_method("var $y = 19.99 as decimal;\nfloat $f = $y;\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// ADR 0054 § 2: `expr as T` is itself a placing position, so a literal
/// written directly under one is exact to the full 29 significant digits
/// rather than round-tripping through an `f64` first — this 25-digit literal
/// is the case that fails if the placing rule is dropped.
#[test]
fn a_twenty_five_digit_literal_under_as_decimal_is_exact() {
    let diags = check_in_method("decimal $d = 1.234567890123456789012345 as decimal;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0054 § 1: 96 bits of mantissa, and no more.
#[test]
fn a_literal_wider_than_96_bits_of_mantissa_is_diagnosed() {
    let diags = check_in_method("decimal $d = 123456789012345678901234567890.0;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DECIMAL_LITERAL_OUT_OF_RANGE)),
        "{diags:?}"
    );
}

/// ADR 0054 § 1: a scale of 0 to 28, and no more. A 29th digit after the
/// point has nowhere to live even though the mantissa itself is tiny.
#[test]
fn a_literal_with_a_scale_past_28_is_diagnosed() {
    let diags = check_in_method("decimal $d = 0.12345678901234567890123456789;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DECIMAL_LITERAL_OUT_OF_RANGE)),
        "{diags:?}"
    );
}

/// An integer literal placed at `decimal` is exact in 96 bits (§ 4's first
/// conversion row), so it is checked against *that* bound rather than `int`'s
/// 64-bit one — a value too large for `uint` is still a fine `decimal`.
#[test]
fn an_integer_literal_placed_at_decimal_gets_the_96_bit_bound() {
    let diags = check_in_method("decimal $d = 99999999999999999999999999;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0054 § 3: `decimal ⊕ float` is a compile error, the same rule and the
/// same reason as `int ⊕ uint`.
#[test]
fn decimal_plus_float_is_diagnosed() {
    let diags = check_in_method("decimal $d = 1.0;\nfloat $f = 1.5;\nvar $x = $d + $f;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DECIMAL_FLOAT_ARITHMETIC)),
        "{diags:?}"
    );
}

/// ADR 0054 § 3: comparison is permitted for exactly the reason arithmetic is
/// not — an exact comparison is always computable, even where no common
/// arithmetic type exists.
#[test]
fn decimal_compared_against_a_float_is_accepted() {
    let diags = check_in_method("decimal $d = 1.0;\nbool $ok = $d < 1.5;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0054 § 3: a `decimal` combined with an integer stays `decimal`, and
/// `decimal / decimal` is `decimal` rather than the `int|float` union `int /
/// int` yields.
#[test]
fn decimal_with_an_integer_operand_stays_decimal_and_division_is_not_a_union() {
    let diags = check_in_method(
        "decimal $d = 1.0;\nuint $q = 3;\n\
         decimal $total = $d * $q;\ndecimal $each = $total / $d;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0054 § 3's last row: `**` on a `decimal` is a compile error naming
/// `Core\Decimal::pow`.
#[test]
fn power_on_a_decimal_is_diagnosed() {
    let diags = check_in_method("decimal $d = 2.0;\nvar $x = $d ** 3;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DECIMAL_FLOAT_ARITHMETIC)),
        "{diags:?}"
    );
}

/// ADR 0054 § 2: `const decimal VAT = 0.19;` — a compile-time constant is one
/// of the three things *Context* names that a `Core\Decimal` class could never
/// be.
#[test]
fn a_decimal_class_constant_is_accepted() {
    let diags = check_src(
        "<?mwl\nclass Tax {\n  public const decimal VAT = 0.19;\n  \
         function m(): void { echo 1; }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}
