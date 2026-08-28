# Handoff

## State

**ADR 0036 § 4's deferral now reaches a subscript, and `examples/targets.nvs`
is green end to end** — every line of Stage 4's `want`, `converted=3` included.
A `mixed` base's `$box[1]` is answered from the operand's runtime tag by
`Helper::ValueIndexGet`/`ValueIndexOptionalGet` over one implementation
(`nvs_runtime::helpers::value_index`); every base whose declared type already
answers, and every `mixed` *write* target, keeps `E0482`. ADR 0007 § 5's body
carries the rule; the plan's `Open now` carries the reasoning.

- **`examples/targets.nvs:53` was wrong, not unimplemented.** `array<int> as
  array<string>` is ADR 0007 § 2's per-element *check* and throws at element 0,
  so the fixture converts to `array<mixed>` and says why in a comment.
  `loop-goal.toml`'s Stage 4 comment records the same correction.
- **A `catch` binding has no callable members** —
  `$e->getMessage()` panics `nvs-ir` at `lower/expr.rs:2281` for `Throwable`
  *and* for a named class. New, unrelated to this item, and in `## Backlog`.
- **`orient.py`'s pack was complete for this item.** The two standing manifest
  gaps are unchanged — `[context] modules` has no `nvs-runtime` and no
  `nvs-diagnostics` entry, and both were files this slice edited.

## Next group

**The two remaining group items, over the operator files this session did not
open plus the one it did.** The files: `crates/nvs-types/src/expr/operators.rs`,
`crates/nvs-ir/src/lower/expr.rs` and `crates/nvs-codegen/src/emit.rs`.

- [ ] **A `void` call as an operator's operand** — ADR 0007 § 4's table has no
      row for a value that is not one, and `E0707` already refuses a `void`
      call at an implicit `.`; the arithmetic and ordering rejections
      (`reject_unordered_operand`, `reject_arith_operand` in
      `crates/nvs-types/src/expr/operators.rs`) should name it the same way
      rather than letting it reach a representation the backend has no row for.
- [ ] **A `.nvst` case for the five roster comments' testable claims** — the
      catch-all rosters in `crates/nvs-codegen/src/emit.rs` (`emit_binop`,
      `emit_unop`) and `crates/nvs-ir/src/lower/expr.rs:@lower_expr` each claim
      a closed list; the *Agreement* shape in `docs/agent/conventions.md` is the
      one that asserts they agree rather than what each answered.
- [ ] **A `.nvst` case for the tagged subscript** — this session's item is
      pinned by `a_subscript_through_a_tagged_base_lowers`
      (`crates/nvs-ir/src/lower/tests.rs`) and by the fixture, but nothing
      pins the *output* of the four rows (element, absent key, non-array base,
      both under `??`) byte for byte. `tests/conformance/lang/` is the home.

## Backlog

- A `catch` binding's methods do not lower — `$e->getMessage()` panics
  `nvs-ir` at `crates/nvs-ir/src/lower/expr.rs:2281`. ADR 0002 owns what a
  `Throwable` exposes; the synthesized class has none of it.
- An element **write** through a `mixed` base is still `E0482` — the deferral's
  other half, and it needs a holder to write the separated buffer back
  through. ADR 0007 § 5's new paragraph says so.
- `[context] modules` in `docs/agent/loop-goal.toml` names neither
  `nvs-runtime` nor `nvs-diagnostics`, both of which this goal edits routinely.
- `docs/spec/02-php-migration.md`'s score, via `python tools/check-migration.py`.
- Virtual dispatch by slot and a `br_table` for a dense `switch` are M12
  (`docs/agent/loop-goal.md` § *Standing decisions*).
