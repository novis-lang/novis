# Handoff

## State

**Item 1 is closed at the refusal level, and the ceiling has fallen from 15 to 6.** The tree is
at **867 conformance plus 189 differential**. Nothing is blocked.

All nine `nvs-codegen` catch-alls item 1 claimed were engine invariants, not language holes: each
already carried a roster comment proving no lowering constructs what it matches, and each is now a
`CodegenError::Internal` through `nvs_codegen::emit::internal`, which is itself an `Internal` now
rather than an `Unsupported`. `crates/nvs-codegen/src/emit.rs`'s module doc is the home of the rule
in that crate; `docs/agent/loop-goal.md` § *Standing decisions* is its home overall.

`crates/nvs-ir/tests/refusals.rs` is unchanged in mechanism — `CEILING` is 6, the allowlist is still
empty and may never grow. The six that stand are item 4's one (`nvs-ir/src/lower/stmt.rs:269`), item
16's three and item 25's two; `python tools/holes.py --item N` prints any of them.

## Next group

**Item 16's three, the named/spread argument refusals.** These are the first of the six that are
*real* holes rather than classifications, and the standing decision orders them: the checker half
lands before the lowering half. Read the sites first — `python tools/holes.py --item 16` — because
this group's shape depends on whether each is a checker gap or a lowering one.
File set: `crates/nvs-types/src/expr/` (the checker half), `crates/nvs-ir/src/lower/call.rs` (the
lowering half), `crates/nvs-ir/tests/refusals.rs` (the `CEILING` constant).

- [ ] **Read item 16's three sites and split them checker-half / lowering-half**, then land the
      checker half: `crates/nvs-ir/src/lower/call.rs` is the anchor `holes.py` reports, and
      `crates/nvs-types/src/expr/operators.rs:403` is the neighbouring checker file the pack maps.
      ADR 0063 R2's options bag is what `call.rs`'s own module doc says it already flattens.
- [ ] **Land the lowering half in `crates/nvs-ir/src/lower/call.rs`**, and lower `CEILING` in
      `crates/nvs-ir/tests/refusals.rs:58` in the same slice — the test says so if you forget.
- [ ] **Item 25's two, `object` as a declared type has a representation arm** — same `CEILING`
      edit, and the standing decision already settles the design (`object` erases to the same
      pointer a named class does), so only the "does anything below read a class label" check is
      work.

## Backlog

- Item 4's one site is `nvs-ir/src/lower/stmt.rs:269`, a control-flow-slice refusal whose message
  is about local declarations rather than about bitwise operators — check the attribution before
  taking it (`docs/agent/loop-goal.md` item 4).
- `docs/agent/guard-name-debt.md` is at 0; the file can go when nothing references it.
- The `[context]` manifest wanted nothing this session did not have; `docs/agent/loop-goal.toml`
  needs no new selector.
