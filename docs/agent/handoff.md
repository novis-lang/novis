# Handoff

## State

**M8 goal 5, stage 7. SQL Server has opened, one layer below where the other drivers did.**
`crates/nvs-db/src/tds.rs` holds TDS 7.4's packet framing written from the specification — there is
no sans-IO crate for the protocol — as a `decode` that answers offsets into a caller's buffer, a
`Codec::encode` that splits a message at the negotiated packet size, and a `Wire<S>` of the shape
`crate::mysql`'s is. `TdsTarget::resolve` reads a `[db.<name>]` block beside it. 17 unit tests, none
of which needs a socket.

**Nothing opens a connection yet**: `crates/nvs-db/src/conn.rs:744`'s `TdsConn` is untouched and is
still the stub carrying `state` and nothing else.

**The standing acceptance failure is unchanged and is not a regression.** Stage 7 reports
`mssql_resets_through_sp_reset_connection_and_loses_its_cache` did not run; § 13's reset needs
PRELOGIN, LOGIN7 and the token stream in front of it, which is the group below in order.

**One § 9 fact recorded rather than decided in an ADR**, because it is the driver's and not the
rule's: SQL Server has no session time zone to *send* the declared zone to — `AT TIME ZONE` is an
expression and `SET` has no such setting — so on this driver alone § 9's zone governs decoding and
`GETDATE()` stays the server's own. `TdsTarget::time_zone`'s doc owns it.

## Next group

**One file set: `crates/nvs-db/src/tds.rs`**, with `crates/nvs-db/src/mysql.rs:1408`'s `connect`
read as the shape — its in-band upgrade is the nearest twin to this one — and
`crates/nvs-host/src/tls.rs`'s `NvsTls::over_bundle` as the wrap. Nothing here needs an ADR: 0132 is
the slot and it is spent.

- [ ] **PRELOGIN, then § 3's TLS tunnelled inside TDS packets** (0067 § 3). The handshake's records
      are payloads of `PacketType::PreLogin` until the tunnel closes and the socket becomes an
      ordinary TLS stream: `crates/nvs-db/src/tds.rs:636`'s `Wire::send` and
      `crates/nvs-db/src/tds.rs:602`'s `Wire::upgrade` are the two halves, and
      `crates/nvs-db/src/mysql.rs:1428` is the upgrade call site to copy.
- [ ] **LOGIN7, and the packet size the server answers with** (0067 § 3).
      `crates/nvs-db/src/tds.rs:174`'s `TdsTarget::resolve` is the input and
      `crates/nvs-db/src/tds.rs:451`'s `Codec::set_packet_size` is where the negotiated size lands.
      The password is obfuscated rather than proven on this protocol, so it goes out only after the
      slice above.
- [ ] **The token stream, as far as `DONE` and `ERROR`** (0067 § 8). A handshake answer is read
      through `crates/nvs-db/src/tds.rs:699`'s `Wire::read_message`, and SQL Server's § 8 code table
      joins the others beside `crates/nvs-db/src/conn.rs:168`'s `BlockError`.

## Backlog

- `TdsConn` gains a wire and § 1's statement cache — `crates/nvs-db/src/conn.rs:744`, ADR 0132 § 5.
- § 13's `sp_reset_connection` and the cache it empties — the standing check, ADR 0067 § 13.
- ADR 0067 § 9's "sent to the server so `CURRENT_TIMESTAMP` agrees" needs a clause for the one
  driver that cannot send it — docs/adr/0067 § 9.
- `crates/nvs-db/src/lib.rs`'s `pub use` gains `tds`'s names once a caller outside the crate needs
  one; until then the module path is the whole surface.
- `python tools/db-matrix.py`'s postgres leg was not re-run: nothing this session touched a landed
  driver, and no new case has a server half at all.
- Stage 4's § 6 checks filed under `-p nvs-db` are `nvs_stdlib::db`'s — docs/agent/playbook.md.
