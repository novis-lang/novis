# Handoff

## State

**Goal `resource-ceilings` — every resource ceiling stops the request that breaks it. Stages 0 and 2
are landed; stage 3 has its two halves that are not the sampler.** Goal `editor-surfaces`'s whole
acceptance list is still this goal's floor and is untouched.

The tree's safepoint word is now publishable. `Ctx::safepoint_view`
(`crates/nvs-runtime/src/ctx/safepoint.rs:151`) hands out a `SafepointView` (`:222`) — the tree
root's `Arc<AtomicU64>`, `Send` where `Ctx` is not, with `request`/`flags` on it — and a watched
core holds one in `Watched` (`crates/nvs-host/src/watchdog.rs:137`), written by
`Registration::publish_safepoint` (`:355`) and read back by `Watchdog::running` (`:311`). It rides
the watched set's own `Mutex` rather than a second atomic beside the deadline, because a core
already takes that lock to register and deregister; `crates/nvs-host/src/timer.rs`'s reason for not
publishing a *deadline* that way is in the watchdog's module doc.

The clock is `crates/nvs-host/src/cpuclock.rs`: `ThreadClock::current()` taken on the core's own
thread, `ThreadClock::burned()` readable from any. `pthread_getcpuclockid` + `clock_gettime` on
Linux, the thread id reopened per sample for `GetThreadTimes` on Windows, `None` elsewhere — both
legs run green (Windows `cargo test`, WSL `cargo test -p nvs-host --lib cpuclock`). macOS is the
`None` arm and gets no CPU ceiling, which is the standing decision's own answer.

**Nothing samples it yet, and what a core publishes is not yet enough to charge one.**
`publish_safepoint` carries the handle alone; a sampler also needs `Ctx::cpu_limit`
(`crates/nvs-runtime/src/ctx/limits.rs:351`, nanoseconds, `0` for no cap) and the `ThreadClock`
reading at the moment that request was published. The next group widens it.

One thing not to re-derive: a core's thread clock measures the *thread*, and a core runs many tasks,
so a window's delta over-charges the published request whenever a neighbour ran in it. The window
that is safe to charge is one in which the core did **not** republish — which is exactly the runaway
case, because a request that never yields is a core that never changes what it is running.

The three stage-0 cases are still red, which is what stage 0 is for, so `python tools/verify.py`'s
conformance step and CI's conformance job carry exactly those three failures until stages 3, 4 and 5
land, and every run of that tree costs an extra 60 s for the CPU case
(`docs/agent/goals/40-resource-ceilings.md` § *Stage 0*). Nothing is blocked.

`[context] modules` still does not print `crates/nvs-host/src/timer.rs`, and it does not print
`crates/nvs-host/src/lib.rs` either — the crate doc is where a new module's one-paragraph entry goes.

## Next group

**Stage 3: the sampler in the watchdog's own loop** — one file set, `crates/nvs-host/src/watchdog.rs`
and `crates/nvs-host/src/cpuclock.rs`, reading the ceiling through `nvs-runtime`'s
`crates/nvs-runtime/src/ctx/limits.rs:351`. The goal's § *Stage 3* is its items 5 to 7, and the
standing decision above them is that this is CPU time and never wall clock.

- [ ] **Widen what a core publishes to what a request can be charged against** — the handle alone
      cannot be. `Registration::publish_safepoint` at `crates/nvs-host/src/watchdog.rs:355` takes a
      `SafepointView`; give it the `cpu_limit` nanoseconds from
      `crates/nvs-runtime/src/ctx/limits.rs:351` and the `ThreadClock::burned`
      (`crates/nvs-host/src/cpuclock.rs:69`) reading at that instant, so `Watched`
      (`crates/nvs-host/src/watchdog.rs:137`) holds a baseline rather than a bare handle. A request
      under no cap publishes nothing and is sampled not at all. `rule:errors/on-limit` is the ceiling
      this serves.
- [ ] **Sample it in the sweep** — `Shared::sweep` at `crates/nvs-host/src/watchdog.rs:384` already
      walks every entry once per interval with the lock held and the reporting done outside it; the
      clock read belongs on that walk. Charge only a window in which the core did not republish, per
      `## State` above. `rule:http-server/a-wedged-core-is-detected-by-its-deadline` owns the walk's
      cost argument, which this must not break: one clock read per core per interval.
- [ ] **Raise both halves** — `SafepointFlags::CPU_LIMIT` through `SafepointView::request`
      (`crates/nvs-runtime/src/ctx/safepoint.rs:227`), which is what compiled code polls, and
      `Ctx::expire_deadline`, which is what `rule:http-server/time-is-bounded-inside-a-helper`'s
      `bounded_loop` polls inside a member whose runtime scales with its input. The second needs a
      second view: `expire_deadline` writes `Ctx::deadline`, which `publish_safepoint` does not carry
      either. Neither branch is new — `nvs_safepoint`'s CPU arm is written and has only ever been
      reached by a test.

## Backlog

- Stage 4's flag path and stage 5's pre-check, both untouched — `docs/agent/goals/40-resource-ceilings.md`.
- macOS has no per-thread clock here and so no CPU ceiling; `thread_info` on a mach port is the
  answer nothing reaches yet — `crates/nvs-host/src/cpuclock.rs` module doc.
- Saying at boot which platforms got no CPU ceiling, per the goal's standing decision — no home yet.
- `Watchdog::register` still has no production caller; `nvs-server` does not start a watchdog —
  `crates/nvs-host/src/watchdog.rs` module doc § *Registering a core*.
- The goal's one new record is unwritten; take the next free number when the slice lands —
  `docs/agent/goals/40-resource-ceilings.md` § *Standing decisions*.
