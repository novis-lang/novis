# Handoff

## State

**Goal 9 stage 6 is whole.** `Core\Db\Schema`'s three members landed earlier;
`nvs schema plan|apply|dump` (`crates/nvs-cli/src/schema.rs`) is this session's, and stage 4's
`nvs schema dump --connection main` and stage 6's `nvs schema plan --schema examples/schema.json`
both answer against the compose PostgreSQL.

**The command drives a connection through `nvs_db::direct`** — one `match` over the five drivers
that reads every catalog cell as text and runs one statement at a time. It exists because the
command has no `Ctx` and cannot reach the statement path `Core\Db` runs a catalog read through,
and it is in `nvs-db` rather than in the CLI because of `rule:core-classes/db-crate-boundary`.
`nvs-cli` opens all five drivers (`crates/nvs-cli/src/schema.rs:opened`), where `nvs queue
migrate`'s own macro opens the three its statement lists cover.

**Stage 4's dump check named an order no catalog answers** — `nvs_dead_jobs` sorts before
`nvs_jobs` — and its `want` now reads in catalog order, with a comment saying why. Stage 6's
`examples/schema.nvs` was already green.

**What is left in goal 9 is stage 7**, the retirement: `nvs queue migrate` still carries four
hand-written `MIGRATION_*` lists where the schema value now exists to replace them.

## Next group

**Stage 7's retirement** — one file set: `crates/nvs-stdlib/src/queue.rs` and
`crates/nvs-cli/src/queue.rs`, with `docs/agent/loop-goal.toml`'s stage 7 checks beside them.

- [ ] **The queue's schema becomes one `nvs_db::schema::Schema` value** —
      `rule:core-classes/queue-storage-is-a-table`, which ADR 0145 amends: the four
      `MIGRATION_*` lists are one value and the DDL comes from `nvs_db::ddl`, so a backend with
      no hand-written list gains one for free. The three named tests are
      `docs/agent/loop-goal.toml:4634-4638`. Anchors:
      `crates/nvs-stdlib/src/queue.rs:201` (`MIGRATION_POSTGRES`, the list being retired),
      `crates/nvs-stdlib/src/queue.rs:272` (`migration`, whose `Option` the third test says must
      go away), `crates/nvs-stdlib/src/queue.rs:153` (`Migration`, the struct the CLI reads),
      `crates/nvs-db/src/ddl.rs:137` (the `CREATE TABLE` emitter that replaces the text).
- [ ] **`nvs queue migrate` runs that value on every driver** — the check is
      `docs/agent/loop-goal.toml:4640-4645` (`--connection mssql --dry-run`, which today refuses
      because `migration` answers `None`). Anchors: `crates/nvs-cli/src/queue.rs:74`
      (`dialect_of`, whose third refusal stops being possible),
      `crates/nvs-cli/src/queue.rs:258` (`apply`, whose macro can become
      `crates/nvs-cli/src/schema.rs:opened` plus `nvs_db::direct::run`).
- [ ] **Spec § 18 owes `Core\Db\Schema` a table** — the registry carries the cards
      (`rule:core-api/reference-card`) and the spec is where a signature is read.
      `docs/spec/01-core-library.md:1256` is the subsection it goes after, `:1286` is where
      § 19 starts.

## Backlog

- `Core\Db\Schema`'s reference doc at `docs/reference/core/Db/Schema.md` — goal prose stage 6.
- `nvs queue migrate` and `nvs schema` hold two openers for the same five blocks; the second
  slice above is where they become one — `crates/nvs-cli/src/queue.rs`'s module doc.
- `nvs_db::direct` is unit-tested on SQLite alone; the other four arms are covered only by the
  acceptance sweep's live servers — `crates/nvs-db/src/direct.rs`.
- A partial unique index (`nvs_jobs_dedupe`) is filtered out of every introspection, so a
  database holding one plans clean — `rule:core-classes/schema-vocabulary-is-closed`.
