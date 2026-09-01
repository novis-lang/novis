# Handoff

## State

**The SQL layer over the state machine is on disk and green.** `crates/nvs-db/src/sql.rs` is the
shared half ADR 0132 § 5 keeps as plain data with no wire in it: ADR 0067 § 5's `?`/`:name` rewriter
over four dialects with `inList` expansion, and § 1's LRU statement cache keyed by SQL text plus that
expansion's arity. `crates/nvs-db/src/pg.rs` is the first driver to spend it — a hit sends no
`Parse`, an eviction's `Close` rides in the batch that replaced it under the same `Sync`, and the
cache is written only once `ParseComplete` has proved the server holds the statement.

**Two calls recorded in that module's doc rather than in an ADR**, because § 5 states the rule and
not the scanner. **Quoting and escaping are dialect properties** — MySQL's backslash escape,
PostgreSQL's dollar quoting and nested block comments, SQL Server's brackets — because getting a
literal's end wrong rewrites a `?` that was never a placeholder, and that is the one way this
module can produce a query nobody wrote. And **malformed SQL is the server's diagnosis**: an
unterminated quote ends the scan at the end of the text and the statement goes out to be refused by
a parser that can say what is actually wrong with it.

**`statement_cache` has no reader**, and it is the one thing § 1 asks for that is not yet true.
Nothing in this crate opens a connection *from* config, so the capacity is a `new()` parameter and
`StatementCache::DEFAULT_CAPACITY` (16) is what `PgConn::connect` passes until the connect path
reads one.

Unchanged: nothing can handshake against `tests/db/compose.yaml` (self-signed, no anchor seam in
`nvs_host::tls`), and the goal's three fixtures are red at `E0405` because `Core\Db\Connection` has
no stdlib rows yet — this goal's ordinary state, and what the driver's last acceptance check
reported.

**Manifest gap, open three sessions now:** `[context] adrs` in `docs/agent/loop-goal.toml` names no
section of ADR 0132, and none of ADR 0067 §§ 1 or 5 — § 5 is the specification of the whole slice
above and was sliced out of the ADR by hand again. Add 0132 §§ 1-5 and 0067 §§ 1, 5.

## Next group

**ADR 0067 § 9's type map, the PostgreSQL half** — `crates/nvs-db/src/pg.rs` and `nvs-runtime`'s
values, nothing else in the crate. Both halves read the same two structs, and § 9's table is the
specification for each row of them.

- [ ] **§ 9's scalar rows** — `crates/nvs-db/src/pg.rs:616`'s `PgColumn::type_oid` to a Novis type,
      decoding the text-format bodies `crates/nvs-db/src/pg.rs:629`'s `PgRow` hands back:
      int/uint, `decimal`, float, bool, `tainted string`, `tainted bytes`, and `NULL` as `?T`.
      `crates/nvs-db/src/pg.rs:1055`'s `columns_of` is where the OID arrives.
- [ ] **§ 9's structured rows** — `Core\Time\Date`, `TimeOfDay`, `Instant` for `TIMESTAMPTZ`,
      `DateTime` in the connection's declared zone, `Core\Uuid`, arrays as `array<T>`, and the
      `tainted string` catch-all § 9 gives `interval`/`hstore`/ranges/`inet` *as the server renders
      them*. Same two anchors: `crates/nvs-db/src/pg.rs:616` and `crates/nvs-db/src/pg.rs:629`.
- [ ] **`statement_cache` gets its reader** — § 1's config field displacing
      `crates/nvs-db/src/sql.rs:224`'s `DEFAULT_CAPACITY` at the connect path. Small, and the last
      § 1 obligation open.

## Backlog

- The four remaining drivers, PostgreSQL-first having done its job — ADR 0067 § 3.
- The per-core pool and its acquire path, over the reset already on disk — ADR 0067 § 13.
- `Core\Db\Connection`'s stdlib rows, which is what the fixtures' `E0405` is — docs/spec/01-core-library.md.
- The `nvs.toml` anchor bundle; without it no driver reaches `tests/db/compose.yaml` — ADR 0132 § 3.
- `[context] adrs` gains ADR 0132 §§ 1-5 and ADR 0067 §§ 1, 5 — docs/agent/loop-goal.toml.
- A doc-cleanup pass, fired by the user and never automatically — docs/agent/doc-cleanup.md.
