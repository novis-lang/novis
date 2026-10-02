//! Type alias substitution — the remaining half of M2 item 3. See the
//! crate's module docs for where this sits relative to the rest of M2.
//!
//! [`crate::resolve::Resolver`] already collects each `type` alias's own
//! declaration into the [`crate::symbol::SymbolTable`] and rejects the
//! single-bare-class shape (`rule:types/alias-is-never-a-bare-class`). What is still open is *what an alias expands to*: § 5 says a `type`
//! alias is "fully transparent" — every occurrence of its name, including
//! inside another alias's own expansion, resolves to the same fully-expanded
//! [`Type`] before anything downstream ever sees the alias's name at all.
//! This module builds that expansion.
//!
//! An alias is written at either of two sites, and they share everything but
//! the key they are filed under: at file or namespace scope, under its own
//! [`QName`], and as a member of a class, interface or enum body, under that
//! owner plus the member's own name. A bare `Name` inside a member's expansion
//! means the owner's own alias before it means the namespace's, and
//! `Owner::Name` reaches one from anywhere; nothing is inherited.
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
//! (`E_TYPE_ALIAS_CYCLE`, `rule:types/type-alias`/7) rather than looped forever or silently bottomed out at `mixed`;
//! every alias name that took part in the cycle still gets an entry in the
//! resulting [`AliasTable`], expanding to `mixed`, so a lookup miss keeps
//! meaning "not an alias" rather than colliding with "an alias that turned
//! out to be broken."
//!
//! **An atom that resolves to nothing declared at all is diagnosed in
//! `nvs-types`, not here.** Whether a name names *something* real is a general
//! type-atom question the type checker owns, and its type lowering reports
//! `E_UNDEFINED_CLASS` for one wherever it is written; this module concerns
//! itself with the alias-substitution question `rule:types/type-alias` asks,
//! the same narrowing [`crate::members`] already applies to `Class::member`
//! references.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, EnumCase, NamespaceDecl, Stmt, StmtKind, Type, TypeAliasDecl,
    TypeAtom, TypeKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::hierarchy::resolve_ref;
use crate::qname::QName;
use crate::resolve::{name_text, qname_segments};

/// What one alias is filed under — the two sites `rule:types/type-alias` lets
/// a `type` declaration be written at.
///
/// A member is keyed by its owner *plus* its own name rather than by a
/// synthetic `Ns\Order\Meta` path, because that path is also the spelling of a
/// class `Meta` declared in namespace `Ns\Order`, and the two must not share a
/// key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum AliasKey {
    /// A file- or namespace-scope alias, under its own fully qualified name.
    Name(QName),
    /// A class, interface or enum member, under its owner and the member name.
    Member(QName, String),
}

impl std::fmt::Display for AliasKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name(qname) => write!(f, "{qname}"),
            Self::Member(owner, name) => write!(f, "{owner}::{name}"),
        }
    }
}

/// Every `type` alias's fully-substituted expansion: the file-scope ones by
/// [`QName`], and the members a body owns by their owner and their own name.
///
/// An expansion is the declared [`Type`] tree, so its spans point into the
/// file that declares the alias, and an alias named inside another one brings
/// a subtree from its own declaration. [`Self::site`] says, for the root of
/// each such subtree, which scope its names are read in.
#[derive(Debug, Default)]
pub struct AliasTable {
    by_name: FxHashMap<QName, Type>,
    by_owner: FxHashMap<QName, FxHashMap<String, Type>>,
    sites: FxHashMap<Span, AliasSite>,
}

/// Where one alias's right-hand side is written: the namespace and `use`
/// imports active at its declaration, and the body that owns it when it is a
/// member. Its spans carry the declaring file, so the source text comes from
/// [`Span::file`].
#[derive(Debug)]
pub struct AliasSite {
    /// The namespace the declaration is written in.
    pub namespace: Vec<String>,
    /// The `use` imports active at the declaration.
    pub imports: FxHashMap<String, QName>,
    /// The class, interface or enum that declares the alias as a member, which
    /// is what `self` and a bare member alias name mean inside it.
    pub owner: Option<QName>,
}

impl AliasTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The fully-substituted expansion for one file-scope alias, if this name
    /// is one.
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&Type> {
        self.by_name.get(qname)
    }

    /// The fully-substituted expansion for `Owner::Name`, if that owner
    /// declares an alias by that name. Nothing is inherited: an owner answers
    /// for the aliases written in its own body and no others
    /// (`rule:types/type-alias`).
    #[must_use]
    pub fn get_member(&self, owner: &QName, name: &str) -> Option<&Type> {
        self.by_owner.get(owner)?.get(name)
    }

    /// The declaration whose right-hand side is exactly `span`, if one is. A
    /// node of an expansion with this span, and everything below it down to
    /// the next node that has a site of its own, is read in this scope and in
    /// this declaration's file, never in the scope of the file using it.
    #[must_use]
    pub fn site(&self, span: Span) -> Option<&AliasSite> {
        self.sites.get(&span)
    }

    /// How many aliases were resolved, both sites together.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len() + self.by_owner.values().map(FxHashMap::len).sum::<usize>()
    }

    /// Whether nothing was resolved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
    /// The body this alias is a member of, if it is one — what a bare `Name`
    /// inside the expansion is looked up against before the namespace.
    owner: Option<QName>,
}

/// Collects a [`AliasTable`] from one or more files' `type` alias
/// declarations, then substitutes every alias's expansion in one closing
/// call. Two calls, same shape as [`crate::hierarchy::HierarchyResolver`],
/// for the same reason: an alias may reference one declared in a file
/// collected later.
#[derive(Debug, Default)]
pub struct AliasResolver {
    pending: FxHashMap<AliasKey, PendingAlias>,
    /// Declaration order, so the top-level resolve loop (and any diagnostics
    /// it produces) doesn't depend on hash-map iteration order.
    order: Vec<AliasKey>,
    /// Every type-atom `Name`'s extracted source text, by its span — recorded
    /// at collection time so resolution needs no [`SourceFile`] afterward,
    /// the same trick [`crate::hierarchy::HierarchyResolver`] uses for a
    /// single name, generalised to a whole type tree.
    names: FxHashMap<Span, String>,
    /// One body's name collisions, found while collecting: an alias sharing a
    /// name with a constant or an enum case. Collection is handed no
    /// [`Diagnostics`], so they wait here for [`AliasResolver::resolve`].
    collisions: Vec<Diagnostic>,
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
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    self.record(
                        AliasKey::Name(qname),
                        decl,
                        src,
                        &current_ns,
                        &imports,
                        None,
                    );
                    // A second declaration of the same name is already
                    // diagnosed as `E_DUPLICATE_DECLARATION` by the symbol
                    // table; the first one wins here too, for consistency.
                }
                // A body's own aliases, keyed by the body that owns them. Every
                // body that has a class-shaped name takes them, and takes them
                // the same way.
                StmtKind::ClassDecl(decl) => {
                    self.record_members(&decl.members, &[], decl.name, src, &current_ns, &imports);
                }
                StmtKind::InterfaceDecl(decl) => {
                    self.record_members(&decl.members, &[], decl.name, src, &current_ns, &imports);
                }
                StmtKind::EnumDecl(decl) => {
                    self.record_members(
                        &decl.members,
                        &decl.cases,
                        decl.name,
                        src,
                        &current_ns,
                        &imports,
                    );
                }
                _ => {}
            }
        }
    }

    /// Every `type` member of one body, under the owner that declares them.
    /// `cases` is an enum's own cases and is empty for a class or an
    /// interface; they are the other half of what
    /// [`Self::check_name_collisions`] compares an alias's name against.
    fn record_members(
        &mut self,
        members: &[ClassMember],
        cases: &[EnumCase],
        owner_name: nvs_syntax::ast::Name,
        src: &SourceFile,
        namespace: &[String],
        imports: &FxHashMap<String, QName>,
    ) {
        let owner = QName::join(namespace, name_text(src, &owner_name));
        self.check_name_collisions(members, cases, &owner, src);
        for member in members {
            let ClassMemberKind::TypeAlias(alias) = &member.kind else {
                continue;
            };
            let key = AliasKey::Member(owner.clone(), name_text(src, &alias.name).to_owned());
            self.record(key, alias, src, namespace, imports, Some(owner.clone()));
        }
    }

    /// An alias, an enum case and a constant are all spelled `Owner::Name`,
    /// so one body declaring two of them under one name is refused where the
    /// later of the two is written — `rule:types/type-alias`, which is
    /// `rule:statements/nothing-gets-a-second-name` for a member. The goal's
    /// resolution order at `Owner::Name` therefore decides nothing a
    /// compiling program can observe.
    ///
    /// Only a collision an alias takes part in is reported here: two
    /// constants or two cases under one name are [`crate::members`]'s to
    /// report. The alias is still recorded, so a reference to it resolves to
    /// its expansion instead of drawing a second, unrelated error.
    fn check_name_collisions(
        &mut self,
        members: &[ClassMember],
        cases: &[EnumCase],
        owner: &QName,
        src: &SourceFile,
    ) {
        let mut declared: Vec<(&str, Span, &str, bool)> = Vec::new();
        for case in cases {
            declared.push((
                name_text(src, &case.name),
                case.name.span,
                "an enum case",
                false,
            ));
        }
        for member in members {
            match &member.kind {
                ClassMemberKind::Const(konst) => declared.push((
                    src.span_text(konst.name).unwrap_or_default(),
                    konst.name,
                    "a class constant",
                    false,
                )),
                ClassMemberKind::TypeAlias(alias) => declared.push((
                    name_text(src, &alias.name),
                    alias.name.span,
                    "a `type` alias",
                    true,
                )),
                _ => {}
            }
        }
        // Source order, because the diagnostic points at the second
        // declaration and the AST keeps an enum's cases beside its members
        // rather than interleaved with them.
        declared.sort_by_key(|(_, span, _, _)| span.start);
        for (index, (name, span, _, is_alias)) in declared.iter().enumerate() {
            let Some((_, first_span, first_kind, first_is_alias)) = declared[..index]
                .iter()
                .find(|(earlier, _, _, _)| earlier == name)
            else {
                continue;
            };
            if !is_alias && !first_is_alias {
                continue;
            }
            self.collisions.push(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("`{owner}::{name}` is already declared as {first_kind}"),
                )
                .with_primary(*span, "duplicate declaration")
                .with_secondary(*first_span, "previously declared here")
                .with_help(
                    "`Owner::Name` in type position names one thing — rename the alias, the \
                     constant or the case (`rule:types/type-alias`)",
                ),
            );
        }
    }

    /// One alias's raw expansion under `key`, with the scope it was written in.
    /// The first declaration under a key wins; a second is a duplicate the
    /// symbol table already reports.
    fn record(
        &mut self,
        key: AliasKey,
        decl: &TypeAliasDecl,
        src: &SourceFile,
        namespace: &[String],
        imports: &FxHashMap<String, QName>,
        owner: Option<QName>,
    ) {
        record_names(&decl.ty, src, &mut self.names);
        if let std::collections::hash_map::Entry::Vacant(entry) = self.pending.entry(key.clone()) {
            entry.insert(PendingAlias {
                ty: decl.ty.clone(),
                decl_span: decl.name.span,
                namespace: namespace.to_vec(),
                imports: imports.clone(),
                owner,
            });
            self.order.push(key);
        }
    }

    /// Substitutes every collected alias's expansion, reporting
    /// `E_TYPE_ALIAS_CYCLE` for a name that, directly or through some chain,
    /// expands back to itself, together with every name collision collection
    /// found ([`Self::check_name_collisions`]). Call once, after every file
    /// sharing this resolver has run [`Self::collect_aliases`].
    #[must_use]
    pub fn resolve(self, diags: &mut Diagnostics) -> AliasTable {
        for collision in self.collisions {
            diags.report(collision);
        }
        let mut ctx = ResolveCtx {
            pending: &self.pending,
            names: &self.names,
            resolved: FxHashMap::default(),
            errored: FxHashSet::default(),
            stack: Vec::new(),
            diags,
        };
        for key in &self.order {
            resolve_one(key, &mut ctx);
        }
        let resolved = ctx.resolved;
        let mut table = AliasTable::new();
        for pending in self.pending.into_values() {
            table.sites.insert(
                pending.ty.span,
                AliasSite {
                    namespace: pending.namespace,
                    imports: pending.imports,
                    owner: pending.owner,
                },
            );
        }
        for (key, ty) in resolved {
            match key {
                AliasKey::Name(qname) => {
                    table.by_name.insert(qname, ty);
                }
                AliasKey::Member(owner, name) => {
                    table.by_owner.entry(owner).or_default().insert(name, ty);
                }
            }
        }
        table
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
        TypeKind::Atom(TypeAtom::CallableSig { params, ret }) => {
            for param in params {
                record_names(param, src, out);
            }
            record_names(ret, src, out);
        }
        TypeKind::Atom(TypeAtom::Name(name, args)) => {
            out.insert(name.span, name_text(src, name).to_owned());
            for arg in args {
                record_names(arg, src, out);
            }
        }
        // `Owner::Name`, which is an alias reference when the owner declares
        // one by that name — and a class constant or an enum case otherwise.
        // Both halves are recorded, since which it is takes the owner resolved.
        TypeKind::Atom(TypeAtom::Member(owner, member)) => {
            out.insert(owner.span, name_text(src, owner).to_owned());
            out.insert(
                *member,
                src.span_text(*member).unwrap_or_default().to_owned(),
            );
        }
        _ => {}
    }
}

/// The read-only tables, the in-progress memo/cycle state, and the
/// diagnostics sink [`resolve_one`]/[`substitute`] need, threaded through one
/// argument instead of several.
struct ResolveCtx<'a> {
    pending: &'a FxHashMap<AliasKey, PendingAlias>,
    names: &'a FxHashMap<Span, String>,
    resolved: FxHashMap<AliasKey, Type>,
    /// Every name that took part in a detected cycle — resolves to `mixed`
    /// from here on, without being recomputed or re-diagnosed.
    errored: FxHashSet<AliasKey>,
    /// The chain of aliases currently being expanded, root to leaf — a name
    /// already on this stack when reached again is the cycle.
    stack: Vec<AliasKey>,
    diags: &'a mut Diagnostics,
}

/// Where one alias's expansion is read from: the namespace and imports active
/// at its declaration, and the body that owns it when it is a member. A bare
/// `Name` inside a member's expansion means the owner's own alias first, which
/// is the short spelling `rule:types/type-alias` gives a member inside its own
/// body.
#[derive(Clone, Copy)]
struct Scope<'a> {
    namespace: &'a [String],
    imports: &'a FxHashMap<String, QName>,
    owner: Option<&'a QName>,
}

fn mixed_at(span: Span) -> Type {
    Type {
        kind: TypeKind::Atom(TypeAtom::Mixed),
        span,
    }
}

fn resolve_one(key: &AliasKey, ctx: &mut ResolveCtx<'_>) -> Type {
    if let Some(ty) = ctx.resolved.get(key) {
        return ty.clone();
    }
    let Some(pending) = ctx.pending.get(key) else {
        unreachable!("resolve_one is only called for a name known to be a pending alias");
    };
    if ctx.errored.contains(key) {
        return mark_mixed(key, pending.decl_span, ctx);
    }
    if let Some(pos) = ctx.stack.iter().position(|q| q == key) {
        let mut chain: Vec<String> = ctx.stack[pos..].iter().map(ToString::to_string).collect();
        chain.push(key.to_string());
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
        let cycle_members: Vec<AliasKey> = ctx.stack[pos..].to_vec();
        for cyclic in &cycle_members {
            let span = ctx
                .pending
                .get(cyclic)
                .map_or(pending.decl_span, |p| p.decl_span);
            mark_mixed(cyclic, span, ctx);
        }
        return mark_mixed(key, pending.decl_span, ctx);
    }

    let scope = Scope {
        namespace: &pending.namespace,
        imports: &pending.imports,
        owner: pending.owner.as_ref(),
    };
    ctx.stack.push(key.clone());
    let substituted = substitute(&pending.ty, scope, ctx);
    ctx.stack.pop();

    if ctx.errored.contains(key) {
        return mark_mixed(key, pending.decl_span, ctx);
    }
    ctx.resolved.insert(key.clone(), substituted.clone());
    substituted
}

/// Records `key` as part of a cycle and caches its expansion as `mixed`,
/// so a later lookup sees a real (if degenerate) entry rather than a miss —
/// a miss would otherwise be indistinguishable from "not an alias at all".
fn mark_mixed(key: &AliasKey, span: Span, ctx: &mut ResolveCtx<'_>) -> Type {
    ctx.errored.insert(key.clone());
    let fallback = mixed_at(span);
    ctx.resolved.insert(key.clone(), fallback.clone());
    fallback
}

/// Rebuilds `ty`, replacing any `TypeAtom::Name` atom that resolves (via
/// [`resolve_ref`], the same lookup [`crate::hierarchy`] uses for
/// `extends`/`implements`) to a pending alias with that alias's own
/// (recursively substituted) expansion. Everything else — a scalar, `self`/
/// `static`/`parent`, a name that resolves to a class/interface/enum instead
/// — is cloned as-is.
fn substitute(ty: &Type, scope: Scope<'_>, ctx: &mut ResolveCtx<'_>) -> Type {
    match &ty.kind {
        TypeKind::Nullable(inner) => Type {
            kind: TypeKind::Nullable(Box::new(substitute(inner, scope, ctx))),
            span: ty.span,
        },
        TypeKind::Union(items) => Type {
            kind: TypeKind::Union(
                items
                    .iter()
                    .map(|item| substitute(item, scope, ctx))
                    .collect(),
            ),
            span: ty.span,
        },
        TypeKind::Intersection(items) => Type {
            kind: TypeKind::Intersection(
                items
                    .iter()
                    .map(|item| substitute(item, scope, ctx))
                    .collect(),
            ),
            span: ty.span,
        },
        TypeKind::Paren(inner) => Type {
            kind: TypeKind::Paren(Box::new(substitute(inner, scope, ctx))),
            span: ty.span,
        },
        TypeKind::Atom(TypeAtom::Array(Some(inner))) => Type {
            kind: TypeKind::Atom(TypeAtom::Array(Some(Box::new(substitute(
                inner, scope, ctx,
            ))))),
            span: ty.span,
        },
        // `rule:types/class-reference`'s class reference, whose argument is a name like any
        // other argument position's: an alias standing for a class expands
        // inside it exactly as it does inside `array<T>`.
        TypeKind::Atom(TypeAtom::ClassRef(inner)) => Type {
            kind: TypeKind::Atom(TypeAtom::ClassRef(Box::new(substitute(inner, scope, ctx)))),
            span: ty.span,
        },
        // `rule:types/callable-signature`: a parameter and a return type are
        // ordinary type positions, so an alias standing for one expands there
        // exactly as it does inside `array<T>`.
        TypeKind::Atom(TypeAtom::CallableSig { params, ret }) => Type {
            kind: TypeKind::Atom(TypeAtom::CallableSig {
                params: params
                    .iter()
                    .map(|param| substitute(param, scope, ctx))
                    .collect(),
                ret: Box::new(substitute(ret, scope, ctx)),
            }),
            span: ty.span,
        },
        TypeKind::Atom(TypeAtom::Name(name, args)) => {
            let text = ctx.names.get(&name.span).map(String::as_str).unwrap_or("");
            // A name *written with* type arguments is never an alias
            // expansion site: `rule:statements/nothing-gets-a-second-name` keeps a `type` alias a synonym for a
            // whole type expression, with no parameters of its own, so
            // `Alias<int>` is an error the checker reports rather than
            // something to expand here. Its arguments still get substituted,
            // so an alias used *as* an argument still expands.
            if !args.is_empty() {
                return Type {
                    kind: TypeKind::Atom(TypeAtom::Name(
                        *name,
                        args.iter().map(|arg| substitute(arg, scope, ctx)).collect(),
                    )),
                    span: ty.span,
                };
            }
            match bare_name_key(text, scope, ctx) {
                Some(key) => resolve_one(&key, ctx),
                None => ty.clone(),
            }
        }
        // `Owner::Name`: an alias the resolved owner declares, or — when it
        // declares none by that name — the class constant or enum case that
        // spelling also means, left as written for the checker to decide.
        TypeKind::Atom(TypeAtom::Member(owner_name, member)) => {
            let owner_text = ctx
                .names
                .get(&owner_name.span)
                .map(String::as_str)
                .unwrap_or("");
            let member_text = ctx.names.get(member).map(String::as_str).unwrap_or("");
            let key = AliasKey::Member(
                resolve_ref(owner_text, scope.namespace, scope.imports),
                member_text.to_owned(),
            );
            if ctx.pending.contains_key(&key) {
                resolve_one(&key, ctx)
            } else {
                ty.clone()
            }
        }
        _ => ty.clone(),
    }
}

/// Which alias, if any, a bare `Name` written in `scope` means: the owner's own
/// member first, then the file-scope name the namespace and imports resolve it
/// to (`rule:types/type-alias`). Nothing is inherited, so an owner that
/// declares no such member falls straight through to the namespace.
fn bare_name_key(text: &str, scope: Scope<'_>, ctx: &ResolveCtx<'_>) -> Option<AliasKey> {
    if let Some(owner) = scope.owner {
        let member = AliasKey::Member(owner.clone(), text.to_owned());
        if ctx.pending.contains_key(&member) {
            return Some(member);
        }
    }
    let qname = AliasKey::Name(resolve_ref(text, scope.namespace, scope.imports));
    ctx.pending.contains_key(&qname).then_some(qname)
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

    /// [`resolve`] over several files, one namespace each.
    fn resolve_files(srcs: &[&str]) -> (AliasTable, Diagnostics) {
        let mut map = SourceMap::new();
        let mut diags = Diagnostics::new();
        let mut resolver = AliasResolver::new();
        for (i, src) in srcs.iter().enumerate() {
            let file = map.add(format!("t{i}.nvs"), *src);
            let stmts = parse_file(map.file(file), &mut diags);
            assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
            resolver.collect_aliases(&stmts, map.file(file));
        }
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
    fn a_class_scoped_alias_is_keyed_by_its_owner_and_not_by_a_namespace_path() {
        let (table, diags) = resolve(
            "<?nvs\nnamespace Ns;\nclass Order { type Meta = decimal; }\ntype Meta = uint;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let member = table
            .get_member(&QName::parse(r"Ns\Order"), "Meta")
            .expect("the owner-keyed entry");
        assert!(is_atom(member, &TypeAtom::Decimal));
        // The file-scope `Meta` in the same namespace is a different alias, and
        // the path that would collide with a class `Meta` names neither.
        assert!(is_atom(
            table.get(&QName::parse(r"Ns\Meta")).unwrap(),
            &TypeAtom::Uint
        ));
        assert!(table.get(&QName::parse(r"Ns\Order\Meta")).is_none());
    }

    #[test]
    fn every_body_that_takes_an_alias_gets_an_owner_keyed_entry() {
        let (table, diags) = resolve(
            "<?nvs\nclass Order { type Meta = decimal; }\n\
             interface Priced { type Meta = uint; }\n\
             enum Status { type Meta = float; Active }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(table.len(), 3);
        for (owner, atom) in [
            ("Order", TypeAtom::Decimal),
            ("Priced", TypeAtom::Uint),
            ("Status", TypeAtom::Float),
        ] {
            let ty = table
                .get_member(&QName::parse(owner), "Meta")
                .unwrap_or_else(|| panic!("{owner} declares an alias"));
            assert!(is_atom(ty, &atom), "{owner}: {ty:?}");
        }
    }

    #[test]
    fn a_bare_name_in_a_body_means_the_owners_alias_before_the_namespaces() {
        let (table, diags) = resolve(
            "<?nvs\ntype Meta = uint;\n\
             class Order { type Meta = decimal; type Wrapped = array<Meta>; }\n\
             type Outer = array<Meta>;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let wrapped = table.get_member(&QName::parse("Order"), "Wrapped").unwrap();
        let TypeKind::Atom(TypeAtom::Array(Some(inner))) = &wrapped.kind else {
            panic!("expected array<...>, got {wrapped:?}");
        };
        assert!(is_atom(inner, &TypeAtom::Decimal));
        // Outside the body the same spelling is the file-scope alias.
        let outer = table.get(&QName::parse("Outer")).unwrap();
        let TypeKind::Atom(TypeAtom::Array(Some(inner))) = &outer.kind else {
            panic!("expected array<...>, got {outer:?}");
        };
        assert!(is_atom(inner, &TypeAtom::Uint));
    }

    #[test]
    fn a_cycle_through_a_class_scoped_alias_is_refused() {
        // Both spellings of a member reference take part: `B` is the bare form
        // inside the owner, `Order::A` the qualified one.
        let (table, diags) =
            resolve("<?nvs\nclass Order { type A = array<B>; type B = array<Order::A>; }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TYPE_ALIAS_CYCLE)),
            "{diags:?}"
        );
        for name in ["A", "B"] {
            let ty = table.get_member(&QName::parse("Order"), name).unwrap();
            assert!(is_atom(ty, &TypeAtom::Mixed), "{name}: {ty:?}");
        }
    }

    #[test]
    fn an_alias_sharing_a_name_with_a_constant_is_refused() {
        let (_, diags) =
            resolve("<?nvs\nclass Holder { public const int ID = 1; type ID = array<uint>; }\n");
        let duplicates: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_DUPLICATE_DECLARATION))
            .collect();
        assert_eq!(duplicates.len(), 1, "{diags:?}");
    }

    #[test]
    fn an_alias_sharing_a_name_with_an_enum_case_is_refused() {
        let (_, diags) = resolve("<?nvs\nenum Colour: int { Red = 1, type Red = array<uint>; }\n");
        let duplicates: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_DUPLICATE_DECLARATION))
            .collect();
        assert_eq!(duplicates.len(), 1, "{diags:?}");
    }

    #[test]
    fn an_alias_and_a_differently_named_constant_are_both_fine() {
        let (table, diags) =
            resolve("<?nvs\nclass Holder { public const int ID = 1; type Ids = array<uint>; }\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            table.get_member(&QName::parse("Holder"), "Ids").is_some(),
            "the alias is still recorded"
        );
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
        let (table, diags) = resolve_files(&[
            "<?nvs\nnamespace App;\ntype Id = uint;\n",
            "<?nvs\nnamespace App\\Http;\nuse App\\Id;\ntype Row = array<Id>;\n",
        ]);
        assert!(!diags.has_errors(), "{diags:?}");
        let row = table.get(&QName::parse("App\\Http\\Row")).unwrap();
        let TypeKind::Atom(TypeAtom::Array(Some(inner))) = &row.kind else {
            panic!("expected array<...>, got {row:?}");
        };
        assert!(is_atom(inner, &TypeAtom::Uint));
    }
}
