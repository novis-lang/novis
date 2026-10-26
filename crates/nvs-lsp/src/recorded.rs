//! What the newest recorded request ran at each `Core\Db` call of a document
//! (`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`).
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
//! **The figures are three sums and one count per site.** SQLite counts the newest request's
//! statements at the site, totals and takes the slowest of their times, and counts the most
//! frequent statement text, so a loop of many iterations is one row of numbers here.
//!
//! # What it costs
//!
//! In the editor process, one connection and a few numbers per query site for each lens request,
//! freed with the answer. One lookup per site on the index's `events_site` index, then one
//! aggregate over that request's rows at the site.

use std::path::{Path, PathBuf};
use std::time::Duration;

use nvs_diagnostics::canonical_key;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

/// What the newest recorded request that reached one site ran there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    /// The request's trace id, which names it in the viewer.
    pub trace: String,
    /// The request's span id, where the record has one.
    pub span: Option<String>,
    /// How many statements ran at the site.
    pub count: u64,
    /// Their time together, in microseconds.
    pub total_us: u64,
    /// The longest one, in microseconds.
    pub slowest_us: u64,
    /// How many times the most frequent statement text ran there.
    pub repeated: u64,
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
            "SELECT e.site_file, r.trace, r.span FROM events e JOIN requests r ON r.trace = e.trace \
             WHERE e.kind = 'query' AND e.site_hash = ?1 AND e.site_line = ?2 \
             GROUP BY e.site_file, r.id ORDER BY r.id DESC",
        )
        .ok()?;
    let mut rows = newest.query(params![hash, line]).ok()?;
    let (written, trace, span) = loop {
        let row = rows.next().ok()??;
        let written: String = row.get(0).ok()?;
        if name_key(root, &written) == key {
            break (written, row.get::<_, String>(1).ok()?, row.get(2).ok()?);
        }
    };
    let mut figures = db
        .prepare_cached(
            "SELECT COUNT(*), COALESCE(SUM(end_us - start_us), 0), COALESCE(MAX(end_us - start_us), 0), \
             (SELECT COALESCE(MAX(n), 0) FROM (SELECT COUNT(*) AS n FROM events \
              WHERE trace = ?1 AND kind = 'query' AND site_hash = ?2 AND site_line = ?3 \
              AND site_file = ?4 GROUP BY sql)) \
             FROM events WHERE trace = ?1 AND kind = 'query' AND site_hash = ?2 AND site_line = ?3 \
             AND site_file = ?4",
        )
        .ok()?;
    let (count, total_us, slowest_us, repeated) = figures
        .query_row(params![trace, hash, line, written], |row| {
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
        trace,
        span,
        count: whole(count),
        total_us: whole(total_us),
        slowest_us: whole(slowest_us),
        repeated: whole(repeated),
    })
}

/// A file name the index holds, as [`canonical_key`] keys the document.
fn name_key(root: Option<&Path>, written: &str) -> PathBuf {
    let path = Path::new(written);
    match root {
        Some(root) if path.is_relative() => canonical_key(&root.join(path)),
        _ => canonical_key(path),
    }
}

/// The lens's title: the count, the total and the slowest time, and how often one statement
/// repeated when it ran more than once.
#[must_use]
pub fn title(ran: &Ran) -> String {
    let mut title = if ran.count == 1 {
        format!("1 query, {}", duration(ran.total_us))
    } else {
        format!(
            "{} queries, {} in total, slowest {}",
            ran.count,
            duration(ran.total_us),
            duration(ran.slowest_us)
        )
    };
    if ran.repeated > 1 {
        title.push_str(&format!(", one statement repeated {} times", ran.repeated));
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

/// The viewer's page for `ran`'s request, served at `listen`, over TLS where `tls` is set.
#[must_use]
pub fn viewer_url(listen: &str, tls: bool, ran: &Ran) -> String {
    let scheme = if tls { "https" } else { "http" };
    match &ran.span {
        Some(span) => format!("{scheme}://{listen}/requests/{}/{span}", ran.trace),
        None => format!("{scheme}://{listen}/requests/{}", ran.trace),
    }
}
