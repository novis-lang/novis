---
looper: 1
title: "A request that computes shares its core, so no request stalls its neighbours"
summary: "A request that never waits gives its core back after a time slice on every core of nvs serve and in nvs run, the slice clock sleeps while no core is busy, a request's memory and CPU accounting stay its own across every switch, a deadline is checked at a slice, every rule this goal changes says what is true, the bench's loop row is on-by-default for Novis with no failed request, and the checks below pass."
context: [docs/rules/http-server/a-requests-blast-radius-is-bounded-at-four-tiers.md, crates/nvs-host/src/scheduler.rs, crates/nvs-runtime/src/ctx/safepoint.rs]
stages:
  - id: 1
    title: "the slice clock: a core publishes its turn, and a long turn's request is asked to yield"
    status: open
    context: [crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/watchdog.rs, crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-cli/src/serve.rs, crates/nvs-ext/src/call.rs]
    extra:
      context:
        rules: [http-server/a-requests-blast-radius-is-bounded-at-four-tiers, http-server/a-wedged-core-is-detected-by-its-deadline, concurrency/one-scheduler]
  - id: 2
    title: "the yield: the request gives its core back behind readiness, and neighbours run"
    status: open
    context: [crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/host.rs, crates/nvs-runtime/src/abi.rs, crates/nvs-host/src/group.rs, crates/nvs-host/src/block_on.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-cli/src/serve.rs]
    extra:
      context:
        rules: [concurrency/the-park-route-is-chosen-once, concurrency/the-parking-contract, http-server/time-is-bounded-inside-a-helper]
  - id: 3
    title: "a request's memory stays its own when its core switches between requests"
    status: open
    context: [crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/watchdog.rs]
    extra:
      context:
        rules: [security/isolate-shares-nothing, errors/on-limit]
  - id: 4
    title: "a deadline stops a task that never waits, and the concurrency rules say so"
    status: open
    context: [docs/rules/concurrency/limit-and-deadline-are-the-only-bounds.md, docs/rules/concurrency/on-worker-runs-the-child-on-another-core.md, docs/rules/concurrency/the-parking-contract.md, tests/hostile/core/Task/all/03-tasks-that-never-wait.nvs]
    extra:
      context:
        rules: [concurrency/limit-and-deadline-are-the-only-bounds, concurrency/on-worker-runs-the-child-on-another-core, concurrency/the-parking-contract]
  - id: 5
    title: "the rule, the attack, and the comparison's loop row"
    status: open
    context: [docs/rules/http-server/a-requests-blast-radius-is-bounded-at-four-tiers.md, tools/nv/cmd/bench-compare.ts, benches/compare/README.md, website/src/content/docs/in-depth/compare.mdx]
    extra:
      context:
        rules: [http-server/a-requests-blast-radius-is-bounded-at-four-tiers, testing/hostile-case-contract]
checks:
  - id: host-tests
    stage: 3
    name: the scheduler, slice and budget tests pass
    run: [bun, nv, verify, --fast, -p, nvs-host]
  - id: serve-tests
    stage: 2
    name: a request that loops does not stall its neighbour on one worker
    run: [bun, nv, verify, --fast, -p, nvs-cli]
  - id: rules-check
    stage: 5
    name: every rule loads and every citation resolves
    run: [bun, nv, rules, --check]
  - id: loop-row
    stage: 5
    name: the comparison's loop row is on-by-default for Novis with no failed request
    run: [bun, nv, bench-compare, --scenarios, --row, loop, --expect, "nvs=on-by-default"]
    timeout: 1800
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, core-time-slice]
    reach: true
    floor: false
---

# A request that computes shares its core, so no request stalls its neighbours

## Why

A core runs its requests as coroutines and switches only when one waits (`crates/nvs-host/src/scheduler.rs:1-28`).
A request that only computes never waits, so it keeps its core until `[limits] cpu_time` stops it, 30
seconds by default. Every other request on that core waits that long. On a one-core server the whole
server stops. The comparison bench measured it on 2026-10-21: under steady load with one runaway loop,
Novis kept 12% of its throughput and 16 requests timed out, where PHP kept 75%
(`benches/compare.json`, row `loop`). One request per core every 30 seconds keeps a server down.

This breaks tier B of `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`: "No request
causes another to fail, stall, or observe its data." Security and request isolation is the first
priority, so this goal comes before every goal after the live one.

The pieces are already there. Compiled code polls one word at every function entry and loop back edge
(`crates/nvs-codegen/src/emit.rs:1192`), and the scheduler documents `Waiting::Yielded` as "what a
long-running task does at a safepoint so that its neighbours are not starved"
(`crates/nvs-host/src/scheduler.rs:189`). Nothing raises it. Only a wasm guest has a real time slice
(`crates/nvs-ext/src/call.rs:109`, `rule:packaging/a-guest-call-yields-on-its-core`).

## Standing decisions

**The slice is 10 ms, a constant, with no configuration key.** Go preempts at 10 ms, and a slice is
long next to a switch's cost. A key is a second thing to explain, and the user's direction is no new
limits. Safe default if stage 5's throughput check loses more than the page's 3% margin on `hello`:
lengthen the constant to 20 ms and write the measured trade-off in the new rule.

**One slice thread per process, which sleeps while no core is in a turn.** It keeps an epoch counter
that ticks every 5 ms while any core is busy. A core stores the current epoch in its own slot when it
resumes a task (`crates/nvs-host/src/scheduler.rs:1024`) and clears the slot when the turn ends
(`crates/nvs-host/src/scheduler.rs:1112`): one relaxed load and one relaxed store, no allocation and no
reference-count change on the resume path. A slot two epochs old has run 10 ms or more, and the thread
raises the yield bit on the request running there. With every slot clear the thread parks, and the
core that starts the next turn unparks it, so an idle server wakes no thread. This is the wasm guest's
epoch pattern (`crates/nvs-ext/src/call.rs:753`), not the watchdog's 500 ms sweep, which reads every
core's thread clock and is twenty times too coarse (`crates/nvs-host/src/watchdog.rs:215`). The thread
lives in a new `crates/nvs-host/src/slice.rs` and is started where the watchdog is
(`crates/nvs-cli/src/serve.rs:751`, and for `nvs run` at `crates/nvs-cli/src/main.rs:2239`).

**The thread finds the running request through the record the watchdog already keeps.**
`watchdog::switch_to` is called before every resume with the task's tree
(`crates/nvs-host/src/watchdog.rs:510`), and each published tree has a `SafepointView`
(`crates/nvs-host/src/watchdog.rs:245`). If that record is thread-local only, the core's slot carries
the running tree's index in the same store as the epoch. Safe default if neither fits in one word: two
relaxed stores, and the cost written in the module doc.

**The yield is a new safepoint bit, `YIELD`, the next free high bit.** Bit 1 stays unused, as the flag
set's doc explains (`crates/nvs-runtime/src/ctx/mod.rs:233`). The bit is the request tree's like every
other, so a placed child on another core may also yield once. That costs one turn and is accepted. No
codegen change: the poll still branches on a non-zero word, and
`a_loop_back_edge_polls_with_one_load` (`crates/nvs-codegen/tests/probes.rs:125`) stays green.

**The yield goes behind readiness, never to the front of the run queue.** A bare `Waiting::Yielded`
keeps `Scheduler::run` from returning to the reactor while a spinner re-queues itself
(`crates/nvs-host/src/scheduler.rs:998`), so the accept loop and every connection would starve. On a
reactor core the yield fires a wake and parks, as `yield_turn` does
(`crates/nvs-host/src/block_on.rs:256`). On a core with no reactor it is `Waiting::Yielded`.

**The yield is suspended inside a `HelperFrame`.** `nvs_safepoint` enters none today
(`crates/nvs-runtime/src/ctx/safepoint.rs:745`), so a suspend from it would look unwindable
(`crates/nvs-host/src/scheduler.rs:1602`) and a teardown could force-unwind through JIT frames. The
`YIELD` branch lowers the bit, enters a frame the way `run_helper` does
(`crates/nvs-runtime/src/abi.rs:421`), and calls a new `Host::yield_slice`
(`crates/nvs-runtime/src/host.rs:630`), because `nvs-runtime` cannot name `nvs-host`.

**A request's memory is charged per turn.** The budget's balance is per thread and "exact for the one
request a thread runs at a time" (`crates/nvs-runtime/src/budget.rs:27`). An await already interleaves
requests, and a slice makes it common. At every switch, the change in the balance since the turn began
is added to the running request's own count, and the memory limit reads that count. O(1) per switch,
no allocation. Safe default if the ceiling check cannot read a per-request count on its hot path: swap
the five budget thread-locals in `yield_on` beside `CurrentStack` (`crates/nvs-host/src/scheduler.rs:1599`).

**A deadline now stops a task that never waits.** Its timer is on the core's reactor, and a slice
hands the core back, so the timer fires at the next slice
(`rule:concurrency/limit-and-deadline-are-the-only-bounds`). The hostile case that pins the old
behaviour, `tests/hostile/core/Task/all/03-tasks-that-never-wait.nvs`, changes its expected output
because the rule changes. That is not editing a frozen output to pass, and the commit says so.

**The attack is a Rust test if the hostile harness cannot hold two requests at once.** It goes beside
`a_runaway_then_an_answer` (`crates/nvs-cli/src/serve.rs:5311`) and is named in the new rule's
`guardedBy`. The behaviour has no syntax and no `Core` member, so it has no feature id and owes no
examples or `about.md`.

**`bench-compare` gains `--expect <peer>=<result>`,** which exits 1 when a row's result for that peer
differs. It is how the `loop-row` check stops. It needs Docker and minutes, so it is not part of
`nv verify` or CI.

**Out of scope:** the wasm guest's own epoch yield stays as it is. Unifying the two clocks is a later
refactor with no user-visible change.

**Lanes.** One lane. Stages 1, 2 and 3 share `crates/nvs-host/src/scheduler.rs` and run in order.
Stage 4 needs stage 2's yield to observe anything. Stage 5 needs all of them.

**Trade-offs.** Request isolation gains: a neighbour waits at most a slice per request ahead of it,
not 30 seconds. Latency: a request that computes pays one reactor poll per 10 ms, well under 1%, and the
resume path gains one relaxed load and one store. Memory: one slot per core and one thread per process,
O(cores), and the thread is parked while the server is idle. Simplicity: no new key and no new syntax.
The scheduler gains one waiting reason it already documents.

## Stages

### 1. the slice clock: a core publishes its turn, and a long turn's request is asked to yield

**Does:** a request that has run 10 ms has `YIELD` raised on its tree's word. Stage 1 only lowers it
again at the safepoint, so nothing yields yet and every existing test stays green.

- [ ] `SafepointFlags::YIELD`, with a doc that says who raises it and why bit 1 stays unused. `crates/nvs-runtime/src/ctx/mod.rs:237`
- [ ] `crates/nvs-host/src/slice.rs`: the epoch thread, one slot per core, park while every slot is clear, raise `YIELD` through the running tree's `SafepointView` on a slot two epochs old. `crates/nvs-host/src/slice.rs:1`, `crates/nvs-host/src/watchdog.rs:245`, `crates/nvs-host/src/watchdog.rs:510`
- [ ] The scheduler stores the epoch at each resume and clears it at the end of the turn. `crates/nvs-host/src/scheduler.rs:1024`, `crates/nvs-host/src/scheduler.rs:1112`
- [ ] `nvs serve` and `nvs run` start the slice thread beside the watchdog. `crates/nvs-cli/src/serve.rs:751`, `crates/nvs-cli/src/main.rs:2239`
- [ ] `nvs_safepoint` lowers `YIELD` with the housekeeping bits for now. `crates/nvs-runtime/src/ctx/safepoint.rs:869`
- [ ] Rust tests: a slot two epochs old is raised, a short turn is not, the thread parks with every core idle and wakes on the next turn. `crates/nvs-host/src/slice.rs:1`

### 2. the yield: the request gives its core back behind readiness, and neighbours run

**Does:** a request that computes gives its core back every 10 ms, and the reactor polls before it runs
again.

- [ ] `Host::yield_slice`, and the `YIELD` branch of `nvs_safepoint` that lowers the bit and calls it inside a `HelperFrame` (`rule:http-server/time-is-bounded-inside-a-helper`). `crates/nvs-runtime/src/host.rs:630`, `crates/nvs-runtime/src/ctx/safepoint.rs:744`, `crates/nvs-runtime/src/abi.rs:421`
- [ ] The host's `yield_slice`: a wake and a park on a reactor core, `Waiting::Yielded` on a core with no reactor (`rule:concurrency/the-park-route-is-chosen-once`). `crates/nvs-host/src/group.rs:150`, `crates/nvs-host/src/block_on.rs:256`, `crates/nvs-host/src/scheduler.rs:189`
- [ ] Rust test: an I/O-parked task runs between the slices of a task that never waits. `crates/nvs-host/src/scheduler.rs:1904`
- [ ] Rust test on one worker: two connections, one request in `while (true)` under a 5 s `cpu_time`, and the other answered while the first is still running. `crates/nvs-cli/src/serve.rs:5311`

### 3. a request's memory stays its own when its core switches between requests

**Does:** two requests that take turns on one core are each charged only their own allocations, and the
memory limit stops the one that crossed it.

- [ ] Charge the balance's change per turn to the running request, and read that count at the ceiling (`rule:security/isolate-shares-nothing`, `rule:errors/on-limit`). `crates/nvs-runtime/src/budget.rs:216`, `crates/nvs-runtime/src/budget.rs:450`, `crates/nvs-runtime/src/ctx/limits.rs:244`, `crates/nvs-host/src/scheduler.rs:1552`
- [ ] The budget module's header says the count is per request across switches. `crates/nvs-runtime/src/budget.rs:27`
- [ ] Rust tests: two interleaved requests that allocate are each charged their own bytes, and only the one over its limit stops. `crates/nvs-runtime/src/budget.rs:1`
- [ ] Rust test: CPU time is still charged per switch with slices on, beside `a_thread_charges_each_published_request_only_for_the_time_it_runs_it`. `crates/nvs-host/src/watchdog.rs:2026`

### 4. a deadline stops a task that never waits, and the concurrency rules say so

**Does:** `Task::all` with a deadline stops two tasks that never wait at about the deadline, and the
rules describe a core that is shared by slices.

- [ ] Rewrite the rule's "fires when the core is handed back" paragraph: a slice hands it back too (`rule:concurrency/limit-and-deadline-are-the-only-bounds`). `docs/rules/concurrency/limit-and-deadline-are-the-only-bounds.md:9`
- [ ] Rewrite the parent-waits lines for a child that computes (`rule:concurrency/on-worker-runs-the-child-on-another-core`). `docs/rules/concurrency/on-worker-runs-the-child-on-another-core.md:58`
- [ ] Rule 4 of the parking contract names the slice beside a park (`rule:concurrency/the-parking-contract`). `docs/rules/concurrency/the-parking-contract.md:14`
- [ ] The hostile case's expectation: the deadline stops both tasks. `tests/hostile/core/Task/all/03-tasks-that-never-wait.nvs:1`
- [ ] A conformance case: a deadline stops a task that never waits. `tests/conformance/isolate/a-deadline-stops-a-task-that-never-waits.nvst:1`

### 5. the rule, the attack, and the comparison's loop row

**Does:** the guarantee is a shipped rule with its attack, and the comparison shows it.

**Split:** after stages 1 to 4 land, the session runs the bench itself. The page text is the session's.

- [ ] The new rule `http-server/a-request-that-computes-shares-its-core`: the slice, the clock, the cost, and its Declined list (a watchdog-driven slice, a configuration key, preemption by signal). Tier B's row cites it. `docs/rules/http-server/a-request-that-computes-shares-its-core.md:1`, `data/rules/http-server/a-request-that-computes-shares-its-core.json:1`, `docs/rules/http-server/a-requests-blast-radius-is-bounded-at-four-tiers.md:6`
- [ ] `rule:http-server/time-is-bounded-inside-a-helper` says the poll also yields. `docs/rules/http-server/time-is-bounded-inside-a-helper.md:1`
- [ ] The attack: one request loops on a one-worker server while another is answered, named in the rule's `guardedBy` (`rule:testing/hostile-case-contract`). `crates/nvs-cli/src/serve.rs:5466`
- [ ] `bench-compare --expect <peer>=<result>` and its test. `tools/nv/cmd/bench-compare.ts:1494`, `tools/nv/test/bench-compare.test.ts:1`
- [ ] The comparison re-recorded from a detached worktree, the cases first and then the rows. Novis's `hello` stays within 3% of 28,207, or the slice becomes 20 ms. `benches/compare.json:1`
- [ ] The page's loop paragraph says what the new run measured. `website/src/content/docs/in-depth/compare.mdx:166`

## Handoff

### State

The goal is new. Nothing is built. The comparison bench's `loop` row records the stall as `no` for
Novis (`benches/compare.json`, 2026-10-21).

### Next

**Stage 1, one group**, on `crates/nvs-host/src/scheduler.rs`, the new `crates/nvs-host/src/slice.rs`
and `crates/nvs-runtime/src/ctx/mod.rs`. No gate.

- [ ] `SafepointFlags::YIELD`. `crates/nvs-runtime/src/ctx/mod.rs:237`
- [ ] The slice thread and its slots. `crates/nvs-host/src/slice.rs:1`, `crates/nvs-host/src/watchdog.rs:510`
- [ ] Store and clear the epoch around each turn. `crates/nvs-host/src/scheduler.rs:1024`, `crates/nvs-host/src/scheduler.rs:1112`
- [ ] Start the thread in `nvs serve` and `nvs run`. `crates/nvs-cli/src/serve.rs:751`, `crates/nvs-cli/src/main.rs:2239`
- [ ] Lower `YIELD` at the safepoint for now. `crates/nvs-runtime/src/ctx/safepoint.rs:869`
- [ ] The slice thread's Rust tests. `crates/nvs-host/src/slice.rs:1`

### Backlog

- Stages 2 to 5 follow in order.
