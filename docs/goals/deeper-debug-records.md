---
looper: 1
title: "a debug record holds a request's memory, handler, sizes and stack, every event its outcome, and a job is recorded like a request"
summary: "Reached when a recorded request carries its memory peak and limit, its handler and where it is declared, and the bytes it read and wrote, an uncaught exception carries its stack frames and its previous chain, a cache, HTTP, file and process event carries its outcome, sending mail and pushing a job are events, a scheduled job, a queue job and a command run are recorded as runs with their events, dumps and logs, the debug viewer shows each of these, the inline debug block links to its request in the viewer, and the rules, the feature proofs and the website say so."
context: [crates/nvs-runtime/src/spool.rs, crates/nvs-server/src/spooled.rs, crates/nvs-cli/src/debug/index.rs, docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md]
stages:
  - id: 1
    title: the rules name every line and field the spool has and gains
    status: done
    context: [docs/agent/doc-style.md, docs/rules/observability/trace-events-carry-a-kind.md, docs/rules/testing/a-recorded-request-keeps-its-input-with-credentials-replaced.md, docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md]
  - id: 2
    title: a recorded request carries its memory peak and limit, its handler and the bytes it read and wrote
    status: done
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-runtime/src/host.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-server/src/serve.rs, crates/nvs-cli/src/debug/ingest.rs, docs/agent/coding.md]
  - id: 3
    title: an uncaught exception is recorded with its stack frames and its previous chain
    status: done
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-runtime/src/host.rs, crates/nvs-runtime/src/throwable.rs, crates/nvs-cli/src/debug/exceptions.rs, viewer/src/views/exceptions.tsx]
  - id: 4
    title: a cache, HTTP, file and process event carries its outcome
    status: done
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-stdlib/src/site.rs, crates/nvs-stdlib/src/cache.rs, crates/nvs-stdlib/src/http.rs, crates/nvs-stdlib/src/io.rs, crates/nvs-stdlib/src/process.rs]
  - id: 5
    title: sending mail and pushing a job are events
    status: done
    context: [crates/nvs-stdlib/src/mail.rs, crates/nvs-stdlib/src/queue.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/site.rs]
  - id: 6
    title: a scheduled job, a queue job and a command run are recorded as runs, and the viewer lists them
    status: done
    context: [crates/nvs-cli/src/debug/ingest.rs, crates/nvs-server/src/metrics.rs, crates/nvs-cli/src/main.rs, viewer/src/router.ts]
  - id: 7
    title: the inline debug block links to its request in the viewer
    status: done
    context: [docs/rules/errors/debug-dump.md, crates/nvs-server/src/debug_style.rs, crates/nvs-runtime/src/ctx/output.rs]
  - id: 8
    title: the feature proofs, the help and the website say what a record holds
    status: open
    context: [docs/agent/prose.md, website/src/content/docs/tools/debug-viewer/index.mdx, website/src/data/changes.json]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: lines
    stage: 2
    name: "every spool line the writer writes reads back with its new fields, and a line without them still reads"
    run: [cargo, test, --test, debug_ingest]
    timeout: 1800
  - id: shown
    stage: 6
    name: "the API gives a request's memory, handler, sizes and frames, each event's outcome, and the runs"
    run: [cargo, test, --test, debug_api]
    timeout: 1800
  - id: proofs
    stage: 8
    name: "every feature the goal's work reaches has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, deeper-debug-records]
    timeout: 600
  - id: close
    stage: 9
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, deeper-debug-records]
    timeout: 900
---

# a debug record holds a request's memory, handler, sizes and stack, every event its outcome, and a job is recorded like a request

## Why

The debug viewer can show only what a record holds, and a record holds less than the runtime knows
at the moment it writes one.

- A `request` line has the method, the route pattern, the path, the status, the duration and the
  exception (`crates/nvs-runtime/src/spool.rs:191`). The memory peak is computed three statements
  away, for a histogram (`crates/nvs-host/src/isolate.rs:1488`), and never reaches the line. Neither
  does the limit, the handler the route matched (`crates/nvs-runtime/src/routes.rs:646`) or a byte
  count.
- An exception is a class, a message and one `file:line` (`spool.rs:223`). The throwable still has
  its frames and its `previous` chain when the record is built
  (`crates/nvs-host/src/isolate.rs:1575`, `crates/nvs-runtime/src/throwable.rs:831`), and both are
  left behind.
- A `cache` event has a key and a member name, and not whether a `get` hit
  (`crates/nvs-stdlib/src/cache.rs:2860`). An `http` event's status is inside its rendered label
  (`crates/nvs-stdlib/src/http.rs:4651`). A process's exit code is known where its event closes
  (`crates/nvs-stdlib/src/process.rs:1713`) and is not recorded.
- Sending mail (`crates/nvs-stdlib/src/mail.rs:1176`) and pushing a job
  (`crates/nvs-stdlib/src/queue.rs:3917`) write no event.
- Only the HTTP door writes a `request` line (`crates/nvs-server/src/serve.rs:2066`). A
  `[[schedule]]` job, a queue job and an `nvs run` write events, dumps and logs with a trace id and
  no line that says what ran and how it ended, so the viewer has no row for them.
- The rule's table of lines is behind the code: it lacks the `profile`, `coverage` and `copies`
  lines and the `cache`, `session` and `file` event kinds
  (`docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md:51`).

## Standing decisions

**The user's calls.** The debug viewer becomes a flagship feature, and the goal looks for what
Novis itself can do better for it. Every decision below is the session's that wrote this goal.

**Nothing is recorded that `[debug] store` does not already gate, and nothing costs where it is
off.** Each new fact is one the runtime has at a point where it already writes a line or closes an
event. With `store` off no new branch runs on the request path beyond the one that already skips
the line. This is the latency priority, and it is why each item below names where the fact already
is.

**Every new field is optional,** so a spool written before this goal still reads, as
`the-debug-stream…` already requires, and the viewer draws a figure only when its record has it.
The index's schema version rises once per stage that adds a column, and the index is rebuilt.

**A request line gains five facts.** `memory_peak` and `memory_limit` in bytes, `handler` (the
name `Route::handler()` gives) with `handler_site` (`file` and `line`, when the route table has
one: an `Explore` subagent settles that first, and without one the name alone is recorded),
`bytes_in` (the body bytes read) and `bytes_out` (the body bytes written). The door writes the
line, so the two memory figures travel on `Completion` (`crates/nvs-runtime/src/host.rs:700`).
Spends: two integers per completed request while `store` is on.

**A stack is frames of function, file and line, and never a value.** `Failure`
(`crates/nvs-runtime/src/host.rs:400`) gains the frames `Throwable::frames()` gives, in the shape
`crates/nvs-runtime/src/floor.rs:166` already renders, and the `previous` chain as a list of class,
message, site and frames. At most 64 frames and 8 links are kept, and the record says when either
was cut. An argument's value is never in a frame, because a trace is a `secret` sink
(`rule:security/redaction-reaches-the-tools-own-renderings`). The frames are copied only when the
failure is uncaught and `store` is on. A caught exception stays unrecorded.

**An event's outcome is a small set of named fields, never text to parse.** `cache`: `hit` on a
`get`, and `tier` (`memory` or `redis`). `http`: `status`, `bytes_in`, `bytes_out`. `file`:
`bytes`. A process `spawn`: `command` (the program's name, never its arguments, which may carry a
credential) and `exit`. The `*Secret` cache members keep recording no key
(`crates/nvs-stdlib/src/cache.rs:2947`).

**Mail and queue are two new event kinds, and they record counts and names only.** `mail`:
the number of recipients and whether the send succeeded. Never an address, a subject or a body.
`queue`: the queue's name, the job's class and its delay. Never the payload.
`rule:observability/trace-events-carry-a-kind` goes from eight kinds to ten, and
`rule:observability/four-kinds-become-a-span` is unchanged: neither becomes a span. Declined: a
session id on a record, hashed or not, because it links a person's requests.

**A run is a line of its own kind, `run`.** It has `of` (`schedule`, `queue` or `command`), `name`
(the schedule's name, the job's class, or the command and its script), `start_us`, `duration_us`,
`outcome` (`ok`, `failed` or `cancelled`), the memory peak and limit, and the exception with its
stack. The viewer gains a Runs view beside Requests with the same list, the same page and the same
tabs, less Request and Response. A scheduled run is written where its outcome already feeds
`nvs_schedule_runs_total` (`crates/nvs-server/src/metrics.rs:272`), and a command run where
`nvs run` and `nvs test` seal their spool (`crates/nvs-cli/src/main.rs:822`). Declined: writing a
run as a `request` line with a made-up method, because every list and filter would then need to
know the fake.

**The inline block names its request.** With `[debug] inline` on in development, the block the
runtime already writes into an HTML body (`rule:errors/debug-dump`) gains one line: the query count
so far and a link to the request in the viewer, built from `[debug] listen`, the trace id and the
span id. The link carries no token: the browser that has opened the viewer once has its cookie. It
is written only when `store` is on, because otherwise the viewer has no such request. Declined: a
toolbar that updates after the response, which needs a script in the program's page.

**Lanes.** Stages 2 and 3 share `isolate.rs` and `host.rs` and run in order. Stages 4 and 5 share
`site.rs` and run in order. Stage 6 needs stages 2 and 3. Stage 7 is alone in its files. Every
viewer item builds on `viewer/src/ui/list.tsx`, which the goal in front of this one on the chain,
`debug-viewer-flagship`, lands. If that file does not exist, a stage lands its runtime, index and
API items, and its viewer items wait in the handoff.

**Stage 1 settled four names the rules now state.** An `http` event's status is `http_status`,
because an event's `status` is already a `call` exit's checked-return status. A `queue` event's and
a queue run's job is its script, because `Core\Queue::push` takes a script and has no job class. In
the index an event's outcome is one nullable JSON column, `outcome`, because the viewer draws it and
never filters or sums on it. An `exceptions` row names its `request` or its `run`, both nullable.

**Stage 2 settled what each request figure is.** The route tables keep no handler declaration
site, so a line records the handler's name alone, and the rule declines `handler_site`. A link to
the handler's source is the viewer's to make from the name. `bytes_in` is the request body bytes
the server read off the wire (`Supply::carried`, the count `upload_total` is held to), and
`bytes_out` is the body length the head knew, the access line's `bytes`, so a streamed answer has
none. The memory limit is the ceiling the operator wrote, `memory_limit` plus the fatal reserve,
and `None` with no ceiling. The facts cross on `Completion::recorded` and the door's boxed
`spooled::Ran`, built only when `store` is on, so a request with it off allocates nothing more.
The list's memory filter is `min_memory`, in bytes, beside `min_us`.

**Stage 3 settled the stack's shape.** The spool's `Exception` carries a flattened `Stack`
(`frames`, `frames_cut`, `previous`, `previous_cut`), one type shared by the line, `Failure` (boxed,
`None` with `store` off) and the API. A frame's and a link's `file` and `line` are optional,
because a `Core` frame and a raise with no site have none. The index keeps the frames and the chain
in two JSON columns, as the rule's row names them, and does not search them. A group's page shows
its latest throw's stack (`latest` on `GET /api/exceptions?class=&file=&line=`), one indexed read.
A `Core` frame is one whose function starts with `Core\`.

**Stage 4 settled the outcome's shape.** `spool::Outcome` is one flattened struct of optional
fields, and a kind sets only its own. `tier` is on every cache call and `hit` only on a plain
`get`: a `getSecret` with a `fill` returns a value on a miss, so its result does not say which.
`bytes` is set on a whole-file read and on a write, and `bytes_in` only where an outbound call
read the whole body, so a streamed reply and a WebSocket have none. `command` is the program's
file name, never its directory. `Record::Event` is boxed, as `Record::Request` is, because the
outcome made it the large variant: one allocation per recorded event line, only with `store` on.
The end-to-end miss-then-hit check is a Rust test in `crates/nvs-cli/tests/request.rs`, which
already reads a served request's spool lines, and not a conformance case: no `.nvst` section
reads the spool, and a new harness section for one check is a mechanism nothing else uses.

**Stage 5 settled where mail and queue are filed.** A `mail` and a `queue` event name no call
site, so neither member joins `RECORDED_SITE_MEMBERS`: a site would add an argument to both
members' ABI, and the editor's lens reads only the five kinds it counts by line. Both file through
`site::watched_with` with a null site word. `recipients` counts `to`, `cc` and `bcc` once all of
them parsed, so a send that fails later still has it, and under `nvs test` a kept mail is `ok`.
`delay_ms` is set only where the push named a `runAt`, clamped at zero. A push's `queue` and
`job` are cloned only when the request is recorded. The index keeps both kinds' outcomes in the
existing `outcome` column, so the schema does not rise, and no `*_us` sum is added for them.

**Stage 6 settled where each run line is written.** A scheduled fire and a queue attempt each
write theirs from the isolate's `Completion`, which already carries the gate, the memory figures
and the stack copied under it, and now the child's trace, so the line joins the run's events. A
fire's run and a job's attempt are `cancelled` when their task's context says so. `nvs run`
writes its line inside the program task, after the exit hooks and the session's write-back, and
not in `sealed`, because only the task still holds the context and the uncaught throw. An `exit`
with code `0` is `ok`. `nvs_schedule_runs_total` is declared but never counted in the tree, so
the scheduled writer sits beside `Fires::ran` instead. A run is forgotten by its start, as a
request is, with every row of its trace. `nvs test` of a program's `#[Test]` methods writes one
line per invocation where its suite returns, named `test` and the entry file, and a `.nvst` tree
writes none of its own, because each case is an `nvs` process that writes its own. A failed test
is no uncaught exception, so that line carries none.

**Stage 6 settled the runs API.** `GET /api/runs` and `GET /api/run?trace=` live in
`crates/nvs-cli/src/debug/runs.rs`, beside the request routes, because `api.rs` is already past
1,300 lines. A run is opened by its trace id alone: a run draws its own trace, so every record of
that trace is its own, and the newest run of a trace is given. The event, dump, log and exception
reads are shared with the request page (`api::events`, `dumps`, `logs`, `exception`, over
`api::Records`). An exception group's sample reads a run's throw as a request's, with no route,
and the group's first page lists at most `limit` of its newest runs beside its requests, with no
cursor of its own.

**Stage 6 settled the viewer's Runs view.** A run's page is `/runs/<trace>`, because a run draws
its own trace. Its tabs are the request page's less Request, Response and Coverage, since a run
records no input, output or coverage. The tabs draw from one shared shape, `Records` in
`viewer/src/api.ts`, and `ownerOf` gives the start and duration of a request or a run, so the
Timeline, Queries, Cache, Dumps, Logs and Exception tabs are the same components on both pages. A
run is opened as a request is, never through a request row with a made-up method. The run list
cannot be saved as a filter yet: `SavedView` and the settings' `view` would both gain a value, and
nothing asked for it. A group's runs are a plain list of links on its page, because the API gives
at most `limit` of them with no cursor.

**Stage 7 settled where the inline link is pinned.** A `.nvst` case with `--GET--` runs through
`nvs run`'s request emulation, which writes its body to standard output and never appends the
inline block, so no conformance case can see the line. Two served tests in
`crates/nvs-cli/tests/request.rs` pin it instead, each with `// covers: directive:debug.inline`,
where stage 4 put its served check. The link names the trace and span the response's
`traceresponse` gives. The count is the request's `query` trace events, one pass over a vector
already capped at the span ceiling. A `listen` with no usable address, such as the port `0`,
leaves the count alone. The line uses the existing `nvs-record` class, so the stylesheet is
unchanged. Spends: one short line per response with a block and `store` on.

**What stops the goal.** One attempt per item and two sessions per stage. The count that decides:
a recorded request with `store` on writes at most one line more than today (the `run` line is in
place of none, and no stage adds a line per request). A stage that would add one stops and says so.

## Stages

### 1. the rules name every line and field the spool has and gains

- [x] `docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md:51`: the table
  gains the `profile`, `coverage`, `copies` and `run` lines, the `cache`, `session`, `file`, `mail`
  and `queue` event kinds, and every new field with what it never holds.
- [x] `docs/rules/observability/trace-events-carry-a-kind.md`: ten kinds, and what `mail` and
  `queue` record. Declined there: an address, a subject, a payload, a session id.
- [x] `docs/rules/testing/a-run-is-recorded-like-a-request.md:1`: the `run` line, where each kind of
  run writes it, and the Runs view. Its Declined entry is the one above.
- [x] `docs/rules/testing/a-recorded-exception-keeps-its-frames-and-never-a-value.md:1`: the frames,
  the chain, the two bounds, when they are copied.
- [x] `docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md:5`: the
  new columns and the `runs` table.
- [x] `docs/rules/errors/debug-dump.md`: the inline block's link line and its two conditions.
- [x] `bun nv rules --render`, then `bun nv rules --check` is green.

### 2. a recorded request carries its memory peak and limit, its handler and the bytes it read and wrote

**Split:** an `Explore` subagent first reports whether a route's row carries the site of its
handler, as `file.rs:NN` and one line.

- [x] `crates/nvs-runtime/src/host.rs:700`: `Completion` carries the peak and the limit, set at
  `crates/nvs-host/src/isolate.rs:1488` only when `store` is on.
- [x] `crates/nvs-runtime/src/spool.rs:191`: the five optional fields on `Request`, and
  `crates/nvs-server/src/spooled.rs:230` writes them. The byte counts come from the body recorder
  (`spool.rs:257`) and the answer the door sends.
- [x] `crates/nvs-cli/tests/fixtures/debug/one-of-each.spool`: its `request` line gains the fields.
- [x] `crates/nvs-cli/src/debug/index.rs:78` and `ingest.rs`: the columns, and `api.rs` gives them.
- [x] The viewer: the memory column and tile with its meter against the limit, the handler link in
  the request page's header, the response size tile, and a *Memory over* filter on the list.
  - [x] `viewer/src/api.ts`: `Row` gains `memory_peak`, `memory_limit`, `handler`, `bytes_in`,
    `bytes_out` (each `number | null`, `handler` a string), and the filter `min_memory`.
  - [x] The request list: a memory column, and a *Memory over* filter beside the duration one in
    `viewer/src/views/requests.tsx` (`min_us` is the pattern at :85, :100, :113, :124, :333).
  - [x] The request page: a memory tile with a meter against the limit, a response size tile, and
    the handler's name in the header.
  - [x] The viewer's tests and its build are green over the rewritten fixtures in
    `viewer/test/fixtures/`.
- [x] Tests in `crates/nvs-cli/tests/debug_ingest.rs` and `debug_api.rs`: the fields round-trip, and
  a line without them reads. The check `lines` is green.

### 3. an uncaught exception is recorded with its stack frames and its previous chain

**Split:** none, one session.

- [x] `crates/nvs-runtime/src/host.rs:400`: `Failure` gains the frames and the chain, filled at
  `crates/nvs-host/src/isolate.rs:1575` under the two bounds.
  - [x] `Thrown::stack()` walks the backtrace and the `previous` chain under 64 and 8
  - [x] `Failure::stack` is filled under the store gate alone
- [x] `crates/nvs-runtime/src/spool.rs:223` and `crates/nvs-server/src/spooled.rs:126`: `Exception`
  carries them.
  - [x] `Stack`, `Frame` and `Link` in the spool, optional on read
  - [x] the sample spool has a stack and is rewritten
- [x] `crates/nvs-cli/src/debug/index.rs:222`: `exceptions` gains a JSON column of the frames and the
  chain. The group stays a class and a throw site.
  - [x] schema 12, `frames`, `frames_cut`, `previous`, `previous_cut`
  - [x] the request API and a group's `latest` give them
- [x] The viewer's Exception tab and the group page: the frames as a list, each a site link, with
  the program's own frames first in weight and `Core` frames dimmed, and the chain below.
  - [x] `viewer/src/ui/stack.tsx` and its styles
  - [x] the request and exceptions browser tests check it
- [x] `nvs debug to-test` is unchanged, and a test says so.
  - [x] `an_exceptions_frames_and_chain_reach_its_row_and_to_test_is_unchanged`
- [x] Tests: a three-deep throw records three frames, a chain of two records both, and 65 frames
  record 64 and the cut.
  - [x] three unit tests in `throwable.rs`

### 4. a cache, HTTP, file and process event carries its outcome

**Split:** none, one session.

- [x] `crates/nvs-runtime/src/spool.rs:538`: the optional outcome fields on an event, and
  `crates/nvs-runtime/src/ctx/trace.rs:732` takes them from the watched member.
  - [x] `spool::Outcome` and `CacheTier`, flattened into `Event`, and `Record::Event` boxed
  - [x] `record_site_event` and `close_spawn_with` take an outcome
- [x] `crates/nvs-stdlib/src/cache.rs:2860` (hit and tier), `http.rs:4651` (status and bytes),
  `io.rs:2321` (bytes), `process.rs:1713` (command and exit).
  - [x] `site::watched_with` reads the outcome off the result only when the event is filed
  - [x] cache, file, HTTP and process members pass theirs
- [x] `crates/nvs-cli/src/debug/index.rs:151` and `ingest.rs:736`: one JSON `outcome` column on
  `events`, and the API gives it.
  - [x] schema 13, and the API gives each event's `target` and `outcome`
- [x] The viewer's timeline row and the Queries tab's neighbours: a hit or miss chip, the status of
  a call, the bytes, the exit code. A cache tab lists hits and misses by key.
  - [x] the timeline's detail and `Hit` chip, and the Cache tab
  - [x] `viewer/test/outcome.test.ts`, and `bun nv viewer --verify` is green
- [x] Tests per kind in `debug_ingest.rs`, and one end-to-end test that reads a recorded `get` as a
  miss and then a hit (the conformance case it replaces is a standing decision).
  - [x] `each_kinds_outcome_reaches_its_events_row_and_an_event_without_one_has_none`
  - [x] a test in `crates/nvs-cli/tests/request.rs` that runs `get`, `put`, `get` with `store`
    on and reads the two `get` lines' `hit` as `false` and then `true`, with `tier` `memory`

### 5. sending mail and pushing a job are events

**Split:** none, one session.

- [x] `crates/nvs-runtime/src/spool.rs:573`: `EventKind` gains `mail` and `queue`.
  - [x] `TraceKind` and `spool::EventKind` gain both, and `Outcome` gains `recipients`, `ok`,
    `queue`, `job` and `delay_ms`
- [x] `crates/nvs-stdlib/src/mail.rs:1176` and `queue.rs:3917`: each goes through `site::watched`
  (`crates/nvs-stdlib/src/site.rs:42`). Neither joins `RECORDED_SITE_MEMBERS`: a standing
  decision gives them no call site.
  - [x] `send` and `push` are each a body `watched_with` files the event around
- [x] `crates/nvs-cli/src/debug/stream.rs:31` and `viewer/src/api.ts`: the two kinds, their tone
  and their stream filter.
  - [x] `EVENT_KINDS`, `KINDS`, `RECORD_KINDS`, `KIND_NAMES`, `--k-mail` and `--k-queue`
  - [x] `outcomeText` writes the count, the script, the queue and the delay
- [x] Tests: a send records a count and no address, and a push records a class and no payload.
  - [x] `a_served_send_and_push_record_counts_and_names_and_no_data` in `request.rs`
  - [x] `viewer/test/outcome.test.ts`, and `bun nv viewer --verify` is green

### 6. a scheduled job, a queue job and a command run are recorded as runs, and the viewer lists them

**Split:** an `Explore` subagent first reports how `crates/nvs-cli/src/debug/ingest.rs` treats
records whose trace id has no `request` line, and where a queue worker ends a job.

- [x] `crates/nvs-runtime/src/spool.rs`: the `run` line and its writer.
  - [x] `Record::Run`, `Run`, `RunOf`, `RunOutcome`, and `Exception::of` and `Exception::thrown`.
  - [x] `spool::run` over a `Completion` and `spool::run_in` over a `Ctx`.
  - [x] The sample spool has a mail, a queue and a run line.
- [x] The three writers: the scheduler beside `crates/nvs-server/src/metrics.rs:272`, the queue
  worker where a job ends, and `crates/nvs-cli/src/main.rs:822` for `nvs run` and `nvs test`.
  - [x] A scheduled fire, in `fire()` (`crates/nvs-server/src/schedule.rs`).
  - [x] A queue attempt, in `run()` (`crates/nvs-cli/src/worker.rs`).
  - [x] `nvs run`, where its program task ends (`crates/nvs-cli/src/main.rs`, `run_run`).
  - [x] `nvs test`, one run line per invocation, named `test <path>`
    (`crates/nvs-cli/src/runner.rs`).
- [x] `crates/nvs-cli/src/debug/index.rs`: the `runs` table, under the deletion window, and
  `GET /api/runs` and `GET /api/run`.
  - [x] The `runs` table, its triggers, `exceptions.run`, schema 14, the deletion window and the
    rebuild's sequence.
  - [x] The ingester's `run` arm and the shared `exception_row`.
  - [x] `GET /api/runs` (paged on `id`, filters `of`, `name`, `outcome`) and `GET /api/run?trace=`.
  - [x] `crates/nvs-cli/src/debug/exceptions.rs:291` joins `requests` alone: a run's exception
    joins its run too.
- [x] The viewer: `/runs` and `/runs/<trace>` on the shared list and the request page's tabs, and
  the rail's Runs item.
  - [x] `viewer/src/api.ts`: `RunRow`, `RunPage`, `RunDetail` and the `runs` field of a group's
    page, and `viewer/test/api.test.ts` checks them against `runs.json`, `run.json` and
    `exception.json`.
  - [x] `viewer/src/router.ts`: `/runs` and `/runs/<trace>`, and the rail's Runs item.
  - [x] The Runs list on `viewer/src/ui/list.tsx`, with the `of`, `outcome`, duration and memory
    filters.
  - [x] The run page: the request page's tabs less Request and Response, and the group page's
    runs.
  - [x] `bun nv viewer --verify` is green, and `viewer/dist/` is rebuilt.
- [x] Tests: each kind of run round-trips, and a failed run carries its exception. The check `shown`
  is green.
  - [x] `each_kind_of_run_reaches_its_row_and_a_failed_run_keeps_its_exception`
    (`crates/nvs-cli/tests/debug_ingest.rs`).
  - [x] A `debug_api` test of `GET /api/runs` and `GET /api/run`, then the check `shown`.

### 7. the inline debug block links to its request in the viewer

**Split:** none, one session.

- [x] `crates/nvs-runtime/src/ctx/output.rs:568`: the block's link line, under the two conditions.
  - [x] `Ctx::inline_link` writes the count and the link ahead of the blocks.
  - [x] `nvs_config::viewer::origin` and `linkable` give both links the viewer's origin.
- [x] A test, served in Rust as the stage 7 decision says: the line is present in development with `inline` and `store` on, and
  absent with either off and in production.

### 8. the feature proofs, the help and the website say what a record holds

- [ ] `website/src/content/docs/tools/debug-viewer/index.mdx`: memory, the handler link, the stack,
  event outcomes, the Runs view and the inline link, with a screenshot of each new view, then stamped.
  - [x] The text: handler, memory and size tiles, the Cache tab, event results, the stack, the
    inline link and a Runs section, stamped.
  - [ ] A scheduled or queue run's exception and frames name a path relative to the project, as a
    request's do, and the run list's name column keeps the name when the row has an exception.
  - [ ] `runs.shot` taken again and shown on the page, and a run page shot (`run.shot`) of the
    failed run's Exception tab with its stack.
  - [ ] Every existing viewer shot taken again, since the rail gained **Runs**.
- [x] Every page that documents `[debug] inline` is reread and stamped. An `Explore` subagent lists
  them first.
- [x] `website/src/data/changes.json`: `changed` the debug viewer (memory, handler, sizes, stack,
  outcomes), `new` the Runs view, `changed` `[debug] inline`.
- [x] A browser test of `/runs` and `/runs/<trace>` beside `viewer/test/requests.browser.test.ts`,
  the Runs view's feature proof.
- [ ] `bun nv proofs --owed --goal deeper-debug-records` names nothing, and the check `proofs` is
  green.
  - [ ] The 114 member benches that stages 4 and 5 made stale are measured again and committed
    (113 are; `Core\IO\Metadata::modifiedAt` is not).
  - [ ] `bun nv proofs --gate --limit 0` and the check `proofs` are green.
- [ ] `bun nv site --check stale`, `changes` and `shots` are green.
  - [x] `stale` and `changes` are green.
  - [ ] `shots` is green.

### 9. the goal is verified once, and every failure is fixed

- [ ] The newcomer review, [prose.md](../agent/prose.md) § *A newcomer reads it*, over every page,
  help text and card this goal wrote, and `bun nv site --check review` is green.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stage 8 is half done. `7f4bb5fc70` adds `viewer/test/runs.browser.test.ts`, green once. `777e626974`
wrote the viewer page's text (tiles, Cache tab, event results, stack, inline link, Runs), the
reference's `/runs` rows, both `about.md` files and the `[debug] inline` help, and three
`changes.json` entries. Three `debug_api` tests cover `tools:cli/the-debug-viewer`, and five
`Core\Test\Response` members are `unseen`. `site --check stale changes prose` is green.
`22269cb53d` records 113 of the 114 stale member benches. `Core\IO\Metadata::modifiedAt` failed its
growth check twice with unsteady slopes (1.22, 4.13): noise under WSL is likely, not shown.

**A bug the shot found.** `website/snippets/tools/debug-viewer/runs/` (two `on = "start"` schedules)
and `runs.shot` are in the tree, uncommitted. A failed scheduled run's exception site is an
absolute path, where a request's is relative: server work compiles its script from the resolved
path (`crates/nvs-server/src/schedule.rs:658`), and `spool::Exception::of`
(`crates/nvs-runtime/src/spool.rs:338`) copies it. The long site also hides the run's name
(`viewer/src/views/runs.tsx` `RunCells`).

Expected red at stage 9: `shots` for every viewer shot (the rail gained **Runs**), a browser test
that counts the rail's items, and `site --check review` for the rewritten texts.

### Next

Stage 8's second session:

1. Make a run's exception site, its frames, its dumps' and its logs' files relative to the project
   for scheduled and queue runs, as a request's are. First find where a request's file becomes
   relative (a request compiles `public/index.nvs` from the app root) and do the same for server
   work's script, or strip the app root once in `spool::run`. A test in
   `crates/nvs-cli/tests/request.rs` beside stage 6's run-line test pins it.
2. `RunCells`: the name keeps its room when the row has an exception, as a request row's path does.
3. `bun nv site --shoot website/snippets/tools/debug-viewer/runs.shot`, and a `run.shot` with a
   `run/` app (a copy of `runs/`) that opens the failed run's Exception tab
   (`click .runs .threw`, then `click .tabs a[href$='tab=exception']`, `wait .exception .frames`).
   Add both `<Shot>`s to the Runs section, look at each picture, then take every viewer shot again.
4. `bun nv proofs --record-perf --only 'Core\IO\Metadata::modifiedAt'` on a quiet machine. If it
   fails again with a steady slope, read `benches/members/core/IO-Metadata/modifiedAt.nvs` for
   work that grows with its batch.
5. `bun nv proofs --gate --limit 0` and `looper check --name proofs`, then `site --check shots`.

### Backlog

- Transaction begin, commit and rollback as events, and a WebSocket's frames and bytes at close.
  Both were found by the survey and left out to keep the goal to one line per request.
- `nvs run`'s request emulation (the `--GET--` cases) never writes the inline block. If it should,
  a conformance case could then pin the link line.
