# Handoff

## State

**MySQL's refusals carry ADR 0067 § 8, driver to language.** `server_refusal`
(`crates/nvs-db/src/mysql.rs:857`) answers an `io::Error::other` holding a `ServerError`: the kind
from `kind_of` (`crates/nvs-db/src/mysql.rs:888`), the `SQLSTATE`, and the vendor integer in
`ServerError::driver_code` — a field added this session at `crates/nvs-db/src/conn.rs:549` that
PostgreSQL answers `None` for. `statement_failure` writes it into `DRIVER_CODE_SLOT`, so
`Db\DbError::$driverCode` reads `1062` on MySQL and stays `null` on PostgreSQL. Three decisions the
two module docs own in full: the table is keyed on the **vendor integer** where `pg.rs`'s is keyed on
the `SQLSTATE`, because MySQL fills `HY000` for a deadlock, a lock timeout and a shutdown alike;
**1205 is a `Timeout` and not a `Deadlock`**, so § 7 will not retry it — InnoDB rolls back only the
statement and leaves the transaction open holding its locks; and the error is an `Other` rather than
the old `PermissionDenied`, which had put every MySQL refusal outside the one arm
`nvs_stdlib::db` throws `Db\DbError` from (now a playbook bullet).

**Two readers ask `ServerError::of` rather than something coarser.** `poison_on_write`
(`crates/nvs-db/src/mysql.rs:2960`), which the change also moves the `LOCAL INFILE` refusal to the
poisoning side of — nobody reads the status the server sends after the empty transfer packet — and
`commit` (`crates/nvs-db/src/mysql.rs:2048`), which was reading a fact about the transaction off the
connection state that answer had left.

**No MySQL path has met a server**, § 8's included: what holds all of it is `mysql.rs`'s scripted
`Peer` — now the code table asserted as a table, and § 7's retryable pair asserted on both sides.
`crates/nvs-db/tests/handshake.rs` is still PostgreSQL only, and that leg is the largest hole in this
goal, so it is the next group.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for a
shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own
ADR, not a slice. Untouched this session.

**`orient.py` gaps:** `[context] adrs` printed § 1, § 9 and § 13 while the item was specified by
**§ 8**, which had to be sliced by hand — the fourth session running where the section naming the
work is the one missing. Add **§ 7 and § 8** to that field.

## Next group

**The MySQL matrix leg, one file set: `crates/nvs-db/tests/handshake.rs` and
`crates/nvs-db/src/matrix.rs`, against `tests/db/compose.yaml`'s `mysql` service. Everything above is
held by a scripted peer; this is what makes a real server hold it.**

- [ ] **`handshake.rs` opens a real `MySqlConn`** — the `postgres()`/`open()` pair twinned for the
      other driver, TLS and the matrix CA included. `crates/nvs-db/tests/handshake.rs:72`,
      `crates/nvs-db/tests/handshake.rs:112`, `crates/nvs-db/src/matrix.rs:107`,
      `tests/db/compose.yaml:98`.
- [ ] **§ 7's transaction against that server** — opened, nested to a savepoint, rolled back to it,
      committed, with the depth read back. ADR 0067 § 7. `crates/nvs-db/src/mysql.rs:1429`,
      `crates/nvs-db/tests/handshake.rs:159`.
- [ ] **§ 8's kind against that server** — a duplicate key answers `1062`/`UniqueViolation` and a
      real MySQL confirms the table a `Peer` can only agree with. ADR 0067 § 8.
      `crates/nvs-db/src/mysql.rs:888`, `crates/nvs-db/tests/handshake.rs:122`.

## Backlog

- `Core\Db::open` waits on a registry type for a shape parameter — `nvs_stdlib::db` known gap 1, and
  the stage-2 acceptance check that has not run yet.
- MariaDB is its own driver and owes its own § 8 table — the goal's standing decisions.
- `crate::queue`'s four members still hold their own `postgres_of` — `nvs_stdlib::db` known gap 2.
- `stream`/`streamAs` and `close` are owed whole — `nvs_stdlib::db` known gap 5.
- SQL Server and SQLite have no driver at all — `docs/plan/m8.md`.
