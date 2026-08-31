# Handoff

## State

**Stage 6's local tier is on disk.** `Core\Cache` (`local`, `shared`) and `Core\Cache\Store` (`put`,
`get`) are rows, cards, bodies, `address()` arms and registry lines in
`crates/nvs-stdlib/src/cache.rs`, with three `.nvst` cases under `tests/conformance/core/cache-*.nvst`
and § 1's three named tests green.

**Two decisions closed with it**, both argued in that module's own doc rather than here. An entry is ADR
0023 § 3's **byte payload**, not a live graph: a copied object holds a raw `ClassDesc` pointer and this
store outlives the request that filled it, so a hot-reload swap would leave it naming a dead descriptor —
and the shared tier needs bytes regardless. And the local tier needs **no grant**: ADR 0059 § 1 and ADR
0112 § 8 now both say so, which closes the one hole 0112's roster flagged.

**`Core\Cache::shared()` throws today** — nothing configures a store — and its `net.connect` row in
`registry::CAPABILITIES` lands with the connection that needs it, not before.

**The driver's acceptance failure on `examples/http.nvs` is still the driver, not the tree.**
`local_origin` is on disk at `tools/loop.py:1125`; the running process imported that module before it
existed, so its sweep serves nothing on 8099. Restarting the run is the whole fix.

## Next group

**Stage 6's remaining two slices, over one file set** — `crates/nvs-stdlib/src/cache.rs`,
`crates/nvs-stdlib/src/registry.rs`, and for the first also `crates/nvs-host/src/net.rs`. The stage's
five test names are at `docs/agent/loop-goal.toml:2382`.

- [ ] **`Core\Cache::shared`, over goal 2's parking stream** — ADR 0059 §§ 1-2: a real store, coherent
      across cores, gated by `net.connect` under ADR 0058's policy, and a put and a get that reach the
      same walk the isolate boundary does. The test is
      `a_cache_put_and_get_use_the_same_graph_copy_as_the_isolate_boundary`
      (`docs/agent/loop-goal.toml:2388`). The throw to replace is
      `crates/nvs-stdlib/src/cache.rs:333`, the tier slot it writes is
      `crates/nvs-stdlib/src/cache.rs:180`, the capability row goes beside
      `crates/nvs-stdlib/src/registry.rs:1308`, and the walk both tiers already share is
      `crates/nvs-runtime/src/graph.rs:672` and `crates/nvs-runtime/src/graph.rs:869`.
- [ ] **The local tier's memory is the core's, and capped** — ADR 0059 § 3: charged to the core, capped
      by an `nvs.toml` directive under ADR 0005's ordinary rules, and exceeding the cap **evicts** rather
      than failing an allocation. The test is `the_local_tiers_memory_is_charged_to_the_core_and_capped`
      (`docs/agent/loop-goal.toml:2390`). The map to bound is `crates/nvs-stdlib/src/cache.rs:256` and
      its writer is `crates/nvs-stdlib/src/cache.rs:260`; the counters a `-p nvs-stdlib` test reads are
      `nvs_runtime::budget`'s, and the playbook's bullet on why it may not install an allocator of its
      own applies.

## Backlog

- `Core\RateLimit` — stage 6's second check, `docs/agent/loop-goal.toml:2393`.
- A `forget` and a TTL on the store — they arrive with eviction; `crates/nvs-stdlib/src/cache.rs`'s
  module doc owns why neither is there yet.
- Restart the loop driver so `local_origin` (`tools/loop.py:1125`) is in the process that sweeps 8099.
- `orient.py` printed no spec text, and a Part II class's roster row
  (`docs/spec/01-core-library.md` § 16) is where its member list lives — if `[context]` grows a spec
  selector, that row is what a stage-6 session wants.
