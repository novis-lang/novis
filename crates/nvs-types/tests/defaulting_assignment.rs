//! `??=` — its guarded read of the target, and the type of the value it
//! writes (`rule:expressions/defaulting-assignment`).
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
