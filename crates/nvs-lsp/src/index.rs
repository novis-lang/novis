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
//! An entry point records **every file its analysis read**, which is
//! `nvs_lsp::Analysed::files` — the same set [`crate::Documents::record_graph`]
//! keeps for republishing — once, and each file it indexed names that entry
//! point. A `didChange` therefore drops the file that changed
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
//! `rule:ide/check-scope-defaults-to-the-workspace` makes `nvs.check.scope` a
//! property of the *query*, and here that is literal: [`CheckScope`] is read by
//! the tree selection, which answers which files are entry points, and by
//! nothing else. The one construction site never sees it at all, so a
//! workspace pass and an open-documents pass produce byte-identical entries for
//! a file they both reach.
//!
//! # Decision: an occurrence is keyed on a name the entry proves, not on the
//! text in front of it
//!
//! `Status::Draft` and `Cart::LIMIT` each write two names against one recorded
//! resolution. The occurrence is the case or the constant, at its own name,
//! because that is what the checker resolved. The qualifier is recorded as a
//! second occurrence only where the entry *proves* what it says — an enum case,
//! since no enum extends another ([`case_qualifier`]) — and never for a class
//! constant, whose entry carries the class that declares it and so cannot say
//! whether `Cart::LIMIT` wrote `Cart` or the `Limits` it inherits from. That
//! keeps a reference list a list of sites that resolved to the symbol asked
//! about rather than a text search for its name.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lsp_types::Uri;
use nvs_diagnostics::{BytePos, Diagnostics, SourceFile, Span, canonical_key};
use nvs_hir::autoload::sole_declaration;
use nvs_hir::{Loaded, QName, SymbolKind};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, Modifier, NamespaceDecl, Stmt, StmtKind, Type, TypeAtom, TypeKind,
};
use nvs_syntax::{Token, TokenKind, tokenize, walk};
use nvs_types::ExprInfo;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::definition::{
    Target, covers, declared_type, named_at, paired, supertype_names, target_of, text_of,
};
use crate::document::{Analysed, Documents, analyse_file};

/// The three visibility levels a [`Declaration`] carries.
///
/// Re-exported rather than restated: `nvs_syntax` is where `private` is spelled
/// once, and a second enum here would be a second answer to what a modifier
/// means.
pub use nvs_syntax::ast::Visibility;

/// Which files the index is built over — the `nvs.check.scope` setting
/// `rule:ide/check-scope-defaults-to-the-workspace` freezes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckScope {
    /// The open documents and the graphs they resolve, and nothing else under
    /// the workspace root. The setting for a tree too large to read in full.
    Open,
    /// Every `.nvs` file under the workspace root as well.
    ///
    /// The default, because a name is completed, found and counted out of what
    /// the index holds, and an index of the open documents alone offers a
    /// developer only the classes they already have in front of them. It is
    /// the expensive setting: the tree is walked once per build and a
    /// repository nobody has opened a file in is still read in full. What that
    /// spends is one declaration and occurrence list per file under the root,
    /// held for the life of the session.
    #[default]
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
    /// The visibility it was written with — `Public` where none was written,
    /// and for everything that is not a class member.
    ///
    /// Unused-member dimming is the only reader
    /// (`rule:ide/five-features-are-one-reference-index`). A public member with
    /// no occurrence in the index is a member *this workspace* does not use,
    /// which is not the same claim and is not one an index of Novis files can
    /// make; a private one with none is unreachable from anywhere by
    /// construction, which is why the tag is only correct there.
    pub visibility: Visibility,
    /// What it directly `extends` or `implements`, spelled as
    /// [`Self::symbol`] is, and empty for everything that is not a class or an
    /// interface.
    ///
    /// The direct edge and never the ancestors: a type hierarchy is expanded a
    /// level at a time by the client asking again
    /// (`rule:ide/five-features-are-one-reference-index`), so an index that
    /// flattened the chain would answer the question nobody asked and lose the
    /// one that was.
    ///
    /// `extends` and `implements` share the field because every reader of it
    /// wants the same "what is above this" step, which is the reason
    /// [`nvs_hir::ClassLinks`] keeps a class's superclass and an interface's
    /// extended interfaces in one field too.
    pub supertypes: Vec<String>,
    /// What a `class` says about `new`, and `None` for everything that is not
    /// a class.
    ///
    /// Completion after `new` is the only reader: it offers the names a
    /// program may construct, and puts the cursor between the parentheses
    /// only where there is an argument to write.
    pub construction: Option<Construction>,
    /// The parameters a method declares, in order, and empty for everything
    /// that is not a method.
    ///
    /// The completion files are the only reader
    /// (`rule:ide/completion-files-offer-values-at-named-parameters`): an
    /// attachment to a parameter the method does not have, or to one that
    /// takes no string, is reported on the file that wrote it.
    pub parameters: Vec<Parameter>,
}

/// One parameter of a method, as a completion file names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    /// Its name, without the `$`.
    pub name: String,
    /// Whether a string argument can reach it. False only where the declared
    /// type says so from its own text: a named type may be an alias of a
    /// string, and the type is not resolved here, so it counts as one.
    pub takes_a_string: bool,
}

/// The two things a class declaration says about `new` on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Construction {
    /// Whether the class is `abstract`, which `new` does not compile on. Its
    /// constructor is still the one a subclass without its own inherits.
    pub is_abstract: bool,
    /// How many parameters its own `constructor` declares, and `None` for a
    /// class that declares none and takes whatever its superclass takes.
    pub parameters: Option<usize>,
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

/// One `use` import, and whether any name in its file reads it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import {
    /// The name it imports, fully qualified.
    pub symbol: String,
    /// Where its path was written.
    pub site: Site,
    /// Whether a short name written in the same namespace scope spells the
    /// name this import binds.
    ///
    /// Read off the file's tokens rather than off [`Occurrence`]s, because an
    /// occurrence is recorded for an expression or a clause and a type
    /// annotation is neither: an import only a parameter type names has no
    /// occurrence and is still read. The token reading errs one way only. A
    /// method or a property spelled like the import counts as a read, so an
    /// import can stay undimmed while unused, and an import a name needs is
    /// never dimmed.
    pub read: bool,
}

/// One file, as the analysis that reached it read it.
#[derive(Debug)]
struct Indexed {
    /// Every name declared here, in source order.
    decls: Vec<Declaration>,
    /// Every resolved use written here, in source order.
    occurrences: Vec<Occurrence>,
    /// Every `use` import written here, in source order.
    imports: Vec<Import>,
    /// The kind of the one type this file declares, when the file has the
    /// shape an autoloaded file must have (`nvs_hir::autoload::sole_declaration`).
    sole: Option<DeclKind>,
    /// The entry point that analysis started from, which is what re-indexing
    /// this file costs, and whose reads in [`SymbolIndex::reads`] are the edge
    /// [`SymbolIndex::invalidate`] follows.
    entry: PathBuf,
}

/// What one [`SymbolIndex::refresh`] did.
#[derive(Debug)]
pub struct Refreshed {
    /// The files whose entries were dropped, in path order.
    pub dropped: Vec<PathBuf>,
    /// The analysis of each open document the refresh re-analysed as an entry
    /// point, with the document's URI.
    pub analysed: Vec<(Uri, Analysed)>,
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
    /// Every file each entry point's analysis read, that entry included, kept
    /// once per entry point that owns at least one file in `files`. Once per
    /// entry and not once per file, because every file one analysis loads
    /// shares the same list, and a copy in each of them grows with the square
    /// of the graph.
    reads: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// Every declaration in `files`, by the name it declares.
    ///
    /// This map and the two below make each lookup by name cost what it
    /// returns, so a lens request over a file of many declarations stays
    /// linear. They spend one path handle and one copy of the name per
    /// declaration, per supertype edge and per occurrence, for as long as the
    /// file is indexed. [`SymbolIndex::post`] and [`SymbolIndex::unpost`] keep
    /// them, and are called where `files` gains and loses an entry.
    by_symbol: Postings,
    /// Every declaration in `files`, by each name it directly extends or
    /// implements.
    by_supertype: Postings,
    /// Every occurrence in `files`, by the name it resolved to.
    by_use: Postings,
}

/// Where one entry of a file's list is: the file's key and the entry's position
/// in that list. A set of them iterates by path and then position, which is the
/// file and then source order every answer here is given in.
type At = (Arc<Path>, usize);

/// The entries of one kind, by name.
type Postings = FxHashMap<String, BTreeSet<At>>;

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
    /// else, returning the files that were dropped and the analyses of the
    /// open documents among the entries it re-analysed.
    ///
    /// The dropped set is the answer the test asks for and the one a caller
    /// wants anyway: it is exactly the files whose declarations and
    /// occurrences have just been replaced, so a reader holding an answer
    /// about one of them knows it is stale.
    ///
    /// An open entry is analysed under its document's own path and version,
    /// which is exactly the analysis [`crate::analyse`] makes of that document,
    /// so the server publishes from it instead of running the front end over
    /// the same version a second time. Every other entry's analysis is dropped
    /// as soon as it is indexed: only an open document is published for, and
    /// keeping one analysis per open entry for the length of one edit is the
    /// memory this spends.
    pub fn refresh(&mut self, documents: &Documents, changed: &Path) -> Refreshed {
        let key = canonical_key(changed);
        let mut entries = self.readers_of(&key);
        // A file the index has never seen — one just opened, or one created
        // under a workspace root — is an entry point in its own right, and
        // nothing above found it because nothing had read it yet.
        if !entries.contains(&key) && open_document(documents, &key).is_some() {
            entries.push(key.clone());
        }
        entries.sort();
        entries.dedup();

        let dropped = self.invalidate(changed);
        let mut analysed = Vec::new();
        for entry in entries {
            let open = open_document(documents, &entry);
            match open.and_then(|document| Some((document, document.path()?))) {
                Some((document, path)) => {
                    if let Some(analysis) = self.absorb(documents, path, document.version()) {
                        analysed.push((document.uri().clone(), analysis));
                    }
                }
                None => {
                    self.absorb(documents, &entry, NO_BUFFER);
                }
            }
        }
        Refreshed { dropped, analysed }
    }

    /// Drops `changed` and every file whose analysis read it, returning them
    /// in path order.
    ///
    /// Separate from [`refresh`](Self::refresh) because the two halves are
    /// separately true: what goes stale is a fact about the read edge, and
    /// what is rebuilt is a fact about the tree.
    pub fn invalidate(&mut self, changed: &Path) -> Vec<PathBuf> {
        let readers = self.readers_of(&canonical_key(changed));
        let stale: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|(_, indexed)| readers.contains(&indexed.entry))
            .map(|(path, _)| path.clone())
            .collect();
        for path in &stale {
            if let Some(indexed) = self.files.remove(path) {
                self.unpost(path, &indexed);
            }
        }
        for entry in &readers {
            self.reads.remove(entry);
        }
        stale
    }

    /// The entry points whose analysis read `key`, in path order.
    fn readers_of(&self, key: &Path) -> Vec<PathBuf> {
        self.reads
            .iter()
            .filter(|(_, reads)| reads.iter().any(|read| read == key))
            .map(|(entry, _)| entry.clone())
            .collect()
    }

    /// The private declarations in `path` that nothing anywhere in the index
    /// refers to.
    ///
    /// Private is what makes the question answerable at all. "No occurrence in
    /// the index" is unreachability for a member only its own file can name,
    /// and says nothing whatever about a public one, which anything outside the
    /// indexed tree may still be using. How much of the workspace *is* indexed
    /// when this is asked is `rule:ide/check-scope-defaults-to-the-workspace`'s
    /// answer, and the caller is what holds the setting.
    #[must_use]
    pub fn unused_private(&self, path: &Path) -> Vec<&Declaration> {
        self.declarations_in(path)
            .iter()
            .filter(|declared| {
                declared.visibility == Visibility::Private
                    && !self.by_use.contains_key(&declared.symbol)
            })
            .collect()
    }

    /// The `use` imports in `path` that no name in their scope reads.
    ///
    /// A file's own question, unlike [`Self::unused_private`]'s: an import
    /// binds a short name for the namespace scope it is written in, and
    /// nothing outside that file can read it.
    #[must_use]
    pub fn unused_imports(&self, path: &Path) -> Vec<&Import> {
        self.files
            .get(&canonical_key(path))
            .map_or(&[][..], |indexed| indexed.imports.as_slice())
            .iter()
            .filter(|import| !import.read)
            .collect()
    }

    /// What `symbol` directly extends or implements, in the order it was
    /// written, and nothing for a name the index does not hold a declaration
    /// for.
    ///
    /// A supertype the index has never seen drops out rather than appearing as
    /// a name with no site: an editor cannot navigate to one, and a `Core`
    /// class or a file outside the scope
    /// `rule:ide/check-scope-defaults-to-the-workspace` selects is exactly
    /// that case.
    #[must_use]
    pub fn supertypes(&self, symbol: &str) -> Vec<&Declaration> {
        self.declaration(symbol).map_or_else(Vec::new, |declared| {
            declared
                .supertypes
                .iter()
                .filter_map(|above| self.declaration(above))
                .collect()
        })
    }

    /// Every declaration that directly extends or implements `symbol`, in file
    /// and then source order.
    #[must_use]
    pub fn subtypes(&self, symbol: &str) -> Vec<&Declaration> {
        self.by_supertype
            .get(symbol)
            .into_iter()
            .flatten()
            .filter_map(|(path, at)| self.files.get(&**path)?.decls.get(*at))
            .collect()
    }

    /// The methods `method` overrides: up each edge its class was written
    /// with, the nearest type that declares a method of the same name, in the
    /// order the edges were written.
    ///
    /// Nearest and not every one: what a method replaces is the one it would
    /// otherwise have inherited, and a declaration further up that branch is
    /// what *that* one overrides. A type reached twice, through a diamond or a
    /// cycle the checker has already reported, is walked once.
    #[must_use]
    pub fn overridden(&self, method: &str) -> Vec<&Declaration> {
        let Some((owner, name)) = method.rsplit_once("::") else {
            return Vec::new();
        };
        let mut seen = FxHashSet::from_iter([owner.to_owned()]);
        let mut found = Vec::new();
        for above in self.supertypes(owner) {
            self.nearest_above(above, name, &mut seen, &mut found);
        }
        found
    }

    /// [`SymbolIndex::overridden`] for one branch, from `ty` upwards.
    fn nearest_above<'a>(
        &'a self,
        ty: &'a Declaration,
        name: &str,
        seen: &mut FxHashSet<String>,
        found: &mut Vec<&'a Declaration>,
    ) {
        if !seen.insert(ty.symbol.clone()) {
            return;
        }
        if let Some(declared) = self.method_of(&ty.symbol, name) {
            found.push(declared);
            return;
        }
        for above in self.supertypes(&ty.symbol) {
            self.nearest_above(above, name, seen, found);
        }
    }

    /// Every method that overrides `method`: each type below its class, at any
    /// depth, that declares a method of the same name, nearer levels first and
    /// each level in file and then source order.
    ///
    /// Every depth and not only the nearest, because each of them replaces
    /// this method for the objects of its own class, which is what a reader
    /// above the declaration wants counted.
    #[must_use]
    pub fn overriders(&self, method: &str) -> Vec<&Declaration> {
        let Some((owner, name)) = method.rsplit_once("::") else {
            return Vec::new();
        };
        let mut seen = FxHashSet::from_iter([owner.to_owned()]);
        let mut pending = self.subtypes(owner);
        let mut found = Vec::new();
        let mut next = 0;
        while let Some(&ty) = pending.get(next) {
            next += 1;
            if !seen.insert(ty.symbol.clone()) {
                continue;
            }
            if let Some(declared) = self.method_of(&ty.symbol, name) {
                found.push(declared);
            }
            pending.extend(self.subtypes(&ty.symbol));
        }
        found
    }

    /// The method `ty` itself declares under `name`, if it declares one.
    fn method_of(&self, ty: &str, name: &str) -> Option<&Declaration> {
        self.declaration(&format!("{ty}::{name}"))
            .filter(|declared| declared.kind == DeclKind::Method)
    }

    /// Where `symbol` was declared, if the index holds a declaration for it.
    ///
    /// One declaration and not a list: a name is declared once
    /// (`nvs_hir::SymbolTable` keeps the first and reports the rest as
    /// duplicates), so a second is a program with a diagnostic on it rather
    /// than an answer this index owes two halves of.
    #[must_use]
    pub fn declaration(&self, symbol: &str) -> Option<&Declaration> {
        let (path, at) = self.by_symbol.get(symbol)?.first()?;
        self.files.get(&**path)?.decls.get(*at)
    }

    /// Every use of `symbol`, in file and then source order.
    #[must_use]
    pub fn occurrences(&self, symbol: &str) -> Vec<&Occurrence> {
        self.by_use
            .get(symbol)
            .into_iter()
            .flatten()
            .filter_map(|(path, at)| self.files.get(&**path)?.occurrences.get(*at))
            .collect()
    }

    /// Every `use` import of `symbol`, in file and then source order.
    ///
    /// Apart from [`Self::occurrences`] because an import brings a name into
    /// scope and uses nothing: *Find References* lists it, and the CodeLens
    /// count of uses does not.
    #[must_use]
    pub fn imports_of(&self, symbol: &str) -> Vec<&Import> {
        self.files
            .values()
            .flat_map(|indexed| &indexed.imports)
            .filter(|import| import.symbol == symbol)
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

    /// Every file that declares exactly one type and nothing else, with that
    /// type's kind, in path order. `nvs/fileKinds` is this list.
    pub fn sole_kinds(&self) -> impl Iterator<Item = (&Path, DeclKind)> {
        self.files
            .iter()
            .filter_map(|(path, indexed)| Some((path.as_path(), indexed.sole?)))
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
    ///
    /// Returns the analysis it indexed, for [`refresh`](Self::refresh) to hand
    /// on.
    fn absorb(&mut self, documents: &Documents, entry: &Path, version: i32) -> Option<Analysed> {
        let analysed = analyse_file(documents, entry, version)?;
        let entry = canonical_key(entry);
        let mut owns_a_file = false;

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
                imports: imports(&analysed, loaded, &key),
                sole: sole_declaration(&loaded.stmts, analysed.map.file(loaded.id))
                    .map(|(_, kind)| DeclKind::of_symbol(kind)),
                entry: entry.clone(),
            };
            self.post(&key, &indexed);
            self.files.insert(key, indexed);
            owns_a_file = true;
        }
        // An entry that indexed no file has nothing an edit could make stale,
        // so its reads are not kept.
        if owns_a_file {
            let reads = analysed.files().map(|file| canonical_key(&file)).collect();
            self.reads.insert(entry, reads);
        }
        Some(analysed)
    }

    /// Adds what `indexed` declares and uses to the three maps, under `key`.
    fn post(&mut self, key: &Path, indexed: &Indexed) {
        let key: Arc<Path> = Arc::from(key);
        for (at, declared) in indexed.decls.iter().enumerate() {
            let posting = (Arc::clone(&key), at);
            for above in &declared.supertypes {
                add(&mut self.by_supertype, above, posting.clone());
            }
            add(&mut self.by_symbol, &declared.symbol, posting);
        }
        for (at, used) in indexed.occurrences.iter().enumerate() {
            add(&mut self.by_use, &used.symbol, (Arc::clone(&key), at));
        }
    }

    /// Removes from the three maps what [`SymbolIndex::post`] added for
    /// `indexed` under `key`.
    fn unpost(&mut self, key: &Path, indexed: &Indexed) {
        let key: Arc<Path> = Arc::from(key);
        for (at, declared) in indexed.decls.iter().enumerate() {
            let posting = (Arc::clone(&key), at);
            for above in &declared.supertypes {
                remove(&mut self.by_supertype, above, &posting);
            }
            remove(&mut self.by_symbol, &declared.symbol, &posting);
        }
        for (at, used) in indexed.occurrences.iter().enumerate() {
            remove(&mut self.by_use, &used.symbol, &(Arc::clone(&key), at));
        }
    }
}

/// Files `posting` under `symbol`.
fn add(postings: &mut Postings, symbol: &str, posting: At) {
    postings
        .entry(symbol.to_owned())
        .or_default()
        .insert(posting);
}

/// Removes `posting` from under `symbol`, and the name with it once nothing is
/// left under it, so the map's size follows what is indexed.
fn remove(postings: &mut Postings, symbol: &str, posting: &At) {
    if let Some(entries) = postings.get_mut(symbol) {
        entries.remove(posting);
        if entries.is_empty() {
            postings.remove(symbol);
        }
    }
}

/// The symbol the cursor at `offset` names, or `None` for a cursor on nothing
/// this index could hold one for.
///
/// The read side's entry point: a reader has a position and the index is keyed
/// by name, so this is the one step between them, and every feature that turns
/// a cursor into a query comes through here rather than reading a name for
/// itself.
///
/// **A name is written two ways and both are answered here.** A use carries
/// what the checker resolved it to, which is [`crate::definition::named_at`] —
/// the same reading `definition` and `hover` answer a cursor with — spelled by
/// `symbol_of` below. A declaration carries the name it *declares*, which no
/// resolution walk sees because a declaration is not an expression, and that is
/// [`declared_at`]. Spelling both in one place is what stops a cursor that
/// jumps to a declaration and a cursor that lists references disagreeing about
/// which name they are about.
///
/// The declaration is asked first. Both readings answering one offset would
/// need a use written inside the bytes of a declared name, which no grammar
/// here has, so the order is which question is cheaper rather than which
/// answer wins.
#[must_use]
pub fn symbol_at(analysed: &Analysed, offset: BytePos) -> Option<String> {
    if let Some(declared) = declared_at(analysed, offset) {
        return Some(declared);
    }
    let (target, _) = named_at(analysed, offset)?;
    Some(symbol_of(&target))
}

/// The name a declaration of the entry document writes at `offset`, spelled the
/// module doc's first decision's way.
///
/// The entry document alone, because that is the file a cursor is ever in
/// (`crate::document::Analysed::index`). The two halves are read where
/// [`declarations`] reads them and in the same order — the type from
/// `nvs_hir::SymbolTable`, the member from the declaration's own node — so a
/// name answered here is a name the index is keyed on rather than a second
/// spelling of one.
fn declared_at(analysed: &Analysed, offset: BytePos) -> Option<String> {
    analysed.module.symbols.iter().find_map(|symbol| {
        if symbol.decl_span.file != analysed.entry {
            return None;
        }
        if covers(symbol.decl_span, offset) {
            return Some(symbol.qname.to_string());
        }
        let (stmt, file) = declared_type(analysed, &symbol.qname)?;
        let class = symbol.qname.to_string();
        member_names(stmt)
            .into_iter()
            .find(|(name, ..)| covers(*name, offset))
            .and_then(|(name, ..)| member_symbol(&class, file, name))
    })
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
pub(crate) fn tree(
    documents: &Documents,
    scope: CheckScope,
    root: Option<&Path>,
) -> Vec<(PathBuf, i32)> {
    let mut selected: BTreeMap<PathBuf, i32> = BTreeMap::new();
    for document in documents.iter() {
        if let Some(path) = document.path() {
            selected.insert(canonical_key(path), document.version());
        }
    }
    if scope == CheckScope::Workspace
        && let Some(root) = root
    {
        for path in nvs_hir::lenders::sources(root) {
            selected.entry(canonical_key(&path)).or_insert(NO_BUFFER);
        }
    }
    selected.into_iter().collect()
}

/// The open buffer for `key`, if a client has one.
fn open_document<'a>(documents: &'a Documents, key: &Path) -> Option<&'a crate::Document> {
    documents
        .iter()
        .find(|document| document.path().map(canonical_key).as_deref() == Some(key))
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
        let declared = declared_type(analysed, &symbol.qname);
        found.push(Declaration {
            symbol: class.clone(),
            kind: DeclKind::of_symbol(symbol.kind),
            site: site(path, symbol.decl_span),
            // A type declaration carries no visibility modifier: it is reachable
            // from every file that resolves its name.
            visibility: Visibility::Public,
            supertypes: supertypes_of(analysed, &symbol.qname),
            construction: declared.and_then(|(stmt, file)| construction_of(stmt, file)),
            parameters: Vec::new(),
        });
        if let Some((stmt, file)) = declared {
            members(stmt, file, &class, path, &mut found);
        }
    }

    found.sort_by_key(|declared| (declared.site.start, declared.site.end));
    found
}

/// The members `stmt` declares, appended to `found`.
fn members(stmt: &Stmt, file: &SourceFile, class: &str, path: &Path, found: &mut Vec<Declaration>) {
    for (name, kind, visibility) in member_names(stmt) {
        let Some(symbol) = member_symbol(class, file, name) else {
            continue;
        };
        found.push(Declaration {
            symbol,
            kind,
            site: site(path, name),
            visibility,
            // A member inherits nothing of its own: what a class extends is a
            // fact about the class, and the override edge a lens shows is read
            // off the two ends of that.
            supertypes: Vec::new(),
            construction: None,
            parameters: if kind == DeclKind::Method {
                parameters_of(stmt, file, name)
            } else {
                Vec::new()
            },
        });
    }
}

/// The parameters of the method `stmt` declares at `name`.
fn parameters_of(stmt: &Stmt, file: &SourceFile, name: Span) -> Vec<Parameter> {
    let declared: &[ClassMember] = match &stmt.kind {
        StmtKind::ClassDecl(decl) => &decl.members,
        StmtKind::InterfaceDecl(decl) => &decl.members,
        StmtKind::EnumDecl(decl) => &decl.members,
        _ => return Vec::new(),
    };
    declared
        .iter()
        .find_map(|member| match &member.kind {
            ClassMemberKind::Method(method) if method.name == name => Some(&method.params),
            _ => None,
        })
        .map_or_else(Vec::new, |params| {
            params
                .iter()
                .map(|param| Parameter {
                    name: text_of(file, param.name).trim_start_matches('$').to_owned(),
                    takes_a_string: param.ty.as_ref().is_none_or(takes_a_string),
                })
                .collect()
        })
}

/// Whether a value of type `ty` may be a string, read from the type's own
/// text. A union may be one when any member may, and a named type always may,
/// because it can be an alias of a string type.
fn takes_a_string(ty: &Type) -> bool {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => takes_a_string(inner),
        TypeKind::Union(members) | TypeKind::Intersection(members) => {
            members.iter().any(takes_a_string)
        }
        TypeKind::Atom(atom) => !matches!(
            atom,
            TypeAtom::Null
                | TypeAtom::Bool
                | TypeAtom::Int
                | TypeAtom::Uint
                | TypeAtom::Float
                | TypeAtom::Decimal
                | TypeAtom::Bytes
                | TypeAtom::TaintedBytes
                | TypeAtom::SecretBytes
                | TypeAtom::SecretTaintedBytes
                | TypeAtom::Array(_)
                | TypeAtom::ClassRef(_)
                | TypeAtom::Object
                | TypeAtom::Shape(_)
                | TypeAtom::Void
                | TypeAtom::Never
                | TypeAtom::True
                | TypeAtom::False
                | TypeAtom::SingleValueInt(_)
        ),
        _ => true,
    }
}

/// The name a class spells its constructor with.
const CONSTRUCTOR: &str = "constructor";

/// What the type `stmt` declares says about `new`, if it is a class.
fn construction_of(stmt: &Stmt, file: &SourceFile) -> Option<Construction> {
    let StmtKind::ClassDecl(decl) = &stmt.kind else {
        return None;
    };
    Some(Construction {
        is_abstract: decl.modifiers.contains(&Modifier::Abstract),
        parameters: decl.members.iter().find_map(|member| match &member.kind {
            ClassMemberKind::Method(method) if text_of(file, method.name) == CONSTRUCTOR => {
                Some(method.params.len())
            }
            _ => None,
        }),
    })
}

/// Every member name `stmt` declares, with what it declares and where it is
/// readable from, in source order.
///
/// One list for the two questions asked about a member name: [`members`] turns
/// each into an index entry, and [`declared_at`] asks which of them the cursor
/// is on. `ClassMemberKind` is `#[non_exhaustive]` and `Error` is recovery, so
/// a member shape this crate has not heard of declares no name either of them
/// could answer about and contributes none.
fn member_names(stmt: &Stmt) -> Vec<(Span, DeclKind, Visibility)> {
    let (declared, cases): (&[ClassMember], &[_]) = match &stmt.kind {
        StmtKind::ClassDecl(decl) => (&decl.members, &[]),
        StmtKind::InterfaceDecl(decl) => (&decl.members, &[]),
        StmtKind::EnumDecl(decl) => (&decl.members, &decl.cases),
        _ => return Vec::new(),
    };

    let mut names: Vec<(Span, DeclKind, Visibility)> = declared
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Property(property) => Some((
                property.name,
                DeclKind::Property,
                visibility_of(&property.modifiers),
            )),
            ClassMemberKind::Const(constant) => Some((
                constant.name,
                DeclKind::Const,
                visibility_of(&constant.modifiers),
            )),
            ClassMemberKind::Method(method) => Some((
                method.name,
                DeclKind::Method,
                visibility_of(&method.modifiers),
            )),
            _ => None,
        })
        .collect();
    // An enum case takes no modifier list and is reachable wherever the enum's
    // own name is.
    names.extend(
        cases
            .iter()
            .map(|case| (case.name.span, DeclKind::EnumCase, Visibility::Public)),
    );
    names
}

/// The visibility `modifiers` declares, which is `Public` when they declare
/// none.
///
/// The plain modifiers only: `private(set)` restricts *writes* and leaves the
/// member readable wherever its own visibility says, so a reference to it is
/// still a reference and dimming it would be wrong.
fn visibility_of(modifiers: &[Modifier]) -> Visibility {
    modifiers
        .iter()
        .find_map(|modifier| match modifier {
            Modifier::Public => Some(Visibility::Public),
            Modifier::Protected => Some(Visibility::Protected),
            Modifier::Private => Some(Visibility::Private),
            _ => None,
        })
        .unwrap_or(Visibility::Public)
}

/// The symbol a member of `class` written at `name` spells, or nothing when the
/// name covers no source bytes — a half-typed declaration at the cursor has no
/// name to be a reference to yet.
///
/// A property keeps its `$`, which is how the module doc's first decision tells
/// `C::$x` from `C::x`.
fn member_symbol(class: &str, file: &SourceFile, name: Span) -> Option<String> {
    let spelling = text_of(file, name);
    (!spelling.is_empty()).then(|| format!("{class}::{spelling}"))
}

/// The names `qname` directly extends or implements, spelled the way the
/// module doc's first decision spells a type.
///
/// Read off `nvs_hir`'s resolved graph and never off the `extends` clause's own
/// text: what is written there is a relative or imported spelling, and the
/// index keys on the fully-qualified name the checker resolved it to.
fn supertypes_of(analysed: &Analysed, qname: &QName) -> Vec<String> {
    analysed
        .module
        .graph
        .get(qname)
        .map_or_else(Vec::new, |links| {
            links
                .extends
                .iter()
                .chain(links.implements.iter())
                .map(QName::to_string)
                .collect()
        })
}

/// Every resolved use one file of an analysis writes, in source order.
///
/// Two walks, because a name is written in two kinds of place. The expressions
/// are one walk of the statements the analysis already parsed, asking the type
/// phase's own table what each node resolved to — which is
/// `rule:ide/five-features-are-one-reference-index`'s occurrence side and
/// `docs/decisions/0099.md` § 3's "resolution applied to every occurrence
/// rather than to the one under a cursor". Nothing is
/// re-resolved here: [`target_of`] is the same reading `definition` and `hover`
/// answer a cursor with, so a reference list and a jump cannot disagree.
///
/// The `extends` and `implements` clauses are the second walk, because a clause
/// is not an expression and no entry of [`Analysed::exprs`](crate::Analysed)
/// covers one. What such a name resolved to is `nvs_hir::ClassLinks`, off the
/// same graph [`supertypes_of`] reads the declaration side from, so an
/// interface counts the classes that implement it and the two sides of one
/// inheritance edge come out of one resolution.
///
/// A use is recorded at the **name**, which [`named`] reads off the node rather
/// than off the expression around it: a highlight box is drawn on exactly the
/// span answered here, and `$u->greet()` highlighted from `$u` is a box around
/// the wrong word.
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
                site: site(path, named(node)),
            });
            found.extend(case_qualifier(info, node, path));
        }
    }

    for symbol in analysed.module.symbols.iter() {
        if symbol.decl_span.file != loaded.id {
            continue;
        }
        let Some(links) = analysed.module.graph.get(&symbol.qname) else {
            continue;
        };
        let Some((stmt, _)) = declared_type(analysed, &symbol.qname) else {
            continue;
        };
        let (extends, implements) = supertype_names(stmt);
        clause_uses(&extends, &links.extends, path, &mut found);
        clause_uses(&implements, &links.implements, path, &mut found);
    }

    // Two nodes can cover the same bytes — a statement that is one expression —
    // and both read the same entry out of the table. The sort is also what puts
    // the two walks into the one order the index answers in.
    found.sort();
    found.dedup();
    found
}

/// One occurrence per clause entry, at the name written and against the name it
/// resolved to.
///
/// Paired off by position, which is [`crate::definition::paired`] and is read
/// by [`crate::definition::clause_at`] as well, so the name a clause entry is
/// recorded against is the name a cursor standing on it asks about.
fn clause_uses(written: &[Span], resolved: &[QName], path: &Path, found: &mut Vec<Occurrence>) {
    let (names, qnames) = paired(written, resolved);
    for (name, qname) in names.iter().zip(qnames) {
        found.push(Occurrence {
            symbol: qname.to_string(),
            site: site(path, *name),
        });
    }
}

/// The enum an `Enum::Case` read writes in front of the case, as a use of the
/// enum itself — and `None` for every other recorded expression.
///
/// The one production that leaves two occurrences, because it is the one that
/// writes two names this walk can key both of: the case is what the read
/// resolved to, and no enum extends another, so the name in front of it is
/// necessarily the enum the entry already carries. `Cart::LIMIT` writes two
/// names as well and only the constant's is proven — the class written may be a
/// subclass of the one that declares it — so the qualifier is left alone there
/// rather than keyed under a name the source did not write.
///
/// The cursor side answers the same two names, and at the same two spans:
/// [`crate::definition::named_at`] reads the qualifier off the node inside the
/// production, so a reader who clicks `Status` is asked about the enum and one
/// who clicks `Draft` about the case.
fn case_qualifier(info: &ExprInfo, node: &walk::Node, path: &Path) -> Option<Occurrence> {
    let ExprInfo::EnumCase { enum_, .. } = info else {
        return None;
    };
    let written = node.children.first()?;
    Some(Occurrence {
        symbol: enum_.to_string(),
        site: site(path, written.name.unwrap_or(written.span)),
    })
}

/// What one resolved use names, spelled the module doc's way.
fn symbol_of(target: &Target<'_>) -> String {
    match target {
        Target::Type(qname) => qname.to_string(),
        Target::Method(call) => format!("{}::{}", call.class, call.method),
        Target::Property { class, name } => format!("{class}::${name}"),
        Target::Constant { class, name } | Target::TypeAlias { class, name } => {
            format!("{class}::{name}")
        }
    }
}

/// Where in `node` the name this use resolved to was written.
///
/// [`nvs_syntax::walk::Node::name`] is the production's own name, and that is
/// the answer wherever the name written and the symbol resolved are the same
/// one: a call resolves to its method, an access to its property, a
/// `Class::CONST` and a `Enum::Case` to the constant each names, a `new` to its
/// class. A call through a callable resolves to a name written on its
/// **callee** side, so it answers the child node that holds it.
///
/// A production that wrote no name at all — `new $class()`, `$u->{$name}` —
/// answers the whole expression, which is the widest true thing there is to
/// say about where it was written. A type test is that shape too: the type on
/// the right of `is` is no child of its own, so the test answers the name it
/// carries over the span it covers.
fn named(node: &walk::Node) -> Span {
    let written = match node.kind {
        "Call" => node.children.first(),
        _ => None,
    }
    .unwrap_or(node);
    written.name.unwrap_or(written.span)
}

/// Every `use` import one file of an analysis writes, each with whether a name
/// in its scope reads it.
///
/// A name reads an import when it is an identifier spelled as the import's last
/// segment, in the same namespace scope, outside every `use` statement. An
/// identifier beside a `\` is part of a qualified name, and
/// `rule:statements/a-qualified-name-is-absolute` makes that one consult no
/// import at all. [`Import::read`] says which way this reading errs.
fn imports(analysed: &Analysed, loaded: &Loaded, path: &Path) -> Vec<Import> {
    let file = analysed.map.file(loaded.id);
    // The file parsed already, so lexing it again reports nothing new.
    let tokens = tokenize(file, &mut Diagnostics::new());
    let end = BytePos::try_from(file.text().len()).unwrap_or(BytePos::MAX);
    let scopes = namespace_scopes(&loaded.stmts, end);
    let mut statements = Vec::new();
    use_statements(&loaded.stmts, &mut statements);

    analysed
        .module
        .imports
        .iter()
        .filter(|import| import.span.file == loaded.id)
        .map(|import| {
            let scope = scope_of(&scopes, import.span.start);
            let read = tokens.iter().enumerate().any(|(at, token)| {
                token.kind == TokenKind::Ident
                    && text_of(file, token.span) == import.short_name
                    && scope_of(&scopes, token.span.start) == scope
                    && !statements
                        .iter()
                        .any(|used| covers(*used, token.span.start))
                    && !beside_a_backslash(&tokens, at)
            });
            Import {
                symbol: import.target.to_string(),
                site: site(path, import.span),
                read,
            }
        })
        .collect()
}

/// The byte ranges a `use` binds its short name over: one per bracketed
/// `namespace { … }` block, and one per run of statements a `namespace Name;`
/// statement starts, with the run before the first one included.
fn namespace_scopes(stmts: &[Stmt], end: BytePos) -> Vec<(BytePos, BytePos)> {
    let mut scopes = Vec::new();
    let mut start = 0;
    for stmt in stmts {
        if let StmtKind::NamespaceDecl(NamespaceDecl { body, .. }) = &stmt.kind {
            if body.is_some() {
                scopes.push((stmt.span.start, stmt.span.end));
            } else {
                scopes.push((start, stmt.span.start));
                start = stmt.span.start;
            }
        }
    }
    scopes.push((start, end));
    scopes
}

/// The narrowest scope that holds `at`, so a bracketed block wins over the run
/// of statements around it.
fn scope_of(scopes: &[(BytePos, BytePos)], at: BytePos) -> Option<(BytePos, BytePos)> {
    scopes
        .iter()
        .copied()
        .filter(|&(start, end)| start <= at && at < end)
        .min_by_key(|&(start, end)| end - start)
}

/// Every `use` statement in `stmts` and in the bracketed namespaces among them.
fn use_statements(stmts: &[Stmt], found: &mut Vec<Span>) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::UseDecl(_) => found.push(stmt.span),
            StmtKind::NamespaceDecl(NamespaceDecl {
                body: Some(block), ..
            }) => use_statements(&block.stmts, found),
            _ => {}
        }
    }
}

/// Whether the token at `at` has a `\` on either side, which makes it one
/// segment of a qualified name.
fn beside_a_backslash(tokens: &[Token], at: usize) -> bool {
    let backslash = |token: Option<&Token>| token.is_some_and(|it| it.kind == TokenKind::Backslash);
    backslash(at.checked_sub(1).and_then(|before| tokens.get(before)))
        || backslash(tokens.get(at + 1))
}

/// A span of one file, as a site the index can keep.
fn site(path: &Path, span: Span) -> Site {
    Site {
        path: path.to_path_buf(),
        start: span.start,
        end: span.end,
    }
}
