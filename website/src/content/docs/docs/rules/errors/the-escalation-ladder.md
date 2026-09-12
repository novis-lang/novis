---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "The escalation ladder"
description: "Four tiers between a failing request and a dead server, none of them retried, ending at a native floor that gives up."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/errors/how-an-error-travels/
  label: "How an error travels"
next:
  link: /docs/rules/errors/diagnostics-and-logging/
  label: "Diagnostics and logging"
---

<p class="nv-section-lead">Four tiers between a failing request and a dead server, none of them retried, ending at a native floor that gives up.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">3</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#on-limit">Tier 1 — a resource limit reaches the request that spent it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-allocation-past-the-ceiling-is-refused-in-front-of-itself">An allocation past the memory ceiling is refused before it is made, and the refusal is a complete no-op</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#escalation-ladder">A failure escalates through four tiers, and no tier is retried</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#on-uncaught-throw">Tier 2 — an uncaught throw reaches the request root as itself</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#handler-script">Tier 3 — the configured handler is an ordinary isolate on the engine's own budget</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#engine-floor">Tier 4 — the floor is native, bounded, and gives up rather than escalating</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#panics-bypass-user-code">An internal panic never runs the failing request's own code</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#compile-failure">A compile failure is catchable mid-execution and reaches the ladder at entry</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="on-limit">

## Tier 1 — a resource limit reaches the request that spent it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#on-limit"><code>errors/on-limit</code></a>
</div>

`Core\Fatal::onLimit(closure(LimitReport): void $handler): void` fires only for a resource-limit
`FATAL` — memory, CPU time, `max_output`, wall time, `max_script_depth` and call-stack depth. An
internal panic never reaches it ([`errors/panics-bypass-user-code`](/docs/rules/errors/the-escalation-ladder/#panics-bypass-user-code "An internal panic never runs the failing request's own code")).

Registration is request-local, living beside the pending-error slot in `Ctx`, and dies with the
request like every other per-request slot. The handler runs on a **reserve carved out of the
request's own budget at request start** and unavailable to ordinary execution — otherwise a request
that exhausted its memory would have nothing left to report with.

There are **two reserves, memory and time, not one per limit**, because those are the only two
resources a handler cannot run without spending; a `max_output` breach refuses the handler nothing.
Sizing them is a `System`-class decision, not the script's: a program choosing the size of its own
safety net is exactly the case where the choice should belong to someone else.

`LimitReport` is an array rather than a class. The report is built where the breach happens, in
`nvs-runtime`, which holds no `Core` class descriptor to instantiate one from — and a keyed array
takes a later field without changing the signature of a handler already written.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The handler is request-local rather than global, and runs on a reserve carved out before execution started</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/errors/how-an-error-travels/#stack-depth" title="Recursion is bounded twice: a catchable error, then a fatal"><code>errors/stack-depth</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#panics-bypass-user-code" title="An internal panic never runs the failing request's own code"><code>errors/panics-bypass-user-code</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/fatal-on-limit-registers-a-handler-without-running-it.nvst"><code>tests/conformance/core/fatal-on-limit-registers-a-handler-without-running-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/fatal-on-uncaught-throw-and-on-limit-are-two-independent-slots.nvst"><code>tests/conformance/core/fatal-on-uncaught-throw-and-on-limit-are-two-independent-slots.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/tests/configured_limits.rs"><code>crates/nvs-runtime/tests/configured_limits.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-allocation-past-the-ceiling-is-refused-in-front-of-itself">

## An allocation past the memory ceiling is refused before it is made, and the refusal is a complete no-op

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#an-allocation-past-the-ceiling-is-refused-in-front-of-itself"><code>errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself</code></a>
</div>

An allocation that would carry a request past `[limits] memory` is refused **before** the block is
taken, and the refusal is a complete no-op. A published flag bounds a *loop* of allocations, because
there is a next one to stop; it cannot bound a single allocation, because there is none. So the
ceiling is asked twice: once behind every growing allocation, where crossing it raises the memory bit
in the word compiled code polls, and once in front of a value allocation, where the size is known and
the caller is ours.

**The refusal is not in the global allocator.** A null there reaches a `Vec`'s or `Box`'s own path
and `handle_alloc_error`, which aborts the process and every request on it — worse than the breach.
It sits one level up, in the allocators that make a Novis `string`, `array` or object, and the
aborting wrappers beside them do not exist: the fallible constructor is the only way to make one.

**A primitive that carries no `ctx` refuses by returning a degenerate value, and acquires no status
channel to say so.** `nvs_str_append` answers its target unchanged, which exactly balances the one
reference it consumes; `nvs_str_concat` and `nvs_str_concat_n` answer the immortal empty string,
whose release is already a no-op; `nvs_array_set` and `nvs_array_set_index` answer their array
unchanged, having released the key and value they were handed. This is sound because **the request is
already dead**: the refusal is recorded and the memory bit published before the value is returned, so
the program runs only to its next poll, and in that window it can build wrong values and compare them
and do nothing else. It can write no output, reach no `Core` member and touch nothing durable,
because each of those passes `run_helper`, which asks the ceiling ahead of the body and reports the
breach instead of running it.

**Complete no-op means the refused write does none of the work of the write.** In particular it does
not separate a shared array: `foreach` walks the snapshot it started on, and a refused write that
separated without writing — or wrote into the shared original — is the one way a refusal reaches a
live cursor's entry expectation. It also holds no partial allocation and leaves no half-grown buffer.

The refusal is sticky for the rest of the request: a request refused once is over, and nothing it
does afterwards brings it back under a ceiling it never held the bytes against. Objects are outside
the pre-check by design — an object's allocation is sized by its class, so no program drives one
unbounded — and they stay bounded by the published flag.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#on-limit" title="Tier 1 — a resource limit reaches the request that spent it"><code>errors/on-limit</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#a-cross-request-stores-bytes-are-its-own-balance" title="A cross-request store's bytes go on a balance of their own, and no request is charged or credited for them"><code>concurrency/a-cross-request-stores-bytes-are-its-own-balance</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0174.md">record 0174</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/one-operation-past-the-ceiling-is-refused-before-it-allocates.nvst"><code>tests/conformance/error/one-operation-past-the-ceiling-is-refused-before-it-allocates.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/a-loop-that-calls-nothing-is-stopped-by-the-memory-ceiling.nvst"><code>tests/conformance/error/a-loop-that-calls-nothing-is-stopped-by-the-memory-ceiling.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/tests/refusal.rs"><code>crates/nvs-runtime/tests/refusal.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/tests/allocator_ceiling.rs"><code>crates/nvs-runtime/tests/allocator_ceiling.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="escalation-ladder">

## A failure escalates through four tiers, and no tier is retried

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#escalation-ladder"><code>errors/escalation-ladder</code></a>
</div>

Nothing Novis runs is silently dropped, but not everything is *caught* — those are different
guarantees. A `FATAL`, or a `THROWN` that reached the request root uncaught, escalates through up
to four tiers, each with a fixed budget, before falling to the next:

| Tier | What runs | Rule |
|---|---|---|
| 1 | the request's own limit handler | [`errors/on-limit`](/docs/rules/errors/the-escalation-ladder/#on-limit "Tier 1 — a resource limit reaches the request that spent it") |
| 2 | the request's own uncaught-throw handler | [`errors/on-uncaught-throw`](/docs/rules/errors/the-escalation-ladder/#on-uncaught-throw "Tier 2 — an uncaught throw reaches the request root as itself") |
| 3 | the operator's configured `.nvs` handler | [`errors/handler-script`](/docs/rules/errors/the-escalation-ladder/#handler-script "Tier 3 — the configured handler is an ordinary isolate on the engine's own budget") |
| 4 | the hardcoded engine floor | [`errors/engine-floor`](/docs/rules/errors/the-escalation-ladder/#engine-floor "Tier 4 — the floor is native, bounded, and gives up rather than escalating") |

**No tier is retried.** A handler that throws, panics, or exceeds its own budget is abandoned where
it stands and the failure drops to the next tier — never to the same one again. A second attempt is
how "catch and log" becomes an infinite loop, so there is no bounded-N knob to size either.

Every tier that writes a log line writes the same record through the same native serialiser
([`errors/log-write`](/docs/rules/errors/diagnostics-and-logging/#log-write "One write path, and the engine floor is its other caller")), so a dashboard never reconciles two shapes depending on which tier
produced a line.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Every failure category ends in a log line, and no handler is ever given a second attempt at the same failure</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#on-limit" title="Tier 1 — a resource limit reaches the request that spent it"><code>errors/on-limit</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#on-uncaught-throw" title="Tier 2 — an uncaught throw reaches the request root as itself"><code>errors/on-uncaught-throw</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#handler-script" title="Tier 3 — the configured handler is an ordinary isolate on the engine's own budget"><code>errors/handler-script</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#engine-floor" title="Tier 4 — the floor is native, bounded, and gives up rather than escalating"><code>errors/engine-floor</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/the-floor-and-core-log-write-record-the-same-error-the-same-way.nvst"><code>tests/conformance/error/the-floor-and-core-log-write-record-the-same-error-the-same-way.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/a-limit-breach-inside-finally-is-a-clean-fatal.nvst"><code>tests/conformance/error/a-limit-breach-inside-finally-is-a-clean-fatal.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="on-uncaught-throw">

## Tier 2 — an uncaught throw reaches the request root as itself

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#on-uncaught-throw"><code>errors/on-uncaught-throw</code></a>
</div>

`Core\Fatal::onUncaughtThrow(closure(Throwable): void $handler): void` fires when an ordinary
`THROWN` propagates through every frame uncaught and reaches the isolate or request root.

It needs no reserve of its own: execution was healthy until this point, so the request's ordinary
remaining budget applies. It receives the **real `Throwable` object**, not copied data — this is
the root itself, not a boundary something has to copy across.

The zero-retry rule of [`errors/escalation-ladder`](/docs/rules/errors/the-escalation-ladder/#escalation-ladder "A failure escalates through four tiers, and no tier is retried") applies unchanged: a handler that itself
faults drops straight to [`errors/handler-script`](/docs/rules/errors/the-escalation-ladder/#handler-script "Tier 3 — the configured handler is an ordinary isolate on the engine's own budget").

After this tier and before native teardown, `Core\Script::onExit`'s queue runs. It is not a tier of
the ladder — it runs at every non-fatal ending including successful ones, needs no reserve, and
observes the ending rather than reporting a failure. A handler faulting here changes nothing about
it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The handler is request-local and fires exactly once; there is no global <code>set_exception_handler</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#handler-script" title="Tier 3 — the configured handler is an ordinary isolate on the engine's own budget"><code>errors/handler-script</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0127.md">record 0127</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/fatal-on-uncaught-throw-fires-once-and-a-handler-that-throws-is-abandoned.nvst"><code>tests/conformance/core/fatal-on-uncaught-throw-fires-once-and-a-handler-that-throws-is-abandoned.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/fatal-on-uncaught-throw-hands-the-handler-the-object-the-program-threw.nvst"><code>tests/conformance/core/fatal-on-uncaught-throw-hands-the-handler-the-object-the-program-threw.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/an-uncaught-throw-prints-a-backtrace.nvst"><code>tests/conformance/error/an-uncaught-throw-prints-a-backtrace.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="handler-script">

## Tier 3 — the configured handler is an ordinary isolate on the engine's own budget

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#handler-script"><code>errors/handler-script</code></a>
</div>

`[log] handler = 'path/to/handler.nvs'` names a script invoked as an ordinary `spawn script`
isolate — fresh arena, fresh globals, its own config overlay, sharing nothing but compiled code —
receiving one explicit report argument through `Core\Script::args()`.

It cannot use ambient `Core\Request`, `Core\Server` or `Core\Session`, and structurally could not
be trusted to: for a compile failure no request context exists yet, and for a panic the runtime's
own state is what is in question. That restriction is not new here; it already applies inside any
spawned isolate.

**The one deliberate exception to how an isolate is funded.** An ordinary child spends its parent's
budget, which is exactly wrong here — a request already at its ceiling has nothing left to give,
and still running when that is true is the whole point of this tier. So this isolate is charged to
a small, fixed, **engine-owned** allotment sized once per worker at boot rather than per request.
It is a named carve-out for this purpose and not a precedent.

The tier is the shared catch-all: it fires for any tier-1 or tier-2 failure, for a handler that was
never registered at all, for an internal panic unconditionally, and for an entry-script compile
failure where no frame ever existed to register anything from.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#engine-floor" title="Tier 4 — the floor is native, bounded, and gives up rather than escalating"><code>errors/engine-floor</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-write" title="One write path, and the engine floor is its other caller"><code>errors/log-write</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div></dl>

</div>

<div class="nv-rule" id="engine-floor">

## Tier 4 — the floor is native, bounded, and gives up rather than escalating

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#engine-floor"><code>errors/engine-floor</code></a>
</div>

Hardcoded in Rust, with no script execution — there is nothing left here that could throw, panic or
need a further tier, because this is the floor. It writes to an operator-owned sink named by `[log]
target`: `stderr`, `file:<path>` or `syslog`.

It fires when `[log] handler` is unset or [`errors/handler-script`](/docs/rules/errors/the-escalation-ladder/#handler-script "Tier 3 — the configured handler is an ordinary isolate on the engine's own budget") failed for any reason. **If
even this write fails — a full disk, a broken pipe — the failure is swallowed**, and the request
tears down normally regardless. That is deliberately boring: an undefined "what then" at the true
floor is worse than a defined "give up".

Because it writes unconditionally, it is bounded against the disk it writes to. A file target
rotates under a retention bound, and repeated identical records inside a window coalesce into one
record carrying a `count`. Both bounds sit on the sink, so no caller has to be trusted to be rare.

**It also restores the terminal**, before it writes and before it gives up. A CLI program holding
raw mode, a hidden cursor or a live region has put the operator's shell into a state only this
ladder can leave; a `finally` is not enough, because an internal panic bypasses user code by
design. A ladder that protects the process and leaves the shell unusable has failed at what it is
for.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#handler-script" title="Tier 3 — the configured handler is an ordinary isolate on the engine's own budget"><code>errors/handler-script</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-write" title="One write path, and the engine floor is its other caller"><code>errors/log-write</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0086.md">record 0086</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/log-target-both-writers-land-in-the-destination-the-deployment-named.nvst"><code>tests/conformance/core/log-target-both-writers-land-in-the-destination-the-deployment-named.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="panics-bypass-user-code">

## An internal panic never runs the failing request's own code

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#panics-bypass-user-code"><code>errors/panics-bypass-user-code</code></a>
</div>

A resource-limit `FATAL` means the *script* asked for too much. The runtime's own state is fine, so
a request-level handler reacting to it — logging with context, deciding whether the job should be
retried — is meaningful.

An internal-runtime-panic `FATAL` means Novis's own Rust code broke its own invariant. The runtime's
state is then exactly what cannot be trusted, and running more script code on top of it is the
riskier move, not the safer one.

So **only a resource-limit `FATAL` reaches [`errors/on-limit`](/docs/rules/errors/the-escalation-ladder/#on-limit "Tier 1 — a resource limit reaches the request that spent it")**. An internal panic goes straight
to [`errors/handler-script`](/docs/rules/errors/the-escalation-ladder/#handler-script "Tier 3 — the configured handler is an ordinary isolate on the engine's own budget") — still user-formattable, still routed wherever the operator sends
diagnostics — without ever calling back into the failing request's own code.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#on-limit" title="Tier 1 — a resource limit reaches the request that spent it"><code>errors/on-limit</code></a> <a href="/docs/rules/errors/how-an-error-travels/#helper-abi" title="A runtime helper never unwinds, and a panic dies inside one request"><code>errors/helper-abi</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#handler-script" title="Tier 3 — the configured handler is an ordinary isolate on the engine's own budget"><code>errors/handler-script</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div></dl>

</div>

<div class="nv-rule" id="compile-failure">

## A compile failure is catchable mid-execution and reaches the ladder at entry

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#compile-failure"><code>errors/compile-failure</code></a>
</div>

**The entry file, at request or isolate start.** No frame ever existed, so tiers 1 and 2 never had
the chance to register anything. It is reported the way any handler-less `FATAL`-class condition is:
straight to [`errors/handler-script`](/docs/rules/errors/the-escalation-ladder/#handler-script "Tier 3 — the configured handler is an ordinary isolate on the engine's own budget"), then [`errors/engine-floor`](/docs/rules/errors/the-escalation-ladder/#engine-floor "Tier 4 — the floor is native, bounded, and gives up rather than escalating"). Whether an HTTP response
shows a generic page or detail is `[http.errors] detail`'s answer, not the never-started script's —
the request had no code path in which it could have decided differently. Its default follows the run
mode: generic in production, full in development.

**Mid-execution**, through `require` or a `spawn script` target failing after its parent is already
running: an ordinary `ParseError`, catchable at the call site like any `Throwable`, which is what
lets a framework fail a bad template or plugin gracefully instead of the whole request. Uncaught, it
rides the normal [`errors/on-uncaught-throw`](/docs/rules/errors/the-escalation-ladder/#on-uncaught-throw "Tier 2 — an uncaught throw reaches the request root as itself") path — by then a frame did exist.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/how-an-error-travels/#throwable-hierarchy" title="A limit report is not a Throwable, and the type checker knows it"><code>errors/throwable-hierarchy</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0091.md">record 0091</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/a-parse-error-carries-an-issue-list.nvst"><code>tests/conformance/error/a-parse-error-carries-an-issue-list.nvst</code></a></dd></div></dl>

</div>
