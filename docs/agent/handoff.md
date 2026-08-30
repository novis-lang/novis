# Handoff

## State

**`[limits] max_script_depth` is enforced end to end, and a recursive spawn now reports as itself.**
`Limit::ScriptDepth` (`crates/nvs-runtime/src/ctx.rs:920`) is the third variant and its `name()` is
the directive spelling, so ADR 0020 § 1's report is branchable on it. `Ctx::script_depth_breach`
(`crates/nvs-runtime/src/ctx.rs:1310`) is the question, asked of the *child* that does not exist yet
and owning why in its doc; a ceiling of `0` — only an explicit `false` — answers `None` however deep
the chain runs.

**The refusal has one home and one converter.** `Isolate::start`
(`crates/nvs-host/src/isolate.rs:159`) asks it ahead of the argument's crossing, runs § 1's tier 1,
records the message on the parent and answers a `Collected` carrying `refused_completion`. It cannot
answer a `Fault` — its `Err` is the argument's `GraphError`, which is a *throw* — so
`Core\Script::spawn` (`crates/nvs-stdlib/src/script.rs:191`) turns a recorded pending into
`Fault::Pending(FATAL)`, which is ADR 0020's fatal no `catch` sees. That two-step is the one design
call this group carried; both halves say so in place.

The driver's failing acceptance check named three tests for item 12 and only the first is now
written. The other two are the next group and share the file they go in.

`orient.py`'s `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed —
the accounting is `crates/nvs-runtime/src/budget.rs`. The pack warns every session.

## Next group

**The rest of `nvs-host (limits at a safepoint)`'s item-12 roster**, which is what still fails the
driver's check. File set: `crates/nvs-host/tests/limits.rs`, with
`crates/nvs-runtime/src/ctx.rs`'s `Ctx::isolate` and `crates/nvs-host/src/isolate.rs`'s `start`
behind it. Both cases are measurements over a *tree* of contexts, which is the shape
`a_recursive_spawn_is_reported_as_max_script_depth_and_not_as_memory` just landed.

- [ ] **`n_concurrent_isolates_cannot_together_exceed_the_trees_budget`** — `docs/plan/m6.md`'s
      *Verify*, "N concurrent isolates cannot together exceed the tree's memory, CPU or output
      budget". `breached()` (`crates/nvs-host/tests/limits.rs:263`) is the shape for putting a
      context over its ceiling, and the question is whether `Ctx::isolate`
      (`crates/nvs-runtime/src/ctx.rs:1790`) leaves a child measuring against the *tree's* counters
      or re-bases it — `crates/nvs-runtime/src/budget.rs`'s counters are process-wide, so read that
      constructor's `memory_base` line before writing the assertion. If it re-bases, the case fails
      honestly and the fix is a slice of its own.
- [ ] **`a_child_cannot_widen_a_capability_its_parent_narrowed`** — the same *Verify* row. The
      ceiling half of this is already true and pinned
      (`crates/nvs-runtime/tests/configured_limits.rs:197`'s narrowed-parent block); what is
      unwritten is the capability half, which crosses at `Ctx::isolate`
      (`crates/nvs-runtime/src/ctx.rs:1790`) through the cloned snapshot. ADR 0118 § 1 and
      `crates/nvs-config/src/capability.rs`'s module doc are the rule.
- [ ] **The plan and `crates/nvs-stdlib/src/script.rs:11` disagree** — the plan's *Open now* calls
      "item 22's `Core\Script` members" a known gap; that module's `# Registered with no members, on
      purpose` says there are none to write and gives the argument. One of them is wrong; the module
      doc is the ADR-backed one, so the plan bullet is the likely loser.

## Backlog

- `examples/limits.nvs` has no depth twin; the `contains` check in `docs/agent/loop-goal.toml` only
  covers memory.
- Item 18's `Core\Secret::reveal()` is not in the registry — `crates/nvs-stdlib/src/registry.rs`.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- The path rule items 6, 10 and 12 share is still unwritten —
  `a_path_reaching_a_granted_root_through_dotdot_or_a_symlink_does_not_match` in `nvs-stdlib`.
- Stage 5's ADR 0042 cache checks (`nvs-cli (the cache)`) are untouched.
