A `[limits]` block may write `memory_high_water`, a fraction between `0.0` and `1.0`; a request whose
memory peak crosses that share of its effective ceiling writes one `Warn` to `Core\Log` at the end of
the request, naming the peak, the ceiling and the route.

**Unwritten is off, and off is silent** — a deployment gets no high-water log it did not ask for. A
written `0` is a threshold every request passes, not a second spelling of off. A request with no
ceiling at all (`[limits.hard] memory = false`) has no threshold either, because there is nothing for
a fraction to be a fraction of. A value outside `0.0..=1.0` is refused at boot, where it is written,
by the same typed-value path that refuses a malformed size.

**A fraction rather than a size**, so it keeps meaning the same thing under an `[app.limits]` block
that narrows the ceiling (`rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling`). A size
would have to be re-derived for every application block, and would silently stop being a warning at
all for any block that narrowed past it.

**One record per request, at the end of it**, from the mark
`rule:observability/a-memory-peak-is-recorded-not-asked-for` already holds — so this adds no probe
site to the measured path and `rule:observability/the-runtime-exports-what-it-already-measures` holds
for it as written.

This is the reading that closes the gap the other two leave. `Script\ExitReport::memoryPeak` requires
an application to have registered a hook, and `nvs_request_memory_peak_bytes` requires an exporter
and someone watching it; the request that matters is the one nobody instrumented and nobody was
watching. A breach needs no equivalent: `rule:errors/on-limit`'s report already names both numbers.
