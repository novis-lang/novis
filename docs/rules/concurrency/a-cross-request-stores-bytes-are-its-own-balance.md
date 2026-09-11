A request is measured against its own allocations, so the bytes of a store that outlives requests
move a balance of their own and no request is charged or credited for them. A request's memory
reading is a thread balance taken against where it started, and a store that one request fills and a
later one empties would otherwise drive that balance down and hand the later request a ceiling of
`[limits] memory` plus whatever it evicted.

The mechanism is an accounting bracket — `nvs_runtime::budget::Detached`, a guard whose lifetime is
the bracket. An allocation or a release made while one is held moves the process's detached balance
and leaves the live balance, every request's reading and every armed ceiling where it found them. A
pre-check inside a bracket refuses nothing, because bytes the process owns are not the request's to
be refused; the allocation counters keep moving, because an allocation made on the process's behalf
is still one this thread made.

**The bracket is held by the store, around every path that allocates or frees what it holds, and
never by a call site that happens to be storing.** What a bracket owes is symmetry: a block allocated
on one balance and freed on the other corrupts both. A store therefore copies what it is handed
rather than keeping the caller's allocation, so that the bytes it holds are allocated and freed
inside its own bracket. It is a guard rather than a pair of calls so that an unwind closes it, and
nesting is safe because each guard puts back what it found.

This does not make a free attributable in general. Charging a release to whoever allocated the block
needs per-request provenance, which is the request arena's to give; what holds today is that each
cross-request store brackets itself. A store whose entries are shared with the request that created
them cannot put the allocation and the release on one balance at all, and bracketing it anyway would
turn a bounded credit into an unbounded one — such a store stays unbracketed and records the gap in
its own module doc rather than taking the guard and breaking its symmetry.
