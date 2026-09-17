---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Where Novis diverges"
description: "Every departure from PHP is listed, and each exists because PHP left a binding untyped. Absent storage is never a zero value."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/php-migration/
  label: "Migrating from PHP"
next:
  link: /docs/rules/php-migration/converting-and-completing/
  label: "Converting and completing PHP"
---

<p class="nv-section-lead">Every departure from PHP is listed, and each exists because PHP left a binding untyped. Absent storage is never a zero value.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">11</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">6</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">5</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">11</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#every-divergence-is-deliberate-and-listed">Every departure from PHP's observable behaviour is listed as a divergence, and each exists only because PHP left a binding untyped</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-declared-type-answers-before-the-program-runs"><code>-&gt;</code> on a receiver that can hold no object, and <code>instanceof</code> on a subject that can hold none, are refused where they are written</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#absent-storage-is-never-a-zero-value">Absent storage never reads as a zero value: an unassigned variable and <code>[]</code> in a read position are refused, and an absent key throws</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-element-write-needs-storage-to-write-back-into">An element write through a temporary is refused, because the separated copy has nowhere to be written back</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-body-never-falls-off-its-end">A non-<code>void</code> body hands back a value at every exit; nothing returns <code>null</code> for a declaration it did not make</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-return-leaves-a-finally">A <code>return</code> never leaves a <code>finally</code> block, and neither does a <code>break</code> or <code>continue</code> whose target lies outside it</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-constructor-return-carries-no-value">A constructor's <code>return</code> carries no value; a bare <code>return;</code> may still leave early</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-readonly-property-declares-no-default">A <code>readonly</code> property declares no default; a value known at the declaration is a <code>const</code></a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-partial-application">Partial function application is not adopted; the closure literal already spells it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#let-and-is-are-reserved"><code>let</code> and <code>is</code> cannot name anything; <code>var</code> declares, <code>as</code> converts, and <code>is</code> is the type test</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#is-takes-pattern-matchings-type-patterns"><code>is</code> implements the type-pattern half of PHP's Pattern Matching RFC and reserves every other row of it</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="every-divergence-is-deliberate-and-listed">

## Every departure from PHP's observable behaviour is listed as a divergence, and each exists only because PHP left a binding untyped

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#every-divergence-is-deliberate-and-listed"><code>php-migration/every-divergence-is-deliberate-and-listed</code></a>
</div>

PHP-compatible observable behaviour is the second priority, so a departure from it is never discovered
later: every one is listed, PHP's behaviour beside Novis's. Each is reachable in PHP only *because* a
binding somewhere is untyped, and each is what a declared type answers instead.

The list, by where each is stated: array keys are always `string` ([`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type")); `int` and
`uint` are distinct types, reported distinctly ([`types/uint`](/docs/rules/types/declarations-and-numbers/#uint "uint is a distinct unsigned 64-bit integer, and it costs no bytes a value did not already hold")); every binding is declared and its
type fixed, `settype()` rejected ([`types/declaration`](/docs/rules/types/declarations-and-numbers/#declaration "Every binding declares its type, and no binding's type ever changes")); `(int)"abc"` is refused as syntax, `"abc"
as int` throws and `"abc" as ?int` is `null` ([`types/no-legacy-cast`](/docs/rules/types/unions-and-conversion/#no-legacy-cast "PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it"), [`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws"));
integer overflow, a fractional value where an integer is wanted, and an `int` too wide for a `float`
all throw ([`types/arithmetic`](/docs/rules/types/declarations-and-numbers/#arithmetic "An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting")); a return type is mandatory on every function, method and closure,
`void` or `never` stated when there is no value ([`types/declaration`](/docs/rules/types/declarations-and-numbers/#declaration "Every binding declares its type, and no binding's type ever changes")); absent storage never reads
as a zero value ([`php-migration/absent-storage-is-never-a-zero-value`](/docs/rules/php-migration/divergences/#absent-storage-is-never-a-zero-value "Absent storage never reads as a zero value: an unassigned variable and [] in a read position are refused, and an absent key throws")); a declared type answers
`->` and `instanceof` before the program runs
([`php-migration/a-declared-type-answers-before-the-program-runs`](/docs/rules/php-migration/divergences/#a-declared-type-answers-before-the-program-runs "-> on a receiver that can hold no object, and instanceof on a subject that can hold none, are refused where they are written")); an element write needs storage
to write back into ([`php-migration/an-element-write-needs-storage-to-write-back-into`](/docs/rules/php-migration/divergences/#an-element-write-needs-storage-to-write-back-into "An element write through a temporary is refused, because the separated copy has nowhere to be written back")); a body
never falls off its end ([`php-migration/a-body-never-falls-off-its-end`](/docs/rules/php-migration/divergences/#a-body-never-falls-off-its-end "A non-void body hands back a value at every exit; nothing returns null for a declaration it did not make")); and a spread carrying a
string key after an integer-looking one is accepted where PHP fatals, the variadic tail being the
array itself ([`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type")).

The consequence to plan around: the imported `.phpt` corpus passes at a **structurally lower** rate
than a compatibility-first design would, and a failure in one of these classes is intentional
divergence, not a bug. The tracked number distinguishes the two, or it reads as regression.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The imported <code>.phpt</code> corpus passes at a structurally lower rate than a compatibility-first design would, and a failure in a listed class is intentional rather than a regression</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#declaration" title="Every binding declares its type, and no binding's type ever changes"><code>types/declaration</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/unions-and-conversion/#no-legacy-cast" title="PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it"><code>types/no-legacy-cast</code></a> <a href="/docs/rules/types/declarations-and-numbers/#arithmetic" title="An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting"><code>types/arithmetic</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a> <a href="/docs/rules/types/declarations-and-numbers/#uint" title="uint is a distinct unsigned 64-bit integer, and it costs no bytes a value did not already hold"><code>types/uint</code></a> <a href="/docs/rules/php-migration/divergences/#absent-storage-is-never-a-zero-value" title="Absent storage never reads as a zero value: an unassigned variable and [] in a read position are refused, and an absent key throws"><code>php-migration/absent-storage-is-never-a-zero-value</code></a> <a href="/docs/rules/php-migration/divergences/#a-declared-type-answers-before-the-program-runs" title="-&gt; on a receiver that can hold no object, and instanceof on a subject that can hold none, are refused where they are written"><code>php-migration/a-declared-type-answers-before-the-program-runs</code></a> <a href="/docs/rules/php-migration/divergences/#an-element-write-needs-storage-to-write-back-into" title="An element write through a temporary is refused, because the separated copy has nowhere to be written back"><code>php-migration/an-element-write-needs-storage-to-write-back-into</code></a> <a href="/docs/rules/php-migration/divergences/#a-body-never-falls-off-its-end" title="A non-void body hands back a value at every exit; nothing returns null for a declaration it did not make"><code>php-migration/a-body-never-falls-off-its-end</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-declared-type-answers-before-the-program-runs">

## `->` on a receiver that can hold no object, and `instanceof` on a subject that can hold none, are refused where they are written

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-declared-type-answers-before-the-program-runs"><code>php-migration/a-declared-type-answers-before-the-program-runs</code></a>
</div>

`$i->name` where `$i` is declared `int` is refused where it is written (`E0495`); PHP warns *"Attempt
to read property"* and yields `null`. A union naming no single class takes the same code, having no
one property set to resolve against. `1 instanceof Box`, or `instanceof` over a declared scalar, an
`array<T>`, an enum or a union naming no class, is refused the same way (`E0497`); PHP answers
`false`, having no declaration to read. The subject's declaration has ruled the question out, so the
test is dead code that reads as a live one — the same call
[`expressions/disjoint-comparison-refused`](/docs/rules/expressions/truthiness-and-equality/#disjoint-comparison-refused "Comparing two types that no single value inhabits is a compile error") makes for `==` over two statically disjoint types.

**What is refused is a subject that can hold no object at all, not a test whose answer is knowable**,
and the two are easy to run together when reading this rule quickly. `$leaf instanceof Leaf` where
`$leaf` is declared `Leaf` is statically true and **accepted**; so is `$leaf instanceof Other` for an
unrelated class, which is statically false. `instanceof` is refused only where it is *inapplicable* —
it needs a class to test against and a scalar has none. The general test that is applicable to every
subject, and so refuses none, is `is` ([`types/type-test`](/docs/rules/types/unions-and-conversion/#type-test "$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable")).

`mixed` is the exception and keeps PHP's timing: it is the one unchecked position, so `$m->name`
defers to [`types/erased-member-access`](/docs/rules/types/objects-and-shapes/#erased-member-access "A member reached through an erased receiver is answered at run time, and a failure is a throw")'s name-keyed fetch, which throws — in PHP's own wording —
for a receiver that turns out not to be an object and for a name its class does not carry. `mixed`,
`object`, a shape and any union holding a class keep the run-time `instanceof` test, and every
non-object tag answers `false` there exactly as PHP does.

The class side is not a divergence. PHP's dynamic `$x instanceof $name` is spelled over a class
reference — `$x instanceof $cls`, where `$cls` is a `class<T>` ([`types/class-reference`](/docs/rules/types/objects-and-shapes/#class-reference "class<T> is a type whose value is a class descriptor, and as is its only source")) — and
tests the class that value holds; a bare `string` on the right is `E0496`, the name having been
checked at the `as` that produced the reference, not at the test.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>$i-&gt;name</code> on an <code>int</code> and <code>1 instanceof Box</code> are compile errors (<code>E0495</code>, <code>E0497</code>) rather than a warning yielding <code>null</code> and a <code>false</code>; only <code>mixed</code>, <code>object</code>, a shape and a union holding a class keep PHP's run-time answer</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/objects-and-shapes/#erased-member-access" title="A member reached through an erased receiver is answered at run time, and a failure is a throw"><code>types/erased-member-access</code></a> <a href="/docs/rules/types/unions-and-conversion/#narrowing" title="Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it"><code>types/narrowing</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a> <a href="/docs/rules/types/objects-and-shapes/#class-reference" title="class&lt;T&gt; is a type whose value is a class descriptor, and as is its only source"><code>types/class-reference</code></a> <a href="/docs/rules/types/unions-and-conversion/#type-test" title="$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable"><code>types/type-test</code></a> <a href="/docs/rules/expressions/truthiness-and-equality/#disjoint-comparison-refused" title="Comparing two types that no single value inhabits is a compile error"><code>expressions/disjoint-comparison-refused</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0150.md">record 0150</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-property-through-a-receiver-that-holds-no-object-is-a-compile-error.nvst"><code>tests/conformance/lang/a-property-through-a-receiver-that-holds-no-object-is-a-compile-error.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/instanceof-refuses-a-subject-that-can-hold-no-object.nvst"><code>tests/conformance/lang/instanceof-refuses-a-subject-that-can-hold-no-object.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/class/instanceof-through-an-erased-subject-answers-every-tag.nvst"><code>tests/conformance/class/instanceof-through-an-erased-subject-answers-every-tag.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="absent-storage-is-never-a-zero-value">

## Absent storage never reads as a zero value: an unassigned variable and `[]` in a read position are refused, and an absent key throws

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#absent-storage-is-never-a-zero-value"><code>php-migration/absent-storage-is-never-a-zero-value</code></a>
</div>

PHP fills a hole with a zero value and warns. Here a hole is answered before the program runs or
thrown at: reading a variable that is not definitely assigned is an error at check time; `$a[]`
anywhere but as an assignment target is refused (`E0481`), so `$a[] .= "x"` — which appends in PHP
only because the element that is not there yet reads as `""` — does not compile; and reading an
absent array key **throws**, because there is no `null` to put in an `array<string>`, so the rule
holds at run time too. A stored `null` in an `array<?T>` is not an absent key and reads back
unchanged.

`$a["k"] ?? $d` is the one exception and is PHP-identical: `??` means "absent or `null`, without the
warning", so the guarded read yields `$d` rather than throwing — refusing there would refuse the
spelling PHP offers for exactly this, and the throw is what makes it worth writing. The guard covers
every level of the chain under it, so `$a["k"]["j"] ?? $d` yields `$d` for an absent key at either
depth, and a `null` base needs no `!= null` test in that one position. `isset` and `empty` are the
same guarded read and answer rather than throw.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Reading an undefined variable or an absent array key does not warn and yield <code>null</code>, and <code>$a[] .= &quot;x&quot;</code> does not append; <code>??</code>, <code>isset</code> and <code>empty</code> keep PHP's answer</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a> <a href="/docs/rules/types/declarations-and-numbers/#declaration" title="Every binding declares its type, and no binding's type ever changes"><code>types/declaration</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#mixed-subscript" title="A subscript through a mixed is the tag's question; every other base answers it where it is written"><code>types/mixed-subscript</code></a> <a href="/docs/rules/classes/declaring-a-class/#no-undefined-value" title="There is no undefined: a property that may legitimately hold no value is ?T"><code>classes/no-undefined-value</code></a> <a href="/docs/rules/classes/declaring-a-class/#an-unwritten-property-read-throws" title="Reading a property that has never been written throws a catchable error and never yields a value"><code>classes/an-unwritten-property-read-throws</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/reading-an-absent-array-key-throws.nvst"><code>tests/conformance/lang/reading-an-absent-array-key-throws.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-coalesce-guards-every-level-of-a-subscript-chain.nvst"><code>tests/conformance/lang/a-coalesce-guards-every-level-of-a-subscript-chain.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/array/an-append-in-a-read-position-is-refused.nvst"><code>tests/conformance/array/an-append-in-a-read-position-is-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/isset-answers-a-null-test-over-every-storage-shape.nvst"><code>tests/conformance/lang/isset-answers-a-null-test-over-every-storage-shape.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-element-write-needs-storage-to-write-back-into">

## An element write through a temporary is refused, because the separated copy has nowhere to be written back

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-element-write-needs-storage-to-write-back-into"><code>php-migration/an-element-write-needs-storage-to-write-back-into</code></a>
</div>

Copy-on-write separates the array before the element is written, and the separated copy has to land
back in whatever held the array. A temporary — a call's result, an array literal, a conditional —
holds it nowhere, so `f()[0] = 2` is refused where it is written (`E0700`). In PHP the write lands
in the temporary and is discarded: no warning, no notice, nothing observable. The only statement this
costs is one that could not have done anything, so no program that ran is lost.

Every root that *is* storage is unaffected: a local, a property, a static property — and the
receiver under a property is evaluated exactly once, PHP's own count, however deep the chain and
whichever spelling writes it. A temporary receiver's property is included, and that is the one
direction this runs the other way: PHP 8.5.9 refuses `(new Box())->rows[0] = 2` at compile time
(*"Cannot use temporary expression in write context"*) while accepting `make()->rows[0] = 2`, and
Novis accepts both, the field being a slot in a heap object either way. Accepting where PHP refuses
loses no program that ran. Parentheses are transparent on both sides — `($a)[0] = 2` writes `$a[0]`
here exactly as it does in PHP.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>f()[0] = 2</code> is <code>E0700</code> rather than a silently discarded write, and <code>(new Box())-&gt;rows[0] = 2</code> is accepted where PHP 8.5 refuses it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-element-write-needs-a-place-to-write-back-into.nvst"><code>tests/conformance/lang/an-element-write-needs-a-place-to-write-back-into.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-element-writes-holder-is-evaluated-once.nvst"><code>tests/conformance/lang/an-element-writes-holder-is-evaluated-once.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-body-never-falls-off-its-end">

## A non-`void` body hands back a value at every exit; nothing returns `null` for a declaration it did not make

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-body-never-falls-off-its-end"><code>php-migration/a-body-never-falls-off-its-end</code></a>
</div>

In PHP a function declared `: int` whose body reaches its own end returns `null`, whatever it
declared. Here that path writes no value at all, and there is no implicit conversion by which `null`
becomes the declared type, so accepting it would change what a binding holds behind its declaration —
the one thing the type system never does. The body is refused where it is written (`E0739`).

A written `return;` is the same promise broken at the other exit, and is refused the same way
(`E0822`); PHP raises a `TypeError` there rather than answering `null`. Both are read over every
block body checked against a declared type — a method's, a `get` hook's and a block-bodied `fn`'s
alike.

`void`, and a declaration that writes no return type at all (a constructor), promise nothing and are
untouched; so is a generator, whose body [`iteration/generators`](/docs/rules/iteration/#generators "A body that yields is a generator, and calling one returns a state object without running a line of it") leaves no return value to
produce. The analysis is asymmetric on purpose, and `nvs_types::returns` owns it: a `while (true)`
with no `break`, a `switch` with a `default`, a `try` every path of which exits, and a body that
always throws are all exits, and every shape it cannot prove reaches the end is treated as one — so
the refusal costs no program that ran. Whether a written `return;` is legal asks none of that: the
declared type is the whole answer, which is why a `never` body is refused one and is still never
asked about the falling-off path.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A body that falls off its end is <code>E0739</code> and a written <code>return;</code> under a non-<code>void</code> declaration is <code>E0822</code>, each where it stands rather than a <code>null</code> return or a <code>TypeError</code> at run time</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#declaration" title="Every binding declares its type, and no binding's type ever changes"><code>types/declaration</code></a> <a href="/docs/rules/iteration/#generators" title="A body that yields is a generator, and calling one returns a state object without running a line of it"><code>iteration/generators</code></a> <a href="/docs/rules/classes/declaring-a-class/#constructor-is-a-method-named-constructor" title="A constructor is an ordinary method named constructor, and __construct does not compile"><code>classes/constructor-is-a-method-named-constructor</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/error/a-method-that-only-throws-satisfies-its-return-type.nvst"><code>tests/conformance/error/a-method-that-only-throws-satisfies-its-return-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-bare-return-is-refused-where-the-declaration-is-not-void.nvst"><code>tests/conformance/reject/a-bare-return-is-refused-where-the-declaration-is-not-void.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-hook-and-a-block-bodied-closure-owe-a-value-at-every-exit.nvst"><code>tests/conformance/reject/a-hook-and-a-block-bodied-closure-owe-a-value-at-every-exit.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-return-leaves-a-finally">

## A `return` never leaves a `finally` block, and neither does a `break` or `continue` whose target lies outside it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-return-leaves-a-finally"><code>php-migration/no-return-leaves-a-finally</code></a>
</div>

`return` inside `finally` does not compile. It would replace whatever the region was leaving with —
including a throw in flight, which would be discarded with no `catch` anywhere in the program — and
it is the only construct in PHP where an unhandled exception vanishes without a handler. PHP 8.6
deprecates the spelling for removal on exactly that argument. The diagnostic names the two rewrites:
change the result in a `catch`, or after the region.

`break` and `continue` whose target lies outside the `finally` are refused on the same grounds; a
loop wholly inside the block keeps both.

This is the one PHP 8.6 refusal with no mechanical rewrite. Whether the override was a bug (usually)
or intent is a human's call, so `nvs convert` points at the line and rewrites nothing.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>return</code> inside <code>finally</code> does not compile, instead of overriding the region's result and discarding a throw in flight (deprecated in 8.6)</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/php-migration/converting-and-completing/#a-deprecation-is-a-refusal" title="What PHP deprecates for removal is refused at compile time, never phased in behind a warning"><code>php-migration/a-deprecation-is-a-refusal</code></a> <a href="/docs/rules/php-migration/divergences/#a-body-never-falls-off-its-end" title="A non-void body hands back a value at every exit; nothing returns null for a declaration it did not make"><code>php-migration/a-body-never-falls-off-its-end</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0124.md">record 0124</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-constructor-return-carries-no-value">

## A constructor's `return` carries no value; a bare `return;` may still leave early

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-constructor-return-carries-no-value"><code>php-migration/a-constructor-return-carries-no-value</code></a>
</div>

`return $value;` inside a `constructor` does not compile: the object under construction is the
result, and nothing else can be. A bare `return;` remains legal as an early exit, provided every
property is definitely assigned on that path — [`classes/definite-property-initialization`](/docs/rules/classes/declaring-a-class/#definite-property-initialization "Every property a class declares is definitely assigned on every path out of every constructor") keeps
checking it, unchanged.

PHP 8.6 deprecates the value-returning form on the same "never made sense" argument. The rewrite is
mechanical — drop the value — and `nvs convert` applies it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>return $value;</code> in a constructor does not compile (deprecated in 8.6)</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/classes/declaring-a-class/#constructor-is-a-method-named-constructor" title="A constructor is an ordinary method named constructor, and __construct does not compile"><code>classes/constructor-is-a-method-named-constructor</code></a> <a href="/docs/rules/classes/declaring-a-class/#definite-property-initialization" title="Every property a class declares is definitely assigned on every path out of every constructor"><code>classes/definite-property-initialization</code></a> <a href="/docs/rules/php-migration/converting-and-completing/#a-deprecation-is-a-refusal" title="What PHP deprecates for removal is refused at compile time, never phased in behind a warning"><code>php-migration/a-deprecation-is-a-refusal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0124.md">record 0124</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-readonly-property-declares-no-default">

## A `readonly` property declares no default; a value known at the declaration is a `const`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-readonly-property-declares-no-default"><code>php-migration/a-readonly-property-declares-no-default</code></a>
</div>

`readonly` with an initializer is refused where it is written. `readonly`'s contract is one
assignment, during construction, in the declaring class's own `constructor` — a property whose
single assignment is its own declaration-site default is a per-instance constant, and a value known
at the declaration already has a spelling: `const` ([`types/class-constant`](/docs/rules/types/objects-and-shapes/#class-constant "::class answers the class the value is, and folding is an optimization of that")). The diagnostic
names it, and names dropping `readonly` as the other fix. Letting the default stand as the one
assignment would make a property spell what `const` spells — two names for one thing, the pattern
[`statements/nothing-gets-a-second-name`](/docs/rules/statements/names-and-require/#nothing-gets-a-second-name "A declaration is reachable under exactly the name it was declared with") refuses.

PHP 8.6 allows the combination, chiefly for hooks in interfaces. This reopens only if that form
becomes idiomatic in the corpora `nvs convert` targets — the trigger is conversion pressure, not the
RFC landing. Until then the rewrite is mechanical: move the default into the constructor, or make it
a `const`.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>readonly int $n = 1;</code> is refused where PHP 8.6 allows it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/objects-and-shapes/#class-constant" title="::class answers the class the value is, and folding is an optimization of that"><code>types/class-constant</code></a> <a href="/docs/rules/classes/properties/#lateinit" title="A lateinit property is assigned after the constructor rather than inside it"><code>classes/lateinit</code></a> <a href="/docs/rules/classes/declaring-a-class/#promotion-is-constructor-only" title="A visibility keyword on a parameter promotes it to a property only in the constructor"><code>classes/promotion-is-constructor-only</code></a> <a href="/docs/rules/statements/names-and-require/#nothing-gets-a-second-name" title="A declaration is reachable under exactly the name it was declared with"><code>statements/nothing-gets-a-second-name</code></a> <a href="/docs/rules/php-migration/converting-and-completing/#a-deprecation-is-a-refusal" title="What PHP deprecates for removal is refused at compile time, never phased in behind a warning"><code>php-migration/a-deprecation-is-a-refusal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0124.md">record 0124</a></dd></div></dl>

</div>

<div class="nv-rule" id="no-partial-application">

## Partial function application is not adopted; the closure literal already spells it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-partial-application"><code>php-migration/no-partial-application</code></a>
</div>

`f(?, $x)` and every application-with-holes shape do not parse. The value it would produce is
exactly what the callable machinery reduced to one shape ([`types/callable-is-a-closure`](/docs/rules/types/closures/#callable-is-a-closure "callable is satisfied by a closure and by nothing else, and no object is callable")), and
the closure literal already spells every partial application: `fn($a) => f($a, $x)`
([`types/closure-literal`](/docs/rules/types/closures/#closure-literal "fn is the only closure literal, with an expression body or a block body")). A second closure-producing spelling, for no capability `fn` lacks,
is what was rejected.

`nvs convert` rewrites a PHP 8.6 `?` placeholder into that wrapper mechanically — each `?` becomes a
fresh parameter, in order. The pipeline operator is unaffected: its `$_` is a parse-time
substitution, not an application ([`expressions/pipeline-substitution`](/docs/rules/expressions/pipeline-and-catch/#pipeline-substitution "|> substitutes one hole at parse time, and has no run-time representation")), and the hole
diagnostic ([`expressions/pipeline-hole-once`](/docs/rules/expressions/pipeline-and-catch/#pipeline-hole-once "The hole is $_, it appears exactly once on a right side, and nowhere else at all")) already teaches the difference.

This reopens only on conversion pressure — widespread `?` placeholders in real conversion targets —
at which point the wrapper either suffices or measurably bloats output.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP 8.6's <code>f(?, $x)</code> does not parse; write <code>fn($a) =&gt; f($a, $x)</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/closures/#closure-literal" title="fn is the only closure literal, with an expression body or a block body"><code>types/closure-literal</code></a> <a href="/docs/rules/types/closures/#callable-is-a-closure" title="callable is satisfied by a closure and by nothing else, and no object is callable"><code>types/callable-is-a-closure</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#first-class-callable-syntax" title="Name(...) is the only spelling that takes a reference to a declared member"><code>expressions/first-class-callable-syntax</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-substitution" title="|&gt; substitutes one hole at parse time, and has no run-time representation"><code>expressions/pipeline-substitution</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-hole-once" title="The hole is $_, it appears exactly once on a right side, and nowhere else at all"><code>expressions/pipeline-hole-once</code></a> <a href="/docs/rules/php-migration/converting-and-completing/#a-deprecation-is-a-refusal" title="What PHP deprecates for removal is refused at compile time, never phased in behind a warning"><code>php-migration/a-deprecation-is-a-refusal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0124.md">record 0124</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0027.md">record 0027</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0031.md">record 0031</a></dd></div></dl>

</div>

<div class="nv-rule" id="let-and-is-are-reserved">

## `let` and `is` cannot name anything; `var` declares, `as` converts, and `is` is the type test

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#let-and-is-are-reserved"><code>php-migration/let-and-is-are-reserved</code></a>
</div>

Neither `let` nor `is` may name a class, interface, trait, enum, constant, function or parameter. PHP
8.6 deprecates both as identifiers; Novis, with no published corpus, refuses them outright and both at
once, because the cost of reserving now is near zero while the cost of taking either back later is a
breaking rename. A converted program renames any `let` or `is` it used as a name, and the rewrite is
mechanical.

**The two are reserved for unrelated reasons, and only one of them still has no construct.** PHP's
*Deprecations for PHP 8.6* RFC gives each its own motivation: `let` for the block-scoping construct,
whose own RFC was declined, and `is` for the Pattern Matching RFC, by name.

- **`let` is the empty kind** — the family of `eval`, `goto` and `list`, where the spelling is held
  and nothing is behind it, so nothing a user wrote has to be renamed out from under a future
  decision.
- **`is` is not.** It is the type test, `$x is T` ([`types/type-test`](/docs/rules/types/unions-and-conversion/#type-test "$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable")), which takes the settled
  half of the RFC PHP reserved the word for and leaves the rest of it unclaimed
  ([`php-migration/is-takes-pattern-matchings-type-patterns`](/docs/rules/php-migration/divergences/#is-takes-pattern-matchings-type-patterns "is implements the type-pattern half of PHP's Pattern Matching RFC and reserves every other row of it")).

The diagnostics name the living spellings: `var` declares an inferred local
([`types/var-inference`](/docs/rules/types/declarations-and-numbers/#var-inference "var takes a local's type from its initializer and fixes it there for good")), `is` and `instanceof` test ([`types/narrowing`](/docs/rules/types/unions-and-conversion/#narrowing "Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it")) and `as` converts
([`expressions/nullable-conversion`](/docs/rules/expressions/conversion-and-intrinsics/#nullable-conversion "expr as ?T yields the converted value or null, and never throws")). Like every reserved word, both match in lower case only
([`classes/reserved-spellings-are-lower-case`](/docs/rules/classes/declaring-a-class/#reserved-spellings-are-lower-case "Keywords, contextual keywords and the <?nvs tag are lower case and nothing else")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>let</code> and <code>is</code> cannot name anything (deprecated as identifiers in 8.6), so a converted program renames them</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#var-inference" title="var takes a local's type from its initializer and fixes it there for good"><code>types/var-inference</code></a> <a href="/docs/rules/types/unions-and-conversion/#narrowing" title="Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it"><code>types/narrowing</code></a> <a href="/docs/rules/types/unions-and-conversion/#type-test" title="$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable"><code>types/type-test</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#nullable-conversion" title="expr as ?T yields the converted value or null, and never throws"><code>expressions/nullable-conversion</code></a> <a href="/docs/rules/classes/declaring-a-class/#reserved-spellings-are-lower-case" title="Keywords, contextual keywords and the &lt;?nvs tag are lower case and nothing else"><code>classes/reserved-spellings-are-lower-case</code></a> <a href="/docs/rules/security/closed-doors/#no-eval" title="There is no eval, and no Core member compiles a string produced at run time"><code>security/no-eval</code></a> <a href="/docs/rules/php-migration/converting-and-completing/#a-deprecation-is-a-refusal" title="What PHP deprecates for removal is refused at compile time, never phased in behind a warning"><code>php-migration/a-deprecation-is-a-refusal</code></a> <a href="/docs/rules/php-migration/divergences/#is-takes-pattern-matchings-type-patterns" title="is implements the type-pattern half of PHP's Pattern Matching RFC and reserves every other row of it"><code>php-migration/is-takes-pattern-matchings-type-patterns</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0124.md">record 0124</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0150.md">record 0150</a></dd></div></dl>

</div>

<div class="nv-rule" id="is-takes-pattern-matchings-type-patterns">

## `is` implements the type-pattern half of PHP's Pattern Matching RFC and reserves every other row of it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#is-takes-pattern-matchings-type-patterns"><code>php-migration/is-takes-pattern-matchings-type-patterns</code></a>
</div>

Novis's `is` implements the **type-pattern half** of PHP's Pattern Matching RFC and deliberately
claims none of the rest, because PHP reserved the spelling for that RFC by name and has not yet voted
on it. PHP 8.6 deprecates `is` as an identifier, and the *Deprecations for PHP 8.6* RFC gives one
motivation and no other: to reserve it for pattern matching. That is a claim on the spelling, so
Novis's job is to be a subset that stays true rather than a superset that has to be taken back.

[`types/type-test`](/docs/rules/types/unions-and-conversion/#type-test "$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable") is the operator and owns its table. This rule owns the **relationship**: what
we took, what we left, and what a future session must read before claiming any of it.

## Taken, and identical to the RFC

Type patterns (`is string`, `is Request`, `is ?array`, `is int|float`, `is iterable`, `is true`,
`is mixed`), unions and intersections under DNF, literal patterns (`is 5`, `is 'yay'`,
`is "beep"|"boop"`, `is null`), and class-constant patterns (`is MyEnum::Case`, `is self::Wild`).
Every one of those is already a `Type` in our grammar, and the RFC's semantics for them are strict —
the same strictness `is_int()` has — which is what `is` answers with.

One row is not taken: `is 3.14`. [`types/literal-types`](/docs/rules/types/text-and-literal-types/#literal-types "A string or int literal is its own type, and a union of them is a closed set") has no float literal type on purpose, so
the test is refused rather than answered differently. A refusal is the safe side of a divergence.

## Reserved, and not designed

Variable binding and capture (`$p is Point(x: 3, y: $y)`), object destructuring, array sequence and
associative patterns, comparison patterns (`is >5 & <10`), pinning (`^$var`), and `match ($x) is {…}`.
Each parses to a refusal naming itself as reserved.

These are precisely the rows the RFC has **not** settled: it is in discussion rather than voted, and
the binding shorthand and the placement of `is` inside `match()` are both under open objection. A
meaning invented for any of them now is a meaning that has to be taken back, and the taking back is a
breaking change to a spelling that already compiles.

## Why `$x is $cls` is refused rather than useful

A bare variable on the right of `is` is a **capture** in PHP's grammar — it binds and always matches —
and the pinned form `^$var` compares with identity, which asks whether the subject *is* that descriptor
rather than an instance of it. So PHP has no dynamic class test through `is`, and the RFC says `is` and
`instanceof` coexist rather than one replacing the other.

Novis could give `$x is $cls` the dynamic meaning, since it has no binding patterns. It does not,
because that is the worst divergence available: the same line would mean two different things in the
two languages and compile silently in both. `$x instanceof $cls` is the dynamic class test
([`types/class-reference-sites`](/docs/rules/types/objects-and-shapes/#class-reference-sites "Three sites accept a class<T>, and every other operand is still refused there")), and it is the one thing `is` structurally cannot express.

## What this rule is for

Before any session claims a reserved row above, it re-reads the RFC as accepted rather than as
described here — the contested parts are the ones most likely to have moved — and records the result.
[`types/type-test`](/docs/rules/types/unions-and-conversion/#type-test "$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable")'s three refusals and this rule's reserved list are one decision seen from two
sides, and neither is edited without the other.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP reserved <code>is</code> for pattern matching and has not voted it; Novis ships the settled type-pattern half now, refuses <code>is 3.14</code> for want of a float literal type, and leaves binding, destructuring, array patterns, comparison patterns and pinning unclaimed</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#type-test" title="$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable"><code>types/type-test</code></a> <a href="/docs/rules/types/text-and-literal-types/#literal-types" title="A string or int literal is its own type, and a union of them is a closed set"><code>types/literal-types</code></a> <a href="/docs/rules/types/objects-and-shapes/#class-reference-sites" title="Three sites accept a class&lt;T&gt;, and every other operand is still refused there"><code>types/class-reference-sites</code></a> <a href="/docs/rules/php-migration/divergences/#let-and-is-are-reserved" title="let and is cannot name anything; var declares, as converts, and is is the type test"><code>php-migration/let-and-is-are-reserved</code></a> <a href="/docs/rules/php-migration/divergences/#every-divergence-is-deliberate-and-listed" title="Every departure from PHP's observable behaviour is listed as a divergence, and each exists only because PHP left a binding untyped"><code>php-migration/every-divergence-is-deliberate-and-listed</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0150.md">record 0150</a></dd></div></dl>

</div>
