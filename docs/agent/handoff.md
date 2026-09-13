# Handoff

## State

**Goal `process-cache` — stage 6 is landed, and both of its checks are green.** The fill table is
`crates/nvs-stdlib/src/cache.rs`: `elected` (:1297) hands one caller per process the key and answers
a `LogicError` to a `fill` that asked for the key it is filling, `waited` (:1340) looks again on a
`FILL_TICK` and throws `TimeoutError` at the bound `wait_of` answers, and `filled` (:1692) runs the
closure and stores what it answered. The hold is a guard, so a throw, a cancellation and the
ordinary answer all release the key.

**A sealed entry now carries the lifetime it was written for beside its expiry** —
`sealed_plaintext` (:1534) — because that is the only place refresh-ahead's last fifth
(`REFRESH_WINDOW`, :1502) can be read from: the shared tier keeps no deadline this process can see.
An entry sealed by an older build would misparse, and nothing on disk holds one. `putSecret` and a
`fill` write through one `seal_and_store` (:1580), so what a fill leaves behind is the entry the
other door would have written.

**The election spans the tiers on purpose.** Its key is `scoped(ctx, &sealed_key(key))`, so one name
is one fetch per process whichever store the answer goes to; a caller on another tier that missed a
fill is answered a miss rather than a value from somewhere else.

**The `[context]` gap is unchanged:** the goal's own stage prose (`docs/agent/loop-goal.md:118-137`)
is reachable from no field, so a session meets a stage only through the header the driver prints
above a failing check.

## Next group

**Stage 7: a user's secret in the session** — one file set: `crates/nvs-stdlib/src/session.rs`,
reading `crates/nvs-stdlib/src/cache.rs:1481` and `crates/nvs-stdlib/src/crypto.rs:1473`.

- [ ] **`setSecret` and `getSecret` on the session store, sealed under a key ring** — the five edits
      twice, rows beside `crates/nvs-stdlib/src/session.rs:135` and bodies beside
      `crates/nvs-stdlib/src/session.rs:886` (`set`) and `crates/nvs-stdlib/src/session.rs:848`
      (`get`) — `rule:http-server/a-session-holds-a-secret-only-sealed`. The seal is
      `crates/nvs-stdlib/src/crypto.rs:1473` and the record holds ciphertext, which
      `session_secret_is_stored_as_ciphertext_in_the_record` is the test for.
- [ ] **A value sealed for the cache does not open as a session secret**: the cache's domain octet
      is `crates/nvs-stdlib/src/cache.rs:1440` and its additional data is
      `crates/nvs-stdlib/src/cache.rs:1481`, so the session states its own — test
      `a_value_sealed_for_the_cache_does_not_open_as_a_session_secret`.
- [ ] **The three cases**: `tests/conformance/core/session-a-secret-round-trips-sealed-and-survives-regenerate.nvst`,
      `tests/conformance/core/session-get-answers-null-for-a-sealed-value.nvst` and
      `tests/conformance/reject/session-set-refuses-a-secret-and-names-set-secret.nvst`, whose
      refusal names `setSecret` from `crates/nvs-stdlib/src/session.rs:886`.

## Backlog

- Refresh-ahead has no `.nvst` case; a program sees it with a short `ttl` and a sleep —
  `tests/conformance/core/`.
- A waiter polls on `FILL_TICK` because a `Waker` belongs to one task and the table is every core's;
  the tick goes away when a wake can cross cores — `crates/nvs-stdlib/src/cache.rs:1215`.
- A fleet-wide single fill is a lease over the shared tier and not this goal —
  `docs/agent/loop-goal.md` § *Standing decisions*.
- The `nvs/rest` package is the first caller of all of this — `docs/agent/carried-gaps.md`.
