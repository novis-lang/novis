# Handoff

## State

**ADR 0119's front end is on disk — stage 9 item 21 is done, item 22 is next.** `ExprKind::Catch`
and `CatchArm` (`crates/nvs-syntax/src/ast.rs:591`, `:920`), `parse_catch` between assignment and
the ternary (`crates/nvs-syntax/src/parser/expr.rs:233`), `E0126` for a `return`/`break`/`continue`
arm body, and every walker's arm. Conformance 1084, differential 206.

**The `( Type $var? )` half is now shared** — `Parser::parse_caught_type`
(`crates/nvs-syntax/src/parser/ty.rs:24`) carries the union refusal for both forms, taking the help
text from its caller, so the block form's E0245 output is byte-identical and an arm's says *write
two arms*.

**§ 2's precedence needed one thing the ADR states only by implication**, and it is recorded in
`parse_ternary_else`'s doc comment rather than in the ADR: a ternary's *else* branch parses at the
assignment level with `catch` cut out, or it swallows the arm that belongs to the enclosing guard.
The playbook bullet is the trap; the four tests in
`crates/nvs-syntax/src/parser/tests/expr.rs:947` are the pin.

**`crates/nvs-ir/src/lower/expr.rs:102` is a `panic!` naming § 6**, so nothing may reach the
lowering yet. That is why both new `.nvst` cases are `--EXPECTF-ERROR--` and why the passing-case
side of § 2's table waits on item 23. Two `nvs-types` walkers past the item's list also gained the
arm — `ctor_init.rs:457` and `lateinit.rs:370` — because their `_ =>` fallback would have skipped
the guarded expression silently.

**The driver's standing failure is closed and was not a regression.** `check-migration at 33%` was
failing on the user's uncommitted `clamp` row; `tools/check-migration.py`'s new
`AHEAD_OF_THE_BUILD` names it. Coverage is unchanged at 34% — the list is not in the denominator.
The user's nine modified docs and ADR 0124 are still uncommitted and were not staged.

**`orient.py`'s `[context]` gaps.** New this session, in `modules`:
**`crates/nvs-types/src/expr/mod.rs`**, **`crates/nvs-types/src/ctor_init.rs`** and
**`crates/nvs-types/src/lateinit.rs`** for item 22, and **`crates/nvs-syntax/src/parser/ty.rs`**.
Standing, each proven earlier: no field selects `docs/reference/lang/*.md` or
`docs/reference/core/*.md`; `docs/adr/divergences.md`; `docs/reference/README.md` § *Examples: the
fence grammar*; `docs/spec/01-core-library.md`'s Part II class table; and in `modules`
`crates/nvs-stdlib/src/arr.rs`. In `adrs`: **0007 §§ 2-4**, **0079 §§ 4 and 24**, **0072 §§ 6-7**,
**0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 5**, **0053 §§ 1-3**, **0027 § 1**, **0031 § 3**,
**0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**, **0090 § 3**, **0057 § 1**, **0096 §§ 1-1a**,
**0117 § 1** and **0119 §§ 4-6**. `orient.py` still warns that `crates/nvs-host/src/budget.rs`
matches nothing, the forward anchor its own comment describes.

## Next group

**Stage 9's checker and its lowering — items 22 and 23, over `crates/nvs-types/` and then
`crates/nvs-ir/src/lower/`.** ADR 0119 §§ 4-6 is the whole design.

- [ ] **Item 22 — the union, the binding, the pre-guard state, and `E0778`.** ADR 0119 §§ 4-5. The
      result type is the union of the guarded expression and every arm, with a `throw` arm typed
      `never` and contributing nothing; the arm's `$e` is a local of the enclosing function under
      the block form's rule (`crates/nvs-types/src/locals.rs`); a `Throwable` arm warns. Today
      `crates/nvs-types/src/expr/mod.rs`'s dispatch falls through to `mixed` — the two new `.nvst`
      cases say `mixed $x =` for exactly that reason and want tightening when this lands.
      `crates/nvs-diagnostics/src/lib.rs:441` is the E02xx neighbourhood; next free `E07xx` is
      `E0794`.
- [ ] **Item 23 — the lowering.** ADR 0119 § 6. Replace the `panic!` at
      `crates/nvs-ir/src/lower/expr.rs:102` with a landing pad per arm over the one guarded
      expression, joined by a phi, reusing `crates/nvs-ir/src/lower/exception.rs`'s block-form
      machinery. Then § 2's table becomes four passing `.nvst` cases rather than parser unit tests.

## Backlog

- Stage 0c item 35's remainder is ADR 0079 § 24's M5-to-M8 schedule, absent on time — `docs/reference/findings.md` § *Triage*.
- `E0126` is spoken for by ADR 0119 § 3 and was not handed out elsewhere; next free `E02xx` is `E0247`.
- ADR 0119 § 2's passing-side table needs `.nvst` cases once item 23 lands — `docs/plan/m6.md`.
- `crates/nvs-host/src/budget.rs` is a `[context] modules` pattern matching nothing — `docs/agent/loop-goal.toml`.
