//! `nvs test <file>.nvs`: which methods are tests, the order they are
//! reported in, what fails and what does not, and the exit status, as
//! `docs/reference/tools/10-cli.md` § *nvs test* states them.
//!
//! Through the built binary, because what is asserted is what CI reads off
//! the command: its report and its status.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A directory of the case's own under the target's scratch, emptied first,
/// so two cases running in parallel never share a fixture.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("test-command")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `nvs test` with `args` from `dir`.
fn test_in(dir: &Path, args: &[&str]) -> Output {
    let data = nvs_repo::scratch_private("nvsdata");
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("test")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// Two classes declared out of name order. `ZetaTest` passes one test and
/// skips one; `AlphaTest` has a test that asserts nothing and a method with
/// no attribute.
const SUITE: &str = "<?nvs
use Core\\Test;

final class ZetaTest {
    #[Test]
    public function second(): void {
        Test::assertSame(1 + 1, 2);
    }

    #[Test(skip: \"not written yet\")]
    public function first(): void {
        Test::assertSame(1, 2);
    }
}

final class AlphaTest {
    #[Test]
    public function checksNothing(): void {
    }

    public function helper(): void {
        Test::assertSame(1, 2);
    }
}

echo \"top level\\n\";
";

/// Position of `needle` in `hay`, or a failure naming the report.
fn at(hay: &str, needle: &str) -> usize {
    hay.find(needle)
        .unwrap_or_else(|| panic!("`{needle}` is in the report: {hay}"))
}

/// Classes come in name order and methods in declaration order; a test that
/// asserts nothing fails, a skip is not a failure, a method without
/// `#[Test]` is not run, and one failure makes the status non-zero.
/// `--filter Class::` then runs one class alone, and with only a pass and a
/// skip left the status is `0`.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_runs_each_test_method_and_fails_on_any_failure() {
    let dir = scratch("suite");
    fs::write(dir.join("suite_test.nvs"), SUITE).unwrap();

    let all = test_in(&dir, &["suite_test.nvs"]);
    let stdout = String::from_utf8_lossy(&all.stdout);
    let stderr = String::from_utf8_lossy(&all.stderr);
    assert_eq!(
        all.status.code(),
        Some(1),
        "a failed test fails the run: {stdout}{stderr}"
    );
    assert!(
        at(&stdout, "AlphaTest") < at(&stdout, "ZetaTest"),
        "classes in name order"
    );
    assert!(
        at(&stdout, "second") < at(&stdout, "first"),
        "methods in declaration order"
    );
    assert!(
        at(&stdout, "✗ checksNothing") > 0,
        "a test that asserts nothing fails"
    );
    assert!(
        !stdout.contains("helper"),
        "a method without `#[Test]` is not a test: {stdout}"
    );
    assert!(
        stdout.contains("not written yet"),
        "a skip carries its reason: {stdout}"
    );
    assert!(stdout.contains("1 failed, 1 passed, 1 skipped"), "{stdout}");
    assert!(
        !stdout.contains("top level"),
        "top-level code is not a test: {stdout}"
    );

    let one = test_in(&dir, &["--filter", "ZetaTest::", "suite_test.nvs"]);
    let stdout = String::from_utf8_lossy(&one.stdout);
    let stderr = String::from_utf8_lossy(&one.stderr);
    assert_eq!(
        one.status.code(),
        Some(0),
        "a skip is not a failure: {stdout}{stderr}"
    );
    assert!(
        !stdout.contains("AlphaTest"),
        "the filter selects one class: {stdout}"
    );
    assert!(stdout.contains("0 failed, 1 passed, 1 skipped"), "{stdout}");
}

/// A `#[Test]` method calls an extension of the tree's `[[extension]]` set, and the call runs: the
/// suite is typed against the set's manifests and the run hosts the loaded components. The
/// extension is the conformance fixture `ledger`, under its real pin; one test calls its class and
/// one calls a class from its source section
/// (`rule:packaging/extension-calls-are-statically-typed`).
#[test]
fn nvs_test_runs_a_test_that_calls_a_configured_extension() {
    let dir = scratch("extension");
    let fixtures = nvs_repo::path("tests/conformance/ext/fixtures");
    fs::copy(fixtures.join("ledger.nvsx"), dir.join("shop.nvsx")).expect("the fixture is copied");
    let pin = fs::read_to_string(fixtures.join("ledger.sha256")).expect("the fixture's pin");
    fs::write(
        dir.join("nvs.toml"),
        format!(
            "[[extension]]\npath = 'shop.nvsx'\nsha256 = \"{}\"\n",
            pin.trim()
        ),
    )
    .unwrap();
    fs::write(
        dir.join("ledger_test.nvs"),
        "<?nvs
use Core\\Test;
use Shop\\Ledger;
use Shop\\Ledger\\Receipt;

final class LedgerTest {
    #[Test]
    public function echoesAnInt(): void {
        Test::assertSame(Ledger::echoInt(7), 7);
    }

    #[Test]
    public function writesAReceiptLine(): void {
        Test::assertSame(Receipt::line(\"Tea\", 250), \"Tea: 250 EUR\");
    }
}
",
    )
    .unwrap();

    let out = test_in(&dir, &["ledger_test.nvs"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stdout}{stderr}");
    assert!(stdout.contains("0 failed, 2 passed"), "{stdout}{stderr}");
}

/// A built-in extension is never named in `nvs.toml`, and every core a run
/// starts can still call it. Before, only the suite's own thread had an
/// extension host, so a test that ran on another core threw "not loaded in
/// this process". Eight tests under `--jobs 4` spread over the cores, and
/// the same suite passes under `--jobs 1`.
#[test]
fn every_core_nvs_test_starts_calls_a_built_in_extension() {
    let dir = scratch("built-in-extension");
    let mut suite = String::from("<?nvs\nuse Core\\Test;\nuse Novis\\Intl\\Collator;\n\n");
    suite.push_str("final class CollatorTest {\n");
    for n in 0..8 {
        suite.push_str(&format!(
            "    #[Test]\n    public function sorts{n}(): void {{\n        \
             Test::assertSame(Collator::sort([\"Zimmermann\", \"Ärzte\", \"Adler\"], \"de\"), \
             [\"Adler\", \"Ärzte\", \"Zimmermann\"]);\n    }}\n\n"
        ));
    }
    suite.push_str("}\n");
    fs::write(dir.join("collator_test.nvs"), suite).unwrap();

    for jobs in ["4", "1"] {
        let out = test_in(&dir, &["--jobs", jobs, "collator_test.nvs"]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(0),
            "--jobs {jobs}: {stdout}{stderr}"
        );
        assert!(
            stdout.contains("0 failed, 8 passed"),
            "--jobs {jobs}: {stdout}{stderr}"
        );
    }
}

/// Each test's extension calls are charged to the test's own CPU budget. Two tests each spin the
/// fixture guest `meter` for 600 ms under `[limits] cpu_time = "1s"` and `--jobs 1`, so together
/// they spend more than one budget, and both pass. A budget the second test shared with the first
/// would stop it at the limit.
#[test]
fn each_nvs_test_test_spends_its_own_extension_cpu_budget() {
    let dir = scratch("extension-budget");
    let fixtures = nvs_repo::path("tests/conformance/ext/fixtures");
    fs::copy(fixtures.join("meter.nvsx"), dir.join("meter.nvsx")).expect("the fixture is copied");
    let pin = fs::read_to_string(fixtures.join("meter.sha256")).expect("the fixture's pin");
    fs::write(
        dir.join("nvs.toml"),
        format!(
            "[limits]\ncpu_time = \"1s\"\n\n[[extension]]\npath = 'meter.nvsx'\nsha256 = \"{}\"\n",
            pin.trim()
        ),
    )
    .unwrap();
    fs::write(
        dir.join("meter_test.nvs"),
        "<?nvs
use Core\\Test;
use Shop\\Meter;

final class MeterTest {
    #[Test]
    public function spinsFirst(): void {
        Meter::spin(600);
        Test::assertTrue(true);
    }

    #[Test]
    public function spinsSecond(): void {
        Meter::spin(600);
        Test::assertTrue(true);
    }
}
",
    )
    .unwrap();

    let out = test_in(&dir, &["--jobs", "1", "meter_test.nvs"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stdout}{stderr}");
    assert!(stdout.contains("0 failed, 2 passed"), "{stdout}{stderr}");
}

/// `nvs test` has no `--php` flag: no case is compared against another
/// language, so the flag is refused like any other it does not know, and no
/// case runs.
#[test]
fn nvs_test_refuses_the_php_flag() {
    let dir = scratch("php-flag");
    fs::write(dir.join("suite_test.nvs"), SUITE).unwrap();

    let out = test_in(&dir, &["--php", "php", "suite_test.nvs"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stdout}{stderr}");
    assert!(
        stderr.contains("unexpected argument '--php'"),
        "the flag is named as unknown: {stderr}"
    );
    assert!(!stdout.contains("ZetaTest"), "no test ran: {stdout}");
}

/// A program for `--mutate`: `atLeast` is tested far from its boundary, so
/// its `>=` to `>` mutant survives, and `twice` is tested exactly.
const MUTATED: &str = r#"<?nvs
use Core\Test;

final class Stock {
    public static function atLeast(int $count, int $floor): bool {
        return $count >= $floor;
    }

    public static function twice(int $count): int {
        return $count + $count;
    }
}

final class StockTest {
    #[Test]
    public function comparesFarFromTheFloor(): void {
        Test::assertTrue(Stock::atLeast(10, 2));
    }

    #[Test]
    public function doubles(): void {
        Test::assertSame(Stock::twice(3), 6);
    }
}
"#;

/// A program whose conditions are equalities: one on an `int`, one on an
/// enum, and one on a `?int` whose `null` test narrows what the line after it
/// reads.
const EQUALITIES: &str = r#"<?nvs
use Core\Test;

enum Mode { Off, On }

final class Gate {
    public static function differsFromTen(int $n): bool { return $n != 10; }
    public static function label(int $n): string {
        if ($n == 10) {
            return 'ten';
        }
        return 'other';
    }
    public static function isOff(Mode $m): bool { return $m == Mode::Off; }
    public static function next(?int $n): int {
        if ($n == null) {
            return 0;
        }
        return $n + 1;
    }
}

final class GateTest {
    #[Test]
    public function nine(): void { Test::assertTrue(Gate::differsFromTen(9)); }

    #[Test]
    public function ten(): void { Test::assertSame(Gate::label(10), 'ten'); }

    #[Test]
    public function off(): void { Test::assertTrue(Gate::isOff(Mode::Off)); }

    #[Test]
    public function next(): void { Test::assertSame(Gate::next(1), 2); }
}
"#;

/// An equality is reversed, and an `if` on one is negated, wherever the value
/// it tests is stored the same way on both edges: an `int` and an enum. A
/// `?int` compared with `null` is narrowed to `int` on one edge, so that
/// condition keeps its operator and its direction.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_reverses_an_equality_unless_it_narrows_a_nullable() {
    let dir = scratch("mutate-equalities");
    fs::write(dir.join("gate_test.nvs"), EQUALITIES).unwrap();

    let json = test_in(&dir, &["gate_test.nvs", "--mutate", "--format", "json"]);
    let document: serde_json::Value = serde_json::from_slice(&json.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    let made: Vec<(u64, String)> = document["mutants"]
        .as_array()
        .expect("a `mutants` array")
        .iter()
        .map(|m| {
            (
                m["line"].as_u64().unwrap_or_default(),
                m["change"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    for (line, change) in [
        (7, "`!=` to `==`"),
        (9, "`==` to `!=`"),
        (9, "`condition` to `!condition`"),
        (14, "`==` to `!=`"),
    ] {
        assert!(
            made.iter().any(|(l, c)| *l == line && c == change),
            "no {change} mutant on line {line}: {made:?}"
        );
    }
    assert!(
        !made.iter().any(|(l, _)| *l == 16),
        "the `null` test that narrows a `?int` was changed: {made:?}"
    );
}

/// The entry of a program in three files: `Cart` under `shop/`, `Post` under
/// `blog/`, and the tests beside them.
const AIMED_ENTRY: &str = r#"<?nvs
use Core\Test;

require 'shop/cart.nvs';
require 'blog/post.nvs';

final class AimTest {
    #[Test]
    public function cart(): void {
        Test::assertSame(Cart::total(2, 3), 5);
        Test::assertTrue(Cart::big(10));
    }

    #[Test]
    public function post(): void {
        Test::assertSame(Post::words(4), 5);
    }
}
"#;

const AIMED_CART: &str = r#"<?nvs
final class Cart {
    public static function total(int $a, int $b): int {
        return $a + $b;
    }

    public static function big(int $n): bool {
        return $n > 5;
    }
}
"#;

const AIMED_POST: &str = r#"<?nvs
final class Post {
    public static function words(int $n): int {
        return $n + 1;
    }
}
"#;

/// Writes the aimed program into a new scratch folder.
fn aimed(case: &str, cart: &str, post: &str) -> PathBuf {
    let dir = scratch(case);
    fs::create_dir_all(dir.join("shop")).unwrap();
    fs::create_dir_all(dir.join("blog")).unwrap();
    fs::write(dir.join("aim_test.nvs"), AIMED_ENTRY).unwrap();
    fs::write(dir.join("shop").join("cart.nvs"), cart).unwrap();
    fs::write(dir.join("blog").join("post.nvs"), post).unwrap();
    dir
}

/// Each mutant of a `--mutate --format json` run as `(file, function, key)`.
fn mutant_rows(dir: &Path, mutate: &str) -> Vec<(String, String, String)> {
    let out = test_in(dir, &["aim_test.nvs", mutate, "--format", "json"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    document["mutants"]
        .as_array()
        .expect("a `mutants` array")
        .iter()
        .map(|m| {
            let text = |key: &str| m[key].as_str().unwrap_or_default().to_owned();
            (text("file"), text("function"), text("key"))
        })
        .collect()
}

/// `--mutate=<folder>` changes only the files in that folder, and each
/// mutant names its function and its key.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_aimed_at_a_folder_mutates_only_its_files() {
    let dir = aimed("mutate-aim-folder", AIMED_CART, AIMED_POST);
    let rows = mutant_rows(&dir, "--mutate=shop");
    assert!(!rows.is_empty(), "the folder has mutants");
    assert!(
        rows.iter()
            .all(|(file, function, _)| file.ends_with("cart.nvs") && function.starts_with("Cart::")),
        "only the folder's file is changed: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(_, _, key)| key == "Cart::total#1: `+` to `-`"),
        "a key is the function, a count and the change: {rows:?}"
    );
    let all = mutant_rows(&dir, "--mutate");
    assert!(
        all.iter().any(|(file, ..)| file.ends_with("post.nvs")),
        "`--mutate` alone changes every file: {all:?}"
    );
}

/// `--mutate=Class::method` changes that method alone, `--mutate=Class`
/// every method of the class, and two targets are separated by a comma.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_aimed_at_a_method_mutates_only_its_body() {
    let dir = aimed("mutate-aim-method", AIMED_CART, AIMED_POST);
    let rows = mutant_rows(&dir, "--mutate=Cart::total");
    let functions: Vec<&str> = rows.iter().map(|(_, f, _)| f.as_str()).collect();
    assert!(
        !functions.is_empty() && functions.iter().all(|f| *f == "Cart::total"),
        "only `Cart::total` is changed: {rows:?}"
    );
    let rows = mutant_rows(&dir, "--mutate=Cart::total,Post");
    let mut functions: Vec<&str> = rows.iter().map(|(_, f, _)| f.as_str()).collect();
    functions.sort_unstable();
    functions.dedup();
    assert_eq!(
        functions,
        ["Cart::total", "Post::words"],
        "a method and a class: {rows:?}"
    );
}

/// `#[NoMutate]` on a method or a class keeps its code unchanged, and one
/// with no reason does not compile.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_skips_a_class_or_method_with_no_mutate() {
    let cart = AIMED_CART.replace(
        "final class Cart {\n    public static function total",
        "use Core\\Test\\NoMutate;\n\nfinal class Cart {\n    public static function total",
    );
    let cart = cart.replace(
        "    public static function big",
        "    #[NoMutate(because: \"only a size check\")]\n    public static function big",
    );
    let post = AIMED_POST.replace(
        "final class Post",
        "use Core\\Test\\NoMutate;\n\n#[NoMutate(because: \"generated code\")]\nfinal class Post",
    );
    let dir = aimed("mutate-no-mutate", &cart, &post);
    let rows = mutant_rows(&dir, "--mutate");
    assert!(
        !rows.is_empty() && rows.iter().all(|(_, f, _)| f == "Cart::total"),
        "only the method with no `#[NoMutate]` is changed: {rows:?}"
    );

    let bare = AIMED_POST.replace(
        "final class Post",
        "use Core\\Test\\NoMutate;\n\n#[NoMutate]\nfinal class Post",
    );
    let dir = aimed("mutate-no-mutate-bare", AIMED_CART, &bare);
    let out = test_in(&dir, &["aim_test.nvs", "--mutate"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_ne!(out.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("E0880") && stderr.contains("no `because`"),
        "a `#[NoMutate]` needs a reason: {stderr}"
    );
}

/// A target that names no folder, file, class or method stops the run with
/// exit code 2, before any test runs.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_aimed_at_nothing_that_exists_exits_with_an_error() {
    let dir = aimed("mutate-aim-nothing", AIMED_CART, AIMED_POST);
    for target in [
        "--mutate=Nowhere",
        "--mutate=Cart::nothing",
        "--mutate=Cart::total,Carts",
    ] {
        let out = test_in(&dir, &["aim_test.nvs", target]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{target}: {stdout}{stderr}");
        assert!(
            stderr.contains("matches no folder, file, class or method"),
            "{target}: {stderr}"
        );
        assert!(
            !stdout.contains("AimTest"),
            "{target}: no test runs: {stdout}"
        );
    }
}

/// Runs `git` with `args` in `dir`, with no global or system configuration,
/// and never in a checkout above `dir`. Fails the test when git does.
fn git_in(dir: &Path, args: &[&str]) {
    let out = git_command(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("`git` is on `PATH`");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A `git` command in `dir` that reads no global or system configuration
/// and finds no checkout above `dir`.
fn git_command(dir: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(dir);
    for (key, value) in git_env(dir) {
        command.env(key, value);
    }
    command
}

/// The environment every `git` of a `--since` test runs under, `nvs`'s own
/// included: an empty global configuration, no system one, and the parent of
/// `dir` as a ceiling, so the repository's own checkout is never found.
fn git_env(dir: &Path) -> Vec<(&'static str, std::ffi::OsString)> {
    let global = dir
        .parent()
        .expect("a scratch folder has a parent")
        .join(format!(
            "{}.gitconfig",
            dir.file_name().unwrap().to_string_lossy()
        ));
    fs::write(&global, "").unwrap();
    vec![
        ("GIT_CONFIG_GLOBAL", global.into_os_string()),
        ("GIT_CONFIG_NOSYSTEM", "1".into()),
        (
            "GIT_CEILING_DIRECTORIES",
            dir.parent().unwrap().as_os_str().to_owned(),
        ),
    ]
}

/// Runs `nvs test` with `args` from `dir`, under [`git_env`].
fn test_in_checkout(dir: &Path, args: &[&str]) -> Output {
    let data = nvs_repo::scratch_private("nvsdata");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(&*data)
        .arg("test")
        .args(args)
        .current_dir(dir);
    for (key, value) in git_env(dir) {
        command.env(key, value);
    }
    command
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// The aimed program in a new checkout, with `aim_test.nvs` and
/// `shop/cart.nvs` committed and `blog/post.nvs` never added.
fn aimed_checkout(case: &str) -> PathBuf {
    let dir = aimed(case, AIMED_CART, AIMED_POST);
    git_in(&dir, &["init", "--quiet"]);
    git_in(&dir, &["add", "aim_test.nvs", "shop/cart.nvs"]);
    git_in(&dir, &["commit", "--quiet", "-m", "start"]);
    dir
}

/// `--since HEAD` changes only the lines edited since the commit, and every
/// line of a file git does not track.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_since_mutates_only_changed_and_untracked_lines() {
    let dir = aimed_checkout("mutate-since-lines");
    fs::write(
        dir.join("shop").join("cart.nvs"),
        AIMED_CART.replace("return $a + $b;", "return $b + $a;"),
    )
    .unwrap();
    let out = test_in_checkout(
        &dir,
        &[
            "aim_test.nvs",
            "--mutate",
            "--since",
            "HEAD",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    let mut functions: Vec<&str> = document["mutants"]
        .as_array()
        .expect("a `mutants` array")
        .iter()
        .map(|m| m["function"].as_str().unwrap_or_default())
        .collect();
    functions.sort_unstable();
    functions.dedup();
    assert_eq!(
        functions,
        ["Cart::total", "Post::words"],
        "the edited line and the untracked file, and not `Cart::big`: {document}"
    );

    let out = test_in_checkout(
        &dir,
        &[
            "aim_test.nvs",
            "--mutate=Post",
            "--since=HEAD",
            "--format",
            "json",
        ],
    );
    let document: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    let mutants = document["mutants"].as_array().expect("a `mutants` array");
    assert!(
        !mutants.is_empty() && mutants.iter().all(|m| m["function"] == "Post::words"),
        "a target and `--since` must both hold: {document}"
    );
}

/// A ref git does not know stops the run with exit code 2, before any test
/// runs.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_since_a_ref_git_does_not_know_exits_with_an_error() {
    let dir = aimed_checkout("mutate-since-unknown");
    let out = test_in_checkout(
        &dir,
        &["aim_test.nvs", "--mutate", "--since=no-such-branch"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stdout}{stderr}");
    assert!(
        stderr.contains("`--since=no-such-branch` names no branch, tag or commit"),
        "{stderr}"
    );
    assert!(!stdout.contains("AimTest"), "no test runs: {stdout}");
}

/// A ref that starts with `-` is refused before git runs, so git never reads
/// it as an option and writes no file.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_since_a_ref_that_starts_with_a_dash_is_not_a_git_option() {
    let dir = aimed_checkout("mutate-since-dash");
    let written = dir.join("written.txt");
    for since in [
        format!("--since=-o{}", written.display()),
        format!("--since=--output={}", written.display()),
    ] {
        let out = test_in_checkout(&dir, &["aim_test.nvs", "--mutate", &since]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{since}: {stdout}{stderr}");
        assert!(
            stderr.contains("never starts with `-`"),
            "{since}: {stderr}"
        );
        assert!(!written.exists(), "{since}: git wrote a file");
        assert!(
            !stdout.contains("AimTest"),
            "{since}: no test runs: {stdout}"
        );
    }
}

/// A program outside a git checkout stops a `--since` run with exit code 2.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_since_outside_a_checkout_exits_with_an_error() {
    let dir = aimed("mutate-since-no-checkout", AIMED_CART, AIMED_POST);
    let out = test_in_checkout(&dir, &["aim_test.nvs", "--mutate", "--since=HEAD"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stdout}{stderr}");
    assert!(stderr.contains("needs a git checkout"), "{stderr}");
    assert!(!stdout.contains("AimTest"), "no test runs: {stdout}");
}

/// `nvs test --mutate` runs the tests, then each mutant: the human report
/// names the survivor and its change, a survivor does not fail the run, and
/// the JSON document's `mutants` records say how many tests ran each one. A
/// test that fails before any change runs no mutant and fails the run.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_reports_each_mutant_and_fails_only_on_a_failing_test() {
    let dir = scratch("mutate");
    fs::write(dir.join("stock_test.nvs"), MUTATED).unwrap();

    let human = test_in(&dir, &["stock_test.nvs", "--mutate"]);
    let stdout = String::from_utf8_lossy(&human.stdout);
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert_eq!(
        human.status.code(),
        Some(0),
        "a survived mutant does not fail the run: {stdout}{stderr}"
    );
    assert!(
        stdout.contains("mutants: 1 killed, 1 survived, 0 timed out, 0 not reached"),
        "{stdout}"
    );
    assert!(
        stdout.contains("survived  stock_test.nvs:6  `>=` to `>`"),
        "the survivor is named with its line and change: {stdout}"
    );

    let json = test_in(&dir, &["stock_test.nvs", "--mutate", "--format", "json"]);
    let document: serde_json::Value = serde_json::from_slice(&json.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    assert_eq!(document["schemaVersion"], 5);
    let mutants = document["mutants"].as_array().expect("a `mutants` array");
    assert_eq!(mutants.len(), 2, "{mutants:?}");
    assert!(
        mutants.iter().all(|m| m["tests"] == 1),
        "each mutant runs the one test that reached its line: {mutants:?}"
    );

    fs::write(
        dir.join("stock_test.nvs"),
        MUTATED.replace(
            "assertSame(Stock::twice(3), 6)",
            "assertSame(Stock::twice(3), 7)",
        ),
    )
    .unwrap();
    let failing = test_in(&dir, &["stock_test.nvs", "--mutate"]);
    let stdout = String::from_utf8_lossy(&failing.stdout);
    let stderr = String::from_utf8_lossy(&failing.stderr);
    assert_eq!(failing.status.code(), Some(1), "{stdout}");
    assert!(
        stderr.contains("no mutant ran: a test failed")
            && stdout.contains("mutants: 0 killed, 0 survived, 0 timed out, 0 not reached"),
        "no mutant runs after a failing test: {stdout}{stderr}"
    );
}
