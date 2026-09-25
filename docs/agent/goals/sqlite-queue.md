---
milestone: M8
---
# Loop goal 35 — the queue runs on SQLite

`Core\Queue`'s members and the in-process worker run on SQLite, so `runs` answers true for three of
the five drivers and a deployment gets a working queue out of a file with no server to run at all.
Nothing about the queue's guarantees is different there: a job is claimed once, attempts are finite,
a dead-letter is kept, and `purge` is bounded. What changes is that the backend a small deployment
already has is one of the backends the queue admits.

SQL Server stays out, and this goal does not narrow the gap it leaves. Its refusal is not the same
refusal: two nulls are equal there, so `dedupe_pending`'s plain unique key admits one released row
rather than any number of them, and the filtered index that fixes it is vocabulary the schema plan
keeps out of v1. That is a schema question in front of a statement question, and it is not this
goal's.

## Why here

After goal `queue-purge` because that goal adds `delete` and `purge`, and a third dialect written in
front of them would be written twice — once for four members and again for six. Before goal
`serve-runs-the-queue` because that goal arms workers under `nvs serve`, and a dialect landing after
it would leave the served path proven on two backends and shipped on three.

It needs nothing that is not already built. `SqliteConn` is a complete driver — `query`,
`execute_many`, `begin`/`commit`/`roll_back` and § 13's `reset`
(`crates/nvs-db/src/sqlite.rs:595-823`) — `Core\Db` already runs SQLite through execute, the pool
and schema, and `nvs queue migrate` already converges § 2's two tables on it, because `migration`
emits `schema` in whichever dialect `nvs_db::ddl` speaks for the driver it was handed
(`crates/nvs-stdlib/src/queue.rs:216-228`). What is missing is § 4's texts, one transaction
primitive underneath them, and the three seams that reach them. Nothing else.

## Stage 0 — the catch-up

The sentences already on disk that this goal makes wrong, each with the file that holds it.

1. **`crates/nvs-cli/src/worker.rs:903-911` is wrong today, not only after this goal.** It tells an
   operator that SQLite "runs no statement at all yet — `Core\Db`'s own known gaps are the list".
   `Core\Db` runs SQLite everywhere: `crates/nvs-stdlib/src/db/execute.rs`,
   `crates/nvs-stdlib/src/db/pool.rs:71` and `crates/nvs-stdlib/src/db/schema.rs:287` each hold a
   `Connection::Sqlite` arm. The gap was only ever `Core\Queue`'s statements, which is what
   `no_dialect`'s own arm says correctly. **Fix that sentence in the first slice that opens the
   file**, whatever stage it belongs to: an operator acting on it today looks in the wrong crate.
2. `runs`'s doc and `no_dialect`'s SQLite arm (`crates/nvs-stdlib/src/queue.rs:2122-2143` and
   `2226-2232`) both describe a two-dialect module. So does `queue_connection`'s "Two arms and not
   five, because that is how many dialects this module has".
3. `crates/nvs-cli/src/worker.rs`'s module doc prices a claim as "a transaction's worth of round
   trips on MySQL and MariaDB where it costs a single statement on PostgreSQL"; a third backend
   joins that sentence. `Wire`'s own doc says "The drivers with no send path are not arms", and
   `report`'s says the affected count is discarded "on both drivers".
4. This directory's `README.md` row and the plan's `Carried by` cell, which `python tools/plan.py
   --sync` writes.

## Stage 1 — the floor

Goal `queue-purge`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never
traded. It runs against PostgreSQL and MySQL, which is why this goal declares a `[docker]` table
even though its own half needs no daemon.

## Stage 2 — the keystone: an immediate transaction, and the claim on top of it

One file set: `crates/nvs-db/src/sqlite.rs`, `crates/nvs-stdlib/src/queue.rs`.

`rule:concurrency/claiming-is-one-statement` already names this backend's mechanism — "an immediate
transaction on SQLite, whose single-writer model makes contention moot" — and `CLAIM_POSTGRES`'s own
doc names it a second time. **The mechanism does not exist yet.** `SqliteConn::begin` emits a bare
`BEGIN` at depth 0 and ignores its `isolation` argument entirely
(`crates/nvs-db/src/sqlite.rs:707-720`), which SQLite reads as `DEFERRED`.

**Why that is the whole stage.** A deferred transaction that reads and then writes takes a shared
lock and asks to upgrade it, and SQLite answers an upgrade it cannot grant with `SQLITE_BUSY`
*immediately, without honouring the busy timeout* — a busy handler cannot back off a transaction
that is already holding a read lock without breaking that reader's own snapshot. So `CLAIM_MYSQL`'s
shape transcribed onto this backend — a locking `select`, then an `update` — is a claim that fails
under exactly the concurrency it exists to survive, and it fails in the way that is hardest to see:
never on one connection, only under two.

1. **The primitive is its own entry point, not a new `Isolation` case.** `Isolation` is the SQL
   standard's five levels and none of them means "take the write lock now"; SQLite's isolation is
   already serializable and the question here is *when* the lock is acquired, which that enum does
   not describe. A dedicated `SqliteConn` entry point keeps `Core\Db::transaction`'s behaviour for
   every existing caller exactly where it is — this goal changes no language surface — and leaves
   the queue as the one caller that asks. A session that finds a better placement records it in the
   ADR slot below rather than widening the enum quietly.
2. **The depth accounting is the part to get right, not the keyword.** `begin` moves `depth` only
   after a command SQLite accepted, `commit` deliberately leaves the depth where it was on a refused
   outermost commit, and a nested call is a `SAVEPOINT`. An immediate transaction is an outermost
   one by definition, so asking for one at depth greater than zero is the refusal to write, beside
   the two that type already has.
3. **The claim is then `CLAIM_MYSQL`'s two statements inside it**, answering `CLAIM_POSTGRES`'s six
   columns in its order — `crates/nvs-cli/src/worker.rs` reads them by position and
   `both_dialects_answer_a_claim_with_the_same_columns` is the test that holds them together.
   Extend that test to a third dialect rather than writing a second one. `attempts + 1 as attempts`
   is `CLAIM_MYSQL`'s reason and not PostgreSQL's: the `update` has not run when the `select`
   answers, so the column is read as the value it is about to have.
4. **`skip locked` has no spelling here and needs none.** The mutual exclusion is the database's
   single writer: inside an immediate transaction no second connection is writing at all, so two
   workers cannot come back with one row. That is stronger than a row lock, not weaker, and it is
   the sentence the statement's own comment owes — a later contributor reading a claim with no
   locking clause will otherwise try to add one.

**The check that decides this stage runs two connections against one file**, not one connection
twice. A claim proven on a single connection is a claim whose whole failure mode was not exercised.

## Stage 3 — the worker's other statements

One file: `crates/nvs-stdlib/src/queue.rs`.

`INSERT`, `DEAD_LETTER`, `SUCCEEDED`, `RETRY` and `QUEUES` in the third dialect.

1. **`SUCCEEDED`, `RETRY` and `QUEUES` are one-table statements with no construct behind them** —
   near-copies of the MySQL texts, differing in the placeholder spelling and nothing else.
2. **`INSERT` is the dedupe, and stage 2's ordering rule is what governs it.** `INSERT_MYSQL` reads
   first and writes second; inside an immediate transaction that is safe here, because the write
   lock was taken before the read. Written outside one it is the upgrade this backend refuses.
3. **`DEAD_LETTER` copies MySQL's order and not PostgreSQL's** — the copy before the delete, both
   halves keyed on the lease, for exactly the reason `DEAD_LETTER_MYSQL`'s doc gives: the columns
   have to be read while they still exist, and a worker that overran § 4's visibility window matches
   no row in either half. Both halves are writes, which is what makes the transaction around them
   cheap.

## Stage 4 — the six members' statements

One file: `crates/nvs-stdlib/src/queue.rs`.

`STATUS`, `CANCEL`, `COUNTS`, `DELETE`, `PURGE` and `PURGE_DEAD`.

1. **`STATUS`, `CANCEL` and `COUNTS` are portable** — the state ordinals stay literals for
   `PENDING`'s reason, and `queue_statements_agree_with_the_state_enum` is extended to the third
   list rather than joined by a second test.
2. **`DELETE` is the one member with no single-statement shape on this backend.** PostgreSQL's is a
   data-modifying CTE and SQLite has `RETURNING` but no data-modifying CTE; MySQL's is a multi-table
   `delete j, d` and SQLite has no spelling for that either. So it is a `Split` of two deletes, and
   `DELETE_MYSQL`'s doc is the argument for why they must be one moment rather than two: § 6's
   dead-letter move can land between them and take away the one row the member exists to leave
   alone. Stage 2's transaction is what holds it.
3. **`PURGE` copies PostgreSQL's bound and not MySQL's.** `delete … limit` needs
   `SQLITE_ENABLE_UPDATE_DELETE_LIMIT` set at compile time and `libsqlite3-sys`'s bundled build does
   not set it; `delete … where id in (select id … order by id limit ?)` is `PURGE_POSTGRES`'s shape
   and needs no build option at all. `order by id` carries over with it, for that constant's reason:
   it is `created_at`'s order without a sort, over the one table in the runtime that grows without
   bound.

## Stage 5 — the three seams

One file set: `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-cli/src/worker.rs`.

1. **`runs` gains SQLite in its true arm**, and the refusal test at
   `crates/nvs-stdlib/src/queue.rs:3410-3435` moves with the seam rather than after it — it is
   already written to read `runs` rather than a second list, and its `driver ==
   nvs_db::Driver::SqlServer` assertion about the filtered index is already written to survive this
   goal. `no_dialect` keeps its SQL Server arm, spelled, and loses its SQLite one.
2. **`Queued` gains a third arm of its own rather than joining `Framed`.** `SqliteConn::query` takes
   an owned `Vec<SqliteValue>` where the framed drivers take `&[Option<&[u8]>]` of already-encoded
   wire bytes, and `crates/nvs-db/src/sqlite.rs:304`'s `encode` is the conversion. That difference
   is a type and not a dialect, which is why it cannot be flattened the way MySQL and MariaDB are.
3. **The worker's `Wire` and its own local `Dialect` gain a third arm**, and `open`'s `SqlServer |
   Sqlite` refusal arm keeps SQL Server alone — still spelled rather than left to a `_`, so a
   driver gaining a send path arrives as a build failure instead of a refusal that has stopped being
   true.

## Stage 6 — the cases, the matrix leg, and the reference

1. **`tools/db-matrix.py` already has the SQLite leg** — "a scratch file, no container"
   (`tools/db-matrix.py:165`) — so this is the first queue work whose own half runs on a machine
   with no daemon at all. The suite list that leg runs is what this stage extends.
2. `crates/nvs-stdlib/tests/queue.rs` holds its own two-armed `Dialect` at line 313, beside the
   worker's; both become three.
3. The conformance cases for the members, beside goal `queue-purge`'s. **No differential case** —
   PHP has no queue, which is `examples/queue.nvs`'s own reason for having none.
4. `examples/queue.nvs` is goal `database`'s and its lines are frozen; a fixture this goal needs is
   its own file beside it.

## Standing decisions

- **This goal has one ADR slot**, taken by the first slice of stage 2: SQLite's claim, the
  transaction primitive underneath it, and where that primitive sits. It is worth a number rather
  than a statement comment because "the mutual exclusion is the single writer, and that is why there
  is no locking clause" is precisely the sentence a later contributor will try to correct toward
  `skip locked`. Nothing else in this goal opens one.
- **`rule:concurrency/claiming-is-one-statement` names the mechanism and this goal implements it
  rather than re-opening it.** A session that believes a single `update … where id = (select …)
  returning …` is the better claim has found a real alternative, and it is still not this goal's to
  take unilaterally: the fragment names the immediate transaction, so changing it is an edit to that
  fragment through a record whose `changes:` block names it. The default is the rule.
- **Every `Split` this goal writes runs inside stage 2's transaction, and none outside one.** The
  deferred-`BEGIN` upgrade is the trap that costs a session a day, because it never reproduces on
  one connection. If a statement can be written as one statement it does not need the transaction —
  but a pair always does.
- **No build option is turned on.** A queue statement that parses only under a non-default
  `libsqlite3-sys` build is a statement that stops parsing the day the dependency moves, and the
  bound `purge` owes has a portable spelling already written on the PostgreSQL side.
- **The bound on `purge` is the rule and the dialect spelling is not**, carried unchanged from goal
  `queue-purge`: dropping it is the one answer that is not available.
- **SQL Server stays out**, and `no_dialect`'s SQL Server arm and its "filtered index" sentence stay
  exactly as they are. A session that finds itself writing a fourth dialect has left the goal.
- **What this spends**, per `rule:programs/memory-priority`: nothing per job on the request path and
  no new allocation the queue was not already making. The claim costs one transaction's worth of
  round trips where PostgreSQL costs one statement, which is `Split`'s existing trade and already
  paid on MySQL and MariaDB; here the round trips are function calls into a file rather than a
  socket, so it is the cheaper end of that trade rather than a new one.
- **The concurrency tradeoff is stated, not designed around.** SQLite has one writer. Workers above
  one against a SQLite block serialize on the database's write lock and wait out the busy timeout
  rather than proceeding in parallel, so `[queue] workers` buys throughput there only up to the
  point the file's single writer allows. That is a property of the database and not a defect of the
  queue, and this goal adds no lock, no shard and no second file to work around it. What it owes is
  a sentence where an operator reads `workers`, saying where the number stops buying anything — not
  a new configuration key.
