# ADR 0014 — Property hooks feed a declared `PropertyObserver`; no undefined-property fallback, no `__call`/`__callStatic`

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** what runs when a declared property is read or written beyond a plain field access — PHP
  8.4-style per-property hooks (already scoped for parsing in [M1](../implementation-plan.md)) and a new
  global `PropertyObserver` interface; what happens when the property named at a read or write site does
  not exist on the class; the explicit rejection of `__call`/`__callStatic` as a dispatch mechanism.
- **Amends:** [docs/implementation-plan.md](../implementation-plan.md) M4 — the bare "magic methods" line
  item is resolved into three concrete facts: property hooks stay exactly PHP 8.4's, a new global
  `PropertyObserver` interface replaces PHP's name-triggered `__get`/`__set`, and `__call`/`__callStatic`
  are not implemented at all. M2 — gains the property-access counterpart to the "no bare-name fallback"
  rule [ADR 0011](0011-functions-and-constants-are-class-members.md) already gives method and constant
  resolution: naming a property that does not exist on the class is refused at the same point, never
  deferred to a magic method.
- **Relates to:** [ADR 0002](0002-error-propagation.md) (a hook or `PropertyObserver` method that throws
  propagates as a checked status like any other call — no new exception path), [ADR 0004](0004-memory-for-simplicity.md)
  (this decision spends nothing beyond what [ADR 0013](0013-comparable-interface.md) already spent: an
  ordinary virtual call, paid only by a class that opts in), [ADR 0007](0007-explicit-type-system.md) (a
  property is a declared, typed binding; naming one that was never declared is refused the same way an
  undeclared local already is), [ADR 0008](0008-static-and-global.md) (no new storage class — a
  `PropertyObserver` implementation needs no entry in § 2's exhaustive table), [ADR 0011](0011-functions-and-constants-are-class-members.md)
  (why `PropertyObserver` is a global reserved interface and not a `Core` domain class — a contract type,
  not a holder of `static` members), [ADR 0013](0013-comparable-interface.md) (the direct precedent this
  ADR follows: replace a PHP behaviour that fires because a method *name* happens to be present with a
  behaviour that fires because an *interface* is declared)

> **In short:** PHP has two disconnected ways to intercept property access — 8.4's per-property hooks, and
> the ambient `__get`/`__set` pair, which fires only for a property that does not exist or is not
> accessible, triggered purely by a method with that exact name being present on the class. MWL keeps the
> first, replaces the second, and removes the case PHP's version exists to catch: **accessing an undeclared
> property is a hard error**, so there is nothing left for `__get`/`__set` to fall back onto. In its place
> is a new global interface, **`PropertyObserver`**, with `onPropertyGet(string $name, mixed $value): void`
> and `onPropertySet(string $name, mixed $value): void` — a class implements it once to observe *every*
> read and write of its own declared properties, hooked or not, from one place, instead of writing a hook on
> each property individually. A property access always runs the same two steps in the same order: first,
> the property's own hook (or plain field storage, if it declares none) produces or commits the value;
> second, if the class implements `PropertyObserver`, the matching method is called with the value the first
> step already settled on — purely to observe it. Its return type is `void`; it cannot override what the
> caller receives from a read or what was stored by a write. `__call`/`__callStatic` get no equivalent
> replacement: they are not recognized by name at all, calling a method that does not exist is already a
> compile-time diagnostic like any other unresolvable symbol, and a method literally named `__call` compiles
> as an ordinary method that the runtime simply never invokes on its own.

## Context

- PHP has three disconnected ways to intercept property/method access: property hooks (kept, already in
  [M1](../implementation-plan.md)'s parser scope), `__get`/`__set` (ambient, undefined-access only), and
  `__call`/`__callStatic` (same ambient shape for methods).
- The gap: PHP has no cross-cutting hook for *every* declared property, hooked or not — `__get`/`__set` only
  ever see undefined access, so real properties with real hooks get no shared observation point.
- Same "a method existing by this name changes behaviour" shape [ADR 0011](0011-functions-and-constants-are-class-members.md),
  [ADR 0012](0012-no-superglobals.md) and [ADR 0013](0013-comparable-interface.md) already closed elsewhere;
  [ADR 0013](0013-comparable-interface.md)'s `Comparable` replacement of property-walk `<`/`>` is the direct
  precedent followed here for `__get`/`__set`. `__call`/`__callStatic` get no replacement at all — dispatch by
  an unresolvable name is rejected outright.
- Undeclared-property access as a hard error extends [ADR 0007](0007-explicit-type-system.md)'s existing rule
  for undeclared locals to property access — not new territory.

## Decision

**A property access on a class always runs its own hook (or plain storage) first, then the class's
`PropertyObserver` methods if it implements that interface. Accessing a property that does not exist is
always a hard error — there is no `__get`/`__set` fallback for it. `__call` and `__callStatic` are not
recognized by name anywhere in the language.**

### 1. Per-property hooks — kept exactly as PHP 8.4 defines them

```php
class Temperature {
    public float $celsius;

    public float $fahrenheit {
        get => $this->celsius * 9 / 5 + 32;
        set(float $f) { $this->celsius = ($f - 32) * 5 / 9; }
    }
}
```

Nothing about a property's own `get`/`set` hook changes here — same syntax, same per-property scoping, same
interaction with `readonly` and asymmetric visibility PHP 8.4 already has. This ADR fixes only where a hook
sits relative to the new mechanism in *2* and *3*; the hook itself is not redesigned, and its finer semantics
(virtual vs. backed properties, hook visibility) belong to `docs/spec/`, which is unwritten — see
[CLAUDE.md](../../CLAUDE.md)'s routing table.

### 2. `PropertyObserver` — a new global reserved interface, not a `Core` domain class

```php
interface PropertyObserver {
    public function onPropertyGet(string $name, mixed $value): void;
    public function onPropertySet(string $name, mixed $value): void;
}
```

`PropertyObserver` lives in the global namespace, exactly where [ADR 0013](0013-comparable-interface.md)
put `Comparable` and PHP itself puts `Stringable` — **not** under `Core`.
[ADR 0011](0011-functions-and-constants-are-class-members.md) reserves `Core` for domain classes holding
`static` methods and constants; `PropertyObserver` holds neither, it is a contract an ordinary class
implements. `$name` is the property's declared name; `$value` is `mixed` because a class may have properties
of any type and one observer method has to see all of them, the same reason `mixed` is the escape hatch
[ADR 0007](0007-explicit-type-system.md) already reserves for exactly this shape of boundary. Both methods
return `void`: an observer reports, it does not decide — see *3*.

The name is deliberately not `__get`/`__set`. PHP's spelling signals "the runtime recognizes this name and
calls it for you," which is exactly the ambient behaviour this ADR replaces with a declared `implements`;
reusing the name after removing the mechanism it advertises would misdescribe what is actually happening at
the declaration site.

### 3. The pipeline: hook (or storage) first, observer second, always both

For **every** read or write of a property declared on a class implementing `PropertyObserver` — whether or
not that specific property has its own hook:

- **Read:** the value is produced exactly as it would be without `PropertyObserver` in the picture — by the
  property's own `get` hook if it declares one, by plain field storage otherwise. That settled value is then
  passed to `onPropertyGet($name, $value)`. The value the caller receives is the one the first step produced,
  not anything `onPropertyGet` returns — its return type is `void` precisely so there is nothing to return.
  If `onPropertyGet` throws, the read still fails: the exception propagates to the caller as a checked status
  like any other call ([ADR 0002](0002-error-propagation.md)), even though the value had already been
  resolved internally.
- **Write:** the property's own `set` hook processes and commits the incoming value if it declares one, or
  plain field storage assigns it otherwise — this step alone decides what is actually stored. The stored
  value is then passed to `onPropertySet($name, $value)` — the value the hook actually committed, not
  necessarily the caller's original argument, since a `set` hook may have transformed it first. As with
  reads, a throwing `onPropertySet` still fails the overall write.

This is a fixed two-step pipeline, not a fallback: a property with its own hook is **not** exempted from
`PropertyObserver`, and a property with no hook still reaches it. A class that wants one function to observe
every property write for logging, dirty-tracking or a serialization boundary gets exactly that, in addition
to — never instead of — whatever per-property hooks already do the real work of computing or validating a
value. `PropertyObserver` cannot itself become the property's storage: there is no way for it to supply a
value a `get` returns or override what a `set` stores, which is deliberate — see *Alternatives rejected* for
why letting it do so was rejected.

### 4. Zero cost for a class that does not implement `PropertyObserver`

Whether a class implements `PropertyObserver` is known at compile time from its declaration, exactly like
`Comparable` in [ADR 0013](0013-comparable-interface.md). A property with no hook on a class that does not
implement `PropertyObserver` compiles to a direct field load or store — no branch, no virtual call, nothing
paid by a class that never asked for either mechanism. The cost of this ADR is exactly one ordinary virtual
call per access, paid only by a class that implements `PropertyObserver`, which [ADR 0004](0004-memory-for-simplicity.md)
already treats as free relative to every other method call in the language.

### 5. Accessing an undeclared property is always a hard error — no `__get`/`__set` fallback

MWL has no dynamic properties: every property is declared with a type, per
[ADR 0007](0007-explicit-type-system.md), and that list is exhaustive for a given class. Naming one that is
not on the list:

- is a **compile-time diagnostic** when the property name is a literal identifier (`$obj->typo`) — the same
  point in the pipeline that already refuses an undeclared local, an unresolvable method or an unresolvable
  constant ([ADR 0007](0007-explicit-type-system.md), [ADR 0011](0011-functions-and-constants-are-class-members.md));
- is a **checked runtime throw** when the property name is only known at runtime (a computed property-access
  expression, or a reflection-based get/set) — the compiler cannot refuse it statically, but the set of valid
  names is still exactly the class's declared properties, and a name outside it throws rather than silently
  creating a new property the way PHP does (deprecated, but still permitted, before 8.2's opt-in
  `#[AllowDynamicProperties]`).

Either way, `PropertyObserver` is never consulted for a name that does not exist — unlike PHP, where
`__get`/`__set` exist *specifically* for that case. There is no path in MWL from "the name is wrong" to any
user code running at all.

### 6. `__call`/`__callStatic` — not recognized by name, no replacement offered

Neither name can even be declared: [ADR 0029](0029-identifier-casing-is-checked.md)'s method-casing rule
requires a lowercase-first `camelCase` name with no leading-underscore allowance of any kind — unlike the
allowance properties/parameters/locals had until [ADR 0030](0030-no-leading-underscores-constructor-spelling.md)
revoked it, methods never had one to begin with — so `__call`/`__callStatic` are rejected by the casing
checker before any call-resolution logic runs, the same generic diagnostic any other double-underscore
method name gets. That is a stronger version of this section's intent, not a different one: calling a
method that does not exist on a class is already a compile-time diagnostic under
[ADR 0011](0011-functions-and-constants-are-class-members.md)'s exhaustive resolution — the same shape *5*
gives properties — so there was never a runtime moment left for dynamic dispatch to intercept; now the name
itself cannot be written down, either. Unlike `__get`/`__set`, this ADR does not offer a declared-interface
replacement for `__call`/`__callStatic`: the requirement behind this decision rejects the concept of
dispatching to a name the class did not declare, not merely PHP's particular spelling of it. A program that
wants "handle a family of unknown calls" writes an ordinary method taking an explicit name and argument
list, or a `match`/lookup table keyed by name — visible in the class body, resolved and type-checked like
any other call.

## Consequences

**Positive**

- One declared place — `PropertyObserver` — covers every property a class has, hooked or not, closing the
  gap PHP's `__get`/`__set` never actually filled (they only ever saw *undefined* access).
- Costs nothing beyond an ordinary virtual call already priced by [ADR 0004](0004-memory-for-simplicity.md)
  and already paid the same way by [ADR 0013](0013-comparable-interface.md) — no new storage class
  ([ADR 0008](0008-static-and-global.md)), no new runtime representation, zero cost for the common case of a
  class that implements neither interface.
- Removes an entire class of "why didn't my `__get` fire" bug reports: there is no invisibility/accessibility
  precondition left to get wrong, since `PropertyObserver` runs for every declared property unconditionally.
- Deleting dynamic properties and `__call`/`__callStatic` outright removes two of PHP's more common sources
  of typo-driven bugs — a mistyped property or method name that silently does something instead of failing
  loudly.

**Negative**

- **A structural break from PHP**, joining the divergence list [ADR 0007](0007-explicit-type-system.md) § 7
  already carries forward: PHP source relying on `__get`/`__set` firing for undefined access, or on
  `__call`/`__callStatic`, does not convert unconverted. `mwl convert` ([M11](../implementation-plan.md))
  must flag these as a `TODO` for a human — see *Alternatives rejected* for why that flag belongs there and
  not in the compiler itself.
- `PropertyObserver`'s methods cannot change what a read returns or what a write stores. A PHP class that
  used `__get`/`__set` to *compute* or *transform* a value — not merely observe it — needs that logic moved
  into the specific property's own hook (*1*), which is the mechanism this ADR keeps for exactly that
  purpose. `PropertyObserver` and per-property hooks are not interchangeable, and a straight name-for-name
  conversion of `__get`/`__set` into `PropertyObserver` is only correct when the original method's job was
  observation, not computation.
- An observer sees the value already committed on a write, not the caller's original argument, if a `set`
  hook transformed it (*3*) — a class that specifically wants the pre-hook raw input has no declared way to
  see it under this ADR.

## Alternatives rejected

- **Ambient name-based `__get`/`__set`**, matching PHP exactly. Rejected: same "behaviour triggered by a name
  being present, not a declaration" shape [ADR 0011](0011-functions-and-constants-are-class-members.md),
  [ADR 0012](0012-no-superglobals.md) and [ADR 0013](0013-comparable-interface.md) already closed.
- **A property's own hook as a fallback**, with `PropertyObserver` only running for hookless properties.
  Rejected: defeats the cross-cutting use case motivating this ADR, forcing that logic to be duplicated into
  every hook instead of declared once.
- **Letting `PropertyObserver` override the value.** Rejected: a second authority able to override a
  hook-decided value creates an ordering question with no non-arbitrary answer; keeping it strictly
  observational (`void`) removes the question entirely.
- **A language-level diagnostic on declaring `__call`/`__callStatic`.** Rejected as a permanent compiler
  feature — a one-off lint with no general principle behind it, when `mwl convert` ([M11](../implementation-plan.md))
  already has the full context to explain why the conversion has no mechanical destination.
- **Threading the pre-write value into `onPropertySet` alongside the committed one**, so an observer can
  compare old and new. Deferred rather than rejected — see *Revisiting*.

## Revisiting

Deferred deliberately, each needing its own argument:

- **Whether `onPropertySet` should also receive the property's previous value**, for change-detection use
  cases that want to compare old and new rather than only observe the new. Not decided here — adding it is a
  signature change to an interface with no implementations yet, so it costs nothing to defer.
- **Per-property hook semantics themselves** — virtual vs. backed properties, hook visibility, interaction
  with `readonly` and asymmetric visibility — belong to `docs/spec/`, unwritten as of this ADR. This document
  fixes only where hooks sit relative to `PropertyObserver`, not their own internal rules.
- **`mwl convert`'s exact `TODO` wording** for PHP source that declares `__get`/`__set` (does the original
  logic look like observation, convertible to `PropertyObserver`, or computation, belonging in a per-property
  hook?) and for `__call`/`__callStatic` (no mechanical destination at all). Belongs with M11's own design,
  not this ADR.

Verification, in the order it becomes possible:

- **M1**: property hooks parse per PHP 8.4's grammar, already scoped; `interface PropertyObserver { ... }`
  parses with the grammar M1 already gives interfaces — nothing new here, since this ADR adds no new syntax
  beyond an ordinary interface declaration.
- **M2**: the checker refuses a property access naming anything not declared on the class (or an ancestor)
  with a diagnostic, for every literal-identifier access, on every receiver — `mwl_hir::members` for
  `$this` (`E_UNDEFINED_PROPERTY`), `mwl-types::expr::check_property_access` for every other statically
  resolvable receiver (`E_UNKNOWN_MEMBER`) — joining the diagnostic corpus
  [ADR 0007](0007-explicit-type-system.md)'s own M2 entry already builds; a method named `__call` or
  `__callStatic` never reaches any resolution logic at all, since `mwl-syntax`'s casing check
  ([ADR 0029](0029-identifier-casing-is-checked.md)) already refuses the name itself.
- **M4**: a class implementing `PropertyObserver` runs its property's own hook (or storage) first and
  `onPropertyGet`/`onPropertySet` second, for both hooked and un-hooked properties, including a throwing
  observer method propagating correctly through [ADR 0002](0002-error-propagation.md)'s checked-return path;
  a class that does not implement `PropertyObserver` shows no measurable overhead over plain field access,
  committed alongside [ADR 0013](0013-comparable-interface.md)'s equivalent guard; a runtime-computed
  property-access expression naming an undeclared property throws rather than creating one; calling an
  undeclared method still fails even when the class defines `__call`.
- **M11**: the converter flags PHP source that declares `__get`/`__set` as needing human review (observation
  → `PropertyObserver`, computation → a per-property hook) and PHP source that declares `__call`/
  `__callStatic` as a `TODO` with no mechanical destination, per *Consequences*' negative list.
