//! The class hierarchy graph: `extends`/`implements` resolved to real
//! [`Symbol`]s (M2 item 1 — see the crate's module docs for what's left
//! after this).
//!
//! There is no trait-use flattening or `insteadof` collision resolution here
//! any more — [ADR 0043](../../../docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)
//! removes `trait` from the language entirely, replacing it with an
//! interface default/private method (shared behavior) and `implements
//! Interface by $field;` delegation (shared state). This module's own
//! superseded trait-use/`insteadof` resolution (`trait_refs`, `insteadof`,
//! `check_trait_conflicts`, `E_TRAIT_METHOD_CONFLICT`, and `ClassLinks`'
//! former `traits` field) was removed along with it; the new default-method/
//! private-method-visibility/`by`-delegation resolution the ADR's own
//! *Consequences* and *Verification* sections call for is a follow-up
//! session's work, not yet built.
//!
//! Built in the same two-pass shape as [`crate::resolve`]: [`HierarchyResolver::collect_links`]
//! walks a file's declarations, recording each one's raw `extends`/
//! `implements` references together with the namespace and `use` imports
//! active at that point (an unqualified reference needs both to resolve the
//! same way PHP would). [`HierarchyResolver::resolve`] then checks every
//! reference against the already-built [`SymbolTable`] in a second pass, so
//! a forward reference to a not-yet-declared parent works the same way a
//! `use` import already does.
//!
//! [`crate::errors`]' exception tree is seeded into every [`ClassGraph`] this
//! module builds, before a single declared link is resolved — see
//! [`seed_exception_tree`] for why that is the graph's job rather than each
//! consumer's.
//!
//! **Known gap:** a target under `Core` ([`QName::is_core`]) is trusted to
//! exist, same as a `use` import — `nvs-stdlib` doesn't exist yet, so its
//! members can't be checked either.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_syntax::ast::{Name, NamespaceDecl, Stmt, StmtKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::qname::QName;
use crate::resolve::{name_text, qname_segments};
use crate::symbol::{SymbolKind, SymbolTable};

/// One class/interface's resolved links to other declarations. Never built
/// for an enum: ADR 0010 § 3 already rejects `implements` on an enum at
/// parse time, and an enum has no `extends` grammar at all.
#[derive(Clone, Debug, Default)]
pub struct ClassLinks {
    /// The superclass (a class has at most one) or the extended interfaces
    /// (an interface may list several) — both live in the same field, since
    /// only the owning [`SymbolKind`] tells them apart, and every consumer
    /// needs the same "walk the parents" operation either way.
    pub extends: Vec<QName>,
    /// The implemented interfaces (class only).
    pub implements: Vec<QName>,
}

/// Every declaration's resolved [`ClassLinks`], keyed by its [`QName`].
#[derive(Debug, Default)]
pub struct ClassGraph {
    links: FxHashMap<QName, ClassLinks>,
}

impl ClassGraph {
    /// The resolved links for one declared class/interface, if it was one of
    /// those kinds (an enum has no entry).
    #[must_use]
    pub fn get(&self, qname: &QName) -> Option<&ClassLinks> {
        self.links.get(qname)
    }

    /// Records one name's links, replacing any already there.
    ///
    /// Exists for the compiler-owned declarations that have no source text to
    /// resolve from — today only [`crate::errors::TREE`], seeded by
    /// [`seed_exception_tree`] before the first declared link is resolved.
    pub fn insert(&mut self, qname: QName, links: ClassLinks) {
        self.links.insert(qname, links);
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

/// One class/interface's raw, not-yet-resolved links, plus the namespace and
/// imports active where it was declared.
#[derive(Debug)]
struct PendingLinks {
    qname: QName,
    /// Whether this is a class or interface declaration — decides which
    /// [`SymbolKind`] an `extends` reference is allowed to name (a class's
    /// superclass must itself be a class; an interface's parents must
    /// themselves be interfaces).
    own_kind: SymbolKind,
    namespace: Vec<String>,
    imports: FxHashMap<String, QName>,
    extends: Vec<RawRef>,
    implements: Vec<RawRef>,
}

impl PendingLinks {
    fn new(
        qname: QName,
        own_kind: SymbolKind,
        namespace: Vec<String>,
        imports: FxHashMap<String, QName>,
    ) -> Self {
        Self {
            qname,
            own_kind,
            namespace,
            imports,
            extends: Vec::new(),
            implements: Vec::new(),
        }
    }
}

/// Resolves `extends`/`implements` across one or more parsed files into a
/// [`ClassGraph`]. Call [`Self::collect_links`] once per file, then a single
/// closing [`Self::resolve`] once every file sharing the target
/// [`SymbolTable`] has been collected — the same two-step shape as
/// [`crate::resolve::Resolver`], for the same reason: a reference may name a
/// declaration from another file.
#[derive(Debug, Default)]
pub struct HierarchyResolver {
    pending: Vec<PendingLinks>,
}

impl HierarchyResolver {
    /// A resolver with nothing collected yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Walks `stmts`, recording every class/interface's raw
    /// `extends`/`implements` references. Recurses into `namespace { ... }`
    /// blocks; a `namespace Name;` statement changes the namespace and
    /// resets the tracked imports for the rest of this call's statement
    /// sequence, matching [`crate::resolve::Resolver::collect_declarations`].
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
                        current_ns.clone(),
                        imports.clone(),
                    );
                    if let Some(base) = &decl.extends {
                        pending.extends.push(raw_ref(src, base));
                    }
                    for iface in &decl.implements {
                        pending.implements.push(raw_ref(src, &iface.name));
                    }
                    self.pending.push(pending);
                }
                StmtKind::InterfaceDecl(decl) => {
                    let mut pending = PendingLinks::new(
                        QName::join(&current_ns, name_text(src, &decl.name)),
                        SymbolKind::Interface,
                        current_ns.clone(),
                        imports.clone(),
                    );
                    for parent in &decl.extends {
                        pending.extends.push(raw_ref(src, parent));
                    }
                    self.pending.push(pending);
                }
                _ => {}
            }
        }
    }

    /// Checks every reference collected so far against `symbols`, reporting
    /// `E_UNDEFINED_CLASS` for one that names nothing declared or the wrong
    /// kind of declaration and `E_CIRCULAR_INHERITANCE` for an `extends`
    /// cycle. Call once, after every file sharing this `SymbolTable` has run
    /// [`Self::collect_links`].
    #[must_use]
    pub fn resolve(self, symbols: &SymbolTable, diags: &mut Diagnostics) -> ClassGraph {
        let mut graph = ClassGraph::default();
        seed_exception_tree(&mut graph);

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

            graph.links.insert(pending.qname.clone(), links);
        }

        detect_cycles(&graph, symbols, diags);

        graph
    }
}

/// Puts [`crate::errors::TREE`]'s own `extends` links into `graph` before any
/// declared one, so `resolve_method`/`resolve_property` walk from a user class
/// through `LogicError` to `Throwable` with the ordinary parent walk and no
/// special case anywhere above this line.
///
/// Seeded here rather than left to each consumer because the graph is what
/// every one of them already asks; see [`crate::errors`] for why the tree has
/// no source declaration to collect from instead.
pub fn seed_exception_tree(graph: &mut ClassGraph) {
    for (name, parent) in crate::errors::TREE {
        let links = ClassLinks {
            extends: parent.iter().map(|p| QName::parse(p)).collect(),
            implements: Vec::new(),
        };
        graph.insert(QName::parse(name), links);
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
///
/// Exported (rather than `pub(crate)`) so `nvs-types` can resolve a type
/// atom's `Name` the same way every resolver in this crate already resolves
/// an `extends`/`implements`/alias reference, instead of duplicating this
/// logic.
pub fn resolve_ref(text: &str, namespace: &[String], imports: &FxHashMap<String, QName>) -> QName {
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
    if qname.is_core() || qname.is_reserved_global_interface() || qname.is_reserved_global_class() {
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

/// Whether `qname` is provably known to satisfy `target` — walking every
/// `extends`/`implements` ancestor, the same shape
/// [`crate::members::member_declared`] and every `nvs-types` signature
/// lookup already walk, generalised here to a plain reachability question
/// rather than a member lookup. `target` itself need not have a
/// [`ClassGraph`] entry — a reserved global interface like ADR 0013's
/// `Comparable` never does, since equality against it is checked before ever
/// calling [`ClassGraph::get`] on it.
///
/// **Reflexive:** a name satisfies itself, in zero steps. That is the answer
/// every caller wants and three of them already spelled for themselves before
/// calling (`is_throwable_shaped`, `is_visible_from`, and
/// `classes_are_unrelated`, which returns early on equal names) — and the one
/// that did not, `nvs_types::expr::operators::require_stringable`, was
/// refusing `echo $s` on a `Stringable $s` for it: a value typed at the
/// interface provably has the member the interface declares, which is the
/// whole of what this predicate is asked. The name still reads
/// `implements_interface` because "does `qname` reach `target`'s members" is
/// what all five callers mean by it; the zero-step case is simply the
/// shortest walk, not a different question.
#[must_use]
pub fn implements_interface(qname: &QName, target: &QName, graph: &ClassGraph) -> bool {
    if qname == target {
        return true;
    }
    let mut seen = FxHashSet::default();
    implements_interface_rec(qname, target, graph, &mut seen)
}

fn implements_interface_rec(
    qname: &QName,
    target: &QName,
    graph: &ClassGraph,
    seen: &mut FxHashSet<QName>,
) -> bool {
    if !seen.insert(qname.clone()) {
        return false;
    }
    let Some(links) = graph.get(qname) else {
        return false;
    };
    links
        .extends
        .iter()
        .chain(links.implements.iter())
        .any(|parent| parent == target || implements_interface_rec(parent, target, graph, seen))
}

/// Walks every `extends` edge looking for a cycle, reporting
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
        for parent in &links.extends {
            visit(parent, graph, symbols, done, stack, diags);
        }
    }
    stack.pop();
    done.insert(qname.clone());
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::SourceMap;
    use nvs_syntax::parse_file;

    use super::*;
    use crate::resolve::resolve_file;

    fn resolve(src: &str) -> (ClassGraph, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        (module.graph, diags)
    }

    #[test]
    fn a_class_extends_a_declared_class() {
        let (graph, diags) = resolve("<?nvs\nclass Base {}\nclass Sub extends Base {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("Sub")).unwrap();
        assert_eq!(links.extends, vec![QName::parse("Base")]);
    }

    #[test]
    fn extending_an_undeclared_class_is_diagnosed() {
        let (_graph, diags) = resolve("<?nvs\nclass Sub extends Missing {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn extending_an_interface_from_a_class_is_diagnosed() {
        let (_graph, diags) = resolve("<?nvs\ninterface Shape {}\nclass Sub extends Shape {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS))
        );
    }

    #[test]
    fn a_class_implements_a_declared_interface() {
        let (graph, diags) = resolve("<?nvs\ninterface Shape {}\nclass Sub implements Shape {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("Sub")).unwrap();
        assert_eq!(links.implements, vec![QName::parse("Shape")]);
    }

    #[test]
    fn a_forward_reference_to_a_later_declaration_resolves() {
        let (graph, diags) = resolve("<?nvs\nclass Sub extends Base {}\nclass Base {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("Sub")).unwrap();
        assert_eq!(links.extends, vec![QName::parse("Base")]);
    }

    #[test]
    fn a_use_import_resolves_an_unqualified_extends() {
        let (graph, diags) = resolve(
            "<?nvs\nnamespace App;\nclass Base {}\nnamespace App\\Http;\nuse App\\Base;\nclass Sub extends Base {}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("App\\Http\\Sub")).unwrap();
        assert_eq!(links.extends, vec![QName::parse("App\\Base")]);
    }

    #[test]
    fn a_two_class_extends_cycle_is_diagnosed() {
        let (_graph, diags) = resolve("<?nvs\nclass A extends B {}\nclass B extends A {}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_CIRCULAR_INHERITANCE))
        );
    }

    #[test]
    fn implementing_the_reserved_comparable_interface_needs_no_declaration() {
        let (graph, diags) = resolve("<?nvs\nclass Money implements Comparable {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        let links = graph.get(&QName::parse("Money")).unwrap();
        assert_eq!(links.implements, vec![QName::parse("Comparable")]);
    }

    #[test]
    fn implements_interface_finds_a_directly_implemented_interface() {
        let (graph, diags) = resolve("<?nvs\nclass Money implements Comparable {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(implements_interface(
            &QName::parse("Money"),
            &QName::parse("Comparable"),
            &graph
        ));
    }

    #[test]
    fn implements_interface_walks_up_a_superclass() {
        let (graph, diags) =
            resolve("<?nvs\nclass Money implements Comparable {}\nclass Cents extends Money {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(implements_interface(
            &QName::parse("Cents"),
            &QName::parse("Comparable"),
            &graph
        ));
    }

    #[test]
    fn implements_interface_walks_an_interface_extends_chain() {
        let (graph, diags) = resolve(
            "<?nvs\ninterface Shape extends Comparable {}\nclass Box implements Shape {}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(implements_interface(
            &QName::parse("Box"),
            &QName::parse("Comparable"),
            &graph
        ));
    }

    #[test]
    fn a_class_extending_the_reserved_global_exception_resolves_with_no_declaration() {
        // Spec § 10: `Throwable` is a global class with no `nvs-hir`
        // declaration of its own — trusted the same way `Core`'s classes are.
        let (graph, diags) = resolve("<?nvs\nclass MyError extends Throwable {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(implements_interface(
            &QName::parse("MyError"),
            &QName::parse("Throwable"),
            &graph
        ));
    }

    #[test]
    fn the_exception_tree_s_own_links_are_in_the_graph_with_nothing_declared() {
        // A user class reaches the root through the seeded links, with the
        // ordinary parent walk and no special case — which is why every
        // consumer can just ask the graph.
        let (graph, diags) = resolve("<?nvs\nclass MyError extends TimeoutError {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        for ancestor in ["TimeoutError", "RuntimeError", "Throwable"] {
            assert!(
                implements_interface(&QName::parse("MyError"), &QName::parse(ancestor), &graph),
                "MyError should reach {ancestor}"
            );
        }
    }

    #[test]
    fn implements_interface_is_false_when_unrelated() {
        let (graph, diags) = resolve("<?nvs\nclass Plain {}\n");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(!implements_interface(
            &QName::parse("Plain"),
            &QName::parse("Comparable"),
            &graph
        ));
    }
}
