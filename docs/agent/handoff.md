# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6: the tree's budget now crosses a core; the worker arm is what is
left.**

`nvs_runtime::TreeState` (`crates/nvs-runtime/src/ctx/safepoint.rs:@TreeState`) is the allocation a
request tree already made for its safepoint word, and it now carries `nvs_runtime::budget::OffCore`
(`crates/nvs-runtime/src/budget.rs:@OffCore`) beside it: what the tree holds and has written on cores
other than its root's. `Ctx::memory_used` (`crates/nvs-runtime/src/ctx/limits.rs:27`) and
`Ctx::output_used` add that pair when the context asking is on the root's core and add nothing when it
is not — a context off that core reads its own thread, which already holds every context beneath it
there.

A far-core member is made by `Ctx::join_tree` (`crates/nvs-runtime/src/ctx/safepoint.rs:82`), which
joins the word and the counters in one call, and publishes its own share at its polls —
`Ctx::publish_off_core` (`crates/nvs-runtime/src/ctx/limits.rs:62`), called from `Ctx::memory_breach`,
the question `run_helper` asks ahead of every `Core` member. Its memory comes off the tree whole when
its context drops; its writing stays. **The cadence is the poll, not the allocation**: nothing
enforced rests on that freshness, because what bounds such a child is the sub-cap it is handed where
it is placed.

**The shape the last handoff proposed was not takeable.** A lazily-created pair held by whichever
`Ctx` placed the child is invisible to that context's own ancestors, and the ceiling that stops a tree
is the root's — a root has no registry of live descendants to correct afterwards, which is the same
argument that put the safepoint word in one allocation per tree. So the pair is the tree's, and a tree
that places nothing pays two words nothing writes plus one relaxed load per reading.
`crates/nvs-runtime/src/budget.rs` § *the part a thread cannot hold* is that argument's one home.

Both placement words still start the child here (`crates/nvs-host/src/group.rs:265`). That gap's
reason is now the unwritten arm rather than the budget, in both module docs.

## Next group

**Stage 6: the worker arm, now that the budget crosses** — one file set: `crates/nvs-host/src/group.rs`,
`crates/nvs-host/src/worker.rs`, `crates/nvs-host/src/isolate.rs`.

- [ ] **The `Placement::Worker` arm secures a core, encodes the argument and posts** — the one arm is
      `crates/nvs-host/src/group.rs:265`; `crates/nvs-host/src/worker.rs:747`'s `destination()`
      secures a core before the work exists and `crates/nvs-host/src/worker.rs:277`'s `post` cannot
      fail. The far core's `Ctx` takes `Ctx::join_tree` with the handle `Ctx::tree_handle` hands over,
      plus a sub-cap set from what remains of the tree's budget here.
      `rule:concurrency/on-worker-runs-the-child-on-another-core` and ADR 0184 § 3 are what it has to
      satisfy.
- [ ] **The far core answers a `Send` completion, decoded into the parent's arena at the join** — the
      answer is copied at every node (ADR 0184 § 2), so it crosses as `nvs_runtime::graph::encode`'s
      bytes (`crates/nvs-runtime/src/graph.rs:719`) and is decoded where the parent parked; the slot
      and wake are `crates/nvs-host/src/blocking.rs:14`'s shape, turned around.
- [ ] **The stage's three `.nvst` cases** — `tests/conformance/isolate/` , named by the check at
      `docs/agent/loop-goal.toml:9881`: a worker child answers as a local one does, a child given
      grants holds only those, a child given limits stops at its own ceiling. The last one is what
      reads the counters above end to end, through `crates/nvs-host/src/group.rs:265`.

## Backlog

- `Ctx::memory_peak` is the reading core's mark alone; a tree's peak across cores is not recorded —
  `crates/nvs-runtime/src/ctx/limits.rs:41`.
- A serving core registers no inbox of its own, so a placement under `nvs serve` reaches a lazily
  started core rather than a sibling — `crates/nvs-host/src/worker.rs:99`, ADR 0184 § 5's fallback.
- `Core\Server::isDraining`'s sentence for `rule:security/request-state-throws-in-an-isolate` — the
  goal's standing decisions, not yet written.
