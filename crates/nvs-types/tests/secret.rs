//! `rule:security/secret-qualifier`'s `secret` qualifier, independent of and composable with `tainted`.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

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
    // `rule:security/secret-propagation`'s known, accepted gap: "shape-proof implies safe"
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
        "<?nvs\n\
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
fn a_secret_value_is_refused_at_the_boundary_unless_revealed() {
    // `rule:security/secret-sinks-refuse`'s `serialize()`-and-`spawn` bullet, at the carrier that
    // compiles today. `Core\Serialize::encode` declares `mixed`, which a
    // `secret string` satisfies, so this is a call-site rule and the written
    // argument is where it is reported.
    let refused = check_in_method(
        "secret string $token = \"literal\";\n\
         bytes $b = Core\\Serialize::encode($token);\n",
    );
    assert!(
        refused
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_CROSSES_A_BOUNDARY)),
        "{refused:?}"
    );

    // The "unless" half: the refusal is on the qualifier, not on the copy, so
    // a plain value crosses with nothing written at the call site. That plain
    // type is what `Core\Secret::reveal(..., "reason")` — which the diagnostic's
    // help names and the registry does not carry yet — produces, so this pins
    // the shape the escape hatch has to land in rather than the call.
    let allowed = check_in_method(
        "string $revealed = \"literal\";\n\
         bytes $b = Core\\Serialize::encode($revealed);\n",
    );
    assert!(!allowed.has_errors(), "{allowed:?}");
}

#[test]
fn a_secret_value_reaching_the_boundary_inside_a_concatenation_is_refused() {
    // The qualifier poisons through concatenation on its own axis, so a
    // secret that reaches the copy composed rather than named is the same
    // refusal — which is the point of reading the argument's inferred type
    // rather than looking for a `secret`-typed variable.
    let diags = check_in_method(
        "secret string $token = \"literal\";\n\
         bytes $b = Core\\Serialize::encode(\"bearer \" . $token);\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_CROSSES_A_BOUNDARY)),
        "{diags:?}"
    );
}

#[test]
fn a_secret_operand_at_log_write_fields_is_refused_despite_the_open_type() {
    // `rule:security/secret-sinks-refuse`'s log bullet, and the one sink whose parameter type is
    // deliberately not the thing that refuses: `fields` stays `array<mixed>`,
    // which a `secret string` element satisfies, so nothing below the call
    // site can tell. Both halves of the bullet are here — the bag that carries
    // the qualifier on its own type, and § 4's named case of the value written
    // inside the literal at the call.
    let bag = check_in_method(
        "secret string $token = \"literal\";\n\
         array<secret string> $bag = [\"token\" => $token];\n\
         Core\\Log::write(Core\\Log\\Level::Info, \"auth\", $bag);\n",
    );
    assert!(
        bag.iter().any(|d| d.code == Some(code::E_SECRET_LOGGED)),
        "{bag:?}"
    );

    let literal = check_in_method(
        "secret string $token = \"literal\";\n\
         Core\\Log::write(Core\\Log\\Level::Info, \"auth\", [\"token\" => $token]);\n",
    );
    assert!(
        literal
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_LOGGED)),
        "{literal:?}"
    );

    // The open type is open, which is the other half of the claim: the same
    // call carrying an ordinary field is a record the rule says nothing about.
    let allowed = check_in_method(
        "string $user = \"literal\";\n\
         Core\\Log::write(Core\\Log\\Level::Info, \"auth\", [\"user\" => $user]);\n",
    );
    assert!(!allowed.has_errors(), "{allowed:?}");
}

#[test]
fn a_secret_argument_to_push_is_refused_where_the_call_is_written() {
    // `rule:security/secret-sinks-refuse`'s serialiser bullet reached one
    // member further on: `Core\Queue::push` encodes its `args` payload into a
    // durable row with `Core\Json::encode`'s own encoder, so it reports that
    // bullet's code — and it has to report it *here*, because `args` is
    // declared `mixed` and the bag arrives as the registry's own `CoreShape`,
    // which is why no declared type below the call site can tell.
    let named = check_in_method(
        "secret string $token = \"literal\";\n\
         Core\\Queue::push(\"send-mail.nvs\", {args: $token});\n",
    );
    assert!(
        named.iter().any(|d| d.code == Some(code::E_SECRET_ENCODED)),
        "{named:?}"
    );

    // The shape the rule exists for: one credential among public fields,
    // written inline at the call in either container spelling.
    for payload in [r#"["to" => "a@b", "token" => $token]"#, "{token: $token}"] {
        let inline = check_in_method(&format!(
            "secret string $token = \"literal\";\n\
             Core\\Queue::push(\"send-mail.nvs\", {{args: {payload}}});\n"
        ));
        assert!(
            inline
                .iter()
                .any(|d| d.code == Some(code::E_SECRET_ENCODED)),
            "{payload}: {inline:?}"
        );
    }

    // The open type stays open, which is the other half of the claim, and the
    // bag's other options are left to their own declared types: `queue` is a
    // `string`, so a `secret` there is the ordinary mismatch and not a second
    // report of this one.
    let allowed = check_in_method(
        "string $to = \"a@b\";\n\
         Core\\Queue::push(\"send-mail.nvs\", {args: [\"to\" => $to], queue: \"mail\"});\n",
    );
    assert!(!allowed.has_errors(), "{allowed:?}");
}

#[test]
fn a_plain_value_passed_to_a_throwable_is_fine() {
    let diags = check_in_method(r#"throw new LogicError("plain message");"#);
    assert!(!diags.has_errors(), "{diags:?}");
}
