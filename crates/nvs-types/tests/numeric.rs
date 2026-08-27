//! `int`/`uint` arithmetic and integer-literal placement — ADR 0007 § 4.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

/// ADR 0035: a condition — `if`/`while`/`for`'s middle clause/`?:`/`&&`/
/// `||`/`!` — accepts any type at all, judged by PHP's full truthy table
/// at runtime, never `E_TYPE_MISMATCH` for not already being `bool`.
/// `bool`-typed positions elsewhere (a parameter, a property, `== `/`===`)
/// are unaffected and still need an explicit `as bool` or comparison.
#[test]
fn a_non_bool_condition_is_never_a_type_mismatch() {
    let diags = check_in_method(
        "string $s = \"\";\nint $n = 0;\narray<int> $rows = [];\n\
         if ($s) {}\nwhile ($n) {}\nfor (; $rows; ) {}\n\
         bool $ok = $s && $n || !$rows;\necho $ok ? 1 : 2;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn int_plus_uint_is_diagnosed() {
    let diags = check_in_method("int $a = 1;\nuint $b = 1;\nint $c = $a + $b;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INT_UINT_ARITHMETIC))
    );
}

#[test]
fn an_integer_literal_assigned_into_a_uint_local_is_fine() {
    // ADR 0007 § 4: a plain integer literal means `uint` exactly where
    // that's the expected type — this must not be diagnosed as `int`
    // vs. `uint` mismatch.
    let diags = check_in_method("uint $n = 1;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0007 § 4: a literal larger than `i64::MAX` (`9223372036854775807`)
/// but still within `uint`'s `u64` range is legal exactly where `uint` is
/// expected.
#[test]
fn a_literal_too_large_for_int_assigned_into_a_uint_local_is_fine() {
    let diags = check_in_method("uint $n = 9223372036854775808;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The same literal has no legal home as a plain `int` — ADR 0007 § 4's
/// "otherwise a diagnostic saying exactly that."
#[test]
fn a_literal_too_large_for_int_assigned_into_an_int_local_is_diagnosed() {
    let diags = check_in_method("int $n = 9223372036854775808;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
        "{diags:?}"
    );
}

/// Even a bare, un-targeted literal with no `uint` in sight still gets
/// range-checked against `int`'s own half of the range.
#[test]
fn a_literal_too_large_for_int_with_no_expected_type_is_diagnosed() {
    let diags = check_in_method("echo 9223372036854775808;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
        "{diags:?}"
    );
}

/// A literal past even `uint`'s full `u64` range is always out of range,
/// regardless of what's expected.
#[test]
fn a_literal_too_large_for_uint_is_diagnosed_even_where_uint_is_expected() {
    let diags = check_in_method("uint $n = 99999999999999999999999999;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
        "{diags:?}"
    );
}

/// The magnitude check applies to every base `nvs-syntax`'s lexer cooks,
/// not just decimal — a hex literal one bit past `uint`'s 64-bit range.
#[test]
fn a_hex_literal_too_large_for_uint_is_diagnosed() {
    let diags = check_in_method("uint $n = 0x1_0000_0000_0000_0000;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
        "{diags:?}"
    );
}

/// A negative literal (`-5`) needs no magnitude-specific diagnostic at
/// all: the inner literal always types as plain `int` (the wrapping
/// `Unary::Neg` node checks it with no expected type), so assigning that
/// `int` into a `uint` target is the ordinary `int`-vs-`uint`
/// `E_TYPE_MISMATCH`, not `E_INT_LITERAL_OUT_OF_RANGE`.
#[test]
fn a_negative_literal_into_a_uint_local_is_an_ordinary_type_mismatch() {
    let diags = check_in_method("uint $n = -5;\n");
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
        "{diags:?}"
    );
}

/// ADR 0007 § 4's "either operand a `float`" row, read as a *widening*: a
/// mixed-representation arithmetic pair produces the wider of the two, so it
/// lands in a `float` binding and is refused at an `int` one. Both operand
/// orders and both integer tags, because the row is stated over "either
/// operand" and says nothing about which side it is on.
#[test]
fn a_mixed_numeric_pair_widens_the_narrower_operand() {
    let widened = check_in_method(
        "int $i = 1;\nuint $u = 1;\nfloat $f = 1.5;\n\
         float $a = $i + $f;\nfloat $b = $f * $i;\n\
         float $c = $u - $f;\nfloat $d = $f + $u;\n",
    );
    assert!(!widened.has_errors(), "{widened:?}");

    // The widening only ever runs toward `float`. Nothing narrows the pair
    // back to fit a declaration — that would be the implicit conversion
    // ADR 0007 § 2 has exactly one of, and this is not it.
    let narrowed = check_in_method("int $i = 1;\nfloat $f = 1.5;\nint $n = $i + $f;\n");
    assert!(
        narrowed
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{narrowed:?}"
    );
}

/// The comparison rows widen the same pairs the arithmetic rows do — and one
/// more, since `int` against `uint` is exact in the mathematical integers and
/// is the pair § 4 refuses to *add*. What they produce is `bool`, never the
/// widened operand type, which is the half a table keyed on the operands
/// alone would get wrong.
#[test]
fn a_mixed_numeric_comparison_widens_the_same_way() {
    let compared = check_in_method(
        "int $i = 1;\nuint $u = 1;\nfloat $f = 1.5;\n\
         bool $a = $i < $f;\nbool $b = $f >= $u;\n\
         bool $c = $i <= $u;\nbool $d = $u > $i;\n",
    );
    assert!(!compared.has_errors(), "{compared:?}");

    // The same `int`/`uint` pair, under `+`, is still the refusal its own
    // neighbour above pins — comparing is widened, arithmetic is not.
    let added = check_in_method("int $i = 1;\nuint $u = 1;\nint $n = $i + $u;\n");
    assert!(
        added
            .iter()
            .any(|d| d.code == Some(code::E_INT_UINT_ARITHMETIC)),
        "{added:?}"
    );

    let mistyped = check_in_method("int $i = 1;\nfloat $f = 1.5;\nfloat $n = $i < $f;\n");
    assert!(
        mistyped
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{mistyped:?}"
    );
}

/// ADR 0007 § 4: `int / int` is the union `int|float` — PHP-exact, `6/3`
/// being an integer and `7/2` a float — and § 2's one implicit conversion is
/// what absorbs it, **at the binding**, never at the operator. So the
/// quotient lands in a declared `float` and is a diagnostic at a declared
/// `int`, which is the ADR's own worked example both ways round.
#[test]
fn an_integer_division_is_a_union_widened_at_its_binding() {
    let widened = check_in_method(
        "int $a = 7;\nint $b = 2;\nuint $c = 7;\nuint $d = 2;\n\
         float $q = $a / $b;\nfloat $r = $c / $d;\n",
    );
    assert!(!widened.has_errors(), "{widened:?}");

    // Widening at the binding is the only thing that absorbs the union: the
    // operator itself keeps it, so an `int` declaration cannot take it.
    let kept = check_in_method("int $a = 7;\nint $b = 2;\nint $q = $a / $b;\n");
    assert!(
        kept.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{kept:?}"
    );

    let kept_uint = check_in_method("uint $a = 7;\nuint $b = 2;\nuint $q = $a / $b;\n");
    assert!(
        kept_uint
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{kept_uint:?}"
    );
}
