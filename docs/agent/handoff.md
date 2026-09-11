# Handoff

## State

**Goal `resource-ceilings` — every resource ceiling stops the request that breaks it. Stages 0 and 2
are landed, and stage 3 is landed for the CLI run path: `nvs run` publishes the one request it is, and
`tests/conformance/error/a-loop-that-allocates-nothing-is-stopped-by-the-cpu-ceiling.nvst` passes.** The
two cases still red under stage 0's check are the memory pair, which is stages 4 and 5. Goal
`editor-surfaces`'s acceptance list is still this goal's floor and is untouched.

Which run paths get a watchdog is settled. `nvs test` runs a `.nvst` case by spawning the `nvs` binary
(`crates/nvs-test/src/run.rs:344`), so the surface every conformance ceiling case reaches is `nvs run`
and not the server. `nvs run` now builds a `Watchdog` of its own immediately before `run_until_idle`
(`crates/nvs-cli/src/main.rs:1993`) and drops it when the scheduler is idle — built only where
`RunningRequest::new` answers `Some`, so a run under no `[limits] cpu_time`, and a platform with no
per-thread clock, start no thread and wake for nothing.

A watched entry is a server core or it is not: `Watched::core`
(`crates/nvs-host/src/watchdog.rs:260`) holds the CPU and the deadline table together, and
`Watchdog::register_requests` (`crates/nvs-host/src/watchdog.rs:404`) takes neither. Such an entry is
sampled against its ceiling on the same walk and reported as a wedged core never — the stderr that
record would land on is the program's own, and there is no fleet to shed its share onto.

**Nothing on the served path publishes yet**, so a runaway inside `nvs serve` is still bounded only by
its deadline. That is the next group. Two known gaps, each written where it bites: a publication carries
the ceiling that stood before `Ctx::run_limit_handler` widened it by `fatal_reserve_time`
(`crates/nvs-runtime/src/ctx/limits.rs:359`), so a tier-1 handler is bounded by the next sweep rather
than by its reserve; and `nvs test`'s in-process suite path (`crates/nvs-cli/src/runner.rs:465`) runs
user code under no sampler at all.

## Next group

**Stage 3: the served publisher — a core publishes each request it takes up** — one file set,
`crates/nvs-cli/src/serve.rs` with `crates/nvs-host/src/watchdog.rs` for the type it publishes.
`rule:errors/on-limit` is the ceiling both serve.

- [ ] **Hold the `Registration` and the core's own `ThreadClock` where a served request's `Ctx` is
      reachable** — `crates/nvs-cli/src/serve.rs:793` makes the registration and drops it straight into
      a `_watched` binding the accept loop cannot reach, while `crates/nvs-cli/src/serve.rs:902` is
      where a request becomes an `Isolate` with a tree of its own. The clock is taken once per core and
      never per request: `RunningRequest::new` (`crates/nvs-host/src/watchdog.rs:205`) must be handed
      the clock of the thread it is published from.
- [ ] **Publish on take-up and clear on completion** — `crates/nvs-cli/src/serve.rs:902`, the same
      shape `crates/nvs-cli/src/main.rs:1993` now has for a run: publish the tree root's
      `Ctx::safepoint_view` with `Ctx::cpu_limit`, and `publish_safepoint(None)` when the isolate is
      done, so a core between requests is charged for nothing.
- [ ] **Say what a served core costs** in `crates/nvs-cli/src/serve.rs:70`'s module doc, per
      `rule:programs/memory-priority`'s *say what you spend*: one clock read per core per interval,
      and one publication per request taken up.

## Backlog

- The tier-1 handler's reserve is not republished to the sampler — `crates/nvs-runtime/src/ctx/limits.rs:359`.
- `nvs test`'s in-process suite path runs user code with no sampler — `crates/nvs-cli/src/runner.rs:465`.
- Stage 4: a loop that calls nothing is stopped by the memory ceiling — goal `resource-ceilings`.
- Stage 5: one operation past the ceiling is refused before it allocates — goal `resource-ceilings`.
- This goal's one record is still unwritten; `docs/decisions/` is at 0173 — goal `resource-ceilings`.
