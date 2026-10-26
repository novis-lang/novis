//! The viewer's JSON API over the index
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! **One read connection, and every list is a page.** The listener's site holds one [`Api`], and
//! every connection thread takes its connection in turn: a page is one indexed query, so the wait
//! is short, and the index has one reader beside its writer. The index is in write-ahead mode, so
//! that reader never waits on the ingester and the ingester never waits on it.
//!
//! **The watermark says when, the cursor says where.** `/api/events` pushes the last committed
//! batch. `GET /api/requests` reads the watermark and the page in one read transaction, so the
//! number it gives is the batch the page is complete up to. The newest page has no cursor;
//! `before=<id>` is the page older than a row the viewer has, and `since=<batch>` is what arrived
//! after a watermark the viewer had. A request's id only rises (the index module's *The viewer's
//! cursor*), so neither moves a row the viewer already shows.
//!
//! **What it spends:** one SQLite connection and its page cache for the service, and one page of
//! at most [`PAGE_MAX`] rows per API request while it is written out.

use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use hyper::StatusCode;
use rusqlite::{Connection, OpenFlags, ToSql};
use serde::Serialize;

/// The rows in a page when the viewer names no `limit`.
const PAGE_DEFAULT: i64 = 100;

/// The most rows in one page.
pub(crate) const PAGE_MAX: i64 = 500;

/// The index, opened once for reading.
pub(crate) struct Api {
    db: Mutex<Connection>,
}

/// One page of `GET /api/requests`.
#[derive(Serialize)]
struct Page {
    /// The last batch this page is complete up to.
    watermark: i64,
    /// The requests, newest first.
    requests: Vec<Row>,
    /// The `before` that gives the next older page, or `None` when this page has the oldest.
    older: Option<i64>,
}

/// One request in the list: the columns the rule names for the main view.
#[derive(Serialize)]
struct Row {
    id: i64,
    batch: i64,
    trace: String,
    span: Option<String>,
    start_us: i64,
    duration_us: i64,
    method: String,
    route: Option<String>,
    path: String,
    status: i64,
    queries: i64,
    dumps: i64,
    logs: i64,
    exception: Option<Threw>,
}

/// The exception a request did not catch: its class and the site of its `throw`.
#[derive(Serialize)]
struct Threw {
    class: String,
    file: String,
    line: i64,
}

/// What `GET /api/requests` was asked for.
#[derive(Default)]
struct Filter {
    before: Option<i64>,
    since: Option<i64>,
    trace: Option<String>,
    span: Option<String>,
    limit: Option<i64>,
}

impl Api {
    /// Opens the index at `path` for reading. The ingester has created it already.
    ///
    /// # Errors
    ///
    /// When SQLite cannot open the file.
    pub(crate) fn open(path: &Path) -> rusqlite::Result<Self> {
        let db = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        db.busy_timeout(Duration::from_secs(1))?;
        Ok(Self { db: Mutex::new(db) })
    }

    /// `GET /api/requests` with `query`: the status and the JSON body.
    pub(crate) fn requests(&self, query: Option<&str>) -> (StatusCode, Vec<u8>) {
        let filter = match Filter::parse(query.unwrap_or_default()) {
            Ok(filter) => filter,
            Err(message) => return error(StatusCode::BAD_REQUEST, &message),
        };
        let mut db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
        match page(&mut db, &filter) {
            Ok(page) => match serde_json::to_vec(&page) {
                Ok(body) => (StatusCode::OK, body),
                Err(err) => error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
            },
            Err(err) => error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("The index cannot be read: {err}"),
            ),
        }
    }
}

impl Filter {
    /// Reads `before`, `since`, `trace`, `span` and `limit`. Any other key is an error, so a
    /// viewer that sends one finds out.
    fn parse(query: &str) -> Result<Self, String> {
        let mut filter = Self::default();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "before" => filter.before = Some(number(key, value)?),
                "since" => filter.since = Some(number(key, value)?),
                "limit" => {
                    let limit = number(key, value)?;
                    if !(1..=PAGE_MAX).contains(&limit) {
                        return Err(format!("`limit` must be from 1 to {PAGE_MAX}."));
                    }
                    filter.limit = Some(limit);
                }
                "trace" => filter.trace = Some(hex(key, value, 32)?),
                "span" => filter.span = Some(hex(key, value, 16)?),
                _ => return Err(format!("`{key}` is not a filter of this list.")),
            }
        }
        Ok(filter)
    }
}

/// `value` as a whole number that is not negative.
fn number(key: &str, value: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .ok()
        .filter(|number| *number >= 0)
        .ok_or_else(|| format!("`{key}` must be a whole number."))
}

/// `value` when it is `digits` lower-case hex digits.
fn hex(key: &str, value: &str, digits: usize) -> Result<String, String> {
    if value.len() == digits
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        Ok(value.to_owned())
    } else {
        Err(format!("`{key}` must be {digits} lower-case hex digits."))
    }
}

/// The page `filter` names, read with the watermark in one transaction.
fn page(db: &mut Connection, filter: &Filter) -> rusqlite::Result<Page> {
    let limit = filter.limit.unwrap_or(PAGE_DEFAULT);
    let mut wheres = Vec::new();
    let mut values: Vec<&dyn ToSql> = Vec::new();
    if let Some(before) = &filter.before {
        wheres.push("r.id < ?");
        values.push(before);
    }
    if let Some(since) = &filter.since {
        wheres.push("r.batch > ?");
        values.push(since);
    }
    if let Some(trace) = &filter.trace {
        wheres.push("r.trace = ?");
        values.push(trace);
    }
    if let Some(span) = &filter.span {
        wheres.push("r.span = ?");
        values.push(span);
    }
    // One row past the page says whether an older page exists.
    let fetched = limit + 1;
    values.push(&fetched);
    let sql = format!(
        "SELECT r.id, r.batch, r.trace, r.span, r.start_us, r.duration_us, r.method, r.route, \
         r.path, r.status, r.queries, r.dumps, r.logs, g.class, g.file, g.line \
         FROM requests r LEFT JOIN exception_groups g ON g.id = r.exception_group \
         {} ORDER BY r.id DESC LIMIT ?",
        if wheres.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", wheres.join(" AND "))
        }
    );
    let tx = db.transaction()?;
    let watermark: i64 = tx.query_row(
        "SELECT value FROM meta WHERE key = 'watermark'",
        [],
        |row| row.get(0),
    )?;
    let mut requests = {
        let mut statement = tx.prepare_cached(&sql)?;
        statement
            .query_map(values.as_slice(), |row| {
                let class: Option<String> = row.get(13)?;
                Ok(Row {
                    id: row.get(0)?,
                    batch: row.get(1)?,
                    trace: row.get(2)?,
                    span: row.get(3)?,
                    start_us: row.get(4)?,
                    duration_us: row.get(5)?,
                    method: row.get(6)?,
                    route: row.get(7)?,
                    path: row.get(8)?,
                    status: row.get(9)?,
                    queries: row.get(10)?,
                    dumps: row.get(11)?,
                    logs: row.get(12)?,
                    exception: match class {
                        Some(class) => Some(Threw {
                            class,
                            file: row.get(14)?,
                            line: row.get(15)?,
                        }),
                        None => None,
                    },
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.commit()?;
    let full = usize::try_from(limit).unwrap_or(usize::MAX);
    let older = (requests.len() > full).then(|| {
        requests.truncate(full);
        requests.last().map_or(0, |row| row.id)
    });
    Ok(Page {
        watermark,
        requests,
        older,
    })
}

/// A JSON error body with `message`.
fn error(status: StatusCode, message: &str) -> (StatusCode, Vec<u8>) {
    let body = serde_json::json!({ "error": message });
    (status, body.to_string().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filter_reads_its_keys_and_names_a_wrong_one() {
        let filter =
            Filter::parse("before=40&since=3&limit=20&trace=4bf92f3577b34da6a3ce929d0e0e4736")
                .expect("a valid filter");
        assert_eq!(filter.before, Some(40));
        assert_eq!(filter.since, Some(3));
        assert_eq!(filter.limit, Some(20));
        assert!(filter.trace.is_some());
        assert!(Filter::parse("").is_ok());
        assert!(Filter::parse("limit=0").is_err());
        assert!(Filter::parse("limit=501").is_err());
        assert!(Filter::parse("before=-1").is_err());
        assert!(Filter::parse("span=00F067AA0BA902B7").is_err());
        assert!(Filter::parse("order=asc").is_err());
    }
}
