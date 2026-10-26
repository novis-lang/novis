---
looper: 1
title: "Requests are recorded to a spool and indexed by `nvs debug`"
summary: "[trace] response_header turns the traceresponse header off, [debug] store writes each request's outcome, input, response, events, dumps and logs to a per-writer spool, nvs debug ingests sealed spool files into a SQLite index that holds everything debug-viewer reads, a development nvs serve starts it, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-config/src/default.toml, crates/nvs-config/src/tree.rs, crates/nvs-server/src/serve.rs, crates/nvs-cli/src/main.rs]
stages:
  - id: 1
    title: "`[trace] response_header` turns the `traceresponse` header off"
    status: done
    context: [crates/nvs-config/src/default.toml, crates/nvs-config/src/tree.rs, crates/nvs-config/src/defaults.rs, crates/nvs-server/src/trace.rs, crates/nvs-server/src/serve.rs, docs/rules/observability/a-trace-id-exists-for-every-request.md, docs/rules/observability/the-traceresponse-header-can-be-turned-off.md]
    extra:
      context:
        rules: [observability/the-traceresponse-header-can-be-turned-off, observability/a-trace-id-exists-for-every-request, observability/metrics-and-trace-blocks-are-system]
  - id: 2
    title: "`[debug] store` opens a spool file per writer, sealed and capped"
    status: done
    context: [crates/nvs-config/src/default.toml, crates/nvs-config/src/tree.rs, crates/nvs-config/src/directive.rs, crates/nvs-config/src/data.rs, crates/nvs-runtime/src/logfile.rs, crates/nvs-runtime/src/ctx/trace.rs, docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md]
    extra:
      context:
        rules: [testing/writing-a-debug-record-to-disk-is-its-own-gate, testing/the-debug-stream-reaches-the-service-through-the-disk, config/ownership-is-the-trust-boundary, config/three-changeability-classes]
  - id: 3
    title: a request's outcome, input and response reach the spool
    status: done
    context: [crates/nvs-server/src/serve.rs, crates/nvs-server/src/trace.rs, crates/nvs-server/src/body.rs, crates/nvs-runtime/src/ctx/inbound.rs, crates/nvs-runtime/src/spool.rs, docs/rules/testing/a-recorded-request-keeps-its-input-with-credentials-replaced.md]
    extra:
      context:
        rules: [testing/a-recorded-request-keeps-its-input-with-credentials-replaced, testing/the-debug-stream-reaches-the-service-through-the-disk, security/redaction-reaches-the-tools-own-renderings]
  - id: 7
    title: a decoded `secret` field is replaced, and a request's events, dumps and logs reach the spool
    status: done
    context: [crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/debug.rs, crates/nvs-runtime/src/ctx/output.rs, crates/nvs-db/src/span.rs, crates/nvs-runtime/src/spool.rs, docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md]
    extra:
      context:
        rules: [testing/a-recorded-request-keeps-its-input-with-credentials-replaced, testing/the-debug-stream-reaches-the-service-through-the-disk, observability/a-query-is-a-trace-event, errors/record-transformations, security/secret-qualifier]
  - id: 4
    title: "`nvs debug` ingests sealed spool files into the index"
    status: done
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/Cargo.toml, crates/nvs-cli/src/debug.rs, crates/nvs-cli/src/debug/index.rs, crates/nvs-runtime/src/spool.rs, crates/nvs-config/src/data.rs, docs/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.md, docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md]
    extra:
      context:
        rules: [testing/the-ingester-runs-whether-or-not-anyone-is-looking, testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time, testing/the-viewer-shows-the-stream-and-hands-off-every-artifact]
  - id: 5
    title: a bad line and a bad file never stop the ingester, and a log file is tailed
    status: done
    context: [crates/nvs-cli/src/debug.rs, crates/nvs-cli/src/debug/ingest.rs, crates/nvs-cli/src/debug/tail.rs, docs/rules/testing/a-bad-line-degrades-and-a-bad-file-is-quarantined.md, docs/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.md]
    extra:
      context:
        rules: [testing/a-bad-line-degrades-and-a-bad-file-is-quarantined, testing/the-ingester-runs-whether-or-not-anyone-is-looking, errors/log-write]
  - id: 6
    title: a development `nvs serve` starts `nvs debug`, and a fixture's requests reach the index
    status: done
    context: [crates/nvs-cli/src/serve.rs, crates/nvs-cli/src/debug.rs, crates/nvs-config/src/data.rs, docs/rules/testing/nvs-debug-is-the-service-and-a-development-server-starts-it.md, docs/reference/tools/10-cli.md]
    extra:
      context:
        rules: [testing/nvs-debug-is-the-service-and-a-development-server-starts-it, testing/writing-a-debug-record-to-disk-is-its-own-gate, testing/debug-probes]
  - id: 9
    title: a derived JSON decoder fills a `secret` property, and the recorded body replaces it
    status: open
    context: [crates/nvs-types/src/derive.rs, crates/nvs-diagnostics/src/code/types.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-cli/tests/request.rs, docs/rules/security/derived-codec-qualifiers.md, docs/rules/testing/a-recorded-request-keeps-its-input-with-credentials-replaced.md]
    extra:
      context:
        rules: [security/derived-codec-qualifiers, security/secret-qualifier, security/secret-sinks-refuse, core-classes/derive-property-list, testing/a-recorded-request-keeps-its-input-with-credentials-replaced]
  - id: 8
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
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
(`crates/nvs-stdlib/src/debug.rs:213`), a request's query, `http` and `spawn` events are kept only when
the request is sampled and leave the process only as OTLP spans (`crates/nvs-server/src/serve.rs:1362`),
and a request's input is gone when it ends. `[debug]` has no `store` key
(`crates/nvs-config/src/default.toml:327`), and five designed rules describe a spool, an ingester and an
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
as the program asks for it (`crates/nvs-server/src/body.rs:279`), and recording keeps that. A body the
program never read is recorded as not read, so a replay of that request sends no body.

**Log records of a request also go to the spool.** The planning session's call. A development server
logs to its terminal, so the viewer would otherwise have no log to show. Tailing a log file is for a
file named with `nvs debug --tail`, so no record is ingested twice.

**The writer lives in `nvs-runtime` and the ingester in `nvs-cli`.** The planning session's call. The
writer must be reachable from a dump and a log record, both written through the runtime, and it is one
per thread, so it needs no lock. The ingester takes `rusqlite`, already in the tree for `Core\Db`, so the
binary gains no new C dependency.

**The spool line's fields have one home: the rule, and the type that writes them.** Stage 2's first
item adds a field table per kind to `rule:testing/the-debug-stream-reaches-the-service-through-the-disk`
§ *What a line is* and the one serde type `nvs_runtime::spool::Line` (`crates/nvs-runtime/src/spool.rs:1`)
that both the writer serializes and the ingester deserializes, with a committed sample line of each kind
(`crates/nvs-cli/tests/fixtures/debug/one-of-each.spool:1`) that the writer's test regenerates and the
ingester's tests read. One type means the two sides cannot disagree, which is the simplicity call; a
field added later is optional on read, so an older spool still ingests.

**Lanes.** Stage 1 and stage 2's first item share `crates/nvs-config/src/default.toml`,
`crates/nvs-config/src/tree.rs` and `crates/nvs-config/src/defaults.rs`, so they run as one group, in
order. Once stage 2 has landed `nvs_runtime::spool`, two lanes are disjoint: the writers (stage 3 in
`nvs-server`, then stage 7 in `nvs-stdlib`, `nvs-db` and `nvs-runtime`'s `output.rs`) and the ingester
(stages 4 then 5, in `crates/nvs-cli/src/debug/`), which tests against the committed sample spool alone.
Either lane can run as a side goal. Stage 6 joins them and runs last.

**The spool writer arms no timer; the thread that owns it seals it.** The session's call in stage 2.
`Writer::deadline` says when the open file is due, every append seals a file past it first, and
`nvs_runtime::spool::seal_due` is called at that time by one task per core, `seal_the_spool` in
`crates/nvs-cli/src/serve.rs`. That task names its wake with `spool::on_open`, which the writer runs
when it opens a file, so it parks with no deadline while the core records nothing, and it ends at the
drain; the core calls `seal_now` once more where it stops. A timer thread would have to reach a
writer's file across threads, which needs the lock the per-thread design avoids, and Windows refuses
the rename of a file another handle has open. Trade-off: one parked task per core, and one wake per
file opened.

**A body longer than 64 KiB goes out as `body` lines.** This session's call, on the handoff's
recommendation. A recorded body (the request's, as the program pulls it, or the response's, as it is
written) is held up to 64 KiB and written inline in the `request` line's `Body` when it ends there.
Past 64 KiB its bytes go out as `kind: "body"` lines, each with the trace id, `of` (`request` or
`response`), `offset` and at most 1 MiB of bytes as `text` or `base64`, and the `request` line's
`Body` carries no bytes and says `chunks` (how many lines). Recording still stops at
`[limits] request_body` and says `cut`. So no line passes 4 MiB, a body is never held whole, and a
small request stays one line for the ingester. Trade-off: per recorded body in flight, at most 1 MiB
held; one more kind for the rule's table, `Line`, the sample and the ingester.

**The `request` line is written at the door, gated by what the request ran under.** This session's
call. Every answer the door counts (`counted` in `serve_connection`) writes one `request` line, so a
`404` or a `503` is in the viewer too. For a request that ran a program, the gate and the mode are read
inside the isolate where it finishes, so a request's own `Core\Config::set` turning `store` off is
honoured, and reach the door on the completion as one field. For an answer the door wrote alone, they
are read off the snapshot it answered under. The spool gains an entry point that takes the
configuration and the trace id instead of a `Ctx`, because the door holds no request context.

**A file response's body is not recorded, and the head is copied under the host's snapshot.** This
session's calls. A file the program sent is on disk already, so its `body` says `read: false` and the
server reads it once, to send it; the rule says so. The head is copied before the handler takes the
request, gated on the host snapshot, because an `[[app]]` block cannot change `[debug]` or `[mode]`.
Trade-offs: one configuration read per request with the store off; a test made from a file
response asserts its status and headers alone.

**A recorded request body is held until the request ends, and a body whose `secret` fields cannot be
found is withheld.** This session's call. `jsonAs` and `postAs` find a `secret` field only once the
program has read the whole body, so a request body's `body` lines are written at the end, after
`spool::Recorder::redact` replaced each range the decoder's walk (`json::secret_spans`,
`json::secret_form_spans`) found. The rule already said the request body is held to the end; the
1 MiB hold now applies to a response body alone. Where the walk cannot find the fields (not UTF-8,
not one JSON document, deeper than 128 levels, a multipart form, a class that writes its own decoder)
and the class reaches a `secret` property, the body is recorded with `withheld: true` and no bytes.
The walk runs before a failed read throws. The replaced bytes keep the rest of the body exactly as
sent, so a replay sends the same JSON. Trade-offs: per recorded request in `development`, up to
`[limits] request_body` held; with no recording, one question of the body per call; with it, one more
read of the document per level the walk descends.

**A derived JSON decoder fills a `secret` property, and encoding that class is still refused.** The
user decided this (decision `d9`, option 2), so stage 9 builds it. A `secret` property on a
`#[Json\Derive]` class compiles, and `jsonAs`, `postAs` and `Core\Json::decodeAs` fill it, so the
replacement code `spool::Recorder::redact` already runs (stage 7) reaches a real field. The
session's calls on how: the refusal moves from the declaration to every call that encodes the class
or a value that reaches it, and that call is a compile error naming the property (`E0462`, its card
and message rewritten for the encode site). A class whose encoder is written by hand still decides
for itself. `#[Db\Derive]` keeps `E0462` at the declaration, because a row is both read and written
by the same derive and the decision is about request bodies. A request decoder's text is `tainted`
always, so the property is `secret tainted string`, which the qualifier rules already allow.
Trade-offs: a class with a `secret` property has a decoder and no usable encoder; one more check per
encode call site at compile time; nothing at run time beyond the replacement stage 7 built.
`rule:security/derived-codec-qualifiers` and
`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced` are amended in stage 9.

**Each event is a spool line as it is filed, and a dump is one `dump` line and no `log` line.** This
session's calls. `Ctx::file_event` writes the `query`, `http` and `gc` events as each is filed and a
`spawn` at its join, so a viewer sees them within one seal and a child never joined writes none. A
query's line takes `QuerySpan::spooled`'s fields through `Ctx::record_query`. `Core\Debug::dump`
writes its `dump` line in both rows of its table, and its log record goes through
`Ctx::write_dump_record`, which writes no `log` line. A root isolate (the door's or a scheduled run's)
files no `spawn` event, because no program spawned it and its trace is the connection's.
Trade-offs: with `store` off, one configuration read per filed event on a run that already records
spans; with it on, one buffered append per event, dump and log record.

**The index keeps two tables the planning table left out, and a search row names its source in its
rowid.** This session's calls in stage 4. `body_parts` holds each `body` line (trace id, request or
response, offset, bytes), because a body's `body` lines arrive before its `request` line and an
`inputs` row cannot exist yet. `dropped` holds each `dropped` line, so the viewer can say what was
lost. A `search` rowid is the source row's id times eight plus the table's code, so a trigger
deletes it by rowid and never scans the FTS5 table; triggers also write it and keep a request's
query, dump and log counts. `index_age` is measured from a request's start, and a row that names no
recorded request goes by its own `at_us`. `index_max` is pages in use, since a freed page is reused.
`nvs debug --once` does one pass and its deletion and exits, which is what the tests run and what a
copied-down spool wants. Trade-offs: two tables and one flag more; a deletion is one indexed lookup
per row.

**A spool line that is not UTF-8 is unparseable, a tailed one is a record, and `--tail` reads the
`json` format.** This session's calls in stage 5, written into the two ingester rules. The writer
writes UTF-8 alone, so a damaged spool line is never inserted as a record; a log line another
program wrote is converted and flagged `lossy`. A tailed line in the `text` format is an
`unparseable` row. After a rotation, the rest of `<file>.1` is read first when its key is the
checkpoint's. A `quarantine` row goes once its file is read or gone, never by age, and a file that
cannot be deleted after its batch committed is quarantined too. A file's identity is
`crate::reload::file_identity`, shared with the configuration watcher. Trade-offs: one 64 KiB buffer
and one line of at most 4 MiB while a file is read, one entry per failing file, and schema 2.

**A server's `nvs debug` watches a pipe, and a held lock ends it quietly.** This session's call in
stage 6, written into `rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`.
The server starts `nvs debug --parent` (hidden) before it binds a socket, with its `--data` (unless
the default folder) and `--config`, and a pipe as standard input. Dropping `debug::Child` closes the
pipe and waits; a killed server closes it too, and the child's wait wakes on it. A `--parent` child
that finds the lock held exits `0` with nothing printed, so the server does not check the lock
itself. The server prints one line saying the service reads into the index. A polled
`--parent <pid>` was declined: a liveness check per platform, and a reused id. Trade-offs: one
blocked thread in the child, the child's handle and pipe in the server, and one process start per
development server boot.

**Narrow test commands.** The served tests in `crates/nvs-server/src/serve.rs` are looked into with
`cargo test --lib traceresponse` or `cargo test --lib spool`, never the whole server suite; the
ingester's with `cargo test --test debug_ingest`; stage 6's end-to-end test with `cargo test --test
debug_service`. The group's one verify is `bun nv affected --run`.

**Trade-offs, all together.** Request path: with `store` off, one predicted branch, as today. With it on,
a buffered append per record, the bytes of the body the program read and of the response it wrote,
held until the request ends, and a replaced value per `secret` field the decoder fills. Memory: one write
buffer per writer while it has an open file, and the recorded request and response bodies per request in
flight. Disk: at most `spool_max` per process and `index_max` for the index. Usability: every
development request is recorded and searchable with no setup beyond `store = true`. Simplicity: four
new `[debug]` keys and one new `[trace]` key, a command, and no socket anywhere.

**A waiting request's recorded input and exception are boxed.** The gate stage's call. The service
future runs on the connection's stack and holds `input` and `ending` across the handler's wait, so
inline they took a waiting request's host stack from 8 to 10 debug pages, over the bound of 9 in
`crates/nvs-cli/src/serve.rs`'s stack test. Boxed, it passes again. Trade-off: one allocation per
recorded development request, and one per request whose program threw, against two resident stack
pages per waiting request.

**`nvs debug` and the four `[debug]` keys get their feature proofs in this goal.** The gate stage's
call, which replaces the Backlog line that put `nvs debug`'s proofs with the viewer. The proofs gate is
a floor check, and the command and the keys ship now, so each gets its `about.md`, an example, an
attack and a test case for what exists today. `debug-viewer` extends them with the viewer.

**A `RuntimeTighten` flag takes `false` and nothing else.** The gate stage's call. The rules for
`[debug] store` and `[debug] inline` said a request may turn each off, but `Core\Config::set`
refused every `RuntimeTighten` key that is not a quantity, so no program reached that path. The rules
were right and the code was wrong: `nvs_config::request::TIGHTEN_FLAGS` lists the two flags, and
`set` takes exactly `false` for them and refuses `true`, even after `false`; `restore` turns one
back on. `false` is off whatever is in force, so it always narrows. Trade-off: one comparison
against two keys per `set` that reaches it.

**A proof whose `nvs.toml` turns on `[debug] store` runs in its own data folder.** The gate stage's
call. `spawnProof` (`tools/nv/proofs/run.ts`) hands such a run `--data` in a fresh folder under
`.agent-tmp/proof-data/` and deletes it after the run, so spool files never add up beside the
binary. Read off the configuration, because that file is what makes the run write there, so a new
proof needs no directive.

## Stages

### 1. `[trace] response_header` turns the `traceresponse` header off

**Does:** a `System` key in `[trace]`, read per response, decides whether the door writes the header.

- [x] The key in the default file beside `propagate`, in `Trace`, in the defaults table, and in the directives test's key list (`rule:observability/the-traceresponse-header-can-be-turned-off`). `crates/nvs-config/src/default.toml:952`, `crates/nvs-config/src/tree.rs:1248`, `crates/nvs-config/src/defaults.rs:241`, `crates/nvs-config/src/defaults.rs:483`, `crates/nvs-config/tests/directives.rs:3567`
- [x] `respond` reads the key off the request's snapshot, the way `access_on` reads `[log] access`, and with `false` removes a header the program set and writes none. `crates/nvs-server/src/trace.rs:301`, `crates/nvs-server/src/trace.rs:316`, `crates/nvs-server/src/serve.rs:1675`
- [x] Served tests for both values, a door-written answer included, and that the access line and `Core\Server::traceId()` still carry the id with the header off, beside the existing door test. `crates/nvs-server/src/serve.rs:12191`, `crates/nvs-server/src/trace.rs:1008`
- [x] `rule:observability/a-trace-id-exists-for-every-request` amended, now that its sentence is true: every response carries the header *by default*, `[trace] response_header = false` removes the header and nothing else, the title says "emitted on the response unless the configuration turns the header off", and `seeAlso` names the new rule. `docs/rules/observability/a-trace-id-exists-for-every-request.md:9`, `data/rules/observability/a-trace-id-exists-for-every-request.json:2`
- [x] The example's page says the header can be turned off. `docs/examples/core/Server/traceId/about.md:5`
- [x] `rule:observability/the-traceresponse-header-can-be-turned-off` `shipped`, its tests in `guardedBy`. `data/rules/observability/the-traceresponse-header-can-be-turned-off.json:3`

### 2. `[debug] store` opens a spool file per writer, sealed and capped

**Does:** with the gate on, each writer appends to its own file in the data folder, seals it by rename,
and keeps its share of the cap.

- [x] `store`, `spool_max`, `index_age` and `index_max` in the default file, in `Debug`, in the defaults table and in the directive table: `store` `RuntimeTighten`, the three bounds `System` (`rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`). `crates/nvs-config/src/default.toml:327`, `crates/nvs-config/src/tree.rs:513`, `crates/nvs-config/src/directive.rs:227`, `crates/nvs-config/src/directive.rs:235`, `crates/nvs-config/src/defaults.rs:400`
- [x] The line format: the field table per kind in the rule, the `Line` type in a new `nvs_runtime::spool`, and the committed sample spool with one line of each kind (`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`). `docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md:28`, `crates/nvs-runtime/src/spool.rs:1`, `crates/nvs-cli/tests/fixtures/debug/one-of-each.spool:1`
- [x] `debug/` and `debug/spool/` in the data folder, created private and checked for ownership. `crates/nvs-config/src/data.rs:233`
- [x] A writer per thread in `nvs_runtime::spool`, shaped like the log file target: an open file only once it has a record, sealed by rename at 4 MiB or 250 ms, each line under 4 MiB, its share of `spool_max` kept by deleting its own oldest file and writing a `dropped` line. `crates/nvs-runtime/src/spool.rs:1`, `crates/nvs-runtime/src/logfile.rs:1`
- [x] With the gate on, a request records its span events whatever the sampling decision. `crates/nvs-runtime/src/ctx/trace.rs:330`
- [x] Tests in the spool module: two writers never share a file, a sealed file is never written again, the cap evicts oldest first and counts what it dropped, a spool directory owned by another account is refused (in `data.rs`, beside the check), with the gate off no file is created, and the sample spool is what the writer writes. `crates/nvs-runtime/src/spool.rs:1`, `crates/nvs-config/src/data.rs:756`

### 3. a request's outcome, input and response reach the spool

**Does:** a `request` line is written with its trace id at each request's end, and the four credential
headers never reach the disk.

**Split:** after the session lands the `request` line and the input and response capture, a writing
subagent writes the redaction and cut tests below as one batch in `serve.rs`'s test module.

- [x] The server drives the writers: `nvs_runtime::spool::set_writers` with the core count at boot, `seal_due` from each core's loop at the deadline it returned, and `seal_now` where a core stops; `nvs run` and `nvs test` call `seal_now` at exit. Then drop the rule's *Not on disk* sentence about `seal_due`. `crates/nvs-runtime/src/spool.rs:663`, `crates/nvs-runtime/src/spool.rs:692`, `crates/nvs-server/src/serve.rs:1269`
- [x] A `request` line at the request's end, from both completion paths: method, route, path, status, duration, counts, and the uncaught exception's class, message and throw site. `crates/nvs-server/src/serve.rs:1362`, `crates/nvs-server/src/serve.rs:2150`, `crates/nvs-server/src/trace.rs:278`
  - [x] The throw site reaches the door: `Failure::location`, split by `Failure::site`.
  - [x] The gate and the mode on the completion, read where the isolate finishes, and a spool entry point that takes them with the trace id.
  - [x] The line written from `counted`, with the route label, the start and the duration, on both paths.
- [x] In `development`, the input: the head from `Inbound` with the four headers replaced by `[redacted]`, and the body chunks the program pulled, up to `request_body` (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`). `crates/nvs-runtime/src/ctx/inbound.rs:804`, `crates/nvs-server/src/body.rs:279`
- [x] In `development`, the response: its status, its headers with the four replaced, and the bytes written, written through to the spool file up to `request_body` and cut there with a marker. `crates/nvs-server/src/serve.rs:1675`
  - [x] A whole body's headers and bytes, read off the answer `counted` writes (`crate::spooled::response`), with a file recorded as not read.
  - [x] A streamed body's chunks, recorded as `Answer::Streaming` hands each to `hyper`, and the `request` line written once both the join and the body's end have happened.
  - [x] A served test: a streamed body past 64 KiB reaches the spool as `body` lines whose offsets join to the bytes sent.
- [x] Tests: a request with each header shows `[redacted]` and no byte of the value in the file, an unread body is recorded as not read, a request body and a response body past `request_body` are each cut and say so, and a production request records its outcome and no header or body. `crates/nvs-server/src/serve.rs:3255`

### 7. a decoded `secret` field is replaced, and a request's events, dumps and logs reach the spool

**Does:** every other record the rules name is written as one line with its trace id, and a `secret`
property's value never reaches the disk.

**Split:** after the session lands the `secret` replacement, a writing subagent writes its tests and the
`event`, `dump` and `log` line tests as one batch.

- [x] A body decoded by `jsonAs<X>()` or `postAs<X>()` is stored with each field that lands in a `secret` property of `X` replaced by `[redacted]` (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`). `crates/nvs-stdlib/src/request.rs:3180`, `crates/nvs-stdlib/src/request.rs:2671`
- [x] `event` lines for the span events, a `query` event with its driver, SQL, rows and its call site id when it has one, and never a parameter (`rule:observability/a-query-is-a-trace-event`). `crates/nvs-db/src/span.rs:87`, `crates/nvs-db/src/span.rs:118`
- [x] `dump` lines with the bounded tree (`rule:errors/record-transformations`) and `log` lines for the request's log records, a `secret` redacted in both. `crates/nvs-stdlib/src/debug.rs:213`, `crates/nvs-runtime/src/ctx/output.rs:371`
- [x] Tests: a `secret` field decoded by `jsonAs<X>()` leaves no byte of its value in the file, a production request records its events and dumps, a query line holds no parameter, and a dump of a `secret` holds no byte of it. `crates/nvs-stdlib/src/request.rs:3180`, `crates/nvs-db/src/pg.rs:6627`
- [x] `rule:testing/the-debug-stream-reaches-the-service-through-the-disk` and `rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced` `shipped`, their tests in `guardedBy`. `data/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.json:3`, `data/rules/testing/a-recorded-request-keeps-its-input-with-credentials-replaced.json:3`

### 4. `nvs debug` ingests sealed spool files into the index

**Does:** the command creates the index, reads each sealed file, inserts its records, commits, and
deletes the file.

- [x] `nvs debug` beside `nvs lsp` in a new `debug` module, holding `debug/service.lock` in the data folder, and `rusqlite` added to `nvs-cli` from the workspace (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`). `crates/nvs-cli/src/main.rs:395`, `crates/nvs-cli/src/main.rs:582`, `crates/nvs-cli/src/debug.rs:1`, `crates/nvs-cli/Cargo.toml:81`, `Cargo.toml:1046`
- [x] The schema the rule's table names, in write-ahead mode with synchronous writes off, and a schema version that a newer binary treats as a cache miss (`rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`). `crates/nvs-cli/src/debug/index.rs:1`
- [x] Read, insert under the dedupe key, commit, then delete, with an exception group's count and first and last seen updated in the same batch, and the FTS5 rows beside their records (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`). `crates/nvs-cli/src/debug/ingest.rs:1`
- [x] The watermark in `meta` after each batch, the directory listed every 250 ms while files arrive and every two seconds after a quiet minute, and an unsealed file untouched for a minute read as sealed. `crates/nvs-cli/src/debug/ingest.rs:1`
- [x] Deletion past `index_age` or `index_max` in batches. `crates/nvs-cli/src/debug/index.rs:1`
- [x] Tests over the sample spool: a crash between commit and delete re-ingests nothing twice, a search finds a dump's text, ten throws from one site make one group of ten, and the deletion window removes every row of a request. `crates/nvs-cli/tests/debug_ingest.rs:1`
- [x] `rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time` `shipped`. `data/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.json:3`

### 5. a bad line and a bad file never stop the ingester, and a log file is tailed

**Does:** the ingester keeps what it cannot parse, quarantines what it cannot read, and reads a named
log file from a checkpoint.

**Split:** after the session lands the degrade, quarantine and tail code, a writing subagent writes the
bad-file fixtures under `crates/nvs-cli/tests/fixtures/debug/` and their tests as one batch.

- [x] A line past 4 MiB is truncated and skipped to the next newline, a line that does not parse is an `unparseable` record with its file, offset and first 4 KiB, and invalid UTF-8 is converted with a flag (`rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`). `crates/nvs-cli/src/debug/ingest.rs:1`
- [x] A file that fails to open or read is quarantined with its error and a retry after one second, doubling up to five minutes, and a full disk or an index error backs off the same way and deletes no file. `crates/nvs-cli/src/debug/ingest.rs:1`
- [x] Work per source per pass is at most 16 MiB, and sources are taken in turn. `crates/nvs-cli/src/debug.rs:1`
- [x] `nvs debug --tail <file>` reads a log file from a checkpoint of its identity, offset and first-line hash, and never writes it (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`). `crates/nvs-cli/src/debug/tail.rs:1`
- [x] Tests and their fixtures: a file that is one enormous line, a file with a bad line between two good ones, an unreadable file listed in `quarantine` while the spool keeps flowing, and a tailed file rotated and truncated in place. `crates/nvs-cli/tests/debug_ingest.rs:1`, `crates/nvs-cli/tests/fixtures/debug/one-of-each.spool:1`
- [x] `rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined` and `rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking` `shipped`. `data/rules/testing/a-bad-line-degrades-and-a-bad-file-is-quarantined.json:3`, `data/rules/testing/the-ingester-runs-whether-or-not-anyone-is-looking.json:3`

### 6. a development `nvs serve` starts `nvs debug`, and a fixture's requests reach the index

**Does:** the server starts the service as a child when it should, and the milestone's acceptance for the
debug service holds.

- [x] A development `nvs serve` with `store` on starts `nvs debug` unless the lock is held, stops its own child on exit, and the child exits within one pass of its parent being gone (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`). `crates/nvs-cli/src/serve.rs:227`, `crates/nvs-cli/src/debug.rs:1`
- [x] A production `nvs serve` never starts it. `crates/nvs-cli/src/serve.rs:227`
- [x] A test: a fixture served with `store` on has its requests in the index, joined by trace id, from sealed spool files only. With `store` off, the same run writes no spool file and its probe counts match a run with no `[debug]` block (`rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`, `rule:testing/debug-probes`). `crates/nvs-cli/tests/debug_service.rs:1`
- [x] The help text for `nvs debug` and `--tail`, and the four keys' comments in the default file. `crates/nvs-cli/src/main.rs:395`, `crates/nvs-config/src/default.toml:327`
- [x] A `# nvs debug` section in the CLI chapter for the ingester half, after `# nvs test`, says what a recorded body keeps. A body read other than through `jsonAs<X>()` or `postAs<X>()` is stored as read. `docs/reference/tools/10-cli.md:302`
- [x] `rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it` and `rule:testing/writing-a-debug-record-to-disk-is-its-own-gate` `shipped`, and `bun nv owners --closes debug-service` passes. `data/rules/testing/nvs-debug-is-the-service-and-a-development-server-starts-it.json:3`, `data/rules/testing/writing-a-debug-record-to-disk-is-its-own-gate.json:3`

### 9. a derived JSON decoder fills a `secret` property, and the recorded body replaces it

**Does:** a `secret` property on a `#[Json\Derive]` class is decoded, and an encode of that class
does not compile. A recorded request body read through `jsonAs` or `postAs` keeps no byte of it.

**Split:** the session lands the checker change and the rule edits. A writing subagent then updates
the conformance cases and the examples that pinned the old refusal, as one batch.

- [ ] `codec_field` keeps a `secret` property of a `#[Json\Derive]` class as a field the decoder fills, and `#[Db\Derive]` still reports `E0462` at the declaration (`rule:security/derived-codec-qualifiers`). `crates/nvs-types/src/derive.rs:2493`
- [ ] Every call that encodes such a class, or a value whose type reaches it, is `E0462` at the call, naming the property. Its card and message are rewritten for that site. `crates/nvs-diagnostics/src/code/types.rs:530`, `crates/nvs-stdlib/src/json.rs:1`
- [ ] The conformance case that pinned the declaration refusal is split: a `#[Json\Derive]` class with a `secret` property decodes it, and encoding it does not compile. `tests/conformance/reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst:1`
- [ ] `LOGIN` decodes `$password` instead of skipping it. A served test reads a whole JSON document through `jsonAs` and a form through `postAs`, and no spool file keeps a byte of the password. `crates/nvs-cli/tests/request.rs:531`
- [ ] `rule:security/derived-codec-qualifiers` and `rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced` say what is now true, with the new tests in `guardedBy`. The `nvs debug` CLI section says a `secret` field is decoded and replaced. `docs/rules/security/derived-codec-qualifiers.md:24`, `docs/reference/tools/10-cli.md:410`

### 8. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone. This stage verifies the whole
goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [x] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [x] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`
- [x] `bun nv proofs --gate --limit 0` owes nothing this goal's commits added. The perf figures for `request.rs` (31), `uri.rs` (23), `queue.rs` (12), `debug.rs` (9), `script.rs` (6) and `json.rs` (4) are recorded. The `debug.*` directive proofs exist, and `Core\Server::traceId`'s `about.md` is cut from 206 words. `crates/nvs-stdlib/src/request.rs:1`, `docs/examples/core/Server/traceId/about.md:1`
- [x] `bun nv site --check questions` finds `/reference/core/server/traceid/` current, and `bun nv ci --check` is green, so the goal end's pre-push check takes the push. `docs/examples/core/Server/traceId/about.md:1`
- [ ] Once stage 9 lands, `bun nv affected --run` runs again over stage 9's change, and every failure is fixed in one pass. `tools/nv/cmd/verify.ts:1`
- [ ] `bun nv verify --doc`, `bun nv owners --closes debug-service` and `bun nv ci --check` are green again after stage 9. `tools/nv/cmd/ci.ts:1`

## Handoff

### State

The user answered decision `d9` with option 2, and it is closed. Stage 9 is new: a derived JSON
decoder fills a `secret` property, and encoding the class does not compile (Standing decisions).
The gate stage is open again for stage 9's change. Stages 1 to 7 are done and stay done.

This session checked the sweep's reds against HEAD and changed no code. `env-secret-proofs` exits 0.
Each failing job of run 40174915924 (built at `9be30356e`) has a later fix:

- the Windows stalled-peer test, by `0b7eadd2c`;
- the Miri timeout of the traced-loop case, by `bd2e51f0a`;
- the two Miri `getrusage` cases, ignored under Miri by `901f4855c`;
- the `overload past capacity` null p99, by `d5aa2be18`.

`[debug] store`'s `about.md` is under 160 words since `6bcb2df7d`. At that commit, `bun nv affected --run` was green (22 of 22, 7777 tests), and so were
`bun nv verify --doc`, `bun nv owners --closes debug-service` and `bun nv ci --check`.

Tests expected to go red once stage 9's checker change lands:

- `tests/conformance/reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst`;
- the `E0462` cases in `crates/nvs-types/tests/derive.rs`;
- every `.nvs` that skips a `secret` property only to get past `E0462`.

### Next

**Stage 9, in `nvs-types`' derive check, then the rules and the tests.** Its files are
`crates/nvs-types/src/derive.rs`, `crates/nvs-diagnostics/src/code/types.rs` and
`crates/nvs-cli/tests/request.rs`.

- Drop the declaration refusal for `Format::Json` in `codec_field` and keep it for `#[Db\Derive]`.
  `crates/nvs-types/src/derive.rs:2493`, `crates/nvs-types/src/derive.rs:2647`
- Find where an encode call checks its argument's codec: `check_json_participation`, and the
  encoder's class arm in `nvs_stdlib::json`. Report `E0462` there when the class reaches a
  `secret` field. `crates/nvs-types/src/derive.rs:1777`, `crates/nvs-diagnostics/src/code/types.rs:530`
- The record's replacement is already written: `json::secret_spans` finds the field once a decoder
  fills it. `crates/nvs-stdlib/src/json.rs:3345`
- `LOGIN` stops skipping `$password`, and its test asserts no byte of the password in the spool.
  `crates/nvs-cli/tests/request.rs:531`
- Then the gate stage: `bun nv affected --run`, then the goal-end gates again.

### Backlog

- The listener, its port, the token and the browser UI are `debug-viewer`'s (`docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md`).
- The viewer's part of `nvs debug`'s feature proofs is `debug-viewer`'s, on top of the ones this goal writes (`docs/rules/testing/feature-proofs.md`).
- The query call site id is passed by `request-aware-editor`; until then a `query` event has none (`docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md`).
- A recorded request becoming a `Core\Test::request` case is `request-aware-editor`'s (`docs/rules/testing/a-recorded-request-keeps-its-input-with-credentials-replaced.md`).
