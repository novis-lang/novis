//! The route table a request is matched against, as one table both
//! measurements build.
//!
//! `benches/routing.rs` tracks the walk's cost precisely and the guard in
//! `tests/perf_guards.rs` fails a build when it leaves its cost class. They
//! read the table from here for the reason `isolate.rs` gives beside it: two
//! spellings of "the table an application declares" would
//! make the bench and the guard answer different questions. What the figure
//! means, and what it does and does not include, is the bench's own module doc.
//!
//! It sits here rather than in `src/` and is reached by `#[path]` from both,
//! because it needs `nvs-runtime` and this package keeps that crate in
//! `[dev-dependencies]` (the manifest says why). `shared/` is a directory cargo
//! does not scan for targets.
//!
//! The rows are shaped like an application's rather than minimally: three
//! shapes in rotation — a literal-only admin path, an `{id}` API path and a
//! `{slug}` edit path — under the four verbs a table is declared with, and
//! every path sharing a prefix with its neighbours. That is what decides where
//! [`nvs_runtime::routes::Routes::match_request`]'s per-row work stops, so a
//! table of distinct first segments would price the walk lower than any real
//! one.

use nvs_runtime::routes::{Capture, CaptureConv, Route, Routes};

/// The request every measurement matches, as the door hands it over: the
/// `{id}` row of `resource4`, which [`table`] declares `Get` at any size it is
/// asked for.
///
/// One row matches it and the walk visits all of them regardless — the match
/// is the smallest rank in the whole table, not the first fill — so where in
/// load order the answer sits changes nothing, and the same pair against two
/// table sizes is what makes the slope between them a per-row figure.
pub(crate) const REQUEST: (&str, &str) = ("GET", "/api/resource4/42");

/// A table of `rows` rows, in the load order a compiler emits.
pub(crate) fn table(rows: usize) -> Routes {
    const VERBS: [&str; 4] = ["Get", "Post", "Put", "Delete"];

    Routes::new(
        (0..rows)
            .map(|index| {
                let (path, captures) = match index % 3 {
                    0 => (format!("/admin/resource{index}/index"), Vec::new()),
                    1 => (
                        format!("/api/resource{index}/{{id}}"),
                        vec![Capture {
                            name: "id".to_owned(),
                            conv: CaptureConv::Uint,
                        }],
                    ),
                    _ => (
                        format!("/resource{index}/{{slug}}/edit"),
                        vec![Capture {
                            name: "slug".to_owned(),
                            conv: CaptureConv::Text,
                        }],
                    ),
                };
                Route::new(
                    VERBS[index % VERBS.len()],
                    path,
                    Some(format!("resource{index}")),
                    format!("Resource{index}Controller::run"),
                    Some("Public".to_owned()),
                    captures,
                )
            })
            .collect(),
    )
}
