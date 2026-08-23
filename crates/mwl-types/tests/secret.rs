//! ADR 0033's `secret` qualifier, independent of and composable with `tainted`.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

// ADR 0033 §§ 2-4: `secret`, the same shape as `tainted` on an
// independent axis.

#[test]
fn a_plain_string_is_assignable_into_a_secret_typed_target() {
    let diags = check_in_method(r#"secret string $s = "literal";"#);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_secret_value_is_not_assignable_into_a_plain_typed_target() {
    let diags = check_in_method(
        "secret string $s = \"literal\";\n\
         string $out = $s;\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn concatenating_a_secret_operand_poisons_the_result_independently_of_tainted() {
    // `secret` poisons through concatenation exactly like `tainted`,
    // with no `tainted` qualifier anywhere in sight — the two axes are
    // independent.
    let diags = check_in_method(
        "secret string $s = \"literal\";\n\
         string $out = $s . \"x\";\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn interpolating_a_secret_operand_poisons_the_result() {
    let diags = check_in_method(
        "secret string $s = \"literal\";\n\
         string $out = \"value: $s\";\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_secret_tainted_operand_poisons_both_axes_through_concatenation() {
    let diags = check_in_method(
        "secret tainted string $s = \"literal\" as secret tainted string;\n\
         tainted string $out = $s . \"x\";\n",
    );
    // `secret tainted` concatenated with a plain string stays poisoned on
    // both axes, so it satisfies neither a plain-`tainted` target...
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    let diags = check_in_method(
        "secret tainted string $s = \"literal\" as secret tainted string;\n\
         secret tainted string $out = $s . \"x\";\n",
    );
    // ...but does satisfy the same, fully-qualified target.
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_checked_conversion_strips_secret_the_same_way_it_strips_tainted() {
    // ADR 0033 § 2's known, accepted gap: "shape-proof implies safe"
    // never actually justified stripping `secret`, but the rule is kept
    // for consistency with `tainted` anyway.
    let diags = check_in_method(
        "secret tainted string $s = \"literal\" as secret tainted string;\n\
         uint $n = $s as uint;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn as_string_does_not_launder_a_secret_source() {
    let diags = check_in_method(
        "secret string $s = \"literal\";\n\
         string $out = $s as string;\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn converting_a_secret_string_to_bytes_preserves_the_qualifier() {
    let diags = check_in_method(
        "secret string $s = \"literal\";\n\
         secret bytes $b = $s as bytes;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_secret_value_converted_to_markup_is_diagnosed_naming_secret() {
    let diags = check_in_method(
        "secret string $s = \"literal\";\n\
         Core\\Html\\Markup $m = $s as Core\\Html\\Markup;\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_MARKUP_UNSUPPORTED)),
        "{diags:?}"
    );
}

#[test]
fn a_secret_value_passed_directly_to_a_throwable_is_diagnosed() {
    let diags = check_in_method(
        "secret string $s = \"literal\";\n\
         throw new LogicError($s);\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_THROWABLE_MESSAGE)),
        "{diags:?}"
    );
}

#[test]
fn a_secret_value_passed_to_a_subclass_of_throwable_is_diagnosed() {
    let diags = check_src(
        "<?mwl\n\
         class MyError extends Throwable {}\n\
         class T {\n  function m(secret string $s): void {\n\
         throw new MyError($s);\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_THROWABLE_MESSAGE)),
        "{diags:?}"
    );
}

#[test]
fn a_plain_value_passed_to_a_throwable_is_fine() {
    let diags = check_in_method(r#"throw new LogicError("plain message");"#);
    assert!(!diags.has_errors(), "{diags:?}");
}
