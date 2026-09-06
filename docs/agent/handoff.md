# Handoff

## State

**Goal 9 stage 7 — the retirement — is whole, and stage 7 was the last stage.** All three of the
stage's `nvs-stdlib` tests pass, `nvs queue migrate --connection mssql --dry-run` prints both
tables for a backend that never had a dialect, and stage 1's two frozen `nvs queue migrate` checks
still hold. `python tools/verify.py` is 7 of 7 green.

**`nvs_stdlib::queue::schema()` is the one home for the queue's tables**
(`crates/nvs-stdlib/src/queue.rs:260`). `MIGRATION_POSTGRES`, `MIGRATION_MYSQL` and `migration`'s
`Option` are gone; `migration(driver) -> Vec<Migration>` (`:181`) emits the value through
`nvs_db::ddl` in whichever dialect a driver speaks, labelling each statement `jobs` or
`dead_letter`. Every statement that moves a job in or out of `Pending` maintains `dedupe_pending`,
which is `rule:core-classes/queue-storage-is-a-table`'s guarantee as a plain column under a plain
unique key.

**`nvs queue migrate` converges** (`crates/nvs-cli/src/queue.rs`): `--dry-run` opens nothing and
prints the create-from-nothing statements, and the applying half runs
`crate::schema::converge` — `nvs schema apply`'s own body, now shared. It refuses a step that is
not `Safe` and takes `--including-risky`, which is new surface on this command.

**The upgrade edge is real and is not code.** A deployment whose queue tables an older Novis built
needs one risky converge: `queue` and `dedupe_key` narrow to `varchar(255)`, and the old *partial*
index `nvs_jobs_dedupe` is invisible to introspection, so it collides by name and has to be dropped
first. This machine's `[db.main]` PostgreSQL was converged by hand that way, which is why stage 1's
applying check is green; the MySQL and MariaDB containers still carry the old generated-column
table, and `crates/nvs-stdlib/tests/queue.rs`'s fixture now drops and rebuilds both tables, so a
matrix leg fixes itself.

## Next group

**The queue's own gap 5 — the two backends that have a schema and no statements** — one file set:
`crates/nvs-stdlib/src/queue.rs` and `crates/nvs-stdlib/tests/queue.rs`. Nothing in goal 9 asks for
it; it is what the retirement made visible, and it is the next honest slice in this module.

- [ ] **SQL Server's dedupe needs the vocabulary before it needs a statement** —
      `rule:core-classes/queue-storage-is-a-table` says so: two nulls are equal there, so a plain
      unique key over `dedupe_pending` admits one released row rather than any number of them, and
      the spelling it wants is the filtered index `rule:core-classes/schema-plan` keeps out of v1.
      Decide whether the vocabulary grows before `Core\Queue` gains a fourth dialect, at
      `crates/nvs-stdlib/src/queue.rs:260` (`schema`) and `:1722` (`no_dialect`, the refusal an
      operator reads today).
- [ ] **`queue_connection` is the roster three places now spell by hand** —
      `rule:core-classes/db-drivers-are-an-enum`. `crates/nvs-stdlib/src/queue.rs:1661` is the
      `match`, and both `the_queues_refusal_is_only_ever_about_a_driver_that_cannot_send` and
      `crates/nvs-stdlib/tests/queue.rs:88` now `matches!` the same three drivers beside it. One
      predicate on that seam would make a fourth backend one edit.

## Backlog

- `nvs queue migrate --including-risky` is new CLI surface no `docs/` page names — `docs/spec/`.
- A partial index a deployment already has is invisible to introspection and collides by name;
  whether that is a diagnostic is `rule:core-classes/schema-introspection`'s question.
- `examples/queue.nvs` leaves `scripts/receipt.nvs` rows behind from the stdlib fixture's pushes,
  so its stderr carries warnings about a script that does not exist — `tests/db/`.
