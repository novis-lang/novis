//! `GET /api/runs` and `GET /api/run`: the scheduled jobs, queue jobs and command runs the index
//! keeps (`rule:testing/a-run-is-recorded-like-a-request`).
//!
//! **A run's list is the request list's shape.** A page reads the watermark and the rows in one
//! read transaction, newest first, and pages on the run's id, which only rises as a request's
//! does. `before=<id>` is the page older than a row the viewer has, and `since=<batch>` is what
//! arrived after a watermark. `of` and `name` read the `runs_name` index. `outcome`, `min_us` and
//! `min_memory` add one predicate each, and a page with any filter reads at most
//! [`api::FILTER_SCAN`] ids below its cursor, as a filtered request page does.
//!
//! **One run is its trace id.** A run writes its line under its own trace, so every event, dump
//! and log of that trace is the run's. `GET /api/run?trace=` gives the newest run of the trace,
//! its uncaught exception with its stack, and its records in time order, each list bounded as a
//! request's is.
//!
//! **What it spends:** one page of at most [`api::PAGE_MAX`] rows per API request while it is
//! written out, and one run's bounded lists.

use rusqlite::{Connection, ToSql, params};
use serde::Serialize;

use super::api::{self, PAGE_DEFAULT, PAGE_MAX};

/// One page of `GET /api/runs`.
#[derive(Serialize)]
struct Page {
    /// The last batch this page is complete up to.
    watermark: i64,
    /// The runs, newest first.
    runs: Vec<Row>,
    /// The `before` that gives the next older page, or `None` when this page has the oldest.
    older: Option<i64>,
}

/// One run in the list.
#[derive(Serialize)]
pub(super) struct Row {
    pub(super) id: i64,
    batch: i64,
    pub(super) trace: String,
    /// `schedule`, `queue` or `command`.
    of: String,
    /// The schedule's name, the job's script, or the command and its script.
    name: String,
    pub(super) start_us: i64,
    duration_us: i64,
    /// `ok`, `failed` or `cancelled`.
    outcome: String,
    queries: i64,
    dumps: i64,
    logs: i64,
    exception: Option<api::Threw>,
    /// The summed durations of its events by kind, in microseconds.
    time_us: api::TimeByKind,
    /// The most times one statement text ran at one call site in it.
    repeats: i64,
    /// The highest its memory was, and its ceiling, in bytes, where the line recorded them.
    memory_peak: Option<i64>,
    memory_limit: Option<i64>,
}

/// `GET /api/run`: one run and everything it recorded, each list in time order.
#[derive(Serialize)]
struct Detail {
    /// The last batch this answer is complete up to.
    watermark: i64,
    run: Row,
    /// The exception it did not catch, with its message and its stack.
    exception: Option<api::Exception>,
    events: Vec<api::Event>,
    dumps: Vec<api::Dump>,
    logs: Vec<api::Log>,
    /// Which lists stopped at their most rows.
    cut: Cut,
}

/// Whether each list of a [`Detail`] stopped at its most rows.
#[derive(Serialize)]
struct Cut {
    events: bool,
    dumps: bool,
    logs: bool,
}

/// What `GET /api/runs` was asked for.
#[derive(Debug, Default, PartialEq)]
struct Filter {
    before: Option<i64>,
    since: Option<i64>,
    limit: Option<i64>,
    of: Option<&'static str>,
    name: Option<String>,
    outcome: Option<&'static str>,
    min_us: Option<i64>,
    min_memory: Option<i64>,
}

/// The columns [`row`] reads, over `runs u` joined to its `exception_groups g`.
pub(super) const ROW: &str = "u.id, u.batch, u.trace, u.of, u.name, u.start_us, u.duration_us, \
                   u.outcome, u.queries, u.dumps, u.logs, g.class, g.file, g.line, \
                   u.db_us, u.http_us, u.cache_us, u.session_us, u.file_us, u.gc_us, u.spawn_us, \
                   u.repeats, u.memory_peak, u.memory_limit";

/// One [`Row`] from the columns [`ROW`] names, in its order.
pub(super) fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok(Row {
        id: row.get(0)?,
        batch: row.get(1)?,
        trace: row.get(2)?,
        of: row.get(3)?,
        name: row.get(4)?,
        start_us: row.get(5)?,
        duration_us: row.get(6)?,
        outcome: row.get(7)?,
        queries: row.get(8)?,
        dumps: row.get(9)?,
        logs: row.get(10)?,
        exception: api::threw(row, 11)?,
        time_us: api::time_by_kind(row, 14)?,
        repeats: row.get(21)?,
        memory_peak: row.get(22)?,
        memory_limit: row.get(23)?,
    })
}

/// `GET /api/runs` with `query`: the status and the JSON body.
pub(crate) fn runs(api: &api::Api, query: Option<&str>) -> (hyper::StatusCode, Vec<u8>) {
    match Filter::parse(query.unwrap_or_default()) {
        Ok(filter) => api::answer(api.read(|db| page(db, &filter))),
        Err(message) => api::error(hyper::StatusCode::BAD_REQUEST, &message),
    }
}

/// `GET /api/run` with `query`: the status and the JSON body.
pub(crate) fn run(api: &api::Api, query: Option<&str>) -> (hyper::StatusCode, Vec<u8>) {
    let trace = match query.unwrap_or_default().split_once('=') {
        Some(("trace", value)) => api::hex("trace", value, 32),
        _ => Err("`trace` is the one key, and it is required.".to_owned()),
    };
    let trace = match trace {
        Ok(trace) => trace,
        Err(message) => return api::error(hyper::StatusCode::BAD_REQUEST, &message),
    };
    match api.read(|db| detail(db, &trace)) {
        Ok(Some(detail)) => api::answer(Ok(detail)),
        Ok(None) => api::error(
            hyper::StatusCode::NOT_FOUND,
            "No run in the index has this trace id.",
        ),
        Err(err) => api::answer::<()>(Err(err)),
    }
}

impl Filter {
    /// Whether it filters at all, so the page reads only through [`api::FILTER_SCAN`].
    const fn scans(&self) -> bool {
        self.of.is_some()
            || self.name.is_some()
            || self.outcome.is_some()
            || self.min_us.is_some()
            || self.min_memory.is_some()
    }

    /// Reads `before`, `since`, `limit`, `of`, `name`, `outcome`, `min_us` and `min_memory`. Any
    /// other key is an error, so a viewer that sends one finds out.
    fn parse(query: &str) -> Result<Self, String> {
        let mut filter = Self::default();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "before" => filter.before = Some(api::number(key, value)?),
                "since" => filter.since = Some(api::number(key, value)?),
                "limit" => {
                    let limit = api::number(key, value)?;
                    if !(1..=PAGE_MAX).contains(&limit) {
                        return Err(format!("`limit` must be from 1 to {PAGE_MAX}."));
                    }
                    filter.limit = Some(limit);
                }
                "of" => {
                    filter.of = Some(match value {
                        "schedule" => "schedule",
                        "queue" => "queue",
                        "command" => "command",
                        _ => {
                            return Err("`of` must be `schedule`, `queue` or `command`.".to_owned());
                        }
                    });
                }
                "name" => filter.name = Some(super::search::decode(key, value)?),
                "outcome" => {
                    filter.outcome = Some(match value {
                        "ok" => "ok",
                        "failed" => "failed",
                        "cancelled" => "cancelled",
                        _ => {
                            return Err(
                                "`outcome` must be `ok`, `failed` or `cancelled`.".to_owned()
                            );
                        }
                    });
                }
                "min_us" => filter.min_us = Some(api::number(key, value)?),
                "min_memory" => filter.min_memory = Some(api::number(key, value)?),
                _ => return Err(format!("`{key}` is not a filter of the runs.")),
            }
        }
        Ok(filter)
    }
}

/// The page `filter` names, read with the watermark in one transaction.
fn page(db: &mut Connection, filter: &Filter) -> rusqlite::Result<Page> {
    let limit = filter.limit.unwrap_or(PAGE_DEFAULT);
    let mut wheres = Vec::new();
    let mut values: Vec<&dyn ToSql> = Vec::new();
    if let Some(before) = &filter.before {
        wheres.push("u.id < ?");
        values.push(before);
    }
    if let Some(since) = &filter.since {
        wheres.push("u.batch > ?");
        values.push(since);
    }
    if let Some(of) = &filter.of {
        wheres.push("u.of = ?");
        values.push(of);
    }
    if let Some(name) = &filter.name {
        wheres.push("u.name = ?");
        values.push(name);
    }
    if let Some(outcome) = &filter.outcome {
        wheres.push("u.outcome = ?");
        values.push(outcome);
    }
    if let Some(min_us) = &filter.min_us {
        wheres.push("u.duration_us >= ?");
        values.push(min_us);
    }
    if let Some(min_memory) = &filter.min_memory {
        wheres.push("u.memory_peak >= ?");
        values.push(min_memory);
    }
    let tx = db.transaction()?;
    // The request page's floor: a filtered page reads at most `FILTER_SCAN` ids below its cursor.
    let (floor, oldest) = if filter.scans() {
        let (newest, oldest): (Option<i64>, Option<i64>) = match &filter.since {
            Some(since) => tx.query_row(
                "SELECT (SELECT max(id) FROM runs), min(id) FROM runs WHERE batch > ?",
                [since],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?,
            None => tx.query_row("SELECT max(id), min(id) FROM runs", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?,
        };
        let top = filter
            .before
            .unwrap_or_else(|| newest.unwrap_or(0).saturating_add(1));
        (Some(top.saturating_sub(api::FILTER_SCAN)), oldest)
    } else {
        (None, None)
    };
    if let Some(floor) = &floor {
        wheres.push("u.id >= ?");
        values.push(floor);
    }
    // One row past the page says whether an older page exists.
    let fetched = limit + 1;
    values.push(&fetched);
    let sql = format!(
        "SELECT {ROW} FROM runs u LEFT JOIN exception_groups g ON g.id = u.exception_group \
         {} ORDER BY u.id DESC LIMIT ?",
        if wheres.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", wheres.join(" AND "))
        }
    );
    let watermark = api::watermark(&tx)?;
    let mut runs = {
        let mut statement = tx.prepare_cached(&sql)?;
        statement
            .query_map(values.as_slice(), row)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.commit()?;
    let full = usize::try_from(limit).unwrap_or(usize::MAX);
    let older = if runs.len() > full {
        runs.truncate(full);
        Some(runs.last().map_or(0, |row| row.id))
    } else {
        floor.filter(|floor| oldest.is_some_and(|oldest| oldest < *floor))
    };
    Ok(Page {
        watermark,
        runs,
        older,
    })
}

/// The newest run of `trace` and everything it recorded, read with the watermark in one
/// transaction, or `None` when no run has that trace id.
fn detail(db: &mut Connection, trace: &str) -> rusqlite::Result<Option<Detail>> {
    let tx = db.transaction()?;
    let watermark = api::watermark(&tx)?;
    let found = {
        let mut statement = tx.prepare_cached(&format!(
            "SELECT {ROW} FROM runs u LEFT JOIN exception_groups g ON g.id = u.exception_group \
             WHERE u.trace = ?1 ORDER BY u.id DESC LIMIT 1"
        ))?;
        statement
            .query_map(params![trace], row)?
            .next()
            .transpose()?
    };
    let Some(run) = found else {
        return Ok(None);
    };
    let exception = api::exception(&tx, trace, None, Some(run.id))?;
    let of = api::Records::all(trace);
    let (events, events_cut) = api::events(&tx, of)?;
    let (dumps, dumps_cut) = api::dumps(&tx, of)?;
    let (logs, logs_cut) = api::logs(&tx, of, run.start_us, run.start_us + run.duration_us)?;
    tx.commit()?;
    Ok(Some(Detail {
        watermark,
        run,
        exception,
        events,
        dumps,
        logs,
        cut: Cut {
            events: events_cut,
            dumps: dumps_cut,
            logs: logs_cut,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_runs_filter_reads_its_keys_and_names_a_wrong_one() {
        let filter = Filter::parse("of=queue&name=jobs%2Fmail.nvs&outcome=failed&limit=5")
            .expect("a valid filter");
        assert_eq!(filter.of, Some("queue"));
        assert_eq!(filter.name.as_deref(), Some("jobs/mail.nvs"));
        assert_eq!(filter.outcome, Some("failed"));
        assert_eq!(filter.limit, Some(5));
        assert!(filter.scans());
        assert!(!Filter::parse("").expect("the newest page").scans());
        assert!(Filter::parse("of=request").is_err());
        assert!(Filter::parse("outcome=crashed").is_err());
        assert!(Filter::parse("method=GET").is_err());
    }
}
