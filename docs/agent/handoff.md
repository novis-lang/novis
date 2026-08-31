# Handoff

## State

**Stage 6's fixture is green end to end.** `examples/cache.nvs` prints `local miss`, `local hit`,
`shared hit`, `allowed=true`, `allowed=false retryAfter=2` against a real store, three runs back to
back. The store is `tests/db/compose.yaml`'s `redis` service on `127.0.0.1:16379` — `nvs.toml`'s
`[cache.shared] url` names it, an `entry = "examples/cache.nvs"` block grants `net.connect` and
`net.internal` for the loopback, and the goal's new `[docker]` block brings it up once per run.
`chain.toml` now marks goal 4 `preflight = "docker"` beside goal 5, so a run without a daemon stops
in its first minute rather than at stage 6 six hours in.

**One store serves both legs**, unlike `tools/origin.py`, which needs a second listener inside the
distro. Docker Desktop forwards the published `127.0.0.1:16379` into WSL — measured, with a RESP
`PING` from the distro — so the WSL leg and the valgrind sweep reach the same container. The goal
file's `[docker]` comment is the home of that fact.

**The fixture draws its limiter key rather than writing one down.** A constant key inherits the
previous process's arrival, because the shared tier is coherent and does not forget when a process
ends, and the sweep runs the program three times in a few seconds — so `"account:1"` printed
`allowed=false` on line 4 of every run after the first. `Core\Random::token(6)` is the whole fix and
the example says why in the source; that line is now a demonstration of the tier's persistence
rather than an accident of it.

**`Core\RateLimit::shed` is one decision away, and the decision is not about GCRA.** See the first
item below: the algorithm is already factored for it, and what stands in the way is which capability
a member that reaches nothing declares.

## Next group

**Stage 6's close, over `crates/nvs-stdlib/src/ratelimit.rs`, `crates/nvs-stdlib/src/cache.rs`,
`crates/nvs-stdlib/src/registry.rs` and `crates/nvs-stdlib/tests/capability.rs`.**

- [ ] **Decide what `shed` declares, then write it** — ADR 0075 §§ 1 and 2, and ADR 0118 § 7 for the
      decision. `shed` reaches nothing, but `Core\RateLimit` is capability-bearing
      (`crates/nvs-stdlib/src/registry.rs:1335`), so `crates/nvs-stdlib/tests/capability.rs:282`
      demands a declaration and `crates/nvs-stdlib/tests/capability.rs:279`'s allowlist is frozen
      against exactly this addition. The playbook bullet added this session states the trap; ADR
      0118 § 7 states the rule, and ADR 0059 § 1 is the precedent it was written for.
- [ ] **`Core\RateLimit::shed`, GCRA in the core's own memory** — ADR 0075 §§ 1 and 2. Everything but
      the step is already shared: the row goes beside `consume` at
      `crates/nvs-stdlib/src/ratelimit.rs:120`, the arm at
      `crates/nvs-stdlib/src/ratelimit.rs:303`, and `window`
      (`crates/nvs-stdlib/src/ratelimit.rs:386`) is § 2's two parameters for both tiers — its
      `refuse` closure hardcodes `::consume()` and wants the member as an argument. The step is
      `crates/nvs-stdlib/src/ratelimit.rs:336`'s script, five lines of Rust over a `thread_local`
      table shaped like `crates/nvs-stdlib/src/cache.rs:325`, and it answers through
      `crates/nvs-stdlib/src/ratelimit.rs:440`'s `built` like `consume` does. A drained entry
      (`tat <= now`) can be swept without changing any later decision, which is what keeps the table
      O(keys under limit).
- [ ] **The local tier's memory is the core's, and capped** — ADR 0059 § 3's `nvs.toml` cap over
      `crates/nvs-stdlib/src/cache.rs:325`'s `ENTRIES`, declared beside
      `crates/nvs-config/src/tree.rs:589`'s `Cache` block. `shed`'s table is the second thing the
      same cap has to bound, so it is cheaper after the item above than before it.

## Backlog

- An `.nvst` case pinning that `shed` decides with no `[cache.shared] url` at all — the sharpest
  statement of the two tiers' difference (`tests/conformance/core/the-two-cache-tiers-have-two-contracts.nvst`).
- `[context] adrs` in `docs/agent/loop-goal.toml` is missing ADR 0075 §§ 1, 2 and 5 and ADR 0118 § 7;
  this session paid for all four by hand and the next one will pay again.
- Stage 7 — `Core\Fatal` and `Core\Log` as one serialiser reached twice (`docs/agent/loop-goal.toml`,
  stage 7 header).
- `examples/cache.nvs` is not in `[valgrind] skip` and now makes two Redis round trips under
  valgrind; if the sweep ever reports its second `consume` as allowed, the two calls drifted more
  than 2s apart and the fixture's window is what to widen (`docs/agent/loop-goal.toml:2413`).
