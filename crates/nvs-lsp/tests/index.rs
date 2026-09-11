//! One workspace symbol index, built in one place and invalidated by what
//! changed.
//!
//! `rule:ide/five-features-are-one-reference-index` asks for three things a
//! test can hold: exactly one construction site, an invalidation that stops at
//! the file that changed and the files that read it, and a scope that selects
//! which files are indexed rather than which code does the indexing. The bound
//! the index has to stay under with all of that warm is `tests/latency.rs`'s,
//! beside the cold one it already measured.
//!
//! The fixtures are on disk rather than buffers alone, on
//! `tests/publish.rs`'s terms: a `require` resolves against the requiring
//! file's own directory, and a workspace pass walks a directory for `.nvs`
//! files, so both halves need files to find.

use std::fs;
use std::path::{Path, PathBuf};

use nvs_lsp::{CheckScope, DeclKind, Documents, SymbolIndex, uri_of};

/// A scratch directory that cleans up after itself.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("nvs-index-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a scratch directory");
        Self { path }
    }

    fn write(&self, name: &str, text: &str) {
        fs::write(self.path.join(name), text).expect("a writable scratch file");
    }

    fn at(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    /// Opens `name` as a buffer over the file already written there.
    fn open(&self, documents: &mut Documents, name: &str, text: &str) {
        let uri = uri_of(&self.at(name)).expect("a temp path is UTF-8");
        documents.open(uri, 1, text.to_owned());
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// A class with a property and a method, so the index has a member declaration
/// to hold and a member use to resolve.
const LIB: &str = "<?nvs\nclass Counter {\n    public int $count = 0;\n    \
                   public function bump(): int {\n        return $this->count;\n    }\n}\n";

/// The document that requires [`LIB`] and uses both of its names.
const MAIN: &str = "<?nvs\nrequire 'lib.nvs';\nvar $c = new Counter();\necho $c->bump();\n";

/// A document in the same directory that requires nothing and is required by
/// nothing — the "and nothing else" half of two of the cases below.
const LONE: &str = "<?nvs\nclass Lonely {\n    public function greet(): string {\n        \
                    return \"hi\";\n    }\n}\n";

/// The file name at the end of a path, for an assertion a reader can read.
fn name_of(path: &Path) -> String {
    path.file_name()
        .expect("an indexed path names a file")
        .to_string_lossy()
        .into_owned()
}

/// The names of a set of paths, sorted.
fn names(paths: &[PathBuf]) -> Vec<String> {
    let mut found: Vec<String> = paths.iter().map(|path| name_of(path)).collect();
    found.sort();
    found
}

/// Every `.rs` file under `dir`, recursively.
fn rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

/// This crate's own source files.
fn crate_sources() -> Vec<PathBuf> {
    let mut found = Vec::new();
    rust_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    assert!(found.len() > 5, "the crate's source was not found");
    found
}

/// Which files hold `needle` in code — a line that is a comment does not count,
/// because a module doc saying what the one construction site is would
/// otherwise read as a second one.
fn code_hits(needle: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for path in crate_sources() {
        let text =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let hits = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .filter(|line| line.contains(needle))
            .count();
        if hits > 0 {
            found.push((name_of(&path), hits));
        }
    }
    found
}

/// The index is built in exactly one place, and every other module that ever
/// names it reads one.
///
/// `rule:ide/five-features-are-one-reference-index`'s structural check, and the
/// thing it is guarding against is not a second `SymbolIndex` type — it is a
/// feature that walks the front end for names of its own because reaching the
/// index from where it sits was inconvenient. So what is counted is the entry
/// construction: one place builds an indexed file, and it is `index.rs`.
#[test]
fn the_crate_has_exactly_one_symbol_index_construction_site() {
    let built = code_hits("Indexed {");
    // The declaration and the one literal, both in `index.rs`, and nothing
    // anywhere else: the type is private to that module, so a second site has
    // to be written there where this test can see it.
    assert_eq!(
        built,
        vec![("index.rs".to_owned(), 2)],
        "an indexed file is built somewhere other than index.rs, or more than \
         once inside it: {built:?}"
    );

    let entry_points = code_hits("SymbolIndex::build");
    assert!(
        entry_points.is_empty() || entry_points == vec![("index.rs".to_owned(), 1)],
        "the index is built from inside the crate somewhere other than its own \
         module: {entry_points:?}"
    );

    // Every other module reads a `&SymbolIndex`. A module holding one by value
    // is a module that built it. `lib.rs` is the crate's export list rather
    // than a reader, and naming the type there is what makes it public at all.
    for (file, _) in code_hits("SymbolIndex") {
        if file == "lib.rs" {
            continue;
        }
        assert_eq!(
            file, "index.rs",
            "{file} names SymbolIndex; a reader takes one by reference and the \
             one construction site is index.rs"
        );
    }
}

/// The declaration side and the occurrence side, from one walk.
#[test]
fn the_index_holds_every_declaration_and_every_resolved_use() {
    let dir = TempDir::new("holds");
    dir.write("lib.nvs", LIB);
    dir.write("main.nvs", MAIN);

    let mut documents = Documents::new();
    dir.open(&mut documents, "main.nvs", MAIN);
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);

    // The required file is indexed although nobody opened it: an open
    // document's graph is in scope under `rule:ide/check-scope-defaults-to-open-documents`'s
    // default.
    assert!(
        index.holds(&dir.at("lib.nvs")),
        "the required file is indexed"
    );

    let class = index
        .declaration("Counter")
        .expect("the class the graph declares");
    assert_eq!(class.kind, DeclKind::Class);
    assert_eq!(name_of(&class.site.path), "lib.nvs");

    assert_eq!(
        index.declaration("Counter::bump").map(|found| found.kind),
        Some(DeclKind::Method),
        "a method is a declared name, spelled Class::member"
    );
    assert_eq!(
        index.declaration("Counter::$count").map(|found| found.kind),
        Some(DeclKind::Property),
        "a property keeps its sigil, which is what tells C::$x from C::x"
    );

    // The occurrence side: the use is in the file that wrote it, not in the
    // file that declared it.
    let used = index.occurrences("Counter");
    assert_eq!(
        used.iter()
            .map(|found| name_of(&found.site.path))
            .collect::<Vec<_>>(),
        vec!["main.nvs"],
        "`new Counter()` is an occurrence of the class, in main.nvs"
    );
    let called = index.occurrences("Counter::bump");
    assert_eq!(
        called.len(),
        1,
        "the call resolved to the method, once: {called:?}"
    );
    assert_eq!(name_of(&called[0].site.path), "main.nvs");
}

/// A change drops the file and the files whose analysis read it, and leaves
/// everything else exactly where it was.
#[test]
fn a_change_invalidates_the_file_and_its_readers_and_nothing_else() {
    let dir = TempDir::new("invalidate");
    dir.write("lib.nvs", LIB);
    dir.write("main.nvs", MAIN);
    dir.write("lone.nvs", LONE);

    let mut documents = Documents::new();
    dir.open(&mut documents, "lib.nvs", LIB);
    dir.open(&mut documents, "main.nvs", MAIN);
    dir.open(&mut documents, "lone.nvs", LONE);

    let mut index = SymbolIndex::build(&documents, CheckScope::Open, None);
    assert_eq!(index.len(), 3, "three files, three entries");
    let untouched: Vec<_> = index.declarations_in(&dir.at("lone.nvs")).to_vec();
    assert!(!untouched.is_empty(), "the third file declares something");

    // The edited file is required by main.nvs and by nothing else.
    let dropped = index.refresh(&documents, &dir.at("lib.nvs"));
    assert_eq!(
        names(&dropped),
        vec!["lib.nvs".to_owned(), "main.nvs".to_owned()],
        "the file that changed and the file that read it, and nothing else"
    );
    assert_eq!(
        index.declarations_in(&dir.at("lone.nvs")),
        untouched.as_slice(),
        "a file that neither changed nor read what did was re-indexed anyway"
    );
    assert_eq!(index.len(), 3, "everything dropped was built again");

    // The other direction is not symmetric, and that is the point: lib.nvs
    // does not read main.nvs, so editing main.nvs leaves it alone.
    let dropped = index.refresh(&documents, &dir.at("main.nvs"));
    assert_eq!(
        names(&dropped),
        vec!["main.nvs".to_owned()],
        "editing a document invalidated a file that only it requires"
    );
}

/// `nvs.check.scope` widens which files are indexed and changes nothing about
/// how one is.
#[test]
fn check_scope_selects_the_tree_and_never_the_construction_site() {
    let dir = TempDir::new("scope");
    dir.write("lib.nvs", LIB);
    dir.write("main.nvs", MAIN);
    dir.write("lone.nvs", LONE);

    let mut documents = Documents::new();
    dir.open(&mut documents, "main.nvs", MAIN);

    // The same root either way, so the only difference is the setting.
    let open = SymbolIndex::build(&documents, CheckScope::Open, Some(&dir.path));
    let workspace = SymbolIndex::build(&documents, CheckScope::Workspace, Some(&dir.path));

    assert!(
        !open.holds(&dir.at("lone.nvs")),
        "the default scope is the open documents and their graphs, and nobody \
         opened or required lone.nvs"
    );
    assert!(
        workspace.holds(&dir.at("lone.nvs")),
        "a workspace pass indexes a file nobody opened"
    );
    assert_eq!(
        workspace.declaration("Lonely").map(|found| found.kind),
        Some(DeclKind::Class),
        "the widened tree is what the wider answer comes from"
    );

    // The file both scopes reached is indexed identically, which is what says
    // the scope selected the tree rather than a second way of building one.
    assert_eq!(
        open.declarations_in(&dir.at("main.nvs")),
        workspace.declarations_in(&dir.at("main.nvs")),
    );
    assert_eq!(
        open.occurrences_in(&dir.at("lib.nvs")),
        workspace.occurrences_in(&dir.at("lib.nvs")),
    );
}
