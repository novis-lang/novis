# Handoff

## State

**§ 4's write side is on disk**: `Core\Db\Connection::execute` answers a `Core\Db\Write` whose
`affected`, `changed` and `lastId` are three readers — `crates/nvs-stdlib/src/db.rs:286` for the
row and `:595` for the class. They are **readers and not § 18's readonly properties**, for the
reason `CoreTy::Instance` states and with `Core\RateLimit\Decision` as the precedent; the class
doc at `db.rs:595` is the home of that, and the spec's table is left as written.

**The statement path is one function for both members.** Everything up to the send — § 18's
`$params` rule, § 5's rewrite, the encoding — is `statement_of` (`crates/nvs-stdlib/src/db.rs:1600`)
and the connection lookup is `postgres_of` beside it, so a write cannot acquire a binding rule a
read does not have. `query` and `execute` are now only two ways of reading a stream that has
already started.

**`execute` drains and decodes nothing.** Ending the stream is what returns the connection to
idle and `lastId` is taken as each row goes past, so the drain is also what finds it — which is
why an `insert … returning` of a `UUID` column answers here while the same column still refuses
in `query` (`structured_column`). `affected` folds a tag carrying no count at all to `0` and
`changed` keeps that absence; on PostgreSQL that is the only difference between them.

**The acceptance check moved two members further**: `examples/transaction.nvs` no longer stops at
`->execute` but at `->transaction` and `->executeMany`, both `E0405`. `Core\Db\Transaction`
already resolves as a class — it has no members yet.

## Next group

**Closing the acceptance check — the file set is `crates/nvs-stdlib/src/db.rs` and
`crates/nvs-db/src/pg.rs`, both of which the write side just rewrote.**

- [ ] **§ 4's `executeMany`** — `docs/spec/01-core-library.md:1159` for the row, ADR 0067 § 4.
      The member joins `query` and `execute` on `crates/nvs-stdlib/src/db.rs:286`'s instance
      roster and answers a bare `uint`; `crates/nvs-db/src/pg.rs:646`'s `execute_many` already
      runs one prepare against many bind sets, so what is owed here is one `statement_of`
      (`crates/nvs-stdlib/src/db.rs:1600`) per set with the *arities* checked to agree — § 5's
      rewrite keys the cache on the expansion, so two sets whose `inList`s differ in width are
      two statements and not one.
- [ ] **`Db\RolledBack` in spec § 10's tree**, which `transaction` throws and cannot land
      without — `crates/nvs-hir/src/errors.rs:60`'s `TREE`, then the five other places the
      playbook's *Adding a row to `nvs_hir::errors::TREE`* bullet names, starting at
      `crates/nvs-runtime/src/throwable.rs:93`'s `ThrownClass`. `Db\DbError` is owed from the
      same list (`crates/nvs-stdlib/src/db.rs`'s known gap 4) and is the same six edits.
- [ ] **§ 7's `transaction`, and the `Core\Db\Transaction` it hands the closure** —
      `docs/spec/01-core-library.md:1162` and `:1172`, ADR 0067 § 7. The three wire members are
      `crates/nvs-db/src/pg.rs:663`, `:680` and `:690`, and ADR 0043 makes `Transaction` delegate
      `Queryable` to its connection, so its rows are `crates/nvs-stdlib/src/db.rs:286`'s repeated
      against the same symbols. The playbook's *A `-p nvs-stdlib` test can hand a `Core` member a
      real `callable`* bullet is how the closure is exercised without a compiler.

## Backlog

- § 9's five structured columns — `crates/nvs-stdlib/src/db.rs`'s known gap 6 is the list.
- `Rows::columns()`, on a `Core\Db\Column` and a `ColumnType` enum — that module's known gap 5.
- `open`, blocked on a `CoreTy` for a shape *parameter* — known gap 1, a language-surface call.
- `{timeout?: Duration}` on `query`/`execute`, blocked on a seam for a statement deadline — gap 7.
- `queryAs<T>`/`stream`/`streamAs`/`close` and § 18's three `Connection` properties — gap 5.
- ADR 0067 § 13's per-core pool, which is Stages 3 to 7 — `docs/agent/loop-goal.toml`.
