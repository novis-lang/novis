# Handoff

## State

**A `catch` binding's members are settled and refused where they are written.** Spec § 10
gives the exception tree properties and no accessors, so `$e->getMessage()` and every
other PHP spelling is `E0405` with a help naming the property that answers the same
question — `nvs_types::expr::calls::report_exception_accessor` is that mapping's home, and
the plan's `Open now` carries the rule. It used to panic `nvs-ir`.

- **`tests/conformance/error/a-catch-binding-has-properties-rather-than-phps-accessors.nvst`
  is new** and pins six spellings in one compile: the four accessors that map to a
  property, `getCode` (which maps to none and names the roster), and the same refusal
  through a `LogicError` binding rather than the root.
- **`python tools/holes.py` reads 16 refusal sites and 9 named cases still to write.**
  Every standing site belongs to a roster the plan already documents as an
  internal-consistency check; the *cases* are the frontier now, not the sites.
- **`orient.py`'s pack was complete for this item.** The two standing manifest gaps are
  unchanged — `[context] modules` has no `nvs-runtime` and no `nvs-diagnostics` entry.

## Next group

**The two conversion-and-tagged corpus cases `holes.py --cases` still names, over the
conversion lowering plus its checker half.** The files:
`crates/nvs-ir/src/lower/convert.rs` and `crates/nvs-types/src/expr/`. Both are cases over
landed work, so they are cheap and belong in one session.

- [ ] **`tests/conformance/lang/every-remaining-conversion-row-runs-or-throws.nvst`** —
      ADR 0007 § 2's grid, every row that has not been pinned yet, succeeding and
      throwing. `crates/nvs-ir/src/lower/convert.rs:60` (`convert`) is the row table and
      `crates/nvs-ir/src/lower/convert.rs:949` (`lower_array_restamp`) the element walk;
      `tests/conformance/lang/conversions-that-succeed.nvst` and
      `the-conversion-table-is-closed.nvst` are what is already pinned — read those first
      so the new case adds boundaries rather than another row of the same shape.
- [ ] **`tests/conformance/lang/a-mixed-value-answers-arithmetic-truth-and-a-subscript.nvst`**
      — one `mixed` value answering all three families, which is an *agreement* case over
      work already landed separately: the `ValueAdd` family, ADR 0035's truthy table and
      `Helper::ValueIndexGet`. The three existing halves are
      `arithmetic-over-a-mixed-operand-is-decided-by-its-tag.nvst`,
      `a-subscript-through-a-mixed-base-is-decided-by-its-tag.nvst` and
      `an-operator-agrees-whether-its-operand-is-typed-or-tagged.nvst`.
- [ ] **`python tools/holes.py --cases` again** once both land, to pick the next pair.

## Backlog

- A `mixed`/union/scalar receiver of an instance method call is still unrefused and
  panics `nvs-ir` at `crates/nvs-ir/src/lower/expr.rs:2344` — the site's own message says
  so; ADR 0036 § 4 is what decides whether it defers or refuses.
- Seven more named cases under `[8 corpus and guards]`, listed by
  `python tools/holes.py --cases`.
- `[context] modules` in `docs/agent/loop-goal.toml` has no `nvs-runtime` and no
  `nvs-diagnostics` entry.
