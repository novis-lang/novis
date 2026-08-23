//! Property and method access through `$this`, `self`, `parent` and a typed local.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

#[test]
fn a_type_alias_is_substituted_into_a_local_declaration() {
    let diags = check_src(
        "<?mwl\ntype Id = uint;\nclass T {\n  function m(): void {\n    Id $x = 1;\n    int $y = $x;\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "`Id` expands to `uint`, so assigning it into a plain `int` should mismatch: {diags:?}"
    );
}

#[test]
fn self_resolves_inside_a_method_body() {
    let diags =
        check_src("<?mwl\nclass T {\n  function m(): void {\n    self $x = new self();\n  }\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_return_type_mismatch_is_diagnosed() {
    let diags = check_src("<?mwl\nclass T {\n  function m(): int {\n    return \"x\";\n  }\n}\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_BAD_RETURN_TYPE))
    );
}

#[test]
fn a_matching_return_type_is_fine() {
    let diags = check_src("<?mwl\nclass T {\n  function m(): int {\n    return 1;\n  }\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_this_property_access_has_its_declared_type() {
    // The property has an inline default, so ADR 0022's own check
    // (`crate::ctor_init`) has nothing to say about a missing
    // constructor here — this fixture is only exercising property-type
    // recovery.
    let diags = check_src(
        "<?mwl\nclass T {\n  public int $count = 0;\n  function m(): void {\n    int $n = $this->count;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_this_property_type_mismatch_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nclass T {\n  public int $count;\n  function m(): void {\n    string $n = $this->count;\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_this_method_call_returns_its_declared_type() {
    let diags = check_src(
        "<?mwl\nclass T {\n  function a(): int { return 1; }\n  function b(): void {\n    int $n = $this->a();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_undeclared_this_method_call_is_diagnosed() {
    let diags =
        check_src("<?mwl\nclass T {\n  function m(): void {\n    $this->missing();\n  }\n}\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
        "{diags:?}"
    );
}

#[test]
fn a_property_access_on_a_new_expression_resolves() {
    // Inline default again, for the same reason as the fixture above.
    let diags = check_src(
        "<?mwl\nclass Foo {\n  public int $count = 0;\n}\nclass T {\n  function m(): void {\n    int $n = (new Foo())->count;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_undeclared_property_on_a_typed_local_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $x = new Foo();\n    $x->missing;\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
        "{diags:?}"
    );
}

#[test]
fn an_arity_mismatch_on_a_method_call_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nclass T {\n  function a(int $x): void {}\n  function b(): void {\n    $this->a();\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_ARITY_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn an_argument_type_mismatch_on_a_method_call_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nclass T {\n  function a(int $x): void {}\n  function b(): void {\n    $this->a(\"s\");\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_constructor_argument_is_type_checked() {
    let diags = check_src(
        "<?mwl\nclass Foo {\n  function constructor(int $x) {}\n}\nclass T {\n  function m(): void {\n    new Foo(\"s\");\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_static_call_return_type_is_recovered() {
    let diags = check_src(
        "<?mwl\nclass T {\n  static function make(): int { return 1; }\n  function m(): void {\n    int $n = self::make();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn new_parent_resolves_to_the_parent_class() {
    let diags = check_src(
        "<?mwl\nclass Base {}\nclass Sub extends Base {\n  function m(): void {\n    Base $x = new parent();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `parent` as a *type* atom (a parameter here) — distinct from `new
/// parent(...)`, which the previous test already covers — resolves
/// against the same `extends` link.
#[test]
fn a_parent_typed_parameter_resolves_to_the_parent_class() {
    let diags = check_src(
        "<?mwl\nclass Base {}\nclass Sub extends Base {\n  function m(parent $x): void {\n    Base $y = $x;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A class with no `extends` has no parent for the `parent` type atom to
/// name — diagnosed rather than silently `mixed`, unlike `new
/// parent(...)`'s deliberate silent fallback for the same shape.
#[test]
fn a_parent_type_atom_with_no_extends_is_diagnosed() {
    let diags = check_src("<?mwl\nclass Base {\n  function m(parent $x): void {\n  }\n}\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_NO_PARENT_CLASS)),
        "{diags:?}"
    );
}

#[test]
fn a_match_expressions_type_is_the_union_of_its_arms() {
    let diags = check_src(
        "<?mwl\nclass T {\n  function m(): void {\n    string $n = match (1) { 1 => 2, default => 3 };\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_ternary_expressions_type_is_the_union_of_its_branches() {
    let diags = check_src(
        "<?mwl\nclass T {\n  function m(): void {\n    int|string $n = true ? 1 : \"s\";\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_ternary_expressions_type_mismatch_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nclass T {\n  function m(): void {\n    int $n = true ? 1 : \"s\";\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}
