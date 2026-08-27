# Handoff

## State

**M4 — language completeness.** `unset()` now has exactly one shape that lowers and no shape that
panics, and `mwl_types::expr::members::check_unset_target`
(`crates/mwl-types/src/expr/members.rs:676`) is that rule's only home.

**The rule is "an array element of a named holder", and ADR 0028 § 3 states it.** A declared
property, static or instance, is `E0413` — the static half is new, and it names the class that
*declares* the slot, out of the `ExprInfo::StaticProperty` entry the check itself recorded. Every
other operand is the new `E0234`, whose two halves are the same rule from either side: a bare local
has no "undefined again" state to return to (ADR 0007 § 1), and a subscript of a temporary has no
slot for ADR 0007 § 5's separated array to land in. A subscript chain is additionally run through
`check_write_target`, so `unset($obj->hooked[0])` and `unset($shape->rows[0])` take the same
`E0478`/`E0480` the assignments already take.

**Two lowering gaps closed with it, both in the holder half.** A static property is now the *third*
root `write_back_array` (`crates/mwl-ir/src/lower/mod.rs:1960`) can re-point — it is already durable
storage by `is_aliasing_read`, so the "no retain and no release" paragraph holds unchanged — which
also makes `Holder::$rows["a"] = "y";` lower. And `lower_unset`
(`crates/mwl-ir/src/lower/stmt.rs:1232`) flattens a nested target the way `lower_store` does, with
one deliberate difference: a level is read with `AbsentKey::Throws` rather than vivified through
`Helper::ArrayRowForWrite`, because a removal that first creates the row it removes from leaves an
entry neither language puts there. `unset($g["nope"]["0"])` therefore throws (ADR 0007 § 7 row 11)
and leaves the count unmoved. `tools/leak-check.sh` clean over both, exit 0.

**Measured, so it is not re-derived**: what still reaches `write_back_array`'s catch-all
(`crates/mwl-ir/src/lower/mod.rs:2028`) is exactly *a root that is not a place* —
`Holder::rows()["a"] = "y";` — and nothing else. `check_write_target` has no fourth entry for it,
and adding one needs a code the types band cannot supply: `E04xx` is full at `E0499`, `E0500` is the
last number the max+1 rule yields, and `E0501` is already `mwl-ir`'s. **The band needs a decision
before a second new types diagnostic**, and the slice below is where it falls due.

## Next group

**The write-target half, all in files this session had open.** The file set:
`crates/mwl-ir/src/lower/stmt.rs`, `crates/mwl-ir/src/lower/mod.rs`,
`crates/mwl-types/src/expr/assign.rs`, `tests/conformance/`.

- [ ] **An element write whose root is not a place** — `Holder::rows()["a"] = "y";` panics at
      `crates/mwl-ir/src/lower/mod.rs:2028`. It is `check_write_target`'s missing fourth entry
      (`crates/mwl-types/src/expr/assign.rs:432`), PHP refusing the same spelling as "temporary
      expression in write context". Take the `E04xx` band decision in the same slice — a paragraph
      in `docs/adr/README.md` § *Decisions taken at project start*, per the goal's standing
      decisions — since this is the diagnostic that spends `E0500`.
- [ ] **Prove `lower_store`'s catch-all dead, or find what reaches it** —
      `crates/mwl-ir/src/lower/stmt.rs:1171`. The three arms above it are a local, a
      compile-time-known property and an element; a static property already lowers, so what is left
      is whatever the slice above refuses, plus anything the reassignment path reaches that `unset`
      does not.
- [ ] **Goal item 7 — `$x++` / `--$x` in both positions** (`mwl-ir` gap 16) —
      `crates/mwl-ir/src/lower/stmt.rs:435` (`lower_incdec_stmt`), `:454` (`lower_incdec`),
      `:494` (`lower_read_modify_write`).

## Backlog

- The `E04xx` band is full at `E0499`; `E0500` is the last number and the band needs a successor —
  `crates/mwl-diagnostics/src/lib.rs`'s own table is the legend to extend.
- `holes.py` lists 2 unattributed refusal sites in `crates/mwl-codegen/src/ty.rs:116,121` — no item
  anchors that file (`docs/agent/loop-goal.md`).
- 14 of 32 named `.mwlt` cases still to write (`python tools/loop.py --list`).
