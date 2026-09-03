# Handoff

## State

**M8 goal 5. Every statement ADR 0084 §§ 1, 4 and 6 name now has both dialects.** The three the
worker sends joined the second list this session: `crates/nvs-stdlib/src/queue.rs:667`'s
`QUEUES_MYSQL`, `SUCCEEDED_MYSQL` and `RETRY_MYSQL`, ordinary transcriptions rather than `Split`s,
beside PostgreSQL twins now named `QUEUES_POSTGRES`, `SUCCEEDED_POSTGRES` and `RETRY_POSTGRES` for
the file's own convention. `each_second_text_binds_a_value_wherever_its_first_names_one` is the new
agreement test: a `?` per `$` over all six non-`Split` pairs, plus the one place the orders differ —
MySQL's retry binds `run_at` first, because the `set` clause is left of the `where` and a `?` is
positional where a `$n` is not. A caller sending PostgreSQL's order into that text would push every
job's next attempt out to its own id.

**Known gap 5 is now only `nvs-cli`'s worker, and it is a wire question rather than a dialect one.**
`crates/nvs-cli/src/worker.rs` takes a `&mut nvs_db::PgConn` from `start` down to `apply` and names
the PostgreSQL half of every pair, so a worker still refuses a MySQL block its own `push`, `status`,
`cancel` and `stats` accept. Two things the next group needs and this session found: `Queued` and
`crate::db::Framed` are `pub(crate)` in `nvs-stdlib`, so `nvs-cli` cannot borrow that seam and needs
its own arm-flattening over `nvs_db::Connection` — and `crates/nvs-cli/src/queue.rs:218`'s
`open_and_apply!` is already the three-arm connect the worker's `open` is missing, in the same crate.

**None of the MySQL text has met a real server yet.** The unit tests hold the dialects against each
other, against the state enum's ordinals and now against each other's parameter counts; that a MySQL
server accepts them is the matrix case below.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` is a busy-state cell. Do not rename it, split it, or write a MySQL-only test
under it.

## Next group

**One file set: `crates/nvs-cli/src/worker.rs`, with `crates/nvs-stdlib/src/queue.rs` read for its
statements only.** The first two are one refactor split at the point where a claim starts decoding
rows, so take them in order; the first leaves the worker opening and reporting over three drivers
with its claim still PostgreSQL-only, which compiles and is honest.

- [ ] **The worker opens and reports over either driver** (0084 §§ 2 and 6).
      `crates/nvs-cli/src/worker.rs:551` is `open`, which resolves a `PgTarget` and connects —
      `crates/nvs-cli/src/queue.rs:218`'s `open_and_apply!` macro is the three-arm shape to follow,
      in this crate already. `crates/nvs-cli/src/worker.rs:139`, `crates/nvs-cli/src/worker.rs:167`
      and `crates/nvs-cli/src/worker.rs:208` thread the connection; `crates/nvs-cli/src/worker.rs:229`
      is `roster` and `crates/nvs-cli/src/worker.rs:465` is `report`, whose write-backs are
      `SUCCEEDED_MYSQL` and `RETRY_MYSQL` (different bind order — `crates/nvs-stdlib/src/queue.rs:697`
      owns why) and `DEAD_LETTER_MYSQL`, which is a `Split` and so needs
      `crates/nvs-cli/src/worker.rs:523`'s `apply` inside one transaction.
- [ ] **The worker's claim over either driver** (0084 § 4).
      `crates/nvs-cli/src/worker.rs:283` is `claim`, which reads six columns by ordinal through
      `PgColumn::scalar`. `crates/nvs-stdlib/src/queue.rs:622`'s `CLAIM_MYSQL` is a `Split` whose
      `first` answers the same six in the same order, so this is one transaction plus a
      `nvs_db::MySqlRows` walk beside the `PgRows` one.
- [ ] **A matrix case that migrates, pushes and claims against a real MySQL server** (0084 § 2).
      `crates/nvs-db/src/matrix.rs:107` is `endpoint`, which is how a test finds a server at all,
      and `crates/nvs-stdlib/tests/queue.rs:233` is the PostgreSQL case whose shape it takes.

## Backlog

- SQL Server and SQLite still send no statement at all — `Core\Db`'s known gap 2 owns the list.
- `nvs-db`'s stage-7 pool check needs a TDS driver, per `docs/agent/loop-goal.toml`.
- § 2's roster has no configured form, so both dialects scan due rows — `QUEUES_POSTGRES`'s doc.
