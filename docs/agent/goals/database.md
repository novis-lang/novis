---
milestone: M8
---
# Loop goal 5 — `Core\Db` and five drivers

Finish **M8's database half** — `rule:core-classes/db-one-api` is the design and
[01-core-library.md](../../spec/01-core-library.md) § 18 is the signature list. **One API replaces `PDO`,
`mysqli`, `pgsql` and `sqlite3`**, over pure-Rust MySQL, MariaDB, PostgreSQL and SQL Server drivers plus
SQLite.

This is goal `database` of the parity program ([goals/README.md](README.md)). It is a separate goal from goal `core-part-ii`
purely because `nvs-db` shares no file with `Core\Cli` — the two would have made one manifest naming every
module in the workspace, which is the cost loop-authoring.md § 2 exists to avoid.

**This goal has an external precondition and the driver enforces it.** `rule:core-classes/db-one-api` verifies against real
servers, so `python tools/loop.py` preflights a reachable Docker daemon before this goal's first
session and stops the run naming it. A run that grinds for six hours against a check that cannot pass is
worse than one that stops in the first minute.

**And PHP is now a real oracle for this goal, which it was not when the program was written.** The
oracle build gained `mysqli`, `pgsql` and `sqlite3`, so the three APIs this goal replaces
can be *run* rather than only described. Two things follow. The migration table gains 227 names that are
all this goal's — a fifth of the whole inventory, and the reason its floor is 94% where every earlier
goal's is in the thirties or seventies. And a `--ORACLE--` case can now put `Core\Db` beside `mysqli` or
`pg_query` against the same server and compare, which is the only way the "PHP-compatible observable
behaviour" half of `rule:core-classes/db-column-types`'s type map is checkable at all: `TINYINT(1)`, `BIGINT UNSIGNED` past
`i64::MAX`, a zone-less `DATETIME` and `affected` versus `changed` are all rows where PHP's answer is the
specification. Write those as differential cases, not as frozen ones.

## The shape every session must hold

**A driver is synchronous code over goal `concurrency`'s parking stream.** `rustls` layers on the same stream for TLS.
There is no async runtime and `sqlx`, `tokio-postgres` and `tiberius` are not usable here — not as a
preference but structurally, because they need a runtime that spawns. What *is* usable is the wire-protocol
half of the ecosystem: `mysql_common`, `postgres-protocol`, and our own TDS. SQLite is `rusqlite`, and it
is [ADR 0051 § 4](../../decisions/0051.md)'s **one audited C exception** — its test suite
is orders of magnitude larger than its source and it is continuously fuzzed, which is the exceptional
verification record that question 2 asks for. Nothing else clears that bar, and goal `core-part-ii` built the
enumeration check that says so.

**Every statement is prepared, and emulated prepares do not exist in any form** (§ 1). That is what makes
escaping-based APIs — `PDO::quote`, `mysqli_real_escape_string` — have nothing to be the correct answer
to. A driver that interpolates inside itself has reintroduced exactly the thing this ADR removes.

## Stage 0 — the catch-up

2. **`#[Db\Derive]` and `#[Db\Field]` join `nvs_types::derive::ATTRIBUTES`.** They are deliberately absent
   today — [derive.rs:47](../../../crates/nvs-types/src/derive.rs) says so: "a closed list that names
   something with no pass behind it is worse than a short one." It is the same pass over a second format,
   and it goes first because § 6's `queryAs<T>` is written against it and a row-mapping written without it
   is written twice.

## Stage 1 — the floor

M4's and goals `core-depth` through `core-part-ii`'s whole acceptance lists, **never traded.**

## Stage 2 — the keystone: one connection, one prepared statement, one row

**No second driver is written until the first one is green end to end.** PostgreSQL first: its extended
protocol pays nothing extra for a prepare (§ *Context*), `postgres-protocol` is the cleanest of the wire
crates, and it has the reset that § 13 calls the good case. The other four are then the same shape.

3. **`crates/nvs-db` exists, and a PostgreSQL connection is opened, TLS-wrapped and authenticated.** Over
   goal `concurrency`'s `NvsTcp` with `rustls` on it — the item is the *seam*, and it is this goal's ADR slot.
4. **A connection is named, or built from settings, and is memoized for the request** — § 2. The settings
   are **five types, not one loose shape**: SQLite takes a `path` and has no `host`, so a `host` on a
   SQLite settings literal is a **compile error** rather than a silently ignored field.
5. **`db.connect` and `db.open`, and what they mean for `rule:http-server/allow-url-pins-the-address`** — § 3. Connections are named in
   root-owned config. A `connect`-named private-range endpoint succeeds with **no `net.connect` grant**,
   while a `db.open` target in a denied range fails — the two capabilities answer different questions and
   collapsing them is the mistake.
6. **`LOCAL INFILE` is refused**, and the fixture that proves it stands up a rogue MySQL server and
   asserts no file is sent. § 3's own verification names it: a malicious server can answer any query with
   a `LOCAL INFILE` request, so this is a property of the *client*.
7. **The connection charset is forced to UTF-8** (`utf8mb4` on MySQL/MariaDB), so text columns arrive as
   text and nothing downstream guesses.

## Stage 3 — the statement

8. **Five ways to run a statement, and only one of them streams** — § 4. Large-result streaming at
   constant memory, and a second statement on a busy connection is a `LogicError` — SQL Server's MARS
   could lift that restriction and it is deliberately not lifted, so that code written against one driver
   runs on all four.
9. **`?` or `:name`, and one parameter is one value** — § 5. The rewriter maps to each driver's own form
   (`$1` on PostgreSQL, `@p1` on SQL Server) and **skips string literals, comments and PostgreSQL's `::`
   cast**; a `:name` used twice binds one value once, which positional form cannot express. `inList`
   expansion and its empty-list throw ride here. The `::` cast and a jsonb `?` operator **in the same
   query** is the case that catches a naive rewriter.
10. **A row is dynamic or declared, and the requested type drives the conversion** — § 6. `queryAs<T>`
    throws **naming the column** for a type mismatch, a missing column and a NULL in a non-nullable field.
    Naming the column is the item: a row mapper that throws without one is unusable at 40 columns.

## Stage 4 — the type map

11. **Every row of § 9's map round-trips.** The rows that are rules rather than mappings, each of which
    has its own case: MySQL's `TINYINT(1)` reads `int` **and** `bool` and throws for a stored `7`;
    `BIGINT UNSIGNED` past `i64::MAX` reads `uint` and throws for `int`; a `DECIMAL` into a `float` field
    **throws** rather than rounding; a zone-less column reads as `DateTime` in the declared zone and a
    `TIMESTAMPTZ` ignores it; `affected` is the matched count on all four drivers while `changed` is
    non-null only on MySQL/MariaDB.
12. **"Everything is a string" does not happen.** § *Context* names it as an artefact of the MySQL text
    protocol and the reason `$row['id'] == 1` silently fails in PHP. A driver that returns strings has
    reproduced the defect this API exists to remove.
13. **Two things come from goal `core-depth` and goal `core-part-ii`**: `Core\Time`'s types for the date columns, and `Core\Json`
    for the JSON ones — which are **not** auto-decoded.

## Stage 5 — transactions

14. **A transaction is a closure, and `Transaction` is a `Queryable` rather than a second surface** — § 7.
    Savepoint nesting, and **`rollBack` surviving an intervening `catch (Throwable)`**, which is the case
    a naive implementation loses because the catch swallows the signal the rollback was waiting on.
15. **`{retries: n}` against an induced deadlock**, and `SerializationFailure` is the kind that makes it
    safe to retry at all.
16. **One `DbError`, with a normalised `kind`** — § 8, across four drivers and **five dialects**: MariaDB
    needs its own code table and not MySQL's. SQLite's `SQLITE_BUSY`/`SQLITE_LOCKED` map to the same kinds
    everything else does. `sqlState` and `driverCode` stay available for what normalisation does not cover,
    and **a `DbError`'s message contains no bound value.**

## Stage 6 — the other four drivers

17. **MySQL, MariaDB, SQL Server, SQLite**, each to the shape Stage 2 set. MariaDB is a **distinct driver
    and not a MySQL flag** — § *Context* argues it: `RETURNING`, a bulk-execute protocol MySQL lacks, a
    native `UUID` type, and its own error-code table. `executeMany` stays N executions there, per § 4:
    a bulk command cannot reproduce what the loop lets a caller observe.
18. **MariaDB's `ed25519` and `parsec` authentication plugins.** `rule:packaging/a-c-dependency-answers-two-questions` named this case in advance
    so it would be answered by the test rather than by convenience: an authentication handshake handles
    attacker-reachable data, so question 2 applies, and the answer is **a Rust implementation of the
    plugin, or a documented refusal to support that auth method** — never a C dependency.

## Stage 7 — the pool

19. **The pool is per core, keyed as `connect`/`open` already key** — § 13. Per core because a heap is
    only ever touched by one thread and a cross-core pool would need atomics on the path a request takes.
20. **The reset is a security boundary, not an optimisation.** PostgreSQL's targeted reset preserves the
    statement cache; MySQL's and SQL Server's protocol resets do not; **a failed reset destroys the
    connection rather than returning it.** A connection returned to the pool carrying one request's state
    is one request reading another's, which is why this is stated as a boundary.

## Stage 8 — `Core\Queue`

21. **`rule:concurrency/enqueue-commits-with-your-write`, whole**: the jobs and dead-letter tables,
    `nvs queue migrate`, per-backend `SKIP LOCKED`-shaped claiming, the visibility timeout, bounded retries
    with jittered backoff.
22. **The transactional-enqueue property is the reason for the whole design** — § 3: an enqueue commits
    with your write, so a job never exists for a row that was rolled back. A queue that is a separate
    broker cannot have that property, and § 8 says why there is no broker.
23. **Queued work and scheduled work are different** — § 7. `[[schedule]]`'s validation landed in goal `governance`;
    this is the other half and the two are not unified.

## Stage 9 — what `nvs check` proves, and the trace

24. **§ 10's three compile-time diagnostics on a literal query**: a placeholder-count mismatch, mixed
    placeholder styles, and an `open` host matching no grant. Plus the security half: **a `tainted` value
    at a query-text parameter is a compile-time diagnostic and the same value at a bound parameter
    compiles**, and a **two-statement literal query** is a diagnostic. A `tainted` `Settings.host` is a
    diagnostic naming `Core\Taint::assertTrusted`.
25. **A query emits a `query` span with no parameter values anywhere in it** — § 11.

## The harness this goal owes, and it is Stage 2's first slice

Two files that do not exist, and the goal cannot verify itself without them. They are named here rather
than left to be invented at 2 a.m. by the session that first needs a server:

- **`tests/db/compose.yaml`** — MySQL, MariaDB, PostgreSQL, SQL Server and Redis, each pinned to a version
  and each with a healthcheck, so `docker compose up -d --wait` means *healthy* rather than *started*.
  Redis is there because goal `core-part-ii`'s shared cache tier and goal `server`'s fleet lease both use it, and one compose
  file is better than two that drift.
- **`python tools/db-matrix.py`** — runs `rule:core-classes/db-one-api`'s per-driver list against those servers and prints one
  `<driver>: ok` line each. It is a harness rather than a test: the assertions are `nvs-db`'s own, and
  this is what points them at five endpoints and reports which one failed.

Write both before the first driver, not after: a driver with no server to run against is a driver whose
tests are all mocks, and that is the one shape `rule:core-classes/db-one-api`'s *Verification* refuses.

## Acceptance

**This goal is retired: its checks are the floor stage of the live goal**, carried
there by the switch that left it and folded forward at every switch since. Four of the five drivers are
verified against real servers in containers, brought up once per run and memoized against `crates/nvs-db`
like the WSL leg already is.

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.**
- **One ADR slot: the driver crate's shape and its wire I/O** (Stage 2, item 3), and it is that stage's
  first slice. Which protocol crate backs which driver, how TLS layers on the parking stream, how a
  connection's busy state is tracked, and how the five drivers share code without a trait that flattens
  their differences. `rule:core-classes/db-one-api` specifies *behaviour* and deliberately does not specify this.
- **No async runtime, and this is structural rather than a preference.** `sqlx`, `tokio-postgres` and
  `tiberius` need a runtime that spawns. If a driver appears to require one, that is a real `BLOCKED`
  naming the driver — not a judgement call, and not a reason to add `tokio` behind a feature flag.
- **Emulated prepares do not exist in any form.** A driver that interpolates a value into SQL internally
  has reintroduced the thing `rule:core-classes/db-one-api` removes, however careful the escaping.
- **MariaDB is its own driver.** Treating it as a MySQL flag is a design error `rule:core-classes/db-one-api` argues at length
  and not a simplification to rediscover.
- **A failed connection reset destroys the connection.** Returning it to the pool is one request reading
  another's state.
- **PostgreSQL first, then the rest.** Its extended protocol pays nothing extra for a prepare, so the
  first driver is the one that exercises the design rather than the driver's own quirks.
- **SQLite's C dependency is the one audited exception**, under `rule:packaging/a-c-dependency-answers-two-questions`'s second question, and goal `core-part-ii`
  built the enumeration check that fails on any *addition* to that list. A second C dependency is a
  `BLOCKED`.
- **Picking every dependency but the two the user named** stays pre-authorized under `rule:packaging/a-c-dependency-answers-two-questions`, with
  the three obligations a Rust dependency owes.

## What this goal does not touch

`Web\Migration`, which `rule:programs/no-migration-runner` records as deliberately
blocked until an ADR closes it — ordering, transactional DDL, fleet locking, reversibility, safety against
a live multi-tenant database. `nvs queue migrate` is **not** that: it creates two tables this ADR
specifies, and a session that finds itself generalising it has drifted into the blocked design. The
listener and everything request-shaped (goal `server`), including `#[Test(db:)]`'s rolled-back transaction, which
needs the test-server half that arrives with it.
