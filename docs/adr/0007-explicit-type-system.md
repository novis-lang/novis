# ADR 0007 — Types are declared, checked, and never change by themselves

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** the type grammar; the declaration requirement at every binding site; `uint`; typed and
  nested arrays; string-only array keys; unions and `mixed`; the conversion operator; the result type of
  every arithmetic operator
- **Supersedes:** the *gradual type system — declared types checked, locals inferred* line in the M2
  milestone and the `mwl-types` crate description. MWL has no inference engine and no untyped position.
- **Amended by:** [0008](0008-static-and-global.md) — the *function `static`* row is gone from § 1. MWL has
  no function-scope `static`, so it has no type slot either; `static` as a type atom in § 3 is unaffected.
  [0010](0010-enums-are-a-value-type.md) — the *enum backing type* row in § 1 drops `string` and defaults to
  `int` when omitted; an enum's name joins `ClassName` as its own kind of atom in § 3.
  [0011](0011-functions-and-constants-are-class-members.md) — the *global constant* row is gone from § 1: a
  constant is never declared outside a class, so there is no such binding site left to require a type for.
  [0013](0013-comparable-interface.md) — § 4's operator table gains the *object ⊕ object* row it names,
  previously silent on two object operands. [0015](0015-no-name-aliasing.md) — resolves the *Negative*
  section's "a `type` alias is deferred" line: § 3's grammar gains `type` alias names as a third kind of
  identifier atom, alongside `ClassName` and an enum's name.
  [0022](0022-definite-property-initialization.md) — § 1's "definite assignment is checked" line was scoped
  to local variables only; that ADR names properties as a second binding kind the same analysis covers.
  [0024](0024-taint-tracking-for-injection-sinks.md) — § 2 gains a `tainted` qualifier axis on the
  `string`/`bytes` rows of the conversion table; every other row is unaffected.
  [0027](0027-callable-is-closures-only.md) — § 3's `callable` atom stays opaque as to signature exactly as
  written here, but is no longer silent on which *values* satisfy it: a `Closure` only, never PHP's
  string/array callable spellings and never an invokable object — MWL has no `__invoke`.
  [0028](0028-closing-the-remaining-magic-methods.md) — § 4's conversion table row "anything → `string`"
  named `__toString`; it now names the declared `Stringable` interface instead.
  [0031](0031-callable-is-the-only-closure-type.md) — § 3's `Closure` atom is retired; `callable` is the
  sole surviving spelling for the same value, so § 3's atom list and its opacity sentence both drop
  `Closure`.
  [0033](0033-secret-qualifier-for-confidential-values.md) — § 2's conversion table row gains a second,
  independent qualifier axis: a checked conversion also strips `secret` on success, the same shape it already
  gives `tainted`.
  [0034](0034-legacy-cast-syntax-rejected.md) — § 2's "PHP's cast syntax `(int)$x` is accepted as a second
  spelling" paragraph is withdrawn; `as` is the only conversion spelling, and § 7's divergence 4 is restated
  to say so. [0035](0035-truthy-boolean-context.md) — names the one place a value's declared type is tested
  without `as`: a condition (`if`/`while`/`for`'s middle clause/`?:`/`&&`/`||`/`!`), judged by PHP's full
  truthy table rather than requiring `bool` already. Every other position § 2 governs is unaffected.
  [0037](0037-var-local-type-inference.md) — § 1's local-decl row gains a second spelling, `var $name =
  expr;`, whose type is the initializer's own checked type; resolves the *Alternatives rejected*/
  *Revisiting* entries this ADR used to carry for exactly that idea.
- **Relates to:** [0002](0002-error-propagation.md) (a refused conversion is a throw, so it propagates as
  a checked status), [0003](0003-extension-system.md) (WIT's `u64` finally has an exact MWL type),
  [0004](0004-memory-for-simplicity.md) (what the type machinery spends),
  [0005](0005-config-changeability.md) (none of this is a directive — the type discipline is not
  per-request configuration, and there is no `strict_types` switch to set),
  [0006](0006-isolated-script-execution.md) (a value crossing an isolate boundary carries its element type
  with it)

> **In short:** every binding — parameter, property, constant, local, loop variable, closure parameter,
> return — declares a type, and **a binding's declared type never changes**. A *value's* type changes only
> where the source says so: a new binding, or the checked conversion operator (provisional spelling
> `expr as T`), which throws rather than silently losing information. The types are
> `null bool int uint float string array<T> <class> callable resource`, plus unions (`int|string`),
> intersections, and `mixed` — the one position that is not checked at all. `int` is signed `i64`;
> **`uint` is new and unsigned**, so the full 64-bit range is representable; `float` is always `f64`.
> Arrays keep PHP's ordered hash exactly, with two changes: **every key is a string**, and the element
> type may be declared and nested to any depth (`array<array<uint>>`), enforced on every write. The
> headline cost is in *Consequences*: **PHP source no longer runs unconverted**, because PHP has no syntax
> for the type of a local.

## Context

PHP's type system is lazy in two ways. The feature is **unions** — a value that's legitimately "an `int` or
a `string`" is common in real code — which MWL keeps. The liability is **mutability of a binding's type**:
a PHP variable holds anything, and the language converts silently to make each operation succeed, so
failures surface as wrong answers far from their cause — `(int)$_GET['id']` on `abc` silently becomes `0`
(often a valid row id); `PHP_INT_MAX + 1` silently becomes a precision-losing `float`; `settype()` makes
every later assumption about a variable stale; `$a[8]` vs `$a["8"]` vs `$a["08"]` splits silently across two
key domains.

This is worse for MWL than for PHP on three priorities:

- **Security (priority 1).** Every value entering a request is untrusted, and PHP's coercions are exactly
  what turn "not a number" into "zero" — `(int)$_GET['id'] → 0` is the shape of a long line of
  authorisation and IDOR bugs. A conversion of untrusted data should be a reviewable place in the source
  that fails loudly.
- **Latency (priority 3).** The baseline tier lowers every operation to a runtime helper that inspects tags
  and dispatches; a statically known type is what lets the backend emit a native instruction instead.
  Mandatory declaration makes types known by construction everywhere, rather than only where inference
  happened to succeed.
- **Simplicity (priority 4).** Gradual typing means two type systems that must agree, a soundness story for
  the boundary, `Unknown` propagating through the IR, and a disagreement rule. Mandatory declaration deletes
  all of it — `mwl-types` becomes a checker, not a solver.

**The 64-bit gap:** PHP's only integer is a signed `i64`, but web software routinely needs the other half of
the range — `BIGINT UNSIGNED` keys, snowflake ids, hash words, nanosecond timestamps, and every `u32`/`u64`
in a WIT world ([0003](0003-extension-system.md)). PHP carries such values as strings, loses precision
through `float`, or reaches for GMP, pushing the problem into every call site that touches the value. A
distinct `uint` costs nothing in the value layout, since the tagged value already carries a `u64` payload.

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
| local variable, at its declaration | `int $n = 0;`, or `var $n = 0;` to infer the type from the initializer ([0037](0037-var-local-type-inference.md)) | **new slot** |
| `foreach` key and value | `foreach ($rows as string $k => array<int> $row)` | **new slot** |
| destructuring | `[int $a, string $b] = $pair;` | **new slot** |
| closure / arrow-function parameters and return | `fn(int $n): string => …` | PHP syntax, now mandatory |
| `catch` | `catch (JsonError $e)` | already typed in PHP |
| enum backing type | `enum Status: uint` — `int` (default) or `uint`; no `string` backing, no unbacked (pure) form | PHP syntax, semantics redefined — see [0010](0010-enums-are-a-value-type.md) |

A binding is declared **once**. Later assignments are bare — `$n = 5;` is an assignment, and it is legal
only if `$n` is already declared in the enclosing function. Re-declaring a live name is a diagnostic naming
the first declaration; there is no shadowing.

Declaration is **function-scoped**, as in PHP: a variable declared inside an `if` is visible after it. What
changes is that *definite assignment is checked* — reading a binding on a path that may not have reached its
initialiser is a compile error rather than PHP's "undefined variable" warning and a `null`.

A reference (`&$x`) binds two names to one slot, so both sides must declare **the same** type. An alias that
widens or narrows is a diagnostic: it would be a second declared type for one storage location, which is
exactly what the rule below forbids.

### 2. A declared type never changes; a value converts only on request

There are exactly two ways to obtain a value of a different type, and they are the two the requirement
names:

```php
uint $id  = Core\Request::query('id') as uint;   // an explicit, checked conversion — the "on purpose" marker
string $s = $id as string;                       // a second binding, with the type you want
```

There is no third way. No assignment, no operator, no function call and no `settype` can change what `$id`
is. `settype()` therefore joins `eval`, `$$var`, `goto`, `global` and `extract()` on the rejected list, with
a diagnostic naming `as` as the replacement. [0008](0008-static-and-global.md) adds two more entries to that
list; the plan's decision table holds it in full.

`as` is **total in intent and checked in fact**: it either produces a value of the target type or throws. It
never rounds, truncates, or substitutes a default.

| conversion | behaviour |
|---|---|
| `int` ↔ `uint` | exact, or throws — a negative into `uint`, or above `i64::MAX` into `int` |
| `int` / `uint` → `float` | exact, or throws above 2^53, where `f64` stops representing every integer |
| `float` → `int` / `uint` | integral and in range, or throws. Rounding is `floor`/`ceil`/`round`, said out loud |
| `string` → `int` / `uint` / `float` | the whole string must be an exact numeric literal, or throws. No leading-garbage rule, no `0` |
| anything → `string` | total for scalars; an object needs `Stringable`, or it throws ([0028](0028-closing-the-remaining-magic-methods.md)) |
| `array<T>` → `array<U>` | every element must satisfy `U`; O(n), see *5* |

PHP's cast syntax, `(int)$x`, does not parse — [ADR 0034](0034-legacy-cast-syntax-rejected.md) rejects it
with a diagnostic naming `$x as int` as the replacement. `as` is the only conversion spelling; there is no
second one to keep in sync with it.

Implicit conversion happens in exactly one place: **`int` or `uint` widening into a `float` position**,
which is the one coercion PHP's own `strict_types` permits, and it throws above 2^53 rather than rounding.
Everything else is a diagnostic.

`as` binds tighter than any binary operator, so `$a as int + 1` is `($a as int) + 1`. One grammar wrinkle
for M1: inside a `foreach` header the `as` belongs to `foreach`, so converting the subject needs
parentheses — `foreach (($m as array<int>) as int $v)`.

### 3. The type grammar

```
type         := union
union        := intersection ('|' intersection)*
intersection := atom ('&' atom)*  |  '(' union ')'            // DNF, as PHP 8.2
atom         := 'null' | 'bool' | 'int' | 'uint' | 'float' | 'string'
              | 'array' | 'array' '<' type '>'
              | 'object' | 'mixed' | 'void' | 'never' | 'true' | 'false'
              | 'iterable' | 'callable' | 'self' | 'static' | 'parent'
              | ClassName
              | '?' atom                                      // sugar for atom|null
```

Unions are canonicalised — flattened, de-duplicated, order-insensitive — so `int|string` and
`string|int|int` are one type. `array` with no argument is exactly `array<mixed>`. `void` and `never` are
return-only. `array<T>` is parsed **only in type position**, so `<` never has to be disambiguated against
comparison; that is also why user-defined generic *functions* are not part of this decision.

`ClassName` is lexically what an enum's name matches too — the grammar does not need a separate `EnumName`
production, only a checker that resolves the identifier to one kind of atom or the other, the same way
`self`/`static`/`parent` are already contextual here. What an enum atom means, and how it differs from a
class one, is [0010](0010-enums-are-a-value-type.md)'s decision, not this one's.

`callable`, `Generator` and container classes are **opaque** in v1 — there is no
`callable(int): string` and no `Generator<T>`. Calling through one is a dynamic call with runtime-checked
arguments, at `mixed`'s cost. Deferred, not rejected; see *Revisiting*.

### 4. `uint`, and what every arithmetic operator returns

`uint` is an unsigned 64-bit integer, `0 … 2^64−1`. It is a new **tag** in the existing tagged value, whose
layout is owned by § *Value representation* in [the plan](../implementation-plan.md) — the payload is
already a `u64`, so `uint` costs **zero additional bytes per value**. `is_int()` is false for a `uint`,
`is_uint()` is added, `gettype()` returns
`"uint"`, and `var_dump` prints `uint(18446744073709551615)`.

An integer literal that does not fit `int` is legal only where a `uint` is expected, and is otherwise a
diagnostic saying exactly that. There is no literal suffix.

| operation | result | on overflow / edge |
|---|---|---|
| `int ⊕ int`, `uint ⊕ uint` for `+ - * ** %` | the same type | **throws `ArithmeticError`.** No wrap, no promotion to `float` |
| `int ⊕ uint` arithmetic | **compile error** | there is no representable common type; convert one side explicitly |
| `int` against `uint` in `< <= > >= == ===` | `bool`, mathematically exact over the full range of both | — |
| `int / int`, `uint / uint` | `int\|float`, `uint\|float` — PHP-exact: `6/3` is an integer, `7/2` is a float | `/ 0` throws `DivisionByZeroError` |
| either operand a `float` | `float` | — |
| `>>` | arithmetic on `int`, **logical on `uint`** | — |
| `& \| ^ ~ <<` | the operand type, preserved | — |
| `object` against `object` in `< <= > >= <=>` | see [ADR 0013](0013-comparable-interface.md) — requires `Comparable`, no fallback | **compile error** when the class does not implement it |

Rejecting mixed-signedness arithmetic while allowing mixed-signedness comparison is the line C gets wrong
and pays for: a comparison has an exact answer in the mathematical integers and can be lowered as one,
while `int + uint` has no type to return. The division rows return unions rather than diverge from PHP, and
in practice the union is absorbed by the target's declared type through the `int → float` widening in *2* —
`float $avg = $sum / $n;` works, `int $n = 7 / 2;` is a diagnostic, and `intdiv()` is there when integer
division is what was meant.

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
  other direction. `"08"` remains a distinct key from `"8"`, exactly as in PHP, so which subscripts collide
  does not change.
- A `float`, `bool` or `null` subscript is **rejected**. PHP truncates a float, stringifies `true` to `"1"`
  and `null` to `""`; each is a silent conversion at the one place where a mistake becomes a missing row.
- Iteration order is **insertion order, always** — `foreach`, `array_keys`, `var_dump`, `json_encode`. Only
  the sort functions reorder, and they say so in their names.
- `json_encode` emits a JSON array iff the keys are exactly `"0" … "n−1"` in order, which is the test PHP
  already applies expressed over strings, so encoded output does not change.

What is observably different is what comes *back*: `array_keys()` returns `array<string>`, `gettype()` of a
key is `"string"`, and `foreach ($a as string $k => …)` types `$k` as `string`. That is the whole blast
radius, and it is listed in *7*.

**The element type may be declared, and nests to any depth.**

```php
array<uint>                 $ids;
array<array<int|string>>    $rows;
array<array<array<float>>>  $cube;
array<User|null>            $lookup;
array                       $anything;    // exactly array<mixed>
```

One type parameter, not two, because the key type is fixed by the language.

- **Enforced on every write** — `$a['k'] = $v`, `$a[] = $v`, `+=`, `array_push`, `array_splice`, and every
  stdlib function that writes into an array. Where the value's static type satisfies the element type the
  check is compile-time and there is no runtime cost; where the value arrives through `mixed` it is a
  runtime check, and a failure is a throw like any other ([0002](0002-error-propagation.md)).
- **Invariant.** `array<int>` is not an `array<int|string>`. Converting is `as array<int|string>`, and costs
  an O(n) restamp — a real copy, since the two cannot share a copy-on-write buffer. Covariance was tempting
  and is rejected in *Alternatives*: it would hide that O(n) inside an assignment.
- The empty literal `[]` has type `array<never>`, which satisfies every `array<T>`, so invariance never gets
  in the way of initialising.
- **Array literals are checked against the target type, never inferred and then compared.** Because every
  binding is annotated, a literal always has a target: `array<int|string> $r = [1, 'x'];` is checked
  directly, and `return [1, 'x'];` is checked against the declared return type. This is the second place
  where the mandatory annotation deletes machinery rather than adding it.
- **At runtime** an array header carries a pointer to an interned, immutable type descriptor. It is what
  lets a value arriving through `mixed`, `json_decode` or an
  [isolate boundary](0006-isolated-script-execution.md) be checked at all, and what lets a diagnostic name
  the type it expected. Descriptors are interned process-wide and are O(distinct types in the program), not
  O(requests served) — the [0004](0004-memory-for-simplicity.md) rule. The cost, stated as that ADR
  requires: **one pointer per array header**, plus the O(n) widening copies above.
- The checker bounds descriptor nesting at depth 32 with a diagnostic, so a pathological type cannot make
  checking superlinear.
- **The stdlib's array signatures are parametric in `T`** — `array_map(callable, array<T>): array<U>`,
  `array_filter(array<T>, callable): array<T>`, `array_merge(array<T>, array<U>): array<T|U>`. Type
  variables are available to declarations the compiler owns: the built-ins, and from M9 the WIT-declared
  extension functions. User-written generic functions are not part of this decision.

### 6. Unions, narrowing, and `mixed`

A union permits only the operations valid for *every* member. Reaching a member's own operations means
narrowing, which is flow-sensitive and branch-local: `is_int()`, `is_uint()`, `is_string()`, `instanceof`,
`=== null`, `match (true)`. Assigning a union into a narrower binding needs a guard or an `as`.

`mixed` is **not checked at all** — that is its entire job. It holds anything, every operation on it is
allowed, and every operation on it is resolved dynamically at runtime through the generic helper path. That
is PHP's semantics, exactly, at PHP's cost, which is the right pressure: the fast path is the typed one.

`mixed` is where untrusted input lands, and deliberately so. `Core\Request::query()`/`::post()`,
`Core\Server::*`, `Core\Script::args()` and `json_decode`'s result are `array<mixed>` (or return `mixed`
per key), because input genuinely is untyped and pretending otherwise would be a lie in the type. These
calls replace PHP's `$_GET`/`$_POST`/`$_SERVER`/`$_ARGS` superglobals — see
[ADR 0012](0012-no-superglobals.md) — without changing this shape at all. Getting a value *out* of `mixed`
into a typed binding is an `as` or a narrowing guard — so validating input becomes a reviewable place in the
source instead of an accident:

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
| 1 | array keys are `int` or `string` | always `string`; which subscripts collide is unchanged, but `array_keys()` returns strings |
| 2 | one integer type | `int` and `uint`; `is_int()` is false for a `uint`, `gettype()` says `"uint"` |
| 3 | a variable holds anything, always | every binding declared, its type fixed; `settype()` rejected |
| 4 | `(int)"abc"` is `0` | the syntax itself is rejected ([ADR 0034](0034-legacy-cast-syntax-rejected.md)); `"abc" as int` throws |
| 5 | `PHP_INT_MAX + 1` becomes a `float` | throws `ArithmeticError` |
| 6 | `(int)9.9` is `9`; `$a[1.7]` is `$a[1]` | throws; `floor`/`round` say it out loud |
| 7 | `int` → `float` rounds silently above 2^53 | throws |
| 8 | reading an undefined variable warns and yields `null` | a definite-assignment error at check time |
| 9 | a function, method, closure or arrow function may omit its return type entirely | mandatory on every one of them — `void` or `never` stated explicitly when there is no value, exactly as the return-type row of *1* already requires |

The consequence to plan around: the imported `.phpt` corpus (M11) will have a **structurally lower** pass
rate than a compatibility-first design would, and failures in these nine classes are intentional
divergence, not bugs. The tracked number must distinguish the two or it will be read as regression.

## Consequences

**Positive**

- The conversion of untrusted input becomes an explicit, reviewable, loudly-failing operation. Priority 1,
  and the single largest reason to accept everything below.
- The baseline backend emits typed operations from M3 instead of dispatching through a generic helper for
  everything, so much of what M12 was for arrives with the first backend. M12's inline caches and unboxing
  then only have to cover `mixed`, unions and dynamic calls.
- No inference engine, no `Unknown` in the IR, no gradual-typing boundary to keep sound. `mwl-types` is a
  checker over declared types plus flow narrowing.
- `uint` closes the 64-bit gap at zero cost in the value layout, and makes the `BIGINT UNSIGNED`, hash-word
  and WIT `u64` boundaries exact rather than lossy.
- Declared types make the [isolate boundary](0006-isolated-script-execution.md) checks largely static: a
  closure, reference or resource that cannot cross becomes mostly a compile error at the spawn site rather
  than a refusal at run time.
- Errors arrive at their cause. Every row of the *Context* table becomes a diagnostic with a span.

**Negative**

- **PHP source no longer runs unconverted, and this is the real price.** PHP has no syntax for the type of a
  local, a `foreach` binding, or a destructuring target, so no existing PHP file satisfies the declaration
  requirement — and a PHP global constant has no MWL binding site at all to satisfy, since
  [ADR 0011](0011-functions-and-constants-are-class-members.md) requires it to move onto a class first.
  "Drop your `.php` files in" is gone; migration goes through `mwl convert`. For a plain local, [0037](0037-var-local-type-inference.md)'s
  `var` means the converter can emit that instead of running its own inference pass — a `foreach` binding
  and a destructuring target still have no type-eliding spelling, so those two positions still need one.
- **Verbosity.** `array<array<int|string>> $rows` at every declaration is a cost against priority 4's
  simplicity of the language surface. A `type` alias is the relief, decided in
  [ADR 0015](0015-no-name-aliasing.md) rather than smuggled in beside this ADR's core decision, and barred
  there from aliasing a single bare class — the one shape that would reopen that ADR's rejection of
  PHP-style name aliasing.
- **Array invariance will chafe** where a function wants to accept `array<int>` and `array<int|string>`
  alike. Mitigated by literals being checked against the target and by `array<never>` for `[]`; the escape
  hatch is an O(n) `as`. If this bites in real code the answer is read-only parameters, which is a separate
  decision.
- **Nine observable divergences** from PHP, each argued above, each a place a ported program can change
  behaviour. Three of them (4, 5, 7) turn a silent wrong answer into a throw, which is still a behaviour
  change even though it is the change we want.
- **Parametric signatures exist for built-ins but not for user code**, a visible asymmetry: the stdlib can
  be generic over `T` and a developer cannot.
- **`mixed` is a hole by design**, and a program can be written entirely in it. It will then run at PHP's
  speed with PHP's failure modes, which is the honest outcome — but code review, not the compiler, is what
  keeps `mixed` at the boundaries.

## Alternatives rejected

- **The gradual system in the original plan** (declared types checked, locals inferred). Leaves untrusted
  input conversion implicit, leaves the baseline tier generic wherever inference fails, is two type systems
  to keep in agreement, and has no honest answer for what an inferred type means once a value changes type.
- **No `uint`; carry big unsigned values as `string` or `float`.** PHP's answer — pushes a conversion into
  every call site, and `float` loses precision silently above 2^53.
- **One arbitrary-precision integer type instead of `int` + `uint`.** Breaks the value layout: integers stop
  fitting the tagged value's `u64`, making arithmetic a possible hot-path allocation; also diverges from
  PHP's `i64` semantics and `PHP_INT_MAX`.
- **Signed/unsigned mixed arithmetic with a promotion rule.** There is no representable common type of
  `int` and `uint`, so any rule silently sacrifices a range.
- **Keeping integer array keys alongside string ones.** PHP's behaviour, and it is two key domains with a
  juggling rule between them (`$a[8]`/`$a["8"]`/`$a["08"]`); one domain removes the rule entirely.
- **A two-parameter `array<K, V>`.** Pointless once keys are always strings — `K` would have exactly one
  inhabitant.
- **Covariant arrays** (`array<int>` accepted where `array<int|string>` is wanted). Sound, but the copy is
  O(n) and the restamp cannot share a copy-on-write buffer, hiding a linear cost inside an
  innocuous-looking assignment.
- **Preserving PHP's lossy casts under their own syntax, with `as` alongside.** Two conversion operators
  differing only in honesty; the lossy one wins by being shorter to type.
- **Overflow promoting to `float`, as PHP does.** Changes a binding's runtime type behind its declaration,
  turning an arithmetic bug into a precision bug elsewhere.
- **Making `mixed` checked at its boundaries** (no true escape hatch). Then there is no way to express
  "this is untyped input" honestly, and `json_decode`/`Core\Request` would need a lie in their signatures.
- **Defaulting an omitted return type to `mixed`.** A second untyped position reached by silence rather
  than by writing `mixed` — the exact accident *1* closes for every other binding site.

## Revisiting

Deferred deliberately, each needing its own argument rather than an extension of this one:

- **User-defined generics, typed callables (`callable(int): string`), `Generator<T>`, generic classes.** The
  stdlib's parametric array signatures already prove the checker can carry type variables; opening them to
  user code is a language-surface decision, not a checker one.
- **Read-only or covariant array parameters**, if invariance is what people actually trip over.
- **Integer literal suffixes**, if "too large for `int`, and no `uint` expected here" turns out to be a
  frequent diagnostic rather than a rare one.

Verification, in the order it becomes possible:

- **M1**: the grammar in *3* parses, including nested `array<…>`, DNF unions and every new declaration slot
  in *1*; `settype` is rejected with the diagnostic naming `as`; the `foreach`-header `as` ambiguity is a
  snapshot test rather than a surprise.
- **M2**: a corpus where every diagnostic in this ADR is one file — an undeclared local, a re-declared
  local, a read before definite assignment, `int + uint`, `int $n = 7 / 2;`, a `mixed` assigned to a typed
  binding, an element-type violation at every nesting depth, a narrowing that is missing and one that is
  present. Plus the negative half: no program in the corpus produces an `Unknown` type, because the IR no
  longer has one.
- **M3**: typed operations lower to native instructions rather than generic helpers — the claim that
  mandatory types pay for themselves on the request path becomes a figure in `benches/`, with a guard.
- **M4**: conformance cases for `uint` at `0`, `i64::MAX`, `i64::MAX + 1` and `2^64 − 1`; every conversion
  row in *2* both succeeding and throwing; overflow throwing rather than promoting; key ordering preserved
  across insert, delete, re-insert and every sort function; `array_keys()` typed `array<string>`;
  `json_encode` output unchanged from PHP's for both lists and maps. `proptest` on the ordered hash covers
  key normalisation.
- **M8**: the parametric array signatures are written once and shared by the built-ins and the WIT world
  ([0003](0003-extension-system.md)), so `uint` maps to `u64` with no conversion and the Tier 0 and Tier 1
  signature languages stay one design.
- **M11**: the converter's inference pass, measured as *annotations written* against *`TODO`s emitted* on a
  real project, and the `.phpt` pass rate split into "fails" and "diverges intentionally, per ADR 0007 §7".
