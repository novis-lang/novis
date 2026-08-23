# ADR 0020 — Fatal errors reach user code through a reserved-budget ladder, never through `catch`

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** what happens after a `FATAL` status ([ADR 0002](0002-error-propagation.md)) or an uncaught
  `THROWN` reaches an isolate/request root; what happens when a script or the entry file itself fails to
  compile; the guarantee that every one of those is logged somewhere, in one shared format, no matter how
  many of the handlers in between also fail
- **Amended by:** [0033](0033-secret-qualifier-for-confidential-values.md) — § 6's `Core\Log::write()`
  keeps its open `fields: array<string, mixed>` parameter, but `mwl check` now inspects that call site's own
  argument expressions and refuses a statically-`secret` operand, since this parameter's looseness was never
  designed to need refusing anything until that ADR gave it a reason to.
- **Relates to:** [0002](0002-error-propagation.md) (this ADR does not touch the checked-return ABI or the
  `OK`/`THROWN`/`FATAL` statuses — it defines what consumes a `FATAL` once it reaches the boundary that ADR
  already says it unwinds to), [0005](0005-config-changeability.md) (new directives and their changeability
  class), [0006](0006-isolated-script-execution.md) (the tier-3 handler below *is* a `spawn script` isolate,
  reusing its mechanism, with one deliberate, narrow exception to its budget rule), [0007](0007-explicit-type-system.md)
  (a resource-limit report is a plain value, not a `Throwable`, and that is enforced by the type checker, not
  by convention), [0011](0011-functions-and-constants-are-class-members.md) (`Core\Fatal` and `Core\Log` join
  the domain-class roster), [0012](0012-no-superglobals.md) (the handler script gets an explicit argument,
  never ambient request state)

> **In short:** nothing MWL runs is ever silently dropped, but not everything is *caught* — those are
> different guarantees, and conflating them is what this ADR avoids. `FATAL` stays exactly what
> [ADR 0002](0002-error-propagation.md) already fixed: uncatchable by an ordinary `catch`, because a
> resource-limit report is not a `Throwable` at all — the type checker refuses `catch (Throwable $e)` from
> ever seeing one. What changes is what happens *after* it reaches the boundary: a four-tier escalation
> ladder, each tier getting a bounded, non-repeating chance to handle the failure before falling to the next.
> Tier 1 and 2 are optional, request-local, user-registered hooks. Tier 3 is an operator-configured `.mwl`
> script — an ordinary `spawn script` isolate, reused wholesale — that lets every error, including internal
> panics and compile failures, be formatted and routed by application code in the language it is already
> written in. Tier 4 is a hardcoded, script-free floor in the engine itself, so there is always a last line
> written even if every tier above it fails. Every tier that writes a log line writes the *same* structured
> JSON-Lines record, through one native serialiser two callers share — application code and the engine floor
> never produce logs that disagree in shape.

## Context

- [ADR 0002](0002-error-propagation.md) fixed `OK`/`THROWN`/`FATAL` and that `FATAL` unwinds to the request
  boundary — tested and not reopened here — but never said what consumes it *at* that boundary, beyond
  [ADR 0006](0006-isolated-script-execution.md)'s narrow answer for a spawned child (`ScriptResult->error`).
- Left open: the **root** isolate has no parent to read a `ScriptResult`; an uncaught `THROWN` has no
  equivalent of PHP's `set_exception_handler`; the **entry file** failing to **compile** has no running
  frame to `catch` into (unlike a mid-execution `include`/`spawn script` failure); an **internal runtime
  panic** is materially different from a resource limit — the runtime's own state, not just the script's, is
  untrustworthy; and, cutting across all of these, **what happens when the reporter itself fails** (a
  logging call that allocates under the exact OOM it is reporting).
- Requirement driving the ADR: every error category reaches a log line, user code gets as much safe chance
  to react as possible, and no handler ever gets a second attempt at the same failure — a second attempt is
  how "catch and log" becomes an infinite loop.

## Decision

**A `FATAL` or an uncaught `THROWN` escalates through up to four tiers, each with a fixed, non-repeating
budget, before falling to the next. No tier is retried. Every tier that writes a log line writes the same
record, through one shared native serialiser.**

### 0. The exception hierarchy, and one deliberate non-member

**The hierarchy itself is [docs/spec/01-core-library.md](../spec/01-core-library.md) § 10's, not this
ADR's** — that section is the stdlib work this ADR deferred, and it is the one home for the class tree, the
`message`/`previous`/`backtrace`/`location` members, and the fact that `Exception` and `Error` are not
class names in MWL at all. What matters *here* is only that these classes are global, are the ordinary
`catch` target for ordinary control flow, and include a **`ParseError`**, thrown when a file pulled in
mid-execution (`require`, or a `spawn script` target that fails *after* its parent isolate is already
running) fails to compile — an ordinary, catchable `Throwable` at the call site, matching PHP's own
`ParseError` and the frameworks that catch it to fail a bad template or plugin gracefully instead of the
whole request.

One deliberate non-member: the value a resource-limit `FATAL` carries (illustrative name `Core\Fatal\LimitReport`
— exact shape is stdlib work, see *Revisiting*) **does not implement `Throwable`.** This is not a runtime
check that could be forgotten at one call site and not another — it is a type-checker fact
([ADR 0007](0007-explicit-type-system.md)): `catch (Throwable $e)` around code that hits a memory or CPU
limit provably cannot catch the report, the same way `int + uint` provably cannot compile. Enforcing "FATAL
is not catchable" in the type system, rather than in runtime discipline, is what keeps the ABI-level
guarantee ADR 0002 already tested from depending on every future `catch` site getting a special case right.

### 1. `Core\Fatal::onLimit(closure(LimitReport): void $handler): void`

Fires **only** for a resource-limit `FATAL` — memory, CPU time, wall time, `max_script_depth`. Request-local
registration, living next to the pending-error slot already in `Ctx` per [ADR 0002](0002-error-propagation.md) —
not global, not ambient, dies with the request like every other per-request slot
([ADR 0008](0008-static-and-global.md), [ADR 0012](0012-no-superglobals.md)).

It runs with a **reserved slice** of the request's own budget, carved out at request start and unavailable
to ordinary execution — new `System`-class `mwl.ini` directives, illustrative names
`[limits] fatal_reserve_memory` / `fatal_reserve_time`. `System`, not `Runtime`: this is the request's own
safety net, and a script choosing its own net's size is exactly the case where the choice most needs to be
made by someone other than the code that might be about to need it.

**Zero retries.** If the handler itself throws, panics, or exceeds its own reserved slice, it is abandoned
immediately — no second call, straight to tier 3.

Internal-runtime-panic `FATAL`s **never reach this tier**, per the split below.

### 2. `Core\Fatal::onUncaughtThrow(closure(Throwable): void $handler): void`

Fires when an ordinary `THROWN` propagates through every frame uncaught and reaches the isolate/request
root. Unlike tier 1, this needs no special reserve — execution was healthy up to this point, so the request's
ordinary remaining budget applies — and it gets the **real `Throwable` object**, not copied data: this is the
root itself, not a boundary [ADR 0006](0006-isolated-script-execution.md) has to copy across.

Same zero-retry rule: a handler that itself faults falls straight to tier 3.

### 3. The configured `.mwl` handler — an ordinary `spawn script` isolate, with one narrow exception

A new `System`-class directive, illustrative name `[log] handler = 'path/to/handler.mwl'`. When set, it is
invoked as a **`spawn script` isolate** — the exact mechanism [ADR 0006](0006-isolated-script-execution.md)
already defines, fresh arena, fresh globals, its own config overlay, sharing nothing but compiled code —
receiving one explicit argument (illustrative shape `Core\Fatal\ErrorReport`: kind, message, request id,
timestamp, and whatever structured detail that failure kind carries) through `Core\Script::args()`, exactly
as any spawn-script target does. It must not, and structurally cannot reliably, use ambient
`Core\Request`/`Core\Server`/`Core\Session`: for a compile failure, no request context may exist yet; for a
panic, the runtime's own state is exactly what is in question. This is not a new restriction —
[ADR 0012](0012-no-superglobals.md) already makes those accessors throw inside any spawned isolate — it is
that restriction applying somewhere it matters more than usual.

**The one deliberate exception to ADR 0006:** that ADR fixes that "an isolate does not receive a budget of
its own... it spends its parent's" — correct for an ordinary child, and exactly wrong here, since a request
already at its ceiling has nothing left to give this isolate, and the whole point of tier 3 is to still run
when that is true. Tier 3's isolate is instead charged to a small, fixed, **engine-owned** allotment —
illustrative directives `[log] handler_reserve_memory` / `handler_reserve_time`, sized once per worker/core
at boot, not per request. This is a narrow, named exception for exactly this purpose, not a precedent: no
other isolate gets a budget independent of the tree it spends from, and this ADR does not open that door
generally.

This tier is the shared catch-all, firing for:

- any tier-1 or tier-2 failure (handler faulted, or none was registered at all — the same way PHP's
  `error_log` fires whether or not `set_exception_handler` was ever called);
- an internal-runtime-panic `FATAL`, **unconditionally** — it never reaches tier 1 or 2 (see below);
- an **entry-script compile failure** — there was never a frame for tiers 1/2 to have been registered from.

**Zero retries**, same as every tier above: a throw, a panic, or exceeding its own reserved allotment inside
this script drops straight to tier 4, with no second invocation.

### 4. The engine-native floor

Hardcoded in Rust. No script execution — nothing left here that could itself throw, panic, or need a further
tier, because this is the floor. Writes to an operator-owned, `System`-class sink (illustrative directive
`[log] target = stderr | file:<path> | syslog`). Fires when `[log] handler` is unset, or tier 3 fails for any
reason. If even this write fails — a full disk, a broken pipe — the failure is swallowed: there is nothing
further to escalate to, and the request or isolate still tears down normally regardless. That is a boring,
explicit answer on purpose: an undefined "what then" at the true floor is worse than a defined "give up."

### 5. Internal panics bypass user code entirely

A resource-limit `FATAL` means the *script* asked for too much — the runtime's own state is fine, and a
request-level handler reacting to it (log with context, decide whether the job should be retried) is
meaningful. An internal-runtime-panic `FATAL` means something in MWL's own Rust code broke its own invariant
— the runtime's state at that point is exactly what cannot be trusted, and running more script code on top
of it is the riskier move, not the safer one. So: **only a resource-limit `FATAL` reaches
`Core\Fatal::onLimit`.** An internal panic goes straight to tier 3 (still user-formattable for ops, still
routed to Loki/Sentry/wherever) without ever calling back into the failing request's own code.

### 6. `Core\Log`: one write path, two callers

Ordinary application code and the tier-3 handler script call the same API — illustrative signature
`Core\Log::write(string $level, string $msg, array<string, mixed> $fields = []): void` — implemented as a
thin binding over the **exact same native serialise-and-write helper** tier 4 calls directly when it has no
script to run at all. One implementation, two callers, the same shape
[ADR 0006](0006-isolated-script-execution.md) already uses for its own isolation code ("one isolation
implementation, not two") — so a Loki dashboard never has to reconcile two log shapes depending on which
tier happened to produce a given line.

The shared record is **JSON Lines**: one JSON object per line — `ts` (RFC3339), `level`, `msg`,
`request_id`, and a `fields` object carrying whatever structured context that call site has (error class,
limit name, a stack summary). Chosen over `logfmt`: an arbitrary error message or a multi-line stack trace
needs escaping that is correct on the first and only attempt at the true floor, and JSON's escaping is a
solved, mechanical problem where `logfmt`'s quoting of embedded quotes/newlines/spaces is not — exactly the
kind of judgment call this ADR does not want resting on tier 4's one shot.

### 7. Compile errors, restated against the ladder above

- **Entry file, at request/isolate start.** No frame ever existed, so tiers 1/2 never had the chance to
  register anything. Reported the same way any `FATAL`-class condition with no registered handler is:
  straight to tier 3 (if configured) then tier 4. `mwl.ini` (not the never-started script) is what decides
  whether an HTTP response shows a generic page or detail — the request had no code path in which it could
  have decided differently.
- **Mid-execution**, via `include`/`require` or a `spawn script` target failing after its parent is already
  running: an ordinary `ParseError` (§ 0), catchable at the call site like any `Throwable`. Uncaught, it rides
  the normal tier-2 path — by then a frame did exist.

## Consequences

**Positive**

- Every failure category — resource limit, uncaught exception, internal panic, entry compile failure,
  mid-execution compile failure — has a defined path ending in a log line, which is the requirement this ADR
  exists to satisfy. None of them can end up silently dropped.
- No catch-loop is possible **by construction**, not by discipline: zero retries at every tier, and a
  resource-limit report is not a `Throwable` at the type level, so "catch a FATAL, handler throws, catch
  that too" cannot even be written for tier 1's case, let alone loop.
- Reuses two already-decided mechanisms wholesale — the checked-return `FATAL` status
  ([ADR 0002](0002-error-propagation.md)) and `spawn script` isolation
  ([ADR 0006](0006-isolated-script-execution.md)) — rather than inventing new ones. One new, narrow rule (the
  engine-owned budget for tier 3) is the only genuinely new mechanism.
- Operators get one place, in the language the application already runs, to format and route every error
  category to their observability stack (Loki, Sentry, anything that reads JSON Lines) — no second,
  host-side configuration surface and no second format to reconcile against `Core\Log`'s.
- `Core\Log` and the engine floor sharing one serialiser means a Loki query never needs a special case for
  "lines the engine wrote itself" versus "lines the application wrote."

**Negative**

- One narrow, explicitly-named exception to [ADR 0006](0006-isolated-script-execution.md)'s "an isolate
  spends its parent's budget" rule. Flagged here so it is read as a deliberate carve-out for exactly this
  purpose, not a precedent for isolates getting independent budgets generally.
- New `mwl.ini` surface: `fatal_reserve_memory`/`fatal_reserve_time`, `[log] handler`,
  `handler_reserve_memory`/`handler_reserve_time`, `[log] target` — a cost against priority 4 (simplicity),
  accepted because the alternative is either an unloggable OOM or a log format that drifts between the engine
  and userland. As [ADR 0004](0004-memory-for-simplicity.md) requires stated: both reserves are small and
  fixed — the tier-1 slice is carved from the *request's own* ceiling at request start, unavailable to
  ordinary execution; the tier-3 allotment is a *worker-level* allocation, sized once per core, not per
  request, so it does not scale with request volume. Exact defaults are M6 config work (see *Revisiting*).
- The tier-3 handler script is one more place `Core\Request`/`Core\Server`/`Core\Session` are unavailable —
  a real thing a handler author has to learn, mitigated by receiving an explicit typed argument instead of
  ambient state, which is [ADR 0012](0012-no-superglobals.md)'s existing rule applying somewhere it matters
  more than usual, not a new kind of restriction.
- Four tiers plus a non-`Throwable` report type is more surface than "just let people catch everything" would
  have been. Accepted per the answers this ADR was built from: reopening ADR 0002 to make `FATAL` catchable
  everywhere was considered and declined precisely because it reintroduces the loop risk at every stack
  depth instead of containing it to one boundary.

## Alternatives rejected

- **Let an ordinary `catch (Throwable)` intercept a `FATAL` anywhere in the call stack.** Reopens
  [ADR 0002](0002-error-propagation.md)'s status model and spreads catch-loop risk to every stack depth.
- **Bounded-N retries** on a failing handler. Just adds a knob to size, for a case that should already be
  rare by tier 3.
- **One handler API for both resource-limit and internal-panic `FATAL`s.** Would rerun user code atop
  runtime state the runtime itself does not trust.
- **Treat every compile failure as boundary-only**, including mid-execution ones. Diverges from PHP's
  catchable `ParseError`.
- **`logfmt` as the shared record format.** Escaping an arbitrary message or multi-line stack trace burdens
  tier 4's one unretried attempt more than JSON's mechanical escaping.
- **A purely engine-native logger, no operator-configurable script tier.** Forces error routing/formatting
  onto a second, host-side configuration surface instead of the language already available.
- **Give the tier-3 handler its own budget drawn from the request tree it is reporting on.** A request
  already at its ceiling has nothing left to give, making the feature unreachable when needed most.

## Revisiting

- **Exact directive names and default sizes** (`fatal_reserve_memory`/`fatal_reserve_time`,
  `[log] handler`, `handler_reserve_memory`/`handler_reserve_time`, `[log] target`) and the
  `Core\Fatal`/`Core\Log`/`LimitReport`/`ErrorReport` class shapes are M6 (limits) and M8 (stdlib) work,
  following the same "ADR fixes semantics, spec/implementation fixes spelling" split
  [ADR 0006](0006-isolated-script-execution.md) used for `spawn script`'s own grammar.
- **Whether internal panics should ever reach a user-registered handler** could be reopened if operators need
  panic-specific application-level alerting badly enough to accept running code atop uncertain runtime
  state — tier 3/4 already log every panic regardless, so reopening this is about whether tier 1/2 ever see
  one, not about whether it is reported at all.
- **How the tier-3 handler script's compiled unit interacts with hot-reload** ([ADR 0017](0017-hot-reload-without-restart.md))
  — pinned at boot, or revalidated like any other path — is an M8 mechanism question this ADR flags but does
  not resolve.

Verification, in the order it becomes possible:

- **M6**, when limits and reserves land: a resource-limit `FATAL` with no `onLimit` registered still reaches
  tier 3/4; a script attempting to widen `fatal_reserve_memory` fails as a `System`-class set; a `catch
  (Throwable)` around code that hits a limit does not catch it — a compile-time-checkable claim, not a
  runtime one; an `onLimit` handler that itself throws falls straight to tier 3 with no second call.
- **M7/M8**, when the HTTP server and `Core` domain classes land: `onUncaughtThrow` receives the real
  `Throwable` with a full trace; the tier-3 isolate runs charged to the engine's own allotment and still
  fires when the reporting request is at its own memory ceiling; `Core\Log`'s output from application code
  and tier 4's native fallback produce schema-identical JSON Lines records for the same error; an internal
  panic never reaches `onLimit` or `onUncaughtThrow` in any test that provokes one.
- **Whenever M1's front-end verification exists to build on:** an entry-script compile failure reaches
  tier 3/4 with tiers 1/2 never invoked, and never brings down the server process.
