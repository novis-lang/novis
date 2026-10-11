An exporter brings no second registry, no second scheduler and no second client: a crate is taken for
a half that needs none of those — an encoder — and never for a half that carries its own execution or
its own socket.

The Prometheus scrape endpoint is served by this server's own accept loop, on the listener
`[metrics] listen` names — one more listening socket driven by the same loop as every other, never a
second HTTP server. A push goes out over `nvs-host`'s parking stream and the one TLS client this
workspace has, under `Core\Http\Client`'s own bounds.

Both wire formats are therefore written here, against their specifications, and what a crate may
supply is encoding: a protobuf encoder is exactly the shape that qualifies, because it is code
generation and bytes with no runtime, no socket and no trust store. Read from their published
manifests, neither `metrics-exporter-prometheus` nor `opentelemetry-otlp` is takeable. The first's
default features bring `tokio` with a multi-threaded runtime, `hyper`'s client half and `hyper-rustls`
with its own TLS provider; with them off it is separable from a transport but not from `metrics` and
`metrics-util`'s registry and storage, a registry with its own recency policy and its own clock, which
would stand beside the per-core one
(`rule:observability/a-registry-is-per-core-and-nothing-reads-it`). The second's every transport
feature is a client, and with all of them off `opentelemetry_sdk` is still a required dependency and
brings `futures-executor`; `opentelemetry-proto`, the message types alone, requires it too.

The objections are security and simplicity agreeing. A second scheduler in a thread-per-core
runtime is what `rule:concurrency/one-scheduler` already refuses; a second HTTP client and a second
TLS provider are a second set of verification defaults to keep in step, and a provider that is a C
dependency is priced again by `rule:packaging/a-c-dependency-answers-two-questions`. What it costs is
that two versioned formats are ours to track, which is the trade taken deliberately: both are
versioned specifications with stability guarantees.

**Declined:**
- **`metrics-exporter-prometheus` with `default-features = false`.** It passes the runtime and
  transport test and fails on the registry: keeping two means copying every series at every scrape,
  and replacing ours means writing the per-core merge and the `max_series` refusal
  (`rule:observability/past-max-series-a-new-series-is-refused`) against a crate's policy.
- **`opentelemetry-otlp` with every transport feature off.** Not available, because
  `opentelemetry_sdk` is required and brings an executor.

Either is taken up again only when a release makes its encoder separable — a Prometheus renderer of
a caller's own series, or OTLP message types that do not require `opentelemetry_sdk` — and the
manifests are read again before concluding either way.
