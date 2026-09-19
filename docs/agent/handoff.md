# Handoff

## State

Goal types:exception, 5 of its 13 features complete: `ArithmeticError`, `Core\Cli\NotInteractive`
and `Core\Db\DbError` landed earlier, `Core\Db\RolledBack` and `Core\Test\Failure` in this session.
Each carries `about.md`, three examples with blessed output, one attack and one `.nvst` case;
exceptions owe no bench. `python tools/dossier.py --verify --group types:exception` names the eight
that are left.

A `Core\Db` proof runs on `[db.schema]` — in-memory SQLite, no container — and needs an `[[app]]`
grant per entry path in `nvs.toml`; seven are written there now.

A `Core\Test` proof needs neither a grant nor a runner. `Core\Test::assertTrue`, `assertEquals` and
`expectFailure` all work under plain `nvs run`, so a failed check is showable in an example and
blessable like any other output. An options argument is written inline — `{message: "…"}` — and
never as a bare second string.

The three trunk classes left are the ones the rest of the tree is defined against, so they share one
vocabulary as well as one file set.

## Next group

**Stage 2: the dossier — one file set: the exception tree and the three proof trees keyed off it** —
`crates/nvs-hir/src/errors.rs`, `docs/examples/types/`, `tests/hostile/types/`,
`tests/conformance/core/`. One slice is one feature with all four proofs, in this order:

- [ ] **`LogicError`** — owes about, examples, hostile, tests. Cheapest next: it is the half the two
      landed `Core\Db` classes are already pinned against, so its sibling assertions are written and
      its examples need no database at all. `rule:testing/four-proofs`.
      `crates/nvs-hir/src/errors.rs:98`
- [ ] **`RuntimeError`** — owes about, examples, hostile, tests. The other trunk class, and the
      parent every `Core\Db` and `Core\Cli` proof already catches by name.
      `rule:testing/four-proofs`. `crates/nvs-hir/src/errors.rs:99`
- [ ] **`Throwable`** — owes about, examples, hostile, tests. The root, and the one name a `catch`
      can use to mean anything at all; it owns `message` and the properties every entry inherits.
      `rule:testing/four-proofs`. `crates/nvs-hir/src/errors.rs:97`

## Backlog

- `Core\Db\RolledBack`'s attack ends on `RecursionError` at 2000 nested savepoints, so where SQLite's
  own savepoint limit sits is still unasserted — `crates/nvs-stdlib/src/db/mod.rs`.
- `Core\Test\Failure`'s proofs assert nothing about the ledger, because a `.nvst` case has no runner
  to read one back — `crates/nvs-stdlib/src/test.rs`.
- `IOError`, `ParseError`, `TimeoutError`, `RecursionError` and `Core\Script\Finished` are the five
  after the trunk — `docs/agent/loop-goal.toml`.
