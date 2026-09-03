# Handoff

## State

**M8 goal 5. `crates/nvs-cli/src/worker.rs` runs on every driver `nvs-db` can send a statement
over.** `open` reads `[db.<name>]`'s `driver` first and connects as that backend, `roster` and
`claim` read their answer through the dialect, and `report`'s three write-backs pick their text and
their bind order the same way — including the retry, whose MySQL order is `run_at`, `id`,
`claimed_at` where PostgreSQL's is `id`, `claimed_at`, `run_at`
(`crates/nvs-stdlib/src/queue.rs:697` owns why). The claim and the dead-letter move are `Split`s on
MySQL, each pair inside one transaction that rolls back before a refusal propagates.

**Three types carry that**, at `crates/nvs-cli/src/worker.rs:921` onward: `Wire` is the owned
connection with one arm per driver, `Dialect` is it borrowed as the two dialects
`nvs_stdlib::queue` writes, and `Framed` flattens MySQL and MariaDB, which differ here only in the
type of the borrow. `nvs-stdlib` has the same last enum and it is `pub(crate)` there — ADR 0132 § 1's
crate graph puts the two on opposite sides, and what is shared is the statements.

**Known gap 5 is now the test legs, not the code.** Nothing outside PostgreSQL has met a real
server: `crates/nvs-stdlib/tests/queue.rs` is `PgConn` from its gate down, so every MySQL statement
the worker now sends is held only by the unit agreements in `crates/nvs-stdlib/src/queue.rs`. That
is the next group.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` is a busy-state cell. Do not rename it, split it, or write a MySQL-only test
under it.

## Next group

**One file set: `crates/nvs-stdlib/tests/queue.rs`, with `crates/nvs-db/src/matrix.rs` read for the
`NVS_DB_MATRIX_*` fields only.** The first slice is what the other two stand on, so take them in
order; each later one is a case that skips with no `NVS_DB_MATRIX_DRIVER` set, exactly as the file's
PostgreSQL cases already do.

- [ ] **The file's helpers open either driver** (0067 § 2, 0084 § 2).
      `crates/nvs-stdlib/tests/queue.rs:57` is `postgres()`, the gate that returns `None` for any
      other driver and so skips the whole file; `crates/nvs-stdlib/tests/queue.rs:69` is `open`,
      `:97` is `schema`, `:159` is `rows`, `:187` is `one` and `:197` is `apply` — all six typed on
      `nvs_db::PgConn`. The shape to follow is `crates/nvs-cli/src/worker.rs:948`'s `Dialect` and
      `crates/nvs-cli/src/worker.rs:968`'s `Framed`, which is the same flattening one crate over.
- [ ] **A matrix case that migrates, pushes and claims against a real MySQL server** (0084 §§ 2, 4).
      `crates/nvs-stdlib/tests/queue.rs:206` is `push` and `:229` is `claim`, both of which send
      `INSERT_POSTGRES`/`CLAIM_POSTGRES` by name; the MySQL twins are `Split`s
      (`crates/nvs-stdlib/src/queue.rs:594` and `:622`) and need the transaction
      `crates/nvs-cli/src/worker.rs:432`'s `framed_claim` wraps them in.
- [ ] **The write-back and the move run on MySQL too** (0084 § 6).
      `crates/nvs-stdlib/tests/queue.rs:290` is the dead-letter case and `:397` the visibility one;
      `crates/nvs-stdlib/tests/queue.rs:247`'s `landed` is what reads the two tables back.

## Backlog

- No test of `nvs-cli`'s worker at all — the dialect branch is proven only by the matrix legs above
  (`docs/agent/loop-goal.toml` stage 8).
- `examples/queue.nvs` runs against PostgreSQL only (`docs/agent/loop-goal.toml` stage 8).
- Stage 7's `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` waits on a TDS driver
  (`crates/nvs-db/src/conn.rs`'s known gaps).
- SQL Server and SQLite run no queue statement at all — `nvs_stdlib::queue`'s `no_dialect` is the
  list.
- A dead-lettered row still carries only the last attempt's error
  (`crates/nvs-cli/src/worker.rs`'s module doc, *Known gap*).
