# Handoff

## State

**`[limits] max_script_depth` is read, and an isolate carries its depth — the refusal is not
written.** The directive is `System` on the reserve's grounds (`crates/nvs-config/src/tree.rs`,
`crates/nvs-config/src/directive.rs`, `crates/nvs-config/src/value.rs`'s `unit_of`), and
`Ctx::max_script_depth` is its reader. It is the only ceiling in `[limits]` whose *unstated* reading
is a number rather than "no cap": `Ctx::DEFAULT_MAX_SCRIPT_DEPTH` is 64, a malformed value takes it
too, and only an explicit `false` reads as 0. `Ctx::max_script_depth`'s field doc is the only home of
why that asymmetry exists — an unbounded recursion of isolates does not run forever, it exhausts the
tree's heap, which is the misreport m6.md's *Verify* asks this key to remove.

**`Ctx::isolate` counts the depth and copies the ceiling down.** `Ctx::script_depth` is 0 for the
request and one more per isolate; the ceiling is copied rather than re-read from the cloned
configuration, so a parent that narrowed it narrowed it for the tree beneath. That constructor's
comment owns the direction.

**Nothing enforces any of it yet.** There is no `script_depth_breach`, no check at
`Isolate::start`, and `Limit` (`crates/nvs-runtime/src/ctx.rs:916`) still has only `Memory` and
`CpuTime` — so a recursive spawn is still stopped by the heap and still *reported* as memory. That
is the driver's failing check, and it stays failing until the group below lands: all three of the
names `nvs-host (limits at a safepoint)` lists for item 12 are still unwritten.

`orient.py`'s `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed —
the accounting is `crates/nvs-runtime/src/budget.rs`. The pack warns every session.

## Next group

**Item 12's refusal, which is the only thing between the tree and the driver's failing check.** File
set: `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-host/src/isolate.rs` and
`crates/nvs-host/tests/limits.rs`. The first slice carries the one design call left, and it is small:
a depth breach is a third `Limit`, so the handler's report names `max_script_depth` literally and the
test name stops being a claim about a message.

- [ ] **`Limit` gains a `ScriptDepth` variant, and `Ctx` gains the breach it reports** — add the
      variant beside `CpuTime` (`crates/nvs-runtime/src/ctx.rs:916`); its `name()` arm returns the
      directive spelling `"max_script_depth"`, which is what makes the report branchable. Then a
      `script_depth_breach(&self) -> Option<Fault>` beside `memory_breach`
      (`crates/nvs-runtime/src/ctx.rs:1253`), answering `Some` when `script_depth + 1` would pass a
      non-zero `max_script_depth`, with `Fault::fatal` naming both the depth and the ceiling the way
      `memory_breach`'s message names both. A case joins
      `crates/nvs-runtime/tests/configured_limits.rs`.
- [ ] **A `spawn script` past the ceiling is refused before the child is built** — the check goes at
      the top of `Isolate::start` (`crates/nvs-host/src/isolate.rs:153`), ahead of the `copy_graph`
      on line 163, so a refusal costs nothing and leaves no half-made isolate. Deliver it the way the
      safepoint's memory branch does (`crates/nvs-runtime/src/ctx.rs:2698`):
      `ctx.run_limit_handler(Limit::ScriptDepth)` then `ctx.set_pending(message)`, and return a
      `Collected` carrying a failed `Completion` (`crates/nvs-host/src/isolate.rs:225`) rather than a
      `GraphError` — that error is the argument's, and the module doc says so.
- [ ] **`a_recursive_spawn_is_reported_as_max_script_depth_and_not_as_memory`** — in
      `crates/nvs-host/tests/limits.rs`, which is where the driver's check looks for it.
      `a_memory_cap_terminates_a_runaway_script_as_a_fatal`
      (`crates/nvs-host/tests/limits.rs:231`) is the shape, and the file's own module doc (lines
      12-15) says a hand-built closure is a whole one here. Assert **both** halves the name promises:
      the status is `FATAL`, and the report the handler was given reads `max_script_depth` and not
      `memory` — the second is the whole point, and a case asserting only the `FATAL` would pass
      against the heap stopping it.

## Backlog

- Item 12's other two check names — `n_concurrent_isolates_cannot_together_exceed_the_trees_budget`
  and `a_child_cannot_widen_a_capability_its_parent_narrowed` (docs/agent/loop-goal.toml § 4).
- Nothing samples a CPU clock against `Ctx::cpu_limit`; the flag is raised by tests alone
  (`Ctx::cpu_limit`'s field doc).
- `Core\Secret::reveal()` is not in the registry — item 18 (docs/implementation-plan.md).
- `Live::admit`'s same-class check asks the answer, not the argument
  (`crates/nvs-runtime/src/graph.rs` § *Known gaps*).
- Item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`).
- `orient.py`'s `[context] modules` names a `crates/nvs-host/src/budget.rs` that never existed.
