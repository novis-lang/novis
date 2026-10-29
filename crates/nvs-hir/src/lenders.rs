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

use ignore::WalkBuilder;
use nvs_diagnostics::{Diagnostics, SourceMap, canonical_key};
use nvs_syntax::{Keyword, Lexer, TokenKind, parse_file};

use crate::autoload::{AutoloadMap, Site};
use crate::hierarchy::CoreRoster;
use crate::requires::resolve_program;

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
    /// `map` is the source map the walk reads through: empty for files on
    /// disk, or holding the open buffers an editor overlays on them. The file
    /// is asked [`is_plain`] before anything is parsed, so a
    /// tree pays one read per file and one walk per file that starts a chain. A
    /// program whose declaration sits in a file it requires holds neither
    /// declaration itself, and only the walk finds it. The walk is name
    /// resolution without the type phase, and it reads every file the program
    /// reaches. Its diagnostics are dropped: the program's own check is where
    /// they are reported.
    #[must_use]
    pub fn walk(mut map: SourceMap, path: &Path) -> Option<Self> {
        let entry = map.load(path).ok()?;
        if is_plain(map.file(entry).text()) {
            return None;
        }

        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(entry), &mut diags);
        let (_, loaded, autoload) =
            resolve_program(entry, stmts, &mut map, CoreRoster::Trusted, &mut diags);
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
        let mut reads: Vec<PathBuf> = loaded
            .iter()
            .skip(1)
            .filter_map(|file| map.file(file.id).path().map(canonical_key))
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
        self.owns(path) || (canonical_key(path).starts_with(&self.dir) && plain())
    }

    /// Whether `path` belongs to this program by name rather than by place:
    /// its walk read the file, or one of its own declarations names a root
    /// that holds it. A lender that owns a file outranks one that lends to it
    /// only by the directory test ([`choose`]).
    #[must_use]
    pub fn owns(&self, path: &Path) -> bool {
        self.reads.binary_search(&canonical_key(path)).is_ok() || self.map.claims(path)
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
    let mut plain = Some(plain);
    let mut is_plain = None;
    let mut by_place = None;
    for lender in lenders {
        if lender.borrow().owns(path) {
            return Some(lender);
        }
        let nearer = by_place.as_ref().is_none_or(|held: &L| {
            lender.borrow().dir.components().count() > held.borrow().dir.components().count()
        });
        if nearer
            && lender.borrow().lends_to(path, || {
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
/// `plain` is [`is_plain`] for the text of `path`. `path` itself is skipped:
/// the caller asks only for a file that declares no map of its own.
#[must_use]
pub fn first_lender(root: &Path, path: &Path, plain: bool) -> Option<Lender> {
    let key = canonical_key(path);
    let mut found: Vec<PathBuf> = sources(root)
        .iter()
        .map(|source| canonical_key(source))
        .filter(|source| *source != key)
        .collect();
    found.sort();
    found.dedup();
    choose(
        found
            .into_iter()
            .filter_map(|source| Lender::walk(SourceMap::new(), &source)),
        path,
        || plain,
    )
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
