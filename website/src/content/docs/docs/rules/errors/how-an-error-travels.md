---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "How an error travels"
description: "A throw is a checked return, not an unwind. What that buys, what it costs, and how deep recursion is allowed to go."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/errors/
  label: "Errors"
next:
  link: /docs/rules/errors/the-escalation-ladder/
  label: "The escalation ladder"
---

<p class="nv-section-lead">A throw is a checked return, not an unwind. What that buys, what it costs, and how deep recursion is allowed to go.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">5</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">5</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">4</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#throwable-hierarchy">A limit report is not a Throwable, and the type checker knows it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#propagation">An error propagates as a checked return, never by unwinding</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#throw-is-not-slower">A throw costs no more than a return, and only its raise allocates</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#helper-abi">A runtime helper never unwinds, and a panic dies inside one request</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#stack-depth">Recursion is bounded twice: a catchable error, then a fatal</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="throwable-hierarchy">

## A limit report is not a Throwable, and the type checker knows it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#throwable-hierarchy"><code>errors/throwable-hierarchy</code></a>
</div>

The `Throwable` classes are global and are the ordinary `catch` target. They include `ParseError`,
thrown when a file pulled in mid-execution fails to compile — see [`errors/compile-failure`](/docs/rules/errors/the-escalation-ladder/#compile-failure "A compile failure is catchable mid-execution and reaches the ladder at entry").

The value a resource-limit `FATAL` carries **does not implement `Throwable`**. That is not a
runtime check some future `catch` site could forget: it is a type-checker fact, so `catch
(Throwable $e)` around code that hits a memory or CPU limit provably cannot see the report, the
same way `int + uint` provably cannot compile.

This is what makes "a fatal is not catchable" hold at the ABI level without every call site
cooperating, and it is why no catch-loop can be written for a resource limit at all — not merely
why one is unlikely.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>catch (Throwable)</code> provably cannot see a resource-limit fatal; <code>Exception</code> and <code>Error</code> are not class names</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#on-limit" title="Tier 1 — a resource limit reaches the request that spent it"><code>errors/on-limit</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#compile-failure" title="A compile failure is catchable mid-execution and reaches the ladder at entry"><code>errors/compile-failure</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/a-limit-fatal-is-not-catchable.nvst"><code>tests/conformance/error/a-limit-fatal-is-not-catchable.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/a-parse-error-carries-an-issue-list.nvst"><code>tests/conformance/error/a-parse-error-carries-an-issue-list.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="propagation">

## An error propagates as a checked return, never by unwinding

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#propagation"><code>errors/propagation</code></a>
</div>

Every compiled function and every runtime helper carries one signature, and errors travel in its
return value:

```rust
extern "C" fn(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32
```

`0` is `OK` and the result is in `*out`. `1` is `THROWN`: an exception is pending in `Ctx` and a
`catch` may take it. `2` is `FATAL`: unrecoverable, bound for the request boundary, and not
catchable by Novis code — see [`errors/throwable-hierarchy`](/docs/rules/errors/how-an-error-travels/#throwable-hierarchy "A limit report is not a Throwable, and the type checker knows it") for why the type checker, rather
than runtime discipline, is what makes that true.

Codegen checks the status after every call and branches to an error block, which is where the
frame's refcount decrements and its `finally` blocks live — the explicit equivalent of a landing
pad. Nothing unwinds, because `cranelift-jit` registers no unwind tables with the OS on any
platform, so an unwinder would walk off a coroutine stack into unrelated memory. Error paths are
therefore ordinary IR the optimiser can see through, and a throw across a coroutine boundary is
not a special case.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An exception is a status value checked after every call, not a stack unwind</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/how-an-error-travels/#helper-abi" title="A runtime helper never unwinds, and a panic dies inside one request"><code>errors/helper-abi</code></a> <a href="/docs/rules/errors/how-an-error-travels/#throw-is-not-slower" title="A throw costs no more than a return, and only its raise allocates"><code>errors/throw-is-not-slower</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/a-throw-crosses-two-frames.nvst"><code>tests/conformance/error/a-throw-crosses-two-frames.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-codegen/tests/throwing.rs"><code>crates/nvs-codegen/tests/throwing.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-codegen/tests/calls.rs"><code>crates/nvs-codegen/tests/calls.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/benches/abi-probe/tests/invariants.rs"><code>benches/abi-probe/tests/invariants.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/benches/abi-probe/tests/unwind_unavailable.rs"><code>benches/abi-probe/tests/unwind_unavailable.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="throw-is-not-slower">

## A throw costs no more than a return, and only its raise allocates

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#throw-is-not-slower"><code>errors/throw-is-not-slower</code></a>
</div>

The marginal cost of [`errors/propagation`](/docs/rules/errors/how-an-error-travels/#propagation "An error propagates as a checked return, never by unwinding")'s status check is **0.85 ns per frame**, measured as
the slope between a 2-frame and an 18-frame chain so the harness's own call-out overhead cancels. A
single L2 miss is roughly 10 ns. The branch is perfectly predicted on the happy path, so what is
actually spent is the instruction slot.

**A throw costs slightly less than a normal return** — 0.79–0.82× at depth 8 — because the error
path returns the status immediately while the success path also copies a 16-byte value up through
every frame. That is the property PHP compatibility rests on, since frameworks throw on ordinary
control-flow paths.

It holds only while *propagating* does not allocate. Storing a message as a `String` made a throw
2.8× a return, more than the whole propagation path it was meant to measure, so the pending-error
slot is a `Cow<'static, str>` and a static exception message allocates nothing.

**The raise itself renders one frame label, and that is the whole of what a throw allocates.** The
exception carries the frame it was raised in, rendered from the site the `throw` was compiled with,
so a `catch` beside the `throw` — the one place no frame is ever unwound out of — reads a backtrace
naming that frame instead of an empty one. It is spent **once per raise and never per frame**, which
is what leaves the slope above untouched, and the label the frame pushes as the throw leaves
replaces that rendering rather than following it, so no frame is named twice. **A checked operator
is handed its statement's site on the same terms**: the blob is baked in the cold block it already
raises from, beside the message bytes, so the arithmetic that does not overflow spends no
instruction on it. A raise a helper makes out of its own fault is handed no site and renders
nothing.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Throwing is not a performance cliff, so exceptions may be used on ordinary control-flow paths</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/benches/abi-probe/tests/perf_guards.rs"><code>benches/abi-probe/tests/perf_guards.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="helper-abi">

## A runtime helper never unwinds, and a panic dies inside one request

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#helper-abi"><code>errors/helper-abi</code></a>
</div>

Because nothing may unwind through a JIT frame, a runtime helper is declared `extern "C"` — never
`extern "C-unwind"` — and wraps its body in `catch_unwind`, turning a panic into `FATAL` with the
message recorded in `Ctx`. The `nvs_helper!` macro generates the wrapper, so it cannot be forgotten
one helper at a time, and `catch_unwind` costs nothing when no panic occurs.

`panic = "unwind"` is therefore load-bearing rather than a preference, and is set explicitly in
every workspace profile: `panic = "abort"` would turn every containable runtime bug into a process
kill, which is request isolation lost.

The wrapper does not stop at the helper. Code with no request beneath it — the accept loop, the
HTTP reader, the compiled-unit cache index — runs outside `nvs_helper!`, so a worker task's own
root carries a `catch_unwind` as well. And a panic raised while a panic is unwinding aborts the
process whatever the profile says, so nothing on a teardown path may panic and teardown does not
recurse.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#panics-bypass-user-code" title="An internal panic never runs the failing request's own code"><code>errors/panics-bypass-user-code</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/benches/abi-probe/tests/unwind_unavailable.rs"><code>benches/abi-probe/tests/unwind_unavailable.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="stack-depth">

## Recursion is bounded twice: a catchable error, then a fatal

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#stack-depth"><code>errors/stack-depth</code></a>
</div>

Novis compiles natively, so every user call is a real machine frame. Exhausting a native stack is a
`SIGSEGV`, which [`errors/helper-abi`](/docs/rules/errors/how-an-error-travels/#helper-abi "A runtime helper never unwinds, and a panic dies inside one request")'s `catch_unwind` does not contain — one request would
take down the worker and every other request on it.

So recursion is bounded twice. A catchable `RecursionError` throws at a **soft** depth, so a
recursive-descent parser or a walk over untrusted-depth data can degrade instead of dying; this is
safe here in a way it is not in PHP, because [`errors/propagation`](/docs/rules/errors/how-an-error-travels/#propagation "An error propagates as a checked return, never by unwinding") pops frames as it unwinds,
so the handler runs with a shallow stack again. A non-catchable `FATAL` at the true limit is the
floor beneath it and reaches [`errors/on-limit`](/docs/rules/errors/the-escalation-ladder/#on-limit "Tier 1 — a resource limit reaches the request that spent it") like any other resource limit.

The ceiling is **8 MB of reserved address space per coroutine**, about 65,000 frames, of which only
touched pages are resident. The check is a comparison of the stack pointer against a `stack_limit`
field in `Ctx`'s existing hot cache line, emitted where the safepoint poll already loads that line,
and elided in a leaf function whose frame fits the reserved slack. The fast path compares against
the soft limit only. Measured against a 1.32 ns per-call slope it adds ≈0.3 ns — ≈0.06% on a
request making 40,000 calls, and nothing at all in loops and leaf-only code.

This bound is emitted at Novis function entry, so it reaches recursion through Novis frames and only
those. Request data also recurses through engine frames — a nested document in a decoder, a nested
value graph in teardown — which are bounded separately by an explicit depth counter and an
iterative teardown.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Recursion is bounded by an 8 MB native stack rather than by <code>memory_limit</code>, so deep PHP recursion needs a rewrite</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#on-limit" title="Tier 1 — a resource limit reaches the request that spent it"><code>errors/on-limit</code></a> <a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-codegen/tests/stack_limit.rs"><code>crates/nvs-codegen/tests/stack_limit.rs</code></a></dd></div></dl>

</div>
