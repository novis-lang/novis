Novis compiles natively, so every user call is a real machine frame. Exhausting a native stack is a
`SIGSEGV`, which `rule:errors/helper-abi`'s `catch_unwind` does not contain — one request would
take down the worker and every other request on it.

So recursion is bounded twice. A catchable `RecursionError` throws at a **soft** depth, so a
recursive-descent parser or a walk over untrusted-depth data can degrade instead of dying; this is
safe because `rule:errors/propagation` pops frames as it unwinds,
so the handler runs with a shallow stack again. A non-catchable `FATAL` at the true limit is the
floor beneath it and reaches `rule:errors/on-limit` like any other resource limit.

The ceiling is **8 MB of reserved address space per coroutine**, about 65,000 frames, of which only
touched pages are resident. The check is a comparison of the stack pointer against a `stack_limit`
field in `Ctx`'s existing hot cache line, emitted where the safepoint poll already loads that line,
and elided in a leaf function whose frame fits the reserved slack. The fast path compares against
the soft limit only. Measured against a 1.32 ns per-call slope it adds ≈0.3 ns — ≈0.06% on a
request making 40,000 calls, and nothing at all in loops and leaf-only code.

This bound is emitted at Novis function entry, so it reaches recursion through Novis frames and only
those. Request data also recurses through engine frames — a nested document in a decoder, a nested
value graph in teardown — which are bounded separately by an explicit depth counter and an
iterative teardown.
