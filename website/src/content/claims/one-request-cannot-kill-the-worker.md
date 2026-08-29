---
claim: Nothing a request does can take down the server
category: security
comparedTo: [Node.js, PHP]
proof: 'ADR 0106: every worker root carries a panic barrier, request memory is capped and attributable, and the guarantee is tested by deliberately crashing requests. ADR 0002 contains runtime panics to the one request that caused them.'
tradeoff: 'Per-request isolation holds memory per request and copies data at isolate boundaries — Novis deliberately spends memory to buy this (ADR 0004).'
draft: true
weight: 20
---

A crashing request, an out-of-memory request or an infinitely-looping request kills
exactly that request. The process, and every other in-flight request, continues. This is
PHP's best operational property — shared-nothing request isolation — kept, while moving
to a long-lived, JIT-compiled process model.

In Node.js an uncaught exception or a blocked event loop affects every request in the
process. In PHP the isolation comes from paying process-per-request costs; Novis keeps
the isolation without paying them.
