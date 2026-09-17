# Handoff

## State

**Goal `one-type-test`, stage 2 is landed.** [ADR 0192](../decisions/0192.md) is written and accepted,
`rule:php-migration/one-type-test` is created (`status: designed`, it flips at stage 7),
`php-migration/is-takes-pattern-matchings-type-patterns` is deleted — fragment, entry and every
`seeAlso` — and the six fragments the record modifies are rewritten to the language this goal ships.
The chapters, `ground-rules.md` and `divergences.md` are re-rendered.

**The rulebook now leads the tree, on purpose.** No code moved: `instanceof` still parses, `E0497`
still refuses a scalar subject and `E0812` still refuses `$x is $cls`. Stages 3 to 6 close that gap;
nothing is blocked, and `python tools/verify.py` was 11 of 11 green at this commit.

Two frozen files had to be touched for `rules.py --check`, in the shape the new playbook bullet
gives: `docs/decisions/0150.md` (the deleted id out of `changes.creates`, one body citation
de-prefixed) and `docs/agent/goals/33-type-test.md` (two citations de-prefixed). The website's rule
mirror is now stale for these rules and nothing gates it, as its own playbook bullet says.

## Next group

**Stage 3: the front end** — one file set: `crates/nvs-syntax/src/parser/expr.rs`,
`crates/nvs-syntax/src/parser/ty.rs`, `crates/nvs-syntax/src/ast.rs`, `crates/nvs-syntax/src/token.rs`,
`crates/nvs-types/src/expr/type_test.rs`, `crates/nvs-types/src/expr/members.rs`,
`crates/nvs-types/src/expr/mod.rs`, `crates/nvs-types/src/locals.rs`,
`crates/nvs-types/src/expr_table.rs`, `crates/nvs-diagnostics/src/lib.rs`. The goal's `.toml` names
the four guard tests this stage owes, by function name, under `stage = "3 the front end"`.

- [ ] **The grammar** — `ExprKind::InstanceOf` deleted at `crates/nvs-syntax/src/ast.rs:932`;
      `ExprKind::TypeTest` carries `against: TestOperand` (`Type` | `Value(Box<Expr>)`);
      `parse_instanceof` at `crates/nvs-syntax/src/parser/expr.rs:553` becomes `parse_type_test`, a
      `$variable` after `is` starting the value arm at the `|>` level and every other token a type;
      `reject_value_in_type_test` at `crates/nvs-syntax/src/parser/ty.rs:190` goes.
      `rule:types/type-test` § *The value arm* is the shape.
- [ ] **The refusal** — `Keyword::InstanceOf` stays a token (`crates/nvs-syntax/src/token.rs:425`) so
      the spelling can be named; met where a binary operator may stand it reports **`E0253`**, the
      number ADR 0192 § 7 fixes, added beside `E_RESERVED_FOR_FUTURE_USE` at
      `crates/nvs-diagnostics/src/lib.rs:514`, and consumes its right operand.
- [ ] **The checker** — the value arm in `infer_type_test` at
      `crates/nvs-types/src/expr/type_test.rs:73`: a `class<T>` operand records
      `ExprInfo::ClassRefTest { base }`, anything else is `E0496`; a subject that can hold no object
      records a settled `false` and no diagnostic. `infer_instanceof`
      (`crates/nvs-types/src/expr/members.rs:303`), `testable_class_name` (`:414`) and
      `instanceof_residue` (`crates/nvs-types/src/locals.rs:477`) go, with the dispatch arm at
      `crates/nvs-types/src/expr/mod.rs:417`; the `is` residue narrows the value form to `T` on the
      true edge (`rule:types/narrowing`).
- [ ] **The codes** — `E0496`'s doc comment at `crates/nvs-diagnostics/src/lib.rs:1331` rewritten to
      its three sites (`rule:types/class-reference-sites`), and its `E_INSTANCEOF_*` constant name
      with it; `E0497` (`crates/nvs-diagnostics/src/lib.rs:1340`) and `E0812` (`:3559`) retired,
      constants and doc comments deleted, numbers never reassigned.

## Backlog

- Stage 4: the primitive renamed — `InstKind::InstanceOf` → `ClassTest` and the runtime entry points
  with it (goal prose § *Stage 4*).
- Stage 5: `Core\Ast`'s roster, `Core\Debug`/`Core\Reflect` docs, the LSP arms, the formatter fixture.
- Stage 6: every `.nvst`/`.lspt`/Rust guard test respelled, and the renames patched everywhere named.
- Stage 7: the prose sweep, `status: shipped`, and the `git grep -i -w instanceof` absence gate.
- The website rule mirror is stale for the rules this stage touched; nothing gates it, and a by-hand
  re-render is the only fix (playbook, *Tooling*).
