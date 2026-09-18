//! `$x is T` and `$x is $cls` in the checker — `rule:types/type-test`: the
//! answer is `bool` for every subject, a settled answer folds to `true` or
//! `false`, and only the right-hand side is ever refused.
//!
//! The acceptances here are the load-bearing half. `is` is **total**, so a test
//! whose answer the declaration already settles has to compile — the failure
//! this suite exists to catch is a session refusing a subject because its
//! declaration already answered, which no arm of this operator does
//! (ADR 0150 § 6).

mod common;

use common::{check_src, check_src_allowing_parse_errors};
use nvs_diagnostics::{Diagnostics, code};

/// The declarations the sweep and the class cases are written over: an enum to
/// name a case of, two interfaces to intersect, and a class implementing both.
const DECLS: &str = "\
enum Mode { Read, Write }
interface Labelled { function label(): string; }
interface Marked { function mark(): string; }
class Node implements Labelled, Marked {
  public const int WIDTH = 3;
  function label(): string { return \"n\"; }
  function mark(): string { return \"m\"; }
}
";

/// Checks `body` inside a method taking `mixed $m` and `Node $node`, with
/// [`DECLS`] above it.
fn check_with_subjects(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\n{DECLS}class T {{\n  function m(mixed $m, Node $node): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// The fold is observed through the binding it is assigned to: `true` and
/// `false` are literal types of `bool` (`rule:types/literal-types`), so a
/// `bool` answer where a `true` was folded is an ordinary type mismatch and a
/// folded one is not.
#[test]
fn a_statically_true_test_over_a_declared_type_checks_and_folds_to_true() {
    let diags = check_with_subjects(
        "    int $n = 7;\n    true $t = $n is int;\n    true $u = $n is mixed;\n    true $v = \
         $node is Labelled;",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The other direction of the same fold: a subject that can never hold the
/// tested type answers `false` rather than being refused.
#[test]
fn a_statically_false_test_over_a_declared_type_checks_and_folds_to_false() {
    let diags = check_with_subjects(
        "    int $n = 7;\n    false $t = $n is string;\n    false $u = $n is Node;\n    false $v = \
         $node is int;",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The trap named in full. `is` refuses no left-hand side at all — a subject
/// that can hold no object is a fold and never a diagnostic — so neither the
/// scalar against a class nor the object against a scalar reports anything.
#[test]
fn neither_direction_reports_anything_about_the_subject() {
    let diags = check_with_subjects(
        "    int $n = 7;\n    bool $a = $n is Node;\n    bool $b = $node is string;",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A union subject settles neither direction, so both tests stay `bool` and
/// both compile — the shape narrowing is written over, and the one a fold that
/// reached too far would break by answering `true` for a member.
#[test]
fn a_test_over_a_union_member_and_over_a_non_member_both_check() {
    let diags = check_src(&format!(
        "<?nvs\n{DECLS}class T {{\n  function m(int|string $u): void {{\n    bool $a = $u is \
         int;\n    bool $b = $u is Node;\n    true $c = $u is int|string;\n  }}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `rule:types/type-test`'s first refusal. Both qualifiers are erased before
/// codegen, so no value carries the bit the test would read — the question is
/// absent rather than knowable.
#[test]
fn is_against_a_tainted_or_secret_qualifier_is_e0813() {
    for target in ["tainted string", "secret bytes", "array<tainted string>"] {
        let diags = check_with_subjects(&format!("    bool $b = $m is {target};"));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_TEST_AGAINST_A_QUALIFIER)),
            "`{target}` was accepted: {diags:?}"
        );
    }
    let unqualified = check_with_subjects("    bool $b = $m is string;");
    assert!(!unqualified.has_errors(), "{unqualified:?}");
}

/// `rule:types/type-test`'s second refusal: no value inhabits either type, so
/// there is no shape a subject could be carrying.
#[test]
fn is_against_void_or_never_is_e0811() {
    for target in ["void", "never"] {
        let diags = check_with_subjects(&format!("    bool $b = $m is {target};"));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_TEST_AGAINST_AN_UNINHABITED_TYPE)),
            "`{target}` was accepted: {diags:?}"
        );
    }
}

/// The fourth refusal `is` reaches is not its own: there is no `float` literal
/// type at all (`rule:types/literal-types`), and the parser says so where the
/// literal is written. This operator claims no code for it.
#[test]
fn is_against_a_float_literal_reuses_the_literal_type_refusal_and_claims_no_new_code() {
    let diags = check_src_allowing_parse_errors(&format!(
        "<?nvs\n{DECLS}class T {{\n  function m(mixed $m): void {{\n    bool $b = $m is \
         3.14;\n  }}\n}}\n"
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FLOAT_LITERAL_TYPE)),
        "{diags:?}"
    );
    assert!(
        !diags.iter().any(|d| {
            d.code == Some(code::E_TYPE_TEST_AGAINST_A_QUALIFIER)
                || d.code == Some(code::E_TYPE_TEST_AGAINST_AN_UNINHABITED_TYPE)
        }),
        "{diags:?}"
    );
}

/// `rule:types/class-reference-sites`' third site: a `class<T>` on the right of
/// `is` narrows the subject to **`T`** on the true edge, which is sound because
/// the reference holds a `T` or an implementor of one. The narrowed binding is
/// read through a member only `Node` declares, so a test that narrowed to
/// nothing — or to the wrong base — fails to compile rather than passing
/// quietly.
#[test]
fn a_class_reference_on_the_right_of_is_narrows_the_subject_to_its_base() {
    let diags = check_src(&format!(
        "<?nvs\n{DECLS}class T {{\n  function m(mixed $m, class<Node> $cls): void {{\n    if ($m \
         is $cls) {{\n      Node $n = $m;\n    }}\n  }}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The one thing the value arm refuses, and it is about the operand rather
/// than the subject: a value that is not a `class<T>` names no class the
/// compiler can resolve, so it is `E0496` — the report `new $v(...)` and
/// `$v::f(...)` already share — with its help naming `as class<T>`.
#[test]
fn a_value_that_is_not_a_class_reference_on_the_right_of_is_is_refused_as_a_dynamic_class_name() {
    let diags = check_src(&format!(
        "<?nvs\n{DECLS}class T {{\n  function m(mixed $m, string $name): void {{\n    bool $b = \
         $m is $name;\n  }}\n}}\n"
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DYNAMIC_CLASS_NAME)),
        "{diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| d.notes.iter().any(|n| n.contains("as class<Base>"))),
        "the conversion an author can take is not named: {diags:?}"
    );
}

/// A subject that can hold no object settles the class test before the program
/// runs, so it folds to `false` like every other settled test — and costs no
/// diagnostic, which is where this arm parts company with the PHP spelling it
/// replaces (`rule:php-migration/one-type-test`).
#[test]
fn a_class_test_over_a_subject_that_holds_no_object_folds_to_false_without_a_diagnostic() {
    let diags = check_src(&format!(
        "<?nvs\n{DECLS}class T {{\n  function m(int $n, class<Node> $cls): void {{\n    false $b \
         = $n is $cls;\n  }}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// Every row of `rule:types/type-test`'s table, asked at once and counted:
/// a member that grew a refusal of its own fails here while every other row
/// still passes, which reading one accepted case cannot catch.
#[test]
fn every_other_type_atom_is_accepted_on_the_right_of_is() {
    let atoms = [
        "int",
        "uint",
        "float",
        "decimal",
        "string",
        "bytes",
        "bool",
        "null",
        "object",
        "Node",
        "Labelled",
        "array",
        "array<int>",
        "{x: int, y: int}",
        "iterable",
        "callable",
        "5",
        "\"yay\"",
        "true",
        "Mode::Read",
        "Node::WIDTH",
        "mixed",
        "int|float",
        "?int",
        "Labelled&Marked",
    ];
    let refused: Vec<&str> = atoms
        .iter()
        .copied()
        .filter(|atom| check_with_subjects(&format!("    bool $b = $m is {atom};")).has_errors())
        .collect();
    assert!(
        refused.is_empty(),
        "`is` refused {} of {} type atoms: {refused:?}",
        refused.len(),
        atoms.len()
    );
}
