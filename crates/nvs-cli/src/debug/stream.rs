//! `GET /api/stream`, the flat stream of every record, and `GET /api/quarantine`, the files the
//! ingester stopped reading (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`,
//! `rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`).
//!
//! **One page is the newest rows of each table, merged.** Each table the filter keeps gives its
//! newest rows past the cursor through its time index, at most a page of them, and the page is
//! the newest of those. A page reads at most six pages of rows whatever the size of the index.
//! The log view is this stream with `kind=log`.
//!
//! **The filters.** `kind` is a comma list of `request`, `call`, `query`, `spawn`, `gc`, `http`,
//! `dump`, `log`, `dropped` and `unparseable`. `level` keeps the log lines at that level. `trace`
//! keeps the records of one trace id, and `route` those of the requests with that route; neither
//! keeps a `dropped` or `unparseable` row, which belong to no trace. `since=<batch>` is what
//! arrived after a watermark, as in [`super::api`], and `before=<time>.<key>` the page older than
//! a record the viewer has.
//!
//! **Every record names its request.** A record's request is found as [`super::search`] finds
//! it, and its trace id, span id, method, path and status come with it, which is how the log view
//! shows the request a line belongs to.
//!
//! **What it spends:** one page of at most [`super::api::PAGE_MAX`] records while it is written
//! out. A filter that keeps few rows of a large table reads back through that table's time index
//! until it has a page, so a rare `kind` costs more than a common one.

use rusqlite::{Connection, ToSql};
use serde::Serialize;
use serde_json::value::RawValue;

use super::api::{self, PAGE_DEFAULT, PAGE_MAX};

/// The kinds of trace event the `events` table holds.
const EVENT_KINDS: [&str; 5] = ["call", "query", "spawn", "gc", "http"];

/// Every kind a stream filter names.
const KINDS: [&str; 10] = [
    "request",
    "call",
    "query",
    "spawn",
    "gc",
    "http",
    "dump",
    "log",
    "dropped",
    "unparseable",
];

/// One page of `GET /api/stream`.
#[derive(Serialize)]
struct Page {
    /// The last batch this page is complete up to.
    watermark: i64,
    /// The records, newest first.
    records: Vec<Record>,
    /// The `before` that gives the next older page, or `None` when this page has the oldest.
    older: Option<String>,
}

/// One record of any kind.
#[derive(Serialize)]
struct Record {
    /// The record's id times eight plus its table's code: unique across the stream.
    key: i64,
    kind: String,
    at_us: i64,
    batch: i64,
    trace: Option<String>,
    /// A request's method and path, an event's label, a dump's `file:line`, a log line's message,
    /// or the first bytes of an unparseable line.
    text: String,
    /// A log line's level.
    level: Option<String>,
    /// A request's status, or a call's checked-return status.
    status: Option<i64>,
    /// A dump's file, or the file an unparseable line was read from.
    file: Option<String>,
    /// A dump's line.
    line: Option<i64>,
    /// A log line's fields. A `dropped` row's `records` and `files`. An unparseable line's
    /// `reason`, `offset`, `length` and `lossy`.
    fields: Option<Box<RawValue>>,
    /// The request the record belongs to.
    request: Option<Owner>,
}

/// The request a record belongs to.
#[derive(Serialize)]
struct Owner {
    trace: String,
    span: Option<String>,
    method: String,
    path: String,
    status: i64,
}

/// One file the ingester stopped reading, and when it tries again.
#[derive(Serialize)]
struct Quarantined {
    path: String,
    at_us: i64,
    retry_us: i64,
    failures: i64,
    reason: String,
}

/// What `GET /api/stream` was asked for.
#[derive(Debug, Default)]
struct Filter {
    kinds: Vec<&'static str>,
    level: Option<String>,
    route: Option<String>,
    trace: Option<String>,
    since: Option<i64>,
    before: Option<(i64, i64)>,
    limit: Option<i64>,
}

/// `GET /api/stream` with `query`: the status and the JSON body.
pub(crate) fn stream(api: &api::Api, query: Option<&str>) -> (hyper::StatusCode, Vec<u8>) {
    let filter = match Filter::parse(query.unwrap_or_default()) {
        Ok(filter) => filter,
        Err(message) => return api::error(hyper::StatusCode::BAD_REQUEST, &message),
    };
    api::answer(api.read(|db| page(db, &filter)))
}

/// `GET /api/quarantine`: every quarantined file, the newest failure first.
pub(crate) fn quarantine(api: &api::Api) -> (hyper::StatusCode, Vec<u8>) {
    api::answer(api.read(|db| {
        let mut statement = db.prepare_cached(
            "SELECT path, at_us, retry_us, failures, reason FROM quarantine ORDER BY at_us DESC",
        )?;
        let files = statement
            .query_map([], |row| {
                Ok(Quarantined {
                    path: row.get(0)?,
                    at_us: row.get(1)?,
                    retry_us: row.get(2)?,
                    failures: row.get(3)?,
                    reason: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(serde_json::json!({ "files": files }))
    }))
}

impl Filter {
    /// Reads `kind`, `level`, `route`, `trace`, `since`, `before` and `limit`. Any other key is an
    /// error.
    fn parse(query: &str) -> Result<Self, String> {
        let mut filter = Self::default();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "kind" => {
                    for kind in value.split(',') {
                        let Some(known) = KINDS.iter().find(|known| **known == kind) else {
                            return Err(format!("`{kind}` is not a kind of record."));
                        };
                        filter.kinds.push(known);
                    }
                }
                "level" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_lowercase()) => {
                    filter.level = Some(value.to_owned());
                }
                "level" => {
                    return Err("`level` must be a lower-case level, such as `error`.".into());
                }
                "route" => filter.route = Some(percent_decoded(value)?),
                "trace" => filter.trace = Some(api::hex(key, value, 32)?),
                "since" => filter.since = Some(api::number(key, value)?),
                "before" => {
                    filter.before = Some(
                        value
                            .split_once('.')
                            .and_then(|(at, key)| Some((at.parse().ok()?, key.parse().ok()?)))
                            .ok_or("`before` must be the `older` value of a page.")?,
                    );
                }
                "limit" => {
                    let limit = api::number(key, value)?;
                    if !(1..=PAGE_MAX).contains(&limit) {
                        return Err(format!("`limit` must be from 1 to {PAGE_MAX}."));
                    }
                    filter.limit = Some(limit);
                }
                _ => return Err(format!("`{key}` is not a filter of the stream.")),
            }
        }
        Ok(filter)
    }

    /// Whether the filter keeps records of `kind`.
    fn keeps(&self, kind: &str) -> bool {
        self.kinds.is_empty() || self.kinds.contains(&kind)
    }
}

/// A route from a URL, with `%XX` escapes decoded.
fn percent_decoded(value: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut rest = value.bytes();
    while let Some(byte) = rest.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let digit = |b: Option<u8>| char::from(b?).to_digit(16);
        match (digit(rest.next()), digit(rest.next())) {
            (Some(high), Some(low)) => bytes.push(u8::try_from(high * 16 + low).unwrap_or(0)),
            _ => return Err("`route` has a `%` that is not followed by two hex digits.".into()),
        }
    }
    String::from_utf8(bytes).map_err(|_| "`route` is not UTF-8.".to_owned())
}

/// One table's part of a page: its select of the [`Record`] columns, its code, its time column,
/// and whether it has a trace id.
struct Source {
    table: &'static str,
    code: i64,
    time: &'static str,
    traced: bool,
    columns: &'static str,
}

/// Every table the stream reads, in the [`Record`] column order.
const SOURCES: [Source; 6] = [
    Source {
        table: "requests",
        code: 1,
        time: "start_us",
        traced: true,
        columns: "'request', start_us, batch, trace, method || ' ' || path, NULL, status, NULL, \
                  NULL, NULL, writer, seq",
    },
    Source {
        table: "events",
        code: 2,
        time: "at_us",
        traced: true,
        columns: "kind, at_us, batch, trace, label, NULL, status, NULL, NULL, NULL, writer, seq",
    },
    Source {
        table: "dumps",
        code: 3,
        time: "at_us",
        traced: true,
        columns: "'dump', at_us, batch, trace, file || ':' || line, NULL, NULL, file, line, NULL, \
                  writer, seq",
    },
    Source {
        table: "logs",
        code: 4,
        time: "at_us",
        traced: true,
        columns: "'log', at_us, batch, trace, message, level, NULL, NULL, NULL, fields, writer, seq",
    },
    Source {
        table: "dropped",
        code: 6,
        time: "at_us",
        traced: false,
        columns: "'dropped', at_us, batch, NULL, '', NULL, NULL, NULL, NULL, \
                  json_object('records', records, 'files', files), NULL, NULL",
    },
    Source {
        table: "unparseable",
        code: 7,
        time: "at_us",
        traced: false,
        columns: "'unparseable', at_us, batch, NULL, excerpt, NULL, NULL, source, NULL, \
                  json_object('reason', reason, 'offset', offset, 'length', length, \
                  'lossy', json(CASE WHEN lossy THEN 'true' ELSE 'false' END)), NULL, NULL",
    },
];

/// The page `filter` names, read with the watermark in one transaction.
fn page(db: &mut Connection, filter: &Filter) -> rusqlite::Result<Page> {
    let limit = filter.limit.unwrap_or(PAGE_DEFAULT);
    let mut parts = Vec::new();
    for source in &SOURCES {
        if let Some(part) = part(source, filter) {
            parts.push(part);
        }
    }
    let tx = db.transaction()?;
    let watermark = api::watermark(&tx)?;
    let mut records = if parts.is_empty() {
        Vec::new()
    } else {
        // `?1` and `?2` are the cursor, `?3` `since`, `?4` the trace id, `?5` the route, `?6` the
        // level and `?7` the rows each part reads. Every part names `?7`, so all seven are bound.
        let sql = format!(
            "WITH t (key, kind, at_us, batch, trace, text, level, status, file, line, fields, \
             writer, seq) AS ({}) \
             SELECT t.key, t.kind, t.at_us, t.batch, t.trace, t.text, t.level, t.status, t.file, \
             t.line, t.fields, r.trace, r.span, r.method, r.path, r.status \
             FROM t LEFT JOIN requests r ON r.id = CASE WHEN t.key % 8 = 1 THEN t.key / 8 \
             ELSE coalesce(\
                (SELECT o.id FROM requests o WHERE o.writer = t.writer AND o.seq > t.seq \
                 AND o.trace = t.trace ORDER BY o.seq LIMIT 1), \
                (SELECT o.id FROM requests o WHERE o.trace = t.trace ORDER BY o.id DESC LIMIT 1)) \
             END ORDER BY t.at_us DESC, t.key DESC LIMIT ?7",
            parts.join(" UNION ALL ")
        );
        let (at, key) = filter.before.unzip();
        let values: [&dyn ToSql; 7] = [
            &at,
            &key,
            &filter.since,
            &filter.trace,
            &filter.route,
            &filter.level,
            &(limit + 1),
        ];
        let mut statement = tx.prepare_cached(&sql)?;
        statement
            .query_map(values.as_slice(), record)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.commit()?;
    let full = usize::try_from(limit).unwrap_or(usize::MAX);
    let older = (records.len() > full).then(|| {
        records.truncate(full);
        records
            .last()
            .map(|record| format!("{}.{}", record.at_us, record.key))
            .unwrap_or_default()
    });
    Ok(Page {
        watermark,
        records,
        older,
    })
}

/// The select of `source`'s newest rows that `filter` keeps, or `None` when it keeps none.
fn part(source: &Source, filter: &Filter) -> Option<String> {
    let Source {
        table,
        code,
        time,
        traced,
        columns,
    } = source;
    let mut wheres = Vec::new();
    match *table {
        "events" => {
            let kinds: Vec<String> = EVENT_KINDS
                .iter()
                .filter(|kind| filter.keeps(kind))
                .map(|kind| format!("'{kind}'"))
                .collect();
            if kinds.is_empty() {
                return None;
            }
            if kinds.len() < EVENT_KINDS.len() {
                wheres.push(format!("kind IN ({})", kinds.join(", ")));
            }
        }
        "requests" if !filter.keeps("request") => return None,
        "dumps" if !filter.keeps("dump") => return None,
        "logs" if !filter.keeps("log") => return None,
        "dropped" if !filter.keeps("dropped") => return None,
        "unparseable" if !filter.keeps("unparseable") => return None,
        _ => {}
    }
    if filter.level.is_some() {
        if *table != "logs" {
            return None;
        }
        wheres.push("level = ?6".into());
    }
    if filter.trace.is_some() || filter.route.is_some() {
        if !traced {
            return None;
        }
        if filter.trace.is_some() {
            wheres.push("trace = ?4".into());
        }
        if filter.route.is_some() {
            wheres.push(if *table == "requests" {
                "route = ?5".into()
            } else {
                "trace IN (SELECT trace FROM requests WHERE route = ?5)".into()
            });
        }
    }
    if filter.since.is_some() {
        wheres.push("batch > ?3".into());
    }
    if filter.before.is_some() {
        wheres.push(format!(
            "{time} <= ?1 AND ({time} < ?1 OR id * 8 + {code} < ?2)"
        ));
    }
    Some(format!(
        "SELECT * FROM (SELECT id * 8 + {code} AS key, {columns} FROM {table} {} \
         ORDER BY {time} DESC, id DESC LIMIT ?7)",
        if wheres.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", wheres.join(" AND "))
        }
    ))
}

/// One [`Record`] from the page's columns, in their order.
fn record(row: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    let fields: Option<String> = row.get(10)?;
    let owner: Option<String> = row.get(11)?;
    Ok(Record {
        key: row.get(0)?,
        kind: row.get(1)?,
        at_us: row.get(2)?,
        batch: row.get(3)?,
        trace: row.get(4)?,
        text: row.get(5)?,
        level: row.get(6)?,
        status: row.get(7)?,
        file: row.get(8)?,
        line: row.get(9)?,
        fields: fields.as_deref().map(api::tree),
        request: match owner {
            Some(trace) => Some(Owner {
                trace,
                span: row.get(12)?,
                method: row.get(13)?,
                path: row.get(14)?,
                status: row.get(15)?,
            }),
            None => None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_filter_reads_its_keys_and_names_a_wrong_one() {
        let filter = Filter::parse(
            "kind=log,query&level=error&route=%2Fitems%2F%7Bid%7D&before=1700000000000000.34",
        )
        .expect("a valid filter");
        assert_eq!(filter.kinds, ["log", "query"]);
        assert_eq!(filter.route.as_deref(), Some("/items/{id}"));
        assert_eq!(filter.before, Some((1_700_000_000_000_000, 34)));
        assert!(Filter::parse("kind=exception").is_err());
        assert!(Filter::parse("level=ERROR").is_err());
        assert!(Filter::parse("before=12").is_err());
        assert!(Filter::parse("order=asc").is_err());
    }

    #[test]
    fn a_level_keeps_the_log_table_alone() {
        let filter = Filter::parse("level=error").expect("a valid filter");
        let kept: Vec<&str> = SOURCES
            .iter()
            .filter(|source| part(source, &filter).is_some())
            .map(|source| source.table)
            .collect();
        assert_eq!(kept, ["logs"]);
        let traced = Filter::parse("trace=4bf92f3577b34da6a3ce929d0e0e4736").expect("valid");
        assert!(
            part(&SOURCES[4], &traced).is_none(),
            "a dropped row has no trace"
        );
    }
}
