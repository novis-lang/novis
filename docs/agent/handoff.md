# Handoff

## State

**Goal `sqlite-queue`, stages 3 and 4 are green.** The gap was not the statements — those landed
with the stages — but where their cases live: the goal drafted `..._on_sqlite` names for
`crates/nvs-stdlib/tests/queue.rs`, and stage 2 wrote `crates/nvs-stdlib/tests/queue_sqlite.rs`
instead, which needs no server and runs under `python tools/verify.py` on every machine. That file
now holds the whole worker roster, and both checks name the cases the tree declares.

- **Goal stage 6 item 2 — "queue.rs's own `Dialect` gains its third arm" — is superseded and should
  not be taken.** `queue.rs` reaches a driver over a socket and its module doc says SQLite's
  statements are `queue_sqlite.rs`'s; a third arm there would re-run, behind the matrix harness,
  every case that already runs everywhere. The previous handoff named it as the next item; this one
  does not.
- **What is still open is stage 5's two checks**, and one of the three names in the `nvs-stdlib` one
  may be another re-point rather than a test — see the group below.
- Nothing is blocked, and no `[context]` field was missing from this session's pack.

## Next group

**Stage 5: the seams, on the statement side** — one file set: `crates/nvs-stdlib/src/queue.rs` and
the two `loop-goal.toml` copies, in this order, because the third item decides whether the second
is a test at all.

- [ ] **`no_sqlite_statement_binds_another_dialects_placeholder`** — the PostgreSQL twin is
      `crates/nvs-stdlib/src/queue.rs:4072` and the roster to read is `sqlite_texts()` at
      `crates/nvs-stdlib/src/queue.rs:4376`, which this session added for the purge-bound case
      beside it. `$1`, `::`, `returning`, `with ` and `for update` are what a SQLite text may not
      spell, under `rule:core-classes/db-drivers-are-an-enum`.
- [ ] **`runs_answers_true_for_three_drivers_and_false_for_sql_server_alone`** — the predicate is
      `crates/nvs-stdlib/src/queue.rs:2368`; nothing asserts it by name today. `rule:core-classes/queue-storage-is-a-table`
      is what makes SQL Server the one refusal, and the goal's § *Standing decisions* forbids a
      fourth dialect.
- [ ] **Decide whether `every_driver_the_queue_refuses_says_what_it_is_waiting_on` is a test or a
      re-point** — `crates/nvs-stdlib/src/queue.rs:4030`'s
      `the_queues_refusal_is_only_ever_about_a_driver_that_cannot_send` walks every driver and
      asserts which gap the sentence names, which is the same claim. If it is, correct the name in
      both goal toml copies rather than writing a second walk.

## Backlog

- Stage 5's `nvs-cli` check: `a_sqlite_block_opens_a_queue_worker`,
  `a_sql_server_block_still_starts_no_worker_and_names_the_driver` and
  `a_worker_reads_the_same_claim_columns_by_position_on_all_three_dialects` — none exists;
  `crates/nvs-cli/src/worker.rs:1201`'s `sqlite_wire` is the seam. Its own file set.
- Goal stage 6 item 1, `tools/db-matrix.py`'s SQLite leg: `SUITES` already runs `--test queue`,
  which skips. Adding `--test queue_sqlite` buys nothing as it stands — those cases open
  `mode=memory&cache=shared` — so what the leg is worth is `queue_sqlite.rs` opening
  `NVS_DB_MATRIX_PATH` when it is set, which is the only way this repository tests SQLite's real
  file locking.
- No valgrind leg sweeps the SQLite queue paths: `examples/queue-sqlite.nvs` is in `[valgrind] skip`
  because the sweep runs `nvs run <file>` with no `--config`.
- The fixture is unverified on the WSL leg; what is unchecked is SQLite's file locking on the drvfs
  mount. `docs/agent/commands.md` § WSL.
