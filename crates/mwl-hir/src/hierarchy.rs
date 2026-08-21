//! The class hierarchy graph: `extends`/`implements` resolved to real
//! [`Symbol`]s, and trait-use collision resolution via `insteadof`
//! (M2 item 1 — see the crate's module docs for what's left after this).
//!
//! Built in the same two-pass shape as [`crate::resolve`]: [`HierarchyResolver::collect_links`]
//! walks a file's declarations, recording each one's raw `extends`/
//! `implements`/trait-use references together with the namespace and `use`
//! imports active at that point (an unqualified reference needs both to
//! resolve the same way PHP would). [`HierarchyResolver::resolve`] then
//! checks every reference against the already-built [`SymbolTable`] in a
//! second pass, so a forward reference to a not-yet-declared parent works the
//! same way a `use` import already does.
//!
//! **Known gap:** a trait pulling in another trait's methods is not
//! flattened recursively — only a trait's own directly-declared methods are
//! checked for a name collision against traits used alongside it. Revisit
//! before this leaves M2 if nested trait composition needs the same check.
//! A target under `Core` ([`QName::is_core`]) is trusted to exist, same as a
//! `use` import — `mwl-stdlib` doesn't exist yet, so its members can't be
//! checked either.

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use mwl_syntax::ast::{
    ClassMember, ClassMemberKind, Name, NamespaceDecl, Stmt, StmtKind, TraitAdaptationKind,
    UseTraitMember,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::qname::QName;
use crate::resolve::{name_text, qname_segments};
use crate::symbol::{SymbolKind, SymbolTable};

/// One class/interface/trait's resolved links to other declarations. Never
/// built for an enum: ADR 0010 § 3 already rejects `implements` on an enum at
/// parse time, and an enum has no `extends`/trait-use grammar at all.
#[derive(Clone, Debug, Default)]
pub struct ClassLinks {
    /// The superclass (a class has at most one) or the extended interfaces
    /// (an interface may list several) — both live in the same field, since
    /// only the owning [`SymbolKind`] tells them apart, and every consumer
    /// needs the same "walk the parents" operation either way.
    pub extends: Vec<QName>,
    /// The implemented interfaces (class only).
    pub implements: Vec<QName>,
    /// The traits pulled in via `use` (class/trait only), in source order,
    /// deduplicated across however many separate `use` clauses named them.
    pub traits: Vec<QName>,
}

/// Every declaration's resolved [`ClassLinks`], keyed by its [`QName`].
#[derive(Debug, Default)]
pub struct ClassGraph {
    links: FxHashMap<QName, ClassLinks>,
}

impl ClassGraph {
    /// The resolved links for one declared class/interface/trait, if it was
    /// one of those kinds (an enum has no entry).
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&ClassLinks> {
        self.links.get(qname)
    }

    /// How many declarations have a links entry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.links.len()
    }

    /// Whether nothing has been resolved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

/// A name as written at some reference site — kept as text plus a span
/// rather than resolved immediately, since resolution needs the whole
/// [`SymbolTable`] and a reference may point forward to a declaration not
/// collected yet.
#[derive(Clone, Debug)]
struct RawRef {
    text: String,
    span: Span,
}

/// One `Trait::method insteadof Other, ...;` adaptation, with its parts kept
/// as raw text — resolved alongside everything else in
/// [`HierarchyResolver::resolve`].
#[derive(Clone, Debug)]
struct RawInsteadOf {
    winner: RawRef,
    method: String,
    over: Vec<RawRef>,
}

/// One class/interface/trait's raw, not-yet-resolved links, plus the
/// namespace and imports active where it was declared.
#[derive(Debug)]
struct PendingLinks {
    qname: QName,
    /// Whether this is a class, interface or trait declaration — decides
    /// which [`SymbolKind`] an `extends` reference is allowed to name (a
    /// class's superclass must itself be a class; an interface's parents
    /// must themselves be interfaces).
    own_kind: SymbolKind,
    decl_span: Span,
    namespace: Vec<String>,
    imports: FxHashMap<String, QName>,
    extends: Vec<RawRef>,
    implements: Vec<RawRef>,
    trait_refs: Vec<RawRef>,
    insteadof: Vec<RawInsteadOf>,
}

impl PendingLinks {
    fn new(
        qname: QName,
        own_kind: SymbolKind,
        decl_span: Span,
        namespace: Vec<String>,
        imports: FxHashMap<String, QName>,
    ) -> Self {
        Self {
            qname,
            own_kind,
            decl_span,
            namespace,
            imports,
            extends: Vec::new(),
            implements: Vec::new(),
            trait_refs: Vec::new(),
            insteadof: Vec::new(),
        }
    }
}

/// Resolves `extends`/`implements`/trait-use across one or more parsed
/// files into a [`ClassGraph`]. Call [`Self::collect_links`] once per file,
/// then a single closing [`Self::resolve`] once every file sharing the
/// target [`SymbolTable`] has been collected — the same two-step shape as
/// [`crate::resolve::Resolver`], for the same reason: a reference may name a
/// declaration from another file.
#[derive(Debug, Default)]
pub struct HierarchyResolver {
    pending: Vec<PendingLinks>,
    /// Each trait's own directly-declared method names, by its `QName`.
    trait_methods: FxHashMap<QName, FxHashSet<String>>,
}

impl HierarchyResolver {
    /// A resolver with nothing collected yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Walks `stmts`, recording every class/interface/trait's raw
    /// `extends`/`implements`/trait-use references. Recurses into
    /// `namespace { ... }` blocks; a `namespace Name;` statement changes the
    /// namespace and resets the tracked imports for the rest of this call's
    /// statement sequence, matching [`crate::resolve::Resolver::collect_declarations`].
    pub fn collect_links(&mut self, stmts: &[Stmt], src: &SourceFile) {
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
                StmtKind::ClassDecl(decl) => {
                    let mut pending = PendingLinks::new(
                        QName::join(&current_ns, name_text(src, &decl.name)),
                        SymbolKind::Class,
                        decl.name.span,
                        current_ns.clone(),
                        imports.clone(),
                    );
                    if let Some(base) = &decl.extends {
                        pending.extends.push(raw_ref(src, base));
                    }
                    for iface in &decl.implements {
                        pending.implements.push(raw_ref(src, iface));
                    }
                    self.collect_trait_uses(src, &decl.members, &mut pending);
                    self.pending.push(pending);
                }
                StmtKind::InterfaceDecl(decl) => {
                    let mut pending = PendingLinks::new(
                        QName::join(&current_ns, name_text(src, &decl.name)),
                        SymbolKind::Interface,
                        decl.name.span,
                        current_ns.clone(),
                        imports.clone(),
                    );
                    for parent in &decl.extends {
                        pending.extends.push(raw_ref(src, parent));
                    }
                    self.pending.push(pending);
                }
                StmtKind::TraitDecl(decl) => {
                    let qname = QName::join(&current_ns, name_text(src, &decl.name));
                    let methods: FxHashSet<String> = decl
                        .members
                        .iter()
                        .filter_map(|m| match &m.kind {
                            ClassMemberKind::Method(method) => {
                                Some(src.span_text(method.name).unwrap_or_default().to_owned())
                            }
                            _ => None,
                        })
                        .collect();
                    self.trait_methods.insert(qname.clone(), methods);

                    let mut pending = PendingLinks::new(
                        qname,
                        SymbolKind::Trait,
                        decl.name.span,
                        current_ns.clone(),
                        imports.clone(),
                    );
                    self.collect_trait_uses(src, &decl.members, &mut pending);
                    self.pending.push(pending);
                }
                _ => {}
            }
        }
    }

    fn collect_trait_uses(
        &self,
        src: &SourceFile,
        members: &[ClassMember],
        pending: &mut PendingLinks,
    ) {
        for member in members {
            let ClassMemberKind::UseTrait(UseTraitMember {
                traits,
                adaptations,
            }) = &member.kind
            else {
                continue;
            };
            for t in traits {
                pending.trait_refs.push(raw_ref(src, t));
            }
            for adaptation in adaptations {
                let TraitAdaptationKind::InsteadOf { method, over } = &adaptation.kind else {
                    // `as` adaptations are already diagnosed at parse time
                    // (ADR 0015 § 3) — nothing left for this pass to do.
                    continue;
                };
                let Some(winner_name) = &method.trait_name else {
                    continue;
                };
                pending.insteadof.push(RawInsteadOf {
                    winner: raw_ref(src, winner_name),
                    method: src.span_text(method.method).unwrap_or_default().to_owned(),
                    over: over.iter().map(|n| raw_ref(src, n)).collect(),
                });
            }
        }
    }

    /// Checks every reference collected so far against `symbols`, reporting
    /// `E_UNDEFINED_CLASS` for one that names nothing declared or the wrong
    /// kind of declaration, `E_CIRCULAR_INHERITANCE` for an `extends`/
    /// trait-use cycle, and `E_TRAIT_METHOD_CONFLICT` for a trait method
    /// name collision with no `insteadof` naming a winner. Call once, after
    /// every file sharing this `SymbolTable` has run [`Self::collect_links`].
    #[must_use]
    pub fn resolve(self, symbols: &SymbolTable, diags: &mut Diagnostics) -> ClassGraph {
        let mut graph = ClassGraph::default();

        for pending in &self.pending {
            let mut links = ClassLinks::default();

            let extends_kinds = [pending.own_kind];
            for raw in &pending.extends {
                if let Some(qname) = resolve_supertype(raw, pending, symbols, &extends_kinds, diags)
                {
                    links.extends.push(qname);
                }
            }

            let implements_kinds = [SymbolKind::Interface];
            for raw in &pending.implements {
                if let Some(qname) =
                    resolve_supertype(raw, pending, symbols, &implements_kinds, diags)
                {
                    links.implements.push(qname);
                }
            }

            let trait_kinds = [SymbolKind::Trait];
            for raw in &pending.trait_refs {
                if let Some(qname) = resolve_supertype(raw, pending, symbols, &trait_kinds, diags)
                    && !links.traits.contains(&qname)
                {
                    links.traits.push(qname);
                }
            }

            check_trait_conflicts(pending, &links.traits, &self.trait_methods, diags);

            graph.links.insert(pending.qname.clone(), links);
        }

        detect_cycles(&graph, symbols, diags);

        graph
    }
}

fn raw_ref(src: &SourceFile, name: &Name) -> RawRef {
    RawRef {
        text: name_text(src, name).to_owned(),
        span: name.span,
    }
}

/// Resolves an unqualified/qualified/fully-qualified reference the same way
/// PHP would: a leading `\` is already fully qualified; otherwise the first
/// segment is checked against `imports`, and failing that the whole
/// reference is taken as relative to `namespace`.
fn resolve_ref(text: &str, namespace: &[String], imports: &FxHashMap<String, QName>) -> QName {
    if text.starts_with('\\') {
        return QName::parse(text);
    }
    let parsed = QName::parse(text);
    let segments = parsed.segments();
    if let Some(target) = imports.get(segments[0].as_str()) {
        if segments.len() == 1 {
            return target.clone();
        }
        let mut combined = target.segments().to_vec();
        combined.extend(segments[1..].iter().cloned());
        return QName::from_segments(combined);
    }
    let mut combined = namespace.to_vec();
    combined.extend(segments.iter().cloned());
    QName::from_segments(combined)
}

fn describe_kinds(kinds: &[SymbolKind]) -> String {
    kinds
        .iter()
        .map(|k| k.describe())
        .collect::<Vec<_>>()
        .join(" or ")
}

fn resolve_supertype(
    raw: &RawRef,
    pending: &PendingLinks,
    symbols: &SymbolTable,
    expected: &[SymbolKind],
    diags: &mut Diagnostics,
) -> Option<QName> {
    let qname = resolve_ref(&raw.text, &pending.namespace, &pending.imports);
    if qname.is_core() {
        return Some(qname);
    }
    match symbols.get(&qname) {
        Some(sym) if expected.contains(&sym.kind) => Some(qname),
        Some(sym) => {
            diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_CLASS,
                    format!(
                        "`{qname}` is {}, not {}",
                        sym.kind.describe(),
                        describe_kinds(expected)
                    ),
                )
                .with_primary(raw.span, "wrong kind of declaration"),
            );
            None
        }
        None => {
            diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_CLASS,
                    format!("`{qname}` is not declared"),
                )
                .with_primary(raw.span, "no matching declaration"),
            );
            None
        }
    }
}

/// ADR 0015 § 3: a method name declared directly by more than one trait used
/// together needs an `insteadof` naming which trait wins over the rest.
/// Only each trait's own directly-declared methods are checked — see the
/// module docs' known gap about nested trait composition.
fn check_trait_conflicts(
    pending: &PendingLinks,
    traits: &[QName],
    trait_methods: &FxHashMap<QName, FxHashSet<String>>,
    diags: &mut Diagnostics,
) {
    let mut owners: FxHashMap<&str, Vec<&QName>> = FxHashMap::default();
    for trait_name in traits {
        let Some(methods) = trait_methods.get(trait_name) else {
            continue; // a `Core` trait, trusted rather than checked.
        };
        for method in methods {
            owners.entry(method.as_str()).or_default().push(trait_name);
        }
    }

    for (method, owning_traits) in owners {
        if owning_traits.len() < 2 {
            continue;
        }
        let resolved_by_insteadof = pending.insteadof.iter().any(|ins| {
            if ins.method != method {
                return false;
            }
            let winner = resolve_ref(&ins.winner.text, &pending.namespace, &pending.imports);
            let losers: FxHashSet<QName> = ins
                .over
                .iter()
                .map(|r| resolve_ref(&r.text, &pending.namespace, &pending.imports))
                .collect();
            owning_traits
                .iter()
                .all(|t| **t == winner || losers.contains(*t))
        });
        if !resolved_by_insteadof {
            let names = owning_traits
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            diags.report(
                Diagnostic::error(
                    code::E_TRAIT_METHOD_CONFLICT,
                    format!(
                        "`{method}` is declared by more than one trait used here ({names}), \
                         with no `insteadof` naming a winner"
                    ),
                )
                .with_primary(pending.decl_span, "ambiguous trait method")
                .with_help(
                    "add `Trait::method insteadof OtherTrait, ...;` to pick a winner \
                     (ADR 0015 § 3)",
                ),
            );
        }
    }
}

/// Walks every `extends`/trait-use edge looking for a cycle, reporting
/// `E_CIRCULAR_INHERITANCE` at the first back-edge found per traversal.
fn detect_cycles(graph: &ClassGraph, symbols: &SymbolTable, diags: &mut Diagnostics) {
    let mut done: FxHashSet<QName> = FxHashSet::default();
    let mut stack: Vec<QName> = Vec::new();

    for qname in graph.links.keys() {
        if !done.contains(qname) {
            visit(qname, graph, symbols, &mut done, &mut stack, diags);
        }
    }
}

fn visit(
    qname: &QName,
    graph: &ClassGraph,
    symbols: &SymbolTable,
    done: &mut FxHashSet<QName>,
    stack: &mut Vec<QName>,
    diags: &mut Diagnostics,
) {
    if done.contains(qname) {
        return;
    }
    if stack.contains(qname) {
        let start = stack.iter().position(|q| q == qname).unwrap_or(0);
        let mut chain: Vec<String> = stack[start..].iter().map(ToString::to_string).collect();
        chain.push(qname.to_string());
        if let Some(sym) = symbols.get(qname) {
            diags.report(
                Diagnostic::error(
                    code::E_CIRCULAR_INHERITANCE,
                    format!("circular inheritance: {}", chain.join(" -> ")),
                )
                .with_primary(sym.decl_span, "part of this cycle"),
            );
        }
        return;
    }

    stack.push(qname.clone());
    if let Some(links) = graph.get(qname) {
        for parent in links.extends.iter().chain(links.traits.iter()) {
            visit(parent, graph, symbols, done, stack, diags);
        }
    }
    stack.pop();
    done.insert(qname.clone());
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::SourceMap;
    use mwl_syntax::parse_file;

    use super::*;
    use crate::resolve::resolve_file;

    fn resolve(src: &str) -> (ClassGraph, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        (module.graph, diags)
    }

    #[test]
    fn a_class_extends_a_declared_class() {
        let (graph, diags) = resolve("<?mwl\nclass Base {}\nclass Sub extends Base {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("Sub")).unwrap();
        assert_eq!(links.extends, vec![QName::parse("Base")]);
    }

    #[test]
    fn extending_an_undeclared_class_is_diagnosed() {
        let (_graph, diags) = resolve("<?mwl\nclass Sub extends Missing {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn extending_an_interface_from_a_class_is_diagnosed() {
        let (_graph, diags) = resolve("<?mwl\ninterface Shape {}\nclass Sub extends Shape {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn a_class_implements_a_declared_interface() {
        let (graph, diags) = resolve("<?mwl\ninterface Shape {}\nclass Sub implements Shape {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("Sub")).unwrap();
        assert_eq!(links.implements, vec![QName::parse("Shape")]);
    }

    #[test]
    fn a_forward_reference_to_a_later_declaration_resolves() {
        let (graph, diags) = resolve("<?mwl\nclass Sub extends Base {}\nclass Base {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("Sub")).unwrap();
        assert_eq!(links.extends, vec![QName::parse("Base")]);
    }

    #[test]
    fn a_use_import_resolves_an_unqualified_extends() {
        let (graph, diags) = resolve(
            "<?mwl\nnamespace App;\nclass Base {}\nnamespace App\\Http;\nuse App\\Base;\nclass Sub extends Base {}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("App\\Http\\Sub")).unwrap();
        assert_eq!(links.extends, vec![QName::parse("App\\Base")]);
    }

    #[test]
    fn a_two_class_extends_cycle_is_diagnosed() {
        let (_graph, diags) = resolve("<?mwl\nclass A extends B {}\nclass B extends A {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_CIRCULAR_INHERITANCE))
        );
    }

    #[test]
    fn a_class_using_one_trait_has_no_conflict() {
        let (graph, diags) = resolve(
            "<?mwl\ntrait Greets { function hello(): void {} }\nclass Foo { use Greets; }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("Foo")).unwrap();
        assert_eq!(links.traits, vec![QName::parse("Greets")]);
    }

    #[test]
    fn two_traits_declaring_the_same_method_with_no_insteadof_conflicts() {
        let (_graph, diags) = resolve(
            "<?mwl\n\
             trait A { function hello(): void {} }\n\
             trait B { function hello(): void {} }\n\
             class Foo { use A, B; }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TRAIT_METHOD_CONFLICT))
        );
    }

    #[test]
    fn insteadof_resolves_a_trait_method_conflict() {
        let (_graph, diags) = resolve(
            "<?mwl\n\
             trait A { function hello(): void {} }\n\
             trait B { function hello(): void {} }\n\
             class Foo { use A, B { A::hello insteadof B; } }\n",
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_TRAIT_METHOD_CONFLICT)),
            "{diags:?}"
        );
    }
}
