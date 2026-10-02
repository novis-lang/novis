//! `nvs test --coverage-lcov <FILE>` and `--coverage-clover <FILE>`: which
//! lines a report lists, what each count is, how files are named, and where
//! the flags are refused, as `docs/reference/tools/10-cli.md` § *nvs test*
//! states them.
//!
//! Through the built binary, because what is asserted is the file a CI
//! service reads and the line the run prints.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A directory of the case's own under the target's scratch, emptied first,
/// so two cases running in parallel never share a fixture.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("test-coverage")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `nvs test` with `args` from `dir`. `NOVIS_NO_INIT` keeps the run from writing the shipped
/// `nvs.toml` into `dir`, so the only files a case finds afterwards are its own and the reports.
fn test_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("test")
        .args(args)
        .env("NOVIS_NO_INIT", "1")
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A class with a branch the tests take one side of and a method no test
/// calls, then the tests. The line numbers in the assertions below are this
/// text's.
const PRICE: &str = "<?nvs
use Core\\Test;

final class Price {
    public static function double(int $cents): int {
        return $cents * 2;
    }

    public static function label(int $cents): string {
        if ($cents == 0) {
            return \"free\";
        }
        return \"paid\";
    }

    public static function neverCalled(): int {
        return 1;
    }
}

final class PriceTest {
    #[Test]
    public function doubles(): void {
        Test::assertSame(Price::double(2), 4);
        Test::assertSame(Price::double(3), 6);
    }

    #[Test]
    public function labelsAFreeItem(): void {
        Test::assertSame(Price::label(0), \"free\");
    }
}
";

#[test]
fn the_lcov_file_lists_every_statement_line_with_its_count() {
    let dir = scratch("lcov");
    fs::write(dir.join("price.nvs"), PRICE).expect("the program is written");
    let out = test_in(&dir, &["price.nvs", "--coverage-lcov", "coverage.lcov"]);
    assert!(
        out.status.success(),
        "{}{}",
        text(&out.stdout),
        text(&out.stderr)
    );

    // Line 6 runs once per call. Line 13 is the branch no test takes, and
    // line 17 is in the method no test calls.
    assert_eq!(
        fs::read_to_string(dir.join("coverage.lcov")).expect("the lcov file was written"),
        "TN:\nSF:price.nvs\nDA:6,2\nDA:10,1\nDA:11,1\nDA:13,0\nDA:17,0\nDA:24,1\nDA:25,1\nDA:30,1\n\
         LF:8\nLH:6\nend_of_record\n"
    );
    assert!(
        text(&out.stdout).ends_with("  6 of 8 lines run (75.0%)\n"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn the_clover_file_carries_the_same_lines_and_the_totals() {
    let dir = scratch("clover");
    fs::write(dir.join("price.nvs"), PRICE).expect("the program is written");
    let out = test_in(&dir, &["price.nvs", "--coverage-clover", "clover.xml"]);
    assert!(
        out.status.success(),
        "{}{}",
        text(&out.stderr),
        text(&out.stdout)
    );

    let document = fs::read_to_string(dir.join("clover.xml")).expect("the Clover file was written");
    assert!(
        document.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<coverage generated=\"")
    );
    assert!(document.contains("<file name=\"price.nvs\">"), "{document}");
    assert!(
        document.contains("<line num=\"6\" type=\"stmt\" count=\"2\"/>"),
        "{document}"
    );
    assert!(
        document.contains("<line num=\"17\" type=\"stmt\" count=\"0\"/>"),
        "{document}"
    );
    assert!(
        document.contains("<metrics files=\"1\" loc=\"33\" ncloc=\"33\" "),
        "{document}"
    );
    assert!(
        document.contains("statements=\"8\" coveredstatements=\"6\""),
        "{document}"
    );
}

#[test]
fn both_files_can_be_written_by_one_run_and_a_machine_format_keeps_stdout_its_own() {
    let dir = scratch("both");
    fs::write(dir.join("price.nvs"), PRICE).expect("the program is written");
    let out = test_in(
        &dir,
        &[
            "price.nvs",
            "--format",
            "json",
            "--coverage-lcov",
            "coverage.lcov",
            "--coverage-clover",
            "clover.xml",
        ],
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(dir.join("coverage.lcov").is_file());
    assert!(dir.join("clover.xml").is_file());
    let stdout = text(&out.stdout);
    assert!(
        !stdout.contains("lines run"),
        "stdout is the JSON document and nothing else: {stdout}"
    );
}

#[test]
fn a_run_without_the_flags_writes_no_file_and_prints_no_coverage_line() {
    let dir = scratch("off");
    fs::write(dir.join("price.nvs"), PRICE).expect("the program is written");
    let out = test_in(&dir, &["price.nvs"]);
    assert!(out.status.success());
    assert!(!text(&out.stdout).contains("lines run"));
    let written: Vec<_> = fs::read_dir(&dir)
        .expect("the scratch directory is readable")
        .map(|entry| entry.expect("an entry").file_name())
        .collect();
    assert_eq!(written, ["price.nvs"]);
}

/// The server a `server: true` test starts runs the file's own statements for
/// each request, in a context of its own. Those lines are counted too.
#[test]
fn a_request_a_server_test_sends_counts_the_lines_that_answer_it() {
    let dir = scratch("server");
    fs::write(
        dir.join("nvs.toml"),
        "[capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n",
    )
    .expect("the configuration is written");
    fs::write(
        dir.join("ping.nvs"),
        "<?nvs
use Core\\Http\\Client;
use Core\\Test;

class PingTest {
    #[Test(server: true)]
    public function itAnswers(): void {
        var $response = Client::get((Test::serverUrl() ?? \"\") . \"/ping\");
        Test::assertSame($response->text(), \"pong\");
    }
}

if (Core\\Request::path() == \"/ping\") {
    echo \"pong\";
} else {
    echo \"unknown\";
}
",
    )
    .expect("the program is written");
    let out = test_in(&dir, &["ping.nvs", "--coverage-lcov", "coverage.lcov"]);
    assert!(
        out.status.success(),
        "{}{}",
        text(&out.stdout),
        text(&out.stderr)
    );
    let report = fs::read_to_string(dir.join("coverage.lcov")).expect("the lcov file was written");
    assert!(
        report.contains("DA:13,1\nDA:14,1\n"),
        "the request's lines ran: {report}"
    );
    assert!(
        report.contains("DA:16,0\n"),
        "the other branch did not: {report}"
    );
}

/// A directory run compiles an entry file it generates. That file is not on
/// disk, so it is not in the report, and a file the bootstrap requires is
/// named relative to the directory the run started in.
#[test]
fn a_directory_run_names_its_files_relative_and_leaves_out_the_generated_entry() {
    let fixtures: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "test-dir"]
        .iter()
        .collect();
    let report = scratch("directory").join("coverage.lcov");
    let out = test_in(
        &fixtures,
        &[
            "shop/tests",
            "--coverage-lcov",
            report.to_str().expect("a UTF-8 path"),
        ],
    );
    assert!(
        out.status.success(),
        "{}{}",
        text(&out.stdout),
        text(&out.stderr)
    );
    let files: Vec<String> = fs::read_to_string(&report)
        .expect("the lcov file was written")
        .lines()
        .filter_map(|line| line.strip_prefix("SF:").map(str::to_owned))
        .collect();
    assert_eq!(
        files,
        [
            "shop/src/Cart.nvs",
            "shop/tests/CartTest.nvs",
            "shop/tests/bootstrap.nvs",
            "shop/tests/unit/HelperTest.nvs"
        ]
    );
}

#[test]
fn coverage_beside_a_case_tree_is_refused() {
    let dir = scratch("nvst");
    fs::write(
        dir.join("one.nvst"),
        "--TEST--\nprints\n--FILE--\n<?nvs\necho 1;\n--EXPECT--\n1\n",
    )
    .expect("the case is written");
    let out = test_in(&dir, &["one.nvst", "--coverage-lcov", "coverage.lcov"]);
    assert!(!out.status.success());
    assert!(
        text(&out.stderr).contains("measure a program's `#[Test]` methods, not a `.nvst` tree"),
        "{}",
        text(&out.stderr)
    );
    assert!(!dir.join("coverage.lcov").exists());
}

#[test]
fn coverage_beside_list_is_refused() {
    let dir = scratch("list");
    fs::write(dir.join("price.nvs"), PRICE).expect("the program is written");
    let out = test_in(
        &dir,
        &["price.nvs", "--list", "--coverage-clover", "clover.xml"],
    );
    assert!(!out.status.success());
    assert!(!dir.join("clover.xml").exists());
}
