```php
Core\Metrics::increment(string $name, {by?: uint, labels?: array<string, string>}): void;
Core\Metrics::observe(string $name, float $value, {labels?: array<string, string>}): void;
Core\Metrics::gauge(string $name, float $value, {labels?: array<string, string>}): void;
```

`increment` for a counter, `observe` for a histogram, `gauge` for a point-in-time value — three
verbs for three kinds, rather than one `record` with a kind enum, because the kind is a property of
the series and not of the call (`rule:observability/a-name-is-fixed-to-one-kind`). A `$name` given
as a string literal is validated at compile time against `[a-z][a-z0-9_]*`, by the same inspection
of string literals at the call site that `rule:security/secret-sinks-refuse` performs.

The `labels` value position is a sink: `rule:security/metric-label-refuses-tainted` is the rule a
user id, a tenant name or an error message runs into, and `secret` is refused there too.

**`Core\Metrics` is always present.** Whether anything leaves the process is the exporter's
question (`rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`); a binary built
without one still accumulates into the per-core registry, so behaviour is identical across builds
except for the export path. A program that never calls it and serves no requests holds zero series.
