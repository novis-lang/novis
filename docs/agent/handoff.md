# Handoff

## State

**§ 13's pool is proven live six ways, and the last two are the bounds that decide what a
pool keeps.** `crates/nvs-db/tests/pool_reuse.rs` now holds four cases over a real server:
reuse, `pool = false`, a connection past its `lifetime`, and a release past `idle`. What
each adds over `nvs_runtime::pool`'s own `Fake` cases is the same thing in both new ones —
the retirement and the over-bound release are a **close**, so the next request is answered
by a different backend, and the connection the pool kept is the one released *under* the
bound rather than the one past it. `pg_backend_pid()` is the whole of the identity, as it
is for the two cases already there. Both names are in `docs/agent/loop-goal.toml`'s stage 7
`cargo-named` check, and `python tools/db-matrix.py --driver postgres` is green.

**The clock moves, not the bound.** `pool::release` and `pool::take` are each handed the
`Instant` they compare against, so the `lifetime` case tests § 13's real 30-minute default
and costs no wall time; only the `idle` case names a bound of its own (`idle = 1`, the
smallest that still keeps something, so "which connection survived" has an answer).

**Unchanged and still true.** The driver's acceptance line for `examples/queue.nvs` is
Stage 8's unlanded `Core\Queue` (ADR 0084), not a regression — nothing of that class is on
disk, and the next group opens it. Stage 6's `mariadb: n/a` / `mssql: n/a` are did-not-run.
§ 7's backoff is still blocked on `nvs-runtime`'s known gap 3
(`crates/nvs-stdlib/src/db.rs:149`). Stage 5's `args = ["test", "-p", "nvs-db"]` still
cannot see the two `nvs-stdlib` tests — the user's call.

**`orient.py`'s pack is still short.** `[context] modules` names none of
`nvs-runtime/src/pool.rs`, `nvs-config/src/db.rs` or `nvs-stdlib/src/db.rs`, and this
session read all three; `[context] adrs` should gain ADR 0084 §§ 1-2 for the group below.

## Next group

**`Core\Queue` opens — ADR 0084's surface, over the `Core\Db` connection an operator names.
The file set is a new `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-stdlib/src/registry.rs`
and `crates/nvs-config/src/tree.rs`, and nothing of the class exists yet: `Core\Queue` is
not in `CLASSES` and there is no `[queue]` block in the config tree.**

- [ ] **`[queue]` is a config block naming a `[db.<name>]`** — ADR 0084 § 2. `connection`,
      `workers` and the finite `maxAttempts` § 6 requires, resolved and trust-checked at
      boot exactly as `db` is: `crates/nvs-config/src/tree.rs:653` is the neighbouring
      block, `crates/nvs-config/src/db.rs:166` the resolver to copy. Everything below
      needs it, and it is the smallest of the three.
- [ ] **`Core\Queue::push` inserts a row on the queue's own connection** — ADR 0084 §§ 1
      and 3, the property the design exists for. A new `crates/nvs-stdlib/src/queue.rs`
      registered in `crates/nvs-stdlib/src/registry.rs:1099`, running its statement the way
      `crates/nvs-stdlib/src/db.rs:370`'s class does; `Queue\Id` is an instance class in
      the shape of `crates/nvs-stdlib/src/channel.rs:117`.
- [ ] **`cancel`, `status` and `stats` complete § 1's roster** — same `queue.rs`, with
      `Queue\State` the enum `examples/queue.nvs:31` already imports and
      `crates/nvs-stdlib/src/registry.rs:1099` the one place it is declared.

## Backlog

- § 2's two tables and `nvs queue migrate --dry-run` naming both — `docs/agent/loop-goal.toml` stage 8.
- § 4's claim statement, `FOR UPDATE SKIP LOCKED` per backend — ADR 0084 § 4, `crates/nvs-db/src/pg.rs`.
- § 6's bounded retries, jittered backoff and the dead letter — ADR 0084 § 6.
- Stage 9's literal-query diagnostics in `-p nvs-types` — `docs/agent/loop-goal.toml` stage 9.
- § 7's `transaction` backoff, blocked on `nvs-runtime`'s known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- Stage 5's two `nvs-stdlib` tests no `-p nvs-db` check can see — the user's call.
