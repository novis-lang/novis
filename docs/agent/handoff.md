# Handoff

## State

**Stage 8's two unwritten acceptance cases are on disk, and the driver's standing failure is closed.**
`tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst` and
`tests/conformance/error/a-limit-fatal-is-not-catchable.nvst` both pass; `loop-goal.toml:2090`'s
eight-case list is complete. Conformance 1082, differential 206.

**What the isolate case pins is `Ctx::isolate`'s own rule** (`crates/nvs-runtime/src/ctx.rs:2064`): a
child is handed the configuration in force where it was spawned, so an `[[app]]` block keyed on the
child's entry file never re-resolves and cannot widen back what the parent's block narrowed. The
`Core\Config::set` half is refused for `request.rs`'s reason — `[capabilities]` is `RuntimeTighten`
and a grant is a list rather than a quantity, so `set` returns `false`.

**Stage 0c has no implementable slice left** — item 35's remaining halves are ADR 0079 § 24's M5-to-M8
schedule, absent on time rather than missing, and `docs/reference/findings.md` § *Triage* holds each
verdict. Items 31–34 are closed.

**Stage 9 is now the top of the queue**, ADR 0119 accepted and nothing implemented. `E0126` is spoken
for and must not be handed out: ADR 0119 § 3 names it in prose for the arm that refuses `return`. The
next free `E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** New this session, in `modules`, for the group below:
**`crates/nvs-syntax/src/casing.rs`**, **`crates/nvs-hir/src/members.rs`**,
**`crates/nvs-hir/src/requires.rs`** and **`crates/nvs-ir/src/lower/control.rs`** — item 21 edits every
file that matches on `ExprKind::Match` and these four are not in the map. Standing, each proven
earlier: no field selects `docs/reference/lang/*.md` or `docs/reference/core/*.md`;
`docs/adr/divergences.md`; `docs/reference/README.md` § *Examples: the fence grammar*;
`docs/spec/01-core-library.md`'s Part II class table; and in `modules`
`crates/nvs-stdlib/src/arr.rs`. In `adrs`: **0007 §§ 2-4**, **0079 §§ 4 and 24**, **0072 §§ 6-7**,
**0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 5**, **0053 §§ 1-3**, **0027 § 1**, **0031 § 3**,
**0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**, **0090 § 3**, **0057 § 1**, **0096 §§ 1-1a**,
**0117 § 1** and — for the group below — **0119 §§ 2 and 4-6**. `orient.py` still warns that
`crates/nvs-host/src/budget.rs` matches nothing, the forward anchor its own comment describes.

## Next group

**Stage 9's front end and its checker — items 21 and 22, which the goal doc names as one file set:
`crates/nvs-syntax/` plus the four `ExprKind::Match` matchers, then `crates/nvs-types/`.** ADR 0119 is
the whole design; neither item invents anything it does not state.

- [ ] **Item 21 — the node, the parser, every walker, and `E0126`.** ADR 0119 §§ 1-3. A `CatchArm`
      beside `MatchArm` (`crates/nvs-syntax/src/ast.rs:582`) and an `ExprKind` variant beside `Match`
      (`crates/nvs-syntax/src/ast.rs:907`); `parse_catch` between `parse_assignment` and `parse_ternary`
      (`crates/nvs-syntax/src/parser/expr.rs:232`), the arm's `( Type $var? )` parsed as
      `parse_catch_clause` parses a clause's (`crates/nvs-syntax/src/parser/stmt.rs:670`) and the body at
      the ternary level so a following `catch` is the next arm; `E0126` beside `E0125`
      (`crates/nvs-diagnostics/src/lib.rs:195`). Every `ExprKind::Match` matcher gains the arm —
      `crates/nvs-syntax/src/casing.rs`, `crates/nvs-hir/src/members.rs:792`,
      `crates/nvs-hir/src/requires.rs:1169`, `crates/nvs-ir/src/lower/control.rs:2272` — and
      `crates/nvs-ir/src/lower/expr.rs:99` gets a `panic!` naming ADR 0119 § 6 until item 23 replaces it.
- [ ] **Item 22 — the union, the binding, the pre-guard state, and `E0778`.** ADR 0119 §§ 4-5. The
      result type is `make_union` over the guard and the arms as the `Match` arm does it
      (`crates/nvs-types/src/expr/mod.rs:593`); the arm's class and variable go through the clause's own
      checks and binding rule (`crates/nvs-types/src/locals.rs:1091`, its `catch`-binding doc at
      `crates/nvs-types/src/locals.rs:654`); each arm is checked from the pre-guard `live` state; `E0778`
      beside `E0777` (`crates/nvs-diagnostics/src/lib.rs:2336`), reported for an unbound `Throwable` arm
      whose body is not a `throw`. Fixtures under `crates/nvs-types/tests/`.

## Backlog

- Item 23 — the lowering, the four `.nvst` cases and the reference section; different file set
  (`crates/nvs-ir/src/lower/exception.rs:114`, `docs/reference/lang/40-statements.md:293`).
- Stage 10 item 36 — the class reference ADR and front end; `docs/agent/loop-goal.md:295`.
- Differential 206 of 210; `docs/agent/loop-goal.toml`'s stage 8 check owns the floor.
- Item 35's M5-to-M8 halves stay parked; `docs/reference/findings.md` § *Triage* owns each verdict.
