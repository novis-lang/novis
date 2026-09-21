//! A file a program autoloads is analysed through that program's `autoload` map.
//!
//! `rule:ide/an-open-document-is-its-own-entry-point` makes the open document
//! the entry of its own walk, and `rule:programs/autoload` forbids an
//! autoloaded file an `autoload` of its own. Put together, a class file opened
//! in an editor has no map to resolve `use Framework\Kernel;` through, and what
//! these cases hold is that it borrows the one its program declares: which
//! program that is, that the borrowed half never reports anything, and that an
//! edit to the declaring file reaches the documents that borrowed from it.
//!
//! The fixtures are on disk, on `tests/index.rs`'s terms: a root is a directory
//! and a survey walks one. The framework sits *outside* the workspace root on
//! purpose, because that is where a shared framework checkout is.

use std::fs;
use std::path::PathBuf;

use lsp_types::Uri;
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::{Analysed, CheckScope, Documents, analyse, uri_of};

/// A scratch directory that cleans up after itself.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("nvs-lsp-autoload-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a scratch directory");
        Self { path }
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.path.join(name);
        fs::create_dir_all(path.parent().expect("a fixture sits in a directory"))
            .expect("a scratch directory");
        fs::write(path, text).expect("a writable scratch file");
    }

    fn at(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    fn uri(&self, name: &str) -> Uri {
        uri_of(&self.at(name)).expect("a temp path is UTF-8")
    }

    /// Opens `name` as a buffer holding `text`.
    fn open(&self, documents: &mut Documents, name: &str, text: &str) {
        documents.open(self.uri(name), 1, text.to_owned());
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The program's entry point: the only file that may declare `autoload`.
const BOOT: &str = "<?nvs\nautoload 'Framework' from '../../Framework/src';\n\
                    autoload 'Blog' from '../src';\n\nuse Blog\\Index;\n\nnew Index();\n";

/// A class the program autoloads, naming a framework class it autoloads too.
const INDEX: &str = "<?nvs\nnamespace Blog;\n\nuse Framework\\Kernel;\n\nclass Index {\n    \
                     public function constructor() {\n        new Kernel();\n    }\n}\n";

/// The framework class, under a root outside the workspace.
const KERNEL: &str = "<?nvs\nnamespace Framework;\n\nclass Kernel {\n    \
                      public function constructor() {}\n}\n";

/// The application as a developer has it on disk, and the workspace root an
/// editor would name for it.
fn application(name: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new(name);
    dir.write("app/public/index.nvs", BOOT);
    dir.write("app/src/Index.nvs", INDEX);
    dir.write("Framework/src/Kernel.nvs", KERNEL);
    let root = dir.at("app");
    (dir, root)
}

/// Every diagnostic code `analysed` reported, in report order.
fn codes(analysed: &Analysed) -> Vec<&'static str> {
    analysed
        .diags
        .iter()
        .filter_map(|diagnostic| diagnostic.code.map(|code| code.as_str()))
        .collect()
}

/// The names of the files `analysed` depends on, as `dir/file`.
fn tails(analysed: &Analysed) -> Vec<String> {
    analysed
        .files()
        .map(|path| {
            let mut parts: Vec<String> = path
                .components()
                .rev()
                .take(2)
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect();
            parts.reverse();
            parts.join("/")
        })
        .collect()
}

/// The premise: with no survey behind it, the open class file is a program of
/// one file, and the name its real program resolves is unresolved.
#[test]
fn an_autoloaded_file_analysed_alone_cannot_resolve_its_imports() {
    let (dir, _root) = application("alone");
    let mut documents = Documents::new();
    dir.open(&mut documents, "app/src/Index.nvs", INDEX);

    let analysed = analyse(&documents, &dir.uri("app/src/Index.nvs")).expect("it is open");
    assert!(
        codes(&analysed).contains(&"E0306"),
        "the fixture no longer shows the gap the survey closes: {:?}",
        codes(&analysed),
    );
}

/// The case the rule was written for: the same document, after a survey of the
/// workspace, resolves the framework class through the program's map — and the
/// walk read the framework file to do it.
#[test]
fn an_autoloaded_file_borrows_the_map_of_the_program_that_autoloads_it() {
    let (dir, root) = application("borrows");
    let mut documents = Documents::new();
    documents.survey(CheckScope::Workspace, Some(&root));
    dir.open(&mut documents, "app/src/Index.nvs", INDEX);

    let analysed = analyse(&documents, &dir.uri("app/src/Index.nvs")).expect("it is open");
    assert_eq!(codes(&analysed), Vec::<&str>::new());
    let read = tails(&analysed);
    assert!(read.contains(&"src/Kernel.nvs".to_owned()), "{read:?}");
    // The declaring file is a dependency without having been read by the walk.
    assert!(read.contains(&"public/index.nvs".to_owned()), "{read:?}");
}

/// What the borrowed map is for, from the cursor's side: a jump from the
/// `use` line of the class file lands in the framework file the map found.
#[test]
fn a_use_line_in_an_autoloaded_file_jumps_to_the_file_it_imports() {
    let (dir, root) = application("jump");
    let mut documents = Documents::new();
    documents.survey(CheckScope::Workspace, Some(&root));
    dir.open(&mut documents, "app/src/Index.nvs", INDEX);

    let analysed = analyse(&documents, &dir.uri("app/src/Index.nvs")).expect("it is open");
    let on_kernel = INDEX.find("Kernel;").expect("the import is written") + 3;
    let offset = u32::try_from(on_kernel).expect("a test document is short");
    let declared = nvs_lsp::definition::at(&analysed, offset, PositionEncoding::Utf8)
        .expect("the import resolved through the borrowed map");
    assert!(
        declared.path.ends_with("Framework/src/Kernel.nvs"),
        "{}",
        declared.path.display(),
    );
    assert_eq!(
        (declared.range.start.line, declared.range.start.character),
        (3, 6)
    );
}

/// The entry point lends and never borrows: it is analysed exactly as
/// `nvs check` analyses it, survey or no survey.
#[test]
fn the_declaring_file_is_analysed_as_it_always_was() {
    let (dir, root) = application("declaring");
    let mut documents = Documents::new();
    documents.survey(CheckScope::Workspace, Some(&root));
    dir.open(&mut documents, "app/public/index.nvs", BOOT);

    let analysed = analyse(&documents, &dir.uri("app/public/index.nvs")).expect("it is open");
    assert_eq!(codes(&analysed), Vec::<&str>::new());
    assert!(analysed.lent.is_empty());
}

/// Under `open` scope nothing under the root is read, so the map is lent only
/// once the declaring file is a document too.
#[test]
fn open_scope_lends_from_the_open_documents_alone() {
    let (dir, root) = application("open-scope");
    let mut documents = Documents::new();
    documents.survey(CheckScope::Open, Some(&root));
    dir.open(&mut documents, "app/src/Index.nvs", INDEX);
    documents.resurvey(&dir.at("app/src/Index.nvs"));

    let uri = dir.uri("app/src/Index.nvs");
    let alone = analyse(&documents, &uri).expect("it is open");
    assert!(codes(&alone).contains(&"E0306"), "{:?}", codes(&alone));

    dir.open(&mut documents, "app/public/index.nvs", BOOT);
    documents.resurvey(&dir.at("app/public/index.nvs"));
    let lent = analyse(&documents, &uri).expect("it is open");
    assert_eq!(codes(&lent), Vec::<&str>::new());
}

/// An edit to the declaring file reaches the document that borrowed from it:
/// it is on the republish list, and its next analysis reads the edited map.
#[test]
fn editing_the_declaring_file_reaches_the_documents_that_borrowed_from_it() {
    let (dir, root) = application("edit");
    let mut documents = Documents::new();
    documents.survey(CheckScope::Workspace, Some(&root));
    dir.open(&mut documents, "app/src/Index.nvs", INDEX);
    dir.open(&mut documents, "app/public/index.nvs", BOOT);

    let uri = dir.uri("app/src/Index.nvs");
    let before = analyse(&documents, &uri).expect("it is open");
    assert_eq!(codes(&before), Vec::<&str>::new());
    documents.record_graph(&uri, before.files());

    // The framework's declaration goes, in the buffer and not on disk.
    let edited = BOOT.replace("autoload 'Framework' from '../../Framework/src';\n", "");
    let boot = dir.at("app/public/index.nvs");
    assert!(documents.change(&dir.uri("app/public/index.nvs"), 2, edited));
    documents.resurvey(&boot);

    assert!(documents.to_republish(&boot).contains(&&uri));
    let after = analyse(&documents, &uri).expect("it is open");
    assert!(codes(&after).contains(&"E0306"), "{:?}", codes(&after));
}

/// Two programs over one source tree: the first in entry-path order lends, so
/// the answer does not depend on which was found first.
#[test]
fn the_first_program_in_path_order_is_the_one_that_lends() {
    let dir = TempDir::new("two-programs");
    let boot = |lib: &str| {
        format!("<?nvs\nautoload 'App' from '../src';\nautoload 'Lib' from '../{lib}';\n")
    };
    dir.write("app/a/boot.nvs", &boot("lib-a"));
    dir.write("app/b/boot.nvs", &boot("lib-b"));
    let thing = "<?nvs\nnamespace Lib;\n\nclass Thing {}\n";
    dir.write("app/lib-a/Thing.nvs", thing);
    dir.write("app/lib-b/Thing.nvs", thing);
    let page = "<?nvs\nnamespace App;\n\nuse Lib\\Thing;\n\nclass Page {\n    \
                public function make(): Thing {\n        return new Thing();\n    }\n}\n";
    dir.write("app/src/Page.nvs", page);

    let mut documents = Documents::new();
    documents.survey(CheckScope::Workspace, Some(&dir.at("app")));
    dir.open(&mut documents, "app/src/Page.nvs", page);

    let analysed = analyse(&documents, &dir.uri("app/src/Page.nvs")).expect("it is open");
    assert_eq!(codes(&analysed), Vec::<&str>::new());
    let read = tails(&analysed);
    assert!(read.contains(&"lib-a/Thing.nvs".to_owned()), "{read:?}");
    assert!(!read.contains(&"lib-b/Thing.nvs".to_owned()), "{read:?}");
}
