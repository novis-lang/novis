# Handoff

## State

**M8 goal 5, stage 7. SQL Server's handshake is written end to end bar the socket.** Framing and
PRELOGIN's tunnelled TLS (`crates/nvs-db/src/tds.rs:1156`), LOGIN7 built from the block
(`crates/nvs-db/src/tds.rs:1390`), and the login's answer read as tokens
(`crates/nvs-db/src/tds.rs:1738`) — `LOGINACK`, `ENVCHANGE`, `ERROR`, `INFO` and `DONE`. 39 unit
tests, none of which needs a socket or a certificate.

**LOGIN7's decisions live on its flag constants, not in a comment block**: `fDatabase` fatal so a
failed initial database fails the login, `fODBC` for the ANSI defaults — implicit transactions off
is what § 7's closure rests on — `fUseDB` for the `ENVCHANGE` that reports a `USE`, and five fields
that go out empty for a reason each. § 8's table is `crates/nvs-db/src/tds.rs:1539`, keyed on the
error number alone because TDS carries no `SQLSTATE`; `547` merges foreign key and `CHECK`, so
`DbErrorKind::CheckViolation` is unreachable on this backend and the doc says so.

**Nothing sends any of it.** `crates/nvs-db/src/conn.rs:744`'s `TdsConn` is still `state` and
nothing else, so there is no connection to sequence a handshake on.

**The standing acceptance failure is unchanged and is not a regression.**
`mssql_resets_through_sp_reset_connection_and_loses_its_cache` needs a live connection, a statement
cache to lose and § 13's reset — the three slices below, in that order.

The two slices landed as one commit because they are one file; `git log` cannot read them apart.

## Next group

**One file set: `crates/nvs-db/src/conn.rs` and `crates/nvs-db/src/tds.rs`.**
`crates/nvs-db/src/mysql.rs:1408`'s `connect` is the sequencing to copy, and the playbook's first
`-p nvs-db` trap is binding here: write the sequencing as a free function generic in the stream, or
no unit test can reach it.

- [ ] **`TdsConn` gains a wire, and `connect` opens one** (0067 § 3). `crates/nvs-db/src/conn.rs:744`,
      `crates/nvs-db/src/tds.rs:1156`, `crates/nvs-db/src/tds.rs:1390`,
      `crates/nvs-db/src/tds.rs:1738`, `crates/nvs-db/src/mysql.rs:1408`. **Two decisions come due
      the moment a token becomes a `ServerError`**: `crates/nvs-db/src/conn.rs:556`'s `driver_code`
      is `Option<u16>` and a SQL Server number is a `LONG` a `THROW` may raise past 65535, and
      `ServerError::sql_state` has no value at all on this backend — TDS sends none, and the five
      characters PDO shows are ODBC's own mapping. Widening the field touches all four drivers and
      `nvs-stdlib`'s reader; leaving it `None` is the cheap answer and loses § 8's `driverCode` for
      every user-raised error.
- [ ] **The result set: `COLMETADATA` and `ROW` over `Wire::read_packet`** (0067 §§ 5 and 9).
      `crates/nvs-db/src/tds.rs:696`, `crates/nvs-db/src/tds.rs:1738`. `Tokens` reads a whole
      message on purpose and a row reader cannot — the module doc's first section is the reason, and
      the two readers share the token constants at `crates/nvs-db/src/tds.rs:1414`.
- [ ] **§ 1's cache over `sp_prepexec`, then § 13's reset through `sp_reset_connection`** (0067 §§ 1
      and 13). `crates/nvs-db/src/tds.rs:338`'s `Status` already carries the `RESET_CONNECTION` bit
      that sends it as a packet flag rather than a statement. This is the slice the driver's
      acceptance check is waiting for.

## Backlog

- A real SQL Server has never answered a Novis LOGIN7; the matrix leg is `crates/nvs-db/src/matrix.rs`.
- Nothing in `nvs-stdlib` builds a `Connection::SqlServer`, so `connect` on an `mssql` block still
  throws (ADR 0067 § 18).
- § 7's nesting on this backend is `SAVE TRANSACTION`, unwritten (ADR 0067 § 7).
- § 9's map over TDS's own type tokens — `datetime2`, `uniqueidentifier`, `decimal` — unwritten.
