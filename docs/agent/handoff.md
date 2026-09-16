# Handoff

## State

**Goal `unowned-closures`. The register is `unowned: 15`** (`python tools/owners.py`), `--deferrals`
green. The 15 unowned are the scheduling questions and none of them is this goal's own gap.

**Every identifier `nvs_db::ddl` writes is delimited for its dialect.**
`crates/nvs-db/src/ddl.rs:568` (`delimited`) is the one place, and every emitter goes through it:
`"x"` on PostgreSQL and SQLite, backticks on MySQL, `[x]` on SQL Server. Nothing is escaped on the
way through because `is_bare_identifier` admits no delimiter byte, which is what makes the quoting
total rather than best effort. `catalog`'s round-trip fixture names its int column `rank` — MySQL 8
reserves it — so the walk over all five servers is what asks whether the emitter quoted.

**What nvs-db still owes**: `crates/nvs-db/src/schema.rs:53` gap 1, now only its index half, and
`crates/nvs-db/src/ddl.rs:87` gap 1. Both are owned by this goal and both are the next group.

**The index refusal has a read-path question the `Decided:` sentence does not answer.**
`crates/nvs-db/src/catalog.rs:1018` builds every key it reads back through `Table::index`, so a
builder that refuses an index over unbounded text also refuses to *introspect* a database that has
one, and two fixtures carry exactly that (`crates/nvs-db/src/ddl.rs:1185`,
`crates/nvs-db/src/catalog.rs:1261`). MySQL's prefix key (`crates/nvs-db/src/ddl.rs:112`) loses its
last caller the moment no `Table` can hold such an index. Refusing at the builder and keeping the
read path total — the catalog dropping the key it cannot express — is the shape that does not break
introspection; whichever way it goes, say so in `schema`'s module doc.

## Next group

**Stage 5: the vocabulary's last portability gap, then the emitter's** — one file set:
`crates/nvs-db/src/schema.rs`, `crates/nvs-db/src/ddl.rs`, `crates/nvs-db/src/catalog.rs`.
`rule:core-classes/schema-is-a-value` is what the vocabulary sits inside and
`rule:core-classes/schema-plan` what the emitter does.

- [ ] **Refuse an index over unbounded text in the builder** — the remaining half of
      `crates/nvs-db/src/schema.rs:53`'s `Decided:` sentence: SQL Server refuses `NVARCHAR(MAX)` as a
      key column outright and MySQL takes it only as a prefix key, so there is no portable spelling
      to emit instead. `crates/nvs-db/src/schema.rs:612` (`index`) and
      `crates/nvs-db/src/schema.rs:598` (`unique`) are where a key's columns are resolved and
      `crates/nvs-db/src/schema.rs:1133` (`SchemaError`) is the enum that needs the case. Decide the
      read path first — `## State` above names what it costs — then move the two fixtures off their
      unbounded column and settle what happens to `crates/nvs-db/src/ddl.rs:112`'s prefix key.
- [ ] **Look a SQL Server default constraint's name up in the emitted batch** —
      `crates/nvs-db/src/ddl.rs:87`'s gap 1 is the `Decided:` sentence: `ALTER COLUMN` there carries a
      type and a nullability and nothing else, so a default change needs the generated constraint
      name, which neither `Change` nor the catalog carries. `crates/nvs-db/src/ddl.rs:687`
      (`change_column`) is the emitter, and the step becomes dynamic SQL over
      `sys.default_constraints` — the one place this module writes a statement that is not a literal
      an operator could paste, so its doc has to say so.

## Backlog

- `crates/nvs-db/src/catalog.rs` reads a bare spelling on a *non*-text column as a parse failure,
  not as an opaque default; the two disagree by design, and `column_default`'s own doc argues it.
- `crates/nvs-cli/src/cache.rs:166` gaps 1 and 2 — `aarch64` and Mach-O's leading underscore, owned
  by this goal and a different file set.
- The unowned 15 are scheduling questions, indexed in `docs/agent/carried-gaps.md` § *Unowned*.
