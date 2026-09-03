# Handoff

## State

**M8 goal 5. ADR 0084 §§ 1, 4 and 6 now have a second statement set** — `INSERT_MYSQL`,
`CLAIM_MYSQL` and `DEAD_LETTER_MYSQL` in `crates/nvs-stdlib/src/queue.rs`, each a `Split`, which is
the pair a caller runs inside one transaction because the construct that answered in one is what
MySQL lacks. **The members still reach PostgreSQL only**, so that module's known gap 5 is now the
*seam* and no longer the SQL.

**The question the last handoff left open is settled: these texts do not go through
`nvs_db::sql::rewrite`.** The queue binds pre-encoded bytes straight to `PgConn::query`, and
PostgreSQL needs its `$n::type` casts to type a text-format parameter at all — so one shared text
is not available in either direction, and the second dialect is a second list exactly as
`MIGRATION_MYSQL` is.

**`INSERT`, `CLAIM` and `DEAD_LETTER` are now `*_POSTGRES`**, following `MIGRATION_POSTGRES`;
`crates/nvs-cli/src/worker.rs` and `crates/nvs-stdlib/tests/queue.rs` moved with them. Each
constant's own doc owns what its dialect cost — the dedupe read goes through `MIGRATION_MYSQL`'s
generated column rather than `dedupe_key`, the claim's select carries `attempts + 1 as attempts` so
a worker reads the same six columns at the same ordinals, and the move copies before it deletes.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` is a busy-state cell. Do not rename it, split it, or write a MySQL-only test
under it.

## Next group

**One file set: `crates/nvs-stdlib/src/queue.rs`, then `crates/nvs-cli/src/worker.rs`.** Take the
first two together: a seam that routes to a MySQL connection while three of the four members still
hold PostgreSQL-only text would refuse for a reason no doc states.

- [ ] **A queue connection is reached by driver, not through `postgres_of`** (0084 §§ 1 and 4).
      `crates/nvs-stdlib/src/queue.rs:1551` is `postgres_of` and
      `crates/nvs-stdlib/src/queue.rs:1597` is `no_dialect`, which is the refusal that shrinks;
      `crates/nvs-stdlib/src/queue.rs:1689` is the push's send and the first caller to grow a
      second arm. `push` on MySQL is `INSERT_MYSQL` inside a transaction, answering
      `nvs_db::MySqlRows::last_id` where PostgreSQL reads a `returning` row.
- [ ] **§ 5's `status`, `cancel` and `stats` in MySQL's dialect** (0084 §§ 3 and 6).
      `crates/nvs-stdlib/src/queue.rs:735` is `STATUS`, `crates/nvs-stdlib/src/queue.rs:754` is
      `CANCEL` and `crates/nvs-stdlib/src/queue.rs:778` is `COUNTS`. These rest on no construct
      MySQL lacks: what differs is the `$n::type` casts and `COUNTS`'s `filter (where …)`, which is
      `sum(case when … then 1 else 0 end)` there. A transcription, so no `Split` — plain `&str`
      twins beside the originals.
- [ ] **The worker claims and reports over either driver** (0084 §§ 4 and 6).
      `crates/nvs-cli/src/worker.rs:207` is `turn`, `:281` is `claim`, `:463` is `report` and
      `:521` is `apply` — every one of them typed `&mut nvs_db::PgConn`, which is the whole of what
      makes the worker PostgreSQL's. `CLAIM_MYSQL`'s doc says the column ordinals do not move.
- [ ] **A matrix case that migrates, pushes and claims against a real MySQL server** (0084 § 2).
      `crates/nvs-stdlib/tests/queue.rs:210` is the shape, and `crates/nvs-db/src/matrix.rs` is
      where a live server is found. Nothing above is proven against a server until this runs.

## Backlog

- `Core\Queue`'s `limits`/`grants` wait on a shape parameter — `queue.rs`'s known gap 1, ADR 0135.
- `$args` is `mixed` and so refuses no `secret` — `queue.rs`'s known gap 2, ADR 0033.
- SQL Server and SQLite send no statement at all — `crates/nvs-stdlib/src/db.rs`'s known gap 2.
- ADR 0067 § 8's fifth field on `DbError` — `db.rs`'s own gaps.
