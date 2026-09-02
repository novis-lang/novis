# Handoff

## State

**MySQL's real-server coverage is § 3, § 7, § 8 and § 13.** Five cases in
`crates/nvs-db/tests/handshake.rs` — the TLS/auth triple, the savepoint, the duplicate key, and now
§ 3's park (`SELECT CAST(SLEEP(0.5) AS CHAR)` under two tasks on one `Scheduler`, asserting a turn
taken by the companion strictly between the statement leaving and its answer arriving) and § 13's
reset, which asks the *property* rather than the command: a session variable reads `NULL` after it,
a temporary table is refused `1146`/`42S02`, `depth()` is 0 from inside an open transaction, and the
identically-spelled probe run either side of the reset proves § 1's cache went with it, because a
surviving entry would name a statement id the server has forgotten.

**`crates/nvs-db/tests/pool_reuse.rs` is twinned three ways on MySQL** — reuse, past `lifetime`,
past `idle` — with `CONNECTION_ID()` standing where `pg_backend_pid()` stands. Its helpers are
`mysql()`, `mysql_open`, `mysql_one_value` and the three query consts at
`crates/nvs-db/tests/pool_reuse.rs:178`; `open` now shares the new `address` helper with them.
`python tools/db-matrix.py --driver postgres --driver mysql --no-up` is `2/2 drivers ok`.

**MariaDB is the largest hole, and its first slice is not the driver.** `tools/db-matrix.py` reports
its leg `n/a` because `tests/db/compose.yaml` serves it no anchor a client can be handed —
`docs/agent/loop-goal.toml:2896` names giving it the `certs` volume as stage 6's own first work, and
the `mysql` service two blocks above is the worked example. `crates/nvs-db/src/conn.rs:707` is still
`MariaConn` with no `connect`.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for
a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own
ADR, not a slice. Untouched this session.

**`orient.py` gaps: none.** The pack carried every section these three slices needed.

## Next group

**The two servers the matrix cannot verify, one file set: `tests/db/compose.yaml` and
`tools/db-matrix.py`. The `mysql` service is the worked example for both, and neither slice touches
`crates/`.**

- [ ] **MariaDB gets a certificate a client can verify** — the `certs` volume read-only, a
      `depends_on` on that job, and `--ssl-ca`/`--ssl-cert`/`--ssl-key` at `/certs/…`, mirroring the
      `mysql` service; then the driver row's anchor so the leg runs rather than reporting `n/a`.
      Stage 6, `docs/agent/loop-goal.toml:2896`. `tests/db/compose.yaml:134`,
      `tests/db/compose.yaml:110`, `tools/db-matrix.py:153`.
- [ ] **SQL Server gets one too** — the same slice on the other server, and the difference is that it
      keeps its certificate inside the instance rather than serving none, so the anchor has to be put
      *on disk* for it rather than mounted. Same stage comment.
      `tests/db/compose.yaml:187`, `tools/db-matrix.py:153`.
- [ ] **`MariaConn::connect`** — the driver behind those legs, and its own file set rather than this
      group's: `mysql.rs`'s codec with MariaDB's auth plugins and its own error table. ADR 0067 § 3.
      `crates/nvs-db/src/conn.rs:707`, `crates/nvs-db/src/mysql.rs:1453`.

## Backlog

- `Core\Db::open`'s shape-parameter registry type — `nvs_stdlib::db` known gap 1, wants an ADR.
- `pool = false` has no MySQL twin in `pool_reuse.rs`; the other three bounds do.
- MariaDB's three stage-6 cases — `docs/agent/loop-goal.toml:2913`.
- MSSQL and SQLite legs of the matrix — stage 6's `want` list.
- § 11's `slow_query` line against a real server — stage 9.
