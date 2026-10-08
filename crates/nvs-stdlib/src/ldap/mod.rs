//! `Core\Ldap`'s connections: a `[ldap.<name>]` block opened under `ldap.connect`, a program's settings opened under `ldap.open`, and the pool a block's connections return to
//!
//! ADR 0278 is the design, and `crates/nvs-ldap` is the wire this module drives.
//! What lives here is the half that reads configuration and grants, which that
//! crate never does: its [`nvs_ldap::Endpoint`] arrives with the address
//! already resolved and the cleartext answer already given.
//!
//! **[`connect`] and [`open`] differ in who wrote the address**, as
//! `rule:core-classes/db-capabilities` splits `Core\Db`'s two. A block's URLs
//! are the operator's, granted by name under `ldap.connect`, and resolved with
//! no address policy in front of them. A settings URL is the program's: its host
//! is asked of `ldap.open` and its address of `rule:security/net-address-policy`
//! before anything is dialled. Both ask `ldap.cleartext` about the host when
//! `tls` is `"none"`, and [`nvs_ldap::Connection::open`] refuses the dial without
//! that answer.
//!
//! **A block's URL list fails over in order** (ADR 0278 § 2), and so does every
//! address one URL's host resolves to. A server that does not answer, or whose
//! TLS handshake fails, moves the walk to the next address or URL. A
//! server that answers and refuses the bind stops it, because the next server
//! would refuse the same credentials and the walk would only add to the
//! account's lockout count.
//!
//! **A block's connection is pooled and stays bound as the block**
//! (`rule:security/ldap-pool-is-bound-as-its-block`). Nothing here rebinds a
//! connection, so the identity a pooled one carries is the block's `user` for
//! its whole life. LDAP keeps no other session state a program can set, so the
//! release gate is [`nvs_ldap::Connection::is_settled`]: a connection with an
//! operation still outstanding is closed at teardown rather than pooled. The
//! pool key is `rule:security/db-pool-reset-is-a-boundary`'s generation-scoped
//! block key under an `ldap:` prefix, and a pooled entry is downcast to
//! [`Held`] before it is reused, so a `Core\Db` connection can never come back
//! from an LDAP key. An `open` connection is not pooled and closes with the
//! request.
//!
//! **What it spends:** one socket and one TLS session per connection a request
//! holds, and up to the block's `pool.idle` of them per core between requests.
//!
//! A failure throws a `RuntimeError` naming the member and ADR 0278 § 10's kind,
//! except [`nvs_ldap::Kind::Unavailable`] (`IOError`) and
//! [`nvs_ldap::Kind::Timeout`] (`TimeoutError`). No message carries a password.

use std::collections::BTreeMap;
use std::net::{SocketAddr, ToSocketAddrs as _};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nvs_config::Cap;
use nvs_config::capability::Scope;
use nvs_runtime::{Ctx, Fault, ThrownClass};

/// `Core\Ldap::connect`, as its refusals spell it.
pub const CONNECT: &str = r"Core\Ldap::connect";

/// `Core\Ldap::open`, as its refusals spell it.
pub const OPEN: &str = r"Core\Ldap::open";

/// `Core\Ldap\Connection::whoami`, as its refusals spell it.
pub const WHOAMI: &str = r"Core\Ldap\Connection::whoami";

/// One open LDAP connection, as a request holds it and the pool keeps it.
#[derive(Debug)]
pub struct Held {
    /// The connection, bound as whoever opened it.
    conn: nvs_ldap::Connection,
    /// How long one operation on it may take.
    timeout: Duration,
}

impl Held {
    /// The connection, with a fresh deadline filed for the operation about to run.
    fn ready(&mut self) -> &mut nvs_ldap::Connection {
        self.conn.set_deadline(Some(Instant::now() + self.timeout));
        &mut self.conn
    }
}

impl nvs_runtime::HeldConnection for Held {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    /// `rule:security/ldap-pool-is-bound-as-its-block`'s release gate: every
    /// operation started on the connection has finished.
    fn is_poolable(&self) -> bool {
        self.conn.is_settled()
    }
}

/// What `Core\Ldap::open` dials: the fields of `Ldap\Settings` that decide the
/// connection, already read out of the program's value.
#[derive(Clone, Copy, Debug)]
pub struct Settings<'a> {
    /// One `ldap://` or `ldaps://` URL.
    pub url: &'a str,
    /// The bind name, or `None` for an anonymous session that never binds.
    pub user: Option<&'a str>,
    /// The bind password. A `user` with an empty one is refused before anything is sent.
    pub password: &'a [u8],
    /// `Required`, or `None` for a host on `[capabilities.ldap] cleartext`.
    pub tls: nvs_config::ldap::Tls,
    /// The trust anchor the server's certificate is checked against, or the default set.
    pub tls_ca_file: Option<&'a Path>,
    /// How long one operation may take, or `None` for [`nvs_config::ldap::DEFAULT_TIMEOUT`].
    pub timeout: Option<Duration>,
}

/// The throw an LDAP failure becomes, worded for `member`.
#[must_use]
pub fn fault_of(member: &str, error: &nvs_ldap::Error) -> Fault {
    let kind = error.kind();
    let message = format!("{member}: {}: {}", kind.name(), error.message());
    match kind {
        nvs_ldap::Kind::Unavailable => Fault::thrown_as(ThrownClass::Io, message),
        nvs_ldap::Kind::Timeout => Fault::thrown_as(ThrownClass::Timeout, message),
        _ => Fault::thrown(message),
    }
}

/// Whether `error` is the server refusing the credentials, which stops a
/// block's URL walk rather than moving it on.
fn refused_the_bind(error: &nvs_ldap::Error) -> bool {
    !matches!(
        error.kind(),
        nvs_ldap::Kind::Unavailable | nvs_ldap::Kind::Timeout
    )
}

/// The block's `tls` as the wire crate spells it.
fn wire_tls(tls: nvs_config::ldap::Tls) -> nvs_ldap::Tls {
    match tls {
        nvs_config::ldap::Tls::Required => nvs_ldap::Tls::Required,
        nvs_config::ldap::Tls::None => nvs_ldap::Tls::None,
    }
}

/// Whether `ldap.cleartext` grants `host`, asked only where the connection says `tls = "none"`.
fn cleartext_granted(ctx: &Ctx, tls: nvs_config::ldap::Tls, host: &str) -> bool {
    tls == nvs_config::ldap::Tls::None
        && nvs_runtime::capability::granted(ctx, Cap::LdapCleartext, Scope::Host(host))
}

/// Dials `url` at `address` and binds as `user`, or not at all for an anonymous session.
fn dial(
    url: &nvs_ldap::Url,
    address: SocketAddr,
    settings: &Settings<'_>,
    cleartext: bool,
) -> Result<Held, nvs_ldap::Error> {
    let timeout = settings
        .timeout
        .unwrap_or(nvs_config::ldap::DEFAULT_TIMEOUT);
    let mut conn = nvs_ldap::Connection::open(&nvs_ldap::Endpoint {
        url,
        address,
        tls: wire_tls(settings.tls),
        ca_file: settings.tls_ca_file,
        cleartext_granted: cleartext,
        deadline: Some(Instant::now() + timeout),
    })?;
    if let Some(user) = settings.user {
        conn.bind(user, settings.password)?;
    }
    Ok(Held { conn, timeout })
}

/// `Core\Ldap::connect`'s body: opens `[ldap.<name>]` for this request, or
/// returns the connection this request already holds under that name, and
/// returns the key it is held under.
///
/// The grant is asked before the configuration is read, so a program with no
/// grant learns nothing about which blocks exist. The block is read from the
/// boot snapshot, which no program can write to.
///
/// # Errors
///
/// A `RuntimeError` for a name outside `ldap.connect`, for a name with no block,
/// and for a bind the server refused. An `IOError` when no URL in the block
/// answered, naming the last failure, and when the pool's `max` is reached and
/// no slot frees within `acquire`.
pub fn connect(ctx: &mut Ctx, name: &str) -> Result<u64, Fault> {
    nvs_runtime::capability::require(ctx, Cap::LdapConnect, Scope::Name(name), CONNECT)?;
    let memo = format!("ldap:{name}");
    if let Some(key) = ctx.memoized_connection(&memo) {
        return Ok(key);
    }

    let snapshot = ctx
        .config()
        .map(|config| Arc::clone(config.snapshot()))
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{CONNECT}: this program is running with no configuration, so there is no \
                 `[ldap.{name}]` block to open"
            ))
        })?;
    let block = snapshot.config.ldap.get(name).ok_or_else(|| {
        Fault::thrown(format!(
            "{CONNECT}: nothing sets `[ldap.{name}]` up, so the name resolves to no block"
        ))
    })?;
    // `nvs_config::ldap::validate` accepted these at boot, so the fallbacks
    // below are never taken: a block that cannot be read is closed with the
    // request rather than pooled.
    let none = BTreeMap::new();
    let timeout = nvs_config::ldap::timeout_for(name, block, &none)
        .unwrap_or(nvs_config::ldap::DEFAULT_TIMEOUT);
    let bounds =
        nvs_config::ldap::pool_for(name, block, &none).unwrap_or(nvs_config::db::PoolBounds::OFF);
    let settings = Settings {
        url: "",
        user: block.user.as_deref(),
        password: block.password.as_deref().unwrap_or_default().as_bytes(),
        tls: nvs_config::ldap::tls_of(block),
        tls_ca_file: block.tls_ca_file.as_deref().map(Path::new),
        timeout: Some(timeout),
    };

    let ticket = nvs_runtime::pool::Ticket::for_block(&snapshot, &memo, bounds);
    let max = bounds.max;
    let full = |waited: &str| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{CONNECT}: `[ldap.{name}]` already holds its `max` of {max} connections on \
                 this core, and {waited}. Raise `[ldap.{name}.pool] max`, or hold fewer \
                 connections open at once"
            ),
        )
    };
    let lease = match nvs_runtime::pool::admit(ticket.clone()) {
        Some(lease) => lease,
        None => crate::db::pool::wait_for_slot(ctx, ticket, &full, Some(Instant::now() + timeout))?,
    };
    let pooled = if bounds.enabled {
        nvs_runtime::pool::take(&lease, Instant::now())
            .and_then(|warm| warm.into_any().downcast::<Held>().ok())
    } else {
        None
    };
    let held = match pooled {
        Some(warm) => *warm,
        None => walk(ctx, name, block, &settings)?,
    };
    Ok(ctx.hold_open_connection(Some(memo), Some(lease), Box::new(held)))
}

/// Tries the block's URLs in order and returns the first connection that opened and bound.
fn walk(
    ctx: &Ctx,
    name: &str,
    block: &nvs_config::tree::LdapDirectory,
    settings: &Settings<'_>,
) -> Result<Held, Fault> {
    let mut last = None;
    for written in nvs_config::ldap::urls_of(block) {
        let Some(url) = nvs_ldap::Url::parse(written) else {
            return Err(Fault::thrown(format!(
                "{CONNECT}: `[ldap.{name}] url` names `{written}`, which is not an `ldap://` \
                 or `ldaps://` URL"
            )));
        };
        // The operator wrote this host, so every address it resolves to is
        // dialled in turn with no address policy in front of it. `localhost`
        // resolves to `::1` and `127.0.0.1`, and a server may listen on one.
        let addresses: Vec<SocketAddr> = (url.host.as_str(), url.port)
            .to_socket_addrs()
            .map(Iterator::collect)
            .unwrap_or_default();
        if addresses.is_empty() {
            last = Some(nvs_ldap::Error::new(
                nvs_ldap::Kind::Unavailable,
                format!("`{}` resolves to no address", url.host),
            ));
        }
        let cleartext = cleartext_granted(ctx, settings.tls, &url.host);
        for address in addresses {
            match dial(&url, address, settings, cleartext) {
                Ok(held) => return Ok(held),
                Err(error) if refused_the_bind(&error) => return Err(fault_of(CONNECT, &error)),
                Err(error) => last = Some(error),
            }
        }
    }
    let error = last.unwrap_or_else(|| {
        nvs_ldap::Error::new(nvs_ldap::Kind::Unavailable, "the block names no URL")
    });
    Err(Fault::thrown_as(
        ThrownClass::Io,
        format!(
            "{CONNECT}: no URL in `[ldap.{name}]` answered, and the last one said: {}",
            error.message()
        ),
    ))
}

/// `Core\Ldap::open`'s body: dials the one URL `settings` names and returns the
/// key the connection is held under for the rest of the request.
///
/// The host is asked of `ldap.open` before anything is resolved, and the
/// address it resolves to is asked of `rule:security/net-address-policy`. The
/// socket opens to exactly that address.
///
/// # Errors
///
/// A `RuntimeError` for a URL that does not parse, a host outside `ldap.open`,
/// an address the policy denies, and every refusal the server makes. An
/// `IOError` for a server that does not answer.
pub fn open(ctx: &mut Ctx, settings: &Settings<'_>) -> Result<u64, Fault> {
    let url = nvs_ldap::Url::parse(settings.url).ok_or_else(|| {
        Fault::thrown(format!(
            "{OPEN}: `url` is not an `ldap://` or `ldaps://` URL"
        ))
    })?;
    nvs_runtime::capability::require(ctx, Cap::LdapOpen, Scope::Host(&url.host), OPEN)?;
    let pinned = nvs_runtime::capability::pinned_address(ctx, &url.host, OPEN)?;
    let cleartext = cleartext_granted(ctx, settings.tls, &url.host);
    let held = dial(&url, SocketAddr::new(pinned, url.port), settings, cleartext)
        .map_err(|error| fault_of(OPEN, &error))?;
    Ok(ctx.hold_open_connection(None, None, Box::new(held)))
}

/// The connection a request holds under `key`.
///
/// # Errors
///
/// A `LogicError` for a key that names no open LDAP connection, which is a
/// connection already closed.
pub fn held<'c>(ctx: &'c mut Ctx, key: u64, member: &str) -> Result<&'c mut Held, Fault> {
    ctx.open_connection_mut(key)
        .and_then(|conn| conn.as_any_mut().downcast_mut::<Held>())
        .ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("{member}: this connection is closed"),
            )
        })
}

/// `Core\Ldap\Connection::whoami`'s body: the identity the server says the
/// connection is bound as, in RFC 4532's `dn:` or `u:` form, and empty for an
/// anonymous session.
///
/// # Errors
///
/// [`held`]'s, and every failure the server reports.
pub fn who_am_i(ctx: &mut Ctx, key: u64) -> Result<String, Fault> {
    held(ctx, key, WHOAMI)?
        .ready()
        .who_am_i()
        .map_err(|error| fault_of(WHOAMI, &error))
}
