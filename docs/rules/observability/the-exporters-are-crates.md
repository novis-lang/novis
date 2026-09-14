The OTLP path, including the W3C TraceContext propagator, and the Prometheus scrape path are somebody
else's specification, and they are implemented against it behind the feature
`rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` describes. A crate is taken for
a half that is only an encoder, and never for one that would bring its own runtime, its own client or
its own registry — `rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client` is
the predicate, and the reading of `metrics-exporter-prometheus` and `opentelemetry-otlp` that neither
passes it.

What is ours, and could not be a crate, is the wiring from the runtime's own event kinds to spans
(`rule:observability/four-kinds-become-a-span`) and the per-core registry
(`rule:observability/a-registry-is-per-core-and-nothing-reads-it`). The registry is deliberately
whole without an exporter: an exporter reads its series in order and formats them, and adding one
changes nothing above it. StatsD is not a wire format here — no histogram semantics worth the name
and no trace story at all.
