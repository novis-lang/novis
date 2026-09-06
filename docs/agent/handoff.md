# Handoff

## State

**Goal 9 stages 5 and 6 are whole.** § 5's acceptance criterion — apply a schema, introspect it back,
and the plan between the two is empty — now has a case per backend
(`crates/nvs-db/src/catalog.rs:1990`, one handshake each), and `python tools/db-matrix.py --all` is
**5/5**. The cases skip without `NVS_DB_MATRIX_DRIVER`, so `verify.py` is unchanged on a machine with
no containers.

**Two real defects stood between the property and the servers**, and both are fixed: the TDS reader
refused token `0xA9` (`ORDER`), which SQL Server sends for every ordered result set — so no catalog
query at all could be read on that backend — and the queue's MySQL list wrote `script text`, a width
`rule:core-classes/schema-vocabulary-is-closed` has no name for, so `schema_of` refused the whole read
on MySQL and MariaDB. The matrix servers' `nvs_jobs` was dropped once so the list rebuilt it.

**Two constructs the vocabulary holds are not portable**, found the same way and written down in
`crates/nvs-db/src/schema.rs`'s gap 1: an index over unbounded text (SQL Server refuses it outright)
and an identifier a backend reserves (`RANK` on MySQL 8). Nothing refuses either yet.

**What is left in goal 9 is stage 7**, the retirement. `2d72fae67` — the driver's unverified WIP
commit — is the **user's** website work (the ADR-to-rules migration), not a session's; it was left
alone.

## Next group

**Stage 7's retirement** — one file set: `crates/nvs-stdlib/src/queue.rs` and
`crates/nvs-cli/src/queue.rs`, with `docs/agent/loop-goal.toml`'s stage 7 checks beside them.

- [ ] **The queue's schema becomes one `nvs_db::schema::Schema` value** —
      `rule:core-classes/queue-storage-is-a-table`, which ADR 0145 amends. **Read ADR 0145
      § Consequences' fourth bullet first: it names the one open question, and the answer is not
      cheap.** The PostgreSQL list dedupes with a *partial* unique index (`… where state = 0`) and the
      MySQL list with a stored generated column; the vocabulary holds neither, and a plain `unique`
      over the nullable `dedupe_key` is **not** the same constraint — SQL Server treats NULLs as equal
      and admits only one, and a full-table unique burns a key once a job succeeds. The spelling that
      does work on all five is a `dedupe_pending` column the *statements* maintain (push writes the
      key, `CLAIM_*` clears it, `RETRY_*` restores it from `dedupe_key`, `CANCEL_*` clears it) with a
      plain unique over it — behaviour-identical, but it edits eight statements, and SQL Server still
      needs the column to be `not null`. Decide it, record it in the rule, and keep the guarantee.
      Anchors: `crates/nvs-stdlib/src/queue.rs:201` (`MIGRATION_POSTGRES`, and its `jobs.dedupe` arm
      is the whole question), `crates/nvs-stdlib/src/queue.rs:272` (`migration`, whose `Option` the
      third named test says must go), `crates/nvs-db/src/ddl.rs:120` (`create_table`, the emitter that
      replaces the text). The three tests are `docs/agent/loop-goal.toml:4638`.
- [ ] **`nvs queue migrate` runs that value on every driver** — the check is
      `docs/agent/loop-goal.toml:4647`, `--connection mssql --dry-run`. **`ddl::create_table` writes
      no `if not exists`**, while stage 1's floor re-runs the applying half against an already-migrated
      server and wants `jobs: applied` (`docs/agent/loop-goal.toml:3148`) — so the command has to
      converge (`nvs_db::plan::diff` against `direct::schema_of`, then apply) rather than run a list,
      and still print a line per table. `crates/nvs-cli/src/schema.rs` already opens all five that way.
      Anchors: `crates/nvs-cli/src/queue.rs:74` (`dialect_of`, whose third refusal goes),
      `crates/nvs-cli/src/queue.rs:310` (`run_all`, which owns the printed labels).

## Backlog

- Spec § 18 owes `Core\Db\Schema` a table — `docs/spec/01-core-library.md`.
- An index over unbounded text and a reserved identifier are unrefused —
  `crates/nvs-db/src/schema.rs` gap 1.
- MySQL's `text` family is unreadable by `catalog::mysql_scalar`, so a plan against any database not
  built by Novis refuses — a decision, not an oversight, and ADR 0145 § 4 is where it would change.
- `crates/nvs-db/src/catalog.rs` is past 2,100 lines.
