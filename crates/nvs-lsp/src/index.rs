//! One workspace symbol index, built in one place.
//!
//! `rule:ide/five-features-are-one-reference-index` is this module's whole
//! specification: find-references, occurrence highlight, CodeLens, type
//! hierarchy and unused-member dimming are **five queries against one index**,
//! and none of them gets a walk of its own. What is here is the index, its one
//! construction site and its invalidation; the readers are a later slice's and
//! they read it rather than building a second one.
//!
//! # Decision: a symbol is a string, and this module owns the spelling
//!
//! A reader asks about a *name*, and the three things a name can be here are a
//! type, a method and a property. They are spelled `App\User`,
//! `App\User::greet` and `App\User::$name` — the fully-qualified name the
//! checker resolved, then the member as its own source writes it, `$` sigil
//! included for a property because that is what tells it from a method of the
//! same name (`class C { public int $x; public function x(): int … }` is legal).
//!
//! A string rather than an id because both sides of the index are read *back*
//! from the checker: `crate::definition::target_of` is what turns one
//! recorded expression into the name it resolved to, and it is the same
//! function `definition` and `hover` already answer a cursor with. An id would
//! need a table mapping it to that resolution, which is the same string keyed
//! twice.
//!
//! # Decision: one entry per file, produced by whichever analysis reached it
//!
//! One analysis is one *entry point* and reads a whole `require`/`autoload`
//! graph ([`crate::analyse`]), so a file in three graphs would be indexed three
//! times over. It is indexed once: the first analysis to reach a file owns its
//! entry, and every later one skips it. The order the tree is walked in is the
//! sorted path order the tree selection returns, so which analysis that is does not
//! depend on a hash map's iteration.
//!
//! # Decision: invalidation follows the read edge, not the `require` edge
//!
//! An entry records **every file the analysis that produced it read**, which is
//! `nvs_lsp::Analysed::files` — the same set [`crate::Documents::record_graph`]
//! keeps for republishing. A `didChange` therefore drops the file that changed
//! and every file whose entry was produced by an analysis that read it, and
//! nothing else, which is what [`SymbolIndex::invalidate`] is and what
//! `tests/index.rs` pins. Following the `require` edge instead would need a
//! second graph, kept by hand, that says the same thing.
//!
//! This is also the whole of why the index is warm at all: a keystroke costs
//! one analysis of the changed file's graph rather than a walk of the
//! workspace, which is the bound `rule:ide/a-full-reanalysis-stays-under-a-bound`
//! names and `tests/latency.rs` measures with the index warm.
//!
//! # Decision: the scope selects the tree, and never the construction site
//!
//! `rule:ide/check-scope-defaults-to-open-documents` makes `nvs.check.scope` a
//! property of the *query*, and here that is literal: [`CheckScope`] is read by
//! the tree selection, which answers which files are entry points, and by
//! nothing else. The one construction site never sees it at all, so a
//! workspace pass and an open-documents pass produce byte-identical entries for
//! a file they both reach.
//!
//! # Known gaps
//!
//! An enum case occurrence is recorded against its **enum**, not against the
//! case, because that is what the checker resolved it to
//! (`nvs_types::ExprInfo::EnumCase`) and `definition` answers the same way. A
//! declaration side that records the case and an occurrence side that cannot
//! name it is the one asymmetry here, and closing it is a change in the
//! checker's table rather than in this walk.
//!
//! Nothing records *visibility*, which unused-member dimming needs to know a
//! member is private. It is a property of the declaration's own node and is
//! added where the declaration walk reads the member, not by a second walk.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{BytePos, SourceFile, Span, canonical_key};
use nvs_hir::{Loaded, SymbolKind};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, Stmt, StmtKind};
use nvs_syntax::walk;

use crate::definition::{Target, declared_type, target_of, text_of};
use crate::document::{Analysed, Documents, analyse_file};

/// Which files the index is built over — the `nvs.check.scope` setting
/// `rule:ide/check-scope-defaults-to-open-documents` freezes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckScope {
    /// The open documents and the graphs they resolve, which is what the
    /// server published diagnostics for before this setting existed.
    #[default]
    Open,
    /// Every `.nvs` file under the workspace root as well.
    ///
    /// The expensive setting, and the one whose cost is measured least, which
    /// is why it is not the default: the tree is walked once per build and a
    /// repository nobody has opened a file in is still read in full.
    Workspace,
}

/// What a declaration declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclKind {
    /// A `class`.
    Class,
    /// An `interface`.
    Interface,
    /// An `enum`.
    Enum,
    /// A `type Name = TypeExpr;` alias.
    TypeAlias,
    /// A method of one of the above.
    Method,
    /// A property of one of the above.
    Property,
    /// A class constant.
    Const,
    /// One `case` of an enum.
    EnumCase,
}

impl DeclKind {
    /// The kind for a type-level declaration `nvs_hir` collected.
    #[must_use]
    pub const fn of_symbol(kind: SymbolKind) -> Self {
        match kind {
            SymbolKind::Class => Self::Class,
            SymbolKind::Interface => Self::Interface,
            SymbolKind::Enum => Self::Enum,
            SymbolKind::TypeAlias => Self::TypeAlias,
        }
    }

    /// A lower-case noun for this kind, on `nvs_hir::SymbolKind::describe`'s
    /// terms.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Class => "a class",
            Self::Interface => "an interface",
            Self::Enum => "an enum",
            Self::TypeAlias => "a type alias",
            Self::Method => "a method",
            Self::Property => "a property",
            Self::Const => "a constant",
            Self::EnumCase => "an enum case",
        }
    }
}

/// Where something was written: a file, and the bytes of the name itself.
///
/// A [`nvs_diagnostics::Span`] cannot be kept here. Its `file` is a
/// `SourceId` into the [`nvs_diagnostics::SourceMap`] of one analysis, and
/// every analysis builds its own — so an id outlives the map it means anything
/// in. The path outlives every analysis, which is what an index is for.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Site {
    /// The file, under `nvs_diagnostics::canonical_key` — the same key
    /// [`crate::Documents`] holds a graph under, so the two agree about what
    /// "the same file" means.
    pub path: PathBuf,
    /// First byte of the name.
    pub start: BytePos,
    /// One past its last byte.
    pub end: BytePos,
}

/// One name a file declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    /// The name, spelled the way the module doc's first decision spells it.
    pub symbol: String,
    /// What it declares.
    pub kind: DeclKind,
    /// Where the declared name itself was written — never the whole
    /// declaration, which is what an editor puts a caret on.
    pub site: Site,
}

/// One place a declared name was used, with what it resolved to.
///
/// The span is the **node's**, not the name's: a member name is a property of
/// the access that writes it rather than a node of its own
/// (`nvs_syntax::walk`'s decision), so `$u->name` reports the whole access.
/// Narrowing that is a node per member name and is that module's to give.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Occurrence {
    /// The name this use resolved to.
    pub symbol: String,
    /// Where it was written.
    pub site: Site,
}

/// One file, as the analysis that reached it read it.
#[derive(Debug)]
struct Indexed {
    /// Every name declared here, in source order.
    decls: Vec<Declaration>,
    /// Every resolved use written here, in source order.
    occurrences: Vec<Occurrence>,
    /// Every file that analysis read, this one included — the edge
    /// [`SymbolIndex::invalidate`] follows.
    reads: Vec<PathBuf>,
    /// The entry point that analysis started from, which is what re-indexing
    /// this file costs.
    entry: PathBuf,
}

/// Every name the workspace declares and every place one is used.
///
/// Built once by [`SymbolIndex::build`] and kept warm by
/// [`SymbolIndex::refresh`]; the module doc is the design and
/// `rule:ide/five-features-are-one-reference-index` is the rule.
#[derive(Debug, Default)]
pub struct SymbolIndex {
    /// One entry per file, keyed by `nvs_diagnostics::canonical_key`, ordered
    /// so every answer this index gives is in a stable order.
    files: BTreeMap<PathBuf, Indexed>,
}

impl SymbolIndex {
    /// The index over the tree `scope` selects, `root` being the workspace
    /// directory a `Workspace` pass walks.
    ///
    /// A `root` of `None` is a client that named no workspace folder, and
    /// under `Workspace` that leaves the open documents — there is no
    /// directory to widen to, and guessing one from an open file's parent
    /// would index whatever happened to be beside it.
    #[must_use]
    pub fn build(documents: &Documents, scope: CheckScope, root: Option<&Path>) -> Self {
        let mut index = Self::default();
        for (path, version) in tree(documents, scope, root) {
            index.absorb(documents, &path, version);
        }
        index
    }

    /// Re-indexes `changed` and every file whose analysis read it, and nothing
    /// else, returning the files that were dropped.
    ///
    /// The dropped set is the answer the test asks for and the one a caller
    /// wants anyway: it is exactly the files whose declarations and
    /// occurrences have just been replaced, so a reader holding an answer
    /// about one of them knows it is stale.
    pub fn refresh(&mut self, documents: &Documents, changed: &Path) -> Vec<PathBuf> {
        let key = canonical_key(changed);
        let mut entries: Vec<PathBuf> = self
            .files
            .values()
            .filter(|indexed| indexed.reads.contains(&key))
            .map(|indexed| indexed.entry.clone())
            .collect();
        // A file the index has never seen — one just opened, or one created
        // under a workspace root — is an entry point in its own right, and
        // nothing above found it because nothing had read it yet.
        if !entries.contains(&key) && documents_version(documents, &key).is_some() {
            entries.push(key.clone());
        }
        entries.sort();
        entries.dedup();

        let dropped = self.invalidate(changed);
        for entry in entries {
            let version = documents_version(documents, &entry).unwrap_or(NO_BUFFER);
            self.absorb(documents, &entry, version);
        }
        dropped
    }

    /// Drops `changed` and every file whose analysis read it, returning them
    /// in path order.
    ///
    /// Separate from [`refresh`](Self::refresh) because the two halves are
    /// separately true: what goes stale is a fact about the read edge, and
    /// what is rebuilt is a fact about the tree.
    pub fn invalidate(&mut self, changed: &Path) -> Vec<PathBuf> {
        let key = canonical_key(changed);
        let stale: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|(_, indexed)| indexed.reads.contains(&key))
            .map(|(path, _)| path.clone())
            .collect();
        for path in &stale {
            self.files.remove(path);
        }
        stale
    }

    /// Where `symbol` was declared, if the index holds a declaration for it.
    ///
    /// One declaration and not a list: a name is declared once
    /// (`nvs_hir::SymbolTable` keeps the first and reports the rest as
    /// duplicates), so a second is a program with a diagnostic on it rather
    /// than an answer this index owes two halves of.
    #[must_use]
    pub fn declaration(&self, symbol: &str) -> Option<&Declaration> {
        self.files
            .values()
            .flat_map(|indexed| &indexed.decls)
            .find(|declared| declared.symbol == symbol)
    }

    /// Every use of `symbol`, in file and then source order.
    #[must_use]
    pub fn occurrences(&self, symbol: &str) -> Vec<&Occurrence> {
        self.files
            .values()
            .flat_map(|indexed| &indexed.occurrences)
            .filter(|occurrence| occurrence.symbol == symbol)
            .collect()
    }

    /// Every name `path` declares, in source order, or nothing for a file the
    /// index does not hold.
    #[must_use]
    pub fn declarations_in(&self, path: &Path) -> &[Declaration] {
        self.files
            .get(&canonical_key(path))
            .map_or(&[], |indexed| indexed.decls.as_slice())
    }

    /// Every use `path` writes, in source order, or nothing for a file the
    /// index does not hold.
    #[must_use]
    pub fn occurrences_in(&self, path: &Path) -> &[Occurrence] {
        self.files
            .get(&canonical_key(path))
            .map_or(&[], |indexed| indexed.occurrences.as_slice())
    }

    /// Every file the index holds, in path order.
    pub fn files(&self) -> impl Iterator<Item = &Path> {
        self.files.keys().map(PathBuf::as_path)
    }

    /// Whether the index holds an entry for `path`.
    #[must_use]
    pub fn holds(&self, path: &Path) -> bool {
        self.files.contains_key(&canonical_key(path))
    }

    /// How many files the index holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the index holds nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Indexes every file one analysis of `entry` reaches.
    ///
    /// **This is the crate's one symbol-index construction site**, and
    /// `tests/index.rs` is what says so: every other module here reads a
    /// `&SymbolIndex`, and a second feature walking the front end for names of
    /// its own is the thing `rule:ide/five-features-are-one-reference-index`
    /// refuses. A file another entry already indexed is skipped rather than
    /// re-indexed, per the module doc's second decision.
    fn absorb(&mut self, documents: &Documents, entry: &Path, version: i32) {
        let Some(analysed) = analyse_file(documents, entry, version) else {
            return;
        };
        let reads: Vec<PathBuf> = analysed.files().map(|file| canonical_key(&file)).collect();

        for loaded in &analysed.loaded {
            let Some(path) = analysed.map.file(loaded.id).path() else {
                continue;
            };
            let key = canonical_key(path);
            if self.files.contains_key(&key) {
                continue;
            }
            let indexed = Indexed {
                decls: declarations(&analysed, loaded, &key),
                occurrences: occurrences(&analysed, loaded, &key),
                reads: reads.clone(),
                entry: canonical_key(entry),
            };
            self.files.insert(key, indexed);
        }
    }
}

/// The version recorded for a file no client has open.
///
/// A version is the client's stamp on a buffer and a file read from disk has
/// none. Nothing compares this one: `Documents::is_current` is asked about
/// documents, and the index publishes nothing.
const NO_BUFFER: i32 = 0;

/// The entry points `scope` selects, in path order, each with the version to
/// record for it.
///
/// The open documents are always in it, `Workspace` or not: a file being edited
/// is the one whose names are most likely to have just changed, and under
/// `Open` it is the whole tree.
fn tree(documents: &Documents, scope: CheckScope, root: Option<&Path>) -> Vec<(PathBuf, i32)> {
    let mut selected: BTreeMap<PathBuf, i32> = BTreeMap::new();
    for document in documents.iter() {
        if let Some(path) = document.path() {
            selected.insert(canonical_key(path), document.version());
        }
    }
    if scope == CheckScope::Workspace
        && let Some(root) = root
    {
        let mut found = Vec::new();
        sources(root, &mut found);
        for path in found {
            selected.entry(canonical_key(&path)).or_insert(NO_BUFFER);
        }
    }
    selected.into_iter().collect()
}

/// Every `.nvs` file under `dir`, recursively.
///
/// A directory this process cannot read is skipped rather than failing the
/// pass: a workspace with one unreadable directory in it is still a workspace,
/// and an index that refused to build would take five features down with it.
fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // A build directory and a dot-directory hold no source anybody
            // wrote, and `vendor` holds source nobody here edits.
            let skip = path
                .file_name()
                .is_some_and(|name| name == "target" || name == "vendor")
                || path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('.'));
            if !skip {
                sources(&path, found);
            }
        } else if path.extension().is_some_and(|ext| ext == "nvs") {
            found.push(path);
        }
    }
}

/// The version of the open buffer for `key`, if a client has one.
fn documents_version(documents: &Documents, key: &Path) -> Option<i32> {
    documents
        .iter()
        .find(|document| document.path().map(canonical_key).as_deref() == Some(key))
        .map(crate::Document::version)
}

/// Every name one file of an analysis declares, in source order.
///
/// The type-level half is `nvs_hir::SymbolTable`'s, read back rather than
/// re-derived, so the index agrees with the checker about which declaration
/// won a duplicated name. The member half is the declaration's own node,
/// because `nvs_hir::MemberTable` records which names a class declares and
/// never where they were written — the same gap [`crate::definition`] closes
/// the same way.
fn declarations(analysed: &Analysed, loaded: &Loaded, path: &Path) -> Vec<Declaration> {
    let mut found = Vec::new();

    for symbol in analysed.module.symbols.iter() {
        if symbol.decl_span.file != loaded.id {
            continue;
        }
        let class = symbol.qname.to_string();
        found.push(Declaration {
            symbol: class.clone(),
            kind: DeclKind::of_symbol(symbol.kind),
            site: site(path, symbol.decl_span),
        });
        if let Some((stmt, file)) = declared_type(analysed, &symbol.qname) {
            members(stmt, file, &class, path, &mut found);
        }
    }

    found.sort_by_key(|declared| (declared.site.start, declared.site.end));
    found
}

/// The members `stmt` declares, appended to `found`.
///
/// A property keeps its `$`, which is how the module doc's first decision tells
/// `C::$x` from `C::x`, and a member whose name covers no bytes contributes
/// nothing — a half-typed declaration at the cursor has no name to be a
/// reference to yet.
fn members(stmt: &Stmt, file: &SourceFile, class: &str, path: &Path, found: &mut Vec<Declaration>) {
    let (declared, cases): (&[ClassMember], &[_]) = match &stmt.kind {
        StmtKind::ClassDecl(decl) => (&decl.members, &[]),
        StmtKind::InterfaceDecl(decl) => (&decl.members, &[]),
        StmtKind::EnumDecl(decl) => (&decl.members, &decl.cases),
        _ => return,
    };

    for member in declared {
        let (name, kind) = match &member.kind {
            ClassMemberKind::Property(property) => (property.name, DeclKind::Property),
            ClassMemberKind::Const(constant) => (constant.name, DeclKind::Const),
            ClassMemberKind::Method(method) => (method.name, DeclKind::Method),
            // `ClassMemberKind` is `#[non_exhaustive]` and `Error` is
            // recovery: a member shape this crate has not heard of declares no
            // name it could answer a reference about.
            _ => continue,
        };
        push_member(found, class, file, name, kind, path);
    }
    for case in cases {
        push_member(found, class, file, case.name.span, DeclKind::EnumCase, path);
    }
}

/// One member declaration, unless its name covers no source bytes.
fn push_member(
    found: &mut Vec<Declaration>,
    class: &str,
    file: &SourceFile,
    name: Span,
    kind: DeclKind,
    path: &Path,
) {
    let spelling = text_of(file, name);
    if spelling.is_empty() {
        return;
    }
    found.push(Declaration {
        symbol: format!("{class}::{spelling}"),
        kind,
        site: site(path, name),
    });
}

/// Every resolved use one file of an analysis writes, in source order.
///
/// One walk of the statements the analysis already parsed, asking the type
/// phase's own table what each node resolved to — which is
/// `rule:ide/five-features-are-one-reference-index`'s occurrence side and
/// `docs/decisions/0099.md` § 3's "resolution applied to every occurrence
/// rather than to the one under a cursor". Nothing is
/// re-resolved here: [`target_of`] is the same reading `definition` and `hover`
/// answer a cursor with, so a reference list and a jump cannot disagree.
fn occurrences(analysed: &Analysed, loaded: &Loaded, path: &Path) -> Vec<Occurrence> {
    let mut found = Vec::new();

    for root in walk::of_stmts(&loaded.stmts) {
        for node in std::iter::once(&root).chain(root.descendants()) {
            let Some(info) = analysed.exprs.lookup(node.span) else {
                continue;
            };
            let Some(target) = target_of(info) else {
                continue;
            };
            found.push(Occurrence {
                symbol: symbol_of(&target),
                site: site(path, node.span),
            });
        }
    }

    // Two nodes can cover the same bytes — a statement that is one expression —
    // and both read the same entry out of the table.
    found.sort();
    found.dedup();
    found
}

/// What one resolved use names, spelled the module doc's way.
fn symbol_of(target: &Target<'_>) -> String {
    match target {
        Target::Type(qname) => qname.to_string(),
        Target::Method(call) => format!("{}::{}", call.class, call.method),
        Target::Property { class, name } => format!("{class}::${name}"),
    }
}

/// A span of one file, as a site the index can keep.
fn site(path: &Path, span: Span) -> Site {
    Site {
        path: path.to_path_buf(),
        start: span.start,
        end: span.end,
    }
}
