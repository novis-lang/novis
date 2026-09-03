# Handoff

## State

**M8 goal 5. The last acceptance check's failure was a flaky test fixture, not a regression, and it
is fixed.** `crates/nvs-db/src/pg.rs`'s SCRAM fake echoed a suffix of the client nonce whenever the
nonce contained `r=` — about one run in 368. It splits on `,r=` now and
`the_fake_server_echoes_a_client_nonce_that_contains_the_attribute_marker` pins it deterministically.
The playbook's *Writing a test case* bullet owns the shape.

**`crates/nvs-stdlib/tests/queue.rs` now holds five framed cases, and every one of them has run
against real MySQL and MariaDB.** New this session: § 2's dedupe, where the guard reads
`dedupe_pending` but `nvs_jobs_dedupe` is what refuses the second row — asserted on both sides, since
the key frees the moment its holder leaves `Pending`, which is how a stored generated column stands in
for the partial index MySQL has not got; and § 4's visibility bound, where the take-over path is the
rung that tells a `Split` reading `attempts + 1` before its own `update` from one reading after it.

**Two helpers are new and both are the framed dialect's alone.** `push_keyed`
(`crates/nvs-stdlib/tests/queue.rs:581`) runs `INSERT_MYSQL.then` with the dedupe slot filled and
answers a `Result`, because the refusal is the assertion; `pending_for`
(`crates/nvs-stdlib/tests/queue.rs:618`) is `INSERT_MYSQL.first`, the guard read no case had issued.

**`python tools/db-matrix.py --driver postgres --driver mysql --driver mariadb` is 3/3 ok**, and both
new cases were proven to *run* by breaking an assertion once — see the playbook's *Running things*
bullet, because `ok` is also what a skipped case reports.

**Unchanged and still correctly filed.** `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs`
needs SQL Server and there is no TDS driver; `TdsConn` is a busy-state cell. `crate::db`'s gap 2 — the
two drivers that send no statement — is what is left of the queue's own gap list.

**`[context]` gap, unchanged and now two sessions old:** the manifest's `adrs` carries no ADR 0084
section and every item below cites one. Add `0084 §3`, `§4` and `§5`.

## Next group

**One file set: `crates/nvs-stdlib/tests/queue.rs`, with `crates/nvs-stdlib/src/queue.rs` read at the
constant each item names.** Every case skips with no `NVS_DB_MATRIX_DRIVER`, as the eleven already
there do. Take them in any order — they share the helpers and touch nothing else.

- [ ] **§ 5's roster answers on the framed dialect, and say what the other three cost** (0084 § 5).
      `crates/nvs-stdlib/src/queue.rs:670` is `QUEUES_MYSQL` and
      `crates/nvs-stdlib/src/queue.rs:813` is `STATUS_MYSQL`; neither has met a server. Both bind the
      two instants `QUEUES_POSTGRES` binds, and the scan each doc costs out is the claim worth a real
      server: § 2's `nvs_jobs_due` is `(queue, state, run_at)`, so `distinct queue` walks the due rows
      on both dialects rather than the index's leading column.
- [ ] **§ 4's `skip locked` on the framed dialect** (0084 § 4).
      `crates/nvs-stdlib/tests/queue.rs:889` is
      `claiming_is_skip_locked_shaped_on_every_backend_that_has_it`, whose own doc says it asserts one
      backend because there was only one. `crates/nvs-stdlib/src/queue.rs:625` is `CLAIM_MYSQL`, whose
      `for update skip locked` is on the `select` half. The landed case's `statement_timeout` is what
      turns a missing `skip locked` from a hang into a verdict; MySQL and MariaDB spell that bound
      `innodb_lock_wait_timeout`, and a matrix leg that hangs reports nothing at all.
- [ ] **§ 3's two transactional cases on the framed dialect** (0084 § 3).
      `crates/nvs-stdlib/tests/queue.rs:375` is `orders`, which builds § 3's application table with
      `bigserial` and is why `crates/nvs-stdlib/tests/queue.rs:966` and
      `crates/nvs-stdlib/tests/queue.rs:1048` are `postgres()`-gated. One dialect arm on that helper
      opens both to the framed drivers, and § 3's property — the job and the write it caused commit
      together or not at all — is the one this file asserts on one driver.

## Backlog
- The two drivers with no wire close `crate::db`'s gap 2 — `crates/nvs-stdlib/src/queue.rs`'s gap list.
- `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` waits on a TDS driver — `docs/agent/loop-goal.toml` stage 7.
- Add `0084 §3`, `§4`, `§5` to `[context] adrs` — `docs/agent/loop-goal.toml`.
