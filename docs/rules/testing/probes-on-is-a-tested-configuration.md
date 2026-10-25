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

CI runs the conformance suite on every leg three times: at `opt_level` `speed`, again with `nvs test
--opt-level none`, and again with `NOVIS_CHECK_TYPES` set. `--opt-level` sets the level in the test
process and, as `NOVIS_OPT_LEVEL`, in every process a case starts (`nvs_codegen::set_opt_level`). It
is a flag so that `nvs test --help` lists it. The artifact cache stays on, because a unit's key
covers the level (`crates/nvs-cli/src/cache.rs`'s `unit_key`): a unit compiled at one level is never
loaded at the other. That third run is the
checked build (`nvs_codegen::compile_checked_types`): each typed read also tests its value against its
declared type, and a value of another type stops the process naming the line. A case whose output is
right can still read a value as another type, and only this run sees that.

**Not on disk.** The probes half. The codegen tests compile with probes on
(`crates/nvs-codegen/tests/probes.rs:276`), but the coverage flags are refused over a `.nvst` tree
(`crates/nvs-cli/src/main.rs:2394`), so no suite run has probes on.
