# Handoff

## State

**Goal `process-cache` — stage 5's first member is on disk and green: `Core\Cache\SecretEntry`, with
its one static `of(secret string $value, Duration $ttl)`.** Nothing seals or opens anything yet:
`putSecret` and `getSecret` are unwritten, so an entry is a value a program can build and nothing can
consume. Stage 4 is unchanged and still green, as are goal `http-client`'s list and this goal's stage
1 floor.

The entry is a **handle** — two slots, `value` and `nanos`, no instance member, and the sentence
`registry.rs`'s `HANDLES` list wants beside it. The lifetime is held as the nanosecond count
`Lifetime` reads rather than as the `Duration` instance, because that is the number every sealing
path turns into a deadline; `crates/nvs-stdlib/src/cache.rs:495` is the class doc that owns both.

**`putSecret` and `getSecret` cannot land in separate sessions.** The conformance floor is per member
and counts cases that *call* one, and a `putSecret` with no reader has nothing a `.nvst` case can
observe — so the next group is the two members together, not one each.

**`crate::crypto` has no associated-data door.** `seal_under`
(`crates/nvs-stdlib/src/crypto.rs:1465`) and `open_under` (`:1522`) take a key, a message and a
member name and nothing else, so binding an entry to the app, its name and its expiry is either a new
AAD-carrying variant beside them or those three fields written into the plaintext ahead of the value
and checked after the open. The second needs no change to `crypto.rs` and the tag covers it either
way. Not decided.

The `[context]` gap this session paid for is closed in the goal's own toml: the base `playbook` now
names `'Tooling > Registering a'`, which every stage left in this goal needs and none of them had.

## Next group

**Stage 5: the sealed secret, as the two members that are its only door** — one file set:
`crates/nvs-stdlib/src/cache.rs`, `crates/nvs-stdlib/src/crypto.rs`, `crates/nvs-stdlib/src/keyring.rs`.

- [ ] **`putSecret(string $key, secret string $value, Duration $ttl, array<secret bytes> $keys): void`
      and `getSecret(string $key, array<secret bytes> $keys): ?secret string`, as one slice** — two
      rows beside `forget` at `crates/nvs-stdlib/src/cache.rs:330`, two helpers after
      `nvs_core_cache_forget` at `crates/nvs-stdlib/src/cache.rs:1611`. Seal under the ring's newest
      key and open against every key of it, `crates/nvs-stdlib/src/keyring.rs:86` being `borrow`,
      `:136` `newest` and `:117` `entries`; the nonce is drawn through `crate::random::draw` as
      `crates/nvs-stdlib/src/crypto.rs:3019` draws one. The sealed payload goes to whichever tier the
      store names, through the same three arms `put` takes. `rule:security/secret-qualifier` is why
      the value parameter is a demand and not an admission, and the spec's `Core\Cache` row is the
      signature. Leave `getSecret`'s `{fill?: …, wait?: …}` bag out — it is stage 6.
- [ ] **The three cargo tests the acceptance check names**, in `cache.rs`'s own `#[cfg(test)]` module
      at `crates/nvs-stdlib/src/cache.rs:1665`:
      `sealed_entry_moved_to_another_key_or_app_is_a_miss`,
      `sealed_entry_past_its_sealed_expiry_is_a_miss_whatever_the_store_says`,
      `every_sealed_entry_nonce_is_drawn_through_core_random`. The first two are what decides the
      binding question in `## State`; the third reads `crate::random`, not the OS.
- [ ] **The four `.nvst` cases the check names**, three under `tests/conformance/core/` and
      `cache-put-refuses-a-secret-and-names-put-secret.nvst` under `tests/conformance/reject/`. The
      reject one is over `put`'s existing `CoreTy::Mixed` value parameter at
      `crates/nvs-stdlib/src/cache.rs:311` and needs no new code — its diagnostic has to name
      `putSecret` as what to write instead.

## Backlog

- Stage 6's single-flight `fill` and its `wait`, which is what `SecretEntry` exists for — `docs/agent/goals/49-process-cache.md`.
- Stage 7's `setSecret`/`getSecret` on the session — same goal file.
- A fleet-wide single fill, a `secret bytes` value and the `nvs/rest` package are all out of this goal — `docs/agent/loop-goal.md` § *Standing decisions*.
