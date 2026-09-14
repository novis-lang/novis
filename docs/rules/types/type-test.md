`$x is T` asks whether a value currently holds a `T` and answers `bool`, for any type a value can
inhabit. It is **strict** — nothing is coerced on the way to the answer — and it is **total**: it
always compiles, and a result the checker can settle by itself folds to a constant rather than
becoming a diagnostic.

The right-hand side is a **type**, parsed by the same production `as` uses
(`rule:types/conversion`), not an expression. `is` is the question `as` was standing in for and never
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
compare per case, and `rule:enums/representation` owns what that can tell apart — a value that reached
`mixed` is its backing integer, so a case is not distinguishable there from that integer nor from
another enum's case of the same value. Every other row is one tag comparison, or the descriptor walk
`instanceof` already does.

There is no float literal type to test against (`rule:types/literal-types`), so `$x is 3.14` is
refused by that rule and not by this one.

## The three refusals

| refused | code | why |
|---|---|---|
| `$x is tainted string`, `is secret bytes` | `E0813` | `rule:security/tainted-qualifier` erases both qualifiers before codegen. There is no runtime bit, so the question has no answer — not merely a knowable one |
| `$x is void`, `$x is never` | `E0811` | no value inhabits either |
| `$x is $cls` | `E0812` | that is a *value*, not a type. `$x instanceof $cls` is the dynamic class test (`rule:types/class-reference-sites`), and the spelling stays refused because PHP's grammar binds a variable there (`rule:php-migration/is-takes-pattern-matchings-type-patterns`) |

Nothing else is refused. In particular a test whose answer the declaration already settles is **not**:
`int $n; $n is int` compiles and is `true`, and `int $n; $n is string` compiles and is `false`. That
differs from `instanceof`, which refuses a subject that can hold no object at all (`E0497`,
`rule:php-migration/a-declared-type-answers-before-the-program-runs`) — but `instanceof` needs a class
to test against and a scalar has none, so the operator is genuinely *inapplicable* there. `is` is
applicable everywhere, because every value has a representation. A knowable answer is not a
meaningless question.

Narrowing is the second reason. Once `is` narrows, a guard written inside an already-narrowed branch
is statically true by construction, and refusing that would let a flow analysis turn working code into
a compile error.

## What it narrows

`is` narrows its subject on the **true edge**, and is the fifth spelling in `rule:types/narrowing` —
which owns every other property of narrowing, including that it changes what is known about a binding
and never its declared type.

## Where it answers differently from PHP

`int` and `uint` are separate tags (`rule:types/uint`), so a value from a `BIGINT UNSIGNED` column
answers `is uint` and **not** `is int`, where PHP's `is_int()` is true for both; `is int|uint` is the
migration spelling. `string` and `bytes` are separate the same way
(`rule:types/string-is-utf8`, `rule:types/bytes`), so binary data answers `is bytes` where PHP's
`is_string()` is true. Both are consequences of a finer type system rather than of this operator, and
`is` is simply the first spelling that makes them reachable from a mechanical rewrite of PHP source.
