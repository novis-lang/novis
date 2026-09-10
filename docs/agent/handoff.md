# Handoff

## State

**Goal `sqlite-queue`, stages 3 and 4 are green, and stage 5's `nvs-stdlib` half is now too.** All
three of that check's names resolve: two of them because this session wrote them, one because it
was already on disk under another name and the check now points at it.

- **Stage 5's two drafted names were both wrong about the tree, in different ways**, and both are
  corrected in `docs/agent/loop-goal.toml` and `docs/agent/goals/35-sqlite-queue.toml` alike.
  `every_driver_the_queue_refuses_says_what_it_is_waiting_on` was
  `the_queues_refusal_is_only_ever_about_a_driver_that_cannot_send` all along;
  `runs_answers_true_for_three_drivers_…` carried a count this goal grows, and is now
  `runs_answers_false_for_sql_server_alone`. The playbook bullet owns the second one.
- **Goal stage 6 item 2 — "queue.rs's own `Dialect` gains its third arm" — is superseded and should
  not be taken**, carried unchanged from the previous handoff: `crates/nvs-stdlib/tests/queue.rs`
  reaches a driver over a socket, and this backend's cases are `queue_sqlite.rs`'s, which need no
  server.
- What is left of stage 5 is the `nvs-cli` check, which is the group below. Nothing is blocked, and
  no `[context]` field was missing from this session's pack.

## Next group

**Stage 5: the seams, on the worker side** — one file set: `crates/nvs-cli/src/worker.rs`, whose
`mod tests` already carries the `selected`/`answered` helpers all three of these read a claim with.
Take the third item first: it is the one that says whether the two block-shaped ones can be written
without a `[db.<name>]` fixture at all.

- [ ] **`a_worker_reads_the_same_claim_columns_by_position_on_all_three_dialects`** — the shape is
      `crates/nvs-cli/src/worker.rs:1385`'s
      `the_worker_reads_args_and_script_at_the_positions_the_claim_statement_returns_them`, and the
      helpers are `selected`/`answered` at `crates/nvs-cli/src/worker.rs:1353`. The claim readers
      to agree are `postgres_claim` at `crates/nvs-cli/src/worker.rs:405`, `framed_claim` at
      `crates/nvs-cli/src/worker.rs:490` and `sqlite_claim` at `crates/nvs-cli/src/worker.rs:616`,
      under `rule:concurrency/claiming-is-one-statement`.
- [ ] **`a_sqlite_block_opens_a_queue_worker`** — `open` is `crates/nvs-cli/src/worker.rs:1134` and
      the arm it reaches is `sqlite_wire` at `crates/nvs-cli/src/worker.rs:1201`. The playbook's
      *a `-p nvs-cli` runner fixture has no `nvs.toml`* bullet is the trap to read first, since a
      `[db.<name>]` block built in Rust is what this needs.
- [ ] **`a_sql_server_block_still_starts_no_worker_and_names_the_driver`** — same `open` at
      `crates/nvs-cli/src/worker.rs:1134`, and the roster it must agree with is
      `nvs_stdlib::queue::runs` at `crates/nvs-stdlib/src/queue.rs:2368`, under
      `rule:core-classes/db-drivers-are-an-enum`.

## Backlog

- Stage 6's matrix leg: `python tools/db-matrix.py --driver sqlite` must print `sqlite: ok`
  (`docs/agent/loop-goal.toml`, stage `6 the matrix leg`).
- Stage 6's three conformance cases, all `…-on-sqlite`-shaped and none of them written
  (`docs/agent/loop-goal.toml`, stage `6 the cases`).
- Stage 6's fixture: `examples/queue-sqlite.nvs` and `examples/queue-sqlite.toml`, plus the
  `nvs queue migrate --config` run that converges its file (`docs/agent/loop-goal.toml`, stage
  `6 the fixture`).
