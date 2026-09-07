# Loop goal 32 — `nvs serve` runs the queue's workers, and the drain stops them

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

Its floor is goal 31's whole list.

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

Goal 31's whole acceptance list, never traded.

## Stage 2 — the keystone: the drain stops a worker

One file: `crates/nvs-cli/src/worker.rs`.

**Before anything is armed under `serve`, the stop condition has to be able to come from the drain**,
because a worker that ignores it is a server that cannot be stopped — and a check that arms workers
first would hang rather than fail.

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
