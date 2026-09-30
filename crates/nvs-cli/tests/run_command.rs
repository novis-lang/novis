//! `nvs run`: the configuration it reads before running, and the exit status
//! it ends with, as `docs/reference/tools/10-cli.md` § *nvs run* states them.
//!
//! Through the built binary, because both are properties of the whole
//! command: which file is read is decided before any program compiles, and
//! the status is what a shell, a cron job or CI sees.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A directory of the case's own under the target's scratch, emptied first,
/// so two cases running in parallel never share a fixture.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("run-command").join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `nvs run` with `args` from `dir`, which is the program's working
/// directory and so where `./nvs.toml` is looked for.
fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("run")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

const PROGRAM: &str = "<?nvs\necho \"ran\", \"\\n\";\n";

/// `./nvs.toml` in the working directory is read, and a `--config` flag reads
/// its own file *instead*: a broken `./nvs.toml` stops a plain run with its
/// diagnostic, and the same run with `--config` never opens it. A named file
/// that does not exist is `E0605`.
// covers: tools:cli/nvs-run
#[test]
fn nvs_run_reads_the_working_directory_s_nvs_toml_unless_a_config_is_named() {
    let dir = scratch("config-lookup");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    fs::write(dir.join("nvs.toml"), "this is [ not a configuration\n").unwrap();
    fs::write(dir.join("empty.toml"), "").unwrap();

    let looked_up = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&looked_up.stderr);
    assert_eq!(looked_up.status.code(), Some(1), "the broken file stops the run: {stderr}");
    assert!(stderr.contains("E0601"), "the diagnostic names the file's error: {stderr}");
    assert!(stderr.contains("nvs.toml"), "and the file it is in: {stderr}");
    assert!(looked_up.stdout.is_empty(), "the program never ran");

    let named = run_in(&dir, &["--config", "empty.toml", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&named.stderr);
    assert_eq!(named.status.code(), Some(0), "`./nvs.toml` is not read at all: {stderr}");
    assert_eq!(String::from_utf8_lossy(&named.stdout), "ran\n");

    let missing = run_in(&dir, &["--config", "absent.toml", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert_eq!(missing.status.code(), Some(1), "a named file must exist: {stderr}");
    assert!(stderr.contains("E0605"), "{stderr}");
    assert!(missing.stdout.is_empty(), "the program never ran");

    fs::remove_file(dir.join("nvs.toml")).unwrap();
    let none = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&none.stderr);
    assert_eq!(none.status.code(), Some(0), "no `./nvs.toml` is no error: {stderr}");
    assert_eq!(String::from_utf8_lossy(&none.stdout), "ran\n");
}

/// Every row of the *Exit status* table: `exit(n)` is `n`, `exit("message")`
/// prints the message and is `0`, an uncaught throwable is `1` with its
/// log record on standard error only, and a wrong command line is `2`.
// covers: tools:cli/nvs-run
#[test]
fn nvs_run_ends_with_the_status_its_reference_table_names() {
    let dir = scratch("exit-status");
    let cases = [
        ("exits.nvs", "<?nvs\necho \"bye\", \"\\n\";\nexit(3);\n", 3, "bye\n"),
        ("says.nvs", "<?nvs\nexit(\"done\");\n", 0, "done"),
        (
            "throws.nvs",
            "<?nvs\necho \"before\", \"\\n\";\nthrow new LogicError(\"boom\");\n",
            1,
            "before\n",
        ),
    ];
    for (file, source, status, stdout) in cases {
        fs::write(dir.join(file), source).unwrap();
        let out = run_in(&dir, &[file]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(status), "{file}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), stdout, "{file}");
        if file == "throws.nvs" {
            assert!(
                stderr.contains("LogicError") && stderr.contains("boom"),
                "the record names the class and the message on standard error: {stderr}"
            );
        }
    }

    let wrong = run_in(&dir, &["--no-such-flag", "exits.nvs"]);
    assert_eq!(wrong.status.code(), Some(2), "the command line was wrong");
    assert!(wrong.stdout.is_empty(), "nothing ran");
}
