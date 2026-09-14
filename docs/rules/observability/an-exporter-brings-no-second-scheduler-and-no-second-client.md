An exporter brings no second registry, no second scheduler and no second client: a crate is taken for
a half that needs none of those — an encoder — and never for a half that carries its own execution or
its own socket.

The Prometheus scrape endpoint is served by this server's own accept loop, on the listener
`[metrics] listen` names — one more listening socket driven by the same loop as every other, never a
second HTTP server. A push goes out over `nvs-host`'s parking stream and the one TLS client this
workspace has, under `Core\Http\Client`'s own bounds.

Both wire formats are therefore written here, against their specifications, and what a crate may
supply is encoding: a protobuf encoder is exactly the shape that qualifies, because it is code
generation and bytes with no runtime, no socket and no trust store. `docs/decisions/0186.md`
§ *Investigation* is the reading of `metrics-exporter-prometheus` and `opentelemetry-otlp` that
found neither takeable — the first is separable from a transport but not from `metrics` and
`metrics-util`'s own registry, which would stand beside the per-core one
(`rule:observability/a-registry-is-per-core-and-nothing-reads-it`); the second cannot be reached
without either a second HTTP client or `opentelemetry_sdk`'s executor, which is a required
dependency rather than an optional one.

The objections are priority 1 and priority 4 agreeing. A second scheduler in a thread-per-core
runtime is what `rule:concurrency/one-scheduler` already refuses; a second HTTP client and a second
TLS provider are a second set of verification defaults to keep in step, and a provider that is a C
dependency is priced again by `rule:packaging/a-c-dependency-answers-two-questions`. What it costs is
that two versioned formats are ours to track, which is the trade taken deliberately.
