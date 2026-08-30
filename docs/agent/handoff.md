# Handoff

## State

**ADR 0020 § 1's reserved slice now has both halves.** `[limits] fatal_reserve_time` is a
directive (`crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/directive.rs` — `System`, like
its memory sibling) and `Ctx::fatal_reserve_time` is its reader: nanoseconds, defaulting to 50 ms
and clamped to a quarter of the ceiling, carved out of `cpu_limit` in the same `refresh_limits`
pass the memory half is carved in. `Ctx::reserve_time_within`'s doc comment is the only home of
both numbers and of why the CPU slice is not sized by the memory half's proportion.

**The CPU branch's slice is no longer zero wide.** `Ctx::run_limit_handler` widens `cpu_limit` by
the reserve *and* lowers `SafepointFlags::CPU_LIMIT` for the length of the call, restoring only
what it lowered; that function's comment owns why the flag edit is the half that matters — a
ceiling alone buys a handler nothing while the flag that stopped it is still up. What is still
missing is the **clock**: nothing samples the request thread's CPU time, so the flag is raised by
tests alone and nothing re-raises it when a handler overruns. `Ctx::cpu_limit`'s field doc owns why
that sampling is the host's and not this crate's.

**The driver's failing check is item 12, unstarted — not a regression.**
`a_recursive_spawn_is_reported_as_max_script_depth_and_not_as_memory` is one of three names
`nvs-host (limits at a safepoint)` lists for it; none of the three exists, and `max_script_depth`
exists nowhere in the tree — not on `nvs_config::Limits`, not in the directive registry, and
`nvs_host::Isolate` carries no depth and no per-tree accounting at all. It is the next group.

`orient.py`'s `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed
— the accounting is `crates/nvs-runtime/src/budget.rs`. The pack warns every session.

## Next group

**Item 12's first third — `max_script_depth`, which closes one of the failing check's three
names.** File set: `crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/directive.rs`,
`crates/nvs-host/src/isolate.rs` and `crates/nvs-host/tests/limits.rs`.

- [ ] **`[limits] max_script_depth` gets a directive and a reader** — the `cpu_time`/
      `fatal_reserve_time` pair that just landed is the shape to copy end to end: a field on
      `Limits` (`crates/nvs-config/src/tree.rs:156`), a row (`crates/nvs-config/src/directive.rs:90`)
      that is `System` for the reason the reserve is — a script choosing its own recursion ceiling
      is the case the class exists for — and a reader beside `configured_cpu_time`
      (`crates/nvs-runtime/src/ctx.rs:1258`). A case joins
      `crates/nvs-runtime/tests/configured_limits.rs:41`.
- [ ] **An isolate carries its depth, and a child is one deeper than its parent** — goal item 12
      (`docs/agent/loop-goal.md:90`) names the whole shape; this slice is the counter and its
      inheritance only. `crates/nvs-host/src/isolate.rs:90` is the struct, `:111` is `new`, `:153`
      is `start`.
- [ ] **A spawn past the ceiling is stopped as `max_script_depth` and never as an out-of-memory** —
      `crates/nvs-host/src/isolate.rs:133` (`run`), plus the `Limit` variant
      `crates/nvs-runtime/src/ctx.rs:868`'s comment already anticipates, so § 1's report names it.
      The test `a_recursive_spawn_is_reported_as_max_script_depth_and_not_as_memory` joins
      `crates/nvs-host/tests/limits.rs:346`, beside the reserved-slice pair.

## Backlog

- A `.nvst` case over the whole `onLimit` ladder — every case reaching it today is a Rust test
  (goal item 11, `docs/agent/loop-goal.md:86`).
- `n_concurrent_isolates_cannot_together_exceed_the_trees_budget` — item 12's per-tree accounting.
- `a_child_cannot_widen_a_capability_its_parent_narrowed` — item 12's overlay derivation.
- Nothing samples a CPU clock against `Ctx::cpu_limit`; that field doc owns the split.
- `orient.py` `[context] modules` names a `crates/nvs-host/src/budget.rs` that never existed.
