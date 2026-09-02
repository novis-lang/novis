# Handoff

## State

**ADR 0084 § 6's dead-letter move is landed and proven against the live container.** An attempt
that fails on a job whose `attempts` have reached its `max_attempts` is no longer left claimed:
`crates/nvs-cli/src/worker.rs:463`'s `report` runs `crates/nvs-stdlib/src/queue.rs:376`'s
`DEAD_LETTER`, one data-modifying CTE that deletes the row out of `nvs_jobs` and inserts it into
`nvs_dead_jobs` in the same moment — so a job is never in both tables or in neither — keyed on the
lease exactly as `SUCCEEDED` is. `run` answers with the `nvs_host::Failure` the attempt produced
rather than a bool, and a refusal (an unresolvable script, a payload that will not cross) crosses
in that same shape under class `Error`, so the row records a refusal and a throw identically.

**The `errors` array is one entry deep, and that is a decision rather than unfinished writing.**
`nvs_jobs` has no column holding what an earlier attempt threw, so § 6's "every attempt's error" is
the last attempt's; `MIGRATION`'s doc comment owns the trade and `dead_errors`
(`crates/nvs-stdlib/src/queue.rs:397`) owns the entry's shape. Every attempt before the last is
visible only on the worker's standard error, which is the worker module's `## Known gap`.

**`examples/queue.nvs` prints all five of stage 8's frozen lines** against `tests/db/compose.yaml`'s
PostgreSQL, `claimed 1` included — that line needed `examples/queue/receipt.nvs` to park, for the
reason the playbook bullet gives.

**Stage 2's `-p nvs-db` check still fails and the analysis is unchanged**: its first two names are
`crates/nvs-db/tests/handshake.rs` and pass under `python tools/db-matrix.py --driver postgres`;
`a_named_connection_is_memoized_for_the_request` is `Core\Db::connect`'s and so `nvs-stdlib`'s,
`local_infile_is_refused_and_no_file_is_sent` is MySQL's with no driver, and
`the_connection_charset_is_forced_to_utf8` is already asserted inside `pg.rs`. The check's `args`
is the fix, per the playbook's *a `loop-goal.toml` check can name a test in a crate that cannot
host it*.

## Next group

**Stage 8's `-p nvs-db` names, none of which exists on disk — one new matrix-gated
`crates/nvs-db/tests/queue.rs` over the statements in `crates/nvs-stdlib/src/queue.rs`, with
`crates/nvs-db/tests/handshake.rs` as the gating shape. All three slices share that file set.**

- [ ] **An exhausted job reaches the dead-letter table** — ADR 0084 § 6,
      `docs/agent/loop-goal.toml:2964`. Push with `crates/nvs-stdlib/src/queue.rs:246`, claim with
      `crates/nvs-stdlib/src/queue.rs:295`, run `crates/nvs-stdlib/src/queue.rs:376` on that lease:
      assert the row left `nvs_jobs`, kept its `id` and `queue`, and carries `errors`. Gate it like
      `crates/nvs-db/tests/handshake.rs:43`, which is the only way a test reaches a real `PgConn`.
- [ ] **The claim is `skip locked`-shaped, and a visibility timeout returns an abandoned job** —
      § 4, `crates/nvs-stdlib/src/queue.rs:295`. Two connections claiming one due row come back
      with one job and none; a row whose `claimed_at` predates the cutoff is claimable again.
- [ ] **An enqueue commits with the write that made it** — § 3,
      `crates/nvs-stdlib/src/queue.rs:246` inside a `BEGIN`/`ROLLBACK` on one connection: the
      rolled-back half leaves no job, which is the property the whole ADR is built around.

## Backlog

- `retries_are_bounded_and_backoff_is_jittered` — stage 8's sixth name, but
  `a_retry_is_exponential_jittered_and_capped` in `-p nvs-stdlib` already holds the ladder:
  `docs/agent/loop-goal.toml:2964`.
- A `-p nvs-cli` test over `report`'s three branches — `crates/nvs-cli/src/worker.rs:463`; there is
  no seam that builds a `PgConn`, per the playbook.
- Stage 2's four unwritable names — `docs/agent/loop-goal.toml:2760`.
- `Core\Queue` reads `nvs_dead_jobs` for depth and `status` only; nothing hands a caller the row —
  docs/adr/0084-durable-background-jobs.md § 1.
- `Core\Db::open` still waits on a shape-parameter type — `crates/nvs-stdlib/src/db.rs`'s gaps.
