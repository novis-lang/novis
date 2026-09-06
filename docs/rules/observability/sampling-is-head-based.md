Sampling is **head-based at the root**: `[trace] sample` is the probability, from `0.0` to `1.0`,
that a *new* trace is recorded. An inbound trace that is already sampled is always continued
regardless of the local fraction, because a partially recorded distributed trace is worse than none.

The decision governs export only. An id exists for every request whether or not it is sampled
(`rule:observability/a-trace-id-exists-for-every-request`), so an unsampled request still has a log
line that can be correlated with a proxy's.

Head sampling records one per cent of the errors too, when one per cent is the fraction. Tail
sampling — decide after the fact, keep the slow and the failed — needs a collector-side component or
a buffering exporter, and is deliberately not built.
