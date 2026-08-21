# ADR 0013 — Ordering two objects requires `Comparable`; PHP's property-walk fallback is rejected

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** the `<`, `>`, `<=`, `>=` and `<=>` operators when both operands are objects; the new global
  `Comparable` interface and its single method; what happens when a class does not implement it
- **Amends:** [0007](0007-explicit-type-system.md) § 4 — adds the object-operand row *Decision § 4* gives
  below to the operator result table, which was previously silent on two object operands.
- **Relates to:** [0002](0002-error-propagation.md) (`compareTo` may throw; it propagates as a checked
  status like any other call, no new exception type needed), [0004](0004-memory-for-simplicity.md) (this
  decision spends nothing — `compareTo` is an ordinary virtual call, already paid for by every other
  method), [0008](0008-static-and-global.md) (no new storage class: a `Comparable` implementation needs no
  entry in § 2's exhaustive table), [0011](0011-functions-and-constants-are-class-members.md) (why
  `Comparable` is a global reserved interface and not a `Core` domain class — it is a contract type, not a
  holder of `static` methods and constants), [0012](0012-no-superglobals.md) (the same shape of decision:
  a PHP behaviour that exists by a name simply being present, rather than by a declared contract, is closed
  rather than preserved)

> **In short:** PHP compares two objects of the same class with `<`/`>` by walking their declared
> properties in order and comparing recursively, stopping at the first difference — a behaviour that exists
> ambiently, with no declaration and no way for a class to opt out or replace it. MWL rejects that fallback
> outright. Ordering two objects with `<`, `>`, `<=`, `>=` or `<=>` requires the class to implement a new
> global interface, **`Comparable`**, with one method: `compareTo(self $other): int`, returning negative,
> zero or positive exactly like `strcmp` or the general `<=>` convention. All five operators lower to a call
> to that method; a class that does not implement it makes those operators a **compile-time diagnostic**,
> not a silent property walk. The interface fixes the other side's type to `self` — two different classes
> are never directly orderable by these operators, even if both implement `Comparable`. `==`/`===`/`!=`/
> `!==` are untouched by this decision and keep whatever equality behaviour they already have.

## Context

PHP's rule for `<`/`>`/`<=`/`>=` between two objects: if they are instances of the same class, compare each
declared property in turn — recursing into nested arrays and objects — and use the first property where
they differ; if the classes differ, or the properties are exhausted with no difference, the result is
whatever PHP's general comparison rules say for the leftover case. Nothing about this is declared anywhere
in the comparing class. It is not a method, not an interface, not an opt-in — it is simply what `<` *does*
to two objects, the same way `$_SERVER` was simply *there* before [ADR 0012](0012-no-superglobals.md). A
class author cannot see, from reading the class, that its instances are orderable at all, let alone by
which rule; a reviewer sees `$invoiceA < $invoiceB` and has to already know PHP's fallback exists to know
what it does. That is precisely the shape [ADR 0008](0008-static-and-global.md),
[0011](0011-functions-and-constants-are-class-members.md) and [0012](0012-no-superglobals.md) already
closed for other PHP features that work "by a name being present" rather than by a declaration — this ADR
continues that line into object comparison.

It is also not a *safe* ambient default. The property walk is unbounded in the size of the object graph it
recurses into — two objects holding attacker-influenced nested structures (a request-derived DTO, for
instance) can be compared with a cost invisible at the `<` call site, and the ordering it produces is
whatever the property declaration order happens to be, not anything the class author chose. Closing it is
a small instance of priority 1 (security), argued in full generality in
[ADR 0004](0004-memory-for-simplicity.md); the larger motivation is priority 2 and priority 4 — an object
comparison should mean what its class says it means, and the class should say it exactly once.

## Decision

**Two objects may be compared with `<`, `>`, `<=`, `>=` or `<=>` only when their class implements
`Comparable`. All five operators lower to one method call. There is no fallback.**

### 1. `Comparable` — a new global reserved interface, not a `Core` domain class

```php
interface Comparable {
    public function compareTo(self $other): int;
}
```

`Comparable` lives in the global namespace, the same place PHP's built-in `Stringable` does — **not** under
`Core`. [ADR 0011](0011-functions-and-constants-are-class-members.md) reserves `Core` for domain classes
that hold `static` methods and constants; `Comparable` holds neither — it is a contract an ordinary class
implements, exactly the shape `Stringable` already has in PHP. Nothing about this ADR asks ADR 0011 to
widen its scope.

`compareTo` returns an `int`: negative if `$this` orders before `$other`, zero if neither orders before the
other, positive if `$this` orders after — the same convention `strcmp` and [ADR 0007](0007-explicit-type-system.md)
§ 4's `<=>` already use for scalars, so a typical implementation is a one-line delegation:

```php
final class Money implements Comparable {
    public function constructor(private readonly int $cents) {}

    public function compareTo(self $other): int {
        return $this->cents <=> $other->cents;   // int <=> int, already defined by ADR 0007 § 4
    }
}
```

### 2. `<`, `>`, `<=`, `>=`, `<=>` lower to `compareTo`

```php
$a < $b     // $a->compareTo($b) < 0
$a <= $b    // $a->compareTo($b) <= 0
$a > $b     // $a->compareTo($b) > 0
$a >= $b    // $a->compareTo($b) >= 0
$a <=> $b   // $a->compareTo($b), unchanged
```

This is a compile-time lowering, not a new runtime mechanism: `compareTo` is called exactly like any other
method, devirtualized when the static type is known exactly like any other call. Nothing here adds a
second dispatch path alongside ordinary method calls.

### 3. No `Comparable`, no ordering — a diagnostic, not PHP's fallback

If the compiler cannot show that both operands' static type implements `Comparable` for the other (see
*4*), using `<`/`>`/`<=`/`>=`/`<=>` on them is a **compile-time diagnostic naming `Comparable` as the fix**.
PHP's recursive property-by-property walk described in *Context* is not implemented anywhere in MWL — there
is no code path that falls back to it. A class that wants its instances ordered says so once, in its own
declaration; a class that does not implement `Comparable` simply cannot be ordered, the same certainty
[ADR 0007](0007-explicit-type-system.md) already gives every other operator whose operand types do not
support it (`int + uint`, arithmetic on a `bool`).

### 4. Same class only — `self`, no cross-class overload

`compareTo(self $other)` fixes the parameter to the implementing class exactly (or, in a subclass, whatever
`self` resolves to there, following the ordinary non-late-static-bound reading `self` already has
everywhere else in the language). Comparing two objects that are not both known to be the same class is a
diagnostic even when both classes implement `Comparable` independently — there is no cross-class overload
of `compareTo` in this decision, and no implicit widening from a subclass's `compareTo` to an ancestor's. A
type that legitimately needs to be ordered against a *different* type expresses that as an ordinary named
method (`Money::isGreaterThan(Distance $d): bool` reads oddly on purpose) rather than through these five
operators. A generic, parameterized `Comparable<T>` allowing a declared non-`self` target type is deferred;
see *Revisiting*.

### 5. `==`, `===`, `!=`, `!==` are unaffected

This decision is scoped to ordering. Equality keeps whatever behaviour it already has independent of
`Comparable` — a class may implement `Comparable` purely to be ordered while its equality stays whatever
PHP's default already gives it. Folding equality into the same interface, so that `compareTo` returning `0`
also means `==`, was considered and rejected; see *Alternatives rejected*.

### 6. The row this adds to [ADR 0007](0007-explicit-type-system.md) § 4

| operation | result | on overflow / edge |
|---|---|---|
| `object ⊕ object` for `< <= > >= <=>`, both operands' static type provably the same class implementing `Comparable` | `bool` (`int` for `<=>`), via `compareTo` | a throwing `compareTo` propagates as a checked status, like any other call |
| `object ⊕ object` for `< <= > >= <=>`, otherwise | **compile error** | diagnostic names `Comparable` as the fix — no fallback exists |

## Consequences

**Positive**

- Every ordering of two objects is backed by a reviewable method a human wrote, never an implicit walk over
  private state in declaration order — the ordering a class produces is exactly the ordering its own code
  says, and nothing else.
- Costs nothing beyond an ordinary virtual call already paid for by every other method
  ([ADR 0004](0004-memory-for-simplicity.md)) — no new storage class ([ADR 0008](0008-static-and-global.md)),
  no new runtime representation.
- Reuses [ADR 0007](0007-explicit-type-system.md) § 4's already-defined `<=>` for scalars inside
  `compareTo`, so the common case (delegate to one field's existing orderable type) is one line.
- Closes an unbounded-cost comparison over attacker-influenced object graphs, a small instance of priority 1
  bought at the same time as priorities 2 and 4.

**Negative**

- **A structural break from PHP**, joining the divergence lists in
  [ADR 0007](0007-explicit-type-system.md) § 7, [ADR 0008](0008-static-and-global.md) and
  [ADR 0010](0010-enums-are-a-value-type.md): PHP source ordering two objects of the same class with `<`/
  `>`, relying on the implicit property walk, does not convert unconverted. `mwl convert`
  ([M11](../implementation-plan.md)) can detect the pattern but must leave adding `Comparable` and writing
  `compareTo` as a `TODO` for a human — there is no mechanical rewrite, because the walk's actual ordering
  depended on property declaration order, which is not something a converter should silently canonicalize
  into a method body.
- Code that sorts an array of objects with a comparator *closure* (`usort($items, fn($a, $b) => $a->x <=> $b->x)`)
  is unaffected — that closure never used the objects' own `<=>`, so this ADR changes nothing about it.
- No cross-class ordering at all, even between a class and its own subclass, until the deferred generic form
  in *Revisiting* is designed — a real (if narrow) loss of PHP's permissiveness.

## Alternatives rejected

- **Per-operator magic methods** (`__lessThan`, `__greaterThan`, `__lessThanOrEqual`, …). Rejected: splits
  one logical decision — "how do two of these order?" — into up to five methods that a class could
  implement inconsistently (`$a < $b` true but `$b > $a` false), where a single sign-returning method
  cannot disagree with itself. The same "one operator, one API" principle
  [ADR 0007](0007-explicit-type-system.md), [ADR 0010](0010-enums-are-a-value-type.md) and
  [ADR 0011](0011-functions-and-constants-are-class-members.md) already lean on elsewhere.
- **Keep PHP's property-walk fallback for classes that do not implement `Comparable`.** Rejected for the
  reason *Context* argues in full: it is exactly the ambient, undeclared behaviour this project has closed
  everywhere else it has been found, and it is not even a safe default — its cost is unbounded in the size
  of the object graph.
- **Fold equality into the same interface** (`compareTo` returning `0` also means `==`). Rejected: it would
  force every class that wants a custom order to also redefine equality, and vice versa, when the two are
  frequently independent — PHP's existing structural equality is often exactly right for a value object even
  when a bespoke order is also wanted. Keeping them as separate concerns (*5*) means a class picks up
  exactly the machinery its problem needs.
- **A generic `Comparable<T>` allowing a declared non-`self` target type**, matching how a handful of other
  languages let a type be ordered against a related-but-different type. Deferred, not rejected — MWL has no
  general user-facing generic type beyond `array<T>` ([ADR 0007](0007-explicit-type-system.md) § 5) to build
  it from yet; see *Revisiting*.

## Revisiting

Deferred deliberately, each needing its own argument:

- **A generic `Comparable<T>`** once, or if, MWL grows a general user-facing generic type mechanism —
  letting a class declare what it is orderable against instead of always `self`.
- **Whether stdlib sorting helpers** (a `Core\Arr`-shaped sort, not yet built — [M8](../implementation-plan.md))
  should require object elements to implement `Comparable`, accept an explicit comparator closure only, or
  both. Not decided here; belongs with `Core\Arr`'s own design.
- **Whether `min()`/`max()`-shaped stdlib helpers** should require `Comparable` for object arguments or
  reject objects outright. Same milestone, same open question.

Verification, in the order it becomes possible:

- **M1**: `interface Comparable { public function compareTo(self $other): int; }` parses with the grammar
  M1 already gives classes/interfaces/traits — nothing new here, since this ADR adds no new syntax.
- **M2**: the checker refuses `<`/`>`/`<=`/`>=`/`<=>` between two objects whose static types are not both
  provably the same `Comparable`-implementing class, with a diagnostic naming `Comparable`; the same
  operators between two objects that do satisfy it type-check as *Decision § 6*'s table gives, joining the
  diagnostic corpus [ADR 0007](0007-explicit-type-system.md)'s own M2 entry already builds.
- **M4**: a `Comparable` implementation's `compareTo` actually runs at all five operators, including a
  throwing `compareTo` propagating correctly through [ADR 0002](0002-error-propagation.md)'s checked-return
  path; two unrelated classes each implementing `Comparable` still refused when compared against each other.
- **M11**: the converter flags PHP source ordering two same-class objects via the implicit property walk as
  a `TODO` naming `Comparable`, per *Consequences*' negative list — no attempted mechanical rewrite.
