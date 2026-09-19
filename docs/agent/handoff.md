# Handoff

## State

Goal types:exception, 3 of its 13 features complete: `ArithmeticError` landed before this session,
`Core\Cli\NotInteractive` and `Core\Db\DbError` landed in it. Each carries `about.md`, three
examples with blessed output, one attack and one `.nvst` case; exceptions owe no bench.

`tools/dossier.py` now starts every subprocess with its input closed, the way `nvs-test`'s runner
already did. Without it a prompting example is interactive under a person's shell and not under the
loop driver, which is two different answers from one file.

A `Core\Db` proof runs on `[db.schema]` — in-memory SQLite, no container — and needs an `[[app]]`
grant per entry path in `nvs.toml`; four are written there now, three examples and the attack.

## Next group

**One file set: the exception tree and the three proof trees keyed off it** —
`crates/nvs-hir/src/errors.rs`, `docs/examples/types/`, `tests/hostile/types/`,
`tests/conformance/core/`. One slice is one feature with all four proofs, in this order:

- [ ] **`Core\Db\RolledBack`** — owes about, examples, hostile, tests. Cheapest next: the `nvs.toml`
      grants, the SQLite block and the sibling assertions are already written for
      `Core\Db\DbError`, and the two are pinned against each other.
      `crates/nvs-hir/src/errors.rs:108`
- [ ] **`Core\Test\Failure`** — owes about, examples, hostile, tests. It hangs off the root rather
      than off `RuntimeError`; `rule:testing/failure-ledger` is what it is for.
      `crates/nvs-hir/src/errors.rs:105`
- [ ] **`LogicError`** — owes about, examples, hostile, tests. The half of the tree the other two
      are defined against: it is what a mistake in the *call* raises.
      `crates/nvs-hir/src/errors.rs:98`

## Backlog

- Items 5, 7 and 9-13 of this goal: `Core\Script\Finished`, `IOError`, `ParseError`,
  `RecursionError`, `RuntimeError`, `Throwable`, `TimeoutError` — `docs/agent/loop-goal.md`.
- The repository's `[queue] connection = "main"` worker starts under any program that lives long
  enough, and one run of the `Core\Db\DbError` attack waited on a PostgreSQL nobody had started
  before finishing; every run since took four seconds. Worth sizing a `timeout-ms` against —
  `nvs.toml:534`.
