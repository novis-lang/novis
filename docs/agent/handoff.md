# Handoff

## State

**M8 goal 5. ADR 0084 § 2's schema now has two dialects and `nvs queue migrate` runs whichever one
its block's driver speaks; the queue's *members* still send PostgreSQL only.** That split is the
state to hold onto: `crates/nvs-stdlib/src/queue.rs:260`'s `migration(driver)` is the one place a
dialect is chosen, `MIGRATION_POSTGRES` and `MIGRATION_MYSQL` are the two lists, and module gap 5
now names what is left — § 4's claim, § 1's `insert` and § 6's move, not the schema under them.

**`MIGRATION_MYSQL`'s doc owns every construct decision** and is the file to read before writing a
second statement set: the dedupe index is a stored generated column (`case when state = 0 then
dedupe_key else null end`) with a plain unique key over it, because MySQL has no partial index and
its unique indexes do not collide on `null`; each table's indexes are declared inside its own
`create table if not exists`, because `create index if not exists` does not exist there; the two
indexed columns are `varchar(255)` because MySQL cannot index a `text` without a prefix, and a
prefix-unique index would refuse two distinct keys sharing a prefix.

**Two agreements are held by tests rather than by matching lists.**
`the_ddl_creates_every_column_the_statements_name` walks *both* lists against the same statements,
so the dialects may differ freely in constructs and not at all in columns;
`the_schema_has_a_dialect_for_every_driver_that_can_be_sent_one` holds `migration` to
`crate::db::rendering_for`, so a schema for a driver nothing sends fails, as does the reverse.

**One latent bug fixed on the way:** `crates/nvs-cli/src/queue.rs`'s shared `address_of` defaulted
every driver's port to PostgreSQL's 5432; it now takes the driver's own, as `nvs_stdlib::db`'s twin
already did.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` is a busy-state cell. Do not rename it, split it, or write a MySQL-only test
under it.

## Next group

**One file set: `crates/nvs-stdlib/src/queue.rs`.** Take the first two together — a second statement
set and the seam that reaches it are one change, and splitting them lands statements nothing calls.

- [ ] **§ 4's claim and § 1's push in MySQL's dialect** (0084 §§ 1 and 4).
      `crates/nvs-stdlib/src/queue.rs:424` is `CLAIM` and `crates/nvs-stdlib/src/queue.rs:375` is
      `INSERT`; `crates/nvs-stdlib/src/queue.rs:505` is `DEAD_LETTER`, the third statement resting
      on a construct MySQL lacks. No `returning` and no data-modifying CTE, so each becomes a
      `select … for update skip locked` and then an `update`, both inside one transaction — two
      statements in one moment, which is the property `CLAIM`'s own doc argues for rather than the
      single statement it happens to use. **First, settle whether these texts go through
      `nvs_db::sql::rewrite`** (ADR 0067 § 5's one-spelling-in rewriter): if they do, the `$n`
      placeholders are already per-driver and only the constructs and the `::text` casts differ,
      which is a materially smaller second list than it looks.
- [ ] **The members reach a connection by driver, not through `postgres_of`** (0084 § 4).
      `crates/nvs-stdlib/src/queue.rs:1422` is `postgres_of` and
      `crates/nvs-stdlib/src/queue.rs:1465` is `no_dialect`, whose two sentences narrow again as
      each member gains its second arm. `crates/nvs-cli/src/queue.rs`'s `dialect_of` is the shape
      that worked for the command.
- [ ] **A matrix case that migrates and pushes against a real MySQL server** (0084 § 2).
      `crates/nvs-stdlib/tests/queue.rs:97` is `schema`, which runs `MIGRATION_POSTGRES` against
      the compose PostgreSQL; the MySQL leg is the same shape over `nvs_db::matrix`.

## Backlog

- MySQL cannot add an index to a table an older Novis created — `MIGRATION_MYSQL`'s doc says so and
  the migration command owns the day there is a schema change to make.
- SQL Server and SQLite have no schema because they send no statement; `crate::db`'s gap 2 is the list.
- `nvs queue migrate` against a MySQL block is untested end to end — no compose leg names one yet.
- Module gap 1: `limits`/`grants` wait on a shape parameter, now that ADR 0135 has specified one.
