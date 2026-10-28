//! One workspace symbol index, built in one place and invalidated by what
//! changed.
//!
//! `rule:ide/five-features-are-one-reference-index` asks for four things a
//! test can hold: exactly one construction site, the eight readers it names and
//! no ninth, an invalidation that stops at the file that changed and the files
//! that read it, and a scope that selects which files are indexed rather than
//! which code does the indexing. The bound the index has to stay under with
//! all of that warm is `tests/latency.rs`'s, beside the cold one it already
//! measured.
//!
//! The fixtures are on disk rather than buffers alone, on
//! `tests/publish.rs`'s terms: a `require` resolves against the requiring
//! file's own directory, and a workspace pass walks a directory for `.nvs`
//! files, so both halves need files to find.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use nvs_lsp::reach::{Reach, reach};
use nvs_lsp::{CheckScope, DeclKind, Documents, SymbolIndex, Visibility, path_of, uri_of};

/// A scratch directory that cleans up after itself.
struct TempDir {
    path: nvs_repo::Scratch,
}

impl TempDir {
    fn new(name: &str) -> Self {
        Self {
            path: nvs_repo::scratch(&format!("lsp-index-{name}")),
        }
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

/// One class writing every modifier shape the visibility field has to tell
/// apart, including the two that are written as nothing and the one that
/// restricts writes rather than reads.
const MODIFIED: &str = "<?nvs\nclass Modified {\n    public int $open = 0;\n    \
                        private int $shut = 0;\n    protected int $kin = 0;\n    \
                        int $bare = 0;\n    public private(set) int $written = 0;\n    \
                        private const HIDDEN = 1;\n    \
                        private function tell(): int {\n        return $this->shut;\n    }\n}\n";

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

    // `build` is called only by the module that owns the store it reads, which
    // is what makes every reader's `&SymbolIndex` come from the same place: a
    // caller anywhere else would be a second index with staleness of its own.
    // `server.rs` builds three and all are that module's: the session's,
    // refreshed as documents arrive; the one a `.lspt` case is answered
    // against; and the `Workspace` `nvs code` asks by name. The last two are
    // built for one question and dropped with its answer.
    for (file, _) in code_hits("SymbolIndex::build") {
        assert_eq!(
            file, "server.rs",
            "the index is built outside the one module that owns one"
        );
    }

    // Every other module reads a `&SymbolIndex`, with a lifetime where the
    // reference is a struct's field. A module holding one by value
    // is a module that built it, and an import is how one says the name at all
    // before it can take a reference to it. `lib.rs` is the crate's export list
    // rather than a reader, and naming the type there is what makes it public
    // at all; `server.rs` is the owner above.
    for path in crate_sources() {
        let file = name_of(&path);
        if matches!(file.as_str(), "index.rs" | "lib.rs" | "server.rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("a crate source");
        let held: Vec<&str> = text
            .lines()
            .map(str::trim_start)
            .filter(|line| !line.starts_with("//") && !line.starts_with("use "))
            .filter(|line| {
                line.contains("SymbolIndex")
                    && !line.contains("&SymbolIndex")
                    && !line.contains("&'a SymbolIndex")
            })
            .collect();
        assert!(
            held.is_empty(),
            "{file} names SymbolIndex other than behind a `&`; a reader takes \
             one by reference and the one construction site is index.rs: {held:?}"
        );
    }
}

/// The index's read-only methods that answer nobody's feature: how much it
/// holds, whether it holds one file, how many of its files had errors, and
/// which entry points read one file.
///
/// The refresh path, `nvs code`, a file move's `require` rewrite and this
/// suite ask these; a feature asks the queries beside them, which is what
/// [`every_reader_queries_the_one_index`] counts. None of them looks a name
/// up.
const BOOKKEEPING: [&str; 6] = [
    "files",
    "holds",
    "len",
    "is_empty",
    "files_with_errors",
    "readers",
];

/// Eight readers, and each one of them is a query against the one index.
///
/// The count is half of `rule:ide/five-features-are-one-reference-index`,
/// which names the five reference features, the file-kind reader, the
/// workspace symbol search and the call graph
/// (`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method`) — so a
/// ninth is a decision and not a slice. The other half is that each of the
/// eight *reads* the index: a feature
/// that walks the front end for names of its own is what the rule refuses, and
/// the construction-site test above cannot see one, because such a feature
/// builds no index at all.
///
/// The two sets have to match in both directions. A query nothing outside
/// `index.rs` calls is a reader that was quietly dropped; a read-only method
/// the table below does not name is a ninth reader that arrived without the
/// decision. A ninth feature answered from a query one of these eight already
/// makes is the one case this cannot see, and `tests/handshake.rs`'s closed
/// capability set is what catches that one.
#[test]
fn every_reader_queries_the_one_index() {
    // Each reader, and what it answers from. Six of them answer a request, the
    // dimming is published unasked, and the call graph is what the "used by"
    // lens walks and call hierarchy reads one step at a time.
    let readers: [(&str, &[&str]); 8] = [
        (
            "textDocument/references",
            &["occurrences", "declaration", "imports_of"],
        ),
        ("textDocument/documentHighlight", &["occurrences_in"]),
        (
            "textDocument/codeLens",
            &[
                "declarations_in",
                "occurrences",
                "subtypes",
                "overridden",
                "overriders",
                "queries_in",
                "parses",
            ],
        ),
        (
            "textDocument/prepareTypeHierarchy",
            &["supertypes", "subtypes"],
        ),
        ("unused-member dimming", &["unused_private"]),
        ("nvs/fileKinds", &["sole_kinds"]),
        ("workspace/symbol", &["matching"]),
        (
            "the call graph",
            &[
                "declaration",
                "occurrences",
                "requests",
                "unfollowed",
                "overridden",
                "occurrences_in",
                "requests_in",
            ],
        ),
    ];

    let mut queried: BTreeSet<&str> = BTreeSet::new();
    for (reader, queries) in readers {
        for query in queries {
            let callers: Vec<(String, usize)> = code_hits(&format!(".{query}("))
                .into_iter()
                .filter(|(file, _)| file != "index.rs")
                .collect();
            assert!(
                !callers.is_empty(),
                "{reader} is one of the seven readers and nothing outside \
                 index.rs calls `{query}`"
            );
            queried.insert(query);
        }
    }

    // Every read-only method the index offers. `build`, `refresh` and
    // `invalidate` are not among them: they take the index mutably, and holding
    // one that way is the construction site's business rather than a feature's.
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("index.rs"),
    )
    .expect("the index's own source");
    let offered: BTreeSet<&str> = source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("pub fn "))
        .filter_map(|rest| rest.split_once("(&self"))
        .map(|(name, _)| name)
        .filter(|name| !BOOKKEEPING.contains(name))
        .collect();

    assert_eq!(
        offered, queried,
        "the index's read side and the eight readers' queries are not the same \
         set, so a reader was dropped or a ninth one was added"
    );
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
    // document's graph is in scope under either value of
    // `rule:ide/check-scope-defaults-to-the-workspace`.
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

/// A declaration records the visibility it was written with, and `public` when
/// nothing was written.
///
/// Unused-member dimming is the whole reason this is here
/// (`rule:ide/five-features-are-one-reference-index`): "no occurrence anywhere
/// in the index" means unreachable for a private member and means nothing at
/// all for a public one, so a field that defaulted the wrong way would dim a
/// library's entire public surface.
#[test]
fn a_declaration_records_the_visibility_it_was_written_with() {
    let dir = TempDir::new("visibility");
    dir.write("lib.nvs", MODIFIED);

    let mut documents = Documents::new();
    dir.open(&mut documents, "lib.nvs", MODIFIED);
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);

    let seen = |symbol: &str| {
        index
            .declaration(symbol)
            .unwrap_or_else(|| panic!("{symbol} is declared"))
            .visibility
    };

    assert_eq!(seen("Modified::$open"), Visibility::Public);
    assert_eq!(seen("Modified::$shut"), Visibility::Private);
    assert_eq!(seen("Modified::$kin"), Visibility::Protected);
    assert_eq!(seen("Modified::HIDDEN"), Visibility::Private);
    assert_eq!(seen("Modified::tell"), Visibility::Private);

    // Two defaults, and they are the same default: a member written with no
    // modifier at all is public, and so is the class itself, which carries no
    // modifier list to read.
    assert_eq!(seen("Modified::$bare"), Visibility::Public);
    assert_eq!(seen("Modified"), Visibility::Public);

    // `private(set)` restricts writes and leaves the member readable, so it is
    // not a private member and dimming it would hide a name in use.
    assert_eq!(seen("Modified::$written"), Visibility::Public);
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
    let refreshed = index.refresh(&documents, &dir.at("lib.nvs"));
    let dropped = refreshed.dropped;
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
    // Both entries it re-analysed are open, so both analyses come back, each
    // at the version its document is at: the server publishes from them.
    let analysed: Vec<(PathBuf, i32)> = refreshed
        .analysed
        .iter()
        .map(|(uri, analysed)| (path_of(uri).expect("a file URI"), analysed.version))
        .collect();
    let opened: Vec<(PathBuf, i32)> = analysed
        .iter()
        .map(|(path, _)| {
            let uri = uri_of(path).expect("a UTF-8 path");
            (path.clone(), documents.get(&uri).expect("open").version())
        })
        .collect();
    assert_eq!(
        names(
            &analysed
                .iter()
                .map(|(path, _)| path.clone())
                .collect::<Vec<_>>()
        ),
        vec!["lib.nvs".to_owned(), "main.nvs".to_owned()],
        "the refresh did not hand back the analyses of the open entries it re-analysed"
    );
    assert_eq!(
        analysed, opened,
        "an analysis was handed back at a stale version"
    );

    // The other direction is not symmetric, and that is the point: lib.nvs
    // does not read main.nvs, so editing main.nvs leaves it alone.
    let dropped = index.refresh(&documents, &dir.at("main.nvs")).dropped;
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
    let open = SymbolIndex::build(&documents, CheckScope::Open, Some(dir.path.path()));
    let workspace = SymbolIndex::build(&documents, CheckScope::Workspace, Some(dir.path.path()));

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

/// Two routes and three tests over a chain of static calls: a direct caller,
/// a chain of two, a method that calls itself, a test that reaches a handler
/// through `Core\Test::request`, and a method only a callable value reaches.
const SHOP: &str = r#"<?nvs
use Core\Access;
use Core\Audience;
use Core\Http\Method;
use Core\Route;
use Core\Test;

final class Shop
{
    #[Route(path: '/items/{id}', method: Method::Get)]
    #[Access(allow: Audience::Public)]
    public static function show(string $id): void
    {
        echo Shop::price($id);
    }

    #[Route(path: '/total', method: Method::Get)]
    #[Access(allow: Audience::Public)]
    public static function total(): void
    {
        echo Shop::sum(3);
    }

    public static function price(string $id): int
    {
        return Shop::tax(1);
    }

    public static function tax(int $amount): int
    {
        return $amount;
    }

    public static function sum(int $n): int
    {
        if ($n == 0) {
            return 0;
        }
        return Shop::sum($n - 1) + Shop::tax($n);
    }

    public static function untouched(): int
    {
        return 1;
    }
}

final class ShopTest
{
    #[Test]
    public function showsAnItem(): void
    {
        var $response = Test::request(Method::Get, '/items/7');
        Test::assertEquals($response->status(), 200);
    }

    #[Test]
    public function addsTax(): void
    {
        Test::assertEquals(Shop::tax(2), 2);
    }

    #[Test]
    public function takesACallable(): void
    {
        var $later = Shop::untouched;
        Test::assertEquals($later(), 1);
    }
}
"#;

/// The index over [`SHOP`], opened as the one document.
fn shop_index(dir: &TempDir) -> SymbolIndex {
    dir.write("shop.nvs", SHOP);
    let mut documents = Documents::new();
    dir.open(&mut documents, "shop.nvs", SHOP);
    SymbolIndex::build(&documents, CheckScope::Open, None)
}

/// The strings a test compares a [`Reach`] with.
fn reached(routes: &[&str], tests: &[&str], at_least: bool) -> Reach {
    Reach {
        routes: routes.iter().map(|route| (*route).to_owned()).collect(),
        tests: tests.iter().map(|test| (*test).to_owned()).collect(),
        at_least,
    }
}

/// Each use records the method it is written in, and a use outside every
/// method records none.
#[test]
fn an_occurrence_records_the_method_it_is_written_in() {
    let dir = TempDir::new("caller");
    let index = shop_index(&dir);

    let callers: BTreeSet<Option<String>> = index
        .occurrences("Shop::tax")
        .into_iter()
        .map(|used| used.caller.clone())
        .collect();
    let want: BTreeSet<Option<String>> = ["Shop::price", "Shop::sum", "ShopTest::addsTax"]
        .into_iter()
        .map(|caller| Some(caller.to_owned()))
        .collect();
    assert_eq!(callers, want, "each call to tax names the method around it");

    let route = index
        .declaration("Shop::show")
        .expect("the handler is declared");
    assert_eq!(route.routes, vec!["GET /items/{id}".to_owned()]);
    assert!(!route.test, "a handler is no test");
    assert!(
        index
            .declaration("ShopTest::addsTax")
            .is_some_and(|declared| declared.test),
        "a #[Test] method is marked"
    );

    let sent = index.requests("Shop::show");
    assert_eq!(
        sent.iter()
            .map(|request| request.caller.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("ShopTest::showsAnItem")],
        "the literal path matched the route, and the request is an edge to its handler"
    );
}

/// A route handler and a test that call the method themselves reach it.
#[test]
fn a_direct_caller_reaches_the_method() {
    let dir = TempDir::new("direct");
    let index = shop_index(&dir);
    assert_eq!(
        reach(&index, "Shop::price"),
        reached(&["GET /items/{id}"], &["ShopTest::showsAnItem"], false),
        "show calls price, and the test requests show's path"
    );
}

/// A route reaches the method through a chain of calls, and a test reaches it
/// directly. The count is static, read off the call graph alone.
#[test]
fn a_chain_of_calls_reaches_the_method() {
    let dir = TempDir::new("transitive");
    let index = shop_index(&dir);
    assert_eq!(
        reach(&index, "Shop::tax"),
        reached(
            &["GET /items/{id}", "GET /total"],
            &["ShopTest::addsTax", "ShopTest::showsAnItem"],
            false
        ),
    );
}

/// A method that calls itself is visited once, so the walk ends.
#[test]
fn a_recursive_chain_ends() {
    let dir = TempDir::new("recursive");
    let index = shop_index(&dir);
    assert_eq!(
        reach(&index, "Shop::sum"),
        reached(&["GET /total"], &[], false)
    );
}

/// A method taken as a callable value is reached by nothing the graph
/// follows, and the answer says the count is at least that.
#[test]
fn a_callable_value_makes_the_count_at_least() {
    let dir = TempDir::new("at-least");
    let index = shop_index(&dir);
    assert_eq!(reach(&index, "Shop::untouched"), reached(&[], &[], true));
}
