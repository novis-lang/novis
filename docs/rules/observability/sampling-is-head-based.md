Sampling is **head-based at the root**: `[trace] sample` is the probability, from `0.0` to `1.0`,
that a *new* trace is recorded. An inbound trace that is already sampled is always continued
regardless of the local fraction, because a partially recorded distributed trace is worse than none.

The decision governs export only. An id exists for every request whether or not it is sampled
(`rule:observability/a-trace-id-exists-for-every-request`), so an unsampled request still has a log
line that can be correlated with a proxy's.

**With no `[trace] exporter`, nothing is recorded.** A recorded request files events and hands its
spans to the export queue, and with no exporter nothing drains that queue. So the door neither draws
against `sample` nor reads an inbound sampled flag: a client sending `-01` on every request with a
long path would otherwise fill the queue with text nothing ships. The trace id is still continued,
and an outbound call carries the flag as `-00`. The exporter is boot-class, so the check is one
lookup in the snapshot a request already reads.

Head sampling records one per cent of the errors too, when one per cent is the fraction.

**Declined:**
- **Passing an inbound sampled flag onward unchanged with no exporter.** It needs a second bit, one
  for this process's recording and one for the header, for a hop that records nothing anyway.
- **Tail sampling**, deciding after the fact and keeping the slow and the failed, which is what
  operators want. It needs a collector-side component, which is configuration and not this runtime's
  code, or an exporter that buffers every span until the request ends, which is a footprint cost
  under `rule:programs/memory-priority` that has no stated bound yet.
