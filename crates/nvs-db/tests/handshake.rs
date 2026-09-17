//! `rule:core-classes/db-capabilities`'s open against a real
//! server, one driver per process: the whole sequence — a socket, the in-band
//! upgrade, the TLS session and the authentication exchange — completes, and
//! the reads it is made of hand the core back rather than holding it.
//!
//! An integration test rather than a `mod tests` beside the driver, for the
//! reason [`pool_reuse`](/crates/nvs-db/tests/pool_reuse.rs)'s module doc gives and this crate's
//! own [`nvs_db::matrix`] exists for: a `PgConn`'s wire is `Wire<NvsTls<NvsTcp>>`
//! at the default type parameter, so nothing reachable only through one of its
//! inherent methods can be built without a socket and a certificate. `pg.rs`'s
//! own cases script a peer and hold each *step* — the eight bytes of the
//! `SSLRequest`, the refusal of a server with no TLS, the SCRAM exchange, the
//! `BackendKeyData` a startup must carry. What none of them can hold is that
//! those steps compose into a connection a real PostgreSQL accepts, which is
//! the only question left once each half is green.
//!
//! **`mysql.rs`'s scripted `Peer` stands in exactly that position for the other
//! driver**, and a `MySqlConn`'s wire is the same concrete type, so the
//! question left over is the same one: that the greeting, the `CLIENT_SSL`
//! handshake response sent before the upgrade, the plugin exchange that follows
//! it and the declared zone compose into a connection a real MySQL accepts.
//! Both drivers' cases live in this one file because the *question* is one
//! question — a scripted peer always agrees with the client that wrote it — and
//! a second file would be the same skip rule and the same helpers written
//! twice.
//!
//! **MariaDB is where the twinning pays for itself.** Its driver borrows
//! `mysql.rs`'s framing and nothing above it — its own authentication roster,
//! its own § 8 code table — so a scripted peer can only ever confirm that this
//! crate agrees with itself about a protocol two servers implement differently.
//! `mariadb()` below is `mysql()`'s twin and the leg it selects is the matrix's
//! own, so the same facts are asked of a real MariaDB: the session it ends up
//! holding is encrypted, it is authenticated as the account named, and the
//! credential it refuses is refused by *MariaDB's* table rather than by
//! MySQL's.
//!
//! **SQL Server is the only driver whose TLS is not a socket upgrade.** § 3's
//! session is negotiated *inside* TDS: the handshake records ride PRELOGIN
//! messages one packet at a time and the stream goes raw again once the login
//! is sent, so `tds.rs`'s own cases hold each step of that tunnel against a
//! peer that agrees with whatever was written. What is left over is the same
//! question every other driver leaves over — that the tunnel completes against
//! a real SQL Server and that a statement then runs over it — and
//! `sys.dm_exec_connections` is where that server keeps its own view of the
//! socket, so it stands exactly where `pg_stat_ssl` and `Ssl_version` do.
//!
//! # Why the server is asked rather than the driver
//!
//! Every assertion below is the server's own answer, not this crate's.
//! `pg_stat_ssl` is PostgreSQL's view of the socket it is holding, so a row
//! saying `ssl` is the server reporting encryption on the connection this
//! process opened — a driver that had quietly fallen back to plaintext cannot
//! produce it, where an assertion made on our side of the wire would pass on
//! whatever we believed. `current_user` is the same shape for the other half,
//! and the refused password beside it is what makes it an *authentication*
//! rather than an admission: a server configured to trust every connection
//! would answer `current_user` just as well.
//!
//! MySQL's twins are the same facts asked of that server: its
//! `Ssl_version` **session** status variable, which is the version negotiated
//! on this session's own socket and is empty on one that never upgraded;
//! `CURRENT_USER()`; and a refusal that arrives as a [`ServerError`] rather
//! than as a sentence, so § 8's `SQLSTATE` and `driverCode` are read off the
//! access denial a real server sends rather than off a peer script that was
//! told what to send.
//!
//! # The skip rule rides with the endpoint
//!
//! A process that finds `NVS_DB_MATRIX_DRIVER` unset asserts nothing at all, so
//! `python tools/verify.py` is green on a machine with no containers and
//! `python tools/db-matrix.py` is what makes these assertions happen —
//! [`nvs_db::matrix`]'s module doc owns that rule and why a field that is *set
//! but unusable* panics instead.
//!
//! # Either transport, one case list
//!
//! `rule:core-classes/db-unix-socket-path` gives PostgreSQL, MySQL and MariaDB
//! a second way to reach the same server, and the only thing a case here has to
//! say about it is that the driver answers the same over both. So the leg the
//! harness pointed this process at is read once into a [`Leg`] — an
//! [`Endpoint`] and the credentials — and every case below runs whichever
//! transport was published without naming which it was. The anchor is the one
//! field that differs, because a socket has no TLS session to anchor and
//! `nvs_db::pg::PgStream` owns why.

use std::cell::Cell;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use nvs_db::conn::Endpoint;
use nvs_db::matrix::{self, Location, Server};
use nvs_db::mysql::scalar;
use nvs_db::tds::{TdsScalar, encode as tds_encode, scalar as tds_scalar};
use nvs_db::{
    DbErrorKind, Driver, Isolation, MariaConn, MariaTarget, MySqlConn, MySqlRows, MySqlScalar,
    MySqlTarget, PgConn, PgTarget, ServerError, TdsConn, TdsTarget,
};
use nvs_host::{Reactor, Scheduler, run_until_idle};
use nvs_runtime::{Ctx, NvsStr, OutputSink, TaskRoot, Value};

/// How long the whole of one handshake here may take.
///
/// Generous, because the server is a container the harness may have started
/// moments ago, and finite because a matrix leg that hangs reports nothing at
/// all — which is the outcome a verification matrix must not produce.
const DEADLINE: Duration = Duration::from_secs(10);

/// How long the statement in [`a_connection_read_parks_its_coroutine_rather_than_blocking_the_core`]
/// keeps the wire quiet.
///
/// Long enough that a companion task waking every [`TURN`] gets many turns
/// inside it, and short enough that the case costs half a second.
const QUIET: &str = "SELECT pg_sleep(0.5)";

/// [`QUIET`]'s twin for
/// [`a_mysql_connection_read_parks_its_coroutine_rather_than_blocking_the_core`],
/// keeping that wire quiet for the same half second.
///
/// `SLEEP` is what `pg_sleep` is. The cast is owed to the helper rather than to
/// the case: `SLEEP` answers a `BIGINT`, § 9 decodes that as
/// [`MySqlScalar::Int`], and [`mysql_one_value`] reads a text column — so the
/// statement asks for its answer in the one shape both drivers' helpers carry,
/// the way the `GROUP_CONCAT` below already does.
const MYSQL_QUIET: &str = "SELECT CAST(SLEEP(0.5) AS CHAR)";

/// The companion task's wake interval, and the resolution of the observation it
/// is making.
const TURN: Duration = Duration::from_millis(10);

/// The published server or the socket this process was pointed at, as
/// everything a case needs to reach it.
///
/// One shape for the two transports, so the second one runs the whole case
/// list rather than a copy of it: what differs between the legs is an
/// [`Endpoint`] and an anchor, and every case is written against the
/// credentials, which do not differ at all.
struct Leg {
    /// What a driver's target carries as its host: over TCP the published name
    /// the certificate is checked against, and over a socket the path an
    /// operator wrote, which nothing on that arm reads — `nvs_db::pg::PgStream`
    /// owns why there is no session there to check it against.
    host: String,
    /// The role the connection authenticates as, and what `current_user` and
    /// its twins below must answer.
    user: String,
    /// The password that opens this leg; the cases offer a wrong one beside it.
    password: String,
    /// The database the connection names.
    database: String,
    /// The anchor the harness exported for this run — `matrix`'s module doc
    /// owns why a published server's required group holds it rather than
    /// carrying it beside — and `None` over a socket, which has no session to
    /// anchor.
    ca: Option<PathBuf>,
    /// Where the driver dials, already the shape its `connect` takes.
    endpoint: Endpoint,
}

/// This process's leg for `driver`, or `None` because nothing pointed it at
/// that one.
///
/// Two shapes of `None` and neither is a failure: no harness at all, and a leg
/// testing one of the other drivers, which runs every test in this crate
/// including these.
///
/// `socket_endpoint` is the driver's own, because
/// `rule:core-classes/db-unix-socket-path` keeps each derivation with the
/// driver it belongs to — a directory for PostgreSQL, the file as written for
/// the two MySQL protocols — and a spelling here would be its second home.
fn leg_for(driver: Driver, socket_endpoint: fn(&str, u16) -> io::Result<Endpoint>) -> Option<Leg> {
    let endpoint = matrix::endpoint()?;
    if endpoint.driver != driver {
        return None;
    }

    Some(match endpoint.location {
        Location::Server(server) => Leg {
            endpoint: address(&server).into(),
            host: server.host,
            user: server.user,
            password: server.password,
            database: server.database,
            ca: Some(server.ca),
        },
        Location::Socket(socket) => Leg {
            endpoint: socket_endpoint(&socket.path, socket.port)
                .expect("the harness published a socket, so this build has that transport"),
            host: socket.path,
            user: socket.user,
            password: socket.password,
            database: socket.database,
            ca: None,
        },
        Location::File(path) => {
            unreachable!("SQLite is the only driver reached by a file, and {path:?} is not it")
        }
    })
}

/// This process's PostgreSQL leg, or `None` because nothing pointed it at one.
fn postgres() -> Option<Leg> {
    leg_for(Driver::Postgres, nvs_db::pg::socket_endpoint)
}

/// Is a session on this leg one TLS is a property of?
///
/// A published server's is, and a socket's is not:
/// `rule:core-classes/db-unix-socket-path`'s transport carries no session to
/// upgrade and nothing that could vouch for one, which is why [`Leg::ca`] is
/// `None` there. The cases asserting the upgrade ask this and return on the
/// socket leg, where every *other* case in this file runs again unchanged —
/// which is what that leg exists for.
fn upgrades(leg: &Leg) -> bool {
    leg.ca.is_some()
}

/// One handshake against `leg`, offering `password`, answering the way the
/// caller of a connection that may not open needs it.
///
/// The password is a parameter because the wrong one is half of what
/// [`a_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream`]
/// asserts: the two calls differ in that field alone, so what the server
/// refuses is the credential and nothing else about the connection.
fn connect_as(leg: &Leg, password: &str) -> io::Result<PgConn> {
    let target = PgTarget {
        host: &leg.host,
        user: &leg.user,
        password,
        database: &leg.database,
        tls_ca_file: leg.ca.as_deref(),
        time_zone: 0,
        statement_cache: 8,
    };

    PgConn::connect(
        leg.endpoint.clone(),
        &target,
        Some(Instant::now() + DEADLINE),
    )
}

/// Where the harness published `server`, as the address a driver connects to.
///
/// Resolved here and not inside either driver: `rule:core-classes/db-capabilities` has the *name* on
/// the target because that is what the certificate is checked against, and the
/// address arrives separately because in a request it is the one the
/// `db.connect` capability approved. A test that handed a driver a name to
/// resolve would be exercising a path no program can reach. A socket leg
/// resolves nothing at all, which is what the other arm of an [`Endpoint`] is.
fn address(server: &Server) -> SocketAddr {
    (server.host.as_str(), server.port)
        .to_socket_addrs()
        .expect("the matrix host is an address")
        .next()
        .expect("the matrix host resolves to somewhere")
}

/// A connection to `leg`, as a request that found the pool empty opens one.
fn open(leg: &Leg) -> PgConn {
    connect_as(leg, &leg.password)
        .expect("the matrix server accepts a handshake on the leg the harness published")
}

/// The first column of the first row `sql` returns, `None` for SQL `NULL` and
/// for a statement that returned no rows at all.
///
/// It drains to the end of the stream whatever it found: a connection is
/// poolable only at a message boundary, and an abandoned portal is not one.
fn one_value(conn: &mut PgConn, sql: &str) -> Option<String> {
    let mut rows = conn.query(sql, &[]).expect("the server ran the statement");
    let mut answer = None;
    let mut first = true;
    while let Some(row) = rows.next_row().expect("a row, or the end of the stream") {
        if first {
            first = false;
            answer = row
                .column(0)
                .expect("the row has a first column")
                .map(|body| {
                    String::from_utf8(body.to_vec()).expect("PostgreSQL's text format is UTF-8")
                });
        }
    }
    answer
}

/// This process's MySQL server, or `None` because nothing pointed it at one.
///
/// [`postgres`]'s twin, and the twinning is what makes the skip rule one rule:
/// every case in this crate runs on every leg of the matrix, and each asks for
/// the one driver it can assert about.
fn mysql() -> Option<Leg> {
    leg_for(Driver::MySql, nvs_db::mysql::socket_endpoint)
}

/// One MySQL handshake against `leg`, offering `password`.
///
/// [`connect_as`]'s twin, down to the password being a parameter for the same
/// reason: the refused credential below differs from the accepted one in that
/// field alone.
fn mysql_connect_as(leg: &Leg, password: &str) -> io::Result<MySqlConn> {
    let target = MySqlTarget {
        host: &leg.host,
        user: &leg.user,
        password,
        database: &leg.database,
        tls_ca_file: leg.ca.as_deref(),
        time_zone: 0,
        statement_cache: 8,
    };

    MySqlConn::connect(
        leg.endpoint.clone(),
        &target,
        Some(Instant::now() + DEADLINE),
    )
}

/// A MySQL connection to `leg`, as a request that found the pool empty opens
/// one.
fn mysql_open(leg: &Leg) -> MySqlConn {
    mysql_connect_as(leg, &leg.password)
        .expect("the matrix server accepts a handshake on the leg the harness published")
}

/// The first column of the first row `sql` returns, as text.
///
/// [`one_value`]'s twin over the binary protocol, so the column arrives already
/// typed and the text is read through § 9's own decoder rather than by parsing
/// the octets here: a statement that answered something other than a string
/// panics rather than rendering into one, since every caller below is asking a
/// server for a name or a version.
///
/// It drains to the end of the stream whatever it found, for [`one_value`]'s
/// reason — a connection is poolable only at a packet boundary.
fn mysql_one_value(conn: &mut MySqlConn, sql: &str) -> Option<String> {
    first_text(conn.query(sql, &[]).expect("the server ran the statement"))
}

/// The walk itself, shared by the two drivers that speak this result set.
///
/// [`MariaConn::query`] answers the *same* [`MySqlRows`] as [`MySqlConn::query`]
/// — one binary protocol, and § 9's decoder is one decoder — so this is the one
/// helper of the MariaDB set that is not a twin of a MySQL one. What that driver
/// does not share is above the framing, and that is what the cases below ask
/// about.
fn first_text(mut rows: MySqlRows<'_>) -> Option<String> {
    // Cloned out before the walk begins: the definitions describe every row,
    // and `next_row` needs the borrow they came from.
    let columns = rows.columns().to_vec();
    let mut answer = None;
    let mut first = true;
    while let Some(row) = rows.next_row().expect("a row, or the end of the stream") {
        if first {
            first = false;
            let value = row.value(0).expect("the row has a first column");
            answer = match scalar(&columns[0], value).expect("the column decodes under § 9") {
                MySqlScalar::Null => None,
                MySqlScalar::Text(text) => Some(text.to_owned()),
                other => panic!("the statement answered {other:?}, which is not text"),
            };
        }
    }
    answer
}

/// Runs `sql` to the end of its answer, for a statement whose point is what it
/// did rather than what it returned.
///
/// The drain is not optional even where no row can arrive: `rule:core-classes/db-statement-members` lets
/// one statement be in flight at a time, and the next call is refused until
/// this one's stream has ended.
fn mysql_run(conn: &mut MySqlConn, sql: &str) {
    mysql_try(conn, sql).expect("the server ran the statement");
}

/// [`mysql_run`] for a statement the server is expected to refuse.
///
/// The refusal is taken from either half of the sequence rather than from the
/// execute alone, because which of them carries it is the protocol's business:
/// a statement rejected at prepare answers on the way out, and one rejected by
/// a constraint answers in the packet that would have opened the result set.
fn mysql_try(conn: &mut MySqlConn, sql: &str) -> io::Result<()> {
    let mut rows = conn.query(sql, &[])?;
    while rows.next_row()?.is_some() {}
    Ok(())
}

/// This process's MariaDB server, or `None` because nothing pointed it at one.
///
/// [`mysql`]'s twin, and the one line that differs is the whole point of the
/// pair: the matrix runs one driver per process, and a MariaDB leg is not a
/// MySQL one however alike the wire is.
fn mariadb() -> Option<Leg> {
    // The socket derivation is `mysql`'s own, handed to both drivers rather
    // than written twice — `rule:core-classes/db-unix-socket-path` says so, and
    // the file as written is what either server binds.
    leg_for(Driver::MariaDb, nvs_db::mysql::socket_endpoint)
}

/// The zone [`a_mariadb_connection_declares_section_9s_zone_and_the_server_holds_it`]
/// declares, as § 9's seconds east of UTC.
///
/// `+01:30`, chosen because no container sets it and no half-hour zone is any
/// server's default: a driver that had sent nothing at all would leave the
/// session on the server's `SYSTEM` zone, and one that had sent a whole number
/// of hours would still agree with a fixture that only checked the sign.
const ZONE: i32 = 5_400;

/// One MariaDB handshake against `server`, offering `password` and declaring
/// `time_zone`.
///
/// [`mysql_connect_as`]'s twin, with § 9's zone lifted into a parameter for the
/// same reason the password is one: the two cases below differ in exactly one
/// field each, so what the server then reports is attributable to that field.
fn mariadb_connect_as(leg: &Leg, password: &str, time_zone: i32) -> io::Result<MariaConn> {
    let target = MariaTarget {
        host: &leg.host,
        user: &leg.user,
        password,
        database: &leg.database,
        tls_ca_file: leg.ca.as_deref(),
        time_zone,
        statement_cache: 8,
    };

    MariaConn::connect(
        leg.endpoint.clone(),
        &target,
        Some(Instant::now() + DEADLINE),
    )
}

/// A MariaDB connection that opened, in UTC.
fn mariadb_open(leg: &Leg) -> MariaConn {
    mariadb_connect_as(leg, &leg.password, 0)
        .expect("the matrix server accepts a handshake on the leg the harness published")
}

/// The first column of the first row `sql` returns, as text.
///
/// [`mysql_one_value`]'s twin, and both are the two lines around [`first_text`].
fn mariadb_one_value(conn: &mut MariaConn, sql: &str) -> Option<String> {
    first_text(conn.query(sql, &[]).expect("the server ran the statement"))
}

/// Runs `sql` to the end of its answer, for a statement whose point is what it
/// did rather than what it returned.
///
/// [`mysql_run`]'s twin, and the drain is not optional here for that helper's
/// reason: § 4 lets one statement be in flight at a time.
fn mariadb_run(conn: &mut MariaConn, sql: &str) {
    let mut rows = conn.query(sql, &[]).expect("the server ran the statement");
    while rows
        .next_row()
        .expect("a row, or the end of the stream")
        .is_some()
    {}
}

/// This process's SQL Server, or `None` because nothing pointed it at one.
///
/// [`mysql`]'s twin once more, and the twinning is the whole of the skip rule:
/// the matrix runs one driver per process, so this case runs on the MariaDB leg
/// as well and returns there without asserting.
///
/// It answers a [`Server`] where the three above answer a [`Leg`], because this
/// driver has one transport: `rule:core-classes/db-unix-socket-path` refuses a
/// path for TDS, and [`TdsConn::connect`] takes an address rather than an
/// [`Endpoint`] because of it.
fn mssql() -> Option<Server> {
    let endpoint = matrix::endpoint()?;
    if endpoint.driver != Driver::SqlServer {
        return None;
    }
    let Location::Server(server) = endpoint.location else {
        unreachable!("TDS speaks over TCP alone, and `matrix` stops a run scheduled otherwise")
    };
    Some(server)
}

/// One SQL Server handshake against `server`, offering `password`.
///
/// [`mysql_connect_as`]'s twin. The declared zone stays a constant where
/// [`mariadb_connect_as`] lifted it into a parameter, because on this backend
/// there is nothing to send it to: [`TdsTarget::time_zone`] governs decoding
/// alone, so a case that varied it would be asking this process a question
/// rather than the server.
fn mssql_connect_as(server: &Server, password: &str) -> io::Result<TdsConn> {
    let target = TdsTarget {
        host: &server.host,
        user: &server.user,
        password,
        database: &server.database,
        tls_ca_file: Some(server.ca.as_path()),
        time_zone: 0,
        statement_cache: 8,
    };

    TdsConn::connect(address(server), &target, Some(Instant::now() + DEADLINE))
}

/// A SQL Server connection to `server`, as a request that found the pool empty
/// opens one.
fn mssql_open(server: &Server) -> TdsConn {
    mssql_connect_as(server, &server.password)
        .expect("the matrix server accepts a handshake verified against its own anchor")
}

/// The first column of the first row `sql` returns, as text.
///
/// [`mysql_one_value`]'s twin, and the one place the twinning stops at the
/// shape: MariaDB could share [`first_text`] because it speaks MySQL's result
/// set, and TDS is a different one with its own decoder, so this walk is
/// written out rather than borrowed.
fn mssql_one_value(conn: &mut TdsConn, sql: &str) -> Option<String> {
    let mut rows = conn.query(sql, &[]).expect("the server ran the statement");
    // Cloned out before the walk begins, for [`first_text`]'s reason: the
    // definitions describe every row, and `next_row` needs the borrow they came
    // from.
    let columns = rows.columns().to_vec();
    let mut answer = None;
    let mut first = true;
    while let Some(row) = rows.next_row().expect("a row, or the end of the stream") {
        if first {
            first = false;
            let value = row.column(0).expect("the row has a first column");
            answer = match tds_scalar(&columns[0], value).expect("the column decodes under § 9") {
                TdsScalar::Null => None,
                TdsScalar::Text(text) => Some(text.into_owned()),
                other => panic!("the statement answered {other:?}, which is not text"),
            };
        }
    }
    answer
}

/// One statement run for its effect, over a real SQL Server.
///
/// [`mysql_run`]'s twin, spelled as § 4's `executeMany` with a single empty set
/// — which is how this crate's surface says *run this once and bind nothing*,
/// there being no `execute` on [`TdsConn`] to say it more directly.
fn mssql_run(conn: &mut TdsConn, sql: &str) {
    conn.execute_many(sql, &[&[]])
        .unwrap_or_else(|refused| panic!("the server refused `{sql}`: {refused}"));
}

/// § 3: the connection this driver opens is TLS-wrapped and authenticated, and
/// the server is the one that says so.
///
/// Each question is answered by a fact only the other end holds.
/// `pg_stat_ssl` reports the version negotiated on *this* backend's socket, so a
/// row at all is the upgrade having happened and the version is which one; a
/// driver that had used the connection in the clear — what `sslmode=prefer`
/// does and what § 3 refuses — answers nothing here. `current_user` is the role
/// the startup message named, arriving back through the encrypted session. And
/// the same handshake with a password the server cannot verify is refused
/// `28P01`, which is what makes the first two an authentication rather than an
/// admission: against a server configured to trust every connection the case
/// above it would read identically.
///
/// *Which* stream all of this ran over is not asserted because it cannot vary:
/// `PgConn`'s wire is `Wire<NvsTls<NvsTcp>>` at the default type parameter, so
/// the parking stream is a compile-time property of the type and the runtime
/// question is only whether the sequence completes over it. That it *parks* is
/// [`a_connection_read_parks_its_coroutine_rather_than_blocking_the_core`]'s.
#[test]
fn a_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream() {
    let Some(server) = postgres() else {
        return;
    };
    if !upgrades(&server) {
        return;
    }
    let mut conn = open(&server);

    let version = one_value(
        &mut conn,
        "SELECT version FROM pg_stat_ssl WHERE pid = pg_backend_pid() AND ssl",
    )
    .expect("the server holds this session on an encrypted socket");
    assert!(
        version.starts_with("TLSv1."),
        "the session negotiated `{version}`, which is not a TLS version this connection should hold",
    );

    assert_eq!(
        one_value(&mut conn, "SELECT current_user").as_deref(),
        Some(server.user.as_str()),
        "the session is authenticated as the role the connection's block names",
    );

    let refused = connect_as(&server, "not-the-password")
        .expect_err("a password the server cannot verify opened a connection");
    let said = refused.to_string();
    assert!(
        said.contains("28P01"),
        "a wrong password was refused, but not by the server's own check: {said}",
    );
}

/// § 3, the half a real server is needed for: a read this connection is waiting
/// on parks the coroutine, so the core turns another task in the meantime.
///
/// `nvs_host::net`'s own cases hold that an `NvsTcp` read with nothing to read
/// registers with the reactor and yields. What they cannot hold is that the
/// property survives everything stacked on top of it here — a `rustls` session
/// that reads in records rather than in whatever arrived, and a state machine
/// that reads until a message is whole. Either could turn a park back into a
/// spin, and the failure would be invisible to every case in this crate that
/// scripts a peer, because a scripted peer always has the next byte ready.
///
/// **The assertion is a turn taken by somebody else while the read was
/// outstanding.** The companion task counts only the wakes that land between
/// the statement leaving and its answer arriving, which is why a driver that
/// held the core would score zero rather than merely fewer: with the core held,
/// this task's first turn comes after the query has already finished. Both
/// tasks are spawned on one scheduler, and the connection is opened inside the
/// first of them so that every read in the case — the handshake's included — is
/// on a core, which is where a worker or a request has them.
#[test]
fn a_connection_read_parks_its_coroutine_rather_than_blocking_the_core() {
    let Some(server) = postgres() else {
        return;
    };

    let mut sched = Scheduler::new();
    let _installed = nvs_host::reactor::install(Reactor::new().expect("the OS refused a poll"));

    // The query is in flight, the query is over: two flags rather than one, so
    // the companion below can tell "not yet" from "already done" and count
    // neither.
    let running = Rc::new(Cell::new(false));
    let done = Rc::new(Cell::new(false));
    let turns = Rc::new(Cell::new(0_u32));

    let in_flight = Rc::clone(&running);
    let finished = Rc::clone(&done);
    sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, move |_ctx| {
        let mut conn = open(&server);
        in_flight.set(true);
        one_value(&mut conn, QUIET);
        in_flight.set(false);
        finished.set(true);
    });

    let observed = Rc::clone(&turns);
    let in_flight = Rc::clone(&running);
    let finished = Rc::clone(&done);
    sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, move |_ctx| {
        // Bounded rather than `while !finished`: a first task that never
        // completes must not turn this one into a scheduler that never goes
        // idle, and the count below says what happened either way.
        for _ in 0..400 {
            if finished.get() {
                break;
            }
            if in_flight.get() {
                observed.set(observed.get() + 1);
            }
            nvs_host::sleep(TURN);
        }
    });

    run_until_idle(&mut sched).expect("the loop failed");

    assert!(
        done.get(),
        "the task holding the connection never finished its statement",
    );
    assert!(
        turns.get() > 0,
        "no other task was turned while a connection's read was outstanding, so the read held the \
         core instead of parking on it",
    );
}

/// § 3 on the other driver: the connection MySQL's handshake opens is
/// TLS-wrapped and authenticated, and the server is the one that says so.
///
/// [`a_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream`]'s
/// questions, asked of MySQL. `Ssl_version` is a **session** status
/// variable, so it is this connection's socket and not the server's
/// configuration, and it is the empty string on a session that stayed in the
/// clear — which is what a driver that had let § 3's upgrade be skipped would
/// produce. `CURRENT_USER()` is the account the handshake response named,
/// arriving back inside the encrypted session; its `@host` half is cut off
/// because the server fills it with the pattern the grant was written for
/// rather than with anything this process knows.
///
/// The refusal is read as a [`ServerError`] rather than as a sentence, which is
/// where this differs from the PostgreSQL case above and is not a difference in
/// taste: § 8's kind is keyed on MySQL's **vendor integer**, `mysql.rs`'s own
/// cases hold that table against a scripted peer, and a table is only as right
/// as the codes a server actually sends. This is the one place a real one says
/// what an access denial is.
#[test]
fn a_mysql_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream() {
    let Some(server) = mysql() else {
        return;
    };
    if !upgrades(&server) {
        return;
    }
    let mut conn = mysql_open(&server);

    let version = mysql_one_value(
        &mut conn,
        "SELECT VARIABLE_VALUE FROM performance_schema.session_status \
         WHERE VARIABLE_NAME = 'Ssl_version'",
    )
    .expect("the server holds a session status row for this connection");
    assert!(
        version.starts_with("TLSv1."),
        "the session negotiated `{version}`, which is not a TLS version this connection should hold",
    );

    assert_eq!(
        mysql_one_value(&mut conn, "SELECT SUBSTRING_INDEX(CURRENT_USER(), '@', 1)").as_deref(),
        Some(server.user.as_str()),
        "the session is authenticated as the account the connection's block names",
    );

    let refused = mysql_connect_as(&server, "not-the-password")
        .expect_err("a password the server cannot verify opened a connection");
    let refusal = ServerError::of(&refused).unwrap_or_else(|| {
        panic!("a wrong password was refused, but not by the server's own check: {refused}")
    });
    assert_eq!(
        (refusal.sql_state.as_str(), refusal.driver_code),
        ("28000", Some(1045)),
        "the refusal is the server's access denial, carrying § 8's raw pair: {refusal}",
    );
}

/// § 7 against a real MySQL: a nested `transaction()` is a `SAVEPOINT`, and
/// rolling back to it undoes its own work and none of the work around it.
///
/// `mysql.rs`'s own cases hold which *command* each depth is given, against a
/// peer that agrees with whatever was written. What only a server can say is
/// what those commands then did to the rows, and the three inserts here are
/// arranged so that a driver which had sent `ROLLBACK` where § 7 asks for
/// `ROLLBACK TO SAVEPOINT` — the failure a scripted peer cannot notice, because
/// it has no rows — loses the first row as well and answers `3` rather than
/// `1,3`.
///
/// The table is `TEMPORARY` so it belongs to this session and needs no cleanup
/// a failing assertion could skip, and it is created before the transaction
/// opens because MySQL's implicit commits are a property of DDL and not
/// something this case is asking about.
#[test]
fn a_mysql_transaction_nests_to_a_savepoint_and_rolls_back_to_it() {
    let Some(server) = mysql() else {
        return;
    };
    let mut conn = mysql_open(&server);
    mysql_run(
        &mut conn,
        "CREATE TEMPORARY TABLE novis_nesting (id INT PRIMARY KEY)",
    );

    assert_eq!(
        conn.depth(),
        0,
        "a fresh connection is inside no transaction"
    );

    conn.begin(None, false).expect("the server began one");
    assert_eq!(conn.depth(), 1, "the outermost level is a transaction");
    mysql_run(&mut conn, "INSERT INTO novis_nesting (id) VALUES (1)");

    conn.begin(None, false)
        .expect("the server took a savepoint");
    assert_eq!(conn.depth(), 2, "a nested level is a savepoint inside it");
    mysql_run(&mut conn, "INSERT INTO novis_nesting (id) VALUES (2)");

    conn.roll_back().expect("the server rolled back to it");
    assert_eq!(conn.depth(), 1, "the outer transaction is still open");
    mysql_run(&mut conn, "INSERT INTO novis_nesting (id) VALUES (3)");

    conn.commit().expect("the server committed");
    assert_eq!(
        conn.depth(),
        0,
        "the connection is outside a transaction again"
    );

    assert_eq!(
        mysql_one_value(
            &mut conn,
            "SELECT CAST(GROUP_CONCAT(id ORDER BY id) AS CHAR) FROM novis_nesting",
        )
        .as_deref(),
        Some("1,3"),
        "the committed rows are the ones outside the savepoint that was rolled back",
    );
}

/// § 8 against a real MySQL: a duplicate key is `1062`, `23000` and
/// [`DbErrorKind::UniqueViolation`], and the server is what says the first two.
///
/// `mysql.rs`'s code table is asserted there as a table, against a `Peer` that
/// was handed the very codes the table names — an agreement that holds however
/// wrong the codes are. This asks the server to produce one, so the entry every
/// application branches on is anchored to what MySQL actually sends rather than
/// to what the table's author believed. The `SQLSTATE` rides along because it
/// is the half that is *not* keyed on here: § 8's kind comes from the integer
/// precisely because `23000` is one of the few classes MySQL fills honestly,
/// and a case that asserted only the kind would not notice the pair coming
/// apart.
#[test]
fn a_mysql_duplicate_key_carries_section_8s_kind_and_the_servers_own_code() {
    let Some(server) = mysql() else {
        return;
    };
    let mut conn = mysql_open(&server);
    mysql_run(
        &mut conn,
        "CREATE TEMPORARY TABLE novis_unique (id INT PRIMARY KEY)",
    );
    mysql_run(&mut conn, "INSERT INTO novis_unique (id) VALUES (1)");

    let refused = mysql_try(&mut conn, "INSERT INTO novis_unique (id) VALUES (1)")
        .expect_err("a second row with the same primary key was accepted");
    let refusal = ServerError::of(&refused).unwrap_or_else(|| {
        panic!("the duplicate key was refused, but not as a server error: {refused}")
    });
    assert_eq!(
        (
            refusal.kind,
            refusal.driver_code,
            refusal.sql_state.as_str()
        ),
        (DbErrorKind::UniqueViolation, Some(1062), "23000"),
        "the server's duplicate key did not land on § 8's row for it: {refusal}",
    );
}

/// § 3 on the other driver: a read a MySQL connection is waiting on parks the
/// coroutine, so the core turns another task in the meantime.
///
/// [`a_connection_read_parks_its_coroutine_rather_than_blocking_the_core`]'s
/// question, asked of MySQL, and it is a separate case rather than a second leg
/// of that one because the stack under the park is not the same stack. Both
/// drivers sit on `NvsTls<NvsTcp>`, so the socket and the `rustls` session are
/// shared, but everything above them is written per driver: MySQL reads a
/// length-prefixed packet header and then its body, where PostgreSQL reads a
/// tagged message, and § 1's `COM_STMT_PREPARE`/`COM_STMT_EXECUTE` pair means a
/// first execution waits on the wire twice rather than once. A read-until-whole
/// loop that had turned into a spin would be invisible to `mysql.rs`'s own
/// cases for the reason the PostgreSQL twin gives: a scripted peer always has
/// the next byte ready.
///
/// The assertion is the same one and is made the same way — a turn taken by
/// somebody else strictly between the statement leaving and its answer
/// arriving, so a driver holding the core scores zero rather than merely fewer.
/// The connection is opened inside the spawned task for the same reason too:
/// the handshake's own reads are then on a core, which is where a worker or a
/// request has them, and here they are two round trips more than PostgreSQL's.
#[test]
fn a_mysql_connection_read_parks_its_coroutine_rather_than_blocking_the_core() {
    let Some(server) = mysql() else {
        return;
    };

    let mut sched = Scheduler::new();
    let _installed = nvs_host::reactor::install(Reactor::new().expect("the OS refused a poll"));

    // The query is in flight, the query is over: two flags rather than one, so
    // the companion below can tell "not yet" from "already done" and count
    // neither.
    let running = Rc::new(Cell::new(false));
    let done = Rc::new(Cell::new(false));
    let turns = Rc::new(Cell::new(0_u32));

    let in_flight = Rc::clone(&running);
    let finished = Rc::clone(&done);
    sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, move |_ctx| {
        let mut conn = mysql_open(&server);
        in_flight.set(true);
        mysql_one_value(&mut conn, MYSQL_QUIET);
        in_flight.set(false);
        finished.set(true);
    });

    let observed = Rc::clone(&turns);
    let in_flight = Rc::clone(&running);
    let finished = Rc::clone(&done);
    sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, move |_ctx| {
        // Bounded rather than `while !finished`: a first task that never
        // completes must not turn this one into a scheduler that never goes
        // idle, and the count below says what happened either way.
        for _ in 0..400 {
            if finished.get() {
                break;
            }
            if in_flight.get() {
                observed.set(observed.get() + 1);
            }
            nvs_host::sleep(TURN);
        }
    });

    run_until_idle(&mut sched).expect("the loop failed");

    assert!(
        done.get(),
        "the task holding the connection never finished its statement",
    );
    assert!(
        turns.get() > 0,
        "no other task was turned while a MySQL connection's read was outstanding, so the read \
         held the core instead of parking on it",
    );
}

/// § 13 against a real MySQL: after the reset there is no temporary table, no
/// session variable, no open transaction and no prepared statement the cache
/// still believes in.
///
/// `mysql.rs`'s own cases hold that `reset` sends `COM_RESET_CONNECTION` and
/// then re-sends the declared zone, against a peer that answers `OK` to
/// whatever arrives. § 13 deliberately states the reset as a **property** and
/// not as that command list — "a backend added later satisfies that property or
/// is not pooled" — and a property about what a *session* no longer holds is
/// one only the session's own server can answer. This is where the two meet:
/// the same connection, before and after, asked what survived.
///
/// The prepared-statement half is the one that needs its shape explaining.
/// § 1's cache is keyed on SQL text, and `COM_RESET_CONNECTION` drops the
/// server's side of it — § 13 names that asymmetry with PostgreSQL as the
/// protocol's rather than a choice. So the probe below is run *twice, spelled
/// identically*: a driver that had kept the cache across the reset would send a
/// `COM_STMT_EXECUTE` naming a statement id the server has already forgotten,
/// and the server would refuse it with `1243`. The second run therefore proves
/// the cache was invalidated by succeeding at all, and proves the session
/// variable is gone by answering `NULL` — one statement for the two, which is
/// also why it is a `CAST(… AS CHAR)`: a user variable's own column type is not
/// something this case wants to be asserting about.
#[test]
fn a_mysql_reset_leaves_no_temporary_table_variable_or_cached_statement() {
    let Some(server) = mysql() else {
        return;
    };
    // Spelled once and used twice on purpose — the two runs must hash to the
    // same entry of § 1's cache or the paragraph above is asserting nothing.
    const PROBE: &str = "SELECT CAST(@novis_reset AS CHAR)";

    let mut conn = mysql_open(&server);
    mysql_run(
        &mut conn,
        "CREATE TEMPORARY TABLE novis_reset (id INT PRIMARY KEY)",
    );
    mysql_run(&mut conn, "SET @novis_reset = 'before'");
    assert_eq!(
        mysql_one_value(&mut conn, PROBE).as_deref(),
        Some("before"),
        "the session variable this case is about was never set on the session",
    );

    // Reset from *inside* a transaction: a connection is released at teardown
    // whatever it was in the middle of, and § 13's first property is that the
    // next request does not inherit it.
    conn.begin(None, false).expect("the server began one");
    assert_eq!(conn.depth(), 1, "the reset below runs inside a transaction");

    let mut conn = conn.reset().expect("the server accepted the reset");

    assert_eq!(
        conn.depth(),
        0,
        "the reset left the connection believing it was still inside a transaction",
    );
    assert_eq!(
        mysql_one_value(&mut conn, PROBE),
        None,
        "a session variable set before the reset was still readable after it",
    );

    let refused = mysql_try(&mut conn, "SELECT id FROM novis_reset")
        .expect_err("a temporary table created before the reset survived it");
    let refusal = ServerError::of(&refused).unwrap_or_else(|| {
        panic!("the missing table was refused, but not by the server's own check: {refused}")
    });
    assert_eq!(
        (refusal.sql_state.as_str(), refusal.driver_code),
        ("42S02", Some(1146)),
        "the refusal is not the server saying the temporary table no longer exists: {refusal}",
    );
}

/// § 3 against a real MariaDB: the connection this driver opens is TLS-wrapped
/// and authenticated, and the server is the one that says so.
///
/// [`a_mysql_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream`]'s
/// twin, asking the same facts of the other server, and the twinning is
/// what makes it worth writing: `maria.rs` reuses `mysql.rs`'s framing and
/// stops there, so everything this case exercises above the packet header — the
/// plugin the greeting names, the proof sent back for it, the code table the
/// refusal is read through — is MariaDB's own and has never met a MySQL server.
///
/// The status variable is read out of `information_schema.SESSION_STATUS` where
/// the MySQL twin reads `performance_schema.session_status`: the second is
/// MySQL 5.7's replacement for the first, and MariaDB, whose `performance_schema`
/// predates it, kept the original. Either way the row is that server's view of
/// the socket this process is holding, so a driver that had fallen back to
/// plaintext cannot produce it.
///
/// The refusal is asserted as § 8's *kind* and not only as the raw pair, because
/// the kind is the half a shared table would get wrong: MariaDB's `1045` and
/// MySQL's happen to agree, and
/// [`mariadb_uses_its_own_code_table_and_not_mysqls`](/crates/nvs-db/src/maria.rs) is where
/// the codes that do not are held.
#[test]
fn a_mariadb_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream() {
    let Some(server) = mariadb() else {
        return;
    };
    if !upgrades(&server) {
        return;
    }
    let mut conn = mariadb_open(&server);

    let version = mariadb_one_value(
        &mut conn,
        "SELECT VARIABLE_VALUE FROM information_schema.SESSION_STATUS \
         WHERE VARIABLE_NAME = 'Ssl_version'",
    )
    .expect("the server holds a session status row for this connection");
    assert!(
        version.starts_with("TLSv1."),
        "the session negotiated `{version}`, which is not a TLS version this connection should hold",
    );

    assert_eq!(
        mariadb_one_value(&mut conn, "SELECT SUBSTRING_INDEX(CURRENT_USER(), '@', 1)").as_deref(),
        Some(server.user.as_str()),
        "the session is authenticated as the account the connection's block names",
    );

    let refused = mariadb_connect_as(&server, "not-the-password", 0)
        .expect_err("a password the server cannot verify opened a connection");
    let refusal = ServerError::of(&refused).unwrap_or_else(|| {
        panic!("a wrong password was refused, but not by the server's own check: {refused}")
    });
    assert_eq!(
        (
            refusal.kind,
            refusal.sql_state.as_str(),
            refusal.driver_code
        ),
        (DbErrorKind::Permission, "28000", Some(1045)),
        "the refusal is not MariaDB's own access denial, normalised by its own table: {refusal}",
    );
}

/// § 9: the zone a connection declares is the zone the server then holds, and
/// the driver keeps the same number to decode zone-less columns with.
///
/// The declared zone is the one field of § 9's map that is not a property of a
/// column, so it is the one a scripted peer cannot check: `mysql.rs`'s cases
/// hold that `SET time_zone` is sent and what its literal reads, and only a
/// server can say that the session moved. Both halves are asked here — the
/// variable, which is the server repeating the literal back, and `TIMEDIFF`
/// against `UTC_TIMESTAMP()`, which is the offset actually applied to a value
/// the server rendered. A driver that had sent the statement and ignored a
/// refusal passes the first and fails the second.
///
/// [`MariaConn::time_zone`] is asked alongside them: it is what a layer up
/// decodes a zone-less `DATETIME` with, and a connection whose record of the
/// zone had drifted from the session's would read every such column wrong and
/// silently.
#[test]
fn a_mariadb_connection_declares_section_9s_zone_and_the_server_holds_it() {
    let Some(server) = mariadb() else {
        return;
    };
    let mut conn = mariadb_connect_as(&server, &server.password, ZONE)
        .expect("the matrix server accepts a handshake declaring a zone");

    assert_eq!(
        conn.time_zone(),
        ZONE,
        "the connection's own record of § 9's zone is not the one the handshake declared",
    );
    assert_eq!(
        mariadb_one_value(&mut conn, "SELECT @@session.time_zone").as_deref(),
        Some("+01:30"),
        "the session is not in the zone the handshake declared",
    );
    assert_eq!(
        mariadb_one_value(
            &mut conn,
            "SELECT CAST(TIMEDIFF(NOW(), UTC_TIMESTAMP()) AS CHAR)"
        )
        .as_deref(),
        Some("01:30:00"),
        "the server rendered a zone-less value at an offset that is not the declared one",
    );
}

/// The first divergence with teeth: MariaDB answers `INSERT … RETURNING` and
/// MySQL refuses the word, over one driver's framing and two code tables.
///
/// This is why `rule:core-classes/db-one-api` makes MariaDB its own driver rather than a MySQL flag,
/// and it is asserted as *one* case over two legs rather than as two cases that
/// never meet. The DDL, the statement and the expectation are spelled once; the
/// only thing that varies between the two runs is which server the matrix
/// pointed this process at. A driver built as `MySql { mariadb: true }` passes
/// either half alone by branching, and fails here the day the branch is wrong,
/// because both halves are reading the same two constants.
///
/// The refusal is the server's own — `1064`, the parse error, arriving from the
/// `COM_STMT_PREPARE` rather than from the execute, since MySQL cannot get as
/// far as a statement to run. That it is `Syntax` and not `Permission` is § 8's
/// normalisation of it, and each driver reaches that kind through its own table.
#[test]
fn mariadb_returning_is_available_and_mysqls_is_not() {
    // Spelled once for the two legs: the case is asserting that one statement
    // is answered by one server and refused by the other, so a second spelling
    // would be asserting about two statements.
    const TABLE: &str = "CREATE TEMPORARY TABLE novis_returning \
                         (id INT PRIMARY KEY AUTO_INCREMENT, v VARCHAR(8))";
    const RETURNING: &str = "INSERT INTO novis_returning (v) VALUES ('one') \
                             RETURNING CAST(id AS CHAR)";

    if let Some(server) = mariadb() {
        let mut conn = mariadb_open(&server);
        mariadb_run(&mut conn, TABLE);
        assert_eq!(
            mariadb_one_value(&mut conn, RETURNING).as_deref(),
            Some("1"),
            "MariaDB's `RETURNING` did not hand back the row the insert had just written",
        );
        return;
    }

    let Some(server) = mysql() else {
        return;
    };
    let mut conn = mysql_open(&server);
    mysql_run(&mut conn, TABLE);
    let refused = mysql_try(&mut conn, RETURNING)
        .expect_err("MySQL ran a statement only MariaDB's dialect has");
    let refusal = ServerError::of(&refused).unwrap_or_else(|| {
        panic!("`RETURNING` was refused, but not by the server's own check: {refused}")
    });
    assert_eq!(
        (
            refusal.kind,
            refusal.sql_state.as_str(),
            refusal.driver_code
        ),
        (DbErrorKind::Syntax, "42000", Some(1064)),
        "the refusal is not MySQL's own parse error, normalised by MySQL's table: {refusal}",
    );
}

/// § 3 against a real SQL Server: the session is the TLS one this driver
/// tunnelled, it is authenticated as the login the block names, and the
/// statement that reports both ran over it.
///
/// [`a_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream`]'s
/// questions, asked of the one driver whose TLS is not a socket upgrade.
/// `encrypt_option` is the server's own view of the connection this process
/// opened, and it reads `TRUE` only for a session that server is itself
/// encrypting — a driver that had left the tunnel and carried on in the clear,
/// which § 3 has no spelling for, cannot produce it. `SUSER_SNAME()` is the
/// login LOGIN7 named, arriving back through that session. And the same
/// handshake offering a password the server cannot verify is refused `18456`,
/// which is what makes the first two an authentication rather than an
/// admission: against a server that accepted anything, the two lines above
/// would read identically.
///
/// **The first assertion is also § 1's one statement.** `sp_prepexec` is how
/// [`TdsConn::query`] asks it, so a green leg here has carried a prepare, an
/// execution and a result set back through the tunnel rather than only a login
/// — which is the half `tds.rs`'s scripted peer answers about itself.
#[test]
fn a_mssql_connection_is_opened_tls_tunnelled_and_authenticated_over_the_parking_stream() {
    let Some(server) = mssql() else {
        return;
    };
    let mut conn = mssql_open(&server);

    assert_eq!(
        mssql_one_value(
            &mut conn,
            "SELECT encrypt_option FROM sys.dm_exec_connections WHERE session_id = @@SPID",
        )
        .as_deref(),
        Some("TRUE"),
        "the server reports this session's own socket as encrypted",
    );

    assert_eq!(
        mssql_one_value(&mut conn, "SELECT SUSER_SNAME()").as_deref(),
        Some(server.user.as_str()),
        "the session is authenticated as the login the connection's block names",
    );

    let refused = mssql_connect_as(&server, "not-the-password")
        .expect_err("a password the server cannot verify opened a connection");
    let refusal = ServerError::of(&refused).unwrap_or_else(|| {
        panic!("a wrong password was refused, but not by the server's own check: {refused}")
    });
    assert_eq!(
        (
            refusal.kind,
            refusal.sql_state.as_str(),
            refusal.driver_code
        ),
        (DbErrorKind::Permission, "", Some(18456)),
        "the refusal is the server's own login failure, and TDS has no SQLSTATE to carry: {refusal}",
    );
}

/// § 7 against a real SQL Server: a nested `transaction()` is a
/// `SAVE TRANSACTION`, rolling back to it undoes its own work and none of the
/// work around it — and every request inside the transaction names it.
///
/// [`a_mysql_transaction_nests_to_a_savepoint_and_rolls_back_to_it`]'s rows,
/// asked of the backend where an open transaction is more than the commands
/// that opened it. TDS answers a `BEGIN TRANSACTION` with a **transaction
/// descriptor** in an `ENVCHANGE`, and refuses every later request whose
/// `ALL_HEADERS` does not carry it — driver code 3989, *new request is not
/// allowed to start because it should come with valid transaction descriptor*.
/// So the first `INSERT` below is the assertion `tds.rs`'s scripted peer cannot
/// make: that peer answers whatever was written to it, and a driver writing a
/// zero descriptor is green there while being unable to run one statement
/// inside a transaction here.
///
/// The table is permanent, and dropped on the way **in** rather than out: a
/// failing assertion skips its own cleanup, and this database is the matrix's
/// own.
#[test]
fn a_mssql_transaction_nests_to_a_savepoint_and_rolls_back_to_it() {
    let Some(server) = mssql() else {
        return;
    };
    let mut conn = mssql_open(&server);
    mssql_run(&mut conn, "DROP TABLE IF EXISTS novis_nesting");
    mssql_run(&mut conn, "CREATE TABLE novis_nesting (id INT PRIMARY KEY)");

    assert_eq!(
        conn.depth(),
        0,
        "a fresh connection is inside no transaction"
    );

    conn.begin(None, false).expect("the server began one");
    assert_eq!(conn.depth(), 1, "the outermost level is a transaction");
    mssql_run(&mut conn, "INSERT INTO novis_nesting (id) VALUES (1)");

    conn.begin(None, false)
        .expect("the server took a savepoint");
    assert_eq!(conn.depth(), 2, "a nested level is a savepoint inside it");
    mssql_run(&mut conn, "INSERT INTO novis_nesting (id) VALUES (2)");

    conn.roll_back().expect("the server rolled back to it");
    assert_eq!(conn.depth(), 1, "the outer transaction is still open");
    mssql_run(&mut conn, "INSERT INTO novis_nesting (id) VALUES (3)");

    conn.commit().expect("the server committed");
    assert_eq!(
        conn.depth(),
        0,
        "the connection is outside a transaction again"
    );

    assert_eq!(
        mssql_one_value(
            &mut conn,
            "SELECT STRING_AGG(CAST(id AS VARCHAR(10)), ',') WITHIN GROUP (ORDER BY id) \
             FROM novis_nesting",
        )
        .as_deref(),
        Some("1,3"),
        "the committed rows are the ones outside the savepoint that was rolled back",
    );
}

/// § 13 against a real SQL Server: a connection released from *inside* a
/// transaction comes back with no transaction, at the login's own isolation
/// level, and able to run a statement at all.
///
/// [`a_mysql_reset_leaves_no_temporary_table_variable_or_cached_statement`]'s
/// property, asked where this backend's own facts make it a different
/// question. `SET TRANSACTION ISOLATION LEVEL` is **session**-scoped here, so a
/// level asked for by one request outlives the transaction that asked for it and
/// would silently become the next request's — `sp_reset_connection` putting it
/// back is what § 13's property means on this driver, and `sys.dm_exec_sessions`
/// is the server's own view of it rather than the driver's own flag. And the
/// reset is the one request that names **no** transaction however deep the
/// session was: the header bit is honoured before the message it rides is
/// processed, so a driver still naming the rolled-back descriptor would be
/// refused here — which is why the probes after the reset are the assertion and
/// not housekeeping.
#[test]
fn a_mssql_reset_from_inside_a_transaction_leaves_none_and_the_logins_level() {
    let Some(server) = mssql() else {
        return;
    };
    // The server's own name for the level this session is sitting at, which is
    // what `TdsConn::isolation_moved` is a claim about.
    const LEVEL: &str = "SELECT CASE transaction_isolation_level \
                         WHEN 1 THEN 'ReadUncommitted' WHEN 2 THEN 'ReadCommitted' \
                         WHEN 3 THEN 'RepeatableRead' WHEN 4 THEN 'Serializable' \
                         WHEN 5 THEN 'Snapshot' ELSE 'Unspecified' END \
                         FROM sys.dm_exec_sessions WHERE session_id = @@SPID";
    const OPEN: &str = "SELECT CAST(@@TRANCOUNT AS VARCHAR(10))";

    let mut conn = mssql_open(&server);
    conn.begin(Some(Isolation::Serializable), false)
        .expect("the server began one at the level it was asked for");
    assert_eq!(conn.depth(), 1, "the reset below runs inside a transaction");
    assert_eq!(
        mssql_one_value(&mut conn, LEVEL).as_deref(),
        Some("Serializable"),
        "the session is not at the level § 7's option asked for, so the restore proves nothing",
    );
    assert_eq!(
        mssql_one_value(&mut conn, OPEN).as_deref(),
        Some("1"),
        "the server does not agree that a transaction is open",
    );

    let mut conn = conn.reset().expect("the server accepted the reset");

    assert_eq!(
        conn.depth(),
        0,
        "the reset left the connection believing it was still inside a transaction",
    );
    assert_eq!(
        mssql_one_value(&mut conn, OPEN).as_deref(),
        Some("0"),
        "a transaction open before the reset was still open after it",
    );
    assert_eq!(
        mssql_one_value(&mut conn, LEVEL).as_deref(),
        Some("ReadCommitted"),
        "the session kept the level one request asked for, and the next request would inherit it",
    );
}

/// § 9's `bytes` row against a real SQL Server: the octets a program bound are
/// stored in a `varbinary` column and come back equal, which is the half
/// `tds::plan`'s `tds_round_trips_a_bytes_through_varbinary` cannot ask — a
/// scripted server answers with whatever the case wrote for it.
///
/// The octets are what a text bind destroys: a zero, a high byte and a lone
/// `0x80`, none of which is UTF-8 or survives a UCS-2 widening, so a driver
/// that had declared the marker `nvarchar` cannot produce this row rather than
/// producing a different one. `VARBINARY(MAX)` is the column so the bind is the
/// only thing under test and not the width the server chose.
#[test]
fn a_mssql_bytes_binds_as_a_varbinary_and_comes_back_equal() {
    let Some(server) = mssql() else {
        return;
    };
    let octets: [u8; 5] = [0x00, 0x61, 0xFF, 0xFE, 0x80];
    let bound = tds_encode(Value::bytes(NvsStr::new(&octets)))
        .expect("a `bytes` § 9 binds")
        .expect("a value and not SQL NULL");

    let mut conn = mssql_open(&server);
    mssql_run(&mut conn, "DROP TABLE IF EXISTS novis_octets");
    mssql_run(
        &mut conn,
        "CREATE TABLE novis_octets (v VARBINARY(MAX) NOT NULL)",
    );

    let set: [Option<&[u8]>; 1] = [Some(&bound)];
    let sets: [&[Option<&[u8]>]; 1] = [&set];
    assert_eq!(
        conn.execute_many("INSERT INTO novis_octets (v) VALUES (@p1)", &sets)
            .expect("the server took the bound octets"),
        1,
        "the insert bound one value and wrote one row",
    );

    let mut rows = conn
        .query("SELECT v FROM novis_octets", &[])
        .expect("the server ran the statement");
    // Cloned out before the walk, for [`mssql_one_value`]'s reason.
    let columns = rows.columns().to_vec();
    let mut back = None;
    while let Some(row) = rows.next_row().expect("a row, or the end of the stream") {
        if back.is_none() {
            let value = row.column(0).expect("the row has a first column");
            back = match tds_scalar(&columns[0], value).expect("the column decodes under § 9") {
                TdsScalar::Bytes(stored) => Some(stored.to_vec()),
                other => panic!("the column answered {other:?}, which is not `bytes`"),
            };
        }
    }

    assert_eq!(
        back.as_deref(),
        Some(&octets[..]),
        "the octets the server stored are not the ones that went out",
    );
}
