---
looper: 1
title: "no Core member, reader or shared queue holds more than its request's memory ceiling"
summary: "bun nv proofs --overshoot exits 0: every attack that ends at its memory limit stopped within 1.5x of its ceiling, and every hole of the 2026-10-30 memory review is fixed and has an attack."
context: [crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/abi.rs, tools/nv/proofs/run.ts, docs/rules/programs.md]
stages:
  - id: 1
    title: "the attack checker fails a memory FATAL past 1.5x its ceiling, and bun nv proofs --overshoot lists every attack's reading"
    status: done
    context: [tools/nv/proofs/run.ts, tools/nv/cmd/proofs.ts, tools/nv/test/hostile-ending.test.ts, crates/nvs-runtime/src/ctx/limits.rs]
  - id: 2
    title: "a memory FATAL names the peak the request held, and a debug build stops a member that passes its ceiling inside one call"
    status: done
    context: [crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/abi.rs, crates/nvs-runtime/src/ctx/limits.rs]
  - id: 3
    title: "an array, a byte buffer and a text writer ask the budget whenever they grow"
    status: done
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/string.rs, crates/nvs-runtime/src/graph.rs, crates/nvs-runtime/src/budget.rs]
  - id: 4
    title: "the graph walk, Core\\Task::map and Core\\Compress stop at the ceiling"
    status: done
    context: [crates/nvs-runtime/src/graph.rs, crates/nvs-stdlib/src/task.rs, crates/nvs-host/src/group.rs, crates/nvs-stdlib/src/compress.rs, crates/nvs-stdlib/src/cache.rs]
  - id: 5
    title: "the string, array, bytes, math and regex members stop at the ceiling"
    status: done
    context: [crates/nvs-stdlib/src/str.rs, crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/bytes.rs, crates/nvs-stdlib/src/math.rs, crates/nvs-stdlib/src/regex.rs]
  - id: 6
    title: "the JSON, CSV, HTML, JWT, URI, XML and AST builders stop at the ceiling"
    status: done
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/html.rs, crates/nvs-stdlib/src/jwt.rs, crates/nvs-stdlib/src/uri.rs, crates/nvs-stdlib/src/xml.rs, crates/nvs-stdlib/src/ast.rs]
  - id: 7
    title: "files, processes, HTTP replies and mail replies are read no further than the request can hold"
    status: done
    context: [crates/nvs-stdlib/src/io.rs, crates/nvs-stdlib/src/process.rs, crates/nvs-stdlib/src/http/transport.rs, crates/nvs-stdlib/src/http/stream.rs, crates/nvs-stdlib/src/mail.rs]
  - id: 8
    title: "database rows and directory entries are read no further than the request can hold"
    status: done
    context: [crates/nvs-db/src/pg.rs, crates/nvs-db/src/tds/rows.rs, crates/nvs-db/src/sqlite.rs, crates/nvs-stdlib/src/db/row.rs, crates/nvs-stdlib/src/db/execute.rs, crates/nvs-ldap/src/conn.rs, crates/nvs-ldap/src/ranged.rs]
  - id: 9
    title: "topic inboxes, core mailboxes, cache slots, spans and idle connections are capped in bytes and charged to who holds them"
    status: done
    context: [crates/nvs-runtime/src/peer.rs, crates/nvs-stdlib/src/topic.rs, crates/nvs-stdlib/src/bus.rs, crates/nvs-stdlib/src/cache.rs, crates/nvs-server/src/otlp.rs, crates/nvs-server/src/trace.rs, crates/nvs-runtime/src/pool.rs, crates/nvs-stdlib/src/regex.rs]
  - id: 10
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: overshoot-judge
    stage: 1
    name: an attack that ends at its memory limit passes only when the reading is within 1.5x of its ceiling plus 8 MiB
    run: [bun, test, tools/nv/test/hostile-ending.test.ts]
    tests:
      - a memory FATAL within the bound passes
      - a memory FATAL past 1.5x its ceiling fails and names the reading and the ceiling
      - a small ceiling gets the 8 MiB allowance
      - a program the host killed counts as red
  - id: ceiling-backstop
    stage: 2
    name: a memory FATAL reports the peak, and a debug build panics naming a member that passed its ceiling inside one call
    run: [cargo, test, --lib, past_its_ceiling]
    tests:
      - a_breach_reports_the_peak_the_request_held_not_the_balance_at_the_poll
      - a_member_past_its_ceiling_inside_one_call_panics_naming_it_in_a_debug_build
  - id: no-overshoot
    stage: 9
    name: every attack that ends at its memory limit stopped within 1.5x of its ceiling plus 8 MiB
    run: [bun, nv, proofs, --overshoot]
    timeout: 1800
    reach: true
    floor: true
  - id: owners-closes
    stage: 10
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, memory-ceiling-holes]
    reach: true
    floor: false
---

# no Core member, reader or shared queue holds more than its request's memory ceiling

## Why

`Core\Serialize::encode` held 17.9 GB against a 256 MB ceiling (`ed76b4102`). The user's call
(2026-10-30): such a hole is not acceptable, so the whole tree was reviewed again, and this goal
fixes what the review found and makes the class impossible to hide.

**Why it was found so late.** The attack that reached it already existed and ran on every proofs run.
Its `FATAL` named 17.9 GB held against a 256 MB ceiling each time, and it passed, because
`judgeHostile` (`tools/nv/proofs/run.ts:822`) passes any clean `FATAL` and never reads the number. Four
causes stacked up:

1. **The allocator counts every byte but refuses none.** A breach is acted on only at a poll:
   `run_helper` before and after each `Core` member (`crates/nvs-runtime/src/abi.rs:440`) and the JIT
   safepoints. Inside one native call nothing stops the growth, so each growing site has to ask
   `budget::affords` itself. The budget module's own doc says a guard per call site is the guard the next
   call site forgets, and that is what happened.
2. **The oracle ignores the reading** (above).
3. **Unit tests run uncapped.** A context with no configured limit answers every `affords` on its
   sentinel, so a unit test never sees an overshoot unless it sets a ceiling.
4. **The class was known and not swept.** `Core\Arr::from` grew to 6 GB on 2026-10-03, was fixed at
   that one site, and the note "other members may share the shape" got no sweep. The dev host absorbs
   17 GB, so the attack only failed under the floor debt's parallel load, as a host out of memory.

**What a scan measured.** Every one of the 225 attacks that ends at a `FATAL`, run with the release
binary of 2026-10-30 17:17 (before `ed76b4102`). All of these pass the checker today:

| Attack | Held / ceiling |
|---|---|
| `tests/hostile/core/Task/map/01-bad-limits-huge-arrays-and-errors-from-every-task.nvs` (step 7) | 20.1 GB / 256 MB, 78x |
| `tests/hostile/core/Str/join/01-a-result-too-large-to-hold.nvs` | 16.4 GB, 61x |
| `tests/hostile/core/Serialize/encode/01-deep-chains-loops-and-hidden-callables.nvs` | 17.9 GB, fixed in `ed76b4102` |
| `tests/hostile/core/Compress-Compressor/finish/01-a-frame-asked-for-at-the-worst-moments.nvs` | 8.2 GB, 32x |
| `tests/hostile/lang/concurrency/core-task-map-one-task-per-element/01-a-map-over-everything-it-will-take.nvs` | 8.1 GB, 31x |
| `tests/hostile/core/Compress-Compressor/add/01-one-buffer-added-a-hundred-thousand-times.nvs` | 2.1 GB, 8x |
| `tests/hostile/core/Compress/compressor/01-a-stream-that-is-never-finished.nvs` | 1.0 GB, 4x |
| `tests/hostile/core/Uri/encodeComponent/01-text-that-grows-three-times.nvs` and `encodeFormValue/01-…` | 954 MB, 3.7x |
| `tests/hostile/core/Bytes/unpack/01-a-format-that-reads-more-than-the-buffer-holds.nvs` | 513 MB, 2.0x |
| `tests/hostile/core/Arr/fromKeysAndValues/01-more-pairs-than-memory.nvs` | 371 MB, 1.45x |
| `tests/hostile/core/Ast-Node/column/01-…`, `line/01-…` | 325 MB, 1.27x |

Every other memory `FATAL` stopped within 1.14x. Run 3 at a time, the Serialize and `Str::join`
attacks ended in exit 9 with no reading: the host ran out.

**What a read-only review found** (six lanes over nvs-stdlib, nvs-runtime, nvs-host, nvs-server,
nvs-db and nvs-ldap). These are leads until an attack or a test pins each one. The four marked
*confirmed* were read at their site, and the stage items below carry every lead with its anchor.
A lead an attack shows is already bounded is closed with that attack, not with a code change.

## Standing decisions

**The bound.** A memory `FATAL` is green when the reading is at most `1.5 x ceiling + 8 MiB`. The
measured legitimate worst is 1.14x. The 8 MiB lets an attack that sets a tiny ceiling, such as
`tests/hostile/core/Config/set/01-a-request-raising-its-own-ceiling.nvs` at 1 byte, through. The bound
is never raised to make an attack pass: a site that needs a reallocation asks for the whole new block
(old and new held at once), as `Encode::put` does. The bound is a constant in `run.ts`, and its doc
comment says why it is 1.5.

**A server-full `FATAL` in an attack is red.** That line names no ceiling. In a proof run the request
is the only one, so the server's memory filling first means the request's own ceiling never stopped
it. No attack targets the server's limit, and the scan saw none of these lines.

**The reading is the peak.** The `FATAL` reports the highest balance the request held
(`Ctx::memory_held`), so a member that grew to 20 GB and freed most of it before the poll still reads
20 GB. `budget`'s existing `PEAK` mark is that number, so `add` gains no tracking: before a breach the
request was under its ceiling, so the mark since its context began is the mark since the breach. The
mark is read only once a poll has found the breach.

**`bun nv proofs --overshoot` is the scan as a command** (AGENTS.md rule 12). It runs every hostile
`.nvs` and `.cmd` whose `ends-early` names `FATAL`, two at a time, and prints the reading worst first.
It exits 1 when any reading is past the bound, or when the host killed a program (an exit status of 9
or a signal). The scratch scan it replaces is `.agent-tmp/overshoot/scan.ts`. Its results are
`.agent-tmp/overshoot/results.json`. Stage 1 deletes both once the command exists. Its run takes about
6.5 minutes of program time (378 s summed over 225 programs).

**The debug backstop.** In a `debug_assertions` build, `run_helper` compares the peak a member call
reached against `1.5 x ceiling + 8 MiB`, and panics naming the member when it passed. Every Rust test
that sets a ceiling, and every debug-built `nvs` run, then catches the class without an attack. The
release build keeps today's behaviour: the request ends at its next poll. The member's name is the
body closure's `type_name`, so `run_helper`'s signature is unchanged. Only a call that raised the mark
is checked, and a body that returns `Fault::Pending` is not, so a callback's outer member is never
blamed for its callee. The panic is contained by `run_helper`'s `catch_unwind` as a `FATAL`, and a
result the body returned leaks on that debug-only path.

**One place asks, not each site** (`rule:programs/memory-priority`). Stage 3 builds the mechanism the
later stages use:
- `NvsArray` asks `budget::affords` for the new backing whenever its `Vec` reallocates, so every member
  that builds an array is bounded without a per-element check. Cost: one comparison per reallocation,
  none per append that fits. A refused growth sets the breach and leaves the array as it was. The JIT's
  `affords_write` stays.
- `Encode::put`'s logic moves into one byte writer in `budget`, which `encode`, the JSON sink, the CSV
  writer, the compressor and the database readers share. It asks for the whole new block at each
  growth, stops writing after a refusal, and reports the refusal to its caller.
- Text outputs use `StrWriter` (`crates/nvs-runtime/src/string.rs:1146`), which already asks.
- A per-element `abi::affordable` is the fallback only where neither fits, and its site says why.

**A refusal stops the work, not only the writes.** A walk that has been refused returns at once, so a
doubling graph (`$a = [$a, $a]` forty times) costs no 2^40 visits after the refusal. The caller gets
an empty value and the breach is already recorded, as `encode` does today.

**No new configuration limits.** Every cap comes from a limit that exists:
- A reader with no `max_output` takes the request's memory limit as its intake bound
  (`crates/nvs-runtime/src/ctx/limits.rs:546`).
- A database or directory frame asks the budget per growth, and the wire's own caps stay.
- A topic inbox holds at most its subscriber's memory limit in bytes, beside today's 256 deliveries,
  and a full inbox takes today's overflow path.
- A core mailbox holds at most one request's memory limit in bytes.
- A span name keeps at most 1 KiB of the path, as `QuerySpan` truncates its SQL.
- The idle pool per core holds at most the block's `max` connections, oldest out first.

**Bytes a queue holds are charged to the holder.** A delivery is charged to the subscriber's tree when
it is queued, never to a publisher whose request has ended. Safe fallback, if the cross-tree charge
cannot be made O(1): the byte cap alone, with the doc comment saying the bytes are counted to the
publishing core.

**The cache stores nothing on a refusal.** `encode_within` gains a third outcome for a refused growth,
distinct from "past the limit". `put` stores nothing and returns. Today it stores an empty payload,
and every later `get` on that key throws a decode error (`crates/nvs-stdlib/src/cache.rs:2668`).

**A stale cache slot holds no key bytes.** The `order` queue holds slot numbers alone, and a map from
a live slot's number to its key handle names the key. `forget` takes the key out of that map at once,
so a stale slot is eight bytes until the sweep. A hash in the slot was declined: the standard map
cannot be looked up by a hash, so matching on one needs a scan. Cost: one map entry per live key.

**A delivery's bytes move between trees in O(1).** The publisher measures each copy as the rise of
`budget::live_bytes()` across it, and `Inbox::push_sized` hands that many bytes to the subscriber's
tree with `budget::hand_to`: `credit_turn` on the running turn and a relaxed add to the parked tree's
`balance`. The inbox records the subscriber's tree at `Ctx::inbox` only where the subscribing stack
runs that tree's turn, and nothing moves where the publisher runs no turn or the subscriber's own, so
a CLI or a self-publish stays as before. The byte cap is the subscriber's `memory_limit` at its last
subscribe. A core mailbox's byte cap is the publishing request's `memory_limit`, `0` for none.

**Every fix has an attack.** Each fixed member gets a hostile step that builds the amplification
(shared references, a doubling graph, a huge separator, a peer that never ends a frame) and ends at
`FATAL` within the bound. An existing attack gains the step where one exists. A remote peer's part uses
the fixtures the member's attacks already use. Attacks are text an end user reads, held to AGENTS.md
§ *Text an end user reads*.

**The collector's scratch is bounded, not out of scope.** The cart attack has no member in its
loop. At about 1.1M live objects, `reclaim`'s members `Vec`, its `HashMap` seat table (rounded to
2M buckets) and its `held` `Vec` take about 50 MiB, which is the cart's 114 MiB reading against
64 MiB. The goal cannot be reached while that attack is red, so the scratch is this goal's. The
fix keeps the collector: a sorted `Vec` and a binary search replace the `HashMap`, which takes the
scratch from about 35 to 16 bytes per live object. Cost: O(n log n) for the walk, which runs only at
the ceiling. Where even that scratch does not fit under the bound, the in-flight walk is skipped and
the request ends on its breach, as it would without a collector. The teardown walk is unchanged.

**A JWT parse is capped before it starts, and an unsigned part by size.** A `serde_json::Value`
tree is up to about 64 times its text (a list of one-digit numbers). A header is read before its
signature, so a forger picks its size: a header past 8 KiB (`MAX_HEADER`) is the ordinary
refusal, a catchable throw, and never a `FATAL`. A key set past 1 MiB (`MAX_KEY_SET`) throws
`not_a_key_set`. Sixteen keys need a few kilobytes. A signed payload, and a key set within the cap,
ask for `parse_bound` first: the text plus 128 bytes per value, counted in one string-aware pass.
A flat factor was declined because it refused `sign`'s own 32 MB single-claim round trip. A
visitor that builds no tree was declined as more code for the same bound.

**A child's two readers share the request's headroom.** The reader threads run under no ceiling,
so `Capture::of` reads `budget::headroom()` on the request's thread and both readers take each
growth out of one atomic room. A growth past it stops the child and returns `CaptureRefused`, and
the member asks `affords` for what the pair wanted, which records the breach and gives the memory
`FATAL`. A captured buffer is charged to the request with `carry` when `drain` receives it, so the
request that frees it was charged for it. Cost: one atomic update per buffer growth. After a
timeout or a refusal a reader frees its buffer and ends at its next read. Reading into a fixed
buffer and discarding was declined: it keeps a thread alive for as long as a grandchild writes.

**A peer's bytes are framed as they arrive, never held whole first.** The chunked decoder moves
each chunk's octets across as they arrive (`Framed::chunk_left`), so a chunk header picks nothing
about what is held, and `whole`'s `REPLY_CEILING` check bounds a buffered reply. Refusing a chunk
larger than what is left under `REPLY_CEILING` was declined: it refuses a streamed reply's
legitimate large chunk. `Http\Stream::events` consumes the lines of a finished block that
dispatched nothing (`EventScan::settle`), and an id such a block set goes to `lastId` then, as the
WHATWG format says. A block still open throws past `BLOCK_CEILING` (2 MiB). SMTP replies throw past
4 KiB a line and 256 lines. Cost: none per byte; one comparison per read.

**An attack needs a way in from a `.nvs` file, and a Rust test stands where there is none.** The
answer table gives a streamed call its whole body, so no `.nvs` reaches the chunked decoder or an
event stream that arrives line by line. Those are Rust tests:
`a_chunk_header_past_any_reply_fails_without_a_panic` and
`lines_that_dispatch_nothing_are_consumed_as_they_are_read`, and the SMTP caps'
`a_reply_is_capped_per_line_and_per_reply`. `IO::stdin` reads through the same `read_bounded` as
`IO::read`, and its attack would need an `.in` file larger than a memory limit in the tree, so
`IO::read`'s attack covers it. Samba sends a range only when one is asked for (a plain read of
2000 values arrives as one message), and `select` refuses a range option, so no `.nvs` reaches
`ranged::complete`. `a_ranged_attribute_that_never_ends_stops_at_the_memory_ceiling` in
`crates/nvs-ldap/tests/samba.rs` is its attack, a scripted peer whose range never ends.
`tests/hostile/core/Ldap-Connection/read/02-` reads the same large attribute whole.

**A database read asks before the bytes exist, and SQLite's own buffers are capped too.**
PostgreSQL asks for a whole frame from its header before `Message::parse` reserves it (the parse
reserves the declared length itself, which is how the first fix passed 2.39x), and a read stops at
the frame's end. SQL Server asks in `copy`, which every value of a row passes. SQLite's engine
memory is counted against no request, so each statement sets `SQLITE_LIMIT_LENGTH` to the
headroom plus the longest parameter; a parameter the request already holds still binds. A
`SQLITE_TOOBIG` under that limit is the memory `FATAL`, not a throw. Setting the limit from the
whole memory limit was declined: a stored value between the headroom and the limit would be
built by the engine and refused only at the copy. Cost: one `sqlite3_limit` call per statement,
one comparison per cell, per frame and per buffer growth.

**Out of scope, decided elsewhere.**
- No cap on open connections before admission
  (`docs/rules/http-server/admission-is-arithmetic-not-a-number.md`).


**Trade-offs.**
- Security: every request's memory is bounded within one native call. That is what the security
  ordering asks of a per-request ceiling.
- Latency: one comparison per buffer reallocation, and none per write that fits.
- Memory: nothing is added. Queues and pools hold less than today.
- Simplicity: one writer and one array rule, where today each site has its own check.

**Lanes.**
- Stages 1 to 4 run in order. Stage 3's types are what 4 to 9 use.
- Stages 5, 6, 7, 8 and 9 touch disjoint files once stage 3 is in. Each may run as a side goal.
- Stage 9 reads `regex.rs`'s cache, and stage 5 changes `regex.rs`'s members. Run 9 after 5, or keep
  the edits to separate functions.

## Stages

### 1. the attack checker fails a memory FATAL past 1.5x its ceiling, and bun nv proofs --overshoot lists every attack's reading

**Does:** the oracle reads the number, and the scan is a command. Once this lands, the attacks in the
table above go red. That is expected: this goal's later stages turn them green.

- [x] `judgeHostile` parses `N bytes held against a ceiling of C` and the "server's memory is full" line, and fails past `1.5 x C + 8 MiB`, naming both numbers. The bound is one exported constant with its reason. `tools/nv/proofs/run.ts:822`
- [x] An exit of 9 or a signal with no reading fails as "the host ran out of memory". `tools/nv/proofs/run.ts:844`
- [x] `bun nv proofs --overshoot` runs every attack whose `ends-early` names `FATAL`, two at a time through `spawnProof` and `proofArgv`, and prints worst first. Exit 1 past the bound or on a killed program. `USAGE` and the module doc name it. `tools/nv/cmd/proofs.ts:8`, `tools/nv/proofs/run.ts:622`
- [x] The four tests the `overshoot-judge` check names. Run once by name: `bun test tools/nv/test/hostile-ending.test.ts`. `tools/nv/test/hostile-ending.test.ts:1`
- [x] Delete `.agent-tmp/overshoot/`. `.agent-tmp/overshoot/scan.ts:1`
- [x] Replace the playbook bullet on `floor-debt`'s red atom with one line: the checker now reads the number. `docs/agent/playbook.md:92`

### 2. a memory FATAL names the peak the request held, and a debug build stops a member that passes its ceiling inside one call

- [x] `budget::add` keeps the highest balance since the breach, on the over-ceiling path only. `crates/nvs-runtime/src/budget.rs:1243`
  The existing `PEAK` mark is that balance, so `add` is unchanged (*The reading is the peak*).
- [x] `memory_fault` reports that peak. `crates/nvs-runtime/src/ctx/limits.rs:628`
- [x] `run_helper`'s debug-build check after each body panics naming the member past the bound. `crates/nvs-runtime/src/abi.rs:496`
- [x] The two tests the `ceiling-backstop` check names, in `budget.rs` and `abi.rs`'s test modules. Run once: `cargo test --lib past_its_ceiling`. `crates/nvs-runtime/src/budget.rs:1161`
- [x] The budget module doc says what the peak and the backstop cost. `crates/nvs-runtime/src/budget.rs:1`

### 3. an array, a byte buffer and a text writer ask the budget whenever they grow

- [x] `NvsArray`'s growth asks for the new backing at each reallocation. `append` and `set` leave the array unchanged on a refusal. `crates/nvs-runtime/src/array.rs:1275`, `crates/nvs-runtime/src/array.rs:1167`, `crates/nvs-runtime/src/array.rs:1155`
- [x] The shared byte writer in `budget`, taken out of `Encode::put`. `encode` uses it. `crates/nvs-runtime/src/graph.rs:661`, `crates/nvs-runtime/src/budget.rs:1171`
- [x] Unit tests: an array and the writer under a 1 MiB ceiling stop at it and record the breach. Run once by name.

### 4. the graph walk, Core\Task::map and Core\Compress stop at the ceiling

- [x] `copy_graph`'s `Live` carrier asks for each copied string and array, so a string held in many slots is not copied unasked. This covers topics, SSE, sockets, isolate hand-off and `clone`. `crates/nvs-runtime/src/graph.rs:537`
- [x] The walk returns at the first refusal, in `encode`, `encode_within` and `copy_graph`. `crates/nvs-runtime/src/graph.rs:238`
- [x] `encode_within`'s refused outcome, and `put` stores nothing on it. `crates/nvs-runtime/src/graph.rs:855`, `crates/nvs-stdlib/src/cache.rs:2668`
- [x] `Core\Task::map` and `Task::all` ask for each task's stack charge before starting it. Step 7 of the Task::map attack (20,000 tasks of 1 MiB) stops at the bound. `crates/nvs-stdlib/src/task.rs:513`, `crates/nvs-host/src/group.rs:428`, `crates/nvs-host/src/group.rs:770`
- [x] `Compressor::add` retains a value that `finish` then copies N times: `finish` writes through the shared writer, and the decompression ceiling is computed before that copy. `crates/nvs-stdlib/src/compress.rs:1175`, `crates/nvs-stdlib/src/compress.rs:1204`, `crates/nvs-stdlib/src/compress.rs:1351`
- [x] Attacks: a doubling graph through `Core\Serialize::encode` and through a topic publish, both ending at `FATAL` within the bound. The Compressor and Task attacks pass unchanged.

### 5. the string, array, bytes, math and regex members stop at the ceiling

**Split:** the attacks go to one writing subagent once the fixes are in.

- [x] `Core\Str::join` writes through `StrWriter`, not a `String` (measured at 61x). `crates/nvs-stdlib/src/str.rs:2069`
- [x] `Str::codePoints`, `graphemes` and `chunk` are bounded through stage 3's array growth. Each gets an attack step. `crates/nvs-stdlib/src/str.rs:2452`, `crates/nvs-stdlib/src/str.rs:2425`, `crates/nvs-stdlib/src/str.rs:2283`
- [x] `Arr::flattenDeep`, `flatten`, `appendAll` and `chunk` over shared rows, and `fromKeysAndValues` (1.45x). `append_values` asks. `crates/nvs-stdlib/src/arr.rs:3902`, `crates/nvs-stdlib/src/arr.rs:3862`, `crates/nvs-stdlib/src/arr.rs:5804`, `crates/nvs-stdlib/src/arr.rs:3438`, `crates/nvs-stdlib/src/arr.rs:3024`, `crates/nvs-stdlib/src/arr.rs:4244`
- [x] `Bytes::unpack` asks for its `Vec<Field>` and its array (2.0x). `crates/nvs-stdlib/src/bytes.rs:1575`
- [x] `Math::format`'s group separator is counted in its ask. `crates/nvs-stdlib/src/math.rs:2113`
- [x] `Regex::matchAll` and `replaceWith` ask per match. `crates/nvs-stdlib/src/regex.rs:1861`, `crates/nvs-stdlib/src/regex.rs:2128`
  - [x] `matchAll` returns at a refusal, and `replaceWith` asks per kept match (`kept`).
  - [x] `replaceWith` with `.` over 50 million characters reads 0.94x of 256 MiB on the proof binary, refused at `found`'s growth. The 12x reading came before stage 5's array fixes. Its attack is `tests/hostile/core/Regex/replaceWith/02-a-match-at-every-character.nvs`.
- [x] `Arr::fillKeys` (2.81x) and `range` (runs out of time before its `FATAL`), which the peak reading of stage 2 put on the scan. `shapeAs` (1.70x) is `json::hydrate`, and moved to stage 6 beside `jsonAs`. `crates/nvs-stdlib/src/arr.rs:4088`, `crates/nvs-stdlib/src/arr.rs:4166`
  - [x] `NvsArray::with_capacity` and the packed-to-hash conversion ask for their room. `crates/nvs-runtime/src/array.rs:481`, `crates/nvs-runtime/src/array.rs:1071`
  - [x] `range` asks for its whole list once. `crates/nvs-stdlib/src/arr.rs:4166`
- [x] `tests/hostile/tools/php-differences/porting-a-program/02-a-cart-that-grows-past-the-memory-limit.nvs` reads 1.78x of 64 MiB. No member: it is the cycle collector's scratch at the poll that finds the breach (see *The collector's scratch* under Standing decisions). `crates/nvs-runtime/src/object.rs:3243`
  - [x] `reclaim` keeps no `HashMap` seat table: each member's seat is its position in a `Vec` sorted by address, found by binary search.
  - [x] Before the in-flight walk, ask whether the scratch fits under the bound; where it does not, skip the walk and report the breach.
  - [x] The cart attack reads within the bound under `bun nv proofs --overshoot`: 1.25x.
- [x] An attack step per member above that has none, each ending at `FATAL` within the bound. `Arr::chunk` gained step 7.

### 6. the JSON, CSV, HTML, JWT, URI, XML and AST builders stop at the ceiling

**Split:** as stage 5's.

- [x] *Confirmed:* `Json::encode`'s `Sink` writes through the shared writer. This also bounds `Response::json`, the HTTP JSON body, SSE, queue job arguments and `Jwt::signObject`. `crates/nvs-stdlib/src/json.rs:781`, `crates/nvs-stdlib/src/json.rs:915`
- [x] `Csv::format` and `Html::join` write through a writer that asks. `Csv::format` is a `ByteWriter` (0.80x), and `Html::join` sizes the page in one pass and asks twice it (a refusal before any copy). `crates/nvs-stdlib/src/csv.rs:894`, `crates/nvs-stdlib/src/html.rs:931`
- [x] `Jwt::sign` asks before its payload and base64 copies (0.75x). `Jwt::verify` and `KeySet::read` cap the JSON they parse before the parse (verify 0.92x). `crates/nvs-stdlib/src/jwt.rs:1113`, `crates/nvs-stdlib/src/jwt.rs:1528`, `crates/nvs-stdlib/src/jwt.rs:2245`
- [x] `Uri::encodeComponent` and `encodeFormValue` (5.2x), now 0.75x: one counting pass sizes the result, written once (`encoded`). `parseQuery`'s top-level `out.set` is bounded through stage 3. `crates/nvs-stdlib/src/uri.rs:3143`, `crates/nvs-stdlib/src/uri.rs:3368`
- [x] `Xml\Writer`'s indent is counted in its ask (0.85x). `crates/nvs-stdlib/src/xml.rs:3049`
- [x] `Ast::parse`'s node objects ask (1.27x), now 1.24x: the parser asks a keep-going callback per statement (`parse_file_within`). `crates/nvs-stdlib/src/ast.rs:454`
- [x] `Http\Response::jsonAs` (1.64x), `Core\Arr::shapeAs` (1.70x, now 1.0x) and `Response::html` (1.54x, now 0.79x through the capture buffer's ask), which the peak reading put on the scan. `jsonAs` and `shapeAs` both hydrate through `json::hydrate`. `jsonAs` reads within the bound since stage 5's array fixes. `crates/nvs-stdlib/src/arr.rs:6462`
- [x] An attack step per member above, each ending at `FATAL` within the bound. `KeySet::read`'s step 10 throws instead, by the JWT parse decision.

### 7. files, processes, HTTP replies and mail replies are read no further than the request can hold

- [x] `intake_limit` falls back to the memory limit when `max_output` is unset, and `IO::read`, `readText` and `lines` read through `ByteWriter::read_from`, which asks before each growth. `crates/nvs-runtime/src/ctx/limits.rs:553`, `crates/nvs-stdlib/src/io.rs:2291`
  - [x] `intake_limit` falls back, and the throw names the setting that set it
  - [x] `read_bounded` reads through `ByteWriter` and asks for the string copy first
- [x] `IO::stdin` reads under that bound, through `read_bounded`. `crates/nvs-stdlib/src/io.rs:2271`
- [x] `IO::list` and `walk` ask per entry: `beneath`'s two listers call `affords_entry`, `read_dir` turns `ListingRefused` into the memory `FATAL`, and `walk` stops at a refused growth. `crates/nvs-runtime/src/beneath.rs`, `crates/nvs-runtime/src/capability.rs:1258`
- [x] Process reader threads stop at the bound and share the request's headroom. After a timeout or a refusal they free what they read and end at the next read. `crates/nvs-stdlib/src/process.rs` `read_capped`, `drain`
  - [x] `Capture` and `Shared`, and `read_capped` takes each growth out of the room
  - [x] `drain` carries a received buffer and stops on `CaptureRefused`
  - [x] `run` and `wait` record the refusal and ask for the copy first
  - [x] `a_capture_stops_at_the_room_its_two_readers_share` passes
- [x] The HTTP client's chunked decoding moves a chunk's octets as they arrive, and a 16-hex-digit size throws. It was not confirmed: the decoder held a whole chunk, and `at + size + 2` overflowed. `crates/nvs-stdlib/src/http/transport.rs` `dechunk`
  - [x] `chunk_left` carries a chunk across reads, and a header with no line end past `LINE_CEILING` throws
  - [x] `a_chunk_header_past_any_reply_fails_without_a_panic` passes
- [x] `Http\Stream::events` consumes the lines of a finished block that dispatches nothing, and an open block throws past `BLOCK_CEILING`. `crates/nvs-stdlib/src/http/stream.rs` `EventScan::settle`
  - [x] `settle`, and `step` consumes what it returns before each pull
  - [x] `lines_that_dispatch_nothing_are_consumed_as_they_are_read` passes
- [x] SMTP replies are capped at 4 KiB a line and 256 lines. `crates/nvs-stdlib/src/mail.rs` `REPLY_LINE_CEILING`, `REPLY_LINES`
- [x] Attacks, each ending at `FATAL` within the bound: a 40 MB file under 16 MiB (0.81x), a folder of 4 MB of names under 2 MiB (0.75x), and a child that prints without end under 32 MiB. The HTTP and SMTP ones are Rust tests, and stdin shares `IO::read`'s attack, by the standing decision.
  - [x] `tests/hostile/core/IO/read/02-a-file-larger-than-the-memory-limit.nvs`
  - [x] `tests/hostile/core/IO/list/02-a-folder-larger-than-the-memory-limit.nvs`
  - [x] `tests/hostile/core/Process/spawn/02-a-child-that-never-stops-printing.nvs`

### 8. database rows and directory entries are read no further than the request can hold

- [x] *Confirmed:* Postgres `read_message` asks before each `inbox` growth, using the frame length once it is known. `crates/nvs-db/src/pg.rs:612`
- [x] Postgres array columns ask per element. `crates/nvs-db/src/pg.rs:2284`
- [x] SQL Server PLP and LONG values ask per chunk. `crates/nvs-db/src/tds/rows.rs:705`, `crates/nvs-db/src/tds/rows.rs:678`
- [x] SQLite: each cell is checked against `headroom` before its copy, and each connection sets `SQLITE_LIMIT_LENGTH` from the request's memory limit. `crates/nvs-db/src/sqlite.rs:303`, `crates/nvs-db/src/sqlite.rs:740`, `crates/nvs-db/src/sqlite.rs:1083`
- [x] `Rows::all`, `iterate` and `queryAs` hydration ask per row. `crates/nvs-stdlib/src/db/row.rs:781`
- [x] LDAP `Cursor::fetch` asks per entry, and the ranged-attribute loop stops when a range does not advance and asks per merge. `crates/nvs-ldap/src/conn.rs:959`, `crates/nvs-ldap/src/ranged.rs:79`
- [x] Attacks: a query that returns one huge value for each driver the fixtures run, and an LDAP group with a ranged member list. Each ends at `FATAL` within the bound.
  - [x] SQLite, PostgreSQL and SQL Server: `tests/hostile/core/Db-Connection/query/02-`, `03-`, `04-`, run once, each green
  - [x] the LDAP ranged member list: `a_ranged_attribute_that_never_ends_stops_at_the_memory_ceiling` (no `.nvs` can ask a range), and `tests/hostile/core/Ldap-Connection/read/02-` for a whole large attribute, each run once, green

### 9. topic inboxes, core mailboxes, cache slots, spans and idle connections are capped in bytes and charged to who holds them

- [x] The topic inbox's byte cap and its charge to the subscriber's tree. `crates/nvs-runtime/src/peer.rs:153`, `crates/nvs-stdlib/src/topic.rs:777`
- [x] `Topic::publish` asks before each copy per subscriber. `receive` asks per decoded envelope. `crates/nvs-stdlib/src/topic.rs:799`, `crates/nvs-stdlib/src/topic.rs:689`
- [x] The core mailbox's byte cap. `crates/nvs-stdlib/src/bus.rs:101`
- [x] A stale cache slot holds no key bytes. `crates/nvs-stdlib/src/cache.rs:912`, `crates/nvs-stdlib/src/cache.rs:974`
- [x] A span name keeps at most 1 KiB. `crates/nvs-server/src/trace.rs:238`, `crates/nvs-server/src/otlp.rs:101`
- [x] The idle pool per core holds at most the block's `max`, and `release` retires expired entries. `crates/nvs-runtime/src/pool.rs:564`
- [x] The regex cache counts pattern bytes beside its 256 entries. `crates/nvs-stdlib/src/regex.rs:908`
- [x] Tests: a subscriber that never reads holds at most its limit; a forgotten cache key leaves no key bytes; a long path makes a 1 KiB span. Run each once by name.
  - [x] `a_subscriber_that_never_reads_holds_at_most_its_memory_limit` and `a_copy_handed_to_a_parked_tree_is_charged_to_that_tree`
  - [x] `a_mailbox_refuses_an_envelope_past_the_publishers_memory_limit`
  - [x] `a_forgotten_key_leaves_no_key_bytes_behind`
  - [x] `a_long_path_makes_a_span_of_at_most_one_kibibyte`
  - [x] `the_idle_store_holds_at_most_max_and_a_release_retires_expired_entries` and `the_cache_keeps_at_most_its_bound_in_pattern_text`
- [x] `bun nv proofs --overshoot` exits 0.

### 10. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] Run `looper verify` over the whole goal.
  - [x] Started in the background as `bun nv affected --run`, on the tree at `da8ac348f`.
  - [x] Its report read, and its failures grouped by cause.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
  - [x] Five undeclared memory-limit texts get tests; `process.rs`'s cast; the group test (`956669248`).
  - [x] The assertion ledger asks before it grows (`94a9b434f`).
  - [x] The reference bench count and the website render (`cea8e01ad`).
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 9 done; overshoot exits 0 (worst 1.45x). The installed `looper` has no `verify`, so
the gate is `bun nv affected --run`. Run 1 found three reds, fixed in `956669248`. Run 2 reached
step 19 (conformance): the assertion ledger, fixed in `94a9b434f`, and six `image-*` cases that
aborted with `memory allocation of ~1.5 MB failed`. Those six pass alone and together on the same
binary, and no job limit exists, so they read as host commit running out under the gate's load.
Run 3 passed conformance, the six included, and stopped at step 20 (reference): `Str::join`'s
bench count fell. That is re-rendered in `cea8e01ad`, and `bun nv reference` holds 678 of 678. Steps
21 to 23 have not run yet.

Expected red at the gate, none a test to loosen: an array built past its ceiling is shorter; one
call past 1.5x plus 8 MiB is an `inside one call` `FATAL`; `N bytes held` may read the higher peak;
a partial `copy_graph` or `Some(empty)` from `encode_within`; `NvsArray::with_capacity` under a
small ceiling is empty; `Arr::range` past it is a `FATAL` first; a collector test may reclaim 0; a
stopped parse is `walk::STOPPED`; a refused capture write writes nothing; JWT caps throw; a file,
stdin or child output past a small limit throws or is the `FATAL`; a chunk or event-stream test
reads the new framing; a SQLite `SQLITE_TOOBIG` under a ceiling is the `FATAL`; an LDAP range with
no value that asks for more is `Kind::Protocol`; a test that reads `Inbox`'s `Debug` sees `held`; a
pool test that releases under many keys with a small `max` finds fewer idle; a MySQL read under a
small ceiling is `OutOfMemory` at the chunk header. Look first at `crates/nvs-runtime/tests/refusal.rs`,
`tests/server_memory.rs` and `crates/nvs-stdlib/src/budget.rs`.

### Next

Stage 10's last run. `bun nv affected --run` with `run_in_background`; steps 1 to 20 are green, so
it runs little more than 21 to 23. Fix what it names, never loosening a test. Then the goal-end gates:
`bun nv verify --doc`, `bun nv owners --closes memory-ceiling-holes`, and `bun nv ci --check`.

- [ ] `bun nv affected --run` green from `tests/conformance/novis/image-a-barcode-renders-as-png.nvst:1`.
- [ ] The goal-end gates, then `DONE`. The owners check is `tools/nv/cmd/owners.ts:1`.

### Backlog

- `tools/looper/` names `looper verify` and `looper check --name`, and the installed `looper.exe`
  has neither. `bun tools/looper/main.ts verify` may be the project's copy of it.
