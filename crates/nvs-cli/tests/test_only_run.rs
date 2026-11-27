//! `rule:testing/a-test-only-declaration-is-test-code`, driven through the built binary:
//! `nvs run` gives a `#[TestOnly]` class no class row, `Core\Program::implementing` lists it in
//! no build, and `nvs test` runs the tests that use it, a package's included.
//!
//! The fixtures are under `tests/fixtures/test-only/`:
//!
//! - `clock.nvs` declares a `Clock` interface, a production `SystemClock` and a `#[TestOnly]`
//!   `FakeClock`, two tests, and a program that looks `FakeClock` up by name.
//! - `Framework/` is a package, `Framework\Auth`, whose `FakeLogin` is `#[TestOnly]`.
//! - `Shop/ShopTest.nvs` autoloads the package, and its test signs in through `FakeLogin`.

use std::path::PathBuf;
use std::process::Command;

/// `tests/fixtures/test-only/<name>`.
fn fixture(name: &str) -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "test-only",
        name,
    ]
    .iter()
    .collect()
}

/// `nvs <command> <fixture>`, with a data folder of its own, as `(output, success)`.
fn nvs(command: &str, file: &str) -> (String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg(command)
        .arg(fixture(file))
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (output, out.status.success())
}

/// `Core\Reflect::forClass` finds no row for a test-only class under `nvs run`, and finds the
/// production class beside it, so the lookup itself works.
// covers: lang:testing/testonly-code-that-only-tests-can-use
#[test]
fn a_run_cannot_find_a_test_only_class_by_name() {
    let (output, ok) = nvs("run", "clock.nvs");
    assert!(ok, "the run failed:\n{output}");
    assert!(output.contains("no FakeClock\n"), "{output}");
    assert!(output.contains("SystemClock found\n"), "{output}");
}

/// The run lists the production clock alone, and the test that asks the same question under
/// `nvs test` passes only when it gets the same list.
#[test]
fn implementing_never_lists_a_test_only_class() {
    let (output, ok) = nvs("run", "clock.nvs");
    assert!(ok, "the run failed:\n{output}");
    assert!(output.contains("listed: SystemClock \n"), "{output}");

    let (report, ok) = nvs("test", "clock.nvs");
    assert!(ok, "the tests failed:\n{report}");
    assert!(
        report.contains("aTestListsTheClocksProductionLists"),
        "the listing test ran:\n{report}"
    );
}

/// An application's test calls a package's `#[TestOnly]` helper, and its production code still
/// runs without it.
#[test]
fn a_package_test_only_class_is_usable_from_the_application_tests() {
    let (report, ok) = nvs("test", "Shop/ShopTest.nvs");
    assert!(ok, "the application's test failed:\n{report}");
    assert!(report.contains("0 failed, 1 passed"), "{report}");

    let (output, ok) = nvs("run", "Shop/ShopTest.nvs");
    assert!(ok, "the application failed to run:\n{output}");
    assert!(output.contains("cannot pay\n"), "{output}");
}

/// `nvs test` lowers the test-only class, so a test may create one and call it.
#[test]
fn nvs_test_runs_a_test_that_uses_test_only_code() {
    let (report, ok) = nvs("test", "clock.nvs");
    assert!(ok, "the tests failed:\n{report}");
    assert!(report.contains("aFakeClockGivesItsOwnTime"), "{report}");
    assert!(report.contains("0 failed, 2 passed"), "{report}");
}
