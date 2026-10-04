//! `rule:core-classes/html-template`: what ``html`…` `` types as, and what one of its holes admits.
//!
//! The literal's own lexing and parsing are `nvs-syntax`'s tests; these are the
//! two questions the checker answers — the node's type, and the three
//! qualifiers a hole can arrive carrying. See `tests/common/mod.rs` for the
//! shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

/// The node, not the part count, is what says `Core\Html\Markup`: a hole-free
/// body is still the carrier, where a quoted literal of the same text would be
/// a `string`.
#[test]
fn a_markup_literal_types_as_core_html_markup() {
    let diags = check_in_method("Core\\Html\\Markup $m = html`<hr>`;\n");
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_in_method("string $s = html`<hr>`;\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// A hole already holding a `Markup` is spliced raw — `Markup + Markup`
/// written in interpolation syntax — so the carrier-as-text refusal that an
/// ordinary interpolation raises must not fire here.
#[test]
fn a_markup_typed_hole_needs_no_conversion() {
    let diags = check_in_method(
        "Core\\Html\\Markup $inner = html`<b>x</b>`;\n\
         Core\\Html\\Markup $page = html`<div>{$inner}</div>`;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_CORE_CLASS_NOT_STRINGABLE)),
        "{diags:?}"
    );
}

/// Escaping neutralises injection structurally, so the sink never
/// distinguishes tainted from untainted: the hole is admitted and the
/// qualifier does not spread to the literal
/// (`rule:core-classes/html-auto-escape`).
#[test]
fn a_tainted_hole_is_admitted_because_the_sink_escapes_it() {
    let diags = check_in_method(
        "tainted string $name = \"literal\" as tainted string;\n\
         Core\\Html\\Markup $m = html`<span>{$name}</span>`;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// Escaping does nothing for confidentiality — an escaped credential is still
/// a leaked one — so the hole is refused where it is written
/// (`rule:security/secret-sinks-refuse`).
#[test]
fn a_secret_hole_is_refused_where_it_is_written() {
    let diags = check_in_method(
        "secret string $token = \"literal\";\n\
         Core\\Html\\Markup $m = html`<span>{$token}</span>`;\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_SECRET_OUTPUT)),
        "{diags:?}"
    );
}
