//! [ADR 0084](../../../docs/adr/0084-durable-background-jobs.md)'s queue
//! statements, run against a real server rather than read.
//!
//! **These cases live in this crate and not in `nvs-db`, and one edge's
//! direction is the whole reason.** What they run is [`nvs_stdlib::queue`]'s
//! statement roster — § 2's schema has one home and that is it — and the only
//! thing that can run a statement is `nvs-db`'s own connection, which is
//! `PgConn`, `MySqlConn` or `MariaConn` as [`Conn`] holds it. ADR 0132 § 1 fixes
//! which way that edge points: `nvs-stdlib` depends on `nvs-db` and never the
//! reverse, so a `crates/nvs-db/tests/queue.rs` would need a `use
//! nvs_stdlib::…` that closes a cycle in the workspace, and the alternative —
//! a copy of the statement text kept beside such a test — would assert over
//! the copy rather than over the statement a `push` actually issues. A test
//! target here sees both crates and needs neither compromise. `tools/db-matrix.py`'s
//! `SUITES` is what points this file at a server as well as `nvs-db`'s own.
//!
//! The skip rule is `nvs-db`'s, because it is the same harness: a process that
//! finds `NVS_DB_MATRIX_DRIVER` unset asserts nothing at all, so `python
//! tools/verify.py` is green on a machine with no containers and `python
//! tools/db-matrix.py` is what makes these assertions happen.
//! [`nvs_db::matrix`]'s module doc owns that rule and why a field that is *set
//! but unusable* panics instead.
//!
//! **Every case owns a queue name and shares nothing else.** Cargo runs them on
//! threads of one process against one database, and a queue is exactly the key
//! every statement here is claimed, counted and ordered by, so a name each is
//! the whole of the isolation *in the rows* — no case reads a row another wrote,
//! and each clears its own before it pushes so a re-run starts where the first
//! run did. It is not the whole of the isolation in the **locks**, which is a
//! different question and one the framed dialect answers differently:
//! [`FRAMED_WRITES`] owns why those cases hold a lock the PostgreSQL ones do not
//! need.
//! The schema itself is the one thing they do share, and [`schema`] says why
//! that needs a `Once` rather than a call each.

use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::Deref;
use std::sync::{Mutex, MutexGuard, Once};
use std::time::{Duration, Instant};

use nvs_db::matrix::{self, Location, Server};
use nvs_db::{
    DbErrorKind, Driver, Isolation, MariaConn, MariaTarget, MySqlConn, MySqlRows, MySqlScalar,
    MySqlTarget, PgConn, PgTarget, QuerySpan, ServerError,
};
use nvs_stdlib::queue;

/// How long the whole of one handshake here may take.
///
/// The bound `crates/nvs-db/tests/handshake.rs` uses, for its reason: the
/// server is a container the harness may have started moments ago, and a
/// matrix leg that hangs reports nothing at all.
const DEADLINE: Duration = Duration::from_secs(10);

/// The ordinal `Core\Queue\State::Pending` is, as `MIGRATION_POSTGRES`'s `state` column
/// holds it — ADR 0010's enums are their ordinal at runtime, so the enum and
/// the column are one representation rather than two.
const PENDING: &[u8] = b"0";

/// The driver this process is testing, and the server the harness published for
/// it.
///
/// One value rather than two arguments threaded side by side, because
/// everything below that opens or spells a statement needs both:
/// [`nvs_stdlib::queue`] is two dialects, and the endpoint is what says which
/// one this leg runs.
struct Leg {
    /// Which backend this process was pointed at.
    driver: Driver,
    /// Where it is, and what vouches for it.
    server: Server,
}

/// This process's leg, when it names a driver [`queue::migration`] has a schema
/// for.
///
/// Three shapes of `None` and none of them is a failure: no harness at all; a
/// leg testing SQLite, which is reached by path and has no queue schema; and a
/// leg testing SQL Server, which has neither a schema nor a driver that can
/// send a statement. The per-case gates below narrow it once more, because a
/// case is written against one dialect's spelling even where both have one.
fn endpoint() -> Option<Leg> {
    let endpoint = matrix::endpoint()?;
    let Location::Server(server) = endpoint.location else {
        return None;
    };
    queue::migration(endpoint.driver)?;
    Some(Leg {
        driver: endpoint.driver,
        server,
    })
}

/// This process's PostgreSQL leg, or `None` because this run is one of the
/// others.
fn postgres() -> Option<Leg> {
    let leg = endpoint()?;
    (leg.driver == Driver::Postgres).then_some(leg)
}

/// This process's MySQL or MariaDB leg, whichever the harness named.
///
/// **One gate for the two rather than one each**, because there is nothing for
/// a second one to say: [`queue::MIGRATION_MYSQL`] and every statement beside
/// it is a text MariaDB runs unchanged, so a case written against either is a
/// case against both and running it twice is the matrix's job, not this
/// predicate's.
fn framed() -> Option<FramedLeg> {
    let leg = endpoint()?;
    matches!(leg.driver, Driver::MySql | Driver::MariaDb).then(|| FramedLeg {
        leg,
        // Poisoning is ignored on purpose: a case that panicked while holding
        // this has already reported, and cascading a `PoisonError` into every
        // case after it would bury the one failure that matters.
        _serial: FRAMED_WRITES
            .lock()
            .unwrap_or_else(|held| held.into_inner()),
    })
}

/// The lock one framed case holds for its whole body, and the reason a queue
/// name each is not the whole of the isolation on this dialect.
///
/// **InnoDB locks what a statement *scans*, not what it matches.** The rows a
/// case owns are its own, but the locks taken over them are not: [`clear`]'s
/// `delete … where queue = ?` over a table of a dozen rows is a scan the
/// optimizer is free to take whole, and it holds every row it looked at until
/// that statement commits. A neighbouring case's `for update skip locked` then
/// *skips its own row* — the claim answers nothing and the case reads it as a
/// queue that lost its work — or the case's `update` waits and the server
/// answers a deadlock instead. Both were measured, in two different cases, in
/// two of eight runs of the framed leg before this lock existed.
///
/// **On this gate rather than on [`endpoint`]**, because it is InnoDB's rule and
/// not a database's: PostgreSQL's readers take no lock a writer waits on, and
/// its cases have never contended. A leg runs one driver, so the framed cases
/// are the only writers on a framed leg and serializing them is total.
static FRAMED_WRITES: Mutex<()> = Mutex::new(());

/// A framed leg, holding [`FRAMED_WRITES`] for as long as the case does.
///
/// The guard is a field rather than a second binding at each case, so there is
/// no case that can forget to take it and none that can drop it early: the
/// [`Deref`] is what keeps `schema(&server)` and `open(&server)` reading as they
/// did when this was a [`Leg`].
struct FramedLeg {
    leg: Leg,
    _serial: MutexGuard<'static, ()>,
}

impl Deref for FramedLeg {
    type Target = Leg;

    fn deref(&self) -> &Leg {
        &self.leg
    }
}

/// A connection to `leg`'s server, as a request that found the pool empty opens
/// one.
///
/// **The three targets differ in their type and in nothing else**, so the arms
/// are a macro rather than three copies of one field list: ADR 0067 § 2's
/// settings are the same seven for every driver that has a wire, and what
/// changes between them is which `connect` reads them.
fn open(leg: &Leg) -> Conn {
    let server = &leg.server;
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

    match leg.driver {
        Driver::Postgres => connect_as!(PgTarget, PgConn, Conn::Postgres),
        Driver::MySql => connect_as!(MySqlTarget, MySqlConn, Conn::MySql),
        Driver::MariaDb => connect_as!(MariaTarget, MariaConn, Conn::MariaDb),
        // Spelled rather than left to a `_`, exactly as `crate::worker`'s own
        // opener spells them: [`endpoint`] skips both before anything is
        // connected, so a driver *gaining* a schema arrives here as a build
        // failure rather than as a refusal that has stopped being true.
        Driver::SqlServer | Driver::Sqlite => {
            unreachable!("`endpoint` skips a driver `nvs_stdlib::queue` has no schema for")
        }
    }
}

/// The connection a case runs on, in the driver the harness named.
///
/// **Owned and three arms, where [`Dialect`] is borrowed and two**, which is
/// `crates/nvs-cli/src/worker.rs`'s `Wire` one crate over and for its reason: a
/// case holds its connection for its whole body, so there has to be a value
/// that *is* the connection, and what every statement below branches on is the
/// dialect it is written in.
enum Conn {
    /// [`queue::MIGRATION_POSTGRES`] and the single-statement roster beside it.
    Postgres(PgConn),
    /// [`queue::MIGRATION_MYSQL`] and the roster whose claim and dead-letter
    /// move are [`queue::Split`]s.
    MySql(MySqlConn),
    /// MariaDB, which runs every one of MySQL's texts unchanged.
    MariaDb(MariaConn),
}

impl Conn {
    /// This connection borrowed as the dialect its statements are written in.
    fn dialect(&mut self) -> Dialect<'_> {
        match self {
            Conn::Postgres(postgres) => Dialect::Postgres(postgres),
            Conn::MySql(mysql) => Dialect::Framed(Framed::MySql(mysql)),
            Conn::MariaDb(maria) => Dialect::Framed(Framed::MariaDb(maria)),
        }
    }

    /// Which backend this is, for the helpers that pick a *statement* rather
    /// than a reader.
    ///
    /// Taken off the arm rather than carried from the [`Leg`], so a connection
    /// answers for itself: nothing below has both in hand, and two copies of
    /// one fact is how the wrong dialect gets sent down a connection that
    /// cannot run it.
    fn driver(&self) -> Driver {
        match self {
            Conn::Postgres(_) => Driver::Postgres,
            Conn::MySql(_) => Driver::MySql,
            Conn::MariaDb(_) => Driver::MariaDb,
        }
    }

    /// A `text` parameter in the first position, as this driver spells one.
    ///
    /// The ad-hoc statements in this file — the deletes a case clears with, the
    /// counts it asserts over — are written in the driver's own spelling for
    /// the reason [`queue`]'s two rosters are: ADR 0067 § 5's rewriter is what
    /// a *program*'s statement goes through, and reaching for it here would put
    /// a second translator between a case and the server it is asserting about.
    fn text(&self) -> &'static str {
        match self {
            Conn::Postgres(_) => "$1::text",
            Conn::MySql(_) | Conn::MariaDb(_) => "?",
        }
    }

    /// ADR 0067 § 7's `BEGIN`, as the driver's own.
    fn begin(&mut self, isolation: Option<Isolation>, read_only: bool) -> io::Result<QuerySpan> {
        match self.dialect() {
            Dialect::Postgres(postgres) => postgres.begin(isolation, read_only),
            Dialect::Framed(mut framed) => framed.begin(isolation, read_only),
        }
    }

    /// § 7's `COMMIT`.
    fn commit(&mut self) -> io::Result<QuerySpan> {
        match self.dialect() {
            Dialect::Postgres(postgres) => postgres.commit(),
            Dialect::Framed(mut framed) => framed.commit(),
        }
    }

    /// § 7's `ROLLBACK`.
    fn roll_back(&mut self) -> io::Result<QuerySpan> {
        match self.dialect() {
            Dialect::Postgres(postgres) => postgres.roll_back(),
            Dialect::Framed(mut framed) => framed.roll_back(),
        }
    }

    /// How many transactions deep this connection is, which is what a case
    /// asserts a commit or a rollback by.
    fn depth(&self) -> u32 {
        match self {
            Conn::Postgres(postgres) => postgres.depth(),
            Conn::MySql(mysql) => mysql.depth(),
            Conn::MariaDb(maria) => maria.depth(),
        }
    }
}

/// A borrowed [`Conn`], narrowed to the two dialects [`nvs_stdlib::queue`]
/// writes.
///
/// The same two arms `crates/nvs-cli/src/worker.rs`'s `Dialect` has, and for
/// the same reason: § 2's schema is two migration lists and §§ 4 and 6's
/// statements two spellings, so a third arm here would be a driver with nothing
/// to send.
enum Dialect<'a> {
    /// One statement in, one answer out.
    Postgres(&'a mut PgConn),
    /// The framed drivers, three of whose statements are pairs inside one
    /// transaction — and MariaDB runs every one of them unchanged.
    Framed(Framed<'a>),
}

/// The two drivers that share one dialect and one send path, borrowed as one.
///
/// **MariaDB is its own driver above the framing, not inside it**, and what it
/// does not duplicate is the wire: `MariaConn::query` delegates into the same
/// framing and hands back the same [`MySqlRows`]. So the only difference this
/// file can observe between the two is the *type of the borrow*, and without
/// this every reader below would grow a second arm copying the first line for
/// line. `nvs-cli` has the same enum for the same reason, on the other side of
/// ADR 0132 § 1's crate graph.
enum Framed<'a> {
    /// MySQL's framing of § 2's schema and § 4's statements.
    MySql(&'a mut MySqlConn),
    /// The same, framed as MariaDB.
    MariaDb(&'a mut MariaConn),
}

impl Framed<'_> {
    /// One statement and the rows it answers with, as the driver's own `query`.
    fn query(&mut self, sql: &str, bound: &[Option<&[u8]>]) -> io::Result<MySqlRows<'_>> {
        match self {
            Framed::MySql(mysql) => mysql.query(sql, bound),
            Framed::MariaDb(maria) => maria.query(sql, bound),
        }
    }

    /// One statement run for its effect, as the driver's own `execute_many`.
    fn execute_many(&mut self, sql: &str, sets: &[&[Option<&[u8]>]]) -> io::Result<u64> {
        match self {
            Framed::MySql(mysql) => mysql.execute_many(sql, sets),
            Framed::MariaDb(maria) => maria.execute_many(sql, sets),
        }
    }

    /// ADR 0067 § 7's `START TRANSACTION`.
    fn begin(&mut self, isolation: Option<Isolation>, read_only: bool) -> io::Result<QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.begin(isolation, read_only),
            Framed::MariaDb(maria) => maria.begin(isolation, read_only),
        }
    }

    /// § 7's `COMMIT`.
    fn commit(&mut self) -> io::Result<QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.commit(),
            Framed::MariaDb(maria) => maria.commit(),
        }
    }

    /// § 7's `ROLLBACK`.
    fn roll_back(&mut self) -> io::Result<QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.roll_back(),
            Framed::MariaDb(maria) => maria.roll_back(),
        }
    }
}

/// § 2's schema, applied once for this whole binary however many cases run.
///
/// **A `Once` rather than a step every case pays**, and not for the time:
/// `create table if not exists` is idempotent against a schema that is already
/// there and *not* safe against a second session running it in the same
/// moment, which answers a uniqueness error on the catalog rather than a
/// no-op. Cargo runs these cases on threads of one process, so that race is the
/// ordinary case here and not a rare one.
fn schema(leg: &Leg) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let mut conn = open(leg);
        let steps = queue::migration(leg.driver).expect("`endpoint` skipped the drivers with none");
        for step in steps {
            apply(&mut conn, step.sql, &[]);
        }
    });
}

/// Whatever an earlier run left on `queue`, in both tables.
fn clear(conn: &mut Conn, queue: &str) {
    let name = queue.as_bytes();
    let text = conn.text();
    for table in ["nvs_jobs", "nvs_dead_jobs"] {
        apply(
            conn,
            &format!("delete from {table} where queue = {text}"),
            &[Some(name)],
        );
    }
}

/// § 3's `insert into orders …`: the application table the two transactional
/// cases write beside the job, created once for this binary and emptied of
/// `queue`'s rows.
///
/// **The whole point of the table is that it is not the queue's.** § 3's
/// property is that a job and the write that caused it commit together, so a
/// case holding only the job would still pass against a `push` that quietly
/// enqueued on a connection of its own — the one thing § 3 exists to forbid.
/// It carries a `queue` column for the reason every other row here does: it is
/// what one case's rows are told apart from another's by.
///
/// The `Once` is [`schema`]'s and for [`schema`]'s reason, and the `delete` is
/// [`clear`]'s half for the one table § 2 does not own.
///
/// **The `create` is two spellings, split the way [`queue::MIGRATION_MYSQL`]
/// splits § 2's and for the same three reasons.** `bigserial` is a sequence and
/// a default in one word and the framed servers have neither, `text` is a column
/// InnoDB will not index at an unbounded width, and `engine=innodb` is what makes
/// a rollback of this table a rollback at all. The `delete` below stays one text,
/// because the only thing that differs there is the placeholder [`Conn::text`]
/// already answers with.
fn orders(leg: &Leg, conn: &mut Conn, queue: &str) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let mut own = open(leg);
        let create = match leg.driver {
            Driver::Postgres => {
                "create table if not exists nvs_stdlib_tests_orders \
                 (id bigserial primary key, queue text not null)"
            }
            _ => {
                "create table if not exists nvs_stdlib_tests_orders \
                 (id bigint not null auto_increment primary key, \
                 queue varchar(255) not null\
                 ) engine=innodb default charset=utf8mb4"
            }
        };
        apply(&mut own, create, &[]);
    });
    let text = conn.text();
    apply(
        conn,
        &format!("delete from nvs_stdlib_tests_orders where queue = {text}"),
        &[Some(queue.as_bytes())],
    );
}

/// One order row on `queue`, answering the id the insert wrote — the framed
/// half of § 3's application write.
///
/// **It reads the id off the OK packet rather than off a `returning`**, which is
/// [`push`]'s answer to the same question and its doc owns the reason. MySQL has
/// no `insert … returning` at all; MariaDB does, and reaching for it would make
/// this the one statement in the file that only one of the two drivers a framed
/// leg covers can run, for a value both of them already sent.
fn order_row(conn: &mut Conn, queue: &str) -> String {
    let Dialect::Framed(mut framed) = conn.dialect() else {
        unreachable!("an OK packet is the framed drivers', and PostgreSQL has `returning`")
    };
    let mut answered = framed
        .query(
            "insert into nvs_stdlib_tests_orders (queue) values (?)",
            &[Some(queue.as_bytes())],
        )
        .expect("the server ran the statement");
    // An insert answers no result set, and draining is both what ends the
    // statement on this driver and what lets `last_id` be read at all.
    while answered
        .next_row()
        .expect("the end of the stream")
        .is_some()
    {}
    answered
        .last_id()
        .filter(|id| *id != 0)
        .expect("the insert answered the `AUTO_INCREMENT` id [`orders`] declares")
        .to_string()
}

/// Every row `sql` answers with, each column as text and `None` for SQL `NULL`.
///
/// It drains to the end of the stream whatever it found, for the reason
/// `Core\Queue::push` does: the connection has to be back at a message boundary
/// before the next statement on it starts.
///
/// **The two arms are two protocols and not two spellings of one walk**, which
/// is why nothing here is shared between them: PostgreSQL's extended query
/// answers a column as the text it renders to, where the framed drivers send a
/// `bigint` as octets against the column definition it arrived under —
/// [`rendered`] is the whole of what that costs a case.
fn rows(conn: &mut Conn, sql: &str, bound: &[Option<&[u8]>]) -> Vec<Vec<Option<String>>> {
    match conn.dialect() {
        Dialect::Postgres(postgres) => {
            let mut answered = postgres
                .query(sql, bound)
                .expect("the server ran the statement");
            // Taken before the first row, because a `PgRows` lends its columns
            // and its rows out of one borrow.
            let width = answered.columns().len();
            let mut all = Vec::new();
            while let Some(row) = answered
                .next_row()
                .expect("a row, or the end of the stream")
            {
                let mut one = Vec::with_capacity(width);
                for at in 0..width {
                    one.push(
                        row.column(at)
                            .expect("the row has that column")
                            .map(|body| {
                                String::from_utf8(body.to_vec())
                                    .expect("PostgreSQL's text format is UTF-8")
                            }),
                    );
                }
                all.push(one);
            }
            all
        }
        Dialect::Framed(mut framed) => {
            let mut answered = framed
                .query(sql, bound)
                .expect("the server ran the statement");
            // Described before the first row for the same borrow reason, and
            // needed whatever that reason: `nvs_db::mysql::scalar` reads a
            // value against the definition it arrived under.
            let columns = answered.columns().to_vec();
            let mut all = Vec::new();
            while let Some(row) = answered
                .next_row()
                .expect("a row, or the end of the stream")
            {
                let mut one = Vec::with_capacity(columns.len());
                for (at, column) in columns.iter().enumerate() {
                    let body = row.value(at).expect("the row has that column");
                    one.push(rendered(
                        nvs_db::mysql::scalar(column, body).expect("the column is one § 9 maps"),
                    ));
                }
                all.push(one);
            }
            all
        }
    }
}

/// One framed column as the text the same column arrives as on PostgreSQL.
///
/// **§ 2's schema is `bigint` and text columns and nothing else**, so the four
/// rows below are the whole of what a statement in this file can answer with
/// and every one of them has a rendering both protocols agree on. Anything else
/// is this file reading a column § 2 does not declare, which says so rather
/// than rendering something plausible a case would then assert against.
fn rendered(read: MySqlScalar<'_>) -> Option<String> {
    match read {
        MySqlScalar::Null => None,
        MySqlScalar::Int(at) => Some(at.to_string()),
        MySqlScalar::UInt(at) => Some(at.to_string()),
        MySqlScalar::Text(text) => Some(text.to_string()),
        MySqlScalar::Bytes(octets) => Some(
            String::from_utf8(octets.to_vec()).expect("§ 2 declares no column that is not text"),
        ),
        other => panic!("§ 2's schema declares no column that reads as {other:?}"),
    }
}

/// One row's first column, as the text it arrived as.
fn one(conn: &mut Conn, sql: &str, bound: &[Option<&[u8]>]) -> String {
    let mut answered = rows(conn, sql, bound);
    assert_eq!(answered.len(), 1, "`{sql}` answered with one row");
    answered
        .remove(0)
        .remove(0)
        .expect("the column is not null")
}

/// A statement run for its effect, answering the count it affected.
fn apply(conn: &mut Conn, sql: &str, bound: &[Option<&[u8]>]) -> u64 {
    match conn.dialect() {
        Dialect::Postgres(postgres) => postgres.execute_many(sql, &[bound]),
        Dialect::Framed(mut framed) => framed.execute_many(sql, &[bound]),
    }
    .expect("the server ran the statement")
}

/// One pending job on `queue`, due at `at`, answering the id the insert wrote.
///
/// `max_attempts` is the case's whole lever over § 6: a job pushed with `1` is
/// exhausted by its first claim, and one pushed with more is owed another.
///
/// **A push with no dedupe key is one statement in both dialects**, which is
/// why this is not a [`queue::Split`] anywhere: [`queue::INSERT_MYSQL`]'s first
/// half is the dedupe read, and `Core\Queue::push` skips it for exactly this
/// case. What does differ is where the id comes from — PostgreSQL's
/// `returning`, and the framed drivers' own OK packet, which
/// [`nvs_db::MySqlRows::last_id`] carries and a `select last_insert_id()` would
/// pay a third round trip for.
fn push(conn: &mut Conn, queue: &str, at: i64, max_attempts: &str) -> String {
    let at = millis(at);
    let (script, args, backoff) = (
        &b"scripts/receipt.nvs"[..],
        &br#"{"order":7}"#[..],
        &b"1000"[..],
    );
    if conn.driver() == Driver::Postgres {
        return one(
            conn,
            queue::INSERT_POSTGRES,
            &[
                // No dedupe key: `INSERT_POSTGRES`'s `existing` arm is empty for a null
                // `$1`, which is the ordinary push.
                None,
                Some(queue.as_bytes()),
                Some(script),
                Some(args),
                Some(PENDING),
                Some(max_attempts.as_bytes()),
                Some(backoff),
                Some(at.as_slice()),
                Some(at.as_slice()),
            ],
        );
    }

    // `INSERT_MYSQL.then`'s own order, which is not `INSERT_POSTGRES`'s: the
    // dedupe key is ninth here and first there, and `attempts` is a literal
    // rather than a parameter.
    let bound = [
        Some(queue.as_bytes()),
        Some(script),
        Some(args),
        Some(PENDING),
        Some(max_attempts.as_bytes()),
        Some(backoff),
        Some(at.as_slice()),
        None,
        Some(at.as_slice()),
    ];
    let Dialect::Framed(mut framed) = conn.dialect() else {
        unreachable!("every driver but PostgreSQL is the framed dialect here")
    };
    let mut answered = framed
        .query(queue::INSERT_MYSQL.then, &bound)
        .expect("the server ran the statement");
    // An insert answers no result set, and draining is both what ends the
    // statement on this driver and what lets `last_id` be read at all.
    while answered
        .next_row()
        .expect("the end of the stream")
        .is_some()
    {}
    answered
        .last_id()
        .filter(|id| *id != 0)
        .expect("the insert answered the `AUTO_INCREMENT` id `MIGRATION_MYSQL` declares")
        .to_string()
}

/// [`queue::INSERT_MYSQL`]'s `then` alone, carrying a dedupe key, answering the
/// id it wrote or the server's own refusal.
///
/// **The refusal is the point, so this answers a `Result` where [`push`]
/// unwraps.** Running the second half without the first is not a shortcut: it is
/// exactly what a concurrent pusher does under `read committed`, where the guard
/// read the table a moment before the row it should have seen was committed.
/// [`queue::INSERT_MYSQL`]'s own doc owns the reason that is safe.
fn push_keyed(conn: &mut Conn, queue: &str, at: i64, key: &str) -> io::Result<String> {
    let at = millis(at);
    // `INSERT_MYSQL.then`'s order, as `push` sends it, with the eighth slot
    // filled: that is the one an ordinary push leaves null.
    let bound = [
        Some(queue.as_bytes()),
        Some(&b"scripts/receipt.nvs"[..]),
        Some(&br#"{"order":7}"#[..]),
        Some(PENDING),
        Some(&b"3"[..]),
        Some(&b"1000"[..]),
        Some(at.as_slice()),
        Some(key.as_bytes()),
        Some(at.as_slice()),
    ];
    let Dialect::Framed(mut framed) = conn.dialect() else {
        unreachable!("this pair is `INSERT_MYSQL`, and only the framed dialect runs it")
    };
    let mut answered = framed.query(queue::INSERT_MYSQL.then, &bound)?;
    // As in `push`: draining is what ends the statement and what lets `last_id`
    // be read, and a refusal surfaces here rather than at `query` when the
    // server sent its `ERR` after the execute was written.
    while answered.next_row()?.is_some() {}
    Ok(answered
        .last_id()
        .filter(|id| *id != 0)
        .expect("the insert answered the `AUTO_INCREMENT` id `MIGRATION_MYSQL` declares")
        .to_string())
}

/// [`queue::INSERT_MYSQL`]'s `first` — the guard half — answering the pending
/// job `key` already has, where it has one.
///
/// This is the read no case in this file had issued: `dedupe_pending = ?`
/// resolves against `MIGRATION_MYSQL`'s stored generated column, so what it
/// matches is not a column any statement writes and only a server can say
/// whether the construct holds.
fn pending_for(conn: &mut Conn, key: &str) -> Option<String> {
    let mut found = rows(conn, queue::INSERT_MYSQL.first, &[Some(key.as_bytes())]);
    (!found.is_empty()).then(|| found.remove(0).remove(0).expect("`id` is not null"))
}

/// One claim against `queue`, taken at `now`, returning jobs whose lease was
/// taken at or before `cutoff` as well as the ones nothing holds.
///
/// **Three values in one order for both dialects**, exactly as
/// `crates/nvs-cli/src/worker.rs`'s own claim sends them, and the answer is
/// [`queue::CLAIM_POSTGRES`]'s `returning` list either way — that is what
/// [`queue::CLAIM_MYSQL`]'s `select` is written to name. What differs is how
/// many statements it took: the pair's `update` is keyed by the id its `select`
/// just locked, and the transaction around them is what carries the lock across
/// the gap a single statement did not have.
fn claim(conn: &mut Conn, queue: &str, now: i64, cutoff: i64) -> Vec<Vec<Option<String>>> {
    let (now, cutoff) = (millis(now), millis(cutoff));
    let bound = [
        Some(queue.as_bytes()),
        Some(now.as_slice()),
        Some(cutoff.as_slice()),
    ];
    if conn.driver() == Driver::Postgres {
        return rows(conn, queue::CLAIM_POSTGRES, &bound);
    }

    conn.begin(None, false)
        .expect("the server opened the transaction");
    let took = rows(conn, queue::CLAIM_MYSQL.first, &bound);
    for row in &took {
        let id = row[0].as_deref().expect("a claimed row names its id");
        apply(
            conn,
            queue::CLAIM_MYSQL.then,
            &[Some(now.as_slice()), Some(id.as_bytes())],
        );
    }
    conn.commit()
        .expect("the server closed the transaction the pair was one moment inside");
    took
}

/// How many jobs and how many orders `queue` has, read in one statement.
///
/// **One read rather than two**, because § 3's property is about the pair: "one
/// exists without the other" is a state two statements can each miss, since
/// whatever happened between them is a moment neither one looked at.
///
/// **The framed spelling binds the name twice for one that is read twice**, and
/// that is the whole of the difference: `$1` is a number a statement may repeat
/// and `?` is a position that cannot, so one text with two occurrences is one
/// parameter there and two here. [`Conn::text`]'s doc owns why these ad-hoc
/// statements are written per driver rather than put through § 5's rewriter.
fn landed(conn: &mut Conn, queue: &str) -> (String, String) {
    let name = queue.as_bytes();
    let postgres = conn.driver() == Driver::Postgres;
    let sql = if postgres {
        "select (select count(*) from nvs_jobs where queue = $1::text), \
                (select count(*) from nvs_stdlib_tests_orders where queue = $1::text)"
    } else {
        "select (select count(*) from nvs_jobs where queue = ?), \
                (select count(*) from nvs_stdlib_tests_orders where queue = ?)"
    };
    let bound = if postgres {
        vec![Some(name)]
    } else {
        vec![Some(name), Some(name)]
    };
    let mut answered = rows(conn, sql, &bound);
    assert_eq!(answered.len(), 1, "a count answers with one row");
    let mut answered = answered.remove(0);
    let orders = answered.remove(1).expect("a count is not null");
    let jobs = answered.remove(0).expect("a count is not null");
    (jobs, orders)
}

/// § 2's roster: the queues a worker idling at `now` would find work in, with a
/// lease taken at or before `cutoff` counting as abandoned.
///
/// The section is § 2 and not § 1 because a roster is not one of that section's
/// four members — [`queue::QUEUES_POSTGRES`]'s own doc calls it § 2's unanswered
/// question, answered by the table because the config block names no queues.
///
/// **The answer is the whole table's and not one queue's**, which is the point
/// of the statement — a worker asks it once and claims against the names it got
/// back — and it is also why every assertion over it here is a *membership*
/// question about a name the case owns. Cargo runs these cases on threads of one
/// process against one database, so a case asserting the vector itself would be
/// asserting what its neighbours happened to be doing at that instant.
///
/// Both dialects bind the same two instants in the same order, which is what
/// [`queue::QUEUES_MYSQL`]'s doc means by the transcription changing nothing but
/// the placeholder spelling.
fn roster(conn: &mut Conn, now: i64, cutoff: i64) -> Vec<String> {
    let (now, cutoff) = (millis(now), millis(cutoff));
    let bound = [Some(now.as_slice()), Some(cutoff.as_slice())];
    let sql = if conn.driver() == Driver::Postgres {
        queue::QUEUES_POSTGRES
    } else {
        queue::QUEUES_MYSQL
    };
    rows(conn, sql, &bound)
        .into_iter()
        .map(|mut row| row.remove(0).expect("`queue` is not null"))
        .collect()
}

/// § 1's `status` for one job, or `None` where neither of § 2's tables holds it.
///
/// **The framed arm binds the pair twice, and that is the statement's whole
/// difference.** `$1` may be named as often as a statement likes and a `?` may
/// not, so [`queue::STATUS_MYSQL`] carries four placeholders for the two values
/// [`queue::STATUS_POSTGRES`] binds. `counted_row` is where the member doubles
/// them once rather than at each of the two call sites; this doubles them here,
/// in the same order, so the wrong-queue read below is asked of the doubling a
/// caller actually sends.
fn status(conn: &mut Conn, id: &str, queue: &str) -> Option<String> {
    let (id, queue) = (id.as_bytes(), queue.as_bytes());
    let (sql, bound) = if conn.driver() == Driver::Postgres {
        (queue::STATUS_POSTGRES, vec![Some(id), Some(queue)])
    } else {
        (
            queue::STATUS_MYSQL,
            vec![Some(id), Some(queue), Some(id), Some(queue)],
        )
    };
    let mut found = rows(conn, sql, &bound);
    (!found.is_empty()).then(|| found.remove(0).remove(0).expect("`state` is not null"))
}

/// § 1's `cancel` on the framed dialect, answering the count the server says it
/// affected.
///
/// **[`queue::CANCEL_MYSQL`] and not a dialect switch**, unlike [`status`]:
/// PostgreSQL's twin reads its answer off a `returning id` and this one off the
/// affected count, so a helper spanning both would hand back two shapes and
/// each caller would unwrap the one its own leg produced. No case sends the
/// PostgreSQL text, which is the half `Core\Queue::cancel`'s own
/// `counted_row` already covers.
fn cancel(conn: &mut Conn, id: &str, queue: &str) -> u64 {
    apply(
        conn,
        queue::CANCEL_MYSQL,
        &[Some(id.as_bytes()), Some(queue.as_bytes())],
    )
}

/// §§ 1 and 6's `stats` on the framed dialect, as the four columns
/// [`queue::COUNTS_MYSQL`] answers with.
///
/// **The columns stay `Option`**, unlike [`one`]'s, because half of what the
/// case that calls this asks is that none of the four is ever null: `count`
/// over no rows and `coalesce`d `sum` are what make that true, and a helper
/// that unwrapped here would assert it by panicking somewhere the message
/// blamed the helper.
///
/// **The name is bound twice** for [`landed`]'s reason: the outer `where` and
/// the dead-letter subquery are two `?` positions, where
/// [`queue::COUNTS_POSTGRES`] names `$1` twice.
fn stats(conn: &mut Conn, queue: &str) -> Vec<Option<String>> {
    let name = queue.as_bytes();
    let mut answered = rows(conn, queue::COUNTS_MYSQL, &[Some(name), Some(name)]);
    assert_eq!(
        answered.len(),
        1,
        "an aggregate with no `group by` is one row however empty the table is"
    );
    answered.remove(0)
}

/// An epoch-millisecond instant as the text a `$n::bigint` placeholder is sent
/// as — `nvs-cli`'s worker binds every instant this way.
fn millis(at: i64) -> Vec<u8> {
    at.to_string().into_bytes()
}

/// `id`, `script`, `attempts` and `max_attempts`'s places in `CLAIM_POSTGRES`'s
/// `returning` list, which that constant's doc owns and which
/// [`queue::CLAIM_MYSQL`]'s `select` names in the same order.
const ID: usize = 0;
const SCRIPT: usize = 1;
const ATTEMPTS: usize = 3;
const MAX_ATTEMPTS: usize = 4;

/// § 6: a job whose last attempt threw leaves `nvs_jobs` for `nvs_dead_jobs`
/// carrying what it threw, rather than being deleted or left claimed forever.
///
/// The three statements are the ones a worker runs and in the order it runs
/// them, so what is asserted is the roster agreeing with itself against a real
/// server: `INSERT_POSTGRES`'s `returning` names the row `CLAIM_POSTGRES` then takes, and the
/// lease `CLAIM_POSTGRES` writes is what `DEAD_LETTER_POSTGRES` is keyed on. `max_attempts = 1`
/// is what makes one claim an exhausted job — the returned `attempts` and
/// `max_attempts` say so, which is the condition `nvs-cli`'s `report` branches
/// on, asserted here so the case states why the move is the legal one rather
/// than arriving at it.
///
/// **The move is asserted from both sides.** § 6's claim is not that the row
/// appears in the dead-letter table but that it is *not discarded*: one
/// data-modifying CTE deletes it and inserts it, so a job left in neither table
/// would satisfy a case that only looked at where it went.
#[test]
fn an_exhausted_job_reaches_the_dead_letter_table_and_is_not_discarded() {
    const QUEUE: &str = "nvs-stdlib-tests-dead";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let now = queue::now_millis();
    let id = push(&mut conn, QUEUE, now, "1");

    let claimed = claim(&mut conn, QUEUE, now, now);
    assert_eq!(claimed.len(), 1, "the claim took the job that was pushed");
    let claimed = &claimed[0];
    assert_eq!(
        claimed[ID].as_deref(),
        Some(id.as_str()),
        "the row claimed is the row pushed"
    );
    assert_eq!(
        claimed[ATTEMPTS].as_deref(),
        Some("1"),
        "the claim counted this attempt"
    );
    assert_eq!(
        claimed[MAX_ATTEMPTS].as_deref(),
        Some("1"),
        "the attempt just counted was the job's last, so the job is exhausted"
    );

    let failed = now + 5;
    let errors = queue::dead_errors(now, "IOError", "the receipt service refused the order");
    let moved = apply(
        &mut conn,
        queue::DEAD_LETTER_POSTGRES,
        &[
            Some(id.as_bytes()),
            // The lease, which is what `CLAIM_POSTGRES` wrote to `claimed_at`.
            Some(millis(now).as_slice()),
            Some(millis(failed).as_slice()),
            Some(errors.as_bytes()),
        ],
    );
    assert_eq!(moved, 1, "one job moved");

    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_jobs where id = $1::bigint",
            &[Some(id.as_bytes())],
        ),
        "0",
        "the job left the jobs table rather than staying claimed"
    );

    let dead = rows(
        &mut conn,
        "select id, queue, errors, failed_at from nvs_dead_jobs where id = $1::bigint",
        &[Some(id.as_bytes())],
    );
    assert_eq!(
        dead.len(),
        1,
        "the job is in the dead-letter table, and so in one of the two rather than in neither"
    );
    let dead = &dead[0];
    assert_eq!(dead[0].as_deref(), Some(id.as_str()), "the job kept its id");
    assert_eq!(dead[1].as_deref(), Some(QUEUE), "the job kept its queue");
    assert_eq!(
        dead[2].as_deref(),
        Some(errors.as_str()),
        "the row carries the errors array whole, quoting and all"
    );
    assert_eq!(
        dead[3].as_deref(),
        Some(failed.to_string().as_str()),
        "the row says when the attempt ended, which is not when it started"
    );

    // The entry's own shape, read the way a reader of the column reads it: one
    // attempt, and what it threw as data rather than as text to grep.
    let entry: serde_json::Value = serde_json::from_str(dead[2].as_deref().expect("not null"))
        .expect("`errors` is a JSON document");
    let entry = entry.as_array().expect("`errors` is an array");
    assert_eq!(
        entry.len(),
        1,
        "one entry, which is the depth this schema pays for"
    );
    assert_eq!(entry[0]["class"], "IOError");
    assert_eq!(entry[0]["message"], "the receipt service refused the order");
    assert_eq!(entry[0]["at"], now);
}

/// § 4: a job whose worker went away is claimable again once its lease is older
/// than the visibility bound, and not one moment before.
///
/// **The bound is asserted on both sides**, because either half alone passes
/// against a statement that has stopped asking: a claim whose `cutoff` is older
/// than the lease answers nothing at all — the job is *held*, which is what
/// makes delivery at-least-once rather than at-most-one-worker-at-a-time — and
/// the same claim a millisecond past it answers with the same job. That second
/// claim is a second attempt and the column says so, which is what ties the
/// timeout to § 6's ladder: an abandoned job spends attempts exactly as a
/// throwing one does, so a worker that keeps dying cannot retry a job forever.
#[test]
fn a_visibility_timeout_returns_an_abandoned_job_to_the_queue() {
    const QUEUE: &str = "nvs-stdlib-tests-visibility";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let now = queue::now_millis();
    let id = push(&mut conn, QUEUE, now, "3");

    let first = claim(&mut conn, QUEUE, now, now);
    assert_eq!(first.len(), 1, "a pending job due now is claimable");
    assert_eq!(first[0][ID].as_deref(), Some(id.as_str()));
    assert_eq!(first[0][ATTEMPTS].as_deref(), Some("1"));

    // Held: the lease was taken at `now`, and this claim is willing to take
    // over only a lease from a millisecond earlier.
    let held = claim(&mut conn, QUEUE, now + 1, now - 1);
    assert!(
        held.is_empty(),
        "a job inside its visibility window is claimed by nobody else"
    );

    // Abandoned: the same claim, willing to take over a lease that old.
    let again = claim(&mut conn, QUEUE, now + 2, now);
    assert_eq!(
        again.len(),
        1,
        "a lease older than the bound returns the job to the queue"
    );
    assert_eq!(
        again[0][ID].as_deref(),
        Some(id.as_str()),
        "the job returned is the abandoned one and not a second row"
    );
    assert_eq!(
        again[0][ATTEMPTS].as_deref(),
        Some("2"),
        "the retaken job spent an attempt, so an abandoning worker cannot loop forever"
    );
    assert_eq!(
        one(
            &mut conn,
            "select claimed_at from nvs_jobs where id = $1::bigint",
            &[Some(id.as_bytes())],
        ),
        (now + 2).to_string(),
        "the lease moved to the claim that took it over"
    );
}

/// § 4: two workers claiming at once take different jobs, because the claim
/// skips a row another transaction holds instead of queueing behind it.
///
/// **This is the property `for update skip locked` exists for, and the only way
/// to see it is a lock that outlives a statement.** Each claim is autocommitted
/// on its own, so a second connection can only meet the first one's row lock
/// while the first is inside an open transaction — which is what this case
/// opens by hand. Without `skip locked` the second claim would *block* on the
/// held row rather than answer wrongly, so the case cannot assert its way to a
/// verdict: `statement_timeout` is what turns that hang into a failure, and it
/// is the reason this file's one hand-written `set` exists. A matrix leg that
/// hangs reports nothing at all.
///
/// The name says *every backend that has it* and this asserts the one whose
/// claim is a single statement. `a_framed_claim_skips_the_row_another
/// _transaction_holds` is the other half, and it is a separate case rather than
/// a second arm of this one because on that dialect the lock and the write are
/// two statements, so the holder is set up differently — not because the
/// property differs.
#[test]
fn claiming_is_skip_locked_shaped_on_every_backend_that_has_it() {
    const QUEUE: &str = "nvs-stdlib-tests-skip-locked";
    /// Long enough that a loaded container is not mistaken for a lock, short
    /// enough that a blocked claim is a failure rather than a hung leg.
    const BLOCKED: &str = "set statement_timeout = 3000";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut first = open(&server);
    clear(&mut first, QUEUE);

    let now = queue::now_millis();
    // Two jobs, due in the order `CLAIM_POSTGRES`'s `order by run_at, id` takes them.
    let older = push(&mut first, QUEUE, now, "3");
    let newer = push(&mut first, QUEUE, now + 1, "3");

    first
        .begin(None, false)
        .expect("the server opened a transaction");
    let held = claim(&mut first, QUEUE, now + 2, now);
    assert_eq!(
        held.len(),
        1,
        "the first worker claimed inside its transaction"
    );
    assert_eq!(
        held[0][ID].as_deref(),
        Some(older.as_str()),
        "the claim took the oldest due job"
    );

    let mut second = open(&server);
    apply(&mut second, BLOCKED, &[]);
    let took = claim(&mut second, QUEUE, now + 2, now);
    assert_eq!(
        took.len(),
        1,
        "the second worker was answered rather than left waiting on a row it cannot have"
    );
    assert_eq!(
        took[0][ID].as_deref(),
        Some(newer.as_str()),
        "it skipped the locked job and took the next due one, which is the whole of `skip locked`"
    );

    first.commit().expect("the server closed the transaction");

    // Both jobs are claimed and neither was claimed twice, which is the
    // property stated as a count rather than read off the two rows above.
    assert_eq!(
        one(
            &mut second,
            "select count(*) from nvs_jobs where queue = $1::text and state = 1 and attempts = 1",
            &[Some(QUEUE.as_bytes())],
        ),
        "2",
        "two jobs, one attempt each, and no job claimed by both workers"
    );
}

/// § 3: a job pushed inside a transaction becomes durable with the write that
/// caused it, in the one moment that transaction commits.
///
/// **The order row is what makes this a test of § 3 rather than of `INSERT_POSTGRES`.**
/// The property is that there is no window in which the job exists without the
/// write or the write without the job, so a case holding only the job would
/// pass just as well against a design that enqueued over a connection of its
/// own — which is the design § 3 rejects. Both rows go in over the one
/// connection the transaction is open on, and both are counted after it closes.
///
/// The `push` is [`queue::INSERT_POSTGRES`] and nothing about it changes inside a
/// transaction: § 3's enlistment is not a mode the statement is issued in but
/// the plain consequence of running it on a connection that is already in one,
/// which is why the design has no outbox in it.
#[test]
fn an_enqueue_commits_with_the_write_that_made_it() {
    const QUEUE: &str = "nvs-stdlib-tests-commit";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);
    orders(&server, &mut conn, QUEUE);

    let now = queue::now_millis();
    assert_eq!(
        landed(&mut conn, QUEUE),
        ("0".to_owned(), "0".to_owned()),
        "the case starts where an earlier run of it started"
    );

    conn.begin(None, false)
        .expect("the server opened a transaction");
    assert_eq!(conn.depth(), 1, "the connection is inside one transaction");

    let order = one(
        &mut conn,
        "insert into nvs_stdlib_tests_orders (queue) values ($1::text) returning id",
        &[Some(QUEUE.as_bytes())],
    );
    let id = push(&mut conn, QUEUE, now, "3");

    conn.commit().expect("the server closed the transaction");
    assert_eq!(conn.depth(), 0, "the transaction is over");

    assert_eq!(
        landed(&mut conn, QUEUE),
        ("1".to_owned(), "1".to_owned()),
        "the job and the write that caused it are both durable"
    );
    let job = rows(
        &mut conn,
        "select id, state, attempts from nvs_jobs where queue = $1::text",
        &[Some(QUEUE.as_bytes())],
    );
    assert_eq!(
        job[0][0].as_deref(),
        Some(id.as_str()),
        "the durable job is the row `INSERT_POSTGRES` answered with inside the transaction"
    );
    assert_eq!(
        job[0][1].as_deref(),
        Some(std::str::from_utf8(PENDING).expect("an ordinal is ASCII")),
        "it is claimable, so a worker that starts now runs it"
    );
    assert_eq!(
        job[0][2].as_deref(),
        Some("0"),
        "nothing has attempted it yet"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_stdlib_tests_orders where id = $1::bigint",
            &[Some(order.as_bytes())],
        ),
        "1",
        "the order the job was pushed for is the one that committed"
    );
}

/// § 3, from the other side: the same push under a `ROLLBACK` leaves no job at
/// all, so the job was never enqueued rather than enqueued and then compensated.
///
/// **The bound is asserted from both sides for the reason the visibility one
/// is**: a `push` that failed for any reason of its own would satisfy a case
/// that only ever rolled back, and one that enqueued outside the transaction
/// would satisfy a case that only ever committed. Neither half alone says
/// anything about the window between them.
///
/// The id is the tell that this is a rollback and not a failure. `INSERT_POSTGRES`
/// answered with one inside the transaction — a sequence does not roll back, so
/// the number was really allocated and really handed out — and what is gone
/// afterwards is the row, which is the only thing § 3 ever promised.
#[test]
fn a_rolled_back_write_leaves_no_job() {
    const QUEUE: &str = "nvs-stdlib-tests-rollback";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);
    orders(&server, &mut conn, QUEUE);

    let now = queue::now_millis();

    conn.begin(None, false)
        .expect("the server opened a transaction");
    apply(
        &mut conn,
        "insert into nvs_stdlib_tests_orders (queue) values ($1::text)",
        &[Some(QUEUE.as_bytes())],
    );
    let id = push(&mut conn, QUEUE, now, "3");
    assert_eq!(
        landed(&mut conn, QUEUE),
        ("1".to_owned(), "1".to_owned()),
        "both rows are there for the transaction that wrote them"
    );

    conn.roll_back().expect("the server undid the transaction");
    assert_eq!(conn.depth(), 0, "the transaction is over");

    assert_eq!(
        landed(&mut conn, QUEUE),
        ("0".to_owned(), "0".to_owned()),
        "neither the job nor the write that caused it survived"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_jobs where id = $1::bigint",
            &[Some(id.as_bytes())],
        ),
        "0",
        "the id `INSERT_POSTGRES` answered with names nothing, so no worker can ever claim it"
    );
}

/// § 6: a failed attempt arms the job for a later one, the ladder it climbs is
/// finite, and two jobs failing in the same moment do not come back in the same
/// one.
///
/// **The ladder's arithmetic is `a_retry_is_exponential_jittered_and_capped`'s
/// and is not asserted again here.** [`queue::retry_at`] is a pure function and
/// that unit test walks twelve rungs of it. What this case can say and that one
/// cannot is that the delay is a wait the *server* enforces:
/// [`queue::RETRY_POSTGRES`] writes it to `run_at`, and a claim a millisecond
/// earlier answers with nothing at all.
///
/// **Bounded is asserted by counting the claims rather than by reading the last
/// row.** Two jobs at two attempts each is four claims and then a queue that
/// answers nothing however far ahead the worker asks — a job armed once too
/// often would answer a fifth, and a case reading only the row in front of it
/// would not notice. The jitter is asserted as the thing § 6 wants it for: the
/// two delays *differ*, which is what stops a hundred jobs retrying an endpoint
/// that came back from landing on it together.
#[test]
fn retries_are_bounded_and_backoff_is_jittered() {
    const QUEUE: &str = "nvs-stdlib-tests-retry";
    /// The `backoff_ms` [`push`] writes, so the first rung is this file's to
    /// compute as well as the server's.
    const BACKOFF: i64 = 1_000;
    /// Two attempts each — the smallest number with a retry in it.
    const CAP: &str = "2";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let now = queue::now_millis();
    let pushed = [
        push(&mut conn, QUEUE, now, CAP),
        push(&mut conn, QUEUE, now, CAP),
    ];
    let mut claims = 0;

    // Each job's first attempt, which throws. `RETRY` is the write-back that
    // arms it for the next one at the instant `retry_at` put it on.
    let mut due = Vec::new();
    for expected in &pushed {
        let claimed = claim(&mut conn, QUEUE, now, now);
        assert_eq!(claimed.len(), 1, "a job due now is claimable");
        claims += 1;
        assert_eq!(
            claimed[0][ID].as_deref(),
            Some(expected.as_str()),
            "the claim took the jobs in the order they were pushed"
        );
        assert_eq!(claimed[0][ATTEMPTS].as_deref(), Some("1"));
        assert_eq!(
            claimed[0][MAX_ATTEMPTS].as_deref(),
            Some(CAP),
            "one attempt of two, so this job is owed another"
        );

        let id: i64 = expected
            .parse()
            .expect("`INSERT_POSTGRES` answered with an id");
        let at = queue::retry_at(now, 1, BACKOFF, id);
        assert_eq!(
            apply(
                &mut conn,
                queue::RETRY_POSTGRES,
                &[
                    Some(expected.as_bytes()),
                    // The lease this claim wrote, which is what the write-back
                    // is keyed on.
                    Some(millis(now).as_slice()),
                    Some(millis(at).as_slice()),
                ],
            ),
            1,
            "the write-back matched the claim's own lease"
        );
        due.push(at);
    }

    assert_ne!(
        due[0], due[1],
        "two jobs that failed in one moment are armed for two different ones"
    );
    for at in &due {
        assert!(
            (now + BACKOFF / 2..=now + BACKOFF).contains(at),
            "the first rung is one base, jittered inside its own top half"
        );
    }

    let armed = rows(
        &mut conn,
        "select id, state, attempts, run_at, claimed_at from nvs_jobs \
         where queue = $1::text order by id",
        &[Some(QUEUE.as_bytes())],
    );
    assert_eq!(armed.len(), 2, "both jobs are back in the queue, not gone");
    for (row, (id, at)) in armed.iter().zip(pushed.iter().zip(due.iter())) {
        assert_eq!(row[0].as_deref(), Some(id.as_str()));
        assert_eq!(
            row[1].as_deref(),
            Some(std::str::from_utf8(PENDING).expect("an ordinal is ASCII")),
            "the job is pending again rather than held by the worker that failed it"
        );
        assert_eq!(
            row[2].as_deref(),
            Some("1"),
            "the attempt it spent is still counted against it"
        );
        assert_eq!(
            row[3].as_deref(),
            Some(at.to_string().as_str()),
            "the server holds the delay `retry_at` computed, to the millisecond"
        );
        assert_eq!(
            row[4], None,
            "nothing holds a job that is waiting out its backoff"
        );
    }

    // Not yet: the backoff is a wait the claim enforces, not a number written
    // beside a row that is claimable anyway.
    let soonest = *due.iter().min().expect("both jobs were armed");
    assert!(
        claim(&mut conn, QUEUE, soonest - 1, soonest - 1).is_empty(),
        "a job inside its backoff is claimed by nobody"
    );

    // Each job's second attempt, which is the last one it has. The jitter is
    // why the order here is nobody's to predict, so the ids are collected and
    // compared as a set rather than one at a time.
    let latest = *due.iter().max().expect("both jobs were armed");
    let mut exhausted = Vec::new();
    for _ in 0..pushed.len() {
        let claimed = claim(&mut conn, QUEUE, latest, latest);
        assert_eq!(
            claimed.len(),
            1,
            "the armed job came back once its delay was out"
        );
        claims += 1;
        let row = &claimed[0];
        assert_eq!(
            row[ATTEMPTS], row[MAX_ATTEMPTS],
            "the attempt just counted was this job's last, which is what `report` branches on"
        );
        let id = row[ID]
            .as_deref()
            .expect("a claimed row has an id")
            .to_owned();
        assert_eq!(
            apply(
                &mut conn,
                queue::DEAD_LETTER_POSTGRES,
                &[
                    Some(id.as_bytes()),
                    Some(millis(latest).as_slice()),
                    Some(millis(latest + 1).as_slice()),
                    Some(
                        queue::dead_errors(latest, "IOError", "the endpoint is still down")
                            .as_bytes()
                    ),
                ],
            ),
            1,
            "the exhausted job moved rather than being armed again"
        );
        exhausted.push(id);
    }
    exhausted.sort();
    let mut expected = pushed.to_vec();
    expected.sort();
    assert_eq!(
        exhausted, expected,
        "the two jobs the second round claimed are the two that were pushed"
    );

    // The ladder ends, stated as the queue having nothing left however far
    // ahead the worker asks rather than as the state of the last row seen.
    let far = latest + BACKOFF * 64;
    assert!(
        claim(&mut conn, QUEUE, far, far).is_empty(),
        "an exhausted job is not armed again, so a job's retries are its `max_attempts` and no more"
    );
    assert_eq!(
        claims, 4,
        "two jobs at two attempts each, counted rather than read off the last row"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_dead_jobs where queue = $1::text and attempts = 2",
            &[Some(QUEUE.as_bytes())],
        ),
        "2",
        "both jobs are dead-lettered having spent every attempt they were given"
    );
}

/// §§ 2 and 4 against a real MySQL or MariaDB server: the schema applies, a job
/// pushed onto it is claimed once, and the claim answers the columns
/// `CLAIM_POSTGRES` answers.
///
/// **This is the first assertion in this file that is not PostgreSQL's**, and
/// what it is for is the half a unit agreement cannot reach: [`queue`]'s two
/// rosters are held against each other by
/// `both_dialects_answer_a_claim_with_the_same_columns`, which proves the two
/// *texts* name the same columns and cannot prove that either text is one a
/// server accepts. § 2's DDL, `skip locked`, the generated column and the
/// `AUTO_INCREMENT` id are each a construct only a server can refuse.
///
/// **Three things here are the [`queue::Split`] and not the statement.**
/// `attempts` is read as the value the row is *about to* have, because the
/// `update` has not run when the `select` answers — a claim answering the
/// pre-increment count would stop § 6's ladder one rung short. `claimed_at` is
/// asserted by value, because the pair's second half binds the lease and the id
/// in that order and a swapped pair still runs. And the second claim finding
/// nothing is where the lock the transaction carries across the two round trips
/// is observable at all.
#[test]
fn a_framed_push_is_claimed_once_and_answers_the_columns_postgresql_does() {
    const QUEUE: &str = "nvs-stdlib-tests-framed";
    const DUE: i64 = 1_000;
    const TAKEN: i64 = 2_000;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let id = push(&mut conn, QUEUE, DUE, "3");
    let took = claim(&mut conn, QUEUE, TAKEN, 0);
    assert_eq!(took.len(), 1, "one due job is one claimed row");
    let row = &took[0];
    assert_eq!(
        row[ID].as_deref(),
        Some(id.as_str()),
        "the claim took the row the insert's own OK packet named"
    );
    assert_eq!(
        row[SCRIPT].as_deref(),
        Some("scripts/receipt.nvs"),
        "the `select` answers what to run, at `CLAIM_POSTGRES`'s ordinal for it"
    );
    assert_eq!(
        row[ATTEMPTS].as_deref(),
        Some("1"),
        "`attempts + 1` is the count this claim will have spent, not the one it found"
    );
    assert_eq!(
        row[MAX_ATTEMPTS].as_deref(),
        Some("3"),
        "the job's own ceiling, which is what § 6's ladder is judged against"
    );

    let written = rows(
        &mut conn,
        "select state, attempts, claimed_at from nvs_jobs where id = ?",
        &[Some(id.as_bytes())],
    );
    assert_eq!(
        written,
        vec![vec![
            Some("1".to_string()),
            Some("1".to_string()),
            Some(TAKEN.to_string()),
        ]],
        "the pair's `update` armed the row it locked, and its lease is the instant it was sent"
    );

    assert!(
        claim(&mut conn, QUEUE, TAKEN + 1, 0).is_empty(),
        "a leased job is claimed once: nothing is due, and the lease is not past its cutoff"
    );
}

/// § 6's two write-backs on the framed dialect: a report reaches the row it
/// claimed and reaches no other.
///
/// **Both are keyed on the lease and the affected count is the whole answer**,
/// because neither dialect's `update` has a `returning` here: a worker that
/// overran § 4's visibility window is a report carrying a `claimed_at` the row
/// no longer has, and what it must do is match nothing rather than cancel the
/// attempt that replaced it. Asserted from both sides — a stale lease affects
/// no row, the real one affects exactly its own.
///
/// **`RETRY_MYSQL` binds `run_at`, `id`, `claimed_at`** where its PostgreSQL
/// twin binds `id`, `claimed_at`, `run_at`, a `?` being bound by the position
/// it occupies and the `set` clause standing left of the `where`. That constant
/// owns why; this is where the order meets a server, and a caller that reused
/// PostgreSQL's would arm the row at an instant of the job's own id.
#[test]
fn a_framed_write_back_reaches_the_lease_it_was_claimed_with_and_no_other() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-report";
    const TAKEN: i64 = 3_000;
    const ARMED: i64 = 9_000;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let retried = push(&mut conn, QUEUE, 1_000, "3");
    let succeeded = push(&mut conn, QUEUE, 1_001, "3");
    assert_eq!(claim(&mut conn, QUEUE, TAKEN, 0).len(), 1, "the older job");
    assert_eq!(claim(&mut conn, QUEUE, TAKEN, 0).len(), 1, "then the other");

    assert_eq!(
        apply(
            &mut conn,
            queue::SUCCEEDED_MYSQL,
            &[
                Some(succeeded.as_bytes()),
                Some(millis(TAKEN - 1).as_slice())
            ],
        ),
        0,
        "a lease the row does not hold is a worker reporting on an attempt that is over"
    );
    assert_eq!(
        apply(
            &mut conn,
            queue::SUCCEEDED_MYSQL,
            &[Some(succeeded.as_bytes()), Some(millis(TAKEN).as_slice())],
        ),
        1,
        "the lease it was claimed with names its own row and nothing else"
    );

    assert_eq!(
        apply(
            &mut conn,
            queue::RETRY_MYSQL,
            &[
                Some(millis(ARMED).as_slice()),
                Some(retried.as_bytes()),
                Some(millis(TAKEN).as_slice()),
            ],
        ),
        1,
        "the retry is keyed on the same lease, in this dialect's own bind order"
    );

    let written = rows(
        &mut conn,
        "select id, state, run_at, claimed_at from nvs_jobs where queue = ? order by id",
        &[Some(QUEUE.as_bytes())],
    );
    assert_eq!(
        written,
        vec![
            vec![
                Some(retried.clone()),
                Some("0".to_string()),
                Some(ARMED.to_string()),
                None,
            ],
            vec![
                Some(succeeded.clone()),
                Some("2".to_string()),
                Some("1001".to_string()),
                None,
            ],
        ],
        "one job is pending again at the instant the ladder chose, the other is done where it ran"
    );

    assert_eq!(
        claim(&mut conn, QUEUE, ARMED, 0).len(),
        1,
        "an armed job is claimable at its new `run_at`, and a succeeded one never again"
    );
}

/// § 6's move on the framed dialect: an exhausted job is in the dead-letter
/// table or in the jobs table, and the pair is what holds it to one of them.
///
/// **The copy runs before the delete here, which is the reverse of
/// PostgreSQL's**, and it is the only order there is once the `delete` cannot
/// answer with what it removed. So the property one data-modifying CTE holds by
/// construction is held by the transaction instead, and this is where that is
/// asserted against a server: both halves keyed on the lease, and the row
/// counted on both sides rather than only where it went.
#[test]
fn a_framed_exhausted_job_moves_to_the_dead_letter_table_in_one_transaction() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-dead";
    const TAKEN: i64 = 4_000;
    const FAILED: i64 = 4_500;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let id = push(&mut conn, QUEUE, 1_000, "1");
    let took = claim(&mut conn, QUEUE, TAKEN, 0);
    assert_eq!(took.len(), 1, "one due job is one claimed row");
    assert_eq!(
        (
            took[0][ATTEMPTS].as_deref(),
            took[0][MAX_ATTEMPTS].as_deref()
        ),
        (Some("1"), Some("1")),
        "the claim spent the job's last attempt, which is what makes the move the legal one"
    );

    let errors = queue::dead_errors(TAKEN, "IOError", "the receipt service refused the order");
    let lease = millis(TAKEN);
    let split = queue::DEAD_LETTER_MYSQL;
    conn.begin(None, false)
        .expect("the server opened the transaction");
    assert_eq!(
        apply(
            &mut conn,
            split.first,
            &[
                Some(millis(FAILED).as_slice()),
                Some(errors.as_bytes()),
                Some(id.as_bytes()),
                Some(lease.as_slice()),
            ],
        ),
        1,
        "the copy reads the columns while they still exist, keyed on the lease"
    );
    assert_eq!(
        apply(
            &mut conn,
            split.then,
            &[Some(id.as_bytes()), Some(lease.as_slice())],
        ),
        1,
        "and the delete is keyed on the same lease, so it cannot take a row from its new owner"
    );
    conn.commit()
        .expect("the server closed the transaction the pair was one moment inside");

    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_jobs where id = ?",
            &[Some(id.as_bytes())],
        ),
        "0",
        "the job left the jobs table rather than staying claimed"
    );
    let dead = rows(
        &mut conn,
        "select id, queue, errors, failed_at from nvs_dead_jobs where id = ?",
        &[Some(id.as_bytes())],
    );
    assert_eq!(
        dead,
        vec![vec![
            Some(id.clone()),
            Some(QUEUE.to_string()),
            Some(errors.clone()),
            Some(FAILED.to_string()),
        ]],
        "it is in the other table, carrying its own columns and the array § 6 asks for"
    );
}

/// § 2's dedupe on the framed dialect: the guard reads, but the **index** is
/// what makes a key unique.
///
/// [`queue::INSERT_MYSQL`]'s doc states the rule and this is where it meets a
/// server, because both halves are constructs no fake can stand in for.
/// `dedupe_pending = ?` resolves against a *stored generated column*, and the
/// uniqueness it feeds is `MIGRATION_MYSQL`'s `unique key nvs_jobs_dedupe` —
/// MySQL has no partial index, so the pending-rows-only scope PostgreSQL writes
/// as a `where` clause is carried here by the column evaluating to null for
/// every row that has left `Pending`. Nothing but a server can say whether that
/// pair behaves as the partial index it stands in for.
///
/// **Asserted on both sides of the bound, which is what makes it the emulation
/// and not merely a unique key.** A second pending row on one key is refused;
/// the same key inserts freely the moment its only holder is claimed. A schema
/// that made the column `dedupe_key` outright would pass the first half and fail
/// the second, and one that dropped the index would pass the second half on a
/// guard that is not a guarantee.
#[test]
fn a_framed_dedupe_push_is_refused_by_the_index_and_not_by_the_guard() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-dedupe";
    // Unique across the whole table rather than within the queue, so it names
    // this case: `nvs_jobs_dedupe` covers the column and not `(queue, column)`.
    const KEY: &str = "nvs-stdlib-tests-framed-dedupe:receipt:7";
    const DUE: i64 = 1_000;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    assert!(
        pending_for(&mut conn, KEY).is_none(),
        "an unseen key holds nothing, which is the arm `push`'s null has never reached"
    );

    let id = push_keyed(&mut conn, QUEUE, DUE, KEY).expect("an unheld key inserts");
    assert_eq!(
        pending_for(&mut conn, KEY).as_deref(),
        Some(id.as_str()),
        "the guard answers the pending job's own id, off a column no statement wrote"
    );

    let refused = push_keyed(&mut conn, QUEUE, DUE, KEY)
        .expect_err("a second pending row on one key is not the server's to accept");
    let raised = ServerError::of(&refused).expect("the refusal is the server's, not the wire's");
    assert_eq!(
        raised.kind,
        DbErrorKind::UniqueViolation,
        "§ 8's kind a caller branches on, from {}'s own code table: {raised}",
        raised.backend
    );
    assert!(
        raised.message.contains("nvs_jobs_dedupe"),
        "the index § 2 names is what refused it, and not some other uniqueness: {raised}"
    );

    assert_eq!(
        rows(
            &mut conn,
            "select count(*) from nvs_jobs where dedupe_key = ?",
            &[Some(KEY.as_bytes())],
        ),
        vec![vec![Some("1".to_string())]],
        "the refused insert wrote nothing"
    );

    // The other side. `state = 1` is `Core\Queue\State::Claimed`, and the
    // generated column reads null for it — which is where MySQL's own rule that
    // a unique key does not constrain nulls becomes § 2's partial index.
    apply(
        &mut conn,
        "update nvs_jobs set state = 1 where id = ?",
        &[Some(id.as_bytes())],
    );
    assert!(
        pending_for(&mut conn, KEY).is_none(),
        "a claimed job holds no key: the column is null for every state but `Pending`"
    );
    let again = push_keyed(&mut conn, QUEUE, DUE, KEY)
        .expect("the key is free the moment its only holder stops being pending");
    assert_ne!(
        again, id,
        "that is a second row and not the first one found again"
    );
}

/// § 4's visibility bound on the framed dialect, asserted on both sides as its
/// PostgreSQL twin is.
///
/// **What is new here is that the bound is enforced by a statement that is not
/// the one writing the lease.** [`queue::CLAIM_POSTGRES`] chooses the row and
/// re-takes it in a single statement, so the predicate and the write cannot
/// disagree. [`queue::CLAIM_MYSQL`] is a [`queue::Split`]: `((state = 0 and
/// run_at <= ?) or (state = 1 and claimed_at <= ?))` is on the `select` alone,
/// and the `update` is keyed by the `id` that `select` named and asks nothing
/// about the lease it is overwriting. What holds them together is the row lock
/// the `for update` took, inside the transaction the helper opens — so the
/// take-over path, and not the fresh claim the landed case already runs, is
/// where that arrangement is worth a server's opinion.
///
/// **The retaken job's `attempts` is the assertion the split can get wrong on
/// its own.** `attempts + 1 as attempts` is read by the `select`, before the
/// `update` that increments the column — PostgreSQL's `returning` runs after
/// its own write and reads the same number for the opposite reason. A fresh
/// claim answers `1` either way and hides a disagreement; the second rung is
/// where a half reading the column it is about to change answers `1` twice and
/// § 6's ladder never reaches `max_attempts`.
#[test]
fn a_framed_visibility_timeout_returns_an_abandoned_job_to_the_queue() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-visibility";

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let now = queue::now_millis();
    let id = push(&mut conn, QUEUE, now, "3");

    let first = claim(&mut conn, QUEUE, now, now);
    assert_eq!(first.len(), 1, "a pending job due now is claimable");
    assert_eq!(first[0][ID].as_deref(), Some(id.as_str()));
    assert_eq!(first[0][ATTEMPTS].as_deref(), Some("1"));

    // Held: the lease was taken at `now`, and this claim will take over only one
    // from a millisecond earlier. The `select`'s second arm is what refuses it,
    // and refusing it there is what stops the `update` running at all.
    assert!(
        claim(&mut conn, QUEUE, now + 1, now - 1).is_empty(),
        "a job inside its visibility window is claimed by nobody else"
    );
    assert_eq!(
        one(
            &mut conn,
            "select claimed_at from nvs_jobs where id = ?",
            &[Some(id.as_bytes())],
        ),
        now.to_string(),
        "a claim that took no row wrote no lease either: the pair's halves stand or fall together"
    );

    // Abandoned: the same claim, willing to take over a lease that old.
    let again = claim(&mut conn, QUEUE, now + 2, now);
    assert_eq!(
        again.len(),
        1,
        "a lease older than the bound returns the job to the queue"
    );
    assert_eq!(
        again[0][ID].as_deref(),
        Some(id.as_str()),
        "the job returned is the abandoned one and not a second row"
    );
    assert_eq!(
        again[0][ATTEMPTS].as_deref(),
        Some("2"),
        "the `select` answered the count the `update` after it would write, on the rung where \
         the two can disagree"
    );

    let written = rows(
        &mut conn,
        "select state, attempts, claimed_at from nvs_jobs where id = ?",
        &[Some(id.as_bytes())],
    );
    assert_eq!(
        written,
        vec![vec![
            Some("1".to_string()),
            Some("2".to_string()),
            Some((now + 2).to_string()),
        ]],
        "the lease moved to the claim that took it over, and the column agrees with what it answered"
    );
}

/// § 2's roster on the framed dialect, asserted on both sides of each of its two
/// arms.
///
/// **What a server is being asked here is that the two arms stay keyed to their
/// own instant.** [`queue::QUEUES_MYSQL`] is one `where` with an `or` in it, and
/// a transcription that let `now` reach the lease arm — or the cutoff reach the
/// due arm — answers plausibly for a queue with any work at all in it, which is
/// the shape a case asking one question per arm would pass. So each bound is
/// named on both sides: a job due at the instant asked about is due and one due
/// a millisecond later is not, and a lease is abandoned at the cutoff and not
/// before it, asked with a `now` far past the row's own `run_at` so that only
/// the second arm can be what answered.
///
/// **`distinct` is the other claim, and it is the one the doc costs out.** § 2's
/// `nvs_jobs_due` is `(queue, state, run_at)`, so neither dialect answers
/// `distinct queue` off the index's leading column: both walk the due rows.
/// What the statement owes in return is that two due jobs in one queue are one
/// name — the difference between a roster and a backlog, and the reason a worker
/// asks this once per idle turn rather than once per job.
#[test]
fn a_framed_roster_names_a_queue_on_either_arm_and_not_past_either_bound() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-roster";
    const DUE: i64 = 6_000;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let held = |named: Vec<String>| named.iter().filter(|name| name.as_str() == QUEUE).count();

    assert_eq!(
        held(roster(&mut conn, DUE, DUE)),
        0,
        "an empty queue is no queue at all: the roster names what has work and not what has a name"
    );

    let id = push(&mut conn, QUEUE, DUE, "3");
    assert_eq!(
        held(roster(&mut conn, DUE, 0)),
        1,
        "a job due at the instant asked about is due — `run_at <= ?` is the bound and this is its \
         last accepted value"
    );
    assert_eq!(
        held(roster(&mut conn, DUE - 1, 0)),
        0,
        "and the millisecond before it is the first refused one"
    );

    let second = push(&mut conn, QUEUE, DUE, "3");
    assert_ne!(second, id, "that is a second row and not the first again");
    assert_eq!(
        held(roster(&mut conn, DUE, 0)),
        1,
        "two due jobs in one queue are one name, which is what `distinct` is paid for"
    );

    // Both rows leased at `DUE`, written directly rather than through `claim`:
    // what is under test is the roster's reading of the two columns, and a claim
    // would decide for itself how many rows it took. Keyed by `id` and not by
    // `queue`, so what it locks is two rows rather than whatever a scan for a
    // queue reaches — every case here shares this table with the others running
    // beside it.
    let lease = millis(DUE);
    for row in [&id, &second] {
        apply(
            &mut conn,
            "update nvs_jobs set state = 1, claimed_at = ? where id = ?",
            &[Some(lease.as_slice()), Some(row.as_bytes())],
        );
    }
    assert_eq!(
        held(roster(&mut conn, DUE + 60_000, DUE - 1)),
        0,
        "a lease inside its window is nobody else's however late the instant asked about: `now` \
         reaches the due arm alone, and no row is in that arm any more"
    );
    assert_eq!(
        held(roster(&mut conn, 0, DUE)),
        1,
        "and a lease taken at the cutoff is abandoned work the roster names, answered with a `now` \
         before every `run_at` in the queue so the second arm is what answered"
    );
}

/// §§ 1 and 6's `status` on the framed dialect, walked through both of the
/// tables § 2 lets a job be in.
///
/// **The `union all` is the construct worth a server.** [`queue::STATUS_MYSQL`]
/// is not a [`queue::Split`] — its own doc says so, because two `select`s joined
/// this way are one statement in both dialects — but "MySQL parses it the same
/// way" is a claim about MySQL, and a `limit` sitting after a `union` is exactly
/// where the two dialects are known to differ about what the limit binds to.
/// Here it must bind to the whole union: the first arm is tried first and the
/// `limit 1` takes it.
///
/// **The four placeholders are the second claim, and the wrong-queue read is
/// what asks it.** The framed spelling binds `(id, queue)` twice where
/// PostgreSQL names `$1` and `$2` again, so a doubling that transposed the pair
/// would still answer `Pending` for a job asked about by its own id — and would
/// answer it for *any* queue. A read naming the right id and the wrong queue is
/// the assertion that separates those, and it is made against both arms, since
/// the dead-letter arm carries its own copy of the same condition.
#[test]
fn a_framed_status_walks_a_job_through_both_of_the_tables_it_can_be_in() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-status";
    const OTHER: &str = "nvs-stdlib-tests-framed-status-elsewhere";
    const DUE: i64 = 7_000;
    const TAKEN: i64 = 7_500;
    const FAILED: i64 = 7_900;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let id = push(&mut conn, QUEUE, DUE, "1");
    assert_eq!(
        status(&mut conn, &id, QUEUE).as_deref(),
        Some("0"),
        "a pushed job is `Pending`, off the state column the first arm reads"
    );
    assert_eq!(
        status(&mut conn, &id, OTHER),
        None,
        "the pair keys the read and the id alone does not, which is what a transposed doubling \
         would get wrong"
    );

    let took = claim(&mut conn, QUEUE, TAKEN, 0);
    assert_eq!(took.len(), 1, "one due job is one claimed row");
    assert_eq!(
        status(&mut conn, &id, QUEUE).as_deref(),
        Some("1"),
        "the same arm answers `Claimed` off the column the claim wrote"
    );

    // § 6's move, run as `a_framed_exhausted_job_moves_to_the_dead_letter_table_in_one_transaction`
    // runs it: what this case adds is the reader that has to follow the row.
    let errors = queue::dead_errors(TAKEN, "IOError", "the receipt service refused the order");
    let lease = millis(TAKEN);
    let split = queue::DEAD_LETTER_MYSQL;
    conn.begin(None, false)
        .expect("the server opened the transaction");
    apply(
        &mut conn,
        split.first,
        &[
            Some(millis(FAILED).as_slice()),
            Some(errors.as_bytes()),
            Some(id.as_bytes()),
            Some(lease.as_slice()),
        ],
    );
    apply(
        &mut conn,
        split.then,
        &[Some(id.as_bytes()), Some(lease.as_slice())],
    );
    conn.commit()
        .expect("the server closed the transaction the pair was one moment inside");

    assert_eq!(
        status(&mut conn, &id, QUEUE).as_deref(),
        Some("3"),
        "the second arm answers for a row that has left `nvs_jobs`: a caller asking what became of \
         its job is owed `Dead` rather than the absence the first arm alone would report"
    );
    assert_eq!(
        status(&mut conn, &id, OTHER),
        None,
        "and that arm carries the same pair, so the doubling is asserted on the side the first arm \
         cannot answer for"
    );
}

/// § 4's `skip locked` on the framed dialect: two workers claiming at once take
/// different jobs rather than one queueing behind the other.
///
/// **The holder is set up differently here, and that difference is the case.**
/// `claiming_is_skip_locked_shaped_on_every_backend_that_has_it` opens a
/// transaction and runs a whole claim inside it, because [`queue::CLAIM_POSTGRES`]
/// is one statement. [`queue::CLAIM_MYSQL`] is a [`queue::Split`], and running
/// its halves through [`claim`] would open a second transaction on a connection
/// already inside one — which on this dialect commits the first rather than
/// nesting. So the holder runs the `select` half alone: that statement is where
/// `for update skip locked` is written, and the lock it takes is the whole of
/// what the second worker must not queue behind.
///
/// **`innodb_lock_wait_timeout` is this dialect's `statement_timeout`**, and it
/// is here for that case's reason: without `skip locked` the second claim blocks
/// rather than answering wrongly, so a case cannot assert its way to a verdict
/// and a matrix leg that hangs reports nothing at all. Three seconds is long
/// enough that a loaded container is not mistaken for a lock.
///
/// **What the second worker is answered *with* is the planner's, and the case
/// does not pin it.** § 4's property is that a claim is answered rather than
/// queued behind a lock, and that no worker takes a row another holds — both of
/// which are asserted. Which rows remain available is a different question, and
/// this dialect answers it differently: measured against MySQL 8.4, the second
/// worker is answered with *nothing* where the PostgreSQL twin is answered with
/// the next due job, because `for update` locks the rows the `select` examined
/// while the `limit` applies to the sorted result — a scan over an `or` of two
/// state arms examines every due row in the queue. That is a throughput
/// property of a small table and a chosen plan, so a case fixing it would fail
/// the day either changes. What is fixed is what happens next: once the holder
/// commits, the deferred row is claimable, which is the difference between work
/// a lock delayed and work a claim lost.
///
/// **Both halves of the split are asserted to have survived the other worker.**
/// The holder finishes its own claim before it commits, so the case ends where
/// the PostgreSQL one does — two jobs claimed, one attempt each — which is the
/// property stated as a count rather than read off the two rows. A `then` keyed
/// by an id its own `select` did not name would pass every assertion above it
/// and fail that one.
#[test]
fn a_framed_claim_skips_the_row_another_transaction_holds() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-skip-locked";
    /// Seconds, which is the unit this server takes; the PostgreSQL twin's
    /// `statement_timeout` is milliseconds.
    const BLOCKED: &str = "set session innodb_lock_wait_timeout = 3";

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut first = open(&server);
    clear(&mut first, QUEUE);

    let now = queue::now_millis();
    // Two jobs, due in the order `CLAIM_MYSQL`'s `order by run_at, id` takes them.
    let older = push(&mut first, QUEUE, now, "3");
    let newer = push(&mut first, QUEUE, now + 1, "3");

    let (taken, cutoff) = (millis(now + 2), millis(now));
    let bound = [
        Some(QUEUE.as_bytes()),
        Some(taken.as_slice()),
        Some(cutoff.as_slice()),
    ];
    first
        .begin(None, false)
        .expect("the server opened a transaction");
    let held = rows(&mut first, queue::CLAIM_MYSQL.first, &bound);
    assert_eq!(
        held.len(),
        1,
        "the first worker locked a row inside its transaction"
    );
    assert_eq!(
        held[0][ID].as_deref(),
        Some(older.as_str()),
        "the claim took the oldest due job"
    );

    let mut second = open(&server);
    apply(&mut second, BLOCKED, &[]);
    let took = claim(&mut second, QUEUE, now + 2, now);
    assert!(
        took.iter()
            .all(|row| row[ID].as_deref() != Some(older.as_str())),
        "the second worker was answered rather than left waiting, and nothing it was answered with \
         is the row the holder is inside a transaction over"
    );

    // The holder's own `update`, which is the half the split still owes, run
    // against the id its `select` named and inside the transaction that named
    // it.
    assert_eq!(
        apply(
            &mut first,
            queue::CLAIM_MYSQL.then,
            &[Some(taken.as_slice()), Some(older.as_bytes())],
        ),
        1,
        "the row the holder locked is still the holder's to write"
    );
    first.commit().expect("the server closed the transaction");

    // Whatever the lock covered, it covered it for one transaction: the work
    // the second worker did not get is work, not a row that went missing.
    let mut claimed = took.len();
    if claimed == 0 {
        let after = claim(&mut second, QUEUE, now + 3, now);
        assert_eq!(
            after.len(),
            1,
            "the row a lock deferred is claimable the moment that lock is gone"
        );
        assert_eq!(
            after[0][ID].as_deref(),
            Some(newer.as_str()),
            "and it is the other job, which no worker has taken yet"
        );
        claimed += after.len();
    }
    assert_eq!(claimed, 1, "one job each and neither worker took two");

    assert_eq!(
        one(
            &mut second,
            "select count(*) from nvs_jobs where queue = ? and state = 1 and attempts = 1",
            &[Some(QUEUE.as_bytes())],
        ),
        "2",
        "two jobs, one attempt each, and no job claimed by both workers"
    );
}

/// § 3 on the framed dialect: the twin of
/// [`an_enqueue_commits_with_the_write_that_made_it`], and what running it a
/// second time buys is that § 3's property is the *connection's* rather than
/// PostgreSQL's.
///
/// § 3's design has no outbox in it because a `push` issued on a connection
/// already inside a transaction is enlisted by that fact alone — "not a mode the
/// statement is issued in". That is a claim about a protocol's transaction, so a
/// case proving it on one driver proves it for one protocol, and nothing on this
/// path is shared with the twin: `START TRANSACTION` is not `BEGIN`,
/// [`queue::INSERT_MYSQL`]'s `then` is not [`queue::INSERT_POSTGRES`] and answers
/// its id off an OK packet rather than a `returning`, and what holds the
/// uncommitted rows is InnoDB rather than PostgreSQL's own MVCC.
///
/// The order row is what makes this a case about § 3 rather than about
/// `INSERT_MYSQL`; the twin's doc argues that at length and this one does not
/// repeat it.
#[test]
fn a_framed_enqueue_commits_with_the_write_that_made_it() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-commit";

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);
    orders(&server, &mut conn, QUEUE);

    let now = queue::now_millis();
    assert_eq!(
        landed(&mut conn, QUEUE),
        ("0".to_owned(), "0".to_owned()),
        "the case starts where an earlier run of it started"
    );

    conn.begin(None, false)
        .expect("the server opened a transaction");
    assert_eq!(conn.depth(), 1, "the connection is inside one transaction");

    let order = order_row(&mut conn, QUEUE);
    let id = push(&mut conn, QUEUE, now, "3");

    conn.commit().expect("the server closed the transaction");
    assert_eq!(conn.depth(), 0, "the transaction is over");

    assert_eq!(
        landed(&mut conn, QUEUE),
        ("1".to_owned(), "1".to_owned()),
        "the job and the write that caused it are both durable"
    );
    let job = rows(
        &mut conn,
        "select id, state, attempts from nvs_jobs where queue = ?",
        &[Some(QUEUE.as_bytes())],
    );
    assert_eq!(
        job[0][ID].as_deref(),
        Some(id.as_str()),
        "the durable job is the row `INSERT_MYSQL` answered the id of inside the transaction"
    );
    assert_eq!(
        job[0][1].as_deref(),
        Some(std::str::from_utf8(PENDING).expect("an ordinal is ASCII")),
        "it is claimable, so a worker that starts now runs it"
    );
    assert_eq!(
        job[0][2].as_deref(),
        Some("0"),
        "nothing has attempted it yet"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_stdlib_tests_orders where id = ?",
            &[Some(order.as_bytes())],
        ),
        "1",
        "the order the job was pushed for is the one that committed"
    );
}

/// § 3's other side on the framed dialect, and the twin of
/// [`a_rolled_back_write_leaves_no_job`]: the same push under a `ROLLBACK`
/// leaves no job at all.
///
/// **The bound is asserted from both sides for the reason the PostgreSQL pair
/// is** — a `push` that failed for any reason of its own satisfies a case that
/// only ever rolled back, and one that enqueued outside the transaction
/// satisfies a case that only ever committed — and that argument is the twin's
/// doc's rather than repeated here.
///
/// What this half adds over the twin is the *mechanism* under the id: an
/// `AUTO_INCREMENT` counter does not roll back any more than a sequence does, so
/// the numbers [`order_row`] and [`push`] were answered with really were
/// allocated and really were handed out. What is gone afterwards is the row,
/// which is the only thing § 3 ever promised.
#[test]
fn a_framed_rolled_back_write_leaves_no_job() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-rollback";

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);
    orders(&server, &mut conn, QUEUE);

    let now = queue::now_millis();

    conn.begin(None, false)
        .expect("the server opened a transaction");
    let order = order_row(&mut conn, QUEUE);
    let id = push(&mut conn, QUEUE, now, "3");
    assert_eq!(
        landed(&mut conn, QUEUE),
        ("1".to_owned(), "1".to_owned()),
        "both rows are there for the transaction that wrote them"
    );

    conn.roll_back().expect("the server undid the transaction");
    assert_eq!(conn.depth(), 0, "the transaction is over");

    assert_eq!(
        landed(&mut conn, QUEUE),
        ("0".to_owned(), "0".to_owned()),
        "neither the job nor the write that caused it survived"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_jobs where id = ?",
            &[Some(id.as_bytes())],
        ),
        "0",
        "the id the OK packet answered with names nothing, so no worker can ever claim it"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_stdlib_tests_orders where id = ?",
            &[Some(order.as_bytes())],
        ),
        "0",
        "and the write it was enqueued beside is gone with it"
    );
}

/// § 1's `cancel` on the framed dialect: the statement decides, and the answer
/// is the count it affected.
///
/// **What needs a server is that the count is the same number PostgreSQL's
/// `returning id` produces.** [`queue::CANCEL_MYSQL`]'s doc rests the two
/// spellings on one reading — `set state = 4 where … and state = 0` changes
/// every row it matches — and that argument is load-bearing here in a way it is
/// not on PostgreSQL: MySQL's affected count is *changed* rows, not matched
/// ones, so a guard that let the statement match a row it then wrote the same
/// value into would answer `0` for a cancel that happened. The `and state = 0`
/// is what makes changed and matched one number, and only a server can say so.
///
/// **The cancel that lost the race answers `0` rather than throwing**, which is
/// § 1's whole reason for a `bool` return: the claim below lands between the
/// push and the cancel exactly as a worker's would, and the statement — not a
/// read before it — is what declines. A cancel of an already-cancelled job is
/// the same answer for the same reason, since `state = 4` fails the same guard.
///
/// **The wrong-queue read is the third claim, and it is asked of a job that is
/// otherwise cancellable.** Two `?` are two positions, so a text that named the
/// id alone, or that transposed the pair, would still cancel from another
/// queue; the last two assertions separate "this job cannot be cancelled" from
/// "this pair does not name it" by cancelling the same row a line later.
#[test]
fn a_framed_cancel_is_decided_by_the_statement_and_read_off_the_affected_count() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-cancel";
    const OTHER: &str = "nvs-stdlib-tests-framed-cancel-elsewhere";
    const DUE: i64 = 8_000;
    const TAKEN: i64 = 8_500;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let pending = push(&mut conn, QUEUE, DUE, "2");
    assert_eq!(
        cancel(&mut conn, &pending, QUEUE),
        1,
        "a pending job is cancellable, and one row changed is the `true` the member answers"
    );
    assert_eq!(
        status(&mut conn, &pending, QUEUE).as_deref(),
        Some("4"),
        "the row is still there carrying `Cancelled`, which is why this is an update and not a \
         delete"
    );
    assert_eq!(
        cancel(&mut conn, &pending, QUEUE),
        0,
        "and cancelling it again changes nothing, because `state = 4` fails the same guard"
    );

    let running = push(&mut conn, QUEUE, DUE, "2");
    let took = claim(&mut conn, QUEUE, TAKEN, 0);
    assert_eq!(took.len(), 1, "the cancelled job is not work a worker sees");
    assert_eq!(
        took[0][ID].as_deref(),
        Some(running.as_str()),
        "so the one claimed row is the job pushed after it"
    );
    assert_eq!(
        cancel(&mut conn, &running, QUEUE),
        0,
        "a cancel that lost the race to that claim answers zero rather than throwing"
    );
    assert_eq!(
        status(&mut conn, &running, QUEUE).as_deref(),
        Some("1"),
        "and it left the claim alone: cancelling does not stop work in flight"
    );

    let elsewhere = push(&mut conn, QUEUE, DUE, "2");
    assert_eq!(
        cancel(&mut conn, &elsewhere, OTHER),
        0,
        "the pair keys the update and the id alone does not"
    );
    assert_eq!(
        status(&mut conn, &elsewhere, QUEUE).as_deref(),
        Some("0"),
        "the job the wrong queue named is untouched"
    );
    assert_eq!(
        cancel(&mut conn, &elsewhere, QUEUE),
        1,
        "and it was cancellable all along, which is what separates the wrong pair from a job past \
         cancelling"
    );
}

/// §§ 1 and 6's `stats` on the framed dialect, over a queue walked from empty to
/// one job pending, one claimed and one dead-lettered.
///
/// **This is the text worth a server most**, and [`queue::COUNTS_MYSQL`]'s doc
/// says why: `count(case when … then 1 end)` is the aggregate filter nothing
/// else in the roster spells, and `cast(coalesce(sum(attempts), 0) as signed)`
/// is there because MySQL answers a `sum` over an integer column as a
/// `decimal`. Both are claims about what a server does with a construct, not
/// about what this module agrees with itself.
///
/// **The empty queue is the first assertion because it is where a `sum` would
/// diverge from a `count`.** § 6 means zero for a queue with no rows, and a
/// `sum(case when … then 1 end)` — the obvious spelling — answers `null` over
/// no rows while `count` ignores the `null` its `case` falls through to. Four
/// non-null zeros over an empty table is that difference, and it is also why
/// this helper hands back `Option`s.
///
/// **The four counters are asserted as one row rather than one at a time**, so
/// a pair swapped in the select list fails here: `pending` and `claimed` are
/// two `case when`s differing only in an ordinal, and each is a plausible
/// number for the other to answer.
///
/// **The third read is the dead-letter move, and it moves two counters at
/// once.** § 6 takes the row out of `nvs_jobs`, so the depth subquery gains one
/// and the `attempts` sum — which reads [`queue::JOBS_TABLE`] alone — loses the
/// attempt that job had used. A sum written over both tables would still be
/// `1` here.
///
/// **The last read is of the other queue**, which is what asks whether both
/// `?`s were bound to the queue the caller named: the subquery counts a table
/// the outer `where` never touches, so a text that scoped one and not the other
/// would report this queue's dead job under a queue that has none.
#[test]
fn a_framed_stats_counts_one_queue_across_both_of_its_tables() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-stats";
    const OTHER: &str = "nvs-stdlib-tests-framed-stats-elsewhere";
    const DUE: i64 = 9_000;
    const TAKEN: i64 = 9_500;
    const FAILED: i64 = 9_900;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);
    clear(&mut conn, OTHER);

    let four = |counts: [&str; 4]| counts.map(|c| Some(c.to_owned())).to_vec();

    assert_eq!(
        stats(&mut conn, QUEUE),
        four(["0", "0", "0", "0"]),
        "an empty queue answers four zeros and no nulls, which is the whole reason the counters \
         count rather than sum"
    );

    let dying = push(&mut conn, QUEUE, DUE, "1");
    let took = claim(&mut conn, QUEUE, TAKEN, 0);
    assert_eq!(took.len(), 1, "one due job is one claimed row");
    let waiting = push(&mut conn, QUEUE, DUE, "2");
    let _ = push(&mut conn, OTHER, DUE, "2");

    assert_eq!(
        stats(&mut conn, QUEUE),
        four(["1", "1", "1", "0"]),
        "one job in each state, and the attempt the claim wrote is the sum's whole content"
    );

    // § 6's move, run as `a_framed_status_walks_a_job_through_both_of_the_tables_it_can_be_in`
    // runs it: what this case adds is the counters that have to follow the row.
    let errors = queue::dead_errors(TAKEN, "IOError", "the receipt service refused the order");
    let lease = millis(TAKEN);
    let split = queue::DEAD_LETTER_MYSQL;
    conn.begin(None, false)
        .expect("the server opened the transaction");
    apply(
        &mut conn,
        split.first,
        &[
            Some(millis(FAILED).as_slice()),
            Some(errors.as_bytes()),
            Some(dying.as_bytes()),
            Some(lease.as_slice()),
        ],
    );
    apply(
        &mut conn,
        split.then,
        &[Some(dying.as_bytes()), Some(lease.as_slice())],
    );
    conn.commit()
        .expect("the server closed the transaction the pair was one moment inside");

    assert_eq!(
        stats(&mut conn, QUEUE),
        four(["1", "0", "0", "1"]),
        "the exhausted job is depth rather than work, and its attempt left `nvs_jobs` with it"
    );
    assert_eq!(
        status(&mut conn, &waiting, QUEUE).as_deref(),
        Some("0"),
        "the job the first counter is still reporting is the one that never left"
    );

    assert_eq!(
        stats(&mut conn, OTHER),
        four(["1", "0", "0", "0"]),
        "and the other queue sees its own pending job and none of this one's depth, which is both \
         `?`s bound to the name that was asked about"
    );
}
