//! SQLite: a file rather than a socket, so every call goes to `nvs-host`'s
//! blocking pool and the rows are in hand before the core is taken back.
//!
//! This is the one driver whose shape is not
//! `rule:core-classes/db-crate-boundary`'s borrowed codec over § 3's parking stream. There is no wire: no
//! framing to borrow, no handshake to write, no readiness a reactor could
//! report. `rusqlite` *is* the protocol, and what this module adds around it is
//! what every other driver gets from its own machinery — § 3's
//! handoff off the core, `rule:core-classes/db-statement-members`'s
//! one-statement-at-a-time rule, § 8's normalised error kinds, and § 7's
//! nesting with the § 13 reset that closes it.
//!
//! # Why every call is a handoff, and what it costs
//!
//! `rule:http-server/a-core-is-never-blocked-on-a-syscall` admits exactly two ways to wait: park on readiness, or run off
//! the core. A `sqlite3_step` that blocks blocks on a file lock and on the disk,
//! neither of which the kernel will report as readiness, so this driver has only
//! the second. [`nvs_host::blocking::run`] takes `FnOnce() -> T + Send +
//! 'static`, and that bound is the whole reason this module's surface differs
//! from every other driver's:
//!
//! - **The handle is an `Arc<Mutex<rusqlite::Connection>>`.** A
//!   `rusqlite::Connection` is `Send` and not `Sync`, so an `Arc` alone will not
//!   cross to a pool thread; the `Mutex` is what makes it `Send`, not a claim
//!   that two callers may hold it. It is never contended — § 4's [`State`] gives
//!   one request the connection at a time — so the cost is one uncontended
//!   atomic per statement.
//! - **Parameters arrive owned**, `Vec<SqliteValue>` where the others take
//!   `&[Option<&[u8]>]` of already-encoded wire bytes. A borrowed slice cannot
//!   be `'static`, and copying one per statement to satisfy the bound would be a
//!   copy nobody asked for: `nvs-stdlib` builds this vector out of the call's own
//!   values, so taking it by value costs nothing at all.
//! - **A buffered statement's rows come back materialized.** `rusqlite`'s `Rows`
//!   borrows the statement, which borrows the connection, so it cannot outlive
//!   the closure that produced it. [`SqliteRows`] therefore holds the whole
//!   result set, which is memory spent — the priority ordering's last item —
//!   to keep every other member one handoff and one answer.
//!
//! # The walk, which is the other side of that bound
//!
//! `rule:core-classes/db-streaming`'s `stream` is the member that may not buffer, so it does the
//! thing the bullet above declined to do for `query`: [`SqliteCursor`] keeps a
//! pool thread for the life of the walk, holding the statement and the cursor
//! stepping it, and answers one owned row per [`SqliteConn::stream_next_row`].
//! Constant memory, on the one backend whose cursor cannot be parked in a field.
//!
//! What that costs is a thread of [`nvs_host::blocking::bound`]'s pool per open
//! walk — `pin`'s own docs own what an exhausted pool does with the next one —
//! and it is released when the last row is read, when the walk is ended, or when
//! the task holding the connection dies. The lock on the connection is held for
//! the same span, which is why filing a deadline mid-walk goes through the
//! thread ([`set_busy_timeout`]) rather than taking it.
//!
//! # What is still § 4's rule and not the file's
//!
//! Materialized rows mean a second statement could physically run while a first
//! result set is unread. It still does not: [`SqliteRows`] borrows the
//! connection for as long as it lives and leaves [`State::Streaming`] set until
//! it is dropped, which is [`crate::tds::TdsRows`]' arrangement and for the reason
//! [`SqliteConn::state`]'s own comment gives — § 4's `LogicError` is a property
//! of the API, not of a socket.
//!
//! # Which statement inserted a row, and what that costs
//!
//! `sqlite3_last_insert_rowid` belongs to the connection and not to the
//! statement: after an insert of row 4, an `update`, a `delete` and a `create
//! table` each answer `4`. That is exactly the `mysqli_insert_id` hazard
//! `Core\Db\Write::lastId`'s reference card promises this class does not have.
//! The counts do not reach it either — `sqlite3_total_changes` moving says rows
//! *changed*, not that any row was inserted, so a fold like [`step`]'s for
//! `affected` still reports the earlier key for an `update` that changed rows.
//!
//! **`sqlite3_update_hook` is what tells an insert from the statements that
//! follow it, and it is paid per changed row.** The hook names the action and
//! the rowid of every row a statement inserts, updates or deletes;
//! [`InsertedRow`] keeps the last insert's, and [`step`] clears it under the
//! connection's own lock before the statement runs. What survives the statement
//! is therefore that statement's key or nothing at all, which is the whole of
//! the promise. A row inserted by a trigger counts as the statement's, and that
//! is the one place this driver's answer and `sqlite3_last_insert_rowid`'s
//! differ: SQLite reverts the connection's value when the trigger ends.
//!
//! **What it spends**: one indirect call and two borrowed names per row a write
//! changes, two relaxed stores per row it inserts, and two atomics per open
//! connection — whether or not the program ever reads `lastId`. Both cheaper
//! answers spend something the priority ordering ranks above latency, which is
//! why a per-row cost is the one taken. Reading the statement's *kind* would
//! mean parsing a dialect this crate does not parse: `WITH … INSERT`, `INSERT
//! OR REPLACE` and an `update` whose trigger inserts each defeat it, and a key
//! belonging to no row is worse than a slow one. `sqlite3_set_last_insert_rowid`
//! as a sentinel around each statement is one call per *statement* rather than
//! per row, and `rusqlite` does not expose it: it is raw FFI through an `unsafe`
//! `Connection::handle`, in the crate whose job is to keep the audit surface of
//! five wire protocols small and under a workspace that forbids `unsafe` rather
//! than denying it. No sentinel is out of reach of a row either, since every
//! `i64` is a legal rowid.

use std::cell::Cell;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use nvs_config::tree::Database;
use nvs_runtime::{Tag, Value};
use rusqlite::types::{ToSqlOutput, ValueRef};

use crate::conn::{
    BlockError, ColumnType, DbErrorKind, Driver, Isolation, ServerError, SqliteConn, State,
    written_value,
};
use crate::pg::{open_transaction, savepoint_name};
use crate::span::QuerySpan;
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
    /// `rule:core-classes/db-capabilities` puts this under `db.connect` exactly as a host is: an
    /// operator wrote it into root-owned configuration, which is the same
    /// authority that granted the capability. A *program-supplied* path is
    /// `open`'s, and § 3 additionally makes that one a path sink needing
    /// `fs.read`/`fs.write` — a rule that lives above this crate, because
    /// nothing here knows which of the two it was handed.
    ///
    /// Resolution against the config file's own directory (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`) has
    /// already happened by the time a block reaches here — `nvs_config::db`'s
    /// `canonicalize` does it at boot, beside the `tls_ca_file` — for the reason
    /// the other drivers' addresses are resolved before they arrive: a driver
    /// that re-resolved a name would be reaching somewhere nobody approved.
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
/// [ADR 0067 § 9](/docs/decisions/0067.md) works around by keying its
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
    /// gets `rule:types/bytes`'s guarantee
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

/// One bound parameter as the storage class SQLite will hold it in —
/// [`crate::encode`]'s, [`crate::mysql::encode`]'s and [`crate::tds::encode`]'s
/// opposite number, and the one that renders no text at all.
///
/// The others answer octets because their protocols carry a parameter as
/// octets. SQLite carries a *value*, so this converts rather than renders and
/// [`SqliteValue`]'s five arms are the whole target — which is also why it
/// answers a bare [`SqliteValue`] where the others answer `Option<Vec<u8>>`:
/// `NULL` is a storage class here rather than the absence of one.
///
/// These arms are decisions and not mappings:
///
/// - **A `bool` is `1`/`0`.** SQLite has no boolean storage class and its own
///   `true` and `false` keywords *are* the integers, so this is the engine's
///   spelling rather than a driver's choice. A column declared `BOOLEAN` reads
///   back as [`ColumnType::Bool`] ([`SqliteColumn::column_type`]), which is the
///   other half of the same rule.
/// - **A `uint` past [`i64::MAX`] is refused.** There is no unsigned storage
///   class — [`SqliteValue::Int`] says so — so such a value has no `INTEGER`
///   form, and the two forms it could take are both wrong: a `REAL` drops the
///   low bits, and a `TEXT` compares as text against every integer already in
///   the column. Refusing is the one answer that does not lose it silently.
/// - **A `NaN` is refused and the two infinities are not.** SQLite stores a
///   bound `NaN` as `NULL` — a value the program did not write, reaching a
///   column that may not even admit it — where `±Infinity` round-trips as
///   `REAL` exactly. This is where this driver parts from
///   [`crate::mysql::encode`] and [`crate::tds::encode`], which refuse all
///   three: those two write a *literal* into text and neither dialect has one
///   for any of them, where this binds a double and two of the three survive it.
/// - **A `decimal` goes out as `TEXT`, and what becomes of it then is the
///   column's.** `rule:types/decimal`'s
///   digits are exact and text is the only arm that keeps them so; a column with
///   `TEXT` affinity holds them exactly, and one with `NUMERIC` affinity — which
///   is what `DECIMAL(10,2)` has — converts them to a `REAL` by SQLite's own
///   affinity rule and rounds. That is the engine's storage model rather than an
///   encoding decided here, and the alternative is refusing `decimal` on this
///   backend outright, which would leave
///   `rule:core-classes/db-column-types`'s `decimal` row with a
///   read half and no write half.
///
/// A `Core\Db\InList` never reaches here for [`crate::encode`]'s reason: § 5's
/// marker has expanded into one bound value per element by the time a statement
/// has its bind list.
///
/// # Errors
///
/// `InvalidInput` for a value with no storage class to take — an array, an
/// object, a closure, a `uint` past [`i64::MAX`] and a `NaN` — where the whole
/// answer is the tag and never the value, for the reason [`crate::tds::encode`]
/// gives.
pub fn encode(value: Value) -> io::Result<SqliteValue> {
    let refused = |why: String| io::Error::new(io::ErrorKind::InvalidInput, why);
    Ok(match value.tag() {
        Some(Tag::Null) => SqliteValue::Null,
        Some(Tag::Bool) => SqliteValue::Int(i64::from(value.as_bool() == Some(true))),
        Some(Tag::Int) => SqliteValue::Int(value.as_int().unwrap_or_default()),
        Some(Tag::Uint) => {
            let held = value.as_uint().unwrap_or_default();
            let narrowed = i64::try_from(held).map_err(|_| {
                refused(format!(
                    "the `uint` {held} is past what a SQLite `INTEGER` holds: there is no unsigned \
                     storage class here, and the two forms it could take — a `REAL` that drops the \
                     low bits and a `TEXT` that compares as text — both lose it silently"
                ))
            })?;
            SqliteValue::Int(narrowed)
        }
        Some(Tag::Float) => {
            let float = value.as_float().unwrap_or_default();
            if float.is_nan() {
                return Err(refused(
                    "a `NaN` binds as SQL `NULL` on SQLite, which would put a value the program \
                     did not write into the column — the two infinities bind as themselves and \
                     are not refused"
                        .to_owned(),
                ));
            }
            SqliteValue::Real(float)
        }
        Some(Tag::Decimal) => SqliteValue::Text(
            value
                .as_decimal()
                .map(|exact| exact.to_string())
                .unwrap_or_default(),
        ),
        // A `string` is UTF-8 by `rule:types/bytes`, so this checks a guarantee rather
        // than converting one. [`SqliteValue::read`] checks the other direction
        // because the *file* may hold anything, and this side may not.
        Some(Tag::Str) => {
            let bytes = value.as_str_bytes().unwrap_or_default();
            let text = std::str::from_utf8(bytes)
                .map_err(|e| refused(format!("a `string` parameter is not UTF-8: {e}")))?;
            SqliteValue::Text(text.to_owned())
        }
        // The one arm SQL Server has no answer for at all: a `BLOB` is an
        // ordinary storage class here, so § 9's `bytes` row needs no gap.
        Some(Tag::Bytes) => SqliteValue::Blob(value.as_bytes().unwrap_or_default().to_vec()),
        _ => {
            return Err(refused(format!(
                "a value of tag {} has no form this driver can bind",
                value.tag_byte()
            )));
        }
    })
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

impl SqliteColumn {
    /// [ADR 0067 § 9](/docs/decisions/0067.md)'s type for this
    /// column, off the *declared* name — the one backend where that is the only
    /// thing to key on.
    ///
    /// The other drivers read a type code the server sent and the value's
    /// encoding follows from it. SQLite has five storage classes and a value
    /// carries its own, so `20260903` in a column declared `date` arrives as an
    /// `INTEGER` and `'2026-09-03'` in the same column arrives as `TEXT`.
    /// Nothing about the *value* says which was meant, which is why § 9 keys
    /// this backend off the schema's word and why turning a cell into a
    /// `Core\Time\Date` throws when it does not parse: the column says what it
    /// is and the value is either that or a mistake. The throw is
    /// `nvs-stdlib`'s, where every driver's is — this half is the description.
    ///
    /// **§ 9's own names are matched first, then SQLite's affinity rules in
    /// their own order.** Affinity has only five answers and none of them is
    /// `DATETIME`, `BOOLEAN`, `UUID` or `DECIMAL`, so it is the fallback rather
    /// than the rule — and where it *does* answer, its answer is kept, quirks
    /// included: a column declared `POINT` contains `INT` and describes as
    /// [`ColumnType::Int`], which is the integer affinity SQLite will really
    /// give it. Order matters inside the first group too: `DATETIME` and
    /// `TIMESTAMP` are checked before `TIME`, and `DATETIME` before `DATE`.
    ///
    /// The answers this map deliberately does not give:
    ///
    /// - **Never [`ColumnType::Uint`].** SQLite has no unsigned storage class,
    ///   and a column declared `UNSIGNED BIG INT` — a real spelling from
    ///   SQLite's own affinity examples — is [`ColumnType::Int`], which is what
    ///   its values are.
    /// - **Never [`ColumnType::Json`].** A column declared `JSON` is
    ///   [`ColumnType::Text`], because SQLite has no JSON type to describe:
    ///   this is MariaDB's case, where the type is a name over a text column,
    ///   and [`ColumnType::Json`]'s own doc settles it for both. § 9's rule that
    ///   JSON is never auto-decoded is untouched either way.
    /// - **An expression is [`ColumnType::Other`]**, the total case, and not the
    ///   `BLOB` affinity SQLite's rule 3 gives a column with no declared type.
    ///   `select 1 + 1` produces an integer, and describing it as bytes would be
    ///   a wrong answer where `Other` is a true one.
    #[must_use]
    pub fn column_type(&self) -> ColumnType {
        let Some(declared) = self.declared.as_deref() else {
            return ColumnType::Other;
        };
        // Folded here rather than at the point it arrives, for the reason
        // `declared`'s own doc gives: SQLite's affinity rules fold case, so one
        // fold in one place is one place for this and the engine to agree.
        let declared = declared.to_ascii_uppercase();
        let has = |needle: &str| declared.contains(needle);

        if has("DATETIME") || has("TIMESTAMP") {
            ColumnType::DateTime
        } else if has("DATE") {
            ColumnType::Date
        } else if has("TIME") {
            ColumnType::Time
        } else if has("BOOL") {
            ColumnType::Bool
        } else if has("UUID") {
            ColumnType::Uuid
        } else if has("DECIMAL") || has("NUMERIC") || has("MONEY") {
            ColumnType::Decimal
        } else if has("INT") {
            ColumnType::Int
        } else if has("CHAR") || has("CLOB") || has("TEXT") || has("JSON") {
            ColumnType::Text
        } else if has("BLOB") {
            ColumnType::Bytes
        } else if has("REAL") || has("FLOA") || has("DOUB") {
            ColumnType::Float
        } else {
            ColumnType::Other
        }
    }
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
    last_insert_id: Option<i64>,
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

    /// How many rows this statement changed: § 4's `execute` answer.
    ///
    /// **This statement's own count and never an earlier one's.**
    /// `sqlite3_changes` reports the last *data-changing* statement on the
    /// connection, so a `SELECT`, a `create table` or a `drop table` leaves it
    /// at a count that belongs to something else — and `Core\Db\Write::affected`
    /// is documented to answer `0` for a statement whose kind has no count of
    /// its own. [`step`] separates the two by whether `sqlite3_total_changes`
    /// moved, which is what makes a statement that changed nothing answer `0`
    /// here rather than repeating a number a program already read.
    #[must_use]
    pub fn affected(&self) -> u64 {
        self.affected
    }

    /// The key of the last row this statement inserted, and `None` where it
    /// inserted none.
    ///
    /// This module's doc § *Which statement inserted a row* owns where the
    /// answer comes from and what it costs. It is the statement's own key and
    /// never the connection's, which is the whole of what
    /// `Core\Db\Write::lastId` promises.
    #[must_use]
    pub fn last_insert_id(&self) -> Option<i64> {
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

/// One question for the thread a parked walk is holding.
#[derive(Debug, Clone, Copy)]
enum Step {
    /// Prepare, bind and describe. Asked once, by [`SqliteConn::stream`].
    Open,
    /// Step the cursor once.
    Next,
    /// File `rule:core-classes/db-statement-members`'s deadline as `sqlite3_busy_timeout`, which on a
    /// connection a walk is holding only this thread can do —
    /// [`set_busy_timeout`] owns why.
    Bound(std::time::Duration),
    /// Release the statement and the connection, walk unfinished.
    End,
}

/// What the thread answers a [`Step`] with.
#[derive(Debug)]
enum Answered {
    /// The description, in answer to [`Step::Open`].
    Opened(Vec<SqliteColumn>),
    /// One row.
    Row(Vec<SqliteValue>),
    /// The deadline is filed, and the walk goes on.
    Bound,
    /// The walk is over and the thread has already let go of everything: the
    /// last row has been read, or [`Step::End`] asked it to stop.
    Done,
    /// SQLite refused, and the walk is over on the same terms as [`Self::Done`].
    Failed(io::Error),
}

/// `rule:core-classes/a-stream-parks-its-read-on-the-connection`'s read state for this driver: the thread walking the
/// statement, and what the statement described.
///
/// **The read state is a thread because it cannot be anything else.** A
/// `rusqlite::Statement` borrows the connection and its `Rows` borrows the
/// statement, so neither can be parked in a field the way the wire drivers park
/// a half-read message — this module's own doc has the lifetimes. What crosses
/// back to the core is one owned row per step, and what stays on the thread is
/// everything that borrows: [`nvs_host::blocking::pin`] is the primitive that
/// allows it, and its docs own what an exhausted pool does with the next walk.
///
/// The thread is released when the last row is read, when [`SqliteConn::end_stream`]
/// asks it to stop, and — because the handle lives in this struct — when the
/// task that opened the walk dies with the connection still holding it. There is
/// no fourth path and nothing to remember.
#[derive(Debug)]
pub(crate) struct SqliteCursor {
    /// The thread, until the walk ends. `None` for a walk read to its end or
    /// abandoned, whose description and span are still worth reading.
    walk: Option<nvs_host::blocking::Pinned<Step, Answered>>,
    /// What the statement described, answered to [`Step::Open`] and handed back
    /// to every later call that decodes a row against it.
    columns: Vec<SqliteColumn>,
    /// `rule:observability/a-query-is-a-trace-event`'s event for the walk.
    span: QuerySpan,
}

impl SqliteCursor {
    /// What the statement described.
    pub(crate) fn columns(&self) -> &[SqliteColumn] {
        &self.columns
    }

    /// The walk's trace event.
    pub(crate) fn span(&self) -> &QuerySpan {
        &self.span
    }

    /// Names the `[db.<name>]` block this walk is running on.
    pub(crate) fn name_connection(&mut self, connection: &str) {
        self.span.name(connection);
    }
}

/// The thread ended without answering, which is a pool shutting down or a job
/// that unwound where nothing here can.
///
/// It is not a server error and carries no `rule:core-classes/db-error` kind: SQLite said
/// nothing, and a caller reading a kind off this would be reading one this
/// runtime invented.
fn lost_walk() -> io::Error {
    io::Error::other(
        "the SQLite walk's thread ended without answering: the statement is gone and the rows \
         cannot be read, so re-run it — with `query`, if this request must not meet it again",
    )
}

/// Opens the file, and answers the connection `rule:core-classes/db-statement-members`'s statements run on.
///
/// The open itself goes off the core: creating or reading a database header is
/// a filesystem call, which is `rule:http-server/a-core-is-never-blocked-on-a-syscall`'s first named example.
///
/// What the open sets on the way out, of which only the first is a number from
/// the block:
///
/// - **§ 1's statement cache is `rusqlite`'s own**, sized here. That crate keeps
///   an LRU of `sqlite3_stmt` handles keyed on the SQL text, which is exactly
///   what § 1 specifies, so this driver writes no cache of its own — the one
///   place in this crate where the borrowed half covers a section outright.
/// - **`PRAGMA foreign_keys = ON`**, which SQLite leaves off and PDO leaves off
///   after it. § 8 declares `ForeignKeyViolation` as a kind *every* driver
///   normalises onto, and with the pragma off that condition cannot arise at
///   all: the same schema and the same write would refuse on every other driver
///   and silently corrupt the reference here. Correctness of semantics is
///   priority 2 and the compatibility this costs is with a PHP default that is
///   a known footgun, so it is not configurable to the unsafe value — § 3's own
///   defaults are settled on the same footing.
/// - **No busy timeout.** A lock contention answers `SQLITE_BUSY` at once, which
///   § 8 normalises to `Deadlock`, which is one of the kinds § 7's `retries`
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
    let inserted = Arc::new(InsertedRow::default());
    let filling = Arc::clone(&inserted);

    let handle = nvs_host::blocking::run(move || -> rusqlite::Result<rusqlite::Connection> {
        let connection = rusqlite::Connection::open(path)?;
        connection.set_prepared_statement_cache_capacity(capacity);
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        connection.update_hook(Some(
            move |action, _database: &str, _table: &str, rowid: i64| {
                if action == rusqlite::hooks::Action::SQLITE_INSERT {
                    filling.record(rowid);
                }
            },
        ))?;
        Ok(connection)
    })
    .map_err(server_error)?;

    Ok(SqliteConn {
        handle: Arc::new(Mutex::new(handle)),
        inserted,
        state: Cell::new(State::Idle),
        depth: Cell::new(0),
        time_zone: target.time_zone,
        reading: None,
    })
}

/// The key of the last row a statement inserted, as `sqlite3_update_hook`
/// reported it.
///
/// This module's doc § *Which statement inserted a row* is why the hook and not
/// one of the two cheaper answers. What is here is the state it fills: one flag
/// for whether this statement inserted anything at all, and the rowid of the
/// last row it did.
///
/// **Every access is `Relaxed`, and the connection's own `Mutex` is what orders
/// them.** SQLite calls the hook from inside `sqlite3_step`, on the same pool
/// thread that is running the statement, so [`step`]'s clear, the hook's stores
/// and [`step`]'s read are in program order on one thread; two statements on
/// one connection are ordered by the lock [`step`] holds across all three. A
/// stronger ordering would buy nothing and would be paid for per changed row.
#[derive(Debug, Default)]
pub struct InsertedRow {
    seen: AtomicBool,
    rowid: AtomicI64,
}

impl InsertedRow {
    /// Forget the previous statement's key. Called with the connection locked
    /// and before the statement runs, so nothing can fill it in between.
    fn clear(&self) {
        self.seen.store(false, Ordering::Relaxed);
    }

    /// One inserted row, from the hook.
    fn record(&self, rowid: i64) {
        self.rowid.store(rowid, Ordering::Relaxed);
        self.seen.store(true, Ordering::Relaxed);
    }

    /// The key of the last row the statement that just ran inserted, or `None`
    /// where it inserted none.
    fn taken(&self) -> Option<i64> {
        self.seen
            .load(Ordering::Relaxed)
            .then(|| self.rowid.load(Ordering::Relaxed))
    }
}

/// The version of the SQLite this binary is linked against, which is what
/// `Core\Db\Connection::serverVersion` answers on this driver —
/// [ADR 0187 § 2](/docs/decisions/0187.md).
///
/// A property of the binary and not of a connection: there is no server here
/// and no handshake, so nothing about it is kept per open database.
pub(crate) fn library_version() -> &'static str {
    rusqlite::version()
}

impl SqliteConn {
    /// [ADR 0067 § 9](/docs/decisions/0067.md)'s declared zone, in
    /// seconds east of UTC.
    ///
    /// Public where the other drivers keep theirs private, because they
    /// decode a zone-less column inside their own codec and this one has no
    /// codec to decode inside: a [`SqliteValue::Int`] out of a column declared
    /// `datetime` is still an integer here, and whoever turns it into a
    /// `Core\Time\DateTime` needs the zone it was written in.
    #[must_use]
    pub fn time_zone(&self) -> i32 {
        self.time_zone
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s statement: prepared
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
        let inserted = Arc::clone(&self.inserted);
        let owned = sql.to_owned();
        // Read on this thread, because the pool thread that steps the statement
        // is armed with no ceiling of its own.
        let headroom = nvs_runtime::budget::headroom();
        let read =
            nvs_host::blocking::run(move || step(&handle, &inserted, &owned, &params, headroom));

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
                // The rows were freed on the pool thread, so this balance never
                // held them. Asking for them here records the refusal on the
                // thread whose poll reports the request's breach.
                if let Some(Overran(held)) = e.get_ref().and_then(|why| why.downcast_ref()) {
                    let _ = nvs_runtime::budget::affords(*held);
                }
                Err(e)
            }
        }
    }

    /// Runs one statement and leaves its answer open, **borrowing nothing**:
    /// the walk is parked on this connection and the rows come off it one
    /// [`Self::stream_next_row`] at a time.
    ///
    /// The driver half of `rule:core-classes/db-streaming`'s `stream`, and the one driver where
    /// it is a thread rather than a message boundary — [`SqliteCursor`] owns why
    /// the statement cannot be parked in a field, and what the thread costs. The
    /// answer is what the result set described, empty for a statement that
    /// described none, and [`Self::stream_columns`] hands the same slice back to
    /// the later calls that decode against it.
    ///
    /// **Constant memory, which is what `query` on this driver cannot offer.**
    /// One row crosses back per step and nothing accumulates, so a walk over a
    /// large table holds a row where [`SqliteRows`] holds the table.
    ///
    /// A second statement is refused while this one is open, which is
    /// `rule:core-classes/db-streaming`'s `LogicError` read off [`State::Streaming`] rather than
    /// off a lifetime. [`Self::end_stream`] is what a program that walks away
    /// from the rows owes; a walk read to its end needs no call at all, and a
    /// connection dropped mid-walk releases the thread with everything else.
    ///
    /// # Errors
    ///
    /// As [`SqliteConn::query`], plus the walk whose thread ended without
    /// answering — a pool shutting down under a request, which is not a server
    /// error and carries no § 8 kind.
    pub fn stream(&mut self, sql: &str, params: Vec<SqliteValue>) -> io::Result<&[SqliteColumn]> {
        if !self.state.get().may_start_statement() {
            return Err(busy(self.state.get()));
        }
        let span = QuerySpan::opened(Driver::Sqlite, sql);
        self.state.set(State::Executing);

        let handle = Arc::clone(&self.handle);
        let owned = sql.to_owned();
        let walking = nvs_host::blocking::pin(move |asked| walk(&handle, &owned, &params, asked));

        match walking.ask(Step::Open) {
            Some(Answered::Opened(columns)) => {
                self.state.set(State::Streaming);
                let reading = self.reading.insert(SqliteCursor {
                    walk: Some(walking),
                    columns,
                    span,
                });
                Ok(reading.columns())
            }
            // The failing paths all drop the handle on the way out, which is what
            // releases the thread: a statement SQLite refused never reached a
            // row, so there is nothing to drain and nothing to park. This
            // driver's failure path returns to `Idle` for [`SqliteConn::query`]'s
            // reason.
            Some(Answered::Failed(refused)) => {
                self.state.set(State::Idle);
                Err(refused)
            }
            Some(_) | None => {
                self.state.set(State::Idle);
                Err(lost_walk())
            }
        }
    }

    /// What the parked walk's statement described, or `None` for a connection
    /// that has not streamed since its last reset.
    ///
    /// It outlives the rows on purpose: a value is measured by the column it
    /// belongs to, and § 9's decode of it happens after the step that produced
    /// it.
    #[must_use]
    pub fn stream_columns(&self) -> Option<&[SqliteColumn]> {
        Some(self.reading.as_ref()?.columns())
    }

    /// The next row of the parked walk, or `None` once it has ended — and `None`
    /// too for a connection with no walk parked on it at all.
    ///
    /// Ending it releases the thread and returns the connection to
    /// [`State::Idle`], exactly as dropping [`SqliteRows`] does. The description
    /// and the span stay parked until the next [`Self::stream`] replaces them or
    /// [`Self::end_stream`] drops them.
    ///
    /// # Errors
    ///
    /// [`server_error`] for anything SQLite refused mid-walk, `InvalidData` for
    /// a `TEXT` cell that is not UTF-8 ([`SqliteValue::read`]), and
    /// [`lost_walk`] for a thread that ended without answering. Every one of them
    /// ends the walk: the thread has already let go of the statement by the time
    /// the refusal arrives.
    pub fn stream_next_row(&mut self) -> io::Result<Option<Vec<SqliteValue>>> {
        let Some(reading) = self.reading.as_mut() else {
            return Ok(None);
        };
        let answer = match reading.walk.as_ref() {
            Some(walk) => walk.ask(Step::Next),
            None => return Ok(None),
        };

        match answer {
            Some(Answered::Row(cells)) => {
                reading.span.row();
                Ok(Some(cells))
            }
            Some(Answered::Done) => {
                reading.walk = None;
                reading.span.finished(None);
                self.state.set(State::Idle);
                Ok(None)
            }
            Some(Answered::Failed(refused)) => {
                reading.walk = None;
                reading.span.finished(None);
                self.state.set(State::Idle);
                Err(refused)
            }
            Some(Answered::Opened(_) | Answered::Bound) | None => {
                reading.walk = None;
                reading.span.finished(None);
                self.state.set(State::Idle);
                Err(lost_walk())
            }
        }
    }

    /// [ADR 0067 § 11](/docs/decisions/0067.md)'s trace event for the
    /// parked walk, or `None` where there is none.
    #[must_use]
    pub fn stream_span(&self) -> Option<&QuerySpan> {
        Some(self.reading.as_ref()?.span())
    }

    /// Names the `[db.<name>]` block the parked walk is running on, and does
    /// nothing where there is no walk — [`crate::MySqlRows::name_connection`]
    /// owns why the driver cannot work the name out for itself.
    pub fn name_stream_connection(&mut self, connection: &str) {
        if let Some(reading) = self.reading.as_mut() {
            reading.name_connection(connection);
        }
    }

    /// Abandons the parked walk: tells the thread to let go, waits for it to say
    /// it has, and forgets the walk.
    ///
    /// **The wait is the point.** The thread holds the statement and the lock on
    /// the connection, so a call that only dropped the handle would leave § 13's
    /// reset — or the next statement — racing a release that has not happened
    /// yet. [`Step::End`] is answered *after* the thread has let go of both, so
    /// a connection is free the instant this returns.
    pub fn end_stream(&mut self) {
        if let Some(reading) = self.reading.as_mut()
            && let Some(walk) = reading.walk.take()
        {
            let _ = walk.ask(Step::End);
            reading.span.finished(None);
        }
        self.reading = None;
        if self.state.get() == State::Streaming {
            self.state.set(State::Idle);
        }
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `executeMany`: one
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

    /// [ADR 0067 § 7](/docs/decisions/0067.md)'s `BEGIN`, or the
    /// `SAVEPOINT` a nested `transaction()` is.
    ///
    /// The nesting, the names and the depth accounting are
    /// [`crate::PgConn::begin`]'s — one rule for every backend that has
    /// savepoints, which is all of them — and what is this driver's own is the
    /// two options.
    ///
    /// **Every one of § 7's isolation levels is accepted, and none of them
    /// renders to anything.** SQLite is always serializable, so a level asked
    /// for here is delivered *at least* as strongly as it was asked for, which
    /// [`Isolation`]'s own doc makes explicitly not the case § 7 says to throw
    /// over: nothing a program can observe is weakened by a stronger guarantee.
    /// The refusal § 7 requires is for a driver that would quietly run the
    /// closure at a *weaker* level, and this backend has no weaker level to run
    /// it at. A `WAL` database's readers not blocking on a writer is the same
    /// serializable history reached without the lock, not a second level.
    ///
    /// **`read_only` is refused at any depth**, as it is on SQL Server and for
    /// the sharper reason. SQLite has no read-only transaction: read-only-ness
    /// is a property of how the *database* was opened, and the only per-session
    /// spelling is `PRAGMA query_only`, which would be session state this
    /// connection carried between transactions — precisely what § 13 says this
    /// backend has none of, and the claim its whole reset rests on. Accepting
    /// the option and opening an ordinary writable transaction is the one
    /// failure mode a `readOnly` exists to prevent.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a `read_only` at any depth and for a nested call
    /// asking for an isolation level, otherwise as [`simple_command`]. The depth
    /// moves only after a command SQLite accepted, so a refused `BEGIN` leaves a
    /// connection that is still in no transaction.
    pub fn begin(&self, isolation: Option<Isolation>, read_only: bool) -> io::Result<QuerySpan> {
        if read_only {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a transaction asked to be read-only and SQLite has no read-only transaction: \
                 drop the option, or take the guarantee where this backend really offers one — a \
                 `[db.<name>]` block of its own naming the same file, opened read-only",
            ));
        }

        let open = self.depth.get();
        let command = if open == 0 {
            String::from("BEGIN")
        } else if isolation.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "a transaction nested {open} deep asked for its own isolation level, and \
                     SQLite settles one for the whole transaction: ask for it on the outermost \
                     `transaction()`, or give this one a `{{shared: false}}` connection of its own"
                ),
            ));
        } else {
            format!("SAVEPOINT {}", savepoint_name(open))
        };

        let span = simple_command(self, &command)?;
        self.depth.set(open + 1);
        Ok(span)
    }

    /// § 7's `BEGIN IMMEDIATE`: the write lock taken before the first
    /// statement, for a transaction that reads a row and writes it back.
    ///
    /// `rule:concurrency/claiming-is-one-statement` names this backend's claim
    /// as an immediate transaction, and this is what makes one available.
    /// [`SqliteConn::begin`]'s bare `BEGIN` is `DEFERRED`: it takes no lock
    /// until its first statement, so a transaction that reads and then writes
    /// holds a shared lock and asks to upgrade it — and SQLite answers an
    /// upgrade it cannot grant with `SQLITE_BUSY` *without honouring the busy
    /// timeout*, because backing a reader off a lock it already holds would
    /// break that reader's own snapshot. Taking the write lock up front removes
    /// the upgrade, and the wait that replaces it is one
    /// [`set_busy_timeout`]'s bound really does cover.
    ///
    /// **It is its own entry point rather than a sixth [`Isolation`] case.**
    /// That enum is the SQL standard's five levels and none of them means "take
    /// the write lock now": SQLite's isolation is serializable either way, and
    /// what this asks is *when* the lock is acquired. So `Core\Db::transaction`
    /// opens the transaction it always has for every caller that does not ask,
    /// and nothing on the language surface moves.
    ///
    /// **An immediate transaction is an outermost one**, so asking for one
    /// inside an open transaction is refused rather than quietly turned into a
    /// `SAVEPOINT`. A savepoint holds whatever lock the transaction around it
    /// took, which makes the one thing the caller asked for the one thing a
    /// nested level cannot give it.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a connection already in a transaction, otherwise as
    /// [`simple_command`] — a `SQLITE_BUSY` here is another connection's write
    /// lock, which § 8 normalises to the `Deadlock` kind § 7's `{retries: n}`
    /// retries. The depth moves only after a command SQLite accepted, so a
    /// refused `BEGIN IMMEDIATE` leaves a connection that is still in no
    /// transaction.
    pub fn begin_immediate(&self) -> io::Result<QuerySpan> {
        let open = self.depth.get();
        if open > 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "an immediate transaction was asked for inside one already {open} deep, and an \
                     immediate transaction is an outermost one: SQLite takes the write lock where \
                     the outermost transaction begins, so ask for it there — or give this level a \
                     `{{shared: false}}` connection of its own"
                ),
            ));
        }

        let span = simple_command(self, "BEGIN IMMEDIATE")?;
        // One counter rule for every level this driver opens: it moves after the
        // command and only for a command SQLite accepted. `open` is 0 past the
        // guard above, so this is `begin`'s `open + 1` with the addition done.
        self.depth.set(1);
        Ok(span)
    }

    /// How many transaction levels are open on this connection — 0 outside one,
    /// 1 inside an outermost `transaction()`, deeper inside a nested one.
    ///
    /// [`crate::PgConn::depth`] owns why this is public at all: § 7 retries an
    /// outermost transaction only, and the caller cannot tell the two apart.
    #[must_use]
    pub fn depth(&self) -> u32 {
        self.depth.get()
    }

    /// § 7's `COMMIT`, or the `RELEASE SAVEPOINT` closing a nested one.
    ///
    /// **A refused outermost commit leaves the depth where it was**, which is
    /// the one place this driver's accounting is not
    /// [`crate::PgConn::commit`]'s. PostgreSQL has already rolled the
    /// transaction back by the time it refuses a `COMMIT`, so the level really
    /// is gone there; SQLite refuses one with `SQLITE_BUSY` and leaves the
    /// transaction open and retryable. Zeroing the count here would send the
    /// next command as a `BEGIN` inside a transaction SQLite still holds, and
    /// § 7's `{retries: n}` is the caller that would do it.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a connection in no transaction, otherwise as
    /// [`simple_command`].
    pub fn commit(&self) -> io::Result<QuerySpan> {
        let open = open_transaction(&self.depth, "commit")?;
        let command = if open == 1 {
            String::from("COMMIT")
        } else {
            format!("RELEASE SAVEPOINT {}", savepoint_name(open - 1))
        };

        let span = simple_command(self, &command)?;
        self.depth.set(open - 1);
        Ok(span)
    }

    /// § 7's `ROLLBACK`, or the `ROLLBACK TO SAVEPOINT` undoing a nested one.
    ///
    /// The nested form releases the savepoint it returned to in the same batch,
    /// for [`crate::PgConn::roll_back`]'s reason: `ROLLBACK TO` leaves the
    /// savepoint established, and a loop that opens and abandons a nested
    /// transaction per iteration would otherwise leave one name per iteration
    /// alive for as long as the outer transaction runs.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a connection in no transaction, otherwise as
    /// [`simple_command`]. A refused rollback leaves the depth where it was: the
    /// level above will roll back over this one anyway.
    pub fn roll_back(&self) -> io::Result<QuerySpan> {
        let open = open_transaction(&self.depth, "roll back")?;
        let command = if open == 1 {
            String::from("ROLLBACK")
        } else {
            let name = savepoint_name(open - 1);
            format!("ROLLBACK TO SAVEPOINT {name}; RELEASE SAVEPOINT {name};")
        };

        let span = simple_command(self, &command)?;
        self.depth.set(open - 1);
        Ok(span)
    }

    /// [ADR 0067 § 13](/docs/decisions/0067.md)'s reset, and the
    /// connection back only if it worked.
    ///
    /// **Rolling back an open transaction is the whole reset**, which is § 13's
    /// own sentence for this backend and not a shortcut taken here: a
    /// `rusqlite::Connection` is a file handle, and every other item on § 13's
    /// property list — a session variable, a `SET ROLE`, an advisory lock, a
    /// listener, a temporary table outside the transaction — is something SQLite
    /// has no session to hold. § 1's statement cache is `rusqlite`'s own and
    /// survives, which is what pooling exists to preserve. Nothing this driver
    /// sets per transaction is session-scoped, and
    /// [`SqliteConn::begin`]'s refusal of `read_only` is what keeps that true.
    ///
    /// **Whether a transaction is open is asked of the engine, not of the
    /// count.** `sqlite3_get_autocommit` is the connection's own answer, so a
    /// reset is provable rather than inferred from a depth this driver
    /// maintains — and it also covers the transaction a caller's own `BEGIN` in
    /// § 4 statement text opened, which the depth never saw.
    ///
    /// **`self` by value is the enforcement**, exactly as on
    /// [`crate::PgConn::reset`]: a connection that cannot be proven clean must
    /// not be returnable to the pool, and a `&mut self` signature would leave
    /// the caller holding one it has to remember not to reuse.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a connection that is not at rest — § 4's rows are
    /// still lent out, and a reset is only meaningful between statements — or
    /// [`server_error`] for a rollback SQLite refused. The connection is
    /// dropped either way.
    pub fn reset(mut self) -> io::Result<SqliteConn> {
        if !self.state.get().is_poolable() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "a connection that is {:?} was asked to reset, and § 13's reset is only \
                     meaningful between statements",
                    self.state.get()
                ),
            ));
        }

        // Before the rollback, which takes the lock a walk would still be
        // holding: a connection that streamed carries its description and its
        // span until something drops them, and a reset is where a request's
        // leavings stop being the next request's. A walk still open cannot reach
        // here — `is_poolable` refused it above — so this is the finished one.
        self.end_stream();

        let handle = Arc::clone(&self.handle);
        nvs_host::blocking::run(move || {
            let guard = lock(&handle);
            if guard.is_autocommit() {
                return Ok(());
            }
            guard.execute_batch("ROLLBACK;")
        })
        .map_err(server_error)?;

        self.depth.set(0);
        Ok(self)
    }
}

/// § 7's commands and § 13's rollback, run off the core as one batch.
///
/// None of them takes a parameter or answers a row, so `execute_batch` is the
/// whole call: it is also the only `rusqlite` entry point that will run the two
/// statements [`SqliteConn::roll_back`]'s nested form sends, and it deliberately
/// does not go through § 1's cache — a `COMMIT` is worth no entry in an LRU of
/// prepared statements.
///
/// **It answers the span of the command it sent**, which is `rule:observability/a-query-is-a-trace-event`'s
/// event for a statement that lends no [`SqliteRows`] out, for
/// [`crate::PgConn::begin`]'s reason: which command a level gets is the depth's
/// answer, so the text only exists down here.
///
/// A refused batch returns the connection to [`State::Idle`] rather than
/// poisoning it, as [`SqliteConn::query`] does and for the same reason — there
/// is no half-written message for this driver to be lost in.
///
/// # Errors
///
/// [`busy`] when the connection is not at rest, [`server_error`] for anything
/// SQLite refused.
fn simple_command(conn: &SqliteConn, sql: &str) -> io::Result<QuerySpan> {
    if !conn.state.get().may_start_statement() {
        return Err(busy(conn.state.get()));
    }

    let mut span = QuerySpan::opened(Driver::Sqlite, sql);
    conn.state.set(State::Executing);

    let handle = Arc::clone(&conn.handle);
    let owned = sql.to_owned();
    let ran = nvs_host::blocking::run(move || lock(&handle).execute_batch(&owned));

    conn.state.set(State::Idle);
    ran.map_err(server_error)?;
    span.finished(None);
    Ok(span)
}

/// What one stepped statement produced, owned and off the pool thread.
struct Read {
    columns: Vec<SqliteColumn>,
    rows: Vec<Vec<SqliteValue>>,
    affected: u64,
    last_insert_id: Option<i64>,
}

/// A stepped result that grew past the request's memory headroom: how many
/// bytes the pool thread held when [`step`] stopped.
#[derive(Debug)]
struct Overran(usize);

impl std::fmt::Display for Overran {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the result set grew to {} bytes, past the request's memory limit",
            self.0
        )
    }
}

impl std::error::Error for Overran {}

/// Prepare, bind, step to exhaustion, and hand back what is owned.
///
/// Free rather than a method for the reason the playbook gives about the other
/// drivers' `start_statement`: the sequencing is the part worth testing, and a
/// method on [`SqliteConn`] can only be reached through a [`SqliteTarget`] and a
/// file. It happens that SQLite's file may be `:memory:`, so this one *could*
/// have been a method — the shape is shared anyway, because a reader comparing
/// the drivers should not have to work out that one of them is different for a
/// reason that is not about the protocol.
///
/// **`headroom` is the caller's, because this thread has no ceiling.** It is
/// what the request may still allocate ([`nvs_runtime::budget::headroom`]), and
/// the rows are measured against it after each one is read. A result that
/// grows past it stops with [`Overran`], so the pool thread holds at most one
/// row more than the request could. The check is two thread-local reads per
/// row. `None` steps to the end.
fn step(
    handle: &Mutex<rusqlite::Connection>,
    inserted: &InsertedRow,
    sql: &str,
    params: &[SqliteValue],
    headroom: Option<usize>,
) -> io::Result<Read> {
    let guard = lock(handle);
    let mut statement = guard.prepare_cached(sql).map_err(server_error)?;
    // Read before the statement runs, because what `sqlite3_changes` answers
    // afterwards is only this statement's where this statement changed rows.
    let before = guard.total_changes();
    // The same reasoning one step further: the hook fills this while the
    // statement runs, so what is left over from the last one is cleared here,
    // inside the lock that made this statement the connection's only one.
    inserted.clear();

    let columns = describe(&statement);
    let width = columns.len();
    let mut rows = Vec::new();
    let mut cursor = statement
        .query(rusqlite::params_from_iter(params))
        .map_err(server_error)?;
    let start = nvs_runtime::budget::live_bytes();
    while let Some(cells) = read_row(&mut cursor, width)? {
        rows.push(cells);
        if let Some(room) = headroom {
            let held =
                usize::try_from(nvs_runtime::budget::live_bytes().wrapping_sub(start)).unwrap_or(0);
            if held > room {
                return Err(io::Error::new(io::ErrorKind::OutOfMemory, Overran(held)));
            }
        }
    }
    drop(cursor);
    drop(statement);

    Ok(Read {
        columns,
        rows,
        // `sqlite3_changes` is the last *data-changing* statement's count, so a
        // `create table` or a `select` leaves it reporting an earlier
        // statement's rows as this one's. `sqlite3_total_changes` moving is what
        // says this statement changed any, and a statement that changed none
        // answers `0` — the same answer § 4's `affected` gives on every other
        // driver for a command whose tag carries no count.
        affected: if guard.total_changes() == before {
            0
        } else {
            guard.changes()
        },
        last_insert_id: inserted.taken(),
    })
}

/// One walk, from the thread that owns it: prepare, describe, and then a row per
/// question until the handle goes away.
///
/// Everything that borrows lives in this frame — the lock on the connection, the
/// statement it prepared, the cursor stepping it — which is the whole reason a
/// walk is a thread on this driver and a field on every other one
/// ([`SqliteCursor`]).
///
/// **It lets go before it answers the last question.** The cursor, the statement
/// and the lock are dropped and only then is [`Answered::Done`] sent, so the
/// step that ends a walk returns to a caller whose connection is already free —
/// which is what lets § 13's reset run straight after one. Ending because the
/// handle was dropped has nothing to answer and lets go on the way out.
fn walk(
    handle: &Mutex<rusqlite::Connection>,
    sql: &str,
    params: &[SqliteValue],
    mut asked: nvs_host::blocking::Asked<Step, Answered>,
) {
    // The lock is taken only once a question has arrived, so a handle dropped
    // before the first step never touches the connection at all.
    let Some(open) = asked.next_question() else {
        return;
    };
    let guard = lock(handle);
    let mut statement = match guard.prepare_cached(sql) {
        Ok(statement) => statement,
        Err(refused) => {
            open.answer(Answered::Failed(server_error(refused)));
            return;
        }
    };
    let columns = describe(&statement);
    let width = columns.len();
    let mut cursor = match statement.query(rusqlite::params_from_iter(params)) {
        Ok(cursor) => cursor,
        Err(refused) => {
            open.answer(Answered::Failed(server_error(refused)));
            return;
        }
    };
    open.answer(Answered::Opened(columns));

    let ending = loop {
        let Some(step) = asked.next_question() else {
            break None;
        };
        // Copied out first: the arms below consume the request to answer it, and
        // a question read through a borrow of it would still be borrowing it.
        let question = *step.question();
        match question {
            Step::Next => match read_row(&mut cursor, width) {
                Ok(Some(cells)) => step.answer(Answered::Row(cells)),
                Ok(None) => break Some((step, Answered::Done)),
                Err(refused) => break Some((step, Answered::Failed(refused))),
            },
            Step::Bound(waiting) => match guard.busy_timeout(waiting) {
                Ok(()) => step.answer(Answered::Bound),
                Err(refused) => break Some((step, Answered::Failed(server_error(refused)))),
            },
            Step::End => break Some((step, Answered::Done)),
            // One walk is opened once, by [`SqliteConn::stream`], and it is the
            // question that got this frame running. A second is this crate's
            // paste error rather than anything a program can reach.
            Step::Open => {
                break Some((
                    step,
                    Answered::Failed(io::Error::other(
                        "a SQLite walk that is already open was asked to open again",
                    )),
                ));
            }
        }
    };

    drop(cursor);
    drop(statement);
    drop(guard);
    if let Some((step, answer)) = ending {
        step.answer(answer);
    }
}

/// What a prepared statement says its result set is, one entry per column.
///
/// Read before the first step, because stepping borrows the statement mutably
/// for as long as the rows live. `sqlite3_column_count` and the declared types
/// are available the moment the statement is prepared, so nothing is lost by
/// reading them here.
fn describe(statement: &rusqlite::Statement<'_>) -> Vec<SqliteColumn> {
    statement
        .columns()
        .iter()
        .map(|column| SqliteColumn {
            name: column.name().to_owned(),
            declared: column.decl_type().map(str::to_owned),
        })
        .collect()
}

/// One stepped row as owned values, or `None` at the end of the set.
///
/// The one place a cell crosses from `rusqlite`'s borrowed `ValueRef` into
/// something that can leave the thread, which is what both readers here need:
/// [`step`] to fill its `Vec`, and [`walk`] to answer one question.
///
/// # Errors
///
/// [`server_error`] for anything SQLite refused mid-set, and `InvalidData` for a
/// `TEXT` cell that is not UTF-8 ([`SqliteValue::read`]).
fn read_row(cursor: &mut rusqlite::Rows<'_>, width: usize) -> io::Result<Option<Vec<SqliteValue>>> {
    let Some(row) = cursor.next().map_err(server_error)? else {
        return Ok(None);
    };
    let mut cells = Vec::with_capacity(width);
    for index in 0..width {
        cells.push(SqliteValue::read(
            row.get_ref(index).map_err(server_error)?,
        )?);
    }
    Ok(Some(cells))
}

/// `rule:core-classes/db-statement-members`'s statement deadline, spelled as the only wait this backend
/// takes.
///
/// The other drivers file the instant on the socket, because a statement
/// there is a conversation and every leg of it is a read that can hang. There is
/// no socket here: once this connection has the database the statement runs to
/// completion on `nvs-host`'s blocking pool, and the one thing it *waits* for
/// first is the write lock another connection is holding. `sqlite3_busy_timeout`
/// is exactly the bound on that wait, so the option means the same thing it
/// means everywhere else — the call answers, one way or the other, rather than
/// blocking a request-serving core until someone else commits.
///
/// `None` restores SQLite's own default, which is to answer `SQLITE_BUSY` at
/// once rather than to wait unboundedly: an unbounded lock wait is not a shape
/// this backend has, and it is not one `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` would allow if it did.
///
/// # Errors
///
/// Whatever `sqlite3_busy_timeout` reported.
pub(crate) fn set_busy_timeout(
    conn: &crate::conn::SqliteConn,
    at: Option<std::time::Instant>,
) -> io::Result<()> {
    let waiting = at.map_or(std::time::Duration::ZERO, |deadline| {
        deadline.saturating_duration_since(std::time::Instant::now())
    });

    // A walk holds the lock on the connection for its whole life, so this is the
    // one call on this driver that cannot simply take it: a core waiting here
    // for a thread that is waiting for this core to ask it for the next row is a
    // wedge, not a wait. The walk's own thread files it instead, which is also
    // the only way the bound reaches the steps it is meant to bound.
    if let Some(walk) = conn
        .reading
        .as_ref()
        .and_then(|reading| reading.walk.as_ref())
    {
        return match walk.ask(Step::Bound(waiting)) {
            Some(Answered::Bound) => Ok(()),
            Some(Answered::Failed(refused)) => Err(refused),
            Some(_) | None => Err(lost_walk()),
        };
    }

    lock(&conn.handle)
        .busy_timeout(waiting)
        .map_err(server_error)
}

/// The connection's handle, with a poisoned lock read through rather than
/// panicked on.
///
/// A poisoned mutex here means a pool thread unwound while holding the
/// connection, and [`nvs_host::blocking::run`] has already carried that panic
/// back to the task that owns it — where `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`'s containment boundary fails
/// one request. Panicking a second time on the next acquire would fail an
/// unrelated one.
fn lock(handle: &Mutex<rusqlite::Connection>) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
    handle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `rule:core-classes/db-statement-members`'s refusal, when a statement is asked for on a connection that is
/// not at rest.
///
/// The wording names both of § 4's fixes, as `pg.rs`'s `second_statement` does
/// and for the same reason: a developer who reaches this has either kept a
/// result set alive or shared a connection across two flows, and the message is
/// the only place that says which two things to look at. `nvs-stdlib` re-words
/// it as the `LogicError` § 4 specifies.
///
/// **`InvalidInput` is what carries the class across the crate boundary**, and
/// it is the same kind `second_statement` uses. This crate builds no fault —
/// its `Cargo.toml` § 1 is that rule — so the only thing a refusal can say
/// about *whose* mistake it is is its [`io::ErrorKind`], and `nvs-stdlib`'s
/// `statement_failure` reads `InvalidInput` as a mistake in the call and
/// everything else as a refusal the engine made. An `Other` here would reach a
/// program as `Core\Db\DbError` carrying a § 8 `kind` no engine ever answered,
/// where § 4's uniform busy rule is a `LogicError` on every driver.
fn busy(state: State) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "this SQLite connection is {state:?}: read or drop the rows of the statement already \
             running before starting another, or open a second connection for the second flow"
        ),
    )
}

/// SQLite's refusal as an `io::Error` carrying § 8's normalised kind.
///
/// Some of [`ServerError`]'s fields are empty by nature of this backend rather
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

/// [ADR 0067 § 8](/docs/decisions/0067.md)'s kind, from SQLite's
/// extended result code.
///
/// Keyed on the extended code because the primary one is too coarse to answer
/// the question § 8 asks: `SQLITE_CONSTRAINT` covers several of § 8's kinds at
/// once, and an application branching on `UniqueViolation` would get nothing
/// from it. Where the extended code adds nothing, the primary code is its low
/// byte and the fallthrough reads that.
///
/// § 8 names these rows itself: `SQLITE_BUSY` and `SQLITE_LOCKED` are
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
    use crate::conn::{
        BlockError, ColumnType, DbErrorKind, Driver, Isolation, ServerError, SqliteConn, State,
    };
    use nvs_config::tree::Database;
    use std::path::Path;

    /// SQLite's own answer to "is a transaction open on this connection", which
    /// is what every depth assertion below is checked against — the count is
    /// this driver's and the autocommit flag is the engine's, so a test that
    /// read only the count would pass on a driver that never sent anything.
    fn autocommit(conn: &SqliteConn) -> bool {
        super::lock(&conn.handle).is_autocommit()
    }

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
    /// to, which is the one thing the other drivers cannot do in a unit test.
    fn connect() -> crate::conn::SqliteConn {
        open(&SqliteTarget::resolve(&block()).expect("the block resolves")).expect("it opens")
    }

    /// `serverVersion` on the one driver with no server: the version of the
    /// library this binary is linked against —
    /// [ADR 0187 § 2](/docs/decisions/0187.md).
    ///
    /// Asserted against what the engine itself answers rather than against a
    /// literal, so the two move together when the library does, and asserted to
    /// be three numbers, because a version a program cannot compare is not the
    /// answer the member promises.
    #[test]
    fn sqlite_answers_its_library_version() {
        let conn = connect();
        let asked: String = conn
            .handle
            .lock()
            .expect("an uncontended connection")
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .expect("SQLite answers for itself");

        assert_eq!(super::library_version(), asked);
        let parts: Vec<&str> = asked.split('.').collect();
        assert_eq!(parts.len(), 3, "`{asked}` is not major.minor.patch");
        assert!(
            parts.iter().all(|part| part.parse::<u32>().is_ok()),
            "`{asked}` is not three numbers"
        );
    }

    /// A block naming one database that two handles can both open, for the
    /// cases whose whole subject is what one connection does to another.
    ///
    /// **Neither handle is a file.** A `mode=memory&cache=shared` URI is a
    /// database shared by every handle in the process that names it, which is
    /// all a lock conflict needs and the one spelling of it that leaves nothing
    /// on disk for a failing case to leak. `open`'s
    /// `rusqlite::Connection::open` carries `SQLITE_OPEN_URI` in its default
    /// flags, so the path is read as a URI rather than as a file with an odd
    /// name. The name is per case, because the process is what scopes it and
    /// these cases run concurrently.
    fn shared(name: &str) -> Database {
        Database {
            driver: Some(String::from("sqlite")),
            path: Some(format!("file:{name}?mode=memory&cache=shared")),
            ..Database::default()
        }
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

    /// The ways the block says it is not this driver's, and the missing `path`
    /// that says it is not openable.
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

    /// The count a statement answers is its own, which is the one thing
    /// `sqlite3_changes` does not say on its own: it reports the last
    /// *data-changing* statement on the connection, so a `create table` run
    /// after an insert would answer the insert's rows. `Core\Db\Write::affected`
    /// answers `0` where a statement's kind has no count of its own, and
    /// [`step`] is where that holds for this backend.
    ///
    /// **Asserted after a statement that really did change rows**, because a
    /// zero read on a quiet connection passes whether the fold is there or not.
    /// Three rows are written first, so every statement below that changes none
    /// has three rows to wrongly report.
    // covers: Core\Db\Write::affected
    #[test]
    fn a_statement_that_changed_no_rows_answers_zero_and_not_an_earlier_count() {
        let conn = connect();
        conn.query(
            "create table t (id integer primary key, name text not null)",
            Vec::new(),
        )
        .expect("the schema applies");

        let inserted = conn
            .query(
                "insert into t (name) values (?), (?), (?)",
                vec![
                    SqliteValue::Text(String::from("ada")),
                    SqliteValue::Text(String::from("grace")),
                    SqliteValue::Text(String::from("alan")),
                ],
            )
            .expect("the insert runs");
        assert_eq!(inserted.affected(), 3);
        drop(inserted);

        for quiet in [
            "create table u (id integer primary key)",
            "drop table u",
            "select id from t",
        ] {
            let answered = conn.query(quiet, Vec::new()).expect("the statement runs");
            assert_eq!(
                answered.affected(),
                0,
                "`{quiet}` answered the count of the insert before it"
            );
            drop(answered);
        }

        let missed = conn
            .query(
                "update t set name = ? where id = ?",
                vec![
                    SqliteValue::Text(String::from("nobody")),
                    SqliteValue::Int(99),
                ],
            )
            .expect("the update runs");
        assert_eq!(missed.affected(), 0);
        drop(missed);

        let deleted = conn
            .query("delete from t", Vec::new())
            .expect("the delete runs");
        assert_eq!(deleted.affected(), 3);
    }

    /// **The key a statement reports is the key of a row that statement
    /// inserted**, which this module's doc § *Which statement inserted a row*
    /// argues `sqlite3_last_insert_rowid` cannot answer on its own: it belongs
    /// to the connection, so every statement after an insert would go on
    /// reporting that insert's key.
    ///
    /// The last step is the corner the update hook decides and the connection's
    /// own value does not: a row a trigger inserted was inserted by the
    /// statement that fired the trigger, so the statement reports it. SQLite
    /// reverts its own value when the trigger ends.
    // covers: Core\Db\Write::lastId
    #[test]
    fn a_statement_reports_the_key_of_a_row_it_inserted_and_none_otherwise() {
        let conn = connect();
        for schema in [
            "create table t (id integer primary key, name text not null)",
            "create table seen (id integer primary key, name text not null)",
            "create trigger t_seen after update on t begin insert into seen (name) \
             values (new.name); end",
        ] {
            conn.query(schema, Vec::new()).expect("the schema applies");
        }

        let key = |sql: &str| -> Option<i64> {
            let mut rows = conn.query(sql, Vec::new()).expect("the statement runs");
            while rows.next_row().is_some() {}
            rows.last_insert_id()
        };

        // 1. The first row written is row 1, and several rows in one statement
        //    report the last of them.
        assert_eq!(key("insert into t (name) values ('ada')"), Some(1));
        assert_eq!(
            key("insert into t (name) values ('grace'), ('alan'), ('kay')"),
            Some(4)
        );

        // 2. A statement that inserted nothing reports nothing, where the
        //    connection's own value would report row 4 for every one of them.
        assert_eq!(key("delete from t where id = 3"), None);
        assert_eq!(key("create table u (id integer primary key)"), None);
        assert_eq!(key("select name from t"), None);

        // 3. An `update` fires the trigger, so a row was inserted after all,
        //    and the key is that row's.
        assert_eq!(key("update t set name = 'ada l.' where id = 1"), Some(1));
        assert_eq!(key("update t set name = 'k.' where id = 4"), Some(2));
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
        assert_eq!(inserted.last_insert_id(), Some(1));
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

    /// `rule:types/bytes`'s guarantee, which every other driver gets from § 3's forced
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

    /// § 7's nesting: `BEGIN` at the bottom, a `SAVEPOINT` above it to any
    /// depth, and the count following each command the engine took. This is the
    /// one driver where the assertion is the engine's own answer rather than a
    /// scripted peer's — `is_autocommit` is SQLite saying whether a transaction
    /// is open, so the depth is checked against the file and not against itself.
    #[test]
    fn nesting_is_begin_then_savepoints_and_the_depth_follows_the_engine() {
        let conn = connect();
        assert_eq!(conn.depth(), 0);
        assert!(autocommit(&conn));

        for level in 1..=3 {
            conn.begin(None, false).expect("a level SQLite took");
            assert_eq!(conn.depth(), level);
            assert!(!autocommit(&conn));
        }

        conn.roll_back().expect("the innermost level, undone");
        assert_eq!(conn.depth(), 2);
        assert!(!autocommit(&conn));

        conn.commit().expect("a nested commit");
        assert_eq!(conn.depth(), 1);
        assert!(!autocommit(&conn));

        let span = conn.commit().expect("the outermost commit");
        assert_eq!(span.driver(), Driver::Sqlite);
        assert_eq!(span.sql(), "COMMIT");
        assert_eq!(conn.depth(), 0);
        assert!(autocommit(&conn));
    }

    /// § 7's nested rollback undoes its own level and leaves the outer one's
    /// writes alone — the composition the closure form exists for, asserted on
    /// the rows rather than on the commands sent.
    #[test]
    fn a_nested_rollback_undoes_its_level_and_not_the_one_around_it() {
        let conn = connect();
        conn.query("create table t (v integer)", Vec::new())
            .expect("the schema applies");

        conn.begin(None, false).expect("the outermost level");
        conn.query("insert into t (v) values (1)", Vec::new())
            .expect("the outer write");
        conn.begin(None, false).expect("a nested level");
        conn.query("insert into t (v) values (2)", Vec::new())
            .expect("the inner write");
        conn.roll_back().expect("the nested level, undone");
        conn.commit().expect("the outermost commit");

        let mut rows = conn
            .query("select v from t order by v", Vec::new())
            .expect("the select runs");
        assert_eq!(rows.next_row(), Some(vec![SqliteValue::Int(1)]));
        assert_eq!(rows.next_row(), None);
    }

    /// A second `ROLLBACK TO` against the same name would fail if the nested
    /// form had not released the savepoint it returned to, which is the leak
    /// `roll_back`'s two-statement batch exists to prevent.
    #[test]
    fn a_nested_rollback_releases_the_savepoint_it_returned_to() {
        let conn = connect();
        conn.begin(None, false).expect("the outermost level");

        for _ in 0..3 {
            conn.begin(None, false).expect("a nested level");
            assert_eq!(conn.depth(), 2);
            conn.roll_back().expect("that level, undone");
            assert_eq!(conn.depth(), 1);
        }

        conn.roll_back().expect("the outermost rollback");
        assert_eq!(conn.depth(), 0);
    }

    /// A write that another connection's open transaction is holding the table
    /// against is § 8's `Deadlock`, which is what makes § 7's `{retries: n}`
    /// mean something on this backend.
    ///
    /// Two connections to one database, per [`shared`].
    ///
    /// The kind is asserted and the extended code is not: a shared-cache
    /// conflict answers `SQLITE_LOCKED` where a file conflict answers
    /// `SQLITE_BUSY`, and § 8 maps both to the same kind precisely so a caller
    /// never has to know which lock it lost. That is the claim `{retries: n}`
    /// rests on, and it is the one this pins.
    #[test]
    fn a_lock_another_connection_holds_is_section_8s_deadlock_kind() {
        let block = shared("nvs-db-lock-conflict");
        let target = SqliteTarget::resolve(&block).expect("the block resolves");
        let holder = open(&target).expect("the first handle opens");
        let waiter = open(&target).expect("the second handle opens");

        holder
            .query("create table t (v integer)", Vec::new())
            .expect("the schema applies");
        holder.begin(None, false).expect("the holder's transaction");
        holder
            .query("insert into t (v) values (1)", Vec::new())
            .expect("the write that takes the table");

        let refused = waiter
            .query("insert into t (v) values (2)", Vec::new())
            .expect_err("the second connection cannot have the table");
        let server = ServerError::of(&refused).expect("a refusal carried no kind");
        assert_eq!(server.kind, DbErrorKind::Deadlock);
        assert_eq!(server.backend, "sqlite");

        holder.commit().expect("the holder still owns its level");
        assert_eq!(
            waiter.depth(),
            0,
            "a refused statement opened no transaction on the connection that lost the lock"
        );
    }

    /// § 7's `Isolation` on the one backend that has a single level: every one
    /// of them is accepted at the outermost level because serializable is
    /// stronger than any of them, none of them renders to a command, and a
    /// nested call naming one is still refused — the level belongs to the whole
    /// transaction.
    #[test]
    fn every_isolation_level_is_accepted_because_sqlite_is_always_serializable() {
        let levels = [
            Isolation::ReadUncommitted,
            Isolation::ReadCommitted,
            Isolation::RepeatableRead,
            Isolation::Snapshot,
            Isolation::Serializable,
        ];

        for level in levels {
            let conn = connect();
            let span = conn.begin(Some(level), false).expect("SQLite took it");
            assert_eq!(span.sql(), "BEGIN");
            assert_eq!(conn.depth(), 1);

            let nested = conn
                .begin(Some(level), false)
                .expect_err("a nested level naming its own is refused");
            assert_eq!(nested.kind(), std::io::ErrorKind::InvalidInput);
            assert_eq!(conn.depth(), 1);

            conn.begin(None, false).expect("the same level, unnamed");
            assert_eq!(conn.depth(), 2);
        }
    }

    /// `rule:concurrency/claiming-is-one-statement`'s immediate transaction, in
    /// the one arrangement that can tell it from a deferred one: a second
    /// connection loses the write before the holder has run a single statement.
    ///
    /// The case below is the other half — the same shape against
    /// [`SqliteConn::begin`], where the write goes through — and the pair is
    /// the whole difference the claim rests on. Neither half of it is visible
    /// on one connection, which is why both run two.
    #[test]
    fn an_immediate_transaction_holds_the_write_lock_before_its_first_statement_runs() {
        let block = shared("nvs-db-immediate-holds-the-lock");
        let target = SqliteTarget::resolve(&block).expect("the block resolves");
        let holder = open(&target).expect("the first handle opens");
        let waiter = open(&target).expect("the second handle opens");

        holder
            .query("create table t (v integer)", Vec::new())
            .expect("the schema applies");

        let span = holder.begin_immediate().expect("the write lock, up front");
        assert_eq!(span.driver(), Driver::Sqlite);
        assert_eq!(span.sql(), "BEGIN IMMEDIATE");
        assert_eq!(holder.depth(), 1);
        assert!(!autocommit(&holder));

        let refused = waiter
            .query("insert into t (v) values (2)", Vec::new())
            .expect_err("the holder took the write lock and has written nothing");
        let server = ServerError::of(&refused).expect("a refusal carried no kind");
        assert_eq!(server.kind, DbErrorKind::Deadlock);
        assert_eq!(server.backend, "sqlite");

        holder.commit().expect("the holder closes its level");
        assert_eq!(holder.depth(), 0);
        waiter
            .query("insert into t (v) values (2)", Vec::new())
            .expect("the lock went with the transaction");
    }

    /// `Core\Db::transaction` opens the transaction it always has: the depth-0
    /// command is still a bare `BEGIN`, which holds no lock until its first
    /// statement, so a second connection writes straight through it.
    ///
    /// That is what makes the immediate entry point additive — no caller that
    /// never asked for a write lock up front now takes one.
    #[test]
    fn core_db_transaction_still_opens_the_same_deferred_transaction_it_always_did() {
        let block = shared("nvs-db-deferred-holds-nothing");
        let target = SqliteTarget::resolve(&block).expect("the block resolves");
        let holder = open(&target).expect("the first handle opens");
        let waiter = open(&target).expect("the second handle opens");

        holder
            .query("create table t (v integer)", Vec::new())
            .expect("the schema applies");

        let span = holder.begin(None, false).expect("the deferred transaction");
        assert_eq!(span.sql(), "BEGIN");
        assert_eq!(holder.depth(), 1);
        assert!(!autocommit(&holder));

        waiter
            .query("insert into t (v) values (1)", Vec::new())
            .expect("a deferred transaction is holding no lock to lose");

        holder.commit().expect("the holder closes its level");
        assert_eq!(holder.depth(), 0);
    }

    /// An immediate transaction is an outermost one, so a depth above zero is
    /// the third refusal this entry point answers — and it is answered before
    /// anything is sent, so the level the caller already holds is left exactly
    /// as it was.
    ///
    /// Refused inside a deferred transaction and inside an immediate one
    /// alike: what a nested level cannot do is take a lock the transaction
    /// around it did not, and a `SAVEPOINT` is all a nested level is.
    #[test]
    fn an_immediate_transaction_asked_for_inside_an_open_one_is_refused() {
        let conn = connect();
        conn.begin(None, false).expect("an ordinary transaction");
        let refused = conn
            .begin_immediate()
            .expect_err("a savepoint takes no lock of its own");
        assert_eq!(refused.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(conn.depth(), 1);

        conn.begin(None, false).expect("a nested level");
        let deeper = conn.begin_immediate().expect_err("refused deeper in too");
        assert_eq!(deeper.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(conn.depth(), 2);
        assert!(!autocommit(&conn));

        let fresh = connect();
        fresh
            .begin_immediate()
            .expect("the outermost immediate level");
        let nested = fresh
            .begin_immediate()
            .expect_err("one inside an immediate transaction is refused as well");
        assert_eq!(nested.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(fresh.depth(), 1);
    }

    /// A `BEGIN IMMEDIATE` the engine refuses leaves the connection in no
    /// transaction, which is this driver's one counter rule: the depth moves
    /// after the command, and only for a command SQLite accepted.
    ///
    /// The refusal is a real one — a second connection's write lock — rather
    /// than the depth guard above, so what is pinned is the engine's answer and
    /// not this file's own check. It is also § 7's `{retries: n}` becoming
    /// meaningful for a claim: the kind is `Deadlock`, and the retry is a whole
    /// transaction rather than an upgrade nothing can back off.
    #[test]
    fn a_refused_immediate_begin_leaves_the_connection_in_no_transaction() {
        let block = shared("nvs-db-immediate-refused");
        let target = SqliteTarget::resolve(&block).expect("the block resolves");
        let holder = open(&target).expect("the first handle opens");
        let waiter = open(&target).expect("the second handle opens");

        holder.begin_immediate().expect("the first write lock");

        let refused = waiter
            .begin_immediate()
            .expect_err("one write lock per database");
        let server = ServerError::of(&refused).expect("a refusal carried no kind");
        assert_eq!(server.kind, DbErrorKind::Deadlock);
        assert_eq!(server.backend, "sqlite");
        assert_eq!(waiter.depth(), 0);
        assert!(autocommit(&waiter));

        holder.commit().expect("the holder closes its level");
        waiter
            .begin_immediate()
            .expect("the lock is there once the holder is done with it");
        assert_eq!(waiter.depth(), 1);
    }

    /// § 7's `readOnly` is refused rather than dropped: SQLite has no read-only
    /// transaction, and opening a writable one under the word is the failure the
    /// option exists to prevent. Refused at every depth, and before the depth
    /// moves.
    #[test]
    fn a_read_only_transaction_is_refused_because_sqlite_has_none() {
        let conn = connect();
        let refused = conn
            .begin(None, true)
            .expect_err("read-only is not a thing SQLite offers");
        assert_eq!(refused.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(conn.depth(), 0);

        conn.begin(None, false).expect("an ordinary transaction");
        let nested = conn.begin(None, true).expect_err("refused when nested too");
        assert_eq!(nested.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(conn.depth(), 1);
    }

    /// § 7 has no `commit()` on the connection, so a call with nothing open is
    /// this driver's own bug and says so rather than sending a bare `ROLLBACK`.
    #[test]
    fn closing_a_transaction_that_was_never_opened_is_refused() {
        let conn = connect();
        for refused in [
            conn.commit().expect_err("nothing to commit"),
            conn.roll_back().expect_err("nothing to roll back"),
        ] {
            assert_eq!(refused.kind(), std::io::ErrorKind::InvalidInput);
        }
    }

    /// § 13's reset is the rollback and nothing else, and it undoes a
    /// transaction the *depth never saw* — one a caller opened in its own § 4
    /// statement text — because what it asks is the engine's `is_autocommit`
    /// rather than this driver's count.
    #[test]
    fn a_reset_rolls_back_whatever_is_open_and_zeroes_the_depth() {
        let conn = connect();
        conn.query("create table t (v integer)", Vec::new())
            .expect("the schema applies");
        conn.begin(None, false).expect("the outermost level");
        conn.begin(None, false).expect("a nested level");
        conn.query("insert into t (v) values (1)", Vec::new())
            .expect("the write nobody committed");
        assert_eq!(conn.depth(), 2);

        let conn = conn.reset().expect("the reset works");
        assert_eq!(conn.depth(), 0);
        assert!(autocommit(&conn));

        let mut rows = conn
            .query("select v from t", Vec::new())
            .expect("the select runs");
        assert_eq!(rows.next_row(), None);
        drop(rows);

        // The depth never sees this one: it is a caller's own text, and the
        // reset still proves the connection clean.
        conn.query("begin", Vec::new()).expect("a raw BEGIN");
        assert_eq!(conn.depth(), 0);
        assert!(!autocommit(&conn));
        let conn = conn.reset().expect("the reset works anyway");
        assert!(autocommit(&conn));
    }

    /// § 13's reset is only meaningful between statements, and a connection
    /// that is not at rest is refused rather than drained.
    ///
    /// § 4's *streaming* half of that is held by the borrow checker instead and
    /// cannot be written as a case at all: [`SqliteRows`] borrows the
    /// connection, so `reset`'s by-value receiver will not compile while one is
    /// alive. What is left to assert is the state a caller can reach with no
    /// rows in hand — `nvs-stdlib` poisons a connection whose request was torn
    /// down mid-call — and that is the security-relevant one: a connection that
    /// cannot be proven clean is closed, never reset.
    #[test]
    fn a_connection_that_is_not_at_rest_is_not_resettable() {
        let conn = connect();
        conn.state.set(State::Poisoned);

        let refused = conn
            .reset()
            .expect_err("a poisoned connection is not clean");
        assert_eq!(refused.kind(), std::io::ErrorKind::InvalidInput);
    }

    /// § 9's map on the one backend that has no types to map: every case the
    /// table names, keyed off what the schema declared, and asserted through a
    /// real statement rather than by calling the map on a string — the whole
    /// question is whether SQLite hands back the word the schema wrote.
    ///
    /// A sweep rather than a line each, and it is the counting shape on
    /// purpose: a map that answered plausibly for `date` and wrongly for
    /// `datetime` reads fine one row at a time.
    #[test]
    fn a_column_describes_off_the_type_its_schema_declared() {
        let declared = [
            ("smallint", ColumnType::Int),
            ("integer", ColumnType::Int),
            ("bigint", ColumnType::Int),
            ("unsigned big int", ColumnType::Int),
            ("real", ColumnType::Float),
            ("double precision", ColumnType::Float),
            ("float", ColumnType::Float),
            ("decimal(10, 2)", ColumnType::Decimal),
            ("numeric", ColumnType::Decimal),
            ("money", ColumnType::Decimal),
            ("text", ColumnType::Text),
            ("varchar(255)", ColumnType::Text),
            ("clob", ColumnType::Text),
            ("json", ColumnType::Text),
            ("blob", ColumnType::Bytes),
            ("boolean", ColumnType::Bool),
            ("date", ColumnType::Date),
            ("time", ColumnType::Time),
            ("datetime", ColumnType::DateTime),
            ("timestamp", ColumnType::DateTime),
            ("uuid", ColumnType::Uuid),
            ("point", ColumnType::Int),
            ("geometry", ColumnType::Other),
        ];

        let conn = connect();
        let columns = declared
            .iter()
            .enumerate()
            .map(|(i, (written, _))| format!("c{i} {written}"))
            .collect::<Vec<_>>()
            .join(", ");
        conn.query(&format!("create table t ({columns})"), Vec::new())
            .expect("the schema applies");

        let rows = conn
            .query("select * from t", Vec::new())
            .expect("the select runs");
        assert_eq!(rows.columns().len(), declared.len());

        let wrong = rows
            .columns()
            .iter()
            .zip(declared)
            .filter(|(column, (_, want))| column.column_type() != *want)
            .map(|(column, (written, want))| {
                format!("{written} declared {:?}, wanted {want:?}", column.declared)
            })
            .collect::<Vec<_>>();
        assert!(wrong.is_empty(), "§ 9 disagrees on: {wrong:?}");
    }

    /// § 9's total case: an expression has no declared type, and describing it
    /// as the `BLOB` affinity SQLite's own rule 3 would give it is a wrong
    /// answer where `Other` is a true one.
    #[test]
    fn an_expression_describes_as_the_total_case_and_not_as_bytes() {
        let conn = connect();
        let rows = conn
            .query("select 1 + 1 as sum", Vec::new())
            .expect("the select runs");
        assert_eq!(rows.columns()[0].declared, None);
        assert_eq!(rows.columns()[0].column_type(), ColumnType::Other);
    }

    /// The storage class of a value says nothing about what the column is,
    /// which is § 9's whole reason for keying this backend off the schema: one
    /// `date` column holding an integer and a string describes the same either
    /// way, and it is `nvs-stdlib` that throws on the cell that does not parse.
    #[test]
    fn the_declared_type_and_not_the_stored_class_is_what_describes_a_column() {
        let conn = connect();
        conn.query("create table t (when_ date)", Vec::new())
            .expect("the schema applies");
        conn.query(
            "insert into t (when_) values (?), (?)",
            vec![
                SqliteValue::Int(20_260_903),
                SqliteValue::Text(String::from("2026-09-03")),
            ],
        )
        .expect("both rows insert");

        let mut rows = conn
            .query("select when_ from t", Vec::new())
            .expect("the select runs");
        assert_eq!(rows.columns()[0].column_type(), ColumnType::Date);
        assert_eq!(rows.next_row(), Some(vec![SqliteValue::Int(20_260_903)]));
        assert_eq!(
            rows.next_row(),
            Some(vec![SqliteValue::Text(String::from("2026-09-03"))])
        );
    }

    /// § 4's rule holds over § 7's commands too: a `BEGIN` written while a
    /// result set is unread is the same busy refusal a second `query` is.
    #[test]
    fn a_transaction_command_while_rows_are_held_is_the_busy_refusal() {
        let conn = connect();
        conn.query("create table t (v integer)", Vec::new())
            .expect("the schema applies");
        let rows = conn.query("select v from t", Vec::new()).expect("it runs");

        let refused = conn.begin(None, false).expect_err("the connection is busy");
        assert!(
            refused.to_string().contains("Streaming"),
            "the refusal names § 4's rule and both its fixes: {refused}"
        );
        assert_eq!(conn.depth(), 0);
        drop(rows);
    }

    /// § 9's write half: a bound value takes a storage class rather than a text
    /// rendering, and `NULL` is one of the five rather than an absence.
    ///
    /// Asserted against a real bind rather than only on the enum, because the
    /// pair that matters is `encode` and [`SqliteValue`]'s `ToSql`: a value that
    /// converts and then binds as something else is what a comparison against
    /// the enum alone would miss.
    #[test]
    fn a_bound_value_takes_a_storage_class_and_not_a_rendering() {
        use nvs_runtime::Value;

        assert_eq!(
            super::encode(Value::null()).expect("SQL NULL"),
            SqliteValue::Null
        );
        // The engine's own spelling, not this driver's: `true` *is* `1` here.
        assert_eq!(
            super::encode(Value::bool(true)).expect("a boolean binds"),
            SqliteValue::Int(1)
        );
        assert_eq!(
            super::encode(Value::int(-7)).expect("an integer binds"),
            SqliteValue::Int(-7)
        );
        assert_eq!(
            super::encode(Value::uint(u64::try_from(i64::MAX).expect("it fits")))
                .expect("the widest `uint` an `INTEGER` holds"),
            SqliteValue::Int(i64::MAX)
        );

        let conn = connect();
        let mut rows = conn
            .query(
                "select typeof(?), typeof(?), typeof(?)",
                vec![
                    super::encode(Value::null()).expect("it binds"),
                    super::encode(Value::bool(false)).expect("it binds"),
                    super::encode(Value::float(1.5)).expect("it binds"),
                ],
            )
            .expect("it runs");
        assert_eq!(
            rows.next_row(),
            Some(vec![
                SqliteValue::Text(String::from("null")),
                SqliteValue::Text(String::from("integer")),
                SqliteValue::Text(String::from("real")),
            ]),
            "the engine agrees with the class this driver chose"
        );
    }

    /// The values with no storage class, refused rather than narrowed — and the
    /// infinity that is not one of them, which is where this driver parts from
    /// MySQL's and SQL Server's encoders.
    ///
    /// A bound on both sides for the `uint`: the widest one an `INTEGER` holds
    /// is asserted above, and the first one past it is asserted here, so an
    /// encoder that stopped one value early fails.
    #[test]
    fn a_value_with_no_storage_class_is_refused_and_an_infinity_is_not() {
        use nvs_runtime::Value;

        let past = super::encode(Value::uint(u64::try_from(i64::MAX).expect("it fits") + 1))
            .expect_err("no `INTEGER` holds it");
        assert_eq!(past.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            past.to_string().contains("unsigned"),
            "the refusal says what SQLite has none of: {past}"
        );

        let nan = super::encode(Value::float(f64::NAN)).expect_err("it would bind as NULL");
        assert!(
            nan.to_string().contains("NULL"),
            "the refusal says what SQLite would have stored instead: {nan}"
        );
        for finite in [f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                super::encode(Value::float(finite)).expect("an infinity is a `REAL` here"),
                SqliteValue::Real(finite),
                "MySQL and SQL Server refuse this one for want of a literal; this driver binds a \
                 double and has no literal to want"
            );
        }
    }

    /// The parked form of a walk on the driver with no wire, which the user's
    /// pinned-thread call is what it is: the statement stays on a pool thread
    /// and one owned row crosses back per step.
    ///
    /// The lock is half the assertion and the only one that can tell a walk from
    /// a buffer: a statement that had been stepped to exhaustion would have let
    /// go of the connection before the first row was read, so a `try_lock` that
    /// fails mid-walk is the result set still being *in* SQLite. The other half
    /// is the run — one row per step, in order, the connection busy for as long
    /// as rows remain and idle the moment the last one is read, with the
    /// description and the span outliving the rows for the decode that reads
    /// them.
    #[test]
    fn sqlite_stream_parks_its_read_or_names_its_recorded_refusal() {
        let mut conn = connect();
        conn.query("create table t (v integer)", Vec::new())
            .expect("the schema applies");
        conn.query("insert into t (v) values (7), (9)", Vec::new())
            .expect("the rows go in");

        let described = conn
            .stream("select v from t order by v", Vec::new())
            .expect("the walk opens")
            .len();
        assert_eq!(described, 1);
        assert_eq!(conn.state.get(), State::Streaming);
        assert!(
            conn.handle.try_lock().is_err(),
            "the walk's thread is not holding the connection, so the rows were read into memory"
        );

        let refused = conn
            .query("select 1", Vec::new())
            .expect_err("a second statement is refused while a walk is open");
        assert!(
            refused.to_string().contains("open a second connection"),
            "§ 4's refusal names both fixes: {refused}"
        );

        let mut seen = Vec::new();
        while let Some(row) = conn.stream_next_row().expect("the walk advanced") {
            assert_eq!(
                conn.state.get(),
                State::Streaming,
                "a walk with rows left to read freed the connection"
            );
            seen.push(row);
        }

        assert_eq!(seen, [vec![SqliteValue::Int(7)], vec![SqliteValue::Int(9)]]);
        assert_eq!(
            conn.state.get(),
            State::Idle,
            "a drained walk left the connection unpoolable"
        );
        assert!(
            conn.handle.try_lock().is_ok(),
            "the last step answered before its thread let go of the connection"
        );
        assert_eq!(
            conn.stream_columns().map(<[_]>::len),
            Some(1),
            "the description is what a row is decoded against, so it outlives the rows"
        );
        assert_eq!(conn.stream_span().map(super::QuerySpan::rows), Some(2));

        conn.end_stream();
        assert!(conn.stream_columns().is_none());
        conn.query("select 1", Vec::new())
            .expect("the connection is free once the walk has ended");
    }

    /// A walk nobody drained: the thread is released by [`super::SqliteConn::end_stream`],
    /// and the connection is usable the instant it returns.
    ///
    /// The abandoned half of `rule:core-classes/db-streaming` — a program that reads two rows of a
    /// million and returns. Nothing here has to be drained, which is this
    /// driver's one advantage over the four with a wire.
    #[test]
    fn an_abandoned_walk_releases_its_thread_and_the_connection() {
        let mut conn = connect();
        conn.query("create table t (v integer)", Vec::new())
            .expect("the schema applies");
        conn.query("insert into t (v) values (1), (2), (3)", Vec::new())
            .expect("the rows go in");

        conn.stream("select v from t order by v", Vec::new())
            .expect("the walk opens");
        assert_eq!(
            conn.stream_next_row().expect("the walk advanced"),
            Some(vec![SqliteValue::Int(1)])
        );

        conn.end_stream();
        assert_eq!(conn.state.get(), State::Idle);
        assert!(
            conn.handle.try_lock().is_ok(),
            "`end_stream` returned before its thread let go of the connection"
        );
        let rows = conn
            .query("select count(*) from t", Vec::new())
            .expect("the connection took a statement straight after the walk");
        drop(rows);

        conn.reset()
            .expect("an ended walk leaves a resettable connection");
    }

    /// § 4's deadline, filed on a connection a walk is holding.
    ///
    /// It cannot take the lock — the walk's thread has it for the walk's whole
    /// life — so [`super::set_busy_timeout`] goes through the thread instead.
    /// This is the case that wedges the core if that ever stops being true.
    #[test]
    fn a_deadline_filed_mid_walk_goes_through_the_walks_own_thread() {
        let mut conn = connect();
        conn.query("create table t (v integer)", Vec::new())
            .expect("the schema applies");
        conn.query("insert into t (v) values (4)", Vec::new())
            .expect("the row goes in");

        conn.stream("select v from t", Vec::new())
            .expect("the walk opens");
        super::set_busy_timeout(
            &conn,
            Some(std::time::Instant::now() + std::time::Duration::from_millis(50)),
        )
        .expect("the walk's thread filed it");

        assert_eq!(
            conn.stream_next_row().expect("the walk advanced"),
            Some(vec![SqliteValue::Int(4)])
        );
        assert_eq!(conn.stream_next_row().expect("the walk ended"), None);
        super::set_busy_timeout(&conn, None).expect("the lock is free once the walk has ended");
    }
}
