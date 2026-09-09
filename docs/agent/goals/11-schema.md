---
milestone: post-parity
---
# Loop goal 11 — `Core\Db\Schema`: tables, columns and indexes, converged

Make a database schema a **value**: written in Novis with typed builders, or read from a file, or read
back off a live server — three spellings of one thing. Diff that value against what a connection actually
holds, grade every difference by what it risks, and print a plan whose every step is complete executable
SQL. Applying it is an operator act.

This is the door `rule:core-classes/db-one-api`'s *Revisiting* leaves open —
*"a portable `Core\Db\Schema`, **if migration tooling in `nvs` itself needs it**, rather than userland"* —
and the trigger has already fired. [crates/nvs-stdlib/src/queue.rs:201](../../../crates/nvs-stdlib/src/queue.rs)
is a hand-written two-dialect DDL emitter for **two tables**, it needed a guard test whose only job is to
stop the two lists drifting, and `migration()` at `:272` answers `None` for SQL Server and SQLite. Goal `database`
owes those two dialects by hand because `rule:core-classes/queue-storage-is-a-table` says all
five backends. **Those hand-written lists are this goal's oracle, not waste**: stage 7 replaces all four
with one schema value, and goal `database`'s own frozen checks — already inside this goal's floor — are what proves
the replacement behaves identically.

Its floor is goal `program-id`'s whole list, which is the parity program, the temp sweep and the program id.

## Why here

It is last for a reason that is not tidiness. And its stage 7 retires `nvs queue migrate`'s four
hand-written dialect lists, which goal `database` writes and freezes the output of: those checks ride in as
this goal's floor and are what proves the replacement, so the throwaway DDL is the oracle rather
than duplicated work. Unlike goals `temp-sweep` and `program-id` its checks bring up every server — the round-trip
property is asserted on all five backends.

## What this is, and the three things it deliberately is not

**It is convergence.** A schema value says what the tables should be; a plan is the difference between
that and what is there. There is no version number, no history table, no ordering between changes, no
`up`/`down`, and no fleet lock. **`Core` never learns what "a migration" is** — that is
`rule:programs/no-migration-runner`'s deliberately-blocked design, it stays
blocked, and `Web\Migration` may still not ship a runner. This goal makes the layer that a runner would
one day sit on, and stops there.

**It is a closed vocabulary.** Tables, columns and indexes — the constructs all five backends have. A
schema value has **no raw escape hatch**: there is no `Schema::raw()` and no `->rawSql()`, because a
schema you can express is exactly what the vocabulary covers and anything else is an `execute()` a human
wrote. Two things follow, and both are the point. The diff is **total** over a schema value, since nothing
opaque can be inside one. And a vendor shipping a new feature costs this project **nothing** — a new
PostgreSQL index type, a new MySQL column option, a new SQL Server data type are all outside the
vocabulary and reachable through `Core\Db::execute` as they are today. The vocabulary grows only when a
construct exists on all five backends *and* something needs it, never to chase a release note.

**It is not a SQL parser, and never will be.** `rule:core-classes/db-literal-query-checking` already refuses
per-dialect SQL grammars for queries — *"it would mean maintaining four vendors' grammars in `nvs-syntax`
forever"* — and that refusal extends to DDL here. Reverse-engineering an existing database is **live
introspection** (`nvs schema dump`), which is strictly better than reading a `CREATE TABLE` its server
printed: catalogs are structured tables rather than grammars, the statement came from that server anyway,
and the introspector has to exist for the diff regardless. A `CREATE TABLE` parser would be a second,
worse implementation of a job already done — `rule:core-api/tier-placement` test 6.

## Stage 0 — the catch-up

Nothing. No fixture predates the rule; the ADR lands inside this goal as stage 2's first slice.

## Stage 1 — the floor

Goal `program-id`'s whole acceptance list — the parity program, the temp sweep and the program id, never traded.
**Goal `database`'s two `nvs queue migrate` checks are inside it**, which is what makes stage 7 a
behaviour-preserving refactor with a frozen expected output rather than a rewrite that has to be trusted.

## Stage 2 — the ADR slot, and the vocabulary it decides

1. **The ADR**, this goal's one design act and its first slice. It carries: the closed vocabulary and the
   rule that it never grows to chase a vendor; convergence rather than versioning, and why `Core` holds no
   notion of a migration; the three grades and the rule that an unknown grade grades *up*; absence never
   destroying; the plan as a document; live introspection as the only reverse path; and the placement
   argument — `rule:core-api/tier-placement` test 2 (DDL is a sink, and only a
   `Core` function may launder an identifier), test 1 (introspection reads a pooled connection), test 6
   (otherwise the queue's emitter, a framework's and userland's are three spellings of one job).
2. **The vocabulary, as types in a new `crates/nvs-db/src/schema.rs`.** A table, its columns, its primary
   key, its unique constraints and its indexes. The column type enum is
   `rule:core-classes/db-column-types`'s type map **in the write direction** — the read direction is
   already on disk and tested, so this is one table used both ways rather than a second table to keep in
   step, and the named test that holds them together is what proves it.
3. **The array form round-trips.** `fromArray(toArray(x)) == x`, as a named test over every construct in
   the vocabulary. This is the one property that makes a file, a live introspection and a hand-written
   builder be the same value — the user's "save the structure and read it back later" and stage 4's
   `dump` are the same mechanism, not two.

## Stage 3 — four dialect emitters, and the plan as a document

1. **Four, not five.** `nvs_db::Dialect` at [crates/nvs-db/src/sql.rs:72](../../../crates/nvs-db/src/sql.rs)
   is already four values for five drivers, because MariaDB and MySQL share SQL text exactly and are two
   drivers for auth plugins and error tables, neither of which reaches DDL. The emitters follow `Dialect`,
   not `Driver`.
2. **Every construct in the vocabulary emits in all four**, as a named test per construct. The traps are
   already written down in [queue.rs:296](../../../crates/nvs-stdlib/src/queue.rs) and are transcription
   rather than discovery: MySQL has no `create index if not exists` and no partial index at all, and
   cannot index a `text` column without a prefix length.
3. **A step carries complete, terminated, dialect-correct SQL** — including the steps this tool will not
   run itself. The plan is a document an operator can paste into a server by hand, which is what makes a
   dry run useful even against a database Novis is never granted permission to write to.

## Stage 4 — five introspectors, and `nvs schema dump`

1. **One catalog reader per driver**, answering the *same* schema value the builder produces:
   `information_schema` for MySQL, MariaDB and SQL Server, `pg_catalog` for PostgreSQL, `sqlite_master`
   plus `pragma table_info`/`index_list` for SQLite.
2. **`nvs schema dump --connection <name>`** writes that value out. This is the whole of
   reverse-engineering an existing database, and it is free: the diff cannot exist without it.

## Stage 5 — the diff, the normalization, and the three grades

1. **The diff compares normalized schema values, never SQL text.** This is where every tool of this kind
   fails, and it fails the same way: the server rewrites what you gave it — default-expression spelling,
   implicit indexes created by a unique constraint, integer display widths, `varchar` promotion, column
   order — and a naive diff reports a phantom change it wants to re-apply forever.
2. **The acceptance criterion is one property, and it is the whole feasibility question**: apply a schema
   to a real server, introspect it back, assert the plan is **empty**. On all five backends.
3. **Three grades, not two.** `Safe` — cannot lose data, cannot fail on existing rows, cannot hold a long
   lock. `Locking` — cannot lose data, can fail or block writes for a long time (a unique index over data
   that already has duplicates, `not null` on a populated column, a type change that rewrites the table).
   `Destructive` — can lose data.
4. **A grade is computed by the dialect emitter, keyed on driver *and* server version**, because adding a
   column with a default is instant on PostgreSQL 11+ and MySQL 8.0.12+ and a full rewrite on older
   servers. **When the emitter does not know, it grades up.** That rule is what keeps the one ageing part
   of this system harmless: an out-of-date grader is an annoyance, never a correctness bug.
5. **Absence never destroys.** A table or column present in the database and not in the schema value is
   *reported* by the plan, with the SQL that would drop it, and is never dropped. Dropping is an operation
   you write, and it is `Destructive`.

## Stage 6 — the member, the CLI, and the risky gate

1. **`Core\Db\Schema` in a new `crates/nvs-stdlib/src/schema.rs`** — the builder, `fromArray`/`toArray`,
   `planAgainst(Queryable)`, and a plan whose steps expose their grade, their reason and their SQL.
   Registry cards per `rule:core-api/reference-card`,
   a reference doc at `docs/reference/core/Db/Schema.md`.
2. **Planning is a read; applying is gated.** `planAgainst` runs ordinary catalog queries under the
   `db.connect` a program already holds. Applying takes a new deny-by-default `db.schema` capability, and
   the two entry points are named for what they do: `applySafe()` refuses if any step is not `Safe`, and
   `applyIncludingRisky()` says so at the call site.
3. **`nvs schema plan|apply|dump`** in a new `crates/nvs-cli/src/schema.rs`, beside `nvs queue migrate`
   and for its reason — `rule:core-classes/queue-storage-is-a-table`'s rule that the runtime
   never issues DDL implicitly at boot or from a request. `plan` is the dry run and prints every step with
   its grade, its reason and its full SQL, including the ones `apply` would refuse.
4. **`examples/schema.nvs`** — define, plan, print the grades, apply the safe half.

## Stage 7 — the retirement

1. **`nvs queue migrate` runs a `Core\Db\Schema` value.** All four `MIGRATION_*` lists in
   [queue.rs](../../../crates/nvs-stdlib/src/queue.rs) collapse to one, `migration()`'s `None` arm for SQL
   Server and SQLite goes away with it, and `the_ddl_creates_every_column_the_statements_name` keeps
   standing over the one remaining home.
2. **It needs no new check.** Goal `database`'s `nvs queue migrate prints both tables` and `creates both tables`
   are already in stage 1's floor with their output frozen. If the schema value emits what the hand-written
   lists emitted, they pass untouched; if it does not, the floor goes red. That is the verification, and
   it is why this goal is at the end of the chain rather than beside goal `database`.

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.**
- **One ADR slot: `Core\Db\Schema` itself**, and it is stage 2's first slice. Everything in *What this is*
  above is settled input to it, not a question it reopens. It amends
  `rule:core-classes/db-one-api` — closing its *Revisiting* item — and
  `rule:core-classes/queue-storage-is-a-table`, whose schema it takes over.
- **`Web\Migration` stays blocked.** `rule:programs/no-migration-runner` is not
  closed by this goal and no session may close it: ordering, fleet locking, reversibility and safety
  against a live multi-tenant database are exactly the things convergence does not need, which is why
  convergence is what is built here.
- **No SQL parser, at any tier, in any form.** Including "just for the CLI", "just best-effort" and "just
  for `CREATE TABLE`". If offline `.sql` input is ever wanted it is a separate tool with its own decision.
- **Foreign keys are out of v1.** The five backends agree on the syntax and disagree on the behaviour —
  SQLite enforces them only under a pragma, adding one is `Locking` because it validates existing rows,
  and they create implicit indexes, which is precisely the normalization trap stage 5 exists to avoid.
  They are added once round-trip emptiness is proven without them.
- **Also out of v1**, and each for the same reason — no portable spelling: partial and expression indexes,
  index types (`GIN`, `GIST`, `FULLTEXT`), collations and charsets, storage and engine options, comments,
  check constraints, generated columns, triggers, views, sequences, partitioning.
- **Defaults are a closed set: a literal of a vocabulary scalar type, or `Default::Now`.** No expression
  defaults. This closes the DDL injection hole and the diff-normalization hole with one rule — an
  expression default is both an unbindable string in a sink and the value the server is most likely to
  spell back differently.
- **SQLite's non-additive alters go through the create-copy-drop-rename rebuild, graded `Destructive`
  unconditionally.** Its `ALTER TABLE` can add, rename (3.25+) and drop a column (3.35+) and essentially
  nothing else. The rebuild is a data copy, so `Destructive` is the honest grade rather than a
  concession, and refusing it outright would make SQLite second-class in the one tool where a dev-machine
  SQLite is most useful.
- **An identifier is validated, never delimited**, by the judgement `Core\Db::quoteIdentifier` already
  states at [crates/nvs-stdlib/src/db/open.rs:1022](../../../crates/nvs-stdlib/src/db/open.rs). A second answer to
  that question is the thing `rule:security/tainted-qualifier` refuses.
- **Ambiguity about a seam resolves toward `nvs-db`**: the vocabulary, the emitters, the introspectors and
  the diff are all sans-io and belong beside `Dialect`, per
  `rule:core-classes/db-drivers-are-an-enum`. `nvs-stdlib` holds
  the members and `nvs-cli` the command, and neither decides anything about DDL.

## What this goal does not touch

The framework half — `Web\Migration`, migration files, ordering, history tables, fleet locking,
reversibility. Schema-aware query checking (`nvs check --schema`), which is
`rule:core-classes/db-one-api`'s own separate *Revisiting* item and needs a build-time dependency
on a reachable database. And the deferred driver items that are not schema at all — LOB streaming, stored
procedures, `COPY`, `LISTEN`/`NOTIFY`.
