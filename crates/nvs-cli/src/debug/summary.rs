//! The viewer's figures over many requests: the summary tiles, the Routes view, the Queries view and
//! the health of the recording
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! **Each answer is a bounded read, and none changes a record.**
//!
//! - `GET /api/summary` counts the requests and the errors in twelve buckets over the five minutes
//!   before `until` (now by default), with the p50, p95 and longest duration. It reads at most the
//!   newest [`SUMMARY_ROWS`] requests of the window on `requests_start`, and `cut` says when it
//!   stopped there.
//! - `GET /api/routes` gives each method and route of the newest [`ROUTE_ROWS`] requests its
//!   count, its errors, its mean query count, its longest duration, when it was last seen, and its
//!   p50 and p95, at most [`ROUTES_MAX`] routes by count, and `partial` says when older requests
//!   were not counted. The percentiles read each route's newest [`ROUTE_DURATIONS`] durations on
//!   the `requests_route` index, and `durations` says how many.
//! - `GET /api/statements` gives each query call site in the newest [`STATEMENT_SCAN`] event ids
//!   its file, line and file hash, the newest statement text, how often it ran, its total and
//!   longest time, how many requests reached it, and in how many of them one statement text ran
//!   there more than once, at most [`STATEMENTS_MAX`] sites by total time, and `partial` says when
//!   older events were not read. A repeat is the one
//!   `rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site` defines, so the editor's
//!   lens and the viewer agree. An edit of the file gives it a new hash, and so a new site.
//! - `GET /api/health` counts the `dropped`, `unparseable` and `quarantine` rows, the records the
//!   dropped rows lost, and says whether a replay target is known.
//!
//! An error is a request with a status of 500 or more, or one that threw. A percentile is the
//! nearest rank of the sorted durations, computed here, because SQLite has none.
//!
//! **What it spends:** each answer reads a fixed number of rows at most, whatever the size of the
//! index, on the one read connection: [`SUMMARY_ROWS`] requests, [`ROUTE_ROWS`] requests and
//! [`ROUTE_DURATIONS`] per listed route, or [`STATEMENT_SCAN`] event ids. One answer holds at most
//! [`SUMMARY_ROWS`] durations, or [`ROUTE_DURATIONS`] for one route at a time, while it is computed,
//! and SQLite's group table for at most the routes or sites those rows name.

use std::time::{SystemTime, UNIX_EPOCH};

use hyper::StatusCode;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use super::api::{Api, answer, error, number};

/// The time the summary covers, in microseconds.
const WINDOW_US: i64 = 300_000_000;

/// How many buckets the summary splits its window into.
const BUCKETS: i64 = 12;

/// The most requests the summary reads.
const SUMMARY_ROWS: i64 = 10_000;

/// The most routes `GET /api/routes` lists.
const ROUTES_MAX: i64 = 200;

/// How many of the newest requests `GET /api/routes` counts.
const ROUTE_ROWS: i64 = 10_000;

/// How many of a route's newest durations its percentiles read.
const ROUTE_DURATIONS: i64 = 500;

/// The most call sites `GET /api/statements` lists.
const STATEMENTS_MAX: i64 = 200;

/// How many of the newest event ids `GET /api/statements` reads.
const STATEMENT_SCAN: i64 = 100_000;

/// `GET /api/summary`.
#[derive(Debug, Serialize)]
struct Summary {
    /// The end of the window, in microseconds since the epoch.
    until_us: i64,
    window_us: i64,
    bucket_us: i64,
    /// The buckets, oldest first.
    buckets: Vec<Bucket>,
    count: i64,
    errors: i64,
    p50_us: Option<i64>,
    p95_us: Option<i64>,
    max_us: Option<i64>,
    /// Whether the window had more than [`SUMMARY_ROWS`] requests, so only the newest are counted.
    cut: bool,
}

/// One bucket of the summary.
#[derive(Clone, Copy, Debug, Default, Serialize)]
struct Bucket {
    count: i64,
    errors: i64,
}

/// `GET /api/routes`.
#[derive(Serialize)]
struct Routes {
    routes: Vec<Route>,
    /// Whether the requests read had more routes than [`ROUTES_MAX`].
    cut: bool,
    /// Whether the index has requests older than the newest [`ROUTE_ROWS`], which are not counted.
    partial: bool,
}

/// One method and route.
#[derive(Serialize)]
struct Route {
    method: String,
    /// The route pattern, or `None` for the requests no route matched.
    route: Option<String>,
    count: i64,
    errors: i64,
    mean_queries: f64,
    max_us: i64,
    /// When its newest request started.
    last_us: i64,
    p50_us: i64,
    p95_us: i64,
    /// How many durations the percentiles stand on, at most [`ROUTE_DURATIONS`].
    durations: i64,
}

/// `GET /api/statements`.
#[derive(Serialize)]
struct Statements {
    statements: Vec<Statement>,
    /// Whether the events read had more sites than [`STATEMENTS_MAX`].
    cut: bool,
    /// Whether the index has events older than the newest [`STATEMENT_SCAN`] ids, which are not
    /// counted.
    partial: bool,
}

/// One query call site.
#[derive(Serialize)]
struct Statement {
    file: String,
    line: i64,
    /// The BLAKE3 of the file's text when the program was compiled.
    hash: String,
    /// The newest statement text that ran there.
    sql: String,
    count: i64,
    total_us: i64,
    max_us: i64,
    /// How many requests ran a statement there.
    requests: i64,
    /// In how many of them one statement text ran there more than once.
    repeated: i64,
}

/// `GET /api/health`.
#[derive(Serialize)]
struct Health {
    /// How many times a writer recorded that it dropped records.
    dropped: i64,
    /// How many records those drops lost.
    dropped_records: i64,
    unparseable: i64,
    quarantine: i64,
    /// Whether this service knows a server to replay a request to.
    replay: bool,
}

/// `GET /api/summary` with `query`, which may name `until`.
pub(crate) fn summary(api: &Api, query: Option<&str>) -> (StatusCode, Vec<u8>) {
    let mut until = None;
    for pair in query
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
    {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "until" => match number(key, value) {
                Ok(value) => until = Some(value),
                Err(message) => return error(StatusCode::BAD_REQUEST, &message),
            },
            _ => {
                return error(
                    StatusCode::BAD_REQUEST,
                    &format!("`{key}` is not a key of the summary."),
                );
            }
        }
    }
    let until = until.unwrap_or_else(now_us);
    answer(api.read(|db| {
        let mut statement = db.prepare_cached(
            "SELECT start_us, duration_us, status >= 500 OR exception_group IS NOT NULL \
             FROM requests WHERE start_us >= ?1 AND start_us < ?2 \
             ORDER BY start_us DESC LIMIT ?3",
        )?;
        let rows = statement
            .query_map(params![until - WINDOW_US, until, SUMMARY_ROWS + 1], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<rusqlite::Result<Vec<(i64, i64, bool)>>>()?;
        Ok(summarise(until, rows))
    }))
}

/// The summary of `rows`, each a start, a duration and whether it failed, newest first, of the
/// window that ends at `until`.
fn summarise(until: i64, mut rows: Vec<(i64, i64, bool)>) -> Summary {
    let cut = rows.len() > usize::try_from(SUMMARY_ROWS).unwrap_or(usize::MAX);
    rows.truncate(usize::try_from(SUMMARY_ROWS).unwrap_or(usize::MAX));
    let bucket_us = WINDOW_US / BUCKETS;
    let from = until - WINDOW_US;
    let mut buckets = vec![Bucket::default(); usize::try_from(BUCKETS).unwrap_or(0)];
    let mut durations = Vec::with_capacity(rows.len());
    let mut errors = 0;
    for (start, duration, failed) in rows {
        let at = usize::try_from((start - from) / bucket_us)
            .unwrap_or(0)
            .min(buckets.len() - 1);
        buckets[at].count += 1;
        if failed {
            buckets[at].errors += 1;
            errors += 1;
        }
        durations.push(duration);
    }
    durations.sort_unstable();
    Summary {
        until_us: until,
        window_us: WINDOW_US,
        bucket_us,
        buckets,
        count: i64::try_from(durations.len()).unwrap_or(i64::MAX),
        errors,
        p50_us: percentile(&durations, 50),
        p95_us: percentile(&durations, 95),
        max_us: durations.last().copied(),
        cut,
    }
}

/// `GET /api/routes`, which takes no key.
pub(crate) fn routes(api: &Api, query: Option<&str>) -> (StatusCode, Vec<u8>) {
    if let Some(message) = no_keys(query, "the routes") {
        return message;
    }
    answer(api.read(|db| {
        let tx = db.transaction()?;
        let mut routes = {
            let mut statement = tx.prepare_cached(
                "SELECT method, route, count(*), \
                 sum(status >= 500 OR exception_group IS NOT NULL), avg(queries), \
                 max(duration_us), max(start_us) \
                 FROM (SELECT method, route, status, exception_group, queries, duration_us, \
                       start_us FROM requests ORDER BY id DESC LIMIT ?1) \
                 GROUP BY method, route \
                 ORDER BY count(*) DESC, method, route LIMIT ?2",
            )?;
            statement
                .query_map([ROUTE_ROWS, ROUTES_MAX + 1], |row| {
                    Ok(Route {
                        method: row.get(0)?,
                        route: row.get(1)?,
                        count: row.get(2)?,
                        errors: row.get(3)?,
                        mean_queries: row.get(4)?,
                        max_us: row.get(5)?,
                        last_us: row.get(6)?,
                        p50_us: 0,
                        p95_us: 0,
                        durations: 0,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let cut = routes.len() > usize::try_from(ROUTES_MAX).unwrap_or(usize::MAX);
        routes.truncate(usize::try_from(ROUTES_MAX).unwrap_or(usize::MAX));
        {
            let mut statement = tx.prepare_cached(
                "SELECT duration_us FROM requests WHERE method = ?1 AND route IS ?2 \
                 ORDER BY start_us DESC LIMIT ?3",
            )?;
            for route in &mut routes {
                let mut durations = statement
                    .query_map(params![route.method, route.route, ROUTE_DURATIONS], |row| {
                        row.get(0)
                    })?
                    .collect::<rusqlite::Result<Vec<i64>>>()?;
                durations.sort_unstable();
                route.p50_us = percentile(&durations, 50).unwrap_or(0);
                route.p95_us = percentile(&durations, 95).unwrap_or(0);
                route.durations = i64::try_from(durations.len()).unwrap_or(i64::MAX);
            }
        }
        let requests: i64 = tx.query_row(
            "SELECT count(*) FROM (SELECT 1 FROM requests ORDER BY id DESC LIMIT ?1)",
            [ROUTE_ROWS + 1],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(Routes {
            routes,
            cut,
            partial: requests > ROUTE_ROWS,
        })
    }))
}

/// `GET /api/statements`, which takes no key.
pub(crate) fn statements(api: &Api, query: Option<&str>) -> (StatusCode, Vec<u8>) {
    if let Some(message) = no_keys(query, "the statements") {
        return message;
    }
    answer(api.read(|db| {
        let tx = db.transaction()?;
        let (newest, oldest): (Option<i64>, Option<i64>) =
            tx.query_row("SELECT max(id), min(id) FROM events", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
        let floor = newest.unwrap_or(0) - STATEMENT_SCAN;
        let sites = {
            let mut statement = tx.prepare_cached(
                "SELECT hash, line, sum(n), sum(spent), max(slowest), count(DISTINCT trace), \
                 count(DISTINCT CASE WHEN n > 1 THEN trace END), max(newest) \
                 FROM (SELECT site_hash AS hash, site_line AS line, trace, count(*) AS n, \
                       sum(max(end_us - start_us, 0)) AS spent, \
                       max(max(end_us - start_us, 0)) AS slowest, max(id) AS newest \
                       FROM events WHERE id > ?1 AND kind = 'query' AND site_hash IS NOT NULL \
                       AND site_line IS NOT NULL \
                       GROUP BY site_hash, site_line, trace, sql) \
                 GROUP BY hash, line ORDER BY sum(spent) DESC, hash, line LIMIT ?2",
            )?;
            statement
                .query_map([floor, STATEMENTS_MAX + 1], |row| {
                    Ok((
                        Statement {
                            file: String::new(),
                            line: row.get(1)?,
                            hash: row.get(0)?,
                            sql: String::new(),
                            count: row.get(2)?,
                            total_us: row.get(3)?,
                            max_us: row.get(4)?,
                            requests: row.get(5)?,
                            repeated: row.get(6)?,
                        },
                        row.get::<_, i64>(7)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let cut = sites.len() > usize::try_from(STATEMENTS_MAX).unwrap_or(usize::MAX);
        let mut statements = Vec::with_capacity(sites.len());
        {
            let mut text = tx.prepare_cached("SELECT site_file, sql FROM events WHERE id = ?1")?;
            for (mut site, id) in sites
                .into_iter()
                .take(usize::try_from(STATEMENTS_MAX).unwrap_or(usize::MAX))
            {
                let found: Option<(Option<String>, Option<String>)> = text
                    .query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
                    .optional()?;
                let (file, sql) = found.unwrap_or_default();
                site.file = file.unwrap_or_default();
                site.sql = sql.unwrap_or_default();
                statements.push(site);
            }
        }
        tx.commit()?;
        Ok(Statements {
            statements,
            cut,
            partial: oldest.is_some_and(|oldest| oldest <= floor),
        })
    }))
}

/// `GET /api/health`, which takes no key. `replay` says whether the service has a target.
pub(crate) fn health(api: &Api, query: Option<&str>, replay: bool) -> (StatusCode, Vec<u8>) {
    if let Some(message) = no_keys(query, "the health") {
        return message;
    }
    answer(api.read(|db| {
        db.query_row(
            "SELECT (SELECT count(*) FROM dropped), \
                    (SELECT coalesce(sum(records), 0) FROM dropped), \
                    (SELECT count(*) FROM unparseable), \
                    (SELECT count(*) FROM quarantine)",
            [],
            |row| {
                Ok(Health {
                    dropped: row.get(0)?,
                    dropped_records: row.get(1)?,
                    unparseable: row.get(2)?,
                    quarantine: row.get(3)?,
                    replay,
                })
            },
        )
    }))
}

/// A `400` when `query` names any key, which a route of `what` does not take.
fn no_keys(query: Option<&str>, what: &str) -> Option<(StatusCode, Vec<u8>)> {
    query
        .unwrap_or_default()
        .split('&')
        .find(|pair| !pair.is_empty())
        .map(|pair| {
            let key = pair.split_once('=').map_or(pair, |(key, _)| key);
            error(
                StatusCode::BAD_REQUEST,
                &format!("`{key}` is not a key of {what}."),
            )
        })
}

/// The nearest-rank `p`th percentile of `sorted`, or `None` when it is empty.
fn percentile(sorted: &[i64], p: usize) -> Option<i64> {
    let rank = (sorted.len() * p).div_ceil(100).max(1);
    sorted.get(rank - 1).copied()
}

/// Now, in microseconds since the epoch.
fn now_us() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_micros()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::{BUCKETS, WINDOW_US, percentile, summarise};

    #[test]
    fn a_percentile_is_the_nearest_rank() {
        assert_eq!(percentile(&[], 50), None);
        assert_eq!(percentile(&[7], 95), Some(7));
        let hundred: Vec<i64> = (1..=100).collect();
        assert_eq!(percentile(&hundred, 50), Some(50));
        assert_eq!(percentile(&hundred, 95), Some(95));
        assert_eq!(percentile(&[1, 2, 3, 4], 50), Some(2));
    }

    #[test]
    fn the_summary_counts_each_request_in_its_bucket() {
        let until = 1_000 * WINDOW_US;
        let bucket = WINDOW_US / BUCKETS;
        let summary = summarise(
            until,
            vec![
                (until - 1, 30, true),
                (until - bucket - 1, 10, false),
                (until - WINDOW_US, 20, false),
            ],
        );
        let counts: Vec<i64> = summary.buckets.iter().map(|b| b.count).collect();
        assert_eq!(counts, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
        assert_eq!(summary.buckets[11].errors, 1);
        assert_eq!((summary.count, summary.errors), (3, 1));
        assert_eq!(
            (summary.p50_us, summary.p95_us, summary.max_us),
            (Some(20), Some(30), Some(30))
        );
        assert!(!summary.cut);
    }
}
