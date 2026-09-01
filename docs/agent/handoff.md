# Handoff

## State

**ADR 0067's two operator-written numbers are on disk, field and reader.** `[db.<name>]` gains
`statement_cache` (§ 1) and `time_zone` (§ 9) on `crates/nvs-config/src/tree.rs`'s `Database`, and
both readers are `crates/nvs-db/src/sql.rs`, beside `StatementCache` and for the same reason: the
answer is the same on all five drivers and has no wire in it. `StatementCache::capacity_for`
answers `DEFAULT_CAPACITY` for a block that omits the field and honours a written `0` as the cache
turned off; `time_zone_for` answers seconds east of UTC, `Some(0)` for a block naming none, and
**`None` — not the default — for a value that is not an offset** (`Europe/Vienna`, the compact
`+0200`, a field past 59, a magnitude past 18 hours). Each function's doc comment is the whole rule.

**A driver takes the answer, never the block.** `PgTarget` now carries `statement_cache` beside
`time_zone`, both already numbers, so `PgConn::connect` sizes its cache from the target instead of
naming `DEFAULT_CAPACITY` and no connect path reaches into a config tree. That is also what keeps
both readers testable with neither a socket nor a configuration file.

**The seam this leaves open is the caller.** Nothing resolves a `[db.<name>]` block into a
`PgTarget` yet, so `time_zone_for`'s `None` has no boot refusal naming the line — that belongs to
`nvs-config`'s resolver, which owns every other boot error, and the function's doc says so.

Unchanged and pointed at rather than restated: § 9's type map and § 4's affected count are
`crates/nvs-db/src/pg.rs`'s module doc and `PgScalar`'s own; the five structured rows stop at
parsed components for the reason both `crates/nvs-db/Cargo.toml` manifests give. Nothing can
handshake against `tests/db/compose.yaml` (self-signed, no anchor seam in `nvs_host::tls`), and the
goal's three fixtures are red at `E0405` because `Core\Db\Connection` has no stdlib rows — this
goal's ordinary state and what the driver's acceptance check reports.

**Manifest gaps, in `docs/agent/loop-goal.toml`:** `[context] adrs` still names no section of ADR
0132 (add §§ 1-5) and none of ADR 0067 §§ 2, 4, 5 — 0067 §§ 1, 9 and 13 did arrive this session and
were enough for this group. `[context]` also reaches nothing under `docs/reference/`, which is the
second home of every config field (see the playbook bullet added this session).

## Next group

**§ 9's last undecoded row and § 4's `lastId`, both inside one file** — the file set is
`crates/nvs-db/src/pg.rs` alone, and both slices sit in its decode half.

- [ ] **PostgreSQL arrays, § 9's last undecoded row** — `crates/nvs-db/src/pg.rs:1026`'s `scalar`
      falls through to `PgScalar::Text` for them today. The element OID is on
      `crates/nvs-db/src/pg.rs:702`'s `PgColumn` from the row description and `array<T>` is a
      `Value`, so this stays inside this crate: a new variant on
      `crates/nvs-db/src/pg.rs:883`'s `PgScalar` is not needed if the element decodes through
      `crates/nvs-db/src/pg.rs:1010`'s `decode`.
- [ ] **`Db\Write::lastId` on PostgreSQL comes from `RETURNING`, not the tag** — ADR 0067 § 4.
      `crates/nvs-db/src/pg.rs:1544`'s `affected` already refuses to read `INSERT 0 3`'s first
      number as an id, so the id is a row `crates/nvs-db/src/pg.rs:1462`'s `PgRows` has to hand
      back rather than a field of the tag.

## Backlog

- Nothing builds a `PgTarget` from a `[db.<name>]` block — the two new readers have no caller, and
  `db.connect`'s capability check is the same seam (ADR 0067 §§ 2-3).
- The pool (§ 13) is Stages 3 to 7 — `docs/plan/m8.md`.
- No anchor seam in `nvs_host::tls`, so no compose server can be handshaked against — ADR 0132 § 3.
- MySQL, MariaDB, SQL Server and SQLite drivers are all still `conn.rs` variants with no wire.
- `Core\Db\Connection`'s stdlib rows are what close the three red fixtures — ADR 0067 § 4.
