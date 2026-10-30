//! A one-shot workspace, asked by fully-qualified name.
//!
//! `nvs code` builds one index for one question and reads it through
//! `nvs_lsp::query`, the functions the editor's references and implementation
//! requests call too. These tests hold that reading to what the editor
//! answers: the same uses, resolved the same way, through the same autoload
//! map.
//!
//! The project is on disk, because a workspace is a directory walked for
//! `.nvs` files and an autoload root is a directory too.

use std::fs;
use std::path::{Path, PathBuf};

use nvs_lsp::query::{self, UseKind};
use nvs_lsp::rename;
use nvs_lsp::{Site, Workspace, analyse_file, symbol_at, workspace};

/// The program's entry point: it autoloads `Shop` and uses three of its
/// classes. Lines 8 to 10 declare a variable each.
const INDEX: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Cart;\n\
                     use Shop\\GiftCart;\nuse Shop\\Wishlist;\n\n\
                     var $cart = new Cart();\nvar $total = $cart->addItem(3);\n\
                     var $wish = new Wishlist();\n$wish->addItem(Cart::MAX);\n\
                     echo $total + (new GiftCart())->price();\n";

/// An interface two classes implement, one of them through its superclass.
const PRICED: &str = "<?nvs\nnamespace Shop;\n\ninterface Priced {\n    \
                      public function price(): int;\n}\n";

/// The class every test asks about, with a doc comment on it and on a method.
const CART: &str = "<?nvs\nnamespace Shop;\n\n/// A cart a customer fills.\n\
                    class Cart implements Priced {\n    public const int MAX = 10;\n    \
                    public int $total = 0;\n\n    \
                    /// Adds `n` items and returns the new total.\n    \
                    public function addItem(int $n): int {\n        \
                    $this->total = $this->total + $n;\n        return $this->total;\n    }\n\n    \
                    public function price(): int {\n        return $this->total;\n    }\n}\n";

/// A subclass that overrides `price`.
const GIFT_CART: &str = "<?nvs\nnamespace Shop;\n\nclass GiftCart extends Cart {\n    \
                         public function price(): int {\n        return 0;\n    }\n}\n";

/// A class with a method of the same name as `Cart::addItem` and no relation
/// to it.
const WISHLIST: &str = "<?nvs\nnamespace Shop;\n\nclass Wishlist {\n    \
                        public function addItem(int $n): int {\n        return $n;\n    }\n}\n";

/// The `Shop` project in a scratch directory, deleted when it is dropped, and
/// the workspace built over it.
struct Project {
    dir: nvs_repo::Scratch,
    workspace: Workspace,
}

impl Project {
    fn new(name: &str) -> Self {
        let dir = nvs_repo::scratch(&format!("lsp-query-{name}"));
        for (path, text) in [
            ("public/index.nvs", INDEX),
            ("src/Priced.nvs", PRICED),
            ("src/Cart.nvs", CART),
            ("src/GiftCart.nvs", GIFT_CART),
            ("src/Wishlist.nvs", WISHLIST),
        ] {
            let at = dir.join(path);
            fs::create_dir_all(at.parent().expect("a file has a directory"))
                .expect("a writable scratch directory");
            fs::write(at, text).expect("a writable scratch file");
        }
        let workspace = workspace(&dir);
        Self { dir, workspace }
    }

    fn at(&self, path: &str) -> PathBuf {
        self.dir.join(path)
    }

    /// The text a site covers.
    fn text_of(site: &Site) -> String {
        let text = fs::read_to_string(&site.path).expect("an indexed file");
        text[site.start as usize..site.end as usize].to_owned()
    }
}

/// The file name a site is in, and the text it covers.
fn place(site: &Site) -> (String, String) {
    (name_of(&site.path), Project::text_of(site))
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .expect("a site names a file")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn references_by_name_are_the_ones_the_editor_lists() {
    let project = Project::new("refs");
    let index = project.workspace.index();
    let by_name = query::references(index, r"Shop\Cart::addItem", true);
    let kinds: Vec<UseKind> = by_name.iter().map(|found| found.kind).collect();
    assert_eq!(kinds, [UseKind::Declared, UseKind::Use], "{by_name:?}");
    assert_eq!(
        place(by_name[0].site),
        ("Cart.nvs".to_owned(), "addItem".to_owned())
    );
    assert_eq!(name_of(&by_name[1].site.path), "index.nvs");

    // The editor reads the name under its cursor and asks the same function,
    // so a cursor on the call in the entry point gives the same list.
    let entry = project.at("public/index.nvs");
    let analysed = analyse_file(project.workspace.documents(), &entry, 0).expect("an analysis");
    let offset = u32::try_from(INDEX.find("addItem(3)").expect("the call")).expect("a small file");
    let symbol = symbol_at(&analysed, offset).expect("a name at the call");
    assert_eq!(symbol, r"Shop\Cart::addItem");
    assert_eq!(query::references(index, &symbol, true), by_name);

    // A class is imported as well as used, and the import is listed in its
    // place among the uses.
    let class = query::references(index, r"Shop\Cart", true);
    assert_eq!(
        class.first().map(|found| found.kind),
        Some(UseKind::Declared)
    );
    assert!(
        class.iter().any(|found| found.kind == UseKind::Import),
        "{class:?}"
    );
}

#[test]
fn a_call_on_another_class_with_the_same_method_name_is_not_a_reference() {
    let project = Project::new("same-name");
    let index = project.workspace.index();
    let cart = query::references(index, r"Shop\Cart::addItem", false);
    let wishlist = query::references(index, r"Shop\Wishlist::addItem", false);
    assert_eq!(cart.len(), 1, "{cart:?}");
    assert_eq!(wishlist.len(), 1, "{wishlist:?}");
    assert_ne!(cart[0].site, wishlist[0].site);
}

#[test]
fn an_autoloaded_class_is_resolved_in_a_one_shot_workspace() {
    let project = Project::new("autoload");
    let index = project.workspace.index();
    let declared = index
        .declaration(r"Shop\Cart")
        .expect("the autoloaded class");
    assert_eq!(name_of(&declared.site.path), "Cart.nvs");
    // `new Cart()` in the entry point resolves through the autoload map.
    let uses: Vec<String> = query::references(index, r"Shop\Cart", false)
        .iter()
        .filter(|found| found.kind == UseKind::Use)
        .map(|found| name_of(&found.site.path))
        .collect();
    assert!(uses.contains(&"index.nvs".to_owned()), "{uses:?}");
    // `GiftCart` is analysed through the map it borrows, so its superclass
    // resolves.
    assert_eq!(
        index
            .declaration(r"Shop\GiftCart")
            .map(|found| found.supertypes.clone()),
        Some(vec![r"Shop\Cart".to_owned()])
    );
}

#[test]
fn implementations_list_every_subtype_transitively_and_every_overrider() {
    let project = Project::new("impls");
    let index = project.workspace.index();
    let symbols = |found: &[&nvs_lsp::Declaration]| -> Vec<String> {
        found
            .iter()
            .map(|declared| declared.symbol.clone())
            .collect()
    };

    let priced = query::implementations(index, r"Shop\Priced");
    assert_eq!(symbols(&priced.below), [r"Shop\Cart", r"Shop\GiftCart"]);
    assert!(priced.above.is_empty());

    let price = query::implementations(index, r"Shop\Priced::price");
    assert_eq!(
        symbols(&price.below),
        [r"Shop\Cart::price", r"Shop\GiftCart::price"]
    );

    let gift = query::implementations(index, r"Shop\GiftCart::price");
    assert!(gift.below.is_empty());
    assert_eq!(symbols(&gift.above), [r"Shop\Cart::price"]);
}

#[test]
fn a_declaration_by_name_carries_its_kind_site_and_doc_comment() {
    let project = Project::new("show");
    let card = query::declaration(&project.workspace, r"Shop\Cart::addItem").expect("a method");
    assert_eq!(card.declaration.kind, nvs_lsp::DeclKind::Method);
    assert_eq!(
        place(&card.declaration.site),
        ("Cart.nvs".to_owned(), "addItem".to_owned())
    );
    assert_eq!(
        card.doc.as_deref(),
        Some("Adds `n` items and returns the new total.")
    );

    let class = query::declaration(&project.workspace, r"Shop\Cart").expect("a class");
    assert_eq!(class.declaration.kind, nvs_lsp::DeclKind::Class);
    assert_eq!(class.doc.as_deref(), Some("A cart a customer fills."));

    let price = query::declaration(&project.workspace, r"Shop\Cart::price").expect("a method");
    assert_eq!(price.doc, None);
    assert!(query::declaration(&project.workspace, r"Shop\Basket").is_none());
}

#[test]
fn an_outline_lists_a_files_declarations_nested_by_owner_in_source_order() {
    let project = Project::new("outline");
    let outline = query::outline(project.workspace.index(), &project.at("src/Cart.nvs"));
    let top: Vec<&str> = outline
        .iter()
        .map(|entry| entry.declaration.symbol.as_str())
        .collect();
    assert_eq!(top, [r"Shop\Cart"]);
    let members: Vec<&str> = outline[0]
        .members
        .iter()
        .map(|entry| entry.declaration.symbol.as_str())
        .collect();
    assert_eq!(
        members,
        [
            r"Shop\Cart::MAX",
            r"Shop\Cart::$total",
            r"Shop\Cart::addItem",
            r"Shop\Cart::price"
        ]
    );
    assert!(query::outline(project.workspace.index(), &project.at("src/None.nvs")).is_empty());
}

#[test]
fn types_list_each_binding_in_a_line_range_with_its_inferred_type() {
    let project = Project::new("types");
    let typed = query::types(&project.workspace, &project.at("public/index.nvs"), 8..=9)
        .expect("a file that can be read");
    let names: Vec<&str> = typed.iter().map(|binding| binding.name.as_str()).collect();
    assert_eq!(names, ["$cart", "$total"], "{typed:?}");
    assert!(typed[0].ty.ends_with("Cart"), "{typed:?}");
    assert_eq!(typed[1].ty, "int");
    assert_eq!(Project::text_of(&typed[1].site), "$total");
}

#[test]
fn rename_by_name_matches_rename_at_the_declaration() {
    let project = Project::new("rename-name");
    let documents = project.workspace.documents();
    let index = project.workspace.index();
    let by_name = query::rename(
        &project.workspace,
        query::Target::Name(r"Shop\Cart::addItem"),
        "addProduct",
    )
    .expect("a method the project declares");

    // The editor's rename with the cursor on the declaration's name.
    let cart = project.at("src/Cart.nvs");
    let analysed = analyse_file(documents, &cart, 0).expect("an analysis");
    let offset = u32::try_from(CART.find("addItem").expect("the declaration")).expect("small");
    let at_declaration =
        rename::rename(documents, index, &analysed, offset, "addProduct").expect("a rename");
    assert_eq!(by_name, at_declaration);
    // The declaration and the call in the entry point, and not `Wishlist`'s
    // method of the same name.
    let files: Vec<String> = by_name
        .rewrites
        .iter()
        .map(|rewrite| name_of(&rewrite.site.path))
        .collect();
    assert_eq!(files, ["index.nvs", "Cart.nvs"], "{by_name:?}");

    let unknown = query::rename(
        &project.workspace,
        query::Target::Name(r"Shop\Cart::none"),
        "x",
    );
    assert!(unknown.is_err(), "{unknown:?}");
}

#[test]
fn rename_by_position_matches_rename_at_that_offset() {
    let project = Project::new("rename-position");
    let documents = project.workspace.documents();
    let index = project.workspace.index();
    let entry = project.at("public/index.nvs");
    // Line 9 is `var $total = $cart->addItem(3);`, and column 5 is its `$`.
    let at = query::Target::Position {
        path: &entry,
        line: 9,
        column: 5,
    };
    let by_position = query::rename(&project.workspace, at, "$sum").expect("a local");

    let analysed = analyse_file(documents, &entry, 0).expect("an analysis");
    let offset = u32::try_from(INDEX.find("$total").expect("the local")).expect("small");
    let at_offset = rename::rename(documents, index, &analysed, offset, "sum").expect("a rename");
    assert_eq!(by_position, at_offset);
    // The declaration and the read in the `echo`.
    let texts: Vec<&str> = by_position
        .rewrites
        .iter()
        .map(|rewrite| rewrite.text.as_str())
        .collect();
    assert_eq!(texts, ["$sum", "$sum"], "{by_position:?}");
    assert!(by_position.file.is_none());
}

#[test]
fn an_unknown_name_gives_the_nearest_names() {
    let project = Project::new("nearest");
    let index = project.workspace.index();
    // The shell removes the backslashes from an unquoted name.
    assert!(index.declaration("ShopCart").is_none());
    let nearest = query::nearest(index, "ShopCart", 3);
    assert_eq!(nearest.first(), Some(&r"Shop\Cart"), "{nearest:?}");

    let methods = query::nearest(index, "addItems", 2);
    assert_eq!(
        methods,
        [r"Shop\Cart::addItem", r"Shop\Wishlist::addItem"],
        "{methods:?}"
    );
}
