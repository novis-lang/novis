# Handoff

## State

**ADR 0067 § 9's scalar rows are on disk for PostgreSQL.** `crates/nvs-db/src/pg.rs`'s
`PgColumn::decode` reads the row description's OID and type modifier and answers with
`nvs-runtime`'s value: int, uint, `decimal`, float, bool, string, bytes, and `null` for an absent
body, which is what makes every column `?T`. `PgColumn` gained `type_modifier` because § 9's
`BIT(1)` row is a `bool` and `BIT(8)` is a `tainted string` and one OID carries both. Everything
without a Novis type of its own — `json`, `inet`, `interval`, and for now § 9's structured rows —
arrives at the table's last row as the server's own rendering.

**Three decisions recorded in that module rather than in an ADR**, because § 9 states the table and
not how a text body is read. **A text body is checked to be UTF-8** before it becomes a `string`:
`client_encoding` makes that true of a *correct* server, ADR 0009's promise is read unchecked in
`nvs-runtime`, and a database is a network peer. **A decode error never quotes the body**, holding
the line `PgRow`'s and `Wire`'s `Debug` implementations already hold. And **`money` is read
positionally** — the last separator is the decimal point when one or two digits follow it — which is
exact everywhere except the three-digit currencies, where the text carries no signal to break the
tie and the column must be cast.

**§ 9's structured rows cannot be done the way the last handoff scoped them.** `Core\Time\Date`,
`TimeOfDay`, `Instant`, `DateTime` and `Core\Uuid` are `nvs-stdlib` classes: an instance needs that
crate's `ClassDesc`, and `crates/nvs-db/Cargo.toml` states on both manifests that an edge to
`nvs-stdlib` is a workspace cycle. So a driver cannot answer with one, and the next group's first
item is the seam rather than the rows — the recommendation is in it.

Unchanged: `statement_cache` still has no reader, nothing can handshake against
`tests/db/compose.yaml` (self-signed, no anchor seam in `nvs_host::tls`), and the goal's three
fixtures are red at `E0405` because `Core\Db\Connection` has no stdlib rows — this goal's ordinary
state and what the driver's acceptance check reports.

**Manifest gap, open four sessions now:** `[context] adrs` in `docs/agent/loop-goal.toml` names no
section of ADR 0132, and none of ADR 0067 §§ 1 or 5. Add 0132 §§ 1-5 and 0067 §§ 1, 5.

## Next group

**The rest of the type map and what pays for it** — `crates/nvs-db/src/pg.rs` throughout, with one
line of `crates/nvs-config/src/tree.rs` in the last item. All three read the two structs the first
one changes.

- [ ] **§ 9's structured rows, and the seam they need** — `crates/nvs-db/src/pg.rs:736`'s `PgScalar`
      gains `Date`, `Time`, `Timestamp`, `Instant` and `Uuid` variants holding *parsed components*
      (civil fields, seconds, nanos, a UTC offset), and becomes this driver's public answer beside
      `Value`: `crates/nvs-db/src/pg.rs:800`'s `decode` keeps the scalar half and `scalar` is what
      `nvs-stdlib` calls for the rest, since only that crate can `construct` a `Core\Time` instance.
      OIDs go in `crates/nvs-db/src/pg.rs:698`'s table (`date` 1082, `time` 1083, `timestamp` 1114,
      `timestamptz` 1184, `uuid` 2950). § 9's zone-less-`DATETIME` half belongs here too: the
      `TimeZone` startup parameter at `crates/nvs-db/src/pg.rs:464`, sent as a numeric offset.
      `crates/nvs-db/Cargo.toml:19`'s "a driver answers with `nvs-runtime`'s values" is the comment
      this amends.
- [ ] **§ 4's affected-row count** — `crates/nvs-db/src/pg.rs:1048`'s `command_tag` is the raw tag
      and its own doc says parsing it is this slice's job: `INSERT 0 3`, `UPDATE 2`, `SELECT 0`, and
      a tag with no count (`BEGIN`, `SET`) answering `None` rather than nought.
- [ ] **`statement_cache` gets its reader** — § 1's config field, `crates/nvs-config/src/tree.rs:511`
      as an `Option<u32>` beside its siblings, resolved into `crates/nvs-db/src/pg.rs:307`'s
      `connect` where `StatementCache::DEFAULT_CAPACITY` currently stands alone.

## Backlog

- The pool itself, ADR 0067 § 13 — plan Stages 3 to 7, and `reset` already takes `self` for it.
- MySQL and MariaDB as their own drivers, ADR 0132 § 5's enum.
- `Core\Db`'s stdlib rows, which is what turns the fixtures' `E0405` green — ADR 0067 §§ 2, 4.
- An anchor seam in `nvs_host::tls` so `tests/db/compose.yaml`'s servers are reachable at all.
- `executeMany`, ADR 0067 § 1's answer to the batch case.
