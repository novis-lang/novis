What crosses the cache boundary is a copy in and a copy out. There is no read-modify-write across
it: no set-if-absent, no compare-and-set, no atomic increment, and no operation a program can reach
that observes an entry and writes it in the same step.

`getSecret`'s `fill` is the one step that does observe and write, and it leaves the sentence above
standing because the observation is not a program's to reach: the miss, the election of the one
caller that fetches and the write are one member's business
(`rule:concurrency/a-secret-fill-runs-once-per-process`). Nothing tells a caller whether it filled or
waited, no key's fill can be made to wait on another's, and there is no way to hold the fill without
asking for the value.

An entry is therefore a payload rather than a live graph, and a rewrite **replaces** the entry
instead of merging into it. Two requests that read the same key, change what they read and write it
back are two last-writer-wins races, not a coordination primitive, and nothing about the boundary
pretends otherwise.

That is why a lock, a counter or a limiter is never built on the cache. The mechanisms that need
those guarantees have their own homes over the shared tier, where the store's own atomicity is what
provides them — `Core\RateLimit::consume` for a limit, a lease for scheduled work, and the database
for anything else.
