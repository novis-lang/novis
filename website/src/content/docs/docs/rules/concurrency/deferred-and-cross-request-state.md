---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Deferred work and cross-request state"
description: "Work that outlives the response but not the request, and the only way a value outlives the request that made it."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/concurrency/tasks/
  label: "Tasks"
next:
  link: /docs/rules/concurrency/connections/
  label: "Persistent connections"
---

<p class="nv-section-lead">Work that outlives the response but not the request, and the only way a value outlives the request that made it.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">11</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">10</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">1</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">8</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#after-response-outlives-the-connection"><code>Core\Task::afterResponse</code> runs after the request's own frame returns, still charged to the request tree</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#only-the-request-registers-deferred-work">Only the request's own task may defer work, and deferred work may not defer more</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#deferred-work-cannot-write-the-response">Deferred work may read the request but not write the response, and its uncaught throw reaches the log alone</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#deferred-is-bounded-by-two-directives"><code>[deferred]</code> carries two bounds: a <code>System</code> <code>max_concurrent</code> and a <code>Runtime</code> <code>deadline</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-full-deferred-executor-throws">Past <code>[deferred] max_concurrent</code>, <code>afterResponse</code> throws at the call site rather than queueing</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#cross-request-state-is-explicit">A value outlives the request that made it only by being put into a named store</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#put-and-get-are-the-whole-boundary">The cache boundary is a copy in and a copy out, so nothing across it is read-modify-write</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-cached-value-is-copied-across-the-boundary">A value is copied into the cache and copied back out, by the same graph copy the isolate boundary uses</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#cache-memory-is-charged-to-the-core">The local tier's memory is charged to the core, capped in <code>nvs.toml</code>, and the cap evicts rather than failing a write</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-cross-request-stores-bytes-are-its-own-balance">A cross-request store's bytes go on a balance of their own, and no request is charged or credited for them</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#the-local-tier-cannot-hold-what-must-be-coherent">Anything a program relies on the value of goes to the shared tier, and the local tier is not offered for it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="after-response-outlives-the-connection">

## `Core\Task::afterResponse` runs after the request's own frame returns, still charged to the request tree

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#after-response-outlives-the-connection"><code>concurrency/after-response-outlives-the-connection</code></a>
</div>

`Core\Task::afterResponse(callable $fn, {deadline?}): void` registers work to run once the request
is done with:

```php
Task::afterResponse(fn(): void => Receipts::send($order), {deadline: 30s});
return $response;         // the client has its bytes; the receipt is still going out
```

The trigger is **the request task's own frame returning**. Under a server that is the moment the
response is fully written, the response being what the frame produced; under `nvs run`, which has no
response at all, it is the end of the script. One rule, and the member means the same thing on every
host. A request that ended by a throw, an `exit` or a `FATAL` runs none of it: that status is what
the host is about to report, and script running over it would lose one of the two.

**Memory, CPU and tasks stay charged to the request tree**, which is why this is affordable at all —
the tree simply stays in flight a little longer than the connection does, and every limit but
`wall_time` still bounds it. The connection is detached from the tree before the work runs, so the
peer going away cancels nothing.

**It is not a queue.** Nothing is durable, nothing retries, and a process that dies loses the work
with no record. Receipts, webhooks, cache warming and audit shipping are what it is for; anything
that *must* happen belongs in the transaction that made it necessary or in a store the application
owns.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>fastcgi_finish_request</code> keeps the whole worker busy with no accounting, no cap and no deadline; this keeps the tree, and the tree is still bounded</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a> <a href="/docs/rules/security/isolates/#isolate-budget-is-the-trees" title="A request and everything it spawns share one budget, accounted at the tree's root"><code>security/isolate-budget-is-the-trees</code></a> <a href="/docs/rules/core-classes/processes-and-files/#temporary-dir-sweep" title="A temporary directory lives under a Novis-owned root and is deleted when its script ends, and the sweep never throws"><code>core-classes/temporary-dir-sweep</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/task-after-response-runs.nvst"><code>tests/conformance/core/task-after-response-runs.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/task-all-composes-inside-deferred-work.nvst"><code>tests/conformance/core/task-all-composes-inside-deferred-work.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-host/tests/deferred.rs"><code>crates/nvs-host/tests/deferred.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="only-the-request-registers-deferred-work">

## Only the request's own task may defer work, and deferred work may not defer more

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#only-the-request-registers-deferred-work"><code>concurrency/only-the-request-registers-deferred-work</code></a>
</div>

Only the request's own task may register deferred work. A `Core\Task` child and a deferred closure
are each born sealed, and a registration made on one is refused with a `RuntimeError` at the call
site — while there is still a request to decide what to do about it — rather than accepted and then
dropped when that child ends.

The queue belongs to one request, so it is the only thing that can drain it. A child that wants to
defer hands the work back to the request that started it; that is what the refusal costs, and what
it buys is that no registration is ever silently lost.

**Deferred work may not defer more.** That is the same rule read once more rather than a second one:
a deferred closure runs as a child, on a sealed context, like every other child. A queue able to
extend itself is a tree that never leaves flight, and the whole argument for
[`concurrency/after-response-outlives-the-connection`](/docs/rules/concurrency/deferred-and-cross-request-state/#after-response-outlives-the-connection "Core\Task::afterResponse runs after the request's own frame returns, still charged to the request tree") is that the tree outlives the connection
only a little.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/deferred-and-cross-request-state/#after-response-outlives-the-connection" title="Core\Task::afterResponse runs after the request's own frame returns, still charged to the request tree"><code>concurrency/after-response-outlives-the-connection</code></a> <a href="/docs/rules/concurrency/tasks/#a-child-belongs-to-the-calling-task" title="Every task is a child of the task that started it, shares that request's accounting, and dies with it"><code>concurrency/a-child-belongs-to-the-calling-task</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/task-after-response-refuses-every-child-task.nvst"><code>tests/conformance/core/task-after-response-refuses-every-child-task.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="deferred-work-cannot-write-the-response">

## Deferred work may read the request but not write the response, and its uncaught throw reaches the log alone

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#deferred-work-cannot-write-the-response"><code>concurrency/deferred-work-cannot-write-the-response</code></a>
</div>

The response is on the wire before deferred work runs, so **the closure may not touch it**. A write
to `Core\Response` is a compile-time diagnostic where the call is statically visible and a throw
otherwise. `Core\Request`, `Core\Server` and `Core\Session` stay readable — it is still the same
tree, and the request's own values are still there to read.

An uncaught throw inside the closure goes through the escalation ladder to the log, carrying the
scheduling request's trace id. **The request's `onUncaughtThrow` handler does not fire**: it was
request-local and that request's own execution is over. There is no response left for the throw to
affect, so the log is the whole of what it can reach.

`all` and `map` compose inside deferred work with no special case, because a deferred closure is an
ordinary task.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Output written after <code>fastcgi_finish_request</code> is silently discarded; here it is refused, at compile time where the call is statically visible</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#on-uncaught-throw" title="Tier 2 — an uncaught throw reaches the request root as itself"><code>errors/on-uncaught-throw</code></a> <a href="/docs/rules/security/laundering/#response-body-is-one-typed-member" title="A response body is written by one typed member, and mixing echo with one of them does not compile"><code>security/response-body-is-one-typed-member</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a></dd></div></dl>

</div>

<div class="nv-rule" id="deferred-is-bounded-by-two-directives">

## `[deferred]` carries two bounds: a `System` `max_concurrent` and a `Runtime` `deadline`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#deferred-is-bounded-by-two-directives"><code>concurrency/deferred-is-bounded-by-two-directives</code></a>
</div>

```toml
[deferred]
max_concurrent = 256      # System   — request trees per core kept alive for deferred work
deadline       = "30s"    # Runtime  — the default a call inherits when it names none
```

**`max_concurrent` is `System`.** It bounds how much a core holds after responses are on the wire,
which is a host-sizing decision and not a request-local one, so a request cannot raise it. Note that
it counts **trees**, not registrations: one request with ten deferred closures is one.

**`deadline` is `Runtime`** — an ordinary per-request default a call may name its own value for,
bounded like every other limit by the tree's remaining budget.

What it spends, stated so an operator sizes it rather than discovers it: a deferred tree holds its
arena at its peak for the length of its deferred work, so the worst case per core is `max_concurrent
× [limits.hard] memory` **on top of** in-flight requests. That is bounded, and it is O(in-flight
deferred trees) rather than O(requests served).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/deferred-and-cross-request-state/#after-response-outlives-the-connection" title="Core\Task::afterResponse runs after the request's own frame returns, still charged to the request tree"><code>concurrency/after-response-outlives-the-connection</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-mode-directive" title="[debug] mode is the ceiling as well as the default, and writing a trace or a profile is its own capability"><code>testing/debug-mode-directive</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0064.md">record 0064</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/directives.rs"><code>crates/nvs-config/tests/directives.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-full-deferred-executor-throws">

## Past `[deferred] max_concurrent`, `afterResponse` throws at the call site rather than queueing

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-full-deferred-executor-throws"><code>concurrency/a-full-deferred-executor-throws</code></a>
</div>

Past `[deferred] max_concurrent`, `Core\Task::afterResponse` throws a `RuntimeError` **at the call
site**. It does not queue.

The throw arrives while the request is still running and can still decide what to do — respond
anyway, do the work inline, or tell the caller to retry — which is the only moment at which a
decision is available. The message names the directive an operator would change, because at this
point the program is not wrong; the deployment is loaded.

An unbounded queue in front of a non-durable executor is the worst of both designs: it hides the
overload and then loses the work anyway, and a bounded one is a durable queue with none of the
durability. A deployment reaching this cap has outgrown
[`concurrency/after-response-outlives-the-connection`](/docs/rules/concurrency/deferred-and-cross-request-state/#after-response-outlives-the-connection "Core\Task::afterResponse runs after the request's own frame returns, still charged to the request tree") and wants the durable queue it declines to
be. Failing loudly is what makes that legible, instead of receipts quietly ceasing to be sent.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no queue in front of this and no silent backlog; overload is reported to the request that caused it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/deferred-and-cross-request-state/#deferred-is-bounded-by-two-directives" title="[deferred] carries two bounds: a System max_concurrent and a Runtime deadline"><code>concurrency/deferred-is-bounded-by-two-directives</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/deferred.rs"><code>crates/nvs-runtime/src/deferred.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="cross-request-state-is-explicit">

## A value outlives the request that made it only by being put into a named store

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#cross-request-state-is-explicit"><code>concurrency/cross-request-state-is-explicit</code></a>
</div>

A value outlives the request that made it only by being **put into a named store**. There is no
ambient place to leave one: no shared segment, no cross-request superglobal, no static that survives
a request, and no process-wide table a later request can read.

`Core\Cache` is the sanctioned exception, and it is two members that hand back a store rather than
one API with a flag, so the choice a program made is visible in review rather than buried in an
argument list. The local tier is a **cache and not a store** — it must always be correct to find
nothing there — and anything whose value is relied upon uses the shared tier
([`concurrency/the-local-tier-cannot-hold-what-must-be-coherent`](/docs/rules/concurrency/deferred-and-cross-request-state/#the-local-tier-cannot-hold-what-must-be-coherent "Anything a program relies on the value of goes to the shared tier, and the local tier is not offered for it")).

The test is **what a program relies on**, not where bytes live. Per-core state derived from its own
inputs and unreadable by any program is not cross-request state: a compiled-pattern memo is
observable only as speed, and a metrics registry holds approximate aggregates that no program can
read and nothing decides on. Both are charged to a core and capped, which is the same accounting
[`concurrency/cache-memory-is-charged-to-the-core`](/docs/rules/concurrency/deferred-and-cross-request-state/#cache-memory-is-charged-to-the-core "The local tier's memory is charged to the core, capped in nvs.toml, and the cap evicts rather than failing a write") records.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>APCu, <code>shmop</code>, <code>sysvshm</code>, <code>sysvsem</code> and <code>sysvmsg</code> are gone, and there is no ambient place a value can be left for the next request to find</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/closed-doors/#no-cross-request-state" title="Nothing a request does is observable by another request except through an explicit, capability-gated store"><code>security/no-cross-request-state</code></a> <a href="/docs/rules/security/closed-doors/#closed-doors" title="Four doors are closed by construction, and no configuration reopens any of them"><code>security/closed-doors</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#two-cache-tiers" title="Cross-request state is two members with two contracts, never one API with a flag"><code>core-api/two-cache-tiers</code></a> <a href="/docs/rules/statements/where-state-lives/#storage-that-outlives-a-call" title="A program holds state in five declared places, and the list is closed"><code>statements/storage-that-outlives-a-call</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0052.md">record 0052</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-local-and-shared-are-two-members-with-two-contracts.nvst"><code>tests/conformance/core/cache-local-and-shared-are-two-members-with-two-contracts.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-local-twice-is-one-store-and-not-two.nvst"><code>tests/conformance/core/cache-local-twice-is-one-store-and-not-two.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="put-and-get-are-the-whole-boundary">

## The cache boundary is a copy in and a copy out, so nothing across it is read-modify-write

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#put-and-get-are-the-whole-boundary"><code>concurrency/put-and-get-are-the-whole-boundary</code></a>
</div>

What crosses the cache boundary is a copy in and a copy out. There is no read-modify-write across
it: no set-if-absent, no compare-and-set, no atomic increment, and no operation that observes an
entry and writes it in the same step.

An entry is therefore a payload rather than a live graph, and a rewrite **replaces** the entry
instead of merging into it. Two requests that read the same key, change what they read and write it
back are two last-writer-wins races, not a coordination primitive, and nothing about the boundary
pretends otherwise.

That is why a lock, a counter or a limiter is never built on the cache. The mechanisms that need
those guarantees have their own homes over the shared tier, where the store's own atomicity is what
provides them — `Core\RateLimit::consume` for a limit, a lease for scheduled work, and the database
for anything else.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>apcu_add</code>, <code>apcu_cas</code> and <code>apcu_inc</code> have no counterpart, so a lock or a counter is not built on the cache at all</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/deferred-and-cross-request-state/#a-cached-value-is-copied-across-the-boundary" title="A value is copied into the cache and copied back out, by the same graph copy the isolate boundary uses"><code>concurrency/a-cached-value-is-copied-across-the-boundary</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-two-members" title="consume and shed are two jobs with two verbs, and neither is a tier of the other"><code>core-classes/ratelimit-two-members</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#two-cache-tiers" title="Cross-request state is two members with two contracts, never one API with a flag"><code>core-api/two-cache-tiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-a-rewrite-replaces-the-entry-rather-than-adding-one.nvst"><code>tests/conformance/core/cache-a-rewrite-replaces-the-entry-rather-than-adding-one.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-cached-value-is-copied-across-the-boundary">

## A value is copied into the cache and copied back out, by the same graph copy the isolate boundary uses

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-cached-value-is-copied-across-the-boundary"><code>concurrency/a-cached-value-is-copied-across-the-boundary</code></a>
</div>

`put` copies a value out of the request heap into the cache; `get` copies one back in. The operation
is the recursive graph copy the language already defines and already shares with the isolate
boundary — not a third mechanism.

This is forced rather than chosen. A request's heap is dropped wholesale when the request ends, so a
value the cache holds cannot live there; and it cannot be handed back by reference either, or that
wholesale drop would free something the cache still holds. Reference-sharing would make a heap's
lifetime depend on what a request happened to read, which is exactly the property the wholesale drop
exists to guarantee.

A cached value is therefore subject to every restriction any crossing value is: a generator does not
go in, and neither does a `secret`. Both are refused at `put` rather than silently degraded.

The copy is a real per-`get` cost, paid to keep the request model intact. An implementation may
later share immutable scalars within a core by refcount, since a core is single-threaded — an
optimisation, and it must not be observable.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>APCu's serialize-on-store is a wire format with its own type losses; this is the language's own copy, and it refuses what may not cross rather than mangling it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/copying-and-serializing/#two-copy-depths" title="A copy is either clone's one level or the graph copy, and no class customizes either"><code>classes/two-copy-depths</code></a> <a href="/docs/rules/security/isolates/#isolate-values-cross-by-copy" title="A value crosses an isolate boundary as one graph copy, and a closure, an alias or a host handle does not cross at all"><code>security/isolate-values-cross-by-copy</code></a> <a href="/docs/rules/security/secrets/#secret-crosses-no-boundary" title="A secret value is refused at the one graph copy, so it reaches neither serialize nor any spawn"><code>security/secret-crosses-no-boundary</code></a> <a href="/docs/rules/iteration/#generator-stays-in-one-isolate" title="A generator does not cross a boundary and does not clone"><code>iteration/generator-stays-in-one-isolate</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-an-entry-is-a-copy-that-shares-nothing-with-the-request.nvst"><code>tests/conformance/core/cache-an-entry-is-a-copy-that-shares-nothing-with-the-request.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-secret-value-cannot-cross-the-graph-copy.nvst"><code>tests/conformance/reject/a-secret-value-cannot-cross-the-graph-copy.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="cache-memory-is-charged-to-the-core">

## The local tier's memory is charged to the core, capped in `nvs.toml`, and the cap evicts rather than failing a write

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#cache-memory-is-charged-to-the-core"><code>concurrency/cache-memory-is-charged-to-the-core</code></a>
</div>

Cache memory is **not attributable to a request**. It is charged to the core that holds it and
capped by an `nvs.toml` directive, and that cap is the deliberate, bounded exception to the rule that
every byte belongs to a request in flight.

**Exceeding the cap evicts rather than failing an allocation.** A write that crosses the ceiling
forgets an older entry and succeeds; a program never sees a store fail because the tier was full,
which is the whole point of a tier whose contract already says any entry may be absent.

Stated in the form the memory rule requires: the local tier costs **O(cores × working set)** — eight
cores hold up to eight copies of the same hot entry — bounded by the configured cap. It is
explicitly **not** O(requests served): an entry's lifetime is governed by TTL and eviction, never by
how much traffic has passed through. That multiplication is the price of the isolation it buys, and
it is recorded so it is a known number rather than a surprise in production.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>apc.shm_size</code> exhaustion makes a store fail; here the cap forgets an older entry and the write succeeds</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#cross-request-state-is-explicit" title="A value outlives the request that made it only by being put into a named store"><code>concurrency/cross-request-state-is-explicit</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-local-forgets-an-entry-rather-than-failing-the-write.nvst"><code>tests/conformance/core/cache-local-forgets-an-entry-rather-than-failing-the-write.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/directives.rs"><code>crates/nvs-config/tests/directives.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-cross-request-stores-bytes-are-its-own-balance">

## A cross-request store's bytes go on a balance of their own, and no request is charged or credited for them

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-cross-request-stores-bytes-are-its-own-balance"><code>concurrency/a-cross-request-stores-bytes-are-its-own-balance</code></a>
</div>

A request is measured against its own allocations, so the bytes of a store that outlives requests
move a balance of their own and no request is charged or credited for them. A request's memory
reading is a thread balance taken against where it started, and a store that one request fills and a
later one empties would otherwise drive that balance down and hand the later request a ceiling of
`[limits] memory` plus whatever it evicted.

The mechanism is an accounting bracket — `nvs_runtime::budget::Detached`, a guard whose lifetime is
the bracket. An allocation or a release made while one is held moves the process's detached balance
and leaves the live balance, every request's reading and every armed ceiling where it found them. A
pre-check inside a bracket refuses nothing, because bytes the process owns are not the request's to
be refused; the allocation counters keep moving, because an allocation made on the process's behalf
is still one this thread made.

**The bracket is held by the store, around every path that allocates or frees what it holds, and
never by a call site that happens to be storing.** What a bracket owes is symmetry: a block allocated
on one balance and freed on the other corrupts both. A store therefore copies what it is handed
rather than keeping the caller's allocation, so that the bytes it holds are allocated and freed
inside its own bracket. It is a guard rather than a pair of calls so that an unwind closes it, and
nesting is safe because each guard puts back what it found.

This does not make a free attributable in general. Charging a release to whoever allocated the block
needs per-request provenance, which is the request arena's to give; what holds today is that each
cross-request store brackets itself. A store whose entries are shared with the request that created
them cannot put the allocation and the release on one balance at all, and bracketing it anyway would
turn a bounded credit into an unbounded one — such a store stays unbracketed and records the gap in
its own module doc rather than taking the guard and breaking its symmetry.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/deferred-and-cross-request-state/#cache-memory-is-charged-to-the-core" title="The local tier's memory is charged to the core, capped in nvs.toml, and the cap evicts rather than failing a write"><code>concurrency/cache-memory-is-charged-to-the-core</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#cross-request-state-is-explicit" title="A value outlives the request that made it only by being put into a named store"><code>concurrency/cross-request-state-is-explicit</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#an-allocation-past-the-ceiling-is-refused-in-front-of-itself" title="An allocation past the memory ceiling is refused before it is made, and the refusal is a complete no-op"><code>errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0174.md">record 0174</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/tests/detached_accounting.rs"><code>crates/nvs-runtime/tests/detached_accounting.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-local-tier-cannot-hold-what-must-be-coherent">

## Anything a program relies on the value of goes to the shared tier, and the local tier is not offered for it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-local-tier-cannot-hold-what-must-be-coherent"><code>concurrency/the-local-tier-cannot-hold-what-must-be-coherent</code></a>
</div>

The local tier is per-core with no coherence between cores: a write on one core is not visible on
another, and any entry may be absent at any time. So anything a program **relies on the value of**
goes to the shared tier or the database — sessions, locks, idempotency keys, and any counter whose
value is acted upon.

This is enforced rather than documented. `Core\Session`'s configurable backends carry no local-tier
entry at all, so pointing a session at it is a configuration-time error naming the file the key was
written in, rather than a race that appears under load on a second core. Rate limits and the lease a
scheduled job takes have their own members over the shared tier for the same reason, so an
application is not left to arrange coherence for itself.

The test is what a program relies on, not where bytes live
([`concurrency/cross-request-state-is-explicit`](/docs/rules/concurrency/deferred-and-cross-request-state/#cross-request-state-is-explicit "A value outlives the request that made it only by being put into a named store")), and one thing that looks like a violation is
not one: a per-core metrics registry outlives requests and passes, because no program can read a
metric at all and its values are approximate aggregates merged arithmetically at scrape.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An APCu-backed session, lock or rate limit does not port: the local tier is not among the backends, so the miswiring is a configuration error rather than a race</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/lifetimes-and-absences/#session-roster" title="The session roster is start and six members, and a member called before start throws naming it"><code>core-api/session-roster</code></a> <a href="/docs/rules/statements/where-state-lives/#no-host-populated-variables" title="No variable is ever populated by the host; every superglobal is a Core class member"><code>statements/no-host-populated-variables</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-two-members" title="consume and shed are two jobs with two verbs, and neither is a tier of the other"><code>core-classes/ratelimit-two-members</code></a> <a href="/docs/rules/security/closed-doors/#no-cross-request-state" title="Nothing a request does is observable by another request except through an explicit, capability-gated store"><code>security/no-cross-request-state</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0075.md">record 0075</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-session-survives-a-request-on-another-core.nvst"><code>tests/conformance/core/a-session-survives-a-request-on-another-core.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-start-reaches-the-configured-shared-store-and-never-one-of-its-own.nvst"><code>tests/conformance/core/session-start-reaches-the-configured-shared-store-and-never-one-of-its-own.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-a-local-entry-may-be-absent-at-any-time.nvst"><code>tests/conformance/core/cache-a-local-entry-may-be-absent-at-any-time.nvst</code></a></dd></div></dl>

</div>
