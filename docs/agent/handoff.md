# Handoff

## State

**Goal `m8-db-queue`. Stage 7 is green.** The last thing open on it was
`a_dead_lettered_row_carries_every_attempts_error`, and closing it meant building what
`rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` and `docs/decisions/0187.md` § 4
already specify: `nvs_jobs` carries a nullable `errors` column, every claim answers it as the last of
its columns, `nvs_stdlib::queue::dead_errors` appends an entry to the array it is handed rather than
building a one-entry one, and both failing write-backs — the retry and the dead-letter move — bind
the grown array. `queue::schema`'s own doc owns the trade and the bound; `MESSAGE_CAP` is the cap the
record asks for.

**Stages 0–6 pass, stage 8 is unbuilt.** None of `push_records_its_grants_and_limits_on_the_row`,
`push_refuses_a_grant_the_enqueuing_request_does_not_hold` or
`a_job_runs_under_the_grants_and_limits_recorded_at_enqueue` exists, and the row has no column for
either fact. Stage 9's socket leg is unbuilt beside it — `tools/db-matrix.py` has no `AF_UNIX`
endpoint, so `mysql over a socket: ok` and its two siblings cannot be answered yet.

**The whole mechanism is proved end to end on SQLite and as texts elsewhere.**
`crates/nvs-stdlib/tests/queue_sqlite.rs` claims, retries and dead-letters against a real database,
so the new column being read back and written is asserted there; `tests/queue.rs` returns early with
no server configured, so the fourth `RETRY_*` parameter reaching a wire driver is
`python tools/db-matrix.py --all`'s to confirm.

## Next group

**Stage 8: a job's budget and grants, recorded at enqueue and applied to the isolate that runs it** —
one file set: `crates/nvs-stdlib/src/queue.rs` and `crates/nvs-cli/src/worker.rs`.
`rule:core-classes/queue-storage-is-a-table` owns what a job row may hold and
`rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` what is recorded, which
`crates/nvs-cli/src/worker.rs:44` § *Why the grants are the run's own* already states as the narrower
rule the schema has no column for yet.

- [ ] **The row carries the grants and the limits the enqueuing request held**, so a claim reads them
      back rather than inferring them. The columns go in `crates/nvs-stdlib/src/queue.rs:311` beside
      the `errors` one, the option readers they are parsed by sit at
      `crates/nvs-stdlib/src/queue.rs:2532`, and the insert that fills them is
      `crates/nvs-stdlib/src/queue.rs:420` and its four siblings; the test is
      `push_records_its_grants_and_limits_on_the_row`.
- [ ] **`push` refuses a grant the enqueuing request does not hold**, which is the widening
      `rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` forbids, asked at the same
      door `rule:security/capability-check-at-the-door` names. The refusal belongs beside the other
      option readers at `crates/nvs-stdlib/src/queue.rs:2532`; the test is
      `push_refuses_a_grant_the_enqueuing_request_does_not_hold`.
- [ ] **The job runs under what the row recorded**, narrowed and never widened, where the worker
      builds the isolate: `crates/nvs-cli/src/worker.rs:952`, with the claim's new columns reaching
      it on `Job` the way `errors` does. The test is
      `a_job_runs_under_the_grants_and_limits_recorded_at_enqueue`.

## Backlog

- Stage 9's socket leg: `tools/db-matrix.py` has no `AF_UNIX` endpoint for any driver.
- `nvs_stdlib::queue::schema`'s `errors` column is not reachable from `Core\Queue` — a dead job's
  array is read by a person looking at the table, per `docs/decisions/0187.md` § 4.
- The `unowned`-tagged gaps in `crates/nvs-stdlib/src/db/mod.rs` and `crates/nvs-stdlib/src/queue.rs`
  are goal `unowned-closures`'s, per `docs/agent/loop-goal.md` § *Standing decisions*.
