# Handoff

## State

**ADR 0067 § 9's whole type map is on disk for PostgreSQL**, and the seam the structured rows
needed is the shape of it. `crates/nvs-db/src/pg.rs`'s `PgScalar` is now this crate's public answer
beside `Value`: `PgColumn::scalar` decodes every row of the table, and `PgColumn::decode` is the
scalar half — it answers `Ok(None)` for `DATE`, `TIME`, `TIMESTAMP`, `TIMESTAMPTZ` and `UUID`,
which is a different absence from SQL `NULL`'s `Some(null)` and is pinned by a test that asserts
both on one column. Those five arrive as parsed components (`PgDate`, `PgTime`, and seconds east of
UTC), because a `Core\Time` or `Core\Uuid` instance needs `nvs-stdlib`'s class descriptors and
`crates/nvs-db/Cargo.toml` states on both manifests why that edge does not exist. **`nvs-stdlib` is
what finishes them**, by matching those five variants and calling `into_value` for the rest.

**Two startup parameters are now part of the decode**, not decoration: `DateStyle = ISO` fixes the
rendering the parsers read positionally, and `TimeZone` — `PgTarget::time_zone`, seconds east of
UTC — fixes the offset a `TIMESTAMPTZ` carries and is § 9's declared zone for the zone-less
`TIMESTAMP`. The sign is the trap and `posix_time_zone`'s doc owns it: PostgreSQL's numeric zone is
a POSIX one, so two hours east is `<+02>-02`, and a bare `+02:00` is not a zone the server parses
at all.

**§ 4's affected count is landed too**: `PgRows::affected` over the free `affected_rows`, which
matches the command word rather than the tag's shape — `INSERT 0 3`'s first number is an OID, not a
count and not a `lastId`. A tag with no count answers `None` rather than `0`.

Unchanged: `statement_cache` still has no reader, nothing can handshake against
`tests/db/compose.yaml` (self-signed, no anchor seam in `nvs_host::tls`), and the goal's three
fixtures are red at `E0405` because `Core\Db\Connection` has no stdlib rows — this goal's ordinary
state and what the driver's acceptance check reports.

**Manifest gap, open five sessions now:** `[context] adrs` in `docs/agent/loop-goal.toml` names no
section of ADR 0132, and none of ADR 0067 §§ 1, 4, 5. Add 0132 §§ 1-5 and 0067 §§ 1, 4, 5 — § 4 was
sliced by hand this session for the affected-row count.

## Next group

**The config block's two missing fields, and what reads them** — `crates/nvs-config/src/tree.rs`
and `crates/nvs-db/src/sql.rs`, with one field of `crates/nvs-db/src/pg.rs` in the second. The
first two share `Database`'s field list and its boot refusal.

- [ ] **§ 1's `statement_cache` gets its config field and its reader** — `Database` gains it at
      `crates/nvs-config/src/tree.rs:511`, and `crates/nvs-db/src/sql.rs:224`'s `StatementCache`
      takes the size from there instead of whatever the constructor is defaulted with today. ADR
      0067 § 1 is the sizing rule; the field list's doc comment above that struct says a field
      the ADR meant and the list omits is a boot refusal, so this closes one.
- [ ] **§ 9's `time_zone` gets its config field** — the same `Database` at
      `crates/nvs-config/src/tree.rs:511`, feeding `crates/nvs-db/src/pg.rs:153`'s
      `PgTarget::time_zone`, which is already read and already sent. Default UTC. § 9's own
      sentence names `timeZone` in `Settings` as the other spelling.
- [ ] **PostgreSQL arrays, § 9's last undecoded row** — `crates/nvs-db/src/pg.rs:1010`'s `scalar`
      falls through to `PgScalar::Text` for them today. The element OID is in the row description
      and `array<T>` is a `Value`, so this one stays inside this crate, unlike the five above.

## Backlog

- The pool (§ 13) is Stages 3 to 7 — `docs/plan/m8.md`.
- No anchor seam in `nvs_host::tls`, so no compose server can be handshaked against — ADR 0132 § 3.
- MySQL, MariaDB, SQL Server and SQLite drivers are all still `conn.rs` variants with no wire.
- `Core\Db\Connection`'s stdlib rows are what close the three red fixtures — ADR 0067 § 4.
- `Db\Write::lastId` on PostgreSQL comes from `RETURNING`, not the tag — ADR 0067 § 4.
