//! The drivers as an enum, and the busy state each one carries.
//!
//! [ADR 0132 § 5](/docs/decisions/0132.md)
//! decides that this is an `enum` and not a `Driver` trait, for three reasons
//! in this project's priority order: the set is **closed** (`rule:core-classes/db-one-api` makes
//! a new backend an ADR rather than a plugin, and wasm extensions cannot host
//! one anyway, so open-set extensibility is the one property a trait buys and
//! this design does not want it); a trait wide enough for every driver would be
//! half `unimplemented!()`, since `executeMany` is one prepare and N executions
//! on most of them and a bulk protocol message on MariaDB, a prepare is a round
//! trip on some and free on SQLite, and a reset is its own command sequence on
//! each wire driver and a rollback on SQLite; and a `Box<dyn Driver>` is an
//! allocation and an indirect call per connection operation where a `match` is
//! something the compiler can see through.
//!
//! So where a signature repeats per driver, **it repeats**. A `fn query` body
//! per protocol, each saying what its own does, is cheaper to read and to fix
//! than one that makes most of them lie, and what keeps them honest is not a
//! type — it is `tools/db-matrix.py` running one assertion set against every
//! real server.
//!
//! This module holds those shapes and the one rule they share. A driver's own
//! wire code — its handshake, its sequencing, its error table — is in that
//! driver's module beside this one ([`crate::pg`] is one), so the enum
//! stays readable as an enum while a protocol grows to the size a protocol is.
//!
//! What is *not* per driver is § 4's rule about when a connection may be
//! written to and when it may be pooled: that is one rule over [`State`], and
//! it lives on that type rather than being restated once per driver. What each
//! driver decides for itself is which state a given wire event leaves it in —
//! whether an abandoned result set can be cancelled and drained back to
//! [`State::Idle`], or is [`State::Poisoned`].

use std::cell::Cell;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use mysql_common::constants::CapabilityFlags;

use crate::mysql::Wire as MyWire;
use crate::pg::{CancelKey, Wire};
use crate::sql::StatementCache;
use crate::tds::Wire as TdsWire;

/// Where one server is: the address a host resolved to, or the path an operator
/// wrote in place of one.
///
/// `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host` is the spelling
/// — a `[db.<name>] host` beginning with a path separator is a socket, on the
/// overload `[server] listen` already established — and
/// `rule:core-classes/db-unix-socket-path` is what each driver then does with the
/// path, which is not the same thing on all of them. This type is where the two
/// transports meet: a driver's `connect` takes it rather than a [`SocketAddr`]
/// only one of them can be said in.
///
/// The socket arm is `#[cfg(unix)]` rather than a variant that refuses when it
/// is dialled, so a build with no `AF_UNIX` transport cannot hold a path to dial
/// at all — the platform half of
/// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`, made
/// impossible by the type instead of caught by a check. `nvs-stdlib`'s
/// `cache::Target` is this same shape over the shared store.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Endpoint {
    /// A `host[:port]`, resolved to the one address the connection is made to by
    /// whoever checked the `db.connect` capability. A driver that re-resolved
    /// the name would be connecting somewhere nobody approved.
    Tcp(SocketAddr),
    /// A path an operator wrote, exactly as they wrote it: there is nothing to
    /// resolve, and
    /// `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`
    /// is why no program can name one.
    #[cfg(unix)]
    Socket(std::path::PathBuf),
}

impl From<SocketAddr> for Endpoint {
    /// An address is an endpoint, so a caller that has resolved one hands it
    /// over as it stands.
    ///
    /// This is what lets a driver widen to [`Endpoint`] without every TCP caller
    /// in the tree being rewritten to say so — `nvs-cli`'s two openers dial one
    /// driver per macro arm and have no place to name a transport, and a socket
    /// is admitted by the *configuration* reader
    /// (`rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`)
    /// rather than by anything downstream of it.
    fn from(address: SocketAddr) -> Self {
        Self::Tcp(address)
    }
}

impl std::fmt::Display for Endpoint {
    /// What a failure names the server as, which is the spelling an operator
    /// wrote rather than a description of it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp(address) => write!(f, "{address}"),
            #[cfg(unix)]
            Self::Socket(path) => write!(f, "{}", path.display()),
        }
    }
}

/// Whether `host` is a socket rather than a name to resolve.
///
/// One predicate, here rather than once per driver: a value beginning with a
/// path separator is a socket and no `host:port` can be spelled that way, which
/// is `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`'s overload
/// and `nvs_config`'s `[server] listen` reader is where it was first written.
#[must_use]
pub fn is_socket_host(host: &str) -> bool {
    host.starts_with(std::path::is_separator)
}

/// The endpoint a socket `host` names.
///
/// # Errors
///
/// Nothing on this platform; the signature is the one the refusal below needs.
#[cfg(unix)]
pub fn socket_endpoint(host: &str) -> std::io::Result<Endpoint> {
    Ok(Endpoint::Socket(std::path::PathBuf::from(host)))
}

/// The refusal a build with no `AF_UNIX` transport answers a socket `host` with.
///
/// A refusal and never a fallback: a Unix spelling on a platform without the
/// transport does not quietly become loopback TCP, because the two are not the
/// same server and an operator who wrote a path did not ask for a port.
/// [ADR 0142 § 4](/docs/decisions/0142.md) is the argument, and
/// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot` the
/// rule it left.
///
/// # Errors
///
/// `Unsupported`, always.
#[cfg(not(unix))]
pub fn socket_endpoint(host: &str) -> std::io::Result<Endpoint> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!(
            "`{host}` is a Unix-domain socket and this build has no `AF_UNIX` transport: \
             a Unix spelling is refused rather than read as loopback TCP"
        ),
    ))
}

/// The backends [ADR 0067 § 12](/docs/decisions/0067.md) closes
/// the set at.
///
/// MariaDB is its own driver and not a MySQL flag: the two have diverged in
/// auth plugins, error tables and bulk protocol, and `rule:core-classes/db-one-api` argues that at
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
    /// Every driver, in the order `rule:core-classes/db-one-api`'s own tables use.
    ///
    /// The matrix harness iterates this rather than a list of its own, so a new
    /// backend cannot be added to the language without appearing in the
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

    /// This backend as its vendor spells it, for a sentence an operator reads.
    ///
    /// [`Driver::matrix_name`]'s opposite: that one is the identifier a file
    /// and a harness write, this one is prose and appears only inside a
    /// [`BlockError`]'s message. Nothing parses it, and nothing may — a second
    /// roster that something matched on is exactly what that method's doc
    /// warns about.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Driver::Postgres => "PostgreSQL",
            Driver::MySql => "MySQL",
            Driver::MariaDb => "MariaDB",
            Driver::SqlServer => "SQL Server",
            Driver::Sqlite => "SQLite",
        }
    }
}

/// Why a `[db.<name>]` block is not a connection of the driver that read it —
/// [`crate::PgTarget::resolve`]'s refusal and [`crate::MySqlTarget::resolve`]'s
/// alike.
///
/// **A value, not a rendered message**: it names the *field* that is wrong and
/// borrows what the block wrote, so the caller composing the operator-facing
/// text decides the wording around it. [`BlockError::refusal`] is that text for
/// a caller that has nothing better to say, and it is the one place a block's
/// name is joined to a field's fault.
///
/// A block is read once, when a connection is opened, so a refusal here is a
/// boot-shaped error arriving at the first `Core\Db::connect` rather than a
/// per-request condition: nothing about it depends on the request, and the same
/// block refuses the same way every time until an operator edits the file.
///
/// **It is here rather than in a driver because every driver refuses the same
/// things**, differing only in which backend was expected — that is the
/// `expected` field on the variants whose sentence names one. A driver
/// with a fault none of these covers adds a variant here; it does not grow an
/// error type of its own, because an operator reading two of those would be
/// reading two vocabularies for one file format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockError<'a> {
    /// The block names no `driver` at all, so nothing decides which of ADR
    /// 0067 § 12's backends it is.
    NoDriver,
    /// `driver` names a backend Novis has, and it is not the one that read it.
    OtherDriver {
        /// What the block wrote.
        written: &'a str,
        /// The driver that spelling names.
        driver: Driver,
        /// The driver whose resolver read the block.
        expected: Driver,
    },
    /// `driver` names no driver Novis has.
    UnknownDriver {
        /// What the block wrote.
        written: &'a str,
    },
    /// A field the startup exchange sends, absent from the block.
    Missing {
        /// The block's key, as an operator wrote it.
        field: &'static str,
        /// The driver whose resolver read the block.
        expected: Driver,
    },
    /// The same field, written with no value in it.
    Blank {
        /// The block's key, as an operator wrote it.
        field: &'static str,
    },
    /// `password_file` is set and no password was materialized from it — the
    /// block was read without `nvs_config::secret`'s pass over the tree, which
    /// is a caller's bug rather than an operator's.
    SecretUnread,
    /// A field belonging to another driver, written on this one. Silently
    /// ignoring it is `rule:core-classes/db-connection-is-named`'s discriminated union giving way.
    Unusable {
        /// The block's key, as an operator wrote it.
        field: &'static str,
        /// The driver whose resolver read the block.
        expected: Driver,
    },
    /// `host` is a Unix-domain socket path ([`is_socket_host`]) written on a
    /// driver whose protocol has no transport for one.
    ///
    /// `rule:core-classes/db-unix-socket-path` puts this here rather than at
    /// the dial: MSSQL reports a path as **a target it does not speak**, not as
    /// a file it could not open, so the refusal arrives when the block is read
    /// and names the field an operator would edit.
    NoSocketTransport {
        /// What the block wrote.
        written: &'a str,
        /// The driver whose resolver read the block.
        expected: Driver,
    },
    /// `time_zone` is written and is not one of § 9's offsets — the `None`
    /// [`crate::sql::time_zone_for`] answers with, turned into a refusal here
    /// rather than folded into UTC.
    TimeZone {
        /// What the block wrote.
        written: &'a str,
    },
}

impl BlockError<'_> {
    /// The refusal as an operator reads it, naming the block it is about.
    ///
    /// `name` is the `[db.<name>]` key, which the block itself does not carry:
    /// a `Database` is the block's *fields*, and which name they were written
    /// under is the map's key in `nvs_config`.
    #[must_use]
    pub fn refusal(&self, name: &str) -> String {
        format!("[db.{name}]: {self}")
    }
}

impl std::fmt::Display for BlockError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockError::NoDriver => {
                write!(
                    f,
                    "the block names no `driver`, so nothing says which database it is"
                )
            }
            BlockError::OtherDriver {
                written,
                driver,
                expected,
            } => write!(
                f,
                "`driver` is `{written}`, which is the {} driver and not {}",
                driver.matrix_name(),
                expected.display_name()
            ),
            BlockError::UnknownDriver { written } => write!(
                f,
                "`driver` is `{written}`, which is none of `postgres`, `mysql`, `mariadb`, \
                 `mssql` or `sqlite`"
            ),
            BlockError::Missing { field, expected } => write!(
                f,
                "the block names no `{field}`, which a {} connection cannot be opened without",
                expected.display_name()
            ),
            BlockError::Blank { field } => {
                write!(
                    f,
                    "`{field}` is written empty, which is not a value to open a connection with"
                )
            }
            BlockError::SecretUnread => write!(
                f,
                "`password_file` is set and no password was read from it, so this tree was never \
                 handed to `nvs_config::secret`"
            ),
            BlockError::Unusable { field, expected } => write!(
                f,
                "`{field}` belongs to another driver, and a {} connection reads nothing from it",
                expected.display_name()
            ),
            BlockError::NoSocketTransport { written, expected } => write!(
                f,
                "`host` is `{written}`, which is a Unix-domain socket, and the {} protocol has no \
                 transport that speaks one: write the `host:port` the server listens on",
                expected.display_name()
            ),
            BlockError::TimeZone { written } => write!(
                f,
                "`time_zone` is `{written}`, which is not an offset: write `+02:00`, `-05:30` or \
                 `UTC`, or leave it unset for UTC"
            ),
        }
    }
}

impl std::error::Error for BlockError<'_> {}

/// One written field of a `[db.<name>]` block, refused by its own key when it
/// is absent or holds nothing.
///
/// Whitespace-only counts as nothing here — a hostname or a user of two spaces
/// is a field an editor left half-written, and sending it would fail against
/// the server with a message about neither. A password does not come through
/// this function, for the opposite reason: `nvs_config::secret` trims nothing
/// off a secret file, so a password of spaces is a password.
pub(crate) fn written_value<'a>(
    value: Option<&'a str>,
    field: &'static str,
    expected: Driver,
) -> Result<&'a str, BlockError<'a>> {
    match value {
        None => Err(BlockError::Missing { field, expected }),
        Some(value) if value.trim().is_empty() => Err(BlockError::Blank { field }),
        Some(value) => Ok(value),
    }
}

/// Where a connection's wire is, between commands —
/// [ADR 0132 § 4](/docs/decisions/0132.md).
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
    /// Rows remain unread. A second statement here is `rule:core-classes/db-statement-members`'s
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
    /// in one place per driver and naming both of `rule:core-classes/db-statement-members`'s fixes —
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
    /// **closed, never reset**: `rule:security/db-pool-reset-is-a-boundary` makes the reset a boundary
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

/// The isolation levels [ADR 0067 § 7](/docs/decisions/0067.md)'s
/// `transaction()` takes, as a caller asks for them rather than as any one
/// server spells them.
///
/// The set is the SQL standard's four plus [`Isolation::Snapshot`], which SQL
/// Server has as a level of its own and the others reach under another name —
/// so this is one enum with a per-driver rendering rather than one overlapping
/// enum per driver, for the same reason [`State`] is one rule rather than one
/// per driver.
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
/// cases `Core\Db\Column::type` answers with.
///
/// **This is not a summary of what a read of the column produces**, and the two
/// questions are deliberately different. [ADR 0067
/// § 9](/docs/decisions/0067.md)'s type map decodes a `JSON` column to
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

/// [ADR 0067 § 8](/docs/decisions/0067.md)'s normalised `ErrorKind`,
/// under a name that cannot be misread as [`std::io::ErrorKind`] in a driver
/// that spells both in one function.
///
/// § 8's whole point is that an application branches on the *condition* rather
/// than on a vendor string: PDO exposes only a `SQLSTATE` and a vendor integer,
/// which is why real PHP matches on `"Duplicate entry"` or hard-codes `1062`.
/// Each driver maps its own codes onto this set and MariaDB needs its own table
/// rather than MySQL's; PostgreSQL's is `pg.rs`'s `kind_of` and MySQL's is
/// `mysql.rs`'s, keyed on the vendor integer where PostgreSQL's is keyed on the
/// `SQLSTATE`, because that is the specified half on each. The raw code stays
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
    /// Whether [ADR 0067 § 7](/docs/decisions/0067.md)'s
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
/// § 8's `driverCode` is a field PostgreSQL will always answer `None` for: the
/// `SQLSTATE` *is* that server's code, and a second integer invented to fill a
/// shape would be a value with no meaning. MySQL sends a real one beside the
/// `SQLSTATE`, and it is the half that server's code table is keyed on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError {
    /// § 8's normalised kind, from the driver's own code table.
    pub kind: DbErrorKind,
    /// The five-character `SQLSTATE`, as the server sent it, or **empty on a
    /// backend that sends none**.
    ///
    /// § 8 makes the field `?string` and SQL Server is why: TDS has no
    /// `SQLSTATE` at all, and the five characters PDO reports for it are ODBC's
    /// own mapping from the error number rather than anything the server said.
    /// An empty string is not a `SQLSTATE` any server can send, so it carries
    /// the absence without widening the field to an `Option` every reader on a
    /// backend that does send one would then have to unwrap; the readers there
    /// are — this type's `Display` below, and `nvs_stdlib`'s `sqlState` slot —
    /// treat it as one, which is what makes the program-visible value `null`
    /// rather than `""`.
    pub sql_state: String,
    /// The server's non-localized severity — `ERROR`, `FATAL`, `PANIC`.
    pub severity: String,
    /// The server's own sentence.
    pub message: String,
    /// The constraint the condition names, where it names one.
    pub constraint: Option<String>,
    /// § 8's `driverCode`: the server's own integer, on a backend that sends
    /// one beside the `SQLSTATE`.
    ///
    /// `u32` because SQL Server's is a `LONG` and a `THROW` may raise one past
    /// 65535 — the whole user-defined range starts at 50000 — where MySQL's
    /// `ERR` packet carries a `u16`. A narrower field would lose `driverCode`
    /// for every error an application raised itself, which is the half of § 8 a
    /// normalised kind cannot cover. § 8 spells the field `?int`, so nothing
    /// above this widens with it.
    ///
    /// The raw value stays here for the conditions [`DbErrorKind`] does not
    /// name, exactly as `sql_state` does — and on SQL Server it is the *only*
    /// raw value there is, since that server sends no `SQLSTATE` to fall back
    /// to.
    pub driver_code: Option<u32>,
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
    /// Severity, message and `SQLSTATE`: the fields an operator acts on.
    ///
    /// The `SQLSTATE` is omitted rather than rendered empty where the backend
    /// sends none — see [`ServerError::sql_state`]. `(SQLSTATE )` would read
    /// as a server that answered nothing when the truth is a protocol that has
    /// no such field, and the number that *is* this backend's code is already
    /// in the sentence the server wrote.
    ///
    /// Bound parameters are not among them and never will be — `rule:core-classes/db-error`
    /// makes a `Throwable` message a `secret` sink, and this sentence is what
    /// reaches one.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.backend, self.severity, self.message)?;
        if self.sql_state.is_empty() {
            Ok(())
        } else {
            write!(f, " (SQLSTATE {})", self.sql_state)
        }
    }
}

impl std::error::Error for ServerError {}

/// A PostgreSQL connection: `postgres-protocol`'s codec plus the extended-query
/// state machine, the SASL handshake and `rule:security/db-pool-reset-is-a-boundary`'s reset, written here.
///
/// The reset is deliberately **not** `DISCARD ALL`, which would deallocate the
/// prepared statements the statement cache exists to preserve.
#[derive(Debug)]
pub struct PgConn {
    /// The stream and the bytes read off it that are not yet a whole message,
    /// once [`crate::pg::PgConn::connect`] has upgraded it.
    ///
    /// `rule:security/one-tls-client`'s in-band upgrade means the `SSLRequest` and its one-byte
    /// answer are the only plaintext this connection ever carries, so there is
    /// no variant here for "not yet encrypted": a connection that did not
    /// upgrade was never built.
    pub(crate) wire: Wire,
    /// `rule:core-classes/db-connection-busy-state`'s busy state. A plain [`Cell`]: no atomic and no lock,
    /// because a task never migrates and a connection is owned by one request
    /// at a time, which `nvs-host`'s `!Send` scheduler makes true rather than
    /// hoped.
    pub(crate) state: Cell<State>,
    /// What the backend handed over at startup so a *second* connection can
    /// cancel this one's query — the only way PostgreSQL offers, and it is
    /// unrepeatable: the key arrives once, during the handshake, and a
    /// connection that dropped it cannot ask again.
    pub(crate) cancel: CancelKey,
    /// The `server_version` the backend reported during startup, verbatim —
    /// [ADR 0187 § 2](/docs/decisions/0187.md)'s answer for this driver.
    ///
    /// Kept because the parameter is delivered once and asking for it again
    /// would be a round trip behind a member that reads like a field read. A
    /// `ParameterStatus` arriving later cannot move it: PostgreSQL reports this
    /// one at startup alone, and a server does not change version underneath a
    /// session.
    ///
    /// **What it spends:** one short string per open connection.
    pub(crate) server_version: String,
    /// `rule:core-classes/db-one-api`'s LRU of server-side prepared statements, keyed by SQL text
    /// plus expansion arity.
    ///
    /// It is on the connection because a prepared statement is a name on one
    /// session: two connections sharing this would bind against names the
    /// other's server has never heard of. It survives this driver's reset,
    /// which is § 13's whole reason for not sending `DISCARD ALL`.
    pub(crate) cache: StatementCache,
    /// `rule:core-classes/db-column-types`'s declared zone, as seconds east of UTC — what a zone-less
    /// `TIMESTAMP` column off this connection is read in.
    ///
    /// Held rather than re-derived because the decode of that row happens in
    /// `nvs-stdlib`, the only crate that can build the `Core\Time\DateTime` it
    /// becomes, and [`PgTarget`](crate::pg::PgTarget) does not outlive the
    /// handshake. Four bytes a connection, against a config lookup a column.
    pub(crate) time_zone: i32,
    /// How many of `rule:core-classes/db-transactions`'s transactions are open on this connection: 0
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
    /// The read state of a statement whose rows a *held cursor* is walking,
    /// parked here rather than lent out inside a borrow of this connection.
    ///
    /// `rule:core-classes/db-statement-members`'s buffered members drain their rows inside the one call
    /// that started the statement, so [`crate::PgRows`] can keep this beside a
    /// borrow of the connection and let the borrow checker be what refuses a
    /// second statement. `Core\Db\Connection::stream` cannot borrow anything: a
    /// cursor is an object the program holds and advances from a *later* call,
    /// with nothing of this connection borrowed in between, so the columns, the
    /// tag, the last id and the span have to sit where that later call can find
    /// them from the connection alone. [`State::Streaming`] is still what
    /// refuses the second statement — the same refusal, read off the state
    /// rather than off a lifetime. `crate::pg::PgCursor` is that state and owns
    /// the rest of the reasoning.
    ///
    /// `None` for a connection that has never streamed. It stays `Some` after a
    /// stream ends, holding what the statement finished with, until the next
    /// `stream` replaces it or `end_stream` drops it — and § 13's reset drops it
    /// too, because "no open cursor" is one of the properties that reset has to
    /// establish before another request may have this connection.
    ///
    /// **What it spends:** one row description and one command tag per
    /// connection that has streamed, held until the next statement replaces
    /// them. That is O(pooled connections) rather than O(requests served), and
    /// § 13's `max` per core is the cap on it.
    pub(crate) reading: Option<crate::pg::PgCursor>,
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
    /// `rule:core-classes/db-connection-busy-state`'s busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
    /// What the two ends agreed this connection can do — the client's set
    /// intersected with the server's greeting.
    ///
    /// Held because MySQL's packets are not self-describing: whether an `OK`
    /// carries session-state changes, and whether a result set is terminated by
    /// an `EOF` packet or by an `OK`, are read off these bits. A driver that
    /// re-derived them per packet would be deciding it twice.
    pub(crate) capabilities: CapabilityFlags,
    /// The greeting's version banner, exactly as the server wrote it —
    /// [ADR 0187 § 2](/docs/decisions/0187.md)'s answer for this driver, taken
    /// off `crate::mysql`'s `Greeting`.
    ///
    /// The banner and not the `(major, minor, patch)` beside it: a
    /// distribution's suffix is part of what is on the other end, and a driver
    /// that answered the triple would hide the half an operator reads.
    ///
    /// **What it spends:** one short string per open connection.
    pub(crate) server_version: String,
    /// `rule:core-classes/db-one-api`'s LRU of server-side prepared statements, keyed by SQL text
    /// plus expansion arity.
    ///
    /// [`PgConn::cache`]'s twin, holding the handle this protocol hands back:
    /// the statement id `COM_STMT_PREPARE` answers with, where PostgreSQL's is
    /// a name that driver mints. It does **not** survive this connection's
    /// reset — § 13's `COM_RESET_CONNECTION` drops the statements themselves —
    /// and `crate::mysql`'s `reset_session` is where the two are emptied
    /// together, so no caller can invalidate one and forget the other.
    pub(crate) cache: StatementCache<crate::mysql::Prepared>,
    /// `rule:core-classes/db-column-types`'s declared zone, as seconds east of UTC — what a zone-less
    /// `DATETIME` or `TIMESTAMP` off this connection is read in.
    ///
    /// Held for [`PgConn::time_zone`]'s reason: the decode of that row happens
    /// in `nvs-stdlib`, and the target does not outlive the handshake.
    pub(crate) time_zone: i32,
    /// How many of `rule:core-classes/db-transactions`'s transactions are open on this connection —
    /// [`PgConn::depth`]'s twin, counted by the same rule and spent on
    /// different commands.
    ///
    /// `crate::mysql`'s `begin` is where the depth decides which one goes out,
    /// and the two drivers differ there rather than here: MySQL's outermost
    /// level is a `START TRANSACTION`, and its nested `ROLLBACK TO SAVEPOINT`
    /// needs no `RELEASE` after it because a same-named `SAVEPOINT` replaces
    /// the one it finds.
    pub(crate) depth: Cell<u32>,
    /// The read state of a statement whose rows a *held cursor* is walking,
    /// parked here rather than lent out inside a borrow of this connection.
    ///
    /// [`PgConn::reading`]'s twin, for that field's reasons and holding this
    /// protocol's own state: the column definitions a binary row can only be
    /// decoded against, the counts the terminator left, and the span. The
    /// refusal a second statement meets is [`State::Streaming`] on either
    /// driver, read off the state rather than off a lifetime.
    ///
    /// `None` for a connection that has never streamed. It stays `Some` after a
    /// walk ends, holding what the statement finished with, until the next
    /// `stream` replaces it or `end_stream` drops it — and § 13's reset drops it
    /// too, after draining whatever the program walked away from, because
    /// `COM_RESET_CONNECTION` is a command and a command written over rows still
    /// arriving is one whose answer nothing can frame.
    ///
    /// **What it spends:** one result set's column definitions per connection
    /// that has streamed, held until the next statement replaces them. That is
    /// O(pooled connections) rather than O(requests served), and § 13's `max`
    /// per core is the cap on it.
    pub(crate) reading: Option<crate::mysql::MySqlCursor>,
}

/// A MariaDB connection: `mysql_common`'s codec, its own auth plugins, its own
/// error table and `RETURNING`.
///
/// Its own driver rather than a MySQL flag — `rule:core-classes/db-one-api` argues that at length and
/// treating it as a flag is a design error, not a simplification.
#[derive(Debug)]
pub struct MariaConn {
    /// The stream, the bytes not yet a whole packet, and the sequence counter —
    /// [`MySqlConn::wire`]'s twin, and the *same* type.
    ///
    /// One protocol is framed once. What differs between the two servers is
    /// carried on that wire as `crate::mysql::Backend`: the name a refusal
    /// reports and `rule:core-classes/db-error`'s table its codes are read against, which for
    /// MariaDB is `crate::maria`'s and not MySQL's.
    pub(crate) wire: MyWire,
    /// `rule:core-classes/db-connection-busy-state`'s busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
    /// What the two ends agreed this connection can do — [`MySqlConn`]'s field
    /// and its reason, MariaDB's packets being as self-describing as MySQL's,
    /// which is to say not at all.
    pub(crate) capabilities: CapabilityFlags,
    /// The greeting's version banner — [`MySqlConn::server_version`]'s field,
    /// and the one place the two servers are told apart by what they say rather
    /// than by which driver opened them: a MariaDB server writes `MariaDB` into
    /// this string itself.
    pub(crate) server_version: String,
    /// `rule:core-classes/db-one-api`'s LRU of server-side prepared statements.
    ///
    /// [`MySqlConn::cache`]'s twin down to the handle type: `COM_STMT_PREPARE`
    /// answers with a statement id on either server, and § 13's
    /// `COM_RESET_CONNECTION` drops the statements along with everything else.
    pub(crate) cache: StatementCache<crate::mysql::Prepared>,
    /// `rule:core-classes/db-column-types`'s declared zone, as seconds east of UTC.
    pub(crate) time_zone: i32,
    /// How many of `rule:core-classes/db-transactions`'s transactions are open — [`MySqlConn::depth`]'s
    /// twin, spent on the same commands.
    pub(crate) depth: Cell<u32>,
    /// The read state of a statement whose rows a *held cursor* is walking —
    /// [`MySqlConn::reading`]'s twin, and the *same* type, because one protocol
    /// is read one way and `crate::mysql`'s row loop serves both servers.
    pub(crate) reading: Option<crate::mysql::MySqlCursor>,
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
    /// The stream and the bytes read off it that are not yet a whole packet,
    /// once [`crate::tds::TdsConn::connect`] has upgraded it.
    ///
    /// Its type is the one that says what § 3 costs on this protocol: a TLS
    /// session over a [`Tunnel`](crate::tds::Tunnel) over the socket, where the
    /// other drivers' is a session over the socket itself. The tunnel is dead
    /// weight after the handshake — it passes bytes through once its end is
    /// told the handshake is done — and it stays in the type because `rustls`
    /// cannot be handed a different stream than the one it handshook over.
    pub(crate) wire: TdsWire,
    /// `rule:core-classes/db-connection-busy-state`'s busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
    /// `major.minor.build` from the `LOGINACK` that accepted the login, decimal
    /// and unpadded — [ADR 0187 § 2](/docs/decisions/0187.md) fixes that
    /// spelling in [`crate::tds::LoginAck::server_version`], because this is the
    /// one backend that sends numbers where the others send a string.
    ///
    /// **What it spends:** one short string per open connection.
    pub(crate) server_version: String,
    /// `rule:core-classes/db-column-types`'s declared zone, as seconds east of UTC — what a `datetime`
    /// or `datetime2` off this connection is read in.
    ///
    /// Held for [`PgConn::time_zone`]'s reason, and **not** also sent: this is
    /// the one driver with no session time zone to send it to, which
    /// [`crate::tds::TdsTarget::time_zone`] owns.
    pub(crate) time_zone: i32,
    /// `rule:core-classes/db-one-api`'s statement cache, keyed on SQL text plus expansion arity.
    ///
    /// Its handle is [`crate::tds::TdsPlan`] rather than the bare number
    /// `sp_prepexec` answers with, and that type's own doc owns why one number
    /// is not enough to decide a hit on this protocol. § 13's reset empties it,
    /// as MySQL's does and unlike PostgreSQL's.
    pub(crate) cache: StatementCache<crate::tds::TdsPlan>,
    /// How many of `rule:core-classes/db-transactions`'s transactions are open — [`MySqlConn::depth`]'s
    /// twin, spent on `BEGIN TRANSACTION` and `SAVE TRANSACTION`.
    pub(crate) depth: Cell<u32>,
    /// Whether this session sits at an isolation level a `transaction()` asked
    /// for rather than at the server's own default.
    ///
    /// **The one piece of state no other driver needs.** T-SQL's
    /// `SET TRANSACTION ISOLATION LEVEL` is *session*-scoped: MySQL's applies to
    /// the next transaction and PostgreSQL's rides the `BEGIN`, but this one
    /// outlives the transaction that asked for it and would silently become the
    /// level of every later statement on the connection. So the level is put
    /// back when the outermost transaction ends, and this flag is what says a
    /// restore is owed — [`crate::tds::begin`] owns the whole rule, including
    /// why the flag is set before the `SET` is written rather than after it
    /// lands.
    pub(crate) isolation_moved: Cell<bool>,
    /// The read state of a statement whose rows a *held cursor* is walking —
    /// [`MySqlConn::reading`]'s field and all of its reasoning, at this
    /// protocol's own read state: what `COLMETADATA` described, what the last
    /// packet carried past the token being parsed, and the counts the `DONE`
    /// left.
    ///
    /// **What it spends:** one result set's column descriptions and about one
    /// packet of buffer, per connection that has streamed, held until the next
    /// statement replaces them. That is O(pooled connections) rather than
    /// O(requests served).
    pub(crate) reading: Option<crate::tds::TdsCursor>,
}

/// A SQLite connection: `rusqlite`, a file handle, and no bytes on any wire.
///
/// It has no socket to park on, so its calls go to `nvs-host`'s blocking pool
/// rather than to a readiness registration. Rolling back an open transaction is
/// the whole of its reset, because a file handle has no session state to leak.
#[derive(Debug)]
pub struct SqliteConn {
    /// The open database, behind the lock that lets it cross to a blocking-pool
    /// thread and back.
    ///
    /// [`mod@crate::sqlite`]'s own doc owns why this is an `Arc<Mutex<_>>` and
    /// not a plain field: `rusqlite::Connection` is `Send` and not `Sync`, so
    /// nothing else satisfies the `Send + 'static` bound
    /// [`nvs_host::blocking::run`] puts on the closure. It is never contended —
    /// [`State`] below gives one request the connection at a time.
    pub(crate) handle: Arc<Mutex<rusqlite::Connection>>,
    /// `rule:core-classes/db-connection-busy-state`'s busy state; the reasoning is on [`PgConn`]. SQLite carries it for
    /// the same reason the others do even with no wire to be mid-message on:
    /// `rule:core-classes/db-statement-members`'s `LogicError` is a property of the API, not of a socket.
    pub(crate) state: Cell<State>,
    /// How many of `rule:core-classes/db-transactions`'s transaction levels are open, exactly as
    /// [`PgConn::depth`] counts them.
    ///
    /// SQLite has `BEGIN` and `SAVEPOINT` like every other backend § 7 reaches,
    /// so the nesting is the same rule; [`mod@crate::sqlite`]'s `begin` owns
    /// which command a level gets and the one place this backend's accounting
    /// differs from PostgreSQL's, which is what a refused outermost `COMMIT`
    /// leaves behind.
    pub(crate) depth: Cell<u32>,
    /// `rule:core-classes/db-column-types`'s declared zone, in seconds east of UTC.
    ///
    /// Read from the block by the same `time_zone_for` every other driver goes
    /// through, and — as on SQL Server — sent nowhere, because there is no
    /// session to send it to. It governs decoding alone.
    pub(crate) time_zone: i32,
    /// `rule:core-classes/a-stream-parks-its-read-on-the-connection`'s parked read, which on this driver is a
    /// thread rather than a buffer.
    ///
    /// [`crate::sqlite::SqliteCursor`] owns why: the statement cannot leave the
    /// thread that prepared it, so the walk is a pool thread holding the
    /// statement and answering a row per step. What is held here is the handle
    /// to it, the description the statement gave and the walk's span.
    ///
    /// **What it spends**: one pooled thread and one row per open walk, plus
    /// that description, for as long as the walk is open — released when it ends
    /// or when the task that opened it does. That is O(open walks) rather than
    /// O(requests served), and it is what replaces [`SqliteRows`](crate::SqliteRows)'
    /// whole result set.
    pub(crate) reading: Option<crate::sqlite::SqliteCursor>,
}

/// One open connection to one database, whichever backend it is.
///
/// [ADR 0132 § 5](/docs/decisions/0132.md):
/// each variant owns its own state machine, its own error-code table and its
/// own reset, and `Core\Db`'s entry points `match` here exactly once.
///
/// `clippy::large_enum_variant` is allowed here and the reasoning is `rule:core-classes/db-drivers-are-an-enum`'s third argument, unchanged: a `Box` around a variant is an allocation
/// and an indirection on every message this enum's own hot path reads. Nothing
/// holds these in an array either: a `Connection` is one live object per pooled
/// connection, which at `rule:security/db-pool-reset-is-a-boundary`'s ceiling of 16 a core is kilobytes.
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

    /// What the other end reported itself as, spelled per driver by
    /// [ADR 0187 § 2](/docs/decisions/0187.md).
    ///
    /// Read off the connection and never asked for: each wire driver kept what
    /// its own handshake had already delivered, so no statement goes out here.
    /// SQLite has no server and no handshake, and its answer is the linked
    /// library's version — a property of this binary rather than of a
    /// connection, so it is read from the library instead of being copied onto
    /// every open database.
    #[must_use]
    pub fn server_version(&self) -> &str {
        match self {
            Connection::Postgres(c) => &c.server_version,
            Connection::MySql(c) => &c.server_version,
            Connection::MariaDb(c) => &c.server_version,
            Connection::SqlServer(c) => &c.server_version,
            Connection::Sqlite(_) => crate::sqlite::library_version(),
        }
    }

    /// Where this connection's wire is — `rule:core-classes/db-connection-busy-state`.
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

    /// Bounds this connection's next exchange by `at`, or lifts the bound —
    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `{timeout?: Duration}`.
    ///
    /// **One clock per connection, on the thing that waits**, which is the same
    /// clock [`PgConn::connect`] and its siblings already file for the
    /// handshake. It bounds a whole *conversation* and not a syscall: a
    /// statement is a prepare, an execute and every row of the answer over one
    /// socket, and it is the statement a caller asked to bound.
    ///
    /// The bound belongs to the connection and outlives the call that set it, so
    /// every statement path files its own — `None` included — and
    /// `nvs_stdlib::db`'s release lifts it before § 13's reset, which is the one
    /// exchange no program's clock may bound.
    ///
    /// # Errors
    ///
    /// Only the SQLite arm can fail, and only as `sqlite3_busy_timeout` fails:
    /// the others write a field on their own socket. See
    /// [`crate::sqlite::set_busy_timeout`] for why that arm bounds a lock wait
    /// where the others bound a read.
    pub fn set_deadline(&mut self, at: Option<std::time::Instant>) -> std::io::Result<()> {
        match self {
            Connection::Postgres(c) => c.wire.set_deadline(at),
            Connection::MySql(c) => c.wire.set_deadline(at),
            Connection::MariaDb(c) => c.wire.set_deadline(at),
            Connection::SqlServer(c) => c.wire.set_deadline(at),
            Connection::Sqlite(c) => return crate::sqlite::set_busy_timeout(c, at),
        }
        Ok(())
    }

    /// Whether a new statement may be written now — `rule:core-classes/db-statement-members`.
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

    /// `rule:security/db-pool-reset-is-a-boundary`'s release gate is [`Connection::is_poolable`], which is
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
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use nvs_config::db::PoolBounds;
    use nvs_config::snapshot::Snapshot;
    use nvs_runtime::pool::{Lease, Ticket, admit, release, take};

    use super::{Connection, DbErrorKind, Driver, SqliteConn, State};

    /// The slot a request holds while one connection under `key` is open.
    ///
    /// `key` is `rule:core-classes/db-connection-is-named`'s, whichever of its two spellings computed it —
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
    /// without a server: `rule:security/one-tls-client` gives it no bytes on any wire, so it is
    /// the one connection type that will never grow a stream a unit test cannot
    /// open. The pool reads the variant no more than it reads the wire — it
    /// asks `is_poolable` and stores the box — so which variant this is says
    /// nothing about the property below.
    fn idle_connection() -> Box<Connection> {
        Box::new(Connection::Sqlite(SqliteConn {
            handle: Arc::new(Mutex::new(
                rusqlite::Connection::open_in_memory().expect("an in-memory database opens"),
            )),
            state: Cell::new(State::Idle),
            depth: Cell::new(0),
            time_zone: 0,
            reading: None,
        }))
    }

    /// `rule:security/db-pool-reset-is-a-boundary`'s first two bullets: the pool is **per core**, and its key
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
        // It is scoped to the generation it was read from, because `rule:config/the-config-is-an-immutable-snapshot`'s
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
        assert_eq!(
            seen.len(),
            5,
            "`rule:core-classes/db-one-api` closes the set at five"
        );
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
    /// over the whole roster rather than on one state is what catches a state
    /// added later without a decision about either question.
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

    /// [ADR 0067 § 8](/docs/decisions/0067.md)'s normalisation read
    /// **across** the drivers instead of down one: one condition, spelled the
    /// way each server spells it, answering one [`DbErrorKind`] on all of them.
    ///
    /// **This is the claim the per-driver cases cannot make.**
    /// `crate::pg`'s `every_sqlstate_classifies_and_only_the_two_the_retry_rule_names_retry`,
    /// `crate::mysql`'s `the_code_table_names_the_kind_and_the_retryable_pair_disagree`
    /// and `crate::maria`'s `mariadb_uses_its_own_code_table_and_not_mysqls`
    /// each read one table alone, so a driver classifying a foreign-key
    /// violation as [`DbErrorKind::Other`] — plausible, since that is exactly
    /// what its own table answers for a code it does not name — passes its case
    /// and still breaks the only promise § 8 makes: that an application
    /// branches on the condition and not on the dialect. Tables that each look
    /// right and disagree with each other are what PDO leaves a caller with,
    /// and it is why real PHP code matches on `"Duplicate entry"`.
    ///
    /// **SQL Server and SQLite are absent because they have no table yet, not
    /// because they are exempt.** § 8's "four drivers and five dialects" is not
    /// met until each of them joins the rows below; a driver landing a
    /// `kind_of` and not a column here has been normalised against nothing.
    #[test]
    fn every_driver_normalises_its_codes_to_one_error_kind() {
        // Each row is one condition an application branches on, then how
        // PostgreSQL, MySQL and MariaDB report it: a `SQLSTATE` for the first,
        // and the vendor integer beside its `SQLSTATE` for the other two, which
        // is the pair their tables are keyed on.
        for (kind, postgres, mysql, maria) in [
            (
                DbErrorKind::UniqueViolation,
                "23505",
                (1062_u16, "23000"),
                (1062_u16, "23000"),
            ),
            (
                DbErrorKind::ForeignKeyViolation,
                "23503",
                (1452, "23000"),
                (1452, "23000"),
            ),
            (
                DbErrorKind::NotNullViolation,
                "23502",
                (1048, "23000"),
                (1048, "23000"),
            ),
            // The row where the three genuinely diverge, and so the row this
            // case exists for: MySQL raises `ER_CHECK_CONSTRAINT_VIOLATED`
            // under its catch-all `HY000`, MariaDB raises its own
            // `ER_CONSTRAINT_FAILED` under the standard's integrity class, and
            // a caller sees one kind.
            (
                DbErrorKind::CheckViolation,
                "23514",
                (3819, "HY000"),
                (4025, "23000"),
            ),
            (
                DbErrorKind::Deadlock,
                "40P01",
                (1213, "40001"),
                (1213, "40001"),
            ),
            // The other class-40 condition: PostgreSQL names it, and both MySQL
            // dialects reach it through the standard's class, `1213` being the
            // one member of the class their tables name outright.
            (
                DbErrorKind::SerializationFailure,
                "40001",
                (9999, "40001"),
                (9999, "40001"),
            ),
            (
                DbErrorKind::ConnectionLost,
                "08006",
                (1053, "08S01"),
                (1053, "08S01"),
            ),
            (
                DbErrorKind::Timeout,
                "57014",
                (1205, "HY000"),
                (1205, "HY000"),
            ),
            (
                DbErrorKind::Syntax,
                "42601",
                (1064, "42000"),
                (1064, "42000"),
            ),
            (
                DbErrorKind::Permission,
                "42501",
                (1045, "28000"),
                (1045, "28000"),
            ),
            // And the kind that means "read the raw values yourself", which has
            // to agree as well: a driver that guessed here would be branching
            // an application on a condition § 8 declines to normalise.
            (
                DbErrorKind::Other,
                "XX000",
                (9999, "HY000"),
                (9999, "HY000"),
            ),
        ] {
            // The condition in words, and a `match` rather than a `{kind:?}`
            // for what it costs a later author: it is exhaustive, so a
            // condition added to `DbErrorKind` stops this crate building until
            // someone stands in this table and decides what each server calls
            // it. A row missing from here is a kind nothing holds the drivers
            // to.
            let condition = match kind {
                DbErrorKind::UniqueViolation => "a duplicate key",
                DbErrorKind::ForeignKeyViolation => "a missing or still-referenced parent row",
                DbErrorKind::NotNullViolation => "a null in a column that refuses one",
                DbErrorKind::CheckViolation => "a `CHECK` constraint the row fails",
                DbErrorKind::Deadlock => "a deadlock the server broke by aborting this transaction",
                DbErrorKind::SerializationFailure => "a serialization conflict the server undid",
                DbErrorKind::ConnectionLost => "the server going away",
                DbErrorKind::Timeout => "a statement that ran out of time",
                DbErrorKind::Syntax => "a malformed statement, or a name that is not there",
                DbErrorKind::Permission => "a privilege the role does not hold",
                DbErrorKind::Other => "a condition § 8 does not normalise",
            };

            for (driver, answered) in [
                (Driver::Postgres, crate::pg::kind_of(postgres)),
                (Driver::MySql, crate::mysql::kind_of(mysql.0, mysql.1)),
                (Driver::MariaDb, crate::maria::kind_of(maria.0, maria.1)),
            ] {
                assert_eq!(
                    answered, kind,
                    "{driver:?} reads {condition} as {answered:?}, and the \
                     other drivers read it as {kind:?} — § 8 exists so that an \
                     application branches on the condition rather than on the \
                     dialect"
                );
            }
        }
    }
}
