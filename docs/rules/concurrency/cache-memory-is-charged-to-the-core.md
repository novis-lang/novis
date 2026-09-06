Cache memory is **not attributable to a request**. It is charged to the core that holds it and
capped by an `nvs.toml` directive, and that cap is the deliberate, bounded exception to the rule that
every byte belongs to a request in flight.

**Exceeding the cap evicts rather than failing an allocation.** A write that crosses the ceiling
forgets an older entry and succeeds; a program never sees a store fail because the tier was full,
which is the whole point of a tier whose contract already says any entry may be absent.

Stated in the form the memory rule requires: the local tier costs **O(cores × working set)** — eight
cores hold up to eight copies of the same hot entry — bounded by the configured cap. It is
explicitly **not** O(requests served): an entry's lifetime is governed by TTL and eviction, never by
how much traffic has passed through. That multiplication is the price of the isolation it buys, and
it is recorded so it is a known number rather than a surprise in production.
