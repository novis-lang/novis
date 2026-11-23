//! `rule:testing/in-process-request`'s second mechanism is armed like its
//! first, driven through the built binary: a request a `#[Test(server: true)]`
//! sends to its own listener reads the test's clock and `env:` variables, and
//! what it logs and mails is kept for the test.
//!
//! The listener is bound before the test's isolate exists, so each request it
//! answers is armed from what that isolate was armed with, when it arrives. A
//! listener that armed nothing read the host's clock and environment, and wrote
//! its records nowhere a test reads.

use std::path::PathBuf;
use std::process::Command;

/// `nvs test wire.nvs --filter <filter>`, run from `tests/fixtures/reach/wire/`,
/// as `(report, success)`.
fn run(filter: &str) -> (String, bool) {
    let dir: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "reach",
        "wire",
    ]
    .iter()
    .collect();
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("test")
        .arg("wire.nvs")
        .args(["--filter", filter])
        .current_dir(&dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (report, out.status.success())
}

/// Runs the one `#[Test]` method `method` and asserts that it alone ran, and
/// passed.
fn passes(method: &str) {
    let (report, passed) = run(method);
    assert!(passed, "`{method}` failed:\n{report}");
    assert!(
        report.contains(method) && report.contains("0 failed, 1 passed"),
        "`{method}` was the one test that ran, and it passed:\n{report}"
    );
}

/// A request off the wire reads the test's clock as it reads when the request
/// arrives, so a `Core\Test::advance` before it moves what it reads, and it
/// reads the test's `env:` variables.
// covers: Core\Test::serverUrl
#[test]
fn a_wire_request_reads_the_tests_clock_and_env() {
    passes("aRequestReadsTheTestsClockAndEnv");
}

/// What a request off the wire logs and mails is in the test's
/// `Core\Test::logs` and `Core\Test::sentMail`.
// covers: Core\Test::serverUrl
#[test]
fn a_wire_requests_logs_and_mail_are_kept_for_the_test() {
    passes("aRequestsLogsAndMailAreKept");
}
