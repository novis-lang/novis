# Handoff

## State

**Goal `process-cache` — stage 6's surface is on disk; its behaviour is not.** `getSecret`'s row
carries `{fill?: callable(): Core\Cache\SecretEntry, wait?: Duration}` as `GET_SECRET_OPTIONS`
(`crates/nvs-stdlib/src/cache.rs:301`), the card documents both options, and the helper's arity moved
3 → 5 for the bag's flattening. The body accepts the two slots and reads neither: a miss is a miss
for every caller alike, and the body doc at `crates/nvs-stdlib/src/cache.rs:2032` says so in one
paragraph the next slice deletes. Stages 1–5 are unchanged.

**Stage 6's config half is already landed, and is not an open item** — whatever the last handoff's
third item said. `[cache.process] fill_wait` has its field (`crates/nvs-config/src/tree.rs:1126`),
its `default.toml` entry shipping `5s`, its `cache.process` directive row (`System`/`Reload`), and
its boot refusal of `0`, `false` and a non-duration with `E0642` at
`crates/nvs-config/src/store.rs:133`, with tests. Only the **stdlib-side reader** is missing.

**A bounded wait is `nvs_host`'s, not `std`'s.** Neither `nvs-stdlib` nor `nvs-runtime` names a
`Condvar` anywhere; `nvs_host::timer::park_until` (`crates/nvs-host/src/timer.rs:251`) and
`group::park` (`crates/nvs-host/src/group.rs:162`) are what a bounded cooperative wait is built
from. A helper that blocked its core on a `std` primitive would hold a worker for up to `fill_wait`.

**The refresh-ahead fraction is the last fifth** of an entry's lifetime, fixed in
`docs/decisions/0181.md` and stated in `docs/spec/01-core-library.md:1190` — not a number to
re-derive.

**The `[context]` gap the last handoff named is now closed** for the rules half:
`[context.stage.6]` in `docs/agent/loop-goal.toml` (and the authored copy under
`docs/agent/goals/`) now names `concurrency/a-secret-fill-runs-once-per-process` and
`concurrency/put-and-get-are-the-whole-boundary`, so the session that writes the table is handed the
rule that specifies it. What is still unprinted is the goal's own current-stage prose —
`docs/agent/loop-goal.md:118-137` is stage 6's, and there is no `[context]` field that reaches it.

## Next group

**Stage 6: the fill table** — one file set: `crates/nvs-stdlib/src/cache.rs`, reading
`crates/nvs-host/src/timer.rs` and `crates/nvs-config/src/tree.rs`.

- [ ] **`[cache.process] fill_wait`'s reader**, so an omitted `wait` inherits it rather than removing
      the bound — `rule:http-server/no-spelling-for-an-unbounded-wait` is the shape one class over,
      `crates/nvs-stdlib/src/cache.rs:1149`'s `process_cap` is the reader to copy, and the field it
      reads is `crates/nvs-config/src/tree.rs:1126`. The shipped `5s` is this module's to state, as
      the cap is.
- [ ] **The fill table: exactly one caller in the process runs `fill` and every other waits for it**,
      bounded by its `wait` and throwing `TimeoutError` past it, a throwing `fill` releasing the
      waiters with nothing, and a `fill` asking for its own key a `LogicError` rather than a wait on
      itself — `rule:concurrency/a-secret-fill-runs-once-per-process`. The body is
      `crates/nvs-stdlib/src/cache.rs:2032`, the entry a fill answers is
      `crates/nvs-stdlib/src/cache.rs:668`'s `SECRET_ENTRY`, and the wait is built from
      `crates/nvs-host/src/timer.rs:251`.
- [ ] **Refresh ahead, and a request that ends releases the key** — an entry inside the last fifth of
      its lifetime is answered to every caller while one of them replaces it
      (`rule:concurrency/a-secret-fill-runs-once-per-process`), at
      `crates/nvs-stdlib/src/cache.rs:2032`. This is also where the acceptance check's five
      `nvs-stdlib` tests and its two `.nvst` cases land.

## Backlog

- Stage 7's `setSecret`/`getSecret` on `Core\Session`, sealed the same way — same goal file.
- Stage 8 flips this goal's four rules to `shipped` and re-renders the rulebook.
- The stage's own prose is unreachable from `[context]`; the stage header the driver already prints
  above the failing check is where it would belong.
- `Core\Cache\SecretEntry` answers nothing, so nothing reads one back — `getSecret` stays the only
  door (`docs/agent/loop-goal.md` § *Standing decisions*).
