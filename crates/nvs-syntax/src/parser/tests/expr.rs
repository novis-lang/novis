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
    // ADR 0007 § 2: `$a as int + 1` is `($a as int) + 1`.
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
    // legacy-cast spelling is rejected (ADR 0034), but it must still
    // consume `!$x` as its operand rather than leaving it dangling.
    let (e, diags) = parse_with_diags("(int) !$x");
    assert!(matches!(e.kind, ExprKind::Error));
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
    // ADR 0036 § 2.
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
    // ADR 0036 § 2: every field is `name: value` — no shorthand, no
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
    // ADR 0036 § 2: `fn() => {...}` already means a block body per
    // ADR 0031 — returning a literal needs `fn() => ({...})` instead.
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
    // ADR 0036 § 2: a statement-initial `{` already means a block —
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

/// ADR 0034: `as` is the only conversion spelling — PHP's legacy
/// `(T)expr` cast syntax is diagnosed, naming `as` as the replacement,
/// the same shape ADR 0021 already gives `include`/`include_once`/
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
        assert!(matches!(e.kind, ExprKind::Error));
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
        assert!(matches!(e.kind, ExprKind::Error));
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
    assert!(matches!(e.kind, ExprKind::Error));
}

/// ADR 0034: at statement start specifically, `(string)$x;` is also a
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
    // ADR 0031 § 1/§ 2: `function` closures don't exist at all, and a
    // `use (&$y)` clause gets its own, more specific diagnostic on top.
    let (e, diags) = parse_with_diags("function (int $x) use (&$y): int { return $x + $y; }");
    assert!(matches!(e.kind, ExprKind::Error));
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
    assert!(matches!(e.kind, ExprKind::Error));
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
    assert!(matches!(e.kind, ExprKind::Error));
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
fn missing_parameter_type_is_diagnosed() {
    let (e, diags) = parse_with_diags("fn ($x) => $x");
    assert!(diags.has_errors());
    let ExprKind::Fn(f) = e.kind else {
        panic!("expected an fn expression: {e:?}");
    };
    assert!(f.params[0].ty.is_none());
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
    assert!(matches!(e.kind, ExprKind::Error));
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
    assert!(matches!(e.kind, ExprKind::Error));
}

#[test]
fn dollar_brace_expr_is_variable_variable() {
    let (e, diags) = parse_with_diags("${$name}");
    assert!(diags.has_errors());
    assert!(matches!(e.kind, ExprKind::Error));
}
