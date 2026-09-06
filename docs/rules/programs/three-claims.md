Novis claims exactly three things about itself, and each is a property no incumbent can add later:

1. **Injection and secret leakage are compile errors**, not findings from a scanner run afterwards.
2. **A request, a scheduled job, a connection and an untrusted script are each an isolate**, in one
   process, sharing only compiled code, with an enforceable memory, CPU and time budget on each.
3. **Suspension has no colour** — any function may yield, so there is no `async` split through the
   library.

**"Faster than PHP" is retired as a headline claim.** It remains true, it remains measured, and it is
the right thing to put in a benchmark table; as a *reason to adopt* it compares against a deployment
nobody defends, and the margin over a modern one does not pay for a rewrite. Latency and throughput
keep their standing place in `rule:programs/memory-priority`'s ordering — what changes is only what the
project says about itself first. The landing page, the tutorial and the first example lead with an
isolate and a qualifier, never with a benchmark and never with a PHP comparison. A PHP comparison page
may exist; it is not the front door.
