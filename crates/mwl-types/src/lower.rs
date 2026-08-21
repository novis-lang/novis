//! Resolves a parsed [`Type`] into an interned [`TypeId`] (ADR 0007 § 3's
//! grammar, made concrete).
//!
//! A `self`/`static` atom resolves against [`Ctx::current_class`]; `parent`
//! is not resolved at all in this slice (see the crate docs' known gaps) and
//! always produces `mixed` plus a diagnostic, since resolving it needs a
//! [`mwl_hir::ClassGraph`] hop this slice's [`Ctx`] does not carry. A `Name`
//! atom resolves via [`mwl_hir::resolve_ref`] — the same
//! unqualified/qualified/fully-qualified resolution every `mwl-hir` resolver
//! already shares — then checks [`mwl_hir::AliasTable`] first (an alias is
//! "resolved eagerly," per ADR 0015 § 5, so its expansion is substituted
//! in and lowered recursively rather than kept as a name), falling back to
//! [`mwl_hir::SymbolTable`] to decide between a class-shaped atom (a class,
//! interface or trait — the type grammar does not distinguish them) and an
//! enum. A name that resolves to neither, and is not trusted as a `Core`
//! reference, is `E_UNDEFINED_CLASS`.
//!
//! `array<...>` nesting is bounded at depth 32 (ADR 0007 § 5) — past that,
//! lowering stops and reports `E_ARRAY_TYPE_TOO_DEEP` rather than recursing
//! further, so a pathological type cannot make lowering superlinear.

use mwl_diagnostics::{Diagnostic, Span, code};
use mwl_hir::SymbolKind;
use mwl_syntax::ast::{Name, Type, TypeAtom, TypeKind};

use crate::ty::TypeId;
use crate::{Ctx, Env, span_text};

const MAX_ARRAY_DEPTH: u32 = 32;

/// Lowers `ty` into an interned [`TypeId`], within the given class/namespace
/// scope.
pub(crate) fn lower_type(ty: &Type, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    lower_type_at_depth(ty, 0, ctx, env)
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
        TypeAtom::String => env.interner.string(),
        TypeAtom::Bytes => env.interner.bytes(),
        TypeAtom::TaintedString => env.interner.tainted_string(),
        TypeAtom::TaintedBytes => env.interner.tainted_bytes(),
        TypeAtom::Array(inner) => {
            let elem = match inner {
                Some(t) => lower_type_at_depth(t, depth + 1, ctx, env),
                None => env.interner.mixed(),
            };
            env.interner.array(elem)
        }
        TypeAtom::Object => env.interner.object(),
        TypeAtom::Mixed => env.interner.mixed(),
        TypeAtom::Void => env.interner.void(),
        TypeAtom::Never => env.interner.never(),
        TypeAtom::True => env.interner.true_ty(),
        TypeAtom::False => env.interner.false_ty(),
        TypeAtom::Iterable => env.interner.iterable(),
        TypeAtom::Callable => env.interner.callable(),
        TypeAtom::SelfTy => resolve_special(span, "self", ctx, env),
        TypeAtom::StaticTy => resolve_special(span, "static", ctx, env),
        TypeAtom::Parent => {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_CLASS,
                    "`parent` is not resolved by this checker slice yet — it needs the class \
                     hierarchy graph",
                )
                .with_primary(span, "`parent` type"),
            );
            env.interner.mixed()
        }
        TypeAtom::Name(name) => resolve_name_type(name, depth, ctx, env),
        _ => env.interner.mixed(),
    }
}

fn resolve_special(span: Span, keyword: &str, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    match ctx.current_class {
        Some(qname) => env.interner.class(qname.clone()),
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

fn resolve_name_type(name: &Name, depth: u32, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    let text = span_text(env.src, name.span);
    let qname = mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports);

    if let Some(alias_ty) = env.aliases.get(&qname) {
        let alias_ty = alias_ty.clone();
        return lower_type_at_depth(&alias_ty, depth, ctx, env);
    }

    match env.symbols.get(&qname) {
        Some(sym) if sym.kind == SymbolKind::Enum => env.interner.enum_(qname),
        Some(_) => env.interner.class(qname),
        None if qname.is_core() => env.interner.class(qname),
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
        };
        let mut env = Env {
            symbols: &module.symbols,
            aliases: &module.aliases,
            src: map.file(file),
            interner: &mut interner,
            diags: &mut diags,
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
        assert!(matches!(interner.get(*elem), Ty::Class(q) if q.to_string() == "Foo"));
    }

    #[test]
    fn an_enum_name_resolves_to_an_enum_type() {
        let (id, interner, diags) =
            lower_alias("<?mwl\nenum Status: uint { Active }\ntype Probe = array<Status>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let Ty::Array(elem) = interner.get(id) else {
            panic!("expected array<...>, got {:?}", interner.get(id));
        };
        assert!(matches!(interner.get(*elem), Ty::Enum(q) if q.to_string() == "Status"));
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
