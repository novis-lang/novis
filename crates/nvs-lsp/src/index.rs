//! One workspace symbol index, built in one place.
//!
//! `rule:ide/five-features-are-one-reference-index` is this module's whole
//! specification: find-references, occurrence highlight, CodeLens, type
//! hierarchy and unused-member dimming are **five queries against one index**,
//! and none of them gets a walk of its own. What is here is the index, its one
//! construction site and its invalidation; the readers live in their own
//! modules and read it rather than building a second one.
//!
//! # Decision: a symbol is a string, and this module owns the spelling
//!
//! A reader asks about a *name*, and the three things a name can be here are a
//! type, a method and a property. They are spelled `App\User`,
//! `App\User::greet` and `App\User::$name` — the fully-qualified name the
//! checker resolved, then the member as its own source writes it, `$` sigil
//! included for a property because that is what tells it from a method of the
//! same name (`class C { public int $x; public function x(): int … }` is legal).
//! A field of an `attribute` declaration is `App\Job::queue`, after the
//! declaration's own name, which is `App\Storable::Index` for one a body owns.
//! A field's declaration is its name in the body, and each key a payload writes
//! for it is a use. A read of the field on a retrieval's result is not a use:
//! the checker types that read against the declaration's fields, which are a
//! shape, and records no link back to the declaration.
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
//! point. A `didChange` therefore re-indexes the file that changed and every
//! file whose entry was produced by an analysis that read it, and nothing
//! else, which is what [`SymbolIndex::refresh`] does and what `tests/index.rs`
//! pins. Following the `require` edge instead would need a second graph, kept
//! by hand, that says the same thing.
//!
//! # Decision: only the open documents are analysed before an answer
//!
//! A keystroke re-analyses the open documents among the readers at once,
//! because those are published and a cursor stands in them. Every other reader
//! waits in [`SymbolIndex::pending`], and the server's workers analyse them
//! while it answers ([`SymbolIndex::advance`], [`crate::workers`]). Each
//! result is filed on the server's thread in entry path order, making the
//! serial loop's decision at each entry's turn, so the index is the one
//! [`SymbolIndex::settle_one`] builds alone. The first build is the same
//! queue. So a completion or a hover waits for one analysis of its own
//! document, never for the tree. Its files stay indexed while it waits: their text did not
//! change, so only what a use in them resolved to can be behind. A reader that
//! needs every file, such as references, rename and the hierarchies, settles
//! the queue before it answers.
//!
//! An entry is analysed only when it would index a file nobody indexed yet, or
//! re-index files it owns. A library that many programs autoload is one graph
//! that every program and every test reaches, so this skips most of the tree.
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
//!
//! # What it costs
//!
//! An edit re-analyses the open entries whose graph read the changed file
//! before the next answer, and every other such entry when the server is idle.
//! The whole is linear in the readers per edit, and the part a cursor waits for
//! is linear in the open documents. The queue spends one path per waiting
//! entry.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use lsp_types::Uri;
use nvs_config::cache::{Digest, content_hash};
use nvs_diagnostics::{BytePos, Diagnostics, SourceFile, Span, canonical_key};
use nvs_hir::autoload::sole_declaration;
use nvs_hir::{Loaded, QName, SymbolKind};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, EnumCase, Modifier, NamespaceDecl, Stmt, StmtKind, Type,
    TypeAtom, TypeKind,
};
use nvs_syntax::{Token, TokenKind, tokenize, walk};
use nvs_types::ExprInfo;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::definition::{
    FileScope, Target, attribute_uses, body_of, covers, named_at, paired, supertype_names,
    target_of, text_of, written_types_in,
};
use crate::disk_index::{self, Input, Stamp};
use crate::document::{Analysed, Documents, Entry, Reads, Work, analyse_through};
use crate::session::Session;

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
    /// An `attribute Name on … { … }` declaration.
    Attribute,
    /// A method of one of the above.
    Method,
    /// A property of one of the above.
    Property,
    /// A class constant.
    Const,
    /// One `case` of an enum.
    EnumCase,
    /// One field of an `attribute` declaration, spelled `Name::field`.
    Field,
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
            SymbolKind::Attribute => Self::Attribute,
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
            Self::Attribute => "an attribute",
            Self::Method => "a method",
            Self::Property => "a property",
            Self::Const => "a constant",
            Self::EnumCase => "an enum case",
            Self::Field => "a field",
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
    /// The routes whose handler this method is, each as its verb in capitals
    /// and its path as written (`GET /users/{id}`), and empty for everything
    /// that is not a route handler.
    ///
    /// Read off the route table of the analysis that indexed the file, which
    /// is the table the server answers with. [`crate::reach`] is the reader.
    pub routes: Vec<String>,
    /// Whether this is a `#[Test]` method, read off the checker's own test
    /// table. [`crate::reach`] is the reader.
    pub test: bool,
    /// The bytes of the whole method, its attributes and its body included,
    /// and `None` for everything that is not a method.
    ///
    /// The "used by" lens is the only reader: a test counts when a coverage
    /// report saw it reach a line inside these bytes
    /// ([`crate::reports::Reports`]).
    pub extent: Option<(BytePos, BytePos)>,
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
    /// The method whose body this use is written in, spelled as its
    /// [`Declaration::symbol`] is, and `None` for a use outside every method
    /// body: top-level code, a property's default, a class constant.
    ///
    /// An occurrence that resolved to a method is therefore one edge of the
    /// call graph, from this method to that one
    /// (`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method`).
    /// A closure's body belongs to the method that writes it. A method taken
    /// as a callable value is `None` here, because it calls nothing where it
    /// is written; [`SymbolIndex::unfollowed`] counts it instead.
    pub caller: Option<String>,
}

/// One `use` import a file writes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import {
    /// The name it imports, fully qualified.
    pub symbol: String,
    /// Where its path was written.
    pub site: Site,
}

/// One file, as the analysis that reached it read it.
///
/// Built by [`analyse_entry`] and filed by [`SymbolIndex::merge`], and read
/// back from disk by [`crate::disk_index::decode`], which restores what an
/// earlier analysis built and analyses nothing.
#[derive(Debug)]
pub(crate) struct Indexed {
    /// Every name declared here, in source order.
    pub(crate) decls: Vec<Declaration>,
    /// Every resolved use written here, in source order.
    pub(crate) occurrences: Vec<Occurrence>,
    /// Every `use` import written here, in source order.
    pub(crate) imports: Vec<Import>,
    /// Every `Core\Test::request` written here with a literal method and path
    /// that the route table matched, in source order: the symbol is the
    /// handler of the route it matched, and the site is the path's literal.
    pub(crate) requests: Vec<Occurrence>,
    /// Every call the call graph does not follow, in source order: a method
    /// taken as a callable value, by its symbol, and a call by name that the
    /// checker did not resolve to one method, as `::` and the name.
    pub(crate) unfollowed: Vec<String>,
    /// Every call a recorded request can name by line (`nvs_types::core_records_site`), as the
    /// start and end of the call expression, in source order: the sites the recorded lens reads
    /// (`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`).
    pub(crate) queries: Vec<(BytePos, BytePos)>,
    /// The kind of the one type this file declares, when the file has the
    /// shape an autoloaded file must have (`nvs_hir::autoload::sole_declaration`).
    pub(crate) sole: Option<DeclKind>,
    /// Whether the file's parse, declarations or own bodies reported an error
    /// ([`Analysed`]'s `erring`), in which case a use written there may be
    /// missing from the index. An error of a pass over several files does not
    /// count: it hides no use, and an analysis that left files out has only
    /// some of their rows.
    pub(crate) errors: bool,
    /// Whether one of those errors is the lexer's or the parser's, so this
    /// file's text did not parse and its declarations may be missing.
    pub(crate) broken: bool,
    /// What other files can see of this file, one row per declaration
    /// ([`crate::surface`]). [`SymbolIndex::refresh`] diffs an edited file's
    /// old and new rows, and queues only the entries that own a file the
    /// changed rows reach.
    pub(crate) rows: crate::surface::Rows,
    /// What the file's last check looked up, or `None` where the analysis
    /// that built this record had no recording of it. A file with `None` is
    /// reached by every surface edit to a file its entry read.
    pub(crate) uses: Option<LookedUp>,
    /// The BLAKE3 hash of the text the analysis read. A load from disk
    /// compares it with the file's text now.
    pub(crate) content: Digest,
    /// The file's size and modification time, where they identify that text.
    /// `None` for an open buffer, whose text is not the file's, and for a
    /// file that changed too close to the analysis to tell.
    pub(crate) stamp: Option<Stamp>,
    /// The entry point that analysis started from, which is what re-indexing
    /// this file costs, and whose reads in [`SymbolIndex::reads`] are the edge
    /// [`SymbolIndex::refresh`] follows.
    pub(crate) entry: PathBuf,
}

/// The names one file's check found and failed to find (`nvs_hir::Uses`),
/// each list sorted.
///
/// They stay strings, written as a symbol is written everywhere in the index.
/// Interning them would buy memory alone, the last item of the priority
/// ordering, and would need a table beside the postings, which already keep
/// one string per distinct name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LookedUp {
    /// Each declaration a lookup found: `App\User`, `App\User::greet`, and
    /// `App\User::$` for the class's property list, which a codec reads.
    pub(crate) used: Vec<String>,
    /// Each name a lookup did not find: `App\User`, `App\User::greet` on the
    /// class it was looked up on, and `::greet` on a value of unknown type.
    pub(crate) missed: Vec<String>,
}

impl LookedUp {
    /// The names `uses` recorded, or `None` when it recorded nothing.
    fn of(uses: &nvs_hir::Uses) -> Option<Self> {
        uses.is_on().then(|| Self {
            used: uses.used_sorted().into_iter().map(str::to_owned).collect(),
            missed: uses
                .missed_sorted()
                .into_iter()
                .map(str::to_owned)
                .collect(),
        })
    }
}

/// The class a member name is written on: `App\User` for `App\User::greet`,
/// and `None` for a class name and for `::greet`.
fn owner_of(name: &str) -> Option<&str> {
    name.split_once("::")
        .map(|(owner, _)| owner)
        .filter(|owner| !owner.is_empty())
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
    /// Every entry of an indexed file's `requests`, by the handler it reached.
    by_request: Postings,
    /// Every entry of an indexed file's `unfollowed`, by what it names.
    by_unfollowed: Postings,
    /// Every name an indexed file's check used ([`LookedUp::used`]), by the
    /// name, and a member's also by its class in `by_used_on`. A refresh reads
    /// them to find the files an edit reaches.
    by_used: Postings,
    by_used_on: Postings,
    /// Every name an indexed file's check missed ([`LookedUp::missed`]), by
    /// the name, and a member's also by its receiver class in `by_missed_on`.
    by_missed: Postings,
    by_missed_on: Postings,
    /// The entry points whose analysis is waiting, in path order: the tree of
    /// a [`planned`](Self::planned) index, and each entry a
    /// [`refresh`](Self::refresh) found reading the changed file and did not
    /// analyse at once. One path per entry, freed as it is settled.
    pending: BTreeSet<PathBuf>,
    /// The entries in [`pending`](Self::pending) whose turn analyses only some
    /// of the files they own: the files a refresh's edit reaches. An entry
    /// queued whole is not here. One path per reached file, freed at its turn.
    partial: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    /// The work of every analysis [`absorb`](Self::absorb) ran since the last
    /// [`take_work`](Self::take_work): those of [`refresh`](Self::refresh)
    /// and of [`settle_one`](Self::settle_one) alike, because queued work is
    /// still work a keystroke caused.
    work: Work,
    /// How many times `files` gained or lost an entry or an entry's stamp
    /// moved. The server writes the index to disk when this differs from
    /// what it was at the last write ([`generation`](Self::generation)).
    generation: u64,
    /// The workers that analyse waiting entries off the server's thread, when
    /// [`use_workers`](Self::use_workers) asked for them. `None` settles each
    /// entry on the caller's thread.
    workers: Option<crate::workers::Pool>,
    /// How many times [`refresh`](Self::refresh) ran. A job carries the
    /// number from when it was sent.
    change: u64,
    /// Each file a [`refresh`](Self::refresh) was about while a job was in
    /// flight, with the counter it moved to. A result that read one of them
    /// after its job was sent is stale. Cleared once no entry waits.
    changed: FxHashMap<PathBuf, u64>,
    /// Each entry whose job panicked on a worker. Its turn skips it, and it is
    /// never analysed on the server's thread, until a refresh of its own file.
    panicked: BTreeSet<PathBuf>,
    /// The entries that panicked since [`take_panicked`](Self::take_panicked)
    /// last ran, for the server to log.
    panic_log: Vec<PathBuf>,
    /// Every configuration input of the entries' directories
    /// (`crate::tree_cache::inputs`), as it was when the index was planned or
    /// loaded. A directory found later adds its inputs as they are then.
    /// [`save_to`](Self::save_to) writes them, and a load that finds one
    /// changed discards the file. One path and one digest per input, usually
    /// one `nvs.toml` per directory.
    inputs: BTreeMap<PathBuf, Input>,
}

/// How a load decides that an indexed file still has the text its entry was
/// built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// A matching size and modification time is trusted, and only another
    /// file is read and hashed. The editor checks this way, since a stale
    /// answer there lasts one keystroke.
    Stamp,
    /// Every file is read and hashed. `nvs code` checks this way, since its
    /// answer must never be stale.
    Content,
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
        let mut index = Self::planned(documents, scope, root);
        index.settle(documents);
        index
    }

    /// [`build`](Self::build) with nothing analysed yet: every entry point the
    /// tree selects waits in [`pending`](Self::pending), and
    /// [`settle_one`](Self::settle_one) analyses them one at a time.
    ///
    /// The server holds this one and settles it while no message is waiting,
    /// so the first answer about an open document never waits for the whole
    /// tree. A reader that needs every file calls [`settle`](Self::settle)
    /// first.
    #[must_use]
    pub fn planned(documents: &Documents, scope: CheckScope, root: Option<&Path>) -> Self {
        Self::planned_tree(tree(documents, scope, root))
    }

    /// [`planned`](Self::planned) over a tree [`tree`] already walked.
    pub(crate) fn planned_tree(tree: Vec<(PathBuf, i32)>) -> Self {
        let mut index = Self {
            pending: tree.into_iter().map(|(path, _)| path).collect(),
            ..Self::default()
        };
        add_inputs(
            &mut index.inputs,
            index.pending.iter().map(PathBuf::as_path),
        );
        index
    }

    /// [`planned`](Self::planned), with every entry of the index file at
    /// `path` that is still right taken from it. Only the stale entry points
    /// wait in [`pending`](Self::pending), so a start on a tree the server saw
    /// before analyses the files that changed, and nothing more.
    ///
    /// The whole file is discarded, and the index is the planned one, for a
    /// wrong magic, format version, build stamp, root or checksum, for any
    /// error in its bytes, for a build that cannot identify itself, and for a
    /// `scope` other than `Workspace` or no `root`. Then each entry is checked
    /// against its file:
    ///
    /// - a size and modification time that match are trusted;
    /// - otherwise the file is hashed, and a hash that matches is trusted and
    ///   gets the file's time;
    /// - otherwise the file is parsed for its surface, and the entry point that
    ///   indexed it waits. If the surface differs, every entry point that read
    ///   the file waits too;
    /// - a file that is gone leaves the index, and every entry point that read
    ///   it waits.
    ///
    /// An entry point that indexed a file with an error waits as well: the
    /// class a use there could not find may exist now. Every file of the tree
    /// that the index does not hold waits, as it would in a planned index.
    #[must_use]
    pub fn load_from(
        documents: &Documents,
        scope: CheckScope,
        root: Option<&Path>,
        path: &Path,
    ) -> Self {
        let tree = tree(documents, scope, root);
        Self::load_from_tree(documents, scope, root, path, tree, Check::Stamp)
    }

    /// [`load_from`](Self::load_from) at `Workspace` scope, with every
    /// indexed file read and hashed ([`Check::Content`]) on one thread per
    /// core. A matching size and modification time is never trusted, so no
    /// entry is kept whose file changed. This is the load `nvs code` makes.
    #[must_use]
    pub fn load_by_content(documents: &Documents, root: &Path, path: &Path) -> Self {
        let tree = tree(documents, CheckScope::Workspace, Some(root));
        Self::load_from_tree(
            documents,
            CheckScope::Workspace,
            Some(root),
            path,
            tree,
            Check::Content,
        )
    }

    /// [`load_from`](Self::load_from) over a tree [`tree`] already walked,
    /// checking each file as `check` says.
    pub(crate) fn load_from_tree(
        documents: &Documents,
        scope: CheckScope,
        root: Option<&Path>,
        path: &Path,
        tree: Vec<(PathBuf, i32)>,
        check: Check,
    ) -> Self {
        match Self::loaded(documents, scope, root, path, &tree, check) {
            Some(index) => index,
            None => Self::planned_tree(tree),
        }
    }

    /// [`load_from`](Self::load_from), and `None` where it discards the file.
    fn loaded(
        documents: &Documents,
        scope: CheckScope,
        root: Option<&Path>,
        path: &Path,
        tree: &[(PathBuf, i32)],
        check: Check,
    ) -> Option<Self> {
        let root = root.filter(|_| scope == CheckScope::Workspace)?;
        let stamp = nvs_config::cache::build_stamp()?;
        let bytes = std::fs::read(path).ok()?;
        let saved = disk_index::decode(&bytes, stamp, &canonical_key(root))?;
        drop(bytes);

        // Every configuration input the saved entries' directories have now
        // must be recorded with the same content. A new include, a new
        // `nvs.toml`, another data folder or an edited `.nvsx` discards the
        // whole file.
        let mut inputs = BTreeMap::new();
        add_inputs(
            &mut inputs,
            saved.reads.iter().map(|(entry, _)| entry.as_path()),
        );
        let recorded: BTreeMap<PathBuf, Input> = saved.inputs.into_iter().collect();
        if inputs
            .iter()
            .any(|(input, now)| recorded.get(input) != Some(now))
        {
            return None;
        }

        let started = SystemTime::now();
        let mut index = Self {
            reads: saved.reads.into_iter().collect(),
            inputs,
            ..Self::default()
        };
        let mut stale = BTreeSet::new();
        let mut moved = Vec::new();
        let mut gone = Vec::new();
        let mut restamped = false;
        let fresh = freshen(&saved.files, started, check);
        for ((key, mut indexed), fresh) in saved.files.into_iter().zip(fresh) {
            match fresh {
                Freshness::Same => {}
                Freshness::Restamped(stamp) => {
                    indexed.stamp = stamp;
                    restamped = true;
                }
                Freshness::Changed(surface) => {
                    stale.insert(indexed.entry.clone());
                    if surface.as_ref().map(crate::surface::Rows::digest)
                        != Some(indexed.rows.digest())
                    {
                        moved.push((key.clone(), indexed.rows.clone(), surface));
                    }
                }
                Freshness::Gone => {
                    gone.push(key);
                    continue;
                }
            }
            if indexed.errors || open_document(documents, &key).is_some() {
                stale.insert(indexed.entry.clone());
            }
            index.post(&key, &indexed);
            index.files.insert(key, indexed);
        }
        // A file whose rows moved queues the readers that own a file its
        // edit reaches, as a refresh does. Every file is posted by now, so
        // the reach sees the whole saved tree.
        for (key, old, new) in &moved {
            let readers = index.readers_of(key);
            let reached = index.reached_entries(key, old, new.as_ref(), &readers);
            stale.extend(reached.into_iter().map(|(entry, _)| entry));
        }
        for key in &gone {
            stale.extend(index.readers_of(key));
        }
        // An entry point that is gone re-indexes nothing, so the files it
        // indexed leave with it, and the tree below queues those that remain.
        let gone_entries: Vec<PathBuf> = gone
            .iter()
            .filter(|key| index.reads.contains_key(*key))
            .cloned()
            .collect();
        index.drop_owned(&gone_entries);
        for key in &gone {
            stale.remove(key);
        }
        // An entry point the walk no longer selects, such as a file an ignore
        // file now names, indexes nothing in a cold build. The files it
        // indexed leave with it, and every entry point that read one of them
        // waits, so the first of them in path order indexes it again.
        let selected: BTreeSet<&Path> = tree.iter().map(|(entry, _)| entry.as_path()).collect();
        let unselected: Vec<PathBuf> = index
            .reads
            .keys()
            .filter(|entry| !selected.contains(entry.as_path()))
            .cloned()
            .collect();
        let orphaned = index.drop_owned(&unselected);
        for key in &orphaned {
            stale.extend(index.readers_of(key));
        }
        for entry in &unselected {
            stale.remove(entry);
        }
        index.pending = stale;
        for (entry, _) in tree {
            if !index.files.contains_key(entry) {
                index.pending.insert(entry.clone());
            }
        }
        add_inputs(
            &mut index.inputs,
            tree.iter().map(|(entry, _)| entry.as_path()),
        );
        // The index now says what the file says, except where a stamp moved
        // or a file left.
        index.generation = u64::from(restamped || !gone.is_empty() || !orphaned.is_empty());
        Some(index)
    }

    /// Writes the index to `path`, as the index of the workspace `root`.
    ///
    /// The bytes go to `<name>.<pid>.<n>.tmp` beside `path`, which is then
    /// renamed over it, so a reader sees the old file or the new one and never
    /// part of one. Two servers on one root each write a whole file, and the
    /// last rename wins. Nothing is synced: a file torn by a crash fails its
    /// checksum and is discarded at the next load.
    ///
    /// # Errors
    ///
    /// When this build cannot identify itself, when a path is not UTF-8, and
    /// when the write or the rename fails, as on Windows while another process
    /// has the file open. Every error deletes the temporary file and leaves
    /// `path` as it was.
    pub fn save_to(&self, path: &Path, root: &Path) -> std::io::Result<()> {
        let refused = |why: &str| std::io::Error::new(std::io::ErrorKind::Unsupported, why);
        let stamp = nvs_config::cache::build_stamp()
            .ok_or_else(|| refused("this build cannot identify itself"))?;
        // An entry whose directory the index found after it was planned
        // records that directory's inputs as they are now.
        let mut inputs = self.inputs.clone();
        add_inputs(&mut inputs, self.reads.keys().map(PathBuf::as_path));
        let bytes = disk_index::encode(
            self.files.iter(),
            self.reads.iter(),
            inputs.iter(),
            stamp,
            &canonical_key(root),
        )
        .ok_or_else(|| refused("a path is not UTF-8"))?;
        disk_index::write(path, &bytes)
    }

    /// A number that changes whenever what [`save_to`](Self::save_to) writes
    /// changes. The server writes the index when it differs from the number
    /// at its last write.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Re-indexes `changed` and every file whose analysis read it. The open
    /// entries among them are re-analysed now, and every other entry is added
    /// to [`pending`](Self::pending). Returns the files dropped now and the
    /// analyses of the open entries.
    ///
    /// When `changed` is open and its new analysis gives the same
    /// declaration-surface hash ([`crate::surface`]) as before, and read the
    /// same files as its last analysis, no reader is added to `pending`: the
    /// edit was inside a body, and nothing another file resolves against
    /// moved. The open document's own analysis re-indexes it. A body that
    /// names a class no file of the graph named before loads that class's
    /// file, which every reader's graph then reads too, so that edit queues
    /// the readers. Where the hash differs, only the readers that own a file
    /// the changed rows reach wait ([`reached_entries`](Self::reached_entries)):
    /// an entry whose files used nothing that moved keeps its records.
    ///
    /// The dropped set is the answer the test asks for and the one a caller
    /// wants anyway: it is exactly the files whose declarations and
    /// occurrences have just been replaced, so a reader holding an answer
    /// about one of them knows it is stale.
    ///
    /// An open entry is analysed under its document's own path and version,
    /// which is exactly the analysis [`crate::analyse`] makes of that document,
    /// so the server publishes from it instead of running the front end over
    /// the same version a second time.
    ///
    /// A pending entry keeps its files indexed until it is settled. Their own
    /// text did not change, so their declarations and the sites of their
    /// occurrences are still right, and only what a use in them resolved to
    /// may be behind the edit. That is what lets a keystroke cost the open
    /// documents' analyses alone: in a library many programs read, every
    /// program and test is a reader of every file.
    pub fn refresh(&mut self, documents: &Documents, changed: &Path) -> Refreshed {
        let key = canonical_key(changed);
        self.change += 1;
        // An entry that panicked on a worker recorded no reads, so it is no
        // reader of its own file. Its file changed, so it waits again.
        if self.panicked.remove(&key) {
            self.queue_whole(key.clone());
        }
        if self.workers.is_some() {
            self.changed.insert(key.clone(), self.change);
        }
        let readers = self.readers_of(&key);
        let mut now: Vec<PathBuf> = readers
            .iter()
            .filter(|entry| open_document(documents, entry).is_some())
            .cloned()
            .collect();
        // A file the index has never seen — one just opened, or one created
        // under a workspace root — is an entry point in its own right, and
        // nothing above found it because nothing had read it yet. An open file
        // that a pending entry owns is re-indexed by its own analysis.
        let reindexed = open_document(documents, &key).is_some();
        if reindexed {
            now.push(key.clone());
        }
        now.sort();
        now.dedup();
        let held: Vec<PathBuf> = readers
            .into_iter()
            .filter(|entry| !now.contains(entry))
            .collect();
        // A file nobody has open is re-indexed only by its readers' analyses,
        // so they all wait. An open one is re-indexed below, and its readers
        // wait only if what they can see of it changed.
        if !reindexed {
            for entry in &held {
                self.queue_whole(entry.clone());
            }
        }
        let before = self.files.get(&key).map(|indexed| indexed.rows.clone());
        let read_before = self.read_set(&key);

        let mut dropped = self.drop_owned(&now);
        if reindexed && let Some(indexed) = self.files.remove(&key) {
            self.unpost(&key, &indexed);
            dropped.push(key.clone());
        }
        dropped.sort();
        dropped.dedup();
        let mut analysed = Vec::new();
        for entry in now {
            self.unqueue(&entry);
            let Some(document) = open_document(documents, &entry) else {
                continue;
            };
            let Some(path) = document.path() else {
                continue;
            };
            let at = Entry { path, key: &entry };
            if let Some(analysis) = self.absorb(documents, at, document.version()) {
                analysed.push((document.uri().clone(), analysis));
            }
        }
        if reindexed {
            // An entry that owned no file kept no reads, so a graph it did not
            // record is taken to be the same. The `didOpen` before every edit
            // records one.
            let graph_moved = read_before.is_some() && read_before != self.read_set(&key);
            let after = self.files.get(&key).map(|indexed| &indexed.rows);
            match before {
                Some(before) if !graph_moved => {
                    if Some(before.digest()) != after.map(crate::surface::Rows::digest) {
                        let reached = self.reached_entries(&key, &before, after, &held);
                        self.queue_reached(reached);
                    }
                }
                _ => {
                    for entry in held {
                        self.queue_whole(entry);
                    }
                }
            }
        }
        Refreshed { dropped, analysed }
    }

    /// The files the last analysis of `entry` read, as a set, where it kept
    /// them.
    fn read_set(&self, entry: &Path) -> Option<BTreeSet<PathBuf>> {
        self.reads
            .get(entry)
            .map(|reads| reads.iter().cloned().collect())
    }

    /// Analyses the first entry point in [`pending`](Self::pending) in path
    /// order, and returns whether any is left.
    ///
    /// An entry whose own file is already indexed and that owns no file is
    /// skipped. Its analysis would read the graph another entry already
    /// indexed, and [`absorb`](Self::absorb) keeps the first analysis that
    /// reaches a file. Where a library is shared by many programs, this skips
    /// most of the tree.
    pub fn settle_one(&mut self, documents: &Documents) -> bool {
        let Some(entry) = self.pending.pop_first() else {
            return false;
        };
        if self.skips(&entry) {
            self.partial.remove(&entry);
        } else {
            let before = self.drop_for_turn(&entry);
            self.analyse_here(documents, &entry);
            self.after_turn(&entry, before);
        }
        if self.pending.is_empty() {
            // The tree is settled, so a parse kept for a file no entry indexed
            // is a file that left it.
            documents.settled(|path| self.files.contains_key(path));
            return false;
        }
        true
    }

    /// Whether `entry`'s turn skips it: its own file is indexed and it owns
    /// no file, so its analysis would read a graph another entry indexed.
    fn skips(&self, entry: &Path) -> bool {
        self.panicked.contains(entry)
            || (self.files.contains_key(entry) && self.owned_by(entry).is_empty())
    }

    /// The entries whose job panicked on a worker since the last call, each
    /// once. The server logs each with its path.
    pub fn take_panicked(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.panic_log)
    }

    /// Analyses `entry` on this thread, through a snapshot of the store, and
    /// files what it found.
    fn analyse_here(&mut self, documents: &Documents, entry: &Path) {
        let snapshot = documents.snapshot();
        let version = snapshot.version_of(entry).unwrap_or(NO_BUFFER);
        let (mut session, mut trees) = documents.caches();
        let files = &self.files;
        let at = Entry {
            path: entry,
            key: entry,
        };
        let analysed = analyse_entry(
            &snapshot,
            &mut session,
            &mut trees,
            at,
            version,
            |key| files.contains_key(key),
            true,
            true,
        );
        drop((session, trees));
        if let Some((_, analysis)) = analysed {
            self.merge(analysis);
        }
    }

    /// Analyses every entry in [`pending`](Self::pending), which is what a
    /// reader that needs every file asks for first. With workers, it waits
    /// for them to drain.
    pub fn settle(&mut self, documents: &Documents) {
        if let Some(mut pool) = self.workers.take() {
            while self.turns(documents, &mut pool) {
                pool.wait();
            }
            self.workers = Some(pool);
            return;
        }
        while self.settle_one(documents) {}
    }

    /// Settles the waiting entries on `count` worker threads from now on
    /// (`rule:ide/the-server-is-synchronous`). A count of zero is one per core
    /// but one, and at least one.
    pub fn use_workers(&mut self, count: usize) {
        self.workers = Some(if count == 0 {
            crate::workers::Pool::sized_to_the_machine()
        } else {
            crate::workers::Pool::new(count)
        });
    }

    /// Stops the workers and waits for each to end. Every entry that still
    /// waits is settled on the caller's thread from now on.
    pub fn stop_workers(&mut self) {
        if let Some(mut pool) = self.workers.take() {
            pool.stop();
        }
    }

    /// How many worker threads run now. A settled index has none.
    #[must_use]
    pub fn workers_running(&self) -> usize {
        self.workers
            .as_ref()
            .map_or(0, crate::workers::Pool::running)
    }

    /// One step of settling without waiting: files every worker result whose
    /// turn has come, in entry path order, and sends the next entries to the
    /// workers. Returns whether any entry still waits. Without workers it is
    /// [`settle_one`](Self::settle_one).
    pub fn advance(&mut self, documents: &Documents) -> bool {
        let Some(mut pool) = self.workers.take() else {
            return self.settle_one(documents);
        };
        pool.collect();
        let left = self.turns(documents, &mut pool);
        self.workers = Some(pool);
        left
    }

    /// The side worker results arrive on, for a caller that waits on it beside
    /// other input. `None` without workers.
    pub(crate) fn results(&self) -> Option<crossbeam_channel::Receiver<crate::workers::Done>> {
        self.workers.as_ref().map(crate::workers::Pool::results)
    }

    /// Keeps a result a caller took off [`results`](Self::results) until its
    /// turn.
    pub(crate) fn receive(&mut self, done: crate::workers::Done) {
        if let Some(pool) = &mut self.workers {
            pool.receive(done);
        }
    }

    /// Takes the waiting entries in path order, as [`settle_one`] does, until
    /// the first one whose result has not arrived. Returns whether any entry
    /// still waits, after sending as many jobs as the pool has room for.
    ///
    /// Each turn is the serial loop's turn, made on the state every earlier
    /// turn left, so the index is the one [`settle_one`] builds. A result is
    /// analysed again here when something it relied on moved since its job
    /// was sent: a file it read was refreshed, or a file it took as indexed
    /// was dropped since.
    ///
    /// [`settle_one`]: Self::settle_one
    fn turns(&mut self, documents: &Documents, pool: &mut crate::workers::Pool) -> bool {
        // An entry a refresh took out of the queue keeps no claim.
        pool.retain(|entry| self.pending.contains(entry));
        loop {
            self.work += pool.take_work();
            // Filed before the next job is sent, so that job reads them.
            for made in pool.take_parses() {
                documents.file_parses(made);
            }
            for entry in pool.take_panicked() {
                if self.panicked.insert(entry.clone()) {
                    self.panic_log.push(entry);
                }
            }
            let Some(head) = self.pending.first().cloned() else {
                pool.stop();
                self.changed.clear();
                documents.settled(|path| self.files.contains_key(path));
                return false;
            };
            if let Some((change, parts)) = pool.take(&head) {
                self.file_parts(documents, &head, change, parts);
                continue;
            }
            if pool.is_running(&head) {
                break;
            }
            if self.skips(&head) {
                self.unqueue(&head);
                continue;
            }
            match self.send(documents, pool, &head) {
                Sent::Plan | Sent::Parts => break,
                Sent::Nothing | Sent::Failed => {
                    self.pending.remove(&head);
                    let before = self.drop_for_turn(&head);
                    self.analyse_here(documents, &head);
                    self.after_turn(&head, before);
                }
            }
        }
        // Entries ahead of their turn are split too, in path order, and each
        // only once every entry before it was. One whose own file is indexed
        // or claimed and that owns nothing is left for its turn, which skips
        // it unless an earlier turn dropped that file.
        while pool.has_room() {
            let next = match &pool.cursor {
                Some(cursor) => self
                    .pending
                    .range::<PathBuf, _>((
                        std::ops::Bound::Excluded(cursor),
                        std::ops::Bound::Unbounded,
                    ))
                    .next()
                    .cloned(),
                None => self.pending.first().cloned(),
            };
            let Some(next) = next else {
                break;
            };
            if pool.is_planning(&next) {
                break;
            }
            let passed = pool.is_split(&next)
                || self.panicked.contains(&next)
                || ((self.files.contains_key(&next) || pool.is_claimed(&next))
                    && self.owned_by(&next).is_empty());
            if !passed {
                match self.send(documents, pool, &next) {
                    Sent::Plan | Sent::Failed => break,
                    Sent::Parts | Sent::Nothing => {}
                }
            }
            pool.cursor = Some(next);
        }
        // Entries past the cursor whose graph nobody knows are planned now,
        // in path order, in the room the split left. A plan claims nothing,
        // so the order of filing does not change. They start once one plan
        // came back, whose parses the next plans read. An entry whose own
        // file a known graph reaches is left for its turn, which most often
        // skips it. So is one that borrows the `autoload` map of a waiting
        // program whose graph is not known yet: that program's plan most
        // often loads it, and a plan of its own beside that one would parse
        // the same files on a second worker.
        while pool.has_room_to_plan() {
            let after = pool.plan_cursor.as_ref().max(pool.cursor.as_ref());
            let next = match after {
                Some(after) => self
                    .pending
                    .range::<PathBuf, _>((
                        std::ops::Bound::Excluded(after),
                        std::ops::Bound::Unbounded,
                    ))
                    .next()
                    .cloned(),
                None => self.pending.first().cloned(),
            };
            let Some(next) = next else {
                break;
            };
            let known = pool.is_sent(&next)
                || self.panicked.contains(&next)
                || pool.graph(&next).is_some()
                || self.reads.contains_key(&next)
                || self.files.contains_key(&next)
                || pool.is_reached(&next)
                || Reads::lender_for(documents, &next).is_some_and(|lender| {
                    let program = lender.entry();
                    self.pending.contains(program)
                        && pool.graph(program).is_none()
                        && !self.reads.contains_key(program)
                });
            if !known && !matches!(self.send(documents, pool, &next), Sent::Plan) {
                break;
            }
            pool.plan_cursor = Some(next);
        }
        true
    }

    /// Sends `entry` to the workers. An entry whose graph is unknown is sent
    /// as a plan. Otherwise the graph's files that no other entry indexed and
    /// no entry in flight claims are split into one part per worker, in path
    /// order.
    ///
    /// The graph is the one the index kept for the entry's last analysis, else
    /// the one its plan returned. A graph that moved since leaves a file out
    /// of every part or puts in one the entry does not load, and its turn then
    /// analyses it on this thread ([`file_parts`](Self::file_parts)).
    fn send(&self, documents: &Documents, pool: &mut crate::workers::Pool, entry: &Path) -> Sent {
        // A pool that runs no worker yet pins the whole tree first, so a parse
        // a worker makes is at the id the server gives the same file.
        let tree: Vec<&Path> = if pool.running() == 0 {
            self.pending
                .iter()
                .chain(self.files.keys())
                .map(PathBuf::as_path)
                .collect()
        } else {
            vec![entry]
        };
        let parses = documents.share_parses(tree);
        let job = |task, part| {
            let snapshot = documents.snapshot();
            let version = snapshot.version_of(entry).unwrap_or(NO_BUFFER);
            crate::workers::Job {
                entry: entry.to_path_buf(),
                snapshot,
                version,
                task,
                part,
                change: self.change,
                parses: parses.clone(),
            }
        };
        let graph = match self.reads.get(entry) {
            Some(reads) => reads.as_slice(),
            None => match pool.graph(entry) {
                Some(graph) => graph,
                None => {
                    return if pool.send_plan(job(crate::workers::Task::Plan, 0)) {
                        Sent::Plan
                    } else {
                        Sent::Failed
                    };
                }
            },
        };
        // A partial entry checks again only the files the edit reached, and
        // its other files keep their records.
        let owned = match self.partial.get(entry) {
            Some(reached) => reached.iter().cloned().collect(),
            None => self.owned_by(entry),
        };
        let mut free: Vec<PathBuf> = graph
            .iter()
            .filter(|file| {
                (!self.files.contains_key(*file) || owned.contains(file)) && !pool.is_claimed(file)
            })
            .cloned()
            .collect();
        free.sort();
        free.dedup();
        if free.is_empty() {
            return Sent::Nothing;
        }
        let size = free.len().div_ceil(pool.size().min(free.len()));
        let jobs = free
            .chunks(size)
            .enumerate()
            .map(|(part, files)| {
                let files: FxHashSet<PathBuf> = files.iter().cloned().collect();
                job(crate::workers::Task::Part(Arc::new(files)), part)
            })
            .collect();
        if pool.send_parts(jobs, free) {
            Sent::Parts
        } else {
            Sent::Failed
        }
    }

    /// Files the parts of the entry whose turn it is, as [`settle_one`] would
    /// have filed its own analysis.
    ///
    /// The parts are joined into one analysis: each part's records, and the
    /// first part's reads. A file the graph loaded that no part recorded and
    /// the index does not hold, because the graph moved or an earlier entry's
    /// result was not filed, sends the entry to this thread's analysis.
    ///
    /// [`settle_one`]: Self::settle_one
    fn file_parts(
        &mut self,
        documents: &Documents,
        entry: &Path,
        change: u64,
        parts: Vec<Option<Analysis>>,
    ) {
        let moved = |key: &Path| self.changed.get(key).is_some_and(|at| *at > change);
        let stale = moved(entry)
            || parts
                .iter()
                .flatten()
                .any(|analysis| analysis.reads.iter().any(|read| moved(read)));
        for analysis in parts.iter().flatten() {
            self.work += analysis.work;
        }
        // A file it read changed after it was sent: it stays waiting, and its
        // next turn sends it again.
        if stale {
            return;
        }
        self.pending.remove(entry);
        if self.skips(entry) {
            self.partial.remove(entry);
            return;
        }
        let before = self.drop_for_turn(entry);
        if parts.iter().all(Option::is_none) {
            return;
        }
        if parts.iter().any(Option::is_none) {
            self.analyse_here(documents, entry);
            self.after_turn(entry, before);
            return;
        }
        let mut records: Vec<(PathBuf, Indexed)> = Vec::new();
        let mut skipped: FxHashSet<PathBuf> = FxHashSet::default();
        let mut reads = Vec::new();
        for part in parts.into_iter().flatten() {
            if reads.is_empty() {
                reads = part.reads;
            }
            records.extend(part.records);
            skipped.extend(part.skipped);
        }
        let recorded: FxHashSet<&PathBuf> = records.iter().map(|(key, _)| key).collect();
        if skipped
            .iter()
            .any(|key| !recorded.contains(key) && !self.files.contains_key(key))
        {
            self.analyse_here(documents, entry);
            self.after_turn(entry, before);
            return;
        }
        // In the order the graph loaded them, which is the serial analysis's.
        let order: FxHashMap<&PathBuf, usize> = reads
            .iter()
            .enumerate()
            .map(|(at, key)| (key, at))
            .collect();
        records.sort_by_key(|(key, _)| order.get(key).copied().unwrap_or(usize::MAX));
        self.merge(Analysis {
            entry: entry.to_path_buf(),
            records,
            reads,
            skipped: Vec::new(),
            work: Work::default(),
        });
        self.after_turn(entry, before);
    }

    /// The entry points that are waiting for [`settle_one`](Self::settle_one),
    /// in path order.
    #[must_use]
    pub fn pending(&self) -> &BTreeSet<PathBuf> {
        &self.pending
    }

    /// The files every analysis since the last [`take_work`](Self::take_work)
    /// parsed, and the files whose bodies it checked, summed.
    #[must_use]
    pub fn work(&self) -> Work {
        self.work
    }

    /// [`work`](Self::work), and the count starts again from zero.
    pub fn take_work(&mut self) -> Work {
        std::mem::take(&mut self.work)
    }

    /// Drops every file the entries in `entries` indexed, and their reads, and
    /// returns the files in path order.
    fn drop_owned(&mut self, entries: &[PathBuf]) -> Vec<PathBuf> {
        let mut stale: Vec<PathBuf> = entries
            .iter()
            .flat_map(|entry| self.owned_by(entry))
            .collect();
        stale.sort();
        stale.dedup();
        for path in &stale {
            if let Some(indexed) = self.files.remove(path) {
                self.unpost(path, &indexed);
            }
        }
        for entry in entries {
            self.reads.remove(entry);
        }
        stale
    }

    /// The files `entry` indexed, in path order.
    ///
    /// Every file an entry indexed is one its analysis read, so its reads are
    /// the only files to look at, and an entry that indexed none has no reads
    /// kept. Looking through every file of the index instead cost one pass
    /// over the whole index per settled entry, which grows with the square of
    /// the tree.
    fn owned_by(&self, entry: &Path) -> Vec<PathBuf> {
        self.reads.get(entry).map_or_else(Vec::new, |reads| {
            reads
                .iter()
                .filter(|read| {
                    self.files
                        .get(*read)
                        .is_some_and(|indexed| indexed.entry == entry)
                })
                .cloned()
                .collect()
        })
    }

    /// The entry points whose analysis read `path`, in path order.
    ///
    /// `crate::rename` reads it when a file moves: every `require` that names
    /// the file is in a graph one of these entries loads.
    #[must_use]
    pub fn readers(&self, path: &Path) -> Vec<PathBuf> {
        self.readers_of(&canonical_key(path))
    }

    /// The entry points whose analysis read `key`, in path order.
    fn readers_of(&self, key: &Path) -> Vec<PathBuf> {
        self.reads
            .iter()
            .filter(|(_, reads)| reads.iter().any(|read| read == key))
            .map(|(entry, _)| entry.clone())
            .collect()
    }

    /// The entries among `readers` that own a file which an edit of `key`
    /// from `old` to `new` rows reaches, in path order, each with the files it
    /// owns that the edit reaches, or `None` where it is analysed whole.
    ///
    /// A file is reached when its check output's key reads a row the edit
    /// moved (`crate::file_key`): a name it used, a name it failed to find
    /// that may now be found, or a row of a class above a class it declares.
    /// A graph row or an attribute that moved, and no new rows, reach every
    /// reader: routes, commands, links and the graph are read across the
    /// whole graph. A reader that owns a file with no recorded uses is
    /// always analysed whole. `readers` is in path order.
    fn reached_entries(
        &self,
        key: &Path,
        old: &crate::surface::Rows,
        new: Option<&crate::surface::Rows>,
        readers: &[PathBuf],
    ) -> Vec<(PathBuf, Option<BTreeSet<PathBuf>>)> {
        use crate::surface::{How, Kind};
        let every = || readers.iter().map(|entry| (entry.clone(), None)).collect();
        let Some(new) = new else {
            return every();
        };
        let changes = crate::surface::diff(old, new);
        if changes
            .iter()
            .any(|change| matches!(change.kind, Kind::Graph | Kind::Attributes))
        {
            return every();
        }
        let mut files: BTreeSet<&Path> = BTreeSet::new();
        for change in &changes {
            let member = !matches!(change.kind, Kind::Header | Kind::Cases | Kind::Properties);
            let class = if member {
                owner_of(&change.symbol).unwrap_or(&change.symbol)
            } else {
                change.symbol.as_str()
            };
            let (below, declaring) = self.below(class);
            files.extend(declaring);
            match change.kind {
                Kind::Header => {
                    self.posted_files(&self.by_used, class, &mut files);
                    self.posted_files(&self.by_used_on, class, &mut files);
                    self.posted_files(&self.by_missed, class, &mut files);
                    for receiver in std::iter::once(class).chain(below.iter().map(String::as_str)) {
                        self.posted_files(&self.by_missed_on, receiver, &mut files);
                    }
                }
                Kind::Cases => self.posted_files(&self.by_used, class, &mut files),
                Kind::Properties => {
                    self.posted_files(&self.by_used, &format!("{class}::$"), &mut files)
                }
                _ => {
                    self.posted_files(&self.by_used, &change.symbol, &mut files);
                    if change.how != How::Changed
                        && let Some((_, name)) = change.symbol.split_once("::")
                    {
                        self.posted_files(&self.by_missed, &format!("::{name}"), &mut files);
                        for receiver in
                            std::iter::once(class).chain(below.iter().map(String::as_str))
                        {
                            self.posted_files(
                                &self.by_missed,
                                &format!("{receiver}::{name}"),
                                &mut files,
                            );
                        }
                    }
                }
            }
        }
        files.remove(key);
        let mut entries: BTreeMap<&Path, Option<BTreeSet<PathBuf>>> = BTreeMap::new();
        for file in files {
            if let Some(indexed) = self.files.get(file) {
                let reached = entries
                    .entry(indexed.entry.as_path())
                    .or_insert_with(|| Some(BTreeSet::new()));
                if let Some(reached) = reached {
                    reached.insert(file.to_path_buf());
                }
            }
        }
        for reader in readers {
            if self.owned_by(reader).iter().any(|file| {
                self.files
                    .get(file)
                    .is_some_and(|indexed| indexed.uses.is_none())
            }) {
                entries.insert(reader, None);
            }
        }
        readers
            .iter()
            .filter_map(|reader| {
                let reached = entries.remove(reader.as_path())?;
                Some((reader.clone(), reached))
            })
            .collect()
    }

    /// Queues `entry` for a whole analysis.
    fn queue_whole(&mut self, entry: PathBuf) {
        self.partial.remove(&entry);
        self.pending.insert(entry);
    }

    /// Queues `entry` to analyse again only the files in `reached`, which it
    /// owns. An entry already waiting for a whole analysis stays whole.
    fn queue_part(&mut self, entry: PathBuf, reached: BTreeSet<PathBuf>) {
        if self.pending.contains(&entry) && !self.partial.contains_key(&entry) {
            return;
        }
        self.partial
            .entry(entry.clone())
            .or_default()
            .extend(reached);
        self.pending.insert(entry);
    }

    /// Queues each of `reached` as [`reached_entries`](Self::reached_entries)
    /// gave it.
    fn queue_reached(&mut self, reached: Vec<(PathBuf, Option<BTreeSet<PathBuf>>)>) {
        for (entry, files) in reached {
            match files {
                Some(files) => self.queue_part(entry, files),
                None => self.queue_whole(entry),
            }
        }
    }

    /// Takes `entry` out of the queue, whole or partial.
    fn unqueue(&mut self, entry: &Path) {
        self.partial.remove(entry);
        self.pending.remove(entry);
    }

    /// Drops what `entry`'s turn analyses again: the files a partial queueing
    /// reached that it still owns, or else every file it owns. Returns the
    /// files it read before, where the turn is partial.
    fn drop_for_turn(&mut self, entry: &Path) -> Option<BTreeSet<PathBuf>> {
        let Some(reached) = self.partial.remove(entry) else {
            self.drop_owned(&[entry.to_path_buf()]);
            return None;
        };
        let before = self.read_set(entry);
        for path in reached {
            if self
                .files
                .get(&path)
                .is_some_and(|indexed| indexed.entry == entry)
                && let Some(indexed) = self.files.remove(&path)
            {
                self.unpost(&path, &indexed);
            }
        }
        before
    }

    /// After a partial turn of `entry`: where its analysis read other files
    /// than before, every entry that reads a file it owns waits for a whole
    /// analysis, as a graph that moved does in a refresh.
    fn after_turn(&mut self, entry: &Path, before: Option<BTreeSet<PathBuf>>) {
        let Some(before) = before else {
            return;
        };
        if self.read_set(entry).is_none_or(|now| now == before) {
            return;
        }
        let mut readers = BTreeSet::new();
        for file in self.owned_by(entry) {
            readers.extend(self.readers_of(&file));
        }
        readers.remove(entry);
        for reader in readers {
            self.queue_whole(reader);
        }
    }

    /// Adds to `files` every indexed file `postings` has under `name`.
    fn posted_files<'a>(&'a self, postings: &Postings, name: &str, files: &mut BTreeSet<&'a Path>) {
        for (path, _) in postings.get(name).into_iter().flatten() {
            if let Some((stored, _)) = self.files.get_key_value(&**path) {
                files.insert(stored.as_path());
            }
        }
    }

    /// Every class below `class` at any depth, each once, and the files that
    /// declare them, through [`by_supertype`](Self::by_supertype).
    fn below(&self, class: &str) -> (BTreeSet<String>, BTreeSet<&Path>) {
        let mut classes = BTreeSet::new();
        let mut files = BTreeSet::new();
        let mut queue = vec![class.to_owned()];
        while let Some(above) = queue.pop() {
            for (path, at) in self.by_supertype.get(&above).into_iter().flatten() {
                let Some((stored, indexed)) = self.files.get_key_value(&**path) else {
                    continue;
                };
                files.insert(stored.as_path());
                if let Some(declared) = indexed.decls.get(*at)
                    && declared.symbol != class
                    && classes.insert(declared.symbol.clone())
                {
                    queue.push(declared.symbol.clone());
                }
            }
        }
        (classes, files)
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

    /// Every declaration in the index whose own name contains `query`, upper
    /// and lower case alike, sorted by symbol and then by where it was written.
    ///
    /// The own name is the last segment of a type and the member of a member,
    /// without a property's `$`, which is the name a person types to find one.
    /// An empty `query` matches every declaration.
    #[must_use]
    pub fn matching(&self, query: &str) -> Vec<&Declaration> {
        let query = query.to_lowercase();
        let mut found: Vec<&Declaration> = self
            .files
            .values()
            .flat_map(|indexed| &indexed.decls)
            .filter(|declared| own_name(&declared.symbol).to_lowercase().contains(&query))
            .collect();
        found.sort_by(|left, right| (&left.symbol, &left.site).cmp(&(&right.symbol, &right.site)));
        found
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

    /// Every `Core\Test::request` whose literal method and path the route
    /// table matched to a route `handler` answers, in file and then source
    /// order. Each one's caller is the method that sends the request.
    ///
    /// Apart from [`Self::occurrences`] because the request names the handler
    /// nowhere in its source: a reference list does not show it, and the call
    /// graph does.
    #[must_use]
    pub fn requests(&self, handler: &str) -> Vec<&Occurrence> {
        self.by_request
            .get(handler)
            .into_iter()
            .flatten()
            .filter_map(|(path, at)| self.files.get(&**path)?.requests.get(*at))
            .collect()
    }

    /// Whether a call the call graph does not follow could reach `method`:
    /// the method is taken as a callable value somewhere, or a call the
    /// checker did not resolve names a method of the same name.
    #[must_use]
    pub fn unfollowed(&self, method: &str) -> bool {
        let by_name = method
            .rsplit_once("::")
            .is_some_and(|(_, name)| self.by_unfollowed.contains_key(&format!("::{name}")));
        by_name || self.by_unfollowed.contains_key(method)
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

    /// Every name every file declares, in path and then source order. A walk
    /// of the stored entries, so no path is resolved again.
    pub fn all_declarations(&self) -> impl Iterator<Item = &Declaration> {
        self.files.values().flat_map(|indexed| &indexed.decls)
    }

    /// Every use `path` writes, in source order, or nothing for a file the
    /// index does not hold.
    #[must_use]
    pub fn occurrences_in(&self, path: &Path) -> &[Occurrence] {
        self.files
            .get(&canonical_key(path))
            .map_or(&[], |indexed| indexed.occurrences.as_slice())
    }

    /// Every call `path` writes that a recorded request can name by line, as the start and end of
    /// the call, in source order, or nothing for a file the index does not hold.
    #[must_use]
    pub fn queries_in(&self, path: &Path) -> &[(BytePos, BytePos)] {
        self.files
            .get(&canonical_key(path))
            .map_or(&[], |indexed| indexed.queries.as_slice())
    }

    /// Every `Core\Test::request` `path` sends that the route table matched,
    /// in source order, or nothing for a file the index does not hold.
    #[must_use]
    pub fn requests_in(&self, path: &Path) -> &[Occurrence] {
        self.files
            .get(&canonical_key(path))
            .map_or(&[], |indexed| indexed.requests.as_slice())
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

    /// Whether the index holds `path` and its text parsed, with no lexer or
    /// parser error in the analysis that indexed it.
    #[must_use]
    pub fn parses(&self, path: &Path) -> bool {
        self.files
            .get(&canonical_key(path))
            .is_some_and(|indexed| !indexed.broken)
    }

    /// How many files the index holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// How many of the files the index holds had an error in the analysis
    /// that indexed them. `nvs code` reports this, because a use inside such
    /// a file may not have resolved.
    #[must_use]
    pub fn files_with_errors(&self) -> usize {
        self.files.values().filter(|indexed| indexed.errors).count()
    }

    /// Whether the index holds nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Indexes every file one analysis of `entry` reaches, reading the store
    /// in place. [`refresh`](Self::refresh) analyses each open entry this way.
    ///
    /// Returns the analysis it indexed, for [`refresh`](Self::refresh) to hand
    /// on.
    fn absorb(
        &mut self,
        documents: &Documents,
        entry: Entry<'_>,
        version: i32,
    ) -> Option<Analysed> {
        let (mut session, mut trees) = documents.caches();
        let files = &self.files;
        let (analysed, analysis) = analyse_entry(
            documents,
            &mut session,
            &mut trees,
            entry,
            version,
            |key| files.contains_key(key),
            true,
            true,
        )?;
        drop((session, trees));
        self.merge(analysis);
        Some(analysed)
    }

    /// Files what one [`analyse_entry`] found.
    ///
    /// **This is the crate's one symbol-index construction site**, and
    /// `tests/index.rs` is what says so: every other module here reads a
    /// `&SymbolIndex`, and a second feature walking the front end for names of
    /// its own is the thing `rule:ide/five-features-are-one-reference-index`
    /// refuses. A file another entry already indexed is skipped, per the
    /// module doc's second decision, so the first analysis that reaches a file
    /// keeps it. An entry that files no record keeps no reads.
    fn merge(&mut self, analysis: Analysis) {
        self.work += analysis.work;
        let mut owns_a_file = false;
        for (key, indexed) in analysis.records {
            if self.files.contains_key(&key) {
                continue;
            }
            self.post(&key, &indexed);
            self.files.insert(key, indexed);
            owns_a_file = true;
        }
        // An entry that indexed no file has nothing an edit could make stale,
        // so its reads are not kept.
        if owns_a_file {
            self.reads.insert(analysis.entry, analysis.reads);
        }
    }

    /// Adds what `indexed` declares and uses to the three maps, under `key`.
    fn post(&mut self, key: &Path, indexed: &Indexed) {
        self.generation += 1;
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
        for (at, sent) in indexed.requests.iter().enumerate() {
            add(&mut self.by_request, &sent.symbol, (Arc::clone(&key), at));
        }
        for (at, named) in indexed.unfollowed.iter().enumerate() {
            add(&mut self.by_unfollowed, named, (Arc::clone(&key), at));
        }
        for (at, name) in indexed.uses.iter().flat_map(|uses| &uses.used).enumerate() {
            add(&mut self.by_used, name, (Arc::clone(&key), at));
            if let Some(owner) = owner_of(name) {
                add(&mut self.by_used_on, owner, (Arc::clone(&key), at));
            }
        }
        for (at, name) in indexed
            .uses
            .iter()
            .flat_map(|uses| &uses.missed)
            .enumerate()
        {
            add(&mut self.by_missed, name, (Arc::clone(&key), at));
            if let Some(owner) = owner_of(name) {
                add(&mut self.by_missed_on, owner, (Arc::clone(&key), at));
            }
        }
    }

    /// Removes from the three maps what [`SymbolIndex::post`] added for
    /// `indexed` under `key`.
    fn unpost(&mut self, key: &Path, indexed: &Indexed) {
        self.generation += 1;
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
        for (at, sent) in indexed.requests.iter().enumerate() {
            remove(&mut self.by_request, &sent.symbol, &(Arc::clone(&key), at));
        }
        for (at, named) in indexed.unfollowed.iter().enumerate() {
            remove(&mut self.by_unfollowed, named, &(Arc::clone(&key), at));
        }
        for (at, name) in indexed.uses.iter().flat_map(|uses| &uses.used).enumerate() {
            let posting = (Arc::clone(&key), at);
            remove(&mut self.by_used, name, &posting);
            if let Some(owner) = owner_of(name) {
                remove(&mut self.by_used_on, owner, &posting);
            }
        }
        for (at, name) in indexed
            .uses
            .iter()
            .flat_map(|uses| &uses.missed)
            .enumerate()
        {
            let posting = (Arc::clone(&key), at);
            remove(&mut self.by_missed, name, &posting);
            if let Some(owner) = owner_of(name) {
                remove(&mut self.by_missed_on, owner, &posting);
            }
        }
    }
}

/// What [`SymbolIndex::send`] sent to the workers for one entry.
enum Sent {
    /// A plan job: the entry's graph was unknown.
    Plan,
    /// The entry's parts.
    Parts,
    /// No part: every file of its graph is indexed or claimed.
    Nothing,
    /// No worker runs and none could be started.
    Failed,
}

/// What one analysis of an entry point found, before it is filed into the
/// index. It owns everything it holds, so the analysis that built it and the
/// [`SymbolIndex::merge`] that files it are apart.
#[derive(Debug)]
pub(crate) struct Analysis {
    /// The entry point, under `canonical_key`.
    entry: PathBuf,
    /// Each file the analysis reached that the index did not hold when it
    /// started, under `canonical_key`, in the order the graph loaded them.
    records: Vec<(PathBuf, Indexed)>,
    /// Every file the analysis read, under `canonical_key`, entry first.
    reads: Vec<PathBuf>,
    /// Each file the analysis reached and left out because the index held it.
    /// A worker's result is analysed again if one of them was dropped before
    /// its turn.
    skipped: Vec<PathBuf>,
    /// The front-end work the analysis did, and all the file-system work of
    /// building its records.
    work: Work,
}

/// Analyses `entry` and builds the record of each file it reached that
/// `indexed` does not name. It reads the store only through `reads`, and
/// keeps its parses and checked outputs in `session`.
///
/// It checks the bodies of the files it will build a record of, and of the
/// entry where `checks_entry` says so, and no others: a file `indexed` names
/// is checked by the analysis that indexes it ([`analyse_through`]). So a
/// cold settle checks each file once however many entries read it. A worker's
/// part passes `false` when the entry is another part's. The files a loaded
/// extension carries are checked where `checks_carried` says so, which a
/// worker's part says only when it is the entry's first.
///
/// Each path the graph loaded is a `canonical_key` already, because the walk
/// resolves every file it loads, so only the entry is resolved, by the caller.
#[allow(clippy::too_many_arguments)]
pub(crate) fn analyse_entry(
    reads: &impl Reads,
    session: &mut Session,
    trees: &mut crate::tree_cache::TreeCache,
    entry: Entry<'_>,
    version: i32,
    indexed: impl Fn(&Path) -> bool,
    checks_entry: bool,
    checks_carried: bool,
) -> Option<(Analysed, Analysis)> {
    let before = nvs_diagnostics::work::so_far();
    let started = SystemTime::now();
    let absent = |key: &Path| {
        if key == entry.key {
            !checks_entry
        } else {
            indexed(key)
        }
    };
    let analysed = analyse_through(
        reads,
        session,
        trees,
        entry,
        version,
        None,
        &absent,
        checks_carried,
    )?;
    let open = reads.open_keys();
    // The entry was loaded by the path the caller gave, and every other file
    // by the key the walk resolved.
    let key_of = |loaded: &Loaded| -> Option<PathBuf> {
        if loaded.id == analysed.entry {
            return Some(entry.key.to_path_buf());
        }
        analysed.map.file(loaded.id).path().map(Path::to_path_buf)
    };
    let mut records = Vec::new();
    let mut skipped = Vec::new();
    for loaded in &analysed.loaded {
        let Some(key) = key_of(loaded) else {
            continue;
        };
        if indexed(&key) {
            skipped.push(key);
            continue;
        }
        let text = analysed.map.file(loaded.id).text();
        let stamp = if open.contains(&key) {
            None
        } else {
            std::fs::metadata(&key)
                .ok()
                .and_then(|metadata| Stamp::of(&metadata, text.len(), started))
        };
        // The parse cache took the surface rows when it parsed this text, and
        // the analysis parsed through it, so it holds that parse now.
        let src = analysed.map.file(loaded.id);
        let rows = session.parses.hashes(src).map_or_else(
            || crate::surface::rows_of_source(text),
            |(_, rows)| rows.clone(),
        );
        // The file's checked output, current after the walk, has what its
        // check looked up. A file the walk left out and that has no output
        // has no recording.
        let uses = session
            .checks
            .get(loaded.id)
            .and_then(|output| LookedUp::of(output.uses()));
        let indexed = record(&analysed, loaded, &key, stamp, (rows, uses), entry.key);
        records.push((key, indexed));
    }
    let reads = analysed
        .loaded
        .iter()
        .filter_map(key_of)
        .chain(analysed.lent.iter().cloned())
        .collect();
    let mut work = analysed.work;
    work.files = nvs_diagnostics::work::so_far() - before;
    let analysis = Analysis {
        entry: entry.key.to_path_buf(),
        records,
        reads,
        skipped,
        work,
    };
    Some((analysed, analysis))
}

/// The files of `entry`'s graph, under `canonical_key`, in the order the walk
/// loaded them, and the work of resolving it. It parses the graph and reads
/// its declarations, and checks no body. `None` when the entry could not be
/// loaded.
pub(crate) fn plan_entry(
    reads: &impl Reads,
    session: &mut Session,
    trees: &mut crate::tree_cache::TreeCache,
    entry: Entry<'_>,
    version: i32,
) -> Option<(Vec<PathBuf>, Work)> {
    let before = nvs_diagnostics::work::so_far();
    let analysed = analyse_through(
        reads,
        session,
        trees,
        entry,
        version,
        None,
        &|_| true,
        false,
    )?;
    let graph = analysed
        .loaded
        .iter()
        .filter_map(|loaded| {
            if loaded.id == analysed.entry {
                return Some(entry.key.to_path_buf());
            }
            analysed.map.file(loaded.id).path().map(Path::to_path_buf)
        })
        .collect();
    let mut work = analysed.work;
    work.files = nvs_diagnostics::work::so_far() - before;
    Some((graph, work))
}

/// The record of one file `analysed` loaded, under `key`, with its surface
/// rows and what its check looked up.
fn record(
    analysed: &Analysed,
    loaded: &Loaded,
    key: &Path,
    stamp: Option<Stamp>,
    (rows, looked_up): (crate::surface::Rows, Option<LookedUp>),
    entry: &Path,
) -> Indexed {
    let text = analysed.map.file(loaded.id).text();
    let scope = FileScope::of(analysed, loaded.id);
    let uses = occurrences(analysed, &scope, loaded, key);
    let in_file = |diag: &nvs_diagnostics::Diagnostic| {
        diag.primary_span()
            .is_some_and(|span| span.file == loaded.id)
    };
    Indexed {
        decls: declarations(analysed, &scope, loaded, key),
        occurrences: uses.occurrences,
        imports: imports(analysed, loaded, key),
        requests: uses.requests,
        unfollowed: uses.unfollowed,
        queries: uses.queries,
        sole: sole_declaration(&loaded.stmts, analysed.map.file(loaded.id))
            .map(|(_, kind)| DeclKind::of_symbol(kind)),
        errors: analysed.erring.contains(&loaded.id),
        broken: analysed
            .diags
            .iter()
            .any(|diag| crate::diagnostics::breaks_the_tree(diag) && in_file(diag)),
        rows,
        uses: looked_up,
        content: content_hash(text.as_bytes()),
        stamp,
        entry: entry.to_path_buf(),
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
    if let Some((declared, _)) = declared_at(analysed, offset) {
        return Some(declared);
    }
    if let Some(key) = crate::attribute_decls::field_at(analysed, offset) {
        return Some(key.symbol());
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
///
/// The span is where the declaration writes the name, which is what a hover
/// on it underlines.
pub(crate) fn declared_at(analysed: &Analysed, offset: BytePos) -> Option<(String, Span)> {
    if let Some(field) = crate::attribute_decls::declared_fields(analysed, analysed.entry)
        .into_iter()
        .find(|field| covers(field.span, offset))
    {
        return Some((field.symbol(), field.span));
    }
    let scope = FileScope::of(analysed, analysed.entry);
    analysed.module.symbols.iter().find_map(|symbol| {
        if symbol.decl_span.file != analysed.entry {
            return None;
        }
        if covers(symbol.decl_span, offset) {
            return Some((symbol.qname.to_string(), symbol.decl_span));
        }
        let body = declared_body(analysed, &scope, &symbol.qname)?;
        let class = symbol.qname.to_string();
        member_names(&body)
            .into_iter()
            .find(|(name, ..)| covers(*name, offset))
            .and_then(|(name, ..)| Some((member_symbol(&class, body.file, name)?, name)))
    })
}

/// The version recorded for a file no client has open.
///
/// A version is the client's stamp on a buffer and a file read from disk has
/// none. Nothing compares this one: `Documents::is_current` is asked about
/// documents, and the index publishes nothing.
const NO_BUFFER: i32 = 0;

/// What a loaded entry's file says about the entry.
enum Freshness {
    /// The file has the text the entry was indexed from.
    Same,
    /// The same, found by hashing the file, and this is its stamp now.
    Restamped(Option<Stamp>),
    /// The text changed, and these are its surface rows, where it is UTF-8.
    Changed(Option<crate::surface::Rows>),
    /// The file is gone or cannot be read.
    Gone,
}

/// Adds to `inputs` the configuration inputs of each directory of `entries`
/// that it does not cover yet, with what each is now. A directory is covered
/// once its `nvs.toml` is among the inputs, present or absent.
fn add_inputs<'a>(inputs: &mut BTreeMap<PathBuf, Input>, entries: impl Iterator<Item = &'a Path>) {
    let dirs: BTreeSet<&Path> = entries
        .filter_map(Path::parent)
        .filter(|dir| !inputs.contains_key(&dir.join(nvs_config::resolve::LOCAL_FILE)))
        .collect();
    if dirs.is_empty() {
        return;
    }
    let data = nvs_config::data::config_file();
    for input in crate::tree_cache::inputs(dirs.into_iter(), data.as_deref()) {
        let now = Input::of(&input);
        inputs.entry(input).or_insert(now);
    }
}

/// [`freshness`] of every file in `files`, in order, on one thread per core.
///
/// Each thread takes one run of the files in order, so the result is the
/// serial one. Reading and hashing every file of a large tree is most of a
/// load by content, and the threads end before this returns.
fn freshen(files: &[(PathBuf, Indexed)], started: SystemTime, check: Check) -> Vec<Freshness> {
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let run = files.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let parts: Vec<_> = files
            .chunks(run)
            .map(|part| {
                scope.spawn(move || {
                    nvs_runtime::budget::run_then_release_pooled_blocks(|| {
                        part.iter()
                            .map(|(key, indexed)| freshness(key, indexed, started, check))
                            .collect::<Vec<Freshness>>()
                    })
                })
            })
            .collect();
        parts
            .into_iter()
            .flat_map(|part| match part.join() {
                Ok(fresh) => fresh,
                Err(panic) => std::panic::resume_unwind(panic),
            })
            .collect()
    })
}

/// Whether the file at `key` still has the text `indexed` was built from.
/// `started` is when the load began, which [`Stamp::of`] needs. Under
/// [`Check::Content`] the file is always read and hashed.
fn freshness(key: &Path, indexed: &Indexed, started: SystemTime, check: Check) -> Freshness {
    let Ok(metadata) = std::fs::metadata(key) else {
        return Freshness::Gone;
    };
    if !metadata.is_file() {
        return Freshness::Gone;
    }
    if check == Check::Stamp && indexed.stamp.is_some_and(|stamp| stamp.matches(&metadata)) {
        return Freshness::Same;
    }
    let Ok(bytes) = std::fs::read(key) else {
        return Freshness::Gone;
    };
    if content_hash(&bytes) == indexed.content {
        let stamp = Stamp::of(&metadata, bytes.len(), started);
        return if stamp == indexed.stamp {
            Freshness::Same
        } else {
            Freshness::Restamped(stamp)
        };
    }
    Freshness::Changed(
        std::str::from_utf8(&bytes)
            .ok()
            .map(crate::surface::rows_of_source),
    )
}

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
        // The walk starts from the canonical root, joins each name as its
        // directory lists it, and follows no link, so every path it yields is
        // already its own canonical key. Canonicalizing each one again opens
        // the file on Windows, and on a tree of many files that was most of a
        // start's cost.
        for path in nvs_hir::lenders::sources(&canonical_key(root)) {
            selected.entry(path).or_insert(NO_BUFFER);
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
fn declarations<'a>(
    analysed: &'a Analysed,
    scope: &FileScope<'a>,
    loaded: &Loaded,
    path: &Path,
) -> Vec<Declaration> {
    let mut found = Vec::new();

    for symbol in analysed.module.symbols.iter() {
        if symbol.decl_span.file != loaded.id {
            continue;
        }
        let class = symbol.qname.to_string();
        let declared = scope.declared_type(&symbol.qname);
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
            routes: Vec::new(),
            test: false,
            extent: None,
        });
        if let Some(body) = declared_body(analysed, scope, &symbol.qname) {
            members(analysed, &body, &class, path, &mut found);
        }
    }
    // Each field of an `attribute` declaration is a name of its own, under
    // `Name::field`, so a key a payload writes is a use of it.
    for field in crate::attribute_decls::declared_fields(analysed, loaded.id) {
        found.push(Declaration {
            symbol: field.symbol(),
            kind: DeclKind::Field,
            site: site(path, field.span),
            visibility: Visibility::Public,
            supertypes: Vec::new(),
            construction: None,
            parameters: Vec::new(),
            routes: Vec::new(),
            test: false,
            extent: None,
        });
    }

    found.sort_by_key(|declared| (declared.site.start, declared.site.end));
    found
}

/// The members and the cases a type's body writes, and the file it is in.
struct Body<'a> {
    members: &'a [ClassMember],
    cases: &'a [EnumCase],
    file: &'a SourceFile,
}

/// The body the type `qname` is declared with, or nothing for a name with no
/// declaration in the analysis.
///
/// An enum a class or interface body owns is no statement, so
/// [`declared_type`] does not find it and [`owned_enum_decl`] finds it through
/// its owner (`rule:enums/class-scoped-enum`). Both spellings of its name are
/// one symbol, so its cases are indexed once and found from either.
fn declared_body<'a>(
    analysed: &'a Analysed,
    scope: &FileScope<'a>,
    qname: &QName,
) -> Option<Body<'a>> {
    if let Some((stmt, file)) = scope.declared_type(qname) {
        let (cases, members) = body_of(stmt)?;
        return Some(Body {
            members,
            cases,
            file,
        });
    }
    let (_, decl) = scope.owned_enum_decl(qname)?;
    let symbol = analysed.module.symbols.get(qname)?;
    Some(Body {
        members: &decl.members,
        cases: &decl.cases,
        file: analysed.map.file(symbol.decl_span.file),
    })
}

/// The members `body` declares, appended to `found`.
///
/// A method's routes and its `#[Test]` mark are read off the analysis's own
/// tables, which name a method `Class::method` exactly as [`member_symbol`]
/// spells it.
fn members(
    analysed: &Analysed,
    body: &Body<'_>,
    class: &str,
    path: &Path,
    found: &mut Vec<Declaration>,
) {
    let tests = analysed.exprs.tests(class).unwrap_or_default();
    for (name, kind, visibility) in member_names(body) {
        let Some(symbol) = member_symbol(class, body.file, name) else {
            continue;
        };
        let method = kind == DeclKind::Method;
        let routes = if method {
            analysed
                .exprs
                .routes()
                .rows()
                .iter()
                .filter(|row| row.handler == symbol)
                .map(|row| format!("{} {}", row.verb.to_ascii_uppercase(), row.path))
                .collect()
        } else {
            Vec::new()
        };
        let test = method && tests.iter().any(|case| case.span == name);
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
            parameters: if method {
                parameters_of(body, name)
            } else {
                Vec::new()
            },
            routes,
            test,
            extent: if method { extent_of(body, name) } else { None },
        });
    }
}

/// The bytes of the whole method `body` declares at `name`.
fn extent_of(body: &Body<'_>, name: Span) -> Option<(BytePos, BytePos)> {
    body.members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(method) if method.name == name => {
            Some((member.span.start, member.span.end))
        }
        _ => None,
    })
}

/// The parameters of the method `body` declares at `name`.
fn parameters_of(body: &Body<'_>, name: Span) -> Vec<Parameter> {
    let file = body.file;
    body.members
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

/// Every member name `body` declares, with what it declares and where it is
/// readable from, in source order.
///
/// One list for the two questions asked about a member name: [`members`] turns
/// each into an index entry, and [`declared_at`] asks which of them the cursor
/// is on. `ClassMemberKind` is `#[non_exhaustive]` and `Error` is recovery, so
/// a member shape this crate has not heard of declares no name either of them
/// could answer about and contributes none.
fn member_names(body: &Body<'_>) -> Vec<(Span, DeclKind, Visibility)> {
    let mut names: Vec<(Span, DeclKind, Visibility)> = body
        .members
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
            // An alias takes no modifier and is reachable wherever its owner's
            // name is (`rule:types/class-scoped-alias`).
            ClassMemberKind::TypeAlias(alias) => {
                Some((alias.name.span, DeclKind::TypeAlias, Visibility::Public))
            }
            // So is an attribute a body declares, and its symbol is the
            // `Owner::Index` key `nvs_hir::AliasTable::attribute` has.
            ClassMemberKind::Attribute(decl) => {
                Some((decl.name.span, DeclKind::Attribute, Visibility::Public))
            }
            _ => None,
        })
        .collect();
    // An enum case takes no modifier list and is reachable wherever the enum's
    // own name is.
    names.extend(
        body.cases
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
pub(crate) fn visibility_of(modifiers: &[Modifier]) -> Visibility {
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
/// `rule:ide/the-request-set-is-closed`'s resolution applied to every
/// occurrence rather than to the one under a cursor. Nothing is
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
/// The written types are the third walk: every class name in a type
/// annotation, and every class written in front of `::`, resolved with the
/// namespace and imports of the file that writes it ([`FileScope::written_name`],
/// the reading a cursor on it gets). Only a type the program declares is
/// recorded, so a `Core` class in an annotation adds nothing. Rename reads
/// these sites to rename a type.
///
/// A use is recorded at the **name**, which [`named`] reads off the node rather
/// than off the expression around it: a highlight box is drawn on exactly the
/// span answered here, and `$u->greet()` highlighted from `$u` is a box around
/// the wrong word.
///
/// **Each use records the method whose body it is written in**, which makes a
/// use that resolved to a method an edge of the call graph
/// (`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method`). The
/// method is the innermost method declaration around the use, read off the
/// same declarations [`declarations`] reads, so the caller is spelled the way
/// a [`Declaration`] is. The walk also records the two things the call graph
/// reads beside the uses: a `Core\Test::request` the route table matched, and
/// a call the graph does not follow ([`Uses`]).
fn occurrences<'a>(
    analysed: &'a Analysed,
    scope: &FileScope<'a>,
    loaded: &Loaded,
    path: &Path,
) -> Uses {
    let mut found = Vec::new();
    let mut requests = Vec::new();
    let mut unfollowed = Vec::new();
    let mut queries = Vec::new();
    let methods = method_bodies(analysed, scope, loaded);
    let caller = |span: Span| caller_at(&methods, span);
    let file = analysed.map.file(loaded.id);
    let mut table = None;

    for root in walk::of_stmts(&loaded.stmts) {
        for node in std::iter::once(&root).chain(root.descendants()) {
            found.extend(
                class_qualifier(analysed, scope, node, path).map(|used| Occurrence {
                    caller: caller(node.span),
                    ..used
                }),
            );
            let info = analysed.exprs.lookup(node.span);
            // Every call a recorded request can name by line: a statement, an
            // HTTP call, a cache or session read or write, or a file open.
            if let Some(ExprInfo::Call(call)) = info
                && nvs_types::core_records_site(&call.class.to_string(), &call.method)
            {
                queries.push((node.span.start, node.span.end));
            }
            let resolved = info.and_then(|info| Some((info, target_of(info)?)));
            let Some((info, target)) = resolved else {
                // A call by name that resolved to no one method: on a `mixed`
                // receiver, or one the checker reported.
                if matches!(node.kind, "MethodCall" | "StaticCall")
                    && let Some(name) = node.name
                {
                    unfollowed.push(format!("::{}", text_of(file, name)));
                }
                continue;
            };
            let symbol = symbol_of(&target);
            let callable = matches!(info, ExprInfo::CallableRef(_));
            if callable {
                unfollowed.push(symbol.clone());
            }
            if let ExprInfo::Call(call) = info
                && let Some(sent) = request_sent(analysed, file, call, node, &mut table)
            {
                requests.push(Occurrence {
                    symbol: sent.0,
                    site: site(path, sent.1),
                    caller: caller(node.span),
                });
            }
            found.push(Occurrence {
                symbol,
                site: site(path, named(node)),
                caller: if callable { None } else { caller(node.span) },
            });
            found.extend(case_qualifier(info, node, path).map(|used| Occurrence {
                caller: caller(node.span),
                ..used
            }));
        }
    }

    for symbol in analysed.module.symbols.iter() {
        if symbol.decl_span.file != loaded.id {
            continue;
        }
        let Some(links) = analysed.module.graph.get(&symbol.qname) else {
            continue;
        };
        let Some((stmt, _)) = scope.declared_type(&symbol.qname) else {
            continue;
        };
        let (extends, implements) = supertype_names(stmt);
        clause_uses(&extends, &links.extends, path, &mut found);
        clause_uses(&implements, &links.implements, path, &mut found);
    }

    let mut names = Vec::new();
    let mut qualified = Vec::new();
    for ty in written_types_in(&loaded.stmts, 0, BytePos::MAX) {
        type_names(ty, &mut names, &mut qualified);
    }
    // A bare name inside an owner's body is the owner's alias first, as the
    // cursor reads it ([`FileScope::owned_alias`]).
    for name in names {
        let symbol = scope
            .owned_alias(name)
            .map(|(owner, alias)| format!("{owner}::{alias}"))
            .or_else(|| declared_name(analysed, scope, name));
        if let Some(symbol) = symbol {
            found.push(Occurrence {
                symbol,
                site: site(path, name),
                caller: caller(name),
            });
        }
    }
    // `Owner::Name` in a type is a use of the owner, and of the alias or the
    // enum it declares under that name.
    for (owner, member) in qualified {
        let Some(class) = scope
            .written_name(owner)
            .and_then(|written| analysed.module.symbols.get(&written))
        else {
            continue;
        };
        found.push(Occurrence {
            symbol: class.qname.to_string(),
            site: site(path, owner),
            caller: caller(owner),
        });
        if let Some(symbol) = owned_member(analysed, &class.qname, text_of(file, member)) {
            found.push(Occurrence {
                symbol,
                site: site(path, member),
                caller: caller(member),
            });
        }
    }

    // Every `#[Name]` and every retrieval's `<Name>` is a use of the attribute
    // declaration the checker resolved it to. A namespace-level one written
    // in a type argument is also found by the walk above, and the dedup below
    // keeps one.
    for (span, name) in attribute_uses(analysed, loaded.id) {
        found.push(Occurrence {
            symbol: name.to_string(),
            site: site(path, span),
            caller: caller(span),
        });
    }
    // Every key a payload writes is a use of the field it names.
    for key in crate::attribute_decls::written_keys(analysed, loaded.id, &loaded.stmts) {
        found.push(Occurrence {
            symbol: key.symbol(),
            site: site(path, key.span),
            caller: caller(key.span),
        });
    }

    // Two nodes can cover the same bytes — a statement that is one expression —
    // and both read the same entry out of the table. The sort is also what puts
    // the two walks into the one order the index answers in.
    found.sort();
    found.dedup();
    requests.sort();
    requests.dedup();
    queries.sort();
    queries.dedup();
    Uses {
        occurrences: found,
        requests,
        unfollowed,
        queries,
    }
}

/// What [`occurrences`] reads off one file.
struct Uses {
    /// Every resolved use, in source order.
    occurrences: Vec<Occurrence>,
    /// Every `Core\Test::request` the route table matched, against the
    /// handler of the route it matched.
    requests: Vec<Occurrence>,
    /// Every call the call graph does not follow, spelled as
    /// [`SymbolIndex::unfollowed`] looks one up.
    unfollowed: Vec<String>,
    /// Every `Core\Db` call that sends a statement, as the call's start and end.
    queries: Vec<(BytePos, BytePos)>,
}

/// Every method with a body that one file declares: the whole declaration's
/// span, and the method's symbol.
fn method_bodies<'a>(
    analysed: &'a Analysed,
    scope: &FileScope<'a>,
    loaded: &Loaded,
) -> Vec<(Span, String)> {
    let mut found = Vec::new();
    for symbol in analysed.module.symbols.iter() {
        if symbol.decl_span.file != loaded.id {
            continue;
        }
        let Some(body) = declared_body(analysed, scope, &symbol.qname) else {
            continue;
        };
        let class = symbol.qname.to_string();
        for member in body.members {
            if let ClassMemberKind::Method(method) = &member.kind
                && method.body.is_some()
                && let Some(declared) = member_symbol(&class, body.file, method.name)
            {
                found.push((member.span, declared));
            }
        }
    }
    found
}

/// The innermost method in `methods` whose declaration covers `span`.
///
/// Innermost, because a method of an anonymous class is written inside the
/// body of another method.
fn caller_at(methods: &[(Span, String)], span: Span) -> Option<String> {
    methods
        .iter()
        .filter(|(body, _)| body.start <= span.start && span.end <= body.end)
        .min_by_key(|(body, _)| body.end - body.start)
        .map(|(_, declared)| declared.clone())
}

/// The handler a `Core\Test::request` call reaches, and the span of its path
/// literal, when the call writes its method as a `Core\Http\Method` case and
/// its path as a string literal, and the route table matches the two.
///
/// The match is `nvs_runtime::routes::Routes::match_request` over the table
/// `nvs_types::crossing::runtime_routes` builds, which is the server's own
/// matcher over the server's own table. `table` builds it once per file, at
/// the first such call. A path built at run time matches nothing here.
fn request_sent(
    analysed: &Analysed,
    file: &SourceFile,
    call: &nvs_types::ResolvedCall,
    node: &walk::Node,
    table: &mut Option<nvs_runtime::routes::Routes>,
) -> Option<(String, Span)> {
    if call.class.to_string() != TEST_CLASS || call.method != "request" {
        return None;
    }
    // The class written in front of `::` is the first child, and the
    // arguments follow it in order.
    let mut args = node.children.iter().skip(1);
    let verb = match analysed.exprs.lookup(args.next()?.span)? {
        ExprInfo::EnumCase { case, .. } => case,
        _ => return None,
    };
    let path = args.next()?;
    if path.kind != "Str" {
        return None;
    }
    let text = nvs_syntax::string_lit::cook_string_literal(file, path.span);
    let routes = table.get_or_insert_with(|| nvs_types::crossing::runtime_routes(&analysed.exprs));
    let matched = routes.match_request(verb, &text)?;
    Some((matched.route().handler().to_owned(), path.span))
}

/// The class whose `request` method sends a request through the routes in a
/// test (`rule:testing/in-process-request`).
const TEST_CLASS: &str = "Core\\Test";

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
            caller: None,
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
        caller: None,
    })
}

/// The class written in front of `::` in a static call, a static property, a
/// class constant or `::class`, as a use of that class — and `None` for every
/// other node, and for a qualifier that names no type the program declares.
///
/// The name is resolved as written, with the namespace and imports of its own
/// file ([`FileScope::written_name`]), so `Cart::LIMIT` records `Cart` even where the
/// constant is declared on a parent. `self`, `static` and `parent` are no
/// name, so they record nothing.
fn class_qualifier(
    analysed: &Analysed,
    scope: &FileScope<'_>,
    node: &walk::Node,
    path: &Path,
) -> Option<Occurrence> {
    if !matches!(
        node.kind,
        "StaticCall" | "StaticPropertyAccess" | "ClassConstAccess" | "ClassNameConst"
    ) {
        return None;
    }
    let written = node.children.first()?;
    if written.kind != "ConstFetch" {
        return None;
    }
    let name = written.name?;
    Some(Occurrence {
        symbol: declared_name(analysed, scope, name)?,
        site: site(path, name),
        caller: None,
    })
}

/// The type the name written at `name` in the file of `scope` resolves to,
/// spelled the module doc's way, when the program declares it.
fn declared_name(analysed: &Analysed, scope: &FileScope<'_>, name: Span) -> Option<String> {
    let resolved = scope.written_name(name)?;
    let symbol = analysed.module.symbols.get(&resolved)?;
    Some(symbol.qname.to_string())
}

/// The alias or the enum `owner` declares under `name`, spelled the module
/// doc's way, and nothing for a constant or a name it does not declare.
fn owned_member(analysed: &Analysed, owner: &QName, name: &str) -> Option<String> {
    if analysed.module.aliases.get_member(owner, name).is_some() {
        return Some(format!("{owner}::{name}"));
    }
    nvs_hir::owned_enum(owner, name, &analysed.module.symbols).map(|owned| owned.to_string())
}

/// The span of every class name written in `ty`, its type arguments included,
/// into `found`, and of the owner and the member of every `Owner::Member`
/// into `qualified`.
fn type_names(ty: &Type, found: &mut Vec<Span>, qualified: &mut Vec<(Span, Span)>) {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => type_names(inner, found, qualified),
        TypeKind::Union(members) | TypeKind::Intersection(members) => {
            for member in members {
                type_names(member, found, qualified);
            }
        }
        TypeKind::Atom(atom) => match atom {
            TypeAtom::Name(name, args) => {
                found.push(name.span);
                for arg in args {
                    type_names(arg, found, qualified);
                }
            }
            TypeAtom::Member(owner, member) | TypeAtom::MemberCase(owner, member, _) => {
                qualified.push((owner.span, *member));
            }
            TypeAtom::Array(Some(arg))
            | TypeAtom::ClassRef(arg)
            | TypeAtom::PropertyKey(arg)
            | TypeAtom::PropertyPath(arg)
            | TypeAtom::StringHoles { key: arg, .. } => type_names(arg, found, qualified),
            TypeAtom::Shape(fields) => {
                for field in fields {
                    type_names(&field.ty, found, qualified);
                }
            }
            TypeAtom::CallableSig { params, ret } => {
                for param in params {
                    type_names(param, found, qualified);
                }
                type_names(ret, found, qualified);
            }
            _ => {}
        },
        _ => {}
    }
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
/// the right of `is` is no child it names, a shape type being a child with no
/// name, so the test answers the name it carries over the span it covers.
pub(crate) fn named(node: &walk::Node) -> Span {
    let written = match node.kind {
        "Call" => node.children.first(),
        _ => None,
    }
    .unwrap_or(node);
    written.name.unwrap_or(written.span)
}

/// Every `use` import one file of an analysis writes.
fn imports(analysed: &Analysed, loaded: &Loaded, path: &Path) -> Vec<Import> {
    reads(
        analysed.map.file(loaded.id),
        loaded,
        &analysed.module.imports,
    )
    .into_iter()
    .map(|found| Import {
        symbol: found.import.target.to_string(),
        site: site(path, found.import.span),
    })
    .collect()
}

/// The name a person types to find `symbol`: what follows its `::`, without a
/// property's `$`, or else the last segment of its namespace path.
pub(crate) fn own_name(symbol: &str) -> &str {
    match symbol.rsplit_once("::") {
        Some((_, member)) => member.trim_start_matches('$'),
        None => symbol.rsplit('\\').next().unwrap_or(symbol),
    }
}

/// One `use` import of a file, as [`reads`] found it.
#[derive(Debug)]
pub(crate) struct Read<'a> {
    pub(crate) import: &'a nvs_hir::Import,
    /// The whole `use` statement that writes it.
    pub(crate) statement: Span,
    /// Whether a short name written in the same namespace scope spells the
    /// name this import binds.
    ///
    /// A name reads an import when it is an identifier spelled as the import's
    /// last segment, in the same namespace scope, outside every `use`
    /// statement. An identifier beside a `\` is part of a qualified name, and
    /// `rule:statements/a-qualified-name-is-absolute` makes that one consult no
    /// import at all.
    ///
    /// Read off the file's tokens rather than off [`Occurrence`]s, because an
    /// occurrence is recorded for an expression or a clause and a type
    /// annotation is neither: an import only a parameter type names has no
    /// occurrence and is still read. The token reading errs one way only. A
    /// method or a property spelled like the import counts as a read, so an
    /// unused import can go unreported, and an import a name needs is never
    /// reported.
    pub(crate) read: bool,
}

/// Every `use` import `file` writes, of the `imports` name resolution saw,
/// each with whether a name in its scope reads it.
///
/// The one reading of an import's use: [`imports`] keeps it in the index, and
/// the unused-`use` warning of [`crate::diagnostics::unused_uses`] reports it,
/// so the editor, organize imports and `nvs check` cannot disagree about a
/// line.
pub(crate) fn reads<'a>(
    file: &SourceFile,
    loaded: &Loaded,
    imports: &'a [nvs_hir::Import],
) -> Vec<Read<'a>> {
    // The file parsed already, so lexing it again reports nothing new.
    let tokens = tokenize(file, &mut Diagnostics::new());
    let end = BytePos::try_from(file.text().len()).unwrap_or(BytePos::MAX);
    let scopes = namespace_scopes(&loaded.stmts, end);
    let mut statements = Vec::new();
    use_statements(&loaded.stmts, &mut statements);

    imports
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
            let statement = statements
                .iter()
                .copied()
                .find(|used| covers(*used, import.span.start))
                .unwrap_or(import.span);
            Read {
                import,
                statement,
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
