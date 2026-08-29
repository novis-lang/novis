# Handoff

## State

**Stage 4's compile-time half is on disk: `Core\Task::all` type-checks, and all three
`nvs-types` names the stage's `cargo-named` check asks for are green**
(`docs/agent/loop-goal.toml:1479`). The registry gained one class and one member.

ADR 0072 § 1's per-field typing needed a second binding site beside `CoreTy::CallableTo`, and
that is the only new idea here: **`CoreTy::CallableShapeTo("S")`**
(`crates/nvs-stdlib/src/registry.rs:230`, interned as `Ty::CallableShapeTo` at
`crates/nvs-types/src/ty.rs:171`) declares that a parameter is a shape literal of written `fn`
literals and that the shape of *their* results binds `S`. Both variants' doc comments are the
design's home; `generics.rs`'s module doc § *The one variable that is not at a position* now
covers the pair. `bind_callable_shape` (`crates/nvs-types/src/expr/args.rs:799`) is the one
place a field is read and the one place `E0773`/`E0774` are reported, which is why the variant
substitutes to `mixed`: that position is already checked in full.

**The body is a placeholder and says so at its own site**
(`crates/nvs-stdlib/src/task.rs:93`). A program that writes `Core\Task::all({...})` compiles
and reaches it; running the closures as children needs `nvs_host::spawn_child`
(`crates/nvs-host/src/scheduler.rs:882`) reachable from a `Core` helper, which nothing is yet.
`examples/tasks.nvs` therefore still fails its acceptance check — **that is not a regression**,
it is the next group.

**The orientation pack was missing what this item was specified by.** `[context] adrs` carries
ADR 0072 §§ 4 and 5 only; §§ 1 and 3 had to be sliced by hand, and both are load-bearing for
every remaining Stage 4 item. Add them. Item 11's own residue — no `Core\Task\Channel` row in
`nvs_stdlib::registry` — is unchanged.

## Next group

**`Core\Task::map` and then the scheduler seam under both members.** File set:
`crates/nvs-stdlib/src/task.rs` (`CLASS:67`, `OPTIONS:53`, the placeholder at `:93`),
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-host/src/scheduler.rs:882`, and
`crates/nvs-types/tests/core_members.rs`.

- [ ] **`Core\Task::map`'s row and its § 2 typing** — ADR 0072 § 2. `map(array<T>, callable,
      {limit?, deadline?}): array<U>` is the shape `Core\Arr::map` already has
      (`crates/nvs-stdlib/src/arr.rs:110`): `CoreTy::Array(&CoreTy::Var("T"))` plus
      `CoreTy::CallableTo("U")`, reusing `OPTIONS` at `task.rs:53`. Needs **three**
      `tests/conformance/` cases naming `Core\Task::map(` — the floor gate, and the new
      playbook bullet says why they may be `--EXPECTF-ERROR--` cases.
- [ ] **The scheduler seam: how a `Core` helper reaches the host.** `nvs_host::spawn_child`
      takes a `Ctx` and a `TaskRoot`; a helper has the `Ctx` and nothing else. Decide and
      record it in `crates/nvs-runtime/src/ctx.rs`'s module doc — a standing decision covers
      this, so do not stop for it.
- [ ] **`nvs_core_task_all` running its children** — ADR 0072 §§ 1, 3, 4. Replaces
      `unimplemented_scheduler_half` (`crates/nvs-stdlib/src/task.rs:93`), and the shape
      argument arrives as one `Tag::Obj` because `nvs-ir` lowers an `ObjectLiteral` outside an
      options position as an ordinary shape. Closes the `examples/tasks.nvs` `all=3` line.

## Backlog

- `Core\Task\Channel`'s registry row — item 11's language surface (`registry.rs:872`).
- `examples/channel.nvs`'s acceptance check, which needs that row.
- `[context] adrs` in `docs/agent/loop-goal.toml` is missing ADR 0072 §§ 1-3.
- `Core\Task::first` — ADR 0072 § 3 defers it and names the spelling; not this goal's.
- `Core\Task` has no `docs/spec/` § of its own; ADR 0072 is its only home.
