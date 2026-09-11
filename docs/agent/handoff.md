# Handoff

## State

**Goal `finish-response` — stage 3's three confirmation items are settled, and no stage-3 code exists
yet.** The marker-object line is workable exactly as the goal specifies it, so § *Standing decisions*'
`Throwable`-subclass fallback is not needed and should not be taken.

**The pending slot is a place for an object, not for a `Throwable`.** `Pending::Thrown` holds one
nullable reference with no class bound (`crates/nvs-runtime/src/ctx/error.rs:147`) and `Ctx::raise`
records whatever it is handed (`crates/nvs-runtime/src/ctx/error.rs:310`). Every read and write of one
is guarded on how *wide* the class is, never on what it descends from — `Thrown::message`, `field`,
`push_frame` and `write_location` each answer emptily below `SLOT_COUNT`
(`crates/nvs-runtime/src/throwable.rs:531`, `:563`, `:625`, `:669`) — while `Ctx::pending_conforms_to`
(`crates/nvs-runtime/src/ctx/error.rs:422`) answers off `ClassDesc::conforms_to_name`
(`crates/nvs-runtime/src/object.rs:900`), which is self-or-ancestor by name. A parentless class is
therefore false against every `catch` naming anything in the tree, and `Pending`'s own doc now states
this. Guard: `a_pending_object_outside_the_throwable_tree_unwinds_intact_and_matches_no_catch`.

**The landing block's one question is `status == THROWN`** (`crates/nvs-ir/src/ir.rs:2624-2651`):
`handler` on `THROWN`, `onward` on every other non-`OK` status, and `onward` releases the frame's
locals and ends in `Propagate`. `handler` is the block that runs this region's `finally`
(`crates/nvs-ir/src/lower/mod.rs:239-253`). So a fifth status takes `onward` at every enclosing region
and runs no `finally` at all — it is `EXITED`'s own edge, which is the goal's named trap, mechanically.
**The marker travels as `THROWN`.**

**`InstanceOf` misses it for free.** `emit_instanceof` (`crates/nvs-codegen/src/emit.rs:2531-2556`) is
a `nvs_object_instanceof(subject, desc)` descriptor test and nothing more. No special case in the arm
walk, no widening.

## Next group

**Stage 3: the marker class, and the raise that carries it through every `finally`** — one file set:
`crates/nvs-hir/src/errors.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/lower/mod.rs`.
Tag it stage 3 so `[context.stage.3]`'s overlay applies.

- [ ] **Seed the marker class as a second root of `TREE`**, one row with `None` for its parent, at
      `crates/nvs-hir/src/errors.rs:78`. `ROOT`'s doc at `crates/nvs-hir/src/errors.rs:93` says every
      other row descends from `Throwable`, which a second root makes false — rewrite it, and grep
      `TREE`'s consumers for the same single-root assumption before adding the row.
      `rule:errors/escalation-ladder`.
- [ ] **Rewrite `Terminator::Throw`'s operand doc** at `crates/nvs-ir/src/ir.rs:2586`, which still
      says the raised value is "an instance of `Throwable` or one of its subclasses". Only the
      sentence changes: the terminator's shape, its `source` carrier and its `landing` edge are all
      already right for the marker, and `write_location` no-ops on a class with no `location` slot.
      `rule:errors/propagation`.
- [ ] **Raise the marker as an ordinary `Terminator::Throw` from `lower`**, against
      `crates/nvs-ir/src/lower/mod.rs:239`'s `TryFrame::handler`: every enclosing region's
      finally-and-re-raise block then runs, and the dispatch's no-arm-matched default hands the same
      reference onward, which is the unwind the goal asks for. `rule:errors/propagation`.

## Backlog

- Stage 3's third check is the `-p nvs-types` refusal (a `catch` arm naming the marker, a `throw` of
  it); `E0780`'s band is full, so the sibling comes from `E08xx`. — docs/agent/loop-goal.toml
- `examples/finish.nvs`, which the driver's acceptance check reports missing, is stage 4's fixture and
  waits on the member. — docs/agent/loop-goal.md
- The `-p nvs-codegen` finally-ordering tests (`a_finish_runs_them_innermost_first` and its two
  siblings) land once the raise exists. — docs/agent/loop-goal.toml
