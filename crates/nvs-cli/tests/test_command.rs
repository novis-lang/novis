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
    Command::new(env!("CARGO_BIN_EXE_nvs"))
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
