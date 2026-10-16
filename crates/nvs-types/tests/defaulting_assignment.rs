//! `??=`, `??+=`, `??-=` and `??.=` — their guarded read of the target, and
//! the type of the value each writes (`rule:expressions/defaulting-assignment`).
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

#[test]
fn coalesce_assign_with_a_non_null_right_side_is_typed_without_null() {
    let diags = check_in_method(concat!(
        "?int $n = null;\n",
        "int $m = $n ??= 1;\n",
        "array<?string> $a = [];\n",
        "string $s = $a[\"k\"] ??= \"d\";\n",
        "mixed $x = null;\n",
        "mixed $y = $x ??= 2;\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn coalesce_assign_with_a_nullable_right_side_keeps_the_target_type() {
    let diags = check_in_method(concat!(
        "?int $n = null;\n",
        "?int $d = null;\n",
        "?int $kept = $n ??= $d;\n",
        "int $m = $n ??= $d;\n",
    ));
    let mismatches: Vec<_> = diags
        .iter()
        .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .collect();
    assert_eq!(mismatches.len(), 1, "{diags:?}");
    assert_eq!(
        diags.iter().filter(|d| d.is_error()).count(),
        1,
        "{diags:?}"
    );
}

#[test]
fn coalesce_assign_returned_from_a_method_with_a_non_null_return_type_compiles() {
    let diags = check_src(concat!(
        "<?nvs\n",
        "class Post {\n",
        "  public ?string $title = null;\n",
        "  public array<int> $views = [];\n",
        "  public function title(): string {\n",
        "    return $this->title ??= \"Untitled\";\n",
        "  }\n",
        "  public function views(string $page): int {\n",
        "    return $this->views[$page] ??= 0;\n",
        "  }\n",
        "}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn coalesce_assign_reads_every_level_of_its_target_guarded() {
    let (diags, declared) = check_src_declared(concat!(
        "<?nvs\n",
        "class Blog {\n",
        "  public array<array<int>> $rows = [];\n",
        "  function m(array<string> $keys): void {\n",
        "    $this->rows[$keys[\"a\"]][\"j\"] ??= 5;\n",
        "    $this->rows[\"x\"][\"y\"] += 1;\n",
        "  }\n",
        "}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    // Every level of the target, the property at its root included.
    assert!(declared.guarded_target_read_at("$this->rows[$keys[\"a\"]][\"j\"]"));
    assert!(declared.guarded_target_read_at("$this->rows[$keys[\"a\"]]"));
    assert!(declared.guarded_target_read_at("$this->rows"));
    // A key is an ordinary read, and so is every level of another operator's target.
    assert!(!declared.guarded_target_read_at("$keys[\"a\"]"));
    assert!(!declared.guarded_target_read_at("$this->rows[\"x\"][\"y\"]"));
    assert!(!declared.guarded_target_read_at("$this->rows[\"x\"]"));
}

#[test]
fn coalesce_assign_through_a_nullable_row_is_still_refused() {
    let diags = check_in_method(concat!(
        "array<?array<int>> $g = [];\n",
        "$g[\"0\"][\"1\"] ??= 5;\n",
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SUBSCRIPT_ON_NON_ARRAY)),
        "{diags:?}"
    );
}

#[test]
fn every_other_compound_assignment_is_still_typed_as_its_target() {
    // Each right side has no `null` in its type, and each value keeps the
    // target's `null`, so each of the three lines is one mismatch.
    let diags = check_in_method(concat!(
        "?int $n = 1;\n",
        "int $a = $n = 5;\n",
        "int|float $f = 1;\n",
        "int $b = $f += 1;\n",
        "?string $s = null;\n",
        "string $c = $s = \"x\";\n",
    ));
    let mismatches = diags
        .iter()
        .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .count();
    assert_eq!(mismatches, 3, "{diags:?}");
}

/// The error codes `diags` carries, in order.
fn error_codes(diags: &nvs_diagnostics::Diagnostics) -> Vec<Option<nvs_diagnostics::Code>> {
    diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.code)
        .collect()
}

#[test]
fn a_defaulting_add_assign_is_typed_as_the_sum_over_a_zero_of_the_target_type() {
    // Each value is the sum, which is never `null`, so each fits an `int`.
    let diags = check_in_method(concat!(
        "?int $n = null;\n",
        "int $a = $n ??+= 1;\n",
        "int $b = $n ??-= 2;\n",
        "array<int> $counts = [];\n",
        "int $c = $counts[\"k\"] ??+= 1;\n",
        "array<array<int>> $grid = [];\n",
        "$grid[\"x\"][\"y\"] ??-= 1;\n",
        "int $plain = 3;\n",
        "int $d = $plain ??+= 1;\n",
        "mixed $any = null;\n",
        "$any ??+= 1;\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_defaulting_concat_assign_is_typed_as_the_concatenation_over_an_empty_string() {
    let diags = check_in_method(concat!(
        "?string $s = null;\n",
        "string $t = $s ??.= \"x\";\n",
        "array<string> $groups = [];\n",
        "string $g = $groups[\"k\"] ??.= \"a\";\n",
        "int|string $u = 1;\n",
        "$u ??.= \"b\";\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_defaulting_assignment_takes_a_uint_a_float_and_a_decimal_target() {
    // The bare `1` takes the target's own type, as it does under `+=`.
    let diags = check_in_method(concat!(
        "?uint $u = null;\n",
        "uint $v = $u ??+= 1;\n",
        "?float $f = null;\n",
        "float $g = $f ??-= 1.5;\n",
        "?decimal $d = null;\n",
        "decimal $e = $d ??+= 1.50;\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_defaulting_assignment_keeps_every_refusal_of_its_operator() {
    // Each pair is the plain operator over a target that cannot be `null`,
    // then the defaulting one over the nullable target. Both are refused, and
    // with the same codes.
    let pairs = [
        (
            "string $s = \"\";\n$s += 1;\n",
            "?string $s = null;\n$s ??+= 1;\n",
        ),
        (
            "int $n = 0;\n$n .= \"x\";\n",
            "?int $n = null;\n$n ??.= \"x\";\n",
        ),
        (
            "uint $u = 0;\nint $i = 1;\n$u += $i;\n",
            "?uint $u = null;\nint $i = 1;\n$u ??+= $i;\n",
        ),
        (
            "bool $b = false;\n$b -= 1;\n",
            "?bool $b = null;\n$b ??-= 1;\n",
        ),
        (
            "array<int> $a = [];\n$a .= \"x\";\n",
            "?array<int> $a = null;\n$a ??.= \"x\";\n",
        ),
    ];
    for (plain, defaulting) in pairs {
        let want = error_codes(&check_in_method(plain));
        let got = error_codes(&check_in_method(defaulting));
        assert!(!want.is_empty(), "{plain}");
        assert_eq!(got, want, "{defaulting}");
    }
}

#[test]
fn a_defaulting_concat_assign_keeps_a_qualifier() {
    let diags = check_src(concat!(
        "<?nvs\n",
        "class Post {\n",
        "  function m(tainted string $in): void {\n",
        "    ?tainted string $t = null;\n",
        "    tainted string $u = $t ??.= \"x\";\n",
        "    ?string $s = null;\n",
        "    $s ??.= $in;\n",
        "    string $plain = $t ??.= \"y\";\n",
        "  }\n",
        "}\n",
    ));
    // The tainted result does not fit `?string`, and it does not fit `string`.
    assert_eq!(
        error_codes(&diags),
        vec![Some(code::E_TYPE_MISMATCH), Some(code::E_TYPE_MISMATCH)],
        "{diags:?}"
    );
}

#[test]
fn a_coalesce_in_parentheses_is_still_not_an_assignment_target() {
    let (diags, _) = check_src_table_allowing_errors(concat!(
        "<?nvs\n",
        "class Shop {\n",
        "  function m(): void {\n",
        "    ?int $a = null;\n",
        "    ($a ?? 100) -= 1;\n",
        "  }\n",
        "}\n",
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INVALID_ASSIGN_TARGET)),
        "{diags:?}"
    );
}
