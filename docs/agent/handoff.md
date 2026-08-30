# Handoff

## State

**Stage 10 item 37 is half on disk — the checker knows `class<T>`.** `Ty::ClassRef(TypeId)`
(`crates/nvs-types/src/ty.rs:65`) is the interned type, lowered by `lower_class_ref`
(`crates/nvs-types/src/lower.rs:156`) from the atom; its argument must be a `Ty::Class` and anything
else is `E0795`, reported only when the argument's own lowering was quiet so an undeclared name stays
one diagnostic. Covariance is the arm below `array<T>`'s in `is_assignable`
(`crates/nvs-types/src/expr/assign.rs:173`). ADR 0125 § 1 now names `E0795` and settles the equality
domain; § 5's `E0794` is declared in the registry, unused, so slice 4 does not have to re-derive a
number the ADR already prints.

**§ 2's two conversion rows are in, and `as` really is the only source.** `ConvKind::ClassRef`
(`crates/nvs-types/src/expr/operators.rs:1719`) is its own kind, so `conversion_row_exists` grants
`string → class<T>` and `class<U> → class<T>` and refuses every other operand by a written-out
`(_, ClassRef) => false` rather than by falling off the end. A `Foo::class` operand is decided where it
stands by `reject_impossible_class_reference_conversion`
(`crates/nvs-types/src/expr/operators.rs:129`), which reports `E_NO_CONVERSION` for a name outside the
hierarchy; a plain string literal is deliberately *not* folded, and that function's doc says why. The
qualifier strips through `apply_qualifier_conversion_rule` untouched, as § 2 says.

**Below the checker, nothing knows the type yet.** `nvs_ir`'s `lower_decl_type` still panics on the
atom, so no `.nvst` case can name `class<T>` until item 38. The two remaining checker slices are what
make a class reference *usable*: without them `new $cls()` is still `E0496`.

**`orient.py`'s `[context]` gaps.** Standing, unchanged and proven again: the pack prints the goal item
but not the `[[check]]` grading it, so this session re-read `loop-goal.toml` for the stage 10 block. No
field selects `docs/reference/lang/*.md` (item 39 needs it); `docs/adr/README.md` and
`docs/adr/ground-rules.md` are not in `modules`. In `adrs`: **0125 §§ 1-2 and 5** — every slice of item
37 pays for them, and this session sliced 0125 twice. `orient.py` still warns that
`crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 37's last two slices, then item 38 — file set `crates/nvs-types/src/expr/` plus
`crates/nvs-hir/src/hierarchy.rs`.** The acceptance check `nvs-types (the class reference)` still wants
its third test; the first two are green.

- [ ] **The three sites, and `E0496`'s help.** `NewTarget::Expr` (`crates/nvs-types/src/expr/calls.rs:1088`)
      types its arguments against `T`'s constructor as `NewTarget::StaticTy` does
      (`crates/nvs-types/src/expr/calls.rs:1076`); the `::` class side
      (`crates/nvs-types/src/expr/calls.rs:231`) resolves the member on `T`; `instanceof`
      (`crates/nvs-types/src/expr/members.rs:250`) takes the operand.
      `reject_dynamic_class_name` (`crates/nvs-types/src/expr/members.rs:532`) keeps `E0496` for every
      other operand, its help now naming `as class<T>`. ADR 0125 § 4.
- [ ] **The constructor rule, `E0794`.** At a `new` over `class<T>`, every implementor of `T`
      (`implementors`, `crates/nvs-hir/src/hierarchy.rs:530`) whose constructor fails
      `check_class_conformance` (`crates/nvs-types/src/conformance.rs:57`) is a refusal at the `new` site
      naming that subclass. The code is already declared
      (`E_DYNAMIC_NEW_DIVERGENT_CONSTRUCTOR`, `crates/nvs-diagnostics/src/lib.rs:2586`) — do not take a
      new number. ADR 0125 § 5 has the wording. Test:
      `a_dynamic_new_is_refused_naming_the_subclass_whose_constructor_differs`, fixtures in
      `crates/nvs-types/tests/classes.rs`.
- [ ] **Item 38, the lowering.** `lower_decl_type`'s panic on the atom
      (`crates/nvs-ir/src/lower/mod.rs:2748`, beside `erase_checked_ty` at
      `crates/nvs-ir/src/lower/mod.rs:2914`) is the first thing any `.nvst` case hits; `NewDynamic` and the
      descriptor constant are ADR 0125 § 4's table. Tests:
      `a_new_through_a_class_reference_lowers_to_new_dynamic`,
      `a_folded_class_constant_lowers_to_a_descriptor_constant`.

## Backlog

- Item 39, the reference page for `class<T>` — `docs/reference/lang/`, which no `[context]` field selects.
- Stage 10's four `.nvst` cases (`docs/agent/loop-goal.toml:2193`), blocked on item 38's lowering.
- `class<T> as string` has no row and no help of its own beyond `(ClassRef, _)`'s — ADR 0125 does not
  decide it; revisit if a case wants the descriptor's name back.
- Stage 8: conformance 1087, differential 206 of 210, migration 37% over its 36% floor.
