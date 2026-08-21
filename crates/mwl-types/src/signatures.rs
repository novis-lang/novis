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

use mwl_diagnostics::{Diagnostics, SourceFile};
use mwl_hir::{AliasTable, ClassGraph, QName, SymbolTable};
use mwl_syntax::ast::{ClassMember, ClassMemberKind, NamespaceDecl, Stmt, StmtKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lower::{lower_optional_type, lower_type};
use crate::ty::TypeId;
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
                table
                    .entry(qname.clone())
                    .properties
                    .insert(strip_sigil(text).to_owned(), ty);
            }
            ClassMemberKind::Method(m) => {
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
}
