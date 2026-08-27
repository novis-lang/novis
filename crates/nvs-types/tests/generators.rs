//! ADR 0053 §§ 4-5: what makes a body a generator, and what `yield` refuses.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

/// ADR 0053 § 4: a body containing `yield` is a generator, its declared
/// return type must be `Iterator<T>`, and each operand is checked
/// against that `T`.
#[test]
fn a_generator_declaring_iterator_of_its_yield_type_checks_clean() {
    let diags = check_src(
        "<?mwl\n\
         class G {\n\
         \x20 static function upTo(int $n): Iterator<int> {\n\
         \x20\x20 var $i = 1;\n\
         \x20\x20 while ($i <= $n) { yield $i; $i = $i + 1; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The return type is checked against the body, not the other way round:
/// `yield` makes it a generator and `int` is then wrong.
#[test]
fn a_generator_declaring_anything_but_a_cursor_is_diagnosed() {
    let diags = check_src(
        "<?mwl\n\
         class G {\n\
         \x20 static function bad(): int { yield 1; }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_GENERATOR_RETURN_TYPE)),
        "{diags:?}"
    );
}

/// A wrong return type reports once — every `yield` in the body is left
/// unchecked rather than each reporting a stray-`yield` diagnostic too.
#[test]
fn a_generator_with_a_wrong_return_type_reports_exactly_once() {
    let diags = check_src(
        "<?mwl\n\
         class G {\n\
         \x20 static function bad(): int { yield 1; yield 2; }\n\
         }\n",
    );
    assert_eq!(
        diags.iter().filter(|d| d.code.is_some()).count(),
        1,
        "{diags:?}"
    );
}

/// The operand is checked against `T`.
#[test]
fn a_yield_operand_must_satisfy_the_declared_element_type() {
    let diags = check_src(
        "<?mwl\n\
         class G {\n\
         \x20 static function g(): Iterator<int> { yield \"x\"; }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// ADR 0053 § 5: no generator return value to retrieve.
#[test]
fn a_generator_returning_a_value_is_diagnosed() {
    let diags = check_src(
        "<?mwl\n\
         class G {\n\
         \x20 static function g(): Iterator<int> { yield 1; return 5; }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_GENERATOR_RETURNS_A_VALUE)),
        "{diags:?}"
    );
}

/// ...but a bare `return;` stops the sequence and is fine.
#[test]
fn a_generator_may_stop_early_with_a_bare_return() {
    let diags = check_src(
        "<?mwl\n\
         class G {\n\
         \x20 static function g(): Iterator<int> { yield 1; return; }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0053 § 4's lexical confinement: a file-scope `yield` has no
/// generator to belong to.
#[test]
fn a_yield_outside_any_generator_is_diagnosed() {
    let diags = check_src("<?mwl\nyield 3;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_YIELD_OUTSIDE_GENERATOR)),
        "{diags:?}"
    );
}

/// ADR 0053 § 5 rejects `yield from`, and § 1 leaves a cursor no key.
#[test]
fn yield_from_and_a_keyed_yield_are_both_refused() {
    for body in ["yield from G::g();", "yield 1 => 2;"] {
        let diags = check_src(&format!(
            "<?mwl\nclass G {{\n  static function g(): Iterator<int> {{ {body} }}\n}}\n"
        ));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_YIELD_FORM_UNSUPPORTED)),
            "{body}: {diags:?}"
        );
    }
}

/// `mixed` is the one unchecked position (ADR 0007 § 1), so it neither
/// yields an element type nor is refused as a subject.
#[test]
fn a_mixed_subject_is_neither_checked_nor_refused() {
    let diags = check_src(
        "<?mwl\n\
         class T {\n\
         \x20 function m(mixed $x): void {\n\
         \x20\x20 foreach ($x as string $s) { echo $s; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}
