# Handoff

## State

**Stage 6's limiter is on disk.** `Core\RateLimit::consume(tainted string $key, uint $limit,
Duration $per, {burst?: uint, cost?: uint})` is a row, a card, a body and a `net.connect` grant, and
it answers a `Core\RateLimit\Decision` whose four fields are four zero-argument members
(`crates/nvs-stdlib/src/ratelimit.rs`). The GCRA step is one `EVAL` of our own Lua script — read the
theoretical arrival time, decide, write it back, atomically — against the same shared store
`Core\Cache::shared()` names, because a deployment has one. Rust owns the *parameters* (`window`:
the emission interval and the tolerance) and the script owns the *step*, which is the split `shed`
inherits so § 2's "both tiers run the identical algorithm" has one home.

**Three decisions, each argued in the module doc it lives in.** `consume` is **its own door** — ADR
0075 §§ 1 and 5 both write it standing alone, so it opens the configured store itself through
`crate::cache::open_configured` rather than requiring a `Core\Cache::shared()` first. Store
*reachability* now throws `IOError` everywhere (§ 5's class, and the one its fail-open `catch`
holds) while an unconfigured or ungranted store stays `RuntimeError`, a deployment mistake being the
wrong thing to fail open on; `Core\Cache`'s three cards moved with it. And `EVAL` may **not** be
replayed once its request has left — `redis::Replay` and `Failure::sent` are that, since a limiter
replayed charges a second arrival — where `SET` and `GET` still replay however far they got.

**`examples/cache.nvs` compiles end to end now and stops at the store.** It prints `local miss`,
`local hit`, then throws because no `[cache.shared] url` is configured. Its `retryAfter` line rounds
the wait **up**, in the source, because the second arrival is a whisker under 2s away and
`toSeconds()` truncates — the frozen `retryAfter=2` is unreachable otherwise. That is the fixture's
half done; the harness and the grant are not.

## Next group

**Stage 6's close, over `examples/cache.nvs`, `docs/agent/loop-goal.toml`,
`crates/nvs-stdlib/src/cache.rs` and `crates/nvs-stdlib/src/ratelimit.rs`.**

- [ ] **The harness `examples/cache.nvs` needs, and its grant** — the fixture wants `shared hit` and
      two decisions from a reachable store, so it needs a Redis the run can reach, a
      `[cache.shared] url` and a `net.connect` grant for its host. The frozen five lines are
      `docs/agent/loop-goal.toml:2413`; the example's own store calls are
      `examples/cache.nvs:50` and `examples/cache.nvs:60`. `tools/origin.py` is the shape a
      stage-5 harness took — read it before deciding whether stage 6 brings up the compose
      service the goal's stage header names or fakes RESP the way
      `crates/nvs-stdlib/src/cache/redis.rs:400`'s cases do.
- [ ] **`Core\RateLimit::shed`, GCRA in the core's own memory** — ADR 0075 §§ 1 and 2: per core and
      approximate, and its card states the arithmetic in as many words ("a limit of 100 across eight
      cores admits up to 800"). It derives its window from `crates/nvs-stdlib/src/ratelimit.rs:412`
      (`window`, already the one home for § 2's two parameters) and needs a per-core
      `thread_local` map of stored arrival times beside
      `crates/nvs-stdlib/src/cache.rs:302`'s shape. The row and card go beside
      `crates/nvs-stdlib/src/ratelimit.rs:120`, the capability question is
      `NEEDS_NO_CAPABILITY` versus a row at `crates/nvs-stdlib/src/registry.rs:1317` — nothing
      leaves the process, so it is the `local()` case again and the standing decision names it.
      Test name: `shed_is_per_core_and_approximate_and_says_so`.
- [ ] **The local tier's memory is the core's, and capped** — ADR 0059 § 3: the `nvs.toml` cap that
      evicts, which is what makes the local tier's footprint O(cores × working set) rather than
      O(traffic). The map is `crates/nvs-stdlib/src/cache.rs:302` and the directives beside it are
      `crates/nvs-stdlib/src/cache.rs:216`. Test name:
      `the_local_tiers_memory_is_charged_to_the_core_and_capped`.

## Backlog

- `Core\RateLimit`'s `.nvst` cases all stop at the unconfigured store; a case over a *reachable* one
  waits on the same harness item 1 does — `docs/agent/loop-goal.toml` stage 6.
- ADR 0075 § 3 still writes `Decision` as readonly properties; the body should say members, per the
  playbook bullet this session added — `docs/adr/0075-core-ratelimit.md`.
- `Core\Cache`'s shared tier has no TTL, eviction or `forget` — `crates/nvs-stdlib/src/cache.rs`'s
  module doc, § *What is not here yet*.
- A password, a database index and `rediss://` are each refused with a sentence —
  `crates/nvs-stdlib/src/cache.rs`'s module doc owns why, and ADR 0103 § 7 is the blocker.
- `EVAL` sends the whole script every call; `EVALSHA` with a `NOSCRIPT` replay is the optimization
  not taken — `crates/nvs-stdlib/src/cache/redis.rs`'s module doc.
- Stage 7 is `Core\Fatal`/`Core\Log`, one serialiser reached twice —
  `docs/agent/loop-goal.toml:2421`.
