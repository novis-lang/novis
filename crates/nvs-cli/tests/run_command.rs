//! `nvs run`: the configuration it reads before running, and the exit status
//! it ends with, as `docs/reference/tools/10-cli.md` § *nvs run* states them.
//!
//! Through the built binary, because both are properties of the whole
//! command: which file is read is decided before any program compiles, and
//! the status is what a shell, a cron job or CI sees.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// A directory of the case's own, so two cases running in parallel never share a fixture, and
/// locked to this account: the data folder a run creates in it, and the `nvs.toml` the run writes
/// there, pass `rule:config/ownership-is-the-trust-boundary`'s check only under a parent that does.
fn scratch(case: &str) -> nvs_repo::Scratch {
    nvs_repo::scratch_private(case)
}

/// Runs `nvs run` with `args` from `dir`, which is the program's working
/// directory and so where `./nvs.toml` is looked for. The data folder is
/// `dir/.nvsdata`, so no case shares the one beside the binary.
fn run_in(dir: &Path, args: &[&str]) -> Output {
    run_with_data(dir, &dir.join(".nvsdata"), args)
}

/// [`run_in`] with the data folder `data`.
fn run_with_data(dir: &Path, data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(data)
        .arg("run")
        .args(args)
        .current_dir(dir)
        .env_remove("NOVIS_NO_INIT")
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

const PROGRAM: &str = "<?nvs\necho \"ran\", \"\\n\";\n";

/// `./nvs.toml` in the working directory is read, and a `--config` flag reads
/// its own file *instead*: a broken `./nvs.toml` stops a plain run with its
/// diagnostic, and the same run with `--config` never opens it. A named file
/// that does not exist is `E0605`. With no `./nvs.toml` the run writes the
/// shipped template into the data folder, never into the working directory.
// covers: tools:cli/nvs-run
#[test]
fn nvs_run_reads_the_working_directory_s_nvs_toml_unless_a_config_is_named() {
    let dir = scratch("config-lookup");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    fs::write(dir.join("nvs.toml"), "this is [ not a configuration\n").unwrap();
    fs::write(dir.join("empty.toml"), "").unwrap();

    let looked_up = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&looked_up.stderr);
    assert_eq!(
        looked_up.status.code(),
        Some(1),
        "the broken file stops the run: {stderr}"
    );
    assert!(
        stderr.contains("E0601"),
        "the diagnostic names the file's error: {stderr}"
    );
    assert!(
        stderr.contains("nvs.toml"),
        "and the file it is in: {stderr}"
    );
    assert!(looked_up.stdout.is_empty(), "the program never ran");

    let named = run_in(&dir, &["--config", "empty.toml", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&named.stderr);
    assert_eq!(
        named.status.code(),
        Some(0),
        "`./nvs.toml` is not read at all: {stderr}"
    );
    assert_eq!(String::from_utf8_lossy(&named.stdout), "ran\n");

    let missing = run_in(&dir, &["--config", "absent.toml", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert_eq!(
        missing.status.code(),
        Some(1),
        "a named file must exist: {stderr}"
    );
    assert!(stderr.contains("E0605"), "{stderr}");
    assert!(missing.stdout.is_empty(), "the program never ran");

    fs::remove_file(dir.join("nvs.toml")).unwrap();
    let none = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&none.stderr);
    assert_eq!(
        none.status.code(),
        Some(0),
        "no `./nvs.toml` is no error: {stderr}"
    );
    assert_eq!(String::from_utf8_lossy(&none.stdout), "ran\n");
    assert!(
        dir.join(".nvsdata").join("nvs.toml").is_file(),
        "the run wrote the template into the data folder"
    );
    assert!(
        !dir.join("nvs.toml").exists(),
        "and nothing into the working directory"
    );
}

/// The data folder's `nvs.toml` is read when the working directory has none,
/// and `./nvs.toml` wins when it is there. A broken data-folder file stops the
/// run and names that file, which is how the case tells which file was read.
// covers: tools:cli/nvs-run
#[test]
fn nvs_run_reads_the_data_folder_s_nvs_toml_when_the_working_directory_has_none() {
    let dir = scratch("data-lookup");
    let data = dir.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    fs::write(data.join("nvs.toml"), "this is [ not a configuration\n").unwrap();

    let from_data = run_with_data(&dir, &data, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&from_data.stderr);
    assert_eq!(
        from_data.status.code(),
        Some(1),
        "the data folder's broken file is read and stops the run: {stderr}"
    );
    assert!(stderr.contains("E0601"), "{stderr}");

    fs::write(dir.join("nvs.toml"), "").unwrap();
    let local = run_with_data(&dir, &data, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&local.stderr);
    assert_eq!(
        local.status.code(),
        Some(0),
        "`./nvs.toml` wins, so the data folder's file is not read: {stderr}"
    );
    assert_eq!(String::from_utf8_lossy(&local.stdout), "ran\n");
}

/// A relative `--data` resolves against the working directory, and
/// `--no-init` skips the file but still creates the folder.
// covers: tools:cli/nvs-run
#[test]
fn a_relative_data_folder_starts_at_the_working_directory_and_no_init_skips_only_the_file() {
    let dir = scratch("data-relative");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();

    let skipped = run_with_data(&dir, Path::new("relative"), &["--no-init", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&skipped.stderr);
    assert_eq!(skipped.status.code(), Some(0), "{stderr}");
    assert!(
        dir.join("relative").is_dir(),
        "the folder is created in the working directory"
    );
    assert!(
        !dir.join("relative").join("nvs.toml").exists(),
        "`--no-init` skips the file"
    );

    let written = run_with_data(&dir, Path::new("relative"), &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&written.stderr);
    assert_eq!(written.status.code(), Some(0), "{stderr}");
    assert!(dir.join("relative").join("nvs.toml").is_file());
    assert!(!dir.join("nvs.toml").exists());
}

/// A data folder that cannot be created prints one warning and the run goes
/// on, on the shipped defaults, writing nothing.
// covers: tools:cli/nvs-run
#[test]
fn an_unusable_data_folder_warns_once_and_the_run_succeeds() {
    let dir = scratch("data-unusable");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    // A file where a folder must be: nothing can be created under it.
    fs::write(dir.join("blocked"), "").unwrap();

    let out = run_with_data(&dir, &dir.join("blocked").join("data"), &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "the run goes on: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ran\n");
    assert_eq!(
        stderr
            .matches("warning: Novis cannot use its data folder")
            .count(),
        1,
        "exactly one warning: {stderr}"
    );
    assert!(stderr.contains("--data"), "it names the flag: {stderr}");
    assert!(!dir.join("nvs.toml").exists(), "nothing is written instead");
}

/// Every row of the *Exit status* table: `exit(n)` is `n`, `exit("message")`
/// prints the message and is `0`, an uncaught throwable is `1` with its
/// log record on standard error only, and a wrong command line is `2`.
// covers: tools:cli/nvs-run
#[test]
fn nvs_run_ends_with_the_status_its_reference_table_names() {
    let dir = scratch("exit-status");
    let cases = [
        (
            "exits.nvs",
            "<?nvs\necho \"bye\", \"\\n\";\nexit(3);\n",
            3,
            "bye\n",
        ),
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
