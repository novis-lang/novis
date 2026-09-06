Each core holds its own metrics registry, and a scrape merges them. That is per-core mutable state
outliving a request, which is the shape `rule:concurrency/cross-request-state-is-explicit` exists
to constrain — and it passes that rule's own test rather than being an exception to it:

- **Nothing reads it to make a decision.** No Novis program can read a metric at all; the only
  reader is a scrape or a push, outside any request. A program that would be incorrect if a read
  returned nothing is using the wrong tier, and no program can read this one.
- **The values are approximate aggregates by design.** Per-core counters merged at scrape are the
  correct implementation, not a compromise — the same reason a metric is not a rate limit
  (`rule:core-classes/ratelimit-two-members`) — and the merge is arithmetic, never coordination.
- **No request-derived value crosses**, because `rule:security/metric-label-refuses-tainted` forbids
  exactly that. What accumulates is a fixed set of series with bounded label sets.
- **Memory is charged to the core and capped** — O(cores × series), bounded by
  `rule:observability/past-max-series-a-new-series-is-refused`, never O(requests served) — the same
  accounting `rule:concurrency/cache-memory-is-charged-to-the-core` records.

A shared, coherent store would be the cross-request coordination that rule closes, bought for a
number that is approximate by definition.
