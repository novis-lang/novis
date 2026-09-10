# Handoff

## State

**Goal `serve-runs-the-queue` — stages 2, 3 and 4 are landed.** `nvs serve` arms `[queue] workers`
where it arms the `[[schedule]]` ticker, on the core that ticks; every worker stops on the process
drain as well as on `Workers::stop`; and `[queue]`'s four keys are now rows in the directive
registry — `System` throughout, `connection` and `workers` `Boot`, `max_attempts` and `visibility`
`Reload`, which is [ADR 0154](../decisions/0154.md) § 5's table. A reload that changes `workers`
therefore carries the running count forward and names the key
(`rule:config/a-reload-names-what-it-could-not-apply`) instead of publishing a count no task was
started or stopped for. Stages 5 and 6 are open.

`Draining::begin` still has **no** production caller under `nvs serve` — the only one is the accept
loop's tail (`crates/nvs-server/src/serve.rs:1386`), which this command's `keep_serving` seam never
reaches, and there is no signal handler in either crate. That is what ADR 0154 § 2 describes and not
a gap this goal closes: goal `net-os-signal` lands the caller, and the predicate is here first so
that it does not read as a defect in signal handling when it arrives.

The design stays closed. ADR 0154 is the whole of it and the goal's § *Standing decisions* holds the
three things a session must not re-decide — the command keeps its name, the workers hold
`TaskRoot::Worker`, and `arm` passes no lease.

## Next group

**Stage 5 — the sentence that stops being wrong**, which is the last thing in this goal that still
describes one subsystem of three. One file set: `crates/nvs-cli/src/main.rs`, the `Serve`
subcommand's doc comment and the crate's own test module. Both cases are `-p nvs-cli` and neither
needs a scheduler.

- [ ] **The sentence** — rewrite the `Serve` variant's doc comment at
      `crates/nvs-cli/src/main.rs:273`, which `clap` renders as the subcommand's `about`, so it
      names the accept loop, the `[[schedule]]` ticker and the queue's workers rather than only the
      first: `rule:concurrency/one-process-serves-requests-schedules-and-jobs` is what an operator
      has to be able to read off `--help`, because the mental model they form of one process is what
      stops them looking for a second one to install.
- [ ] **`the_serve_help_names_requests_schedules_and_queue_workers`** in
      `crates/nvs-cli/src/main.rs:2099`, over `clap`'s rendered help rather than over the string
      constant, so the case fails if the text stops reaching the surface it is written for.
- [ ] **`the_command_is_still_spelled_serve_and_service_is_still_the_other_namespace`**, in the same
      module at `crates/nvs-cli/src/main.rs:2099`, over the `Command` enum at
      `crates/nvs-cli/src/main.rs:273`: ADR 0154 § 6 and `rule:packaging/a-service-is-one-stored-argv`
      settle the name, and a stage that rewrites the sentence is exactly when it gets renamed by
      accident.

## Backlog

- Stage 6 — `examples/queue.nvs` under the served binary, and the **deadline-bounded** shutdown
  case; `docs/agent/loop-goal.md` § *Stage 6* is the shape, and its two `-p nvs-cli` test names are
  in `docs/agent/loop-goal.toml`'s stage 6 block.
- `Draining::begin` has no production caller — goal `net-os-signal`, per ADR 0154 § 2.
