---
looper: 1
title: "a request writes no cache line that other cores share, and blocks its core on no file call"
summary: "Reached when the shared writes of one request are counted and guarded, a request reads its configuration, policy and compiled unit from its own core's record where the count confirms the cost, the inbound head is one buffer, static files are read off the core thread, and no setting is parsed from text per request."
context: [crates/nvs-server/src/serve.rs, crates/nvs-cli/src/serve.rs, docs/agent/coding.md, docs/rules/programs/memory-priority.md]
stages:
  - id: 1
    title: the shared writes of one request are counted and guarded
    status: done
    context: [crates/nvs-host/src/counts.rs, crates/nvs-diagnostics/src/work.rs, crates/nvs-config/src/snapshot.rs, crates/nvs-cli/src/script.rs, docs/rules/testing/perf-two-mechanisms.md]
  - id: 2
    title: "the door reads the publish and the policy from its core's record"
    status: done
    context: [crates/nvs-config/src/snapshot.rs, crates/nvs-config/src/limits.rs, crates/nvs-server/src/secure.rs, crates/nvs-cli/src/serve/mounts.rs, docs/rules/config/the-config-is-an-immutable-snapshot.md, docs/rules/security/no-cross-request-state.md]
  - id: 3
    title: "the handler reads its mount's snapshot, folder and unit from its core's record"
    status: done
    context: [crates/nvs-cli/src/serve/mounts.rs, crates/nvs-cli/src/script.rs, docs/rules/config/a-request-keeps-the-unit-it-resolved.md, docs/rules/config/an-edit-reaches-the-next-request-without-a-restart.md]
  - id: 4
    title: "the request head is one buffer, and a dispatching unit keeps one copy of it"
    status: done
    context: [crates/nvs-runtime/src/ctx/inbound.rs, docs/rules/http-server/a-request-head-has-at-most-100-header-lines.md, docs/rules/routing/a-gate-answers-around-every-handler.md]
  - id: 5
    title: "a static file's calls run off the core"
    status: done
    context: [crates/nvs-server/src/mount.rs, crates/nvs-server/src/statics.rs, crates/nvs-host/src/blocking.rs, docs/rules/http-server/a-core-is-never-blocked-on-a-syscall.md, docs/rules/http-server/static-serving-is-one-policy.md]
  - id: 6
    title: "a setting the door reads is parsed once per configuration"
    status: done
    context: [crates/nvs-config/src/limits.rs, crates/nvs-config/src/http.rs, crates/nvs-config/src/app.rs, crates/nvs-runtime/src/ctx/output.rs, crates/nvs-runtime/src/logfile.rs]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: shared-writes-counted
    stage: 1
    name: "a keep-alive request makes the same counted shared writes at any request count, and no more than its bound"
    run: [cargo, test, --lib, --bin, nvs, --, a_keep_alive_request_makes_its_counted_shared_writes]
    timeout: 1800
    tests:
      - a_keep_alive_request_makes_its_counted_shared_writes
  - id: door-record
    stage: 2
    name: "the door takes no lock under the standing configuration, and a reload reaches the next request on the same core"
    run: [cargo, test, --lib, --bin, nvs, --, a_reload_between_two_requests_on_one_core_reaches_the_second, the_door_takes_no_lock_under_the_standing_configuration]
    timeout: 1800
    tests:
      - a_reload_between_two_requests_on_one_core_reaches_the_second
      - the_door_takes_no_lock_under_the_standing_configuration
  - id: unit-slot
    stage: 3
    name: "a mount's entry is read from its core's record, and a swapped unit reaches the next request on the same core"
    run: [cargo, test, --lib, --bin, nvs, --, a_mounts_entry_is_read_from_its_cores_record_without_a_lock, a_unit_swapped_between_two_requests_on_one_core_reaches_the_second]
    timeout: 1800
    tests:
      - a_mounts_entry_is_read_from_its_cores_record_without_a_lock
      - a_unit_swapped_between_two_requests_on_one_core_reaches_the_second
  - id: one-buffer-head
    stage: 4
    name: "a framed head allocates the same for three header lines and for a hundred, and a rerun shares it"
    run: [cargo, test, --lib, --bin, nvs, --, a_framed_head_allocates_the_same_for_three_header_lines_and_for_a_hundred, a_dispatching_units_rerun_shares_the_first_runs_head]
    timeout: 1800
    tests:
      - a_framed_head_allocates_the_same_for_three_header_lines_and_for_a_hundred
      - a_dispatching_units_rerun_shares_the_first_runs_head
  - id: static-off-core
    stage: 5
    name: "every shape of static request is answered with no file call on the thread that serves it"
    run: [cargo, test, --lib, --bin, nvs, --, a_static_request_makes_no_file_call_on_its_core]
    timeout: 1800
    tests:
      - a_static_request_makes_no_file_call_on_its_core
  - id: settings-resolved-once
    stage: 6
    name: "the door's settings are resolved once per snapshot, and a held log file is opened once"
    run: [cargo, test, --lib, --bin, nvs, --, the_door_settings_are_resolved_once_per_snapshot, a_held_log_file_is_opened_once_for_any_number_of_requests]
    timeout: 1800
    tests:
      - the_door_settings_are_resolved_once_per_snapshot
      - a_held_log_file_is_opened_once_for_any_number_of_requests
---

# a request writes no cache line that other cores share, and blocks its core on no file call

## Why

A connection never leaves its core. The door still reads its configuration through process-wide locks
and reference counts. Each one is an atomic write on a cache line every other core writes too. A
trace of one keep-alive `GET` that runs a compiled unit gives these sites:

- **Read locks, two writes each.** `Current` twice: the door (`crates/nvs-server/src/serve.rs:1890`,
  `crates/nvs-config/src/snapshot.rs:727`) and the handler (`crates/nvs-cli/src/serve.rs:1124`,
  `crates/nvs-config/src/snapshot.rs:738`). The policy lock (`crates/nvs-server/src/serve.rs:1132`).
  `Published::resolved` for the folder and, with any `[[app]]` block, for the entry
  (`crates/nvs-config/src/snapshot.rs:618`, `:657`). The compile cache's `paths`, `env` and `units`
  (`crates/nvs-cli/src/script.rs:826`, `:675`, `:1029`).
- **Reference counts, two writes each.** `Arc<Published>`, `Arc<Snapshot>` about seven times
  (`crates/nvs-server/src/serve.rs:1891`, `:2168`, `:2260`, `:2261`), `Arc<Policy>`, the folder path
  three or four times (`crates/nvs-cli/src/serve.rs:1345`, `:1398`), the unit and three parts of it
  (`crates/nvs-cli/src/script.rs:1855`, `:1857`, `:2094`, `:2099`), and three policy header values
  (`crates/nvs-server/src/secure.rs:158`).
- `admit`'s update of `in_flight` and its release (`crates/nvs-server/src/admit.rs:245`).

That is about 55 atomic writes on about 17 shared lines per request. On one core it is cheap. With
many cores each line moves between cores on nearly every request, so the door's cost grows with the
core count. Nothing counts these writes today. The figure is a trace, not a measurement.

Three more costs sit on the same path:

- **The head is copied line by line.** `Inbound` stores `Vec<(Box<str>, Box<[u8]>)>`
  (`crates/nvs-runtime/src/ctx/inbound.rs:210`), and `push_header` boxes each name and each value
  (`:819`). A head of n lines costs 2n + 2 allocations plus the vector's growth. A remote client
  chooses n, up to 100. A unit that calls `Core\Router::dispatch` also keeps `hyper`'s whole `Parts`
  in its `Rerun` closure for the request's life (`crates/nvs-cli/src/serve.rs:1353`).
- **Static serving calls the file system on the core thread.** `OnDisk::file` runs `metadata` and a
  canonical lookup (`crates/nvs-server/src/mount.rs:139`). A path through a link adds `lstat`, two
  canonical lookups and a directory listing per link (`:164`). `OnDisk::open` runs `metadata`, `open`
  and `fstat` (`crates/nvs-server/src/statics.rs:150`), and its own comment says the open blocks the
  connection's thread. Each 64 KiB chunk is a `read_exact` inside `poll_frame`
  (`crates/nvs-server/src/serve.rs:485`). With `[server] static = true` every request pays the first
  `metadata`, dynamic ones too (`crates/nvs-server/src/mount.rs:401`). A slow or cold disk then stalls
  every connection on that core, and a client that asks for ranges of large files chooses when.
  `rule:http-server/a-core-is-never-blocked-on-a-syscall` forbids exactly this.
- **Four settings are parsed from text per request when set.** The CSRF key is base64-decoded into a
  new `Vec` (`crates/nvs-config/src/http.rs:638`). `upload_total` is parsed
  (`crates/nvs-config/src/app.rs:614`). The disconnect keys build a `Vec<String>`
  (`crates/nvs-config/src/app.rs:324`, read at `crates/nvs-server/src/serve.rs:2273`). The log target is
  read again by the first log write of every request (`crates/nvs-runtime/src/ctx/output.rs:613`,
  `:1365`), through the process-wide `OPEN` mutex (`crates/nvs-runtime/src/logfile.rs:99`). With no
  other handle alive, that request opens the log file and closes it again.

None of this is a vulnerability. It is latency every request pays, and two parts of it are reachable by
a remote client: the header count, and the timing of a stalled core.

## Standing decisions

**The count comes first, and stages 2 and 3 are built only if it confirms the trace.** A big change
waits for a measured wall. Stage 1 adds a counter and a guard, and writes the measured number into
this section. Stages 2 and 3 are built when the count is at least 28 shared writes per request, half
of the traced figure. Below 28, stage 1's last item replaces every item of stages 2 and 3 with one
item. That item records the number in the `# What it costs` of `crates/nvs-server/src/serve.rs:79` and
deletes the checks `door-record` and `unit-slot`. The goal then goes on with stage 4. **What it
spends:** one session before any saving lands.

**The counter is debug-only and thread-local, as `nvs_host::counts` is.**
`rule:testing/perf-two-mechanisms` allows no counter on a production path for a test. So the module
exists under `debug_assertions` only, in `nvs-diagnostics` beside `work`, where every counted crate
can reach it. A site names its line and how many writes it makes there. The test reads a delta on the
serving thread. **What it spends:** nothing in a release binary, and one word per named line per
thread in a debug one. Safe default if a counted crate cannot depend on `nvs-diagnostics`: its caller
counts that site. Declined: a cargo feature, which builds every crate above it twice. Declined: a
hardware counter, which is a different tool on each platform.

**Each core keeps its own record of the standing configuration, checked against one generation
counter per request.** `Current` gains an `AtomicU64` that only a publish moves, under its write lock.
A core's record holds the publish, the host's snapshot and the core's own render of the policy. The
door loads the generation once per request. An equal generation means the record is read with no lock
and no shared write. A moved one means the record is replaced before the request goes on. **A reload
reaches every core at its next request, so no request ever runs on a configuration older than the one
current when it started.** That sentence is the isolation condition and the no-restart condition, and
`a_reload_between_two_requests_on_one_core_reaches_the_second` holds it. **What it spends:** one record
per core, O(cores), with one rendered policy of a few hundred bytes, and one atomic load per request.
It costs simplicity: one cache and one generation per publisher. Safe default if a path cannot reach
its core's record: it reads `Current` under the lock as today, and the guard counts it.

**Nothing of a request moves into the record.** The record holds only values that are the same for
every request under that generation. No header, path, cookie, peer address, overlay or body is stored
there, and no map in it is keyed by anything a request supplies
(`rule:security/no-cross-request-state`). A request still takes one `Arc<Snapshot>` of its own for its
context, because a child placed on another core reads it. That reference moves through `ran_under`,
`under` and `set_config`. It is cloned once, not three times.

**The record is built when the core's loop is built, and replaced at the door before the request has
a place.** Nothing on the request path is made on first use (`docs/agent/coding.md`). So
`serve_on_this_core` builds the record before it accepts, beside `mounts::Local::new`
(`crates/nvs-cli/src/serve.rs:1035`). After a reload, the first request a core frames replaces it at
the top of the door, before `admit` and before the request's tree exists. That is the moment
`Serving::policy` and `Local::table` rebuild today (`crates/nvs-server/src/serve.rs:1128`,
`crates/nvs-cli/src/serve/mounts.rs:177`), and decision "a reload reaches every core at its next
request" needs the check there in any case. The publisher does the costly part first: it fills
`limits::resolved` for the host's snapshot and every folded entry before the swap. So the replace
copies references and renders one policy. Declined: a wake from the publisher to every core. A
request can still arrive first, so the door's check would stay and the wake would be a second path.

**The policy's header values are the core's own.** `Secure::fill` clones three values per response,
and a value shared by every core is a shared count. Each core renders its own `Secure` and `Cors`
when it builds its record. `Serving::policy` and its `RwLock` go.

**The handler takes the record as an argument.** The door passes its core's record to the handler, an
`Rc` clone with no atomic write. `request.extensions_mut().insert(published)` and `published_of` go,
which also removes three allocations per request. The door's answer and the program's configuration
still come from one publish, because both read one record. This changes the handler's signature, and
an item updates every handler in the tree.

**A mount row's own entry has a slot on each core, and any other file takes the shared lookup.** The
slot holds the entry's snapshot, the core's own copy of the folder path and the compiled unit. It is
valid for one pair: the publish generation and an epoch the compile cache moves under every write
guard. A reader loads the epoch before it looks, so a slot is never newer in name than in content. A
file `dispatch = "path"` selects below the mount takes the locked lookup as today. Its path comes
from the request, and a per-core table keyed by request paths would grow with what clients ask.
**What it spends:** one slot per mount row per core, O(cores × mounts). The slot is filled when the
core's table is built, from the boot compile's cache, and again by the first request after the epoch
or the generation moved. An entry first compiled on a request compiles there as today.

**The inbound head is one buffer with an offset table, behind one shared handle.** Method, path,
query and every name and value lie back to back in one byte buffer. One vector of offsets is made
with exact capacity, because `hyper` has framed the head before the first copy. `Inbound::framed`
builds it in at most three allocations for any line count. `push_header` stays for builders that do
not know the count, such as tests and the in-process request, and appends to the same buffer.
Offsets are `usize`. The handle is an `Rc`, and a dispatching unit's `Rerun` keeps that handle, not
`hyper`'s `Parts`. An `Rc` and not an `Arc`, because `Inbound` already holds `Rc` cells and never
leaves its thread, so an atomic count would buy nothing. `Parts` is dropped at the door, so the
connection's read buffer is free during the request. **What it spends:** two words per header line,
and no atomic write. Readers keep their signatures. Declined: reusing a head buffer for
the next request, which is cross-request state.

**Static file calls go through `nvs_host::blocking::run`, inside `OnDisk`.** That is the pool
`rule:http-server/a-core-is-never-blocked-on-a-syscall` names, and no new pool is added. `OnDisk` is
the one production implementor of `Existing` and `Source`, so both methods hand their whole body to
the pool as one job. Every caller is covered: mount selection, the default document, a static answer
and `Core\Response::sendFile`. A file body reads each chunk as one job, with the file moved in and
back, and its first job does the seek. **What it spends:** one hand-off per path asked and one per
64 KiB chunk, which the rule prices at microseconds. A request under `static = true` that reaches
step 5 pays one to three hand-offs before its program runs. The pool charges a job's CPU time and
bytes to the task that asked. Safe default if a `RefCell` borrow is found held across the handler
call or `poll_frame`: narrow the borrow, never skip the hand-off.

**The miss path's `stat` leaves the core too.** A `stat` on a network file system has no bound, and
the rule states none. One rule on every platform and every path is simpler than a stated exception.
Declined: keeping the first `metadata` on the core with a bound in the rule. Declined: a per-core
cache of what exists, which would be a second static policy with its own staleness.

**The door's settings live in `limits::Resolved`, parsed once per snapshot.** The cell gains the
decoded CSRF key, `upload_total`, the `Disconnect` pair, the log target with its bounds, and the log
level and format. The key's type prints nothing of itself, because `Resolved` derives `Debug`.
`log.level` and `log.format` are `Runtime` keys, so a request whose overlay holds one, or a mode
flip, reads it through `get`, as `Request::deprecated` does. The other values are `System` keys or
are read off the standing tree, so no overlay holds them. **What it spends:** under a kilobyte more
per snapshot, and nothing per request.

**The publisher holds each configured log file open.** `limits::Resolved` cannot hold a `LogFile`,
because `nvs-config` sits below `nvs-runtime`. So boot and each reload take one `LogFile` per file
target of the snapshots they publish, open it, and keep it until the next publish stands. A request's
`LogFile::with_bounds` then finds the open file. **What it spends:** one descriptor per configured log
file for the configuration's life. A logging request still takes the `OPEN` mutex and the file's own
mutex. Both guard one shared file, and the guard in stage 1 does not count a tree with no log file.
Safe default if the open fails at publish: the failure is ignored and the first write opens the file
as today.

**The `hyper` crate stays.** Replacing it would put new framing code in front of every peer to buy
simplicity, which the priority ordering does not allow. Two changes around it are in this goal: the
one-buffer head, and the workspace manifest's comment that says `Core\Http\Client` is `ureq`'s. No
manifest names `ureq`, and the client is hand-written.

**The measured count.** One keep-alive request to a compiled unit, with no `[[app]]` block and the
shipped header set, makes **48** shared writes in debug, the same over 20 and over 40 requests.
That is at least 28, so stages 2 and 3 are built. Once the door read its core's record, the
`current-lock`, `policy-lock`, `publish` and `policy` lines fell to zero. Once the door borrowed the
host's snapshot and moved the entry's, the `snapshot` line fell by 4. Once the handler read its
row's slot, the `units-lock` and `folder` lines fell to zero and the `unit` line to 8. The guard's
bound is 16: `snapshot` 6, `unit` 8 and `admit` 2 (`crates/nvs-cli/src/serve.rs`'s `SHARED_WRITES`).

**The slot holds a unit only, never a failure.** `Compiler::standing_unit` fills a slot from step 1
alone: the pointer's unit where it compiled, and nothing for a path with no pointer, a compile in
flight or a failed edit. A slot with nothing to run sends its request to the shared lookup, which
compiles or answers the failure as before. A slot that kept a failure could hide the fix of a path
no background check watches, because only a request walks a path with no pointer. **What it
spends:** nothing more for a row whose entry does not compile, which pays the shared lookup as
before. The `Rerun` closure runs the unit and the folder the first run read, through the request's
`Rc<Entry>`, so a rerun takes no lock and runs the same program
(`rule:config/a-request-keeps-the-unit-it-resolved`).

| line | writes | where |
|---|---|---|
| `current-lock` | 2 | `Current::published` |
| `publish-lock` | 2 | `Published::folder`'s read |
| `policy-lock` | 2 | `Serving::policy` |
| `units-lock` | 6 | the compile cache's `paths`, `env` and `units` reads |
| `publish` | 2 | the door's `Arc<Published>` |
| `snapshot` | 10 | the door's host clone, `under` and `set_config`, `Published::entry`'s host clone, `Ctx::isolate`'s copy |
| `policy` | 4 | the `Arc<Policy>`, and the rendered `Content-Security-Policy` value |
| `folder` | 4 | `Published::folder`'s clone, and `in_folder` |
| `unit` | 14 | `reply`'s unit, routes and class table, and `program_over`'s statics, class table, id and query sites |
| `admit` | 2 | the in-flight count |

An entry an `[[app]]` block matches adds one `publish-lock` read and a `ran_under` clone. A
dispatching unit under refusal mode adds the `Rerun` closure's compiler and folder clones.
`Ctx::isolate`'s copies are counted by the door through `Ctx::isolate_copies`, because
`nvs-runtime` does not depend on `nvs-diagnostics`. Every check runs `cargo test --lib --bin nvs`,
because `nvs-cli` is a binary crate and `--lib` alone runs none of its tests.

**Lanes:** one lane, in order. Every stage edits `crates/nvs-server/src/serve.rs` or
`crates/nvs-cli/src/serve.rs`. Stages 4, 5 and 6 do not depend on stages 2 and 3, so they are built
whatever the count decides. Inside stage 5 and stage 6 the `Explore` searches run side by side.

## Stages

### 1. the shared writes of one request are counted and guarded

**Does:** count every shared atomic write the door and the handler make for one request, guard the
number, and decide stages 2 and 3 by it.

- [x] `crates/nvs-diagnostics/src/shared.rs:1`: a debug-only module `shared` in `nvs-diagnostics`.
  It has one thread-local count per named line, a `wrote(line, writes)` call and a `now()`/`since()`
  reading, as `crates/nvs-host/src/counts.rs:27` does it (`rule:testing/perf-two-mechanisms`,
  `crates/nvs-diagnostics/src/lib.rs:37`).
- [x] `crates/nvs-config/src/snapshot.rs:726`: count the lock sites, two writes each
  (`rule:testing/perf-two-mechanisms`). They are `Current::published` and `Current::load` (`:726`,
  `:738`), `Published::folder` and `Published::entry` (`crates/nvs-config/src/snapshot.rs:618`,
  `:657`, `:663`), `Serving::policy` (`crates/nvs-server/src/serve.rs:1132`), and the compile cache's
  `shared` and `exclusive` (`crates/nvs-cli/src/script.rs:2009`, `:2015`).
- [x] `crates/nvs-server/src/serve.rs:1891`: count the reference-count sites, two writes each
  (`rule:testing/perf-two-mechanisms`). They are the door's (`:1891`, `:2168`, `:2260`, `:2261`,
  `:2285`), the policy's (`crates/nvs-server/src/serve.rs:1129`,
  `crates/nvs-server/src/secure.rs:158`, `:167`), the handler's (`crates/nvs-cli/src/serve.rs:1345`,
  `:1398`), the unit's (`crates/nvs-cli/src/script.rs:1855`, `:1857`, `:2094`, `:2099`, `:2101`), the
  publish's (`crates/nvs-config/src/snapshot.rs:619`, `:654`, `:658`), the context's
  (`crates/nvs-runtime/src/ctx/isolate.rs:551`) and `admit` (`crates/nvs-server/src/admit.rs:245`).
- [x] `crates/nvs-cli/src/serve.rs:4940`: test `a_keep_alive_request_makes_its_counted_shared_writes`
  in that file's tests, under `#[cfg(debug_assertions)]`, on the harness of
  `a_waiting_request_holds_its_connection_and_its_isolate_on_the_heap`. One client sends two warm-up
  requests and then 20 and 40 keep-alive requests to a compiled unit. The per-request count is the
  same for both sizes and at most the bound. A failure prints the count per line
  (`rule:testing/perf-two-mechanisms`).
- [x] `docs/goals/request-path-per-core.md:1`: run the test once with `cargo test --bin nvs --
  a_keep_alive_request_makes_its_counted_shared_writes --nocapture`. Set the bound to the number it
  prints. Write the number and the per-line table under **The measured count** in this file
  (`rule:testing/perf-two-mechanisms`).
- [x] `docs/goals/request-path-per-core.md:1`: decide stages 2 and 3 by the number, as the first
  standing decision says. Below 28, replace their items and delete their two checks in the same
  commit (`rule:programs/memory-priority`). The count is 48, so both are built.
- [x] `docs/rules/testing/perf-two-mechanisms.md:1`: `rule:testing/perf-two-mechanisms` names
  `nvs_diagnostics::shared` beside `nvs_host::counts` as a debug-only counter.

### 2. the door reads the publish and the policy from its core's record

**Does:** give `Current` a generation, give each core a record of the standing publish and its own
policy, and make the door and the handler read that record.

**Gate:** built only when stage 1's count is at least 28 (`## Standing decisions`, **The measured
count**).

**Split:** an `Explore` subagent lists every handler passed to `serve_connection` or
`serve_on_this_core`, tests included, as `file.rs:NN`, before the signature changes.

- [x] `Current` gains a generation that both publish paths move under the write lock, through one
  private swap function, and a `generation()` load
  (`rule:config/the-config-is-an-immutable-snapshot`, `crates/nvs-config/src/snapshot.rs:705`,
  `:787`, `:817`).
- [x] The publisher fills the limits cell of the host's snapshot and of every folded entry before
  the swap. `Current::new`, `Current::of` and the slow path of `Published::entry` fill it too
  (`rule:config/the-config-is-an-immutable-snapshot`, `crates/nvs-config/src/limits.rs:147`,
  `crates/nvs-config/src/snapshot.rs:710`, `:716`, `:666`). The module doc's *Filled on first read*
  paragraph follows (`crates/nvs-config/src/limits.rs:17`).
- [x] `nvs-server`: a per-core record with the generation, the publish, the host's snapshot and the
  core's own `Policy`. `serve_on_this_core` builds it before it accepts
  (`rule:security/no-cross-request-state`, `crates/nvs-server/src/serve.rs:3406`).
- [x] The door reads the record with one generation load, and replaces it before `admit` when the
  generation moved (`rule:config/the-config-is-an-immutable-snapshot`,
  `crates/nvs-server/src/serve.rs:1890`, `:1984`).
- [x] `Serving::policy` and its lock go, and the module doc's `# What it costs` says what a request
  reads now (`rule:programs/memory-priority`, `crates/nvs-server/src/serve.rs:1128`, `:79`).
- [x] The handler takes the record as its third argument. The extension insert and `published_of` go.
  Every handler the list names is updated (`rule:config/the-config-is-an-immutable-snapshot`,
  `crates/nvs-server/src/serve.rs:1653`, `:2113`, `:724`).
- [x] The `nvs serve` handler reads the host's snapshot and the publish from the record
  (`rule:config/the-config-is-an-immutable-snapshot`, `crates/nvs-cli/src/serve.rs:1124`, `:1184`,
  `:1188`).
- [x] The request's one snapshot reference moves through `chosen`, `ran_under`, `under` and
  `set_config` (`rule:security/no-cross-request-state`, `crates/nvs-server/src/serve.rs:2168`,
  `:2260`).
- [x] Test `a_reload_between_two_requests_on_one_core_reaches_the_second` in
  `crates/nvs-server/src/serve.rs` tests: one keep-alive connection, a publish between its two
  requests, and the second answer carries the new policy header
  (`rule:config/the-config-is-an-immutable-snapshot`, `crates/nvs-server/src/serve.rs:10679`).
- [x] Test `the_door_takes_no_lock_under_the_standing_configuration` beside it, under
  `#[cfg(debug_assertions)]`: after one warm-up request, 20 requests count zero on the `Current` and
  policy lines (`rule:testing/perf-two-mechanisms`, `crates/nvs-server/src/serve.rs:10679`).
- [x] `rule:config/the-config-is-an-immutable-snapshot` gains the per-core record, the sentence on
  reloads and its cost line (`docs/rules/config/the-config-is-an-immutable-snapshot.md:1`). The doc
  of `Current` follows (`crates/nvs-config/src/snapshot.rs:696`).

### 3. the handler reads its mount's snapshot, folder and unit from its core's record

**Does:** give the compile cache an epoch and each core a slot per mount row, so a request for a
row's own entry takes no lock.

**Gate:** built only when stage 1's count is at least 28, and after stage 2.

**Split:** an `Explore` subagent lists every call of `exclusive(` in `crates/nvs-cli/src/script.rs`
as `script.rs:NN` and the map it writes, before the epoch lands.

- [x] `Compiler` gains an epoch that every write guard moves before it is released. `exclusive`
  becomes the one function that does it (`rule:config/a-request-keeps-the-unit-it-resolved`,
  `crates/nvs-cli/src/script.rs:492`, `:2015`).
- [x] `Compiler` answers a lookup with the unit itself, so a slot can keep it and build a `Program`
  with one reference (`rule:config/a-request-keeps-the-unit-it-resolved`,
  `crates/nvs-cli/src/script.rs:809`, `:1849`, `:2084`).
- [x] `Built` gains one slot per row: the entry's snapshot, the core's own copy of the folder and the
  unit, with the generation and epoch it was read under. `Local::new` and each rebuild fill every
  slot (`rule:config/an-edit-reaches-the-next-request-without-a-restart`,
  `crates/nvs-cli/src/serve/mounts.rs:133`, `:158`, `:177`).
- [x] The handler reads the slot when the selected file is the row's entry, and refreshes it when
  the epoch or the generation moved. Any other file takes the shared lookup
  (`rule:config/a-request-keeps-the-unit-it-resolved`, `crates/nvs-cli/src/serve.rs:1184`, `:1194`).
- [x] The `Rerun` closure and the isolate take the folder from the slot
  (`rule:routing/a-gate-answers-around-every-handler`, `crates/nvs-cli/src/serve.rs:1345`, `:1398`).
- [x] Test `a_mounts_entry_is_read_from_its_cores_record_without_a_lock` in
  `crates/nvs-cli/src/serve.rs` tests, under `#[cfg(debug_assertions)]`: after a warm-up, 20 requests
  count zero on every lock line (`rule:testing/perf-two-mechanisms`,
  `crates/nvs-cli/src/serve.rs:4940`).
- [x] Test `a_unit_swapped_between_two_requests_on_one_core_reaches_the_second` beside it: the entry
  is rewritten and revalidated between two requests on one connection, and the second answer is the
  new program's (`rule:config/an-edit-reaches-the-next-request-without-a-restart`,
  `crates/nvs-cli/src/serve.rs:4940`).
- [x] Lower the bound of `a_keep_alive_request_makes_its_counted_shared_writes` to what remains, and
  name each remaining line in its doc comment (`rule:testing/perf-two-mechanisms`,
  `crates/nvs-cli/src/serve.rs:4940`).
- [x] The module doc of `mounts.rs` and the `Compiler` field docs say what a request reads now and
  what a slot spends (`rule:programs/memory-priority`, `crates/nvs-cli/src/serve/mounts.rs:5`,
  `crates/nvs-cli/src/script.rs:533`).

### 4. the request head is one buffer, and a dispatching unit keeps one copy of it

**Does:** store the head as one buffer and one offset table behind a shared handle, and make the
rerun keep that handle.

- [x] `Inbound` stores its method, path, query and header lines in one shared head. `Inbound::framed`
  builds it with exact capacity, and `push_header` appends to it
  (`rule:http-server/a-request-head-has-at-most-100-header-lines`,
  `crates/nvs-runtime/src/ctx/inbound.rs:210`, `:626`, `:818`).
- [x] `method`, `path`, `query`, `headers` and `cookie` keep their signatures over the new storage
  (`rule:http-server/a-request-head-has-at-most-100-header-lines`,
  `crates/nvs-runtime/src/ctx/inbound.rs:801`, `:826`, `:849`).
- [x] `carrier` builds the head with `Inbound::framed`
  (`rule:http-server/a-request-head-has-at-most-100-header-lines`,
  `crates/nvs-cli/src/serve.rs:2548`, `:1237`).
- [x] The `Rerun` closure keeps the first carrier's head handle and builds its carrier from it.
  `hyper`'s `Parts` is dropped once the body is split (`rule:routing/a-gate-answers-around-every-handler`,
  `crates/nvs-cli/src/serve.rs:1314`, `:1343`, `:1361`). The doc of `Rerun` follows
  (`crates/nvs-server/src/serve.rs:665`).
- [x] Test `a_framed_head_allocates_the_same_for_three_header_lines_and_for_a_hundred` in
  `crates/nvs-runtime/src/ctx/inbound.rs` tests: `nvs_runtime::budget::allocations` around
  `Inbound::framed` reads the same for both sizes and at most three
  (`rule:testing/perf-two-mechanisms`, `crates/nvs-runtime/src/ctx/inbound.rs:1250`).
- [x] Test `a_dispatching_units_rerun_shares_the_first_runs_head` in `crates/nvs-cli/src/serve.rs`
  tests: the rerun's carrier reads the same lines from the same buffer as the first run's
  (`rule:routing/a-gate-answers-around-every-handler`, `crates/nvs-cli/src/serve.rs:4940`).
- [x] The workspace manifest's comment says `Core\Http\Client` is hand-written
  (`rule:packaging/a-c-dependency-answers-two-questions`, `Cargo.toml:177`).
- [x] `rule:http-server/a-request-head-has-at-most-100-header-lines` gains what a head spends: its
  bytes once and two words per line
  (`docs/rules/http-server/a-request-head-has-at-most-100-header-lines.md:1`).

### 5. a static file's calls run off the core

**Does:** hand every file call of `OnDisk` and every chunk read of a file body to the blocking pool.

**Split:** an `Explore` subagent lists every `std::fs` call, `File` method and canonical lookup in
`crates/nvs-server/src` outside tests, and every one `Core\Response::sendFile` reaches in
`crates/nvs-stdlib/src/response.rs`, as `file.rs:NN` and the thread it runs on.

- [x] `OnDisk::file` runs its whole body, `spelled_through_links` included, as one
  `nvs_host::blocking::run` job (`rule:http-server/a-core-is-never-blocked-on-a-syscall`,
  `crates/nvs-server/src/mount.rs:139`, `:164`).
- [x] `OnDisk::open` runs `metadata`, `open` and `fstat` as one job, and its comment follows
  (`rule:http-server/a-core-is-never-blocked-on-a-syscall`, `crates/nvs-server/src/statics.rs:150`).
- [x] A file body reads each chunk as one job, with the file moved in and back, and the first job
  seeks (`rule:http-server/a-core-is-never-blocked-on-a-syscall`,
  `crates/nvs-server/src/serve.rs:336`, `:485`).
- [x] No `RefCell` borrow is held across the handler call or the body's poll
  (`rule:concurrency/one-future-per-connection`, `crates/nvs-server/src/serve.rs:2114`, `:2686`).
- [x] Each call the list names on a core thread, and not through `OnDisk`, is routed through the pool
  or written under `### Backlog` with its reason
  (`rule:http-server/a-core-is-never-blocked-on-a-syscall`, `crates/nvs-server/src/mount.rs:139`).
- [x] Test `a_static_request_makes_no_file_call_on_its_core` in `crates/nvs-server/src/serve.rs`
  tests. A `cfg(test)` thread-local counts each raw file call on the thread that makes it. One core
  serves 20 keep-alive requests over a scratch folder: a hit, a miss that reaches the entry, a path
  through a link, a default document, a range of a file of three chunks and a `sendFile`. Every
  answer is right, the serving thread counts zero, and `nvs_host::blocking::pool_size` shows a thread
  (`rule:http-server/a-core-is-never-blocked-on-a-syscall`, `crates/nvs-server/src/serve.rs:10679`).
- [x] `rule:http-server/a-core-is-never-blocked-on-a-syscall` names static serving and its cost: one
  hand-off per path asked and per chunk
  (`docs/rules/http-server/a-core-is-never-blocked-on-a-syscall.md:1`).
- [x] `rule:http-server/static-serving-is-one-policy` and the `# What it spends` of `statics.rs`
  say the same (`docs/rules/http-server/static-serving-is-one-policy.md:1`,
  `crates/nvs-server/src/statics.rs:51`).

### 6. a setting the door reads is parsed once per configuration

**Does:** move the CSRF key, `upload_total`, the disconnect keys and the log settings into the
resolved cell, and keep a configured log file open.

**Split:** an `Explore` subagent lists every read of `snapshot.config`, `config.get(` and
`Quantity::parse` reachable per request from the door and the handler, as `file.rs:NN` and the key.

- [x] `Resolved` gains the decoded CSRF key in a type that prints nothing, `upload_total`, the
  `Disconnect` pair, and the log target, bounds, level and format
  (`rule:config/the-config-is-an-immutable-snapshot`, `crates/nvs-config/src/limits.rs:80`, `:101`).
- [x] `Snapshot` and `Request` gain the readers, and the three text readers fill the cell
  (`rule:security/csrf-is-on-by-default`, `crates/nvs-config/src/snapshot.rs:144`,
  `crates/nvs-config/src/request.rs:249`, `crates/nvs-config/src/http.rs:638`,
  `crates/nvs-config/src/app.rs:324`, `:614`).
- [x] The handler reads the key and `upload_total` from the cell, and copies the key only for a form
  whose token is in its body (`rule:security/csrf-is-on-by-default`,
  `rule:http-server/request-body-and-upload-total-are-two-caps`,
  `crates/nvs-cli/src/serve.rs:1287`, `crates/nvs-server/src/route.rs:408`).
- [x] The door reads the `Disconnect` pair from the cell of the snapshot the program runs under
  (`rule:http-server/a-request-outlives-a-client-that-goes-away`,
  `crates/nvs-server/src/serve.rs:2272`).
- [x] `log_target`, `log_minimum` and `log_format` read the cell, with the overlay check for the two
  `Runtime` keys (`rule:errors/log-write`, `crates/nvs-runtime/src/ctx/output.rs:613`, `:1365`,
  `:1400`, `:1417`).
- [x] `LogFile::with_bounds` takes a borrowed path and allocates only for a path it has not seen
  (`rule:http-server/the-floor-cannot-fill-the-disk`, `crates/nvs-runtime/src/logfile.rs:98`).
- [x] Boot and each reload take and open one `LogFile` per file target they publish, and keep it
  until the next publish stands (`rule:http-server/the-floor-cannot-fill-the-disk`,
  `crates/nvs-cli/src/serve.rs:501`, `crates/nvs-cli/src/reload.rs:357`).
- [x] Each per-request read the list names that this stage does not move is written under
  `### Backlog` with its key (`rule:config/the-config-is-an-immutable-snapshot`,
  `crates/nvs-config/src/limits.rs:80`).
- [x] Test `the_door_settings_are_resolved_once_per_snapshot` in `crates/nvs-config/src/limits.rs`
  tests: two reads of the key, `upload_total` and the `Disconnect` pair under one snapshot return the
  same addresses, and a clone of the snapshot resolves its own
  (`rule:config/the-config-is-an-immutable-snapshot`, `crates/nvs-config/src/limits.rs:147`).
- [x] Test `a_held_log_file_is_opened_once_for_any_number_of_requests` in
  `crates/nvs-runtime/src/logfile.rs` tests: with one handle held, 20 handles made, written and
  dropped count one open on a `cfg(test)` counter
  (`rule:http-server/the-floor-cannot-fill-the-disk`, `crates/nvs-runtime/src/logfile.rs:98`).
- [x] The module doc of `limits.rs` states what the cell holds and spends now
  (`rule:programs/memory-priority`, `crates/nvs-config/src/limits.rs:23`).

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 6 are done. `limits::Resolved` (`crates/nvs-config/src/limits.rs:97`) holds the decoded
CSRF key as `CsrfKey`, whose `Debug` prints `CsrfKey(..)`, `upload_total`, the `Disconnect` pair
and `LogSink` with the log level and format. `Snapshot::csrf_key`, `upload_total`, `disconnect` and
`log_sink` read it, and `Request::log_level` and `log_format` read it unless the overlay holds the
key or flips the mode. The door, `Core\Csrf` and the disconnect read use the cell, and the door
copies the key only for a form whose token is in its body. `LogFile::with_bounds` takes a `&Path`.
`LogFile::held` opens a handle now, and `logfile::held_for` takes one per file target of a
publish. `reload::Process::holding_logs` holds the boot's set, and each reload replaces it after
the publish stands, opening the new set before it drops the old one. An entry folded on its first
request after the publish has no held handle, and its first log write opens the file. Both stage
tests pass, and build and clippy are green.

The sweep's `env-secret-proofs` and the nightly Miri failures of run 43201099245 were stale a seventh
time. No such check log exists, and that run tested `578534cc7c`, before `7568aaa0fb` skipped the
four tests under Miri. A sweep that names them again needs no work.

### Next

Stage 7, the gate. No test is expected red, but the stage 6 change of `LogFile::with_bounds` and of
the log readers reaches every crate that logs.

- `crates/nvs-config/src/limits.rs:97` run `looper verify` in the background, then fix every
  failure in one pass.
- `crates/nvs-runtime/src/logfile.rs:98` then the goal-end gates: `bun nv verify --doc`,
  `bun nv owners --closes request-path-per-core` and `bun nv ci --check`.

### Backlog

- The door reads `[trace] sample` through `nvs_config::export::head_sample` and `[session] cookie`
  through `nvs_stdlib::session::cookie_in` on every request (`crates/nvs-cli/src/serve.rs:1250`,
  `:1284`). Both read the typed tree with no text parsing and no allocation, so stage 6 kept them.
- `Core\Response::sendFile` checks its grant with `capability::metadata` and `capability::readable`
  on the core thread, and off a request it also opens and reads there
  (`crates/nvs-stdlib/src/response.rs:2113`). Every `Core` file door in `nvs_runtime::capability`
  takes `&Ctx` and runs its call on the core, as `Core\IO` does. Moving the call after the grant
  check to the pool, as `pin_host` does for a name lookup, is one change for every door at once.
  The file a request sends is opened and read through `OnDisk`, off the core.
- `Table::withholding` makes each `[db]` file canonical on the core thread, once per configuration
  and never per request (`crates/nvs-server/src/mount.rs:288`).
- `spooled::door_store` clones the snapshot into a `nvs_config::Request` on every request, and no
  counted line sees it (`crates/nvs-server/src/spooled.rs:200`). A borrowed read would remove two
  shared writes.
- `Compiler::advance` takes a write guard on every background pass, even for a pointer that did not
  move, so the epoch moves once per `revalidate_freq` and each core reads its slots again then
  (`crates/nvs-cli/src/script.rs:1333`). A read check before the write would keep slots current
  across a quiet pass.
- The first log write of a request to a file target still writes on the core thread.
- `trace::respond` builds the `traceresponse` value as a `String` and copies it
  (`crates/nvs-server/src/trace.rs:366`).
- An `SseSlot` and a `BodySlot` are made for every request (`crates/nvs-server/src/serve.rs:2179`).
- A file first seen under `dispatch = "path"` resolves and compiles on the core thread of its first
  request.
