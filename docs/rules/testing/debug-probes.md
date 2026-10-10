`Ctx` carries one small mutable bitset — coverage, branch, trace and profile — and codegen emits an
unconditional load-and-branch against it at exactly two kinds of site, present in every compiled
unit whether or not any request ever sets a bit. At **every statement boundary**: a per-line hit
counter under coverage, and the same at every conditional edge under branch, using the edge identity
the IR already carries. At **every call site**: an entry and an exit probe under trace, carrying the
same checked-return status the call site already branches on, so a trace records a thrown or fatal
exit as it happened rather than as a reconstruction; under profile the same pair accumulates
self and inclusive time per callee.

Nothing else. No probe at expression granularity and none inside a native scalar sequence, so the
added check count is bounded by source size rather than by how many instructions a statement lowers
to.

Every bit off costs one cached load and one predicted-not-taken branch — the cost class the
safepoint poll already pays. Turning a bit on for a running request is exactly setting the word: no
recompilation, no re-resolution, no second compiled unit. That is what makes starting and stopping
coverage **mid-request** work at all, which a compiled-in-advance instrumented tier cannot do.

The probe sites are their own, not the safepoint poll's. Safepoints sit only at loop back-edges and
function entry, deliberately sparse, and line and branch counting need a site at every statement and
edge. So the probes add more checks than the poll has, each of the same cheap shape, and the
all-bits-off cost per site is a measured guard rather than an assumption. If that guard ever shows a
real regression, the fix is coarsening the site to one per basic block, never dropping the flag word.
A debugger's breakpoints stay on the safepoint poll itself: a debugged request may or may not also
collect coverage, and the two are independent mechanisms.

**Declined:**
- **A second, instrumented compiled tier chosen per request.** A request's tier is fixed once it is
  resolved, so starting coverage mid-request would need frames already running to have been
  instrumented from the start. That either costs every request whether or not it asks, or leaves the
  per-test bracketing a coverage harness relies on broken.
- **Hooking a central dispatch routine, the way an interpreter's tracer does.** There is none to hook:
  typed scalar operations lower to native instructions with no common dispatch point.
