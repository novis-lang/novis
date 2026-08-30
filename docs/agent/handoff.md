# Handoff

## State

**Item 12's roster is complete**, which is what the driver's `nvs-host (limits at a safepoint)`
check was failing on: all three cases are in `crates/nvs-host/tests/limits.rs` and the binary is
green. `n_concurrent_isolates_cannot_together_exceed_the_trees_budget` pins the accounting the
plan states — `Ctx::isolate` re-bases `memory_base` through `Ctx::new`, so each isolate reads back
only its own share and none is ever over the ceiling, while the root's base predates all of them
and `nvs_runtime::budget`'s counters are one per thread, so every child's byte is still on the
root's reading. `a_child_cannot_widen_a_capability_its_parent_narrowed` pins the other direction:
the grant crosses in the cloned snapshot, `Request::set` refuses the block both ways, and the
child's snapshot is the parent's `Arc` rather than a re-resolution.

`crates/nvs-host` gained `nvs-config` and `toml` as **dev**-dependencies for that second case;
nothing in its `src/` resolves a configuration and the Cargo.toml comment says so.

**Two things m6's *Verify* row names are not enforced anywhere**, both found by writing the case
above and both stated in its doc comment: `[limits] max_output` exists only as a configuration key
(`crates/nvs-config/src/tree.rs:166`) — `Ctx` holds no output ceiling, so no isolate can be over
one — and the deadline word crosses **by value** at `crates/nvs-runtime/src/ctx.rs:1814`, so a
child built before its parent's deadline expires never observes that expiry. The second is a
design call nobody has made in writing; it is the next group's second item.

`orient.py`'s `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed
— the accounting is `crates/nvs-runtime/src/budget.rs`. The pack warns every session.

## Next group

**The two unenforced thirds of m6's "memory, CPU or output budget" row.** File set:
`crates/nvs-runtime/src/ctx.rs` and `crates/nvs-host/tests/limits.rs`, the same pair item 12 was
written over, with `crates/nvs-config/src/tree.rs` behind them for the directive's spelling.

- [ ] **A `[limits] max_output` ceiling `Ctx` can answer for** — `docs/plan/m6.md`'s *Verify*, "N
      concurrent isolates cannot together exceed the tree's memory, CPU or **output** budget". The
      shape is the memory ceiling's, one field further along: `Ctx::memory_limit`
      (`crates/nvs-runtime/src/ctx.rs:1074`) and `Ctx::over_memory_limit`
      (`crates/nvs-runtime/src/ctx.rs:1111`) are what to copy, `Ctx::refresh_limits` is where the
      directive is read (`crates/nvs-config/src/tree.rs:166` is the key, already parsed as bytes by
      `crates/nvs-config/src/value.rs:119`), and the breach belongs beside the memory branch in
      `nvs_safepoint` (`crates/nvs-runtime/src/ctx.rs:2697`) as ADR 0020 § 1's third `Limit`
      variant. Decide first whether an isolate's buffered sink counts against the root: the answer
      the memory half gives is yes, and `Ctx::isolate`'s doc (`crates/nvs-runtime/src/ctx.rs:1781`)
      is the sentence to keep true.
- [ ] **Decide what the deadline copy means, and record it** — `crates/nvs-runtime/src/ctx.rs:1814`
      snapshots the parent's word into the child rather than sharing it, so a tree whose deadline
      expires mid-flight stops only the contexts built after it. Either share the atom or write
      down why the copy is right (the scheduler cancels a running child through ADR 0072 § 5, which
      may already be the whole answer). One paragraph in that constructor's doc either way; a
      shared word also wants a case beside
      `n_concurrent_isolates_cannot_together_exceed_the_trees_budget`
      (`crates/nvs-host/tests/limits.rs:588`).
- [ ] **Extend that case with the output third** once the ceiling exists — its doc comment
      (`crates/nvs-host/tests/limits.rs:588`) currently says why output is not asserted, and that
      paragraph is what has to stop being true.

## Backlog

- Item 18's `Core\Secret::reveal()` is not in the registry — no `Core\Secret` class exists at all;
  the roster is `crates/nvs-stdlib/src/registry.rs:984` and `conventions.md`'s five edits are the
  shape (`docs/implementation-plan.md`'s *Open now*).
- `Live::admit`'s same-class check is asked of the answer and not of the argument
  (`crates/nvs-runtime/src/graph.rs` § *Known gaps*).
- `[context] modules` in `docs/agent/loop-goal.toml` names a `crates/nvs-host/src/budget.rs` that
  never existed; the module is `crates/nvs-runtime/src/budget.rs`.
- `nvs_config::Request::set` cannot express a capability change in either direction; if a request
  is ever meant to narrow one, that is a decision and an ADR, not an implementation gap
  (`crates/nvs-config/src/request.rs:106`).
