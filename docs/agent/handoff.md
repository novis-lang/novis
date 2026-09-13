# Handoff

## State

**Goal `process-cache` — stage 4 is on disk and green: an entry has a lifetime, and a name can be
forgotten, on every tier.** `put` takes `rule:core-api/shape-rules` R2's trailing `{ttl?: Duration}`,
the two in-process tiers hold the deadline beside the payload, and `forget(string $key): void` is the
store's third member. Stage 5 (the sealed secret), stage 6 (the single-flight `fill`) and stage 7 (the
session's own secret) are unwritten; goal `http-client`'s list, this goal's stage 1 floor, still passes.

`Lifetime` at `crates/nvs-stdlib/src/cache.rs:519` is what the call *wrote* — `Forever`, or
`For(Duration)` — and each tier turns it into what it can hold: the in-process tiers into an `Instant`
deadline beside the payload, the shared tier into `SET … PX`. The module doc is the home for the three
decisions behind that: why the deadline is monotonic and not wall-clock, why the read judges an expired
entry without taking it out (`get` stays a read lock), and why a lifetime already over is not a write at
all but takes the exit an oversized arrival takes. `Connection::set_expiring` now takes a `Duration` and
sends `PX` rather than `EX`, which is why `crate::session` and its fake store moved with it.

The `[context]` gap this session paid for is closed in the goal's own toml rather than only described:
`[context.stage.4]` and `[context.stage.5]` now name `core-api/shape-rules` and `core-api/reference-card`,
which every stage adding a `Core` member needs and neither had.

## Next group

**Stage 5: the sealed secret, as the two members and the class a `fill` answers** — one file set:
`crates/nvs-stdlib/src/cache.rs`, `crates/nvs-stdlib/src/crypto.rs`, `crates/nvs-stdlib/src/keyring.rs`.

- [ ] **`Core\Cache\SecretEntry`, with its one static `of(secret string $value, Duration $ttl)`** — a
      second `CoreClass` beside `STORE` at `crates/nvs-stdlib/src/cache.rs:290`, registered where
      `STORE` is. It lands first because `getSecret`'s `{fill?: callable(): Cache\SecretEntry}` has no
      return type to declare until it exists. `rule:core-api/shape-rules` R16 is the singular noun and
      R14 why a thing with a lifetime is an object; the spec's `Core\Cache` row is the signature.
- [ ] **`putSecret(string $key, secret string $value, Duration $ttl, array<secret bytes> $keys): void`**
      — the row after `forget` at `crates/nvs-stdlib/src/cache.rs:290`, sealing with
      `crates/nvs-stdlib/src/crypto.rs:1432`'s `cipher` under the ring's newest key
      (`crates/nvs-stdlib/src/keyring.rs:136`). The associated data binds the `[[app]]`, the entry's own
      name and the sealed expiry, so a moved or replayed entry is a miss;
      `crates/nvs-stdlib/src/signature.rs:46` is the domain-byte precedent to follow rather than invent.
      `rule:security/secret-crosses-no-boundary` is why the ciphertext is what the tier holds.
- [ ] **`getSecret(string $key, array<secret bytes> $keys): ?secret string`, without the `fill` half** —
      opens under each key of the ring in turn (`crates/nvs-stdlib/src/keyring.rs:117`), and a sealed
      entry that opens under none of them, or whose sealed expiry has passed, is a **miss** and never a
      throw. The `{fill?: …, wait?: …}` options bag and its single-flight table are stage 6, and adding
      the bag empty here would be a shape no case can ask about.

## Backlog

- A fleet-wide single fill — a lease over the shared tier — is named *not this goal* by the goal's
  § *Standing decisions*, which is its home.
- A `secret bytes` value, and a user's refresh token in a cache tier, likewise: the goal's
  § *Standing decisions*.
- The `nvs/rest` package is the first caller of all of this and is unscheduled —
  [carried-gaps.md](carried-gaps.md).
- An expired in-process entry holds its bytes against the cap until the eviction order or an overwrite
  reaches it; the cache module doc states it as the cost of a read-only `get`.
