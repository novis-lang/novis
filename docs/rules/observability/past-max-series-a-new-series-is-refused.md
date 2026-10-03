`[metrics] max_series` bounds the number of distinct label combinations one **core** holds. Past
it, a series that does not exist yet is **refused** — the call is a no-op, the refusal is counted
against the metric's name, and one warning per window is written to `Core\Log` naming the metric.
A series that already exists is untouched.

An existing series is **never evicted**, and this is a deliberate correction to the obvious
design. Evicting a counter and later recreating it makes its value appear to reset, which every
backend reads as a process restart and which silently corrupts every `rate()` and `increase()`
over it — a wrong number on a dashboard, which is worse than a missing one. Refusing the new loses
the newest labels and keeps every existing series exactly correct; the assertion that distinguishes
the two designs is that every value is unchanged across the boundary.

What it spends: O(cores × series), bounded per core, a counter costing its key and eight bytes and
a histogram its bucket array. Charged to the core, not to a request, exactly as
`rule:concurrency/cache-memory-is-charged-to-the-core` charges the cache, and never O(requests
served). The ten of `rule:observability/default-series` are seeded ahead of the bound.
