# Handoff

## State

**M8 goal 5. The queue is the last PostgreSQL-only surface in `nvs-stdlib`, and it stays that way for
one more group: what it lacks is SQL, not a send.** `crates/nvs-stdlib/src/queue.rs:1342`'s
`no_dialect` is that decision's home — the queue keeps a refusal of its own rather than borrowing
`crate::db`'s `Framed`, because that seam unifies a *wire* two drivers share and nothing here is
blocked on the wire. Its sentence now splits on whether the driver sends: MySQL and MariaDB earn "the
queue's statements are PostgreSQL's dialect" (this module's gap 5), SQL Server and SQLite earn "runs
no statement at all" (`Core\Db`'s gap 2). `crate::db::rendering_for` is `pub(crate)` so
`the_queues_refusal_says_which_of_the_two_things_is_missing` reads the crate's one roster instead of
a second list.

**The finding that re-scopes the rest of the group: MySQL has no partial unique index, no
data-modifying CTE and no `RETURNING`** (MariaDB has `RETURNING` but not the CTE), and
`MIGRATION`'s `jobs.dedupe`, `INSERT`, `CLAIM` and `DEAD_LETTER` each rest on one of those. So §§ 2
and 4 are a second design rather than a translation, and the schema is worth writing only together
with the statements that read it — `the_ddl_creates_every_column_the_statements_name`
(`crates/nvs-stdlib/src/queue.rs:1924`) is what binds the two lists, and a DDL no statement claims
has nothing to be held to.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` is a busy-state cell. Do not rename it, split it, or write a MySQL-only test
under it.

## Next group

**One file set: `crates/nvs-stdlib/src/queue.rs`, with `crates/nvs-cli/src/queue.rs` behind it** —
the migrate command is what chooses a list, so the schema slice lands in both. Take the first two
together: a schema written apart from the statements that read it is the thing this session's State
block argues against.

- [ ] **§ 2's schema in MySQL's dialect, with the dedupe index as a generated column** (0084 § 2).
      `crates/nvs-stdlib/src/queue.rs:181` is `MIGRATION` and its doc owns the dialect argument;
      `crates/nvs-stdlib/src/queue.rs:1924` is the test binding DDL to statements and needs to walk
      both lists. Two constructs have no MySQL spelling and the answer is decided per construct, not
      per statement: a partial unique index becomes a stored generated column (`case when state = 0
      then dedupe_key else null end`) with a plain unique index over it, since NULLs do not collide;
      and `create index if not exists` does not exist on MySQL, so an index declared inside the
      `create table if not exists` is what keeps § 2's *created and upgraded* reading.
- [ ] **§ 4's claim needs a dialect, not a translation** (0084 § 4).
      `crates/nvs-stdlib/src/queue.rs:302` is `CLAIM`, `crates/nvs-stdlib/src/queue.rs:253` is
      `INSERT` and `crates/nvs-stdlib/src/queue.rs:383` is `DEAD_LETTER` — all three are
      data-modifying CTEs with a `returning`, and MySQL has neither half. `for update skip locked`
      *is* there (MySQL 8, MariaDB 10.6), so the claim is a select-then-update inside one
      transaction keyed on the lease, and the lease key is what already makes that safe.
- [ ] **`nvs queue migrate` picks the list by driver** (0084 § 2).
      `crates/nvs-cli/src/queue.rs:70` is `DIALECT`, a `&str` that is one driver by construction, and
      `crates/nvs-cli/src/queue.rs:141` is the `match` that refuses everything else. The refusal's
      wording is already right; what changes is that two drivers stop reaching it.

## Backlog

- Gap 5's last mile: `postgres_of` (`crates/nvs-stdlib/src/queue.rs:1300`) widens to a per-driver
  gate only once the three slices above land — `crates/nvs-stdlib/src/queue.rs`'s module doc.
- Queue gap 1 says the registry cannot spell a shape parameter, so `limits`/`grants` are undeclared;
  ADR 0135 decided `CoreTy::Shape` and `Core\Db::open` is live, so that gap may be stale —
  `crates/nvs-stdlib/src/queue.rs:41`.
- Stage 7's `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` stays open until a TDS
  driver exists — `docs/agent/loop-goal.toml`.
- `Core\Queue`'s `$args` is `mixed` and so does not refuse a `secret`, which ADR 0084 § 1 asks for —
  module gap 2.
