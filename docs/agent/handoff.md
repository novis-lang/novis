# Handoff

## State

Goal `core-db-transaction-and-1-more`: `Core\Db\Transaction` is finished — all eight members carry
their feature proofs and `python tools/dossier.py --gate --group 'Core\Db\Transaction'` is green.
Of `Core\Db\Write`'s three readers, `affected` is finished; `changed` and `lastId` are the rest of
the goal.

`affected`'s attack found a real bug, and it is fixed. On SQLite a statement whose kind carries no
count — a `create table`, a `drop table` — answered the *previous* data-changing statement's row
count, because `sqlite3_changes` reports the last DML on the connection rather than the statement
that just ran. `crates/nvs-db/src/sqlite.rs`'s `step` now reports `0` unless `sqlite3_total_changes`
moved, which is what `Core\Db\Write::affected`'s reference card says and what the other four drivers
already do from their command tags.

What that fix does **not** reach is `changed`. On SQLite it is `Some(0)` where the card says `null`
for a kind that carries no count at all, because this driver has no tag to read the absence from.
That is `changed`'s own slice to decide, and the first item below names it.

The whole `Core\Db\Transaction` group was re-measured in one quiet sweep: the first `transaction`
record was taken while a release build held the target directory, so `--record-perf --force` replaced
it. `transaction` is 54024 ns/op, 5.00 statements and 49.10 allocations — half of `stream`'s clock
and about one and a half times `rollBack`'s, which is a `BEGIN`, a `SAVEPOINT`, a write, a `RELEASE`
and a `COMMIT` per round. `Core\Db\Write::affected` is 41.2 ns/op at 0.00 allocations, as its bench
declares.

`target/release/nvs.exe` is current with the tree.

## Next group

**Stage: feature proofs for the last two `Core\Db\Write` readers** — one file set:
`crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/execute.rs`,
`crates/nvs-db/src/sqlite.rs`, the root `nvs.toml`, and the proof trees under `core/Db-Write/changed/`
and `core/Db-Write/lastId/`. Both readers come off the same builder and the same statement, so their
examples and their benches are cheapest written together.

- [ ] **`Core\Db\Write::changed`** — owes about, examples, hostile, perf, tests. The registry row is
      at `crates/nvs-stdlib/src/db/registry.rs:1496` and the reader at
      `crates/nvs-stdlib/src/db/row.rs:1252`; the slot is filled per driver, and SQLite's half is
      `sqlite_write` at `crates/nvs-stdlib/src/db/execute.rs:955`. Decide there what a statement with
      no count of its own answers on this driver — `Some(0)` today, `null` on PostgreSQL.
      `rule:core-classes/db-statement-members` and `rule:testing/feature-proofs`.
- [ ] **`Core\Db\Write::lastId`** — owes about, examples, hostile, perf, tests. The row is at
      `crates/nvs-stdlib/src/db/registry.rs:1505` and the reader at
      `crates/nvs-stdlib/src/db/row.rs:1261`. SQLite reads it from
      `crates/nvs-db/src/sqlite.rs:1283`'s `last_insert_rowid()`, which carries the staleness
      `affected` had: a statement that inserted nothing answers the id of the last one that did.
      `tests/conformance/core/db-last-id-belongs-to-the-write-and-not-the-connection.nvst` is what
      already pins the member. `rule:core-classes/db-statement-members`.

## Backlog

- `Core\Db\Write::changed` cannot report an absent count on SQLite — decide it in that slice,
  `crates/nvs-stdlib/src/db/execute.rs`'s `sqlite_write`.
- `Core\Db\Write::lastId` has SQLite's stale-value hazard that `affected` just lost — `lastId`'s
  slice either folds it or records why it cannot.
- A proof program still cannot open a second database flow, so a hostile case abandons at most one
  walk and that step goes last — the playbook bullet owns it.
