# Handoff

## State

**ADR 0084 § 2's `[queue]` block is live end to end at boot.** `crates/nvs-config/src/queue.rs`
holds `QueueBounds` and `queue_for`, wired into `resolve.rs`'s pass list immediately after
`db::validate` — the `[db]` roster it checks `connection` against only exists once the merge is
done. `crates/nvs-config/tests/queue.rs` is six cases over it. The block itself is
`tree::Queue`, four `Option` fields with `deny_unknown_fields`, so § 2's own example parses whole.

**`E0617` is the new code**, `E_BAD_QUEUE`, for the four things a `[queue]` can say that leave
nothing able to run a job: no `connection`, one naming a `[db.<name>]` the tree does not hold, a
`max_attempts` of `0`, a `visibility` of `0`. Its reasoning is `E0611`'s one subsystem over — a
queue fails silently, so every question is a boot question. A `visibility` that is not a duration
at all stays `E0601` from `nvs_config::value`, in that module's words.

**Defaults are § 2's own numbers** — `workers = 4`, `max_attempts = 5`, `visibility = 5m` — and
`connection` has none, because naming the database is the block's whole point. `workers = 0` is
accepted: § 2 states it as an enqueue-only instance, not a disabled queue. The block's own absence
is how a deployment has no queue, so `queue_for` answers `Ok(None)` rather than a refusal.

**Unchanged and still true.** The driver's acceptance line for `examples/queue.nvs` is still
Stage 8's `Core\Queue` surface, which the two items below land; nothing of that class is in
`nvs_stdlib::registry` yet. Stage 6's `mariadb: n/a` / `mssql: n/a` are did-not-run. § 7's backoff
is still blocked on `nvs-runtime`'s known gap 3 (`crates/nvs-stdlib/src/db.rs:149`). Stage 5's
`args = ["test", "-p", "nvs-db"]` still cannot see the two `nvs-stdlib` tests — the user's call.

**`orient.py`'s pack was short in the same two places the last session named**, and one more:
`[context] modules` still names no `nvs-config/src/*`, so the map printed nothing for the crate
this session wrote in, and `[context] adrs` should carry ADR 0084 §§ 1 and 5 for the group below.

## Next group

**`Core\Queue`'s surface, over the connection `[queue]` now names. The file set is a new
`crates/nvs-stdlib/src/queue.rs`, `crates/nvs-stdlib/src/registry.rs` and
`crates/nvs-stdlib/src/db.rs` — the last for its connection-reaching shape only, not to edit.**

- [ ] **`Core\Queue::push` inserts a row on the queue's own connection** — ADR 0084 §§ 1 and 3.
      The class goes in `CLASSES` beside `Core\Db` at `crates/nvs-stdlib/src/registry.rs:1399`, the
      body reaches its connection the way `nvs_core_db_connect` does at
      `crates/nvs-stdlib/src/db.rs:2298`, its `address()` arm sits with the others at
      `crates/nvs-stdlib/src/db.rs:5040`, and the connection's name comes from
      `nvs_config::queue::queue_for` at `crates/nvs-config/src/queue.rs:98`, whose `connection`
      field is already proven to name a real block. § 2's tables are the operator's
      `nvs queue migrate` and are not created here.
- [ ] **`cancel`, `status` and `stats` complete § 1's roster** — same `queue.rs`, same registry
      row at `crates/nvs-stdlib/src/registry.rs:1399`, three more `address()` arms in the new
      module's own `address`. `stats` reports dead-letter depth, which § 6 requires and
      `crates/nvs-config/src/queue.rs:49`'s `max_attempts` is the bound behind.
- [ ] **`examples/queue.nvs` runs against the compose PostgreSQL** — the acceptance line the
      driver has been failing since stage 8 opened. It needs the two items above plus a `[queue]`
      block in the example's own `nvs.toml`, resolved by `crates/nvs-config/src/queue.rs:81`.

## Backlog

- § 7's backoff, blocked on `nvs-runtime`'s known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- Stage 5's `-p nvs-db` args cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml`.
- `[context] modules` names no `nvs-config` pattern — `docs/agent/loop-goal.toml`.
- MariaDB and SQL Server are `n/a` in the matrix — `tools/db-matrix.py`, `tests/db/compose.yaml`.
- `open` still waits on a shape-parameter type — ADR 0067 § 2, `crates/nvs-stdlib/src/db.rs`.
- `nvs queue migrate` owns the schema and does not exist — ADR 0084 § 2.
