# Handoff

## State

**Stage 5's `nvs-stdlib` half is two of its three names**, both `#[test]`s in
`crates/nvs-stdlib/src/db.rs`'s own test module:
`a_transaction_is_a_closure_and_transaction_is_a_queryable` (§ 7's closure form, the *absence* of
`commit`/`rollBack`/`inTransaction` on the connection, and the delegation asserted as identical rows
— symbol and position included — over `CONNECTION`'s whole roster, so a member added there fails
until `TRANSACTION` carries it) and `roll_back_survives_an_intervening_catch_of_throwable` (the
reason recorded in `REASON_SLOT` outlives both `ctx.take_pending()` and the scope close, which is the
order `nvs_core_db_connection_transaction` reads it in).

**The third name is not a test slice.** § 7's options bag is not on the row at all, so there is
nothing to retry; the row's doc at `crates/nvs-stdlib/src/db.rs:480` said the enum was missing and
that is now false and corrected. Two of the three things `{retries: n}` needs have landed —
`ISOLATION` is registered (`crates/nvs-stdlib/src/db.rs:625`) and
`nvs_db::DbErrorKind::is_retryable` (`crates/nvs-db/src/conn.rs:335`) already names the two kinds
§ 7 re-runs on. It is the next group, as a feature.

**The `args` wall is unchanged and is the user's call**: the Stage 5 check's
`args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) cannot see either test landed
this session, so five of its seven names stay unfound there. One widening closes Stage 4's check and
Stage 5's together.

**The driver's acceptance line still names `examples/queue.nvs`**, Stage 8's unlanded `Core\Queue`
(ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s module doc owns why.

## Next group

**§ 7's `{retries: n}` and the options bag it lives in. The file set is
`crates/nvs-stdlib/src/db.rs`, with one read of `crates/nvs-stdlib/src/registry.rs`.**

- [ ] **The bag on the row** — `CoreTy::Options(&[CoreOption])` as the last parameter of
      `crates/nvs-stdlib/src/db.rs:487`, `{isolation?, readOnly?, retries?}` at § 7's defaults, and
      one `ParamDoc` per option in the card at `crates/nvs-stdlib/src/db.rs:1496`. The variant and
      its rules are `crates/nvs-stdlib/src/registry.rs:594` and
      `crates/nvs-stdlib/src/registry.rs:608`; a bag is always last and never nested. ADR 0067 § 7.
- [ ] **The helper reads them** — a bag flattens to one argument per option, so
      `crates/nvs-stdlib/src/db.rs:3166`'s `args: [2]` becomes `args: [5]`, with `isolation` and
      `readOnly` reaching the `begin` call at `crates/nvs-stdlib/src/db.rs:3168`. ADR 0067 § 7.
- [ ] **The retry loop** — outermost transactions only, on `is_retryable` alone, default 0,
      exponential backoff with jitter that suspends the coroutine the way
      `crates/nvs-stdlib/src/time.rs:3414` does rather than blocking the core. **Check first how the
      kind survives the closure's throw**: the deadlock is raised by a statement *inside* the
      closure and arrives at `crates/nvs-stdlib/src/db.rs:3184` as a pending exception, and
      `statement_failure` at `crates/nvs-stdlib/src/db.rs:2394` is where the kind either travels or
      is lost. ADR 0067 § 7.

## Backlog

- `retries_recover_an_induced_deadlock` needs two connections against the matrix, and the loop it
  tests is `nvs-stdlib`'s — the same `args` wall — `docs/agent/loop-goal.toml:2830`.
- Stage 6's four drivers gate two Stage 5 names — `docs/agent/loop-goal.toml:2830`.
- Stage 4's four leftovers are the same `args` question — `docs/agent/loop-goal.toml:2806`.
- Stage 7's pool is `-p nvs-db` work with nothing on disk yet — `docs/agent/loop-goal.toml:2884`.
- `examples/queue.nvs` needs Stage 8's `Core\Queue` — ADR 0084.
- `open` waits on a shape-parameter type — `docs/implementation-plan.md`.
