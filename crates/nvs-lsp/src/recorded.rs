//! What the newest recorded request ran at each call of a document that a recorded request names
//! by line: a `Core\Db` statement, an HTTP call, a cache or session read or write, or a file open
//! (`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`), and what it copied on
//! each line where the compiler kept a copy (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! **The debug index is read, never written.** It is `nvs debug`'s SQLite file in the data folder
//! (`nvs_config::data::Folder::debug_index`), opened read-only with no busy wait, so an index that
//! is absent, that the service holds locked, or whose schema has no site columns gives no lens and
//! no error. Nothing here makes a network request.
//!
//! **A site is the file, the line and the file's hash.** A traced query records the row of its
//! call's site (`rule:observability/a-query-span-names-its-call-site`), and the lens asks for the
//! line its call starts on and the BLAKE3 of the text the editor holds. A record made from any
//! other text of the file has another hash and is not shown, so a lens never sits on a line that
//! ran something else. The file is the name the program gave it, so it is compared under
//! [`canonical_key`] after a relative name is joined to the workspace root, the way a coverage
//! report's file is ([`crate::reports`]).
//!
//! **The figures are three sums and one count per site, of one kind.** The site's kind is the
//! kind of the newest request's events there: `query`, `http`, `cache`, `session` or `file`, and
//! the lens names it. SQLite counts that request's events of the kind at the site, totals and
//! takes the slowest of their times, and counts the most frequent statement text, or for any
//! other kind the most frequent `target` (the host and path, the key or the file path), so a loop
//! of many iterations is one row of numbers here. The newest request is the one with the highest
//! `requests.id`, and its figures count every event of the kind in its trace at the site, so two
//! recorded requests that share one trace id are counted together.
//!
//! **The site's line is where its call expression starts.** A call written as `$db` on one line
//! and `->query(...)` on the next has the receiver's line, and its lens sits there.
//!
//! **A copy is counted by line, from the `copies` table.** A recorded request writes one row per
//! copy site it copied at, with the site's file, line and hash. [`copies_in`] reads every row of
//! the document's hash, and for each line keeps the rows of the newest request that copied there,
//! so two sites on one line are one lens that sums them and shows the help of the larger one.
//!
//! **A route, a dump and a throw site have a lens of their own**
//! (`rule:ide/a-lens-shows-what-a-route-a-dump-and-a-throw-recorded`). A route is keyed on its
//! method and its pattern ([`route`]), so an edit to the file does not move it. A dump and an
//! exception group are keyed on their file and line ([`dumps_in`], [`throws_in`]), and the caller
//! passes when the file was last written, so a record older than the file's text gives no lens.
//! No line is remapped through an edit.
//!
//! # What it costs
//!
//! In the editor process, one connection and a few numbers per query site for each lens request,
//! freed with the answer. One lookup per site on the index's `events_site` index, then one
//! aggregate over that request's rows at the site. One lookup per document on the `copies_site`
//! index, and one small row per line that copied. Per route handler, three reads on the
//! `requests_route` index bounded by [`ROUTE_ROWS`], and [`ROUTE_DURATIONS`] integers. Per
//! document, the newest [`SITE_ROWS`] dump and exception group rows without their trees, then
//! one tree per line kept, and one line of at most [`DUMP_WIDTH`] characters per dump lens.

use std::path::{Path, PathBuf};
use std::time::Duration;

use nvs_diagnostics::canonical_key;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

/// What the newest recorded request that reached one site ran there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    /// The kind of the events at the site, as the index spells it: `query`, `http`, `cache`,
    /// `session` or `file`.
    pub kind: String,
    /// The request's trace id, which names it in the viewer.
    pub trace: String,
    /// The request's span id, where the record has one.
    pub span: Option<String>,
    /// How many events of the kind ran at the site.
    pub count: u64,
    /// Their time together, in microseconds.
    pub total_us: u64,
    /// The longest one, in microseconds.
    pub slowest_us: u64,
    /// How many times the most frequent statement text, host and path, key or file path ran
    /// there.
    pub repeated: u64,
}

/// What the newest recorded request that copied on one line of a file copied there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copied {
    /// The 1-based line.
    pub line: usize,
    /// The request's trace id, which names it in the viewer.
    pub trace: String,
    /// The request's span id, where the record has one.
    pub span: Option<String>,
    /// How many copies it made on the line.
    pub copies: u64,
    /// The entries or bytes those copies copied.
    pub units: u64,
    /// The help of the reason of the site that copied the most on the line, or `None` for a
    /// reason this build does not know.
    pub help: Option<&'static str>,
}

/// The debug index at `path`, opened for reading, or `None` where it is absent or cannot be
/// opened.
#[must_use]
pub fn open(path: &Path) -> Option<Connection> {
    if !path.is_file() {
        return None;
    }
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    db.busy_timeout(Duration::ZERO).ok()?;
    Some(db)
}

/// What the newest recorded request ran at line `line` of the file `file`, whose text hashes to
/// `hash`. A relative file name in the index is joined to `root`. `None` when no recorded request
/// reached the site, and when the index cannot answer.
#[must_use]
pub fn at_site(
    db: &Connection,
    root: Option<&Path>,
    file: &Path,
    line: usize,
    hash: &str,
) -> Option<Ran> {
    let key = canonical_key(file);
    let line = i64::try_from(line).ok()?;
    let mut newest = db
        .prepare_cached(
            "SELECT e.site_file, r.trace, r.span, e.kind FROM events e \
             JOIN requests r ON r.trace = e.trace \
             WHERE e.site_hash = ?1 AND e.site_line = ?2 AND e.kind IN (?3, ?4, ?5, ?6, ?7) \
             GROUP BY e.site_file, e.kind, r.id ORDER BY r.id DESC",
        )
        .ok()?;
    let [query, http, cache, session, file_kind] = KINDS;
    let mut rows = newest
        .query(params![hash, line, query, http, cache, session, file_kind])
        .ok()?;
    let (written, trace, span, kind) = loop {
        let row = rows.next().ok()??;
        let written: String = row.get(0).ok()?;
        if name_key(root, &written) == key {
            break (
                written,
                row.get::<_, String>(1).ok()?,
                row.get(2).ok()?,
                row.get::<_, String>(3).ok()?,
            );
        }
    };
    // A query repeats its statement text, and every other kind its `target`: the host and
    // path, the key or the file path.
    let mut figures = db
        .prepare_cached(
            "SELECT COUNT(*), COALESCE(SUM(end_us - start_us), 0), COALESCE(MAX(end_us - start_us), 0), \
             (SELECT COALESCE(MAX(n), 0) FROM (SELECT COUNT(*) AS n FROM events \
              WHERE trace = ?1 AND kind = ?5 AND site_hash = ?2 AND site_line = ?3 \
              AND site_file = ?4 GROUP BY COALESCE(sql, target))) \
             FROM events WHERE trace = ?1 AND kind = ?5 AND site_hash = ?2 AND site_line = ?3 \
             AND site_file = ?4",
        )
        .ok()?;
    let (count, total_us, slowest_us, repeated) = figures
        .query_row(params![trace, hash, line, written, kind], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .optional()
        .ok()??;
    let whole = |value: i64| u64::try_from(value).unwrap_or(0);
    (count > 0).then(|| Ran {
        kind,
        trace,
        span,
        count: whole(count),
        total_us: whole(total_us),
        slowest_us: whole(slowest_us),
        repeated: whole(repeated),
    })
}

/// Each line of the file `file`, whose text hashes to `hash`, at which a recorded request copied
/// a value, with what the newest such request copied there, in line order. A relative file name
/// in the index is joined to `root`. Empty when no recorded request copied in this text of the
/// file, and when the index cannot answer, such as one whose schema has no `copies` table.
///
/// One lookup on the `copies_site` index by the hash, which every site of the file shares, so a
/// document costs one query whatever its number of sites.
#[must_use]
pub fn copies_in(db: &Connection, root: Option<&Path>, file: &Path, hash: &str) -> Vec<Copied> {
    let key = canonical_key(file);
    let Ok(mut statement) = db.prepare_cached(
        "SELECT c.file, c.line, c.reason, c.copies, c.units, r.id, r.trace, r.span \
         FROM copies c JOIN requests r ON r.trace = c.trace AND (c.span IS NULL OR r.span = c.span) \
         WHERE c.hash = ?1 ORDER BY c.line, r.id DESC",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map(params![hash], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
        ))
    }) else {
        return Vec::new();
    };
    let whole = |value: i64| u64::try_from(value).unwrap_or(0);
    let mut found: Vec<Copied> = Vec::new();
    // The request each line's figures are from, and the units of the row whose help it shows.
    let mut newest = (0_i64, 0_u64);
    for row in rows {
        let Ok((written, line, reason, copies, units, request, trace, span)) = row else {
            return Vec::new();
        };
        let Ok(line) = usize::try_from(line) else {
            continue;
        };
        if name_key(root, &written) != key {
            continue;
        }
        let (copies, units) = (whole(copies), whole(units));
        let help = nvs_ir::copies::Reason::from_name(&reason).map(nvs_ir::copies::Reason::help);
        match found.last_mut() {
            // Rows of an older request on a line already found are not its figures.
            Some(last) if last.line == line && newest.0 != request => {}
            Some(last) if last.line == line => {
                last.copies += copies;
                last.units += units;
                if units > newest.1 {
                    last.help = help;
                    newest.1 = units;
                }
            }
            _ => {
                newest = (request, units);
                found.push(Copied {
                    line,
                    trace,
                    span,
                    copies,
                    units,
                    help,
                });
            }
        }
    }
    found
}

/// A copies lens's title: how many copies the line made and how much they copied, then the help.
#[must_use]
pub fn copies_title(copied: &Copied) -> String {
    let times = if copied.copies == 1 {
        "Copied once".to_owned()
    } else {
        format!("Copied {} times", copied.copies)
    };
    let mut title = format!("{times} here, {} entries or bytes.", copied.units);
    if let Some(help) = copied.help {
        title.push(' ');
        title.push_str(help);
    }
    title
}

/// A file name the index holds, as [`canonical_key`] keys the document.
fn name_key(root: Option<&Path>, written: &str) -> PathBuf {
    let path = Path::new(written);
    match root {
        Some(root) if path.is_relative() => canonical_key(&root.join(path)),
        _ => canonical_key(path),
    }
}

/// The kinds of event a site records, as the index spells them: a statement, an outbound HTTP
/// call, a cache read or write, a session read or write, and a file open.
const KINDS: [&str; 5] = ["query", "http", "cache", "session", "file"];

/// The words a lens uses for `kind`: one event, more than one, and what repeats.
fn words(kind: &str) -> (&'static str, &'static str, &'static str) {
    match kind {
        "http" => ("HTTP call", "HTTP calls", "one host and path"),
        "cache" => ("cache call", "cache calls", "one key"),
        "session" => ("session call", "session calls", "one key"),
        "file" => ("file open", "file opens", "one file"),
        _ => ("query", "queries", "one statement"),
    }
}

/// The lens's title: the count of the kind, the total and the slowest time, and how often one
/// statement, host and path, key or file repeated when it ran more than once.
#[must_use]
pub fn title(ran: &Ran) -> String {
    let (one, many, repeat) = words(&ran.kind);
    let mut title = if ran.count == 1 {
        format!("1 {one}, {}", duration(ran.total_us))
    } else {
        format!(
            "{} {many}, {} in total, slowest {}",
            ran.count,
            duration(ran.total_us),
            duration(ran.slowest_us)
        )
    };
    if ran.repeated > 1 {
        title.push_str(&format!(", {repeat} repeated {} times", ran.repeated));
    }
    title
}

/// `us` microseconds, in the unit that keeps the number short.
fn duration(us: u64) -> String {
    if us < 1_000 {
        format!("{us} µs")
    } else if us < 1_000_000 {
        format!("{:.1} ms", us as f64 / 1_000.0)
    } else {
        format!("{:.2} s", us as f64 / 1_000_000.0)
    }
}

/// The viewer's page at `page`, a path such as `/requests/<trace>/<span>`, served at `listen`,
/// over TLS where `tls` is set.
#[must_use]
pub fn page_url(listen: &str, tls: bool, page: &str) -> String {
    let scheme = if tls { "https" } else { "http" };
    format!("{scheme}://{listen}{page}")
}

/// How many of a route's newest requests [`route`] counts and searches for a failure.
pub const ROUTE_ROWS: i64 = 10_000;

/// How many of a route's newest durations its median is taken over.
pub const ROUTE_DURATIONS: i64 = 500;

/// How many of the newest dumps and exception groups [`dumps_in`] and [`throws_in`] read.
pub const SITE_ROWS: i64 = 10_000;

/// What the recorded requests of one route did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteRan {
    /// How many requests the index holds for the route, at most [`ROUTE_ROWS`].
    pub count: u64,
    /// The median duration of the newest [`ROUTE_DURATIONS`] of them, in microseconds.
    pub p50_us: u64,
    /// The status and the start, in microseconds since the epoch, of the newest counted request
    /// that ended with a status of 500 or more.
    pub failed: Option<(i64, i64)>,
}

/// What the recorded requests of the route `method` `route` did, the route as its pattern is
/// written (`/items/{id}`). `None` when the index holds no request for it, and when it cannot
/// answer.
///
/// Three reads on the `requests_route` index, each bounded by rows: the count of at most
/// [`ROUTE_ROWS`], the newest [`ROUTE_DURATIONS`] durations, and the newest failure among the
/// newest [`ROUTE_ROWS`]. The median is the nearest rank, as the viewer's Routes view takes it.
#[must_use]
pub fn route(db: &Connection, method: &str, route: &str) -> Option<RouteRan> {
    let count: i64 = db
        .prepare_cached(
            "SELECT COUNT(*) FROM (SELECT 1 FROM requests WHERE method = ?1 AND route = ?2 LIMIT ?3)",
        )
        .ok()?
        .query_row(params![method, route, ROUTE_ROWS], |row| row.get(0))
        .ok()?;
    if count == 0 {
        return None;
    }
    let mut durations: Vec<i64> = db
        .prepare_cached(
            "SELECT duration_us FROM requests WHERE method = ?1 AND route = ?2 \
             ORDER BY start_us DESC LIMIT ?3",
        )
        .ok()?
        .query_map(params![method, route, ROUTE_DURATIONS], |row| row.get(0))
        .ok()?
        .collect::<Result<_, _>>()
        .ok()?;
    durations.sort_unstable();
    let rank = durations.len().div_ceil(2).max(1);
    let p50 = durations.get(rank - 1).copied().unwrap_or(0);
    let failed = db
        .prepare_cached(
            "SELECT status, start_us FROM (SELECT status, start_us FROM requests \
             WHERE method = ?1 AND route = ?2 ORDER BY start_us DESC LIMIT ?3) \
             WHERE status >= 500 ORDER BY start_us DESC LIMIT 1",
        )
        .ok()?
        .query_row(params![method, route, ROUTE_ROWS], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .optional()
        .ok()?;
    let whole = |value: i64| u64::try_from(value).unwrap_or(0);
    Some(RouteRan {
        count: whole(count),
        p50_us: whole(p50),
        failed,
    })
}

/// A route lens's title: the count of requests, their median duration, and the newest failure
/// with its age at `now_us`.
#[must_use]
pub fn route_title(ran: &RouteRan, now_us: i64) -> String {
    let count = u64::try_from(ROUTE_ROWS).unwrap_or(u64::MAX);
    let mut title = match ran.count {
        1 => "1 request".to_owned(),
        n if n >= count => format!("{n}+ requests"),
        n => format!("{n} requests"),
    };
    title.push_str(&format!(", p50 {}", duration(ran.p50_us)));
    if let Some((status, at)) = ran.failed {
        title.push_str(&format!(", last {status} {}", age(now_us, at)));
    }
    title
}

/// The newest value a recorded request dumped on one line of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dumped {
    /// The 1-based line.
    pub line: usize,
    /// The dump's id, which names it on the request's Dumps tab.
    pub id: i64,
    /// The request's trace id.
    pub trace: String,
    /// The request's span id, where the record has one.
    pub span: Option<String>,
    /// The value as one line of at most [`DUMP_WIDTH`] characters.
    pub value: String,
}

/// The most characters a dump lens shows of the value.
pub const DUMP_WIDTH: usize = 80;

/// Each line of the file `file` with the newest value a recorded request dumped there, in line
/// order, when that dump was made at `since_us` or later. A relative file name in the index is
/// joined to `root`. Empty when the index cannot answer.
///
/// `since_us` is when the file was last written, because a line number of an older record may
/// name another line of the file now. The read is the newest [`SITE_ROWS`] dumps, then one row
/// for each line kept. The tree is the record's, which redaction already reached.
#[must_use]
pub fn dumps_in(db: &Connection, root: Option<&Path>, file: &Path, since_us: i64) -> Vec<Dumped> {
    let key = canonical_key(file);
    let Ok(mut statement) = db
        .prepare_cached("SELECT id, file, line, at_us, trace FROM dumps ORDER BY id DESC LIMIT ?1")
    else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map(params![SITE_ROWS], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
        ))
    }) else {
        return Vec::new();
    };
    let mut names = Names::default();
    let mut newest: Vec<(usize, i64, i64, String)> = Vec::new();
    for row in rows {
        let Ok((id, written, line, at_us, trace)) = row else {
            return Vec::new();
        };
        let Ok(line) = usize::try_from(line) else {
            continue;
        };
        if newest.iter().any(|kept| kept.0 == line) || !names.is(root, &written, &key) {
            continue;
        }
        newest.push((line, id, at_us, trace));
    }
    let mut found: Vec<Dumped> = newest
        .into_iter()
        .filter(|&(_, _, at_us, _)| at_us >= since_us)
        .filter_map(|(line, id, _, trace)| {
            let tree: String = db
                .prepare_cached("SELECT tree FROM dumps WHERE id = ?1")
                .ok()?
                .query_row(params![id], |row| row.get(0))
                .ok()?;
            let tree: serde_json::Value = serde_json::from_str(&tree).ok()?;
            Some(Dumped {
                line,
                id,
                span: span_of(db, &trace),
                trace,
                value: one_line(&tree, DUMP_WIDTH),
            })
        })
        .collect();
    found.sort_by_key(|dumped| dumped.line);
    found
}

/// A dump lens's title: the value dumped.
#[must_use]
pub fn dump_title(dumped: &Dumped) -> String {
    format!("Dumped: {}", dumped.value)
}

/// The viewer's path of a dump: its request's Dumps tab, at the dump.
#[must_use]
pub fn dump_page(dumped: &Dumped) -> String {
    let request = match &dumped.span {
        Some(span) => format!("/requests/{}/{span}", dumped.trace),
        None => format!("/requests/{}", dumped.trace),
    };
    format!("{request}?tab=dumps#dump-{}", dumped.id)
}

/// One exception group whose site is a line of a file: one class thrown there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thrown {
    /// The 1-based line.
    pub line: usize,
    /// The class thrown.
    pub class: String,
    /// The file as the index has it, which names the group in the viewer.
    pub file: String,
    /// How often it was thrown there.
    pub count: u64,
    /// When last, in microseconds since the epoch.
    pub last_us: i64,
}

/// Each exception group whose site is a line of the file `file`, last thrown at `since_us` or
/// later, in line order and the newest first on a line. A relative file name in the index is
/// joined to `root`. Empty when the index cannot answer.
///
/// `since_us` is when the file was last written, as for [`dumps_in`]. The read is the
/// [`SITE_ROWS`] groups thrown most recently, on the `exception_groups_last` index.
#[must_use]
pub fn throws_in(db: &Connection, root: Option<&Path>, file: &Path, since_us: i64) -> Vec<Thrown> {
    let key = canonical_key(file);
    let Ok(mut statement) = db.prepare_cached(
        "SELECT class, file, line, count, last_us FROM exception_groups \
         ORDER BY last_us DESC LIMIT ?1",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map(params![SITE_ROWS], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
        ))
    }) else {
        return Vec::new();
    };
    let mut names = Names::default();
    let mut found = Vec::new();
    for row in rows {
        let Ok((class, written, line, count, last_us)) = row else {
            return Vec::new();
        };
        let Ok(line) = usize::try_from(line) else {
            continue;
        };
        if last_us < since_us || !names.is(root, &written, &key) {
            continue;
        }
        found.push(Thrown {
            line,
            class,
            file: written,
            count: u64::try_from(count).unwrap_or(0),
            last_us,
        });
    }
    // The rows are newest first, and a stable sort keeps that order on a line.
    found.sort_by_key(|thrown| thrown.line);
    found
}

/// A throw lens's title: the class, how often it was thrown, and when last, at `now_us`.
#[must_use]
pub fn throw_title(thrown: &Thrown, now_us: i64) -> String {
    let times = match thrown.count {
        1 => "once".to_owned(),
        n => format!("{n} times"),
    };
    format!(
        "{} thrown {times}, last {}",
        thrown.class,
        age(now_us, thrown.last_us)
    )
}

/// The viewer's path of an exception group: `/exceptions` with its class, file and line.
#[must_use]
pub fn throw_page(thrown: &Thrown) -> String {
    format!(
        "/exceptions?class={}&file={}&line={}",
        component(&thrown.class),
        component(&thrown.file),
        thrown.line
    )
}

/// The viewer's path of a route's request list, filtered to `method` and `route`.
#[must_use]
pub fn route_page(method: &str, route: &str) -> String {
    format!("/?method={}&route={}", component(method), component(route))
}

/// The span id of the newest request with trace id `trace`, where it has one.
fn span_of(db: &Connection, trace: &str) -> Option<String> {
    db.prepare_cached("SELECT span FROM requests WHERE trace = ?1 ORDER BY id DESC LIMIT 1")
        .ok()?
        .query_row(params![trace], |row| row.get::<_, Option<String>>(0))
        .optional()
        .ok()
        .flatten()
        .flatten()
}

/// The file names already compared with the document, so a name the index repeats is resolved
/// once.
#[derive(Default)]
struct Names(std::collections::HashMap<String, bool>);

impl Names {
    /// Whether `written`, joined to `root` when relative, is the file keyed `key`.
    fn is(&mut self, root: Option<&Path>, written: &str, key: &Path) -> bool {
        if let Some(&same) = self.0.get(written) {
            return same;
        }
        let same = name_key(root, written) == key;
        self.0.insert(written.to_owned(), same);
        same
    }
}

/// `text` as one component of a URL's query: every byte but a letter, a digit and `-._~` is
/// percent-encoded, as the viewer's own links are.
fn component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// How long ago `at_us` was at `now_us`, in the largest unit that is at least one. A time less
/// than five seconds ago, or after `now_us`, is "just now".
fn age(now_us: i64, at_us: i64) -> String {
    let seconds = now_us.saturating_sub(at_us) / 1_000_000;
    match seconds {
        ..5 => "just now".to_owned(),
        5..60 => format!("{seconds} s ago"),
        60..3_600 => format!("{} min ago", seconds / 60),
        3_600..86_400 => format!("{} h ago", seconds / 3_600),
        _ => format!("{} d ago", seconds / 86_400),
    }
}

/// A dumped value's tree (`nvs_render`'s JSON rendering) as one line of at most `width`
/// characters, cut with `…` where it is longer.
#[must_use]
pub fn one_line(tree: &serde_json::Value, width: usize) -> String {
    let mut out = String::new();
    write_node(tree, &mut out, width + 1);
    if out.chars().count() > width {
        let mut cut: String = out.chars().take(width.saturating_sub(1)).collect();
        cut.push('…');
        return cut;
    }
    out
}

/// Writes `node` into `out`, stopping once `out` has `budget` characters.
fn write_node(node: &serde_json::Value, out: &mut String, budget: usize) {
    use serde_json::Value;
    if out.chars().count() >= budget {
        return;
    }
    match node {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => out.push_str(&number.to_string()),
        Value::String(text) => out.push_str(&serde_json::to_string(text).unwrap_or_default()),
        Value::Array(items) => {
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push_str(", ");
                }
                write_node(item, out, budget);
                if out.chars().count() >= budget {
                    return;
                }
            }
            out.push(']');
        }
        Value::Object(map) => {
            if let Some(class) = map.get("$class").and_then(Value::as_str) {
                out.push_str(class);
                out.push(' ');
                match map.get("$properties") {
                    Some(properties) => write_node(properties, out, budget),
                    None => out.push_str("{}"),
                }
                return;
            }
            if let (1, Some((tag, value))) = (map.len(), map.iter().next()) {
                match tag.as_str() {
                    "$redacted" => return out.push_str("[redacted]"),
                    "$elided" => return out.push('…'),
                    "$cycle" => return out.push_str(&format!("#{value}")),
                    "$callable" => return out.push_str("callable"),
                    "$bytes" => {
                        let hex = value.as_str().unwrap_or_default();
                        return out.push_str(&format!("bytes({})", hex.len() / 2));
                    }
                    "$decimal" | "$float" | "$enum" | "$span" => {
                        return match value.as_str() {
                            Some(text) => out.push_str(text),
                            None => write_node(value, out, budget),
                        };
                    }
                    _ => {}
                }
            }
            out.push('{');
            for (at, (name, value)) in map.iter().enumerate() {
                if at > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
                out.push_str(": ");
                write_node(value, out, budget);
                if out.chars().count() >= budget {
                    return;
                }
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One second, in microseconds.
    const SECOND: i64 = 1_000_000;

    /// When the fixture's records were made.
    const AT: i64 = 1_700_000_000 * SECOND;

    /// An index with the columns the three lenses read: two routes, two dumps on one line and one
    /// on another, and two exception groups, one of them in another file.
    fn fixture() -> Connection {
        let db = Connection::open_in_memory().expect("an in-memory index");
        db.execute_batch(&format!(
            "CREATE TABLE requests (id INTEGER PRIMARY KEY, trace TEXT NOT NULL, span TEXT, \
               start_us INTEGER NOT NULL, duration_us INTEGER NOT NULL, method TEXT NOT NULL, \
               route TEXT, status INTEGER NOT NULL);
             CREATE TABLE dumps (id INTEGER PRIMARY KEY, trace TEXT NOT NULL, at_us INTEGER NOT NULL, \
               file TEXT NOT NULL, line INTEGER NOT NULL, tree TEXT NOT NULL);
             CREATE TABLE exception_groups (id INTEGER PRIMARY KEY, class TEXT NOT NULL, \
               file TEXT NOT NULL, line INTEGER NOT NULL, count INTEGER NOT NULL, \
               first_us INTEGER NOT NULL, last_us INTEGER NOT NULL);
             INSERT INTO requests VALUES (1, 'a1', 's1', {a}, 3000, 'GET', '/items/{{id}}', 200);
             INSERT INTO requests VALUES (2, 'a2', 's2', {b}, 1000, 'GET', '/items/{{id}}', 503);
             INSERT INTO requests VALUES (3, 'a3', 's3', {c}, 2000, 'GET', '/items/{{id}}', 200);
             INSERT INTO requests VALUES (4, 'a4', NULL, {c}, 9000, 'POST', '/items', 201);
             INSERT INTO dumps VALUES (1, 'a1', {a}, 'case.nvs', 4, '{{\"old\":1}}');
             INSERT INTO dumps VALUES (2, 'a3', {c}, 'case.nvs', 4, \
               '{{\"$class\":\"Shop\\\\Item\",\"$id\":1,\"$properties\":{{\"name\":\"Lamp\",\"card\":{{\"$redacted\":true}}}}}}');
             INSERT INTO dumps VALUES (3, 'a4', {a}, 'case.nvs', 2, '[1,2,3]');
             INSERT INTO dumps VALUES (4, 'a4', {c}, 'other.nvs', 4, '7');
             INSERT INTO exception_groups VALUES (1, 'Shop\\OutOfStock', 'case.nvs', 7, 3, {a}, {c});
             INSERT INTO exception_groups VALUES (2, 'Shop\\Gone', 'other.nvs', 7, 1, {a}, {c});",
            a = AT,
            b = AT + SECOND,
            c = AT + 2 * SECOND,
        ))
        .expect("the fixture");
        db
    }

    fn root() -> PathBuf {
        nvs_repo::path("crates/nvs-lsp")
    }

    #[test]
    fn a_route_lens_counts_its_requests_and_names_the_newest_failure() {
        let db = fixture();
        let ran = route(&db, "GET", "/items/{id}").expect("the route has requests");
        assert_eq!(
            ran,
            RouteRan {
                count: 3,
                p50_us: 2000,
                failed: Some((503, AT + SECOND)),
            }
        );
        assert_eq!(
            route_title(&ran, AT + 3 * 60 * SECOND),
            "3 requests, p50 2.0 ms, last 503 2 min ago"
        );
        assert_eq!(
            route_page("GET", "/items/{id}"),
            "/?method=GET&route=%2Fitems%2F%7Bid%7D"
        );
        let one = route(&db, "POST", "/items").expect("the route has a request");
        assert_eq!(route_title(&one, AT), "1 request, p50 9.0 ms");
        assert_eq!(route(&db, "DELETE", "/items/{id}"), None);
    }

    #[test]
    fn a_dump_lens_shows_the_newest_value_on_each_line_as_one_short_line() {
        let db = fixture();
        let root = root();
        let found = dumps_in(&db, Some(&root), &root.join("case.nvs"), AT);
        assert_eq!(
            found,
            vec![
                Dumped {
                    line: 2,
                    id: 3,
                    trace: "a4".to_owned(),
                    span: None,
                    value: "[1, 2, 3]".to_owned(),
                },
                Dumped {
                    line: 4,
                    id: 2,
                    trace: "a3".to_owned(),
                    span: Some("s3".to_owned()),
                    value: "Shop\\Item {card: [redacted], name: \"Lamp\"}".to_owned(),
                },
            ]
        );
        assert_eq!(dump_page(&found[1]), "/requests/a3/s3?tab=dumps#dump-2");
        assert_eq!(dump_page(&found[0]), "/requests/a4?tab=dumps#dump-3");
    }

    #[test]
    fn no_dump_or_throw_lens_for_a_file_newer_than_its_record() {
        let db = fixture();
        let root = root();
        let file = root.join("case.nvs");
        // Written after the dump on line 2 and before the one on line 4.
        let found = dumps_in(&db, Some(&root), &file, AT + SECOND);
        assert_eq!(found.iter().map(|d| d.line).collect::<Vec<_>>(), vec![4]);
        assert!(dumps_in(&db, Some(&root), &file, AT + 3 * SECOND).is_empty());
        assert!(throws_in(&db, Some(&root), &file, AT + 3 * SECOND).is_empty());
    }

    #[test]
    fn a_throw_lens_counts_its_group_and_opens_it() {
        let db = fixture();
        let root = root();
        let found = throws_in(&db, Some(&root), &root.join("case.nvs"), AT);
        assert_eq!(
            found,
            vec![Thrown {
                line: 7,
                class: "Shop\\OutOfStock".to_owned(),
                file: "case.nvs".to_owned(),
                count: 3,
                last_us: AT + 2 * SECOND,
            }]
        );
        assert_eq!(
            throw_title(&found[0], AT + 2 * SECOND + 3 * 3_600 * SECOND),
            "Shop\\OutOfStock thrown 3 times, last 3 h ago"
        );
        assert_eq!(
            throw_page(&found[0]),
            "/exceptions?class=Shop%5COutOfStock&file=case.nvs&line=7"
        );
    }

    #[test]
    fn no_lens_without_an_index() {
        let root = root();
        assert!(open(&root.join("no-such-index.sqlite")).is_none());
        // An index whose schema has none of the tables gives no lens and no error.
        let empty = Connection::open_in_memory().expect("an in-memory index");
        assert_eq!(route(&empty, "GET", "/"), None);
        assert!(dumps_in(&empty, Some(&root), &root.join("case.nvs"), 0).is_empty());
        assert!(throws_in(&empty, Some(&root), &root.join("case.nvs"), 0).is_empty());
    }

    #[test]
    fn a_long_value_is_cut_to_the_lens_width() {
        let tree = serde_json::json!({ "text": "x".repeat(200) });
        let line = one_line(&tree, DUMP_WIDTH);
        assert_eq!(line.chars().count(), DUMP_WIDTH);
        assert!(line.ends_with('…'));
        assert_eq!(
            one_line(&serde_json::json!({ "$elided": {"cut": "depth"} }), 80),
            "…"
        );
    }
}
