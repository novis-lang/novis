//! `nvs check --todos`, `--deny` and `--fix`, driven through the built binary,
//! because `rule:tooling/a-todo-is-a-comment-the-tools-list` promises what the
//! command prints, what it writes and how it exits.
//!
//! Every program here is written into a scratch directory of its own, because
//! `--fix` writes to the files it checks.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// `nvs check <args...> <path>`, as `(stdout, stderr, success)`.
fn run_check(args: &[&str], path: &Path) -> (String, String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("check")
        .args(args)
        .arg(path)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the output is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// A library whose `Api::cents` is deprecated, with a replacement that names
/// `Money`, a class a caller in another namespace has to import.
const SHOP: &str = r#"<?nvs
namespace Shop;
class Money { public static function of(int $c): int { return $c; } }
class Api {
  #[Core\Deprecated(replace: 'Money::of($cents)')]
  public static function cents(int $cents): int { return Money::of($cents); }
}
class Legacy {
  public function total(): int { return Api::cents(3); }
}
"#;

/// Two uses of `Api::cents` in a file that imports `Api` and not `Money`.
fn page(shop: &str) -> String {
    format!(
        r#"<?nvs
namespace Blog;
require '{shop}';
use Shop\Api;
class Page {{
  public function total(): int {{ return Api::cents(5); }}
  public function more(): int {{ return Api::cents(7); }}
}}
"#
    )
}

/// A program with two todos, one of them in a file it requires.
const TODOS: &str = r#"<?nvs
require './b.nvs';
// TODO: page through the results.
echo "a\n";
// todo: a lower-case one is not a todo.
// TODO: send the mail later.
/// TODO: neither is a doc comment.
class Note {}
"#;

const TODOS_B: &str = r#"<?nvs
// TODO: read the limit from the request.
echo "b\n";
"#;

/// `files` written under a fresh scratch directory, which is returned.
fn scratch(name: &str, files: &[(&str, &str)]) -> nvs_repo::Scratch {
    let dir = nvs_repo::scratch(name);
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().expect("a file has a directory"))
            .expect("the scratch directory is writable");
        std::fs::write(&path, text).expect("the scratch directory is writable");
    }
    dir
}

#[test]
fn check_deny_deprecated_fails_on_a_deprecated_use() {
    let dir = scratch(
        "check-deny-deprecated",
        &[("shop.nvs", SHOP), ("page.nvs", &page("./shop.nvs"))],
    );
    let page = dir.join("page.nvs");
    let (out, err, ok) = run_check(&[], &page);
    assert!(ok, "a deprecated use is a warning: {err}");
    assert!(err.contains("W1003"), "the use is reported: {err}");

    let (denied_out, denied_err, ok) = run_check(&["--deny", "deprecated"], &page);
    assert!(!ok, "`--deny deprecated` fails the check");
    assert_eq!(denied_out, out, "the flag changes nothing that is printed");
    assert_eq!(denied_err, err, "the flag changes nothing that is printed");

    let (_, _, ok) = run_check(&["--deny", "todo"], &page);
    assert!(ok, "another kind does not fail on a deprecated use");
}

#[test]
fn check_deny_todo_fails_on_a_todo_comment() {
    let dir = scratch("check-deny-todo", &[("a.nvs", TODOS), ("b.nvs", TODOS_B)]);
    let entry = dir.join("a.nvs");
    let (out, err, ok) = run_check(&["--deny", "todo"], &entry);
    assert!(!ok, "a todo fails `--deny todo`: {err}");
    assert_eq!(out, "no errors\n", "without `--todos` no todo is printed");

    let (_, _, ok) = run_check(&["--deny", "deprecated"], &entry);
    assert!(ok, "a todo is not a deprecated use");

    let clean = scratch("check-deny-todo-clean", &[("a.nvs", "<?nvs\necho 1;\n")]);
    let (_, err, ok) = run_check(
        &["--deny", "todo", "--deny", "deprecated"],
        &clean.join("a.nvs"),
    );
    assert!(ok, "a program with neither passes: {err}");
}

#[test]
fn check_deny_ignores_what_is_under_vendor() {
    let dir = scratch(
        "check-deny-vendor",
        &[
            (
                "vendor/shop.nvs",
                &format!("{SHOP}// TODO: a vendored todo.\n"),
            ),
            ("main.nvs", "<?nvs\nrequire './vendor/shop.nvs';\necho 1;\n"),
        ],
    );
    let main = dir.join("main.nvs");
    let (_, err, ok) = run_check(&["--todos"], &main);
    assert!(ok, "{err}");
    assert!(
        err.contains("W1003"),
        "the vendored use is still reported: {err}"
    );
    let (out, err, ok) = run_check(
        &["--todos", "--deny", "deprecated", "--deny", "todo"],
        &main,
    );
    assert!(ok, "what is under `vendor/` is never denied: {err}");
    assert!(
        out.contains("a vendored todo."),
        "it is still listed: {out}"
    );
}

#[test]
fn check_fix_applies_every_safe_fix_until_none_applies() {
    let dir = scratch(
        "check-fix",
        &[("shop.nvs", SHOP), ("page.nvs", &page("./shop.nvs"))],
    );
    let page = dir.join("page.nvs");
    let (out, err, ok) = run_check(&["--fix"], &page);
    assert!(ok, "{err}");

    let fixed = std::fs::read_to_string(&page).expect("the page is still there");
    assert!(
        fixed.contains("Money::of(5)") && fixed.contains("Money::of(7)"),
        "both uses are rewritten:\n{fixed}"
    );
    assert_eq!(
        fixed.matches("use Shop\\Money;").count(),
        1,
        "the import both fixes need is written once:\n{fixed}"
    );
    let library = std::fs::read_to_string(dir.join("shop.nvs")).expect("the library is there");
    assert!(
        library.contains("return Money::of(3);"),
        "every file of the program is fixed:\n{library}"
    );

    assert!(
        out.starts_with("made 4 edits in 2 files\n"),
        "the count comes first: {out}"
    );
    assert!(
        !err.contains("W1003"),
        "the last check finds nothing to fix: {err}"
    );

    let (out, _, ok) = run_check(&["--fix"], &page);
    assert!(ok);
    assert!(
        out.starts_with("made 0 edits in 0 files\n"),
        "a second run has nothing left: {out}"
    );
}

#[test]
fn check_fix_never_writes_under_vendor() {
    let dir = scratch(
        "check-fix-vendor",
        &[
            ("vendor/shop.nvs", SHOP),
            ("page.nvs", &page("./vendor/shop.nvs")),
        ],
    );
    let (out, err, ok) = run_check(&["--fix"], &dir.join("page.nvs"));
    assert!(ok, "{err}");
    assert!(out.starts_with("made 3 edits in 1 file\n"), "{out}");
    assert_eq!(
        std::fs::read_to_string(dir.join("vendor/shop.nvs")).expect("the library is there"),
        SHOP,
        "a vendored file is left exactly as it was"
    );
}

#[test]
fn check_todos_lists_every_todo_in_path_and_line_order() {
    let dir = scratch("check-todos", &[("a.nvs", TODOS), ("b.nvs", TODOS_B)]);
    let (out, err, ok) = run_check(&["--todos"], &dir.join("a.nvs"));
    assert!(ok, "a todo is not a diagnostic: {err}");
    assert_eq!(err, "", "nothing is reported for a todo");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 4, "three todos, then the success line: {out}");
    assert!(
        lines[0].ends_with("a.nvs:3: page through the results."),
        "{out}"
    );
    assert!(lines[1].ends_with("a.nvs:6: send the mail later."), "{out}");
    assert!(
        lines[2].ends_with("b.nvs:2: read the limit from the request."),
        "{out}"
    );
    assert_eq!(lines[3], "no errors");

    let (out, _, _) = run_check(&[], &dir.join("a.nvs"));
    assert_eq!(out, "no errors\n", "without the flag no todo is printed");
}

#[test]
fn check_json_carries_the_todos() {
    let dir = scratch("check-json-todos", &[("a.nvs", TODOS), ("b.nvs", TODOS_B)]);
    let (out, _, ok) = run_check(&["--json", "--todos"], &dir.join("a.nvs"));
    assert!(ok);
    let document: Value = serde_json::from_str(&out).expect("standard output is the document");
    let todos = document["todos"].as_array().expect("`todos` is an array");
    let read: Vec<(u64, &str)> = todos
        .iter()
        .map(|todo| {
            (
                todo["line"].as_u64().expect("a line"),
                todo["text"].as_str().expect("a text"),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (3, "page through the results."),
            (6, "send the mail later."),
            (2, "read the limit from the request."),
        ]
    );
    assert!(
        todos[2]["file"]
            .as_str()
            .expect("a file")
            .ends_with("b.nvs")
    );

    let (out, _, _) = run_check(&["--json"], &dir.join("a.nvs"));
    let document: Value = serde_json::from_str(&out).expect("standard output is the document");
    assert_eq!(
        document["todos"],
        serde_json::json!([]),
        "without the flag the list is empty"
    );
}
