Sampling is **head-based at the root**: `[trace] sample` is the probability, from `0.0` to `1.0`,
that a *new* trace is recorded. An inbound trace that is already sampled is always continued
regardless of the local fraction, because a partially recorded distributed trace is worse than none.

The decision governs export only. An id exists for every request whether or not it is sampled
(`rule:observability/a-trace-id-exists-for-every-request`), so an unsampled request still has a log
line that can be correlated with a proxy's.

Head sampling records one per cent of the errors too, when one per cent is the fraction.

**Declined:**
- **Tail sampling**, deciding after the fact and keeping the slow and the failed, which is what
  operators want. It needs a collector-side component, which is configuration and not this runtime's
  code, or an exporter that buffers every span until the request ends, which is a footprint cost
  under `rule:programs/memory-priority` that has no stated bound yet.
