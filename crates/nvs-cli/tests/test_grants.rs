//! `nvs test` reads the tree's `[capabilities]`, driven through the built
//! binary: a test's isolate is built from the suite's context, so a grant in the
//! configuration a run names reaches a test exactly as it reaches `nvs run`.
//!
//! The subject is `Core\Test::serverUrl`'s listener, because that is the test
//! that cannot exist without a grant: reaching `127.0.0.1` needs the
//! `net.connect` and `net.internal` pair, and a suite whose context carried no
//! snapshot denied both whatever the file said.

use std::path::PathBuf;
use std::process::Command;

/// `tests/fixtures/test-grants/<name>`.
fn fixture(name: &str) -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "test-grants",
        name,
    ]
    .iter()
    .collect()
}

/// `nvs test --config <config> wire.nvs`, as `(report, success)`.
fn run_under(config: &str) -> (String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("test")
        .arg("--config")
        .arg(fixture(config))
        .arg(fixture("wire.nvs"))
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (report, out.status.success())
}

/// Both sides of one grant, in one test: the configuration that names the
/// pair lets the test reach its own listener, and the one that names neither
/// fails it with the capability named. A runner that ignored the file would
/// fail the first half, and one that granted loopback to every test would fail
/// the second.
// covers: Core\Test::serverUrl
#[test]
fn a_server_test_reaches_its_listener_only_under_the_grant_its_configuration_names() {
    let (report, passed) = run_under("granted.toml");
    assert!(passed, "the granted run failed:\n{report}");
    assert!(
        report.contains("0 failed, 1 passed"),
        "the one test passed under the grant:\n{report}"
    );

    let (report, passed) = run_under("denied.toml");
    assert!(!passed, "the run with no grant passed:\n{report}");
    assert!(
        report.contains("needs the capability `net.connect` for 127.0.0.1"),
        "the failure names the missing grant:\n{report}"
    );
}
