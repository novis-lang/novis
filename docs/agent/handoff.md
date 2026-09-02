# Handoff

## State

**§ 7's options bag is on the `transaction` row**, at
`crates/nvs-stdlib/src/db.rs:508`'s `TRANSACTION_OPTIONS`: `isolation` as
`CoreTy::Enum(ISOLATION_NAME)` defaulting to `Const::Null` (absent, so the server's own level
stands), `readOnly` as `bool` defaulting to false, `retries` as `uint` defaulting to 0. The card
carries one `ParamDoc` per option and the registry test now pins the three names, their order and
their defaults as one comparison, so an option added without being specified fails there.

**Two of the three are wired and the third is declared and unread.**
`nvs_core_db_connection_transaction` (`crates/nvs-stdlib/src/db.rs:3300`) is `args: [5]` and hands
`isolation`/`readOnly` straight to `nvs_db::PgConn::begin`, which already renders them and already
refuses a *nested* call that carries either — that refusal is an `InvalidInput` and so a
`LogicError`, and the card names it. `retries` arrives in slot 4 and nothing reads it; **this
module's new known gap 9 is the whole of what is left**, including why: § 7's backoff suspends the
coroutine and `nvs-runtime` has no yielder (`crates/nvs-runtime/src/lib.rs:208`). A call that does
not write `{retries: n}` is unaffected, and one that does gets the conflict surfaced instead —
weaker than § 7, never wrong about what happened.

**The `args` wall is unchanged and is still the user's call**: Stage 5's
`args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) cannot see the two
`nvs-stdlib` tests, so five of its seven names stay unfound there.

**The driver's acceptance line still names `examples/queue.nvs`** — Stage 8's unlanded `Core\Queue`
(ADR 0084), not a regression. **The CA is still not in git**; `nvs_host::tls`'s module doc owns why.

**`orient.py` did not print ADR 0067 § 7** — the pack carried §§ 1, 9 and 13, and § 7 is the section
every slice of this group is specified by. Add `0067 § 7` to `[context] adrs`.

## Next group

**§ 7's retry loop — the one half of the bag that does not work yet. The file set is
`crates/nvs-db/src/pg.rs` and `crates/nvs-stdlib/src/db.rs`.**

- [ ] **A depth reader on the connection** — `pub fn depth(&self) -> u32` beside
      `crates/nvs-db/src/pg.rs:698`'s `begin`, over the `Cell` initialised at
      `crates/nvs-db/src/pg.rs:619`. § 7 retries **outermost transactions only** and nothing outside
      that crate can currently tell which a call is. ADR 0067 § 7.
- [ ] **The decision the loop cannot be written without, and it is one call** — § 7 wants
      exponential backoff with jitter *that suspends the coroutine*, and there is no yielder
      (`crates/nvs-runtime/src/lib.rs:208`). Choose between waiting on the blocking pool
      (`crates/nvs-host/src/blocking.rs:320`, the way `crates/nvs-stdlib/src/process.rs:368` waits —
      keeps the core free, holds a pool worker) and retrying with no wait at all, and record the
      choice in known gap 9 at `crates/nvs-stdlib/src/db.rs:64`. `rand` is already this crate's, for
      the jitter. Pre-authorized: it is a design call, not a `BLOCKED`.
- [ ] **The loop itself** — `crates/nvs-stdlib/src/db.rs:3300`: re-run the closure while attempts
      are under slot 4's count, the depth was 0, and the failure's
      `nvs_db::DbErrorKind::is_retryable` (`crates/nvs-db/src/conn.rs:335`) is true. Each attempt
      needs its own `BEGIN` **and its own scope object** — the one built at
      `crates/nvs-stdlib/src/db.rs:3300` is closed and discarded on every path already, so the retry
      goes around that whole block, not inside it. ADR 0067 § 7.

## Backlog

- `examples/transaction.nvs` writes no `{isolation: …}`, so nothing exercises
  `BEGIN ISOLATION LEVEL` end to end — ADR 0067 § 7.
- Widening Stage 5's check `args` is one edit that closes Stage 4's and Stage 5's together —
  `docs/agent/loop-goal.toml:2830`.
- Stage 8's `Core\Queue` is what the driver's acceptance check names — ADR 0084.
- `open` waits on a registry type for a shape *parameter* — `crates/nvs-stdlib/src/db.rs` gap 1.
- `stream`/`streamAs` are owed on both `Queryable` classes — that module's gap 5.
- `Db\DbError` is not in spec § 10's tree, so a refusal carries no `kind` — that module's gap 4.
