# Handoff

## State

**Goal `resource-ceilings` — every resource ceiling stops the request that breaks it. Stages 0 and 2
are landed, and stage 3's three items are now on disk: a core publishes what a request can be charged
against, the watchdog's own walk samples it, and a breach raises both polls.** Goal `editor-surfaces`'s
acceptance list is still this goal's floor and is untouched.

`RunningRequest` (`crates/nvs-host/src/watchdog.rs:174`) is what a core publishes — the tree's
`SafepointView`, the `cpu_limit` nanoseconds, and the `ThreadClock::burned` reading at that instant.
`RunningRequest::new` answers `None`, meaning publish nothing and sample nothing, for a request under no
cap and for a platform with no per-thread clock. `Shared::sweep`
(`crates/nvs-host/src/watchdog.rs:507`) reads the charge on the walk it already makes, one clock read
per core that published, and a request at its ceiling gets both polls raised under the lock — a store
into a word cannot block a core the way the stall sink can.

`SafepointView` (`crates/nvs-runtime/src/ctx/safepoint.rs:233`) carries both of the tree's words for
that reason: the flag compiled code reads between calls, and the deadline `bounded_loop` reads from
inside one long call. `deadline_passed` (`crates/nvs-runtime/src/abi.rs:345`) reads the flags to name
which ceiling stopped a walk, so a CPU breach inside a member no longer reports a deadline that never
passed.

**Nothing publishes outside the watchdog's own tests, which is why stage 0's three cases are still
red** — the sampler walks an empty set. Stage 3's remaining work is the publisher, and the handoff's
next group is it.

Known gap, written where it bites in `crates/nvs-runtime/src/ctx/safepoint.rs`'s CPU arm: a publication
carries the ceiling that stood before `Ctx::run_limit_handler` widened it by `fatal_reserve_time`, so a
tier-1 handler is bounded by the next sweep rather than by its reserve. What fixes it is what the
publisher hands over.

## Next group

**Stage 3: the publisher — a core publishes the request it takes up** — one file set,
`crates/nvs-cli/src/serve.rs` with `crates/nvs-host/src/watchdog.rs` for the type it publishes.
`rule:errors/on-limit` is the ceiling all three serve.

- [ ] **Settle which run paths get a watchdog at all** — `crates/nvs-cli/src/serve.rs:383` builds the
      process's `Watchdog` and `crates/nvs-cli/src/serve.rs:793` is the only production `register`, so
      `nvs serve` has one and it is not established that `nvs run` or `nvs test` does. Stage 0's case
      `tests/conformance/error/a-loop-that-allocates-nothing-is-stopped-by-the-cpu-ceiling.nvst` runs
      under the test runner, not the server, so the answer decides this group's file set. One
      `grep -rn "Watchdog" crates/nvs-cli/src` settles it before anything is written.
- [ ] **Hold the `Registration` and the core's own `ThreadClock` where a request's `Ctx` is reachable**
      — `crates/nvs-cli/src/serve.rs:793` makes the registration on the worker's own thread, which is
      the one thread `ThreadClock::current()` (`crates/nvs-host/src/cpuclock.rs:58`) may be called on,
      and `Host::isolate` at `crates/nvs-cli/src/serve.rs:902` is where a request's `Ctx` exists.
- [ ] **Publish on take-up and clear on completion** —
      `RunningRequest::new(ctx.safepoint_view(), clock, ctx.cpu_limit())` into
      `Registration::publish_safepoint` (`crates/nvs-host/src/watchdog.rs:463`), cleared where
      `Host::ran` (`crates/nvs-cli/src/serve.rs:928`) finishes one, so the next request on that core is
      never charged the last one's window.

## Backlog

- A publication carries the pre-handler ceiling, so a tier-1 handler is bounded by the sweep rather
  than by its reserve — `crates/nvs-runtime/src/ctx/safepoint.rs`'s CPU arm.
- `[context] modules` does not print `crates/nvs-cli/src/serve.rs`, which the next group needs, nor
  `crates/nvs-host/src/timer.rs` or `crates/nvs-host/src/lib.rs`.
- Stage 4 (the allocating loop) and stage 5 (the single operation past the ceiling) are untouched —
  `docs/agent/goals/40-resource-ceilings.md`.
- Every conformance run still costs an extra 60 s for the red CPU case, until the publisher lands.
