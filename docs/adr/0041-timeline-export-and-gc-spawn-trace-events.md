# ADR 0041 — Trace/profile output gains a speedscope-evented timeline export, plus GC-pause and isolate-spawn trace events

- **Status:** Accepted
- **Date:** 2026-08-22
- **Scope:** the trace/profile event taxonomy and export formats defined in
  [ADR 0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md). Adds a `kind` field to trace
  events (`call` | `gc` | `spawn` | `query`), three new instrumentation points (the cycle collector's run
  routine; the three isolate-spawn/join runtime routines; each `Core\Db` statement, per
  [ADR 0067](0067-core-db.md) § 11), and a speedscope "evented" export alongside ADR 0018's existing
  Clover/lcov/Callgrind/NDJSON output. Does **not** add any probe to the per-statement/per-call hot path ADR
  0018 already committed to, and does not cover coroutine suspend/resume events, an external/live attach
  mechanism, or memory/allocation profiling — all named and deliberately deferred, see *Revisiting*.
- **Amends:** [0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
- **Amended by:** 0067 — the fold is applied below; this body states the current rule.
- **Relates to:** 0002, 0006, 0040, 0067

> **In short:** ADR 0018's deterministic profiler exports aggregate Callgrind totals (whole-run self/inclusive
> time per function, no per-instance timeline) and a per-instance trace only as MWL-native NDJSON, which no
> ecosystem viewer reads. Neither GC pauses nor isolate-spawn boundaries appear as events at all — a
> stop-the-world collection run gets silently folded into whichever function's self time happened to be
> executing, and a `spawn`'s cost is one opaque number that does not separate real child compute from
> isolate-scheduling/copy-out overhead. This amendment adds a `kind` tag (`call`/`gc`/`spawn`/`query`) to
> trace events, instruments the cycle collector's run routine, the three fixed isolate-spawn/join routines
> and each database statement
> (gated by the same `TRACE`/`PROFILE` bits `Ctx` already carries), and adds a speedscope-evented export that
> renders all four kinds as one scrollable timeline in speedscope.app — reusing the exact format ADR 0040
> already committed to for the sampling profiler, so no bespoke viewer is built. None of this touches the
> per-statement/per-call check ADR 0018 measures on the hot path: the new instrumentation lives entirely
> inside routines that are already rare and already slow (a GC run, a spawn), so the marginal cost is
> effectively free relative to what those routines already cost, and zero when tracing is off.

## Context

- Raised directly by the user: they want a Chrome-DevTools-style, scrollable timeline of "the complete
  program flow" — not just function calls, but GC pauses and isolate boundaries too — while explicitly not
  wanting to "bloat core code for little advantage."
- ADR 0018's `PROFILE` bit produces Callgrind's aggregate format (total self/inclusive time per function
  across the whole run), which cannot show "this specific call at this specific timestamp took this long."
  `TRACE` *does* carry per-instance entry/exit timestamps, but only as MWL-native NDJSON — ADR 0018 already
  names the absence of a third-party viewer for that format as a known gap.
- Two kinds of program event are invisible today regardless of format: a cycle-collector stop-the-world pass
  (already an accepted mechanism, firing from the safepoint poll per the project-start architecture decision)
  and an isolate boundary (`spawn` / `spawn worker` / `spawn script`, [ADR 0006](0006-isolated-script-execution.md)).
  Both currently distort the numbers ADR 0018 already reports: a GC pause that happens to run while some
  function is executing gets counted as that function's own self time, and a spawn's total wall time gives no
  way to tell real child work from scheduling overhead.
- Checked whether isolate-spawn tracing could ride ADR 0018's existing "every call site" probe for free, the
  way this amendment's approach would ideally cost nothing new to wire up: [ADR 0006](0006-isolated-script-execution.md)
  rules this out explicitly — it rejects modeling `spawn script` as a function call in so many words ("...
  which is exactly what it is not"), so spawn does not flow through `emit_call()` and needs its own
  instrumentation point.

## Investigation

- **Cycle collector.** Already fires from the existing safepoint poll (a per-task flag check at every loop
  back-edge and function entry, accepted from the first backend commit). The poll itself needs no change —
  only the collector's own run routine, invoked rarely, gets a debug-flag check at its start and end. That
  routine is already the slow path (a stop-the-world pass), so adding one more branch and a timestamp pair
  inside it costs nothing relative to what it already costs to run.
- **Isolate spawn.** Exactly three fixed forms exist — `spawn`, `spawn worker`, `spawn script` — each already
  funneling through one dedicated runtime routine per ADR 0006 (its own arena, its own task stack, a
  capability-narrowing check, `ScriptResult` construction on return). Instrumenting those three routines once
  is a fixed, small change — not a new probe type scattered through codegen the way ADR 0018's per-statement
  and per-call checks are.
- **Net assessment:** this spends nothing on the hot path ADR 0018 already measures and plans to guard with a
  `perf_guards.rs`-style test at M3. It adds instrumentation to O(1) already-rare runtime routines, not O(statements)
  or O(calls) — a materially different, and much smaller, cost than ADR 0018's own per-statement/per-call
  additions.

## Decision

### 1. Trace events carry a `kind` tag

`call | gc | spawn | query`. Existing `call`-kind events are unchanged in shape from ADR 0018 (callee name,
args, entry/exit timestamp, checked-return status, result). The `query` kind is emitted from inside
`Core\Db`'s own statement routines and carries duration, driver, connection name, truncated SQL text, rows
returned and rows affected — **never a bound parameter value**, since a trace is a `secret` sink
([ADR 0067](0067-core-db.md) § 11, [ADR 0033](0033-secret-qualifier-for-confidential-values.md)). Like `gc`
and `spawn` it sits in a routine that is already slow, so it adds nothing to the hot path.

### 2. GC-pause events (`kind: gc`)

Emitted from inside the cycle collector's own run routine — never from the safepoint poll itself — gated by
the same `TRACE`/`PROFILE` bits already in `Ctx`. Recorded: start timestamp, duration, objects freed. Effect:
a GC pause becomes its own event instead of being silently absorbed into whichever function's self time was
running, so `PROFILE`'s self/inclusive attribution stops being distorted by collection pauses it did not
cause.

### 3. Isolate spawn/join events (`kind: spawn`)

Emitted at each of the three spawn-construct runtime routines and at their join/result point, gated by the
same bits. Recorded: start timestamp, spawn kind (`spawn` / `spawn worker` / `spawn script`), join timestamp,
and a computed overhead split — parent-observed wall time minus the child-reported wall time already arriving
on `ScriptResult` (ADR 0018's existing rule for how a child's own coverage/trace/profile data crosses back) —
giving "real child compute" vs. "isolate scheduling/copy-out cost" as two separate numbers instead of one
opaque total.

The child's own trace/profile stream is **not** merged live into the parent's during the spawn — that would
cross the arena boundary [ADR 0006](0006-isolated-script-execution.md)'s isolation model exists to prevent,
the identical reasoning ADR 0018 already gives for not merging a child's coverage data live. Stitching a
child's events into one visual timeline under its parent's `spawn` event, anchored at the spawn's timestamp,
is a **tooling-side, export-time** operation (in the CLI's exporter) over data that already crosses the
boundary exactly as ADR 0018 defined — not a new live cross-arena mechanism.

### 4. A speedscope-evented export, alongside the existing three

A new export renders recorded `call`/`gc`/`spawn`/`query` events as speedscope's evented-profile JSON — open/close
pairs at a timestamp, which is exactly what a `TRACE`-flagged run already produces. This reuses the identical
open format [ADR 0040](0040-vscode-deep-tooling-and-resilient-parsing.md)/M10's sampling profiler already
commits to, rather than inventing a second timeline format with its own bespoke viewer to build and maintain.
Callgrind (aggregate profile), Clover/lcov (coverage) and MWL-native NDJSON (raw trace) are unchanged from
ADR 0018 — this is an additional export, not a replacement. The exact CLI flag spelling is left to whoever
implements M10's exporters (see *Revisiting*), the same way ADR 0018 already left exact Clover/lcov shape to
implementation.

## Consequences

**Positive**

- No new probe site on the per-statement/per-call hot path — ADR 0018's own cost class, and its still-pending
  `perf_guards.rs`-style guard test, are unaffected by this amendment.
- GC pauses and isolate-spawn overhead become visible and correctly attributed instead of silently distorting
  the self time of whatever function happened to be running — the concrete gap this amendment closes.
- Reuses an already-open, already-committed viewer format (speedscope) instead of building a bespoke one,
  consistent with ADR 0040's own reasoning for the sampling profiler.
- Isolate-boundary data crossing is untouched: this amendment reads exactly the data ADR 0018/0006 already
  place on `ScriptResult`, and does timeline-stitching at export time, not via a new live cross-arena
  mechanism.

**Negative**

- The trace event schema now has four kinds instead of one; every exporter (the NDJSON writer, the new
  speedscope-evented writer) has to handle all four — a small, ongoing surface that grows further if a
  future primitive needs its own kind.
- Two more low-cardinality runtime routines (the cycle collector's run function; the spawn/join routines) now
  carry a debug-flag check each, to be kept working as those routines evolve — a bounded, named cost, not a
  hidden one.
- The MWL-native NDJSON trace format remains the ecosystem gap ADR 0018 already named (no third-party tool
  reads it directly); the new speedscope export mitigates this for visualization specifically, but does not
  close the gap for other tooling that might want to consume MWL's raw trace.

## Alternatives rejected

- **Hooking GC events into the safepoint poll itself, rather than the collector's run routine.** Rejected:
  the poll is checked on every loop back-edge and function entry — precisely the hot path this amendment is
  trying not to touch — while the actual collection run is already the rare, slow path where a check costs
  nothing extra.
- **Merging a child isolate's trace/profile stream live into the parent's at spawn/join time.** Rejected: it
  crosses the arena boundary ADR 0006's isolation model exists to prevent, the identical reasoning ADR 0018
  already used for not merging a child's coverage data live.
- **A bespoke MWL timeline-viewer webview instead of speedscope's evented format.** Rejected per ADR 0040's
  own reasoning: an open, already-maintained viewer exists, and MWL is already committed to it for the
  sampling profiler; building a second one is unnecessary scope against the simplicity priority.
- **Coroutine suspend/resume as a further event kind, an external/live attach mechanism, and a memory/
  allocation timeline** — all considered in the same discussion this amendment came out of, and explicitly
  deferred rather than folded in; see *Revisiting*.

## Revisiting

- **Coroutine suspend/resume as a trace event kind**, if a debugging session needs to see scheduling gaps
  between suspension and resume — the same low-cost-instrumentation shape as this amendment's GC/spawn
  events, deferred only because no concrete need has been named yet.
- **An external/live attach mechanism** (starting or stopping tracing on an already-running process from
  outside it, without the request's own code calling `Core\Debug`) was raised in the same discussion and
  deliberately excluded from this amendment: it is a new trust-boundary/attack-surface question against
  priority 1 (a network-reachable channel that can read call arguments and timing from a running process),
  not a formatting or instrumentation-site question, and would need its own ADR with an explicit threat model
  if ever pursued.
- **A memory/allocation timeline** (heap snapshots, allocation events) — no probe mechanism exists for this
  today; a separate, larger design question, not addressed here.
- **The exact CLI flag spelling for the speedscope export** is left open, per *Decision § 4*.

Verification, in the order it becomes possible:

- **M5** (Concurrency and script isolates, where the `Isolate` type and its spawn/join routines are built):
  each of the three spawn-construct runtime routines produces a `spawn`-kind event with a correct
  parent/child overhead split once the `TRACE`/`PROFILE` bits from ADR 0018 exist alongside it.
- **Whenever the mark-sweep cycle collector itself is implemented** (named in the plan's Architecture section
  as an accepted design, not yet pinned to a specific milestone's deliverables): the collector's run routine
  produces a `gc`-kind event with a correct freed-object count and duration, in the same change that builds
  the collector.
- **M10** (alongside ADR 0018's `Core\Debug`/exporters and ADR 0040's sampling profiler): the speedscope
  export produces a file speedscope.app opens showing calls, GC pauses and spawn/join boundaries on one
  timeline, with a spawned child's events visible nested under its parent's `spawn` event; the per-statement/
  per-call debug-flag-off cost guard test already committed in ADR 0018's own *Revisiting* shows no regression
  attributable to this amendment, since no site was added to that path.
