//! Type alias substitution — the remaining half of M2 item 3. See the
//! crate's module docs for where this sits relative to the rest of M2.
//!
//! [`crate::resolve::Resolver`] already collects each `type` alias's own
//! declaration into the [`crate::symbol::SymbolTable`] and rejects the
//! single-bare-class shape (`rule:statements/nothing-gets-a-second-name`
//! § 6). What is still open is *what an alias expands to*: § 5 says a `type`
//! alias is "fully transparent" — every occurrence of its name, including
//! inside another alias's own expansion, resolves to the same fully-expanded
//! [`Type`] before anything downstream ever sees the alias's name at all.
//! This module builds that expansion.
//!
//! Same two-pass shape as [`crate::hierarchy::HierarchyResolver`]:
//! [`AliasResolver::collect_aliases`] walks a file's `type` declarations,
//! recording each one's raw expansion together with the namespace/`use`
//! imports active where it was declared — needed to resolve an unqualified
//! name inside it the same way [`crate::hierarchy::resolve_ref`] already does
//! for `extends`/`implements`. [`AliasResolver::resolve`] then substitutes
//! every alias's expansion in one closing pass: an atom that resolves to
//! another alias is replaced by that alias's own (already-substituted)
//! expansion, recursively and memoized so a name referenced from several
//! places is only computed once; an atom that resolves to a class,
//! interface or enum — or isn't a name atom at all, like a scalar or
//! `array<...>`'s own shape — is left exactly as written.
//!
//! A cycle (`type A = B; type B = A;`, or any longer chain) is diagnosed
//! (`E_TYPE_ALIAS_CYCLE`, `rule:statements/nothing-gets-a-second-name`
//! § 5/7) rather than looped forever or silently bottomed out at `mixed`;
//! every alias name that took part in the cycle still gets an entry in the
//! resulting [`AliasTable`], expanding to `mixed`, so a lookup miss keeps
//! meaning "not an alias" rather than colliding with "an alias that turned
//! out to be broken."
//!
//! **Known gap:** an atom that resolves to nothing declared at all — not a
//! class, not an alias, not `Core` — is not diagnosed here. Whether a name
//! names *something* real is a general type-atom question the type checker
//! (`nvs-types`, not yet started) owns; this module only concerns itself with
//! the alias-substitution question ADR 0015 § 5 asks, the same narrowing
//! [`crate::members`] already applies to `Class::member` references.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_syntax::ast::{NamespaceDecl, Stmt, StmtKind, Type, TypeAtom, TypeKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::hierarchy::resolve_ref;
use crate::qname::QName;
use crate::resolve::{name_text, qname_segments};

/// Every `type` alias's fully-substituted expansion, keyed by its [`QName`].
#[derive(Debug, Default)]
pub struct AliasTable {
    by_name: FxHashMap<QName, Type>,
}

impl AliasTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The fully-substituted expansion for one declared alias, if this name
    /// is one.
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&Type> {
        self.by_name.get(qname)
    }

    /// How many aliases were resolved.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether nothing was resolved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// One `type` alias's raw expansion, plus the namespace/imports active where
/// it was declared.
#[derive(Debug)]
struct PendingAlias {
    ty: Type,
    decl_span: Span,
    namespace: Vec<String>,
    imports: FxHashMap<String, QName>,
}

/// Collects a [`AliasTable`] from one or more files' `type` alias
/// declarations, then substitutes every alias's expansion in one closing
/// call. Two calls, same shape as [`crate::hierarchy::HierarchyResolver`],
/// for the same reason: an alias may reference one declared in a file
/// collected later.
#[derive(Debug, Default)]
pub struct AliasResolver {
    pending: FxHashMap<QName, PendingAlias>,
    /// Declaration order, so the top-level resolve loop (and any diagnostics
    /// it produces) doesn't depend on hash-map iteration order.
    order: Vec<QName>,
    /// Every type-atom `Name`'s extracted source text, by its span — recorded
    /// at collection time so resolution needs no [`SourceFile`] afterward,
    /// the same trick [`crate::hierarchy::HierarchyResolver`] uses for a
    /// single name, generalised to a whole type tree.
    names: FxHashMap<Span, String>,
}

impl AliasResolver {
    /// A resolver with nothing collected yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Walks `stmts`, recording every `type` alias's raw expansion. Recurses
    /// into `namespace { ... }` blocks; a `namespace Name;` statement changes
    /// the namespace and resets the tracked imports for the rest of this
    /// call's statement sequence, matching
    /// [`crate::resolve::Resolver::collect_declarations`].
    pub fn collect_aliases(&mut self, stmts: &[Stmt], src: &SourceFile) {
        self.collect_in(stmts, src, &[]);
    }

    fn collect_in(&mut self, stmts: &[Stmt], src: &SourceFile, namespace: &[String]) {
        let mut current_ns: Vec<String> = namespace.to_vec();
        let mut imports: FxHashMap<String, QName> = FxHashMap::default();

        for stmt in stmts {
            match &stmt.kind {
                StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                    let new_ns = name
                        .as_ref()
                        .map_or_else(Vec::new, |n| qname_segments(src, n));
                    match body {
                        Some(block) => self.collect_in(&block.stmts, src, &new_ns),
                        None => {
                            current_ns = new_ns;
                            imports.clear();
                        }
                    }
                }
                StmtKind::UseDecl(use_decl) => {
                    let target = QName::parse(name_text(src, &use_decl.path));
                    imports.insert(target.short_name().to_owned(), target);
                }
                StmtKind::TypeAliasDecl(decl) => {
                    record_names(&decl.ty, src, &mut self.names);
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        self.pending.entry(qname.clone())
                    {
                        entry.insert(PendingAlias {
                            ty: decl.ty.clone(),
                            decl_span: decl.name.span,
                            namespace: current_ns.clone(),
                            imports: imports.clone(),
                        });
                        self.order.push(qname);
                    }
                    // A second declaration of the same name is already
                    // diagnosed as `E_DUPLICATE_DECLARATION` by the symbol
                    // table; the first one wins here too, for consistency.
                }
                _ => {}
            }
        }
    }

    /// Substitutes every collected alias's expansion, reporting
    /// `E_TYPE_ALIAS_CYCLE` for a name that, directly or through some chain,
    /// expands back to itself. Call once, after every file sharing this
    /// resolver has run [`Self::collect_aliases`].
    #[must_use]
    pub fn resolve(self, diags: &mut Diagnostics) -> AliasTable {
        let mut ctx = ResolveCtx {
            pending: &self.pending,
            names: &self.names,
            resolved: FxHashMap::default(),
            errored: FxHashSet::default(),
            stack: Vec::new(),
            diags,
        };
        for qname in &self.order {
            resolve_one(qname, &mut ctx);
        }
        AliasTable {
            by_name: ctx.resolved,
        }
    }
}

/// Walks a type expression looking for `TypeAtom::Name` leaves, recording
/// each one's source text by its span.
fn record_names(ty: &Type, src: &SourceFile, out: &mut FxHashMap<Span, String>) {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => record_names(inner, src, out),
        TypeKind::Union(items) | TypeKind::Intersection(items) => {
            for item in items {
                record_names(item, src, out);
            }
        }
        TypeKind::Atom(TypeAtom::Array(Some(inner)) | TypeAtom::ClassRef(inner)) => {
            record_names(inner, src, out);
        }
        TypeKind::Atom(TypeAtom::Name(name, args)) => {
            out.insert(name.span, name_text(src, name).to_owned());
            for arg in args {
                record_names(arg, src, out);
            }
        }
        _ => {}
    }
}

/// The read-only tables, the in-progress memo/cycle state, and the
/// diagnostics sink [`resolve_one`]/[`substitute`] need, threaded through one
/// argument instead of several.
struct ResolveCtx<'a> {
    pending: &'a FxHashMap<QName, PendingAlias>,
    names: &'a FxHashMap<Span, String>,
    resolved: FxHashMap<QName, Type>,
    /// Every name that took part in a detected cycle — resolves to `mixed`
    /// from here on, without being recomputed or re-diagnosed.
    errored: FxHashSet<QName>,
    /// The chain of aliases currently being expanded, root to leaf — a name
    /// already on this stack when reached again is the cycle.
    stack: Vec<QName>,
    diags: &'a mut Diagnostics,
}

fn mixed_at(span: Span) -> Type {
    Type {
        kind: TypeKind::Atom(TypeAtom::Mixed),
        span,
    }
}

fn resolve_one(qname: &QName, ctx: &mut ResolveCtx<'_>) -> Type {
    if let Some(ty) = ctx.resolved.get(qname) {
        return ty.clone();
    }
    let Some(pending) = ctx.pending.get(qname) else {
        unreachable!("resolve_one is only called for a name known to be a pending alias");
    };
    if ctx.errored.contains(qname) {
        return mark_mixed(qname, pending.decl_span, ctx);
    }
    if let Some(pos) = ctx.stack.iter().position(|q| q == qname) {
        let mut chain: Vec<String> = ctx.stack[pos..].iter().map(ToString::to_string).collect();
        chain.push(qname.to_string());
        ctx.diags.report(
            Diagnostic::error(
                code::E_TYPE_ALIAS_CYCLE,
                format!(
                    "type alias cycle: {} never bottoms out in a concrete type",
                    chain.join(" -> ")
                ),
            )
            .with_primary(pending.decl_span, "part of this cycle"),
        );
        // Every name in the cycle, `qname` included, resolves to `mixed` from
        // here on — none of them ever bottoms out in a concrete type, so
        // none of them can be resolved a second, more specific way either.
        let cycle_members: Vec<QName> = ctx.stack[pos..].to_vec();
        for cyclic in &cycle_members {
            let span = ctx
                .pending
                .get(cyclic)
                .map_or(pending.decl_span, |p| p.decl_span);
            mark_mixed(cyclic, span, ctx);
        }
        return mark_mixed(qname, pending.decl_span, ctx);
    }

    ctx.stack.push(qname.clone());
    let substituted = substitute(&pending.ty, &pending.namespace, &pending.imports, ctx);
    ctx.stack.pop();

    if ctx.errored.contains(qname) {
        return mark_mixed(qname, pending.decl_span, ctx);
    }
    ctx.resolved.insert(qname.clone(), substituted.clone());
    substituted
}

/// Records `qname` as part of a cycle and caches its expansion as `mixed`,
/// so a later lookup sees a real (if degenerate) entry rather than a miss —
/// a miss would otherwise be indistinguishable from "not an alias at all".
fn mark_mixed(qname: &QName, span: Span, ctx: &mut ResolveCtx<'_>) -> Type {
    ctx.errored.insert(qname.clone());
    let fallback = mixed_at(span);
    ctx.resolved.insert(qname.clone(), fallback.clone());
    fallback
}

/// Rebuilds `ty`, replacing any `TypeAtom::Name` atom that resolves (via
/// [`resolve_ref`], the same lookup [`crate::hierarchy`] uses for
/// `extends`/`implements`) to a pending alias with that alias's own
/// (recursively substituted) expansion. Everything else — a scalar, `self`/
/// `static`/`parent`, a name that resolves to a class/interface/enum instead
/// — is cloned as-is.
fn substitute(
    ty: &Type,
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    ctx: &mut ResolveCtx<'_>,
) -> Type {
    match &ty.kind {
        TypeKind::Nullable(inner) => Type {
            kind: TypeKind::Nullable(Box::new(substitute(inner, namespace, imports, ctx))),
            span: ty.span,
        },
        TypeKind::Union(items) => Type {
            kind: TypeKind::Union(
                items
                    .iter()
                    .map(|item| substitute(item, namespace, imports, ctx))
                    .collect(),
            ),
            span: ty.span,
        },
        TypeKind::Intersection(items) => Type {
            kind: TypeKind::Intersection(
                items
                    .iter()
                    .map(|item| substitute(item, namespace, imports, ctx))
                    .collect(),
            ),
            span: ty.span,
        },
        TypeKind::Paren(inner) => Type {
            kind: TypeKind::Paren(Box::new(substitute(inner, namespace, imports, ctx))),
            span: ty.span,
        },
        TypeKind::Atom(TypeAtom::Array(Some(inner))) => Type {
            kind: TypeKind::Atom(TypeAtom::Array(Some(Box::new(substitute(
                inner, namespace, imports, ctx,
            ))))),
            span: ty.span,
        },
        // ADR 0125 § 1's class reference, whose argument is a name like any
        // other argument position's: an alias standing for a class expands
        // inside it exactly as it does inside `array<T>`.
        TypeKind::Atom(TypeAtom::ClassRef(inner)) => Type {
            kind: TypeKind::Atom(TypeAtom::ClassRef(Box::new(substitute(
                inner, namespace, imports, ctx,
            )))),
            span: ty.span,
        },
        TypeKind::Atom(TypeAtom::Name(name, args)) => {
            let text = ctx.names.get(&name.span).map(String::as_str).unwrap_or("");
            let qname = resolve_ref(text, namespace, imports);
            // A name *written with* type arguments is never an alias
            // expansion site: `rule:statements/nothing-gets-a-second-name` keeps a `type` alias a synonym for a
            // whole type expression, with no parameters of its own, so
            // `Alias<int>` is an error the checker reports rather than
            // something to expand here. Its arguments still get substituted,
            // so an alias used *as* an argument still expands.
            if args.is_empty() && ctx.pending.contains_key(&qname) {
                resolve_one(&qname, ctx)
            } else if args.is_empty() {
                ty.clone()
            } else {
                Type {
                    kind: TypeKind::Atom(TypeAtom::Name(
                        *name,
                        args.iter()
                            .map(|arg| substitute(arg, namespace, imports, ctx))
                            .collect(),
                    )),
                    span: ty.span,
                }
            }
        }
        _ => ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::SourceMap;
    use nvs_syntax::parse_file;

    use super::*;

    fn resolve(src: &str) -> (AliasTable, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let mut resolver = AliasResolver::new();
        resolver.collect_aliases(&stmts, map.file(file));
        let table = resolver.resolve(&mut diags);
        (table, diags)
    }

    fn is_atom(ty: &Type, expected: &TypeAtom) -> bool {
        matches!(&ty.kind, TypeKind::Atom(atom) if atom == expected)
    }

    #[test]
    fn a_scalar_alias_expands_to_the_scalar() {
        let (table, diags) = resolve("<?nvs\ntype UserId = uint;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let ty = table.get(&QName::parse("UserId")).unwrap();
        assert!(is_atom(ty, &TypeAtom::Uint));
    }

    #[test]
    fn an_alias_of_an_alias_expands_all_the_way_through() {
        let (table, diags) = resolve("<?nvs\ntype Inner = uint;\ntype Outer = array<Inner>;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let outer = table.get(&QName::parse("Outer")).unwrap();
        let TypeKind::Atom(TypeAtom::Array(Some(inner))) = &outer.kind else {
            panic!("expected array<...>, got {outer:?}");
        };
        assert!(is_atom(inner, &TypeAtom::Uint));
    }

    #[test]
    fn a_union_alias_substitutes_every_member() {
        let (table, diags) =
            resolve("<?nvs\ntype Id = uint;\ntype Score = float;\ntype IdOrScore = Id|Score;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let ty = table.get(&QName::parse("IdOrScore")).unwrap();
        let TypeKind::Union(members) = &ty.kind else {
            panic!("expected a union, got {ty:?}");
        };
        assert!(is_atom(&members[0], &TypeAtom::Uint));
        assert!(is_atom(&members[1], &TypeAtom::Float));
    }

    #[test]
    fn a_class_shaped_alias_leaves_the_class_name_untouched() {
        let (table, diags) = resolve("<?nvs\nclass Foo {}\ntype MaybeFoo = ?Foo;\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let ty = table.get(&QName::parse("MaybeFoo")).unwrap();
        let TypeKind::Nullable(inner) = &ty.kind else {
            panic!("expected ?Foo, got {ty:?}");
        };
        assert!(matches!(&inner.kind, TypeKind::Atom(TypeAtom::Name(..))));
    }

    #[test]
    fn a_direct_two_alias_cycle_is_diagnosed() {
        let (_table, diags) = resolve("<?nvs\ntype A = B;\ntype B = A;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_ALIAS_CYCLE))
        );
    }

    #[test]
    fn a_self_referential_alias_is_a_cycle_of_one() {
        let (_table, diags) = resolve("<?nvs\ntype Loop = array<Loop>;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_ALIAS_CYCLE))
        );
    }

    #[test]
    fn a_cycle_still_produces_an_entry_that_expands_to_mixed() {
        let (table, _diags) = resolve("<?nvs\ntype A = B;\ntype B = A;\n");
        let ty = table.get(&QName::parse("A")).unwrap();
        assert!(is_atom(ty, &TypeAtom::Mixed));
    }

    #[test]
    fn an_unrelated_alias_still_resolves_when_another_one_cycles() {
        let (table, diags) = resolve("<?nvs\ntype A = B;\ntype B = A;\ntype Fine = uint;\n");
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == Some(code::E_TYPE_ALIAS_CYCLE))
                .count(),
            1,
            "one cycle, one diagnostic: {diags:?}"
        );
        let ty = table.get(&QName::parse("Fine")).unwrap();
        assert!(is_atom(ty, &TypeAtom::Uint));
    }

    #[test]
    fn a_use_import_resolves_an_unqualified_alias_reference() {
        let (table, diags) = resolve(
            "<?nvs\nnamespace App;\ntype Id = uint;\nnamespace App\\Http;\nuse App\\Id;\ntype Row = array<Id>;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let row = table.get(&QName::parse("App\\Http\\Row")).unwrap();
        let TypeKind::Atom(TypeAtom::Array(Some(inner))) = &row.kind else {
            panic!("expected array<...>, got {row:?}");
        };
        assert!(is_atom(inner, &TypeAtom::Uint));
    }
}
