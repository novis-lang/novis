# Handoff

## State

**Goal `m8-db-queue`, stage 7. The SQL Server leg of the queue is closed at both ends.** A case can
open one: `Conn::SqlServer` (`crates/nvs-stdlib/tests/queue.rs:249`) holds a `TdsConn`, `Dialect`
has a third arm, and `rows` renders a T-SQL column through `nvs_db::tds::scalar`. And a worker can
claim one back: `Wire::SqlServer` (`crates/nvs-cli/src/worker.rs:1473`) with `tds_claim`,
`tds_roster`, `tds_apply` and `tds_dead_letter_in_two` beside the framed ones, so `open` takes the
`open_as!` arm every other socket driver takes and `sql_server_gap` is deleted rather than narrowed.
`tools/db-matrix.py`'s `SUITES` gained `["--bin", "nvs", "worker::"]` for the worker's own case.

**Stage 4's matrix check is red on MariaDB alone, and not on anything this goal has left to
build.** `python tools/db-matrix.py --all --no-up` answers `4/5 drivers ok`: mysql, postgres, mssql
and sqlite pass and `catalog::tests::an_applied_schema_introspects_back_to_an_empty_plan_on_mariadb`
fails, deterministically, with *the schema this server was given is not the schema it answers* over
one step — `ALTER TABLE wide MODIFY COLUMN slug VARCHAR(64)`. The older ledger entries for that
check are a different failure: it ran without `--no-up` on a cold tree and never reached a driver.
`docker compose … up -d --wait` over the four services answers `0` today.

## Next group

**Stage 4: the MariaDB catalog read-back** — one file set: `crates/nvs-db/src/catalog.rs` and
`crates/nvs-db/src/ddl.rs`. This is what stage 4's `[[check]]`
(`docs/agent/loop-goal.toml:10544`) is red on, and it outranks stage 7's remaining work.
`rule:core-classes/schema-plan` owns what a plan step is and `rule:core-classes/schema-introspection`
what a catalog reader owes.

- [ ] **A `varchar(64)` this emitter applied reads back as a column that plans a `MODIFY` to
      `VARCHAR(64)` again**, so an applied schema does not introspect to an empty plan on this
      driver. The assertion is `crates/nvs-db/src/catalog.rs:2275`, inside the shared
      `introspects_back_to_an_empty_plan`, and the case that hands it this connection is
      `crates/nvs-db/src/catalog.rs:2332`. MySQL's own leg passes that same shared body, so what
      differs is one server's `information_schema` answer rather than the shape of the reader.
- [ ] **Re-run the whole matrix before calling it closed** — `python tools/db-matrix.py --all
      --no-up` with the containers already healthy, which is the check's own argv minus the
      bring-up. Its `want` list is the five `<driver>: ok` lines at
      `docs/agent/loop-goal.toml:10551`.

## Backlog

- Stage 7's remainder is the `errors` array: no `RETRY_*` text writes that column
  (`crates/nvs-stdlib/src/queue.rs:554`, `:946`, `:962`), so a retry appends nothing.
- `dead_errors` (`crates/nvs-stdlib/src/queue.rs:977`) answers the exhausting attempt alone, and
  `report` (`crates/nvs-cli/src/worker.rs:1044`) binds it into a move that never reads the column.
- `a_dead_lettered_row_carries_every_attempts_error` exists nowhere; it is `-p nvs-cli`'s
  (`docs/agent/loop-goal.toml:10669`), beside `crates/nvs-cli/src/worker.rs:1771`.
- The `unowned`-tagged gaps in `crates/nvs-stdlib/src/queue.rs` and `crates/nvs-stdlib/src/db/mod.rs`
  are goal `unowned-closures`', per this goal's § *Standing decisions*.
- `crates/nvs-db/src/tds/rows.rs` has no `last_id`; nothing needs one while `output inserted.id`
  answers, and `push_in_tds`'s doc is the one home for why.
