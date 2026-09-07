//! `rule:types/callable-is-a-closure`: which values satisfy `callable`, and what `$obj(...)` refuses.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// `rule:types/callable-is-a-closure`: `callable` is satisfied by exactly one shape of value.

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

// `rule:types/callable-literal-inference`: a `fn` literal answers its own
// written signature, which is what gives every row of
// `rule:types/callable-arity` and `rule:types/callable-variance` a source
// spelling to be reached by.

#[test]
fn a_fn_literal_satisfies_a_written_signature() {
    let diags = check_in_method("callable(int): string $format = fn (int $n): string => \"n\";\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_fn_literal_still_satisfies_bare_callable() {
    let diags = check_in_method("callable $format = fn (int $n): string => \"n\";\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_fn_literal_answering_another_type_is_refused_where_it_is_written() {
    let diags = check_in_method("callable(int): string $format = fn (int $n): int => $n;\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_fn_literal_taking_a_narrower_parameter_is_refused() {
    let diags =
        check_in_method("callable(int|string): string $format = fn (int $n): string => \"n\";\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

// The prefix rule, asserted on both sides: the last arity accepted and the
// first refused, so a relation that stopped one parameter early fails here
// while still looking right on either line alone.

#[test]
fn a_fn_literal_declaring_fewer_parameters_satisfies_the_type() {
    let diags =
        check_in_method("callable(int, string): string $format = fn (int $n): string => \"n\";\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_fn_literal_declaring_more_parameters_than_the_type_is_refused() {
    let diags = check_in_method(
        "callable(int): string $format = fn (int $n, string $k): string => \"n\";\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}
