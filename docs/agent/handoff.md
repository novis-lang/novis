# Handoff

## State

**Goal `process-cache` — a value outlives a request in the serving process, and a secret does so only
sealed. Stage 3 is on disk and green: `Core\Cache::process()` hands back a store over one map every core
of the process reads.** `[cache.process] max_size` and `fill_wait` are configured and read,
`[session] backend = "process"` is still refused at boot, and goal `http-client`'s list — this goal's
Stage 1 floor — still passes. Stages 4 to 6 (a lifetime and `forget`, the sealed secret, the session's
own secret) are unwritten.

The tier is 64 shards, each an `Entries<Arc<[u8]>>` behind an `RwLock`, and a `get` takes a read lock
only. `Entries` at `crates/nvs-stdlib/src/cache.rs:434` is now generic over the key handle and is both
in-process tiers' map, so the eviction policy the two are specified to share is one implementation. The
cap is divided by the shard count and a shard evicts against its own share, which is why a cap case
cannot be sized like the local tier's — the playbook bullet owns that arithmetic.

**`nvs_config::Snapshot` gained `generation: u64`** — monotonic, never reused, `0` for a snapshot no
boot built — because the process tier keys an entry on the generation it was written under and a
snapshot's *address* cannot answer for that: `nvs_runtime::pool::Ticket` may key on the address because
it holds the `Arc`, and a cache entry cannot hold one without freeing a boot-time allocation inside a
`budget::Detached` bracket. `scoped` at `crates/nvs-stdlib/src/cache.rs:734` builds the real key.

`rule:concurrency/the-process-tier-is-one-store-per-process` is now `shipped` rather than `designed`,
and the rulebook and `docs/novis.md` are re-rendered. [ADR 0181](../decisions/0181.md) is the goal's one
record.

## Next group

**Stage 4: a lifetime and `forget`, on every tier** — one file set: `crates/nvs-stdlib/src/cache.rs` and
`crates/nvs-stdlib/src/cache/redis.rs`. The first two want one session between them: an expiry nothing
reads back is a field no case can ask about.

- [ ] **`put`'s trailing `{ttl?: Duration}`, and the expiry held beside the payload on both in-process
      tiers** — the row at `crates/nvs-stdlib/src/cache.rs:249` and the map at
      `crates/nvs-stdlib/src/cache.rs:434`, whose value type becomes the payload and its deadline.
      `rule:core-api/shape-rules` R2 is the options shape and `rule:core-api/a-lifetime-is-written` how
      a duration is written; the module doc's § *What is not here yet* at
      `crates/nvs-stdlib/src/cache.rs:102` is the paragraph this deletes. An omitted `ttl` keeps
      today's behaviour, so `charged` and the cap arithmetic do not move.
- [ ] **An expired entry is absent, checked on the read** — `store_get` at
      `crates/nvs-stdlib/src/cache.rs:622` and `process_get` at `crates/nvs-stdlib/src/cache.rs:788`
      drop what they find expired; the shared tier writes `PX` in
      `crates/nvs-stdlib/src/cache/redis.rs`. `rule:concurrency/put-and-get-are-the-whole-boundary` is
      the boundary this stays inside — a read that drops an expired entry takes the shard's write lock,
      and the alternative (answering `None` and leaving it) is the one that keeps `get` a read; decide
      it in the slice and say which in the commit.
- [ ] **`forget(string $key): void` on the store class, all three tiers** — the instance array at
      `crates/nvs-stdlib/src/cache.rs:244`, `DEL` in `crates/nvs-stdlib/src/cache/redis.rs`, and the
      five edits `docs/agent/conventions.md` § *A `Core` member* lists. Stage 4's two `.nvst` cases are
      `tests/conformance/core/cache-put-with-a-ttl-is-absent-after-it-on-every-tier.nvst` and
      `tests/conformance/core/cache-forget-removes-an-entry-on-every-tier.nvst`, which
      `docs/agent/loop-goal.toml`'s stage 4 check names and neither of which exists.

## Backlog

- A fleet-wide single fill is a lease over the shared tier, and not this goal — `docs/agent/loop-goal.md`
  § *Standing decisions*.
- `Core\Cache::process()` has no TTL, so nothing yet bounds an entry by time — stage 4 above.
- The `nvs/rest` package is the first caller of all of this and is unscheduled —
  `memory/novis-rest-client-plan.md`'s plan, `docs/agent/carried-gaps.md` if it outlives this goal.
