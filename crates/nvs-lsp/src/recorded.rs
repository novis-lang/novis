//! What the newest recorded request ran at each `Core\Db` call of a document
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
//! **The figures are three sums and one count per site.** SQLite counts the newest request's
//! statements at the site, totals and takes the slowest of their times, and counts the most
//! frequent statement text, so a loop of many iterations is one row of numbers here. The newest
//! request is the one with the highest `requests.id`, and its figures count every `query` event of
//! its trace at the site, so two recorded requests that share one trace id are counted together.
//!
//! **The site's line is where its call expression starts.** A call written as `$db` on one line
//! and `->query(...)` on the next has the receiver's line, and its lens sits there.
//!
//! **A copy is counted by line, from the `copies` table.** A recorded request writes one row per
//! copy site it copied at, with the site's file, line and hash. [`copies_in`] reads every row of
//! the document's hash, and for each line keeps the rows of the newest request that copied there,
//! so two sites on one line are one lens that sums them and shows the help of the larger one.
//!
//! # What it costs
//!
//! In the editor process, one connection and a few numbers per query site for each lens request,
//! freed with the answer. One lookup per site on the index's `events_site` index, then one
//! aggregate over that request's rows at the site. One lookup per document on the `copies_site`
//! index, and one small row per line that copied.

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

/// The viewer's page for the request with trace id `trace` and span id `span`, served at
/// `listen`, over TLS where `tls` is set.
#[must_use]
pub fn viewer_url(listen: &str, tls: bool, trace: &str, span: Option<&str>) -> String {
    let scheme = if tls { "https" } else { "http" };
    match span {
        Some(span) => format!("{scheme}://{listen}/requests/{trace}/{span}"),
        None => format!("{scheme}://{listen}/requests/{trace}"),
    }
}
