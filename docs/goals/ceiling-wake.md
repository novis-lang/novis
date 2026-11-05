---
looper: 1
title: A request that keeps its default CPU limit never wakes the watchdog
summary: "Reached when a served request that leaves its CPU limit alone makes no watchdog wake and no futex call of its own, and a request that lowers its limit below the next sweep is still stopped on time."
context: [crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-host/src/watchdog.rs, docs/rules/errors/on-limit.md]
stages:
  - id: 1
    title: "a tree's first CPU limit wakes the watchdog only when the next sweep would come too late"
    status: done
    context: [crates/nvs-runtime/src/ctx/limits.rs, benches/compare/profile.sh]
  - id: 2
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: first-limit-wake
    stage: 1
    name: a tree's first CPU limit wakes the watchdog only when it is shorter than one sweep interval, and a lowered limit always does
    run: [cargo, test, --lib, "watchdog::"]
    timeout: 1800
    tests:
      - a_first_limit_longer_than_one_interval_wakes_nothing
      - a_first_limit_shorter_than_one_interval_wakes_the_watchdog
      - a_limit_lowered_from_an_earlier_one_wakes_the_watchdog
---

# A request that keeps its default CPU limit never wakes the watchdog

## Why

A profile of `nvs serve` on one pinned core (`bun nv bench-compare --profile json-api`, 2026-11-05 at
`293a3b9b9`) charged 5.3% of the serving core to `wake_for_lowered_ceiling`
(`crates/nvs-host/src/watchdog.rs:1492`). The watchdog thread, pinned to the same core, took another
8.8%. `strace` counted 2.16 futex calls and 6.99 `clock_gettime` calls per request. The case served
18,041 requests a second, 55.4 µs each. html-page showed the same cost: 4.1% in the wake and 4.6% in the
watchdog thread.

The cause is one condition. `publish_cpu_limit` (`crates/nvs-runtime/src/ctx/safepoint.rs:104`) treats a
tree's first limit as a lowered one: `before == 0` counts as lowered. Every served request starts a tree
with no limit and gets the default `[limits] cpu_time` from `now_serving`
(`crates/nvs-runtime/src/ctx/limits.rs:1004`). So every request notifies the watchdog's condition
variable, and the watchdog sweeps once per request. The wake's own doc says it costs "nothing on a
request that leaves its ceiling alone", and that is not true today.

The wake exists for a real reason. A sweep comes at most `DEFAULT_INTERVAL` (50 ms,
`crates/nvs-host/src/watchdog.rs:249`) after the last one. A limit lowered to less than that could be
crossed before the next sweep reads it. That was the macOS CPU overshoot `ci-green` chased. A 30 s limit
cannot be crossed before the next sweep, so waking for it buys nothing.

This cost is paid by every served request in every program, including `hello`.

## Standing decisions

1. **The rule:** a tree's first limit wakes the watchdog only when that limit is shorter than
   `DEFAULT_INTERVAL`. A limit lowered from an earlier one wakes it, as today. Nothing else changes.
   The stop is exactly as precise as before. A limit at least one interval long is read by the next
   regular sweep, and that sweep already brings its next wake forward to the earliest crossing
   (`next_wait`).
2. **The watchdog side decides, not the runtime.** `nvs-runtime` does not know the interval. The
   `CpuLowered` hook (`crates/nvs-runtime/src/ctx/safepoint.rs:1377`) passes the new limit in
   nanoseconds and whether there was an earlier one. `wake_for_lowered_ceiling` compares it with the
   interval of the watchdog that registered the thread (`Shared::interval`), not with the constant, so
   a test watchdog with a short interval stays right. Safe default, if the hook cannot carry both: keep
   the `before == 0` test in the runtime and pass only the new limit.
3. **The thread CPU clock reads stay.** `strace` shows about 7 `clock_gettime` system calls per request.
   The thread CPU clock has no vDSO path on Linux, so each read is a real system call. They charge each
   request for the time it ran (`rule:errors/on-limit`). Stage 1 removes a read only where it repeats
   one made at the same switch, with nothing run in between. A read that charges a different moment
   stays, whatever it costs. If no read is such a repeat, nothing changes, and the module doc's
   *Charging a request for the time it runs* section says so, with the count.
4. **The floor holds.** `no-overshoot` (`bun nv proofs --overshoot`) is a floor check, and every
   CPU-limit case under `tests/hostile/` must still stop within its bound. macOS is checked by CI on
   the goal-end push.
5. **Measured by counts.** The evidence is the `strace` counts per request and the profile's shares,
   before and after, in the commit message. Requests per second is a secondary note, because the
   machine runs other work.
6. **Lanes:** one. The goal is one stage of work.
7. **A first limit set while the program runs is judged by the time left under it.** Stage 1 found
   this case. `refresh_limits_at_run_time` (`crates/nvs-runtime/src/ctx/limits.rs:893`) already
   reads what the tree has spent. Where no earlier limit stood, it calls the hook again with the
   limit less that reading. So a 2 s limit set after 1.98 s of work still wakes the watchdog. It costs
   nothing on a request that never changes its limit.

## Stages

### 1. a tree's first CPU limit wakes the watchdog only when the next sweep would come too late

**Does:** stops the per-request wake and keeps every stop on time.

- [x] Take the "before" counts: `bun nv bench-compare --profile hello` and `--profile json-api`. Keep
  the futex and `clock_gettime` calls per request, the requests per second, and the shares of
  `wake_for_lowered_ceiling` and the watchdog thread. `benches/compare/profile.sh:1`
- [x] Give the `CpuLowered` hook the new limit and whether an earlier one existed, and call it from
  `publish_cpu_limit` on decision 1's condition. Rewrite the doc comment above it as a whole.
  `rule:errors/on-limit` `crates/nvs-runtime/src/ctx/safepoint.rs:104`
  `crates/nvs-runtime/src/ctx/safepoint.rs:1377`
- [x] Make `wake_for_lowered_ceiling` compare the limit with the registered watchdog's interval, and
  correct its "What it spends" line. `crates/nvs-host/src/watchdog.rs:1492`
- [x] Write the three tests the `first-limit-wake` check names, in `watchdog.rs`'s test module. Each
  counts the notifies one publication causes. `crates/nvs-host/src/watchdog.rs:1492`
- [x] Count the thread CPU clock reads in one served request, with their call sites. Apply decision 3.
  `crates/nvs-host/src/watchdog.rs:427`
- [x] Take the "after" counts with the same two profiles, and put both sets in the commit message.
  Futex calls per request should fall to near zero.

### 2. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Both stages are done, and the goal is reached. `looper verify` was green on its first run (589 s), so
the fix pass had nothing to fix. The goal-end gates are green: `bun nv verify --doc`, `bun nv owners
--closes ceiling-wake` (no gap) and `bun nv ci --check`. `looper check` passed `first-limit-wake` and
the floor's `no-overshoot`.

Decisions 1, 2 and 7 live in the code: `wake_for_lowered_ceiling` and `wakes_for` in
`crates/nvs-host/src/watchdog.rs`, the `CpuLowered` doc in `crates/nvs-runtime/src/ctx/safepoint.rs`,
and the comment in `refresh_limits_at_run_time` (`crates/nvs-runtime/src/ctx/limits.rs`). Decision
3's count is in the watchdog module doc. Stage 1's commit message has the before and after counts.

The sweep's `env-secret-proofs` red was stale. That check belonged to the retired
`defaults-that-work` goal, and `bun nv proofs --gate --only "Core\Env::secret"` is green on this tree.

### Next

Nothing in this goal. macOS runs on CI at the goal-end push. Next on the chain is `json-one-pass`.

### Backlog
