# Handoff

## State

**M4 — language completeness.** The write path's two evaluation questions are both closed, and
measured against PHP 8.5.9 rather than reasoned about. `lower_read_modify_write`'s assertion
(`crates/mwl-ir/src/lower/stmt.rs:517`) has **no reachable target**: the parser's
`Parser::require_write_target` admits four kinds, `mwl_types::expr::assign::check_write_target`
then refuses the nullsafe one (`E0479`) and the element write whose root is not a place (`E0700`),
and `Self::stage_target_address` stages the one level each surviving composite shape carries that
is not re-readable. That function's own doc comment is the proof's only home; the assert now reads
as a gate-admitted-something-it-does-not-model tripwire rather than a known gap.

**An element write evaluates the receiver under its root holder exactly once.** It used to run it
twice — `write_back_array` re-points the holder by lowering that receiver again — so
`$b->self()->rows["k"] = "z"` printed `self()` twice, the nested
`$b->self()->grid["r"]["k"] .= "b"` three times, and `(new Box())->rows["k"] .= "b"` constructed
two objects. `lower_store`'s element arm now stages the root's address itself, and
`stage_address_of` descends a property or element level instead of staging that level's *value*,
which is what left the slot visible to the write-back at all. `mwl-ir`'s gap 16 records the whole
shape and now names only `**=` as open.

**One direction of ADR 0007 § 7 row 15 runs the other way, and the row says so now**: 8.5.9
refuses `(new Box())->rows["k"] = "z"` at compile time (*"Cannot use temporary expression in write
context"*) while accepting `make()->rows["k"] = "z"`. MWL accepts both — the field is a slot in a
heap object either way — and `is_a_place`'s doc comment no longer claims PHP agrees.

Two conformance cases pin the counts (`a-compound-assignments-target-is-evaluated-once.mwlt`,
`an-element-writes-holder-is-evaluated-once.mwlt`), and `tools/leak-check.sh` is clean over five
scratch shapes covering both fixes.

## Next group

**The three erased-or-unrecorded receiver panics left in the write path, all in one pair of
files.** The file set: `crates/mwl-ir/src/lower/stmt.rs`, `crates/mwl-ir/src/lower/mod.rs`,
`crates/mwl-types/src/expr/assign.rs`, `tests/conformance/lang/`.

- [ ] **`stmt.rs:933` — a property assignment target with no declaring class recorded.** The
      panic names an ADR 0036 § 4 erased receiver (a shape's field, a plain `object`), and
      `check_write_target` (`crates/mwl-types/src/expr/assign.rs:443`) refuses only the *element*
      write through one (`E0480`) — a plain `$o->p = v` through an erased receiver has no refusal
      in front of it. ADR 0036 § 4 says a write through an erased view is a checked, catchable
      throw whose incoming value is checked against the field's real type, so the decision is
      lower-it-by-name or take a new `E07xx` code; either way it stops being a panic. Anchors:
      `crates/mwl-ir/src/lower/stmt.rs:933`, `crates/mwl-types/src/expr/assign.rs:443`.
- [ ] **`mod.rs:2004` — the same question inside `write_back_array`.** Its message already
      argues it is unreachable (`check_write_target` refusing the erased root as `E0480`), so this
      is today's item one file over: prove it dead in its own doc comment, or find the program
      that reaches it. Anchors: `crates/mwl-ir/src/lower/mod.rs:1964` (the function),
      `crates/mwl-ir/src/lower/mod.rs:2004` (the panic).
- [ ] **`stmt.rs:1238` — an intermediate level of a nested element write with no element type.**
      `row_ty_of`'s panic blames a base that erased to `mixed`; `E0482` refuses a base declaring no
      element type, so the open question is whether a `mixed` level can still arrive. Anchors:
      `crates/mwl-ir/src/lower/stmt.rs:1231` (the function), `:1238`, `:1248` (the not-an-array
      assert beside it).

## Backlog

- `**=` does not lower — `ir::BinOp` has no `**` row (`mwl-ir` gap 16).
- `array<T> as array<U>` does not lower (`crates/mwl-ir/src/lower/expr.rs:877`), which is what
  blocks a case from indexing past the first level of an `array<mixed>` (playbook).
- An abandoned generator's `finally` never runs (`mwl-ir` gap 18; pre-authorized in
  `docs/agent/loop-goal.md` § *Standing decisions*).
- A named or spread call argument does not lower (`crates/mwl-ir/src/lower/call.rs:75`).
- `Class::method(...)` as a first-class callable panics `mwl-ir` (gap 1).
- **`orient.py` printed no section of ADR 0007 § 5 or § 7**, and both were needed: § 5 owns the
  copy-on-write separation this group's every slice is about, § 7 row 15 the divergence it edits.
  Add `0007:5` and `0007:7` to `[context] adrs` in `docs/agent/loop-goal.toml`.
