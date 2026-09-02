# Handoff

## State

**A worker runs what it claimed.** `crates/nvs-cli/src/worker.rs`'s `run` is ADR 0084 § 5's root
isolate, reached through `nvs_runtime::script::resolve` — the same door `spawn script` uses, so
`script.spawn` is asked of the worker's own context, which is handed the run's boot snapshot for
that reason. The payload crosses as the value `nvs_stdlib::queue::payload` decodes off the `args`
column, which is the other half of that module's `payload_of`. **The attempt is not written back**:
that is the module doc's *Known gap* and the next slice, so a succeeding job's row still sits in
`Claimed` until § 4's visibility timeout hands it back.

**Stage 1's `a warm-cache CLI start stays under 10ms` is green again, and it was a real
regression, not the stale binary the ledger named.** `[queue] workers` spawned its tasks *before*
the script's, so every `nvs run` in a tree with a queue waited out a PostgreSQL handshake it had no
use for — a paired measurement put the same empty program at 16.7 ms with the workers spawned first
and 8.9 ms with `workers = 0`. They are spawned after the script's task now and read the stop flag
before `open`, so a program that never parks gives them no turn and pays nothing for them.
`tools/bench.py`'s own figure went from 17.0 ms to 8.7 ms against the 10 ms budget.

**The driver builds `target/release/nvs.exe` itself now** — `tools/loop.py`'s `Goal.release_cli`,
in `prebuild`'s existing background thread. Nothing else in the loop built the binary
`tools/bench.py` measures, so that check had been reporting whatever was last built by hand.

**`examples/queue.nvs` still prints its last three lines off bounded polls that time out**, which is
~15s of its ~19s; only the write-back below closes them.

## Next group

**Reporting the attempt, over `crates/nvs-cli/src/worker.rs`, `crates/nvs-stdlib/src/queue.rs` and
`examples/queue.nvs`.**

- [ ] **Write the attempt back** — ADR 0084 § 6. `crates/nvs-cli/src/worker.rs:321`'s `run` drops
      the completion today: `Succeeded` when it is `ok`, and otherwise back to `Pending` with
      `run_at` pushed out by the row's own `backoff_ms`. Two statements beside
      `crates/nvs-stdlib/src/queue.rs:319`, and `crates/nvs-cli/src/worker.rs:247`'s `Job` gains the
      four columns `crates/nvs-stdlib/src/queue.rs:289`'s `returning` already sends —
      `crates/nvs-cli/src/worker.rs:303` is where the column indexes are written down.
- [ ] **§ 6's dead-letter move** — an exhausted job's row moves to `nvs_dead_jobs`
      (`crates/nvs-stdlib/src/queue.rs:206` is the table) with its payload, its attempts and its
      timing, and is never deleted. One statement, taken at the failing write-back above when
      `attempts >= max_attempts`.
- [ ] **The fixture's last three lines print off the state they name** — `examples/queue.nvs:91`
      and the two `Poll::until` calls below it echo whether or not the poll succeeded, so all five
      lines are green today and three of them prove nothing. With the two slices above they are
      real, and the fixture's ~19s collapses to the polls actually returning.

## Backlog

- § 5's "grants narrowed from those recorded at enqueue" has nowhere to be recorded — § 2's schema
  in `docs/adr/0084-durable-background-jobs.md` has no column for them, and a worker runs a job with
  the deployment's own grants meanwhile.
- § 5 names `nvs serve` and `nvs work` as the two processes that run jobs; this tree runs them
  inside `nvs run` as well, which is what the fixture needs and what the ADR does not say.
- `orient.py`'s `[context] modules` still names no `nvs-cli/src/*`, `nvs-host/src/*` or
  `nvs-config/src/*`, and this session also needed `nvs-runtime/src/script.rs`,
  `nvs-host/src/isolate.rs` and `nvs-stdlib/src/json.rs`; all were sliced by hand.
- `[context] adrs` carried ADR 0067 §§ 1, 9 and 13 for a session that needed ADR 0084 §§ 4 and 5,
  which are the goal's own stage 8 sections.
- `Core\Db::open` still waits on a shape-parameter type — `docs/plan/m8.md`.
