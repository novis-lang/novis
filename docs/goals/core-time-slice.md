---
looper: 1
title: "A request that computes shares its core, so no request stalls its neighbours"
summary: "A request that never waits gives its core back after a time slice on every core of nvs serve and in nvs run, the slice clock sleeps while no core is busy, a request's memory and CPU accounting stay its own across every switch, a deadline is checked at a slice, every rule this goal changes says what is true, the bench's loop row is on-by-default for Novis with no failed request, and the checks below pass."
context: [docs/rules/http-server/a-requests-blast-radius-is-bounded-at-four-tiers.md, crates/nvs-host/src/scheduler.rs, crates/nvs-runtime/src/ctx/safepoint.rs]
stages:
  - id: 1
    title: "the slice clock: a core publishes its turn, and a long turn's request is asked to yield"
    status: done
    context: [crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/watchdog.rs, crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-cli/src/serve.rs, crates/nvs-ext/src/call.rs]
    extra:
      context:
        rules: [http-server/a-requests-blast-radius-is-bounded-at-four-tiers, http-server/a-wedged-core-is-detected-by-its-deadline, concurrency/one-scheduler]
  - id: 2
    title: "the yield: the request gives its core back behind readiness, and neighbours run"
    status: done
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
  - id: 6
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
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
(`crates/nvs-host/src/scheduler.rs:1112`): a relaxed load of the epoch, a sequentially consistent store
into the slot and a sequentially consistent load of the parked flag, no allocation and no
reference-count change on the resume path. The store and load are not relaxed, because with relaxed
ordering the core and the parking thread can each miss the other and the thread sleeps through a turn
that never ends. A slot two epochs old has run 10 ms or more, and the thread
raises the yield bit on the request running there. With every slot clear the thread parks, and the
core that starts the next turn unparks it, so an idle server wakes no thread. This is the wasm guest's
epoch pattern (`crates/nvs-ext/src/call.rs:753`), not the watchdog's 500 ms sweep, which reads every
core's thread clock and is twenty times too coarse (`crates/nvs-host/src/watchdog.rs:215`). The thread
lives in `crates/nvs-host/src/slice.rs` and is started where the watchdog is, then handed to it with
`Watchdog::slice_with`, so every thread that registers takes a seat: each core, the `nvs run` thread
and a worker core that adopts the watchdog.

**The thread finds the running request through the record the watchdog already keeps.** A slot holds
the epoch alone. Its raise locks the watchdog's state, finds the registration's publications, and
raises `YIELD` on the one whose `Charge` has its running bit set, which is the tree
`watchdog::switch_to` named last (`Shared::ask_to_yield` in `crates/nvs-host/src/watchdog.rs`). The
lock is taken only for an old slot, at most once per tick per busy core. A thread with no per-thread
clock publishes nothing, so it gets no seat.

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
no allocation. The armed ceiling, its refusal and its reserve already travel with the task:
`budget::ArmedStack` is swapped in `yield_on` (`crates/nvs-host/src/scheduler.rs:1623`) and around
every `Task::resume` (`crates/nvs-host/src/scheduler.rs:656`), and a spawned context reaches its own
stack through `Ctx::leave_this_stack` and `Ctx::arm_on_this_stack`. The state-bleed suite's
interleaved memory row pins it. So this stage moves the balance only. Safe default if the ceiling
check cannot read a per-request count on its hot path: swap the balance cells `LIVE`, `PEAK` and
`WRITTEN` in `yield_on` beside that state (`crates/nvs-host/src/scheduler.rs:1630`).

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

**Gate.** Stage 3 starts after `failure-paths-hold` stage 2 has landed: `budget::ArmedStack` exists in
`crates/nvs-runtime/src/budget.rs`.

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

- [x] `SafepointFlags::YIELD`, with a doc that says who raises it and why bit 1 stays unused. `crates/nvs-runtime/src/ctx/mod.rs:237`
- [x] `crates/nvs-host/src/slice.rs`: the epoch thread, one slot per core, park while every slot is clear, raise `YIELD` through the running tree's `SafepointView` on a slot two epochs old. `crates/nvs-host/src/slice.rs:1`, `crates/nvs-host/src/watchdog.rs:245`, `crates/nvs-host/src/watchdog.rs:510`
- [x] The scheduler stores the epoch at each resume and clears it at the end of the turn. `crates/nvs-host/src/scheduler.rs:1024`, `crates/nvs-host/src/scheduler.rs:1112`
- [x] `nvs serve` and `nvs run` start the slice thread beside the watchdog. `crates/nvs-cli/src/serve.rs:751`, `crates/nvs-cli/src/main.rs:2239`
- [x] `nvs_safepoint` lowers `YIELD` with the housekeeping bits for now. `crates/nvs-runtime/src/ctx/safepoint.rs:869`
- [x] Rust tests: a slot two epochs old is raised, a short turn is not, the thread parks with every core idle and wakes on the next turn. `crates/nvs-host/src/slice.rs:1`

### 2. the yield: the request gives its core back behind readiness, and neighbours run

**Does:** a request that computes gives its core back every 10 ms, and the reactor polls before it runs
again.

- [x] `Host::yield_slice`, and the `YIELD` branch of `nvs_safepoint` that lowers the bit and calls it inside a `HelperFrame` (`rule:http-server/time-is-bounded-inside-a-helper`). `crates/nvs-runtime/src/host.rs:630`, `crates/nvs-runtime/src/ctx/safepoint.rs:744`, `crates/nvs-runtime/src/abi.rs:421`
- [x] The host's `yield_slice`: a wake and a park on a reactor core, `Waiting::Yielded` on a core with no reactor (`rule:concurrency/the-park-route-is-chosen-once`). `crates/nvs-host/src/group.rs:150`, `crates/nvs-host/src/block_on.rs:256`, `crates/nvs-host/src/scheduler.rs:189`
- [x] Rust test: an I/O-parked task runs between the slices of a task that never waits. `crates/nvs-host/src/scheduler.rs:1904`
- [x] Rust test on one worker: two connections, one request in `while (true)` under a 5 s `cpu_time`, and the other answered while the first is still running. `crates/nvs-cli/src/serve.rs:5311`

### 3. a request's memory stays its own when its core switches between requests

**Does:** two requests that take turns on one core are each charged only their own allocations, and the
memory limit stops the one that crossed it.

- [ ] Charge the balance's change per turn to the running request, and read that count at the ceiling (`rule:security/isolate-shares-nothing`, `rule:errors/on-limit`). `crates/nvs-runtime/src/budget.rs:216`, `crates/nvs-runtime/src/budget.rs:450`, `crates/nvs-runtime/src/ctx/limits.rs:244`, `crates/nvs-host/src/scheduler.rs:1582`
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
- [ ] The session row, gated on `defaults-that-work` stage 7 (`rule:http-server/a-session-is-loaded-once-and-written-whole` shipped): a probe that starts a session, sends `/slow` (writes, then works 2 s) and 200 ms later `/fast` with the same cookie, and times `/fast`. Under 1 s is `built-in`; PHP held by its session lock is `your-code` naming `session_start(['read_and_close' => true])` or `session_write_close()`; Bun and Go are a fixed `your-code`, because neither has sessions without a library. The page gets a "One visitor, two requests" chart of the fast request's wait for Novis and PHP, and a section that explains PHP's lock, its two ways out, and the cost of no lock: two requests of one visitor that change the same value keep the last write. The programs exist. `benches/compare/scenarios/session/main.nvs:1`, `benches/compare/scenarios/session/index.php:1`, `tools/nv/cmd/bench-compare.ts:1`, `website/src/content/docs/in-depth/compare.mdx:1`
- [ ] The comparison re-recorded from a detached worktree, the cases first and then the rows. Novis's `hello` stays within 3% of 28,207, or the slice becomes 20 ms. `benches/compare.json:1`
- [ ] The page's loop paragraph says what the new run measured. `website/src/content/docs/in-depth/compare.mdx:166`

### 6. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1 and 2 are done: a task that never waits now gives its core back every slice. The safepoint's
`YIELD` branch sits below the limit branches and above the cancel branch
(`crates/nvs-runtime/src/ctx/safepoint.rs:873`). It lowers the bit, enters a `HelperFrame` and calls
`Host::yield_slice`, and a `Woken::Cancelled` sets `ctx.cancelled` so the branch below stops the
request. `SchedulerHost::yield_slice` (`crates/nvs-host/src/group.rs:205`) fires
`reactor::wake_this_task()` and parks, or suspends `Waiting::Yielded` with no reactor. The three test
hosts answer `Woken::Elapsed`. The runaway cases in `crates/nvs-cli/src/serve.rs` share
`serve_a_runaway_and_an_answer`, which starts a `SliceClock` beside the watchdog.

Ran once by name, green: `scheduler::tests::a_parked_task_runs_between_the_slices_of_a_task_that_never_waits`,
`serve::tests::a_served_while_true_does_not_stall_a_second_connection_on_its_core` and its sibling
`a_served_while_true_is_ended_as_a_fatal_and_the_core_answers_the_next_request`. The full
`serve-tests` check runs at the gate. Expected to go red: `tests/hostile/core/Task/all/03-tasks-that-never-wait.nvs`,
whose deadline can now fire at a slice. Stage 4 rewrites its expectation.

The sweep's red items were stale again: `bun nv proofs --gate --only 'Core\Env::secret'` exits 0 at
HEAD, and nightly run 39567373487 ran at `afbb1edc2`, before the site fix `b3ed92165`.

### Next

**Stage 3, one group**, on `crates/nvs-runtime/src/budget.rs`, `crates/nvs-runtime/src/ctx/limits.rs`,
`crates/nvs-host/src/scheduler.rs` and `crates/nvs-host/src/watchdog.rs`. Its gate is met:
`failure-paths-hold` is reached and `budget::ArmedStack` exists.

- [ ] Charge the balance's change per turn to the running request, and read that count at the ceiling. `crates/nvs-runtime/src/budget.rs:216`, `crates/nvs-runtime/src/budget.rs:450`, `crates/nvs-runtime/src/ctx/limits.rs:244`, `crates/nvs-host/src/scheduler.rs:1631`
- [ ] The budget module's header says the count is per request across switches. `crates/nvs-runtime/src/budget.rs:27`
- [ ] Rust tests: two interleaved requests that allocate are each charged their own bytes, and only the one over its limit stops. `crates/nvs-runtime/src/budget.rs:1`
- [ ] Rust test: CPU time is still charged per switch with slices on. `crates/nvs-host/src/watchdog.rs:2026`

### Backlog

- Stages 3 to 5 follow in order.
