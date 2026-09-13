# Handoff

## State

**Goal `process-cache` — stage 5 is on disk and green.** `Core\Cache\Store::putSecret` and
`getSecret` are the only door a secret meets a cache through, with the three cargo tests and the four
`.nvst` cases the acceptance check names. Stages 1–4 are unchanged, as is goal `http-client`'s list.
Stage 6's `{fill?, wait?}` bag is deliberately absent from `getSecret`'s row.

**What a sealed entry is**, so the next session does not re-derive it — `crates/nvs-stdlib/src/cache.rs`'s
`bound`, `sealed_plaintext` and `SEALED_SPACE` own it in full. The AEAD's additional data is a domain
octet ‖ the application ‖ the entry's name; the plaintext is an eight-octet big-endian expiry in
milliseconds ‖ the value; the stored key is `SEALED_SPACE` ‖ the program's key, which is what makes a
plain `get` on the same name an ordinary miss rather than a decode failure.

**`crate::crypto::seal_under` and `open_under` now take `bound: &[u8]`** — the AEAD's additional data —
and every other caller passes `&[]`, which is the empty-associated-data construction and therefore the
same octets they produced before. The goal's own stage-5 prose settled that question (additional data,
not a plaintext prefix) where the last handoff had it open.

**`put` did not refuse a `secret` before this session**, whatever its row comment said: `CoreTy::Mixed`
refuses `tainted` only. `crates/nvs-types/src/expr/quals.rs`'s `reject_secret_cached_argument` is now the
graph copy's fourth carrier and the one that names a member taking the secret rather than a reveal; it is
wired in from the *instance*-call arm of `expr/calls.rs`. The playbook bullet is the trap.

**The `[context]` gap this session paid for**: the pack prints the goal's `## Standing decisions` but not
the current stage's own prose, and `docs/agent/loop-goal.md:92-117` is what settled the additional-data
question above. There is no `[context]` field for it; the stage header the driver already prints above
the failing check is where it would belong.

## Next group

**Stage 6: `fill`, once per process** — one file set: `crates/nvs-stdlib/src/cache.rs`,
`crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/default.toml`.

- [ ] **`getSecret`'s trailing `{fill?: callable(): Core\Cache\SecretEntry, wait?: Duration}` bag**, as
      the row and the card alone — the row at `crates/nvs-stdlib/src/cache.rs:367`, the card at
      `crates/nvs-stdlib/src/cache.rs:574`, and `PUT_OPTIONS` at `crates/nvs-stdlib/src/cache.rs:279`
      the shape to copy. `rule:core-api/shape-rules` R2 is the trailing-shape rule, and
      `crates/nvs-stdlib/src/cache.rs:622`'s `SECRET_ENTRY` slots are what a fill's answer is read out of.
- [ ] **The fill table: exactly one caller in the process runs `fill`, every other waits for it** —
      beside `PROCESS` at `crates/nvs-stdlib/src/cache.rs:1050`, with the miss path at
      `crates/nvs-stdlib/src/cache.rs:1980`. A `fill` that throws releases the waiters with nothing, and
      a request that ends holding one releases it; `rule:security/no-cross-request-state` is why an
      exception may not cross.
- [ ] **`[cache.process] fill_wait`, the directive `wait` inherits** —
      `crates/nvs-config/src/tree.rs:1098`'s block beside `max_size`, the roster row at
      `crates/nvs-config/src/directive.rs:180`, and the commentary at
      `crates/nvs-config/src/default.toml:523`. Past the wait is a `TimeoutError` and never an unbounded
      one (`rule:http-server/no-spelling-for-an-unbounded-wait`).

## Backlog

- Refresh-ahead: an entry in the last part of its lifetime is still answered while one caller refills —
  `docs/agent/goals/49-process-cache.md` stage 6 fixes the fraction.
- Stage 7's `setSecret`/`getSecret` on `Core\Session`, sealed the same way — same goal file.
- A fleet-wide single fill is a lease over the shared tier and is not this goal — same goal file.
- `Core\Cache\Store::forget` does not reach a sealed entry; nothing yet says whether it should —
  `crates/nvs-stdlib/src/cache.rs`'s `SEALED_SPACE`.
