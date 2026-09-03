# Handoff

## State

**M8 goal 5, stage 7. SQL Server is encrypted: PRELOGIN and ADR 0067 § 3's tunnelled TLS.**
`crates/nvs-db/src/tds.rs:947`'s `prelogin` sends the option table and refuses any server that will
not encrypt the whole session; `crates/nvs-db/src/tds.rs:1154`'s `negotiate_tls` then runs the whole
`rustls` handshake through `crates/nvs-db/src/tds.rs:989`'s `Tunnel`, which frames every record as a
`PreLogin` payload until `TunnelEnd::handshake_done` and passes through afterwards. 33 unit tests,
none of which needs a socket or a certificate.

**Nothing opens a connection yet**: `crates/nvs-db/src/conn.rs:744`'s `TdsConn` is untouched and is
still the stub carrying `state` and nothing else, so nothing calls `negotiate_tls`.

**The standing acceptance failure is unchanged and is not a regression.** Stage 7 reports
`mssql_resets_through_sp_reset_connection_and_loses_its_cache` did not run; § 13's reset needs LOGIN7
and the token stream in front of it, which is the group below in order.

**The group below is three slices and it was left whole on purpose.** LOGIN7's title half — "the
packet size the server answers with" — is an `ENVCHANGE` token, so it cannot land before the token
stream reads one; splitting it across two sessions costs more than doing both with one file loaded.

## Next group

**One file set: `crates/nvs-db/src/tds.rs`**, plus `crates/nvs-db/src/conn.rs` for the third slice.
`crates/nvs-db/src/tds.rs:853`'s `prelogin_request` is the shape every message below takes — an
offset-and-length table in front of its blobs — and `crates/nvs-db/src/mysql.rs:1408`'s `connect` is
the sequencing to copy. Nothing here needs an ADR: 0132 is the slot and it is spent.

- [ ] **LOGIN7, built from the target** (0067 § 3). A 94-byte fixed header, then thirteen
      little-endian offset/length pairs whose lengths are *characters*, then UCS-2LE blobs; the
      password is nibble-swapped and XORed with `0xA5`, which is why § 3's TLS is not optional here.
      `crates/nvs-db/src/tds.rs:134`'s `TdsTarget` is every field it carries, and
      `crates/nvs-db/src/tds.rs:761`'s `TDS_VERSION` is the version field. Decide and write down what
      `OptionFlags1` sets: a failed initial-database change must be fatal, since
      `crates/nvs-db/src/tds.rs:134` makes `database` required precisely so a connection cannot mean
      whatever the server's default was.
- [ ] **The token stream, as far as `DONE`, `ERROR`, `LOGINACK` and `ENVCHANGE`** (0067 §§ 3 and 8).
      A handshake answer is one `crates/nvs-db/src/tds.rs:726` `read_message`, parsed as tokens;
      `ENVCHANGE` type 4 is the packet size the server chose, which goes to
      `crates/nvs-db/src/tds.rs:465`'s `set_packet_size` and is the second half of the slice above.
      `ERROR` is § 8's `DbError`, so its normalisation table is SQL Server's own, the way
      `crates/nvs-db/src/maria.rs` has MariaDB's.
- [ ] **`TdsConn` gains a wire, and `connect` opens one** (0067 § 3).
      `crates/nvs-db/src/conn.rs:744` is the stub to widen, and its wire type is
      `Wire<NvsTls<Tunnel<NvsTcp>>>` — the tunnel stays in the type for the connection's life, which
      `crates/nvs-db/src/tds.rs:989`'s doc argues. Write the sequencing as free functions generic in
      the stream, per the playbook's `PgConn` bullet, or none of it is reachable from a unit test.

## Backlog

- `mssql_resets_through_sp_reset_connection_and_loses_its_cache` — § 13's reset, after the group
  above; `docs/agent/loop-goal.toml` stage 7 owns the check.
- § 5's placeholder rewriting for this dialect (`@P1`), `crates/nvs-db/src/sql.rs`'s `Dialect`.
- § 1's statement cache over `sp_prepexec`/`sp_execute`, which is `PacketType::Rpc`.
- The five-driver container matrix needs a SQL Server service in `tests/db/compose.yaml`.
