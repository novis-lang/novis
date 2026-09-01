# Handoff

## State

**ADR 0067 § 18's two connectionless entry points are on disk in `nvs-stdlib`.**
`crates/nvs-stdlib/src/db.rs` is the new module: `Core\Db` with `inList` and `quoteIdentifier`,
and `Core\Db\InList` as a memberless carrier over one `values` slot. Its module doc is the home
of the two decisions this side of the boundary owns — **`inList` holds the array and computes
nothing**, because the markers are `$1` on PostgreSQL and `@p1` on SQL Server so only a driver
knows what a run expands to (`nvs_db::sql`'s `expand`), and **`quoteIdentifier` validates a bare
ASCII identifier and adds no delimiter**, because `Core\Db` has no connection and so no dialect,
and on MySQL a double-quoted identifier is a string literal rather than a name. A name that would
need delimiting is refused, not mangled.

**The acceptance check is still red at `E0405` on `Core\Db\Connection::query`**, which is the
expected state: every remaining § 18 member needs a connection, and nothing resolves a
`[db.<name>]` block into a `PgTarget` yet. So `nvs_db::sql::time_zone_for`'s `None` still has no
boot refusal, and nothing can handshake against `tests/db/compose.yaml` (self-signed, no anchor
seam in `nvs_host::tls`). `Core\Db` carries no `registry::CAPABILITIES` row yet either — the two
members that landed reach nothing, and `db.connect`/`db.open` arrive with the members that need
them.

**Manifest gaps, in `docs/agent/loop-goal.toml`:** `[context] adrs` still names none of ADR 0067
§§ 2, 3, 4, 7, 8 and no section of ADR 0132; `[context]` still reaches nothing under `docs/spec/`,
so § 18's class tables come in only as the three lines around an anchor. `[context] modules` does
not name `nvs-config`, whose `capability.rs` holds the `DbConnect`/`DbOpen` spellings the next
group needs.

## Next group

**The connection, which is what the acceptance check is failing on** — the file set is
`crates/nvs-db/src/pg.rs` and `crates/nvs-db/src/sql.rs`'s `[db.*]` readers on the wire side, and
`crates/nvs-stdlib/src/db.rs` plus its capability row on the stdlib side.

- [ ] **A `[db.<name>]` block resolved into a `PgTarget`, with a boot refusal for what it cannot
      read** — ADR 0067 § 3 for the block's fields, § 9 for `time_zone`. The struct is
      `crates/nvs-db/src/pg.rs:154` and the reader beside it is
      `crates/nvs-db/src/sql.rs:742`; the borrowed `'a` is the thing to decide first, since a
      resolved target has to outlive the call that opens with it.
- [ ] **`Core\Db::connect` and `open` over it** — `docs/spec/01-core-library.md:1144` for the two
      signatures and ADR 0067 § 2 for the memoization key. The rows go above `inList` at
      `crates/nvs-stdlib/src/db.rs:97`, the capability rows at
      `crates/nvs-stdlib/src/registry.rs:1440` (`nvs_config::Cap::DbConnect` and `DbOpen`,
      `crates/nvs-config/src/capability.rs:176`), and `§18 connect`/`§18 open` come off
      `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:57`.
- [ ] **`Core\Db\Connection` and `Queryable::query`, which is the member the check names** —
      `docs/spec/01-core-library.md:1156`. The instance rows are a second `CoreClass` beside
      `crates/nvs-stdlib/src/db.rs:127`, over `crates/nvs-db/src/conn.rs:325`'s `PgConn`; the
      playbook's `-p nvs-db` bullet is why the sequencing behind it is a free function generic in
      the stream.

## Backlog

- `Core\Db\Rows`/`Write` and the `queryAs<T>` half — `docs/spec/01-core-library.md` § 18.
- `transaction` as a closure over `nvs_db::pg`'s `begin`/`commit`/`roll_back` — ADR 0067 § 7.
- The per-core pool and its reset — ADR 0067 § 13, Stages 3 to 7 of the goal.
- A usable certificate for `tests/db/compose.yaml`, which needs an anchor seam in
  `nvs_host::tls` — `docs/agent/handoff.md`'s predecessors have carried this since the handshake
  landed.
- The four remaining drivers, each its own `nvs_db::conn::Connection` arm — ADR 0132.
- `docs/novis.md`'s anchors collide when a class and a member differ only in case: `Core\Db\InList`
  and `Core\Db::inList` both render `#core-core-db-inlist`. The generator's, not the registry's —
  ADR 0117 and whatever in `nvs-cli` builds the page.
