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
