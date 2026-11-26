//! `GET /api/mails` and `GET /api/mail`: the messages a `[mail.<name>] capture = true` block kept,
//! for the viewer's Mail page. A block that sends writes only its `mail` event, so a sent message
//! is never here.
//!
//! **The list is the run list's shape.** A page reads the watermark and the rows in one read
//! transaction, newest first, and pages on the row's id, which only rises. `before=<id>` is the
//! page older than a row the viewer has, and `since=<batch>` is what arrived after a watermark. A
//! row has the block, the subject and the `to` addresses the ingester wrote into their own
//! columns, so the list never reads a message's bodies.
//!
//! **One message is its row id.** `GET /api/mail?id=` gives the whole `mail` line as the ingester
//! kept it: the headers, the text, the HTML as written and as sent, the CSS report and the
//! attachments as base64.
//!
//! **What it spends:** one page of at most [`api::PAGE_MAX`] rows per API request while it is
//! written out, and one message's line, which the spool bounded to its line size.

use rusqlite::{Connection, ToSql, params};
use serde::Serialize;
use serde_json::value::RawValue;

use super::api::{self, PAGE_DEFAULT, PAGE_MAX};

/// One page of `GET /api/mails`.
#[derive(Serialize)]
struct Page {
    /// The last batch this page is complete up to.
    watermark: i64,
    /// The messages, newest first.
    mails: Vec<Row>,
    /// The `before` that gives the next older page, or `None` when this page has the oldest.
    older: Option<i64>,
}

/// One message in the list.
#[derive(Serialize)]
struct Row {
    id: i64,
    batch: i64,
    trace: String,
    at_us: i64,
    /// The `[mail.<name>]` block named.
    endpoint: String,
    subject: String,
    /// The `to` addresses, joined with `, `.
    recipients: String,
}

/// `GET /api/mail`: one message, its whole line.
#[derive(Serialize)]
struct Detail {
    /// The last batch this answer is complete up to.
    watermark: i64,
    id: i64,
    trace: String,
    at_us: i64,
    /// The `mail` line, as the ingester kept it.
    message: Box<RawValue>,
}

/// What `GET /api/mails` was asked for.
#[derive(Debug, Default, PartialEq, Eq)]
struct Filter {
    before: Option<i64>,
    since: Option<i64>,
    limit: Option<i64>,
}

/// The columns [`row`] reads, in its order.
const ROW: &str = "id, batch, trace, at_us, endpoint, subject, recipients";

/// One [`Row`] from the columns [`ROW`] names.
fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok(Row {
        id: row.get(0)?,
        batch: row.get(1)?,
        trace: row.get(2)?,
        at_us: row.get(3)?,
        endpoint: row.get(4)?,
        subject: row.get(5)?,
        recipients: row.get(6)?,
    })
}

/// `GET /api/mails` with `query`: the status and the JSON body.
pub(crate) fn mails(api: &api::Api, query: Option<&str>) -> (hyper::StatusCode, Vec<u8>) {
    match Filter::parse(query.unwrap_or_default()) {
        Ok(filter) => api::answer(api.read(|db| page(db, &filter))),
        Err(message) => api::error(hyper::StatusCode::BAD_REQUEST, &message),
    }
}

/// `GET /api/mail` with `query`: the status and the JSON body.
pub(crate) fn mail(api: &api::Api, query: Option<&str>) -> (hyper::StatusCode, Vec<u8>) {
    let id = match query.unwrap_or_default().split_once('=') {
        Some(("id", value)) => api::number("id", value),
        _ => Err("`id` is the one key, and it is required.".to_owned()),
    };
    let id = match id {
        Ok(id) => id,
        Err(message) => return api::error(hyper::StatusCode::BAD_REQUEST, &message),
    };
    match api.read(|db| detail(db, id)) {
        Ok(Some(detail)) => api::answer(Ok(detail)),
        Ok(None) => api::error(
            hyper::StatusCode::NOT_FOUND,
            "No message in the index has this id.",
        ),
        Err(err) => api::answer::<()>(Err(err)),
    }
}

impl Filter {
    /// Reads `before`, `since` and `limit`. Any other key is an error, so a viewer that sends one
    /// finds out.
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
                _ => return Err(format!("`{key}` is not a filter of the mail list.")),
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
        wheres.push("id < ?");
        values.push(before);
    }
    if let Some(since) = &filter.since {
        wheres.push("batch > ?");
        values.push(since);
    }
    // One row past the page says whether an older page exists.
    let fetched = limit + 1;
    values.push(&fetched);
    let sql = format!(
        "SELECT {ROW} FROM mails {} ORDER BY id DESC LIMIT ?",
        if wheres.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", wheres.join(" AND "))
        }
    );
    let tx = db.transaction()?;
    let watermark = api::watermark(&tx)?;
    let mut mails = {
        let mut statement = tx.prepare_cached(&sql)?;
        statement
            .query_map(values.as_slice(), row)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.commit()?;
    let full = usize::try_from(limit).unwrap_or(usize::MAX);
    let older = if mails.len() > full {
        mails.truncate(full);
        mails.last().map(|row| row.id)
    } else {
        None
    };
    Ok(Page {
        watermark,
        mails,
        older,
    })
}

/// The message with row id `id`, read with the watermark in one transaction, or `None` when the
/// index has no such row.
fn detail(db: &mut Connection, id: i64) -> rusqlite::Result<Option<Detail>> {
    let tx = db.transaction()?;
    let watermark = api::watermark(&tx)?;
    let found = {
        let mut statement =
            tx.prepare_cached("SELECT id, trace, at_us, message FROM mails WHERE id = ?1")?;
        statement
            .query_map(params![id], |row| {
                let message: String = row.get(3)?;
                Ok(Detail {
                    watermark,
                    id: row.get(0)?,
                    trace: row.get(1)?,
                    at_us: row.get(2)?,
                    message: api::tree(&message),
                })
            })?
            .next()
            .transpose()?
    };
    tx.commit()?;
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mail_filter_reads_its_keys_and_names_a_wrong_one() {
        let filter = Filter::parse("before=40&since=3&limit=5").expect("a valid filter");
        assert_eq!(
            filter,
            Filter {
                before: Some(40),
                since: Some(3),
                limit: Some(5),
            }
        );
        assert_eq!(Filter::parse(""), Ok(Filter::default()));
        assert!(Filter::parse("limit=0").is_err());
        assert!(Filter::parse("subject=hi").is_err());
    }
}
