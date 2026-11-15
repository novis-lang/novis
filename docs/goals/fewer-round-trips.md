---
looper: 1
title: "a request waits on no round trip, hand-off or copy it can avoid"
summary: "Reached when every socket sends without Nagle's delay, a database row shares its keys, a pool reset is one write, a static file is one hand-off, and an extension instance resolves its imports once."
context: [crates/nvs-host/src/net.rs, crates/nvs-stdlib/src/db/execute.rs, crates/nvs-db/src/mysql.rs, crates/nvs-server/src/statics.rs, crates/nvs-ext/src/call.rs, docs/rules/programs/memory-priority.md]
stages:
  - id: 1
    title: a socket sends at once, and a client request is one write
    status: done
    context: [crates/nvs-host/src/net.rs, crates/nvs-stdlib/src/http/transport.rs, crates/nvs-stdlib/src/bus.rs, crates/nvs-stdlib/src/sse.rs]
  - id: 2
    title: "a result's rows share their keys, in one row loop"
    status: done
    context: [crates/nvs-stdlib/src/db/execute.rs, crates/nvs-stdlib/src/db/stream.rs, crates/nvs-stdlib/src/db/span.rs]
  - id: 3
    title: a pool reset and a batch wait for fewer answers
    status: done
    context: [crates/nvs-db/src/mysql.rs, crates/nvs-db/src/sqlite.rs, crates/nvs-cli/src/worker.rs, docs/rules/security.md]
  - id: 4
    title: the door and the request members copy each value once
    status: open
    context: [crates/nvs-server/src/spooled.rs, crates/nvs-server/src/mount.rs, crates/nvs-server/src/serve.rs, crates/nvs-stdlib/src/uri.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-stdlib/src/html.rs, crates/nvs-stdlib/src/ratelimit.rs]
  - id: 5
    title: a static file is one hand-off
    status: open
    context: [crates/nvs-server/src/mount.rs, crates/nvs-server/src/statics.rs, crates/nvs-server/src/serve.rs, docs/rules/http-server.md]
  - id: 6
    title: an extension instance resolves its imports once
    status: open
    context: [crates/nvs-ext/src/call.rs, crates/nvs-ext/src/load.rs, crates/nvs-cli/src/extensions.rs, crates/nvs-cli/src/cache.rs]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: nodelay-set
    stage: 1
    name: "an accepted and a connected TCP stream both have TCP_NODELAY set"
    run: [cargo, test, --lib, --, an_accepted_and_a_connected_stream_send_without_delay]
    timeout: 1800
    tests:
      - an_accepted_and_a_connected_stream_send_without_delay
  - id: rows-share-keys
    stage: 2
    name: "a result of many rows allocates its column keys once"
    run: [cargo, test, --lib, --, a_results_rows_share_one_set_of_keys]
    timeout: 1800
    tests:
      - a_results_rows_share_one_set_of_keys
  - id: static-one-hand-off
    stage: 5
    name: "a small static file is one blocking job, and a 304 opens no file"
    run: [cargo, test, --lib, --bin, nvs, --, a_small_static_file_is_one_blocking_job, a_not_modified_answer_opens_no_file]
    timeout: 1800
    tests:
      - a_small_static_file_is_one_blocking_job
      - a_not_modified_answer_opens_no_file
---

# a request waits on no round trip, hand-off or copy it can avoid

## Why

Each cost below was read in the code. None was timed.

- **No socket sets `TCP_NODELAY`.** A search of `crates/` finds it only in the debug listener. The
  server's accepted streams (`crates/nvs-host/src/net.rs:617`) and every client connection (`:399`)
  leave Nagle's algorithm on. The HTTP client also writes a request's head and body as two writes
  (`crates/nvs-stdlib/src/http/transport.rs:2405`). A small second write waits for the peer's
  delayed acknowledgement, about 40 ms on Linux. Loopback does not show it.
- **Every cell of every row allocates its column name.** All four drivers call
  `NvsStr::new(column.name)` per cell (`crates/nvs-stdlib/src/db/execute.rs:643`, `:933`, `:1028`,
  `:1245`), and `stream.rs` repeats it (`crates/nvs-stdlib/src/db/stream.rs:266`).
- **A MySQL checkout is four round trips.** `reset_session` sends four commands and reads each
  answer before the next (`crates/nvs-db/src/mysql.rs:2243`). Postgres sends its reset in one write.
- **A static file is three thread-pool hand-offs,** and a 304 opens the file only to read what the
  first `stat` already had (`crates/nvs-server/src/mount.rs:120`,
  `crates/nvs-server/src/statics.rs:156`, `crates/nvs-server/src/serve.rs:507`).
- **Each extension instance resolves every import by name.** `instantiate_async` runs per request
  per extension (`crates/nvs-ext/src/call.rs:1409`), and no `InstancePre` exists outside the tests.
- **The door clones the shared snapshot for a debug store that is off**
  (`crates/nvs-server/src/spooled.rs:200`), two atomic writes on a line every core shares.
- **A query field is copied two to four times** (`crates/nvs-stdlib/src/uri.rs:1304`), and
  `#[Query]` binding decodes the whole string once per parameter
  (`crates/nvs-stdlib/src/router.rs:3068`).

One finding is about safety and not speed. `Core\RateLimit::shed` stores an entry with no lifetime
(`crates/nvs-stdlib/src/ratelimit.rs:869`), and the memory tier evicts in first-write order
(`crates/nvs-stdlib/src/cache.rs:896`). A flood of new keys evicts the oldest live limiter first.

## Standing decisions

**Limits: what stops this goal.**

- **A count pins each change:** allocations per row, blocking jobs per request, writes per reset.
  The checks use the debug-only counters of `rule:testing/perf-two-mechanisms`. No load test and no
  bench run decides anything here.
- **A reset is never joined to a statement.** Only the reset's own commands share a write, and every
  answer is read before the connection is handed out
  (`rule:security/db-pool-reset-is-a-boundary`). An item that cannot keep that is declined.
- **Anything that needs a live database is tried once.** Stage 3's MySQL items run their driver
  tests once with `bun nv db-matrix`. If the server is not reachable in that session, the item is
  built, checked by build alone, and named in the handoff for CI to prove.
- **An item gets one attempt, and a stage two sessions,** counted in `### State`. At the ceiling the
  open items move to `### Backlog` as declined.
- **No item is added.**

**`TCP_NODELAY` is set on every TCP stream, with no setting.** Every other server and client does
this, and a setting would be one more thing to get wrong. **What it spends:** a program that writes
many small pieces sends more segments. The HTTP paths write whole messages, so they do not.

**A client body of 16 KiB or less joins the head's buffer.** Larger bodies keep two writes.
**What it spends:** one copy of a small body.

**An idle event stream parks for one second, not 50 ms.** A publish from any core wakes it through
the bus, so the bound only decides how soon a parked stream sees that its client left: nothing
wakes a writer on that close. **What it spends:** up to one second of a closed stream's isolate.
`CLOSE_TICK` in `crates/nvs-stdlib/src/sse.rs` holds it.

**A failed `set_nodelay` is ignored.** A socket that refuses the option is one its peer already
reset, and its next read or write reports that. Failing the accept for it would read as a listener
failure.

**No name-lookup cache.** The shared store and the HTTP client resolve the host on each call. A
cached answer changes when a DNS change is seen. Declined until a deployment measures it.

**A result's keys are built once and retained per row.** Strings are counted by reference, so a row
that is written to separates as today. The four row loops become one generic function in the same
change, with one closure per driver for the cell.

**A limiter entry lives as long as its window.** After its window an entry answers as an absent
one, so the sweep may take it. The fixed test timeline keeps `Forever`. **What it spends:** nothing.
It makes a key flood evict dead entries before live ones.

**`InstancePre` is held on the extension, made on its first instance.** A store and an instance are
still made per request, so nothing is shared between requests
(`rule:security/no-cross-request-state`). **What it spends:** one pre-resolved import table per
extension per process.

**The epoch thread parks while no instance is live.** Safe default if a test shows a guest started
from idle missing its CPU deadline: the thread ticks as today.

**The artifact keeps one checksum.** The outer BLAKE3 covers the payload, so the inner SHA-256 goes
(`rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`). The format version
moves, so an old entry is a miss.

**Lanes:** stages 2 and 3 touch the database code and run in order. Stages 4 and 5 share
`crates/nvs-server/src/` and run in order. Stage 6 touches only `crates/nvs-ext/` and
`crates/nvs-cli/src/extensions.rs` and can run as a side goal.

## Stages

### 1. a socket sends at once, and a client request is one write

**Does:** set `TCP_NODELAY` where a TCP stream is made, join a small client body to its head, and
remove two wake-up costs of the bus.

- [x] `crates/nvs-host/src/net.rs:399`: `set_nodelay(true)` in `from_std`, the connect paths at
  `:425`, `:449` and `:496`, and on a stream the acceptor at `:617` returns
  (`rule:programs/memory-priority`).
- [x] `crates/nvs-stdlib/src/http/transport.rs:2405`: a held body of 16 KiB or less is appended to
  the head and written once (`rule:programs/memory-priority`).
- [x] `crates/nvs-stdlib/src/bus.rs:263`: `note_subscribers` takes the read lock first and the write
  lock only when the count changed (`rule:programs/memory-priority`).
- [x] `crates/nvs-stdlib/src/sse.rs:803`: an idle `receive` parks on the bus's hand-off wake at
  `crates/nvs-stdlib/src/bus.rs:388`, as `crates/nvs-stdlib/src/socket.rs:1260` does
  (`rule:concurrency/the-parking-contract`).
- [x] `crates/nvs-host/src/net.rs:399`: the test
  `an_accepted_and_a_connected_stream_send_without_delay` (`rule:testing/perf-two-mechanisms`).

### 2. a result's rows share their keys, in one row loop

**Does:** build a result's keys once, and write the row loop once for all four drivers.

- [x] `crates/nvs-stdlib/src/db/execute.rs:605`: one generic row function, with keys built before
  the loop and `NvsArray::with_capacity` per row. The loops at `:853`, `:968` and `:1194` call it
  (`rule:programs/memory-priority`).
  - [x] `RowKeys` and `collected_rows` exist.
  - [x] The four `*_rows` call `collected_rows` with one cell closure each.
- [x] `crates/nvs-stdlib/src/db/stream.rs:266`: the stream readers at `:336`, `:387` and `:448` use
  the same keys (`rule:programs/memory-priority`).
  - [x] A sixth slot, `keys`, keeps the walk's keys.
  - [x] The four steps build their row with `RowKeys::row`.
- [x] `crates/nvs-stdlib/src/db/span.rs:169`: `slow_query` is parsed once per snapshot
  (`rule:config/the-config-is-an-immutable-snapshot`).
  - [x] The resolved limits hold each block's threshold, read by `Snapshot::slow_query`.
- [x] `crates/nvs-stdlib/src/db/execute.rs:605`: the test `a_results_rows_share_one_set_of_keys`,
  on SQLite (`rule:testing/perf-two-mechanisms`).
  - [x] The test is written and green, run once by name.

### 3. a pool reset and a batch wait for fewer answers

**Does:** send the MySQL reset in one write, prepare SQLite's reset once, and stop a busy queue
worker from asking the roster between jobs.

- [x] `crates/nvs-db/src/mysql.rs:2243`: the four reset packets go out in one flush and their four
  answers are read in order (`rule:security/db-pool-reset-is-a-boundary`).
- [x] `crates/nvs-db/src/sqlite.rs:1709`: `clean` uses `prepare_cached`, and the pragma text is
  built once (`rule:security/db-pool-reset-is-a-boundary`).
- [x] `crates/nvs-cli/src/worker.rs:698`: after a round that claimed a job, the worker claims again
  on the same queues and asks the roster when a claim is empty
  (`rule:programs/memory-priority`).
- [x] `crates/nvs-db/src/mysql.rs:2795`: `execute_many` writes its executes in windows of 64 KiB
  and reads each window's answers (`rule:programs/memory-priority`). This item has one session. If
  it is not green, it is declined.

### 4. the door and the request members copy each value once

**Does:** remove the copies and repeated parses a request makes before and inside its handler.

- [ ] `crates/nvs-server/src/spooled.rs:200`: `door_store` tests a `debug_store` flag on the
  snapshot at `crates/nvs-config/src/snapshot.rs:147` before it builds a `Request`
  (`rule:config/the-config-is-an-immutable-snapshot`).
- [ ] `crates/nvs-server/src/mount.rs:445`: `under()` is called once for the static and the script
  step (`rule:http-server/static-serving-is-one-policy`).
- [ ] `crates/nvs-server/src/serve.rs:2210`: the two stream slots are made in the arm that runs a
  program (`rule:programs/memory-priority`).
- [ ] `crates/nvs-stdlib/src/uri.rs:1304`: `decode` borrows a field with no `%` or `+`, and
  `parse_query` at `:3303` builds each value once (`rule:errors/ambiguous-input-refused`).
- [ ] `crates/nvs-stdlib/src/router.rs:3068`: the repeated keys are counted in one pass
  (`rule:errors/ambiguous-input-refused`).
- [ ] `crates/nvs-stdlib/src/html.rs:868`: `markup_concat`, `join` at `:938` and `escaped_text` at
  `:618` build the result string directly, and `join` asks the budget for one page
  (`rule:programs/memory-priority`).
- [ ] `crates/nvs-stdlib/src/ratelimit.rs:869`: the entry's lifetime is its window
  (`rule:programs/memory-priority`).

### 5. a static file is one hand-off

**Does:** look a static file up, open it and read its first chunk in one blocking job, and answer a
304 from the `stat`.

- [ ] `crates/nvs-server/src/statics.rs:156`: lookup, open and the first chunk are one job when the
  request has no range and no condition (`rule:http-server/a-core-is-never-blocked-on-a-syscall`).
- [ ] `crates/nvs-server/src/statics.rs:180`: a not-modified answer is made from the first `stat`
  (`rule:http-server/static-serving-is-one-policy`).
- [ ] `crates/nvs-server/src/statics.rs:344`: the media type is looked up once, without an
  allocation (`rule:programs/memory-priority`).
- [ ] `crates/nvs-server/src/statics.rs:1`: the two tests of the check `static-one-hand-off`, and
  the module doc's cost section says one hand-off (`rule:testing/perf-two-mechanisms`).

### 6. an extension instance resolves its imports once

**Does:** keep an `InstancePre` per extension, check an artifact once, and let the epoch thread
sleep.

- [ ] `crates/nvs-ext/src/call.rs:1409`: `instantiate_pre` once per extension, and the export index
  at `:1419` kept beside it (`rule:security/no-cross-request-state`).
- [ ] `crates/nvs-ext/src/load.rs:366`: the inner SHA-256 goes, and `Modules::get` at
  `crates/nvs-cli/src/extensions.rs:1431` borrows the verified payload
  (`rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`).
- [ ] `crates/nvs-ext/src/call.rs:995`: the ticker parks while no instance is live, with a test that
  a guest started from idle is stopped at its CPU deadline (`rule:errors/on-limit`).
- [ ] `crates/nvs-cli/src/extensions.rs:301`: the pool is sized from a count, not from a clone of
  every manifest (`rule:programs/memory-priority`).

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1, 2 and 3 are done. Each stage 3 item was checked by clippy and its new tests, run once by
name (green). `bun nv db-matrix --no-up --driver mysql --driver mariadb` passed the MySQL and
MariaDB TCP legs in both stage 3 sessions. Its socket legs fail on Windows (`/mnt/wsl` paths are
not supported), and no stage 3 change touches a socket path.
Sessions used: stage 1: 1, stage 2: 1, stage 3: 2, stage 4: 0, stage 5: 0, stage 6: 0.

- `execute_many` (`crates/nvs-db/src/mysql.rs`) prepares once, then writes `execute_payload`s in
  windows of `BATCH_WINDOW` (64 KiB) through `Wire::send_all`, and reads each set's answer with
  `read_set`. `read_set` counts a result set's rows without decoding them, and follows
  `SERVER_MORE_RESULTS_EXISTS` to a `CALL`'s last status packet. A set whose parameters do not fit
  a packet now ends the batch with the connection idle; before, it poisoned it.
  `execute_one` is gone. `rule:core-classes/db-statement-members` says the batch is windowed.
- The sweep's `env-secret-proofs` is stale state in looper's service: no log and no definition in
  the tree. Nothing in the tree can fix it.

Tests that may go red at stage 7: an SSE case that waits less than one second for a stream whose
client left to end, an HTTP client test that counts writes on the origin side for a small body,
and any test that builds `Core\Db\Stream` with five slots (`instance::build` asserts the count).
From stage 3: a MySQL test whose scripted server answers a flush rather than each packet in it, a
worker test that counts turns or roster statements for a queue with several jobs, a test that
expects a too-large `executeMany` set to poison the connection, and a test that expects a `CALL`
in `executeMany` to fail with "a second result set".

### Next

Stage 4, in its first session. Its items share `crates/nvs-server/src/` and `crates/nvs-stdlib/src/`;
take them in order while the context allows. The stage checks with a build alone.

- [ ] `crates/nvs-server/src/spooled.rs:200`: `door_store` tests the `debug_store` flag on the
  snapshot (`crates/nvs-config/src/snapshot.rs:147`) before it builds a `Request`.
- [ ] `crates/nvs-server/src/mount.rs:811`: `under()` is called once for the static and the
  script step. Find its two callers with `bun nv peek --locate`.
- [ ] `crates/nvs-server/src/serve.rs:2210`: the two stream slots are made in the arm that runs a
  program.
- [ ] `crates/nvs-stdlib/src/uri.rs:1304`: `decode` borrows a field with no `%` or `+`, and the query
  parser near `:3303` builds each value once.
- [ ] `crates/nvs-stdlib/src/router.rs:3068`, `crates/nvs-stdlib/src/html.rs:868` (`markup_concat`,
  `join`, `escaped_text` at `:618`) and `crates/nvs-stdlib/src/ratelimit.rs:869`, as the stage
  lists them.

### Backlog

- Declined before the run: joining a pool reset to the first statement. A failed reset would run
  the statement under the last request's role.
- Declined before the run: a precompiled artifact of the built-in components inside the binary. It
  is several sessions, and the on-disk cache already serves every start after the first.
- Declined before the run: keeping an SMTP session open between messages, and moving the SQLite
  session store off the core thread. Neither has a measured program behind it.
- To check, outside this goal: `normalized` at `crates/nvs-stdlib/src/uri.rs:1923` pushes each byte
  as a `char`. If `parse` lets a byte of 0x80 or above reach it, the output is wrong.
