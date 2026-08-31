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
    /// What a relative path is resolved against, empty until a case says otherwise — which leaves
    /// every relative path spelled as it was written, the way this fake behaved before it had one.
    cwd: PathBuf,
}

impl Disk {
    /// A disk holding exactly these paths.
    fn of(paths: &[&str]) -> Self {
        Self {
            real: paths.iter().map(|path| p(path)).collect(),
            links: Vec::new(),
            cwd: PathBuf::new(),
        }
    }

    /// The same disk, plus a symlink at `at` landing on `target`.
    fn linking(mut self, at: &str, target: &str) -> Self {
        self.links.push((p(at), p(target)));
        self
    }

    /// The same disk, seen from `dir` — the current directory a relative argument is resolved
    /// against, which a process always has and only a case with a relative path in it needs.
    fn at(mut self, dir: &str) -> Self {
        self.cwd = p(dir);
        self
    }
}

impl Files for Disk {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        self.canonical(path).map_err(Untrusted::Unreadable)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        // The empty path names nothing, on every platform `std::fs::canonicalize` runs on. Saying so
        // here is what makes a case over a bare relative name mean anything: `Path::parent` hands the
        // resolver this, and a fake that quietly answered the current directory for it would pass
        // whether or not `capability.rs` spells it `.` itself.
        if path.as_os_str().is_empty() {
            return Err("the empty path names nothing".to_string());
        }
        let asked = lexical(&self.cwd.join(path));
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

/// § 4's argument side reaches the current directory for a **bare** relative name, so a grant of `.`
/// covers the ordinary spelling of a path a program writes.
///
/// `Path::parent` of `copy.txt` is `""`, which canonicalizes nowhere, and a resolver that ran out of
/// components there denied every bare name under every grant — `Core\IO::read("missing.txt")` and
/// `Core\IO::write("copy.txt")` alike, both of them against a tree that granted the directory they
/// are in. The two halves are asserted together because a grant that resolves a relative name has to
/// place it, not merely accept it: the last row climbs out of the grant and is still refused.
#[test]
fn a_bare_relative_path_resolves_against_its_grant() {
    let disk = Disk::of(&[
        "/srv",
        "/srv/app",
        "/srv/app/data",
        "/srv/app/data/note.txt",
    ])
    .at("/srv/app");
    let granted = granting("[fs]\nread = [\".\"]\nwrite = [\".\"]\n", &disk);

    for (why, cap, path) in [
        (
            "a file that does not exist yet, which is every write",
            Cap::FsWrite,
            raw("copy.txt"),
        ),
        (
            "a file that will never exist, which is a read that is about to fail",
            Cap::FsRead,
            raw("missing.txt"),
        ),
        (
            "a bare name under a directory that does exist",
            Cap::FsRead,
            raw("data/note.txt"),
        ),
        (
            "a bare name under a directory that does not",
            Cap::FsWrite,
            raw("out/report.txt"),
        ),
    ] {
        assert!(
            granted.allows(cap, Scope::Path(path.as_path()), &disk),
            "a grant of `.` refused {} — {why}",
            path.display(),
        );
    }

    // Resolving a relative name against the current directory is not resolving every relative name
    // *into* the grant: one that climbs above it lands outside and is refused as it was before.
    for outside in [raw("../escape.txt"), raw("../app-next-door/x.txt")] {
        assert!(
            !granted.allows(Cap::FsWrite, Scope::Path(outside.as_path()), &disk),
            "a grant of `.` reached {}",
            outside.display(),
        );
    }
}

/// An address literal, for the exception test below.
fn ip(text: &str) -> std::net::IpAddr {
    text.parse()
        .unwrap_or_else(|_| panic!("{text} is not an address"))
}

/// ADR 0058 § 3's operator exception excepts the addresses it *names* and widens nothing else — not
/// a range around one, not the table, and not a name that resolves to one.
///
/// The two refusals a reader would expect to be grants are the point of the test rather than
/// incidental to it: `true` is the one `Setting` spelling that does not mean everything here, and a
/// hostname entry matches nothing at all, because this list is read before resolution and a name
/// checked here would exempt whatever it resolved to afterwards.
#[test]
fn an_operator_exception_names_one_address_and_widens_nothing_else() {
    let disk = Disk::of(&["/srv"]);
    let excepting = |text: &str| granting(text, &disk);

    // Deny by default: the table answers before any exception is written, and it answers the same
    // way for a tree with no `[net]` block at all.
    for (why, text) in [
        ("no `[net]` block", ""),
        ("a `net` block granting nothing", "[net]\n"),
        (
            "a `connect` grant for the same host",
            "[net]\nconnect = [\"127.0.0.1\"]\n",
        ),
    ] {
        assert_eq!(
            excepting(text).address_refused(ip("127.0.0.1")),
            Some("loopback (127.0.0.0/8)"),
            "{why} reached the loopback",
        );
    }

    // The exception itself, in both spellings of the same machine: an IPv4-mapped address is
    // matched as the address it maps to, from either side of the comparison.
    let one = excepting("[net]\ninternal = [\"127.0.0.1\"]\n");
    let mapped = excepting("[net]\ninternal = [\"::ffff:10.0.0.7\"]\n");
    assert_eq!(one.address_refused(ip("127.0.0.1")), None);
    assert_eq!(one.address_refused(ip("::ffff:127.0.0.1")), None);
    assert_eq!(mapped.address_refused(ip("10.0.0.7")), None);

    // And what it did not name stays denied — a neighbour in the same range, the *other* loopback,
    // and the link-local endpoint § 3 calls out.
    for outside in ["127.0.0.2", "::1", "169.254.169.254", "10.0.0.7"] {
        assert!(
            one.address_refused(ip(outside)).is_some(),
            "`internal = [\"127.0.0.1\"]` excepted {outside}",
        );
    }

    // Two entries that grant nothing, each of them a widening this shape refuses to have.
    for (why, text) in [
        ("`internal = true`", "[net]\ninternal = true\n"),
        ("a hostname entry", "[net]\ninternal = [\"localhost\"]\n"),
        ("a range", "[net]\ninternal = [\"127.0.0.0/8\"]\n"),
    ] {
        assert_eq!(
            excepting(text).address_refused(ip("127.0.0.1")),
            Some("loopback (127.0.0.0/8)"),
            "{why} excepted the loopback",
        );
    }

    // An address the table never denied is not something the exception has an opinion about.
    for caps in [excepting(""), one] {
        assert_eq!(caps.address_refused(ip("93.184.216.34")), None);
    }
}
