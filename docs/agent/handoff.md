# Handoff

## State

**Goal `test-doubles` — `Core\Test`'s double half. Nothing of the member is on disk yet**, and what
this session changed is the goal's own stage order, not code.

Stage 2 was the checker and stage 3 the runtime. A registry row cannot be registered before its helper
exists (the playbook bullet this session added names the two sweeps), so **stage 2 is now the two rows
plus the runtime plus one `core/` case, and stage 3 is the four refusals**. `docs/agent/loop-goal.md`,
`docs/agent/loop-goal.toml` and their sources under `docs/agent/goals/` carry it; `[context.stage.2]`
and `[context.stage.3]` and the `[[check]]` blocks were swapped with the prose.

The driver's red acceptance line — *no recorded gap is owed by anyone but a future milestone* — is goal
`gap-zero`'s own check (`docs/agent/goals/70-gap-zero.toml:185`) and not this goal's. It stays red
until `test-doubles` and `bigint` close the gaps that name them; it is not a regression to chase.

Settled and verified for the next group: `CoreTy::Written("T")` is the written type parameter
(`crates/nvs-stdlib/src/registry.rs:554`); `WRITTEN_CLASS_MEMBERS` is how a call site's `T` reaches a
helper as a descriptor; `MethodRow` carries no data word beside `code`, so a trampoline holds its slot
in its own identity.

## Next group

**Stage 2: the rows and the runtime, one group because neither lands alone** — one file set:
`crates/nvs-stdlib/src/test.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/instance.rs`, `crates/nvs-runtime/src/object.rs`,
`crates/nvs-runtime/src/closure.rs`. Take `double` alone if the group runs long — `partial`'s ratchet
line may be struck a session later.

- [ ] **The double's descriptor, and what a trampoline carries** — the pick stage 2's prose leaves
      open, recorded in `crates/nvs-stdlib/src/test.rs:1`'s module doc as it is made.
      `crates/nvs-runtime/src/object.rs:1847`'s `ClassTable::define` and `:2287`'s `set_methods` are
      the builder; `crates/nvs-runtime/src/object.rs:625`'s `MethodRow` is what a row holds, and its
      `native` flag is the one `crates/nvs-stdlib/src/instance.rs` already sets. `rule:testing/doubles`.
- [ ] **The `double` row, its card, its `address()` arm and its helper** —
      `crates/nvs-stdlib/src/test.rs:321`'s `CLASS` takes the row (`params` the shape, `return_ty`
      `CoreTy::Written("T")`), the card goes after `crates/nvs-stdlib/src/test.rs:1289`'s
      `EXPECT_FAILURE_DOC`, the arm at `crates/nvs-stdlib/src/test.rs:1853`, and the ratchet line at
      `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:29` is struck in the same
      edit. `rule:testing/doubles`.
- [ ] **The interface's descriptor reaches the helper** —
      `crates/nvs-stdlib/src/registry.rs:3030`'s `WRITTEN_CLASS_MEMBERS` gains the row, and
      `crates/nvs-types/src/expr/args.rs:1671`'s `written_class_of` has to admit an interface where it
      reads `Ty::Class` today. `rule:testing/doubles`.
- [ ] **The case ADR 0079 § 10 writes** —
      `tests/conformance/core/a-double-is-passed-where-its-interface-is-taken.nvst`, the `Clock`
      example, which is also what `crates/nvs-stdlib/tests/conformance_coverage.rs:52` wants for the
      new row. `rule:testing/doubles`.

## Backlog

- Stage 3's four refusals and its two new `E08xx` codes — `docs/agent/loop-goal.md` § *Stage 3*.
- `partial`'s row, helper and ratchet line, if stage 2's group takes `double` alone — same file.
- Stage 0's `test.rs` module-doc section on the double half — `docs/agent/loop-goal.md` § *Stage 0*.
- The `core/` case naming `Core\Test::partial<` that `conformance_coverage.rs` will want with it.
