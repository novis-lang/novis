A request's page in the viewer shows everything its record carries, one tab per kind of thing, and
draws every body as text or as a value tree, so nothing a request sent or received ever runs in the
viewer.

**The header** names the method, the route and the path, the status, the duration and when it
started, with *Replay* as the page's one primary action
(`rule:testing/two-requests-compare-and-a-replay-asks-first`), *Make a test*
(`rule:testing/a-recorded-request-becomes-a-test`) and *Copy as curl*. Under it is the request's time
split: one bar of its duration divided by event kind, in the tones of
`rule:testing/the-viewer-wears-the-websites-palette`, read from the `requests` row's per-kind sums
(`rule:testing/the-viewer-answers-per-route-and-per-statement`).

**The tabs** are Timeline, Request, Response, Queries, Dumps, Logs and Coverage, and Exception when
the request threw. A tab with nothing to show is drawn disabled with its count of zero, so the row of
tabs has the same shape on every request. The tab is part of the link,
`/requests/<trace id>/<span id>?tab=queries`, so a lens or a colleague can open a request at the
answer.

- **Timeline** draws every event of the request on one time axis: `query`, `http`, `cache`,
  `session`, `file`, `gc`, `spawn` and the program's own `call` spans, each kind in its tone.
- **Request** and **Response** show the headers as a table and the body as text. A body whose type
  is JSON is parsed in the page and drawn with the value tree; a body that does not parse is drawn as
  text. An HTML body is drawn as text. A body the record cut short says how many bytes it had.
- **Queries** groups the statements by call site, never by text
  (`rule:observability/a-query-span-names-its-call-site`), with each site's count, total and slowest
  time, and the site as a link to the editor
  (`rule:testing/a-site-in-the-viewer-opens-in-the-editor`). A statement that ran more than once at
  one site in the request is marked in gold with its count. What counts as repeated is defined once,
  by `rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`, so the editor's lens and
  the viewer's mark agree.
- **Dumps** and **Logs** list the request's dumps and log records with their sites; **Coverage**
  is the request's lines as `rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`
  draws them; **Exception** is the class, the message, the site and the group.

**Copy as curl** builds the command from the recorded input, with `[redacted]` where the record has
it, as a test made from the request does
(`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`). It copies text and runs
nothing.

**Nothing on the page runs.** Every value reaches the DOM as text, and the page's
`Content-Security-Policy` admits no inline script, no frame and no other origin
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).

- **Memory:** in the browser, the one request's record and the parsed tree of the body on the open
  tab, freed when another request opens. Nothing on the service.

**Declined:**
- **Drawing an HTML response in a frame.** A frame runs the response's scripts and loads its
  resources, which turns a recorded attack into a live one inside the viewer.
- **Drawing an image body.** An image decoder is attack surface, and the size and type in the headers
  answer what a developer checks.
- **One page per kind instead of tabs.** Every kind is part of one request, and a link with `?tab=`
  opens any of them as directly.
