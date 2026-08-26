# ADR 0022 — Properties are definitely initialized at compile time; no observable uninitialized state

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** what a declared, non-nullable property holds before anything has assigned it — the
  compile-time obligation on every constructor to definitely assign every property the class declares (own
  or inherited), the one residual case static analysis cannot cover (an object built through `Core\Reflect`
  without running a constructor), and the explicit rejection of a new `undefined` value or type to model the
  gap.
- **Amends:** [ADR 0007](0007-explicit-type-system.md) § 1 — "definite assignment is checked" was written
  for local variables only; this ADR names properties as the second binding kind the same analysis covers,
  and is the decision that paragraph's scope was silently missing.
- **Amended by:** 0038, 0043

> **In short:** PHP's typed properties can exist in a third state, neither assigned nor `null`, and reading
> one throws — a correct but purely runtime-discovered failure that surfaces far from the missing
> assignment that caused it. MWL keeps the throw as a last resort but removes the reason it is usually
> needed: **every property a class declares must be definitely assigned along every path out of every
> constructor that class exposes**, checked by the same flow analysis [ADR 0007](0007-explicit-type-system.md)
> already commits to for local variables, extended here to a second binding kind. A class with no
> constructor and a non-nullable property with no inline default is refused at the property declaration,
> before any object is ever built. The only state static analysis cannot rule out is an object constructed
> through `Core\Reflect` with no constructor run at all; reading such a property before its first write is
> a checked runtime throw — the same "hard error, never a silent value" shape every other "accessed a thing
> that is not there" case in this project already takes. There is **no new `undefined` type or value**:
> adding one would make every declared property implicitly `T|undefined`, which is exactly the "a binding's
> declared type silently holds something else" failure ADR 0007 exists to close, in a new shape. A property
> that may legitimately hold no value already has a spelling — `?T` — and this ADR does not touch it.

## Context

- PHP 7.4 typed properties introduced a state distinct from `null` — declared but unassigned; reading one
  throws only when discovered at the read, often far from the constructor that forgot to set it.
- **JavaScript's `undefined` was proposed and rejected**: JS has no declared property types to violate, while
  MWL's typed properties are exactly the guarantee [ADR 0007](0007-explicit-type-system.md) built to prevent
  a declared type silently holding something else; a universal `undefined` would reintroduce that failure one
  binding kind later — the same ambient-magic shape already closed for undeclared properties/`__get`/`__set`
  ([ADR 0014](0014-property-observer.md)) and superglobals ([ADR 0012](0012-no-superglobals.md)).
- Other statically-typed languages split between compile-time-only (Rust/Swift), an opt-in throw-on-early-
  read modifier (Kotlin's `lateinit`), and a silent per-type default (C#/Java) — the last rejected here for
  the same reason ADR 0007 rejects silent coercion.
- MWL already has the mechanism: ADR 0007 §1 commits to definite-assignment checking for locals. A property
  is a second, structurally similar binding kind, extended with `parent::constructor(...)` as what discharges
  inherited properties — mirroring Java/Kotlin's mandatory `super()`.

## Decision

**No new type or value. Every property a class declares must be definitely assigned along every path out
of every constructor that class exposes, checked at compile time by the same flow analysis ADR 0007 already
commits to for locals. The one case that analysis cannot reach — an object built through `Core\Reflect`
without running a constructor — throws a checked, catchable error on first read, exactly once, and never
produces a value.**

### 1. `?T` already covers "may legitimately hold no value" — this ADR does not add a second way

Nullability is a fact about a property's *value space*, decided once at its declaration. Whether it has
been *assigned yet* is a completely different axis, and conflating the two is what leads to `undefined`
being tempting in the first place. A property meant to be legitimately absent is `?T`, with an explicit
initializer if it should start `null`; a property with no `?` is a promise that every read sees a real `T`,
and that promise is what the rest of this ADR makes the compiler keep.

### 2. Definite assignment, extended from locals to properties

For every constructor a class declares (including the implicit default constructor a class with none is
given):

- Every property declared **by that class itself** — a plain field or a promoted constructor parameter,
  including one used as a [ADR 0043](0043-interface-default-methods-and-delegation-replace-traits.md)
  `by`-delegation target, which is an ordinary property with no special case of its own — must be assigned on
  every path from the constructor's entry to every one of its returns, before this ADR's check passes. A
  promoted parameter (`public int $x` in the parameter list) satisfies its own obligation by construction —
  binding the parameter *is* the assignment. An inline default (`public int $x = 0;`) satisfies it before the
  constructor body runs at all.
- A subclass constructor discharges the properties **it inherits** by calling `parent::constructor(...)` on
  every path. The analysis trusts that call rather than re-deriving it: the parent class's own constructors
  were already checked against this same rule when the parent was compiled, exactly as a function call's
  callee is trusted rather than re-verified at every call site elsewhere in the checker.
- A class that declares **no constructor of its own** and has a non-nullable property with no inline
  default is refused at the property's declaration — there is no constructor body to attach the diagnostic
  to, so it names the property directly and suggests either fix: a default, or a constructor that sets it.
- A property backed by a `set` hook ([ADR 0014](0014-property-observer.md)) discharges its obligation the
  moment the hook itself commits a value during construction — the hook's storage is the property's
  storage for this purpose. A **virtual** property with a `get` hook and no backing field has nothing to
  initialize and is outside this rule entirely, the same way it is outside plain field access.

This is refused with a diagnostic at the same point and the same M2 timing as the read-before-definite-
assignment error ADR 0007 already plans for locals — one analysis pass, two binding kinds.

### 3. The residual runtime case: `Core\Reflect` bypassing every constructor

[ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md) makes object construction without running
a constructor a first-class, reachable operation, so it is the one place *2*'s compile-time guarantee
cannot reach. A property on such an object that has never been written and is then read:

- throws a checked error — an ordinary `Throwable`, propagated exactly like any other checked failure
  ([ADR 0002](0002-error-propagation.md)) — **not** routed through
  [ADR 0020](0020-error-escalation-ladder.md)'s fatal ladder, because this is a recoverable, catchable
  condition a caller can reasonably handle, not a resource-limit or internal-panic class of failure;
- never hands back a value standing in for "not yet set" — there is nothing a caller can inspect, compare
  against, or receive from a read that succeeds this way, because the read never succeeds;
- is unaffected by writing first: a write to an unset property on such an object works exactly like a write
  to any other, and every later read sees it.

Internally, a property's storage needs one extra, non-user-observable state — "never written" — distinct
from every legal value including `null`. This is **not** a new entry in the type system, the checker, or
anything an expression can produce: it is a transient storage state that either gets overwritten by the
first write (the overwhelmingly common case, guaranteed by *2* for every ordinarily-constructed object) or
is caught and turned into the throw in *3* before it is ever handed to user code. [ADR 0004](0004-memory-for-simplicity.md)
requires stating the cost: the same tagged-value representation that gives `uint` a free tag in ADR 0007 §4
gives this marker a free tag too — one more discriminant on the existing value representation, **zero
additional bytes per property**.

## Consequences

**Positive**

- The PHP failure mode the user opened this decision wanting to avoid — a surprising runtime throw far from
  the missing assignment — becomes, for the overwhelming majority of real programs, a compile-time refusal
  at the constructor that forgot the assignment, or at the property declaration if there is no constructor
  at all. The runtime throw in *3* survives only for the narrow, explicit case of reflection-driven
  construction, not as the default experience of using typed properties.
- No new type, no implicit `T|undefined` widening of every declared property — ADR 0007's "a declared type
  never silently holds something else" stays intact rather than gaining an exception on its first
  anniversary.
- One definite-assignment analysis covers two binding kinds (locals, properties) instead of two separate
  mechanisms that would have to be kept in agreement.
- Consistent with every other "hard error, never a silent default" precedent already in this project:
  undeclared property access and no `__get`/`__set` fallback ([ADR 0014](0014-property-observer.md)),
  undeclared locals and rejected `settype` ([ADR 0007](0007-explicit-type-system.md)).

**Negative**

- `Core\Reflect`-constructed objects keep a genuine, if narrow, runtime-only failure mode — unlike Rust's
  fully static guarantee, MWL cannot extend the compile-time promise across a boundary that exists
  specifically to bypass constructors. This is a smaller surface than PHP's (which allows the runtime state
  from *any* construction path, not just a reflective one), but it is not zero.
- Extending the analysis through constructor chains is more work for M2's checker than a locals-only
  definite-assignment pass would have been — real complexity cost against priority 4, though it reuses one
  mechanism rather than adding a second.
- Stricter than PHP at compile time: PHP happily compiles a constructor that leaves a typed property unset
  on some path, and only fails when that path is actually read. Porting PHP source with such a gap needs a
  fix, not just a recompile — `mwl convert` (M11) must flag it, joining the TODO classes ADR 0007 §7 and
  ADR 0014 already grow.
- One more internal discriminant on the tagged-value representation, though at zero additional bytes per
  [ADR 0004](0004-memory-for-simplicity.md)'s accounting — see *3*.

## Alternatives rejected

- **A new `undefined` type or value**, modeled on JavaScript. Rejected: JavaScript's `undefined` is safe
  only absent a declared-type guarantee to violate; MWL has exactly that guarantee, so this recurs ADR
  0007's failure shape one binding kind later, and is the same ambient-default shape ADR 0012/0014 already
  rejected.
- **Per-type silent defaults** (C#/Java). Rejected for the same reason ADR 0007 rejects silent coercion: a
  plausible-looking wrong value is worse than a loud one, since "forgot to initialize" would look identical
  to "legitimately zero."
- **PHP's status quo — runtime-only throw, no compile-time check.** Rejected: forgoes both correctness
  caught at its cause and the "one analysis, not two" simplicity gain available for free once ADR 0007's
  mechanism already exists.
- **An opt-in `lateinit`-style modifier** (Kotlin), reachable from ordinary code rather than only reflection.
  Not rejected outright — deferred at the time this ADR was written, since the compile-time-only, no-opt-out
  decision made here was the smaller commitment while no implementation existed yet to make backward-
  incompatible. Added by [ADR 0038](0038-lateinit-property-modifier.md).

## Revisiting

Deferred deliberately, each needing its own argument once there is real code to argue from:

- **Whether `Core\Reflect`'s constructor-bypassing instantiation should require every non-nullable
  property's value up front**, closing the residual runtime case in *3* entirely rather than leaving it to
  a first-read throw. That is a decision for `Core\Reflect`'s own API surface
  ([ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md)), not this one.
- **The exact propagation of *2* through abstract classes with no constructor of their own, multi-level
  inheritance, and interfaces** (which declare no storage at all) belongs to `docs/spec/`, unwritten as of
  this ADR — this document fixes the top-level rule only, the same deferral
  [ADR 0014](0014-property-observer.md) already uses for per-property hook internals.

Verification, in the order it becomes possible:

- **M2**: the checker refuses a constructor path that can return without every own-declared, non-nullable
  property assigned, joining the diagnostic corpus ADR 0007's own M2 entry already builds — a missing
  assignment on one branch of an `if`/`else`, a subclass constructor with a path that never calls
  `parent::constructor(...)`, a class with no constructor and a non-nullable property with no default. A
  promoted parameter and an inline default both compile with no diagnostic.
- **M4**: reading a property on a `Core\Reflect`-constructed instance that was never written throws the
  checked error described in *3*; writing first and then reading succeeds normally; a class implementing
  `PropertyObserver` ([ADR 0014](0014-property-observer.md)) still runs its pipeline correctly once a value
  has actually been committed, hooked or not.
- **M11**: the converter flags PHP source whose constructor leaves a typed property unset on some path as
  needing a fix (not merely a recompile), per this ADR's negative consequence above.
