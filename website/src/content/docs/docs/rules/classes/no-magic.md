---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Behaviour is declared, never magic"
description: "No method changes a class by being present. Ordering, string conversion and dumping each go through a named interface."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/classes/interfaces-and-delegation/
  label: "Interfaces and delegation"
next:
  link: /docs/rules/classes/copying-and-serializing/
  label: "Copying and serializing"
---

<p class="nv-section-lead">No method changes a class by being present. Ordering, string conversion and dumping each go through a named interface.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">7</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#comparable">Two objects order only when their class implements <code>Comparable</code>, and there is no property-walk fallback</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#comparable-is-same-class-only"><code>compareTo</code> fixes the other operand to <code>self</code>, so two different classes are never ordered against each other</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#ordering-lowers-to-compare-to"><code>&lt;</code>, <code>&gt;</code>, <code>&lt;=</code>, <code>&gt;=</code> and <code>&lt;=&gt;</code> on two objects lower to one <code>compareTo</code> call</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#stringable">An object becomes a string only through <code>Stringable::toString</code>, and everywhere else is a diagnostic</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-magic-methods">No method name changes a class's behaviour by being present; every PHP magic method is replaced by a declared interface or gone</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-call-magic">There is no <code>__call</code> or <code>__callStatic</code>: a call to a method the class does not declare is a diagnostic</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-destructors">There are no destructors: cleanup is an explicit method call the holder makes</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-debug-hook">A debug dump shows a class's real declared properties and their real values, and no class can change that</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="comparable">

## Two objects order only when their class implements `Comparable`, and there is no property-walk fallback

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#comparable"><code>classes/comparable</code></a>
</div>

Two objects may be compared with `<`, `>`, `<=`, `>=` or `<=>` only when their class implements the
global interface `Comparable`, whose single member is `compareTo(self $other): int` returning
negative, zero or positive. A class that does not implement it cannot be ordered, and the attempt is
a compile-time diagnostic naming `Comparable` as the fix. There is no fallback path anywhere in the
implementation.

`compareTo` is also what every member with a natural ordering reads: `Core\Arr::sort`, `Core\Arr::min`
and `max`, `Core\Math::min`, `max` and `clamp`, and a `Core\Heap` built without a comparator all order
two objects by it and throw for a pair whose class does not implement it. A `Core`-owned instance — a
`Core\BigInt`, a `Core\Time\Instant` — is ordered the same way and through the same one entry point.
An ordering a class states once is the only one anything in the language reads.

PHP walks two same-class objects' declared properties in order and takes the first difference —
behaviour that exists ambiently, that no class opts into or out of, and whose cost is unbounded in
the size of the graph it recurses into. An ordering a class produces should be the ordering its own
code states, once, reviewably.

`Comparable` lives in the global namespace beside `Stringable`, not under `Core`, because it is a
contract an ordinary class implements rather than a domain class holding `static` members. Equality
is a separate question and is unaffected: a `Comparable` class still compares by identity under `==`
([`expressions/object-identity-equality`](/docs/rules/expressions/truthiness-and-equality/#object-identity-equality "Two objects are equal only when they are the same object, and no class may change that")), and asking the content question explicitly is
`$a->compareTo($b) == 0`.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP's recursive property-by-property comparison of two same-class objects does not exist; ordering an unorderable pair is a compile error rather than an ambient walk</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/no-magic/#ordering-lowers-to-compare-to" title="&lt;, &gt;, &lt;=, &gt;= and &lt;=&gt; on two objects lower to one compareTo call"><code>classes/ordering-lowers-to-compare-to</code></a> <a href="/docs/rules/classes/no-magic/#comparable-is-same-class-only" title="compareTo fixes the other operand to self, so two different classes are never ordered against each other"><code>classes/comparable-is-same-class-only</code></a> <a href="/docs/rules/expressions/truthiness-and-equality/#one-equality-operator" title="== and != are the whole of equality, and ===, !== and &lt;&gt; do not parse"><code>expressions/one-equality-operator</code></a> <a href="/docs/rules/expressions/truthiness-and-equality/#object-identity-equality" title="Two objects are equal only when they are the same object, and no class may change that"><code>expressions/object-identity-equality</code></a> <a href="/docs/rules/types/text-and-literal-types/#ordering" title="Only the types the table orders may be ordered, and two objects need Comparable"><code>types/ordering</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0013.md">record 0013</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/class/comparable-orders-two-objects.nvst"><code>tests/conformance/class/comparable-orders-two-objects.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/class/ordering-two-objects-without-comparable-is-refused.nvst"><code>tests/conformance/class/ordering-two-objects-without-comparable-is-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/comparable_and_stringable.rs"><code>crates/nvs-types/tests/comparable_and_stringable.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="comparable-is-same-class-only">

## `compareTo` fixes the other operand to `self`, so two different classes are never ordered against each other

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#comparable-is-same-class-only"><code>classes/comparable-is-same-class-only</code></a>
</div>

`compareTo(self $other)` fixes the other operand to the implementing class. Comparing two objects
that are not both known to be that same class is a diagnostic even when both classes implement
`Comparable` independently — there is no cross-class overload and no implicit widening from a
subclass's `compareTo` to an ancestor's.

The alternative is a resolution question with no non-arbitrary answer: which of two classes' methods
decides the order of a mixed pair, and what a program should conclude when they disagree. Fixing the
parameter to `self` removes the question instead of answering it.

A type that genuinely needs to be ordered against a different type says so with an ordinary named
method — `Money::isGreaterThan(Distance $d): bool` reads oddly on purpose. The cost is real: PHP's
permissiveness here is gone until a parameterized `Comparable<T>` is designed, and no such generic
exists yet.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Two objects of different classes are a compile error rather than a comparison PHP answers by falling through its own coercion rules</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/no-magic/#comparable" title="Two objects order only when their class implements Comparable, and there is no property-walk fallback"><code>classes/comparable</code></a> <a href="/docs/rules/classes/no-magic/#ordering-lowers-to-compare-to" title="&lt;, &gt;, &lt;=, &gt;= and &lt;=&gt; on two objects lower to one compareTo call"><code>classes/ordering-lowers-to-compare-to</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0013.md">record 0013</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/comparable_and_stringable.rs"><code>crates/nvs-types/tests/comparable_and_stringable.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="ordering-lowers-to-compare-to">

## `<`, `>`, `<=`, `>=` and `<=>` on two objects lower to one `compareTo` call

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#ordering-lowers-to-compare-to"><code>classes/ordering-lowers-to-compare-to</code></a>
</div>

All five relational spellings lower to one call: `$a < $b` is `$a->compareTo($b) < 0`, `$a >= $b` is
`$a->compareTo($b) >= 0`, and `$a <=> $b` is the call itself. The lowering happens at compile time
and produces an ordinary method call — devirtualized when the static type is known exactly, like any
other call — so no second dispatch path exists beside the one every method already uses.

One method therefore fixes all five operators at once, and they cannot disagree with each other the
way five separate hooks could. A subclass that overrides `compareTo` changes how a base-typed pair
answers, because the call reaches the receiver's own implementation like any other virtual call.

A `compareTo` that throws propagates as a checked status ([`errors/propagation`](/docs/rules/errors/how-an-error-travels/#propagation "An error propagates as a checked return, never by unwinding")), so an ordering
is a call site with a failure edge rather than an operator that cannot fail.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/no-magic/#comparable" title="Two objects order only when their class implements Comparable, and there is no property-walk fallback"><code>classes/comparable</code></a> <a href="/docs/rules/types/declarations-and-numbers/#arithmetic" title="An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting"><code>types/arithmetic</code></a> <a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0013.md">record 0013</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/class/comparable-answers-every-relational-operator.nvst"><code>tests/conformance/class/comparable-answers-every-relational-operator.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/class/comparable-orders-a-subclass-through-its-override.nvst"><code>tests/conformance/class/comparable-orders-a-subclass-through-its-override.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="stringable">

## An object becomes a string only through `Stringable::toString`, and everywhere else is a diagnostic

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#stringable"><code>classes/stringable</code></a>
</div>

An object reaches a string only through the global interface `Stringable`, whose single member is
`toString(): string`. Every implicitly converting position — interpolation, concatenation, `echo` and
`print`, and an `as string` conversion — accepts an object only when its static type provably
implements it, and calls `toString()`. An object whose class does not is a compile-time diagnostic
naming `Stringable` as the fix; PHP's own answer here is already a fatal error, so nothing permissive
is being removed.

`Stringable` lives in the global namespace, not under `Core`: it is a contract an ordinary class
implements, not a domain class holding `static` members. The method is `toString`, not `__toString`,
for the same reason `PropertyObserver`'s members are not `__get` — a magic spelling would misdescribe
what the declaration site is doing.

The refusal is made wherever the static type names a class, which is the whole of what a compile-time
rule can promise. Through a `mixed` or a plain `object` the same question is answered from the
instance's runtime class and a class with no `toString` throws there, because there was no site to
refuse at.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>__toString</code> is not recognized; the capability is <code>implements Stringable</code> and its absence is refused where the object is rendered</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/no-magic/#no-magic-methods" title="No method name changes a class's behaviour by being present; every PHP magic method is replaced by a declared interface or gone"><code>classes/no-magic-methods</code></a> <a href="/docs/rules/classes/no-magic/#comparable" title="Two objects order only when their class implements Comparable, and there is no property-walk fallback"><code>classes/comparable</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/objects-and-shapes/#erased-member-access" title="A member reached through an erased receiver is answered at run time, and a failure is a throw"><code>types/erased-member-access</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0028.md">record 0028</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0013.md">record 0013</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/class/a-stringable-class-renders-through-tostring.nvst"><code>tests/conformance/class/a-stringable-class-renders-through-tostring.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/class/a-stringable-object-stringifies-at-every-implicit-site.nvst"><code>tests/conformance/class/a-stringable-object-stringifies-at-every-implicit-site.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/an-object-without-stringable-cannot-be-interpolated.nvst"><code>tests/conformance/reject/an-object-without-stringable-cannot-be-interpolated.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/comparable_and_stringable.rs"><code>crates/nvs-types/tests/comparable_and_stringable.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-magic-methods">

## No method name changes a class's behaviour by being present; every PHP magic method is replaced by a declared interface or gone

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-magic-methods"><code>classes/no-magic-methods</code></a>
</div>

No method name changes what a class does by being present. Each of PHP's magic methods is either
replaced by a declared interface — `__get`/`__set` by `PropertyObserver`
([`classes/property-observer`](/docs/rules/classes/properties/#property-observer "A class observes every read and write of its own properties by implementing PropertyObserver, never by declaring __get/__set")), `__toString` by `Stringable` ([`classes/stringable`](/docs/rules/classes/no-magic/#stringable "An object becomes a string only through Stringable::toString, and everywhere else is a diagnostic")), and
ordering's implicit property walk by `Comparable` ([`classes/comparable`](/docs/rules/classes/no-magic/#comparable "Two objects order only when their class implements Comparable, and there is no property-walk fallback")) — or removed outright:
`__call`/`__callStatic` ([`classes/no-call-magic`](/docs/rules/classes/no-magic/#no-call-magic "There is no __call or __callStatic: a call to a method the class does not declare is a diagnostic")), `__destruct`
([`classes/no-destructors`](/docs/rules/classes/no-magic/#no-destructors "There are no destructors: cleanup is an explicit method call the holder makes")), `__clone` and the four serialization hooks
([`classes/two-copy-depths`](/docs/rules/classes/copying-and-serializing/#two-copy-depths "A copy is either clone's one level or the graph copy, and no class customizes either")), `__isset`/`__unset` ([`classes/unset-is-refused-on-a-property`](/docs/rules/classes/properties/#unset-is-refused-on-a-property "unset on a declared property is a compile error, and isset on one is an ordinary null test")),
`__debugInfo` ([`classes/no-debug-hook`](/docs/rules/classes/no-magic/#no-debug-hook "A debug dump shows a class's real declared properties and their real values, and no class can change that")), `__invoke` ([`types/callable-is-a-closure`](/docs/rules/types/closures/#callable-is-a-closure "callable is satisfied by a closure and by nothing else, and no object is callable")) and
`__set_state`, whose reconstruct-from-generated-code use is answered by the closed round trip instead.

Every one of those names is refused where it is *written*: the method-casing rule allows no leading
underscore, so a class cannot declare a hook for the runtime to decline to call. That is stronger than
"never invoked", and it is what makes an absence checkable at all.

`__autoload` needs no decision — PHP removed it, and every class reference resolves statically, so
there is no runtime moment for a loader callback to attach to. What replaces
`spl_autoload_register` is [`programs/no-runtime-autoload`](/docs/rules/programs/names-and-files/#no-runtime-autoload "A name reaches its file while compiling; there is no runtime loader of any kind").

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A double-underscore method name is refused where it is written, so no class can declare a hook the runtime would call on its own</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/no-magic/#stringable" title="An object becomes a string only through Stringable::toString, and everywhere else is a diagnostic"><code>classes/stringable</code></a> <a href="/docs/rules/classes/no-magic/#no-destructors" title="There are no destructors: cleanup is an explicit method call the holder makes"><code>classes/no-destructors</code></a> <a href="/docs/rules/classes/no-magic/#no-call-magic" title="There is no __call or __callStatic: a call to a method the class does not declare is a diagnostic"><code>classes/no-call-magic</code></a> <a href="/docs/rules/classes/properties/#property-observer" title="A class observes every read and write of its own properties by implementing PropertyObserver, never by declaring __get/__set"><code>classes/property-observer</code></a> <a href="/docs/rules/classes/copying-and-serializing/#two-copy-depths" title="A copy is either clone's one level or the graph copy, and no class customizes either"><code>classes/two-copy-depths</code></a> <a href="/docs/rules/programs/names-and-files/#no-runtime-autoload" title="A name reaches its file while compiling; there is no runtime loader of any kind"><code>programs/no-runtime-autoload</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0028.md">record 0028</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0014.md">record 0014</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0023.md">record 0023</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0027.md">record 0027</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0029.md">record 0029</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/every-magic-method-name-this-adr-closes-is-unspellable.nvst"><code>tests/conformance/reject/every-magic-method-name-this-adr-closes-is-unspellable.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-call-magic">

## There is no `__call` or `__callStatic`: a call to a method the class does not declare is a diagnostic

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-call-magic"><code>classes/no-call-magic</code></a>
</div>

Calling a method a class does not declare is a compile-time diagnostic, like any other unresolvable
name. There is no `__call` and no `__callStatic`, and neither name can be declared at all — the
method-casing rule refuses a leading underscore before any resolution logic runs.

Nothing replaces them. The requirement is not that PHP's spelling is wrong but that dispatching to a
name the class never declared is: it is invisible from the declaration, unreadable by a checker or an
IDE without reimplementing PHP's dispatch rules, and it turns a mistyped method into behaviour rather
than an error.

A program that wants to handle a family of unknown calls writes an ordinary method taking an explicit
name and argument list, or a `match` keyed by name — visible in the class body and type-checked like
every other call. What that costs is real: a proxy or a fluent facade generated from `__call` has no
mechanical translation and needs a human to write the surface out.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Dispatch to an undeclared method name does not exist in any form, and the two names cannot even be written down</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/no-magic/#no-magic-methods" title="No method name changes a class's behaviour by being present; every PHP magic method is replaced by a declared interface or gone"><code>classes/no-magic-methods</code></a> <a href="/docs/rules/classes/properties/#no-dynamic-properties" title="A property that the class does not declare cannot be read or written, and no user code runs for the attempt"><code>classes/no-dynamic-properties</code></a> <a href="/docs/rules/types/closures/#callable-is-a-closure" title="callable is satisfied by a closure and by nothing else, and no object is callable"><code>types/callable-is-a-closure</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0014.md">record 0014</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0029.md">record 0029</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/class/an-unknown-method-is-a-compile-error.nvst"><code>tests/conformance/class/an-unknown-method-is-a-compile-error.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/every-magic-method-name-this-adr-closes-is-unspellable.nvst"><code>tests/conformance/reject/every-magic-method-name-this-adr-closes-is-unspellable.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-destructors">

## There are no destructors: cleanup is an explicit method call the holder makes

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-destructors"><code>classes/no-destructors</code></a>
</div>

Novis has no destructors. There is no refcount-triggered cleanup hook and no scope-exit hook, and
`__destruct` cannot even be declared. Cleanup that PHP puts there — closing a handle, releasing a
lock, flushing a buffer — becomes an explicit method the holder calls when it is actually done.

Two independent arguments each suffice. There is no sound place to report a throw: every call returns
a checked status to a caller, and a destructor has no call site — it fires from wherever a refcount
happens to reach zero, which is an assignment, a loop step, or a return that has nothing to do with
the failure. And it would undo the wholesale heap drop, whose whole point is not walking live objects
individually at request end.

One thing does run when a refcount reaches zero, and it is not a destructor: a generator suspended
inside a `try ... finally` is resumed in a return-like mode so the `finally` runs, matching PHP
([`iteration/generators`](/docs/rules/iteration/#generators "A body that yields is a generator, and calling one returns a state object without running a line of it")). Nothing is declared, no name is recognized, and the release resumes a
frame the program had already entered. A throw escaping such a `finally` is discarded, since a release
is exactly the site with nowhere to report one.

What it costs is real: no RAII, so a caller who forgets an explicit `close()` gets nothing — PHP's
`__destruct` was an unreliable safety net, but it was a net.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>__destruct</code> does not exist in any form, and nothing user-declared runs when a refcount reaches zero</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/no-magic/#no-magic-methods" title="No method name changes a class's behaviour by being present; every PHP magic method is replaced by a declared interface or gone"><code>classes/no-magic-methods</code></a> <a href="/docs/rules/iteration/#generators" title="A body that yields is a generator, and calling one returns a state object without running a line of it"><code>iteration/generators</code></a> <a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0028.md">record 0028</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/every-magic-method-name-this-adr-closes-is-unspellable.nvst"><code>tests/conformance/reject/every-magic-method-name-this-adr-closes-is-unspellable.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/iter/an-abandoned-generator-runs-the-finally-it-is-suspended-inside.nvst"><code>tests/conformance/iter/an-abandoned-generator-runs-the-finally-it-is-suspended-inside.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-debug-hook">

## A debug dump shows a class's real declared properties and their real values, and no class can change that

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-debug-hook"><code>classes/no-debug-hook</code></a>
</div>

A debug dump shows a class's real declared properties and their real current values. There is no hook
to filter, rename or synthesize what appears: `__debugInfo` does not exist and cannot be declared, so
what you declare is what a dump shows. What the view looks like belongs to the diagnostic record
([`errors/debug-dump`](/docs/rules/errors/diagnostics-and-logging/#debug-dump "A dump goes to the log, and reaches a response body only in development")); the rule here is the absence of the hook.

The alternative would be a customization surface with no unsafe default to close — the dump is a
fixed, built-in operation a developer did not write and cannot call with attacker-influenced arguments
to bypass anything.

That is unrelated to reflection, which enforces the same visibility check an ordinary access would:
one is a built-in view, the other a call site a script constructs, and they have different threat
models. A `secret`-typed property is redacted wherever it is dumped, which is the qualifier's rule and
not an exception to this one.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>__debugInfo</code> does not exist, so a dump cannot be filtered, renamed or synthesized by the class being dumped</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/no-magic/#no-magic-methods" title="No method name changes a class's behaviour by being present; every PHP magic method is replaced by a declared interface or gone"><code>classes/no-magic-methods</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#debug-dump" title="A dump goes to the log, and reaches a response body only in development"><code>errors/debug-dump</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#no-render-hook" title="A class cannot change how it is dumped"><code>errors/no-render-hook</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0028.md">record 0028</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0092.md">record 0092</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0019.md">record 0019</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-dump-renders-one-record-through-one-plaintext-view.nvst"><code>tests/conformance/core/a-dump-renders-one-record-through-one-plaintext-view.nvst</code></a></dd></div></dl>

</div>
