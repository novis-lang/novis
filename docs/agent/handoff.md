# Handoff

## State

**ADR 0067 § 11's `query` event is filed on either driver.** `MySqlRows`
(`crates/nvs-db/src/mysql.rs:2174`) carries a `QuerySpan` opened in
`start_statement` before the prepare — § 1's first round trip is part of what the caller waited —
counted per row in `next_row`, and finished at the result-set terminator or, for a statement with no
result set, at the status packet the execution answered with. `nvs-stdlib`'s `mysql_rows` hands back
`QueryWatch::taken`'s pair exactly as `postgres_rows` does, and `name_span` is now generic over the
`NamesConnection` trait so the block-naming rule has one home rather than one per driver.

**§ 4's `execute` and `executeMany` answer on a MySQL connection.** `execute` branches on
`filed_connection` into `postgres_write`/`mysql_write` (`crates/nvs-stdlib/src/db.rs:3701`), which
answer a `Written` pair — and MySQL's `lastId` of `0` maps to null, because § 4's field is `?uint`
and the zero is the protocol's spelling of absence. `executeMany` opens § 11's span at the
connection's own driver and calls either `execute_many`. `nvs_db::mysql::execute_many` costs one
prepare and **N round trips**, not PostgreSQL's one flush: a MySQL command restarts the packet
sequence id, so two in flight cannot be framed apart. What a caller observes is the same on both —
the writes before a failure stand, the sets after it are still attempted, the *first* refusal is the
answer — and a wire failure is the one thing that ends the batch early.

**Still PostgreSQL-only, and this is now all of the module's known gap 2**: § 7's `transaction`
(`postgres_of`, `crates/nvs-stdlib/src/db.rs:3568`) and `crate::queue`'s four members. MariaDB,
SQL Server and SQLite are refused by `driverless` before anything is sent.

**No MySQL path has met a server.** `verify.py` has no matrix leg, so what holds all of this is the
type checker plus six unit tests against `mysql.rs`'s scripted `Peer`; `tools/db-matrix.py` and
`crates/nvs-db/src/matrix.rs` are the leg that would.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for
a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its
own ADR, not a slice. Untouched this session.

**`orient.py` gaps:** `[context] adrs` wants ADR 0067 § 4 and § 11 — both slices this session were
specified by sections the pack did not print, for the second session running. § 7 is the next
group's.

## Next group

**§ 7's `transaction` over a MySQL connection, one file set:
`crates/nvs-db/src/mysql.rs` and `crates/nvs-stdlib/src/db.rs`, against
`crates/nvs-db/src/pg.rs`'s § 7 half. The driver first — the stdlib arm has nothing to call
until the three commands and the depth exist.**

- [ ] **§ 7's `BEGIN`/`COMMIT`/`ROLLBACK` and the `SAVEPOINT` nesting on `MySqlConn`** — ADR 0067
      § 7. `crates/nvs-db/src/pg.rs:2963` (`begin`), `crates/nvs-db/src/pg.rs:3008` (`commit`),
      `crates/nvs-db/src/pg.rs:3053` (`roll_back`), `crates/nvs-db/src/pg.rs:3128`
      (`begin_command`, which renders the two options) and `crates/nvs-db/src/pg.rs:3168`
      (`simple_command`, which opens the span each answers with) are the shapes;
      `crates/nvs-db/src/pg.rs:608` is `depth`. MySQL has no `COM_QUERY` path in this driver yet,
      so the first question is whether these go out as `COM_QUERY` or as prepared statements —
      `crates/nvs-db/src/mysql.rs:1571` (`start_statement`) is the only send path there is today.
- [ ] **`transaction` branches on the connection instead of demanding a `PgConn`** — ADR 0067 § 7.
      `crates/nvs-stdlib/src/db.rs:4709`, `:4716`, `:4765` and `:4788` are the four `postgres_of`
      calls the member makes; `crates/nvs-stdlib/src/db.rs:3568` is `postgres_of` itself, which
      after this owes only `crate::queue`.
- [ ] **A `.nvst` or matrix case over a MySQL write** — `crates/nvs-db/src/matrix.rs:1` names the
      `NVS_DB_MATRIX_*` fields and `tools/db-matrix.py` brings a server up. Nothing in this
      session's work has met one.

## Backlog

- `Core\Db::open`'s shape parameter needs a registry type — `nvs_stdlib::db` known gap 1, and it
  wants its own ADR.
- `crate::queue`'s four members are PostgreSQL-only — `nvs_stdlib::queue`.
- SQL Server and SQLite have no connect path — `nvs_db` module doc.
- The MySQL statement cache is invalidated by § 13's reset, untested against a server —
  `nvs_db::mysql`.
