# Handoff

## State

**ADR 0067 § 9's type map is complete on PostgreSQL, arrays included.** An array is not a row of
its own: `crates/nvs-db/src/pg.rs`'s `PgScalar::Array` holds the elements, each decoded through
the same table, so a multi-dimensional array is elements that are themselves arrays and a `text[]`
holds what a `text` column holds. `oid::element` is the array-to-element table — that OID is
`pg_type.typelem`, catalog data the row description does not carry — and an array it cannot name
stays whole at § 9's last row. The functions' own doc comments are the rule for both.

**Two consequences worth knowing before touching the decoder.** `PgScalar::Text` is a
`Cow<'_, str>`, because an element carrying a `\` escape cannot be unescaped in place;
`PgScalar::into_owned` is the only place the owned case is made. And `into_value` asks
`is_value` first, before allocating anything: an `array<Core\Time\Date>` is `nvs-stdlib`'s to
finish whole, at any nesting, and a half-built array is one this crate cannot free.

**§ 4's `lastId` is a `RETURNING` row's, never the tag** — the last returned row's first column
where the statement declared it an integer. `INSERT 0 3`'s first number is an OID and is `0` on
every server since PostgreSQL 12. It is captured in `next_row` as the row goes past, because a
`PgRow` borrows the wire's buffer and there is nothing to read it out of afterwards.

Unchanged and pointed at rather than restated: the `[db.<name>]` readers are
`crates/nvs-db/src/sql.rs` beside `StatementCache`, and nothing resolves a block into a
`PgTarget` yet, so `time_zone_for`'s `None` still has no boot refusal. Nothing can handshake
against `tests/db/compose.yaml` (self-signed, no anchor seam in `nvs_host::tls`), and the goal's
three fixtures stay red at `E0405` because `Core\Db\Connection` has no stdlib rows — this goal's
ordinary state and what the driver's acceptance check reports every iteration.

**Manifest gaps, in `docs/agent/loop-goal.toml`:** `[context] adrs` still names no section of ADR
0132 (add §§ 1-5) and none of ADR 0067 §§ 2, 5 — § 4 was needed this session and read by hand, so
add it. `lastId`'s *type* is not in ADR 0067 at all (§ 4's table defers to § 7, which is
transactions): it is `?uint` in `docs/spec/01-core-library.md`'s § 6 class table, and `[context]`
reaches nothing under `docs/spec/`, which is the second gap.

## Next group

**§ 4's remaining two rules, both inside one file** — the file set is `crates/nvs-db/src/pg.rs`
alone, and both sit in its statement-sequencing half rather than the decode half just finished.

- [ ] **`executeMany` is one prepare and N executions** — ADR 0067 § 4's fourth row, and on
      PostgreSQL that is one `Parse` and N `Bind`/`Execute` pairs in a single flush, answering the
      summed affected count. Generalise `crates/nvs-db/src/pg.rs:2044`'s `start_statement`, which
      already writes the one-execution batch, and hang the two-line delegation beside
      `crates/nvs-db/src/pg.rs:386`'s `query`. An empty set list is a no-op returning `0`, which
      § 4 states outright.
- [ ] **A statement attempted while a stream is live names both fixes** — § 4 requires the
      refusal to say `->all()` *or* a `{shared: false}` connection.
      `crates/nvs-db/src/pg.rs:2044` already refuses on `State::Streaming`; check what its message
      says, and if it already names both, the slice is the test that pins it.

## Backlog

- § 13's per-core pool, the reset that is a security boundary, and `pool = false` — ADR 0067 § 13,
  over `crates/nvs-db/src/conn.rs`; `reset_session` is already written and takes `self`.
- `Core\Db\Connection`'s stdlib rows, which is what the goal's three fixtures need — the class
  table in `docs/spec/01-core-library.md` § 6.
- The other four drivers, each its own state machine over the shared wire shape — ADR 0132 § 2.
- Resolving a `[db.<name>]` block into a `PgTarget`, with the boot refusal `time_zone_for`'s
  `None` is waiting for — `nvs-config`'s resolver owns every other boot error.
