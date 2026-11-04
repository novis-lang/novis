---
looper: 1
title: no test or proof depends on how fast the machine is
summary: "every time-limit attack is judged against what the watchdog measured on that machine in that run, the flaky Rust tests and conformance case compare readings of one clock or the order of events, and a rule says how a test holds at any speed"
context: [docs/agent/testing.md, tools/nv/proofs/run.ts, crates/nvs-host/src/watchdog.rs, crates/nvs-host/src/cpuclock.rs]
stages:
  - id: 1
    title: "the rule is written, and the tests it covers are listed"
    status: open
    context: [docs/rules/testing/a-test-has-a-wall-time-limit.md, docs/rules/testing/runner-is-strict.md, data/rules/testing/ci-lanes.json, benches/abi-probe/tests/perf_guards.rs]
  - id: 2
    title: "a time-limit stop is judged against what the watchdog measured in the same run"
    status: open
    context: [crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/abi.rs, crates/nvs-stdlib/src/compress.rs, crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/regex.rs, crates/nvs-host/src/slice.rs]
  - id: 3
    title: "the tests that failed in CI compare readings of one clock or the order of events"
    status: open
    context: [crates/nvs-runtime/src/metrics.rs, crates/nvs-cli/tests/serve_isolation.rs, tests/conformance/config/wall-time-stops-a-loop-that-calls-nothing.nvst, tests/hostile/core/Task/all/03-tasks-that-never-wait.nvs, crates/nvs-host/src/group.rs]
  - id: 4
    title: "the rest of the listed tests follow the rule, and every bound taken out leaves its path under a bench or a count"
    status: open
    context: [benches/abi-probe/tests/perf_guards.rs]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
    context: [tools/nv/cmd/ci.ts, .github/workflows/ci.yml]
checks:
  - id: rule-written
    stage: 1
    name: the rule on how a test holds at any speed is in the rulebook
    run: [bun, nv, peek, "rule:testing/a-test-holds-at-any-speed"]
    timeout: 120
  - id: lateness-recorded
    stage: 2
    name: the watchdog records how late each sweep woke, and the proofs runner judges a stop against it
    run: [cargo, test, --lib, --, "watchdog::tests::"]
    timeout: 1800
    tests:
      - a_sweep_records_how_late_it_woke_against_the_wait_it_computed
      - a_sweep_that_wakes_on_time_records_no_lateness
  - id: overshoot-judged
    stage: 2
    name: the proofs runner explains an overshoot by the measured lateness and one poll, and nothing else
    run: [bun, test, tools/nv/test/proofs-time.test.ts]
    timeout: 300
    tests:
      - an overshoot within the interval, the lateness and one poll is green
      - an overshoot past the interval, the lateness and one poll is red
      - a sweep that computed a long wait explains no overshoot
  - id: time-attacks-green
    stage: 2
    name: the time-limit attacks that failed in CI are green on this machine
    run: [bun, nv, proofs, --verify, --only, "Core\\Compress::compress", "Core\\Arr::sort", "Core\\Regex::replace", "Core\\Task::all", "Core\\Http\\Stream::events", "directive:limits.fatal_reserve_time"]
    timeout: 3600
  - id: watchdog-charge
    stage: 3
    name: the watchdog charges a slice against the thread's own clock reading, not a fixed span
    run: [cargo, test, --lib, --, "watchdog::tests::a_thread_charges_each_request_for_its_own_time_with_slices_on"]
    timeout: 1800
    tests:
      - a_thread_charges_each_request_for_its_own_time_with_slices_on
  - id: metrics-gather
    stage: 3
    name: a gather after a core ended waits for the event that drops it
    run: [cargo, test, --lib, --, "metrics::tests::a_gather_reaches_every_serving_core_and_drops_one_that_ended"]
    timeout: 1800
    tests:
      - a_gather_reaches_every_serving_core_and_drops_one_that_ended
  - id: serve-on-limit
    stage: 3
    name: a looping guest reaches its limit handler on any machine
    run: [cargo, test, --test, serve_isolation, --, a_looping_guest_reaches_on_limit_and_the_next_request_is_served]
    timeout: 1800
    tests:
      - a_looping_guest_reaches_on_limit_and_the_next_request_is_served
---

# no test or proof depends on how fast the machine is

## Why

Between 2026-11-01 and 2026-11-03, about 14 of 22 failed CI jobs on `main` (Miri and fuzz left out)
failed because a machine was slow, not because code was wrong. Each one cost a CI round.

- **Ten were time-limit attacks on `proofs / macos-aarch64`.** `Core\Compress::compress`,
  `Core\Arr::sort`, `Core\Regex::replace`, `Core\Task::all` and `limits.fatal_reserve_time` went
  103 to 142 ms past their CPU-time limit, against a fixed slack of 100 ms
  (`TIME_OVERSHOOT_SLACK_MS`, `tools/nv/proofs/run.ts:328`). The comparator sort has no poll of its
  own: its compiled comparator stops at its own safepoints. So most of the overshoot is the
  watchdog thread waking late. The watchdog samples every 50 ms (`DEFAULT_INTERVAL`,
  `crates/nvs-host/src/watchdog.rs:234`), and on a loaded runner the OS wakes it later than that.
- **Five were Rust tests or a conformance case with a fixed time span**, one failure each:
  - `watchdog::tests::a_thread_charges_each_request_for_its_own_time_with_slices_on` on Windows
  - `metrics::tests::a_gather_reaches_every_serving_core_and_drops_one_that_ended`
  - `serve_isolation::a_looping_guest_reaches_on_limit_and_the_next_request_is_served` on Windows
  - `tests/conformance/config/wall-time-stops-a-loop-that-calls-nothing.nvst` on macOS
  - the cost guard `the_memory_mark_stays_small_beside_the_allocation_it_fronts`

The user's requirement: **a test passes on a machine of any speed, under any load.** Running tests
alone, a test group, a slow mode and retries were all considered and declined below. They help only
with load we create ourselves, or they hide the failure.

## Standing decisions

**The rule.** A test holds at any speed by one of four patterns, and stage 1 writes them as
`rule:testing/a-test-holds-at-any-speed`:

1. **The order of events.** "The limit handler ran, then the next request was served." A wait is for
   the event, and a time bound is only a hang guard, as large as the hostile timeout.
2. **Two readings of the same clock.** A test that checks how much time was charged compares it with
   a reading of the same clock, taken by the test on the same thread. It never compares it with a
   constant like 20 ms. CPU time and wall time are different clocks: a thread can be off its CPU for
   most of a wall-clock span.
3. **A clock the test controls**, where the code under test reads time through a seam that already
   exists (`crate::slice`'s `Clock`, `crates/nvs-host/src/slice.rs:91`). No new clock abstraction is
   added only for a test. The user prefers direct references over mapping layers. A test that needs
   one where none exists uses pattern 1 or 2 instead.
4. **A bound measured in the same run.** Where the property itself is about time, the bound is what
   the machine measured during that run, plus the code's own fixed term. Stage 2 builds this for the
   time-limit attacks.

The rule also states: **a test may drop a time bound only if the path it covers has a bench or a
counted cost guard.** If it has neither, the session adds one before removing the bound.

**How a time-limit stop is judged (stage 2).** The watchdog stays a sampler on its own thread. Every
CPU-clock read is a system call (`crates/nvs-host/src/cpuclock.rs`, *A switch reads the clock with a
system call*), so a read at each poll on the request thread is declined: it puts a system call on
the request path, which latency and throughput forbid. Instead:

- Each sweep records how late it woke: the instant it woke minus the instant the wait it computed
  should have ended. The lateness is read on the watchdog's thread and costs the request path
  nothing. Memory: one duration per sweep in flight, per core.
- A stop's overshoot is explained by: the sweep interval, the measured lateness of the sweeps
  between crossing the limit and the stop, and one poll gap. The poll gap is the fixed term of the
  code: `POLL_EVERY` bytes for compress (`crates/nvs-stdlib/src/compress.rs:819`), one match for
  regex (`crates/nvs-stdlib/src/regex.rs:1876`). The proofs runner reds an overshoot that these do
  not explain. `TIME_OVERSHOOT_SLACK_MS` is replaced by that sum, not raised.
- **The lateness is measured against the wait the code computed**, not against the interval. A code
  change that makes the watchdog sleep longer leaves the lateness at zero, and so the overshoot is
  unexplained and red. A native call that stops polling gives an overshoot no lateness explains, and
  it is red on every machine.
- The lateness reaches the proofs runner on a stderr line that only the runner asks for, through an
  environment variable the runner sets. The `FATAL` message a user reads does not change. If the
  runner already reads such a line for memory readings, the same shape is used. Otherwise
  `tools/nv/proofs/run.ts` gets a new one, and its module doc names it.
- Wall-time readings (`TIME_USED_RE`'s `wall` kind) are judged the same way.

**A safety-net limit in an attack is not the property it measures.** In
`tests/hostile/core/Task/all/03-tasks-that-never-wait.nvs` the 200 ms CPU limit exists to end the
program if the 50 ms deadline is missed. The session finds why the CPU limit fired first on macOS:
the group deadline is read only at a group wake (`crates/nvs-host/src/group.rs:648`), and a spinning
task wakes the group only after a slice yield (`SLICE`, `crates/nvs-host/src/slice.rs:74`). If the
cause is the slice thread waking late, the attack is judged as stage 2 judges a stop. If the cause
is code, it is a bug and is fixed.

**A limit handler that does not run is a cause to find, never a bound to raise.** In the
`serve_isolation` test and the `wall_time` conformance case, `Core\Fatal::onLimit` did not run on a
slow machine. Find out whether `fatal_reserve_time` ran out, the process was killed, or something
else happened. If the reserve ran out for a handler that writes one line, that is a product bug: a
user's handler must not depend on host speed. Then the reserve's default or its accounting is the
fix, and `rule:errors/on-limit` records it with its cost.

**Declined, and written into the rule's Declined list:**
- **Retries** (nextest `retries`, `bun nv ci --rerun` as a fix). They make the build green and hide a
  real stop that overshoots. `rule:testing/runner-is-strict` already says a retry is never green.
- **Running timing tests alone, or in a nextest test group.** It removes only the load we create
  ourselves. A slow runner or a busy developer machine stays flaky.
- **A `--slow` mode** that pins tests to few cores or burns the other cores. It is a flaky way to find
  flaky tests.
- **Raising `TIME_OVERSHOOT_SLACK_MS`** or any fixed bound. It hides a real regression on a fast
  machine and still fails on a slower one.
- **A CPU-clock read on the request thread at each poll.** It adds a system call per poll to the
  request path.
- **A per-thread kernel CPU timer** (`timer_create` on `CLOCK_THREAD_CPUTIME_ID`). It exists on Linux
  only, so macOS and Windows would have a second mechanism CI does not run on Linux.

**What this goal spends.** Nothing on the request path. Per core, one duration per sweep on the
watchdog's thread. A wall-time stop is no longer judged by how close to its limit it stops, only by
whether it stops within its explained bound. Some tests get a bench or a cost guard they did not
have, which adds CI minutes.

**Where the answers are written.** The rule: `docs/rules/testing/a-test-holds-at-any-speed.md` and
`data/rules/testing/a-test-holds-at-any-speed.json`. The first bullet of
`docs/agent/testing.md` § *A test holds on every CI runner* (`docs/agent/testing.md:28`) points to
the rule and stops restating "ten times the time it takes here". The lateness: the watchdog's module
doc (`crates/nvs-host/src/watchdog.rs:1`). The judged bound: `timeOvershoot`'s doc
(`tools/nv/proofs/run.ts:355`).

**The gate pushes and watches CI to green**, macOS and Windows included, with `bun nv ci --push`
and `bun nv ci --watch`, before `DONE`. The local run cannot show a slow runner.

**Lanes.** Stages 2 and 3 both edit `crates/nvs-host/src/watchdog.rs`, so the stages run in order.
Within stage 3 the four fixes touch disjoint files.

## Stages

### 1. the rule is written, and the tests it covers are listed

**Does:** writes the rule with its four patterns, the bench-or-count condition and the Declined list,
and lists every test it covers.

**Split:** the list goes to an `Explore` subagent. It searches `crates/*/tests/`, `#[cfg(test)]`
modules in `crates/*/src/`, `benches/*/tests/` and `tests/conformance/` for an assert on
`elapsed()`, an `Instant` difference or a `Duration` compared with a measured span, and for
`.nvst` cases whose `--EXPECT--` depends on a time limit firing. It skips poll intervals (a
`Duration` passed only to a sleep or a park inside a wait loop). Its answer is `path:line`, one line
each, with the bound.

- [ ] Write `docs/rules/testing/a-test-holds-at-any-speed.md` and its JSON, as § *Standing decisions*
  says. `docs/rules/testing/a-test-holds-at-any-speed.md:1`
- [ ] Replace the first bullet of § *A test holds on every CI runner* with a pointer to the rule.
  `docs/agent/testing.md:28`
- [ ] Render and check the rulebook: `bun nv rules --render`, then `bun nv rules --check`.
- [ ] Write the subagent's list into the handoff's `### Next` under stage 4, grouped by file, each
  entry with the pattern that fixes it. The five tests stage 3 owns are left out.

### 2. a time-limit stop is judged against what the watchdog measured in the same run

**Does:** the watchdog records how late each sweep woke, the proofs runner judges a time stop
against it, and the six time-limit attacks are green.

- [ ] Record each sweep's lateness against the wait it computed, in `Shared::sweep_and_wait`
  (`crates/nvs-host/src/watchdog.rs:1313`) and `next_wait` (`crates/nvs-host/src/watchdog.rs:1551`).
  Write the two unit tests the `lateness-recorded` check names. They drive the computed wait and the
  wake instant directly, so they hold at any speed. `crates/nvs-host/src/watchdog.rs:1313`
- [ ] Put the lateness of the sweeps between the crossing and the stop on the runner-only stderr line,
  where the CPU stop is raised (`crates/nvs-host/src/watchdog.rs:1373`), and the same for a wall-time
  stop. `crates/nvs-host/src/watchdog.rs:1373`
- [ ] Replace `TIME_OVERSHOOT_SLACK_MS` with the explained bound in `timeOvershoot`, and write
  `tools/nv/test/proofs-time.test.ts` with the three tests the `overshoot-judged` check names.
  `tools/nv/proofs/run.ts:355`
- [ ] Find why `Core\Task::all`'s CPU limit fired before its deadline, and fix it as § *Standing
  decisions* says. `tests/hostile/core/Task/all/03-tasks-that-never-wait.nvs:1`
- [ ] Run the `time-attacks-green` check's command once: green.

### 3. the tests that failed in CI compare readings of one clock or the order of events

**Does:** fixes the five tests the CI runs named, each by the pattern its cause needs. Each item first
names the cause in the handoff, then changes the test or the code.

**Split:** the four groups touch disjoint files. The watchdog test runs in the session, since stage 2
just edited its file. The other three go to writing subagents on the session's model, side by side,
none of which builds. The session builds once they all end.

- [ ] The watchdog charge test compares each charge with the thread's own CPU-clock reading over the
  same span (pattern 2), not with `Duration::from_millis(20)`. `crates/nvs-host/src/watchdog.rs:2636`
- [ ] The metrics gather test finds the event that drops an ended core's registry and asserts after
  it (pattern 1). If no such event exists, the gather has a race, and the race is fixed in code.
  `crates/nvs-runtime/src/metrics.rs:1761`
- [ ] `a_looping_guest_reaches_on_limit_and_the_next_request_is_served` and the `wall_time`
  conformance case: find why the limit handler did not run, and fix it as § *Standing decisions*
  says. The conformance case's `--EXPECT--` does not change. `crates/nvs-cli/tests/serve_isolation.rs:381`
- [ ] The cost guard `the_memory_mark_stays_small_beside_the_allocation_it_fronts`: if its figure can
  be a count, it becomes a count. Otherwise record in the handoff why the hard bound held a
  1.05x reading, and fix that bound's cause. `benches/abi-probe/tests/perf_guards.rs:2850`
- [ ] Run each stage-3 check's command once: green.

### 4. the rest of the listed tests follow the rule, and every bound taken out leaves its path under a bench or a count

**Does:** works through the stage-1 list in the handoff, one file group per slice. Each bound taken
out names the bench or cost guard that covers its path, or adds one.

**Split:** each file group of the list goes to a writing subagent on the session's model. Groups in
disjoint files run side by side, and none of them builds. The session builds once they all end.

- [ ] Work through the list, one file group per slice. Each entry names its pattern and, for a
  removed bound, its bench or count. `docs/goals/speed-independent-tests.md:1`
- [ ] Every bench or cost guard added for a removed bound is listed in the handoff with its path.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.
- [ ] Push with `bun nv ci --push` and watch with `bun nv ci --watch` until CI is green on every
  platform.

## Handoff

### State

Written 2026-11-04 from the CI failures of 2026-11-01 to 2026-11-03. Nothing is built yet.

### Next

Stage 1, from its first item: write `docs/rules/testing/a-test-holds-at-any-speed.md`.

### Backlog
