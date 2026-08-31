# Handoff

## State

**Stage 5 is closed on the tree.** Its `nvs-stdlib` check names six tests and all six are green — the
last, `a_socket_read_runs_on_the_reactor_and_parks_its_coroutine`, pins ADR 0051 § 3's "over the
runtime's own reactor, not a second event loop" from the core's side: the exchange is driven with
`Scheduler::run` alone, the park is filed with the reactor, and the origin holds half the reply back so
that what parked can only be a read. That test's own doc comment owns why it turns until the origin
signals rather than asserting the first park it sees.

**The driver's acceptance failure on `examples/http.nvs` is still the driver, not the tree.**
`local_origin` is on disk at `tools/loop.py:1125`; the running process imported that module before it
existed, so its sweep serves nothing on 8099. Restarting the run is the whole fix — nothing in this
tree changes it.

**Stage 6 opens next**, and nothing of it is on disk: no `cache` module, no `Core\Cache` row.

## Next group

**Stage 6's local tier first, then the shared one over goal 2's graph copy.** All three slices share
one file set — `crates/nvs-stdlib/src/cache.rs` (new), `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/lib.rs` — so they are one group, and the five test names the stage wants are at
`docs/agent/loop-goal.toml:2382`.

- [ ] **`Core\Cache::local`, as the five edits** — ADR 0059 §§ 1-3: two members with separate
      contracts, not one API with a flag, and a local entry that may be absent at any time. The three
      tests are `local_and_shared_are_separate_members_with_separate_contracts`,
      `a_local_entry_may_be_absent_at_any_time_and_the_contract_says_so` and
      `a_local_write_on_one_core_is_not_visible_on_another`
      (`docs/agent/loop-goal.toml:2384`). `mod cache;` goes at
      `crates/nvs-stdlib/src/lib.rs:212`, alphabetically ahead of `channel`, and the class joins
      `CLASSES` at `crates/nvs-stdlib/src/registry.rs:1054`.
- [ ] **`Core\Cache::shared`, over goal 2's graph copy** — ADR 0059 § 2: a put and a get use the same
      copy the isolate boundary does, not a third mechanism, which is
      `a_cache_put_and_get_use_the_same_graph_copy_as_the_isolate_boundary`
      (`docs/agent/loop-goal.toml:2388`). Redis is the default shared store and picking it is
      pre-authorized; rows join `crates/nvs-stdlib/src/registry.rs:1054` beside the local pair.
- [ ] **The local tier's memory is the core's, and capped** — ADR 0059 § 3, as
      `the_local_tiers_memory_is_charged_to_the_core_and_capped`
      (`docs/agent/loop-goal.toml:2390`). Last because it asserts over what the first two slice in,
      and it is the one that decides where the bytes are counted.

## Backlog

- `examples/http.nvs`'s exact check passes only once the driver is restarted — `tools/loop.py:1125`.
- `Core\RateLimit`'s five tests, stage 6's second check — `docs/agent/loop-goal.toml:2398`.
- The outbound transport has no connection pool, by decision — its module doc, § *One connection per
  attempt, closed by the reply*.
- `https` outbound stays refused until the trust-anchor set has an owner — the same module doc.
- ADR 0060 § 5's "a verified signature does not launder" is standing and unasserted anywhere.
