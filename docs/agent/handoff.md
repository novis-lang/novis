# Handoff

## State

**M4 — language completeness**, and **item 22 is closed**. The three assignment targets
that have nowhere to write to are refused in one place: `check_write_target`
(`crates/mwl-types/src/expr/assign.rs:399`), called from `check_assign`'s general arm and
from `check_compound_assign` *after* the target is checked, because the hooked and erased
halves both read the `ExprInfo` that checking the access records. E0478 is an element
write through an ADR 0014 § 1 hooked property, E0479 a nullsafe assignment target, E0480
an element write through a property whose receiver erased to a shape or a plain `object`.
The rationale for each is on its `Code::new` in `crates/mwl-diagnostics/src/lib.rs`.

`write_back_array` (`crates/mwl-ir/src/lower/mod.rs:1810`) lost its hooked-property
assert and the "does not yet lower" clause of its erased-receiver panic; what is left
there is a pure internal-consistency panic, and its doc comment names the two diagnostics
that make it unreachable. No runtime code changed, so there is no new refcount edge and
the valgrind leg was not owed.

`verify.py` 6 of 6 green — conformance **572**, differential 162 — and `holes.py` is at
**9 items, 38 sites**.

## Next group

**All three share `crates/mwl-ir/src/lower/stmt.rs`'s flatten, `write_back_array`
(`crates/mwl-ir/src/lower/mod.rs:1810`) and the `tests/conformance/array/` tree.** The
second is a live bug rather than a hole, so take it first if you only take one.

- [ ] **An element write through a *narrowed nullable* receiver fails cranelift
      verification** — not a panic, so nothing on the worklist names it. `?Box $m = new
      Box(); if ($m != null) { $m->rows["0"] = "w"; }` aborts with *"invalid pointer width
      (got 128, expected 64)"* on the `FieldSet` store. The receiver stays `Ty::Tagged`
      through the narrowing and `emit_field_set` (`crates/mwl-ir/src/lower/mod.rs:1659`,
      codegen at `crates/mwl-codegen/src/emit.rs:2235`) stores into it as a pointer; the
      plain `$m->rows = [...]` half is worth reproducing in the same scratch to find out
      whether the hole is the element write or every write through a narrowed receiver.
- [ ] **`$g[][0] = 1` — an append at an intermediate level** — `mwl-ir` gap 23,
      `crates/mwl-ir/src/lib.rs:485`, whose text carries the recipe: an append level has
      nothing to descend into, so its row is a fresh `InstKind::ArrayNew` rather than an
      `ArrayRowForWrite`, and the climb back out stores it with `InstKind::ArrayAppend`
      rather than `ArraySet`, which is why the appended row's key never has to be named.
      The flatten to widen is `crates/mwl-ir/src/lower/stmt.rs:800`.
- [ ] **`mixed $m; $m["0"] = 1;`** — the "an array-index assignment target … has no
      resolved element type" panic at `crates/mwl-ir/src/lower/stmt.rs:800`, which E0480
      just took off the *property* path and which is still open for a bare `mixed` base.
      Same shape of fix, in `check_write_target` beside the other three; **next free types
      code is E0481**. Confirm the panic on a scratch first — this session met it only
      through a property.

## Backlog

- `instanceof` does not narrow a local *declared* `object`, so E0477's and E0480's shared
  "narrow the receiver first" advice is only true via `as ClassName` — E0480's help says
  only that; E0477's (`crates/mwl-types/src/expr/calls.rs:469`) still offers `instanceof`
  and should be re-checked.
- A `SlotSet` write-through would let a *shape* field's element write lower instead of
  taking E0480 (`InstKind::SlotSet` at `crates/mwl-ir/src/ir.rs:587` borrows its value, so
  it is a refcount edge and a valgrind leg). Refusing was the group's pre-authorized
  answer; this is the upgrade path if a program ever wants it.
- `orient.py`'s `[context] modules` printed no `mwl-types` expression module: this item
  lived in `src/expr/assign.rs`, `src/expr/members.rs` and `src/expr_table.rs`, all read
  by hand. Add those three to the manifest. ADR 0036 § 4 would have earned a place in
  `[context] adrs` too; the module doc comments carried it this time.
- `mwl_stdlib::json`, `mwl_stdlib::hash` and `crates/mwl-stdlib/src/cli.rs` gaps stand
  (`docs/implementation-plan.md` § *Open now*).
- ADR 0086 § 1's substitution table is M8; ADRs 0091/0093/0097/0100 § 3 are out of scope
  (`docs/agent/loop-goal.md` § *Standing decisions*).
