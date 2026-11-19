With `[debug] store` on, each `Core\Db` call that sends a statement gets a code lens read from the debug
service's index (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`). It describes the
newest recorded request whose trace reached that site: how many statements ran there, their total time
and the slowest one. When one statement text ran at the site more than once in that request, the lens
says it repeated and how many times, which is the N+1 pattern a query inside a loop makes. A click
opens that request in the viewer, at the `[debug] listen` address of the program's configuration
tree, through the editor's own `vscode.open` command, which a client names in
`capabilities.experimental.commands` as it names the others. The server sends no command to a client
that did not name it, and the lens is then a title alone.

The same lens sits on four more kinds of call, the ones `nvs_stdlib::registry::RECORDED_SITE_MEMBERS`
lists: an outbound HTTP call on `Core\Http\Client`, a read or write on a `Core\Cache\Store`, a read or
write of `Core\Session`, and a `Core\IO` member that opens a file. The debug index records each as an
event of kind `http`, `cache`, `session` or `file`, with the same site columns a query has, and the
lens names the kind it counts: "5 HTTP calls, 12.0 ms in total, slowest 4.1 ms". Where a query
repeats its statement text, the other kinds repeat their `target`: the host and path, the cache or
session key, or the file path. A loop that opens one file many times reads "one file repeated 40
times". A site's kind is the kind of the newest request's events there, and an event of any other
kind at the line is not counted with it. The target is never a value the call sent or read.

A query is grouped by its call site, the file, line and file hash its traced span records
(`rule:observability/a-query-span-names-its-call-site`), never by its text. So the figures are a few
numbers per site a request reached, however many times a loop ran the query, which is
`rule:testing/debug-surface`'s argument for line counters. The debug index looks a site up on an
index of its own over the hash and the line. The statement is shown as written, its placeholders
still placeholders: no parameter value is recorded or shown
(`rule:observability/a-query-is-a-trace-event`).

Each site carries the hash of its file's text when the program was compiled, and a record whose hash is
not the hash of the text the editor holds is not shown, so an unsaved edit hides the lens until a
request runs the new text. A lens never sits on a line that ran something else.

The server opens the index read-only, from the data folder, and makes no network request. An index
that is absent, or that the service holds locked, gives no lens and no error.

- **Memory:** in the editor process, a few numbers per query site per lens request, freed with the
  answer, and the start and end of each call the lens can sit on in the reference index. In the
  spool and the index, one file, line and hash per traced site event, one target per event of the
  four other kinds, and one entry of the site index, under the service's byte cap.

**Declined:**
- **One parameter set kept per site.** No query span holds a parameter value, and a parameter is
  where request input and secrets are.
- **Variable values shown inline from recorded requests.** Recording values grows with every iteration
  and every request. A conditional breakpoint or a logpoint in `nvs dap` answers the same question on
  demand.
- **Picking the request in the viewer.** It needs state shared between the viewer and the editor. The
  newest request that reached the site is the one a developer just sent.
