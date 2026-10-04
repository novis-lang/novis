//! The declaration grammar's tests: classes, interfaces, enums, their
//! members, attributes, the file-scope forms, and `parse_file`'s HTML/code
//! round trip.
//!
//! Part of [`super`]'s test suite, split to mirror the grammar modules
//! themselves; the helpers every module here calls are in [`super`].

use super::*;

#[test]
fn class_with_extends_implements_and_members() {
    let s = parse_stmt_ok(
        "class Account extends Base implements Comparable, Countable { \
             public readonly uint $id; \
             public const int MAX = 10; \
             public function constructor(public readonly string $name) {} \
             }",
    );
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    assert!(class.extends.is_some());
    assert_eq!(class.implements.len(), 2);
    assert_eq!(class.members.len(), 3);
    let ClassMemberKind::Property(prop) = &class.members[0].kind else {
        panic!("expected a property: {:?}", class.members[0]);
    };
    assert!(prop.modifiers.contains(&Modifier::Readonly));
    let ClassMemberKind::Const(c) = &class.members[1].kind else {
        panic!("expected a const: {:?}", class.members[1]);
    };
    assert!(c.ty.is_some());
    let ClassMemberKind::Method(m) = &class.members[2].kind else {
        panic!("expected a method: {:?}", class.members[2]);
    };
    assert_eq!(m.params.len(), 1);
    assert!(m.params[0].modifiers.contains(&Modifier::Public));
    assert!(m.params[0].modifiers.contains(&Modifier::Readonly));
    assert!(m.body.is_some());
}

#[test]
fn lateinit_property_modifier_parses() {
    // `rule:classes/lateinit-restrictions`: `lateinit` is a property modifier like `readonly` —
    // which non-nullable/scalar/promoted-parameter combinations it's
    // actually legal on is `nvs-types`' job, not the parser's.
    let s = parse_stmt_ok("class Container { public lateinit Logger $logger; }");
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    let ClassMemberKind::Property(prop) = &class.members[0].kind else {
        panic!("expected a property: {:?}", class.members[0]);
    };
    assert!(prop.modifiers.contains(&Modifier::Lateinit));
    assert!(prop.modifiers.contains(&Modifier::Public));
}

#[test]
fn abstract_and_final_class_modifiers() {
    let s = parse_stmt_ok("abstract class Shape { public abstract function area(): float; }");
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    assert!(class.modifiers.contains(&Modifier::Abstract));
    let ClassMemberKind::Method(m) = &class.members[0].kind else {
        panic!("expected a method: {:?}", class.members[0]);
    };
    assert!(m.modifiers.contains(&Modifier::Abstract));
    assert!(m.body.is_none());

    let s = parse_stmt_ok("final class Sealed {}");
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    assert!(class.modifiers.contains(&Modifier::Final));
}

#[test]
fn interface_with_multiple_extends() {
    let s = parse_stmt_ok(
        "interface Shape extends Comparable, Countable { public function area(): float; }",
    );
    let StmtKind::InterfaceDecl(iface) = s.kind else {
        panic!("expected an interface decl: {s:?}");
    };
    assert_eq!(iface.extends.len(), 2);
    assert_eq!(iface.members.len(), 1);
}

#[test]
fn trait_declaration_is_rejected() {
    // `rule:classes/no-traits`: `trait` does not exist — this still consumes the
    // whole declaration (so the parser doesn't desynchronize) and
    // produces a plain `Error` statement, with no `TraitDecl` AST node.
    let (s, diags) = parse_stmt_with_diags("trait Greets { function hello(): void {} }");
    assert!(matches!(s.kind, StmtKind::Error));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TRAIT_NOT_SUPPORTED))
    );
}

#[test]
fn class_body_use_trait_is_rejected() {
    // `rule:classes/no-traits`: a class-body `use Trait, ...;` — adaptation block,
    // `insteadof`, and all — is rejected the same way, down to a plain
    // `Error` member with no `UseTraitMember` node.
    let (s, diags) = parse_stmt_with_diags(
        "class Greeter { use Greets, Announces { Greets::hello insteadof Announces; } }",
    );
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    assert!(matches!(class.members[0].kind, ClassMemberKind::Error));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TRAIT_NOT_SUPPORTED))
    );
}

#[test]
fn implements_by_field_delegation_parses() {
    // `rule:classes/delegation-by-field`: `by $field` is an optional suffix on one
    // `implements` entry, recorded but not yet resolved (that's
    // `nvs-hir`'s follow-up job).
    let s = parse_stmt_ok(
        "class Post implements Timestamped by $timestamps { \
             private TimestampTracker $timestamps; \
             }",
    );
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    assert_eq!(class.implements.len(), 1);
    assert!(class.implements[0].by_field.is_some());
}

#[test]
fn implements_without_by_field_has_no_delegation() {
    let s = parse_stmt_ok("class Foo implements Comparable {}");
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    assert_eq!(class.implements.len(), 1);
    assert!(class.implements[0].by_field.is_none());
}

#[test]
fn interface_default_and_private_methods_parse() {
    // `rule:classes/interface-default-methods` and `rule:classes/interface-private-methods`: an interface method may carry a body — `public`
    // makes it a default method, `private` an internal-only helper. Both
    // already fall out of the existing shared class-body grammar with no
    // parser change needed; this test locks that in.
    let s = parse_stmt_ok(
        "interface Greets { \
             public function name(): string; \
             public function greet(): string { return $this->name(); } \
             private function helper(): void {} \
             }",
    );
    let StmtKind::InterfaceDecl(iface) = s.kind else {
        panic!("expected an interface decl: {s:?}");
    };
    assert_eq!(iface.members.len(), 3);
    let ClassMemberKind::Method(abstract_method) = &iface.members[0].kind else {
        panic!("expected a method: {:?}", iface.members[0]);
    };
    assert!(abstract_method.body.is_none());
    let ClassMemberKind::Method(default_method) = &iface.members[1].kind else {
        panic!("expected a method: {:?}", iface.members[1]);
    };
    assert!(default_method.body.is_some());
    assert!(default_method.modifiers.contains(&Modifier::Public));
    let ClassMemberKind::Method(private_method) = &iface.members[2].kind else {
        panic!("expected a method: {:?}", iface.members[2]);
    };
    assert!(private_method.body.is_some());
    assert!(private_method.modifiers.contains(&Modifier::Private));
}

#[test]
fn enum_cases_and_explicit_backing_type() {
    let s = parse_stmt_ok("enum Status { Active, Banned, }");
    let StmtKind::EnumDecl(e) = s.kind else {
        panic!("expected an enum decl: {s:?}");
    };
    assert!(e.backing.is_none());
    assert_eq!(e.cases.len(), 2);
    assert!(e.cases[0].value.is_none());

    let s = parse_stmt_ok("enum Permission: uint { Read = 0b001, Write = 0b010, Admin = 0b100 }");
    let StmtKind::EnumDecl(e) = s.kind else {
        panic!("expected an enum decl: {s:?}");
    };
    assert!(matches!(
        e.backing.unwrap().kind,
        TypeKind::Atom(TypeAtom::Uint)
    ));
    assert_eq!(e.cases.len(), 3);
    assert!(e.cases[0].value.is_some());
}

/// An enum case is a `PascalCase` name (`rule:core-api/identifier-casing`), and every keyword is
/// matched at its exact lower-case spelling (`rule:classes/reserved-spellings-are-lower-case`), so a case whose
/// spelling *reads* as a keyword never collides with one. Swept rather than
/// spot-checked, because a single keyword that lexed at any other casing would
/// hand `parse_enum_body` a different token — and the lower-case half below
/// pins what that token then gets.
#[test]
fn an_enum_case_named_with_a_keyword_parses() {
    let names = [
        "Match", "List", "Class", "Default", "Static", "Print", "New", "Echo", "Function", "Use",
        "Case", "For", "Do", "Try", "Enum",
    ];
    for name in names {
        let s = parse_stmt_ok(&format!("enum E {{ {name} }}"));
        let StmtKind::EnumDecl(e) = s.kind else {
            panic!("expected an enum decl for `{name}`: {s:?}");
        };
        assert_eq!(e.cases.len(), 1, "`{name}` did not parse as a case");
        assert!(e.cases[0].value.is_none());
    }

    // All fifteen in one body, so a case-per-statement fluke cannot pass.
    let s = parse_stmt_ok(&format!("enum E {{ {} }}", names.join(", ")));
    let StmtKind::EnumDecl(e) = s.kind else {
        panic!("expected an enum decl: {s:?}");
    };
    assert_eq!(e.cases.len(), names.len());

    // The other side of the bound: the exact keyword spelling lexes as the
    // keyword and is still a case, because only a case could follow it — so
    // the casing check's `must be PascalCase` answers it, not the member
    // refusal `rule:enums/no-class-machinery` owes a method.
    let (s, diags) = parse_stmt_with_diags("enum E { match }");
    let StmtKind::EnumDecl(e) = s.kind else {
        panic!("expected an enum decl: {s:?}");
    };
    assert_eq!(e.cases.len(), 1, "`match` did not parse as a case");
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_ENUM_MEMBER_UNSUPPORTED)),
        "a keyword-spelled case drew the enum-member refusal: {diags:?}"
    );
}

/// A property inside an `interface` body is refused where it is written
/// (`rule:classes/interfaces-declare-no-state`), once per property, and the
/// members beside it still parse: the refusal is a diagnostic on a body the
/// grammar accepted, not a recovery.
// covers: lang:classes/interfaces
#[test]
fn an_interface_property_is_refused_and_the_rest_of_the_body_parses() {
    let (s, diags) = parse_stmt_with_diags(
        "interface View { public string $path; public const string NAME = \"v\"; \
         public function render(): string; }",
    );
    let StmtKind::InterfaceDecl(i) = s.kind else {
        panic!("expected an interface decl: {s:?}");
    };
    assert_eq!(
        i.members.len(),
        3,
        "the body lost a member: {:?}",
        i.members
    );
    let refused = diags
        .iter()
        .filter(|d| d.code == Some(code::E_INTERFACE_PROPERTY_UNSUPPORTED))
        .count();
    assert_eq!(refused, 1, "one property, one refusal: {diags:?}");

    let (_, diags) = parse_stmt_with_diags("interface Named { public function name(): string; }");
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_INTERFACE_PROPERTY_UNSUPPORTED)),
        "a body with no property drew the refusal: {diags:?}"
    );
}

/// PHP's `case Hearts = 1;` body, answered the way `trait` and `(int)$x`
/// already are: one `E02xx` code on the keyword, naming the spelling that
/// works. The count is half the assertion — this shape used to produce a
/// four-diagnostic cascade per case, led by an `E0220` whose "move this to a
/// separate class" help is the answer for a *method* in an enum body and is
/// actively wrong for a case, which belongs in the enum.
#[test]
fn a_php_shaped_enum_case_is_refused_naming_the_spelling_that_works() {
    let (s, diags) = parse_stmt_with_diags("enum Suit: int { case Hearts = 1; case Spades = 2; }");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec![
            code::E_PHP_ENUM_CASE_UNSUPPORTED,
            code::E_PHP_ENUM_CASE_UNSUPPORTED
        ],
        "one code per `case` and nothing else: {diags:?}"
    );

    // The spelling that works is named, not merely the one that does not.
    assert!(
        diags
            .iter()
            .all(|d| d.notes.iter().any(|n| n.contains("`Name = 1,`"))),
        "the help must name the comma-list spelling: {diags:?}"
    );

    // The other half: the cases are *kept*, with their values, so no later
    // phase sees an enum missing the members a program goes on to name — the
    // same discipline `rule:iteration/for-init-refusals` uses for a refused `for` init clause.
    let StmtKind::EnumDecl(e) = s.kind else {
        panic!("expected an enum decl: {s:?}");
    };
    assert_eq!(e.cases.len(), 2);
    assert!(e.cases.iter().all(|c| c.value.is_some()));

    // `rule:classes/reserved-spellings-are-lower-case`: a keyword matches at its exact lower-case spelling, so
    // `Case` is an ordinary `PascalCase` case name and stays one. The bound
    // is asserted on both sides so that widening the keyword arm to an
    // ASCII-caseless match would fail here rather than silently refuse a
    // legal enum.
    let s = parse_stmt_ok("enum E { Case = 1 }");
    let StmtKind::EnumDecl(e) = s.kind else {
        panic!("expected an enum decl: {s:?}");
    };
    assert_eq!(e.cases.len(), 1);
}

#[test]
fn enum_implements_method_and_string_backing_are_rejected() {
    let (_, diags) = parse_stmt_with_diags("enum Status implements Comparable { Active }");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ENUM_IMPLEMENTS_UNSUPPORTED))
    );

    let (_, diags) = parse_stmt_with_diags("enum Status: string { Active }");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ENUM_STRING_BACKING_UNSUPPORTED))
    );

    let (s, diags) =
        parse_stmt_with_diags("enum Status { Active, public function foo(): void {} }");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ENUM_MEMBER_UNSUPPORTED))
    );
    let StmtKind::EnumDecl(e) = s.kind else {
        panic!("expected an enum decl: {s:?}");
    };
    assert_eq!(e.cases.len(), 1);
    assert_eq!(e.members.len(), 1);
}

#[test]
fn property_hooks_get_and_set() {
    let s = parse_stmt_ok(
        "class Temperature { \
             public float $celsius; \
             public float $fahrenheit { \
                 get => $this->celsius * 9 / 5 + 32; \
                 set(float $f) { $this->celsius = ($f - 32) * 5 / 9; } \
             } \
             }",
    );
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    let ClassMemberKind::Property(prop) = &class.members[1].kind else {
        panic!("expected a property: {:?}", class.members[1]);
    };
    let hooks = prop.hooks.as_ref().expect("hooked property");
    assert_eq!(hooks.len(), 2);
    assert_eq!(hooks[0].kind, PropertyHookKind::Get);
    assert!(matches!(hooks[0].body, Some(PropertyHookBody::Expr(_))));
    assert_eq!(hooks[1].kind, PropertyHookKind::Set);
    assert!(hooks[1].param.is_some());
    assert!(matches!(hooks[1].body, Some(PropertyHookBody::Block(_))));
}

#[test]
fn asymmetric_visibility_modifier() {
    let s = parse_stmt_ok("class Point { public private(set) int $x; }");
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    let ClassMemberKind::Property(prop) = &class.members[0].kind else {
        panic!("expected a property: {:?}", class.members[0]);
    };
    assert!(prop.modifiers.contains(&Modifier::Public));
    assert!(
        prop.modifiers
            .contains(&Modifier::SetVisibility(Visibility::Private))
    );
}

#[test]
fn namespace_statement_form() {
    // The braced form parses to the same node and is refused as it does; that
    // half is `a_braced_namespace_is_e0243`.
    let s = parse_stmt_ok("namespace App\\Models;");
    let StmtKind::NamespaceDecl(ns) = s.kind else {
        panic!("expected a namespace decl: {s:?}");
    };
    assert!(ns.name.is_some());
    assert!(ns.body.is_none());
}

#[test]
fn namespace_core_is_reserved() {
    let (_, diags) = parse_stmt_with_diags("namespace Core;");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_RESERVED_CORE_NAMESPACE))
    );

    let (_, diags) = parse_stmt_with_diags("namespace Core\\Sub;");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_RESERVED_CORE_NAMESPACE))
    );
}

#[test]
fn use_import_plain_and_rejected_alias() {
    let s = parse_stmt_ok("use App\\Models\\User;");
    let StmtKind::UseDecl(u) = s.kind else {
        panic!("expected a use decl: {s:?}");
    };
    assert!(u.alias.is_none());

    let (s, diags) = parse_stmt_with_diags("use App\\Models\\User as Model;");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_IMPORT_ALIAS_UNSUPPORTED))
    );
    let StmtKind::UseDecl(u) = s.kind else {
        panic!("expected a use decl: {s:?}");
    };
    assert!(u.alias.is_some());
}

/// `use function Foo\bar;` and `use const Foo\BAZ;` name a kind of thing the
/// language does not have (`rule:classes/no-free-functions-or-constants`), so
/// each gets the refusal by name on the keyword — not the generic parse error
/// that reading `function` as the imported short name used to produce.
///
/// Both sides of the bound: the third case is the spelling that is *not* this
/// refusal, a namespace whose own first segment is the word `function`.
#[test]
fn use_function_and_use_const_get_the_targeted_refusal() {
    for src in ["use function Foo\\bar;", "use const Foo\\BAZ;"] {
        let (stmt, diags) = parse_stmt_with_diags(src);
        let reported: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_IMPORT_OF_FUNCTION_OR_CONST_UNSUPPORTED))
            .collect();
        assert_eq!(reported.len(), 1, "one refusal for `{src}`: {diags:?}");
        assert!(
            reported[0].notes.iter().any(|n| n.contains("Class::name")),
            "the help names what to write instead: {reported:?}"
        );
        // The path behind the keyword still parses, so nothing downstream sees
        // a half-read statement and the `;` is not left to reopen as one.
        let StmtKind::UseDecl(u) = stmt.kind else {
            panic!("`{src}` still yields an import: {stmt:?}");
        };
        assert!(u.alias.is_none());
    }

    let (_, diags) = parse_stmt_with_diags("use function\\Foo;");
    assert!(
        !diags.has_errors(),
        "`function` is an ordinary first name segment: {diags:?}"
    );
}

/// Every reserved word is a legal name segment after the first one —
/// `Parser::is_name_segment` is `Ident | Keyword(_)`, which is what lets
/// `App\Static` and `App\List` be names at all.
///
/// The sweep is over [`Keyword::ALL`] rather than a list written here, so a
/// word added to the table is covered the day it is added; the round trip
/// through `from_lowercase` asserts the table's two halves still describe one
/// word each.
#[test]
fn every_keyword_spelling_is_a_name_segment_past_the_first() {
    for kw in Keyword::ALL {
        let spelling = kw.name();
        assert_eq!(
            Keyword::from_lowercase(spelling),
            Some(*kw),
            "`{spelling}` lexes back to the word it spells"
        );

        let (stmt, diags) = parse_stmt_with_diags(&format!("use App\\{spelling};"));
        assert!(!diags.has_errors(), "`use App\\{spelling};`: {diags:?}");
        let StmtKind::UseDecl(u) = stmt.kind else {
            panic!("`use App\\{spelling};` is an import: {stmt:?}");
        };
        assert_eq!(
            u.path.span.end - u.path.span.start,
            u32::try_from("App\\".len() + spelling.len()).unwrap(),
            "the whole of `App\\{spelling}` is one name"
        );
    }
}

/// PHP's group-use form is refused, not parsed: one `use` names one import.
/// `docs/adr/README.md` § *Decisions taken at project start* owns the rule.
#[test]
fn a_grouped_use_parses_or_names_the_rule_that_refuses_it() {
    let (s, diags) = parse_stmt_with_diags("use App\\Models\\{User, Post};");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_IMPORT_GROUP_UNSUPPORTED)),
        "expected the group-use refusal: {diags:?}"
    );
    // The prefix still yields a `use` of its own, so nothing downstream sees a
    // half-parsed import, and the `;` is consumed rather than left to reopen as
    // a second statement.
    let StmtKind::UseDecl(u) = s.kind else {
        panic!("expected a use decl: {s:?}");
    };
    assert!(u.alias.is_none());

    // A nested group is the same refusal, once — the brace walk is depth-aware.
    let (_, diags) = parse_stmt_with_diags("use App\\{Models\\{User}, Post};");
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_IMPORT_GROUP_UNSUPPORTED))
            .count(),
        1,
        "expected exactly one refusal: {diags:?}"
    );
}

/// One file is one namespace: a `namespace X;` after another one, or after a
/// declaration it would not cover, is `E0243`, and a single one at the top is not.
#[test]
fn a_second_or_late_namespace_statement_is_e0243() {
    for (src, message) in [
        (
            "<?nvs\nnamespace Shop;\nclass Order {}\nnamespace Blog;\n",
            "a file has only one namespace",
        ),
        (
            "<?nvs\nclass Receipt {}\nnamespace Shop;\n",
            "a `namespace` statement comes after a declaration",
        ),
        (
            "<?nvs\ntype Id = int;\nnamespace Shop;\n",
            "a `namespace` statement comes after a declaration",
        ),
    ] {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", src.to_string());
        let mut diags = Diagnostics::new();
        let _ = parse_file(map.file(id), &mut diags);
        let all: Vec<_> = diags.iter().collect();
        assert_eq!(all.len(), 1, "for {src:?}: {all:?}");
        assert_eq!(all[0].code, Some(code::E_BRACED_NAMESPACE_UNSUPPORTED));
        assert_eq!(all[0].message, message, "for {src:?}");
    }
    parse_file_ok("<?nvs\nuse Core\\Str;\nnamespace Shop;\nuse Core\\Arr;\nclass Order {}\n");
}

/// `namespace X;` is the only namespace statement, and the braced form is
/// still parsed after it is refused so the declarations inside it report
/// their own problems in the same run. docs/adr/README.md § *Decisions taken
/// at project start*.
#[test]
fn a_braced_namespace_is_e0243() {
    let (s, diags) = parse_stmt_with_diags("namespace App\\Billing { class Invoice {} }");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec![code::E_BRACED_NAMESPACE_UNSUPPORTED],
        "{diags:?}"
    );
    let StmtKind::NamespaceDecl(n) = s.kind else {
        panic!("expected a namespace decl: {s:?}");
    };
    assert!(n.body.is_some(), "the block is still parsed");

    let (_, diags) = parse_stmt_with_diags("namespace App\\Billing;");
    assert!(!diags.has_errors(), "the statement form: {diags:?}");
}

/// `rule:statements/a-leading-separator-does-not-parse`: the leading separator is refused in all three positions PHP
/// gave it three different meanings in — redundant in a `use` path,
/// load-bearing at a reference, and illegal in a `namespace` declaration. Each
/// reports once and the statement still parses, so one mistake is one
/// diagnostic and nothing downstream sees a half-parsed name.
#[test]
fn a_leading_separator_is_refused_in_every_position_that_takes_a_name() {
    for src in [
        "use \\App\\Models\\User;",
        "namespace \\App;",
        "class T extends \\App\\Base {}",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == Some(code::E_LEADING_BACKSLASH_UNSUPPORTED))
                .count(),
            1,
            "expected exactly one refusal for `{src}`: {diags:?}"
        );
    }

    // The name still parses to the same import it would have without the
    // separator — the report is a refusal of the spelling, not of the name.
    let (s, _) = parse_stmt_with_diags("use \\App\\Models\\User;");
    let StmtKind::UseDecl(u) = s.kind else {
        panic!("expected a use decl: {s:?}");
    };
    assert!(u.alias.is_none());

    // A name with no leading separator is untouched, in the same positions.
    for src in [
        "use App\\Models\\User;",
        "namespace App;",
        "class T extends App\\Base {}",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_LEADING_BACKSLASH_UNSUPPORTED)),
            "`{src}` is the spelling `rule:statements/a-qualified-name-is-absolute` asks for: {diags:?}"
        );
    }
}

#[test]
fn type_alias_declaration() {
    let s = parse_stmt_ok("type UserId = uint;");
    let StmtKind::TypeAliasDecl(t) = s.kind else {
        panic!("expected a type alias decl: {s:?}");
    };
    assert!(matches!(t.ty.kind, TypeKind::Atom(TypeAtom::Uint)));

    // A single bare class atom parses fine — the restriction is M2's.
    parse_stmt_ok("type Id = SomeClass;");
}

/// Parses `<owner> { /// run \n type Meta = {...}; }` and asserts the body's
/// one member is that alias, documented by the run above it — what every body
/// `rule:types/type-alias` names answers identically.
fn assert_type_alias_member(owner: &str) {
    let src = format!(
        "<?nvs\n{owner} {{\n    /// What an order's meta holds.\n    \
         type Meta = {{total: decimal, note?: string}};\n}}\n"
    );
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src.clone());
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    assert!(!diags.has_errors(), "`{owner}`: {diags:?}");
    assert_eq!(stmts.len(), 1, "`{owner}`: {stmts:?}");
    let members = match &stmts[0].kind {
        StmtKind::ClassDecl(class) => &class.members,
        StmtKind::InterfaceDecl(iface) => &iface.members,
        StmtKind::EnumDecl(e) => &e.members,
        other => panic!("expected a body declaration: {other:?}"),
    };
    assert_eq!(members.len(), 1, "`{owner}`: {members:?}");
    assert!(
        members[0].doc.is_some(),
        "the run documents the member: {:?}",
        members[0]
    );
    let ClassMemberKind::TypeAlias(alias) = &members[0].kind else {
        panic!("expected a type alias member: {:?}", members[0]);
    };
    assert_eq!(text(&map, id, alias.name.span), "Meta");
    assert!(
        matches!(alias.ty.kind, TypeKind::Atom(TypeAtom::Shape(_))),
        "{alias:?}"
    );
    assert!(alias.doc.is_none(), "the member carries the run: {alias:?}");
}

/// A class body owns aliases (`rule:types/type-alias`), by the same production
/// the file-scope form uses, and the `///` run above one attaches to the member
/// as it does to any other.
#[test]
fn a_type_alias_is_a_class_member() {
    assert_type_alias_member("class Order");
}

/// The other two bodies with a class-shaped name answer the same way: one rule
/// for every body is what `rule:types/type-alias` states, so nothing about the
/// member differs between the three.
#[test]
fn a_type_alias_is_an_interface_and_an_enum_member() {
    assert_type_alias_member("interface Priced");
    assert_type_alias_member("enum Status");
}

/// An enum's cases still parse as cases beside an alias: `type` is contextual,
/// and only the `Name =` after it tells the declaration from a case whose own
/// name is being spelled.
#[test]
fn an_enum_case_named_beside_an_alias_is_still_a_case() {
    let (s, diags) =
        parse_stmt_with_diags("enum Status { type Pair = array<int>; Active, Banned }");
    assert!(!diags.has_errors(), "{diags:?}");
    let StmtKind::EnumDecl(e) = s.kind else {
        panic!("expected an enum decl: {s:?}");
    };
    assert_eq!(e.cases.len(), 2, "{:?}", e.cases);
    assert_eq!(e.members.len(), 1, "{:?}", e.members);
}

/// A constant declares its type like every other binding, so PHP 8.3's
/// untyped spelling is refused where it is written — and only there: a
/// top-level `const` is already `E0216` whole, and one with no visibility is
/// already `E0122`, so neither collects a second code for one rewrite.
#[test]
fn a_class_constant_without_a_type_is_e0246() {
    let (_, diags) = parse_stmt_with_diags("class C { public const LIMIT = 9; }");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, vec![code::E_CONSTANT_WITHOUT_TYPE], "{diags:?}");

    let (_, diags) = parse_stmt_with_diags("class C { public const int LIMIT = 9; }");
    assert!(!diags.has_errors(), "{diags:?}");

    let (_, diags) = parse_stmt_with_diags("const FOO = 1;");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert!(
        !codes.contains(&code::E_CONSTANT_WITHOUT_TYPE),
        "a top-level `const` is refused whole: {diags:?}"
    );
}

/// An anonymous class is refused at `new class` and then parsed whole, so the
/// members inside it are checked in the same run and nothing downstream meets
/// a half-built declaration. docs/adr/README.md § *Decisions taken at project
/// start* is the refusal; the shape below is what the parser still builds.
#[test]
fn anonymous_class_as_new_target_is_e0244() {
    let (s, diags) =
        parse_stmt_with_diags("$x = new class (1) implements Comparable { public int $n = 1; };");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec![code::E_ANONYMOUS_CLASS_UNSUPPORTED],
        "{diags:?}"
    );
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    let ExprKind::Assign { value, .. } = e.kind else {
        panic!("expected an assignment: {e:?}");
    };
    let ExprKind::New { target, args, .. } = value.kind else {
        panic!("expected a `new`: {value:?}");
    };
    let NewTarget::AnonClass(decl) = target else {
        panic!("expected an anonymous class target: {target:?}");
    };
    assert_eq!(decl.implements.len(), 1);
    assert_eq!(decl.members.len(), 1);
    assert!(matches!(args, CallArgs::List(list) if list.len() == 1));
}

#[test]
fn toplevel_function_and_const_are_rejected() {
    let (s, diags) = parse_stmt_with_diags("function greet(): void {}");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TOPLEVEL_FUNCTION_UNSUPPORTED))
    );
    assert!(matches!(s.kind, StmtKind::TopLevelFunction(_)));

    let (s, diags) = parse_stmt_with_diags("const FOO = 1;");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TOPLEVEL_CONST_UNSUPPORTED))
    );
    let StmtKind::TopLevelConst(consts) = s.kind else {
        panic!("expected a rejected top-level const: {s:?}");
    };
    assert_eq!(consts.len(), 1);

    // An anonymous function statement (`fn`) is unaffected — only the
    // named, top-level `function` declaration above is rejected.
    parse_stmt_ok("fn () => 1;");
}

// ========================================================================
// `parse_file`: the whole-file HTML/code round trip
// ========================================================================

#[test]
fn a_pure_code_file_has_no_inline_html() {
    let stmts = parse_file_ok("<?nvs echo 1;");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0].kind, StmtKind::Echo(_)));
}

#[test]
fn leading_html_before_the_open_tag_is_kept_verbatim() {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", "hello <?nvs echo 1;");
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    assert!(!diags.has_errors());

    let StmtKind::InlineHtml(span) = stmts[0].kind else {
        panic!("expected leading inline HTML: {:?}", stmts[0]);
    };
    assert_eq!(map.file(id).span_text(span), Some("hello "));
    assert!(matches!(stmts[1].kind, StmtKind::Echo(_)));
}

#[test]
fn php_open_tag_is_diagnosed_naming_nvs_tag() {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", "<?php echo 1; ?>".to_string());
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    assert!(diags.has_errors());
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_PHP_OPEN_TAG_UNSUPPORTED))
    );
    // The tag is rejected, but the code after it still parses as code —
    // not misread as inline HTML.
    assert!(matches!(stmts[0].kind, StmtKind::Echo(_)));
}

/// `cargo fuzz run parse` found this exact byte sequence — minimized to
/// a `switch` keyword followed by nothing resembling `case`/`default`/
/// `}` — spinning forever and growing `cases`/`body` without bound
/// instead of terminating, because neither `parse_switch`'s case loop
/// nor a case body's own statement loop had the force-progress guard
/// [`Parser::parse_block`]'s copy has (see the module docs' "Error
/// recovery" section). If this regresses, the test hangs rather than
/// fails cleanly — same as the bug itself did.
#[test]
fn a_malformed_switch_does_not_hang_or_grow_without_bound() {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", "<?=\n\0\0switch]]\0\0w]]]]\n".to_string());
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    assert!(diags.has_errors());
    assert!(
        stmts.len() < 1000,
        "runaway recovery: {} statements",
        stmts.len()
    );
}

/// `cargo fuzz run parse` also found deeply nested parens overflowing
/// the native call stack outright — not a hang, an immediate crash,
/// since unlike the `switch` case above this recursion is entirely
/// well-formed at every level (`parse_type_operand`/`parse_low_or`
/// legitimately calling back into themselves), so no force-progress
/// guard applies. [`Parser::enter_recursive`] bounds it instead. This
/// input is two full orders of magnitude past the limit; if the guard
/// regresses, this crashes the test process rather than failing it
/// cleanly.
///
/// Under Miri the input is one order of magnitude past the limit, not two.
/// What Miri checks here is the parse that runs once the guard has tripped,
/// and a thousand levels reach that as surely as ten thousand. The native
/// run keeps the larger input, because the stack it guards is a native one.
#[test]
fn extremely_deep_nesting_does_not_overflow_the_stack() {
    let depth = if cfg!(miri) { 1_000 } else { 10_000 };
    for opener in ['(', '['] {
        let closer = if opener == '(' { ')' } else { ']' };
        let mut src = String::from("<?nvs $x = ");
        src.extend(std::iter::repeat_n(opener, depth));
        src.push('1');
        src.extend(std::iter::repeat_n(closer, depth));
        src.push(';');
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        assert!(diags.has_errors());
        assert!(!stmts.is_empty());
    }
}

/// A long postfix chain (`$x[0][0][0]...`) builds an equally long
/// `Box`-nested `Expr` through a *loop*, not recursion, so it survives
/// parsing regardless — but the resulting structure used to grow
/// without bound, and a fuzz-run investigation found that overflowing
/// the stack in the very first ordinary recursive walk over it
/// afterwards (originally `nvs ast`'s pretty-printer). Confirms the
/// chain itself gets folded back to a bounded depth: this walks the
/// `Index`/`base` links by hand (not `{:#?}`, to keep the test's own
/// assertion from being exactly the kind of unbounded recursive walk
/// this is guarding against) and checks it stops within a small
/// multiple of the guard's limit.
///
/// Under Miri the chain is a thousand links, the shortest one the assertion
/// below still tells apart from an unfolded chain. Every link past the limit
/// is still parsed, one token at a time, and at ten thousand links Miri
/// spends longer on this one test than on the rest of the crate together.
#[test]
fn a_long_postfix_chain_is_folded_back_to_a_bounded_depth() {
    let links = if cfg!(miri) { 1_000 } else { 10_000 };
    let mut src = String::from("<?nvs $x");
    for i in 0..links {
        src.push_str(&format!("[{i}]"));
    }
    src.push(';');
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    assert!(diags.has_errors());

    let StmtKind::Expr(mut e) = stmts[0].kind.clone() else {
        panic!("expected an expression statement: {:?}", stmts[0]);
    };
    let mut depth = 0u32;
    while let ExprKind::Index { base, .. } = e.kind {
        e = *base;
        depth += 1;
    }
    assert!(depth < 1000, "chain was not folded back: depth {depth}");
}

#[test]
fn a_file_that_never_opens_a_tag_is_all_inline_html() {
    let stmts = parse_file_ok("just some text, no code at all");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0].kind, StmtKind::InlineHtml(_)));
}

#[test]
fn closing_and_reopening_a_tag_mid_block_is_legal() {
    // `if ($x) { ?>html<?nvs }` — PHP allows leaving code mode inside a
    // block; the `}` that closes the `if` is itself back in code mode.
    let stmts = parse_file_ok("<?nvs if ($x) { ?>html<?nvs } ?>tail");
    let StmtKind::If { arms, .. } = &stmts[0].kind else {
        panic!("expected an if: {:?}", stmts[0]);
    };
    let then = &arms[0].then;
    let StmtKind::Block(block) = &then.kind else {
        panic!("expected a block body: {then:?}");
    };
    assert!(
        block
            .stmts
            .iter()
            .any(|s| matches!(s.kind, StmtKind::InlineHtml(_))),
        "expected inline HTML inside the block: {block:?}"
    );
    let StmtKind::InlineHtml(_) = stmts.last().unwrap().kind else {
        panic!("expected trailing inline HTML: {:?}", stmts.last());
    };
}

#[test]
fn short_echo_tag_is_sugar_for_echo() {
    let stmts = parse_file_ok("<?= $name ?>");
    let StmtKind::Echo(exprs) = &stmts[0].kind else {
        panic!("expected an echo: {:?}", stmts[0]);
    };
    assert_eq!(exprs.len(), 1);
}

#[test]
fn short_echo_tag_semicolon_before_close_tag_is_optional_but_allowed() {
    parse_file_ok("<?= $name ?>");
    parse_file_ok("<?= $name; ?>");
}

// ========================================================================
// `autoload` — `rule:programs/autoload`, spec `00-overview.md` § 2
// ========================================================================

/// Both file-scope forms, spelled exactly as the spec's grammar block
/// writes them, plus the one thing adding a second word to the grammar could
/// have broken: `discover` stays an ordinary identifier everywhere else.
#[test]
fn an_autoload_declaration_parses() {
    let mut map = SourceMap::new();
    let id = map.add(
        "t.nvs",
        r"<?nvs
autoload 'Framework' from './';
autoload 'Acme\Legacy' from '../vendor/acme/lib', '../vendor/acme/compat';
autoload discover '../../*/src';
$registry->discover();
",
    );
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    assert!(!diags.has_errors(), "unexpected diagnostics: {diags:?}");
    assert_eq!(stmts.len(), 4);

    let StmtKind::AutoloadDecl(one) = &stmts[0].kind else {
        panic!("expected an autoload decl: {:?}", stmts[0]);
    };
    let AutoloadKind::Prefix { prefix, roots } = &one.kind else {
        panic!("expected the prefix form: {one:?}");
    };
    assert_eq!(text(&map, id, *prefix), "'Framework'");
    assert_eq!(roots.len(), 1);
    assert_eq!(text(&map, id, roots[0]), "'./'");

    let StmtKind::AutoloadDecl(two) = &stmts[1].kind else {
        panic!("expected an autoload decl: {:?}", stmts[1]);
    };
    let AutoloadKind::Prefix { prefix, roots } = &two.kind else {
        panic!("expected the prefix form: {two:?}");
    };
    assert_eq!(text(&map, id, *prefix), r"'Acme\Legacy'");
    assert_eq!(roots.len(), 2);
    assert_eq!(text(&map, id, roots[1]), "'../vendor/acme/compat'");

    let StmtKind::AutoloadDecl(three) = &stmts[2].kind else {
        panic!("expected an autoload decl: {:?}", stmts[2]);
    };
    let AutoloadKind::Discover { glob } = &three.kind else {
        panic!("expected the discover form: {three:?}");
    };
    assert_eq!(text(&map, id, *glob), "'../../*/src'");

    // `discover` is contextual: it means the second form only straight after
    // `autoload`, and is an ordinary member name anywhere else.
    assert!(matches!(stmts[3].kind, StmtKind::Expr(_)));
}

/// `rule:programs/autoload`'s literal-only restriction, which is `require`'s
/// (`rule:statements/require-is-the-only-inclusion-construct`): a path assembled at run time could not contribute to a map
/// built at compile time. Each spelling reports once — a malformed
/// declaration is swallowed through its `;` rather than also failing on the
/// token the parser stopped at.
#[test]
fn an_autoload_path_that_is_not_a_literal_is_a_compile_error() {
    for src in [
        "autoload 'App' from $dir;",
        "autoload 'App' from './' . $sub;",
        "autoload discover \"$root/*/src\";",
        "autoload \"App{$n}\" from './';",
    ] {
        let (stmt, diags) = parse_stmt_with_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_AUTOLOAD_PATH_NOT_WRITTEN_DIRECTLY)),
            "expected E_AUTOLOAD_PATH_NOT_WRITTEN_DIRECTLY for {src:?}: {diags:?}"
        );
        assert_eq!(diags.iter().count(), 1, "for {src:?}: {diags:?}");
        assert!(
            matches!(stmt.kind, StmtKind::Error),
            "expected error recovery for {src:?}: {stmt:?}"
        );
    }
}

/// The prefix form owes a `from` and at least one root; both misses are
/// ordinary "expected" parse errors rather than a silently accepted
/// half-declaration.
#[test]
fn an_autoload_declaration_owes_a_from_and_a_root() {
    for src in ["autoload 'App';", "autoload 'App' from;"] {
        let (stmt, diags) = parse_stmt_with_diags(src);
        assert!(diags.has_errors(), "expected an error for {src:?}");
        assert!(
            matches!(stmt.kind, StmtKind::Error),
            "expected error recovery for {src:?}: {stmt:?}"
        );
    }
}

/// `rule:attributes/attach-sites-and-forms`'s two attach forms carry one payload between them: the named
/// `Name(field: value)` and the bare `{field: value}` differ in whether a
/// name was written and in nothing else, and a name written with no list at
/// all attaches an empty literal rather than a second kind of attribute.
#[test]
fn both_attach_forms_carry_one_anon_object_payload() {
    let s = parse_stmt_ok(
        "#[Route(path: \"/users\", method: \"GET\"), {tag: 1}] \
         #[Audit] \
         class UserController {}",
    );
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    assert_eq!(class.attributes.len(), 2);
    let named = &class.attributes[0].attributes[0];
    assert!(named.name.is_some());
    assert_eq!(named.fields.len(), 2);
    let bare = &class.attributes[0].attributes[1];
    assert!(bare.name.is_none());
    assert_eq!(bare.fields.len(), 1);
    let listless = &class.attributes[1].attributes[0];
    assert!(listless.name.is_some());
    assert!(listless.fields.is_empty());
}

/// An attribute payload is `rule:types/anonymous-object`'s literal without its braces, so it
/// takes that literal's rules rather than an argument list's: a positional
/// value has no field name to be, in either form.
#[test]
fn an_attribute_payload_has_no_positional_field() {
    for src in [
        "#[Route(\"/users\")] class C {}",
        "#[{\"/users\"}] class C {}",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(diags.has_errors(), "expected an error for {src:?}");
    }
}

/// The named form's name may be `Owner::Name`, an alias declared in `Owner`'s
/// body (`rule:attributes/attach-sites-and-forms`): the owner is the name and
/// the alias is the member, and the payload follows as for a plain name.
#[test]
fn an_attribute_name_may_be_an_owner_member() {
    let s = parse_stmt_ok("#[Shop\\Page::Meta(title: \"x\"), Page::Tag] class C {}");
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    let [with_list, listless] = class.attributes[0].attributes.as_slice() else {
        panic!("expected two attributes: {class:?}");
    };
    assert!(with_list.name.is_some() && with_list.member.is_some());
    assert_eq!(with_list.fields.len(), 1);
    assert!(listless.member.is_some());
    assert_eq!(listless.written_name(), Some(listless.payload));
}

/// A malformed attribute group is one error, and the declaration after it
/// parses: the rest of the group is skipped up to its own `]`, or up to the
/// declaration's first keyword when the `]` is missing.
#[test]
fn a_malformed_attribute_group_is_one_error() {
    for src in [
        "#[Page::] class C {}",
        "#[Page::Meta(title: \"x\"] class C {}",
        "#[Page::Meta(title: \"x\") Page::Meta(title: \"y\")] class C {}",
        "#[Page::Meta(title: [1, 2]) extra] class C {}",
        "#[Page::Meta(title: \"x\") class C {}",
    ] {
        let (stmt, diags) = parse_stmt_with_diags(src);
        assert_eq!(diags.iter().count(), 1, "for {src:?}: {diags:?}");
        assert!(
            matches!(stmt.kind, StmtKind::ClassDecl(_)),
            "expected the class to parse for {src:?}: {stmt:?}"
        );
    }
}

/// `rule:php-migration/a-constructor-return-carries-no-value`: the object
/// under construction is the result, so only the *value* is refused — a bare
/// `return;` still leaves early, and a `return` that belongs to a body nested
/// inside the constructor leaves that body instead.
#[test]
fn a_constructor_return_carries_no_value() {
    let refused = "class C { public function constructor() { return 1; } }";
    let (_, diags) = parse_stmt_with_diags(refused);
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, vec![code::E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE]);

    for src in [
        // The early exit the rule keeps.
        "class C { public function constructor() { return; } }",
        // An anonymous function's own body, and an ordinary method's.
        "class C { public function constructor() { $f = fn() => 1; } }",
        "class C { public function value(): int { return 1; } }",
        // A method declared inside the constructor's body does not inherit it.
        "class C { public function constructor() { class D { public function v(): int { return 1; } } } }",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE)),
            "{src}: {diags:?}"
        );
    }
}

/// `rule:php-migration/a-readonly-property-declares-no-default`: a property
/// whose single assignment is its own default is a per-instance constant, and
/// `const` already spells one. The refusal lands on the value, and a property
/// that is not `readonly` keeps its default.
#[test]
fn a_readonly_property_declares_no_default() {
    let (_, diags) = parse_stmt_with_diags("class C { public readonly int $n = 1; }");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, vec![code::E_READONLY_PROPERTY_WITH_DEFAULT]);

    for src in [
        "class C { public int $n = 1; }",
        "class C { public readonly int $n; }",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(!diags.has_errors(), "{src}: {diags:?}");
    }
}
