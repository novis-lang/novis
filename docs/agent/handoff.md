# Handoff

## State

**Stage 4's compile-time half is complete: both `Core\Task::all` and `Core\Task::map`
type-check.** ADR 0072 § 2's row (`crates/nvs-stdlib/src/task.rs:92`) needed no new machinery —
`CoreTy::Array(&CoreTy::Var("T"))`, `CoreTy::CallableTo("U")`, the shared `OPTIONS` at `task.rs:67`
and `CoreTy::Array(&CoreTy::Var("U"))` back — and that contrast is § 2's own argument for two
members rather than one, recorded in the module doc rather than restated anywhere else.

**`examples/tasks.nvs` now compiles whole and fails only at the placeholder.** Every block its
acceptance check freezes type-checks; `./target/debug/nvs run examples/tasks.nvs` prints the
placeholder's own line from `unimplemented_scheduler_half` (`task.rs:127`) and aborts. Both rows
share that one body, on purpose: what stops them is the same missing thing.

**What is missing is the seam, and nothing else.** No `Core` helper can reach the `nvs-host`
scheduler — `nvs_host::spawn_child` (`crates/nvs-host/src/scheduler.rs:882`) takes a `Ctx` and a
`TaskRoot`, and a helper holds the `Ctx` (`crates/nvs-runtime/src/ctx.rs:246`) but has no way to the
rest. `nvs-host`'s own `Cargo.toml:36` comment says the yielder already arrives through `Ctx` as an
opaque pointer, which is where to start.

**The orientation pack is still missing the ADR sections this stage is specified by.** `[context]
adrs` carries ADR 0072 §§ 4 and 5 only; § 2 had to be sliced by hand this session exactly as §§ 1
and 3 were last session. Add **§§ 1, 2 and 3** — every remaining Stage 4 item reads them. Item 11's
residue — no `Core\Task\Channel` row in `nvs_stdlib::registry` — is unchanged.

## Next group

**The scheduler seam, then a body on each row.** File set: `crates/nvs-stdlib/src/task.rs`
(`CLASS:81`, `address:110`, the placeholder at `:127`), `crates/nvs-runtime/src/ctx.rs:246`,
`crates/nvs-host/src/scheduler.rs` (`spawn_child:882`, `current_task:985`, `TaskTree:220`), and
`crates/nvs-runtime/src/closure.rs:132`.

- [ ] **The seam: how a `Core` helper reaches the host.** Decide it and record it in the module doc
      that owns it — no ADR slot is free, and `scheduler.rs`'s module doc is already the home of the
      task tree's design. The shape to weigh: a thread-local, as `reactor.rs` already chose for
      reaching the reactor, against a second opaque pointer in `Ctx`. `run_task`
      (`crates/nvs-runtime/src/abi.rs:579`) is the containment boundary the children run inside.
- [ ] **`nvs_core_task_all` running its children** — ADR 0072 §§ 1, 3, 4. Each field's closure is
      spawned with `spawn_child`, `nvs_runtime::call_closure` (`closure.rs:132`) invokes it, and the
      call does not return with work still running. `{deadline}` throws `TimeoutError`; § 5 says the
      teardown runs no script code.
- [ ] **`nvs_core_task_map` over the array** — § 2's "preserving the input's keys and order
      regardless of completion order", and `{limit}` as a shaper that schedules rather than throwing.
      `examples/tasks.nvs`'s `Gauge` block is the peak-not-total assertion this owes.

## Backlog

- No `Core\Task\Channel` row in `nvs_stdlib::registry` — Stage 3 item 11's language surface;
  `crates/nvs-host/src/channel.rs`'s module doc is the design.
- `examples/channel.nvs` acceptance check is unmet and needs that row first — `loop-goal.toml:1494`.
- Add ADR 0072 §§ 1, 2, 3 to `[context] adrs` in `docs/agent/loop-goal.toml`.
- M4 residue: the 1000-case conformance corpus count — `docs/implementation-plan.md`.
