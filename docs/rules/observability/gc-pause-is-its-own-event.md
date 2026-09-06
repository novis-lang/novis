A stop-the-world collection run is a trace event of its own, `kind: gc`, recorded from **inside the
cycle collector's run routine** and gated by the same `TRACE`/`PROFILE` bits `Ctx` already carries.
It records a start timestamp, a duration and the number of objects freed.

It is emitted from the collector's routine and never from the safepoint poll, because the poll is
checked on every loop back-edge and function entry — precisely the hot path tracing must not touch —
while the collection run is already the rare, slow path where one more branch and a timestamp pair
cost nothing relative to the run itself.

What the event buys is honest attribution. Without it a pause is silently folded into the self time
of whichever function happened to be executing, and `PROFILE`'s self/inclusive figures carry a cost
that function did not cause. With it the pause is its own bar on the timeline, and in production
telemetry it becomes a metric rather than a span (`rule:observability/a-call-never-becomes-a-span`).
