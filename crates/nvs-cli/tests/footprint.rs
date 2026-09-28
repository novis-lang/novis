//! `NVS_FOOTPRINT_LOG` and `NOVIS_NO_FILE_CACHE`, driven through the built binary: a run records
//! the `Core` classes it looked up, the files, directories and paths it read, and its configuration
//! by part, `nvs agent show`
//! records the cards it printed, and a run with the switch set compiles without an artifact cache.
//!
//! The fixture under `tests/fixtures/footprint/` requires a second file, reads a data file, lists a
//! directory, tests a path that is not there, and calls `Core\Math`, `Core\IO` and `Core\Arr`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `tests/fixtures/footprint`.
fn fixture() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "footprint"]
        .iter()
        .collect()
}

/// A fresh directory under this test's own scratch space.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("footprint-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch directory");
    dir
}

/// `nvs <global...> run main.nvs` in the fixture, with `env` added, asserting that it succeeds, and
/// its standard error.
fn run_with(global: &[&Path], env: &[(&str, &Path)]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    for config in global {
        command.arg("--config").arg(config);
    }
    command.args(["run", "main.nvs"]).current_dir(fixture());
    // A recorded run of this test binary has both set, and each case here decides them itself.
    command.env_remove("NVS_FOOTPRINT_LOG");
    command.env_remove("NOVIS_NO_FILE_CACHE");
    for (name, value) in env {
        command.env(name, value);
    }
    let out = command
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "the fixture runs: {stdout}{stderr}");
    assert!(stdout.contains("the data file"), "{stdout}");
    stderr
}

/// `nvs run main.nvs` in the fixture, with `env` added, asserting that it succeeds.
fn run(env: &[(&str, &Path)]) {
    run_with(&[], env);
}

/// The path a log line spells for `name` inside the fixture, or for the fixture itself when `name`
/// is empty.
fn shown(name: &str) -> String {
    let path = if name.is_empty() {
        fixture()
    } else {
        fixture().join(name)
    };
    let path = std::path::absolute(path).expect("an absolute path");
    let text = path.to_string_lossy().replace('\\', "/");
    text.strip_prefix("//?/").unwrap_or(&text).to_string()
}

#[test]
fn a_run_records_the_classes_it_looked_up_and_what_it_read() {
    let dir = scratch("log");
    let log = dir.join("footprint.log");
    run(&[("NVS_FOOTPRINT_LOG", &log), ("NOVIS_NO_FILE_CACHE", &dir)]);
    let text = std::fs::read_to_string(&log).expect("the run wrote its log");
    let lines: Vec<&str> = text.lines().collect();

    for want in [
        format!("file\t{}", shown("main.nvs")),
        format!("file\t{}", shown("helper.nvs")),
        format!("config\t{}", shown("nvs.toml")),
        format!("app\t{}", shown("main.nvs")),
        format!("app\t{}", shown("")),
        format!("file\t{}", shown("data.txt")),
        format!("dir\t{}", shown("listed")),
        format!("exists\t{}", shown("absent.txt")),
        "class\tCore\\Math".to_string(),
        "class\tCore\\IO".to_string(),
        "class\tCore\\Arr".to_string(),
    ] {
        assert!(lines.contains(&want.as_str()), "`{want}` in:\n{text}");
    }
    assert!(
        !lines.contains(&format!("file\t{}", shown("nvs.toml")).as_str()),
        "a configuration file is recorded by part, never whole:\n{text}"
    );
    assert!(
        !text.contains("Core\\Uuid"),
        "a class the program never names is not recorded:\n{text}"
    );
    assert!(
        !text.contains("Footprint\\Helper"),
        "a class of the program's own is not a registry class:\n{text}"
    );
    assert!(
        !lines.iter().any(|line| line.starts_with("card\t")),
        "a program prints no card, so an edit to one never selects it:\n{text}"
    );
    let mut distinct = lines.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        lines.len(),
        "each line is written once:\n{text}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn printing_a_card_records_the_cards_it_read() {
    let dir = scratch("card");
    let log = dir.join("footprint.log");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["agent", "show", r"Core\Math"])
        .env("NVS_FOOTPRINT_LOG", &log)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&log).expect("the run wrote its log");
    assert!(text.lines().any(|line| line == "card\t*"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_run_without_the_variable_writes_no_log() {
    let dir = scratch("unset");
    run(&[("NOVIS_NO_FILE_CACHE", &dir), ("TMP", &dir), ("TEMP", &dir)]);
    let left: Vec<_> = std::fs::read_dir(&dir)
        .expect("the scratch directory")
        .collect();
    assert!(left.is_empty(), "nothing was written: {left:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A configuration granting the fixture's reads and naming `artifacts` as the cache directory.
fn cache_config(dir: &Path, artifacts: &Path) -> PathBuf {
    let config = dir.join("nvs.toml");
    let written = format!(
        "[capabilities.fs]\nread = ['{}']\n\n[opcache]\nfile_cache_dir = '{}'\n",
        fixture().display(),
        artifacts.display()
    );
    std::fs::write(&config, written).expect("the configuration is written");
    config
}

#[test]
fn the_switch_leaves_a_configured_artifact_cache_alone() {
    // A configured cache directory is either used, and holds an artifact after the run, or refused
    // by the ownership check, which says so on standard error. Either way the run consulted it.
    let on = scratch("cache-on");
    let artifacts = on.join("artifacts");
    let config = cache_config(&on, &artifacts);
    let stderr = run_with(&[&config], &[]);
    let consulted = artifacts.is_dir() || stderr.contains("file_cache_dir");
    assert!(
        consulted,
        "without the switch the run consults the cache: {stderr}"
    );

    let off = scratch("cache-off");
    let artifacts = off.join("artifacts");
    let config = cache_config(&off, &artifacts);
    let stderr = run_with(&[&config], &[("NOVIS_NO_FILE_CACHE", Path::new("1"))]);
    assert!(!artifacts.exists(), "with the switch nothing is written");
    assert!(
        !stderr.contains("file_cache_dir"),
        "with the switch the directory is never examined: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&on);
    let _ = std::fs::remove_dir_all(&off);
}
