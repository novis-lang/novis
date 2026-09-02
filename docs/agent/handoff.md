# Handoff

## State

**MySQL's driver opens.** `crates/nvs-db/src/mysql.rs` holds the greeting, ADR 0132 § 3's
`CLIENT_SSL` upgrade, the authentication exchange over `mysql_common`'s plugins, ADR 0067 § 3's
forced `utf8mb4` collation, § 9's declared zone as a `SET time_zone` and § 13's
`COM_RESET_CONNECTION`. `MySqlConn` now carries a wire, the agreed capability set and the zone.

**The plugin roster is the security decision this slice took, and it is the module doc's to
explain**: `caching_sha2_password` and `mysql_native_password` only, refused by name at *both*
places a plugin can be named — the greeting and an `AuthSwitchRequest` — because the switch is how
a server talks a client into handing over the password itself.

**Stage 2's `local_infile_is_refused_and_no_file_is_sent` now passes**, so the driver's acceptance
check should go green. It is a property of the client twice over: `CLIENT_LOCAL_FILES` is absent
from `CLIENT_CAPABILITIES`, and `read_ok` refuses a `0xFB` anyway, answering with the empty packet
that terminates a transfer having sent nothing.

**`mysql_common` is taken with `default-features = false`** and `flate2` is named directly to pick
`rust_backend` — the workspace manifest's comment owns why, and the playbook has the trap.
`tools/gen-attribution.py` re-ran clean, so no C dependency was added.

**Nothing above this crate can reach a MySQL connection yet**: there is no `MySqlTarget::resolve`,
so a `[db.<name>]` block with `driver = "mysql"` still finds no opener. `crates/nvs-db/src/lib.rs`'s
module doc says so, and the statement path is the next group.

**`orient.py` gaps:** `[context] adrs` wants ADR 0067 §§ 4 and 5 (the extended-query state machine's
contract and the placeholder rewriter) for the statement slices below; `modules` already covers
`nvs-db/src/*`.

## Next group

**MySQL's statement path — one file set: `crates/nvs-db/src/mysql.rs`, `crates/nvs-db/src/sql.rs`
and `crates/nvs-db/src/conn.rs`. `crates/nvs-db/src/pg.rs:2676` (`start_statement`) is the shape all
three slices copy, and `crates/nvs-db/src/mysql.rs:704` (`read_ok`) is the packet reader they widen.**

- [ ] **One statement over `COM_STMT_PREPARE`/`COM_STMT_EXECUTE`** — ADR 0067 § 1's two round
      trips, and the cost recorded rather than hidden. Widen `crates/nvs-db/src/mysql.rs:704`'s
      `read_ok` into a response reader that also answers a column count, then write the prepare and
      the binary-protocol execution beside it at `crates/nvs-db/src/mysql.rs:777`.
      `crates/nvs-db/src/pg.rs:2676` is the free-function-generic-in-the-stream shape that keeps it
      testable, and `crates/nvs-db/src/sql.rs:99` is the `?` placeholder this dialect already emits.
- [ ] **`MySqlTarget::resolve` off a `[db.<name>]` block** — ADR 0067 § 2, mirroring
      `crates/nvs-db/src/pg.rs:386`'s `PgTarget::resolve` and its `BlockError`. Until this exists
      nothing outside the crate can open one; the struct is `crates/nvs-db/src/mysql.rs:206`.
- [ ] **§ 1's statement cache on this connection** — the LRU is already shared data
      (`crates/nvs-db/src/sql.rs`'s `StatementCache`), but MySQL's key is a server-side statement id
      rather than a name, and § 13's reset invalidates the whole cache. Add the field at
      `crates/nvs-db/src/conn.rs:467` and clear it in `reset_session`.

## Backlog

- MariaDB is its own driver — ADR 0067, ADR 0132 § 2; `MariaConn` is still the placeholder.
- § 8's `DbErrorKind` table for MySQL — the handshake reports a raw code and `SQLSTATE` today.
- Stage 9's other two items stay blocked three deep — known gap 6 of `crates/nvs-types/src/intrinsics.rs`.
- `tools/db-matrix.py` reports MariaDB and SQL Server `n/a` for want of a trust anchor — `crates/nvs-db/src/lib.rs`'s module doc.
- SQL Server's TDS 7.4 is written by hand — ADR 0132 § 2, the largest single piece left.
- SQLite's `rusqlite` is the one audited C dependency — ADR 0051 § 4, and the slice that takes it also writes the ledger row.
