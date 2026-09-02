# Handoff

## State

**A worker claims, and it is a task on the run's own scheduler.**
`crates/nvs-cli/src/worker.rs` is the whole of where it runs; its module doc owns the three
decisions — a task rather than a thread (`nvs_host`'s parking stream blocks off a core, so a
blocking worker would stop the very program it claims for), `nvs-cli` rather than `nvs-stdlib`
(a task is a thing only the scheduler's owner can spawn), and a stop flag rather than a
cancellation (`run_until_idle` never returns while a worker polls).

**§ 2's unanswered question — which queues a worker asks about — is answered by the table.**
`nvs_stdlib::queue::QUEUES` is the roster statement, landed beside `CLAIM`, which is now `pub`
along with `now_millis`. § 2's block names no queues and this invents no fifth key: a queue exists
exactly when a row names it. That constant's doc owns the cost and the gap.

**`examples/queue.nvs` prints `claimed 1`, and does so on every run.** Two changes, both fixture:
each run draws its queue names from `Core\Uuid::v4()`, because § 2's tables are the deployment's
and nothing empties them between runs; and the count is read *by* the wait that waited for it
(`Poll::counted`), because `claimed` is a state a job passes through and asking again after the wait
is a second instant. The remaining three lines still print off bounded polls that time out, which is
~15s of the fixture's ~19s and collapses when the next group lands.

**The claimed job is not run.** That is the known gap in `worker.rs`'s module doc and the next two
slices; a claimed row sits in `Claimed` until § 4's visibility timeout hands it back.

**`orient.py`'s pack was short in the same place as the last three sessions**: `[context] modules`
names no `nvs-cli/src/*`, `nvs-config/src/*` or `nvs-host/src/*`, and `[context] adrs` carried
neither ADR 0072 §§ 1 and 6 nor ADR 0106 § 2, which are what decide a task's root and its teardown.
All were sliced by hand.

## Next group

**Running what the worker claimed, over `crates/nvs-cli/src/worker.rs`,
`crates/nvs-stdlib/src/queue.rs` and `crates/nvs-cli/src/script.rs`.**

- [ ] **Run the claimed job as an isolate** — ADR 0084 §§ 4 and 5. `CLAIM`'s `returning` list is
      already what running one needs (`crates/nvs-stdlib/src/queue.rs:289`), and
      `crates/nvs-cli/src/worker.rs:154` is the `turn` that drops it on the floor today. The isolate
      seam is installed for the whole run by `main.rs`'s `nvs_runtime::script::scoped`, so a worker
      task is inside it: `crates/nvs-cli/src/script.rs:166`'s `run_child` is the shape, and
      `crates/nvs-cli/src/script.rs:134`'s `granting_ctx` is the context a child gets.
- [ ] **Report the attempt** — ADR 0084 § 6. Two statements beside `QUEUES` at
      `crates/nvs-stdlib/src/queue.rs:319`: a success moves the row to `Succeeded`, a failure returns
      it to `Pending` with `run_at` pushed out by the row's own `backoff_ms`. `attempts` is already
      incremented by the claim, so neither statement touches it — `CLAIM`'s doc at
      `crates/nvs-stdlib/src/queue.rs:272` owns why.
- [ ] **§ 6's dead-letter move** — the exhausted job's row moves to `nvs_dead_jobs`
      (`crates/nvs-stdlib/src/queue.rs:114`) with `failed_at`, rather than being deleted. The columns
      that table has are `MIGRATION`'s at `crates/nvs-stdlib/src/queue.rs:171`, and `DEAD_TABLE`'s
      doc is deliberate about reading only `id` and `queue` off it.

## Backlog

- Every run leaves two rows under drawn queue names, re-claimed once per visibility window until the
  dead-letter slice drains them — `crates/nvs-cli/src/worker.rs`'s module doc.
- `Core\Queue`'s `limits` and `grants` wait on a shape parameter, with `Core\Db::open` — that
  module's gap 1.
- The other four ADR 0067 backends have no statement path, so `postgres_of` refuses them by name —
  gap 5 in the same doc.
- `QUEUES` is a `distinct` scan per idle turn; it wants a roster key in § 2 if a deployment's backlog
  ever makes that matter — that constant's own doc.
- `[context]` in `docs/agent/loop-goal.toml` still selects no `nvs-cli`, `nvs-config` or `nvs-host`
  module lines.
