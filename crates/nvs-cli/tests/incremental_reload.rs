//! `[opcache] incremental`, the edit state a serving process keeps between two compiles of one
//! program: each source file's checked result and each function's relocatable code.
//!
//! The directive's startup default follows the mode, as `settle`'s does
//! (`rule:config/a-startup-default-is-never-flipped`), and it is `System`-class, so no request can
//! make a server keep memory or free it. The counts a reload does with it on are the
//! `program reloaded` record's, and every claim here is a count, never a clock
//! (`rule:programs/less-work-is-taken`).

use std::path::{Path, PathBuf};
use std::process::Command;

/// An empty directory of this case's own under `CARGO_TARGET_TMPDIR`.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("incremental-reload-{case}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("the case's directory is created");
    dir
}

/// The `opcache.incremental` line of `nvs config dump --origin` over `toml`, written as `dir`'s
/// `nvs.toml`, with `dir/.nvsdata` as the data folder.
fn incremental_line(dir: &Path, toml: &str) -> String {
    std::fs::write(dir.join("nvs.toml"), toml).expect("the case's directory takes a file");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(dir.join(".nvsdata"))
        .args(["config", "dump", "--origin", "nvs.toml"])
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stdout = String::from_utf8(out.stdout).expect("the audit is UTF-8");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
        .lines()
        .find(|line| line.starts_with("opcache.incremental "))
        .unwrap_or_else(|| panic!("`opcache.incremental` has a row: {stdout}"))
        .trim_end()
        .to_owned()
}

/// An unwritten `incremental` is `false` in `production` and `true` in `development`, and the
/// dump names the mode that chose it. A written value wins over the mode. No request may set it,
/// and a reload applies it.
#[test]
fn incremental_startup_default_follows_the_mode() {
    use nvs_config::directive::{Apply, lookup};

    let dir = scratch("default");

    let production = incremental_line(&dir, "[mode]\ndefault = \"production\"\n");
    assert!(
        production.contains("= false ") && production.ends_with("    default (production)"),
        "production frees the edit state: {production}"
    );

    let development = incremental_line(&dir, "[mode]\ndefault = \"development\"\n");
    assert!(
        development.contains("= true ") && development.ends_with("    default (development)"),
        "development keeps the edit state: {development}"
    );

    let written = incremental_line(
        &dir,
        "[mode]\ndefault = \"development\"\n\n[opcache]\nincremental = false\n",
    );
    assert!(
        written.contains("= false ") && written.ends_with("nvs.toml"),
        "a written value wins over the mode's default: {written}"
    );

    let row = lookup("opcache.incremental").expect("`opcache.incremental` has a registry row");
    assert_eq!(row.apply, Apply::Reload, "a reload applies it");
    assert_eq!(
        nvs_config::settable::how("opcache.incremental"),
        None,
        "it is `System`: no request may set it"
    );

    drop(std::fs::remove_dir_all(&dir));
}
