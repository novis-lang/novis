# Handoff

## State

**M8 goal 5. All four of ADR 0084 § 1's members run on PostgreSQL, MySQL and MariaDB alike.**
`queue_connection` in `crates/nvs-stdlib/src/queue.rs` is the seam: it borrows the filed connection
as `Queued::Postgres` or as `crate::db::Framed`, which is `pub(crate)` now and carries § 7's
`begin`/`commit`/`roll_back` beside the send because a `Split` is a pair one transaction runs.
`push` runs `INSERT_MYSQL` as that pair; `status`, `cancel` and `stats` have plain second texts —
`STATUS_MYSQL`, `CANCEL_MYSQL`, `COUNTS_MYSQL` — since no construct is missing for them, and one
reader (`counted_row`) answers for all three over both dialects because all three read integers.

**Known gap 5 has narrowed to `nvs-cli`'s worker.** `QUEUES`, `SUCCEEDED` and `RETRY` are still
PostgreSQL-only text and `worker.rs` takes a `&mut PgConn` throughout, so a worker refuses a MySQL
block that the members on that same block accept. That is the next group's first two items.

**`no_dialect` is one sentence again** — `Core\Db`'s gap 2, the two drivers that send nothing — and
a sending driver reaching it is now a bug rather than a shortfall;
`the_queues_refusal_is_only_ever_about_a_driver_that_cannot_send` holds that.

**None of the MySQL text has met a real server yet.** The unit tests hold the dialects against each
other and against the enum's ordinals; what is not asserted anywhere is that a MySQL server accepts
them, which is the matrix case below.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` is a busy-state cell. Do not rename it, split it, or write a MySQL-only test
under it.

## Next group

**One file set: `crates/nvs-stdlib/src/queue.rs`, then `crates/nvs-cli/src/worker.rs`.** Take the
first two together for the reason this session's pair went together: a worker routed to a MySQL
connection while its three statements are PostgreSQL's would refuse for a reason no doc states.

- [ ] **The worker's three statements in MySQL's dialect** (0084 §§ 4 and 6).
      `crates/nvs-stdlib/src/queue.rs:463` is `QUEUES`, `crates/nvs-stdlib/src/queue.rs:483` is
      `SUCCEEDED` and `crates/nvs-stdlib/src/queue.rs:494` is `RETRY` — all three are ordinary
      transcriptions rather than `Split`s (`$n::type` casts and the placeholder, nothing more), so
      they follow `STATUS_MYSQL` beside them and join
      `the_mysql_statements_spell_nothing_only_postgresql_has`'s list.
- [ ] **The worker claims and reports over either driver** (0084 §§ 4 and 6).
      `crates/nvs-cli/src/worker.rs:207` is `turn`, `:227` is `roster`, `:281` is `claim` and
      `:464` is `report`, each taking `&mut nvs_db::PgConn`. `CLAIM_MYSQL` is a `Split`, so the
      claim needs the transaction `push_in_two` shows the shape of; `crate::db::Framed` is not
      reachable from `nvs-cli`, so this owes a decision about where that borrow lives.
- [ ] **A matrix case that migrates, pushes and claims against a real MySQL server** (0084 § 2).
      `crates/nvs-stdlib/tests/queue.rs:57` is `postgres()` and `:69` its `open` — both name
      `PgConn`, so the case is a second pair beside them rather than a generalisation of them.

## Backlog

- `Core\Db` gap 2: SQL Server and SQLite run no statement at all — `crates/nvs-stdlib/src/db.rs`.
- `Core\Queue` gaps 1 and 2 need a shape parameter in the registry — ADR 0135 specifies it.
- § 6's `stats` counts four things and no fifth; a dead-lettered job's attempts are not summed —
  `crates/nvs-stdlib/src/queue.rs`'s known gap 4.
- The five-driver CI matrix ADR 0067's *Verification* names is PostgreSQL, MySQL and MariaDB so far.
