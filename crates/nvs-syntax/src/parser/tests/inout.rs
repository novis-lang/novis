//! ADR 0107's surface, at the parser: `inout` in the modifier slot of all
//! three binding positions, the same word again at a call site, and `&`
//! refused wherever it used to mean by-reference while keeping the two jobs
//! it still has.
//!
//! Part of [`super`]'s test suite, split to mirror the grammar modules
//! themselves; the helpers every module here calls are in [`super`].

use super::*;

/// Whether `diags` reported [`code::E_BY_REFERENCE_MARKER_RETIRED`] at all.
fn refused_the_marker(diags: &Diagnostics) -> bool {
    diags
        .iter()
        .any(|d| d.code == Some(code::E_BY_REFERENCE_MARKER_RETIRED))
}

/// The one parameter of the single method in a one-class file.
fn only_param(src: &str) -> Param {
    let stmts = parse_file_ok(src);
    let [stmt] = &stmts[..] else {
        panic!("expected one statement: {stmts:?}");
    };
    let StmtKind::ClassDecl(decl) = &stmt.kind else {
        panic!("expected a class: {stmt:?}");
    };
    let [member] = &decl.members[..] else {
        panic!("expected one member: {decl:?}");
    };
    let ClassMemberKind::Method(m) = &member.kind else {
        panic!("expected a method: {member:?}");
    };
    let [param] = &m.params[..] else {
        panic!("expected one parameter: {m:?}");
    };
    param.clone()
}

#[test]
fn an_inout_parameter_parses_in_the_modifier_slot() {
    // ADR 0107 § 1: the word goes before the type, in the slot
    // `parse_modifiers` already runs, so it composes with a promoted
    // property's own modifiers and with a variadic tail.
    let p = only_param(
        "<?mwl class A { public static function bump(inout int $n): int { return $n; } }",
    );
    assert!(p.inout);
    assert!(p.ty.is_some());
    assert!(!p.variadic);

    let p = only_param(
        "<?mwl class A { public static function all(inout array<int> ...$xs): int { return 1; } }",
    );
    assert!(p.inout);
    assert!(p.variadic);

    let p = only_param("<?mwl class A { public function __construct(public inout int $n) {} }");
    assert!(p.inout);
    assert_eq!(p.modifiers.len(), 1);

    // The absence is the default, and nothing else in the slot sets it.
    let p = only_param("<?mwl class A { public static function sum(int $a): int { return $a; } }");
    assert!(!p.inout);
}

#[test]
fn an_inout_foreach_binding_and_destructuring_leaf_parse() {
    // The two positions ADR 0107 § 2 explicitly leaves alone — neither has a
    // call site, so for them this ADR is a rename and nothing else.
    let s = parse_stmt_ok("foreach ($xs as inout int $v) { }");
    let StmtKind::Foreach {
        key, value_inout, ..
    } = s.kind
    else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(
        key.is_none(),
        "`inout` on the first binding rules out a key"
    );
    assert!(value_inout);

    let s = parse_stmt_ok("foreach ($xs as string $k => inout array<int> $row) { }");
    let StmtKind::Foreach {
        key, value_inout, ..
    } = s.kind
    else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(key.is_some());
    assert!(value_inout);

    let s = parse_stmt_ok("[inout int $a, int $b] = $pair;");
    let StmtKind::Destructure { target, .. } = s.kind else {
        panic!("expected a destructuring assignment: {s:?}");
    };
    let marks: Vec<bool> = target
        .elements
        .iter()
        .map(|e| matches!(e, DestructureElement::Leaf { inout: true, .. }))
        .collect();
    assert_eq!(marks, vec![true, false]);
}

#[test]
fn an_inout_argument_parses_at_a_call_site() {
    // ADR 0107 § 2's marker is on the binding, so it sits outside a named
    // argument's `name:` and is independent of the spread marker.
    let e = parse_ok("Adder::bump(inout $n)");
    let CallArgs::List(args) = call_args(&e) else {
        panic!("expected an argument list: {e:?}");
    };
    let [arg] = &args[..] else {
        panic!("expected one argument: {args:?}");
    };
    assert!(arg.inout);
    assert!(arg.name.is_none());
    assert!(!arg.spread);

    let e = parse_ok("Adder::bump($n, inout slot: $m, ...$rest)");
    let CallArgs::List(args) = call_args(&e) else {
        panic!("expected an argument list: {e:?}");
    };
    let marks: Vec<(bool, bool, bool)> = args
        .iter()
        .map(|a| (a.inout, a.name.is_some(), a.spread))
        .collect();
    assert_eq!(
        marks,
        vec![
            (false, false, false),
            (true, true, false),
            (false, false, true)
        ]
    );
}

/// The argument list of a call expression, whatever kind of call it is.
fn call_args(e: &Expr) -> CallArgs {
    match &e.kind {
        ExprKind::Call { args, .. }
        | ExprKind::MethodCall { args, .. }
        | ExprKind::StaticCall { args, .. } => args.clone(),
        other => panic!("expected a call: {other:?}"),
    }
}

#[test]
fn an_ampersand_by_reference_marker_is_refused_naming_inout() {
    // ADR 0107 § 3: every position where `&` meant by-reference is E0237,
    // and the marker is still *recognized* there — that is what lets the
    // site name the fix rather than failing on a malformed intersection.
    for src in [
        "class A { public static function bump(int &$n): int { return $n; } }",
        "class A { public static function &bump(int $n): int { return $n; } }",
        "class A { public int $n { &get => 1; } }",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(refused_the_marker(&diags), "not refused: {src}");
    }
    for src in ["foreach ($xs as int &$v) { }", "[int &$a] = $pair;"] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(refused_the_marker(&diags), "not refused: {src}");
    }

    // The spellings refused because the language has no such thing keep
    // their own codes: this one is `mwl_types`' E0701, and the parser
    // reports nothing at all.
    let (_, diags) = parse_stmt_with_diags("$a = &$b;");
    assert!(!refused_the_marker(&diags));
}

#[test]
fn an_ampersand_still_parses_as_bitwise_and_and_as_an_intersection() {
    // The two meanings ADR 0107 § 3 keeps. With the by-reference one gone an
    // `&` in a type is always an intersection — the lookahead survives only
    // so the refusal above can be worded, never to change what a type means.
    let e = parse_ok("$a & $b");
    assert!(matches!(
        e.kind,
        ExprKind::Binary {
            op: BinaryOp::BitAnd,
            ..
        }
    ));

    let s =
        parse_stmt_ok("class A { public static function f(Comparable&Stringable $x): void {} }");
    let StmtKind::ClassDecl(decl) = s.kind else {
        panic!("expected a class: {s:?}");
    };
    let ClassMemberKind::Method(m) = &decl.members[0].kind else {
        panic!("expected a method: {decl:?}");
    };
    let ty = m.params[0].ty.as_ref().expect("a declared type");
    let TypeKind::Intersection(items) = &ty.kind else {
        panic!("expected an intersection: {ty:?}");
    };
    assert_eq!(items.len(), 2);
    assert!(!m.params[0].inout);
}
