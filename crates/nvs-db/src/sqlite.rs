//! SQLite: a file rather than a socket, so every call goes to `nvs-host`'s
//! blocking pool and the rows are in hand before the core is taken back.
//!
//! This is the fifth driver and the only one whose shape is not
//! [ADR 0132](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
//! § 2's borrowed codec over § 3's parking stream. There is no wire: no
//! framing to borrow, no handshake to write, no readiness a reactor could
//! report. `rusqlite` *is* the protocol, and what this module adds around it is
//! the three things the other four get from their own machinery — § 3's
//! handoff off the core, [ADR 0067](../../../docs/adr/0067-core-db.md) § 4's
//! one-statement-at-a-time rule, and § 8's normalised error kinds.
//!
//! # Why every call is a handoff, and what it costs
//!
//! ADR 0106 § 6 admits exactly two ways to wait: park on readiness, or run off
//! the core. A `sqlite3_step` that blocks blocks on a file lock and on the disk,
//! neither of which the kernel will report as readiness, so this driver has only
//! the second. [`nvs_host::blocking::run`] takes `FnOnce() -> T + Send +
//! 'static`, and that bound is the whole reason this module's surface differs
//! from the other four's:
//!
//! - **The handle is an `Arc<Mutex<rusqlite::Connection>>`.** A
//!   `rusqlite::Connection` is `Send` and not `Sync`, so an `Arc` alone will not
//!   cross to a pool thread; the `Mutex` is what makes it `Send`, not a claim
//!   that two callers may hold it. It is never contended — § 4's [`State`] gives
//!   one request the connection at a time — so the cost is one uncontended
//!   atomic per statement.
//! - **Parameters arrive owned**, `Vec<SqliteValue>` where the other four take
//!   `&[Option<&[u8]>]` of already-encoded wire bytes. A borrowed slice cannot
//!   be `'static`, and copying one per statement to satisfy the bound would be a
//!   copy nobody asked for: `nvs-stdlib` builds this vector out of the call's own
//!   values, so taking it by value costs nothing at all.
//! - **The rows come back materialized.** `rusqlite`'s `Rows` borrows the
//!   statement, which borrows the connection, so it cannot outlive the closure
//!   that produced it. [`SqliteRows`] therefore holds the whole result set. That
//!   is memory spent — the priority ordering's last item, bought here for
//!   priority 1: the alternative is pinning the connection to a pool thread for
//!   the life of a cursor, which is a second lifetime rule for one backend and a
//!   thread one request can hold indefinitely. § 4's `stream` is where that bound
//!   has to become a chunk size, and it is not this module's yet.
//!
//! # What is still § 4's rule and not the file's
//!
//! Materialized rows mean a second statement could physically run while a first
//! result set is unread. It still does not: [`SqliteRows`] borrows the
//! connection for as long as it lives and leaves [`State::Streaming`] set until
//! it is dropped, which is [`crate::tds::TdsRows`]' arrangement and for the reason
//! [`SqliteConn::state`]'s own comment gives — § 4's `LogicError` is a property
//! of the API, not of a socket.

use std::cell::Cell;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use nvs_config::tree::Database;
use rusqlite::types::{ToSqlOutput, ValueRef};

use crate::conn::{BlockError, DbErrorKind, Driver, ServerError, SqliteConn, State, written_value};
use crate::sql::{statement_cache_for, time_zone_for};

/// What this backend calls itself on a [`ServerError`] and in a refusal.
const BACKEND: &str = "sqlite";

/// One `[db.<name>]` block as everything opening a SQLite connection needs.
///
/// The target borrows the block for [`crate::MySqlTarget`]'s reason, though the
/// argument is weaker here: there is no password to keep a second copy of, and
/// what is borrowed is a path. It is a target rather than a bare `&Path` because
/// § 1's cache size and § 9's declared zone are read from the same block by the
/// same two readers every other driver goes through, and a driver that took the
/// path alone would be the one backend where those two keys silently did
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteTarget<'a> {
    /// The database file, as the block wrote it.
    ///
    /// ADR 0067 § 3 puts this under `db.connect` exactly as a host is: an
    /// operator wrote it into root-owned configuration, which is the same
    /// authority that granted the capability. A *program-supplied* path is
    /// `open`'s, and § 3 additionally makes that one a path sink needing
    /// `fs.read`/`fs.write` — a rule that lives above this crate, because
    /// nothing here knows which of the two it was handed.
    ///
    /// Resolution against the config file's own directory (ADR 0103 § 5) has
    /// already happened by the time a block reaches here, for the reason the
    /// other drivers' addresses are resolved before they arrive: a driver that
    /// re-resolved a name would be reaching somewhere nobody approved.
    pub path: &'a Path,
    /// The zone a zone-less datetime off this connection is read in, as a whole
    /// number of seconds east of UTC.
    ///
    /// § 9's declared zone, read by the same `time_zone_for` every other block
    /// goes through. Like SQL Server's it governs decoding alone and is sent
    /// nowhere: SQLite has no session state to send it to, and `CURRENT_TIMESTAMP`
    /// is UTC by definition of the function rather than by a setting.
    pub time_zone: i32,
    /// How many prepared statements this connection keeps alive, § 1's
    /// `statement_cache` through the reader every other block goes through.
    ///
    /// `rusqlite` owns the cache itself — see [`open`] — so this is the number
    /// handed to it and not the size of a structure written here.
    pub statement_cache: usize,
}

impl<'a> SqliteTarget<'a> {
    /// One `[db.<name>]` block as this driver's target, or why it is not one.
    ///
    /// # Errors
    ///
    /// [`BlockError`], in the order [`crate::TdsTarget::resolve`] runs its own:
    /// the `driver` first, then every field belonging to another driver, then
    /// `path`, then § 9's zone. The middle group is the long one here and it is
    /// the interesting half — a `[db.x]` block that names `sqlite` and also
    /// writes `host`, `user` and `password` is one an operator retyped from a
    /// server block, and opening it would open a *local file* with the
    /// credentials silently discarded. § 2's discriminated union is refused
    /// field by field rather than by ignoring what does not apply.
    pub fn resolve(block: &'a Database) -> Result<SqliteTarget<'a>, BlockError<'a>> {
        let written = block.driver.as_deref().ok_or(BlockError::NoDriver)?;
        match Driver::from_config_name(written) {
            Some(Driver::Sqlite) => {}
            Some(driver) => {
                return Err(BlockError::OtherDriver {
                    written,
                    driver,
                    expected: Driver::Sqlite,
                });
            }
            None => return Err(BlockError::UnknownDriver { written }),
        }

        // Every key of the server arm, by its own name. `password_file` is in
        // the list beside `password` because a block that named only the file
        // has a secret on disk this connection would not be using, and the
        // refusal an operator needs names the key they wrote.
        for (field, written) in [
            ("host", block.host.is_some()),
            ("port", block.port.is_some()),
            ("user", block.user.is_some()),
            ("password", block.password.is_some()),
            ("password_file", block.password_file.is_some()),
            ("database", block.database.is_some()),
            ("tls_ca_file", block.tls_ca_file.is_some()),
        ] {
            if written {
                return Err(BlockError::Unusable {
                    field,
                    expected: Driver::Sqlite,
                });
            }
        }

        let path = written_value(block.path.as_deref(), "path", Driver::Sqlite)?;

        let Some(time_zone) = time_zone_for(block) else {
            return Err(BlockError::TimeZone {
                // `time_zone_for` answers `Some(0)` for an absent field, so
                // reaching here means the block wrote one.
                written: block.time_zone.as_deref().unwrap_or_default(),
            });
        };

        Ok(SqliteTarget {
            path: Path::new(path),
            time_zone,
            statement_cache: statement_cache_for(block),
        })
    }
}

/// One value, in either direction: SQLite's five storage classes.
///
/// The same enum binds a parameter and carries a read cell, because on this
/// backend those genuinely are one set — SQLite stores a value as one of these
/// five whatever the column was declared as, which is the fact
/// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md) works around by keying its
/// map off the *declared* type. Nothing here is that map: this is the storage
/// class, and turning `Int(20260903)` into a `Core\Time\Date` because the column
/// says `date` happens a layer up.
///
/// `rusqlite`'s own `Value` is deliberately not re-exported in its place. This
/// crate exists to concentrate the audit surface (`Cargo.toml`'s header says
/// so), and a dependency's type in the signature `nvs-stdlib` calls would make
/// that crate depend on the C dependency's Rust wrapper to say anything at all.
#[derive(Debug, Clone, PartialEq)]
pub enum SqliteValue {
    /// SQL `NULL`: § 9's row that makes every column `?T`.
    Null,
    /// `INTEGER` — up to eight bytes, signed. There is no unsigned storage
    /// class, so § 9's `uint` row has no SQLite half.
    Int(i64),
    /// `REAL` — an IEEE-754 double.
    Real(f64),
    /// `TEXT`, already known to be UTF-8: see [`SqliteValue::read`] for why an
    /// invalid one is an error rather than a lossy conversion.
    Text(String),
    /// `BLOB`, which § 9 reads as `tainted bytes` and never as text.
    Blob(Vec<u8>),
}

impl SqliteValue {
    /// One cell of a stepped row, owned so it can outlive the pool thread.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a `TEXT` cell that is not UTF-8. Every other driver
    /// gets [ADR 0009](../../../docs/adr/0009-string-and-bytes.md)'s guarantee
    /// from § 3's forced connection charset; SQLite has no charset to force and
    /// will store whatever bytes were handed to it, so the guarantee has to be
    /// checked here or abandoned. Lossy conversion is the one answer that is
    /// not available — it would hand a program a string the database does not
    /// contain, and a `BLOB` is how bytes that are not text are stored.
    fn read(value: ValueRef<'_>) -> io::Result<SqliteValue> {
        Ok(match value {
            ValueRef::Null => SqliteValue::Null,
            ValueRef::Integer(int) => SqliteValue::Int(int),
            ValueRef::Real(real) => SqliteValue::Real(real),
            ValueRef::Text(bytes) => {
                let text = std::str::from_utf8(bytes).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("a TEXT column holds bytes that are not UTF-8: {e}"),
                    )
                })?;
                SqliteValue::Text(text.to_owned())
            }
            ValueRef::Blob(bytes) => SqliteValue::Blob(bytes.to_vec()),
        })
    }
}

impl rusqlite::types::ToSql for SqliteValue {
    /// Borrowed in every arm, so binding a large `BLOB` or `TEXT` parameter
    /// copies nothing on the way to `sqlite3_bind_*`.
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(match self {
            SqliteValue::Null => ValueRef::Null,
            SqliteValue::Int(int) => ValueRef::Integer(*int),
            SqliteValue::Real(real) => ValueRef::Real(*real),
            SqliteValue::Text(text) => ValueRef::Text(text.as_bytes()),
            SqliteValue::Blob(bytes) => ValueRef::Blob(bytes),
        }))
    }
}

/// One result column, as the statement described it before it was stepped.
///
/// `declared` is the column's *declared* type and is the whole reason this type
/// exists: § 9 maps a SQLite column off what the schema called it, since the
/// storage class of a value says nothing about whether `20260903` was meant as a
/// number or a date. It is `None` for an expression — `select 1 + 1` has no
/// declared type — which is § 9's total case rather than a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteColumn {
    /// The column's name, as `AS` or the schema spelled it.
    pub name: String,
    /// What the schema declared it as, or `None` for an expression.
    ///
    /// **Upper case, whatever the schema wrote** — `rusqlite` normalises it on
    /// the way out, so a column created as `weight real` describes as `REAL`.
    /// It is carried as it arrives rather than re-cased here: § 9's map has to
    /// fold case regardless, since SQLite's own type affinity rules do, and a
    /// second normalisation in between would only be a second place for the two
    /// to disagree.
    pub declared: Option<String>,
}

/// The rows of one statement, already read, and the connection they hold.
///
/// The result set is in memory — this module's doc says why it cannot be
/// otherwise — so `next_row` never fails and never blocks. What the borrow buys
/// is § 4's rule: the connection is [`State::Streaming`] until this is dropped,
/// so a second statement on it is the same refusal it is on every other driver.
#[derive(Debug)]
pub struct SqliteRows<'a> {
    state: &'a Cell<State>,
    columns: Vec<SqliteColumn>,
    rows: std::vec::IntoIter<Vec<SqliteValue>>,
    affected: u64,
    last_insert_id: i64,
}

impl SqliteRows<'_> {
    /// What the statement described, one entry per result column.
    #[must_use]
    pub fn columns(&self) -> &[SqliteColumn] {
        &self.columns
    }

    /// The next row, or `None` once the set is exhausted.
    pub fn next_row(&mut self) -> Option<Vec<SqliteValue>> {
        self.rows.next()
    }

    /// `sqlite3_changes` after the statement finished: § 4's `execute` answer.
    ///
    /// A `SELECT` leaves it at whatever the last data-changing statement on this
    /// connection reported, which is SQLite's own rule and not something to
    /// paper over — `nvs-stdlib` reads this for `execute` and the row count for
    /// `query`, and the two members are where that distinction belongs.
    #[must_use]
    pub fn affected(&self) -> u64 {
        self.affected
    }

    /// `sqlite3_last_insert_rowid` after the statement finished.
    #[must_use]
    pub fn last_insert_id(&self) -> i64 {
        self.last_insert_id
    }
}

impl Drop for SqliteRows<'_> {
    /// Releasing the result set is what ends § 4's statement.
    ///
    /// A poisoned connection stays poisoned: there is no state here to prove
    /// clean, and the rule that a poisoned connection is closed rather than
    /// reused (§ 13) is not a rule one driver relaxes because its failure mode
    /// looks harmless.
    fn drop(&mut self) {
        if self.state.get() == State::Streaming {
            self.state.set(State::Idle);
        }
    }
}

/// Opens the file, and answers the connection ADR 0067 § 4's statements run on.
///
/// The open itself goes off the core: creating or reading a database header is
/// a filesystem call, which is ADR 0106 § 6's first named example.
///
/// Three things are set on the way out, and only the first is a number from the
/// block:
///
/// - **§ 1's statement cache is `rusqlite`'s own**, sized here. That crate keeps
///   an LRU of `sqlite3_stmt` handles keyed on the SQL text, which is exactly
///   what § 1 specifies, so this driver writes no cache of its own — the one
///   place in this crate where the borrowed half covers a section outright.
/// - **`PRAGMA foreign_keys = ON`**, which SQLite leaves off and PDO leaves off
///   after it. § 8 declares `ForeignKeyViolation` as a kind *every* driver
///   normalises onto, and with the pragma off that condition cannot arise at
///   all: the same schema and the same write would refuse on the other four and
///   silently corrupt the reference here. Correctness of semantics is priority 2
///   and the compatibility this costs is with a PHP default that is a known
///   footgun, so it is not configurable to the unsafe value — § 3's own three
///   defaults are settled on the same footing.
/// - **No busy timeout.** A lock contention answers `SQLITE_BUSY` at once, which
///   § 8 normalises to `Deadlock`, which is one of the two kinds § 7's `retries`
///   re-runs a closure on. A timeout set here would block a pool thread inside C
///   for the duration and hide the conflict from the mechanism written to handle
///   it.
///
/// # Errors
///
/// The `io::Error` [`server_error`] builds, carrying § 8's kind: a path that
/// cannot be opened, a file that is not a database, a directory the process may
/// not read.
pub fn open(target: &SqliteTarget<'_>) -> io::Result<SqliteConn> {
    let path = target.path.to_path_buf();
    let capacity = target.statement_cache;

    let handle = nvs_host::blocking::run(move || -> rusqlite::Result<rusqlite::Connection> {
        let connection = rusqlite::Connection::open(path)?;
        connection.set_prepared_statement_cache_capacity(capacity);
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(connection)
    })
    .map_err(server_error)?;

    Ok(SqliteConn {
        handle: Arc::new(Mutex::new(handle)),
        state: Cell::new(State::Idle),
        time_zone: target.time_zone,
    })
}

impl SqliteConn {
    /// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s declared zone, in
    /// seconds east of UTC.
    ///
    /// Public where the other four drivers keep theirs private, because they
    /// decode a zone-less column inside their own codec and this one has no
    /// codec to decode inside: a [`SqliteValue::Int`] out of a column declared
    /// `datetime` is still an integer here, and whoever turns it into a
    /// `Core\Time\DateTime` needs the zone it was written in.
    #[must_use]
    pub fn time_zone(&self) -> i32 {
        self.time_zone
    }

    /// [ADR 0067 § 4](../../../docs/adr/0067-core-db.md)'s statement: prepared
    /// through § 1's cache, stepped off the core, and answered with its rows.
    ///
    /// **`execute` is this same method**, for [`crate::TdsConn::query`]'s reason:
    /// what tells a statement with a result set from one without is
    /// [`SqliteRows::affected`] rather than a second call, and `nvs-stdlib` is
    /// where the two members part.
    ///
    /// # Errors
    ///
    /// [`busy`] when the connection is not at rest, [`server_error`] for
    /// anything SQLite refused, and `InvalidData` for a `TEXT` cell that is not
    /// UTF-8 ([`SqliteValue::read`]).
    pub fn query(&self, sql: &str, params: Vec<SqliteValue>) -> io::Result<SqliteRows<'_>> {
        if !self.state.get().may_start_statement() {
            return Err(busy(self.state.get()));
        }
        self.state.set(State::Executing);

        let handle = Arc::clone(&self.handle);
        let owned = sql.to_owned();
        let read = nvs_host::blocking::run(move || step(&handle, &owned, &params));

        match read {
            Ok(read) => {
                self.state.set(State::Streaming);
                Ok(SqliteRows {
                    state: &self.state,
                    columns: read.columns,
                    rows: read.rows.into_iter(),
                    affected: read.affected,
                    last_insert_id: read.last_insert_id,
                })
            }
            Err(e) => {
                // A refused statement leaves the connection exactly where it
                // was: SQLite either prepared and stepped or did neither, and
                // there is no half-written message to be lost in. This is the
                // one driver whose failure path can honestly return to `Idle`.
                self.state.set(State::Idle);
                Err(e)
            }
        }
    }

    /// [ADR 0067 § 4](../../../docs/adr/0067-core-db.md)'s `executeMany`: one
    /// prepare, N executions, and the affected counts summed.
    ///
    /// The whole loop is *one* handoff off the core rather than one per set.
    /// That is what the blocking pool is for — the thread is held, the core is
    /// not — and the alternative pays § 3's park and wake per row for a call
    /// whose executions are microseconds apart.
    ///
    /// A refusal stops the loop and the sets already applied stay applied, which
    /// is § 4's rule on every driver: `executeMany` is not a transaction, and a
    /// caller that needs one writes § 7's closure around it.
    ///
    /// # Errors
    ///
    /// As [`SqliteConn::query`].
    pub fn execute_many(&self, sql: &str, sets: Vec<Vec<SqliteValue>>) -> io::Result<u64> {
        if !self.state.get().may_start_statement() {
            return Err(busy(self.state.get()));
        }
        self.state.set(State::Executing);

        let handle = Arc::clone(&self.handle);
        let owned = sql.to_owned();
        let applied = nvs_host::blocking::run(move || {
            let guard = lock(&handle);
            let mut statement = guard.prepare_cached(&owned)?;
            let mut affected = 0_u64;
            for set in &sets {
                affected += statement.execute(rusqlite::params_from_iter(set))? as u64;
            }
            Ok(affected)
        });

        self.state.set(State::Idle);
        applied.map_err(server_error)
    }
}

/// What one stepped statement produced, owned and off the pool thread.
struct Read {
    columns: Vec<SqliteColumn>,
    rows: Vec<Vec<SqliteValue>>,
    affected: u64,
    last_insert_id: i64,
}

/// Prepare, bind, step to exhaustion, and hand back what is owned.
///
/// Free rather than a method for the reason the playbook gives about the other
/// drivers' `start_statement`: the sequencing is the part worth testing, and a
/// method on [`SqliteConn`] can only be reached through a [`SqliteTarget`] and a
/// file. It happens that SQLite's file may be `:memory:`, so this one *could*
/// have been a method — the shape is shared anyway, because a reader comparing
/// the five drivers should not have to work out that one of them is different
/// for a reason that is not about the protocol.
fn step(
    handle: &Mutex<rusqlite::Connection>,
    sql: &str,
    params: &[SqliteValue],
) -> io::Result<Read> {
    let guard = lock(handle);
    let mut statement = guard.prepare_cached(sql).map_err(server_error)?;

    // Before the step, because `query` borrows the statement mutably for as
    // long as the rows live. `sqlite3_column_count` and the declared types are
    // available the moment the statement is prepared, so nothing is lost by
    // reading them here.
    let columns: Vec<SqliteColumn> = statement
        .columns()
        .iter()
        .map(|column| SqliteColumn {
            name: column.name().to_owned(),
            declared: column.decl_type().map(str::to_owned),
        })
        .collect();

    let width = columns.len();
    let mut rows = Vec::new();
    let mut cursor = statement
        .query(rusqlite::params_from_iter(params))
        .map_err(server_error)?;
    while let Some(row) = cursor.next().map_err(server_error)? {
        let mut cells = Vec::with_capacity(width);
        for index in 0..width {
            cells.push(SqliteValue::read(
                row.get_ref(index).map_err(server_error)?,
            )?);
        }
        rows.push(cells);
    }
    drop(cursor);
    drop(statement);

    Ok(Read {
        columns,
        rows,
        affected: guard.changes(),
        last_insert_id: guard.last_insert_rowid(),
    })
}

/// The connection's handle, with a poisoned lock read through rather than
/// panicked on.
///
/// A poisoned mutex here means a pool thread unwound while holding the
/// connection, and [`nvs_host::blocking::run`] has already carried that panic
/// back to the task that owns it — where ADR 0106's containment boundary fails
/// one request. Panicking a second time on the next acquire would fail an
/// unrelated one.
fn lock(handle: &Mutex<rusqlite::Connection>) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
    handle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// ADR 0067 § 4's refusal, when a statement is asked for on a connection that is
/// not at rest.
///
/// The wording names both of § 4's fixes, as `pg.rs`'s `second_statement` does
/// and for the same reason: a developer who reaches this has either kept a
/// result set alive or shared a connection across two flows, and the message is
/// the only place that says which two things to look at. `nvs-stdlib` re-words
/// it as the `LogicError` § 4 specifies.
fn busy(state: State) -> io::Error {
    io::Error::other(format!(
        "this SQLite connection is {state:?}: read or drop the rows of the statement already \
         running before starting another, or open a second connection for the second flow"
    ))
}

/// SQLite's refusal as an `io::Error` carrying § 8's normalised kind.
///
/// Two of [`ServerError`]'s fields are empty by nature of this backend rather
/// than by accident. `sql_state` is empty because SQLite sends none — the same
/// absence SQL Server has, carried the same way — and `severity` is always
/// `ERROR` because there is no server to have a log level. `driver_code` is the
/// **extended** result code and not the primary one: `SQLITE_CONSTRAINT` alone
/// does not say which constraint, and the extended code is what an application
/// falling through the normalisation would have to match on.
fn server_error(error: rusqlite::Error) -> io::Error {
    let rusqlite::Error::SqliteFailure(failure, message) = &error else {
        // Not a refusal from the engine: a parameter that would not convert, a
        // statement handed no query at all. It is this crate's bug or its
        // caller's, and it carries no code to classify, exactly as `pg.rs`'s
        // undecodable error response does.
        return io::Error::other(error.to_string());
    };

    let message = message.clone().unwrap_or_else(|| failure.to_string());
    io::Error::other(ServerError {
        kind: kind_of(failure.extended_code),
        // Not a `SQLSTATE` this backend can send — see the field.
        sql_state: String::new(),
        severity: String::from("ERROR"),
        constraint: constraint_of(&message),
        message,
        driver_code: u32::try_from(failure.extended_code).ok(),
        backend: BACKEND,
    })
}

/// [ADR 0067 § 8](../../../docs/adr/0067-core-db.md)'s kind, from SQLite's
/// extended result code.
///
/// Keyed on the extended code because the primary one is too coarse to answer
/// the question § 8 asks: `SQLITE_CONSTRAINT` covers four of the eleven kinds at
/// once, and an application branching on `UniqueViolation` would get nothing
/// from it. Where the extended code adds nothing, the primary code is its low
/// byte and the fallthrough reads that.
///
/// § 8 names two of these rows itself: `SQLITE_BUSY` and `SQLITE_LOCKED` are
/// `Deadlock`, so § 7's `{retries: n}` works on this backend too.
fn kind_of(extended: i32) -> DbErrorKind {
    use rusqlite::ffi;

    match extended {
        ffi::SQLITE_CONSTRAINT_UNIQUE | ffi::SQLITE_CONSTRAINT_PRIMARYKEY => {
            DbErrorKind::UniqueViolation
        }
        ffi::SQLITE_CONSTRAINT_FOREIGNKEY => DbErrorKind::ForeignKeyViolation,
        ffi::SQLITE_CONSTRAINT_NOTNULL => DbErrorKind::NotNullViolation,
        // A `CHECK` and the trigger-raised refusal are one kind: `RAISE(ABORT)`
        // in a trigger is how a schema writes a check SQLite's own `CHECK`
        // cannot express, and § 8 normalises the condition rather than the
        // mechanism.
        ffi::SQLITE_CONSTRAINT_CHECK | ffi::SQLITE_CONSTRAINT_TRIGGER => {
            DbErrorKind::CheckViolation
        }
        _ => match extended & 0xff {
            ffi::SQLITE_BUSY | ffi::SQLITE_LOCKED => DbErrorKind::Deadlock,
            ffi::SQLITE_PERM | ffi::SQLITE_READONLY | ffi::SQLITE_AUTH => DbErrorKind::Permission,
            // The file is gone, unreadable or not a database: the handle cannot
            // be used again, which is what `ConnectionLost` means to a caller
            // even where there was never a connection to lose.
            ffi::SQLITE_IOERR | ffi::SQLITE_CANTOPEN | ffi::SQLITE_NOTADB => {
                DbErrorKind::ConnectionLost
            }
            // `sqlite3_interrupt` is how a statement is cancelled, so this is
            // the deadline having fired and not an unspecified failure.
            ffi::SQLITE_INTERRUPT => DbErrorKind::Timeout,
            // `SQLITE_ERROR` is the whole of "the engine will not run this":
            // a syntax error, an unknown table, a column that does not exist.
            ffi::SQLITE_ERROR => DbErrorKind::Syntax,
            _ => DbErrorKind::Other,
        },
    }
}

/// § 8's `constraint`, out of the sentence SQLite writes it into.
///
/// The C API has no field for it — `sqlite3_errmsg` is all there is — and the
/// text is `UNIQUE constraint failed: t.c` in a form the engine has written the
/// same way for its whole history. Reading it is the only way this field is ever
/// non-`null` on this backend, and answering `None` where the shape does not
/// match costs a caller nothing it did not already have to handle: § 8 makes the
/// field `?string` because some boundaries are driver-dependent.
fn constraint_of(message: &str) -> Option<String> {
    let named = message.split_once(" constraint failed: ")?.1;
    let named = named.trim();
    (!named.is_empty()).then(|| named.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{SqliteTarget, SqliteValue, open};
    use crate::conn::{BlockError, DbErrorKind, Driver, ServerError, State};
    use nvs_config::tree::Database;
    use std::path::Path;

    /// A `[db.<name>]` block naming this driver and a file, and nothing else.
    fn block() -> Database {
        Database {
            driver: Some(String::from("sqlite")),
            path: Some(String::from(":memory:")),
            ..Database::default()
        }
    }

    /// A connection to a private in-memory database, which is a real SQLite
    /// engine and not a fixture: every case below runs the statements it claims
    /// to, which is the one thing the other four drivers cannot do in a unit
    /// test.
    fn connect() -> crate::conn::SqliteConn {
        open(&SqliteTarget::resolve(&block()).expect("the block resolves")).expect("it opens")
    }

    /// § 2's block, read as this driver's target.
    #[test]
    fn a_sqlite_block_resolves_to_its_path_and_the_two_shared_keys() {
        let mut block = block();
        block.path = Some(String::from("/srv/app/app.db"));
        block.statement_cache = Some(4);
        block.time_zone = Some(String::from("+02:00"));

        let target = SqliteTarget::resolve(&block).expect("the block resolves");
        assert_eq!(target.path, Path::new("/srv/app/app.db"));
        assert_eq!(target.statement_cache, 4);
        assert_eq!(target.time_zone, 2 * 3600);
    }

    /// § 2's discriminated union, refused key by key rather than by ignoring
    /// what does not apply — a block retyped from a server one opens a local
    /// file with the credentials silently dropped.
    #[test]
    fn every_server_field_on_a_sqlite_block_is_refused_by_its_own_key() {
        /// One key of the server arm, written onto an otherwise valid block.
        type Field = (&'static str, fn(&mut Database));

        let fields: [Field; 7] = [
            ("host", |b| b.host = Some(String::from("db.internal"))),
            ("port", |b| b.port = Some(5432)),
            ("user", |b| b.user = Some(String::from("app"))),
            ("password", |b| b.password = Some(String::from("s3cret"))),
            ("password_file", |b| {
                b.password_file = Some(String::from("/run/secrets/db"));
            }),
            ("database", |b| b.database = Some(String::from("app"))),
            ("tls_ca_file", |b| {
                b.tls_ca_file = Some(String::from("/etc/ssl/ca.pem"));
            }),
        ];

        for (name, write) in fields {
            let mut block = block();
            write(&mut block);
            assert_eq!(
                SqliteTarget::resolve(&block),
                Err(BlockError::Unusable {
                    field: name,
                    expected: Driver::Sqlite,
                }),
                "`{name}` was not refused by its own key"
            );
        }
    }

    /// The three ways the block says it is not this driver's, and the missing
    /// `path` that says it is not openable.
    #[test]
    fn a_block_of_another_driver_or_with_no_path_is_refused() {
        let mut other = block();
        other.driver = Some(String::from("postgres"));
        assert_eq!(
            SqliteTarget::resolve(&other),
            Err(BlockError::OtherDriver {
                written: "postgres",
                driver: Driver::Postgres,
                expected: Driver::Sqlite,
            })
        );

        let mut pathless = block();
        pathless.path = None;
        assert_eq!(
            SqliteTarget::resolve(&pathless),
            Err(BlockError::Missing {
                field: "path",
                expected: Driver::Sqlite,
            })
        );

        let mut blank = block();
        blank.path = Some(String::from("  "));
        assert_eq!(
            SqliteTarget::resolve(&blank),
            Err(BlockError::Blank { field: "path" })
        );
    }

    /// § 4's statement end to end: a schema, a bound insert, and the read back
    /// carrying every storage class it was given.
    #[test]
    fn a_statement_runs_off_the_core_and_answers_its_rows() {
        let conn = connect();
        conn.query(
            "create table t (id integer primary key, name text, weight real, raw blob, gap text)",
            Vec::new(),
        )
        .expect("the schema applies");

        let inserted = conn
            .query(
                "insert into t (id, name, weight, raw, gap) values (?, ?, ?, ?, ?)",
                vec![
                    SqliteValue::Int(1),
                    SqliteValue::Text(String::from("novis")),
                    SqliteValue::Real(1.5),
                    SqliteValue::Blob(vec![0, 159, 146, 150]),
                    SqliteValue::Null,
                ],
            )
            .expect("the insert runs");
        assert_eq!(inserted.affected(), 1);
        assert_eq!(inserted.last_insert_id(), 1);
        drop(inserted);

        let mut rows = conn
            .query(
                "select id, name, weight, raw, gap from t where id = ?",
                vec![SqliteValue::Int(1)],
            )
            .expect("the select runs");
        assert_eq!(
            rows.columns()
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["id", "name", "weight", "raw", "gap"]
        );
        assert_eq!(
            rows.columns()[2].declared.as_deref(),
            Some("REAL"),
            "§ 9 keys off the declared type, which arrives upper-cased whatever the schema wrote"
        );
        assert_eq!(
            rows.next_row(),
            Some(vec![
                SqliteValue::Int(1),
                SqliteValue::Text(String::from("novis")),
                SqliteValue::Real(1.5),
                SqliteValue::Blob(vec![0, 159, 146, 150]),
                SqliteValue::Null,
            ])
        );
        assert_eq!(rows.next_row(), None);
    }

    /// An expression has no declared type, which § 9 reads as its total case
    /// rather than as a failure.
    #[test]
    fn an_expression_column_declares_no_type() {
        let conn = connect();
        let rows = conn
            .query("select 1 + 1 as sum", Vec::new())
            .expect("it runs");
        assert_eq!(rows.columns()[0].name, "sum");
        assert_eq!(rows.columns()[0].declared, None);
    }

    /// § 4's `LogicError`, which is a property of the API and not of a socket:
    /// the rows are already in memory, and the connection is still held.
    #[test]
    fn a_second_statement_while_rows_are_held_is_the_busy_refusal() {
        let conn = connect();
        let rows = conn.query("select 1", Vec::new()).expect("it runs");
        assert_eq!(conn.state.get(), State::Streaming);

        let refused = conn
            .query("select 2", Vec::new())
            .expect_err("a second statement is refused");
        assert!(
            refused.to_string().contains("open a second connection"),
            "the refusal names both fixes: {refused}"
        );
        assert!(
            ServerError::of(&refused).is_none(),
            "§ 4's refusal is ours and carries no server kind"
        );

        drop(rows);
        assert_eq!(conn.state.get(), State::Idle);
        conn.query("select 2", Vec::new())
            .expect("the connection is free once the rows are dropped");
    }

    /// § 4's `executeMany`: one prepare, N executions, the counts summed.
    #[test]
    fn execute_many_is_n_executions_and_sums_what_they_affected() {
        let conn = connect();
        conn.query("create table t (n integer)", Vec::new())
            .expect("the schema applies");

        let applied = conn
            .execute_many(
                "insert into t (n) values (?)",
                vec![
                    vec![SqliteValue::Int(1)],
                    vec![SqliteValue::Int(2)],
                    vec![SqliteValue::Int(3)],
                ],
            )
            .expect("every set applies");
        assert_eq!(applied, 3);

        let mut rows = conn
            .query("select count(*) from t", Vec::new())
            .expect("it runs");
        assert_eq!(rows.next_row(), Some(vec![SqliteValue::Int(3)]));
    }

    /// § 8's normalisation, and the constraint name read out of the sentence
    /// because SQLite's C API has no field for it.
    #[test]
    fn a_unique_violation_normalises_and_names_its_constraint() {
        let conn = connect();
        conn.query(
            "create table t (id integer primary key, tag text unique)",
            Vec::new(),
        )
        .expect("the schema applies");
        conn.query("insert into t (id, tag) values (1, 'a')", Vec::new())
            .expect("the first row inserts");

        let refused = conn
            .query("insert into t (id, tag) values (2, 'a')", Vec::new())
            .expect_err("the second row is refused");
        let server = ServerError::of(&refused).expect("a refusal carried no kind");
        assert_eq!(server.kind, DbErrorKind::UniqueViolation);
        assert_eq!(server.constraint.as_deref(), Some("t.tag"));
        assert_eq!(server.backend, "sqlite");
        assert!(
            server.sql_state.is_empty(),
            "SQLite sends no SQLSTATE, and § 8 carries that absence as an empty string"
        );
        assert!(
            server.driver_code.is_some(),
            "the extended code is the raw value"
        );
        assert_eq!(
            conn.state.get(),
            State::Idle,
            "a refusal leaves nothing in flight"
        );
    }

    /// `PRAGMA foreign_keys = ON`, without which § 8's `ForeignKeyViolation`
    /// could not arise on this backend at all.
    #[test]
    fn foreign_keys_are_enforced_so_section_8s_kind_is_reachable() {
        let conn = connect();
        conn.query("create table parent (id integer primary key)", Vec::new())
            .expect("the parent applies");
        conn.query(
            "create table child (id integer primary key, parent integer references parent(id))",
            Vec::new(),
        )
        .expect("the child applies");

        let refused = conn
            .query("insert into child (id, parent) values (1, 99)", Vec::new())
            .expect_err("a dangling reference is refused");
        assert_eq!(
            ServerError::of(&refused)
                .expect("a refusal carried no kind")
                .kind,
            DbErrorKind::ForeignKeyViolation
        );
    }

    /// A statement the engine will not run is § 8's `Syntax`, off the primary
    /// code rather than off any extended one.
    #[test]
    fn an_unrunnable_statement_is_section_8s_syntax_kind() {
        let conn = connect();
        let refused = conn
            .query("select * from nothing_here", Vec::new())
            .expect_err("an unknown table is refused");
        assert_eq!(
            ServerError::of(&refused)
                .expect("a refusal carried no kind")
                .kind,
            DbErrorKind::Syntax
        );
    }

    /// ADR 0009's guarantee, which every other driver gets from § 3's forced
    /// charset and this one has to check: a `TEXT` cell that is not UTF-8 is an
    /// error, never a lossy string.
    #[test]
    fn a_text_cell_that_is_not_utf8_is_refused_rather_than_mangled() {
        let conn = connect();
        conn.query("create table t (v text)", Vec::new())
            .expect("the schema applies");
        // `cast` is the only way to put invalid UTF-8 into a TEXT column: a
        // bound `BLOB` stays a blob, and this is what a file written by another
        // application can hold.
        conn.query("insert into t (v) values (cast(x'ff' as text))", Vec::new())
            .expect("the insert runs");

        let refused = conn
            .query("select v from t", Vec::new())
            .expect_err("the read is refused");
        assert_eq!(refused.kind(), std::io::ErrorKind::InvalidData);
    }
}
