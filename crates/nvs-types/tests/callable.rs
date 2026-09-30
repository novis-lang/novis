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

/// PHP's variable function: the variable is a `string`, and no `string` is a
/// closure, so the call is refused where it is written. So is a call on every
/// other type whose values are never closures, and on a union of only those.
#[test]
fn calling_a_value_whose_type_is_never_a_closure_is_diagnosed() {
    for local in [
        "string $f = 'strlen';",
        "int $f = 7;",
        "float $f = 1.5;",
        "bool $f = true;",
        "array<string> $f = ['Core\\\\Str', 'length'];",
        "?string $f = null;",
        "int|string $f = 7;",
    ] {
        let diags = check_in_method(&format!("{local}\n$f('Zoë');\n"));
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_NOT_CALLABLE)),
            "{local}: {diags:?}"
        );
    }
}

/// A callee that may be a closure is left to the call: `mixed` holds anything,
/// and a nullable `callable` is a closure on one of its arms.
#[test]
fn calling_a_value_whose_type_may_be_a_closure_is_not_diagnosed() {
    for local in [
        "mixed $f = fn (string $s): uint => 1;",
        "?callable $f = fn (string $s): uint => 1;",
    ] {
        let diags = check_in_method(&format!("{local}\n$f('Zoë');\n"));
        assert!(
            !diags.iter().any(|d| d.code == Some(code::E_NOT_CALLABLE)),
            "{local}: {diags:?}"
        );
    }
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

/// `rule:types/callable-variance`: bare `callable` is the top of the lattice, so
/// a literal and a written signature alike land in it.
#[test]
fn every_callable_signature_is_assignable_to_bare_callable() {
    let diags = check_in_method("callable $format = fn (int $n): string => \"n\";\n");
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_in_method(
        "callable(int): string $format = fn (int $n): string => \"n\";\ncallable $any = $format;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

// `rule:types/callable-variance`, asserted on both sides: the return is
// covariant and the parameters contravariant, so each direction has the
// spelling it accepts named beside the one it refuses.

#[test]
fn a_callable_return_wider_than_the_slot_is_refused() {
    let diags = check_in_method("callable(int): string $format = fn (int $n): int => $n;\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );

    // A return that merely admits more than the slot declares is the same
    // refusal: covariance holds one way only.
    let diags = check_in_method("callable(int): string $f = fn (int $n): string|int => \"n\";\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_callable_return_may_be_narrower_than_the_slot_declares() {
    let diags = check_in_method("callable(int): int|string $f = fn (int $n): int => $n;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_callable_parameter_narrower_than_the_slot_is_refused() {
    let diags =
        check_in_method("callable(int|string): string $format = fn (int $n): string => \"n\";\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_callable_parameter_may_be_wider_than_the_slot_declares() {
    let diags =
        check_in_method("callable(int): string $f = fn (int|string $n): string => \"n\";\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

// `rule:types/callable-arity`'s prefix rule, asserted on both sides: the last
// arity accepted and the first refused, so a relation that stopped one
// parameter early fails here while still looking right on either line alone.

#[test]
fn a_closure_of_lower_arity_satisfies_a_wider_callable_type() {
    let diags =
        check_in_method("callable(int, string): string $format = fn (int $n): string => \"n\";\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_closure_of_higher_arity_than_the_type_is_refused() {
    let diags = check_in_method(
        "callable(int): string $format = fn (int $n, string $k): string => \"n\";\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

// `rule:types/callable-literal-inference`: a parameter the literal did not
// annotate takes its type from the position the literal is written in, and a
// position with no signature to give is named rather than guessed at.

#[test]
fn an_unannotated_fn_parameter_takes_the_expected_types_position() {
    // `$n * 2` is the assertion: `mixed` has no arithmetic, so this compiles
    // only if `$n` really arrived as the `int` the type names.
    let diags = check_in_method("callable(int): int $twice = fn ($n): int => $n * 2;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_written_parameter_beside_an_inferred_one_keeps_what_it_wrote() {
    let diags =
        check_in_method("callable(int, string): string $f = fn ($n, string $k): string => $k;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_unannotated_parameter_under_a_bare_callable_is_refused() {
    let diags = check_in_method("callable $f = fn ($n): int => 1;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CLOSURE_PARAMETER_TYPE_NOT_INFERABLE)),
        "{diags:?}"
    );
}

#[test]
fn an_unannotated_parameter_past_the_signatures_end_is_refused() {
    let diags = check_in_method("callable(int): int $f = fn ($n, $k): int => $n;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CLOSURE_PARAMETER_TYPE_NOT_INFERABLE)),
        "{diags:?}"
    );
}

// `rule:types/callable-literal-inference`: a parameter the literal wrote for
// itself is checked against the position rather than taking it, so
// `rule:types/callable-variance` decides it — each spelling annotating one
// parameter beside an inferred one, which is what makes it the *literal's*
// annotation being judged rather than the bare relation between two types.

#[test]
fn an_annotated_fn_parameter_may_be_wider_than_the_expected_type() {
    let diags = check_in_method(
        "callable(int, string): string $f = fn (int|string $n, $k): string => $k;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_annotated_fn_parameter_narrower_than_expected_is_refused() {
    let diags = check_in_method(
        "callable(int|string, string): string $f = fn (int $n, $k): string => $k;\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// `rule:types/closure-literal`: the expected type gives a literal its
/// parameter types and never its return type, so a block body still writes
/// its own — inferring one there is whole-body return-type inference.
#[test]
fn a_block_bodied_fn_still_declares_its_return_type() {
    let diags =
        check_in_method("callable(int): int $twice = fn ($n): int => { return $n * 2; };\n");
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_in_method("callable(int): int $twice = fn (int $n) => { return $n * 2; };\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CLOSURE_RETURN_TYPE_REQUIRED)),
        "{diags:?}"
    );
}
