The viewer renders the correlated request stream and the log, and every artifact that an existing
tool already renders is handed to that tool rather than drawn.

It is held to the standard of a tool every developer keeps open all day: it is the window a Novis
developer watches their program through, so it is fast at any size of index, works from the keyboard
alone, and never shows a value in a way that runs it. A view that is slow, a list that jumps while it
is read, or a link that stops working after a restart is a bug in this rule's sense, not a polish item.

A profile is handed to speedscope in the format `rule:observability/speedscope-timeline-export`
already commits to, and a coverage report to whatever reads Clover or lcov. A coverage **heatmap**
over source is drawn, because that is a per-line background over text rather than a viewer. This is
`rule:ide/the-extension-builds-no-ui-the-editor-already-has`'s test applied rather than set aside:
where a renderer exists, feed it. The live, correlated request stream is the one thing here that has
no incumbent in any ecosystem, which is the whole of the exception. The boundary is written as a rule
because the failure mode of an in-house dashboard is that it slowly grows a second copy of every tool
the project deliberately did not build.

**The request is the primary object.** The main view lists requests — method, route, status,
duration, query count, dump count and whether it threw — and one opens to its own timeline: the
calls, queries, spawns, garbage-collection pauses, dumps and log lines of that request on one time
axis. The flat filterable stream and the application's log are the second and third views, not the
front page. A request is keyed by its trace id and the span id the server drew for it
(`rule:observability/a-trace-id-exists-for-every-request`), because an inbound `traceparent` can give
two requests one trace id. The response carries both in its `traceresponse` header unless
`[trace] response_header = false`, so pasting that header's value, or a bare trace id, into the
viewer's search opens the request it names.

**The baseline every view has.** The live view follows new records and pauses at a keystroke or
when the developer scrolls away from the newest row, and a paused view counts what arrived without
moving what is on screen. Every view and every request has a link that survives a restart of the
service and a reload of the tab: `/requests/<trace id>/<span id>`, and a filter is part of its view's
link. Every action is reachable from the keyboard, `?` lists the keys, and lists and value trees
follow the WAI-ARIA listbox and tree patterns. A dark and a light theme follow the browser's
`prefers-color-scheme` and can be fixed in the viewer's settings, both at WCAG AA contrast. A long list
keeps only the rows on screen in the DOM, so the browser's cost does not grow with the index.

Four features complete it, each with its own rule: full-text search with saved filters
(`rule:testing/the-viewer-searches-every-record-and-saves-a-filter`), exceptions grouped by class and
site (`rule:testing/an-exception-is-one-row-per-class-and-site`), comparing two requests and replaying
one (`rule:testing/two-requests-compare-and-a-replay-asks-first`), and every `file:line` opening in the
editor (`rule:testing/a-site-in-the-viewer-opens-in-the-editor`).

The value tree the browser receives is already finite — `rule:errors/record-transformations` bounded
it with elision and resolved every repeat to an identity before it was written — so a repeat renders
as a link to the node it names and expansion needs no fetch. Children are built from data already
loaded.

**The front end is TypeScript and Solid**, in the repository's top-level `viewer/` folder, a Bun
workspace of the root package so the root `bun.lock` pins it. `solid-js` is its one runtime
dependency, at a caret range. Bun bundles it into one JavaScript file and one CSS file, with a
bundler plugin running Solid's JSX preset at build time, because Bun's own JSX transform has only the
React-style runtimes. The two files are committed under `viewer/dist/`, `bun nv viewer --check`
rebuilds them and fails on any byte difference, and the crate that serves `nvs debug` embeds them with
`include_bytes!`. A Rust build therefore never runs Bun, and the bytes in a binary are the reviewed
ones. Solid's licence is reproduced in the notice the binary carries
(`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`). A second runtime
dependency is a change to this rule, because each one ships in every binary.

The browser talks to one JSON API on the same listener. The Rust side owns its shapes: a test writes a
fixture from a real index, the TypeScript types are checked against that fixture by `tsc`, and the
browser tests run against it, so the two halves cannot drift without a failing build.

**The listener is the service's own.** The viewer is a subcommand, `nvs debug`, binds
`127.0.0.1:8137` by default, a fixed port so a link outlives a restart, and is never mounted on the
application's listener: there every path-normalization bug and proxy misconfiguration would become
disclosure, and a reserved path prefix would collide with the compile-time route table. `[debug]
listen` and `nvs debug --listen` move it. A taken port stops a service started by hand with a message
naming both; a service a development server started with `--parent` says the same as a warning and
ingests on without the viewer, because the server's records still need it. It is `hyper` and
`rustls`, both already in the binary, so it adds no HTTP or TLS crate. `nvs debug` has no core, so each
connection has a thread of its own whose reads and writes block, bounded in count, in read time and in
write time, and `hyper` is driven by `nvs_host::block_on`. `half_close` keeps `hyper` from reading
while it writes, so an event stream waits on the watermark alone. The page is one `<div id="app">`
with `/viewer.css` and `/viewer.js`, served for every path outside `/api/`.
`crates/nvs-cli/src/debug/listen.rs` is the listener.

**Off loopback, `nvs debug` secures itself.** `[debug] listen` is the one key a container needs. With
an address off loopback, `nvs debug` generates a self-signed certificate with `rcgen`, already in the
graph, and a random token, keeps both in the data folder's `debug/` directory, private to the
service's account, and prints a URL that carries the token. The browser warns about the certificate
once. `[debug] tls_cert_file`, `tls_key_file` and `token_file` are optional overrides for someone who
brings their own, and boot does not fail when they are absent. The token is accepted once from the
printed URL and then lives in an `HttpOnly`, `Secure`, `SameSite=Strict` cookie, and a request without
that cookie is refused. A bind off loopback never serves plain HTTP and never serves without a token.
`crates/nvs-cli/src/debug/secure.rs` makes and loads the certificate and the token.

Because it is a browser-facing local listener, `Host` and `Origin` are
allowlisted against DNS rebinding. On loopback a `Host` names `localhost` or a loopback address with
the bound port. Off loopback a published port and a container's name change the host a browser uses,
so any well-formed `Host` passes and the token cookie decides, which a rebound name never carries. An
`Origin` that is sent must be the page's own, a request that changes anything needs one and a
JSON body, and every page is sent with a `Content-Security-Policy` that admits no inline script, no
inline style, no `eval` and no other origin. Its renderer builds DOM rather than assigning markup: no
value from a record reaches `innerHTML`, and Solid's compiled templates are constant strings fixed at
build time. It runs no Novis code.

The viewer is pushed a watermark as server-sent events on `/api/events`
(`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`): the ingest loop publishes the
last committed batch after every pass that read something, and a stream sends the newest number it
has not sent, or a comment every 15 seconds so a closed tab is found. Every list is a page keyed on
that sequence, so reading the newest page costs the same with a hundred requests in the index as with
a million.

The viewer's settings — the theme, the editor link, path mappings and saved filters — are one file
beside the index in the data folder, outside the index, so discarding and re-ingesting the index keeps
them, and a restart on the same port finds them.

- **Memory:** the two bundled files are read-only data in every binary, and a server process never
  pages them in, because only `nvs debug` reaches them. `nvs debug` holds one read connection to the
  index, one thread per open connection up to 64, and one sequence number per open tab. The browser
  holds one page of rows and the request it shows.

**Not on disk.** The listener serves the page, the two files and `/api/events`, and every other
`/api/` path is a `404`: the JSON API, its fixture, the views, search, grouping, compare, replay and
the settings file are not built. The one hand-off built is the editor's: the VS Code extension reads
the lcov file `nvs test` writes (`editors/vscode/src/tests.ts:237`).

**Declined:**
- **Fetching a value tree lazily as the developer expands it.** The tree is already bounded before it
  is written, so a fetch per expansion would add round trips and server-side state to make a bounded
  thing safe again.
- **Building the bundle from a Cargo build script.** Every Rust build, on every CI leg and every
  contributor's machine, would need Bun and the viewer's dependencies installed.
- **A second runtime dependency for a virtual list, a chart or a component kit.** Each one ships in
  every binary and widens what the notice and the audit cover. The viewer draws them with Solid.
- **A WebSocket for the watermark.** The push is one integer in one direction, and server-sent events
  carry it over the same HTTP connection with no upgrade and no framing to keep correct.
- **A port chosen at random on each start.** Every link and every open tab would break on a restart.
