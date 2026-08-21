# ADR 0018 — Coverage, tracing and profiling are safepoint-shaped probes, not a second compiled tier

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** `Core\Debug`, the `[debug]` `mwl.ini` section, the `debug.trace`/`debug.profile` capabilities,
  the probe-emission points added to `mwl-codegen`, the `Ctx` fields that back them, and the coverage/trace/
  profile output formats
- **Relates to:** [0002](0002-error-propagation.md) (`emit_call()` is the one place a trace/profile probe
  attaches to a call), [0004](0004-memory-for-simplicity.md) (what a live debug session spends, and that it
  must stay attributable to a request), [0005](0005-config-changeability.md) (`[debug] mode` uses the
  existing three-class directive model, no new mechanism), [0006](0006-isolated-script-execution.md) (fills
  in the still-provisional `ScriptResult` shape with a `coverage`/`trace`/`profile` field, crossing by the
  same copy-out rule `value`/`error`/`usage` already use — not an amendment, since 0006 left that surface
  provisional pending M5), [0016](0016-ide-integration.md) (`mwl dap` and the M10 sampling profiler are a
  different, already-decided mechanism this one is designed to sit beside, not replace)

> **In short:** MWL gets first-class, Xdebug-equivalent code coverage, function-call tracing and a
> deterministic per-call profiler — enabled with one `mwl.ini` directive or one `Core\Debug` call, exported
> as Clover/lcov (coverage) and Callgrind (profiling) so existing PHP-ecosystem tooling reads MWL's output
> with no new client. The mechanism is **not** a second, instrumented compiled tier: it is a small,
> always-present, per-request debug-flags check at each probe site — the same shape as the safepoint poll
> already committed to for CPU limits, cancellation and the cycle collector — because coverage and tracing
> must be start/stoppable *mid-request* (a PHPUnit-style harness starting coverage around one test inside a
> long-lived process), which only a runtime-checked flag can do; a compiled-in-advance instrumented tier
> cannot, since the request would already be running the wrong one before it knew to ask.

## Context

PHP developers reach for Xdebug for four things: step debugging, code coverage (which lines a test suite
actually exercised, feeding CI coverage gates), call tracing (what was called, with what arguments, how
long it took), and profiling (a call-graph with self/inclusive time, usually viewed in KCachegrind or
Webgrind). Step debugging is already decided — [ADR 0016](0016-ide-integration.md) commits `mwl dap` to
using safepoints for breakpoints, with editor UI wiring deferred but the adapter itself in M10 — and the
project-start section of [the ADR index](README.md) already names "debugger breakpoints" as one of five
jobs the safepoint poll does. Coverage, tracing and profiling are not yet decided, and they are exactly the
kind of decision [CLAUDE.md](../../CLAUDE.md) wants settled before codegen exists: retrofitting instrumentation
points onto ten milestones' worth of already-written statement lowering is the same shape of expensive
mistake the safepoint paragraph already warns about for a different mechanism.

The requirement asks for more than porting Xdebug's feature list: **"the whole language should be very
convenient for developers to use and to debug, so entry barrier is as low as possible."** That constrains
the design as much as the mechanism does — an MWL-native format nobody's tooling reads would satisfy the
letter and fail the point.

Two things make this harder than it looks given what MWL has already decided:

1. **The baseline tier is not an interpreter loop.** M3's plan already commits to typed scalar operations
   lowering to native Cranelift instructions, falling back to a helper call only for `mixed`, unions and
   dynamic calls (`docs/implementation-plan.md`, M3). So "hook the helper call every operation already goes
   through" — the obvious move for an Xdebug-style design, since PHP's own tracing/coverage hooks live in
   `zend_execute`'s per-opcode dispatch — is not available. There is no single dispatch point every statement
   passes through; there is Cranelift-generated native code with different shapes depending on the operand
   types.
2. **Coverage and tracing must be start/stoppable inside one request's execution**, not just once at request
   start. `xdebug_start_code_coverage()` / `xdebug_stop_code_coverage()` bracketing individual test cases
   inside one long-running PHPUnit process is the dominant real-world usage pattern, and the requirement's
   "low barrier to entry" goal means MWL should support the same pattern, not a coarser "coverage is a
   request-level on/off switch set before the request begins" version of it.

## Investigation

**Why not a second, instrumented compiled tier, selected per request** (the design this ADR's author
initially reached for, extending [ADR 0017](0017-hot-reload-without-restart.md)'s content-addressed cache
with a `tier` dimension so a debugged request resolves `UnitKey { path, content_hash, tier: Instrumented }`
instead of `Optimized`). This looked attractive — zero cost on the untouched tier, reuses the existing
single-flight/write-once cache machinery, no new subsystem. It fails on point 2 above: by the time a
request's code has been resolved and is running, its compiled tier is fixed. A test harness calling
`Core\Debug::startCoverage()` mid-request would need the *already-running* frames to have been running the
instrumented tier all along — which means either compiling every unit twice from the start of every request
regardless of whether coverage is ever requested (paying the cost priority 3 exists to avoid, for the common
case that never asks), or accepting that coverage silently only covers code entered *after* the start call,
which is a correctness gap Xdebug does not have and PHPUnit's own usage pattern would immediately expose.
Two compiled tiers is a design for "on or off for the whole request, decided in advance" — that is not this
requirement.

**Why the safepoint shape instead.** A safepoint is already exactly the primitive this needs: a cheap poll
— load a flag, branch — inserted at fixed points by codegen, present unconditionally, doing real work only
when the flag says to. It was chosen for CPU limits and cancellation for the identical reason it fits here:
those, too, must be enabled for a request already in flight (a client can disconnect mid-request), and the
project already accepted the "always emit the check, gate the work" cost for that mechanism rather than
inventing a "cancellable" and "non-cancellable" compiled tier. Reusing the shape rather than the safepoint
*mechanism itself* — a new set of probe sites, not overloading the loop-back-edge/function-entry poll — is
necessary because coverage and branch counting need finer granularity than safepoints intentionally have:
a safepoint per statement would defeat the reason safepoints are sparse (loop back-edges and function entry
only) in the first place.

**Why Clover/lcov/Callgrind rather than an MWL-native format.** The requirement's low-barrier-to-entry goal
is best served by *not* asking a developer to learn a new format or write a new importer. Clover XML is
what PHPUnit already emits and what every CI coverage dashboard already parses; lcov is the format
everything outside the PHP ecosystem expects; Callgrind's format is what KCachegrind, QCachegrind and
Webgrind already visualise, unmodified. A trace log has no equivalent ecosystem-standard third-party
consumer — Xdebug's own trace format is mostly read by Xdebug's own tooling — so MWL's trace output is
newline-delimited JSON, one object per call, and stays open to gaining an Xdebug-compatible mode later if
migration tooling wants one; that option is named, not silently foreclosed (see *Revisiting*).

## Decision

### One mutable per-request debug-flags word, checked at fixed probe sites

`Ctx` (the same struct carrying the pending-error slot from [ADR 0002](0002-error-propagation.md)) gains a
small bitset — `COVERAGE | BRANCH | TRACE | PROFILE` — plus the request-owned sinks each flag writes to.
Codegen emits an unconditional check against this word, load-and-branch, at three fixed points, present in
every compiled unit from the first backend commit regardless of whether any request ever sets a bit:

1. **Every statement boundary** (the head of each lowered HIR statement): under `COVERAGE`, bump a per-line
   hit counter. Under `BRANCH`, do the same at every conditional CFG edge, using the edge identity the
   CFG/SSA IR already carries.
2. **Every call site** — the single `emit_call()` path [ADR 0002](0002-error-propagation.md) already
   requires: under `TRACE`, emit an entry probe (callee name, arguments if requested, a timestamp) before
   the call and an exit probe (the checked-return status, the result, a timestamp) after it — the same
   status value the call site already branches on, so a trace log records a thrown or `FATAL` exit exactly
   as it happened, not as a reconstruction. Under `PROFILE`, the entry/exit pair instead accumulates
   self/inclusive wall-clock time per callee into a call-graph structure, keyed the way Callgrind's own
   format wants it.
3. **Nothing else.** No probe at expression granularity, no probe inside a native scalar instruction
   sequence — coverage and tracing operate at the statement/call granularity Xdebug itself reports at, which
   is also the granularity that keeps the added check count bounded by source size rather than by how many
   native instructions a statement happens to lower to.

Turning every bit off costs one cached load and one predicted-not-taken branch per site, the same cost class
already accepted for the safepoint poll ([the ADR index](README.md), project-start section) — not a new
kind of cost, a second application of one already paid for. Turning a bit on for a running request is
exactly setting the word; no recompilation, no re-resolution, no second `UnitKey`. This is what makes
start/stop mid-request ("wrap coverage around one test") work at all, which the rejected tiered-compilation
design in *Investigation* could not do.

### Coverage and branch data live in the request's own arena; trace and profile data stream to a sink

Line and branch hit-counters are small — bounded by the size of the code a request touches, not by how many
times a loop iterates — so they live as an ordinary per-request structure in the request's arena, dropped
wholesale at request end like everything else ([ADR 0004](0004-memory-for-simplicity.md)'s O(in-flight)
rule, satisfied for free by reusing the existing arena lifetime rather than adding a new one). They are
read back mid-request or at request end through `Core\Debug::getCoverage()`.

Trace and profile data are call-count-proportional and can be large for a long-running request, so they
default to **streaming to a sink** named when tracing/profiling starts, rather than buffering the whole run
in the arena — the same reasoning Xdebug's own trace-to-disk behaviour already reflects, and the same
"footprint vs. traffic" distinction [ADR 0004](0004-memory-for-simplicity.md) draws: an unbounded in-memory
trace buffer would be a memory question this ADR would have to bound with a new limit directive; a stream
is bounded by nothing this ADR needs to invent, because nothing accumulates.

### `[debug] mode` is `RuntimeTighten`, and writing a trace or profile is its own capability

```ini
[debug]                       ; RuntimeTighten — mwl.ini states the default AND the ceiling in one
mode = off                    ; off | any comma-combination of: coverage, branch, trace, profile

[capabilities]
debug.trace   = /var/log/mwl/trace     ; RuntimeTighten, deny-by-default — same shape as script.spawn
debug.profile = /var/log/mwl/profile
```

`[debug] mode` reuses [ADR 0005](0005-config-changeability.md)'s existing three-class directive model with
no new mechanism: a production `mwl.ini` sets `mode = off` and no request-side call can ever turn any bit
on, because `RuntimeTighten` only narrows; a development or CI host's `mwl.ini` sets a wider ceiling (say
`coverage,branch,trace,profile`) and a specific test run may narrow further via `Core\Debug` or `ini_set`.
This is deliberately the same class capability grants already use, and for the same reason
[ADR 0017](0017-hot-reload-without-restart.md) makes `opcache.validate` `System`-class: whether a running
request's internals are observable is not a request-local decision, because an attacker-controlled request
that could turn tracing on for itself in production would gain a reconnaissance channel over call arguments
and timing that priority 1 does not trade away.

`debug.trace`/`debug.profile` are a **separate** grant from `fs.write`, mirroring
[ADR 0006](0006-isolated-script-execution.md)'s `script.spawn`/`fs.read` split ("being able to read a file is
not permission to run it"): being able to write an ordinary file is not permission to persist a continuous
log of every call's arguments, which can carry request data a coverage-only deployment never needed to grant
access to.

### `Core\Debug` — the same API shape Xdebug developers already know

```php
Core\Debug::startCoverage(bool $branch = false): void;
Core\Debug::stopCoverage(): void;
Core\Debug::getCoverage(): array<string, array<int,int>>;   // path => line => hit count

Core\Debug::startTrace(string $path): void;
Core\Debug::stopTrace(): void;

Core\Debug::startProfiling(string $path): void;             // writes Callgrind format on stop
Core\Debug::stopProfiling(): void;
```

placed under `Core` per [ADR 0011](0011-functions-and-constants-are-class-members.md), deliberately shaped
close to `xdebug_start_code_coverage()`/`xdebug_get_code_coverage()` so a PHPUnit-style coverage collector
ported by `mwl convert` needs its calls renamed, not its logic rewritten — the concrete payoff of "low
barrier to entry" this ADR is answering.

`mwl test`/`mwl run` grow flags that wrap a whole run in these calls with no source changes at all, which is
the ergonomics CI actually uses day to day:

```sh
mwl test --coverage=clover:build/coverage.xml
mwl run script.mwl --profile=out.callgrind
```

### Crossing an isolate boundary

A `spawn script`/`spawn worker` child ([ADR 0006](0006-isolated-script-execution.md)) starts with the
parent's *current* debug-flags word, derived the same way its config overlay already is — "a copy of the
parent's effective config" — and can only narrow it further, the identical rule [ADR 0006](0006-isolated-script-execution.md)
already states for capabilities ("nothing widens... an isolate is a new place that rule applies, not an
exception to it"). The child's own coverage/trace/profile data does not merge into the parent's live state —
that would require the mutable cross-arena sharing the whole architecture exists to avoid — and instead comes
back as data on the result, the same way a child's failure already does: `ScriptResult` gains
`coverage`/`trace`/`profile` fields, populated only when the corresponding flag was active, crossing by the
same copy-out rule `value`/`error`/`usage` already use. [ADR 0006](0006-isolated-script-execution.md) named
its `ScriptResult` shape provisional pending M5; this fills in that detail rather than amending a decided one.

### Relationship to the M10 sampling profiler and `mwl dap`

This ADR's profiler is **deterministic**: it times every call, attributing exact self/inclusive time per
function, the way Xdebug's and Callgrind's profilers do. `docs/implementation-plan.md`'s M10 already commits
to a separate **sampling** profiler emitting flamegraphs — periodic stack samples, not per-call timing —
which stays exactly as planned; the two answer different questions (a low-overhead always-affordable
production sampling view vs. an opt-in exact call graph for a specific debugging session) and this ADR does
not fold one into the other. `mwl dap`'s breakpoints stay on the safepoint poll itself, unrelated to the
per-statement/per-call probes added here — a debugged request may or may not also be collecting coverage or
a trace, and the two mechanisms are independent bits, not tiers of the same thing.

## Consequences

**Positive**

- No new compiled-unit dimension, no new cache key, no new subsystem — one bitset in `Ctx`, checked at sites
  that reuse identity the CFG/SSA IR and the call ABI already carry.
- Start/stop works mid-request, which is the pattern real coverage tooling (PHPUnit-style, per-test
  bracketing) actually uses, and which a compiled-tier design could not support at all.
- Output formats existing tooling already reads: a coverage report opens in the same CI dashboard a PHP
  project already has configured; a profile opens in KCachegrind/Webgrind unmodified. Zero new client
  tooling to write for v1.
- `Core\Debug`'s shape means `mwl convert` has a close-to-mechanical rewrite for `xdebug_*` calls in ported
  test suites, the same payoff [ADR 0006](0006-isolated-script-execution.md) notes for `spawn script`
  against `exec('php …')`.
- Reuses the `System`/`Runtime`/`RuntimeTighten` model wholesale for `[debug] mode`, and the
  capability-grant model wholesale for `debug.trace`/`debug.profile` — no new changeability class, no new
  grant shape, just two more entries in registries [ADR 0005](0005-config-changeability.md) and
  [ADR 0006](0006-isolated-script-execution.md) already defined.

**Negative**

- **More probe sites than the safepoint poll has.** Safepoints sit only at loop back-edges and function
  entry, deliberately sparse; this ADR adds a check at every statement and every call, which is a larger
  surface even though each check is the same cheap shape. The marginal per-statement cost when every flag is
  off must be measured, not assumed — a guard test analogous to
  `a_checked_return_frame_stays_cheap` belongs in `benches/abi-probe` once M3 lands probes, and if it shows a
  real regression the fix is coarsening probe granularity (e.g. one check per basic block rather than per
  statement), not abandoning the flag-check design.
- **Trace and profile output is only as trustworthy as the sink's capability grant.** An operator who grants
  `debug.trace` too broadly has created a channel for request data (call arguments) to reach disk; this is
  the same class of risk `script.spawn`'s path-rooted grant already manages, and is mitigated the same way —
  named explicitly rather than left implicit.
- **A fourth Xdebug-adjacent format (Callgrind) to serialise correctly**, alongside Clover and lcov for
  coverage — three exporters instead of one canonical format, a cost against priority 4. Accepted because
  the alternative is a format the ecosystem's existing tools cannot read, which fails the requirement's own
  "low barrier to entry" test more directly than one extra serialiser costs.
- **The trace format is MWL-native** (newline-delimited JSON), not Xdebug-compatible, because no third-party
  tooling beyond Xdebug's own widely consumes Xdebug's trace format either — named as a considered gap, not
  an oversight.

## Alternatives rejected

- **A second, instrumented compiled tier selected per request**, extending [ADR 0017](0017-hot-reload-without-restart.md)'s
  cache key with a `tier` dimension. See *Investigation* — fails the mid-request start/stop requirement,
  which is the pattern that matters most for the "low barrier to entry" goal.
- **Hooking the baseline tier's helper-call dispatch**, the PHP/Xdebug-shaped design. Not available: M3
  already commits typed scalar operations to native Cranelift instructions with no common dispatch point,
  so there is no single call site every statement passes through to hook.
- **An MWL-native coverage/profile format with a converter tool.** Rejected for the same reason the trace
  format's gap is accepted rather than closed the same way: coverage and profiling *do* have a dominant
  ecosystem-standard consumer (CI dashboards, KCachegrind/Webgrind) that a converter step would sit in front
  of for every single use, which is exactly the friction "low barrier to entry" argues against paying.
- **Making `[debug] mode` a plain `Runtime` directive**, settable to any value by any request up to
  `[limits.hard]`-style ceiling. Rejected on the same reasoning [ADR 0017](0017-hot-reload-without-restart.md)
  gives for `opcache.validate`: whether a request's internals are observable to itself is not a question a
  request should answer for itself in a shared production process.
- **Merging a spawned isolate's coverage/trace data live into its parent's.** Would require mutable state
  crossing the arena boundary, which is the exact thing [ADR 0006](0006-isolated-script-execution.md)'s
  isolation exists to prevent; returning it as data on the `ScriptResult`, which a caller may then merge in
  ordinary code, costs nothing structurally new.

## Revisiting

Reopen if a migration tool genuinely needs to read or write Xdebug's own trace-file format rather than MWL's
newline-delimited JSON — named above as a deliberate gap, not a closed question. Reopen the per-statement
probe granularity if the guard test named in *Consequences* shows a measurable regression once M3's baseline
backend exists; the fix is coarsening the probe site (per basic block, not per statement), not the flag-check
shape itself. Reopen the "coverage/branch data lives in the arena" choice only if a workload needs coverage
data to *outlive* the request that produced it before `Core\Debug::getCoverage()` reads it back — that is a
lifetime question this ADR has not had to answer because coverage is always read back before the arena that
holds it is dropped.

Verification, in the order it becomes possible:

- **M2**, when the CFG/SSA IR lands: every lowered statement and every conditional CFG edge carries a stable
  id a probe can address, threaded through from the same span information diagnostics already need.
- **M3**, alongside the safepoint poll landing with the first backend commit: the debug-flags check compiles
  at every statement boundary and call site with every bit off costing no more than the accepted safepoint
  cost class — a guard test in `benches/abi-probe` holding that number, per *Consequences*.
- **M10**, when `Core\Debug`, the exporters and the CLI flags land alongside `mwl dap` and the sampling
  profiler: `Core\Debug::startCoverage()`/`stopCoverage()` bracketing part of a request produces line hits
  for exactly the statements executed in that window, not the whole request; `mwl test --coverage=clover:…`
  produces a Clover file a real coverage dashboard parses without modification; `mwl run --profile=out.callgrind`
  produces a file KCachegrind/Webgrind opens and attributes time to the right MWL functions; `debug.trace`
  refuses a sink path outside its granted roots, including via `..`, the same test shape
  [ADR 0006](0006-isolated-script-execution.md) already runs for `script.spawn`; a production-shaped
  `mwl.ini` (`[debug] mode = off`) makes every `Core\Debug::start*()` call a no-op regardless of what the
  request's own code asks for.
