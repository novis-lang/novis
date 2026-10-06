`$x is T` asks whether a value currently holds a `T` and answers `bool`, for any type a value can
inhabit. It is **strict** — nothing is coerced on the way to the answer — and it is **total**: it
always compiles, and a result the checker can settle by itself folds to a constant rather than
becoming a diagnostic. It is Novis's only type test: there is no second operator for the class case
(`rule:types/one-type-test`).

The right-hand side is a **type**, parsed by the same production `as` uses
(`rule:types/conversion`), not an expression — with the single exception of § *The value arm* below,
which a `$` opens and nothing else does. `is` is the question `as` was standing in for and never
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
| a single-value type | `$x is 5`, `$x is 'yay'`, `$x is true` |
| a class constant or an enum case | `$x is Mode::Read`, `$x is Limits::MAX` |
| an enum | `$x is Rank` |
| `mixed` | `$x is mixed` — always `true`, the wildcard |
| a union or an intersection of any of those | `$x is int\|float`, `$x is Stringable&Comparable` |
| a class reference held in a binding | `$x is $cls` — § *The value arm* |

`array<T>` with a named element type, and a shape, each cost an O(n) walk — the same walk
`as array<T>` already performs, in a spelling that answers instead of throwing. An enum is its cases:
`$x is Rank` answers exactly what asking every `$x is Rank::Case` in turn answers, at one payload
compare per case after one tag comparison, and `rule:enums/representation` owns what that can tell
apart — a case that reached `mixed` carries an enum tag, so it is told apart from an integer, but not
from another enum's case of the same value and backing. Every other row is one tag comparison, or the descriptor walk
the class-test instruction performs.

There is no `float` single-value type to test against (`rule:types/single-value-types`), so `$x is 3.14` is
refused by that rule and not by this one.

## The value arm

A `$` after `is` opens the one arm that is a value rather than a type: `$x is $cls` is the **dynamic
class test**, where `$cls` is a `class<T>` (`rule:types/class-reference`). It tests the class the
subject holds against the descriptor the reference carries — the same descriptor walk a written class
name lowers to — and narrows the subject to `T` on the true edge. `$x is $this->cls` is the same arm;
every other token after `is` starts a type, which keeps a DNF type's opening `(` a type.

A value on the right that is **not** a `class<T>` is `E0496`, the one report `new $v(...)` and
`$v::f(...)` already share, with its help naming `as class<T>`
(`rule:types/class-reference-sites`). A program that wants a computed class reference binds it to a
local and converts it there; a call or a constant after `is` is read as a type and resolves or fails
as one.

## The two refusals

| refused | code | why |
|---|---|---|
| `$x is tainted string`, `is secret bytes` | `E0813` | `rule:security/tainted-qualifier` erases both qualifiers before codegen. There is no runtime bit, so the question has no answer — not merely a knowable one |
| `$x is void`, `$x is never` | `E0811` | no value inhabits either |

Nothing else is refused. In particular a test whose answer the declaration already settles is **not**:
`int $n; $n is int` compiles and is `true`, `int $n; $n is string` compiles and is `false`, and
`int $n; $n is Request` compiles and is `false` like the rest. `is` is applicable to every subject,
because every value has a representation, and a knowable answer is not a meaningless question.

A target nothing is ever an instance of settles the same way. A `Core` **namespace** class is a name
for static members alone, so `$x is Core\Str` is `false` for every subject and every spelling that
reaches one: a member of an intersection settles the whole test, while a union answers on its live
members — `$x is int|Core\Str` is the `int` row. Deeper, the answer belongs to the position rather
than to the test, so `[] is array<Core\Str>` is `true`, an empty array having no element to fail.

Narrowing is the second reason. Once `is` narrows, a guard written inside an already-narrowed branch
is statically true by construction, and refusing that would let a flow analysis turn working code into
a compile error.

## What it narrows

`is` narrows its subject on the **true edge**, and is one of the four spellings in
`rule:types/narrowing` — which owns every other property of narrowing, including that it changes what
is known about a binding and never its declared type.

## Where it answers differently from PHP

`int` and `uint` are separate tags (`rule:types/uint`), so a value from a `BIGINT UNSIGNED` column
answers `is uint` and **not** `is int`, where PHP's `is_int()` is true for both; `is int|uint` is the
migration spelling. `string` and `bytes` are separate the same way
(`rule:types/string-is-utf8`, `rule:types/bytes`), so binary data answers `is bytes` where PHP's
`is_string()` is true. Both are consequences of a finer type system rather than of this operator, and
`is` is simply the first spelling that makes them reachable from a mechanical rewrite of PHP source.
What that rewrite does with PHP's own class-test operator is `rule:types/one-type-test`'s.
