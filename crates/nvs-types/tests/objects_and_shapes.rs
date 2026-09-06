//! `rule:types/object-top`'s `object` top type and inline shape types, plus ADR 0028's `unset()` refusal.
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
    // ADR 0028 § 3 leaves `unset()` one job, and `rule:types/declaration`'s declare-once,
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
fn an_object_literal_is_assignable_to_object() {
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
fn an_object_literal_with_exactly_the_shapes_fields_is_fine() {
    let diags = check_in_method("({x: int, y: int}) $p = {x: 1, y: 2};");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_object_literal_with_extra_fields_still_satisfies_a_narrower_shape() {
    // Width subtyping: a source with extra fields beyond the shape still
    // satisfies it.
    let diags = check_in_method("({x: int}) $p = {x: 1, y: 2};");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_object_literal_missing_a_shapes_field_is_diagnosed() {
    let diags = check_in_method("({x: int, y: int}) $p = {x: 1};");
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn an_object_literal_with_a_mismatched_field_type_is_diagnosed() {
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
fn reading_a_field_a_shape_does_not_name_is_erased_with_no_diagnostic() {
    // Deferred to ADR 0014 § 5's runtime-checked fallback (M4) — this
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
