//! `nvs test <tree> --cases <file>` and `--record <dir>`, driven through the built binary: a case
//! list runs only the listed cases of a tree, judged and summed up as a whole-tree run is, and a
//! recording run leaves each case's footprint log under the case's name.
//!
//! The tree under `tests/fixtures/record/` holds two cases, one in a subdirectory.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `tests/fixtures/record`.
fn tree() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "record"]
        .iter()
        .collect()
}

/// A fresh directory under this test's own scratch space.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("nvst-select-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch directory");
    dir
}

/// `nvs test <tree> <args...>`, as `(stdout, stderr, success)`, with the cases' working
/// directories under `tmp`.
fn nvs_test(args: &[&std::ffi::OsStr], tmp: &Path) -> (String, String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("test")
        .arg(tree())
        .args(args)
        .env("TMP", tmp)
        .env("TEMP", tmp)
        .env("TMPDIR", tmp)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

#[test]
fn a_case_list_runs_only_the_listed_cases() {
    let dir = scratch("list");
    let list = dir.join("cases.txt");
    let listed = tree().join("nested").join("text.nvst");
    std::fs::write(&list, format!("{}\n\n", listed.display())).expect("the list is written");

    let (out, err, ok) = nvs_test(&["--cases".as_ref(), list.as_os_str()], &dir);
    assert!(ok, "the listed case passes: {out}{err}");
    assert_eq!(out.trim_end(), "1 passed, 0 failed, 0 skipped", "{err}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_case_list_naming_a_case_the_tree_lacks_runs_nothing() {
    let dir = scratch("stray");
    let list = dir.join("cases.txt");
    std::fs::write(&list, tree().join("gone.nvst").display().to_string())
        .expect("the list is written");

    let (out, err, ok) = nvs_test(&["--cases".as_ref(), list.as_os_str()], &dir);
    assert!(!ok, "a stray case is an error: {out}");
    assert!(err.contains("not a case of the named trees"), "{err}");
    assert!(!out.contains("passed"), "nothing ran: {out}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_recording_run_leaves_each_cases_log_under_its_name() {
    let dir = scratch("record");
    let records = dir.join("records");
    let (out, err, ok) = nvs_test(&["--record".as_ref(), records.as_os_str()], &dir);
    assert!(ok, "both cases pass: {out}{err}");
    assert_eq!(out.trim_end(), "2 passed, 0 failed, 0 skipped");

    let mut logs: Vec<String> = std::fs::read_dir(&records)
        .expect("the record directory was made")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        // A binary built with coverage also leaves each process's counters here.
        .filter(|name| !name.ends_with(".profraw"))
        .collect();
    logs.sort();
    assert_eq!(logs.len(), 2, "{logs:?}");
    // A case's name is its path as the report names it, cut and hashed when it is long, and how long
    // it is depends on where the checkout is.
    let log_of = |parts: &[&str]| {
        let mut path = tree();
        path.extend(parts);
        format!("{}.log", nvs_test::record_name(&path.display().to_string()))
    };
    let math = log_of(&["math.nvst"]);
    assert!(
        logs.contains(&math),
        "the square-root case's log {math}: {logs:?}"
    );
    let text_log = log_of(&["nested", "text.nvst"]);
    assert!(logs.contains(&text_log), "{text_log}: {logs:?}");
    let text = std::fs::read_to_string(records.join(math)).expect("the log is readable");
    assert!(
        text.lines().any(|line| line == "class\tCore\\Math"),
        "{text}"
    );
    assert!(
        !text.contains("Core\\Str"),
        "one case's log holds only its own run: {text}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_program_takes_neither_a_case_list_nor_a_record_directory() {
    let dir = scratch("program");
    let program = dir.join("suite.nvs");
    std::fs::write(&program, "<?nvs\n").expect("the program is written");
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("test")
        .arg(&program)
        .arg("--record")
        .arg(dir.join("records"))
        .current_dir(&dir)
        .output()
        .expect("the binary runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        err.contains("`--cases` and `--record` run `.nvst` cases"),
        "{err}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
