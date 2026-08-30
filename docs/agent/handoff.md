# Handoff

## State

**Stage 9 — the expression-level `catch` — is the open stage, and it runs ahead of the rest of
stage 8.** The user decided it on 2026-08-30, in an interactive session that wrote
[ADR 0119](../adr/0119-an-expression-level-catch-is-a-typed-arm-on-one-guarded-expression.md) — the
whole design, accepted — and `docs/agent/loop-goal.md` § *Stage 9*, three items with their anchors.
Nothing of it is implemented yet: the ADR, the goal stage, the three `[[check]]`s at the end of
`docs/agent/loop-goal.toml` and the `[context]` additions are the entire footprint so far. The reason it
goes first is in the stage's own paragraph: every `.nvst` case and reference example written after it
can use the form.

**Stage 8 stands where the previous handoff left it**: six of its eight named conformance cases are
written, the counts were 1017 conformance / 206 differential against floors of 1050 / 210, and the two
cases it still owes are listed after the group below. `crates/nvs-cli/src/meta.rs`, which the previous
handoff reported as another agent's uncommitted work failing the gate at `fmt`, has since been committed
(`fa85d887`), so `python tools/verify.py` is expected green — **run it whole once at the end of the
first group; nothing has confirmed it since.**

**A `#[Test]` cannot read the configuration.** `--RUN-- test` runs each test in its own isolate, but the
runner resolves no tree (`crates/nvs-cli/src/runner.rs:245`), so `Core\Config` answers empty there. The
playbook's *Writing a test case* bullet owns the spelling; whether the runner *should* resolve
`./nvs.toml` is in `## Backlog`.

## Next group

**Stage 9's front end and checker — items 21 and 22.** One file set: the AST and parser, every walker
that matches on `ExprKind::Match`, the checker's `match` arm and the clause binding, and the diagnostic
registry. Read ADR 0119 §§ 1–5 before the code; the goal file's items carry every `file.rs:NN`.

- [ ] **Item 21 — the node, the parser, every walker, `E0126`.** ADR 0119 §§ 1–3. `CatchArm` beside
      `MatchArm` (`crates/nvs-syntax/src/ast.rs:582`) and the `ExprKind` variant beside `Match` (`:907`);
      `parse_catch` between `parse_assignment` and `parse_ternary` (`crates/nvs-syntax/src/parser/expr.rs:232`),
      the arm's `( Type $var? )` parsed the way `parse_catch_clause` does a clause's
      (`crates/nvs-syntax/src/parser/stmt.rs:670`), the body at the ternary level so a following `catch`
      is the next arm of the same guard; `E0126` beside `E0125` (`crates/nvs-diagnostics/src/lib.rs:195`)
      for `return`/`break`/`continue` at the head of an arm. Every `ExprKind::Match` match gains the
      arm — `crates/nvs-syntax/src/casing.rs`, `crates/nvs-hir/src/members.rs:792`,
      `crates/nvs-hir/src/requires.rs:1169`, `crates/nvs-ir/src/lower/control.rs:2272` — and
      `crates/nvs-ir/src/lower/expr.rs:99` gets a `panic!` naming ADR 0119 § 6 until item 23. Tests
      named by the acceptance check, in `crates/nvs-syntax/src/parser/tests/expr.rs`.
- [ ] **Item 22 — the union, the binding, the pre-guard state, `E0778`.** ADR 0119 §§ 4–5. `make_union`
      over the guard and the arms as `ExprKind::Match` does it (`crates/nvs-types/src/expr/mod.rs:593`);
      the class and variable through the clause's own checks and binding (`crates/nvs-types/src/locals.rs:1091`,
      the binding rule's doc at `:654`); each arm checked from the pre-guard `live` state; `E0778`
      beside `E0777` (`crates/nvs-diagnostics/src/lib.rs:2336`), a warning for an unbound `Throwable`
      arm whose body is not a `throw`. Fixtures under `crates/nvs-types/tests/`, named by the acceptance
      check.

**Then, in this order:** item 23 (the lowering, the four `.nvst` cases and the reference section —
`docs/agent/loop-goal.md` § *Stage 9*, a different file set), and after it the two stage-8 cases the
previous handoff carried:

- `tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst` — M6's
  *Verify* and ADR 0118 § 1. `spawn script`'s `with(grants: …)` is `E0777` at its own site, so the
  narrowing a child inherits comes from the configuration (`crates/nvs-config/src/capability.rs:189`),
  not the spawn expression; a `Core\Config::set` in a parent is visible to a child it spawns.
- `tests/conformance/error/a-limit-fatal-is-not-catchable.nvst` — ADR 0020: a `[limits]` breach is a
  `FATAL` no ordinary `catch` sees. `crates/nvs-runtime/src/budget.rs:6`, `crates/nvs-runtime/src/ctx.rs:1128`,
  `crates/nvs-config/src/tree.rs:194`.

## Backlog

- Decide whether `nvs test` resolves the configuration tree the way `nvs run` does — `docs/plan/m6.md`.
- 33 more conformance cases and 4 more differential ones to reach stage 8's floors — `docs/agent/loop-goal.toml`,
  the stage 8 `conformance` and `differential` checks.
- `orient.py` warns that `[context] modules`' `crates/nvs-host/src/budget.rs` matches no module; it is
  `crates/nvs-runtime/src/budget.rs` now — `docs/agent/loop-goal.toml`.
- `[context] modules` is missing `crates/nvs-test/src/*.rs`: the `--RUN--` subcommand roster
  (`crates/nvs-test/src/case.rs:60`) is what a stage 8 case has to pick from — `docs/agent/loop-goal.toml`.
- ADR 0042's cache payload, decided in `crates/nvs-cli/src/cache.rs`'s *Known gaps* — that module doc.
- A lint naming `as ?int` for `$s as int catch (…) => null` — ADR 0119 *Revisiting*.
