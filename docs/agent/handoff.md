# Handoff

## State

**M4 — language completeness**, and **item 11 is closed**. A ternary's, an elvis's and a
`match`'s branches may now lower to two different representations: they join at
`mwl_ir::Ty::Tagged`, which is exactly what `erase_checked_ty` gives the union the
checker already typed the whole expression as, so the phi carries the erasure rather
than a promotion of one branch into the other. `Lowering::join_representations`
(`crates/mwl-ir/src/lower/expr.rs:1630`) owns the rule and both callers share it; each
branch is tagged in its **own** block, ahead of its jump, which is why neither is sealed
until both have a type. An **arm-less** `match` is `E0476` at the checker
(`crates/mwl-types/src/expr/mod.rs:346`), so `mwl-ir`'s own assert is an engine
invariant now, not a refusal.

Both decisions are recorded in `docs/adr/README.md` § *Decisions taken at project
start* — the load-bearing half is that ADR 0007 § 4's promotion rows belong to an
*operator* and § 2's implicit widening to a `float` **position**, so `$c ? 1 : 2.5`
keeps PHP's `int` on its truthy path and `float $x = $c ? 1 : 2.5;` widens once, at the
binding, through `Lowering::coerce`'s `(Ty::Tagged, Ty::Float)` row. Same line integer
`/` already draws.

`python tools/holes.py` is the worklist and no session re-derives it — **10 items, 39
sites**. Conformance 564, differential 162, `verify.py` 6 of 6 green, and the WSL
`leak-check.sh` leg is clean over a fixture that joins an aliasing `string` arm against
an `int` one (the join's one new refcount edge: `Tag` transfers, and a tagged
non-refcounted payload is a no-op for the runtime's tag-dispatched release).

## Next group

**Both items live in `crates/mwl-ir/src/lower/mod.rs`**, which is the file set they
share; take item 25 first, it is the cheaper one and its decision is already made.

- [ ] **`object` as a declared type has a representation arm** —
      `crates/mwl-ir/src/lower/mod.rs:2232` (*"only lowers bool/int/uint/float/void/
      string/bytes/array/`?T`/a union/a plain class name as a declared type"*) and
      `mod.rs:2315`, the same list for a resolved call's parameter/return type.
      loop-goal.md § *Standing decisions* settles the representation outright —
      `object` erases to the same pointer a named class does — so the work is the two
      arms plus a check that nothing below reads a class label off one. ADR 0007 § 3 is
      the opaque-top-of-every-class-type paragraph; `erase_checked_ty` at `mod.rs:2372`
      already maps `CheckedTy::Object` to `Ty::Object`, which is the shape to copy.
- [ ] **A nested `$grid[0][1] = v` writes back** — `crates/mwl-ir/src/lower/mod.rs:1713`,
      with `mod.rs:1819` and `mod.rs:1827` the two refusals in the same function. The
      hooked-property half is **already decided**: an array-element write through an
      ADR 0014 § 1 hooked property is a compile error (PHP raises "indirect modification
      of overloaded property"), so it takes a new `E04xx` — next free is **E0477** —
      rather than a write-back rule. Cases that may belong:
      `tests/conformance/array/a-nested-element-write-separates-only-the-inner-array.mwlt`.

## Backlog

- Item 17 — a `...spread` array-literal element lowers, and `&value` is refused by a
  diagnostic naming loop-goal.md § *Standing decisions*; one site,
  `crates/mwl-ir/src/lower/expr.rs:3599`.
- Item 16 — a named or spread call argument, `crates/mwl-ir/src/lower/call.rs:71` and
  `:77`; the checker half lands first, per § *Standing decisions*.
- Item 20 — `foreach (… as &$v)`, `crates/mwl-ir/src/lower/control.rs:712`.
- Item 19 — a closure or generator written where a `&$x` parameter is in scope,
  `closure.rs:160` and `generator.rs:155`.
- `void` in a value position is unlowered independently of any join: `mixed $m = f();`
  over a `void` method already fails with *"an operand used before it is defined"*, and
  no item claims it. `mwl_types` accepting a `void` expression as a value is the hole.
- Two unattributed sites, `crates/mwl-codegen/src/ty.rs:116` and `:121`
  (`python tools/holes.py`).
