# ADR 0031 — `callable` is the only closure type; `fn` is the only closure literal

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** the anonymous-function/closure literal grammar (PHP's `function(...) use (...) {...}` vs
  `fn(...) => ...`); explicit capture (`use`, by value and by reference); whether `Closure` and `callable`
  are two type names or one; `Closure::fromCallable`, `call_user_func`, `call_user_func_array`;
  self-referencing recursion for an otherwise-anonymous closure
- **Amends:** [0007](0007-explicit-type-system.md) § 3 — the `Closure` type atom is retired; `callable` is
  the sole surviving spelling for the same value, so § 3's atom list and its opacity sentence both drop
  `Closure`. [0008](0008-static-and-global.md) § 4 — the `$this`-binding rule is unchanged, but every
  mention of `bindTo()`/`Closure::bind()` there now names a `callable` operation, not a `Closure` one; the
  `use`-clause capture mechanism its prose assumed still exists is itself retired by this ADR.
  [0027](0027-callable-is-closures-only.md) — every value that ADR calls "a `Closure`" is the same value
  this ADR types `callable`; the string/array-spelling and `__invoke` rejections it records are unchanged
  and stay recorded there, including its own diagnostics and verification list, just read with `callable`
  substituted for `Closure` throughout. `Closure::fromCallable` is additionally dropped here, since after
  0027 there is nothing left for it to convert *from*.
- **Relates to:** [0011](0011-functions-and-constants-are-class-members.md) (why recursion is solved with a
  lexical self-name rather than a declared-method requirement — the self-name never becomes a free,
  globally-callable function, so it doesn't reopen that ADR's rule), [0015](0015-no-name-aliasing.md) (the
  precedent this ADR follows in refusing two names for one value — `Closure`/`callable` collapse for
  exactly the reason `class_alias` and import-`as` were already refused), [0029](0029-identifier-casing-is-checked.md)/
  [0030](0030-no-leading-underscores-constructor-spelling.md) (a closure's self-name follows the same
  casing rule as any local binding, no exception needed)

> **In short:** PHP's anonymous-function surface collapses to one literal and one type. **`fn(...)` is the
> only closure literal** — with or without a body (`fn($x) => $x + 1` and `fn($x) => { ...; return $x; }`
> both parse; PHP's `function(...) use (...) {...}` form does not). **There is no `use` clause of any kind**
> — capture is always implicit, always by value, and limited to the variables the body actually reads; the
> by-reference spelling (`use (&$y)`) has no replacement, and sharing mutable state between two independent
> closures is an ordinary object in user code, not a language feature. **A closure that needs to call itself
> may carry an optional self-name** (`fn factorial($n) => ... factorial($n - 1) ...`), visible only inside
> its own body — the one capability `use (&$y)` provided that has no other route back. **`callable` is the
> only surviving type name** — `Closure` is retired, since after [ADR 0027](0027-callable-is-closures-only.md)
> the two names had identical membership. `Closure::fromCallable`, `call_user_func` and
> `call_user_func_array` are all dropped as dead weight: nothing is left to convert from, and direct
> invocation (`$fn(...)` / `$fn(...$args)`) already does what they did.

## Context

- PHP's closure surface is eleven moving parts across two literal forms, two capture modes, and a handful of
  dynamic-dispatch mechanisms already narrowed by [ADR 0027](0027-callable-is-closures-only.md) and
  [ADR 0008](0008-static-and-global.md) § 4: the block-bodied `function(...) use (...) {...}` literal, the
  expression-bodied `fn(...) => ...` arrow literal, `use ($y)` capture by value, `use (&$y)` capture by
  reference, the `static` closure modifier, `__invoke`, three dynamic callable-string/array spellings, and
  the `Closure` class's own `bind`/`bindTo`/`call`/`fromCallable` API.
- [ADR 0027](0027-callable-is-closures-only.md) already closed the string/array spellings and `__invoke`:
  `callable` accepts only a `Closure` value. [ADR 0008](0008-static-and-global.md) § 4 already closed the
  `static` modifier by making the property it asserted (a closure that doesn't reference `$this` can't
  extend the enclosing object's lifetime) true unconditionally, decided by whether the body uses `$this`.
- What was left after those two: two literal spellings for the same underlying value, an explicit `use`
  clause with two capture modes, and a `Closure`/`callable` type-name pair that — once ADR 0027 landed —
  have identical membership: every value satisfying `callable` is a `Closure`, and there is no other value
  either name could ever refer to. Two names, zero remaining semantic difference — the same pattern
  [ADR 0015](0015-no-name-aliasing.md) already refuses for `class_alias` and import renaming.
- The remaining literal form doesn't need to stay two spellings either. PHP kept both `function(){}` and
  `fn() => ...` only because the arrow form was added later and never grew a block body; there is no reason
  MWL's `fn` needs the same restriction.
- Explicit capture (`use`) exists to make a closure's free variables visible at the call site. Auto-capture
  (what PHP's arrow functions already do) buys the same simplicity `use`-based capture would; the deciding
  factor was whether the *by-reference* half of `use` has a real, non-substitutable use case, since that is
  the one thing implicit auto-capture cannot express.

## Decision

### 1. One closure literal, with or without a block body

```php
fn($x) => $x + 1                                  // expression body, implicit return — unchanged from PHP
fn($x) => { $y = $x + 1; return $y * 2; }         // block body, explicit `return` required — new
fn(int $x): int => $x + 1                         // typed params / return type — unaffected either way
```

`function(...) {...}` and `function(...) use (...) {...}` do not parse; the diagnostic names `fn` as the
replacement. This is not a new capability — first-class callable syntax and the `fn(...) => expr` literal
already produce exactly the value a block-bodied closure would — it removes a second spelling of the same
thing, the same argument [ADR 0015](0015-no-name-aliasing.md) already makes for names.

### 2. No `use` clause, ever — capture is implicit, by value, and minimal

A closure captures exactly the outer variables its body reads, snapshotted by value at the point the closure
literal is evaluated — the same rule PHP's arrow functions already use, generalized to the block-bodied form
too. There is no syntax to opt a variable in or out, and no by-reference capture:

```php
$y = 10;
$fn = fn($x) => $x + $y;        // captures $y by value — unchanged from an arrow function today
$fn = function($x) use (&$y) {  // rejected — no replacement syntax
    $y += $x;
};
```

Two consequences follow from dropping `use (&$y)` specifically, and both are deliberate:

- **Recursion** gets the self-name in § 3, not a captured reference to the closure's own variable.
- **Sharing one mutable cell between two independent closures** (a getter/setter pair, an accumulator fed
  from outside) has no builtin replacement. It is done in user code with an ordinary object, because
  capturing an object by value still shares the same heap object — only rebinding a bare scalar or
  copy-on-write `array<T>` local from inside a closure is actually lost:

  ```php
  class Counter { public int $value = 0; }
  $count = new Counter();
  $increment = fn() => $count->value++;   // both closures capture $count by value...
  $get = fn() => $count->value;           // ...but $count is a heap object, so they still share it
  ```

  No `Core\Ref<T>`/boxed-cell builtin is introduced for this. If real MWL code shows this pattern is common
  enough to deserve stdlib space, that is a narrower, separately-argued follow-up ADR — not a reason to keep
  `use (&$y)` around now on the strength of a hypothetical.

### 3. Recursion: an optional, purely lexical self-name

```php
$fact = fn factorial($n) => $n <= 1 ? 1 : $n * factorial($n - 1);
```

`factorial` is visible only inside that closure's own body. It is not a capture (it isn't in the set of
outer variables the body reads), not a second declared name reachable from anywhere else, and not a runtime
slot — it resolves the same way a method resolves `self::`, entirely at compile time, with no cost at
`fn`-literals that don't use it. It composes with both body shapes in § 1; it is not a third closure form.

This does not reopen [ADR 0011](0011-functions-and-constants-are-class-members.md)'s "every callable is a
declared class member" rule: that rule bars a *free, globally-callable* function existing outside a class.
`factorial` above is unreachable from anywhere but its own body — the same status as a parameter name, not a
declaration. A recursive helper that *is* reusable elsewhere still belongs on a class as a named method, per
that ADR; the self-name only covers the case where the sole reason a closure would otherwise need a name is
to call itself once.

### 4. `callable` absorbs `Closure`; nothing is left for `Closure` to mean on its own

`Closure` is retired as a type name. `callable` is the only spelling — for a parameter, a property, a return
type, or a stdlib signature like `Core\Arr::map(callable, array<T>): array<U>`. This is a rename, not a
behavior change: [ADR 0027](0027-callable-is-closures-only.md)'s decision (only a closure/arrow-function
value or a first-class-callable-syntax reference satisfies it; PHP's string/array spellings and `__invoke`
are still refused) is entirely unaffected, just read with `callable` in every place that ADR wrote `Closure`.

`callable` also reads more accurately than `Closure` did for values that capture nothing at all — a
first-class-callable-syntax reference to a static method (`Foo::bar(...)`) has no captured environment, so
calling that value "a closure" was always a slight misnomer inherited from PHP's own implementation detail
(the runtime class every callable value happened to be an instance of). "Callable" describes what the value
does, not how PHP happened to represent it.

The two operations `Closure::bind`/`bindTo`/`call` still exist, called with the same method-call syntax, now
under `callable` rather than a class named `Closure` — they are builtin operations on an opaque type, not
inherited methods from a base class a program could ever `instanceof` or extend, exactly as
[ADR 0007](0007-explicit-type-system.md) § 3 already made `Closure` opaque. `Closure::fromCallable` is
dropped: after [ADR 0027](0027-callable-is-closures-only.md), the only value that ever satisfied `callable`
was already a `Closure`/`callable` value, so there is nothing left for `fromCallable` to normalize away from.

### 5. `call_user_func`/`call_user_func_array` are dropped from the stdlib

Both exist in PHP only to dispatch through the string/array callable shapes
[ADR 0027](0027-callable-is-closures-only.md) already refuses. Every `callable` value now supports direct
invocation:

```php
$result = $fn($arg);          // replaces call_user_func($fn, $arg)
$result = $fn(...$args);      // replaces call_user_func_array($fn, $args)
```

### 6. Diagnostics

- `function($x) {...}` or `function($x) use ($y) {...}` → *anonymous `function` literals are not supported;
  use `fn($x) => ...` (an expression body) or `fn($x) => { ... }` (a block body)*
- `use ($y)` / `use (&$y)` on any closure → *closures have no `use` clause; every outer variable a closure's
  body reads is captured automatically, by value*
- `use (&$y)` specifically, where the diagnostic can tell the intent was mutation visible outside the
  closure → *capture by reference is not supported; share the value through an object property instead*
- `Closure` named as a type → *`Closure` is not a type in MWL; use `callable`*
- `Closure::fromCallable(...)` → *not supported; every `callable` value is already directly invocable*
- `call_user_func(...)` / `call_user_func_array(...)` → *not supported; call the value directly —
  `$fn(...)` / `$fn(...$args)`*

## Consequences

**Positive**

- One closure literal instead of two, one type name instead of two, one capture rule (implicit, by value,
  minimal) instead of two capture modes plus an opt-in clause — a strict reduction in the language's surface
  area with no loss of expressiveness for anything but the one narrow pattern named below.
- Removing `use (&$y)` removes a well-known PHP footgun for free: closures created inside a loop that
  capture the loop variable by reference and all end up observing the last iteration's value instead of
  their own. That is a correctness win (priority 2), not merely a simplification (priority 4).
- The self-name (§ 3) gives `mwl convert` a mechanical, human-free rewrite for the one `use (&$fn)` idiom
  that was load-bearing (self-recursion), rather than needing to synthesize a wrapper class the way the
  function-`static` rewrite in [ADR 0008](0008-static-and-global.md) does.
- `callable` replacing `Closure` costs nothing: [ADR 0007](0007-explicit-type-system.md) already made
  `Closure` opaque with no signature, so the rename carries no type-checker behavior change, only a spelling
  change propagated through diagnostics and stdlib signatures.
- `Closure::fromCallable`, `call_user_func`, `call_user_func_array` all become dead code with nothing left
  to do, once `callable` has exactly one shape — three fewer stdlib entries to implement and document.

**Negative**

- **A structural break from PHP**, joining the divergence list [ADR 0007](0007-explicit-type-system.md) § 7
  already carries: `function(...) use (...) {...}` is common PHP, and every occurrence needs conversion.
  Mechanical for `mwl convert` in the overwhelming majority of cases — rewrite to `fn`, drop `use ($y)`
  entirely since capture is now automatic — except `use (&$y)`, which needs a human decision between the
  self-name rewrite (recursion) and the wrapper-object rewrite (shared mutable cell), since the converter
  cannot always tell which one applies from the syntax alone.
- **Sharing mutable state between two independent closures has no first-class construct**, only the
  object-property pattern in § 2. This is a genuine capability gap relative to PHP, accepted deliberately
  rather than filled with a builtin boxed-reference type on the strength of a hypothetical need; see
  *Revisiting*.
- The stdlib loses three long-standing entry points (`Closure::fromCallable`, `call_user_func`,
  `call_user_func_array`); any converted code calling them needs the direct-invocation rewrite in § 5, which
  `mwl convert` can do mechanically.

## Alternatives rejected

- **Keep both `function(){}` and `fn() => ...`.** Two literal spellings producing an identical value, purely
  a matter of which one the author happened to type — the exact redundant-surface pattern
  [ADR 0015](0015-no-name-aliasing.md) already refuses elsewhere.
- **Keep `use ($y)` as documentation even though capture is now implicit.** Considered because an explicit
  capture list does document a closure's free variables at the call site. Rejected because it would be
  optional decoration with no enforcement — nothing would stop it from going stale relative to what the body
  actually reads — and "optional, unchecked annotation" is a shape MWL avoids everywhere else in the
  language.
- **Keep `use (&$y)`, drop only the by-value form.** Backwards from the real cost/benefit: by-value capture
  is the common, safe case and the one auto-capture already replaces for free; by-reference is the rare,
  footgun-prone case this ADR exists to remove.
- **A builtin `Core\Ref<T>`/boxed-cell type**, to give the shared-mutable-cell pattern a first-class,
  ready-made answer instead of "write a one-property class." Deferred, not rejected — see *Revisiting*.
- **Keep both `callable` and `Closure` as distinct spellings**, on the theory that a future typed-closure
  signature (`Closure(int): string`) might want `Closure` reserved for that. Rejected: if typed-closure
  signatures are ever added, the natural spelling is `callable(int): string`, consistent with every other
  parametric stdlib signature already using `callable` bare; nothing about keeping the type-signature
  question open (as [ADR 0007](0007-explicit-type-system.md) § 3 already leaves it) requires keeping two
  names alive today.
- **Drop the `fn` keyword for bare `($x) => ...`, JavaScript-style.** Rejected on parser grounds, not taste:
  a bare `(...)` before `=>` is ambiguous with a parenthesized expression until the parser has consumed the
  matching `)` and checked what follows — the same "arrow function head" problem JavaScript's own grammar
  needs a dedicated cover-grammar production to resolve. `fn` gives single-token lookahead for free;
  dropping it re-imports a parsing cost PHP's own choice of keyword already avoided, for no semantic gain
  and at the cost of being the one construct in MWL's surface grammar with no leading keyword at all.
- **Make blocks expression-valued** (the last statement's value becomes the block's value, no `return`
  needed, à la Rust), to unify the expression- and block-bodied forms even further. Rejected as far larger
  than this decision's scope: it would mean deciding statements-vs-expressions for the whole language, not
  just closures, in tension with [ADR 0007](0007-explicit-type-system.md) § 7's PHP-compatible-observable-
  behavior stance, for a benefit (saving one `return` keyword in block-bodied closures) this ADR does not
  need.

## Revisiting

- **A builtin boxed-reference type for the shared-mutable-cell pattern** should be reconsidered if the
  `.phpt`/real-code corpus imported in [M11](../implementation-plan.md) shows `use (&$y)` used for
  cross-closure sharing (as opposed to recursion, which § 3 already answers) is common enough that "write a
  one-property class every time" is a real ergonomics tax rather than a rare pattern. The fallback if so is
  a narrow, explicitly-named stdlib type — not reviving `use (&$y)` itself.
- **Typed closure signatures** (`callable(int): string`) stay exactly where
  [ADR 0007](0007-explicit-type-system.md) § 3 already left them, deferred rather than decided; this ADR's
  rename of `Closure` to `callable` does not need that question answered and does not reopen it.

Verification, in the order it becomes possible:

- **M1** (parser) — **done**: `fn($x) => expr`, `fn($x) => { ... }`, and `fn name($x) => ...`/
  `fn name($x) => { ... }` all parse to one AST shape (`FnExpr`/`FnBody` in
  [`mwl-syntax/src/ast.rs`](../../crates/mwl-syntax/src/ast.rs)); `function(...) {...}` and
  `function(...) use (...) {...}` are rejected with a diagnostic naming `fn` (`E0222`); `use (&$y)`/
  `use ($y)` on a closure literal is rejected with its own diagnostic distinguishing the by-reference case
  (`E0223`/`E0224`). `ClosureExpr`/`ClosureUse`/`ArrowFnExpr` no longer exist anywhere in the crate.
- **M2** (checker/resolver): a corpus entry for each rejected spelling in § 6; `callable` accepted everywhere
  the checker currently reads `Closure`, and `Closure` named as a type is refused with the diagnostic naming
  `callable`; a self-named closure resolves its own name only inside its own body and produces a resolver
  error if referenced anywhere else; a closure with a self-name that shadows an outer variable of the same
  name resolves to the self-reference inside the body without a diagnostic, the same shadowing rule ordinary
  nested scopes already use.
- **M4** (stdlib): `Closure::fromCallable`, `call_user_func`, `call_user_func_array` are not implemented;
  `Core\Arr::map`/`filter`/similar accept a `callable` argument exactly as before, with the name updated in
  their signatures.
