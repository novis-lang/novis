//! `rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`'s
//! byte source: the files a bundled executable resolves against instead of the
//! filesystem.
//!
//! A bundle carries its whole `require` graph as source appended to the host
//! binary (§ 2). At process start `nvs-cli` reads that payload and calls
//! [`install`]; from then on the compiler's two ways of touching the disk for a
//! *source* file — [`crate::SourceMap::load`] and `nvs_hir`'s canonicalization
//! of a `require` target — answer out of this table first. Nothing else in the
//! pipeline changes, which is § 4's "the same `nvs run <entry>` code path with
//! one different byte source for reads".
//!
//! **Why a process-global and not a field on [`crate::SourceMap`].** The two
//! readers are in different crates and one of them (`nvs_hir`'s
//! `canonicalize`) is reached from a helper that holds no map. More to the
//! point, the fact being modelled *is* process-wide and write-once: a bundled
//! executable has exactly one byte source for its whole life, decided before
//! anything parses, and a payload that could be swapped mid-run would be a
//! capability this feature does not want to own. [`OnceLock`] is that shape
//! exactly, and an ordinary `nvs` never installs one.
//!
//! **The synthetic root.** Payload paths are relative (§ 2), so [`install`]
//! joins them onto a root the caller supplies — the running executable's own
//! path. That is absolute on every platform and cannot also be a directory on
//! disk, so a synthetic path can never collide with a real file, while still
//! reading as something a user recognises in a diagnostic. The name a
//! diagnostic prints is the payload's own relative path, so a bundle's errors
//! look like the source tree's, not like the machine that built it.
//!
//! **Known gap.** `autoload` probing (`rule:programs/no-runtime-autoload`) lists real directories and is
//! not routed through here, so a bundled program that reaches a name only
//! through an autoload root does not resolve it. `require` — which is what
//! `rule:packaging/a-bundled-require-resolves-at-build-time` makes the closed-world rule about — does.
//! Decided: Resolve autoload roots into the bundle at build time — A bundled program behaves like the
//! source tree, and the root set is frozen into the build.
//! — owner: decided-closures

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

/// One bundled program's files, keyed by the synthetic path they resolve at.
#[derive(Debug)]
pub struct Payload {
    /// The entry point, as [`crate::SourceMap::load`] must be handed it.
    entry: PathBuf,
    /// Synthetic path -> (the relative name a diagnostic prints, the source).
    files: HashMap<PathBuf, (String, String)>,
    /// Every ancestor of every file, so [`canonicalize`] can answer for the
    /// directory a `require` is resolved relative to as well as for a file.
    dirs: HashSet<PathBuf>,
}

impl Payload {
    /// The entry point's synthetic path — what a bundled process runs.
    #[must_use]
    pub fn entry(&self) -> &Path {
        &self.entry
    }
}

static PAYLOAD: OnceLock<Payload> = OnceLock::new();

/// Installs the payload this process runs from, and hands back its entry path.
///
/// `root` is the synthetic directory every relative path is joined onto, and
/// `files` is § 2's flat list in payload order — **the entry file first**, the
/// same contract `nvs_hir::resolve_program` states for the other direction.
///
/// # Panics
///
/// Panics if called twice, or if `files` is empty. Both are bugs in the loader
/// rather than input: the payload is read once, from bytes whose footer has
/// already been validated.
pub fn install(root: &Path, files: Vec<(String, String)>) -> &'static Path {
    assert!(!files.is_empty(), "a payload has at least its entry file");
    let mut table = HashMap::with_capacity(files.len());
    let mut dirs = HashSet::new();
    let mut entry = None;
    for (name, text) in files {
        let path = normalize(&root.join(&name));
        let mut dir = path.parent();
        while let Some(d) = dir {
            if !dirs.insert(d.to_path_buf()) {
                break;
            }
            dir = d.parent();
        }
        entry.get_or_insert_with(|| path.clone());
        table.insert(path, (name, text));
    }
    let payload = Payload {
        entry: entry.expect("checked non-empty above"),
        files: table,
        dirs,
    };
    assert!(PAYLOAD.set(payload).is_ok(), "a payload is installed once");
    get().expect("just installed").entry()
}

/// The payload this process runs from, or `None` in an ordinary `nvs`.
#[must_use]
pub fn get() -> Option<&'static Payload> {
    PAYLOAD.get()
}

/// Whether this process is a bundled executable.
#[must_use]
pub fn is_active() -> bool {
    PAYLOAD.get().is_some()
}

/// The `(display name, source)` of a bundled file, or `None` for any path this
/// payload does not carry — including every path when there is no payload.
#[must_use]
pub fn text(path: &Path) -> Option<(&'static str, &'static str)> {
    let payload = PAYLOAD.get()?;
    let (name, text) = payload.files.get(&normalize(path))?;
    Some((name.as_str(), text.as_str()))
}

/// [`Path::canonicalize`]'s answer for a bundled path: the lexically
/// normalized form, and only for a path the payload actually holds.
///
/// A closed world has no symlinks and no case folding, so normalization *is*
/// canonicalization here — which is also why `nvs_hir`'s
/// `rule:classes/names-resolve-case-sensitively`
/// case check can only ever pass inside a bundle: both sides of its comparison
/// come from this one table, spelled once.
#[must_use]
pub fn canonicalize(path: &Path) -> Option<PathBuf> {
    let payload = PAYLOAD.get()?;
    let path = normalize(path);
    (payload.files.contains_key(&path) || payload.dirs.contains(&path)).then_some(path)
}

/// `.` dropped and `..` popped, with no filesystem access.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push(Component::ParentDir);
                }
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `..` is popped and `.` dropped, which is what makes a `require "../x"`
    /// inside a bundle name the same entry the build wrote.
    #[test]
    fn normalize_pops_a_parent_and_drops_a_current_dir() {
        let joined = Path::new("/app.exe/src/./lib/../lib/db.nvs");
        assert_eq!(normalize(joined), PathBuf::from("/app.exe/src/lib/db.nvs"));
    }

    /// Nothing answers out of an uninstalled payload — an ordinary `nvs` reads
    /// every source file from disk, and this table is invisible to it.
    #[test]
    fn an_uninstalled_payload_answers_nothing() {
        assert!(!is_active());
        assert!(text(Path::new("/anything.nvs")).is_none());
        assert!(canonicalize(Path::new("/anything.nvs")).is_none());
    }
}
