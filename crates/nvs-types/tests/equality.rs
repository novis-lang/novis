//! ADR 0090 § 2: two statically disjoint operands do not compile — and its
//! § 6, which makes a `switch` label and a `match` arm the same check against
//! their subject.
//!
//! The acceptances here carry as much weight as the refusals. The rule is
//! *disjointness*, not identity of types, so every shape ordinary code already
//! writes has to keep compiling; `nvs_types::expr::operators`' own
//! `types_are_disjoint` says why it answers "may overlap" for everything it
//! does not model.

mod common;

use common::{check_in_method, check_src};
use nvs_diagnostics::{Diagnostics, code};

fn refuses(diags: &Diagnostics) -> bool {
    diags
        .iter()
        .any(|d| d.code == Some(code::E_DISJOINT_EQUALITY))
}

/// The ADR's own worked example: the compiler already knows `"1" == 1` is
/// false, so the author did not mean to write it.
#[test]
fn a_disjoint_equality_does_not_compile() {
    let diags = check_in_method(
        "string $s = \"1\";\n    int $n = 1;\n    if ($s == $n) {\n      echo \"x\";\n    }\n",
    );
    assert!(refuses(&diags), "{diags:?}");
}

/// ...and the fix the same example names: convert once, deliberately, then
/// compare.
#[test]
fn converting_one_side_makes_the_same_comparison_compile() {
    let diags = check_in_method(
        "string $s = \"1\";\n    int $n = 1;\n    if ($s == ($n as string)) {\n      echo \"x\";\n    }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0009 makes `string` and `bytes` different types on purpose, so the
/// table refuses the comparison rather than deciding which encoding it meant.
#[test]
fn a_string_never_compares_against_bytes() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(string $s, bytes $b): void {\n    \
         if ($s == $b) {\n      echo \"x\";\n    }\n  }\n}\n",
    );
    assert!(refuses(&diags), "{diags:?}");
}

/// ADR 0010 § 3 already makes `$e as int` cheap, which is what the author
/// wanted here.
#[test]
fn an_enum_never_compares_against_its_underlying_integer() {
    let diags = check_src(
        "<?nvs\nenum Status: int { Active = 1, Banned = 2 }\nclass T {\n  \
         function m(Status $s): void {\n    if ($s == 1) {\n      echo \"x\";\n    }\n  }\n}\n",
    );
    assert!(refuses(&diags), "{diags:?}");
}

/// Two enums are two closed sets, so nothing inhabits both.
#[test]
fn two_different_enums_never_compare() {
    let diags = check_src(
        "<?nvs\nenum Status { Active, Banned }\nenum Color { Red, Blue }\nclass T {\n  \
         function m(Status $s, Color $c): void {\n    if ($s == $c) {\n      echo \"x\";\n    }\n  }\n}\n",
    );
    assert!(refuses(&diags), "{diags:?}");
}

#[test]
fn two_unrelated_classes_never_compare() {
    let diags = check_src(
        "<?nvs\nclass A {\n}\nclass B {\n}\nclass T {\n  \
         function m(A $a, B $b): void {\n    if ($a == $b) {\n      echo \"x\";\n    }\n  }\n}\n",
    );
    assert!(refuses(&diags), "{diags:?}");
}

/// A class against an ancestor of it compares — one object is both, which is
/// the whole of the table's class row.
#[test]
fn a_class_compares_against_its_own_ancestor() {
    let diags = check_src(
        "<?nvs\nclass A {\n}\nclass B extends A {\n}\nclass T {\n  \
         function m(A $a, B $b): void {\n    if ($a == $b) {\n      echo \"x\";\n    }\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// An interface names no instance of its own, so some third class the
/// comparison never mentions may implement it alongside the class — the check
/// sees two names and must not claim otherwise.
#[test]
fn an_interface_operand_is_never_disjoint_from_a_class() {
    let diags = check_src(
        "<?nvs\ninterface I {\n}\nclass C {\n}\nclass T {\n  \
         function m(I $i, C $c): void {\n    if ($i == $c) {\n      echo \"x\";\n    }\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The table's `null` row: the diagnostic says the type cannot hold `null`,
/// because the author is testing a binding that was never declared optional.
#[test]
fn a_non_nullable_type_against_null_does_not_compile() {
    let diags = check_src(
        "<?nvs\nclass A {\n}\nclass T {\n  function m(A $a): void {\n    \
         if ($a == null) {\n      echo \"x\";\n    }\n  }\n}\n",
    );
    assert!(refuses(&diags), "{diags:?}");
}

// `int`, `uint`, `float` and `decimal` are one domain, and that row is pinned
// end to end rather than at the checker alone:
// `tests/conformance/lang/equality-across-overlapping-types-still-compiles.nvst`
// compiles *and runs* every pairing among the four, which is the stronger
// assertion and the one home for the fact.

/// A `mixed` operand is § 5's runtime case, never § 2's compile error.
#[test]
fn a_mixed_operand_compares_against_anything() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(mixed $x, int $n): void {\n    \
         if ($x == $n) {\n      echo \"x\";\n    }\n    \
         if ($x == null) {\n      echo \"y\";\n    }\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A union compares against any type one of its members can hold — one
/// overlapping member is enough, which is exactly what keeps the null test in
/// the next test compiling.
#[test]
fn a_union_compares_against_any_type_one_member_holds() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(int|string $v, int $n): void {\n    \
         if ($v == $n) {\n      echo \"x\";\n    }\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0090 § 6: a `case` label is compared against the subject by the one
/// equality rule, so a disjoint label is § 2's refusal without the operator.
#[test]
fn a_disjoint_switch_label_does_not_compile() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(string $s): void {\n    \
         switch ($s) {\n      case 1:\n        echo \"x\";\n        break;\n    }\n  }\n}\n",
    );
    assert!(refuses(&diags), "{diags:?}");
}

#[test]
fn a_disjoint_match_arm_does_not_compile() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(string $s): void {\n    \
         string $r = match ($s) {\n      1 => \"a\",\n      default => \"b\",\n    };\n    \
         echo $r;\n  }\n}\n",
    );
    assert!(refuses(&diags), "{diags:?}");
}

/// § 6's exception, and the shape `examples/match.nvs` is written in: every
/// arm of a `match (true)` is a `bool` tested against a `bool`.
#[test]
fn a_match_over_true_is_unaffected() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(int $score): void {\n    \
         string $r = match (true) {\n      $score >= 90 => \"A\",\n      default => \"F\",\n    };\n    \
         echo $r;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}
