# Handoff

## State

**Goal `m8-db-queue`. Stages 0-7 are green, and the floor's three queue fixtures are green again.**
The regression they reported was not in the queue: session 0007 gave `nvs_jobs` an `errors` column
and every dialect's claim answers with it, so a database whose `nvs queue migrate` had not been
re-run answered nothing and the workers died on the first claim. The two applying migrations sat
above the fixtures in `docs/agent/loop-goal.toml` for exactly this reason, but a `command` check
runs behind every program leg whatever it sits above, so the sweep could never have converged the
table first. `setup = true` (`tools/loop.py:2085`) is a tier of its own, ahead of the floor, and
those two blocks carry it with `memoize = false`; the playbook's bullet is the trap.

**A worker that stops on a failed statement now says so** (`crates/nvs-cli/src/worker.rs:247`).
`take_turns` hands its refusal back rather than swallowing it, which is what made the column
mismatch above cost a session to find: the queue silently never drained and `stats` stayed at zero
with nothing on either stream.

**Stage 8 is unbuilt and stage 9's socket leg beside it.** None of the three stage-8 tests exists
and the row has no column for either fact; `tools/db-matrix.py` has no `AF_UNIX` endpoint, so
`mysql over a socket: ok` and its two siblings cannot be answered yet.

## Next group

**Stage 8: a job's budget and grants, recorded at enqueue and applied to the isolate that runs it**
— one file set: `crates/nvs-stdlib/src/queue.rs` and `crates/nvs-cli/src/worker.rs`.
`rule:core-classes/queue-storage-is-a-table` owns what a job row may hold,
`rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` what is recorded, and
`crates/nvs-cli/src/worker.rs:44` § *Why the grants are the run's own* is what the worker does today.

**These three are one unit, and the order inside it is a security question, not a preference.**
`crates/nvs-stdlib/src/queue.rs:64` gap 1 prices a declared-and-dropped `grants` as
`rule:concurrency/an-upgrades-options-are-spawn-scripts`' accepted-and-dropped narrowing — the job
keeps the authority the request meant to give up — so `push` declares neither option until the
isolate applies them, which `limits_and_grants_are_refused_by_name_until_an_isolate_enforces_them`
(`crates/nvs-stdlib/src/queue.rs:6321`) holds. Either land all three in one session, or make item 1
record the *enqueuing context's own* grants and limits and declare no option, which widens nothing
and leaves that test green.

- [ ] **The row carries the grants and the limits the enqueuing request held**, so a claim reads
      them back rather than inferring them. The columns go in `crates/nvs-stdlib/src/queue.rs:311`
      beside the `errors` one, the readers sit beside `max_attempts_of` at
      `crates/nvs-stdlib/src/queue.rs:2532`, and the insert that fills them is
      `crates/nvs-stdlib/src/queue.rs:420` and its siblings at `:675`, `:703` and `:730`, whose
      bound slots are the `eleven` array at `crates/nvs-stdlib/src/queue.rs:2856`; the test is
      `push_records_its_grants_and_limits_on_the_row`.
- [ ] **`push` refuses a grant the enqueuing request does not hold**, which is the widening
      `rule:concurrency/a-jobs-budget-and-grants-are-recorded-at-enqueue` forbids, declared beside
      the other options at `crates/nvs-stdlib/src/queue.rs:1743` and answered where the bag is read
      at `crates/nvs-stdlib/src/queue.rs:2532`; the test is
      `push_refuses_a_grant_the_enqueuing_request_does_not_hold`.
- [ ] **The job runs under what the row recorded**, narrowed and never widened: the claim answers
      with both columns at `crates/nvs-cli/src/worker.rs:474` and the isolate the worker starts
      takes them at `crates/nvs-cli/src/worker.rs:44`'s door
      (`rule:concurrency/a-job-runs-as-a-root-isolate`); the test is
      `a_job_runs_under_the_grants_and_limits_recorded_at_enqueue`.

## Backlog

- A queue whose schema is behind is served anyway — `crates/nvs-stdlib/src/queue.rs:88` gap 2's
  `Decided:` is to refuse at boot, and it is the tree-side answer to what `setup = true` works
  around for the sweep alone.
- `tools/db-matrix.py` has no `AF_UNIX` endpoint, so stage 9's three socket legs cannot be answered.
- The `unowned`-tagged gaps in `crates/nvs-stdlib/src/db/mod.rs` and `crates/nvs-stdlib/src/queue.rs`
  are goal `unowned-closures`', never this one's.
