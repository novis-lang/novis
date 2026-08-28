# Handoff

## State

**ADR 0007 § 4's arithmetic table is closed at the operand end, and
`emit_binop`'s operator catch-all has no reachable target left.** An operand
naming no arithmetic row is `E0716` where it is written and `%` over a `float`
is `E0717`, both mirrored by `nvs_runtime::helpers::value_arith`'s catchable
throw at the tagged end. The plan's `Open now` carries the reasoning; ADR 0007
§ 4's body carries the rule.

- **Item 26 is closed and needed no lowering.** `bool as string` already ran
  and `$yes as int` was already `E0708`; what was stale was
  `examples/targets.nvs:48`, which now spells the predicate out loud, and the
  goal's own check comment, which claimed three conversion rows where one is a
  branch. `loop-goal.toml` lost `a_bool_converts_to_an_int_and_to_a_string`
  (there is nothing in `nvs-ir` to test) and gained the three `nvs-types` names
  this session wrote in `crates/nvs-types/tests/numeric.rs`.
- **`examples/targets.nvs` is still red, one line earlier**: `$box[1]` over a
  `mixed` is `E0482`, which is item 24's subscript-through-a-tag half and is
  *not* landed despite the plan's item-24 paragraph. That is the next thing
  standing between this fixture and its stage-4 check.
- **`nvs-codegen` has no unlowered-shape test any more**, and the comment where
  the two stood (`crates/nvs-codegen/tests/unit_layout.rs:42`) says why: every
  `CodegenError::Unsupported` in `emit_binop` is now an internal-consistency
  check, and hand-building an IR to reach one would contradict that file's own
  module doc.
- **`orient.py`'s pack was complete for both items.** The two standing manifest
  gaps are unchanged — `[context] modules` has no `nvs-runtime` and no
  `nvs-diagnostics` entry.

## Next group

**The `mixed` operand's remaining holes and the roster comments' testable
claims, over the three files this session already opened.** The files:
`crates/nvs-types/src/expr/operators.rs`, `crates/nvs-codegen/src/emit.rs` and
`crates/nvs-ir/src/lower/expr.rs`.

- [ ] **A subscript through a tagged base** — `examples/targets.nvs:39`'s
      `elem=2`, refused today by `E0482` at `crates/nvs-types/src/expr/mod.rs`'s
      subscript check. ADR 0007 § 5 with ADR 0036 § 4's deferral: a `mixed` base
      defers *whether* there is an array, so the read is the tag's question, not
      the site's. The plan's item-24 paragraph claims this and the tree does not
      have it.
- [ ] **A `void` call as an operator's operand** —
      `class H { public static function n(): void {} } H::n() + 1;` dies with
      *"does not lower an operand used before it is defined"*, an internal
      message naming nothing. `equality_domain` answers `None` for `Ty::Void`,
      so `reject_unrowed_arithmetic_operand`
      (`crates/nvs-types/src/expr/operators.rs:547`) lets it through on purpose;
      the `as` table already refuses a `void` on either side (`E0708`), so this
      is the same rule one operator over.
- [ ] **A `.nvst` case for the five roster comments' testable claims** —
      `crates/nvs-codegen/src/emit.rs:1165` and `:1310` each name a roster; the
      claims that a program can reach are the `E0706`/`E0716`/`E0717` refusals,
      and `tests/conformance/lang/the-arithmetic-table-is-closed.nvst` is the
      shape to extend rather than a second file.

## Backlog

- Item 24's subscript-through-a-tag half — `docs/agent/loop-goal.md`'s item list.
- `Ty::Void` as an operand of any operator, not just an arithmetic one — same file.
- `[context] modules` has no `nvs-runtime`/`nvs-diagnostics` entry — `docs/agent/loop-goal.toml`.
- `emit_binop`'s representation catch-all still names `Void` as unreachable; the probe above
  suggests a `void` operand dies earlier instead — `crates/nvs-codegen/src/emit.rs:1165`.
