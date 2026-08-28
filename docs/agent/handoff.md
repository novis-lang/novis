# Handoff

## State

**M4, Stage 00, and the first two of its six shapes are closed.** ADR 0109's `for` header is done end
to end: the grammar, `E0124`/`E0125`, its conformance case, and the corpus — 55 loops across 29 files
under `tests/conformance/`, `tests/differential/` and `examples/` now declare their counter in the
header. What stayed declared above is § *Consequences*' own exclusion (a counter read after its loop,
or shared by two loops); `tests/differential/lang/a-for-loop-matches-php.nvst` is entirely that shape,
by design, and `tests/conformance/lang/a-for-header-declares-its-own-counter.nvst` deliberately holds
both forms. Item 50 is closed too: PHP's `case Hearts = 1;` is `E0239` on the keyword, once, keeping
the case and naming the comma-list spelling — ADR 0010 § 1 now states that rule and names the code.

The gate's `nvs-syntax (the for header)` block passes all five of its names, so the acceptance test
moves on to `nvs-types (constants, `static` and `instanceof`)`, which is items 45-47 and is the next
group below. `docs/agent/guard-name-debt.md` § *Stage 00's remaining names* is reconciled: six names
left, not ten.

Two blind spots from earlier handoffs are unchanged and still worth knowing: `refusals.rs` counts
**arms**, not shapes, and `holes.py` recognizes a refusal by **phrasing**, so both undercount. Items
45 and 49 are where that is fixed.

## Next group

**Items 45-47, the gate's `nvs-types` check.** One file set: `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-types/src/expr/members.rs`, `crates/nvs-types/src/consts.rs` and
`crates/nvs-types/src/locals.rs`, with the tests in `crates/nvs-types/tests/`. All three are checker
work over a table that already exists, which is why they belong together.

- [ ] **Item 45 — a user-declared class constant reads at its declared type.** `nvs_types` records a
      value only for an enum case or a `Core` constant, so `Class::CONST` on a user class has no
      declared type to answer with. `crates/nvs-types/src/expr/members.rs:59` is the
      `ExprKind::ClassConstAccess` arm and `crates/nvs-types/src/consts.rs:189` is
      `build_const_table`; the lowering half is `crates/nvs-ir/src/lower/expr.rs:262`. Gate names:
      `a_user_declared_class_constant_reads_at_its_declared_type` and
      `..._lowers_to_its_value`.
- [ ] **Item 46 — `new static()` and a `: static` return resolve to the called class.**
      `crates/nvs-types/src/expr/calls.rs:171` is `infer_static_call`, which is where a call site has
      to give `static` the *receiver's* class rather than the declaring one. ADR 0008's late static
      binding is the rule; `docs/plan/m4.md`'s acceptance names the two-level case outright. Gate
      name: `a_static_return_type_resolves_to_the_called_class`.
- [ ] **Item 47 — `instanceof` narrows to an interface, not only to a class.**
      `crates/nvs-types/src/locals.rs:421` is `instanceof_residue` and `:364` its one call site; the
      module doc at `:29-67` owns what a narrowing proves. The class direction is green at
      `crates/nvs-types/tests/narrowing.rs:145` under a near-twin name — guard-name-debt.md warns
      that matching the new name against it would pass by substring, so the new test needs its own
      name: `an_instanceof_test_narrows_its_subject_to_an_interface`.

## Backlog

- Item 48 — `new` on a `Core` class with no constructor panics; `crates/nvs-types/src/expr/calls.rs`,
  `crates/nvs-ir/src/lower/expr.rs:2368`. `docs/agent/loop-goal.md` item 48.
- Item 49 — widen `crates/nvs-ir/tests/type_atoms.rs` to source *shapes*, widen `tools/holes.py`'s
  `REFUSAL` past its three-phrase match, and re-derive `refusals.rs`'s `CEILING` from the true count.
- Stage 00's three unwritten `.nvst` cases belong to items 45-47 and land with them;
  `docs/agent/guard-name-debt.md` names them.
- `docs/adr/README.md` § *Where to look* has no row for the `E02xx` rejected-construct band; a reader
  looking for "which ADR owns this refusal" greps the codes instead.
