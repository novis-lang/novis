# ADR 0027 — `callable` means a `Closure`; MWL has no `__invoke` and no way to call an object with `()`

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** what values satisfy the `callable` type atom in [ADR 0007](0007-explicit-type-system.md) § 3;
  PHP's three dynamic callable spellings (a bare name string, an `"Class::method"` string, a
  `[$obj_or_class, 'method']` array); whether an object may ever be invoked with `()` syntax; how a
  first-class-callable-syntax expression (`Foo::bar(...)`, `$obj->method(...)`) relates to `Closure`
- **Amends:** [0007](0007-explicit-type-system.md) § 3 — `callable` was left as an opaque atom with no
  stated rule for *which values* satisfy it, only that calling through one is dynamic. This ADR gives it
  that rule without touching the "opaque, no `callable(int): string` signature" part, which stays deferred
  exactly as that ADR already has it.
- **Amended by:** [0031](0031-callable-is-the-only-closure-type.md) — retires `Closure` as a type name.
  Every value this ADR calls "a `Closure`" is unaffected in substance; it is typed `callable` everywhere
  from here on, since after this decision the two names had identical membership. `Closure::fromCallable`
  is dropped there too, for the same reason. Read every occurrence of `Closure` below as `callable`.
  [0063](0063-core-api-conventions.md) — the illustrative `Core\Arr::map` signature is restated
  subject-first, and `Core\Str::len` is spelled `Core\Str::length`; nothing about `callable` changes.
- **Relates to:** [0011](0011-functions-and-constants-are-class-members.md) (every callable is a declared
  method — this ADR is what makes a *value* referencing one of those methods safe to pass around),
  [0012](0012-no-superglobals.md) and [0015](0015-no-name-aliasing.md) (the precedent for closing an
  ambient, string-addressed path to something that has a declared name), [0013](0013-comparable-interface.md)
  (the only operator MWL overloads at all, and only through one narrow, interface-gated mechanism — the
  precedent this ADR follows in refusing a second one), [0014](0014-property-observer.md) (rejects
  `__call`/`__callStatic` for hiding which method a call site actually runs; this ADR rejects `__invoke` for
  the same reason), [0016](0016-ide-integration.md) (the LSP goto-definition/hover this decision keeps
  possible)

> **In short:** `callable` means **a `Closure` value — nothing else.** PHP's three dynamic spellings — a
> bare string (`"strlen"`), an `"Class::method"` string, and a `[$obj, 'method']` array — are all rejected
> with a diagnostic pointing at first-class callable syntax. **MWL has no `__invoke` and no other mechanism
> for calling an object with `()` syntax** — `$obj(...)` is a diagnostic whenever `$obj` is not a `Closure`,
> full stop, regardless of what methods its class declares. No new syntax is introduced: PHP 8.1's
> first-class callable syntax (`Foo::bar(...)`, `$obj->method(...)`, `self::`/`static::`/`parent::method(...)`)
> already parses as of M1 and already produces exactly the `Closure` value this decision requires — closing
> the string/array spellings, and closing `__invoke`, is the whole change.

## Context

- PHP's `callable` pseudo-type accepts five shapes: `Closure`, an invokable object (`__invoke`), a bare
  function-name string, an `"Class::method"` string, and a `[$obj, 'method']` array.
- The three string/array shapes resolve by name at the call site against whatever happens to be declared —
  the same problem [ADR 0012](0012-no-superglobals.md) closed for superglobals and
  [ADR 0015](0015-no-name-aliasing.md) closed for renaming: unresolvable by a reader, checker, or IDE
  without re-implementing PHP's own lookup rules.
- PHP 8.1's first-class callable syntax (`Foo::bar(...)`) already solves the authoring half — a real,
  statically resolvable expression — and already parses in MWL as of M1
  ([implementation-plan.md M1](../implementation-plan.md)); the open question was only whether `callable`
  should still also accept the string/array spellings that syntax was invented to replace.
- `__invoke` closed too, not kept as a fifth shape: an earlier draft kept it, reasoning it's just a normal
  declared method — but that's backward, since `$obj(...)` names *no method at all* at the call site, the
  same hidden-dispatch problem [ADR 0014](0014-property-observer.md) already closed for
  `__call`/`__callStatic`. It would also reopen a second operator-overload mechanism where
  [ADR 0013](0013-comparable-interface.md) deliberately kept only one narrow one (`Comparable` for
  ordering).

## Decision

**`callable` is satisfied by exactly one shape of value: a `Closure`. There is no `__invoke`, and no other
mechanism, that makes an object callable with `()` syntax — `$obj(...)` is refused with a diagnostic unless
`$obj` is itself a `Closure`, regardless of what methods the object's class declares.**

### 1. What still parses, and what does not

```php
// kept — produces a Closure, resolved at the reference itself, not at call time
$fn = Core\Str::length(...);
$fn = $user->getName(...);
$fn = self::helper(...);           // early-bound, like Class::method(...)
$fn = static::helper(...);         // late-bound, exactly like static::class
$fn = fn(int $x): int => $x + 1;   // arrow function, already a Closure
$sum = $fn(1, 2);                  // kept — calling a Closure is unaffected

class Adder {
    public function add(int $a, int $b): int { return $a + $b; }
    // public function __invoke(int $a, int $b): int { ... }   // rejected — no such method exists in MWL
}
$adder = new Adder();
$sum = $adder->add(1, 2);          // kept — call the named method directly
$sum = $adder(1, 2);               // rejected — "Adder is not callable; MWL has no `__invoke`"

// rejected — each names a value this ADR closes off, diagnostic names the replacement
Core\Func::call('strlen', $s);            // bare string — "use `Core\Str::length(...)`"
Core\Func::call('User::validate', $u);    // "Class::method" string — "use `User::validate(...)`"
Core\Func::call([$user, 'getName']);      // array-callable — "use `$user->getName(...)`"
```

`Core\Func::call` above is illustrative of *any* position typed `callable` — a parameter, a property, a
return type, a stdlib signature like `Core\Arr::map(array<T>, callable): array<U>` (already noted as
parametric in [ADR 0007](0007-explicit-type-system.md) § 3's *Consequences*). The rule is the same at every
one of them: the argument must already be a `Closure` by the time it reaches that position, never a string,
an array, or an arbitrary object the checker would have to interpret or special-case.

### 2. Why first-class callable syntax is the only reference-taking spelling

MWL keeps exactly one way to take a reference to a declared method or function: PHP 8.1's `Name(...)`
syntax, already accepted by the M1 grammar. No `Foo::method::ref`-style spelling, and no reuse of `::class`
for this, is introduced — `::class` stays a class-name-to-string operator per PHP's own semantics, unrelated
to producing a callable value. Adding a second reference-taking spelling next to one that already does the
job would repeat the mistake [ADR 0015](0015-no-name-aliasing.md) already argues against elsewhere: two
names for one capability, for no semantic gain.

### 3. Diagnostics

- `"strlen"` (or any string) passed where `callable` is expected → *a string is not callable in MWL; take a
  reference with first-class callable syntax instead — `Core\Str::length(...)`*
- `[$obj, 'method']` (or `[ClassName::class, 'method']`) passed where `callable` is expected → *an array is
  not callable in MWL; use `$obj->method(...)` (or `ClassName::method(...)`)*
- `$obj(...)` where `$obj` is any non-`Closure` value → *`ClassName` is not callable; MWL has no `__invoke`
  — call a named method instead, e.g. `$obj->methodName(...)`*
- `public function __invoke(...)` declared in a class → *`__invoke` has no special meaning in MWL and does
  not make instances callable; rename it to a method with a name callers write explicitly*

## Consequences

**Positive**

- Every `callable`-typed value in a checked program is a `Closure` — never a string the checker would have
  to parse and re-resolve against the symbol table, and never an arbitrary object whose callability depends
  on a hidden method the call site doesn't name.
- `()` stays exactly one thing — calling a `Closure` — rather than a second operator any class can opt into,
  keeping [ADR 0013](0013-comparable-interface.md)'s "one narrow, interface-gated overload, no general
  facility" the only exception to "operators mean one thing" in the whole language.
- An IDE's "go to definition" on any callable value now always lands somewhere real: the
  first-class-callable-syntax reference that created it, or the closure literal — never a string with no
  declared home, and never a `()` call site that silently depends on a class declaring a magic method name.
- Nothing new was invented to get here: first-class callable syntax already parses (M1), and `Closure`
  already exists as a type atom ([ADR 0007](0007-explicit-type-system.md) § 3). The change is subtractive —
  three PHP string/array spellings and the `__invoke` convention, all removed — not a new mechanism to
  design and maintain.

**Negative**

- **A structural break from PHP**, joining the divergence list [ADR 0007](0007-explicit-type-system.md) § 7
  already carries forward: existing PHP code passing `"functionName"`, `"Class::method"`, or
  `[$obj, 'method']` anywhere a callable is expected, or declaring `__invoke` and calling instances with
  `$obj(...)`, does not convert unconverted. Mechanical for `mwl convert` ([M11](../implementation-plan.md))
  in the string/array cases — rewrite to the equivalent first-class-callable-syntax expression — except
  where the string was itself dynamic, which needs a human decision. The `__invoke` case needs a human
  decision unconditionally: `__invoke` must be renamed to a name callers will now write explicitly, and
  every `$obj(...)` call site rewritten to `$obj->thatName(...)`.
- The stdlib's `callable`-typed signatures (`Core\Arr::map`, `Core\Arr::filter`, and future ones) now reject
  a caller passing PHP's string/array spellings or an invokable object, one more small piece of ceremony
  than PHP accepts today — accepted as the same trade every other closed-off PHP dynamism in this project
  already makes.

## Alternatives rejected

- **Keep all five PHP shapes.** Keeps the unresolvable-by-construction path this ADR exists to close, unused
  right next to the syntax that already replaces it.
- **Keep `__invoke` as the sole exception, reject only string/array spellings.** This decision's first
  draft; reconsidered per *Context* — same hidden-dispatch problem [ADR 0014](0014-property-observer.md)
  closed, and reopens a second operator-overload mechanism [ADR 0013](0013-comparable-interface.md)
  deliberately declined to generalize.
- **A dedicated `MethodRef`/`FunctionRef` type distinct from `Closure`.** `Closure` is already the type
  first-class callable syntax produces ([ADR 0007](0007-explicit-type-system.md) § 3); a second type buys no
  behavior, just the redundant-surface pattern [ADR 0015](0015-no-name-aliasing.md) argues against.
- **Reuse `::class` for a callable reference** (`Class::method::class`). `::class` is a class-to-string
  operator with no notion of a member; overloading it would make one token mean two unrelated things.
- **Decide typed closure signatures (`Closure(int): string`) here too.** Already scoped as its own deferred
  question in [ADR 0007](0007-explicit-type-system.md) § 3; this ADR only narrows which *values* satisfy
  `callable`, not what the checker can see through one.

## Revisiting

- **Typed closure signatures** stay deferred exactly where [ADR 0007](0007-explicit-type-system.md) § 3
  already left them — this decision does not reopen that question, and does not need it answered: the
  resolvability this ADR is after comes from the reference at the value's creation site, not from the
  static type carrying a signature.
- **A general operator-overloading facility**, if one is ever proposed, would be where `()`-as-invocation
  gets re-argued from scratch — against the same bar [ADR 0013](0013-comparable-interface.md) set for
  ordering — not a reason to reopen this ADR narrowly for `__invoke` alone.

Verification, in the order it becomes possible:

- **M1** (already true): the parser accepts `Foo::bar(...)`, `$obj->method(...)`,
  `self::`/`static::`/`parent::method(...)` as first-class-callable-syntax expressions producing a
  `Closure`; a method literally named `__invoke` parses as an ordinary method with no special AST shape.
- **M2**: a corpus entry for each rejected spelling — a bare string, an `"Class::method"` string, and a
  `[$obj, 'method']` array — passed where `callable` or `Closure` is the declared type, each refused with a
  diagnostic naming the first-class-callable-syntax replacement; `$obj(...)` refused for every non-`Closure`
  `$obj`, including one whose class declares a method named `__invoke`, naming the class and stating MWL has
  no `__invoke`.
