//! `nvs code`, driven through the built binary in a project on disk, because
//! `rule:tooling/nvs-code-answers-about-the-project`'s contract is what the
//! command prints: the positions, the escaping, the exit status and the
//! project it finds from the working directory.
//!
//! The project is the `Shop` fixture `nvs-lsp`'s `tests/query.rs` asks the
//! same questions of by function, with an `nvs.toml` at its root so the
//! command stops there.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

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

/// The `Shop` project and `extra` files in a scratch directory, deleted when
/// it is dropped.
fn project(name: &str, extra: &[(&str, &str)]) -> nvs_repo::Scratch {
    let dir = nvs_repo::scratch(&format!("code-{name}"));
    let files = [
        ("nvs.toml", ""),
        ("public/index.nvs", INDEX),
        ("src/Priced.nvs", PRICED),
        ("src/Cart.nvs", CART),
        ("src/GiftCart.nvs", GIFT_CART),
        ("src/Wishlist.nvs", WISHLIST),
    ];
    for (path, text) in files.iter().chain(extra) {
        let at = dir.join(path);
        fs::create_dir_all(at.parent().expect("a file has a directory"))
            .expect("a writable scratch directory");
        fs::write(at, text).expect("a writable scratch file");
    }
    dir
}

/// `nvs code <args...>` run in `cwd`, as `(stdout, stderr, success)`.
fn code(cwd: &Path, args: &[&str]) -> (String, String, bool) {
    run(cwd, "code", args)
}

fn run(cwd: &Path, command: &str, args: &[&str]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(cwd)
        .arg(command)
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the output is UTF-8"),
        String::from_utf8(out.stderr).expect("the output is UTF-8"),
        out.status.success(),
    )
}

fn parsed(out: &str) -> Value {
    serde_json::from_str(out).unwrap_or_else(|err| panic!("{err}: {out}"))
}

// covers: tools:cli/nvs-code
#[test]
fn refs_prints_the_declaration_first_then_each_use_with_its_position_and_line() {
    let dir = project("refs", &[]);
    let (out, err, ok) = code(&dir, &["refs", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    let lines: Vec<Vec<&str>> = out
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert_eq!(lines[0][..2], ["src/Cart.nvs:10:21", "declared"], "{out}");
    assert_eq!(
        lines[0][2..].join(" "),
        "public function addItem(int $n): int {"
    );
    // The call on the `Wishlist` is another method and is not listed.
    assert_eq!(lines[1][..2], ["public/index.nvs:9:21", "use"], "{out}");
    assert_eq!(lines[1][2..].join(" "), "var $total = $cart->addItem(3);");

    let (class, _, _) = code(&dir, &["refs", r"Shop\Cart"]);
    let kinds: Vec<&str> = class
        .lines()
        .map(|line| line.split_whitespace().nth(1).unwrap_or_default())
        .collect();
    assert_eq!(kinds.first(), Some(&"declared"), "{class}");
    assert!(kinds.contains(&"import"), "{class}");
}

// covers: tools:cli/nvs-code
#[test]
fn refs_of_a_name_nothing_uses_prints_nothing_and_exits_zero() {
    let dir = project("unused", &[]);
    // `Core\Str::length` is a real name the project does not use.
    let (out, err, ok) = code(&dir, &["refs", r"Core\Str::length"]);
    assert!(ok, "{err}");
    assert_eq!(out, "");
    let (out, _, ok) = code(&dir, &["refs", r"Core\Str::length", "--json"]);
    assert!(ok);
    assert_eq!(parsed(&out)["references"], Value::Array(Vec::new()));
}

// covers: tools:cli/nvs-code
#[test]
fn show_prints_the_signature_nvs_meta_writes_and_the_doc_comment() {
    let dir = project("show", &[]);
    let (meta, err, ok) = run(&dir, "meta", &["--json", "public/index.nvs"]);
    assert!(ok, "{err}");
    let meta = parsed(&meta);
    let cart = meta["program"]["classes"]
        .as_array()
        .expect("the program's classes")
        .iter()
        .find(|class| class["name"] == r"Shop\Cart")
        .expect("the program half has `Shop\\Cart`");
    let written = |name: &str| -> String {
        cart["members"]
            .as_array()
            .expect("members")
            .iter()
            .find(|member| member["name"] == name)
            .and_then(|member| member["signature"].as_str())
            .expect("a member with a signature")
            .to_owned()
    };

    let (out, err, ok) = code(&dir, &["show", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    assert_eq!(
        out,
        format!(
            "Shop\\Cart::addItem  method, declared at src/Cart.nvs:10:21\n{}\n\
             Adds `n` items and returns the new total.\n",
            written("addItem")
        )
    );
    let (out, _, _) = code(&dir, &["show", r"Shop\Cart::$total", "--json"]);
    assert_eq!(parsed(&out)["signature"], written("$total").as_str());

    let (out, _, ok) = code(&dir, &["show", r"Shop\Cart"]);
    assert!(ok);
    assert_eq!(
        out,
        "Shop\\Cart  class, declared at src/Cart.nvs:5:7\nA cart a customer fills.\n"
    );
}

// covers: tools:cli/nvs-code
#[test]
fn show_of_a_core_name_prints_the_card_nvs_agent_show_prints() {
    let dir = project("core", &[]);
    for args in [&[r"Core\IO::read"][..], &[r"Core\IO::read", "--json"]] {
        let (by_code, err, ok) = code(&dir, &[&["show"], args].concat());
        assert!(ok, "{err}");
        let (by_agent, _, _) = run(&dir, "agent", &[&["show"], args].concat());
        assert!(!by_code.is_empty());
        assert_eq!(by_code, by_agent);
    }
}

// covers: tools:cli/nvs-code
#[test]
fn an_unknown_name_fails_and_lists_the_nearest_names_with_their_backslashes() {
    let dir = project("nearest", &[]);
    // The shell removes the backslashes from an unquoted name.
    for verb in ["show", "refs", "impls"] {
        let (out, err, ok) = code(&dir, &[verb, "ShopCart"]);
        assert!(!ok, "{verb}");
        assert_eq!(out, "", "{verb}");
        assert!(err.contains("ShopCart"), "{err}");
        let nearest: Vec<&str> = err
            .lines()
            .skip_while(|line| *line != "nearest:")
            .skip(1)
            .map(str::trim)
            .collect();
        assert_eq!(nearest.first(), Some(&r"Shop\Cart"), "{err}");
    }
    let (_, err, ok) = code(&dir, &["outline", "src/Crat.nvs"]);
    assert!(!ok);
    assert!(err.contains("  src/Cart.nvs"), "{err}");
}

// covers: tools:cli/nvs-code
#[test]
fn every_verb_json_follows_the_check_json_conventions() {
    let dir = project("json", &[]);
    let answers: [(&[&str], &[&str]); 5] = [
        (
            &["show", r"Shop\Cart::price"],
            &[
                "symbol",
                "kind",
                "path",
                "line",
                "column",
                "start",
                "end",
                "signature",
                "doc",
            ],
        ),
        (&["refs", r"Shop\Cart"], &["symbol", "references"]),
        (&["impls", r"Shop\Priced"], &["symbol", "below", "above"]),
        (&["outline", "src/Cart.nvs"], &["path", "declarations"]),
        (&["types", "public/index.nvs:8-10"], &["path", "types"]),
    ];
    for (args, keys) in answers {
        let (out, err, ok) = code(&dir, &[args, &["--json"]].concat());
        assert!(ok, "{args:?}: {err}");
        let document = parsed(&out);
        assert_eq!(document["schemaVersion"], 1, "{args:?}");
        for key in keys {
            assert!(
                document.get(key).is_some(),
                "{args:?} has no `{key}`: {out}"
            );
        }
    }
    // A key with nothing to say is present as `null`.
    let (out, _, _) = code(&dir, &["show", r"Shop\Cart::price", "--json"]);
    assert_eq!(parsed(&out)["doc"], Value::Null);
    // Nested members carry their position and their own `members`.
    let (out, _, _) = code(&dir, &["outline", "src/Cart.nvs", "--json"]);
    let members = &parsed(&out)["declarations"][0]["members"];
    assert_eq!(members[0]["symbol"], r"Shop\Cart::MAX");
    assert_eq!(members[0]["members"], Value::Array(Vec::new()));
    let (out, _, _) = code(&dir, &["types", "public/index.nvs:9", "--json"]);
    assert_eq!(parsed(&out)["types"][0]["type"], "int");

    for args in [
        &["show", "ShopCart"][..],
        &["refs", "ShopCart"],
        &["impls", "ShopCart"],
        &["outline", "src/Crat.nvs"],
        &["types", "src/Crat.nvs"],
    ] {
        let (out, _, ok) = code(&dir, &[args, &["--json"]].concat());
        assert!(!ok, "{args:?}");
        let document = parsed(&out);
        assert_eq!(document["schemaVersion"], 1);
        assert!(document["error"].is_string(), "{out}");
        assert!(
            document["nearest"]
                .as_array()
                .is_some_and(|nearest| !nearest.is_empty()),
            "{out}"
        );
    }
}

// covers: tools:cli/nvs-code
#[test]
fn a_project_with_errors_still_answers_and_counts_them_on_stderr() {
    let broken = "<?nvs\nnamespace Shop;\n\nclass Broken {\n    \
                  public function total(): int {\n        return 'none';\n    }\n}\n";
    let dir = project("errors", &[("src/Broken.nvs", broken)]);
    let (out, err, ok) = code(&dir, &["refs", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    assert_eq!(out.lines().count(), 2, "{out}");
    assert_eq!(
        err.trim(),
        "warning: 1 file in this project has errors. A use inside it may be missing."
    );
    let (out, _, ok) = code(&dir, &["outline", "src/Broken.nvs"]);
    assert!(ok);
    assert!(out.contains(r"Shop\Broken::total"), "{out}");

    let clean = project("clean", &[]);
    let (_, err, _) = code(&clean, &["refs", r"Shop\Cart"]);
    assert_eq!(err, "");
}

// covers: tools:cli/nvs-code
#[test]
fn the_project_is_the_nearest_nvs_toml_at_or_above_the_working_directory() {
    let lib = "<?nvs\nnamespace Lib;\n\nclass Tool {\n}\n";
    let dir = project(
        "root",
        &[("vendor/lib/nvs.toml", ""), ("vendor/lib/Tool.nvs", lib)],
    );
    // From a folder inside the project, the whole project is read, and a path
    // is relative to the working directory.
    let (out, err, ok) = code(&dir.join("public"), &["refs", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    assert!(out.starts_with("../src/Cart.nvs:10:21 "), "{out}");
    assert!(out.contains("\nindex.nvs:9:21 "), "{out}");
    // A folder with its own `nvs.toml` is a project of its own.
    let (_, _, ok) = code(&dir.join("vendor/lib"), &["show", r"Shop\Cart"]);
    assert!(!ok);
    let (out, _, ok) = code(&dir.join("vendor/lib"), &["show", r"Lib\Tool"]);
    assert!(ok);
    assert!(out.contains("declared at Tool.nvs:4:7"), "{out}");
}

// covers: tools:cli/nvs-code
#[test]
fn a_position_is_one_based_and_counts_characters() {
    let till = "<?nvs\nnamespace Shop;\n\nclass Till {\n    \
                public function ring(): int {\n        \
                return /* é € */ (new Cart())->addItem(1);\n    }\n}\n";
    let dir = project("position", &[("src/Till.nvs", till)]);
    let (out, err, ok) = code(&dir, &["refs", r"Shop\Cart::addItem", "--json"]);
    assert!(ok, "{err}");
    let document = parsed(&out);
    let found = document["references"]
        .as_array()
        .expect("references")
        .iter()
        .find(|found| found["path"] == "src/Till.nvs")
        .expect("the call in `Till`")
        .clone();
    let line = till.lines().nth(5).expect("the sixth line");
    let before = &line[..line.find("addItem").expect("the call")];
    assert_eq!(found["line"], 6);
    assert_eq!(found["column"], before.chars().count() + 1);
    assert_ne!(before.chars().count(), before.len());
    let line_start = till.find(line).expect("the line");
    assert_eq!(found["start"], line_start + before.len());
    assert_eq!(found["end"], line_start + before.len() + "addItem".len());
    assert!(
        out.contains("return /* é € */ (new Cart())->addItem(1);"),
        "{out}"
    );
}

// covers: tools:cli/nvs-code
#[test]
fn control_characters_from_the_project_are_printed_escaped() {
    let noisy = "<?nvs\nnamespace Shop;\n\n/// Rings \u{1b}[31mred\u{1b}[0m.\nclass Noisy {\n    \
                 public function ring(): int {\n        \
                 return (new Cart())->addItem(1); // \u{1b}[2J\u{7}\n    }\n}\n";
    let dir = project("escape", &[("src/Noisy.nvs", noisy)]);
    let (out, err, ok) = code(&dir, &["show", r"Shop\Noisy"]);
    assert!(ok, "{err}");
    assert!(out.contains(r"Rings \u{1b}[31mred\u{1b}[0m."), "{out}");
    assert!(!out.contains('\u{1b}'), "{out}");

    let (out, _, _) = code(&dir, &["refs", r"Shop\Cart::addItem"]);
    assert!(out.contains(r"// \u{1b}[2J\u{7}"), "{out}");
    assert!(!out.contains('\u{1b}') && !out.contains('\u{7}'), "{out}");

    let (out, _, _) = code(&dir, &["show", r"Shop\Noisy", "--json"]);
    assert_eq!(parsed(&out)["doc"], r"Rings \u{1b}[31mred\u{1b}[0m.");
}

/// The attack under `tests/hostile/tools/cli/nvs-code/`, as one file of the
/// project: a file that requires itself, whose doc comments carry escape and
/// bell bytes and a second line spelled like a `refs` line. `show` and
/// `outline` answer and stop, every control byte is escaped, and the fake line
/// is the doc comment's text, never a position of its own.
// covers: tools:cli/nvs-code
#[test]
fn a_doc_comment_that_writes_to_the_terminal_is_printed_as_text() {
    let name = "01-a-doc-comment-that-writes-to-the-terminal.nvs";
    let attack = fs::read_to_string(nvs_repo::path(&format!(
        "tests/hostile/tools/cli/nvs-code/{name}"
    )))
    .expect("the attack is in the tree");
    let dir = project("hostile", &[(&format!("src/{name}"), &attack)]);

    let (out, err, ok) = code(&dir, &["show", r"Shop\Alarm"]);
    assert!(ok, "{err}");
    assert!(
        out.contains(
            r"Clears the screen \u{1b}[2J\u{1b}[H and prints \u{1b}[31mred\u{1b}[0m text."
        ),
        "{out}"
    );
    assert!(!out.contains('\u{1b}'), "{out}");

    let (out, _, _) = code(&dir, &["show", r"Shop\Alarm", "--json"]);
    let doc = parsed(&out)["doc"]
        .as_str()
        .expect("a doc string")
        .to_owned();
    assert!(
        doc.ends_with("\nsrc/Admin.nvs:1:1  declared  class Admin"),
        "{doc}"
    );

    let (out, _, ok) = code(&dir, &["show", r"Shop\Alarm::ring"]);
    assert!(ok);
    assert!(
        out.contains(r"Rings the bell \u{7} and moves the cursor \u{1b}[10A up."),
        "{out}"
    );
    assert!(!out.contains('\u{1b}') && !out.contains('\u{7}'), "{out}");

    let (out, err, ok) = code(&dir, &["outline", &format!("src/{name}")]);
    assert!(ok, "{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "the class and its method, and nothing else: {out}"
    );
    assert!(lines[0].ends_with(r"class  Shop\Alarm"), "{out}");
    assert!(!out.contains("Admin"), "{out}");
}

/// The fixture's files, as `(path, text)` in a fixed order, for a test that
/// checks nothing was written.
fn files(dir: &Path) -> Vec<(&'static str, String)> {
    [
        "public/index.nvs",
        "src/Priced.nvs",
        "src/Cart.nvs",
        "src/GiftCart.nvs",
        "src/Wishlist.nvs",
    ]
    .into_iter()
    .map(|path| (path, fs::read_to_string(dir.join(path)).unwrap_or_default()))
    .collect()
}

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|err| panic!("{path}: {err}"))
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_renames_a_method_at_every_site_and_in_its_overrides() {
    let dir = project("rename-method", &[]);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Cart::price", "cost"]);
    assert!(ok, "{err}");
    // The interface declares it, `Cart` implements it, `GiftCart` overrides
    // it, and the entry point calls it.
    for path in [
        "src/Priced.nvs",
        "src/Cart.nvs",
        "src/GiftCart.nvs",
        "public/index.nvs",
    ] {
        let text = read(&dir, path);
        assert!(text.contains("cost()"), "{path}: {text}");
        assert!(!text.contains("price"), "{path}: {text}");
        assert!(out.contains(path), "{out}");
    }
    assert!(out.contains("public/index.nvs  1 edit\n"), "{out}");
    assert_eq!(read(&dir, "src/Wishlist.nvs"), WISHLIST);
    let (_, err, ok) = run(&dir, "check", &["public/index.nvs"]);
    assert!(ok, "the renamed project still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_renames_a_type_and_moves_the_file_named_after_it() {
    let dir = project("rename-type", &[]);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Wishlist", "Favourites"]);
    assert!(ok, "{err}");
    assert!(!dir.join("src/Wishlist.nvs").exists());
    assert!(
        read(&dir, "src/Favourites.nvs").contains("class Favourites"),
        "{out}"
    );
    let index = read(&dir, "public/index.nvs");
    assert!(index.contains("use Shop\\Favourites;"), "{index}");
    assert!(index.contains("new Favourites()"), "{index}");
    assert!(
        out.contains("moved src/Wishlist.nvs -> src/Favourites.nvs"),
        "{out}"
    );
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_renames_a_local_at_a_position() {
    let dir = project("rename-local", &[]);
    // Line 9 is `var $total = $cart->addItem(3);`, and column 5 is its `$`.
    let (out, err, ok) = code(&dir, &["rename", "public/index.nvs:9:5", "sum"]);
    assert!(ok, "{err}");
    assert_eq!(out, "public/index.nvs  2 edits\n");
    let index = read(&dir, "public/index.nvs");
    assert!(index.contains("var $sum = $cart->addItem(3);"), "{index}");
    assert!(index.contains("echo $sum + "), "{index}");
    // The property of the same name in `Cart` is another symbol.
    assert_eq!(read(&dir, "src/Cart.nvs"), CART);
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_refuses_a_clash_and_writes_nothing() {
    let dir = project("rename-clash", &[]);
    let before = files(&dir);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Cart::addItem", "price"]);
    assert!(!ok, "{out}");
    assert!(err.contains("price"), "{err}");
    assert_eq!(files(&dir), before);

    let (out, _, ok) = code(&dir, &["rename", r"Shop\Cart::addItem", "price", "--json"]);
    assert!(!ok);
    let answer = parsed(&out);
    assert!(answer["error"].as_str().is_some(), "{out}");
    assert_eq!(answer["nearest"], Value::Array(Vec::new()));
    assert_eq!(files(&dir), before);
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_refuses_a_parameter_and_writes_nothing() {
    let dir = project("rename-parameter", &[]);
    let before = files(&dir);
    // Line 10 is `    public function addItem(int $n): int {`, and column 33
    // is the `$` of `$n`.
    let (out, err, ok) = code(&dir, &["rename", "src/Cart.nvs:10:33", "count"]);
    assert!(!ok, "{out}");
    assert!(err.starts_with("error: "), "{err}");
    assert_eq!(files(&dir), before);

    // A position on nothing renames nothing, with the message the editor shows.
    let (_, err, ok) = code(&dir, &["rename", "src/Cart.nvs:1:2", "x"]);
    assert!(!ok);
    assert!(
        err.contains("There is nothing to rename here. You can rename a class"),
        "{err}"
    );
    assert_eq!(files(&dir), before);
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_of_an_unknown_name_prints_the_nearest() {
    let dir = project("rename-unknown", &[]);
    let before = files(&dir);
    let (_, err, ok) = code(&dir, &["rename", r"Shop\Cart::addItems", "x"]);
    assert!(!ok);
    assert!(err.contains(r"Shop\Cart::addItem"), "{err}");
    let (out, _, ok) = code(&dir, &["rename", r"Shop\Cart::addItems", "x", "--json"]);
    assert!(!ok);
    let nearest = parsed(&out)["nearest"].clone();
    assert_eq!(nearest[0], r"Shop\Cart::addItem", "{out}");

    let (_, err, ok) = code(&dir, &["rename", "src/Carts.nvs:1:1", "x"]);
    assert!(!ok);
    assert!(err.contains("src/Cart.nvs"), "{err}");
    assert_eq!(files(&dir), before);
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_dry_run_prints_every_edit_and_writes_nothing() {
    let dir = project("rename-dry", &[]);
    let before = files(&dir);
    let (out, err, ok) = code(
        &dir,
        &["rename", r"Shop\Cart::addItem", "addProduct", "--dry-run"],
    );
    assert!(ok, "{err}");
    let lines: Vec<Vec<&str>> = out
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(
        lines,
        [
            ["public/index.nvs:9:21", "addItem", "->", "addProduct"],
            ["src/Cart.nvs:10:21", "addItem", "->", "addProduct"],
        ],
        "{out}"
    );
    assert_eq!(files(&dir), before);

    let (out, _, ok) = code(
        &dir,
        &["rename", r"Shop\Wishlist", "Favourites", "--dry-run"],
    );
    assert!(ok);
    assert!(
        out.contains("move src/Wishlist.nvs -> src/Favourites.nvs"),
        "{out}"
    );
    assert_eq!(files(&dir), before);
    assert!(!dir.join("src/Favourites.nvs").exists());
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_json_lists_every_edit_and_the_moved_file() {
    let dir = project("rename-json", &[]);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Wishlist", "Favourites", "--json"]);
    assert!(ok, "{err}");
    let answer = parsed(&out);
    assert_eq!(answer["schemaVersion"], 1, "{out}");
    assert_eq!(answer["written"], true, "{out}");
    assert_eq!(
        answer["moved"],
        serde_json::json!([{ "from": "src/Wishlist.nvs", "to": "src/Favourites.nvs" }]),
        "{out}"
    );
    let edits = answer["edits"].as_array().expect("an edits list");
    assert!(!edits.is_empty(), "{out}");
    for edit in edits {
        for key in ["path", "line", "column", "start", "end", "old", "new"] {
            assert!(edit.get(key).is_some(), "{key}: {out}");
        }
        // A `use` line's edit covers the whole name, `Shop\Wishlist`.
        let old = edit["old"].as_str().expect("the old text");
        assert!(old.ends_with("Wishlist"), "{out}");
        assert_eq!(
            edit["new"],
            old.replace("Wishlist", "Favourites").as_str(),
            "{out}"
        );
    }
    assert!(dir.join("src/Favourites.nvs").exists());
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_refuses_a_file_that_changed_after_it_was_read() {
    let dir = project("rename-changed", &[]);
    let before = files(&dir);
    // The debug binary changes this file after it computes the edits and
    // before it compares the bytes again.
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(&*dir)
        .env(
            "NVS_TEST_CODE_CHANGE_BEFORE_WRITE",
            dir.join("src/Cart.nvs"),
        )
        .args(["code", "rename", r"Shop\Cart::addItem", "addProduct"])
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let err = String::from_utf8(out.stderr).expect("the output is UTF-8");
    assert!(!out.status.success(), "{err}");
    assert!(err.contains("src/Cart.nvs"), "{err}");
    assert!(err.contains("changed while the command ran"), "{err}");
    // Only the change the hook made is on disk.
    assert_eq!(read(&dir, "src/Cart.nvs"), format!("{CART}\n"));
    assert_eq!(read(&dir, "public/index.nvs"), before[0].1);
}

/// A second entry point that requires two function files, one of which
/// requires the other.
const APP: &str = "<?nvs\nrequire '../lib/helpers.nvs';\nrequire '../lib/text/upper.nvs';\n\n\
                   echo shout(greet()), \"\\n\";\n";

const HELPERS: &str = "<?nvs\nfunction greet(): string {\n    return 'hi';\n}\n";

const UPPER: &str = "<?nvs\nrequire '../helpers.nvs';\n\n\
                     function shout(string $s): string {\n    return $s . '!';\n}\n";

/// The `Shop` project with [`APP`] and the two files it requires.
fn move_project(name: &str) -> nvs_repo::Scratch {
    project(
        name,
        &[
            ("public/app.nvs", APP),
            ("lib/helpers.nvs", HELPERS),
            ("lib/text/upper.nvs", UPPER),
        ],
    )
}

/// Every file of [`move_project`] that exists, as `(path, text)`, for a test
/// that checks nothing was written or moved.
fn move_files(dir: &Path) -> Vec<(String, String)> {
    let mut paths: Vec<&str> = files(dir).iter().map(|(path, _)| *path).collect();
    paths.extend(["public/app.nvs", "lib/helpers.nvs", "lib/text/upper.nvs"]);
    paths
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(dir.join(path)).ok()?;
            Some((path.to_owned(), text))
        })
        .collect()
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_moves_a_file_and_rewrites_each_require_of_it() {
    let dir = move_project("move-file");
    let (out, err, ok) = code(&dir, &["move", "lib/helpers.nvs", "lib/greetings.nvs"]);
    assert!(ok, "{err}");
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        [
            "lib/text/upper.nvs  1 edit",
            "public/app.nvs  1 edit",
            "moved lib/helpers.nvs -> lib/greetings.nvs",
        ],
        "{out}"
    );
    assert!(!dir.join("lib/helpers.nvs").exists());
    assert_eq!(read(&dir, "lib/greetings.nvs"), HELPERS);
    assert_eq!(
        read(&dir, "public/app.nvs"),
        APP.replace("lib/helpers.nvs", "lib/greetings.nvs")
    );
    assert_eq!(
        read(&dir, "lib/text/upper.nvs"),
        UPPER.replace("helpers.nvs", "greetings.nvs")
    );
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_moves_a_directory_and_rewrites_each_require_into_it() {
    let dir = move_project("move-directory");
    let (out, err, ok) = code(&dir, &["move", "lib/text", "text"]);
    assert!(ok, "{err}");
    assert!(out.contains("moved lib/text -> text"), "{out}");
    assert!(!dir.join("lib/text").exists());
    assert_eq!(
        read(&dir, "public/app.nvs"),
        APP.replace("../lib/text/upper.nvs", "../text/upper.nvs")
    );
    // The moved file's own `require` names the file it stayed beside.
    assert_eq!(
        read(&dir, "text/upper.nvs"),
        UPPER.replace("../helpers.nvs", "../lib/helpers.nvs")
    );
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_keeps_a_relative_require_relative_and_an_absolute_one_absolute() {
    let dir = move_project("move-absolute");
    // An absolute path with `/` between its parts, as a program writes one,
    // without the `\\?\` a canonical Windows path starts with.
    let root = dir.canonicalize().expect("the scratch directory exists");
    let root = root.to_string_lossy().replace('\\', "/");
    let root = root.strip_prefix("//?/").unwrap_or(&root);
    let absolute = format!("<?nvs\nrequire '{root}/lib/helpers.nvs';\n\necho greet();\n");
    fs::write(dir.join("public/absolute.nvs"), &absolute).expect("a writable scratch file");

    let (_, err, ok) = code(&dir, &["move", "lib/helpers.nvs", "public/helpers.nvs"]);
    assert!(ok, "{err}");
    assert_eq!(
        read(&dir, "public/app.nvs"),
        APP.replace("'../lib/helpers.nvs'", "'helpers.nvs'")
    );
    let text = read(&dir, "public/absolute.nvs");
    let literal = text
        .lines()
        .find_map(|line| line.strip_prefix("require '")?.strip_suffix("';"))
        .unwrap_or_else(|| panic!("a require line: {text}"));
    assert!(Path::new(literal).is_absolute(), "{text}");
    assert_eq!(
        Path::new(literal).canonicalize().ok(),
        dir.join("public/helpers.nvs").canonicalize().ok(),
        "{text}"
    );
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_refuses_a_target_that_exists_and_writes_nothing() {
    let dir = move_project("move-exists");
    let before = move_files(&dir);
    let (out, err, ok) = code(&dir, &["move", "lib/helpers.nvs", "src/Cart.nvs"]);
    assert!(!ok, "{out}");
    assert!(err.contains("`src/Cart.nvs` already exists"), "{err}");
    assert_eq!(move_files(&dir), before);

    // A directory cannot move into itself, and a missing directory is not
    // created.
    let (_, err, ok) = code(&dir, &["move", "lib", "lib/inner"]);
    assert!(!ok);
    assert!(err.contains("cannot be moved into itself"), "{err}");
    let (_, err, ok) = code(&dir, &["move", "lib/helpers.nvs", "lib/none/helpers.nvs"]);
    assert!(!ok);
    assert!(err.contains("does not exist"), "{err}");
    assert!(!dir.join("lib/none").exists());

    // A source that does not exist prints the nearest files.
    let (_, err, ok) = code(&dir, &["move", "lib/helper.nvs", "lib/greetings.nvs"]);
    assert!(!ok);
    assert!(err.contains("lib/helpers.nvs"), "{err}");
    assert_eq!(move_files(&dir), before);
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_refuses_a_path_outside_the_project_and_writes_nothing() {
    let dir = move_project("move-outside");
    let before = move_files(&dir);
    let name = "code-move-outside-target.nvs";
    let (out, err, ok) = code(&dir, &["move", "lib/helpers.nvs", &format!("../{name}")]);
    assert!(!ok, "{out}");
    assert!(err.contains("is outside the project"), "{err}");
    assert!(!dir.parent().expect("a parent").join(name).exists());

    // The project's parent directory exists and is outside it.
    let (_, err, ok) = code(&dir, &["move", "..", "moved"]);
    assert!(!ok);
    assert!(err.contains("is outside the project"), "{err}");
    assert!(!dir.join("moved").exists());
    assert_eq!(move_files(&dir), before);
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_dry_run_prints_every_edit_and_moves_nothing() {
    let dir = move_project("move-dry");
    let before = move_files(&dir);
    let (out, err, ok) = code(
        &dir,
        &["move", "lib/helpers.nvs", "lib/greetings.nvs", "--dry-run"],
    );
    assert!(ok, "{err}");
    let lines: Vec<Vec<&str>> = out
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(
        lines,
        [
            [
                "lib/text/upper.nvs:2:9",
                "'../helpers.nvs'",
                "->",
                "'../greetings.nvs'"
            ],
            [
                "public/app.nvs:2:9",
                "'../lib/helpers.nvs'",
                "->",
                "'../lib/greetings.nvs'"
            ],
            ["move", "lib/helpers.nvs", "->", "lib/greetings.nvs"],
        ],
        "{out}"
    );
    assert_eq!(move_files(&dir), before);
    assert!(!dir.join("lib/greetings.nvs").exists());
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_json_lists_every_edit_and_the_move() {
    let dir = move_project("move-json");
    let (out, err, ok) = code(&dir, &["move", "lib/text", "text", "--json"]);
    assert!(ok, "{err}");
    let answer = parsed(&out);
    assert_eq!(answer["schemaVersion"], 1, "{out}");
    assert_eq!(answer["written"], true, "{out}");
    assert_eq!(
        answer["moved"],
        serde_json::json!([{ "from": "lib/text", "to": "text" }]),
        "{out}"
    );
    let edits = answer["edits"].as_array().expect("an edits list");
    assert_eq!(edits.len(), 2, "{out}");
    for edit in edits {
        for key in ["path", "line", "column", "start", "end", "old", "new"] {
            assert!(edit.get(key).is_some(), "{key}: {out}");
        }
    }
    // The edits name each file where it was before the move.
    assert_eq!(edits[0]["path"], "lib/text/upper.nvs", "{out}");
    assert_eq!(edits[0]["new"], "'../lib/helpers.nvs'", "{out}");
    assert_eq!(edits[1]["path"], "public/app.nvs", "{out}");
    assert_eq!(edits[1]["new"], "'../text/upper.nvs'", "{out}");
    assert!(dir.join("text/upper.nvs").exists());
}

/// A route handler, the methods it calls and two tests: one sends a request
/// to the route, and one calls a method directly. `Prices::base` is also
/// taken as a callable value on the last line.
const CALLS: &str = "<?nvs\nuse Core\\Http\\Method;\nuse Core\\Test;\n\n\
                     class Prices {\n    public function base(): int {\n        return 3;\n    }\n    \
                     public function plain(): int {\n        return 5;\n    }\n}\n\n\
                     class Shop {\n    #[Core\\Route(path: '/cart', method: Method::Get)]\n    \
                     #[Core\\Access(allow: Core\\Audience::Public)]\n    \
                     public function cart(): void {\n        var $p = new Prices();\n        \
                     echo $p->base();\n        echo $p->plain();\n    }\n}\n\n\
                     class ShopTest {\n    #[Test]\n    public function cartLoads(): void {\n        \
                     var $page = Test::request(Method::Get, '/cart');\n        \
                     Test::assertEquals(200, $page->status());\n    }\n    #[Test]\n    \
                     public function plainIsFive(): void {\n        \
                     Test::assertEquals(5, new Prices()->plain());\n    }\n}\n\n\
                     var $f = new Prices()->base(...);\n";

/// A project of one file, [`CALLS`], in a scratch directory.
fn calls_project(name: &str) -> nvs_repo::Scratch {
    let dir = nvs_repo::scratch(&format!("code-{name}"));
    fs::write(dir.join("nvs.toml"), "").expect("a writable scratch file");
    fs::write(dir.join("app.nvs"), CALLS).expect("a writable scratch file");
    dir
}

/// Each line of `out`, its runs of spaces made one.
fn words(out: &str) -> Vec<String> {
    out.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

#[test]
fn code_find_lists_each_declaration_whose_own_name_contains_the_query() {
    let dir = project("find", &[]);
    let (out, err, ok) = code(&dir, &["find", "item"]);
    assert!(ok, "{err}");
    assert_eq!(
        words(&out),
        [
            r"src/Cart.nvs:10:21 method Shop\Cart::addItem",
            r"src/Wishlist.nvs:5:21 method Shop\Wishlist::addItem",
        ],
        "{out}"
    );
    // The namespace is not part of a declaration's own name.
    let (out, err, ok) = code(&dir, &["find", "shop"]);
    assert!(ok, "{err}");
    assert_eq!(out, "");
}

#[test]
fn code_find_ignores_case() {
    let dir = project("find-case", &[]);
    let (lower, _, _) = code(&dir, &["find", "additem"]);
    let (upper, err, ok) = code(&dir, &["find", "ADDITEM"]);
    assert!(ok, "{err}");
    assert_eq!(upper.lines().count(), 2, "{upper}");
    assert_eq!(upper, lower);
}

#[test]
fn code_find_with_no_match_prints_nothing_and_succeeds() {
    let dir = project("find-none", &[]);
    let (out, err, ok) = code(&dir, &["find", "nowhere"]);
    assert!(ok, "{err}");
    assert_eq!(out, "");
    assert_eq!(err, "");
}

#[test]
fn code_calls_lists_callers_and_callees_grouped_by_method() {
    let dir = calls_project("calls");
    let (out, err, ok) = code(&dir, &["calls", "Prices::plain"]);
    assert!(ok, "{err}");
    assert_eq!(
        words(&out),
        [
            "Shop::cart caller",
            "app.nvs:20:18 echo $p->plain();",
            "ShopTest::plainIsFive caller",
            "app.nvs:32:45 Test::assertEquals(5, new Prices()->plain());",
        ],
        "{out}"
    );
    let (out, err, ok) = code(&dir, &["calls", "Shop::cart"]);
    assert!(ok, "{err}");
    let lines = words(&out);
    // The callers come first, then one line per callee and its sites.
    assert_eq!(lines[0], "ShopTest::cartLoads caller", "{out}");
    assert_eq!(
        lines[2..],
        [
            "Prices::base callee",
            "app.nvs:19:18 echo $p->base();",
            "Prices::plain callee",
            "app.nvs:20:18 echo $p->plain();",
        ],
        "{out}"
    );
}

#[test]
fn code_calls_includes_a_test_request_to_a_route() {
    let dir = calls_project("calls-request");
    let (out, err, ok) = code(&dir, &["calls", "Shop::cart"]);
    assert!(ok, "{err}");
    assert_eq!(
        words(&out)[..2],
        [
            "ShopTest::cartLoads caller",
            "app.nvs:27:48 var $page = Test::request(Method::Get, '/cart');",
        ],
        "{out}"
    );
    let (out, err, ok) = code(&dir, &["calls", "ShopTest::cartLoads"]);
    assert!(ok, "{err}");
    assert_eq!(words(&out)[0], "Shop::cart callee", "{out}");
}

#[test]
fn code_calls_of_an_unknown_name_prints_the_nearest() {
    let dir = calls_project("calls-unknown");
    // A class is a name of the project, but it is not a method.
    let (out, err, ok) = code(&dir, &["calls", "Prices"]);
    assert!(!ok);
    assert_eq!(out, "");
    assert!(err.contains("no method `Prices` in this project"), "{err}");
    assert!(err.contains("  Prices::base\n"), "{err}");
    assert!(!err.contains("  Prices\n"), "{err}");

    let (out, _, ok) = code(&dir, &["reach", "--json", "Prices::bse"]);
    assert!(!ok);
    let answer = parsed(&out);
    assert_eq!(answer["nearest"][0], "Prices::base", "{out}");
}

#[test]
fn code_reach_lists_the_routes_and_the_tests() {
    let dir = calls_project("reach");
    let (out, err, ok) = code(&dir, &["reach", "Prices::plain"]);
    assert!(ok, "{err}");
    assert_eq!(
        words(&out),
        [
            "route GET /cart",
            "test ShopTest::cartLoads",
            "test ShopTest::plainIsFive",
        ],
        "{out}"
    );
}

#[test]
fn code_reach_says_at_least_when_a_call_is_not_followed() {
    let dir = calls_project("reach-at-least");
    let (out, err, ok) = code(&dir, &["reach", "Prices::base"]);
    assert!(ok, "{err}");
    let lines = words(&out);
    assert_eq!(
        lines[..2],
        ["route GET /cart", "test ShopTest::cartLoads"],
        "{out}"
    );
    assert_eq!(lines.len(), 3, "{out}");
    assert!(lines[2].starts_with("at least:"), "{out}");
    let (out, _, _) = code(&dir, &["reach", "Prices::plain"]);
    assert!(!out.contains("at least"), "{out}");
}

#[test]
fn code_find_calls_and_reach_json_have_every_key() {
    let dir = calls_project("calls-json");
    let (out, err, ok) = code(&dir, &["find", "--json", "plain"]);
    assert!(ok, "{err}");
    let answer = parsed(&out);
    assert_eq!(answer["schemaVersion"], 1, "{out}");
    assert_eq!(answer["query"], "plain", "{out}");
    let found = answer["declarations"]
        .as_array()
        .expect("a declarations list");
    assert_eq!(found.len(), 2, "{out}");
    for key in ["path", "line", "column", "start", "end", "symbol", "kind"] {
        assert!(found[0].get(key).is_some(), "{key}: {out}");
    }
    assert_eq!(found[0]["symbol"], "Prices::plain", "{out}");
    assert_eq!(found[0]["kind"], "method", "{out}");

    let (out, err, ok) = code(&dir, &["calls", "--json", "Shop::cart"]);
    assert!(ok, "{err}");
    let answer = parsed(&out);
    assert_eq!(answer["schemaVersion"], 1, "{out}");
    assert_eq!(answer["symbol"], "Shop::cart", "{out}");
    assert_eq!(
        answer["callers"][0]["symbol"], "ShopTest::cartLoads",
        "{out}"
    );
    let callees = answer["callees"].as_array().expect("a callees list");
    assert_eq!(callees.len(), 2, "{out}");
    let site = &callees[0]["sites"][0];
    for key in ["path", "line", "column", "start", "end", "text"] {
        assert!(site.get(key).is_some(), "{key}: {out}");
    }
    assert_eq!(site["text"], "echo $p->base();", "{out}");

    let (out, err, ok) = code(&dir, &["reach", "--json", "Prices::base"]);
    assert!(ok, "{err}");
    assert_eq!(
        parsed(&out),
        serde_json::json!({
            "schemaVersion": 1,
            "symbol": "Prices::base",
            "routes": ["GET /cart"],
            "tests": ["ShopTest::cartLoads"],
            "atLeast": true,
        }),
        "{out}"
    );
}
