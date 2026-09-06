An isolate's arena is its context together with the set of values reachable from that context's roots.
Nothing about being inside an isolate changes where a byte comes from: allocation goes through the
process allocator exactly as it does on the parent's own frames. Isolation is therefore a property of
**reachability**, enforced at the one place a value can move between two contexts
(`rule:security/isolate-values-cross-by-copy`) — a compiled function reaches heap state through its
arguments and its context's statics word, and nothing else.

A region arena is the wrong implementation, not merely a heavier one. Freeing a region runs no native
drop, so an isolate holding a transaction would leave it open; values resize, and a bump region cannot
reuse what it freed, so a loop appending to a string would make the footprint O(work done) rather than
O(live), which `rule:programs/memory-priority` calls a leak; and the latency a region is reached for
is already collected by the pooled allocator.
