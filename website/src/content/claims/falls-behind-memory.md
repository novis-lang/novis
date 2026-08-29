---
claim: "Higher memory floor: Novis deliberately spends memory that other runtimes save"
category: honesty
comparedTo: [PHP, Node.js]
proof: 'ADR 0004 is the written policy: memory footprint is the lowest design priority, spent deliberately to buy security, correctness, speed and simplicity.'
draft: true
weight: 30
---

Per-request isolation holds memory per request. JIT-compiled code lives in memory.
Copy-on-write values copy when written to. Novis requires memory to stay attributable
and capped — growth with traffic is treated as a leak — but it will not win a
lowest-RSS benchmark against a tuned PHP-FPM pool or a single-heap Node.js process, and
it does not try to. On small containers this is a real cost.
