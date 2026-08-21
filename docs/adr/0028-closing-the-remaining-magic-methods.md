# ADR 0028 — Closing the remaining magic methods: `Stringable` replaces `__toString`; no `__destruct`, `__debugInfo`, or `__set_state`; `unset()` is refused on an object property

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** every PHP object magic method no prior ADR has ruled on — `__toString`, `__destruct`,
  `__isset`/`__unset`, `__debugInfo`, `__set_state` — plus a closing note on `__autoload`. Also serves as
  the single index of every magic method's disposition across the project, since [ADR 0014](0014-property-observer.md)
  first opened the "magic methods" line item and three more ADRs have each closed a piece of it since.
- **Amends:** [0007](0007-explicit-type-system.md) § 4 — the conversion table's "anything → `string`" row
  said "an object needs `__toString`, or it throws"; it now names `Stringable` instead. Also
  [implementation-plan.md](../implementation-plan.md) M4 — the `var_dump`/`print_r`/`json_encode` line item
  gains this ADR's citation for what those two no longer do.
- **Amended by:** [0030](0030-no-leading-underscores-constructor-spelling.md) — adds the `__construct`
  disposition this ADR's own magic-method table never carried: kept, not closed, but respelled
  `constructor`.
  [0033](0033-secret-qualifier-for-confidential-values.md) — § 4's "`var_dump()`/`print_r()` always show a
  class's real declared properties and their real current values" guarantee gets one carve-out: a property
  whose declared type carries `secret` shows a fixed redaction placeholder instead. This is a built-in,
  type-keyed rule the dump implementation applies uniformly, not the per-class `DebugRepresentable`-style hook
  this section already rejected — that distinction still holds.
- **Relates to:** [0002](0002-error-propagation.md) (why a destructor has no sound place to report a thrown
  status — part of § 2's reasoning below), [0004](0004-memory-for-simplicity.md) (the request heap is
  dropped wholesale rather than walked object-by-object, which a `__destruct` guarantee would undo),
  [0006](0006-isolated-script-execution.md) (an isolate already tears down without per-object cleanup;
  nothing here changes that), [0011](0011-functions-and-constants-are-class-members.md) (why `Stringable`
  is a global interface, not a `Core` domain class), [0013](0013-comparable-interface.md) and
  [0014](0014-property-observer.md) (the declared-interface-over-ambient-name pattern this ADR applies to
  `__toString`, and the precedent — `PropertyObserver`'s "no `__get`/`__set` fallback" — that `__isset`/
  `__unset` fall out of directly), [0019](0019-reflection-and-ast-parsing-are-core-features.md) (why
  removing `__debugInfo` does not reopen that ADR's visibility-checked value access — they are different
  mechanisms), [0022](0022-definite-property-initialization.md) (why `unset()` on an object property is
  refused), [0023](0023-clone-serialize-and-cross-boundary-copy.md) (its closed serialize format supersedes
  `__set_state`'s use case), [0027](0027-callable-is-closures-only.md) (the most recent instance of the same
  ambient-name closure this ADR continues)

> **In short:** `__toString` is replaced by a declared global `Stringable` interface
> (`public function toString(): string`), the same treatment [0013](0013-comparable-interface.md) gave
> ordering and [0014](0014-property-observer.md) gave property access. **`__destruct` does not exist in any
> form** — MWL has no destructors, period; object cleanup is always an explicit method call. `__isset`/
> `__unset` need no replacement: [0014](0014-property-observer.md) already closed the ambient fallback they
> depend on, so this ADR only has to say what `isset`/`unset` now mean on an object property — `isset`
> reduces to the ordinary "not null" check every other binding already has, and **`unset()` on a declared
> object property is a compile-time diagnostic**, full stop, because [0022](0022-definite-property-initialization.md)
> already guarantees no declared property is ever anything other than definitely initialized. `__debugInfo`
> is removed with no replacement — `var_dump`/`print_r` always show a class's real declared properties and
> their real current values. `__set_state` is removed with no replacement — reconstructing an object from
> external data is already [0023](0023-clone-serialize-and-cross-boundary-copy.md)'s job. `__autoload` gets
> no decision at all: PHP itself removed it in 8.0, and MWL resolves every class reference statically at
> compile time, so there is no runtime moment left for it to attach to.

## Context

PHP declares seventeen-ish magic methods; four of the ADR-worthy ones are already closed:

| magic method(s) | disposition | decided in |
|---|---|---|
| `__get` / `__set` | replaced by declared `PropertyObserver` | [0014](0014-property-observer.md) |
| `__call` / `__callStatic` | rejected outright, no replacement | [0014](0014-property-observer.md) |
| `__clone` | kept, PHP-shallow, no hook | [0023](0023-clone-serialize-and-cross-boundary-copy.md) |
| `__serialize` / `__unserialize` / `__sleep` / `__wakeup` | rejected; closed graph-copy `serialize()`/`unserialize()` instead | [0023](0023-clone-serialize-and-cross-boundary-copy.md) |
| `__invoke` | rejected outright, no replacement | [0027](0027-callable-is-closures-only.md) |
| `__toString` | replaced by declared `Stringable` | **this ADR, § 1** |
| `__destruct` | rejected outright, no replacement | **this ADR, § 2** |
| `__isset` / `__unset` | no replacement needed; `unset()` on an object property is refused | **this ADR, § 3** |
| `__debugInfo` | rejected outright, no replacement | **this ADR, § 4** |
| `__set_state` | rejected outright, no replacement | **this ADR, § 5** |
| `__autoload` | moot — removed by PHP itself, and by static resolution | **this ADR, § 6** |

- Every closed row above shares one argument, made in full in
  [0011](0011-functions-and-constants-are-class-members.md), [0012](0012-no-superglobals.md),
  [0013](0013-comparable-interface.md) and repeated at [0014](0014-property-observer.md)'s and
  [0027](0027-callable-is-closures-only.md)'s own Context sections: a behavior triggered by a reserved
  method name is invisible from the class declaration, unreadable by a checker or IDE without
  reimplementing PHP's dispatch rules, and often the exact shape of PHP's worst security history
  (`__wakeup`/`__unserialize` object injection, closed in
  [0023](0023-clone-serialize-and-cross-boundary-copy.md)).
- This ADR applies that reasoning to the five remaining cases, plus one reason specific to each — most
  notably `__destruct`, which fails for a second, independent reason (§ 2).

## Decision

### 1. `__toString` → declared `Stringable`, same shape as `Comparable`/`PropertyObserver`

```php
interface Stringable {
    public function toString(): string;
}
```

`Stringable` lives in the global namespace, not under `Core` — [0011](0011-functions-and-constants-are-class-members.md)
reserves `Core` for domain classes holding `static` members, and `Stringable` is a contract an ordinary
class implements, exactly the shape [0013](0013-comparable-interface.md) already anticipated when it noted
`Comparable` "lives in the global namespace, the same place PHP's built-in `Stringable` does."

Every position that implicitly converts a value to `string` — string interpolation (`"$obj"`,
`"{$obj}"`), concatenation (`$obj . ''`), `echo`/`print`, and the `as string` conversion
([0007](0007-explicit-type-system.md) § 4) — accepts an object only when its static type provably
implements `Stringable`, and calls `toString()` to produce the value. An object whose class does not
implement `Stringable` used in any of those positions is a **compile-time diagnostic** naming `Stringable`
as the fix, the same certainty [0013](0013-comparable-interface.md) gives an unorderable pair and
[0014](0014-property-observer.md) gives an undeclared property — there is no runtime fallback that
stringifies an object some other way (PHP's own fallback here is already just a fatal error, so nothing
permissive is being removed).

The method is named `toString()`, not `__toString()`: the same reasoning [0014](0014-property-observer.md)
gave `PropertyObserver`'s methods applies verbatim — a name that still read `__toString` after the ambient
trigger is gone would misdescribe what is actually happening at the declaration site (a caller sees
`implements Stringable`, not a magic name, to know the capability exists).

[0007](0007-explicit-type-system.md) § 4's conversion table row is updated by this ADR:

| conversion | behaviour |
|---|---|
| anything → `string` | total for scalars; an object needs `Stringable`, or it's a compile-time diagnostic |

### 2. `__destruct` — rejected outright, no replacement of any kind

**MWL has no destructors.** A method named `__destruct` compiles as an ordinary method the runtime never
invokes implicitly, exactly the treatment [0014](0014-property-observer.md) gives a method named `__call`.
There is no refcount-triggered cleanup hook, no scope-exit hook, nothing. Two independent arguments, either
one sufficient on its own:

- **No sound place to report a throw.** [0002](0002-error-propagation.md) is normative: every call returns
  a checked status, and nothing unwinds through a JIT frame. An explicit call site has an obvious place to
  receive that status — the caller. A destructor has no call site at all; it fires from wherever a refcount
  happens to hit zero, which can be an assignment, a `foreach` iteration, an array element being
  overwritten, or a function returning — none of which is a natural place to surface a new checked failure
  that has nothing to do with the operation actually being performed there. PHP's own answer to this
  ("Fatal error: Uncaught \[Exception\] in destructor") is not a design, it is the absence of one — the
  language noticing at the last possible moment that it has no good place to put the answer.
- **It would undo [0004](0004-memory-for-simplicity.md)'s wholesale heap drop.** "Shared-nothing requests,
  heap dropped wholesale" is in that ADR's own table specifically *because* walking live objects
  individually at request end costs time proportional to what a request allocated, and the only two
  destructor semantics an implementation could offer are both bad: guarantee every live object's
  `__destruct` runs at teardown (reintroducing the individual walk ADR 0004 exists to avoid, on every single
  request, not just ones that leak) or reproduce PHP's own well-known inconsistency (a destructor runs
  reliably when a refcount hits zero mid-request, but is simply skipped for anything still reachable — most
  commonly a reference cycle — when the process reclaims it in bulk). Removing destructors entirely is the
  one option that does not have to pick between those two, and it costs nothing extra: the request heap was
  always going to be dropped wholesale regardless of what runs first.

Object cleanup that PHP code currently puts in `__destruct` — closing a file handle, releasing a lock,
flushing a buffer — becomes an explicit method (`$conn->close()`, a project's own name), called by whoever
holds the reference when they are actually done with it. The underlying host `resource` a `resource`-typed
value wraps is still reclaimed correctly by the runtime when nothing references it — that is memory
management, an implementation detail this ADR does not touch — but no *user-level* code runs at that moment,
because there is no destructor left to run it.

### 3. `__isset`/`__unset` need no replacement — and `unset()` on an object property is refused

`__isset($name)`/`__unset($name)` exist in PHP purely to intercept `isset()`/`empty()`/`unset()` for a
property that is undefined or inaccessible — the identical ambient-fallback shape
[0014](0014-property-observer.md) already closed for `__get`/`__set`. With that fallback gone, there is
nothing left for a magic pair to intercept, and neither language construct needs a declared-interface
replacement — but this ADR still has to say what each construct means on an object now that
[0022](0022-definite-property-initialization.md) guarantees every declared property is definitely
initialized, permanently, after construction:

- **`isset($obj->prop)`** — unchanged from what `isset` already means for every other binding: `$obj->prop
  !== null`. For a non-nullable typed property this is always `true` — such a property can never hold
  `null` — and for a nullable one it reflects whatever was last assigned. No magic method is consulted;
  accessing an undeclared `$obj->prop` is already [0014](0014-property-observer.md)'s hard error before
  `isset` even gets involved.
- **`unset($obj->prop)`** — **a compile-time diagnostic**, for every declared property regardless of
  nullability. PHP's `unset()` removes the property outright, leaving later access to fall through to
  `__get`/trigger a notice — an "uninitialized again" state that [0022](0022-definite-property-initialization.md)
  exists specifically to make impossible for a declared property. There is no way to honor both ADRs at
  once, and [0022](0022-definite-property-initialization.md) was Accepted first. A script that wants a
  nullable property back to its empty state writes `$obj->prop = null;` — an ordinary assignment, not a
  structural removal — which already does everything PHP's `unset()` was being used for in that case.

This section is scoped to **object properties only**. `unset()` on an array element or a local variable is
untouched — PHP's existing rules for those keep working exactly as they do today.

### 4. `__debugInfo` — rejected, no replacement

`var_dump()`/`print_r()` always show a class's real declared properties and their real current values,
annotated with each property's declared visibility exactly as PHP's own output already is — there is no
hook to filter, rename, or synthesize fields for the dump. This is unrelated to, and does not reopen,
[0019](0019-reflection-and-ast-parsing-are-core-features.md) § 2's rule that a `Core\Reflect` *value* read
enforces the same visibility check an ordinary access would: a debug dump is a fixed, built-in operation
a developer did not write and cannot be called with attacker-influenced arguments to bypass anything, not a
`Core\Reflect` call site a script constructs — the two are different mechanisms with different threat
models, and this ADR does not ask [0019](0019-reflection-and-ast-parsing-are-core-features.md) to change.

### 5. `__set_state` — rejected, no replacement

`var_export()`'s object-reconstruction-from-generated-code use case is already answered by
[0023](0023-clone-serialize-and-cross-boundary-copy.md)'s closed `serialize()`/`unserialize()` round trip,
which gives the same "get an object's data out, get an equivalent object back" capability without emitting
PHP source that has to be `eval`'d or compiled to reconstruct it — a category of trust problem
[0023](0023-clone-serialize-and-cross-boundary-copy.md) already closed for exactly this reason. A class
declaring `__set_state` compiles as an ordinary method the runtime never invokes implicitly, same as every
other rejected magic method in this document.

### 6. `__autoload` — moot, closing note only

PHP itself removed `__autoload()` in 8.0 (replaced by `spl_autoload_register()`, itself not a magic method).
It is noted here only so a future reader does not wonder why it is missing from the table above: MWL
resolves every class/interface/trait/enum reference statically at compile time
([implementation-plan.md](../implementation-plan.md) M2, `mwl-hir`'s `SymbolTable`), so there is no runtime
moment at which an unresolved class name could trigger a loader callback in the first place. This is a
consequence of the static-resolution architecture already being built, not a new decision.

## Consequences

**Positive**

- Closes the "magic methods" line item [0014](0014-property-observer.md) opened, across every remaining
  case — a future contributor asking "what does MWL do with `__x`" now has exactly one table to check.
- `Stringable` costs nothing beyond an ordinary virtual call, already priced the same way
  [0013](0013-comparable-interface.md) and [0014](0014-property-observer.md) priced their own interfaces
  ([0004](0004-memory-for-simplicity.md)).
- Removing destructors removes an entire class of PHP footguns in one motion: resurrection (storing `$this`
  somewhere during `__destruct`), the "exception in destructor" fatal, and shutdown-order dependence between
  two objects' destructors — none of these can exist in a language with no destructors.
- `unset()`'s refusal on a declared property closes a real, previously-unnoticed gap between
  [0022](0022-definite-property-initialization.md)'s guarantee and PHP's own `unset()` semantics, before any
  code could have depended on the contradiction.
- `var_dump`/`print_r` become exactly as predictable as every other closed mechanism in this project: what
  you declare is what a dump shows, never what a class chose to substitute.

**Negative**

- **A structural break from PHP**, joining the divergence list [0007](0007-explicit-type-system.md) § 7
  already carries forward: PHP source relying on `__toString` (mechanical rename to `Stringable`/`toString()`
  for `mwl convert`, [M11](../implementation-plan.md)), `__destruct` (needs a human decision — an explicit
  cleanup method, called from every place the original relied on implicit timing), `__isset`/`__unset`
  (mechanical removal, since [0014](0014-property-observer.md) already made the fallback they served
  unreachable), `__debugInfo` (mechanical removal), or `__set_state` (needs a human decision — points at
  `serialize()`/`unserialize()` instead) does not convert unconverted.
- **No RAII-style automatic cleanup exists at all.** A class wrapping a resource that must be released
  promptly (not just eventually, at request end) now requires every caller to remember to call an explicit
  method — PHP's `__destruct` was a genuine, if unreliable, safety net for the case where a caller forgets.
  Accepted per § 2's argument that the net was never reliable to begin with once cycles or process shutdown
  were involved.
- `unset($obj->nullableProp)` no longer works as an idiom for "reset the property"; `mwl convert` rewrites it
  to `$obj->nullableProp = null;` mechanically, but a non-nullable property has no equivalent at all —
  correctly, since [0022](0022-definite-property-initialization.md) never allowed such a property to be
  anything but initialized.

## Alternatives rejected

- **A declared `Disposable`/`Closeable` interface with automatic invocation at scope exit.** A deterministic
  middle ground for § 2, but scope-exit-triggered calls don't exist anywhere else in MWL — a new construct
  to justify a magic method this ADR is otherwise just deleting; see *Revisiting*.
- **Keep `__destruct`, forbid throwing, skip it at request-heap teardown.** PHP's behavior in stricter
  clothing — still requires walking every live object whose refcount reaches zero mid-request, the exact
  per-object cost [0004](0004-memory-for-simplicity.md)'s wholesale drop avoids paying.
- **Keep ambient `__isset`/`__unset` for parity.** They exist only to serve `__get`/`__set`'s fallback,
  already closed by [0014](0014-property-observer.md) — nothing left to intercept.
- **Let `unset()` reset a non-nullable property to a type default.** [0022](0022-definite-property-initialization.md)
  already rejected per-type silent defaults for exactly this reason.
- **A `DebugRepresentable`-style interface for `var_dump()`.** Unlike `Stringable`/`Comparable`, there's no
  unsafe default to close off — PHP's own dump with no `__debugInfo` is already safe, so this would add a
  customization surface with no motivating problem.
- **Keep PHP's open `var_export()`/`__set_state` format for `mwl convert`.** Same reason
  [0023](0023-clone-serialize-and-cross-boundary-copy.md) rejected PHP's open serialize format:
  generated-code-as-data-format is the "well-shaped payload constructs arbitrary objects" surface that ADR
  already closed.

## Revisiting

- **A `Disposable`/scope-exit-cleanup construct**, if `mwl convert`'s M11 pass over real PHP `__destruct`
  usage shows a pattern common enough that "call an explicit method" is a systematic burden rather than an
  occasional one. Not decided here — see *Alternatives rejected*.
- **Whether `Stringable` should have a stdlib-wide expectation** (e.g., every exception class implementing
  it) is a stdlib design question for whichever milestone builds exceptions' base class, not this ADR.

Verification, in the order it becomes possible:

- **M2**: the checker refuses string interpolation, concatenation, `echo`, and `as string` on an object
  whose static type does not provably implement `Stringable`, naming `Stringable` as the fix — joining the
  diagnostic corpus [0007](0007-explicit-type-system.md)'s own M2 entry already builds. A method literally
  named `__destruct`, `__isset`, `__unset`, `__debugInfo`, or `__set_state` type-checks as an ordinary method
  with no special resolution path. `unset($obj->prop)` is refused for every declared property with a
  diagnostic naming [0022](0022-definite-property-initialization.md)'s guarantee as the reason.
- **M4**: a class implementing `Stringable` is accepted at every implicit-conversion site in § 1 and
  produces the value `toString()` returns; no method named `__destruct` is ever invoked by the language
  even when refcounts legitimately reach zero mid-request; `isset($obj->prop)` matches `$obj->prop !== null`
  for a nullable property and is always `true` for a non-nullable one; `var_dump`/`print_r` show every
  declared property and its live value with no observable effect from a declared `__debugInfo`/`__set_state`
  method.
- **M11**: the converter mechanically rewrites `__toString` to `Stringable`/`toString()` and
  `unset($obj->nullableProp)` to `$obj->nullableProp = null;`; it flags PHP source using `__destruct` or
  `__set_state` as a `TODO` needing a human decision, per *Consequences*' negative list.
