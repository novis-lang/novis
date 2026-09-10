# Handoff

## State

**Goal `serve-runs-the-queue` — `nvs serve` runs the queue's workers, and the drain stops them — has just started; nothing
of it has landed yet.** Goal `queue-purge`'s whole list is this goal's Stage 1 floor.

The design is finished and is not this goal's to re-open. [ADR 0154](../decisions/0154.md) holds all
of it, and `rule:concurrency/one-process-serves-requests-schedules-and-jobs` is what this ships.

This is a small goal with one way to get it badly wrong, and the stage order exists to stop it.

**Three things a session must not re-decide:**

1. **The drain is the stop condition, and stage 2 comes before stage 3 because of it.** `serve.rs`'s
   loop ends on `parked == 0` and a worker polling its idle turn is parked, so an unstoppable worker
   is a process that can only be killed. **Check the premise before arguing with it**: when this goal
   was written nothing began a drain under `nvs serve` at all — `Draining::begin` is called only in
   the accept loop's tail after `keep_serving` breaks, that seam continues forever, and there was no
   signal handler in either crate. **Goal `net-os-signal`'s stage 4 changes that before you get here**, landing
   `Core\Signal` as the entry into this same drain, so check whether it has: if it has, an ignored
   drain turns a graceful `SIGTERM` back into a kill, which is a production defect and not only a
   test one. Either way the order holds — a test breaks that seam too, so arming workers before they
   can be stopped gives you a check that hangs rather than fails, and a hanging check reports
   nothing. Land the predicate first.
2. **`TaskRoot::Worker`, not `TaskRoot::Request`.** The ticker three lines above holds `Request`
   because a fire is a child of the loop that serves; a worker has no request beneath it to charge a
   panic to (`rule:http-server/containment-does-not-end-at-the-helper`). This is the one line of the
   change that looks right when it is wrong, and copying the neighbour is exactly how it goes in.
3. **The command keeps its name.** `nvs serve` stays `nvs serve` — `nvs service` is already the
   platform-service-manager namespace, `daemon` is a noun in a list of verbs, and
   `rule:packaging/a-service-is-one-stored-argv` makes the argv a thing already written into units on
   disk. What changes is the sentence under it, in stage 5. ADR 0154 § 6 is the argument and the user
   has agreed it.

## Next group

**Stage 2 + stage 3, in that order, in one slice** — the stop condition and then the arming, because
the second without the first is a server that will not exit. One file set:
`crates/nvs-cli/src/worker.rs`, `crates/nvs-cli/src/serve.rs`.

- [ ] **The predicate** — a worker's stop reads a `nvs_server::Draining` beside the existing
      `Workers::stop` flag, at the top of each turn. `Draining` is `Clone` and `is_draining()` is the
      whole of what a worker needs; `crates/nvs-server/src/serve.rs:318` is the type and its own doc
      owns who may write the bit.
- [ ] **`nvs run` unchanged** — its workers still stop when the script's task exits. The two binaries
      share `worker::start` and differ in one predicate, which is what makes moving work between them
      operational and never behavioural.
- [ ] **The arming** — `nvs_config::queue::queue_for` off the boot snapshot as `run_run` reads it, then
      `worker::start` on the scheduler `serve.rs` already creates, beside `nvs_server::arm` and
      **before** the accept loop is spawned. `start` already takes `&mut Scheduler`, `&QueueBounds`,
      `&Database` and `&Arc<Snapshot>` — `serve.rs` holds all four at that point, which is why this is
      a handful of lines.
- [ ] **`TaskRoot::Worker`.** See above.
- [ ] **No lease and no `Leases` argument.** `arm` passes `None` because `Core\Cache`'s shared tier has
      no set-if-absent; that blocker does not touch the queue at all, because
      `rule:concurrency/claiming-is-one-statement` already puts the mutual exclusion in the database.
      A fleet each running its own workers is the intended deployment, not a hazard.
- [ ] **Armed once, on one core.** `workers` is per instance. Today `serve` turns one scheduler on one
      core so the two coincide — the per-core slice is where this has to be honoured, and ADR 0154 § 3
      is the decision it inherits rather than one it may re-take.

## Backlog

- **Stage 4** — four `[queue]` rows in `crates/nvs-config/src/directive.rs`, `System` throughout,
  `connection`/`workers` `Boot` and `max_attempts`/`visibility` `Reload`. The block has **no row at
  all** today: `lookup` answers `Option` and nothing governs `queue`, so a reload that changed
  `workers` is published, changes nothing and says nothing. No new diagnostic — the reload's naming
  mechanism exists and this gives it rows to find. The census in
  `crates/nvs-config/tests/directives.rs` gains four entries.
- **Stage 5** — the help line in `crates/nvs-cli/src/main.rs:259`, which says "Serve a Novis file over
  HTTP, on one core, until stopped" and after this goal describes the least of three subsystems.
- **Stage 6** — `examples/queue.nvs`'s properties under the *served* binary, and the shutdown case.
  **Write the shutdown case with a deadline**: the failure mode of stage 2 being missed is a hang, and
  a check that hangs when the feature regresses is worse than one that fails, because it has nothing
  to report. **No differential case**: PHP has no queue.
