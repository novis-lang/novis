//! Completion offers only the types the cursor's program can load
//! (`rule:ide/a-bare-name-reaches-every-type-and-imports-the-one-accepted`).
//!
//! A workspace of two modules, `Shop` and `Blog`, each with an entry whose
//! `autoload` map names a shared `Framework` folder and its own `src`. `Shop`
//! also maps a package and reads `Blog` through an `autoload links` line,
//! which loads nothing. One class file lies under no program at all.
//!
//! What these cases hold: a type list in one module leaves out the other
//! module's types, keeps the framework's and the package's, and keeps `Core`;
//! an `autoload` prefix still offers every namespace; references still reach
//! every module; and a file no program loads is offered every type.

use std::path::PathBuf;

use lsp_types::CompletionItem;
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{CheckScope, Client, Documents, SymbolIndex, analyse, completion, uri_of};
use nvs_test::temp::TempDir;

/// The mark a document below writes where its cursor is.
const CURSOR: &str = "<|>";

/// The shared framework's class every module uses.
const KERNEL: &str = "<?nvs\nnamespace Framework;\n\nclass Kernel {\n    \
                      public function constructor() {}\n}\n";

/// A framework class no program uses yet, so no program has loaded it.
const ROUTER: &str = "<?nvs\nnamespace Framework;\n\nclass Router {}\n";

/// The package `Shop` maps and does not use yet.
const MAILER: &str = "<?nvs\nnamespace Acme;\n\nclass Mailer {}\n";

/// `Shop`'s entry: the framework, its own classes, the package, and `Blog`
/// for links only.
const SHOP_ENTRY: &str = "<?nvs\nautoload 'Framework' from '../../Framework/src';\n\
                          autoload 'Shop' from '../src';\n\
                          autoload 'Acme' from '../../packages/acme-mail/src';\n\
                          autoload links 'Blog' from '../../Blog/src';\n\n\
                          use Shop\\Cart;\n\nnew Cart();\n";

/// `Blog`'s entry: the framework and its own classes.
const BLOG_ENTRY: &str = "<?nvs\nautoload 'Framework' from '../../Framework/src';\n\
                          autoload 'Blog' from '../src';\n\n\
                          use Blog\\Post;\n\nnew Post();\n";

/// A `Shop` class no program has loaded yet.
const ORDER: &str = "<?nvs\nnamespace Shop;\n\nclass Order {}\n";

/// A `Blog` class no program has loaded yet.
const COMMENT: &str = "<?nvs\nnamespace Blog;\n\nclass Comment {}\n";

/// A block in `Shop` that adds a method to the framework's class.
const KERNEL_TOOLS: &str = "<?nvs\nnamespace Shop;\n\nuse Framework\\Kernel;\n\n\
                            extend Kernel as KernelTools {\n    \
                            public function tool(): int {\n        return 1;\n    }\n}\n";

/// A block in `Blog` that adds a method to the framework's class.
const KERNEL_NOTES: &str = "<?nvs\nnamespace Blog;\n\nuse Framework\\Kernel;\n\n\
                            extend Kernel as KernelNotes {\n    \
                            public function note(): string {\n        return '';\n    }\n}\n";

/// A class file under no program's entry and no program's root.
const SCRATCH: &str = "<?nvs\nnamespace Loose;\n\nclass Scratch {}\n";

/// A class of `module` that uses the framework's class, with `line` written
/// in its constructor after `$kernel` is made.
fn class_using_kernel(module: &str, class: &str, line: &str) -> String {
    format!(
        "<?nvs\nnamespace {module};\n\nuse Framework\\Kernel;\n\nclass {class} {{\n    \
         public function constructor() {{\n        var $kernel = new Kernel();\n        \
         {line}\n    }}\n}}\n"
    )
}

/// The workspace on disk, and its root.
fn workspace(name: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new(nvs_repo::scratch(&format!("lsp-loadable-{name}")));
    dir.write("Framework/src/Kernel.nvs", KERNEL);
    dir.write("Framework/src/Router.nvs", ROUTER);
    dir.write("packages/acme-mail/src/Mailer.nvs", MAILER);
    dir.write("Shop/public/index.nvs", SHOP_ENTRY);
    dir.write("Shop/src/Cart.nvs", &class_using_kernel("Shop", "Cart", ""));
    dir.write("Shop/src/Order.nvs", ORDER);
    dir.write("Shop/src/KernelTools.nvs", KERNEL_TOOLS);
    dir.write("Blog/public/index.nvs", BLOG_ENTRY);
    dir.write("Blog/src/Post.nvs", &class_using_kernel("Blog", "Post", ""));
    dir.write("Blog/src/Comment.nvs", COMMENT);
    dir.write("Blog/src/KernelNotes.nvs", KERNEL_NOTES);
    dir.write("loose/Scratch.nvs", SCRATCH);
    let root = dir.root().to_path_buf();
    (dir, root)
}

/// What is offered at the [`CURSOR`] in `text`, opened as `file` of the
/// workspace `name`. The workspace is surveyed and indexed whole first, the
/// way the editor has it once its index settles.
fn offered(name: &str, file: &str, text: &str) -> Vec<CompletionItem> {
    let (dir, root) = workspace(name);
    let cursor = text.find(CURSOR).expect("the text marks its cursor");
    let text = text.replacen(CURSOR, "", 1);
    dir.write(file, &text);
    let mut documents = Documents::new();
    documents.survey(CheckScope::Workspace, Some(&root));
    let uri = uri_of(&dir.at(file)).expect("a temp path is UTF-8");
    documents.open(uri.clone(), 1, text);
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    let index = SymbolIndex::build(&documents, CheckScope::Workspace, Some(&root));
    completion::at(
        &analysed,
        &documents,
        &index,
        &CompletionFiles::default(),
        u32::try_from(cursor).expect("a test document is short"),
        Client::default(),
        PositionEncoding::Utf8,
    )
}

/// The qualified name of every type `items` offers, whatever its label.
fn types(items: &[CompletionItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.data.as_ref()?.get("type")?.as_str().map(str::to_owned))
        .collect()
}

/// Asserts that `items` offer every type in `present` and none in `absent`.
fn offers(items: &[CompletionItem], present: &[&str], absent: &[&str]) {
    let types = types(items);
    for name in present {
        assert!(
            types.iter().any(|offered| offered == name),
            "{name} is not offered: {types:?}"
        );
    }
    for name in absent {
        assert!(
            !types.iter().any(|offered| offered == name),
            "{name} is offered: {types:?}"
        );
    }
}

#[test]
fn a_bare_name_offers_no_type_of_another_module() {
    let text = class_using_kernel("Blog", "Post", "S<|>");
    let items = offered("bare-other", "Blog/src/Post.nvs", &text);
    offers(
        &items,
        &[r"Blog\Comment"],
        &[r"Shop\Cart", r"Shop\Order", r"Acme\Mailer"],
    );
}

#[test]
fn a_bare_name_offers_the_framework_and_vendor_types_the_map_loads() {
    let text = class_using_kernel("Shop", "Cart", "R<|>");
    let items = offered("bare-loads", "Shop/src/Cart.nvs", &text);
    offers(
        &items,
        &[
            r"Framework\Router",
            r"Framework\Kernel",
            r"Acme\Mailer",
            r"Shop\Order",
            r"Core\Str",
        ],
        &[r"Blog\Comment"],
    );
}

#[test]
fn a_namespace_list_offers_only_loadable_types() {
    let framework = offered(
        "list-framework",
        "Shop/src/Cart.nvs",
        &class_using_kernel("Shop", "Cart", r"new Framework\<|>"),
    );
    offers(&framework, &[r"Framework\Router", r"Framework\Kernel"], &[]);
    let blog = offered(
        "list-blog",
        "Shop/src/Cart.nvs",
        &class_using_kernel("Shop", "Cart", r"new Blog\<|>"),
    );
    offers(&blog, &[], &[r"Blog\Post", r"Blog\Comment"]);
}

#[test]
fn a_use_line_offers_only_loadable_types() {
    let text = "<?nvs\nnamespace Shop;\n\nuse <|>\n\nclass Cart {}\n";
    let items = offered("use-line", "Shop/src/Cart.nvs", text);
    offers(
        &items,
        &[
            r"Framework\Router",
            r"Acme\Mailer",
            r"Shop\Order",
            r"Core\Str",
        ],
        &[r"Blog\Post", r"Blog\Comment"],
    );
}

#[test]
fn new_offers_only_loadable_classes() {
    let text = class_using_kernel("Blog", "Post", "new C<|>");
    let items = offered("new", "Blog/src/Post.nvs", &text);
    offers(
        &items,
        &[r"Blog\Comment", r"Framework\Router"],
        &[r"Shop\Order", r"Acme\Mailer"],
    );
}

#[test]
fn an_unloaded_extend_block_of_another_module_is_not_offered() {
    let text = class_using_kernel("Shop", "Cart", "$kernel-><|>");
    let items = offered("extend", "Shop/src/Cart.nvs", &text);
    let shown: Vec<String> = items
        .iter()
        .map(|item| {
            let detail = item
                .label_details
                .as_ref()
                .and_then(|details| details.detail.clone());
            format!("{}{}", item.label, detail.unwrap_or_default())
        })
        .collect();
    assert!(
        shown.iter().any(|row| row.contains(r"[Shop\KernelTools]")),
        "the module's own block is offered: {shown:?}"
    );
    assert!(
        !shown.iter().any(|row| row.contains(r"[Blog\KernelNotes]")),
        "another module's block is offered: {shown:?}"
    );
}

#[test]
fn an_autoload_prefix_still_offers_every_namespace() {
    let text = "<?nvs\nautoload 'Framework' from '../../Framework/src';\n\
                autoload 'Shop' from '../src';\nautoload '<|>' from '../lib';\n";
    let items = offered("prefix", "Shop/public/index.nvs", text);
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"Blog"), "{labels:?}");
    assert!(labels.contains(&"Loose"), "{labels:?}");
}

#[test]
fn a_links_root_type_is_not_offered_at_a_bare_name() {
    let text = class_using_kernel("Shop", "Cart", "P<|>");
    let items = offered("links", "Shop/src/Cart.nvs", &text);
    offers(&items, &[r"Shop\Order"], &[r"Blog\Post", r"Blog\Comment"]);
}

#[test]
fn a_file_no_program_loads_is_offered_every_type() {
    let text = "<?nvs\nnamespace Loose;\n\nclass Scratch {\n    \
                public function run(): void {\n        S<|>\n    }\n}\n";
    let items = offered("loose", "loose/Scratch.nvs", text);
    offers(
        &items,
        &[
            r"Shop\Cart",
            r"Shop\Order",
            r"Blog\Post",
            r"Framework\Router",
            r"Core\Str",
        ],
        &[],
    );
}

#[test]
fn references_still_reach_every_module() {
    let (_dir, root) = workspace("references");
    let mut documents = Documents::new();
    documents.survey(CheckScope::Workspace, Some(&root));
    let index = SymbolIndex::build(&documents, CheckScope::Workspace, Some(&root));
    let files: Vec<String> = index
        .occurrences(r"Framework\Kernel")
        .iter()
        .map(|used| used.site.path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert!(
        files.iter().any(|path| path.ends_with("Shop/src/Cart.nvs")),
        "{files:?}"
    );
    assert!(
        files.iter().any(|path| path.ends_with("Blog/src/Post.nvs")),
        "{files:?}"
    );
}
