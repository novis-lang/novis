The exporter — the Prometheus scrape endpoint and the OTLP client — is a **Native, feature-gated**
subsystem (`rule:core-api/five-placements`), on by default in the server distribution, so a CLI
binary or a single-file executable does not carry an OTLP client and its transitive dependencies.

`Core\Metrics` is **Tier 0 in every build**. A build without the exporter still compiles every
`Core\Metrics` call and still accumulates into the registry, so behaviour is identical across builds
except for the export path. It is the same split the Core roster already uses for `Core\Cache`'s
Redis backend, where the class is always there and the driver is a feature. The server build pays
the exporter's dependency tree; a CLI build does not.

- **Memory:** a CLI program that calls `Core\Metrics` holds a registry nothing will ever read,
  bounded by `rule:observability/past-max-series-a-new-series-is-refused`. A program that never calls
  it holds none.

**Declined:**
- **`Core\Metrics` at Tier 2 beside its exporter**, for a smaller Tier 0. A program's instrumentation
  calls would compile in one build and not another, which makes the `Core` namespace conditional
  (`rule:core-api/core-means-always-present`).
- **Members that are no-ops in a build without the exporter.** That is a difference in behaviour
  between builds, which is worse than a small registry nobody reads.
