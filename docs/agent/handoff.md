# Handoff

## State

**Goal 9 stage 7 — the retirement — is started, not whole.** `nvs_stdlib::queue::schema()` is on
disk (`crates/nvs-stdlib/src/queue.rs:429`): `rule:core-classes/queue-storage-is-a-table`'s two
tables as one `nvs_db::Schema`, which `nvs_db::ddl` emits in all four dialects, so SQL Server and
SQLite have a schema for the first time. One of the check's three tests,
`every_driver_has_a_queue_schema_and_none_answers_none`, is green on it.

**The retirement's own open question is decided and recorded** in the rule: `dedupe_pending` is a
plain nullable column the statements maintain — the key while pending, `null` once not — under a
plain unique key. A partial index and a stored generated column are two dialects' answers to one
requirement and neither is in the vocabulary. A `not null` column with a generated token was refused
for a reason that outlives SQL Server's null-equality: it cannot be added to a table that already
holds rows, so no existing deployment could converge to it.

**Nothing's behaviour changed yet.** `MIGRATION_POSTGRES` (`:201`), `MIGRATION_MYSQL` (`:339`) and
`migration()`'s `Option` (`:272`) are all still there, and the statements still reach the guarantee
the old way.

**The three remaining slices cannot be split**, which is the session's main finding. `nvs_db::ddl`
writes no `IF NOT EXISTS` (`crates/nvs-db/src/ddl.rs:1210` pins the absence), so the moment the
lists go, `nvs queue migrate` must converge instead of running statements; and the moment the value
is what it runs, PostgreSQL's dedupe is a unique key over `dedupe_pending` rather than a partial
index, so the statements must be maintaining that column by then or the guarantee is gone.

## Next group

**Stage 7's retirement, the rest of it** — one file set: `crates/nvs-stdlib/src/queue.rs`,
`crates/nvs-cli/src/queue.rs` and `crates/nvs-cli/src/schema.rs`. Take all three together; the first
two are unsafe to land apart.

- [ ] **The statements maintain `dedupe_pending`** — `rule:core-classes/queue-storage-is-a-table`,
      whose new paragraph is the specification. PostgreSQL's probe becomes
      `where dedupe_pending = $1::text` (no `state = 0`; `$1` repeats, so no new bind), the insert
      names the column, and `set dedupe_pending = null` joins the three transitions out of pending
      while `RETRY_*` restores it from `dedupe_key`. MySQL's probe already reads the column; its
      insert needs a tenth bound slot, because a `?` cannot repeat — the array is
      `crates/nvs-stdlib/src/queue.rs:1962` (`[Option<&[u8]>; 9]`, the `Queued::Framed` arm).
      Anchors: `crates/nvs-stdlib/src/queue.rs:512` (`INSERT_POSTGRES`), `:565` (`CLAIM_POSTGRES`),
      `:622` (`SUCCEEDED_POSTGRES`), `:636` (`RETRY_POSTGRES`), `:964` (`CANCEL_POSTGRES`), `:724`
      (`INSERT_MYSQL`), `:752` (`CLAIM_MYSQL`), `:810` (`SUCCEEDED_MYSQL`), `:827` (`RETRY_MYSQL`),
      `:976` (`CANCEL_MYSQL`). `DEAD_LETTER_*` needs nothing — the row leaves the table.
- [ ] **`migration()` becomes total and the two lists go** —
      `rule:core-classes/queue-storage-is-a-table`; `the_queues_schema_is_one_value_and_no_dialect_list`
      is the check (`docs/agent/loop-goal.toml:4639`). `Migration`'s fields become owned `String`s
      (`crates/nvs-stdlib/src/queue.rs:153`) and `migration(driver) -> Vec<Migration>` walks
      `schema().tables()` through `nvs_db::ddl::create_table`, labelling the create-table statement
      `jobs` / `dead_letter` — the labels are what stage 1's frozen `want` lists read, not the table
      names. **~35 doc links name `MIGRATION_POSTGRES` or `MIGRATION_MYSQL`** across
      `crates/nvs-stdlib/src/{queue,lib}.rs`, `crates/nvs-cli/src/{queue,worker}.rs` and
      `crates/nvs-stdlib/tests/queue.rs`; they are broken intra-doc links, not prose to leave.
      `the_schema_value_declares_every_column_the_lists_declare` (`:3414`) retires with them.
- [ ] **`nvs queue migrate` converges instead of running a list** — the check is
      `docs/agent/loop-goal.toml:4644`, and stage 1's two frozen ones
      (`:3136`, `:3148`) are what it must not break. Make `crates/nvs-cli/src/schema.rs:282`
      (`opened`) and `:202` (`planned`) `pub(crate)` and call them: `--dry-run` prints
      `-- <label>` plus `ddl::create_table` per table and opens nothing, and the applying half runs
      `plan.runnable()` and then prints `-- jobs: applied` / `-- dead_letter: applied` per table so
      a second run is still green with an empty plan. `rule:core-classes/schema-absence-never-destroys`
      is why diffing the queue's two tables against a whole database is safe: every other table is a
      report and reports are never runnable. Anchors: `crates/nvs-cli/src/queue.rs:74`
      (`dialect_of`, whose `None` arm goes), `:118` (`migrate`), `:258` (`apply`), `:310`
      (`run_all`).

## Backlog

- SQL Server reads two nulls as equal, so its `nvs_jobs_dedupe` admits one released row — recorded
  in `rule:core-classes/queue-storage-is-a-table`, and it needs a filtered index the vocabulary
  does not hold. Reopen with the queue's SQL Server statements, not before.
- The two non-portable constructs the vocabulary holds — an index over unbounded text, a reserved
  identifier — still refuse nothing; `crates/nvs-db/src/schema.rs`'s gap 1 owns them.
- `Core\Queue` still throws `no_dialect` for SQL Server and SQLite: a schema for them is not
  statements for them. `crates/nvs-stdlib/src/queue.rs`'s gap 5.
- `docs/decisions/0145.md` § Consequences' first bullet claims the drift-guard test disappears with
  the second list; check that reading when the lists go.
