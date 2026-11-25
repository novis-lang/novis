//! The goal `enum-tables`: an enum is a full key.
//!
//! `rule:expressions/a-match-over-an-enum-covers-every-case`: a `match` over an enum `E` or `?E`
//! with no `default` arm names every case, and `null` for `?E`. A missing case is `E1016` at the
//! `match` keyword, and the message lists every missing case.
//!
//! Every fixture declares `enum Priority { Low, Medium, High }` and a method `Labels::label` whose
//! body is the `match` under test.

mod common;

use common::check_src;
use nvs_diagnostics::{Diagnostics, code};

/// `Priority`, a class `Defaults` with `consts`, and `label($p)` returning `body`.
fn fixture(param: &str, consts: &str, body: &str) -> String {
    format!(
        "<?nvs\nenum Priority {{ Low, Medium, High }}\nclass Defaults {{\n{consts}\n}}\n\
         class Labels {{\n  public static function label({param} $p): string {{\n    return {body};\n  }}\n}}\n"
    )
}

fn misses(diags: &Diagnostics) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::E_MATCH_MISSES_A_CASE))
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn a_match_over_an_enum_that_misses_a_case_does_not_compile() {
    let diags = check_src(&fixture(
        "Priority",
        "",
        "match ($p) { Priority::Low => 'low' }",
    ));
    assert_eq!(
        misses(&diags),
        ["this `match` does not handle `Priority::Medium` and `Priority::High`"],
        "{diags:?}"
    );
    let every = check_src(&fixture(
        "Priority",
        "",
        "match ($p) { Priority::Low => 'low', Priority::Medium, Priority::High => 'high' }",
    ));
    assert!(!every.has_errors(), "{every:?}");
}

#[test]
fn a_match_with_a_default_arm_needs_no_other_case() {
    let diags = check_src(&fixture(
        "Priority",
        "",
        "match ($p) { Priority::Low => 'low', default => 'other' }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_match_over_a_nullable_enum_needs_a_null_arm() {
    let all = "Priority::Low => 'low', Priority::Medium => 'medium', Priority::High => 'high'";
    let diags = check_src(&fixture("?Priority", "", &format!("match ($p) {{ {all} }}")));
    assert_eq!(
        misses(&diags),
        ["this `match` does not handle `null`"],
        "{diags:?}"
    );
    let with_null = check_src(&fixture(
        "?Priority",
        "",
        &format!("match ($p) {{ {all}, null => 'none' }}"),
    ));
    assert!(!with_null.has_errors(), "{with_null:?}");
}

#[test]
fn an_arm_that_is_not_a_case_covers_nothing() {
    // The last arm calls a method that returns a case. A call is not a case written in the arm,
    // so the checker does not count it.
    let diags = check_src(&fixture(
        "Priority",
        "  public static function high(): Priority {\n    return Priority::High;\n  }",
        "match ($p) { Priority::Low => 'low', Priority::Medium => 'medium', Defaults::high() => 'high' }",
    ));
    assert_eq!(
        misses(&diags),
        ["this `match` does not handle `Priority::High`"],
        "{diags:?}"
    );
}

#[test]
fn a_class_constant_that_folds_to_a_case_covers_it() {
    let diags = check_src(&fixture(
        "Priority",
        "  const Priority URGENT = Priority::High;",
        "match ($p) { Priority::Low => 'low', Priority::Medium => 'medium', Defaults::URGENT => 'urgent' }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_match_over_mixed_or_a_union_is_unchanged() {
    let mixed = check_src(&fixture(
        "mixed",
        "",
        "match ($p) { Priority::Low => 'low' }",
    ));
    assert!(misses(&mixed).is_empty(), "{mixed:?}");
    let union = check_src(&fixture(
        "Priority|string",
        "",
        "match ($p) { Priority::Low => 'low' }",
    ));
    assert!(misses(&union).is_empty(), "{union:?}");
    let condition = check_src(&fixture(
        "Priority",
        "",
        "match (true) { $p == Priority::Low => 'low' }",
    ));
    assert!(misses(&condition).is_empty(), "{condition:?}");
}
