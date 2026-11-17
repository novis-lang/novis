---
looper: 1
title: "a request's fixed cost is counted, and every part of it is work the answer needs"
summary: "Reached when bun nv bench-compare --direct prints each server's CPU time per request with no proxy, a test holds the clock readings and the allocations of a served request to a recorded count, a request body that has arrived whole reaches the program with no wait, and every allocation left on a hello request is listed with the reason the answer needs it."
context: [benches/compare/README.md, crates/nvs-server/src/serve.rs, crates/nvs-server/src/body.rs, crates/nvs-host/src/watchdog.rs]
stages:
  - id: 1
    title: the direct measure, and the counts of one request
    status: done
    context: [tools/nv/cmd/bench-compare.ts, benches/compare/profile.sh]
  - id: 2
    title: a request body that has arrived whole reaches the program with no wait
    status: done
    context: [crates/nvs-stdlib/src/request.rs]
  - id: 3
    title: a hello request allocates only what its answer needs, and its head is scanned a line at a time
    status: done
    context: [crates/nvs-server/src/trace.rs, crates/nvs-server/src/heads.rs]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: direct-tool
    stage: 1
    name: "the direct measure builds its commands and reads a cgroup's CPU time as its tests say"
    run: [bun, test, tools/nv/test/bench-compare.test.ts]
    timeout: 900
  - id: direct-smoke
    stage: 1
    name: "every HTTP peer serves every case directly under the one-CPU limit, a few hundred requests each"
    run: [bun, nv, bench-compare, --direct, --smoke]
    timeout: 1800
  - id: body-no-wait
    stage: 2
    name: "a body that arrived with its head reaches the program with no park and one clock reading, a late body still arrives whole, and a request that expects 100-continue is not read early"
    run: [cargo, test, --bin, nvs, --, a_body_that_arrived_with_its_head_reaches_the_program_with_no_park, a_request_with_a_body_reads_the_core_clock_once, a_body_that_arrives_late_is_still_read_whole, a_request_that_expects_continue_is_not_read_before_the_program_asks]
    timeout: 1800
    tests:
      - a_body_that_arrived_with_its_head_reaches_the_program_with_no_park
      - a_request_with_a_body_reads_the_core_clock_once
      - a_body_that_arrives_late_is_still_read_whole
      - a_request_that_expects_continue_is_not_read_before_the_program_asks
  - id: request-counts
    stage: 3
    name: "a served hello request allocates its recorded count and no more, and every allocation has a row that says why"
    run: [cargo, test, --bin, nvs, --, a_served_request_allocates_its_recorded_count, every_allocation_of_a_served_request_has_a_row]
    timeout: 1800
    tests:
      - a_served_request_allocates_its_recorded_count
      - every_allocation_of_a_served_request_has_a_row
  - id: head-scan
    stage: 3
    name: "the head scanner gives the verdict of the byte-at-a-time scanner on every head and every split of it"
    run: [cargo, test, --lib, --, the_line_scanner_agrees_with_the_byte_scanner_on_every_split, the_line_scanner_looks_at_one_byte_at_a_time_only_in_a_framing_line]
    timeout: 1800
    tests:
      - the_line_scanner_agrees_with_the_byte_scanner_on_every_split
      - the_line_scanner_looks_at_one_byte_at_a_time_only_in_a_framing_line
---

# a request's fixed cost is counted, and every part of it is work the answer needs

## Why

`hello` runs no program work, so its cost is what every request pays before the program's first
statement. Measured on 2026-11-17, each server alone under the comparison's one-CPU limit, with no
proxy, CPU time per request read from the container's `cpu.stat`:

| Case | Novis | Go | Bun |
|---|---|---|---|
| `hello` | 21.5 µs | 18.3 µs | 11.4 µs |
| `json-api` | 48.5 µs | 44.9 µs | 20.6 µs |

About half of a `hello` request is kernel TCP work, which all three servers pay. The other half is
Novis's own code, and no function in it is above 2.2 percent of the request: the cost is spread over
the head scan, the isolate's start, the context, the watchdog and the scheduler. Earlier goals removed
the top functions of a profile. That method finds nothing more here, and nothing counted the path, so
nothing held what was gained.

Two facts the profile of 2026-11-17 did show:

- A `POST` whose 1.7 KB body arrives in the same read as its head parks the program twice before it has
  the body: once for the data, once for the end (`crates/nvs-server/src/body.rs:344`, `:378`). Under
  load other requests run in each park, and each switch between two requests reads the core's clock.
  `strace` counts 5.9 clock readings per `json-api` request and 1.0 per `hello` request. The readings
  are about 9 percent of the `json-api` request.
- Every response allocates a 55-character `String` for its `traceparent` header and copies it
  (`crates/nvs-server/src/trace.rs:366`), and every request copies its path into a `Box<str>`
  (`crates/nvs-server/src/serve.rs:1925`).

## Standing decisions

**Nothing that protects a request changes.** The isolate per request, the CPU and memory limits, the
accounting of every allocation, every row of `rule:errors/http-message-defects` and the default
security headers stay exactly as they are. A change this goal makes gives the same answer, the same
refusals and the same charge to the same request. Security and request isolation are not spent here.

**No object that held one request's data is used by another request.** An allocation is removed by
not making it: the value is written in place, borrowed, or built once when the configuration loads.
Nothing is pooled across requests, and no context, isolate or buffer of one request is handed to the
next. A candidate that needs such reuse is declined and its row says so.

**The measure (my call).** Counts decide, as `rule:testing/member-perf-ledger` has it for members:

- allocations per served request, counted in process by a test;
- readings of the core's clock per request, counted by a test-only counter beside
  `crates/nvs-host/src/cpuclock.rs:386`, which is compiled out of every other build;
- system calls per request, from `bun nv bench-compare --profile`, written into `### State`.

`bun nv bench-compare --direct` prints CPU time per request for Novis, Go and Bun side by side. It is
what a person reads to see where Novis stands. It is never a gate, records nothing, and no figure of
Go's or Bun's is a reason for a change.

**The body hand-over (my call).** Two changes in `crates/nvs-server/src/body.rs`, both invisible to a
program:

- When `Supply::pump` takes a data frame and `Incoming::is_end_stream()` is then true, the end is
  delivered with that frame. The last chunk never costs a park of its own.
- Before the program starts, the connection task polls `Incoming` once, with its own `Context`, when
  the head declared a `Content-Length` and sent no `Expect` header. A frame that is ready becomes the
  pull's first chunk. A poll that is pending changes nothing. The poll never waits.

`[limits] request_body` is checked where it is today, over the same bytes. A request with `Expect` is
left alone, because polling the body is what makes `hyper` send `100 Continue`, and the program has not
asked yet. **What it spends:** nothing held. The bytes are the ones the transport already had, kept
as the same `Bytes` and freed when the request ends, read or not.

**The head scanner (my call).** `Scanner::feed` (`crates/nvs-server/src/heads.rs:158`) finds each
line's end with `memchr` and steps byte by byte only through a line that can be `Content-Length` or
`Transfer-Encoding`. A bare LF is still found, from the byte before each LF. The byte-at-a-time step
stays in the file under `#[cfg(test)]` as the reference the new one is compared with.

**Limits: what stops this goal.** Counts decide and never the clock. An item gets one attempt. A
candidate whose removal would change an answer, a refusal or a charge is not built, and its row in the
allocation table says why. Each stage gets two sessions, counted in `### State`; at the second
session's end its open items move to `### Backlog` as declined. No item is added to a stage. No target
figure holds this goal open: it is reached when the counts are held by tests, whatever `--direct`
prints for the other servers.

**Lanes.** Stage 1's tool work touches `tools/nv/` and `benches/compare/` only, and its count tests
touch `crates/nvs-cli/src/` and `crates/nvs-host/src/cpuclock.rs`. Stages 2 and 3 share
`crates/nvs-server/src/serve.rs` and run in order. The goal touches no compiler crate, so it can run as
a side goal beside a compiler goal.

## Stages

### 1. the direct measure, and the counts of one request

**Does:** makes the measurement of 2026-11-17 a command, and puts today's counts under tests so the
later stages lower a number a test holds.

**Split:** one writing subagent writes `--direct` and its tests in `tools/nv/`. The session writes the
Rust count tests. The two touch disjoint files and run side by side.

- [x] `tools/nv/cmd/bench-compare.ts:1903` the options, `:1944` `parseArgs`, `:1978` `help()`,
  `:2114` `main`: `--direct` beside `--profile`. For each of `hello`, `json-api` and `html-page` and
  each peer that listens on HTTP (`nvs`, `go`, `bun` in `PEERS`, `:117`): the backend container alone
  with `--cpus 1`, `oha` in the container's network on other cores, 20,000 requests to warm it, then
  five passes of 10 seconds at 64 connections with the peers taking turns. Each pass reads the
  container's `cpu.stat` before and after. It prints, per peer, the median requests per second, the
  CPU microseconds per request split into user and system, and the times the quota stopped the
  container. PHP is left out, because FPM has no HTTP listener. `--smoke` sends a few hundred requests
  once. It records nothing, as `--profile` does.
- [x] `tools/nv/test/bench-compare.test.ts:1`: tests for the `docker` arguments `--direct` builds and
  for reading two `cpu.stat` texts into microseconds per request.
- [x] `benches/compare/README.md:274`: a section before it, `## The direct measure: every HTTP peer
  with no proxy`, that says what the command measures, why PHP is absent, and that it is never a gate.
  `:295` *Running it* gains the command.
- [x] An `Explore` subagent first finds where a test can serve requests in process and count
  allocations: `TestServer` in `crates/nvs-cli/src/runner.rs`, and the counter in
  `crates/nvs-runtime/src/counting_alloc.rs:71`. The count tests go there. If that place is not the
  `nvs` binary's tests, edit the `body-no-wait` and `request-counts` checks with `looper goal check`.
- [x] `a_served_request_allocates_its_recorded_count`: one keep-alive connection, 50 `hello` requests to
  warm it, then 200 counted. It asserts the allocations per request equal a constant written beside
  it, which is today's count. Run once by name.
- [x] A test-only counter of clock readings beside `crates/nvs-host/src/cpuclock.rs:386`, and
  `a_request_with_a_body_reads_the_core_clock_once` written with today's count as its bound and a
  comment that stage 2 makes it one. Run once by name.
- [x] `bun nv bench-compare --direct` once and `--profile hello` once. Write the three peers' CPU per
  request and Novis's system calls per request into `### State`.

### 2. a request body that has arrived whole reaches the program with no wait

**Does:** the two changes of § *The body hand-over*.

- [x] `crates/nvs-server/src/body.rs:195` `Supply::pump`: the end is delivered with the last data
  frame. `:344` `Pull::next_chunk` then returns the end without a park when the chunk before it carried
  it.
- [x] `crates/nvs-server/src/body.rs:97` `body::of` and its caller in
  `crates/nvs-server/src/serve.rs:2429`: the one early poll, under the two conditions the standing
  decision names. `crates/nvs-stdlib/src/request.rs:3491` `pull_whole` needs no change if the pull
  answers its first chunk and the end from what it holds; confirm it by reading, and change it only
  where it parks without need.
- [x] The four tests the `body-no-wait` check names. The late body is written in two parts by the
  test's client, which waits for the server's read between them and never for time. Run once by name.
- [x] `crates/nvs-host/src/watchdog.rs:133` *How many readings a request makes* is rewritten as a
  whole with the new counts for one connection and for eight.
- [x] `bun nv bench-compare --profile json-api` once. Write the clock readings per request into
  `### State`.

### 3. a hello request allocates only what its answer needs, and its head is scanned a line at a time

**Does:** lists every allocation of one request, removes the ones the answer does not need, and makes
the head scan step through framing lines only.

**Split:** one writing subagent rewrites `Scanner::feed` and writes its two tests in
`crates/nvs-server/src/heads.rs`, which nothing else in this stage touches.

- [x] Print every allocation of one warm `hello` request with its call site, from the stage 1 test. Add
  the call-site trace to the counter under `#[cfg(test)]` if it has none. Write the table into
  `### State`: call site, bytes, kept or removed, and the reason.
- [x] `crates/nvs-server/src/trace.rs:359` `respond`: the `traceparent` value is written into a buffer
  on the stack and becomes the header value with one allocation or none.
  `crates/nvs-runtime/src/trace_context.rs:232` keeps `traceparent()` for the outbound client.
- [x] `crates/nvs-server/src/serve.rs:1925`: the path is borrowed from the request where every reader
  of it lives no longer than the request. Where one outlives it, the copy stays and its row says so.
- [x] Every other row marked removed in the table, in the table's order. A row is removed only under
  § *No object that held one request's data is used by another request*.
- [x] `every_allocation_of_a_served_request_has_a_row`: the test holds the table as data, and fails
  naming a call site that allocates with no row. The constant of
  `a_served_request_allocates_its_recorded_count` becomes the number of kept rows. Run both once by
  name.
- [x] `crates/nvs-server/src/heads.rs:158` `Scanner::feed` and `:174` `step`, as § *The head scanner*
  says, with the two tests the `head-scan` check names. The first feeds every head of the file's
  existing tests to both scanners at every split point. The second counts the bytes stepped one at a
  time. Run once by name.
- [x] `bun nv bench-compare --direct` once. Write the three peers' CPU per request into `### State`
  beside stage 1's.

### 4. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done. `looper verify`, `bun nv verify --doc`, `bun nv owners --closes` and
`bun nv ci --check` are green; the last was red only on `recompile-leaks` having no roadmap theme,
which it now has. Each standing decision has its home: the `body.rs` and `heads.rs` module docs,
`benches/compare/README.md`, `cpuclock.rs` and `HELLO_ROWS`. The allocation table is `HELLO_ROWS` in `crates/nvs-cli/src/serve.rs`: 25 kept, 5 removed, each
with its reason. `every_allocation_of_a_served_request_has_a_row` reads backtraces through the
debug-only `nvs_runtime::budget::watch_allocations` hook, and failed with a kept row marked removed.
The largest kept rows are hyper's: 8,192 B read ahead for a client that hangs up (only `half_close`
avoids it, which changes how a lost client is noticed), and the path copy that keeps that buffer
unshared. Debug counts on Windows: **25** allocations per `hello` (Linux and macOS unrun), **1**
clock reading and **0** parks per request with a body.

`--direct` at stage 3's end, CPU µs per request (user + system) and requests/s; stage 1 measured
Novis at 15.0, 40.2 and 58.1:

| case | Novis | Bun | Go |
|---|---|---|---|
| `hello` | 14.8 (6.0 + 8.8), 69,274/s | 9.5 (2.1 + 7.4), 106,560/s | 16.6 (5.8 + 10.8), 66,766/s |
| `json-api` | 36.8 (27.0 + 9.8), 28,572/s | 17.8 (8.0 + 9.8), 57,886/s | 42.3 (26.2 + 16.1), 26,085/s |
| `html-page` | 55.5 (44.5 + 11.0), 18,667/s | 34.1 (21.5 + 12.6), 30,817/s | 275.6 (236.7 + 39.0), 4,056/s |
### Next

Nothing: the goal is reached, and the driver retires it. CI may go red on other runners: `every_allocation_of_a_served_request_has_a_row` on Linux or macOS if a frame name differs
there (its sites are platform-neutral names, and a generic dependency function is named without its
path); `a_served_request_allocates_its_recorded_count` if a platform counts other than 25, in which
case the rows, not the constant, are per platform. `nvs-host`'s `take_ended` now returns a `Drain`.

### Backlog

- `start_as_task`'s end flag (`crates/nvs-host/src/isolate.rs:1165`) could live in the completion cell,
  one allocation fewer. Declined for scope in stage 3.
- `nvs_runtime::script::Program` is a `Box<dyn Fn>`, so each request boxes a closure over the shared
  unit (8 B). An `Rc` program would share code only, but changes every caller of the type.
- The three `--direct` backends of a case share core 1, and two are idle while the third is measured.
  A fresh backend per pass would keep a busy host from moving a figure.
