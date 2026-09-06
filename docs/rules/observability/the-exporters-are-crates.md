The OTLP path, including the W3C TraceContext propagator, and the Prometheus scrape path are
somebody else's specification and are taken as dependencies — `opentelemetry` and
`opentelemetry-otlp` for the one, `metrics-exporter-prometheus` for the other — behind the feature
`rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` describes.

What is ours, and could not be a crate, is the wiring from the runtime's own event kinds to spans
(`rule:observability/four-kinds-become-a-span`) and the per-core registry
(`rule:observability/a-registry-is-per-core-and-nothing-reads-it`). The registry is deliberately
whole without an exporter: an exporter reads its series in order and formats them, and adding one
changes nothing above it. StatsD is not a wire format here — no histogram semantics worth the name
and no trace story at all.
