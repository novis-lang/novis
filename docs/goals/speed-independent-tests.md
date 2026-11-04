---
looper: 1
title: no test or proof depends on how fast the machine is
summary: "every time-limit attack is judged against what the watchdog measured on that machine in that run, the flaky Rust tests and conformance case compare readings of one clock or the order of events, and a rule says how a test holds at any speed"
context: [docs/agent/testing.md, tools/nv/proofs/run.ts, crates/nvs-host/src/watchdog.rs, crates/nvs-host/src/cpuclock.rs]
stages:
  - id: 1
    title: "the rule is written, and the tests it covers are listed"
    status: done
    context: [docs/rules/testing/a-test-has-a-wall-time-limit.md, docs/rules/testing/runner-is-strict.md, data/rules/testing/ci-lanes.json, benches/abi-probe/tests/perf_guards.rs]
  - id: 2
    title: "a time-limit stop is judged against what the watchdog measured in the same run"
    status: done
    context: [crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/abi.rs, crates/nvs-stdlib/src/compress.rs, crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/regex.rs, crates/nvs-host/src/slice.rs]
  - id: 3
    title: "the tests that failed in CI compare readings of one clock or the order of events"
    status: done
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
  - id: stale-cpu-raise
    stage: 3
    name: a CPU raise from a sweep that read the ceiling before the handler's entry does not stop the handler
    run: [cargo, test, --test, limits, --, a_raise_that_read_the_ceiling_from_before_the_handler_does_not_stop_it]
    timeout: 1800
    tests:
      - a_raise_that_read_the_ceiling_from_before_the_handler_does_not_stop_it
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

**One poll gap is `nvs_runtime::POLL_GAP`, one sweep interval of CPU time.** Measuring each gap
would put a clock read on the request path, which is declined. A helper whose batch runs longer than
one interval on a machine holds its core past a stop. That is a latency defect, so the runner reds
it. A CPU bound is `threads * (interval + summed lateness + poll)`. A wall bound is the claim's
lateness from the deadline plus one poll. A run with no sweep line explains no overshoot.

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

**A check of a test's own premise is not a time bound.** Where an assert only checks that sleeps
set up the order the test wants, and the property is asserted separately, the premise check goes
and no bench is owed. `parallel_report_is_in_declaration_order` (`crates/nvs-cli/src/runner.rs`)
keeps its declaration-order assert alone.

**A latency claim with no bench keeps its bound, measured in the run (pattern 4).** The idle-edit
test in `crates/nvs-cli/tests/live_edit.rs` bounds its compile by the check interval, `settle` and
the run's measured boot, which compiled the program once. Dropping it would owe a bench first.

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

- [x] Write `docs/rules/testing/a-test-holds-at-any-speed.md` and its JSON, as § *Standing decisions*
  says. `docs/rules/testing/a-test-holds-at-any-speed.md:1`
- [x] Replace the first bullet of § *A test holds on every CI runner* with a pointer to the rule.
  `docs/agent/testing.md:28`
- [x] Render and check the rulebook: `bun nv rules --render`, then `bun nv rules --check`.
- [x] Write the subagent's list into the handoff's `### Next` under stage 4, grouped by file, each
  entry with the pattern that fixes it. The five tests stage 3 owns are left out.

### 2. a time-limit stop is judged against what the watchdog measured in the same run

**Does:** the watchdog records how late each sweep woke, the proofs runner judges a time stop
against it, and the six time-limit attacks are green.

- [x] Record each sweep's lateness against the wait it computed, in `Shared::sweep_and_wait`
  (`crates/nvs-host/src/watchdog.rs:1313`) and `next_wait` (`crates/nvs-host/src/watchdog.rs:1551`).
  Write the two unit tests the `lateness-recorded` check names. They drive the computed wait and the
  wake instant directly, so they hold at any speed. `crates/nvs-host/src/watchdog.rs:1313`
- [x] Put the lateness of the sweeps between the crossing and the stop on the runner-only stderr line,
  where the CPU stop is raised (`crates/nvs-host/src/watchdog.rs:1373`), and the same for a wall-time
  stop. `crates/nvs-host/src/watchdog.rs:1373`
- [x] Replace `TIME_OVERSHOOT_SLACK_MS` with the explained bound in `timeOvershoot`, and write
  `tools/nv/test/proofs-time.test.ts` with the three tests the `overshoot-judged` check names.
  `tools/nv/proofs/run.ts:355`
- [x] Find why `Core\Task::all`'s CPU limit fired before its deadline, and fix it as § *Standing
  decisions* says. `tests/hostile/core/Task/all/03-tasks-that-never-wait.nvs:1`
- [x] Run the `time-attacks-green` check's command once: green.

### 3. the tests that failed in CI compare readings of one clock or the order of events

**Does:** fixes the five tests the CI runs named, each by the pattern its cause needs. Each item first
names the cause in the handoff, then changes the test or the code.

**Split:** the four groups touch disjoint files. The watchdog test runs in the session, since stage 2
just edited its file. The other three go to writing subagents on the session's model, side by side,
none of which builds. The session builds once they all end.

- [x] The watchdog charge test compares each charge with the thread's own CPU-clock reading over the
  same span (pattern 2), not with `Duration::from_millis(20)`. `crates/nvs-host/src/watchdog.rs:2636`
  - [x] The cause is named: a slice is wall time, so a loaded thread was asked to yield before it ran 20 ms.
  - [x] Each spin burns once at least and returns this thread's own clock reading over it.
  - [x] The test runs green by name.
- [x] The metrics gather test finds the event that drops an ended core's registry and asserts after
  it (pattern 1). If no such event exists, the gather has a race, and the race is fixed in code.
  `crates/nvs-runtime/src/metrics.rs:1761`
- [x] `a_looping_guest_reaches_on_limit_and_the_next_request_is_served` and the `wall_time`
  conformance case: find why the limit handler did not run, and fix it as § *Standing decisions*
  says. The conformance case's `--EXPECT--` does not change. `crates/nvs-cli/tests/serve_isolation.rs:381`
  - [x] The `wall_time` case's cause is found and fixed: the handler's slice is counted from its entry.
  - [x] The serve test's cause is found from evidence, not from reading alone.
  - [x] The serve test's cause is fixed in code, and the test runs green by name.
- [x] The cost guard `the_memory_mark_stays_small_beside_the_allocation_it_fronts`: if its figure can
  be a count, it becomes a count. Otherwise record in the handoff why the hard bound held a
  1.05x reading, and fix that bound's cause. `benches/abi-probe/tests/perf_guards.rs:2850`
- [x] Run each stage-3 check's command once: green.

### 4. the rest of the listed tests follow the rule, and every bound taken out leaves its path under a bench or a count

**Does:** works through the stage-1 list in the handoff, one file group per slice. Each bound taken
out names the bench or cost guard that covers its path, or adds one.

**Split:** each file group of the list goes to a writing subagent on the session's model. Groups in
disjoint files run side by side, and none of them builds. The session builds once they all end.

- [ ] Work through the list, one file group per slice. Each entry names its pattern and, for a
  removed bound, its bench or count. `docs/goals/speed-independent-tests.md:1`
  - [x] The files the first sweep did not read are swept, and their hits join the list.
  - [x] The tight bounds follow the rule: `html_later.rs`, `time.rs`, `io.rs`, `transport.rs`,
    `sqlite_contention.rs`, `telemetry.rs`, `otlp.rs` and the bounded-wait conformance case.
  - [x] The hang guards under the hostile timeout are raised to it.
  - [x] The sleep-ordered conformance cases and Rust tests wait for an event.
  - [ ] The growth properties (`spreadsheet_read.rs:874`, the case-insensitive `indexOf` case)
    have a bench with its growth or a count.
- [ ] Every bench or cost guard added for a removed bound is listed in the handoff with its path.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.
- [ ] Push with `bun nv ci --push` and watch with `bun nv ci --watch` until CI is green on every
  platform.

## Handoff

### State

Stages 1, 2 and 3 are done. In stage 4 the tight bounds, the hang guards and the sleep-ordered
tests are done. The growth properties and one check in `latency.rs` are left. Clippy over all
targets is green, and each rewritten test passed once by name. The five conformance cases passed
under the debug `nvs test`.

- **Sleep-ordered tests and short guards** (`23b798bca`, `c372bb2a6`, `923c46311`, `56e9c5d7f`,
  `b97fd3c02`): conformance cases, slotted fills, LDAP and heartbeat wait on events. Guards in
  `serve.rs`, `socket.rs` and `reactor.rs` are 10 s. `watchdog.rs` reads the thread's own clock.
  `a-wait-parks-only-...` has a new expected line, `short ran while long slept: yes`.
- The handshake cases returned at once here, most likely with no database server, so CI is their
  first real run. `live_edit.rs`'s idle-edit test is renamed and keeps a measured bound.
- **Tight bounds** (`84028a2b9`..`13e8b4a98`): done. Only `time.rs`'s bound was dropped, and
  `benches/members/core/Time/sleep.nvs` covers it. No bench or cost guard was added.
- The run's reds are stale: `env-secret-proofs` is defined nowhere, nightly 41618735447 ran
  `98845d92`, and `supply chain` belongs to `ci-green`. No test is expected to go red.
  `slotted/tests.rs` searches raw chunked bytes, so a needle split across a chunk boundary would
  fail at the 10 s guard. Look there first if it goes red.

### Next

Stage 4 has three entries left. Then its last item is ticked and stage 5 runs.

1. **The growth properties need a bench with its growth or a count.** Keep each bound until one
   exists, and name its path in the handoff.
   - `crates/nvs-ext/tests/spreadsheet_read.rs:874`, the linear-time header `< 10 s`.
   - `tests/conformance/core/str-index-of-case-insensitive-near-miss-is-linear.nvst:7`, bounded by
     the case timeout.
   A bench declares `// bench: complexity linear` with a `.scale.nvs` beside it (playbook,
   `tools/nv/proofs/roster.ts`). A count of comparisons is the alternative where the code has a
   counter to read. Hand each to its own writing subagent.
2. **`crates/nvs-lsp/tests/latency.rs:212-243`, used at `:306`:** check that its hard bound fails
   only after three attempts, as `perf_guards.rs`'s `judge` does. If it does, it stays.
3. **Tick stage 4's last item** once the growth benches are listed here with their paths, and set
   stage 4 done in the front matter.

**Stage 4's list, as swept.** Every entry not named above is done or stays as its kind says. From an `Explore` sweep, so each anchor is a pointer to check. The five tests stage
3 owns are left out. Kinds: a **lower** bound (`elapsed >= X`) only grows on a slow machine, so it holds
at any speed and stays unless the clock's resolution can undercut it. A **hang guard** is pattern 1 and
stays if it is at least the hostile timeout's size. A **tight** bound fails on a slow machine and is
fixed by the named pattern; where it is removed, its path needs a bench or a count first.

- `crates/nvs-stdlib/tests/db_stream.rs`: `:684`, `:712` `< 10 s` against a 30 s statement: hang
  guards, stay.
- `crates/nvs-cli/tests/one_binary.rs:311` `< 50 s` against a 60 s sleep: hang guard, stays.
- `crates/nvs-cli/tests/live_edit.rs`: `:903` compile seen within 5.3 s: pattern 1, wait for the
  compile event, bound as hang guard. `:1032` derives its bound from the measured boot time (`:1008`):
  pattern 4 already, stays.
- `crates/nvs-cli/tests/live_config.rs:2542` `< 20 s` for a 1 s header timeout: hang guard, stays.
- `crates/nvs-cli/tests/serve_isolation.rs:586` `PROBE_BOUND` 10 s per probe: hang guard, stays (this
  file's stage-3 test is a separate slice).
- `crates/nvs-ext/tests/spreadsheet_read.rs:874` linear-time header `< 10 s`: the property is growth,
  so it needs a bench with its growth; pattern 4 or a count of comparisons.
- `crates/nvs-ext/tests/call.rs:415` `< 10 s` for a 50 ms CPU limit: hang guard, stays.
- `crates/nvs-lsp/tests/latency.rs:212-243`, used at `:306`: two-bound judge like `perf_guards.rs`;
  check its hard bound fails only after three attempts, then stays.
- `crates/nvs-ldap/tests/samba.rs:382-386`: a 200 ms deadline must expire before a late peer answers:
  pattern 1, the peer answers only after the client has timed out.
- `crates/nvs-host/src/timer.rs`: `:360`, `:441`, `:659` lower, stay. `:477` `< 1 s` for a 5 s wait:
  hang guard, stays.
- `crates/nvs-host/src/reactor.rs`: `:1214` turn `< 2 s`: pattern 1 if the turn can be counted;
  else hang guard. `:1556` `< 60 s`: hang guard, stays.
- `crates/nvs-host/src/net.rs`: `:2361`, `:2418` lower, stay. `:2413` `< 5 s` for a 50 ms bound:
  hang guard, stays.
- `crates/nvs-host/src/watchdog.rs`: `:2058` CPU burned `< 1 s`: pattern 2, compare with the thread's
  own clock reading. `:2543` charged `>= 20 ms` lower: check against the thread's own reading
  (pattern 2), since stage 3 changes its neighbour at `:2636`.
- `crates/nvs-runtime/src/ctx/safepoint.rs:1702` lower, stays.
- `crates/nvs-stdlib/src/cache.rs:3975` lower, stays.
- `crates/nvs-stdlib/src/cli.rs:4703` 20 ms wait `< 5 s`: hang guard, stays.
- `crates/nvs-stdlib/src/db/execute.rs`: `:1767` lower, stays. `:1828` `< 5 s` for 200 ms: hang guard,
  stays.
- `crates/nvs-stdlib/src/process.rs`: `:2973`, `:3022`, `:3105` `< 20 s`: hang guards, stay.
- `crates/nvs-stdlib/src/http.rs:8483` `< 10 s` against a 20 s timeout: hang guard, stays.
- `crates/nvs-stdlib/src/http/socket.rs`: `:1231` `< 3 s`: hang guard, raise to the hostile timeout.
  `:1275` `> 1 s` with 50 ms pings: pattern 1, count the pings the peer answered. `:1532` `< 5 s`:
  hang guard, stays.
- `crates/nvs-stdlib/src/http/transport.rs`: `:4324` `< 5 s` budget and `:5243`, `:5379`: hang guards,
  stay.
- `crates/nvs-db/src/sqlite.rs:2248` `< 10 s` for 100 ms: hang guard, stays.
- `crates/nvs-server/src/serve.rs`: `:5657`, `:5686`, `:5858`, `:12063`, `:12137`, `:12247`, `:12345`
  `< 5 s`: hang guards; raise each to the hostile timeout where 5 s is under it.
- `crates/nvs-test/src/run.rs`: `:1093` `< 20 s`, `:1157` `< 8 s`: hang guards, stay.
- `crates/nvs-cli/src/script.rs:3857` revalidate wait `<= 300 ms`: pattern 3 if the settle wait reads
  a seam; else pattern 1.
- `crates/nvs-cli/src/runner.rs:6763` compares two children's elapsed: pattern 1, order of events.
- `benches/abi-probe/tests/wasm_async.rs:285` lower, stays.
- `tests/conformance/config/wall-time-ends-a-sleep-that-would-pass-it.nvst:6`, `isolate/a-worker-placed-
  child-that-never-ends-is-stopped-by-its-trees-cpu-time.nvst:11`, `error/a-loop-that-allocates-nothing-
  is-stopped-by-the-cpu-ceiling.nvst:35`, `error/a-cpu-time-narrowed-by-config-set-stops-a-loop-that-
  calls-nothing.nvst:14`, `lang/a-test-past-its-wall-time-limit-fails-and-names-the-limit.nvst:20`: a
  limit fires and the expected output names it; they hold at any speed unless the handler misses, which
  is stage 3's cause.
- `tests/conformance/isolate/a-deadline-stops-a-task-that-never-waits.nvst:22`,
  `task/a-deliberate-deadlock-is-ended-by-the-deadline-and-leaves-no-child-running.nvst:37`: a 20 ms
  sleep gives survivors a turn: pattern 1, wait for an event the survivors would cause.
- `tests/conformance/task/a-wait-parks-only-the-calling-task-and-answers-a-value.nvst:33`,
  `task/close-wakes-a-waiting-receiver-and-what-was-queued-still-arrives.nvst:26`,
  `core/late-init-a-second-task-waits-for-the-first.nvst:17`: ordering by sleep lengths: pattern 1.
- `tests/conformance/core/str-index-of-case-insensitive-near-miss-is-linear.nvst:7`: the case timeout
  is the bound on a growth property: needs a bench with its growth, or a count of comparisons.
- `core/time-sleep-is-bounded-at-zero-and-waits-in-whatever-unit-it-is-given.nvst:59` lower, and
  `core/a-cancelled-task-dies-where-it-sleeps.nvst:24`, `core/late-init-a-failed-init-is-retried-by-
  the-next-read.nvst:70`, `task/a-task-tree-dies-with-its-parent.nvst:41` (20 ms against 2 s): stay.

From the second sweep, of the files the first did not read. `nvs-stdlib/tests/queue.rs`,
`nvs-db/tests/pool_reuse.rs`, `nvs-ext/src` and the benches other than `abi-probe` have no hits.

- `crates/nvs-db/tests/handshake.rs`: `:1493`, `:1516`, `:1536`, `:1559` sleep 2 s past a 2 s
  handshake budget, then the statement must still reach the server: pattern 1, wait for an event that
  shows the budget is over. `:706`, `:949` sleep 10 ms per turn, up to 400 turns, until a read is in
  flight: a polling wait, check that its turn count is a hang guard and the assert reads an event.
- `crates/nvs-cli/src/service.rs`: `:6155`, `:6177` 10 s patience against a 5 ms ping: hang guards,
  stay. `:6164`, `:6166`, `:6184` sleep 100 ms, then assert no new ping: an absence judged after a
  sleep, so pattern 1 needs an event that ends the window, such as a later ping the test controls.
- `crates/nvs-server/src/slotted/tests.rs`: `:121` and `:152` a 300 ms slot must still run when the
  shell arrives; `:165-177` a 30 ms slot ends before a 300 ms one; `:38`, `:484-509` a 60 ms slot
  ends after two others: all sleep-ordered, pattern 1, each slot waits on a gate the test opens.
  `:518` not read yet.

### Backlog

- A served request that ends in a `FATAL` writes nothing to the server's stderr
  (`crates/nvs-cli/tests/serve_isolation.rs:401` can only print the status). A floor line per served
  `FATAL` would let a test tell a stopped handler from a skipped one. It needs its cost per request
  and its coalescing, as `rule:http-server/the-floor-cannot-fill-the-disk` says.

- `crates/nvs-runtime/src/budget.rs` § *What it spends* (near `:205`) says a balance update is "a
  small fraction of a platform heap one". On glibc that holds only while `carry` is inlined, and the
  section could say so.
