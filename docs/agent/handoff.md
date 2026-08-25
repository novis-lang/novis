# Handoff

## State

**Stage 0 items 1, 2, 3, 4 and 9 are done; item 5 is in progress — 5a landed, 5b is next.** ADR 0047's
three type atoms now *intern* instead of being refused: `Ty::StringLiteral(String)`,
`Ty::IntLiteral(i64)` and `Ty::EnumCase(QName, EnumBacking, String)`, each documented on its own variant
in `crates/mwl-types/src/ty.rs`. § 2's fold and § 3's narrowing are two different variants on purpose —
`Mode::Read` is never the int literal of its backing value, which is the hole ADR 0010 § 5 closed.

§ 2 needed a value for a *declared* class constant, and nothing collected one, so there is a new pass:
`crates/mwl-types/src/consts.rs` (`ConstTable`), built beside the enum table and before
`build_signatures` for the same reason it is. It holds **values, not types**, so it does not close the
crate's existing "a user class constant's type is unmodeled" gap; its own module doc says why an
ineligible value is recorded rather than dropped. A `Core` class constant folds through
`core_lib::constant` instead, with `is_registered`'s narrowing intact.

§ 5's "zero additional runtime representation" is three erasure arms in
`mwl_ir::lower::lower_checked_ty` plus two in `lower_decl_type`. `E0457` (`E_LITERAL_TYPE_UNCHECKED`) is
**retired**, not reused — item 5b's checklist line asked for that and it is already done. `E0468`
(`E_LITERAL_TYPE_NOT_CONST`) is new, for § 2's ineligible-value case; an undeclared constant or case in
type position takes `E0405` (`E_UNKNOWN_MEMBER`), the same code the identical mistake takes on the
expression side.

`python tools/verify.py` is green (1338 tests). No valgrind run: the slice adds no refcount edge — every
new type erases to a representation that already existed.

**Not yet true, and expected:** a literal-typed binding still fails to check, exactly as a `true`-typed
one always has, because § 4's assignability table is 5b. `equality_domain` puts a literal type in its
base's ADR 0090 domain, so `$mode == "z"` compares two strings rather than being refused as disjoint;
refusing it on non-overlapping literal *sets* would be a new row in ADR 0090 § 2, not a consequence of
this one.

## Next group — ADR 0047 § 4's assignability, diagnostics and the runtime half (Stage 0 item 5)

**Shared file set:** `crates/mwl-types/src/expr/assign.rs` and `src/expr/literals.rs` for 5b, then
`crates/mwl-types/tests/literal_types.rs` and `tests/conformance/lang/` for 5c. `ty.rs`'s
`TypeInterner::literal_base` ([ty.rs:568](../../crates/mwl-types/src/ty.rs#L568)) already answers § 4's
first four rows and is unused outside the tests — 5b is its first real caller.

- [ ] **5b — § 4's assignability and conversion table.** `is_assignable` in
      [expr/assign.rs](../../crates/mwl-types/src/expr/assign.rs) gains the four free-widening rows
      (literal → base, case → enum, and each union form) over `literal_base`, and refuses the reverse.
      § 4 also needs the step ADR 0047's own *Verification* M2 row names first: a **literal expression**
      must type as its literal type, in `crate::expr::literals`, widened back to the base at every other
      position — without it nothing a caller writes ever satisfies one of these types except through an
      `as`.
- [ ] **5c — § 4's checked `as`, and § 6's two diagnostics.** `base`/`mixed → literal` and
      `enum → case-subset` are checked conversions with a membership test, on the shape `as uint` already
      has. § 6 names `E_LITERAL_TYPE_MISMATCH` and `E_ENUM_CASE_SUBSET_MISMATCH`, codes assigned at
      implementation (next free: **E0469**), each generated from the type via
      `TypeInterner::describe`, which already renders all three atoms.
- [ ] **5d — the runtime half.** A `.mwlt` case under `tests/conformance/lang/` pinning that a
      literal-typed binding behaves *exactly* as its base at run time (§ 5 promises there is nothing
      else to see). Run the rows in a scratch `.agent-tmp/*.mwl` first — the checker accepting a row is
      not the same as it lowering.

## Backlog

- Item 6 — `private`/`protected` are enforced at an access site (loop-goal.md § *Stage 0*).
- Item 7 — `Comparable`/`Stringable` carry their member signatures; `instanceof Stringable` panics
  `mwl-ir` today (loop-goal.md § *Stage 0*).
- Item 8 — ADR 0061's `autoload` grammar and its name-to-file fixpoint (loop-goal.md § *Stage 0*).
- A `type` alias naming a class is not expanded before `Alias::CONST` in type position resolves —
  `mwl_types::lower::lower_member_type` resolves the left-hand name directly (that module's own doc).
- `+` and `<` over two numeric representations still fail in codegen — ADR 0007 § 4's promotion table,
  `mwl-ir`'s gap 19. `==` is out of that hole; the arithmetic operators are not.
- Stage 3 resumes at `examples/collect.mwl`: `Core\Path` first, then `Encoding`/`Hash`/`Uuid`
  (implementation-plan.md § *Blocking*).
