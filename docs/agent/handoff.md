# Handoff

## State

**Goal 9's stages are whole; what moved is the queue's own gap 5 — which backend waits on what.**
`nvs queue migrate` converges `rule:core-classes/queue-storage-is-a-table`'s two tables on all five
drivers, so a driver `Core\Queue` refuses is waiting on § 4's statements and never on a schema. The
refusal, this module's gap 5 and `crate::db`'s gap 2 all said the opposite; all three now agree with
the tree, and `crate::db`'s gap 2 has no driver gap left at all.

**`nvs_stdlib::queue::runs` is the one roster** for which drivers the queue has statements for —
PostgreSQL, MySQL and MariaDB. `no_dialect`'s last arm is its complement, the src test asks it
rather than a second list, and `crates/nvs-stdlib/tests/queue.rs`'s leg gate calls it, so a fourth
backend is one exhaustive `match` to edit rather than three lists to remember.

**SQL Server's order is decided and recorded, and it was already the rule's:** the vocabulary grows
the filtered index `rule:core-classes/schema-plan` keeps out of v1 *before* `Core\Queue` gains a
fourth dialect, because two nulls are equal there and a plain unique key over `dedupe_pending` would
admit one released row rather than any number of them. So the fourth dialect is SQLite's, which
needs nothing added to the vocabulary at all. `python tools/verify.py` is green.

## Next group

**The queue's fourth dialect — SQLite, the one that needs no vocabulary first** — one file set:
`crates/nvs-stdlib/src/queue.rs` and `crates/nvs-stdlib/tests/queue.rs`.

- [ ] **§ 4's claim and § 6's move as a third text** — `rule:concurrency/claiming-is-one-statement`.
      SQLite has `returning` and no `skip locked`, and one writer at a time is what makes the second
      unnecessary rather than missing — so decide whether the claim is one statement like
      `crates/nvs-stdlib/src/queue.rs:357` (`INSERT_POSTGRES`) or a `Split` like
      `crates/nvs-stdlib/src/queue.rs:591` (`INSERT_MYSQL`), and write the six texts beside the pair
      they are read against.
- [ ] **`Queued` grows a third arm and `runs` a third `true`** —
      `rule:core-classes/db-drivers-are-an-enum`, at `crates/nvs-stdlib/src/queue.rs:1721`
      (`Queued`), `crates/nvs-stdlib/src/queue.rs:1689` (`queue_connection`) and
      `crates/nvs-stdlib/src/queue.rs:1668` (`runs`). `crates/nvs-stdlib/src/queue.rs:1750`
      (`no_dialect`) then loses its `Sqlite` arm and keeps SQL Server's, which is the one that names
      the vocabulary.
- [ ] **A leg the matrix can run it on** — `crates/nvs-stdlib/tests/queue.rs:83` (`endpoint`)
      returns `None` for anything that is not `Location::Server`, and SQLite is reached by path, so
      the gate a SQLite case needs is a second one rather than a widening of that predicate.

## Backlog

- The filtered index itself in `nvs_db::schema`'s vocabulary — `rule:core-classes/schema-plan` keeps
  it out of v1, so it is a record's decision rather than a session's.
- The other `gap N` citations across `crates/nvs-stdlib/src/` may carry the same inversion; one grep
  for `gap [0-9]` against each cited list would say.
- Goal 9's acceptance, `python tools/db-matrix.py --all`, last failed bringing four containers up on
  a cold tree rather than on anything in the tree.
