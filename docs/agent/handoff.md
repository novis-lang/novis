# Handoff

## State

**M8 goal 5. `python tools/db-matrix.py` is green on postgres, mysql and mariadb**; the run was three
legs, not `--all`, because nothing this session touched a driver.

**The driver's standing acceptance failure was a mis-filed name, and it is split.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` paired a landed half with an unwritable
one; stage 7 now names `a_reset_invalidates_the_statement_cache` (landed, `crates/nvs-db/src/mysql.rs`)
and `mssql_resets_through_sp_reset_connection_and_loses_its_cache` (open, and it lands with the TDS
driver). The playbook's *Tooling* bullet owns the shape. **The reported failure will now be the MSSQL
name, and that is correct** — `TdsConn` is a stub, so it is an open item and not a regression.

**§ 3's transactional pair runs on the framed dialect.**
`a_framed_enqueue_commits_with_the_write_that_made_it` and `a_framed_rolled_back_write_leaves_no_job`,
each proven to *run* by breaking an assertion once and watching MySQL report it by name.

**Three helpers grew a framed spelling, and one of them was hiding a parameter-count difference rather
than a spelling one.** `orders` splits its `create` (`bigint auto_increment`, `varchar(255)`,
`engine=innodb`); `landed` binds the queue name **twice** on the framed dialect, because `$1` is a
number a statement may repeat and `?` is a position that cannot; `order_row` is new and reads the
order's id off the OK packet, since MySQL has no `insert … returning` and MariaDB's would be a
statement only half a framed leg can run.

**Two texts have still never been parsed by a server**, and `crates/nvs-stdlib/src/queue.rs`'s gap 5
says what each costs. Both are still private where `STATUS_MYSQL` is `pub`.

## Next group

**One file set: `crates/nvs-stdlib/tests/queue.rs`, with `crates/nvs-stdlib/src/queue.rs` read at the
constant each item names.** Every case skips with no `NVS_DB_MATRIX_DRIVER`, and a framed case takes
`framed()` rather than a `Leg` so it holds `FRAMED_WRITES` for its whole body. Both are one case each
over helpers that already exist; `crates/nvs-stdlib/tests/queue.rs:2243` is the newest one to copy the
shape from, and `crates/nvs-stdlib/tests/queue.rs:2018`'s `status` case is the nearest reader.

- [ ] **`CANCEL_MYSQL` meets a server** (0084 § 1). `crates/nvs-stdlib/src/queue.rs:856`; it needs
      `pub` as `STATUS_MYSQL` at `crates/nvs-stdlib/src/queue.rs:825` has. The whole claim is that
      `and state = 0` is enforced by the statement and read through the affected count, so a cancel
      that lost the race to a claim answers `0` rather than throwing — which is
      `crates/nvs-stdlib/tests/queue.rs:584`'s `apply` return value and needs no new reader.
- [ ] **`COUNTS_MYSQL` meets a server** (0084 § 6). `crates/nvs-stdlib/src/queue.rs:897`; it needs
      `pub` for the same reason. The claim worth a server is `count(case when … then 1 end)`
      answering `0` over no rows, and `cast(… as signed)` over the `sum` MySQL answers as a
      `decimal` — one aggregate row read four ways, through
      `crates/nvs-stdlib/tests/queue.rs:497`'s `rows`.

## Backlog

- The `[context] adrs` gap, four sessions old: no ADR 0084 section at all. Add `0084 §1`, `§2`, `§3`,
  `§4` and `§6`; § 5 is *Running a job* and is not the one these items mean —
  `docs/agent/loop-goal.toml`.
- `mssql_resets_through_sp_reset_connection_and_loses_its_cache` waits on the TDS driver, which
  `crates/nvs-db/src/conn.rs:744`'s `TdsConn` has none of.
- `crate::db`'s gap 2: the two drivers that send no statement — `crates/nvs-db/src/lib.rs`.
- The "§ 5's three readers" numbering, in three places — `crates/nvs-stdlib/src/queue.rs`.
- Stage 6's comment still says `MariaConn` has no `connect`, which stopped being true at session
  0007 — `docs/agent/loop-goal.toml`.
