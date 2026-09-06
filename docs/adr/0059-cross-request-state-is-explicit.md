# ADR 0059 — Cross-request state is explicit: `Core\Cache` is per-core, copied in and out, and capped

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** what replaces APCu and the SysV shared-memory family; the per-core cache's coherence model,
  value-crossing rule, memory accounting and cap. Not in scope: the `Core\Cache` method roster, or the
  Redis backend's own configuration, both of which M8 designs.
- **Amends:** [0004](0004-memory-for-simplicity.md) — § 3 records a second, deliberate exception to
  "memory is attributable to a request": cache memory is charged to a **core**, with its own cap.
- **Amended by:** 0075, 0083, 0084, 0112, 0142

> **In short:** APCu's cross-process shared segment is closed by [ADR 0052](0052-closed-doors.md) § 3, and
> what replaces it is a **per-core in-process cache** — one copy per core, no coherence between them. That
> spends memory, and this ADR says how much, in the form [ADR 0004](0004-memory-for-simplicity.md)
> requires: **O(cores × working set)**, not O(cores × requests served). Values are **copied** across the
> cache boundary using the same graph-copy operation as `serialize` and the isolate boundary, because a
> cached value must not live in a request heap that is dropped wholesale. The local tier is a **cache, not
> a store**: it must always be correct to find nothing there. Anything needing coherence — sessions, locks,
> rate limits, counters — uses the shared tier, which is a real store over the network.

## Context

- Five PHP mechanisms — `shmop`, `sysvshm`, `sysvsem`, `sysvmsg` and APCu — are one capability: a channel
  through which one request observes another. [ADR 0052](0052-closed-doors.md) § 3 closes it as
  incompatible with strict shared-nothing requests, which is priority 1. That leaves a real need
  unaddressed, and this ADR addresses it rather than pretending the need was illusory.
- The need is not one thing. "Cache a parsed config for 60 seconds" and "hold a distributed lock" look
  similar in APCu and are completely different problems; the first tolerates loss and staleness, the second
  does not. PHP's single API for both is why applications routinely use APCu for things that break silently
  under multiple processes.
- AGENTS.md's memory rule — attributable to a request, under an enforceable cap, O(in-flight) rather than
  O(requests served) — does not fit a cache, whose whole purpose is to outlive the request that filled it.
  The rule needs an explicit, bounded exception rather than a quiet one.

## Decision

### 1. Two tiers, named differently on purpose

- **`Core\Cache::local()`** — per-core, in process. One instance per core, no coherence between cores. Its
  contract states that any entry may be absent at any time, for any reason, and that a write on one core is
  not visible on another. A program that would be incorrect if a `get` returned nothing is using the wrong
  tier. **No capability gates this tier**, which closes the one hole
  [0112 § 8](0112-authority-is-keyed-on-the-enclosing-namespace.md)'s roster left open:
  [0118](0118-a-capability-is-checked-at-the-door-to-the-effect.md) § 1 checks a grant at the door to an
  *effect*, and this tier has no door — nothing leaves the process, no name is resolved and no file is
  opened. What is left to bound is footprint, and § 3's `nvs.toml` cap is the instrument for a bound; a
  boolean grant is not one, and adding it would price the tier as an authority question that a
  deployment would then have to answer for every application that caches anything.
- **`Core\Cache::shared()`** — a real store over the network (Redis by default), coherent across cores and
  across machines, gated by `cache.shared`
  ([ADR 0142](0142-a-configured-store-is-authorized-by-its-configuring.md) § 1): the endpoint is one an
  operator wrote into root-owned configuration, so the grant names the store rather than a host and
  [ADR 0058](0058-outbound-request-policy.md) § 3's address policy is not asked of it. That store may be a
  Unix socket, which is the one transport with no address for a policy to read.

The two are separate methods rather than one API with a flag, so the choice is made in the source and is
visible in review. This is the same reasoning [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 3
gives for refusing a generic `sanitize()`: one name covering two different guarantees invites using the
weaker one by accident.

### 2. Values are copied across the boundary

`put` copies out of the request heap into the cache; `get` copies into the request heap. The operation is
the recursive graph copy [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) already defines and
already shares with the isolate boundary — not a third mechanism.

This is forced, not chosen. A request's heap is dropped wholesale at the end of the request, so a value the
cache holds cannot live there; and a value the cache holds cannot be handed into a request heap by
reference either, or the wholesale drop would free it. The same rule means a cached value is subject to the
same restrictions as any crossing value: a generator does not go in
(`rule:iteration/generator-stays-in-one-isolate`), and neither does a `secret`
([ADR 0033](0033-secret-qualifier-for-confidential-values.md)).

The copy is a real per-`get` cost (priority 3) paid to keep the request model intact (priority 1). An
implementation may later avoid copying immutable scalars by sharing a refcount within a core, since a core
is single-threaded — an optimisation, not a semantic change, and it must not be visible.

### 3. Memory: charged to the core, capped, and stated

Cache memory is **not attributable to a request**. It is charged to the core that holds it and capped by an
`nvs.toml` directive under [ADR 0005](0005-config-changeability.md)'s ordinary rules; exceeding the cap
evicts rather than failing an allocation.

Stated in the form [ADR 0004](0004-memory-for-simplicity.md) requires: the local tier costs
**O(cores × working set)** — eight cores hold up to eight copies of the same hot entry — and is bounded by
the configured cap. It is explicitly **not** O(requests served); an entry's lifetime is governed by TTL and
eviction, never by how much traffic has passed through.

That multiplication is the price of the isolation ADR 0052 § 3 buys, and it is exactly the trade ADR 0004
mandates: footprint is the last thing protected and is spent deliberately to buy priority 1. It is recorded
here so it is a known number rather than a surprise in production.

### 4. What may not use the local tier

`Core\Session` ([ADR 0012](0012-no-superglobals.md)) may not be backed by the local tier. A session read on
one core and written on another must see one value, and a per-core cache cannot provide that. The same
applies to locks, rate limits, idempotency keys and any counter whose value is relied upon. These use the
shared tier or the database.

This is enforced rather than documented: `Core\Session`'s configurable backends do not include the local
tier as an option. Two of the things named above now have their own homes over the shared tier rather than
being left to the application: **rate limits** are [ADR 0075](0075-core-ratelimit.md)'s
`Core\RateLimit::consume`, and the **fleet lease** a scheduled job takes is
[ADR 0073](0073-scheduled-work-is-config.md) § 3's.

The test this section states is a test of *what a program relies on*, not of where bytes live, and one
thing that looks like a violation is not one: a per-core **metrics** registry
([ADR 0076](0076-observability-export.md) § 5) is mutable state outliving a request, and it passes, because
no program ever reads it to make a decision and its values are approximate aggregates merged at scrape.

## Consequences

- **APCu-shaped code does not port mechanically.** An application using APCu as a coordination mechanism
  needs its caching layer reconsidered, not rewritten line by line. `nvs convert` (M11) emits a diagnostic
  naming both tiers rather than guessing which was meant — guessing wrong in the "it was actually a lock"
  direction produces a silent correctness bug, which is the one outcome a converter must never risk.
- **Cache hit rates are lower than a single shared segment's**, by roughly the core count for
  uniformly-distributed traffic on a cold cache. Warm caches converge. This is a real throughput cost,
  paid once per core rather than per request.
- **There is no in-process coordination primitive at all.** A program needing one pays a network round trip.
  That floor is deliberate: an in-process one would be the channel ADR 0052 § 3 closes.
- **The local tier is genuinely fast** — no serialization to a wire format, no socket, no syscall; a
  graph copy within one core's heap. For the parsed-config and compiled-template cases that dominate APCu's
  real usage, it is faster than APCu was.

## Alternatives rejected

- **A shared segment across cores, with locking.** Restores APCu's hit rate and coherence. Rejected by
  [ADR 0052](0052-closed-doors.md) § 3: it is the cross-request channel shared-nothing exists to remove,
  and locking makes one request's failure able to stall or corrupt another's view.
- **A single API with a `coherent: true` flag** instead of two methods. Terser and easier to switch between
  tiers. Rejected: the default would be chosen once and copied thereafter, and the failure mode of choosing
  wrong is silent. Two names cost four characters and make the guarantee visible.
- **Reference-sharing instead of copying**, with the cache pinning values against the wholesale heap drop.
  Removes the per-`get` copy. Rejected: it makes the request heap's lifetime conditional on what a request
  happened to read, which is precisely the property the wholesale drop exists to guarantee, and it would
  leak a mutable value between requests unless every cached value were deeply immutable — a second value
  model to maintain.
- **No local tier; everything goes to the shared store.** One coherence model, nothing to explain.
  Rejected on priority 3: it puts a network round trip in front of the parsed-config and template cases
  that need no coherence at all, which is most of what caching is used for.

## Verification

- **M8:** a value written on one core and read on another via the local tier is absent, asserted rather
  than left implicit, so the coherence contract is tested; the same value via the shared tier is present.
- **M8:** a `secret` value and a generator are each refused at `put`, reusing the ADR 0023/0033 fixtures.
- **M8:** filling the local tier past its configured cap evicts and does not fail an allocation, and the
  memory it holds is reported against the core rather than against any in-flight request — checked against
  the same accounting the request memory cap uses.
- **M8:** configuring `Core\Session` onto the local tier is a configuration-time error naming § 4.
- **M11:** an APCu call in `nvs convert`'s corpus produces a diagnostic naming both tiers, never a
  translation.
