Novis's `is` implements the **type-pattern half** of PHP's Pattern Matching RFC and deliberately
claims none of the rest, because PHP reserved the spelling for that RFC by name and has not yet voted
on it. PHP 8.6 deprecates `is` as an identifier, and the *Deprecations for PHP 8.6* RFC gives one
motivation and no other: to reserve it for pattern matching. That is a claim on the spelling, so
Novis's job is to be a subset that stays true rather than a superset that has to be taken back.

`rule:types/type-test` is the operator and owns its table. This rule owns the **relationship**: what
we took, what we left, and what a future session must read before claiming any of it.

## Taken, and identical to the RFC

Type patterns (`is string`, `is Request`, `is ?array`, `is int|float`, `is iterable`, `is true`,
`is mixed`), unions and intersections under DNF, literal patterns (`is 5`, `is 'yay'`,
`is "beep"|"boop"`, `is null`), and class-constant patterns (`is MyEnum::Case`, `is self::Wild`).
Every one of those is already a `Type` in our grammar, and the RFC's semantics for them are strict —
the same strictness `is_int()` has — which is what `is` answers with.

One row is not taken: `is 3.14`. `rule:types/literal-types` has no float literal type on purpose, so
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
(`rule:types/class-reference-sites`), and it is the one thing `is` structurally cannot express.

## What this rule is for

Before any session claims a reserved row above, it re-reads the RFC as accepted rather than as
described here — the contested parts are the ones most likely to have moved — and records the result.
`rule:types/type-test`'s three refusals and this rule's reserved list are one decision seen from two
sides, and neither is edited without the other.
