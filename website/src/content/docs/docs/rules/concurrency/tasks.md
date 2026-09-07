---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Tasks"
description: "Every task is a child of the one that started it. Control never leaves a group with a child still running."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/concurrency/
  label: "Concurrency"
next:
  link: /docs/rules/concurrency/deferred-and-cross-request-state/
  label: "Deferred work and cross-request state"
---

<p class="nv-section-lead">Every task is a child of the one that started it. Control never leaves a group with a child still running.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">9</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">1</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">7</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#one-scheduler">Concurrency is the <code>Core\Task</code> roster over the runtime's own single scheduler</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-child-belongs-to-the-calling-task">Every task is a child of the task that started it, shares that request's accounting, and dies with it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#all-answers-a-typed-shape"><code>Core\Task::all</code> answers a shape with the argument's own field names, each field keeping its own type</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-all-field-answers-what-its-callable-declares">An <code>all</code> field answers its own callable's declared return type, and a callable declaring none answers <code>mixed</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#map-preserves-keys-and-order"><code>Core\Task::map</code> answers an array of the callback's own return type, in the input's keys and order</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#limit-and-deadline-are-the-only-bounds"><code>{limit, deadline}</code> is the whole of what bounds a group, and there is no <code>timeout</code> member and no <code>race</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#nothing-is-still-running-when-a-call-returns">Control never leaves <code>all</code> or <code>map</code> with a child still running</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#cancellation-runs-no-user-code">A cancelled task runs no <code>catch</code>, no cleanup and no handler, and cancellation is not a <code>Throwable</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-deadline-bounds-the-cancel-not-the-return">A deadline bounds when cancellation is asked for, not when the call returns</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="one-scheduler">

## Concurrency is the `Core\Task` roster over the runtime's own single scheduler

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#one-scheduler"><code>concurrency/one-scheduler</code></a>
</div>

Concurrency is `Core\Task` and nothing else. Three members carry it — `Task::all` over a fixed set,
`Task::map` over a collection, and `Task::afterResponse` for work that follows a response — and
`spawn`/`await` and `Core\Task\Channel` are the primitives beneath them.

They run on one scheduler, which is the runtime's own: stackful coroutines, thread-per-core,
shared-nothing. **A second scheduler may not be linked into the binary.** The prior art is
`tokio::JoinSet` and `tokio::Semaphore`, and they are unusable here because they are built on a
future model this runtime does not have; a crate that brings an executor, a reactor or a `spawn`
with it brings a second concurrency model beside the coroutines, and the dependency graph is
checked for exactly that. Where `tokio` appears at all it is compiled with `sync` alone — no `rt`,
no `net`, no `time`, no executor — which makes it a channel library and not a runtime.

The guarantee the whole roster is built on is [`concurrency/nothing-is-still-running-when-a-call-returns`](/docs/rules/concurrency/tasks/#nothing-is-still-running-when-a-call-returns "Control never leaves all or map with a child still running"),
and it is a property of the call rather than of a scope object: there is no nursery, no task group
and no handle to leak, so a task tree is bounded by the same accounting a request already has.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A PHP request has no concurrency to express at all, and the nearest equivalents — <code>curl_multi_*</code>, <code>pcntl_fork</code>, an extension's own event loop — have no counterpart here</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/isolates/#isolate-shares-nothing" title="Running another script is an in-process isolate that shares nothing with its parent but compiled code"><code>security/isolate-shares-nothing</code></a> <a href="/docs/rules/core-api/what-belongs-in-core/#tier-roster" title="Every subsystem's tier is recorded once in the roster, including the ones no milestone has built"><code>core-api/tier-roster</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/tests/manifest_policy.rs"><code>crates/nvs-runtime/tests/manifest_policy.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-child-belongs-to-the-calling-task">

## Every task is a child of the task that started it, shares that request's accounting, and dies with it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-child-belongs-to-the-calling-task"><code>concurrency/a-child-belongs-to-the-calling-task</code></a>
</div>

Every task starts as a child of the task that started it. There is no unparented task and no way to
write one: a group's closures are children of the calling task, a served connection is a child of
the task accepting on that core, a scheduled fire is a child of the ticker's task, and a `spawn
script` isolate is a child of the frame that spawned it.

Parentage is what carries accounting. A child shares its request's memory, CPU and capability
grants rather than opening an account of its own, so a tree's cost is attributable to one request
and bounded by that request's limits with nothing added. It is also what carries death: a parent
that ends cancels what it left running, so the tree cannot outlive it and there are no orphans.

A host with no calling task therefore has no place to put children, which is why every entry point
that runs a program makes a task first even where one buys nothing else.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP has no task, so it has no orphan either; here an unparented task is impossible rather than discouraged</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/isolates/#isolate-shares-nothing" title="Running another script is an in-process isolate that shares nothing with its parent but compiled code"><code>security/isolate-shares-nothing</code></a> <a href="/docs/rules/security/isolates/#isolate-budget-is-the-trees" title="A request and everything it spawns share one budget, accounted at the tree's root"><code>security/isolate-budget-is-the-trees</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/task/a-task-tree-dies-with-its-parent.nvst"><code>tests/conformance/task/a-task-tree-dies-with-its-parent.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/task/a-spawned-task-runs-and-its-value-is-awaited.nvst"><code>tests/conformance/task/a-spawned-task-runs-and-its-value-is-awaited.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="all-answers-a-typed-shape">

## `Core\Task::all` answers a shape with the argument's own field names, each field keeping its own type

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#all-answers-a-typed-shape"><code>concurrency/all-answers-a-typed-shape</code></a>
</div>

`Core\Task::all({...}, {limit?, deadline?}): S` takes a shape whose every field is a zero-argument
callable, runs them concurrently, and answers **a shape with the same field names, each field
carrying that field's own declared return type**:

```php
$page = Task::all({
    user:   fn(): User         => Users::load($id),
    orders: fn(): array<Order> => Orders::recent($id, 20),
}, {deadline: 2s});

echo $page->user->name;          // typed User, not mixed
```

That typing is the whole reason the member is worth having. The uniform alternative answers
`array<mixed>` and every call site then pays a cast, which is the untypeable-container failure the
language refuses everywhere else. No writable type says it, because the answer's field names are the
argument's own and the call site is what chooses them — so the parameter is a type the registry
carries for this one purpose, and what each field is worth is read off
[`concurrency/an-all-field-answers-what-its-callable-declares`](/docs/rules/concurrency/tasks/#an-all-field-answers-what-its-callable-declares "An all field answers its own callable's declared return type, and a callable declaring none answers mixed").

`all` over a one-field shape is legal and pointless, and nothing special-cases it. Its subject is a
shape and never an array; that is the line between it and [`concurrency/map-preserves-keys-and-order`](/docs/rules/concurrency/tasks/#map-preserves-keys-and-order "Core\Task::map answers an array of the callback's own return type, in the input's keys and order"),
and each member refuses the other's subject.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no PHP construct that runs several calls at once and hands back their results already typed</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/objects-and-shapes/#object-top" title="object is the opaque top of every class type"><code>types/object-top</code></a> <a href="/docs/rules/core-api/naming-and-shape/#shape-rules" title="Every Core member obeys the same twenty shape rules, R1–R20"><code>core-api/shape-rules</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0136.md">record 0136</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-task-all-answers-a-shape-each-field-keeping-its-own-type.nvst"><code>tests/conformance/core/a-task-all-answers-a-shape-each-field-keeping-its-own-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/core_members.rs"><code>crates/nvs-types/tests/core_members.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-all-field-answers-what-its-callable-declares">

## An `all` field answers its own callable's declared return type, and a callable declaring none answers `mixed`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#an-all-field-answers-what-its-callable-declares"><code>concurrency/an-all-field-answers-what-its-callable-declares</code></a>
</div>

Each field of a `Core\Task::all` argument answers **its own callable's declared return type**. A
field whose callable declares none — bare `callable`, the top of the lattice — answers `mixed`, and
only that field: every other one still carries the type it declared.

The field's type is read off the *argument's type*, not off the expression written at the call, so
where the closure came from stopped mattering. A literal, a first-class callable, a parameter and a
variable holding the whole shape are all equally good, and a shape assembled somewhere else and
passed in is too. What the parameter still refuses is a value that is not a shape of callables at
all: an array, a scalar or a field holding something that cannot be called, each reported as the
ordinary type mismatch.

[`types/callable-signature`](/docs/rules/types/closures/#callable-signature "A callable type may name its parameters, and must then name its return type") is what made this possible, and the restriction it replaces is worth
naming: until a `callable` carried a signature, the type existed only at the written literal, so a
framework storing closures in a variable could not use `all` at all. It can now, and it pays only
for the signatures it declines to write.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/closures/#callable-signature" title="A callable type may name its parameters, and must then name its return type"><code>types/callable-signature</code></a> <a href="/docs/rules/types/closures/#closure-literal" title="fn is the only closure literal, with an expression body or a block body"><code>types/closure-literal</code></a> <a href="/docs/rules/types/closures/#callable-absorbs-closure" title="callable is the only closure type name, and a callable value is invoked directly"><code>types/callable-absorbs-closure</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0114.md">record 0114</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0136.md">record 0136</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-task-all-field-answers-what-its-own-callable-declares.nvst"><code>tests/conformance/core/a-task-all-field-answers-what-its-own-callable-declares.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/core_members.rs"><code>crates/nvs-types/tests/core_members.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="map-preserves-keys-and-order">

## `Core\Task::map` answers an array of the callback's own return type, in the input's keys and order

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#map-preserves-keys-and-order"><code>concurrency/map-preserves-keys-and-order</code></a>
</div>

`Core\Task::map(array<T> $items, callable $fn, {limit?, deadline?}): array<U>` runs one callback per
element concurrently. Subject first, the callback receiving `($value, $key)` like every other
callback in the library, and the answer typed `array<U>` from that one callback's declared return
type.

**The result preserves the input's keys and its order, whatever order the children finished in.**
Completion order is a scheduling detail and is never observable in the answer.

`map` and [`concurrency/all-answers-a-typed-shape`](/docs/rules/concurrency/tasks/#all-answers-a-typed-shape "Core\Task::all answers a shape with the argument's own field names, each field keeping its own type") are two members because they are two jobs — a
fixed set of differently-typed things, and one operation over many same-typed things — and not two
spellings of one. A shape literal cannot express "one per element of a runtime array", and an array
cannot carry a per-element type. Each refuses the other's subject rather than coercing it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>array_map</code> runs sequentially; this one runs the callback concurrently and still answers in the input's order</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/naming-and-shape/#subject-first" title="The subject is parameter 1 of every Core member, with no exceptions"><code>core-api/subject-first</code></a> <a href="/docs/rules/core-api/parameters-and-options/#callback-receives-value-and-key" title="A callback always receives ($value, $key), and may declare fewer parameters than the call site passes"><code>core-api/callback-receives-value-and-key</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-task-map-answers-an-array-of-the-callbacks-own-return-type.nvst"><code>tests/conformance/core/a-task-map-answers-an-array-of-the-callbacks-own-return-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-task-map-and-a-task-all-refuse-each-others-subject.nvst"><code>tests/conformance/core/a-task-map-and-a-task-all-refuse-each-others-subject.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="limit-and-deadline-are-the-only-bounds">

## `{limit, deadline}` is the whole of what bounds a group, and there is no `timeout` member and no `race`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#limit-and-deadline-are-the-only-bounds"><code>concurrency/limit-and-deadline-are-the-only-bounds</code></a>
</div>

`all` and `map` take one trailing options shape with the same two fields.

**`limit: uint`** is the greatest number of the call's own children running at once. Exceeding it
**schedules** — it never throws. It is a concurrency shaper, deliberately unlike
[`concurrency/a-full-deferred-executor-throws`](/docs/rules/concurrency/deferred-and-cross-request-state/#a-full-deferred-executor-throws "Past [deferred] max_concurrent, afterResponse throws at the call site rather than queueing")'s host bound, which does throw. Both members
default to unbounded.

**`deadline: Duration`** bounds the whole call, not each child, and is written as a duration
literal. There is no default: a call naming no deadline is bounded by its request tree's own
`wall_time`, which is finite, and that is the only reason omitting it is safe.

These two are the whole vocabulary. There is no `timeout` member — a timeout over a group is this
option, and a timeout on one I/O call is that call's own option — and there is no `race`: over a
heterogeneous shape its answer would be a union the caller must discriminate, which is `mixed` and a
cast in practice. The homogeneous case that is actually wanted, hedging one request across two
replicas, has a name reserved for it so it cannot arrive twice.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A wait is bounded by an option on the call rather than by <code>set_time_limit</code>, <code>default_socket_timeout</code> or an ini setting read from somewhere else</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/parameters-and-options/#options-bag" title="A member's optional knobs are one trailing shape literal, never a flag or a bitmask"><code>core-api/options-bag</code></a> <a href="/docs/rules/types/text-and-literal-types/#duration-literal" title="1h30m is a Core\Time\Duration constant, in one grammar shared by source, parse and nvs.toml"><code>types/duration-literal</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-one-api" title="Core\Db is the only database API, and every statement it runs is prepared"><code>core-classes/db-one-api</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-task-call-is-bounded-by-limit-and-deadline-and-by-no-other-spelling.nvst"><code>tests/conformance/core/a-task-call-is-bounded-by-limit-and-deadline-and-by-no-other-spelling.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-task-map-takes-the-same-limit-and-deadline-shape-as-all.nvst"><code>tests/conformance/core/a-task-map-takes-the-same-limit-and-deadline-shape-as-all.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/task/a-deadline-is-the-only-spelling-for-a-bounded-wait.nvst"><code>tests/conformance/task/a-deadline-is-the-only-spelling-for-a-bounded-wait.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="nothing-is-still-running-when-a-call-returns">

## Control never leaves `all` or `map` with a child still running

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#nothing-is-still-running-when-a-call-returns"><code>concurrency/nothing-is-still-running-when-a-call-returns</code></a>
</div>

`all` and `map` do not return while a child is still running. Every way out goes through the same
four steps — cancel the siblings, wait for those cancellations, collect, return:

| what happens | what the call does |
|---|---|
| every child returns | the shape (`all`) or the array (`map`) |
| one child throws | every sibling is cancelled, the call waits, and the **first** throw propagates |
| two children throw | the first by completion order propagates; the second is reported, never swallowed |
| the deadline expires | every child is cancelled, the call waits, `TimeoutError` is thrown |
| the calling task is cancelled | every child is cancelled and nothing is returned |

This is the guarantee the rest of the roster is built on. It is what makes a group inside a database
transaction safe to reason about: when the call returns, no child is still holding a row lock, and
nothing can write to a slot the code after the `catch` has already read.

It is bought at a stated price — [`concurrency/a-deadline-bounds-the-cancel-not-the-return`](/docs/rules/concurrency/tasks/#a-deadline-bounds-the-cancel-not-the-return "A deadline bounds when cancellation is asked for, not when the call returns") — and
it is why cancellation is the runtime's own teardown rather than anything a program participates in
([`concurrency/cancellation-runs-no-user-code`](/docs/rules/concurrency/tasks/#cancellation-runs-no-user-code "A cancelled task runs no catch, no cleanup and no handler, and cancellation is not a Throwable")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A PHP script that starts something in the background has no construct that waits for it, so nothing there guarantees this</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/tasks/#cancellation-runs-no-user-code" title="A cancelled task runs no catch, no cleanup and no handler, and cancellation is not a Throwable"><code>concurrency/cancellation-runs-no-user-code</code></a> <a href="/docs/rules/security/isolates/#isolate-shares-nothing" title="Running another script is an in-process isolate that shares nothing with its parent but compiled code"><code>security/isolate-shares-nothing</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-write" title="One write path, and the engine floor is its other caller"><code>errors/log-write</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/task/a-task-tree-dies-with-its-parent.nvst"><code>tests/conformance/task/a-task-tree-dies-with-its-parent.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/task/a-cancelled-task-runs-no-user-code.nvst"><code>tests/conformance/task/a-cancelled-task-runs-no-user-code.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="cancellation-runs-no-user-code">

## A cancelled task runs no `catch`, no cleanup and no handler, and cancellation is not a `Throwable`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#cancellation-runs-no-user-code"><code>concurrency/cancellation-runs-no-user-code</code></a>
</div>

A cancelled task is torn down by the runtime where it parked. **No `catch` clause runs, no
`finally`-shaped cleanup runs, and no registered handler runs.** Native teardown still runs, because
none of it is script: the arena is dropped, refcounts are released, an open transaction is rolled
back by its own drop, an open file is closed.

Cancellation is therefore **not a `Throwable`** and cannot be caught, exactly as a resource-limit
report is not one. There are three ways to be cancelled — a sibling threw, a deadline expired, or
the parent died, which for a served request includes the client disconnecting — and all three mean
the request is already failing. Running arbitrary user cleanup there means running unbudgeted code
inside a failure, which is how a timeout becomes a hang, and a `catch (Throwable)` that swallowed a
cancellation would do exactly that.

The consequence to learn: **work that must happen does not go in a cancellable task's cleanup.** It
goes inside the transaction that made it necessary, in
[`concurrency/after-response-outlives-the-connection`](/docs/rules/concurrency/deferred-and-cross-request-state/#after-response-outlives-the-connection "Core\Task::afterResponse runs after the request's own frame returns, still charged to the request tree")'s deferred work, or in a durable queue.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The <code>finally</code>-shaped cleanup a PHP developer would reach for does not run, and there is nothing to catch</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/how-an-error-travels/#throwable-hierarchy" title="A limit report is not a Throwable, and the type checker knows it"><code>errors/throwable-hierarchy</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/security/isolates/#isolate-teardown-is-a-drain-then-a-sweep" title="An isolate ends by draining its roots through the one release worklist and then sweeping what a cycle held up"><code>security/isolate-teardown-is-a-drain-then-a-sweep</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/task/a-cancelled-task-runs-no-user-code.nvst"><code>tests/conformance/task/a-cancelled-task-runs-no-user-code.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-cancelled-task-dies-where-it-sleeps.nvst"><code>tests/conformance/core/a-cancelled-task-dies-where-it-sleeps.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/deadline_poll.rs"><code>crates/nvs-stdlib/tests/deadline_poll.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-deadline-bounds-the-cancel-not-the-return">

## A deadline bounds when cancellation is asked for, not when the call returns

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#a-deadline-bounds-the-cancel-not-the-return"><code>concurrency/a-deadline-bounds-the-cancel-not-the-return</code></a>
</div>

A `deadline` bounds when cancellation is **requested**, not when the call returns.

A child blocked mid-statement cannot simply be abandoned: a database connection with an unread
result set is unusable, so the statement is cancelled through the driver's own mechanism and the
call waits until that connection is back in a known state. That drain is bounded in turn by the
connection's own `timeout`, after which the connection is **closed rather than returned to the
pool**.

So `deadline: 2s` can return at two seconds plus one connection timeout in the pathological case.
Overrunning a stated deadline by a bounded amount is the correct trade against handing a poisoned
connection back to a pool, where it would fail some later, unrelated request. A deadline is a bound
on when the work stops being asked for; it is not a hard wall-clock guarantee on return.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/tasks/#nothing-is-still-running-when-a-call-returns" title="Control never leaves all or map with a child still running"><code>concurrency/nothing-is-still-running-when-a-call-returns</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-one-api" title="Core\Db is the only database API, and every statement it runs is prepared"><code>core-classes/db-one-api</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-busy-state" title="Busy state is a field on the connection, and a wire not at a message boundary is closed rather than reset"><code>core-classes/db-connection-busy-state</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a></dd></div></dl>

</div>
