---
looper: 1
title: "Requests are recorded to a spool and indexed by `nvs debug`"
summary: "[trace] response_header turns the traceresponse header off, [debug] store writes each request's outcome, input, response, events, dumps and logs to a per-writer spool, nvs debug ingests sealed spool files into a SQLite index that holds everything debug-viewer reads, a development nvs serve starts it, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-config/src/default.toml, crates/nvs-config/src/tree.rs, crates/nvs-server/src/serve.rs, crates/nvs-cli/src/main.rs]
stages:
  - id: 1
    title: "`[trace] response_header` turns the `traceresponse` header off"
    status: open
    context: [crates/nvs-config/src/default.toml, crates/nvs-config/src/tree.rs, crates/nvs-config/src/defaults.rs, crates/nvs-server/src/trace.rs, crates/nvs-server/src/serve.rs, docs/rules/observability/a-trace-id-exists-for-every-request.md, docs/rules/observability/the-traceresponse-header-can-be-turned-off.md]
    extra:
      context:
        rules: [observability/the-traceresponse-header-can-be-turned-off, observability/a-trace-id-exists-for-every-request, observability/metrics-and-trace-blocks-are-system]
  - id: 2
    title: "`[debug] store` opens a spool file per writer, sealed and capped"
    status: open
    context: [crates/nvs-config/src/default.toml, crates/nvs-config/src/tree.rs, crates/nvs-config/src/directive.rs, crates/nvs-config/src/data.rs, crates/nvs-runtime/src/logfile.rs, crates/nvs-runtime/src/ctx/trace.rs, docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md]
    extra:
      context:
        rules: [testing/writing-a-debug-record-to-disk-is-its-own-gate, testing/the-debug-stream-reaches-the-service-through-the-disk, config/ownership-is-the-trust-boundary, config/three-changeability-classes]
  - id: 3
    title: a request's outcome, input, response, events, dumps and logs reach the spool
    status: open
    context: [crates/nvs-server/src/serve.rs, crates/nvs-server/src/body.rs, crates/nvs-runtime/src/ctx/inbound.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-runtime/src/ctx/output.rs, crates/nvs-stdlib/src/debug.rs, crates/nvs-db/src/span.rs, docs/rules/testing/a-recorded-request-keeps-its-input-with-credentials-replaced.md]
    extra:
      context:
        rules: [testing/a-recorded-request-keeps-its-input-with-credentials-replaced, testing/the-debug-stream-reaches-the-service-through-the-disk, observability/a-query-is-a-trace-event, errors/record-transformations, security/redaction-reaches-the-tools-own-renderings, security/secret-qualifier]
  - id: 4
    title: "`nvs debug` ingests sealed spool files into the index"
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/Cargo.toml, crates/nvs-config/src/data.rs, docs/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.md, docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md]
    extra:
      context:
        rules: [testing/the-ingester-runs-whether-or-not-anyone-is-looking, testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time, testing/the-viewer-shows-the-stream-and-hands-off-every-artifact]
  - id: 5
    title: a bad line and a bad file never stop the ingester, and a log file is tailed
    status: open
    context: [crates/nvs-cli/src/main.rs, docs/rules/testing/a-bad-line-degrades-and-a-bad-file-is-quarantined.md, docs/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.md]
    extra:
      context:
        rules: [testing/a-bad-line-degrades-and-a-bad-file-is-quarantined, testing/the-ingester-runs-whether-or-not-anyone-is-looking, errors/log-write]
  - id: 6
    title: a development `nvs serve` starts `nvs debug`, and a fixture's requests reach the index
    status: open
    context: [crates/nvs-cli/src/serve.rs, crates/nvs-cli/src/main.rs, crates/nvs-config/src/data.rs, docs/rules/testing/nvs-debug-is-the-service-and-a-development-server-starts-it.md, docs/reference/tools/10-cli.md]
    extra:
      context:
        rules: [testing/nvs-debug-is-the-service-and-a-development-server-starts-it, testing/writing-a-debug-record-to-disk-is-its-own-gate, testing/debug-probes]
checks:
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, debug-service]
    reach: true
    floor: false
---

# Requests are recorded to a spool and indexed by `nvs debug`

## Why

Nothing records what a request did. `Core\Debug::dump` inside a request becomes one log line
(`crates/nvs-stdlib/src/debug.rs:311`), a request's query, `http` and `spawn` events are kept only when
the request is sampled and leave the process only as OTLP spans (`crates/nvs-server/src/serve.rs:1346`),
and a request's input is gone when it ends. `[debug]` has no `store` key
(`crates/nvs-config/src/default.toml:326`), and five designed rules describe a spool, an ingester and an
index that do not exist. This goal builds them, so that `debug-viewer` has data to show and
`request-aware-editor` has recorded requests to turn into tests. It first settles the `traceresponse`
header, because the viewer moves from a response to its timeline by that header and its rule and the
viewer's rule disagree about production.

## Standing decisions

**`traceresponse` is sent by default, and `nvs.toml` can turn it off.** The user decided this. The key
is `[trace] response_header = true`, beside `[trace] propagate`; the key's name is the planning
session's call. `false` sends no `traceresponse`, and the trace id still exists for the request, its
logs and its spans. `rule:observability/the-traceresponse-header-can-be-turned-off` holds it, and
`rule:observability/a-trace-id-exists-for-every-request` is amended when stage 1's code lands. Trade-offs:
one key read per response off the snapshot; 55 bytes fewer per response when off.

**The debug service is `nvs debug`, started automatically.** The user decided this. It runs the ingester
and the viewer on loopback. A development `nvs serve` with `[debug] store` on starts it as a child and
stops it on exit, and it also runs by hand. The spool and the index live in the data folder.
`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it` holds it. This goal builds
the ingester half; the listener, its port and the browser UI are `debug-viewer`'s.

**The spool records request input and the response, bounded and redacted.** The user decided this.
With `[debug] store` on, each development request's method, path, query, headers and body, up to
`[limits] request_body`, and its response's status, headers and body, go to the spool, with
`Authorization`, `Cookie`, `Set-Cookie` and `Proxy-Authorization` values replaced. The response body
is capped at the same `[limits] request_body`, because `max_output` has no default cap: a longer body
is cut at the cap with a marker, it is written through to the spool file and never held whole in
memory, and no new limit is added. The planning session made that call. A test made from a recorded
request asserts the status, key headers and the body, only the status and headers when the body was
cut, and the viewer's compare view diffs two responses. `rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`
holds it.

**A `secret` field of a decoded body is replaced.** The user decided this. When the handler decodes the
body through `Core\Request::jsonAs<X>()` or `postAs<X>()`, every field that lands in a `secret` property
of `X` is replaced with `[redacted]` in the stored body, on top of the four headers. A body not read
through those calls is stored as read, in the owner-only directory, and the `nvs debug` reference page
says so.

**With `store` on in production, request input is never recorded.** The user decided this. A production
request records its outcome, its trace events and its dumps, and no header and no body of the request or
the response.

**A query's call site id is stored with its span.** The user decided that the compiler passes a
constant site id to each `Core\Db` query call (`request-aware-editor` builds it). This goal stores it on
the spool's `query` event and in the index's `events` table when the event carries one.

**The spool's numbers.** The planning session's call, written into
`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`: a file is sealed at 4 MiB or
250 ms after its first record, a line is at most 4 MiB, and `[debug] spool_max` (256 MiB, `System`)
bounds one process's spool, split evenly between its writers, each evicting its own oldest file. 250 ms
is the delay between a dump and the viewer showing it. An idle writer opens no file and arms no timer.

**The index's schema and deletion window.** The planning session's call, written into
`rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`: one SQLite file with a
table per thing the viewer lists, one FTS5 table for search, exception groups keyed by class and throw
site, and the request input for compare and replay. `[debug] index_age` (`7d`) and `[debug] index_max`
(1 GiB) bound it. Recorded bodies are request data, so the index forgets them on its own. Saved filters
are the viewer's and are not in the index.

**A recorded body is the bytes the program read.** The planning session's call, written into
`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`. The server reads a body only
as the program asks for it (`crates/nvs-server/src/body.rs:261`), and recording keeps that. A body the
program never read is recorded as not read, so a replay of that request sends no body.

**Log records of a request also go to the spool.** The planning session's call. A development server
logs to its terminal, so the viewer would otherwise have no log to show. Tailing a log file is for a
file named with `nvs debug --tail`, so no record is ingested twice.

**The writer lives in `nvs-runtime` and the ingester in `nvs-cli`.** The planning session's call. The
writer must be reachable from a dump and a log record, both written through the runtime, and it is one
per thread, so it needs no lock. The ingester takes `rusqlite`, already in the tree for `Core\Db`, so the
binary gains no new C dependency.

**Trade-offs, all together.** Request path: with `store` off, one predicted branch, as today. With it on,
a buffered append per record, the bytes of the body the program read and of the response it wrote,
held until the request ends, and a replaced value per `secret` field the decoder fills. Memory: one write
buffer per writer while it has an open file, and the recorded request and response bodies per request in
flight. Disk: at most `spool_max` per process and `index_max` for the index. Usability: every
development request is recorded and searchable with no setup beyond `store = true`. Simplicity: four
new `[debug]` keys and one new `[trace]` key, a command, and no socket anywhere.

## Stages

### 1. `[trace] response_header` turns the `traceresponse` header off

**Does:** a `System` key in `[trace]`, read per response, decides whether the door writes the header.

- [ ] The key in the default file beside `propagate`, in `Trace`, in the defaults table, and in the directives test's key list (`rule:observability/the-traceresponse-header-can-be-turned-off`). `crates/nvs-config/src/default.toml:948-957`, `crates/nvs-config/src/tree.rs:1240-1250`, `crates/nvs-config/src/defaults.rs:241`, `crates/nvs-config/src/defaults.rs:483`, `crates/nvs-config/tests/directives.rs:3550`
- [ ] `respond` reads the key off the request's snapshot, the way `access_on` reads `[log] access`, and with `false` removes a header the program set and writes none. `crates/nvs-server/src/trace.rs:286-306`, `crates/nvs-server/src/trace.rs:316`, `crates/nvs-server/src/serve.rs:1652`
- [ ] Served tests for both values, a door-written answer included, and that the access line and `Core\Server::traceId()` still carry the id with the header off. `crates/nvs-server/src/serve.rs:11878`
- [ ] `rule:observability/a-trace-id-exists-for-every-request` amended, now that its sentence is true: every response carries the header *by default*, `[trace] response_header = false` removes the header and nothing else, the title says "emitted on the response unless the configuration turns the header off", and `seeAlso` names the new rule. The new rule `shipped`, and the example's page says the header can be turned off. `docs/rules/observability/a-trace-id-exists-for-every-request.md:9`, `docs/examples/core/Server/traceId/about.md:5`

### 2. `[debug] store` opens a spool file per writer, sealed and capped

**Does:** with the gate on, each writer appends to its own file in the data folder, seals it by rename,
and keeps its share of the cap.

- [ ] `store`, `spool_max`, `index_age` and `index_max` in the default file, in `Debug`, and in the directive table: `store` `RuntimeTighten`, the three bounds `System` (`rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`). `crates/nvs-config/src/default.toml:326-346`, `crates/nvs-config/src/tree.rs:513`, `crates/nvs-config/src/directive.rs:227`, `crates/nvs-config/src/directive.rs:235`, `crates/nvs-config/src/defaults.rs:400`
- [ ] `debug/` and `debug/spool/` in the data folder, created private and checked for ownership. `crates/nvs-config/src/data.rs:163`, `crates/nvs-config/src/data.rs:446`, `crates/nvs-config/src/data.rs:506`
- [ ] A writer per thread in a new `nvs_runtime::spool`, shaped like the log file target: an open file only once it has a record, sealed by rename at 4 MiB or 250 ms, each line under 4 MiB, its share of `spool_max` kept by deleting its own oldest file and writing a `dropped` line (`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`). `crates/nvs-runtime/src/logfile.rs:1-12`
- [ ] With the gate on, a request records its span events whatever the sampling decision. `crates/nvs-runtime/src/ctx/trace.rs:240`
- [ ] Tests: two writers never share a file, a sealed file is never written again, the cap evicts oldest first and counts what it dropped, a spool directory owned by another account is refused, and with the gate off no file is created.

### 3. a request's outcome, input, response, events, dumps and logs reach the spool

**Does:** every record the rules name is written as one line with its trace id, and the four credential
headers never reach the disk.

- [ ] A `request` line at the request's end, from both completion paths: method, route, path, status, duration, counts, and the uncaught exception's class, message and throw site. `crates/nvs-server/src/serve.rs:1346`, `crates/nvs-server/src/serve.rs:2113`
- [ ] In `development`, the input: the head from `Inbound` with the four headers replaced by `[redacted]`, and the body chunks the program pulled, up to `request_body` (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`). `crates/nvs-runtime/src/ctx/inbound.rs:157`, `crates/nvs-server/src/body.rs:261`
- [ ] In `development`, the response: its status, its headers with the four replaced, and the bytes written, written through to the spool file up to `request_body` and cut there with a marker.
- [ ] A body decoded by `jsonAs<X>()` or `postAs<X>()` is stored with each field that lands in a `secret` property of `X` replaced by `[redacted]`. `crates/nvs-stdlib/src/request.rs:3098`, `crates/nvs-stdlib/src/request.rs:2583`
- [ ] `event` lines for the span events, a `query` event with its driver, SQL, rows and its call site id when it has one, and never a parameter (`rule:observability/a-query-is-a-trace-event`). `crates/nvs-db/src/span.rs:91-112`
- [ ] `dump` lines with the bounded tree (`rule:errors/record-transformations`) and `log` lines for the request's log records, a `secret` redacted in both. `crates/nvs-stdlib/src/debug.rs:311`, `crates/nvs-runtime/src/ctx/output.rs` (`write_log_record`)
- [ ] Tests: a request with each header shows `[redacted]` and no byte of the value in the file, a `secret` field decoded by `jsonAs<X>()` leaves no byte of its value in the file, an unread body is recorded as not read, a request body and a response body past `request_body` are each cut and say so, and a production request records its outcome, events and dumps and no header or body.

### 4. `nvs debug` ingests sealed spool files into the index

**Does:** the command creates the index, reads each sealed file, inserts its records, commits, and
deletes the file.

- [ ] `nvs debug` beside `nvs lsp`, holding `debug/service.lock` in the data folder, and `rusqlite` added to `nvs-cli` from the workspace (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`). `crates/nvs-cli/src/main.rs:853`, `crates/nvs-cli/Cargo.toml:81`, `Cargo.toml:1039`
- [ ] The schema the rule's table names, in write-ahead mode with synchronous writes off, and a schema version that a newer binary treats as a cache miss (`rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`).
- [ ] Read, insert under the dedupe key, commit, then delete, with an exception group's count and first and last seen updated in the same batch, and the FTS5 rows beside their records (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).
- [ ] The watermark in `meta` after each batch, the directory listed every 250 ms while files arrive and every two seconds after a quiet minute, and an unsealed file untouched for a minute read as sealed.
- [ ] Deletion past `index_age` or `index_max` in batches. Tests: a crash between commit and delete re-ingests nothing twice, a search finds a dump's text, ten throws from one site make one group of ten, and the deletion window removes every row of a request.

### 5. a bad line and a bad file never stop the ingester, and a log file is tailed

**Does:** the ingester keeps what it cannot parse, quarantines what it cannot read, and reads a named
log file from a checkpoint.

- [ ] A line past 4 MiB is truncated and skipped to the next newline, a line that does not parse is an `unparseable` record with its file, offset and first 4 KiB, and invalid UTF-8 is converted with a flag (`rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`).
- [ ] A file that fails to open or read is quarantined with its error and a retry after one second, doubling up to five minutes, and a full disk or an index error backs off the same way and deletes no file.
- [ ] Work per source per pass is at most 16 MiB, and sources are taken in turn.
- [ ] `nvs debug --tail <file>` reads a log file from a checkpoint of its identity, offset and first-line hash, and never writes it (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).
- [ ] Tests: a file that is one enormous line, a file with a bad line between two good ones, an unreadable file listed in `quarantine` while the spool keeps flowing, and a tailed file rotated and truncated in place.

### 6. a development `nvs serve` starts `nvs debug`, and a fixture's requests reach the index

**Does:** the server starts the service as a child when it should, and the milestone's acceptance for the
debug service holds.

- [ ] A development `nvs serve` with `store` on starts `nvs debug` unless the lock is held, stops its own child on exit, and the child exits within one pass of its parent being gone (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`). `crates/nvs-cli/src/serve.rs:161`
- [ ] A production `nvs serve` never starts it.
- [ ] A test: a fixture served with `store` on has its requests in the index, joined by trace id, from sealed spool files only. With `store` off, the same run writes no spool file and its probe counts match a run with no `[debug]` block (`rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`).
- [ ] The help text for `nvs debug` and `--tail`, the four keys' comments in the default file, and a `# nvs debug` section in `docs/reference/tools/10-cli.md` for the ingester half, after `# nvs test`, which says what a recorded body keeps: a body read other than through `jsonAs<X>()` or `postAs<X>()` is stored as read. `docs/reference/tools/10-cli.md:192`
- [ ] Every rule this goal owns `shipped`, and `bun nv owners --closes debug-service` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written. `crates/nvs-cli/src/main.rs`, `crates/nvs-runtime/src/ctx/mod.rs` and
`crates/nvs-runtime/src/ctx/output.rs` were being edited by another session at the time, so find their
anchors by name (`Lsp {`, `write_log_record`) if a line has moved. Stage 1's amendment of
`rule:observability/a-trace-id-exists-for-every-request` waits for its code, because the rule is
`shipped`.

### Next

Stage 1, `[trace] response_header`, on `crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/defaults.rs`
and `crates/nvs-server/src/trace.rs`:
- the key in the default file and in `Trace` (`rule:observability/the-traceresponse-header-can-be-turned-off`). `crates/nvs-config/src/default.toml:954`, `crates/nvs-config/src/tree.rs:1249`
- `respond` reads it per response. `crates/nvs-server/src/trace.rs:301`
- served tests for both values. `crates/nvs-server/src/serve.rs:11878`

### Backlog

- The listener, its port, the token and the browser UI are `debug-viewer`'s (`docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md`).
- The feature proofs of `nvs debug` land with the viewer, the part a developer sees (`docs/rules/testing/feature-proofs.md`).
- The query call site id is passed by `request-aware-editor`; until then a `query` event has none (`docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md`).
- A recorded request becoming a `Core\Test::request` case is `request-aware-editor`'s (`docs/rules/testing/a-recorded-request-keeps-its-input-with-credentials-replaced.md`).
