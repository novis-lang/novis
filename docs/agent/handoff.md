# Handoff

## State

**Goal `serve-runs-the-queue` — stages 2, 3, 4 and 5 are landed.** `nvs serve` arms `[queue] workers`
where it arms the `[[schedule]]` ticker, on the core that ticks; every worker stops on the process
drain as well as on `Workers::stop`; `[queue]`'s four keys are rows in the directive registry, so a
reload that changes `workers` carries the running count forward and names the key; and the
subcommand's help now names all three subsystems rather than the accept loop alone
(`rule:concurrency/one-process-serves-requests-schedules-and-jobs`,
[ADR 0154](../decisions/0154.md) § 6). Stage 6 is the only one open.

The help is asserted in two renderings — the listing line `nvs --help` prints and `serve --help`'s
own long help — because the first is read before an operator has decided this is the command, and it
was the one that described one subsystem of three. The old sentence's "on one core" is gone with it:
the accept loop is one worker per core (`crates/nvs-cli/src/serve.rs:336`) and it is the ticker and
the workers that are armed once, on the core that ticks.

`Draining::begin` still has **no** production caller under `nvs serve` — the only one is the accept
loop's tail (`crates/nvs-server/src/serve.rs:1386`), which this command's `keep_serving` seam never
reaches, and there is no signal handler in either crate. That is what ADR 0154 § 2 describes and not
a gap this goal closes: goal `net-os-signal` lands the caller, and the predicate is here first so
that it does not read as a defect in signal handling when it arrives.

The design stays closed. ADR 0154 is the whole of it and the goal's § *Standing decisions* holds the
three things a session must not re-decide — the command keeps its name, the workers hold
`TaskRoot::Worker`, and `arm` passes no lease.

## Next group

**Stage 6 — the bounded end-to-end case**, which is the last stage in this goal. One file set:
`crates/nvs-cli/src/worker.rs`'s test module, whose existing SQLite fixtures already build a
`[queue]` block, a scheduler and a worker, so both cases are `-p nvs-cli` and neither needs a real
server. The stage's third piece, the `examples/queue.nvs` `exact` check, is a separate file set and
is deliberately not in this group.

- [ ] **`a_job_pushed_to_a_server_with_one_worker_reaches_succeeded`** — one job enqueued against a
      SQLite `[queue]` served by a run with `workers = 1`, driven to the `succeeded` state and
      asserted there rather than on the worker having polled. Copy the fixture at
      `crates/nvs-cli/src/worker.rs:1575`, which is the block-to-worker path already pinned, and take
      the claim-in-flight case at `crates/nvs-cli/src/worker.rs:1659` for how a job is pushed and
      read back. `rule:concurrency/one-process-serves-requests-schedules-and-jobs` is the rule, and
      `rule:concurrency/a-job-runs-as-a-root-isolate` says what the run must be.
- [ ] **`a_served_process_with_workers_exits_within_its_deadline_once_the_drain_begins`** — the same
      fixture with the drain begun, asserting the run returns inside a deadline the test names, so a
      worker that ignored the drain fails instead of hanging. The predicate is
      `crates/nvs-cli/src/worker.rs:1659`'s sibling
      `a_worker_handed_a_draining_handle_returns_at_the_top_of_its_next_turn`; what this case adds is
      the whole run rather than the one worker.

## Backlog

- `examples/queue.nvs`'s `exact` check (stage 6 fixture, `docs/agent/loop-goal.toml:7618`) — the five
  `want` lines are echoed by the file already; whether it runs green is unmeasured here.
- The conformance and differential suites are stage 6's last two checks and are the goal's floor.
- `Draining::begin`'s production caller is goal `net-os-signal`, not this goal.
