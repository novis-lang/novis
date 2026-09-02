# Handoff

## State

**ADR 0067's MariaDB leg now asserts the driver rather than the container.**
`crates/nvs-db/tests/handshake.rs` carries `mariadb()`, `mariadb_connect_as`, `mariadb_open`,
`mariadb_one_value` and `mariadb_run` as the `mysql()` set's twins, and three cases pass against
`tests/db/compose.yaml`'s MariaDB 11.4: § 3's upgrade and authentication, § 9's declared zone, and
stage 6's named `mariadb_returning_is_available_and_mysqls_is_not`. Both legs are green —
`python tools/db-matrix.py --driver mariadb` and `--driver mysql`.

**Every assertion is the server's own answer**, which is what a scripted peer cannot be: the
`information_schema.SESSION_STATUS` row for `Ssl_version` (MariaDB kept the table MySQL 5.7 replaced
with `performance_schema.session_status`), `CURRENT_USER()`, a wrong password refused as
`DbErrorKind::Permission`/`28000`/`1045` through MariaDB's *own* § 8 table, `@@session.time_zone` and
`TIMEDIFF(NOW(), UTC_TIMESTAMP())` both reading `+01:30` with `MariaConn::time_zone()` agreeing, and
one `INSERT … RETURNING` answered by MariaDB and refused `Syntax`/`42000`/`1064` by MySQL 8.4. The
row walk is *shared* rather than twinned — `first_text` takes the `MySqlRows` both drivers answer,
since § 9's decoder is one decoder — and everything above the framing is MariaDB's own.

**The workspace manifest now excludes two `benches/` data trees, not one.** `benches/members/` is
the `.nvs` tree the user is building for ADR 0134's per-feature figures; `members = ["benches/*"]`
made every `cargo` command in the repository fail on it until `exclude` named it. See the new
playbook bullet — the failure names a directory no session wrote.

**The driver's stage-2 check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for a
shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own
ADR, not a slice.

**`orient.py` gaps: none.** The pack's ADR 0067 §§ 3 and 9 and the `mysql()` anchors were exactly
what the slices turned on.

## Next group

**MariaDB's remaining stage-6 and stage-7 specifics, one file set:
`crates/nvs-db/tests/pool_reuse.rs` and `crates/nvs-db/src/maria.rs`.
`two_requests_on_one_core_share_one_mysql_connection` at `crates/nvs-db/tests/pool_reuse.rs:463` is
the worked example for the first, and `handshake.rs`'s MariaDB half is the fixture shape to twin.**

- [ ] **`pool_reuse.rs` twinned a fourth way** — ADR 0067 § 13's `COM_RESET_CONNECTION` over
      MariaDB: two requests on one core draw the *same* connection, `CONNECTION_ID()` says so, and
      the reset left no session variable behind. `crates/nvs-db/tests/pool_reuse.rs:463`,
      `crates/nvs-db/tests/pool_reuse.rs:131`, `crates/nvs-db/src/maria.rs:478`.
- [ ] **`execute_many_uses_the_bulk_protocol_on_mariadb`** — stage 6's named check:
      `COM_STMT_BULK_EXECUTE` is MariaDB's alone and MySQL has no such command, so the two
      `execute_many`s cannot be one. `crates/nvs-db/src/maria.rs:410`,
      `crates/nvs-db/src/mysql.rs:1459`.
- [ ] **`the_mariadb_auth_plugins_are_implemented_in_rust_or_refused_by_name`** — stage 6's third
      check, which ADR 0051 § 4 named in advance so it would be answered by a test rather than by
      convenience. `crates/nvs-db/src/maria.rs:159`, `crates/nvs-db/src/mysql.rs:733`.

## Backlog

- `Core\Db::open` needs a registry type for a shape **parameter** — `crates/nvs-stdlib/src/db.rs`
  known gap 1, what stage 2's failing check waits on, and it wants its own ADR.
- § 3's "the reason is a source literal" for `Core\Html::toSource` is unenforced —
  `crates/nvs-stdlib/src/html.rs` *Known gaps*; both type bands are full.
- SQL Server has no `connect` at all — `docs/agent/loop-goal.toml` stage 6, and its matrix leg
  asserts only what the crate reaches without one.
- MariaDB's `Core\Db` half is untested end to end — no `[db.<name>]` block example runs against it.
