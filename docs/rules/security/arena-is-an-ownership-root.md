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

Entering an isolate is a context construction and a pooled stack, with no mapping call in it, so
spawn-to-result for a trivial child stays in microseconds. A region would put a map and an unmap on
every isolate, which for a trivial child is most of its lifetime. Because both sides allocate from the
same place, a move at refcount 1 across the boundary can hand the allocation over without copying it
(`rule:security/isolate-values-cross-by-copy`).

With no address range to fall back on, the graph copy is the only way state can bleed between
isolates. That concentrates the audit in one file with one test suite, rather than spreading an
invariant across every allocation site, and it is why `rule:classes/graph-copy`'s one operation with
two carriers carries weight here.

**Declined:**
- **A bump-pointer region per isolate, reclaimed with one call.** It skips native teardown, its
  footprint tracks work done rather than live data, and it competes for a latency win the pooled
  allocator has already taken.
- **A separate allocator instance per isolate.** It costs a pointer in the context and a load on every
  allocation to buy accounting that arithmetic at the allocation site already gives
  (`rule:security/isolate-budget-is-the-trees`), and it fragments the per-thread cache that made the
  allocator worth having.
- **An arena pointer in the context.** Compiled code loads the context's fields at fixed offsets, so a
  new field is an ABI change, and once the arena is an ownership root there is nothing for it to point
  at.
- **Segregating isolates by address range and checking pointers at the boundary.** A check cheap enough
  for the request path can only ask which region an address is in. It cannot see that a property is
  `secret` or that a class is unresolvable on the far side, and the walk answers all three in one pass.
- **An OS process per isolate.** It is the one design that gives an unreachable address range for free,
  and it pays the cost and the ambient authority `rule:security/isolate-shares-nothing` declines.
