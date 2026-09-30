//! `rule:security/capability-question-is-grant-and-scope`'s question, asked of the capability `m6.md`'s *Verify* names on its own: a
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

/// An endpoint written the way an operator writes one into `nvs.toml`, as the address a program
/// binds: [`Scope::Endpoint`] carries a resolved endpoint and never the string it was spelled with.
fn ep(endpoint: &str) -> std::net::SocketAddr {
    endpoint.parse().expect("the case wrote an endpoint")
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
    /// every relative path spelled exactly as it was written.
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

/// `spawn script` without `script.spawn` fails, in every spelling of "without" `rule:security/capability-question-is-grant-and-scope` denies
/// by default, and in the ways a granted tree still says no to *this* target.
///
/// The granted case is asserted first so that every `false` below is the absence of the grant and
/// not the harness answering `false` to everything.
// covers: tools:config/capabilities
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
    // an omission at all: reading a file is not permission to run it (`rule:security/script-spawn-capability`), so a tree
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
/// `Path::parent` of `copy.txt` is `""`, which canonicalizes nowhere, and a resolver that ran out
/// of components there would deny every bare name under every grant —
/// `Core\IO::read("missing.txt")` and `Core\IO::write("copy.txt")` alike, both of them against a
/// tree that granted the directory they are in. The two halves are asserted together because a
/// grant that resolves a relative name has to place it, not merely accept it: the last row climbs
/// out of the grant and is still refused.
// covers: tools:config/capabilities
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
    // *into* the grant: one that climbs above it lands outside and is refused.
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

/// `rule:security/net-address-policy`'s table reads an IPv6 address that carries an IPv4 address —
/// mapped, IPv4-compatible or behind the NAT64 well-known prefix — as the address it carries, and an
/// exception written for the IPv4 address does not reach the two wider spellings.
#[test]
fn an_ipv6_address_carrying_an_internal_ipv4_address_is_denied() {
    for (carried, range) in [
        ("::ffff:127.0.0.1", "loopback (127.0.0.0/8)"),
        ("::127.0.0.1", "loopback (127.0.0.0/8)"),
        ("::a9fe:a9fe", "link-local (169.254.0.0/16)"),
        ("64:ff9b::7f00:1", "loopback (127.0.0.0/8)"),
        ("64:ff9b::10.0.0.7", "private (10/8, 172.16/12, 192.168/16)"),
        ("64:ff9b::169.254.169.254", "link-local (169.254.0.0/16)"),
        ("::0.0.0.2", "unspecified (0.0.0.0/8)"),
    ] {
        assert_eq!(
            nvs_config::capability::denied_by_default(ip(carried)),
            Some(range),
            "{carried} is judged by the address it carries",
        );
    }

    // The two addresses of their own inside `::/96` keep their own answers, and a public IPv4
    // address carried either way stays public.
    assert_eq!(
        nvs_config::capability::denied_by_default(ip("::1")),
        Some("loopback (::1)")
    );
    assert_eq!(
        nvs_config::capability::denied_by_default(ip("::")),
        Some("unspecified (::)")
    );
    for public in [
        "64:ff9b::203.0.113.10",
        "::203.0.113.10",
        "2001:db8::7f00:1",
    ] {
        assert_eq!(
            nvs_config::capability::denied_by_default(ip(public)),
            None,
            "{public} carries no internal address",
        );
    }

    let disk = Disk::of(&["/srv"]);
    let one = granting("[net]\ninternal = [\"127.0.0.1\"]\n", &disk);
    for wider in ["::127.0.0.1", "64:ff9b::7f00:1"] {
        assert!(
            one.address_refused(ip(wider)).is_some(),
            "`internal = [\"127.0.0.1\"]` excepted {wider}",
        );
    }
}

/// `rule:security/net-address-policy`'s operator exception excepts the addresses it *names* and widens nothing else — not
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

    // Entries that grant nothing, each of them a widening this shape refuses to have.
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
/// (`rule:core-classes/db-literal-query-checking` and § 3), and a check that disagreed with the run it
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

/// `rule:core-classes/db-capabilities`'s own worked grant, `db.open = ["*.tenants.internal"]`, matches a host under that
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
/// nothing at all, which is the deny-by-default direction and what `rule:core-api/shape-rules` R17 asks for.
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
/// `rule:http-server/allow-url-pins-the-address`'s door is why the two capabilities differ: `net.connect` names what a program may
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

/// `rule:core-classes/db-capabilities`'s `db.schema` grant: it names blocks, matches
/// one exactly, and is **not** implied by the `db.connect` that reached the same database.
///
/// The implication is the half worth pinning. `db.schema` gates a different question from either of
/// its neighbours — not which database may be reached but whether its shape may be changed
/// (`rule:core-classes/schema-apply-capability`) — so a
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

/// `net.listen` and `net.local` are rows on the roster, and each is denied until an operator has
/// written one (`rule:security/net-listen-is-a-separate-grant-from-net-connect`).
///
/// Two grants and two shapes. `net.local` is path-scoped like every other path grant, so § 4's
/// canonicalise-then-prefix governs it and a `..` out of a granted directory is refused rather than
/// resolved. `net.listen` is asked of an **endpoint**, and both sides of the comparison are parsed:
/// what a program binds is an address, not the string an operator typed, so one address's several
/// spellings are one endpoint and an entry that is not an endpoint at all matches nothing.
#[test]
fn net_listen_and_net_local_are_on_the_roster_and_denied_by_default() {
    let disk = Disk::of(&["/run", "/run/redis.sock", "/var/run", "/var/run/other.sock"]);
    let sock = p("/run/redis.sock");
    let bound = ep("127.0.0.1:8080");

    // Deny by default, one spelling of "nothing granted" per row, asked of both grants.
    for (why, text) in [
        ("no `[capabilities]` block at all", ""),
        ("a `net` block granting nothing", "[net]\n"),
        (
            "a grant of `false`",
            "[net]\nlocal = false\nlisten = false\n",
        ),
        ("an empty list", "[net]\nlocal = []\nlisten = []\n"),
    ] {
        let caps = granting(text, &disk);
        assert!(
            !caps.allows(Cap::NetLocal, Scope::Path(sock.as_path()), &disk),
            "{why} granted `net.local` for {}",
            sock.display(),
        );
        assert!(
            !caps.allows(Cap::NetListen, Scope::Endpoint(bound), &disk),
            "{why} granted `net.listen` for {bound}",
        );
        for cap in [Cap::NetListen, Cap::NetLocal] {
            assert!(
                !caps.allows(cap, Scope::Unscoped, &disk),
                "{why} granted `{}` unscoped",
                cap.name(),
            );
        }
    }

    // A path is granted at a directory prefix, but not outside it — including through the `..` a
    // program supplies rather than an operator.
    let rooted = granting("[net]\nlocal = [\"/run\"]\n", &disk);
    assert!(rooted.allows(Cap::NetLocal, Scope::Path(sock.as_path()), &disk));
    for (why, outside) in [
        ("a sibling directory", p("/var/run/other.sock")),
        (
            "a `..` back out of the root",
            raw("/run/../var/run/other.sock"),
        ),
    ] {
        assert!(
            !rooted.allows(Cap::NetLocal, Scope::Path(outside.as_path()), &disk),
            "`net.local = [\"/run\"]` reached {} through {why}",
            outside.display(),
        );
    }

    // A socket is granted before anything has bound it, so its grant names a file that does not
    // exist yet, and the directory it sits in is often reached through a symlink. The grant is
    // resolved by the argument's own walk, so it matches the argument that names the same file.
    let linked = Disk::of(&["/private/var/run"]).linking("/var/run", "/private/var/run");
    let unbound = granting("[net]\nlocal = [\"/var/run/app.sock\"]\n", &linked);
    let (app, other) = (p("/var/run/app.sock"), p("/var/run/other.sock"));
    assert!(
        unbound.allows(Cap::NetLocal, Scope::Path(app.as_path()), &linked),
        "a grant of a socket not bound yet, under a symlinked directory, refused that socket",
    );
    assert!(
        !unbound.allows(Cap::NetLocal, Scope::Path(other.as_path()), &linked),
        "a grant of one socket reached its neighbour",
    );

    // An endpoint is granted where it was written and nowhere else. The three granted spellings
    // below are two endpoints, which is the whole reason the comparison is of addresses: a grant
    // that matched the operator's spelling alone would be defeated by the program writing another.
    let listening = granting(
        "[net]\nlisten = [\"127.0.0.1:8080\", \"[::1]:9000\", \"10.4.0.9\"]\n",
        &disk,
    );
    for granted in ["127.0.0.1:8080", "[::ffff:127.0.0.1]:8080", "[::1]:9000"] {
        assert!(
            listening.allows(Cap::NetListen, Scope::Endpoint(ep(granted)), &disk),
            "`net.listen` refused {granted}, which is an endpoint it names",
        );
    }
    for (why, refused) in [
        ("another port at a granted address", "127.0.0.1:9000"),
        ("another address at a granted port", "192.168.1.4:8080"),
        (
            "the unspecified address, which is the exposed endpoint and not the loopback one",
            "0.0.0.0:8080",
        ),
        ("a bare address, which is not an endpoint", "10.4.0.9:8080"),
        ("a bare address, asked at port zero", "10.4.0.9:0"),
    ] {
        assert!(
            !listening.allows(Cap::NetListen, Scope::Endpoint(ep(refused)), &disk),
            "`net.listen` reached {refused} through {why}",
        );
    }

    // `true` is every endpoint this process may bind, which is the one spelling that says so.
    let anywhere = granting("[net]\nlisten = true\n", &disk);
    assert!(anywhere.allows(Cap::NetListen, Scope::Endpoint(ep("0.0.0.0:443")), &disk));

    // Both are on the roster, spelled the way a refusal prints them, and neither takes a host
    // wildcard: neither grant has a host in it to pattern-match. Only the path one is
    // path-scoped, which is what decides whether canonicalization has work to do.
    assert!(Cap::NetLocal.is_path_scoped());
    assert!(!Cap::NetListen.is_path_scoped());
    for (cap, name) in [(Cap::NetListen, "net.listen"), (Cap::NetLocal, "net.local")] {
        assert!(!cap.takes_host_wildcard(), "`{name}` took a host wildcard");
        assert_eq!(cap.name(), name);
        assert_eq!(Cap::parse(name), Some(cap));
        assert!(Cap::ALL.contains(&cap), "`{name}` is not on the roster");
    }
}

/// `net.connect` is still the grant over a **host**, and neither of the grants beside it widens it
/// (`rule:security/net-listen-is-a-separate-grant-from-net-connect`).
///
/// Asked of all three at each one's widest spelling, because the failure this guards against is
/// that a grant is read as another's shorthand, and it is asymmetric in every direction: a program
/// that may reach the network reaching a socket path is the way onto the local machine
/// `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it` spends a whole rule
/// refusing; a program that may open one socket reaching the internet is the widening
/// `rule:config/net-local-is-named-and-not-on-the-roster` declined to write; and a program that may
/// bind its own port reaching a host would hand the address policy away at the bind, which is what
/// would make a datagram socket the way around it.
#[test]
fn net_connect_still_carries_its_host_scope_and_neither_new_grant_widens_it() {
    let disk = Disk::of(&["/run", "/run/redis.sock"]);
    let sock = p("/run/redis.sock");
    let bound = ep("127.0.0.1:8080");

    // The host scope, unchanged: matched against the list and case-insensitively, because DNS is.
    let listed = granting("[net]\nconnect = [\"Reports.Internal\"]\n", &disk);
    assert!(listed.allows(Cap::NetConnect, Scope::Host("reports.internal"), &disk));
    assert!(!listed.allows(Cap::NetConnect, Scope::Host("other.internal"), &disk));

    // And none of the three follows from either of the others, each granted as wide as it goes.
    for (granted, text) in [
        ("net.connect", "[net]\nconnect = true\n"),
        ("net.listen", "[net]\nlisten = true\n"),
        ("net.local", "[net]\nlocal = true\n"),
    ] {
        let caps = granting(text, &disk);
        for (asked, held) in [
            (
                "net.connect",
                caps.allows(Cap::NetConnect, Scope::Host("reports.internal"), &disk),
            ),
            (
                "net.listen",
                caps.allows(Cap::NetListen, Scope::Endpoint(bound), &disk),
            ),
            (
                "net.local",
                caps.allows(Cap::NetLocal, Scope::Path(sock.as_path()), &disk),
            ),
        ] {
            assert_eq!(
                held,
                asked == granted,
                "`{granted} = true` answered {held} for `{asked}`",
            );
        }
    }

    // A datagram socket is why the split has to hold in both directions at once: it asks
    // `net.listen` for its own port and `net.connect` for every destination it sends to, so the
    // bind grant reaching a host would be the whole address policy handed away at the bind.
    let both = granting(
        "[net]\nlisten = [\"0.0.0.0:5353\"]\nconnect = [\"resolver.internal\"]\n",
        &disk,
    );
    assert!(both.allows(Cap::NetListen, Scope::Endpoint(ep("0.0.0.0:5353")), &disk));
    assert!(both.allows(Cap::NetConnect, Scope::Host("resolver.internal"), &disk));
    assert!(!both.allows(Cap::NetConnect, Scope::Host("elsewhere.internal"), &disk));
    assert!(!both.allows(Cap::NetListen, Scope::Endpoint(ep("0.0.0.0:53")), &disk));
}

/// Whether `caps` grants `queue.purge` for the queue `name`, which is the one question
/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s two members ask.
fn purges(caps: &Capabilities, name: &str, disk: &Disk) -> bool {
    caps.allows(Cap::QueuePurge, Scope::Name(name), disk)
}

/// `queue.purge` reads the three spellings every other name-scoped grant reads, and reads them the
/// same way — `true` is every queue, a list is those queues, a bare string is the one-entry list,
/// and an empty list is nothing.
///
/// The point of asserting all four together is that a new grant is a new place for a spelling to be
/// read *specially*. `Setting` is the one reader and `grant_of` is the one interpretation, so a row
/// that reached its own field correctly cannot also decide what `true` means; a case granting only
/// lists would never find out.
#[test]
fn queue_purge_reads_true_a_list_a_bare_string_and_an_empty_list_as_every_other_name_does() {
    let disk = Disk::of(&["/srv"]);

    let every = granting("[queue]\npurge = true\n", &disk);
    assert!(purges(&every, "email", &disk));
    assert!(purges(&every, "reports", &disk));

    let listed = granting("[queue]\npurge = [\"email\", \"reports\"]\n", &disk);
    assert!(purges(&listed, "email", &disk));
    assert!(purges(&listed, "reports", &disk));
    assert!(!purges(&listed, "webhooks", &disk));

    // A bare string is the one-entry list and not a second shape: an operator with one queue
    // writes the queue, and what they get is the list they would have written.
    let one = granting("[queue]\npurge = \"email\"\n", &disk);
    assert!(purges(&one, "email", &disk));
    assert!(!purges(&one, "reports", &disk));

    // An empty list grants nothing, which is the spelling that reads as "I mean this deliberately"
    // and still has to deny — the same reading `net.listen = []` gets one grant over.
    let none = granting("[queue]\npurge = []\n", &disk);
    assert!(!purges(&none, "email", &disk));
}

/// A tree with no `[capabilities.queue]` block grants nothing, and that absence **is** the denial
/// rather than a gap something else fills in.
///
/// Deny-by-default is what makes the grant worth having: `delete` and `purge` destroy the record
/// that work existed, and a deployment that never wrote a grant is one that never asked for a
/// member to be able to. Every spelling of "nothing granted" is asked, because each is a different
/// arm — an absent block, an empty one, a `false`, and an empty list.
#[test]
fn an_absent_queue_block_grants_nothing_and_is_the_denial() {
    let disk = Disk::of(&["/srv"]);
    for (why, text) in [
        ("no `[capabilities]` block at all", ""),
        ("a `queue` block granting nothing", "[queue]\n"),
        ("a grant of `false`", "[queue]\npurge = false\n"),
        ("an empty list", "[queue]\npurge = []\n"),
        // A neighbouring grant is not this one: reaching the database the queue lives in says
        // nothing about being allowed to remove rows out of it.
        ("`db.connect` alone", "[db]\nconnect = [\"main\"]\n"),
    ] {
        let caps = granting(text, &disk);
        assert!(
            !purges(&caps, "email", &disk),
            "{why} granted `queue.purge`"
        );
        assert!(
            !caps.allows_unscoped(Cap::QueuePurge),
            "{why} granted `queue.purge` unscoped",
        );
    }
}

/// A queue name is matched exactly: another queue, a second casing of this one, and a `*` entry are
/// all outside a grant that named one queue.
///
/// The `*` half is the one worth pinning. `db.open` is the single capability whose entries may be
/// written as patterns, because its targets are *hosts* a program supplies; a queue name is
/// compared against a name the program wrote at the enqueue, so a `*` here is a queue called `*`
/// and grants nothing else — which is what [`Cap::takes_host_wildcard`] answering `false` means and
/// is asserted beside it.
#[test]
fn a_queue_name_is_matched_exactly_and_a_star_entry_grants_nothing() {
    let disk = Disk::of(&["/srv"]);
    let one = granting("[queue]\npurge = [\"email\"]\n", &disk);
    assert!(purges(&one, "email", &disk));
    for name in ["reports", "Email", "email ", "*", "*.email"] {
        assert!(
            !purges(&one, name, &disk),
            "`purge = [\"email\"]` covered {name}"
        );
    }

    // An entry written as a pattern is a name like any other, and covers only a queue spelled that
    // way — never the queues it looks like it was meant to reach.
    let starred = granting("[queue]\npurge = [\"*\"]\n", &disk);
    assert!(!purges(&starred, "email", &disk));
    assert!(purges(&starred, "*", &disk));

    assert!(!Cap::QueuePurge.is_path_scoped());
    assert!(!Cap::QueuePurge.takes_host_wildcard());
}

/// `queue.purge` is on the roster the way every other capability is: in [`Cap::ALL`], round-tripping
/// through the name a refusal prints, under the `[capabilities.queue]` table that name implies, and
/// reading its own field out of the tree.
///
/// `grant_mut` is the mirror the type keeps by hand, and what a test can reach of it is that
/// `canonicalize` — its only caller — leaves this grant exactly as the operator wrote it. A queue
/// name is not a path, so the canonicalizer has no work to do here, and a grant it had rewritten
/// into a host-native path would stop matching the name a receipt carries. The arms agreeing at all
/// is the compiler's, since both matches are exhaustive over this enum.
#[test]
fn queue_purge_appears_in_cap_all_and_in_both_grant_and_grant_mut() {
    let disk = Disk::of(&["/srv"]);
    assert!(Cap::ALL.contains(&Cap::QueuePurge));
    assert_eq!(Cap::QueuePurge.name(), "queue.purge");
    assert_eq!(Cap::parse("queue.purge"), Some(Cap::QueuePurge));
    assert_eq!(Cap::QueuePurge.family(), "queue");

    // Its own field and no other's: a tree granting a neighbour and not this one has to answer
    // `false`, which is what catches an arm that reached the wrong block.
    let elsewhere = granting("[cache]\nshared = true\n[db]\nconnect = true\n", &disk);
    assert!(!purges(&elsewhere, "email", &disk));

    let mut written: Capabilities =
        toml::from_str("[queue]\npurge = [\"email\"]\n").expect("the case wrote a grant");
    let before = written.clone();
    written.canonicalize(&disk);
    assert_eq!(written, before, "canonicalizing rewrote a name grant");
    assert!(purges(&written, "email", &disk));
}

/// `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s last sentence about spelling: the
/// six grants that name hosts a weakening applies to read a `true` as nothing at all.
///
/// The direction is what matters. `true` is "every host" for every other capability, so the failure
/// this guards against is a deployment writing the spelling it knows and relaxing verification
/// everywhere while believing it wrote a list. Reading it as `Grant::Nothing` fails closed: the
/// grant is refused until it names the hosts it meant, which is `CapNet::internal`'s answer to the
/// same question and the whole point of these grants being lists — what was relaxed stays
/// legible host by host in review.
///
/// A list is asserted first so that every `false` below is the `true` and not the harness answering
/// `false` to everything, and `false` is asserted after so that the refusal is not the value simply
/// being unreadable.
#[test]
fn a_tls_grant_of_true_is_refused() {
    let disk = Disk::of(&["/srv"]);
    let relaxing = [
        (Cap::TlsAnchors, "tls", "anchors", "tls.anchors"),
        (Cap::TlsPin, "tls", "pin", "tls.pin"),
        (Cap::TlsAnyName, "tls", "any_name", "tls.any_name"),
        (Cap::TlsInsecure, "tls", "insecure", "tls.insecure"),
        (Cap::NetConnectTo, "net", "connect_to", "net.connect_to"),
        (Cap::NetDowngrade, "net", "downgrade", "net.downgrade"),
    ];

    for (cap, table, key, name) in relaxing {
        let listed = granting(
            &format!("[{table}]\n{key} = [\"Partner.Example.Com\"]\n"),
            &disk,
        );
        assert!(
            listed.allows_host(cap, "partner.example.com"),
            "`{name}` did not grant the host it lists",
        );
        assert!(
            !listed.allows_host(cap, "other.example.com"),
            "`{name}` granted a host it does not list",
        );

        // The spelling the rule refuses, beside the one that has always meant nothing. Neither
        // names a host; `false` is asked as well so that the refusal above reads as the roster's
        // answer to a `true` rather than as a block that failed to deserialize at all.
        for wrote in ["true", "false"] {
            let boolean = granting(&format!("[{table}]\n{key} = {wrote}\n"), &disk);
            assert!(
                !boolean.allows_host(cap, "partner.example.com"),
                "`{name} = {wrote}` relaxed a host",
            );
            assert!(
                !boolean.allows_unscoped(cap),
                "`{name} = {wrote}` answered an unscoped question",
            );
        }

        // On the roster the way every capability is, under the table its own name implies, and with
        // no pattern of any kind: these are hosts an operator wrote out one by one.
        assert!(Cap::ALL.contains(&cap), "`{name}` is not on the roster");
        assert_eq!(cap.name(), name);
        assert_eq!(Cap::parse(name), Some(cap));
        assert_eq!(cap.family(), table);
        assert!(
            !cap.takes_true_spelling(),
            "`{name}` kept a `true` spelling"
        );
        assert!(!cap.takes_host_wildcard(), "`{name}` took a host wildcard");
        assert!(!cap.is_path_scoped(), "`{name}` was read as a path grant");
    }

    // `true` still means every host where a capability has that spelling, so what refuses above is
    // the roster's answer for these six and not a reading of `true` that changed underneath the
    // rest of them.
    let anywhere = granting("[net]\nconnect = true\n", &disk);
    assert!(anywhere.allows_host(Cap::NetConnect, "partner.example.com"));
    assert!(Cap::NetConnect.takes_true_spelling());
}
