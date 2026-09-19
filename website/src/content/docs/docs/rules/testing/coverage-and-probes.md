---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Coverage, tracing and the debug probes"
description: "One per-request flag word at fixed probe sites, exporting formats existing tooling already reads — never a second compiled tier."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/testing/feature-proofs/
  label: "Feature proofs"
next:
  link: /docs/rules/testing/measuring-performance/
  label: "Measuring performance"
---

<p class="nv-section-lead">One per-request flag word at fixed probe sites, exporting formats existing tooling already reads — never a second compiled tier.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">11</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">4</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">7</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">1</span><span class="nv-count-label">differs from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#capability-closure-test">Every member of a capability-bearing class declares a capability or declares none, and there is no allowlist</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#attribution-is-diffed-in-ci">The third-party notice is committed, and a check fails when the lockfile moved without it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#debug-mode-directive"><code>[debug] mode</code> is the ceiling as well as the default, and writing a trace or a profile is its own capability</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#debug-probes">Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#debug-surface">Each probe starts and stops mid-request, and exports what existing tooling already reads</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#probes-on-is-a-tested-configuration">The conformance suite runs with the probes on, and once per optimisation level</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#writing-a-debug-record-to-disk-is-its-own-gate"><code>[debug] store</code> is off by default and can only subtract, so no run mode or log level ever turns debug records on</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-debug-stream-reaches-the-service-through-the-disk">A debug record reaches the service through a sealed spool file that one writer owns, never over a socket</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-ingester-runs-whether-or-not-anyone-is-looking">The ingester owns every write to the index and runs without a browser, and the viewer only ever reads</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#a-bad-line-degrades-and-a-bad-file-is-quarantined">A malformed line becomes a visible record and an unreadable file is quarantined under a backoff, so ingestion never stops</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-viewer-shows-the-stream-and-hands-off-every-artifact">The viewer renders the correlated request stream and hands every artifact with an existing viewer to it</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

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

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/feature-proofs/#feature-proofs" title="A feature is finished when its feature proofs exist: a description, a test from both sides, three examples, a measured figure and an attack"><code>testing/feature-proofs</code></a> <a href="/docs/rules/testing/feature-proofs/#hostile-case-contract" title="A hostile case passes when nothing came apart, and a compile diagnostic is never a pass"><code>testing/hostile-case-contract</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/capability.rs"><code>crates/nvs-stdlib/tests/capability.rs</code></a></dd></div></dl>

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

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-mode-directive" title="[debug] mode is the ceiling as well as the default, and writing a trace or a profile is its own capability"><code>testing/debug-mode-directive</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#report-formats" title="One verdict, three renderings, a machine format owns stdout alone, and the JSON one locates every test it reports"><code>testing/report-formats</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a></dd></div></dl>

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

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/testing/continuous-integration/#the-deep-lane" title="Miri, the fuzzers and the unsafe audit run nightly and at release, and the fuzz corpus persists"><code>testing/the-deep-lane</code></a> <a href="/docs/rules/testing/feature-proofs/#nvst-is-separate" title=".nvst proves the language; #[Test] is how a program written in Novis tests itself"><code>testing/nvst-is-separate</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0018.md">record 0018</a></dd></div></dl>

</div>

<div class="nv-rule" id="writing-a-debug-record-to-disk-is-its-own-gate">

## `[debug] store` is off by default and can only subtract, so no run mode or log level ever turns debug records on

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#writing-a-debug-record-to-disk-is-its-own-gate"><code>testing/writing-a-debug-record-to-disk-is-its-own-gate</code></a>
</div>

`[debug] store` decides whether debug records are persisted at all, is off by default, and can only
subtract — no run mode, no log level and no other directive turns it on.

It is deliberately not a reading of the run mode and deliberately not a level threshold. `[log] level`
is a legitimate operational dial whose production default is `Info` and which a tree may legally
raise to chase a bug; if the debug stream rode it, that ordinary action would silently begin
persisting complete typed dumps of request data. **A reasonable operational dial must not double as a
data-exposure switch.**

So this is a second gate read together with every ceiling already in force, and off wherever either
says off. [`testing/debug-mode-directive`](/docs/rules/testing/coverage-and-probes/#debug-mode-directive "[debug] mode is the ceiling as well as the default, and writing a trace or a profile is its own capability")'s reconnaissance argument does not weaken because a
second key said yes, and a tree that writes no `[debug]` block gets no debug stream at all.

With the gate off nothing reaches the disk, so a debug record that survives into a deployment costs
the predicted-not-taken branch [`testing/debug-probes`](/docs/rules/testing/coverage-and-probes/#debug-probes "Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier") already pays and nothing further.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#debug-mode-directive" title="[debug] mode is the ceiling as well as the default, and writing a trace or a profile is its own capability"><code>testing/debug-mode-directive</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-surface" title="Each probe starts and stops mid-request, and exports what existing tooling already reads"><code>testing/debug-surface</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#debug-dump" title="A dump goes to the log, and reaches a response body only in development"><code>errors/debug-dump</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-level" title="Five levels, and the mapping to syslog is fixed"><code>errors/log-level</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0163.md">record 0163</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-debug-stream-reaches-the-service-through-the-disk">

## A debug record reaches the service through a sealed spool file that one writer owns, never over a socket

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-debug-stream-reaches-the-service-through-the-disk"><code>testing/the-debug-stream-reaches-the-service-through-the-disk</code></a>
</div>

A debug record reaches the service by being written to a file the writing core owns alone, never over
a socket, and the service reads a spool file only once it has been sealed.

The application therefore opens no listener and makes no outbound connection for this: there is
nothing on the request path but a buffered append to an already-open descriptor, which is cheaper
than a socket write rather than more expensive. It is never `fsync`ed per record — that, and only
that, is what would make this cost milliseconds. Nothing is lost while the service is stopped,
because the files are simply still there when it starts.

**One file per writer, sealed by an atomic rename** at a size or an age, whichever comes first. Per
writer because the server accepts per core, and `O_APPEND` gives an atomic offset bump and not an
atomic large write, so a shared file would tear records that a dumped object tree easily makes large
enough to tear. Sealed by rename because a reader then never has to ask whether a file is still being
written. Deleted after ingest, so this stream needs no rotation of its own. A file a crash left
unsealed and untouched for longer than any live writer would take is swept by the same pass.

The seal interval is the latency between a dump and its appearing, which is what buys everything
above.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#writing-a-debug-record-to-disk-is-its-own-gate" title="[debug] store is off by default and can only subtract, so no run mode or log level ever turns debug records on"><code>testing/writing-a-debug-record-to-disk-is-its-own-gate</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-surface" title="Each probe starts and stops mid-request, and exports what existing tooling already reads"><code>testing/debug-surface</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#engine-floor" title="Tier 4 — the floor is native, bounded, and gives up rather than escalating"><code>errors/engine-floor</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0163.md">record 0163</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-ingester-runs-whether-or-not-anyone-is-looking">

## The ingester owns every write to the index and runs without a browser, and the viewer only ever reads

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-ingester-runs-whether-or-not-anyone-is-looking"><code>testing/the-ingester-runs-whether-or-not-anyone-is-looking</code></a>
</div>

The ingester owns every write to the index and runs from the moment the service starts, whether or
not a browser has ever connected; the viewer is a reader that never opens a log file, parses a line
or scans a directory.

That split is both the performance argument and the correctness one. The viewer's worst case is a
bounded indexed query, so no enormous or corrupt file can reach the UI; and draining the spool is not
an effect of somebody having a tab open. Putting the ingester inside the application process is
refused: it would put directory scanning and index writes on the request path, in every deployment,
to save a hop that costs nothing.

**Two ingest modes.** A spool file is read whole, inserted, and deleted. The application's own log is
read from a checkpoint, inserted, and never touched — the checkpoint keyed on the file's identity
plus its offset plus a hash of its first line, because a path misses a rotation and an identity alone
misses identity reuse and truncation in place. Every record carries a **dedupe key** under a unique
index and inserts ignoring conflicts, so re-ingesting anything is harmless: the writer's id and
sequence for a spool record, the file identity and byte offset for a tailed one.

The order is read, insert, commit, **then** delete. A crash in that window re-ingests, which the
dedupe key makes a no-op. The index holds nothing that cannot be rebuilt from the files, which is
what licenses running it in its fastest mode and makes a corrupt index a cache miss to discard rather
than a failure to resolve.

**The viewer is pushed a watermark, not records** — a monotonic sequence published after each
committed batch, with the browser fetching the delta through the same query path it uses for
everything else. One rendering path serves live and historical data, a backgrounded tab cannot make
the ingester buffer, and a browser that was closed needs no replay buffer. That sequence, not a
record's own timestamp, orders the live view, which makes it immune to clock skew between writers and
to a garbage timestamp in a corrupt file.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#the-debug-stream-reaches-the-service-through-the-disk" title="A debug record reaches the service through a sealed spool file that one writer owns, never over a socket"><code>testing/the-debug-stream-reaches-the-service-through-the-disk</code></a> <a href="/docs/rules/testing/coverage-and-probes/#a-bad-line-degrades-and-a-bad-file-is-quarantined" title="A malformed line becomes a visible record and an unreadable file is quarantined under a backoff, so ingestion never stops"><code>testing/a-bad-line-degrades-and-a-bad-file-is-quarantined</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-write" title="One write path, and the engine floor is its other caller"><code>errors/log-write</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0163.md">record 0163</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-bad-line-degrades-and-a-bad-file-is-quarantined">

## A malformed line becomes a visible record and an unreadable file is quarantined under a backoff, so ingestion never stops

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#a-bad-line-degrades-and-a-bad-file-is-quarantined"><code>testing/a-bad-line-degrades-and-a-bad-file-is-quarantined</code></a>
</div>

A malformed line becomes a record that says so and an unreadable file is quarantined under a backoff,
so nothing on disk can stop the ingester.

Line length is capped, and past the cap the line is truncated, marked, and skipped to the next
newline. A corrupt file is often one enormous line with no newline in it, and reading it whole is how
a log tailer dies.

A line that will not parse is kept as an `unparseable` record carrying its file, its offset and its
truncated bytes, and is shown. Dropping it silently would leave a viewer quietly disagreeing with
what is on disk, which is worse than an ugly row. Invalid UTF-8 is a lossy conversion with a flag,
never an error.

A file that fails at the file level — permissions, an IO error, a mount that went away — is
quarantined with its error and a retry-after, never retried hot, and listed in the UI, so a file that
stopped being read says so instead of merely not appearing. Other files keep flowing.

Work is bounded per source per pass and sources are taken in turn, so one enormous file cannot starve
the live spool. A disk that is full or an index that errors backs off rather than spinning, and never
deletes a spool file whose batch did not commit.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/coverage-and-probes/#the-ingester-runs-whether-or-not-anyone-is-looking" title="The ingester owns every write to the index and runs without a browser, and the viewer only ever reads"><code>testing/the-ingester-runs-whether-or-not-anyone-is-looking</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0163.md">record 0163</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-viewer-shows-the-stream-and-hands-off-every-artifact">

## The viewer renders the correlated request stream and hands every artifact with an existing viewer to it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-viewer-shows-the-stream-and-hands-off-every-artifact"><code>testing/the-viewer-shows-the-stream-and-hands-off-every-artifact</code></a>
</div>

The viewer renders the correlated request stream and the log, and every artifact that an existing
tool already renders is handed to that tool rather than drawn.

A profile is handed to speedscope in the format [`observability/speedscope-timeline-export`](/docs/rules/observability/traces/#speedscope-timeline-export "The four event kinds export as one speedscope evented timeline, beside Callgrind, Clover/lcov and NDJSON")
already commits to, and a coverage report to whatever reads Clover or lcov. A coverage **heatmap**
over source is drawn, because that is a per-line background over text rather than a viewer. This is
[`ide/the-extension-builds-no-ui-the-editor-already-has`](/docs/rules/ide/the-vs-code-extension/#the-extension-builds-no-ui-the-editor-already-has "Coverage, server health, profiles and the debugger reach the editor through its own APIs and open formats — FileCoverage, LanguageStatusItem, DAP's UI, speedscope — and the extension builds none of them")'s test applied rather than set aside:
where a renderer exists, feed it. The live, correlated request stream is the one thing here that has
no incumbent in any ecosystem, which is the whole of the exception.

The boundary is written as a rule rather than left as an intention because the failure mode of an
in-house dashboard is that it slowly grows a second copy of every tool the project deliberately did
not build.

**The request is the primary object.** The main view lists requests — route, status, duration, query
count, dump count — and one opens to its own timeline; the flat filterable stream is the second view,
not the front page. The correlation key is the trace id every request already carries
([`observability/a-trace-id-exists-for-every-request`](/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request "A trace id exists for every request whatever the sampling decision, and it is the only request identifier")), so it costs nothing to derive. In
development the response also carries that id in a header so the viewer can move from the call being
looked at to its timeline; in production it does not, being a correlation and fingerprinting leak.

The value tree the browser receives is already finite — [`errors/record-transformations`](/docs/rules/errors/diagnostics-and-logging/#record-transformations "Redaction, control bytes, bidi and elision are decided in the record") bounded
it with elision and resolved every repeat to an identity before it was written — so a repeat renders
as a link to the node it names and expansion needs no fetch. Children are built from data already
loaded.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-vs-code-extension/#the-extension-builds-no-ui-the-editor-already-has" title="Coverage, server health, profiles and the debugger reach the editor through its own APIs and open formats — FileCoverage, LanguageStatusItem, DAP's UI, speedscope — and the extension builds none of them"><code>ide/the-extension-builds-no-ui-the-editor-already-has</code></a> <a href="/docs/rules/observability/traces/#speedscope-timeline-export" title="The four event kinds export as one speedscope evented timeline, beside Callgrind, Clover/lcov and NDJSON"><code>observability/speedscope-timeline-export</code></a> <a href="/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request" title="A trace id exists for every request whatever the sampling decision, and it is the only request identifier"><code>observability/a-trace-id-exists-for-every-request</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#record-transformations" title="Redaction, control bytes, bidi and elision are decided in the record"><code>errors/record-transformations</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0163.md">record 0163</a></dd></div></dl>

</div>
