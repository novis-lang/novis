---
looper: 1
title: "A request pays for its own program, and every security check stays where it is"
summary: "Every security check on the request path stays in force and runs no later than before, while a request stops paying for work the server repeats: a Core class's descriptor is found with one hash lookup, Json::decodeAs fills its class in one pass, escaping scans text once, a request's limits come from a table resolved once per unit, a read the reactor has not reported ready and a wake on a core's own task make no system call, pooled stacks give their pages back when a core goes idle, every touched area's attacks and the comparison's fault rows are green, and the checks below pass."
context: [docs/rules/programs/memory-priority.md, docs/rules/testing/perf-two-mechanisms.md, docs/rules/testing/member-perf-ledger.md, benches/compare/README.md]
stages:
  - id: 1
    title: "the profile is one command, it runs again on main, and every touched area's attacks are green"
    status: done
    context: [tools/nv/cmd/bench-compare.ts, benches/compare/README.md, tools/nv/cmd/proofs.ts, tests/hostile/README.md]
    extra:
      context:
        rules: [testing/hostile-case-contract, testing/perf-two-mechanisms]
  - id: 2
    title: "a Core class's descriptor is found with one hash lookup"
    status: done
    context: [crates/nvs-stdlib/src/instance.rs, crates/nvs-stdlib/src/html.rs, crates/nvs-stdlib/src/json.rs]
    extra:
      context:
        rules: [core-api/core-classes-are-sealed, security/isolate-values-cross-by-copy]
  - id: 3
    title: "`Json::decodeAs` fills its class in one pass"
    status: open
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/budget.rs]
    extra:
      context:
        rules: [http-server/engine-depth-is-bounded-per-decoder, errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself, security/taint-propagation, security/derived-codec-qualifiers]
  - id: 4
    title: "escaping scans text once, and a request under its memory limit passes the poll cheaply"
    status: open
    context: [crates/nvs-render/src/html.rs, crates/nvs-render/src/bidi.rs, crates/nvs-render/src/text.rs, crates/nvs-runtime/src/ctx/limits.rs]
    extra:
      context:
        rules: [security/bidi-predicate, security/bidi-boundaries, core-classes/html-escape-answers-markup, errors/on-limit]
  - id: 5
    title: a request reads its limits from a table its unit resolved once
    status: open
    context: [crates/nvs-config/src/request.rs, crates/nvs-config/src/defaults.rs, crates/nvs-config/src/snapshot.rs, crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-server/src/trace.rs, crates/nvs-stdlib/src/config.rs]
    extra:
      context:
        rules: [config/a-runtime-set-is-request-local, config/the-config-is-an-immutable-snapshot, config/a-request-keeps-the-unit-it-resolved, security/no-cross-request-state]
  - id: 6
    title: "a read the reactor has not reported ready, and a wake on a core's own task, make no system call"
    status: open
    context: [crates/nvs-host/src/net.rs, crates/nvs-host/src/block_on.rs, crates/nvs-host/src/reactor.rs, crates/nvs-server/src/io.rs, crates/nvs-server/src/serve.rs]
    extra:
      context:
        rules: [concurrency/try-the-syscall-then-park, concurrency/a-waker-is-one-permission-to-poll, concurrency/one-permission-per-drive, http-server/four-idle-waits-all-finite, concurrency/connection-bounds-are-finite]
  - id: 7
    title: "pooled stacks do not grow: one pool per role, and pages go back when a core is idle"
    status: open
    context: [crates/nvs-host/src/stack.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/isolate.rs, crates/nvs-cli/src/serve.rs]
    extra:
      context:
        rules: [concurrency/a-task-stack-is-reserved-wide-and-pooled, concurrency/a-tasks-recursion-limit-comes-from-its-own-stack, errors/stack-depth]
  - id: 8
    title: "the ledgers and the comparison are recorded again, and the fault rows keep their results"
    status: open
    context: [tools/nv/cmd/bench-compare.ts, website/src/content/docs/in-depth/compare.mdx, benches/compare.json]
    extra:
      context:
        rules: [testing/member-perf-ledger, http-server/a-requests-blast-radius-is-bounded-at-four-tiers]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: stdlib-tests
    stage: 3
    name: "a Core class's descriptor and a decodeAs object are built with one hash lookup and in one pass"
    run: [bun, nv, verify, --fast, -p, nvs-stdlib]
    tests: [every_core_class_finds_its_descriptor_in_one_hash_lookup, a_decode_as_builds_no_array_for_the_object_it_fills, a_decode_as_refuses_a_repeated_key_the_class_does_not_declare]
  - id: render-tests
    stage: 4
    name: escaping scans text once and still replaces an open direction control
    run: [bun, nv, verify, --fast, -p, nvs-render]
    tests: [text_with_nothing_to_escape_is_scanned_once]
  - id: runtime-tests
    stage: 5
    name: a request that sets nothing reads its limits without allocating
    run: [bun, nv, verify, --fast, -p, nvs-runtime]
    tests: [a_request_that_sets_nothing_reads_its_limits_without_allocating]
  - id: config-tests
    stage: 5
    name: a runtime set stays request-local with the resolved table
    run: [bun, nv, verify, --fast, -p, nvs-config]
  - id: server-tests
    stage: 6
    name: "a keep-alive request reads and wakes within its count, and the slow-client deadlines hold"
    run: [bun, nv, verify, --fast, -p, nvs-server]
    tests: [a_keep_alive_request_reads_and_wakes_within_its_count]
  - id: host-tests
    stage: 7
    name: "reads, wakes and pooled stacks make only the system calls they need"
    run: [bun, nv, verify, --fast, -p, nvs-host]
    tests: [a_read_the_reactor_has_not_reported_ready_makes_no_system_call, a_drive_that_wakes_itself_on_its_own_core_writes_no_wake, a_pooled_stack_gives_its_pages_back_when_its_core_is_idle, a_connection_stack_and_a_program_stack_come_from_their_own_pools]
  - id: hostile-groups
    stage: 8
    name: every attack in the areas this goal touched passes
    run: [bun, nv, proofs, --run, --group, "Core\\Json", --group, "Core\\Html", --group, "Core\\Str", --group, "Core\\Budget", --group, "Core\\Config", --group, "Core\\Request", --group, "Core\\Task", --group, "config:directives", --group, "tools:server", --group, "lang:classes", --group, "lang:errors"]
    timeout: 3600
  - id: rules-check
    stage: 8
    name: every rule loads and every citation resolves
    run: [bun, nv, rules, --check]
  - id: memory-row
    stage: 8
    name: "the comparison's memory row is still on-by-default for Novis"
    run: [bun, nv, bench-compare, --scenarios, --row, memory, --expect, "nvs=on-by-default"]
    timeout: 1800
  - id: loop-row
    stage: 8
    name: "the comparison's loop row is still on-by-default for Novis"
    run: [bun, nv, bench-compare, --scenarios, --row, loop, --expect, "nvs=on-by-default"]
    timeout: 1800
---

# A request pays for its own program, and every security check stays where it is

## Why

`bun nv bench-compare --profile <case>` on main (commit bd86f7a9d, release binary, one core under
`-c 64`, no nginx or TLS) finds that Novis's compiled code is a few percent of a request. The rest is
runtime work that the server repeats on every request. Bun's column is from the first profile, taken
by hand at e96f0adec.

| Case | Novis per request | Bun per request | Top self cost | System calls per request (`-c 8`) |
|---|---|---|---|---|
| hello | 27.1 µs | 10 µs | `defaults::in_force` 3.4% and `defaults::spells` 2.9%, then the kernel | `recvfrom` 3.0 (2.0 `EAGAIN`), `write` 1.0, `clock_gettime` 1.0 |
| json-api | 60.9 µs | 18 µs | `json::decode_object` 5.1%, the budget allocator 3.9%, serde 2.1%, `Descriptors::find` 1.1% | `recvfrom` 6.0 (5.0 `EAGAIN`), `write` 2.0, `clock_gettime` 5.0 |
| html-page | 88.4 µs | 34 µs | `format` 4.2%, `bidi::scan` 3.8%, the budget allocator 6.8%, `html::escape` 3.3%, `Descriptors::find` 3.2%, `memory_breach` 2.1% | `recvfrom` 3.0 (2.0 `EAGAIN`), `write` 1.0, `clock_gettime` 1.0 |
| wait | 801.5 µs (it sleeps) | — | `defaults::in_force` 1.9%, the scheduler 1.7% | `recvfrom` 4.1 (3.1 `EAGAIN`), `clock_gettime` 4.3, `epoll_wait` 1.3, `futex` 1.1 |

The `wait` case held 1000 requests in flight twice. The server's cgroup peaked at 86.8 KB per request
over idle in the first pass and at 100.7 KB in the second, so the pools still grow between loads.

- Every `Core` object build looks its class up by name, with a binary search of string compares
  (`crates/nvs-stdlib/src/instance.rs:593`).
- `Json::decodeAs` first builds a generic array with SipHash keys, then reads each field back out
  of it (`crates/nvs-stdlib/src/json.rs:2034`).
- Every request formats configuration keys and default values as text and parses them back
  (`crates/nvs-config/src/request.rs:434`, `crates/nvs-config/src/defaults.rs:595`).
- After a whole request arrives in one read, the connection reads up to five more times and gets
  `EAGAIN` each time (`crates/nvs-host/src/net.rs:1525`). A core wakes itself through the eventfd
  twice per request (`crates/nvs-host/src/block_on.rs:328`).
- Each read zeroes an 8 KiB scratch array on the connection's stack (`crates/nvs-server/src/io.rs:445`).
- A waiting request costs 87–101 KB. Two task stacks per request account for 57 KiB of it, and
  that figure still grows with each round of load. The stack pool keeps every page a stack ever
  touched, and it mixes connection stacks with program stacks (`crates/nvs-host/src/stack.rs:236`).

Every one of these sits next to a security check: the class seal, the JSON depth and memory
ceilings, the Trojan Source bidi check, the per-request configuration overlay, the slow-client
deadlines and the CPU charge per request. `security-hardening` adds more checks to the same code.
Speed and memory rank below security and request isolation, so this goal takes the speed and keeps
every check.

## Standing decisions

**The security floor: no check is removed, weakened or moved behind work an input reaches first.**

- A check may move earlier, for example into a table built once per unit. It may do so only when
  every attack that covers it still fails with the same output.
- A frozen expected output of a hostile case or a conformance case is never edited in this goal. A
  changed message is a regression, not a re-bless.
- Each stage names its guards: the rules, the Rust tests and the hostile cases of its area.
- Each stage ends with a review item. An `Explore` subagent gets the stage's diff and its guard list,
  and nothing else. It names every check that moved, was removed, or now runs later than before.
  Each finding is fixed before the stage's last commit. The subagent is not given the reasoning, so
  its reading is not anchored on the session's.

**Gate.** Stage 1 starts after `security-hardening` is reached, which is when
`docs/goals/security-hardening.md` no longer exists. Its fixes are in the same files:

- stage 3's UTF-8 checks: `crates/nvs-runtime/src/string.rs`, and the lint at `tools/nv/cmd/lints.ts`;
- stage 4's tainted `Str::format` result: `crates/nvs-stdlib/src/str.rs`;
- stage 6's fallible array builds in `crates/nvs-stdlib/src/json.rs`;
- stage 7's connection memory bound: `crates/nvs-server/src/serve.rs`.

This goal builds on those fixes and measures after them. `core-time-slice` is also reached by then.
Its slice and per-turn memory charge are on the paths stages 6 and 7 change.

**Counts are the evidence, never a clock.** A gain is shown by a count:

- allocations per operation in the member ledger (`rule:testing/member-perf-ledger`);
- system calls or wakes per request in a Rust test;
- resident pages of a pooled stack.

Throughput is reported on the comparison page and is never a check (`rule:testing/perf-two-mechanisms`).
No counter goes on a production path for a test. A test counts with a wrapper it supplies, such as
a counting `Read` under `NvsStream`, or with a `cfg(test)` counter in the crate whose own tests read
it.

**The reads and wakes are counted by `nvs_host::counts`, which exists under `debug_assertions` only.**
`nvs-server`'s count test cannot use either shape above. A `cfg(test)` counter in `nvs-host` is
invisible to another crate's tests. A wrapper under `NvsStream` cannot reach a served socket, because
`NvsConnection` names its source types (`crates/nvs-host/src/net.rs:1032`). A cargo feature would build
every crate above `nvs-host` twice. So two thread-local counts sit at the `WouldBlock` read and at
the wake-descriptor poke, and the release binary has neither. Stage 6's `nvs-host` tests read the
same counts. Memory: two `u64` per thread, in debug builds.

**Stage 1: profiling becomes `bun nv bench-compare --profile <case>`.** It was done once by hand,
and this goal does it twice (`AGENTS.md` rule 12). The steps:

1. Start a privileged container from the bench image, with `linux-perf` and `strace`.
2. Serve the case on one pinned core.
3. Load it with the pinned `oha`.
4. Record `perf record -F 1999` for 15 s.
5. Print the top self symbols and the syscalls per request from `strace -c`.

For the `wait` case it also prints the cgroup's peak minus idle, per request in flight. The recipe
is in this goal's first handoff. Safe default if `perf` cannot run in a container on the machine:
the flag prints the recipe and exits 1, and `benches/compare/README.md` documents the steps by hand.

**Stage 2: a descriptor is found with one hash lookup of its name.** Each `Core` class is a
`pub const CLASS: CoreClass`, which is `Copy`, so there is no static to hold a descriptor in. A
const cannot hold a reference to a static with interior mutability either. So `Descriptors` keeps
an `FxHashMap` from each name to the first id defined under it, and `id_of` reads it. Every caller
of `find` gets the lookup, `core_class_desc` included.

- An index by the address of the name literal was built and declined. A debug build copies each
  literal per codegen unit, so the index missed even for `registry::CLASSES`' own entries, and its
  test could pass only in release.
- A compile-time `ClassId` gives the same win but changes the IR, so it is declined.
- A field on `CoreClass` would edit every class literal in the crate for one pointer, so it is declined.
- Turning the class consts into statics would make the address an identity, but it changes the type
  of `CLASSES` and every class literal, so it is declined.
- The map is filled once from the binary's own names and only read after. A name a program looks up
  cannot grow a bucket, so an unkeyed hash is safe.
- The `by_name` index stays for `ids_named`, which marks every id under a name.
- `every_core_sees_one_descriptor_address`, `instance.rs:1182`, stays green.
- So does the host-handle bit, `instance.rs:541`, with its test at `instance.rs:1127`.

Memory: one map entry, a name and an id, per `Core` class name, for the process, which is
O(classes).

**Stage 3: `decodeAs` reads with a `DeserializeSeed` driven by the class's `Contract`.** It keeps
every check of the two-pass path:

- the depth check at the same nesting (`json.rs:1808`, `json.rs:1826`);
- the refusal of a repeated key, for every key, including a key the class does not declare. A bitset
  over the declared fields tracks them, and a small set for undeclared keys is allocated only when
  one appears. That set is charged to the request.
- the budget checks `security-hardening` stage 6 added, for every array and string decode builds;
- the integer range (`json.rs:1793`);
- the stack check before each constructor (`json.rs:3031`);
- the same error message and path for every document.

A field typed `mixed` still goes through the generic visitor. `Json::decode`, which returns an array,
keeps its path. Safe default: if one pass cannot give the same error for a shape the hostile cases
cover, that shape keeps two passes, and `json.rs`'s module doc says why.

**Stage 4: one byte scan finds both an escapable byte and the `0xE2` lead byte.** The bidi analysis
(`crates/nvs-render/src/bidi.rs:127`) runs only when `0xE2` appears. It runs exactly as today, and so
does the U+FFFD replacement (`crates/nvs-render/src/html.rs:115`). The CLI sink
(`crates/nvs-render/src/text.rs:60`) gets the same single scan if it walks twice.

**Stage 4: the memory poll stays on every `Core` member.** `Ctx::memory_breach`
(`crates/nvs-runtime/src/ctx/limits.rs:481`) is the poll every member passes through. Under the
limit it should cost one root-core test and one comparison. Safe default: if the 2% it costs is
that comparison, it stays as it is. The per-allocation charge in `budget::Accounting`
(`crates/nvs-runtime/src/budget.rs:1100`) is not changed. The allocator's share falls because stages
2, 3 and 5 allocate less.

**Stage 5: a unit's snapshot resolves its limits once.** When the configuration snapshot is built,
`[limits]` and `log.access` are resolved into typed values: `memory`, `fatal_reserve`, `cpu_time`,
`fatal_reserve_time`, `wall_time`, `max_script_depth`, `max_output`, `memory_high_water` and
`deprecated`.

- `refresh_limits` reads that table for every key the request's overlay does not hold.
- `Core\Config::set` and `restore` keep calling `refresh_limits` (`crates/nvs-stdlib/src/config.rs:223`).
- A live edit builds a new snapshot with a new table (`rule:config/a-request-keeps-the-unit-it-resolved`).
- `canonical()` returns a static string.
- `in_force` renders each default once per process. The table is fixed by the binary, so this is
  O(rows) and never grows with traffic.
- No cache is keyed by anything a request supplies, so no state crosses requests
  (`rule:security/no-cross-request-state`).

Memory: a few dozen bytes per snapshot.

**Stage 6: a stream keeps a readiness bit, as tokio does.**

- `WouldBlock` clears the bit, and the reactor's event for the token sets it again.
- A read with the bit clear files its deadline, arms, and returns `Pending` with no system call.
- The first read after registration still tries the system call
  (`a_first_read_that_finds_buffered_bytes_never_touches_the_reactor`, `crates/nvs-host/src/net.rs:1780`).
- The header, body and idle deadlines are filed exactly as now (`net.rs:1606`), so
  `a_head_sent_one_byte_at_a_time_is_closed_at_the_header_deadline` (`crates/nvs-server/src/serve.rs:11205`)
  and `the_four_idle_timeouts_are_finite` (`crates/nvs-server/src/io.rs:660`) hold unchanged.
- `rule:concurrency/try-the-syscall-then-park` is rewritten to say this.

**Stage 6: a wake from a drive's own running task on its own core does not use the eventfd.**
The permission is taken as now (`rule:concurrency/one-permission-per-drive`). The task is queued on
the core's local wake list, as `Ended::drop` does (`crates/nvs-host/src/isolate.rs:1046`).
`is_held_by_its_running_task` (`crates/nvs-host/src/reactor.rs:320`) detects it.
`a_future_that_wakes_itself_inside_its_own_poll_is_polled_again` (`crates/nvs-host/src/block_on.rs:387`)
stays green.

**Stage 6: the read scratch moves to the connection.** It becomes a buffer on `ConnectionIo`,
zeroed once per connection. No `unsafe` is used, because `nvs-server` is not on `UNSAFE_CRATES`.
Memory: 8 KiB of heap per open connection, which `rule:concurrency/connection-bounds-are-finite`
bounds. It replaces 8 KiB of stack that every read touched and zeroed.

**The CPU clock is not changed.** The per-thread clock read at a cross-tree switch charges
`[limits] cpu_time` to the right request (`crates/nvs-host/src/watchdog.rs:92`). No read is removed
or coarsened in this goal.

**Stage 7: one pool per role, and pages go back only while a core is idle.**

- Connection stacks and program or child stacks come from separate pools. Each pool is capped as
  the one pool is today (`crates/nvs-host/src/scheduler.rs:1319`, `crates/nvs-cli/src/serve.rs:951`).
- A core that has been idle since its last trim gives back each stack returned since then, except the
  top 8 KiB. It does this at most once a second.
  - Unix uses `madvise(MADV_DONTNEED)`.
  - Windows uses `DiscardVirtualMemory`.
- A trim never runs on each give, which would be a system call per request.
- The `unsafe` block is in `nvs-host`, which is on `UNSAFE_CRATES`, and has its safety comment.
- The corosensei guard page and the armed floor (`crates/nvs-host/src/stack.rs:150`) are not touched.

A trim is not a scrubbing guarantee, and no rule may claim one. Novis code cannot read old stack
bytes because the language is memory-safe.

**One stack per request is declined.** A request can outlive its connection, and the server watches
its wall time after the peer is gone (`crates/nvs-server/src/serve.rs:1311`). The program needs a
task of its own.

**Stage 7: the 32 KiB a sleeping program touches is found.** The session measures the frames on
`start_as_task` → program entry → the sleep helper (`crates/nvs-host/src/isolate.rs:1066`). It fixes
a large frame if one is in Novis's own code. Safe default: the finding goes into `stack.rs`'s
module doc.

**Out of scope, in `### Backlog`:**

- the CPU clock;
- `Core\Math::intDiv` as an out-of-line helper (2% of html-page);
- the isolate's start and finish (about 2.5 µs);
- splitting a literal `Str::format` template while compiling. The template is parsed once per call
  already (`crates/nvs-stdlib/src/format.rs:101`).

**Lanes.**

- Stage 1 runs first, and stage 8 runs after every other stage.
- Stages 2 and 3 share `crates/nvs-stdlib/src/json.rs` only through the encode path's
  `is_instance` (`json.rs:1322`), so they run in order.
- Stages 4 and 5 share `crates/nvs-runtime/src/ctx/limits.rs` and run in order.
- Stages 6 and 7 touch disjoint `nvs-host` files, except a possible idle hook in `scheduler.rs`.
  Stage 7 runs after stage 6.
- Stage 2 or 3 may run as a side goal beside stage 6.

**Trade-offs.**

- Security and request isolation: unchanged by decision. Every check stays, every guard runs, and
  the review item reads each stage's diff.
- Latency and throughput: the profile puts the gain at about 10–13 µs per HTML page, 10–15 µs per
  JSON request, 3–6 µs per request in fixed costs, and 2–6 µs per request in the kernel.
- Simplicity: one more pool role, one readiness bit per stream, one typed table per snapshot, and a
  second decode path for `decodeAs`. That path is the largest addition.
- Memory: about 17 KiB less per waiting request once the pool stops growing. 8 KiB moves from stack
  to heap per open connection. One pointer per `Core` class and a few dozen bytes per snapshot are
  added.

## Stages

### 1. the profile is one command, it runs again on main, and every touched area's attacks are green

**Does:** the profile is repeatable, today's numbers replace the ones in § *Why*, and the security
floor is known to be green before anything changes.

**Split:** the hostile run and the profile run in the background, side by side. The session writes
the command.

- [x] `bun nv bench-compare --profile <case>` and its test, as the standing decision says. `tools/nv/cmd/bench-compare.ts:1`, `tools/nv/test/bench-compare.test.ts:1`, `benches/compare/README.md:1`
- [x] Profile `hello`, `json-api`, `html-page` and `wait` on main, and rewrite § *Why*'s table. Strike every item below whose hot spot is gone. `docs/goals/request-overhead.md:1`
- [x] Run the `hostile-groups` check's command once. A red case is `security-hardening`'s or `core-time-slice`'s, not this goal's: the session stops with `BLOCKED` naming it, because this goal must not start on a red floor. `tools/nv/cmd/proofs.ts:1`
- [x] The count test: a keep-alive connection serves 20 `hello` requests, and the test counts the reads that returned `WouldBlock` and the eventfd wakes per request. It asserts today's numbers as upper bounds, and stage 6 lowers them. `crates/nvs-server/src/serve.rs:10102`

### 2. a Core class's descriptor is found with one hash lookup

**Does:** building a `Core` object, and the encoder's `is_instance` test, find the descriptor with
one hash lookup of the name.

- [x] `Descriptors::id_of` reads a map from each name to its first id, as § *Standing decisions* says (`rule:core-api/core-classes-are-sealed`). `crates/nvs-stdlib/src/instance.rs:417`
  - [x] `Descriptors` fills the map in definition order, so each name keeps its first id
  - [x] `id_of` reads it, so `find`, `descriptor` and `core_class_desc` all do
  - [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green for this crate
- [x] `is_instance` and `build` use it, through `descriptor()`. `crates/nvs-stdlib/src/instance.rs:663`, `crates/nvs-stdlib/src/instance.rs:688`
- [x] Rust test `every_core_class_finds_its_descriptor_in_one_hash_lookup`: for every name, the map's id is the sorted index's first. `crates/nvs-stdlib/src/instance.rs:1208`
- [x] The guards stay green and unchanged: `instance.rs:1127`, `:1182`, `:1355` and `:1423` pass, and so does the `lang:classes` hostile group. `crates/nvs-stdlib/src/instance.rs:1127`
  - [x] `cargo test --lib instance::tests`: 12 passed
  - [x] `bun nv proofs --run --group lang:classes` passes: 90 examples and 20 attacks, none failed
- [x] Review: an `Explore` subagent read the stage's diff against this stage's guards. It found no check moved or removed, and one comment that said request names never reach the map. The comment is fixed. `crates/nvs-stdlib/src/instance.rs:417`

### 3. `Json::decodeAs` fills its class in one pass

**Does:** a document decoded into a class builds no intermediate array, and every refusal is the
same.

- [ ] A `DeserializeSeed` driven by the `Contract` fills each field as serde reads its key (`rule:security/derived-codec-qualifiers`). `crates/nvs-stdlib/src/json.rs:2034`, `crates/nvs-stdlib/src/json.rs:2461`, `crates/nvs-stdlib/src/json.rs:2577`, `crates/nvs-stdlib/src/json.rs:2757`, `crates/nvs-stdlib/src/json.rs:3297`
- [ ] The depth check, the integer range, the constructor's stack check, and the budget checks of `security-hardening` stage 6 are in the new path, at the same points (`rule:http-server/engine-depth-is-bounded-per-decoder`, `rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`). `crates/nvs-stdlib/src/json.rs:1808`, `crates/nvs-stdlib/src/json.rs:1793`, `crates/nvs-stdlib/src/json.rs:3031`
- [ ] A repeated key is refused, declared or not, with the message of `a_repeated_key_is_refused_and_named`. `crates/nvs-stdlib/src/json.rs:1832`, `crates/nvs-stdlib/src/json.rs:4009`
- [ ] Rust tests `a_decode_as_builds_no_array_for_the_object_it_fills` (an allocation count) and `a_decode_as_refuses_a_repeated_key_the_class_does_not_declare`. `crates/nvs-stdlib/src/json.rs:4009`
- [ ] The guards stay green and unchanged. Hostile: `core/Json/decodeAs/01`, `core/Json/decode/01` and `02`, `core/Request/jsonAs/01` and `core/Http-Response/jsonAs/01`. Conformance: `core/a-json-body-past-max-depth-is-refused.nvst`. Rust: `request.rs:7324`. `tests/hostile/core/Json/decodeAs/01-documents-that-do-not-fit-the-class.nvs:1`
- [ ] Review: an `Explore` subagent reads the stage's diff against this stage's guards, and every finding is fixed. `crates/nvs-stdlib/src/json.rs:1`

### 4. escaping scans text once, and a request under its memory limit passes the poll cheaply

**Does:** text with nothing to escape is read once, and the memory poll costs one comparison under
the limit.

- [ ] One byte scan for an escapable byte and `0xE2`, and the bidi analysis only after `0xE2` (`rule:security/bidi-predicate`). `crates/nvs-render/src/html.rs:131`, `crates/nvs-render/src/bidi.rs:127`
- [ ] The CLI sink gets the same scan if it walks twice. `crates/nvs-render/src/text.rs:60`
- [ ] Rust test `text_with_nothing_to_escape_is_scanned_once`, beside `an_unterminated_bidi_control_is_replaced_and_not_referenced`. `crates/nvs-render/src/html.rs:423`
- [ ] Find what `memory_breach` costs under the limit, and make that path a root-core test and one comparison if it is more (`rule:errors/on-limit`). `crates/nvs-runtime/src/ctx/limits.rs:481`
- [ ] The guards stay green and unchanged. Rust: `bidi.rs:162`, `nvs-render/src/html.rs:423`, and `nvs-stdlib/src/html.rs:2300` and `:2462`. Hostile: `core/Html/escape/01` and `core/Cli/escape/01`. Conformance: the `security/bidi-boundaries` cases and `error/one-operation-past-the-ceiling-is-refused-before-it-allocates.nvst`. `crates/nvs-render/src/bidi.rs:162`
- [ ] Review: an `Explore` subagent reads the stage's diff against this stage's guards, and every finding is fixed. `crates/nvs-render/src/html.rs:1`

### 5. a request reads its limits from a table its unit resolved once

**Does:** a request that sets nothing resolves its limits with no allocation and no text parsing.

- [ ] The snapshot's typed limits table, built with the snapshot (`rule:config/the-config-is-an-immutable-snapshot`). `crates/nvs-config/src/snapshot.rs:736`
- [ ] `refresh_limits`, `configured_bytes`, `configured_wall_time` and `report_memory_peak` read the table unless the overlay holds the key. `crates/nvs-runtime/src/ctx/limits.rs:692`, `crates/nvs-runtime/src/ctx/limits.rs:1194`, `crates/nvs-runtime/src/ctx/limits.rs:799`, `crates/nvs-runtime/src/ctx/limits.rs:145`
- [ ] `canonical()` returns a static string, and `in_force` renders each default once per process. `crates/nvs-config/src/request.rs:434`, `crates/nvs-config/src/defaults.rs:595`, `crates/nvs-config/src/defaults.rs:729`
- [ ] `access_on` reads the table, with no new `Request`. `crates/nvs-server/src/trace.rs:316`
- [ ] Rust test `a_request_that_sets_nothing_reads_its_limits_without_allocating`, which uses the allocator's own totals. `crates/nvs-runtime/src/ctx/limits.rs:692`
- [ ] The guards stay green and unchanged. Rust: `nvs-config/tests/request.rs:125` and `:539`, and `nvs-cli/tests/live_edit.rs`. Conformance: `config/config-set-is-invisible-to-the-next-request.nvst` and `core/config-set-memory-above-the-default-moves-the-ceiling-the-runtime-enforces.nvst`. Hostile: the `config:directives` group. `crates/nvs-config/tests/request.rs:125`
- [ ] Review: an `Explore` subagent reads the stage's diff against this stage's guards, and every finding is fixed. `crates/nvs-config/src/request.rs:1`

### 6. a read the reactor has not reported ready, and a wake on a core's own task, make no system call

**Does:** a keep-alive request makes the reads and wakes it needs, and every deadline is filed as
before.

- [ ] The readiness bit on `NvsStream`, and `rule:concurrency/try-the-syscall-then-park` rewritten for it. `crates/nvs-host/src/net.rs:1520`, `crates/nvs-host/src/net.rs:1595`, `crates/nvs-host/src/net.rs:1475`
- [ ] A self-wake on the drive's own core is queued locally (`rule:concurrency/one-permission-per-drive`). `crates/nvs-host/src/block_on.rs:316`, `crates/nvs-host/src/reactor.rs:320`, `crates/nvs-server/src/serve.rs:2127`
- [ ] The read scratch is a per-connection buffer. `crates/nvs-server/src/io.rs:445`, `crates/nvs-server/src/io.rs:120`
- [ ] Rust tests `a_read_the_reactor_has_not_reported_ready_makes_no_system_call` and `a_drive_that_wakes_itself_on_its_own_core_writes_no_wake`. Stage 1's count test is renamed `a_keep_alive_request_reads_and_wakes_within_its_count`, with its bounds lowered to the new counts. `crates/nvs-host/src/net.rs:1820`, `crates/nvs-host/src/block_on.rs:387`, `crates/nvs-server/src/serve.rs:10102`
- [ ] The guards stay green and unchanged. `serve.rs:11151`, `:11205`, `:11285` and `:12953`; `io.rs:660` and `:733`; `reactor.rs:937` to `:1381`; and `block_on.rs:372` to `:459`. `crates/nvs-server/src/serve.rs:11205`
- [ ] Review: an `Explore` subagent reads the stage's diff against this stage's guards, and every finding is fixed. `crates/nvs-host/src/net.rs:1`

### 7. pooled stacks do not grow: one pool per role, and pages go back when a core is idle

**Does:** after repeated rounds of load, a waiting request's stacks stay at their first round's
size, and an idle server gives the pages back.

- [ ] One pool per role, with `take` and `give` by role and a cap per pool (`rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`). `crates/nvs-host/src/stack.rs:159`, `crates/nvs-host/src/scheduler.rs:865`, `crates/nvs-host/src/scheduler.rs:1089`, `crates/nvs-host/src/scheduler.rs:1179`, `crates/nvs-host/src/scheduler.rs:1319`, `crates/nvs-cli/src/serve.rs:951`
- [ ] The idle trim, at most once a second per core, through an audited `unsafe` block on each platform. `crates/nvs-host/src/stack.rs:236`, `crates/nvs-host/src/scheduler.rs:998`
- [ ] Find the 32 KiB a sleeping program touches, and fix it or write it into the module doc. `crates/nvs-host/src/isolate.rs:1066`, `crates/nvs-host/src/stack.rs:1`
- [ ] Rust tests `a_pooled_stack_gives_its_pages_back_when_its_core_is_idle` (resident pages through `mincore` on Linux, and through the platform's query elsewhere) and `a_connection_stack_and_a_program_stack_come_from_their_own_pools`. `crates/nvs-host/src/stack.rs:258`
- [ ] The rule says what the trim costs and that it is not a scrubbing guarantee. `docs/rules/concurrency/a-task-stack-is-reserved-wide-and-pooled.md:1`
- [ ] The guards stay green and unchanged. `stack.rs:258`, `:277`, `:287` and `:345`; `scheduler.rs:1863`, `:1886`, `:1907`, `:1946` and `:2221`; `nvs-codegen/tests/stack_limit.rs`; the state-bleed suite at `serve.rs:8521` to `:8727`; and the `lang:errors` hostile group. `crates/nvs-host/src/scheduler.rs:1886`
- [ ] Review: an `Explore` subagent reads the stage's diff against this stage's guards, and every finding is fixed. `crates/nvs-host/src/stack.rs:1`

### 8. the ledgers and the comparison are recorded again, and the fault rows keep their results

**Does:** the gain is shown in counts and on the comparison page, and the fault rows prove that
the limits still stop a runaway request end to end.

**Split:** after stages 2 to 7 land, the session runs the recordings itself. The page text is the
session's, written to `AGENTS.md` § *Text an end user reads*.

- [ ] Record again the member ledgers of every file stages 2 to 5 changed. Each touched member's allocations per operation are no higher than before. `tools/nv/cmd/proofs.ts:1`
- [ ] `bun nv bench-compare --profile` for the four cases, compared with stage 1's table, and the result written into the handoff. `tools/nv/cmd/bench-compare.ts:1`
- [ ] The comparison recorded again from a detached worktree, and the page's speed and memory paragraphs say what the new run measured. `benches/compare.json:1`, `website/src/content/docs/in-depth/compare.mdx:62`
- [ ] The `hostile-groups`, `memory-row` and `loop-row` checks pass. `tools/nv/cmd/bench-compare.ts:1`

### 9. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. A failing guard of the security floor is fixed in the optimization, never in the guard. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1 and 2 are done: a0f56ab2f and e03aa7e53.

- All four profiles ran on main at bd86f7a9d, and § *Why*'s table holds them: hello 27.1 µs,
  json-api 60.9 µs, html-page 88.4 µs, wait 801.5 µs per request. A waiting request costs 87–101 KB,
  and the second load's peak is higher than the first's. No hot spot is gone yet, so no bullet is struck.
- Stage 2 changed its design. A `Core` class is a `const`, so no static can cache its descriptor.
  An index by the name literal's address missed in debug builds and was declined. `Descriptors` now
  keeps an `FxHashMap` from each name to its first id, and `id_of` reads it. The standing decision,
  the stage's title and the `stdlib-tests` check's test name
  (`every_core_class_finds_its_descriptor_in_one_hash_lookup`) say this now.
- `cargo test --lib instance::tests` passed, 12 tests. The review subagent found no check moved or
  removed. Its one finding, a comment, is fixed.
- `bun nv proofs --run --group lang:classes` passed after the change: 90 examples and 20 attacks.
- The env-secret-proofs check the sweep reported red passes when it is run again (exit 0, nothing
  owed). The nightly run 39746060276 was cancelled by a newer push, not by a failure.
- The proofs job is red on main: 765 perf records are stale since 38c110098. Stage 8 records the
  ledgers again, and `ci-green` passes over it until then.
- Another session has uncommitted edits in `crates/nvs-cli/tests/serve_isolation.rs`. Two
  `eprintln!` lines there fail `cargo clippy --all-targets`. They are not this goal's, so clippy on
  this goal's crates is read with `--keep-going`.

### Next

Stage 3.

- Stage 3: `Json::decodeAs` reads with a `DeserializeSeed` driven by the class's `Contract`, as
  § *Standing decisions* says. Write its checkpoints first. `crates/nvs-stdlib/src/json.rs:2070`,
  `crates/nvs-stdlib/src/json.rs:2613`
- Stage 3's tests: `a_decode_as_builds_no_array_for_the_object_it_fills` and
  `a_decode_as_refuses_a_repeated_key_the_class_does_not_declare`.

No test is expected to go red.

### Backlog

- The CPU clock read at each cross-tree switch (`crates/nvs-host/src/cpuclock.rs:171`) is a real
  system call. The json-api profile shows 5 `clock_gettime` per request. A cheaper clock must still
  charge `[limits] cpu_time` to the right request.
- `Core\Math::intDiv` is an out-of-line helper (`crates/nvs-stdlib/src/math.rs:1825`). The `/`
  operator is already inline (`crates/nvs-codegen/src/emit.rs:1909`).
- The isolate's start and finish cost about 2.5 µs per request (`crates/nvs-host/src/isolate.rs:627`,
  `crates/nvs-host/src/isolate.rs:1250`).
- A literal `Str::format` template could be split while compiling (`crates/nvs-stdlib/src/format.rs:91`).
- The heap's 26–34 KiB per waiting request is not broken down. hyper's buffers and the isolate are
  the likely parts.
- The bench image recompiles from `nvs-types` up on every build while the tree has edits.
  `--profile` could take a prebuilt binary, as `bench-proxied --nvs-bin` does
  (`tools/nv/cmd/bench-compare.ts`).
