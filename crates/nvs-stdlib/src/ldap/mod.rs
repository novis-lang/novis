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
//! **A search is an [`Entries`] the program owns** (ADR 0278 § 5): the key of
//! the connection it runs on and an [`nvs_ldap::Cursor`] holding one page. The
//! first page is read by [`search`] itself, so a base that does not exist or a
//! filter the server refuses throws at the call. Every later page is read
//! when the loop reaches it. A size or time limit the server hit throws, and
//! the page it came with is dropped. A connection released with a search still
//! paging is not settled, so it is closed and not pooled. [`read`] is a
//! base-scope search that returns `None` for `noSuchObject`.
//!
//! **A Novis program's search is parked on its connection.** An object slot
//! holds a `Value`, so [`Entries::park`] moves the cursor into [`Held`] under
//! an id, and `Ldap\Entries` carries the connection's key and that id. [`step`]
//! reads the next entry, and a search leaves [`Held`] at its last entry or its
//! first failure. One the program stops reading stays until the request ends,
//! and a pooled connection drops what is left before it is reused. `search.rs`
//! is the Novis half: the helpers for `Ldap\Connection`, `Ldap\Entries`,
//! `Ldap\Entry` and `Ldap\Filter`.
//!
//! **What it spends:** one socket and one TLS session per connection a request
//! holds, and up to the block's `pool.idle` of them per core between requests.
//! A search holds one page of up to [`PAGE_SIZE`] entries by default, per
//! search a request has open, and an `Ldap\Entry` holds its attributes' values
//! for as long as the program keeps it.
//!
//! **Every failure the directory or the wire reports is one `Ldap\LdapError`**
//! (ADR 0278 § 10), built by [`fault_of`]: `$kind` is the [`ERROR_KIND`] case
//! [`nvs_ldap::Kind`] names, and `$code` is the LDAP result code, or `null`
//! where no server sent one. A pool with no free slot is an `IOError` and a
//! malformed URL or DN is the program's own `LogicError` or `RuntimeError`, as
//! for `Core\Db`. No message carries a password.

use std::collections::BTreeMap;
use std::net::{SocketAddr, ToSocketAddrs as _};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nvs_config::Cap;
use nvs_config::capability::Scope;
use nvs_runtime::{Ctx, Fault, Tag, ThrownClass, Value};

mod registry;
mod search;

pub(crate) use self::registry::*;
pub(crate) use self::search::{
    ENTRIES_ADVANCE_SYMBOL, ENTRIES_CURRENT_SYMBOL, ENTRIES_ITERATE_SYMBOL,
};

/// `Core\Ldap::connect`, as its refusals spell it.
pub const CONNECT: &str = r"Core\Ldap::connect";

/// `Core\Ldap::open`, as its refusals spell it.
pub const OPEN: &str = r"Core\Ldap::open";

/// `Core\Ldap\Connection::whoami`, as its refusals spell it.
pub const WHOAMI: &str = r"Core\Ldap\Connection::whoami";

/// `Core\Ldap\Connection::search`, as its refusals spell it.
pub const SEARCH: &str = r"Core\Ldap\Connection::search";

/// `Core\Ldap\Connection::read`, as its refusals spell it.
pub const READ: &str = r"Core\Ldap\Connection::read";

/// `Core\Ldap\Entries`, as a failure reading its next page spells it.
pub const ENTRIES_MEMBER: &str = r"Core\Ldap\Entries";

/// How many entries one page of a search holds when the program does not say.
pub const PAGE_SIZE: u32 = 1000;

/// One open LDAP connection, as a request holds it and the pool keeps it.
#[derive(Debug)]
pub struct Held {
    /// The connection, bound as whoever opened it.
    conn: nvs_ldap::Connection,
    /// How long one operation on it may take.
    timeout: Duration,
    /// The block's `base`, where a search that names none starts, or `None`
    /// for a connection `open` made.
    base: Option<String>,
    /// The searches a program is reading on this connection, by the id its
    /// `Ldap\Entries` carries. A search leaves when its last entry is read.
    searches: BTreeMap<u64, nvs_ldap::Cursor>,
    /// The id the next parked search gets.
    next_search: u64,
}

impl Held {
    /// A connection just opened, with nothing parked on it.
    fn new(conn: nvs_ldap::Connection, timeout: Duration) -> Self {
        Self {
            conn,
            timeout,
            base: None,
            searches: BTreeMap::new(),
            next_search: 0,
        }
    }

    /// The connection, with a fresh deadline filed for the operation about to run.
    fn ready(&mut self) -> &mut nvs_ldap::Connection {
        self.conn.set_deadline(Some(Instant::now() + self.timeout));
        &mut self.conn
    }

    /// The next entry of the search parked under `id`, or `None` after its
    /// last one. A search that ended, or failed, is no longer parked.
    fn advance(&mut self, id: u64) -> Option<Result<nvs_ldap::Entry, nvs_ldap::Error>> {
        let cursor = self.searches.get_mut(&id)?;
        self.conn.set_deadline(Some(Instant::now() + self.timeout));
        let next = cursor.next(&mut self.conn);
        if !matches!(next, Some(Ok(_))) {
            self.searches.remove(&id);
        }
        next
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

/// The `Ldap\LdapError` an LDAP failure becomes, worded for `member`.
#[must_use]
pub fn fault_of(member: &str, error: &nvs_ldap::Error) -> Fault {
    let message = format!("{member}: {}: {}", error.kind().name(), error.message());
    ldap_error(message, error)
}

/// An `Ldap\LdapError` with `message`, carrying `error`'s kind and, where a
/// server sent one, its result code. A code the client found itself leaves
/// `$code` unwritten, and an unwritten slot reads `null`.
fn ldap_error(message: String, error: &nvs_ldap::Error) -> Fault {
    let mut slots = vec![(nvs_runtime::LDAP_KIND_SLOT, error_kind_value(error.kind()))];
    if let Some(code) = error.code() {
        slots.push((nvs_runtime::LDAP_CODE_SLOT, Value::int(i64::from(code))));
    }
    Fault::thrown_with_slots(ThrownClass::LdapError, message, slots)
}

/// An [`nvs_ldap::Kind`] as the [`ERROR_KIND`] case a program matches on,
/// which at runtime is that case's ordinal. The two enums are joined by name,
/// and `every_ldap_kind_names_a_registered_case` makes the `expect` unreachable.
pub(crate) fn error_kind_value(kind: nvs_ldap::Kind) -> Value {
    let (_, ordinal) = ERROR_KIND
        .cases
        .iter()
        .find(|(name, _)| *name == kind.name())
        .expect("every `nvs_ldap::Kind` names a case `ERROR_KIND` registers");
    Value::int(*ordinal)
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
    Ok(Held::new(conn, timeout))
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
/// A `RuntimeError` for a name outside `ldap.connect` and for a name with no
/// block. An `Ldap\LdapError` for a bind the server refused, and for a block
/// whose URLs did not answer, naming the last failure. An `IOError` when the
/// pool's `max` is reached and no slot frees within the block's `timeout`.
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
    let mut held = match pooled {
        Some(warm) => *warm,
        None => walk(ctx, name, block, &settings)?,
    };
    // A search the last request left unread holds no page the server is
    // still sending, or the connection would not have been pooled, and its id
    // belongs to an `Ldap\Entries` that request freed.
    held.searches.clear();
    held.base.clone_from(&block.base);
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
    let message = format!(
        "{CONNECT}: no URL in `[ldap.{name}]` answered, and the last one said: {}",
        error.message()
    );
    Err(ldap_error(message, &error))
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

/// A search a request is reading: the connection it runs on, by key, and the
/// page it holds.
#[derive(Debug)]
pub struct Entries {
    key: u64,
    cursor: nvs_ldap::Cursor,
}

impl Entries {
    /// The next entry, reading the next page when this one is used up, or
    /// `None` after the last one.
    ///
    /// # Errors
    ///
    /// [`held`]'s, a size or time limit the server hit, and every other
    /// failure the server reports.
    pub fn next(&mut self, ctx: &mut Ctx) -> Result<Option<nvs_ldap::Entry>, Fault> {
        if self.cursor.is_finished() && self.cursor.held() == 0 {
            return Ok(None);
        }
        let conn = held(ctx, self.key, ENTRIES_MEMBER)?.ready();
        self.cursor
            .next(conn)
            .transpose()
            .map_err(|error| fault_of(ENTRIES_MEMBER, &error))
    }

    /// The continuation references the search returned so far, none of them followed.
    #[must_use]
    pub fn references(&self) -> &[String] {
        self.cursor.references()
    }

    /// Moves the search onto the connection it runs on and returns the id
    /// [`step`] reads it back by, which is what an `Ldap\Entries` carries:
    /// an object slot holds a `Value`, and a cursor is not one.
    ///
    /// # Errors
    ///
    /// [`held`]'s.
    pub fn park(self, ctx: &mut Ctx) -> Result<u64, Fault> {
        let held = held(ctx, self.key, SEARCH)?;
        let id = held.next_search;
        held.next_search += 1;
        held.searches.insert(id, self.cursor);
        Ok(id)
    }
}

/// The next entry of the search [`Entries::park`] parked under `id` on the
/// connection held under `key`, or `None` after its last one.
///
/// # Errors
///
/// [`held`]'s, a size or time limit the server hit, and every other failure
/// the server reports.
pub fn step(ctx: &mut Ctx, key: u64, id: u64) -> Result<Option<nvs_ldap::Entry>, Fault> {
    held(ctx, key, ENTRIES_MEMBER)?
        .advance(id)
        .transpose()
        .map_err(|error| fault_of(ENTRIES_MEMBER, &error))
}

/// The `base` of the block the connection held under `key` was opened from,
/// or `None` for a connection `open` made.
///
/// # Errors
///
/// [`held`]'s.
pub fn base_of(ctx: &mut Ctx, key: u64) -> Result<Option<String>, Fault> {
    Ok(held(ctx, key, SEARCH)?.base.clone())
}

/// `Core\Ldap\Connection::search`'s body: starts `request` on the connection
/// held under `key` and reads its first page.
///
/// # Errors
///
/// [`held`]'s, and every failure the server reports for the first page.
pub fn search(
    ctx: &mut Ctx,
    key: u64,
    request: &nvs_ldap::SearchRequest<'_>,
) -> Result<Entries, Fault> {
    let mut cursor = nvs_ldap::Cursor::new(request);
    cursor
        .start(held(ctx, key, SEARCH)?.ready())
        .map_err(|error| fault_of(SEARCH, &error))?;
    Ok(Entries { key, cursor })
}

/// `Core\Ldap\Connection::read`'s body: the entry at `dn` with `attributes`,
/// or `None` when no entry is there.
///
/// # Errors
///
/// [`held`]'s, and every failure the server reports but `noSuchObject`.
pub fn read(
    ctx: &mut Ctx,
    key: u64,
    dn: &str,
    attributes: &[&str],
) -> Result<Option<nvs_ldap::Entry>, Fault> {
    let every = nvs_ldap::Filter::Present("objectClass".to_owned());
    let mut cursor = nvs_ldap::Cursor::new(&nvs_ldap::SearchRequest {
        base: dn,
        scope: nvs_ldap::Scope::Base,
        filter: &every,
        attributes,
        page_size: 1,
        size_limit: 0,
        time_limit: 0,
    });
    let conn = held(ctx, key, READ)?.ready();
    let mut found = None;
    // A base search has one entry at most, and the loop runs to the end so
    // the connection is settled when it returns.
    while let Some(next) = cursor.next(conn) {
        match next {
            Ok(entry) => found = found.or(Some(entry)),
            Err(error) if error.kind() == nvs_ldap::Kind::NoSuchObject => return Ok(None),
            Err(error) => return Err(fault_of(READ, &error)),
        }
    }
    Ok(found)
}

/// A [`CONNECTION`] over the connection held under `key`.
fn connection(key: u64) -> Value {
    crate::instance::build(&CONNECTION, [Value::uint(key)])
}

/// An optional text field of `Ldap\Settings`, or `None` for the `null` an
/// omitting call site passed.
fn settings_text<'a>(args: &'a [Value], at: usize, key: &str) -> Result<Option<&'a str>, Fault> {
    if matches!(args[at].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    args[at].as_text().map(Some).ok_or_else(|| {
        Fault::fatal(format!(
            "{OPEN} expected a `string` for `{key}`, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// `Ldap\Settings.tls`, where an absent value is `Tls::Required`.
fn settings_tls(value: &Value) -> Result<nvs_config::ldap::Tls, Fault> {
    match value.as_int() {
        None | Some(0) => Ok(nvs_config::ldap::Tls::Required),
        Some(1) => Ok(nvs_config::ldap::Tls::None),
        Some(_) => Err(Fault::fatal(format!(
            "{OPEN} expected a `{TLS_NAME}` case for `tls`, got tag {}",
            value.tag_byte()
        ))),
    }
}

/// `Ldap\Settings.timeout`, or `None` where the program left it out.
fn settings_timeout(args: &[Value]) -> Result<Option<Duration>, Fault> {
    if matches!(args[TIMEOUT_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let nanos = crate::time::nanos_of(args, TIMEOUT_ARG, "timeout")?;
    if nanos <= 0 {
        return Err(Fault::thrown(format!(
            "{OPEN}: `timeout` must be a positive duration, and this one is {nanos}ns"
        )));
    }
    Ok(Some(Duration::from_nanos(nanos.unsigned_abs())))
}

nvs_runtime::nvs_helper! {
    /// `Core\Ldap::connect(string $name): Ldap\Connection` — [`connect`] for a
    /// Novis program.
    fn nvs_core_ldap_connect(ctx, args: [1]) {
        // Unreachable from source: the parameter is a `string` in `CLASS`.
        let name = args[0]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{CONNECT} expected a `string` name, got tag {}",
                    args[0].tag_byte()
                ))
            })?
            .to_owned();
        let key = connect(ctx, &name)?;
        Ok(connection(key))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Ldap::open(Ldap\Settings $settings): Ldap\Connection` — [`open`]
    /// for a Novis program, over the five slots [`SETTINGS`] flattens to.
    fn nvs_core_ldap_open(ctx, args: [5]) {
        // Unreachable from source: `url` is a required `string` field.
        let url = settings_text(args, URL_ARG, "url")?.unwrap_or_default();
        let user = settings_text(args, USER_ARG, "user")?;
        let password = settings_text(args, PASSWORD_ARG, "password")?.unwrap_or_default();
        let settings = Settings {
            url,
            user,
            password: password.as_bytes(),
            tls: settings_tls(&args[TLS_ARG])?,
            tls_ca_file: None,
            timeout: settings_timeout(args)?,
        };
        let key = open(ctx, &settings)?;
        Ok(connection(key))
    }
}

/// The address of one of this module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_ldap_connect" => (nvs_core_ldap_connect as *const ()).cast(),
        "nvs_core_ldap_open" => (nvs_core_ldap_open as *const ()).cast(),
        _ => return search::address(symbol),
    })
}
