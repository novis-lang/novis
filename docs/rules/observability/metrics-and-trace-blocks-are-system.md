```toml
[metrics]                       # System
exporter   = false              # false | "prometheus" | "otlp"
listen     = "127.0.0.1:9090"   # prometheus scrape endpoint
endpoint   = ""                 # otlp collector URL
max_series = 10000              # per core

[trace]                         # System
exporter  = false               # false | "otlp"
endpoint  = ""
sample    = 0.01                # head-based, 0.0–1.0
propagate = true                # send traceparent on outbound Core\Http\Client calls
```

Both blocks are **`System`** (`rule:config/system-means-a-request-may-not-set-it`): where a process
ships telemetry, and how much it costs to do so, is a deployment decision, and a request able to
turn tracing on for itself is the reconnaissance channel `rule:testing/debug-mode-directive` already
refuses. A developer wanting a trace of their own request has `Core\Debug`, which is unaffected.

The two blocks share a grammar and not a roster: `exporter` is `false` or one word naming a
protocol, metrics take a scrape or a push, a trace takes only a push, and `sample` is a fraction of
one. Anything else is **refused at boot, where it is written** — a wrong value here produces
silence, a collector nothing writes to, and there is no later moment at which that reports itself.
