The viewer's API answers per route and per statement as well as per request, and every answer reads
a bounded number of rows of the index, so a question about the whole index costs the same with a
hundred requests in it as with a million.

**The index holds what the list draws, computed once at ingest.** The `requests` row carries
`db_us`, `http_us`, `cache_us`, `session_us`, `file_us`, `gc_us` and `spawn_us`, the summed durations
of the request's events by kind, and `repeats`, the largest number of times one statement at one
call site ran in the request. The trigger that keeps the row's `queries` count keeps them as each
event is inserted, so the request list draws a time split and a repeated-query badge per row with no
read of `events`. What counts as one statement repeating is
`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`'s definition, so the editor's
lens and the viewer's badge agree. The time in the program's own code is the duration less the
summed kinds, never below zero, and is computed where it is drawn. A new column raises the schema
version, and the index is rebuilt from the spool, which is what a cache is for
(`rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`).

**Every answer is a bounded read:**

| Route | Answers | Bound |
|---|---|---|
| `GET /api/requests` | the page of requests, filtered by `method`, `status` (a code, or a class such as `5xx`), `route`, `min_us` and `threw`, each an indexed predicate | the page size |
| `GET /api/summary` | twelve buckets over the last five minutes of the request count and the error count, and the p50, p95 and largest duration | the newest 10,000 rows of the window, and the answer says when it was cut |
| `GET /api/routes` | per method and route: the count, the error count, the mean queries, the largest duration, when it was last seen, p50 and p95 | 200 routes by count; each route's percentiles read its newest 500 durations through the index `requests_route` on `(method, route, start_us)` |
| `GET /api/statements` | per call site: the newest statement text, the count, the total and the largest time, the requests it reached, and the requests in which it repeated | 200 sites by total time, through the index `events_site` |
| `GET /api/health` | the counts of `dropped`, `unparseable` and `quarantine` rows, and whether a replay target is known | three counts |

**A percentile is computed in Rust,** from the sorted durations the bounded read returned, because
SQLite has no percentile function. The answer names how many durations it read, so a reader can see
when a percentile stands on few requests.

**No route changes a record,** and each is a `GET` on the service's one read connection, with the
host, origin and token checks every route has
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`). `crates/nvs-cli/tests/debug_api.rs`
writes one fixture per route under `viewer/test/fixtures/`, and `viewer/test/api.test.ts` checks the
viewer's types against each.

- **Memory:** in the index, eight integers per request row and the entries of `requests_route`, under
  `[debug] index_max`. In the service, at most 10,000 durations for a summary and 500 per route for
  the routes answer, freed with the answer.

**Declined:**
- **An aggregate with no bound.** A count over the whole index grows with total traffic, and the
  viewer's cost must not.
- **A second store for aggregates.** A table of running totals has to be kept equal to the index
  through every deletion, and a bounded read of the index already answers.
- **Computing the time split from `events` at read time.** The list would read every event of every
  row on screen, at every push of the watermark.
