The conformance suite runs **at least once with coverage and tracing probes enabled**, and **once
per Cranelift optimisation level**, so neither is a shape only production ever takes.

This is not box-ticking; it is the one bug class the probe design is specifically exposed to —
instrumentation crossed with compiled code, which is where other JIT-compiled engines have
dereferenced stale caches and produced wrong results under optimisation. There is no interpreter
here to fall back to.

The differential oracle cannot find these. It checks that Novis agrees with **PHP**, not that Novis
agrees with **itself** under different codegen, and a probe-attached run and an optimised run are
both Novis. The cost is CI wall-clock proportional to the added axes, and nothing at all at run time.
