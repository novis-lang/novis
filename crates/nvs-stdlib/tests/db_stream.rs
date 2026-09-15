//! `rule:core-classes/db-streaming`'s three promises, run against a real server
//! rather than read: a walk that answers on the driver the harness named,
//! constant memory over a result set whose size the answer must not depend on,
//! and the second statement refused.
//!
//! **These cases live in this crate because the promise is the member's, not a
//! driver's.** `crates/nvs-stdlib/src/db/stream.rs` is where
//! `Core\Db\Connection::stream` lives, and its in-crate
//! `stream_answers_rows_without_holding_the_result_set` counts the one parked
//! `Db\Row` over a thousand rows with no wire underneath it at all. What that
//! test cannot reach is the other half of the same sentence — the read state a
//! driver leaves between two steps, `rule:core-classes/a-stream-parks-its-read-on-the-connection` — because a
//! socket is what no `-p nvs-stdlib` unit test has. These cases are that half,
//! and they assert it through `nvs_db`'s own connections: the walk is asked for
//! one row at a time and the connection is watched between the steps.
//!
//! The skip rule is `nvs-db`'s, because it is the same harness: a process that
//! finds `NVS_DB_MATRIX_DRIVER` unset asserts nothing at all, so `python
//! tools/verify.py` is green on a machine with no containers and `python
//! tools/db-matrix.py` is what makes these assertions happen.
//! [`nvs_db::matrix`]'s module doc owns that rule and why a field that is *set
//! but unusable* panics instead.
//!
//! **Every case runs on whichever driver this process was pointed at, and one
//! case is written rather than five.** The statement a walk is opened over is
//! [`counting`] — the same series of whole numbers in each backend's own
//! spelling — so what changes between the legs is the dialect and never the
//! assertion, which is exactly what `rule:core-classes/db-streaming`'s *both members answer
//! on all five drivers* claims. Nothing here creates a table: a generated
//! series needs no schema, no cleanup and no isolation from the case running
//! beside it.

use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use nvs_config::tree::Database;
use nvs_db::matrix::{self, Location, Server};
use nvs_db::{
    Connection, Driver, MariaConn, MariaTarget, MySqlConn, MySqlTarget, PgConn, PgTarget,
    SqliteTarget, SqliteValue, State, TdsConn, TdsTarget,
};
use nvs_runtime::Value;

/// How long the whole of one handshake here may take.
///
/// The bound `crates/nvs-db/tests/handshake.rs` uses, for its reason: the
/// server is a container the harness may have started moments ago, and a matrix
/// leg that hangs reports nothing at all. The walk itself files no deadline —
/// what a bound would cover is the whole series rather than one step, so a slow
/// container would report as a broken promise.
const DEADLINE: Duration = Duration::from_secs(10);

/// How many rows the constant-memory case walks.
///
/// The thousand `stream_answers_rows_without_holding_the_result_set` sweeps for
/// the same reason it does: the number the answer must not depend on is the
/// result set's size, and three rows is a size every buffering implementation
/// also survives.
const ROWS: i64 = 1_000;

/// The statement a second one is attempted with, and the one that proves an
/// ended walk left the connection usable.
///
/// Every backend answers it and none needs a table for it.
const PROBE: &str = "select 1";

/// This process's connection, opened to whatever the harness published, or
/// `None` because no harness pointed it anywhere.
///
/// Two shapes of `None` and neither is a failure: no harness at all, and a leg
/// published on a Unix-domain socket, which is `nvs_db::matrix`'s known gap 1 —
/// the transport is asked for and nothing publishes it, so a case reaching for
/// one would be asserting about a leg that never runs.
fn leg() -> Option<Connection> {
    let endpoint = matrix::endpoint()?;
    match endpoint.location {
        Location::Server(server) => Some(over_wire(endpoint.driver, &server)),
        Location::File(path) => {
            let block = Database {
                driver: Some(String::from("sqlite")),
                path: Some(path.display().to_string()),
                ..Database::default()
            };
            let target = SqliteTarget::resolve(&block).expect("the block resolves to a path");
            Some(Connection::Sqlite(
                nvs_db::sqlite::open(&target).expect("the harness's scratch database opens"),
            ))
        }
        Location::Socket(_) => None,
    }
}

/// A connection to `server`, as a request that found the pool empty opens one.
///
/// **The four targets differ in their type and in nothing else**, so the arms
/// are a macro rather than four copies of one field list — `queue.rs`'s own
/// opener beside this file makes the same trade for the three it reaches:
/// `rule:core-classes/db-connection-is-named`'s settings are the same seven for every driver that has a
/// wire, and what changes between them is which `connect` reads them.
fn over_wire(driver: Driver, server: &Server) -> Connection {
    let addr: SocketAddr = (server.host.as_str(), server.port)
        .to_socket_addrs()
        .expect("the matrix host is an address")
        .next()
        .expect("the matrix host resolves to somewhere");
    let deadline = Some(Instant::now() + DEADLINE);

    macro_rules! connect_as {
        ($target:ident, $conn:ident, $arm:path) => {
            $arm(
                $conn::connect(
                    addr,
                    &$target {
                        host: &server.host,
                        user: &server.user,
                        password: &server.password,
                        database: &server.database,
                        tls_ca_file: Some(server.ca.as_path()),
                        time_zone: 0,
                        statement_cache: 8,
                    },
                    deadline,
                )
                .expect("the matrix server accepts a handshake verified against its own anchor"),
            )
        };
    }

    match driver {
        Driver::Postgres => connect_as!(PgTarget, PgConn, Connection::Postgres),
        Driver::MySql => connect_as!(MySqlTarget, MySqlConn, Connection::MySql),
        Driver::MariaDb => connect_as!(MariaTarget, MariaConn, Connection::MariaDb),
        Driver::SqlServer => connect_as!(TdsTarget, TdsConn, Connection::SqlServer),
        // Spelled rather than left to a `_`, exactly as `queue.rs`'s opener
        // spells the drivers it skips: SQLite is reached by path and [`leg`]
        // has already opened it, so a location arriving here is a harness
        // publishing a SQLite server rather than a case to skip.
        Driver::Sqlite => unreachable!("SQLite is a file, and `leg` opens it from its path"),
    }
}

/// `rows` whole numbers counting up from one, in `driver`'s own spelling.
///
/// A generated series rather than a table, because a walk is the only thing
/// under test: a `create table` would put a schema, a cleanup and an isolation
/// question between the case and the promise it is asserting. The spellings are
/// each backend's own for the reason `queue.rs`'s ad-hoc statements are —
/// `rule:core-classes/db-parameters`'s rewriter is what a *program*'s statement goes through, and
/// reaching for it here would put a second translator between a case and the
/// server it is asserting about.
///
/// The cast is what makes one assertion answer for five backends: § 9's `int`
/// is what every one of these columns then describes as, so [`whole`] reads the
/// same row of the type table whichever leg this is. SQL Server needs its
/// recursion bound lifted by hand — the default stops a recursive common table
/// expression at a hundred rows, and that is a server setting rather than
/// anything this walk could ask for.
fn counting(driver: Driver, rows: i64) -> String {
    match driver {
        Driver::Postgres => {
            format!("select n::bigint as n from generate_series(1, {rows}) as g(n)")
        }
        Driver::MySql | Driver::MariaDb => format!(
            "with recursive counted (n) as (select 1 union all select n + 1 from counted where n \
             < {rows}) select cast(n as signed) as n from counted"
        ),
        Driver::SqlServer => format!(
            "with counted (n) as (select 1 union all select n + 1 from counted where n < {rows}) \
             select cast(n as bigint) as n from counted option (maxrecursion 0)"
        ),
        Driver::Sqlite => format!(
            "with recursive counted (n) as (select 1 union all select n + 1 from counted where n \
             < {rows}) select cast(n as integer) as n from counted"
        ),
    }
}

/// Opens a walk over `sql`, answering how many columns the server described.
///
/// The description is counted rather than returned because it borrows the
/// connection, and every step below needs the connection back.
///
/// # Errors
///
/// Whatever the driver's own `stream` answered — a refusal [`recorded_refusal`]
/// is what reads.
fn stream(conn: &mut Connection, sql: &str) -> io::Result<usize> {
    match conn {
        Connection::Postgres(pg) => pg.stream(sql, &[]).map(<[_]>::len),
        Connection::MySql(mysql) => mysql.stream(sql, &[]).map(<[_]>::len),
        Connection::MariaDb(maria) => maria.stream(sql, &[]).map(<[_]>::len),
        Connection::SqlServer(tds) => tds.stream(sql, &[]).map(<[_]>::len),
        Connection::Sqlite(sqlite) => sqlite.stream(sql, Vec::new()).map(<[_]>::len),
    }
}

/// The next row of the parked walk as the whole number it holds, or `None` once
/// the walk has ended.
///
/// One column is read per step against the description the connection is still
/// holding, which is the decode `Core\Db\Stream` performs per `advance()`: §
/// 9's type map is read off a column, and a walk that had forgotten its
/// description could not answer a row at all.
///
/// # Errors
///
/// Whatever the driver's own `stream_next_row` answered.
fn next(conn: &mut Connection) -> io::Result<Option<i64>> {
    match conn {
        Connection::Postgres(pg) => {
            let Some(row) = pg.stream_next_row()? else {
                return Ok(None);
            };
            let columns = pg
                .stream_columns()
                .expect("an open walk describes its columns");
            Ok(Some(whole(columns[0].decode(row.column(0)?)?)))
        }
        Connection::MySql(mysql) => {
            let Some(row) = mysql.stream_next_row()? else {
                return Ok(None);
            };
            let columns = mysql
                .stream_columns()
                .expect("an open walk describes its columns");
            let cell = row.value(0).expect("the row has the column it selected");
            Ok(Some(whole(nvs_db::mysql::decode(&columns[0], cell)?)))
        }
        Connection::MariaDb(maria) => {
            let Some(row) = maria.stream_next_row()? else {
                return Ok(None);
            };
            let columns = maria
                .stream_columns()
                .expect("an open walk describes its columns");
            let cell = row.value(0).expect("the row has the column it selected");
            Ok(Some(whole(nvs_db::mysql::decode(&columns[0], cell)?)))
        }
        Connection::SqlServer(tds) => {
            let Some(row) = tds.stream_next_row()? else {
                return Ok(None);
            };
            let columns = tds
                .stream_columns()
                .expect("an open walk describes its columns");
            let cell = row.column(0).expect("the row has the column it selected");
            Ok(Some(whole(nvs_db::tds::decode_column(&columns[0], cell)?)))
        }
        Connection::Sqlite(sqlite) => {
            let Some(cells) = sqlite.stream_next_row()? else {
                return Ok(None);
            };
            // This driver answers in its own vocabulary rather than in § 9's
            // values: there is no wire and nothing was encoded, so the cell is
            // the integer SQLite is holding.
            match cells.first().expect("the row has the column it selected") {
                SqliteValue::Int(n) => Ok(Some(*n)),
                other => panic!("the counting statement selected an integer: {other:?}"),
            }
        }
    }
}

/// A decoded cell as § 9's `int`.
///
/// Both signed rows of the type table are accepted because two of these
/// backends describe a counted series as unsigned, and which one a server
/// chooses is not what any case here is asserting.
fn whole(cell: Option<Value>) -> i64 {
    let cell = cell.expect("the counting statement's column is never NULL");
    cell.as_int()
        .or_else(|| cell.as_uint().and_then(|n| i64::try_from(n).ok()))
        .expect("the cast makes every backend's column § 9's `int`")
}

/// How many rows the parked walk has handed back so far, off the walk's own
/// trace event.
///
/// **This is what tells a walk from a buffer at this layer.** The count is
/// raised where a row is *read*, so a driver that had read the result set into
/// memory to answer the first step would report every row of it before the
/// second step was asked for. `rule:observability/a-query-is-a-trace-event` owns the span itself.
fn counted(conn: &Connection) -> u64 {
    let span = match conn {
        Connection::Postgres(pg) => pg.stream_span(),
        Connection::MySql(mysql) => mysql.stream_span(),
        Connection::MariaDb(maria) => maria.stream_span(),
        Connection::SqlServer(tds) => tds.stream_span(),
        Connection::Sqlite(sqlite) => sqlite.stream_span(),
    };
    span.expect("an open walk files a span").rows()
}

/// Abandons the parked walk, as a program that stopped reading its rows does.
fn end_stream(conn: &mut Connection) {
    match conn {
        Connection::Postgres(pg) => pg.end_stream(),
        Connection::MySql(mysql) => mysql.end_stream(),
        Connection::MariaDb(maria) => maria.end_stream(),
        Connection::SqlServer(tds) => tds.end_stream(),
        Connection::Sqlite(sqlite) => sqlite.end_stream(),
    }
}

/// Runs [`PROBE`] and reads whatever it answered, which is the second statement
/// § 4 refuses while a walk is open.
///
/// **Read to its end rather than dropped**, because the answer is only half the
/// question: a case that goes on to assert the connection is idle needs the
/// probe's own result set gone, and one driver's rows have no `Drop` that would
/// drain them.
///
/// # Errors
///
/// Whatever the driver's own `query` answered — the refusal
/// [`names_both_fixes`] reads.
fn probe(conn: &mut Connection) -> io::Result<()> {
    macro_rules! drained {
        ($rows:expr) => {{
            let mut rows = $rows?;
            while rows.next_row()?.is_some() {}
            Ok(())
        }};
    }

    match conn {
        Connection::Postgres(pg) => drained!(pg.query(PROBE, &[])),
        Connection::MySql(mysql) => drained!(mysql.query(PROBE, &[])),
        Connection::MariaDb(maria) => drained!(maria.query(PROBE, &[])),
        Connection::SqlServer(tds) => drained!(tds.query(PROBE, &[])),
        Connection::Sqlite(sqlite) => {
            let mut rows = sqlite.query(PROBE, Vec::new())?;
            while rows.next_row().is_some() {}
            Ok(())
        }
    }
}

/// Whether a refusal names both of § 4's fixes.
///
/// Two wordings rather than one because there are two: the four drivers with a
/// wire share `pg.rs`'s `second_statement`, and SQLite writes its own for a
/// connection that has no stream object underneath it at all
/// (`rule:core-classes/db-connection-busy-state`). Both name the same pair — read the rows you have,
/// or open a second connection — and what this asserts is that the developer
/// who reaches one is told which two things to look at.
fn names_both_fixes(driver: Driver, refusal: &str) -> bool {
    match driver {
        Driver::Sqlite => {
            refusal.contains("read or drop the rows")
                && refusal.contains("open a second connection")
        }
        _ => refusal.contains("->all()") && refusal.contains("{shared: false}"),
    }
}

/// Asserts a driver that answered no walk refused in the shape the goal's
/// § *Standing decisions* records.
///
/// **A buffer is never the fallback**: a driver whose protocol cannot park a
/// read throws naming itself and `query`, so a program is told which member to
/// reach for instead. SQLite is the one driver that takes no refusal at all —
/// its walk is a pinned pool thread rather than a message boundary, which is
/// the user's call — so a refusal from it fails here rather than being read as
/// this fallback.
fn recorded_refusal(driver: Driver, refused: &io::Error) {
    assert_ne!(
        driver,
        Driver::Sqlite,
        "SQLite streams on a pinned thread and has no recorded refusal: {refused}"
    );
    let refusal = refused.to_string();
    assert!(
        refusal.contains(driver.display_name()) && refusal.contains("query"),
        "a driver that cannot park a read names itself and the member to reach for: {refusal}"
    );
}

/// `rule:core-classes/db-streaming`'s *both members answer on all five drivers*, asserted on
/// whichever one this process was pointed at.
///
/// The rows are read back as well as counted, because a walk that answered the
/// same row three times, or the rows of the result set before it, would count
/// correctly and be wrong in the one way a stream can be: what it hands back is
/// the statement's own rows, in the server's order.
#[test]
fn stream_answers_on_every_driver_or_names_its_recorded_refusal() {
    let Some(mut conn) = leg() else {
        return;
    };
    let driver = conn.driver();

    let described = match stream(&mut conn, &counting(driver, 3)) {
        Ok(described) => described,
        Err(refused) => return recorded_refusal(driver, &refused),
    };
    assert_eq!(described, 1, "the walk describes the column it selected");

    let mut seen = Vec::new();
    while let Some(n) = next(&mut conn).expect("the walk advanced") {
        seen.push(n);
    }
    assert_eq!(seen, [1, 2, 3], "the walk answers the statement's own rows");
    assert_eq!(
        conn.state(),
        State::Idle,
        "a drained walk left the connection unpoolable"
    );
    probe(&mut conn).expect("a drained walk left the connection able to run a statement");
}

/// The member's one promise, measured where a driver could break it: a walk
/// over a result set of any size reads **one row per step**.
///
/// **Asserted by counting the rows the connection has read, at every step**,
/// rather than by reading the last one: a driver that had drained the result
/// set into memory to answer the first step would answer every row correctly
/// and hold the whole set while doing it, which is precisely what `query`
/// already does and what this member exists not to. The busy state is the
/// second half of the same measurement — a connection whose rows are all in
/// memory has nothing left to hold the wire for, so one that is still
/// `Streaming` a row at a time is one whose rows are still on the server.
///
/// [`ROWS`] rows rather than three, because the number the answer must not
/// depend on is the result set's size.
#[test]
fn stream_holds_one_row_and_not_the_result_set_on_every_driver() {
    let Some(mut conn) = leg() else {
        return;
    };
    let driver = conn.driver();
    let Ok(_) = stream(&mut conn, &counting(driver, ROWS)) else {
        // The refusal is one case's assertion and not three; the case above
        // owns its shape.
        return;
    };

    for expected in 1..=ROWS {
        let n = next(&mut conn)
            .expect("the walk advanced")
            .expect("a walk with rows left answers one");
        assert_eq!(
            n, expected,
            "the walk answers the rows in the server's order"
        );
        assert_eq!(
            counted(&conn),
            u64::try_from(expected).expect("a row ordinal is a count"),
            "the connection has read as many rows as the walk was stepped, so nothing read the \
             result set ahead of it"
        );
        assert_eq!(
            conn.state(),
            State::Streaming,
            "a walk with rows left to read freed the connection"
        );
    }

    assert!(
        next(&mut conn).expect("the walk ended").is_none(),
        "the walk ends where the statement's rows do"
    );
    assert_eq!(
        counted(&conn),
        u64::try_from(ROWS).expect("a row count is a count"),
        "the span counts every row the walk produced"
    );
    assert_eq!(
        conn.state(),
        State::Idle,
        "a drained walk left the connection unpoolable"
    );
}

/// § 4's uniform busy rule: a statement attempted on a streaming connection is
/// refused on every driver, including the one whose protocol could carry it.
///
/// The refusal is `nvs-db`'s `io::Error` here rather than the `LogicError`
/// `Core\Db` re-words it as — `rule:core-classes/db-connection-busy-state` owns that seam — and what is
/// asserted is the pair of facts a program depends on: the wording names both
/// fixes, and the connection is left streaming rather than poisoned, so the
/// walk the caller interrupted is still there to finish.
#[test]
fn a_second_statement_on_a_streaming_connection_is_a_logic_error_on_every_driver() {
    let Some(mut conn) = leg() else {
        return;
    };
    let driver = conn.driver();
    let Ok(_) = stream(&mut conn, &counting(driver, 3)) else {
        return;
    };
    assert_eq!(
        conn.state(),
        State::Streaming,
        "an open walk holds the connection"
    );

    let refused = probe(&mut conn).expect_err("a second statement is refused while a walk is open");
    assert!(
        names_both_fixes(driver, &refused.to_string()),
        "the refusal names both of § 4's fixes: {refused}"
    );
    assert_eq!(
        conn.state(),
        State::Streaming,
        "a refused statement wrote nothing, so the walk under it is still readable"
    );
    assert_eq!(
        next(&mut conn)
            .expect("the walk advanced")
            .expect("a walk with rows left answers one"),
        1,
        "the refusal cost the walk none of its rows"
    );

    end_stream(&mut conn);
    assert_eq!(
        conn.state(),
        State::Idle,
        "an abandoned walk left the connection unpoolable"
    );
    probe(&mut conn).expect("an ended walk left the connection able to run a statement");
}
