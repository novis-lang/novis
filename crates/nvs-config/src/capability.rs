//! `rule:security/capability-question-is-grant-and-scope`'s capability question: a grant, a scope, and the `bool` the two of them answer.
//!
//! This module is the whole decision procedure and it is **pure** — a [`Capabilities`], a [`Cap`], an
//! argument, a `bool`. It takes no context, throws nothing, and reports nothing, so it is testable
//! without a compiler or a request in front of it. The refusal a program sees is
//! `nvs_runtime::capability::require`, which asks this and turns a `false` into § 5's `RuntimeError`;
//! keeping the two apart is what lets `-p nvs-config` assert the *rule* and `-p nvs-stdlib` assert the
//! *diagnostic*.
//!
//! **Deny by default, at every step.** A capability whose block is absent is denied, one whose grant is
//! `false` is denied, one granted an empty list is denied, and a scoped argument that cannot be resolved
//! to a canonical path is denied. There is no path through [`Capabilities::allows`] that returns `true`
//! without an operator having written something that says so.
//!
//! **A default is [`Authority`]'s alone.** [`Capabilities`] answers for the keys a tree wrote, and
//! [`Authority`] adds [`crate::defaults::capability`] for a key it did not, given the [`Run`]. Every
//! door asks through [`Authority`].
//!
//! [`Cap`] is also the one place a capability's configuration *name* maps to the field of
//! [`Capabilities`] that grants it. A new capability is a variant, a `name` arm and a `grant` arm —
//! never a string compared in a second module.
//!
//! # A `db.open` entry may be a `*.` wildcard
//!
//! `rule:core-classes/db-capabilities` writes `db.open = ["*.tenants.internal"]`, and this is
//! what that entry means. An entry beginning `*.` matches a host whose name ends with the entry's
//! remainder **at a label boundary**: `*.tenants.internal` grants `a.tenants.internal` and
//! `a.b.tenants.internal`, and grants neither `tenants.internal` itself nor `evil-tenants.internal`.
//! Matching stays case-insensitive, because the exact spelling of an entry is not something DNS
//! preserves either.
//!
//! These refusals hold the rule to that shape:
//!
//! - **A bare `*` is not a spelling.** `true` is already "every host" — `Grant::Everything` — and
//!   a grant reachable two ways is what `rule:core-api/shape-rules` R17 forbids. `*` alone therefore matches no host
//!   at all, including a host literally named `*`, and so does `*.` with nothing after it — and
//!   [`validate`] refuses both at load, so a tree that wrote one is told rather than denied.
//! - **The wildcard is `db.open`'s and `ldap.open`'s alone**, which is [`Cap::takes_host_wildcard`]. `net.connect`'s
//!   grant is asked of a *name* and then `rule:http-server/allow-url-pins-the-address`
//!   's door pins the address that name resolved to; a pattern there would widen the set of
//!   names an attacker-influenced argument may reach without the operator having written any one of
//!   them down, which is the whole thing that rule refuses. `db.open`'s targets are program-supplied
//!   too, but a tenant-per-subdomain deployment cannot enumerate them, and its blast radius is one
//!   operator-named zone rather than the internet.
//! - **A wildcard never crosses a label**, so a suffix match inside a label — the
//!   `evil-tenants.internal` case, which is the whole reason this is not `ends_with` — is refused.
//!   Registering that name is the cheapest attack there is against a suffix check.
//!

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Files, Origin};
use crate::tree::{Capabilities, Config, Setting};

/// One capability, by the name `nvs.toml` grants it under.
///
/// The roster is closed: a member needing something not in it has no `Cap` to pass, which is
/// `rule:security/capability-check-at-the-door`'s last
/// consequence — a capability the configuration cannot express fails visibly at the door rather than
/// quietly at a review.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cap {
    /// `fs.read` — the roots readable.
    FsRead,
    /// `fs.write` — the roots writable.
    FsWrite,
    /// `script.spawn` — the roots a `spawn script` target may live under
    /// (`rule:security/script-spawn-capability`). Being able to read a file is not permission to run
    /// it, which is why this is not implied by `fs.read`.
    ScriptSpawn,
    /// `net.connect` — the hosts an outbound connection may reach.
    NetConnect,
    /// `net.listen` — the endpoints a program may bind
    /// (`rule:security/net-listen-is-a-separate-grant-from-net-connect`).
    ///
    /// Asked at [`Scope::Endpoint`] and carrying no address policy, because the policy's terms
    /// **invert** under a bind: binding loopback is the contained case and binding the unspecified
    /// address is the exposed one, so [`NetConnect`](Self::NetConnect)'s table would deny the safe
    /// spelling and permit the dangerous one. What a bind risks — occupying a port another service
    /// expects, or exposing a surface to a network nobody intended — is answered by naming the
    /// endpoint instead, which is also what makes it legible in a review.
    ///
    /// It is a grant of its own for [`NetLocal`](Self::NetLocal)'s reason: reaching a host and
    /// binding one are different powers, so holding either says nothing about the other. One socket
    /// can need two of them, because it does two things — a datagram socket asks this once for its
    /// own port and `net.connect` for every destination it sends to, since granting the whole
    /// address policy away at the bind is what would make UDP the way around it.
    NetListen,
    /// `net.local` — the socket paths a program may connect to or bind
    /// (`rule:config/net-local-is-named-and-not-on-the-roster`).
    ///
    /// Asked at [`Scope::Path`] and carrying no address policy, because there is no address to ask
    /// one about. It is a grant of its own rather than [`NetConnect`](Self::NetConnect) widened to
    /// admit paths: that grant's whole character is the table it carries, and hosts governed by a
    /// table beside paths governed by nothing is one name covering two guarantees. Reaching the
    /// network and opening something on this machine are different powers, so holding either says
    /// nothing about the other — `rule:security/net-listen-is-a-separate-grant-from-net-connect`.
    ///
    /// It governs **both ends** of a path. Binding one is granted the same way as connecting to
    /// one, because a program that may create a socket at a path is a program whatever else on the
    /// host finds it may speak to.
    NetLocal,
    /// `net.connect_to` — the hosts a call may name its own address for with `connectTo`
    /// (`rule:http-server/an-outbound-call-names-its-address-only-under-a-grant`).
    ///
    /// Asked at [`Scope::Host`] against the URL's host and never against the address written, which
    /// is judged by [`denied_by_default`] and [`Capabilities::address_refused`] exactly as a
    /// resolved one is; the certificate is still checked against that host. So the option chooses
    /// among the addresses this deployment already reaches rather than widening the set, and it is
    /// a grant of its own beside [`NetConnect`](Self::NetConnect) for the reason those two grants
    /// are two: being allowed to reach a host is not being allowed to decide where that host is.
    ///
    /// It has no `true` spelling ([`takes_true_spelling`](Self::takes_true_spelling)).
    NetConnectTo,
    /// `net.downgrade` — the hosts a redirect from `https` to `http` may land on
    /// (`rule:http-server/an-https-redirect-never-becomes-plaintext`).
    ///
    /// Asked at [`Scope::Host`] for the hop's target, and owed alongside the call's own
    /// `redirectToHttp`. A plain `http` URL asked for directly does not reach this grant at all — a
    /// program fetching an internal `http` endpoint is not being attacked by itself. What is worth
    /// naming a host for is the other case: a server stripping a call's TLS with one `Location`
    /// header, while the caller sees a `200`.
    ///
    /// It has no `true` spelling ([`takes_true_spelling`](Self::takes_true_spelling)).
    NetDowngrade,
    /// `tls.anchors` — the hosts a call may trust PEM certificates of its own for with `tlsCa`,
    /// in place of `[http.client.tls] roots`
    /// (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`).
    ///
    /// The first of the four grants that relax certificate verification, which share one shape and
    /// so are documented once here. Each is asked at [`Scope::Host`], each answers a different
    /// question about *how much* is skipped, and each is owed **together with** the option beside
    /// it: the deployment says where this may happen and the call says here, so a grant changes
    /// nothing about a call that does not ask, and an option whose host the grant does not list
    /// throws before a connection is made. None of the four has a `true` spelling
    /// ([`takes_true_spelling`](Self::takes_true_spelling)), because the whole value of the grant is
    /// that a reviewer can read which hosts a deployment relaxed.
    TlsAnchors,
    /// `tls.pin` — the hosts a call may accept on a `sha256//` public-key pin alone with `tlsPin`,
    /// building no chain. [`TlsAnchors`](Self::TlsAnchors)'s shape.
    TlsPin,
    /// `tls.any_name` — the hosts a call may skip the name check for with `tlsVerifyHost: false`,
    /// the chain still built and checked. [`TlsAnchors`](Self::TlsAnchors)'s shape.
    TlsAnyName,
    /// `tls.insecure` — the hosts a call may check neither chain nor name for with
    /// `tlsVerify: false`. [`TlsAnchors`](Self::TlsAnchors)'s shape, and the widest of the four.
    TlsInsecure,
    /// `process.exec` — the programs a subprocess may be started from.
    ProcessExec,
    /// `debug.trace` — where a trace may be written (`rule:testing/debug-probes`): the paths
    /// `Core\Debug::startTrace` may name. Separate from `fs.write`, which grants none of them.
    DebugTrace,
    /// `debug.profile` — where a profile may be written (`rule:testing/debug-probes`): the paths
    /// `Core\Debug::startProfile` may name. Separate from `fs.write`, which grants none of them.
    DebugProfile,
    /// `db.connect` — which `[db.<name>]` blocks a program may open by name (`rule:core-classes/db-capabilities`).
    DbConnect,
    /// `db.open` — which hosts a program-supplied `Db\Settings` may reach.
    DbOpen,
    /// `db.schema` — which `[db.<name>]` blocks a program may issue DDL to (`rule:core-classes/schema-apply-capability`).
    ///
    /// Named by block, which is [`DbConnect`](Self::DbConnect)'s shape, but it gates a different
    /// question from the grants above: not *which* database may be reached, but whether this
    /// program may change the shape of one at all. Being able to open a connection is not permission
    /// to alter what is behind it, so this does not follow from `db.connect` — the same split
    /// [`ScriptSpawn`](Self::ScriptSpawn) makes against `fs.read`. Computing a plan reads the
    /// catalog and asks nothing beyond the `db.connect` the program already holds; only applying one
    /// arrives here.
    DbSchema,
    /// `ldap.connect` — which `[ldap.<name>]` blocks a program may open by name
    /// (`rule:core-classes/ldap-capabilities`).
    ///
    /// [`DbConnect`](Self::DbConnect)'s shape and its reasoning: every URL in a block was written by
    /// an operator, so a block named here is pre-approved against
    /// `rule:security/net-address-policy`'s denied ranges, where every directory controller lives.
    LdapConnect,
    /// `ldap.open` — which hosts a program-supplied `Ldap\Settings` may reach.
    /// [`DbOpen`](Self::DbOpen)'s shape: a `*.` entry matches at a label boundary, and the address
    /// the host resolves to is still asked the policy's question.
    LdapOpen,
    /// `ldap.cleartext` — the hosts a simple bind may cross plain `ldap://` to
    /// (`rule:security/ldap-cleartext-bind-is-granted-per-host`).
    ///
    /// Asked at [`Scope::Host`] and only for a block or settings value that also says
    /// `tls = "none"`, before the password is written. A host list with no `true`, the shape of
    /// [`NetDowngrade`](Self::NetDowngrade) and [`TlsInsecure`](Self::TlsInsecure): the password and
    /// every answer cross the network readable and changeable, so the grant names each host that
    /// pays that.
    LdapCleartext,
    /// `mail.send` — which `[mail.<name>]` blocks a program may send through (`rule:programs/framework-core-half`).
    ///
    /// Named by block and never by host, which is [`DbConnect`](Self::DbConnect)'s shape and
    /// `rule:core-classes/db-capabilities`'s reasoning: the endpoint an operator wrote into root-owned configuration carries
    /// the authority that granted this capability, so it is pre-approved and is not additionally
    /// asked about `rule:security/net-address-policy`'s denied ranges — where every ordinary relay lives.
    MailSend,
    /// `cache.shared` — whether this program may reach the coherent cache tier
    /// (`rule:config/cache-shared-is-the-grant-over-the-configured-store`).
    ///
    /// Asked at [`Scope::Unscoped`], which is the whole of what makes it a different grant from
    /// [`NetConnect`](Self::NetConnect): the store is the one `[cache.shared] url` names, and it is
    /// named by an operator in root-owned configuration, so [`MailSend`](Self::MailSend)'s reasoning
    /// above applies unchanged and is not restated here. Unscoped rather than named because a
    /// deployment has one shared store — a scope with one possible value would read as a choice
    /// nobody made — and so moving that store between a container, a loopback daemon and a socket
    /// changes one block and no program's grant list.
    CacheShared,
    /// `queue.purge` — which queues a program may remove rows from
    /// (`rule:concurrency/queue-deletion-is-explicit-and-bounded`).
    ///
    /// Asked at [`Scope::Name`] and named by queue, which is [`DbConnect`](Self::DbConnect)'s shape:
    /// the queue is a name the program itself wrote at the enqueue, matched exactly because it is a
    /// name and not a hostname. It is the only grant `Core\Queue` takes at all — `push`, `status`,
    /// `cancel` and `stats` read or release what the caller already holds a receipt for, and these
    /// two destroy the record that work existed.
    ///
    /// One grant over both members rather than one each. `delete` names a single job and `purge` a
    /// filter over many, but the authority they need is the same one — permission to make a queue's
    /// rows stop existing — and splitting it would let a deployment grant the unbounded half while
    /// withholding the bounded one. It is a grant of its own rather than [`DbSchema`](Self::DbSchema)
    /// widened, for that capability's own reason read the other way: removing rows is DML on the
    /// queue's connection and changes no table's shape, so `nvs queue migrate` stays the only thing
    /// that does.
    QueuePurge,
}

/// What a capability is being asked *about* — the second half of § 1's question.
#[derive(Clone, Copy, Debug)]
pub enum Scope<'a> {
    /// The grant is the whole answer: there is no argument to place inside it.
    Unscoped,
    /// A filesystem path, resolved by § 4's canonicalise-then-prefix rule.
    Path(&'a Path),
    /// A hostname, matched case-insensitively because DNS is.
    Host(&'a str),
    /// An endpoint an opening would bind, matched against entries **parsed as endpoints** rather
    /// than against the strings they were written as.
    ///
    /// A `SocketAddr` and not a `&str` because that is what "matched exactly" has to mean of an
    /// address: `127.0.0.1:80` and `[::ffff:127.0.0.1]:80` are one endpoint and would be two grants
    /// under a string comparison, and the caller has resolved one before it can bind anything
    /// anyway. An entry that does not parse as an endpoint matches nothing,
    /// which is `rule:security/net-listen-is-a-separate-grant-from-net-connect`'s own last sentence.
    Endpoint(std::net::SocketAddr),
    /// A name the deployment wrote — a `[db.<name>]` block, or the queue a `Core\Queue` receipt
    /// carries — matched exactly, because a name is not a hostname and two of them differing only
    /// in case are two names.
    Name(&'a str),
}

/// The run a capability is asked for: what a default depends on, and nothing else.
///
/// A run is a CLI run or server work, and it has an app folder: the canonical folder of the entry
/// file it started from, or the `root` of the most specific `[[app]]` block covering that entry
/// ([`crate::app::folder`]). An isolate takes the run of the tree that spawned it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run<'a> {
    /// Whether this is server work: a request, a connection, a scheduled run, a queued job, or an
    /// isolate in one of their trees.
    pub serving: bool,
    /// The app folder, or `None` where the run has no entry file to take one from.
    pub folder: Option<&'a Path>,
    /// The directories `Core\IO::temporaryDir` made for this context, canonical. Empty for an
    /// asker with no context, which then meets the narrower default.
    pub temporary: &'a [PathBuf],
    /// Whether the configuration names `development` as the mode this run started in. A runtime
    /// flip does not move it (`rule:config/a-startup-default-is-never-flipped`).
    pub development: bool,
}

impl<'a> Run<'a> {
    /// A CLI run: `nvs run`, a `#[Command]` method or `nvs test`.
    #[must_use]
    pub fn cli(folder: Option<&'a Path>) -> Self {
        Self {
            serving: false,
            folder,
            temporary: &[],
            development: false,
        }
    }

    /// Server work.
    #[must_use]
    pub fn server(folder: Option<&'a Path>) -> Self {
        Self {
            serving: true,
            folder,
            temporary: &[],
            development: false,
        }
    }

    /// This run, started in `development` where `development` is true.
    #[must_use]
    pub fn in_development(self, development: bool) -> Self {
        Self {
            development,
            ..self
        }
    }

    /// This run, with `temporary` as the directories its context made.
    #[must_use]
    pub fn with_temporary(self, temporary: &'a [PathBuf]) -> Self {
        Self { temporary, ..self }
    }

    /// Whether the canonical `path` is one [`crate::defaults::Granted::AppFolder`] covers: inside
    /// the app folder with no component below it that starts with `.` and none named `nvs.toml`,
    /// or inside one of this run's temporary directories.
    ///
    /// The name is compared without case, so `NVS.TOML` on a filesystem that ignores case is
    /// refused too. Refusing it on one that does not is the safe direction.
    fn reads_by_default(&self, path: &Path) -> bool {
        self.in_app_folder(path) || self.writes_by_default(path)
    }

    /// Whether the canonical `path` is one [`crate::defaults::Granted::AppCode`] covers: inside the
    /// app folder with no component below it that starts with `.` and none named `nvs.toml`, and
    /// inside none of this run's temporary directories, so a run never spawns a script it wrote.
    fn spawns_by_default(&self, path: &Path) -> bool {
        self.in_app_folder(path) && !self.writes_by_default(path)
    }

    /// Whether the canonical `path` is inside the app folder with no component below it that
    /// starts with `.` and none named `nvs.toml`.
    fn in_app_folder(&self, path: &Path) -> bool {
        self.folder
            .and_then(|folder| path.strip_prefix(folder).ok())
            .is_some_and(|below| !kept_from_requests(below))
    }

    /// Whether the canonical `path` is inside one of this run's temporary directories:
    /// [`crate::defaults::Granted::Temporary`].
    fn writes_by_default(&self, path: &Path) -> bool {
        self.temporary.iter().any(|dir| path.starts_with(dir))
    }
}

/// Server work with no app folder: the run no default can be wider than, for a caller that does
/// not know which run it is asking for.
impl Default for Run<'_> {
    fn default() -> Self {
        Self::server(None)
    }
}

/// What one run may do: the `[capabilities]` written for it, and the defaults
/// ([`crate::defaults::capability`]) for every key not written.
///
/// **Every asker goes through this**, so the runtime, the boot's schedule check, an extension's
/// caller and `Core\Cap::has` cannot meet two different defaults. The methods mirror
/// [`Capabilities`]' own, which answer for the written keys alone. A written key replaces its
/// default whole, per key: `read = []` grants nothing, and a block header with no key under it
/// leaves every default in place.
#[derive(Clone, Copy, Debug)]
pub struct Authority<'a> {
    written: Option<&'a Capabilities>,
    run: Run<'a>,
}

impl<'a> Authority<'a> {
    /// The authority of `run` under `written`, where `None` is a tree with no `[capabilities]`.
    #[must_use]
    pub fn new(written: Option<&'a Capabilities>, run: Run<'a>) -> Self {
        Self { written, run }
    }

    /// This authority, with `temporary` as the directories its context made: what a door with a
    /// context in hand asks, so [`crate::defaults::Granted::Temporary`] covers them.
    #[must_use]
    pub fn with_temporary(self, temporary: &'a [PathBuf]) -> Self {
        Self {
            run: self.run.with_temporary(temporary),
            ..self
        }
    }

    /// The default of `cap`, or `None` where `cap`'s key is written and the written grant answers.
    fn default_of(&self, cap: Cap) -> Option<crate::defaults::Granted> {
        let written = self.written.and_then(|caps| cap.grant(caps)).is_some();
        (!written).then(|| crate::defaults::capability(cap, self.run))
    }

    /// Whether the default `granted` covers the canonical `path`.
    fn default_covers(&self, granted: crate::defaults::Granted, path: &Path) -> bool {
        use crate::defaults::Granted;

        match granted {
            Granted::Nothing => false,
            Granted::Everything => true,
            Granted::AppFolder => self.run.reads_by_default(path),
            Granted::AppCode => self.run.spawns_by_default(path),
            Granted::Temporary => self.run.writes_by_default(path),
        }
    }

    /// [`Capabilities::allows`], with the default where `cap` is not written.
    #[must_use]
    pub fn allows(&self, cap: Cap, scope: Scope<'_>, files: &dyn Files) -> bool {
        use crate::defaults::Granted;

        match (self.default_of(cap), self.written, scope) {
            // An unscoped `script.spawn` is the method form, whose code is already in the unit.
            (Some(Granted::Everything | Granted::AppCode), _, Scope::Unscoped)
            | (Some(Granted::Everything), _, _) => true,
            (Some(granted), _, Scope::Path(path)) => {
                resolved(path, files).is_some_and(|path| self.default_covers(granted, &path))
            }
            (Some(_), _, _) | (None, None, _) => false,
            (None, Some(caps), _) => caps.allows(cap, scope, files),
        }
    }

    /// [`Capabilities::holds`], with the default where `cap` is not written.
    ///
    /// A default that covers part of the filesystem ([`crate::defaults::Granted::AppFolder`] and
    /// [`crate::defaults::Granted::Temporary`]) has no list of written roots to give, so it holds
    /// [`Grant::Nothing`] here. The caller is an extension's grant, which then gets no files by
    /// default in server work.
    #[must_use]
    pub fn holds(&self, cap: Cap) -> Grant<'a> {
        use crate::defaults::Granted;

        match (self.default_of(cap), self.written) {
            (Some(Granted::Everything), _) => Grant::Everything,
            (Some(_), _) | (None, None) => Grant::Nothing,
            (None, Some(caps)) => caps.holds(cap),
        }
    }

    /// [`Capabilities::allows_resolved`], with the default where `cap` is not written.
    #[must_use]
    pub fn allows_resolved(&self, cap: Cap, path: &Path) -> bool {
        match (self.default_of(cap), self.written) {
            (Some(granted), _) => self.default_covers(granted, path),
            (None, None) => false,
            (None, Some(caps)) => caps.allows_resolved(cap, path),
        }
    }

    /// [`Capabilities::allows_host`], with the default where `cap` is not written.
    #[must_use]
    pub fn allows_host(&self, cap: Cap, host: &str) -> bool {
        match (self.default_of(cap), self.written) {
            (Some(granted), _) => granted == crate::defaults::Granted::Everything,
            (None, None) => false,
            (None, Some(caps)) => caps.allows_host(cap, host),
        }
    }

    /// [`Capabilities::allows_name`], with the default where `cap` is not written.
    #[must_use]
    pub fn allows_name(&self, cap: Cap, name: &str) -> bool {
        match (self.default_of(cap), self.written) {
            (Some(granted), _) => granted == crate::defaults::Granted::Everything,
            (None, None) => false,
            (None, Some(caps)) => caps.allows_name(cap, name),
        }
    }

    /// [`Capabilities::address_refused`] for this run. With no `[capabilities.net] internal`
    /// written, the exceptions are [`crate::defaults::internal`]'s: loopback on a CLI run and in
    /// development, and none in production server work.
    #[must_use]
    pub fn address_refused(&self, address: std::net::IpAddr) -> Option<&'static str> {
        match self.written {
            Some(caps) if caps.internal().is_some() => caps.address_refused(address),
            _ => {
                let range = denied_by_default(address)?;
                let defaults = crate::defaults::internal(self.run).iter().copied();
                (!internal_covers(defaults, address)).then_some(range)
            }
        }
    }
}

/// The address ranges `rule:security/net-address-policy` denies
/// before any grant is consulted, named so a refusal can say which one it was.
///
/// **Here rather than in the client**, which is § 5: `Core\Http`, `Core\Net`, `Core\Db::open`'s
/// program-supplied target and any socket a host import hands a Tier 1 extension are all subject to
/// the same policy, and a copy of this table in each of them is a set of copies that agree until
/// one of them does not. The one class of address it does not govern is a `[db.<name>]` block an
/// operator wrote into root-owned configuration and granted by name (§ 3), which is why the caller
/// asks this rather than it being folded into [`Cap::NetConnect`]'s own grant check.
///
/// It is asked of a **resolved address**, never of a hostname: a hostname the operator never named
/// can resolve into any of these, which is the whole of why § 2's launderer pins.
///
/// This is the table alone. The operator's exception half — a service that must reach an internal
/// API saying so in `nvs.toml` — is [`Capabilities::address_refused`], which is what a door asks:
/// nothing widens *this* function, so a caller holding no configuration gets § 3's answer.
#[must_use]
pub fn denied_by_default(address: std::net::IpAddr) -> Option<&'static str> {
    use std::net::IpAddr;

    let address = embedded(address);
    match address {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            if v4.is_loopback() {
                Some("loopback (127.0.0.0/8)")
            } else if octets[0] == 0 {
                Some("unspecified (0.0.0.0/8)")
            } else if v4.is_link_local() {
                Some("link-local (169.254.0.0/16)")
            } else if v4.is_private() {
                Some("private (10/8, 172.16/12, 192.168/16)")
            } else if octets[0] == 100 && octets[1] & 0xc0 == 64 {
                Some("shared address space (100.64.0.0/10)")
            } else if octets[..3] == [192, 0, 0] {
                Some("protocol assignments (192.0.0.0/24)")
            } else if octets[0] == 198 && octets[1] & 0xfe == 18 {
                Some("benchmarking (198.18.0.0/15)")
            } else if v4.is_multicast() {
                Some("multicast (224.0.0.0/4)")
            } else if v4.is_broadcast() {
                Some("broadcast (255.255.255.255)")
            } else if octets[0] >= 240 {
                Some("reserved (240.0.0.0/4)")
            } else {
                None
            }
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            let first = segments[0];
            if v6.is_loopback() {
                Some("loopback (::1)")
            } else if v6.is_unspecified() {
                Some("unspecified (::)")
            } else if first & 0xffc0 == 0xfe80 {
                Some("link-local (fe80::/10)")
            } else if first & 0xffc0 == 0xfec0 {
                Some("site-local (fec0::/10)")
            } else if first & 0xfe00 == 0xfc00 {
                Some("private (fc00::/7)")
            } else if segments[..3] == [0x64, 0xff9b, 1] {
                // RFC 8215's prefix is a local translator's, and RFC 6052 lets it carry an IPv4
                // address in any of six layouts. Which one is the translator's configuration, never
                // the address's, so no reading of the bits can say which address it reaches.
                Some("local-use NAT64 (64:ff9b:1::/48)")
            } else if v6.is_multicast() {
                Some("multicast (ff00::/8)")
            } else {
                None
            }
        }
    }
}

/// An IPv4-mapped IPv6 address as the address it maps to, and every other address unchanged.
///
/// The same machine is reachable under two spellings, so both the table above and the exception
/// list below read an address through this rather than as a sixteenth of the v6 space — `rule:security/net-address-policy`
/// names the mapped forms explicitly because omitting them is how this check is usually
/// defeated, and an exception matched in one spelling and denied in the other would be the same
/// hole from the other side.
fn unmapped(address: std::net::IpAddr) -> std::net::IpAddr {
    use std::net::IpAddr;

    match address {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        held => held,
    }
}

/// A `net.internal` entry: an address, or a CIDR range such as `172.18.0.0/16`. `None` for anything
/// else, which [`validate`] refuses at load: a hostname, a prefix longer than the address, and a
/// range with bits set below its prefix (`10.0.0.5/8`), which reads as one host and grants a block.
///
/// An IPv4-mapped entry is taken unmapped, as an address asked of it is.
fn internal_range(entry: &str) -> Option<(std::net::IpAddr, u8)> {
    use std::net::IpAddr;

    let (address, prefix) = match entry.split_once('/') {
        Some((address, prefix)) => (address, Some(prefix)),
        None => (entry, None),
    };
    let address = unmapped(address.parse::<IpAddr>().ok()?);
    let width = if address.is_ipv4() { 32 } else { 128 };
    let prefix = match prefix {
        None => width,
        Some(digits) if digits.bytes().all(|byte| byte.is_ascii_digit()) => digits.parse().ok()?,
        Some(_) => return None,
    };
    (prefix <= width && masked(address, prefix) == address).then_some((address, prefix))
}

/// `address` with every bit below its first `prefix` bits cleared.
fn masked(address: std::net::IpAddr, prefix: u8) -> std::net::IpAddr {
    use std::net::IpAddr;

    match address {
        IpAddr::V4(v4) => {
            let mask = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
            IpAddr::V4((u32::from(v4) & mask).into())
        }
        IpAddr::V6(v6) => {
            let mask = u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
            IpAddr::V6((u128::from(v6) & mask).into())
        }
    }
}

/// Whether one of the `net.internal` `entries` covers `address`. An entry [`internal_range`] cannot
/// read covers nothing.
fn internal_covers<'e>(
    entries: impl IntoIterator<Item = &'e str>,
    address: std::net::IpAddr,
) -> bool {
    let wanted = unmapped(address);
    entries
        .into_iter()
        .filter_map(internal_range)
        .any(|(network, prefix)| {
            network.is_ipv4() == wanted.is_ipv4() && masked(wanted, prefix) == network
        })
}

/// An IPv6 address that carries an IPv4 address in its low 32 bits, as that IPv4 address; every
/// other address as [`unmapped`] reads it.
///
/// The table asks this rather than [`unmapped`] because four more prefixes route to an IPv4 address
/// they embed. The IPv4-compatible `::a.b.c.d` (`::/96`, apart from `::` and `::1`, which are
/// addresses of their own) and the NAT64 well-known prefix `64:ff9b::/96` carry it in the low 32
/// bits. 6to4 (`2002::/16`) carries it in the 32 bits after the prefix, and a relay delivers to it.
/// Teredo (`2001::/32`) carries the client's IPv4 address in the low 32 bits with every bit
/// inverted. Each spelling of `127.0.0.1` reaches whatever that address reaches, so each is denied
/// exactly when the address it carries is. The local-use NAT64 prefix `64:ff9b:1::/48` is not read
/// here: its layout is its translator's, so [`denied_by_default`] denies the whole prefix instead.
///
/// The exception list and endpoint matching keep [`unmapped`]: an operator's `internal` entry is an
/// address the operator wrote, and it excepts that address in its two usual spellings and nothing
/// the table reads more widely.
fn embedded(address: std::net::IpAddr) -> std::net::IpAddr {
    use std::net::{IpAddr, Ipv4Addr};

    let address = unmapped(address);
    let IpAddr::V6(v6) = address else {
        return address;
    };
    let s = v6.segments();
    let o = v6.octets();
    let compatible = s[..6] == [0; 6] && !v6.is_loopback() && !v6.is_unspecified();
    let nat64 = s[..6] == [0x64, 0xff9b, 0, 0, 0, 0];
    if compatible || nat64 {
        IpAddr::V4(Ipv4Addr::new(o[12], o[13], o[14], o[15]))
    } else if s[0] == 0x2002 {
        IpAddr::V4(Ipv4Addr::new(o[2], o[3], o[4], o[5]))
    } else if s[..2] == [0x2001, 0] {
        IpAddr::V4(Ipv4Addr::new(!o[12], !o[13], !o[14], !o[15]))
    } else {
        address
    }
}

impl Cap {
    /// Every capability, for a guard test and for `nvs meta`.
    pub const ALL: &'static [Self] = &[
        Self::FsRead,
        Self::FsWrite,
        Self::ScriptSpawn,
        Self::NetConnect,
        Self::NetListen,
        Self::NetLocal,
        Self::NetConnectTo,
        Self::NetDowngrade,
        Self::TlsAnchors,
        Self::TlsPin,
        Self::TlsAnyName,
        Self::TlsInsecure,
        Self::ProcessExec,
        Self::DebugTrace,
        Self::DebugProfile,
        Self::DbConnect,
        Self::DbOpen,
        Self::DbSchema,
        Self::LdapConnect,
        Self::LdapOpen,
        Self::LdapCleartext,
        Self::MailSend,
        Self::CacheShared,
        Self::QueuePurge,
    ];

    /// The name `nvs.toml` grants it under, which is also the name a refusal prints — the operator
    /// reading that message is about to paste this string into a configuration file.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FsRead => "fs.read",
            Self::FsWrite => "fs.write",
            Self::ScriptSpawn => "script.spawn",
            Self::NetConnect => "net.connect",
            Self::NetListen => "net.listen",
            Self::NetLocal => "net.local",
            Self::NetConnectTo => "net.connect_to",
            Self::NetDowngrade => "net.downgrade",
            Self::TlsAnchors => "tls.anchors",
            Self::TlsPin => "tls.pin",
            Self::TlsAnyName => "tls.any_name",
            Self::TlsInsecure => "tls.insecure",
            Self::ProcessExec => "process.exec",
            Self::DebugTrace => "debug.trace",
            Self::DebugProfile => "debug.profile",
            Self::DbConnect => "db.connect",
            Self::DbOpen => "db.open",
            Self::DbSchema => "db.schema",
            Self::LdapConnect => "ldap.connect",
            Self::LdapOpen => "ldap.open",
            Self::LdapCleartext => "ldap.cleartext",
            Self::MailSend => "mail.send",
            Self::CacheShared => "cache.shared",
            Self::QueuePurge => "queue.purge",
        }
    }

    /// The `[capabilities.<family>]` table a grant for it is written under, without the brackets —
    /// `fs` for `fs.read`, which a refusal points the operator at.
    ///
    /// It is the family half of [`name`](Self::name) rather than a table of its own, because a
    /// dotted capability name *is* that nesting
    /// (`rule:config/lists-are-arrays-and-repeated-records-are-arrays-of-tables`): `fs.read = [...]`
    /// under `[capabilities]` and a `read` key under `[capabilities.fs]` are the same input. A
    /// second list here would be one more thing to keep in step with the one above it, and would
    /// buy nothing the split does not.
    #[must_use]
    pub fn family(self) -> &'static str {
        let name = self.name();
        name.split_once('.').map_or(name, |(family, _)| family)
    }

    /// The capability that [`name`](Self::name) spells, or `None` for a name no capability has.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|cap| cap.name() == name)
    }

    /// Whether this capability's grant is a list of **paths**, and so whether
    /// [`Capabilities::canonicalize`] has work to do on it.
    #[must_use]
    pub const fn is_path_scoped(self) -> bool {
        matches!(
            self,
            Self::FsRead
                | Self::FsWrite
                | Self::ScriptSpawn
                | Self::NetLocal
                | Self::ProcessExec
                | Self::DebugTrace
                | Self::DebugProfile
        )
    }

    /// Whether a grant entry for this capability may be written `*.suffix`, per the module doc's
    /// § *A `db.open` entry may be a `*.` wildcard*.
    ///
    /// `db.open` and `ldap.open`, the two whose program-supplied targets a tenant-per-subdomain
    /// deployment cannot enumerate. The knowledge lives here rather than in `host_granted` for the reason the
    /// module doc gives about [`name`](Self::name): a capability's properties are arms of this type,
    /// never a string compared in a second place.
    #[must_use]
    pub const fn takes_host_wildcard(self) -> bool {
        matches!(self, Self::DbOpen | Self::LdapOpen)
    }

    /// Whether this capability's grant is a list of **hosts**, compared by [`host_granted`] against
    /// the bare host a call names — so an entry carrying a port or a scheme matches nothing, and
    /// [`validate`] refuses one.
    ///
    /// `db.open` is here and also takes an absolute SQLite path, which [`validate`] tells apart by
    /// its separator.
    #[must_use]
    pub const fn is_host_scoped(self) -> bool {
        matches!(
            self,
            Self::NetConnect
                | Self::NetConnectTo
                | Self::NetDowngrade
                | Self::TlsAnchors
                | Self::TlsPin
                | Self::TlsAnyName
                | Self::TlsInsecure
                | Self::DbOpen
                | Self::LdapOpen
                | Self::LdapCleartext
        )
    }

    /// Whether `true` is a spelling this capability's grant has at all — every capability's but the
    /// seven that name hosts a weakening applies to
    /// (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`).
    ///
    /// A `true` written for one of those reads as [`Grant::Nothing`] rather than as everything,
    /// which is [`CapNet::internal`](crate::tree::CapNet::internal)'s answer to the same question:
    /// turning a check off wholesale is not what these grants are for, and their value is that a
    /// reviewer can see which hosts a deployment bought back. It fails closed, so a deployment that
    /// wrote `true` relaxes nothing until it names the hosts it meant.
    ///
    /// Here rather than at each asker for [`name`](Self::name)'s reason: a capability's properties
    /// are arms of this type, never a string compared in a second place.
    #[must_use]
    pub const fn takes_true_spelling(self) -> bool {
        !matches!(
            self,
            Self::NetConnectTo
                | Self::NetDowngrade
                | Self::TlsAnchors
                | Self::TlsPin
                | Self::TlsAnyName
                | Self::TlsInsecure
                | Self::LdapCleartext
        )
    }

    /// What `caps` grants for this capability, or `None` when the block is absent — which is a
    /// refusal, not an omission.
    #[must_use]
    pub fn grant(self, caps: &Capabilities) -> Option<&Setting> {
        match self {
            Self::FsRead => caps.fs.as_ref()?.read.as_ref(),
            Self::FsWrite => caps.fs.as_ref()?.write.as_ref(),
            Self::ScriptSpawn => caps.script.as_ref()?.spawn.as_ref(),
            Self::NetConnect => caps.net.as_ref()?.connect.as_ref(),
            Self::NetListen => caps.net.as_ref()?.listen.as_ref(),
            Self::NetLocal => caps.net.as_ref()?.local.as_ref(),
            Self::NetConnectTo => caps.net.as_ref()?.connect_to.as_ref(),
            Self::NetDowngrade => caps.net.as_ref()?.downgrade.as_ref(),
            Self::TlsAnchors => caps.tls.as_ref()?.anchors.as_ref(),
            Self::TlsPin => caps.tls.as_ref()?.pin.as_ref(),
            Self::TlsAnyName => caps.tls.as_ref()?.any_name.as_ref(),
            Self::TlsInsecure => caps.tls.as_ref()?.insecure.as_ref(),
            Self::ProcessExec => caps.process.as_ref()?.exec.as_ref(),
            Self::DebugTrace => caps.debug.as_ref()?.trace.as_ref(),
            Self::DebugProfile => caps.debug.as_ref()?.profile.as_ref(),
            Self::DbConnect => caps.db.as_ref()?.connect.as_ref(),
            Self::DbOpen => caps.db.as_ref()?.open.as_ref(),
            Self::DbSchema => caps.db.as_ref()?.schema.as_ref(),
            Self::LdapConnect => caps.ldap.as_ref()?.connect.as_ref(),
            Self::LdapOpen => caps.ldap.as_ref()?.open.as_ref(),
            Self::LdapCleartext => caps.ldap.as_ref()?.cleartext.as_ref(),
            Self::MailSend => caps.mail.as_ref()?.send.as_ref(),
            Self::CacheShared => caps.cache.as_ref()?.shared.as_ref(),
            Self::QueuePurge => caps.queue.as_ref()?.purge.as_ref(),
        }
    }

    /// [`grant`](Self::grant)'s mirror, for [`Capabilities::canonicalize`] alone.
    ///
    /// Written out a second time rather than derived: the arms *are* the name-to-field mapping this
    /// type exists to hold once, and a mapping expressed as a shared traversal would be a third thing
    /// to keep in step with both.
    fn grant_mut(self, caps: &mut Capabilities) -> Option<&mut Setting> {
        match self {
            Self::FsRead => caps.fs.as_mut()?.read.as_mut(),
            Self::FsWrite => caps.fs.as_mut()?.write.as_mut(),
            Self::ScriptSpawn => caps.script.as_mut()?.spawn.as_mut(),
            Self::NetConnect => caps.net.as_mut()?.connect.as_mut(),
            Self::NetListen => caps.net.as_mut()?.listen.as_mut(),
            Self::NetLocal => caps.net.as_mut()?.local.as_mut(),
            Self::NetConnectTo => caps.net.as_mut()?.connect_to.as_mut(),
            Self::NetDowngrade => caps.net.as_mut()?.downgrade.as_mut(),
            Self::TlsAnchors => caps.tls.as_mut()?.anchors.as_mut(),
            Self::TlsPin => caps.tls.as_mut()?.pin.as_mut(),
            Self::TlsAnyName => caps.tls.as_mut()?.any_name.as_mut(),
            Self::TlsInsecure => caps.tls.as_mut()?.insecure.as_mut(),
            Self::ProcessExec => caps.process.as_mut()?.exec.as_mut(),
            Self::DebugTrace => caps.debug.as_mut()?.trace.as_mut(),
            Self::DebugProfile => caps.debug.as_mut()?.profile.as_mut(),
            Self::DbConnect => caps.db.as_mut()?.connect.as_mut(),
            Self::DbOpen => caps.db.as_mut()?.open.as_mut(),
            Self::DbSchema => caps.db.as_mut()?.schema.as_mut(),
            Self::LdapConnect => caps.ldap.as_mut()?.connect.as_mut(),
            Self::LdapOpen => caps.ldap.as_mut()?.open.as_mut(),
            Self::LdapCleartext => caps.ldap.as_mut()?.cleartext.as_mut(),
            Self::MailSend => caps.mail.as_mut()?.send.as_mut(),
            Self::CacheShared => caps.cache.as_mut()?.shared.as_mut(),
            Self::QueuePurge => caps.queue.as_mut()?.purge.as_mut(),
        }
    }
}

/// What a [`Setting`] means when it is read as a grant, and what [`Capabilities::holds`] returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grant<'a> {
    /// Nothing is granted — an absent block, `false`, an empty list, or a number, which is not a
    /// spelling any capability has.
    Nothing,
    /// Everything is granted: the operator wrote `true`.
    Everything,
    /// These entries and no others.
    These(&'a [String]),
}

fn grant_of(setting: &Setting) -> Grant<'_> {
    match setting {
        Setting::Bool(true) => Grant::Everything,
        Setting::Text(one) => Grant::These(std::slice::from_ref(one)),
        Setting::List(many) if !many.is_empty() => Grant::These(many),
        Setting::Bool(false) | Setting::List(_) | Setting::Integer(_) | Setting::Float(_) => {
            Grant::Nothing
        }
    }
}

/// [`grant_of`] read for one capability: the same reading, with `true` downgraded to
/// [`Grant::Nothing`] wherever [`Cap::takes_true_spelling`] says that capability has no such
/// spelling.
///
/// Every asker below goes through this rather than through [`grant_of`] directly, so the runtime's
/// question, the compiler's and the boot's cannot answer one operator's `true` three ways. It is a
/// second function because [`grant_of`] is the reading of a [`Setting`] alone, and a capability
/// there would put the roster inside a value's own interpretation.
fn grant_for(cap: Cap, setting: &Setting) -> Grant<'_> {
    match grant_of(setting) {
        Grant::Everything if !cap.takes_true_spelling() => Grant::Nothing,
        read => read,
    }
}

/// Whether `host` is one of the hosts `list` grants, **case-insensitively because DNS is**.
///
/// One home for the comparison, because [`Capabilities::allows`] and
/// [`Capabilities::allows_host`] are the runtime's asker and the compiler's, and a check that
/// disagreed with the run it precedes is the one failure `rule:expressions/preparation-preserves-behaviour` forbids outright. `cap` is a
/// parameter for the same reason: [`Cap::takes_host_wildcard`] decides whether an entry may be a
/// pattern at all, and the two askers must not be able to answer that differently either.
fn host_granted(cap: Cap, list: &[String], host: &str) -> bool {
    list.iter().any(|entry| {
        if cap.takes_host_wildcard() && entry.starts_with('*') {
            // A pattern-looking entry is read as a pattern and never falls back to an exact match:
            // a `*` that silently became a grant for one host named `*` is the trap this arm exists
            // to close, and denying more than the exact arm would is the safe direction.
            wildcard_granted(entry, host)
        } else {
            entry.eq_ignore_ascii_case(host)
        }
    })
}

/// The module doc's wildcard rule, as the comparison: `entry` is `*.suffix`, and `host` ends with
/// `suffix` **at a label boundary** with at least one label of its own in front.
///
/// Compared over bytes rather than over `str` slices because a hostname arrives from a program and
/// need not be ASCII — a byte index taken from the *end* of one is not guaranteed to be a character
/// boundary, and slicing a `str` there panics. ASCII-case folding over the bytes is the same
/// comparison [`host_granted`]'s exact arm makes, for the same DNS reason.
fn wildcard_granted(entry: &str, host: &str) -> bool {
    let Some(suffix) = entry.strip_prefix("*.") else {
        return false;
    };
    if suffix.is_empty() {
        return false;
    }
    let (host, suffix) = (host.as_bytes(), suffix.as_bytes());
    let Some(dot) = host.len().checked_sub(suffix.len() + 1) else {
        return false;
    };
    dot > 0 && host[dot] == b'.' && host[dot + 1..].eq_ignore_ascii_case(suffix)
}

/// Whether `list` grants `endpoint`, per
/// `rule:security/net-listen-is-a-separate-grant-from-net-connect`: an entry is an `address:port`
/// literal, matched exactly, and an entry that does not parse as one matches nothing.
///
/// **Both sides are parsed**, which is what exact matching means here and is the whole reason
/// [`Scope::Endpoint`] carries a `SocketAddr`. A string comparison would make an operator's
/// `127.0.0.1:80` miss a program binding `[::ffff:127.0.0.1]:80` — and, worse, would read a typo as
/// a grant of whatever the typo happens to equal rather than as the nothing it is.
///
/// The two halves are compared separately rather than as whole `SocketAddr`s: a v6 endpoint carries
/// a flow label and a scope id, neither of which an operator writes into `nvs.toml` and neither of
/// which is part of which endpoint this is. The address goes through [`unmapped`] for
/// [`Capabilities::address_refused`]'s reason — the same machine is reachable under two spellings,
/// and a grant that matched one and not the other would be the same hole from the other side.
/// [`Scope::Name`]'s comparison: an entry granting a name it equals, and nothing else.
///
/// Exact, with no pattern of any kind — the names asked here are a `[db.<name>]` block, a mail
/// endpoint and a queue, each a flat string with no structure for a wildcard to match part of. It
/// is a function rather than a line in [`Capabilities::allows`] because
/// [`allows_name`](Capabilities::allows_name) asks the same question without a [`Files`] in hand,
/// and two spellings of *granted* are how a check and a run come to disagree.
fn name_granted(list: &[String], name: &str) -> bool {
    list.iter().any(|entry| entry == name)
}

fn path_granted(list: &[String], path: &Path) -> bool {
    list.iter().any(|root| path.starts_with(Path::new(root)))
}

fn endpoint_granted(list: &[String], endpoint: std::net::SocketAddr) -> bool {
    list.iter().any(|entry| {
        entry.parse::<std::net::SocketAddr>().is_ok_and(|granted| {
            granted.port() == endpoint.port() && unmapped(granted.ip()) == unmapped(endpoint.ip())
        })
    })
}

impl Capabilities {
    /// § 1's question: does this configuration grant `cap` for `scope`?
    ///
    /// `files` is the canonicalizer, and it is a parameter rather than the filesystem because
    /// [`Files::canonical`] is [`trust::canonical`](crate::trust::canonical) in production and a fake
    /// under test — the same seam [`app`](crate::app) resolves an `[[app]]` key through, and for the
    /// same reason: a second canonicalizer is how a `..` gets through.
    #[must_use]
    pub fn allows(&self, cap: Cap, scope: Scope<'_>, files: &dyn Files) -> bool {
        let Some(setting) = cap.grant(self) else {
            return false;
        };
        match (grant_for(cap, setting), scope) {
            (Grant::Nothing, _) => false,
            (Grant::Everything, _) => true,
            (Grant::These(_), Scope::Unscoped) => true,
            (Grant::These(list), Scope::Host(host)) => host_granted(cap, list, host),
            (Grant::These(list), Scope::Endpoint(endpoint)) => endpoint_granted(list, endpoint),
            (Grant::These(list), Scope::Name(name)) => name_granted(list, name),
            (Grant::These(list), Scope::Path(path)) => {
                resolved(path, files).is_some_and(|path| path_granted(list, &path))
            }
        }
    }

    /// What this configuration grants `cap`, read the way [`allows`](Self::allows) reads it.
    ///
    /// For a caller that intersects the grant with another set rather than asking about one
    /// argument: `nvs_ext::grants` keeps the part of an extension's root that a narrower root
    /// here covers, and no single `allows` question returns that part.
    #[must_use]
    pub fn holds(&self, cap: Cap) -> Grant<'_> {
        cap.grant(self)
            .map_or(Grant::Nothing, |setting| grant_for(cap, setting))
    }

    /// [`allows`](Self::allows) for a path that is already resolved, compared as it is written.
    ///
    /// A door that opens a file resolves its path once, asks this about the result, and opens that same result
    /// without following a link (`rule:security/writes-open-beneath-a-handle`). Resolving it a
    /// second time here could give a different path than the one the door opens, if a link
    /// changed in between.
    #[must_use]
    pub fn allows_resolved(&self, cap: Cap, path: &Path) -> bool {
        let Some(setting) = cap.grant(self) else {
            return false;
        };
        match grant_for(cap, setting) {
            Grant::Nothing => false,
            Grant::Everything => true,
            Grant::These(list) => path_granted(list, path),
        }
    }

    /// [`allows`](Self::allows) for a host, and the one form of the question a caller with no
    /// filesystem in front of it can ask.
    ///
    /// [`Scope::Host`] never reaches [`Files`] — a hostname is matched against the grant list and
    /// nothing is canonicalized — so demanding one is demanding a parameter the answer does not
    /// depend on. The compiler is the caller that has none:
    /// `rule:core-classes/db-compile-time-query-checking` has `nvs check` refuse a **literal**
    /// `Core\Db::open` host no `db.open` grant covers, and a checking pass has no request, no
    /// resolver and no reason to grow one. Both spellings share this list walk, so a run and a
    /// check cannot disagree about which hosts are granted.
    #[must_use]
    pub fn allows_host(&self, cap: Cap, host: &str) -> bool {
        let Some(setting) = cap.grant(self) else {
            return false;
        };
        match grant_for(cap, setting) {
            Grant::Nothing => false,
            Grant::Everything => true,
            Grant::These(list) => host_granted(cap, list, host),
        }
    }

    /// [`allows`](Self::allows) for a name matched exactly, and the second form of the question a
    /// caller with no filesystem in front of it can ask.
    ///
    /// [`Scope::Name`] never reaches [`Files`] for [`allows_host`](Self::allows_host)'s reason —
    /// a name is compared against the grant list as written, and there is nothing to canonicalize.
    /// The caller with no filesystem is the compiler again:
    /// `rule:concurrency/queue-deletion-is-explicit-and-bounded` has `nvs check` refuse a
    /// **literal** `Core\Queue::purge` queue name no `queue.purge` grant covers, under `E0637`,
    /// and it walks this same list so that a check and a run cannot disagree about which queues
    /// are granted.
    ///
    /// There is no pattern here and no `*.` spelling to add one: a queue name is a flat string the
    /// program picked, so it has no labels to match at a boundary of.
    #[must_use]
    pub fn allows_name(&self, cap: Cap, name: &str) -> bool {
        let Some(setting) = cap.grant(self) else {
            return false;
        };
        match grant_for(cap, setting) {
            Grant::Nothing => false,
            Grant::Everything => true,
            Grant::These(list) => name_granted(list, name),
        }
    }

    /// [`allows`](Self::allows) for a grant that has nothing to be asked *about*, and the third
    /// form of the question a caller with no filesystem in front of it can ask.
    ///
    /// [`Scope::Unscoped`] never reaches [`Files`] for [`allows_host`](Self::allows_host)'s
    /// reason: there is no argument to canonicalize, so any grant an operator wrote is the whole
    /// answer. The caller with none is the boot — [`crate::store::advise`] reads this out of the
    /// merged tree to report `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s
    /// pair, long before there is a request or a resolver.
    #[must_use]
    pub fn allows_unscoped(&self, cap: Cap) -> bool {
        let Some(setting) = cap.grant(self) else {
            return false;
        };
        match grant_for(cap, setting) {
            Grant::Nothing => false,
            Grant::Everything | Grant::These(_) => true,
        }
    }

    /// Every host `[capabilities.tls]` relaxes outbound verification for, as the capability that
    /// relaxes it and the host it names — what the boot prints, one line per grant and host, per
    /// `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s last sentence.
    ///
    /// The grants are found by [`Cap::family`] rather than by a roster of their own, because every
    /// capability under `tls` is a relaxation of verification — that is what the block is — so one
    /// added later is reported the day it exists rather than the day someone notices a second list.
    ///
    /// The reading is [`grant_for`]'s, like every other asker's: a `true`, which none of these
    /// grants has a spelling for, names no host here either, so a deployment that wrote one is told
    /// about no weakening and has none. The order is the roster's, then the operator's within a
    /// grant.
    #[must_use]
    pub fn tls_relaxations(&self) -> Vec<(Cap, &str)> {
        Cap::ALL
            .iter()
            .copied()
            .filter(|cap| cap.family() == "tls")
            .flat_map(|cap| {
                let hosts = match cap.grant(self).map(|setting| grant_for(cap, setting)) {
                    Some(Grant::These(hosts)) => hosts,
                    Some(Grant::Nothing | Grant::Everything) | None => &[][..],
                };
                hosts.iter().map(move |host| (cap, host.as_str()))
            })
            .collect()
    }

    /// `rule:security/net-address-policy`'s question, asked of a
    /// **resolved address** rather than of a name: which denied range this address is in, or `None`
    /// when nothing refuses it.
    ///
    /// This is [`denied_by_default`] plus the operator's half of the policy — `net.internal`, the
    /// addresses a deployment says it reaches anyway. Its shape, each part of it a refusal to widen
    /// further than the rule does:
    ///
    /// - **An entry is an IP address or a CIDR range** (`172.18.0.0/16`), and one that does not
    ///   parse as either matches nothing. A hostname there would be read before resolution and so
    ///   would exempt whatever the name resolved to *afterwards*, which is the rebinding gap
    ///   `rule:http-server/allow-url-pins-the-address`'s pinning closes. A range names a container
    ///   network or a subnet the operator runs, which no list of addresses can follow.
    /// - **`true` grants nothing**, which is the one place a `Setting::Bool(true)` does not mean
    ///   everything. Turning the whole table off is not an exception, and the value of the exception
    ///   is that a reviewer can see which addresses a deployment bought back.
    ///
    /// It answers for the written key alone: with no `internal` written, the table refuses every
    /// address it names. [`Authority::address_refused`] adds the default.
    #[must_use]
    pub fn address_refused(&self, address: std::net::IpAddr) -> Option<&'static str> {
        let range = denied_by_default(address)?;
        let Some(setting) = self.internal() else {
            return Some(range);
        };
        let Grant::These(named) = grant_of(setting) else {
            return Some(range);
        };
        (!internal_covers(named.iter().map(String::as_str), address)).then_some(range)
    }

    /// The written `[capabilities.net] internal` key, or `None` where it is not written.
    fn internal(&self) -> Option<&Setting> {
        self.net.as_ref().and_then(|net| net.internal.as_ref())
    }

    /// § 4's grant side: replaces every path-scoped root with its canonical spelling, once.
    ///
    /// Called when the snapshot is built, exactly as an `[[app]]` block's key is canonicalized at the
    /// same point and for the same reason — a root still spelled the way the operator typed it is a
    /// comparison against the wrong thing, and doing it per call would put a `realpath` on the grant
    /// side of every check rather than only on the argument's.
    ///
    /// A root is resolved by [`resolved`], the walk the argument side takes, so a root that does not
    /// exist yet — a socket `net.local` names before anything has bound it, a directory a program
    /// creates — keeps its deepest existing ancestor canonical and its missing tail as written. Both
    /// sides of the comparison are then spelled by one resolution: a root under a symlinked
    /// directory (`/tmp` and `/var` on macOS, `/var/run` on most Linux systems) matches the argument
    /// that names the same file, where a root left as the operator typed it could never match the
    /// canonical argument and denied the one path it was written to grant.
    ///
    /// A root [`resolved`] has no answer for — no ancestor exists, or the missing tail holds a `..`
    /// — is **left as written**, which fails closed: a canonical argument holds no `..` and is not
    /// under it. Dropping it instead would silently discard what an operator asked for.
    pub fn canonicalize(&mut self, files: &dyn Files) {
        for cap in Cap::ALL.iter().copied().filter(|cap| cap.is_path_scoped()) {
            let Some(setting) = cap.grant_mut(self) else {
                continue;
            };
            match setting {
                Setting::Text(one) => canonical_root(one, files),
                Setting::List(many) => {
                    for one in many.iter_mut() {
                        canonical_root(one, files);
                    }
                }
                Setting::Bool(_) | Setting::Integer(_) | Setting::Float(_) => {}
            }
        }
    }
}

/// Makes every relative path-scoped grant absolute against the directory of the file that wrote it
/// (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`), in the global
/// `[capabilities]` block and in every `[[app]]` block's.
///
/// Runs over the merged tree for [`mod@crate::db`]'s reason: which file wrote a grant is a question
/// only the merge has answered, and a later file may have replaced it. [`Capabilities::canonicalize`]
/// then resolves a root that no longer depends on the directory the process was started in — without
/// this, `read = ["data"]` in `/etc/nvs/nvs.toml` granted whichever `data` sat beside the server's
/// working directory. The table is rewritten beside the config, because a snapshot is retyped from
/// the table (`Snapshot::retype`'s § *The seam every `resolve()` pass is measured against*).
///
/// An entry with a root of its own is left alone, and so is an empty one: `""` resolved against a
/// directory would grant that directory, where left as written it grants nothing.
pub fn anchor(config: &mut Config, table: &mut toml::Table, origins: &BTreeMap<String, Origin>) {
    anchor_block(
        config.capabilities.as_mut(),
        table.get_mut("capabilities"),
        "capabilities",
        origins,
    );
    let mut blocks = table.get_mut("app").and_then(toml::Value::as_array_mut);
    for (index, app) in config.app.iter_mut().enumerate() {
        let written = blocks
            .as_deref_mut()
            .and_then(|blocks| blocks.get_mut(index))
            .and_then(toml::Value::as_table_mut)
            .and_then(|block| block.get_mut("capabilities"));
        anchor_block(
            app.capabilities.as_mut(),
            written,
            &format!("app.{index}.capabilities"),
            origins,
        );
    }
}

/// [`anchor`] for one `[capabilities]` block, `prefix` being the dotted key it was merged under.
fn anchor_block(
    mut caps: Option<&mut Capabilities>,
    mut written: Option<&mut toml::Value>,
    prefix: &str,
    origins: &BTreeMap<String, Origin>,
) {
    for cap in Cap::ALL.iter().copied().filter(|cap| cap.is_path_scoped()) {
        let base = crate::db::written_in(origins, &format!("{prefix}.{}", cap.name()));
        let anchor_one = |one: &mut String| match cap {
            Cap::ProcessExec => anchor_program(one, base),
            _ => anchor_root(one, base),
        };
        match caps.as_deref_mut().and_then(|caps| cap.grant_mut(caps)) {
            Some(Setting::Text(one)) => anchor_one(one),
            Some(Setting::List(many)) => many.iter_mut().for_each(&anchor_one),
            _ => {}
        }
        let (family, key) = cap.name().split_once('.').unwrap_or((cap.name(), ""));
        let value = written
            .as_deref_mut()
            .and_then(toml::Value::as_table_mut)
            .and_then(|block| block.get_mut(family))
            .and_then(toml::Value::as_table_mut)
            .and_then(|family| family.get_mut(key));
        match value {
            Some(toml::Value::String(one)) => anchor_one(one),
            Some(toml::Value::Array(many)) => {
                for one in many.iter_mut() {
                    if let toml::Value::String(one) = one {
                        anchor_one(one);
                    }
                }
            }
            _ => {}
        }
    }
}

fn anchor_root(root: &mut String, base: &Path) {
    if !root.is_empty() && !Path::new(root.as_str()).has_root() {
        *root = crate::resolve::absolute(base, Path::new(root.as_str()))
            .to_string_lossy()
            .into_owned();
    }
}

/// [`anchor_root`] for a `process.exec` entry: a bare name such as `git` becomes the program
/// [`on_path`] finds, once, when the configuration loads
/// (`rule:security/process-exec-capability`). A name nothing on `PATH` answers stays as written,
/// and [`validate`] refuses it.
fn anchor_program(entry: &mut String, base: &Path) {
    match bare_program(Path::new(entry.as_str())).map(on_path) {
        Some(Some(found)) => *entry = found.to_string_lossy().into_owned(),
        Some(None) => {}
        None => anchor_root(entry, base),
    }
}

/// The program name `path` is when it is a bare name such as `git`: no separator anywhere in it,
/// no drive, and not `.` or `..`. So `bin/` and `./bin` name a folder, and `bin` names a program.
#[must_use]
pub fn bare_program(path: &Path) -> Option<&str> {
    let name = path.to_str()?;
    let separators: &[char] = if cfg!(windows) {
        &['/', '\\', ':']
    } else {
        &['/']
    };
    (!name.is_empty() && name != "." && name != ".." && !name.contains(separators)).then_some(name)
}

/// The program a shell would start for the bare name `name`: the first match in a folder of the
/// `PATH` variable, as an absolute path. A folder in `PATH` that does not start at a root is
/// skipped, because it names a different place for every working directory.
///
/// On Windows a name is also tried with each extension of `PATHEXT`, in order. On other systems
/// the file must be executable.
///
/// **Cost:** one `stat` per folder and extension tried. A CLI run asks it once per bare-name
/// call, and the configuration once per bare-name entry when it loads.
#[must_use]
pub fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|folder| folder.has_root())
        .find_map(|folder| program_in(&folder, name))
}

#[cfg(windows)]
fn program_in(folder: &Path, name: &str) -> Option<PathBuf> {
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    let as_written = Path::new(name)
        .extension()
        .map(|_| folder.join(name))
        .filter(|file| file.is_file());
    as_written.or_else(|| {
        extensions
            .split(';')
            .filter(|extension| !extension.is_empty())
            .map(|extension| folder.join(format!("{name}{extension}")))
            .find(|file| file.is_file())
    })
}

#[cfg(not(windows))]
fn program_in(folder: &Path, name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let file = folder.join(name);
    std::fs::metadata(&file)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .then_some(file)
}

/// `rule:config/check-and-dump-audit-the-tree-offline`'s refusal of a grant entry that grants
/// nothing however it is asked, in the global `[capabilities]` block, every `[[app]]` block's and
/// every `[[schedule]]`'s `grants`.
///
/// Each of these used to read as [`Grant::Nothing`] or as an entry no argument can equal, so the
/// deployment that wrote it was denied with no word about why: a host with a port or a scheme
/// ([`host_granted`] compares the bare host), a `*` (no grant takes a pattern but `*.` on
/// [`Cap::takes_host_wildcard`]'s two), `true` where [`Cap::takes_true_spelling`] says no, a
/// SQLite path under `db.open` (a SQLite file asks `fs.read` and `fs.write` alone),
/// a `net.listen` entry that is not an endpoint, and a `net.internal` entry that is not an address
/// ([`Capabilities::address_refused`]). The message names the entry and the file, and its help is
/// the line that grants what was meant.
///
/// Runs after [`anchor`], so a path-scoped entry is already absolute here, and over the merged tree
/// for that function's reason. `nvs config check` and the boot both resolve through it.
///
/// # Errors
///
/// `E0601` for the first such entry.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    validate_block(config.capabilities.as_ref(), "capabilities", origins)?;
    for (index, app) in config.app.iter().enumerate() {
        let prefix = format!("app.{index}.capabilities");
        validate_block(app.capabilities.as_ref(), &prefix, origins)?;
    }
    for (index, schedule) in config.schedule.iter().enumerate() {
        let prefix = format!("schedule.{index}.grants");
        validate_block(schedule.grants.as_ref(), &prefix, origins)?;
    }
    Ok(())
}

/// [`validate`] for one block, `prefix` being the dotted key it was merged under.
fn validate_block(
    caps: Option<&Capabilities>,
    prefix: &str,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let Some(caps) = caps else {
        return Ok(());
    };
    // `[capabilities.fs]`, `[app.capabilities.fs]` or `[schedule.grants.fs]`: the header the
    // operator writes, which carries no index.
    let table = |family: &str| {
        let block: Vec<&str> = prefix
            .split('.')
            .filter(|part| part.parse::<usize>().is_err())
            .collect();
        format!("[{}.{family}]", block.join("."))
    };
    let refuse = |key: &str, found: Silent, family: &str| {
        let mut refused = Diagnostic::error(code::E_BAD_DIRECTIVE, found.headline)
            .with_note(found.note.to_string());
        if let Some(origin) = origins.get(key) {
            refused = refused.with_note(format!("It is written in `{}`.", origin.path.display()));
        }
        let family = found.family.unwrap_or(family);
        refused.with_help(format!("write `{}` under `{}`", found.line, table(family)))
    };
    for cap in Cap::ALL.iter().copied() {
        let Some(setting) = cap.grant(caps) else {
            continue;
        };
        let key = format!("{prefix}.{}", cap.name());
        if let Some(found) = silent(cap, setting, crate::db::written_in(origins, &key)) {
            return Err(refuse(&key, found, cap.family()));
        }
    }
    if let Some(setting) = caps.net.as_ref().and_then(|net| net.internal.as_ref()) {
        let key = format!("{prefix}.net.internal");
        if let Some(found) = silent_internal(setting) {
            return Err(refuse(&key, found, "net"));
        }
    }
    Ok(())
}

/// One entry [`validate`] refuses: what the message says, the one sentence below it, and the line
/// to write instead.
struct Silent {
    headline: String,
    note: &'static str,
    line: String,
    /// The family whose table `line` goes under, where it is not the refused capability's own.
    family: Option<&'static str>,
}

impl Silent {
    fn new(headline: String, note: &'static str, line: String) -> Self {
        Self {
            headline,
            note,
            line,
            family: None,
        }
    }

    /// This refusal, with `line` written under `family`'s table.
    fn under(self, family: &'static str) -> Self {
        Self {
            family: Some(family),
            ..self
        }
    }
}

/// What is wrong with `setting` as a grant of `cap`, or `None` when every entry in it can grant
/// something. `base` is the folder of the file that wrote it.
fn silent(cap: Cap, setting: &Setting, base: &Path) -> Option<Silent> {
    let name = cap.name();
    let key = name.split_once('.').map_or(name, |(_, key)| key);
    let entries = match setting {
        Setting::Bool(true) if !cap.takes_true_spelling() => {
            return Some(Silent::new(
                format!("`{name}` does not accept `true`"),
                "This setting needs a list of hosts.",
                grant_list(key, "api.example.com"),
            ));
        }
        Setting::Text(one) => std::slice::from_ref(one),
        Setting::List(many) => many.as_slice(),
        Setting::Bool(_) | Setting::Integer(_) | Setting::Float(_) => return None,
    };
    entries
        .iter()
        .find_map(|entry| silent_entry(cap, key, entry, base))
}

/// [`silent`] for one entry of a list.
fn silent_entry(cap: Cap, key: &str, entry: &str, base: &Path) -> Option<Silent> {
    let name = cap.name();
    let pattern = || format!("the `{name}` entry `{entry}` is a pattern, so it matches nothing");
    if cap == Cap::ProcessExec
        && let Some(program) = bare_program(Path::new(entry))
        && on_path(program).is_none()
    {
        return Some(Silent::new(
            format!("the `process.exec` entry `{entry}` names no program on `PATH`"),
            "A program name is looked up on `PATH` when the configuration loads.",
            grant_list(key, &format!("/full/path/to/{program}")),
        ));
    }
    if cap.is_path_scoped() {
        // Anchored already, so the folder in front of the pattern is the one the operator meant,
        // cut from the entry as written so its separators stay theirs.
        let star = entry.find('*')?;
        let folder = entry[..star]
            .rfind(['/', '\\'])
            .map_or("./", |end| &entry[..=end]);
        return Some(Silent::new(
            pattern(),
            "A folder entry allows every file below it.",
            grant_list(key, folder),
        ));
    }
    if entry == "*" || entry == "*." {
        return Some(if cap.takes_true_spelling() {
            Silent::new(
                pattern(),
                "`true` allows every value.",
                format!("{key} = true"),
            )
        } else {
            Silent::new(
                pattern(),
                "This setting needs a list of hosts.",
                grant_list(key, "api.example.com"),
            )
        });
    }
    if cap == Cap::DbOpen && entry.contains(['/', '\\']) {
        // A SQLite file asks the file rule alone (`rule:core-classes/db-capabilities`), so a
        // path here grants nothing. The line to write is its folder under `fs.write`, which a
        // database needs for its journal.
        let absolute = crate::resolve::absolute(base, Path::new(entry));
        let folder = absolute.parent().unwrap_or(&absolute);
        return Some(
            Silent::new(
                format!("the `db.open` entry `{entry}` is a file path, so it matches nothing"),
                "A SQLite file needs `fs.read` and `fs.write`, not `db.open`.",
                grant_list("write", &folder.to_string_lossy()),
            )
            .under("fs"),
        );
    }
    if cap.is_host_scoped() {
        if entry.starts_with('*') && !cap.takes_host_wildcard() {
            return Some(Silent::new(
                format!("the `{name}` entry `{entry}` is a pattern, so it matches no host"),
                "Only `db.open` and `ldap.open` accept an entry that starts with `*.`.",
                grant_list(key, "api.example.com"),
            ));
        }
        if let Some(host) = bare_host(entry) {
            return Some(Silent::new(
                format!(
                    "the `{name}` entry `{entry}` has a port or a scheme, so it matches no host"
                ),
                "An entry is the host alone, and it allows every port.",
                grant_list(key, &host),
            ));
        }
    }
    if cap == Cap::NetListen && entry.parse::<std::net::SocketAddr>().is_err() {
        return Some(Silent::new(
            format!(
                "the `net.listen` entry `{entry}` is not an IP address and a port, so it matches \
                 nothing"
            ),
            "A host name is not allowed here.",
            grant_list(key, "127.0.0.1:8080"),
        ));
    }
    None
}

/// [`silent`] for `net.internal`, which is not a [`Cap`] and takes IP addresses and CIDR ranges
/// alone.
fn silent_internal(setting: &Setting) -> Option<Silent> {
    const NOTE: &str = "This setting needs a list of IP addresses or ranges, such as `10.0.0.0/8`.";
    let line = || grant_list("internal", "10.0.0.5");
    let entries = match setting {
        Setting::Bool(true) => {
            return Some(Silent::new(
                "`net.internal` does not accept `true`".to_string(),
                NOTE,
                line(),
            ));
        }
        Setting::Text(one) => std::slice::from_ref(one),
        Setting::List(many) => many.as_slice(),
        Setting::Bool(_) | Setting::Integer(_) | Setting::Float(_) => return None,
    };
    let entry = entries
        .iter()
        .find(|entry| internal_range(entry).is_none())?;
    Some(Silent::new(
        format!(
            "the `net.internal` entry `{entry}` is not an IP address or a range, so it matches nothing"
        ),
        NOTE,
        line(),
    ))
}

/// The host a host-list entry names when it also carries a scheme, a user, a path or a port, or
/// `None` when the entry is a host alone.
///
/// An IPv6 address is a host alone without brackets, and keeps its brackets when a port follows them.
fn bare_host(entry: &str) -> Option<String> {
    let rest = entry.split_once("://").map_or(entry, |(_, rest)| rest);
    let rest = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let rest = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    let host = if rest.parse::<std::net::IpAddr>().is_ok() {
        rest
    } else if let Some(end) = rest.strip_prefix('[').and_then(|inner| inner.find(']')) {
        &rest[..end + 2]
    } else {
        match rest.rsplit_once(':') {
            Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => {
                host
            }
            _ => rest,
        }
    };
    (host != entry && !host.is_empty()).then(|| host.to_string())
}

/// The line a refusal of `cap` over `scope` tells the operator to add under
/// `[capabilities.<family>]` ([`Cap::family`]), so the help under a denial is a line to paste rather
/// than a name to look up (`rule:security/denial-is-a-runtime-error`).
///
/// A file or folder under `fs.read`, `fs.write` or `script.spawn` gives the folder it is in, cut
/// from the path as the door was handed it so its separators stay the program's; a path under any
/// other grant, a host, a name and an endpoint give themselves. An unscoped refusal gives
/// `key = true`, or a host list where [`Cap::takes_true_spelling`] says `true` grants nothing.
/// Every line it gives is one [`validate`] accepts.
#[must_use]
pub fn grant_line(cap: Cap, scope: Scope<'_>) -> String {
    let name = cap.name();
    let key = name.split_once('.').map_or(name, |(_, key)| key);
    match scope {
        Scope::Unscoped if cap.takes_true_spelling() => format!("{key} = true"),
        Scope::Unscoped => grant_list(key, "api.example.com"),
        Scope::Path(path) => {
            let path = path.to_string_lossy();
            let entry = match cap {
                Cap::FsRead | Cap::FsWrite | Cap::ScriptSpawn => {
                    path.rfind(['/', '\\']).map_or(&*path, |end| &path[..=end])
                }
                _ => &path,
            };
            grant_list(key, entry)
        }
        Scope::Host(host) | Scope::Name(host) => grant_list(key, host),
        Scope::Endpoint(endpoint) => grant_list(key, &endpoint.to_string()),
    }
}

/// The `[capabilities.net]` line that excepts `address` from the address table:
/// `internal = ["10.0.0.5"]`, which the address refusal prints as its help.
#[must_use]
pub fn internal_line(address: std::net::IpAddr) -> String {
    grant_list("internal", &address.to_string())
}

/// `key = ["value"]`, the value written as a TOML basic string.
fn grant_list(key: &str, value: &str) -> String {
    format!("{key} = [{}]", toml_string(value))
}

/// `value` as a TOML basic string, so a Windows path pastes back as the path it is.
fn toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn canonical_root(root: &mut String, files: &dyn Files) {
    if let Some(found) = resolved(Path::new(root.as_str()), files) {
        *root = found.to_string_lossy().into_owned();
    }
}

/// The bit an own-file word sets (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
/// The bits below it, except bit 32, are [`own_file_word`]'s hash. Bit 32 is `Core\Cap::has`'s
/// tag, so a word meant for one member never reads as another's.
pub const OWN_FILE_WORD: i64 = 1 << 62;

/// The word that approves a read of the file at `resolved`, a path [`resolved`] gave, with no
/// `fs.read` scope: [`OWN_FILE_WORD`] and 61 bits of the path's BLAKE3 hash.
///
/// The compiler mints it for a package's string literal that names a file of the package's own set,
/// and a `Core\IO` read door compares it with the word of the path it resolved when it runs. So the
/// door passes exactly the file the compiler approved, and a link changed since then fails the
/// comparison and leaves the operator's scope to decide. **Cost:** one hash of the path's bytes.
#[must_use]
pub fn own_file_word(resolved: &Path) -> i64 {
    let hash = blake3::hash(resolved.as_os_str().as_encoded_bytes());
    let mut low = [0; 8];
    low.copy_from_slice(&hash.as_bytes()[..8]);
    let bits = i64::from_le_bytes(low) & (OWN_FILE_WORD - 1) & !(1 << 32);
    OWN_FILE_WORD | bits
}

/// § 4's argument side: `path` canonicalized, or its **deepest existing ancestor** canonicalized with
/// the rest re-appended.
///
/// A write creates the file it names, so its path cannot be canonicalized before the check — and
/// creating it to find out whether creating it is allowed is the wrong order. Pinning the deepest
/// ancestor that does exist resolves every symlink and every `..` above that point, which is where an
/// escape would have to come from.
///
/// `None` when even the deepest ancestor cannot be resolved, and when a component that is still
/// unresolved is `..` — [`Path::file_name`] is `None` for one, so the loop exits on it. That is the
/// single component that could still escape after the ancestor is pinned, and there is no legitimate
/// spelling of a new file's path that needs one.
///
/// A **bare** relative name — `copy.txt` — has [`Path::parent`] `""`, which is not a path any
/// canonicalizer can answer for; [`here`] spells it `.` so that the ancestor walk reaches the
/// current directory instead of running out of components and denying. That is the ordinary
/// spelling of a path a program writes, so denying it denied a grant of `.` its whole point.
///
/// Public because `nvs_runtime::capability`'s resolution door answers `Core\IO::within` with the
/// **same** walk this check makes. A launderer that proved containment against a second
/// canonicalizer would be proving it about a different path than the one the grant was compared
/// against, and a `..` gets through exactly there — the same reason [`Capabilities::allows`] takes
/// its canonicalizer as a parameter rather than reaching for the filesystem.
///
/// **The walk skips the ancestors [`Files::missing`] rules out, by a binary search.** Asking
/// [`Files::canonical`] about each ancestor in turn costs one call per missing folder, each over the
/// whole remaining prefix, so a path a request names with many missing folders costs time quadratic
/// in its depth. The ancestors the walk can anchor at are the trailing run of plain names, since it
/// stops at the first `..`, root or prefix. Along that run "missing" is closed downwards: a folder
/// that is not there has nothing below it, and a prefix too long to name has only longer ones below
/// it. So the missing ancestors are exactly the deepest few, the search finds the deepest ancestor
/// that is not missing in a logarithmic number of calls, and the walk resumes there — every one it
/// skipped would have failed to canonicalize, so the answer is the one the full walk gives. "Canonicalizes" is **not** closed the same way,
/// which is why the search asks `missing` and never `canonical`: on Windows a folder whose ACL
/// denies reading it fails to canonicalize while a folder inside it does not, and a search that took
/// that failure for absence would anchor above it and leave a reparse point below it unresolved.
///
/// **A path that starts with two separators is `None`, and nothing is asked about it.** On Windows
/// it is a network, device or verbatim path, and canonicalizing one opens it: `\\host\share`
/// connects to `host`. [`windows_alias`] is the test, and only its first form is read here, since the
/// other forms are harmless to resolve and an 8.3 temporary folder must resolve to its long name.
#[must_use]
pub fn resolved(path: &Path, files: &dyn Files) -> Option<PathBuf> {
    if network_path(&path.to_string_lossy()) {
        return None;
    }
    if let Ok(found) = files.canonical(path) {
        return Some(found);
    }
    // `walk[i]` is `path` with its last `i` names removed, for every `i` the walk may anchor at.
    let mut walk = vec![path];
    while let Some(&cursor) = walk.last()
        && cursor.file_name().is_some()
        && let Some(parent) = cursor.parent()
    {
        walk.push(here(parent));
    }
    let (mut first, mut last) = (1, walk.len());
    while first < last {
        let middle = first + (last - first) / 2;
        if files.missing(walk[middle]) {
            first = middle + 1;
        } else {
            last = middle;
        }
    }
    (first..walk.len()).find_map(|anchor| {
        let mut resolved = files.canonical(walk[anchor]).ok()?;
        resolved.extend(
            walk[..anchor]
                .iter()
                .rev()
                .filter_map(|step| step.file_name()),
        );
        Some(resolved)
    })
}

/// Whether `below`, a path relative to an app folder, is one a request never reads there by
/// default: a component that starts with `.` (`.env`, `.git/config`) or is named `nvs.toml`, in
/// any case.
///
/// Server work's default `fs.read` and a static mount's files both ask this, so a file the
/// program cannot read is never one the server sends
/// (`rule:http-server/static-serving-is-one-policy`). Refusing `NVS.TOML` on a filesystem that
/// keeps case is the safe direction.
#[must_use]
pub fn kept_from_requests(below: &Path) -> bool {
    below.components().any(|part| {
        let part = part.as_os_str().to_string_lossy();
        part.starts_with('.') || part.eq_ignore_ascii_case("nvs.toml")
    })
}

/// Whether one path component is a Windows device name, read the way Windows reads it:
/// `rule:errors/path-component-refusals`'s list, on every platform.
///
/// Windows reads only the part before the first `.` or `:`, without its trailing spaces, so
/// `CON`, `con.txt`, `NUL.` and `COM1 .log` all name a device in every folder. `CONIN$` and
/// `CONOUT$` are the console's two ends, and `COM¹` to `LPT³` are devices as `COM1` is. A name that
/// only contains one of these, such as `console`, is a name.
///
/// The file doors (`nvs_runtime::capability`), a mount's captured segment and a static file's
/// segment all ask this, so the three cannot disagree about which names are devices.
#[must_use]
pub fn is_reserved_device(name: &str) -> bool {
    let stem = name.split(['.', ':']).next().unwrap_or(name);
    let stem = stem.trim_end_matches(' ').to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    stem.strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|digit| {
            matches!(
                digit,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
}

/// Why one path component names something other than what it spells, on Windows, or `None`:
/// `rule:errors/path-component-refusals`'s trailing dot or space and 8.3 short name, on every
/// platform.
///
/// Windows drops a trailing dot or space when it opens a name, so `a.nvs.` opens `a.nvs`. A short
/// name is a component with `~` and digits before its end or its first `.`, such as `PROGRA~1` or
/// `PROBE~12.NVS`, and it opens whichever long name the volume gave it. `.` and `..` are not names.
#[must_use]
pub fn alias_component(name: &str) -> Option<&'static str> {
    if name == "." || name == ".." {
        return None;
    }
    if name.ends_with(['.', ' ']) {
        return Some("ends in a dot or a space");
    }
    let stem = name.split('.').next().unwrap_or(name);
    let short = stem.rsplit_once('~').is_some_and(|(_, digits)| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    });
    short.then_some("is a short 8.3 name")
}

/// Why the path `text` names something other than what it spells, on Windows, or `None`.
///
/// The path is judged by its text alone, before anything asks the file system about it, on every
/// platform. A path that starts with two separators is a network path (`\\host\share`), a device
/// path (`\\.\pipe\x`) or a verbatim one (`\\?\GLOBALROOT\...`), and opening one connects to the
/// host it names. A `:` after the drive letter names a stream of a file (`a.nvs::$DATA`). Each
/// component is [`alias_component`]. Both `/` and `\` separate, since Windows reads both.
///
/// **Cost:** one scan of the path per door call, and no system call.
#[must_use]
pub fn windows_alias(text: &str) -> Option<&'static str> {
    if network_path(text) {
        return Some("starts with two separators, which is a network or device path");
    }
    let bytes = text.as_bytes();
    let rest = if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        &text[2..]
    } else {
        text
    };
    if rest.contains(':') {
        return Some("has a `:` after the drive letter, which names a stream of a file");
    }
    rest.split(['/', '\\']).find_map(alias_component)
}

/// Whether `text` starts with two separators: a network (`\\host\share`), device (`\\.\`) or
/// verbatim (`\\?\`) path on Windows, in either slash.
fn network_path(text: &str) -> bool {
    text.starts_with(['/', '\\']) && text[1..].starts_with(['/', '\\'])
}

/// The empty parent, spelled as the current directory; every other parent unchanged.
///
/// `Path::new("copy.txt").parent()` is `Some("")`, and an empty path canonicalizes on no platform —
/// so without this the walk in [`resolved`] pins nothing, [`Path::file_name`] of `""` is `None`, and
/// a bare relative name is denied under every grant including `.`. Substituting `.` asks the same
/// canonicalizer the same question the operating system will answer when the path is opened, which
/// keeps the resolution on the one seam § 4 puts it on rather than reading a current directory here.
fn here(parent: &Path) -> &Path {
    if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    }
}
