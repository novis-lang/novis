# Handoff

## State

**Goal `m8-db-queue`. Stage 4 is green: `python tools/db-matrix.py --all` answers `5/5 drivers ok`.**
MariaDB's catalog read-back was the last thing red on it —
`an_applied_schema_introspects_back_to_an_empty_plan_on_mariadb` gave every nullable column a text
default of the four characters `NULL`, because that server prints the keyword in `COLUMN_DEFAULT`
where MySQL prints null itself. `catalog::column_default`'s own doc owns the reading and the one
spelling it costs.

**Stages 0–7 pass; stage 8 is unbuilt.** None of `push_records_its_grants_and_limits_on_the_row`,
`push_refuses_a_grant_the_enqueuing_request_does_not_hold` or
`a_job_runs_under_the_grants_and_limits_recorded_at_enqueue` exists in the tree, and the row has no
column for either fact. Stage 9's socket leg is unbuilt beside it — `tools/db-matrix.py` has no
`AF_UNIX` endpoint at all, so `mysql over a socket: ok` and its two siblings cannot be answered yet.

A failing matrix leg now carries the line under its panic into the ledger, so the next red one says
what the server answered rather than only which test fired.

## Next group

**Stage 8: a job's budget and grants, recorded at enqueue and applied to the isolate that runs it** —
one file set: `crates/nvs-stdlib/src/queue.rs` and `crates/nvs-cli/src/worker.rs`.
`rule:core-classes/queue-storage-is-a-table` owns what a job row may hold and
`rule:concurrency/a-job-runs-as-a-root-isolate` what the run is given, which
`crates/nvs-cli/src/worker.rs:44` § *Why the grants are the run's own* already states as the narrower
rule the schema has no column for yet.

- [ ] **The row carries the grants and the limits the enqueuing request held**, so a claim reads them
      back rather than inferring them. The columns go in `crates/nvs-stdlib/src/queue.rs:308` and the
      insert that fills them is `crates/nvs-stdlib/src/queue.rs:416`; the test is
      `push_records_its_grants_and_limits_on_the_row`.
- [ ] **`push` refuses a grant the enqueuing request does not hold**, which is the widening the
      module doc's § *Why the grants are the run's own* refuses, at
      `crates/nvs-stdlib/src/queue.rs:416`. The test is
      `push_refuses_a_grant_the_enqueuing_request_does_not_hold`.
- [ ] **The job runs under what the row recorded**, narrowed and never widened, where the worker
      builds the root isolate — `crates/nvs-cli/src/worker.rs:897`. The test is
      `a_job_runs_under_the_grants_and_limits_recorded_at_enqueue`.

## Backlog

- Stage 9's `AF_UNIX` legs: `tools/db-matrix.py` publishes TCP endpoints only (that tool's module doc).
- `crates/nvs-db/src/catalog.rs` gaps 1–3 — the two-sided normalisation and the opaque read-only
  default case, owner `unowned-closures`.
