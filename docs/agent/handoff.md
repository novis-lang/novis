# Handoff

## State

**M8 goal 5, stage 10. The driver's failing acceptance check is closed**: the differential suite is
**256** against its floor of 255. The six new cases are `tests/differential/core/db-*.nvst`, each
running a live SQLite `Core\Db` with PHP's own `SQLite3` as the oracle — `execute`/`query`'s counts,
a bound value as data, § 7's closure against a hand-written `BEGIN`/`COMMIT`/`ROLLBACK`, `inList`
against the implode-of-question-marks idiom, `executeMany` against the loop it replaces, and
`connect`'s memoization as an `--ORACLE-DIVERGES--` because `mysqli_connect` has no behaviour here
to compare against.

**A `.nvst` case can run a real database, and this is the technique the next group rests on** — the
playbook bullet owns it, along with why the queue is the one thing it does *not* reach.

**The previous handoff's next group is not writable as it was specified, and none of it is missing
work.** Item 1's claim is already pinned one layer down by
`crates/nvs-stdlib/src/queue.rs:3204`'s `every_stats_counter_reads_the_slot_its_member_is_named_for`,
which sweeps all four counters across member name, slot roster and `*_AT` index — a `.nvst` could
only restate it, and could not run `stats()` at all. Items 2 and 3 are runtime queue claims and are
out of reach for the same reason. `python tools/gaps.py --coverage` ranks `Core\Queue\Stats` thin
because it counts `.nvst` cases, which is the wrong denominator for a class no case can call.

**Orientation gap, carried:** `[context]` still has no field that can name a `docs/spec/` file.

## Next group

**One file set: `tests/conformance/core/db-*.nvst`, each a live SQLite case over the block in the
playbook bullet, with the registry rows in `crates/nvs-stdlib/src/db.rs`.** These are the three
thinnest `Core\Db` members `python tools/gaps.py --coverage` names that are now reachable at all,
and each adds a boundary rather than another row of the same shape.

- [ ] **`Core\Db\Write::changed` against `affected`, over a write where the two differ.** The pair
      is the whole reason `changed` exists, and an update that sets a column to the value it already
      holds is where SQLite's own counting separates them. `crates/nvs-stdlib/src/db.rs:1711`
      (`changed`), `crates/nvs-stdlib/src/db.rs:1702` (`affected`), extending
      `tests/conformance/core/db-write-tells-an-absent-count-from-a-zero-one.nvst`.
- [ ] **`Core\Db\Column::nullable` read off a declared schema, both sides of the bound.** A `not
      null` column, a plain one, and a computed column the declaration does not describe — ADR 0067
      § 9's last sentence is what decides the third. `crates/nvs-stdlib/src/db.rs:1783`
      (`nullable`), `crates/nvs-stdlib/src/db.rs:1774` (`type`), beside
      `tests/conformance/core/db-columns-describe-a-statement-and-not-a-row.nvst`.
- [ ] **`Core\Db\Transaction::executeMany` rolls back whole where the connection's own does not.**
      § 4 makes a batch not a transaction; inside § 7's closure it is one, and the failing set
      therefore takes the sets before it with it. `crates/nvs-stdlib/src/db.rs:926`
      (`Transaction::executeMany`), `crates/nvs-stdlib/src/db.rs:762` (`Connection::executeMany`) —
      `tests/differential/core/db-execute-many-sums-the-counts-a-hand-written-loop-of-sqlite3stmt-execute-sums.nvst`
      pins the outside half already.

## Backlog

- The queue's depth items must change layer, not wording: `cancel` and `status`'s runtime claims are
  `-p nvs-stdlib` `#[test]`s or `tests/db/` fixtures (`crates/nvs-stdlib/src/queue.rs`).
- `[context]` gains no `docs/spec/` selector (`docs/agent/loop-goal.toml`).
- `python tools/gaps.py --differential` now names only `Core\Db::quoteIdentifier`, whose twin needs a
  connection PHP will not give without a server (`tools/gaps.py`).
- `Core\IO\Metadata` is the thinnest class in the tree at depth 1.0, and is outside this goal
  (`docs/agent/loop-goal.toml`).
