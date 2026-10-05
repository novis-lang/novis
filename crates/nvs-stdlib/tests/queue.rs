//! `rule:concurrency/enqueue-commits-with-your-write`'s queue
//! statements, run against a real server rather than read.
//!
//! **These cases live in this crate and not in `nvs-db`, and one edge's
//! direction is the whole reason.** What they run is [`nvs_stdlib::queue`]'s
//! statement roster — § 2's schema has one home and that is it — and the only
//! thing that can run a statement is `nvs-db`'s own connection, which is
//! `PgConn`, `MySqlConn`, `MariaConn` or `TdsConn` as [`Conn`] holds it. `rule:core-classes/db-crate-boundary` fixes
//! which way that edge points: `nvs-stdlib` depends on `nvs-db` and never the
//! reverse, so a `crates/nvs-db/tests/queue.rs` would need a `use
//! nvs_stdlib::…` that closes a cycle in the workspace, and the alternative —
//! a copy of the statement text kept beside such a test — would assert over
//! the copy rather than over the statement a `push` actually issues. A test
//! target here sees both crates and needs neither compromise. `tools/nv/cmd/db-matrix.ts`'s
//! `SUITES` is what points this file at a server as well as `nvs-db`'s own.
//!
//! The skip rule is `nvs-db`'s, because it is the same harness: a process that
//! finds `NVS_DB_MATRIX_DRIVER` unset asserts nothing at all, so `bun
//! nv verify` is green on a machine with no containers and
//! `bun nv db-matrix` is what makes these assertions happen.
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
use nvs_db::tds::TdsScalar;
use nvs_db::{
    DbErrorKind, Driver, Isolation, MariaConn, MariaTarget, MySqlConn, MySqlRows, MySqlScalar,
    MySqlTarget, PgConn, PgTarget, QuerySpan, ServerError, TdsConn, TdsTarget,
};
use nvs_stdlib::queue;

/// How long the whole of one handshake here may take.
///
/// The bound `crates/nvs-db/tests/handshake.rs` uses, for its reason: the
/// server is a container the harness may have started moments ago, and a
/// matrix leg that hangs reports nothing at all.
const DEADLINE: Duration = Duration::from_secs(10);

/// The ordinal `Core\Queue\State::Pending` is, as `queue::schema`'s `state` column
/// holds it — `rule:enums/closed-integer-type`'s enums are their ordinal at runtime, so the enum and
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

/// This process's leg, when it names a driver [`queue::runs`] answers for.
///
/// Two shapes of `None` and neither is a failure: no harness at all, and a leg
/// reached by path rather than over a socket, which is SQLite, whose own
/// statements are executed by `queue_sqlite.rs` beside this file. The third
/// used to be a driver the queue had § 2's schema for and no statements for,
/// and [`queue::runs`] is still what is asked rather than that being dropped:
/// the roster is the seam, `queue_runs_on_every_driver` is what holds it to
/// `true`, and a driver falling out of it should move this gate rather than
/// fail every case under it. The per-case gates below narrow it once more,
/// because a case is written against one dialect's spelling even where several
/// have one.
fn endpoint() -> Option<Leg> {
    let endpoint = matrix::endpoint()?;
    let Location::Server(server) = endpoint.location else {
        return None;
    };
    // Every driver has § 2's schema, so what a leg needs is the narrower thing: a driver
    // `Core\Queue` has statements for. That roster is `queue::runs` and is not spelled again here.
    if !queue::runs(endpoint.driver) {
        return None;
    }
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
/// a second one to say: [`queue::INSERT_MYSQL`] and every statement beside
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

/// This process's SQL Server leg, or `None` because this run is one of the
/// others.
///
/// [`postgres`]'s shape and not [`framed`]'s: there is no lock to take here,
/// because InnoDB's scan-locking rule is InnoDB's and a leg runs one driver, so
/// the only writers on a SQL Server leg are the cases this gate opens.
fn sqlserver() -> Option<Leg> {
    let leg = endpoint()?;
    (leg.driver == Driver::SqlServer).then_some(leg)
}

/// A connection to `leg`'s server, as a request that found the pool empty opens
/// one.
///
/// **The targets differ in their type and in nothing else**, so the arms
/// are a macro rather than three copies of one field list: `rule:core-classes/db-connection-is-named`'s
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
        Driver::SqlServer => connect_as!(TdsTarget, TdsConn, Conn::SqlServer),
        // Spelled rather than left to a `_`, exactly as `crate::worker`'s own
        // opener spells them: [`endpoint`] skips SQLite before anything is
        // connected, because every target here is an address and that driver is
        // a path — so a sixth driver arrives here as a build failure rather
        // than as a refusal that has stopped being true.
        Driver::Sqlite => {
            unreachable!("no gate above opens a leg this file has no connection arm for")
        }
    }
}

/// The connection a case runs on, in the driver the harness named.
///
/// **Owned where [`Dialect`] is borrowed, and one arm per driver where that is
/// one arm per dialect**, which is `crates/nvs-cli/src/worker.rs`'s `Wire` one
/// crate over and for its reason: a case holds its connection for its whole
/// body, so there has to be a value that *is* the connection, and what every
/// statement below branches on is the dialect it is written in.
enum Conn {
    /// [`queue::INSERT_POSTGRES`] and the single-statement roster beside it.
    Postgres(PgConn),
    /// [`queue::INSERT_MYSQL`] and the roster whose claim and dead-letter
    /// move are [`queue::Split`]s.
    MySql(MySqlConn),
    /// MariaDB, which runs every one of MySQL's texts unchanged.
    MariaDb(MariaConn),
    /// [`queue::INSERT_SQLSERVER`] and the T-SQL roster beside it.
    SqlServer(TdsConn),
}

impl Conn {
    /// This connection borrowed as the dialect its statements are written in.
    fn dialect(&mut self) -> Dialect<'_> {
        match self {
            Conn::Postgres(postgres) => Dialect::Postgres(postgres),
            Conn::MySql(mysql) => Dialect::Framed(Framed::MySql(mysql)),
            Conn::MariaDb(maria) => Dialect::Framed(Framed::MariaDb(maria)),
            Conn::SqlServer(tds) => Dialect::SqlServer(tds),
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
            Conn::SqlServer(_) => Driver::SqlServer,
        }
    }

    /// A `text` parameter in the first position, as this driver spells one.
    ///
    /// The ad-hoc statements in this file — the deletes a case clears with, the
    /// counts it asserts over — are written in the driver's own spelling for
    /// the reason [`queue`]'s two rosters are: `rule:core-classes/db-parameters`'s rewriter is what
    /// a *program*'s statement goes through, and reaching for it here would put
    /// a second translator between a case and the server it is asserting about.
    fn text(&self) -> &'static str {
        match self {
            Conn::Postgres(_) => "$1::text",
            Conn::MySql(_) | Conn::MariaDb(_) => "?",
            Conn::SqlServer(_) => "@p1",
        }
    }

    /// `rule:core-classes/db-transactions`'s `BEGIN`, as the driver's own.
    fn begin(&mut self, isolation: Option<Isolation>, read_only: bool) -> io::Result<QuerySpan> {
        match self.dialect() {
            Dialect::Postgres(postgres) => postgres.begin(isolation, read_only),
            Dialect::Framed(mut framed) => framed.begin(isolation, read_only),
            Dialect::SqlServer(tds) => tds.begin(isolation, read_only),
        }
    }

    /// § 7's `COMMIT`.
    fn commit(&mut self) -> io::Result<QuerySpan> {
        match self.dialect() {
            Dialect::Postgres(postgres) => postgres.commit(),
            Dialect::Framed(mut framed) => framed.commit(),
            Dialect::SqlServer(tds) => tds.commit(),
        }
    }

    /// § 7's `ROLLBACK`.
    fn roll_back(&mut self) -> io::Result<QuerySpan> {
        match self.dialect() {
            Dialect::Postgres(postgres) => postgres.roll_back(),
            Dialect::Framed(mut framed) => framed.roll_back(),
            Dialect::SqlServer(tds) => tds.roll_back(),
        }
    }

    /// How many transactions deep this connection is, which is what a case
    /// asserts a commit or a rollback by.
    fn depth(&self) -> u32 {
        match self {
            Conn::Postgres(postgres) => postgres.depth(),
            Conn::MySql(mysql) => mysql.depth(),
            Conn::MariaDb(maria) => maria.depth(),
            Conn::SqlServer(tds) => tds.depth(),
        }
    }
}

/// A borrowed [`Conn`], as the dialect [`nvs_stdlib::queue`] writes its
/// statements in.
///
/// The same arms `crates/nvs-cli/src/worker.rs`'s `Dialect` has, and for the
/// same reason: § 2's schema is a migration list per dialect and §§ 4 and 6's
/// statements a spelling per dialect, so an arm here is a roster and a driver
/// that shares another's gets no arm of its own.
enum Dialect<'a> {
    /// One statement in, one answer out.
    Postgres(&'a mut PgConn),
    /// The framed drivers, three of whose statements are pairs inside one
    /// transaction — and MariaDB runs every one of them unchanged.
    Framed(Framed<'a>),
    /// T-SQL, whose enqueue, dead-letter move and delete are [`queue::Split`]s
    /// inside one transaction and whose insert answers its id through an
    /// `output` clause rather than a `returning` or an OK packet.
    SqlServer(&'a mut TdsConn),
}

/// The two drivers that share one dialect and one send path, borrowed as one.
///
/// **MariaDB is its own driver above the framing, not inside it**, and what it
/// does not duplicate is the wire: `MariaConn::query` delegates into the same
/// framing and hands back the same [`MySqlRows`]. So the only difference this
/// file can observe between the two is the *type of the borrow*, and without
/// this every reader below would grow a second arm copying the first line for
/// line. `nvs-cli` has the same enum for the same reason, on the other side of
/// `rule:core-classes/db-crate-boundary`'s crate graph.
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

    /// `rule:core-classes/db-transactions`'s `START TRANSACTION`.
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
/// **A `Once` rather than a step every case pays**, and not for the time: two
/// sessions building one table in the same moment answer a uniqueness error on
/// the catalog rather than a no-op. Cargo runs these cases on threads of one
/// process, so that race is the ordinary case here and not a rare one.
///
/// **Dropped and rebuilt rather than created where absent**, because the schema
/// is a value: a table an older Novis built is the shape that value has since
/// changed, and it would be kept by exactly the `if not exists` the emitter no
/// longer writes ([`queue::migration`]). Starting from nothing is the one form
/// of convergence a fixture can reach without a catalog reader, and this binary
/// owns both tables outright.
fn schema(leg: &Leg) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let mut conn = open(leg);
        for table in ["nvs_jobs", "nvs_dead_jobs"] {
            apply(&mut conn, &format!("drop table if exists {table}"), &[]);
        }
        for step in queue::migration(leg.driver) {
            apply(&mut conn, &step.sql, &[]);
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
/// **The `create` is a spelling per dialect, split the way [`queue::Split`]
/// splits § 4's and for its reasons.** `bigserial` is a sequence and
/// a default in one word and the framed servers have neither, `text` is a column
/// InnoDB will not index at an unbounded width, and `engine=innodb` is what makes
/// a rollback of this table a rollback at all. T-SQL's spelling is a third for a
/// reason of its own: `if not exists` is not a clause `create table` takes on
/// this backend, so the existence question is asked of the catalog and the
/// `create` is what the answer guards. The `delete` below stays one text,
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
            Driver::SqlServer => {
                "if object_id('nvs_stdlib_tests_orders', 'U') is null \
                 create table nvs_stdlib_tests_orders \
                 (id bigint identity(1, 1) primary key, queue nvarchar(255) not null)"
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
/// **Each arm is a protocol and not a spelling of one walk**, which is why
/// nothing here is shared between them: PostgreSQL's extended query answers a
/// column as the text it renders to, where the framed drivers and TDS each send
/// a `bigint` as octets against the column definition it arrived under —
/// [`rendered`] and [`tds_rendered`] are the whole of what that costs a case.
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
        Dialect::SqlServer(tds) => {
            let mut answered = tds.query(sql, bound).expect("the server ran the statement");
            // Described before the first row for the framed arm's borrow
            // reason, and read against those definitions for its decoding one:
            // `nvs_db::tds::scalar` measures a value by the column it arrived
            // under.
            let columns = answered.columns().to_vec();
            let mut all = Vec::new();
            while let Some(row) = answered
                .next_row()
                .expect("a row, or the end of the stream")
            {
                let mut one = Vec::with_capacity(columns.len());
                for (at, column) in columns.iter().enumerate() {
                    let body = row.column(at).expect("the row has that column");
                    one.push(tds_rendered(
                        nvs_db::tds::scalar(column, body).expect("the column is one § 9 maps"),
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

/// One T-SQL column as the text the same column arrives as on PostgreSQL.
///
/// [`rendered`]'s twin over the other binary protocol, and shorter than it by
/// exactly what § 9 says about this driver: every integer width arrives on one
/// row of that table rather than on a signed and an unsigned one, because
/// `tinyint` is the only unsigned type SQL Server has and every value of it fits
/// an `int`. Text is owned or borrowed depending on whether the column was one
/// of the `N` types, which is what the `Cow` carries and what [`String::from`]
/// flattens.
fn tds_rendered(read: TdsScalar<'_>) -> Option<String> {
    match read {
        TdsScalar::Null => None,
        TdsScalar::Int(at) => Some(at.to_string()),
        TdsScalar::Text(text) => Some(text.into_owned()),
        TdsScalar::Bytes(octets) => Some(
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
        Dialect::SqlServer(tds) => tds.execute_many(sql, &[bound]),
    }
    .expect("the server ran the statement")
}

/// One pending job on `queue`, due at `at`, answering the id the insert wrote.
///
/// `max_attempts` is the case's whole lever over § 6: a job pushed with `1` is
/// exhausted by its first claim, and one pushed with more is owed another.
///
/// **A push with no dedupe key is one statement in every dialect**, which is
/// why this is not a [`queue::Split`] anywhere: [`queue::INSERT_MYSQL`]'s first
/// half is the dedupe read, and `Core\Queue::push` skips it for exactly this
/// case. What does differ is where the id comes from — PostgreSQL's
/// `returning`, T-SQL's `output inserted.id`, and the framed drivers' own OK
/// packet, which [`nvs_db::MySqlRows::last_id`] carries and a `select
/// last_insert_id()` would pay a third round trip for.
///
/// **T-SQL binds [`queue::INSERT_MYSQL`]'s list in [`queue::INSERT_MYSQL`]'s
/// order**, so the two share one array here: [`queue::INSERT_SQLSERVER`] writes
/// the same columns in the same places, and what it adds is a cast at each one
/// the table types as a number, which is a property of the text rather than of
/// what is bound to it.
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
                // No tag: nothing here groups jobs, and `$10` is the option
                // `push` leaves null on every enqueue that does not name one.
                None,
                // The narrowing pair, null for the same reason a helper's push
                // is: this stands in for a context that narrowed neither half.
                None,
                None,
            ],
        );
    }

    // `INSERT_MYSQL.then`'s own order, which is not `INSERT_POSTGRES`'s: the
    // dedupe key is ninth and tenth here and first there, and `attempts` is a
    // literal rather than a parameter. Two slots because it is written to two
    // columns and a `?` cannot be named twice, which that constant's doc owns.
    let bound = [
        Some(queue.as_bytes()),
        Some(script),
        Some(args),
        Some(PENDING),
        Some(max_attempts.as_bytes()),
        Some(backoff),
        Some(at.as_slice()),
        None,
        None,
        Some(at.as_slice()),
        // The tag, then the narrowing pair, at the end for the reason
        // `INSERT_POSTGRES`'s doc gives: a value added anywhere else is a column
        // list every other statement has to be checked against.
        None,
        None,
        None,
    ];
    if conn.driver() == Driver::SqlServer {
        // A statement whose `output` clause answers a row is read the way every
        // other answer in this file is, and this driver has no `last_id` at all.
        return one(conn, queue::INSERT_SQLSERVER.then, &bound);
    }
    let Dialect::Framed(mut framed) = conn.dialect() else {
        unreachable!("every driver but PostgreSQL and SQL Server is the framed dialect here")
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
        .expect("the insert answered the `AUTO_INCREMENT` id `queue::schema` declares")
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
    // `INSERT_MYSQL.then`'s order, as `push` sends it, with the two dedupe slots
    // filled: those are the ones an ordinary push leaves null. The key goes out
    // twice because `dedupe_key` and `dedupe_pending` are one value in two
    // columns, and the second is what `nvs_jobs_dedupe` is built over.
    let bound = [
        Some(queue.as_bytes()),
        Some(&b"scripts/receipt.nvs"[..]),
        Some(&br#"{"order":7}"#[..]),
        Some(PENDING),
        Some(&b"3"[..]),
        Some(&b"1000"[..]),
        Some(at.as_slice()),
        Some(key.as_bytes()),
        Some(key.as_bytes()),
        Some(at.as_slice()),
        // No tag: a key and a tag are opposites, and this helper is the key's
        // half — `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`.
        None,
        // The narrowing pair, which a hand-built push narrows neither half of.
        None,
        None,
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
        .expect("the insert answered the `AUTO_INCREMENT` id `queue::schema` declares")
        .to_string())
}

/// [`queue::INSERT_POSTGRES`] carrying the two values [`push`] leaves null: the
/// dedupe key that admits one pending job, and the tag that names a group of
/// them.
///
/// **It answers what the statement answered, which is not always a row it
/// wrote.** A key some pending job already holds inserts nothing and comes back
/// with that job's own id off the `existing` arm, which is why there is no
/// `Result` here where [`push_keyed`] has one: on this dialect a deduped push is
/// an ordinary answer rather than the server's refusal. That is the whole of the
/// difference between one statement and [`queue::INSERT_MYSQL`]'s pair, and it
/// is what makes the key half below an equality rather than an error kind.
///
/// **One helper for both slots rather than one each**, because what the case
/// calling it asserts is that they are opposites: the same push with the other
/// value filled is what separates a group of many from a group of one.
fn push_marked(
    conn: &mut Conn,
    queue: &str,
    at: i64,
    key: Option<&str>,
    tag: Option<&str>,
) -> String {
    assert_eq!(
        conn.driver(),
        Driver::Postgres,
        "this is `INSERT_POSTGRES`, and only PostgreSQL runs it"
    );
    let at = millis(at);
    // `INSERT_POSTGRES`'s order, as `push` sends it: the key is first because
    // `$1` is read by the guard before it is written to either of the two dedupe
    // columns, and the tag is last for the reason that constant's doc gives.
    one(
        conn,
        queue::INSERT_POSTGRES,
        &[
            key.map(str::as_bytes),
            Some(queue.as_bytes()),
            Some(&b"scripts/receipt.nvs"[..]),
            Some(&br#"{"order":7}"#[..]),
            Some(PENDING),
            Some(&b"3"[..]),
            Some(&b"1000"[..]),
            Some(at.as_slice()),
            Some(at.as_slice()),
            tag.map(str::as_bytes),
            // The narrowing pair, which neither slot this helper fills is.
            None,
            None,
        ],
    )
}

/// [`queue::INSERT_MYSQL`]'s `first` — the guard half — answering the pending
/// job `key` already has, where it has one.
///
/// This is the read no case in this file had issued: `dedupe_pending = ?`
/// resolves against the column the inserts above write and every transition out
/// of `Pending` clears, so what a real server answers here is whether those
/// statements maintain it rather than whether one of them ran.
fn pending_for(conn: &mut Conn, key: &str) -> Option<String> {
    let mut found = rows(conn, queue::INSERT_MYSQL.first, &[Some(key.as_bytes())]);
    (!found.is_empty()).then(|| found.remove(0).remove(0).expect("`id` is not null"))
}

/// One claim against `queue`, taken at `now`, returning jobs whose lease was
/// taken at or before `cutoff` as well as the ones nothing holds.
///
/// **The values `crates/nvs-cli/src/worker.rs`'s own claim sends, in its
/// order**: the queue, now and the cutoff, with the queue sent twice on the
/// framed dialect because both of [`queue::CLAIM_MYSQL`]'s arms read it. The
/// answer is [`queue::CLAIM_POSTGRES`]'s `returning` list either way — that is
/// what [`queue::CLAIM_MYSQL`]'s `select` is written to name. What differs is how
/// many statements it took: the pair's `update` is keyed by the id its `select`
/// just locked, and the transaction around them is what carries the lock across
/// the gap a single statement did not have.
fn claim(conn: &mut Conn, queue: &str, now: i64, cutoff: i64) -> Vec<Vec<Option<String>>> {
    let (now, cutoff) = (millis(now), millis(cutoff));
    if conn.driver() == Driver::Postgres {
        let bound = [
            Some(queue.as_bytes()),
            Some(now.as_slice()),
            Some(cutoff.as_slice()),
        ];
        return rows(conn, queue::CLAIM_POSTGRES, &bound);
    }

    let bound = [
        Some(queue.as_bytes()),
        Some(now.as_slice()),
        Some(queue.as_bytes()),
        Some(cutoff.as_slice()),
    ];
    // At the worker's own level, which `crates/nvs-cli/src/worker.rs`'s
    // `framed_claim` owns the reason for.
    conn.begin(Some(nvs_db::Isolation::ReadCommitted), false)
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
/// that is the whole of the difference: `$1` and `@p1` are numbers a statement
/// may repeat and `?` is a position that cannot, so one text with two
/// occurrences is one parameter on the other dialects and two there.
/// [`Conn::text`]'s doc owns why these ad-hoc statements are written per driver
/// rather than put through § 5's rewriter.
fn landed(conn: &mut Conn, queue: &str) -> (String, String) {
    let name = queue.as_bytes();
    let (sql, bound) = match conn.driver() {
        Driver::Postgres => (
            "select (select count(*) from nvs_jobs where queue = $1::text), \
                    (select count(*) from nvs_stdlib_tests_orders where queue = $1::text)",
            vec![Some(name)],
        ),
        Driver::SqlServer => (
            "select (select count(*) from nvs_jobs where queue = @p1), \
                    (select count(*) from nvs_stdlib_tests_orders where queue = @p1)",
            vec![Some(name)],
        ),
        _ => (
            "select (select count(*) from nvs_jobs where queue = ?), \
                    (select count(*) from nvs_stdlib_tests_orders where queue = ?)",
            vec![Some(name), Some(name)],
        ),
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

/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `delete` on the
/// framed dialect, answering the count the server says it affected across both
/// of the tables the statement names.
///
/// **[`cancel`]'s shape and for [`cancel`]'s reason**: [`queue::DELETE_MYSQL`]
/// is read off its affected count where [`queue::DELETE_POSTGRES`] is read off
/// a returned row, so a helper spanning both dialects would hand back two
/// shapes. The PostgreSQL text is sent by
/// [`delete_answers_false_for_a_claimed_job_and_true_for_a_pending_one`], which
/// is where that half is asserted.
///
/// **Two placeholders and not four**, unlike [`status`]: the pair is named once
/// in the derived table the multi-table delete is driven from, and that
/// constant's doc owns why.
fn delete(conn: &mut Conn, id: &str, queue: &str) -> u64 {
    apply(
        conn,
        queue::DELETE_MYSQL,
        &[Some(id.as_bytes()), Some(queue.as_bytes())],
    )
}

/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `purge` over the jobs table, in
/// whichever dialect this leg runs, answering the count it removed.
///
/// **One helper across both dialects, unlike [`cancel`]**, because both texts
/// are read off the affected count: the count *is* the member's answer, so
/// there is no second shape for a caller to unwrap.
///
/// **The framed leg binds the same five values in eight slots**, which
/// [`queue::PURGE_MYSQL`]'s doc owns: a `$n` may be named twice and a `?` may
/// not, so each of the two null-checked options costs a slot per mention. Doing
/// that here rather than at each call site is [`status`]'s arrangement, for
/// [`status`]'s reason.
fn purge(
    conn: &mut Conn,
    queue: &str,
    state: Option<&str>,
    tag: Option<&str>,
    before: Option<i64>,
    limit: &str,
) -> u64 {
    let before = before.map(millis);
    let before = before.as_deref();
    let (name, state, tag) = (
        Some(queue.as_bytes()),
        state.map(str::as_bytes),
        tag.map(str::as_bytes),
    );
    let bound = Some(limit.as_bytes());
    let (sql, sent) = if conn.driver() == Driver::Postgres {
        (queue::PURGE_POSTGRES, vec![name, state, tag, before, bound])
    } else {
        (
            queue::PURGE_MYSQL,
            vec![name, state, state, tag, tag, before, before, bound],
        )
    };
    apply(conn, sql, &sent)
}

/// ADR 0153 § 6's other table: [`purge`] with `state: Dead`, which is a
/// different statement because it is a different table.
///
/// It takes no state at all, and [`queue::PURGE_DEAD_POSTGRES`]'s doc says why:
/// [`queue::DEAD_TABLE`] has no `state` column, since being in that table is
/// what `Dead` is.
fn purge_dead(
    conn: &mut Conn,
    queue: &str,
    tag: Option<&str>,
    before: Option<i64>,
    limit: &str,
) -> u64 {
    let before = before.map(millis);
    let before = before.as_deref();
    let (name, tag) = (Some(queue.as_bytes()), tag.map(str::as_bytes));
    let bound = Some(limit.as_bytes());
    let (sql, sent) = if conn.driver() == Driver::Postgres {
        (queue::PURGE_DEAD_POSTGRES, vec![name, tag, before, bound])
    } else {
        (
            queue::PURGE_DEAD_MYSQL,
            vec![name, tag, tag, before, before, bound],
        )
    };
    apply(conn, sql, &sent)
}

/// One job put straight into `state`, with `lease` written to `claimed_at`.
///
/// **Setup and not an assertion**, which is why this is an ad-hoc `update`
/// rather than the statement a worker would have run to get there. A purge is
/// asked what a *selection* takes, and how a row reached the state it is in is
/// a question [`queue::SUCCEEDED_POSTGRES`], [`queue::CANCEL_MYSQL`] and § 6's
/// move each already have a case of their own for. Walking every row through
/// its own claim would put four round trips in front of each assertion and make
/// which row a claim took part of what the purge case depends on.
fn set_state(conn: &mut Conn, id: &str, state: &str, lease: Option<i64>) {
    let lease = lease.map(millis);
    let lease = lease.as_deref();
    let sql = if conn.driver() == Driver::Postgres {
        "update nvs_jobs set state = $1::smallint, claimed_at = $2::bigint where id = $3::bigint"
    } else {
        "update nvs_jobs set state = ?, claimed_at = ? where id = ?"
    };
    assert_eq!(
        apply(
            conn,
            sql,
            &[Some(state.as_bytes()), lease, Some(id.as_bytes())]
        ),
        1,
        "a case's own setup names one row by its id"
    );
}

/// §§ 1 and 6's `stats` on the framed dialect, as the five columns
/// [`queue::COUNTS_MYSQL`] answers with.
///
/// **The columns stay `Option`**, unlike [`one`]'s, because half of what the
/// case that calls this asks is that none of them is ever null: `count`
/// over no rows and `coalesce`d `sum` are what make that true, and a helper
/// that unwrapped here would assert it by panicking somewhere the message
/// blamed the helper.
///
/// **The name is bound three times** for [`landed`]'s reason: the outer `where`
/// and the two dead-letter subqueries are three `?` positions, where
/// [`queue::COUNTS_POSTGRES`] names `$1` three times.
fn stats(conn: &mut Conn, queue: &str) -> Vec<Option<String>> {
    let name = queue.as_bytes();
    let mut answered = rows(
        conn,
        queue::COUNTS_MYSQL,
        &[Some(name), Some(name), Some(name)],
    );
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

/// Every statement the queue sends has a text in SQL Server's dialect, so the
/// roster is whole rather than four dialects deep.
///
/// **The one case in this file that asks no server**, and it is here rather
/// than beside the unit tests because what it reads is `queue::texts` — the
/// roster the cases below send, which is `pub` for this target's sake. Nothing
/// in it needs a leg, so it asserts on a machine with no containers too, which
/// is where a text that was never written would otherwise go unnoticed until a
/// matrix run.
///
/// Two properties, in the order a wrong text fails them, and the third — that
/// the roster is the *same set of members* the other dialects have — is
/// [`queue_runs_on_every_driver`]'s, which asks it of all five at once. Each
/// text names `@p1`, which is this driver's marker and the one spelling
/// `sp_prepexec`'s parameter declaration writes, and none of them names another
/// dialect's — a `$1` or a `?` reaches the server as a syntax error naming a
/// statement no case here can see. And the markers run `1..n` with no gap:
/// the driver declares one parameter per number it wrote, so a skipped one is a
/// value bound into the column beside the one it was meant for, which the
/// server accepts and answers wrongly.
#[test]
fn every_statement_the_queue_sends_has_a_sql_server_text() {
    for (member, sql) in queue::texts(Driver::SqlServer) {
        for absent in ["$1", "?", "::", "returning", "for update", "limit "] {
            assert!(
                !sql.contains(absent),
                "{member}'s SQL Server text spells `{absent}`, which is another dialect's: {sql}"
            );
        }
        let mut markers: Vec<usize> = Vec::new();
        let mut rest = sql;
        while let Some(at) = rest.find("@p") {
            rest = &rest[at + "@p".len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            markers.push(
                digits
                    .parse()
                    .unwrap_or_else(|_| panic!("{member}'s `@p` names no number: {sql}")),
            );
        }
        markers.sort_unstable();
        markers.dedup();
        assert!(
            !markers.is_empty(),
            "{member}'s SQL Server text binds nothing, so it is not the statement it replaces"
        );
        assert_eq!(
            markers,
            (1..=markers.len()).collect::<Vec<usize>>(),
            "{member}'s SQL Server text numbers its markers `1..n` with no gap, because the \
             driver declares one parameter per number: {sql}"
        );
    }
}

/// `Core\Queue` sends its statements over every driver `Core\Db` opens, which
/// is what `rule:core-classes/queue-storage-is-a-table`'s one schema is for.
///
/// **The predicate and the roster are asserted together, because separately
/// neither is a fact.** [`queue::runs`] is a `match` a session can widen with
/// one arm, and a roster is a list of texts nothing obliges a member to be in;
/// what makes the answer true is that every driver's roster names the same
/// members, so a driver the predicate claims for cannot be one the texts
/// skipped. Every other reader takes the predicate as an oracle — [`endpoint`]
/// above is one — so this is where it is held to something.
///
/// **The one case here that asks no server**, as the SQL Server texts' own case
/// is and for its reason: a roster that had lost a member would otherwise go
/// unnoticed until a matrix run.
#[test]
fn queue_runs_on_every_driver() {
    fn members(driver: Driver) -> Vec<&'static str> {
        let mut named: Vec<&'static str> = queue::texts(driver)
            .into_iter()
            .map(|(member, _)| member)
            .collect();
        named.sort_unstable();
        named.dedup();
        named
    }
    let roster = members(Driver::Postgres);
    assert!(
        !roster.is_empty(),
        "the dialect every other one is compared against has statements of its own"
    );
    for driver in Driver::ALL {
        assert!(
            queue::runs(driver),
            "{driver:?}: `Core\\Queue` opens a connection to every driver `Core\\Db` does, so \
             there is none it may then refuse to send a statement over"
        );
        assert_eq!(
            members(driver),
            roster,
            "{driver:?}: a member the queue sends a statement for has one in every dialect, or \
             the roster `queue::runs` answers for is not whole"
        );
    }
}

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
    let errors = queue::dead_errors(
        None,
        now,
        "IOError",
        "the receipt service refused the order",
    );
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
                    // The attempt that just failed, appended to the array the
                    // claim answered with — here `null`, this being the first.
                    Some(
                        queue::dead_errors(None, now, "IOError", "the endpoint is down").as_bytes(),
                    ),
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
                        queue::dead_errors(None, latest, "IOError", "the endpoint is still down")
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

/// `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them` against a real
/// server: one tag admits every job that names it, and one key admits one.
///
/// **The two halves are the same push with the other slot filled**, sent down
/// one connection at one queue, so what separates a group of many from a group
/// of one is the column and nothing else. That is what makes this the rule
/// rather than two unrelated counts: a schema folding the pair into one column
/// would cap every group at one pending job, silently, at the enqueue that
/// created the group.
///
/// **The key half is asserted by what the statement answered and not by a
/// refusal.** [`queue::INSERT_POSTGRES`] is one statement, so a key a pending
/// job already holds comes back with that job's own id off the `existing` arm
/// and writes nothing, where [`queue::INSERT_MYSQL`]'s pair is refused by
/// `nvs_jobs_dedupe` —
/// [`a_framed_dedupe_push_is_refused_by_the_index_and_not_by_the_guard`] is that
/// side. This is also the first case in this file to bind `$1` at all: every
/// other push here leaves the key null, so the `existing` arm has never met a
/// server.
///
/// **The group is counted rather than read off the rows it was pushed as**,
/// because what the rule says is about a population: a tag that admitted two of
/// the three still answers plausibly for either one of them.
#[test]
fn many_pending_jobs_share_one_tag_and_two_pending_jobs_never_share_one_key() {
    const QUEUE: &str = "nvs-stdlib-tests-tag-and-key";
    const TAG: &str = "nvs-stdlib-tests-tag-and-key:batch";
    // Unique across the whole table rather than within the queue, so it names
    // this case: `nvs_jobs_dedupe` covers the column and not `(queue, column)`.
    const KEY: &str = "nvs-stdlib-tests-tag-and-key:receipt:7";
    // The group, as § 2's own index would be read: `(queue, tag)` leftmost
    // first, narrowed to the state the key's half is about.
    const TAGGED: &str = "select count(*) from nvs_jobs \
         where queue = $1::text and tag = $2::text and state = $3::smallint";
    const DUE: i64 = 1_000;

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    // The group. Three pushes naming one tag, and nothing about the tag makes
    // the second or the third of them different from the first.
    let mut grouped: Vec<String> = (0..3)
        .map(|nth| push_marked(&mut conn, QUEUE, DUE + nth, None, Some(TAG)))
        .collect();
    let pushed = grouped.len();
    grouped.sort();
    grouped.dedup();
    assert_eq!(
        grouped.len(),
        pushed,
        "each push wrote a row of its own, and none of them answered another's: {grouped:?}"
    );
    let counted = [Some(QUEUE.as_bytes()), Some(TAG.as_bytes()), Some(PENDING)];
    assert_eq!(
        one(&mut conn, TAGGED, &counted),
        "3",
        "every job that named the tag is pending under it, which is what a group is"
    );

    // The key, sent as that same push carrying the tag as well, so the two
    // columns are asserted about one row rather than about two arrangements.
    let first = push_marked(&mut conn, QUEUE, DUE, Some(KEY), Some(TAG));
    let again = push_marked(&mut conn, QUEUE, DUE, Some(KEY), Some(TAG));
    assert_eq!(
        again, first,
        "the second push answered the pending job the key already had, off the `existing` arm"
    );
    assert_eq!(
        one(
            &mut conn,
            "select count(*) from nvs_jobs where dedupe_pending = $1::text",
            &[Some(KEY.as_bytes())],
        ),
        "1",
        "the key admits one pending job, and the push it turned away wrote no row"
    );
    assert_eq!(
        one(&mut conn, TAGGED, &counted),
        "4",
        "five pushes named the tag and the tag took every row that was written"
    );
}

/// § 2's two ordinary arms against a real PostgreSQL server: `delete` removes a
/// pending job and answers `true`, and refuses a claimed one and answers
/// `false` — `rule:concurrency/queue-deletion-is-explicit-and-bounded`.
///
/// **The bound is asserted on both sides**, because either half alone passes
/// against a statement that has stopped asking. A [`queue::DELETE_POSTGRES`]
/// whose `state <> 1` had been dropped removes both rows and looks right on the
/// pending one; a text whose predicate refused everything removes neither and
/// looks right on the claimed one. Naming the two together is what pins the
/// place the statement stops.
///
/// **The claimed row is asserted still *claimed*, not merely still there**,
/// which is one read rather than two: [`queue::STATUS_POSTGRES`] answers
/// nothing at all for a row that was removed, so `Claimed` says both that the
/// job survived the call and that the lease the worker is running under did.
///
/// **The answer is the row the statement returned**, which is what
/// `Core\Queue::delete`'s `bool` is read off — the two data-modifying CTEs
/// `union all` their `returning id` and the member asks whether a row came
/// back. That is also why the receipt is asserted by value: either arm answers
/// one row, and reading the id back is what says which job it was about.
///
/// **The two jobs are separated by `run_at` rather than by the order they were
/// pushed**, so which one is claimed is decided by the statement rather than by
/// this case: [`queue::CLAIM_POSTGRES`] takes the oldest *due* job, and a job
/// due a second later is not due at the instant this claim is taken whatever
/// else the table holds.
#[test]
fn delete_answers_false_for_a_claimed_job_and_true_for_a_pending_one() {
    const QUEUE: &str = "nvs-stdlib-tests-delete";

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let now = queue::now_millis();
    let held = push(&mut conn, QUEUE, now, "3");
    let waiting = push(&mut conn, QUEUE, now + 1_000, "3");

    let took = claim(&mut conn, QUEUE, now, now);
    assert_eq!(took.len(), 1, "the claim took the one job that was due");
    assert_eq!(
        took[0][ID].as_deref(),
        Some(held.as_str()),
        "the row a worker now holds is the one this case is about"
    );

    let name = Some(QUEUE.as_bytes());
    let refused = rows(
        &mut conn,
        queue::DELETE_POSTGRES,
        &[Some(held.as_bytes()), name],
    );
    assert!(
        refused.is_empty(),
        "a claimed job is not removable, so neither arm answered a row: {refused:?}"
    );
    assert_eq!(
        status(&mut conn, &held, QUEUE).as_deref(),
        Some("1"),
        "the job is still there and still claimed, under the lease its worker is running on"
    );

    let removed = rows(
        &mut conn,
        queue::DELETE_POSTGRES,
        &[Some(waiting.as_bytes()), name],
    );
    assert_eq!(
        removed.len(),
        1,
        "the pending job was removed, and one row is what makes the member's answer `true`"
    );
    assert_eq!(
        removed[0][0].as_deref(),
        Some(waiting.as_str()),
        "the row answered is the receipt the call named"
    );
    assert_eq!(
        status(&mut conn, &waiting, QUEUE),
        None,
        "the pending row is gone rather than moved, so `status` has nothing to answer about"
    );
}

/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `purge` against a
/// real PostgreSQL server: the default set is what has finished, each of the
/// three filters narrows what that set reaches, and no call takes more rows than
/// its `limit`.
///
/// **One queue holding one row in every state there is**, because what the rule
/// says is about a selection, and a selection is only ever wrong about the rows
/// it was not asked to take. A case with nothing pending, claimed or
/// dead-lettered in it passes against a text whose `where` had been dropped
/// altogether.
///
/// **The bound is asserted before the group it bounds is emptied**, and it is
/// the first assertion for that reason: a `limit` of one over two rows that both
/// match is the only arrangement in which taking one row rather than two is
/// visible at all. Which of the two goes is the statement's `order by id` — the
/// identity column numbers rows in the order they were enqueued, so the oldest
/// is what a retention sweep drains first.
///
/// **`before` is asserted on both sides**, one millisecond apart, because either
/// half alone passes against a text that had stopped asking: an inclusive bound
/// and an exclusive one differ on exactly one instant, and that instant is the
/// one a caller passing `now` writes.
///
/// **`Dead` and `Pending` are each asked twice** — once as rows a purge naming
/// no state leaves alone, and once as a selection that names them and takes
/// them. Opt-in is two claims and not one: a text that could never reach the
/// dead-letter table would pass the first half of each pair.
///
/// **`Claimed` is asked of the statement and not only of the member.** The
/// member refuses `State::Claimed` at the call, which is where a caller finds
/// out; this asserts that a call that got past it anyway would still remove
/// nothing, which is the arm ADR 0153 § 2 says is not a policy choice.
#[test]
fn purge_removes_what_has_finished_within_its_filters_and_stops_at_its_limit() {
    const QUEUE: &str = "nvs-stdlib-tests-purge";
    const TAG: &str = "nvs-stdlib-tests-purge:batch";
    const DUE: i64 = 20_000;
    const LATER: i64 = 30_000;
    const LEASE: i64 = 21_000;
    const FAILED: i64 = 21_500;

    let Some(server) = postgres() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    // Six rows, one per state and one of them under no tag at all, pushed a
    // millisecond apart so `created_at` orders them the way `id` does.
    let done = push_marked(&mut conn, QUEUE, DUE, None, Some(TAG));
    let gone = push_marked(&mut conn, QUEUE, DUE + 1, None, Some(TAG));
    let plain = push_marked(&mut conn, QUEUE, DUE + 2, None, None);
    let buried = push_marked(&mut conn, QUEUE, DUE + 3, None, Some(TAG));
    let held = push_marked(&mut conn, QUEUE, DUE + 4, None, Some(TAG));
    // The one row left pending, and the newest of the six, so the age filter
    // below has a row on the far side of it as well.
    let waiting = push_marked(&mut conn, QUEUE, LATER, None, Some(TAG));

    set_state(&mut conn, &done, "2", None);
    set_state(&mut conn, &gone, "4", None);
    set_state(&mut conn, &plain, "2", None);
    set_state(&mut conn, &held, "1", None);
    // The dead-lettered row goes through § 6's own statement, because the row it
    // writes into the other table is what this case's last assertion is about.
    set_state(&mut conn, &buried, "1", Some(LEASE));
    assert_eq!(
        apply(
            &mut conn,
            queue::DEAD_LETTER_POSTGRES,
            &[
                Some(buried.as_bytes()),
                Some(millis(LEASE).as_slice()),
                Some(millis(FAILED).as_slice()),
                Some(
                    queue::dead_errors(None, LEASE, "IOError", "the receipt service refused")
                        .as_bytes()
                ),
            ],
        ),
        1,
        "the row moved to the other table, which is where a `state: Dead` selection looks"
    );

    assert_eq!(
        purge(&mut conn, QUEUE, None, Some(TAG), None, "1"),
        1,
        "two tagged rows had finished and the bound took one of them"
    );
    assert_eq!(
        status(&mut conn, &done, QUEUE),
        None,
        "and the one it took is the oldest, which is what makes a bounded loop drain"
    );
    assert_eq!(
        status(&mut conn, &gone, QUEUE).as_deref(),
        Some("4"),
        "the row the bound stopped short of is untouched rather than half-removed"
    );

    assert_eq!(
        purge(&mut conn, QUEUE, None, Some(TAG), None, "10"),
        1,
        "the rest of the group is one row, and a roomy bound takes it"
    );
    assert_eq!(
        status(&mut conn, &plain, QUEUE).as_deref(),
        Some("2"),
        "the finished job that named no tag is not in the group, so the tag is what narrowed it"
    );

    assert_eq!(
        purge(&mut conn, QUEUE, None, None, Some(DUE + 2), "10"),
        0,
        "`before` is the instant the row was enqueued and not one after it"
    );
    assert_eq!(
        purge(&mut conn, QUEUE, None, None, Some(DUE + 3), "10"),
        1,
        "and one millisecond later the same row is inside the age the call named"
    );

    assert_eq!(
        purge(&mut conn, QUEUE, None, None, None, "10"),
        0,
        "what is left has not finished, and a purge naming no state removes what has"
    );
    assert_eq!(
        (
            status(&mut conn, &waiting, QUEUE).as_deref(),
            status(&mut conn, &held, QUEUE).as_deref(),
            status(&mut conn, &buried, QUEUE).as_deref(),
        ),
        (Some("0"), Some("1"), Some("3")),
        "a retention sweep leaves work waiting, work in flight and the record that work was lost"
    );

    assert_eq!(
        purge(&mut conn, QUEUE, Some("1"), None, None, "10"),
        0,
        "a claimed job is not removable, and no option reaches it"
    );
    assert_eq!(
        purge(&mut conn, QUEUE, Some("0"), None, None, "10"),
        1,
        "`Pending` is opt-in rather than unreachable, which is what makes it a default and not a \
         rule"
    );
    assert_eq!(
        status(&mut conn, &waiting, QUEUE),
        None,
        "and the job that named it is gone"
    );

    assert_eq!(
        purge_dead(
            &mut conn,
            QUEUE,
            Some("nvs-stdlib-tests-purge:other"),
            None,
            "10"
        ),
        0,
        "the dead-letter table carries the tag its row was enqueued under, and not another"
    );
    assert_eq!(
        purge_dead(&mut conn, QUEUE, Some(TAG), None, "10"),
        1,
        "and a call that names `State::Dead` reaches the table § 6 moved the row into"
    );
    assert_eq!(
        status(&mut conn, &buried, QUEUE),
        None,
        "which leaves the receipt naming nothing, because that record is what was purged"
    );
}

/// §§ 2 and 4 against a real MySQL or MariaDB server: the schema applies, a job
/// pushed onto it is claimed once, and the claim answers the columns
/// `CLAIM_POSTGRES` answers.
///
/// **This is the first assertion in this file that is not PostgreSQL's**, and
/// what it is for is the half a unit agreement cannot reach: [`queue`]'s
/// rosters are held against each other by
/// `all_three_dialects_answer_a_claim_with_the_same_columns`, which proves the
/// *texts* name the same columns and cannot prove that any of them is one a
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
/// **`RETRY_MYSQL` binds `run_at`, `errors`, `id`, `claimed_at`** where its
/// PostgreSQL twin binds `id`, `claimed_at`, `run_at`, `errors`, a `?` being
/// bound by the position it occupies and the `set` clause standing left of the
/// `where`. That constant
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
                Some(queue::dead_errors(None, TAKEN, "IOError", "the endpoint is down").as_bytes(),),
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

    let errors = queue::dead_errors(
        None,
        TAKEN,
        "IOError",
        "the receipt service refused the order",
    );
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
/// `dedupe_pending = ?` resolves against the column the statements maintain, and
/// the uniqueness it feeds is `queue::schema`'s `nvs_jobs_dedupe` —
/// neither backend has a partial index in the vocabulary, so the pending-rows-only
/// scope is carried by the column holding null for
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

    // The other side. The claim is what carries the job out of `Pending`, and
    // `dedupe_pending` is a plain column, so clearing it is that statement's own
    // work rather than something the server derives from `state`. An ad-hoc
    // `update` here would assert the emulation against a row no statement of § 2's
    // maintains, which is why this half runs [`queue::CLAIM_MYSQL`] itself — the
    // pairing of a null column with MySQL's rule that a unique key does not
    // constrain nulls is what stands in for the partial index.
    let took = claim(&mut conn, QUEUE, DUE, 0);
    assert_eq!(
        took.iter()
            .map(|row| row[0].as_deref().expect("a claimed row names its id"))
            .collect::<Vec<_>>(),
        vec![id.as_str()],
        "the pending job is the one the claim named, and it is the key's only holder"
    );
    assert!(
        pending_for(&mut conn, KEY).is_none(),
        "a claimed job holds no key: the claim cleared the column as it left `Pending`"
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
/// disagree. [`queue::CLAIM_MYSQL`] is a [`queue::Split`]: `state = 1 and
/// claimed_at <= ?` is on the `select`'s second arm alone,
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
    let errors = queue::dead_errors(
        None,
        TAKEN,
        "IOError",
        "the receipt service refused the order",
    );
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
        Some(QUEUE.as_bytes()),
        Some(cutoff.as_slice()),
    ];
    first
        .begin(Some(nvs_db::Isolation::ReadCommitted), false)
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

/// ADR 0153 § 6 on the framed dialect: a receipt names a job across the move, so
/// `delete` finds it in the dead-letter table exactly as [`status`] already
/// does — `rule:concurrency/queue-deletion-is-explicit-and-bounded`.
///
/// **This is the arm a server decides and no unit agreement can.**
/// [`queue::DELETE_MYSQL`] is a multi-table delete driven from a one-row derived
/// table, and a job § 6 has already moved has no `nvs_jobs` row for the join to
/// attach at all. The obvious spelling — the state predicate in a trailing
/// `where` rather than on the `on` clause — drops the driving row for exactly
/// that job, because `j.state <> 1` over a missing `j` is null; the arm that
/// reads the other table then never runs, and the member becomes a receipt that
/// expires the moment a job exhausts its attempts. Which of the two texts is on
/// disk is a question only a server answers.
///
/// **Three states in one case, because each one is a different arm of the same
/// statement**: the pending job the jobs arm removes, the claimed one the join's
/// own predicate refuses, and the dead-lettered one the other arm reaches. A
/// text that had lost any one of those still answers plausibly for the other
/// two.
///
/// **The wrong queue is asked of the dead-lettered receipt** rather than of the
/// pending one, because that is where the pair is most easily half-bound: the
/// dead-letter arm carries its own copy of the queue condition, and a text
/// missing it removes another queue's record of lost work while every other
/// assertion here still passes.
///
/// **`1` and never `2` is the count § 6 promises.** The move is one moment, so
/// a receipt is in one of the two tables and never in both, and a statement that
/// affected two rows would be reporting a schema that had stopped being true.
#[test]
fn delete_finds_a_receipt_in_the_dead_letter_table_the_way_status_already_does() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-delete";
    const OTHER: &str = "nvs-stdlib-tests-framed-delete-elsewhere";
    const DUE: i64 = 9_000;
    const TAKEN: i64 = 9_500;
    const FAILED: i64 = 9_600;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    // Three jobs a millisecond apart, so which one each claim below takes is
    // decided by `CLAIM_MYSQL`'s `order by run_at, id limit 1` rather than by
    // the order this case happened to push them in.
    let buried = push(&mut conn, QUEUE, DUE, "1");
    let held = push(&mut conn, QUEUE, DUE + 1, "2");
    let waiting = push(&mut conn, QUEUE, DUE + 2, "2");

    let took = claim(&mut conn, QUEUE, TAKEN, 0);
    assert_eq!(took.len(), 1, "one due job is one claimed row");
    assert_eq!(
        took[0][ID].as_deref(),
        Some(buried.as_str()),
        "the oldest due job is the one claimed, and it is the one this case exhausts"
    );
    let lease = millis(TAKEN);
    let errors = queue::dead_errors(
        None,
        TAKEN,
        "IOError",
        "the receipt service refused the order",
    );
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
                Some(buried.as_bytes()),
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
            &[Some(buried.as_bytes()), Some(lease.as_slice())],
        ),
        1,
        "and the delete is keyed on the same lease, which is what makes the pair one move"
    );
    conn.commit()
        .expect("the server closed the transaction the pair was one moment inside");
    assert_eq!(
        status(&mut conn, &buried, QUEUE).as_deref(),
        Some("3"),
        "the receipt still names a job, which is the whole of what § 6 owes a caller holding one"
    );

    let took = claim(&mut conn, QUEUE, TAKEN, 0);
    assert_eq!(
        took.len(),
        1,
        "the job § 6 moved is not work a worker sees, so the next due one is"
    );
    assert_eq!(
        took[0][ID].as_deref(),
        Some(held.as_str()),
        "the second claim took the next due job, and a worker now holds it"
    );

    assert_eq!(
        delete(&mut conn, &waiting, QUEUE),
        1,
        "the pending job was removed out of the jobs table, which is the member's `true`"
    );
    assert_eq!(
        status(&mut conn, &waiting, QUEUE),
        None,
        "and it is in neither table, so the receipt has nothing left to name"
    );

    assert_eq!(
        delete(&mut conn, &held, QUEUE),
        0,
        "a claimed job is not removable, and the join's own predicate is what refuses it"
    );
    assert_eq!(
        status(&mut conn, &held, QUEUE).as_deref(),
        Some("1"),
        "the row is still there and still claimed, under the lease its worker is running on"
    );

    assert_eq!(
        delete(&mut conn, &buried, OTHER),
        0,
        "the dead-letter arm is keyed on the pair, so another queue's name reaches no record"
    );
    assert_eq!(
        delete(&mut conn, &buried, QUEUE),
        1,
        "and the receipt reaches the row in the other table, one row rather than two"
    );
    assert_eq!(
        status(&mut conn, &buried, QUEUE),
        None,
        "which is the arm a `where` on the driving row would have dropped without saying so"
    );
}

/// [`queue::PURGE_MYSQL`] and [`queue::PURGE_DEAD_MYSQL`] against a real MySQL or
/// MariaDB server: the same bound, the same default set, and the same two tables
/// as the PostgreSQL twin —
/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`.
///
/// **The bound is where the two dialects genuinely differ**, and it is what a
/// server has to decide rather than a unit agreement: PostgreSQL has no
/// `delete … limit` and bounds a subquery instead, while this dialect takes
/// `order by … limit` on the delete itself. Neither text can be checked against
/// the other, because they are not the same construct.
///
/// **Eight placeholders for five values, sent once.** A `?` is a position that
/// cannot be named twice, so every null-checked option here is bound as many
/// times as it is mentioned; a text and a caller that disagree about how many
/// there are is the failure this leg exists to catch, and it is invisible to
/// anything that does not prepare the statement.
///
/// **The tag is asked for as a group nothing here is in.** No push on this leg
/// names one, so a call naming a tag selects nothing — which is the answer that
/// says the arm binds and narrows rather than being ignored.
///
/// **Everything the PostgreSQL twin asks about `before` and the opt-in states is
/// asked there and not repeated here.** What differs between the two texts is
/// the bound and the placeholders; the rest is one selection written twice, and
/// a case that transcribed the whole of it would cost every framed leg the same
/// six rows to assert nothing new.
#[test]
fn a_framed_purge_is_bounded_and_reaches_the_dead_letter_table_only_when_asked() {
    const QUEUE: &str = "nvs-stdlib-tests-framed-purge";
    const TAG: &str = "nvs-stdlib-tests-framed-purge:batch";
    const DUE: i64 = 12_000;
    const LATER: i64 = 13_000;
    const LEASE: i64 = 12_500;
    const FAILED: i64 = 12_600;

    let Some(server) = framed() else {
        return;
    };
    schema(&server);
    let mut conn = open(&server);
    clear(&mut conn, QUEUE);

    let done = push(&mut conn, QUEUE, DUE, "2");
    let gone = push(&mut conn, QUEUE, DUE + 1, "2");
    let buried = push(&mut conn, QUEUE, DUE + 2, "2");
    let held = push(&mut conn, QUEUE, DUE + 3, "2");
    let waiting = push(&mut conn, QUEUE, LATER, "2");

    set_state(&mut conn, &done, "2", None);
    set_state(&mut conn, &gone, "4", None);
    set_state(&mut conn, &held, "1", None);
    set_state(&mut conn, &buried, "1", Some(LEASE));
    let errors = queue::dead_errors(None, LEASE, "IOError", "the receipt service refused");
    let lease = millis(LEASE);
    let split = queue::DEAD_LETTER_MYSQL;
    conn.begin(None, false)
        .expect("the server opened the transaction");
    apply(
        &mut conn,
        split.first,
        &[
            Some(millis(FAILED).as_slice()),
            Some(errors.as_bytes()),
            Some(buried.as_bytes()),
            Some(lease.as_slice()),
        ],
    );
    apply(
        &mut conn,
        split.then,
        &[Some(buried.as_bytes()), Some(lease.as_slice())],
    );
    conn.commit()
        .expect("the server closed the transaction the pair was one moment inside");

    assert_eq!(
        purge(&mut conn, QUEUE, None, Some(TAG), None, "10"),
        0,
        "nothing on this leg was pushed under a tag, so the group the call named is empty"
    );
    assert_eq!(
        purge(&mut conn, QUEUE, None, None, None, "1"),
        1,
        "two rows had finished and the bound this dialect spells took one of them"
    );
    assert_eq!(
        status(&mut conn, &done, QUEUE),
        None,
        "and it took the oldest, which is the `order by` in front of that bound"
    );
    assert_eq!(
        purge(&mut conn, QUEUE, None, None, None, "10"),
        1,
        "the other finished row goes to the next call, as a caller's loop would make it"
    );
    assert_eq!(
        (
            status(&mut conn, &waiting, QUEUE).as_deref(),
            status(&mut conn, &held, QUEUE).as_deref(),
            status(&mut conn, &buried, QUEUE).as_deref(),
        ),
        (Some("0"), Some("1"), Some("3")),
        "and left the pending row, the claimed one and the dead-lettered one where they were"
    );

    assert_eq!(
        purge_dead(&mut conn, QUEUE, None, Some(LATER), "10"),
        1,
        "the dead-letter selection is its own statement, and it reads the age off the same column"
    );
    assert_eq!(
        status(&mut conn, &buried, QUEUE),
        None,
        "which is the one table `State::Dead` selects, and the receipt now names nothing"
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
/// no rows while `count` ignores the `null` its `case` falls through to. Five
/// non-null zeros over an empty table is that difference, and it is also why
/// this helper hands back `Option`s.
///
/// **The counters are asserted as one row rather than one at a time**, so
/// a pair swapped in the select list fails here: `pending` and `claimed` are
/// two `case when`s differing only in an ordinal, and each is a plausible
/// number for the other to answer.
///
/// **The third read is the dead-letter move, and it moves three counters at
/// once.** § 6 takes the row out of `nvs_jobs`, so the depth subquery gains one,
/// the `attempts` sum — which reads [`queue::JOBS_TABLE`] alone — loses the
/// attempt that job had used, and the subquery that sums the other table's
/// `attempts` gains exactly that attempt. A sum written over both tables would
/// be `1` throughout and would say nothing about where the work went.
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

    let five = |counts: [&str; 5]| counts.map(|c| Some(c.to_owned())).to_vec();

    assert_eq!(
        stats(&mut conn, QUEUE),
        five(["0", "0", "0", "0", "0"]),
        "an empty queue answers five zeros and no nulls, which is the whole reason the counters \
         count rather than sum"
    );

    let dying = push(&mut conn, QUEUE, DUE, "1");
    let took = claim(&mut conn, QUEUE, TAKEN, 0);
    assert_eq!(took.len(), 1, "one due job is one claimed row");
    let waiting = push(&mut conn, QUEUE, DUE, "2");
    let _ = push(&mut conn, OTHER, DUE, "2");

    assert_eq!(
        stats(&mut conn, QUEUE),
        five(["1", "1", "1", "0", "0"]),
        "one job in each state, the attempt the claim wrote is the live sum's whole content, and \
         nothing has been buried for the other sum to reach"
    );

    // § 6's move, run as `a_framed_status_walks_a_job_through_both_of_the_tables_it_can_be_in`
    // runs it: what this case adds is the counters that have to follow the row.
    let errors = queue::dead_errors(
        None,
        TAKEN,
        "IOError",
        "the receipt service refused the order",
    );
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
        five(["1", "0", "0", "1", "1"]),
        "the exhausted job is depth rather than work, and its attempt left `nvs_jobs` with it for \
         the counter that reads the table it landed in"
    );
    assert_eq!(
        status(&mut conn, &waiting, QUEUE).as_deref(),
        Some("0"),
        "the job the first counter is still reporting is the one that never left"
    );

    assert_eq!(
        stats(&mut conn, OTHER),
        five(["1", "0", "0", "0", "0"]),
        "and the other queue sees its own pending job and none of this one's depth, which is every \
         `?` bound to the name that was asked about"
    );
}

/// § 3 on SQL Server: the job and the write that caused it are durable together.
///
/// [`an_enqueue_commits_with_the_write_that_made_it`]'s claim on this backend,
/// and that case's doc owns why § 3 is a property of the pair rather than of
/// either half. What this one adds is the *mechanism* under the id, which is
/// neither of the other two's: [`queue::INSERT_SQLSERVER`]'s `output
/// inserted.id` hands the row back inside the insert's own round trip, where
/// PostgreSQL's `returning` is a clause and the framed drivers' id is a field on
/// an OK packet. So the id asserted below is the one the insert itself reported
/// from inside the transaction, and the row carrying it afterwards is the commit
/// having covered both writes.
///
/// **The enqueue is a pair of statements on this dialect and is still one
/// statement here**, because this push names no dedupe key:
/// [`queue::INSERT_SQLSERVER::first`](queue::INSERT_SQLSERVER) is the dedupe
/// read `Core\Queue::push` skips for exactly that case, which is [`push`]'s own
/// doc. The transaction this case opens is therefore the *application's*, which
/// is the only one § 3 is about.
#[test]
fn an_enqueue_commits_with_the_write_on_sql_server() {
    const QUEUE: &str = "nvs-stdlib-tests-mssql-commit";

    let Some(server) = sqlserver() else {
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
        "insert into nvs_stdlib_tests_orders (queue) output inserted.id values (@p1)",
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
        "select id, state, attempts from nvs_jobs where queue = @p1",
        &[Some(QUEUE.as_bytes())],
    );
    assert_eq!(
        job[0][ID].as_deref(),
        Some(id.as_str()),
        "the durable job is the row `INSERT_SQLSERVER`'s `output` clause answered with inside the \
         transaction"
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
            "select count(*) from nvs_stdlib_tests_orders where id = @p1",
            &[Some(order.as_bytes())],
        ),
        "1",
        "the order the job was pushed for is the one that committed"
    );
}
