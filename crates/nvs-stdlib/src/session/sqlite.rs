//! The `db` session store: one SQLite file in the data folder, in WAL mode
//! (`rule:http-server/no-session-block-means-the-data-folder-store`).
//!
//! It answers `rule:http-server/a-session-store-answers-four-operations`'s operations over one
//! table, `nvs_sessions`, keyed by the application and the identifier, with the record as a blob
//! and its expiry as milliseconds since the epoch. The application is the innermost `[[app]]`
//! block's path, the one `crate::cache::application` reads, so an id one application issued is
//! absent in every other one that shares the data folder. Each operation is one short statement in SQLite's own autocommit
//! transaction, so nothing is held between `start` and the write-back at the end of the request:
//! two requests of one session never wait on each other, and the last write wins
//! (`rule:http-server/a-session-is-loaded-once-and-written-whole`). WAL mode lets every reader run
//! beside the one writer, and `busy_timeout` covers the moment two writers meet.
//!
//! **An expired row is absent at once and deleted later.** [`load`] reads only a row whose expiry
//! is still ahead, so expiry needs no sweep to be correct. [`sweep`] deletes the expired rows at
//! most once a minute per core, from a write, so the file holds O(live sessions) rather than
//! O(sessions ever issued) (`rule:http-server/session-expiry-belongs-to-the-store`).
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
//! Cost: one connection and its statement cache per core, O(cores). Per request, one indexed read at
//! `start`, and one indexed update for a changed record or one insert for a new one, plus one
//! indexed delete per core per minute.

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nvs_db::{SqliteConn, SqliteTarget, SqliteValue};

/// What a new connection runs before its first statement, in order.
///
/// `synchronous = NORMAL` is WAL's ordinary setting: a write survives a crash of the process, and
/// only the last writes before a power loss can be lost. That is the right trade for a session,
/// whose next request can sign the user in again. `WITHOUT ROWID` stores the record in the key's
/// own b-tree, so a read is one lookup.
///
/// The `DROP` removes the table an earlier build keyed by the identifier alone. Its rows name no
/// application, so no application may open them. It runs once per connection, and finds nothing
/// after the first.
const SCHEMA: &[&str] = &[
    "PRAGMA synchronous = NORMAL",
    "PRAGMA busy_timeout = 5000",
    "DROP TABLE IF EXISTS nvs_session",
    "CREATE TABLE IF NOT EXISTS nvs_sessions (\
         app BLOB NOT NULL, \
         id TEXT NOT NULL, \
         record BLOB NOT NULL, \
         expires_at INTEGER NOT NULL, \
         PRIMARY KEY (app, id)\
     ) WITHOUT ROWID",
    "CREATE INDEX IF NOT EXISTS nvs_sessions_expiry ON nvs_sessions (expires_at)",
];

/// How often one core deletes the expired rows.
///
/// A minute, because [`load`] already treats an expired row as absent: the sweep only bounds the
/// file's size, and a minute of expired rows is nothing beside `[session] ttl`'s two hours.
const SWEEP_EVERY: Duration = Duration::from_secs(60);

/// How many prepared statements one connection keeps: the five this module runs.
const STATEMENTS: usize = 5;

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
    })?;
    for statement in SCHEMA {
        drop(conn.query(statement, Vec::new())?);
    }
    Ok(Held {
        path: path.to_path_buf(),
        conn,
        swept: None,
    })
}

/// Milliseconds since the epoch, `lifetime` from now.
fn expiry(now: i64, lifetime: Duration) -> i64 {
    now.saturating_add(i64::try_from(lifetime.as_millis()).unwrap_or(i64::MAX))
}

/// `load`: the record under `id`, or absent for an identifier with no row or an expired one.
///
/// # Errors
///
/// SQLite's refusal, which `start` throws as an `IOError` and never reads as absent. A record
/// larger than the request's memory has left is that error too, and its refusal is recorded, so
/// the member ends with the memory `FATAL`.
///
/// The record is asked of the request's memory ceiling before it is held. The query reads the
/// record's length and gives the record only when it fits the request's headroom, so a record
/// that does not fit is never read out of the file. `save` always writes a blob, so the length
/// is in bytes. What this spends: one length read per load, in the same statement.
pub(crate) fn load(path: &Path, app: &[u8], id: &str) -> io::Result<Option<Vec<u8>>> {
    let now = crate::queue::now_millis();
    let room = nvs_runtime::budget::headroom()
        .and_then(|room| i64::try_from(room).ok())
        .unwrap_or(i64::MAX);
    with(path, |held| {
        let mut rows = held.conn.query(
            "SELECT length(record), CASE WHEN length(record) <= ?3 THEN record END \
             FROM nvs_sessions WHERE app = ?4 AND id = ?1 AND expires_at > ?2",
            vec![
                SqliteValue::Text(id.to_owned()),
                SqliteValue::Int(now),
                SqliteValue::Int(room),
                SqliteValue::Blob(app.to_vec()),
            ],
        )?;
        let Some(row) = rows.next_row() else {
            return Ok(None);
        };
        let mut row = row.into_iter();
        let length = match row.next() {
            Some(SqliteValue::Int(length)) => usize::try_from(length).unwrap_or(usize::MAX),
            // An empty record is zero bytes, which SQLite may hand back as `NULL`.
            _ => 0,
        };
        let record = match row.next() {
            Some(SqliteValue::Blob(record)) => record,
            Some(SqliteValue::Text(record)) => record.into_bytes(),
            _ => Vec::new(),
        };
        if record.len() < length {
            nvs_runtime::affordable(Some(length), "Core\\Session").map_err(|_| {
                io::Error::other(
                    "the stored session is larger than the request's memory limit has left",
                )
            })?;
            // The headroom was read before the query, which allocates too.
            return Err(io::Error::other(
                "the stored session did not fit the memory the request had left",
            ));
        }
        Ok(Some(record))
    })
}

/// `create`: a new row under `id`, with its expiry `lifetime` from now, and whether it was written.
///
/// An insert and never a replace of a live row. `false` is an `id` that already has a live row,
/// which keeps its record. An expired row is absent, as [`load`] reads it, so its `id` is taken
/// over, as the shared store's `SET … NX` takes an expired key. A due [`sweep`] runs after it on
/// the same connection.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn create(
    path: &Path,
    app: &[u8],
    id: &str,
    record: &[u8],
    lifetime: Duration,
) -> io::Result<bool> {
    written(
        path,
        "INSERT INTO nvs_sessions (app, id, record, expires_at) VALUES (?5, ?1, ?2, ?3) \
         ON CONFLICT (app, id) DO UPDATE SET record = excluded.record, \
         expires_at = excluded.expires_at WHERE nvs_sessions.expires_at <= ?4",
        app,
        id,
        record,
        lifetime,
    )
}

/// `replace`: the record under `id`, replaced whole, with its expiry `lifetime` from now, and
/// whether a row was changed.
///
/// An update of a row whose expiry is still ahead, and never an insert. `false` is an `id` with no
/// live row, which stays absent: another request destroyed the session, or it expired
/// (`rule:http-server/a-session-is-loaded-once-and-written-whole`). A due [`sweep`] runs after it.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn replace(
    path: &Path,
    app: &[u8],
    id: &str,
    record: &[u8],
    lifetime: Duration,
) -> io::Result<bool> {
    written(
        path,
        "UPDATE nvs_sessions SET record = ?2, expires_at = ?3 \
         WHERE app = ?5 AND id = ?1 AND expires_at > ?4",
        app,
        id,
        record,
        lifetime,
    )
}

/// Runs `statement`, the write of [`create`] or [`replace`], and then a due [`sweep`].
///
/// The statement takes the id, the record, the new expiry, now and the application as `?1` to
/// `?5`. It answers
/// whether it changed a row.
///
/// # Errors
///
/// As [`load`].
fn written(
    path: &Path,
    statement: &str,
    app: &[u8],
    id: &str,
    record: &[u8],
    lifetime: Duration,
) -> io::Result<bool> {
    let now = crate::queue::now_millis();
    with(path, |held| {
        let parameters = vec![
            SqliteValue::Text(id.to_owned()),
            SqliteValue::Blob(record.to_vec()),
            SqliteValue::Int(expiry(now, lifetime)),
            SqliteValue::Int(now),
            SqliteValue::Blob(app.to_vec()),
        ];
        let changed = held.conn.query(statement, parameters)?.affected() > 0;
        if held.swept.is_none_or(|last| last.elapsed() >= SWEEP_EVERY) {
            sweep(&held.conn, now)?;
            held.swept = Some(Instant::now());
        }
        Ok(changed)
    })
}

/// `destroy`: the row under `id`, deleted, and whether there was one.
///
/// Deleting a row that is not there is not a failure. `false` tells `regenerate` that another
/// request destroyed the session first.
///
/// # Errors
///
/// As [`load`].
pub(crate) fn destroy(path: &Path, app: &[u8], id: &str) -> io::Result<bool> {
    with(path, |held| {
        let rows = held.conn.query(
            "DELETE FROM nvs_sessions WHERE app = ?1 AND id = ?2",
            vec![
                SqliteValue::Blob(app.to_vec()),
                SqliteValue::Text(id.to_owned()),
            ],
        )?;
        Ok(rows.affected() > 0)
    })
}

/// Deletes every row whose expiry is at or before `now`, and returns how many it deleted.
///
/// # Errors
///
/// As [`load`].
fn sweep(conn: &SqliteConn, now: i64) -> io::Result<u64> {
    let rows = conn.query(
        "DELETE FROM nvs_sessions WHERE expires_at <= ?1",
        vec![SqliteValue::Int(now)],
    )?;
    Ok(rows.affected())
}

/// Closes this thread's connection, so a case's scratch directory can be deleted.
#[cfg(test)]
pub(crate) fn closed() {
    HELD.with_borrow_mut(Option::take);
}

/// How many rows the file holds, expired or not, read through a connection of its own.
#[cfg(test)]
pub(crate) fn rows(path: &Path) -> i64 {
    let held = opened(path).expect("a second connection");
    let mut answer = held
        .conn
        .query("SELECT COUNT(*) FROM nvs_sessions", Vec::new())
        .expect("the count");
    match answer.next_row().and_then(|row| row.into_iter().next()) {
        Some(SqliteValue::Int(count)) => count,
        other => panic!("a count is an integer: {other:?}"),
    }
}

/// Moves the expiry of `app`'s row under `id` into the past, as if its lifetime had run out.
#[cfg(test)]
pub(crate) fn lapse(path: &Path, app: &[u8], id: &str) {
    with(path, |held| {
        drop(held.conn.query(
            "UPDATE nvs_sessions SET expires_at = 0 WHERE app = ?1 AND id = ?2",
            vec![
                SqliteValue::Blob(app.to_vec()),
                SqliteValue::Text(id.to_owned()),
            ],
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

    use super::{closed, create, load, replace, rows, sweep, with};

    /// The application every case here writes under.
    const APP: &[u8] = b"/srv/blog";

    /// A store file in a scratch directory one case owns, and the guard that deletes the
    /// directory when the case ends.
    fn store(name: &str) -> (nvs_repo::Scratch, PathBuf) {
        let dir = nvs_repo::scratch(name);
        let path = dir.join("session.sqlite");
        (dir, path)
    }

    /// `rule:http-server/a-session-is-loaded-once-and-written-whole`: two requests of one session
    /// both load it, the second saves and finishes while the first is still running, and then the
    /// first saves. Neither waits for the other, and the record is the last write.
    ///
    /// Two threads are two cores, each with its own connection, which is what two concurrent
    /// requests are. The first thread is held between its load and its save until the second has
    /// finished, so a lock taken at `load` would hang this test instead of passing it.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn two_requests_of_one_session_both_finish_and_the_last_write_wins() {
        const ID: &str = "PmA5tKz1QvR3sYbNqLwXe2";
        let (_dir, path) = store("session-two-requests");
        assert!(
            create(&path, APP, ID, b"start", Duration::from_secs(60))
                .expect("the session is issued")
        );

        let (loaded, first_loaded) = mpsc::channel();
        let (go_on, second_done) = mpsc::channel::<()>();
        let first = {
            let path = path.clone();
            thread::spawn(move || {
                let seen = load(&path, APP, ID).expect("the first request loads");
                loaded.send(()).expect("the test is waiting");
                second_done
                    .recv_timeout(Duration::from_secs(30))
                    .expect("the second request finishes while the first is still open");
                assert!(
                    replace(&path, APP, ID, b"first", Duration::from_secs(60))
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
                let seen = load(&path, APP, ID).expect("the second request loads");
                assert!(
                    replace(&path, APP, ID, b"second", Duration::from_secs(60))
                        .expect("the second saves")
                );
                seen
            })
        };
        assert_eq!(
            second.join().expect("the second request"),
            Some(b"start".to_vec())
        );
        assert_eq!(
            load(&path, APP, ID).expect("a read between the two"),
            Some(b"second".to_vec())
        );
        go_on.send(()).expect("the first request is waiting");
        assert_eq!(
            first.join().expect("the first request"),
            Some(b"start".to_vec())
        );

        assert_eq!(
            load(&path, APP, ID).expect("the record after both"),
            Some(b"first".to_vec()),
            "the last write wins over the whole record"
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
        assert!(
            create(&path, APP, ID, b"user=17", Duration::from_secs(60))
                .expect("the first process writes")
        );
        closed();

        let read = {
            let path = path.clone();
            thread::spawn(move || load(&path, APP, ID).expect("the second process reads"))
        };
        assert_eq!(
            read.join().expect("the second process"),
            Some(b"user=17".to_vec())
        );
    }

    /// `rule:http-server/session-expiry-belongs-to-the-store`: an expired row is absent at once,
    /// and the sweep deletes it while it keeps the live one.
    #[cfg_attr(miri, ignore = "SQLite is C, which Miri cannot interpret")]
    #[test]
    fn an_expired_row_is_absent_and_the_sweep_deletes_it() {
        const LIVE: &str = "LwXe2PmA5tKz1QvR3sYbNq";
        const GONE: &str = "A5tKz1QvR3sYbNqLwXe2Pm";
        let (_dir, path) = store("session-expired");
        assert!(
            create(&path, APP, LIVE, b"live", Duration::from_secs(3600)).expect("a live record")
        );
        assert!(
            create(&path, APP, GONE, b"gone", Duration::ZERO)
                .expect("a record that expires at once")
        );

        assert_eq!(load(&path, APP, GONE).expect("the store answers"), None);
        assert_eq!(rows(&path), 2, "the expired row is still in the file");

        let now = crate::queue::now_millis();
        let deleted = with(&path, |held| sweep(&held.conn, now)).expect("the sweep");
        assert_eq!(deleted, 1);
        assert_eq!(rows(&path), 1);
        assert_eq!(
            load(&path, APP, LIVE).expect("the store answers"),
            Some(b"live".to_vec())
        );
        closed();
    }
}
