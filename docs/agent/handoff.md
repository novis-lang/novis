# Handoff

## State

**ADR 0020 § 1's tier 1 is wired end to end for memory.** A breach at either seam — the helper
boundary (`crates/nvs-runtime/src/abi.rs:321`) and the safepoint poll
(`crates/nvs-runtime/src/ctx.rs:2285`) — calls `Ctx::run_limit_handler` (`ctx.rs:1075`) before it
records the `FATAL`, so a throw of the handler's own is overwritten by the breach rather than
reported in its place. **Zero retries is an ownership move, not a flag:** the slot is emptied before
the call, so the handler's own first helper call finds no registration and falls through.

**The reserved slice is carved, not added.** `refresh_limits` sets `memory_limit` to `[limits]
memory` *less* the reserve, and `run_limit_handler` adds it back for exactly the length of the call.
The size is the new `System`-class `[limits] fatal_reserve_memory` (`nvs-config`'s registry,
`tree.rs`'s `Limits`, `value.rs`'s `unit_of`), defaulting to **1 MiB clamped to a quarter of the
ceiling** — `Ctx::reserve_within` (`ctx.rs:1180`) is that decision's only home, since ADR 0020 § 1
states the slice exists and states no number. The breach message now names the whole budget and the
reserve inside it. Three tests in the new `crates/nvs-host/tests/limits.rs` pin all of it, under the
names the goal's Stage 4 check asks for.

The driver's failing check is closed: `a_key_set_in_two_files_resolves_to_the_later_one_with_both_origins_reported`
was on disk under a paraphrase in `crates/nvs-config/tests/resolve.rs` and already asserted both
origins, so it was a rename and nothing more.

Still unfixed: `orient.py`'s `[context] modules` names `crates/nvs-host/src/budget.rs`, which never
existed — the accounting is `crates/nvs-runtime/src/budget.rs`. The pack warns about it every
session.

## Next group

**The rest of ADR 0020 § 1's ladder.** File set: `crates/nvs-runtime/src/ctx.rs`
(`run_limit_handler` at :1075, `nvs_safepoint`'s CPU branch at :2285, `fatal_reserve` at :970),
`crates/nvs-host/tests/limits.rs` (the three cases and the hand-built closure they share), and
`crates/nvs-stdlib/src/fatal.rs` for the report's class.

- [ ] **The CPU-time limit reaches the same ladder** — ADR 0020 § 1
      (`docs/adr/0020-error-escalation-ladder.md:66`). `nvs_safepoint`'s `CPU_LIMIT` branch
      (`ctx.rs:2285`) sets its pending message and returns without asking `has_limit_handler` at
      all, so it is one `ctx.run_limit_handler()` above the `set_pending` — the same two lines the
      memory branch below it already carries. `[limits] cpu_time` still sits unread by `Ctx`; the
      reserve's time half (`fatal_reserve_time`, which § 1 names beside the memory one) has no
      directive row yet on purpose, so add it with whatever reads it and not before.
- [ ] **`LimitReport` is the argument the handler is handed** — § 1 spells the parameter
      `closure(LimitReport): void`. `run_limit_handler` passes `&[]` today and says so in its doc;
      a handler declaring a parameter is refused by `call_closure` and abandoned like any other
      failure of the handler's own, so this slice is what makes such a handler work at all. Decide
      whether the report is a `Core` instance class or a shape — `crates/nvs-stdlib/src/fatal.rs`
      module doc records why the registry row is `callable` either way — and it needs at minimum
      *which* limit was reached, since a handler now cannot tell memory from CPU.
- [ ] **A `.nvst` case over the whole ladder.** Nothing in `tests/conformance/` registers a handler
      and breaches: `examples/limits.nvs` exits 1 with no handler in sight. With the reserve in
      place a handler can now `echo`, so the case is expressible — a tight `[limits] memory` in a
      per-app block, a handler that prints, and `--EXPECT--` carrying its output before the `FATAL`.

## Backlog

- Item 18: `Core\Secret::reveal()` is not in the registry — `crates/nvs-stdlib/src/secret.rs`.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- Item 22: `Core\Script`'s members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `orient.py`'s `[context] modules` glob for `budget.rs` names a crate it never lived in —
  `docs/agent/loop-goal.toml`.
- Stage 4's `-p nvs-host` check still wants `a_recursive_spawn_is_reported_as_max_script_depth_and_not_as_memory`,
  `n_concurrent_isolates_cannot_together_exceed_the_trees_budget` and
  `a_child_cannot_widen_a_capability_its_parent_narrowed` — `crates/nvs-host/src/isolate.rs`.
