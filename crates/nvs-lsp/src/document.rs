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
use nvs_diagnostics::{Diagnostics, SourceId, SourceMap, canonical_key};
use nvs_hir::{Loaded, Module, resolve_program};
use nvs_syntax::{check_declarations, parse_file};

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
    /// What the lexer, the parser, `check_declarations` and name resolution
    /// reported. The type phase is not run here — that is the diagnostics
    /// slice's, along with `rule:ide/diagnostics-are-phase-gated`'s gate.
    pub diags: Diagnostics,
}

impl Analysed {
    /// The path of every file the graph read, entry first.
    ///
    /// This is what [`Documents::record_graph`] is handed: a file with no path
    /// cannot be edited, so it cannot be what a later change names.
    pub fn files(&self) -> impl Iterator<Item = PathBuf> {
        self.loaded
            .iter()
            .filter_map(|file| self.map.file(file.id).path().map(Path::to_path_buf))
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
#[must_use]
pub fn analyse(documents: &Documents, uri: &Uri) -> Option<Analysed> {
    let document = documents.get(uri)?;
    let path = document.path()?;

    let mut map = SourceMap::new();
    documents.overlay(&mut map);
    // The entry is loaded rather than added, even though its text is right
    // here: a `require` resolves against the requiring file's own directory,
    // and a file added by name has no path to take one from. The overlay
    // registered above is what makes that read reach the buffer.
    let entry = map.load(path).ok()?;

    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(entry), &mut diags);
    check_declarations(&stmts, map.file(entry), &mut diags);
    let (module, loaded, _autoload) = resolve_program(entry, stmts, &mut map, &mut diags);

    Some(Analysed {
        map,
        entry,
        version: document.version,
        module,
        loaded,
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
}
