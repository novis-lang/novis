# Handoff

## State

**Goal `resource-ceilings` — every resource ceiling stops the request that breaks it. Stages 0, 2
and 3 are landed, and stage 3 now reaches the served path end to end.** A served request reads the
tree the instance booted on: `Serving` carries the boot `Arc<nvs_config::Snapshot>`
(`crates/nvs-server/src/serve.rs:385`), and `serve_connection` writes it to the connection's
context at each request's start, before the isolate answering it starts
(`crates/nvs-server/src/serve.rs:835`). So the context an isolate publishes from
(`crates/nvs-host/src/isolate.rs:433`) states the tree's `Ctx::cpu_limit()`, every `[limits]` key
reads, and a capability an entry asks for is answered off the tree instead of denied. Pinned by
`a_served_request_reads_the_tree_the_instance_booted_on`
(`crates/nvs-server/src/serve.rs:4923`), which prints `no tree` and fails when the write at 835 is
taken out — checked that way, not assumed.

**The seam is `Serving` and not a parameter of `serve_on_this_core`.** That function already takes
seven arguments and an eighth trips `clippy::too_many_arguments`, while the tree is shared exactly
as the valve and the header set are: one per instance, cloned per connection, never re-read under a
request that has begun. `nvs-host` may name no configuration crate
(`crates/nvs-host/Cargo.toml:52`), so the `Isolate` builder was never a candidate for it. A test
tree is built by `booted_on` (`crates/nvs-server/src/serve.rs:4890`), which fills **both** halves of
the snapshot, because a `[limits]` key is answered off `Snapshot::table` and everything typed off
the `Config` beside it.

What is still red under stage 0's check is the memory pair — `a-loop-that-calls-nothing-…` and
`one-operation-past-the-ceiling-…` — which is stages 4 and 5. Neither has a line on disk: none of
stage 4's four named tests exists yet. Goal `editor-surfaces`'s acceptance list is untouched.

## Next group

**Stage 4: the allocator publishes** — one file set, `crates/nvs-runtime/src/budget.rs` with
`crates/nvs-runtime/src/ctx/limits.rs` and `crates/nvs-runtime/src/ctx/mod.rs` for the flag word and
the ceilings. The goal's § *Standing decisions* settles the shape twice over: the refusal never goes
in the global allocator, and what a growing allocation does past the ceiling is **raise the bit**
that the two poll sites already read. `rule:errors/on-limit` is what the report owes.

- [ ] **Arm a threshold when a request has a ceiling, and nothing when it has none** —
      `crates/nvs-runtime/src/ctx/limits.rs:292`'s `refresh_limits` is the one pass that reads the
      ceiling, and `crates/nvs-runtime/src/budget.rs:260`'s `Accounting` is what a growing
      allocation goes through. Lands `an_uncapped_request_arms_no_threshold_and_pays_one_compare`,
      which is the cost half of the goal's *what it spends*.
- [ ] **Raise the memory bit at the allocation and stop at the next back edge** —
      `crates/nvs-runtime/src/ctx/mod.rs:204`'s `SafepointFlags` holds the word, and
      `crates/nvs-runtime/src/ctx/limits.rs:207`'s `memory_breach` is the report it becomes. Lands
      `a_growing_allocation_past_the_ceiling_sets_the_memory_bit` and the `.nvst` case
      `tests/conformance/error/a-loop-that-calls-nothing-is-stopped-by-the-memory-ceiling.nvst`.
- [ ] **Re-arm it when the ceiling moves, and restore it when an isolate ends** —
      `crates/nvs-runtime/src/ctx/limits.rs:62`'s `set_memory_limit` and
      `crates/nvs-runtime/src/ctx/isolate.rs:351`'s `Ctx::isolate`, which carries no ceiling across
      on purpose. Lands `a_ceiling_raised_mid_request_rearms_the_threshold` and
      `an_isolate_restores_the_threshold_its_parent_armed`.

## Backlog

- Stage 5's pre-check and its fallback — the goal's § *Standing decisions* names both halves.
- Stage 6 brackets the known stores; provenance waits for M6's arena (same section).
- A served request's budget is the connection tree's root, so what a second request on one
  keep-alive socket is charged is unread — `crates/nvs-runtime/src/budget.rs`'s module doc.
- The goal opens one new record, taking the next free number when the slice is written
  (`docs/decisions/`, highest 0173 at this commit).
