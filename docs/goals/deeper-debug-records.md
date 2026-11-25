---
looper: 1
title: "a debug record holds a request's memory, handler, sizes and stack, every event its outcome, and a job is recorded like a request"
summary: "Reached when a recorded request carries its memory peak and limit, its handler and where it is declared, and the bytes it read and wrote, an uncaught exception carries its stack frames and its previous chain, a cache, HTTP, file and process event carries its outcome, sending mail and pushing a job are events, a scheduled job, a queue job and a command run are recorded as runs with their events, dumps and logs, the debug viewer shows each of these, the inline debug block links to its request in the viewer, and the rules, the feature proofs and the website say so."
context: [crates/nvs-runtime/src/spool.rs, crates/nvs-server/src/spooled.rs, crates/nvs-cli/src/debug/index.rs, docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md]
stages:
  - id: 1
    title: the rules name every line and field the spool has and gains
    status: open
    context: [docs/agent/doc-style.md, docs/rules/observability/trace-events-carry-a-kind.md, docs/rules/testing/a-recorded-request-keeps-its-input-with-credentials-replaced.md, docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md]
  - id: 2
    title: a recorded request carries its memory peak and limit, its handler and the bytes it read and wrote
    status: open
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-runtime/src/host.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-server/src/serve.rs, crates/nvs-cli/src/debug/ingest.rs, docs/agent/coding.md]
  - id: 3
    title: an uncaught exception is recorded with its stack frames and its previous chain
    status: open
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-runtime/src/host.rs, crates/nvs-runtime/src/throwable.rs, crates/nvs-cli/src/debug/exceptions.rs, viewer/src/views/exceptions.tsx]
  - id: 4
    title: a cache, HTTP, file and process event carries its outcome
    status: open
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-stdlib/src/site.rs, crates/nvs-stdlib/src/cache.rs, crates/nvs-stdlib/src/http.rs, crates/nvs-stdlib/src/io.rs, crates/nvs-stdlib/src/process.rs]
  - id: 5
    title: sending mail and pushing a job are events
    status: open
    context: [crates/nvs-stdlib/src/mail.rs, crates/nvs-stdlib/src/queue.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/site.rs]
  - id: 6
    title: a scheduled job, a queue job and a command run are recorded as runs, and the viewer lists them
    status: open
    context: [crates/nvs-cli/src/debug/ingest.rs, crates/nvs-server/src/metrics.rs, crates/nvs-cli/src/main.rs, viewer/src/router.ts]
  - id: 7
    title: the inline debug block links to its request in the viewer
    status: open
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
viewer item needs the goal `debug-viewer-flagship` reached: the gate is that `viewer/src/ui/list.tsx`
exists. While it does not, a stage lands its runtime, index and API items, and its viewer items
wait in the handoff.

**What stops the goal.** One attempt per item and two sessions per stage. The count that decides:
a recorded request with `store` on writes at most one line more than today (the `run` line is in
place of none, and no stage adds a line per request). A stage that would add one stops and says so.

## Stages

### 1. the rules name every line and field the spool has and gains

- [ ] `docs/rules/testing/the-debug-stream-reaches-the-service-through-the-disk.md:51`: the table
  gains the `profile`, `coverage`, `copies` and `run` lines, the `cache`, `session`, `file`, `mail`
  and `queue` event kinds, and every new field with what it never holds.
- [ ] `docs/rules/observability/trace-events-carry-a-kind.md`: ten kinds, and what `mail` and
  `queue` record. Declined there: an address, a subject, a payload, a session id.
- [ ] `docs/rules/testing/a-run-is-recorded-like-a-request.md:1`: the `run` line, where each kind of
  run writes it, and the Runs view. Its Declined entry is the one above.
- [ ] `docs/rules/testing/a-recorded-exception-keeps-its-frames-and-never-a-value.md:1`: the frames,
  the chain, the two bounds, when they are copied.
- [ ] `docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md:5`: the
  new columns and the `runs` table.
- [ ] `docs/rules/errors/debug-dump.md`: the inline block's link line and its two conditions.
- [ ] `bun nv rules --render`, then `bun nv rules --check` is green.

### 2. a recorded request carries its memory peak and limit, its handler and the bytes it read and wrote

**Split:** an `Explore` subagent first reports whether a route's row carries the site of its
handler, as `file.rs:NN` and one line.

- [ ] `crates/nvs-runtime/src/host.rs:700`: `Completion` carries the peak and the limit, set at
  `crates/nvs-host/src/isolate.rs:1488` only when `store` is on.
- [ ] `crates/nvs-runtime/src/spool.rs:191`: the five optional fields on `Request`, and
  `crates/nvs-server/src/spooled.rs:230` writes them. The byte counts come from the body recorder
  (`spool.rs:257`) and the answer the door sends.
- [ ] `crates/nvs-cli/tests/fixtures/debug/one-of-each.spool`: its `request` line gains the fields.
- [ ] `crates/nvs-cli/src/debug/index.rs:78` and `ingest.rs`: the columns, and `api.rs` gives them.
- [ ] The viewer: the memory column and tile with its meter against the limit, the handler link in
  the request page's header, the response size tile, and a *Memory over* filter on the list.
- [ ] Tests in `crates/nvs-cli/tests/debug_ingest.rs` and `debug_api.rs`: the fields round-trip, and
  a line without them reads. The check `lines` is green.

### 3. an uncaught exception is recorded with its stack frames and its previous chain

**Split:** none, one session.

- [ ] `crates/nvs-runtime/src/host.rs:400`: `Failure` gains the frames and the chain, filled at
  `crates/nvs-host/src/isolate.rs:1575` under the two bounds.
- [ ] `crates/nvs-runtime/src/spool.rs:223` and `crates/nvs-server/src/spooled.rs:126`: `Exception`
  carries them.
- [ ] `crates/nvs-cli/src/debug/index.rs:222`: `exceptions` gains a JSON column of the frames and the
  chain. The group stays a class and a throw site.
- [ ] The viewer's Exception tab and the group page: the frames as a list, each a site link, with
  the program's own frames first in weight and `Core` frames dimmed, and the chain below.
- [ ] `nvs debug to-test` is unchanged, and a test says so.
- [ ] Tests: a three-deep throw records three frames, a chain of two records both, and 65 frames
  record 64 and the cut.

### 4. a cache, HTTP, file and process event carries its outcome

**Split:** none, one session.

- [ ] `crates/nvs-runtime/src/spool.rs:538`: the optional outcome fields on an event, and
  `crates/nvs-runtime/src/ctx/trace.rs:732` takes them from the watched member.
- [ ] `crates/nvs-stdlib/src/cache.rs:2860` (hit and tier), `http.rs:4651` (status and bytes),
  `io.rs:2321` (bytes), `process.rs:1713` (command and exit).
- [ ] `crates/nvs-cli/src/debug/index.rs:151` and `ingest.rs:736`: one JSON `detail` column on
  `events`, and the API gives it.
- [ ] The viewer's timeline row and the Queries tab's neighbours: a hit or miss chip, the status of
  a call, the bytes, the exit code. A cache tab lists hits and misses by key.
- [ ] Tests per kind in `debug_ingest.rs`, and one conformance case under `tests/conformance/` that
  reads a recorded `get` as a miss and then a hit.

### 5. sending mail and pushing a job are events

**Split:** none, one session.

- [ ] `crates/nvs-runtime/src/spool.rs:573`: `EventKind` gains `mail` and `queue`.
- [ ] `crates/nvs-stdlib/src/mail.rs:1176` and `queue.rs:3917`: each goes through `site::watched`
  (`crates/nvs-stdlib/src/site.rs:42`), and its member joins `RECORDED_SITE_MEMBERS`
  (`crates/nvs-stdlib/src/registry.rs:4278`).
- [ ] `crates/nvs-cli/src/debug/stream.rs:31` and `viewer/src/api.ts`: the two kinds, their tone
  and their stream filter.
- [ ] Tests: a send records a count and no address, and a push records a class and no payload.

### 6. a scheduled job, a queue job and a command run are recorded as runs, and the viewer lists them

**Split:** an `Explore` subagent first reports how `crates/nvs-cli/src/debug/ingest.rs` treats
records whose trace id has no `request` line, and where a queue worker ends a job.

- [ ] `crates/nvs-runtime/src/spool.rs`: the `run` line and its writer.
- [ ] The three writers: the scheduler beside `crates/nvs-server/src/metrics.rs:272`, the queue
  worker where a job ends, and `crates/nvs-cli/src/main.rs:822` for `nvs run` and `nvs test`.
- [ ] `crates/nvs-cli/src/debug/index.rs`: the `runs` table, under the deletion window, and
  `GET /api/runs` and `GET /api/run`.
- [ ] The viewer: `/runs` and `/runs/<trace>` on the shared list and the request page's tabs, and
  the rail's Runs item.
- [ ] Tests: each kind of run round-trips, and a failed run carries its exception. The check `shown`
  is green.

### 7. the inline debug block links to its request in the viewer

**Split:** none, one session.

- [ ] `crates/nvs-runtime/src/ctx/output.rs:568`: the block's link line, under the two conditions.
- [ ] A conformance case: the line is present in development with `inline` and `store` on, and
  absent with either off and in production.

### 8. the feature proofs, the help and the website say what a record holds

- [ ] `website/src/content/docs/tools/debug-viewer/index.mdx`: memory, the handler link, the stack,
  event outcomes, the Runs view and the inline link, with a screenshot of each new view, then stamped.
- [ ] Every page that documents `[debug] inline` is reread and stamped. An `Explore` subagent lists
  them first.
- [ ] `website/src/data/changes.json`: `changed` the debug viewer (memory, handler, sizes, stack,
  outcomes), `new` the Runs view, `changed` `[debug] inline`.
- [ ] `bun nv proofs --owed --goal deeper-debug-records` names nothing, and the check `proofs` is
  green.
- [ ] `bun nv site --check stale`, `changes` and `shots` are green.

### 9. the goal is verified once, and every failure is fixed

- [ ] The newcomer review, [prose.md](../agent/prose.md) § *A newcomer reads it*, over every page,
  help text and card this goal wrote, and `bun nv site --check review` is green.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by an interactive session on 2026-11-25, with the goal `debug-viewer-flagship`. Nothing of
this goal is built. The `path:line` anchors come from a read-only survey of that day, so a stage
checks an anchor before it edits there.

### Next

Stage 1 from its first item.

### Backlog

- Transaction begin, commit and rollback as events, and a WebSocket's frames and bytes at close.
  Both were found by the survey and left out to keep the goal to one line per request.
