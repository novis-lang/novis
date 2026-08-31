# Handoff

## State

**Stage 10 item 38's first slice is closed — `class<T>` lowers, and ADR 0125 § 2's two `as` rows
run.** `Ty::ClassRef` erases to `Ty::ClassDesc` in both directions the crate translates a type
(`crates/nvs-ir/src/lower/mod.rs:2748`'s `lower_decl_type` and its `erase_checked_ty` twin), so a
descriptor now reaches a local, a parameter, a field and a return — `Ty::ClassDesc`'s own doc
comment is the home for what that costs and what the erasure drops.

**§ 2's rows are one function and one instruction.** `Lowering::lower_class_reference`
(`crates/nvs-ir/src/lower/convert.rs:886`) folds a written-out `Foo::class` to the
`InstKind::ClassDescConst` `Foo::bar()` already bakes, and sends everything else through
`InstKind::ClassDescIn` (`crates/nvs-ir/src/ir.rs:593`) — whose null answer it turns into ADR 0007
§ 2's throw. Codegen answers that instruction from the closed set `Classes::conforming_to`
(`crates/nvs-codegen/src/lib.rs:741`) reads off the unit's own hierarchy: a branch-free chain of
`nvs_str_eq` calls for the string door and of `icmp` for the `class<U>` narrowing, no runtime
function and no name registry. Both rows were run end to end under `nvs run` before the wrap.

**What item 38 still owes is the three dynamic sites**, and the seam is unchanged from the last
session: the checker records nothing in the expr table for `new $cls()`, `$cls::f()` or
`$x instanceof $cls`, deliberately and with both record sites guarded, so `nvs-ir` finds no entry
and panics. That is what the next group closes, and item 39's `.nvst` cases wait on it.

**`as ?class<T>` has no row and needs a representation decision**, which ADR 0125 § 2 promises and
nothing yet delivers: both spellings reach `convert_or_null`'s existing refusals today.
`Lowering::lower_class_reference`'s *Known gaps* names the shape that would work — the null
descriptor `ClassDescIn` already produces, which makes `?class<T>` a `Ty::ClassDesc` rather than a
`Ty::Tagged` and needs `null`'s comparison rows against that representation.

**`orient.py`'s `[context]` gaps.** Unchanged and standing: the pack prints the goal item but not
the `[[check]]` grading it; no field selects `docs/reference/lang/*.md` (item 39 needs it);
`docs/adr/README.md` and `ground-rules.md` are not in `modules`. In `adrs`: **0125 §§ 4-5** for the
next group. The `crates/nvs-host/src/budget.rs` warning is the documented forward anchor.

## Next group

**Item 38, the three dynamic sites — file set `crates/nvs-types/src/expr_table.rs`,
`crates/nvs-types/src/expr/calls.rs`, `crates/nvs-types/src/expr/members.rs` and
`crates/nvs-ir/src/lower/expr.rs`.** The checker has to record `T` somewhere the guarded `New`/
`Call`/`InstanceOf` entries cannot hold it, and lowering has to read whatever it records.

- [ ] **`new $cls(...)` records its class reference and lowers to `NewDynamic`.** The entry is the
      one `crates/nvs-types/src/expr_table.rs:353`'s `New` guard refuses to be, so it is a variant
      of its own carrying the bound rather than a written class; `check_new_target`
      (`crates/nvs-types/src/expr/calls.rs:1274`) is where `T` is in hand, and
      `crates/nvs-ir/src/lower/expr.rs:212` reads it back into `InstKind::NewDynamic` with the
      operand's descriptor as `desc`. ADR 0125 § 4. The goal check names
      `a_new_through_a_class_reference_lowers_to_new_dynamic`.
- [ ] **`$cls::f()` lowers to `CallVirtual` on the descriptor in hand.** `infer_static_call`
      (`crates/nvs-types/src/expr/calls.rs:212`) resolves the member on `T`'s roster already; what
      is missing is the record and `crates/nvs-ir/src/lower/expr.rs:212`'s neighbouring call arm
      reading it. `lsb` is the shape to copy — a `CallVirtual` whose `lsb` is the operand rather
      than the frame's. ADR 0125 § 4.
- [ ] **`$x instanceof $cls` tests two descriptors.** `infer_instanceof`
      (`crates/nvs-types/src/expr/members.rs:281`) accepts the operand without consulting `T`, and
      `crates/nvs-types/src/expr_table.rs:615`'s `InstanceOf` entry names a written class;
      `crates/nvs-ir/src/lower/expr.rs:263` needs `ClassDescOf` on the subject plus a compare
      against the operand, which is `InstKind::ClassDescIn`'s descriptor arm with the bound taken
      from the operand instead. ADR 0125 § 4.

## Backlog

- Item 39: the four `.nvst` cases the stage 10 check names, plus the reference section —
  `docs/agent/loop-goal.toml`'s stage 10 `nvs-suite` check lists the paths.
- `as ?class<T>` needs a representation decision before it can lower —
  `crates/nvs-ir/src/lower/convert.rs:886`'s *Known gaps*.
- § 2 asks for the offending class in the conversion's throw message; the message names the bound
  instead — same *Known gaps*.
- `class<Animal>` is not a row in `crates/nvs-ir/tests/type_atoms.rs`'s atom ratchet, so the shape
  is not held by that gate.
- Stage 8: conformance 1087, differential 206 of 210, migration 37% — `docs/plan/m6.md`.
