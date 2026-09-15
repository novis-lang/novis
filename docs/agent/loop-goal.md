---
milestone: M8
---
# Loop goal 58 — every Core\\Db and Core\\Queue member answers on all five drivers

The goal is M8's database half finished on all five drivers. `stream` and `streamAs` walk a result in
constant memory on all five, or refuse on one driver with a sentence a record wrote. `serverVersion`
answers what the server said. `queryAs<T>` says its *no* while compiling, and a `decimal`, an `Instant`
or a `bytes` field hydrates rather than landing on `Opaque`. `Core\Queue` runs on SQL Server, and a
dead-lettered job keeps every attempt's error. The five-driver matrix runs over the socket transport
too, and the CI workflow runs it.

## Why here

This goal comes after goal `m7-server-surface`, and it is the first of M8's two goals. It takes the
database half and goal `m8-stdlib-depth` takes the rest, split by file set: `nvs-db`, `nvs-stdlib/src/db/`
and `queue.rs` share nothing with the stdlib roster. It sits in front of goal `unowned-closures`, which
edits the same `nvs-db` schema files once a user decision sheet has settled them. It also sits in front
of goal `gap-zero`, because a register is emptied only after everything that adds to it has run.
Before this goal, the stage 5 work of the old `gap-zero` goal was this work, filed under the wrong
goal.

What it needs is already built. `stream` works on PostgreSQL over a parked cursor
(`crates/nvs-db/src/pg.rs:2575` `PgCursor`, `:741-777`). `Core\Queue` works on four backends
(`crates/nvs-stdlib/src/queue.rs:2368` `runs`), with the worker's SQLite arm at
`crates/nvs-cli/src/worker.rs:331` and `:455`. `queryAs<T>`'s compile-time pass exists
(`crates/nvs-types/src/derive.rs:713` `check_row_sites`, `E0806`). The matrix harness exists
(`tools/db-matrix.py`).

## Stage 0 — the catch-up

These sentences are already wrong, or will be once this goal takes them over. Correct each one first.
The anchors are where each sentence was on the day this goal was written, so re-grep before editing.

1. **Owner tags that name the old goal.** `crates/nvs-stdlib/src/db/mod.rs:290` and `:307`, and
   `crates/nvs-stdlib/src/queue.rs:109`, say `— owner: gap-zero`. `crates/nvs-stdlib/src/queue.rs:85`
   says `— owner: unowned-sweep`, a retired goal. `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:35`
   says `# carried-gaps`, also retired. Every one of these becomes `m8-db-queue`. The M8-tagged gaps at
   `crates/nvs-types/src/derive.rs:71`, `:83` and `crates/nvs-cli/src/worker.rs:101` take the same tag,
   so each owner gate names the goal that is doing the work.
2. **`crates/nvs-stdlib/src/queue.rs:101-108` gap 4 is half stale.** It says the worker has no SQLite
   arm, but `crates/nvs-cli/src/worker.rs:331`, `:455` and `:1244` show one, and
   `crates/nvs-stdlib/tests/queue_sqlite.rs` runs claims against it. Rewrite the gap whole, about SQL
   Server alone.
3. **`crates/nvs-stdlib/src/db/mod.rs:291-307` gap 4 is mostly stale.** Three refusals are already
   compile-time `E0806`, from `crates/nvs-types/src/derive.rs:713-765` (called at
   `crates/nvs-types/src/check.rs:221` and pinned by `crates/nvs-types/tests/derive.rs:500-614`): a list
   `T`, a `T` without `#[Db\Derive]`, and a mapping that cannot fill the constructor. Only the
   `Opaque`-erased field is still refused per row. Rewrite the gap to say so. Stage 6 closes what is left.
4. **`crates/nvs-stdlib/src/queue.rs:64-85` gap 1 no longer owes the secret refusal.**
   `crates/nvs-stdlib/src/queue.rs:48-55` and `nvs_types::expr::quals::reject_secret_enqueued_argument`
   (`crates/nvs-types/src/expr/quals.rs:677`) refuse a secret, and
   `tests/conformance/reject/queue-push-refuses-a-secret.nvst` pins it. Only `limits` and `grants` are
   left, and stage 8 builds them.
5. **`crates/nvs-types/src/derive.rs:72-83` gap 2 says `fromRow` needs `Core\Db\Row` to exist.** It
   exists, and `crates/nvs-stdlib/src/db/row.rs:122` `hydrate` is the decoder walk. Rewrite the gap to
   name what is left: gap 1's erased types.
6. **`docs/agent/playbook.md:3880`'s bullet says `Core\Queue`'s statements exist "for PostgreSQL and
   MySQL only".** They exist for four backends (`crates/nvs-stdlib/src/queue.rs:2368-2375`). Edit the
   bullet in place, and edit it again when stage 7 adds the fifth.

## Stage 1 — the floor

Goal `m7-server-surface`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the record

One new record, and no other number. Its body is § *Standing decisions* below, argued, and it decides
four things:

1. **How each driver parks a read** for `stream` on MySQL, MariaDB, SQL Server and SQLite. This is
   the hole `rule:core-classes/db-one-api` leaves on purpose: it specifies § 4's behaviour and not
   the mechanism. For each driver the record states either the parked state, by analogy to `PgCursor`,
   or a **recorded refusal naming the driver**.
2. **What `serverVersion` answers on each driver**: the server's own string where it sends one, and
   the spelling where it sends numbers.
3. **A unique key reads nulls as distinct on every backend.** This is the vocabulary growth that
   `rule:core-classes/queue-storage-is-a-table` puts before SQL Server's queue dialect. SQL Server
   spells it as a filtered unique index.
4. **Where a job's earlier errors live**, so the dead-letter row carries the `errors` array that
   `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` asks for.

It creates three rules, all `designed`:

| Rule | Says |
|---|---|
| `core-classes/a-stream-parks-its-read-on-the-connection` | per driver, the read state that `stream` leaves on the connection between steps, or the recorded refusal for a driver that has none; never a buffer |
| `core-classes/server-version-is-what-the-server-said` | the string each driver answers, kept from the handshake, with no round trip |
| `core-classes/a-unique-key-reads-nulls-as-distinct` | the schema vocabulary's unique key has the same null semantics on all five, and SQL Server's DDL is a filtered index |

It modifies these existing rules:

- `core-classes/db-streaming`: its *Not shipped whole* paragraph goes.
- `core-classes/schema-plan`: the filtered index leaves its v1 exclusions, for this one shape.
- `core-classes/queue-storage-is-a-table`: its SQL Server paragraph becomes a statement of fact.
- `concurrency/attempts-are-finite-and-a-dead-letter-is-kept`: the column that carries `errors`.

## Stage 3 — the keystone: a parked read on the MySQL wire

This stage covers `crates/nvs-db/src/mysql.rs`, `crates/nvs-db/src/maria.rs`, `crates/nvs-db/src/conn.rs`
and `crates/nvs-stdlib/src/db/stream.rs`. MySQL and MariaDB share one text-protocol row loop, so one
parked state covers both, and it is the first proof that the record's shape holds beyond PostgreSQL.

1. **The parked state.** It is split from the borrow the way `crates/nvs-db/src/pg.rs:2555-2583`'s
   `PgCursor` is, and `State::Streaming` refuses the second statement under
   `rule:core-classes/db-connection-busy-state`. It is proved against a scripted server, the way
   `pg.rs:2592`'s generic `Wire` is.
2. **`stream_step` grows the arm**: `crates/nvs-stdlib/src/db/stream.rs:133` `stream_step` gains a
   `Connection::MySql`/`MariaDb` arm beside the PostgreSQL one at `:150`, and `:294` `unstreamed`
   stops naming PostgreSQL as the only driver that streams.
3. **The real-server half joins the matrix.** Add a new `crates/nvs-stdlib/tests/db_stream.rs`, gated
   on `NVS_DB_MATRIX_DRIVER` the way `crates/nvs-stdlib/tests/queue.rs` is, and list it in
   `tools/db-matrix.py:115` `SUITES`. The matrix runs only `nvs-db` and `--test queue` today, so without
   this nothing checks a stdlib stream against a server.

## Stage 4 — the other two: SQL Server and SQLite

This stage covers `crates/nvs-db/src/tds/rows.rs`, `crates/nvs-db/src/tds/mod.rs`,
`crates/nvs-db/src/sqlite.rs` and `crates/nvs-stdlib/src/db/stream.rs`. It is one slice per driver, and
the two drivers share nothing except `stream_step`.

1. **SQL Server.** The row tokens are read up to `DONE` (`crates/nvs-db/src/tds/rows.rs:251`, `:751`),
   with the column metadata parked.
2. **SQLite streams, on one pinned thread per open walk** — the user's call, 2026-09-13.
   `crates/nvs-db/src/sqlite.rs:33-40` gives the reason SQLite materializes rows today: `rusqlite`'s
   `Rows` borrows the statement, the statement borrows the connection, and the handle is an
   `Arc<Mutex<Connection>>` used from a pool thread. A `stream` walk therefore owns a thread for its
   life: the statement is stepped there and each row handed across, and the thread is released when the
   walk ends, is dropped, or its task ends. That is O(open streams), never O(requests served). A
   refusal on SQLite is **not** an option this goal may take. That module doc's closing sentence, "§ 4's
   `stream` is where that bound has to become a chunk size", contradicts
   `rule:core-classes/db-streaming`'s refused `chunk`, so it is rewritten whole. SQLite is the driver a
   `.nvst` case can reach (a `:memory:` block), so the streaming proof lives there first.

## Stage 5 — `streamAs` and `serverVersion`

This stage covers `crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/stream.rs`,
`crates/nvs-stdlib/src/db/row.rs`, `crates/nvs-stdlib/src/db/mod.rs`,
`crates/nvs-stdlib/tests/spec_registry_coverage.rs`, and each driver's handshake.

1. **`streamAs<T>`** is `stream` at a written type, exactly as `queryAs` is `query` at one. It goes on
   both `Queryable` classes (`crates/nvs-stdlib/src/db/registry.rs:346-375`, `:426`), each row goes
   through `crates/nvs-stdlib/src/db/row.rs:122` `hydrate`, and `check_row_sites`
   (`crates/nvs-types/src/derive.rs:713`) asks the same question of it as of `queryAs`. Strike
   `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:35`.
2. **`serverVersion`** requires each connection to keep what it currently parses and throws away:
   - PostgreSQL's `server_version` `ParameterStatus`, dropped at `crates/nvs-db/src/pg.rs:1157`;
   - the MySQL/MariaDB greeting string, parsed into a `(u16, u16, u16)` at `crates/nvs-db/src/mysql.rs:814` and `:866`;
   - TDS's `LOGINACK` version, read at `crates/nvs-db/src/tds/stream.rs:202-222`;
   - SQLite's library version.

   The row goes on `CONNECTION` (`crates/nvs-stdlib/src/db/registry.rs:207-221`, and
   `BEYOND_QUERYABLE` at `:434`).
3. **Close the hole that let `serverVersion` go unlisted.** `part_two_members`
   (`crates/nvs-stdlib/tests/spec_registry_coverage.rs:951`) reads a table only when its first header
   cell is `Member`. Spec § 18's `| Type | Members beyond Queryable |` table
   (`docs/spec/01-core-library.md:1245-1248`) is therefore never read, so `Connection::close`,
   `driver`, `serverVersion`, `isOpen` and `Transaction::rollBack` are enumerated by nothing. Teach
   the walk that table's shape, with a named test proving it does.

## Stage 6 — the row decoders

This stage covers `crates/nvs-types/src/derive.rs`, `crates/nvs-stdlib/src/db/row.rs`,
`crates/nvs-stdlib/src/db/column.rs`. It is one erasure with two doors, and this goal takes the **row**
door only: the JSON door (`crates/nvs-stdlib/src/json.rs` gap 1) is goal `m8-stdlib-depth`'s, built to
the wire form the user's decision sheet fixed. The `CodecTy` arms both doors share land here, since this
goal runs first.

1. **Every codec-reachable type gets a decoder instead of `CodecTy::Opaque`**
   (`crates/nvs-types/src/derive.rs:44-71` gap 1). That covers `decimal`, `Instant` and `bytes` on the
   row side (under `rule:core-classes/db-column-types`'s map); the JSON side is goal `m8-stdlib-depth`'s.
   An inline shape has no column form, so on the row side it becomes
   `E0806` at the call site, which is the last per-row refusal of
   `crates/nvs-stdlib/src/db/mod.rs:291` moving to compile time.
2. **`fromRow`** (`crates/nvs-types/src/derive.rs:72-83` gap 2). The run-time walk
   (`crates/nvs-stdlib/src/db/row.rs:122`) *is* the generated decoder, observably. What is left is a
   class that writes `Core\Db\Codec::fromRow` by hand (`crates/nvs-types/src/derive.rs:1257`): a
   `queryAs` over it must call that method. Check whether it does, and close or strike the gap with
   the evidence.
3. **ADR 0054's M8 check, its database half** (`docs/decisions/0054.md:251-252`): a `NUMERIC(30,10)`
   PostgreSQL column throws on read rather than truncating. The JSON half is goal `m8-stdlib-depth`'s
   stage 5.

   Neither half exists (a grep for `30,10` and `30, 10` across `crates/` and `tests/` finds nothing). The
   first is a unit test over the text form `column_value` converts, plus a
   `crates/nvs-stdlib/tests/db_stream.rs`-style matrix case against the real server.

## Stage 7 — the queue on SQL Server, and the `errors` array

This stage covers `crates/nvs-db/src/ddl.rs`, `crates/nvs-db/src/schema.rs`,
`crates/nvs-db/src/catalog.rs`, `crates/nvs-stdlib/src/queue.rs` and `crates/nvs-cli/src/worker.rs`.

1. **The vocabulary first.** Stage 2's unique key emits a filtered unique index on SQL Server
   (`crates/nvs-db/src/ddl.rs:641-676`, where `:656` spells `ADD CONSTRAINT … UNIQUE` today), and the
   catalog reads it back as the same key, so `plan` converges instead of re-proposing it. This goal
   edits those files and does not close their `unowned` gaps, which belong to goal `unowned-closures`.
2. **Then the dialect.** Write SQL Server texts for every statement the queue sends, make
   `crates/nvs-stdlib/src/queue.rs:2368` `runs` answer `true` for all five, and have `:2460`
   `no_dialect` lose its last live arm. The worker needs the same:
   - a `Dialect` arm for TDS (`crates/nvs-cli/src/worker.rs:1348`);
   - an `open` arm in place of `:1248-1251`, retiring `:1261` `sql_server_gap` and its test at `:1620`.

   Tests go in `crates/nvs-stdlib/tests/queue.rs`, which already runs under the matrix.
3. **The `errors` array** (`crates/nvs-cli/src/worker.rs:93-101`). Each failed attempt's entry is
   kept on the jobs row (stage 2's column), and the dead-letter move
   (`crates/nvs-stdlib/src/queue.rs:877` `dead_errors`, `crates/nvs-cli/src/worker.rs:944-968`) copies
   all of them. `crates/nvs-stdlib/src/queue.rs:313` `schema` gains the column as a nullable `Safe`
   step, for the reason `rule:core-classes/queue-storage-is-a-table` gives.

## Stage 8 — a job's budget and grants

This stage covers `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-types/src/expr/isolate.rs` and
`crates/nvs-cli/src/worker.rs`.

`crates/nvs-stdlib/src/queue.rs:64-85` gap 1: `push` declares `grants` (a list of capability names, as
`nvs.toml` spells them) and `limits` (one option per sub-cap) as ordinary options. They are recorded on
the row at enqueue, and the worker applies the narrowing to the isolate it starts for the job
(`rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue`,
`rule:concurrency/a-job-runs-as-a-root-isolate`). The narrowing only ever narrows. A grant the
enqueuing request does not hold is refused at `push`. `crates/nvs-types/src/expr/isolate.rs:143-160`'s
`E0777` refusal of `limits:`/`grants:` on `Core\Isolate::spawn` is lifted by goal `m5-proofs`, which runs
first and builds that narrowing; this stage reuses it for a job rather than writing a second one.

## Stage 9 — the matrix's socket leg, and CI

This stage covers `tests/db/compose.yaml`, `tools/db-matrix.py`, `crates/nvs-db/src/matrix.rs`,
`crates/nvs-db/tests/handshake.rs`, `.github/workflows/ci.yml` and `tools/ci-changes.py`.

1. **The socket leg** (`crates/nvs-db/src/matrix.rs:43-54` gap 1). The PostgreSQL, MySQL and MariaDB
   services each publish their socket directory onto the host. `redis` already does this at
   `tests/db/compose.yaml:100-109` and `:288-303`, so copy its shape. `tools/db-matrix.py` then sets
   `SOCKET_VAR` and runs each of those drivers' suites a second time over `AF_UNIX`, printing a line of
   its own after the TCP legs. This covers the transport `rule:core-classes/db-unix-socket-path`
   specifies. SQL Server and SQLite have no socket leg, and the tool says so rather than printing a
   green line for them.
2. **The CI leg.** `.github/workflows/ci.yml` gains a job that brings the compose services up and
   runs `python tools/db-matrix.py --all` on `ubuntu-latest`, gated by a new lane in
   `tools/ci-changes.py:34` `LANES` that covers the database paths and `.github/workflows/`. CI is not
   running while the user fixes a billing block, so the check reads the workflow file and the lane
   tool locally, and never waits on a run.

## Stage 10 — the rulebook and the module docs

Flip stage 2's three rules to `shipped`, with `guardedBy` filled from this goal's cases and tests, and
run `python tools/rules.py --render`. Rewrite each module doc whose gap closed as a whole (AGENTS.md
rule 6): `crates/nvs-stdlib/src/db/mod.rs` gaps 3–4, `crates/nvs-stdlib/src/db/stream.rs`'s module doc,
`crates/nvs-stdlib/src/queue.rs` gaps 1 and 4, `crates/nvs-types/src/derive.rs` gaps 1–2,
`crates/nvs-cli/src/worker.rs` § *Known gap*, and `crates/nvs-db/src/matrix.rs` gap 1. The owner gates
are satisfied once no `— owner: m8-db-queue` tag is left on the tree.

## Standing decisions

- **Settled with the user: M0–M8 are complete when this chain ends.** Every promise a past milestone's
  plan made is built. An item is deferred to M9 or later only if it *cannot be built* until that
  milestone's work exists, never because it is large. Nothing in this goal meets that bar, so nothing
  in it is deferred.
- **Design calls are decided under ADR 0004's ordering and recorded, never `BLOCKED`.** The ordering
  is security, then PHP-compatible correctness, then request-path latency, then simplicity, then
  memory. The home is stage 2's record, or the module doc that owns the code.
- **One new record, and no other number.** It is stage 2's. The new record claims its number when it
  lands, so no stage names one.
- **`stream` answers on all five drivers, and never buffers behind the caller's back.** Buffering
  breaks the member's one promise, constant memory, and § 4's *uniform* busy rule
  (`rule:core-classes/db-streaming`). **SQLite streams on one pinned thread per open walk** — the user's
  call, 2026-09-13 — so SQLite takes no refusal. For a wire driver whose protocol turns out unable to
  park a read, the fallback is still a `RuntimeError` naming the driver and `query`, stated in the
  record — never a buffer.
- **`serverVersion` is the server's own string and costs no round trip.** It is what the handshake
  already delivered, stored on the connection. Where a server sends numbers (TDS's `LOGINACK`), the
  record fixes the spelling once. A driver never issues `SELECT version()` for it.
- **A unique key's nulls are distinct on every backend.** That is the SQL standard and four of the five
  backends, and SQL Server is brought into line through its DDL rather than the queue working around
  it. The alternative that `rule:core-classes/queue-storage-is-a-table` refuses (a `not null` column
  with a generated token) stays refused. A program's own `Core\Db\Schema` on SQL Server gains the same
  semantics, and that is the correctness fix `rule:core-classes/db-one-api`'s one-API promise asks
  for, not a side effect. The safe fallback, if a filtered index cannot be read back from the catalog
  as the same key, is to refuse `plan` on a SQL Server table holding a nullable unique column. It
  never emits DDL that will not converge.
- **`queryAs<T>`'s last run-time refusal becomes `E0806`**, the code that already carries the other
  three (`crates/nvs-diagnostics/src/lib.rs:3374-3391`). No new code is spent. The run-time checks in
  `crates/nvs-stdlib/src/db/row.rs:130-135` stay as a backstop for a class built by hand, and each one
  says so.
- **A `decimal` column that does not fit is refused, never narrowed** (`rule:types/decimal`). A
  `NUMERIC` value past the decimal's mantissa or scale throws naming the column.
- **The `errors` array is bounded by construction.** Attempts are finite
  (`rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`), and each entry's message is
  capped at the length `dead_errors` already writes, so a row's growth is at most attempts × cap.
- **Queue proofs run against real servers, not `.nvst` cases**, because no `.nvst` case can reach a
  live queue. Queue dialects are proved in `crates/nvs-stdlib/tests/queue.rs` under
  `python tools/db-matrix.py --all`. A `.nvst` case reaches SQLite alone, through a `:memory:` block.
- **Every check runs locally.** CI is not running (the user is fixing a GitHub billing block), so the
  CI leg is proved by a `command` check that reads the workflow file and the lane tool, and nothing
  waits on a remote run.
- **What it spends.** Everything is per open connection or per job row; no new spending is per core or
  per process.
  - A streaming connection holds its driver's parked read state, the column descriptions and one
    decode buffer, which is what PostgreSQL already spends. That *reduces* what a large read holds on
    four drivers, whose alternative today is `query`'s whole result set.
  - `serverVersion` is one short string per open connection.
  - A SQLite `stream` walk holds one pooled thread for its life, released when the walk ends or its
    task does — O(open streams), never O(requests served).
  - The `errors` column is at most attempts × the capped message per job row, released when the row
    is deleted.
  - The filtered index is one index per SQL Server queue table.
  - The decoders are code.
- **ADR slots**: the one record of stage 2.
- **Not this goal**:
  - the `unowned`-tagged gaps in these files (`crates/nvs-stdlib/src/db/mod.rs` gaps 1–2,
    `crates/nvs-stdlib/src/queue.rs` gaps 1–2, and `nvs-db`'s `catalog.rs`, `ddl.rs` and `schema.rs`),
    which are goal `unowned-closures`'s, after its decision sheet;
  - `crates/nvs-types/src/derive.rs` gap 3, `unowned` and `rule:core-classes/derive-attribute`'s to
    answer;
  - `Core\Decimal`'s `divExact`/`divRound`/`allocate` and the rest of the stdlib roster, which are
    goal `m8-stdlib-depth`'s;
  - a `chunk` option on `stream`, refused by `rule:core-classes/db-streaming`.

  A session that finds one of these on its path writes it to the handoff's `## Backlog`.
