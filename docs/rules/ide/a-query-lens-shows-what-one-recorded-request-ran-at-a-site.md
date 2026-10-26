With `[debug] store` on, each `Core\Db` call that sends a statement gets a code lens read from the debug
service's index (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`). It describes the
newest recorded request whose trace reached that site: how many statements ran there, their total time
and the slowest one. When one statement text ran at the site more than once in that request, the lens
says it repeated and how many times, which is the N+1 pattern a query inside a loop makes. A click
opens that request in the viewer.

A query is grouped by the site id the compiler gave its call
(`rule:observability/a-query-span-names-its-call-site`), never by its text. So the figures are a few
numbers per site a request reached, however many times a loop ran the query, which is
`rule:testing/debug-surface`'s argument for line counters. The statement is shown as written, its
placeholders still placeholders: no parameter value is recorded or shown
(`rule:observability/a-query-is-a-trace-event`).

Each site carries the hash of its file's text when the program was compiled, and a record whose hash is
not the hash of the file on disk is not shown. A lens never sits on a line that ran something else.

The server opens the index read-only, from the data folder, and makes no network request. An index
that is absent, or that the service holds locked, gives no lens and no error.

- **Memory:** in the editor process, a few numbers per query site per lens request, freed with the
  answer. In the spool and the index, one file, line and hash per traced query, under the service's
  byte cap.

**Declined:**
- **One parameter set kept per site.** No query span holds a parameter value, and a parameter is
  where request input and secrets are.
- **Variable values shown inline from recorded requests.** Recording values grows with every iteration
  and every request. A conditional breakpoint or a logpoint in `nvs dap` answers the same question on
  demand.
- **Picking the request in the viewer.** It needs state shared between the viewer and the editor. The
  newest request that reached the site is the one a developer just sent.
