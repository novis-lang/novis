//! [ADR 0067](../../../docs/adr/0067-core-db.md) § 3's open against a real
//! server: the whole sequence — a socket, the in-band upgrade, the TLS session
//! and SCRAM — completes, and the reads it is made of hand the core back rather
//! than holding it.
//!
//! An integration test rather than a `mod tests` beside the driver, for the
//! reason [`pool_reuse`](./pool_reuse.rs)'s module doc gives and this crate's
//! own [`nvs_db::matrix`] exists for: a `PgConn`'s wire is `Wire<NvsTls<NvsTcp>>`
//! at the default type parameter, so nothing reachable only through one of its
//! inherent methods can be built without a socket and a certificate. `pg.rs`'s
//! own cases script a peer and hold each *step* — the eight bytes of the
//! `SSLRequest`, the refusal of a server with no TLS, the SCRAM exchange, the
//! `BackendKeyData` a startup must carry. What none of them can hold is that
//! those steps compose into a connection a real PostgreSQL accepts, which is
//! the only question left once each half is green.
//!
//! # Why the server is asked rather than the driver
//!
//! Both assertions below are the server's own answer, not this crate's.
//! `pg_stat_ssl` is PostgreSQL's view of the socket it is holding, so a row
//! saying `ssl` is the server reporting encryption on the connection this
//! process opened — a driver that had quietly fallen back to plaintext cannot
//! produce it, where an assertion made on our side of the wire would pass on
//! whatever we believed. `current_user` is the same shape for the other half,
//! and the refused password beside it is what makes it an *authentication*
//! rather than an admission: a server configured to trust every connection
//! would answer `current_user` just as well.
//!
//! # The skip rule rides with the endpoint
//!
//! A process that finds `NVS_DB_MATRIX_DRIVER` unset asserts nothing at all, so
//! `python tools/verify.py` is green on a machine with no containers and
//! `python tools/db-matrix.py` is what makes these assertions happen —
//! [`nvs_db::matrix`]'s module doc owns that rule and why a field that is *set
//! but unusable* panics instead.

use std::cell::Cell;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;
use std::time::{Duration, Instant};

use nvs_db::matrix::{self, Location, Server};
use nvs_db::{Driver, PgConn, PgTarget};
use nvs_host::{Reactor, Scheduler, run_until_idle};
use nvs_runtime::{Ctx, OutputSink, TaskRoot};

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

/// The companion task's wake interval, and the resolution of the observation it
/// is making.
const TURN: Duration = Duration::from_millis(10);

/// This process's PostgreSQL server, or `None` because nothing pointed it at
/// one.
///
/// Two shapes of `None` and neither is a failure: no harness at all, and a leg
/// testing one of the other four drivers, which runs every test in this crate
/// including these.
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

/// One handshake against `server`, offering `password`, answering the way the
/// caller of a connection that may not open needs it.
///
/// The password is a parameter because the wrong one is half of what
/// [`a_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream`]
/// asserts: the two calls differ in that field alone, so what the server
/// refuses is the credential and nothing else about the connection.
fn connect_as(server: &Server, password: &str) -> io::Result<PgConn> {
    let addr: SocketAddr = (server.host.as_str(), server.port)
        .to_socket_addrs()
        .expect("the matrix host is an address")
        .next()
        .expect("the matrix host resolves to somewhere");
    let target = PgTarget {
        host: &server.host,
        user: &server.user,
        password,
        database: &server.database,
        // The anchor the harness exported for this run — `matrix`'s module doc
        // owns why it is in the required group rather than beside it.
        tls_ca_file: Some(server.ca.as_path()),
        time_zone: 0,
        statement_cache: 8,
    };

    PgConn::connect(addr, &target, Some(Instant::now() + DEADLINE))
}

/// A connection to `server`, as a request that found the pool empty opens one.
fn open(server: &Server) -> PgConn {
    connect_as(server, &server.password)
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

/// § 3: the connection this driver opens is TLS-wrapped and authenticated, and
/// the server is the one that says so.
///
/// Three questions, and each is answered by a fact only the other end holds.
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
