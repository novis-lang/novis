# ADR 0038 — `lateinit` defers a non-nullable object property's first assignment past the constructor

- **Status:** Accepted
- **Date:** 2026-08-22
- **Scope:** one new property modifier, `lateinit`, usable only on a non-nullable, class/interface-typed
  property. Nothing else changes: scalar-typed properties, `?T` properties, locals, and every other rule
  [ADR 0022](0022-definite-property-initialization.md) states keep working exactly as before.
- **Amends:** [0022](0022-definite-property-initialization.md) — resolves the *Revisiting* entry "An opt-in
  `lateinit`-equivalent" that ADR left open, and reuses rather than replaces its § 3 runtime mechanism.
- **Relates to:** 0002, 0004, 0014, 0017

> **In short:** `lateinit` marks a non-nullable, object-typed property as exempt from
> [ADR 0022](0022-definite-property-initialization.md) § 2's constructor-must-assign rule, for the case that
> ADR named but declined to solve — a DI container, an ORM, or any setter-injection pattern that populates a
> property after `new` returns rather than inside a constructor. It adds **no new type or value**: a
> `lateinit` property is exactly as non-nullable as any other, it just carries the "never written" tag ADR
> 0022 § 3 already defined for `Core\Reflect`-constructed objects, now reachable from ordinary code too. A
> read before the first write throws the same checked, catchable error ADR 0022 § 3 already throws in that
> case — this ADR opens up an existing mechanism rather than building a second one. `lateinit` is restricted
> to non-nullable class/interface types only (a scalar always has a free, real default — `= 0`, not a
> deferred one — so there is nothing for `lateinit` to buy there), and once written, a `lateinit` property is
> an ordinary, freely reassignable property with no write-once restriction; it cannot be combined with
> `readonly`, whose write-once contract is the opposite promise. Alongside the runtime check, one
> **intraprocedural, false-positive-free** compile-time check is added for free — it reuses the definite-
> assignment dataflow ADR 0022 already runs, so it costs nothing new to build, but it only proves the
> narrow, calls-free case; nothing wider is attempted, because that would need whole-program analysis that
> fights [ADR 0017](0017-hot-reload-without-restart.md)'s per-file hot-reload model (see *Alternatives
> rejected*).

## Context

- ADR 0022 committed to compile-time definite assignment for every non-nullable property but deliberately
  left one case open: a DI container or ORM that populates properties after `new` rather than inside a
  constructor. That pattern is common enough in ported PHP (a container/setter filling in dependencies after
  construction, a circular reference between two objects, an ORM hydrating a lazy relation) that requiring
  `?T` forever or a placeholder instance is a real ergonomic wall, not a hypothetical one.
- Kotlin's `lateinit var` is the direct precedent: an opt-in modifier exempting a property from
  definite-assignment checking, deferring the guarantee to a runtime throw on first read, restricted to
  non-primitive, non-nullable types for the same reason restated in *Decision § 1* — a primitive already has
  a free real default.

## Decision

**A non-nullable, class- or interface-typed property may be declared `lateinit`. Doing so exempts it from
ADR 0022 § 2's constructor-must-assign obligation. Reading it before its first write throws the same
checked `Throwable` ADR 0022 § 3 already defines for a `Core\Reflect`-constructed object's unwritten
property — this ADR extends that throw's reachability from reflection-only construction to any `lateinit`
property, on any object, however it was constructed. No new type, no new value, no new runtime mechanism.**

### 1. Where `lateinit` may appear

- Only on a non-nullable property of a class or interface type (`object`, a named class, or a named
  interface). `int`, `uint`, `float`, `bool`, `string`, `bytes`, enums, and every other scalar type are
  refused with `E_LATEINIT_NOT_OBJECT_TYPE`, naming the fix: give the property a real inline default and
  reassign it later, same as any other scalar. This is not a hardship — a scalar's "no value yet" is already
  free (`= 0`), which is exactly the case `lateinit` exists to cover for object types that have no such free
  placeholder.
- `?T lateinit` is refused with `E_LATEINIT_NULLABLE` — nullability already spells "may legitimately hold no
  value" ([ADR 0022](0022-definite-property-initialization.md) § 1); a nullable property has no need for a
  second, overlapping "not yet set" state, and `lateinit ?T $x;` read before any write would leave open
  exactly the ambiguity ADR 0022 § 1 exists to close (is a `null` read the deliberate value, or the "not yet
  written" tag surfacing?).
- A promoted constructor parameter (`public Foo $x` in the parameter list) cannot be `lateinit`:
  `E_LATEINIT_PROMOTED_PARAM`. Binding the parameter already is the assignment ADR 0022 § 2 requires, so
  there is nothing left to defer — the two modifiers contradict each other by construction.
- `lateinit readonly` is refused with `E_LATEINIT_READONLY_CONFLICT`. `readonly`'s contract is "assigned
  exactly once, and that assignment happens during construction"; `lateinit`'s contract, decided here, is
  "assigned after construction, and freely reassignable thereafter." The two are opposite promises about the
  same property, not a composable pair. (A write-once-but-deferred variant was considered and deferred — see
  *Revisiting*.)
- A class with no constructor of its own is unaffected: ADR 0022 § 2's "refused at the property declaration"
  rule simply does not fire for a `lateinit` property, because there is no constructor-assignment obligation
  to check in the first place.

### 2. Runtime semantics — reusing ADR 0022 § 3's mechanism, not building a second one

A `lateinit` property's storage carries the same "never written" tag ADR 0022 § 3 already introduced for
`Core\Reflect`-constructed objects, at the same zero-additional-bytes cost (one more discriminant on the
existing tagged-value representation, per [ADR 0004](0004-memory-for-simplicity.md)'s accounting — nothing
new to account for here). What changes is only *which properties can carry that tag outside of reflection*:
before this ADR, only a `Core\Reflect`-bypassed construction could leave a non-nullable property unwritten;
after it, an ordinarily-constructed object can too, for any property its class marked `lateinit`.

- Reading a `lateinit` property that has never been written throws the identical checked `Throwable` ADR
  0022 § 3 defines — an ordinary, catchable error, not routed through
  [ADR 0020](0020-error-escalation-ladder.md)'s fatal ladder, propagated exactly like any other checked
  failure ([ADR 0002](0002-error-propagation.md)).
- Writing a `lateinit` property — the first time or any later time — behaves exactly like writing any other
  mutable property. There is no write-once tracking: once assigned, it stays an ordinary property for the
  rest of the object's life, indistinguishable from one that was assigned in the constructor.
- Nothing here changes how a `lateinit` property crosses `clone`, `serialize()`/`unserialize()`, or an
  isolate boundary ([ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md)): a copy's source object is
  already fully in scope for that operation, and if the source itself still holds an unwritten `lateinit`
  property, the copy inherits the same unwritten tag and throws under the same rule on the same first read —
  no special case needed.

### 3. The one compile-time check added — intraprocedural, and provably free of false positives

ADR 0022's definite-assignment dataflow already computes, at every program point inside a function body,
whether a given property is provably assigned yet. Extending that same pass to also run inside *every*
method (not only constructors), with a `lateinit` property entering the method in the "not yet proven
written" state instead of being exempt from the check outright, catches one narrow, genuinely free case: a
read of a `lateinit` property reachable from the method's entry with **no intervening write to it and no
intervening call to any method or function** on that path. A call is treated as opaque and immediately
moves the property to "assumed written" — the analysis cannot know whether the callee wrote it, and the
sound direction is to stay silent rather than risk a false positive, since a check that cries wolf on
legitimate DI-populated code would cost more in trust than it gives back in caught bugs. This means the
check fires only on the narrowest shape — reading a `lateinit` property in the same straight-line-or-branchy,
call-free stretch of code before it is ever set — and says nothing at all about the cross-method,
cross-object case that motivated `lateinit` in the first place; that case still relies entirely on § 2's
runtime throw. The diagnostic is `E_LATEINIT_READ_BEFORE_WRITE_LOCAL`, phrased as a warning-shaped compile
error rather than a suggestion, since — within the narrow shape it covers — it is exactly as certain as any
other definite-assignment diagnostic ADR 0022 already emits.

No interprocedural or whole-program analysis is added — see *Alternatives rejected* for why that option,
raised and considered, is not worth its cost here.

## Consequences

**Positive**

- Closes ADR 0022's own deferred item with the smallest addition that actually solves the motivating case:
  DI/setter injection and ORM-style post-construction population no longer force a property to be `?T`
  forever or backed by a placeholder instance it never legitimately holds.
- Zero new runtime mechanism and zero new memory cost: the "never written" tag and its throw already exist
  from ADR 0022 § 3; this ADR only widens which properties may carry it.
- The one compile-time addition reuses the existing definite-assignment pass rather than adding a second
  analysis, and is constructed to never produce a false positive, so it costs nothing in code trust for
  whatever small number of bugs it does catch.
- Keeps ADR 0022's core guarantee — no new `undefined` type, no silent per-type default — completely intact.
  `lateinit` does not weaken the promise a non-nullable type makes about its *value space*; it only moves
  *when* the one, still-mandatory real assignment is allowed to happen, and still fails loudly, never
  silently, if that promise is broken.

**Negative**

- A second way for a non-nullable property read to throw at runtime (alongside `Core\Reflect`-constructed
  objects), now reachable from code that never touches reflection at all. This is the accepted cost of
  solving the motivating case — the alternative (forcing `?T` everywhere) was the ergonomic wall this ADR
  exists to remove.
- `lateinit` is one more modifier to teach, with three rejected-combination diagnostics (`?T`, promoted
  parameter, `readonly`) a developer has to learn are rejected rather than silently allowed.
- The compile-time check in § 3 is narrow enough that most real `lateinit` usage will never trigger it —
  it is a genuine bonus, not a safety net developers should rely on. That expectation has to be documented
  clearly, or its rare successes will be mistaken for a broader guarantee than it makes.

## Alternatives rejected

- **Interprocedural / whole-program compile-time checking**, tracing which methods can write a `lateinit`
  property before which methods read it. Rejected: MWL's hot-reload model
  ([ADR 0017](0017-hot-reload-without-restart.md)) revalidates and swaps one file's compiled unit at a time;
  a whole-program analysis would widen that blast radius on every edit, for a check whose easy cases § 3's
  free intraprocedural pass already covers and whose hard cases are cross-function/cross-object by nature and
  would fall through to the runtime throw anyway.
- **A write-once ("deferred readonly") variant**, assignable exactly once at any point, then frozen. Deferred
  rather than rejected outright (*Revisiting*) — it needs its own tracked write-count check this ADR's
  freely-reassignable design doesn't build, and no real code exists yet to argue the shape from.
- **Allowing `lateinit` on any non-nullable type, including scalars.** Rejected, per Kotlin's own precedent:
  a scalar always has a free, real default (`= 0`, `= false`, `= ""`), so `lateinit` there would only be a
  lazier spelling of writing that default and reassigning later.
- **Leaving this to the `Core\Reflect` escape hatch alone.** Rejected: that forces ordinary setter injection
  to route through reflection-based construction just to get an initialization order the language should
  express directly, and reflection's throw-on-unwritten-read behavior is exactly what `lateinit` needs.

## Revisiting

- **A write-once `lateinit` variant, composable with something like `readonly`'s intent.** If real ported
  code shows a common pattern of "written exactly once, by exactly one caller, then must never change again,
  but not necessarily during construction," this deserves its own ADR rather than retrofitting one here —
  it needs a tracked write-count check this ADR's freely-reassignable design does not build.
- **Whether a `lateinit` property backed by a `set` hook** ([ADR 0014](0014-property-observer.md)) should
  discharge its "never written" tag the moment the hook first commits a value, mirroring how ADR 0022 § 2
  already treats a hooked property inside a constructor. Left to `docs/spec/`, the same deferral ADR 0022
  already uses for its own per-property hook internals.
- **Whether `mwl convert` (M11) should recognize a PHP class's setter-injection pattern and suggest
  `lateinit`** rather than leaving the converted property `?T` with manual null-checks everywhere. A
  migration-quality question, not a language-semantics one — belongs with ADR 0022's own M11 entry once that
  milestone starts.

Verification, in the order it becomes possible:

- **M2**: the checker accepts `lateinit` only on a non-nullable class/interface-typed property, refusing
  `?T lateinit`, a `lateinit` promoted parameter, and `lateinit readonly` with the three diagnostics named
  in § 1; a `lateinit` property does **not** trigger ADR 0022 § 2's constructor-must-assign diagnostic; the
  § 3 intraprocedural check fires on a call-free read-before-write inside one method and stays silent once
  any call intervenes on that path.
- **M4**: reading a never-written `lateinit` property throws the same checked error ADR 0022 § 3's
  verification already covers for `Core\Reflect`-constructed objects; writing it first and reading after
  succeeds; a second, later write succeeds with no write-once restriction.
