# Handoff

## State

**Stage 8's live half is landed.** `crates/nvs-stdlib/tests/queue.rs` runs ADR 0084 § 1's `INSERT`,
§ 4's `CLAIM` and § 6's `DEAD_LETTER` against `tests/db/compose.yaml`'s PostgreSQL and asserts three
of the stage 8 check's six names: the dead-letter move from both sides (the row is in one table
rather than in neither), the visibility bound from both sides (held inside the window, returned with
an attempt spent past it), and `skip locked` against a second connection inside an open transaction,
where `statement_timeout` turns a block into a failure instead of a hung leg.

**Those cases are in `nvs-stdlib`, not the `nvs-db` the check named, and two things forced it.** ADR
0132 § 1 fixes the crate edge — a test target in `nvs-db` cannot name `nvs_stdlib::queue` — and
`tools/db-matrix.py` ran only `-p nvs-db`, so a case anywhere else silently skipped on every driver
leg. It now runs a `SUITES` list, and stage 8's check is `-p nvs-stdlib` with the reason in its
comment. The playbook bullet owns the trap.

**Stage 2's `-p nvs-db` check cannot go green in this goal-run, and its ledger line is not this
session's failure.** `local_infile_is_refused_and_no_file_is_sent` is MySQL's, and stage 2's own
comment forbids a second driver until PostgreSQL is green end to end — so that check reports "did
not run" permanently, and closing the name it currently reports only moves the report to that one.
The name it reports now, `a_named_connection_is_memoized_for_the_request`, has no test anywhere and
is `Core\Db::connect`'s, so `nvs-stdlib`'s; it is the third item below.

**The `errors` array is one entry deep by decision, not by omission** — `MIGRATION`'s doc comment
owns the trade and `crates/nvs-stdlib/src/queue.rs:400`'s `dead_errors` owns the entry's shape.

## Next group

**The file this session created, plus one statement's home — `crates/nvs-stdlib/tests/queue.rs`
(helpers: `push` at :175, `claim` at :198, `rows`/`one`/`apply` at :128, :156 and :166) and
`crates/nvs-stdlib/src/queue.rs`. The third item leaves that set for `crates/nvs-stdlib/src/db.rs`.**

- [ ] **An enqueue commits with the write that made it, and a rolled-back write leaves no job** —
      ADR 0084 § 3, two names in `docs/agent/loop-goal.toml:2964`. Push with
      `crates/nvs-stdlib/src/queue.rs:249` inside a transaction opened by
      `crates/nvs-db/src/pg.rs:698`, close it with `crates/nvs-db/src/pg.rs:735`, and count the rows;
      then the same push under § 7's `ROLLBACK` (`crates/nvs-db/src/pg.rs:739`) and count zero. The
      property is § 3's whole reason for the design, so assert it over one connection — a second one
      would be testing a different design.
- [ ] **Retries are bounded and the backoff is jittered** — § 6,
      `docs/agent/loop-goal.toml:2969`. `crates/nvs-stdlib/src/queue.rs:434`'s `retry_at` and :457's
      `jitter` are pure, so this one needs no server: assert the ladder doubles, that it stops at
      `RETRY_CAP_MS` (:416), and that two ids at the same attempt land on different delays. It
      belongs in that module's own `#[cfg(test)]` block at :1764, not in the live file.
- [ ] **A named connection is memoized for the request** — ADR 0067 § 2, the name stage 2's check
      reports (`docs/agent/loop-goal.toml:2764`). `crates/nvs-runtime/src/ctx.rs:3129`'s
      `memoized_connection` is the mechanism and `crates/nvs-stdlib/src/db.rs`'s `open_named` is the
      caller; the check's `args` is `-p nvs-db`, which cannot host it, so the name moves to a
      `-p nvs-stdlib` check in the same edit — and `docs/agent/goals/5-database.toml` is copied from
      the live file afterwards.

## Backlog

- `no_driver_path_interpolates_a_value_into_sql` — stage 2's remaining writable name, ADR 0067 § 1.
- `local_infile_is_refused_and_no_file_is_sent` — MySQL's, and blocked on that driver existing.
- Stage 5's other three names — `docs/agent/loop-goal.toml`, stage 5's block.
- `Core\Db::open` and `Core\Queue`'s `limits`/`grants` wait on a shape-parameter type —
  `crates/nvs-stdlib/src/queue.rs`'s *Known gaps* 1.
- `$args` is `mixed` and so does not refuse a `secret` — same gap list, 2.
