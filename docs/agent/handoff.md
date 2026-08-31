# Handoff

## State

**Stage 10 is closed.** ADR 0125's class reference is landed end to end: the type checks, all three
of § 4's dynamic sites lower, the `.nvst` corpus is green, and item 39's documentation is written —
the atom and the `as` rows in `docs/reference/lang/20-types.md`, the three sites and the `E0794` cost
in `docs/reference/lang/50-classes.md`, and the four tables that still said the dynamic spelling did
not exist (the PHP-differences row, findings P9, ADR 0007 § 7 row 14, ADR 0019's DI bullet).

**The driver's acceptance failure was the valgrind sweep, not the tree.** `examples/limits.nvs` exits
nonzero by design — its own `[[check]]` asserts `exit = "nonzero"` with `FATAL` and `memory` on
stderr — and the sweep grades on exit status, so it reported as a leak. It joins `fatal.nvs` and
`uncaught.nvs` in `[valgrind] skip`; the playbook bullet is the general rule.

**A dynamic `instanceof` narrows nothing, and that is now a known fact rather than a guess.**
`crates/nvs-types/src/locals.rs:481` narrows off `ExprInfo::InstanceOf { class }`, which the dynamic
site deliberately does not record (`Lowering::lower_instanceof`'s doc comment owns why). Nothing in
the reference claims either way — deciding it is the next group's second item.

**Known gaps, unchanged:** `as ?class<T>` still has no row and still needs ADR 0125 § 2's
representation decision; `$cls::f(...)` written as ADR 0027's first-class callable still records
`ExprInfo::CallableRef`, so the `Closure` names `T`'s method rather than the implementor's.

**`orient.py`'s `[context]` gaps, all hit this session.** No field selects `docs/reference/lang/*.md`,
`docs/reference/tools/*.md` or `docs/reference/findings.md`, and this item was three of them. `adrs`
did not name ADR 0125 §§ 1, 2, 4, 5 even though the item cites §§ 2 and 4 by number. Standing:
`docs/adr/README.md` and `ground-rules.md` are not in `modules`, the pack prints the goal item but not
the `[[check]]` grading it, and `modules` still names `crates/nvs-host/src/budget.rs`, which matches
nothing.

## Next group

**The three loose ends `class<T>` left, file set `crates/nvs-types/src/expr/operators.rs`,
`crates/nvs-types/src/locals.rs`, `crates/nvs-ir/src/lower/convert.rs` and
`docs/reference/lang/20-types.md`.** All three are checker-and-lowering work over a landed feature;
none needs a new ADR, and ADR 0125 §§ 2 and 4 already decide what each should do.

- [ ] **`as ?class<T>` answers `null` where `as class<T>` throws.** ADR 0125 § 2 states it and there
      is no row: the conversion kinds at `crates/nvs-types/src/expr/operators.rs:1863`, the lowering
      at `crates/nvs-ir/src/lower/convert.rs:880` whose *Known gaps* at
      `crates/nvs-ir/src/lower/convert.rs:858` names the shape. Find out first whether it is refused
      or panics today. The `as ?T` paragraph at `docs/reference/lang/20-types.md:530` gains the line.
- [ ] **Decide what `$x instanceof $cls` narrows.** `crates/nvs-types/src/locals.rs:481` reads
      `ExprInfo::InstanceOf { class }` and the dynamic site records none, so it narrows nothing today.
      Either record `T` there (sound: the value is a `class<T>`) or state it in
      `crates/nvs-ir/src/lower/expr.rs:4253`'s doc comment and in the narrowing list at
      `docs/reference/lang/20-types.md:654`. ADR 0125 § 4.
- [ ] **`$cls::f(...)` as a first-class callable.** `crates/nvs-types/src/expr_table.rs:412`'s
      `ClassRefCall` is right; the callable form at `crates/nvs-types/src/expr/calls.rs:241` still
      records `ExprInfo::CallableRef` and names `T`'s method. A refusal is the cheap correct answer;
      a descriptor-carrying closure is the other. ADR 0027, ADR 0125 § 4.

## Backlog

- Stage 8's bars: conformance 1091, differential 206 of 210, migration 37% over a 36% floor —
  `docs/agent/loop-goal.md` § *Stage 8*.
- `class<T>` in `Core\Reflect`'s surface: ADR 0019 now points at it, and no member takes one.
- The `[context]` manifest gaps above, in `docs/agent/loop-goal.toml`.
