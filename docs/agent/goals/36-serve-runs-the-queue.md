---
milestone: M7
---
# Loop goal 36 — `nvs serve` runs the queue's workers, and the drain stops them

`[[schedule]]` fires under `nvs serve`. `[queue] workers = 4` under `nvs serve` arms nothing, reports
nothing and refuses nothing — the workers are wired into `run_run` and nowhere else, so the key is
read, validated at boot, and then silently ignored by the binary a deployment actually runs. Every
production deployment therefore needs a second `nvs run` process it was never told about.

[ADR 0154](../../decisions/0154.md) is the whole design and this goal is its implementation: one
`Option` and one call in `serve.rs`, one predicate in `worker.rs`, four rows in the directive table,
and one sentence of help text that stops being wrong.
`rule:concurrency/one-process-serves-requests-schedules-and-jobs` is what this ships, and
`rule:concurrency/who-runs-a-job-is-configuration` is what it finally makes true. **Neither is this
goal's to re-open**, and neither is the command's name: § 6 of the record settles that `nvs serve`
stays, because `nvs service` is already the platform-service-manager namespace, `daemon` is a noun in
a list of verbs, and `rule:packaging/a-service-is-one-stored-argv` makes the argv a thing already
written into units on disk.

Its floor is goal `queue-purge`'s whole list.

## Why here

Directly after goal `queue-purge` because the two are one file set and one subject: goal `queue-purge` is the entry that
gives the queue its missing member, while this is the one that gives it its missing process. M7
rather than M8: nothing here is `Core` surface — it is where a subsystem runs and what stops it.

## The shape, in one block

```toml
# One process. One unit. One thing to supervise.
[queue]
connection = "main"
workers    = 4        # armed once per INSTANCE, on the core the ticker is armed on

[[schedule]]
name = "nightly"
cron = "0 3 * * *"
```

```
$ nvs serve app.nvs
arming 1 scheduled entry
arming 4 queue workers
```

## Stage 1 — the floor

Goal `queue-purge`'s whole acceptance list, never traded.

## Stage 2 — the keystone: the drain stops a worker

One file: `crates/nvs-cli/src/worker.rs`.

**Before anything is armed under `serve`, the stop condition has to be able to come from the drain.**
Nothing begins a drain under `nvs serve` as this goal is written — `Draining::begin` is called only in
the accept loop's tail after `keep_serving` breaks, this command's seam continues forever, and there
is no signal handler in either crate — so a killed process is how a served instance ends right now.
**That will not still be true when this goal is walked**: goal `net-os-signal`'s stage 4 lands `Core\Signal` as the
entry into this same drain, and it is ahead of this entry on the chain. So a `SIGTERM` will be asking
this command to drain, and workers that ignore it turn that graceful shutdown back into a kill. The
goal's own acceptance case is the other reason for the order: a **test** breaks that seam too, so a
check that arms workers before they can be stopped **hangs rather than fails**, and a check that hangs
when the feature regresses reports nothing.

1. **`Workers` gains a second way to be stopped**, or `Workers::stop` gains a second writer: a
   `Draining` handle read at the top of each turn beside the existing flag. `nvs_server::Draining` is
   `Clone` and its `is_draining()` is the whole of what a worker needs.
2. **`nvs run` is unchanged.** Its workers still stop when the script's task exits, for the reason
   `worker.rs`'s own module doc gives. The two binaries share `start` and differ in one predicate.
3. **A claimed job runs to completion.** A drain means stop taking *new* work, and claiming is taking
   new work — the same contract an accepted request and an in-flight `[[schedule]]` fire have. The
   tail a shutdown pays is the one that module doc already prices: one `IDLE_TURN` ordinarily, one
   statement's round trip mid-claim, one `CONNECT_DEADLINE` at worst.

## Stage 3 — the arming

One file: `crates/nvs-cli/src/serve.rs`.

`nvs_config::queue::queue_for` off the boot snapshot exactly as `run_run` reads it, then
`worker::start` on the scheduler this command already creates — beside `nvs_server::arm` and **before**
the accept loop is spawned, which is where the ticker goes and for the ticker's reason.

1. **`TaskRoot::Worker`, not `TaskRoot::Request`.** The ticker holds `Request` because a fire is a
   child of the loop that serves; a worker has no request beneath it to charge a panic to
   (`rule:http-server/containment-does-not-end-at-the-helper`). **This is the one line of this change
   that looks right when it is wrong** — the ticker is three lines above and holds the other one.
2. **No lease, and no `Leases` argument.** `arm` takes `None` for `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`'s lease
   because `Core\Cache`'s shared tier has no set-if-absent. That blocker does not touch the queue:
   `rule:concurrency/claiming-is-one-statement` puts the mutual exclusion in the database, so a fleet
   of instances each running workers is the intended deployment.
3. **Armed once, on one core.** `workers` is per instance. Today `serve` turns one scheduler on one
   core so the two coincide; the per-core slice is where this has to be honoured rather than
   discovered, and § 3 of the record is the decision it inherits rather than one it may re-take.
4. **A tree with no `[queue]` block arms nothing and spawns no task**, which is the ticker's shape:
   an `Option` read at boot and no cost at all when it is absent.

## Stage 4 — `[queue]` joins the directive table

One file set: `crates/nvs-config/src/directive.rs`, `crates/nvs-config/tests/directives.rs`.

`directive::lookup` answers `Option` and no row governs `queue`, so the block has no changeability
class and no apply class at all. Four rows, `System` throughout, split the way `[deferred]` is split:

| Key | Apply |
|---|---|
| `queue.connection` | `Boot` — a connection swapped under running workers strands every claim in flight |
| `queue.workers` | `Boot` — a worker is a spawned task, and applying a count means starting or stopping tasks |
| `queue.max_attempts` | `Reload` — read per job out of the snapshot, re-creating nothing |
| `queue.visibility` | `Reload` — the same, a lease length read when a claim is taken |

**No new diagnostic.** `rule:config/a-reload-names-what-it-could-not-apply` already carries a changed
`Boot` key forward and names it; this only gives that mechanism rows to find. The census in
`crates/nvs-config/tests/directives.rs` is what holds the pairs, and it gains four entries.

## Stage 5 — the sentence that stops being wrong

One file: `crates/nvs-cli/src/main.rs`.

> *Serve a Novis file over HTTP, on one core, until stopped.*

That describes one of three subsystems and, after this goal, the least of them. The command's help
names the accept loop, the `[[schedule]]` ticker and the queue's workers, because the mental model an
operator forms of one process is what stops them looking for a second one. **The command's name does
not change** — § 6 of ADR 0154 is why, and it is not this stage's to revisit.

## Stage 6 — the fixture and the cases

`examples/queue.nvs`'s properties under the **served** binary rather than only under `nvs run`: a job
pushed to a server with `workers = 1` is claimed, runs, and reaches `Succeeded`.

The shutdown case is the one that matters and it must be **bounded**: a served process with workers
exits once its drain begins. Written as a test with a deadline rather than as a smoke run, because the
failure mode of stage 2 being missed is a hang, and a check that hangs when the feature regresses is
worse than one that fails — it has nothing to report.

**No differential case**: PHP has no queue, which is the same reason `examples/queue.nvs` has none.

## Standing decisions

- **This goal opens no new ADR number.** [ADR 0154](../../decisions/0154.md) is accepted and is the
  whole design: one `Option` and one call in `serve.rs`, one predicate in `worker.rs`, four rows in the
  directive table, one sentence of help. The two rules the lead paragraph names are not this goal's to
  re-open.
- **The command keeps its name.** § 6 of the record settles it, and it is the question this goal is
  most likely to be asked on the way past: `nvs service` is the platform-service-manager namespace,
  `daemon` is a noun in a list of verbs, and `rule:packaging/a-service-is-one-stored-argv` makes the
  argv a thing already written into units on disk. Not a stage's to revisit.
- **The drain comes from goal `net-os-signal`, which is ahead of this entry on the chain.** `Core\Signal` is what
  makes a `SIGTERM` a drain rather than a kill, and stage 2 is written so that workers answer that
  drain when it arrives. If this goal is somehow reached with that path absent, stage 2 is still the
  first slice and the acceptance case is still deadline-bounded — the order exists so a check that
  regresses fails instead of hanging, and that property does not depend on which goal landed first.
- **No lease, and that is not the lease blocker one file over.**
  `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease` waits on a set-if-absent `Core\Cache`
  has not got; the queue does not need it, because `rule:concurrency/claiming-is-one-statement` puts
  the mutual exclusion in the database. A fleet of instances each running workers is the intended
  deployment, so `arm` takes `None` deliberately.
- **`TaskRoot::Worker`, and the ticker three lines above holds the other one.** This is the one line of
  the change that looks right when it is wrong; `rule:http-server/containment-does-not-end-at-the-helper`
  is why a worker has no request beneath it to charge a panic to.
- **What this spends**, per `rule:programs/memory-priority`: nothing at all in a tree with no `[queue]`
  block — an `Option` read at boot and no task spawned, which is the ticker's shape. Where the block is
  present it is `workers` tasks per instance, armed on the core the ticker is armed on, and the tail a
  shutdown pays is the one `worker.rs`'s module doc already prices.
