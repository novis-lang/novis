//! The type grammar's tests: every atom in every declaration slot, the
//! qualifiers, the shape type, and the spellings type position refuses.
//!
//! Part of [`super`]'s test suite, split to mirror the grammar modules
//! themselves; the helpers every module here calls are in [`super`].

use super::*;

#[test]
fn nested_array_generic_closes_through_a_split_shift_token() {
    // `>>` must split into two `>` closes, one per nesting level.
    let e = parse_ok("$m as array<array<uint>>");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    let TypeKind::Atom(TypeAtom::Array(Some(inner))) = ty.kind else {
        panic!("expected `array<...>`: {ty:?}");
    };
    assert!(matches!(
        inner.kind,
        TypeKind::Atom(TypeAtom::Array(Some(_)))
    ));
}

#[test]
fn a_name_in_type_position_carries_its_type_arguments() {
    let e = parse_ok("$m as Iterator<int>");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    let TypeKind::Atom(TypeAtom::Name(_, args)) = ty.kind else {
        panic!("expected a name atom: {ty:?}");
    };
    assert_eq!(args.len(), 1);
    assert!(matches!(args[0].kind, TypeKind::Atom(TypeAtom::Int)));
}

/// `>>` splits for a name's arguments exactly as it does for `array<...>`
/// -- both go through `expect_type_close_angle`.
#[test]
fn a_nested_generic_name_closes_through_a_split_shift_token() {
    let e = parse_ok("$m as array<Iterator<uint>>");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    let TypeKind::Atom(TypeAtom::Array(Some(inner))) = ty.kind else {
        panic!("expected `array<...>`: {ty:?}");
    };
    let TypeKind::Atom(TypeAtom::Name(_, args)) = inner.kind else {
        panic!("expected a name atom: {inner:?}");
    };
    assert_eq!(args.len(), 1);
}

#[test]
fn an_implements_entry_carries_its_type_arguments_and_its_delegation() {
    let stmts =
        parse_file_ok("<?nvs\nclass C implements Iterable<int>, Greets by $g, Comparable {}\n");
    let StmtKind::ClassDecl(decl) = &stmts[0].kind else {
        panic!("expected a class: {stmts:?}");
    };
    assert_eq!(decl.implements.len(), 3);
    assert_eq!(decl.implements[0].type_args.len(), 1);
    assert!(decl.implements[0].by_field.is_none());
    assert!(decl.implements[1].type_args.is_empty());
    assert!(decl.implements[1].by_field.is_some());
    assert!(decl.implements[2].type_args.is_empty());
}

// --- ADR 0047: literal and enum-case type atoms -------------------------

/// Parses `<ty>` in the one type position an expression test can reach —
/// a conversion's target — and returns it with the source text its span
/// covers, since half of what these atoms have to get right is *how much*
/// they span (a `-`, a pair of quotes, a `::`).
fn conversion_type(ty_src: &str) -> (Type, String) {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", format!("<?nvs $m as {ty_src}"));
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump(); // OpenTagNvs
    let e = p.parse_expr();
    assert!(
        !diags.has_errors(),
        "unexpected diagnostics for {ty_src:?}: {diags:?}"
    );
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    let written = text(&map, id, ty.span).to_owned();
    (ty, written)
}

/// Whether parsing `<ty>` in type position reported `want`.
fn conversion_type_reports(ty_src: &str, want: nvs_diagnostics::Code) -> bool {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", format!("<?nvs $m as {ty_src}"));
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump(); // OpenTagNvs
    let _ = p.parse_expr();
    diags.iter().any(|d| d.code == Some(want))
}

#[test]
fn a_string_literal_is_a_type_atom() {
    for src in [r#""a""#, "'a'"] {
        let (ty, written) = conversion_type(src);
        assert!(
            matches!(ty.kind, TypeKind::Atom(TypeAtom::StringLiteral(_))),
            "expected a string literal type for {src}: {ty:?}"
        );
        assert_eq!(written, src, "the span covers the quotes");
    }
}

#[test]
fn an_int_literal_is_a_type_atom_with_or_without_a_sign() {
    for src in ["1", "-1"] {
        let (ty, written) = conversion_type(src);
        assert!(
            matches!(ty.kind, TypeKind::Atom(TypeAtom::IntLiteral(_))),
            "expected an int literal type for {src}: {ty:?}"
        );
        assert_eq!(written, src, "the span covers a leading `-`");
    }
}

#[test]
fn literal_atoms_union_and_take_the_nullable_sugar() {
    let (ty, _) = conversion_type(r#""a"|"b"|"c""#);
    let TypeKind::Union(members) = ty.kind else {
        panic!("expected a union: {ty:?}");
    };
    assert_eq!(members.len(), 3);
    assert!(
        members
            .iter()
            .all(|m| matches!(m.kind, TypeKind::Atom(TypeAtom::StringLiteral(_))))
    );

    // `?"a"` is the same `?atom` production every other atom already has.
    let (ty, _) = conversion_type(r#"?"a""#);
    let TypeKind::Nullable(inner) = ty.kind else {
        panic!("expected `?T`: {ty:?}");
    };
    assert!(matches!(
        inner.kind,
        TypeKind::Atom(TypeAtom::StringLiteral(_))
    ));

    // A heterogeneous union is not this ADR's business to restrict.
    let (ty, _) = conversion_type("1|2|int");
    let TypeKind::Union(members) = ty.kind else {
        panic!("expected a union: {ty:?}");
    };
    assert_eq!(members.len(), 3);
    assert!(matches!(members[2].kind, TypeKind::Atom(TypeAtom::Int)));
}

#[test]
fn a_class_constant_or_enum_case_parses_in_type_position() {
    let (ty, written) = conversion_type("Foo::TYPE_A");
    assert!(
        matches!(ty.kind, TypeKind::Atom(TypeAtom::Member(..))),
        "expected a member atom: {ty:?}"
    );
    assert_eq!(written, "Foo::TYPE_A");

    // A namespace-qualified owner, and a union of two — the shape
    // ADR 0047 §§ 2-3 are both written in.
    let (ty, _) = conversion_type("App\\Mode::Read|App\\Mode::Write");
    let TypeKind::Union(members) = ty.kind else {
        panic!("expected a union: {ty:?}");
    };
    assert!(
        members
            .iter()
            .all(|m| matches!(m.kind, TypeKind::Atom(TypeAtom::Member(..))))
    );
}

#[test]
fn a_float_literal_is_refused_in_type_position() {
    for src in ["1.5", "-1.5"] {
        assert!(
            conversion_type_reports(src, code::E_FLOAT_LITERAL_TYPE),
            "expected E_FLOAT_LITERAL_TYPE for {src}"
        );
    }
}

#[test]
fn an_interpolated_string_is_refused_in_type_position() {
    assert!(
        conversion_type_reports(r#""a$b""#, code::E_INTERPOLATION_IN_TYPE),
        "expected E_INTERPOLATION_IN_TYPE"
    );
}

/// The statement-position half: a literal type declares a local exactly
/// as any other type does, and a statement that merely *starts* with a
/// literal still reaches the expression path through
/// `parse_stmt_maybe_local_decl`'s backtrack.
#[test]
fn a_literal_type_declares_a_local_without_swallowing_literal_expressions() {
    let s = parse_stmt_ok(r#""a"|"b" $mode = "a";"#);
    let StmtKind::LocalDecl { ty: Some(ty), .. } = s.kind else {
        panic!("expected a typed local: {s:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Union(_)));

    for src in ["1 + 2;", r#""x" . $y;"#, "1.5 * $x;", "-1 + $x;"] {
        let s = parse_stmt_ok(src);
        assert!(
            matches!(s.kind, StmtKind::Expr(_)),
            "expected an expression statement for {src}: {s:?}"
        );
    }
}

#[test]
fn a_literal_type_declares_a_parameter() {
    let stmts = parse_file_ok(
        "<?nvs\nclass C { public function setMode(\"a\"|\"b\"|\"c\" $mode): void {} }\n",
    );
    let StmtKind::ClassDecl(decl) = &stmts[0].kind else {
        panic!("expected a class: {stmts:?}");
    };
    let ClassMemberKind::Method(m) = &decl.members[0].kind else {
        panic!("expected a method: {decl:?}");
    };
    assert!(matches!(
        m.params[0].ty.as_ref().unwrap().kind,
        TypeKind::Union(_)
    ));
}

#[test]
fn union_and_intersection_types() {
    let e = parse_ok("$m as int|string");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    let TypeKind::Union(members) = ty.kind else {
        panic!("expected a union: {ty:?}");
    };
    assert_eq!(members.len(), 2);

    let e = parse_ok("$m as A&B");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Intersection(_)));
}

#[test]
fn decimal_is_a_type_atom_in_every_slot() {
    // ADR 0054 § 1: `decimal` is a scalar type atom, so it parses
    // wherever `float` does and stays distinct from it in the AST --
    // § 3's `decimal ⊕ float` compile error is only expressible if the
    // two never collapse.
    let e = parse_ok("$m as decimal");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Atom(TypeAtom::Decimal)));

    let s = parse_stmt_ok("decimal $price = 19.99;");
    let StmtKind::LocalDecl { ty, .. } = s.kind else {
        panic!("expected a local decl: {s:?}");
    };
    assert!(matches!(
        ty.map(|t| t.kind),
        Some(TypeKind::Atom(TypeAtom::Decimal))
    ));

    // Parameter, return and property slots, plus § 2's compile-time
    // constant -- the position a `Core\Decimal` class could never occupy.
    parse_stmt_ok(
        "class Invoice { \
             public const decimal VAT = 0.19; \
             public decimal $total = 0.0; \
             public function line(decimal $unit, uint $qty): decimal { return $unit * $qty; } \
             }",
    );

    // Nullable, union and `array<T>` element positions.
    parse_stmt_ok("type Money = ?decimal;");
    parse_stmt_ok("type Amount = decimal|int;");
    parse_stmt_ok("type Ledger = array<decimal>;");
}

#[test]
fn a_decimal_literal_suffix_does_not_parse() {
    // ADR 0054 § 2: there is no literal suffix, so `19.99m` is a float
    // literal followed by a stray identifier rather than a decimal --
    // `19.99 as decimal` is the only spelling. The lexer's
    // `a_trailing_m_is_not_a_decimal_literal_suffix` pins the token pair;
    // this pins that the parser refuses it rather than silently dropping
    // the `m`.
    let (_, diags) = parse_stmt_with_diags("$x = 19.99m;");
    assert!(diags.has_errors(), "expected `19.99m` to be refused");
}

#[test]
fn tainted_qualifies_string_and_bytes() {
    let e = parse_ok("$m as tainted string");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Atom(TypeAtom::TaintedString)));

    let e = parse_ok("$m as tainted bytes");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Atom(TypeAtom::TaintedBytes)));
}

#[test]
fn tainted_qualifier_parses_in_every_declaration_slot() {
    // Parameter and return type (ADR 0024 § 1).
    let s = parse_stmt_ok(
        "class C { \
             public function f(tainted string $s): tainted bytes { return $s as bytes; } \
             public tainted string $p; \
             } \
             ",
    );
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    let ClassMemberKind::Method(m) = &class.members[0].kind else {
        panic!("expected a method: {:?}", class.members[0]);
    };
    assert!(matches!(
        m.params[0].ty.as_ref().map(|t| &t.kind),
        Some(TypeKind::Atom(TypeAtom::TaintedString))
    ));
    assert!(matches!(
        m.return_type.as_ref().map(|t| &t.kind),
        Some(TypeKind::Atom(TypeAtom::TaintedBytes))
    ));
    let ClassMemberKind::Property(prop) = &class.members[1].kind else {
        panic!("expected a property: {:?}", class.members[1]);
    };
    assert!(matches!(
        prop.ty.kind,
        TypeKind::Atom(TypeAtom::TaintedString)
    ));

    // Local declaration.
    let s = parse_stmt_ok("tainted string $q;");
    let StmtKind::LocalDecl { ty, .. } = s.kind else {
        panic!("expected a local decl: {s:?}");
    };
    assert!(matches!(
        ty.unwrap().kind,
        TypeKind::Atom(TypeAtom::TaintedString)
    ));

    // `foreach` binding.
    let s = parse_stmt_ok("foreach ($rows as tainted string $row) { }");
    let StmtKind::Foreach { value, .. } = s.kind else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(matches!(
        value.ty.as_ref().map(|t| &t.kind),
        Some(TypeKind::Atom(TypeAtom::TaintedString))
    ));
}

#[test]
fn tainted_rejects_a_non_scalar_operand() {
    // An `as`-conversion type position isn't ambiguous with an expression
    // the way a bare statement's leading tokens can be, so the diagnostic
    // isn't swallowed by `parse_stmt_maybe_local_decl`'s trial-parse
    // fallback (see its comment) the way it would be at statement start.
    let (_, diags) = parse_with_diags("$m as tainted int");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TAINTED_NON_SCALAR)),
        "expected E_TAINTED_NON_SCALAR, got {diags:?}"
    );
}

#[test]
fn secret_qualifies_string_and_bytes_independently_of_tainted() {
    // ADR 0033 § 1: `secret` and `tainted` are independent bits — a value
    // can be `secret string`/`secret bytes` alone, or composed with
    // `tainted` (only in that order).
    let e = parse_ok("$m as secret string");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Atom(TypeAtom::SecretString)));

    let e = parse_ok("$m as secret bytes");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Atom(TypeAtom::SecretBytes)));

    let e = parse_ok("$m as secret tainted string");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(
        ty.kind,
        TypeKind::Atom(TypeAtom::SecretTaintedString)
    ));

    let e = parse_ok("$m as secret tainted bytes");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(
        ty.kind,
        TypeKind::Atom(TypeAtom::SecretTaintedBytes)
    ));
}

#[test]
fn secret_qualifier_parses_in_every_declaration_slot() {
    // Mirrors `tainted_qualifier_parses_in_every_declaration_slot` —
    // parameter, return type, property, local declaration, `foreach`
    // binding (ADR 0033 § 1).
    let s = parse_stmt_ok(
        "class C { \
             public function f(secret string $s): secret bytes { return $s as bytes; } \
             public secret string $p; \
             } \
             ",
    );
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    let ClassMemberKind::Method(m) = &class.members[0].kind else {
        panic!("expected a method: {:?}", class.members[0]);
    };
    assert!(matches!(
        m.params[0].ty.as_ref().map(|t| &t.kind),
        Some(TypeKind::Atom(TypeAtom::SecretString))
    ));
    assert!(matches!(
        m.return_type.as_ref().map(|t| &t.kind),
        Some(TypeKind::Atom(TypeAtom::SecretBytes))
    ));
    let ClassMemberKind::Property(prop) = &class.members[1].kind else {
        panic!("expected a property: {:?}", class.members[1]);
    };
    assert!(matches!(
        prop.ty.kind,
        TypeKind::Atom(TypeAtom::SecretString)
    ));

    // Local declaration.
    let s = parse_stmt_ok("secret string $q;");
    let StmtKind::LocalDecl { ty, .. } = s.kind else {
        panic!("expected a local decl: {s:?}");
    };
    assert!(matches!(
        ty.unwrap().kind,
        TypeKind::Atom(TypeAtom::SecretString)
    ));

    // `foreach` binding.
    let s = parse_stmt_ok("foreach ($rows as secret string $row) { }");
    let StmtKind::Foreach { value, .. } = s.kind else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(matches!(
        value.ty.as_ref().map(|t| &t.kind),
        Some(TypeKind::Atom(TypeAtom::SecretString))
    ));
}

#[test]
fn secret_rejects_a_non_scalar_operand() {
    let (_, diags) = parse_with_diags("$m as secret int");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_NON_SCALAR)),
        "expected E_SECRET_NON_SCALAR, got {diags:?}"
    );
}

#[test]
fn tainted_secret_wrong_order_is_diagnosed() {
    // ADR 0033 § 1: `secret` must be spelled before `tainted` — the
    // reverse order is a diagnostic, not a second valid spelling.
    let (_, diags) = parse_with_diags("$m as tainted secret string");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_TAINTED_ORDER)),
        "expected E_SECRET_TAINTED_ORDER, got {diags:?}"
    );
}

#[test]
fn shape_type_parses_in_every_declaration_slot() {
    // ADR 0036 § 3, mirroring `tainted`/`secret`'s own
    // every-declaration-slot tests.
    let s = parse_stmt_ok(
        "class C { \
             public function f({x: int} $p): {y: int} { return $p; } \
             public {x: int} $p; \
             } \
             ",
    );
    let StmtKind::ClassDecl(class) = s.kind else {
        panic!("expected a class decl: {s:?}");
    };
    let ClassMemberKind::Method(m) = &class.members[0].kind else {
        panic!("expected a method: {:?}", class.members[0]);
    };
    let Some(TypeKind::Atom(TypeAtom::Shape(fields))) = m.params[0].ty.as_ref().map(|t| &t.kind)
    else {
        panic!("expected a shape param type: {:?}", m.params[0].ty);
    };
    assert_eq!(fields.len(), 1);
    assert!(matches!(fields[0].ty.kind, TypeKind::Atom(TypeAtom::Int)));
    assert!(matches!(
        m.return_type.as_ref().map(|t| &t.kind),
        Some(TypeKind::Atom(TypeAtom::Shape(_)))
    ));
    let ClassMemberKind::Property(prop) = &class.members[1].kind else {
        panic!("expected a property: {:?}", class.members[1]);
    };
    assert!(matches!(prop.ty.kind, TypeKind::Atom(TypeAtom::Shape(_))));

    // `foreach` binding.
    let s = parse_stmt_ok("foreach ($rows as {x: int} $row) { }");
    let StmtKind::Foreach { value, .. } = s.kind else {
        panic!("expected a foreach: {s:?}");
    };
    assert!(matches!(
        value.ty.as_ref().map(|t| &t.kind),
        Some(TypeKind::Atom(TypeAtom::Shape(_)))
    ));

    // Reusable via a `type` alias (ADR 0015), same as the ADR's own
    // `type Point = {x: int, y: int};` example.
    let s = parse_stmt_ok("type Point = {x: int, y: int};");
    let StmtKind::TypeAliasDecl(alias) = s.kind else {
        panic!("expected a type alias: {s:?}");
    };
    assert!(matches!(alias.ty.kind, TypeKind::Atom(TypeAtom::Shape(_))));
}

#[test]
fn shape_type_can_be_empty_and_composes_with_array_and_union() {
    // ADR 0036 § 3: an empty `{}` in type position carries the same
    // "no field promised" meaning as plain `object` — no ambiguity with
    // a block exists in type position, unlike expression position.
    let e = parse_ok("$m as {}");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    assert!(matches!(ty.kind, TypeKind::Atom(TypeAtom::Shape(f)) if f.is_empty()));

    // Nested inside `array<T>`.
    let e = parse_ok("$m as array<{x: int}>");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    let TypeKind::Atom(TypeAtom::Array(Some(inner))) = ty.kind else {
        panic!("expected an array type: {ty:?}");
    };
    assert!(matches!(inner.kind, TypeKind::Atom(TypeAtom::Shape(_))));

    // As one member of a union.
    let e = parse_ok("$m as {x: int}|null");
    let ExprKind::Conversion { ty, .. } = e.kind else {
        panic!("expected a conversion: {e:?}");
    };
    let TypeKind::Union(members) = ty.kind else {
        panic!("expected a union: {ty:?}");
    };
    assert!(matches!(
        members[0].kind,
        TypeKind::Atom(TypeAtom::Shape(_))
    ));
}
