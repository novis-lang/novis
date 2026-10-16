`[metrics] max_series` bounds the number of distinct label combinations one **core** holds. Past
it, a series that does not exist yet is **refused** — the call is a no-op, the refusal is counted
against the metric's name, and one warning per window is written to `Core\Log` naming the metric.
A series that already exists is untouched.

An existing series is **never evicted**. Refusing the new loses the newest labels, often the ones
being debugged, and one warning per window is easy to miss; it still keeps every existing series
exactly correct, which is what a dashboard is for. The assertion that distinguishes this from
eviction is that every value is unchanged across the boundary. The ten of
`rule:observability/default-series` are seeded ahead of the bound.

- **Memory:** O(cores × series), bounded by `max_series` per core, a counter costing its key and
  eight bytes and a histogram its bucket array. Charged to the core, not to a request, and never
  O(requests served). Zero series until something registers one.

**Declined:**
- **Evicting the least-recently-used series past the cap.** A recreated counter appears to reset,
  which every backend reads as a process restart and which silently corrupts every `rate()` and
  `increase()` over it. A wrong number on a dashboard is worse than a missing one.
