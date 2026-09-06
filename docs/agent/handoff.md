# Handoff

## State

**Goal 9 — `Core\Db\Schema` — has its ADR and its vocabulary.**
[ADR 0145](../adr/0145-a-schema-is-a-value-core-db-schema-converges-a-closed.md) is stage 2's first
slice and this goal's only design act: it closes ADR 0067's *Revisiting* item, takes over ADR 0084
§ 2's schema, adds a third deny-by-default capability (`db.schema`) to ADR 0067 § 3, and makes § 9's
type map a table read in both directions. `Web\Migration` stays blocked and no session may close ADR
0082 § 7.

`crates/nvs-db/src/schema.rs` is stage 2's second slice: `Schema`/`Table`/`Column`/`Key`,
`ScalarType` as § 9's write direction, a closed `ColumnDefault` set, and every construction rule
refused at build time so no emitter re-checks. Eight `-p nvs-db` tests, including the queue's jobs
table said in the vocabulary. `is_bare_identifier` moved here from `nvs-stdlib` and
`Core\Db::quoteIdentifier` now calls it, so the identifier judgement has one home.

**Two decisions the implementation forced, both folded into the ADR**: a column carries an
**identity** flag (§ 2 — every backend has one, three shared rules: integer, one per table, in the
primary key), and an index is **never unique** (a unique constraint is the canonical spelling, so one
schema is not expressible two ways).

## Next group

**Stage 2's tail and stage 3's head** — one file set: `crates/nvs-db/src/schema.rs`, beside
`crates/nvs-db/src/sql.rs:72`'s four-valued `Dialect`.

- [ ] **The canonical array form, and the round trip.** ADR 0145 § 1: the array form is canonical and
      **ordered** — declaration order for columns, name order for everything else — and
      `to_array(from_array(a)) == a` is the property over every construct. `nvs-db` is sans-io and
      holds no Novis value, so this is a small ordered-node form here and one conversion in
      `nvs-stdlib` at stage 6, never two serializations. Anchors:
      `crates/nvs-db/src/schema.rs:164` (`ScalarType`), `crates/nvs-db/src/schema.rs:271`
      (`ColumnDefault`), `crates/nvs-db/src/schema.rs:432` (`Key`),
      `crates/nvs-db/src/schema.rs:454` (`Table`), `crates/nvs-db/src/schema.rs:622` (`Schema`).
- [ ] **`CREATE TABLE` in all four dialects**, one named test per construct — goal stage 3.2 and ADR
      0145 § 8. Emitters follow `Dialect` and not `Driver`; the traps are transcription from
      `crates/nvs-stdlib/src/queue.rs:296` (no `create index if not exists` on MySQL, no partial
      index at all, no `text` index without a prefix length). Anchors:
      `crates/nvs-db/src/sql.rs:72`, `crates/nvs-db/src/schema.rs:164`.
- [ ] **A step, its grade and its reason.** ADR 0145 § 6: three grades, keyed on driver *and* server
      version, and an emitter that does not know **grades up**; SQLite's non-additive alters are the
      create-copy-drop-rename rebuild, `Destructive` unconditionally. Anchors:
      `crates/nvs-db/src/schema.rs:622`, `crates/nvs-db/src/sql.rs:72`.

## Backlog

- The queue's PostgreSQL dedupe index is **partial** (`where state = 0`) and § 11 keeps partial
  indexes out of v1 — stage 7's retirement owes it one spelling both dialects can hold. ADR 0145,
  *Consequences*.
- Stage 4's five introspectors and `nvs schema dump`; the diff and its normalization is stage 5.
- Stage 6 owes: `Core\Db\Schema` in `nvs-stdlib`, `docs/spec/01-core-library.md` § 18's rows,
  `docs/reference/core/Db/Schema.md`, and the `db.schema` capability in `nvs-config`.
- `[context] modules`' `crates/nvs-db/src/schema.rs` pattern matched nothing when this session
  opened; the file now exists, so `orient.py`'s warning is spent.
