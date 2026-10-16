//! What `rule:routing/matched-once-before-the-handler`'s match costs, and how
//! that cost grows with the table.
//!
//! [`nvs_runtime::routes::Routes::match_request`] descends a trie over the
//! rows' fixed prefixes and compares the request only against the rows sharing
//! its own, so the question the arms below answer is **whether the table's
//! length shows in a match at all**. The module doc of `nvs_runtime::routes`
//! owns the trie and why precedence survives it.
//!
//! * `match_request/hit/8`, `/64`, `/512` — the same request against tables
//!   that differ only in how many rows they hold. The **slope** between them
//!   is what one more row costs a request, and is close to nothing; the
//!   intercept is one `Vec<&str>` split of the path, the trie descent and the
//!   capture the answer carries.
//! * `match_request/miss/512` — a match that answers `None`, which is the
//!   `404`'s first half: the descent stops at the first piece no row's prefix
//!   names.
//!
//! The table is `shared/routes.rs`', and that module's doc owns why its rows
//! share prefixes rather than being cheap to tell apart.

// `criterion_group!` expands to an undocumented public function.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

// The same table the guard in `tests/perf_guards.rs` walks, so the bench and
// the guard cannot drift apart; that file's own doc owns why it lives outside
// `src/`.
#[path = "../shared/routes.rs"]
mod routes;

fn route_table_walk(c: &mut Criterion) {
    let mut group = c.benchmark_group("routing");
    let (method, path) = routes::REQUEST;

    for rows in [8usize, 64, 512] {
        let table = routes::table(rows);
        group.bench_with_input(
            BenchmarkId::new("match_request/hit", rows),
            &rows,
            |b, _| {
                b.iter(|| black_box(table.match_request(black_box(method), black_box(path))));
            },
        );
    }

    let table = routes::table(512);
    group.bench_function("match_request/miss/512", |b| {
        b.iter(|| black_box(table.match_request(black_box(method), black_box("/health/live"))));
    });

    group.finish();
}

criterion_group!(benches, route_table_walk);
criterion_main!(benches);
