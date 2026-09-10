# Handoff

## State

**Goal `serve-runs-the-queue` is closed — all six stages are on disk.** Stage 6's two cases live in
`crates/nvs-cli/src/worker.rs`'s test module and pass in 0.2s: one job enqueued against a SQLite
`[queue]` served by `workers = 1` is read back in `succeeded`, and a served instance holding two
workers ends within a bound once the drain begins. The stage's third piece, the `examples/queue.nvs`
`exact` check, was already green — the example's stdout is the five wanted lines exactly; only the
driver's own invocation had to be understood, which the new playbook bullet owns.

The end-to-end fixture is now in-crate and cheap to reuse: `a_queue_file`
(`crates/nvs-cli/src/worker.rs:1812`) hands out a `[db.<name>]` block over a temp SQLite file,
`converged` (`:1835`) builds `rule:core-classes/queue-storage-is-a-table`'s two tables from
`nvs_stdlib::queue::migration`'s own DDL, `pushed` (`:1877`) enqueues one due job, and
`draining_once_the_job_leaves` (`:1969`) is the operator's shutdown *and* the case's clock. The grant
a worker needs is `crate::script::granting_snapshot` (`crates/nvs-cli/src/script.rs:770`), which is
`granting_ctx`'s capability literal split in two because a worker builds its own context.

`Draining::begin` still has **no** production caller under `nvs serve` — goal `net-os-signal` lands
it, and ADR 0154 § 2 is why the predicate is here first. The design stays closed: ADR 0154 is the
whole of it, and the goal's § *Standing decisions* holds the three things not to re-decide.

## Next group

**The write-back's other two branches, through a real worker** — one file set:
`crates/nvs-cli/src/worker.rs`'s test module, whose new SQLite fixture above already builds a
`[queue]`, arms workers and drives them to a state. Both cases are `-p nvs-cli` and neither needs a
server. Take these only if the driver is still on this goal; a goal switch overwrites this file.

- [ ] **a job whose script throws is retried before it is dead-lettered** — pushed with
      `max_attempts = 2`, driven until the row leaves `nvs_jobs`, and asserted on the dead-letter
      row rather than on the attempt count. `crates/nvs-cli/src/worker.rs:938` is the
      `attempts >= max_attempts` branch that has no end-to-end case, and
      `crates/nvs-cli/src/worker.rs:1877`'s `pushed` is the enqueue to widen with the two columns.
      `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` is the rule.
- [ ] **a served instance whose `[db.<name>]` never opens says so once and still ends** —
      `crates/nvs-cli/src/worker.rs:1202` warns and returns no worker, and
      `crates/nvs-cli/src/worker.rs:250` is the stopping read that happens before the connection, so
      such an instance is silently worker-less and passes a shutdown case for the wrong reason.
      That is exactly what the liveness assertion in
      `a_served_process_with_workers_exits_within_its_deadline_once_the_drain_begins` guards, and it
      deserves a case of its own. `rule:concurrency/one-process-serves-requests-schedules-and-jobs`.

## Backlog

- `Draining::begin` has no production caller under `nvs serve`; goal `net-os-signal` lands it (ADR
  0154 § 2).
- What must survive a goal switch belongs in `docs/agent/carried-gaps.md`, not here.
