//! The `db` session store: one SQLite file in the data folder, in WAL mode
//! (`rule:http-server/no-session-block-means-the-data-folder-store`).
//!
//! It answers `rule:http-server/a-session-store-answers-six-operations`'s operations over two
//! tables keyed by the application and the identifier: `nvs_session_heads`, one row per session
//! with its expiry as milliseconds since the epoch, and `nvs_session_entries`, one row per key with
//! its encoded value. The application is the innermost `[[app]]` block's path, the one
//! `crate::cache::application` reads, so an id one application issued is absent in every other one
//! that shares the data folder.
//!
//! `load` is one statement, and every write is one short transaction on this core's connection,
//! so nothing is held between the first use and the write-back at the end of the request. Two requests
//! of one session never wait on each other beyond the few statements of a write, two that change
//! different keys keep both, and two that change one key keep the last
//! (`rule:http-server/a-session-is-loaded-once-and-written-back-per-key`). WAL mode lets every
//! reader run beside the one writer, and `busy_timeout` covers the moment two writers meet.
//!
//! **An expired head is absent at once and deleted later.** [`load`] reads only a head whose
//! expiry is still ahead, so expiry needs no sweep to be correct. [`sweep`] deletes the expired
//! sessions at most once a minute per core, from a write, so the file holds O(live sessions)
//! rather than O(sessions ever issued) (`rule:http-server/session-expiry-belongs-to-the-store`).
//!
//! **The file is Novis's own, so no capability is asked.** The operator chose the data folder, the
//! name is fixed, and no program-supplied path reaches it; it is `Core\Cache`'s artifact cache
//! and the data folder's `tmp/` in that respect, not a `Core\Db::open` path.
//!
//! **One connection per core**, opened at its first use and kept, as the shared tier's socket is.
//! The connection is taken out of its slot for the length of a statement and put back after, so a
//! second task on the same core that runs while the first is parked on the statement opens its own
//! rather than finding the slot borrowed. Every statement runs on the blocking pool
//! ([`nvs_db::SqliteConn::query`]), so a core never waits on the disk.
//!
//! Cost: one connection and its statement cache per core, O(cores). Per request, one indexed range
//! read at the first use, and for a changed session one transaction that writes one row per changed key
//! and the head, plus two indexed deletes per core per minute. The file holds one row per key of
//! each live session.

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nvs_db::{SqliteConn, SqliteTarget, SqliteValue};

use super::{Changes, Record, Whole};
use crate::cache::redis::Renamed;

/// What a new connection runs as it opens, before the driver's pragma allow list applies.
///
/// `synchronous = NORMAL` is WAL's ordinary setting: a write survives a crash of the process, and
/// only the last writes before a power loss can be lost. That is the right trade for a session,
/// whose next request can sign the user in again. A request may not write `synchronous`, so the
/// open runs it before the driver's authorizer is installed.
const SETUP: &[&str] = &["PRAGMA synchronous = NORMAL"];

/// What a new connection runs before its first statement, in order.
///
/// `nvs_session_heads` has one row per session: its expiry, its short token, and the ends of its
/// short and long clocks, each time in milliseconds since the epoch. `nvs_session_entries` has one
/// row per key, with `long` set for a long-lived one. `WITHOUT ROWID` stores each row in its key's
/// own b-tree, so a session's entries sit together and a load is one range read.
///
/// The two `DROP`s remove the tables earlier builds kept a session in whole, `nvs_session` keyed
/// by the identifier alone and `nvs_sessions` with one record per session. Their sessions are not
/// converted, so every user of an earlier build is signed out once. Each `DROP` runs once per
/// connection, and finds nothing after the first. [`opened`] drops the two tables below too where
/// they lack a lifetime's columns, for the same reason.
const SCHEMA: &[&str] = &[
    "DROP TABLE IF EXISTS nvs_session",
    "DROP TABLE IF EXISTS nvs_sessions",
    "CREATE TABLE IF NOT EXISTS nvs_session_heads (\
         app BLOB NOT NULL, \
         id TEXT NOT NULL, \
         expires_at INTEGER NOT NULL, \
         short_token TEXT, \
         short_expires_at INTEGER NOT NULL, \
         long_expires_at INTEGER NOT NULL, \
         PRIMARY KEY (app, id)\
     ) WITHOUT ROWID",
    "CREATE INDEX IF NOT EXISTS nvs_session_heads_expiry ON nvs_session_heads (expires_at)",
    "CREATE TABLE IF NOT EXISTS nvs_session_entries (\
         app BLOB NOT NULL, \
         id TEXT NOT NULL, \
         key BLOB NOT NULL, \
         value BLOB NOT NULL, \
         long INTEGER NOT NULL, \
         PRIMARY KEY (app, id, key)\
     ) WITHOUT ROWID",
];

/// One statement per table that fails where the table is missing or lacks the columns this build
/// writes. [`opened`] drops both tables when either fails with SQLite's "no such" error.
const PROBES: &[&str] = &[
    "SELECT short_token, short_expires_at, long_expires_at FROM nvs_session_heads LIMIT 0",
    "SELECT long FROM nvs_session_entries LIMIT 0",
];

/// How often one core deletes the expired rows.
///
/// A minute, because [`load`] already treats an expired row as absent: the sweep only bounds the
/// file's size, and a minute of expired rows is nothing beside `[session] short_lived`'s two hours.
const SWEEP_EVERY: Duration = Duration::from_secs(60);

/// How many prepared statements one connection keeps: about the twenty-five this module runs.
const STATEMENTS: usize = 28;

/// One core's open connection, the file it is open on, and when it last swept.
struct Held {
    path: PathBuf,
    conn: SqliteConn,
    swept: Option<Instant>,
}

thread_local! {
    /// This core's connection, or `None` before the first session operation and while a statement
    /// has it out.
    static HELD: RefCell<Option<Held>> = const { RefCell::new(None) };
}

/// Runs `operation` on this core's connection to `path`, opening it first where there is none.
///
/// A connection held for another file is dropped: the data folder is fixed for a process, so this
/// happens only in a test that moves it.
///
/// # Errors
///
/// The open's error, or `operation`'s.
fn with<T>(path: &Path, operation: impl FnOnce(&mut Held) -> io::Result<T>) -> io::Result<T> {
    let taken = HELD
        .with_borrow_mut(Option::take)
        .filter(|held| held.path == path);
    let mut held = match taken {
        Some(held) => held,
        None => opened(path)?,
    };
    let answer = operation(&mut held);
    // A connection whose statement failed is dropped rather than kept, so a file that went away
    // or a connection SQLite poisoned is opened again on the next use.
    if answer.is_ok() {
        HELD.with_borrow_mut(|slot| {
            if slot.is_none() {
                *slot = Some(held);
            }
        });
    }
    answer
}

/// A new connection to `path` in WAL mode, with [`SCHEMA`] applied.
///
/// # Errors
///
/// SQLite's refusal: a folder that does not exist, a file that is not a database.
fn opened(path: &Path) -> io::Result<Held> {
    let conn = nvs_db::sqlite::open(&SqliteTarget {
        path,
        time_zone: 0,
        statement_cache: STATEMENTS,
        read_only: false,
        wal: true,
        deadline: None,
        setup: SETUP,
    })?;
    drop(conn.query("PRAGMA busy_timeout = 5000", Vec::new())?);
    // A table of an earlier layout is dropped, and its sessions with it. A missing table fails the
    // same way, and dropping it does nothing. Any other failure is the open's.
    for probe in PROBES {
        if let Err(why) = conn.query(probe, Vec::new()) {
            // The error's `Display` leaves out SQLite's own sentence, so it is read off the error.
            let missing = nvs_db::conn::ServerError::of(&why)
                .is_some_and(|refused| refused.message.starts_with("no such"));
            if !missing {
                return Err(why);
            }
            drop(conn.query("DROP TABLE IF EXISTS nvs_session_heads", Vec::new())?);
            drop(conn.query("DROP TABLE IF EXISTS nvs_session_entries", Vec::new())?);
            break;
        }
    }
    for statement in SCHEMA {
        drop(conn.query(statement, Vec::new())?);
    }
    Ok(Held {
        path: path.to_path_buf(),
        conn,
        swept: None,
    })
}

/// `load`: every key of the session under `id` with its lifetime, the short token and both clocks,
/// or absent for an identifier with no live head.
///
/// One statement: the live head `LEFT JOIN` its entries. No row is absent, and one row with a
/// `NULL` key is a session with no keys.
///
/// # Errors
///
/// SQLite's refusal, which the first use throws as an `IOError` and never reads as absent. A session
/// larger than the request's memory has left is that error too, and its refusal is recorded, so
/// the member ends with the memory `FATAL`.
///
/// The session is asked of the request's memory ceiling before it is held. The query reads the
/// session's total length beside each row and gives the keys and values only when that fits the
/// request's headroom, so a session that does not fit is never read out of the file. Every key and
/// value is a blob, so the length is in bytes. What this spends: one sum over the session's
/// entries per load, in the same statement.
pub(crate) fn load(path: &Path, app: &[u8], id: &str) -> io::Result<Option<Record>> {
    let now = crate::queue::now_millis();
    let room = nvs_runtime::budget::headroom()
        .and_then(|room| i64::try_from(room).ok())
        .unwrap_or(i64::MAX);
    with(path, |held| {
        let mut rows = held.conn.query(
            "SELECT size.total, \
                    CASE WHEN size.total <= ?4 THEN e.key END, \
                    CASE WHEN size.total <= ?4 THEN e.value END, \
                    e.long, h.short_token, h.short_expires_at, h.long_expires_at \
             FROM nvs_session_heads h \
             CROSS JOIN (SELECT coalesce(sum(length(key) + length(value)), 0) AS total \
                         FROM nvs_session_entries WHERE app = ?1 AND id = ?2) size \
             LEFT JOIN nvs_session_entries e ON e.app = h.app AND e.id = h.id \
             WHERE h.app = ?1 AND h.id = ?2 AND h.expires_at > ?3",
            vec![
                SqliteValue::Blob(app.to_vec()),
                SqliteValue::Text(id.to_owned()),
                SqliteValue::Int(now),
                SqliteValue::Int(room),
            ],
        )?;
        let mut record = Record::default();
        let mut live = false;
        while let Some(row) = rows.next_row() {
            let mut row = row.into_iter();
            let total = match row.next() {
                Some(SqliteValue::Int(total)) => usize::try_from(total).unwrap_or(usize::MAX),
                _ => 0,
            };
            let (key, value) = (row.next().and_then(blob), row.next().and_then(blob));
            let long = matches!(row.next(), Some(SqliteValue::Int(1)));
            if !live {
                live = true;
                record.token = row
                    .next()
                    .and_then(blob)
                    .and_then(|token| String::from_utf8(token).ok());
                record.short_until = integer(row.next());
                record.long_until = integer(row.next());
            }
            let (Some(key), Some(value)) = (key, value) else {
                if total > 0 {
                    nvs_runtime::affordable(Some(total), "Core\\Session").map_err(|_| {
                        io::Error::other(
                            "the stored session is larger than the request's memory limit has left",
                        )
                    })?;
                    // The headroom was read before the query, which allocates too.
                    return Err(io::Error::other(
                        "the stored session did not fit the memory the request had left",
                    ));
                }
                // A head with no entries: a session with no keys.
                continue;
            };
            if long {
                record.long.insert(key.clone());
            }
            record.entries.insert(key, value);
        }
        Ok(live.then_some(record))
    })
}

/// An `INTEGER` column's value, and `0` for any other.
fn integer(value: Option<SqliteValue>) -> i64 {
    match value {
        Some(SqliteValue::Int(number)) => number,
        _ => 0,
    }
}

/// A column's bytes, for a `BLOB` or a `TEXT` one, and `None` for `NULL`.
fn blob(value: SqliteValue) -> Option<Vec<u8>> {
    match value {
        SqliteValue::Blob(bytes) => Some(bytes),
        SqliteValue::Text(text) => Some(text.into_bytes()),
        _ => None,
    }
}

/// Runs `steps` in one `BEGIN IMMEDIATE` transaction on `conn`. It commits when `steps` answers
/// `(true, _)`, and rolls back when it answers `(false, _)` or fails.
///
/// `IMMEDIATE` takes the write lock at the start, so two writers never both read and then both
/// try to upgrade: the second waits `busy_timeout` for the first's few statements. The
/// transaction lasts only as long as its own statements, so no request holds it between the first
/// use and the write-back.
///
/// # Errors
///
/// As [`load`].
fn atomically<T>(
    conn: &SqliteConn,
    steps: impl FnOnce(&SqliteConn) -> io::Result<(bool, T)>,
) -> io::Result<T> {
    conn.begin_immediate()?;
    match steps(conn) {
        Ok((true, answer)) => {
            conn.commit()?;
            Ok(answer)
        }
        Ok((false, answer)) => {
            conn.roll_back()?;
            Ok(answer)
        }
        Err(why) => {
            // The connection is dropped after a failure, which ends the transaction too; the roll
            // back here only makes that explicit, and its own failure says nothing new.
            drop(conn.roll_back());
            Err(why)
        }
    }
}

/// Runs a due [`sweep`] on `held`'s connection, after a write.
///
/// # Errors
///
/// As [`load`].
fn swept(held: &mut Held, now: i64) -> io::Result<()> {
    if held.swept.is_none_or(|last| last.elapsed() >= SWEEP_EVERY) {
        sweep(&held.conn, now)?;
        held.swept = Some(Instant::now());
    }
    Ok(())
}

/// The application and identifier as the first two parameters every statement here takes.
fn named(app: &[u8], id: &str) -> Vec<SqliteValue> {
    vec![
        SqliteValue::Blob(app.to_vec()),
        SqliteValue::Text(id.to_owned()),
    ]
}

/// `key`, `value` and the lifetime as the third to fifth parameters, after [`named`]'s.
fn entry_of(app: &[u8], id: &str, key: &[u8], value: &[u8], long: bool) -> Vec<SqliteValue> {
    let mut row = named(app, id);
    row.push(SqliteValue::Blob(key.to_vec()));
    row.push(SqliteValue::Blob(value.to_vec()));
    row.push(SqliteValue::Int(i64::from(long)));
    row
}

/// A short token as a parameter: its text, or `NULL`.
fn token_of(token: Option<&str>) -> SqliteValue {
    token.map_or(SqliteValue::Null, |token| {
        SqliteValue::Text(token.to_owned())
    })
}

/// The upsert of one entry, which [`issue`] and [`save`] share. A key written with the other
/// lifetime takes the new one.
const UPSERT: &str = "INSERT INTO nvs_session_entries (app, id, key, value, long) \
     VALUES (?1, ?2, ?3, ?4, ?5) \
     ON CONFLICT (app, id, key) DO UPDATE SET value = excluded.value, long = excluded.long";

/// The insert of a head that takes over an expired one. It takes the application, the identifier,
/// the expiry, the short token, the two clocks and now, and changes no row for a live head.
const NEW_HEAD: &str = "INSERT INTO nvs_session_heads \
     (app, id, expires_at, short_token, short_expires_at, long_expires_at) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
     ON CONFLICT (app, id) DO UPDATE SET expires_at = excluded.expires_at, \
         short_token = excluded.short_token, short_expires_at = excluded.short_expires_at, \
         long_expires_at = excluded.long_expires_at \
     WHERE nvs_session_heads.expires_at <= ?7";

/// The delete of every entry of one session.
const DROP_ENTRIES: &str = "DELETE FROM nvs_session_entries WHERE app = ?1 AND id = ?2";

/// The record's expiry, set from its clocks after a save: the later clock's end while a long-lived
/// entry remains, and the short clock's end otherwise.
const SET_EXPIRY: &str = "UPDATE nvs_session_heads SET expires_at = CASE \
         WHEN EXISTS (SELECT 1 FROM nvs_session_entries \
                      WHERE app = ?1 AND id = ?2 AND long = 1) \
         THEN max(short_expires_at, long_expires_at) ELSE short_expires_at END \
     WHERE app = ?1 AND id = ?2";

/// `issue`: a new session under `id` holding `whole`, and whether it was written.
///
/// One transaction, and never a write over a live session. `false` is an `id` that is already
/// live, which keeps what it has. An expired head is absent, as [`load`] reads it, so its `id` is
/// taken over, as the shared store's issue takes an expired key, and the entries it left are
/// deleted first. A due [`sweep`] runs after it on the same connection.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn issue(path: &Path, app: &[u8], id: &str, whole: &Whole<'_>) -> io::Result<bool> {
    let clocks = whole.clocks;
    with(path, |held| {
        let issued = atomically(&held.conn, |conn| {
            let mut head = named(app, id);
            head.push(SqliteValue::Int(clocks.expiry(whole.has_long())));
            head.push(token_of(whole.token));
            head.push(SqliteValue::Int(clocks.short_until));
            head.push(SqliteValue::Int(clocks.long_until));
            head.push(SqliteValue::Int(clocks.now));
            if conn.query(NEW_HEAD, head)?.affected() == 0 {
                return Ok((false, false));
            }
            drop(conn.query(DROP_ENTRIES, named(app, id))?);
            if !whole.entries.is_empty() {
                let rows = whole
                    .entries
                    .iter()
                    .map(|(key, value)| entry_of(app, id, key, value, whole.long.contains(key)))
                    .collect();
                conn.execute_many(UPSERT, rows)?;
            }
            Ok((true, true))
        })?;
        swept(held, clocks.now)?;
        Ok(issued)
    })
}

/// `save`: one request's `changes` applied to the session under `id`, and whether the session was
/// still live.
///
/// One transaction. It first sets a live head's clocks and writes the token it is given. The short
/// clock becomes now plus `[session] short_lived`. The long clock moves forward to the one sent,
/// and never past now plus `[session] long_lived`, so a clock stored far ahead comes down to its
/// lifetime. A head that is gone or expired changes no row: the transaction rolls back and answers
/// `false`, so the session stays absent — another request destroyed it, or it expired
/// (`rule:http-server/a-session-is-loaded-once-and-written-back-per-key`). Then, in order: every
/// entry of a cleared lifetime deleted, the removed keys deleted, the changed keys upserted, and
/// the expiry set from the clocks. A due [`sweep`] runs after it.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn save(path: &Path, app: &[u8], id: &str, changes: &Changes<'_>) -> io::Result<bool> {
    let clocks = changes.clocks;
    let now = clocks.now;
    with(path, |held| {
        let saved = atomically(&held.conn, |conn| {
            let mut head = named(app, id);
            head.push(token_of(changes.token));
            head.push(SqliteValue::Int(clocks.short_until));
            head.push(SqliteValue::Int(clocks.long_until));
            head.push(SqliteValue::Int(now));
            head.push(SqliteValue::Int(clocks.long_ceiling));
            let live = conn
                .query(
                    "UPDATE nvs_session_heads SET short_token = coalesce(?3, short_token), \
                         short_expires_at = ?4, \
                         long_expires_at = min(max(long_expires_at, ?5), ?7) \
                     WHERE app = ?1 AND id = ?2 AND expires_at > ?6",
                    head,
                )?
                .affected()
                > 0;
            if !live {
                return Ok((false, false));
            }
            if changes.cleared_short && changes.cleared_long {
                drop(conn.query(DROP_ENTRIES, named(app, id))?);
            } else if changes.cleared_short || changes.cleared_long {
                let mut lifetime = named(app, id);
                lifetime.push(SqliteValue::Int(i64::from(changes.cleared_long)));
                drop(conn.query(
                    "DELETE FROM nvs_session_entries WHERE app = ?1 AND id = ?2 AND long = ?3",
                    lifetime,
                )?);
            }
            if !changes.removed.is_empty() {
                let rows = changes
                    .removed
                    .iter()
                    .map(|key| {
                        let mut row = named(app, id);
                        row.push(SqliteValue::Blob(key.to_vec()));
                        row
                    })
                    .collect();
                conn.execute_many(
                    "DELETE FROM nvs_session_entries WHERE app = ?1 AND id = ?2 AND key = ?3",
                    rows,
                )?;
            }
            if !changes.changed.is_empty() {
                let rows = changes
                    .changed
                    .iter()
                    .map(|(key, value, long)| entry_of(app, id, key, value, *long))
                    .collect();
                conn.execute_many(UPSERT, rows)?;
            }
            drop(conn.query(SET_EXPIRY, named(app, id))?);
            Ok((true, true))
        })?;
        swept(held, now)?;
        Ok(saved)
    })
}

/// `move`: the session under `old` moved to `new` with its expiry, token and clocks, as one
/// transaction that runs only while the old head is live.
///
/// A gone old head and a live new one roll back and move nothing. Otherwise an expired session
/// under `new` is deleted, and the old head and its entries are re-pointed at `new`.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn relocate(path: &Path, app: &[u8], old: &str, new: &str) -> io::Result<Renamed> {
    let now = crate::queue::now_millis();
    with(path, |held| {
        atomically(&held.conn, |conn| {
            const LIVE: &str = "SELECT 1 FROM nvs_session_heads \
                 WHERE app = ?1 AND id = ?2 AND expires_at > ?3";
            let mut head = named(app, old);
            head.push(SqliteValue::Int(now));
            if conn.query(LIVE, head)?.next_row().is_none() {
                return Ok((false, Renamed::Gone));
            }
            let mut fresh = named(app, new);
            fresh.push(SqliteValue::Int(now));
            if conn.query(LIVE, fresh)?.next_row().is_some() {
                return Ok((false, Renamed::Taken));
            }
            drop(conn.query(DROP_ENTRIES, named(app, new))?);
            drop(conn.query(
                "DELETE FROM nvs_session_heads WHERE app = ?1 AND id = ?2",
                named(app, new),
            )?);
            let mut moving = named(app, old);
            moving.push(SqliteValue::Text(new.to_owned()));
            drop(conn.query(
                "UPDATE nvs_session_heads SET id = ?3 WHERE app = ?1 AND id = ?2",
                moving,
            )?);
            let mut moving = named(app, old);
            moving.push(SqliteValue::Text(new.to_owned()));
            drop(conn.query(
                "UPDATE nvs_session_entries SET id = ?3 WHERE app = ?1 AND id = ?2",
                moving,
            )?);
            Ok((true, Renamed::Done))
        })
    })
}

/// `destroy`: the session under `id`, its head and its entries deleted in one transaction, and
/// whether there was a head.
///
/// Deleting a session that is not there is not a failure. `false` tells a caller that another
/// request destroyed the session first.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn destroy(path: &Path, app: &[u8], id: &str) -> io::Result<bool> {
    with(path, |held| {
        atomically(&held.conn, |conn| {
            drop(conn.query(DROP_ENTRIES, named(app, id))?);
            let rows = conn.query(
                "DELETE FROM nvs_session_heads WHERE app = ?1 AND id = ?2",
                named(app, id),
            )?;
            Ok((true, rows.affected() > 0))
        })
    })
}

/// `take`: `key` deleted from the live session under `id` where it still holds `value`, and
/// whether a row was deleted.
///
/// One `DELETE`, so two requests that take the same key cannot both delete it. An expired or gone
/// head, a key another request took, and a key written again since this request loaded it all
/// delete nothing and answer `false`. The head and its expiry are left as they are.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn take(
    path: &Path,
    app: &[u8],
    id: &str,
    key: &[u8],
    value: &[u8],
) -> io::Result<bool> {
    let now = crate::queue::now_millis();
    with(path, |held| {
        let mut row = named(app, id);
        row.push(SqliteValue::Blob(key.to_vec()));
        row.push(SqliteValue::Blob(value.to_vec()));
        row.push(SqliteValue::Int(now));
        let rows = held.conn.query(
            "DELETE FROM nvs_session_entries \
             WHERE app = ?1 AND id = ?2 AND key = ?3 AND value = ?4 \
               AND EXISTS (SELECT 1 FROM nvs_session_heads \
                           WHERE app = ?1 AND id = ?2 AND expires_at > ?5)",
            row,
        )?;
        Ok(rows.affected() > 0)
    })
}

/// Deletes every session whose expiry is at or before `now`, its entries first and then its head,
/// and returns how many heads it deleted.
///
/// Two statements and no transaction: an entry is deleted only while its head is expired, and a
/// head an [`issue`] takes over between the two is live again and stays.
///
/// # Errors
///
/// As [`load`].
fn sweep(conn: &SqliteConn, now: i64) -> io::Result<u64> {
    drop(conn.query(
        "DELETE FROM nvs_session_entries WHERE (app, id) IN \
         (SELECT app, id FROM nvs_session_heads WHERE expires_at <= ?1)",
        vec![SqliteValue::Int(now)],
    )?);
    let rows = conn.query(
        "DELETE FROM nvs_session_heads WHERE expires_at <= ?1",
        vec![SqliteValue::Int(now)],
    )?;
    Ok(rows.affected())
}

/// Closes this thread's connection, so a case's scratch directory can be deleted.
#[cfg(test)]
pub(crate) fn closed() {
    HELD.with_borrow_mut(Option::take);
}

/// How many sessions the file holds, expired or not, read through a connection of its own.
#[cfg(test)]
pub(crate) fn rows(path: &Path) -> i64 {
    let held = opened(path).expect("a second connection");
    let mut answer = held
        .conn
        .query("SELECT COUNT(*) FROM nvs_session_heads", Vec::new())
        .expect("the count");
    match answer.next_row().and_then(|row| row.into_iter().next()) {
        Some(SqliteValue::Int(count)) => count,
        other => panic!("a count is an integer: {other:?}"),
    }
}

/// Moves the expiry of `app`'s session under `id` into the past, as if its lifetime had run out.
#[cfg(test)]
pub(crate) fn lapse(path: &Path, app: &[u8], id: &str) {
    with(path, |held| {
        drop(held.conn.query(
            "UPDATE nvs_session_heads SET expires_at = 0 WHERE app = ?1 AND id = ?2",
            named(app, id),
        )?);
        Ok(())
    })
    .expect("the row lapses");
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::super::{Clocks, Entries, LongKeys, Whole};
    use super::{Changes, closed, issue, load, rows, save, sweep, with};

    /// No long-lived key.
    static SHORT_ONLY: LongKeys = LongKeys::new();

    /// The clocks of a write taken now whose short clock runs for `lifetime`.
    fn clocks(lifetime: Duration) -> Clocks {
        let now = crate::queue::now_millis();
        Clocks {
            now,
            short_until: now + i64::try_from(lifetime.as_millis()).expect("a short lifetime"),
            long_until: 0,
            long_ceiling: 0,
        }
    }

    /// `issue` of short-lived `entries` under `id`, whose short clock runs for `lifetime`.
    fn issued(path: &std::path::Path, id: &str, entries: &Entries, lifetime: Duration) -> bool {
        let whole = Whole {
            entries,
            long: &SHORT_ONLY,
            token: None,
            clocks: clocks(lifetime),
        };
        issue(path, APP, id, &whole).expect("the session is issued")
    }

    /// The keys of the live session under `id`, or absent.
    fn keys_under(path: &std::path::Path, id: &str) -> Option<Entries> {
        load(path, APP, id)
            .expect("the store answers")
            .map(|record| record.entries)
    }

    /// The application every case here writes under.
    const APP: &[u8] = b"/srv/blog";

    /// A session holding `pairs`.
    fn entries(pairs: &[(&[u8], &[u8])]) -> Entries {
        pairs
            .iter()
            .map(|(key, value)| (key.to_vec(), value.to_vec()))
            .collect()
    }

    /// One request's changes that set `key` to `value`.
    fn setting<'a>(key: &'a [u8], value: &'a [u8]) -> Changes<'a> {
        Changes {
            changed: vec![(key, value, false)],
            clocks: clocks(Duration::from_secs(60)),
            ..Changes::default()
        }
    }

    /// A store file in a scratch directory one case owns, and the guard that deletes the
    /// directory when the case ends.
    fn store(name: &str) -> (nvs_repo::Scratch, PathBuf) {
        let dir = nvs_repo::scratch(name);
        let path = dir.join("session.sqlite");
        (dir, path)
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-back-per-key`: two requests of one session
    /// both load it, the second saves one key and finishes while the first is still running, and
    /// then the first saves another key. Neither waits for the other, and the session keeps both
    /// keys.
    ///
    /// Two threads are two cores, each with its own connection, which is what two concurrent
    /// requests are. The first thread is held between its load and its save until the second has
    /// finished, so a lock taken at `load` would hang this test instead of passing it.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn two_requests_of_one_session_keep_both_keys_in_the_file() {
        const ID: &str = "PmA5tKz1QvR3sYbNqLwXe2";
        let (_dir, path) = store("session-two-requests");
        let start = entries(&[(b"cart", b"empty")]);
        assert!(issued(&path, ID, &start, Duration::from_secs(60)));

        let (loaded, first_loaded) = mpsc::channel();
        let (go_on, second_done) = mpsc::channel::<()>();
        let first = {
            let path = path.clone();
            thread::spawn(move || {
                let seen = keys_under(&path, ID);
                loaded.send(()).expect("the test is waiting");
                second_done
                    .recv_timeout(Duration::from_secs(30))
                    .expect("the second request finishes while the first is still open");
                assert!(
                    save(&path, APP, ID, &setting(b"cart", b"three books"))
                        .expect("the first saves")
                );
                seen
            })
        };
        first_loaded
            .recv_timeout(Duration::from_secs(30))
            .expect("the first request loaded");

        let second = {
            let path = path.clone();
            thread::spawn(move || {
                let seen = keys_under(&path, ID);
                assert!(
                    save(&path, APP, ID, &setting(b"userId", b"17")).expect("the second saves")
                );
                seen
            })
        };
        assert_eq!(
            second.join().expect("the second request"),
            Some(start.clone())
        );
        assert_eq!(
            keys_under(&path, ID),
            Some(entries(&[(b"cart", b"empty"), (b"userId", b"17")]))
        );
        go_on.send(()).expect("the first request is waiting");
        assert_eq!(first.join().expect("the first request"), Some(start));

        assert_eq!(
            keys_under(&path, ID),
            Some(entries(&[(b"cart", b"three books"), (b"userId", b"17")])),
            "each request's key is kept"
        );
        closed();
    }

    /// `rule:http-server/no-session-block-means-the-data-folder-store`: a record written through
    /// one connection is read through another that opened the file afterwards. A second server
    /// process on the same data folder is exactly such a connection: the record is in the file and
    /// its WAL, not in any process.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn a_record_survives_into_a_connection_another_process_would_open() {
        const ID: &str = "QvR3sYbNqLwXe2PmA5tKz1";
        let (_dir, path) = store("session-another-process");
        let written = entries(&[(b"user", b"17")]);
        assert!(issued(&path, ID, &written, Duration::from_secs(60)));
        closed();

        let read = {
            let path = path.clone();
            thread::spawn(move || keys_under(&path, ID))
        };
        assert_eq!(read.join().expect("the second process"), Some(written));
    }

    /// `rule:http-server/session-expiry-belongs-to-the-store`: an expired row is absent at once,
    /// and the sweep deletes it while it keeps the live one.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn an_expired_row_is_absent_and_the_sweep_deletes_it() {
        const LIVE: &str = "LwXe2PmA5tKz1QvR3sYbNq";
        const GONE: &str = "A5tKz1QvR3sYbNqLwXe2Pm";
        let (_dir, path) = store("session-expired");
        let live = entries(&[(b"state", b"live")]);
        assert!(issued(&path, LIVE, &live, Duration::from_secs(3600)));
        assert!(issued(
            &path,
            GONE,
            &entries(&[(b"state", b"gone")]),
            Duration::ZERO
        ));

        assert_eq!(keys_under(&path, GONE), None);
        assert_eq!(rows(&path), 2, "the expired row is still in the file");

        let now = crate::queue::now_millis();
        let deleted = with(&path, |held| sweep(&held.conn, now)).expect("the sweep");
        assert_eq!(deleted, 1);
        assert_eq!(rows(&path), 1);
        assert_eq!(keys_under(&path, LIVE), Some(live));
        closed();
    }
}
