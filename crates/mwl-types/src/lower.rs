//! Resolves a parsed [`Type`] into an interned [`TypeId`] (ADR 0007 § 3's
//! grammar, made concrete).
//!
//! A `self`/`static` atom resolves against [`Ctx::current_class`]; `parent`
//! resolves the same way `crate::expr::resolve_class_expr`'s `ParentExpr` arm
//! and `check_new_target`'s `NewTarget::ParentTy` arm do — [`Ctx::current_class`]'s
//! first `extends` link via [`Env::graph`] — except a `parent` that can't
//! resolve (no enclosing class, or a class with no `extends`) is a
//! diagnostic here rather than those two's silent `mixed` fallback, since a
//! *type* position naming an unresolvable class is a real authoring mistake
//! the same way an out-of-class `self`/`static` already is. A `Name`
//! atom resolves via [`mwl_hir::resolve_ref`] — the same
//! unqualified/qualified/fully-qualified resolution every `mwl-hir` resolver
//! already shares — then checks [`mwl_hir::AliasTable`] first (an alias is
//! "resolved eagerly," per ADR 0015 § 5, so its expansion is substituted
//! in and lowered recursively rather than kept as a name), falling back to
//! [`mwl_hir::SymbolTable`] to decide between a class-shaped atom (a class or
//! interface — the type grammar does not distinguish them) and an enum. A
//! name that resolves to neither, and is not trusted as a `Core`
//! reference or one of [`mwl_hir::errors`]' exception classes
//! ([`mwl_hir::QName::is_reserved_global_class`]), is `E_UNDEFINED_CLASS`.
//!
//! `array<...>` nesting is bounded at depth 32 (ADR 0007 § 5) — past that,
//! lowering stops and reports `E_ARRAY_TYPE_TOO_DEEP` rather than recursing
//! further, so a pathological type cannot make lowering superlinear.

use mwl_diagnostics::{Diagnostic, Span, code};
use mwl_hir::SymbolKind;
use mwl_syntax::ast::{ImplementsClause, Name, Type, TypeAtom, TypeKind};

use crate::ty::TypeId;
use crate::{Ctx, Env, span_text};

const MAX_ARRAY_DEPTH: u32 = 32;

/// Lowers `ty` into an interned [`TypeId`], within the given class/namespace
/// scope.
pub(crate) fn lower_type(ty: &Type, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    let id = lower_type_at_depth(ty, 0, ctx, env);
    // Persisted for `mwl-ir`, which lowers a declared type off the AST and so
    // cannot resolve a name-shaped atom for itself — see
    // `crate::expr_table::ExprTypeTable::declared_ty`. Recorded here, at the
    // one entry point every annotation goes through, rather than at each of
    // this function's callers.
    env.exprs.record_type(ty.span, id);
    id
}

/// Lowers an optional declared type, e.g. a `foreach` binding or destructure
/// leaf that omitted its type (already diagnosed elsewhere, per
/// [`mwl_syntax::ast::ForeachBinding::ty`]'s own doc), defaulting to `mixed`.
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
        TypeAtom::Object => env.interner.object(),
        TypeAtom::Shape(fields) => {
            let mut out = Vec::with_capacity(fields.len());
            for field in fields {
                let name = span_text(env.src, field.name).to_owned();
                let field_ty = lower_type_at_depth(&field.ty, depth + 1, ctx, env);
                out.push((name, field_ty));
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
        TypeAtom::SelfTy => resolve_special(span, "self", ctx, env),
        TypeAtom::StaticTy => resolve_special(span, "static", ctx, env),
        TypeAtom::Parent => resolve_parent(span, ctx, env),
        TypeAtom::Name(name, args) => resolve_name_type(name, args, span, depth, ctx, env),
        TypeAtom::StringLiteral(_) | TypeAtom::IntLiteral(_) | TypeAtom::Member(..) => {
            reject_unchecked_literal_type(atom, span, env)
        }
        _ => env.interner.mixed(),
    }
}

/// ADR 0047's three atoms parse (M1) and are not yet checked (M2) — see
/// [`code::E_LITERAL_TYPE_UNCHECKED`], which owns why refusing is the only
/// honest answer in between.
fn reject_unchecked_literal_type(atom: &TypeAtom, span: Span, env: &mut Env<'_>) -> TypeId {
    let what = match atom {
        TypeAtom::StringLiteral(_) | TypeAtom::IntLiteral(_) => "a literal type",
        _ => "a class-constant or enum-case type",
    };
    env.diags.report(
        Diagnostic::error(
            code::E_LITERAL_TYPE_UNCHECKED,
            format!("{what} is not checked yet"),
        )
        .with_primary(span, "parses, but the checker has no rule for it")
        .with_help(
            "declare the base type (`string`, `int`, or the enum) for now — ADR 0047 § 4's \
             assignability and conversion table is M2's slice",
        ),
    );
    env.interner.mixed()
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
/// `extends` link — the same hop `crate::expr::resolve_class_expr` uses for
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
/// the concrete type arguments it fixes — ADR 0053 § 2's one narrow
/// extension, in the one position that extension exists for.
///
/// Deliberately *not* [`lower_type`] over a synthesized name atom, for one
/// reason: an `implements` entry naming something undeclared is already
/// `mwl_hir::hierarchy`'s diagnostic, and routing through the type lowerer
/// would report it a second time. What is checked here is only what the
/// hierarchy resolver cannot see — that the name takes type arguments at all,
/// and that it was given the right number.
pub(crate) fn lower_implemented_interface(
    clause: &ImplementsClause,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> (mwl_hir::QName, Vec<TypeId>) {
    let text = span_text(env.src, clause.name.span);
    let qname = mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports);
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
fn report_not_generic(qname: &mwl_hir::QName, span: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_TYPE_ARGS_NOT_GENERIC,
            format!("`{qname}` takes no type arguments"),
        )
        .with_primary(span, "type arguments written here")
        .with_help(
            "user-declared type parameters are deferred (ADR 0007 § 1); only the \
             compiler-owned `Iterable<T>`/`Iterator<T>` may be written with one \
             (ADR 0053 § 2)",
        ),
    );
}

/// `qname`'s declared type parameters when it is a compiler-owned *generic*
/// interface — `None` for every other name, including the non-generic
/// reserved interfaces, which take the ordinary path below.
///
/// The roster is [`mwl_hir::interfaces::RESERVED`] and nothing else: ADR
/// 0053 § 2's extension is to *compiler-owned* declarations, so a user
/// interface that happens to be named `Iterable` in its own namespace is not
/// one (the name must be a single global segment, which
/// [`mwl_hir::QName::is_reserved_global_interface`] already requires).
fn generic_params(qname: &mwl_hir::QName) -> Option<&'static [&'static str]> {
    if !qname.is_reserved_global_interface() {
        return None;
    }
    mwl_hir::interfaces::type_params(qname.short_name()).filter(|params| !params.is_empty())
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
    qname: mwl_hir::QName,
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
        .with_help(format!(
            "ADR 0053 § 1 declares `{qname}<{names}>`; the argument fixes what it iterates over, \
             and there is no spelling that leaves it open"
        )),
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
    let qname = mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports);

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
        None if qname.is_core()
            || qname.is_reserved_global_class()
            || qname.is_reserved_global_interface() =>
        {
            env.interner.class(qname)
        }
        None => {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_CLASS,
                    format!("`{qname}` is not declared"),
                )
                .with_primary(name.span, "no matching declaration"),
            );
            env.interner.mixed()
        }
    }
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::{Diagnostics, SourceMap, code};
    use mwl_hir::resolve_file;
    use mwl_syntax::ast::{StmtKind, Type};
    use mwl_syntax::parse_file;

    use super::*;
    use crate::ty::{Ty, TypeInterner};

    /// Parses `src` (whose sole top-level statement must be a `type Probe =
    /// ...;` alias declaration), resolves it, and lowers that alias's own
    /// expansion — a convenient way to get a `Type` AST node plus a fully
    /// resolved `Module` to lower it against, without hand-building a
    /// method body.
    fn lower_alias(src: &str) -> (TypeId, TypeInterner, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
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
        };
        let signatures = crate::signatures::SignatureTable::new();
        let mut exprs = crate::expr_table::ExprTypeTable::new();
        let enums = crate::enums::build_enum_table(&stmts, map.file(file), &mut diags);
        let mut env = Env {
            symbols: &module.symbols,
            aliases: &module.aliases,
            graph: &module.graph,
            signatures: &signatures,
            enums: &enums,
            src: map.file(file),
            interner: &mut interner,
            exprs: &mut exprs,
            diags: &mut diags,
            closure_seq: 0,
        };
        let id = lower_type(&probe_ty, &ctx, &mut env);
        (id, interner, diags)
    }

    #[test]
    fn scalars_lower_to_their_own_singleton() {
        let (id, interner, diags) = lower_alias("<?mwl\ntype Probe = uint;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(*interner.get(id), Ty::Uint);
    }

    #[test]
    fn nullable_lowers_to_a_union_with_null() {
        let (id, interner, diags) = lower_alias("<?mwl\nclass Foo {}\ntype Probe = ?Foo;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(interner.describe(id).split('|').count(), 2);
    }

    #[test]
    fn a_class_name_resolves_to_a_class_type() {
        // A bare `type Probe = Foo;` is itself rejected by ADR 0015 § 6 (a
        // type alias aliasing a single bare class), so this wraps it in
        // `array<...>` to exercise class-name resolution instead.
        let (id, interner, diags) = lower_alias("<?mwl\nclass Foo {}\ntype Probe = array<Foo>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let Ty::Array(elem) = interner.get(id) else {
            panic!("expected array<...>, got {:?}", interner.get(id));
        };
        assert!(matches!(interner.get(*elem), Ty::Class(q, _) if q.to_string() == "Foo"));
    }

    #[test]
    fn a_reserved_global_exception_class_resolves_with_no_declaration() {
        // ADR 0020 § 0: `Exception` never needs a source declaration —
        // trusted the same way a `Core\*` name is.
        let (id, interner, diags) = lower_alias("<?mwl\ntype Probe = array<LogicError>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let Ty::Array(elem) = interner.get(id) else {
            panic!("expected array<...>, got {:?}", interner.get(id));
        };
        assert!(matches!(interner.get(*elem), Ty::Class(q, _) if q.to_string() == "LogicError"));
    }

    #[test]
    fn an_enum_name_resolves_to_an_enum_type() {
        let (id, interner, diags) =
            lower_alias("<?mwl\nenum Status: uint { Active }\ntype Probe = array<Status>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let Ty::Array(elem) = interner.get(id) else {
            panic!("expected array<...>, got {:?}", interner.get(id));
        };
        assert!(matches!(interner.get(*elem), Ty::Enum(q, _) if q.to_string() == "Status"));
    }

    #[test]
    fn an_undeclared_name_is_diagnosed() {
        let (_id, _interner, diags) = lower_alias("<?mwl\ntype Probe = Missing;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn a_type_alias_is_substituted_recursively() {
        let (id, interner, diags) =
            lower_alias("<?mwl\ntype Inner = uint;\ntype Probe = array<Inner>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(interner.describe(id), "array<uint>");
    }

    #[test]
    fn array_nesting_past_32_deep_is_diagnosed() {
        let mut src = "<?mwl\ntype Probe = ".to_owned();
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
