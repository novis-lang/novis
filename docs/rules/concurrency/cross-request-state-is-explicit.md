A value outlives the request that made it only by being **put into a named store**. There is no
ambient place to leave one: no shared segment, no cross-request superglobal, no static that survives
a request, and no process-wide table a later request can read.

`Core\Cache` is the sanctioned exception, and it is a member per tier — each handing back a store,
none taking a flag — so the choice a program made is visible in review rather than buried in an
argument list. The memory tier is a **cache and not a store** — it must always be correct to find
nothing there — and anything whose value is relied upon uses the shared tier
(`rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent`).

The test is **what a program relies on**, not where bytes live. Per-core state derived from its own
inputs and unreadable by any program is not cross-request state: a compiled-pattern memo is
observable only as speed, and a metrics registry holds approximate aggregates that no program can
read and nothing decides on. Both are capped and neither grows with the requests served, and a
store's bytes stay off every request's reading by the balance
`rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance` describes.
