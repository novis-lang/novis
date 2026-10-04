//! `nvs check` with the machine's own configuration in front of it —
//! `rule:core-classes/db-compile-time-query-checking`'s check-time question and the
//! configuration error that outranks it.
//!
//! Through the built binary rather than by calling `front_end_granted`, for the
//! reason [`bundle`](bundle) already writes down: `nvs-cli` is a binary crate
//! with no library target. It is also the only way to ask this particular
//! question, because what is under test is *which `nvs.toml` a command resolves*
//! — `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 2 reads `./nvs.toml` out of the working directory, and a
//! working directory is a property of a process rather than of a function call.
//!
//! Each test owns a private directory for that reason: the tests run
//! concurrently and the fixture is the child's `cwd`.

use std::path::Path;
use std::process::{Command, Output};

/// A program whose one statement is § 18's `Core\Db::open` with a **literal**
/// host, which is what makes the grant a question the compiler can answer.
const OPENS_A_LITERAL_HOST: &str = r#"<?nvs
var $far = Core\Db::open({
    driver: Core\Db\Driver::Postgres,
    host: "db.example.test",
    database: "shop",
    user: "app",
    password: "hunter2",
});
"#;

/// A fresh directory holding `case.nvs`, plus `nvs.toml` when `config` names
/// one. The name is the test's, so two tests never share a working directory.
fn fixture(name: &str, config: Option<&str>) -> nvs_repo::Scratch {
    let dir = nvs_repo::scratch(&format!("check-grants-{name}"));
    std::fs::write(dir.join("case.nvs"), OPENS_A_LITERAL_HOST).expect("the program is written");
    if let Some(config) = config {
        std::fs::write(dir.join("nvs.toml"), config).expect("the configuration is written");
    }
    dir
}

/// `nvs check case.nvs`, run *in* `dir` so `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 2 finds the
/// fixture's own `nvs.toml` and no other.
fn check(dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["check", "case.nvs"])
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// Both streams together: a diagnostic is rendered to standard error and
/// `no errors` to standard output, and a test that asserts on one of them
/// should not have to know which.
fn shown(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn nvs_check_reports_an_open_host_no_grant_covers() {
    // `rule:core-classes/db-compile-time-query-checking`: `nvs.toml` is read at boot on the machine that compiles, so
    // both halves of `db.open`'s question — the grant and the host — are facts
    // before the program runs. The grant here covers a different host, which is
    // what makes this a walk of the list rather than a check that one is
    // present.
    let dir = fixture(
        "ungranted",
        Some("[capabilities.db]\nopen = [\"db.granted.test\"]\n"),
    );
    let out = check(&dir);
    let shown = shown(&out);
    assert!(
        !out.status.success(),
        "an ungranted literal host checked clean: {shown}"
    );
    assert!(
        shown.contains("E0618") && shown.contains("db.example.test"),
        "the refusal did not name the host or its code: {shown}"
    );
}

#[test]
fn nvs_check_with_no_config_reports_no_grant_diagnostic() {
    // The goal's § *Standing decisions* item 6, and `check_program_granted`'s own
    // doc: no configuration read is not an empty grant set. A program checked
    // outside any project root is measured against nothing, because the door is
    // still `nvs_runtime::capability::require` and this pass only ever moves one
    // of its refusals earlier (`rule:expressions/preparation-preserves-behaviour`).
    let dir = fixture("no-config", None);
    let out = check(&dir);
    let shown = shown(&out);
    assert!(
        out.status.success(),
        "a program with no configuration to measure it was refused: {shown}"
    );
    assert!(
        !shown.contains("E0618"),
        "a grant was asked about with nothing granting: {shown}"
    );
}

#[test]
fn nvs_check_reports_a_broken_nvs_toml_as_a_config_error() {
    // Item 6's other half: a command that reads configuration is a command a
    // broken configuration can fail. The program in this directory *also* has an
    // ungranted host, so the assertion that E0618 is absent is what says the
    // configuration error arrived first rather than merely arrived.
    let dir = fixture("broken-config", Some("[capabilities.db\nopen = [\n"));
    let out = check(&dir);
    let shown = shown(&out);
    assert!(
        !out.status.success(),
        "a `nvs.toml` that does not parse checked clean: {shown}"
    );
    assert!(
        shown.contains("nvs.toml"),
        "the failure did not name the configuration file: {shown}"
    );
    assert!(
        !shown.contains("E0618") && !shown.contains("no errors"),
        "the program was checked against half a tree: {shown}"
    );
}
