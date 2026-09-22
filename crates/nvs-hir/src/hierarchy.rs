//! The class hierarchy graph: `extends`/`implements` resolved to real
//! [`crate::Symbol`]s (M2 item 1 — see the crate's module docs for what's left
//! after this).
//!
//! There is no trait-use flattening or `insteadof` collision resolution here:
//! `rule:classes/no-traits`
//! leaves the language no `trait` at all, offering instead an
//! interface default/private method (shared behavior) and `implements
//! Interface by $field;` delegation (shared state). Neither of those is this
//! module's business either: a default method's body and a private one's visibility are
//! `nvs_types`', and § 4's `by $field` forwards are resolved in
//! `nvs_types::conformance` and emitted in `nvs_ir::lower`, both of which need
//! the signature table this pass runs before. This module's own share is the
//! edge — the resolved `implements` target the delegation clause hangs off.
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
//! The graph answers in two directions. [`implements_interface`] asks it
//! downward — does this one name reach that one — and every member lookup in
//! `nvs-types` rides on that. [`implementors`] asks it upward — which names
//! reach *this* one — which is `rule:programs/implementing`'s `Core\Program::implementing<T>()`
//! and, through the same enumeration, `rule:routing/routes-are-compiled-not-registered`'s route table. The upward
//! question is why a link record carries [`ClassLinks::concrete`]: it is the
//! only one whose answer excludes an abstract class.
//!
//! [`crate::errors`]' exception tree is seeded into every [`ClassGraph`] this
//! module builds, before a single declared link is resolved — see
//! [`seed_exception_tree`] for why that is the graph's job rather than each
//! consumer's.
//!
//! **A target under `Core` ([`QName::is_core`]) is resolved against a roster
//! the caller hands in**, so every link error still comes from this one pass.
//! The roster arrives as a [`CoreRoster`] rather than being read here because
//! this crate depends on `nvs-diagnostics` and `nvs-syntax` and nothing else,
//! which is the graph position that makes a class link resolvable before the
//! stdlib exists. `nvs_stdlib::registry::link_targets` is what the front ends
//! build one from, and that function owns the bound on which names are in it;
//! a caller with no stdlib in hand passes [`CoreRoster::Trusted`], never an
//! empty slice.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_syntax::ast::{Modifier, Name, NamespaceDecl, Stmt, StmtKind};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::imports::{self, ImportSite};
use crate::qname::QName;
use crate::resolve::{name_text, qname_segments};
use crate::symbol::{SymbolKind, SymbolTable};

/// Which names the `Core` namespace declares, as the link pass is told them.
///
/// `Core` is compiler-owned (`rule:core-api/reserved-namespace`), so no source
/// file declares a name under it and the [`SymbolTable`] this pass checks
/// against can never hold one. A roster is therefore the only authority there
/// is, and it arrives from the caller rather than from `nvs_stdlib::registry`
/// directly: this crate depends on `nvs-diagnostics` and `nvs-syntax` and
/// nothing else, which is the graph position that makes a class link
/// resolvable before the stdlib exists.
///
/// **There are two cases and no third, because an empty roster is a real
/// answer.** A caller holding the stdlib passes [`Self::Names`], and a link
/// naming something outside it is refused; a caller with no stdlib in hand —
/// [`crate::resolve::resolve_file`] and every fixture helper that resolves a
/// file on its own — passes [`Self::Trusted`], and every `Core` name is taken
/// on faith the way a `use` import is. A bare slice could not tell those
/// apart: `&[]` would mean "the `Core` namespace declares nothing" and refuse
/// every link in those fixtures.
#[derive(Clone, Copy, Debug, Default)]
pub enum CoreRoster<'a> {
    /// Every name under `Core` exists. What a caller that cannot see the
    /// stdlib's roster is asking for, and never a claim that the roster is
    /// empty.
    #[default]
    Trusted,
    /// Exactly these names exist, fully qualified and spelled as source writes
    /// them (`Core\Arr`). `nvs_stdlib::registry::class_names` is what builds
    /// one.
    Names(&'a [&'a str]),
}

impl CoreRoster<'_> {
    /// The names this roster holds, spelled as source writes them, and empty
    /// for [`Self::Trusted`]: a roster that takes every name on faith can
    /// list none, which is the right answer for a reader offering one of
    /// them — a fixture with no stdlib in hand has no `Core\Request` to
    /// import.
    #[must_use]
    pub const fn names(&self) -> &[&str] {
        match self {
            Self::Trusted => &[],
            Self::Names(names) => names,
        }
    }

    /// Whether this roster says `qname` exists. Matched without regard to
    /// ASCII case, the same comparison [`QName::is_core`] makes on the first
    /// segment, so one roster answers every spelling of a name.
    #[must_use]
    pub fn holds(&self, qname: &QName) -> bool {
        match self {
            Self::Trusted => true,
            Self::Names(names) => {
                let written = qname.to_string();
                names.iter().any(|name| name.eq_ignore_ascii_case(&written))
            }
        }
    }
}

/// One class/interface's resolved links to other declarations. Never built
/// for an enum: `rule:enums/no-class-machinery` already rejects `implements` on an enum at
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
    /// Whether this is a class the program can instantiate — a `class`
    /// declaration carrying no `abstract` modifier. An interface is never
    /// one.
    ///
    /// It lives on the edge record rather than on [`crate::Symbol`] because
    /// the one question that asks it — [`implementors`], `rule:programs/implementing`'s
    /// "non-abstract classes implementing `T`" — is already walking this
    /// graph for the `implements` half of the same answer, and a second
    /// table consulted per candidate would be a second place for the two to
    /// disagree.
    pub concrete: bool,
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
    /// [`ClassLinks::concrete`], read off the declaration's own modifiers
    /// here because this is the last pass holding them.
    concrete: bool,
    namespace: Vec<String>,
    imports: FxHashMap<String, QName>,
    /// Where a `use` line goes for a name this declaration's clauses write,
    /// read while the file's statements were still in hand
    /// (`crate::imports::site_in`): the undeclared-name diagnostic a clause
    /// can raise carries the import that would resolve it, and by the time
    /// [`HierarchyResolver::resolve`] raises one the statements are gone.
    import_site: Option<ImportSite>,
    extends: Vec<RawRef>,
    implements: Vec<RawRef>,
}

impl PendingLinks {
    fn new(
        qname: QName,
        own_kind: SymbolKind,
        concrete: bool,
        namespace: Vec<String>,
        imports: FxHashMap<String, QName>,
        import_site: Option<ImportSite>,
    ) -> Self {
        Self {
            qname,
            own_kind,
            concrete,
            namespace,
            imports,
            import_site,
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
pub struct HierarchyResolver<'a> {
    pending: Vec<PendingLinks>,
    core: CoreRoster<'a>,
}

impl<'a> HierarchyResolver<'a> {
    /// A resolver with nothing collected yet, holding the roster every link
    /// under `Core` is resolved against. See [`CoreRoster`] for what a caller
    /// with no stdlib in hand passes.
    #[must_use]
    pub fn new(core: CoreRoster<'a>) -> Self {
        Self {
            pending: Vec::new(),
            core,
        }
    }

    /// Walks `stmts`, recording every class/interface's raw
    /// `extends`/`implements` references. Recurses into `namespace { ... }`
    /// blocks; a `namespace Name;` statement changes the namespace and
    /// resets the tracked imports for the rest of this call's statement
    /// sequence, matching [`crate::resolve::Resolver::collect_declarations`].
    pub fn collect_links(&mut self, stmts: &[Stmt], src: &SourceFile) {
        self.collect_in(stmts, src, &[], false);
    }

    /// `bracketed` says whether `stmts` is the block of a `namespace Name {}`
    /// rather than the file, which is what decides where a `use` line for a
    /// clause's undeclared name would go (`crate::imports::site_in`).
    fn collect_in(
        &mut self,
        stmts: &[Stmt],
        src: &SourceFile,
        namespace: &[String],
        bracketed: bool,
    ) {
        let mut current_ns: Vec<String> = namespace.to_vec();
        let mut imports: FxHashMap<String, QName> = FxHashMap::default();

        for stmt in stmts {
            match &stmt.kind {
                StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                    let new_ns = name
                        .as_ref()
                        .map_or_else(Vec::new, |n| qname_segments(src, n));
                    match body {
                        Some(block) => self.collect_in(&block.stmts, src, &new_ns, true),
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
                        !decl.modifiers.contains(&Modifier::Abstract),
                        current_ns.clone(),
                        imports.clone(),
                        imports::site_in(stmts, src, stmt.span.start, bracketed),
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
                        false,
                        current_ns.clone(),
                        imports.clone(),
                        imports::site_in(stmts, src, stmt.span.start, bracketed),
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
            let mut links = ClassLinks {
                concrete: pending.concrete,
                ..ClassLinks::default()
            };

            let extends_kinds = [pending.own_kind];
            for raw in &pending.extends {
                if let Some(qname) =
                    resolve_supertype(raw, pending, symbols, self.core, &extends_kinds, diags)
                {
                    links.extends.push(qname);
                }
            }

            let implements_kinds = [SymbolKind::Interface];
            for raw in &pending.implements {
                if let Some(qname) =
                    resolve_supertype(raw, pending, symbols, self.core, &implements_kinds, diags)
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
            // Every § 10 entry is a class a program throws, so every one of
            // them is instantiable; the tree holds no abstract shape.
            concrete: true,
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

/// Resolves a written reference to the name it means, per
/// `rule:statements/a-qualified-name-is-absolute`:
/// **a name with a separator in it is absolute** and is returned as written,
/// consulting neither `namespace` nor `imports`; a name without one is a
/// short name, looked up in `imports` and failing that joined onto
/// `namespace`. There is no third step — a short name in neither does not
/// fall back to the root, because `rule:classes/no-free-functions-or-constants` removed the free functions and
/// constants PHP's fallback existed for (§ 2).
///
/// This is deliberately **not** PHP's rule, which read a qualified name as
/// relative to the current namespace and gave a leading `\` the job of
/// escaping that. The leading `\` does not parse at all
/// ([`nvs_diagnostics::code::E_LEADING_BACKSLASH_UNSUPPORTED`]), so a name
/// arriving here has none to strip, and
/// [`relative_spelling`] is what turns the one construct whose meaning
/// changed into a diagnostic that names its replacement.
///
/// Exported (rather than `pub(crate)`) so `nvs-types` can resolve a type
/// atom's `Name` the same way every resolver in this crate already resolves
/// an `extends`/`implements`/alias reference, instead of duplicating this
/// logic. **Keep it the only place that decides what a name means** — a
/// second one is the state `rule:statements/one-function-resolves-every-name` exists to prevent.
pub fn resolve_ref(text: &str, namespace: &[String], imports: &FxHashMap<String, QName>) -> QName {
    let parsed = QName::parse(text);
    if parsed.segments().len() > 1 {
        return parsed;
    }
    if let Some(target) = imports.get(parsed.segments()[0].as_str()) {
        return target.clone();
    }
    QName::join(namespace, parsed.segments()[0].as_str())
}

/// The name PHP's rule would have reached for a reference [`resolve_ref`] has
/// just failed to resolve — `Models\User` inside `namespace App;` meaning
/// `App\Models\User`. `None` unless the reference is qualified and the
/// namespace is non-empty, which is exactly the case `rule:statements/a-qualified-name-is-absolute` changes the
/// meaning of.
///
/// Callers use it to upgrade an undeclared-name error into
/// [`nvs_diagnostics::code::E_RELATIVE_QUALIFIED_NAME`], which names the
/// absolute spelling instead of leaving a generic "not declared" to be worked
/// backwards from — `rule:statements/a-qualified-name-is-absolute`.
#[must_use]
pub fn relative_spelling(text: &str, namespace: &[String]) -> Option<QName> {
    if namespace.is_empty() || !text.contains('\\') {
        return None;
    }
    let mut segments = namespace.to_vec();
    segments.extend_from_slice(QName::parse(text).segments());
    Some(QName::from_segments(segments))
}

/// The diagnostic for a class/interface/enum reference that resolved to
/// nothing — built here, once, so that every site reporting it makes the same
/// `rule:statements/a-qualified-name-is-absolute` distinction rather than a copy per site drifting apart.
///
/// A class/interface/enum reference that resolved to nothing, and what the
/// diagnostic for it needs to carry the fix.
///
/// The statements and the source are the file the name was written in, which
/// is where its `use` line would go (`crate::imports::import_site`); a caller
/// that has the site in hand already, or has none to give, reaches
/// [`undeclared_name_at`] directly.
#[derive(Clone, Copy, Debug)]
pub struct Undeclared<'a> {
    /// What the reference resolved to, and failed to find.
    pub qname: &'a QName,
    /// The reference as source wrote it.
    pub text: &'a str,
    /// Where it was written.
    pub span: Span,
    /// The namespace in force there.
    pub namespace: &'a [String],
    /// The top-level statements of the file it was written in.
    pub stmts: &'a [Stmt],
    /// That file.
    pub src: &'a SourceFile,
}

/// The diagnostic for a class/interface/enum reference that resolved to
/// nothing, with the `use` line that would resolve it where one is known.
///
/// `core` is every `Core` type a caller can name — `nvs_stdlib::registry`'s
/// roster where the caller holds the stdlib, and nothing where it does not —
/// and is what lets `Request` offer `use Core\Request;`.
///
/// [`undeclared_name_at`] is the rest of this function, given the site.
#[must_use]
pub fn undeclared_name(at: Undeclared<'_>, symbols: &SymbolTable, core: &[&str]) -> Diagnostic {
    undeclared_name_at(
        at.qname,
        at.text,
        at.span,
        at.namespace,
        symbols,
        core,
        imports::import_site(at.stmts, at.src, at.span.start),
    )
}

/// Ordinarily [`nvs_diagnostics::code::E_UNDEFINED_CLASS`]. Where the
/// reference is the one construct `rule:statements/a-qualified-name-is-absolute` changed the meaning of — a
/// qualified name inside a namespace, which PHP read as relative — **and**
/// that relative reading names something that *is* declared, it is
/// [`nvs_diagnostics::code::E_RELATIVE_QUALIFIED_NAME`] instead, naming the
/// absolute spelling. A converted PHP file hits this on its first such name
/// and is told what to write, rather than being told a class it can see is
/// missing.
///
/// **An unqualified name that resolved through the namespace alone carries a
/// fix per type it could have meant**: every class, interface or enum in
/// `symbols` or `core` whose last segment is the written name
/// ([`crate::imports::candidates`]), each as the `use` line inserted at
/// `site` (`rule:ide/an-undeclared-name-offers-its-import`). One candidate is
/// a machine-applicable fix; several are each offered for a person to choose
/// between, because the checker cannot. A qualified name is absolute
/// (`rule:statements/a-qualified-name-is-absolute`) and no import changes what
/// it means; a short name an existing `use` already resolves to something
/// undeclared is that import's mistake, and a second `use` of the same short
/// name would not compile — so neither is offered one. With no site there is
/// no fix, and the diagnostic is what it was.
#[must_use]
pub fn undeclared_name_at(
    qname: &QName,
    text: &str,
    span: Span,
    namespace: &[String],
    symbols: &SymbolTable,
    core: &[&str],
    site: Option<ImportSite>,
) -> Diagnostic {
    if let Some(relative) = relative_spelling(text, namespace)
        && symbols.contains(&relative)
    {
        return Diagnostic::error(
            code::E_RELATIVE_QUALIFIED_NAME,
            format!("`{qname}` is not declared"),
        )
        .with_primary(span, "a qualified name is read from the root")
        .with_help(format!(
            "`{relative}` is what is declared. A name with a `\\` in it is absolute in Novis, \
             where PHP would have read this one as relative to the enclosing namespace \
             (`rule:statements/a-qualified-name-is-absolute`) — write `{relative}`, or `use {relative};` and write `{}`",
            relative.short_name()
        ));
    }
    let mut diagnostic = Diagnostic::error(
        code::E_UNDEFINED_CLASS,
        format!("`{qname}` is not declared"),
    )
    .with_primary(span, "no matching declaration");
    let through_namespace = !text.contains('\\') && QName::join(namespace, text) == *qname;
    if let Some(site) = site.filter(|_| through_namespace) {
        let candidates = imports::candidates(text, symbols, core.iter().copied());
        let unique = candidates.len() == 1;
        for candidate in &candidates {
            let message = format!("import `{candidate}`");
            let line = site.use_line(candidate);
            diagnostic = if unique {
                diagnostic.with_fix(site.span(span.file), line, message)
            } else {
                diagnostic.with_unsafe_fix(site.span(span.file), line, message)
            };
        }
    }
    diagnostic
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
    core: CoreRoster<'_>,
    expected: &[SymbolKind],
    diags: &mut Diagnostics,
) -> Option<QName> {
    let qname = resolve_ref(&raw.text, &pending.namespace, &pending.imports);
    // A name under `Core` is asked of the roster, since `symbols` cannot hold
    // one. The two reserved global rosters keep their spelling test instead:
    // `crate::interfaces::RESERVED` and `crate::errors::TREE` are data this
    // crate already carries, so nothing has to be handed in for them.
    if qname.is_core() {
        if core.holds(&qname) {
            return Some(qname);
        }
        diags.report(
            Diagnostic::error(
                code::E_UNDEFINED_CLASS,
                format!("`{qname}` is not a class the `Core` namespace declares"),
            )
            .with_primary(raw.span, "no such `Core` class"),
        );
        return None;
    }
    if qname.is_reserved_global_interface() || qname.is_reserved_global_class() {
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
            diags.report(undeclared_name_at(
                &qname,
                &raw.text,
                raw.span,
                &pending.namespace,
                symbols,
                core.names(),
                pending.import_site,
            ));
            None
        }
    }
}

/// Whether `qname` is provably known to satisfy `target` — walking every
/// `extends`/`implements` ancestor, the same shape
/// [`crate::members::member_declared`] and every `nvs-types` signature
/// lookup already walk, generalised here to a plain reachability question
/// rather than a member lookup. `target` itself need not have a
/// [`ClassGraph`] entry — a reserved global interface like `rule:classes/comparable`'s
/// `Comparable` never does, since equality against it is checked before ever
/// calling [`ClassGraph::get`] on it.
///
/// **Reflexive:** a name satisfies itself, in zero steps. That is the answer
/// every caller wants — `is_throwable_shaped`, `is_visible_from` and
/// `classes_are_unrelated` spell it for themselves before calling (the last
/// returns early on equal names), and
/// `nvs_types::expr::operators::require_stringable` leans on it here so that
/// `echo $s` on a `Stringable $s` is accepted: a value typed at the
/// interface provably has the member the interface declares, which is the
/// whole of what this predicate is asked. The name still reads
/// `implements_interface` because "does `qname` reach `target`'s members" is
/// what every caller means by it; the zero-step case is simply the
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

/// Every non-abstract class in `graph` that satisfies `target`, sorted by
/// fully-qualified name — `rule:programs/implementing`'s enumeration, and the list
/// `Core\Program::implementing<T>()` expands to one `new` expression per
/// entry of.
///
/// The sort is what the ADR asks for by name: the order must not depend on
/// filesystem enumeration, since the same program compiled on two machines
/// otherwise runs its modules in two orders. It compares segments rather
/// than the rendered string, matching [`crate::AutoloadMap::render`]'s own
/// ordering, so `Acme\App` sorts against `AcmeApp` the same way in both.
///
/// `target` itself never appears in the answer even though
/// [`implements_interface`] is reflexive: an interface is not a class, so
/// its own entry carries [`ClassLinks::concrete`] `false`. Neither does a
/// class that reaches `target` only through an abstract intermediate — the
/// intermediate is filtered, not the walk through it.
///
/// The answer is over the declarations the graph *holds*. What makes that
/// "every class the program declares" rather than "every class something
/// required" is § 3's scan: [`crate::AutoloadMap::enumerate`] names the
/// files, `crate::requires` loads them, and this runs over the graph that
/// results.
#[must_use]
pub fn implementors(target: &QName, graph: &ClassGraph) -> Vec<QName> {
    let mut found: Vec<QName> = graph
        .links
        .iter()
        .filter(|(qname, links)| links.concrete && implements_interface(qname, target, graph))
        .map(|(qname, _)| qname.clone())
        .collect();
    found.sort_by(|a, b| a.segments().cmp(b.segments()));
    found
}

/// Walks every `extends` edge looking for a cycle, reporting
/// `E_CIRCULAR_INHERITANCE` at the first back-edge found per traversal.
pub(crate) fn detect_cycles(graph: &ClassGraph, symbols: &SymbolTable, diags: &mut Diagnostics) {
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
    use crate::resolve::{Resolver, resolve_file};

    fn resolve(src: &str) -> (ClassGraph, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        (module.graph, diags)
    }

    /// [`resolve`] with the roster named at the call, which
    /// [`resolve_file`] cannot do: that door trusts `Core` by construction.
    fn links(core: CoreRoster<'_>, src: &str) -> (ClassGraph, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let mut resolver = Resolver::new();
        resolver.collect_declarations(&stmts, map.file(file), &mut diags);
        resolver.resolve_imports(&mut diags);
        let mut hierarchy = HierarchyResolver::new(core);
        hierarchy.collect_links(&stmts, map.file(file));
        let module = resolver.into_module();
        let graph = hierarchy.resolve(&module.symbols, &mut diags);
        (graph, diags)
    }

    /// `rule:core-api/reserved-namespace` makes `Core` compiler-owned, so no
    /// declaration can put a name there and the [`SymbolTable`] can never
    /// answer for one. The roster the caller hands in is therefore the whole
    /// authority: a link naming something on it resolves into the graph, and
    /// one naming anything else is `E_UNDEFINED_CLASS` from this pass rather
    /// than a spelling silently trusted.
    ///
    /// The third case is what a bare slice could not express. The same source
    /// the roster refuses is accepted under [`CoreRoster::Trusted`], so a
    /// caller with no stdlib in hand is not reading an empty roster as "the
    /// `Core` namespace declares nothing".
    #[test]
    fn a_core_name_the_roster_lacks_is_refused_by_the_link_pass() {
        const ROSTER: &[&str] = &[r"Core\Arr"];
        let listed = "<?nvs\nclass Listish implements Core\\Arr {}\n";
        let missing = "<?nvs\nclass Listish implements Core\\Arrr {}\n";
        let declared = QName::parse("Listish");

        let (graph, diags) = links(CoreRoster::Names(ROSTER), listed);
        assert!(!diags.has_errors(), "a rostered name resolves: {diags:?}");
        assert_eq!(
            graph
                .get(&declared)
                .expect("the class has links")
                .implements,
            vec![QName::parse(r"Core\Arr")]
        );

        let (graph, diags) = links(CoreRoster::Names(ROSTER), missing);
        assert!(
            diags.iter().any(
                |d| d.code == Some(code::E_UNDEFINED_CLASS) && d.message.contains(r"Core\Arrr")
            ),
            "a name the roster lacks is refused: {diags:?}"
        );
        assert!(
            graph
                .get(&declared)
                .expect("the class has links")
                .implements
                .is_empty(),
            "a refused link is left out of the graph"
        );

        let (_graph, diags) = links(CoreRoster::Trusted, missing);
        assert!(
            !diags.has_errors(),
            "`Trusted` is not an empty roster: {diags:?}"
        );
    }

    /// `rule:statements/a-qualified-name-is-absolute`, as the cases the rule has and nothing between
    /// them. A name with a separator is returned as written no matter what
    /// namespace or imports surround it — the case PHP resolved relative, and
    /// the whole of what this ADR changed. A name without one is looked up in
    /// the imports first and the enclosing namespace second. Asserted with a
    /// namespace *and* an import in force at every call, so a rule that
    /// consulted either one for a qualified name would have to be wrong in
    /// two directions at once to pass.
    #[test]
    fn a_qualified_name_is_absolute_and_a_short_name_is_not() {
        let ns = vec!["App".to_owned()];
        let mut imports = FxHashMap::default();
        imports.insert("User".to_owned(), QName::parse(r"Vendor\Lib\User"));

        // Qualified: neither the namespace nor the import is consulted, and
        // the first segment colliding with an import name changes nothing.
        assert_eq!(
            resolve_ref(r"Models\User", &ns, &imports),
            QName::parse(r"Models\User")
        );
        assert_eq!(
            resolve_ref(r"User\Inner", &ns, &imports),
            QName::parse(r"User\Inner")
        );
        assert_eq!(
            resolve_ref(r"App\Models\User", &ns, &imports),
            QName::parse(r"App\Models\User")
        );

        // Short: the import wins where there is one.
        assert_eq!(
            resolve_ref("User", &ns, &imports),
            QName::parse(r"Vendor\Lib\User")
        );
        // …and the enclosing namespace where there is not. § 2: no third step,
        // so a root-level `Throwable` is `App\Throwable` here and reaching the
        // real one takes a `use`.
        assert_eq!(
            resolve_ref("Post", &ns, &imports),
            QName::parse(r"App\Post")
        );
        assert_eq!(
            resolve_ref("Throwable", &ns, &imports),
            QName::parse(r"App\Throwable")
        );
        // At the root namespace a short name is already the whole name.
        assert_eq!(
            resolve_ref("Throwable", &[], &imports),
            QName::parse("Throwable")
        );
    }

    /// The migration diagnostic's input (`rule:statements/a-qualified-name-is-absolute`): what PHP's rule would
    /// have reached, offered only for the construct whose meaning changed.
    #[test]
    fn a_relative_spelling_exists_only_for_a_qualified_name_inside_a_namespace() {
        let ns = vec!["App".to_owned()];
        assert_eq!(
            relative_spelling(r"Models\User", &ns),
            Some(QName::parse(r"App\Models\User"))
        );
        // A short name is not the changed construct — it still resolves
        // against the namespace, so there is nothing to suggest.
        assert_eq!(relative_spelling("User", &ns), None);
        // At the root there is no relative reading to have meant.
        assert_eq!(relative_spelling(r"Models\User", &[]), None);
    }

    /// `rule:statements/a-qualified-name-is-absolute`'s migration diagnostic. `Models\Module` inside
    /// `namespace App;` is the one construct § 1 changed the meaning of, and
    /// the thing it named in PHP *is* declared here — so the report names
    /// `App\Models\Module` rather than saying a class the author can see is
    /// missing. The second half is what keeps the distinction honest: a
    /// qualified name that resolves neither way is the ordinary `E0303`, so
    /// the new code cannot become a synonym for "unresolved".
    #[test]
    fn a_relative_qualified_name_is_told_its_absolute_spelling() {
        let (_, diags) = resolve(concat!(
            "<?nvs\n",
            "namespace App\\Models;\n",
            "interface Module {}\n",
            "namespace App;\n",
            "class Thing implements Models\\Module {}\n",
        ));
        let relative: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_RELATIVE_QUALIFIED_NAME))
            .collect();
        assert_eq!(relative.len(), 1, "expected the § 5 report: {diags:?}");
        assert!(
            relative[0]
                .notes
                .iter()
                .any(|n| n.contains(r"App\Models\Module")),
            "the help names the absolute spelling: {:?}",
            relative[0].notes
        );

        // Neither reading names anything: the ordinary undeclared-class error.
        let (_, diags) = resolve(concat!(
            "<?nvs\n",
            "namespace App;\n",
            "class Thing implements Models\\Missing {}\n",
        ));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_CLASS)),
            "expected the ordinary error: {diags:?}"
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_RELATIVE_QUALIFIED_NAME)),
            "nothing to suggest, so nothing suggested: {diags:?}"
        );
    }

    /// `rule:statements/no-fallback-to-the-root-namespace`: a short name resolves through the imports then the
    /// enclosing namespace and stops, so the reserved exception tree is
    /// reached from inside a namespace by importing it — and only by
    /// importing it. Both halves are asserted, because § 2's price is only
    /// paid honestly if the import is what buys the name.
    #[test]
    fn a_root_level_name_needs_an_import_from_inside_a_namespace() {
        let (_, diags) = resolve(concat!(
            "<?nvs\n",
            "namespace App;\n",
            "use Throwable;\n",
            "class Boom implements Throwable {}\n",
        ));
        assert!(!diags.has_errors(), "the import reaches it: {diags:?}");

        let (_, diags) = resolve(concat!(
            "<?nvs\n",
            "namespace App;\n",
            "class Boom implements Throwable {}\n",
        ));
        assert!(
            diags.has_errors(),
            "without the import the short name is `App\\Throwable`: {diags:?}"
        );
    }

    /// `rule:programs/implementing`'s three words, one assertion each: *non-abstract*,
    /// *classes*, *implementing `T`*. The fixture declares one of everything
    /// the filter has to drop — an abstract implementor, the interface
    /// itself, an unrelated class — and the answer is asserted whole rather
    /// than by membership, so a filter that lets one through fails here.
    #[test]
    fn implementors_are_the_non_abstract_classes_that_reach_the_interface() {
        let (graph, diags) = resolve(concat!(
            "<?nvs\n",
            "interface Module {}\n",
            "class Zebra implements Module {}\n",
            "abstract class Partial implements Module {}\n",
            "class Alpha extends Partial {}\n",
            "class Loose {}\n",
        ));
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(
            implementors(&QName::parse("Module"), &graph),
            vec![QName::parse("Alpha"), QName::parse("Zebra")],
        );
    }

    /// The sort is `rule:programs/implementing`'s own requirement and not a convenience: the
    /// order must not depend on filesystem enumeration, and a `FxHashMap`
    /// iteration order is exactly the kind of thing that varies. Declaration
    /// order here is the reverse of the answer's, so an implementation that
    /// forgot to sort would have to be wrong in the same direction twice to
    /// pass.
    #[test]
    fn implementors_sort_by_fully_qualified_name_across_namespaces() {
        let (graph, diags) = resolve(concat!(
            "<?nvs\n",
            "namespace Vendor;\n",
            "interface Module {}\n",
            "class Widget implements Module {}\n",
            "namespace Acme;\n",
            "class Thing implements Vendor\\Module {}\n",
            "class Gadget implements Vendor\\Module {}\n",
        ));
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(
            implementors(&QName::parse(r"Vendor\Module"), &graph),
            vec![
                QName::parse(r"Acme\Gadget"),
                QName::parse(r"Acme\Thing"),
                QName::parse(r"Vendor\Widget"),
            ],
        );
    }

    /// The key is the name's *segments*, and that is a different order from
    /// the one its rendered text sorts under — not a spelling of the same
    /// one. `\` is byte 0x5C, above every upper-case letter, so a sort over
    /// rendered strings puts `App\SubA` ahead of `App\Sub\A` while a sort
    /// over segments compares `Sub` against `SubA` and answers the other
    /// way. The cross-namespace case above cannot pin this: every pair in it
    /// sorts identically under either key, so both implementations pass it.
    /// `rule:programs/implementing` asks only that the order not depend on filesystem
    /// enumeration; which of the two candidate orders is the answer is
    /// decided here, on the ground that a qualified name is a path and a
    /// path orders by segment.
    #[test]
    fn an_interface_enumeration_is_sorted_by_qualified_name() {
        let (graph, diags) = resolve(concat!(
            "<?nvs\n",
            "namespace App;\n",
            "interface Module {}\n",
            "class SubA implements Module {}\n",
            "class Beta implements Module {}\n",
            "namespace App\\Sub;\n",
            "class A implements App\\Module {}\n",
        ));
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(
            implementors(&QName::parse(r"App\Module"), &graph),
            vec![
                QName::parse(r"App\Beta"),
                QName::parse(r"App\Sub\A"),
                QName::parse(r"App\SubA"),
            ],
        );
    }

    /// `abstract` is asked at the candidate and nowhere else on the walk:
    /// `Leaf` reaches `Module` only through two abstract intermediates and is
    /// still the whole answer, while the intermediates themselves are
    /// dropped. The [`ClassLinks::concrete`] flags are asserted beside the
    /// enumeration because they are what the filter reads, so a graph that
    /// recorded `abstract` wrongly fails here on the field rather than on the
    /// answer three steps later. § 3 expands to `new` expressions, which is
    /// why an abstract class in the answer is a program that cannot run.
    #[test]
    fn an_abstract_class_is_not_enumerated() {
        let (graph, diags) = resolve(concat!(
            "<?nvs\n",
            "interface Module {}\n",
            "abstract class Base implements Module {}\n",
            "abstract class Middle extends Base {}\n",
            "class Leaf extends Middle {}\n",
        ));
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(
            implementors(&QName::parse("Module"), &graph),
            vec![QName::parse("Leaf")],
        );
        for (name, concrete) in [
            ("Module", false),
            ("Base", false),
            ("Middle", false),
            ("Leaf", true),
        ] {
            assert_eq!(
                graph
                    .get(&QName::parse(name))
                    .expect("every declaration has links")
                    .concrete,
                concrete,
                "{name}",
            );
        }
    }

    /// [`implements_interface`] is reflexive, so the interface would list
    /// itself if `concrete` were not asked — and a program cannot `new` an
    /// interface. Its sibling assertion is that the answer for a class
    /// target is still the classes below it, since § 3 restricts the *type
    /// parameter* to an interface rather than restricting this walk.
    #[test]
    fn an_interface_is_never_its_own_implementor() {
        let (graph, diags) = resolve(concat!(
            "<?nvs\n",
            "interface Module {}\n",
            "class Only implements Module {}\n",
        ));
        assert!(!diags.has_errors(), "{diags:?}");
        assert_eq!(
            implementors(&QName::parse("Module"), &graph),
            vec![QName::parse("Only")],
        );
        assert_eq!(
            implementors(&QName::parse("Only"), &graph),
            vec![QName::parse("Only")],
        );
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
