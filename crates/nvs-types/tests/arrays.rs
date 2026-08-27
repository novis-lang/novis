//! Array literals, element types at depth, and what may be a subscript — ADR 0007 § 5.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

#[test]
fn integer_division_into_a_plain_int_is_diagnosed() {
    let diags = check_in_method("int $n = 7 / 2;\n");
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn integer_division_into_a_union_target_is_fine() {
    let diags = check_in_method("int|float $n = 7 / 2;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn assigning_mixed_into_a_typed_local_is_diagnosed() {
    let diags =
        check_src("<?mwl\nclass T {\n  function m(mixed $m): void {\n    int $n = $m;\n  }\n}\n");
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn an_array_literal_element_mismatch_at_depth_one_is_diagnosed() {
    let diags = check_in_method(r#"array<int> $a = [1, "x"];"#);
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn an_array_literal_element_mismatch_at_depth_two_is_diagnosed() {
    let diags = check_in_method(r#"array<array<int>> $a = [[1, 2], [1, "x"]];"#);
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn an_array_literal_element_mismatch_at_depth_three_is_diagnosed() {
    let diags = check_in_method(r#"array<array<array<int>>> $a = [[[1], [1, "x"]]];"#);
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_correctly_typed_nested_array_literal_is_fine() {
    let diags = check_in_method("array<array<int>> $a = [[1, 2], [3]];\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

// ADR 0007 § 5: an array key is `int`, `uint`, or `string`; a `float`,
// `bool`, or `null` key is rejected outright.

#[test]
fn a_string_key_array_literal_is_fine() {
    let diags = check_in_method(r#"array<int> $a = ["x" => 1, "y" => 2];"#);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_int_key_array_literal_is_fine() {
    let diags = check_in_method(r#"array<int> $a = [5 => 1, 6 => 2];"#);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_dynamic_string_key_array_literal_is_fine() {
    let diags = check_in_method("string $k = \"x\";\narray<int> $a = [$k => 1];\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_float_key_array_literal_is_diagnosed() {
    let diags = check_in_method(r#"array<int> $a = [1.5 => 1];"#);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ARRAY_KEY_INVALID_TYPE)),
        "{diags:?}"
    );
}

#[test]
fn a_bool_key_array_literal_is_diagnosed() {
    let diags = check_in_method(r#"array<int> $a = [true => 1];"#);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ARRAY_KEY_INVALID_TYPE)),
        "{diags:?}"
    );
}

#[test]
fn a_null_key_array_literal_is_diagnosed() {
    let diags = check_in_method(r#"array<int> $a = [null => 1];"#);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ARRAY_KEY_INVALID_TYPE)),
        "{diags:?}"
    );
}

#[test]
fn a_bool_subscript_key_is_diagnosed() {
    let diags = check_in_method("array<int> $a = [1];\nbool $b = true;\n$a[$b] = 1;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ARRAY_KEY_INVALID_TYPE)),
        "{diags:?}"
    );
}

#[test]
fn an_int_subscript_key_is_fine() {
    let diags = check_in_method("array<int> $a = [1, 2, 3];\nint $x = $a[1];\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0069 § 2: PHP's array union operator is removed rather than migrated,
/// and the diagnostic names the member that replaces it.
#[test]
fn two_arrays_do_not_combine_with_plus() {
    let diags = check_in_method("array<int> $a = [1];\narray<int> $b = [2];\n$a + $b;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ARRAY_PLUS_UNSUPPORTED)),
        "{diags:?}"
    );
}

/// `+=` is the same operator, and reports the same thing **once** — the
/// recovery type is the array so the write-back check does not pile a type
/// mismatch on top of it.
#[test]
fn two_arrays_do_not_combine_with_plus_equals() {
    let diags = check_in_method("array<int> $a = [1];\narray<int> $b = [2];\n$a += $b;\n");
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_ARRAY_PLUS_UNSUPPORTED))
            .count(),
        1,
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 1, "{diags:?}");
}
