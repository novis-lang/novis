//! The open documents, and the buffers that shadow the files under them.
//!
//! `rule:ide/an-open-document-is-its-own-entry-point` is this module in one
//! sentence: the client pushes a whole document per keystroke
//! (`TextDocumentSyncKind::FULL`, which [`crate::server_capabilities`]
//! declares), the store holds it under its URI with the version the client
//! stamped it with, and an analysis resolves one open document's
//! `require`/`autoload` graph **with the open buffers overlaid on disk**. A
//! class edited in one tab and named in another therefore resolves to the
//! unsaved text, which is what the person typing is looking at.
//!
//! The overlay itself is `nvs_diagnostics::SourceMap::overlay`'s rather than
//! this crate's: the graph walk loads by path and is the only thing that knows
//! which paths a program reads, so the substitution has to happen where the
//! read is. This module decides *what* is overlaid, and [`analyse`] is the one
//! place a graph is walked with the buffers in front of it.
//!
//! **A file a program autoloads borrows that program's `autoload` map**
//! (`rule:ide/an-autoloaded-file-borrows-its-programs-map`). It may not declare
//! one, so as its own entry point it would resolve none of the names its
//! program resolves. [`Documents::survey`] finds the programs that declare one,
//! [`Documents::resurvey`] keeps that current, and [`analyse_file`] is the one
//! place a map is lent.
//!
//! **Diagnostics are published only for open documents.** Publishing for a file
//! nobody opened is workspace-wide analysis, which is M10's, and
//! [`Documents::to_republish`] is where that boundary sits. It answers the
//! other half of the rule too — editing one document re-analyses every open
//! document whose graph contains it — out of the reverse index
//! [`Documents::record_graph`] builds from the analysis that has just run,
//! rather than by re-walking anything.
//!
//! **An answer computed for a version the client has already replaced is
//! dropped rather than sent**, which is what
//! `rule:ide/the-server-is-synchronous` asks of this stage:
//! [`analyse_current`] refuses to start one and [`Documents::is_current`] is
//! asked again before an answer goes out. The analysis thread that rule also
//! names arrives with the first answer that has somewhere to go — spawning it
//! before there is a diagnostic to publish buys a channel and an ownership
//! question and nothing else, and the policy above is about versions rather
//! than about threads.
//!
//! A URI is a document's identity on the wire and a path is its identity to the
//! compiler, so [`path_of`] and [`uri_of`] convert between the two here and
//! nowhere else. Only a `file:` URI has a path: a document opened under any
//! other scheme is held and analysed as nothing, because its `require` targets
//! have no directory to resolve against.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use lsp_types::Uri;
use nvs_diagnostics::{BytePos, Diagnostics, SourceId, SourceMap, Span, canonical_key};
use nvs_hir::{AutoloadMap, Loaded, Module, resolve_program, resolve_program_borrowing};
use nvs_syntax::{SyntaxIndex, Trivia, check_declarations, parse};
use nvs_types::{ExprTypeTable, LocalBinding, TypeId, TypeInterner};

/// One open buffer: what the editor holds, which is not what is on disk.
#[derive(Debug, Clone)]
pub struct Document {
    uri: Uri,
    path: Option<PathBuf>,
    version: i32,
    text: String,
}

impl Document {
    /// Its identity on the wire.
    #[must_use]
    pub const fn uri(&self) -> &Uri {
        &self.uri
    }

    /// The file it is a buffer for, when its URI names one.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The version the client stamped this text with.
    #[must_use]
    pub const fn version(&self) -> i32 {
        self.version
    }

    /// The text, byte for byte as the client sent it.
    ///
    /// Nothing is normalised on the way in: a CRLF document keeps its CRLFs and
    /// a leading BOM stays where it is, because spans are byte offsets and
    /// rewriting a line ending server-side would shift every column in the file
    /// (`rule:ide/positions-have-one-home`).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Every document the client has opened and not yet closed.
#[derive(Debug, Default)]
pub struct Documents {
    buffers: HashMap<Uri, Document>,
    /// For each open document, every file its last analysis read, under
    /// `canonical_key`. The reverse index the rule's last paragraph asks for:
    /// the graph is already in hand from the analysis that produced that
    /// document's diagnostics, so keeping it is cheaper than deriving it.
    graphs: HashMap<Uri, Vec<PathBuf>>,
    /// Every program [`survey`](Self::survey) found that declares `autoload`,
    /// in entry-path order, which is what makes [`lender_for`](Self::lender_for)
    /// a deterministic choice. One resolved map per such program, held for the
    /// session.
    lenders: Vec<Lender>,
}

/// One program that declares `autoload`, as the files it autoloads borrow it.
///
/// `rule:ide/an-open-document-is-its-own-entry-point` is why this exists: a
/// file a program autoloads is analysed as an entry point, may not write an
/// `autoload` itself, and so has no map unless it is lent the one that reaches
/// it.
#[derive(Debug)]
struct Lender {
    /// The file the walk started from, under `canonical_key`.
    entry: PathBuf,
    /// Every file of its `require` chain that wrote a declaration, under
    /// `canonical_key` — the files whose edit makes this lender stale.
    declaring: Vec<PathBuf>,
    /// The map those declarations built, which says what the program claims
    /// and carries the sites it lends.
    map: AutoloadMap,
}

/// `path` as a [`Lender`], or `None` for a file whose program declares no
/// `autoload`.
///
/// The text is searched for the keyword before anything is parsed, so a
/// workspace pays one read per file and one walk per bootstrap file. That walk
/// is `nvs check`'s name resolution without the type phase, and it reads every
/// file the program reaches. Its diagnostics are dropped: they are published
/// when the file itself is analysed.
fn lender(documents: &Documents, path: &Path) -> Option<Lender> {
    let mut map = SourceMap::new();
    documents.overlay(&mut map);
    let entry = map.load(path).ok()?;
    if !map.file(entry).text().contains("autoload") {
        return None;
    }

    let mut diags = Diagnostics::new();
    let stmts = parse(map.file(entry), &mut diags).stmts;
    let (_, _, autoload) = resolve_program(
        entry,
        stmts,
        &mut map,
        nvs_hir::CoreRoster::Trusted,
        &mut diags,
    );
    if autoload.sites().is_empty() {
        return None;
    }

    let mut declaring: Vec<PathBuf> = autoload
        .sites()
        .iter()
        .filter_map(|site| map.file(site.span.file).path().map(canonical_key))
        .collect();
    declaring.sort();
    declaring.dedup();
    Some(Lender {
        entry: canonical_key(path),
        declaring,
        map: autoload,
    })
}

impl Documents {
    /// A store with nothing open.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `textDocument/didOpen`.
    ///
    /// Opening a URI that is already open replaces the buffer, which is what a
    /// client that reopened a document without closing it means.
    pub fn open(&mut self, uri: Uri, version: i32, text: String) {
        let path = path_of(&uri);
        self.buffers.insert(
            uri.clone(),
            Document {
                uri,
                path,
                version,
                text,
            },
        );
    }

    /// `textDocument/didChange` under `FULL` sync, where `text` is the whole
    /// document rather than a range.
    ///
    /// A change for a document nobody opened is dropped rather than opening
    /// one: the protocol sends `didOpen` first, and a store that invented a
    /// document here would hold a buffer no `didClose` ever removes. Versions
    /// are not compared, because one connection delivers its notifications in
    /// order and the last to arrive is the newest by construction.
    ///
    /// Returns whether a buffer was there to change.
    pub fn change(&mut self, uri: &Uri, version: i32, text: String) -> bool {
        let Some(document) = self.buffers.get_mut(uri) else {
            return false;
        };
        document.version = version;
        document.text = text;
        true
    }

    /// `textDocument/didClose`, which drops the buffer and its recorded graph.
    ///
    /// The graph goes with it: nothing is published for a closed document, so
    /// the reverse index has nothing left to answer about it, and an entry per
    /// file ever opened is a store that grows with the session rather than with
    /// what is open.
    ///
    /// Returns whether a buffer was there to close.
    pub fn close(&mut self, uri: &Uri) -> bool {
        self.graphs.remove(uri);
        self.buffers.remove(uri).is_some()
    }

    /// The buffer for `uri`, if it is open.
    #[must_use]
    pub fn get(&self, uri: &Uri) -> Option<&Document> {
        self.buffers.get(uri)
    }

    /// Every open document, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = &Document> {
        self.buffers.values()
    }

    /// How many documents are open.
    #[must_use]
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Whether nothing is open.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Registers every open buffer on `map`, so a graph resolved through it
    /// reads the unsaved text rather than the file under it.
    ///
    /// Every buffer, not the entry document's alone: the file being edited in
    /// another tab is exactly the one whose staleness would be visible.
    pub fn overlay(&self, map: &mut SourceMap) {
        for document in self.buffers.values() {
            if let Some(path) = document.path() {
                map.overlay(path, document.text.clone());
            }
        }
    }

    /// Finds every program under the tree `scope` selects that declares
    /// `autoload`, replacing whatever an earlier survey found.
    ///
    /// Run once, before the first analysis, so an autoloaded file already
    /// resolves through its program's map the first time it is read —
    /// [`crate::SymbolIndex::build`] included, which is why this comes before
    /// it. [`resurvey`](Self::resurvey) keeps the answer current after that.
    pub fn survey(&mut self, scope: crate::CheckScope, root: Option<&Path>) {
        let lenders = crate::index::tree(self, scope, root)
            .into_iter()
            .filter_map(|(path, _)| lender(self, &path))
            .collect();
        self.lenders = lenders;
    }

    /// [`survey`](Self::survey), for the one file that changed: `changed`
    /// itself and every lender whose declarations it wrote are walked again,
    /// and no other file is read.
    ///
    /// An edit to a file that declares nothing costs one search of its text.
    pub fn resurvey(&mut self, changed: &Path) {
        let key = canonical_key(changed);
        let mut entries: Vec<PathBuf> = self
            .lenders
            .iter()
            .filter(|lender| lender.entry == key || lender.declaring.contains(&key))
            .map(|lender| lender.entry.clone())
            .collect();
        entries.push(key);
        entries.sort();
        entries.dedup();

        self.lenders
            .retain(|lender| !entries.contains(&lender.entry));
        for entry in entries {
            if let Some(found) = lender(self, &entry) {
                self.lenders.push(found);
            }
        }
        self.lenders.sort_by(|a, b| a.entry.cmp(&b.entry));
    }

    /// The program `path` borrows its `autoload` map from: the first, in
    /// entry-path order, that autoloads it.
    ///
    /// The first and not a union, because two programs sharing a source tree
    /// may give one prefix different roots, and a union of their maps is a map
    /// neither of them runs with. A program never lends to its own entry, which
    /// already has every declaration it would be lent.
    fn lender_for(&self, path: &Path) -> Option<&Lender> {
        let key = canonical_key(path);
        self.lenders
            .iter()
            .find(|lender| lender.entry != key && lender.map.claims(path))
    }

    /// Records every file `uri`'s last analysis read.
    ///
    /// A document that is not open records nothing, so the index never holds a
    /// graph for something [`close`](Self::close) will not clear.
    pub fn record_graph(&mut self, uri: &Uri, files: impl IntoIterator<Item = PathBuf>) {
        if !self.buffers.contains_key(uri) {
            return;
        }
        let files = files.into_iter().map(|file| canonical_key(&file)).collect();
        self.graphs.insert(uri.clone(), files);
    }

    /// Whether `version` is still the version of `uri` the client is looking
    /// at, which is what makes an answer worth sending.
    ///
    /// A second edit arriving while the first is being analysed makes the first
    /// answer one about a document nobody has on screen any more
    /// (`rule:ide/the-server-is-synchronous`), and the version it ran for is
    /// the only thing that says so. Asked twice: before a walk starts, by
    /// [`analyse_current`], and again before its answer is sent, because the
    /// keystroke that supersedes it can land at either moment.
    ///
    /// A document that has been closed is current at no version at all.
    #[must_use]
    pub fn is_current(&self, uri: &Uri, version: i32) -> bool {
        self.get(uri)
            .is_some_and(|document| document.version == version)
    }

    /// Every open document that has to be analysed again now `changed` has been
    /// edited, sorted by URI so a publish order is deterministic.
    ///
    /// That is the edited document itself when it is open, and every open
    /// document whose last analysis read it — an open `a.nvs` that requires the
    /// edited `b.nvs` is stale until touched otherwise, which reads as the
    /// server being wrong. Nothing else: a document nobody opened is not
    /// published for at all.
    #[must_use]
    pub fn to_republish(&self, changed: &Path) -> Vec<&Uri> {
        let changed = canonical_key(changed);
        let mut republish: Vec<&Uri> = self
            .buffers
            .values()
            .filter(|document| {
                document.path().map(canonical_key) == Some(changed.clone())
                    || self
                        .graphs
                        .get(&document.uri)
                        .is_some_and(|files| files.contains(&changed))
            })
            .map(Document::uri)
            .collect();
        republish.sort();
        republish
    }
}

/// One open document, analysed as its own entry point.
#[derive(Debug)]
pub struct Analysed {
    /// Every file the walk read, with the open buffers overlaid — so the text
    /// here is not necessarily the text on disk.
    pub map: SourceMap,
    /// The entry document, as [`map`](Self::map) knows it.
    pub entry: SourceId,
    /// The version of the entry document this walk read, which is what says
    /// whether the answer is still wanted — see [`Documents::is_current`].
    pub version: i32,
    /// The names the whole graph declares, resolved.
    pub module: Module,
    /// Each file the graph reached, entry first, with the statements the walk
    /// parsed once and every later phase needs again.
    pub loaded: Vec<Loaded>,
    /// The files whose `autoload` declarations this walk borrowed, and empty
    /// for an entry no other program autoloads. They are not in
    /// [`loaded`](Self::loaded) — the walk never opened them — and an edit to
    /// one still changes what this analysis resolves, which is why
    /// [`files`](Self::files) names them.
    pub lent: Vec<PathBuf>,
    /// Every whitespace run and every comment of the **entry** document, in
    /// source order — the other two thirds of the one parse that produced
    /// `loaded`'s first entry (`rule:ide/one-grammar-one-tree`).
    ///
    /// Only the entry has them, because only the entry is parsed here: a
    /// `require`d file is read by the graph walk, which takes the strict entry
    /// point, and a cursor is only ever in the open document.
    pub trivia: Vec<Trivia>,
    /// Which node of the entry document a byte offset is inside, and what that
    /// node is inside (`rule:ide/the-index-answers-the-cursor`).
    ///
    /// Scoped to the entry for the same reason [`trivia`](Self::trivia) is.
    pub index: SyntaxIndex,
    /// What the type phase resolved at each expression it recorded, keyed by
    /// that expression's span — the whole graph's, not the entry's alone,
    /// because a class the cursor names may be declared in a required file.
    ///
    /// This is the other half of a cursor answer: [`index`](Self::index) says
    /// which node an offset is in, and this says what that node's name and type
    /// resolved to (`rule:ide/the-index-answers-the-cursor`). It is
    /// `nvs_types`' own narrow table rather than a typed AST, so an expression
    /// shape it records nothing for is a cursor question this server cannot
    /// answer — which is the table's own documented shape and not a gap here.
    pub exprs: ExprTypeTable,
    /// The interner every [`TypeId`](nvs_types::TypeId) in
    /// [`exprs`](Self::exprs) was interned against.
    ///
    /// Kept beside the table because a type id read against any other interner
    /// is a different type, or none: an id is an index into exactly this run's
    /// table, so the two travel together or neither is readable.
    pub interner: TypeInterner,
    /// Every diagnostic the front end reported, ungated: the lexer's, the
    /// parser's, `check_declarations`', name resolution's and the type
    /// phase's.
    ///
    /// `rule:ide/diagnostics-are-phase-gated`'s gate is not applied here.
    /// It is presentation rather than analysis, so it belongs on the way to
    /// the wire — [`crate::diagnostics::phase_gated`] — and a case asking
    /// `phase=all` is one that wants exactly what the gate would have held
    /// back.
    pub diags: Diagnostics,
}

impl Analysed {
    /// The path of every file this analysis depends on: the graph it read,
    /// entry first, then each file it borrowed an `autoload` declaration from.
    ///
    /// This is what [`Documents::record_graph`] is handed: a file with no path
    /// cannot be edited, so it cannot be what a later change names.
    pub fn files(&self) -> impl Iterator<Item = PathBuf> {
        self.loaded
            .iter()
            .filter_map(|file| self.map.file(file.id).path().map(Path::to_path_buf))
            .chain(self.lent.iter().cloned())
    }

    /// Every body whose span covers `offset` in the entry document, innermost
    /// first.
    ///
    /// A closure's body is inside the body that wrote it and shares none of its
    /// bindings, and a method's is inside the file's own script frame on the
    /// same terms — so which of them a reader takes is its own question, and
    /// this orders them rather than answering it.
    ///
    /// **A cursor past the end of every body is in the script frame.** A span is
    /// half-open, so the cursor at the last byte of a file with no trailing
    /// newline is outside the frame that covers the whole file — and that is
    /// where a developer types in a file they have just started. Nothing else
    /// can be there: a method or a closure ends at a `}` that the frame still
    /// covers. So where no body covers `offset`, the widest body that starts
    /// before it answers, which is that frame.
    pub(crate) fn bodies_at(&self, offset: BytePos) -> Vec<&[LocalBinding]> {
        let scopes = || {
            self.exprs
                .local_scopes()
                .filter(|(body, _)| body.file == self.entry && body.start <= offset)
        };
        let mut bodies: Vec<(Span, &[LocalBinding])> =
            scopes().filter(|(body, _)| offset < body.end).collect();
        if bodies.is_empty() {
            bodies.extend(scopes().max_by_key(|(body, _)| body.end - body.start));
        }
        bodies.sort_by_key(|(body, _)| body.end - body.start);
        bodies.into_iter().map(|(_, locals)| locals).collect()
    }

    /// The type the binding named at `span` was declared with, `offset` being
    /// the position whose bodies are in scope.
    ///
    /// Innermost body first, and the first one that declares the name wins: a
    /// closure's body is inside the body that wrote it and shares none of its
    /// bindings, so the enclosing body's entry is the right answer for a name
    /// the closure captured and the wrong one for a name it declared itself.
    ///
    /// It lives here rather than beside either caller because two features now
    /// ask it: [`crate::completion`] resolves the members off `$u->`, and
    /// [`crate::semantic`] reads a qualifier off every use of a name. A local's
    /// declared type is a fact about [`Analysed`], so this is its one home.
    pub(crate) fn local_ty(&self, span: Span, offset: BytePos) -> Option<TypeId> {
        let text = self.map.file(self.entry).text();
        let name = text.get(span.range())?.strip_prefix('$')?;
        self.bodies_at(offset)
            .into_iter()
            .find_map(|locals| locals.iter().find(|local| local.name == name))
            .map(|local| local.ty)
    }
}

/// Analyses `uri` as its own entry point, with every open buffer overlaid on
/// the files under them.
///
/// `None` when `uri` is not open, when it names no file, or when the entry
/// itself will not load — a URI whose bytes are not valid UTF-8 is refused
/// here rather than panicking further in (`rule:ide/positions-have-one-home`).
///
/// The walk is `nvs check`'s, called the way `nvs-cli`'s `front_end` calls it,
/// because a graph an editor resolves differently from the compiler is a server
/// that disagrees with the build.
///
/// Every call re-reads and re-parses the whole graph, which is the price
/// `rule:ide/one-grammar-one-tree` paid for having one tree instead of a
/// `rowan`-shaped second one. What says that price is still worth paying is a
/// measurement rather than an assumption: `tests/latency.rs` names the bound a
/// ~1,000-line document's analysis stays under, what it was measured on, and
/// what to do first if it ever goes red.
#[must_use]
pub fn analyse(documents: &Documents, uri: &Uri) -> Option<Analysed> {
    let document = documents.get(uri)?;
    analyse_file(documents, document.path()?, document.version)
}

/// [`analyse`], for a file that need not be open.
///
/// The walk itself, with the entry named by path rather than by URI and its
/// `version` handed in. Both callers have one to hand: a request analyses the
/// document a client opened and stamped, and [`crate::index`] analyses whatever
/// file the scope selected — which for a file no client has open has no client
/// version at all, and records the one it is given. Nothing compares that
/// version: [`Documents::is_current`] is asked about open documents, and
/// nothing is published for a file nobody opened.
///
/// The open buffers are overlaid either way, so a file read from disk still
/// resolves its graph against the text an editor holds
/// (`rule:ide/an-open-document-is-its-own-entry-point`).
#[must_use]
pub fn analyse_file(documents: &Documents, path: &Path, version: i32) -> Option<Analysed> {
    let mut map = SourceMap::new();
    documents.overlay(&mut map);
    // The entry is loaded rather than added, even though its text is right
    // here: a `require` resolves against the requiring file's own directory,
    // and a file added by name has no path to take one from. The overlay
    // registered above is what makes that read reach the buffer.
    let entry = map.load(path).ok()?;

    let mut diags = Diagnostics::new();
    // The entry takes the lossless entry point and every required file keeps
    // the strict one: same grammar, same statements and same diagnostics either
    // way (`rule:ide/one-grammar-one-tree`), so this costs the trivia and the
    // index of one file and changes nothing else about the walk. Where each
    // modifier was written is dropped here: `nvs fmt` is what reads it, and a
    // server that reformats a document calls that. A cursor is only ever in the
    // open document, so a required file has no question to answer that its
    // statements do not already.
    let nvs_syntax::Parsed {
        stmts,
        trivia,
        modifiers: _,
        index,
    } = parse(map.file(entry), &mut diags);
    check_declarations(&stmts, map.file(entry), &mut diags);
    let core = nvs_stdlib::registry::link_targets();
    // The one place this walk is not `nvs check`'s, which is always handed the
    // file a program starts from. A file some program autoloads has no map of
    // its own to resolve a name through, so it borrows that program's
    // (`rule:ide/an-open-document-is-its-own-entry-point`).
    let lender = documents.lender_for(path);
    let lent = lender.map_or_else(Vec::new, |lender| lender.declaring.clone());
    let (module, loaded, _autoload) = resolve_program_borrowing(
        entry,
        stmts,
        &mut map,
        nvs_hir::CoreRoster::Names(&core),
        &mut diags,
        lender.map_or(&[], |lender| lender.map.sites()),
    );

    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    {
        // The type phase, continued into exactly the way `nvs-cli`'s
        // `front_end_granted` continues into it: `E0301`, `E0302` and every
        // `E04xx` are reported here and nowhere earlier, so a walk that
        // stopped above this block published parse and declaration
        // diagnostics alone.
        //
        // No grants are passed, which is the one place this front end is
        // deliberately not `nvs check`'s: `rule:core-classes/db-literal-query-checking`'s
        // check-time question is asked of a `--config` the editor never
        // names, and answering it against whichever `nvs.toml` this machine
        // happens to resolve would put a diagnostic on a line for a reason
        // the person typing cannot see.
        let files: Vec<nvs_types::ProgramFile<'_>> = loaded
            .iter()
            .map(|file| nvs_types::ProgramFile {
                src: map.file(file.id),
                stmts: &file.stmts,
            })
            .collect();
        // The enum table is dropped and the expression table is not: nothing
        // here lowers, so a case's constant value has no reader, while what
        // each expression resolved to is what a cursor request asks about.
        let _ = nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
    }

    Some(Analysed {
        map,
        entry,
        version,
        module,
        loaded,
        lent,
        trivia,
        index,
        exprs,
        interner,
        diags,
    })
}

/// [`analyse`], unless `version` has already been superseded.
///
/// This is the whole of cancellation, and it is deliberately a check *around*
/// the walk rather than one inside it: `resolve_program` is a single call with
/// no cancellation point in it, and an analysis a keystroke overtakes is either
/// one that has not started — dropped here — or one whose answer is discarded
/// before it is sent, which is [`Documents::is_current`] asked a second time.
/// Interrupting a walk in flight would need the front end to carry a token
/// through every phase, and it buys one document's parse.
#[must_use]
pub fn analyse_current(documents: &Documents, uri: &Uri, version: i32) -> Option<Analysed> {
    if !documents.is_current(uri, version) {
        return None;
    }
    analyse(documents, uri)
}

/// The file a `file:` URI names, or `None` for any other scheme.
///
/// Decoded here rather than through a URI library's decoder because the whole
/// conversion is the ten lines below, and the shape it has to get right is
/// Windows's: a client writes `file:///c%3A/src/a.nvs`, whose path component
/// begins with a `/` that is not part of the path at all.
#[must_use]
pub fn path_of(uri: &Uri) -> Option<PathBuf> {
    if !uri
        .scheme()
        .is_some_and(|scheme| scheme.as_str().eq_ignore_ascii_case("file"))
    {
        return None;
    }
    let decoded = percent_decode(uri.path().as_str())?;
    Some(PathBuf::from(strip_drive_slash(&decoded)))
}

/// The `file:` URI naming `path`, which is how a document the server found on
/// disk is named back to the client.
///
/// `None` for a path this process cannot spell as UTF-8, which no editor could
/// have sent in the first place.
#[must_use]
pub fn uri_of(path: &Path) -> Option<Uri> {
    let path = path.to_str()?;
    // `\\?\D:\a` and `D:\a` are the same file, and no client has ever been sent
    // the first spelling — `Path::canonicalize` is where it comes from.
    let path = path.strip_prefix(r"\\?\").unwrap_or(path);
    let mut uri = String::from("file://");
    if !path.starts_with(['/', '\\']) {
        // A drive-letter path is absolute with no leading separator, and the
        // empty authority's own slash goes in front of it.
        uri.push('/');
    }
    for byte in path.bytes() {
        match byte {
            b'\\' | b'/' => uri.push('/'),
            b':' | b'-' | b'.' | b'_' | b'~' => uri.push(char::from(byte)),
            _ if byte.is_ascii_alphanumeric() => uri.push(char::from(byte)),
            _ => write!(uri, "%{byte:02X}").ok()?,
        }
    }
    uri.parse().ok()
}

/// A percent-encoded string's bytes, as UTF-8.
///
/// `None` when an escape is truncated or is not hex, and when what comes out is
/// not UTF-8: a path this server cannot name is better refused than guessed at.
fn percent_decode(encoded: &str) -> Option<String> {
    let bytes = encoded.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0;

    while let Some(&byte) = bytes.get(at) {
        if byte == b'%' {
            decoded.push(u8::from_str_radix(encoded.get(at + 1..at + 3)?, 16).ok()?);
            at += 3;
        } else {
            decoded.push(byte);
            at += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

/// `/C:/src/a.nvs` without the slash a URI writes in front of a drive letter.
///
/// Unconditional rather than `#[cfg(windows)]`: no absolute POSIX path has a
/// `:` as its second character, so the test cannot fire where it should not,
/// and a conversion that behaves differently per host is one nobody can write a
/// case for.
fn strip_drive_slash(path: &str) -> &str {
    let mut chars = path.chars();
    let looks_like_a_drive = chars.next() == Some('/')
        && chars
            .next()
            .is_some_and(|letter| letter.is_ascii_alphabetic())
        && chars.next() == Some(':');

    if looks_like_a_drive {
        path.get(1..).unwrap_or(path)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use nvs_diagnostics::{Code, Diagnostic, code};

    use super::*;

    /// A directory of fixtures, because a `require` graph is resolved against
    /// the filesystem and there is nowhere else to put one.
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!("nvs-lsp-document-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn write(&self, name: &str, contents: &str) {
            fs::write(self.path.join(name), contents).expect("write fixture");
        }

        fn read(&self, name: &str) -> String {
            fs::read_to_string(self.path.join(name)).expect("read fixture")
        }

        /// What a client would call the fixture, which is the only name the
        /// store knows it by.
        fn uri(&self, name: &str) -> Uri {
            uri_of(&self.path.join(name)).expect("a temp path is UTF-8")
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// Open every named fixture with the text on disk.
    fn open_from_disk(documents: &mut Documents, dir: &TempDir, names: &[&str]) {
        for name in names {
            documents.open(dir.uri(name), 1, dir.read(name));
        }
    }

    /// `rule:ide/an-open-document-is-its-own-entry-point`: the graph is
    /// resolved with the open buffers overlaid on disk, so a class edited in
    /// one tab is the class another tab resolves against.
    ///
    /// The required file declares `Unsaved` in the buffer and `Saved` on disk,
    /// so what the walk collected says which of the two it read — the text in
    /// the map, and the name in the resolved program, have to be the buffer's.
    #[test]
    fn an_unsaved_buffer_shadows_the_file_on_disk() {
        let dir = TempDir::new("shadow");
        dir.write("lib.nvs", "<?nvs\nclass Saved {}\n");
        dir.write(
            "main.nvs",
            "<?nvs\nrequire 'lib.nvs';\n$u = new Unsaved();\n",
        );

        let mut documents = Documents::new();
        open_from_disk(&mut documents, &dir, &["main.nvs"]);
        documents.open(
            dir.uri("lib.nvs"),
            7,
            "<?nvs\nclass Unsaved {}\n".to_owned(),
        );

        let analysed = analyse(&documents, &dir.uri("main.nvs")).expect("main.nvs is open");

        let lib = analysed
            .loaded
            .iter()
            .map(|file| analysed.map.file(file.id))
            .find(|file| file.path().is_some_and(|path| path.ends_with("lib.nvs")))
            .expect("the walk followed the require");

        assert_eq!(
            lib.text(),
            "<?nvs\nclass Unsaved {}\n",
            "the graph read the file on disk instead of the open buffer"
        );
        assert_eq!(
            dir.read("lib.nvs"),
            "<?nvs\nclass Saved {}\n",
            "the fixture stopped being a shadow: disk and buffer now agree"
        );
        let declared: Vec<&str> = analysed
            .module
            .symbols
            .iter()
            .map(|symbol| symbol.qname.short_name())
            .collect();
        assert!(
            declared.contains(&"Unsaved") && !declared.contains(&"Saved"),
            "name resolution collected the declarations on disk rather than the \
             buffer's: {declared:?}"
        );
    }

    /// The rule's last paragraph: editing one document re-analyses every open
    /// document whose graph contains it, and nothing else — an open `main.nvs`
    /// that requires an edited `lib.nvs` is stale until touched otherwise,
    /// which reads as the server being wrong.
    #[test]
    fn editing_a_required_file_republishes_the_requiring_document() {
        let dir = TempDir::new("republish");
        dir.write("lib.nvs", "<?nvs\nclass User {}\n");
        dir.write("main.nvs", "<?nvs\nrequire 'lib.nvs';\n$u = new User();\n");
        dir.write("other.nvs", "<?nvs\nclass Other {}\n");

        let mut documents = Documents::new();
        open_from_disk(&mut documents, &dir, &["main.nvs", "other.nvs"]);
        for name in ["main.nvs", "other.nvs"] {
            let uri = dir.uri(name);
            let analysed = analyse(&documents, &uri).expect("both are open");
            let files: Vec<PathBuf> = analysed.files().collect();
            documents.record_graph(&uri, files);
        }

        // The edit, as a client makes one: `lib.nvs` is opened and changed.
        documents.open(dir.uri("lib.nvs"), 1, dir.read("lib.nvs"));
        assert!(documents.change(
            &dir.uri("lib.nvs"),
            2,
            "<?nvs\nclass User { public int $id; }\n".to_owned()
        ));

        let lib_path = dir.path.join("lib.nvs");
        let republished = |documents: &Documents| -> Vec<String> {
            documents
                .to_republish(&lib_path)
                .iter()
                .map(|uri| uri.as_str().to_owned())
                .collect()
        };

        let after_the_edit = republished(&documents);
        assert!(
            after_the_edit.contains(&dir.uri("main.nvs").as_str().to_owned()),
            "the document that requires the edited file was left stale: {after_the_edit:?}"
        );
        assert!(
            after_the_edit.contains(&dir.uri("lib.nvs").as_str().to_owned()),
            "the edited document itself was not republished: {after_the_edit:?}"
        );
        assert!(
            !after_the_edit.contains(&dir.uri("other.nvs").as_str().to_owned()),
            "a document whose graph never named the edited file was republished: \
             {after_the_edit:?}"
        );

        // Diagnostics are published only for open documents, so closing the
        // requirer takes it out of the answer rather than leaving a graph
        // behind for it.
        documents.close(&dir.uri("main.nvs"));
        assert!(
            !republished(&documents).contains(&dir.uri("main.nvs").as_str().to_owned()),
            "a closed document is still being published for"
        );
    }

    /// `rule:ide/the-server-is-synchronous`: the next keystroke makes the
    /// analysis under way an answer about a document nobody is looking at, and
    /// the version it was started for is the only thing that says so.
    #[test]
    fn an_analysis_for_a_superseded_version_is_cancelled() {
        let dir = TempDir::new("superseded");
        dir.write("main.nvs", "<?nvs\nclass Typed {}\n");

        let mut documents = Documents::new();
        let uri = dir.uri("main.nvs");
        open_from_disk(&mut documents, &dir, &["main.nvs"]);
        assert_eq!(
            analyse_current(&documents, &uri, 1).map(|analysed| analysed.version),
            Some(1),
            "the version the document was opened at is the one on screen"
        );

        // The keystroke that arrives while that walk is still queued.
        assert!(documents.change(&uri, 2, "<?nvs\nclass Retyped {}\n".to_owned()));
        assert!(
            analyse_current(&documents, &uri, 1).is_none(),
            "an analysis for a version the client has already replaced ran anyway"
        );

        let current = analyse_current(&documents, &uri, 2).expect("version 2 is on screen");
        assert_eq!(current.version, 2);
        assert!(
            current.map.file(current.entry).text().contains("Retyped"),
            "the walk read a buffer the change had already replaced"
        );

        // A closed document is current at no version at all: an answer still in
        // flight when the tab closed has nowhere left to go.
        documents.close(&uri);
        assert!(!documents.is_current(&uri, 2));
        assert!(analyse_current(&documents, &uri, 2).is_none());
    }

    /// A URI is the identity on the wire and a path is the one the compiler
    /// uses, and the round trip has to survive a space and a drive letter —
    /// `C:\Program Files` is where this would first be noticed.
    #[test]
    fn a_file_uri_round_trips_through_the_path_it_names() {
        let dir = TempDir::new("round trip");
        dir.write("a.nvs", "<?nvs\n");
        let path = dir.path.join("a.nvs");

        let uri = uri_of(&path).expect("a temp path is UTF-8");
        assert!(uri.as_str().starts_with("file:///"), "{}", uri.as_str());
        assert_eq!(
            path_of(&uri).map(|round_tripped| canonical_key(&round_tripped)),
            Some(canonical_key(&path))
        );
    }

    /// Only a `file:` URI has a path, and a document under any other scheme is
    /// held rather than guessed about.
    #[test]
    fn a_document_that_names_no_file_is_open_and_analysed_as_nothing() {
        let uri: Uri = "untitled:Untitled-1".parse().expect("a valid URI");
        let mut documents = Documents::new();
        documents.open(uri.clone(), 1, "<?nvs\n".to_owned());

        assert_eq!(documents.len(), 1);
        assert!(documents.get(&uri).expect("it is open").path().is_none());
        assert!(analyse(&documents, &uri).is_none());
    }

    /// The bands `rule:ide/diagnostics-are-phase-gated` holds back — name
    /// resolution's, and the type check's across all three it occupies.
    /// Spelled out here rather than read off [`crate::diagnostics`] so a test
    /// and the code it gates cannot agree by sharing one mistake.
    const READS_THE_TREE: &[&str] = &["E03", "E04", "E07", "E08"];

    /// The codes among `diagnostics` that point into the fixture named `name`.
    fn codes_in<'a>(
        diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
        analysed: &Analysed,
        name: &str,
    ) -> Vec<&'static str> {
        diagnostics
            .into_iter()
            .filter(|diagnostic| {
                diagnostic.primary_span().is_some_and(|span| {
                    analysed
                        .map
                        .file(span.file)
                        .path()
                        .is_some_and(|path| path.ends_with(name))
                })
            })
            .filter_map(|diagnostic| diagnostic.code.map(Code::as_str))
            .collect()
    }

    /// Whether any of `codes` is one the gate holds back.
    fn any_reads_the_tree(codes: &[&str]) -> bool {
        codes
            .iter()
            .any(|code| READS_THE_TREE.iter().any(|band| code.starts_with(band)))
    }

    /// `rule:ide/diagnostics-are-phase-gated` in the direction that motivates
    /// it: a file whose parse failed publishes the parse error and not the
    /// cascade underneath it, and the other file in the same graph keeps every
    /// diagnostic of its own — a broken buffer in one tab is not a reason to go
    /// dark in another.
    ///
    /// The first assertion is the one that keeps this honest. Without it the
    /// test passes just as well over an analysis that never ran the type phase
    /// at all, which is exactly the state this crate was in before.
    #[test]
    fn a_parse_error_suppresses_the_type_diagnostics_of_that_file_only() {
        let dir = TempDir::new("gate");
        dir.write("lib.nvs", "<?nvs\nclass User {}\nvar $stray = $nope;\n");
        dir.write(
            "main.nvs",
            "<?nvs\nrequire 'lib.nvs';\nvar $broken = ;\nvar $other = $nope;\n",
        );

        let mut documents = Documents::new();
        open_from_disk(&mut documents, &dir, &["main.nvs"]);
        let analysed = analyse(&documents, &dir.uri("main.nvs")).expect("main.nvs is open");

        let ungated = codes_in(analysed.diags.iter(), &analysed, "main.nvs");
        assert!(
            any_reads_the_tree(&ungated),
            "the walk reported nothing for the gate to suppress, so this case \
             would pass over an analysis that stops at name resolution: {ungated:?}"
        );

        let published = crate::diagnostics::phase_gated(&analysed.diags);
        let main = codes_in(published.iter().copied(), &analysed, "main.nvs");
        assert!(
            main.contains(&code::E_EXPECTED_EXPR.as_str()),
            "the parse error itself was not published: {main:?}"
        );
        assert!(
            !any_reads_the_tree(&main),
            "a diagnostic from below the broken parse reached the editor: {main:?}"
        );

        let lib = codes_in(published.iter().copied(), &analysed, "lib.nvs");
        assert!(
            any_reads_the_tree(&lib),
            "the gate went workspace-wide: a file that parses fine lost its own \
             diagnostics because another file in the graph did not: {lib:?}"
        );
    }

    /// The other direction, which the rule is equally explicit about: the
    /// suppression is one-directional. Resolution and the type check read the
    /// same whole tree, so neither cascades into the other the way a failed
    /// parse cascades into both, and a file with one of each publishes both.
    #[test]
    fn a_resolution_error_does_not_suppress_a_type_error() {
        let dir = TempDir::new("one way");
        dir.write(
            "main.nvs",
            "<?nvs\nclass Greeter {\n    public function count(): int {\n        \
             return \"not an int\";\n    }\n}\nvar $stray = $nope;\n",
        );

        let mut documents = Documents::new();
        open_from_disk(&mut documents, &dir, &["main.nvs"]);
        let analysed = analyse(&documents, &dir.uri("main.nvs")).expect("main.nvs is open");

        let published = crate::diagnostics::phase_gated(&analysed.diags);
        let main = codes_in(published.iter().copied(), &analysed, "main.nvs");

        assert!(
            main.iter().any(|code| code.starts_with("E03")),
            "the resolution error is the premise of this case and it is not \
             here: {main:?}"
        );
        assert!(
            main.iter().any(|code| code.starts_with("E04")
                || code.starts_with("E07")
                || code.starts_with("E08")),
            "the type error was suppressed by a resolution error, which nothing \
             asked for: {main:?}"
        );
        assert_eq!(
            main,
            codes_in(analysed.diags.iter(), &analysed, "main.nvs"),
            "the gate held something back in a file that parses"
        );
    }

    /// `rule:ide/one-grammar-one-tree`: the walk's own parse is the lossless
    /// one, so the trivia and the index come out of the same pass that produced
    /// the statements rather than out of a second one.
    ///
    /// The required file carries a comment too, and it is the control: only the
    /// entry is parsed here, so a trivium from `lib.nvs` in this list would
    /// mean the analysis had started answering about a file no cursor can be
    /// in.
    #[test]
    fn the_entry_documents_trivia_and_index_survive_the_walk() {
        let dir = TempDir::new("lossless");
        dir.write(
            "lib.nvs",
            "<?nvs\n// the required file's comment\nclass Lib {}\n",
        );
        dir.write(
            "main.nvs",
            "<?nvs\nrequire 'lib.nvs';\n// the entry's own comment\nclass Main {}\n",
        );

        let mut documents = Documents::new();
        open_from_disk(&mut documents, &dir, &["main.nvs"]);
        let analysed = analyse(&documents, &dir.uri("main.nvs")).expect("main.nvs is open");

        let text = analysed.map.file(analysed.entry).text();
        let comments: Vec<&str> = analysed
            .trivia
            .iter()
            .filter(|trivium| trivium.kind == nvs_syntax::TriviaKind::LineComment)
            .map(|trivium| &text[trivium.span.start as usize..trivium.span.end as usize])
            .collect();
        assert_eq!(
            comments,
            ["// the entry's own comment"],
            "the entry's comments are not what the walk kept"
        );
        assert!(
            analysed
                .trivia
                .iter()
                .all(|trivium| trivium.span.file == analysed.entry),
            "a trivium came from a file that is not the entry"
        );

        let class = text.find("class Main").expect("the fixture declares it");
        let path = analysed
            .index
            .at(u32::try_from(class).expect("a fixture is short"));
        assert_eq!(
            path.innermost().map(|node| node.kind),
            Some("ClassDecl"),
            "the index does not answer at the entry's own class declaration"
        );
    }

    /// `rule:ide/an-open-document-is-its-own-entry-point`: what the type phase
    /// resolved is kept rather than dropped, because the walk that resolved it
    /// is the only one an editor runs and a cursor request asks exactly that.
    ///
    /// The required file is the control the other way round from the test
    /// above: the class the entry allocates is declared over there, so an
    /// entry-scoped table could not name it. And the type the entry recorded is
    /// read back through the interner kept beside it, which is the whole reason
    /// the two travel together.
    #[test]
    fn the_entry_documents_expression_types_survive_the_walk() {
        let dir = TempDir::new("typed");
        dir.write("lib.nvs", "<?nvs\nclass Lib {}\n");
        dir.write("main.nvs", "<?nvs\nrequire 'lib.nvs';\n$l = new Lib();\n");

        let mut documents = Documents::new();
        open_from_disk(&mut documents, &dir, &["main.nvs"]);
        let analysed = analyse(&documents, &dir.uri("main.nvs")).expect("main.nvs is open");

        let text = analysed.map.file(analysed.entry).text();
        let written = "new Lib()";
        let start = text.find(written).expect("the fixture writes it");
        let span = nvs_diagnostics::Span::new(
            analysed.entry,
            u32::try_from(start).expect("a fixture is short"),
            u32::try_from(start + written.len()).expect("a fixture is short"),
        );

        let Some(nvs_types::ExprInfo::New { class, ty, .. }) = analysed.exprs.lookup(span) else {
            panic!("the walk kept no resolution for the `new` the entry writes");
        };
        assert_eq!(
            class.to_string(),
            "Lib",
            "the `new` resolved to a class the required file does not declare"
        );
        assert!(
            matches!(analysed.interner.get(*ty), nvs_types::Ty::Class(named, args)
                if named == class && args.is_empty()),
            "the type recorded for the `new` does not read back through the interner beside it"
        );
    }
}
