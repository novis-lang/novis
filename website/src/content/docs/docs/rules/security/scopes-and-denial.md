---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Scopes, denial and the address policy"
description: "How a path scope is decided, why a denial is catchable, and why the network address policy lives inside the capability itself."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/security/capabilities/
  label: "Capabilities and authority"
next:
  link: /docs/rules/security/tainted-data/
  label: "Tainted data and its sinks"
---

<p class="nv-section-lead">How a path scope is decided, why a denial is catchable, and why the network address policy lives inside the capability itself.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">10</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">3</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">2</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#path-scope-canonicalise-then-prefix">A path scope is canonicalise-then-prefix over whole components, and a path that does not exist yet is its deepest existing ancestor</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#denial-is-a-runtime-error">A denied capability is a catchable <code>RuntimeError</code> naming the capability, never a fatal</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#optional-capability-degrades">A capability is required or optional, and only a required one refuses to build</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#no-runtime-grant">Two layers enforce a capability and only the static one grants; nothing anywhere widens</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#capability-costs-nothing-unasked">A member that needs no capability pays nothing, and one that does pays only beside a syscall</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#package-authority-is-granted-one-line-at-a-time">A dependency's authority is granted by the application one line at a time, and a declaration grants nothing</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#process-exec-capability">Executing a program is deny-by-default under its own capability, and there is no shell to interpolate into</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#net-address-policy"><code>net.connect</code> carries an address policy that denies the private ranges, and an exception is one written IP address</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#net-listen-is-a-separate-grant-from-net-connect">The grant follows what the program is doing rather than the transport: reaching out is <code>net.connect</code>, binding is <code>net.listen</code>, and neither widens the other</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-policy-lives-in-the-capability">The address policy lives in the capability, so every client obeys it and none of them may hold its own</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="path-scope-canonicalise-then-prefix">

## A path scope is canonicalise-then-prefix over whole components, and a path that does not exist yet is its deepest existing ancestor

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#path-scope-canonicalise-then-prefix"><code>security/path-scope-canonicalise-then-prefix</code></a>
</div>

A path-bearing capability resolves by one comparison: **canonicalise both sides, then compare whole
components.** A traversal through `..` does not reach a root it was not already under, a symlink
planted under a granted root does not carry the root's grant to its target, and a sibling directory
whose name merely starts with the root's is not inside it. There is one implementation of this rule in
the tree, and adding a second is how one of the callers ends up accepting a symlink.

The granted roots are canonicalised **once, when the snapshot is built**: a root still spelled the way
the operator typed it is a comparison against the wrong thing, and doing it per call would put a
resolution on the grant side of every check.

A write to a file that does not exist yet cannot be canonicalised, and creating it to find out whether
creating it is allowed is obviously wrong. So the argument canonicalises its **deepest existing
ancestor** and re-appends the remainder, with the remainder refused outright if it contains `..` — the
one component that could still escape after the ancestor is pinned.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>..</code> and a planted symlink cannot leave a granted root, because the comparison is made after resolution and never on the text</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/capabilities/#capability-question-is-grant-and-scope" title="A capability question is a grant and a scope, asked of the request's own configuration snapshot"><code>security/capability-question-is-grant-and-scope</code></a> <a href="/docs/rules/security/closed-doors/#a-path-is-not-a-url" title="A path is a filesystem path: no member dispatches on a scheme prefix, and nothing may register one"><code>security/a-path-is-not-a-url</code></a> <a href="/docs/rules/security/closed-doors/#script-spawn-capability" title="Executing code is its own capability, and the entry path is canonicalised and then prefix-checked"><code>security/script-spawn-capability</code></a> <a href="/docs/rules/errors/ambiguous-input/#path-component-refusals" title="A path component that does not spell what it resolves to is refused, on every platform"><code>errors/path-component-refusals</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0104.md">record 0104</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/cap/a-path-outside-the-granted-roots-is-refused.nvst"><code>tests/conformance/cap/a-path-outside-the-granted-roots-is-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/file-refuses-every-spelling-of-an-ungranted-path.nvst"><code>tests/conformance/core/file-refuses-every-spelling-of-an-ungranted-path.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/capability.rs"><code>crates/nvs-stdlib/tests/capability.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="denial-is-a-runtime-error">

## A denied capability is a catchable `RuntimeError` naming the capability, never a fatal

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#denial-is-a-runtime-error"><code>security/denial-is-a-runtime-error</code></a>
</div>

A denied capability throws a `RuntimeError`, and a program may catch it and degrade. It is not an
escalation: a limit breach is fatal because the request has already consumed something it cannot give
back, whereas a capability denial is known *before* any work is done and leaves nothing behind. A
cache that falls back to recomputing when writing is not granted is a reasonable program, and making
the refusal uncatchable would forbid it ([`errors/escalation-ladder`](/docs/rules/errors/the-escalation-ladder/#escalation-ladder "A failure escalates through four tiers, and no tier is retried")).

No new class is added: `RuntimeError` is "the world said no", where the world is the operator. The
message names the capability **in its configuration spelling**, and for a scoped one the argument that
fell outside the grant — because the reader of that message is usually the operator, and the grant
name is the string they will add to their configuration.

A predicate that would otherwise answer `false` is refused rather than answered, so an ungranted
deployment never looks like a negative result.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/capabilities/#capability-check-at-the-door" title="The capability check lives inside the function that performs the effect, and that door is the only way out of the process"><code>security/capability-check-at-the-door</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/errors/how-an-error-travels/#throwable-hierarchy" title="A limit report is not a Throwable, and the type checker knows it"><code>errors/throwable-hierarchy</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/cap/an-ungranted-capability-throws-naming-it.nvst"><code>tests/conformance/cap/an-ungranted-capability-throws-naming-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/io-is-writable-is-refused-by-the-capability-rather-than-answered-false.nvst"><code>tests/conformance/core/io-is-writable-is-refused-by-the-capability-rather-than-answered-false.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="optional-capability-degrades">

## A capability is required or optional, and only a required one refuses to build

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#optional-capability-degrades"><code>security/optional-capability-degrades</code></a>
</div>

A package's manifest splits what it asks for into `required` and `optional`. A **required** capability
that is not granted is a compile error naming the namespace, the capability and the grant line that
would fix it. An **optional** one compiles either way, and each such call site carries a guard that
throws if it is reached — an ordinary catchable throwable, because a package that declared a
capability optional has said it can proceed without it.

The split exists because the compile-time check's granularity is **reachability, not execution**: a
class no name reaches is never part of your program, but inside a class your program does name, every
call site is checked including a branch that never runs. Without the split, the reachability of a
*class* would be the unit of capability granularity, which is far too coarse for a class with two
halves.

`Core\Cap::has` reports what the call site already holds. It is **not** a runtime grant: nothing
widens ([`security/no-runtime-grant`](/docs/rules/security/scopes-and-denial/#no-runtime-grant "Two layers enforce a capability and only the static one grants; nothing anywhere widens")).

**Partly on disk.** `Core\Cap::has` exists and refuses a written name outside the roster while
checking. The required/optional manifest split, the guard at an optional call site and the compile
error for a required one have no representation in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/scopes-and-denial/#no-runtime-grant" title="Two layers enforce a capability and only the static one grants; nothing anywhere widens"><code>security/no-runtime-grant</code></a> <a href="/docs/rules/security/capabilities/#capability-roster-is-closed" title="Every capability name that exists is on one roster, and a name outside it is refused where it is written"><code>security/capability-roster-is-closed</code></a> <a href="/docs/rules/security/capabilities/#authority-is-the-enclosing-namespace" title="Authority is a property of the namespace enclosing the code, never of the nesting depth or the name being called"><code>security/authority-is-the-enclosing-namespace</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0112.md">record 0112</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0081.md">record 0081</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0061.md">record 0061</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-capability-query-answers-the-grant-table-both-ways.nvst"><code>tests/conformance/core/a-capability-query-answers-the-grant-table-both-ways.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-capability-query-refuses-a-name-no-grant-can-hold.nvst"><code>tests/conformance/core/a-capability-query-refuses-a-name-no-grant-can-hold.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-runtime-grant">

## Two layers enforce a capability and only the static one grants; nothing anywhere widens

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#no-runtime-grant"><code>security/no-runtime-grant</code></a>
</div>

Two layers enforce a capability. The **static** one reads the grant tables at compile time, costs
nothing, and answers *may this code ever do this?* The **runtime** one reads the request's effective
configuration at every door and at every optional guard, costs one branch, and answers *may this
request, right now?*

The runtime layer exists because narrowing exists: a request may tighten a capability, and an isolate
may drop grants at the spawn site ([`security/script-spawn-capability`](/docs/rules/security/closed-doors/#script-spawn-capability "Executing code is its own capability, and the entry path is canonicalised and then prefix-checked")). **It may only ever
drop.** There is no runtime grant, and adding one would spend the property this whole design rests on
in exchange for reintroducing the dynamic escape the language closed elsewhere — by having no `eval`
([`security/no-eval`](/docs/rules/security/closed-doors/#no-eval "There is no eval, and no Core member compiles a string produced at run time")), no string or array callables, and reflection that hands back an inert tree
([`security/reflection-needs-no-capability`](/docs/rules/security/closed-doors/#reflection-needs-no-capability "Reflection and AST parsing are capability-free, because neither reaches the world outside the process")).

Static attribution is total in Novis only because those doors are shut. A runtime widen would be a
further one opened, and every claim above it would have to be qualified.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/capabilities/#capability-check-at-the-door" title="The capability check lives inside the function that performs the effect, and that door is the only way out of the process"><code>security/capability-check-at-the-door</code></a> <a href="/docs/rules/security/closed-doors/#closed-doors" title="Four doors are closed by construction, and no configuration reopens any of them"><code>security/closed-doors</code></a> <a href="/docs/rules/security/closed-doors/#script-spawn-capability" title="Executing code is its own capability, and the entry path is canonicalised and then prefix-checked"><code>security/script-spawn-capability</code></a> <a href="/docs/rules/security/scopes-and-denial/#optional-capability-degrades" title="A capability is required or optional, and only a required one refuses to build"><code>security/optional-capability-degrades</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0112.md">record 0112</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0052.md">record 0052</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0031.md">record 0031</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0019.md">record 0019</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst"><code>tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-capability-query-answers-the-grant-table-both-ways.nvst"><code>tests/conformance/core/a-capability-query-answers-the-grant-table-both-ways.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="capability-costs-nothing-unasked">

## A member that needs no capability pays nothing, and one that does pays only beside a syscall

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#capability-costs-nothing-unasked"><code>security/capability-costs-nothing-unasked</code></a>
</div>

A member that needs no capability pays nothing at all: no table lookup, no branch, no field on its
row, no code emitted at its call sites. That falls directly out of the check living inside a function
such a member never calls, and the declaration being data no execution path reads.

A member that does need one pays, on top of a syscall: one enum-indexed field read on the snapshot it
already holds, and for a scoped capability one canonicalisation of the argument plus a component-wise
prefix compare per granted root. The canonicalisation is on the order of a microsecond and is dwarfed
by the open it precedes.

Every member that reaches this check is by construction about to make a syscall, so **the check is
never on a hot path**. That is the whole latency argument, and it holds because of *where* the check
is rather than because of how it is written.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/capabilities/#capability-check-at-the-door" title="The capability check lives inside the function that performs the effect, and that door is the only way out of the process"><code>security/capability-check-at-the-door</code></a> <a href="/docs/rules/security/capabilities/#capability-declaration-is-one-table" title="What each Core member needs is declared once in one table, and nothing at run time reads it"><code>security/capability-declaration-is-one-table</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/capability.rs"><code>crates/nvs-stdlib/tests/capability.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="package-authority-is-granted-one-line-at-a-time">

## A dependency's authority is granted by the application one line at a time, and a declaration grants nothing

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#package-authority-is-granted-one-line-at-a-time"><code>security/package-authority-is-granted-one-line-at-a-time</code></a>
</div>

A package **declares** what it requests, split into required and optional. That declaration is
documentation and an upper bound on itself; it grants nothing. The **application grants explicitly,
one line at a time**, and a namespace granted nothing holds nothing. Adding a dependency prints every
capability requested by that package *and its whole transitive subgraph* before a human writes
anything, so the authority a new dependency brings is visible in one diff at the moment it is
introduced rather than discoverable by audit later.

What this buys, stated plainly: a fully malicious package that reaches the compiler cannot open a
socket, read a file, spawn a process, reach a database or spawn a script unless a human wrote its name
in a grant line. The compromise of a transitive dependency degrades from *arbitrary action with the
process's authority* to *arbitrary computation with no authority at all*.

**Not on disk.** There is no package manager, no manifest, and no grant line writer in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/capabilities/#authority-is-the-enclosing-namespace" title="Authority is a property of the namespace enclosing the code, never of the nesting depth or the name being called"><code>security/authority-is-the-enclosing-namespace</code></a> <a href="/docs/rules/security/capabilities/#grants-are-keyed-on-a-namespace" title="The application's grant table is keyed on namespace prefixes, and a capability it names must be on the roster"><code>security/grants-are-keyed-on-a-namespace</code></a> <a href="/docs/rules/security/scopes-and-denial/#no-runtime-grant" title="Two layers enforce a capability and only the static one grants; nothing anywhere widens"><code>security/no-runtime-grant</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0081.md">record 0081</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0112.md">record 0112</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0055.md">record 0055</a></dd></div></dl>

</div>

<div class="nv-rule" id="process-exec-capability">

## Executing a program is deny-by-default under its own capability, and there is no shell to interpolate into

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#process-exec-capability"><code>security/process-exec-capability</code></a>
</div>

Running another program is deny-by-default under `process.exec`, the same as every other
syscall-touching entry point: a request may narrow it further and never widen it, and calling without
the grant throws ([`security/denial-is-a-runtime-error`](/docs/rules/security/scopes-and-denial/#denial-is-a-runtime-error "A denied capability is a catchable RuntimeError naming the capability, never a fatal")). The grant is asked for before the target
is looked at, so an ungranted deployment never learns whether a binary exists.

The capability is only half of it, and the other half is that **there is no shell to interpolate
into**. An executable path and an argv array, with nothing in between that parses a command line,
removes the escaping question rather than answering it — which is a stronger guarantee than any amount
of quoting. Under the sink predicate the argv elements are therefore *data* while the executable path
is an instruction, so the path is the sink and the arguments are not
([`security/sink-predicate`](/docs/rules/security/tainted-data/#sink-predicate "A string or bytes parameter is a sink when its content becomes an instruction a parser executes")).

A grant that names executable roots resolves the same way a spawn root does, canonicalise-then-prefix
([`security/path-scope-canonicalise-then-prefix`](/docs/rules/security/scopes-and-denial/#path-scope-canonicalise-then-prefix "A path scope is canonicalise-then-prefix over whole components, and a path that does not exist yet is its deepest existing ancestor")).

The target is a path and never a `PATH` lookup. A bare name resolves against the current directory,
which is the resolution the grant was compared against, so the program the operating system starts is
the one the check approved — a name a `PATH` entry would have answered instead is a program from a
directory no grant named.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>shell_exec</code>, <code>system</code>, <code>passthru</code> and the backtick operator do not exist; there is an argv array and a grant</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/capabilities/#capability-check-at-the-door" title="The capability check lives inside the function that performs the effect, and that door is the only way out of the process"><code>security/capability-check-at-the-door</code></a> <a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/security/closed-doors/#script-spawn-capability" title="Executing code is its own capability, and the entry path is canonicalised and then prefix-checked"><code>security/script-spawn-capability</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-refuses-a-shell-target" title="A target only a second command-line parser could run is refused, on every platform"><code>core-classes/process-refuses-a-shell-target</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0112.md">record 0112</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-exec-is-deny-by-default.nvst"><code>tests/conformance/core/process-exec-is-deny-by-default.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-run-asks-for-the-capability-before-it-looks-at-the-target.nvst"><code>tests/conformance/core/process-run-asks-for-the-capability-before-it-looks-at-the-target.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-run-refuses-every-shell-target-and-only-those.nvst"><code>tests/conformance/core/process-run-refuses-every-shell-target-and-only-those.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="net-address-policy">

## `net.connect` carries an address policy that denies the private ranges, and an exception is one written IP address

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#net-address-policy"><code>security/net-address-policy</code></a>
</div>

`net.connect` is not a boolean and not merely a host list. It carries an address policy enforced on
**every outbound connection whose address the program supplies**, hardcoded URLs included, because a
hardcoded hostname can resolve into a private range and because deployment configuration supplies most
real endpoint URLs.

Denied by default: loopback, the private ranges, **link-local**, unspecified, and the IPv4-mapped IPv6
forms of all of them. An operator grants an exception as an **IP address literal** beside the connect
grant. Three things it is not, each a widening this refuses: not a hostname, because the policy is
asked of a resolved address and a name would except whatever it resolved to afterwards; not a range,
because an operator writing a whole `/8` hands back most of the table without naming a host; and not
`true`, which is the one place a capability's `true` does not mean everything.

The one class of address it does not govern is an endpoint an operator wrote into root-owned
configuration and granted by name — that address is not attacker-influenceable, and applying the
policy there would deny every ordinary deployment. A program-supplied target stays governed in full.

A configured forward proxy is such an endpoint and is not asked the table's question; and under
`[http.client.proxy] resolve = "proxy"` the destination's resolved address is not asked it either,
because Novis never sees one — [`http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`](/docs/rules/http-server/the-http-client/#a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise "resolve is mandatory: local tunnels to the address Novis approved and keeps the pin, proxy narrows the policy to the URL's text and warns at every boot")
is where the operator writes that word and what the boot says every time they have.

"Connection" here means every outbound destination, not only a connected stream: a `Core\Net` datagram
sent to an address the program supplied is asked the same question at the send that a TCP connect is
asked at the connect. What the table does **not** govern is a *bind*, whose terms invert —
[`security/net-listen-is-a-separate-grant-from-net-connect`](/docs/rules/security/scopes-and-denial/#net-listen-is-a-separate-grant-from-net-connect "The grant follows what the program is doing rather than the transport: reaching out is net.connect, binding is net.listen, and neither widens the other") is that grant and says why it carries
no policy of its own.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#outbound-url-is-a-sink" title="An outbound URL or address is a sink, and a tainted one is refused where it is written"><code>security/outbound-url-is-a-sink</code></a> <a href="/docs/rules/http-server/the-http-client/#a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise" title="resolve is mandatory: local tunnels to the address Novis approved and keeps the pin, proxy narrows the policy to the URL's text and warns at every boot"><code>http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise</code></a> <a href="/docs/rules/security/scopes-and-denial/#the-policy-lives-in-the-capability" title="The address policy lives in the capability, so every client obeys it and none of them may hold its own"><code>security/the-policy-lives-in-the-capability</code></a> <a href="/docs/rules/security/capabilities/#capability-question-is-grant-and-scope" title="A capability question is a grant and a scope, asked of the request's own configuration snapshot"><code>security/capability-question-is-grant-and-scope</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-listen-is-a-separate-grant-from-net-connect" title="The grant follows what the program is doing rather than the transport: reaching out is net.connect, binding is net.listen, and neither widens the other"><code>security/net-listen-is-a-separate-grant-from-net-connect</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-capabilities" title="Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it"><code>core-classes/db-capabilities</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0142.md">record 0142</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0162.md">record 0162</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0182.md">record 0182</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-granted-host-is-still-refused-at-a-denied-address.nvst"><code>tests/conformance/core/a-granted-host-is-still-refused-at-a-denied-address.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-outbound-url-is-judged-before-the-capability-is-consulted.nvst"><code>tests/conformance/core/an-outbound-url-is-judged-before-the-capability-is-consulted.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-open-asks-the-grant-about-the-host-and-then-the-address.nvst"><code>tests/conformance/core/db-open-asks-the-grant-about-the-host-and-then-the-address.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="net-listen-is-a-separate-grant-from-net-connect">

## The grant follows what the program is doing rather than the transport: reaching out is `net.connect`, binding is `net.listen`, and neither widens the other

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#net-listen-is-a-separate-grant-from-net-connect"><code>security/net-listen-is-a-separate-grant-from-net-connect</code></a>
</div>

Which network capability an opening asks is decided by **what the program is doing**, not by which
transport it does it over. Reaching outward is `net.connect`, asked at `Scope::Host` and carrying
[`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address")'s denied-range table. Binding an endpoint is `net.listen`, asked of
the endpoint and carrying no address policy. Opening or binding a socket path is `net.local`
([`config/net-local-is-named-and-not-on-the-roster`](/docs/rules/config/stores-and-caches/#net-local-is-named-and-not-on-the-roster "A program-supplied socket path needs net.local, a path-scoped grant carrying no address policy and governing both ends")). **None of the three widens either of the
others**: a program that may reach a host may not bind one, and a program that may open a local socket
may not reach the network.

One socket can need two of them, because it does two things. A `Core\Net` datagram socket asks
`net.listen` once for its local port and `net.connect` for **every destination it sends to**, resolved
and checked at the send exactly as a TCP connect is checked at the connect. Granting it once at the
bind instead would make UDP a way around the address policy entirely: a program holding only its own
port could send to any address in any denied range.

`net.listen`'s entries are `address:port` literals matched exactly, and `true` is "any endpoint this
process may bind". There is no denied-range table here because the policy's terms **invert** under a
bind: binding loopback is the contained case and binding the unspecified address is the exposed one,
so importing the outbound table would deny the safe spelling and permit the dangerous one. What a bind
actually risks — occupying a port another service expects, or exposing a surface to a network the
operator did not intend — is answered by naming the endpoint rather than by classifying its range.
Exact matching rather than a pattern or a port range is `net.internal`'s choice for `net.internal`'s
reason: an operator writing a range hands back more than the endpoint they meant, and the grant's
value is that a reviewer can see what a deployment opened. An entry that does not parse as an endpoint
matches nothing.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/config/stores-and-caches/#net-local-is-named-and-not-on-the-roster" title="A program-supplied socket path needs net.local, a path-scoped grant carrying no address policy and governing both ends"><code>config/net-local-is-named-and-not-on-the-roster</code></a> <a href="/docs/rules/security/capabilities/#capability-roster-is-closed" title="Every capability name that exists is on one roster, and a name outside it is refused where it is written"><code>security/capability-roster-is-closed</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#net-one-api-three-transports" title="Core\Net is one class over TCP, UDP and Unix sockets, reached through five entry points that park on the runtime's own reactor"><code>core-classes/net-one-api-three-transports</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0162.md">record 0162</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/net-a-listen-grant-does-not-reach-a-host.nvst"><code>tests/conformance/core/net-a-listen-grant-does-not-reach-a-host.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/net-a-connect-grant-does-not-bind-a-port.nvst"><code>tests/conformance/core/net-a-connect-grant-does-not-bind-a-port.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-policy-lives-in-the-capability">

## The address policy lives in the capability, so every client obeys it and none of them may hold its own

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#the-policy-lives-in-the-capability"><code>security/the-policy-lives-in-the-capability</code></a>
</div>

Every client — the HTTP client, the raw socket layer, a program-supplied database target, and any
socket a host import hands to an extension — is subject to the same policy, enforced at the point the
connection is made rather than inside any one of them.

A policy held by a client is a policy the next client does not have. Putting it in the capability
means an extension cannot be granted a socket that escapes it, which matters because an extension may
legitimately be an I/O source and several network clients sit outside the standard library. It also
means the answer to "what may this deployment reach" is one grant an operator reads, not a survey of
every class that opens a connection.

The exception is the same one [`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address") names — a config-named endpoint the
operator has already approved by writing it — and it is a property of the address, not of the client
that dials it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/security/capabilities/#capability-check-at-the-door" title="The capability check lives inside the function that performs the effect, and that door is the only way out of the process"><code>security/capability-check-at-the-door</code></a> <a href="/docs/rules/security/tainted-data/#outbound-url-is-a-sink" title="An outbound URL or address is a sink, and a tainted one is refused where it is written"><code>security/outbound-url-is-a-sink</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0055.md">record 0055</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-outbound-url-is-judged-before-the-capability-is-consulted.nvst"><code>tests/conformance/core/an-outbound-url-is-judged-before-the-capability-is-consulted.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-open-asks-the-grant-about-a-sqlite-path-and-not-a-host.nvst"><code>tests/conformance/core/db-open-asks-the-grant-about-a-sqlite-path-and-not-a-host.nvst</code></a></dd></div></dl>

</div>
