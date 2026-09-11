---
milestone: M8
---
# Loop goal 49 — a value outlives a request in the serving process, and a secret does so only sealed

`Core\Cache::process()` joins `local()` and `shared()`: one store per serving process, coherent across
every core of it, held in memory only and gone when the process ends. Every tier gains a lifetime and a
`forget`. And a secret may now outlive the request that fetched it — **only sealed**: `putSecret` and
`getSecret` take a key ring and store XChaCha20-Poly1305 ciphertext bound to the app, the entry's name and
its expiry, so no `secret` value ever enters any tier, and `getSecret`'s `fill` fetches on a miss once per
process while every other core waits for it. This is where an OAuth token lives between requests — and a
user's own token lives in their session the same way, sealed, through `Core\Session`'s `setSecret` and
`getSecret`.

## Why here

After goal `http-client`, on the user's call — the third of the three goals the REST client was split
into. It needs nothing that goal builds; it needs `crate::crypto`'s crate-private seal seams
(`crates/nvs-stdlib/src/crypto.rs:103-113`), which goal `webcrypto` keeps as they are while it changes the
members above them, and `crate::keyring` (`crates/nvs-stdlib/src/keyring.rs:1-20`). Before goal `gap-zero`
for that goal's standing reason.

What it needs already built, all on disk: `Core\Cache`'s two members and the one store class every tier
answers (`crates/nvs-stdlib/src/cache.rs:147-250`); the local tier's per-core map, cap and eviction
(`cache.rs:385-420`); the byte payload an entry is (`cache.rs:11-36`); the shared tier's Redis client
(`crates/nvs-stdlib/src/cache/redis.rs`); the detached accounting bracket
(`rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`); and the per-core workers
`nvs serve` starts (`crates/nvs-cli/src/serve.rs`, whose count is tested at `:1519`); and `Core\Session`'s
rows over the byte carrier its record already crosses as (`crates/nvs-stdlib/src/session.rs:135-215`).

## Stage 0 — the catch-up

Sentences on disk this goal makes wrong. Each is corrected in the stage that makes it wrong. Re-grep before
editing: these are anchors, and files move.

- `crates/nvs-stdlib/src/cache.rs:1-9` — "as two members that hand back a store", and `local()` versus
  `shared()` as the whole choice. Stage 3.
- `crates/nvs-stdlib/src/cache.rs:102-107` — § *What is not here yet*: "A TTL and a `forget`". Stage 4.
- `docs/spec/01-core-library.md:1178` — `Core\Cache` as `local()`, `shared()`, `put` and `get`. Stage 2.
- Six rule fragments, each by the record's `changes.modifies` in stage 2:
  `core-api/two-cache-tiers` ("two members with two contracts"), `concurrency/cross-request-state-is-explicit`
  (`Core\Cache` as "two members"), `http-server/the-accept-fan-out-is-one-worker-per-core` ("What the cores
  share is compiled program text and nothing else"), `concurrency/put-and-get-are-the-whole-boundary`
  (`fill` is the one step that observes and writes), `concurrency/a-cached-value-is-copied-across-the-boundary`
  ("neither does a `secret`" — still true of `put`, and now pointing at the sealed door), and
  `concurrency/the-local-tier-cannot-hold-what-must-be-coherent` (the process tier is coherent within one
  process and so cannot hold what a fleet relies on either).

## Stage 1 — the floor

Goal `http-client`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the record

One new record, and no other number. Its body is § *Standing decisions* below, argued. It creates four
rules, all `designed` — `concurrency/the-process-tier-is-one-store-per-process`,
`concurrency/a-secret-is-cached-only-sealed`, `concurrency/a-secret-fill-runs-once-per-process` and
`http-server/a-session-holds-a-secret-only-sealed` — and modifies the six Stage 0 names. **It decides whether `core-api/two-cache-tiers` is amended in place or
superseded** by a rule whose id does not count tiers; either way the three members stay three members with
three contracts and no flag. The spec row at `:1178` moves with it.

## Stage 3 — the keystone: the process tier

`Core\Cache::process(): Core\Cache\Store`, answering the one store class with a third `tier`. Behind it,
**one map per process** that every core reads — created where `nvs serve` starts its workers, before the
first one accepts, and under `nvs run` for the length of the run. It is the first mutable state the cores
share, so the map is sharded behind locks and the record says how many; `get` takes a shard's read lock
and never a write, for the local tier's reason (`cache.rs:93-100`). An entry is the byte payload the local
tier already holds (`cache.rs:11-36`), copied in and out, so a unit swap resolves classes by name exactly
as it does there.

- **Keys are scoped per `[[app]]` and per configuration generation**, as `rule:security/db-pool-reset-is-a-boundary`
  scopes a pool: two apps on one server never read each other's entries, and a reload never serves an
  entry written under the configuration it replaced.
- **No grant**, for `rule:core-api/two-cache-tiers`' reason about the local tier (`two-cache-tiers.md:11-14`):
  nothing leaves the process. What is left to bound is footprint: `[cache.process] max_size`, `System`
  and `Reload` like `[cache.local] max_size` (`crates/nvs-config/src/directive.rs:158-162`), evicting the
  key written longest ago and never failing a `put`.
- **Every byte is on the detached balance**, under `nvs_runtime::budget::Detached`, held by the store
  around every path that allocates or frees what it holds.
- **A session backend refuses it** as it refuses the local tier: coherent within one process is not
  coherent across a fleet.

## Stage 4 — a lifetime and `forget`, on every tier

`put(string $key, mixed $value, {ttl?: Duration})` and `forget(string $key): void`, on the store class, so
all three tiers gain them at once. An entry past its `ttl` is absent: the local and process tiers check on
`get` and drop what they find expired; the shared tier writes `PX` and `DEL`
(`crates/nvs-stdlib/src/cache/redis.rs`). An omitted `ttl` keeps today's behaviour — the entry lives until
it is overwritten, forgotten or evicted.

## Stage 5 — a secret, sealed

Two members on the store class, and the only two through which a secret meets a cache:

- `putSecret(string $key, secret string $value, Duration $ttl, array<secret bytes> $keys): void` — `ttl`
  is required, because a secret never outlives a lifetime someone stated.
- `getSecret(string $key, array<secret bytes> $keys, {fill?, wait?}): ?secret string` — the value, or
  `null` for a miss.

**What is stored is ciphertext.** The plaintext is the expiry and the value; the ring's newest key seals it
through `crate::crypto`'s `seal_under` (`crates/nvs-stdlib/src/crypto.rs:103-113`), and the additional
data is a domain byte of the cache's own ‖ the app ‖ the entry's name — so a ciphertext moved to another
key or another app does not open, and an old one written back under a longer store lifetime is still past
its sealed expiry. The sealed bytes go into the tier by its ordinary path, as `bytes`: **no `secret`
crosses the boundary**, so `rule:security/secret-crosses-no-boundary` is unchanged, and a sealed entry is
as safe on the shared tier as on the process one.

- **A sealed entry that does not open is a miss** — under every key of the ring, or past its sealed expiry
  — and never an error: `fill` runs if it was given, and a rotated ring re-fetches rather than failing.
  A ring that is wrong in itself is `crate::keyring`'s `LogicError` (`keyring.rs:39-50`).
- **`get` answers `null` for a sealed entry**, and `put` still refuses a `secret` at the boundary: the
  sealed door is the only door.
- **The registry gains `secret string`.** It spells `secret bytes`, `secret tainted string` and their
  parameter forms (`crates/nvs-stdlib/src/registry.rs:312-377`), and no plain `secret string`, which both
  members need — a return that is a promise, a parameter that demands what assignment already grants.

## Stage 6 — `fill`, once per process

`fill: callable(): Core\Cache\SecretEntry`, where `SecretEntry::of(secret string $value, Duration $ttl)`
carries what the fetch learned — a token endpoint answers its own lifetime. On a miss, **exactly one
caller in the process runs `fill`**, in its own request with its own capabilities; every other caller on
every core waits for that one, each for at most its `wait` — a `Duration`, inheriting
`[cache.process] fill_wait` — and past it throws `TimeoutError`.

- **A failure is not shared.** When `fill` throws, the waiters are released with nothing and the next one
  to ask runs `fill` itself; an exception crossing from one request into another would be exactly the
  cross-request state `rule:security/no-cross-request-state` closes.
- **Refresh ahead.** An entry in the last part of its lifetime — the record fixes the fraction — is still
  answered to every caller while exactly one of them runs `fill` to replace it.
- **A `fill` that asks for its own key** is a `LogicError`, not a wait on itself.
- A request that ends while it holds the fill releases it, so a cancelled fetch never wedges a key.

This is the one step in `Core\Cache` that observes an entry and writes it, and the record amends
`rule:concurrency/put-and-get-are-the-whole-boundary` for `getSecret`'s `fill` alone. On the shared tier
it is once per process, not once per fleet: a fleet-wide fill would be a lease, and that is not this goal.

## Stage 7 — a user's secret in the session

`$session->setSecret(string $key, secret string $value, array<secret bytes> $keys): void` and
`$session->getSecret(string $key, array<secret bytes> $keys): ?secret string`, in
`crates/nvs-stdlib/src/session.rs:135-215`, over stage 5's sealing — one construction, and the session
door's own domain byte, so a value sealed for the cache never opens as a session value. This is where a
web app keeps the access and refresh token of the user it acts for.

- **No `ttl`.** A session value lives as long as its session
  (`rule:http-server/session-expiry-belongs-to-the-store`), and the sealed plaintext carries no expiry of
  its own.
- **The additional data is the domain byte ‖ the app ‖ the key, and not the session id**, because
  `regenerate` issues a new id over the same record, and a value bound to the old id would stop opening.
  Moving a ciphertext between two sessions takes write access to the store, which already means owning
  every session in it.
- **`get` answers `null` for a sealed value and `set` still refuses a secret**, as the cache's pair does.
  A sealed value that does not open under the ring is absent, never an error.
- The record crosses the store as the byte carrier it already does
  (`docs/rules/http-server/a-session-store-answers-four-operations.md:15-16`), so no `secret` reaches the
  store and `rule:security/secret-crosses-no-boundary` is unchanged here too.

## Stage 8 — the rulebook

Flip stage 2's four rules to `shipped`, with `guardedBy` filled from this goal's cases and tests, and
`python tools/rules.py --render`.

## Standing decisions

- **Settled with the user, not to re-decide:** the process tier is a general third tier, not a
  secrets-only store; a secret meets a cache only through `putSecret` and `getSecret`, both taking a key
  ring, and is stored only sealed; `fill` is single-flight per process; and a user's secret in the session
  is sealed the same way, through `setSecret` and `getSecret`. The record argues these, and does
  not re-open them.
- **What sealing buys, stated exactly.** It keeps a secret from being read by code that knows an entry's
  name but not the ring, it lets a secret reach the shared tier without leaving the process in the clear,
  and it makes a tampered, moved or replayed entry a miss. **It does not protect a secret from a
  compromised process** or a memory dump: the ring lives in the same memory. The record and
  `putSecret`'s reference card both say so, so nobody reads "encrypted" as more than it is.
- **Cost per call**: one XChaCha20-Poly1305 seal per `putSecret` and one open per ring key tried per
  `getSecret`, plus the tier's own copy.
- **What it spends**: the process tier holds at most `[cache.process] max_size` bytes, once per process —
  O(working set), where the local tier is O(cores × working set) — on the detached balance, never
  O(requests served). The fill table holds one slot per key being filled at that moment, so
  O(concurrent fills). A sealed entry costs its ciphertext: the value plus a nonce, a tag and eight octets
  of expiry.
- **ADR slots**: the one record of stage 2.
- **Not this goal**: a fleet-wide single fill, which is a lease over the shared tier; a `secret bytes`
  value; a user's refresh token in a cache tier, which must survive eviction and so goes to the session; and
  the REST package, which is the first caller of all of this. A session that finds one on its path writes
  it to the handoff's `## Backlog`.
