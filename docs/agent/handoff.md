# Handoff

## State

**ADR 0067 § 7's `transaction` runs on a MySQL connection, both halves.** The driver's
`begin`/`commit`/`roll_back` are `crates/nvs-db/src/mysql.rs:1880` onward, free and generic in the
stream as the playbook prescribes, sharing `pg.rs`'s `open_transaction` and `savepoint_name` so the
naming rule and the no-transaction refusal have one home each. Three decisions the module doc owns in
full: § 7's commands go out as `COM_QUERY` and never as § 1's prepared statements; an isolation level
is a `SET TRANSACTION` of its own *ahead* of `START TRANSACTION`, because MySQL takes no level on the
statement that opens a transaction, and both round trips sit inside one § 11 span; and a nested
`ROLLBACK TO SAVEPOINT` needs no `RELEASE` after it, because MySQL deletes the savepoint a same-named
one finds where PostgreSQL stacks it — this driver never negotiates `CLIENT_MULTI_STATEMENTS`, so
`pg.rs`'s two-statements-in-one-command trick is not available and is not needed.

**`nvs-stdlib`'s `transaction` branches on the connection.** `postgres_of` is gone from
`crates/nvs-stdlib/src/db.rs`; `Transacting` (`:3572`) is the enum its five call sites now borrow
through, and a driver with no § 7 path refuses under `driverless`'s one sentence. `crate::queue`'s
four members are now *all* of the module's known gap 2 above the handshake, and they keep their own
`postgres_of`.

**The gap this session found, and the reason it is the next group.** MySQL's `server_refusal`
(`crates/nvs-db/src/mysql.rs:844`) builds a plain `PermissionDenied` `io::Error` and attaches **no
`ServerError`**, where `pg.rs` attaches one carrying § 8's kind. So on MySQL a deadlock is not
retryable under § 7's `{retries: n}`, `Db\DbError`'s kind reads as nothing, and `mysql.rs`'s `commit`
has to tell "the server refused this" from "we refused it" by reading the connection state
`poison_on_write` left rather than by asking `ServerError::of` as `pg.rs` does.

**No MySQL path has met a server**, § 7's included: what holds all of it is the type checker plus ten
unit tests against `mysql.rs`'s scripted `Peer`. `tools/db-matrix.py` and
`crates/nvs-db/src/matrix.rs` are the leg that would, and it is still the largest hole in this goal.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for a
shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own
ADR, not a slice. Untouched this session.

**`orient.py` gaps:** `[context] adrs` printed § 1, § 9 and § 13 while the item was specified by
**§ 7**, which had to be sliced by hand — the third session running where the section naming the work
was the one missing. The next group wants **§ 8** there too.

## Next group

**MySQL's refusals carry § 8's kind, one file set: `crates/nvs-db/src/mysql.rs` and
`crates/nvs-stdlib/src/db.rs`, against `crates/nvs-db/src/pg.rs`'s § 8 half. The driver first — the
stdlib arm has nothing to read until the kind is on the error.**

- [ ] **`server_refusal` answers a `ServerError` carrying § 8's kind** — ADR 0067 § 8.
      `crates/nvs-db/src/mysql.rs:844` is the function, `crates/nvs-db/src/pg.rs:962`
      (`server_error`) and `crates/nvs-db/src/pg.rs:971` (`kind_of`) are the shapes. MySQL has both a
      vendor integer and a `SQLSTATE`, and § 8's `driverCode` field exists for the first — 1213 and
      1205 are the two that decide a retry, and `ServerError::backend` is `"mysql"`.
- [ ] **`commit` asks `ServerError::of` instead of reading the connection state** — ADR 0067 § 7.
      `crates/nvs-db/src/mysql.rs:1967`. The state test there is exact but indirect, and it exists
      only because the kind above was missing; landing it makes the two drivers' `commit` read alike.
- [ ] **A `.nvst` or matrix case over a MySQL transaction** — `crates/nvs-db/src/matrix.rs:1` names
      the `NVS_DB_MATRIX_*` fields and `tools/db-matrix.py` starts the container. § 7's nesting is
      the case worth writing: a savepoint the server really keeps, and a rollback that really undoes
      only the inner level.

## Backlog

- `crate::queue`'s four members are PostgreSQL-only — `crates/nvs-stdlib/src/queue.rs:1295`.
- § 7's retry has no backoff at all — `nvs_stdlib::db` known gap 9.
- `Core\Db::open` waits on a registry type for a shape parameter — `nvs_stdlib::db` known gap 1, and
  the driver's own acceptance check.
- MariaDB binds and has nowhere to send — `nvs_stdlib::db` known gap 2.
- No matrix leg runs in `verify.py`, so every MySQL claim rests on a scripted peer.
