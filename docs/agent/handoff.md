# Handoff

## State

**§ 13's pool for `open` is live.** `nvs_runtime::pool::Ticket::for_settings` carries the hash
`settings_key` computes instead of rendering a key out of a block name, and is deliberately not
generation-scoped — that constructor's own doc owns the argument, and the `Option` field it leaves
`None` says the same thing from the struct's side. `open` now runs `connect`'s
bounds-then-ticket-then-`admit` shape and files a `Some(lease)`, so `{shared: false}` draws from and
returns to the pool as well. A settings literal has no `[db.<name>.pool]` table, so its bounds are
`PoolBounds::DEFAULT`; what that spends — and the `pool = false` an operator cannot reach an `open`
with — is this module's rewritten known gap 1.

**The pool key hashes two fields the memo did not need:** § 9's declared zone and the statement
cache's size. A drawn connection carries both from the request that opened it, and § 13's reset
restores the zone rather than re-reading it, so leaving them out would have answered a second
request in the first one's zone.

**The driver's stage-7 check cannot close yet, and the reason is the second cause, not a filing
bug.** `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server, and there is
no TDS driver at all: `TdsConn` in `crates/nvs-db/src/conn.rs` is a busy-state cell and nothing
else. MySQL's half is landed and already asserted, by `a_reset_invalidates_the_statement_cache` in
`crates/nvs-db/src/mysql.rs`. The check is correctly filed and stays open until that driver exists —
do not rename it, split it, or write a MySQL-only test under that name.

## Next group

**Three slices in one file: `crates/nvs-stdlib/src/db.rs`.** The first is what ADR 0067 § 7 requires
and known gap 9 names; the other two are the gaps a session already in that file can close beside
it.

- [ ] **§ 7's wait between attempts — exponential backoff and jitter that suspends the coroutine**
      (0067 § 7). `crates/nvs-stdlib/src/db.rs:5872` is the comment where the loop re-runs with no
      wait, inside `transacted` at `crates/nvs-stdlib/src/db.rs:5741`; known gap 9's text to rewrite
      is `crates/nvs-stdlib/src/db.rs:170`. `wait_for_slot` at `crates/nvs-stdlib/src/db.rs:4223` is
      the shape for a bounded park — `host.park(Some(until))` through
      `nvs_runtime::host::with_current`, with the `Woken::Cancelled` arm that ends the wait. Decide
      where the jitter's randomness comes from: this module reaches no RNG today, and a dependency
      for one is ADR 0051 § 4's question rather than a free pick.
- [ ] **`open` opens a MariaDB** (0067 § 2, known gap 2). The driver landed —
      `crates/nvs-db/src/maria.rs:328` is `MariaConn`'s own `impl` and
      `crates/nvs-db/src/maria.rs:215` is `MariaTarget` — but `open`'s match still refuses it.
      `crates/nvs-stdlib/src/db.rs:3353` is the Postgres arm to follow, one arm above the
      `other =>` refusal that currently names MariaDB.
- [ ] **§ 8's `sql` is the fifth raw value and no throw carries it** (0067 § 8).
      `crates/nvs-stdlib/src/db.rs:3685` is `statement_failure`, which builds the four that are
      carried; `crates/nvs-stdlib/src/db.rs:200` is the module doc's account of how a kind reaches
      `nvs_runtime::KIND_SLOT`, which is where a fifth value has to fit.

## Backlog

- The stage-7 check's mssql half waits on the TDS driver; its ADR slot is pre-authorized as
  stage 2 item 3 of `docs/agent/loop-goal.md`.
- Where an operator writes pool bounds for a key only the program knows — an ADR 0067 § 13
  question, and this module's known gap 1.
- `open` still refuses SQLite and SQL Server outright — known gap 2, same module.
- SQLite has no code table, so it contributes no column to
  `every_driver_normalises_its_codes_to_one_error_kind` in `crates/nvs-db/src/conn.rs`.
- `docs/agent/loop-goal.toml` and `docs/agent/goals/5-database.toml` are still not byte-identical —
  the live one carries ADR 0133's stage 0, so an edit goes into both by hand.
- `[context] modules` in `docs/agent/loop-goal.toml` names no `nvs-runtime` or `nvs-config` pattern,
  so the map printed nothing for `crates/nvs-runtime/src/pool.rs` even though the item anchored on
  it; both crates own half of this goal's pool work.
