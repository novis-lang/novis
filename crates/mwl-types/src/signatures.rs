//! Per-class property and method signatures — the groundwork the M2
//! follow-up list's item 1 named as blocking property/method-call/`new`
//! expression typing: `mwl_hir::members` only ever checked whether a member
//! *exists*, never what type it has, since that needed this crate's type
//! table in the first place.
//!
//! [`build_signatures`] walks a file once, ahead of any body-checking, the
//! same shape [`crate::check::check_program`] itself walks (mirroring
//! `mwl_hir::members`'s own two-pass split): a property's declared type and
//! a method's parameter/return types are lowered the same way
//! `check::check_method` lowers a method body's own parameters, into a
//! [`SignatureTable`] every method body is then checked against.
//! [`resolve_property`]/[`resolve_method`] look a name up on a class and,
//! failing that, walk its `extends`/`implements` ancestors via
//! [`mwl_hir::ClassGraph`] — the same ancestor walk
//! `mwl_hir::members::member_declared` already does for existence-only
//! checking.
//!
//! **Known gaps:**
//! - A promoted constructor-parameter property (`function constructor(public
//!   int $x) {}`) is not recorded as a property here, matching
//!   `mwl_hir::members`'s own member table, which has the same gap.
//! - A variadic parameter's declared type is matched against every argument
//!   from its position onward (an element-type check) rather than being
//!   modeled as its own `array<T>` — see [`crate::expr`]'s docs for where
//!   that's used.
//! - A class constant's value has no recorded type here at all —
//!   `Class::CONST` stays `mixed` regardless of receiver, same as before
//!   this module existed.

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use mwl_hir::{ClassGraph, QName, SymbolKind};
use mwl_syntax::ast::{
    ClassMember, ClassMemberKind, Modifier, NamespaceDecl, PropertyMember, Stmt, StmtKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lower::{lower_optional_type, lower_type};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// One method's own declared shape: its parameters' types, in declaration
/// order, and its return type. Never includes an inherited override —
/// walking through `extends`/`implements` to find one is [`resolve_method`]'s
/// job, not this type's.
#[derive(Clone, Debug)]
pub struct MethodSig {
    /// Each parameter's declared type (`mixed` for one written with none —
    /// already diagnosed elsewhere).
    pub params: Vec<TypeId>,
    /// Whether each parameter is declared `&$x`, positionally — one entry per
    /// [`Self::params`] entry, read through [`Self::is_by_ref`] rather than
    /// indexed directly so the variadic rule stays in one place.
    ///
    /// A parallel `Vec` rather than a field on a per-parameter struct because
    /// every existing consumer reads [`Self::params`] positionally already
    /// (see [`Self::param_at`]), and a by-reference parameter is rare enough
    /// that turning one `Vec<TypeId>` into a `Vec<Param>` would rewrite every
    /// one of those call sites to buy nothing.
    pub by_ref: Vec<bool>,
    /// Whether the last parameter is `...$x` — every argument from that
    /// position onward is checked against its type instead of requiring an
    /// exact count.
    pub variadic: bool,
    /// The declared return type (`mixed` if omitted).
    pub return_ty: TypeId,
    /// Whether the declaration carries the `static` modifier — ADR 0008 § 1's
    /// one surviving meaning of the keyword.
    ///
    /// Recorded because a call's *shape* does not settle it: `parent::method()`
    /// and `self::method()` are written like a static call but invoke an
    /// instance method with the enclosing `$this` whenever the target is not
    /// static, which is how a subclass constructor reaches its parent's. A
    /// consumer that got this wrong would pass `null` where the callee expects
    /// a receiver.
    pub is_static: bool,
    /// Whether this is a `private` interface method (ADR 0043 § 3) —
    /// declared with the `private` modifier inside an `interface`, not a
    /// `class`. General class-level method visibility is not modeled at all
    /// yet (see `mwl_hir::members`'s own known gaps); this field exists only
    /// so [`crate::expr`] can enforce the one visibility rule ADR 0043 § 3
    /// actually requires — a private interface method is not part of that
    /// interface's contract, so it is never reachable outside that
    /// interface's own method bodies, not even from an implementing class.
    pub interface_private: bool,
    /// Whether the declaration carries a *body* — false for an `abstract`
    /// method and for an interface method declared without a default (ADR
    /// 0043 § 2).
    ///
    /// Recorded for the same reason [`Self::is_static`] is: the call site
    /// cannot see it. A call resolving to a bodiless declaration has no
    /// compiled function to name, so it must dispatch on the receiver's
    /// runtime class instead — which is exactly what an interface default
    /// method calling back into the contract it declares
    /// (`$this->name()` inside `Greets::greet`) does.
    pub has_body: bool,
}

impl MethodSig {
    /// The parameter type at `index`, following the variadic rule: every
    /// argument from the last parameter's position onward is checked against
    /// that parameter's own type. `None` for an index past a non-variadic
    /// signature's parameters, which the arity check has already reported.
    #[must_use]
    pub fn param_at(&self, index: usize) -> Option<TypeId> {
        if self.variadic && index >= self.params.len().saturating_sub(1) {
            return self.params.last().copied();
        }
        self.params.get(index).copied()
    }

    /// Whether the parameter at `index` is declared `&$x`, following the same
    /// variadic rule [`Self::param_at`] does — every argument from a variadic
    /// parameter's position onward binds the way that parameter declares.
    /// `false` for an index past a non-variadic signature's parameters, which
    /// the arity check has already reported.
    #[must_use]
    pub fn is_by_ref(&self, index: usize) -> bool {
        if self.variadic && index >= self.by_ref.len().saturating_sub(1) {
            return self.by_ref.last().copied().unwrap_or(false);
        }
        self.by_ref.get(index).copied().unwrap_or(false)
    }

    /// Whether any parameter is declared `&$x` — the cheap test a call site
    /// runs before doing any by-reference work at all.
    #[must_use]
    pub fn has_by_ref(&self) -> bool {
        self.by_ref.iter().any(|&r| r)
    }

    /// Whether this signature mentions a type variable anywhere — the test
    /// that decides whether a call site needs [`crate::generics`] at all.
    /// Always false for a user-declared signature: ADR 0007 parks
    /// user-declared generics, so only a `Core` member registered through
    /// [`crate::core_lib`] can answer true.
    #[must_use]
    pub fn is_generic(&self, interner: &crate::ty::TypeInterner) -> bool {
        self.params
            .iter()
            .chain(std::iter::once(&self.return_ty))
            .any(|id| crate::generics::mentions_type_var(*id, interner))
    }

    /// This signature with `bindings` applied to every parameter and to the
    /// return type — see [`crate::generics`] for the binding rule and for why
    /// an unbound variable becomes `mixed`.
    #[must_use]
    pub(crate) fn substituted(
        self,
        bindings: &crate::generics::Bindings,
        interner: &mut crate::ty::TypeInterner,
    ) -> Self {
        Self {
            params: self
                .params
                .iter()
                .map(|id| crate::generics::substitute(*id, bindings, interner))
                .collect(),
            return_ty: crate::generics::substitute(self.return_ty, bindings, interner),
            ..self
        }
    }
}

/// Which of a property's two ADR 0014 § 1 hooks a declaration actually
/// writes with a body. A property with neither has no entry in
/// [`ClassSignature::hooked_properties`] at all.
///
/// # Every hooked property is still *backed*
///
/// PHP 8.4 splits hooked properties into "backed" (some hook body mentions
/// `$this->thatSameProperty`, so the slot is kept) and "virtual" (no hook
/// mentions it, so the slot is dropped). MWL keeps the slot either way —
/// [`crate::layout`] gives every declared property a slot, hooked or not —
/// which is why nothing here records backedness. The trade is one machine
/// word per instance for a property whose hooks never touch storage, bought
/// against an AST walk over every hook body, a second layout rule, and a
/// second legality rule for what a hook body may say. CLAUDE.md's priority
/// ordering puts simplicity above footprint and names exactly this shape of
/// trade; the word is per *instance* of a class that declares a virtual
/// hooked property, so it is O(in-flight objects), not O(traffic).
///
/// The observable consequence is that ADR 0014 § 1's "same as PHP 8.4" holds
/// for every program PHP accepts, and MWL additionally accepts one PHP
/// rejects: writing to a property whose hooks never mention it stores into
/// that slot instead of being refused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PropertyHooks {
    /// A `get` hook with a body is declared.
    pub get: bool,
    /// A `set` hook with a body is declared.
    pub set: bool,
}

/// The label the compiled function for `class`'s `$name` property hook is
/// emitted under — `Ns\Class::$prop::get`.
///
/// Spelled here rather than at either end so a hook's *definition* (recorded
/// by [`crate::check`] through `ExprTypeTable::record_method`) and its *call*
/// site (`ExprInfo::HookedProperty`) agree by construction, exactly the
/// reason `ExprTypeTable::method_label` exists for an ordinary method. The
/// `$` is what keeps a hook label out of any method's namespace: a class name
/// never contains `::`, so `A::$b::get` can only ever be a hook.
#[must_use]
pub fn hook_label(class: &QName, name: &str, kind: mwl_syntax::ast::PropertyHookKind) -> String {
    let accessor = match kind {
        mwl_syntax::ast::PropertyHookKind::Get => "get",
        mwl_syntax::ast::PropertyHookKind::Set => "set",
    };
    format!("{class}::${name}::{accessor}")
}

/// The declaring class's label back out of a [`hook_label`] — `Ns\Class` for
/// `Ns\Class::$prop::get`. `None` for any string that is not a hook label.
///
/// The inverse lives beside the spelling for the same reason the spelling is
/// centralised at all: `mwl-ir` has to name the field a `set => expr;` hook
/// stores into, and it holds the hook's label but has no `QName` machinery of
/// its own to rebuild one from. Splitting here keeps both halves of the
/// format in one file, so a change to it cannot leave the two disagreeing.
#[must_use]
pub fn hook_label_class(label: &str) -> Option<&str> {
    let (class, rest) = label.split_once("::$")?;
    (rest.ends_with("::get") || rest.ends_with("::set")).then_some(class)
}

/// One class/interface/enum's own directly-declared property types and
/// method signatures — never anything pulled in via `extends`/`implements`;
/// walking those is [`resolve_property`]/[`resolve_method`]'s job, done at
/// check time.
#[derive(Clone, Debug, Default)]
pub struct ClassSignature {
    /// Instance property types, keyed by name with the `$` sigil stripped.
    pub properties: FxHashMap<String, TypeId>,
    /// This declaration's own properties that carry an ADR 0014 § 1 hook
    /// block, by name — see [`PropertyHooks`]. A property with no hooks, or
    /// whose hooks are all bodiless (an abstract hook in an interface), is
    /// absent.
    pub hooked_properties: FxHashMap<String, PropertyHooks>,
    /// Method signatures, keyed by method name.
    pub methods: FxHashMap<String, MethodSig>,
    /// This declaration's own properties that ADR 0022 § 2 requires a
    /// constructor to definitely assign: non-nullable (`TypeInterner::is_nullable`
    /// is false), no inline default, and no hook block. A hooked property is
    /// exempted here entirely rather than modeled — see
    /// `crate::ctor_init`'s module docs for why. A `lateinit` property is
    /// also excluded (ADR 0038 § 1: it is exempt from this obligation by
    /// design, not merely by accident of shape). Name, declaration span, in
    /// declaration order.
    pub required_properties: Vec<(String, Span)>,
    /// This declaration's own properties declared `lateinit` (ADR 0038 § 1),
    /// by name. Never includes one pulled in from an `extends`/`implements`
    /// ancestor — [`own_lateinit_properties`] flattens those in.
    pub lateinit_properties: FxHashSet<String>,
    /// This declaration's own `implements` entries, in source order, each
    /// with the concrete type arguments it fixed (ADR 0053 § 2). Empty
    /// arguments for every interface but `Iterable`/`Iterator`, which is
    /// every interface in the language today except those two.
    ///
    /// [`mwl_hir::ClassGraph`] already records *which* interfaces a class
    /// implements, and is the right table for a reachability question. This
    /// one exists because the arguments need [`TypeId`]s, which `mwl-hir` has
    /// no interner for — so a question like "what does a `foreach` over a
    /// `Counter` yield" is answered here and the plain "does `Counter` reach
    /// `Iterable` at all" stays there.
    pub implements: Vec<(QName, Vec<TypeId>)>,
}

/// Every declaration's own [`ClassSignature`], keyed by its [`QName`].
#[derive(Debug, Default)]
pub struct SignatureTable {
    by_class: FxHashMap<QName, ClassSignature>,
}

impl SignatureTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One declaration's own signatures, if any were collected under this
    /// name.
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&ClassSignature> {
        self.by_class.get(qname)
    }

    fn entry(&mut self, qname: QName) -> &mut ClassSignature {
        self.by_class.entry(qname).or_default()
    }

    /// Installs a whole class's properties and method signatures at once,
    /// with no constructor obligations — the one shape a *native* declaration
    /// has, since [`crate::core_lib`] and [`crate::error_lib`] are the only
    /// callers and neither has source text to collect from. Deliberately not
    /// a general insertion point: everything else goes through
    /// [`build_signatures`]'s own walk.
    ///
    /// `required_properties` stays empty on purpose. ADR 0022's obligation is
    /// a check on a *written* constructor, and neither caller has one — a
    /// `Core` class has no state at all, and `Throwable`'s constructor is
    /// synthesized by `mwl_ir::lower`.
    pub(crate) fn seed_class(
        &mut self,
        qname: QName,
        properties: FxHashMap<String, TypeId>,
        methods: FxHashMap<String, MethodSig>,
    ) {
        let entry = self.entry(qname);
        entry.properties = properties;
        entry.methods = methods;
    }
}

/// Builds a [`SignatureTable`] for every class/interface/enum declared in
/// `stmts`, lowering every property/parameter/return type through the
/// same [`AliasTable`]/[`SymbolTable`] a method body's own types go through.
///
/// This writes into its own `table` return value rather than `env.signatures`
/// — the [`Env`] this function builds internally points `signatures` at an
/// unrelated, empty placeholder (never read during collection, only during
/// later body-checking), which is what lets this run *before* the table it
/// produces exists: [`crate::Env`] borrows `interner`/`diags` mutably, and a
/// `SignatureTable` under active construction can't also be borrowed
/// immutably through the same `Env` at once.
pub fn build_signatures(
    stmts: &[Stmt],
    module: &mwl_hir::Module,
    enums: &crate::enums::EnumTable,
    src: &SourceFile,
    interner: &mut crate::ty::TypeInterner,
    diags: &mut Diagnostics,
) -> SignatureTable {
    let (symbols, aliases, graph) = (&module.symbols, &module.aliases, &module.graph);
    let mut table = SignatureTable::default();
    // `Core` first, so a user declaration can never be collected under a name
    // the stdlib already owns without the later insertion being visible.
    crate::core_lib::seed(&mut table, interner);
    crate::error_lib::seed(&mut table, interner);
    crate::iter_lib::seed(&mut table, interner);
    let placeholder = SignatureTable::default();
    // Same placeholder idea as `signatures` above: signature collection only
    // ever lowers property/parameter/return *type annotations*, never a call
    // expression, so nothing during this pass ever records into `exprs`.
    let mut placeholder_exprs = crate::expr_table::ExprTypeTable::default();
    let mut env = Env {
        symbols,
        aliases,
        graph,
        signatures: &placeholder,
        enums,
        src,
        interner,
        exprs: &mut placeholder_exprs,
        diags,
    };
    collect_stmts(stmts, &[], &FxHashMap::default(), &mut table, &mut env);
    table
}

fn qname_segments(src: &SourceFile, name: &mwl_syntax::ast::Name) -> Vec<String> {
    QName::parse(span_text(src, name.span)).segments().to_vec()
}

fn collect_stmts(
    stmts: &[Stmt],
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    table: &mut SignatureTable,
    env: &mut Env<'_>,
) {
    let mut current_ns: Vec<String> = namespace.to_vec();
    let mut current_imports: FxHashMap<String, QName> = imports.clone();

    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name
                    .as_ref()
                    .map_or_else(Vec::new, |n| qname_segments(env.src, n));
                match body {
                    Some(block) => {
                        collect_stmts(&block.stmts, &new_ns, &FxHashMap::default(), table, env);
                    }
                    None => {
                        current_ns = new_ns;
                        current_imports.clear();
                    }
                }
            }
            StmtKind::UseDecl(use_decl) => {
                let target = QName::parse(span_text(env.src, use_decl.path.span));
                current_imports.insert(target.short_name().to_owned(), target);
            }
            StmtKind::ClassDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                };
                // Before the members: ADR 0053 § 2's type arguments are part
                // of the declaration's own shape, not of any one member's.
                let implements: Vec<(QName, Vec<TypeId>)> = decl
                    .implements
                    .iter()
                    .map(|clause| crate::lower::lower_implemented_interface(clause, &ctx, env))
                    .collect();
                table.entry(qname.clone()).implements = implements;
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                };
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                };
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            _ => {}
        }
    }
}

fn collect_members(
    members: &[ClassMember],
    qname: &QName,
    ctx: &Ctx<'_>,
    table: &mut SignatureTable,
    env: &mut Env<'_>,
) {
    // Needed only to decide `MethodSig::interface_private` below — a
    // `private` method modifier means something (ADR 0043 § 3) exactly when
    // the enclosing declaration is an `interface`, not a `class`/`enum`.
    let is_interface = env
        .symbols
        .get(qname)
        .is_some_and(|sym| sym.kind == SymbolKind::Interface);
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                let ty = lower_type(&p.ty, ctx, env);
                let text = span_text(env.src, p.name);
                let name = strip_sigil(text).to_owned();
                let is_lateinit = p.modifiers.contains(&Modifier::Lateinit);
                if is_lateinit {
                    check_lateinit_property(p, ty, env);
                }
                let required = p.default.is_none()
                    && p.hooks.is_none()
                    && !is_lateinit
                    && !env.interner.is_nullable(ty);
                let hooks = declared_hooks(p);
                let sig = table.entry(qname.clone());
                sig.properties.insert(name.clone(), ty);
                if hooks != PropertyHooks::default() {
                    sig.hooked_properties.insert(name.clone(), hooks);
                }
                if required {
                    sig.required_properties.push((name.clone(), p.name));
                }
                if is_lateinit {
                    sig.lateinit_properties.insert(name);
                }
            }
            ClassMemberKind::Method(m) => {
                for p in &m.params {
                    if p.modifiers.contains(&Modifier::Lateinit) {
                        env.diags.report(
                            Diagnostic::error(
                                code::E_LATEINIT_PROMOTED_PARAM,
                                "`lateinit` cannot be used on a constructor parameter",
                            )
                            .with_primary(p.name, "declared `lateinit` here")
                            .with_help(
                                "binding a parameter already assigns it — `lateinit` has \
                                 nothing left to defer",
                            ),
                        );
                    }
                }
                let params: Vec<TypeId> = m
                    .params
                    .iter()
                    .map(|p| lower_optional_type(p.ty.as_ref(), ctx, env))
                    .collect();
                let by_ref: Vec<bool> = m.params.iter().map(|p| p.by_ref).collect();
                let variadic = m.params.last().is_some_and(|p| p.variadic);
                let return_ty = lower_optional_type(m.return_type.as_ref(), ctx, env);
                let interface_private = is_interface && m.modifiers.contains(&Modifier::Private);
                let is_static = m.modifiers.contains(&Modifier::Static);
                let name = span_text(env.src, m.name).to_owned();
                table.entry(qname.clone()).methods.insert(
                    name,
                    MethodSig {
                        params,
                        by_ref,
                        variadic,
                        return_ty,
                        is_static,
                        interface_private,
                        has_body: m.body.is_some(),
                    },
                );
            }
            ClassMemberKind::Const(_) | ClassMemberKind::Error => {}
            _ => {}
        }
    }
}

/// Which hooks `p` declares *with a body* — a bodiless `get;` in an
/// interface or abstract class declares a requirement, not code to call, so
/// it contributes nothing here.
fn declared_hooks(p: &PropertyMember) -> PropertyHooks {
    let mut out = PropertyHooks::default();
    for hook in p.hooks.iter().flatten() {
        if hook.body.is_none() {
            continue;
        }
        match hook.kind {
            mwl_syntax::ast::PropertyHookKind::Get => out.get = true,
            mwl_syntax::ast::PropertyHookKind::Set => out.set = true,
        }
    }
    out
}

/// Validates a `lateinit` property against ADR 0038 § 1's three rejected
/// shapes — nullability, a non-object type, and `readonly` — reporting each
/// diagnostic that applies. `ty` is `p`'s already-lowered type.
fn check_lateinit_property(p: &PropertyMember, ty: TypeId, env: &mut Env<'_>) {
    if env.interner.is_nullable(ty) {
        env.diags.report(
            Diagnostic::error(
                code::E_LATEINIT_NULLABLE,
                "`lateinit` cannot be combined with a nullable type",
            )
            .with_primary(p.name, "declared `lateinit` here")
            .with_help(
                "a nullable property already has a value for \"not set yet\" — drop either \
                 `lateinit` or the `?`",
            ),
        );
    } else if !matches!(env.interner.get(ty), Ty::Object | Ty::Class(..)) {
        env.diags.report(
            Diagnostic::error(
                code::E_LATEINIT_NOT_OBJECT_TYPE,
                "`lateinit` is only allowed on a class- or interface-typed property",
            )
            .with_primary(p.name, "declared `lateinit` here")
            .with_help("give this property a real default value instead, e.g. `= 0`"),
        );
    }
    if p.modifiers.contains(&Modifier::Readonly) {
        env.diags.report(
            Diagnostic::error(
                code::E_LATEINIT_READONLY_CONFLICT,
                "`lateinit` cannot be combined with `readonly`",
            )
            .with_primary(p.name, "both modifiers declared on this property")
            .with_help(
                "`readonly` requires assignment during construction; `lateinit` requires \
                 assignment after it — pick one",
            ),
        );
    }
}

/// Looks `name` up as a property on `qname`, falling back to walking its
/// `extends`/`implements` ancestors — the same shape
/// `mwl_hir::members::member_declared` already walks for existence-only
/// checking, generalised to return the type found rather than a bool.
#[must_use]
pub fn resolve_property(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<TypeId> {
    resolve_property_owned(qname, name, table, graph).map(|(_, ty)| ty)
}

/// [`resolve_property`], plus the [`QName`] that actually *declares* the
/// property — the receiver's own class, or whichever ancestor it inherited
/// the declaration from.
///
/// Separate from [`resolve_property`] because only one caller needs the
/// owner: a hooked property's compiled hook is labelled with its declaring
/// class ([`hook_label`]), the same way a method call names the class that
/// declares the method rather than the one the call was written on. A field
/// *slot* needs no such thing — [`crate::layout`] flattens an ancestor's
/// slots into every subclass, so a `FieldGet` names the receiver's own class.
#[must_use]
pub fn resolve_property_owned(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<(QName, TypeId)> {
    let mut seen = FxHashSet::default();
    resolve_property_rec(qname, name, table, graph, &mut seen)
}

fn resolve_property_rec(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<(QName, TypeId)> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname)
        && let Some(&ty) = sig.properties.get(name)
    {
        return Some((qname.clone(), ty));
    }
    let links = graph.get(qname)?;
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .find_map(|parent| resolve_property_rec(parent, name, table, graph, seen))
}

/// Which ADR 0014 § 1 hooks the property `owner::$name` declares — `owner`
/// being the *declaring* class [`resolve_property_owned`] returned, not the
/// class the access was written on. [`PropertyHooks::default`] (neither hook)
/// for an ordinary stored property.
#[must_use]
pub fn hooks_of(owner: &QName, name: &str, table: &SignatureTable) -> PropertyHooks {
    table
        .get(owner)
        .and_then(|sig| sig.hooked_properties.get(name).copied())
        .unwrap_or_default()
}

/// Looks `name` up as a method on `qname`, falling back to walking ancestors
/// the same way [`resolve_property`] does. Returns the [`QName`] that actually
/// declares it alongside a clone of its signature — the owner is needed by
/// [`crate::expr`] to enforce ADR 0043 § 3's private-interface-method
/// visibility rule (private is only visible from inside its own declaring
/// interface, never through whatever class or subinterface the lookup
/// started from), not just to type-check the call.
#[must_use]
pub fn resolve_method(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<(QName, MethodSig)> {
    let mut seen = FxHashSet::default();
    resolve_method_rec(qname, name, table, graph, &mut seen)
}

fn resolve_method_rec(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<(QName, MethodSig)> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname)
        && let Some(found) = sig.methods.get(name)
    {
        return Some((qname.clone(), found.clone()));
    }
    let links = graph.get(qname)?;
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .find_map(|parent| resolve_method_rec(parent, name, table, graph, seen))
}

/// Every property `qname`'s own constructor must definitely assign per
/// ADR 0022 § 2: exactly `qname`'s own [`ClassSignature::required_properties`].
/// Deliberately excludes `extends`/`implements`: an inherited property is
/// discharged by calling `parent::constructor(...)`, not by assigning it a
/// second time — see `crate::ctor_init`.
///
/// Before [ADR 0043](../../../docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md),
/// this also flattened in every used trait's own required properties
/// (recursively, through nested trait-use); that ancestor walk is gone along
/// with traits themselves — a `by`-target field used for delegation is an
/// ordinary declared property of `qname` itself, already covered by
/// `required_properties` with no special case needed (ADR 0043's own
/// amendment to ADR 0022 § 2).
#[must_use]
pub fn own_required_properties(qname: &QName, table: &SignatureTable) -> Vec<(String, Span)> {
    table
        .get(qname)
        .map(|sig| sig.required_properties.clone())
        .unwrap_or_default()
}

/// Every `lateinit` property `$this` can read anywhere in `qname`'s own
/// methods per ADR 0038 § 3: exactly `qname`'s own
/// [`ClassSignature::lateinit_properties`] — an inherited (`extends`)
/// `lateinit` property is checked when *its own* declaring class's methods
/// are checked, not re-checked here. See `crate::lateinit`'s module docs for
/// the resulting known gap (a subclass method reading an inherited
/// `lateinit` property through `$this` is not covered by this
/// intraprocedural pass).
///
/// [ADR 0043](../../../docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)
/// retired this function's former trait-flattening role the same way it did
/// [`own_required_properties`]'s.
#[must_use]
pub fn own_lateinit_properties(qname: &QName, table: &SignatureTable) -> FxHashSet<String> {
    table
        .get(qname)
        .map(|sig| sig.lateinit_properties.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::SourceMap;
    use mwl_hir::resolve_file;
    use mwl_syntax::parse_file;

    use super::*;
    use crate::ty::TypeInterner;

    fn build(src: &str) -> (SignatureTable, mwl_hir::Module, TypeInterner, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let enums = crate::enums::build_enum_table(&stmts, map.file(file), &mut diags);
        let table = build_signatures(
            &stmts,
            &module,
            &enums,
            map.file(file),
            &mut interner,
            &mut diags,
        );
        (table, module, interner, diags)
    }

    #[test]
    fn a_property_type_is_recorded() {
        let (table, _module, interner, _diags) = build("<?mwl\nclass Foo { public int $count; }\n");
        let ty = table
            .get(&QName::parse("Foo"))
            .and_then(|sig| sig.properties.get("count"))
            .copied()
            .expect("property recorded");
        assert_eq!(interner.describe(ty), "int");
    }

    #[test]
    fn a_method_signature_is_recorded() {
        let (table, _module, interner, _diags) =
            build("<?mwl\nclass Foo { function greet(string $name): bool { return true; } }\n");
        let sig = table
            .get(&QName::parse("Foo"))
            .and_then(|sig| sig.methods.get("greet"))
            .expect("method recorded");
        assert!(!sig.variadic);
        assert_eq!(interner.describe(sig.params[0]), "string");
        assert_eq!(interner.describe(sig.return_ty), "bool");
    }

    #[test]
    fn a_property_is_resolved_through_an_ancestor() {
        let (table, module, interner, _diags) =
            build("<?mwl\nclass Base { public int $count; }\nclass Sub extends Base {}\n");
        let ty = resolve_property(&QName::parse("Sub"), "count", &table, &module.graph)
            .expect("inherited property resolves");
        assert_eq!(interner.describe(ty), "int");
    }

    #[test]
    fn a_method_is_resolved_through_an_ancestor() {
        let (table, module, interner, _diags) = build(
            "<?mwl\nclass Base { function hello(): int { return 1; } }\nclass Sub extends Base {}\n",
        );
        let (owner, sig) = resolve_method(&QName::parse("Sub"), "hello", &table, &module.graph)
            .expect("inherited method resolves");
        assert_eq!(owner, QName::parse("Base"));
        assert_eq!(interner.describe(sig.return_ty), "int");
    }

    #[test]
    fn an_undeclared_member_does_not_resolve() {
        let (table, module, _interner, _diags) = build("<?mwl\nclass Foo {}\n");
        assert!(resolve_property(&QName::parse("Foo"), "missing", &table, &module.graph).is_none());
        assert!(resolve_method(&QName::parse("Foo"), "missing", &table, &module.graph).is_none());
    }

    // ------------------------------------------------------------------
    // ADR 0038 § 1 -- `lateinit`'s four rejected shapes.
    // ------------------------------------------------------------------

    #[test]
    fn a_lateinit_class_typed_property_is_excluded_from_required_properties() {
        let (table, _module, _interner, diags) =
            build("<?mwl\nclass Logger {}\nclass Widget { public lateinit Logger $logger; }\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let sig = table
            .get(&QName::parse("Widget"))
            .expect("signature recorded");
        assert!(sig.lateinit_properties.contains("logger"));
        assert!(sig.required_properties.is_empty());
    }

    #[test]
    fn lateinit_on_a_scalar_property_is_diagnosed() {
        let (_table, _module, _interner, diags) =
            build("<?mwl\nclass Widget { public lateinit int $count; }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_NOT_OBJECT_TYPE)),
            "{diags:?}"
        );
    }

    #[test]
    fn lateinit_on_a_nullable_property_is_diagnosed() {
        let (_table, _module, _interner, diags) =
            build("<?mwl\nclass Logger {}\nclass Widget { public lateinit ?Logger $logger; }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_NULLABLE)),
            "{diags:?}"
        );
    }

    #[test]
    fn lateinit_on_a_promoted_parameter_is_diagnosed() {
        let (_table, _module, _interner, diags) = build(
            "<?mwl\nclass Logger {}\nclass Widget {\n  function constructor(public lateinit Logger $logger) {}\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_PROMOTED_PARAM)),
            "{diags:?}"
        );
    }

    #[test]
    fn lateinit_combined_with_readonly_is_diagnosed() {
        let (_table, _module, _interner, diags) = build(
            "<?mwl\nclass Logger {}\nclass Widget { public lateinit readonly Logger $logger; }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_LATEINIT_READONLY_CONFLICT)),
            "{diags:?}"
        );
    }
}
