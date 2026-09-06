The exporter — the Prometheus scrape endpoint and the OTLP client — is a **Native, feature-gated**
subsystem (`rule:core-api/five-placements`), on by default in the server distribution, so a CLI
binary or a single-file executable does not carry an OTLP client and its transitive dependencies.

`Core\Metrics` is **Tier 0 in every build**. Placing it beside its exporter would make a program's
instrumentation calls compile in one build and not another, which makes the `Core` namespace
conditional — precisely what `rule:core-api/core-means-always-present` forbids. So a build without
the exporter still compiles every `Core\Metrics` call and still accumulates into the registry; a CLI
program that calls it holds a registry nothing will ever read, bounded and tiny, and the alternative
— members that are no-ops in one build and not in another — is a difference between builds, which
is worse. It is the same split the Core roster already uses for `Core\Cache`'s Redis backend, where
the class is always there and the driver is a feature.
