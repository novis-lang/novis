---
claim: Throwing an exception costs no more than returning a value
category: performance
comparedTo: [PHP, Node.js]
proof: 'Measured at 0.79–0.82× of a normal return at depth 8 (ADR 0002), and the numbers are guarded continuously — the benchmark suite fails the build on an order-of-magnitude regression.'
tradeoff: 'The happy path pays one perfectly-predicted compare-and-branch per call, measured at 0.85 ns per frame — accepted, and written down.'
draft: true
weight: 10
---

Web frameworks throw exceptions on ordinary control-flow paths — a missing record, a
validation failure, a redirect. In most runtimes that means a performance cliff: stack
unwinding, trace capture, allocation. Novis propagates errors as a checked status the
optimizer can see, a throw allocates nothing, and the result is that error paths run at
the speed of success paths.

This claim is unusual enough that we publish the measurement methodology and keep the
benchmark in the repository, wired to fail the build if it stops being true.
