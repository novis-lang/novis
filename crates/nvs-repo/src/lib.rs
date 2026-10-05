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
//! removed. [`named`] records `**/<name>`, which stands for every file of that name anywhere in
//! the tree. Without the variable set -- a bare `cargo test` -- nothing is written and the
//! functions only build paths.
//!
//! **This is the only way out.** A test source that joins `..` onto `CARGO_MANIFEST_DIR`, walks
//! up with `.parent()`, reads its working directory or starts a process with `Command::new`
//! cannot be shown to stay inside its package, and `bun nv verify` then runs its binary on
//! every change to the tree. `tools/data/impact-wide.txt` lists each such binary and the reason.
//!
//! Each path is also written to the footprint log `NVS_FOOTPRINT_LOG` names ([`nvs_footprint`]):
//! a path as a `tree` line, which stands for everything beneath it, and `**/<name>` as a `named`
//! line. A recorded test binary then has one log for the classes, files and paths its own code and
//! the `nvs` processes it starts used.
//!
//! A log that is named and cannot be written panics: a read that went unrecorded is a test that
//! is later skipped over a change it depends on, which is the one failure this crate exists to
//! prevent.
//!
//! [`scratch`] is where a test writes. It returns a fresh directory under `target/test-scratch/`
//! and deletes it when the guard drops, also when the test panics. No test writes into the system
//! temp directory. A scratch directory is written and never read from the tree, so it records
//! nothing, and a test that only uses one stays narrow. [`scratch_private`] is the same directory
//! locked to this account, for a test that runs the configuration's ownership check against it.
//! [`socket`] is a scratch directory and the absolute path of a socket inside it, checked against
//! the platform's limit on a socket path's length.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    match rel.strip_prefix("**/") {
        Some(name) => nvs_footprint::named(name),
        None if rel == WHOLE_TREE => nvs_footprint::tree(&repository()),
        None => nvs_footprint::tree(&repository().join(rel)),
    }
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

/// Every file called `name` in the repository, sorted, and the record `**/<name>`: this binary
/// reads each of them, and would read one that arrived anywhere. The walk leaves out `target`,
/// `node_modules` and every directory whose name starts with `.`, which hold no file of ours.
///
/// # Panics
///
/// If `name` is empty or holds a separator, if a directory cannot be listed, and if the log
/// `NVS_READS_LOG` names cannot be written.
#[must_use]
pub fn named(name: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, name: &str, out: &mut Vec<PathBuf>) {
        let entries =
            std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            let file = entry.file_name();
            if path.is_dir() {
                let skipped = file == "target"
                    || file == "node_modules"
                    || file.to_string_lossy().starts_with('.');
                if !skipped {
                    walk(&path, name, out);
                }
            } else if file == name {
                out.push(path);
            }
        }
    }
    assert!(
        !name.is_empty() && !name.contains(['/', '\\']),
        "nvs_repo::named takes a file name, and `{name}` is not one"
    );
    record(&format!("**/{name}"));
    let mut out = Vec::new();
    walk(&repository(), name, &mut out);
    out.sort();
    out
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

/// The directory [`scratch`] or [`scratch_private`] made. It derefs to the directory's path, and
/// dropping it deletes the directory and everything in it. A test that needs the directory across
/// two steps keeps the guard alive for both.
#[derive(Debug)]
pub struct Scratch {
    path: PathBuf,
    /// What the drop deletes: `path` itself, or the locked directory around it.
    root: PathBuf,
}

impl Scratch {
    /// The directory, which exists and was empty when the guard was made.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    /// A file still open on Windows can stop the delete. Drop cannot report that, and a panic
    /// here while a failing test unwinds would abort the binary, so what is left stays under
    /// `target/test-scratch/` for `cargo clean`.
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A new empty directory `target/test-scratch/<name>-<pid>-<n>` for a test to write into, where
/// `n` counts the calls in this process, so two tests that pass the same `name` never share one.
/// Nothing is recorded: the directory is written, never read from the tree.
///
/// # Panics
///
/// If `name` is empty or holds a separator or `..`, and if the directory cannot be created.
#[must_use]
pub fn scratch(name: &str) -> Scratch {
    let path = fresh(name);
    Scratch {
        root: path.clone(),
        path,
    }
}

/// As [`scratch`], for a test that runs `rule:config/ownership-is-the-trust-boundary`'s check
/// against what it writes. That check reads a path and its parent, and `target/` may inherit a
/// write grant to a group from wherever the checkout sits, so a plain scratch directory can fail
/// it on one machine and pass on another. This one is `case/` inside a directory only this
/// account can reach: mode `0700` on Unix, and on Windows a DACL with inheritance cut and one
/// entry, `OWNER RIGHTS` full control for this directory and everything created in it. The entry
/// is a SID, because account names are localized, and `OWNER RIGHTS` names whoever owns each
/// file, which is this account for everything the test creates. Both directories are deleted
/// when the guard drops.
///
/// One `icacls` process per call on Windows, so a test that needs no trust check uses
/// [`scratch`].
///
/// # Panics
///
/// As [`scratch`], and if the directory cannot be locked.
#[must_use]
pub fn scratch_private(name: &str) -> Scratch {
    let root = fresh(name);
    lock(&root);
    let path = root.join("case");
    std::fs::create_dir(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    // A Windows DACL is inherited by `case/`, and a Unix mode is not.
    #[cfg(unix)]
    lock(&path);
    Scratch { path, root }
}

/// The longest socket path the platform binds, counted in bytes: `sun_path` is 108 bytes on
/// Linux and 104 on macOS and the BSDs, and one of them is the terminating NUL.
#[cfg(any(target_os = "linux", target_os = "android"))]
const SOCKET_PATH_MAX: usize = 107;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
const SOCKET_PATH_MAX: usize = 103;

/// A new empty directory `target/test-scratch/sock-<pid>-<n>` and the absolute path of a socket
/// called `name` inside it, for a test that binds a Unix socket. The path is built from the
/// repository root with no `..` in it, because `rule:config/ownership-is-the-trust-boundary`'s
/// check walks a socket's parent directories and a relative path cuts that walk short. The
/// directory is deleted when the guard drops; the socket file is not created.
///
/// # Panics
///
/// If `name` is empty or holds a separator or `..`, if the directory cannot be created, and on
/// Unix if the path is longer than a socket path may be there. That message names the limit:
/// the fix is a checkout at a shorter path.
#[must_use]
pub fn socket(name: &str) -> (Scratch, PathBuf) {
    assert!(
        !name.is_empty() && !name.contains(['/', '\\']) && !name.contains(".."),
        "nvs_repo::socket takes a file name, and `{name}` is not one"
    );
    let dir = fresh_under("sock");
    let path = dir.join(name);
    let dir = Scratch {
        root: dir.clone(),
        path: dir,
    };
    #[cfg(unix)]
    fits_a_socket(&path);
    (dir, path)
}

/// Stops the test when `path` is too long to bind as a socket.
#[cfg_attr(not(unix), allow(dead_code))]
fn fits_a_socket(path: &Path) {
    let len = path.as_os_str().len();
    assert!(
        len <= SOCKET_PATH_MAX,
        "nvs_repo::socket: `{}` is {len} bytes, and a socket path on this platform is at most \
         {SOCKET_PATH_MAX}. Move the checkout to a shorter path.",
        path.display()
    );
}

/// `target/test-scratch/<name>-<pid>-<n>`, created empty.
fn fresh(name: &str) -> PathBuf {
    assert!(
        !name.is_empty() && !name.contains(['/', '\\']) && !name.contains(".."),
        "nvs_repo::scratch takes a directory name, and `{name}` is not one"
    );
    fresh_under(name)
}

/// `<root>/target/test-scratch/<name>-<pid>-<n>`, created empty, where `n` counts the calls in
/// this process. The root is the repository's with no `..` in it, so the path is the one a
/// program that normalizes it prints back, and every byte of a socket path is one it needs.
fn fresh_under(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("a package sits two directories below the repository root")
        .join("target")
        .join("test-scratch")
        .join(format!("{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    path
}

/// Mode `0700`.
#[cfg(unix)]
fn lock(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
        .unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
}

/// The DACL [`scratch_private`] describes.
#[cfg(windows)]
fn lock(dir: &Path) {
    let done = Command::new("icacls")
        .arg(dir)
        .args(["/inheritance:r", "/grant:r", "*S-1-3-4:(OI)(CI)F"])
        .output()
        .unwrap_or_else(|err| panic!("icacls: {err}"));
    assert!(
        done.status.success(),
        "icacls on {}: {}",
        dir.display(),
        String::from_utf8_lossy(&done.stdout),
    );
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

    #[test]
    fn named_finds_every_file_of_that_name_and_none_under_target() {
        let found = named("Cargo.toml");
        assert!(found.contains(&repository().join("Cargo.toml")));
        assert!(found.contains(&repository().join("crates/nvs-repo/Cargo.toml")));
        assert!(
            found
                .iter()
                .all(|p| !p.components().any(|c| c.as_os_str() == "target"))
        );
    }

    #[test]
    fn each_path_asked_for_is_a_footprint_line() {
        let ((), lines) = nvs_footprint::capture(|| {
            let _ = path("crates/nvs-repo");
            let _ = named("Cargo.toml");
        });
        let dir = nvs_footprint::shown(&repository().join("crates/nvs-repo"));
        assert_eq!(
            lines,
            [format!("tree\t{dir}"), "named\tCargo.toml".to_string()]
        );
    }

    #[test]
    #[should_panic(expected = "a file name")]
    fn named_refuses_a_path() {
        let _ = named("crates/Cargo.toml");
    }

    #[test]
    fn a_scratch_dir_is_under_the_target_dir() {
        let (dirs, lines) = nvs_footprint::capture(|| (scratch("probe"), scratch("probe")));
        let under = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .join("target")
            .join("test-scratch");
        assert!(
            dirs.0.is_dir() && dirs.0.starts_with(&under),
            "{}",
            dirs.0.display()
        );
        assert!(
            !dirs
                .0
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        );
        assert_ne!(dirs.0.path(), dirs.1.path());
        assert_eq!(std::fs::read_dir(dirs.0.path()).unwrap().count(), 0);
        assert!(lines.is_empty(), "scratch recorded {lines:?}");
    }

    #[test]
    fn a_scratch_dir_is_removed_when_its_guard_drops() {
        let dir = scratch("dropped");
        let kept = dir.to_path_buf();
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        drop(dir);
        assert!(!kept.exists());

        let failed = std::panic::catch_unwind(|| {
            let dir = scratch("failed");
            std::fs::write(dir.join("a.txt"), "a").unwrap();
            panic!("{}", dir.display());
        });
        let message = failed.unwrap_err();
        let shown = message.downcast_ref::<String>().unwrap();
        assert!(!Path::new(shown).exists());
    }

    #[test]
    fn a_private_scratch_dir_is_removed_with_the_dir_around_it() {
        let dir = scratch_private("private");
        let root = dir.parent().unwrap().to_path_buf();
        assert!(dir.is_dir() && dir.ends_with("case"));
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        drop(dir);
        assert!(!root.exists());
    }

    #[test]
    #[should_panic(expected = "a directory name")]
    fn scratch_refuses_a_path() {
        let _ = scratch("../elsewhere");
    }

    #[test]
    fn a_socket_path_is_under_the_target_dir() {
        let (dir, sock) = socket("control.sock");
        assert!(sock.is_absolute(), "{}", sock.display());
        assert!(
            !sock
                .components()
                .any(|c| c == std::path::Component::ParentDir),
            "{}",
            sock.display()
        );
        assert_eq!(sock.parent(), Some(dir.path()));
        assert!(dir.is_dir() && !sock.exists());
        assert!(
            sock.ends_with(
                Path::new("target")
                    .join("test-scratch")
                    .join(dir.file_name().unwrap())
                    .join("control.sock")
            )
        );
        let kept = dir.to_path_buf();
        drop(dir);
        assert!(!kept.exists());
    }

    #[test]
    #[should_panic(expected = "a socket path on this platform is at most")]
    fn a_socket_path_the_platform_cannot_bind_stops_with_the_limit() {
        let long = Path::new("/").join("x".repeat(SOCKET_PATH_MAX));
        fits_a_socket(&long);
    }
}
