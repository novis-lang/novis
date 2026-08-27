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
    // ADR 0038 § 1: `lateinit` is a property modifier like `readonly` —
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
    // ADR 0043 § 1: `trait` does not exist — this still consumes the
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
    // ADR 0043 § 1: a class-body `use Trait, ...;` — adaptation block,
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
    // ADR 0043 § 4: `by $field` is an optional suffix on one
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
    // ADR 0043 §§ 2-3: an interface method may carry a body — `public`
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
fn namespace_statement_and_block_forms() {
    let s = parse_stmt_ok("namespace App\\Models;");
    let StmtKind::NamespaceDecl(ns) = s.kind else {
        panic!("expected a namespace decl: {s:?}");
    };
    assert!(ns.name.is_some());
    assert!(ns.body.is_none());

    let s = parse_stmt_ok("namespace App { class Foo {} }");
    let StmtKind::NamespaceDecl(ns) = s.kind else {
        panic!("expected a namespace decl: {s:?}");
    };
    assert!(ns.body.is_some());
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

#[test]
fn anonymous_class_as_new_target() {
    let s = parse_stmt_ok("$x = new class (1) implements Comparable { public int $n = 1; };");
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

    // An anonymous `fn` closure statement is unaffected — only the
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
#[test]
fn extremely_deep_nesting_does_not_overflow_the_stack() {
    for opener in ['(', '['] {
        let closer = if opener == '(' { ')' } else { ']' };
        let mut src = String::from("<?nvs $x = ");
        src.extend(std::iter::repeat_n(opener, 10_000));
        src.push('1');
        src.extend(std::iter::repeat_n(closer, 10_000));
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
#[test]
fn a_long_postfix_chain_is_folded_back_to_a_bounded_depth() {
    let mut src = String::from("<?nvs $x");
    for i in 0..10_000 {
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
    let StmtKind::If { then, .. } = &stmts[0].kind else {
        panic!("expected an if: {:?}", stmts[0]);
    };
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
// `autoload` — ADR 0061 § 1, spec `00-overview.md` § 2
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

/// ADR 0061 § 1's literal-only restriction, which is `require`'s
/// (ADR 0021): a path assembled at run time could not contribute to a map
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
                .any(|d| d.code == Some(code::E_AUTOLOAD_PATH_NOT_LITERAL)),
            "expected E_AUTOLOAD_PATH_NOT_LITERAL for {src:?}: {diags:?}"
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
