---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Coverage, tracing and the debug probes"
description: "One per-request flag word at fixed probe sites, exporting formats existing tooling already reads — never a second compiled tier."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/testing/the-four-proofs/
  label: "The four proofs"
next:
  link: /docs/rules/testing/measuring-performance/
  label: "Measuring performance"
---

<p class="nv-section-lead">One per-request flag word at fixed probe sites, exporting formats existing tooling already reads — never a second compiled tier.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">6</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">4</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">2</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">1</span><span class="nv-count-label">differs from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#capability-closure-test">Every member of a capability-bearing class declares a capability or declares none, and there is no allowlist</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#attribution-is-diffed-in-ci">The third-party notice is committed, and a check fails when the lockfile moved without it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#debug-mode-directive"><code>[debug] mode</code> is the ceiling as well as the default, and writing a trace or a profile is its own capability</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#debug-probes">Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#debug-surface">Each probe starts and stops mid-request, and exports what existing tooling already reads</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#probes-on-is-a-tested-configuration">The conformance suite runs with the probes on, and once per optimisation level</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="capability-closure-test">

## Every member of a capability-bearing class declares a capability or declares none, and there is no allowlist

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#capability-closure-test"><code>testing/capability-closure-test</code></a>
</div>

A class is **capability-bearing** when any of its members has a row in the capability table. For
such a class **every member owes exactly one row**, and a member that genuinely needs no
capability — one that manipulates a string and touches no disk — declares that by entering the table
with *no capability*, rather than by staying out of it.

There is no allowlist, and that is the point. A member that is hard to classify is a member whose
capability has not been thought about, and the answer is to think about it, not to exempt it — so
the one move this design forbids is the one an exception list makes cheapest. The claim is about the
whole *set*, and a set with a growable exception list makes no claim at all. Declaring "nothing"
costs what declaring a real capability costs, is reviewed in the same table beside its reason, and
grants nothing, because no row grants anything.

A second closure test covers the other half: nothing in the standard library reaches the operating
system except through a door. Neither test subsumes the other — one catches a member that goes
through a door undeclared, the other a member that reaches the OS with no door at all.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/the-four-proofs/#four-proofs" title="A feature is finished when it has a test from both sides, three examples, a measured figure and an attack"><code>testing/four-proofs</code></a> <a href="/docs/rules/testing/the-four-proofs/#hostile-case-contract" title="A hostile case passes when nothing came apart, and a compile diagnostic is never a pass"><code>testing/hostile-case-contract</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/capability.rs"><code>crates/nvs-stdlib/tests/capability.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="attribution-is-diffed-in-ci">

## The third-party notice is committed, and a check fails when the lockfile moved without it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#attribution-is-diffed-in-ci"><code>testing/attribution-is-diffed-in-ci</code></a>
</div>

`python tools/gen-attribution.py --check` regenerates the third-party notice and compares it,
failing when the lockfile has moved and the notice has not. It runs beside the dependency-policy
job, which it completes: one decides what may be linked, this decides what must be shipped.

Committing a generated file is deliberate. It makes the notice reviewable in a diff at the moment a
dependency changes, and it keeps the build from depending on network access or on Python.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/continuous-integration/#lane-table" title="One Python table decides what a diff runs, and the platform matrix is never one of the gates"><code>testing/lane-table</code></a> <a href="/docs/rules/testing/continuous-integration/#ci-lanes" title="One workflow holds every job, and three lanes decide which of them a run needs"><code>testing/ci-lanes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0065.md">record 0065</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0143.md">record 0143</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tools/gen-attribution.py"><code>tools/gen-attribution.py</code></a> <a href="https://github.com/novis-lang/novis/blob/main/.github/workflows/ci.yml"><code>.github/workflows/ci.yml</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="debug-mode-directive">

## `[debug] mode` is the ceiling as well as the default, and writing a trace or a profile is its own capability

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#debug-mode-directive"><code>testing/debug-mode-directive</code></a>
</div>

`[debug] mode` in `nvs.toml` states the default **and** the ceiling in one value: `[]` is off, and
any subset of `coverage`, `branch`, `trace` and `profile` is a ceiling a request may narrow and can
never widen. A production tree sets `mode = []` and no request-side call can turn any bit on.

Whether a running request's internals are observable is not a request-local decision. An
attacker-controlled request that could turn tracing on for itself in production would gain a
reconnaissance channel over call arguments and timing, which is not a trade this language makes.

Writing a trace or a profile is its own capability — `debug.trace` and `debug.profile`, each granted
to named roots, deny-by-default, and **separate from the grant to write a file**. Being able to
write an ordinary file is not permission to persist a continuous log of every call's arguments,
which can carry request data a coverage-only deployment never needed to expose.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-surface" title="Each probe starts and stops mid-request, and exports what existing tooling already reads"><code>testing/debug-surface</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/tree.rs"><code>crates/nvs-config/src/tree.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="debug-probes">

## Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#debug-probes"><code>testing/debug-probes</code></a>
</div>

`Ctx` carries one small mutable bitset — coverage, branch, trace and profile — and codegen emits an
unconditional load-and-branch against it at exactly two kinds of site, present in every compiled
unit whether or not any request ever sets a bit. At **every statement boundary**: a per-line hit
counter under coverage, and the same at every conditional edge under branch, using the edge identity
the IR already carries. At **every call site**: an entry and an exit probe under trace, carrying the
same checked-return status the call site already branches on, so a trace records a thrown or fatal
exit as it happened rather than as a reconstruction; under profile the same pair accumulates
self and inclusive time per callee.

Nothing else. No probe at expression granularity and none inside a native scalar sequence, so the
added check count is bounded by source size rather than by how many instructions a statement lowers
to.

Every bit off costs one cached load and one predicted-not-taken branch — the cost class the
safepoint poll already pays. Turning a bit on for a running request is exactly setting the word: no
recompilation, no re-resolution, no second compiled unit. That is what makes starting and stopping
coverage **mid-request** work at all, which a compiled-in-advance instrumented tier cannot do.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no instrumented build and no extension to load; the probes are compiled into every unit and a bit is flipped on a request already in flight</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#debug-surface" title="Each probe starts and stops mid-request, and exports what existing tooling already reads"><code>testing/debug-surface</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-mode-directive" title="[debug] mode is the ceiling as well as the default, and writing a trace or a profile is its own capability"><code>testing/debug-mode-directive</code></a> <a href="/docs/rules/testing/measuring-performance/#bench-counters" title="#[Bench] reports counted semantic work, and CI may gate on the counts but never on wall-clock"><code>testing/bench-counters</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0041.md">record 0041</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-codegen/tests/probes.rs"><code>crates/nvs-codegen/tests/probes.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/ctx/trace.rs"><code>crates/nvs-runtime/src/ctx/trace.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="debug-surface">

## Each probe starts and stops mid-request, and exports what existing tooling already reads

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#debug-surface"><code>testing/debug-surface</code></a>
</div>

`Core\Debug` starts and stops each probe from inside a request — coverage with an optional branch
half, a trace to a named sink, a profile to another — and reads coverage back as a path-to-line-to-
count map. `nvs test` and `nvs run` grow flags that wrap a whole run in those calls with no source
change at all, which is the ergonomics CI uses day to day.

What comes out is what existing tooling already reads: Clover and lcov for coverage, Callgrind for
the profile. The trace is newline-delimited JSON, because no third-party tooling widely consumes any
trace format either.

Line and branch counters are bounded by the code a request touches, not by how often a loop
iterates, so they live in the request's arena and are dropped with it. Trace and profile data are
call-count-proportional, so they **stream to a sink** named when they start rather than accumulating
in memory — nothing to bound with a limit this rule would otherwise have to invent.

A spawned isolate starts from its parent's current flag word and may only narrow it. Its own data
never merges into the parent's live state; it comes back as fields on the result, by the same
copy-out rule a value, an error and a usage figure already use.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-mode-directive" title="[debug] mode is the ceiling as well as the default, and writing a trace or a profile is its own capability"><code>testing/debug-mode-directive</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#report-formats" title="One verdict, three renderings, and a machine format owns stdout alone"><code>testing/report-formats</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a></dd></div></dl>

</div>

<div class="nv-rule" id="probes-on-is-a-tested-configuration">

## The conformance suite runs with the probes on, and once per optimisation level

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#probes-on-is-a-tested-configuration"><code>testing/probes-on-is-a-tested-configuration</code></a>
</div>

The conformance suite runs **at least once with coverage and tracing probes enabled**, and **once
per Cranelift optimisation level**, so neither is a shape only production ever takes.

This is not box-ticking; it is the one bug class the probe design is specifically exposed to —
instrumentation crossed with compiled code, which is where other JIT-compiled engines have
dereferenced stale caches and produced wrong results under optimisation. There is no interpreter
here to fall back to.

The differential oracle cannot find these. It checks that Novis agrees with **PHP**, not that Novis
agrees with **itself** under different codegen, and a probe-attached run and an optimised run are
both Novis. The cost is CI wall-clock proportional to the added axes, and nothing at all at run time.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/testing/continuous-integration/#the-deep-lane" title="Miri, the fuzzers and the unsafe audit run nightly and at release, and the fuzz corpus persists"><code>testing/the-deep-lane</code></a> <a href="/docs/rules/testing/the-four-proofs/#nvst-is-separate" title=".nvst proves the language; #[Test] is how a program written in Novis tests itself"><code>testing/nvst-is-separate</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a></dd></div></dl>

</div>
