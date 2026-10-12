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

**Not on disk.** The codegen tests compile with probes on (`crates/nvs-codegen/tests/probes.rs:276`),
and CI runs the conformance suite once, with probes off (`.github/workflows/ci.yml:184`). The coverage
flags are refused over a `.nvst` tree (`crates/nvs-cli/src/main.rs:3862`), and Cranelift's `opt_level`
is fixed at `speed` (`crates/nvs-codegen/src/lib.rs:1949`).
