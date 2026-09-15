//! `rule:security/db-pool-reset-is-a-boundary`'s pool against a real
//! server: two requests on one core share one connection, and it is the *same*
//! connection rather than a second one that answers as well; with
//! `pool = false` they share nothing, which is the same question asked of the
//! switch that turns the whole thing off; and the bounds that decide what a
//! pool keeps — `lifetime` and `idle` — close a socket rather than only drop a
//! row from a store.
//!
//! An integration test rather than a `mod tests` beside the driver, because
//! what it needs is exactly what a unit test in this crate deliberately does
//! not have — a socket, a certificate and a server that answers.
//! [`nvs_db::matrix`] is where it gets all three, and this crate's skip rule
//! rides with them: a process that finds `NVS_DB_MATRIX_DRIVER` unset asserts
//! nothing at all, so `python tools/verify.py` is green on a machine with no
//! containers and `python tools/db-matrix.py` is what makes these assertions
//! happen.
//!
//! # What this proves that neither half proves alone
//!
//! `nvs_runtime::pool`'s own cases run over a `Fake` connection: they hold the
//! store's bookkeeping and know nothing about a wire. This crate's cases script
//! a peer: they hold the protocol and know nothing about the pool. The seam
//! between them is where a released connection becomes the next request's, and
//! the only assertion that tells *this connection* from *a working connection*
//! is one the server itself answers. `pg_backend_pid()` is the identity of the
//! process on the other end of the socket, so an equal pid is one connection
//! and can be nothing else — a case that merely ran a second query would pass
//! with no pool underneath it at all.
//!
//! **`CONNECTION_ID()` is that identity on MySQL**, and every case here is
//! written twice for the reason the pool itself gives: `nvs_runtime::pool`
//! stores a `Box<dyn …>` and knows nothing about a driver, so "the connection
//! comes back" is a claim about the `Connection` this crate files and unfiles,
//! and it is a different variant, a different reset and a different socket per
//! driver. What changes between the twins beyond the query is the marker: § 13's
//! reset is asked to have removed a **session variable** on MySQL rather than a
//! temporary table, because there the variable is the one whose absence has a
//! single spelling — a missing temporary table is an error rather than a value,
//! and [`handshake`](/crates/nvs-db/tests/handshake.rs) is where that half is asserted.
//!
//! **MariaDB retells the reuse case and not the bounds.** `lifetime` and `idle`
//! are decided by `nvs_runtime::pool` before any driver is consulted: the
//! connection they retire is never handed back to anyone, so another copy of
//! them would re-ask a question with no MariaDB anywhere in it. What is this
//! driver's own is the acquire path — the `Connection::MariaDb` variant filed
//! and unfiled, and `MariaConn::reset`'s own `COM_RESET_CONNECTION` standing
//! where `MySqlConn::reset`'s stands, over a socket that authenticated through
//! a different plugin roster.
//!
//! # The teardown is `pool::release`, called the way `Ctx` calls it
//!
//! `nvs_runtime::Ctx`'s own `Drop` releases each connection a request still
//! holds under the lease it was filed with, and that one call is what this file
//! makes directly: a `Ctx` needs a request and a request needs a scheduler,
//! neither of which a `cargo test` process has. What sits between the two — that
//! a `Ctx` files the lease with the connection and consumes it exactly once — is
//! `nvs-runtime`'s own to assert, and its module doc owns the rule.

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nvs_config::db::PoolBounds;
use nvs_config::snapshot::Snapshot;
use nvs_db::matrix::{self, Location, Server};
use nvs_db::mysql::scalar;
use nvs_db::{
    Connection, Driver, MariaConn, MariaTarget, MySqlConn, MySqlRows, MySqlScalar, MySqlTarget,
    PgConn, PgTarget,
};
use nvs_runtime::pool::{self, Ticket};

/// How long the whole of one handshake here may take.
///
/// Generous, because the server is a container the harness may have started
/// moments ago, and finite because a matrix leg that hangs reports nothing at
/// all — which is the outcome a verification matrix must not produce.
const DEADLINE: Duration = Duration::from_secs(10);

/// This process's PostgreSQL server, or `None` because nothing pointed it at
/// one.
///
/// Two shapes of `None` and neither is a failure: no harness at all, and a leg
/// testing one of the other drivers, which runs every test in this crate
/// including this one.
fn postgres() -> Option<Server> {
    let endpoint = matrix::endpoint()?;
    if endpoint.driver != Driver::Postgres {
        return None;
    }
    let Location::Server(server) = endpoint.location else {
        // A location that is not a published server: the socket leg this driver also runs, or
        // SQLite's file. What this case asserts is the pool's own reuse rather than a transport,
        // and the same statements cross either one, so it is asked once -- over the port.
        // `crates/nvs-db/tests/handshake.rs` is the case list the socket leg exists to run.
        return None;
    };
    Some(server)
}

/// This process's MySQL server, or `None` because nothing pointed it at one.
///
/// [`postgres`]'s twin, and the twinning is what makes the skip rule one rule:
/// every case in this crate runs on every leg of the matrix, and each asks for
/// the one driver it can assert about.
fn mysql() -> Option<Server> {
    let endpoint = matrix::endpoint()?;
    if endpoint.driver != Driver::MySql {
        return None;
    }
    let Location::Server(server) = endpoint.location else {
        // A leg that is not a published server, as [`postgres`] above says.
        return None;
    };
    Some(server)
}

/// This process's MariaDB server, or `None` because nothing pointed it at one.
///
/// [`mysql`]'s twin, and the one line that differs is why the pair exists: the
/// matrix runs one driver per process, and a MariaDB leg is not a MySQL one
/// however alike the wire underneath it is.
fn mariadb() -> Option<Server> {
    let endpoint = matrix::endpoint()?;
    if endpoint.driver != Driver::MariaDb {
        return None;
    }
    let Location::Server(server) = endpoint.location else {
        // A leg that is not a published server, as [`postgres`] above says.
        return None;
    };
    Some(server)
}

/// Where `server` listens, which is the one part of opening a connection that
/// is not the driver's business.
fn address(server: &Server) -> SocketAddr {
    (server.host.as_str(), server.port)
        .to_socket_addrs()
        .expect("the matrix host is an address")
        .next()
        .expect("the matrix host resolves to somewhere")
}

/// A fresh connection to `server`, as a request that found the pool empty opens
/// one.
fn open(server: &Server) -> PgConn {
    let target = PgTarget {
        host: &server.host,
        user: &server.user,
        password: &server.password,
        database: &server.database,
        // The anchor the harness exported for this run — `matrix`'s module doc
        // owns why it is in the required group rather than beside it.
        tls_ca_file: Some(server.ca.as_path()),
        time_zone: 0,
        statement_cache: 8,
    };

    PgConn::connect(address(server), &target, Some(Instant::now() + DEADLINE))
        .expect("the matrix server accepts a handshake verified against its own anchor")
}

/// [`open`]'s twin on the other driver.
fn mysql_open(server: &Server) -> MySqlConn {
    let target = MySqlTarget {
        host: &server.host,
        user: &server.user,
        password: &server.password,
        database: &server.database,
        tls_ca_file: Some(server.ca.as_path()),
        time_zone: 0,
        statement_cache: 8,
    };

    MySqlConn::connect(address(server), &target, Some(Instant::now() + DEADLINE))
        .expect("the matrix server accepts a handshake verified against its own anchor")
}

/// [`mysql_open`]'s twin, and the target type is the whole of the difference:
/// `MariaTarget` carries the same fields and reaches a different auth roster
/// and a different error table.
fn mariadb_open(server: &Server) -> MariaConn {
    let target = MariaTarget {
        host: &server.host,
        user: &server.user,
        password: &server.password,
        database: &server.database,
        tls_ca_file: Some(server.ca.as_path()),
        time_zone: 0,
        statement_cache: 8,
    };

    MariaConn::connect(address(server), &target, Some(Instant::now() + DEADLINE))
        .expect("the matrix server accepts a handshake verified against its own anchor")
}

/// [`one_value`]'s twin over the binary protocol, where the column arrives
/// already typed and is read through § 9's own decoder rather than by parsing
/// the octets here.
fn mysql_one_value(conn: &mut MySqlConn, sql: &str) -> Option<String> {
    first_text(conn.query(sql, &[]).expect("the server ran the statement"))
}

/// [`mysql_one_value`] on the MariaDB driver.
fn mariadb_one_value(conn: &mut MariaConn, sql: &str) -> Option<String> {
    first_text(conn.query(sql, &[]).expect("the server ran the statement"))
}

/// The walk itself, shared by the two drivers that speak this result set.
///
/// [`MariaConn::query`] answers the *same* [`MySqlRows`] as [`MySqlConn::query`]
/// — one binary protocol, and § 9's decoder is one decoder — so the walk is
/// shared where every helper around it is twinned. What MariaDB does not share
/// with MySQL is above the framing, and that is what the cases below ask about.
///
/// Every caller below asks for its answer as `CHAR`, so a statement that
/// answered anything else panics rather than being rendered into text: what
/// `CONNECTION_ID()`'s own column type is has nothing to do with what these
/// cases are asserting, and pinning it here would make them fail for a reason
/// that is not theirs.
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

/// MySQL's `pg_backend_pid()`: the identity of the session on the other end of
/// this socket, and the whole of what tells one connection from another here.
///
/// This constant and the two below are read by the MariaDB cases as well, for
/// [`first_text`]'s reason: the spelling is the family's rather than either
/// server's, and a second copy under a `MARIA_` name would be the same string
/// twice.
const MYSQL_ID: &str = "SELECT CAST(CONNECTION_ID() AS CHAR)";

/// The session state a request leaves behind for § 13's reset to remove, and the
/// read that says whether it survived — the module doc owns why it is a variable
/// on this driver and a temporary table on the other.
const MYSQL_MARK: &str = "SET @novis_pool_marker = 'marker'";
const MYSQL_MARKER: &str = "SELECT CAST(@novis_pool_marker AS CHAR)";

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

/// § 13: the connection one request released at teardown is the connection the
/// next request on that core draws, and it arrives reset.
///
/// The identity is the assertion. A request handed a *fresh* connection would
/// answer the second query just as well, so `pg_backend_pid()` is what makes
/// the case fail when the pool hands back anything but the connection it was
/// given.
#[test]
fn two_requests_on_one_core_share_one_connection() {
    let Some(server) = postgres() else {
        return;
    };

    // One configuration generation, which is what § 13's key is scoped to —
    // `Ticket::key_for` owns why a reload is two pools rather than one.
    let generation = Arc::new(Snapshot::default());
    let ticket = Ticket::for_block(&generation, "main", PoolBounds::DEFAULT);

    // The first request. The pool is empty, so it opens; it leaves a temporary
    // table behind for the reset to remove. A temporary table rather than a
    // session variable because its absence has one spelling: `to_regclass`
    // answers `NULL` for a name nothing declares, where a custom GUC survives
    // `RESET ALL` as the empty string its placeholder was born with.
    let lease = pool::admit(ticket.clone()).expect("an unused key is under `max`");
    let mut opened = open(&server);
    let first = one_value(&mut opened, "SELECT pg_backend_pid()").expect("a backend has a pid");
    one_value(&mut opened, "CREATE TEMP TABLE novis_pool_marker (n int)");
    assert_eq!(
        one_value(&mut opened, "SELECT to_regclass('novis_pool_marker')").as_deref(),
        Some("novis_pool_marker"),
        "the session state this request is about to leave in the pool",
    );

    // Teardown: the one call `Ctx`'s `Drop` makes, which consumes the lease and
    // so gives the key's slot back with it.
    pool::release(
        lease,
        Instant::now(),
        Box::new(Connection::Postgres(opened)),
    );

    // The second request, on this same core: it admits under the same key and
    // finds a connection warm rather than opening one.
    let lease = pool::admit(ticket).expect("the released connection gave its slot back");
    let held = pool::take(&lease, Instant::now()).expect("this core released one under this key");
    let held = held
        .into_any()
        .downcast::<Connection>()
        .expect("this crate filed it, so this crate's type is what comes back");
    let Connection::Postgres(warm) = *held else {
        panic!("the driver under test on this leg is PostgreSQL")
    };
    // § 13's reset is the acquiring request's and not the releasing one's —
    // `nvs_runtime::pool`'s *Where the reset is* owns why — and it consumes the
    // connection, so a reset that failed could not hand one back here at all.
    let mut warm = warm.reset().expect("a healthy connection resets");

    assert_eq!(
        one_value(&mut warm, "SELECT pg_backend_pid()").as_deref(),
        Some(first.as_str()),
        "the second request is talking to the first request's backend, over its socket",
    );
    assert_eq!(
        one_value(&mut warm, "SELECT to_regclass('novis_pool_marker')"),
        None,
        "§ 13's reset removed the session state the first request left behind",
    );
}

/// § 13: `pool = false` restores connect-per-request **exactly** — nothing is
/// taken back, so the next request on this core opens its own connection.
///
/// The same shape as the case above with one field changed, which is the point:
/// an operator who cannot accept a reused connection has a supported answer
/// rather than a workaround, and what makes it supported is that the switch
/// changes this one thing and no other. `nvs_runtime::pool`'s own cases hold
/// the half a `Fake` can answer — that a pool which is off refuses no admission
/// — and this holds the half only a socket can: the connection a request
/// released is *gone*, not merely unreferenced by the store.
#[test]
fn a_pool_that_is_off_hands_the_next_request_nothing() {
    let Some(server) = postgres() else {
        return;
    };

    let generation = Arc::new(Snapshot::default());
    let ticket = Ticket::for_block(&generation, "main", PoolBounds::OFF);

    // A pool that is off still leases: `max` is a ceiling on connections held
    // live and not on pooling, so the ask is answered here exactly as it is
    // under `DEFAULT`, and only the release below parts company.
    let lease = pool::admit(ticket.clone()).expect("a pool that is off refuses nothing");
    let mut opened = open(&server);
    let first = one_value(&mut opened, "SELECT pg_backend_pid()").expect("a backend has a pid");
    pool::release(
        lease,
        Instant::now(),
        Box::new(Connection::Postgres(opened)),
    );

    // The second request. Its `take` is the assertion: `None` here is the store
    // holding nothing under this key, which is `release` having closed the
    // socket with the request that opened it rather than filing it.
    let lease = pool::admit(ticket).expect("an unused key is under `max`");
    assert!(
        pool::take(&lease, Instant::now()).is_none(),
        "`pool = false` files nothing, so there is nothing for this request to draw",
    );

    // And the connection it opens instead is a different backend — the observable
    // half, and the same `pg_backend_pid()` reading the case above turns around.
    let mut second = open(&server);
    let again = one_value(&mut second, "SELECT pg_backend_pid()").expect("a backend has a pid");
    assert_ne!(
        first, again,
        "connect-per-request means this request is talking to a backend of its own",
    );
}

/// § 13: a connection past its `lifetime` is retired rather than handed on, so
/// the request that follows opens a backend of its own.
///
/// The live double of `nvs_runtime::pool`'s
/// `a_connection_past_its_lifetime_is_never_handed_out`, which moves a clock
/// over a `Fake` and can therefore say only that the store stopped offering the
/// entry. What a socket adds is the other half: the retirement is a *close*, and
/// the pid the next request reads is the inverse of the assertion
/// [`two_requests_on_one_core_share_one_connection`] makes on the same query.
///
/// **The clock moves, not the bound.** `release` and `take` are each handed the
/// `Instant` they compare against, so § 13's real 30-minute `lifetime` is what
/// is under test here rather than a short one written to make the test finish —
/// which is also why this case costs no wall time.
#[test]
fn a_connection_past_its_lifetime_is_not_the_one_the_next_request_draws() {
    let Some(server) = postgres() else {
        return;
    };

    let generation = Arc::new(Snapshot::default());
    let ticket = Ticket::for_block(&generation, "main", PoolBounds::DEFAULT);
    let now = Instant::now();

    // The first request, exactly as the reuse case above leaves it: a healthy
    // connection filed under the key at teardown.
    let lease = pool::admit(ticket.clone()).expect("an unused key is under `max`");
    let mut opened = open(&server);
    let first = one_value(&mut opened, "SELECT pg_backend_pid()").expect("a backend has a pid");
    pool::release(lease, now, Box::new(Connection::Postgres(opened)));

    // The second request arrives past that entry's retirement, which is the one
    // difference between this case and the one above.
    let expired = now + PoolBounds::DEFAULT.lifetime + Duration::from_secs(1);
    let lease = pool::admit(ticket).expect("the released connection gave its slot back");
    assert!(
        pool::take(&lease, expired).is_none(),
        "a connection past its `lifetime` is retired by the scan that walks it, not handed on",
    );

    // So it opens its own, and the backend answering it is not the one the first
    // request left behind: the retired connection was closed, not parked.
    let mut second = open(&server);
    let again = one_value(&mut second, "SELECT pg_backend_pid()").expect("a backend has a pid");
    assert_ne!(
        first, again,
        "a retired connection is gone, so this request is talking to a backend of its own",
    );
}

/// § 13: `idle` bounds what a pool keeps, and the release past that bound closes
/// the connection instead of filing it.
///
/// The live double of `nvs_runtime::pool`'s
/// `a_release_past_the_idle_bound_closes_the_connection`. The `Fake` half can
/// say the store ends up holding one entry; only a server can say *which*
/// connection that entry is, and `pg_backend_pid()` is what tells them apart.
/// `idle = 1` because one is the smallest bound that still keeps something, so
/// the question has an answer: `release` refuses the second connection before it
/// is ever filed, which makes the survivor the first one released.
///
/// This is the bound that makes § 13's footprint O(in-flight) rather than
/// O(requests served) — `nvs_runtime::pool`'s module doc owns that argument —
/// and a socket is the only thing that can show the difference between a
/// connection dropped from the store and a connection actually closed.
#[test]
fn a_release_past_the_idle_bound_closes_the_second_connection() {
    let Some(server) = postgres() else {
        return;
    };

    let generation = Arc::new(Snapshot::default());
    let ticket = Ticket::for_block(
        &generation,
        "main",
        PoolBounds {
            idle: 1,
            ..PoolBounds::DEFAULT
        },
    );
    let now = Instant::now();

    // Two requests on this core at once, each holding its own connection. `max`
    // is the default 16, so both are admitted and the only bound in play below
    // is the one under test.
    let first_lease = pool::admit(ticket.clone()).expect("an unused key is under `max`");
    let second_lease = pool::admit(ticket.clone()).expect("two live connections are under `max`");
    let mut kept = open(&server);
    let mut spare = open(&server);
    let kept_pid = one_value(&mut kept, "SELECT pg_backend_pid()").expect("a backend has a pid");
    let spare_pid = one_value(&mut spare, "SELECT pg_backend_pid()").expect("a backend has a pid");
    assert_ne!(kept_pid, spare_pid, "two connections are two backends");

    pool::release(first_lease, now, Box::new(Connection::Postgres(kept)));
    pool::release(second_lease, now, Box::new(Connection::Postgres(spare)));

    // What the pool kept is the first release, and the assertion is that pid:
    // a case that only counted the store's entries would pass just as well if
    // the bound had thrown away the wrong one.
    let lease = pool::admit(ticket.clone()).expect("both releases gave their slots back");
    let held = pool::take(&lease, now).expect("the first release is filed under this key");
    let held = held
        .into_any()
        .downcast::<Connection>()
        .expect("this crate filed it, so this crate's type is what comes back");
    let Connection::Postgres(warm) = *held else {
        panic!("the driver under test on this leg is PostgreSQL")
    };
    let mut warm = warm.reset().expect("a healthy connection resets");
    assert_eq!(
        one_value(&mut warm, "SELECT pg_backend_pid()").as_deref(),
        Some(kept_pid.as_str()),
        "the connection kept is the one released under the bound, not the one past it",
    );

    // And there is nothing behind it: the second release closed its socket with
    // the request that opened it, exactly as `pool = false` does with every one.
    let lease = pool::admit(ticket).expect("an unused slot");
    assert!(
        pool::take(&lease, now).is_none(),
        "`idle = 1` keeps one connection under a key, so this request draws nothing",
    );
}

/// § 13 on the other driver: the connection one request released is the
/// connection the next request on that core draws, and it arrives reset.
///
/// [`two_requests_on_one_core_share_one_connection`]'s assertion, made of the
/// session id MySQL answers with. The pool is driver-blind — it files a
/// `Box<dyn …>` and gives it back — so what this adds over the PostgreSQL case
/// is everything between that box and a socket: the `Connection::MySql` variant
/// this crate files, the downcast that unfiles it, and
/// `MySqlConn::reset`'s `COM_RESET_CONNECTION` standing where the other
/// driver's `RESET ALL` stands.
#[test]
fn two_requests_on_one_core_share_one_mysql_connection() {
    let Some(server) = mysql() else {
        return;
    };

    // One configuration generation, which is what § 13's key is scoped to —
    // `Ticket::key_for` owns why a reload is two pools rather than one.
    let generation = Arc::new(Snapshot::default());
    let ticket = Ticket::for_block(&generation, "main", PoolBounds::DEFAULT);

    // The first request. The pool is empty, so it opens; it leaves a session
    // variable behind for the reset to remove.
    let lease = pool::admit(ticket.clone()).expect("an unused key is under `max`");
    let mut opened = mysql_open(&server);
    let first = mysql_one_value(&mut opened, MYSQL_ID).expect("a session has an id");
    mysql_one_value(&mut opened, MYSQL_MARK);
    assert_eq!(
        mysql_one_value(&mut opened, MYSQL_MARKER).as_deref(),
        Some("marker"),
        "the session state this request is about to leave in the pool",
    );

    // Teardown: the one call `Ctx`'s `Drop` makes, which consumes the lease and
    // so gives the key's slot back with it.
    pool::release(lease, Instant::now(), Box::new(Connection::MySql(opened)));

    // The second request, on this same core: it admits under the same key and
    // finds a connection warm rather than opening one.
    let lease = pool::admit(ticket).expect("the released connection gave its slot back");
    let held = pool::take(&lease, Instant::now()).expect("this core released one under this key");
    let held = held
        .into_any()
        .downcast::<Connection>()
        .expect("this crate filed it, so this crate's type is what comes back");
    let Connection::MySql(warm) = *held else {
        panic!("the driver under test on this leg is MySQL")
    };
    // § 13's reset is the acquiring request's and not the releasing one's —
    // `nvs_runtime::pool`'s *Where the reset is* owns why — and it consumes the
    // connection, so a reset that failed could not hand one back here at all.
    let mut warm = warm.reset().expect("a healthy connection resets");

    assert_eq!(
        mysql_one_value(&mut warm, MYSQL_ID).as_deref(),
        Some(first.as_str()),
        "the second request is talking to the first request's session, over its socket",
    );
    assert_eq!(
        mysql_one_value(&mut warm, MYSQL_MARKER),
        None,
        "§ 13's reset removed the session state the first request left behind",
    );
}

/// § 13 on MariaDB: that driver's own `COM_RESET_CONNECTION` hands the
/// connection one request released to the next request on that core, clean.
///
/// [`two_requests_on_one_core_share_one_mysql_connection`]'s assertion over the
/// other half of the family. Nothing it *reads* is new — the SQL is the same
/// spelling and the result set is the same decoder — and that is the point: what
/// is new is every step between `pool::release` and that read. The
/// `Connection::MariaDb` variant this crate files, the downcast that unfiles it
/// as *that* variant rather than the MySQL one, and [`MariaConn::reset`], which
/// is a second implementation of § 13's reset and not a call into the first.
#[test]
fn two_requests_on_one_core_share_one_mariadb_connection() {
    let Some(server) = mariadb() else {
        return;
    };

    // One configuration generation, which is what § 13's key is scoped to —
    // `Ticket::key_for` owns why a reload is two pools rather than one.
    let generation = Arc::new(Snapshot::default());
    let ticket = Ticket::for_block(&generation, "main", PoolBounds::DEFAULT);

    // The first request. The pool is empty, so it opens; it leaves a session
    // variable behind for the reset to remove.
    let lease = pool::admit(ticket.clone()).expect("an unused key is under `max`");
    let mut opened = mariadb_open(&server);
    let first = mariadb_one_value(&mut opened, MYSQL_ID).expect("a session has an id");
    mariadb_one_value(&mut opened, MYSQL_MARK);
    assert_eq!(
        mariadb_one_value(&mut opened, MYSQL_MARKER).as_deref(),
        Some("marker"),
        "the session state this request is about to leave in the pool",
    );

    // Teardown: the one call `Ctx`'s `Drop` makes, which consumes the lease and
    // so gives the key's slot back with it.
    pool::release(lease, Instant::now(), Box::new(Connection::MariaDb(opened)));

    // The second request, on this same core: it admits under the same key and
    // finds a connection warm rather than opening one.
    let lease = pool::admit(ticket).expect("the released connection gave its slot back");
    let held = pool::take(&lease, Instant::now()).expect("this core released one under this key");
    let held = held
        .into_any()
        .downcast::<Connection>()
        .expect("this crate filed it, so this crate's type is what comes back");
    let Connection::MariaDb(warm) = *held else {
        panic!("the driver under test on this leg is MariaDB")
    };
    // § 13's reset is the acquiring request's and not the releasing one's, and
    // it consumes the connection — so a reset that failed could not hand one
    // back here at all.
    let mut warm = warm.reset().expect("a healthy connection resets");

    assert_eq!(
        mariadb_one_value(&mut warm, MYSQL_ID).as_deref(),
        Some(first.as_str()),
        "the second request is talking to the first request's session, over its socket",
    );
    assert_eq!(
        mariadb_one_value(&mut warm, MYSQL_MARKER),
        None,
        "§ 13's reset removed the session state the first request left behind",
    );
}

/// § 13 on the other driver: a MySQL connection past its `lifetime` is retired
/// rather than handed on.
///
/// [`a_connection_past_its_lifetime_is_not_the_one_the_next_request_draws`]'s
/// twin, and the bound is the real 30-minute one for that case's reason: the
/// clock moves rather than the bound, so this costs no wall time either.
#[test]
fn a_mysql_connection_past_its_lifetime_is_not_the_one_the_next_request_draws() {
    let Some(server) = mysql() else {
        return;
    };

    let generation = Arc::new(Snapshot::default());
    let ticket = Ticket::for_block(&generation, "main", PoolBounds::DEFAULT);
    let now = Instant::now();

    let lease = pool::admit(ticket.clone()).expect("an unused key is under `max`");
    let mut opened = mysql_open(&server);
    let first = mysql_one_value(&mut opened, MYSQL_ID).expect("a session has an id");
    pool::release(lease, now, Box::new(Connection::MySql(opened)));

    let expired = now + PoolBounds::DEFAULT.lifetime + Duration::from_secs(1);
    let lease = pool::admit(ticket).expect("the released connection gave its slot back");
    assert!(
        pool::take(&lease, expired).is_none(),
        "a connection past its `lifetime` is retired by the scan that walks it, not handed on",
    );

    let mut second = mysql_open(&server);
    let again = mysql_one_value(&mut second, MYSQL_ID).expect("a session has an id");
    assert_ne!(
        first, again,
        "a retired connection is gone, so this request is talking to a session of its own",
    );
}

/// § 13 on the other driver: the release past `idle` closes that connection
/// instead of filing it.
///
/// [`a_release_past_the_idle_bound_closes_the_second_connection`]'s twin, and
/// the same reason for `idle = 1`: one is the smallest bound that still keeps
/// something, so the survivor has a name and the case can assert *which*
/// connection the pool kept rather than how many it holds.
#[test]
fn a_release_past_the_idle_bound_closes_the_second_mysql_connection() {
    let Some(server) = mysql() else {
        return;
    };

    let generation = Arc::new(Snapshot::default());
    let ticket = Ticket::for_block(
        &generation,
        "main",
        PoolBounds {
            idle: 1,
            ..PoolBounds::DEFAULT
        },
    );
    let now = Instant::now();

    let first_lease = pool::admit(ticket.clone()).expect("an unused key is under `max`");
    let second_lease = pool::admit(ticket.clone()).expect("two live connections are under `max`");
    let mut kept = mysql_open(&server);
    let mut spare = mysql_open(&server);
    let kept_id = mysql_one_value(&mut kept, MYSQL_ID).expect("a session has an id");
    let spare_id = mysql_one_value(&mut spare, MYSQL_ID).expect("a session has an id");
    assert_ne!(kept_id, spare_id, "two connections are two sessions");

    pool::release(first_lease, now, Box::new(Connection::MySql(kept)));
    pool::release(second_lease, now, Box::new(Connection::MySql(spare)));

    let lease = pool::admit(ticket.clone()).expect("both releases gave their slots back");
    let held = pool::take(&lease, now).expect("the first release is filed under this key");
    let held = held
        .into_any()
        .downcast::<Connection>()
        .expect("this crate filed it, so this crate's type is what comes back");
    let Connection::MySql(warm) = *held else {
        panic!("the driver under test on this leg is MySQL")
    };
    let mut warm = warm.reset().expect("a healthy connection resets");
    assert_eq!(
        mysql_one_value(&mut warm, MYSQL_ID).as_deref(),
        Some(kept_id.as_str()),
        "the connection kept is the one released under the bound, not the one past it",
    );

    let lease = pool::admit(ticket).expect("an unused slot");
    assert!(
        pool::take(&lease, now).is_none(),
        "`idle = 1` keeps one connection under a key, so this request draws nothing",
    );
}
