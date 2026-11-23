//! `nvs test --since <ref>` runs more tests when it cannot tell, never fewer:
//! a deleted file, a renamed file, a file outside the program's folder and a
//! run that left a test out each still select the tests the change reached,
//! as `crates/nvs-cli/src/runner/reach.rs` § *What `--since` selects* states.
//!
//! Through the built binary, in a git checkout of the case's own: the
//! selection reads git's diff and the reached map a run wrote before.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The interface, and a test that adds up the score of every class that
/// implements it. It reaches `Extra::score` through
/// `Core\Program::implementing`, so deleting `Extra`'s file still compiles.
const CHECKS: &str = r#"<?nvs
use Core\Test;

interface Check
{
    public function score(): int;
}

final class CheckTest
{
    #[Test]
    public function scores(): void
    {
        var $total = 0;
        foreach (Core\Program::implementing<Check>() as Check $check) {
            $total += $check->score();
        }
        Test::assertTrue($total >= 0);
    }
}
"#;

/// A test that reaches nothing the cases change.
const PLAIN: &str = r#"<?nvs
use Core\Test;

final class PlainTest
{
    #[Test]
    public function one(): void
    {
        Test::assertSame(1, 1);
    }
}
"#;

/// The one class that implements `Check`.
const EXTRA: &str = r#"<?nvs
final class Extra implements Check
{
    public function score(): int
    {
        return 2;
    }
}
"#;

const EVERY_TEST: [&str; 2] = ["CheckTest::scores", "PlainTest::one"];

/// The folder of programs the cases run: `lib/app/`. The folder of a
/// directory of programs is the one it is in, `lib/`.
fn app(root: &Path) -> PathBuf {
    root.join("lib").join("app")
}

/// A checkout of the case's own: the program is the folder `lib/app/`, and
/// `data/prices.csv` and `gone/notes.txt` are outside the program's folder.
/// Every file is committed. Gives the checkout's root and a data folder that
/// lasts as long as the value.
fn checkout(case: &str) -> (PathBuf, nvs_repo::Scratch) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("since-selection")
        .join(case);
    let _ = fs::remove_dir_all(&root);
    for folder in ["lib/app", "data", "gone"] {
        fs::create_dir_all(root.join(folder)).expect("the target's scratch folder is writable");
    }
    fs::write(app(&root).join("a_checks.nvs"), CHECKS).unwrap();
    fs::write(app(&root).join("b_plain.nvs"), PLAIN).unwrap();
    fs::write(app(&root).join("c_extra.nvs"), EXTRA).unwrap();
    fs::write(root.join("data").join("prices.csv"), "1,2\n").unwrap();
    fs::write(root.join("gone").join("notes.txt"), "kept\n").unwrap();
    git(&root, &["init", "--quiet"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "--quiet", "-m", "start"]);
    (root, nvs_repo::scratch_private("nvsdata"))
}

/// The environment every `git` runs under, `nvs`'s own included: an empty
/// global configuration, no system one, and the parent of `root` as a
/// ceiling, so the repository's own checkout is never found.
fn git_env(root: &Path) -> Vec<(&'static str, std::ffi::OsString)> {
    let global = root.with_extension("gitconfig");
    fs::write(&global, "").unwrap();
    vec![
        ("GIT_CONFIG_GLOBAL", global.into_os_string()),
        ("GIT_CONFIG_NOSYSTEM", "1".into()),
        (
            "GIT_CEILING_DIRECTORIES",
            root.parent().unwrap().as_os_str().to_owned(),
        ),
    ]
}

/// Runs `git` with `args` in `root`, and fails the test when it fails.
fn git(root: &Path, args: &[&str]) {
    let mut command = nvs_repo::spawn("git", &[]);
    command.current_dir(root).args([
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
    ]);
    for (key, value) in git_env(root) {
        command.env(key, value);
    }
    let out = command.args(args).output().expect("`git` is on `PATH`");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Runs `nvs test . --since HEAD --format json` and `more` in the checkout's
/// `lib/app/` folder with the data folder `data`. Gives the `Class::method` of
/// every test that ran, sorted, and the standard error.
fn since_run(root: &Path, data: &Path, more: &[&str]) -> (Vec<String>, String) {
    let mut command: Command = nvs_repo::spawn(env!("CARGO_BIN_EXE_nvs"), &[]);
    command
        .arg("--data")
        .arg(data)
        .args(["test", ".", "--since", "HEAD", "--format", "json"])
        .args(more)
        .current_dir(app(root));
    for (key, value) in git_env(root) {
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

/// A deleted file's functions are missing from the program, and the tests
/// that reached them run. A test that reached none of them does not.
// covers: tools:cli/nvs-test
#[test]
fn a_deleted_file_runs_the_tests_that_reached_it() {
    let (root, data) = checkout("deleted");
    let (ran, _) = since_run(&root, &data, &[]);
    assert_eq!(ran, EVERY_TEST, "the first run records every test");
    fs::remove_file(app(&root).join("c_extra.nvs")).unwrap();
    let (ran, stderr) = since_run(&root, &data, &[]);
    assert_eq!(ran, ["CheckTest::scores"], "{stderr}");
    assert!(!stderr.contains("Every selected test runs"), "{stderr}");
}

/// A renamed file is a deleted file and a new one, and the tests that reached
/// it run.
// covers: tools:cli/nvs-test
#[test]
fn a_renamed_file_runs_the_tests_that_reached_it() {
    let (root, data) = checkout("renamed");
    since_run(&root, &data, &[]);
    git(&root, &["mv", "lib/app/c_extra.nvs", "lib/app/d_extra.nvs"]);
    let (ran, stderr) = since_run(&root, &data, &[]);
    assert!(
        ran.iter().any(|test| test == "CheckTest::scores"),
        "{ran:?}: {stderr}"
    );
}

/// A file outside the program's folder may be data a test reads, so every
/// test runs, and the first line says why.
// covers: tools:cli/nvs-test
#[test]
fn a_file_outside_the_folder_runs_every_test() {
    let (root, data) = checkout("outside");
    since_run(&root, &data, &[]);
    fs::write(root.join("data").join("prices.csv"), "1,3\n").unwrap();
    let (ran, stderr) = since_run(&root, &data, &[]);
    assert_eq!(ran, EVERY_TEST, "{stderr}");
    assert!(
        stderr.contains("changed, and it is outside the program's folder"),
        "{stderr}"
    );
}

/// A deleted file whose folder is gone has no path to judge, so every test
/// runs.
// covers: tools:cli/nvs-test
#[test]
fn a_path_that_does_not_resolve_runs_every_test() {
    let (root, data) = checkout("unresolved");
    since_run(&root, &data, &[]);
    fs::remove_dir_all(root.join("gone")).unwrap();
    let (ran, stderr) = since_run(&root, &data, &[]);
    assert_eq!(ran, EVERY_TEST, "{stderr}");
    assert!(
        stderr.contains("its path could not be resolved"),
        "{stderr}"
    );
}

/// A run that left `CheckTest` out keeps the hash `CheckTest` ran against. A
/// change committed after the first run is not in git's diff, and the next
/// run still runs `CheckTest` against it. `PlainTest` ran against its own
/// change, so it does not run again.
// covers: tools:cli/nvs-test
#[test]
fn a_run_that_left_a_test_out_keeps_the_hashes_it_reached() {
    let (root, data) = checkout("left-out");
    since_run(&root, &data, &[]);
    fs::write(
        app(&root).join("c_extra.nvs"),
        EXTRA.replace("return 2;", "return 3;"),
    )
    .unwrap();
    fs::write(
        app(&root).join("b_plain.nvs"),
        PLAIN.replace("assertSame(1, 1)", "assertSame(2, 2)"),
    )
    .unwrap();
    git(&root, &["commit", "--quiet", "-am", "edit"]);
    let (ran, stderr) = since_run(&root, &data, &["--filter", "PlainTest::"]);
    assert_eq!(ran, ["PlainTest::one"], "{stderr}");
    let (ran, stderr) = since_run(&root, &data, &[]);
    assert_eq!(ran, ["CheckTest::scores"], "{stderr}");
}
