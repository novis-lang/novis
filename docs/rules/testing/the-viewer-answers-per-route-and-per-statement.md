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
| `GET /api/requests` | the page of requests, filtered by `method`, `status` (a code, or a class such as `5xx`), `route`, `min_us`, `min_memory` and `threw`, each a bound parameter | the page size, and a filtered page reads at most 20,000 request ids below its cursor; its `older` then names where it stopped |
| `GET /api/runs` | the page of runs (`rule:testing/a-run-is-recorded-like-a-request`), filtered by `of`, `name`, `outcome`, `min_us` and `min_memory`, paged on the run's id as the requests are | the requests' bound, over run ids |
| `GET /api/run?trace=` | the newest run of the trace, its uncaught exception with its stack, and every event, dump and log line of its trace | 5,000 events, 200 dumps and 1,000 log lines, as one request's answer, and `cut` says which list stopped |
| `GET /api/mails` | the page of kept mail (`rule:core-classes/mail-transport`), newest first, paged on the row id with `before`, `since` and `limit`; each row has the block, the subject and the `to` addresses, and no body | the page size |
| `GET /api/mail?id=` | one kept message's whole `mail` line: its headers, its text, its HTML as written and as sent, its CSS report and its attachments as base64 | one line, which the spool bounds to its line size |
| `GET /api/summary` | twelve buckets over the last five minutes of the request count and the error count, and the p50, p95 and largest duration | the newest 10,000 rows of the window, and the answer says when it was cut |
| `GET /api/routes` | per method and route: the count, the error count, the mean queries, the largest duration, when it was last seen, p50 and p95 | the newest 10,000 requests and 200 routes by count, and the answer says when older requests were not counted; each route's percentiles read its newest 500 durations through the index `requests_route` on `(method, route, start_us)` |
| `GET /api/statements` | per call site: the newest statement text, the count, the total and the largest time, the requests it reached, and the requests in which it repeated | the newest 100,000 event ids and 200 sites by total time, and the answer says when older events were not read |
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
