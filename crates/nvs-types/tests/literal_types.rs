//! Literal and enum-case types — `rule:types/single-value-types`, `rule:types/constant-in-type-position` and `rule:types/enum-case-type`, at the point each atom becomes a real type and a value of one
//! becomes writable.
//!
//! Three halves are pinned here: *what an atom interns to* (§§ 1-3), § 4's
//! free-widening rows plus the placement rule that makes a value of one of
//! these types writable at all, and § 4's checked `as` with § 6's two
//! diagnostics. What is **not** here is the run-time membership test a
//! conversion from `mixed` performs, which is `nvs-ir`'s and `nvs-runtime`'s.
//!
//! The two facts this file exists to hold are the ones § 3 turns on — an enum
//! case is **not** an int literal of its backing value, and a class constant
//! **is** its value's own literal type — because unifying either one reopens
//! `rule:types/conversion`'s hole.

mod common;

use common::*;
use nvs_diagnostics::code;
use nvs_types::ty::Ty;

/// § 1's two literal atoms: each interns to its own singleton type, holding
/// the value it names rather than the text it was written as.
#[test]
fn a_literal_type_atom_is_checked() {
    let (diags, types) = check_src_declared(concat!(
        "<?nvs\n",
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
        "<?nvs\n",
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
        "<?nvs\n",
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
            nvs_hir::QName::parse("Mode"),
            nvs_types::EnumBacking::Int,
            "Read".to_owned()
        )
    );
    let zero = types.interner.int_literal(0);
    assert_ne!(
        types.of("Mode::Read", "$m"),
        zero,
        "folding a case to its backing value is exactly the hole `rule:types/conversion` closed"
    );
}

/// § 3's case-subset union is its own type, and § 4's fourth row widens it to
/// exactly its enum — the one row of that table `TypeInterner::literal_base`
/// already answers.
#[test]
fn a_case_subset_union_is_not_its_enum() {
    let (diags, mut types) = check_src_declared(concat!(
        "<?nvs\n",
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
        "<?nvs\n",
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
            "<?nvs\n",
            "class Foo { public const string TYPE_A = \"a\"; }\n",
            "class T { function m(Foo::NOPE $c): void {} }\n",
        ),
        concat!(
            "<?nvs\n",
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
        "<?nvs\n",
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
        "<?nvs\n",
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

/// § 4's producer half: a literal expression takes `rule:types/single-value-types`'s singleton
/// type from the position it lands in, so writing the value out is what
/// satisfies the type — the step without which nothing but an `as` ever could.
#[test]
fn a_literal_expression_satisfies_the_literal_type_its_position_names() {
    let diags = check_in_method(concat!(
        "    \"a\"|\"b\" $mode = \"a\";
",
        "    ?\"x\" $maybe = \"x\";
",
        "    ?\"x\" $none = null;
",
        "    1|2 $n = 2;
",
        "    -1 $neg = -1;
",
        "    0x10 $hex = 16;
",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The other side of the same rule: a literal *outside* the named set is an
/// ordinary mismatch, reported once, at the assignment.
#[test]
fn a_literal_outside_the_named_set_is_a_mismatch() {
    for body in [
        "    \"a\"|\"b\" $mode = \"z\";
",
        "    1|2 $n = 3;
",
        "    -1 $neg = 1;
",
    ] {
        let diags = check_in_method(body);
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "{body}: {diags:?}"
        );
    }
}

/// § 4's first three rows, through an assignment rather than through
/// `literal_base` directly: a literal type and a literal union widen to their
/// base for free, and compose with everything already below that rule — a
/// nullable base, and `rule:security/tainted-qualifier`'s `tainted` axis.
#[test]
fn a_literal_type_widens_to_its_base_for_free() {
    let diags = check_in_method(concat!(
        "    \"a\"|\"b\" $mode = \"a\";
",
        "    string $s = $mode;
",
        "    ?string $maybe = $mode;
",
        "    tainted string $t = $mode;
",
        "    mixed $m = $mode;
",
        "    1|2 $n = 1;
",
        "    int $i = $n;
",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// And never the reverse — § 4's last row makes narrowing a checked `as`, so
/// a base-typed value in a literal-typed position is a mismatch, not a
/// silent membership test.
#[test]
fn a_base_type_does_not_narrow_to_a_literal_type_by_assignment() {
    let diags = check_in_method(concat!(
        "    string $s = \"a\";
",
        "    \"a\"|\"b\" $mode = $s;
",
    ));
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// § 3's atom, both halves at once: `Mode::Read` written where the position
/// names it *is* that case's type, it widens to `Mode` for free, and the raw
/// `int` its backing value equals still does not satisfy it — which is the
/// hole `rule:types/conversion`
/// closed and § 3 refuses to reopen.
#[test]
fn an_enum_case_expression_satisfies_a_case_subset_type_and_widens_to_the_enum() {
    let diags = check_src(concat!(
        "<?nvs
",
        "enum Mode { Read, Write, Admin }
",
        "class T {
",
        "  function m(): void {
",
        "    Mode::Read|Mode::Write $m = Mode::Read;
",
        "    Mode $whole = $m;
",
        "    Mode::Read $one = Mode::Read;
",
        "  }
",
        "}
",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    for value in ["Mode::Admin", "0"] {
        let diags = check_src(&format!(
            concat!(
                "<?nvs
",
                "enum Mode {{ Read, Write, Admin }}
",
                "class T {{
",
                "  function m(): void {{
",
                "    Mode::Read|Mode::Write $m = {value};
",
                "  }}
",
                "}}
",
            ),
            value = value,
        ));
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "{value}: {diags:?}"
        );
    }
}

/// § 2's fold is sugar, so the constant and the value it names are
/// interchangeable at the call site — that is the whole argument for folding
/// it, and this is the observable form of it.
#[test]
fn a_folded_class_constant_accepts_the_bare_value_it_names() {
    let diags = check_src(concat!(
        "<?nvs
",
        "class Foo { public const string TYPE_A = \"a\"; }
",
        "class T {
",
        "  function m(): void {
",
        "    Foo::TYPE_A $c = \"a\";
",
        "    string $s = $c;
",
        "  }
",
        "}
",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// § 4's checked `as`, on the rows the operand settles by itself: a literal
/// the target names is *statically* satisfied — `rule:types/numeric-literal-placement`'s placement,
/// extended to § 1's string atom — while a base-typed operand still converts,
/// which is the row that has to keep compiling.
#[test]
fn a_literal_operand_is_placed_at_its_conversion_target() {
    let diags = check_in_method(concat!(
        "    \"a\"|\"b\" $mode = \"a\" as \"a\"|\"b\";\n",
        "    1|2 $n = 1 as 1|2;\n",
        "    string $raw = \"z\";\n",
        "    \"a\"|\"b\" $checked = $raw as \"a\"|\"b\";\n",
        "    string $wide = \"z\" as string;\n",
        // `rule:expressions/nullable-conversion`'s `as ?T` yields `null` rather than throwing, so the target
        // is not a closed set at all and nothing below is impossible.
        "    ?\"a\" $maybe = \"z\" as ?\"a\";\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// § 6's first diagnostic: the operand names a value the closed set does not
/// contain, so the conversion could only ever throw. The accepted set is
/// generated from the target type — asserted by membership rather than as one
/// string, since a union orders its members by type id.
#[test]
fn a_literal_conversion_the_operand_disproves_is_refused() {
    for (body, quoted) in [
        ("    \"a\"|\"b\" $mode = \"z\" as \"a\"|\"b\";\n", "`\"z\"`"),
        ("    1|2 $n = 3 as 1|2;\n", "`3`"),
    ] {
        let diags = check_in_method(body);
        let reported = diags
            .iter()
            .find(|d| d.code == Some(code::E_LITERAL_TYPE_MISMATCH))
            .unwrap_or_else(|| panic!("{body}: {diags:?}"));
        assert!(
            reported
                .message
                .starts_with(&format!("{quoted} is not one of ")),
            "{}",
            reported.message
        );
    }

    let diags = check_in_method("    \"a\"|\"b\" $mode = \"z\" as \"a\"|\"b\";\n");
    let reported = diags
        .iter()
        .find(|d| d.code == Some(code::E_LITERAL_TYPE_MISMATCH))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert!(
        reported.message.contains("`\"a\"`") && reported.message.contains("`\"b\"`"),
        "{}",
        reported.message
    );
}

/// The operand settling the question by itself settles it both ways: an `int`
/// literal carrying a case's value converts into a set holding that case, and
/// one carrying no case's value does not. `1 as Mode::Read|Mode::Write` reaches
/// `Mode::Write` on every execution, so refusing it would refuse a conversion
/// that works, and the membership test is over the cases' values because that
/// is what runs. Assigning `1` to a `Mode` is a different question and stays
/// refused.
// covers: lang:enums/a-union-of-cases-is-a-narrower-type
#[test]
fn an_int_literal_converts_into_the_case_set_that_holds_its_value() {
    let diags = check_src(concat!(
        "<?nvs\n",
        "enum Mode { Read, Write, Admin }\n",
        "enum Mask: uint { None = 0, All = 7 }\n",
        "class T {\n",
        "  function m(): void {\n",
        "    Mode::Read|Mode::Write $named = 1 as Mode::Read|Mode::Write;\n",
        "    Mask::None|Mask::All $wide = 7 as Mask::None|Mask::All;\n",
        "  }\n",
        "}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_src(concat!(
        "<?nvs\n",
        "enum Mode { Read, Write, Admin }\n",
        "class T {\n",
        "  function m(): void {\n",
        "    Mode::Read|Mode::Write $named = 9 as Mode::Read|Mode::Write;\n",
        "  }\n",
        "}\n",
    ));
    let reported = diags
        .iter()
        .find(|d| d.code == Some(code::E_ENUM_CASE_SUBSET_MISMATCH))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert!(
        reported.message.starts_with("`9` is not one of "),
        "{}",
        reported.message
    );

    let diags = check_src(concat!(
        "<?nvs\n",
        "enum Mode { Read, Write, Admin }\n",
        "class T {\n",
        "  function m(): void {\n",
        "    Mode $whole = 1;\n",
        "  }\n",
        "}\n",
    ));
    assert!(diags.has_errors(), "{diags:?}");
}

/// § 6's second diagnostic, and why it is a second one: the set it names is a
/// set of *cases*. The two rows around it stay legal — the case the subset
/// does name, and the whole enum arriving at run time, which is § 4's checked
/// row itself.
#[test]
fn an_enum_case_conversion_the_operand_disproves_is_refused() {
    let diags = check_src(concat!(
        "<?nvs\n",
        "enum Mode { Read, Write, Admin }\n",
        "class T {\n",
        "  function m(Mode $any): void {\n",
        "    Mode::Read|Mode::Write $named = Mode::Read as Mode::Read|Mode::Write;\n",
        "    Mode::Read|Mode::Write $run = $any as Mode::Read|Mode::Write;\n",
        "  }\n",
        "}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_src(concat!(
        "<?nvs\n",
        "enum Mode { Read, Write, Admin }\n",
        "class T {\n",
        "  function m(): void {\n",
        "    Mode::Read|Mode::Write $m = Mode::Admin as Mode::Read|Mode::Write;\n",
        "  }\n",
        "}\n",
    ));
    let reported = diags
        .iter()
        .find(|d| d.code == Some(code::E_ENUM_CASE_SUBSET_MISMATCH))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert!(
        reported.message.starts_with("`Mode::Admin` is not one of ")
            && reported.message.contains("`Mode::Read`")
            && reported.message.contains("`Mode::Write`"),
        "{}",
        reported.message
    );
}

/// A literal type is usable at every binding site `rule:types/declaration` lists, not
/// only at a local — so the same free widening runs at a parameter and at a
/// return.
#[test]
fn a_literal_type_is_assignable_at_a_parameter_and_a_return() {
    let diags = check_src(concat!(
        "<?nvs
",
        "class T {
",
        "  function take(\"a\"|\"b\" $mode): string { return $mode; }
",
        "  function call(): void { string $s = $this->take(\"a\"); }
",
        "}
",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}
