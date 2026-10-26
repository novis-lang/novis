---
looper: 1
title: "The debug viewer shows every request, its timeline, its logs and its exceptions, and is the tool a developer keeps open"
summary: "nvs debug serves a TypeScript and Solid viewer embedded in the binary, on a listener only its own page can use; the request list follows live, a request opens to its timeline, the stream and the log are searchable with saved filters, exceptions are grouped, every file:line opens in the editor, two requests compare and one replays after asking, the viewer has its feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md, docs/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.md, crates/nvs-config/src/default.toml]
stages:
  - id: 1
    title: "the viewer's front end is built from `viewer/` and embedded in `nvs`"
    status: done
    context: [package.json, tsconfig.json, CONTRIBUTING.md, tools/nv/main.ts, tools/nv/cmd/verify.ts, tools/nv/cmd/ci-changes.ts, tools/nv/cmd/gen-attribution.ts, crates/nvs-cli/src/info.rs, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md, docs/rules/packaging/the-third-party-notice-is-generated-never-written-by-hand.md]
    extra:
      context:
        rules: [testing/the-viewer-shows-the-stream-and-hands-off-every-artifact, packaging/the-third-party-notice-is-generated-never-written-by-hand, packaging/the-notice-is-embedded-in-the-binary, testing/attribution-is-diffed-in-ci]
  - id: 2
    title: "`nvs debug` serves the viewer, and only to its own page"
    status: done
    context: [crates/nvs-cli/src/debug.rs, crates/nvs-cli/src/debug/listen.rs, crates/nvs-cli/Cargo.toml, crates/nvs-config/src/default.toml, crates/nvs-config/src/data.rs, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md, docs/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.md]
    extra:
      context:
        rules: [testing/the-viewer-shows-the-stream-and-hands-off-every-artifact, testing/the-ingester-runs-whether-or-not-anyone-is-looking, concurrency/one-scheduler, config/ownership-is-the-trust-boundary]
  - id: 3
    title: "the request list follows live, with the baseline every view reuses"
    status: done
    context: [crates/nvs-cli/src/debug/api.rs, crates/nvs-cli/src/debug/index.rs, crates/nvs-server/src/trace.rs, viewer/src/app.tsx, viewer/src/api.ts, tools/nv/test/chromium.ts, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md]
    extra:
      context:
        rules: [testing/the-viewer-shows-the-stream-and-hands-off-every-artifact, observability/a-trace-id-exists-for-every-request]
  - id: 8
    title: a request opens to its timeline and its value trees
    status: done
    context: [crates/nvs-cli/src/debug/api.rs, crates/nvs-runtime/src/ctx/trace.rs, viewer/src/views/request.tsx, viewer/src/views/timeline.tsx, viewer/src/views/value-tree.tsx, tools/nv/test/chromium.ts, docs/rules/errors/record-transformations.md]
    extra:
      context:
        rules: [testing/the-viewer-shows-the-stream-and-hands-off-every-artifact, errors/record-transformations, observability/trace-events-carry-a-kind]
  - id: 4
    title: "the stream, the log and full-text search with saved filters"
    status: done
    context: [crates/nvs-cli/src/debug/api.rs, crates/nvs-cli/src/debug/settings.rs, crates/nvs-config/src/data.rs, viewer/src/views/stream.tsx, viewer/src/views/search.tsx, docs/rules/testing/the-viewer-searches-every-record-and-saves-a-filter.md, docs/rules/testing/a-bad-line-degrades-and-a-bad-file-is-quarantined.md]
    extra:
      context:
        rules: [testing/the-viewer-searches-every-record-and-saves-a-filter, testing/a-bad-line-degrades-and-a-bad-file-is-quarantined, testing/the-ingester-runs-whether-or-not-anyone-is-looking]
  - id: 5
    title: "grouped exceptions, and every site is a link into the editor"
    status: done
    context: [crates/nvs-cli/src/debug/api.rs, viewer/src/views/exceptions.tsx, viewer/src/editor-link.ts, docs/rules/testing/an-exception-is-one-row-per-class-and-site.md, docs/rules/testing/a-site-in-the-viewer-opens-in-the-editor.md]
    extra:
      context:
        rules: [testing/an-exception-is-one-row-per-class-and-site, testing/a-site-in-the-viewer-opens-in-the-editor]
  - id: 9
    title: "a profile is handed to VS Code, and a request's coverage is a heatmap over its source"
    status: done
    context: [editors/vscode/src/extension.ts, editors/vscode/package.json, editors/vscode/src/tests.ts, crates/nvs-cli/src/debug/source.rs, viewer/src/views/coverage.tsx, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md]
    extra:
      context:
        rules: [ide/the-extension-builds-no-ui-the-editor-already-has, observability/speedscope-timeline-export, testing/the-viewer-shows-the-stream-and-hands-off-every-artifact]
  - id: 6
    title: "two requests compare, and one replays after the developer confirms"
    status: open
    context: [crates/nvs-cli/src/debug/replay.rs, crates/nvs-cli/src/debug/api.rs, viewer/src/views/compare.tsx, docs/rules/testing/two-requests-compare-and-a-replay-asks-first.md, docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md]
    extra:
      context:
        rules: [testing/two-requests-compare-and-a-replay-asks-first, testing/the-debug-stream-reaches-the-service-through-the-disk, testing/writing-a-debug-record-to-disk-is-its-own-gate]
  - id: 7
    title: "the viewer has its feature proofs, and its rules are shipped"
    status: open
    context: [docs/reference/tools/10-cli.md, data/proofs/skip/tools/cli/nvs-info.json, docs/rules/testing/feature-proofs.md]
    extra:
      context:
        rules: [testing/feature-proofs, testing/hostile-case-contract, testing/examples-live-in-the-repository]
  - id: 10
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: owners-closes
    stage: 7
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, debug-viewer]
    reach: true
    floor: false
---

# The debug viewer shows every request, its timeline, its logs and its exceptions, and is the tool a developer keeps open

## Why

The user's words: "The viewer need to be top notch. That will be one of the main tools every developer
has open all the time." `debug-service` writes every request's records, query spans, dumps, logs and
input into an index, and nothing shows them: there is no viewer and no `viewer/` folder
(`docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md`). A developer today
reads a dump in the response body or a log line in a terminal, with nothing joining the two to the
request that made them. This goal builds the window a Novis developer watches their program through:
the request list, each request's timeline, the stream, the log, search, grouped exceptions, links into
the editor, compare and replay. It is held to the standard the user set, which
`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact` now states: fast at any size
of index, usable from the keyboard alone, and never showing a value in a way that runs it.

It needs `debug-service` (`nvs debug`, the spool, the ingester and the index schema) and
`trace-timeline` (a start and an end on every trace event), stage by stage as the gates below say.
`request-aware-editor` comes after it and adds the button that makes a recorded request a test.

## Standing decisions

**The viewer is its own goal, written in TypeScript and Solid and embedded in the binary.** The user
decided this. Bun bundles it into one JavaScript file and one CSS file, `nvs` embeds them with
`include_bytes!`, and `solid-js` is a dependency at a caret range with the lock committed (AGENTS.md
§ *Where Novis departs from the stack files*).

**Four features beyond the stream are part of it.** The user decided this: every `file:line` opens in
the editor, `vscode://` by default with a setting for others
(`rule:testing/a-site-in-the-viewer-opens-in-the-editor`); full-text search over requests, dumps, logs,
queries and exceptions with saved filters
(`rule:testing/the-viewer-searches-every-record-and-saves-a-filter`); exceptions grouped one row per
class and site with a count, first and last seen and links to their requests
(`rule:testing/an-exception-is-one-row-per-class-and-site`); and compare and replay, asking before a
replay because it runs side effects again (`rule:testing/two-requests-compare-and-a-replay-asks-first`).

**`traceresponse` is sent by default and `[trace] response_header = false` turns it off.** The user
decided this, and `debug-service` stage 1 builds it. The viewer rule's sentence about the header says
so already, and a pasted header value opens its request.

**In a container, `nvs debug` secures itself.** The user decided this. With `[debug] listen` off
loopback, `nvs debug` generates a self-signed certificate with `rcgen`, already in the graph
(`Cargo.toml:333`), and a random token, and prints a URL carrying the token. The browser warns about the
certificate once. `listen` is the one required key. `tls_cert_file`, `tls_key_file` and `token_file`
stay as optional overrides for someone who brings their own, and boot does not fail when they are
absent. The generated pair is kept in the data folder's private `debug/` directory, so a restart keeps
the link and the browser's exception. Trade-offs: one certificate generation at the first start off
loopback, and a browser warning a developer clicks through once.

**The baseline is the planning session's call:** live follow and pause, keyboard navigation with `?`
listing the keys, a dark and a light theme, and a stable link to every request and every filtered view.
So is the folder: the front end lives in a top-level `viewer/`, and its row goes in `CONTRIBUTING.md`'s
layout block, which `bun nv layout --check` holds to the tree.

**How it is built is the planning session's call.** `viewer/` is a Bun workspace of the root package,
so the root `bun.lock` pins it and one `bun install` covers it. Bun's own JSX transform has only the
React-style runtimes (checked in the installed Bun's bundler reference), so a bundler plugin runs
Solid's JSX preset through Babel at build time. The two output files are committed under `viewer/dist/`
and `bun nv viewer --check` fails when they differ from a rebuild, the way `THIRD-PARTY-LICENSES.txt` is
committed and diffed. A Rust build never runs Bun. `solid-js` is the one runtime dependency, and the
notice reproduces its licence.

**`nvs debug` and its listener live in `nvs-cli`.** `debug-service` puts the command and the ingester
in `crates/nvs-cli/src/debug.rs`, so the listener is `crates/nvs-cli/src/debug/listen.rs` and the API
`crates/nvs-cli/src/debug/api.rs`, and the embedded bundle is `crates/nvs-cli/src/viewer.rs`, beside
`info.rs`'s embedded notice. `nvs-cli` takes `hyper` from the workspace (`Cargo.toml:193`); `rustls` and
`rcgen` it already has (`crates/nvs-cli/Cargo.toml:133`). Why: the application server's crate never
links a line of the viewer, which keeps the request path untouched, and no crate is added. Safe default
if a module grows past one concern: another file under `crates/nvs-cli/src/debug/`, never a new crate.

**The fixed default port is `127.0.0.1:8137`.** Nothing in the tree binds it: the server's default is
`8000` (`crates/nvs-config/src/default.toml:512`), and tests use `8080`, `8099`, `8443` and `9000`-`9100`.
A fixed port keeps a link valid across a restart, which the rule requires. When the port is taken,
`nvs debug` exits with a message that names `[debug] listen` and `--listen`, and never falls back to a
random port (the rule's Declined list).

**The listener's API is the planning session's call.** `hyper` and `rustls`, both already in the graph
(`Cargo.toml:193`, `Cargo.toml:316`), so `nvs debug` adds no HTTP or TLS crate. The watermark is pushed
as server-sent events on `/api/events`. Every list is a page keyed on the watermark sequence. The
Rust side owns the API's shapes: a Rust test (`crates/nvs-cli/tests/debug_api.rs`) writes one JSON
fixture per view under `viewer/test/fixtures/` from a real index, and `tsc` and the browser tests read
that fixture. A request is keyed by its trace id and its span id, because an inbound `traceparent` can
give two requests one trace id.

**A page's cursor is the row id, and the watermark says when to fetch.** Stage 3's call. A batch can
hold more rows than a page, so the watermark alone cannot page. `requests.id` is `AUTOINCREMENT`
(index schema 3), so it only rises in commit order and never names a newer row once the deletion
window empties the table. `before=<id>` is the next older page, `since=<batch>` the live delta, and the
watermark and the page are read in one transaction. The spool's `request` line carries the server's
`span`, and `trace`/`span` filters open one request. Trade-offs: one `sqlite_sequence` update per
ingested request, off the request path; one read connection behind a mutex, which a page's bounded
query keeps short.

**The listener is a thread per connection over `std` sockets.** Stage 2's call. `nvs debug` has no core
and no reactor, so the host's parking sockets would only block the thread anyway. Each connection's
thread blocks in its reads and writes, `nvs_host::block_on` drives `hyper` over them, and `half_close`
keeps `hyper` from reading while it writes, so an event stream waits on the watermark alone. Bounds: 64
connections, 60 s per read, 30 s per write, and a comment every 15 s so a closed tab ends its thread.
An event stream's body yields once after each event, because `hyper` keeps polling an always-ready body
without flushing. Trade-offs: a thread per open connection where a reactor would need none, bought for
no scheduler in `nvs debug` and no `Pending` path to get right.

**Off loopback any well-formed `Host` passes, and the token cookie decides.** Stage 2's call. A
published port and a container's name change the host a browser uses, so no list fits, and a page that
rebinds its own name to the service never carries the `__Host-` cookie. On loopback, `Host` must be
`localhost` or a loopback address with the bound port.

**A `--parent` service on a taken port ingests on without the viewer.** Stage 2's call. Development
servers in parallel tests, or two projects on one machine, each start a service on `127.0.0.1:8137`, and
the server's records still need ingesting. A service started by hand stops instead, as the rule says.

**The browser tests use the repository's Chromium driver.** The planning session's call, corrected
against the tree: Playwright's own driver does not launch under Bun on Windows, so
`tools/nv/test/chromium.ts` drives the headless shell `playwright-core` downloads over the DevTools
protocol (`tools/nv/test/chromium.ts:5-11`), and CI's `browser` job runs it with `NV_BROWSER` set
(`.github/workflows/ci.yml:494`). A stage runs only its own view's file, `NV_BROWSER=1 bun test
viewer/test/<view>.browser.test.ts`, never the whole browser suite; the extension's part runs
`npm run test:headless` in `editors/vscode`.

**The details inside the four features are the planning session's calls**, written in their rules: an
exception's site is the file and line of its `throw`; a link is built only from a structured site,
never from text in a message; the editor setting and the saved filters live in one settings file beside
the index in the data folder; a replay goes only to the development server that started the service,
asks every time with cancel focused, and sends a redacted header only if the developer types its value.

**The viewer's perf proof is a count, not a `.nvs` bench.** The planning session's call. The viewer
runs no Novis code, so a bench under `benches/members/` would measure nothing of it. A Rust test counts
the SQLite steps of the newest page at two index sizes and fails if they grow, and the proofs policy
names that test in a `perf` skip.

**A request's records are its trace's, narrowed by writer when the trace is shared.** Stage 8's call.
`events`, `dumps` and `logs` carry a trace id and no span id, so `GET /api/request` gives every record
of a trace that names one request. When several share it, a request's records are the ones its writer
wrote after the previous request of that trace and before its own `request` line, and a tailed log line
is the one inside its time. Two such requests in flight on one thread at once cannot be told apart.
Each list is capped (5000 events, 200 dumps, 1000 log lines) and `cut` says which stopped. A tree is
passed through as the index stores it (`RawValue`). Trade-offs: no schema change and no new spool field,
for an exact split in every case but concurrent requests that share a trace id on one thread.

**Gates.** Stage 1 needs no other goal. Stage 2 needs `nvs debug` to exist: `crates/nvs-cli/src/debug.rs`
is on disk (`debug-service` stage 4). Stages 3, 4, 5 and 6 need the index: stage 4 of
`docs/goals/debug-service.md` is `done`, or that file is gone; stage 4's quarantine list also needs its
stage 5, and stage 6's replay target its stage 6. Stage 8 needs a start and an end on every trace event:
stage 1 of `docs/goals/trace-timeline.md` is `done`, or that file is gone. Stage 9's heatmap needs the
coverage map of `rule:testing/debug-surface`, stage 2 of `docs/goals/trace-timeline.md`. A session whose
gate fails takes stage 1, then stage 2, which need nothing else, and says so in the handoff.

**Lanes.** Stage 1 (TypeScript build in `viewer/` and `tools/nv/`) and stage 2 (Rust listener in
`crates/nvs-cli/src/debug/listen.rs`) touch disjoint files and run in any order or side by side. In
stages 3, 8, 4, 5 and 6 the Rust API and its JSON fixture go first; the TypeScript view that reads the
fixture is then a lane of its own. Those stages share `crates/nvs-cli/src/debug/api.rs` and
`viewer/src/app.tsx`, so they run in the listed order. Stage 9's extension part (`editors/vscode/`) is
its own lane, disjoint from every other stage. Stage 7 runs last.

**The page's shape is stage 1's call, and stage 2 serves it.** The page is one `<div id="app">`,
`/viewer.css` and `/viewer.js` loaded as a module, served for every view path; the JSON API and the
event stream are under `/api/`. `bun nv viewer --dev` serves that same shape on `127.0.0.1:8138` and
passes every `/api/` request to `nvs debug` with `Origin` rewritten to the service's own, so stage 2's
allowlist admits nothing for it. `crates/nvs-cli/src/viewer.rs` holds the two files as `JS` and
`CSS`. `bunfig.toml` keeps Bun's hoisted `node_modules`, since a workspace otherwise switches the
whole install to one `node_modules` per package and the tools import from the root.

**An exception group is named by its site, and its row reads a bounded sample.** Stage 5's call.
`GET /api/exceptions?class=&file=&line=` names a group the way its row does, because a group's id
changes when the deletion window ends it or a discard rebuilds the index. Each row's latest message,
24-count chart (equal spans from first to last seen) and routes come from at most 1000 of the
group's newest throws on `exceptions_group`, and `sampled` says when older throws are left out.
Groups change in place, so the viewer reads the first page again at each watermark and pages on
`<last seen>.<id>`. Trade-offs: one bounded indexed read per row on a page, for a page whose cost
does not grow with a group's count.

**The editor setting is a preset name, a template and mappings, and the viewer builds the link.**
Stage 5's call. `editor` is `vscode` (the default), `vscode-insiders`, `vscodium`, `cursor`, `zed`,
`idea`, `phpstorm` or `custom`; the JetBrains presets are the `idea://open?file=&line=` and
`phpstorm://open?...` links, and any other JetBrains IDE takes the template. `editor_template` is
read only for `custom`, and the service and the viewer both refuse a `javascript:`, `data:`,
`vbscript:` or `blob:` scheme. `path_mappings` replace the longest matching prefix. The `://file/`
presets drop a path's leading `/`, so a POSIX path is not read as a UNC host. Trade-offs: none on
the request path; one small settings file.

**A profile reaches the index as a spool `profile` line, and the page downloads it by row.** Stage
9's call. `Core\Debug::startProfile` writes the file's absolute path once it has created the file,
under `[debug] store`. The index keeps it in `profiles` (schema 4). `GET /api/request` lists a
request's profiles, and `GET /api/profile?id=` sends one row's file, at most 256 MiB. The page
names a row and never a path. The `vscode://novis-lang.nvs/profile?file=` link is offered for
`vscode`, `vscode-insiders`, `vscodium` and `cursor`, which run the extension, and only for a
`.speedscope.json` file, the one name the extension opens. Every profile also has a download. The
extension refuses a relative path, a network path and any other file. Trade-offs: one spool line
per started profile, off the request's answer; a download holds its file in `nvs debug`'s memory
while it is sent.

**A request's coverage is counted by a development server whose `[debug] mode` is `["coverage"]`.**
Stage 9's call. `rule:testing/debug-mode-directive` refused every probe on a server, so the rule
is amended: a server whose `[mode] default` is `development` may name `coverage` alone, and any
other probe, or `coverage` in production, still fails the boot (`nvs_config::debug::refuse_serving`,
`counts_served`). Whether a server counts is fixed at boot; a reload that changes it is refused
(`refuse_switch`). The serve compiler (`Compiler::counting`) builds a located `StmtHits` beside each
unit from `Sites::of` at compile, in `nvs-cli`, which is the server's compiler crate, so nothing
moves crate. `program_over` gives each `development` request a zeroed copy; the request's
`Completion::coverage` carries it out, and `nvs_server::spooled` writes a spool `coverage` line
(`spool::Coverage`: span, file to line to count) beside its `request` line, or at the join for a
streamed answer. No modified time rides on the line: `GET /api/source` says a file changed when its
modified time is after the request's `start_us`, since a development server recompiles an edited
file before the next request. Trade-offs: one `u64` per statement and edge and one file and line
per statement per unit, once; one `u64` per statement and edge per development request in flight,
freed with it; nothing where the tree names no `coverage`, which is every production host.

**The project root `GET /api/source` reads under is the working directory `nvs debug` started in.**
Stage 9's call. Configuration is found in the working directory and never by a walk upward
(`nvs_config::resolve::roots`), and the service a development server starts inherits the server's
working directory, so the root is the folder the server was started in. It is resolved once at start
(`debug::source::root`). A file is read only when the coverage row names it and its resolved path,
links followed, is under that root; anything else is a `403`, and a file the row does not name a
`404`. The index keeps a coverage line's files as the line's JSON text, and the API passes it
through. A heat step is one of four on a log scale up to the file's most-run line
(`viewer/src/heat.ts`), and a source is a virtual list of fixed-height lines. Trade-offs: a service
shared by two projects on one data folder shows the source of the one it started in alone; one
source file of at most 4 MiB in `nvs debug`'s memory while it is sent.

**The replay target reaches the service on the pipe its parent already holds.** Stage 6's call.
The child is started before any socket is bound, so it inherits no listener, and a `:0` port is
known only after the bind. The server writes the first TCP address it bound as one line on the
child's standard input (`debug::Child::target`), an unspecified address replaced by the loopback
of its family (`serve::Socket::replay_target`), and the service keeps it in a `OnceLock`
(`debug::Target`). A Unix-only server and a service started by hand have none. `POST
/api/replay` sends HTTP/1.1 by hand over a `std` socket with `Connection: close`, reads only the
answer's head, and returns its status and the `traceresponse` trace and span; a cut or withheld
body is a `409`, never a partial send. Trade-offs: one line on an existing pipe; one recorded
input in memory per replay; a viewer thread waits up to 120 s for a slow handler.

**Compare joins two `GET /api/request` answers in the browser.** Stage 6's call. A compare
endpoint would only return the two answers side by side, so `GET /api/request` gains an `input`
object (query, headers, request body and response headers and body, each body as text or base64
with its `read`, `cut` and `withheld` flags, bodies put together from `body_parts` with the
writer-and-sequence split) and the view diffs the two answers. Trade-offs: no new route; a
request's answer carries its recorded bodies, at most the spool's body cap each.

**The dialog reads its target from `GET /api/replay`.** Stage 6's call. The target belongs to the
listener's site and not to the index, so `GET /api/replay` gives `{"target": "<address>"}` or a
`null` target, beside the `POST` it describes, and `GET /api/request` stays a read of the index
alone. The request page disables Replay with its reason before the dialog opens. A marked request
in the list opens compare with the marked one on the left; the previous request to a route is the
nearest older row with the same method and route on one page of 500, since the list has no route
filter. Trade-offs: one small read per request page; no new request-list filter.

**Trade-offs, all together.** Request path: nothing. The viewer is another process reading the index,
and a server process never reaches its code. Memory: the two bundled files are read-only data in every
binary, never paged in by a server; `nvs debug` holds one read connection and one sequence per open tab;
the browser holds one page of rows. Usability: the main debugging tool, with no setup beyond `[debug]
store`. Simplicity: a third TypeScript project beside the tools and the extension, one runtime
dependency, a committed bundle with a freshness check, and one new `nv verify` step.

## Stages

### 1. the viewer's front end is built from `viewer/` and embedded in `nvs`

**Does:** a Solid app that builds to two committed files, checked for freshness, embedded in the binary,
with its licence in the notice.

- [x] `viewer/` as a workspace of the root package: `solid-js` its one runtime dependency, Babel and Solid's JSX preset at build time, the root `bun.lock` updated, its own `tsconfig.json` with the DOM library, `"workspaces": ["viewer"]` in the root `package.json`, and its row after `editors/` in `CONTRIBUTING.md`'s layout block, in the commit that creates the folder because `bun nv layout --check` fails on a row with no folder (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`). `package.json:11`, `CONTRIBUTING.md:183`, `viewer/package.json:1`, `viewer/tsconfig.json:1`, `viewer/src/main.tsx:1`
- [x] `bun nv viewer` builds `viewer/dist/viewer.js` and `viewer/dist/viewer.css`, `--check` rebuilds and fails on a byte difference, and `--dev` serves the source with reload against a running `nvs debug`. A test that two builds give the same bytes. `tools/nv/main.ts:43`, `tools/nv/cmd/viewer.ts:1`, `tools/nv/test/viewer.test.ts:1`
- [x] `nv verify` gains a `viewer` step, run when a file under `viewer/` changed: `tsc`, the bundle check and the viewer's unit tests. `tools/nv/cmd/verify.ts:162`
- [x] CI's `browser` job gains the viewer's browser suite, and `ci-changes.ts`'s `browser` paths gain `viewer/`. `tools/nv/cmd/ci-changes.ts:106`, `.github/workflows/ci.yml:494`
- [x] `bun nv gen-attribution` reads the viewer's runtime dependencies from `viewer/package.json`'s `dependencies` at the versions `bun.lock` resolves, each licence file from its installed package, and no build-time package. In the same commit the packaging rule's paragraph on what the notice is generated from says so, and its `guardedBy` gains `viewer/package.json` (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`). `tools/nv/cmd/gen-attribution.ts:288`, `docs/rules/packaging/the-third-party-notice-is-generated-never-written-by-hand.md:5`
- [x] `nvs-cli` embeds the two files with `include_bytes!` in a module of their own, as `nvs info` embeds its notice, with a test that the embedded bytes are the committed files. `crates/nvs-cli/src/viewer.rs:1`, `crates/nvs-cli/src/info.rs:34`

### 2. `nvs debug` serves the viewer, and only to its own page

**Does:** a listener on loopback that serves the shell, the two files and the JSON API, refuses every
other host and origin, and pushes the watermark.

- [x] The rule names the fixed default port, `127.0.0.1:8137`. `[debug] listen` and `--listen` change the address, and the shell is served for every view path (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`). `docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md:63`, `crates/nvs-config/src/default.toml:327`, `crates/nvs-cli/src/debug/listen.rs:1`, `crates/nvs-cli/Cargo.toml:133`
- [x] `Host` and `Origin` are allowlisted, a changing request needs an allowed `Origin` and a JSON body, and every response carries the rule's `Content-Security-Policy`, `nosniff` and `no-referrer`. `crates/nvs-cli/src/debug/listen.rs:1`
- [x] Off loopback: a self-signed certificate made with `rcgen` and a random token, generated by `nvs debug` and kept private in the data folder's `debug/`, a printed URL carrying the token, and the token taken once from that URL into an `HttpOnly`, `Secure`, `SameSite=Strict` cookie. `[debug] tls_cert_file`, `tls_key_file` and `token_file` override the generated ones when set, and boot does not fail when they are absent. `crates/nvs-cli/src/debug/listen.rs:1`, `crates/nvs-config/src/data.rs:506`, `Cargo.toml:316`, `Cargo.toml:333`
- [x] The watermark is pushed as server-sent events, one sequence per committed batch, and a slow tab makes nothing buffer (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`). `crates/nvs-cli/src/debug/listen.rs:1`
- [x] The attack, as Rust tests: a rebinding `Host`, a forged `Origin` on a changing request, a non-loopback bind that serves plain HTTP or no token, and a request without the cookie off loopback. `crates/nvs-cli/tests/debug_viewer.rs:1`

### 3. the request list follows live, with the baseline every view reuses

**Does:** the main view, and the baseline every later view reuses.

**Split:** the session lands the API page and its fixture; a writing subagent then writes the list view
and its browser test against the fixture alone.

- [x] The API pages requests by the watermark sequence with method, route, status, duration, query count, dump count and whether it threw. A Rust test writes the JSON fixture from a real index, and `viewer/` types are checked against it. `crates/nvs-cli/src/debug/api.rs:1`, `crates/nvs-cli/tests/debug_api.rs:1`, `viewer/test/fixtures/requests.json:1`, `viewer/src/api.ts:1`
- [x] The list keeps only visible rows in the DOM, follows live, pauses at a key or a scroll and counts what arrived, and moves by keyboard. Dark and light themes, and `?` lists the keys. `viewer/src/app.tsx:1`, `viewer/src/views/requests.tsx:1`
- [x] A request opens at `/requests/<trace id>/<span id>`, and a pasted `traceresponse` value or trace id opens its request (`rule:observability/a-trace-id-exists-for-every-request`). `viewer/src/app.tsx:1`, `crates/nvs-server/src/trace.rs:286`
- [x] Browser tests over the fixture with the Chromium driver, one of them counting the rows in the DOM with many requests in the index. `viewer/test/requests.browser.test.ts:1`, `tools/nv/test/chromium.ts:31`

### 8. a request opens to its timeline and its value trees

**Does:** the request view: one time axis for everything the request did, and its dumped values.

**Split:** the session lands the API's request shape and its fixture; a writing subagent then writes the
timeline and value-tree views and their browser test.

- [x] The API returns one request's events, dumps and log lines with their start and end, and a Rust test writes its fixture. `crates/nvs-cli/src/debug/api.rs:1`, `crates/nvs-cli/tests/debug_api.rs:1`, `viewer/test/fixtures/request.json:1`
- [x] The timeline puts calls, queries, spawns, garbage-collection pauses, dumps and log lines of the request on one time axis, from the trace events' start and end. `viewer/src/views/timeline.tsx:1`, `crates/nvs-runtime/src/ctx/trace.rs:102`
- [x] A value tree renders as text nodes in the tree pattern, a repeat is a link to the node it names, and a redacted or elided node says so (`rule:errors/record-transformations`). `viewer/src/views/value-tree.tsx:1`
- [x] Browser tests for the timeline and a value tree with a repeat and a redacted node. `viewer/test/request.browser.test.ts:1`

### 4. the stream, the log and full-text search with saved filters

**Does:** the second and third views, one search box over every record, and filters that are saved and
linked.

**Split:** the session lands the search API and the settings file; a writing subagent then writes the
stream, log and search views and their browser test against the fixtures.

- [x] The stream view filters by kind, level, route and trace, and the log view shows the application's log joined to requests by `request_id`. Unparseable records and quarantined files are shown, and the index can be discarded and rebuilt from a button (`rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`). `crates/nvs-cli/src/debug/api.rs:1`, `viewer/src/views/stream.tsx:1`, `viewer/src/views/log.tsx:1`
- [x] Search reads the FTS5 table `debug-service`'s index holds, with words, phrases and the field terms the rule lists, newest first and grouped by request (`rule:testing/the-viewer-searches-every-record-and-saves-a-filter`). The bundled SQLite compiles FTS5 in, so no feature is added. `crates/nvs-cli/src/debug/api.rs:1`, `viewer/src/views/search.tsx:1`, `Cargo.toml:1046`
- [x] Saved filters and the other viewer settings live in one file beside the index in the data folder, and each filter has its own link. `crates/nvs-cli/src/debug/settings.rs:1`, `crates/nvs-config/src/data.rs:119`
- [x] Rust tests: a `secret` value is not found, a phrase and each field term, a filter kept across an index rebuild. `crates/nvs-cli/tests/debug_api.rs:1`
- [x] Browser tests for both views. `viewer/test/stream.browser.test.ts:1`
- [x] `rule:testing/the-viewer-searches-every-record-and-saves-a-filter` `shipped`. `data/rules/testing/the-viewer-searches-every-record-and-saves-a-filter.json:3`

### 5. grouped exceptions, and every site is a link into the editor

**Does:** the exceptions view, and an editor link on every structured site.

- [x] The exceptions API and view: one row per class and throw site with the latest message, count, first and last time, a count chart and routes, and a row opens to its requests (`rule:testing/an-exception-is-one-row-per-class-and-site`). `crates/nvs-cli/src/debug/api.rs:1`, `viewer/src/views/exceptions.tsx:1`
- [x] Every structured site is an editor link, with the presets, the template, path mappings and VS Code as the default (`rule:testing/a-site-in-the-viewer-opens-in-the-editor`). `viewer/src/editor-link.ts:1`
- [x] Tests: a link from a logged string is not built, a path mapping, and the exceptions view in the browser. `viewer/test/editor-link.test.ts:1`, `viewer/test/exceptions.browser.test.ts:1`
- [x] `rule:testing/an-exception-is-one-row-per-class-and-site` `shipped`. `data/rules/testing/an-exception-is-one-row-per-class-and-site.json:3`

### 9. a profile is handed to VS Code, and a request's coverage is a heatmap over its source

**Does:** a profile opens in the extension's view, and the viewer draws a request's coverage over its
source.

- [x] The extension handles `vscode://novis-lang.nvs/` links and opens a profile in its *View Profile* view, and other editors get a download (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). `editors/vscode/src/extension.ts:120`, `editors/vscode/package.json:24`
- [x] The URI handler in the extension's headless suite. `editors/vscode/scripts/headless.mjs:1`
- [x] A request's coverage is drawn as a per-line heatmap over its source. The service reads only files under the project root that the record names, and says so when a file changed since the request. The lcov hand-off stays as it is. `crates/nvs-cli/src/debug/source.rs:1`, `viewer/src/views/coverage.tsx:1`, `editors/vscode/src/tests.ts:92`
- [x] Rust test: a source path outside the project root is refused. `crates/nvs-cli/tests/debug_api.rs:1`
- [x] `rule:testing/a-site-in-the-viewer-opens-in-the-editor` `shipped`, now that its profile link is built. `data/rules/testing/a-site-in-the-viewer-opens-in-the-editor.json:3`

### 6. two requests compare, and one replays after the developer confirms

**Does:** a compare view and a replay that can only reach the development server.

**Split:** the session lands the replay endpoint and its Rust tests; a writing subagent then writes the
compare view and the dialog's browser test.

- [x] Compare sets input, the response (status, headers and body, a JSON body compared structurally), duration, aligned queries, dumps and log lines of two requests side by side, and a redacted header is never a difference (`rule:testing/two-requests-compare-and-a-replay-asks-first`). `crates/nvs-cli/src/debug/api.rs:1`, `viewer/src/views/compare.tsx:1`
- [x] Replay sends to the address the development `nvs serve` gave its child and to nothing else. The dialog names method, path and target, has cancel focused, asks every time, and takes a redacted header's value once without storing it. `crates/nvs-cli/src/debug/replay.rs:1`, `viewer/src/views/replay.tsx:1`
- [x] The replayed request opens beside its original in the compare view. `viewer/src/views/compare.tsx:1`
- [x] Rust tests: a replay from a forged `Origin` is refused, a service started by hand has no target, a typed header value is found in no file. `crates/nvs-cli/tests/debug_viewer.rs:1`
- [x] The browser test for the dialog. `viewer/test/replay.browser.test.ts:1`
- [ ] `rule:testing/two-requests-compare-and-a-replay-asks-first` `shipped`. `data/rules/testing/two-requests-compare-and-a-replay-asks-first.json:3`

### 7. the viewer has its feature proofs, and its rules are shipped

**Does:** the proofs `rule:testing/feature-proofs` asks of a tool, and the last rule `shipped`.

**Split:** after the CLI section lands, a writing subagent writes the three examples and `about.md`;
then another writes the hostile program and its browser test. The session checks both and writes the
perf test and the rule flip.

- [ ] A `# The debug viewer` section in the CLI chapter, so the feature is `tools:cli/the-debug-viewer` and its help is in the binary. `docs/reference/tools/10-cli.md:655`
- [ ] `about.md` and three examples: a route that dumps a value, a handler whose queries repeat, and a handler that throws. `docs/examples/tools/cli/the-debug-viewer/about.md:1`, `docs/examples/tools/cli/the-debug-viewer/01-a-route-that-dumps-a-value.nvs:1`, `docs/examples/tools/cli/the-debug-viewer/02-queries-that-repeat.nvs:1`, `docs/examples/tools/cli/the-debug-viewer/03-a-handler-that-throws.nvs:1`
- [ ] The attack: a program whose requests dump, log and throw values with markup, script URLs, control and bidi characters and very long strings, and a browser test that no script runs and no element is made from them (`rule:testing/hostile-case-contract`). `tests/hostile/tools/cli/the-debug-viewer/values-that-look-like-markup.nvs:1`, `viewer/test/hostile.browser.test.ts:1`
- [ ] The four `[debug]` keys stage 2 added (`crates/nvs-config/src/default.toml:327`) have their proofs, which CI's `proofs` job owes now: `directive:debug.listen`, `directive:debug.tls_cert_file`, `directive:debug.tls_key_file` and `directive:debug.token_file` each get `about.md`, one example and one hostile case under `docs/examples/config/debug-<key>/` and `tests/hostile/config/debug-<key>/`, and the test case the gate counts, or a skip with its reason. `bun nv proofs --gate --limit 0` names none of them. `docs/examples/config/debug-listen/about.md:1`, `tests/hostile/config/debug-listen/`, `data/proofs/skip/`
- [ ] A Rust test that the newest page reads the same number of SQLite steps at two index sizes, and a `perf` skip that names it. `crates/nvs-cli/tests/debug_api.rs:1`, `data/proofs/skip/tools/cli/the-debug-viewer.json:1`, `data/proofs/skip/tools/cli/nvs-info.json:1`
- [ ] `bun nv proofs --id 'tools:cli/the-debug-viewer'` passes, `rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact` `shipped`, and `bun nv owners --closes debug-viewer` passes. `data/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.json:3`

### 10. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1, 2, 3, 8, 4, 5 and 9 are done. Stage 6 is all in but marking from every list and the
rule flip. This session landed `59bea9e2d`: `GET /api/request` carries `input` (query, headers, the
request and response bodies with `read`, `cut` and `withheld`, text or base64, parts joined with the
writer-and-sequence split, `crates/nvs-cli/src/debug/api.rs`'s `input`), and `GET /api/replay`
gives `{target}` or `null` (`crates/nvs-cli/src/debug/replay.rs`'s `target`). Then `86990f9e0`:
`viewer/src/views/compare.tsx` at `/compare/<trace>/<span>/<trace>/<span>`,
`viewer/src/views/replay.tsx` (Cancel focused, target named, an empty field per redacted header,
nothing stored), `m` marks in the request list, `c` and `r` on a request's page, and
`viewer/test/replay.browser.test.ts`. The Standing decision *The dialog reads its target from
`GET /api/replay`* records the calls.

Checked once: build and clippy clean; `cargo test --test debug_api` (fixture `request.json`
rewritten, then green), `debug_service`'s `a_replay_reaches_the_server_...` (now also reads the
target), `debug_viewer`'s `a_changing_request_...` (a hand-started service's target is `null`);
`bun nv viewer --check`; `NV_BROWSER=1 bun test viewer/test/replay.browser.test.ts` 6 pass, and the
subagent ran `request.browser.test.ts` and `requests.browser.test.ts` green. The sweep's reds were
stale again: `env-secret-proofs`' log exits 0, and nightly 40291801939 built `d6f131752`.

Expected red: CI `proofs` (`directive:debug.*`, the four `[debug]` keys) until stage 7. Possibly
`crates/nvs-cli/tests/live_config.rs:670`, if its server runs in `development`: it expects a
reload naming `coverage` to be refused with "cannot be used by a server", and `refuse_switch` now
refuses it with another message. Possibly `crates/nvs-cli/tests/debug_ingest.rs`, whose `counts`
lists `coverage` and expects a row from the sample spool, and `crates/nvs-cli/tests/debug_viewer.rs`'s
ingest case, which reads the same sample.

### Next

**Finish stage 6, then stage 7.** The rule says a second request is marked "in any list", and only
the request list marks. Add the mark to the exception group's request list and to the search's
request hits, with the same `m` key and the helpers `compare.tsx` exports (`toggleMark`,
`openLink`), extend `replay.browser.test.ts` for one of them, rebuild with `bun nv viewer`, then flip
the rule and set stage 6 `done`. Stage 7 follows its `**Split:**` line.

- [x] `GET /api/request` gains `input`, and the fixture test rewrites `viewer/test/fixtures/request.json` with it. `crates/nvs-cli/src/debug/api.rs:1`
- [x] `GET /api/replay` names the target, so the page disables Replay with its reason first. `crates/nvs-cli/src/debug/replay.rs:1`
- [x] Compare, the dialog and its browser test. `viewer/src/views/compare.tsx:1`, `viewer/src/views/replay.tsx:1`, `viewer/test/replay.browser.test.ts:1`
- [ ] A request is marked for compare in the exception group's list and in the search, as in the request list. `viewer/src/views/exceptions.tsx:1`, `viewer/src/views/search.tsx:1`
- [ ] `rule:testing/two-requests-compare-and-a-replay-asks-first` `shipped`, and stage 6 `done`. `data/rules/testing/two-requests-compare-and-a-replay-asks-first.json:3`

### Backlog

- The top comments of `viewer/src/app.tsx`, `views/saved.tsx`, `views/request.tsx` and `views/timeline.tsx` are over `bun nv proofs --comments`' bounds; stage 7's proofs pass rewrites them.
- A group's `count` is kept by the ingester and does not fall when the size bound deletes its requests, so it can exceed the requests `?class=&file=&line=` lists (`crates/nvs-cli/src/debug/index.rs:440`).
- The list's `dumps` and `logs` counts are per trace id (`crates/nvs-cli/src/debug/index.rs:241`), so two requests that share a trace id each count both's records; `viewer/test/fixtures/requests.json` shows it. Count them with the writer-and-sequence split `GET /api/request` uses.
- A dump's properties reach the index in key order, because the spool's `Dump::tree` is a `serde_json::Value` (`crates/nvs-runtime/src/spool.rs:593`). A `RawValue` there keeps the declared order; the API already passes the stored text through.

- The button that makes a recorded request a `Core\Test::request` case is `request-aware-editor`'s, beside its generator (`docs/goals/request-aware-editor.md`).
- The development error page links to the request's timeline when `nvs serve` started the service (`docs/rules/errors/renderings.md`).
- Compare aligns queries by call site once each query carries its site id (`docs/goals/request-aware-editor.md`).
- Viewer screenshots for the website come from `tools-website` (`docs/goals/tools-website.md`).
- `bun nv ci-changes --files -` is refused by its own argument parser; `--files=-` works (`tools/nv/cmd/ci-changes.ts:6`).
- The request list does not read a query, so a saved filter of the `requests` view opens the whole list (`viewer/src/views/requests.tsx:1`, `viewer/src/views/saved.tsx:1`).
- The search's field-only `kind:` and `level:` use the writer-and-sequence owner rule, but the list's counts do not (the first backlog item); fixing the counts lets both read the same columns.
