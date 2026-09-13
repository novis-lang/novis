# Handoff

## State

**Goal `process-cache` — a value outlives a request in the serving process, and a secret does so only sealed — has just started; nothing of it has landed yet.** Goal `http-client`'s whole list is this goal's Stage 1 floor.

**The design is settled with the user and is written into the goal's § *Standing decisions*; the record
that states it is not written yet.** Stage 2 writes it — one new record, the goal names no number — and it
transcribes those decisions rather than re-deriving them. The things not to re-decide: `process()` is a
general third tier beside `local()` and `shared()`; a secret meets a cache only through `putSecret` and
`getSecret`, both taking a key ring, and is stored as ciphertext bound to the app, the name and the
expiry; a sealed entry that does not open is a miss; `fill` runs once per process and a failure is never
shared. A user's secret in `Core\Session` is sealed the same way, through `setSecret` and `getSecret`.

## Next group

**Stage 2: the record** — one file set: `docs/decisions/`, `docs/rules/concurrency*`,
`docs/rules/core-api*`, `docs/rules/http-server*`, `docs/spec/`.

- [ ] **The record** — the next free number in `docs/decisions/`, `changes.creates` the four rules the
      goal's stage 2 names, `changes.modifies` the six Stage 0 lists. It says whether
      `core-api/two-cache-tiers` is amended in place or superseded, how many shards the process map has,
      the refresh-ahead fraction, and the shipped `[cache.process] max_size` and `fill_wait`.
- [ ] **The four rule fragments and their JSON entries**, all `designed`, and the six modified ones,
      then `python tools/rules.py --render`.
- [ ] **The spec row** — `docs/spec/01-core-library.md:1178` (`Core\Cache`) takes `process()`, `ttl`,
      `forget`, `putSecret` and `getSecret`, and `Core\Session`'s row takes `setSecret` and `getSecret`.

## Backlog

- Stage 3 — the process tier. `crates/nvs-stdlib/src/cache.rs`, `crates/nvs-config/src/directive.rs`,
  and `crates/nvs-cli/src/serve.rs` where the map is made. The keystone. Its own session.
- Stage 4 — `ttl` and `forget` on every tier. `cache.rs`, `cache/redis.rs`. Shares `cache.rs` with
  stage 3.
- Stage 5 — `putSecret`, `getSecret`, and `secret string` in the registry. `cache.rs`, `crypto.rs`'s
  seams, `keyring.rs`, `registry.rs`. Its own session.
- Stage 6 — `fill`. `cache.rs` and the process-wide fill table. Shares `cache.rs` with stage 5.
- Stage 7 — a user's secret in the session. `crates/nvs-stdlib/src/session.rs`, over stage 5's sealing;
  its cases need Redis, which the floor already starts.
- Stage 8 — the flips. `docs/rules/` only.
- When this goal's last check goes green the driver takes goal `outbound-proxy`.
