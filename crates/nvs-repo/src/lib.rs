//! How a test reaches a file outside its own package.
//!
//! A test binary is run again only when something it reads has changed, and what it is compiled
//! from is known without asking: `tools/nv/keys/` takes that from cargo and rustc. What it
//! opens while it runs is not, so a test that leaves its package says so here. [`path`] returns
//! the path and records that it was asked for; the runner names a file in `NVS_READS_LOG`, and
//! every path asked for is appended to it, one per line, relative to the repository root.
//!
//! A recorded directory stands for every file beneath it, so a test that walks
//! `tests/conformance` records that one line and is run again when a case is added, changed or
//! removed. Without the variable set -- a bare `cargo test` -- nothing is written and the
//! functions only build paths.
//!
//! **This is the only way out.** A test source that joins `..` onto `CARGO_MANIFEST_DIR`, walks
//! up with `.parent()`, reads its working directory or starts a process with `Command::new`
//! cannot be shown to stay inside its package, and `bun nv verify` then runs its binary on
//! every change to the tree. `tools/data/impact-wide.txt` lists each such binary and the reason.
//!
//! A log that is named and cannot be written panics: a read that went unrecorded is a test that
//! is later skipped over a change it depends on, which is the one failure this crate exists to
//! prevent.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// The environment variable naming the file each recorded path is appended to.
pub const LOG_ENV: &str = "NVS_READS_LOG";

/// What [`root`] records: the whole tree, which `tools/nv/keys/` reads as "run this binary on
/// any change".
pub const WHOLE_TREE: &str = ".";

static LOG: Mutex<()> = Mutex::new(());

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn record(rel: &str) {
    let Some(log) = std::env::var_os(LOG_ENV) else {
        return;
    };
    let line = format!("{}\n", rel.replace('\\', "/"));
    let _held = LOG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let written = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .and_then(|mut file| file.write_all(line.as_bytes()));
    if let Err(error) = written {
        panic!(
            "nvs_repo: `{rel}` could not be recorded in {}: {error}",
            Path::new(&log).display()
        );
    }
}

/// The path of `rel`, which is written relative to the repository root with `/` between its
/// segments -- `"tests/conformance"`, `"docs/spec/02-php-migration.md"` -- and the record that
/// this binary reads it. A directory stands for everything beneath it.
///
/// # Panics
///
/// If `rel` is absolute or holds a `..` segment, since neither names a place in the tree, and
/// if the log `NVS_READS_LOG` names cannot be written.
#[must_use]
pub fn path(rel: &str) -> PathBuf {
    let inside = !rel.is_empty()
        && !Path::new(rel).is_absolute()
        && rel
            .split(['/', '\\'])
            .all(|part| part != ".." && !part.is_empty());
    assert!(
        inside,
        "nvs_repo::path takes a path inside the repository, and `{rel}` is not one"
    );
    record(rel);
    repository().join(rel)
}

/// The repository root, recorded as [`WHOLE_TREE`]: a test that asks for it may open anything,
/// so its binary runs on every change. Ask [`path`] for the directory that is needed instead
/// wherever one can be named.
///
/// # Panics
///
/// If the log `NVS_READS_LOG` names cannot be written.
#[must_use]
pub fn root() -> PathBuf {
    record(WHOLE_TREE);
    repository()
}

/// A [`Command`] for `program`, and the record of what that process reads in the tree, each
/// entry written as [`path`] takes it. The list is the caller's statement about the program it
/// starts: a script that walks `crates/` is `&["crates", "tools"]`, and a program that opens
/// nothing in the tree is `&[]`.
///
/// # Panics
///
/// As [`path`], for each entry of `reads`.
pub fn spawn(program: impl AsRef<OsStr>, reads: &[&str]) -> Command {
    for rel in reads {
        let _ = path(rel);
    }
    Command::new(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_under_the_repository_root() {
        assert!(path("Cargo.toml").is_file());
        assert!(path("crates/nvs-repo").is_dir());
    }

    #[test]
    #[should_panic(expected = "inside the repository")]
    fn a_path_that_climbs_is_refused() {
        let _ = path("crates/../../elsewhere");
    }

    #[test]
    #[should_panic(expected = "inside the repository")]
    fn an_empty_path_is_refused() {
        let _ = path("");
    }
}
