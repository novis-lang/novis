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
//! failing that, walk its `extends`/`implements`/trait-use ancestors via
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
use mwl_hir::{AliasTable, ClassGraph, QName, SymbolTable};
use mwl_syntax::ast::{
    ClassMember, ClassMemberKind, Modifier, NamespaceDecl, PropertyMember, Stmt, StmtKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lower::{lower_optional_type, lower_type};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// One method's own declared shape: its parameters' types, in declaration
/// order, and its return type. Never includes an inherited override —
/// walking through `extends`/`implements`/trait-use to find one is
/// [`resolve_method`]'s job, not this type's.
#[derive(Clone, Debug)]
pub struct MethodSig {
    /// Each parameter's declared type (`mixed` for one written with none —
    /// already diagnosed elsewhere).
    pub params: Vec<TypeId>,
    /// Whether the last parameter is `...$x` — every argument from that
    /// position onward is checked against its type instead of requiring an
    /// exact count.
    pub variadic: bool,
    /// The declared return type (`mixed` if omitted).
    pub return_ty: TypeId,
}

/// One class/interface/trait/enum's own directly-declared property types and
/// method signatures — never anything pulled in via
/// `extends`/`implements`/trait-use; walking those is [`resolve_property`]/
/// [`resolve_method`]'s job, done at check time.
#[derive(Clone, Debug, Default)]
pub struct ClassSignature {
    /// Instance property types, keyed by name with the `$` sigil stripped.
    pub properties: FxHashMap<String, TypeId>,
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
    /// by name. Never includes one pulled in from a used trait or an
    /// `extends`/`implements` ancestor — [`own_lateinit_properties`]
    /// flattens those in.
    pub lateinit_properties: FxHashSet<String>,
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
}

/// Builds a [`SignatureTable`] for every class/interface/trait/enum declared
/// in `stmts`, lowering every property/parameter/return type through the
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
    symbols: &SymbolTable,
    aliases: &AliasTable,
    graph: &ClassGraph,
    src: &SourceFile,
    interner: &mut crate::ty::TypeInterner,
    diags: &mut Diagnostics,
) -> SignatureTable {
    let mut table = SignatureTable::default();
    let placeholder = SignatureTable::default();
    let mut env = Env {
        symbols,
        aliases,
        graph,
        signatures: &placeholder,
        src,
        interner,
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
                };
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                };
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            StmtKind::TraitDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                };
                collect_members(&decl.members, &qname, &ctx, table, env);
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
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
                let sig = table.entry(qname.clone());
                sig.properties.insert(name.clone(), ty);
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
                let variadic = m.params.last().is_some_and(|p| p.variadic);
                let return_ty = lower_optional_type(m.return_type.as_ref(), ctx, env);
                let name = span_text(env.src, m.name).to_owned();
                table.entry(qname.clone()).methods.insert(
                    name,
                    MethodSig {
                        params,
                        variadic,
                        return_ty,
                    },
                );
            }
            ClassMemberKind::Const(_) | ClassMemberKind::UseTrait(_) | ClassMemberKind::Error => {}
            _ => {}
        }
    }
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
    } else if !matches!(env.interner.get(ty), Ty::Object | Ty::Class(_)) {
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
/// `extends`/`implements`/trait-use ancestors — the same shape
/// `mwl_hir::members::member_declared` already walks for existence-only
/// checking, generalised to return the type found rather than a bool.
#[must_use]
pub fn resolve_property(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<TypeId> {
    let mut seen = FxHashSet::default();
    resolve_property_rec(qname, name, table, graph, &mut seen)
}

fn resolve_property_rec(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<TypeId> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname)
        && let Some(&ty) = sig.properties.get(name)
    {
        return Some(ty);
    }
    let links = graph.get(qname)?;
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .chain(links.traits.iter())
        .find_map(|parent| resolve_property_rec(parent, name, table, graph, seen))
}

/// Looks `name` up as a method on `qname`, falling back to walking ancestors
/// the same way [`resolve_property`] does. Returns a clone since a
/// [`MethodSig`] is cheap and the ancestor it was found on is not otherwise
/// tracked by the caller.
#[must_use]
pub fn resolve_method(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<MethodSig> {
    let mut seen = FxHashSet::default();
    resolve_method_rec(qname, name, table, graph, &mut seen)
}

fn resolve_method_rec(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> Option<MethodSig> {
    if !seen.insert(qname.clone()) {
        return None;
    }
    if let Some(sig) = table.get(qname)
        && let Some(found) = sig.methods.get(name)
    {
        return Some(found.clone());
    }
    let links = graph.get(qname)?;
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .chain(links.traits.iter())
        .find_map(|parent| resolve_method_rec(parent, name, table, graph, seen))
}

/// Every property `qname`'s own constructor must definitely assign per
/// ADR 0022 § 2: `qname`'s own [`ClassSignature::required_properties`],
/// plus every used trait's own (recursively, through nested trait-use — the
/// same flattening [`resolve_property`]/[`resolve_method`] walk, but
/// restricted to `traits` alone). Deliberately excludes `extends`/
/// `implements`: an inherited property is discharged by calling
/// `parent::constructor(...)`, not by assigning it a second time — see
/// `crate::ctor_init`. A name already collected from `qname` itself (or an
/// earlier trait) is not collected again from a later trait, since it is the
/// same storage location either way.
#[must_use]
pub fn own_required_properties(
    qname: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Vec<(String, Span)> {
    let mut seen_classes = FxHashSet::default();
    let mut seen_names = FxHashSet::default();
    let mut out = Vec::new();
    collect_own_required(
        qname,
        table,
        graph,
        &mut seen_classes,
        &mut seen_names,
        &mut out,
    );
    out
}

fn collect_own_required(
    qname: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen_classes: &mut FxHashSet<QName>,
    seen_names: &mut FxHashSet<String>,
    out: &mut Vec<(String, Span)>,
) {
    if !seen_classes.insert(qname.clone()) {
        return;
    }
    if let Some(sig) = table.get(qname) {
        for (name, span) in &sig.required_properties {
            if seen_names.insert(name.clone()) {
                out.push((name.clone(), *span));
            }
        }
    }
    if let Some(links) = graph.get(qname) {
        for trait_q in &links.traits {
            collect_own_required(trait_q, table, graph, seen_classes, seen_names, out);
        }
    }
}

/// Every `lateinit` property `$this` can read anywhere in `qname`'s own
/// methods per ADR 0038 § 3: `qname`'s own
/// [`ClassSignature::lateinit_properties`], plus every used trait's own
/// (recursively, through nested trait-use) — the same trait-only flattening
/// [`own_required_properties`] does, for the same reason: an inherited
/// (`extends`) `lateinit` property is checked when *its own* declaring
/// class's methods are checked, not re-checked here. See
/// `crate::lateinit`'s module docs for the resulting known gap (a subclass
/// method reading an inherited `lateinit` property through `$this` is not
/// covered by this intraprocedural pass).
#[must_use]
pub fn own_lateinit_properties(
    qname: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> FxHashSet<String> {
    let mut seen_classes = FxHashSet::default();
    let mut out = FxHashSet::default();
    collect_own_lateinit(qname, table, graph, &mut seen_classes, &mut out);
    out
}

fn collect_own_lateinit(
    qname: &QName,
    table: &SignatureTable,
    graph: &ClassGraph,
    seen_classes: &mut FxHashSet<QName>,
    out: &mut FxHashSet<String>,
) {
    if !seen_classes.insert(qname.clone()) {
        return;
    }
    if let Some(sig) = table.get(qname) {
        out.extend(sig.lateinit_properties.iter().cloned());
    }
    if let Some(links) = graph.get(qname) {
        for trait_q in &links.traits {
            collect_own_lateinit(trait_q, table, graph, seen_classes, out);
        }
    }
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
        let table = build_signatures(
            &stmts,
            &module.symbols,
            &module.aliases,
            &module.graph,
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
    fn a_method_is_resolved_through_a_trait() {
        let (table, module, interner, _diags) = build(
            "<?mwl\ntrait Greets { function hello(): int { return 1; } }\nclass Foo { use Greets; }\n",
        );
        let sig = resolve_method(&QName::parse("Foo"), "hello", &table, &module.graph)
            .expect("trait method resolves");
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
