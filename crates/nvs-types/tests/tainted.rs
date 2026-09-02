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

// ADR 0133: the escaped value is a carrier, and the two halves of that which
// cannot be read off a registry row. `Core\Html::escape`'s return type is a
// signature and `nvs-stdlib` asserts it; these are claims about the *language*
// — what `.` admits and what `as` converts — and only this crate can make them.

#[test]
fn markup_is_not_stringable_and_has_no_concat_row() {
    // ADR 0133 § 2: `tainted string $line = "Hello " . $m;` is refused. The
    // claim is not about the assignment's qualifier — it is that `.` has no
    // row for the carrier at all, so the refusal stands whatever the target
    // type is. `.` otherwise admits a `Stringable` object, which is exactly why
    // this needs asserting: a carrier that grew a `toString` would start
    // concatenating silently, and the eager-escape bug ADR 0133 removes would
    // be back with `.` spelling it instead of `escape`.
    let diags = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         string $line = \"Hello \" . $m;\n",
    );
    assert!(
        diags.has_errors(),
        "`.` has no row for a carrier: {diags:?}"
    );

    // The other side of the same bound, so a rule that refused every operand
    // of `.` fails here: `+` *does* have a row for two carriers, and it
    // produces the carrier back. That is ADR 0024 § 5's composition, and ADR
    // 0133 § 2 leans on it — the correction it offers the reader is to write
    // `+` where they wrote `.`, so `+` has to work.
    let diags = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         Core\\Html\\Markup $both = (\"Hello \" as Core\\Html\\Markup) + $m;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    // And `+` refuses the mixed pair rather than escaping the plain half. An
    // operator that escaped silently would be the surprise this ADR removes.
    let diags = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         Core\\Html\\Markup $bad = $m + \"x\";\n",
    );
    assert!(
        diags.has_errors(),
        "`+` composes two carriers and lifts neither: {diags:?}"
    );
}

#[test]
fn markup_does_not_convert_to_string() {
    // ADR 0133 § 3: there is no `Markup as string`, because one would reopen
    // the hole in a keystroke — `Core\Html::escape($x) as string . $tainted` is
    // the *Context* bug with an extra word in it. `Core\Html::toSource` is the
    // only way out and it is a member, so it is greppable and carries a written
    // reason at the site.
    let diags = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         string $s = $m as string;\n",
    );
    assert!(
        diags.has_errors(),
        "`Markup as string` is not a conversion: {diags:?}"
    );

    // Nor by assignment, which is the same claim without the operator: the
    // carrier is a class type and a `string` target does not admit one.
    let diags = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         string $s = $m;\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );

    // The exit that does exist, asserted alongside so that a rule closing the
    // conversion by closing the type fails here. `toSource` answers a plain
    // `string` — not a `tainted` one, since the bytes were already laundered.
    let diags = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         string $s = Core\\Html::toSource($m, \"the one way out\");\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}
