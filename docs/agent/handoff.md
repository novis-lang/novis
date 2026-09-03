# Handoff

## State

**Stage 5's `-p nvs-stdlib` check is closed — all three of its names run.** § 7's retry loop is no
longer inside `nvs_core_db_connection_transaction`: it is `transacted` at
`crates/nvs-stdlib/src/db.rs:5652`, handed an `Attempts` — the four questions an attempt asks of its
connection — instead of reading one back out of the ctx at four points. `Filed` is the only
implementation the runtime builds and does exactly what those four sites did; the trait's own doc
owns why the methods take the context rather than borrow out of it, and why three of them answer a
`Result` inside a `Result`.

**`retries_recover_an_induced_deadlock` induces the conflict the way a server does.** The scripted
closure throws what `statement_failure` renders a PostgreSQL `40P01` into, so the loop's decision
goes through `Ctx::pending_slot` and § 8's normalised kind exactly as it does in a request. It reads
the commands back in order rather than counting attempts, and asserts both sides of § 7's bound —
at `retries: 0` the same conflict reaches the caller with the closure run once.

**`nvs_stdlib::db`'s known gap 1 is untouched, and one thing about it is now known:** an `open` pool
has no `[db.<name>].pool` table to take its bounds from. `connect` resolves them with
`nvs_config::db::pool_for(name, block, …)` at `crates/nvs-stdlib/src/db.rs:2916`, and a settings
literal has no block — so slice 1 below has to decide what bounds a blockless pool gets (the
defaults, or `PoolBounds::OFF`) before it can build a ticket. Within a request § 2's memo holds.

## Next group

**§ 13's pool for `open`, then what the docs still say about it. File set:
`crates/nvs-stdlib/src/db.rs` with `crates/nvs-runtime/src/pool.rs`.** The first two are the group
the last three handoffs named and are unchanged; the third is the paragraph they invalidate.

- [ ] **A ticket keyed on the settings hash, so `open` pools** (0067 § 13). `Ticket` is
      `crates/nvs-runtime/src/pool.rs:127` and its one constructor
      `crates/nvs-runtime/src/pool.rs:152` takes a block *name*, which
      `crates/nvs-runtime/src/pool.rs:180` renders as `{generation:p}:{name}`; the hash `open`
      already computes is `crates/nvs-stdlib/src/db.rs:3132` and cannot collide with a name.
      `crates/nvs-stdlib/src/db.rs:2916` is `connect`'s bounds-then-ticket-then-`admit`, the shape
      to follow, and `crates/nvs-stdlib/src/db.rs:3284` is `open`'s `hold_open_connection` with the
      `None` lease that is the whole gap. Decide the blockless bounds question named in `## State`
      and say what it spends.
- [ ] **`{shared: false}` still draws from and returns to that pool** (0067 § 13). § 13 says the
      option bypasses memoization within the request and never pooling across requests;
      `crates/nvs-stdlib/src/db.rs:2986` is where `connect` already words that and files the lease
      beside a `None` memo, and `crates/nvs-stdlib/src/db.rs:3284` is where `open` must do the same.
- [ ] **The two places that still say `open` does not pool** (0067 § 13). Known gap 1 is
      `crates/nvs-stdlib/src/db.rs:66` and names `Ticket::for_block` as the reason; the member's own
      *what it spends* paragraph is `crates/nvs-stdlib/src/db.rs:3165`. Both are wrong the moment
      slice 1 lands, and `OPEN_DOC`'s card is worth a look beside them.

## Backlog

- § 7's wait between attempts — exponential backoff and jitter that suspends the coroutine. This
  module's known gap 9, `crates/nvs-stdlib/src/db.rs`.
- MariaDB and SQL Server have no `connect`, so `open` refuses both. Known gap 2, same module.
- § 8's `sql` is the fifth raw value and no throw carries it yet — ADR 0067 § 8.
- SQLite has no code table, so it contributes no column to
  `every_driver_normalises_its_codes_to_one_error_kind` in `crates/nvs-db/src/conn.rs`.
- `docs/agent/loop-goal.toml` and `docs/agent/goals/5-database.toml` are still not byte-identical —
  the live one carries ADR 0133's stage 0, so an edit goes into both by hand.
