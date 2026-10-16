---
looper: 1
title: "The debug viewer shows every request, its timeline, its logs and its exceptions, and is the tool a developer keeps open"
summary: "nvs debug serves a TypeScript and Solid viewer embedded in the binary, on a listener only its own page can use; the request list follows live, a request opens to its timeline, the stream and the log are searchable with saved filters, exceptions are grouped, every file:line opens in the editor, two requests compare and one replays after asking, the viewer has its feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md, docs/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.md, crates/nvs-config/src/default.toml]
stages:
  - id: 1
    title: "the viewer's front end is built from `viewer/` and embedded in `nvs`"
    status: open
    context: [package.json, tsconfig.json, CONTRIBUTING.md, tools/nv/cmd/verify.ts, tools/nv/cmd/ci-changes.ts, tools/nv/cmd/gen-attribution.ts, crates/nvs-cli/src/info.rs, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md, docs/rules/packaging/the-third-party-notice-is-generated-never-written-by-hand.md]
    extra:
      context:
        rules: [testing/the-viewer-shows-the-stream-and-hands-off-every-artifact, packaging/the-third-party-notice-is-generated-never-written-by-hand, packaging/the-notice-is-embedded-in-the-binary, testing/attribution-is-diffed-in-ci]
  - id: 2
    title: "`nvs debug` serves the viewer, and only to its own page"
    status: open
    context: [crates/nvs-config/src/default.toml, crates/nvs-server/src/trace.rs, tools/nv/test/chromium.ts, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md, docs/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.md]
    extra:
      context:
        rules: [testing/the-viewer-shows-the-stream-and-hands-off-every-artifact, testing/the-ingester-runs-whether-or-not-anyone-is-looking, concurrency/one-scheduler, config/ownership-is-the-trust-boundary]
  - id: 3
    title: the request list follows live, and a request opens to its timeline
    status: open
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-server/src/trace.rs, tools/nv/test/chromium.ts, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md, docs/rules/errors/record-transformations.md]
    extra:
      context:
        rules: [testing/the-viewer-shows-the-stream-and-hands-off-every-artifact, observability/a-trace-id-exists-for-every-request, errors/record-transformations, observability/trace-events-carry-a-kind]
  - id: 4
    title: the stream, the log and full-text search with saved filters
    status: open
    context: [crates/nvs-config/src/data.rs, docs/rules/testing/the-viewer-searches-every-record-and-saves-a-filter.md, docs/rules/testing/a-bad-line-degrades-and-a-bad-file-is-quarantined.md]
    extra:
      context:
        rules: [testing/the-viewer-searches-every-record-and-saves-a-filter, testing/a-bad-line-degrades-and-a-bad-file-is-quarantined, testing/the-ingester-runs-whether-or-not-anyone-is-looking]
  - id: 5
    title: grouped exceptions, links into the editor, and the hand-offs
    status: open
    context: [editors/vscode/src/extension.ts, editors/vscode/package.json, editors/vscode/src/tests.ts, docs/rules/testing/an-exception-is-one-row-per-class-and-site.md, docs/rules/testing/a-site-in-the-viewer-opens-in-the-editor.md]
    extra:
      context:
        rules: [testing/an-exception-is-one-row-per-class-and-site, testing/a-site-in-the-viewer-opens-in-the-editor, ide/the-extension-builds-no-ui-the-editor-already-has, observability/speedscope-timeline-export]
  - id: 6
    title: two requests compare, and one replays after the developer confirms
    status: open
    context: [docs/rules/testing/two-requests-compare-and-a-replay-asks-first.md, docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md]
    extra:
      context:
        rules: [testing/two-requests-compare-and-a-replay-asks-first, testing/the-debug-stream-reaches-the-service-through-the-disk, testing/writing-a-debug-record-to-disk-is-its-own-gate]
  - id: 7
    title: the viewer has its feature proofs, and its rules are shipped
    status: open
    context: [docs/reference/tools/10-cli.md, data/proofs/policy.json, docs/rules/testing/feature-proofs.md]
    extra:
      context:
        rules: [testing/feature-proofs, testing/hostile-case-contract, testing/examples-live-in-the-repository]
checks:
  - id: owners-closes
    stage: 7
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, debug-viewer]
    reach: true
    floor: false
extra:
  milestone: M10
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
`trace-timeline` (a start and an end on every trace event) reached first. `request-aware-editor` comes
after it and adds the button that makes a recorded request a test.

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

**The listener and its API are the planning session's calls.** `hyper` over the host's sockets and
`rustls`, both already in the graph (`Cargo.toml:193`, `Cargo.toml:316`), so `nvs debug` adds no HTTP
or TLS crate. A fixed default port, so a link outlives a restart. The watermark is pushed as
server-sent events. Every list is a page keyed on the watermark sequence. The Rust side owns the API's
shapes, a Rust test writes a JSON fixture from a real index, and `tsc` and the browser tests read that
fixture. A request is keyed by its trace id and its span id, because an inbound `traceparent` can give
two requests one trace id.

**The browser tests use the repository's Chromium driver.** The planning session's call, corrected
against the tree: Playwright's own driver does not launch under Bun on Windows, so
`tools/nv/test/chromium.ts` drives the headless shell `playwright-core` downloads over the DevTools
protocol (`tools/nv/test/chromium.ts:5-11`), and CI's `browser` job runs it with `NV_BROWSER` set.

**The details inside the four features are the planning session's calls**, written in their rules: an
exception's site is the file and line of its `throw`; a link is built only from a structured site,
never from text in a message; the editor setting and the saved filters live in one settings file beside
the index in the data folder; a replay goes only to the development server that started the service,
asks every time with cancel focused, and sends a redacted header only if the developer types its value.

**The viewer's perf proof is a count, not a `.nvs` bench.** The planning session's call. The viewer
runs no Novis code, so a bench under `benches/members/` would measure nothing of it. A Rust test counts
the SQLite steps of the newest page at two index sizes and fails if they grow, and the proofs policy
names that test in a `perf` skip.

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

- [ ] `viewer/` as a workspace of the root package: `solid-js` its one runtime dependency, Babel and Solid's JSX preset at build time, the root `bun.lock` updated, its own `tsconfig.json` with the DOM library, `"workspaces": ["viewer"]` in the root `package.json`, and its row after `editors/` in `CONTRIBUTING.md`'s layout block, in the commit that creates the folder because `bun nv layout --check` fails on a row with no folder (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`). `package.json:14`, `CONTRIBUTING.md:183`
- [ ] `bun nv viewer` builds `viewer/dist/viewer.js` and `viewer/dist/viewer.css`, `--check` rebuilds and fails on a byte difference, and `--dev` serves the source with reload against a running `nvs debug`. A test that two builds give the same bytes.
- [ ] `nv verify` gains a `viewer` step, run when a file under `viewer/` changed: `tsc`, the bundle check and the viewer's unit tests. CI's `browser` job gains the viewer's browser suite. `tools/nv/cmd/verify.ts:162`, `tools/nv/cmd/verify.ts:910`, `tools/nv/cmd/ci-changes.ts:107`
- [ ] `bun nv gen-attribution` reads the viewer's runtime dependencies from `viewer/package.json`'s `dependencies` at the versions `bun.lock` resolves, each licence file from its installed package, and no build-time package. In the same commit the packaging rule's paragraph on what the notice is generated from says so, and its `guardedBy` gains `viewer/package.json` (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`). `tools/nv/cmd/gen-attribution.ts:288`, `docs/rules/packaging/the-third-party-notice-is-generated-never-written-by-hand.md:6-9`
- [ ] The crate that serves `nvs debug` embeds the two files with `include_bytes!`, as `nvs info` embeds its notice. `crates/nvs-cli/src/info.rs:34`

### 2. `nvs debug` serves the viewer, and only to its own page

**Does:** a listener on loopback that serves the shell, the two files and the JSON API, refuses every
other host and origin, and pushes the watermark.

- [ ] The rule names the fixed default port. `[debug] listen` and `--listen` change the address, and the shell is served for every view path (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`). `crates/nvs-config/src/default.toml:326`, `Cargo.toml:193`
- [ ] `Host` and `Origin` are allowlisted, a changing request needs an allowed `Origin` and a JSON body, and every response carries the rule's `Content-Security-Policy`, `nosniff` and `no-referrer`.
- [ ] Off loopback: a self-signed certificate made with `rcgen` and a random token, generated by `nvs debug` and kept private in the data folder's `debug/`, a printed URL carrying the token, and the token taken once from that URL into an `HttpOnly`, `Secure`, `SameSite=Strict` cookie. `[debug] tls_cert_file`, `tls_key_file` and `token_file` override the generated ones when set, and boot does not fail when they are absent. `Cargo.toml:316`, `Cargo.toml:333`
- [ ] The watermark is pushed as server-sent events, one sequence per committed batch, and a slow tab makes nothing buffer (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).
- [ ] The attack, as Rust tests: a rebinding `Host`, a forged `Origin` on a changing request, a non-loopback bind that serves plain HTTP or no token, and a request without the cookie off loopback.

### 3. the request list follows live, and a request opens to its timeline

**Does:** the main view and the request view, with the baseline every later view reuses.

- [ ] The API pages requests by the watermark sequence with method, route, status, duration, query count, dump count and whether it threw. A Rust test writes the JSON fixture from a real index, and `viewer/` types are checked against it.
- [ ] The list keeps only visible rows in the DOM, follows live, pauses at a key or a scroll and counts what arrived, and moves by keyboard. Dark and light themes, and `?` lists the keys.
- [ ] A request opens at `/requests/<trace id>/<span id>`, and a pasted `traceresponse` value or trace id opens its request (`rule:observability/a-trace-id-exists-for-every-request`). `crates/nvs-server/src/trace.rs:286`, `crates/nvs-server/src/trace.rs:301`
- [ ] The timeline puts calls, queries, spawns, garbage-collection pauses, dumps and log lines of the request on one time axis, from the trace events' start and end. `crates/nvs-runtime/src/ctx/trace.rs:69`, `crates/nvs-runtime/src/ctx/trace.rs:102`
- [ ] A value tree renders as text nodes in the tree pattern, a repeat is a link to the node it names, and a redacted or elided node says so (`rule:errors/record-transformations`).
- [ ] Browser tests over the fixture with the Chromium driver, one of them counting the rows in the DOM with many requests in the index. `tools/nv/test/chromium.ts:31`

### 4. the stream, the log and full-text search with saved filters

**Does:** the second and third views, one search box over every record, and filters that are saved and
linked.

- [ ] The stream view filters by kind, level, route and trace, and the log view shows the application's log joined to requests by `request_id`. Unparseable records and quarantined files are shown, and the index can be discarded and rebuilt from a button (`rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`).
- [ ] Search reads the FTS5 table `debug-service`'s index holds, with words, phrases and the field terms the rule lists, newest first and grouped by request (`rule:testing/the-viewer-searches-every-record-and-saves-a-filter`). The bundled SQLite compiles FTS5 in, so no feature is added. `Cargo.toml:1039`
- [ ] Saved filters and the other viewer settings live in one file beside the index in the data folder, and each filter has its own link. `crates/nvs-config/src/data.rs:119`
- [ ] Tests: a `secret` value is not found, a phrase and each field term, a filter kept across an index rebuild, and the browser suite for both views.

### 5. grouped exceptions, links into the editor, and the hand-offs

**Does:** the exceptions view, editor links on every site, a profile handed to VS Code, and a coverage
heatmap over source.

- [ ] The exceptions view: one row per class and throw site with the latest message, count, first and last time, a count chart and routes, and a row opens to its requests (`rule:testing/an-exception-is-one-row-per-class-and-site`).
- [ ] Every structured site is an editor link, with the presets, the template, path mappings and VS Code as the default (`rule:testing/a-site-in-the-viewer-opens-in-the-editor`).
- [ ] The extension handles `vscode://novis-lang.nvs/` links and opens a profile in its *View Profile* view, and other editors get a download (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). `editors/vscode/src/extension.ts:129`, `editors/vscode/package.json:6`
- [ ] A request's coverage is drawn as a per-line heatmap over its source. The service reads only files under the project root that the record names, and says so when a file changed since the request. The lcov hand-off stays as it is. `editors/vscode/src/tests.ts:237`
- [ ] Tests: a link from a logged string is not built, a path mapping, the URI handler in the extension's headless suite, and a source path outside the project root refused.

### 6. two requests compare, and one replays after the developer confirms

**Does:** a compare view and a replay that can only reach the development server.

- [ ] Compare sets input, the response (status, headers and body, a JSON body compared structurally), duration, aligned queries, dumps and log lines of two requests side by side, and a redacted header is never a difference (`rule:testing/two-requests-compare-and-a-replay-asks-first`).
- [ ] Replay sends to the address the development `nvs serve` gave its child and to nothing else. The dialog names method, path and target, has cancel focused, asks every time, and takes a redacted header's value once without storing it.
- [ ] The replayed request opens beside its original in the compare view.
- [ ] Tests: a replay from a forged `Origin` is refused, a service started by hand has no target, a typed header value is found in no file, and the browser suite for the dialog.

### 7. the viewer has its feature proofs, and its rules are shipped

**Does:** the proofs `rule:testing/feature-proofs` asks of a tool, and the five rules `shipped`.

- [ ] A `# The debug viewer` section in the CLI chapter, so the feature is `tools:cli/the-debug-viewer` and its help is in the binary. `docs/reference/tools/10-cli.md:655`
- [ ] `about.md` and three examples under `docs/examples/tools/cli/the-debug-viewer/`: a route that dumps a value, a handler whose queries repeat, and a handler that throws.
- [ ] The attack under `tests/hostile/tools/cli/the-debug-viewer/`: a program whose requests dump, log and throw values with markup, script URLs, control and bidi characters and very long strings, and a browser test that no script runs and no element is made from them.
- [ ] A Rust test that the newest page reads the same number of SQLite steps at two index sizes, and a `perf` skip in the proofs policy that names it. `data/proofs/policy.json:117`
- [ ] `bun nv proofs --id 'tools:cli/the-debug-viewer'` passes, the five rules this goal owns are `shipped`, and `bun nv owners --closes debug-viewer` passes.

## Handoff

### State

The goal is new. No stage is started. It waits for `debug-service` and `trace-timeline`. The anchors
above were checked against the tree when it was written.

### Next

Stage 1, the front end's build, on `package.json`, `tools/nv/cmd/verify.ts` and
`tools/nv/cmd/gen-attribution.ts`:
- create `viewer/` as a workspace with `solid-js` and the Babel preset, and its layout row (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`). `package.json:14`, `CONTRIBUTING.md:183`
- `bun nv viewer` with `--check` and `--dev`, and the `viewer` step in `nv verify`. `tools/nv/cmd/verify.ts:162`
- Solid's licence in the notice, with the packaging rule's paragraph (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`). `tools/nv/cmd/gen-attribution.ts:288`

### Backlog

- The button that makes a recorded request a `Core\Test::request` case is `request-aware-editor`'s, beside its generator (`docs/goals/request-aware-editor.md`).
- The development error page links to the request's timeline when `nvs serve` started the service (`docs/rules/errors/renderings.md`).
- Compare aligns queries by call site once each query carries its site id (`docs/goals/request-aware-editor.md`).
- Viewer screenshots for the website come from `tools-website` (`docs/goals/tools-website.md`).
