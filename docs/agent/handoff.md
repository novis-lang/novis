# Handoff

## State

**Goal `process-cache` — a value outlives a request in the serving process, and a secret does so only
sealed. Stage 3's two configuration slices are on disk; the tier itself is still unwritten.**
`[cache.process]` takes `max_size` and `fill_wait`, both `System`/`Reload`, and `[session] backend`
refuses `process` with the local tier's own sentence. Goal `http-client`'s list is this goal's Stage 1
floor and still passes.

No Rust of the tier exists yet: `Core\Cache::process()` is unwritten, so stage 3's two acceptance
checks are red because their artefacts are not written, not because anything regressed. `E0642` is new
and implemented — a `fill_wait` of `0` or `false` is refused at boot in `nvs_config::store::validate`;
`E0626` widened in place rather than gaining a code.

[ADR 0181](../decisions/0181.md) is the goal's one record. The four new rules are `designed`, the six it
amended are rendered, and spec § 16 and § 15 carry the new members.

## Next group

**Stage 3: the process tier, the keystone** — one file set: `crates/nvs-stdlib/src/cache.rs`, with
`crates/nvs-cli/src/serve.rs` for where the map is created. Take these in order; the first two want one
session between them, because a map nothing reaches is a `dead_code` failure rather than a slice.

- [ ] **The map, and the three tests the check names** — beside the local tier's at
      `crates/nvs-stdlib/src/cache.rs:385`. 64 shards behind read/write locks, `get` taking a read lock
      and never a write; an entry's real key carries the `[[app]]` and the configuration generation;
      every byte inside a `nvs_runtime::budget::Detached` bracket the store itself holds; eviction by
      write age past `[cache.process] max_size`, never a failed `put`.
      `rule:concurrency/the-process-tier-is-one-store-per-process` is the whole specification, and
      `rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance` is the bracket's. The three
      `process_tier_is_one_map_every_core_reads`, `process_tier_keys_are_scoped_per_app_and_per_generation`
      and `process_tier_bytes_are_on_the_detached_balance` land with it — they are the stage's second
      check and name what the map must be. Delete `max_size`'s `[unread:]` trailer at
      `crates/nvs-config/src/tree.rs:1119` when the map reads it; that trailer is the only reason
      `tools/directives.py --check` is green over the key today.
- [ ] **`process()` and the third tier on the store class** — `crates/nvs-stdlib/src/cache.rs:147`, the
      five edits `docs/agent/conventions.md` § *A `Core` member* lists. It answers the same
      `Core\Cache\Store` class with a third `tier`, takes no grant
      (`rule:core-api/two-cache-tiers`), and is created where `nvs serve` starts its workers — before
      the first one accepts — and for the length of a `nvs run`, which is
      `crates/nvs-cli/src/serve.rs`.
- [ ] **The two `.nvst` cases the failing check names**, neither written yet:
      `tests/conformance/core/cache-process-tier-is-read-by-a-later-request.nvst` and
      `tests/conformance/core/cache-process-tier-evicts-past-its-cap-and-never-fails-a-put.nvst`.
      The local tier's own eviction case is the shape to follow —
      `tests/conformance/core/cache-local-forgets-an-entry-rather-than-failing-the-write.nvst:1` —
      and the cap case is sized against a probe rather than a guess, which
      `docs/agent/playbook.md:3605` owns the arithmetic for.

**A `[context] modules` gap this session paid for twice:** the manifest names
`crates/nvs-config/{tree,directive,session}.rs` but not `crates/nvs-config/src/store.rs`, which is
where a `[cache]` block's boot refusal is written, nor `crates/nvs-diagnostics/src/lib.rs`, where a new
code is claimed. Two more files a new directive always touches and nothing points at:
`crates/nvs-config/src/default.toml` and `docs/reference/tools/20-config.md` (the block table
`tools/reference.py` renders into `docs/novis.md`). Both of `playbook.md`'s bullets about
`[unread:]` trailers and `NOT IMPLEMENTED` notes hang off that template file, and the pack filters
the playbook to the item's own paths — so a session adding a key pays for them again unless
`default.toml` is in `modules`.

## Backlog

- A fleet-wide single fill is a lease over the shared tier, not a cache key — no record, out of this goal
  (ADR 0181 § *Alternatives rejected*).
- A `secret bytes` value: the sealed door is written over `secret string` only (ADR 0181 § *Revisiting*).
- A user's refresh token belongs in the session, never a cache tier — already ruled, `rule:http-server/a-session-holds-a-secret-only-sealed`.
- The `nvs/rest` package is the first caller of all of this and is unscheduled —
  [carried-gaps.md](carried-gaps.md).
- `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot` still names `local`
  alone, where the code now refuses both weak tiers; ADR 0181's amendment set is six rules and does not
  include it, so stage 8 decides whether the fragment says so itself or leans on
  `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`.
- The pack does not print the goal's own `## Stage N` section, only § *Standing decisions*; an item that
  says "the rules the goal's stage 2 names" cannot be worked without reading
  `docs/agent/loop-goal.md`, and no `[context]` field carries it. Either the item names the ids or
  `orient.py` slices the stage.
