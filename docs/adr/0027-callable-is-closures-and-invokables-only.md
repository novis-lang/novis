# ADR 0027 — `callable` accepts a `Closure` or an invokable object; PHP's string and array callable spellings are rejected

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** what values satisfy the `callable` type atom in [ADR 0007](0007-explicit-type-system.md) § 3;
  PHP's three dynamic callable spellings (a bare name string, an `"Class::method"` string, a
  `[$obj_or_class, 'method']` array); how a first-class-callable-syntax expression
  (`Foo::bar(...)`, `$obj->method(...)`) relates to `Closure`; the minimal rule for calling an invokable
  object via `$obj(...)`
- **Amends:** [0007](0007-explicit-type-system.md) § 3 — `callable` was left as an opaque atom with no
  stated rule for *which values* satisfy it, only that calling through one is dynamic. This ADR gives it
  that rule without touching the "opaque, no `callable(int): string` signature" part, which stays deferred
  exactly as that ADR already has it.
- **Relates to:** [0011](0011-functions-and-constants-are-class-members.md) (every callable is a declared
  method — this ADR is what makes a *value* referencing one of those methods safe to pass around),
  [0012](0012-no-superglobals.md) and [0015](0015-no-name-aliasing.md) (the precedent for closing an
  ambient, string-addressed path to something that has a declared name), [0014](0014-property-observer.md)
  (draws the line between this decision's `__invoke` and that ADR's rejected `__call`/`__callStatic`),
  [0016](0016-ide-integration.md) (the LSP goto-definition/hover this decision keeps possible)

> **In short:** `callable` means **a `Closure` value, or an instance of a class declaring `__invoke`** —
> nothing else. PHP's three dynamic spellings — a bare string (`"strlen"`), an `"Class::method"` string, and
> a `[$obj, 'method']` array — are all rejected with a diagnostic pointing at first-class callable syntax.
> No new syntax is introduced: PHP 8.1's first-class callable syntax (`Foo::bar(...)`, `$obj->method(...)`,
> `self::`/`static::`/`parent::method(...)`) already parses as of M1 and already produces exactly the
> `Closure` value this decision requires — closing the string/array spellings is the whole change.

## Context

PHP's `callable` pseudo-type is satisfied by five different shapes: a `Closure`, an invokable object, a
bare string naming a global function, an `"Class::method"` string, and a two-element array
(`[$obj, 'method']` or `[ClassName::class, 'method']`). The last three are resolved by *name*, at the call
site, against whatever happens to be declared at that point in the program — the same shape of problem
[ADR 0012](0012-no-superglobals.md) closed for superglobals and [ADR 0015](0015-no-name-aliasing.md) closed
for renaming: a name reachable through a string rather than through a declared reference, unresolvable by a
reader, a checker, or an IDE without re-implementing PHP's own runtime lookup rules.

This is precisely the gap a request to "let me store a reference to a method the way `::class` lets me
reference a class" is pointing at: `"Class::method"` looks like it should be as resolvable as `Class::class`
is, but it is a plain string until something calls it, so nothing statically connects it to the method it
names. PHP 8.1 already solved the *authoring* half of this with first-class callable syntax
(`Foo::bar(...)`) — a real expression, not a string, that the parser resolves the same way an ordinary call
is resolved, and that already parses in MWL as of M1 ([implementation-plan.md M1](../implementation-plan.md)).
What was still open is whether the *type* `callable` continues to also accept the three string/array
spellings that syntax was invented to replace — keeping them would mean the same resolvability gap survives
sitting right next to its own fix.

**Why an invokable object (`__invoke`) is not the same problem.** `$obj(...)` dispatches to a method named
`__invoke` that `$obj`'s class must have declared — the checker resolves it exactly like any other method
call, because it is one. This is unlike the `__call`/`__callStatic` magic [ADR 0014](0014-property-observer.md)
already rejects, which intercepts a call to a method that was *never declared at all*. `__invoke` adds no
new fallback path; it reuses the `()` call operator on a value that happens to be an object, dispatching to
a method a reader can find with an ordinary "go to definition."

## Decision

**`callable` is satisfied by exactly two shapes of value: a `Closure`, and an instance of a class that
declares a public `__invoke` method. A `string` or an `array` value is never `callable`, however it is
spelled.**

### 1. What still parses, and what does not

```php
// kept — produces a Closure, resolved at the reference itself, not at call time
$fn = Core\Str::len(...);
$fn = $user->getName(...);
$fn = self::helper(...);           // early-bound, like Class::method(...)
$fn = static::helper(...);         // late-bound, exactly like static::class
$fn = fn(int $x): int => $x + 1;   // arrow function, already a Closure

class Adder {
    public function __invoke(int $a, int $b): int { return $a + $b; }
}
$add = new Adder();
$sum = $add(1, 2);                  // kept — dispatches to Adder::__invoke

// rejected — each names a value this ADR closes off, diagnostic names the replacement
Core\Func::call('strlen', $s);            // bare string — "use `Core\Str::len(...)`"
Core\Func::call('User::validate', $u);    // "Class::method" string — "use `User::validate(...)`"
Core\Func::call([$user, 'getName']);      // array-callable — "use `$user->getName(...)`"
```

`Core\Func::call` above is illustrative of *any* position typed `callable` — a parameter, a property, a
return type, a stdlib signature like `Core\Arr::map(callable, array<T>): array<U>` (already noted as
parametric in [ADR 0007](0007-explicit-type-system.md) § 3's *Consequences*). The rule is the same at every
one of them: the argument must already be a `Closure` or an invokable object by the time it reaches that
position, never a string or array the checker would have to interpret.

### 2. Why first-class callable syntax is the only reference-taking spelling

MWL keeps exactly one way to take a reference to a declared method or function: PHP 8.1's `Name(...)`
syntax, already accepted by the M1 grammar. No `Foo::method::ref`-style spelling, and no reuse of `::class`
for this, is introduced — `::class` stays a class-name-to-string operator per PHP's own semantics, unrelated
to producing a callable value. Adding a second reference-taking spelling next to one that already does the
job would repeat the mistake [ADR 0015](0015-no-name-aliasing.md) already argues against elsewhere: two
names for one capability, for no semantic gain.

### 3. `__invoke` is a declared method, not a new hook family

`__invoke` is declared exactly like any other method — public, checked by the same visibility and arity
rules as every call — and `$obj(...)` is ordinary call syntax dispatching to it. This is not a fourth magic
method joining `__call`/`__callStatic`/`__get`/`__set`; those exist to catch calls to members that were
*never declared*, which is precisely what [ADR 0014](0014-property-observer.md) already refuses. `__invoke`
requires a declaration to exist at all, so it adds no fallback path and no new resolution rule for the
checker to special-case.

### 4. Diagnostics

- `"strlen"` (or any string) passed where `callable` is expected → *a string is not callable in MWL; take a
  reference with first-class callable syntax instead — `Core\Str::len(...)`*
- `[$obj, 'method']` (or `[ClassName::class, 'method']`) passed where `callable` is expected → *an array is
  not callable in MWL; use `$obj->method(...)` (or `ClassName::method(...)`)*
- calling `$obj(...)` where `$obj`'s class declares no `__invoke` → *`ClassName` is not callable; it has no
  `__invoke` method*

## Consequences

**Positive**

- Every `callable`-typed value in a checked program is either a `Closure` or an invokable object — never a
  string the checker would have to parse and re-resolve against the symbol table at every call site, which
  keeps `callable` resolution a lookup rather than a second, string-shaped name-resolution pass.
- An IDE's "go to definition" on any callable value now always lands somewhere real: either the
  first-class-callable-syntax reference that created it, the closure literal, or the class declaring
  `__invoke` — never a string with no declared home, which is exactly the guessing problem motivating this
  decision.
- Nothing new was invented to get here: first-class callable syntax already parses (M1), `Closure` already
  exists as a type atom ([ADR 0007](0007-explicit-type-system.md) § 3), and `__invoke` reuses the ordinary
  call operator. The change is subtractive — three PHP spellings removed — not a new mechanism to design and
  maintain.

**Negative**

- **A structural break from PHP**, joining the divergence list [ADR 0007](0007-explicit-type-system.md) § 7
  already carries forward: existing PHP code passing `"functionName"`, `"Class::method"`, or
  `[$obj, 'method']` anywhere a callable is expected does not convert unconverted. Mechanical for
  `mwl convert` ([M11](../implementation-plan.md)) in the common case — rewrite to the equivalent
  first-class-callable-syntax expression — except where the string was itself dynamic (built from a
  variable, config, or user input), which has no static equivalent and needs a human decision.
- The stdlib's `callable`-typed signatures (`Core\Arr::map`, `Core\Arr::filter`, and future ones) now reject
  a caller passing PHP's string/array spellings, one more small piece of ceremony than PHP accepts today —
  accepted as the same trade every other closed-off PHP dynamism in this project already makes.

## Alternatives rejected

- **Keep all five PHP shapes, including the three string/array spellings.** Rejected in *Context*: it keeps
  exactly the unresolvable-by-construction path this decision exists to close, sitting unused right next to
  first-class callable syntax, which already replaces it.
- **Introduce a dedicated `MethodRef`/`FunctionRef` type distinct from `Closure`.** Rejected: `Closure` is
  already the type first-class callable syntax produces per [ADR 0007](0007-explicit-type-system.md) § 3;
  a second type for the same value would need its own conversion rules to and from `Closure` for no
  behavioural gain, the redundant-surface pattern [ADR 0015](0015-no-name-aliasing.md) already argues
  against.
- **Reuse `::class` to also produce a callable reference** (e.g. `Class::method::class`). Rejected: `::class`
  is a class-name-to-string operator in PHP, with no notion of a member; overloading it to also mean
  "take a callable reference" would make one token spell two unrelated operations depending on what follows
  it, which is worse for a reader than the two names first-class callable syntax and `::class` already are.
- **Decide typed closure signatures (`Closure(int): string`) as part of this ADR**, since it touches the same
  atom. Rejected: [ADR 0007](0007-explicit-type-system.md) § 3 already scoped that as its own deferred
  decision needing the same type-variables-in-user-code argument as user-defined generics; this ADR only
  narrows *which values* satisfy `callable`/`Closure`, not what the checker can see through one.

## Revisiting

- **Typed closure signatures** stay deferred exactly where [ADR 0007](0007-explicit-type-system.md) § 3
  already left them — this decision does not reopen that question, and does not need it answered: the
  resolvability this ADR is after comes from the reference at the value's creation site, not from the
  static type carrying a signature.
- **`__invoke` arity/visibility interaction with default parameters, and whether an abstract class or
  interface may declare it to make an interface-typed value callable**, are M2 checker-design questions this
  ADR does not resolve in detail — the shape (a public, ordinarily declared method) is fixed here; the
  checker's exact acceptance rule is not.

Verification, in the order it becomes possible:

- **M1** (already true): the parser accepts `Foo::bar(...)`, `$obj->method(...)`,
  `self::`/`static::`/`parent::method(...)` as first-class-callable-syntax expressions producing a `Closure`.
- **M2**: a corpus entry for each rejected spelling — a bare string, an `"Class::method"` string, and a
  `[$obj, 'method']` array — passed where `callable` or `Closure` is the declared type, each refused with a
  diagnostic naming the first-class-callable-syntax replacement; calling `$obj(...)` where the class
  declares no `__invoke` refused with a diagnostic naming the class; calling `$obj(...)` where it does
  declare `__invoke` dispatches like any other method call.
