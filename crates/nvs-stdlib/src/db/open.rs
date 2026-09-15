//! Getting a connection: `Core\Db::connect`'s configured block,
//! `Core\Db::open`'s twelve settings, and the two members that need no
//! connection at all.
//!
//! [ADR 0067 § 3](/docs/decisions/0067.md) splits the first two on
//! who wrote the endpoint. A block is the operator's word and is taken as one;
//! arguments are the program's, so `open` re-checks the address they resolve to
//! against the policy in front of it. `inList` and `quoteIdentifier` are here
//! because they are what `Core\Db` can answer before anything is open, and
//! [`mod@super`]'s doc owns what each can honestly promise.

use super::*;

/// `Core\Db::connect`, as its own refusals spell it.
pub(super) const CONNECT: &str = r"Core\Db::connect";

/// `Core\Db::open`, as its own refusals spell it — including the two `rule:core-classes/db-capabilities` gives it and gives `connect` no equivalent of.
pub(super) const OPEN: &str = r"Core\Db::open";

/// `Core\Db\Connection::query`, as its own refusals spell it.
pub(super) const QUERY: &str = r"Core\Db\Connection::query";

/// `Core\Db\Connection::queryAs`, as its own refusals spell it — under
/// [``rule:classes/no-traits``](/docs/decisions/0043.md)'s
/// delegation, a call through a `Core\Db\Transaction` names the connection's
/// member here exactly as [`QUERY`] does.
pub(super) const QUERY_AS: &str = r"Core\Db\Connection::queryAs";

/// `Core\Db\Connection::execute`, as its own refusals spell it. Both halves are
/// passed together to everything on the statement path — the short name for
/// [`connection_of`], which builds `Class::member` itself, and this one for the
/// messages that already hold a class.
pub(super) const EXECUTE: &str = r"Core\Db\Connection::execute";

/// `Core\Db\Connection::executeMany`, as its own refusals spell it. See
/// [`EXECUTE`] for why both spellings travel together.
pub(super) const EXECUTE_MANY: &str = r"Core\Db\Connection::executeMany";

/// `Core\Db\Connection::driver`, as the one refusal it has spells it: the
/// `LogicError` [`crate::db::pool::filed_connection`] raises for a connection
/// spec § 18's `close` has already released. `close` and `isOpen` need no
/// spelling of their own — neither reaches for the connection, so neither has
/// a message that holds a class.
pub(super) const DRIVER_MEMBER: &str = r"Core\Db\Connection::driver";

/// `Core\Db\Connection::serverVersion`, for the same refusal's sentence.
pub(super) const SERVER_VERSION_MEMBER: &str = r"Core\Db\Connection::serverVersion";

/// `transaction`'s own name for a refusal, spelled on the connection because
/// that is the class that declares the row — a nested call on a
/// [`TRANSACTION`] reaches the same helper and so names the same member, which
/// is what delegating rather than re-declaring means.
pub(super) const TRANSACTION_MEMBER: &str = r"Core\Db\Connection::transaction";

/// `rollBack`'s, which is the one member of that class with a body of its own.
pub(super) const ROLL_BACK: &str = r"Core\Db\Transaction::rollBack";

/// The ABI slot each of `connect`'s two options arrives in — the row's one
/// positional parameter, then the bag flattened in declaration order.
pub(super) const SHARED_ARG: usize = 1;
/// See [`SHARED_ARG`].
pub(super) const TIMEOUT_ARG: usize = 2;

/// The ABI slot each of `transaction`'s options arrives in — the receiver, then
/// the row's one positional parameter, then [`TRANSACTION_OPTIONS`] flattened
/// in declaration order.
///
pub(super) const ISOLATION_ARG: usize = 2;
/// See [`ISOLATION_ARG`].
pub(super) const READ_ONLY_ARG: usize = 3;
/// See [`ISOLATION_ARG`].
pub(super) const RETRIES_ARG: usize = 4;

/// The instant the handshake must be done by, or `None` for a call that named
/// no `timeout`.
///
/// # Errors
///
/// A thrown `RuntimeError` for a duration that is zero or negative, on
/// `Core\Http`'s reading: `rule:http-server/no-spelling-for-an-unbounded-wait` has no spelling for an unbounded wait,
/// and a zero one is that spelling said quietly. A [`Fault::fatal`] for a slot
/// that is neither a `Duration` nor `Tag::Null`, which the row's type rules out.
pub(super) fn deadline_of(args: &[Value]) -> Result<Option<std::time::Instant>, Fault> {
    if matches!(args[TIMEOUT_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let nanos = crate::time::nanos_of(args, TIMEOUT_ARG, "timeout")?;
    if nanos <= 0 {
        return Err(Fault::thrown(format!(
            "{CONNECT}: `timeout` must be a positive duration, and this one is {nanos}ns"
        )));
    }
    Ok(Some(
        std::time::Instant::now() + std::time::Duration::from_nanos(nanos.unsigned_abs()),
    ))
}

/// Where a block's `host` and `port` are, as one address.
///
/// **Pinned here and asked nothing else**, which is `rule:core-classes/db-capabilities`: the endpoint
/// was written into root-owned configuration by the same authority that granted
/// `db.connect`, so it is pre-approved and is *not* additionally checked against
/// `rule:security/net-address-policy`'s denied
/// ranges — where every database on a container network or a `10/8` estate
/// lives. `Core\Db::open`'s host is program-supplied and stays subject to that
/// policy in full, which is the whole difference between the two members.
///
/// The name is resolved once and the resolved address is what the socket is
/// opened to, so nothing re-resolves between the check and the connection. What
/// the certificate is checked against stays the written host, which is the
/// target's own `host` and not this.
///
/// `default_port` is the driver's, and it is a parameter because 5432 and 3306
/// are different servers: the caller has already decided which handshake goes
/// out, and it is the only one that knows what a block writing no `port` meant.
///
/// # Errors
///
/// A thrown `IOError` for a host that resolves to nothing.
pub(super) fn address_of(
    host: &str,
    port: Option<u16>,
    default_port: u16,
    name: &str,
) -> Result<SocketAddr, Fault> {
    let port = port.unwrap_or(default_port);
    let bare = host
        .strip_prefix('[')
        .and_then(|held| held.strip_suffix(']'))
        .unwrap_or(host);
    if let Ok(literal) = bare.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(literal, port));
    }
    (host, port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut found| found.next())
        .ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{CONNECT}: `[db.{name}]` names the host `{host}`, which resolves to no \
                         address"
                ),
            )
        })
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::connect(string $name, {shared?: bool, timeout?: Duration}): Db\Connection`
    /// — `rule:core-classes/db-connection-is-named`'s named connection, memoized for the request.
    ///
    /// **The grant is asked first**, before the configuration is read at all,
    /// which is `Core\Mail::send`'s ordering and is the same property: a
    /// program with no grant learns nothing about which blocks a deployment
    /// wrote. A name outside the grant and a name with no block behind it are
    /// two different sentences, and only the second is reachable by a program
    /// the operator already trusted with that name.
    ///
    /// **The block is read from the boot snapshot and never through
    /// `Request::get`.** Every other reader of a `[…]` block in this crate goes
    /// through the per-request view, because a directive an operator marked
    /// `Runtime` can be moved by `Core\Config::set`; a credential is not one of
    /// those, and reading one through a table a program can write to would let
    /// a request choose the server its own query is answered by.
    ///
    /// **What it spends:** one connection — a socket, a TLS session and § 1's
    /// statement cache — per distinct name a request opens, held by the request
    /// and closed with it. A second `connect("main")` spends nothing at all,
    /// which is what § 2's memoization is for; `{shared: false}` opts out of
    /// that and is charged again.
    fn nvs_core_db_connect(ctx, args: [3]) {
        // Unreachable from source: parameter 0 is a `string` in `CLASS` above,
        // so a non-text argument is refused at `E0401` first — the same
        // judgement `quoteIdentifier`'s guard states. Owned, because the
        // capability check and the table below both want `ctx` back.
        let name = args[0]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Db::connect expected a `string` name, got tag {}",
                    args[0].tag_byte()
                ))
            })?
            .to_owned();
        // Unreachable from source for the same reason: the option is declared
        // `bool` and defaults to one, so the slot is never anything else.
        let shared = args[SHARED_ARG].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Db::connect expected a `bool` for `shared`, got tag {}",
                args[SHARED_ARG].tag_byte()
            ))
        })?;
        let deadline = deadline_of(args)?;

        nvs_runtime::capability::require(
            ctx,
            nvs_config::Cap::DbConnect,
            nvs_config::capability::Scope::Name(&name),
            CONNECT,
        )?;

        let key = open_named(ctx, &name, shared, deadline, CONNECT)?;
        let block = Value::str(NvsStr::new(name.as_bytes()));
        Ok(crate::instance::build(&CONNECTION, [Value::uint(key), block]))
    }
}

/// Opens `[db.<name>]` for this request and files it, answering the key it is filed
/// under — `Core\Db::connect`'s body from the memo down, and every `rule:security/db-pool-reset-is-a-boundary`
/// decision with it.
///
/// **Separate from the member because it has a second caller, and that caller is a
/// property rather than a convenience.** [`crate::queue`]'s `push` has to run on the
/// connection this request already holds under this name, because `rule:concurrency/enqueue-commits-with-your-write`'s
/// transactional enqueue *is* "the same connection". A second implementation would be
/// a second pool key, a second memo and a second reset — three places for § 13's
/// bounds to drift apart, and one silent way to lose § 3.
///
/// **The capability check is deliberately not in here.** `connect` asks `db.connect`
/// about a name the *program* wrote, which is what stops a request choosing its own
/// database; a queue's name comes out of root-owned configuration and no program can
/// influence it. Moving the check in would demand a grant for a name nobody chose —
/// [`crate::queue`]'s module doc is where that reading lives.
///
/// `named` is the member a refusal is worded for, so one failure reads as
/// `Core\Db::connect` or as `Core\Queue::push` depending on who asked.
///
/// # Errors
///
/// A thrown `RuntimeError` for a name no `[db.<name>]` block covers, or a block that
/// cannot be read as a connection. An `IOError` for a host that does not resolve, or
/// a connection, TLS handshake or login that failed. A `Db\DbError` for a pool whose
/// `acquire` bound expired with no slot free.
pub(crate) fn open_named(
    ctx: &mut nvs_runtime::Ctx,
    name: &str,
    shared: bool,
    deadline: Option<std::time::Instant>,
    named: &str,
) -> Result<u64, Fault> {
    let held = if shared {
        ctx.memoized_connection(name)
    } else {
        None
    };
    if let Some(key) = held {
        return Ok(key);
    }

    // The snapshot is cloned rather than borrowed because the block, the
    // target that borrows it and the `ctx` that files the connection are
    // all live at once. It is an `Arc` and a boot generation is shared by
    // every request on the core, so the clone is one refcount.
    let snapshot = ctx
        .config()
        .map(|config| std::sync::Arc::clone(config.snapshot()))
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{named}: this program is running with no configuration at all, so there is \
                     no `[db.{name}]` block to open"
            ))
        })?;
    let block = snapshot.config.db.get(name).ok_or_else(|| {
        Fault::thrown(format!(
            "{named}: nothing sets `[db.{name}]` up, so the name resolves to a block an \
             operator has not written yet"
        ))
    })?;
    // `rule:security/db-pool-reset-is-a-boundary`'s ticket, through the reader that applies the unscoped
    // `pool = false` before it reads this block's own table. The bounds were
    // validated at boot by `nvs_config::db::validate`, so the refusal below
    // cannot fire; if it ever did, `OFF` is the answer that closes this
    // connection with the request rather than pooling it under bounds nobody
    // could resolve.
    let bounds = nvs_config::db::bounds_for(
        &snapshot.config,
        Some(name),
        &std::collections::BTreeMap::new(),
    )
    .unwrap_or(nvs_config::db::PoolBounds::OFF);
    let ticket = nvs_runtime::pool::Ticket::for_block(&snapshot, name, bounds);
    // § 13's ceiling, taken before the handshake so that `max` bounds every
    // connection this core has live under the key and not only the ones the
    // pool itself supplied. The slot goes back when the request ends and on
    // every failure path between here and there — `Lease`'s own `Drop` is
    // what makes that true, so no path below has to remember it.
    //
    // The clone is the price of asking without queueing first: a slot is
    // there on all but the busiest request, and paying one short `String`
    // for the case that has to wait beats a queue registration — a boxed
    // wake and a `Vec` push — on every `connect` that never waits at all.
    let max = ticket.bounds.max;
    let full = |waited: &str| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{CONNECT}: `[db.{name}]` already holds its `max` of {max} connections on this \
                 core, and {waited} — raise `[db.{name}.pool] max`, or hold fewer connections \
                 open at once"
            ),
        )
    };
    let lease = match nvs_runtime::pool::admit(ticket.clone()) {
        Some(lease) => lease,
        None => wait_for_slot(ctx, ticket, &full, deadline)?,
    };

    // § 13's acquire: this core's pool first, and what comes out of it is
    // reset before this request may use it. `warm_connection` is where a
    // failed reset destroys the connection, and `None` from it is
    // indistinguishable here from an empty pool — either way the fall-back
    // is the handshake below, which is what a request did before there was
    // a pool at all.
    let pooled = bounds.enabled.then(|| warm_connection(&lease)).flatten();
    // One wording for both handshakes: which driver could not reach its server
    // is the block's business, and what a program can do about either is the
    // same thing.
    let opening = |at: &dyn std::fmt::Display, err: &std::io::Error| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!("{named}: `[db.{name}]` at {at} did not open: {err}"),
        )
    };
    // `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`'s overload,
    // admitted here and nowhere else: a `host` beginning with a path separator
    // is a socket, asked of `nvs_db::is_socket_host` so the predicate has one
    // home. `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`
    // is why the settings path further down takes a pinned address and never a
    // path — this is the door an operator's own configuration comes through.
    // `socket` is the driver's own spelling of a path — MySQL's file as written,
    // PostgreSQL's `<directory>/.s.PGSQL.<port>` — because
    // `rule:core-classes/db-unix-socket-path` is one piece of protocol trivia
    // per driver and each keeps its own. The port an arm passes is the one it
    // would have dialled.
    let endpoint_of =
        |host: &str,
         port: Option<u16>,
         default_port: u16,
         socket: fn(&str, u16) -> std::io::Result<nvs_db::Endpoint>| {
            if nvs_db::is_socket_host(host) {
                return socket(host, port.unwrap_or(default_port)).map_err(|why| {
                    Fault::thrown_as(
                        ThrownClass::Io,
                        format!(
                            "{named}: `[db.{name}]` names a socket that cannot be opened: {why}"
                        ),
                    )
                });
            }
            address_of(host, port, default_port, name).map(nvs_db::Endpoint::Tcp)
        };
    let written = block.driver.as_deref().unwrap_or("");
    let driver = nvs_db::Driver::from_config_name(written);
    let opened = match pooled {
        Some(warm) => warm,
        // `rule:core-classes/db-connection-is-named`'s `driver` decides which handshake goes out, and it is
        // read here rather than inside a driver: the openers share nothing but
        // this shape — their own target, their own default port, their own
        // `Connection` variant — and one resolver answering for all of them is
        // the trait `rule:core-classes/db-drivers-are-an-enum` declines to write.
        //
        // Every other spelling goes to PostgreSQL, including the block that
        // writes no `driver` at all and the one whose `driver` no backend
        // answers to: `PgTarget::resolve` is where each of those refusals is
        // worded, and it names what was written rather than what it wanted.
        None => match driver {
            // The one arm with nothing to resolve. § 3 puts a block's `path`
            // under `db.connect` for the same reason it pre-approves a block's
            // host — the operator who granted the name wrote the path — so
            // this reaches the opener with the other four's two resolution
            // steps absent rather than skipped, and `opening` above goes unused
            // here because there is no address to name in a refusal.
            Some(nvs_db::Driver::Sqlite) => {
                let target = nvs_db::SqliteTarget::resolve(block).map_err(|refused| {
                    Fault::thrown(format!("{named}: {}", refused.refusal(name)))
                })?;
                let conn = nvs_db::sqlite::open(&target).map_err(|err| {
                    Fault::thrown_as(
                        ThrownClass::Io,
                        format!(
                            "{named}: `[db.{name}]` at `{}` did not open: {err}",
                            target.path.display()
                        ),
                    )
                })?;
                nvs_db::Connection::Sqlite(conn)
            }
            Some(nvs_db::Driver::MySql) => {
                let target = nvs_db::MySqlTarget::resolve(block).map_err(|refused| {
                    Fault::thrown(format!("{named}: {}", refused.refusal(name)))
                })?;
                let endpoint = endpoint_of(
                    target.host,
                    block.port,
                    nvs_db::mysql::DEFAULT_PORT,
                    nvs_db::mysql::socket_endpoint,
                )?;
                let conn = nvs_db::MySqlConn::connect(endpoint.clone(), &target, deadline)
                    .map_err(|err| opening(&endpoint, &err))?;
                nvs_db::Connection::MySql(conn)
            }
            Some(nvs_db::Driver::MariaDb) => {
                let target = nvs_db::MariaTarget::resolve(block).map_err(|refused| {
                    Fault::thrown(format!("{named}: {}", refused.refusal(name)))
                })?;
                // `nvs_db::mysql::socket_endpoint` on a MariaDB block, because
                // the spelling is the thing being shared and it is MySQL's: a
                // MariaDB socket file is named by whoever configured the
                // server, so `rule:core-classes/db-unix-socket-path` opens the
                // path as written here too, and a second identical function
                // under this driver's name would be a copy to keep in step.
                let endpoint = endpoint_of(
                    target.host,
                    block.port,
                    nvs_db::maria::DEFAULT_PORT,
                    nvs_db::mysql::socket_endpoint,
                )?;
                let conn = nvs_db::MariaConn::connect(endpoint.clone(), &target, deadline)
                    .map_err(|err| opening(&endpoint, &err))?;
                nvs_db::Connection::MariaDb(conn)
            }
            Some(nvs_db::Driver::SqlServer) => {
                let target = nvs_db::TdsTarget::resolve(block).map_err(|refused| {
                    Fault::thrown(format!("{named}: {}", refused.refusal(name)))
                })?;
                let address = address_of(target.host, block.port, nvs_db::tds::DEFAULT_PORT, name)?;
                let conn = nvs_db::TdsConn::connect(address, &target, deadline)
                    .map_err(|err| opening(&address, &err))?;
                nvs_db::Connection::SqlServer(conn)
            }
            _ => {
                let target = nvs_db::PgTarget::resolve(block).map_err(|refused| {
                    Fault::thrown(format!("{named}: {}", refused.refusal(name)))
                })?;
                let endpoint = endpoint_of(
                    target.host,
                    block.port,
                    nvs_db::pg::DEFAULT_PORT,
                    nvs_db::pg::socket_endpoint,
                )?;
                let conn = nvs_db::PgConn::connect(endpoint.clone(), &target, deadline)
                    .map_err(|err| opening(&endpoint, &err))?;
                nvs_db::Connection::Postgres(conn)
            }
        },
    };
    // The lease is filed even for `{shared: false}`, whose `None` memo is
    // the slot beside it: that option bypasses memoization *within* the
    // request and never pooling across requests — § 13 says so in as many
    // words.
    let key = ctx.hold_open_connection(
        shared.then(|| name.to_owned()),
        Some(lease),
        Box::new(opened),
    );
    Ok(key)
}

/// `rule:core-api/shape-flattens-at-the-abi`'s merged field list of [`SETTINGS`], as ABI slots: the server
/// arm's ten fields in order, then the SQLite arm's `path` — `driver`,
/// `timeZone` and `timeout` are already spoken for — then the trailing bag's
/// `shared`. Twelve, which is what `nvs_core_db_open` declares.
///
/// A slot belonging to an arm the caller did not write arrives as `Tag::Null`,
/// which is why the reads below check the driver first and then look only at
/// the slots that arm declares.
pub(super) const DRIVER_ARG: usize = 0;
/// See [`DRIVER_ARG`].
pub(super) const HOST_ARG: usize = 1;
/// See [`DRIVER_ARG`].
pub(super) const PORT_ARG: usize = 2;
/// See [`DRIVER_ARG`].
pub(super) const DATABASE_ARG: usize = 3;
/// See [`DRIVER_ARG`].
pub(super) const USER_ARG: usize = 4;
/// See [`DRIVER_ARG`].
pub(super) const PASSWORD_ARG: usize = 5;
/// See [`DRIVER_ARG`].
pub(super) const TLS_ARG: usize = 6;
/// See [`DRIVER_ARG`].
pub(super) const TIME_ZONE_ARG: usize = 7;
/// See [`DRIVER_ARG`].
pub(super) const OPEN_TIMEOUT_ARG: usize = 8;
/// See [`DRIVER_ARG`].
pub(super) const STATEMENT_CACHE_ARG: usize = 9;
/// See [`DRIVER_ARG`].
pub(super) const PATH_ARG: usize = 10;
/// See [`DRIVER_ARG`].
pub(super) const OPEN_SHARED_ARG: usize = 11;

/// The backend a `driver` slot names.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not one of [`DRIVER`]'s ordinals,
/// which the shape's own type rules out — [`isolation_of`] states the same
/// judgement at length.
pub(super) fn settings_driver(value: &Value) -> Result<nvs_db::Driver, Fault> {
    match value.as_int() {
        Some(0) => Ok(nvs_db::Driver::MySql),
        Some(1) => Ok(nvs_db::Driver::MariaDb),
        Some(2) => Ok(nvs_db::Driver::Postgres),
        Some(3) => Ok(nvs_db::Driver::Sqlite),
        Some(4) => Ok(nvs_db::Driver::SqlServer),
        _ => Err(Fault::fatal(format!(
            "{OPEN} expected a `{DRIVER_NAME}` case for `driver`, got tag {}",
            value.tag_byte()
        ))),
    }
}

/// What a written `tls` key means, which is either "the only mode there is" or
/// a refusal naming what was asked for.
///
/// # Errors
///
/// A thrown `RuntimeError` for any of [`TLS`]'s weaker three. That enum's doc
/// owns why they are refused rather than honoured; the short of it is that
/// `rule:core-classes/db-capabilities`'s TLS default is not configurable to the unsafe value, and a
/// connection that verified more than it was asked to would be a promise made
/// quietly.
pub(super) fn settings_tls(value: &Value) -> Result<(), Fault> {
    let asked = match value.as_int() {
        None => return Ok(()),
        Some(3) => return Ok(()),
        Some(0) => "Disabled",
        Some(1) => "Required",
        Some(2) => "VerifyCa",
        Some(_) => {
            return Err(Fault::fatal(format!(
                "{OPEN} expected a `{TLS_NAME}` case for `tls`, got tag {}",
                value.tag_byte()
            )));
        }
    };
    Err(Fault::thrown(format!(
        "{OPEN}: `tls` asks for `Tls::{asked}`, and this runtime opens every TCP connection at \
         `Tls::VerifyFull` — `rule:core-classes/db-capabilities` has no spelling for verifying less. A private \
         certificate authority is a `tls_ca_file` on a `[db.<name>]` block, which changes whose \
         certificates are believed and not whether they are checked"
    )))
}

/// A `uint` slot as its number, or `None` for the [`Const::Null`] an omitting
/// call site passed.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is neither, which the shape's own type
/// rules out.
pub(super) fn settings_uint(value: &Value, key: &str) -> Result<Option<u64>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    value.as_uint().map(Some).ok_or_else(|| {
        Fault::fatal(format!(
            "{OPEN} expected a `uint` for `{key}`, got tag {}",
            value.tag_byte()
        ))
    })
}

/// A text slot the *written arm* requires.
///
/// **Thrown and not [`Fault::fatal`], which the two members either side of it
/// would be.** `E0402` refuses a literal that omits a key the merged list
/// requires, and the merged list can only require a key **every** arm does:
/// `driver` is one, `host` is not, because the SQLite arm does not declare it.
/// So a missing `host` reaches here as a `Tag::Null`, and until `rule:core-api/shape-arms-are-disjoint`'s
/// arm selection lands — this module's known gap 1 — it is a program error and
/// a catchable one, rather than an impossible state worth a `FATAL`.
///
/// # Errors
///
/// A thrown `RuntimeError` naming the key the written `driver` needed.
pub(super) fn settings_text<'a>(args: &'a [Value], at: usize, key: &str) -> Result<&'a str, Fault> {
    args[at].as_text().ok_or_else(|| {
        Fault::thrown(format!(
            "{OPEN}: this `driver` needs a `{key}`, and the settings do not give one"
        ))
    })
}

/// The memo key one settings literal opens under — `rule:core-classes/db-connection-is-named`'s "a hash of
/// every settings field", where `connect`'s key is the name an operator wrote.
///
/// **It cannot collide with a `connect` key**, which is the one property this
/// spelling has to have: both members file into the same per-request table, and
/// a `[db.<name>]` block's name is a configuration key and so can never begin
/// with a NUL byte. Two calls whose fields all agree share a connection, which
/// is what § 2 promises, and two that differ anywhere — including in the
/// password — do not.
///
/// **It is § 13's pool key as well as the memo**, which decides what it hashes:
/// every field that says what the connection *is* once it is open. The endpoint
/// and the credentials are the obvious ones; the declared zone and the
/// statement cache's size are the two that are only obvious from the pool's
/// side, because a drawn connection carries both from the request that opened
/// it — § 13's reset restores the zone rather than re-reading it, as a startup
/// parameter on PostgreSQL and as `set_session_time_zone` on MySQL — and a
/// second request that declared a different one would silently be answered in
/// the first one's. `timeout` and `tls` are deliberately not among them: the
/// first bounds the act of opening rather than the connection it produces, and
/// the second has one accepted value, so hashing either would only split a pool
/// two requests could have shared.
pub(super) fn settings_key(
    fields: &[&str],
    port: Option<u64>,
    zone: i32,
    statement_cache: Option<u64>,
) -> String {
    use std::hash::{Hash as _, Hasher as _};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for field in fields {
        field.hash(&mut hasher);
    }
    port.hash(&mut hasher);
    zone.hash(&mut hasher);
    statement_cache.hash(&mut hasher);
    format!("\u{0}open:{:016x}", hasher.finish())
}

/// The [`settings_key`] a `[db.<name>]` block's own fields hash to, or `None`
/// for a block whose `driver` or `time_zone` is not a value any connection could
/// be opened with.
///
/// **This is how an `open` finds the block whose bounds are its own**, which is
/// `rule:security/db-pool-reset-is-a-boundary`'s answer for a member that names no block. `connect` is keyed
/// on the name an operator wrote and reads that block's `[db.<name>.pool]`
/// directly; a settings literal names nothing, so the only honest question is
/// whether the settings it wrote *are* a block's — and the memo key already
/// answers it, since § 2 hashes exactly the fields that say what the connection
/// is. Building the key from the block rather than comparing its fields one at a
/// time is the point: a second spelling of "the same connection" is how the pool
/// and the memo would come to disagree, and a program that opens a block's
/// endpoint by hand shares that block's pool for the same reason it shares its
/// connection.
///
/// A field the block leaves unwritten hashes as the empty string, which is what
/// a settings literal writing nothing for it hashes too — [`settings_text`]
/// makes every one of the four a `&str`. `port` is the one that cannot be
/// defaulted into agreement: a literal writing `5432` and a block leaving the
/// server's default implicit are two keys and so two pools. That is the hash's
/// own rule rather than this function's, and what an operator loses by it is
/// § 13's bounds on that second pool, never a connection.
pub(super) fn block_settings_key(block: &nvs_config::tree::Database) -> Option<String> {
    let driver = nvs_db::Driver::from_config_name(block.driver.as_deref()?)?;
    let zone = nvs_db::sql::time_zone_for(block)?;
    let statement_cache = block.statement_cache.map(u64::from);
    if driver == nvs_db::Driver::Sqlite {
        let path = block.path.as_deref()?;
        return Some(settings_key(
            &[path, "", "", "", driver.matrix_name()],
            None,
            zone,
            statement_cache,
        ));
    }
    Some(settings_key(
        &[
            block.host.as_deref().unwrap_or_default(),
            block.user.as_deref().unwrap_or_default(),
            block.database.as_deref().unwrap_or_default(),
            block.password.as_deref().unwrap_or_default(),
            driver.matrix_name(),
        ],
        block.port.map(u64::from),
        zone,
        statement_cache,
    ))
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::open(Db\Settings $settings, {shared?: bool}): Db\Connection`
    /// — `rule:core-classes/db-connection-is-named`'s connection the *program* describes.
    ///
    /// **The whole difference from `connect` is which authority wrote the
    /// endpoint**, and § 3 turns that into two checks this body makes and that
    /// one does not. `db.open` is asked about the host rather than a block
    /// name, because a host is what a settings literal chooses; and the
    /// address it resolves to is then put through
    /// `rule:security/net-address-policy`'s
    /// denied ranges in full, which is the check a `connect`-named endpoint is
    /// deliberately exempt from — [`address_of`]'s doc owns that asymmetry from
    /// the other side.
    ///
    /// **The settings become a `[db.<name>]` block and are resolved as one.**
    /// The two drivers' resolvers already own every refusal a set of fields can
    /// earn — a field belonging to another driver, a blank password, a zone
    /// that is not an offset — and re-deciding any of it here would be a second
    /// answer to a question `rule:core-classes/db-connection-is-named` has one of. What this body decides is
    /// only what the config path has no equivalent of: the two checks above,
    /// and the `tls` key, which no block has.
    ///
    /// **What it spends:** one connection per distinct set of settings a
    /// request opens, and § 13's pool keeps up to `idle` of them per key on this
    /// core between the requests that use them. A settings literal names no
    /// block, so what sizes that is
    /// [`crate::db::pool::settings_bounds`]: the `[db.<name>.pool]` table of the
    /// block these very settings describe, if a deployment wrote one, and
    /// `PoolBounds::DEFAULT` otherwise — the same bounds a block that writes no
    /// `pool` key takes, which is what `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s *finite with nothing
    /// configured* already means one layer down, and the only other candidate
    /// (`OFF`) is § 13 declining to pool `open` at all, which that section
    /// spends a bullet requiring. The consequence an operator has to be told
    /// rather than discover: the ceiling on the database is `cores × max` *per
    /// distinct settings hash*, and a literal that differs from the block in any
    /// hashed field — a written `port` where the block left the server's default
    /// implicit — is a second key and so a second pool of that size.
    fn nvs_core_db_open(ctx, args: [12]) {
        let driver = settings_driver(&args[DRIVER_ARG])?;
        // The SQLite arm, whole and taken here: its `path` is the field that
        // says the caller wrote it, and everything below this block is about an
        // address § 2's other arm has and this one does not. § 13's pool is the
        // same four calls on both paths and is written twice rather than
        // extracted, because what feeds them — the key's fields, the
        // capabilities asked, whether a name is resolved at all — is disjoint
        // between the arms and a shared helper would take every one of them as
        // a parameter.
        if driver == nvs_db::Driver::Sqlite {
            return sqlite_settings(ctx, args, driver);
        }

        let host = settings_text(args, HOST_ARG, "host")?;
        let user = settings_text(args, USER_ARG, "user")?;
        let database = settings_text(args, DATABASE_ARG, "database")?;
        let password = settings_text(args, PASSWORD_ARG, "password")?;
        let port = settings_uint(&args[PORT_ARG], "port")?;
        let statement_cache = settings_uint(&args[STATEMENT_CACHE_ARG], "statementCache")?;
        settings_tls(&args[TLS_ARG])?;
        let shared = args[OPEN_SHARED_ARG].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "{OPEN} expected a `bool` for `shared`, got tag {}",
                args[OPEN_SHARED_ARG].tag_byte()
            ))
        })?;

        // § 3's grant, asked before anything is resolved or opened: a host
        // outside it is refused whether or not it exists.
        nvs_runtime::capability::require(
            ctx,
            nvs_config::Cap::DbOpen,
            nvs_config::capability::Scope::Host(host),
            OPEN,
        )?;
        // And § 3's other half — `rule:http-server/allow-url-pins-the-address`'s table, which is what a
        // program-supplied address is subject to and an operator-written one is
        // not. The name is resolved once, here, and the socket below opens to
        // exactly the address that was checked.
        let pinned = nvs_runtime::capability::pinned_address(ctx, host, OPEN)?;

        // § 9's declared zone, resolved here rather than beside the target that
        // carries it because it is one of the fields the key below hashes: a
        // pooled connection is in the zone the request that opened it declared,
        // and § 13's reset restores that zone rather than re-reading it. It is
        // set on the resolved target rather than written into the block, because
        // the block's field is an offset *spelling* and this arrives as a
        // `Core\Time\Zone`: rendering it to text for the resolver to parse back
        // would be two conversions and one more place for the two to disagree.
        let zone = if matches!(args[TIME_ZONE_ARG].tag(), Some(Tag::Null)) {
            0
        } else {
            crate::time::zone_offset_now(args, TIME_ZONE_ARG, "open")?
        };
        let deadline = open_deadline(args)?;
        let memo = settings_key(
            &[host, user, database, password, driver.matrix_name()],
            port,
            zone,
            statement_cache,
        );
        if shared && let Some(key) = ctx.memoized_connection(&memo) {
            return Ok(crate::instance::build(
                &CONNECTION,
                [Value::uint(key), Value::str(NvsStr::new(host.as_bytes()))],
            ));
        }

        // § 13's ticket, under the key § 2 hashes rather than the block name
        // `connect` keys on — `Ticket::for_settings` owns why only one of the
        // two is scoped to a configuration generation, and
        // `crate::db::pool::settings_bounds` owns which block's table these
        // bounds came out of, if any did.
        let ticket =
            nvs_runtime::pool::Ticket::for_settings(memo.clone(), crate::db::pool::settings_bounds(ctx, &memo));
        let max = ticket.bounds.max;
        let full = |waited: &str| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{OPEN}: these settings already hold their `max` of {max} connections to \
                     {host} on this core, and {waited} — a settings literal is keyed on its own \
                     fields, so it is bounded by the `[db.<name>.pool]` of the block describing \
                     that same endpoint if one is written, and by the defaults if none is; open \
                     fewer of them at once"
                ),
            )
        };
        let lease = match nvs_runtime::pool::admit(ticket.clone()) {
            Some(lease) => lease,
            None => wait_for_slot(ctx, ticket, &full, deadline)?,
        };
        // § 13's acquire, and `{shared: false}` reaches it too: that option
        // bypasses the memo above and never the pool, which § 13 says in as
        // many words. `warm_connection` is where a failed reset destroys the
        // connection, so a `None` here is indistinguishable from an empty pool
        // and falls through to the handshake either way.
        let pooled = lease
            .bounds()
            .enabled
            .then(|| warm_connection(&lease))
            .flatten();
        let opened = match pooled {
            Some(warm) => warm,
            // The block the two resolvers read, built only on the path that
            // needs it. Every field is one the settings literal wrote, so what
            // comes back out is the same target a `[db.<name>]` block of the
            // same content would resolve to — including its refusals, which is
            // the point of going through them.
            None => {
                let block = nvs_config::tree::Database {
                    driver: Some(driver.matrix_name().to_owned()),
                    host: Some(host.to_owned()),
                    port: port.and_then(|written| u16::try_from(written).ok()),
                    user: Some(user.to_owned()),
                    password: Some(password.to_owned()),
                    database: Some(database.to_owned()),
                    statement_cache: statement_cache.and_then(|held| u32::try_from(held).ok()),
                    ..nvs_config::tree::Database::default()
                };
                let refused = |refusal: nvs_db::BlockError<'_>| {
                    Fault::thrown(format!("{OPEN}: {}", refusal.refusal("<settings>")))
                };
                let opening = |address: SocketAddr, err: &std::io::Error| {
                    Fault::thrown_as(
                        ThrownClass::Io,
                        format!("{OPEN}: {address} did not open: {err}"),
                    )
                };
                match driver {
                    nvs_db::Driver::MySql => {
                        let mut target = nvs_db::MySqlTarget::resolve(&block).map_err(refused)?;
                        target.time_zone = zone;
                        let address =
                            SocketAddr::new(pinned, port_of(port, nvs_db::mysql::DEFAULT_PORT));
                        let conn = nvs_db::MySqlConn::connect(address, &target, deadline)
                            .map_err(|err| opening(address, &err))?;
                        nvs_db::Connection::MySql(conn)
                    }
                    nvs_db::Driver::MariaDb => {
                        let mut target = nvs_db::MariaTarget::resolve(&block).map_err(refused)?;
                        target.time_zone = zone;
                        let address =
                            SocketAddr::new(pinned, port_of(port, nvs_db::maria::DEFAULT_PORT));
                        let conn = nvs_db::MariaConn::connect(address, &target, deadline)
                            .map_err(|err| opening(address, &err))?;
                        nvs_db::Connection::MariaDb(conn)
                    }
                    nvs_db::Driver::Postgres => {
                        let mut target = nvs_db::PgTarget::resolve(&block).map_err(refused)?;
                        target.time_zone = zone;
                        let address =
                            SocketAddr::new(pinned, port_of(port, nvs_db::pg::DEFAULT_PORT));
                        let conn = nvs_db::PgConn::connect(address, &target, deadline)
                            .map_err(|err| opening(address, &err))?;
                        nvs_db::Connection::Postgres(conn)
                    }
                    nvs_db::Driver::SqlServer => {
                        let mut target = nvs_db::TdsTarget::resolve(&block).map_err(refused)?;
                        target.time_zone = zone;
                        let address =
                            SocketAddr::new(pinned, port_of(port, nvs_db::tds::DEFAULT_PORT));
                        let conn = nvs_db::TdsConn::connect(address, &target, deadline)
                            .map_err(|err| opening(address, &err))?;
                        nvs_db::Connection::SqlServer(conn)
                    }
                    // SQLite is the only driver left, and it never arrives: its
                    // arm is taken above at its own field, before a host is
                    // resolved at all. This arm is what keeps the match total,
                    // and the refusal it words is the honest one for a backend
                    // added later whose opener has not landed with it.
                    other => {
                        return Err(Fault::thrown(format!(
                            "{OPEN}: `{}` is a driver this build has no connection path for yet, \
                             so the settings naming it cannot be opened",
                            other.display_name()
                        )));
                    }
                }
            }
        };
        let key =
            ctx.hold_open_connection(shared.then_some(memo), Some(lease), Box::new(opened));
        Ok(crate::instance::build(
            &CONNECTION,
            [Value::uint(key), Value::str(NvsStr::new(host.as_bytes()))],
        ))
    }
}

/// § 2's `open` over the SQLite arm of `rule:core-api/shape-parameter`'s union: a file, § 13's
/// pool around it, and none of the address machinery the server arm is.
///
/// **§ 3's grants are three and not one.** That section puts a *config-written*
/// path under `db.connect` and says a program-supplied one "additionally needs
/// `fs.read`/`fs.write` and is a path sink", so this asks `db.open` for the
/// path, then both filesystem grants, before it has looked at whether the file
/// exists. `db.open`'s scope is the path rather than a host — the capability is
/// "which dynamic settings may be reached" and a grant list holds host patterns
/// for the four drivers that have a host and path prefixes for the one that has
/// a file, each matched by `nvs_config::capability::Scope`'s own rule for what
/// it is. `fs.write` is asked of a read-only workload too, because SQLite
/// writes its journal beside the database and a connection that cannot is one
/// that fails at the first statement rather than at `open`.
///
/// **`rule:http-server/allow-url-pins-the-address`'s address table is not consulted**, and that is not an omission:
/// there is no address. The reason § 3 subjects `open`'s host to it — a
/// program-supplied address can be attacker-influenced into the ranges a
/// database lives at — is answered here by `fs.read`/`fs.write`, which is the
/// grant an operator writes about a path.
///
/// **A relative path stays relative to the process**, and that is the deliberate
/// asymmetry with a `[db.<name>] path`, which `nvs_config::db`'s `canonicalize`
/// resolves against the configuration file that wrote it (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`). There
/// is no file to resolve against here: the program computed this string, and
/// resolving it against a configuration file it never named would make the
/// meaning of a program's own path depend on where the operator keeps `nvs.toml`.
/// So the base is the working directory, which is the base every other path a
/// program hands `Core\Fs` already has, and the grant list is where an operator
/// bounds it.
///
/// # Errors
///
/// The three capability refusals, `nvs_db::BlockError`'s for a `path` that is
/// not one, and a thrown `Io` for a file that did not open.
pub(super) fn sqlite_settings(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    driver: nvs_db::Driver,
) -> Result<Value, Fault> {
    let path = settings_text(args, PATH_ARG, "path")?;
    let statement_cache = settings_uint(&args[STATEMENT_CACHE_ARG], "statementCache")?;
    let shared = args[OPEN_SHARED_ARG].as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "{OPEN} expected a `bool` for `shared`, got tag {}",
            args[OPEN_SHARED_ARG].tag_byte()
        ))
    })?;

    let file = std::path::Path::new(path);
    for cap in [
        nvs_config::Cap::DbOpen,
        nvs_config::Cap::FsRead,
        nvs_config::Cap::FsWrite,
    ] {
        nvs_runtime::capability::require(
            ctx,
            cap,
            nvs_config::capability::Scope::Path(file),
            OPEN,
        )?;
    }

    // § 9's declared zone, and § 13's key hashes it for [`nvs_core_db_open`]'s
    // reason: a pooled connection is in the zone the request that opened it
    // declared.
    let zone = if matches!(args[TIME_ZONE_ARG].tag(), Some(Tag::Null)) {
        0
    } else {
        crate::time::zone_offset_now(args, TIME_ZONE_ARG, "open")?
    };
    let deadline = open_deadline(args)?;
    // The path stands where a host stands, and the three credential fields are
    // empty because this arm declares none — two settings literals naming
    // different files are two keys, which is all § 13 asks of it.
    let memo = settings_key(
        &[path, "", "", "", driver.matrix_name()],
        None,
        zone,
        statement_cache,
    );
    if shared && let Some(key) = ctx.memoized_connection(&memo) {
        return Ok(crate::instance::build(
            &CONNECTION,
            [Value::uint(key), Value::str(NvsStr::new(path.as_bytes()))],
        ));
    }

    let ticket = nvs_runtime::pool::Ticket::for_settings(
        memo.clone(),
        crate::db::pool::settings_bounds(ctx, &memo),
    );
    let max = ticket.bounds.max;
    let full = |waited: &str| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{OPEN}: these settings already hold their `max` of {max} connections to \
                 `{path}` on this core, and {waited} — a settings literal is keyed on its own \
                 fields, so it is bounded by the `[db.<name>.pool]` of the block naming that same \
                 file if one is written, and by the defaults if none is; open fewer of them at \
                 once"
            ),
        )
    };
    let lease = match nvs_runtime::pool::admit(ticket.clone()) {
        Some(lease) => lease,
        None => wait_for_slot(ctx, ticket, &full, deadline)?,
    };
    let pooled = lease
        .bounds()
        .enabled
        .then(|| warm_connection(&lease))
        .flatten();
    let opened = match pooled {
        Some(warm) => warm,
        // The block the resolver reads, carrying the two keys this arm shares
        // with the server one and nothing else: `SqliteTarget::resolve` refuses
        // a `host`, a `user` or a `password` beside a `path`, and a block built
        // with those fields empty is the same target § 2's own arm selection
        // has already guaranteed the literal wrote.
        None => {
            let block = nvs_config::tree::Database {
                driver: Some(driver.matrix_name().to_owned()),
                path: Some(path.to_owned()),
                statement_cache: statement_cache.and_then(|held| u32::try_from(held).ok()),
                ..nvs_config::tree::Database::default()
            };
            let mut target = nvs_db::SqliteTarget::resolve(&block).map_err(|refusal| {
                Fault::thrown(format!("{OPEN}: {}", refusal.refusal("<settings>")))
            })?;
            target.time_zone = zone;
            let conn = nvs_db::sqlite::open(&target).map_err(|err| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{OPEN}: `{path}` did not open: {err}"),
                )
            })?;
            nvs_db::Connection::Sqlite(conn)
        }
    };
    let key = ctx.hold_open_connection(shared.then_some(memo), Some(lease), Box::new(opened));
    Ok(crate::instance::build(
        &CONNECTION,
        [Value::uint(key), Value::str(NvsStr::new(path.as_bytes()))],
    ))
}

/// Which port a settings literal reaches: the one it wrote, or the driver's.
///
/// [`address_of`]'s rule, applied to the member that resolved its host
/// somewhere else: a written port that does not fit a `u16` cannot be a port at
/// all, so it falls back rather than truncating into one.
pub(super) fn port_of(written: Option<u64>, default_port: u16) -> u16 {
    written
        .and_then(|held| u16::try_from(held).ok())
        .unwrap_or(default_port)
}

/// [`deadline_of`]'s twin for `open`, whose `timeout` is a *shape field* and so
/// arrives in a slot of its own.
///
/// # Errors
///
/// A thrown `RuntimeError` for a duration that is zero or negative, and a
/// [`Fault::fatal`] for a slot that is neither a `Duration` nor `Tag::Null`.
pub(super) fn open_deadline(args: &[Value]) -> Result<Option<std::time::Instant>, Fault> {
    if matches!(args[OPEN_TIMEOUT_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let nanos = crate::time::nanos_of(args, OPEN_TIMEOUT_ARG, "timeout")?;
    if nanos <= 0 {
        return Err(Fault::thrown(format!(
            "{OPEN}: `timeout` must be a positive duration, and this one is {nanos}ns"
        )));
    }
    Ok(Some(
        std::time::Instant::now() + std::time::Duration::from_nanos(nanos.unsigned_abs()),
    ))
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::inList(array<mixed> $values): Db\InList` — `rule:core-classes/db-parameters`'s
    /// explicit expansion marker.
    ///
    /// The body is the refusal and a carrier around the argument: what the
    /// marker expands *to* is the driver's, for the reason this module's docs
    /// give. The array is retained rather than copied, so a list of a thousand
    /// ids costs one reference and not a second array.
    fn nvs_core_db_in_list(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is `array<mixed>` in `CLASS`
        // above, so a non-container argument is refused at `E0401` before any
        // of this runs. The guard is what makes the count below sound.
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Db::inList expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for the length of this call"
        )]
        let count = unsafe { nvs_runtime::nvs_array_count(array) };
        if count == 0 {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "Core\\Db::inList() was given an empty list, and `rule:core-classes/db-parameters` refuses one: an \
                 empty list matches nothing inside `IN` and everything inside `NOT IN`, the \
                 rewriter cannot tell which it is in, and the caller has to branch"
                    .to_owned(),
            ));
        }
        let values = args[0];
        #[expect(
            unsafe_code,
            reason = "the array is owned by the caller's argument slot, which \
                      outlives this call, so the slot this builds needs a \
                      reference of its own"
        )]
        unsafe {
            values.retain();
        }
        Ok(crate::instance::build(&IN_LIST, [values]))
    }
}

/// Whether `name` is an identifier every backend reads the same way.
///
/// [`nvs_db::is_bare_identifier`] is the rule and its one home: the same
/// judgement decides what may enter a
/// `rule:core-classes/schema-is-a-value`
/// schema value, and a second copy here would be two answers to the question
/// `rule:security/tainted-qualifier` allows one
/// answer to. The *length* half of that module's rule is deliberately not
/// applied here — a name laundered for a statement someone else wrote has no
/// schema to be too long for.
fn is_bare_identifier(name: &str) -> bool {
    nvs_db::is_bare_identifier(name)
}

nvs_runtime::nvs_helper! {
    /// `Core\Db::quoteIdentifier(tainted string $name): string` — `rule:security/sink-predicate`
    /// 's launderer for the statement-text sink.
    ///
    /// It validates rather than escapes, and this module's docs are the whole
    /// argument for why: with no connection there is no dialect, and the
    /// character class below is the one answer that is safe under all five at
    /// once.
    fn nvs_core_db_quote_identifier(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is a `string` in `CLASS` above,
        // so a non-text argument is refused at `E0401` first — the same
        // judgement `Core\Arr::count`'s guard states.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Db::quoteIdentifier expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        if !is_bare_identifier(name) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "Core\\Db::quoteIdentifier() takes a bare identifier — a letter or `_`, then \
                     letters, digits or `_` — and `{name}` is not one. `Core\\Db` has no \
                     connection and so no dialect to delimit for, so a name that needs \
                     delimiting is refused rather than quoted for the wrong backend"
                ),
            ));
        }
        Ok(Value::str(NvsStr::new(name.as_bytes())))
    }
}

/// One [`nvs_db::Driver`] as the [`DRIVER`] case a program compares against.
///
/// [`crate::db::column::column_type_value`]'s shape and for its reason: an enum
/// is its ordinal at run time (`rule:enums/closed-integer-type`),
/// and the ordinal is looked up in the registered roster rather than written
/// out here, so the two cannot drift apart. Exhaustive on purpose — a sixth
/// backend arrives as a non-exhaustive `match` rather than as a connection that
/// names the wrong driver.
fn driver_value(of: nvs_db::Driver) -> Value {
    let case = match of {
        nvs_db::Driver::MySql => "MySql",
        nvs_db::Driver::MariaDb => "MariaDb",
        nvs_db::Driver::Postgres => "Postgres",
        nvs_db::Driver::Sqlite => "Sqlite",
        nvs_db::Driver::SqlServer => "SqlServer",
    };
    let (_, ordinal) = DRIVER
        .cases
        .iter()
        .find(|(name, _)| *name == case)
        .expect("every `nvs_db::Driver` names a case `DRIVER` registers");
    Value::int(*ordinal)
}

nvs_runtime::nvs_helper! {
    /// `$c->close(): void` — spec § 18's `Connection` row, over
    /// [`nvs_runtime::Ctx::close_open_connection`].
    ///
    /// **The work is the runtime's, and deliberately all of it.** `rule:security/db-pool-reset-is-a-boundary`
    /// 's release is one piece of code — the two lines a request's teardown
    /// runs over every connection it still holds — and a `close` is that code
    /// reached early for one of them. Writing the release again here would be a
    /// second answer to "where does a connection go", and the pool's bounds are
    /// counted against the lease this consumes.
    ///
    /// Nothing is refused. A `close` of a connection already closed is the
    /// state the caller asked for, and a handle whose key this request never
    /// filed cannot be built by a program — [`connection_of`] has already said
    /// so with a `Fault::fatal` before this line.
    fn nvs_core_db_connection_close(ctx, args: [1]) {
        let (key, _) = connection_of(args[0], "close")?;
        ctx.close_open_connection(key);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$c->driver(): Core\Db\Driver` — which backend this connection speaks
    /// to.
    ///
    /// Read off the connection rather than off the `[db.<name>]` block in the
    /// receiver's second slot, because a `Core\Db::open` has no block: the
    /// driver a settings literal named is a field of the thing that was opened,
    /// and that is the one place both entry points agree.
    fn nvs_core_db_connection_driver(ctx, args: [1]) {
        let (key, _) = connection_of(args[0], "driver")?;
        let driver = crate::db::pool::filed_connection(ctx, key, DRIVER_MEMBER)?.driver();
        Ok(driver_value(driver))
    }
}

nvs_runtime::nvs_helper! {
    /// `$c->serverVersion(): string` — what the other end said it is.
    ///
    /// **A field read and not a statement.** Every driver kept the version its
    /// own handshake had already delivered
    /// ([ADR 0187 § 2](/docs/decisions/0187.md)), so this spends no round trip
    /// and answers between two rows of a `stream`, where the connection is held
    /// and a statement is refused. A member that looked like a field read and
    /// cost a `SELECT version()` would be a latency trap on the request path.
    ///
    /// It reaches for the connection, so a closed one refuses it — `driver`'s
    /// side of `rule:security/db-pool-reset-is-a-boundary`'s boundary rather
    /// than `isOpen`'s.
    fn nvs_core_db_connection_server_version(ctx, args: [1]) {
        let (key, _) = connection_of(args[0], "serverVersion")?;
        let filed = crate::db::pool::filed_connection(ctx, key, SERVER_VERSION_MEMBER)?;
        Ok(Value::str(NvsStr::new(filed.server_version().as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `$c->isOpen(): bool` — whether a statement may still run on this
    /// connection.
    ///
    /// **The one member that reads the table without asking it for the
    /// connection**, which is what lets it answer after a `close` where
    /// everything else throws. It is a question about the request's own
    /// bookkeeping and never about the socket: a server that has gone away is
    /// discovered by the statement that fails, since asking the wire would mean
    /// a round trip on a member a program writes in a condition.
    fn nvs_core_db_connection_is_open(ctx, args: [1]) {
        let (key, _) = connection_of(args[0], "isOpen")?;
        Ok(Value::bool(ctx.open_connection_mut(key).is_some()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::{Ctx, OutputSink};

    /// `rule:security/db-pool-reset-is-a-boundary`'s key for `open` is § 2's memo key, so what that hash
    /// separates is what two requests are refused a shared connection over.
    ///
    /// **Asserted as a sweep that every field moves the key**, rather than as
    /// one pair per field: a key that dropped a field would still answer
    /// plausibly on every other pair, and the two the pool added — the declared
    /// zone and the statement cache's size — are exactly the ones a reader
    /// checking "did it hash the credentials?" would not miss. The count is the
    /// assertion, so a field added to the hash without a case here fails it.
    #[test]
    fn every_settings_field_the_pool_keys_on_moves_the_key() {
        let base = settings_key(&["h", "u", "d", "p", "postgres"], Some(5432), 0, Some(64));
        let variants = [
            settings_key(
                &["other", "u", "d", "p", "postgres"],
                Some(5432),
                0,
                Some(64),
            ),
            settings_key(
                &["h", "other", "d", "p", "postgres"],
                Some(5432),
                0,
                Some(64),
            ),
            settings_key(
                &["h", "u", "other", "p", "postgres"],
                Some(5432),
                0,
                Some(64),
            ),
            settings_key(
                &["h", "u", "d", "other", "postgres"],
                Some(5432),
                0,
                Some(64),
            ),
            settings_key(&["h", "u", "d", "p", "mysql"], Some(5432), 0, Some(64)),
            settings_key(&["h", "u", "d", "p", "postgres"], Some(3306), 0, Some(64)),
            settings_key(&["h", "u", "d", "p", "postgres"], None, 0, Some(64)),
            settings_key(
                &["h", "u", "d", "p", "postgres"],
                Some(5432),
                2 * 3600,
                Some(64),
            ),
            settings_key(&["h", "u", "d", "p", "postgres"], Some(5432), 0, Some(32)),
            settings_key(&["h", "u", "d", "p", "postgres"], Some(5432), 0, None),
        ];
        let mut keys: Vec<&str> = variants.iter().map(String::as_str).collect();
        keys.push(&base);
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), variants.len() + 1, "{keys:?}");
        assert_eq!(
            base,
            settings_key(&["h", "u", "d", "p", "postgres"], Some(5432), 0, Some(64))
        );
        // The one property § 2 needs of the spelling: a `connect` key is a
        // block's name and a name is a configuration key, so no settings hash
        // can be one.
        assert!(base.starts_with('\u{0}'));
    }

    /// `rule:core-classes/db-connection-is-named`'s memoization, asserted where it is written: a second
    /// `Core\Db::connect("main")` in one request answers the connection the
    /// first one opened, and performs no handshake of its own.
    ///
    /// **The first call is stood in for by its own last statement.** What sits
    /// between a `connect` and its memo entry is a socket, a TLS session and a
    /// login — a server, which no `-p nvs-stdlib` test has — and this case is
    /// about what the *second* call does. So the context starts where the
    /// first call leaves it: one connection filed under `main`, by the
    /// [`nvs_runtime::Ctx::hold_open_connection`] that is the line
    /// [`open_named`] ends on.
    ///
    /// **"No handshake" is asserted by making one impossible.** This context
    /// carries no configuration at all, so every path past the memo refuses
    /// before it can read a `[db.main]` block, let alone open a socket. The
    /// shared call answering a key is therefore proof it returned at
    /// [`nvs_runtime::Ctx::memoized_connection`] and nowhere later, and the
    /// `{shared: false}` call reaching that refusal on the same context is the
    /// other half of the same evidence — § 2's opt-out is charged again, as
    /// [`CONNECT_DOC`] tells a caller it is.
    ///
    /// **One connection, counted rather than read off the key.** A memoized
    /// call that answered the right key and *also* filed a connection would
    /// pass every assertion above while leaving the request holding two, so
    /// the next name filed has to land at the slot after the first.
    #[test]
    fn a_named_connection_is_memoized_for_the_request() {
        /// A connection that has already been opened, which is the whole of
        /// what this case needs one to be: the memo answers with a key and
        /// never asks what the key holds.
        #[derive(Debug)]
        struct Opened;

        impl nvs_runtime::HeldConnection for Opened {
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }

        // The default the two bare calls in § 2's sentence are made under:
        // memoization is what a program gets for writing no option at all, and
        // `{shared: false}` is the only way out of it.
        let connect = CLASS
            .methods
            .iter()
            .find(|row| row.name == "connect")
            .expect("spec § 18's `connect` is this class's own row");
        let Some(CoreTy::Options(options)) = connect.params.last() else {
            panic!("R2's one trailing options bag is `connect`'s last parameter")
        };
        let shared = options
            .iter()
            .find(|option| option.name == "shared")
            .expect("§ 2's opt-out is an option of that bag");
        assert_eq!(
            format!("{:?}", shared.default),
            format!("{:?}", Const::Bool(true)),
            "a `connect` that writes no option is a shared one, which is what \
             makes the pair below the ordinary case rather than an opt-in"
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let first = ctx.hold_open_connection(Some("main".to_owned()), None, Box::new(Opened));

        let second = open_named(&mut ctx, "main", true, None, CONNECT)
            .expect("§ 2's memo answers before anything reads a configuration");
        assert_eq!(
            first, second,
            "two `connect(\"main\")` calls in one request name one connection"
        );

        let refused = open_named(&mut ctx, "main", false, None, CONNECT).expect_err(
            "`{shared: false}` bypasses the memo, and there is no `[db.main]` behind it",
        );
        let Fault::Thrown(ThrownClass::Runtime, message) = refused else {
            panic!("a name with no configuration behind it refuses as a plain `RuntimeError`")
        };
        assert!(
            message.contains("no configuration at all"),
            "the unshared call got past the memo and refused for want of a \
             block — which is what the shared one did not do: {message}"
        );

        let other = ctx.hold_open_connection(Some("reports".to_owned()), None, Box::new(Opened));
        assert_eq!(
            other,
            first + 1,
            "the memoized call filed nothing, so the next name takes the very \
             next slot — one connection, not two under one key"
        );
        assert_eq!(
            ctx.memoized_connection("main"),
            Some(first),
            "and the name still resolves to the connection the first call opened"
        );
    }

    /// Spec § 18's `close`, asserted as the pair of facts it is: the request
    /// stops holding the connection, and every member that needs one then
    /// refuses.
    ///
    /// **The refusal is asked of [`crate::db::pool::filed_connection`] rather
    /// than of `query`.** That helper is where every statement-running member
    /// reaches the table, so asking it is asking all four at once; asking
    /// `query` instead would need a server behind the handle to get past the
    /// bind, and would still be testing this one line.
    ///
    /// **A `LogicError` and not the `Fault::fatal` next to it.** Those two
    /// answers are one `None` from
    /// [`nvs_runtime::Ctx::open_connection_mut`] and they mean opposite things
    /// — a program that closed its own connection, and a key this crate wrote
    /// into a handle slot wrongly — so the case pins the class as well as the
    /// refusal.
    ///
    /// **`isOpen` is called through the ABI**, because it is the one member
    /// whose whole job is to still answer here, and calling the body directly
    /// would not prove the registered symbol reaches it.
    #[test]
    fn close_releases_the_connection_and_a_later_member_refuses() {
        /// The same stand-in [`a_named_connection_is_memoized_for_the_request`]
        /// files, and for the same reason: what a `close` does to the request's
        /// table is decided without asking the connection anything.
        #[derive(Debug)]
        struct Opened;

        impl nvs_runtime::HeldConnection for Opened {
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }

        let mut ctx = Ctx::new(OutputSink::Sink);
        let key = ctx.hold_open_connection(Some("main".to_owned()), None, Box::new(Opened));
        let handle = crate::instance::build(
            &CONNECTION,
            [Value::uint(key), Value::str(NvsStr::new(b"main"))],
        );

        let before = nvs_runtime::call(nvs_core_db_connection_is_open, &mut ctx, &[handle])
            .expect("`isOpen` answers a `bool` and refuses nothing");
        assert_eq!(
            before.as_bool(),
            Some(true),
            "a connection the request is holding is open"
        );

        nvs_runtime::call(nvs_core_db_connection_close, &mut ctx, &[handle])
            .expect("§ 18's `close` answers `void`");

        let after = nvs_runtime::call(nvs_core_db_connection_is_open, &mut ctx, &[handle])
            .expect("`isOpen` is the member a closed connection still answers");
        assert_eq!(
            after.as_bool(),
            Some(false),
            "the handle is still a handle, and it names nothing"
        );
        assert_eq!(
            ctx.memoized_connection("main"),
            None,
            "§ 2 memoizes a connection, so a name whose connection has gone is \
             free for the next `connect` to open"
        );

        let refused = filed_connection(&mut ctx, key, QUERY)
            .expect_err("a statement needs the connection this program released");
        let Fault::Thrown(ThrownClass::Logic, message) = refused else {
            panic!("using a closed connection is the program's mistake, not the wire's")
        };
        assert!(
            message.contains("has been closed"),
            "the refusal says which of the two `None`s it was: {message}"
        );

        nvs_runtime::call(nvs_core_db_connection_close, &mut ctx, &[handle])
            .expect("a second `close` asks for a state that already holds");
        assert!(
            !ctx.close_open_connection(key),
            "and it releases nothing a second time, which is what keeps the \
             pool's count of this lease at one"
        );
    }

    /// `rule:core-classes/db-capabilities`'s asymmetry, asserted as the **contrast** it is: the very
    /// loopback address `rule:security/net-address-policy`'s door refuses is the address
    /// `Core\Db::connect` opens to, on a deployment that grants `db.connect`
    /// and writes nothing under `[capabilities.net]` at all.
    ///
    /// On its own, "`connect` reached `127.0.0.1`" is not the claim — an
    /// address nobody denies prints the same line. So the door is asked first,
    /// twice, and both of its refusals are pinned: with no `net.connect` the
    /// host never reaches the range table, and with `net.connect` but no
    /// `net.internal` the range table is what refuses. Only then is
    /// [`address_of`] asked, and it answers.
    ///
    /// The deepest half is a signature rather than an assertion, and is worth
    /// saying because no `assert!` can reach it: [`address_of`] takes no
    /// [`Ctx`], so there is no configuration in front of that path to consult.
    /// The pre-approval is structural, and adding a check to it later would
    /// have to change what the function is handed first.
    #[test]
    fn a_connect_named_private_endpoint_needs_no_net_connect_grant() {
        const HOST: &str = "127.0.0.1";

        // What the operator wrote: one block may be opened by name, and this
        // deployment reaches no host and excepts no address.
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_config(crate::tests::granting(
            "[capabilities.db]\nconnect = [\"main\"]\n",
        ));

        // § 3's two capabilities answer different questions: the one this path
        // asks, and the neighbour it does not.
        nvs_runtime::capability::require(
            &ctx,
            nvs_config::Cap::DbConnect,
            nvs_config::capability::Scope::Name("main"),
            CONNECT,
        )
        .expect("`db.connect` grants the block by name, and this deployment granted it");
        let ungranted = nvs_runtime::capability::require(
            &ctx,
            nvs_config::Cap::NetConnect,
            nvs_config::capability::Scope::Host(HOST),
            CONNECT,
        )
        .expect_err("and nothing here grants `net.connect` for any host at all");

        // `rule:http-server/allow-url-pins-the-address`'s door on that same deployment, refusing the host before it
        // is resolved — the first of the two ways a database on loopback would
        // be unreachable if a named endpoint went through it.
        let by_door = nvs_runtime::capability::pin_host(&ctx, HOST, CONNECT)
            .expect_err("the door asks `net.connect` first — `rule:security/net-address-policy`");
        assert_eq!(
            format!("{by_door:?}"),
            format!("{ungranted:?}"),
            "the door's first question is the capability's own, unchanged"
        );

        // And the second way: buy the host back, and § 3's range table is what
        // refuses. That is the check `rule:core-classes/db-capabilities` says a named endpoint is not
        // additionally put through — where every database on `10/8`, a
        // container network or loopback lives.
        let mut reachable = Ctx::new(OutputSink::Sink);
        reachable.set_config(crate::tests::granting(
            "[capabilities.db]\nconnect = [\"main\"]\n\
             [capabilities.net]\nconnect = [\"127.0.0.1\"]\n",
        ));
        let by_range = nvs_runtime::capability::pin_host(&reachable, HOST, CONNECT)
            .expect_err("loopback is the first range § 3 denies");
        assert!(
            format!("{by_range:?}").contains("net.internal"),
            "the range half names the key that would except it: {by_range:?}"
        );

        // The path `Core\Db::connect` actually takes, on the deployment that
        // granted neither `net` key: it answers the address both refusals above
        // just named.
        let pinned = address_of(HOST, Some(5432), nvs_db::pg::DEFAULT_PORT, "main").expect(
            "a `connect`-named endpoint is pre-approved — `rule:core-classes/db-capabilities`",
        );
        assert_eq!(
            pinned,
            SocketAddr::from(([127, 0, 0, 1], 5432)),
            "and it is the written host's own address, resolved once"
        );
    }

    /// `rule:core-classes/db-capabilities`'s other side, and the same address: a target
    /// `Core\Db::open` was *granted* is still refused when it resolves into one
    /// of `rule:security/net-address-policy`'s denied ranges, because a program-supplied address
    /// stays subject to that policy in full.
    ///
    /// The claim is the pair, not either half. `db.open` granting the host is
    /// asserted first, so the refusal that follows cannot be read as an
    /// ungranted one; and [`address_of`] — the path `connect` takes to the very
    /// same address — is asked last and answers, so the refusal cannot be read
    /// as "this runtime will not open loopback". What separates them is which
    /// authority wrote the endpoint, and that is the whole of § 3.
    ///
    /// The deepest half is again a signature: `pinned_address` takes the
    /// [`Ctx`], so the policy has a deployment's `net.internal` in front of it,
    /// where [`address_of`] has nothing in front of it at all.
    #[test]
    fn a_db_open_target_in_a_denied_range_fails() {
        const HOST: &str = "127.0.0.1";

        // What the operator wrote: this program may open a database at that
        // host, and nothing excepts any address from § 3's table.
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_config(crate::tests::granting(
            "[capabilities.db]\nopen = [\"127.0.0.1\"]\n",
        ));

        nvs_runtime::capability::require(
            &ctx,
            nvs_config::Cap::DbOpen,
            nvs_config::capability::Scope::Host(HOST),
            OPEN,
        )
        .expect("`db.open` grants the host by name, and this deployment granted it");

        let by_range = nvs_runtime::capability::pinned_address(&ctx, HOST, OPEN).expect_err(
            "a granted host is not a permitted address — `rule:core-classes/db-capabilities`",
        );
        let said = format!("{by_range:?}");
        assert!(
            // `::open` and not [`OPEN`] itself: this is the `Debug` rendering
            // and it escapes the class's own backslash.
            said.contains("net.internal") && said.contains("::open"),
            "the refusal names the key that would except the range, and the member that \
             asked: {said}"
        );

        // And the same address down `connect`'s path, on a deployment that
        // grants no `net` key either: pre-approved, because an operator wrote
        // the endpoint into root-owned configuration.
        let pinned = address_of(HOST, Some(5432), nvs_db::pg::DEFAULT_PORT, "main").expect(
            "a `connect`-named endpoint is pre-approved — `rule:core-classes/db-capabilities`",
        );
        assert_eq!(
            pinned,
            SocketAddr::from(([127, 0, 0, 1], 5432)),
            "so the two members differ in the check and not in the address"
        );
    }

    /// A block that writes no `port` reaches **its own** driver's port — ADR
    /// 0067 § 2, where the `driver` field is what the rest of the block is read
    /// against.
    ///
    /// The claim is the disagreement rather than either number: [`address_of`]
    /// cannot see the block, so a default read off one driver would send a
    /// `mysql` block's handshake to 5432 and fail as a connection refused,
    /// which reads like a server that is down and not like a bug here. Both
    /// constants are asked for by the same expression the branch in
    /// [`open_named`] uses, so a third driver landing with a port of its own
    /// cannot quietly inherit one of these two.
    #[test]
    fn a_block_with_no_port_reaches_its_own_drivers_default() {
        const HOST: &str = "127.0.0.1";

        assert_ne!(
            nvs_db::pg::DEFAULT_PORT,
            nvs_db::mysql::DEFAULT_PORT,
            "the two servers do not listen in the same place, which is why the port is a \
             parameter at all"
        );
        for (default, expected) in [
            (nvs_db::pg::DEFAULT_PORT, 5432),
            (nvs_db::mysql::DEFAULT_PORT, 3306),
        ] {
            let pinned = address_of(HOST, None, default, "main")
                .expect("a literal host resolves to itself with no name service at all");
            assert_eq!(
                pinned,
                SocketAddr::from(([127, 0, 0, 1], expected)),
                "a block writing no `port` means the port its driver listens on"
            );
        }
    }
}
