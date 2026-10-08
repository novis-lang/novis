//! `rule:security/tainted-qualifier`'s `tainted` qualifier — propagation, laundering, and the sinks that refuse it.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// `rule:security/taint-propagation` and `rule:security/launderers-are-sink-named`: `tainted` propagation and laundering.

#[test]
fn a_plain_string_is_assignable_into_a_tainted_typed_target() {
    // `rule:security/taint-propagation`: a trusted value is always a safe over-approximation
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
    // `rule:security/taint-propagation`'s own example: `as uint` already throws on a
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

// `rule:security/unclassified-parameter-refuses-tainted`: what a registry row's parameter classification does at a call.
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
    // `rule:types/conversion`, amended by `rule:security/taint-propagation`: `bytes`/`string` conversion
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
fn converting_a_written_string_to_markup_is_fine() {
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
            .any(|d| d.code == Some(code::E_MARKUP_NEEDS_WRITTEN_STRING)),
        "{diags:?}"
    );
}

#[test]
fn serialize_decode_refuses_a_tainted_operand() {
    // `rule:classes/serialize-is-a-closed-format`'s last bullet and `rule:security/sink-predicate`: `Core\Serialize::decode`'s
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
    // `rule:core-classes/html-auto-escape`: only a literal token qualifies — even an untainted
    // runtime value is refused.
    let diags = check_in_method(
        "string $s = \"literal\";\n\
         Core\\Html\\Markup $m = $s as Core\\Html\\Markup;\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MARKUP_NEEDS_WRITTEN_STRING)),
        "{diags:?}"
    );
}

// `rule:security/launderer-answers-a-carrier`: the escaped value is a carrier, and the two halves of that which
// cannot be read off a registry row. `Core\Html::escape`'s return type is a
// signature and `nvs-stdlib` asserts it; these are claims about the *language*
// — what `.` admits and what `as` converts — and only this crate can make them.

#[test]
fn markup_is_not_stringable_and_has_no_concat_row() {
    // `rule:core-classes/html-escape-answers-markup`: `tainted string $line = "Hello " . $m;` is refused. The
    // claim is not about the assignment's qualifier — it is that `.` has no
    // row for the carrier at all, so the refusal stands whatever the target
    // type is. `.` otherwise admits a `Stringable` object, which is exactly why
    // this needs asserting: a carrier that grew a `toString` would start
    // concatenating silently, and the eager-escape bug `rule:security/launderer-answers-a-carrier` removes would
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
    // produces the carrier back. That is `rule:core-classes/html-auto-escape`'s composition, and ADR
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
    // `rule:core-classes/html-to-source`: there is no `Markup as string`, because one would reopen
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

#[test]
fn a_computed_to_source_reason_is_refused_at_the_call() {
    // `rule:core-classes/html-to-source`'s second sentence, which `nvs_types::reasons` owns: the
    // reason is what makes this hatch safe, so a computed one is refused where
    // it is written rather than left to a run time that would accept any
    // string at all.
    let computed = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         string $why = \"cached fragment\";\n\
         string $s = Core\\Html::toSource($m, $why);\n",
    );
    assert!(
        computed
            .iter()
            .any(|d| d.code == Some(code::E_REASON_NOT_WRITTEN_DIRECTLY)),
        "{computed:?}"
    );

    // Concatenation is the same refusal and the interesting half of it: half
    // the text is in the source, and a reason that is *partly* written is one
    // nobody wrote.
    let joined = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         string $who = \"the cache\";\n\
         string $s = Core\\Html::toSource($m, \"stored by \" . $who);\n",
    );
    assert!(
        joined
            .iter()
            .any(|d| d.code == Some(code::E_REASON_NOT_WRITTEN_DIRECTLY)),
        "{joined:?}"
    );

    // The other half of § 3's sentence is *not* this pass's, asserted here so
    // that widening it fails a test rather than a conformance case: an empty
    // literal is written text, and `to-source-refuses-an-empty-reason-….nvst`
    // pins the body's throw over it.
    let empty = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         string $s = Core\\Html::toSource($m, \"   \");\n",
    );
    assert!(!empty.has_errors(), "{empty:?}");

    // A `name:` argument is read at the parameter it fills, wherever it was
    // written — `rule:core-api/parameters-are-callable-by-name`, through the
    // argument mapping `nvs_types::expr::args` hands this pass.
    let named = check_in_method(
        "Core\\Html\\Markup $m = \"<b>\" as Core\\Html\\Markup;\n\
         string $why = \"cached fragment\";\n\
         string $s = Core\\Html::toSource(reason: $why, markup: $m);\n",
    );
    assert!(
        named
            .iter()
            .any(|d| d.code == Some(code::E_REASON_NOT_WRITTEN_DIRECTLY)),
        "{named:?}"
    );
}

#[test]
fn a_const_reason_is_written_in_the_code_and_compiles() {
    // The refusal above is "not in the source", not "not a string literal
    // token". A `const` folds, so the justification is still greppable and
    // still readable at the site — which is the shape a long reason wants, and
    // `Core\Secret::reveal`'s own position on the same question
    // (`nvs_stdlib::html`'s module doc).
    let diags = check_src(
        "<?nvs
class Fragments {
    const string WHY = \"the cache stores rendered bytes, not the guarantee\";

    public function store(Core\\Html\\Markup $m): string
    {
        return Core\\Html::toSource($m, Fragments::WHY);
    }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

// `rule:security/tainted-qualifier`: `tainted {…}` over a shape, which the parser
// distributes to every text-carrying field and then erases.

#[test]
fn tainted_over_a_shape_taints_every_text_field() {
    // The qualifier reached `a`, so a tainted value fits it. Without the
    // distribution the field would be a plain `string` and this would be the
    // mismatch `a_tainted_value_is_not_assignable_into_a_plain_typed_target`
    // pins one storage kind along.
    let diags = check_in_method(
        "tainted {a: string, n: int} $s = {a: \"literal\" as tainted string, n: 1};\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_tainted_shape_is_not_assignable_to_the_same_shape_unqualified() {
    // The distribution is what makes this fall out of rules that already stand:
    // the qualified shape's fields are `tainted string`, the unqualified one's
    // are `string`, and shape assignability is field-by-field ordinary
    // assignability (`rule:types/shape-type`) over an axis that only widens one
    // way (`a_tainted_value_is_not_assignable_into_a_plain_typed_target`). So a
    // shape is no laundering route, and no rule of its own says so.
    let diags = check_in_method(
        "tainted {a: string} $t = {a: \"literal\" as tainted string};\n\
         ({a: string}) $plain = $t;\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_field_read_off_a_tainted_shape_is_tainted() {
    let diags = check_in_method(
        "tainted {a: string} $s = {a: \"literal\" as tainted string};\n\
         string $bad = $s->a;\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn tainted_reaches_through_an_array_element_and_a_nested_shape() {
    // The two spellings are one type, so each is assignable into the other.
    // Asserting it in both directions is what makes this the distribution
    // rather than one-directional widening.
    let diags = check_src(
        "<?nvs
type Qualified = tainted {rows: array<string>, inner: {b: string}};
type Spelled = {rows: array<tainted string>, inner: {b: tainted string}};
class T {
    public function m(Qualified $a, Spelled $b): void
    {
        $b = $a;
        $a = $b;
    }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_tainted_shape_naming_no_text_field_is_refused() {
    // A qualifier that promises nothing still reads as a promise, so the empty
    // distribution is a diagnostic rather than a no-op. Written in a parameter,
    // because a local's type is trial-parsed and any diagnostic raised inside
    // one backtracks the whole statement (`crates/nvs-syntax/src/parser/stmt.rs:956`).
    let diags = check_src_allowing_parse_errors(
        "<?nvs
class T {
    public function m(tainted {n: int} $p): void {}
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TAINTED_SHAPE_HAS_NO_TEXT)),
        "{diags:?}"
    );
}

// `rule:security/taint-propagation`: text out of `mixed` is `tainted`.

#[test]
fn every_way_text_leaves_mixed_answers_the_tainted_form() {
    let diags = check_in_method(
        "mixed $m = 1;\n\
         tainted string $a = $m as string;\n\
         ?tainted string $b = $m as ?string;\n\
         tainted bytes $c = $m as bytes;\n\
         array<tainted string> $d = $m as array<string>;\n\
         secret tainted string $e = $m as secret string;\n\
         tainted string $f = \"x\" . $m;\n\
         tainted string $g = \"x $m\";\n\
         int $h = $m as int;\n\
         bool $i = $m as bool;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn text_out_of_mixed_is_not_assignable_into_a_plain_string() {
    for line in [
        "string $s = $m as string;",
        "string $s = \"x\" . $m;",
        "string $s = \"x $m\";",
        "?string $s = $m as ?string;",
        "array<string> $s = $m as array<string>;",
    ] {
        let diags = check_in_method(&format!("mixed $m = 1;\n{line}\n"));
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "{line}: {diags:?}"
        );
    }
}

#[test]
fn a_conversion_keeps_tainted_through_a_nullable_and_an_array() {
    for line in [
        "?tainted string $t = null;\nstring $s = $t as string;",
        "array<tainted string> $t = [];\narray<string> $s = $t as array<string>;",
    ] {
        let diags = check_in_method(line);
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "{line}: {diags:?}"
        );
    }
}

#[test]
fn a_shape_or_binding_receiving_text_out_of_mixed_must_be_written_tainted() {
    for (line, refused) in [
        ("$p = ($m as {n: string})->n;", true),
        ("$p = ($m as tainted {n: string})->n;", false),
        ("$p = ($m as {n: int})->n;", false),
        ("if ($m is {n: string}) {}", true),
        ("if ($m is tainted {n: string}) {}", false),
        ("foreach ($m as string $v) {}", true),
        ("foreach ($m as tainted string $v) {}", false),
    ] {
        let diags = check_in_method(&format!("mixed $m = 1;\n{line}\n"));
        let got = diags
            .iter()
            .any(|d| d.code == Some(code::E_UNCHECKED_TEXT_NOT_TAINTED));
        assert_eq!(got, refused, "{line}: {diags:?}");
    }
}
