# Handoff

## State

**ADR 0067 § 13's pool now has a MariaDB telling, and it is the server that says so.**
`crates/nvs-db/tests/pool_reuse.rs` carries `mariadb()`, `mariadb_open`, `mariadb_one_value` and
`two_requests_on_one_core_share_one_mariadb_connection`: two requests on one core draw the same
`CONNECTION_ID()`, and the session variable the first left behind is gone after
`MariaConn::reset`'s `COM_RESET_CONNECTION`. `python tools/db-matrix.py --driver mariadb` is green,
and the case was confirmed to *run and assert* rather than skip — flipping its marker expectation
made the leg report `FAILED` naming this test, and flipping it back made it `ok`.

The row walk is shared and everything around it is twinned, exactly as `handshake.rs` already does
it: `first_text` takes the `MySqlRows` both drivers answer, and `MYSQL_ID`/`MYSQL_MARK`/
`MYSQL_MARKER` are read by both legs because the spelling is the family's rather than either
server's. **`lifetime` and `idle` are deliberately not twinned a third time** — those bounds are
decided by `nvs_runtime::pool` before a driver is consulted, so a MariaDB copy would re-ask a
question with no MariaDB in it. The module doc owns that reasoning.

**Stage 6's third check was already on disk**: `the_mariadb_auth_plugins_are_implemented_in_rust_or_refused_by_name`
is `crates/nvs-db/src/maria.rs:558` and passes. That group item was stale; the group's only real
remainder is the bulk protocol below.

**The driver's stage-2 check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for
a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its
own ADR, not a slice.

**`orient.py` gaps: none.** The pack's § 13 and the `pool_reuse.rs` anchors were exactly what the
slice turned on.

## Next group

**`execute_many_uses_the_bulk_protocol_on_mariadb` is a wire feature and not a test, and it is the
last of stage 6. One file set: `crates/nvs-db/src/mysql.rs`, `crates/nvs-db/src/maria.rs` and
`crates/nvs-db/tests/handshake.rs`. `MariaConn::execute_many` today delegates to MySQL's N-executes
loop, so all three slices are about replacing that one delegation.**

- [ ] **MariaDB's extended capability word** — `MARIADB_CLIENT_STMT_BULK_OPERATIONS` is bit 34, and
      `mysql_common`'s `CapabilityFlags` is 32 bits wide, so it is not in `CLIENT_CAPABILITIES` and
      cannot be. Decide what this driver writes where the handshake response's last reserved four
      bytes are, and whether `HandshakeResponse::new` can carry it at all or the reply is composed
      here. Refusing the feature is a legitimate outcome, and it is then written down rather than
      left implied. `crates/nvs-db/src/mysql.rs:247`, `crates/nvs-db/src/mysql.rs:819`,
      `crates/nvs-db/src/mysql.rs:838`.
- [ ] **`COM_STMT_BULK_EXECUTE` (`0xFA`) in `maria.rs`** — one prepare, one command carrying every
      set: statement id, bulk flags, the parameter types once, then per row one indicator byte
      (`0` value, `1` NULL) and the value. `MariaConn::execute_many` sends it when the bit was
      negotiated and falls back to `crate::mysql::execute_many` when it was not, which is also the
      answer if the slice above refuses the feature. `crates/nvs-db/src/maria.rs:410`,
      `crates/nvs-db/src/mysql.rs:1935`, `crates/nvs-db/src/mysql.rs:1677`.
- [ ] **`execute_many_uses_the_bulk_protocol_on_mariadb`** — stage 6's named check, in
      `handshake.rs` beside the MariaDB fixtures. Assert it from the *server's* counters and not
      from what the client thinks it sent: `SHOW SESSION STATUS LIKE 'Com_stmt_bulk_execute'` rises
      by one for a three-set `executeMany` while `Com_stmt_execute` does not rise by three, which is
      the only reading that fails when the fallback path silently stays in place.
      `crates/nvs-db/tests/handshake.rs:930`, `crates/nvs-db/src/maria.rs:410`.

## Backlog

- `a_db_open_target_in_a_denied_range_fails` — waits on `Core\Db::open`'s shape parameter,
  `nvs_stdlib::db` known gap 1.
- SQL Server has no `connect` at all — `docs/agent/loop-goal.toml`'s stage 6 header.
- `pool_reuse.rs`'s `lifetime`/`idle` cases stay two-driver on purpose; that file's module doc.
- Stage 7's remaining pool checks — `docs/agent/loop-goal.toml`'s stage 7 block.
