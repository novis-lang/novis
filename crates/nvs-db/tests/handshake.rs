//! [ADR 0067](../../../docs/adr/0067-core-db.md) § 3's open against a real
//! server, one driver per process: the whole sequence — a socket, the in-band
//! upgrade, the TLS session and the authentication exchange — completes, and
//! the reads it is made of hand the core back rather than holding it.
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
//! MySQL's twins are the same three facts asked of that server: its
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

use std::cell::Cell;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;
use std::time::{Duration, Instant};

use nvs_db::matrix::{self, Location, Server};
use nvs_db::mysql::scalar;
use nvs_db::{
    DbErrorKind, Driver, MySqlConn, MySqlScalar, MySqlTarget, PgConn, PgTarget, ServerError,
};
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

    PgConn::connect(address(server), &target, Some(Instant::now() + DEADLINE))
}

/// Where the harness published `server`, as the address a driver connects to.
///
/// Resolved here and not inside either driver: ADR 0067 § 3 has the *name* on
/// the target because that is what the certificate is checked against, and the
/// address arrives separately because in a request it is the one the
/// `db.connect` capability approved. A test that handed a driver a name to
/// resolve would be exercising a path no program can reach.
fn address(server: &Server) -> SocketAddr {
    (server.host.as_str(), server.port)
        .to_socket_addrs()
        .expect("the matrix host is an address")
        .next()
        .expect("the matrix host resolves to somewhere")
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
        unreachable!("SQLite is the only driver reached by path, and this is not it")
    };
    Some(server)
}

/// One MySQL handshake against `server`, offering `password`.
///
/// [`connect_as`]'s twin, down to the password being a parameter for the same
/// reason: the refused credential below differs from the accepted one in that
/// field alone.
fn mysql_connect_as(server: &Server, password: &str) -> io::Result<MySqlConn> {
    let target = MySqlTarget {
        host: &server.host,
        user: &server.user,
        password,
        database: &server.database,
        tls_ca_file: Some(server.ca.as_path()),
        time_zone: 0,
        statement_cache: 8,
    };

    MySqlConn::connect(address(server), &target, Some(Instant::now() + DEADLINE))
}

/// A MySQL connection to `server`, as a request that found the pool empty opens
/// one.
fn mysql_open(server: &Server) -> MySqlConn {
    mysql_connect_as(server, &server.password)
        .expect("the matrix server accepts a handshake verified against its own anchor")
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
    let mut rows = conn.query(sql, &[]).expect("the server ran the statement");
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
/// The drain is not optional even where no row can arrive: ADR 0067 § 4 lets
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

/// § 3 on the other driver: the connection MySQL's handshake opens is
/// TLS-wrapped and authenticated, and the server is the one that says so.
///
/// [`a_connection_is_opened_tls_wrapped_and_authenticated_over_the_parking_stream`]'s
/// three questions, asked of MySQL. `Ssl_version` is a **session** status
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
