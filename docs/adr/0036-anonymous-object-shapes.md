# ADR 0036 — `object` is the opaque top of every class type; `{...}` builds an anonymous, methodless instance; an inline `{name: T, ...}` shape is Novis's one structurally-checked type

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** PHP's `stdClass` and the general "a bag of named values, shared by reference, without declaring
  a class" need — the `object` type atom (already reserved in `nvs-syntax`/`nvs-types` but never given
  subtyping semantics), a new anonymous object-literal expression (`{a: 1, b: 2}`), and a new inline
  structural shape type (`{name: T, ...}`) usable anywhere a type is expected. Does not add methods,
  inheritance, or any nominal contract to these values — they stay pure data.
- **Amends:** [ADR 0007](0007-explicit-type-system.md) — `object` moves from a reserved-but-unused atom to
  a real member of the type lattice: every class type, named or anonymous-literal, is now a subtype of it,
  and it is in turn a subtype of `mixed`. [ADR 0014](0014-property-observer.md) § 5 — its "a computed
  property name is a checked runtime throw, never a fallback" rule gains a second trigger: an *erased
  receiver type* (plain `object`, or a shape missing the named field), not only a *dynamic name*; and gains
  a case § 5 never needed before — a write whose target's real field type isn't statically visible through
  the erased view is also a checked runtime type-check, not just a name check. [ADR 0031](0031-callable-is-the-only-closure-type.md) —
  this is the "ordinary object in user code" its own § 2 already pointed at for two closures sharing mutable
  state; it also documents the one new grammar wrinkle that decision's block-body dispatch creates for this
  literal (see *Decision § 2*). [docs/implementation-plan.md](../implementation-plan.md) M1 — gains a third
  pending grammar item (the literal and the shape-type syntax), M2 — gains the atom's real subtyping and the
  shape's structural check.

> **In short:** PHP's `stdClass` needs dynamic, undeclared properties — closed for good reason in
> [ADR 0014](0014-property-observer.md) — so Novis cannot offer it directly. Instead: **`object` becomes the
> real, opaque supertype of every class type** (named or anonymous), reusing a keyword and atom that were
> already reserved but never wired up. **`{a: 1, b: 2}` is sugar that instantiates a compiler-synthesized,
> methodless class** with exactly those fields, types inferred from the initializers — no constructor, no
> `implements`, ordinary object (shared-by-reference) semantics, because it *is* an ordinary object. Passing
> one across a function boundary typed plainly as `object` needs no shape declared anywhere; the receiving
> function can still read and write it, at the price of a runtime-checked name/type lookup instead of a
> compile-time-verified field access — the same checked-throw shape [ADR 0014](0014-property-observer.md) §5
> already committed to for a dynamically computed property name, now also triggered by an erased *receiver*
> type. For the case where compile-time field safety *is* wanted without declaring a class, **an inline
> shape type — `{name: T, ...}` — is usable directly in a signature**: a structural, width-subtyped,
> compile-time-only constraint checked at each call/assignment site against the source's real, already-known
> type. This is the **one deliberate exception** to Novis's otherwise fully nominal type system, and it is
> scoped tightly to this one type family — `Comparable`, `PropertyObserver` and ordinary `interface`
> satisfaction stay exactly as nominal as [ADR 0013](0013-comparable-interface.md)/[ADR 0014](0014-property-observer.md)/`rule:statements/nothing-gets-a-second-name`
> already left them.

## Context

- The user asked for PHP's `stdClass` — a way to pass a shared, named bag of values across a function
  boundary without declaring a class. Novis cannot offer it as-is: every property must be declared and typed
  ([ADR 0007](0007-explicit-type-system.md)), and accessing/creating an undeclared one is a hard error with
  no fallback ([ADR 0014](0014-property-observer.md) § 5) — exactly the mechanism `stdClass` needs closed.
- [ADR 0031](0031-callable-is-the-only-closure-type.md) already named the gap this fills: two closures
  sharing mutable state need "an ordinary object in user code," with no ceremony-reduced way to write one.
- A fully general, first-class structural record type (interned/canonicalized everywhere a type can appear)
  was explored first and rejected as more machinery than the need justifies (*Alternatives rejected*). The
  surviving design splits into two small pieces: an opaque top type for every object, and a narrow,
  call-site-local structural check for the one case an opaque view isn't enough.
- `object` was already a reserved keyword and interned type atom (`Keyword::Object`, `Ty::Object`) with no
  subtyping wired up — this ADR finishes an atom already sitting in the grammar, not a new addition.
- Two grammar collisions were found in `nvs-syntax/src/parser.rs`: `parse_fn_expr` already commits `{` after
  `=>` to a block body, and `parse_statement_inner` commits a statement-initial `{` to a block statement —
  the identical ambiguity JavaScript has for `() => {...}`, solved the identical way (parenthesize).

## Decision

**`object` is the opaque supertype of every class type. `{name: value, ...}` builds an anonymous, methodless
instance of a compiler-synthesized class. `{name: T, ...}` in type position is a structural shape type,
checked at compile time by width subtyping and ordinary field-type assignability — Novis's one deliberate
exception to nominal typing, scoped to this type family alone.**

### 1. `object` — real subtyping for an atom that already existed

Every class type — a named, declared class, or the anonymous class a literal synthesizes (*2*) — is a
subtype of `object`. `object` itself carries no field or method information statically; it is to the class
hierarchy what `mixed` is to the whole type system, one level narrower. `object <: mixed`, and every class
type `<: object`. No class needs to declare anything to satisfy it — this is the one place membership in a
type is structural by construction, but only because `object` promises nothing about shape at all, so there
is nothing to check.

### 2. `{name: value, ...}` — an anonymous, methodless object literal

```php
$point = {x: 1, y: 2};
$box   = {count: 0};
```

Each occurrence's *precise* type is a private, compiler-synthesized class with exactly the fields written,
each field's type inferred from its initializer expression — the same inference already used for an
`array<T>` literal's element type. The synthesized class:

- has **no methods**, no `implements`, no user-reachable name — it exists only for the checker and codegen,
  never appears in a diagnostic as anything other than "an anonymous object with fields `x: int, y: int`";
- needs **no constructor**: the literal itself assigns every field it declares, satisfying
  [ADR 0022](0022-definite-property-initialization.md) § 2's definite-assignment obligation by construction,
  the identical shape a promoted constructor parameter already gets;
- is an **ordinary object** in every other respect — reference (shared, not copy-on-write) semantics
  identical to any class instance, because it is one; `clone` and `serialize`/`unserialize` work on it with
  no special case, via [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md)'s existing, uniform
  mechanism, since it has real (if compiler-named) declared properties;
- field names are ordinary property names, so [ADR 0029](0029-identifier-casing-is-checked.md)'s
  `camelCase` rule and [ADR 0030](0030-no-leading-underscores-constructor-spelling.md)'s no-leading-`_` rule
  both apply unchanged.

**Deliberately not supported**, to keep "no dynamic properties, ever" intact: no shorthand `{x, y}` (every
field is written `name: value`), and no computed/dynamic key (`{[$expr]: 1}`). Every field name in a literal
is a static identifier, full stop — there is no path from this literal back to PHP's dynamic-property
behaviour.

**Grammar note, found by reading `nvs-syntax/src/parser.rs` directly rather than assuming:** `parse_fn_expr`
already parses `{` immediately after `=>` as the start of a block body ([ADR 0031](0031-callable-is-the-only-closure-type.md)),
so `fn() => {a: 1, b: 2}` parses as a block body attempting to parse `a: 1, b: 2` as statements, not as a
returned literal. Returning a literal directly from an expression-bodied arrow needs the same fix
JavaScript already uses for the identical ambiguity: `fn() => ({a: 1, b: 2})`. The same applies to a bare,
discarded literal used as a top-level statement (`parse_statement_inner` already commits a statement-initial
`{` to a block) — parenthesize to force the expression reading. Both are closed, already-identified call
sites, not an open-ended grammar ambiguity.

### 3. `{name: T, ...}` in type position — an inline structural shape type

```php
function move(object {x: int, y: int} $p): void { $p->x += 1; }
type Point = {x: int, y: int};                     // reusable name, via `rule:statements/nothing-gets-a-second-name`'s existing `type` alias
```

A shape type is **not** a class and **not** an interface — it is a compile-time-only structural constraint,
checked once at each site a value flows into a binding of this type (assignment, call argument, return):
the source's already fully known type (a named class or a literal's synthesized shape from *2*) must have
at least the named fields, each satisfying the shape's declared type by the same assignability rule already
used for every other parameter — no new comparison logic invented for this.

- **Width subtyping**: a source with *extra* fields beyond the shape still satisfies it. An already-shaped
  value never needs re-wrapping just because a caller only cares about two of its five fields.
- **Field-type compatibility**: ordinary assignability, unchanged from anywhere else a type is checked — a
  shape does not get its own variance rule.
- `object` with no shape remains the fully erased form; every shape type is a subtype of plain `object`.
- Reusable for free via `rule:statements/nothing-gets-a-second-name`'s existing `type` alias mechanism: a shape is a
  type *expression*, never a single bare class, so it was never excluded by that ADR's one restriction.
- **This is the one deliberate, tightly scoped exception to Novis's otherwise fully nominal type system.**
  Two unrelated named classes that happen to share field names/types become interchangeable wherever a shape
  type is used. This is intentional and is exactly what delivers "no shape needs to be declared anywhere for
  two sides to agree" — but it applies only to this type family. `Comparable`
  ([ADR 0013](0013-comparable-interface.md)), `PropertyObserver` ([ADR 0014](0014-property-observer.md)), and
  ordinary `interface` satisfaction all stay exactly as nominal as their own ADRs already decided; nothing
  here reopens any of them.

### 4. Property access through an erased view — extends ADR 0014 § 5

A field named by a shape type, accessed through that shape, is proven present at compile time — reading it
never throws. Until or unless a future optimization specializes per call site (see *Revisiting*), the actual
fetch is still a name-keyed lookup rather than a fixed offset, since two different concrete objects
satisfying the same shape may lay fields out differently; the compile-time proof buys "cannot fail," not yet
"as cheap as a plain field load."

A field accessed through plain `object`, or a name a shape does not list, cannot be checked at compile time
at all. This is [ADR 0014](0014-property-observer.md) § 5's already-decided fallback for a *dynamically
computed* property name, now also triggered by an *erased receiver type*.

**A `mixed` receiver is the third trigger, and the widest**: [ADR 0007](0007-explicit-type-system.md) § 2
makes it the one unchecked position, so `$m->name` defers not only which class is behind the handle but
whether there is one at all. The fetch below is what answers both, and a receiver whose tag turns out not
to be an object is one more catchable throw, worded as PHP words its warning. Every receiver whose
*declared* type can hold no object — a scalar, an `array<T>`, a union naming no single class — is refused
where it is written instead (`E0495`, ADR 0007 § 7 row 13): the deferral is what `mixed` is for, and a type
that already answers the question does not get to ask it again at run time. The two rules below apply to
all three triggers:

- **Read:** a checked, catchable throw if the concrete instance behind the handle does not actually have
  that name — never a silent value, never PHP's warning-plus-`null`.
- **Write:** the same missing-name throw, plus a case § 5 never needed before, because every prior case it
  covered still had a statically known field type up to the point of erasure — a write's incoming value is
  checked against the field's *real*, concrete declared type, throwing the same kind of checked, catchable
  error on a mismatch. A write through an erased view can never create a new field, matching § 5's existing
  "no dynamic creation" rule exactly.

Both throws are ordinary `Throwable`s, propagated by checked return like any other call
(`rule:errors/propagation`), never routed through the fatal escalation ladder
(`rule:errors/escalation-ladder`) — the same classification [ADR 0022](0022-definite-property-initialization.md) §3
already gives its own residual runtime case.

### 5. What this deliberately does not add

No methods, no `implements`, no inheritance, no `Comparable` or `PropertyObserver` support for an anonymous
literal's synthesized class — there is nothing to hook since there is no method body to write one in. This
stays a pure, methodless data carrier. A program that wants behaviour on a shared bag of values already has
the answer: declare an ordinary class.

## Consequences

**Positive**

- Closes the exact gap [ADR 0031](0031-callable-is-the-only-closure-type.md) already named — "an ordinary
  object in user code" for sharing mutable state between closures — with a genuinely lightweight way to
  write that object, instead of a full structural-record subsystem.
- Reuses four already-decided mechanisms end to end rather than inventing new ones: ADR 0014 § 5's
  runtime-checked fallback, ADR 0022's promoted-parameter-style trivial definite assignment, `rule:statements/nothing-gets-a-second-name`'s
  `type`-alias reuse, and ADR 0023's uniform clone/serialize/isolate-crossing.
- Zero new runtime representation: a literal instance is an ordinary object allocation, exactly
  `rule:programs/memory-priority`'s existing accounting for any object — no tagged-value
  discriminant, no boxing scheme unique to this feature.
- Delivers "pass it across a function boundary with no shape declared anywhere" — the specific ergonomic
  asked for — without making the general type system structural. The exception is named and bounded, not
  incidental.

**Negative**

- **The one deliberate crack in an otherwise fully nominal type system.** Worth restating plainly so no
  future reader assumes Novis is structurally typed anywhere else: it is not — only `object` and shape types
  work this way.
- Property access through an erased `object` view, or a field a shape doesn't list, costs a real
  runtime name lookup (and, for writes, a type check) that a fully statically-known object never pays. This
  is an honest, opt-in latency line item under `rule:programs/memory-priority` priority 3 — paid
  only by code that chose to erase shape information, never by code that didn't.
- Two grammar collisions, both closed with a known fix but both real surface to document: `fn() => {...}`
  means a block body, not a returned literal, unless parenthesized; a bare literal statement needs the same.
- PHP's `stdClass`/dynamic-property idiom still has no mechanical translation — a PHP object built by
  assigning arbitrary properties after construction has no Novis literal equivalent, since every field must be
  fixed at the point of construction. `nvs convert` (M11) must flag this as a `TODO`, joining the TODO
  classes [ADR 0007](0007-explicit-type-system.md) § 7 and [ADR 0014](0014-property-observer.md) already
  carry.

## Alternatives rejected

- **A fully general, first-class structural record type**, interned/canonicalized globally and usable
  anywhere a type can appear. Rejected: sizable checker machinery (shape canonicalization, structural
  subtyping threaded through the whole lattice) for a need the opaque-`object`-plus-local-shape-check design
  already meets far cheaper. Revisitable if real converted code shows the local check insufficient.
- **Reusing the `interface` keyword** for shapes (TypeScript's model). Rejected: PHP/Novis's `interface`
  already means a nominal, `implements`-declared, method-bearing contract; overloading it for something
  structural and property-only is the same "two meanings, one name" problem
  `rule:statements/nothing-gets-a-second-name` already refuses elsewhere.
- **PHP's `stdClass` directly.** Rejected outright: it requires dynamic, undeclared properties, which
  [ADR 0014](0014-property-observer.md) closes for exactly this reason.
- **Exact-match shape types (no width subtyping).** Rejected: strictly narrower for no safety benefit, and
  would force re-wrapping an already-compatible value just to add one more field elsewhere in the program.

## Revisiting

Deferred deliberately, each needing its own argument once there is real code to argue from:

- **Per-call-site specialization** of shape-typed field access to a fixed offset instead of a name-keyed
  lookup, if the erased-access cost in *Decision § 4* shows up on a hot path in practice. Not attempted here.
- **Whether a shape type should ever require a method**, making it a hybrid structural interface rather than
  pure data. Deliberately out of scope — this ADR's shapes are data-only, matching the original request.
- **`nvs convert`'s exact handling of PHP's `stdClass`/post-construction dynamic-property patterns** —
  belongs to M11's own design, per *Consequences*' negative list.

Verification, in the order it becomes possible:

- **M1**: `{name: value, ...}` parses as a primary expression producing an anonymous-literal AST node; no
  shorthand and no computed key parse (both refused with a diagnostic naming the static-identifier
  requirement); `{name: T, ...}` parses in every type position `token_starts_type` already recognizes;
  `fn() => {...}` without parentheses reports the existing block-body-dispatch diagnostic naming the
  parenthesize fix, and a bare `{...};` statement gets the equivalent diagnostic.
- **M2**: `object` carries real subtyping — every named or literal-synthesized class type is provably `<:
  object`; a shape type is checked structurally (width subtyping plus ordinary field assignability) at every
  assignment, call argument and return; a `type` alias naming a shape type resolves exactly like any other
  `type` alias; [ADR 0014](0014-property-observer.md) § 5's diagnostic path is extended to fire for a read or
  write through plain `object` or a shape missing the named field, joining that ADR's own corpus.
- **M4**: reading a field proven by a shape never throws; reading through plain `object`, or a field a shape
  doesn't list, throws a checked `Throwable` if genuinely missing and never creates one; writing through
  either erased view type-checks the incoming value against the concrete field's real declared type and
  throws on mismatch; a literal instance clones, serializes and crosses the isolate boundary exactly like any
  other object, per [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md).
