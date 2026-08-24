//! Literal and enum-case types — [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md),
//! at the M1/M2 boundary.
//!
//! The grammar is landed (`mwl_syntax::parser`'s own ADR 0047 tests), and § 4's
//! assignability and conversion table is not. What is pinned here is the seam
//! between the two: `check_src`'s own `assert!` proves each fixture *parses*,
//! and every one of the three atoms is then refused by name rather than
//! silently widened to its base type — see
//! [`mwl_diagnostics::code::E_LITERAL_TYPE_UNCHECKED`] for why refusing is the
//! only honest answer in between. When M2's row lands, these tests invert:
//! that is the signal the arm was removed deliberately rather than by accident.

mod common;

use common::*;
use mwl_diagnostics::code;

fn refuses(body: &str) -> bool {
    check_in_method(body)
        .iter()
        .any(|d| d.code == Some(code::E_LITERAL_TYPE_UNCHECKED))
}

/// § 1's two literal atoms, alone and unioned, and § 7's `?` sugar over one.
#[test]
fn a_literal_type_parses_and_is_refused_by_name() {
    for body in [
        r#""a" $mode = "a";"#,
        r#""a"|"b"|"c" $mode = "a";"#,
        "1 $one = 1;",
        "-1 $neg = -1;",
        "1|2|3 $small = 1;",
        r#"?"a" $maybe = null;"#,
    ] {
        assert!(refuses(body), "expected a refusal for {body}");
    }
}

/// §§ 2-3's one atom with two meanings — a class constant that folds and an
/// enum case that narrows. Neither is resolved yet, so both take the same
/// refusal; telling them apart is exactly what M2's slice adds.
#[test]
fn a_class_constant_or_enum_case_type_parses_and_is_refused_by_name() {
    let diags = check_src(concat!(
        "<?mwl\n",
        "class Foo { public const string TYPE_A = \"a\"; }\n",
        "enum Mode { Read, Write, Admin }\n",
        "class T {\n",
        "  function m(Foo::TYPE_A $c, Mode::Read|Mode::Write $m): void {}\n",
        "}\n",
    ));
    // Counted by span rather than by report: a parameter's type is lowered
    // once when the signature is collected and again when the body is
    // checked, so three atoms produce six reports.
    let mut spans: Vec<_> = diags
        .iter()
        .filter(|d| d.code == Some(code::E_LITERAL_TYPE_UNCHECKED))
        .filter_map(|d| d.labels.first().map(|l| l.span))
        .collect();
    spans.sort_by_key(|s| (s.start, s.end));
    spans.dedup();
    assert_eq!(
        spans.len(),
        3,
        "one per atom — the constant, and each of the two cases: {diags:?}"
    );
}

/// The refusal is scoped to ADR 0047's three atoms and nothing near them:
/// `true`/`false` were literal types before this ADR and take no refusal.
///
/// They do still fail to check, for the reason this ADR's own M2 row has to
/// answer for `"a"` too — a `true` *expression* types as `bool`, so nothing
/// satisfies a `true` *type* today. That is ADR 0047 § 4's first table row,
/// not this arm's business.
#[test]
fn the_bool_literal_atoms_are_untouched() {
    let diags = check_in_method("true $t = true;\nfalse $f = false;\n");
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_LITERAL_TYPE_UNCHECKED)),
        "{diags:?}"
    );
}
