# Handoff

## State

**Goal `process-cache` — a value outlives a request in the serving process, and a secret does so only
sealed — has its record. Stage 2 is done; no code of the goal has landed yet.** Goal `http-client`'s whole
list is this goal's Stage 1 floor and still passes.

[ADR 0181](../decisions/0181.md) is the goal's one record, and it decided the four open numbers: `core-api/two-cache-tiers`
is **amended in place** (the id is a path, 82 citations in 49 files, nine of them in seven frozen records);
the process map is a **fixed 64 shards**; refresh-ahead is the **last fifth** of an entry's lifetime; and
`[cache.process]` ships `max_size = 32M` — the same figure as the local tier, which means less memory
because it is held once per process — and `fill_wait = 5s`, with `0` refused under **`E0642`** (claimed, not
yet implemented). `E0626` widens to refuse the process tier as a session backend rather than gaining a code.

The four rules are on disk as `designed`, the six modified ones are amended and rendered, and spec § 16's
`Core\Cache` row and § 15's `Core\Session` bullet carry the new members. Stage 3 is the keystone and is all
Rust.

## Next group

**Stage 3: the process tier, the keystone** — one file set: `crates/nvs-stdlib/src/cache.rs`,
`crates/nvs-config/{tree,directive,session}.rs`, `crates/nvs-cli/src/serve.rs`.

- [ ] **`[cache.process]`, two keys** — the struct beside `CacheLocal` at
      `crates/nvs-config/src/tree.rs:1078`, and the rows beside `cache.local` at
      `crates/nvs-config/src/directive.rs:168`. `max_size` and `fill_wait`, both `System`/`Reload`; the
      shipped figures and the `0` refusal are ADR 0181 § 14, and `rule:concurrency/cache-memory-is-charged-to-the-core`
      is the cap's reasoning.
- [ ] **The map** — beside the local tier's at `crates/nvs-stdlib/src/cache.rs:385`. 64 shards behind
      read/write locks, `get` taking a read lock only, eviction by write age, keys carrying the `[[app]]`
      and the configuration generation, every byte inside the store's own `Detached` bracket.
      `rule:concurrency/the-process-tier-is-one-store-per-process` specifies it.
- [ ] **`process()` and the third tier on the store class** — `crates/nvs-stdlib/src/cache.rs:147`, with
      the registry rows at `crates/nvs-stdlib/src/registry.rs:312`; the map created before the first
      worker accepts at `crates/nvs-cli/src/serve.rs:1519`'s fan-out, and for the length of a `nvs run`.
      `rule:core-api/two-cache-tiers` now reads as a member per tier.
- [ ] **`E0626` refuses it as a session backend** — `crates/nvs-config/src/session.rs:28` has no variant
      for a weak tier and must not gain one; the message at `:81` names the process tier as well, per
      `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`.

## Backlog

- A fleet-wide single fill is a lease over the shared tier, not a cache key — no record, out of this goal
  (ADR 0181 § *Alternatives rejected*).
- A `secret bytes` value: the sealed door is written over `secret string` only (ADR 0181 § *Revisiting*).
- A user's refresh token belongs in the session, never a cache tier — already ruled, `rule:http-server/a-session-holds-a-secret-only-sealed`.
- The `nvs/rest` package is the first caller of all of this and is unscheduled —
  [carried-gaps.md](carried-gaps.md).
- The pack does not print the goal's own `## Stage N` section, only § *Standing decisions*; an item that
  says "the rules the goal's stage 2 names" cannot be worked without reading
  `docs/agent/loop-goal.md`, and no `[context]` field carries it. Either the item names the ids or
  `orient.py` slices the stage.
