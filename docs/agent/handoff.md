# Handoff

## State

**A worker reports the attempt it ran.** `crates/nvs-cli/src/worker.rs:435`'s `report` writes
`nvs_stdlib::queue::SUCCEEDED` for a completion whose `ok` is true and `queue::RETRY` for one that
is not, both keyed on the lease — `id` *and* the `claimed_at` this worker's own claim wrote — so a
worker that overran § 4's visibility window cannot cancel the attempt that replaced it. The retry's
`run_at` is ADR 0084 § 6's ladder, `crates/nvs-stdlib/src/queue.rs:379`'s `retry_at`: exponential in
the attempts the claim already counted, capped at five minutes, and jittered off the job's own id so
that a hundred jobs failing against one endpoint do not come back together. Verified against the
live container end to end — a receipt job lands in `Succeeded` at one attempt, and `flaky.nvs`
throws, retries and exhausts.

**An exhausted job is the one branch left open**: it is announced on standard error and left
`Claimed`, which the worker's module doc carries as its *Known gap*.

**Stage 2's first two `-p nvs-db` names exist**, as `crates/nvs-db/tests/handshake.rs` — matrix-gated
like `pool_reuse.rs`, so they skip with no container and assert against a real PostgreSQL under
`python tools/db-matrix.py --driver postgres`, where both pass. **The check still fails, and its
other four names are not writable yet**: `local_infile_is_refused_and_no_file_is_sent` is MySQL's and
there is no MySQL driver, `a_named_connection_is_memoized_for_the_request` is `Core\Db::connect`'s
and so `nvs-stdlib`'s rather than a `-p nvs-db` test at all, `no_driver_path_interpolates_a_value_into_sql`
is a claim over five drivers of which one exists, and `the_connection_charset_is_forced_to_utf8`'s
claim is already asserted inside `pg.rs`'s successful-handshake case. The playbook's *a
`loop-goal.toml` check can name a test in a crate that cannot host it* is the shape of the first two.

## Next group

**§ 6's floor, over `crates/nvs-cli/src/worker.rs`, `crates/nvs-stdlib/src/queue.rs` and
`examples/queue.nvs` — the same three files this session held.**

- [ ] **The dead-letter move** — ADR 0084 § 6. Replace the exhaustion branch at
      `crates/nvs-cli/src/worker.rs:451` with a statement beside
      `crates/nvs-stdlib/src/queue.rs:350`: one `with moved as (delete from nvs_jobs … returning …)
      insert into nvs_dead_jobs …`, keyed on the lease as `SUCCEEDED` is. `nvs_jobs` has no column
      holding earlier attempts' errors (`crates/nvs-stdlib/src/queue.rs:205`'s DDL), so § 6's
      `errors` array can only carry this attempt's — decide that in the slice and say so in the
      column's doc.
- [ ] **The fixture's last two lines print off the state they name** —
      `examples/queue.nvs:112`'s `deadLettered` poll passes once the move lands, and
      `examples/queue.nvs:106`'s `attempts` poll already does; what is left is the ~5s the
      dead-letter poll still waits out. Stage 8's frozen `want` is `enqueued`, `claimed 1`, `ran`,
      `retried`, `dead-lettered`.
- [ ] **A `-p nvs-cli` test over `report`'s three branches** — `crates/nvs-cli/src/worker.rs:435`.
      Nothing pins which statement each completion picks; the crate has no PostgreSQL double, so the
      cheap half is `retry_at`'s ladder (already pinned in `-p nvs-stdlib`) plus a test that the two
      statements name the lease columns the claim writes.

## Backlog

- Stage 2's four remaining `cargo-named` names need the goal file corrected, not tests — see *State*;
  fix `docs/agent/goals/<goal>.toml` alongside the live copy (`docs/agent/playbook.md`).
- `examples/queue/nope.nvs` rows from an old experiment sit in the shared test database and are
  claimed by every fixture run; harmless, but they print two warnings per run.
- ADR 0084 § 5's "grants narrowed from those recorded at enqueue" still has no column
  (`crates/nvs-cli/src/worker.rs`'s module doc).
- § 4's visibility timeout re-claims an exhausted job forever until the dead-letter move lands.
- Stage 5 is four of seven in `-p nvs-db` (`docs/implementation-plan.md`).
