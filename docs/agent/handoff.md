# Handoff

## State

**M8 goal 5, stage 7. SQL Server reads a result set back.**
`crates/nvs-db/src/tds.rs:3226`'s `read_rows` takes a wire with a request already on it and answers a
`TdsRows`: `ROW`, `NBCROW` and `PLP` over `Wire::read_packet`, with the remainder held across the
packet boundary so a token cut at any offset is still one token. Its own doc owns the two things it
holds that `MySqlRows` does not — the buffer, and the reuse of `Tokens` for every token but the two
row ones, which `Tokens::consumed` is what makes possible. A `PLP` value's chunks are copied into
the row as they arrive, so a `varbinary(max)` costs its size once and never lands in the buffer.

**Nothing writes a request yet**: no RPC, no `sp_prepexec`, no cache, no reset, and `TdsConn` still
has no method that runs a statement. The standing acceptance failure
`mssql_resets_through_sp_reset_connection_and_loses_its_cache` is therefore **unchanged and not a
regression** — it needs the whole of the group below, and the playbook's bullet on that check's
`_and_` conjunction still applies.

`RETURNSTATUS` and `RETURNVALUE` are the two tokens a procedure call adds and this reader refuses by
their byte. That is deliberate — only an RPC can produce one, and nothing sends an RPC yet — so the
first slice below adds them where a test can script the shape it will really receive.

## Next group

**One file set: `crates/nvs-db/src/tds.rs`, with `crates/nvs-db/src/conn.rs` and
`crates/nvs-db/src/sql.rs` for the shared state.** `crates/nvs-db/src/mysql.rs:1687`'s `prepare` and
`crates/nvs-db/src/mysql.rs:1872`'s `start_statement` are the shape to copy, and the playbook's
`PgConn` trap is why each of these is a free function generic in the stream.

- [ ] **An RPC out: `sp_prepexec`'s request, § 5's parameters as its arguments, and the two
      procedure tokens that answer it** (0067 §§ 1 and 5). `crates/nvs-db/src/tds.rs:1419`,
      `crates/nvs-db/src/tds.rs:3226`, `crates/nvs-db/src/tds.rs:2733`,
      `crates/nvs-db/src/mysql.rs:1687`. `login7_request` is the message-building shape; the reader
      is already there, so this is the write half plus `RETURNSTATUS`/`RETURNVALUE` in `step` and
      `shape` — a `RETURNVALUE` is a `TYPE_INFO` and a value, which `Tokens::type_info` and
      `TdsRows::value` already measure between them.
- [ ] **§ 1's cache keyed on SQL plus arity, and § 13's reset through `sp_reset_connection`** (0067
      §§ 1 and 13). `crates/nvs-db/src/sql.rs:441`, `crates/nvs-db/src/tds.rs:369`,
      `crates/nvs-db/src/mysql.rs:3958`. The reset is `Status::RESET_CONNECTION` on the next
      message's first packet rather than a round trip of its own, and it drops the server's prepared
      handles — so the cache is cleared with it, which is MySQL's asymmetry and its test's claim.
      This is the slice the standing acceptance check names.
- [ ] **`Driver::SqlServer` runs a statement end to end from `conn.rs`** (0067 §§ 4 and 5).
      `crates/nvs-db/src/conn.rs:772`, `crates/nvs-db/src/tds.rs:3329`,
      `crates/nvs-db/src/mysql.rs:1872`. `TdsConn` gains the two-line delegations over the free
      functions above, and § 4's `may_start_statement` refusal is `pg.rs`'s `second_statement`.

## Backlog

- § 7's `BEGIN`/`COMMIT`/`ROLLBACK` as `PacketType::Transaction`, not as text — `docs/adr/0067-core-db.md` § 7.
- `nvs-stdlib`'s decode of a `TdsRow`'s bytes into § 9's values, against `TdsColumn::type_info`.
- A real SQL Server in `tests/db/compose.yaml`'s matrix, per ADR 0067's *Verification*.
- `crates/nvs-db/src/tds.rs` is past 5,000 lines; the split point is the framing half against the token half.
