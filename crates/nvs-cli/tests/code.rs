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

/// Sets the modification time of every file `project` wrote in `dir` to an
/// hour ago. An analysis records a file's size and time only when the file
/// changed more than two seconds before it started, so with old files the
/// first call records them, and a later call has nothing to add.
fn age(dir: &Path) {
    let hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    for path in [
        "nvs.toml",
        "public/index.nvs",
        "src/Priced.nvs",
        "src/Cart.nvs",
        "src/GiftCart.nvs",
        "src/Wishlist.nvs",
    ] {
        fs::File::options()
            .write(true)
            .open(dir.join(path))
            .and_then(|file| file.set_modified(hour_ago))
            .expect("a scratch file takes a modification time");
    }
}

/// `nvs code <args...>` run in `cwd`, as `(stdout, stderr, success)`.
fn code(cwd: &Path, args: &[&str]) -> (String, String, bool) {
    run(cwd, "code", args)
}

/// `nvs <command> <args...>` run in `cwd` with its data folder in `cwd`'s
/// `.nvsdata`, so no index file reaches the binary's own data folder. The
/// project walk skips a dot-directory, so the folder is never analysed.
fn run(cwd: &Path, command: &str, args: &[&str]) -> (String, String, bool) {
    run_with_data(cwd, &cwd.join(".nvsdata"), command, args)
}

/// `nvs --data <data> <command> <args...>` run in `cwd`.
fn run_with_data(cwd: &Path, data: &Path, command: &str, args: &[&str]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(cwd)
        .arg("--data")
        .arg(data)
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

/// The index files in the data folder `data`, the one per project root the
/// editor reads and writes.
fn index_files(data: &Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(data.join("cache/lsp-index"))
        .map(|dir| dir.filter_map(|entry| Some(entry.ok()?.path())).collect())
        .unwrap_or_default()
}

// covers: tools:cli/nvs-code
#[test]
fn the_first_call_saves_the_index_in_the_data_folder() {
    let dir = project("index-saved", &[]);
    age(&dir);
    let data = dir.join(".nvsdata");
    assert!(index_files(&data).is_empty());
    let (first, err, ok) = code(&dir, &["refs", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    let saved = index_files(&data);
    assert_eq!(saved.len(), 1, "{saved:?}");
    let bytes = fs::read(&saved[0]).expect("the saved index file reads");
    assert!(bytes.starts_with(b"NVSIDX"), "{saved:?}");
    // A second call loads the file, analyses nothing and saves nothing, so
    // the file keeps its bytes and the answer is the same.
    let (second, err, ok) = code(&dir, &["refs", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    assert_eq!(second, first);
    assert_eq!(fs::read(&saved[0]).expect("the index file reads"), bytes);
}

// covers: tools:cli/nvs-code
#[test]
fn an_edit_between_two_calls_reaches_the_second_answer() {
    let dir = project("index-edit", &[]);
    let (before, err, ok) = code(&dir, &["refs", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    assert_eq!(before.lines().count(), 2, "{before}");
    assert_eq!(index_files(&dir.join(".nvsdata")).len(), 1);
    // A new call in the program, and a body edit in the library that keeps
    // the file's size, both reach the second answer.
    let index = dir.join("public/index.nvs");
    let mut text = fs::read_to_string(&index).expect("the program reads");
    text.push_str("$cart->addItem(5);\n");
    fs::write(&index, text).expect("the program writes");
    let cart = dir.join("src/Cart.nvs");
    let edited = CART.replace("$this->total + $n;", "$this->addItem(1);");
    assert_eq!(edited.len(), CART.len());
    fs::write(&cart, edited).expect("the library writes");
    let (after, err, ok) = code(&dir, &["refs", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    assert_eq!(after.lines().count(), 4, "{after}");
    assert!(after.contains("\npublic/index.nvs:13:8 "), "{after}");
    assert!(after.contains("\nsrc/Cart.nvs:11:31 "), "{after}");
}

// covers: tools:cli/nvs-code
#[test]
fn a_call_without_a_data_folder_answers_cold() {
    let dir = project("index-cold", &[]);
    let (with, err, ok) = code(&dir, &["refs", r"Shop\Cart::addItem"]);
    assert!(ok, "{err}");
    // A data folder that is a file has no index file to load or save, so
    // every call analyses the whole project and gives the same answer.
    let file = dir.join("not-a-folder");
    fs::write(&file, "").expect("a writable scratch file");
    for _ in 0..2 {
        let (without, err, ok) =
            run_with_data(&dir, &file, "code", &["refs", r"Shop\Cart::addItem"]);
        assert!(ok, "{err}");
        assert_eq!(without, with);
    }
    assert!(file.is_file());
    assert_eq!(fs::read(&file).expect("the file reads"), b"");
}

/// A program with one route and a method that takes a route, and a call that
/// passes the route method to it on line 12.
const MENU: &str = "<?nvs\nclass Pages {\n    \
                    #[Core\\Route(path: '/pages/{id}', method: Core\\Http\\Method::Get)]\n    \
                    #[Core\\Access(allow: Core\\Audience::Public)]\n    \
                    public static function show(uint $id): string { return 'page'; }\n}\n\
                    class Menu {\n    \
                    public static function entry(#[Core\\Router\\Target] callable $route,\n        \
                    #[Core\\Router\\Params] array $params): string { return 'entry'; }\n}\n\n\
                    echo Menu::entry(Pages::show, ['id' => 3]);\n";

// covers: tools:cli/nvs-code
#[test]
fn refs_lists_a_route_target_argument_as_a_use_of_the_route_method() {
    let dir = project("route-target", &[("public/menu.nvs", MENU)]);
    let (out, err, ok) = code(&dir, &["refs", "Pages::show"]);
    assert!(ok, "{err}");
    let lines: Vec<Vec<&str>> = out
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert_eq!(lines[0][..2], ["public/menu.nvs:5:28", "declared"], "{out}");
    assert_eq!(lines[1][..2], ["public/menu.nvs:12:25", "use"], "{out}");
    assert_eq!(
        lines[1][2..].join(" "),
        "echo Menu::entry(Pages::show, ['id' => 3]);"
    );
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

/// A second entry point that writes `Wishlist` with its full name.
const LIST: &str = "<?nvs\nautoload 'Shop' from '../src';\n\n\
                    var $wish = new Shop\\Wishlist();\necho $wish->addItem(1);\n";

// covers: tools:cli/nvs-code
#[test]
fn code_rename_moves_a_type_to_another_namespace_and_rewrites_every_use() {
    let dir = project("rename-namespace", &[("public/list.nvs", LIST)]);
    let (out, err, ok) = code(
        &dir,
        &["rename", r"Shop\Wishlist", r"Shop\Billing\Wishlist"],
    );
    assert!(ok, "{err}");
    assert!(!dir.join("src/Wishlist.nvs").exists(), "{out}");
    let moved = read(&dir, "src/Billing/Wishlist.nvs");
    assert!(moved.contains("namespace Shop\\Billing;"), "{moved}");
    assert!(moved.contains("class Wishlist"), "{moved}");
    let index = read(&dir, "public/index.nvs");
    assert!(index.contains("use Shop\\Billing\\Wishlist;"), "{index}");
    assert!(!index.contains("use Shop\\Wishlist;"), "{index}");
    let list = read(&dir, "public/list.nvs");
    assert!(list.contains("new Shop\\Billing\\Wishlist()"), "{list}");
    assert!(
        out.contains("moved src/Wishlist.nvs -> src/Billing/Wishlist.nvs"),
        "{out}"
    );
    for entry in ["public/index.nvs", "public/list.nvs"] {
        let (_, err, ok) = run(&dir, "check", &[entry]);
        assert!(ok, "{entry} still checks: {err}");
    }
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_adds_a_use_line_where_a_bare_name_relied_on_the_old_namespace() {
    let dir = project("rename-namespace-bare", &[]);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Cart", r"Shop\Billing\Cart"]);
    assert!(ok, "{err}");
    // `GiftCart` is in `Shop` and wrote `Cart` with no `use` line.
    let gift = read(&dir, "src/GiftCart.nvs");
    assert!(
        gift.contains("namespace Shop;\n\nuse Shop\\Billing\\Cart;\n"),
        "{out}\n{gift}"
    );
    assert!(gift.contains("extends Cart"), "{gift}");
    let (_, err, ok) = run(&dir, "check", &["public/index.nvs"]);
    assert!(ok, "the moved project still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_adds_use_lines_inside_the_moved_file_for_its_old_neighbours() {
    let dir = project("rename-namespace-neighbours", &[]);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Cart", r"Shop\Billing\Cart"]);
    assert!(ok, "{err}");
    // `Cart` wrote `Priced` with no `use` line, because both were in `Shop`.
    let cart = read(&dir, "src/Billing/Cart.nvs");
    assert!(
        cart.contains("namespace Shop\\Billing;\n\nuse Shop\\Priced;\n"),
        "{out}\n{cart}"
    );
    assert!(cart.contains("class Cart implements Priced"), "{cart}");
}

/// A class in `Shop\Billing`.
const INVOICE: &str = "<?nvs\nnamespace Shop\\Billing;\n\nclass Invoice {\n    \
                       public int $total = 0;\n}\n";

/// A class in `Shop\Billing` that imports `Wishlist` from `Shop`.
const REFUND: &str = "<?nvs\nnamespace Shop\\Billing;\n\nuse Shop\\Wishlist;\n\n\
                      class Refund {\n    public function list(): Wishlist {\n        \
                      return new Wishlist();\n    }\n}\n";

/// `Wishlist`, importing `Invoice` from `Shop\Billing`.
const WISHLIST_INVOICE: &str = "<?nvs\nnamespace Shop;\n\nuse Shop\\Billing\\Invoice;\n\n\
                                class Wishlist {\n    \
                                public function addItem(int $n): int {\n        \
                                return $n;\n    }\n\n    \
                                public function invoice(): Invoice {\n        \
                                return new Invoice();\n    }\n}\n";

/// An entry point that loads `Refund`.
const REFUNDS: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Billing\\Refund;\n\n\
                       echo (new Refund())->list()->addItem(2);\n";

// covers: tools:cli/nvs-code
#[test]
fn code_rename_deletes_a_use_line_that_names_its_own_namespace() {
    let dir = project(
        "rename-namespace-own",
        &[
            ("src/Wishlist.nvs", WISHLIST_INVOICE),
            ("src/Billing/Invoice.nvs", INVOICE),
            ("src/Billing/Refund.nvs", REFUND),
            ("public/refunds.nvs", REFUNDS),
        ],
    );
    let (out, err, ok) = code(
        &dir,
        &["rename", r"Shop\Wishlist", r"Shop\Billing\Wishlist"],
    );
    assert!(ok, "{err}");
    // `Refund` is in the new namespace, so it needs no `use` line for it, and
    // the blank line after the `namespace` line stays one line.
    let refund = read(&dir, "src/Billing/Refund.nvs");
    assert_eq!(
        refund,
        REFUND.replace("use Shop\\Wishlist;\n\n", ""),
        "{out}"
    );
    // The moved file is now in `Invoice`'s namespace.
    let moved = read(&dir, "src/Billing/Wishlist.nvs");
    assert_eq!(
        moved,
        WISHLIST_INVOICE
            .replace("namespace Shop;", "namespace Shop\\Billing;")
            .replace("use Shop\\Billing\\Invoice;\n\n", ""),
    );
    for entry in ["public/index.nvs", "public/refunds.nvs"] {
        let (_, err, ok) = run(&dir, "check", &[entry]);
        assert!(ok, "{entry} still checks: {err}");
    }
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_creates_the_new_namespace_directory_under_its_root() {
    let dir = project("rename-namespace-directory", &[]);
    assert!(!dir.join("src/Billing").exists());
    let (out, err, ok) = code(
        &dir,
        &["rename", r"Shop\Wishlist", r"Shop\Billing\Lists\Wishlist"],
    );
    assert!(ok, "{err}");
    assert!(
        dir.join("src/Billing/Lists/Wishlist.nvs").is_file(),
        "{out}"
    );
    let (_, err, ok) = run(&dir, "check", &["public/index.nvs"]);
    assert!(ok, "the moved project still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_refuses_a_namespace_no_autoload_root_reaches() {
    let dir = project("rename-namespace-unreached", &[]);
    let before = files(&dir);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Wishlist", r"Billing\Wishlist"]);
    assert!(!ok, "{out}");
    assert!(err.contains("autoload"), "{out}{err}");
    assert_eq!(files(&dir), before);
}

/// A file that declares two classes, which only a `require` could load.
const LABELS: &str = "<?nvs\nnamespace Shop\\Tools;\n\nclass Label {\n}\n\nclass Tag {\n}\n";

// covers: tools:cli/nvs-code
#[test]
fn code_rename_refuses_to_move_a_type_that_shares_its_file() {
    let dir = project("rename-namespace-shared", &[("lib/labels.nvs", LABELS)]);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Tools\Label", r"Shop\Other\Label"]);
    assert!(!ok, "{out}");
    assert!(err.contains("own file"), "{out}{err}");
    assert_eq!(read(&dir, "lib/labels.nvs"), LABELS);
}

/// A class with a type alias it uses by its bare name, and a constant.
/// `nvs code rename --from <list>` in `dir`, with `list` written to
/// `renames.txt` first.
fn rename_from(dir: &Path, list: &str, extra: &[&str]) -> (String, String, bool) {
    fs::write(dir.join("renames.txt"), list).expect("a writable scratch file");
    let mut args = vec!["rename", "--from", "renames.txt"];
    args.extend_from_slice(extra);
    code(dir, &args)
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_from_a_list_applies_every_rename_in_one_write() {
    let dir = project("rename-from", &[]);
    let list = "# Two renames.\nShop\\Cart::addItem addProduct\n\nShop\\Wishlist Favourites\n";

    let (out, err, ok) = rename_from(&dir, list, &["--dry-run", "--json"]);
    assert!(ok, "{err}");
    let answer = parsed(&out);
    let edits = answer["edits"].as_array().expect("an edit list");
    let from_lines: std::collections::BTreeSet<u64> = edits
        .iter()
        .map(|edit| {
            edit["fromLine"]
                .as_u64()
                .expect("each edit has its list line")
        })
        .collect();
    assert_eq!(from_lines, [2, 4].into(), "{out}");
    assert_eq!(answer["written"], Value::Bool(false), "{out}");

    let (out, err, ok) = rename_from(&dir, list, &[]);
    assert!(ok, "{err}");
    assert!(
        read(&dir, "src/Cart.nvs").contains("function addProduct("),
        "{out}"
    );
    let index = read(&dir, "public/index.nvs");
    assert!(index.contains("$cart->addProduct(3)"), "{index}");
    assert!(index.contains("new Favourites()"), "{index}");
    // `Wishlist::addItem` is another method, so it keeps its name.
    assert!(
        read(&dir, "src/Favourites.nvs").contains("function addItem("),
        "{out}"
    );
    assert!(!dir.join("src/Wishlist.nvs").exists(), "{out}");
    let (_, err, ok) = run(&dir, "check", &["public/index.nvs"]);
    assert!(ok, "the renamed project still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_from_a_list_swaps_two_names() {
    let dir = project("rename-from-swap", &[]);
    let (out, err, ok) = rename_from(&dir, "Shop\\Cart Wishlist\nShop\\Wishlist Cart\n", &[]);
    assert!(ok, "{err}");
    let cart = read(&dir, "src/Cart.nvs");
    assert!(cart.contains("class Cart {"), "{out}\n{cart}");
    let wishlist = read(&dir, "src/Wishlist.nvs");
    assert!(
        wishlist.contains("class Wishlist implements Priced"),
        "{out}\n{wishlist}"
    );
    assert!(
        read(&dir, "src/GiftCart.nvs").contains("extends Wishlist"),
        "{out}"
    );
    let index = read(&dir, "public/index.nvs");
    assert!(index.contains("$wish->addItem(Wishlist::MAX)"), "{index}");
    let (_, err, ok) = run(&dir, "check", &["public/index.nvs"]);
    assert!(ok, "the swapped project still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_from_a_list_refuses_two_renames_of_one_site_and_writes_nothing() {
    let dir = project("rename-from-overlap", &[]);
    let before = files(&dir);
    let list = "Shop\\Cart::addItem addProduct\nShop\\Cart::addItem addMany\n";
    let (out, err, ok) = rename_from(&dir, list, &[]);
    assert!(!ok, "{out}");
    assert!(err.contains("line 1 and line 2 both change"), "{err}");
    assert_eq!(files(&dir), before);
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_from_a_list_refuses_one_new_name_given_twice() {
    let dir = project("rename-from-twice", &[]);
    let before = files(&dir);
    let list = "Shop\\Cart::addItem add\nShop\\Cart::price add\n";
    let (out, err, ok) = rename_from(&dir, list, &[]);
    assert!(!ok, "{out}");
    assert!(
        err.contains("line 1 and line 2 both give the name"),
        "{err}"
    );
    assert_eq!(files(&dir), before);
}

const ORDER: &str = "<?nvs\nnamespace Shop;\n\nfinal class Order {\n    \
                     public const int LIMIT = 10;\n\n    type Total = int;\n\n    \
                     public function total(): Total {\n        return 3;\n    }\n}\n";

/// A second entry point that writes the alias with its owner's name.
const REPORT: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Order;\n\n\
                      final class Report {\n    \
                      public static function show(Order::Total $total): void {\n        \
                      echo $total;\n    }\n}\n\n\
                      Report::show((new Order())->total());\n";

// covers: tools:cli/nvs-code
#[test]
fn code_rename_renames_a_class_scoped_type_alias_at_every_site() {
    let dir = project(
        "rename-owned-alias",
        &[("src/Order.nvs", ORDER), ("public/report.nvs", REPORT)],
    );
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Order::Total", "Amount"]);
    assert!(ok, "{err}");
    let order = read(&dir, "src/Order.nvs");
    assert!(order.contains("type Amount = int;"), "{order}");
    assert!(order.contains("function total(): Amount {"), "{order}");
    let report = read(&dir, "public/report.nvs");
    assert!(
        report.contains("show(Order::Amount $total): void"),
        "{report}"
    );
    assert!(out.contains("src/Order.nvs  2 edits\n"), "{out}");
    assert!(out.contains("public/report.nvs  1 edit\n"), "{out}");
    assert!(!out.contains("moved"), "{out}");
    let (_, err, ok) = run(&dir, "check", &["public/report.nvs"]);
    assert!(ok, "the renamed project still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_refuses_a_class_scoped_alias_name_its_owner_uses() {
    let dir = project(
        "rename-owned-alias-clash",
        &[("src/Order.nvs", ORDER), ("public/report.nvs", REPORT)],
    );
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Order::Total", "LIMIT"]);
    assert!(!ok, "{out}");
    assert!(
        err.contains(r"`Shop\Order` already has a constant named `LIMIT`."),
        "{err}"
    );
    assert_eq!(read(&dir, "src/Order.nvs"), ORDER);
    assert_eq!(read(&dir, "public/report.nvs"), REPORT);
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
fn code_rename_rewrites_a_parameter_and_its_uses() {
    let dir = project("rename-parameter", &[]);
    // Line 10 is `    public function addItem(int $n): int {`, and column 33
    // is the `$` of `$n`. No call passes `n:` by name, so the declaration and
    // its one use in the body are the two edits.
    let (out, err, ok) = code(&dir, &["rename", "src/Cart.nvs:10:33", "count"]);
    assert!(ok, "{err}");
    assert!(out.contains("src/Cart.nvs  2 edits"), "{out}");
    let cart = fs::read_to_string(dir.join("src/Cart.nvs")).expect("the cart reads");
    assert!(
        cart.contains("public function addItem(int $count): int {"),
        "{cart}"
    );
    assert!(cart.contains("$this->total + $count;"), "{cart}");
    let before = files(&dir);

    // A position on nothing renames nothing, with the message the editor shows.
    let (_, err, ok) = code(&dir, &["rename", "src/Cart.nvs:1:2", "x"]);
    assert!(!ok);
    assert!(
        err.contains("There is nothing to rename here. You can rename a class"),
        "{err}"
    );
    assert_eq!(files(&dir), before);
}

/// A `Page` whose `initProperty` compares its name parameter with `'views'`.
const LATE_PAGE: &str = "<?nvs\nnamespace Shop;\n\nuse LateInit;\n\nclass Page implements LateInit {\n    \
                         public lateinit int $views;\n\n    \
                         public function initProperty(string $name): void {\n        \
                         if ($name == 'views') {\n            $this->views = 0;\n        }\n    \
                         }\n}\n";

/// An entry point that reads `$views` of a `Page`.
const PAGE_ENTRY: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Page;\n\n\
                          echo (new Page())->views;\n";

/// The `Shop` project with `page` as `src/Page.nvs` and an entry point that
/// uses it.
fn page_project(name: &str, page: &str) -> nvs_repo::Scratch {
    project(
        name,
        &[("src/Page.nvs", page), ("public/page.nvs", PAGE_ENTRY)],
    )
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_of_a_lateinit_property_renames_the_name_init_property_compares() {
    let dir = page_project("rename-lateinit-name", LATE_PAGE);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Page::$views", "hits"]);
    assert!(ok, "{err}");
    let page = read(&dir, "src/Page.nvs");
    assert!(page.contains("public lateinit int $hits;"), "{page}");
    assert!(page.contains("if ($name == 'hits') {"), "{page}");
    assert!(page.contains("$this->hits = 0;"), "{page}");
    assert!(out.contains("src/Page.nvs  3 edits\n"), "{out}");
    assert!(read(&dir, "public/page.nvs").contains("->hits;"));
    let (out, err, ok) = run(&dir, "run", &["public/page.nvs"]);
    assert!(ok, "the renamed project still runs: {err}");
    assert_eq!(out, "0");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_of_an_observed_property_renames_the_name_its_hooks_compare() {
    let page = "<?nvs\nnamespace Shop;\n\nuse PropertyObserver;\n\n\
                class Page implements PropertyObserver {\n    \
                public int $views = 0;\n    public string $title = '';\n\n    \
                public function onPropertyGet(string $name, mixed $value): void {\n        \
                if (\"views\" == $name) {\n            echo 'read ';\n        }\n    }\n\n    \
                public function onPropertySet(string $name, mixed $value): void {\n        \
                if ($name != \"views\") {\n            return;\n        }\n        \
                echo 'write ';\n    }\n}\n";
    let dir = page_project("rename-observed-name", page);
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Page::$views", "hits"]);
    assert!(ok, "{err}");
    let page = read(&dir, "src/Page.nvs");
    // Each literal keeps its own quotes.
    assert!(page.contains("if (\"hits\" == $name) {"), "{page}");
    assert!(page.contains("if ($name != \"hits\") {"), "{page}");
    assert!(page.contains("$title = '';"), "{page}");
    assert!(out.contains("src/Page.nvs  3 edits\n"), "{out}");
    let (out, err, ok) = run(&dir, "run", &["public/page.nvs"]);
    assert!(ok, "the renamed project still runs: {err}");
    assert_eq!(out, "read 0");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_renames_a_match_arm_on_the_property_name() {
    let page = "<?nvs\nnamespace Shop;\n\nuse LateInit;\n\nclass Page implements LateInit {\n    \
                public lateinit int $views;\n\n    \
                public function initProperty(string $name): void {\n        \
                $this->views = match ($name) {\n            'views' => 7,\n            \
                default => 0,\n        };\n    }\n}\n";
    let dir = page_project("rename-match-name", page);
    let (_, err, ok) = code(&dir, &["rename", r"Shop\Page::$views", "hits"]);
    assert!(ok, "{err}");
    let page = read(&dir, "src/Page.nvs");
    assert!(page.contains("'hits' => 7,"), "{page}");
    let (out, err, ok) = run(&dir, "run", &["public/page.nvs"]);
    assert!(ok, "the renamed project still runs: {err}");
    assert_eq!(out, "7");
}

// covers: tools:cli/nvs-code
#[test]
fn code_rename_refuses_a_property_whose_name_a_callback_uses_another_way() {
    // `'views'` is printed, not compared, so a rename cannot tell whether it
    // names the property.
    let page = LATE_PAGE.replace(
        "        if ($name == 'views') {",
        "        echo 'views';\n        if ($name == 'views') {",
    );
    let dir = page_project("rename-callback-other-use", &page);
    let before = read(&dir, "src/Page.nvs");
    let (out, err, ok) = code(&dir, &["rename", r"Shop\Page::$views", "hits"]);
    assert!(!ok, "{out}");
    assert!(
        err.contains("`initProperty` in `Page.nvs` uses `'views'` on line 10"),
        "{err}"
    );
    assert_eq!(read(&dir, "src/Page.nvs"), before);

    // A body that writes its name parameter is refused too.
    let page = LATE_PAGE.replace(
        "        if ($name == 'views') {",
        "        $name = 'views';\n        if ($name == 'views') {",
    );
    let dir = page_project("rename-callback-assigned", &page);
    let (_, err, ok) = code(&dir, &["rename", r"Shop\Page::$views", "hits"]);
    assert!(!ok);
    assert!(
        err.contains("`initProperty` in `Page.nvs` changes its first parameter"),
        "{err}"
    );
}

// covers: tools:cli/nvs-code
#[test]
fn code_refs_of_a_lateinit_property_lists_the_compared_name() {
    let dir = page_project("refs-lateinit-name", LATE_PAGE);
    let (out, err, ok) = code(&dir, &["refs", r"Shop\Page::$views"]);
    assert!(ok, "{err}");
    let lines: Vec<Vec<&str>> = out
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    // Line 10 is `        if ($name == 'views') {`, and column 23 is the
    // first letter between the quotes.
    assert!(
        lines
            .iter()
            .any(|line| line[..2] == ["src/Page.nvs:10:23", "use"]
                && line[2..].join(" ") == "if ($name == 'views') {"),
        "{out}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line[0].starts_with("public/page.nvs:6:")),
        "{out}"
    );
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
        .arg("--data")
        .arg(dir.join(".nvsdata"))
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

// covers: tools:cli/nvs-code
#[test]
fn code_move_of_an_autoloaded_file_renames_its_type_to_the_new_namespace() {
    let dir = project(
        "move-namespace",
        &[
            ("src/Billing/Invoice.nvs", INVOICE),
            ("public/list.nvs", LIST),
        ],
    );
    let (out, err, ok) = code(
        &dir,
        &["move", "src/Wishlist.nvs", "src/Billing/Wishlist.nvs"],
    );
    assert!(ok, "{err}");
    assert!(
        out.contains("moved src/Wishlist.nvs -> src/Billing/Wishlist.nvs"),
        "{out}"
    );
    let moved = read(&dir, "src/Billing/Wishlist.nvs");
    assert!(moved.contains("namespace Shop\\Billing;"), "{moved}");
    let index = read(&dir, "public/index.nvs");
    assert!(index.contains("use Shop\\Billing\\Wishlist;"), "{index}");
    let list = read(&dir, "public/list.nvs");
    assert!(list.contains("new Shop\\Billing\\Wishlist()"), "{list}");
    for entry in ["public/index.nvs", "public/list.nvs"] {
        let (_, err, ok) = run(&dir, "check", &[entry]);
        assert!(ok, "{entry} still checks: {err}");
    }
}

/// A class in `Shop\Billing` that writes its neighbour `Invoice` by its bare
/// name, and imports `Wishlist` from `Shop`.
const REFUND_INVOICE: &str = "<?nvs\nnamespace Shop\\Billing;\n\nuse Shop\\Wishlist;\n\n\
                              class Refund {\n    public function list(): Wishlist {\n        \
                              return new Wishlist();\n    }\n\n    \
                              public function invoice(): Invoice {\n        \
                              return new Invoice();\n    }\n}\n";

// covers: tools:cli/nvs-code
#[test]
fn code_move_of_a_directory_renames_the_namespace_of_every_type_under_it() {
    let dir = project(
        "move-namespace-directory",
        &[
            ("src/Billing/Invoice.nvs", INVOICE),
            ("src/Billing/Refund.nvs", REFUND_INVOICE),
            ("public/refunds.nvs", REFUNDS),
        ],
    );
    let (out, err, ok) = code(&dir, &["move", "src/Billing", "src/Accounts"]);
    assert!(ok, "{err}");
    assert!(out.contains("moved src/Billing -> src/Accounts"), "{out}");
    assert_eq!(
        read(&dir, "src/Accounts/Invoice.nvs"),
        INVOICE.replace("Shop\\Billing", "Shop\\Accounts"),
        "{out}"
    );
    // `Invoice` moved with `Refund`, so `Refund` still writes it with no `use`
    // line.
    assert_eq!(
        read(&dir, "src/Accounts/Refund.nvs"),
        REFUND_INVOICE.replace("Shop\\Billing", "Shop\\Accounts"),
        "{out}"
    );
    let refunds = read(&dir, "public/refunds.nvs");
    assert!(refunds.contains("use Shop\\Accounts\\Refund;"), "{refunds}");
    let (_, err, ok) = run(&dir, "check", &["public/refunds.nvs"]);
    assert!(ok, "the moved project still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_to_a_new_file_name_renames_the_type() {
    let dir = project("move-file-name", &[]);
    let (out, err, ok) = code(&dir, &["move", "src/Wishlist.nvs", "src/Favourites.nvs"]);
    assert!(ok, "{err}");
    assert_eq!(
        read(&dir, "src/Favourites.nvs"),
        WISHLIST.replace("class Wishlist", "class Favourites"),
        "{out}"
    );
    let index = read(&dir, "public/index.nvs");
    assert!(index.contains("use Shop\\Favourites;"), "{index}");
    assert!(index.contains("new Favourites()"), "{index}");
    let (_, err, ok) = run(&dir, "check", &["public/index.nvs"]);
    assert!(ok, "the moved project still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_refuses_an_autoloaded_file_no_root_reaches_at_its_new_path() {
    let dir = move_project("move-namespace-unreached");
    let before = move_files(&dir);
    let (out, err, ok) = code(&dir, &["move", "src/Wishlist.nvs", "lib/Wishlist.nvs"]);
    assert!(!ok, "{out}");
    assert!(err.contains("autoload"), "{out}{err}");
    assert_eq!(move_files(&dir), before);
}

/// A class only a `require` reaches, in a file not named after it.
const LABEL: &str = "<?nvs\nnamespace Shop\\Tools;\n\nclass Label {\n    \
                     public string $text = 'new';\n}\n";

/// An entry point that requires [`LABEL`].
const LABELLED: &str = "<?nvs\nrequire '../lib/label.nvs';\n\nuse Shop\\Tools\\Label;\n\n\
                        echo (new Label())->text;\n";

// covers: tools:cli/nvs-code
#[test]
fn code_move_of_a_required_file_keeps_its_names() {
    let dir = project(
        "move-required",
        &[("lib/label.nvs", LABEL), ("public/labels.nvs", LABELLED)],
    );
    let (out, err, ok) = code(&dir, &["move", "lib/label.nvs", "lib/Tag.nvs"]);
    assert!(ok, "{err}");
    assert_eq!(read(&dir, "lib/Tag.nvs"), LABEL, "{out}");
    assert_eq!(
        read(&dir, "public/labels.nvs"),
        LABELLED.replace("label.nvs", "Tag.nvs")
    );
    let (_, err, ok) = run(&dir, "check", &["public/labels.nvs"]);
    assert!(ok, "the moved project still checks: {err}");
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

// covers: tools:cli/nvs-code
#[test]
fn reach_of_several_methods_prints_each_under_its_name() {
    let dir = calls_project("reach-several");
    let (out, err, ok) = code(
        &dir,
        &["reach", "Prices::plain", "Prices::base", "Prices::plain"],
    );
    assert!(ok, "{err}");
    let lines: Vec<&str> = out.lines().collect();
    // Each name prints once, in the order given, and its lines are indented.
    assert_eq!(
        lines[..5],
        [
            "Prices::plain",
            "  route  GET /cart",
            "  test   ShopTest::cartLoads",
            "  test   ShopTest::plainIsFive",
            "Prices::base",
        ],
        "{out}"
    );
    assert_eq!(
        lines[5..7],
        ["  route  GET /cart", "  test   ShopTest::cartLoads"],
        "{out}"
    );
    assert_eq!(lines.len(), 8, "{out}");
    assert!(lines[7].starts_with("  at least:"), "{out}");
}

// covers: tools:cli/nvs-code
#[test]
fn calls_of_several_methods_prints_each_under_its_name() {
    let dir = calls_project("calls-several");
    let (out, err, ok) = code(&dir, &["calls", "Prices::plain", "ShopTest::cartLoads"]);
    assert!(ok, "{err}");
    let (plain, _, _) = code(&dir, &["calls", "Prices::plain"]);
    let (loads, _, _) = code(&dir, &["calls", "ShopTest::cartLoads"]);
    let mut expected = String::from("Prices::plain\n");
    for line in plain.lines() {
        expected.push_str(&format!("  {line}\n"));
    }
    expected.push_str("ShopTest::cartLoads\n");
    for line in loads.lines() {
        expected.push_str(&format!("  {line}\n"));
    }
    assert_eq!(out, expected);
    assert_eq!(
        words(&out)[..3],
        [
            "Prices::plain",
            "Shop::cart caller",
            "app.nvs:20:18 echo $p->plain();"
        ],
        "{out}"
    );
}

// covers: tools:cli/nvs-code
#[test]
fn several_methods_in_json_are_a_methods_list() {
    let dir = calls_project("several-json");
    let (out, err, ok) = code(&dir, &["reach", "--json", "Prices::base", "Prices::plain"]);
    assert!(ok, "{err}");
    let mut base = parsed(&code(&dir, &["reach", "--json", "Prices::base"]).0);
    let mut plain = parsed(&code(&dir, &["reach", "--json", "Prices::plain"]).0);
    for one in [&mut base, &mut plain] {
        one.as_object_mut()
            .expect("an object")
            .remove("schemaVersion");
    }
    assert_eq!(
        parsed(&out),
        serde_json::json!({ "schemaVersion": 1, "methods": [base, plain] }),
        "{out}"
    );

    let (out, err, ok) = code(&dir, &["calls", "--json", "Shop::cart", "Prices::base"]);
    assert!(ok, "{err}");
    let answer = parsed(&out);
    assert_eq!(answer["schemaVersion"], 1, "{out}");
    let methods = answer["methods"].as_array().expect("a methods list");
    assert_eq!(methods.len(), 2, "{out}");
    assert_eq!(methods[0]["symbol"], "Shop::cart", "{out}");
    assert_eq!(methods[1]["symbol"], "Prices::base", "{out}");
    assert!(methods[0].get("schemaVersion").is_none(), "{out}");
    assert!(methods[0].get("callers").is_some(), "{out}");
}

// covers: tools:cli/nvs-code
#[test]
fn one_unknown_method_among_several_fails_the_call_and_names_it() {
    let dir = calls_project("several-unknown");
    let (out, err, ok) = code(&dir, &["calls", "Prices::base", "Prices::bse", "Prices"]);
    assert!(!ok);
    // Nothing prints for the name that is a method.
    assert_eq!(out, "");
    assert!(
        err.starts_with("error: no methods `Prices::bse`, `Prices` in this project\n"),
        "{err}"
    );
    assert!(
        err.contains("nearest to Prices::bse:\n  Prices::base\n"),
        "{err}"
    );
    assert!(err.contains("nearest to Prices:\n"), "{err}");

    let (out, _, ok) = code(&dir, &["reach", "--json", "Prices::plain", "Prices::bse"]);
    assert!(!ok);
    let answer = parsed(&out);
    assert_eq!(answer["schemaVersion"], 1, "{out}");
    assert_eq!(
        answer["error"], "no method `Prices::bse` in this project",
        "{out}"
    );
    let unknown = answer["unknown"].as_array().expect("an unknown list");
    assert_eq!(unknown.len(), 1, "{out}");
    assert_eq!(unknown[0]["symbol"], "Prices::bse", "{out}");
    assert_eq!(unknown[0]["nearest"][0], "Prices::base", "{out}");
    assert!(answer.get("methods").is_none(), "{out}");
}

// covers: tools:cli/nvs-code
#[test]
fn one_method_prints_what_it_printed_before() {
    let dir = calls_project("one-method");
    // One name keeps the output with no heading and no indent.
    let (out, err, ok) = code(&dir, &["reach", "Prices::plain"]);
    assert!(ok, "{err}");
    assert_eq!(
        out,
        "route  GET /cart\ntest   ShopTest::cartLoads\ntest   ShopTest::plainIsFive\n"
    );
    // The same name twice is one name.
    let (twice, _, ok) = code(&dir, &["reach", "Prices::plain", "Prices::plain"]);
    assert!(ok);
    assert_eq!(twice, out);
    let (out, err, ok) = code(&dir, &["calls", "Prices::plain"]);
    assert!(ok, "{err}");
    assert!(
        out.starts_with("Shop::cart  caller\n  app.nvs:20:18  "),
        "{out}"
    );
    // One unknown name keeps its error and its `nearest` list.
    let (out, _, ok) = code(&dir, &["calls", "--json", "Prices::bse"]);
    assert!(!ok);
    let answer = parsed(&out);
    assert_eq!(
        answer["error"], "no method `Prices::bse` in this project",
        "{out}"
    );
    assert_eq!(answer["nearest"][0], "Prices::base", "{out}");
    assert!(answer.get("unknown").is_none(), "{out}");
}

/// A class whose method, property and constant are written in the reverse of
/// the member order.
const BASKET: &str = "<?nvs\nnamespace Shop;\n\nclass Basket\n{\n    \
                      public function count(): int\n    {\n        return $this->size;\n    }\n\n    \
                      private int $size = 0;\n\n    public const int LIMIT = 5;\n}\n";

/// A derived class whose `private` field is written before its `public` one,
/// so ordering it changes its JSON key order.
const RECEIPT: &str = "<?nvs\nnamespace Shop;\n\nuse Core\\Json\\Derive;\n\n#[Derive]\n\
                       class Receipt\n{\n    private string $note;\n    public int $id;\n\n    \
                       public function constructor(int $id, string $note)\n    {\n        \
                       $this->id = $id;\n        $this->note = $note;\n    }\n}\n";

/// A file with a missing `}`.
const BROKEN: &str = "<?nvs\nclass Broken\n{\n    public function run(): void\n    {\n}\n";

/// The `Shop` project with `Basket` beside it, and `extra` files.
fn order_project(name: &str, extra: &[(&str, &str)]) -> nvs_repo::Scratch {
    project(name, &[&[("src/Basket.nvs", BASKET)][..], extra].concat())
}

/// Whether each of `parts` is in `text`, in this order.
fn in_order(text: &str, parts: &[&str]) -> bool {
    let at: Vec<Option<usize>> = parts.iter().map(|part| text.find(part)).collect();
    at.iter().all(Option::is_some) && at.windows(2).all(|pair| pair[0] < pair[1])
}

// covers: tools:cli/nvs-code
#[test]
fn code_order_writes_the_member_order_and_formats_each_file() {
    let dir = order_project("order-write", &[]);
    let before = files(&dir);
    let (out, err, ok) = code(&dir, &["order", "src"]);
    assert!(ok, "{err}");
    assert_eq!(out, "src/Basket.nvs  1 class\n");
    let basket = read(&dir, "src/Basket.nvs");
    assert!(
        in_order(&basket, &["LIMIT", "$size", "function count"]),
        "{basket}"
    );
    // The written file is in the standard layout, and the ordered files are
    // as they were.
    let (_, err, ok) = run(&dir, "fmt", &["--check", "src/Basket.nvs"]);
    assert!(ok, "{err}");
    assert_eq!(files(&dir), before);
    // A second run finds nothing to move.
    let (out, err, ok) = code(&dir, &["order", "--check"]);
    assert!(ok, "{err}");
    assert_eq!(out, "");
}

// covers: tools:cli/nvs-code
#[test]
fn code_order_check_lists_each_class_out_of_order_and_writes_nothing() {
    let dir = order_project("order-check", &[]);
    let (out, _, ok) = code(&dir, &["order", "--check"]);
    assert!(!ok, "a class out of order fails the check: {out}");
    assert_eq!(out, "src/Basket.nvs:4:1  Shop\\Basket\n");
    assert_eq!(read(&dir, "src/Basket.nvs"), BASKET);
}

// covers: tools:cli/nvs-code
#[test]
fn code_order_dry_run_prints_each_class_it_would_move_and_writes_nothing() {
    let dir = order_project("order-dry-run", &[("src/Receipt.nvs", RECEIPT)]);
    let (out, err, ok) = code(&dir, &["order", "--dry-run"]);
    assert!(ok, "{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "two classes and one note: {out}");
    assert!(lines[0].starts_with("src/Basket.nvs:4:1 "), "{out}");
    assert!(lines[0].ends_with("Shop\\Basket"), "{out}");
    assert!(lines[1].ends_with("Shop\\Receipt"), "{out}");
    assert!(lines[2].contains("note:"), "{out}");
    assert_eq!(read(&dir, "src/Basket.nvs"), BASKET);
    assert_eq!(read(&dir, "src/Receipt.nvs"), RECEIPT);
}

// covers: tools:cli/nvs-code
#[test]
fn code_order_prints_a_note_for_a_derived_class_whose_key_order_changes() {
    let dir = order_project("order-note", &[("src/Receipt.nvs", RECEIPT)]);
    let (out, err, ok) = code(&dir, &["order", "src/Receipt.nvs"]);
    assert!(ok, "{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "src/Receipt.nvs  1 class", "{out}");
    assert!(
        lines[1].starts_with("src/Receipt.nvs:6:1  Shop\\Receipt  note:"),
        "{out}"
    );
    assert!(lines[1].contains("JSON key order"), "{out}");
    let receipt = read(&dir, "src/Receipt.nvs");
    assert!(
        in_order(&receipt, &["public int $id;", "private string $note;"]),
        "{receipt}"
    );
    // Only the file the path names is ordered.
    assert_eq!(read(&dir, "src/Basket.nvs"), BASKET);
}

// covers: tools:cli/nvs-code
#[test]
fn code_order_skips_a_file_with_a_syntax_error_and_names_it() {
    let dir = order_project("order-broken", &[("src/Broken.nvs", BROKEN)]);
    let (out, err, ok) = code(&dir, &["order"]);
    assert!(ok, "{err}");
    assert!(err.contains("`src/Broken.nvs` has a syntax error"), "{err}");
    assert_eq!(out, "src/Basket.nvs  1 class\n");
    assert_eq!(read(&dir, "src/Broken.nvs"), BROKEN);
}

// covers: tools:cli/nvs-code
#[test]
fn code_order_refuses_a_file_that_changed_after_it_was_read() {
    let dir = order_project("order-changed", &[]);
    // The debug binary changes this file after it orders it and before it
    // compares the bytes again.
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(&*dir)
        .env(
            "NVS_TEST_CODE_CHANGE_BEFORE_WRITE",
            dir.join("src/Basket.nvs"),
        )
        .arg("--data")
        .arg(dir.join(".nvsdata"))
        .args(["code", "order"])
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let err = String::from_utf8(out.stderr).expect("the output is UTF-8");
    assert!(!out.status.success(), "{err}");
    assert!(err.contains("src/Basket.nvs"), "{err}");
    assert!(err.contains("changed while the command ran"), "{err}");
    // Only the change the hook made is on disk.
    assert_eq!(read(&dir, "src/Basket.nvs"), format!("{BASKET}\n"));
}

// covers: tools:cli/nvs-code
#[test]
fn code_order_json_lists_every_class_and_note() {
    let dir = order_project(
        "order-json",
        &[("src/Receipt.nvs", RECEIPT), ("src/Broken.nvs", BROKEN)],
    );
    let (out, err, ok) = code(&dir, &["order", "--dry-run", "--json"]);
    assert!(ok, "{err}");
    let answer = parsed(&out);
    assert_eq!(answer["schemaVersion"], 1, "{out}");
    assert_eq!(answer["written"], false, "{out}");
    let files = answer["files"].as_array().expect("a files list");
    assert_eq!(files.len(), 2, "{out}");
    assert_eq!(files[0]["path"], "src/Basket.nvs", "{out}");
    let class = &files[0]["classes"][0];
    for key in ["path", "line", "column", "start", "end", "symbol"] {
        assert!(class.get(key).is_some(), "{key}: {out}");
    }
    assert_eq!(class["symbol"], "Shop\\Basket", "{out}");
    assert_eq!(answer["notes"][0]["symbol"], "Shop\\Receipt", "{out}");
    assert_eq!(answer["notes"][0]["note"], "keyOrderChanges", "{out}");
    assert_eq!(
        answer["skipped"],
        serde_json::json!([
            { "path": "src/Broken.nvs", "reason": "syntaxError", "symbol": null },
        ]),
        "{out}"
    );
    assert_eq!(read(&dir, "src/Basket.nvs"), BASKET);

    let (out, _, ok) = code(&dir, &["order", "--check", "--json"]);
    assert!(!ok, "a class out of order fails the check: {out}");
    assert_eq!(parsed(&out)["written"], false, "{out}");
    let (out, err, ok) = code(&dir, &["order", "--json"]);
    assert!(ok, "{err}");
    assert_eq!(parsed(&out)["written"], true, "{out}");
    assert_ne!(read(&dir, "src/Basket.nvs"), BASKET);
}

/// A program whose `Order` class has an unused private property, an unused
/// constant and an unused method, beside a class nothing names.
const ORDERS: &str = "<?nvs\nclass Order {\n    private int $count = 0;\n    \
                      private int $spare = 0;\n    public const int MAX = 3;\n    \
                      public const int MIN = 1;\n\n    \
                      public function add(): int {\n        \
                      $this->count = $this->count + 1;\n        return $this->count;\n    }\n\n    \
                      public function clear(): void {}\n}\n\nclass Coupon {}\n\n\
                      var $order = new Order();\necho $order->add() + Order::MAX;\n";

// covers: tools:cli/nvs-code
#[test]
fn code_unused_lists_every_declaration_nothing_uses() {
    let dir = project("unused", &[("public/orders.nvs", ORDERS)]);
    let (out, err, ok) = code(&dir, &["unused", "public/orders.nvs"]);
    assert!(ok, "{err}");
    assert_eq!(
        words(&out),
        [
            r"public/orders.nvs:4:17 property Order::$spare",
            r"public/orders.nvs:6:22 constant Order::MIN",
            r"public/orders.nvs:13:21 method Order::clear",
            r"public/orders.nvs:16:7 class Coupon",
        ],
        "{out}"
    );
    let (out, err, ok) = code(&dir, &["unused", "public/orders.nvs", "--json"]);
    assert!(ok, "{err}");
    let declarations = parsed(&out)["declarations"].clone();
    assert_eq!(declarations.as_array().map(Vec::len), Some(4), "{out}");
    assert_eq!(declarations[3]["symbol"], "Coupon", "{out}");
}

/// A route, a test, an override reached through its parent, a `Core`
/// interface's method and a name written in a string. Only `Pages::draft`
/// is unused.
const REACHED: &str = "<?nvs\nuse Core\\Test;\n\nclass Pages {\n    \
                       #[Core\\Route(path: '/pages/{id}', method: Core\\Http\\Method::Get)]\n    \
                       #[Core\\Access(allow: Core\\Audience::Public)]\n    \
                       public static function show(uint $id): string { return 'page'; }\n    \
                       private function draft(): string { return 'unfinished'; }\n}\n\n\
                       class PagesTest {\n    #[Test]\n    \
                       public function showWorks(): void {\n        \
                       Test::assertEquals('page', Pages::show(3));\n    }\n}\n\n\
                       abstract class Shape {\n    abstract public function area(): int;\n}\n\n\
                       class Square extends Shape {\n    \
                       public function area(): int { return 4; }\n}\n\n\
                       class Label implements Stringable {\n    \
                       public function toString(): string { return 'label'; }\n}\n\n\
                       class Hooks {\n    public function onSave(): void {}\n}\n\n\
                       function measure(Shape $shape): int { return $shape->area(); }\n\n\
                       echo measure(new Square());\necho new Label();\necho 'onSave';\n";

// covers: tools:cli/nvs-code
#[test]
fn code_unused_counts_routes_tests_overrides_and_strings_as_uses() {
    let dir = project("unused-reached", &[("public/pages.nvs", REACHED)]);
    let (out, err, ok) = code(&dir, &["unused", "public/pages.nvs"]);
    assert!(ok, "{err}");
    assert_eq!(
        words(&out),
        [r"public/pages.nvs:8:22 method Pages::draft"],
        "{out}"
    );
}

/// A package's class: a public method nothing calls and a private one.
const LIBRARY: &str = "<?nvs\nnamespace Acme;\n\nclass Ledger {\n    \
                       public function total(): int { return 0; }\n    \
                       protected function base(): int { return 0; }\n    \
                       private function spare(): int { return 0; }\n}\n";

// covers: tools:cli/nvs-code
#[test]
fn code_unused_skips_the_public_api_of_a_package() {
    let dir = project(
        "unused-package",
        &[
            ("package.toml", "[package]\nname = 'Acme\\Ledger'\n"),
            ("src/Ledger.nvs", LIBRARY),
        ],
    );
    let (out, err, ok) = code(&dir, &["unused", "src/Ledger.nvs"]);
    assert!(ok, "{err}");
    assert_eq!(
        words(&out),
        [r"src/Ledger.nvs:7:22 method Acme\Ledger::spare"],
        "{out}"
    );
}

/// A class whose `clear` nothing calls, with a doc comment above it.
const BOOK: &str = "<?nvs\nnamespace Shop;\n\nclass Book {\n    \
                    public function total(): int {\n        return 1;\n    }\n\n    \
                    /// Clears the book.\n    public function clear(): void {}\n}\n";

/// `BOOK` once `clear` is deleted.
const BOOK_CLEARED: &str = "<?nvs\nnamespace Shop;\n\nclass Book {\n    \
                            public function total(): int {\n        return 1;\n    }\n}\n";

/// An entry point that calls `Book::total`.
const BOOKS: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Book;\n\n\
                     echo (new Book())->total();\n";

// covers: tools:cli/nvs-code
#[test]
fn code_delete_deletes_an_unused_method_with_its_doc_comment() {
    let dir = project(
        "delete-method",
        &[("src/Book.nvs", BOOK), ("public/books.nvs", BOOKS)],
    );
    let (out, err, ok) = code(&dir, &["delete", r"Shop\Book::clear", "--dry-run"]);
    assert!(ok, "{err}");
    assert_eq!(read(&dir, "src/Book.nvs"), BOOK, "{out}");
    let (out, err, ok) = code(&dir, &["delete", r"Shop\Book::clear"]);
    assert!(ok, "{err}");
    assert!(out.contains("src/Book.nvs  1 edit"), "{out}");
    assert_eq!(read(&dir, "src/Book.nvs"), BOOK_CLEARED, "{out}");
    let (_, err, ok) = run(&dir, "check", &["public/books.nvs"]);
    assert!(ok, "the program still checks: {err}");
}

/// A class alone in its file that nothing uses.
const COUPON: &str = "<?nvs\nnamespace Shop;\n\n/// A discount.\nclass Coupon {\n    \
                      public function percent(): int {\n        return 5;\n    }\n}\n";

// covers: tools:cli/nvs-code
#[test]
fn code_delete_deletes_the_file_of_an_unused_type() {
    let dir = project("delete-type", &[("src/Coupon.nvs", COUPON)]);
    let (out, err, ok) = code(&dir, &["delete", r"Shop\Coupon", "--dry-run"]);
    assert!(ok, "{err}");
    assert!(out.contains("delete src/Coupon.nvs"), "{out}");
    assert!(dir.join("src/Coupon.nvs").exists());
    let (out, err, ok) = code(&dir, &["delete", r"Shop\Coupon", "--json"]);
    assert!(ok, "{err}");
    let answer = parsed(&out);
    assert_eq!(
        answer["deleted"],
        serde_json::json!(["src/Coupon.nvs"]),
        "{out}"
    );
    assert_eq!(answer["written"], true, "{out}");
    assert!(!dir.join("src/Coupon.nvs").exists());
    let (_, err, ok) = run(&dir, "check", &["public/index.nvs"]);
    assert!(ok, "the program still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_delete_refuses_a_declaration_something_uses_and_writes_nothing() {
    let dir = project("delete-refused", &[]);
    let (out, err, ok) = code(&dir, &["delete", r"Shop\Cart::addItem"]);
    assert!(!ok, "{out}");
    assert!(err.contains("is used at index.nvs:9"), "{out}{err}");
    // Nothing calls `Cart::price` itself, but it implements `Priced::price`
    // and `GiftCart::price` overrides it, so another body would run.
    let (out, err, ok) = code(&dir, &["delete", r"Shop\Cart::price"]);
    assert!(!ok, "{out}");
    assert!(err.contains("overrides a method"), "{out}{err}");
    for (path, text) in [("src/Cart.nvs", CART), ("src/GiftCart.nvs", GIFT_CART)] {
        assert_eq!(read(&dir, path), text);
    }
}

/// A class whose `total` multiplies on line 6, columns 20 to 34, and whose
/// `capped` returns early from the `if` on lines 11 to 13.
const TILL: &str = "<?nvs\nnamespace Shop;\n\nclass Till {\n    \
                    public function total(int $price, int $count): int {\n        \
                    var $sum = $price * $count + 2;\n        return $sum;\n    }\n\n    \
                    public function capped(int $price): int {\n        \
                    if ($price > 100) {\n            return 100;\n        }\n        \
                    return $price;\n    }\n}\n";

/// An entry point that calls both methods of `Till`.
const TILLS: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Till;\n\n\
                     var $till = new Till();\necho $till->total(3, 4) + $till->capped(200);\n";

// covers: tools:cli/nvs-code
#[test]
fn code_extract_variable_writes_a_typed_local() {
    let dir = project(
        "extract-variable",
        &[("src/Till.nvs", TILL), ("public/tills.nvs", TILLS)],
    );
    let range = "src/Till.nvs:6:20-6:34";
    let (out, err, ok) = code(&dir, &["extract", "variable", range, "--dry-run"]);
    assert!(ok, "{err}");
    assert!(out.contains("int $value = $price * $count;"), "{out}");
    assert_eq!(read(&dir, "src/Till.nvs"), TILL, "{out}");
    let (out, err, ok) = code(&dir, &["extract", "variable", range, "--name", "subtotal"]);
    assert!(ok, "{err}");
    assert!(out.contains("src/Till.nvs"), "{out}");
    let written = read(&dir, "src/Till.nvs");
    assert!(
        written.contains(
            "        int $subtotal = $price * $count;\n        var $sum = $subtotal + 2;\n"
        ),
        "{written}"
    );
    let (_, err, ok) = run(&dir, "check", &["public/tills.nvs"]);
    assert!(ok, "the program still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_extract_method_writes_a_private_method() {
    let dir = project(
        "extract-method",
        &[("src/Till.nvs", TILL), ("public/tills.nvs", TILLS)],
    );
    let (out, err, ok) = code(
        &dir,
        &[
            "extract",
            "method",
            "src/Till.nvs:6:20-6:34",
            "--name",
            "product",
            "--json",
        ],
    );
    assert!(ok, "{err}");
    assert_eq!(parsed(&out)["written"], true, "{out}");
    let written = read(&dir, "src/Till.nvs");
    assert!(
        written.contains("        var $sum = $this->product($price, $count) + 2;\n"),
        "{written}"
    );
    assert!(
        written.contains(
            "    }\n\n    private function product(int $price, int $count): int {\n        \
             return $price * $count;\n    }\n"
        ),
        "{written}"
    );
    let (_, err, ok) = run(&dir, "check", &["public/tills.nvs"]);
    assert!(ok, "the program still checks: {err}");
}

// covers: tools:cli/nvs-code
#[test]
fn code_extract_refuses_a_selection_with_an_early_return_and_says_why() {
    let dir = project("extract-refused", &[("src/Till.nvs", TILL)]);
    let (out, err, ok) = code(&dir, &["extract", "method", "src/Till.nvs:11:9-13:9"]);
    assert!(!ok, "{out}");
    assert!(err.contains("has a `return`"), "{out}{err}");
    // A list of statements is no expression, so it cannot become a variable.
    let (out, err, ok) = code(&dir, &["extract", "variable", "src/Till.nvs:11:9-13:9"]);
    assert!(!ok, "{out}");
    assert!(err.contains("Only one expression"), "{out}{err}");
    let (out, err, ok) = code(&dir, &["extract", "method", "src/Till.nvs:6:9"]);
    assert!(!ok, "{out}");
    assert!(err.contains("is not a range"), "{out}{err}");
    assert_eq!(read(&dir, "src/Till.nvs"), TILL);
}

/// A class that implements `Priced` and does not declare `price`.
const TAG: &str = "<?nvs\nnamespace Shop;\n\nclass Tag implements Priced {\n    \
                   public function label(): string {\n        return 'tag';\n    }\n}\n";

/// An entry point that asks a `Tag` for its price.
const TAGS: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Tag;\n\n\
                    echo (new Tag())->price();\n";

// covers: tools:cli/nvs-code
#[test]
fn code_implement_writes_every_missing_method() {
    let dir = project(
        "implement",
        &[("src/Tag.nvs", TAG), ("public/tags.nvs", TAGS)],
    );
    let (_, _, ok) = run(&dir, "check", &["public/tags.nvs"]);
    assert!(!ok, "`Tag` does not declare `price` yet");
    let (out, err, ok) = code(&dir, &["implement", r"Shop\Tag"]);
    assert!(ok, "{err}");
    assert!(out.contains("src/Tag.nvs"), "{out}");
    let written = read(&dir, "src/Tag.nvs");
    assert!(
        written.contains("public function price(): int\n"),
        "{written}"
    );
    assert!(
        written.contains("Tag::price is not implemented"),
        "{written}"
    );
    let (_, err, ok) = run(&dir, "check", &["public/tags.nvs"]);
    assert!(ok, "the program checks once `price` is written: {err}");
    let (out, err, ok) = code(&dir, &["implement", r"Shop\Tag"]);
    assert!(!ok, "{out}");
    assert!(err.contains("already has every method"), "{out}{err}");
}

/// A class whose `gross` declares `$sum` on line 10, whose `label` writes
/// `$text` twice on lines 15 and 16, and whose `taxed` is called on line 6,
/// column 23, and on line 11.
const TAXED: &str = "<?nvs\nnamespace Shop;\n\nclass Order {\n    \
                     public function net(int $gross): int {\n        \
                     return $this->taxed($gross, 2) - 1;\n    }\n\n    \
                     public function gross(int $amount): int {\n        \
                     int $sum = $amount + 1;\n        \
                     return $this->taxed($sum) + $sum;\n    }\n\n    \
                     public function label(): string {\n        \
                     var $text = 'items';\n        $text = 'item';\n        \
                     return $text;\n    }\n\n    \
                     private function taxed(int $amount, int $rate = 3): int {\n        \
                     return $amount * $rate;\n    }\n}\n";

/// An entry point that prints `net(10) + gross(2)`, which is `31`.
const ORDERS_ENTRY: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Order;\n\n\
                            var $order = new Order();\n\
                            echo $order->net(10) + $order->gross(2);\n";

/// The `Shop` project with `order` as `src/Order.nvs` and an entry point
/// that calls it.
fn order_entry_project(name: &str, order: &str) -> nvs_repo::Scratch {
    project(
        name,
        &[
            ("src/Order.nvs", order),
            ("public/orders.nvs", ORDERS_ENTRY),
        ],
    )
}

/// Runs the entry point of an [`order_entry_project`] and checks it prints
/// what it printed before the edit.
fn still_prints_31(dir: &Path) {
    let (out, err, ok) = run(dir, "run", &["public/orders.nvs"]);
    assert!(ok, "the inlined project still runs: {err}");
    assert_eq!(out, "31");
}

// covers: tools:cli/nvs-code
#[test]
fn code_inline_replaces_each_read_of_a_local_and_deletes_it() {
    let dir = order_entry_project("inline-local", TAXED);
    let (out, err, ok) = code(&dir, &["inline", "src/Order.nvs:10:13"]);
    assert!(ok, "{err}");
    assert!(out.contains("src/Order.nvs"), "{out}");
    let order = read(&dir, "src/Order.nvs");
    assert!(!order.contains("$sum"), "{order}");
    // The second read is part of a sum, so it is put in parentheses.
    assert!(
        order.contains(
            "    public function gross(int $amount): int {\n        \
             return $this->taxed($amount + 1) + ($amount + 1);\n    }\n"
        ),
        "{order}"
    );
    still_prints_31(&dir);
}

// covers: tools:cli/nvs-code
#[test]
fn code_inline_refuses_a_local_written_twice() {
    let dir = order_entry_project("inline-written-twice", TAXED);
    let (out, err, ok) = code(&dir, &["inline", "src/Order.nvs:15:13"]);
    assert!(!ok, "{out}");
    assert!(err.contains("`$text` is changed on line 16"), "{out}{err}");
    assert_eq!(read(&dir, "src/Order.nvs"), TAXED);
}

// covers: tools:cli/nvs-code
#[test]
fn code_inline_of_a_method_inlines_every_call_and_deletes_it() {
    let dir = order_entry_project("inline-method", TAXED);
    let (out, err, ok) = code(&dir, &["inline", r"Shop\Order::taxed", "--json"]);
    assert!(ok, "{err}");
    assert_eq!(parsed(&out)["written"], true, "{out}");
    let order = read(&dir, "src/Order.nvs");
    // The missing `$rate` takes its default, `3`.
    assert!(
        order.contains("        return ($gross * 2) - 1;\n"),
        "{order}"
    );
    assert!(
        order.contains("        return ($sum * 3) + $sum;\n"),
        "{order}"
    );
    assert!(!order.contains("taxed"), "{order}");
    assert!(
        order.ends_with("        return $text;\n    }\n}\n"),
        "{order}"
    );
    still_prints_31(&dir);
}

// covers: tools:cli/nvs-code
#[test]
fn code_inline_at_one_call_keeps_the_method() {
    let dir = order_entry_project("inline-one-call", TAXED);
    let (_, err, ok) = code(&dir, &["inline", "src/Order.nvs:6:23"]);
    assert!(ok, "{err}");
    let order = read(&dir, "src/Order.nvs");
    assert!(
        order.contains("        return ($gross * 2) - 1;\n"),
        "{order}"
    );
    assert!(
        order.contains("return $this->taxed($sum) + $sum;"),
        "{order}"
    );
    assert!(order.contains("private function taxed("), "{order}");
    still_prints_31(&dir);
}

// covers: tools:cli/nvs-code
#[test]
fn code_inline_refuses_an_overridden_method() {
    let fee = "<?nvs\nnamespace Shop;\n\nclass Fee {\n    \
               public function rate(): int {\n        return 2;\n    }\n}\n";
    let flat = "<?nvs\nnamespace Shop;\n\nclass FlatFee extends Fee {\n    \
                public function rate(): int {\n        return 3;\n    }\n}\n";
    let entry = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Fee;\n\n\
                 echo (new Fee())->rate();\n";
    let dir = project(
        "inline-overridden",
        &[
            ("src/Fee.nvs", fee),
            ("src/FlatFee.nvs", flat),
            ("public/fees.nvs", entry),
        ],
    );
    let (out, err, ok) = code(&dir, &["inline", r"Shop\Fee::rate"]);
    assert!(!ok, "{out}");
    assert!(err.contains("is overridden"), "{out}{err}");
    assert_eq!(read(&dir, "src/Fee.nvs"), fee);
}

// covers: tools:cli/nvs-code
#[test]
fn code_inline_refuses_an_argument_that_is_not_a_variable_or_a_literal() {
    let order = TAXED.replace("$this->taxed($gross, 2)", "$this->taxed($gross + 1, 2)");
    let dir = order_entry_project("inline-argument", &order);
    let (out, err, ok) = code(&dir, &["inline", r"Shop\Order::taxed"]);
    assert!(!ok, "{out}");
    assert!(
        err.contains("`$gross + 1` on line 6 is not a variable or a literal"),
        "{out}{err}"
    );
    assert_eq!(read(&dir, "src/Order.nvs"), order);
}

/// What the entry point of an [`order_entry_project`] prints.
fn orders_print(dir: &Path) -> String {
    let (out, err, ok) = run(dir, "run", &["public/orders.nvs"]);
    assert!(ok, "the project runs: {err}");
    out
}

// covers: tools:cli/nvs-code
#[test]
fn code_signature_adds_a_parameter_with_a_default_and_leaves_callers() {
    let dir = order_entry_project("signature-default", TAXED);
    let (out, err, ok) = code(
        &dir,
        &[
            "signature",
            r"Shop\Order::taxed",
            "int $amount, int $rate = 3, int $extra = 0",
        ],
    );
    assert!(ok, "{err}");
    assert!(out.contains("src/Order.nvs"), "{out}");
    let order = read(&dir, "src/Order.nvs");
    assert!(
        order.contains("private function taxed(int $amount, int $rate = 3, int $extra = 0): int"),
        "{order}"
    );
    assert!(order.contains("$this->taxed($gross, 2)"), "{order}");
    assert!(order.contains("$this->taxed($sum)"), "{order}");
    still_prints_31(&dir);
}

// covers: tools:cli/nvs-code
#[test]
fn code_signature_adds_a_parameter_with_a_value_at_every_call() {
    let dir = order_entry_project("signature-value", TAXED);
    let (_, err, ok) = code(
        &dir,
        &[
            "signature",
            r"Shop\Order::taxed",
            "int $amount, int $extra, int $rate = 3",
            "--value",
            "$extra=0",
        ],
    );
    assert!(ok, "{err}");
    let order = read(&dir, "src/Order.nvs");
    assert!(order.contains("$this->taxed($gross, 0, 2)"), "{order}");
    assert!(order.contains("$this->taxed($sum, 0)"), "{order}");
    still_prints_31(&dir);
}

// covers: tools:cli/nvs-code
#[test]
fn code_signature_removes_a_parameter_and_its_argument() {
    let order = TAXED.replace("return $amount * $rate;", "return $amount * 2;");
    let dir = order_entry_project("signature-remove", &order);
    let before = orders_print(&dir);
    let (_, err, ok) = code(&dir, &["signature", r"Shop\Order::taxed", "int $amount"]);
    assert!(ok, "{err}");
    let written = read(&dir, "src/Order.nvs");
    assert!(
        written.contains("private function taxed(int $amount): int"),
        "{written}"
    );
    assert!(written.contains("$this->taxed($gross) - 1"), "{written}");
    assert!(written.contains("$this->taxed($sum) + $sum"), "{written}");
    assert_eq!(orders_print(&dir), before);
}

// covers: tools:cli/nvs-code
#[test]
fn code_signature_reorders_positional_arguments_and_keeps_named_ones() {
    let order = TAXED.replace("$this->taxed($sum)", "$this->taxed($sum, rate: 3)");
    let dir = order_entry_project("signature-reorder", &order);
    let (_, err, ok) = code(
        &dir,
        &["signature", r"Shop\Order::taxed", "int $rate, int $amount"],
    );
    assert!(ok, "{err}");
    let written = read(&dir, "src/Order.nvs");
    assert!(
        written.contains("private function taxed(int $rate, int $amount): int"),
        "{written}"
    );
    assert!(written.contains("$this->taxed(2, $gross)"), "{written}");
    // A named argument keeps its name, and so must every argument after it.
    assert!(
        written.contains("$this->taxed(rate: 3, amount: $sum)"),
        "{written}"
    );
    still_prints_31(&dir);
}

// covers: tools:cli/nvs-code
#[test]
fn code_signature_changes_every_method_of_the_override_family() {
    let fee = "<?nvs\nnamespace Shop;\n\nclass Fee {\n    \
               /// @param $base The amount the fee is for.\n    \
               /// @param $round Unused.\n    \
               public function rate(int $base, bool $round): int {\n        \
               return $base + 2;\n    }\n}\n";
    let flat = "<?nvs\nnamespace Shop;\n\nclass FlatFee extends Fee {\n    \
                public function rate(int $amount, bool $round): int {\n        \
                return $amount + 3;\n    }\n}\n";
    let entry = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Fee;\nuse Shop\\FlatFee;\n\n\
                 echo (new Fee())->rate(1, true) + (new FlatFee())->rate(1, false);\n";
    let dir = project(
        "signature-family",
        &[
            ("src/Fee.nvs", fee),
            ("src/FlatFee.nvs", flat),
            ("public/fees.nvs", entry),
        ],
    );
    let (_, err, ok) = code(
        &dir,
        &["signature", r"Shop\Fee::rate", "int $base, int $extra = 0"],
    );
    assert!(ok, "{err}");
    let fee_after = read(&dir, "src/Fee.nvs");
    assert!(
        fee_after.contains("public function rate(int $base, int $extra = 0): int"),
        "{fee_after}"
    );
    assert!(fee_after.contains("@param $base"), "{fee_after}");
    assert!(!fee_after.contains("@param $round"), "{fee_after}");
    // The override keeps its own name for the first parameter.
    let flat_after = read(&dir, "src/FlatFee.nvs");
    assert!(
        flat_after.contains("public function rate(int $amount, int $extra = 0): int"),
        "{flat_after}"
    );
    assert!(
        read(&dir, "public/fees.nvs").contains("(new Fee())->rate(1) + (new FlatFee())->rate(1)"),
        "the literal arguments are removed"
    );
    let (out, err, ok) = run(&dir, "run", &["public/fees.nvs"]);
    assert!(ok, "the changed project runs: {err}");
    assert_eq!(out, "7");
}

// covers: tools:cli/nvs-code
#[test]
fn code_signature_refuses_removing_an_argument_that_runs_code() {
    let order = TAXED
        .replace("return $amount * $rate;", "return $amount * 2;")
        .replace("$this->taxed($gross, 2)", "$this->taxed($gross, $this->two())")
        .replace(
            "    private function taxed",
            "    private function two(): int {\n        return 2;\n    }\n\n    private function taxed",
        );
    let dir = order_entry_project("signature-runs-code", &order);
    let (out, err, ok) = code(&dir, &["signature", r"Shop\Order::taxed", "int $amount"]);
    assert!(!ok, "{out}");
    assert!(err.contains("`$this->two()` on line 6 of"), "{out}{err}");
    assert!(err.contains("may run code"), "{out}{err}");
    assert_eq!(read(&dir, "src/Order.nvs"), order);
}

/// A class with a constant and a static method that reads it.
const TAX: &str = "<?nvs\nnamespace Shop;\n\nclass Tax {\n    \
                   public const int RATE = 20;\n\n    \
                   /// The tax on an amount.\n    \
                   public static function of(int $amount): int {\n        \
                   return $amount * self::RATE;\n    }\n}\n";

/// A class in another namespace that members of [`TAX`] move to.
const MONEY: &str = "<?nvs\nnamespace Shop\\Util;\n\nclass Money {\n    \
                     public static function zero(): int {\n        return 0;\n    }\n}\n";

/// The entry point of a [`tax_project`], which prints `100`.
const TAX_ENTRY: &str = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Tax;\n\n\
                         echo Tax::of(2) + Tax::of(3);\n";

/// A project with [`TAX`] as written by `tax`, [`MONEY`] and [`TAX_ENTRY`].
fn tax_project(name: &str, tax: &str) -> nvs_repo::Scratch {
    project(
        name,
        &[
            ("src/Tax.nvs", tax),
            ("src/Util/Money.nvs", MONEY),
            ("public/billing.nvs", TAX_ENTRY),
        ],
    )
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_member_moves_a_static_method_and_rewrites_every_call() {
    let dir = tax_project("move-member-static", TAX);
    let (out, err, ok) = code(&dir, &["move", r"Shop\Tax::of", r"Shop\Util\Money"]);
    assert!(ok, "{out}{err}");
    let money = read(&dir, "src/Util/Money.nvs");
    assert!(money.contains("use Shop\\Tax;"), "{money}");
    assert!(money.contains("    /// The tax on an amount.\n    public static function of(int $amount): int {\n        return $amount * Tax::RATE;\n    }\n"), "{money}");
    let tax = read(&dir, "src/Tax.nvs");
    assert!(!tax.contains("function of"), "{tax}");
    assert!(!tax.contains("The tax on"), "{tax}");
    let entry = read(&dir, "public/billing.nvs");
    assert!(entry.contains("use Shop\\Util\\Money;"), "{entry}");
    assert!(
        entry.contains("echo Money::of(2) + Money::of(3);"),
        "{entry}"
    );
    let (out, err, ok) = run(&dir, "run", &["public/billing.nvs"]);
    assert!(ok, "the changed project runs: {err}");
    assert_eq!(out, "100");
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_member_moves_a_constant_and_qualifies_its_self_reads() {
    let dir = tax_project("move-member-constant", TAX);
    let (out, err, ok) = code(&dir, &["move", r"Shop\Tax::RATE", r"Shop\Util\Money"]);
    assert!(ok, "{out}{err}");
    let money = read(&dir, "src/Util/Money.nvs");
    assert!(
        money.contains("    public const int RATE = 20;\n"),
        "{money}"
    );
    let tax = read(&dir, "src/Tax.nvs");
    assert!(tax.contains("use Shop\\Util\\Money;"), "{tax}");
    assert!(tax.contains("return $amount * Money::RATE;"), "{tax}");
    assert!(!tax.contains("const int RATE"), "{tax}");
    let (out, err, ok) = run(&dir, "run", &["public/billing.nvs"]);
    assert!(ok, "the changed project runs: {err}");
    assert_eq!(out, "100");
}

/// A parent class with one property and one method.
const DOCUMENT: &str = "<?nvs\nnamespace Shop;\n\nclass Document {\n    \
                        protected string $number = '7';\n\n    \
                        public function stamp(): string {\n        return 'paid';\n    }\n}\n";

/// A child of [`DOCUMENT`] with a method that reads the parent's property.
const BILLED_INVOICE: &str = "<?nvs\nnamespace Shop;\n\nclass Invoice extends Document {\n    \
                       public function title(): string {\n        \
                       return 'Invoice ' . $this->number;\n    }\n}\n";

// covers: tools:cli/nvs-code
#[test]
fn code_move_member_pulls_an_instance_method_up_to_its_parent() {
    let entry = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Invoice;\n\n\
                 echo (new Invoice())->title();\n";
    let dir = project(
        "move-member-pull-up",
        &[
            ("src/Document.nvs", DOCUMENT),
            ("src/Invoice.nvs", BILLED_INVOICE),
            ("public/documents.nvs", entry),
        ],
    );
    let (out, err, ok) = code(&dir, &["move", r"Shop\Invoice::title", r"Shop\Document"]);
    assert!(ok, "{out}{err}");
    let document = read(&dir, "src/Document.nvs");
    assert!(
        document.contains("public function title(): string"),
        "{document}"
    );
    assert!(
        !read(&dir, "src/Invoice.nvs").contains("title"),
        "the child no longer has it"
    );
    assert_eq!(
        read(&dir, "public/documents.nvs"),
        entry,
        "a call on an object stays"
    );
    let (out, err, ok) = run(&dir, "run", &["public/documents.nvs"]);
    assert!(ok, "the changed project runs: {err}");
    assert_eq!(out, "Invoice 7");
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_member_pushes_a_method_down_when_nothing_reaches_it_through_the_parent() {
    let entry = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Invoice;\n\n\
                 echo (new Invoice())->stamp();\n";
    let dir = project(
        "move-member-push-down",
        &[
            ("src/Document.nvs", DOCUMENT),
            ("src/Invoice.nvs", BILLED_INVOICE),
            ("public/documents.nvs", entry),
        ],
    );
    let (out, err, ok) = code(&dir, &["move", r"Shop\Document::stamp", r"Shop\Invoice"]);
    assert!(ok, "{out}{err}");
    assert!(
        !read(&dir, "src/Document.nvs").contains("stamp"),
        "the parent no longer has it"
    );
    let invoice = read(&dir, "src/Invoice.nvs");
    assert!(
        invoice.contains("public function stamp(): string"),
        "{invoice}"
    );
    let (out, err, ok) = run(&dir, "run", &["public/documents.nvs"]);
    assert!(ok, "the changed project runs: {err}");
    assert_eq!(out, "paid");

    // A sibling still calls it, so the same move is refused.
    let receipt = "<?nvs\nnamespace Shop;\n\nclass Receipt extends Document {\n}\n";
    let sibling = "<?nvs\nautoload 'Shop' from '../src';\n\nuse Shop\\Receipt;\n\n\
                   echo (new Receipt())->stamp();\n";
    let dir = project(
        "move-member-push-down-sibling",
        &[
            ("src/Document.nvs", DOCUMENT),
            ("src/Invoice.nvs", BILLED_INVOICE),
            ("src/Receipt.nvs", receipt),
            ("public/documents.nvs", sibling),
        ],
    );
    let (out, err, ok) = code(&dir, &["move", r"Shop\Document::stamp", r"Shop\Invoice"]);
    assert!(!ok, "{out}");
    assert!(err.contains("would not compile"), "{out}{err}");
    assert_eq!(read(&dir, "src/Document.nvs"), DOCUMENT);
    assert_eq!(read(&dir, "src/Invoice.nvs"), BILLED_INVOICE);
}

// covers: tools:cli/nvs-code
#[test]
fn code_move_member_refuses_a_private_member_its_class_still_uses() {
    let tax = TAX.replace("public const int RATE", "private const int RATE");
    let dir = tax_project("move-member-private", &tax);
    let (out, err, ok) = code(&dir, &["move", r"Shop\Tax::RATE", r"Shop\Util\Money"]);
    assert!(!ok, "{out}");
    assert!(err.contains("Make it public first"), "{out}{err}");
    assert_eq!(read(&dir, "src/Tax.nvs"), tax);
    assert_eq!(read(&dir, "src/Util/Money.nvs"), MONEY);
}
