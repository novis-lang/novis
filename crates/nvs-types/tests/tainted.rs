//! ADR 0024's `tainted` qualifier — propagation, laundering, and the sinks that refuse it.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// ADR 0024 §§ 2-3: `tainted` propagation and laundering.

#[test]
fn a_plain_string_is_assignable_into_a_tainted_typed_target() {
    // ADR 0024 § 2: a trusted value is always a safe over-approximation
    // of "may be tainted" — the one-directional widening this ADR adds,
    // mirrored from `mixed`'s own one-directional rule.
    let diags = check_in_method(r#"tainted string $t = "literal";"#);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_tainted_value_is_not_assignable_into_a_plain_typed_target() {
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         string $s = $t;\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn concatenating_a_tainted_operand_poisons_the_result() {
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         string $s = $t . \"x\";\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn interpolating_a_tainted_operand_poisons_the_result() {
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         string $s = \"value: $t\";\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn concatenating_two_untainted_operands_stays_untainted() {
    let diags = check_in_method(r#"string $s = "a" . "b";"#);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_checked_conversion_launders_a_tainted_source() {
    // ADR 0024 § 2's own example: `as uint` already throws on a
    // malformed shape, so a value that survives it is proven safe.
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         uint $n = $t as uint;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn as_string_does_not_launder_a_tainted_source() {
    // The identity-shaped conversion this ADR must not treat as
    // laundering — otherwise `$tainted as string` would be a silent
    // bypass of the whole mechanism.
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         string $s = $t as string;\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

// ADR 0088 § 2: what a registry row's parameter classification does at a call.
// The `Sink` half is pinned by
// `tests/conformance/reject/bytes-pack-and-unpack-refuse-a-tainted-format.nvst`
// over five routes, so what is asserted here is the two marks that *accept*.

#[test]
fn a_neutral_parameter_admits_a_tainted_argument() {
    // `Core\Str::length` answers a `uint`, which carries no byte of its
    // subject — so there is nothing for the qualifier to be carried into and
    // nothing the admission can launder.
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         uint $n = Core\\Str::length($t);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_contagious_parameter_admits_a_tainted_argument_and_taints_the_result() {
    // `Core\Str::join`'s separator decides which bytes come back, so the
    // answer is tainted whenever the separator was.
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         tainted string $s = Core\\Str::join([\"a\", \"b\"], $t);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_contagious_call_does_not_launder_its_argument() {
    // The other half of the same rule, and the half that is a security
    // property rather than a convenience: admitting the argument without
    // carrying the qualifier into the answer would make every `Core` member a
    // laundering hole.
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         string $s = Core\\Str::join([\"a\", \"b\"], $t);\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn converting_a_tainted_string_to_bytes_preserves_the_qualifier() {
    // ADR 0009 § 3, amended by ADR 0024 § 2: `bytes`/`string` conversion
    // preserves `tainted` across either direction.
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         tainted bytes $b = $t as bytes;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn converting_a_tainted_string_to_bytes_does_not_launder_it() {
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         bytes $b = $t as bytes;\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn converting_a_literal_string_to_markup_is_fine() {
    let diags = check_in_method("Core\\Html\\Markup $m = \"literal\" as Core\\Html\\Markup;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn converting_a_tainted_string_to_markup_is_diagnosed() {
    let diags = check_in_method(
        "tainted string $t = \"literal\" as tainted string;\n\
         Core\\Html\\Markup $m = $t as Core\\Html\\Markup;\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MARKUP_REQUIRES_LITERAL)),
        "{diags:?}"
    );
}

#[test]
fn serialize_decode_refuses_a_tainted_operand() {
    // ADR 0023 § 3's last bullet and ADR 0088 § 1: `Core\Serialize::decode`'s
    // parameter is `CoreTy::Blob(Qual::Sink)`, so a payload that came in off
    // the wire is refused where `Core\Json::decode`'s `Text(Qual::Sink)` is —
    // and the danger is not the same one. A JSON decode produces `mixed` a
    // program then has to narrow; this format names *classes* and rebuilds
    // their declared properties, so an attacker-chosen payload picks which
    // class the program is handed. There is no launderer for it: the way in
    // is `Core\Json::decode` plus a checked conversion, not a cast.
    let diags = check_in_method(
        "tainted string $wire = \"payload\" as tainted string;\n\
         mixed $v = Core\\Serialize::decode($wire as bytes);\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn serialize_decode_takes_a_plain_operand() {
    // The other side of the bound above: the sink refuses the qualifier, not
    // the type, so bytes the program itself produced cross with nothing
    // written at the call site.
    let diags = check_in_method(
        "bytes $b = Core\\Serialize::encode(1);\n\
         mixed $v = Core\\Serialize::decode($b);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn converting_a_runtime_computed_untainted_string_to_markup_is_still_diagnosed() {
    // ADR 0024 § 5: only a literal token qualifies — even an untainted
    // runtime value is refused.
    let diags = check_in_method(
        "string $s = \"literal\";\n\
         Core\\Html\\Markup $m = $s as Core\\Html\\Markup;\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MARKUP_REQUIRES_LITERAL)),
        "{diags:?}"
    );
}
