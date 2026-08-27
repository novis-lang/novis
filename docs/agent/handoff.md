# Handoff

## State

**M4 — language completeness**, and **item 22's own hole is closed**. A nested
`$grid[0][1] = v` writes back: `lower_reassignment`'s `Index` arm
(`crates/mwl-ir/src/lower/stmt.rs:796`) flattens the target to its root plus one key per
level, lowers the root and every key exactly once, descends with
`Helper::ArrayRowForWrite`, and emits the `ArraySet`s back up with the outermost last, so
`write_back_array` (`crates/mwl-ir/src/lower/mod.rs:1801`) is handed the *root* and its
two-holder rule is unchanged. Three levels, an int subscript, a variable key, an
innermost append and a property root all run.

**Auto-vivification landed with it and was not a choice.** `InstKind::ArrayGet` borrows
*and* answers an absent key with a null-shaped value, so a descent through one aborted the
process (`exit 127`, playbook § Tooling). `Helper::ArrayRowForWrite`
(`crates/mwl-ir/src/ir.rs:1204`, `mwl_array_row_for_write` at
`crates/mwl-runtime/src/helpers.rs:139`) answers with exactly one owned reference either
way — a retain of the row that was there, or a fresh empty array — so the lowering emits
no retain beside it and needs no branch. `$g[9][0] = 1` over an empty `$g` is now
byte-identical to PHP's `json_encode` of the same statements.

The price is in `mwl-ir`'s module doc: every level below the root reaches its write at a
refcount ≥ 2, so a nested write always separates the inner row — **O(inner) per write**,
correct rather than merely acceptable, and removable later by a write-through descent
(one runtime entry point taking the whole key chain), which is an M9 ABI question.

Conformance 567, differential 162, `verify.py` 6 of 6 green, and `wsl.exe -- bash
/mnt/<drive>/<repo>/tools/leak-check.sh` clean over the case body — the retained intermediate row
is a new refcount edge, so that leg was owed and was run.

## Next group

**All three share `crates/mwl-ir/src/lower/mod.rs`'s `write_back_array` and the
`tests/conformance/array/` tree**; the first two also touch `crates/mwl-types/src/expr/`
and `crates/mwl-diagnostics/src/lib.rs`. Take them in order.

- [ ] **An array-element write through a hooked property is a diagnostic** — the standing
      decision in `docs/agent/loop-goal.md` (PHP raises "indirect modification of
      overloaded property", so refusing *is* the PHP-compatible answer). The assert is
      `crates/mwl-ir/src/lower/mod.rs:1821`; it has to move up to the checker, beside
      `check_assign` (`crates/mwl-types/src/expr/assign.rs:319`), which is where the
      target's `ExprInfo::HookedProperty` is already recorded
      (`crates/mwl-types/src/expr/members.rs:444`). **Next free types code is E0478.**
      Model it on `report_method_on_erased_receiver`
      (`crates/mwl-types/src/expr/calls.rs:469`), which is the same shape one item back.
      The same decision names the nullsafe target `$a?->b = v` as a compile error too —
      take both under one code only if they read as one rule, otherwise E0478 and E0479.
- [ ] **An element write whose property base erased to a shape or a plain `object`** —
      `crates/mwl-ir/src/lower/mod.rs:1829`, item 22's other remaining site. ADR 0036 § 4
      gave the property *read* a name-keyed runtime fetch and stopped there, so this is
      the write half of the same question and the cheap answer is the one item 25 already
      took: refuse it at the checker where it is written.
- [ ] **`$g[][0] = 1` — an append at an intermediate level** — `mwl-ir` gap 23, in
      `crates/mwl-ir/src/lib.rs`, whose text already carries the recipe: an append level
      has nothing to descend into, so its row is a fresh `InstKind::ArrayNew` rather than
      an `ArrayRowForWrite`, and the climb back out stores it with `InstKind::ArrayAppend`
      rather than `ArraySet` — which is why the appended row's key never has to be named.
      The flatten to widen is at `crates/mwl-ir/src/lower/stmt.rs:800`.

## Backlog

- `lower_decl_type`/`lower_checked_ty`'s catch-alls still refuse `decimal`, `never`,
  `iterable`, `self`/`static`/`parent`, a shape type and an intersection as a *declared*
  type — item 25's file, no item's prose (`docs/implementation-plan.md` § Open now).
- A write-through nested descent removes the O(inner) copy this session bought
  (`crates/mwl-ir/src/lib.rs` § Design choices; M9 freezes the runtime ABI).
- `array<T> as array<U>` still panics `mwl-ir`, which is what blocks four playbook-named
  case shapes (ADR 0007 § 2's conversion row).
- Stage 0's last two items and item 1's 12 promotion-table sites (`python
  tools/holes.py --item 1`).
- 20 of the 32 named `.mwlt` cases are still to write (`python tools/loop.py --list`).
