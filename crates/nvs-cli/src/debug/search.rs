//! `GET /api/search`: one search box over every request, query, dump, log line and exception
//! (`rule:testing/the-viewer-searches-every-record-and-saves-a-filter`).
//!
//! **One FTS5 query, then the rows it names.** The index's `search` table has one row per
//! searchable record, and its rowid is the record's id times eight plus its table's code (the index
//! module's *What the triggers keep*). The words and phrases of a search become one `MATCH`, and
//! each match is joined to its record by primary key, so the cost of a search grows with the
//! records that match it and never with the records kept. What the table holds is the record after
//! `rule:errors/record-transformations`, so a `secret` value is a redacted node and is not found.
//!
//! **What a search is.** A bare word matches a word that starts with it, a quoted phrase matches
//! as a phrase, and the words are joined by AND. A field term narrows: `status:500` or `status:5xx`,
//! `method:POST`, `route:/items/{id}`, `kind:query` (also `request`, `dump`, `log` and
//! `exception`), `level:error`, `duration:>200ms` (with `>`, `>=`, `<`, `<=` or `=`, and `us`, `ms`
//! or `s`) and `trace:<id>`. A bare trace id, or a pasted `traceresponse` value, is a `trace:` term,
//! and the value also names the span. A term with a name that is not a field is a phrase.
//!
//! **Grouped by request, newest first.** A match belongs to the request of its trace id, split by
//! writer as [`super::api`]'s *One request* says; a tailed log line of a shared trace id goes to the
//! newest request of it. A match no request owns, such as a log line written outside one, is a
//! group of its own. Groups come newest first by the request's start or the record's time, and a
//! group lists at most [`HITS_MAX`] matches in time order and counts the rest. A search with only
//! field terms lists the requests they name, with no matches: `kind:` then says the request has a
//! record of that kind, and `level:` that it wrote a log line at that level.
//!
//! **The cursor is a time and a key.** `older` is `<time>.<key>`, the last group's time and its
//! key (the request's search rowid, or the match's), and `before=` gives the groups after it in
//! that order, so two groups that started in the same microsecond are both listed.
//!
//! **What it spends:** SQLite's sort of the matches, which spills to a temporary file past its
//! cache, and one page of at most [`super::api::PAGE_MAX`] groups while it is written out.

use rusqlite::{Connection, ToSql};
use serde::Serialize;

use super::api::{self, PAGE_DEFAULT, PAGE_MAX, Row};

/// The most matches a group lists.
const HITS_MAX: usize = 20;

/// The longest search, in bytes.
const QUERY_MAX: usize = 2_000;

/// The kinds of record the `search` table holds.
const KINDS: [&str; 5] = ["request", "query", "dump", "log", "exception"];

/// One page of `GET /api/search`.
#[derive(Serialize)]
struct Results {
    /// The last batch this page is complete up to.
    watermark: i64,
    groups: Vec<Group>,
    /// The `before` that gives the next page, or `None` when this page has the last group.
    older: Option<String>,
    /// The request the search names, when it is a trace id alone and one request has it.
    opens: Option<Opens>,
}

/// The matches that belong to one request, or one match that belongs to none.
#[derive(Serialize)]
struct Group {
    request: Option<Row>,
    hits: Vec<Hit>,
    /// The matches past [`HITS_MAX`].
    more: i64,
}

/// One record a search matched, with the words around the match.
#[derive(Serialize)]
struct Hit {
    kind: String,
    id: i64,
    at_us: i64,
    /// The text around the match. U+0002 starts a matched word and U+0003 ends it.
    snippet: String,
}

/// The request a trace-id search opens.
#[derive(Serialize)]
struct Opens {
    trace: String,
    span: Option<String>,
}

/// How `status:` compares.
#[derive(Debug, PartialEq)]
enum Status {
    Exact(i64),
    /// `5xx`: from `500` to `599`.
    Class(i64),
}

/// A parsed search: its words and phrases, its field terms and its page.
#[derive(Debug, Default, PartialEq)]
struct Query {
    /// Each word as `(text, phrase)`.
    words: Vec<(String, bool)>,
    status: Option<Status>,
    method: Option<String>,
    route: Option<String>,
    kind: Option<String>,
    level: Option<String>,
    /// The comparison and the microseconds.
    duration: Option<(&'static str, i64)>,
    trace: Option<String>,
    span: Option<String>,
    before: Option<(i64, i64)>,
    limit: Option<i64>,
}

/// `GET /api/search` with `query`: the status and the JSON body.
pub(crate) fn search(api: &api::Api, query: Option<&str>) -> (hyper::StatusCode, Vec<u8>) {
    let query = match Query::from_url(query.unwrap_or_default()) {
        Ok(query) => query,
        Err(message) => return api::error(hyper::StatusCode::BAD_REQUEST, &message),
    };
    api::answer(api.read(|db| results(db, &query)))
}

impl Query {
    /// Reads `q`, `before` and `limit` from a URL's query. Any other key is an error.
    fn from_url(url: &str) -> Result<Self, String> {
        let (mut text, mut before, mut limit) = (String::new(), None, None);
        for pair in url.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "q" => text = decode(key, value)?,
                "before" => before = Some(cursor(value)?),
                "limit" => {
                    let value = api::number(key, value)?;
                    if !(1..=PAGE_MAX).contains(&value) {
                        return Err(format!("`limit` must be from 1 to {PAGE_MAX}."));
                    }
                    limit = Some(value);
                }
                _ => return Err(format!("`{key}` is not a key of a search.")),
            }
        }
        if text.len() > QUERY_MAX {
            return Err(format!("A search is at most {QUERY_MAX} bytes."));
        }
        let mut query = Self::parse(&text)?;
        query.before = before;
        query.limit = limit;
        Ok(query)
    }

    /// Reads the search box's text into words, phrases and field terms.
    fn parse(text: &str) -> Result<Self, String> {
        let mut query = Self::default();
        for (token, quoted) in tokens(text) {
            if quoted {
                query.words.push((token, true));
                continue;
            }
            if let Some((trace, span)) = trace_value(&token) {
                query.set_trace(trace, span)?;
                continue;
            }
            let Some((field, value)) = token.split_once(':') else {
                query.words.push((token, false));
                continue;
            };
            match field {
                "status" => set(&mut query.status, "status", status(value)?)?,
                "method" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_alphabetic()) => {
                    set(&mut query.method, "method", value.to_ascii_uppercase())?;
                }
                "method" => return Err("`method:` needs a method, such as `method:POST`.".into()),
                "route" if !value.is_empty() => set(&mut query.route, "route", value.to_owned())?,
                "route" => {
                    return Err("`route:` needs a route, such as `route:/items/{id}`.".into());
                }
                "kind" if KINDS.contains(&value) => set(&mut query.kind, "kind", value.to_owned())?,
                "kind" => {
                    return Err(format!(
                        "`kind:{value}` is not a kind a search finds. Use `request`, `query`, \
                         `dump`, `log` or `exception`."
                    ));
                }
                "level" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_alphabetic()) => {
                    set(&mut query.level, "level", value.to_ascii_lowercase())?;
                }
                "level" => return Err("`level:` needs a level, such as `level:error`.".into()),
                "duration" => set(&mut query.duration, "duration", duration(value)?)?,
                "trace" => match trace_value(value) {
                    Some((trace, span)) => query.set_trace(trace, span)?,
                    None => {
                        return Err(
                            "`trace:` needs a trace id of 32 hex digits, or a `traceresponse` value."
                                .into(),
                        );
                    }
                },
                _ => query.words.push((token, true)),
            }
        }
        Ok(query)
    }

    fn set_trace(&mut self, trace: String, span: Option<String>) -> Result<(), String> {
        set(&mut self.trace, "trace", trace)?;
        if let Some(span) = span {
            self.span = Some(span);
        }
        Ok(())
    }

    /// The `MATCH` expression of the words, or `None` when there are none to match.
    fn fts(&self) -> Option<String> {
        let terms: Vec<String> = self
            .words
            .iter()
            .filter(|(word, _)| word.chars().any(char::is_alphanumeric))
            .map(|(word, phrase)| {
                let quoted = format!("\"{}\"", word.replace('"', "\"\""));
                if *phrase { quoted } else { quoted + "*" }
            })
            .collect();
        (!terms.is_empty()).then(|| terms.join(" "))
    }

    /// Whether the search is a trace id and nothing else.
    fn is_trace_alone(&self) -> bool {
        self.trace.is_some()
            && self.words.is_empty()
            && self.status.is_none()
            && self.method.is_none()
            && self.route.is_none()
            && self.kind.is_none()
            && self.level.is_none()
            && self.duration.is_none()
    }
}

/// Sets `slot` once, so a field written twice is an error rather than one of them quietly lost.
fn set<T>(slot: &mut Option<T>, field: &str, value: T) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("`{field}:` is given twice."));
    }
    *slot = Some(value);
    Ok(())
}

/// The search box's text split on white space, with each `"..."` one token, also after a field's
/// colon. Each token says whether it was a quoted phrase.
fn tokens(text: &str) -> Vec<(String, bool)> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        let Some(&first) = chars.peek() else {
            return tokens;
        };
        let mut token = String::new();
        if first == '"' {
            chars.next();
            token.extend(chars.by_ref().take_while(|c| *c != '"'));
            tokens.push((token, true));
            continue;
        }
        while let Some(c) = chars.next_if(|c| !c.is_whitespace()) {
            if c == '"' && token.ends_with(':') {
                token.extend(chars.by_ref().take_while(|c| *c != '"'));
                break;
            }
            token.push(c);
        }
        tokens.push((token, false));
    }
}

/// A trace id of 32 hex digits, or a `traceresponse` value `00-<trace>-<span>-<flags>`, as the
/// trace id and the span id in lower case.
fn trace_value(value: &str) -> Option<(String, Option<String>)> {
    let is_hex = |text: &str, digits: usize| {
        text.len() == digits && text.bytes().all(|b| b.is_ascii_hexdigit())
    };
    if is_hex(value, 32) {
        return Some((value.to_ascii_lowercase(), None));
    }
    let mut parts = value.split('-');
    match (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) {
        (Some(version), Some(trace), Some(span), Some(flags), None)
            if is_hex(version, 2) && is_hex(trace, 32) && is_hex(span, 16) && is_hex(flags, 2) =>
        {
            Some((trace.to_ascii_lowercase(), Some(span.to_ascii_lowercase())))
        }
        _ => None,
    }
}

/// `500` or `5xx`.
fn status(value: &str) -> Result<Status, String> {
    let wrong = || format!("`status:{value}` is not a status. Use three digits, or `5xx`.");
    match value.as_bytes() {
        [first @ b'1'..=b'5', b'x' | b'X', b'x' | b'X'] => {
            Ok(Status::Class(i64::from(first - b'0') * 100))
        }
        [b'1'..=b'5', b'0'..=b'9', b'0'..=b'9'] => {
            value.parse().map(Status::Exact).map_err(|_| wrong())
        }
        _ => Err(wrong()),
    }
}

/// `>200ms` as `(">", 200_000)`.
fn duration(value: &str) -> Result<(&'static str, i64), String> {
    let wrong = || {
        format!(
            "`duration:{value}` is not a duration. Write it like `duration:>200ms`, with `us`, `ms` or `s`."
        )
    };
    let (op, rest) = [">=", "<=", ">", "<", "="]
        .into_iter()
        .find_map(|op| value.strip_prefix(op).map(|rest| (op, rest)))
        .unwrap_or(("=", value));
    let (number, scale) = if let Some(number) = rest.strip_suffix("us") {
        (number, 1)
    } else if let Some(number) = rest.strip_suffix("ms") {
        (number, 1_000)
    } else if let Some(number) = rest.strip_suffix('s') {
        (number, 1_000_000)
    } else {
        return Err(wrong());
    };
    let number: i64 = number.parse().ok().filter(|n| *n >= 0).ok_or_else(wrong)?;
    Ok((op, number.checked_mul(scale).ok_or_else(wrong)?))
}

/// `<time>.<key>`, the cursor the module doc names.
fn cursor(value: &str) -> Result<(i64, i64), String> {
    value
        .split_once('.')
        .and_then(|(time, key)| Some((time.parse().ok()?, key.parse().ok()?)))
        .ok_or_else(|| "`before` must be the `older` value of a page.".to_owned())
}

/// The URL query value of `key` with `+` as a space and `%XX` escapes decoded.
pub(super) fn decode(key: &str, value: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut rest = value.bytes();
    while let Some(byte) = rest.next() {
        match byte {
            b'+' => bytes.push(b' '),
            b'%' => {
                let hex = [rest.next(), rest.next()];
                let digit = |b: Option<u8>| char::from(b?).to_digit(16);
                match (digit(hex[0]), digit(hex[1])) {
                    (Some(high), Some(low)) => {
                        bytes.push(u8::try_from(high * 16 + low).unwrap_or(0))
                    }
                    _ => {
                        return Err(format!(
                            "`{key}` has a `%` that is not followed by two hex digits."
                        ));
                    }
                }
            }
            byte => bytes.push(byte),
        }
    }
    String::from_utf8(bytes).map_err(|_| format!("`{key}` is not UTF-8."))
}

/// Which request a record of the table aliased `t` belongs to: the next request line its writer
/// wrote for the trace, else the newest request of the trace.
const OWNER: &str = "coalesce(\
    (SELECT o.id FROM requests o WHERE o.writer = t.writer AND o.seq > t.seq AND o.trace = t.trace \
     ORDER BY o.seq LIMIT 1), \
    (SELECT o.id FROM requests o WHERE o.trace = t.trace ORDER BY o.id DESC LIMIT 1))";

/// The page `query` names, read with the watermark in one transaction.
fn results(db: &mut Connection, query: &Query) -> rusqlite::Result<Results> {
    let limit = usize::try_from(query.limit.unwrap_or(PAGE_DEFAULT)).unwrap_or(usize::MAX);
    let tx = db.transaction()?;
    let watermark = api::watermark(&tx)?;
    let (groups, older) = match query.fts() {
        Some(fts) => matched(&tx, query, &fts, limit)?,
        None => requests(&tx, query, limit)?,
    };
    tx.commit()?;
    let opens = match groups.as_slice() {
        [
            Group {
                request: Some(row), ..
            },
        ] if query.is_trace_alone() && older.is_none() => Some(Opens {
            trace: row.trace.clone(),
            span: row.span.clone(),
        }),
        _ => None,
    };
    Ok(Results {
        watermark,
        groups,
        older,
        opens,
    })
}

/// The request-level conditions of `query` on `requests r`, pushed onto `wheres` and `values`.
fn request_terms(query: &Query, wheres: &mut Vec<String>, values: &mut Vec<Box<dyn ToSql>>) {
    match query.status {
        Some(Status::Exact(status)) => {
            wheres.push("r.status = ?".into());
            values.push(Box::new(status));
        }
        Some(Status::Class(low)) => {
            wheres.push("r.status BETWEEN ? AND ?".into());
            values.push(Box::new(low));
            values.push(Box::new(low + 99));
        }
        None => {}
    }
    for (column, value) in [
        ("r.method", &query.method),
        ("r.route", &query.route),
        ("r.span", &query.span),
    ] {
        if let Some(value) = value {
            wheres.push(format!("{column} = ?"));
            values.push(Box::new(value.clone()));
        }
    }
    if let Some((op, us)) = query.duration {
        wheres.push(format!("r.duration_us {op} ?"));
        values.push(Box::new(us));
    }
}

/// The groups of a search with words: each match joined to its record and its request.
fn matched(
    tx: &rusqlite::Transaction<'_>,
    query: &Query,
    fts: &str,
    limit: usize,
) -> rusqlite::Result<(Vec<Group>, Option<String>)> {
    let mut values: Vec<Box<dyn ToSql>> = vec![Box::new(fts.to_owned())];
    let mut wheres = Vec::new();
    request_terms(query, &mut wheres, &mut values);
    if let Some(trace) = &query.trace {
        wheres.push("coalesce(r.trace, x.trace) = ?".into());
        values.push(Box::new(trace.clone()));
    }
    for (column, value) in [("x.kind", &query.kind), ("x.level", &query.level)] {
        if let Some(value) = value {
            wheres.push(format!("{column} = ?"));
            values.push(Box::new(value.clone()));
        }
    }
    let at = "coalesce(r.start_us, x.at_us)";
    let key = "coalesce(r.id * 8 + 1, x.rowid)";
    if let Some((time, last)) = query.before {
        wheres.push(format!("({at} < ? OR ({at} = ? AND {key} < ?))"));
        values.extend([
            Box::new(time) as Box<dyn ToSql>,
            Box::new(time),
            Box::new(last),
        ]);
    }
    let sql = format!(
        "WITH m AS MATERIALIZED ( \
            SELECT rowid, snippet(search, 0, char(2), char(3), '…', 12) AS snip \
            FROM search WHERE search MATCH ?), \
         x AS ( \
            SELECT m.rowid, 'request' AS kind, t.id, t.start_us AS at_us, t.trace, \
                NULL AS level, t.id AS request, m.snip \
                FROM m JOIN requests t ON t.id = m.rowid / 8 WHERE m.rowid % 8 = 1 \
            UNION ALL SELECT m.rowid, t.kind, t.id, t.start_us, t.trace, NULL, {OWNER}, m.snip \
                FROM m JOIN events t ON t.id = m.rowid / 8 WHERE m.rowid % 8 = 2 \
            UNION ALL SELECT m.rowid, 'dump', t.id, t.at_us, t.trace, NULL, {OWNER}, m.snip \
                FROM m JOIN dumps t ON t.id = m.rowid / 8 WHERE m.rowid % 8 = 3 \
            UNION ALL SELECT m.rowid, 'log', t.id, t.at_us, t.trace, t.level, {OWNER}, m.snip \
                FROM m JOIN logs t ON t.id = m.rowid / 8 WHERE m.rowid % 8 = 4 \
            UNION ALL SELECT m.rowid, 'exception', t.id, o.start_us + o.duration_us, t.trace, NULL, \
                t.request, m.snip \
                FROM m JOIN exceptions t ON t.id = m.rowid / 8 JOIN requests o ON o.id = t.request \
                WHERE m.rowid % 8 = 5) \
         SELECT {ROW}, x.kind, x.id, x.at_us, x.snip, {at}, {key} \
         FROM x LEFT JOIN requests r ON r.id = x.request \
         LEFT JOIN exception_groups g ON g.id = r.exception_group \
         {} ORDER BY {at} DESC, {key} DESC, x.at_us, x.rowid",
        if wheres.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", wheres.join(" AND "))
        },
        ROW = api::ROW,
    );
    let mut statement = tx.prepare(&sql)?;
    let mut rows = statement.query(rusqlite::params_from_iter(values.iter()))?;
    let mut groups: Vec<Group> = Vec::new();
    let mut last: Option<(i64, i64)> = None;
    while let Some(found) = rows.next()? {
        let group_at: i64 = found.get(20)?;
        let group_key: i64 = found.get(21)?;
        if last != Some((group_at, group_key)) {
            if groups.len() == limit {
                return Ok((groups, last.map(|(at, key)| format!("{at}.{key}"))));
            }
            let request = match found.get::<_, Option<i64>>(0)? {
                Some(_) => Some(api::row(found)?),
                None => None,
            };
            groups.push(Group {
                request,
                hits: Vec::new(),
                more: 0,
            });
            last = Some((group_at, group_key));
        }
        let Some(group) = groups.last_mut() else {
            break;
        };
        if group.hits.len() == HITS_MAX {
            group.more += 1;
            continue;
        }
        group.hits.push(Hit {
            kind: found.get(16)?,
            id: found.get(17)?,
            at_us: found.get(18)?,
            snippet: found.get(19)?,
        });
    }
    Ok((groups, None))
}

/// The groups of a search with field terms alone: the requests they name, newest first.
fn requests(
    tx: &rusqlite::Transaction<'_>,
    query: &Query,
    limit: usize,
) -> rusqlite::Result<(Vec<Group>, Option<String>)> {
    let mut values: Vec<Box<dyn ToSql>> = Vec::new();
    let mut wheres = Vec::new();
    request_terms(query, &mut wheres, &mut values);
    if let Some(trace) = &query.trace {
        wheres.push("r.trace = ?".into());
        values.push(Box::new(trace.clone()));
    }
    // A record counts for the request that owns it, so two requests that share a trace id do not
    // each count the other's.
    let owned = |table: &str, and: &str| {
        format!("EXISTS (SELECT 1 FROM {table} t WHERE t.trace = r.trace{and} AND {OWNER} = r.id)")
    };
    match query.kind.as_deref() {
        Some("query") => wheres.push(owned("events", " AND t.kind = 'query'")),
        Some("dump") => wheres.push(owned("dumps", "")),
        Some("log") => wheres.push(owned("logs", "")),
        Some("exception") => wheres.push("r.exception_group IS NOT NULL".into()),
        _ => {}
    }
    if let Some(level) = &query.level {
        wheres.push(owned("logs", " AND t.level = ?"));
        values.push(Box::new(level.clone()));
    }
    if let Some((time, last)) = query.before {
        wheres.push("(r.start_us < ? OR (r.start_us = ? AND r.id * 8 + 1 < ?))".into());
        values.extend([
            Box::new(time) as Box<dyn ToSql>,
            Box::new(time),
            Box::new(last),
        ]);
    }
    values.push(Box::new(
        i64::try_from(limit).unwrap_or(i64::MAX).saturating_add(1),
    ));
    let sql = format!(
        "SELECT {ROW} FROM requests r LEFT JOIN exception_groups g ON g.id = r.exception_group \
         {} ORDER BY r.start_us DESC, r.id DESC LIMIT ?",
        if wheres.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", wheres.join(" AND "))
        },
        ROW = api::ROW,
    );
    let mut statement = tx.prepare(&sql)?;
    let mut rows: Vec<Row> = statement
        .query_map(rusqlite::params_from_iter(values.iter()), api::row)?
        .collect::<rusqlite::Result<_>>()?;
    let older = (rows.len() > limit).then(|| {
        rows.truncate(limit);
        rows.last()
            .map(|row| format!("{}.{}", row.start_us, row.id * 8 + 1))
            .unwrap_or_default()
    });
    let groups = rows
        .into_iter()
        .map(|row| Group {
            request: Some(row),
            hits: Vec::new(),
            more: 0,
        })
        .collect();
    Ok((groups, older))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_reads_words_phrases_and_each_field_term() {
        let query = Query::parse(
            "lamp \"out of stock\" status:5xx method:post route:/items/{id} kind:dump \
             level:Error duration:>=200ms",
        )
        .expect("a valid search");
        assert_eq!(
            query.words,
            [
                ("lamp".to_owned(), false),
                ("out of stock".to_owned(), true)
            ]
        );
        assert_eq!(query.status, Some(Status::Class(500)));
        assert_eq!(query.method.as_deref(), Some("POST"));
        assert_eq!(query.route.as_deref(), Some("/items/{id}"));
        assert_eq!(query.kind.as_deref(), Some("dump"));
        assert_eq!(query.level.as_deref(), Some("error"));
        assert_eq!(query.duration, Some((">=", 200_000)));
        assert_eq!(query.fts().as_deref(), Some("\"lamp\"* \"out of stock\""));
        assert_eq!(
            Query::parse("route:\"/a b\"")
                .expect("valid")
                .route
                .as_deref(),
            Some("/a b")
        );
        assert_eq!(
            Query::parse("status:404").expect("valid").status,
            Some(Status::Exact(404))
        );
    }

    #[test]
    fn a_pasted_traceresponse_names_the_trace_and_the_span() {
        let query = Query::parse("00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01")
            .expect("a valid search");
        assert_eq!(
            query.trace.as_deref(),
            Some("4bf92f3577b34da6a3ce929d0e0e4736")
        );
        assert_eq!(query.span.as_deref(), Some("00f067aa0ba902b7"));
        assert!(query.is_trace_alone());
        let bare = Query::parse("trace:4bf92f3577b34da6a3ce929d0e0e4736").expect("valid");
        assert_eq!(bare.span, None);
    }

    #[test]
    fn a_wrong_term_names_itself() {
        for (text, names) in [
            ("status:600", "status:600"),
            ("kind:call", "kind:call"),
            ("duration:200", "duration:200"),
            ("trace:abc", "trace:"),
            ("status:200 status:500", "twice"),
        ] {
            let message = Query::parse(text).expect_err(text);
            assert!(message.contains(names), "{text}: {message}");
        }
        assert_eq!(
            Query::parse("note:lamp").expect("valid").words,
            [("note:lamp".to_owned(), true)]
        );
        assert_eq!(Query::parse("- !").expect("valid").fts(), None);
    }

    #[test]
    fn a_url_query_is_decoded() {
        let query = Query::from_url("q=status%3A500+lamp&before=1700000002000000.25&limit=5")
            .expect("a valid query");
        assert_eq!(query.status, Some(Status::Exact(500)));
        assert_eq!(query.words, [("lamp".to_owned(), false)]);
        assert_eq!(query.before, Some((1_700_000_002_000_000, 25)));
        assert!(Query::from_url("q=%G1").is_err());
        assert!(Query::from_url("order=asc").is_err());
    }
}
