# ADR 0007 — Types are declared, checked, and never change by themselves

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** the type grammar; the declaration requirement at every binding site; `uint`; typed and
  nested arrays; string-only array keys; unions and `mixed`; the conversion operator; the result type of
  every arithmetic operator
- **Amended by:** 0008, 0010, 0011, 0012, 0013, 0015, 0022, 0024, 0027, 0028, 0031, 0033, 0034, 0035, 0036, 0037, 0047, 0053, 0054, 0063, 0066, 0069, 0090

> **In short:** every binding — parameter, property, constant, local, loop variable, closure parameter,
> return — declares a type, and **a binding's declared type never changes**. A *value's* type changes only
> where the source says so: a new binding, or the conversion operator `expr as T`, which throws rather than
> silently losing information (`expr as ?T` yields `null` instead of throwing —
> [ADR 0066](0066-nullable-conversion-operator.md)). The types are
> `null bool int uint float decimal string bytes array<T> object <class> callable`, plus unions
> (`int|string`), intersections, and `mixed` — the one position that is not checked at all. `int` is signed
> `i64`; **`uint` is unsigned**, so the full 64-bit range is representable; `float` is always `f64`. Arrays
> keep PHP's ordered hash exactly, with two changes: **every key is a string**, and the element type may be
> declared and nested to any depth (`array<array<uint>>`), enforced on every write. The headline cost is in
> *Consequences*: **PHP source no longer runs unconverted**, because PHP has no syntax for the type of a
> local.

## Context

- PHP keeps a useful feature — **unions** — but pairs it with a liability: **a variable's type can change**,
  and PHP converts silently to make each operation succeed. Failures surface far from their cause:
  `(int)$_GET['id']` on `"abc"` is `0` (often a valid row id); `PHP_INT_MAX + 1` becomes a precision-losing
  `float`; `settype()` invalidates every later assumption; `$a[8]`/`$a["8"]`/`$a["08"]` split across two key
  domains.
- That is worse for MWL on three priorities: **security** (untrusted input coercing to a wrong-but-valid
  value is the shape of many IDOR bugs), **latency** (a statically known type is what lets the backend emit
  a native instruction instead of a tag-dispatching helper), and **simplicity** (gradual typing means two
  type systems to keep in agreement, an `Unknown` in the IR, and a soundness boundary — mandatory
  declaration deletes all three, and `mwl-types` becomes a checker rather than a solver).
- **The 64-bit gap:** PHP's only integer is signed `i64`, but web software needs the other half —
  `BIGINT UNSIGNED` keys, snowflake ids, nanosecond timestamps, WIT's `u32`/`u64`. A distinct `uint` costs
  nothing extra in the value layout, since the tagged value already carries a `u64` payload.

## Decision

**MWL is statically and explicitly typed. Every binding declares its type; no binding's type ever changes;
a value's type changes only through an explicit, checked conversion. `mixed` is the single opt-out, and it
is opt-out from checking, not from safety.**

### 1. Every binding site declares a type

Positions PHP already has a type slot for become **mandatory**. Positions PHP has no slot for **get one**:

| binding site | spelling | new? |
|---|---|---|
| function / method parameter | `function f(int $n, array<string> $rows): void` | PHP syntax, now mandatory |
| return type, including `void` / `never` | `: array<User>` | PHP syntax, now mandatory |
| property, promoted constructor parameter | `public readonly uint $id;` | PHP syntax, now mandatory |
| class constant | `public const int MAX = 10;` | PHP 8.3 syntax, now mandatory |
| local variable, at its declaration | `int $n = 0;`, or `var $n = 0;` to infer from the initializer ([0037](0037-var-local-type-inference.md)) | **new slot** |
| `foreach` key and value | `foreach ($rows as string $k => array<int> $row)` | **new slot** |
| destructuring | `[int $a, string $b] = $pair;` | **new slot** |
| closure parameters and return | `fn(int $n): string => …` | PHP syntax, now mandatory |
| `catch` | `catch (JsonError $e)` | already typed in PHP |
| enum backing type | `enum Status: uint` — `int` (default) or `uint` ([0010](0010-enums-are-a-value-type.md)) | PHP syntax, semantics redefined |

There is no function-scope `static` and no global constant, so neither has a binding site here
([0008](0008-static-and-global.md), [0011](0011-functions-and-constants-are-class-members.md)). The
concrete statement grammar for the three new slots is [`docs/spec/00-overview.md`](../spec/00-overview.md)
§ 3.

A binding is declared **once**. Later assignments are bare — `$n = 5;` is legal only if `$n` is already
declared in the enclosing function. Re-declaring a live name is a diagnostic naming the first declaration;
there is no shadowing.

Declaration is **function-scoped**, as in PHP: a variable declared inside an `if` is visible after it. What
changes is that *definite assignment is checked* — reading a binding on a path that may not have reached its
initialiser is a compile error rather than PHP's warning and a `null`. The same analysis covers a second
binding kind, a property against its constructor ([0022](0022-definite-property-initialization.md)).

A reference (`&$x`) binds two names to one slot, so both sides must declare **the same** type. An alias that
widens or narrows is a diagnostic.

### 2. A declared type never changes; a value converts only on request

There are exactly two ways to obtain a value of a different type:

```php
uint $id  = Core\Request::query('id') as uint;   // an explicit, checked conversion
string $s = $id as string;                       // a second binding, with the type you want
```

There is no third way. No assignment, no operator and no function call can change what `$id` is. `settype()`
joins `eval`, `$$var`, `goto`, `global` and `extract()` on the rejected list, with a diagnostic naming `as`.

`as` is **total in intent and checked in fact**: it either produces a value of the target type or throws. It
never rounds, truncates, or substitutes a default. `expr as ?T` is the same operator over a nullable target
and yields `null` exactly where `expr as T` would throw — [ADR 0066](0066-nullable-conversion-operator.md)
owns which conversions admit that form.

| conversion | behaviour |
|---|---|
| `int` ↔ `uint` | exact, or throws — a negative into `uint`, or above `i64::MAX` into `int` |
| `int` / `uint` → `float` | exact, or throws above 2^53, where `f64` stops representing every integer |
| `float` → `int` / `uint` | integral and in range, or throws. Rounding is `Core\Math::floor`/`ceil`/`round`, said out loud |
| `string` → `int` / `uint` / `float` | the whole string must be an exact numeric literal, or throws. No leading-garbage rule, no `0` |
| anything → `string` | total for scalars; an object needs `Stringable`, or it throws ([0028](0028-closing-the-remaining-magic-methods.md)) |
| `array<T>` → `array<U>` | every element must satisfy `U`; O(n), see *5* |
| every row involving `decimal` | [ADR 0054](0054-decimal-scalar-type.md) § 4 owns them |
| `string` ↔ `bytes` | [ADR 0009](0009-string-and-bytes.md) § 3 owns them |
| any row above, under a qualifier | a successful checked conversion strips `tainted` and `secret` ([0024](0024-taint-tracking-for-injection-sinks.md), [0033](0033-secret-qualifier-for-confidential-values.md)); `as` is never a launderer for a value that keeps its type |

PHP's cast syntax `(int)$x` does not parse — [ADR 0034](0034-legacy-cast-syntax-rejected.md) rejects it with
a diagnostic naming `$x as int`. `as` is the only conversion spelling.

Implicit conversion happens in exactly one place: **`int` or `uint` widening into a `float` position**, which
is the one coercion PHP's own `strict_types` permits, and it throws above 2^53 rather than rounding.
Everything else is a diagnostic. A numeric literal is untyped until placed, so it takes `int`, `uint`,
`float` or `decimal` from its target rather than converting ([0054](0054-decimal-scalar-type.md) § 2).

The one position where a value is *tested* without `as` is a condition — `if`/`while`/`for`'s middle
clause/`?:`/`&&`/`||`/`!` — which accepts any type and resolves PHP's full truthy table at runtime
([ADR 0035](0035-truthy-boolean-context.md)). Every other `bool` position still needs an explicit `as bool`
or a comparison.

`as` binds tighter than any binary operator, so `$a as int + 1` is `($a as int) + 1`. One grammar wrinkle:
inside a `foreach` header the `as` belongs to `foreach`, so converting the subject needs parentheses —
`foreach (($m as array<int>) as int $v)`.

### 3. The type grammar

```
type         := qualified
qualified    := ('secret')? ('tainted')? union            // qualifiers: 0024, 0033
union        := intersection ('|' intersection)*
intersection := atom ('&' atom)*  |  '(' union ')'        // DNF, as PHP 8.2
atom         := 'null' | 'bool' | 'int' | 'uint' | 'float' | 'decimal'
              | 'string' | 'bytes'
              | 'array' | 'array' '<' type '>'
              | 'object' | 'mixed' | 'void' | 'never' | 'true' | 'false'
              | 'iterable' | 'callable' | 'self' | 'static' | 'parent'
              | StringLiteral | IntLiteral                // 0047
              | '{' field (',' field)* '}'                // shape type, 0036
              | Name                                      // class, interface, enum, enum case, `type` alias
              | '?' atom                                  // sugar for atom|null
field        := identifier ':' type
```

Unions are canonicalised — flattened, de-duplicated, order-insensitive — so `int|string` and
`string|int|int` are one type. `array` with no argument is exactly `array<mixed>`. `void` and `never` are
return-only. `array<T>` is parsed **only in type position**, where a `<` is unambiguously a type-argument
list. Two expression positions also admit one, and they are the same production under one rule: a **call
site's own** `<...>`, written between a member name and its `(` (§ 5's third bullet), and a **`new`
target's**, written between the class named and its `(` — `new Core\ObjectMap<Tag, int>()`. Both are
settled by a checkpointed trial parse that commits only when the list parses cleanly and a `(` follows, so
`new Foo < $x` and `Foo::BAR < $x` are both still comparisons. Which target may carry one is a resolution
question, not a grammatical one: only a compiler-owned generic declaration, exactly as for `Name` below.
User-defined generic *functions* and *classes* are still not part of this decision.

`Name` covers four kinds of atom that share one lexical production and are told apart by resolution: a
class/interface name, an enum's name, an enum case ([0047](0047-literal-and-enum-case-types.md)), and a
`type` alias ([0015](0015-no-name-aliasing.md)). A class name may carry concrete arguments where it names a
compiler-owned generic interface — `Iterator<User>` — and nothing else
([0053](0053-iteration-and-generators.md) § 2).

`object` is the opaque top of every class type, and a `{a: int}` shape type is MWL's one structurally
checked type ([0036](0036-anonymous-object-shapes.md)). `callable` is opaque as to signature — there is no
`callable(int): string` — and is satisfied by exactly one shape of value, a closure
([0031](0031-callable-is-the-only-closure-type.md)). Calling through one is a dynamic call with
runtime-checked arguments, at `mixed`'s cost. Deferred, not rejected; see *Revisiting*.

**There is no `resource` type.** A host handle is an ordinary object with an explicit `close()` —
`Core\IO\File`, `Core\Process\Child` — which is what makes MWL's lack of destructors
([0028](0028-closing-the-remaining-magic-methods.md)) coherent, and what
[`docs/spec/01-core-library.md`](../spec/01-core-library.md) § 14 already states.

### 4. `uint`, and what every arithmetic operator returns

`uint` is an unsigned 64-bit integer, `0 … 2^64−1`. It is a new **tag** in the existing tagged value, whose
layout is owned by § *Value representation* in [the plan's design record](../plan/design.md) — the payload is
already a `u64`, so `uint` costs **zero additional bytes per value**. `Core\Reflect::typeOf` reports it as
its own kind; there is no `is_int`-style predicate to disagree with, because there are no free functions
([0011](0011-functions-and-constants-are-class-members.md)).

An integer literal that does not fit `int` is legal only where a `uint` is expected, and is otherwise a
diagnostic saying exactly that. There is no literal suffix, for any numeric type.

| operation | result | on overflow / edge |
|---|---|---|
| `int ⊕ int`, `uint ⊕ uint` for `+ - * ** %` | the same type | **throws `ArithmeticError`.** No wrap, no promotion to `float` |
| `int ⊕ uint` arithmetic | **compile error** | there is no representable common type; convert one side explicitly |
| `int` against `uint` in `< <= > >= ==` | `bool`, mathematically exact over the full range of both | — |
| `int / int`, `uint / uint` | `int\|float`, `uint\|float` — PHP-exact: `6/3` is an integer, `7/2` is a float | `/ 0` throws `ArithmeticError` |
| either operand a `float` | `float` | — |
| `>>` | arithmetic on `int`, **logical on `uint`** | — |
| `& \| ^ ~ <<` | the operand type, preserved | — |
| any operation involving `decimal` | [ADR 0054](0054-decimal-scalar-type.md) § 3 owns those rows | — |
| `object` against `object` in `< <= > >= <=>` | requires `Comparable` ([0013](0013-comparable-interface.md)), no fallback | **compile error** when the class does not implement it |

Rejecting mixed-signedness arithmetic while allowing mixed-signedness comparison is the line C gets wrong
and pays for: a comparison has an exact answer in the mathematical integers and can be lowered as one,
while `int + uint` has no type to return. The division rows return unions rather than diverge from PHP, and
in practice the union is absorbed by the target's declared type through the `int → float` widening in *2* —
`float $avg = $sum / $n;` works, `int $n = 7 / 2;` is a diagnostic, and `Core\Math::intDiv` is there when
integer division is what was meant.

Overflow throwing is a divergence from PHP (*7*), and the one this ADR is least willing to trade: a silent
promotion to `float` changes a binding's type behind its declaration, and a silent wrap is the classic
size-computation bug. Code that genuinely wants unbounded magnitude declares `float`, or converts.

### 5. Arrays: PHP's ordered hash, with string keys and a declared element type

The container is unchanged — an insertion-ordered hash with copy-on-write value semantics. Two changes.

**Every key is a `string`.** There is no integer key.

- `$a[] = $v` appends under the next integer index rendered in decimal — `"0"`, `"1"`, `"2"` — from the same
  counter PHP keeps, so lists behave as they always did.
- An `int` or `uint` subscript is normalised to its decimal string at the subscript: `$a[8]` is `$a["8"]`.
  This is key normalisation, not a value conversion, and it needs no `as` — PHP already normalises, in the
  other direction. `"08"` remains a distinct key from `"8"`, exactly as in PHP.
- A `float`, `bool` or `null` subscript is **rejected**. PHP truncates a float, stringifies `true` to `"1"`
  and `null` to `""`; each is a silent conversion at the one place where a mistake becomes a missing row.
- Iteration order is **insertion order, always**. Only the sort members reorder, and they say so in their
  names.
- **Binary `+` and `+=` over two arrays are a diagnostic**, naming `Core\Arr::underlay`. PHP's array union
  is a set operation wearing arithmetic notation, and — like `array_merge` — its rule is chosen by a key's
  type, which is a distinction this section has just deleted
  ([0069](0069-array-combination-is-key-type-independent.md)).
- `Core\Json::encode` emits a JSON array iff the keys are exactly `"0" … "n−1"` in order, which is the test
  PHP already applies expressed over strings, so encoded output does not change.

What is observably different is what comes *back*: `Core\Arr::keys()` returns `array<string>`, and
`foreach ($a as string $k => …)` types `$k` as `string`. That is the whole blast radius, listed in *7*.

**The element type may be declared, and nests to any depth.**

```php
array<uint>                 $ids;
array<array<int|string>>    $rows;
array<array<array<float>>>  $cube;
array<User|null>            $lookup;
array                       $anything;    // exactly array<mixed>
```

One type parameter, not two, because the key type is fixed by the language.

- **Enforced on every write** — `$a['k'] = $v`, `$a[] = $v`, `+=`, and every `Core\Arr` member that builds an
  array. Where the value's static type satisfies the element type the check is compile-time and free; where
  the value arrives through `mixed` it is a runtime check, and a failure is a throw like any other
  ([0002](0002-error-propagation.md)).
- **Invariant.** `array<int>` is not an `array<int|string>`. Converting is `as array<int|string>`, and costs
  an O(n) restamp — a real copy, since the two cannot share a copy-on-write buffer. Covariance was tempting
  and is rejected in *Alternatives*: it would hide that O(n) inside an assignment.
- The empty literal `[]` has type `array<never>`, which satisfies every `array<T>`, so invariance never gets
  in the way of initialising.
- **Array literals are checked against the target type, never inferred and then compared.** Because every
  binding is annotated, a literal always has a target. This is the second place where the mandatory
  annotation deletes machinery rather than adding it, and it is why
  [0037](0037-var-local-type-inference.md) refuses a bare array-literal initializer.
- **At runtime** an array header carries a pointer to an interned, immutable type descriptor. It is what
  lets a value arriving through `mixed`, `Core\Json::decode` or an
  [isolate boundary](0006-isolated-script-execution.md) be checked at all, and what lets a diagnostic name
  the type it expected. Descriptors are interned process-wide and are O(distinct types in the program), not
  O(requests served) — the [0004](0004-memory-for-simplicity.md) rule. The cost, stated as that ADR
  requires: **one pointer per array header**, plus the O(n) widening copies above.
- The checker bounds descriptor nesting at depth 32 with a diagnostic, so a pathological type cannot make
  checking superlinear.
- **The stdlib's array signatures are parametric in `T`** — `Core\Arr::map(array<T>, callable): array<U>`,
  `Core\Arr::filter(array<T>, callable): array<T>`, `Core\Arr::overlay(array<T>, array<U>): array<T|U>`
  (subject-first per [0063](0063-core-api-conventions.md) R1). Type variables are available to declarations
  the compiler owns: the built-ins, from M9 the WIT-declared extension functions, and — at a concrete
  argument only — a user class implementing a compiler-owned generic interface
  ([0053](0053-iteration-and-generators.md) § 2). User code gets one further concrete-argument door and no
  more: a **call site may write the type argument** for a compiler-owned member that declares one it cannot
  infer, which is `Core\Json::decodeAs<User>($body)` (spec § 6) and `Core\Db`'s `queryAs<T>`
  ([0067](0067-core-db.md) § 6). Every other member infers its variables from its arguments and refuses a
  written list, so nothing gains a second, unchecked spelling. User-written generic functions are not part
  of this decision.

### 6. Unions, narrowing, and `mixed`

A union permits only the operations valid for *every* member. Reaching a member's own operations means
narrowing, which is flow-sensitive and branch-local: `instanceof`, `== null`, a comparison against a
literal-typed value, and `match (true)`. There is no `is_int()`-style predicate to narrow with, because
there are no free functions; getting a scalar out of a union or out of `mixed` is `as T` (throwing) or
`as ?T` (yielding `null`), which is deliberately the same reviewable spelling either way and is why
`Core\Validate` carries no numeric predicates ([0066](0066-nullable-conversion-operator.md)).

`mixed` is **not checked at all** — that is its entire job. It holds anything, every operation on it is
allowed, and every operation on it is resolved dynamically at runtime through the generic helper path. That
is PHP's semantics, exactly, at PHP's cost, which is the right pressure: the fast path is the typed one.
`Core\Reflect::typeOf` is the one type-introspection member, and it is meaningful only on a `mixed` — the
checker already knows every other case.

`mixed` is where untrusted input lands, and deliberately so. `Core\Request::query()`/`::post()`,
`Core\Server::*`, `Core\Script::args()` and `Core\Json::decode`'s result are `array<mixed>` (or return
`mixed` per key), because input genuinely is untyped and pretending otherwise would be a lie in the type.
These replace PHP's superglobals — [ADR 0012](0012-no-superglobals.md) — without changing this shape at all:

```php
uint $id = Core\Request::query('id') as uint;     // throws on "abc", on "-1", on "" — never quietly 0
```

`mixed` never absorbs implicitly in the other direction either: `int $n = $m;` where `$m` is `mixed` is a
diagnostic, not a runtime check.

### 7. Deliberate divergences from PHP

Priority 2 is PHP-compatible observable behaviour, so every departure is listed here rather than discovered
later. Each is reachable in PHP only *because* a binding somewhere is untyped:

| # | PHP | MWL |
|---|---|---|
| 1 | array keys are `int` or `string` | always `string`; which subscripts collide is unchanged, but `Core\Arr::keys()` returns strings |
| 2 | one integer type | `int` and `uint`, reported distinctly by `Core\Reflect::typeOf` |
| 3 | a variable holds anything, always | every binding declared, its type fixed; `settype()` rejected |
| 4 | `(int)"abc"` is `0` | the syntax itself is rejected ([0034](0034-legacy-cast-syntax-rejected.md)); `"abc" as int` throws, `"abc" as ?int` is `null` |
| 5 | `PHP_INT_MAX + 1` becomes a `float` | throws `ArithmeticError` |
| 6 | `(int)9.9` is `9`; `$a[1.7]` is `$a[1]` | throws; `Core\Math::floor`/`round` say it out loud |
| 7 | `int` → `float` rounds silently above 2^53 | throws |
| 8 | reading an undefined variable warns and yields `null` | a definite-assignment error at check time |
| 9 | a function, method or closure may omit its return type | mandatory on every one of them — `void` or `never` stated explicitly when there is no value |
| 10 | `$a[] .= "x"` appends, the element that is not there yet reading as `""` | refused at check time (`E0481`), like every other read of `[]` — nothing makes an absent element read as a zero value, which is row 8 one storage kind along |

The consequence to plan around: the imported `.phpt` corpus (M11) will have a **structurally lower** pass
rate than a compatibility-first design would, and failures in these ten classes are intentional
divergence, not bugs. The tracked number must distinguish the two or it will be read as regression.

## Consequences

**Positive**

- The conversion of untrusted input becomes an explicit, reviewable, loudly-failing operation. Priority 1,
  and the single largest reason to accept everything below.
- The baseline backend emits typed operations from M3 instead of dispatching through a generic helper for
  everything, so much of what M12 was for arrives with the first backend.
- No inference engine, no `Unknown` in the IR, no gradual-typing boundary to keep sound.
- `uint` closes the 64-bit gap at zero cost in the value layout.
- Declared types make the [isolate boundary](0006-isolated-script-execution.md) checks largely static.
- Errors arrive at their cause. Every row of the *Context* list becomes a diagnostic with a span.

**Negative**

- **PHP source no longer runs unconverted, and this is the real price.** PHP has no syntax for the type of a
  local, a `foreach` binding, or a destructuring target, so no existing PHP file satisfies the declaration
  requirement. Migration goes through `mwl convert`. For a plain local,
  [0037](0037-var-local-type-inference.md)'s `var` means the converter can emit that instead of running its
  own inference pass; a `foreach` binding and a destructuring target still have no type-eliding spelling.
- **Verbosity.** `array<array<int|string>> $rows` at every declaration is a cost against priority 4. A
  `type` alias is the relief, decided in [0015](0015-no-name-aliasing.md).
- **Array invariance will chafe** where a function wants to accept `array<int>` and `array<int|string>`
  alike. Mitigated by literals being checked against the target and by `array<never>` for `[]`.
- **Nine observable divergences** from PHP, each a place a ported program can change behaviour.
- **Parametric signatures exist for built-ins but not for user code**, a visible asymmetry.
- **`mixed` is a hole by design**, and a program can be written entirely in it. It will then run at PHP's
  speed with PHP's failure modes — code review, not the compiler, is what keeps `mixed` at the boundaries.

## Alternatives rejected

- **A gradual system** (declared types checked, locals inferred) — two type systems to keep in agreement.
- **No `uint`; carry big unsigned values as `string`/`float`** — PHP's answer; pushes conversion into every
  call site and loses precision above 2^53.
- **One arbitrary-precision integer instead of `int`+`uint`** — breaks the tagged value's `u64` payload.
- **Signed/unsigned mixed arithmetic with a promotion rule** — no representable common type.
- **Keeping integer array keys alongside string ones** — two key domains and a juggling rule between them.
- **A two-parameter `array<K, V>`** — pointless once keys are always strings.
- **Covariant arrays** — hides an O(n) restamp inside an innocuous assignment.
- **Preserving PHP's lossy casts alongside `as`** — the lossy one wins by being shorter to type.
- **Overflow promoting to `float`** — changes a binding's runtime type behind its declaration.
- **Making `mixed` checked at its boundaries** — no way left to express "this is untyped input" honestly.
- **Defaulting an omitted return type to `mixed`** — a second untyped position reached by silence.

## Revisiting

- **User-defined generics, typed callables (`callable(int): string`), generic classes** — parked against a
  stated test, not against taste. **This entry is the one home for what forces the typed-`callable` half**,
  and three things do: [0061](0061-compile-time-autoload-and-program-discovery.md) § 3's
  `implementing<T>()`, [0031](0031-callable-is-the-only-closure-type.md)'s boxed-cell entry, and
  [0072](0072-core-task-structured-concurrency.md) § 1's `Task::all`. Routing is not one of them
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 9). An ADR
  that adds a fourth adds it here and nowhere else — a count kept in two files is a count that goes stale. Application code does not reach for them: a typed collection is
  `array<T>` and `Core\Arr`, a `Result<T, E>` is `?T` and a throw, an envelope is a structurally checked
  `{items: array<User>, total: uint}` shape ([0036](0036-anonymous-object-shapes.md)), and the two
  boundaries where a caller's type genuinely cannot be inferred already write it —
  `Core\Json::decodeAs<User>` and `Core\Db`'s `queryAs<T>`. Library code is the real consumer, and *5*
  already hands parametricity to both library tiers the compiler declares: `Core`, and from M9 an
  extension's WIT world. One case is left uncovered — **a pure-MWL third-party package** — so that is the
  one thing measured.
  - **The test**, fired once the package manager ships and never before: across the published packages,
    count the API members whose return type had to widen to `object` or `mixed` because it depends on the
    caller's type, and the `as T` an application writes at those call sites. Few of either and this entry
    is **deleted** rather than renewed. Many, and the count names the narrowest door that closes them —
    most likely a *written* type argument on a user-declared method, which is `MethodSig::type_params`
    reaching a second declaration site rather than a new kind of type.
  - **A generic *class* stays the expensive answer whatever that count says**: it needs bounds the moment
    one sorts ([0013](0013-comparable-interface.md)), a variance rule of its own beside *5*'s, and either
    erasure — checking, with none of the typed path's speed — or per-instantiation monomorphisation, which
    fights [0042](0042-on-disk-artifact-cache-format.md)'s one-immutable-file-per-unit cache and
    [0017](0017-hot-reload-without-restart.md)'s reload.
- **Read-only or covariant array parameters**, if invariance is what people actually trip over.
- **Integer literal suffixes**, if "too large for `int`, and no `uint` expected here" turns out to be
  frequent rather than rare.

## Verification

- **M1**: the grammar in *3* parses, including nested `array<…>`, DNF unions and every new declaration slot
  in *1*; `settype` is rejected with the diagnostic naming `as`; the `foreach`-header `as` ambiguity is a
  snapshot test rather than a surprise.
- **M2**: a corpus where every diagnostic in this ADR is one file — an undeclared local, a re-declared
  local, a read before definite assignment, `int + uint`, `int $n = 7 / 2;`, a `mixed` assigned to a typed
  binding, an element-type violation at every nesting depth, a narrowing that is missing and one present.
  Plus the negative half: no program produces an `Unknown` type, because the IR no longer has one.
- **M3**: typed operations lower to native instructions rather than generic helpers — guarded by
  `a_typed_arithmetic_loop_stays_in_the_native_cost_class` in [`benches/abi-probe`](../../benches/abi-probe/).
- **M4**: conformance cases for `uint` at `0`, `i64::MAX`, `i64::MAX + 1` and `2^64 − 1`; every conversion
  row in *2* both succeeding and throwing; overflow throwing rather than promoting; key ordering preserved
  across insert, delete, re-insert and every sort member; `Core\Arr::keys()` typed `array<string>`;
  `Core\Json::encode` output unchanged from PHP's for both lists and maps. `proptest` on the ordered hash
  covers key normalisation.
- **M8**: the parametric array signatures are written once and shared by the built-ins and the WIT world
  ([0003](0003-extension-system.md)), so `uint` maps to `u64` with no conversion.
- **M11**: the converter's inference pass, measured as *annotations written* against *`TODO`s emitted* on a
  real project, and the `.phpt` pass rate split into "fails" and "diverges intentionally, per § 7".
