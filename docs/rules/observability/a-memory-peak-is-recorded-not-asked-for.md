The runtime keeps a request's memory high-water mark in every build, updated where the live-byte
balance is updated, and a nested `Ctx` restores the larger of its own mark and the one it displaced
rather than clobbering it.

The peak is **recorded, not sampled and not asked for**. It moves in the same allocator function that
already maintains the live balance, inside the branch that already tests for a positive delta, so it
is exact for every allocation rather than approximate between two reads. A sampled peak would miss
precisely the short spike that deterministic release makes invisible, which is the case the mark
exists for.

**Why a current figure is not enough.** Novis releases memory when the last reference dies
(`rule:security/arena-is-an-ownership-root`), so held bytes fall back toward the baseline as soon as
values die, and a reading taken at the end of a request says nothing about the high-water mark. A
Novis request that decoded a
90 MB payload and returned a 2 KB summary reports the 2 KB, and a request that sat at 96% of its
ceiling for most of its life is indistinguishable at exit from one that never passed 30%. The better
memory behaviour is what destroys the evidence, so the evidence is kept deliberately.

**Nesting saves and restores.** A `Ctx` created inside another rebases the mark to the current
balance and holds the enclosing value; on drop it publishes `max(enclosing, reached)`. Without that,
an isolate that allocated little would erase the peak of the request that spawned it. What the mark
is a peak *of* — one request, or an isolate tree — is
`rule:security/isolate-budget-is-the-trees`' to settle, and this rule inherits that boundary rather
than deciding it.

**It is not resettable.** The peak is evidence an operator needs, and a member that set it back to
the current figure would let an application hide the number the request-level notices exist to
surface. Bounding one section of a program is what `Core\Debug`'s probes are for.

**Memory:** one `isize` per thread and one per `Ctx`, the saved enclosing mark. Nothing per process,
nothing that grows with requests served, and nothing that outlives a request. On the request path the
update is one thread-local read and a well-predicted branch inside a branch that already exists; the
ABI probe's perf guards measure that cost rather than assert it.

**Declined:**
- **Leaving the peak to `Core\Debug`'s profiler.** A profiler answers "where did this go" for a
  developer who is already looking, and the request that matters is the one nobody was looking at.
- **Having the program record the peak itself** as the only mechanism. The request nobody
  instrumented is the one that matters; `Core\Budget::memoryPeak` is kept as a reading, never as the
  mechanism.
