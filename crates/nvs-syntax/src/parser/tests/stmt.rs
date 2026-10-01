//! The statement grammar's tests, including both backtracking sites and
//! every statement-shaped reject.
//!
//! Part of [`super`]'s test suite, split to mirror the grammar modules
//! themselves; the helpers every module here calls are in [`super`].

use super::*;

#[test]
fn if_elseif_else_chain() {
    let s = parse_stmt_ok("if ($a) { 1; } elseif ($b) { 2; } else { 3; }");
    let StmtKind::If { then, else_, .. } = s.kind else {
        panic!("expected an if: {s:?}");
    };
    assert!(matches!(then.kind, StmtKind::Block(_)));
    let else_ = else_.expect("elseif chain");
    let StmtKind::If {
        else_: inner_else, ..
    } = else_.kind
    else {
        panic!("expected `elseif` to produce a nested if: {else_:?}");
    };
    assert!(matches!(inner_else.unwrap().kind, StmtKind::Block(_)));
}

#[test]
fn else_if_two_words_matches_elseif() {
    // `else if (...)` recurses through the ordinary statement dispatch
    // rather than a dedicated `elseif` production, but produces the same
    // nested-`If` shape.
    let s = parse_stmt_ok("if ($a) { 1; } else if ($b) { 2; }");
    let StmtKind::If { else_, .. } = s.kind else {
        panic!("expected an if: {s:?}");
    };
    assert!(matches!(else_.unwrap().kind, StmtKind::If { .. }));
}

#[test]
fn while_do_while_and_for() {
    let s = parse_stmt_ok("while ($i < 10) { $i++; }");
    assert!(matches!(s.kind, StmtKind::While { .. }));

    let s = parse_stmt_ok("do { $i++; } while ($i < 10);");
    assert!(matches!(s.kind, StmtKind::DoWhile { .. }));

    let s = parse_stmt_ok("for ($i = 0; $i < 10; $i++) { }");
    let StmtKind::For {
        init, cond, step, ..
    } = s.kind
    else {
        panic!("expected a for loop: {s:?}");
    };
    assert_eq!(init.exprs().len(), 1);
    assert_eq!(cond.len(), 1);
    assert_eq!(step.len(), 1);
}

/// The init clause of `for` as [`ForInit`], for a source that must parse
/// clean — `rule:iteration/for-init-clause`'s two alternatives are told apart by asking this.
fn for_init_of(src: &str) -> ForInit {
    let s = parse_stmt_ok(src);
    let StmtKind::For { init, .. } = s.kind else {
        panic!("expected a for loop: {s:?}");
    };
    init
}

#[test]
fn a_for_init_clause_declares_one_typed_local() {
    // `rule:iteration/for-init-clause`: the declaration form, in every spelling `local-decl`
    // covers — a scalar type, `rule:types/var-inference`'s `var`, a nullable, a generic whose
    // `<` the expression grammar also claims.
    for src in [
        "for (int $i = 0; $i < 3; $i = $i + 1) { }",
        "for (var $i = 0; $i < 3; $i = $i + 1) { }",
        "for (?string $i = null; $i == null; $i = \"x\") { }",
        "for (array<int> $row = []; $i < 3; $i = $i + 1) { }",
    ] {
        let init = for_init_of(src);
        let decl = init
            .decl()
            .unwrap_or_else(|| panic!("expected a declaration: {src}"));
        let StmtKind::LocalDecl { name, value, .. } = &decl.kind else {
            panic!("expected a local declaration: {decl:?}");
        };
        assert!(!name.is_empty(), "{src}");
        assert!(value.is_some(), "{src}");
        assert!(init.exprs().is_empty(), "{src}");
    }
}

#[test]
fn a_for_init_clause_is_still_a_list_of_expressions() {
    // `rule:iteration/for-init-clause`'s trial parse in the other direction, plus the shape the
    // corpus used before it. Each of the first four begins with a token that
    // starts a *type* — a `Name`, `static`, `(` — and is an expression all
    // the same, settled only by what does not follow it; that is the same
    // backtracking site `parse_stmt_maybe_local_decl` already owns.
    for (src, items) in [
        ("for ($i = 0; $i < 2; $i = $i + 1) { }", 1),
        ("for (Counter::tick(); $i < 2; $i = $i + 1) { }", 1),
        ("for (static::tick(); $i < 2; $i = $i + 1) { }", 1),
        ("for (($a || $b) ? f() : g(); $i < 2; $i = $i + 1) { }", 1),
        ("for ($i = 0, $j = 1; $i < 2; $i = $i + 1) { }", 2),
        ("for (;;) { }", 0),
    ] {
        let init = for_init_of(src);
        assert!(init.decl().is_none(), "should not be a declaration: {src}");
        assert_eq!(init.exprs().len(), items, "{src}");
    }
}

#[test]
fn a_for_init_clause_mixing_a_declaration_and_an_expression_is_e0124() {
    // `rule:iteration/for-init-refusals`: one diagnostic naming the rule, in either order, and
    // nothing else — the twelve-error resynchronisation cascade in that
    // ADR's *Context* is what this replaces, so the count is the assertion.
    for src in [
        "for (int $i = 0, $j = 1; $i < 3; $i = $i + 1) { }",
        "for ($j = 1, int $i = 0; $i < 3; $i = $i + 1) { }",
    ] {
        let (s, diags) = parse_stmt_with_diags(src);
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        assert_eq!(
            codes,
            vec![code::E_FOR_INIT_MIXES_DECL_AND_EXPR],
            "for {src:?}: {diags:?}"
        );
        // The declaration is kept either way, so no later phase sees an
        // undeclared counter and reports its own error about it.
        let StmtKind::For { init, .. } = s.kind else {
            panic!("expected a for loop: {s:?}");
        };
        assert!(init.decl().is_some(), "{src}");
    }
}

#[test]
fn a_for_init_clause_with_two_declarations_is_e0125() {
    // `rule:iteration/for-init-refusals`'s other code, for the shape a reader coming from C
    // writes. Separate from E0124 because the fix is different: the second
    // declaration goes above the loop.
    let src = "for (int $i = 0, int $j = 0; $i < 3; $i = $i + 1) { }";
    let (s, diags) = parse_stmt_with_diags(src);
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec![code::E_FOR_INIT_TWO_DECLARATIONS],
        "for {src:?}: {diags:?}"
    );
    let StmtKind::For { init, .. } = s.kind else {
        panic!("expected a for loop: {s:?}");
    };
    assert!(init.decl().is_some());
}

/// A `try` guards something or it is not a `try`: either clause satisfies the
/// rule, and the statement is still built either way so the rest of the file
/// is parsed rather than abandoned at the keyword.
#[test]
fn a_try_without_a_clause_is_e0242() {
    let (s, diags) = parse_stmt_with_diags("try { $a = 1; }");
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, vec![code::E_TRY_WITHOUT_CLAUSE], "{diags:?}");
    assert!(matches!(s.kind, StmtKind::Try { .. }));

    for src in [
        "try { $a = 1; } catch (Throwable $e) { }",
        "try { $a = 1; } finally { }",
        "try { $a = 1; } catch (Throwable $e) { } finally { }",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(!diags.has_errors(), "for {src:?}: {diags:?}");
    }
}

/// A clause's binding carries one static type, so the type grammar's union —
/// which parses here for free — is refused rather than supported. The clause
/// is still built from its first class, so the block behind it is parsed.
#[test]
fn a_catch_clause_naming_two_classes_is_e0245() {
    let src = "try { $a = 1; } catch (LogicError | RuntimeError $e) { $a = 2; }";
    let (s, diags) = parse_stmt_with_diags(src);
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec![code::E_CATCH_UNION_TYPE_UNSUPPORTED],
        "{diags:?}"
    );
    let StmtKind::Try { catches, .. } = s.kind else {
        panic!("expected a try: {s:?}");
    };
    assert_eq!(catches.len(), 1);
    assert!(
        matches!(catches[0].ty.kind, TypeKind::Atom(_)),
        "the clause stands in for the union with its first class: {:?}",
        catches[0].ty.kind
    );
    assert_eq!(catches[0].body.stmts.len(), 1);
}

#[test]
fn empty_statement_and_empty_for_body() {
    let s = parse_stmt_ok(";");
    assert!(matches!(s.kind, StmtKind::Empty));
    let s = parse_stmt_ok("for (;;) ;");
    let StmtKind::For { body, .. } = s.kind else {
        panic!("expected a for loop: {s:?}");
    };
    assert!(matches!(body.kind, StmtKind::Empty));
}

#[test]
fn foreach_with_typed_key_and_value() {
    let s = parse_stmt_ok("foreach ($rows as string $k => array<int> $row) { }");
    let StmtKind::Foreach {
        key,
        value,
        value_inout,
        ..
    } = s.kind
    else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(key.is_some());
    assert!(value.written_ty().is_some());
    assert!(!value_inout);
}

#[test]
fn foreach_value_only_by_reference() {
    let s = parse_stmt_ok("foreach ($items as inout int $v) { }");
    let StmtKind::Foreach {
        key, value_inout, ..
    } = s.kind
    else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(key.is_none());
    assert!(value_inout);
}

#[test]
fn foreach_key_and_by_reference_value() {
    let s = parse_stmt_ok("foreach ($items as string $k => inout int $v) { }");
    let StmtKind::Foreach {
        key, value_inout, ..
    } = s.kind
    else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(key.is_some());
    assert!(value_inout);
}

/// `rule:types/var-inference`: `var` stands where either binding's type goes.
#[test]
fn foreach_binding_accepts_var_for_the_key_and_the_value() {
    let s = parse_stmt_ok("foreach ($stock as var $name => var $qty) { }");
    let StmtKind::Foreach { key, value, .. } = s.kind else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(
        matches!(key.map(|k| k.ty), Some(ForeachBindingTy::Var(_))),
        "the key is `var`"
    );
    assert!(matches!(value.ty, ForeachBindingTy::Var(_)), "{value:?}");
}

#[test]
fn foreach_binding_accepts_var_after_inout() {
    let s = parse_stmt_ok("foreach ($items as inout var $v) { }");
    let StmtKind::Foreach {
        value, value_inout, ..
    } = s.kind
    else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(value_inout);
    assert!(matches!(value.ty, ForeachBindingTy::Var(_)), "{value:?}");
}

/// Neither a type nor `var` is still the parse error it was, and the binding
/// says so — the checker reads `Omitted` as recovery, never as `var`.
#[test]
fn foreach_binding_with_no_type_and_no_var_is_still_refused() {
    let (s, diags) = parse_stmt_with_diags("foreach ($items as $v) { }");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_EXPECTED_TOKEN)),
        "{diags:?}"
    );
    let StmtKind::Foreach { value, .. } = s.kind else {
        panic!("expected a foreach: {s:?}");
    };
    assert_eq!(value.ty, ForeachBindingTy::Omitted);
}

#[test]
fn foreach_header_as_belongs_to_foreach_not_conversion() {
    // `rule:types/conversion` / docs/spec/00-overview.md § 3.2: converting the
    // *subject* inside a `foreach` header needs parens, since a bare
    // `as` right after the subject is `foreach`'s own separator.
    let s = parse_stmt_ok("foreach (($m as array<int>) as int $v) { }");
    insta::assert_debug_snapshot!(s);
}

#[test]
fn switch_with_fallthrough_and_default() {
    let s = parse_stmt_ok("switch ($x) { case 1: case 2: echo $x; break; default: echo 0; }");
    let StmtKind::Switch { cases, .. } = s.kind else {
        panic!("expected a switch: {s:?}");
    };
    assert_eq!(cases.len(), 3);
    assert!(cases[0].cond.is_some());
    assert!(cases[0].body.is_empty(), "fallthrough case has no body");
    assert!(cases[2].cond.is_none(), "the last arm is `default`");
}

#[test]
fn break_and_continue_with_level() {
    let s = parse_stmt_ok("break;");
    assert!(matches!(s.kind, StmtKind::Break(None)));
    let s = parse_stmt_ok("continue 2;");
    assert!(matches!(s.kind, StmtKind::Continue(Some(_))));
}

/// `rule:php-migration/no-return-leaves-a-finally`: the `return` would replace
/// whatever the region was leaving with, so it is refused wherever it belongs
/// to the `finally` itself — and left alone in a nested body, which it leaves
/// instead of the block.
#[test]
fn a_return_never_leaves_a_finally() {
    for src in [
        // A bare `return` leaves the region the same way a valued one does.
        "try { $a = 1; } finally { return 1; }",
        "try { $a = 1; } finally { return; }",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        assert_eq!(codes, vec![code::E_RETURN_LEAVES_A_FINALLY], "{src}");
    }

    for src in [
        // The `try` and the `catch` both keep it.
        "try { return 1; } catch (TypeError $e) { return 2; }",
        // A closure written in the block returns from itself.
        "try { $a = 1; } finally { $f = fn() => { return 1; }; }",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_RETURN_LEAVES_A_FINALLY)),
            "{src}: {diags:?}"
        );
    }
}

/// `rule:php-migration/no-return-leaves-a-finally`: a `break` or `continue` is
/// refused only where its target lies outside the `finally`, so the level is
/// read against the loops and `switch`es the block itself opened rather than
/// against the ones around the `try`.
#[test]
fn a_break_leaves_a_finally_only_when_its_target_is_outside() {
    for src in [
        "while ($a) { try { $b = 1; } finally { break; } }",
        "while ($a) { try { $b = 1; } finally { while ($c) { continue 2; } } }",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        assert_eq!(codes, vec![code::E_BREAK_LEAVES_A_FINALLY], "{src}");
    }

    for src in [
        "while ($a) { try { $b = 1; } finally { while ($c) { break; } } }",
        "while ($a) { try { $b = 1; } finally { switch ($c) { case 1: break; } } }",
        "while ($a) { try { $b = 1; } finally { foreach ($c as int $d) { continue; } } }",
        // Outside a `finally` every level is ordinary.
        "while ($a) { while ($b) { break 2; } }",
    ] {
        let (_, diags) = parse_stmt_with_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_BREAK_LEAVES_A_FINALLY)),
            "{src}: {diags:?}"
        );
    }
}

/// Two classes are two clauses — the union spelling is
/// [`a_catch_clause_naming_two_classes_is_e0245`] — and each clause carries a
/// `finally` past all of them.
#[test]
fn try_two_catches_and_finally() {
    let s = parse_stmt_ok(
        "try { risky(); } catch (TypeError $e) { } catch (ValueError $v) { } finally { cleanup(); }",
    );
    let StmtKind::Try {
        catches, finally, ..
    } = s.kind
    else {
        panic!("expected a try: {s:?}");
    };
    assert_eq!(catches.len(), 2);
    assert!(matches!(catches[0].ty.kind, TypeKind::Atom(_)));
    assert!(catches[0].var.is_some());
    assert!(finally.is_some());
}

#[test]
fn catch_without_a_variable() {
    let s = parse_stmt_ok("try { } catch (Throwable) { }");
    let StmtKind::Try { catches, .. } = s.kind else {
        panic!("expected a try: {s:?}");
    };
    assert!(catches[0].var.is_none());
}

#[test]
fn echo_and_unset() {
    let s = parse_stmt_ok("echo 1, 2, 3;");
    let StmtKind::Echo(exprs) = s.kind else {
        panic!("expected echo: {s:?}");
    };
    assert_eq!(exprs.len(), 3);

    let s = parse_stmt_ok("unset($a, $b);");
    let StmtKind::Unset(exprs) = s.kind else {
        panic!("expected unset: {s:?}");
    };
    assert_eq!(exprs.len(), 2);
}

#[test]
fn typed_local_declaration_with_and_without_initializer() {
    let s = parse_stmt_ok("int $n = 0;");
    let StmtKind::LocalDecl { ty, value, .. } = s.kind else {
        panic!("expected a local decl: {s:?}");
    };
    assert!(matches!(ty.unwrap().kind, TypeKind::Atom(TypeAtom::Int)));
    assert!(value.is_some());

    let s = parse_stmt_ok("array<uint> $ids;");
    let StmtKind::LocalDecl { value, .. } = s.kind else {
        panic!("expected a local decl: {s:?}");
    };
    assert!(value.is_none());
}

/// `rule:types/var-inference`: `var $n = 0;` parses to the same `LocalDecl` shape as the
/// typed spelling, but with `ty: None` — the checker fills it in from
/// `value`'s own type.
#[test]
fn var_local_declaration_has_no_written_type() {
    let s = parse_stmt_ok("var $n = 0;");
    let StmtKind::LocalDecl { ty, value, .. } = s.kind else {
        panic!("expected a local decl: {s:?}");
    };
    assert!(ty.is_none());
    assert!(value.is_some());
}

/// `var` has nothing to infer a type from without an initializer, so
/// (unlike the typed spelling) one is mandatory.
#[test]
fn var_local_declaration_requires_an_initializer() {
    let (_, diags) = parse_stmt_with_diags("var $n;");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_EXPECTED_TOKEN)),
        "expected E_EXPECTED_TOKEN, got {diags:?}"
    );
}

#[test]
fn a_parenthesized_expression_statement_is_not_confused_with_a_type() {
    // `(` also starts a type (a parenthesized union/intersection), so a
    // statement beginning with `(non-type-expr)` trial-parses as a local
    // decl first. The trial must fail cleanly here: `parse_type_atom`'s
    // error recovery doesn't consume the offending token, so without
    // checking whether the trial itself reported anything, the cursor
    // landing on a `$variable` by coincidence (as it does right after
    // `$a` here) reads as "yes, a type was followed by a variable."
    let s = parse_stmt_ok("($a > 0 || $b > 0) ? f($a) : g($a);");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    assert!(matches!(e.kind, ExprKind::Ternary { .. }));
}

#[test]
fn a_name_used_as_a_type_is_not_confused_with_a_static_call() {
    let s = parse_stmt_ok("User $owner = User::find($id);");
    assert!(matches!(s.kind, StmtKind::LocalDecl { .. }));

    let s = parse_stmt_ok("Foo::bar();");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    assert!(matches!(e.kind, ExprKind::StaticCall { .. }));
}

/// `docs/spec/01-core-library.md` § 6's `decodeAs<T>` spelling: the
/// `<...>` between a member name and its `(` is a type-argument list, not
/// two comparisons.
#[test]
fn a_static_call_takes_a_written_type_argument() {
    let s = parse_stmt_ok(r"Core\Json::decodeAs<User>($body);");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    let ExprKind::StaticCall { type_args, .. } = e.kind else {
        panic!("expected a static call: {e:?}");
    };
    assert_eq!(type_args.len(), 1);
}

/// Two of them, the second nested — so the `>>` the lexer already
/// committed to has to be split, exactly as `array<array<uint>>` in type
/// position does.
#[test]
fn a_method_call_takes_written_type_arguments_and_closes_a_nested_one() {
    let s = parse_stmt_ok("$db->queryAs<int, array<array<string>>>($sql);");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    let ExprKind::MethodCall { type_args, .. } = e.kind else {
        panic!("expected a method call: {e:?}");
    };
    assert_eq!(type_args.len(), 2);
}

/// The trial parse only commits when a `(` follows the closing `>`, so a
/// bare comparison chain is untouched — and so is one whose operands
/// could never be types.
#[test]
fn a_comparison_chain_is_not_a_type_argument_list() {
    for src in [
        "$x = Foo::BAR < Baz;",
        "$x = Foo::BAR < $baz > $qux;",
        "$x = $a->count < $b > $c;",
    ] {
        let s = parse_stmt_ok(src);
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement for {src:?}: {s:?}");
        };
        let ExprKind::Assign { value, .. } = e.kind else {
            panic!("expected an assignment for {src:?}: {e:?}");
        };
        assert!(
            matches!(value.kind, ExprKind::Binary { .. }),
            "expected a comparison for {src:?}: {value:?}"
        );
    }
}

/// A member name with no `<` at all keeps every existing shape: a call
/// with an empty list, and a property access that never looked for one.
#[test]
fn a_call_without_type_arguments_carries_an_empty_list() {
    let s = parse_stmt_ok("$user->name();");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    let ExprKind::MethodCall { type_args, .. } = e.kind else {
        panic!("expected a method call: {e:?}");
    };
    assert!(type_args.is_empty());

    let s = parse_stmt_ok("$user->name;");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    assert!(matches!(e.kind, ExprKind::PropertyAccess { .. }));
}

#[test]
fn a_bitwise_or_of_two_constants_is_not_confused_with_a_union_type() {
    let s = parse_stmt_ok("Foo | Bar;");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    assert!(matches!(
        e.kind,
        ExprKind::Binary {
            op: BinaryOp::BitOr,
            ..
        }
    ));
}

#[test]
fn destructure_brackets_typed_nested_and_skipped() {
    let s = parse_stmt_ok("[int $a, , string $b] = $triple;");
    let StmtKind::Destructure { target, .. } = s.kind else {
        panic!("expected a destructure: {s:?}");
    };
    assert_eq!(target.elements.len(), 3);
    assert!(matches!(
        target.elements[0],
        DestructureElement::Leaf { .. }
    ));
    assert!(matches!(target.elements[1], DestructureElement::Skip));
    assert!(matches!(
        target.elements[2],
        DestructureElement::Leaf { .. }
    ));

    let s = parse_stmt_ok("[[int $x, int $y], string $label] = $point;");
    let StmtKind::Destructure { target, .. } = s.kind else {
        panic!("expected a destructure: {s:?}");
    };
    assert!(matches!(
        target.elements[0],
        DestructureElement::Nested { .. }
    ));
}

#[test]
fn destructure_with_string_keys() {
    let s = parse_stmt_ok("['id' => uint $id, 'name' => string $name] = $row;");
    let StmtKind::Destructure { target, .. } = s.kind else {
        panic!("expected a destructure: {s:?}");
    };
    let DestructureElement::Leaf { key, .. } = &target.elements[0] else {
        panic!("expected a leaf: {:?}", target.elements[0]);
    };
    assert!(key.is_some());
}

/// ADR 0050: `list(...)` is diagnosed naming `[...]`, and never reaches
/// the AST as a live `StmtKind::Destructure` the way it used to.
#[test]
fn list_is_diagnosed_naming_bracket_destructuring() {
    for src in [
        "list(int $a, string $b) = $pair;",
        "list(, int $second) = $triple;",
        "list('id' => uint $id) = $row;",
    ] {
        let (s, diags) = parse_stmt_with_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LIST_DESTRUCTURING_UNSUPPORTED)),
            "expected E_LIST_DESTRUCTURING_UNSUPPORTED for {src:?}, got {diags:?}"
        );
        assert!(
            matches!(s.kind, StmtKind::Error),
            "expected a rejected statement for {src:?}, got {s:?}"
        );
    }

    // The `[...]` spelling of each of the same shapes is unaffected.
    for src in [
        "[int $a, string $b] = $pair;",
        "[, int $second] = $triple;",
        "['id' => uint $id] = $row;",
    ] {
        let s = parse_stmt_ok(src);
        assert!(
            matches!(s.kind, StmtKind::Destructure { .. }),
            "expected a destructure for {src:?}, got {s:?}"
        );
    }
}

#[test]
fn a_plain_array_literal_statement_is_not_confused_with_destructuring() {
    let s = parse_stmt_ok("[1, 2, 3];");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    assert!(matches!(e.kind, ExprKind::ArrayLiteral(_)));
}

#[test]
fn global_is_diagnosed_but_still_parses() {
    let (s, diags) = parse_stmt_with_diags("global $a, $b;");
    assert!(diags.has_errors());
    let StmtKind::Global(vars) = s.kind else {
        panic!("expected global: {s:?}");
    };
    assert_eq!(vars.len(), 2);
}

#[test]
fn goto_is_diagnosed_but_still_parses() {
    let (s, diags) = parse_stmt_with_diags("goto done;");
    assert!(diags.has_errors());
    assert!(matches!(s.kind, StmtKind::Goto(_)));
}

#[test]
fn function_scope_static_is_diagnosed_but_still_parses() {
    let (s, diags) = parse_stmt_with_diags("static $calls = 0;");
    assert!(diags.has_errors());
    let StmtKind::StaticLocal { ty, vars } = s.kind else {
        panic!("expected a static local: {s:?}");
    };
    assert!(ty.is_none());
    assert_eq!(vars.len(), 1);
}

#[test]
fn typed_function_scope_static_is_also_diagnosed() {
    let (s, diags) = parse_stmt_with_diags("static int $calls = 0;");
    assert!(diags.has_errors());
    let StmtKind::StaticLocal { ty, .. } = s.kind else {
        panic!("expected a static local: {s:?}");
    };
    assert!(ty.is_some());
}

#[test]
fn static_closure_and_static_property_are_not_confused_with_function_static() {
    // `static fn`/`static function` are already diagnosed as unsupported
    // closure modifiers (chunk 1) — the point here is that they must
    // NOT also be routed into the function-scope-`static` rejection.
    let (s, diags) = parse_stmt_with_diags("static fn (int $x) => $x;");
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_STATIC_LOCAL_UNSUPPORTED))
    );
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    assert!(matches!(e.kind, ExprKind::Fn(_)));

    parse_stmt_ok("static::method();");
    parse_stmt_ok("$x = static::$prop;");
}

#[test]
fn eval_extract_settype_are_diagnosed() {
    for (src, code) in [
        ("eval($src);", code::E_EVAL_UNSUPPORTED),
        ("extract($arr);", code::E_EXTRACT_UNSUPPORTED),
        ("settype($x, 'int');", code::E_SETTYPE_UNSUPPORTED),
    ] {
        let (s, diags) = parse_stmt_with_diags(src);
        assert!(diags.has_errors(), "expected a diagnostic for {src:?}");
        assert!(
            diags.iter().any(|d| d.code == Some(code)),
            "expected {code:?} for {src:?}, got {diags:?}"
        );
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(e.kind, ExprKind::Error(_)));
    }
}

#[test]
fn die_is_diagnosed_naming_exit() {
    for src in ["die;", "die();", "die('bye');"] {
        let (s, diags) = parse_stmt_with_diags(src);
        assert!(diags.has_errors(), "expected a diagnostic for {src:?}");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DIE_UNSUPPORTED)),
            "expected E_DIE_UNSUPPORTED for {src:?}, got {diags:?}"
        );
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(e.kind, ExprKind::Error(_)));
    }

    // `exit` in every one of the same shapes is unaffected.
    for src in ["exit;", "exit();", "exit('bye');", "exit(1);"] {
        let (s, diags) = parse_stmt_with_diags(src);
        assert!(
            !diags.has_errors(),
            "unexpected diagnostics for {src:?}: {diags:?}"
        );
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(e.kind, ExprKind::Exit(_)));
    }
}

#[test]
fn require_is_an_expression() {
    let s = parse_stmt_ok("$x = require 'a.nvs';");
    let StmtKind::Expr(e) = s.kind else {
        panic!("expected an expression statement: {s:?}");
    };
    let ExprKind::Assign { value, .. } = e.kind else {
        panic!("expected an assignment: {e:?}");
    };
    assert!(matches!(value.kind, ExprKind::Require { .. }));

    parse_stmt_ok("require 'd.nvs';");
}

/// ADR 0021: `require` is the only same-frame inclusion keyword kept —
/// `include`, `include_once` and `require_once` are all diagnosed.
#[test]
fn include_family_is_diagnosed() {
    for src in [
        "include 'a.nvs';",
        "include_once 'c.nvs';",
        "require_once 'b.nvs';",
    ] {
        let (s, diags) = parse_stmt_with_diags(src);
        assert!(diags.has_errors(), "expected a diagnostic for {src:?}");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INCLUDE_FAMILY_UNSUPPORTED)),
            "expected E_INCLUDE_FAMILY_UNSUPPORTED for {src:?}, got {diags:?}"
        );
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(e.kind, ExprKind::Error(_)));
    }
}
