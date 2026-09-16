# Handoff

## State

**Goal `unowned-closures`. The register is `unowned: 15`, goal-owned is down to 47 over 88 items**
(`python tools/owners.py`), `--deferrals` green. The 15 unowned are the scheduling questions and none
of them is this goal's own gap.

**`ColumnDefault::Opaque` is built, and it is read-only in two separate halves.** Nothing emits it —
`crates/nvs-db/src/ddl.rs:510`'s arm is an `unreachable!`, because a step emits the *wanted* schema's
default and a wanted schema came through `ColumnDefault::from_node`, which refuses the one key
`to_node` writes. Nothing outside a catalog read builds one. `fits` takes it on every type on purpose:
that question is whether a literal may be *written* on every backend, and this case is never written.

**A bare text default is the server's words on every dialect but MySQL.** `unquote` now reports
whether the server quoted, and `crates/nvs-db/src/catalog.rs:@column_default` reads that flag on the
one type that takes whatever it is handed. Every other type validates its own text by parsing it, so
an expression fails without the quoting being consulted, and the refusals that function already made
are unchanged. `crates/nvs-db/src/catalog.rs`'s `# Known gaps` block is gone with its last item.

**What nvs-db still owes**: `crates/nvs-db/src/ddl.rs:84` gap 1 and `crates/nvs-db/src/schema.rs:53`
gap 1, both owned by this goal and both in the next group.

## Next group

**Stage 5: the vocabulary's two portability gaps** — one file set: `crates/nvs-db/src/ddl.rs`,
`crates/nvs-db/src/schema.rs`. Items 1 and 2 are the two halves of one `Decided:` sentence;
`rule:core-classes/schema-is-a-value` is what the vocabulary sits inside and
`rule:core-classes/schema-plan` what the emitter does.

- [ ] **Delimit every identifier the emitter writes** — `crates/nvs-db/src/schema.rs:53`'s gap 1 is
      the `Decided:` sentence, and `RANK` on MySQL 8 is the case it names: the vocabulary validates
      an identifier rather than delimiting it, so a reserved word is a `CREATE TABLE` the server will
      not parse. `crates/nvs-db/src/ddl.rs:404` (`column_clause`) and
      `crates/nvs-db/src/ddl.rs:137` (`create_table`) write a bare `name()`; the per-dialect
      delimiter is the same judgement `crates/nvs-db/src/ddl.rs:553` (`quoted`) already makes for a
      text literal, and the module doc names `Core\Db::quoteIdentifier` as where it came from. The
      catalog reads names back undelimited, so nothing on that side moves.
- [ ] **Refuse an index over unbounded text in the builder** — the second half of the same
      `Decided:` sentence: `crates/nvs-db/src/schema.rs:613` (`index`) and
      `crates/nvs-db/src/schema.rs:599` (`unique`) are where a key's columns are checked, and there
      is no portable spelling to emit instead — SQL Server refuses `NVARCHAR(MAX)` as a key column
      outright and MySQL takes it only as a prefix key. A new `SchemaError` case is what the refusal
      needs; `crates/nvs-db/src/schema.rs:@SchemaError` is the enum.
- [ ] **Look a SQL Server default constraint's name up in the emitted batch** —
      `crates/nvs-db/src/ddl.rs:84`'s gap 1 is the `Decided:` sentence: `ALTER COLUMN` there carries
      a type and a nullability and nothing else, so a default change needs the generated constraint
      name, which neither `Change` nor the catalog carries. `crates/nvs-db/src/ddl.rs:654`
      (`change_column`) is the emitter, and the step becomes dynamic SQL over
      `sys.default_constraints` — the one place this module writes a statement that is not a literal
      an operator could paste, so its doc has to say so.

## Backlog

- `crates/nvs-db/src/catalog.rs` reads a bare spelling on a *non*-text column as a parse failure,
  not as an opaque default; the two disagree by design, and `column_default`'s own doc argues it.
- `crates/nvs-cli/src/cache.rs:166` gaps 1 and 2 — `aarch64` and Mach-O's leading underscore, owned
  by this goal and a different file set.
- The unowned 15 are scheduling questions, indexed in `docs/agent/carried-gaps.md` § *Unowned*.
