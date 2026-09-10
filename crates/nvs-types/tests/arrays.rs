//! Array literals, element types at depth, and what may be a subscript — `rule:types/arrays`.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

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
        check_src("<?nvs\nclass T {\n  function m(mixed $m): void {\n    int $n = $m;\n  }\n}\n");
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

// `rule:types/arrays`: an array key is `int`, `uint`, or `string`; a `float`,
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

/// `rule:types/array-combination`: PHP's array union operator is removed rather than migrated,
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

// `rule:types/unions-and-mixed`'s assignability relation, at its one covariant
// name: `array<T>` accepts an argument whose element type widens to `T`. The
// reasoning, and why copy-on-write is what makes it sound, is
// `nvs_types::expr::assign::is_assignable`'s own doc comment.

/// `class T` with a method taking `params` and a caller writing `body` — a
/// parameter position, which `check_in_method` has no way to reach.
fn call_with(params: &str, body: &str) -> String {
    format!(
        "<?nvs\nclass T {{\n  static function m({params}): void {{}}\n  static function go(): void {{\n{body}\n  }}\n}}\n"
    )
}

#[test]
fn an_array_of_int_satisfies_an_array_of_int_or_string_parameter() {
    let src = call_with(
        "array<int|string> $items",
        "array<int> $a = [1, 2];\nT::m($a);",
    );
    let diags = check_src(&src);
    assert!(
        !diags.has_errors(),
        "an `array<int>` is what a caller means by an `array<int|string>` parameter: {diags:?}"
    );
}

#[test]
fn the_widening_accepts_strictly_more_programs_and_breaks_none_that_compile_today() {
    // *Strictly more*: every row here is a program the invariant relation
    // refused, plus the invariant row itself, which is what says the widening
    // is an addition rather than a replacement. Counted rather than read off a
    // line, so a relation that answers plausibly row by row still fails.
    let widens = [
        ("array<int|string>", "array<int> $a = [1];"),
        ("array<int|float>", "array<int> $a = [1];"),
        ("array<mixed>", "array<int> $a = [1];"),
        ("array<array<int|string>>", "array<array<int>> $a = [[1]];"),
        ("array<int>", "array<int> $a = [1];"),
    ];
    let accepted = widens
        .iter()
        .filter(|(param, decl)| {
            let src = call_with(&format!("{param} $items"), &format!("{decl}\nT::m($a);"));
            !check_src(&src).has_errors()
        })
        .count();
    assert_eq!(
        accepted,
        widens.len(),
        "every element type that widens to the parameter's is accepted, at depth too"
    );

    // *Breaks none*: covariance runs one way only, so each of these is still
    // exactly one mismatch at the argument — never zero, which would be the
    // relation having become symmetric.
    let refused = [
        ("array<int>", "array<int|string> $a = [1];"),
        ("array<int>", "array<string> $a = [\"x\"];"),
        ("array<int>", "array<mixed> $a = [1];"),
        ("array<array<int>>", "array<array<int|string>> $a = [[1]];"),
    ];
    let rejected = refused
        .iter()
        .filter(|(param, decl)| {
            let src = call_with(&format!("{param} $items"), &format!("{decl}\nT::m($a);"));
            let diags = check_src(&src);
            diags
                .iter()
                .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
                .count()
                == 1
                && diags.error_count() == 1
        })
        .count();
    assert_eq!(
        rejected,
        refused.len(),
        "a narrowing is not a widening, and neither is an unrelated element type"
    );
}

#[test]
fn a_write_through_the_widened_parameter_does_not_reach_the_callers_array() {
    // What a checker can observe of `rule:types/arrays`'s copy-on-write value
    // semantics is both halves of the soundness argument: the callee's write
    // is checked against the *parameter's* element type, and the caller's own
    // binding keeps the element type it declared, so the value read back out
    // of it is still an `int`.
    let src = "<?nvs\nclass T {\n  static function m(array<int|string> $items): void {\n    $items[0] = \"x\";\n  }\n  static function go(): void {\n    array<int> $a = [1];\n    T::m($a);\n    int $n = $a[0];\n  }\n}\n";
    let diags = check_src(src);
    assert!(
        !diags.has_errors(),
        "the write is well typed at the parameter, and the caller still reads an `int`: {diags:?}"
    );

    // The other side of the same bound: being passed does not widen the
    // caller's array, so the `string` the callee wrote is not something the
    // narrow binding can be read as.
    let src = "<?nvs\nclass T {\n  static function m(array<int|string> $items): void {\n    $items[0] = \"x\";\n  }\n  static function go(): void {\n    array<int> $a = [1];\n    T::m($a);\n    string $s = $a[0];\n  }\n}\n";
    let diags = check_src(src);
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
            .count(),
        1,
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 1, "{diags:?}");
}
