//! `nvs test <file>.nvs`: which methods are tests, the order they are
//! reported in, what fails and what does not, and the exit status, as
//! `docs/reference/tools/10-cli.md` § *nvs test* states them.
//!
//! Through the built binary, because what is asserted is what CI reads off
//! the command: its report and its status.

use std::collections::BTreeMap;
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

/// `Gate::flag`'s `true` is stored and never read, so flipping it compiles to
/// the same code. `Gate::same`'s `==` negated and its `if` negated compile
/// alike.
const COMPILED_ALIKE: &str = r#"<?nvs
use Core\Test;

final class Gate {
    public static function same(int $a, int $b): string {
        if ($a == $b) {
            return 'same';
        }
        return 'other';
    }

    public static function flag(int $n): int {
        bool $unused = true;
        return $n;
    }
}

final class GateTest {
    #[Test]
    public function same(): void {
        Test::assertSame(Gate::same(1, 1), 'same');
        Test::assertSame(Gate::same(1, 2), 'other');
    }

    #[Test]
    public function flag(): void { Test::assertSame(Gate::flag(3), 3); }
}
"#;

/// Each mutant of [`COMPILED_ALIKE`]'s run, by key: its outcome, its
/// `sameAs` and how many tests ran. Also the human report.
fn compiled_alike(case: &str) -> (BTreeMap<String, (String, String, u64)>, String) {
    let dir = scratch(case);
    fs::write(dir.join("gate_test.nvs"), COMPILED_ALIKE).unwrap();
    let json = test_in(&dir, &["gate_test.nvs", "--mutate", "--format", "json"]);
    assert_eq!(
        json.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&json.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&json.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    let rows = document["mutants"]
        .as_array()
        .expect("a `mutants` array")
        .iter()
        .map(|m| {
            let text = |key: &str| m[key].as_str().unwrap_or_default().to_owned();
            (
                text("key"),
                (
                    text("outcome"),
                    text("sameAs"),
                    m["tests"].as_u64().unwrap_or(99),
                ),
            )
        })
        .collect();
    let human = test_in(&dir, &["gate_test.nvs", "--mutate"]);
    (rows, String::from_utf8_lossy(&human.stdout).into_owned())
}

/// A mutant whose function compiles to the program's own code is
/// `equivalent`: no test runs, and the score leaves it out.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_drops_a_mutant_that_compiles_to_the_programs_code() {
    let (rows, human) = compiled_alike("mutate-equivalent");
    assert_eq!(
        rows.get("Gate::flag#1: `true` to `false`"),
        Some(&("equivalent".to_owned(), String::new(), 0)),
        "{rows:?}"
    );
    assert!(
        human.contains("1 equivalent") && human.contains("1 of 1 reached mutants"),
        "the human report counts it and leaves it out of the score: {human}"
    );
}

/// Of two mutants whose function compiles alike, the first runs its tests
/// and the second is a `duplicate` that names it.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_runs_one_of_two_mutants_that_compile_alike() {
    let (rows, human) = compiled_alike("mutate-duplicate");
    let first = "Gate::same#1: `==` to `!=`";
    assert_eq!(
        rows.get(first),
        Some(&("killed".to_owned(), String::new(), 1)),
        "{rows:?}"
    );
    assert_eq!(
        rows.get("Gate::same#1: `condition` to `!condition`"),
        Some(&("duplicate".to_owned(), first.to_owned(), 0)),
        "{rows:?}"
    );
    assert!(human.contains("1 duplicate"), "{human}");
}

/// Three survivors on `Shelf::fits`'s one line, one survivor in
/// `Shelf::count`, and a log call and an exception message that make no
/// mutant.
const JUDGED: &str = r#"<?nvs
use Core\Test;
use Core\Log;

final class Shelf {
    public static function fits(int $size, int $room): bool {
        return $size <= $room && $size >= 0;
    }

    public static function count(int $n): int {
        Log::write(Log::Level::Info, 'counting', ['n' => $n + 1]);
        if ($n < 0) {
            throw new LogicError('negative ' . ($n - 1));
        }
        return $n;
    }
}

final class ShelfTest {
    #[Test]
    public function fitsFarInside(): void {
        Test::assertTrue(Shelf::fits(2, 10));
    }

    #[Test]
    public function counts(): void { Test::assertSame(Shelf::count(3), 3); }
}
"#;

/// The key of `Shelf::count`'s one survivor.
const COUNT_SURVIVOR: &str = "Shelf::count#1: `<` to `<=`";

/// A scratch folder with [`JUDGED`] and the given `mutants.toml`.
fn judged_dir(case: &str, judged: &str) -> PathBuf {
    let dir = scratch(case);
    fs::write(dir.join("shelf_test.nvs"), JUDGED).unwrap();
    fs::write(dir.join("mutants.toml"), judged).unwrap();
    dir
}

/// A survivor `mutants.toml` judged is counted and not listed, and an
/// `equivalent` one leaves the score. Its key still matches once a class is
/// added above its function.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_reads_judged_survivors_from_mutants_toml() {
    let dir = judged_dir(
        "mutate-judged",
        &format!(
            "[[judged]]\nmutant = \"{COUNT_SURVIVOR}\"\nreason = \"a count is never negative\"\nequivalent = true\n"
        ),
    );
    let check = |out: &std::process::Output| {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(0), "{stdout}{stderr}");
        assert!(
            stdout.contains("judged: 1 survived")
                && !stdout.contains("survived in Shelf::count")
                && stdout.contains("survived in Shelf::fits (3):")
                && stdout.contains("of 4 reached mutants"),
            "the judged survivor is counted, not listed, and out of the score: {stdout}"
        );
        assert!(!stderr.contains("warning:"), "{stderr}");
    };
    check(&test_in(&dir, &["shelf_test.nvs", "--mutate"]));

    fs::write(
        dir.join("shelf_test.nvs"),
        JUDGED.replace(
            "final class Shelf {",
            "final class Above {\n    public static function one(int $n): int {\n        return $n - 1;\n    }\n}\n\nfinal class Shelf {",
        ),
    )
    .unwrap();
    check(&test_in(&dir, &["shelf_test.nvs", "--mutate"]));

    let json = test_in(&dir, &["shelf_test.nvs", "--mutate", "--format", "json"]);
    let document: serde_json::Value = serde_json::from_slice(&json.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    let record = document["mutants"]
        .as_array()
        .expect("a `mutants` array")
        .iter()
        .find(|m| m["key"] == COUNT_SURVIVOR)
        .expect("the judged survivor has a record");
    assert_eq!(record["outcome"], "survived");
    assert_eq!(record["judged"], true);
    assert_eq!(record["reason"], "a count is never negative");
}

/// An entry whose key no mutant of the program has is named in a warning,
/// and the run still passes.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_names_a_judged_entry_that_matches_no_mutant() {
    let dir = judged_dir(
        "mutate-judged-stale",
        "[[judged]]\nmutant = \"Shelf::gone#1: `<` to `<=`\"\nreason = \"it was removed\"\n",
    );
    let out = test_in(&dir, &["shelf_test.nvs", "--mutate"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains(
            "warning: mutants.toml: no mutant of this program has the key `Shelf::gone#1: `<` to `<=``"
        ),
        "{stderr}"
    );
}

/// An entry with no reason is an error, and no test runs.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_refuses_a_judged_entry_with_no_reason() {
    let dir = judged_dir(
        "mutate-judged-no-reason",
        &format!("[[judged]]\nmutant = \"{COUNT_SURVIVOR}\"\nreason = \"\"\n"),
    );
    let out = test_in(&dir, &["shelf_test.nvs", "--mutate"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stdout}{stderr}");
    assert!(
        stderr.contains("error: ") && stderr.contains("needs a non-empty `reason`"),
        "{stderr}"
    );
    assert!(!stdout.contains("fitsFarInside"), "no test ran: {stdout}");
}

/// Survivors are grouped by function, most first, one line per source line
/// with the key to paste. A log call and an exception message make no
/// mutant.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_groups_survivors_by_function_with_one_line_per_source_line() {
    let dir = judged_dir("mutate-grouped", "");
    let out = test_in(&dir, &["shelf_test.nvs", "--mutate"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    let listed: Vec<&str> = stdout
        .lines()
        .skip_while(|line| !line.starts_with("  survived in"))
        .collect();
    assert_eq!(listed.len(), 4, "{stdout}");
    assert_eq!(listed[0], "  survived in Shelf::fits (3):", "{stdout}");
    assert!(
        listed[1].starts_with("    shelf_test.nvs:7  ")
            && listed[1].contains(" (+2 more on this line)  mutant = \"Shelf::fits#1: "),
        "{stdout}"
    );
    assert_eq!(listed[2], "  survived in Shelf::count (1):", "{stdout}");
    assert_eq!(
        listed[3],
        format!("    shelf_test.nvs:12  `<` to `<=`  mutant = \"{COUNT_SURVIVOR}\""),
    );

    let json = test_in(&dir, &["shelf_test.nvs", "--mutate", "--format", "json"]);
    let document: serde_json::Value = serde_json::from_slice(&json.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    let lines: Vec<u64> = document["mutants"]
        .as_array()
        .expect("a `mutants` array")
        .iter()
        .filter_map(|m| m["line"].as_u64())
        .collect();
    assert!(
        !lines.contains(&11) && !lines.contains(&13),
        "the log call and the exception message made a mutant: {lines:?}"
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

/// Two test classes over `Cart`: `TotalTest`'s two tests call `Cart::total`,
/// and `BigTest`'s one test calls `Cart::big`. The line numbers matter to the
/// `<file>:<line>` tests: `adds` is on lines 7 to 10, and `TotalTest` starts
/// on line 6.
const SELECT_ENTRY: &str = r#"<?nvs
use Core\Test;

require 'shop/cart.nvs';

final class TotalTest {
    #[Test]
    public function adds(): void {
        Test::assertSame(Cart::total(2, 3), 5);
    }

    #[Test]
    public function addsZero(): void {
        Test::assertSame(Cart::total(0, 3), 3);
    }
}

final class BigTest {
    #[Test]
    public function big(): void {
        Test::assertTrue(Cart::big(10));
    }
}
"#;

/// The selection program in a new scratch folder.
fn selected_program(case: &str) -> PathBuf {
    let dir = scratch(case);
    fs::create_dir_all(dir.join("shop")).unwrap();
    fs::write(dir.join("select_test.nvs"), SELECT_ENTRY).unwrap();
    fs::write(dir.join("shop").join("cart.nvs"), AIMED_CART).unwrap();
    dir
}

/// The selection program in a new checkout, every file committed, and a
/// data folder that lasts as long as the value.
fn selected_checkout(case: &str) -> (PathBuf, nvs_repo::Scratch) {
    let dir = selected_program(case);
    git_in(&dir, &["init", "--quiet"]);
    git_in(&dir, &["add", "."]);
    git_in(&dir, &["commit", "--quiet", "-m", "start"]);
    (dir, nvs_repo::scratch_private("nvsdata"))
}

/// Runs `nvs test select_test.nvs --since HEAD --format json` from `dir`
/// with the data folder `data`, under [`git_env`]. Gives the `Class::method`
/// of every test that ran, sorted, and the standard error.
fn since_run(dir: &Path, data: &Path) -> (Vec<String>, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(data)
        .args([
            "test",
            "select_test.nvs",
            "--since",
            "HEAD",
            "--format",
            "json",
        ])
        .current_dir(dir);
    for (key, value) in git_env(dir) {
        command.env(key, value);
    }
    let out = command
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    (ran_tests(&out), stderr)
}

/// The `Class::method` of every test a `--format json` run reports, sorted.
fn ran_tests(out: &Output) -> Vec<String> {
    let document: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "`--format json` writes one document: {e}: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let mut ran: Vec<String> = document["tests"]
        .as_array()
        .expect("a `tests` array")
        .iter()
        .map(|t| {
            format!(
                "{}::{}",
                t["class"].as_str().unwrap_or_default(),
                t["method"].as_str().unwrap_or_default()
            )
        })
        .collect();
    ran.sort();
    ran
}

const EVERY_TEST: [&str; 3] = ["BigTest::big", "TotalTest::adds", "TotalTest::addsZero"];

/// After a run recorded what each test reached, an edit inside `Cart::total`
/// runs the two tests that called it, and not `BigTest`. A later run still
/// knows `BigTest`, which the narrowed run did not run.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_since_runs_only_the_tests_that_reached_a_changed_function() {
    let (dir, data) = selected_checkout("since-select-reached");
    let (ran, _) = since_run(&dir, &data);
    assert_eq!(ran, EVERY_TEST, "the first run records every test");
    fs::write(
        dir.join("shop").join("cart.nvs"),
        AIMED_CART.replace("return $a + $b;", "return $b + $a;"),
    )
    .unwrap();
    for _ in 0..2 {
        let (ran, stderr) = since_run(&dir, &data);
        assert_eq!(
            ran,
            ["TotalTest::adds", "TotalTest::addsZero"],
            "only the tests that called `Cart::total`: {stderr}"
        );
        assert!(!stderr.contains("Every selected test runs"), "{stderr}");
    }
}

/// An edited test and a new test class run. A test that reached nothing
/// that changed does not.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_since_runs_a_new_or_edited_test() {
    let (dir, data) = selected_checkout("since-select-new-test");
    since_run(&dir, &data);
    let edited = SELECT_ENTRY.replace("Cart::big(10)", "Cart::big(11)")
        + "\nfinal class NewTest {\n    #[Test]\n    public function fresh(): void {\n        Test::assertSame(1, 1);\n    }\n}\n";
    fs::write(dir.join("select_test.nvs"), edited).unwrap();
    let (ran, stderr) = since_run(&dir, &data);
    assert_eq!(ran, ["BigTest::big", "NewTest::fresh"], "{stderr}");
}

/// A changed method signature is outside every body, so every test runs, and
/// the first line says why.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_since_runs_every_test_when_a_declaration_changed() {
    let (dir, data) = selected_checkout("since-select-declaration");
    since_run(&dir, &data);
    fs::write(
        dir.join("shop").join("cart.nvs"),
        AIMED_CART.replace("big(int $n)", "big(int $n = 0)"),
    )
    .unwrap();
    let (ran, stderr) = since_run(&dir, &data);
    assert_eq!(ran, EVERY_TEST, "{stderr}");
    assert!(
        stderr.contains("Every selected test runs, because line 7 of `shop/cart.nvs` changed"),
        "{stderr}"
    );
}

/// A new file in the program's folder that is not a `.nvs` file may be data
/// a test reads, so every test runs.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_since_runs_every_test_when_a_file_that_is_not_nvs_changed() {
    let (dir, data) = selected_checkout("since-select-data-file");
    since_run(&dir, &data);
    fs::write(dir.join("prices.csv"), "1,2\n").unwrap();
    let (ran, stderr) = since_run(&dir, &data);
    assert_eq!(ran, EVERY_TEST, "{stderr}");
    assert!(
        stderr.contains("`prices.csv` changed, and it is not a `.nvs` file"),
        "{stderr}"
    );
}

/// With no earlier run recorded, every test runs, and the line says why.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_since_runs_every_test_with_no_stored_map() {
    let (dir, data) = selected_checkout("since-select-no-map");
    fs::write(
        dir.join("shop").join("cart.nvs"),
        AIMED_CART.replace("return $a + $b;", "return $b + $a;"),
    )
    .unwrap();
    let (ran, stderr) = since_run(&dir, &data);
    assert_eq!(ran, EVERY_TEST, "{stderr}");
    assert!(
        stderr.contains("Every selected test runs, because no earlier run recorded"),
        "{stderr}"
    );
}

/// The record was written while `Cart::total` was edited. Putting the edit
/// back leaves nothing in git's diff, and `Cart::total` still counts as
/// changed, because its text differs from the recorded one.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_since_counts_a_file_edited_since_the_map_as_changed() {
    let (dir, data) = selected_checkout("since-select-edited-since-map");
    let cart = dir.join("shop").join("cart.nvs");
    fs::write(
        &cart,
        AIMED_CART.replace("return $a + $b;", "return $b + $a;"),
    )
    .unwrap();
    since_run(&dir, &data);
    fs::write(&cart, AIMED_CART).unwrap();
    let (ran, stderr) = since_run(&dir, &data);
    assert_eq!(ran, ["TotalTest::adds", "TotalTest::addsZero"], "{stderr}");
}

/// Runs `nvs test <path> --format json` from `dir`.
fn at_line_run(dir: &Path, path: &str) -> Output {
    test_in(dir, &[path, "--format", "json"])
}

/// A line inside a test method runs that test alone.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_at_a_line_runs_the_test_that_holds_it() {
    let dir = selected_program("at-line-test");
    for line in [7, 9, 10] {
        let out = at_line_run(&dir, &format!("select_test.nvs:{line}"));
        assert_eq!(out.status.code(), Some(0), "line {line}");
        assert_eq!(ran_tests(&out), ["TotalTest::adds"], "line {line}");
    }
}

/// A line of a test class outside its tests runs every test of the class.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_at_a_class_line_runs_its_class() {
    let dir = selected_program("at-line-class");
    for line in [6, 11, 16] {
        let out = at_line_run(&dir, &format!("select_test.nvs:{line}"));
        assert_eq!(out.status.code(), Some(0), "line {line}");
        assert_eq!(
            ran_tests(&out),
            ["TotalTest::adds", "TotalTest::addsZero"],
            "line {line}"
        );
    }
}

/// A line outside every test class stops the run with exit code 2, and the
/// error names the nearest test class.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_at_a_line_outside_test_code_exits_with_an_error() {
    let dir = selected_program("at-line-outside");
    let out = test_in(&dir, &["select_test.nvs:4"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stdout}{stderr}");
    assert!(
        stderr.contains("The nearest test class is `TotalTest`, on line 6"),
        "{stderr}"
    );
    assert!(!stdout.contains("TotalTest"), "no test runs: {stdout}");
}

/// An absolute path keeps its own colons, such as a Windows drive letter, and
/// only the last `:` and digits are the line.
// covers: tools:cli/nvs-test
#[test]
fn nvs_test_at_a_line_keeps_a_windows_drive_letter() {
    let dir = selected_program("at-line-absolute");
    let file = dir.join("select_test.nvs");
    let out = at_line_run(&dir, &format!("{}:20", file.display()));
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(ran_tests(&out), ["BigTest::big"]);
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
        stdout.contains("survived in Stock::atLeast (1):")
            && stdout.contains("stock_test.nvs:6  `>=` to `>`"),
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

/// Five mutants in one function: two boundaries the tests never reach, a
/// connector, a condition and an addition. Each changes only its own site.
const ONE_FUNCTION: &str = r#"<?nvs
use Core\Test;

final class Price {
    public static function cap(int $x, int $limit): int {
        if ($x > $limit && $limit > 0) {
            return $limit;
        }
        return $x + 1;
    }
}

final class PriceTest {
    #[Test]
    public function caps(): void {
        Test::assertSame(Price::cap(500, 100), 100);
        Test::assertSame(Price::cap(5, 100), 6);
    }
}
"#;

/// A mutation run compiles the program once, whatever the number of mutants,
/// and says so in its report.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_compiles_the_program_once_for_every_mutant() {
    let dir = scratch("mutate-once");
    fs::write(dir.join("stock_test.nvs"), MUTATED).unwrap();
    fs::write(dir.join("price_test.nvs"), ONE_FUNCTION).unwrap();

    let run = test_in(&dir, &[".", "--mutate"]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(0), "{stdout}{stderr}");
    assert!(
        stdout.contains("compiles: 1, for the 7 mutants that ran tests"),
        "seven mutants run their tests from one compile: {stdout}{stderr}"
    );
    assert!(
        stdout.contains("mutants: 4 killed, 3 survived, 0 timed out, 0 not reached"),
        "{stdout}"
    );
}

/// Under one compile, each mutant's tests see its own change and no other:
/// the two boundaries no test reaches survive, and the other three are
/// killed.
// covers: tools:cli/nvs-test-mutate
#[test]
fn nvs_test_mutate_two_mutants_in_one_function_each_change_only_their_own_site() {
    let dir = scratch("mutate-own-site");
    fs::write(dir.join("price_test.nvs"), ONE_FUNCTION).unwrap();

    let json = test_in(&dir, &["price_test.nvs", "--mutate", "--format", "json"]);
    let document: serde_json::Value = serde_json::from_slice(&json.stdout)
        .unwrap_or_else(|e| panic!("`--format json` writes one document: {e}"));
    let mut outcomes: Vec<(String, String)> = document["mutants"]
        .as_array()
        .expect("a `mutants` array")
        .iter()
        .map(|m| {
            (
                m["key"].as_str().unwrap_or_default().to_owned(),
                m["outcome"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    outcomes.sort();
    let expected: Vec<(String, String)> = [
        ("Price::cap#1: `&&` to `||`", "killed"),
        ("Price::cap#1: `+` to `-`", "killed"),
        ("Price::cap#1: `>` to `>=`", "survived"),
        ("Price::cap#1: `condition` to `!condition`", "killed"),
        ("Price::cap#2: `>` to `>=`", "survived"),
    ]
    .iter()
    .map(|(key, outcome)| ((*key).to_owned(), (*outcome).to_owned()))
    .collect();
    assert_eq!(outcomes, expected);
}
