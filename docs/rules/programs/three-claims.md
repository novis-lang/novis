Novis claims exactly three things about itself, and each is a property no incumbent can add later:

1. **Injection and secret leakage are compile errors**, not findings from a scanner run afterwards.
2. **A request, a scheduled job, a connection and an untrusted script are each an isolate**, in one
   process, sharing only compiled code, with an enforceable memory, CPU and time budget on each.
3. **Suspension has no colour** — any function may yield, so there is no `async` split through the
   library.

The first claim is the strongest. In every incumbent language taint checking is an external scanner
sold as its own product, because a type qualifier cannot be added to that language without breaking
the programs it already runs; `secret` has the same shape. A competitor closes a gap in a type system
with a minor release, and it cannot close this one.

**"Faster than PHP" is retired as a headline claim.** It remains true, it remains measured, and it is
the right thing to put in a benchmark table; as a *reason to adopt* it compares against a deployment
nobody defends, and the margin over a modern one does not pay for a rewrite. Latency and throughput
keep their standing place in `rule:programs/memory-priority`'s ordering — what changes is only what the
project says about itself first. The landing page, the tutorial and the first example lead with an
isolate and a qualifier, never with a benchmark and never with a PHP comparison. A PHP comparison page
may exist; it is not the front door.

**The first claim is worded about who enforces it, never about how complete the escaping is.** Not
"XSS is impossible": auto-escaping closes text-node XSS, while an unquoted or URL-valued attribute,
an inline `<script>` and CSS context are positions the escape does not cover
(`rule:core-classes/html-template`), and each needs the launderer for its own sink
(`rule:security/launderers-are-sink-named`). *Injection is a compile error* is the sentence the
project writes, and it is the stronger one. Nor is the first claim paired with "faster than PHP":
pairing a retired claim with a live one weakens both.
