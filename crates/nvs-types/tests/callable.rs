//! ADR 0027: which values satisfy `callable`, and what `$obj(...)` refuses.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// ADR 0027: `callable` is satisfied by exactly one shape of value.

#[test]
fn a_bare_string_where_callable_is_expected_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function run(callable $fn): void {}\n  function m(): void {\n    $this->run(\"strlen\");\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CALLABLE_STRING_UNSUPPORTED)),
        "{diags:?}"
    );
}

#[test]
fn an_array_callable_spelling_where_callable_is_expected_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function run(callable $fn): void {}\n  function m(): void {\n    $this->run([$this, \"m\"]);\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CALLABLE_ARRAY_UNSUPPORTED)),
        "{diags:?}"
    );
}

#[test]
fn a_first_class_callable_reference_satisfies_a_callable_parameter() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function run(callable $fn): void {}\n  function target(): void {}\n  function m(): void {\n    $this->run($this->target(...));\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn calling_a_non_callable_object_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Adder {\n  function add(int $a, int $b): int { return $a + $b; }\n}\nclass T {\n  function m(): void {\n    Adder $adder = new Adder();\n    $adder(1, 2);\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_NOT_CALLABLE)),
        "{diags:?}"
    );
}

#[test]
fn calling_a_closure_value_is_unaffected() {
    let diags = check_in_method("callable $fn = fn(): int => 1;\n$fn();\n");
    assert!(!diags.has_errors(), "{diags:?}");
}
