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

CI runs the conformance suite on every leg four times: at `opt_level` `speed`, again with `nvs test
--opt-level none`, again with `nvs test --probes`, and again with `NOVIS_CHECK_TYPES` set. `--opt-level` sets the level in the test
process and, as `NOVIS_OPT_LEVEL`, in every process a case starts (`nvs_codegen::set_opt_level`). It
is a flag so that `nvs test --help` lists it. The artifact cache stays on, because a unit's key
covers the level (`crates/nvs-cli/src/cache.rs`'s `unit_key`): a unit compiled at one level is never
loaded at the other.

`--probes` turns `nvs_runtime::DebugFlags::SITES` on in every context of the test process and, as
`NOVIS_PROBES`, of every process a case starts (`nvs_runtime::probe_every_site`). Compiled code
tests the whole flags word, so every statement, branch and call site calls its probe helper, and no
helper acts on that bit: the run records nothing and writes nothing. The data is the probe helpers'
half, which their unit tests cover; the compiled half, a call at every site, is the bug class above.
A program's own `Core\Debug` probes behave as in any run, because `Core\Debug` never starts or
stops `SITES` and a host call that sets the flags keeps it. Probes change no compiled code, so the
cache needs no key for them. Request path: one relaxed load per context made, and the bit is off
outside this run. Memory: nothing.

The fourth run is the checked build (`nvs_codegen::compile_checked_types`): each typed read also
tests its value against its declared type, and a value of another type stops the process naming the
line. A case whose output is right can still read a value as another type, and only this run sees
that.

Why one bit that records nothing, and not every recording bit on with its data discarded: a
recording bit is observable. `TRACE` files query spans and warns of a missing grant, and
`COVERAGE` counts into a table a program reads, so a case that tests `Core\Debug` would print
something else. Declined: turning on `COVERAGE`, `BRANCH`, `TRACE`, `PROFILE` and `COUNT` for the
run.
