# Handoff

## State

**§ 4's `executeMany` is on disk**: the row is `crates/nvs-stdlib/src/db.rs:319` and the body
`:2051`. It answers a bare `uint` and not a `Core\Db\Write` — § 4 gives the batch a sum, because
`changed` and `lastId` would each have to pick one execution to be about.

**A set is a whole `$params`, not a new binding rule.** `batch_of` (`crates/nvs-stdlib/src/db.rs:1808`)
runs `statement_of` once per set, so § 18's keying rule, § 5's rewrite and § 9's encoding reach a
batch through the member that already owns them. The sets must rewrite to **one text**: that is the
arity check § 1's cache needs, made on the thing the count stands for, and it is why `Batch` holds
one `sql` and a `Vec` of bind sets rather than a `Vec<Statement>`. `postgres_of` now takes the key
and the block rather than a `&Statement`, so a batch reaches the connection without pretending to be
a statement.

**The batch is not a transaction.** Each execution carries its own `Sync`
(`crates/nvs-db/src/pg.rs:2658` argues it), so a failure part way through leaves the writes before it
standing. An empty `$sets` writes nothing and answers `0` with § 4's state check still ahead of it.

**The acceptance check has moved one member further**: `examples/transaction.nvs` no longer stops at
`->executeMany`, only at `->transaction` (`E0405`). `Core\Db\Transaction` resolves as a class and has
no members yet, and `Core\Db\RolledBack` is not in spec § 10's tree.

## Next group

**Closing the acceptance check — the file set is spec § 10's roster, which the playbook's *Adding a
row to `nvs_hir::errors::TREE`* bullet enumerates in full, then `crates/nvs-stdlib/src/db.rs`.**

- [ ] **`Db\RolledBack` in spec § 10's tree** — spec `docs/spec/01-core-library.md:1234` states it
      (`RolledBack extends RuntimeError`, readonly `reason: string`), ADR 0067 § 7. The full roster
      is the `TREE` row at `crates/nvs-hir/src/errors.rs:70`, the `ThrownClass` variant at
      `crates/nvs-runtime/src/throwable.rs:109`, its `name()` arm at
      `crates/nvs-runtime/src/throwable.rs:148`, its `ALL` entry at
      `crates/nvs-runtime/src/throwable.rs:156`, the hard-coded label list in
      `crates/nvs-ir/src/lower/tests.rs:3082`, and the spec's own tree drawing at
      `docs/spec/01-core-library.md:753`. The `reason` property is the part the roster does not
      cover — `Throwable`'s own slots are `crates/nvs-hir/src/errors.rs:77`, and whether a § 10 class
      may add one is the question to settle first.
- [ ] **§ 7's `transaction`, and the `Core\Db\Transaction` it hands the closure** — ADR 0067 § 7,
      spec `docs/spec/01-core-library.md:1162` and `:1172`. The member joins the roster at
      `crates/nvs-stdlib/src/db.rs:319`; `crates/nvs-db/src/pg.rs:663` is `begin`, `:680` `commit`,
      and `begin_command` at `crates/nvs-db/src/pg.rs:3049` owns the isolation spellings. The closure
      is called through `nvs_runtime::call_closure` — the playbook's *A `-p nvs-stdlib` test can hand
      a `Core` member a real `callable`* bullet is what that reads off a closure value, and the
      callee owes the exit sweep.
- [ ] **`Transaction` delegates `Queryable` to its connection** (ADR 0043) — the three landed members
      re-declared on `crates/nvs-stdlib/src/db.rs`'s new `TRANSACTION` class, reading the same handle
      slot, so `$tx->executeMany(...)` in `examples/transaction.nvs:53` resolves.

## Backlog

- `queryAs`, `stream`, `streamAs`, `close` and § 18's three readonly properties — `db.rs`'s known gap 5.
- `Rows::columns()` and the `Core\ColumnType` enum — `db.rs`'s known gap 5, needs a `PgColumn` OID map.
- `Db\DbError` in § 10's tree, so a server refusal carries `sqlState` — `db.rs`'s known gap 4.
- `Core\Db::open` waits on a shape *parameter* in the registry — `db.rs`'s known gaps.
- The other four drivers — ADR 0132, and `db.rs`'s known gap 2 is the refusal that names them.
