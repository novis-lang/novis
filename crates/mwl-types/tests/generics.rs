//! ADR 0053 § 2's one narrow generic door — a user class implementing a compiler-owned generic interface at a concrete type.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

// ADR 0053 §§ 1-2: the two compiler-owned generic interfaces, and the one
// door user code has onto a type parameter.

#[test]
fn a_class_may_implement_a_compiler_owned_generic_interface_at_a_concrete_type() {
    let diags = check_src(
        "<?mwl\n\
         class Counter implements Iterable<int> {\n\
         \x20 function iterate(): Iterator<int> { return Counter::empty(); }\n\
         \x20 static function empty(): Iterator<int> { return Counter::empty(); }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_cursor_s_element_type_comes_from_the_receiver_s_type_argument() {
    let diags = check_src(
        "<?mwl\n\
         class T {\n\
         \x20 function m(Iterator<int> $it): void {\n\
         \x20\x20 bool $more = $it->advance();\n\
         \x20\x20 int $n = $it->current();\n\
         \x20\x20 echo $n . $more;\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The half that proves the argument is actually *used*: the same body
/// against `Iterator<string>` must not type-check.
#[test]
fn a_cursor_s_element_type_is_not_mixed() {
    let diags = check_src(
        "<?mwl\n\
         class T {\n\
         \x20 function m(Iterator<string> $it): void {\n\
         \x20\x20 int $n = $it->current();\n\
         \x20\x20 echo $n;\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_user_declared_name_written_with_type_arguments_is_refused() {
    let diags = check_src(
        "<?mwl\n\
         class Animal {}\n\
         class T {\n\
         \x20 function m(Animal<int> $a): void { echo \"x\"; }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
}

/// A reserved interface that takes *no* parameters is refused by the same
/// rule, in the position ADR 0053 § 2 opened -- an `implements` clause.
#[test]
fn a_non_generic_reserved_interface_takes_no_type_arguments_either() {
    let diags = check_src("<?mwl\nclass M implements Comparable<int> {}\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
}

#[test]
fn a_generic_interface_written_bare_is_refused() {
    let diags =
        check_src("<?mwl\nclass T {\n  function m(Iterator $it): void { echo \"x\"; }\n}\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_ARG_COUNT)),
        "{diags:?}"
    );
}

#[test]
fn a_generic_interface_written_with_too_many_arguments_is_refused() {
    let diags = check_src("<?mwl\nclass C implements Iterable<int, string> {}\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_ARG_COUNT)),
        "{diags:?}"
    );
}

/// ADR 0015 gives a `type` alias no parameters of its own, so an alias
/// written with arguments lands on the same refusal a class does -- while
/// an alias used *as* an argument still expands.
#[test]
fn a_type_alias_has_no_type_parameters_but_may_be_one() {
    let diags = check_src(
        "<?mwl\n\
         type Id = int;\n\
         class T {\n\
         \x20 function ok(Iterator<Id> $it): void { int $n = $it->current(); echo $n; }\n\
         \x20 function bad(Id<int> $x): void { echo \"x\"; }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}
