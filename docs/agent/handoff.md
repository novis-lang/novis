# Handoff

## State

**Goal 22 — the on-disk artifact cache has a producer — has just started; nothing of it has landed
yet.** Goal 21's whole list is this goal's Stage 1 floor.

The store is already written: `crates/nvs-cli/src/cache.rs` is ADR 0042 as specified, with its own
tests, and **no caller anywhere**. What is missing is the payload, and that module's own known gap is
the whole diagnosis — read it before anything else, because it names the three `emit.rs` sites that
make a page dump wrong in the next process and is why the answer is a second `Module` rather than a
serializer.

## Next group

**Stage 2: the keystone — `nvs-codegen` emits the same IR a second way.** One file set:
`crates/nvs-codegen/src/lib.rs` and `crates/nvs-codegen/src/emit.rs`.

- [ ] **A named symbol for the class-descriptor address** — `crates/nvs-codegen/src/emit.rs`, the
      `class_desc` lowering. It is an `iconst` immediate today and must become a relocation against a
      symbol; under `JITModule` that symbol resolves to the address it bakes now, so nothing on the
      hot path changes.
- [ ] **The same for the `instanceof` lowering and for `method_address`** — the other two sites, in
      the same file, and the reason a byte-perfect page dump would be wrong rather than merely
      unavailable.
- [ ] **`cranelift-object` behind the same `nvs_ir::Program` walk** —
      `crates/nvs-codegen/src/lib.rs:449`'s `compile` gains a sibling, not a fork. If the walk cannot
      be shared the goal stops and says so: a second lowering is a second semantics.

## Backlog

- Stage 3 is the wiring — `crates/nvs-cli/src/runner.rs:384` and `crates/nvs-cli/src/script.rs:84` —
  and it cannot start before stage 2 produces something to store.
- Stage 4's bench is what decides whether any of this was worth it. If warm does not beat cold, the
  standing decision is to delete `cache.rs` and retire ADR 0042 rather than keep it.
- Gaps this goal does not take live in `docs/agent/carried-gaps.md`, not here.
- When this goal's last check goes green the driver takes goal 7 — the post-parity temp sweep.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
