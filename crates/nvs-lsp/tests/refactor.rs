//! The check every refactoring's edit passes before it is written
//! (`rule:ide/a-refactoring-checks-its-edit-before-it-writes`).
//!
//! `nvs_lsp::refactor::checked` applies rewrites in memory and analyses each
//! entry point that reads a changed file, before the edit and after it. These
//! tests hold it to its three answers: an added error refuses the edit and
//! names that error, an edit that adds none comes back unchanged, and a
//! warning never refuses it. The last test counts the entries it analysed.
//!
//! The project is on disk, because a workspace is a directory walked for
//! `.nvs` files and an autoload root is a directory too.

use std::fs;
use std::path::PathBuf;

use nvs_diagnostics::canonical_key;
use nvs_lsp::refactor::{self, Broken};
use nvs_lsp::rename::Rewrite;
use nvs_lsp::{Site, Workspace, workspace};

/// The program's entry point: it autoloads `Shop` and calls `Cart::addItem`
/// on line 6.
const INDEX: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Cart;\n\n\
                     var $total = (new Cart())->addItem(3);\necho $total;\n";

/// The class the edits change.
const CART: &str = "<?nvs\nnamespace Shop;\n\nclass Cart {\n    public int $total = 0;\n\n    \
                    public function addItem(int $n): int {\n        \
                    $this->total = $this->total + $n;\n        return $this->total;\n    }\n}\n";

/// A class of the same namespace that does not read `Cart`.
const WISHLIST: &str = "<?nvs\nnamespace Shop;\n\nclass Wishlist {\n    \
                        public function count(): int {\n        return 0;\n    }\n}\n";

/// A second program that reads nothing of `Shop`.
const OTHER: &str = "<?nvs\necho 'other';\n";

/// The `Shop` project in a scratch directory, deleted when it is dropped, and
/// the workspace built over it.
struct Project {
    dir: nvs_repo::Scratch,
    workspace: Workspace,
}

impl Project {
    fn new(name: &str) -> Self {
        let dir = nvs_repo::scratch(&format!("lsp-refactor-{name}"));
        for (path, text) in [
            ("public/index.nvs", INDEX),
            ("src/Cart.nvs", CART),
            ("src/Wishlist.nvs", WISHLIST),
            ("other/main.nvs", OTHER),
        ] {
            let at = dir.join(path);
            fs::create_dir_all(at.parent().expect("a file has a directory"))
                .expect("a writable scratch directory");
            fs::write(at, text).expect("a writable scratch file");
        }
        let workspace = workspace(&dir);
        Self { dir, workspace }
    }

    fn key(&self, path: &str) -> PathBuf {
        canonical_key(&self.dir.join(path))
    }

    /// A rewrite of the first `old` in the file at `path` to `new`.
    fn rewrite(&self, path: &str, text: &str, old: &str, new: &str) -> Rewrite {
        let start = text.find(old).expect("the text has the old part");
        let pos = |at: usize| u32::try_from(at).expect("a small file");
        Rewrite {
            site: Site {
                path: self.key(path),
                start: pos(start),
                end: pos(start + old.len()),
            },
            text: new.to_owned(),
        }
    }

    fn checked(&self, rewrites: Vec<Rewrite>) -> Result<refactor::Checked, Broken> {
        refactor::checked(
            self.workspace.documents(),
            self.workspace.index(),
            rewrites,
            &[],
        )
    }
}

#[test]
fn an_edit_that_adds_an_error_is_refused_and_names_the_error() {
    let project = Project::new("adds-an-error");
    let rewrite = project.rewrite("src/Cart.nvs", CART, "addItem", "addItems");

    let broken = project
        .checked(vec![rewrite])
        .expect_err("the call in index.nvs no longer finds its method");

    assert_eq!(broken.site.path, project.key("public/index.nvs"));
    assert_eq!(broken.code.as_deref(), Some("E0405"), "{broken:?}");
    assert_eq!(broken.line, 6, "{broken:?}");
    let refusal = broken.refusal();
    assert!(
        refusal.contains("index.nvs:6:") && refusal.contains("E0405"),
        "{refusal}"
    );
}

#[test]
fn an_edit_that_adds_no_error_is_returned_unchanged() {
    let project = Project::new("adds-no-error");
    let rewrites = vec![
        project.rewrite("src/Cart.nvs", CART, "addItem", "addItems"),
        project.rewrite("public/index.nvs", INDEX, "addItem", "addItems"),
    ];

    let checked = project
        .checked(rewrites.clone())
        .expect("both sites are renamed together");

    assert_eq!(checked.rewrites, rewrites);
    assert!(
        checked.analysed.contains(&project.key("public/index.nvs")),
        "{:?}",
        checked.analysed
    );
}

#[test]
fn a_warning_the_edit_adds_does_not_refuse_it() {
    let project = Project::new("adds-a-warning");
    // A `use` line nothing reads is the warning W1026.
    let rewrite = project.rewrite(
        "public/index.nvs",
        INDEX,
        "use Shop\\Cart;\n",
        "use Shop\\Cart;\nuse Shop\\Wishlist;\n",
    );

    let checked = project
        .checked(vec![rewrite.clone()])
        .expect("an unused import is a warning");

    assert_eq!(checked.rewrites, vec![rewrite]);
}

#[test]
fn the_check_analyses_the_changed_files_and_their_readers_only() {
    let project = Project::new("readers-only");
    let rewrite = project.rewrite(
        "src/Cart.nvs",
        CART,
        "return $this->total;",
        "return $this->total + 0;",
    );

    let checked = project
        .checked(vec![rewrite])
        .expect("the edit adds no error");

    assert!(
        checked.analysed.contains(&project.key("src/Cart.nvs"))
            && checked.analysed.contains(&project.key("public/index.nvs")),
        "the changed file and the program that reads it: {:?}",
        checked.analysed
    );
    for unread in ["src/Wishlist.nvs", "other/main.nvs"] {
        assert!(
            !checked.analysed.contains(&project.key(unread)),
            "{unread} does not read Cart: {:?}",
            checked.analysed
        );
    }
    assert_eq!(checked.analysed.len(), 2, "{:?}", checked.analysed);

    let nothing = project.checked(Vec::new()).expect("an empty edit");
    assert!(nothing.analysed.is_empty(), "{:?}", nothing.analysed);
}
