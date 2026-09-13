# Handoff

## State

**Goal `process-cache` — stage 6's bound is readable; the fill it bounds is not.** `wait_of`
(`crates/nvs-stdlib/src/cache.rs:1208`) answers the `{wait?}` a call wrote, then
`[cache.process] fill_wait`, then the `5s` `DEFAULT_FILL_WAIT` states
(`crates/nvs-stdlib/src/cache.rs:1093`), so an omitted option inherits the bound instead of removing
it and a directive that will not parse leaves it standing. Nothing calls it yet — it carries
`#[cfg_attr(not(test), expect(dead_code, …))]`, and the slice that builds the table is what deletes
that attribute. `getSecret`'s body (`crates/nvs-stdlib/src/cache.rs:2096`) still reads neither slot,
and its doc paragraph says so.

**The fill's three missing pieces each have an anchor now.** A `fill` is invoked through
`nvs_runtime::closure::call_closure` (`crates/nvs-runtime/src/closure.rs:186`), which takes a
`&mut Ctx`; a bounded wait is `nvs_host`'s — `timer::park_until` (`crates/nvs-host/src/timer.rs:251`)
and `group::park` (`crates/nvs-host/src/group.rs:162`) — because neither `nvs-stdlib` nor
`nvs-runtime` names a `Condvar`, and a `std` one would hold a worker for the whole wait. The
refresh-ahead window is the last fifth of a lifetime, fixed in `docs/decisions/0181.md` and stated in
`docs/spec/01-core-library.md:1190`.

**The reference floor is green, and the prose under it is the user's live work.** `Core\Cache` has a
hand-written chapter (`docs/reference/core/Cache.md`) and `docs/novis.md` is its regeneration, both
committed by hand at `cd70da4f0`; that chapter was being edited while this session ran, so a red
`reference.py` check here is prose to regenerate rather than a member to fix — the playbook bullet
says how to tell the two apart.

**The `[context]` gap is unchanged:** the goal's own stage prose (`docs/agent/loop-goal.md:118-137`)
is reachable from no field, so a session meets the stage only through the header the driver prints
above a failing check.

## Next group

**Stage 6: the fill table** — one file set: `crates/nvs-stdlib/src/cache.rs`, reading
`crates/nvs-runtime/src/closure.rs` and `crates/nvs-host/src/timer.rs`.

- [ ] **Exactly one caller in the process runs `fill` and every other waits for it**, each for at
      most the bound `wait_of` answers and past it a `TimeoutError` —
      `rule:concurrency/a-secret-fill-runs-once-per-process`. The table is keyed the way the entry is
      (`scoped(ctx, &sealed_key(key))`), the miss to fill from is
      `crates/nvs-stdlib/src/cache.rs:2096`, the reader to call is
      `crates/nvs-stdlib/src/cache.rs:1208` — deleting its `expect(dead_code)` — the `fill` is
      invoked through `crates/nvs-runtime/src/closure.rs:186`, and the wait parks on
      `crates/nvs-host/src/timer.rs:251`.
- [ ] **A failure is never shared, and a request that ends releases the key**: a throwing `fill`
      releases its waiters with nothing and the next caller runs `fill` itself, a cancelled request
      never wedges a key, and a `fill` that asks for its own key is a `LogicError` rather than a wait
      on itself — same rule, same body at `crates/nvs-stdlib/src/cache.rs:2096`.
- [ ] **Refresh ahead**: an entry inside the last fifth of its lifetime is answered to every caller
      while exactly one of them runs `fill` to replace it, so a hot key's expiry costs nobody a wait.
      The expiry the fraction is taken of is the one `sealed_value` reads, at
      `crates/nvs-stdlib/src/cache.rs:2096`.

## Backlog

- Stage 7's `setSecret`/`getSecret` on `Core\Session`, sealed the same way — same goal file.
- Stage 8 flips this goal's four rules to `shipped` and re-renders the rulebook.
- The stage's own prose is unreachable from `[context]`; the stage header the driver already prints
  above the failing check is where it would belong.
- `Core\Cache\SecretEntry` answers nothing, so nothing reads one back — `getSecret` stays the only
  door (`docs/agent/loop-goal.md` § *Standing decisions*).
- `docs/reference/core/Cache.md` is hand-written and under live edit; `python tools/reference.py`
  after it settles is what keeps the one-file reference green.
