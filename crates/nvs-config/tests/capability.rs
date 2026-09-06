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

/// Whether `caps` grants `cap` for `host`, asked in **both** spellings and asserted to agree.
///
/// `allows_host` is what a `nvs check` pass asks and `allows` is what a running request asks
/// ([ADR 0067](/docs/adr/0067-core-db.md) § 10 and § 3), and a check that disagreed with the run it
/// precedes is the one failure `rule:expressions/preparation-preserves-behaviour` forbids outright. Every wildcard case below goes through
/// here rather than through either half, so a wildcard read by one caller and not the other fails.
fn grants_host(caps: &Capabilities, cap: Cap, host: &str, disk: &Disk) -> bool {
    let checked = caps.allows_host(cap, host);
    assert_eq!(
        checked,
        caps.allows(cap, Scope::Host(host), disk),
        "`{}` answered a check and a run differently for {host}",
        cap.name(),
    );
    checked
}

/// ADR 0067 § 3's own worked grant, `db.open = ["*.tenants.internal"]`, matches a host under that
/// zone — at one label of depth and at several, and in any casing, because DNS preserves none.
#[test]
fn a_wildcard_grant_matches_a_subdomain_at_a_label_boundary() {
    let disk = Disk::of(&["/srv"]);
    let caps = granting("[db]\nopen = [\"*.tenants.internal\"]\n", &disk);

    for host in [
        "a.tenants.internal",
        "a.b.tenants.internal",
        "A.Tenants.INTERNAL",
        "x-1.deep.nest.tenants.internal",
    ] {
        assert!(
            grants_host(&caps, Cap::DbOpen, host, &disk),
            "`*.tenants.internal` did not cover {host}",
        );
    }

    // The wildcard widens the entry it is written on and nothing else: a sibling zone stays denied,
    // and so does a host that merely contains the suffix somewhere other than at its end.
    for outside in [
        "a.tenants.example",
        "tenants.internal.evil.test",
        "a.tenants.internal.evil.test",
    ] {
        assert!(
            !grants_host(&caps, Cap::DbOpen, outside, &disk),
            "`*.tenants.internal` covered {outside}",
        );
    }
}

/// `*.tenants.internal` is a grant for what is *under* the zone, so the zone's own name is not in
/// it — nor is the empty label a bare dot in front of it would make.
///
/// Asserted beside the grant that does cover it, because "the bare domain is denied" is only
/// meaningful next to the entry that would have granted it: an operator who wants both writes both.
#[test]
fn a_wildcard_grant_does_not_match_the_bare_domain() {
    let disk = Disk::of(&["/srv"]);
    let wild = granting("[db]\nopen = [\"*.tenants.internal\"]\n", &disk);
    let both = granting(
        "[db]\nopen = [\"*.tenants.internal\", \"tenants.internal\"]\n",
        &disk,
    );

    for bare in ["tenants.internal", ".tenants.internal", "internal"] {
        assert!(
            !grants_host(&wild, Cap::DbOpen, bare, &disk),
            "`*.tenants.internal` covered {bare}",
        );
    }
    assert!(grants_host(&both, Cap::DbOpen, "tenants.internal", &disk));
    assert!(grants_host(&both, Cap::DbOpen, "a.tenants.internal", &disk));
}

/// The reason this is a label-boundary rule and not `ends_with`: `evil-tenants.internal` is a name
/// anybody can register, and a suffix check would hand it every credential the zone's grant covers.
#[test]
fn a_wildcard_grant_does_not_match_a_suffix_inside_a_label() {
    let disk = Disk::of(&["/srv"]);
    let caps = granting("[db]\nopen = [\"*.tenants.internal\"]\n", &disk);

    for inside in [
        "evil-tenants.internal",
        "eviltenants.internal",
        "xtenants.internal",
        "a.eviltenants.internal",
    ] {
        assert!(
            !grants_host(&caps, Cap::DbOpen, inside, &disk),
            "`*.tenants.internal` covered {inside}, which is a suffix match inside a label",
        );
    }
}

/// `open = true` is already "every host", so `"*"` is not a second way to write it — it grants
/// nothing at all, which is the deny-by-default direction and what ADR 0063 R20 asks for.
///
/// The `true` case is asserted first so that the refusals below are the entry being inert and not
/// the fixture answering `false` to everything.
#[test]
fn a_bare_star_is_refused_as_a_second_spelling_of_every_host() {
    let disk = Disk::of(&["/srv"]);
    let everything = granting("[db]\nopen = true\n", &disk);
    assert!(grants_host(
        &everything,
        Cap::DbOpen,
        "anything.test",
        &disk
    ));

    // Neither spelling of a pattern with no zone behind it grants a host, including one named the
    // way the entry is: a pattern-looking entry is never read as an exact host.
    for entry in ["*", "*.", "**"] {
        let caps = granting(&format!("[db]\nopen = [\"{entry}\"]\n"), &disk);
        for host in ["a.tenants.internal", "tenants.internal", entry] {
            assert!(
                !grants_host(&caps, Cap::DbOpen, host, &disk),
                "`open = [\"{entry}\"]` covered {host}",
            );
        }
    }
}

/// The wildcard is `db.open`'s alone. A `*.` entry under `net.connect` grants no host, and the
/// exact entry beside it still does.
///
/// ADR 0058 § 2's door is why the two capabilities differ: `net.connect` names what a program may
/// *reach*, and the address it is pinned to is resolved from that name afterwards, so widening the
/// set of names by a pattern widens it by every name an attacker can get into the zone's DNS
/// without the operator having written any one of them down. `db.open`'s zone is one the operator
/// named and runs.
#[test]
fn net_connect_takes_no_wildcard_because_it_is_asked_of_an_address() {
    let disk = Disk::of(&["/srv"]);
    let caps = granting(
        "[net]\nconnect = [\"*.tenants.internal\", \"api.tenants.internal\"]\n",
        &disk,
    );

    assert!(grants_host(
        &caps,
        Cap::NetConnect,
        "api.tenants.internal",
        &disk
    ));
    for host in ["a.tenants.internal", "b.tenants.internal", "*"] {
        assert!(
            !grants_host(&caps, Cap::NetConnect, host, &disk),
            "`net.connect = [\"*.tenants.internal\"]` covered {host}",
        );
    }

    // And the same list under `db.open` does read the pattern — the difference is the capability
    // and nothing about the entry.
    let open = granting(
        "[db]\nopen = [\"*.tenants.internal\", \"api.tenants.internal\"]\n",
        &disk,
    );
    assert!(grants_host(&open, Cap::DbOpen, "a.tenants.internal", &disk));
}

/// Whether `caps` grants `cap` for the configuration block `name` — [`Scope::Name`], which is the
/// question `db.connect` and `db.schema` are both asked.
fn grants_name(caps: &Capabilities, cap: Cap, name: &str, disk: &Disk) -> bool {
    caps.allows(cap, Scope::Name(name), disk)
}

/// [ADR 0067](/docs/adr/0067-core-db.md) § 3's third `db.*` grant: `db.schema` names blocks, matches
/// one exactly, and is **not** implied by the `db.connect` that reached the same database.
///
/// The implication is the half worth pinning. `db.schema` gates a different question from either of
/// its neighbours — not which database may be reached but whether its shape may be changed
/// ([ADR 0145](/docs/adr/0145-a-schema-is-a-value-core-db-schema-converges-a-closed.md) § 9) — so a
/// deployment that granted `connect` alone has granted no DDL, and a roster arm reading the wrong
/// field would be invisible to every test that only ever grants both. The grant is asserted first so
/// that the refusals below are the absence of a grant rather than the fixture answering `false` to
/// everything.
#[test]
fn db_schema_names_blocks_and_does_not_follow_from_db_connect() {
    let disk = Disk::of(&["/srv"]);
    let both = granting("[db]\nconnect = [\"main\"]\nschema = [\"main\"]\n", &disk);
    assert!(grants_name(&both, Cap::DbSchema, "main", &disk));

    // Reaching a database is not permission to alter it: the same tree without the `schema` line
    // opens the connection and refuses the DDL.
    let read_only = granting("[db]\nconnect = [\"main\"]\n", &disk);
    assert!(grants_name(&read_only, Cap::DbConnect, "main", &disk));
    assert!(!grants_name(&read_only, Cap::DbSchema, "main", &disk));

    // A name is not a hostname: it is matched exactly, so neither another block nor a second casing
    // of this one is covered — and `db.open`'s wildcard is not a spelling here, since the entry is
    // compared against a name the operator wrote rather than against a resolved host.
    let one = granting(
        "[db]\nconnect = [\"main\", \"replica\"]\nschema = [\"main\"]\n",
        &disk,
    );
    for name in ["replica", "Main", "*", "*.main"] {
        assert!(
            !grants_name(&one, Cap::DbSchema, name, &disk),
            "`db.schema = [\"main\"]` covered {name}",
        );
    }
    assert!(!Cap::DbSchema.is_path_scoped());
    assert!(!Cap::DbSchema.takes_host_wildcard());

    // And the name a refusal prints is the name a grant line is written under, both ways round, so
    // an operator pasting the one back into `nvs.toml` gets the capability that refused them.
    assert_eq!(Cap::DbSchema.name(), "db.schema");
    assert_eq!(Cap::parse("db.schema"), Some(Cap::DbSchema));
}
