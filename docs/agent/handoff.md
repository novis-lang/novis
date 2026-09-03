# Handoff

## State

**M8 goal 5. Every statement `crates/nvs-stdlib/src/queue.rs` sends has now been parsed by a real
server**, on both framed drivers. That closes the last text-shaped half of that module's gap 5:
what is left of it is `crate::db`'s gap 2, the two drivers that send no statement at all.

`CANCEL_MYSQL` and `COUNTS_MYSQL` are `pub` as `STATUS_MYSQL` is, with one case each —
`a_framed_cancel_is_decided_by_the_statement_and_read_off_the_affected_count` and
`a_framed_stats_counts_one_queue_across_both_of_its_tables` — each proven to *run* by breaking one
assertion and watching MySQL report it by name. `cancel` and `stats` are the two test-file helpers
they added, beside `status`.

**`python tools/db-matrix.py` is green on mysql and mariadb**; postgres was not re-run, because
nothing this session touched a driver and neither new case has a PostgreSQL half.

**The standing acceptance failure is unchanged and is still not a regression.** Stage 7 reports
`mssql_resets_through_sp_reset_connection_and_loses_its_cache` did not run; it lands with the TDS
driver and not before, and `crates/nvs-db/src/conn.rs:744`'s `TdsConn` is a stub carrying `state`
and nothing else. The group below is the start of closing it.

## Next group

**One file set: a new `crates/nvs-db/src/tds.rs`, with `crates/nvs-db/src/conn.rs` and
`crates/nvs-db/src/mysql.rs` read at the anchors each item names.** ADR 0132 § 5 is the shape — a
driver owns its own state machine, its own error table and its own codec — and `crates/nvs-db/src/mysql.rs`
is the nearer of the two landed drivers to copy from, because SQL Server's upgrade is in-band as
MySQL's is rather than PostgreSQL's one-byte reply. Nothing here needs an ADR: 0132 is the slot,
and it is spent.

- [ ] **TDS 7.4's packet framing, as a sans-IO codec** (0132 § 5). New `crates/nvs-db/src/tds.rs`,
      named in `crates/nvs-db/src/lib.rs:100`'s *What is here, and what is not yet* the way `mysql`
      is. An 8-byte header — type, status, big-endian length, spid, packet id, window — in front of
      every message, so this is the piece all three items below rest on and it is worth its own
      slice. `crates/nvs-db/src/mysql.rs:2285`'s `simple_command` is the shape a
      `Read + Write`-generic step takes here.
- [ ] **`TdsTarget::resolve` reads a `[db.<name>]` block** (0067 § 3). `crates/nvs-db/src/mysql.rs:276`'s
      `MySqlTarget` is the shape, down to what refuses `tainted` and how `password_file` is read;
      `crates/nvs-db/src/conn.rs:744`'s `TdsConn` gains the fields it answers with, beside the
      `state` cell it already carries.
- [ ] **PRELOGIN, then § 3's TLS tunnelled inside TDS packets** (0067 § 3).
      `crates/nvs-db/src/mysql.rs:544`'s `upgrade` is the in-band shape and `NvsTls` is already
      generic over its transport for exactly this; TLS defaults to `VerifyFull` and a server that
      will not offer it is refused rather than downgraded, as `crates/nvs-db/src/mysql.rs:3502`'s
      case pins for `CLIENT_SSL`.

## Backlog

- The SQLite driver: `rusqlite` over `nvs-host`'s blocking pool, `crates/nvs-db/src/conn.rs:755`'s
  `SqliteConn` — `crates/nvs-db/src/lib.rs` § *What is here, and what is not yet* owns the order.
- `Core\Queue`'s `limits` and `grants` are undeclared, waiting on a shape parameter —
  `crates/nvs-stdlib/src/queue.rs` gap 1, and ADR 0135 is now the spelling it was waiting for.
- `$args` is `mixed` and so does not refuse a `secret` — `crates/nvs-stdlib/src/queue.rs` gap 2.
- No PostgreSQL `cancel`/`stats` case exists in `crates/nvs-stdlib/tests/queue.rs`; that half is
  `counted_row`'s and is covered by the member's own tests, so this is a note and not a gap.
- `tools/db-matrix.py --all`'s `mssql: ok` asserts what the crate reaches *without* a driver, and
  will start asserting the driver the moment the group above lands.
