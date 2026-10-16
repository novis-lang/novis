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

#[test]
fn an_array_of_a_narrower_element_type_is_accepted_where_a_wider_one_is_read() {
    // The relation is applied at three positions and the covariance belongs to
    // the relation, not to the call: the parameter is the test above, and
    // these are the other two `nvs_types::expr::assign` names — an assignment
    // to a wider binding, and a return into a wider declared type.
    let src = "<?nvs\nclass T {\n  public static function widened(): array<int|string> {\n    array<int> $a = [1, 2];\n    array<int|string> $b = $a;\n    return $b;\n  }\n  public static function returned(): array<int|string> {\n    array<int> $a = [1];\n    return $a;\n  }\n}\n";
    let diags = check_src(src);
    assert!(
        !diags.has_errors(),
        "an assignment and a return widen for the same reason a parameter does: {diags:?}"
    );

    // Each position refuses the narrowing on its own, so neither is accepting
    // everything. One mismatch apiece, never zero, and each reported by the
    // code belonging to the position rather than by a shared catch-all.
    let narrowings = [
        (
            "array<int|string> $a = [1];\n    array<int> $b = $a;\n    return [1];",
            code::E_TYPE_MISMATCH,
        ),
        (
            "array<int|string> $a = [1];\n    return $a;",
            code::E_BAD_RETURN_TYPE,
        ),
    ];
    for (body, expected) in narrowings {
        let src = format!(
            "<?nvs\nclass T {{\n  public static function m(): array<int> {{\n    {body}\n  }}\n}}\n"
        );
        let diags = check_src(&src);
        assert_eq!(
            diags.iter().filter(|d| d.code == Some(expected)).count(),
            1,
            "a widening is not a narrowing, in either position: {diags:?}"
        );
        assert_eq!(diags.error_count(), 1, "{diags:?}");
    }
}

#[test]
fn a_covariant_array_argument_is_not_restamped_at_the_call() {
    // `rule:types/conversion` prices `array<T> as array<U>` at an O(n)
    // restamp, one tag test per element. The covariant read is not that
    // conversion spelled implicitly, and what a checker can see of the
    // difference is both ends of the call: the callee is called at the type it
    // declared, and the caller's local keeps the type *it* declared, so there
    // is no third, widened array for `nvs_ir::lower` to have had to build.
    let src = "<?nvs\nclass T {\n  public static function m(array<int|string> $items): void {}\n  public static function go(): void {\n    array<int> $a = [1, 2];\n    T::m($a);\n  }\n}\n";
    let (diags, declared) = check_src_declared(src);
    assert!(!diags.has_errors(), "{diags:?}");

    let narrow = declared.of("array<int>", "$a");
    let wide = declared.of("array<int|string>", "$items");
    assert_ne!(
        narrow, wide,
        "the two element types intern distinctly, or nothing below says anything"
    );

    let Some(nvs_types::expr_table::ExprInfo::Call(call)) = declared.folded_at("T::m($a)") else {
        panic!("the call resolved to no entry at all");
    };
    assert_eq!(
        call.param_tys[0], wide,
        "the callee is called at its own declared element type"
    );

    let a = declared
        .exprs()
        .local_scopes()
        .flat_map(|(_, locals)| locals)
        .find(|local| local.name == "a")
        .expect("the caller declares `$a`");
    assert_eq!(
        a.ty, narrow,
        "being read through a wider view leaves the caller's array stamped as it was declared"
    );
}

#[test]
fn a_write_through_a_widened_array_view_is_checked_against_its_element_type() {
    // The widened view is not a hole: a write through it is checked against
    // the element type *it* declares, which is the half of the soundness
    // argument that does not depend on copy-on-write. Both spellings of a
    // write are checked, and both members of the union are admitted.
    let accepted = [
        "$items[0] = \"x\";",
        "$items[0] = 7;",
        "$items[] = \"x\";",
        "$items[] = 7;",
    ];
    for write in accepted {
        let src = format!(
            "<?nvs\nclass T {{\n  public static function m(array<int|string> $items): void {{\n    {write}\n  }}\n}}\n"
        );
        let diags = check_src(&src);
        assert!(
            !diags.has_errors(),
            "`{write}` writes a member of the view's own element type: {diags:?}"
        );
    }

    let refused = ["$items[0] = 1.5;", "$items[] = 1.5;"];
    for write in refused {
        let src = format!(
            "<?nvs\nclass T {{\n  public static function m(array<int|string> $items): void {{\n    {write}\n  }}\n}}\n"
        );
        let diags = check_src(&src);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
                .count(),
            1,
            "`{write}` is checked against `int|string`, not against what the caller happened to hold: {diags:?}"
        );
    }
}
