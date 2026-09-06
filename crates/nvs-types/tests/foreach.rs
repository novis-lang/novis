//! `rule:iteration/foreach-subjects`'s three accepted `foreach` subjects, and the element type each binds.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// `rule:iteration/foreach-subjects`: what `foreach` accepts, and what it yields.

#[test]
fn a_foreach_over_an_array_binds_the_element_type() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(array<int> $a): void {\n\
         \x20\x20 foreach ($a as int $n) { echo $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_foreach_binding_that_disagrees_with_the_element_type_is_diagnosed() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(array<int> $a): void {\n\
         \x20\x20 foreach ($a as string $s) { echo $s; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_foreach_over_a_cursor_binds_its_type_argument() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(Iterator<int> $it): void {\n\
         \x20\x20 foreach ($it as int $n) { echo $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The element type reaches the loop through the class's own
/// `implements Iterable<int>` clause, not through the subject's written
/// type -- which is the whole reason `ClassSignature::implements` records
/// the resolved argument.
#[test]
fn a_foreach_over_a_class_reaches_its_implements_clause_for_the_element_type() {
    let diags = check_src(
        "<?nvs\n\
         class Counter implements Iterable<int> {\n\
         \x20 function iterate(): Iterator<int> { return Counter::empty(); }\n\
         \x20 static function empty(): Iterator<int> { return Counter::empty(); }\n\
         }\n\
         class T {\n\
         \x20 function m(Counter $c): void {\n\
         \x20\x20 foreach ($c as string $s) { echo $s; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_foreach_over_a_class_that_implements_neither_interface_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Bag {}\n\
         class T {\n\
         \x20 function m(Bag $b): void {\n\
         \x20\x20 foreach ($b as int $n) { echo $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FOREACH_SUBJECT_NOT_ITERABLE)),
        "{diags:?}"
    );
}

#[test]
fn a_foreach_over_a_scalar_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(int $n): void {\n\
         \x20\x20 foreach ($n as int $x) { echo $x; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FOREACH_SUBJECT_NOT_ITERABLE)),
        "{diags:?}"
    );
}

#[test]
fn a_key_binding_over_a_cursor_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(Iterator<int> $it): void {\n\
         \x20\x20 foreach ($it as int $k => int $n) { echo $k . $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FOREACH_KEY_ON_CURSOR)),
        "{diags:?}"
    );
}

/// `rule:types/arrays` gives an `array<T>` one stored key type, and it is the one
/// this binding declares -- see `crate::expr::check_foreach_key`.
#[test]
fn a_string_key_binding_over_an_array_is_accepted() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(array<int> $a): void {\n\
         \x20\x20 foreach ($a as string $k => int $n) { echo $k . $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// Every other declared key type is always wrong rather than unprovable:
/// no array can produce one. `rule:types/arrays`.
#[test]
fn a_non_string_key_binding_over_an_array_is_refused() {
    for key_ty in ["int", "uint", "mixed", "int|string"] {
        let diags = check_src(&format!(
            "<?nvs\n\
             class T {{\n\
             \x20 function m(array<int> $a): void {{\n\
             \x20\x20 foreach ($a as {key_ty} $k => int $n) {{ echo $n; }}\n\
             \x20 }}\n\
             }}\n"
        ));
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_FOREACH_KEY_TY)),
            "{key_ty}: {diags:?}"
        );
    }
}
