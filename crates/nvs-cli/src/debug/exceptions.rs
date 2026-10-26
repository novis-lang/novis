//! `GET /api/exceptions`: one row per exception class and throw site, and the requests of one
//! (`rule:testing/an-exception-is-one-row-per-class-and-site`).
//!
//! **The groups are the ingester's.** It writes one `exception_groups` row per class and site,
//! with its count and its first and last time, as it inserts the exception (the index module's
//! schema). A page reads those rows newest last seen first, so the list is one indexed query. A
//! group changes in place as it is thrown again, so the viewer reads the newest page again at each
//! watermark; `before=<last seen>.<id>` is the page after a row it has.
//!
//! **Each row reads its latest throws, bounded.** The latest message, the chart and the routes come
//! from at most [`SAMPLE_MAX`] of the group's newest throws, read on the group's index. The chart
//! is [`CHART_BUCKETS`] counts of equal time from the group's first throw to its last, and
//! `sampled` says that throws older than the sample are not in it.
//!
//! **One group is its class, file and line.** `?class=&file=&line=` names a group the way its
//! site does, so a link to it stays valid when the deletion window or a discard gives the group
//! another id. It lists the group's requests newest first, and `before=<older>` gives the next
//! page.
//!
//! **What it spends:** one page of at most [`PAGE_MAX`] groups while it is written out, and for
//! each group one indexed read of at most [`SAMPLE_MAX`] rows.

use rusqlite::{Connection, OptionalExtension as _, params};
use serde::Serialize;

use super::api::{self, PAGE_DEFAULT, PAGE_MAX, ROW, Row};
use super::search::decode;

/// The most throws of one group a row reads for its message, chart and routes.
const SAMPLE_MAX: i64 = 1_000;

/// The counts in a group's chart.
const CHART_BUCKETS: usize = 24;

/// The most routes a row lists.
const ROUTES_MAX: usize = 5;

/// One page of `GET /api/exceptions`.
#[derive(Serialize)]
struct Page {
    /// The last batch this page is complete up to.
    watermark: i64,
    /// The groups, newest last seen first.
    groups: Vec<Group>,
    /// The `before` that gives the next page, or `None` when this page has the last group.
    older: Option<String>,
}

/// One class and throw site.
#[derive(Serialize)]
struct Group {
    id: i64,
    class: String,
    file: String,
    line: i64,
    count: i64,
    first_us: i64,
    last_us: i64,
    /// The message of the latest throw still in the index.
    message: Option<String>,
    /// The throws in each of [`CHART_BUCKETS`] equal spans from `first_us` to `last_us`.
    chart: Vec<i64>,
    /// The routes of the latest throws, newest first.
    routes: Vec<String>,
    /// Whether the group has throws older than the ones the chart and routes read.
    sampled: bool,
}

/// `GET /api/exceptions?class=&file=&line=`: one group and a page of its requests.
#[derive(Serialize)]
struct Requests {
    /// The last batch this page is complete up to.
    watermark: i64,
    group: Group,
    /// The requests that threw it, newest first.
    requests: Vec<Row>,
    /// The `before` that gives the next older page, or `None` when this page has the oldest.
    older: Option<i64>,
}

/// What `GET /api/exceptions` was asked for.
#[derive(Debug, Default, PartialEq)]
struct Query {
    /// The group's class, file and line.
    site: Option<(String, String, i64)>,
    /// `<last seen>.<id>` for the groups, an exception id for one group's requests.
    before: Option<String>,
    limit: Option<i64>,
}

/// `GET /api/exceptions` with `query`: the status and the JSON body.
pub(crate) fn exceptions(api: &api::Api, query: Option<&str>) -> (hyper::StatusCode, Vec<u8>) {
    let query = match Query::parse(query.unwrap_or_default()) {
        Ok(query) => query,
        Err(message) => return api::error(hyper::StatusCode::BAD_REQUEST, &message),
    };
    match &query.site {
        None => {
            let before = match query.before.as_deref().map(cursor).transpose() {
                Ok(before) => before,
                Err(message) => return api::error(hyper::StatusCode::BAD_REQUEST, &message),
            };
            api::answer(api.read(|db| groups(db, before, query.limit)))
        }
        Some(site) => {
            let before = match query
                .before
                .as_deref()
                .map(|value| api::number("before", value))
            {
                None => None,
                Some(Ok(before)) => Some(before),
                Some(Err(message)) => return api::error(hyper::StatusCode::BAD_REQUEST, &message),
            };
            match api.read(|db| requests(db, site, before, query.limit)) {
                Ok(Some(page)) => api::answer(Ok(page)),
                Ok(None) => api::error(
                    hyper::StatusCode::NOT_FOUND,
                    "No exception in the index has this class and site.",
                ),
                Err(err) => api::answer::<()>(Err(err)),
            }
        }
    }
}

impl Query {
    /// Reads `class`, `file` and `line`, which come together, `before` and `limit`. Any other key
    /// is an error.
    fn parse(text: &str) -> Result<Self, String> {
        let (mut class, mut file, mut line) = (None, None, None);
        let mut query = Self::default();
        for pair in text.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "class" => class = Some(decode(key, value)?),
                "file" => file = Some(decode(key, value)?),
                "line" => line = Some(api::number(key, value)?),
                "before" => query.before = Some(value.to_owned()),
                "limit" => {
                    let limit = api::number(key, value)?;
                    if !(1..=PAGE_MAX).contains(&limit) {
                        return Err(format!("`limit` must be from 1 to {PAGE_MAX}."));
                    }
                    query.limit = Some(limit);
                }
                _ => return Err(format!("`{key}` is not a filter of the exceptions.")),
            }
        }
        query.site = match (class, file, line) {
            (Some(class), Some(file), Some(line)) => Some((class, file, line)),
            (None, None, None) => None,
            _ => return Err("`class`, `file` and `line` name a group together.".to_owned()),
        };
        Ok(query)
    }
}

/// `<last seen>.<id>`, the cursor of the groups.
fn cursor(value: &str) -> Result<(i64, i64), String> {
    value
        .split_once('.')
        .and_then(|(time, id)| Some((time.parse().ok()?, id.parse().ok()?)))
        .ok_or_else(|| "`before` must be the `older` value of a page.".to_owned())
}

/// The columns [`group`] reads, over `exception_groups`.
const GROUP: &str = "id, class, file, line, count, first_us, last_us";

/// One group from the columns [`GROUP`] names, and its latest throws.
fn group(db: &Connection, row: &rusqlite::Row<'_>) -> rusqlite::Result<Group> {
    let mut group = Group {
        id: row.get(0)?,
        class: row.get(1)?,
        file: row.get(2)?,
        line: row.get(3)?,
        count: row.get(4)?,
        first_us: row.get(5)?,
        last_us: row.get(6)?,
        message: None,
        chart: vec![0; CHART_BUCKETS],
        routes: Vec::new(),
        sampled: false,
    };
    let mut statement = db.prepare_cached(
        "SELECT e.message, r.start_us, r.route FROM exceptions e \
         JOIN requests r ON r.id = e.request \
         WHERE e.exception_group = ?1 ORDER BY e.id DESC LIMIT ?2",
    )?;
    let mut rows = statement.query(params![group.id, SAMPLE_MAX + 1])?;
    let span = (group.last_us - group.first_us).max(1);
    let mut read = 0;
    while let Some(row) = rows.next()? {
        read += 1;
        if read > SAMPLE_MAX {
            group.sampled = true;
            break;
        }
        if group.message.is_none() {
            group.message = Some(row.get(0)?);
        }
        let start: i64 = row.get(1)?;
        let at = i128::from(start - group.first_us) * CHART_BUCKETS as i128 / i128::from(span);
        let bucket = usize::try_from(at.max(0))
            .unwrap_or(0)
            .min(CHART_BUCKETS - 1);
        group.chart[bucket] += 1;
        if let Some(route) = row.get::<_, Option<String>>(2)?
            && group.routes.len() < ROUTES_MAX
            && !group.routes.contains(&route)
        {
            group.routes.push(route);
        }
    }
    Ok(group)
}

/// The page of groups after `before`, read with the watermark in one transaction.
fn groups(
    db: &mut Connection,
    before: Option<(i64, i64)>,
    limit: Option<i64>,
) -> rusqlite::Result<Page> {
    let limit = limit.unwrap_or(PAGE_DEFAULT);
    let tx = db.transaction()?;
    let watermark = api::watermark(&tx)?;
    let (time, id) = before.unwrap_or((i64::MAX, i64::MAX));
    let mut groups = {
        let mut statement = tx.prepare_cached(&format!(
            "SELECT {GROUP} FROM exception_groups WHERE (last_us, id) < (?1, ?2) \
             ORDER BY last_us DESC, id DESC LIMIT ?3"
        ))?;
        let mut rows = statement.query(params![time, id, limit + 1])?;
        let mut groups = Vec::new();
        while let Some(row) = rows.next()? {
            groups.push(group(&tx, row)?);
        }
        groups
    };
    tx.commit()?;
    let full = usize::try_from(limit).unwrap_or(usize::MAX);
    let older = (groups.len() > full).then(|| {
        groups.truncate(full);
        groups.last().map_or_else(String::new, |group| {
            format!("{}.{}", group.last_us, group.id)
        })
    });
    Ok(Page {
        watermark,
        groups,
        older,
    })
}

/// The group `site` names and the page of its requests before the exception id `before`, or
/// `None` when no group has that site.
fn requests(
    db: &mut Connection,
    site: &(String, String, i64),
    before: Option<i64>,
    limit: Option<i64>,
) -> rusqlite::Result<Option<Requests>> {
    let limit = limit.unwrap_or(PAGE_DEFAULT);
    let tx = db.transaction()?;
    let watermark = api::watermark(&tx)?;
    let group = tx
        .query_row(
            &format!(
                "SELECT {GROUP} FROM exception_groups WHERE class = ?1 AND file = ?2 AND line = ?3"
            ),
            params![site.0, site.1, site.2],
            |row| group(&tx, row),
        )
        .optional()?;
    let Some(group) = group else {
        return Ok(None);
    };
    let mut found = {
        let mut statement = tx.prepare_cached(&format!(
            "SELECT {ROW}, e.id FROM exceptions e JOIN requests r ON r.id = e.request \
             LEFT JOIN exception_groups g ON g.id = r.exception_group \
             WHERE e.exception_group = ?1 AND e.id < ?2 ORDER BY e.id DESC LIMIT ?3"
        ))?;
        statement
            .query_map(
                params![group.id, before.unwrap_or(i64::MAX), limit + 1],
                |found| Ok((api::row(found)?, found.get::<_, i64>(16)?)),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.commit()?;
    let full = usize::try_from(limit).unwrap_or(usize::MAX);
    let older = (found.len() > full).then(|| {
        found.truncate(full);
        found.last().map_or(0, |(_, id)| *id)
    });
    Ok(Some(Requests {
        watermark,
        group,
        requests: found.into_iter().map(|(row, _)| row).collect(),
        older,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_is_named_by_its_class_file_and_line_together() {
        let query = Query::parse("class=Shop%5COutOfStock&file=%2Fapp%2Fshop.nvs&line=12&limit=5")
            .expect("a valid query");
        assert_eq!(
            query.site,
            Some((
                "Shop\\OutOfStock".to_owned(),
                "/app/shop.nvs".to_owned(),
                12
            ))
        );
        assert_eq!(query.limit, Some(5));
        assert_eq!(Query::parse("").expect("the groups"), Query::default());
        assert!(Query::parse("class=Shop%5COutOfStock&line=12").is_err());
        assert!(Query::parse("limit=0").is_err());
        assert!(Query::parse("order=count").is_err());
        assert_eq!(cursor("1700.4"), Ok((1700, 4)));
        assert!(cursor("1700").is_err());
    }
}
