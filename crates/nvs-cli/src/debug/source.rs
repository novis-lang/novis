//! `GET /api/coverage`, the lines one request ran, and `GET /api/source`, the text of one file it
//! ran, which the viewer draws its coverage heatmap over
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! **A coverage row is passed through.** A development server whose `[debug] mode` names
//! `coverage` writes a spool `coverage` line per request: its span id and, per file, the count of
//! each line that ran. The ingester stores the files as the JSON text the line had, and
//! `GET /api/coverage?id=` sends it as it is, with the request's start.
//!
//! **A source is read only where a record and the project root both allow it.**
//! `GET /api/source?coverage=<id>&file=<path>` reads a file only when the coverage row names it,
//! so the page never names a path of its own choosing, and only when the file, with its links
//! resolved, is under the project root: the working directory `nvs debug` started in, which the
//! service a development server starts inherits from it. A file outside it is a `403`. `changed`
//! says the file was modified after the request started, because a development server compiles an
//! edited file again before its next request, so the counts may no longer match the lines.
//!
//! **What it spends:** one coverage row's text, or one source file of at most [`SOURCE_MAX`]
//! bytes, while it is written out. The project root is resolved once, when the service starts.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use hyper::StatusCode;
use rusqlite::{OptionalExtension as _, params};
use serde::Serialize;
use serde_json::value::RawValue;

use super::api::{self, Api};
use super::search;

/// The largest source file `GET /api/source` sends.
const SOURCE_MAX: u64 = 4 << 20;

/// `GET /api/coverage`: the lines one request ran.
#[derive(Serialize)]
struct Coverage {
    id: i64,
    trace: String,
    span: Option<String>,
    /// When the request started, or when the line was written if the request is not in the index.
    start_us: i64,
    /// File path to line to count. A line that did not run is absent.
    files: Box<RawValue>,
    /// Whether the spool line was too long, so `files` is empty.
    elided: bool,
}

/// `GET /api/source`: one file's text, and whether it changed after the request started.
#[derive(Serialize)]
struct Source {
    file: String,
    text: String,
    changed: bool,
}

/// The project root [`source`] reads under: the working directory, with its links resolved, or
/// `None` when it cannot be read.
pub(crate) fn root() -> Option<PathBuf> {
    std::env::current_dir().and_then(std::fs::canonicalize).ok()
}

/// `GET /api/coverage` with `query`: the status and the JSON body.
pub(crate) fn coverage(api: &Api, query: Option<&str>) -> (StatusCode, Vec<u8>) {
    let id = match query.unwrap_or_default().split_once('=') {
        Some(("id", value)) if !value.contains('&') => api::number("id", value),
        _ => Err("`id` is the one key, and it is required.".to_owned()),
    };
    let id = match id {
        Ok(id) => id,
        Err(message) => return api::error(StatusCode::BAD_REQUEST, &message),
    };
    match api.read(|db| read(db, id)) {
        Ok(Some(coverage)) => api::answer(Ok(coverage)),
        Ok(None) => api::error(
            StatusCode::NOT_FOUND,
            "No coverage in the index has this id.",
        ),
        Err(err) => api::answer::<()>(Err(err)),
    }
}

/// `GET /api/source` with `query`: the status and the JSON body.
pub(crate) fn source(api: &Api, query: Option<&str>) -> (StatusCode, Vec<u8>) {
    let (id, file) = match parse(query.unwrap_or_default()) {
        Ok(asked) => asked,
        Err(message) => return api::error(StatusCode::BAD_REQUEST, &message),
    };
    let coverage = match api.read(|db| read(db, id)) {
        Ok(Some(coverage)) => coverage,
        Ok(None) => {
            return api::error(
                StatusCode::NOT_FOUND,
                "No coverage in the index has this id.",
            );
        }
        Err(err) => return api::answer::<()>(Err(err)),
    };
    let named =
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(coverage.files.get())
            .is_ok_and(|files| files.contains_key(&file));
    if !named {
        return api::error(
            StatusCode::NOT_FOUND,
            "This request's coverage does not name this file.",
        );
    }
    let Some(root) = api.root() else {
        return api::error(
            StatusCode::FORBIDDEN,
            "`nvs debug` cannot read the folder it runs in, so it reads no source file.",
        );
    };
    match read_under(root, &file, coverage.start_us) {
        Ok((text, changed)) => api::answer(Ok(Source {
            file,
            text,
            changed,
        })),
        Err(failed) => failed,
    }
}

/// Reads `file`, which must be under `root`, and says whether it was modified after `start_us`.
fn read_under(
    root: &Path,
    file: &str,
    start_us: i64,
) -> Result<(String, bool), (StatusCode, Vec<u8>)> {
    let unreadable = |err: std::io::Error| {
        api::error(
            StatusCode::NOT_FOUND,
            &format!("The file `{file}` cannot be read: {err}"),
        )
    };
    let path = std::fs::canonicalize(root.join(file)).map_err(unreadable)?;
    if !path.starts_with(root) {
        return Err(api::error(
            StatusCode::FORBIDDEN,
            &format!(
                "The file `{file}` is outside the folder `nvs debug` runs in. \
                 `nvs debug` reads only files in that folder."
            ),
        ));
    }
    let meta = std::fs::metadata(&path).map_err(unreadable)?;
    if meta.len() > SOURCE_MAX {
        return Err(api::error(
            StatusCode::PAYLOAD_TOO_LARGE,
            &format!("The file `{file}` is larger than 4 MiB. Open it in your editor."),
        ));
    }
    let changed = meta
        .modified()
        .ok()
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .and_then(|at| i64::try_from(at.as_micros()).ok())
        .is_some_and(|at| at > start_us);
    let bytes = std::fs::read(&path).map_err(unreadable)?;
    Ok((String::from_utf8_lossy(&bytes).into_owned(), changed))
}

/// Reads `coverage` and `file`, which are both required. Any other key is an error.
fn parse(query: &str) -> Result<(i64, String), String> {
    let (mut id, mut file) = (None, None);
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "coverage" => id = Some(api::number("coverage", value)?),
            "file" => file = Some(search::decode("file", value)?),
            _ => return Err(format!("`{key}` is not a key of this path.")),
        }
    }
    match (id, file) {
        (Some(id), Some(file)) => Ok((id, file)),
        _ => Err("`coverage` and `file` are both required.".to_owned()),
    }
}

/// The coverage row `id`, with its request's start.
fn read(db: &mut rusqlite::Connection, id: i64) -> rusqlite::Result<Option<Coverage>> {
    db.query_row(
        "SELECT c.id, c.trace, c.span, \
         coalesce((SELECT r.start_us FROM requests r WHERE r.trace = c.trace \
         AND (c.span IS NULL OR r.span = c.span) ORDER BY r.id LIMIT 1), c.at_us), \
         c.files, c.elided FROM coverage c WHERE c.id = ?1",
        params![id],
        |row| {
            Ok(Coverage {
                id: row.get(0)?,
                trace: row.get(1)?,
                span: row.get(2)?,
                start_us: row.get(3)?,
                files: api::tree(&row.get::<_, String>(4)?),
                elided: row.get(5)?,
            })
        },
    )
    .optional()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_query_needs_its_coverage_and_its_file() {
        assert_eq!(
            parse("coverage=3&file=%2Fsrv%2Fshop%2Fitems.nvs"),
            Ok((3, "/srv/shop/items.nvs".to_owned()))
        );
        assert!(parse("coverage=3").is_err());
        assert!(parse("file=a.nvs").is_err());
        assert!(parse("coverage=3&file=a.nvs&path=b.nvs").is_err());
    }
}
