//! [ADR 0067](../../../docs/adr/0067-core-db.md) § 13's pool against a real
//! server: two requests on one core share one connection, and it is the *same*
//! connection rather than a second one that answers as well — and with
//! `pool = false` they share nothing, which is the same question asked of the
//! switch that turns the whole thing off.
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
use nvs_db::{Connection, Driver, PgConn, PgTarget};
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
/// testing one of the other four drivers, which runs every test in this crate
/// including this one.
fn postgres() -> Option<Server> {
    let endpoint = matrix::endpoint()?;
    if endpoint.driver != Driver::Postgres {
        return None;
    }
    let Location::Server(server) = endpoint.location else {
        unreachable!("SQLite is the only driver reached by path, and this is not it")
    };
    Some(server)
}

/// A fresh connection to `server`, as a request that found the pool empty opens
/// one.
fn open(server: &Server) -> PgConn {
    let addr: SocketAddr = (server.host.as_str(), server.port)
        .to_socket_addrs()
        .expect("the matrix host is an address")
        .next()
        .expect("the matrix host resolves to somewhere");
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

    PgConn::connect(addr, &target, Some(Instant::now() + DEADLINE))
        .expect("the matrix server accepts a handshake verified against its own anchor")
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
