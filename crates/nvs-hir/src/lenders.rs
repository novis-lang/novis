//! Which program lends its `autoload` map to a file that is not where a
//! program starts (`rule:ide/an-autoloaded-file-borrows-its-programs-map`).
//!
//! A class file a program autoloads may not declare `autoload` itself
//! (`rule:programs/autoload`), and a file a program requires usually does not.
//! Walked as its own entry point, either resolves none of the names its program
//! resolves. A **lender** is a program whose `require` chain declares
//! `autoload`, and [`Lender::lends_to`] is the one test of which files it
//! lends its map to. [`resolve_program_borrowing`](crate::resolve_program_borrowing)
//! and [`resolve_program_linted`](crate::resolve_program_linted) take its
//! [`sites`](Lender::sites) as the borrowed declarations.
//!
//! Two callers survey a tree for lenders, and both go through this module so
//! they agree on what a lender is. `nvs-lsp` walks every file under the
//! workspace once and keeps the lenders for the session. `nvs check` walks the
//! files under the project root in path order with [`first_lender`] and stops
//! at the first lender that owns the checked file. It does that only when the
//! checked file declares no map of its own and names something no file
//! declares that a map could declare ([`a_map_could_declare`]). Both pick through [`choose`].

use std::borrow::Borrow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ignore::WalkBuilder;
use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceId, SourceMap, canonical_key};
use nvs_syntax::ast::Stmt;
use nvs_syntax::{Keyword, Lexer, TokenKind, check_declarations, parse_file};
use rustc_hash::FxHashMap;

use crate::autoload::{AutoloadMap, Site};
use crate::hierarchy::CoreRoster;
use crate::requires::{Parses, resolve_program_borrowing};

/// What one search for lenders shares across the programs it walks: one
/// source map that reads each file once, and one parse of each file.
///
/// Programs in one tree mostly autoload one shared library. Walked each in a
/// new map, every program read and parsed that library again: a search over
/// 115 test programs read 9,067 files. Here each file is read and parsed once
/// per search, and its path resolved once. It costs the text, the statements
/// and the resolved path of every file the search read, held until the search
/// ends. The editor's survey and `nvs check`'s
/// search each make one and drop it when they finish.
#[derive(Debug)]
pub struct Survey {
    map: SourceMap,
    /// Each file's statements and the diagnostics its parse reported, by id.
    /// A file keeps its id for the whole search ([`SourceMap::loading_once`]).
    parsed: FxHashMap<SourceId, (Arc<Vec<Stmt>>, Vec<Diagnostic>)>,
    /// Each file's path under `canonical_key`, by id, resolved the first time a
    /// lender names the file.
    keys: FxHashMap<SourceId, Option<PathBuf>>,
}

impl Survey {
    /// A search that reads every file from disk.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: SourceMap::loading_once(),
            parsed: FxHashMap::default(),
            keys: FxHashMap::default(),
        }
    }

    /// The map the walks read through. An editor overlays its open buffers on
    /// it before the first walk.
    pub fn map_mut(&mut self) -> &mut SourceMap {
        &mut self.map
    }
}

impl Default for Survey {
    fn default() -> Self {
        Self::new()
    }
}

/// The parses of one [`Survey`], apart from its map so a walk can borrow both.
struct Kept<'a>(&'a mut FxHashMap<SourceId, (Arc<Vec<Stmt>>, Vec<Diagnostic>)>);

impl Parses for Kept<'_> {
    fn parse(&mut self, src: &SourceFile, diags: &mut Diagnostics) -> Arc<Vec<Stmt>> {
        let (stmts, reported) = self.0.entry(src.id()).or_insert_with(|| {
            let mut own = Diagnostics::new();
            let stmts = Arc::new(parse_file(src, &mut own));
            check_declarations(&stmts, src, &mut own);
            (stmts, own.iter().cloned().collect())
        });
        for diagnostic in reported.iter() {
            diags.report(diagnostic.clone());
        }
        Arc::clone(stmts)
    }
}

/// One program that declares `autoload`, as the files it autoloads or requires
/// borrow it.
#[derive(Debug)]
pub struct Lender {
    /// The file the walk started from, under `canonical_key`.
    entry: PathBuf,
    /// Every file of its `require` chain that wrote a declaration, under
    /// `canonical_key`. An edit to one of them makes this lender stale.
    declaring: Vec<PathBuf>,
    /// Every other file its `require` chain read, under `canonical_key`. These
    /// files borrow this map without lying under a root.
    reads: Vec<PathBuf>,
    /// The directory of its entry, under `canonical_key`. A plain file under
    /// it, one whose code writes neither `require` nor `autoload`
    /// ([`is_plain`]), borrows this map too.
    /// That is how a directory `nvs test` runs as one program is analysed the
    /// way it runs.
    dir: PathBuf,
    /// The map those declarations built. It says what the program claims and
    /// carries the sites it lends.
    map: AutoloadMap,
}

impl Lender {
    /// `path` as a lender, or `None` for a file whose program declares no
    /// `autoload`.
    ///
    /// `survey` is what the walks of one search share ([`Survey`]): every file
    /// is read and parsed once per search, however many programs reach it. The
    /// file is asked [`is_plain`] before anything is parsed, so a
    /// tree pays one read per file and one walk per file that starts a chain. A
    /// program whose declaration sits in a file it requires holds neither
    /// declaration itself, and only the walk finds it. The walk is name
    /// resolution without the type phase, and it reads every file the program
    /// reaches. Its diagnostics are dropped: the program's own check is where
    /// they are reported.
    #[must_use]
    pub fn walk(survey: &mut Survey, path: &Path) -> Option<Self> {
        let map = &mut survey.map;
        let entry = map.load(path).ok()?;
        if is_plain(map.file(entry).text()) {
            return None;
        }

        nvs_diagnostics::work::count(nvs_diagnostics::work::Kind::Programs);
        let mut diags = Diagnostics::new();
        let mut kept = Kept(&mut survey.parsed);
        let stmts = kept.parse(map.file(entry), &mut diags);
        let (_, loaded, autoload) = resolve_program_borrowing(
            entry,
            stmts,
            map,
            CoreRoster::Trusted,
            &mut diags,
            &[],
            Some(&mut kept),
        );
        if autoload.sites().is_empty() {
            return None;
        }

        let keys = &mut survey.keys;
        let mut key_of = |id: SourceId| {
            keys.entry(id)
                .or_insert_with(|| map.file(id).path().map(canonical_key))
                .clone()
        };
        let mut declaring: Vec<PathBuf> = autoload
            .sites()
            .iter()
            .filter_map(|site| key_of(site.span.file))
            .collect();
        declaring.sort();
        declaring.dedup();
        let mut reads: Vec<PathBuf> = loaded
            .iter()
            .skip(1)
            .filter_map(|file| key_of(file.id))
            .collect();
        reads.sort();
        reads.dedup();
        let key = canonical_key(path);
        let dir = key.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        Some(Self {
            entry: key,
            declaring,
            reads,
            dir,
            map: autoload,
        })
    }

    /// The file the program starts from, under `canonical_key`.
    #[must_use]
    pub fn entry(&self) -> &Path {
        &self.entry
    }

    /// Every file of the program's `require` chain that wrote a declaration,
    /// under `canonical_key`, sorted.
    #[must_use]
    pub fn declaring(&self) -> &[PathBuf] {
        &self.declaring
    }

    /// Every other file the program's `require` chain read, under
    /// `canonical_key`, sorted.
    #[must_use]
    pub fn reads(&self) -> &[PathBuf] {
        &self.reads
    }

    /// The declarations a borrowing walk places behind its own.
    #[must_use]
    pub fn sites(&self) -> &[Site] {
        self.map.sites()
    }

    /// The map the program's declarations built.
    #[must_use]
    pub fn map(&self) -> &AutoloadMap {
        &self.map
    }

    /// Whether this program lends its map to `path`: its walk read the file,
    /// the file is under a root one of its own declarations names, or the file
    /// is under the directory of its entry and is plain.
    ///
    /// `plain` says whether the code of `path` writes neither `require` nor
    /// `autoload` ([`is_plain`]). It is asked only when the directory test
    /// needs it, so a caller can read the text lazily. Whether `path` is itself
    /// a lender is the caller's question: a lender never borrows.
    pub fn lends_to(&self, path: &Path, plain: impl FnOnce() -> bool) -> bool {
        self.lends_to_at(&At::of(path), plain)
    }

    /// [`lends_to`](Self::lends_to) for a path [`At::of`] already resolved.
    fn lends_to_at(&self, at: &At, plain: impl FnOnce() -> bool) -> bool {
        self.owns_at(at) || (at.key.starts_with(&self.dir) && plain())
    }

    /// Whether `path` belongs to this program by name rather than by place:
    /// its walk read the file, or one of its own declarations names a root
    /// that holds it. A lender that owns a file outranks one that lends to it
    /// only by the directory test ([`choose`]).
    #[must_use]
    pub fn owns(&self, path: &Path) -> bool {
        self.owns_at(&At::of(path))
    }

    /// [`owns`](Self::owns) for a path [`At::of`] already resolved.
    fn owns_at(&self, at: &At) -> bool {
        self.reads.binary_search(&at.key).is_ok() || self.map.claims_resolved(&at.claimed)
    }
}

/// One path, resolved once for every lender [`choose`] asks about it.
///
/// Resolving a path is a system call, and on Windows it opens the file. A
/// tree with many lenders asked about each of many files paid it once per
/// lender, several times over.
struct At {
    /// The path under `canonical_key`, as `reads` and `dir` hold paths.
    key: PathBuf,
    /// The path as [`AutoloadMap::claims`] compares it with its roots.
    claimed: PathBuf,
}

impl At {
    /// Outside a bundled executable the two forms are one system call's
    /// answer, so the path is resolved once. A bundle resolves against its
    /// payload, and there the claim is resolved on its own.
    fn of(path: &Path) -> Self {
        let key = canonical_key(path);
        let claimed = if nvs_diagnostics::embedded::is_active() {
            crate::autoload::resolved(path)
        } else {
            key.clone()
        };
        Self { key, claimed }
    }
}

/// The lender among `lenders`, taken in entry-path order, that lends `path`
/// its map: the first that [owns](Lender::owns) the file, else the nearest that
/// lends to it by the directory test.
///
/// The directory test is the weakest evidence. A program at a project's root
/// passes it for every plain file in the tree, and its map need not name the
/// file's namespace at all, while a program whose declaration names the file's
/// root is the one the file is built through. Among the programs that pass the
/// directory test, the one whose entry sits deepest lends: every such directory
/// holds `path`, so the deepest is the nearest, and a test directory's
/// bootstrap is the program `nvs test` runs its plain files with, where a
/// program at the root never runs them. Two in one directory keep entry-path
/// order. `plain` is [`is_plain`] for the text of `path`, asked at most once
/// and only when the directory test needs it. The search stops at the first
/// owner, so a caller that walks each lender lazily walks none past it.
#[must_use]
pub fn choose<L: Borrow<Lender>>(
    lenders: impl IntoIterator<Item = L>,
    path: &Path,
    plain: impl FnOnce() -> bool,
) -> Option<L> {
    let at = At::of(path);
    let mut plain = Some(plain);
    let mut is_plain = None;
    let mut by_place = None;
    for lender in lenders {
        if lender.borrow().owns_at(&at) {
            return Some(lender);
        }
        let nearer = by_place.as_ref().is_none_or(|held: &L| {
            lender.borrow().dir.components().count() > held.borrow().dir.components().count()
        });
        if nearer
            && lender.borrow().lends_to_at(&at, || {
                *is_plain.get_or_insert_with(|| plain.take().is_some_and(|plain| plain()))
            })
        {
            by_place = Some(lender);
        }
    }
    by_place
}

/// Whether `text` writes neither a `require` nor an `autoload` as code, so the
/// file starts no program of its own.
///
/// The lexer's tokens decide it, so the word in a comment or a string never
/// counts, and neither does a member of that name (`->require`, `::autoload`,
/// `function autoload`). A `require` anywhere in the code counts, as every one
/// is a file the program's walk reads. A text without either word is plain
/// without being lexed, so most files cost one search; the rest cost one copy
/// of the text and one lex, with the lexer's diagnostics dropped.
#[must_use]
pub fn is_plain(text: &str) -> bool {
    if !text.contains("require") && !text.contains("autoload") {
        return true;
    }
    let mut map = SourceMap::new();
    let id = map.add("", text);
    let mut diags = Diagnostics::new();
    let mut lexer = Lexer::new(map.file(id));
    let mut after_member = false;
    loop {
        let kind = lexer.next_token(&mut diags).kind;
        match kind {
            TokenKind::Eof => return true,
            TokenKind::Keyword(Keyword::Require | Keyword::RequireOnce | Keyword::Autoload)
                if !after_member =>
            {
                return false;
            }
            _ => {}
        }
        after_member = matches!(
            kind,
            TokenKind::Arrow
                | TokenKind::NullsafeArrow
                | TokenKind::DoubleColon
                | TokenKind::Keyword(Keyword::Function)
        );
    }
}

/// Whether `diagnostic` says a type is not declared (`E0303`) or a `use` does
/// not resolve (`E0306`), for a name a borrowed map could declare.
///
/// A prefix is at least one namespace segment and matching appends a `\`, so
/// no map declares a name with one segment, such as `Countable`. No map
/// declares a name under `Core` either. Neither starts a search for a lender,
/// and neither gets a note about one, the same two names
/// [`AutoloadMap::note_searched_roots`] passes over.
#[must_use]
pub fn a_map_could_declare(diagnostic: &nvs_diagnostics::Diagnostic) -> bool {
    crate::hierarchy::undeclared_subject(diagnostic)
        .or_else(|| crate::resolve::unresolved_import_subject(diagnostic))
        .is_some_and(|name| !name.is_core() && name.segments().len() >= 2)
}

/// The lender among the `.nvs` files under `root` that lends its map to
/// `path`, as [`choose`] picks it in entry-path order.
///
/// The files are walked one by one and the search stops at the first lender
/// that owns `path`, so the cost is one read per file and one walk per program
/// up to that lender. When no lender owns it, every program under `root` is
/// walked before the nearest that lends by the directory test is returned.
/// The walks share one [`Survey`], so a library many programs reach is read and
/// parsed once. `plain` is [`is_plain`] for the text of `path`. `path` itself
/// is skipped: the caller asks only for a file that declares no map of its own.
///
/// The paths are sorted as the walk found them, all under `root`, which is the
/// order of their resolved paths too, because the walk follows no link. A
/// path is resolved only when its program is walked, and to tell `path` apart
/// only when its file name is the same.
#[must_use]
pub fn first_lender(root: &Path, path: &Path, plain: bool) -> Option<Lender> {
    let mut key = None;
    let mut found = sources(root);
    found.sort();
    let mut survey = Survey::new();
    choose(
        found
            .into_iter()
            .filter(|source| {
                source.file_name() != path.file_name()
                    || canonical_key(source) != *key.get_or_insert_with(|| canonical_key(path))
            })
            .filter_map(|source| Lender::walk(&mut survey, &source)),
        path,
        || plain,
    )
}

/// The folder [`first_lender`] searches for `path`: the nearest folder that
/// holds an `nvs.toml` or a `package.toml`, at or above `path` and at or below
/// `root`, else `root`.
///
/// A file's project is the folder of its nearest `nvs.toml` or `package.toml`,
/// and the program that loads the file is in that project. Searching from a
/// folder above it walked every other project too: a class file checked from
/// the root of a checkout with many applications listed 4,550 folders, where
/// its own project has 5. A `path` outside `root` searches `root`, as before.
/// It costs one existence test per marker per folder between `path` and
/// `root`, and resolves no path.
#[must_use]
pub fn project_root(root: &Path, path: &Path) -> PathBuf {
    let (Ok(top), Ok(file)) = (std::path::absolute(root), std::path::absolute(path)) else {
        return root.to_path_buf();
    };
    file.ancestors()
        .skip(1)
        .take_while(|dir| dir.starts_with(&top))
        .find(|dir| dir.join("nvs.toml").is_file() || dir.join("package.toml").is_file())
        .map_or_else(|| root.to_path_buf(), Path::to_path_buf)
}

/// Every `.nvs` file under `root`, recursively, in no particular order.
///
/// This is the one project walk: `nvs code`, the editor's workspace index,
/// `nvs check`'s lender search and `nvs fmt` all read the files it returns
/// (`rule:tooling/nvs-code-answers-about-the-project`). It goes through the
/// `ignore` crate's [`WalkBuilder`], and:
///
/// * a `.gitignore` or `.ignore` in `root` or any directory under it is read,
///   also outside a git repository, and a path one of them names is skipped;
/// * the global git excludes file, `.git/info/exclude` and an ignore file
///   above `root` are not read, so one tree gives one list on every machine;
/// * a directory named `target` or `vendor` and a dot-directory are not
///   entered, because they hold no source anybody here writes, while a
///   dot-file is still read;
/// * `root` itself is always walked, whatever it is called and whatever an
///   ignore file says, because naming it is the caller's choice;
/// * a symbolic link is not followed, and a directory this process cannot
///   read is skipped: a tree with one unreadable directory in it is still a
///   tree.
///
/// Cost: each ignore file is read once per walk, and its parsed patterns, a
/// few KiB, live until the walk ends. Nothing here runs on the request path.
#[must_use]
pub fn sources(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .ignore(true)
        .git_ignore(true)
        .require_git(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false)
        .filter_entry(|entry| {
            let dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            let name = entry.file_name().to_str();
            !(dir
                && name.is_some_and(|name| {
                    name == "target" || name == "vendor" || name.starts_with('.')
                }))
        })
        .build()
        .filter_map(Result::ok)
        .inspect(|entry| {
            if entry.file_type().is_some_and(|kind| kind.is_dir()) {
                nvs_diagnostics::work::count(nvs_diagnostics::work::Kind::Listed);
            }
        })
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.extension().is_some_and(|ext| ext == "nvs"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A scratch tree with `files` written into it, each `(path, contents)`.
    fn tree(name: &str, files: &[(&str, &str)]) -> nvs_repo::Scratch {
        let dir = nvs_repo::scratch(&format!("hir-sources-{name}"));
        for (rel, contents) in files {
            let path = dir.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create fixture directory");
            }
            fs::write(&path, contents).expect("write fixture");
        }
        dir
    }

    /// What [`sources`] found under `root`, relative to it, with `/`, sorted.
    fn found(root: &Path) -> Vec<String> {
        let mut found: Vec<String> = sources(root)
            .iter()
            .map(|path| {
                path.strip_prefix(root)
                    .expect("a source under the root")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        found.sort();
        found
    }

    /// Asking which lender a file belongs to resolves its path once, for the
    /// lenders' reads and their maps' roots alike (`rule:testing/bench-counters`).
    /// It resolved the path twice, once for each.
    #[test]
    fn a_path_asked_about_is_resolved_once() {
        let dir = tree("at-once", &[("src/Thing.nvs", "<?nvs\n")]);
        let before = nvs_diagnostics::work::so_far();
        let at = At::of(&dir.join("src/Thing.nvs"));
        let after = nvs_diagnostics::work::so_far();
        assert_eq!(after.resolved - before.resolved, 1);
        assert_eq!(at.key, at.claimed);
    }

    #[test]
    fn a_require_in_a_comment_or_a_string_leaves_a_file_plain() {
        assert!(is_plain(
            "<?nvs\n// This test needs the bootstrap to require.\n"
        ));
        assert!(is_plain("<?nvs\n/* autoload */\necho \"require me\";\n"));
        assert!(is_plain("<?nvs\n$loader->autoload();\nFoo::require();\n"));
        assert!(is_plain("plain html that says require and autoload\n"));
    }

    #[test]
    fn a_require_or_an_autoload_written_as_code_makes_a_file_a_program() {
        assert!(!is_plain("<?nvs\nrequire __DIR__ . '/bootstrap.nvs';\n"));
        assert!(!is_plain("<?nvs\nautoload 'App\\\\' => 'src';\n"));
        assert!(!is_plain("<?nvs\n// require\nrequire 'a.nvs';\n"));
    }

    #[test]
    fn a_file_a_gitignore_names_is_skipped_outside_a_git_repository() {
        let dir = tree(
            "gitignore",
            &[
                (".gitignore", "generated.nvs\nbuild/\n"),
                ("main.nvs", ""),
                ("generated.nvs", ""),
                ("build/out.nvs", ""),
            ],
        );
        assert_eq!(found(&dir), ["main.nvs"]);
    }

    #[test]
    fn a_file_an_ignore_file_names_is_skipped() {
        let dir = tree(
            "ignore",
            &[
                (".ignore", "scratch.nvs\n"),
                ("main.nvs", ""),
                ("scratch.nvs", ""),
            ],
        );
        assert_eq!(found(&dir), ["main.nvs"]);
    }

    #[test]
    fn a_nested_gitignore_applies_under_its_own_directory() {
        let dir = tree(
            "nested",
            &[
                ("src/.gitignore", "cache.nvs\n"),
                ("src/cache.nvs", ""),
                ("src/main.nvs", ""),
                ("cache.nvs", ""),
            ],
        );
        assert_eq!(found(&dir), ["cache.nvs", "src/main.nvs"]);
    }

    #[test]
    fn vendor_target_and_a_dot_directory_are_not_entered_and_a_dot_file_is_read() {
        let dir = tree(
            "skipped",
            &[
                ("vendor/lib.nvs", ""),
                ("target/out.nvs", ""),
                (".cache/x.nvs", ""),
                (".hidden.nvs", ""),
                ("main.nvs", ""),
                ("notes.txt", ""),
            ],
        );
        assert_eq!(found(&dir), [".hidden.nvs", "main.nvs"]);
    }

    #[test]
    fn the_root_is_walked_whatever_it_is_called_or_an_ignore_file_says() {
        let dir = tree(
            "root",
            &[
                (".gitignore", "vendor/\n"),
                ("vendor/lib.nvs", ""),
                ("vendor/sub/deep.nvs", ""),
            ],
        );
        let vendor = dir.join("vendor");
        assert_eq!(found(&vendor), ["lib.nvs", "sub/deep.nvs"]);
        assert!(found(&dir).is_empty());
    }
}
