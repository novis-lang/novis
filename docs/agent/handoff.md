# Handoff

## State

**`nvs queue migrate` applies § 2's schema.** It opens the `[db.<name>]` it proved and runs
`nvs_stdlib::queue::MIGRATION` one statement at a time, printing `-- <label>: applied` per
statement; `--dry-run` is unchanged and still prints the DDL without opening anything.
`crates/nvs-cli/src/queue.rs`'s module doc owns the two decisions: `nvs-cli` gained an `nvs-db`
dependency rather than borrowing `main.rs`'s task, because `nvs_host`'s parking stream blocks the
calling thread off a core (that module's § *Off a core, it blocks*), and `db.connect` is not asked
because it is keyed on an entry file this command does not have.

**The compose server holds both tables, and the sweep creates them itself**: stage 8 of
`loop-goal.toml` now runs the applying command above the `examples/queue.nvs` fixture, and that
block's position is load-bearing. Re-running is safe — every statement carries `if not exists`.

**`push` wrote SQL `null` into a not-null column** for a call that left `{backoff}` out, which is
what the fixture failed on once the table existed. `DEFAULT_BACKOFF_MS` (one second) is now what
such a call agreed to, and its doc owns why the default lives in `nvs-stdlib` and not in `[queue]`.

**`examples/queue.nvs` prints all five lines and one is wrong: `claimed 0`.** Nothing claims,
because there is no worker yet — that is the whole remaining gap in stage 8, and § 4's `CLAIM` is
written beside `INSERT` for it, under `#[cfg_attr(not(test), expect(dead_code))]` until the worker
reads it. Two tests hold it to the DDL and to `State`'s ordinals.

**The fixture leaves two rows in `nvs_jobs` per run and nothing cleans them.** Harmless today; once
a worker exists, `claimed 1` is a count over rows this sweep accumulated across iterations, so the
worker slice has to decide between a per-run queue name and a fixture that clears its own queues. I
emptied both tables at the end of this session.

**`orient.py`'s pack was short in the same places as the last two sessions**: `[context] modules`
names no `nvs-cli/src/*` or `nvs-config/src/*`, and `[context] adrs` did not carry ADR 0084 § 4,
which this session's second slice is specified by. All three were sliced by hand.

## Next group

**The worker, over `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-cli/src/main.rs` and
`crates/nvs-config/src/queue.rs`.**

- [ ] **Where the in-process worker runs** — ADR 0084 §§ 2 and 4. Nothing outside the CLI calls
      `crates/nvs-config/src/queue.rs:98`, so `[queue] workers` is read by no run today;
      `crates/nvs-cli/src/main.rs:786` is the only path that runs work inside an `nvs-host` task, and
      a worker is a sibling task of the script's. It claims with `crates/nvs-stdlib/src/queue.rs:291`,
      whose doc states the two open ends: the visibility cutoff is the caller's to compute from
      `[queue] visibility`, and the queue roster one worker asks about is § 2's question.
- [ ] **Running a claimed job and reporting the attempt** — ADR 0084 §§ 4-6. The `returning` list of
      `crates/nvs-stdlib/src/queue.rs:291` is what running one needs; success writes
      `State::Succeeded`, a failure returns the row to `Pending` with its backoff, and
      `crates/nvs-stdlib/src/queue.rs:948` is the delay it waits.
- [ ] **§ 6's dead-letter move** — the exhausted job's row moves to `nvs_dead_jobs` with `failed_at`
      and `errors`, in one transaction. The columns are `crates/nvs-stdlib/src/queue.rs:204` and the
      depth `stats` reports is `crates/nvs-stdlib/src/queue.rs:307`.

## Backlog

- `Core\Db::open` waits on a shape-parameter type — `docs/implementation-plan.md`, Open now.
- Stage 5 is four of seven in `-p nvs-db` — `docs/agent/loop-goal.toml`, stage 5.
- A second driver brings its own `MIGRATION` list, not a dialect switch — `nvs_stdlib::queue`'s doc.
- `[context] modules` owes `nvs-cli/src/*` and `nvs-config/src/*`; `adrs` owes ADR 0084 § 4 —
  `docs/agent/loop-goal.toml`.
