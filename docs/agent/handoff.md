# Handoff

## State

**MariaDB's second capability word is negotiated, and both packets that carry one say the same
thing.** `MARIADB_CLIENT_STMT_BULK_OPERATIONS` is bit 34 and `CapabilityFlags` is `u32`, so it was
never going to fit `CLIENT_CAPABILITIES`; MariaDB's answer is to redefine the handshake's trailing
filler as 19 bytes plus a little-endian `u32`, and `mysql_common` 0.38.2 models exactly that on
`SslRequest`, `HandshakeResponse` and `HandshakePacket`. **Nothing is composed by hand** — the
question the item asked is answered `with_mariadb_ext_capabilities`, and
`crates/nvs-db/src/mysql.rs:741`'s `agreed_extended` doc is that answer's home.

The word is a third field on `crate::mysql::Backend`, which is already the one place the shared
framing asks which server it is framing for: MySQL's is empty, MariaDB's is
`maria::EXTENDED_CAPABILITIES`, and the greeting's own word intersects it. A MySQL server offers
zero because to it those bytes are filler, so the *intersection alone* is the gate and no code asks
which server this is. `the_mariadb_handshake_claims_bulk_operations_and_the_mysql_one_claims_nothing`
asserts all three outcomes over both packets; it was confirmed to assert by dropping the response's
`.with_…` call, which made it fail naming the packet that lost the word. `python tools/db-matrix.py
--driver mariadb --driver mysql` is green, so both real servers still accept the handshake.

**The bulk protocol itself was not taken, and the reason is that it is not a slice.** Reading far
enough to size it turned up three ways `COM_STMT_BULK_EXECUTE` changes what a caller observes, all
against the paragraph in `crates/nvs-db/src/mysql.rs:1983`'s doc that says the drivers agree
deliberately. ADR 0067 § 4 mandates the feature in one clause and settles none of the three, so the
next group opens with the decision rather than with the packet. Details in the group below.

**The driver's stage-2 check is unchanged and still open**: `a_db_open_target_in_a_denied_range_fails`
waits on `Core\Db::open`, blocked on a registry type for a shape **parameter**
(`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own ADR, not a slice.

**`orient.py` gap: `[context] adrs` did not print ADR 0067 § 4**, and it is the section that says
`executeMany` is one prepare and N executions and that MariaDB 10.2+ uses `COM_STMT_BULK_EXECUTE`.
Every remaining slice of this group is about that clause. Add `0067:4` to the manifest.

## Next group

**One decision, then one command, then the check that names it. Same file set as this session's:
`crates/nvs-db/src/mysql.rs`, `crates/nvs-db/src/maria.rs`, `crates/nvs-db/src/conn.rs` and
`crates/nvs-db/tests/handshake.rs`.**

- [ ] **Decide what MariaDB's `executeMany` observes, and write it down.** Bulk is one command, so
      it diverges from the N-executes loop three ways and each needs an answer: a refusal **ends**
      the batch (the loop attempts every later set and reports the first error — the property
      `crates/nvs-db/src/mysql.rs:1983`'s doc calls a deliberate cross-driver agreement); a set that
      answers with a result set — `CALL` — is not something the bulk command accepts, and the loop
      supports it; and the affected count is one aggregate the server computed rather than a sum
      this driver added up. ADR 0067 § 4 (`docs/adr/0067-core-db.md:150`) names the feature in one
      table row and settles none of them. Refusing bulk is still legitimate and is then an amendment
      to that row, not silence. The pre-authorized shape is decide-and-record; an ADR slot for it is
      not in the goal's standing list, so prefer amending § 4 plus the module doc at
      `crates/nvs-db/src/mysql.rs:1983`.
- [ ] **`COM_STMT_BULK_EXECUTE` (`0xFA`) in `maria.rs`** — `mysql_common`'s
      `ComStmtBulkExecuteRequestBuilder` builds it, splits a batch that outgrows `max_allowed_packet`
      across several commands, and refuses mixed arity itself; `MariadbBulkIndicator` is the per-value
      indicator byte. Two gates before it is reachable: the negotiated word has to be **stored** —
      `MariaConn` (`crates/nvs-db/src/conn.rs:707`) has no field for it yet and
      `crates/nvs-db/src/maria.rs:352` is where `agreed_extended` would be called, the greeting still
      being in scope — and a zero-arity batch must stay on the loop, the builder's own doc refusing a
      statement with no parameters. `crates/nvs-db/src/mysql.rs:1812`'s `cached_statement` is private
      and is the prepare half. `crates/nvs-db/src/maria.rs:437` is the delegation to replace.
- [ ] **`execute_many_uses_the_bulk_protocol_on_mariadb`** — stage 6's named check, in
      `crates/nvs-db/tests/handshake.rs:930`'s neighbourhood, against the real server
      `tools/db-matrix.py --driver mariadb` brings up. What it must assert is that the batch cost
      **one** command and not N, which the server will say through `Com_stmt_bulk_execute` in
      `SHOW SESSION STATUS` — a count read before and after, in the shape
      `mariadb_returning_is_available_and_mysqls_is_not` reads its own answer.

## Backlog

- `Core\Db::open` waits on a registry type for a shape parameter — `nvs_stdlib::db` known gap 1.
- `MARIADB_CLIENT_BULK_UNIT_RESULTS` (11.5.1+) would give a per-set answer and reopen the
  affected-count question — `crates/nvs-db/src/maria.rs:111`'s doc says why it is not claimed.
- ADR 0067 § 8's fifth field on `DbError` is still unfilled — `docs/adr/0067-core-db.md` § 8.
