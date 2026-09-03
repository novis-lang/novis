# Handoff

## State

**M8 goal 5, and the acceptance check is green: `python tools/db-matrix.py --all` is 5/5.** The
failure recorded after session 0005 was the cold-tree first boot it said it was.

**§ 2's roster, §§ 1 and 6's `status` and § 4's `skip locked` now answer on the framed dialect** —
three cases in `crates/nvs-stdlib/tests/queue.rs`, each proven to *run* by breaking an assertion once.
`STATUS_POSTGRES` and `STATUS_MYSQL` are `pub` for that: nothing outside the module runs a `status`,
but the matrix target has to be able to send one.

**The framed cases serialize on `crates/nvs-stdlib/tests/queue.rs:139`'s `FRAMED_WRITES`, and that is
a fixture fix a green leg was hiding.** InnoDB locks what a statement scans; the playbook's *Writing a
test case* bullet owns the shape and the measurement (2 failures in 8 runs before, 13 clean runs
after).

**One real dialect divergence is now pinned rather than assumed.** With a holder inside a transaction,
MySQL answers a second worker's claim with *nothing* where PostgreSQL answers the next due job:
`for update` locks the rows the `select` examined and the `limit` applies after the sort. The case
asserts § 4's property — answered rather than blocked, and never the held row — plus that the deferred
row is claimable once the holder commits, and deliberately does not pin the plan.

**Two texts have still never been parsed by a server**, and `crates/nvs-stdlib/src/queue.rs`'s gap 5
says what each costs: `CANCEL_MYSQL` is `SUCCEEDED_MYSQL`'s shape, `COUNTS_MYSQL` is the one with a
construct nothing else here has.

**A citation warning for the next session:** this module says "§ 5's three readers" for `status`,
`cancel` and `stats`, but ADR 0084 § 5 is *Running a job* — the roster is § 2's own doc's wording and
`status` is §§ 1 and 6's. Name the constant, not the section, until someone decides which numbering
that shorthand belongs to.

**`[context]` gap, three sessions old:** `adrs` carries no ADR 0084 section at all. Add `0084 §1`,
`§2`, `§3`, `§4` and `§6`; § 5 is not the one this goal's items mean.

## Next group

**One file set: `crates/nvs-stdlib/tests/queue.rs`, with `crates/nvs-stdlib/src/queue.rs` read at the
constant each item names.** Every case skips with no `NVS_DB_MATRIX_DRIVER`. Take the first — the
other two are one case each over helpers that already exist.

- [ ] **§ 3's two transactional cases on the framed dialect** (0084 § 3). The PostgreSQL twins are
      `crates/nvs-stdlib/tests/queue.rs:1074` and `crates/nvs-stdlib/tests/queue.rs:1156`. The
      prerequisite is `crates/nvs-stdlib/tests/queue.rs:426`'s `orders`, whose `create` is
      PostgreSQL's spelling and whose own doc says a case needing it elsewhere splits the statement
      the way `MIGRATION_MYSQL` splits § 2's — `bigserial` is the one construct with no framed
      spelling.
- [ ] **`COUNTS_MYSQL` meets a server** (0084 § 6). `crates/nvs-stdlib/src/queue.rs:897`; it needs
      `pub` as `STATUS_MYSQL` now has. The claim worth a server is `count(case when … then 1 end)`
      answering `0` over no rows, and `cast(… as signed)` over the `sum` MySQL answers as a
      `decimal` — one aggregate row read four ways.
- [ ] **`CANCEL_MYSQL` meets a server** (0084 § 1). `crates/nvs-stdlib/src/queue.rs:856`; the whole
      claim is that `and state = 0` is enforced by the statement and read through the affected count,
      so a cancel that lost the race to a claim answers `0` rather than throwing.

## Backlog

- The `[context] adrs` gap above — `docs/agent/loop-goal.toml`.
- `crate::db`'s gap 2: the two drivers that send no statement — `crates/nvs-db/src/lib.rs`.
- `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` waits on a TDS driver — same gap.
- The "§ 5's three readers" numbering, in three places — `crates/nvs-stdlib/src/queue.rs`.
