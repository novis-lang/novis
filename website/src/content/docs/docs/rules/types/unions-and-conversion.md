---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Unions, narrowing and conversion"
description: "What a union permits, the four spellings of narrowing, and the single conversion operator that replaced the cast."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/types/arrays-and-property-keys/
  label: "Arrays and property keys"
next:
  link: /docs/rules/types/closures/
  label: "Closures and callables"
---

<p class="nv-section-lead">What a union permits, the four spellings of narrowing, and the single conversion operator that replaced the cast.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">1</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">4</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#unions-and-mixed">A union permits only what every member permits, and <code>mixed</code> is the one position checked nowhere</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#type-test"><code>$x is T</code> tests whether a value holds a <code>T</code>, answers <code>bool</code>, and never refuses because the answer is knowable</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#narrowing">Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#conversion"><code>expr as T</code> is the only conversion, and it produces a <code>T</code> or throws</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-legacy-cast">PHP's <code>(T)expr</code> cast does not parse, and the diagnostic names the <code>as</code> that replaces it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#type-alias">A <code>type</code> alias is a transparent, compile-time-only synonym for a type expression</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#alias-is-never-a-bare-class">A <code>type</code> alias may not name a single bare class</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#class-scoped-alias">A <code>type</code> alias is also a member of a class, interface or enum, reached as <code>Owner::Name</code> and as a bare <code>Name</code> inside its owner</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="unions-and-mixed">

## A union permits only what every member permits, and `mixed` is the one position checked nowhere

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#unions-and-mixed"><code>types/unions-and-mixed</code></a>
</div>

A union permits only the operations valid for *every* member. Reaching a member's own operations means
narrowing ([`types/narrowing`](/docs/rules/types/unions-and-conversion/#narrowing "Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it")), and the spelling that narrows is the `is` operator
([`types/type-test`](/docs/rules/types/unions-and-conversion/#type-test "$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable")) — never an `is_int()`-style free function, because there are no free
functions, which is also why `Core\Validate` carries no numeric predicates. *Getting* a scalar out of
a union or out of `mixed` is a different question from testing for one: that is `as T`, which throws,
or `as ?T`, which yields `null`. `as` converts and so accepts what can be converted — `"7" as ?int` is
`7` — while `is` reads the representation and so answers `false` for the same value.

`mixed` is **not checked at all** — that is its entire job. It holds anything, every operation on it
is allowed, and every operation on it is resolved dynamically at runtime through the generic helper
path. That is PHP's semantics, exactly, at PHP's cost, which is the right pressure: the fast path is
the typed one. `Core\Reflect::typeOf` is the one type-introspection member, and it is meaningful only
on a `mixed`, because the checker already knows every other case.

`mixed` is where untrusted input lands, deliberately. `Core\Request::query()`/`::post()`,
`Core\Server::*`, `Core\Script::args()` and `Core\Json::decode`'s result are `array<mixed>`, or return
`mixed` per key, because input genuinely is untyped and pretending otherwise would be a lie in the
type:

```php
uint $id = Core\Request::query('id') as uint;     // throws on "abc", on "-1", on "" — never quietly 0
```

`mixed` never absorbs implicitly in the other direction: `int $n = $m;` where `$m` is `mixed` is a
diagnostic, not a runtime check.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>is_int()</code>-style free function to narrow with — the question is the <code>is</code> operator (<code>rule:types/type-test</code>) — and <code>mixed</code> never absorbs implicitly: <code>int $n = $m;</code> is a diagnostic rather than a runtime check</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#narrowing" title="Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it"><code>types/narrowing</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/declarations-and-numbers/#grammar" title="The type grammar is a closed set of atoms under unions and intersections"><code>types/grammar</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#mixed-subscript" title="A subscript through a mixed is the tag's question; every other base answers it where it is written"><code>types/mixed-subscript</code></a> <a href="/docs/rules/types/unions-and-conversion/#type-test" title="$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable"><code>types/type-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0012.md">record 0012</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0066.md">record 0066</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0150.md">record 0150</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-mixed-value-converts-to-a-scalar-on-request.nvst"><code>tests/conformance/lang/a-mixed-value-converts-to-a-scalar-on-request.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-mixed-value-answers-arithmetic-truth-and-a-subscript.nvst"><code>tests/conformance/lang/a-mixed-value-answers-arithmetic-truth-and-a-subscript.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-union-operand-renders-by-its-runtime-tag.nvst"><code>tests/conformance/lang/a-union-operand-renders-by-its-runtime-tag.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/narrowing.rs"><code>crates/nvs-types/tests/narrowing.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="type-test">

## `$x is T` tests whether a value holds a `T`, answers `bool`, and never refuses because the answer is knowable

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#type-test"><code>types/type-test</code></a>
</div>

`$x is T` asks whether a value currently holds a `T` and answers `bool`, for any type a value can
inhabit. It is **strict** — nothing is coerced on the way to the answer — and it is **total**: it
always compiles, and a result the checker can settle by itself folds to a constant rather than
becoming a diagnostic.

The right-hand side is a **type**, parsed by the same production `as` uses
([`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws")), not an expression. `is` is the question `as` was standing in for and never
answered: `"7" as ?int` is `7`, because `string → int` is a conversion row, while `"7" is int` is
`false`, because a `string` is not an `int`. One asks what a value can *become*, the other what it
*is*.

```php
mixed $m = Core\Request::query('id');
if ($m is int) {
    // $m is an int here — no `as`, no throw path
}
```

## What may appear on the right

| form | example |
|---|---|
| a scalar type | `$x is int`, `is uint`, `is float`, `is decimal`, `is string`, `is bytes`, `is bool`, `is null` |
| `object`, a class, an interface | `$x is object`, `$x is Request` |
| an array | `$x is array`, `$x is array<int>` |
| a shape | `$x is {x: int, y: int}` |
| `iterable`, `callable`, a callable signature | `$x is iterable`, `$x is callable` |
| a literal type | `$x is 5`, `$x is 'yay'`, `$x is true` |
| a class constant or an enum case | `$x is Mode::Read`, `$x is self::Wild` |
| an enum | `$x is Rank` |
| `mixed` | `$x is mixed` — always `true`, and the RFC's wildcard |
| a union or an intersection of any of those | `$x is int\|float`, `$x is Countable&Traversable` |

`array<T>` with a named element type, and a shape, each cost an O(n) walk — the same walk
`as array<T>` already performs, in a spelling that answers instead of throwing. An enum is its cases:
`$x is Rank` answers exactly what asking every `$x is Rank::Case` in turn answers, at one payload
compare per case, and [`enums/representation`](/docs/rules/enums/#representation "An enum value costs nothing beyond the integer it is") owns what that can tell apart — a value that reached
`mixed` is its backing integer, so a case is not distinguishable there from that integer nor from
another enum's case of the same value. Every other row is one tag comparison, or the descriptor walk
`instanceof` already does.

There is no float literal type to test against ([`types/literal-types`](/docs/rules/types/text-and-literal-types/#literal-types "A string or int literal is its own type, and a union of them is a closed set")), so `$x is 3.14` is
refused by that rule and not by this one.

## The three refusals

| refused | code | why |
|---|---|---|
| `$x is tainted string`, `is secret bytes` | `E0813` | [`security/tainted-qualifier`](/docs/rules/security/tainted-data/#tainted-qualifier "tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen") erases both qualifiers before codegen. There is no runtime bit, so the question has no answer — not merely a knowable one |
| `$x is void`, `$x is never` | `E0811` | no value inhabits either |
| `$x is $cls` | `E0812` | that is a *value*, not a type. `$x instanceof $cls` is the dynamic class test ([`types/class-reference-sites`](/docs/rules/types/objects-and-shapes/#class-reference-sites "Three sites accept a class<T>, and every other operand is still refused there")), and the spelling stays refused because PHP's grammar binds a variable there ([`php-migration/is-takes-pattern-matchings-type-patterns`](/docs/rules/php-migration/divergences/#is-takes-pattern-matchings-type-patterns "is implements the type-pattern half of PHP's Pattern Matching RFC and reserves every other row of it")) |

Nothing else is refused. In particular a test whose answer the declaration already settles is **not**:
`int $n; $n is int` compiles and is `true`, and `int $n; $n is string` compiles and is `false`. That
differs from `instanceof`, which refuses a subject that can hold no object at all (`E0497`,
[`php-migration/a-declared-type-answers-before-the-program-runs`](/docs/rules/php-migration/divergences/#a-declared-type-answers-before-the-program-runs "-> on a receiver that can hold no object, and instanceof on a subject that can hold none, are refused where they are written")) — but `instanceof` needs a class
to test against and a scalar has none, so the operator is genuinely *inapplicable* there. `is` is
applicable everywhere, because every value has a representation. A knowable answer is not a
meaningless question.

Narrowing is the second reason. Once `is` narrows, a guard written inside an already-narrowed branch
is statically true by construction, and refusing that would let a flow analysis turn working code into
a compile error.

## What it narrows

`is` narrows its subject on the **true edge**, and is the fifth spelling in [`types/narrowing`](/docs/rules/types/unions-and-conversion/#narrowing "Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it") —
which owns every other property of narrowing, including that it changes what is known about a binding
and never its declared type.

## Where it answers differently from PHP

`int` and `uint` are separate tags ([`types/uint`](/docs/rules/types/declarations-and-numbers/#uint "uint is a distinct unsigned 64-bit integer, and it costs no bytes a value did not already hold")), so a value from a `BIGINT UNSIGNED` column
answers `is uint` and **not** `is int`, where PHP's `is_int()` is true for both; `is int|uint` is the
migration spelling. `string` and `bytes` are separate the same way
([`types/string-is-utf8`](/docs/rules/types/text-and-literal-types/#string-is-utf8 "A string is valid UTF-8 for its whole lifetime, and its unmarked unit is the grapheme cluster"), [`types/bytes`](/docs/rules/types/text-and-literal-types/#bytes "bytes is a primitive peer to string for data that carries no encoding")), so binary data answers `is bytes` where PHP's
`is_string()` is true. Both are consequences of a finer type system rather than of this operator, and
`is` is simply the first spelling that makes them reachable from a mechanical rewrite of PHP source.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>int</code> and <code>uint</code> are separate tags, so a <code>BIGINT UNSIGNED</code> value answers <code>is uint</code> and not <code>is int</code> where <code>is_int()</code> was true for both, and <code>string</code> and <code>bytes</code> are separate the same way; <code>is 3.14</code> is refused outright, there being no float literal type</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#narrowing" title="Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it"><code>types/narrowing</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a> <a href="/docs/rules/types/text-and-literal-types/#literal-types" title="A string or int literal is its own type, and a union of them is a closed set"><code>types/literal-types</code></a> <a href="/docs/rules/types/objects-and-shapes/#class-reference-sites" title="Three sites accept a class&lt;T&gt;, and every other operand is still refused there"><code>types/class-reference-sites</code></a> <a href="/docs/rules/php-migration/divergences/#is-takes-pattern-matchings-type-patterns" title="is implements the type-pattern half of PHP's Pattern Matching RFC and reserves every other row of it"><code>php-migration/is-takes-pattern-matchings-type-patterns</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0150.md">record 0150</a></dd></div></dl>

</div>

<div class="nv-rule" id="narrowing">

## Narrowing is flow-sensitive and branch-local, and there are exactly five spellings of it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#narrowing"><code>types/narrowing</code></a>
</div>

Narrowing is flow-sensitive and **branch-local**, and there are five spellings of it: `is`,
`instanceof`, a `== null` test, a comparison against a literal-typed value, and `match (true)`. A
`switch (true)` narrows per arm the same way. A write inside a narrowed block widens the binding
again, because the narrowing described the value that was there, not the slot.

`is` is the general one — it tests a value against any type a value can inhabit, where `instanceof`
tests only a class ([`types/type-test`](/docs/rules/types/unions-and-conversion/#type-test "$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable") owns both the accepted set and why the two coexist). Every
spelling narrows on the **true edge alone**. Subtracting a union member on the failing edge is
deliberately not done by any of the five: it is a separable improvement, and one that has to be taken
for all of them at once or not at all.

Nothing else narrows. In particular an equality against an enum case does not — `$m == Mode::Read`
leaves `$m` at its declared type in the branch it guards, and `$m as Mode::Read|Mode::Write` is how a
case-subset type is reached ([`types/enum-case-type`](/docs/rules/types/text-and-literal-types/#enum-case-type "An enum case used as a type is a narrowed subtype of its enum, never its backing integer")). Adding equality-driven narrowing would
have to be stated again for `!=`, `&&`, `||` and negation, where `as` says the same thing in one
place.

The subject of every spelling is a **binding**, named. A property, an element or any other place is
never the thing narrowed: `$e->previous != null` proves nothing about the next read of
`$e->previous`, so a nullable one is reached through `?->` or bound to a local and tested there.

Narrowing never changes a binding's declared type ([`types/declaration`](/docs/rules/types/declarations-and-numbers/#declaration "Every binding declares its type, and no binding's type ever changes")); it changes what the
checker knows about it on one path. A value that has to *stay* narrowed is a second binding at the
type you want, or a checked `as` ([`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/text-and-literal-types/#enum-case-type" title="An enum case used as a type is a narrowed subtype of its enum, never its backing integer"><code>types/enum-case-type</code></a> <a href="/docs/rules/types/unions-and-conversion/#type-test" title="$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable"><code>types/type-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0066.md">record 0066</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0150.md">record 0150</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-null-test-narrows-a-nullable-local.nvst"><code>tests/conformance/lang/a-null-test-narrows-a-nullable-local.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-instanceof-narrows-its-subject-on-the-true-edge.nvst"><code>tests/conformance/lang/an-instanceof-narrows-its-subject-on-the-true-edge.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-match-and-a-switch-over-true-narrow-per-arm.nvst"><code>tests/conformance/lang/a-match-and-a-switch-over-true-narrow-per-arm.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-comparison-against-a-literal-narrows-its-subject.nvst"><code>tests/conformance/lang/a-comparison-against-a-literal-narrows-its-subject.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/narrowing.rs"><code>crates/nvs-types/tests/narrowing.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="conversion">

## `expr as T` is the only conversion, and it produces a `T` or throws

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#conversion"><code>types/conversion</code></a>
</div>

There are exactly two ways to obtain a value of a different type: `expr as T`, and a second binding
declared at the type you want. `as` is **total in intent and checked in fact** — it produces a value
of the target type or it throws. It never rounds, truncates, or substitutes a default. `expr as ?T` is
the same operator over a nullable target and yields `null` exactly where `expr as T` would throw.

This is the whole conversion surface:

| conversion | behaviour |
|---|---|
| `int` ↔ `uint` | exact, or throws — a negative into `uint`, or above `i64::MAX` into `int` |
| `int` / `uint` → `float` | exact, or throws above 2^53, where `f64` stops representing every integer |
| `float` → `int` / `uint` | integral and in range, or throws. Rounding is `Core\Math::floor`/`ceil`/`round`, said out loud |
| `string` → `int` / `uint` / `float` | the whole string must be an exact numeric literal, or throws. No leading-garbage rule, no `0` |
| anything → `string` | total for scalars; an object needs `Stringable`, or it throws |
| `array<T>` → `array<U>` | every element must satisfy `U`; an O(n) restamp, one tag test per element, sharing the one copy-on-write buffer. An element type naming a class, an enum, a literal type or a union is refused where it is written, `array<mixed>` being the way round it |
| `int` / `uint` → `decimal` | always exact — both fit in 96 bits |
| `decimal` → `int` / `uint` | integral and in range, or throws. Rounding is `Core\Decimal::floor`/`ceil`/`round` |
| `float` → `decimal` | the shortest decimal that round-trips to that `float` — `0.1 as decimal` is `0.1` |
| `decimal` → `float` | nearest `f64`, lossy, and written like every other `as` |
| `string` → `decimal` | the whole string must be an exact decimal literal, or throws |
| `decimal` → `string` | total, and preserves scale: `19.90` renders `"19.90"` |
| `string as bytes` | total and free — valid UTF-8 is already a valid byte sequence, so the same buffer is reinterpreted |
| `bytes as string` | checked: the buffer must be well-formed UTF-8, or it throws. Never replaces, drops or substitutes an invalid byte |
| `EnumName` → its backing `int`/`uint` | total and free — the same representation, reinterpreted |
| backing type / `mixed` → `EnumName` | checked. Throws unless the value equals some case's value |
| `EnumName` → a different `EnumName` | **rejected**, even through `as`, whatever backs them; converting is a `match` naming every case |
| base type / `mixed` → a literal or literal-union type | checked against the named set ([`types/literal-types`](/docs/rules/types/text-and-literal-types/#literal-types "A string or int literal is its own type, and a union of them is a closed set")) |
| an enum / `mixed` → a case-subset type | checked against the named cases ([`types/enum-case-type`](/docs/rules/types/text-and-literal-types/#enum-case-type "An enum case used as a type is a narrowed subtype of its enum, never its backing integer")) |
| `string` / `class<U>` → `class<T>` | the name must be `T` or a class that is one, or it throws. `Foo::class` is decided at compile time, and `class<T>` → `string` is total — the descriptor's own name, not the annotation's |
| `string` / `property<U>` → `property<T>` | the name must be one of `T`'s public declared properties, or it throws. A written-out name is decided at compile time, and `property<T>` → `string` is total |
| any row above, under a qualifier | a successful checked conversion strips `tainted` and `secret`; `as` is never a launderer for a value that keeps its type |

A conversion the operand disproves by itself is a **compile** error rather than a run-time throw:
the target has to be a closed set and the operand has to name one value. Everything else is answered
where it runs.

`as` binds tighter than any binary operator, so `$a as int + 1` is `($a as int) + 1`. Inside a
`foreach` header the `as` belongs to `foreach`, so converting the subject takes parentheses:
`foreach (($m as array<int>) as int $v)`. Two conversions are *not* spelled with it: an `int` or
`uint` widening into a `float` position, which is implicit ([`types/implicit-widening`](/docs/rules/types/declarations-and-numbers/#implicit-widening "An int or uint widening into a float position is the only implicit conversion in the language")), and a
condition, which tests any type against PHP's truthy table without asking for one. PHP's cast syntax
is not a second spelling — it does not parse at all ([`types/no-legacy-cast`](/docs/rules/types/unions-and-conversion/#no-legacy-cast "PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Nothing converts silently — <code>(int)&quot;abc&quot;</code> is not <code>0</code>, <code>PHP_INT_MAX + 1</code> is not a <code>float</code>, and <code>settype</code> does not exist; every conversion is written, checked, and throws rather than substituting</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#no-legacy-cast" title="PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it"><code>types/no-legacy-cast</code></a> <a href="/docs/rules/types/declarations-and-numbers/#implicit-widening" title="An int or uint widening into a float position is the only implicit conversion in the language"><code>types/implicit-widening</code></a> <a href="/docs/rules/types/declarations-and-numbers/#arithmetic" title="An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting"><code>types/arithmetic</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0010.md">record 0010</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0028.md">record 0028</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0034.md">record 0034</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0054.md">record 0054</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0066.md">record 0066</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0125.md">record 0125</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0126.md">record 0126</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0144.md">record 0144</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/conversions-that-succeed.nvst"><code>tests/conformance/lang/conversions-that-succeed.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/the-conversion-table-is-closed.nvst"><code>tests/conformance/lang/the-conversion-table-is-closed.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/the-nullable-conversion-table-is-closed-at-both-ends.nvst"><code>tests/conformance/lang/the-nullable-conversion-table-is-closed-at-both-ends.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-lossy-conversion-throws.nvst"><code>tests/conformance/lang/a-lossy-conversion-throws.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/every-remaining-conversion-row-runs-or-throws.nvst"><code>tests/conformance/lang/every-remaining-conversion-row-runs-or-throws.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-string-that-is-not-a-number-refuses-to-convert.nvst"><code>tests/conformance/lang/a-string-that-is-not-a-number-refuses-to-convert.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-legacy-cast">

## PHP's `(T)expr` cast does not parse, and the diagnostic names the `as` that replaces it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-legacy-cast"><code>types/no-legacy-cast</code></a>
</div>

`(int)expr`, `(uint)expr`, `(float)expr`, `(string)expr`, `(bool)expr`, `(array)expr` and
`(object)expr` are all rejected at parse time. The parser still recognises the shape — `(` a
scalar/`array`/`object` type keyword `)` — so that it can emit `E0225` naming the exact `as` spelling
to use, but it produces an error node; there is no cast node in the AST.

```php
(int)$x        // rejected — "use `$x as int` — it throws instead of silently truncating"
(string)$x     // rejected — "use `$x as string` — it throws instead of silently truncating"
```

The diagnostic fires only for that `(` *keyword* `)` shape in an operand position; the type keywords
keep their ordinary meaning everywhere else, and the rejected form still consumes its operand, so
`(int) !$x` leaves nothing dangling. `$x as int` is the only conversion spelling
([`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws")), and a PHP file carrying a legacy cast needs that one mechanical rewrite
before it parses.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>(int)</code>, <code>(uint)</code>, <code>(float)</code>, <code>(string)</code>, <code>(bool)</code>, <code>(array)</code> and <code>(object)</code> are all parse errors rather than a second spelling of <code>as</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0034.md">record 0034</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0049.md">record 0049</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-legacy-cast-does-not-parse.nvst"><code>tests/conformance/reject/a-legacy-cast-does-not-parse.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-syntax/src/parser/tests/expr.rs"><code>crates/nvs-syntax/src/parser/tests/expr.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="type-alias">

## A `type` alias is a transparent, compile-time-only synonym for a type expression

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#type-alias"><code>types/type-alias</code></a>
</div>

`type Name = TypeExpr;` declares a compile-time-only synonym for a type expression, written either at
file and namespace scope alongside `use` and `namespace` or as a member of a class, interface or enum
body ([`types/class-scoped-alias`](/docs/rules/types/unions-and-conversion/#class-scoped-alias "A type alias is also a member of a class, interface or enum, reached as Owner::Name and as a bare Name inside its owner")) — never inside a method body, a block or a closure body, where
it is `E0233` by name like any other declaration written where control flow can reach it.

```php
type UserId = uint;
type Result = User|NotFoundError;
type Matrix = array<array<float>>;
type Point  = {x: int, y: int};
```

`TypeExpr` is any production of the type grammar ([`types/grammar`](/docs/rules/types/declarations-and-numbers/#grammar "The type grammar is a closed set of atoms under unions and intersections")) but one
([`types/alias-is-never-a-bare-class`](/docs/rules/types/unions-and-conversion/#alias-is-never-a-bare-class "A type alias may not name a single bare class")). The alias name is then lexically valid anywhere a class
name is, resolved by the same contextual lookup that already tells `self`/`static`/`parent` and an
enum's name apart from a class's, and reached through ordinary `use`/FQN resolution — nothing is
auto-imported.

An alias is **fully transparent, never nominal**: `UserId` and `uint` are the same type everywhere, in
both directions, needing no `as`, because after the checker resolves the alias there is only one type
there. It is not a newtype, and it has **zero runtime footprint** — nothing downstream of the checker
ever sees the alias name, not codegen, not the value layout, not `Core\Reflect::typeOf`, not an
isolate boundary crossing. That is what keeps it clear of [`statements/nothing-gets-a-second-name`](/docs/rules/statements/names-and-require/#nothing-gets-a-second-name "A declaration is reachable under exactly the name it was declared with"):
it creates no name a runtime observer can see.

Aliases are resolved eagerly and a cycle is a diagnostic — `type A = B; type B = A;` is rejected at
check time rather than left to loop or bottom out at `mixed`. They are non-parametric:
`type Rows<T> = …` is out of scope while user-defined generics are.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#class-scoped-alias" title="A type alias is also a member of a class, interface or enum, reached as Owner::Name and as a bare Name inside its owner"><code>types/class-scoped-alias</code></a> <a href="/docs/rules/types/unions-and-conversion/#alias-is-never-a-bare-class" title="A type alias may not name a single bare class"><code>types/alias-is-never-a-bare-class</code></a> <a href="/docs/rules/types/declarations-and-numbers/#grammar" title="The type grammar is a closed set of atoms under unions and intersections"><code>types/grammar</code></a> <a href="/docs/rules/types/objects-and-shapes/#shape-type" title="{name: T} in type position is a structural shape checked by width subtyping, and {name?: T} marks a key that may be absent"><code>types/shape-type</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0015.md">record 0015</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0036.md">record 0036</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0190.md">record 0190</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-type-declared-inside-a-body-is-a-compile-error.nvst"><code>tests/conformance/lang/a-type-declared-inside-a-body-is-a-compile-error.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-type-alias-declared-inside-a-body-is-refused-by-name.nvst"><code>tests/conformance/reject/a-type-alias-declared-inside-a-body-is-refused-by-name.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/an-attribute-name-is-a-shape-typed-type-alias.nvst"><code>tests/conformance/reject/an-attribute-name-is-a-shape-typed-type-alias.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-hir/src/aliases.rs"><code>crates/nvs-hir/src/aliases.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="alias-is-never-a-bare-class">

## A `type` alias may not name a single bare class

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#alias-is-never-a-bare-class"><code>types/alias-is-never-a-bare-class</code></a>
</div>

A `TypeExpr` that is nothing but one bare `ClassName`, `EnumName`, `self`, `static` or `parent` atom —
with no union, intersection, array wrapper or `?` sugar around it — is refused (`E0307`).

```php
type Id = SomeClass;              // rejected
type Ids = array<SomeClass>;      // fine
type Result = SomeClass|NotFound; // fine
```

Allowing the bare form would be `use SomeClass as Id;` wearing the type grammar as a disguise: a
second name a *runtime* observer would plausibly expect to mean what `SomeClass` means everywhere,
which is exactly what [`statements/nothing-gets-a-second-name`](/docs/rules/statements/names-and-require/#nothing-gets-a-second-name "A declaration is reachable under exactly the name it was declared with") closes. A `type` alias exists to
give a short name to a **shape** — a union, an intersection, a parameterised array, an object shape —
never to a single already-named class.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#type-alias" title="A type alias is a transparent, compile-time-only synonym for a type expression"><code>types/type-alias</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0015.md">record 0015</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-hir/src/resolve.rs"><code>crates/nvs-hir/src/resolve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="class-scoped-alias">

## A `type` alias is also a member of a class, interface or enum, reached as `Owner::Name` and as a bare `Name` inside its owner

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#class-scoped-alias"><code>types/class-scoped-alias</code></a>
</div>

A `type` alias is also a member of a class, interface or enum body, taking no visibility modifier,
reached as `Owner::Name` from anywhere and as a bare `Name` inside its owner's own body, and never
inherited.

```php
final class Order {
    type Meta = {total: decimal, note?: string};

    public function meta(): Meta { … }          // the short form, inside the owner
}

function show(Order::Meta $m): void { … }       // the qualified form, anywhere
```

Everything [`types/type-alias`](/docs/rules/types/unions-and-conversion/#type-alias "A type alias is a transparent, compile-time-only synonym for a type expression") says about an alias holds here unchanged: it is transparent in
both directions, it is erased before codegen, a cycle is a diagnostic, and it may not name one bare
class-shaped atom ([`types/alias-is-never-a-bare-class`](/docs/rules/types/unions-and-conversion/#alias-is-never-a-bare-class "A type alias may not name a single bare class")). The member is accepted in a class, an
interface and an enum body alike — an interface's alias is not a contract an implementor satisfies,
and an enum's alias has nothing to do with its cases.

- **No visibility, ever.** A modifier run or an attribute group written in front of a body's `type`
  is `E0133`. Visibility restricts reaching a name a running program has, and an alias has none; a
  `private` alias would hide the name while leaving the type it expands to writable by anyone.
- **No inheritance.** `Sub::Name`, where only an ancestor of `Sub` declares `Name`, is `E0405`
  naming the owner that does. A class constant is inherited because a subclass genuinely has one; an
  alias is an entry on nothing, and inheriting it would give one type as many names as its owner has
  descendants — what [`statements/nothing-gets-a-second-name`](/docs/rules/statements/names-and-require/#nothing-gets-a-second-name "A declaration is reachable under exactly the name it was declared with") closes.
- **Two spellings and no third.** `self::Name` and `static::Name` in type position are not spellings
  of this member and stay refused. An alias is resolved before there is a receiver, so `static::`
  could only ever mean the lexical class, which the bare `Name` already says.
- **A name is one thing.** A body's alias sharing a name with a class constant or an enum case is
  `E0304` at the later of the two declarations. `Owner::Name` in type position is therefore read as
  an alias, then an enum case, then a class constant, and no program that compiles depends on that
  order — it exists so the diagnostic for a name that resolves to nothing can say what was looked
  for.

The member costs nothing per request, because nothing about it survives the checker, and one
alias-table entry per declaration at compile time, keyed by the owner's `QName` and the member name
rather than by a namespace path — `Ns\Order\Meta` is also the spelling of a class `Meta` in namespace
`Ns\Order`, and the two must not share a key.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#type-alias" title="A type alias is a transparent, compile-time-only synonym for a type expression"><code>types/type-alias</code></a> <a href="/docs/rules/types/unions-and-conversion/#alias-is-never-a-bare-class" title="A type alias may not name a single bare class"><code>types/alias-is-never-a-bare-class</code></a> <a href="/docs/rules/types/objects-and-shapes/#shape-type" title="{name: T} in type position is a structural shape checked by width subtyping, and {name?: T} marks a key that may be absent"><code>types/shape-type</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0190.md">record 0190</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-class-scoped-type-alias-agrees-with-the-file-scope-form.nvst"><code>tests/conformance/lang/a-class-scoped-type-alias-agrees-with-the-file-scope-form.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-class-scoped-type-alias-is-the-shape-it-names.nvst"><code>tests/conformance/lang/a-class-scoped-type-alias-is-the-shape-it-names.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-interface-and-an-enum-own-a-type-alias-too.nvst"><code>tests/conformance/lang/an-interface-and-an-enum-own-a-type-alias-too.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-class-scoped-type-alias-takes-no-modifier.nvst"><code>tests/conformance/reject/a-class-scoped-type-alias-takes-no-modifier.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-class-scoped-type-alias-is-not-inherited.nvst"><code>tests/conformance/reject/a-class-scoped-type-alias-is-not-inherited.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-class-scoped-type-alias-does-not-share-a-name-with-a-constant-or-a-case.nvst"><code>tests/conformance/reject/a-class-scoped-type-alias-does-not-share-a-name-with-a-constant-or-a-case.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-class-scoped-type-alias-of-a-bare-class-is-refused.nvst"><code>tests/conformance/reject/a-class-scoped-type-alias-of-a-bare-class-is-refused.nvst</code></a></dd></div></dl>

</div>
