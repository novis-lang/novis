//! Resolves a parsed [`Type`] into an interned [`TypeId`] (`rule:types/grammar`'s
//! grammar, made concrete).
//!
//! A `self`/`static` atom resolves against [`Ctx::current_class`]; `parent`
//! resolves the same way `crate::expr::members::resolve_class_expr`'s `ParentExpr` arm
//! and `check_new_target`'s `NewTarget::ParentTy` arm do — [`Ctx::current_class`]'s
//! first `extends` link via [`Env::graph`] — except a `parent` that can't
//! resolve (no enclosing class, or a class with no `extends`) is a
//! diagnostic here rather than those two's silent `mixed` fallback, since a
//! *type* position naming an unresolvable class is a real authoring mistake
//! the same way an out-of-class `self`/`static` already is. A `Name`
//! atom resolves via [`nvs_hir::resolve_ref`] — the same
//! unqualified/qualified/fully-qualified resolution every `nvs-hir` resolver
//! already shares — then checks [`nvs_hir::AliasTable`] first (an alias is
//! "resolved eagerly," per `rule:types/type-alias`, so its expansion is substituted
//! in and lowered recursively rather than kept as a name), falling back to
//! [`nvs_hir::SymbolTable`] to decide between a class-shaped atom (a class or
//! interface — the type grammar does not distinguish them) and an enum. A
//! name that resolves to neither, and is not trusted as a `Core`
//! reference or one of [`nvs_hir::errors`]' exception classes
//! ([`nvs_hir::QName::is_reserved_global_class`]), is `E_UNDEFINED_CLASS`.
//!
//! `array<...>` nesting is bounded at depth 32 (`rule:types/arrays`) — past that,
//! lowering stops and reports `E_ARRAY_TYPE_TOO_DEEP` rather than recursing
//! further, so a pathological type cannot make lowering superlinear.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::SymbolKind;
use nvs_syntax::ast::{ImplementsClause, Name, Type, TypeAtom, TypeKind};
use rustc_hash::FxHashSet;

use crate::ty::TypeId;
use crate::{Ctx, Env, span_text};

const MAX_ARRAY_DEPTH: u32 = 32;

/// Lowers `ty` into an interned [`TypeId`], within the given class/namespace
/// scope.
pub(crate) fn lower_type(ty: &Type, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    let id = lower_type_at_depth(ty, 0, ctx, env);
    // Persisted for `nvs-ir`, which lowers a declared type off the AST and so
    // cannot resolve a name-shaped atom for itself — see
    // `crate::expr_table::ExprTypeTable::declared_ty`. Recorded here, at the
    // one entry point every annotation goes through, rather than at each of
    // this function's callers.
    env.exprs.record_type(ty.span, id);
    id
}

/// Lowers an optional declared type, e.g. a `foreach` binding or destructure
/// leaf that omitted its type (already diagnosed elsewhere, per
/// [`nvs_syntax::ast::ForeachBinding::ty`]'s own doc), defaulting to `mixed`.
///
/// A plain `ty.map_or_else(|| env.interner.mixed(), |t| lower_type(t, ctx,
/// env))` does not borrow-check: the two closures would each need their own
/// exclusive borrow of `env` while both exist as arguments, before either
/// runs — an ordinary `match` has no such restriction.
pub(crate) fn lower_optional_type(ty: Option<&Type>, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    match ty {
        Some(t) => lower_type(t, ctx, env),
        None => env.interner.mixed(),
    }
}

/// Lowers one declared type, bounded at [`MAX_ARRAY_DEPTH`] so that no
/// annotation can make checking superlinear (`rule:types/arrays`).
///
/// # Known gaps
///
/// 1. The bound's message names an `array type` whatever the type actually
///    was, so a shape past the depth is refused with a sentence about arrays.
///    A `type` alias naming itself is the easiest way to reach it — the alias
///    expands until the depth stops it, and each of the shape's fields reports
///    — and a self-referential alias has no diagnostic of its own. That is a
///    cycle check in alias resolution; this wording is a separate, smaller
///    fix.
fn lower_type_at_depth(ty: &Type, depth: u32, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    if depth > MAX_ARRAY_DEPTH {
        env.diags.report(
            Diagnostic::error(
                code::E_ARRAY_TYPE_TOO_DEEP,
                format!("array type nests past depth {MAX_ARRAY_DEPTH}"),
            )
            .with_primary(ty.span, "too deeply nested"),
        );
        return env.interner.mixed();
    }

    match &ty.kind {
        TypeKind::Nullable(inner) => {
            let inner_id = lower_type_at_depth(inner, depth, ctx, env);
            let null_id = env.interner.null();
            env.interner.make_union([inner_id, null_id])
        }
        TypeKind::Union(items) => {
            let ids: Vec<TypeId> = items
                .iter()
                .map(|t| lower_type_at_depth(t, depth, ctx, env))
                .collect();
            env.interner.make_union(ids)
        }
        TypeKind::Intersection(items) => {
            let ids: Vec<TypeId> = items
                .iter()
                .map(|t| lower_type_at_depth(t, depth, ctx, env))
                .collect();
            env.interner.make_intersection(ids)
        }
        TypeKind::Paren(inner) => lower_type_at_depth(inner, depth, ctx, env),
        TypeKind::Atom(atom) => lower_atom(atom, ty.span, depth, ctx, env),
        _ => env.interner.mixed(),
    }
}

fn lower_atom(atom: &TypeAtom, span: Span, depth: u32, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    match atom {
        TypeAtom::Null => env.interner.null(),
        TypeAtom::Bool => env.interner.bool_ty(),
        TypeAtom::Int => env.interner.int(),
        TypeAtom::Uint => env.interner.uint(),
        TypeAtom::Float => env.interner.float(),
        TypeAtom::Decimal => env.interner.decimal(),
        TypeAtom::String => env.interner.string(),
        TypeAtom::Bytes => env.interner.bytes(),
        TypeAtom::TaintedString => env.interner.tainted_string(),
        TypeAtom::TaintedBytes => env.interner.tainted_bytes(),
        TypeAtom::SecretString => env.interner.secret_string(),
        TypeAtom::SecretBytes => env.interner.secret_bytes(),
        TypeAtom::SecretTaintedString => env.interner.secret_tainted_string(),
        TypeAtom::SecretTaintedBytes => env.interner.secret_tainted_bytes(),
        TypeAtom::Array(inner) => {
            let elem = match inner {
                Some(t) => lower_type_at_depth(t, depth + 1, ctx, env),
                None => env.interner.mixed(),
            };
            env.interner.array(elem)
        }
        TypeAtom::ClassRef(inner) => lower_class_ref(inner, depth, ctx, env),
        TypeAtom::PropertyKey(inner) => lower_property_key(inner, depth, ctx, env),
        TypeAtom::Object => env.interner.object(),
        TypeAtom::Shape(fields) => {
            let mut out = Vec::with_capacity(fields.len());
            for field in fields {
                let name = span_text(env.src, field.name).to_owned();
                let field_ty = lower_type_at_depth(&field.ty, depth + 1, ctx, env);
                out.push(crate::ty::ShapeField {
                    name,
                    ty: field_ty,
                    required: field.required,
                });
            }
            env.interner.shape(out)
        }
        TypeAtom::Mixed => env.interner.mixed(),
        TypeAtom::Void => env.interner.void(),
        TypeAtom::Never => env.interner.never(),
        TypeAtom::True => env.interner.true_ty(),
        TypeAtom::False => env.interner.false_ty(),
        TypeAtom::Iterable => env.interner.iterable(),
        TypeAtom::Callable => env.interner.callable(),
        // `rule:types/callable-signature`: a written signature is a type of its
        // own, so it lowers field-wise. Each parameter and the return type go
        // one level deeper, under the same nesting bound every other type
        // argument is checked against — a signature nests the way `array<T>`
        // does and gets no bound of its own.
        TypeAtom::CallableSig { params, ret } => {
            let mut lowered = Vec::with_capacity(params.len());
            for param in params {
                lowered.push(lower_type_at_depth(param, depth + 1, ctx, env));
            }
            let ret = lower_type_at_depth(ret, depth + 1, ctx, env);
            env.interner.callable_sig(lowered, ret)
        }
        TypeAtom::SelfTy => resolve_special(span, "self", ctx, env),
        TypeAtom::StaticTy => resolve_special(span, "static", ctx, env),
        TypeAtom::Parent => resolve_parent(span, ctx, env),
        TypeAtom::Name(name, args) => resolve_name_type(name, args, span, depth, ctx, env),
        TypeAtom::StringLiteral(lit) => {
            let value = crate::string_lit::cook_string_literal(env.src, *lit);
            env.interner.string_literal(value)
        }
        TypeAtom::IntLiteral(lit) => lower_int_literal_type(*lit, env),
        TypeAtom::Member(name, member) => lower_member_type(name, *member, span, depth, ctx, env),
        _ => env.interner.mixed(),
    }
}

/// `rule:types/class-reference`'s `class<T>`: the argument names one class or one interface,
/// and anything else is refused where it is written.
///
/// The refusal lives here rather than in the parser because the parser sees
/// only the spelling, and `class<int>` and `class<Undeclared>` are two
/// different mistakes ([`TypeAtom::ClassRef`]'s own doc says so). The second is
/// already `E_UNDEFINED_CLASS`, reported by the recursive lowering below, which
/// then recovers as `mixed` — so this reports only when that lowering was
/// itself quiet. Without that guard an unresolvable name would collect a second
/// diagnostic here saying `mixed` is not a class, which is true and useless.
///
/// A refused argument recovers as `mixed` rather than as `class<mixed>`: there
/// is no such type — [`crate::ty::Ty::ClassRef`] promises its argument is a
/// class — and `mixed` is the one recovery every other atom in this module
/// already falls back to.
fn lower_class_ref(inner: &Type, depth: u32, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    let before = env.diags.error_count();
    let arg = lower_type_at_depth(inner, depth + 1, ctx, env);
    if matches!(env.interner.get(arg), crate::ty::Ty::Class(..)) {
        return env.interner.class_ref(arg);
    }
    if env.diags.error_count() == before {
        let described = env.interner.describe(arg);
        env.diags.report(
            Diagnostic::error(
                code::E_CLASS_REF_ARGUMENT_NOT_A_CLASS,
                format!("`class<{described}>` names something that is not a class"),
            )
            .with_primary(inner.span, "a class or interface name is required here")
            .with_help(
                "a class reference's value is a class descriptor, so its argument is the class \
                 or interface every descriptor it can hold conforms to",
            ),
        );
    }
    env.interner.mixed()
}

/// `rule:types/property-key`'s `property<T>`: the argument names one class, and anything
/// else is refused where it is written.
///
/// [`lower_class_ref`]'s shape, with its guard and its `mixed` recovery, and
/// one row narrower: a class reference admits an interface because its value is
/// a descriptor conforming to that interface, while a key's values are the
/// *names* an implementor declares — which an interface does not have. The
/// symbol's kind is the whole test, and `Ty::Class` cannot answer it, since a
/// class, an interface and an enum all intern as one.
///
/// **The set's own emptiness is not asked here.** `rule:types/property-key` also refuses a
/// class declaring no public property at all, and that needs the flattened
/// public roster the `as` conversion is built around — so it lands with the
/// conversion, as
/// [`crate::expr::operators::reject_empty_property_key_set`], whose doc says
/// why this pass cannot host it: this one runs during signature collection
/// too, where every roster is still empty.
fn lower_property_key(inner: &Type, depth: u32, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    let before = env.diags.error_count();
    let arg = lower_type_at_depth(inner, depth + 1, ctx, env);
    let named_class = match env.interner.get(arg) {
        crate::ty::Ty::Class(qname, _) => {
            let qname = qname.clone();
            env.symbols
                .get(&qname)
                .is_some_and(|sym| sym.kind == SymbolKind::Class)
        }
        _ => false,
    };
    if named_class {
        return env.interner.property_key(arg);
    }
    if env.diags.error_count() == before {
        let described = env.interner.describe(arg);
        env.diags.report(
            Diagnostic::error(
                code::E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS,
                format!("`property<{described}>` names something that is not a class"),
            )
            .with_primary(inner.span, "a class name is required here")
            .with_help(
                "a property key's values are the names of that class's public declared \
                 properties, so its argument has to be something that declares them — an \
                 interface's implementors satisfy it with storage it does not declare itself",
            ),
        );
    }
    env.interner.mixed()
}

/// `rule:types/literal-types`'s `int` literal atom.
///
/// The atom's span covers a leading `-` when one was written
/// ([`TypeAtom::IntLiteral`]), so the sign is split off here and the digits go
/// through [`crate::expr::int_literal_digits`] — the one integer grammar, the
/// same one an `enum` case value and a parameter default already read.
///
/// A magnitude no `int` holds is `E_INT_LITERAL_OUT_OF_RANGE`, the same
/// diagnostic the identical mistake takes in a *value* position, and recovers
/// as plain `int`: that is the base type the author meant, so nothing
/// downstream meets a type it has no rule for.
fn lower_int_literal_type(lit: Span, env: &mut Env<'_>) -> TypeId {
    let text = span_text(env.src, lit);
    let digits_text = text.strip_prefix('-').map(str::trim_start);
    let negated = digits_text.is_some();
    let offset = u32::try_from(text.len() - digits_text.unwrap_or(text).len()).unwrap_or(0);
    let digits_span = Span::new(lit.file, lit.start.saturating_add(offset), lit.end);
    let (radix, digits) = crate::expr::int_literal_digits(env.src, digits_span);
    let value = u64::from_str_radix(&digits, radix)
        .ok()
        .and_then(|magnitude| {
            if negated {
                negate_magnitude(magnitude)
            } else {
                i64::try_from(magnitude).ok()
            }
        });
    match value {
        Some(value) => env.interner.int_literal(value),
        None => {
            env.diags.report(
                Diagnostic::error(
                    code::E_INT_LITERAL_OUT_OF_RANGE,
                    "this integer literal type names a value no `int` holds",
                )
                .with_primary(lit, "outside `int`'s range")
                .with_help(
                    "`rule:types/literal-types`'s literal atom is an `int` literal, so the value has to be \
                     one an `int` can hold",
                ),
            );
            env.interner.int()
        }
    }
}

/// `-magnitude` as an `i64`, `i64::MIN`'s own magnitude included — the same
/// edge [`crate::defaults`] and [`crate::consts`] each answer at the other two
/// places a written magnitude is negated.
fn negate_magnitude(magnitude: u64) -> Option<i64> {
    if magnitude == (i64::MAX as u64) + 1 {
        return Some(i64::MIN);
    }
    i64::try_from(magnitude).ok().and_then(i64::checked_neg)
}

/// `Foo::BAR` in type position — one atom with three meanings, told apart here
/// because telling them apart is what resolution is for.
///
/// An **alias the owner declares** comes first: `rule:types/type-alias`'s
/// class-scoped member expands and re-lowers, the way [`resolve_name_type`]
/// substitutes a file-scope one, so `Order::Meta` and the shape it names are
/// one type in both directions. An **enum** name then gives
/// `rule:types/enum-case-type`'s [`crate::ty::Ty::EnumCase`]: a narrowed
/// subtype of the enum, never its backing value, so a bare `int` still cannot
/// satisfy it. Anything else is `rule:types/constant-in-type-position`'s class
/// constant, which folds to *its own literal type* — sugar, and safe precisely
/// because a scalar `const` is not a distinct nominal type, so `Foo::TYPE_A`
/// genuinely is the string `"a"`. The order decides only which of the three an
/// unknown name is reported against.
///
/// The enum test is [`crate::expr::members::infer_class_const`]'s, verbatim: a
/// `Core`-owned enum has no [`SymbolKind::Enum`] entry — nothing declared it —
/// but it is in the same [`crate::enums::EnumTable`], seeded from
/// `nvs_stdlib::registry::ENUMS`.
///
/// The name *left* of the `::` is read through [`Env::aliases`] first: an alias
/// standing for a single name atom stands in for that name before the member is
/// read, and any other expansion has no member to read at all, which is
/// `E_UNKNOWN_MEMBER` with a help naming what the alias expands to.
fn lower_member_type(
    name: &Name,
    member: Span,
    span: Span,
    depth: u32,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let text = span_text(env.src, name.span);
    let case = span_text(env.src, member).to_owned();

    let qname = match alias_left_of_member(text, ctx, env) {
        Some(expansion) => match name_atom_of(&expansion, ctx, env) {
            Some(stood_in) => stood_in,
            None => {
                return report_expansion_has_no_member(
                    &expansion, text, &case, span, depth, ctx, env,
                );
            }
        },
        None => nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports),
    };

    if let Some(alias_ty) = env.aliases.get_member(&qname, &case) {
        let alias_ty = alias_ty.clone();
        return lower_type_at_depth(&alias_ty, depth, ctx, env);
    }

    let is_enum = matches!(env.symbols.get(&qname), Some(sym) if sym.kind == SymbolKind::Enum)
        || (qname.is_core() && env.enums.get(&qname).is_some());
    if is_enum {
        let backing = env.enums.backing_of(&qname);
        return match env.enums.case(&qname, &case) {
            Some(_) => env.interner.enum_case(qname, backing, case),
            // Recovered as the whole enum rather than as `mixed`: the author
            // named one of its cases, so the enum is what they meant, and a
            // later diagnostic about the *value* is more use than one about a
            // type nothing checks.
            None => {
                report_unknown(span, &qname, &case, "case", env);
                env.interner.enum_(qname, backing)
            }
        };
    }
    lower_class_const_type(&qname, &case, span, env)
}

/// The expansion the name left of a `::` stands for, when that name is itself a
/// `type` alias.
///
/// The enclosing body's own member is looked up before the file-scope name the
/// namespace and imports resolve to — the order `nvs_hir::aliases` already uses
/// for a bare name, and the same one [`resolve_name_type`] applies to a name
/// with no `::` after it. Nothing is inherited, so an owner declaring no such
/// member falls straight through to the namespace.
fn alias_left_of_member(text: &str, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<nvs_syntax::ast::Type> {
    if let Some(owner) = ctx.current_class
        && let Some(ty) = env.aliases.get_member(owner, text)
    {
        return Some(ty.clone());
    }
    let qname = nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports);
    env.aliases.get(&qname).cloned()
}

/// The single class, interface or enum name `ty` is, if it is nothing else.
///
/// Parentheses are transparent here for the same reason they are everywhere
/// else in the grammar, and a name written with type arguments is not one of
/// these: an argument list means a generic class, whose members are not reached
/// through the alias that named it.
fn name_atom_of(ty: &Type, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<nvs_hir::QName> {
    let mut kind = &ty.kind;
    while let TypeKind::Paren(inner) = kind {
        kind = &inner.kind;
    }
    let TypeKind::Atom(TypeAtom::Name(name, args)) = kind else {
        return None;
    };
    if !args.is_empty() {
        return None;
    }
    let text = span_text(env.src, name.span);
    Some(nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports))
}

/// `E_UNKNOWN_MEMBER` for `Alias::Name` where the alias expands to something
/// with no members to read — an array, a union, a shape, a scalar.
///
/// The expansion is lowered before it is named, so the help says what the
/// author's own alias means rather than repeating the spelling they already
/// wrote. Recovering as `mixed` matches every other unreadable member here:
/// there is no narrower type that could honestly be meant.
fn report_expansion_has_no_member(
    expansion: &Type,
    text: &str,
    case: &str,
    span: Span,
    depth: u32,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let expanded = lower_type_at_depth(expansion, depth, ctx, env);
    let shown = env.interner.describe(expanded);
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{text}` expands to `{shown}`, which has no member named `{case}`"),
        )
        .with_primary(span, "read through a `type` alias here")
        .with_help(format!(
            "`rule:types/type-alias` makes an alias its expansion everywhere, so `{text}::{case}` \
             reads `{case}` from `{shown}`; only an alias standing for a single class, interface \
             or enum name has a member to read"
        )),
    );
    env.interner.mixed()
}

/// `rule:types/constant-in-type-position`'s fold, for a name that resolved to something other than an
/// enum.
///
/// A **declared** class's constants come from [`crate::consts::ConstTable`],
/// which walked them before the first annotation was interned. A `Core` class's
/// come from the registry through [`crate::core_lib::constant`], which is where
/// every other `Core` constant is already read from; a `Core` class the registry
/// does not state stays trusted and lowers to `mixed`, exactly the narrowing
/// [`crate::expr::members::infer_class_const`] applies to the same question on
/// the expression side.
///
/// Both of § 2's mistakes are diagnosed at the *use*, not the declaration: a
/// value with no literal type to fold to is `E_LITERAL_TYPE_NOT_CONST`, and a
/// name nothing declares is `E_UNKNOWN_MEMBER`. Each recovers as `mixed`,
/// since neither leaves a narrower type that could honestly be meant.
fn lower_class_const_type(
    qname: &nvs_hir::QName,
    name: &str,
    span: Span,
    env: &mut Env<'_>,
) -> TypeId {
    if qname.is_core() {
        return match crate::core_lib::constant(qname, name, env.interner) {
            Some((_, crate::defaults::ConstArg::Str(value))) => env.interner.string_literal(value),
            Some((_, crate::defaults::ConstArg::Int(value))) => env.interner.int_literal(value),
            Some((_, crate::defaults::ConstArg::Uint(value))) => match i64::try_from(value) {
                Ok(value) => env.interner.int_literal(value),
                Err(_) => report_not_const(span, qname, name, env),
            },
            Some(_) => report_not_const(span, qname, name, env),
            None => {
                // The registry is the whole roster of `Core`, so a class it
                // does not hold has no constant either — the same withdrawal of
                // blanket trust `crate::expr::members` makes one position over.
                report_missing_member(span, qname, name, "constant", env);
                env.interner.mixed()
            }
        };
    }
    match env.consts.get(qname, name, env.graph) {
        Some(crate::consts::ConstValue::Str(value)) => {
            let value = value.clone();
            env.interner.string_literal(value)
        }
        Some(crate::consts::ConstValue::Int(value)) => {
            let value = *value;
            env.interner.int_literal(value)
        }
        // `rule:types/constant-in-type-position`'s literal types are `string` and `int`; a `bool` or
        // `float` constant is folded (`crate::defaults` reads the value) but
        // has no literal type to *be*, so it is the same mistake as an array
        // constant here.
        Some(
            crate::consts::ConstValue::Bool(_)
            | crate::consts::ConstValue::Float(_)
            | crate::consts::ConstValue::Ineligible,
        ) => report_not_const(span, qname, name, env),
        None => {
            report_missing_member(span, qname, name, "constant", env);
            env.interner.mixed()
        }
    }
}

/// Nothing `qname` itself declares answers to `name`. An **ancestor**'s own
/// `type` alias may, and that is a different mistake with a different repair:
/// a class-scoped alias is reached through the owner that declares it and is
/// inherited by nothing (`rule:types/type-alias`), so the help names the
/// spelling that resolves instead of leaving the author to guess which of the
/// three member kinds the name was looked up as.
fn report_missing_member(
    span: Span,
    qname: &nvs_hir::QName,
    name: &str,
    kind: &str,
    env: &mut Env<'_>,
) {
    let Some(owner) = alias_owned_by_an_ancestor(qname, name, env) else {
        report_unknown(span, qname, name, kind, env);
        return;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{qname}` has no `type` named `{name}`"),
        )
        .with_primary(span, "not inherited")
        .with_help(format!(
            "`{owner}::{name}` is where it is declared, and that is the spelling that resolves \
             — a class-scoped `type` is reached through its own owner and through no subclass or \
             implementor (`rule:types/type-alias`)"
        )),
    );
}

/// The nearest `extends`/`implements` ancestor of `qname` that declares a
/// `type` alias called `name`, if one does. Breadth-first, so a name declared
/// twice up the chain answers with the closest owner.
fn alias_owned_by_an_ancestor(
    qname: &nvs_hir::QName,
    name: &str,
    env: &Env<'_>,
) -> Option<nvs_hir::QName> {
    let mut seen: FxHashSet<nvs_hir::QName> = FxHashSet::default();
    let mut queue = supertypes_of(qname, env);
    let mut next = 0;
    while next < queue.len() {
        let candidate = queue[next].clone();
        next += 1;
        if !seen.insert(candidate.clone()) {
            continue;
        }
        if env.aliases.get_member(&candidate, name).is_some() {
            return Some(candidate);
        }
        queue.extend(supertypes_of(&candidate, env));
    }
    None
}

/// One declaration's supertypes: `extends` and `implements` together, which is
/// the single "walk the parents" step `nvs_hir`'s class links are shaped for.
fn supertypes_of(qname: &nvs_hir::QName, env: &Env<'_>) -> Vec<nvs_hir::QName> {
    env.graph.get(qname).map_or_else(Vec::new, |links| {
        links
            .extends
            .iter()
            .chain(links.implements.iter())
            .cloned()
            .collect()
    })
}

/// `rule:types/constant-in-type-position`'s "a constant backed by a non-scalar type is not eligible, and
/// using one this way is a diagnostic naming the eligible types."
fn report_not_const(span: Span, qname: &nvs_hir::QName, name: &str, env: &mut Env<'_>) -> TypeId {
    env.diags.report(
        Diagnostic::error(
            code::E_LITERAL_TYPE_NOT_CONST,
            format!("`{qname}::{name}` is not a `string` or `int` compile-time constant"),
        )
        .with_primary(span, "no literal type to fold to")
        .with_help(
            "`rule:types/constant-in-type-position` folds a class constant used as a type to that value's own literal \
             type, so only a `string` or `int` constant may be written here — write the base \
             type instead",
        ),
    );
    env.interner.mixed()
}

/// The `Foo` has no `kind` named `NAME` diagnostic, at the two type-position
/// spellings that can reach it — the same message
/// [`crate::expr::members::report_unknown_member`] gives the identical mistake
/// on the expression side, reported here rather than shared because that one is
/// `pub(super)` to `expr` and this position is not one of its callers.
fn report_unknown(span: Span, qname: &nvs_hir::QName, name: &str, kind: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNKNOWN_MEMBER,
            format!("`{qname}` has no {kind} named `{name}`"),
        )
        .with_primary(span, "referenced here"),
    );
}

fn resolve_special(span: Span, keyword: &str, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    match ctx.current_class {
        // ADR 0010: an enum has no methods to declare a `self`/`static` type
        // atom inside in a well-formed program, but the parser still
        // recovers a rejected member (`E_ENUM_MEMBER_UNSUPPORTED`) and hands
        // it to this checker anyway — resolve the same way `resolve_name_type`
        // below already does for an explicit enum name, rather than always
        // interning `Ty::Class`.
        Some(qname) => match env.symbols.get(qname) {
            Some(sym) if sym.kind == SymbolKind::Enum => {
                let backing = env.enums.backing_of(qname);
                env.interner.enum_(qname.clone(), backing)
            }
            _ => env.interner.class(qname.clone()),
        },
        None => {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_CLASS,
                    format!("`{keyword}` used outside any class"),
                )
                .with_primary(span, "no enclosing class"),
            );
            env.interner.mixed()
        }
    }
}

/// Resolves the `parent` type atom against [`Ctx::current_class`]'s first
/// `extends` link — the same hop `crate::expr::members::resolve_class_expr` uses for
/// `parent::` on the expression side, and `check_new_target`'s
/// `NewTarget::ParentTy` arm uses for `new parent(...)`. Unlike those two,
/// which fall back to `mixed` with no diagnostic for a `parent` that can't
/// resolve, this is a *type* position (a parameter, property or return type)
/// where naming an unresolvable class the same way `self`/`static` do outside
/// a class is a real authoring mistake worth its own diagnostic.
fn resolve_parent(span: Span, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    let Some(current) = ctx.current_class else {
        env.diags.report(
            Diagnostic::error(code::E_UNDEFINED_CLASS, "`parent` used outside any class")
                .with_primary(span, "no enclosing class"),
        );
        return env.interner.mixed();
    };
    match env
        .graph
        .get(current)
        .and_then(|links| links.extends.first())
    {
        Some(parent) => env.interner.class(parent.clone()),
        None => {
            env.diags.report(
                Diagnostic::error(
                    code::E_NO_PARENT_CLASS,
                    format!("`{current}` has no parent class to refer to as `parent`"),
                )
                .with_primary(span, "`parent` type"),
            );
            env.interner.mixed()
        }
    }
}

/// Resolves one `implements Name<...>` entry to the interface it names and
/// the concrete type arguments it fixes — `rule:iteration/concrete-generic-implements`'s one narrow
/// extension, in the one position that extension exists for.
///
/// Deliberately *not* [`lower_type`] over a synthesized name atom, for one
/// reason: an `implements` entry naming something undeclared is already
/// `nvs_hir::hierarchy`'s diagnostic, and routing through the type lowerer
/// would report it a second time. What is checked here is only what the
/// hierarchy resolver cannot see — that the name takes type arguments at all,
/// and that it was given the right number.
pub(crate) fn lower_implemented_interface(
    clause: &ImplementsClause,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (nvs_hir::QName, Vec<TypeId>) {
    let text = span_text(env.src, clause.name.span);
    let qname = nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports);
    let args: Vec<TypeId> = clause
        .type_args
        .iter()
        .map(|arg| lower_type(arg, ctx, env))
        .collect();

    if let Some(params) = generic_params(&qname) {
        let id = lower_generic_interface(qname.clone(), params, args, clause.span, env);
        // Read back rather than reused: a wrong count is recovered as the
        // no-arguments shape, and this record must agree with the type that
        // was actually interned.
        let args = match env.interner.get(id) {
            crate::ty::Ty::Class(_, args) => args.clone(),
            _ => Vec::new(),
        };
        return (qname, args);
    }
    if !args.is_empty() {
        report_not_generic(&qname, clause.span, env);
    }
    (qname, Vec::new())
}

/// `E_TYPE_ARGS_NOT_GENERIC`, from the two positions a type-argument list can
/// be written in — a name in type position, and an `implements` entry.
fn report_not_generic(qname: &nvs_hir::QName, span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_ARGS_NOT_GENERIC,
            format!("`{qname}` takes no type arguments"),
        )
        .with_primary(span, "type arguments written here")
        .with_help(
            "user-declared type parameters are deferred (`rule:types/declaration`); only a \
             compiler-owned generic declaration may be written with one — \
             `Iterable<T>`/`Iterator<T>` (`rule:iteration/concrete-generic-implements`) and \
             `docs/spec/01-core-library.md` § 9's `Core` collections",
        ),
    );
}

/// `qname`'s declared type parameters when it is a compiler-owned *generic*
/// declaration — `None` for every other name, including the non-generic
/// reserved interfaces, which take the ordinary path below.
///
/// Two rosters, because `rule:types/grammar` makes "which name may carry a list" a
/// resolution question and there are two kinds of compiler-owned answer.
/// [`nvs_stdlib::registry::GENERIC_CLASSES`] holds spec § 9's collections,
/// named in full because `Core\ObjectSet` is the only spelling there is.
/// [`nvs_hir::interfaces::RESERVED`] holds `rule:iteration/concrete-generic-implements`'s two interfaces,
/// named by their short name and only as a single global segment — so a user
/// interface that happens to be called `Iterable` in its own namespace is not
/// one, which [`nvs_hir::QName::is_reserved_global_interface`] already
/// requires.
fn generic_params(qname: &nvs_hir::QName) -> Option<&'static [&'static str]> {
    if let Some(params) = nvs_stdlib::registry::class_type_params(&qname.to_string()) {
        return Some(params);
    }
    if !qname.is_reserved_global_interface() {
        return None;
    }
    nvs_hir::interfaces::type_params(qname.short_name()).filter(|params| !params.is_empty())
}

/// Checks a compiler-owned generic interface's type-argument count and
/// interns `Name<args...>`.
///
/// A wrong count is reported and then *recovered from* by interning the name
/// with no arguments at all, rather than with a padded or truncated list: an
/// argument the author did not write has no honest value, and `Iterator` with
/// an empty list is already the shape every non-generic name has, so nothing
/// downstream meets a case it has no rule for.
fn lower_generic_interface(
    qname: nvs_hir::QName,
    params: &'static [&'static str],
    args: Vec<TypeId>,
    span: Span,
    env: &mut Env<'_>,
) -> TypeId {
    if args.len() == params.len() {
        return env.interner.generic_class(qname, args);
    }
    let expected = params.len();
    let got = args.len();
    let names = params.join(", ");
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_ARG_COUNT,
            format!("`{qname}` takes {expected} type argument(s), not {got}"),
        )
        .with_primary(span, format!("write `{qname}<{names}>`"))
        .with_help(
            if nvs_stdlib::registry::class_type_params(&qname.to_string()).is_some() {
                format!(
                    "`docs/spec/01-core-library.md` § 9 declares `{qname}<{names}>`; the arguments \
                 are positional, and nothing about a `Core` collection infers them"
                )
            } else {
                format!(
                    "`rule:iteration/two-interfaces` declares `{qname}<{names}>`; the argument fixes what it iterates \
                 over, and there is no spelling that leaves it open"
                )
            },
        ),
    );
    env.interner.class(qname)
}

fn resolve_name_type(
    name: &Name,
    args: &[Type],
    span: Span,
    depth: u32,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let text = span_text(env.src, name.span);
    let qname = nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports);

    // Lowered up front, and unconditionally: an argument written on a name
    // that turns out not to be generic is still a type the author wrote, and
    // a mistake inside it deserves its own diagnostic rather than being
    // swallowed by the outer refusal.
    let args: Vec<TypeId> = args
        .iter()
        .map(|arg| lower_type_at_depth(arg, depth + 1, ctx, env))
        .collect();

    if let Some(params) = generic_params(&qname) {
        return lower_generic_interface(qname, params, args, span, env);
    }
    if !args.is_empty() {
        report_not_generic(&qname, span, env);
    }

    // `rule:types/type-alias`'s short spelling: inside its owner's own body a
    // bare `Name` means that body's own alias before it means the namespace's,
    // which is the one `Order::Meta` reaches from anywhere. Tried in that order
    // here because `nvs_hir::aliases` already resolves it in that order inside
    // another alias's expansion, and one name cannot mean two things depending
    // on which layer read it.
    if let Some(owner) = ctx.current_class
        && let Some(alias_ty) = env.aliases.get_member(owner, text)
    {
        let alias_ty = alias_ty.clone();
        return lower_type_at_depth(&alias_ty, depth, ctx, env);
    }

    if let Some(alias_ty) = env.aliases.get(&qname) {
        let alias_ty = alias_ty.clone();
        return lower_type_at_depth(&alias_ty, depth, ctx, env);
    }

    match env.symbols.get(&qname) {
        Some(sym) if sym.kind == SymbolKind::Enum => {
            let backing = env.enums.backing_of(&qname);
            env.interner.enum_(qname, backing)
        }
        Some(_) => env.interner.class(qname),
        // A `Core`-owned enum is in no symbol table — nothing declared it —
        // but `crate::enums` seeded it exactly as `lower_member_type` above
        // reads it back for the `Core\Digest::Sha1` spelling. Without this
        // arm a written `Core\Digest` annotation interned as `Ty::Class`,
        // which no `Core` signature's `Ty::Enum` unified with: the two print
        // the same qname, so the mismatch arrived as "expected `Core\Digest`,
        // found `Core\Digest`" and a digest could not be passed through a
        // parameter at all.
        None if qname.is_core() && env.enums.get(&qname).is_some() => {
            let backing = env.enums.backing_of(&qname);
            env.interner.enum_(qname, backing)
        }
        None if qname.is_core()
            || qname.is_reserved_global_class()
            || qname.is_reserved_global_interface() =>
        {
            env.interner.class(qname)
        }
        None => {
            env.diags.report(nvs_hir::undeclared_name(
                nvs_hir::Undeclared {
                    qname: &qname,
                    text,
                    span: name.span,
                    namespace: ctx.namespace,
                    stmts: env.stmts,
                    src: env.src,
                },
                env.symbols,
                &nvs_stdlib::registry::type_names(),
            ));
            env.interner.mixed()
        }
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap, code};
    use nvs_hir::resolve_file;
    use nvs_syntax::ast::{StmtKind, Type};
    use nvs_syntax::parse_file;

    use super::*;
    use crate::ty::{Ty, TypeInterner};

    /// Parses `src` (whose sole top-level statement must be a `type Probe =
    /// ...;` alias declaration), resolves it, and lowers that alias's own
    /// expansion — a convenient way to get a `Type` AST node plus a fully
    /// resolved `Module` to lower it against, without hand-building a
    /// method body.
    fn lower_alias(src: &str) -> (TypeId, TypeInterner, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);

        let probe_ty: Type = stmts
            .iter()
            .find_map(|s| match &s.kind {
                StmtKind::TypeAliasDecl(decl)
                    if span_text(map.file(file), decl.name.span) == "Probe" =>
                {
                    Some(decl.ty.clone())
                }
                _ => None,
            })
            .expect("fixture must declare `type Probe = ...;`");

        let mut interner = TypeInterner::new();
        let empty_imports = rustc_hash::FxHashMap::default();
        let ctx = Ctx {
            namespace: &[],
            imports: &empty_imports,
            current_class: None,
            current_hook: None,
            generator_elem: None,
            in_constructor: false,
            in_closure: false,
        };
        let signatures = crate::signatures::SignatureTable::new();
        let mut exprs = crate::expr_table::ExprTypeTable::new();
        let files = [crate::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }];
        let enums = crate::enums::build_enum_table(&files, &mut diags);
        let consts = crate::consts::build_const_table(&files);
        let attributes = crate::retrieval::AttributeTable::default();
        let mut env = Env {
            symbols: &module.symbols,
            aliases: &module.aliases,
            graph: &module.graph,
            signatures: &signatures,
            enums: &enums,
            consts: &consts,
            attributes: &attributes,
            grants: None,
            src: map.file(file),
            stmts: &stmts,
            interner: &mut interner,
            exprs: &mut exprs,
            routes: &mut crate::routes::RouteTable::default(),
            commands: &mut crate::commands::CommandTable::default(),
            links: &mut Vec::new(),
            codec_sites: &mut Vec::new(),
            row_sites: &mut Vec::new(),
            json_sites: &mut Vec::new(),
            decode_sites: &mut Vec::new(),
            diags: &mut diags,
            closure_seq: 0,
            fn_self: None,
            exit_targets: Vec::new(),
            write_target_levels: rustc_hash::FxHashMap::default(),
            coalesce_guarded: rustc_hash::FxHashSet::default(),
            method_ref_args: rustc_hash::FxHashSet::default(),
            body_writers: crate::response::BodyWriters::default(),
            in_call_argument: false,
        };
        let id = lower_type(&probe_ty, &ctx, &mut env);
        (id, interner, diags)
    }

    #[test]
    fn scalars_lower_to_their_own_singleton() {
        let (id, interner, diags) = lower_alias("<?nvs\ntype Probe = uint;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(*interner.get(id), Ty::Uint);
    }

    #[test]
    fn nullable_lowers_to_a_union_with_null() {
        let (id, interner, diags) = lower_alias("<?nvs\nclass Foo {}\ntype Probe = ?Foo;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(interner.describe(id).split('|').count(), 2);
    }

    #[test]
    fn a_class_name_resolves_to_a_class_type() {
        // A bare `type Probe = Foo;` is itself rejected by `rule:types/alias-is-never-a-bare-class` (a
        // type alias aliasing a single bare class), so this wraps it in
        // `array<...>` to exercise class-name resolution instead.
        let (id, interner, diags) = lower_alias("<?nvs\nclass Foo {}\ntype Probe = array<Foo>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let Ty::Array(elem) = interner.get(id) else {
            panic!("expected array<...>, got {:?}", interner.get(id));
        };
        assert!(matches!(interner.get(*elem), Ty::Class(q, _) if q.to_string() == "Foo"));
    }

    #[test]
    fn a_core_enum_name_resolves_to_an_enum_type_not_a_class() {
        // Nothing declares `Core\Digest`, so the symbol table has no entry for
        // it and the `is_core()` arm below used to intern a `Ty::Class` — which
        // no registry row's `Ty::Enum` unified with, though both print the same
        // qname. `array<...>` for the same `rule:types/alias-is-never-a-bare-class` reason as above.
        let (id, interner, diags) = lower_alias("<?nvs\ntype Probe = array<Core\\Digest>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let Ty::Array(elem) = interner.get(id) else {
            panic!("expected array<...>, got {:?}", interner.get(id));
        };
        assert!(
            matches!(interner.get(*elem), Ty::Enum(q, _) if q.to_string() == r"Core\Digest"),
            "expected an enum type, got {:?}",
            interner.get(*elem)
        );
    }

    #[test]
    fn a_reserved_global_exception_class_resolves_with_no_declaration() {
        // `rule:errors/throwable-hierarchy`: `Exception` never needs a source declaration —
        // trusted the same way a `Core\*` name is.
        let (id, interner, diags) = lower_alias("<?nvs\ntype Probe = array<LogicError>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let Ty::Array(elem) = interner.get(id) else {
            panic!("expected array<...>, got {:?}", interner.get(id));
        };
        assert!(matches!(interner.get(*elem), Ty::Class(q, _) if q.to_string() == "LogicError"));
    }

    #[test]
    fn an_enum_name_resolves_to_an_enum_type() {
        let (id, interner, diags) =
            lower_alias("<?nvs\nenum Status: uint { Active }\ntype Probe = array<Status>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let Ty::Array(elem) = interner.get(id) else {
            panic!("expected array<...>, got {:?}", interner.get(id));
        };
        assert!(matches!(interner.get(*elem), Ty::Enum(q, _) if q.to_string() == "Status"));
    }

    #[test]
    fn an_undeclared_name_is_diagnosed() {
        let (_id, _interner, diags) = lower_alias("<?nvs\ntype Probe = Missing;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn a_type_alias_is_substituted_recursively() {
        let (id, interner, diags) =
            lower_alias("<?nvs\ntype Inner = uint;\ntype Probe = array<Inner>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(interner.describe(id), "array<uint>");
    }

    #[test]
    fn array_nesting_past_32_deep_is_diagnosed() {
        let mut src = "<?nvs\ntype Probe = ".to_owned();
        for _ in 0..40 {
            src.push_str("array<");
        }
        src.push_str("int");
        for _ in 0..40 {
            src.push('>');
        }
        src.push_str(";\n");
        let (_id, _interner, diags) = lower_alias(&src);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ARRAY_TYPE_TOO_DEEP))
        );
    }
}
