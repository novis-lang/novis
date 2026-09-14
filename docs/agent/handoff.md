# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6: a worker placement crosses — for the method entry form.**

`Placement::Worker` is a real arm now (`crates/nvs-host/src/group.rs:236`). It asks
`nvs_host::placed::destination_for` and, on a core, encodes the argument, builds the seed and posts;
on any other answer it falls through to the same-core body, which keeps every promise the placement
makes but the core. `crates/nvs-host/src/placed.rs` is the crossing's one home: what crosses is the
`Entry`'s two names, `nvs_runtime::graph::encode`'s bytes, `nvs_runtime::PlacedIsolate` and the
answer slot, and `Crossed` is the `Send` completion the far core answers with, decoded into the
parent's arena at the join against the parent's own class table.

`nvs_runtime::PlacedIsolate` (`crates/nvs-runtime/src/ctx/isolate.rs:@PlacedIsolate`) is
`Ctx::isolate`'s copy list with the thread boundary in the middle: `Ctx::placed_isolate` reads the
parent and computes the sub-cap from what remained of the tree's budget there, `PlacedIsolate::build`
writes the child, joins the tree and arms that cap — in that order, because arming reads the
safepoint address. A spent tree hands a cap of `1` and never `0`, `0` being the sentinel for no
ceiling at all. Four cases in `crates/nvs-runtime/tests/tree_budget.rs`, three in `placed.rs`.

**Only the method form is placed.** A path becomes code through
`nvs_runtime::script::resolve`, which reads a resolver a *thread* was installed with, and only the
thread `nvs-cli` booted on has one (`crates/nvs-cli/src/main.rs:2157`) — so a path that crossed would
answer `NoResolver`, a failure value where the program asked for a core. `placed.rs`'s `# Known gaps`
is that gap's one home; `group.rs` and `worker.rs` point at it. The method form needs no resolver:
its label is looked up in the `Arc<ClassTable>` the seed carries, which is `Send + Sync` because a
compiled unit is read by every core.

The driver's red check `[2 the deadlock]` did not reproduce — see the new playbook bullet.

## Next group

**Stage 6: the `.nvst` cases, now that a method entry crosses** — one file set:
`tests/conformance/isolate/`, `crates/nvs-host/src/placed.rs`.

- [ ] **A child on a worker core answers as one on this core does** —
      `tests/conformance/isolate/a-child-on-a-worker-core-answers-as-one-on-this-core-does.nvst`,
      the case the stage's `nvs-suite` check names. Spawn the **method** form with `on: "worker"`
      and the same one with `on: "here"`, and assert the two answers agree, which is the agreement
      shape `conventions.md` § *A `.nvst` test case* names. The arm it exercises is
      `crates/nvs-host/src/group.rs:236` and `crates/nvs-host/src/placed.rs:113`.
      `rule:concurrency/on-worker-runs-the-child-on-another-core` is what it pins.
- [ ] **A child given limits is stopped at its own ceiling** —
      `tests/conformance/isolate/a-child-given-limits-is-stopped-at-its-own-ceiling.nvst`. The
      sub-cap the placement hands the child is `crates/nvs-runtime/src/ctx/isolate.rs:702`, armed at
      `crates/nvs-runtime/src/ctx/isolate.rs:619`'s `build`, and
      `rule:security/isolate-budget-is-the-trees` is the rule: tighter than what remains, never
      wider.
- [ ] **A child given grants holds only those and cannot widen them** —
      `tests/conformance/isolate/a-child-given-grants-holds-only-those-and-cannot-widen-them.nvst`.
      The overlay crosses inside the seed (`crates/nvs-runtime/src/ctx/isolate.rs:622`), so the far
      core asks the same grant question the parent would have, through
      `crates/nvs-host/src/placed.rs:84`.
      `rule:concurrency/an-upgrades-options-are-spawn-scripts` names the four options.

## Backlog

- A **path** entry on a worker core needs a resolver that thread can reach; the unit cache behind
  `nvs-cli`'s `Compiler` is `RwLock`-based and looks `Sync`, but `script::scoped` hands out a
  stack borrow as `&'static`, so a worker thread outliving the scope is the soundness question —
  `crates/nvs-host/src/placed.rs` § *Known gaps*.
- `nvs serve` registers no inbox of its own, so a placement there reaches a lazily started worker
  core rather than a sibling serving one — `crates/nvs-host/src/worker.rs` § *Known gaps*, ADR 0184
  § 5's pre-authorized fallback.
- A placed child's answer is copied twice (the arena copy `finish` makes, then the encode);
  `crates/nvs-host/src/placed.rs` § *What it spends* states it.
- `[context] modules` names neither `crates/nvs-host/src/placed.rs` nor
  `crates/nvs-runtime/src/ctx/isolate.rs`, and this session needed both.
