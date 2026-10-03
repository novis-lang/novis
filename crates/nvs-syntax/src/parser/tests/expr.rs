//! The expression grammar's tests: precedence and associativity, the
//! postfix chain, every primary, and the PHP spellings that parse only to be
//! diagnosed.
//!
//! Part of [`super`]'s test suite, split to mirror the grammar modules
//! themselves; the helpers every module here calls are in [`super`].

use super::*;

#[test]
fn multiplication_binds_tighter_than_addition() {
    let e = parse_ok("1 + 2 * 3");
    let ExprKind::Binary {
        op: BinaryOp::Add,
        lhs,
        rhs,
    } = e.kind
    else {
        panic!("expected a top-level `+`: {e:?}");
    };
    assert!(matches!(lhs.kind, ExprKind::Int(_)));
    assert!(matches!(
        rhs.kind,
        ExprKind::Binary {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn power_is_right_associative() {
    let e = parse_ok("2 ** 3 ** 2");
    let ExprKind::Binary {
        op: BinaryOp::Pow,
        lhs,
        rhs,
    } = e.kind
    else {
        panic!("expected `**`: {e:?}");
    };
    assert!(matches!(lhs.kind, ExprKind::Int(_)));
    assert!(matches!(
        rhs.kind,
        ExprKind::Binary {
            op: BinaryOp::Pow,
            ..
        }
    ));
}

#[test]
fn power_binds_tighter_than_unary_minus() {
    // `-2 ** 2` is `-(2 ** 2)`, PHP's own rule.
    let e = parse_ok("-2 ** 2");
    let ExprKind::Unary {
        op: UnaryOp::Neg,
        expr,
    } = e.kind
    else {
        panic!("expected unary `-`: {e:?}");
    };
    assert!(matches!(
        expr.kind,
        ExprKind::Binary {
            op: BinaryOp::Pow,
            ..
        }
    ));
}

#[test]
fn as_conversion_binds_tighter_than_any_binary_operator() {
    // `rule:types/conversion`: `$a as int + 1` is `($a as int) + 1`.
    let e = parse_ok("$a as int + 1");
    let ExprKind::Binary {
        op: BinaryOp::Add,
        lhs,
        rhs,
    } = e.kind
    else {
        panic!("expected a top-level `+`: {e:?}");
    };
    assert!(matches!(lhs.kind, ExprKind::Conversion { .. }));
    assert!(matches!(rhs.kind, ExprKind::Int(_)));
}

#[test]
fn assignment_is_right_associative() {
    let e = parse_ok("$a = $b = 1");
    let ExprKind::Assign {
        op: AssignOp::Assign,
        target,
        value,
        ..
    } = e.kind
    else {
        panic!("expected `=`: {e:?}");
    };
    assert!(matches!(target.kind, ExprKind::Variable(_)));
    assert!(matches!(
        value.kind,
        ExprKind::Assign {
            op: AssignOp::Assign,
            ..
        }
    ));
}

/// `rule:expressions/defaulting-assignment`: each of the three is an
/// assignment like `+=`, so `$a ??+= $b ??-= 1` nests on the right and `??`
/// on the right side is an ordinary operand.
#[test]
fn the_defaulting_assignment_operators_parse_as_right_associative_assignments() {
    for (src, want) in [
        ("$a ??+= 1", AssignOp::CoalesceAddAssign),
        ("$a ??-= 1", AssignOp::CoalesceSubAssign),
        ("$a ??.= 'x'", AssignOp::CoalesceConcatAssign),
    ] {
        let e = parse_ok(src);
        assert!(
            matches!(e.kind, ExprKind::Assign { op, by_ref: false, .. } if op == want),
            "{src}: {e:?}"
        );
        assert!(want.defaults());
    }
    let e = parse_ok("$a ??+= $b ??-= $c ?? 1");
    let ExprKind::Assign {
        op: AssignOp::CoalesceAddAssign,
        value,
        ..
    } = e.kind
    else {
        panic!("expected `??+=`: {e:?}");
    };
    let ExprKind::Assign {
        op: AssignOp::CoalesceSubAssign,
        value: inner,
        ..
    } = value.kind
    else {
        panic!("expected `??-=` on the right: {value:?}");
    };
    assert!(matches!(
        inner.kind,
        ExprKind::Binary {
            op: BinaryOp::Coalesce,
            ..
        }
    ));
    assert!(!AssignOp::CoalesceAssign.defaults());
    assert!(!AssignOp::AddAssign.defaults());
}

#[test]
fn invalid_assignment_target_is_diagnosed_but_still_parses() {
    let (e, diags) = parse_with_diags("1 = 2");
    assert!(diags.has_errors());
    assert!(matches!(e.kind, ExprKind::Assign { .. }));
}

#[test]
fn reference_assignment_sets_the_by_ref_flag() {
    let e = parse_ok("$a = &$b");
    let ExprKind::Assign {
        op: AssignOp::Assign,
        by_ref,
        ..
    } = e.kind
    else {
        panic!("expected `=`: {e:?}");
    };
    assert!(by_ref);
}

#[test]
fn compound_assignment_has_no_reference_form() {
    // `+=&` is not PHP syntax; a plain `+=` never sets `by_ref` even
    // though the RHS could, on its own, start with a legal expression.
    let e = parse_ok("$a += $b");
    let ExprKind::Assign {
        op: AssignOp::AddAssign,
        by_ref,
        ..
    } = e.kind
    else {
        panic!("expected `+=`: {e:?}");
    };
    assert!(!by_ref);
}

#[test]
fn not_nests_inside_a_rejected_cast_and_other_unary_operators() {
    // `!` sits at a looser precedence tier than a cast or unary op in the
    // grammar ([`Parser::parse_not`]), but a cast/unary op's operand
    // recurses straight into [`Parser::parse_unary`], skipping that
    // tier — so without `parse_unary`'s own `Bang` arm, these would fail
    // to parse at all rather than nesting the way PHP accepts. The
    // legacy-cast spelling is rejected (`rule:types/no-legacy-cast`), but it must still
    // consume `!$x` as its operand rather than leaving it dangling.
    let (e, diags) = parse_with_diags("(int) !$x");
    let ExprKind::Conversion {
        expr, legacy: true, ..
    } = e.kind
    else {
        panic!("expected a legacy conversion, got {:?}", e.kind);
    };
    assert!(
        matches!(
            expr.kind,
            ExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        ),
        "got {:?}",
        expr.kind
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_LEGACY_CAST_UNSUPPORTED)),
        "expected E_LEGACY_CAST_UNSUPPORTED, got {diags:?}"
    );

    let e = parse_ok("-!$x");
    let ExprKind::Unary {
        op: UnaryOp::Neg,
        expr,
    } = e.kind
    else {
        panic!("expected unary `-`: {e:?}");
    };
    assert!(matches!(
        expr.kind,
        ExprKind::Unary {
            op: UnaryOp::Not,
            ..
        }
    ));
}

#[test]
fn ternary_and_elvis() {
    let e = parse_ok("$a ? $b : $c");
    let ExprKind::Ternary { then, .. } = e.kind else {
        panic!("expected ternary: {e:?}");
    };
    assert!(then.is_some());

    let e = parse_ok("$a ?: $c");
    let ExprKind::Ternary { then, .. } = e.kind else {
        panic!("expected elvis ternary: {e:?}");
    };
    assert!(then.is_none());
}

#[test]
fn coalesce_is_right_associative() {
    let e = parse_ok("$a ?? $b ?? $c");
    let ExprKind::Binary {
        op: BinaryOp::Coalesce,
        rhs,
        ..
    } = e.kind
    else {
        panic!("expected `??`: {e:?}");
    };
    assert!(matches!(
        rhs.kind,
        ExprKind::Binary {
            op: BinaryOp::Coalesce,
            ..
        }
    ));
}

#[test]
fn method_call_index_and_property_chain() {
    // $obj->foo()->bar[0]
    let e = parse_ok("$obj->foo()->bar[0]");
    let ExprKind::Index { base, index } = e.kind else {
        panic!("expected an index: {e:?}");
    };
    assert!(index.is_some());
    let ExprKind::PropertyAccess {
        object,
        property: MemberName::Ident(_),
        ..
    } = base.kind
    else {
        panic!("expected a property access: {base:?}");
    };
    assert!(matches!(object.kind, ExprKind::MethodCall { .. }));
}

#[test]
fn nullsafe_method_call() {
    let e = parse_ok("$obj?->foo()");
    assert!(matches!(
        e.kind,
        ExprKind::MethodCall { nullsafe: true, .. }
    ));
}

#[test]
fn static_access_forms() {
    assert!(matches!(
        parse_ok("Foo::BAR").kind,
        ExprKind::ClassConstAccess { .. }
    ));
    assert!(matches!(
        parse_ok("Foo::bar()").kind,
        ExprKind::StaticCall { .. }
    ));
    assert!(matches!(
        parse_ok("Foo::class").kind,
        ExprKind::ClassNameConst { .. }
    ));
    assert!(matches!(
        parse_ok("Foo::$prop").kind,
        ExprKind::StaticPropertyAccess { .. }
    ));
}

#[test]
fn new_with_args_and_dynamic_class() {
    let e = parse_ok("new Foo(1, 2)");
    let ExprKind::New {
        target: NewTarget::Name(_),
        args: CallArgs::List(args),
        ..
    } = e.kind
    else {
        panic!("expected `new Foo(1, 2)`: {e:?}");
    };
    assert_eq!(args.len(), 2);

    // The trailing `()` must belong to `new`, not to `$cls`.
    let e = parse_ok("new $cls()");
    assert!(matches!(
        e.kind,
        ExprKind::New {
            target: NewTarget::Expr(_),
            ..
        }
    ));
}

#[test]
fn new_carries_a_written_type_argument_list() {
    // `new Core\ObjectSet<Tag>()` — the same `<...>` a static call already
    // reads, in the one other position that names a class before a `(`.
    let e = parse_ok(r"new Core\ObjectSet<Tag>()");
    let ExprKind::New {
        target: NewTarget::Name(_),
        type_args,
        args: CallArgs::List(args),
    } = e.kind
    else {
        panic!("expected a `new` with type arguments: {e:?}");
    };
    assert_eq!(type_args.len(), 1);
    assert!(args.is_empty());

    let e = parse_ok("new ObjectMap<Tag, int>($seed)");
    let ExprKind::New {
        type_args, args, ..
    } = e.kind
    else {
        panic!("expected a `new` with two type arguments: {e:?}");
    };
    assert_eq!(type_args.len(), 2);
    assert!(matches!(args, CallArgs::List(list) if list.len() == 1));

    // Nothing written is still the empty list, not a missing one.
    let e = parse_ok("new Foo(1)");
    assert!(matches!(e.kind, ExprKind::New { type_args, .. } if type_args.is_empty()));
}

#[test]
fn a_new_target_followed_by_a_comparison_is_still_a_comparison() {
    // The trial parse commits only when the list parses cleanly *and* a `(`
    // follows, so `new Foo` remains a complete expression under `<`.
    let e = parse_ok("new Foo < $x");
    assert!(matches!(e.kind, ExprKind::Binary { .. }), "{e:?}");

    // A `(` follows here, but `$x` does not parse as a type — so this stays
    // two comparisons rather than becoming a call with type arguments.
    let e = parse_ok("new Foo < $x > ($y)");
    assert!(matches!(e.kind, ExprKind::Binary { .. }), "{e:?}");
}

#[test]
fn object_literal_parses_as_a_primary_expression() {
    // `rule:types/object-literal`.
    let e = parse_ok("{x: 1, y: 2}");
    let ExprKind::ObjectLiteral(fields) = e.kind else {
        panic!("expected an object literal: {e:?}");
    };
    assert_eq!(fields.len(), 2);
    assert!(matches!(fields[0].value.kind, ExprKind::Int(_)));
    assert!(matches!(fields[1].value.kind, ExprKind::Int(_)));

    // A trailing comma is allowed, same as an array literal.
    let e = parse_ok("{count: 0,}");
    let ExprKind::ObjectLiteral(fields) = e.kind else {
        panic!("expected an object literal: {e:?}");
    };
    assert_eq!(fields.len(), 1);
}

#[test]
fn object_literal_rejects_shorthand_and_computed_key() {
    // `rule:types/object-literal`: every field is `name: value` — no shorthand, no
    // computed key.
    let (_, diags) = parse_with_diags("$o = {x};");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_OBJECT_LITERAL_SHORTHAND)),
        "expected E_OBJECT_LITERAL_SHORTHAND, got {diags:?}"
    );

    let (_, diags) = parse_with_diags("$o = {[$k]: 1};");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_OBJECT_LITERAL_COMPUTED_KEY)),
        "expected E_OBJECT_LITERAL_COMPUTED_KEY, got {diags:?}"
    );
}

#[test]
fn object_literal_needs_parens_in_an_arrow_body() {
    // `rule:types/object-literal`: `fn() => {...}` already means a block body per
    // `rule:types/closure-literal` — returning a literal needs `fn() => ({...})` instead.
    let (_, diags) = parse_with_diags("$f = fn() => {x: 1, y: 2};");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_OBJECT_LITERAL_NEEDS_PARENS)),
        "expected E_OBJECT_LITERAL_NEEDS_PARENS, got {diags:?}"
    );

    // Parenthesized, it's an ordinary returned literal with no
    // diagnostic at all.
    let (_, diags) = parse_with_diags("$f = fn() => ({x: 1, y: 2});");
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_OBJECT_LITERAL_NEEDS_PARENS)),
        "did not expect E_OBJECT_LITERAL_NEEDS_PARENS, got {diags:?}"
    );
    let e = parse_ok("fn() => ({x: 1, y: 2})");
    let ExprKind::Fn(f) = e.kind else {
        panic!("expected a fn expr: {e:?}");
    };
    let FnBody::Expr(body) = f.body else {
        panic!("expected an expr body: {:?}", f.body);
    };
    let ExprKind::Paren(inner) = body.kind else {
        panic!("expected a paren: {body:?}");
    };
    assert!(matches!(inner.kind, ExprKind::ObjectLiteral(_)));

    // An ordinary block body is completely unaffected.
    let e = parse_ok("fn() => { return 1; }");
    let ExprKind::Fn(f) = e.kind else {
        panic!("expected a fn expr: {e:?}");
    };
    assert!(matches!(f.body, FnBody::Block(_)));
}

#[test]
fn object_literal_needs_parens_as_a_bare_statement() {
    // `rule:types/object-literal`: a statement-initial `{` already means a block —
    // a discarded literal needs `({...});` instead.
    let (_, diags) = parse_stmt_with_diags("{x: 1, y: 2};");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_OBJECT_LITERAL_NEEDS_PARENS)),
        "expected E_OBJECT_LITERAL_NEEDS_PARENS, got {diags:?}"
    );

    let s = parse_stmt_ok("({x: 1, y: 2});");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expr statement: {s:?}");
    };
    let ExprKind::Paren(inner) = e.kind else {
        panic!("expected a paren: {e:?}");
    };
    assert!(matches!(inner.kind, ExprKind::ObjectLiteral(_)));

    // An ordinary empty block is completely unaffected — the
    // disambiguating lookahead only fires for a non-empty literal
    // attempt.
    let s = parse_stmt_ok("{}");
    assert!(matches!(s.kind, StmtKind::Block(_)));
}

/// `rule:types/no-legacy-cast`: `as` is the only conversion spelling — PHP's legacy
/// `(T)expr` cast syntax is diagnosed, naming `as` as the replacement,
/// the same shape `rule:statements/require-is-the-only-inclusion-construct` already gives `include`/`include_once`/
/// `require_once` in favor of `require`.
#[test]
fn legacy_cast_is_diagnosed() {
    for src in [
        "(int)$x",
        "(uint)$x",
        "(float)$x",
        "(string)$x",
        "(bool)$x",
        "(array)$x",
        "(object)$x",
    ] {
        let (e, diags) = parse_with_diags(src);
        assert!(diags.has_errors(), "expected a diagnostic for {src:?}");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LEGACY_CAST_UNSUPPORTED)),
            "expected E_LEGACY_CAST_UNSUPPORTED for {src:?}, got {diags:?}"
        );
        assert!(
            matches!(e.kind, ExprKind::Conversion { legacy: true, .. }),
            "expected a legacy conversion for {src:?}, got {:?}",
            e.kind
        );
    }
}

/// ADR 0045: `&&`/`||` are the only logical connectives — PHP's
/// low-precedence `and`/`or`/`xor` keyword operators are diagnosed,
/// `and`/`or` naming `&&`/`||` as the replacement and `xor` naming none.
#[test]
fn and_or_xor_keywords_are_diagnosed() {
    for src in ["$a and $b", "$a or $b", "$a xor $b"] {
        let (e, diags) = parse_with_diags(src);
        assert!(diags.has_errors(), "expected a diagnostic for {src:?}");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LOGICAL_KEYWORD_UNSUPPORTED)),
            "expected E_LOGICAL_KEYWORD_UNSUPPORTED for {src:?}, got {diags:?}"
        );
        assert!(matches!(e.kind, ExprKind::Error(_)));
    }
}

/// A chained `$a and $b and $c` reports once per keyword rather than
/// cascading into unrelated "expected token" errors past the first one —
/// each keyword consumes its own right-hand operand before the loop in
/// `Parser::parse_rejected_logical_keyword` checks for another.
#[test]
fn chained_low_keyword_operators_report_once_each() {
    let (e, diags) = parse_with_diags("$a and $b or $c");
    let count = diags
        .iter()
        .filter(|d| d.code == Some(code::E_LOGICAL_KEYWORD_UNSUPPORTED))
        .count();
    assert_eq!(
        count, 2,
        "expected one diagnostic per keyword, got {diags:?}"
    );
    assert!(matches!(e.kind, ExprKind::Error(_)));
}

/// `rule:types/no-legacy-cast`: at statement start specifically, `(string)$x;` is also a
/// syntactically valid (if pointless) local declaration with redundant
/// parens around its type and no initializer — `(string)` parses fine as
/// a one-member parenthesized union. Without routing this shape past
/// `parse_stmt_maybe_local_decl`'s trial parse, it would silently commit
/// to that reading and never report the cast diagnostic at all.
#[test]
fn legacy_cast_is_diagnosed_even_as_a_bare_statement() {
    let (s, diags) = parse_stmt_with_diags("(string)$x;");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_LEGACY_CAST_UNSUPPORTED)),
        "expected E_LEGACY_CAST_UNSUPPORTED, got {diags:?}"
    );
    assert!(!matches!(s.kind, StmtKind::LocalDecl { .. }));
}

#[test]
fn match_expression() {
    let e = parse_ok("match ($x) { 1, 2 => 'a', default => 'b' }");
    let ExprKind::Match { arms, .. } = e.kind else {
        panic!("expected match: {e:?}");
    };
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].conditions.as_ref().map(Vec::len), Some(2));
    assert!(arms[1].conditions.is_none());
}

#[test]
fn function_closure_with_use_by_ref_is_rejected() {
    // `rule:types/closure-literal`/§ 2: `function` closures don't exist at all, and a
    // `use (&$y)` clause gets its own, more specific diagnostic on top.
    let (e, diags) = parse_with_diags("function (int $x) use (&$y): int { return $x + $y; }");
    assert!(matches!(e.kind, ExprKind::Error(_)));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FUNCTION_CLOSURE_UNSUPPORTED))
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CLOSURE_USE_BY_REF_UNSUPPORTED))
    );
}

#[test]
fn function_closure_with_use_by_value_is_rejected() {
    let (e, diags) = parse_with_diags("function () use ($y) { return $y; }");
    assert!(matches!(e.kind, ExprKind::Error(_)));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FUNCTION_CLOSURE_UNSUPPORTED))
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CLOSURE_USE_UNSUPPORTED))
    );
}

#[test]
fn function_closure_without_use_is_rejected_once() {
    let (e, diags) = parse_with_diags("function () { return 1; }");
    assert!(matches!(e.kind, ExprKind::Error(_)));
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_FUNCTION_CLOSURE_UNSUPPORTED))
            .count(),
        1
    );
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_CLOSURE_USE_UNSUPPORTED)
                || d.code == Some(code::E_CLOSURE_USE_BY_REF_UNSUPPORTED))
    );
}

#[test]
fn fn_expr_captures_by_expression() {
    let e = parse_ok("fn (int $x): int => $x + $y");
    let ExprKind::Fn(f) = e.kind else {
        panic!("expected an fn expression: {e:?}");
    };
    assert_eq!(f.params.len(), 1);
    assert!(f.name.is_none());
    let FnBody::Expr(body) = f.body else {
        panic!("expected an expression body: {f:?}");
    };
    assert!(matches!(
        body.kind,
        ExprKind::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
}

#[test]
fn fn_expr_with_block_body() {
    let e = parse_ok("fn (int $x): int => { $y = $x + 1; return $y * 2; }");
    let ExprKind::Fn(f) = e.kind else {
        panic!("expected an fn expression: {e:?}");
    };
    let FnBody::Block(block) = f.body else {
        panic!("expected a block body: {f:?}");
    };
    assert_eq!(block.stmts.len(), 2);
}

#[test]
fn fn_expr_self_name_for_recursion() {
    let e = parse_ok("fn factorial(int $n) => $n <= 1 ? 1 : $n * factorial($n - 1)");
    let ExprKind::Fn(f) = e.kind else {
        panic!("expected an fn expression: {e:?}");
    };
    assert!(f.name.is_some());
}

#[test]
fn a_closure_parameter_may_omit_its_type() {
    // `rule:types/callable-literal-inference`: the omission parses, and the
    // type comes from the position the literal is written in. A parameter
    // beside it may still name one.
    let (e, diags) = parse_with_diags("fn ($x, int $y) => $x");
    assert!(!diags.has_errors(), "unexpected diagnostics: {diags:?}");
    let ExprKind::Fn(f) = e.kind else {
        panic!("expected an fn expression: {e:?}");
    };
    assert!(f.params[0].ty.is_none());
    assert!(f.params[1].ty.is_some());
}

#[test]
fn a_declared_parameter_still_names_its_type() {
    // `rule:types/declaration`: only a closure literal stands in a position
    // that has a type to offer, so a method's list keeps the refusal.
    let (_, diags) = parse_stmt_with_diags("class A { public function f($x): int { return 1; } }");
    assert!(diags.has_errors());
}

#[test]
fn first_class_callable_syntax() {
    let e = parse_ok("strlen(...)");
    assert!(matches!(
        e.kind,
        ExprKind::Call {
            args: CallArgs::FirstClassCallable,
            ..
        }
    ));
}

#[test]
fn named_and_spread_arguments() {
    let e = parse_ok("foo(x: 1, ...$rest)");
    let ExprKind::Call {
        args: CallArgs::List(args),
        ..
    } = e.kind
    else {
        panic!("expected a call: {e:?}");
    };
    assert_eq!(args.len(), 2);
    assert!(args[0].name.is_some());
    assert!(args[1].spread);
}

#[test]
fn a_keyword_spelled_parameter_name_is_a_named_argument() {
    // `rule:core-api/shape-rules` R2 makes every parameter callable by the `$name` the spec
    // writes, and `Core\Arr::map`'s is `$fn` — a spelling the lexer reserves.
    // The `:` is the whole disambiguation.
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", "<?nvs Core\\Arr::map(fn: $f, array: $xs)");
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump();
    let e = p.parse_expr();
    assert!(!diags.has_errors(), "unexpected diagnostics: {diags:?}");
    let ExprKind::StaticCall {
        args: CallArgs::List(args),
        ..
    } = e.kind
    else {
        panic!("expected a static call: {e:?}");
    };
    assert_eq!(args.len(), 2);
    assert_eq!(text(&map, id, args[0].name.expect("a name")), "fn");
    assert_eq!(text(&map, id, args[1].name.expect("a name")), "array");
}

#[test]
fn a_closure_argument_is_still_a_closure() {
    // The other side of the rule above: a keyword is only a name when a `:`
    // follows it immediately, so an `fn` literal — which always has its
    // parameter list next — is untouched.
    let e = parse_ok("usort($xs, fn (int $a, int $b): int => $a <=> $b)");
    let ExprKind::Call {
        args: CallArgs::List(args),
        ..
    } = e.kind
    else {
        panic!("expected a call: {e:?}");
    };
    assert!(args[1].name.is_none());
    assert!(matches!(args[1].value.kind, ExprKind::Fn(_)));
}

#[test]
fn array_literal_with_key_spread_and_by_ref() {
    let e = parse_ok("['a' => 1, &$x, ...$rest]");
    let ExprKind::ArrayLiteral(items) = e.kind else {
        panic!("expected an array literal: {e:?}");
    };
    assert_eq!(items.len(), 3);
    assert!(items[0].key.is_some());
    assert!(items[1].by_ref);
    assert!(items[2].spread);
}

#[test]
fn double_quoted_string_without_interpolation_collapses_to_a_plain_literal() {
    assert!(matches!(parse_ok(r#""plain text""#).kind, ExprKind::Str(_)));
}

#[test]
fn double_quoted_string_with_interpolation() {
    let e = parse_ok(r#""a $name->prop b""#);
    let ExprKind::Interpolated(parts) = e.kind else {
        panic!("expected an interpolated string: {e:?}");
    };
    assert_eq!(parts.len(), 3);
    assert!(matches!(parts[0], StringPart::Text(_)));
    assert!(matches!(parts[1], StringPart::Expr(_)));
    assert!(matches!(parts[2], StringPart::Text(_)));
    let StringPart::Expr(inner) = &parts[1] else {
        unreachable!()
    };
    assert!(matches!(inner.kind, ExprKind::PropertyAccess { .. }));
}

#[test]
fn a_markup_literal_without_holes_stays_a_markup_node() {
    // Unlike a quoted string, a hole-free body does not collapse to
    // `ExprKind::Str`: the node, not the part count, is what says
    // `Core\Html\Markup` (`rule:core-classes/html-literal`).
    let e = parse_ok("html`<hr>`");
    let ExprKind::Markup(parts) = e.kind else {
        panic!("expected a markup literal: {e:?}");
    };
    assert_eq!(parts.len(), 1);
    assert!(matches!(parts[0], StringPart::Text(_)));
}

#[test]
fn a_markup_hole_is_a_strings_hole() {
    let e = parse_ok("html`<span>{$u->fullName()}</span>`");
    let ExprKind::Markup(parts) = e.kind else {
        panic!("expected a markup literal: {e:?}");
    };
    assert_eq!(parts.len(), 3);
    assert!(matches!(parts[0], StringPart::Text(_)));
    assert!(matches!(parts[2], StringPart::Text(_)));
    let StringPart::Expr(inner) = &parts[1] else {
        panic!("expected a hole in the middle: {parts:?}");
    };
    assert!(matches!(inner.kind, ExprKind::MethodCall { .. }));
}

#[test]
fn a_tag_hole_is_the_same_part_as_a_brace_hole_and_takes_a_static_call() {
    // What differs between the two holes is decided in the lexer; by here a
    // `<?= … ?>` is one more `StringPart::Expr`, so nothing downstream learns a
    // second kind of hole (`rule:core-classes/html-literal`).
    let e = parse_ok("html`<td><?= Money::format($c) ?></td>`");
    let ExprKind::Markup(parts) = e.kind else {
        panic!("expected a markup literal: {e:?}");
    };
    assert_eq!(parts.len(), 3);
    let StringPart::Expr(inner) = &parts[1] else {
        panic!("expected a hole in the middle: {parts:?}");
    };
    assert!(matches!(inner.kind, ExprKind::StaticCall { .. }));
}

#[test]
fn spawn_script_with_options() {
    let e = parse_ok("spawn script 'jobs/report.nvs' with(args: $a, grants: $g)");
    let ExprKind::SpawnScript { options, .. } = e.kind else {
        panic!("expected spawn script: {e:?}");
    };
    assert_eq!(options.len(), 2);
    assert_eq!(options[0].key, SpawnOptionKey::Args);
    assert_eq!(options[1].key, SpawnOptionKey::Grants);
}

#[test]
fn spawn_script_without_with_clause() {
    let e = parse_ok("spawn script 'jobs/report.nvs'");
    let ExprKind::SpawnScript { options, .. } = e.kind else {
        panic!("expected spawn script: {e:?}");
    };
    assert!(options.is_empty());
}

#[test]
fn spawn_and_script_are_still_plain_identifiers_elsewhere() {
    // `spawn` alone (no following `script`) must stay an ordinary
    // constant-fetch name, not misfire the `spawn script` production.
    assert!(matches!(parse_ok("spawn").kind, ExprKind::ConstFetch(_)));
}

#[test]
fn await_is_a_prefix_expression_over_the_handle() {
    // `docs/spec/00-overview.md` § 2: the awaitable a `spawn script` produces
    // becomes a `ScriptResult` at a subsequent `await`. The operand is a unary
    // expression, so the postfix chain binds tighter than the keyword does.
    let e = parse_ok("await $handle");
    let ExprKind::Await(inner) = e.kind else {
        panic!("expected await: {e:?}");
    };
    assert!(matches!(inner.kind, ExprKind::Variable(_)));

    let e = parse_ok("await $h->result");
    let ExprKind::Await(inner) = e.kind else {
        panic!("expected await: {e:?}");
    };
    assert!(matches!(inner.kind, ExprKind::PropertyAccess { .. }));

    // The inline spelling § 2 calls out: a spawn awaited where it is written.
    let e = parse_ok("await spawn script 'jobs/report.nvs'");
    let ExprKind::Await(inner) = e.kind else {
        panic!("expected await: {e:?}");
    };
    assert!(matches!(inner.kind, ExprKind::SpawnScript { .. }));
}

#[test]
fn await_is_still_a_plain_identifier_with_no_operand() {
    // Contextual, like `spawn`: with nothing to await after it, the spelling
    // is an ordinary constant-fetch name and not a half-parsed construct.
    assert!(matches!(parse_ok("await").kind, ExprKind::ConstFetch(_)));
}

#[test]
fn alt_colon_syntax_end_words_are_plain_identifiers() {
    // PHP's alternative colon syntax (`if (...): ... endif;`) is deliberately
    // out of scope; `endif`/`endfor`/`endforeach`/`endswitch`/`endwhile`/
    // `enddeclare` must not be reserved words.
    for name in [
        "endif",
        "endfor",
        "endforeach",
        "endswitch",
        "endwhile",
        "enddeclare",
    ] {
        assert!(matches!(parse_ok(name).kind, ExprKind::ConstFetch(_)));
    }
}

#[test]
fn yield_forms() {
    assert!(matches!(parse_ok("fn () => yield").kind, ExprKind::Fn(_)));

    let e = parse_ok("yield $x");
    assert!(matches!(
        e.kind,
        ExprKind::Yield {
            key: None,
            value: Some(_)
        }
    ));

    let e = parse_ok("yield $k => $v");
    assert!(matches!(
        e.kind,
        ExprKind::Yield {
            key: Some(_),
            value: Some(_)
        }
    ));

    let e = parse_ok("yield from $gen");
    assert!(matches!(e.kind, ExprKind::YieldFrom(_)));
}

#[test]
fn isset_and_empty() {
    let e = parse_ok("isset($a, $b)");
    let ExprKind::Isset(args) = e.kind else {
        panic!("expected isset: {e:?}");
    };
    assert_eq!(args.len(), 2);

    assert!(matches!(parse_ok("empty($a)").kind, ExprKind::Empty(_)));
}

#[test]
fn name_span_covers_the_qualified_name() {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", "<?nvs Core\\Bytes::fromHex('ab')");
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump();
    let e = p.parse_expr();
    assert!(!diags.has_errors());
    let ExprKind::StaticCall { class, .. } = e.kind else {
        panic!("expected a static call: {e:?}");
    };
    let ExprKind::ConstFetch(name) = class.kind else {
        panic!("expected a name: {class:?}");
    };
    assert_eq!(text(&map, id, name.span), "Core\\Bytes");
}

#[test]
fn static_function_closure_is_diagnosed_as_a_function_closure() {
    // `static function () {}` hits the same "not supported, use `fn`"
    // diagnostic as the unqualified spelling — there is no separate
    // static-modifier complaint once the literal itself is rejected.
    let (e, diags) = parse_with_diags("static function () { return 1; }");
    assert!(matches!(e.kind, ExprKind::Error(_)));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FUNCTION_CLOSURE_UNSUPPORTED))
    );
}

#[test]
fn static_fn_modifier_is_diagnosed_but_still_parses() {
    let (e, diags) = parse_with_diags("static fn ($x) => $x");
    assert!(diags.has_errors());
    let ExprKind::Fn(f) = e.kind else {
        panic!("expected an fn expression: {e:?}");
    };
    assert!(f.is_static);
}

#[test]
fn ordinary_fn_expr_is_not_diagnosed() {
    parse_ok("fn (int $x) => $x");
    parse_ok("fn (int $x) => { return $x; }");
}

#[test]
fn superglobals_are_diagnosed_but_still_parse_as_variables() {
    for name in [
        "$GLOBALS",
        "$_REQUEST",
        "$_GET",
        "$_POST",
        "$_COOKIE",
        "$_FILES",
        "$_SERVER",
        "$_SESSION",
        "$_ENV",
        "$argv",
        "$argc",
        "$_ARGS",
    ] {
        let (e, diags) = parse_with_diags(name);
        assert!(diags.has_errors(), "expected a diagnostic for {name}");
        assert!(matches!(e.kind, ExprKind::Variable(_)), "for {name}");
    }
}

#[test]
fn an_ordinary_variable_is_not_mistaken_for_a_superglobal() {
    parse_ok("$_getter");
    parse_ok("$server");
    parse_ok("$globals");
}

#[test]
fn dollar_dollar_name_is_variable_variable() {
    let (e, diags) = parse_with_diags("$$name");
    assert!(diags.has_errors());
    assert!(matches!(e.kind, ExprKind::Error(_)));
}

#[test]
fn dollar_brace_expr_is_variable_variable() {
    let (e, diags) = parse_with_diags("${$name}");
    assert!(diags.has_errors());
    assert!(matches!(e.kind, ExprKind::Error(_)));
}

// --- `rule:expressions/catch-expression`'s expression `catch` ------------------------------------------
// § 2's table, one test a row. The lowering is not written, so these are the
// only place the grouping is pinned until stage 9's item 23 lands.

#[test]
fn catch_is_tighter_than_assignment_and_looser_than_the_ternary_level() {
    // `$x = (($a / $b) catch (ArithmeticError) => 0)` — the guard covers the
    // whole division, and the assignment covers the whole guard.
    let e = parse_ok("$x = 6 / 2 catch (ArithmeticError) => 0");
    let ExprKind::Assign { value, .. } = e.kind else {
        panic!("expected a top-level `=`: {e:?}");
    };
    let ExprKind::Catch { guarded, arms } = value.kind else {
        panic!("expected the assigned value to be a `catch`: {value:?}");
    };
    assert!(matches!(
        guarded.kind,
        ExprKind::Binary {
            op: BinaryOp::Div,
            ..
        }
    ));
    assert_eq!(arms.len(), 1);
    assert!(arms[0].var.is_none());
}

#[test]
fn a_guard_covers_a_whole_coalesce_chain() {
    // `(1 ?? 2) catch (IOError) => 3` — `??` is below the ternary level, so
    // the guard takes all of it rather than only its right-hand side.
    let e = parse_ok("1 ?? 2 catch (IOError) => 3");
    let ExprKind::Catch { guarded, .. } = e.kind else {
        panic!("expected a top-level `catch`: {e:?}");
    };
    assert!(matches!(
        guarded.kind,
        ExprKind::Binary {
            op: BinaryOp::Coalesce,
            ..
        }
    ));
}

#[test]
fn an_expression_catch_is_arms_on_one_guarded_expression() {
    // § 1: arms are clauses of one guard, not guards of each other. The `?:`
    // is the first arm's whole body and the `B` arm does not guard it.
    let e = parse_ok("1 catch (A $e) => $y ?: 2 catch (B) => 3");
    let ExprKind::Catch { guarded, arms } = e.kind else {
        panic!("expected a top-level `catch`: {e:?}");
    };
    assert!(matches!(guarded.kind, ExprKind::Int(_)));
    assert_eq!(arms.len(), 2);
    assert!(arms[0].var.is_some());
    assert!(matches!(arms[0].body.kind, ExprKind::Ternary { .. }));
    assert!(arms[1].var.is_none());
    assert!(matches!(arms[1].body.kind, ExprKind::Int(_)));
}

/// § 3's refusal, and the reason it is a code of its own: a reader who writes
/// `return` here is told the block form exists rather than which token was
/// expected. The keyword is consumed and what follows it becomes the arm's
/// body, so the file reports the rest of its problems in the same run.
#[test]
fn a_statement_keyword_in_a_catch_arm_is_e0126() {
    for (src, body_is_int) in [
        ("1 catch (IOError $e) => return 2", true),
        ("1 catch (IOError $e) => break", false),
        ("1 catch (IOError $e) => continue", false),
    ] {
        let (e, diags) = parse_with_diags(src);
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        assert_eq!(codes, vec![code::E_CATCH_ARM_NOT_AN_EXPRESSION], "{src}");
        let ExprKind::Catch { arms, .. } = e.kind else {
            panic!("expected a top-level `catch`: {e:?}");
        };
        assert_eq!(arms.len(), 1, "{src}");
        assert_eq!(
            matches!(arms[0].body.kind, ExprKind::Int(_)),
            body_is_int,
            "the keyword is consumed and what follows it is the body: {src}"
        );
    }
}

#[test]
fn a_throw_arm_is_an_expression_and_parses() {
    // § 3: `throw expr` is already an expression, so the arm admits it with no
    // rule of its own — where `return` is E0126.
    let e = parse_ok(r#"1 catch (IOError $e) => throw new IOError("no")"#);
    let ExprKind::Catch { arms, .. } = e.kind else {
        panic!("expected a top-level `catch`: {e:?}");
    };
    assert_eq!(arms.len(), 1);
    assert!(matches!(arms[0].body.kind, ExprKind::Throw(_)));
}

// --- recovery a consumer can read (`rule:ide/recovery-is-explicit`) --------------------------------
// Two shapes an editor meets on nearly every keystroke — a caret after `->`,
// and a construct the language refuses — and one claim about both: the
// *variant* says the parser invented the node, so nothing has to measure a
// span to find that out.

/// Parses `src` as an expression and keeps the source map, so a span can be
/// read back as the text it covers — which is the only way to check what a
/// recovery node stood in for. [`parse_with_diags`] drops the map.
fn parse_keeping_source(src: &str) -> (Expr, Diagnostics, SourceMap, nvs_diagnostics::SourceId) {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", format!("<?nvs {src}"));
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump(); // OpenTagNvs
    let e = p.parse_expr();
    (e, diags, map, id)
}

#[test]
fn a_missing_member_name_is_explicit_not_an_empty_span() {
    // `$u->` with the caret after the arrow is still a property access, and
    // the name it carries is `Missing` rather than an `Ident` spanning
    // nothing. The span is asserted to be exactly that caret — what used to
    // be the only tell, and is now not the signal at all.
    let (e, diags, map, id) = parse_keeping_source("$u->");
    assert!(diags.has_errors(), "the missing name is still diagnosed");
    let ExprKind::PropertyAccess {
        object, property, ..
    } = e.kind
    else {
        panic!("expected a property access: {e:?}");
    };
    assert!(matches!(object.kind, ExprKind::Variable(_)));
    let MemberName::Missing(span) = property else {
        panic!("expected a missing member name: {property:?}");
    };
    assert_eq!(text(&map, id, span), "");
    assert_eq!(span.start, span.end, "an insertion point is a caret");
}

#[test]
fn a_member_name_the_user_wrote_is_never_missing() {
    // The other half of the distinction, and the one completion rests on:
    // every spelling a program can write stays what it was, so `Missing`
    // means "invented at the cursor" and nothing else does.
    for src in [
        "$u->name",
        "$u->greet()",
        "$u->$key",
        "$u->{$key}",
        "User::make()",
    ] {
        let e = parse_ok(src);
        let name = match &e.kind {
            ExprKind::PropertyAccess { property, .. } => property,
            ExprKind::MethodCall { method, .. } | ExprKind::StaticCall { method, .. } => method,
            other => panic!("expected a member access for {src:?}: {other:?}"),
        };
        assert!(
            !matches!(name, MemberName::Missing(_)),
            "{src:?} was written, but parsed as a name nobody wrote"
        );
    }
}

#[test]
fn an_error_expression_carries_the_span_it_stood_in_for() {
    // A refused construct: the placeholder names the source it replaced, so a
    // consumer holding only the kind can point at it without walking back out
    // to the node that carries it.
    let (e, diags, map, id) = parse_keeping_source("$a and $b");
    assert!(diags.has_errors());
    let ExprKind::Error(span) = e.kind else {
        panic!("expected a recovery node: {e:?}");
    };
    assert_eq!(text(&map, id, span), "$a and $b");

    // An expression that was required and never written: the span is the
    // caret it would have started at. It is empty, which is exactly why the
    // variant rather than the width is what says recovery happened.
    let (e, diags, map, id) = parse_keeping_source("1 +");
    assert!(diags.has_errors());
    let ExprKind::Binary { rhs, .. } = e.kind else {
        panic!("expected a binary expression: {e:?}");
    };
    let ExprKind::Error(span) = &rhs.kind else {
        panic!("expected a recovery node: {rhs:?}");
    };
    assert_eq!(text(&map, id, *span), "");
    assert_eq!(*span, rhs.span);
}

// --- the pipeline operator (`rule:expressions/pipeline-substitution`) ------------------------------
// The design's whole claim is that `|>` leaves no trace: what the parser emits
// is the tree the nested spelling emits, so the tests below compare the two
// rather than inspecting a node of their own — there is no node of their own to
// inspect.

/// The expression's `Debug` shape with every span rewritten: to its own text
/// when `spans_as_text`, and to `_` otherwise.
///
/// Two spellings of one tree cannot agree on positions — the substituted left
/// side keeps the span it had on the *left* of the `|>` — so the structural
/// comparison is made modulo them, and the text form is what a spot check of a
/// leaf uses to say *which* value landed in the hole.
fn rendered(src: &str, spans_as_text: bool) -> String {
    let full = format!("<?nvs {src}");
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", full.clone());
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump(); // OpenTagNvs
    let e = p.parse_expr();
    assert!(
        !diags.has_errors(),
        "unexpected diagnostics for {src:?}: {diags:?}"
    );

    // A span renders as `file:start..end`, and every test here parses one
    // file, so `0:` is where one begins and nothing else can spell it.
    let debug = format!("{e:?}");
    let mut out = String::new();
    let mut rest = debug.as_str();
    while let Some(cut) = rest.find("0:") {
        out.push_str(&rest[..cut]);
        let (start, tail) = leading_number(&rest[cut + 2..]);
        let tail = tail
            .strip_prefix("..")
            .expect("a span renders as `file:start..end`");
        let (end, tail) = leading_number(tail);
        if spans_as_text {
            out.push('`');
            out.push_str(&full[start..end]);
            out.push('`');
        } else {
            out.push('_');
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

fn leading_number(s: &str) -> (usize, &str) {
    let cut = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    (
        s[..cut].parse().expect("a span offset is a number"),
        &s[cut..],
    )
}

/// `rule:expressions/pipeline-substitution`, and
/// `rule:expressions/pipeline-precedence`'s table in the same breath: each row
/// is a piped spelling and the nesting it must become, and the last four are
/// exactly the four groupings a looser placement would get wrong.
#[test]
fn a_pipeline_produces_the_same_ast_as_the_nested_spelling() {
    for (piped, nested) in [
        // The four shapes a right side may take.
        ("$a |> Str::trim($_)", "Str::trim($a)"),
        ("$a |> $_[0]", "$a[0]"),
        ("$a |> $_->name", "$a->name"),
        ("$n |> ($_ * 2)", "($n * 2)"),
        // The precedence table: tighter than every binary, looser than unary.
        ("-$a |> Math::abs($_)", "Math::abs(-$a)"),
        (r#""x=" . $a |> Str::upper($_)"#, r#""x=" . Str::upper($a)"#),
        ("$a |> Str::length($_) > 5", "Str::length($a) > 5"),
        ("$x = $a |> Str::trim($_)", "$x = Str::trim($a)"),
    ] {
        assert_eq!(
            rendered(piped, false),
            rendered(nested, false),
            "{piped} must parse as {nested}"
        );
    }

    // Modulo positions is not modulo *values*: the left side, and not some
    // other variable, is what the hole became.
    assert!(
        rendered("$a |> Str::trim($_)", true).contains("Variable(`$a`)"),
        "the left side is what landed in the hole: {}",
        rendered("$a |> Str::trim($_)", true)
    );
}

#[test]
fn a_pipeline_chain_associates_left_to_right() {
    assert_eq!(
        rendered("$a |> Str::trim($_) |> Str::lower($_)", false),
        rendered("Str::lower(Str::trim($a))", false),
    );
}

/// The same bound at every left-associative tier, on both sides. Each spelling
/// is one flat chain of one operator, so a tier that charged nothing for its
/// loop would accept an arbitrarily deep left-nested tree — the general form of
/// what a `|>` chain could do. `is` is here because it is the one
/// left-associative tier that does not go through `parse_left_assoc`.
#[test]
fn a_left_associative_chain_is_charged_to_the_recursion_guard() {
    let over = MAX_RECURSION_DEPTH as usize + 1;
    for link in [" + 1", " . $b", " || $b", " is Foo"] {
        parse_ok(&format!("$a{}", link.repeat(64)));

        let (_, diags) = parse_with_diags(&format!("$a{}", link.repeat(over)));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TOO_DEEPLY_NESTED)),
            "a chain of `{link}` past the recursion limit must be refused: {diags:?}"
        );
    }
}

/// The bound on both sides. A `|>` chain is a loop, so nothing about parsing
/// one grows the parser's own stack — but each stage wraps the previous tree in
/// one more node, and `rule:expressions/pipeline-substitution` makes that tree
/// the nested spelling's, which is refused once it is deep enough. Every stage
/// here is individually shallow, so the refusal can only come from the stages
/// *accumulating* against the shared budget rather than each being charged and
/// released.
#[test]
fn a_pipeline_chain_is_charged_to_the_recursion_guard() {
    let stage = " |> Str::trim($_)";

    let short = format!("$a{}", stage.repeat(64));
    parse_ok(&short);

    let long = format!("$a{}", stage.repeat(MAX_RECURSION_DEPTH as usize + 1));
    let (_, diags) = parse_with_diags(&long);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TOO_DEEPLY_NESTED)),
        "a chain past the recursion limit must be refused: {diags:?}"
    );
}

/// A hole belongs to the `|>` whose right side encloses it most closely, which
/// falls out of the substitution being eager: the inner operator has already
/// taken its own left side by the time the outer one counts.
#[test]
fn the_hole_binds_to_the_nearest_enclosing_right_side() {
    assert_eq!(
        rendered("$a |> Str::format($b |> Str::trim($_), $_)", false),
        rendered("Str::format(Str::trim($b), $a)", false),
    );
    let shape = rendered("$a |> Str::format($b |> Str::trim($_), $_)", true);
    assert!(
        shape.contains("Variable(`$b`)") && shape.contains("Variable(`$a`)"),
        "each hole takes its own left side: {shape}"
    );
}

/// `rule:expressions/pipeline-hole-once`'s lower bound. Recovery keeps the
/// right side, so the rest of the file reports its own problems in this run.
#[test]
fn a_right_side_with_no_hole_is_refused() {
    let (e, diags) = parse_with_diags("$a |> Str::trim($b)");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, vec![code::E_PIPELINE_RIGHT_SIDE_HAS_NO_HOLE]);
    assert!(matches!(e.kind, ExprKind::StaticCall { .. }), "{e:?}");
}

/// The upper bound, and the reason it is exactly one rather than at least one:
/// a second hole would need the left side evaluated twice or bound to a
/// temporary, and neither is a substitution.
#[test]
fn a_right_side_with_two_holes_is_refused() {
    let (_, diags) = parse_with_diags("$a |> Math::max($_, $_)");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, vec![code::E_PIPELINE_RIGHT_SIDE_REPEATS_THE_HOLE]);
}

/// `$_` is not a variable anywhere, so it is refused before the casing rule
/// that rejects an all-underscore identifier ever sees one.
#[test]
fn a_hole_outside_a_pipeline_is_refused() {
    for src in ["$_ + 1", "Str::trim($_)", "$_"] {
        let (_, diags) = parse_with_diags(src);
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        assert_eq!(codes, vec![code::E_HOLE_OUTSIDE_A_PIPELINE], "{src}");
    }
}

/// The shared spelling with PHP 8.5 is affordable because the habit is named
/// where it is written: PHP's `|>` applies a callable, and a right side that
/// *is* a callable is the shape a reader carrying that habit reaches for.
#[test]
fn the_php_callable_shape_is_named_in_the_help_of_the_no_hole_refusal() {
    for src in ["$a |> Str::trim(...)", "$a |> fn($x) => $x"] {
        let (_, diags) = parse_with_diags(src);
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        assert!(
            codes.contains(&code::E_PIPELINE_RIGHT_SIDE_HAS_NO_HOLE),
            "{src}: {codes:?}"
        );
        assert!(
            diags
                .iter()
                .any(|d| d.notes.iter().any(|n| n.contains("PHP 8.5"))),
            "{src}: the callable shape is not named"
        );
    }

    // A right side that is an ordinary call gets the refusal without the note:
    // the reader who forgot the hole is not told about a design they were not
    // reaching for.
    let (_, diags) = parse_with_diags("$a |> Str::trim($b)");
    assert!(
        diags
            .iter()
            .all(|d| d.notes.iter().all(|n| !n.contains("PHP 8.5")))
    );
}

/// `rule:php-migration/let-and-is-are-reserved`: `let` is a reserved word with
/// no construct behind it, so it is refused wherever a program could have used
/// it as a name, and the help names `var` — the spelling that declares an
/// inferred local.
#[test]
fn let_is_a_reserved_spelling() {
    for src in ["let", "let(1)", "$a + let"] {
        let (_, diags) = parse_with_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_RESERVED_FOR_FUTURE_USE)),
            "expected E_RESERVED_FOR_FUTURE_USE for {src:?}, got {diags:?}"
        );
        assert!(
            diags
                .iter()
                .any(|d| d.notes.iter().any(|n| n.contains("`var`"))),
            "{src}: the living spelling is not named"
        );
    }
}

/// `rule:types/type-test`: `is` is the type test, so the refusal it carried
/// while it was a held spelling is gone from the operator position. `let` has
/// no construct behind it and keeps its half of
/// `rule:php-migration/let-and-is-are-reserved` untouched — as does `is` in
/// the one position where it is a *name* and not an operator, where the help
/// now names the operator it became.
#[test]
fn is_no_longer_reports_the_reserved_word_refusal_and_let_still_does() {
    let (e, diags) = parse_with_diags("$a is Foo");
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(matches!(e.kind, ExprKind::TypeTest { .. }), "{e:?}");

    for src in ["let", "let(1)", "$a + let"] {
        let (_, diags) = parse_with_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_RESERVED_FOR_FUTURE_USE)),
            "expected E_RESERVED_FOR_FUTURE_USE for {src:?}, got {diags:?}"
        );
        assert!(
            diags
                .iter()
                .any(|d| d.notes.iter().any(|n| n.contains("`var`"))),
            "{src}: the living spelling is not named"
        );
    }

    let (_, diags) = parse_with_diags("is($a)");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_RESERVED_FOR_FUTURE_USE)),
        "a call through `is` names it: {diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| d.notes.iter().any(|n| n.contains("`$x is T`"))),
        "the operator `is` became is not named: {diags:?}"
    );
}

/// The type on the right of an `is`, or a panic naming what was there instead.
fn tested_type(e: Expr) -> Type {
    let ExprKind::TypeTest { against, .. } = e.kind else {
        panic!("expected a type test: {:?}", e.span);
    };
    match against {
        TestOperand::Type(ty) => ty,
        TestOperand::Value(value) => panic!("expected a type operand: {value:?}"),
    }
}

/// `rule:types/type-test`: the right-hand side is read by `parse_type`, so
/// every shape the type grammar admits reaches it. `array<int>` and a shape
/// are the tells — the first would be two comparisons on the expression side
/// and the second has no expression spelling that means this at all.
#[test]
fn is_parses_its_right_hand_side_with_parse_type_and_not_as_an_expression() {
    let e = parse_ok("$x is int");
    let ExprKind::TypeTest { expr, against } = e.kind else {
        panic!("expected a type test: {e:?}");
    };
    assert!(matches!(expr.kind, ExprKind::Variable(_)), "{expr:?}");
    let TestOperand::Type(ty) = against else {
        panic!("expected a type operand: {against:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Atom(TypeAtom::Int)), "{ty:?}");

    let ty = tested_type(parse_ok("$x is array<int>"));
    let TypeKind::Atom(TypeAtom::Array(Some(elem))) = ty.kind else {
        panic!("expected an array type: {ty:?}");
    };
    assert!(
        matches!(elem.kind, TypeKind::Atom(TypeAtom::Int)),
        "{elem:?}"
    );

    let ty = tested_type(parse_ok("$x is {a: int, b: string}"));
    let TypeKind::Atom(TypeAtom::Shape(fields)) = ty.kind else {
        panic!("expected a shape: {ty:?}");
    };
    assert_eq!(fields.len(), 2);
}

/// `rule:types/type-test`: one precedence level, left-associative, so a chain
/// reads left to right and `!` applies to the `bool` the test answers rather
/// than to its subject.
#[test]
fn is_binds_below_unary_not_and_above_every_binary_operator() {
    let e = parse_ok("!$x is int");
    let ExprKind::Unary {
        op: UnaryOp::Not,
        expr,
    } = e.kind
    else {
        panic!("expected a top-level `!`: {e:?}");
    };
    assert!(matches!(expr.kind, ExprKind::TypeTest { .. }), "{expr:?}");

    // A chain is left-associative: the inner test is the *subject* of the
    // outer one, which is what one shared level means.
    let e = parse_ok("$x is Foo is bool");
    let ExprKind::TypeTest { expr, .. } = e.kind else {
        panic!("expected a top-level type test: {e:?}");
    };
    assert!(matches!(expr.kind, ExprKind::TypeTest { .. }), "{expr:?}");

    // Tighter than every binary operator, `as` excepted, which is postfix.
    let e = parse_ok("$a + $x is int");
    let ExprKind::Binary {
        op: BinaryOp::Add,
        rhs,
        ..
    } = e.kind
    else {
        panic!("expected a top-level `+`: {e:?}");
    };
    assert!(matches!(rhs.kind, ExprKind::TypeTest { .. }), "{rhs:?}");
}

/// `rule:types/type-test`: a union, an intersection and a `?` on the right are
/// one type each. Read as expressions they would be a bitwise chain and a
/// ternary, which is the whole reason the type arm is not `parse_pipe`.
#[test]
fn is_over_a_union_and_a_nullable_parses_as_one_type_and_not_as_a_comparison_chain() {
    let ty = tested_type(parse_ok("$x is int|string"));
    let TypeKind::Union(members) = ty.kind else {
        panic!("expected a union: {ty:?}");
    };
    assert_eq!(members.len(), 2);

    let ty = tested_type(parse_ok("$x is Countable&Traversable"));
    let TypeKind::Intersection(members) = ty.kind else {
        panic!("expected an intersection: {ty:?}");
    };
    assert_eq!(members.len(), 2);

    let ty = tested_type(parse_ok("$x is ?Foo"));
    assert!(matches!(ty.kind, TypeKind::Nullable(_)), "{ty:?}");
}

/// `rule:php-migration/one-type-test`: the word stays a keyword so the refusal
/// can name it, and the parser consumes the operand it was written with, so
/// the site costs exactly one diagnostic rather than a second about a token
/// nothing expected.
#[test]
fn instanceof_is_refused_naming_is_and_costs_one_diagnostic() {
    let (_, diags) = parse_with_diags("$x instanceof Foo");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec![code::E_INSTANCEOF_IS_NOT_AN_OPERATOR],
        "{diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.message.contains("`is`")),
        "the operator that replaces it is not named: {diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| d.notes.iter().any(|n| n.contains("$x is $cls"))),
        "the class-reference rewrite is not named: {diags:?}"
    );

    // The dynamic spelling is refused the same way and costs the same one
    // report: its right operand is consumed with it.
    let (_, diags) = parse_with_diags("$x instanceof $cls");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec![code::E_INSTANCEOF_IS_NOT_AN_OPERATOR],
        "{diags:?}"
    );
}

/// `rule:types/type-test` § *The value arm*: one token after `is` decides the
/// arm. A `$` opens the value one, parsed at the `|>` level so a property
/// chain is one operand; every other token starts a type, which is what keeps
/// a DNF type's opening `(` a type rather than a parenthesized value.
#[test]
fn a_variable_after_is_parses_as_a_value_operand_and_a_parenthesis_as_a_type() {
    let (e, diags) = parse_with_diags("$x is $cls");
    assert!(!diags.has_errors(), "{diags:?}");
    let ExprKind::TypeTest { against, .. } = e.kind else {
        panic!("expected a type test: {e:?}");
    };
    let TestOperand::Value(value) = against else {
        panic!("expected a value operand: {against:?}");
    };
    assert!(matches!(value.kind, ExprKind::Variable(_)), "{value:?}");

    // The value arm is parsed at the `|>` level, so the whole property chain
    // is the operand and not just the `$this`.
    let e = parse_ok("$x is $this->cls");
    let ExprKind::TypeTest { against, .. } = e.kind else {
        panic!("expected a type test: {e:?}");
    };
    let TestOperand::Value(value) = against else {
        panic!("expected a value operand: {against:?}");
    };
    assert!(
        matches!(value.kind, ExprKind::PropertyAccess { .. }),
        "{value:?}"
    );

    // A `(` is a type: this is the DNF spelling, not a parenthesized value.
    let ty = tested_type(parse_ok("$x is (Countable&Traversable)|int"));
    assert!(matches!(ty.kind, TypeKind::Union(_)), "{ty:?}");
}
