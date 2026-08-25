//! Literal and enum-case types — [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md)
//! §§ 1-3, at the point each atom becomes a real type.
//!
//! What is pinned here is *what an atom interns to*, not yet what a value of
//! one may be assigned to: § 4's assignability and conversion table is its own
//! slice, and until it lands a literal-typed binding still fails to check the
//! same way a `true`-typed one always has. The two facts this file exists to
//! hold are the ones § 3 turns on — an enum case is **not** an int literal of
//! its backing value, and a class constant **is** its value's own literal type
//! — because unifying either one reopens
//! [ADR 0010](../../../docs/adr/0010-enums-are-a-value-type.md) § 5's hole.

mod common;

use common::*;
use mwl_diagnostics::code;
use mwl_types::ty::Ty;

/// § 1's two literal atoms: each interns to its own singleton type, holding
/// the value it names rather than the text it was written as.
#[test]
fn a_literal_type_atom_is_checked() {
    let (diags, types) = check_src_declared(concat!(
        "<?mwl\n",
        "class T {\n",
        "  function m(\"a\" $s, 1 $one, -1 $neg, 0x10 $hex): void {}\n",
        "}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(
        types.interner.get(types.of("\"a\"", "$s")),
        &Ty::StringLiteral("a".to_owned())
    );
    assert_eq!(
        types.interner.get(types.of("1", "$one")),
        &Ty::IntLiteral(1)
    );
    assert_eq!(
        types.interner.get(types.of("-1", "$neg")),
        &Ty::IntLiteral(-1)
    );
    // The one integer grammar, not a second one: a radix prefix reads here
    // exactly as it does in a value position.
    assert_eq!(
        types.interner.get(types.of("0x10", "$hex")),
        &Ty::IntLiteral(16)
    );
}

/// § 1 again, through the two spellings that wrap an atom — a union of
/// literals, and `?` sugar over one. Compared against a rebuilt union rather
/// than a `describe` string, since a union orders its members by type id.
#[test]
fn a_union_of_literal_atoms_is_one_closed_set() {
    let (diags, mut types) = check_src_declared(concat!(
        "<?mwl\n",
        "class T {\n",
        "  function m(\"a\"|\"b\"|\"c\" $mode, ?\"x\" $maybe): void {}\n",
        "}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let mode = types.of("\"a\"|\"b\"|\"c\"", "$mode");
    let maybe = types.of("?\"x\"", "$maybe");

    let members: Vec<_> = ["a", "b", "c"]
        .into_iter()
        .map(|v| types.interner.string_literal(v))
        .collect();
    let expected = types.interner.make_union(members);
    assert_eq!(mode, expected);

    let x = types.interner.string_literal("x");
    let null = types.interner.null();
    let expected = types.interner.make_union([x, null]);
    assert_eq!(maybe, expected);
}

/// §§ 2-3's one atom with two meanings, told apart by what the name resolves
/// to — and the fact that makes § 3 worth having: `Mode::Read` and the int
/// literal type of its backing value `0` are **two** types, never one.
#[test]
fn a_class_constant_folds_and_an_enum_case_narrows() {
    let (diags, mut types) = check_src_declared(concat!(
        "<?mwl\n",
        "class Foo { public const string TYPE_A = \"a\"; public const int RANK = 7; }\n",
        "enum Mode { Read, Write, Admin }\n",
        "class T {\n",
        "  function m(Foo::TYPE_A $c, Foo::RANK $r, Mode::Read $m): void {}\n",
        "}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // § 2: the constant is sugar for its value's own literal type, so it is
    // the *same* interned type as writing that value out.
    let written = types.interner.string_literal("a");
    assert_eq!(types.of("Foo::TYPE_A", "$c"), written);
    let written = types.interner.int_literal(7);
    assert_eq!(types.of("Foo::RANK", "$r"), written);

    // § 3: the case is a narrowed subtype of its enum, not its backing value.
    assert_eq!(
        types.interner.get(types.of("Mode::Read", "$m")),
        &Ty::EnumCase(
            mwl_hir::QName::parse("Mode"),
            mwl_types::EnumBacking::Int,
            "Read".to_owned()
        )
    );
    let zero = types.interner.int_literal(0);
    assert_ne!(
        types.of("Mode::Read", "$m"),
        zero,
        "folding a case to its backing value is exactly the hole ADR 0010 § 5 closed"
    );
}

/// § 3's case-subset union is its own type, and § 4's fourth row widens it to
/// exactly its enum — the one row of that table `TypeInterner::literal_base`
/// already answers.
#[test]
fn a_case_subset_union_is_not_its_enum() {
    let (diags, mut types) = check_src_declared(concat!(
        "<?mwl\n",
        "enum Mode { Read, Write, Admin }\n",
        "class T {\n",
        "  function m(Mode::Read|Mode::Write $subset, Mode $whole): void {}\n",
        "}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let subset = types.of("Mode::Read|Mode::Write", "$subset");
    let whole = types.of("Mode", "$whole");
    assert_ne!(subset, whole);
    assert_eq!(types.interner.literal_base(subset), whole);
}

/// § 2's eligibility rule: a constant whose value has no literal type to fold
/// to is a diagnostic naming the eligible types, not a silent widening.
#[test]
fn an_ineligible_class_constant_in_type_position_is_diagnosed() {
    let diags = check_src(concat!(
        "<?mwl\n",
        "class Foo { public const float RATE = 1.5; }\n",
        "class T {\n",
        "  function m(Foo::RATE $r): void {}\n",
        "}\n",
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_LITERAL_TYPE_NOT_CONST)),
        "{diags:?}"
    );
}

/// A name nothing declares is the *other* mistake, and takes the same
/// diagnostic it takes on the expression side.
#[test]
fn an_undeclared_constant_or_case_in_type_position_is_diagnosed() {
    for src in [
        concat!(
            "<?mwl\n",
            "class Foo { public const string TYPE_A = \"a\"; }\n",
            "class T { function m(Foo::NOPE $c): void {} }\n",
        ),
        concat!(
            "<?mwl\n",
            "enum Mode { Read }\n",
            "class T { function m(Mode::Nope $m): void {} }\n",
        ),
    ] {
        let diags = check_src(src);
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
            "{diags:?}"
        );
    }
}

/// § 1's atom is an `int` literal, so a magnitude no `int` holds has no
/// literal type to be — the same diagnostic the identical mistake takes in a
/// value position.
#[test]
fn an_out_of_range_int_literal_type_is_diagnosed() {
    let diags = check_src(concat!(
        "<?mwl\n",
        "class T { function m(99999999999999999999 $n): void {} }\n",
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
        "{diags:?}"
    );
}

/// § 5's promise, from the checker's side: a literal type's base is what every
/// question about representation is answered against, and a union widens
/// member-wise rather than to itself.
#[test]
fn a_literal_type_widens_to_its_base() {
    let (diags, mut types) = check_src_declared(concat!(
        "<?mwl\n",
        "class T { function m(\"a\"|\"b\" $mode, 1 $one): void {} }\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let mode = types.of("\"a\"|\"b\"", "$mode");
    let string = types.interner.string();
    assert_eq!(types.interner.literal_base(mode), string);

    let one = types.of("1", "$one");
    let int = types.interner.int();
    assert_eq!(types.interner.literal_base(one), int);
}
