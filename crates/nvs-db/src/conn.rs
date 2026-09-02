//! The five drivers as an enum, and the busy state each one carries.
//!
//! [ADR 0132 § 5](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
//! decides that this is an `enum` and not a `Driver` trait, for three reasons
//! in this project's priority order: the set is **closed** (ADR 0067 § 12 makes
//! a new backend an ADR rather than a plugin, and wasm extensions cannot host
//! one anyway, so open-set extensibility is the one property a trait buys and
//! this design does not want it); a trait wide enough for all five would be
//! half `unimplemented!()`, since `executeMany` is one prepare and N executions
//! on four drivers and a bulk protocol message on the fifth, a prepare is a
//! round trip on two and free on one, and a reset is four different command
//! sequences and a rollback on the fifth; and a `Box<dyn Driver>` is an
//! allocation and an indirect call per connection operation where a `match` is
//! something the compiler can see through.
//!
//! So where a signature repeats five times, **it repeats**. Five `fn query`
//! bodies that each say what their own protocol does are cheaper to read and to
//! fix than one that makes four of them lie, and what keeps them honest is not
//! a type — it is `tools/db-matrix.py` running one assertion set against five
//! real servers.
//!
//! This module holds the five shapes and the one rule they share. A driver's
//! own wire code — its handshake, its sequencing, its error table — is in that
//! driver's module beside this one ([`crate::pg`] is the first), so the enum
//! stays readable as an enum while a protocol grows to the size a protocol is.
//!
//! What is *not* per driver is § 4's rule about when a connection may be
//! written to and when it may be pooled: that is one rule over [`State`], and
//! it lives on that type rather than being restated in five places. What each
//! driver decides for itself is which state a given wire event leaves it in —
//! whether an abandoned result set can be cancelled and drained back to
//! [`State::Idle`], or is [`State::Poisoned`].

use std::cell::Cell;

use mysql_common::constants::CapabilityFlags;

use crate::mysql::Wire as MyWire;
use crate::pg::{CancelKey, Wire};
use crate::sql::StatementCache;

/// The five backends [ADR 0067 § 12](../../../docs/adr/0067-core-db.md) closes
/// the set at.
///
/// MariaDB is its own driver and not a MySQL flag: the two have diverged in
/// auth plugins, error tables and bulk protocol, and ADR 0067 argues that at
/// length. The spellings here are also the ones `NVS_DB_MATRIX_DRIVER`,
/// `tools/db-matrix.py --driver` and a `[db.<name>]` block's own `driver` field
/// use, so that roster has one home — see [`Driver::matrix_name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Driver {
    /// PostgreSQL, over `postgres-protocol` and the extended-query protocol.
    Postgres,
    /// MySQL, over `mysql_common` and `COM_STMT_*`.
    MySql,
    /// MariaDB, over `mysql_common` with its own auth plugins and error table.
    MariaDb,
    /// Microsoft SQL Server, over a TDS 7.4 implementation written here.
    SqlServer,
    /// SQLite, over `rusqlite`, with no wire and no socket at all.
    Sqlite,
}

impl Driver {
    /// Every driver, in the order ADR 0067's own tables use.
    ///
    /// The matrix harness iterates this rather than a list of its own, so a
    /// sixth backend cannot be added to the language without appearing in the
    /// verification matrix on the same commit.
    pub const ALL: [Driver; 5] = [
        Driver::Postgres,
        Driver::MySql,
        Driver::MariaDb,
        Driver::SqlServer,
        Driver::Sqlite,
    ];

    /// The name this driver has in `NVS_DB_MATRIX_DRIVER` and in
    /// `tools/db-matrix.py --driver`.
    ///
    /// These are wire-adjacent identifiers rather than display names, so they
    /// are lower case and stable; nothing renders them to a user.
    #[must_use]
    pub fn matrix_name(self) -> &'static str {
        match self {
            Driver::Postgres => "postgres",
            Driver::MySql => "mysql",
            Driver::MariaDb => "mariadb",
            Driver::SqlServer => "mssql",
            Driver::Sqlite => "sqlite",
        }
    }

    /// The driver a `NVS_DB_MATRIX_DRIVER` value names, or `None` for a value
    /// no driver answers to.
    ///
    /// Deliberately not `FromStr`: this parses one harness-supplied identifier,
    /// exactly, and a configuration file's own `driver` field goes through
    /// [`Driver::from_config_name`] instead — same roster, one spelling rule
    /// looser.
    #[must_use]
    pub fn from_matrix_name(name: &str) -> Option<Driver> {
        Driver::ALL.into_iter().find(|d| d.matrix_name() == name)
    }

    /// The driver a `[db.<name>]` block's `driver` field names, or `None` for a
    /// value no driver answers to.
    ///
    /// **The same roster as [`Driver::matrix_name`], deliberately**: an
    /// operator's `driver = "postgres"` and the harness's
    /// `NVS_DB_MATRIX_DRIVER=postgres` name one backend, and a second roster is
    /// how one of them gains a spelling the other refuses — a deployment that
    /// boots and a matrix that skips it.
    ///
    /// The comparison is ASCII-case-insensitive where
    /// [`from_matrix_name`](Driver::from_matrix_name)'s is exact, and that is
    /// the whole difference: this value is typed by hand into a file, where
    /// `Postgres` is a capital letter rather than a different backend, and that
    /// one is set by a script that can spell it the one way.
    #[must_use]
    pub fn from_config_name(written: &str) -> Option<Driver> {
        Driver::ALL
            .into_iter()
            .find(|d| written.eq_ignore_ascii_case(d.matrix_name()))
    }
}

/// Where a connection's wire is, between commands —
/// [ADR 0132 § 4](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md).
///
/// This is not on the stream and cannot be: `NvsStream`'s readiness
/// registration is one task's and is invisible above `Read`/`Write`, a
/// connection is busy whether or not its socket is readable, and SQLite must
/// answer the same `LogicError` with no stream underneath it at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    /// The wire is at a message boundary; a new command may be written.
    Idle,
    /// A buffered statement is in flight, and only its own call may write.
    Executing,
    /// Rows remain unread. A second statement here is ADR 0067 § 4's
    /// `LogicError`.
    Streaming,
    /// The wire is *not* at a known message boundary — a deadline fired
    /// mid-message, a decode failed, or a stream was abandoned in a shape the
    /// driver cannot drain.
    Poisoned,
}

impl State {
    /// Whether a new statement may be written on a connection in this state.
    ///
    /// Only [`State::Idle`] permits one. The driver builds the refusal itself,
    /// in one place per driver and naming both of ADR 0067 § 4's fixes —
    /// `pg.rs`'s `second_statement` owns that wording. `nvs-stdlib` re-words it
    /// as § 4's `LogicError`, because the fault class is its own and so is the
    /// call site's spelling, which only the standard library knows.
    #[must_use]
    pub fn may_start_statement(self) -> bool {
        matches!(self, State::Idle)
    }

    /// Whether a connection in this state may be reset and returned to the
    /// per-core pool.
    ///
    /// This is the security-relevant one. A [`State::Poisoned`] connection is
    /// **closed, never reset**: ADR 0067 § 13 makes the reset a boundary
    /// because a connection carrying one request's state into another's is a
    /// cross-tenant leak, and a `RESET ALL` written into the middle of an
    /// unfinished message is not a reset — it is a fragment of one request's
    /// protocol stream that the next request will read as its own. Draining
    /// first would mean trusting a length prefix that has already proven
    /// untrustworthy. Closing costs one handshake and is the only answer that
    /// is provable.
    ///
    /// A connection still `Executing` or `Streaming` is not poolable either,
    /// but for the ordinary reason: it is in the middle of a call. Its own
    /// driver returns it to [`State::Idle`] first, or poisons it.
    #[must_use]
    pub fn is_poolable(self) -> bool {
        matches!(self, State::Idle)
    }
}

/// The isolation levels [ADR 0067 § 7](../../../docs/adr/0067-core-db.md)'s
/// `transaction()` takes, as a caller asks for them rather than as any one
/// server spells them.
///
/// The set is the SQL standard's four plus [`Isolation::Snapshot`], which SQL
/// Server has as a level of its own and the others reach under another name —
/// so this is one enum with a per-driver rendering rather than five overlapping
/// ones, for the same reason [`State`] is one rule rather than five.
///
/// § 7 requires a driver that **lacks** a level to throw rather than quietly
/// run the closure at a weaker one. A driver that renders a level as a
/// *stronger* guarantee than was asked for is not that case and does not
/// throw: nothing a program can observe is weakened by it. Where each driver
/// draws that line is in the driver — `pg.rs`'s `begin_command` is
/// PostgreSQL's, and it is the only place that reasoning is spent for
/// PostgreSQL.
///
/// The variants are in § 7's own order. Nothing derives `Ord` from it, because
/// `Snapshot` and the two levels either side of it are not one chain on every
/// backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Isolation {
    /// A statement may read rows another transaction has written and not
    /// committed, on a backend that implements the level at all.
    ReadUncommitted,
    /// A statement sees the rows committed before that statement began.
    ReadCommitted,
    /// Every statement in the transaction sees one snapshot of committed rows.
    RepeatableRead,
    /// The transaction reads from one snapshot taken when it began, and writes
    /// conflict rather than block — SQL Server's own level, and what the
    /// row-versioning backends call `REPEATABLE READ`.
    Snapshot,
    /// Concurrent transactions produce a result some serial order of them would
    /// have produced.
    Serializable,
}

/// What a result column was *declared* as — spec § 18's `ColumnType`, whose
/// fourteen cases `Core\Db\Column::type` answers with.
///
/// **This is not a summary of what a read of the column produces**, and the two
/// questions are deliberately different. [ADR 0067
/// § 9](../../../docs/adr/0067-core-db.md)'s type map decodes a `JSON` column to
/// a `tainted string` exactly as it decodes a `TEXT` one — it has to, since
/// MariaDB's `JSON` is `LONGTEXT` with a check constraint and is not detectable
/// at all — while a *description* of the column can tell them apart wherever the
/// backend has a type of its own, so [`ColumnType::Json`] is a case here. The
/// same split is why there is no array case: § 9 reads a PostgreSQL array as
/// `array<T>` and MySQL's `SET` as `array<string>`, and both describe as
/// [`ColumnType::Other`].
///
/// [`ColumnType::Other`] is the total case rather than a failure. Every column
/// type § 9's table has no row for — `inet`, a range, `hstore`, geometry,
/// `interval` — reads as `tainted string` and describes as `Other`, so
/// `columns()` answers for every column a server can send rather than only for
/// the ones this enum enumerates.
///
/// The variants are the spec's own order, and nothing derives `Ord` from it
/// because they are a set and not a scale. Each driver classifies its own type
/// codes; PostgreSQL's is [`crate::PgColumn::column_type`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnType {
    /// `SMALLINT`/`INTEGER`/`BIGINT`, read as `int`.
    Int,
    /// A column § 9 reads as `uint`: an `UNSIGNED` integer on MySQL and
    /// MariaDB, `oid` on PostgreSQL.
    Uint,
    /// `FLOAT`/`REAL`/`DOUBLE`, read as `float`.
    Float,
    /// `DECIMAL`/`NUMERIC`/`MONEY`, read as `decimal`.
    Decimal,
    /// A text-family column — `CHAR`/`VARCHAR`/`TEXT`/`ENUM` — read as
    /// `tainted string`.
    Text,
    /// `BINARY`/`BLOB`/`BYTEA`, read as `tainted bytes`.
    Bytes,
    /// `BOOLEAN`, and `BIT(1)`.
    Bool,
    /// `DATE`, read as a `Core\Time\Date`.
    Date,
    /// `TIME`, read as a `Core\Time\TimeOfDay`.
    Time,
    /// A zone-less `DATETIME`/`TIMESTAMP`, read as a `Core\Time\DateTime` in
    /// the zone the connection declared.
    DateTime,
    /// `TIMESTAMPTZ`, read as a `Core\Time\Instant` because it carries its own
    /// offset.
    Instant,
    /// `UUID`/`uniqueidentifier`, read as a `Core\Uuid`. MySQL's `BINARY(16)`
    /// is [`ColumnType::Bytes`], as § 9 says.
    Uuid,
    /// A column the backend types as JSON. The value still reads as a `tainted
    /// string` — § 9's rule that JSON is never auto-decoded is untouched — and
    /// on a backend where JSON is an aliased text type the column is
    /// [`ColumnType::Text`], because that is what it is.
    Json,
    /// Every other column: one with no Novis type of its own, and every array.
    Other,
}

/// [ADR 0067 § 8](../../../docs/adr/0067-core-db.md)'s normalised `ErrorKind`,
/// under a name that cannot be misread as [`std::io::ErrorKind`] in a driver
/// that spells both in one function.
///
/// § 8's whole point is that an application branches on the *condition* rather
/// than on a vendor string: PDO exposes only a `SQLSTATE` and a vendor integer,
/// which is why real PHP matches on `"Duplicate entry"` or hard-codes `1062`.
/// Each driver maps its own codes onto this set and MariaDB needs its own table
/// rather than MySQL's; PostgreSQL's is `pg.rs`'s `kind_of`. The raw code stays
/// on [`ServerError`] for the conditions normalising does not reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DbErrorKind {
    /// A row with this key already exists.
    UniqueViolation,
    /// A referenced row does not exist, or a referencing one still does.
    ForeignKeyViolation,
    /// A column that may not be null was written null.
    NotNullViolation,
    /// A `CHECK` constraint refused the row.
    CheckViolation,
    /// Two transactions each hold what the other is waiting for, and the server
    /// aborted this one to break it.
    Deadlock,
    /// The transaction could not be serialised against a concurrent one and was
    /// aborted — the ordinary outcome under `REPEATABLE READ` or stronger.
    SerializationFailure,
    /// The connection is gone, or the server is going away.
    ConnectionLost,
    /// A statement or an idle transaction ran past a bound and was cancelled.
    Timeout,
    /// The statement is not something the server will run: a syntax error, an
    /// undefined table, a type it cannot resolve.
    Syntax,
    /// The role may not do this.
    Permission,
    /// Anything the driver's own table does not name, including a condition one
    /// backend has and the others do not.
    Other,
}

impl DbErrorKind {
    /// Whether [ADR 0067 § 7](../../../docs/adr/0067-core-db.md)'s
    /// `{retries: n}` re-runs the closure over this.
    ///
    /// **These two, and nothing else.** A retry is sound only where the server
    /// aborted the transaction *because* of a conflict it expects to be gone on
    /// the next attempt; re-running a closure over a lock timeout or a lost
    /// connection would be running a side-effecting function again on a guess,
    /// which is also why § 7's default is 0 retries. The backoff and the re-run
    /// are `nvs-stdlib`'s — this is the part of the rule only a driver can
    /// answer, because only a driver knows what its server's codes mean.
    #[must_use]
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            DbErrorKind::Deadlock | DbErrorKind::SerializationFailure
        )
    }
}

/// A refusal the server worded, with § 8's normalised kind beside it.
///
/// Carried **inside** the `io::Error` every driver entry point already answers
/// with, so a caller that only prints the sentence is unchanged and a caller
/// that must branch — § 7's retry rule is the first of them — asks
/// [`ServerError::of`] rather than matching on the text.
///
/// § 8's `driverCode` has no field here and PostgreSQL will always answer
/// `None` for it: the `SQLSTATE` *is* this server's code, and a second integer
/// invented to fill a shape would be a value with no meaning. MySQL has a real
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError {
    /// § 8's normalised kind, from the driver's own code table.
    pub kind: DbErrorKind,
    /// The five-character `SQLSTATE`, as the server sent it.
    pub sql_state: String,
    /// The server's non-localized severity — `ERROR`, `FATAL`, `PANIC`.
    pub severity: String,
    /// The server's own sentence.
    pub message: String,
    /// The constraint the condition names, where it names one.
    pub constraint: Option<String>,
    /// What the rendered sentence calls this backend.
    ///
    /// [`Driver::matrix_name`] is deliberately not this: those are harness keys
    /// and nothing renders them to a reader, so tying an operator-facing
    /// message to them would freeze one against the other.
    pub backend: &'static str,
}

impl ServerError {
    /// The server's refusal inside an `io::Error`, where that is what it is.
    ///
    /// A wire failure, a decode refusal and § 4's busy-connection error are all
    /// the same `io::Error` type and none of them is one of these — which is
    /// why this answers `None` rather than a kind of [`DbErrorKind::Other`].
    #[must_use]
    pub fn of(error: &std::io::Error) -> Option<&ServerError> {
        error.get_ref()?.downcast_ref::<ServerError>()
    }
}

impl std::fmt::Display for ServerError {
    /// Severity, message and `SQLSTATE`: the three fields an operator acts on.
    ///
    /// Bound parameters are not among them and never will be — ADR 0067 § 8
    /// makes a `Throwable` message a `secret` sink, and this sentence is what
    /// reaches one.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}: {} (SQLSTATE {})",
            self.backend, self.severity, self.message, self.sql_state
        )
    }
}

impl std::error::Error for ServerError {}

/// A PostgreSQL connection: `postgres-protocol`'s codec plus the extended-query
/// state machine, the SASL handshake and ADR 0067 § 13's reset, written here.
///
/// The reset is deliberately **not** `DISCARD ALL`, which would deallocate the
/// prepared statements the statement cache exists to preserve.
#[derive(Debug)]
pub struct PgConn {
    /// The stream and the bytes read off it that are not yet a whole message,
    /// once [`crate::pg::PgConn::connect`] has upgraded it.
    ///
    /// ADR 0132 § 3's in-band upgrade means the `SSLRequest` and its one-byte
    /// answer are the only plaintext this connection ever carries, so there is
    /// no variant here for "not yet encrypted": a connection that did not
    /// upgrade was never built.
    pub(crate) wire: Wire,
    /// ADR 0132 § 4's busy state. A plain [`Cell`]: no atomic and no lock,
    /// because a task never migrates and a connection is owned by one request
    /// at a time, which `nvs-host`'s `!Send` scheduler makes true rather than
    /// hoped.
    pub(crate) state: Cell<State>,
    /// What the backend handed over at startup so a *second* connection can
    /// cancel this one's query — the only way PostgreSQL offers, and it is
    /// unrepeatable: the key arrives once, during the handshake, and a
    /// connection that dropped it cannot ask again.
    pub(crate) cancel: CancelKey,
    /// ADR 0067 § 1's LRU of server-side prepared statements, keyed by SQL text
    /// plus expansion arity.
    ///
    /// It is on the connection because a prepared statement is a name on one
    /// session: two connections sharing this would bind against names the
    /// other's server has never heard of. It survives this driver's reset,
    /// which is § 13's whole reason for not sending `DISCARD ALL`.
    pub(crate) cache: StatementCache,
    /// ADR 0067 § 9's declared zone, as seconds east of UTC — what a zone-less
    /// `TIMESTAMP` column off this connection is read in.
    ///
    /// Held rather than re-derived because the decode of that row happens in
    /// `nvs-stdlib`, the only crate that can build the `Core\Time\DateTime` it
    /// becomes, and [`PgTarget`](crate::pg::PgTarget) does not outlive the
    /// handshake. Four bytes a connection, against a config lookup a column.
    pub(crate) time_zone: i32,
    /// How many of ADR 0067 § 7's transactions are open on this connection: 0
    /// for none, 1 for the outermost `BEGIN`, and one more per nested
    /// `transaction()` — each of which is a `SAVEPOINT` named by the depth it
    /// opened at.
    ///
    /// It is the *connection's* and not the caller's because § 7 gives no
    /// explicit savepoint API and no `inTransaction()`: a library that wraps
    /// its own writes must stay callable from inside a caller's transaction
    /// without being able to ask whether it is in one. `pg.rs`'s `begin` is
    /// where the depth decides which command goes out.
    pub(crate) depth: Cell<u32>,
}

/// A MySQL connection: `mysql_common`'s codec plus the handshake, `COM_STMT_*`
/// sequencing and the `LOCAL INFILE` refusal, written here.
///
/// Its reset is `COM_RESET_CONNECTION`, which is atomic and complete and also
/// drops prepared statements — a real asymmetry with PostgreSQL that is the
/// protocol's and not a choice.
#[derive(Debug)]
pub struct MySqlConn {
    /// The stream and the bytes read off it that are not yet a whole packet,
    /// once [`crate::mysql::MySqlConn::connect`] has upgraded it.
    ///
    /// It carries the sequence counter too, which PostgreSQL's has no analogue
    /// of: MySQL numbers the packets of one command and a mismatch is a wire
    /// nothing can find a boundary in.
    pub(crate) wire: MyWire,
    /// ADR 0132 § 4's busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
    /// What the two ends agreed this connection can do — the client's set
    /// intersected with the server's greeting.
    ///
    /// Held because MySQL's packets are not self-describing: whether an `OK`
    /// carries session-state changes, and whether a result set is terminated by
    /// an `EOF` packet or by an `OK`, are read off these bits. A driver that
    /// re-derived them per packet would be deciding it twice.
    pub(crate) capabilities: CapabilityFlags,
    /// ADR 0067 § 9's declared zone, as seconds east of UTC — what a zone-less
    /// `DATETIME` or `TIMESTAMP` off this connection is read in.
    ///
    /// Held for [`PgConn::time_zone`]'s reason: the decode of that row happens
    /// in `nvs-stdlib`, and the target does not outlive the handshake.
    pub(crate) time_zone: i32,
}

/// A MariaDB connection: `mysql_common`'s codec, its own auth plugins and its
/// own error table, plus `COM_STMT_BULK_EXECUTE` and `RETURNING`.
///
/// Its own driver rather than a MySQL flag — ADR 0067 argues that at length and
/// treating it as a flag is a design error, not a simplification.
#[derive(Debug)]
pub struct MariaConn {
    /// ADR 0132 § 4's busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
}

/// A SQL Server connection over TDS 7.4, all of which is written here: there is
/// no sans-IO crate for the protocol, and the only implementation is async over
/// `futures-io` with its TLS behind `tokio-rustls`.
///
/// This is the largest single piece of new wire code the database goal buys,
/// and § 3's handshake — TLS tunnelled inside TDS packets before the connection
/// becomes an ordinary TLS stream — is why `NvsTls` is generic over its
/// transport.
#[derive(Debug)]
pub struct TdsConn {
    /// ADR 0132 § 4's busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
}

/// A SQLite connection: `rusqlite`, a file handle, and no bytes on any wire.
///
/// It has no socket to park on, so its calls go to `nvs-host`'s blocking pool
/// rather than to a readiness registration. Rolling back an open transaction is
/// the whole of its reset, because a file handle has no session state to leak.
#[derive(Debug)]
pub struct SqliteConn {
    /// ADR 0132 § 4's busy state; the reasoning is on [`PgConn`]. SQLite carries it for
    /// the same reason the others do even with no wire to be mid-message on:
    /// ADR 0067 § 4's `LogicError` is a property of the API, not of a socket.
    pub(crate) state: Cell<State>,
}

/// One open connection to one database, whichever backend it is.
///
/// [ADR 0132 § 5](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md):
/// each variant owns its own state machine, its own error-code table and its
/// own reset, and `Core\Db`'s entry points `match` here exactly once.
///
/// `clippy::large_enum_variant` is allowed here and the reasoning is ADR 0132
/// § 5's third argument, unchanged: a `Box` around a variant is an allocation
/// and an indirection on every message this enum's own hot path reads, and the
/// lint is measuring a transitional shape rather than a real disparity —
/// [`PgConn`] carries a TLS session because its driver landed first, and the
/// other four are one byte only until theirs do. Nothing holds these in an
/// array either: a `Connection` is one live object per pooled connection, which
/// at ADR 0067 § 13's ceiling of 16 a core is kilobytes.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Connection {
    /// See [`PgConn`].
    Postgres(PgConn),
    /// See [`MySqlConn`].
    MySql(MySqlConn),
    /// See [`MariaConn`].
    MariaDb(MariaConn),
    /// See [`TdsConn`].
    SqlServer(TdsConn),
    /// See [`SqliteConn`].
    Sqlite(SqliteConn),
}

impl Connection {
    /// Which backend this is.
    #[must_use]
    pub fn driver(&self) -> Driver {
        match self {
            Connection::Postgres(_) => Driver::Postgres,
            Connection::MySql(_) => Driver::MySql,
            Connection::MariaDb(_) => Driver::MariaDb,
            Connection::SqlServer(_) => Driver::SqlServer,
            Connection::Sqlite(_) => Driver::Sqlite,
        }
    }

    /// Where this connection's wire is — ADR 0132 § 4.
    #[must_use]
    pub fn state(&self) -> State {
        match self {
            Connection::Postgres(c) => c.state.get(),
            Connection::MySql(c) => c.state.get(),
            Connection::MariaDb(c) => c.state.get(),
            Connection::SqlServer(c) => c.state.get(),
            Connection::Sqlite(c) => c.state.get(),
        }
    }

    /// Move this connection to `state`.
    ///
    /// Takes `&self` rather than `&mut self` because the state is a [`Cell`]:
    /// a driver sets it from inside a borrow of the connection it is reading
    /// rows out of, which is the whole reason § 4 puts it in a cell.
    pub fn set_state(&self, state: State) {
        match self {
            Connection::Postgres(c) => c.state.set(state),
            Connection::MySql(c) => c.state.set(state),
            Connection::MariaDb(c) => c.state.set(state),
            Connection::SqlServer(c) => c.state.set(state),
            Connection::Sqlite(c) => c.state.set(state),
        }
    }

    /// Mark the wire as no longer at a known message boundary.
    ///
    /// Once poisoned a connection is closed rather than reset, and never
    /// rejoins the pool — [`State::is_poolable`] owns why. This is one-way on
    /// purpose: there is no state a driver could inspect to prove a poisoned
    /// wire has become trustworthy again.
    pub fn poison(&self) {
        self.set_state(State::Poisoned);
    }

    /// Whether a new statement may be written now — ADR 0067 § 4.
    #[must_use]
    pub fn may_start_statement(&self) -> bool {
        self.state().may_start_statement()
    }

    /// Whether this connection may be reset and returned to the per-core pool.
    #[must_use]
    pub fn is_poolable(&self) -> bool {
        self.state().is_poolable()
    }
}

/// A connection is what a request holds open, and `nvs_runtime` is where a
/// request's own state lives — the trait's own doc comment owns why the seam
/// exists rather than a field typed for this enum.
impl nvs_runtime::HeldConnection for Connection {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    /// ADR 0067 § 13's release gate is [`Connection::is_poolable`], which is
    /// this connection's own wire state and nothing else — the trait method
    /// exists because `nvs-runtime` learns when a request ends and cannot name
    /// this type to ask.
    fn is_poolable(&self) -> bool {
        Connection::is_poolable(self)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::Arc;
    use std::time::Instant;

    use nvs_config::db::PoolBounds;
    use nvs_config::snapshot::Snapshot;
    use nvs_runtime::pool::{Lease, Ticket, admit, release, take};

    use super::{Connection, Driver, SqliteConn, State};

    /// The slot a request holds while one connection under `key` is open.
    ///
    /// `key` is ADR 0067 § 2's, whichever of its two spellings computed it —
    /// [`Ticket::for_block`] is the only constructor of a pool key, and takes
    /// the string rather than deciding it.
    fn lease(generation: &Arc<Snapshot>, key: &str) -> Lease {
        admit(Ticket::for_block(
            generation,
            key,
            PoolBounds {
                idle: 2,
                ..PoolBounds::DEFAULT
            },
        ))
        .expect("the default `max` admits a case's one connection")
    }

    /// A connection at a message boundary, which is the state § 13's release
    /// gate lets into the pool.
    ///
    /// SQLite's variant because it is the one this crate can *always* build
    /// without a server: ADR 0132 § 3 gives it no bytes on any wire, so it is
    /// the one connection type that will never grow a stream a unit test cannot
    /// open. The pool reads the variant no more than it reads the wire — it
    /// asks `is_poolable` and stores the box — so which variant this is says
    /// nothing about the property below.
    fn idle_connection() -> Box<Connection> {
        Box::new(Connection::Sqlite(SqliteConn {
            state: Cell::new(State::Idle),
        }))
    }

    /// ADR 0067 § 13's first two bullets: the pool is **per core**, and its key
    /// is § 2's — the block *name* for `connect`, a hash of every settings
    /// field for `open` — so two config blocks are two pools and two database
    /// users never share a connection.
    ///
    /// One case answers for both of § 2's keys because both reach the pool
    /// through one door: `Ticket::for_block` is the only constructor of a key,
    /// and the two spellings differ in the *string* they compute rather than in
    /// where they file. `Core\Db::open` waits on a shape-parameter type; when
    /// it lands, its settings hash is the `key` argument here, and it inherits
    /// this property by having nowhere else to put a connection.
    ///
    /// What this crate adds over `nvs_runtime::pool`'s own cases — which assert
    /// the same key arithmetic over a fake connection — is the type actually
    /// filed: a [`Connection`] goes in, and a `Connection` comes back out
    /// through [`nvs_runtime::HeldConnection::into_any`], which is the downcast
    /// `nvs-stdlib`'s `warm_connection` performs before it resets anything.
    ///
    /// Per core is asserted as **another core finding nothing**: the store is a
    /// `thread_local!`, which is why § 13's acquire path needs no lock.
    #[test]
    fn the_pool_is_per_core_and_keyed_as_connect_and_open_key() {
        let now = Instant::now();
        let generation = Arc::new(Snapshot::default());
        let reloaded = Arc::new(Snapshot::default());

        release(lease(&generation, "main"), now, idle_connection());

        // Another core, sharing the generation and asking under the same name.
        let elsewhere = Arc::clone(&generation);
        let crossed = std::thread::spawn(move || take(&lease(&elsewhere, "main"), now).is_some())
            .join()
            .expect("the probe thread does not panic");
        assert!(!crossed, "a second core reached this core's pool");

        // § 2's `connect` key is the block name, so a second block is a second
        // pool even on the core that filed this one.
        assert!(
            take(&lease(&generation, "reports"), now).is_none(),
            "`[db.reports]` was handed `[db.main]`'s connection"
        );
        // It is scoped to the generation it was read from, because ADR 0078's
        // reload can put a different database user behind the same name.
        assert!(
            take(&lease(&reloaded, "main"), now).is_none(),
            "a reloaded generation drew a connection authenticated as the old one"
        );
        // § 2's `open` key is a hash of every settings field, and a differing
        // field is a differing string through the same door — the hash below
        // stands for one, since `open` has no caller yet.
        assert!(
            take(&lease(&generation, "7c1f9a2e0b6d4f38"), now).is_none(),
            "an `open` key drew a connection filed under another key"
        );

        let Some(taken) = take(&lease(&generation, "main"), now) else {
            panic!("the connection did not come back under its own key");
        };
        let Ok(connection) = taken.into_any().downcast::<Connection>() else {
            panic!("the pool handed back something that is not a `Connection`");
        };
        assert!(matches!(&*connection, Connection::Sqlite(_)));
        assert!(connection.is_poolable());
    }

    /// Every driver round-trips through the name the matrix harness uses, and
    /// no two share one — the roster `tools/db-matrix.py` selects on is this
    /// array and not a copy of it.
    #[test]
    fn every_drivers_matrix_name_round_trips_and_is_unique() {
        let mut seen = Vec::new();
        for driver in Driver::ALL {
            let name = driver.matrix_name();
            assert_eq!(Driver::from_matrix_name(name), Some(driver));
            assert!(!seen.contains(&name), "two drivers answer to {name}");
            seen.push(name);
        }
        assert_eq!(seen.len(), 5, "ADR 0067 § 12 closes the set at five");
        assert_eq!(Driver::from_matrix_name("oracle"), None);
    }

    /// The two readers agree about every driver, which is the property one
    /// roster buys: a `[db.<name>]` block's `driver` and the harness's
    /// `NVS_DB_MATRIX_DRIVER` name the same backend, and the only difference
    /// between them is the case a hand-written file is allowed.
    #[test]
    fn a_config_blocks_driver_names_the_same_backend_the_matrix_does() {
        for driver in Driver::ALL {
            let name = driver.matrix_name();
            assert_eq!(Driver::from_config_name(name), Some(driver));
            assert_eq!(
                Driver::from_config_name(&name.to_uppercase()),
                Some(driver),
                "{name} written in capitals is the same backend"
            );
            assert_eq!(Driver::from_matrix_name(&name.to_uppercase()), None);
        }
        assert_eq!(Driver::from_config_name("oracle"), None);
        assert_eq!(Driver::from_config_name(""), None);
    }

    /// § 4's rule, asserted on both sides: `Idle` is the only state that
    /// accepts a statement and the only one that may be pooled. Asserting it
    /// over the whole roster rather than on one state is what catches a fifth
    /// state added later without a decision about either question.
    #[test]
    fn only_an_idle_connection_accepts_a_statement_or_rejoins_the_pool() {
        for state in [
            State::Idle,
            State::Executing,
            State::Streaming,
            State::Poisoned,
        ] {
            let idle = state == State::Idle;
            assert_eq!(state.may_start_statement(), idle, "{state:?}");
            assert_eq!(state.is_poolable(), idle, "{state:?}");
        }
    }

    /// The one that is a security boundary rather than an ordinary refusal: a
    /// poisoned wire is never poolable, and no path in this module returns it
    /// to `Idle`.
    #[test]
    fn a_poisoned_connection_is_never_poolable() {
        assert!(!State::Poisoned.is_poolable());
        assert!(!State::Poisoned.may_start_statement());
    }
}
