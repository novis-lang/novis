//! `rule:types/object-top`'s `object` top type and inline shape types, plus `rule:classes/no-magic-methods`'s `unset()` refusal.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

#[test]
fn unset_on_a_declared_property_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo { public int $x; function constructor() { $this->x = 1; } }\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    unset($a->x);\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNSET_ON_PROPERTY)),
        "{diags:?}"
    );
}

#[test]
fn unset_on_a_local_variable_is_diagnosed() {
    // `rule:classes/unset-is-refused-on-a-property` leaves `unset()` one job, and `rule:types/declaration`'s declare-once,
    // definitely-assigned binding has no "undefined again" state for a local
    // to be put back into.
    let diags = check_in_method("mixed $x = 1;\nunset($x);");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNSET_TARGET_NOT_AN_ELEMENT)),
        "{diags:?}"
    );
}

#[test]
fn unset_on_an_element_of_a_temporary_is_diagnosed() {
    // The other half of the same rule: `rule:types/arrays` separates the array
    // before the entry goes, and a call's result has no slot for the
    // separated copy to be written back into.
    let diags = check_src(
        "<?nvs\nclass T {\n  static function rows(): array<string> { return []; }\n  function m(): void {\n    unset(T::rows()[\"a\"]);\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNSET_TARGET_NOT_AN_ELEMENT)),
        "{diags:?}"
    );
}

#[test]
fn unset_on_an_element_of_a_local_is_accepted() {
    let diags = check_in_method("array<string> $a = [\"k\" => \"1\"];\nunset($a[\"k\"]);");
    assert!(!diags.has_errors(), "{diags:?}");
}

// `rule:types/object-top`: `object` is the real supertype of every class type.

#[test]
fn a_class_instance_is_assignable_to_object() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    object $o = new Foo();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_anon_object_is_assignable_to_object() {
    let diags = check_in_method("object $o = {x: 1};");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_scalar_is_not_assignable_to_object() {
    let diags = check_in_method("object $o = 1;");
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

// `rule:types/shape-type`: a shape type is checked structurally, by width subtyping
// plus ordinary field-type assignability.

#[test]
fn an_anon_object_with_exactly_the_shapes_fields_is_fine() {
    let diags = check_in_method("({x: int, y: int}) $p = {x: 1, y: 2};");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_source_with_extra_fields_still_satisfies_the_shape() {
    // Width subtyping: a source with extra fields beyond the shape still
    // satisfies it.
    let diags = check_in_method("({x: int}) $p = {x: 1, y: 2};");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_source_missing_a_required_field_does_not() {
    let diags = check_in_method("({x: int, y: int}) $p = {x: 1};");
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

// `rule:types/shape-type`: the mismatch prints both shapes whole, so the key
// that is actually at fault is named in a `help:` line rather than left for the
// reader to diff out. Same code, because it is the same mistake.

#[test]
fn an_anon_object_missing_a_required_key_names_the_key() {
    let diags = check_in_method(r#"({age: int, name: string}) $p = {name: "x"};"#);
    let diag = diags
        .iter()
        .find(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .expect("the mismatch is still reported");
    assert!(diag.notes.iter().any(|n| n.contains("`age`")), "{diag:?}");
}

#[test]
fn an_anon_object_missing_two_required_keys_names_both() {
    let diags = check_in_method("({x: int, y: int, z: int}) $p = {y: 2};");
    let diag = diags
        .iter()
        .find(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .expect("the mismatch is still reported");
    assert!(
        diag.notes
            .iter()
            .any(|n| n.contains("`x`") && n.contains("`z`") && !n.contains("`y`")),
        "{diag:?}"
    );
}

#[test]
fn a_source_field_that_is_itself_optional_is_named_as_missing() {
    // The source carries the key and still cannot supply it: `{x?: int}` is not
    // proven to hold an `x`, which is `shape_satisfied`'s rule and not a second
    // one written for the message.
    let diags = check_in_method("({x?: int}) $a = {x: 1};\n({x: int}) $b = $a;");
    let diag = diags
        .iter()
        .find(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .expect("the mismatch is still reported");
    assert!(diag.notes.iter().any(|n| n.contains("`x`")), "{diag:?}");
}

#[test]
fn a_field_present_at_the_wrong_type_names_no_missing_key() {
    // The help line is not noise: nothing is absent here, so nothing is named
    // and the two printed shapes are the whole diagnosis.
    let diags = check_in_method(r#"({x: int}) $p = {x: "s"};"#);
    let diag = diags
        .iter()
        .find(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .expect("the mismatch is still reported");
    assert!(
        !diag.notes.iter().any(|n| n.contains("required here")),
        "{diag:?}"
    );
}

#[test]
fn a_parenthesized_group_reports_one_mismatch_and_not_two() {
    // One mistake is one diagnostic: the group's arm re-enters `check_expr`, so
    // the report belongs to the inner expression's span and the enclosing group
    // adds nothing. Asked of a shape and of a scalar, because the rule is
    // `check_expr`'s and not the shape arm's.
    let shape = check_in_method(r#"({age: int, name: string}) $p = ({name: "x"});"#);
    assert_eq!(
        shape
            .iter()
            .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
            .count(),
        1,
        "{shape:?}"
    );
    let scalar = check_in_method(r#"int $x = ("s");"#);
    assert_eq!(
        scalar
            .iter()
            .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
            .count(),
        1,
        "{scalar:?}"
    );
}

#[test]
fn a_class_missing_a_shapes_field_names_the_field() {
    // A declared property is always present, so a class source answers presence
    // alone — and the absent one is named the same way a shape's is.
    let diags = check_src(
        "<?nvs\nclass Foo {\n  public int $x = 0;\n}\nclass T {\n  function m(): void {\n    ({x: int, y: int}) $p = new Foo();\n  }\n}\n",
    );
    let diag = diags
        .iter()
        .find(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .expect("the mismatch is still reported");
    assert!(diag.notes.iter().any(|n| n.contains("`y`")), "{diag:?}");
}

#[test]
fn an_anon_object_with_a_mismatched_field_type_is_diagnosed() {
    let diags = check_in_method(r#"({x: int}) $p = {x: "s"};"#);
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_class_with_a_matching_property_satisfies_a_shape_type() {
    let diags = check_src(
        "<?nvs\nclass Foo {\n  public int $x = 0;\n}\nclass T {\n  function m(): void {\n    ({x: int}) $p = new Foo();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_class_missing_a_shapes_field_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    ({x: int}) $p = new Foo();\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

// `rule:types/shape-type`: a field marked `name?:` relaxes presence and
// nothing else, and it is a different question from the field's type being
// nullable.

#[test]
fn a_source_missing_an_optional_field_satisfies_the_shape() {
    let diags = check_in_method("({x: int, y?: int}) $p = {x: 1};");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_optional_field_that_is_present_is_still_type_checked() {
    let diags = check_in_method(r#"({x?: int}) $p = {x: "s"};"#);
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn an_optional_field_does_not_admit_null_and_a_nullable_one_does_not_admit_absence() {
    // The two halves of `{a?: int}` versus `{a: ?int}`, asserted together so
    // that a checker collapsing them into one type fails here whichever way
    // round it collapsed them.
    let optional_given_null = check_in_method("({a?: int}) $p = {a: null};");
    assert!(
        optional_given_null
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{optional_given_null:?}"
    );
    let nullable_given_absent = check_in_method("({a: ?int}) $p = {b: 1};");
    assert!(
        nullable_given_absent
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{nullable_given_absent:?}"
    );
    let nullable_given_null = check_in_method("({a: ?int}) $p = {a: null};");
    assert!(!nullable_given_null.has_errors(), "{nullable_given_null:?}");
}

#[test]
fn a_source_whose_own_field_is_optional_does_not_fill_a_required_one() {
    // Presence has to be *proven*, not merely possible: a `{a?: int}` value
    // widens to another `{a?: int}` and to `{}`, and to nothing that promises
    // an `a`.
    let diags = check_src(
        "<?nvs\ntype Maybe = {a?: int};\ntype Sure = {a: int};\nclass T {\n  function m(Maybe $q): void {\n    Sure $p = $q;\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_class_missing_the_property_still_satisfies_an_optional_field() {
    let diags = check_src(
        "<?nvs\nclass Foo { public int $x; function constructor() { $this->x = 1; } }\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    ({x: int, y?: string}) $p = $a;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_shape_type_alias_resolves_like_any_other_alias() {
    let diags = check_src(
        "<?nvs\ntype Point = {x: int, y: int};\nclass T {\n  function m(): void {\n    Point $p = {x: 1, y: 2};\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

// `rule:types/erased-member-access`: property access through an erased view.

#[test]
fn reading_a_field_a_shape_names_recovers_its_type_with_no_diagnostic() {
    let diags = check_in_method("({x: int}) $p = {x: 1};\nint $n = $p->x;");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn reading_an_optional_field_answers_the_declared_type_not_a_nullable_one() {
    // `rule:types/shape-type`: the shape proves the type and not the presence,
    // so an ordinary read answers `int` and absence is the runtime throw —
    // deliberately not widened to `?int`, which would answer "the key may be
    // absent" the way nullability answers "the value may be `null`".
    let diags = check_in_method("({a?: int}) $p = {a: 1};\nint $n = $p->a;");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn reading_a_field_a_shape_does_not_name_is_erased_with_no_diagnostic() {
    // Deferred to `rule:classes/no-dynamic-properties`'s runtime-checked fallback (M4) — this
    // compile-time checker cannot know either way, so it reports
    // nothing rather than guessing.
    let diags = check_in_method("({x: int}) $p = {x: 1};\nmixed $n = $p->y;");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn reading_a_field_through_plain_object_is_erased_with_no_diagnostic() {
    let diags = check_in_method("object $o = {x: 1};\nmixed $n = $o->x;");
    assert!(!diags.has_errors(), "{diags:?}");
}
