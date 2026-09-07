//! Captures the handful of facts `nvs info` reports that only exist at build
//! time — the target triple, the profile, the compiler, the commit and its
//! date, and the code generator's version.
//!
//! Everything here is passed through `cargo:rustc-env`, so `src/info.rs`
//! reads them with `env!` and holds no build logic of its own. Nothing in
//! this file may fail the build: a fact that cannot be determined becomes
//! the string `unknown`, because a source tarball with no `.git` is a
//! perfectly ordinary way to build Novis and is not an error.
//!
//! No build timestamp is recorded. A build date would make two builds of the
//! same commit differ, and `nvs info` is the wrong place to spend a
//! reproducible build on a cosmetic field — the commit already answers "which
//! source is this?" exactly. The commit's own committer date is recorded in
//! its place: it answers how old a binary is, and because it is derived from
//! the commit rather than read off the clock, every rebuild of one commit
//! still produces the same bytes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env("CARGO_MANIFEST_DIR"));
    // crates/nvs-cli -> the workspace root.
    let workspace = manifest
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| manifest.clone(), Path::to_path_buf);

    emit("NVS_TARGET", &env("TARGET"));
    emit("NVS_HOST", &env("HOST"));
    emit("NVS_PROFILE", &env("PROFILE"));
    emit("NVS_RUSTC", &rustc_version());
    emit("NVS_COMMIT", &commit(&workspace));
    emit("NVS_COMMIT_DATE", &commit_date(&workspace));
    emit(
        "NVS_CRANELIFT",
        &locked_version(&workspace, "cranelift-codegen"),
    );

    // The attribution file is `include_str!`d by `src/info.rs`; Cargo does
    // not track a file included from outside the crate directory on its own.
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("THIRD-PARTY-LICENSES.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("LICENSE").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("Cargo.lock").display()
    );
    println!("cargo:rerun-if-env-changed=NVS_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=NVS_BUILD_COMMIT_DATE");
}

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

fn emit(key: &str, value: &str) {
    // A newline here would forge a second instruction to Cargo, and every
    // value below comes from a subprocess or the environment.
    let sanitised = value.replace(['\n', '\r'], " ");
    println!("cargo:rustc-env={key}={sanitised}");
}

/// `rustc --version`, via the compiler Cargo is actually driving.
///
/// `RUSTC` rather than a bare `rustc`: under `cargo +nightly`, a rustup
/// override or a distribution wrapper, the two are different programs and
/// only the first one built this binary.
fn rustc_version() -> String {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    run(Command::new(rustc).arg("--version")).unwrap_or_else(unknown)
}

/// The short commit hash, with `-dirty` appended if the tree was modified.
///
/// `NVS_BUILD_COMMIT` wins if set, so a distribution packaging Novis from a
/// tarball can supply the revision it built from without a `.git` present.
fn commit(workspace: &Path) -> String {
    if let Ok(supplied) = std::env::var("NVS_BUILD_COMMIT")
        && !supplied.trim().is_empty()
    {
        return supplied.trim().to_owned();
    }

    let git = workspace.join(".git");
    if !git.exists() {
        return unknown();
    }
    // Re-run when the checked-out commit moves. `.git/index` covers a
    // `git add`, which is what flips the dirty marker below; neither path
    // existing is fine, Cargo simply ignores a missing dependency.
    println!("cargo:rerun-if-changed={}", git.join("HEAD").display());
    println!("cargo:rerun-if-changed={}", git.join("index").display());

    let Some(hash) =
        run(Command::new("git")
            .current_dir(workspace)
            .args(["rev-parse", "--short=9", "HEAD"]))
    else {
        return unknown();
    };

    let dirty = run(Command::new("git").current_dir(workspace).args([
        "status",
        "--porcelain",
        "--untracked-files=no",
    ]))
    .is_some_and(|status| !status.is_empty());

    if dirty { format!("{hash}-dirty") } else { hash }
}

/// The commit's own committer date, as `YYYY-MM-DD`.
///
/// The commit's date and never the build's, which is what lets the banner say
/// how old a binary is without costing the reproducible build this file's
/// module doc protects: the value is derived from the commit, so every
/// rebuild of one commit emits the same string. `NVS_BUILD_COMMIT_DATE` is
/// the tarball escape hatch `NVS_BUILD_COMMIT` already is for the hash.
fn commit_date(workspace: &Path) -> String {
    if let Ok(supplied) = std::env::var("NVS_BUILD_COMMIT_DATE")
        && !supplied.trim().is_empty()
    {
        return supplied.trim().to_owned();
    }
    if !workspace.join(".git").exists() {
        return unknown();
    }
    // `commit` above already registered the rerun triggers for a moved HEAD.
    run(Command::new("git")
        .current_dir(workspace)
        .args(["log", "-1", "--format=%cs", "HEAD"]))
    .unwrap_or_else(unknown)
}

/// The version of `name` recorded in the workspace `Cargo.lock`.
///
/// Cranelift exposes no version constant of its own and a build script
/// cannot read a dependency's `CARGO_PKG_VERSION`, so the lockfile — which
/// is committed, and is what resolved the build — is the honest source.
/// Parsed by hand rather than with a TOML dependency: the two lines that
/// matter have a fixed shape, and this is a build script for a compiler
/// whose supply chain is the thing it advertises.
fn locked_version(workspace: &Path, name: &str) -> String {
    let Ok(lock) = std::fs::read_to_string(workspace.join("Cargo.lock")) else {
        return unknown();
    };
    let mut in_package = false;
    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            in_package = false;
        } else if let Some(value) = line.strip_prefix("name = ") {
            in_package = value.trim_matches('"') == name;
        } else if in_package && let Some(value) = line.strip_prefix("version = ") {
            return value.trim_matches('"').to_owned();
        }
    }
    unknown()
}

/// Runs a command, returning its trimmed stdout, or `None` for any failure.
///
/// "Any failure" is the point: the program missing, a non-zero exit and
/// non-UTF-8 output are all just facts we do not have.
fn run(command: &mut Command) -> Option<String> {
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn unknown() -> String {
    "unknown".to_owned()
}
