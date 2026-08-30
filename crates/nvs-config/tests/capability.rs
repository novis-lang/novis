//! ADR 0118 § 1's question, asked of the capability `m6.md`'s *Verify* names on its own: a
//! `spawn script` whose tree never granted `script.spawn`.
//!
//! The check that opens this case names `-p nvs-config`, so what is asserted here is the **rule** —
//! a [`Capabilities`], a [`Cap`] and a [`Scope`], with no compiler and no request in front of them.
//! The refusal a program *sees* is `nvs_runtime::capability::require`'s and belongs to
//! `-p nvs-stdlib`; keeping the two apart is `capability.rs`'s own module doc.

use std::path::{Component, Path, PathBuf};

use nvs_config::capability::{Cap, Scope};
use nvs_config::resolve::Files;
use nvs_config::tree::Capabilities;
use nvs_config::trust::Untrusted;

/// A path written the way an ADR writes one, as a path the host spells its own way.
fn p(path: &str) -> PathBuf {
    lexical(Path::new(path))
}

/// A path spelled host-natively and **left as written**, `..` and all: the escape half of § 4 only
/// means anything if the case can hand `allows` a path nothing has resolved yet.
fn raw(path: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for component in Path::new(path).components() {
        out.push(component.as_os_str());
    }
    out
}

/// `path` with its `.` and `..` components resolved away — every escape a fake filesystem has to
/// answer for, since it has no symlink of its own until [`Disk::linking`] gives it one.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The filesystem § 4's canonicalizer asks: a fixed set of paths that exist, plus any symlink the
/// case wanted, and nothing else. It is a parameter of `allows` precisely so this can stand in for
/// `trust::canonical` without the case having to create a real directory to be refused out of.
struct Disk {
    /// Every path that exists, already canonical.
    real: Vec<PathBuf>,
    /// A symlink, as the path it is written and the canonical path it lands on.
    links: Vec<(PathBuf, PathBuf)>,
}

impl Disk {
    /// A disk holding exactly these paths.
    fn of(paths: &[&str]) -> Self {
        Self {
            real: paths.iter().map(|path| p(path)).collect(),
            links: Vec::new(),
        }
    }

    /// The same disk, plus a symlink at `at` landing on `target`.
    fn linking(mut self, at: &str, target: &str) -> Self {
        self.links.push((p(at), p(target)));
        self
    }
}

impl Files for Disk {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        self.canonical(path).map_err(Untrusted::Unreadable)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        let asked = lexical(path);
        if let Some((_, target)) = self.links.iter().find(|(at, _)| *at == asked) {
            return Ok(target.clone());
        }
        if self.real.contains(&asked) {
            Ok(asked)
        } else {
            Err(format!("no such path: {}", asked.display()))
        }
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        self.canonical(path).map(|_| String::new())
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.read(path).map(String::into_bytes)
    }

    fn exposure(&self, _path: &Path) -> Option<String> {
        None
    }

    fn list(&self, _dir: &Path) -> Result<Vec<PathBuf>, String> {
        Ok(self.real.clone())
    }

    fn exists(&self, path: &Path) -> bool {
        self.canonical(path).is_ok()
    }
}

/// The `[capabilities]` block `text` spells, canonicalized once as the snapshot's build does it —
/// the grant side of § 4's canonicalise-then-prefix, which the comparison in `allows` is written to
/// assume has already happened.
fn granting(text: &str, disk: &Disk) -> Capabilities {
    let mut caps: Capabilities =
        toml::from_str(text).unwrap_or_else(|err| panic!("refused: {err}"));
    caps.canonicalize(disk);
    caps
}

/// `spawn script` without `script.spawn` fails, in every spelling of "without" ADR 0118 § 1 denies
/// by default, and in the two ways a granted tree still says no to *this* target.
///
/// The granted case is asserted first so that every `false` below is the absence of the grant and
/// not the harness answering `false` to everything.
#[test]
fn spawn_script_without_the_capability_fails() {
    let disk = Disk::of(&[
        "/srv/app",
        "/srv/app/job.nvs",
        "/srv/other",
        "/srv/other/job.nvs",
    ])
    .linking("/srv/app/link.nvs", "/srv/other/job.nvs");
    let target = p("/srv/app/job.nvs");

    let granted = granting("[script]\nspawn = [\"/srv/app\"]\n", &disk);
    assert!(granted.allows(Cap::ScriptSpawn, Scope::Path(target.as_path()), &disk));

    // Deny by default, one spelling of "nothing granted" per row. The last is the one that is not
    // an omission at all: reading a file is not permission to run it (ADR 0006 § 5), so a tree
    // granting `fs.read` over the very same root still refuses the spawn.
    for (why, text) in [
        ("no `[capabilities]` block at all", ""),
        ("a `script` block granting nothing", "[script]\n"),
        ("`spawn = false`", "[script]\nspawn = false\n"),
        ("an empty list", "[script]\nspawn = []\n"),
        (
            "`fs.read` over the same root",
            "[fs]\nread = [\"/srv/app\"]\n",
        ),
    ] {
        let caps = granting(text, &disk);
        assert!(
            !caps.allows(Cap::ScriptSpawn, Scope::Path(target.as_path()), &disk),
            "{why} granted `script.spawn` for {}",
            target.display(),
        );
        // And not as an argument the check happened to reject: the grant itself is empty, so the
        // question with no argument in it is refused too.
        assert!(
            !caps.allows(Cap::ScriptSpawn, Scope::Unscoped, &disk),
            "{why} granted `script.spawn` unscoped",
        );
    }

    // Granted, but not here. § 4 canonicalizes the argument before the prefix test, so the two ways
    // out of a granted root — a `..` and a symlink pointing across — are the same refusal as a
    // target that never claimed to be inside one.
    for (why, outside) in [
        ("a sibling root", p("/srv/other/job.nvs")),
        (
            "a `..` back out of the root",
            raw("/srv/app/../other/job.nvs"),
        ),
        ("a symlink inside the root", p("/srv/app/link.nvs")),
    ] {
        assert!(
            !granted.allows(Cap::ScriptSpawn, Scope::Path(outside.as_path()), &disk),
            "`script.spawn = [\"/srv/app\"]` reached {} through {why}",
            outside.display(),
        );
    }
}
