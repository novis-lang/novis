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
//!   at all, including a host literally named `*`, and so does `*.` with nothing after it.
//! - **The wildcard is `db.open`'s alone**, which is [`Cap::takes_host_wildcard`]. `net.connect`'s
//!   grant is asked of a *name* and then `rule:http-server/allow-url-pins-the-address`
//!   's door pins the address that name resolved to; a pattern there would widen the set of
//!   names an attacker-influenced argument may reach without the operator having written any one of
//!   them down, which is the whole thing that ADR refuses. `db.open`'s targets are program-supplied
//!   too, but a tenant-per-subdomain deployment cannot enumerate them, and its blast radius is one
//!   operator-named zone rather than the internet.
//! - **A wildcard never crosses a label**, so a suffix match inside a label — the
//!   `evil-tenants.internal` case, which is the whole reason this is not `ends_with` — is refused.
//!   Registering that name is the cheapest attack there is against a suffix check.
//!

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::resolve::Files;
use crate::tree::{Capabilities, Setting};

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
    /// `debug.trace` — where a trace may be written (`rule:testing/debug-probes`).
    DebugTrace,
    /// `debug.profile` — where a profile may be written (`rule:testing/debug-probes`).
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
    /// `mail.send` — which `[mail.<name>]` blocks a program may send through (`rule:programs/framework-core-half`).
    ///
    /// Named by block and never by host, which is [`DbConnect`](Self::DbConnect)'s shape and ADR
    /// 0067 § 3's reasoning: the endpoint an operator wrote into root-owned configuration carries
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
            } else {
                None
            }
        }
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            if v6.is_loopback() {
                Some("loopback (::1)")
            } else if v6.is_unspecified() {
                Some("unspecified (::)")
            } else if first & 0xffc0 == 0xfe80 {
                Some("link-local (fe80::/10)")
            } else if first & 0xfe00 == 0xfc00 {
                Some("private (fc00::/7)")
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

/// An IPv6 address that carries an IPv4 address in its low 32 bits, as that IPv4 address; every
/// other address as [`unmapped`] reads it.
///
/// The table asks this rather than [`unmapped`] because two more prefixes route to the address they
/// embed: the IPv4-compatible `::a.b.c.d` (`::/96`, apart from `::` and `::1`, which are addresses of
/// their own), and the NAT64 well-known prefix `64:ff9b::/96`, which a translating gateway turns
/// into a connection to the IPv4 address. Either spelling of `127.0.0.1` reaches whatever that
/// address reaches, so each is denied exactly when the address it carries is.
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
    let compatible = s[..6] == [0; 6] && !v6.is_loopback() && !v6.is_unspecified();
    let nat64 = s[..6] == [0x64, 0xff9b, 0, 0, 0, 0];
    if compatible || nat64 {
        let [.., a, b, c, d] = v6.octets();
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
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
    /// `db.open` alone. The knowledge lives here rather than in `host_granted` for the reason the
    /// module doc gives about [`name`](Self::name): a capability's properties are arms of this type,
    /// never a string compared in a second place.
    #[must_use]
    pub const fn takes_host_wildcard(self) -> bool {
        matches!(self, Self::DbOpen)
    }

    /// Whether `true` is a spelling this capability's grant has at all — every capability's but the
    /// six that name hosts a weakening applies to
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
            Self::MailSend => caps.mail.as_mut()?.send.as_mut(),
            Self::CacheShared => caps.cache.as_mut()?.shared.as_mut(),
            Self::QueuePurge => caps.queue.as_mut()?.purge.as_mut(),
        }
    }
}

/// What a [`Setting`] means when it is read as a grant.
enum Grant<'a> {
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
                let Some(path) = resolved(path, files) else {
                    return false;
                };
                list.iter().any(|root| path.starts_with(Path::new(root)))
            }
        }
    }

    /// [`allows`](Self::allows) for a host, and the one form of the question a caller with no
    /// filesystem in front of it can ask.
    ///
    /// [`Scope::Host`] never reaches [`Files`] — a hostname is matched against the grant list and
    /// nothing is canonicalized — so demanding one is demanding a parameter the answer does not
    /// depend on. The compiler is the caller that has none:
    /// `rule:core-classes/db-literal-query-checking` has `nvs check` refuse a **literal**
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
    /// This is [`denied_by_default`] plus the operator's half of § 3 — `net.internal`, the addresses
    /// a deployment says it reaches anyway. Its shape, each part of it a refusal to widen further
    /// than the ADR does:
    ///
    /// - **An entry is an IP address literal**, and one that does not parse as an address matches
    ///   nothing. A hostname there would be read before resolution and so would exempt whatever the
    ///   name resolved to *afterwards*, which is the rebinding gap § 2's pinning closes.
    /// - **No ranges.** An operator writing `10.0.0.0/8` would hand back most of the table without
    ///   naming a single host it meant, and prefix matching here would be a second address-matching
    ///   implementation beside the table it is meant to except from.
    /// - **`true` grants nothing**, which is the one place a `Setting::Bool(true)` does not mean
    ///   everything. Turning the whole table off is not an exception, and the value of the exception
    ///   is that a reviewer can see which address a deployment bought back.
    ///
    /// It is not a widening on its own: the host still has to be inside [`Cap::NetConnect`]'s grant,
    /// asked separately and first by `nvs_runtime::capability::pin_host`.
    #[must_use]
    pub fn address_refused(&self, address: std::net::IpAddr) -> Option<&'static str> {
        let range = denied_by_default(address)?;
        let Some(setting) = self.net.as_ref().and_then(|net| net.internal.as_ref()) else {
            return Some(range);
        };
        let Grant::These(named) = grant_of(setting) else {
            return Some(range);
        };
        let wanted = unmapped(address);
        let excepted = named
            .iter()
            .filter_map(|entry| entry.parse::<std::net::IpAddr>().ok())
            .any(|entry| unmapped(entry) == wanted);
        (!excepted).then_some(range)
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

fn canonical_root(root: &mut String, files: &dyn Files) {
    if let Some(found) = resolved(Path::new(root.as_str()), files) {
        *root = found.to_string_lossy().into_owned();
    }
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
#[must_use]
pub fn resolved(path: &Path, files: &dyn Files) -> Option<PathBuf> {
    if let Ok(found) = files.canonical(path) {
        return Some(found);
    }
    let mut tail: Vec<&OsStr> = Vec::new();
    let mut cursor = path;
    loop {
        let name = cursor.file_name()?;
        let parent = here(cursor.parent()?);
        tail.push(name);
        if let Ok(base) = files.canonical(parent) {
            let mut resolved = base;
            resolved.extend(tail.iter().rev());
            return Some(resolved);
        }
        cursor = parent;
    }
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
