# Handoff

## State

**Goal `type-test` — stages 0 and 2 are on disk.** `is` parses `expr is Type` into
`ExprKind::TypeTest`, sharing `instanceof`'s left-associative precedence level, and the reserved-word
refusal is gone from that position. `$x is $cls` is `E0812` in the parser. Goal `signed-urls`'s whole
list is still this goal's Stage 1 floor and passes.

**Nothing checks or lowers the node yet, and no build error says so.** `ExprKind` is
`#[non_exhaustive]` (`crates/nvs-syntax/src/ast.rs:770`), so `$x is int` falls into the checker's
`_ => env.interner.mixed()` at `crates/nvs-types/src/expr/mod.rs:896` — the subject is not even
visited — and lowering has never seen one. The playbook bullet under *Writing Novis itself* is the
general trap.

The design is finished and is not this goal's to re-open: [ADR 0150](../decisions/0150.md),
`rule:types/type-test`, `rule:php-migration/is-takes-pattern-matchings-type-patterns`. The two calls a
session must not re-decide — `is` is total, and the right-hand side is a `Type` — are `loop-goal.md`
§ *Standing decisions*, which every pack prints.

## Next group

**Stage 3: the checker** — one file set: `crates/nvs-types/src/expr/mod.rs`,
`crates/nvs-types/src/expr/members.rs`, `crates/nvs-diagnostics/src/lib.rs`.

- [ ] **`TypeTest` answers `bool` for every subject, and checks that subject** — an arm beside the
      conversion one at `crates/nvs-types/src/expr/mod.rs:397`, because today the node reaches the
      catch-all at `crates/nvs-types/src/expr/mod.rs:896` instead. `rule:types/type-test`.
- [ ] **A settled answer folds to a constant rather than a diagnostic** — same arm,
      `crates/nvs-types/src/expr/mod.rs:397`. `int $n; $n is int` is `true` and `$n is string` is
      `false`, and neither is refused. `rule:types/type-test`.
- [ ] **Do not copy `instanceof`'s refusal** — `infer_instanceof` at
      `crates/nvs-types/src/expr/members.rs:280` refuses a subject that `!can_hold_an_object`, and ADR
      0150 § 6 is why that reasoning does not transfer to an operator every value has an answer for.
- [ ] **The two remaining refusals, and one wrong code in the rule** — `E0811` for `is void`/`is
      never`, and the tainted/secret one, which **cannot be `E0810`**: that code has been
      `E_DECODED_FIELD_NOT_TAINTED` at `crates/nvs-diagnostics/src/lib.rs:3296` since before ADR 0150
      wrote its table. Take the next free code in the band and fix `rule:types/type-test`'s cell
      through a record whose `changes:` block names it. `E0812` is already declared at
      `crates/nvs-diagnostics/src/lib.rs:3313`.

## Backlog

- Stage 4 — narrowing on the true edge, the fifth spelling at `crates/nvs-types/src/locals.rs:515`.
- Stage 5 — lowering beside the conversion and `instanceof` arms, reusing their walks and never a
  second one; `crates/nvs-ir/src/lower/expr.rs:391`.
- Stage 6 — `examples/type-test.nvs`, which `loop-goal.toml`'s `files` list already names and which
  does not exist yet, plus the conformance cases.
- `[context] modules` named none of the AST walkers a new `ExprKind` needs an arm in —
  `crates/nvs-syntax/src/walk.rs`, `crates/nvs-syntax/src/casing.rs`, `crates/nvs-hir/src/members.rs`,
  `crates/nvs-hir/src/requires.rs`, `crates/nvs-types/src/ctor_init.rs`,
  `crates/nvs-types/src/lateinit.rs`, `crates/nvs-lsp/src/semantic.rs`,
  `crates/nvs-ir/src/lower/control.rs`. The driver sweeps `modules` from the paths these commits
  touched, so it closes itself.
