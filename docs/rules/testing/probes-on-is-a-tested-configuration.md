The conformance suite runs **at least once with coverage and tracing probes enabled**, and **once
per Cranelift optimisation level**, so neither is a shape only production ever takes.

This is not box-ticking; it is the one bug class the probe design is specifically exposed to —
instrumentation crossed with compiled code, which is where other JIT-compiled engines have
dereferenced stale caches and produced wrong results under optimisation. There is no interpreter
here to fall back to.

A suite run under one configuration cannot find these. Each case checks that Novis prints what the
rules say under that one codegen, and this bug class lives only where a probe-attached or optimised
build of the same program prints something else. The cost is CI wall-clock proportional to the added
axes, and nothing at all at run time.
