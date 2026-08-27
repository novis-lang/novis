# ADR 0028 — Closing the remaining magic methods: `Stringable` replaces `__toString`; no `__destruct`, `__debugInfo`, or `__set_state`; `unset()` is refused on an object property

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** every PHP object magic method no prior ADR has ruled on — `__toString`, `__destruct`,
  `__isset`/`__unset`, `__debugInfo`, `__set_state` — plus a closing note on `__autoload`. Also serves as
  the single index of every magic method's disposition across the project, since [ADR 0014](0014-property-observer.md)
  first opened the "magic methods" line item and three more ADRs have each closed a piece of it since.
- **Amends:** [0007](0007-explicit-type-system.md) § 4 — the conversion table's "anything → `string`" row
  said "an object needs `__toString`, or it throws"; it now names `Stringable` instead. Also
  [implementation-plan.md](../implementation-plan.md) M4 — the debug-dump line item
  gains this ADR's citation for what a dump no longer does.
- **Amended by:** 0030, 0033, 0053, 0061, 0092

> **In short:** `__toString` is replaced by a declared global `Stringable` interface
> (`public function toString(): string`), the same treatment [0013](0013-comparable-interface.md) gave
> ordering and [0014](0014-property-observer.md) gave property access. **`__destruct` does not exist in any
> form** — MWL has no destructors, period; object cleanup is always an explicit method call. `__isset`/
> `__unset` need no replacement: [0014](0014-property-observer.md) already closed the ambient fallback they
> depend on, so this ADR only has to say what `isset`/`unset` now mean on an object property — `isset`
> reduces to the ordinary "not null" check every other binding already has, and **`unset()` on a declared
> object property is a compile-time diagnostic**, full stop, because [0022](0022-definite-property-initialization.md)
> already guarantees no declared property is ever anything other than definitely initialized. `__debugInfo`
> is removed with no replacement — `Core\Debug::dump` always shows a class's real declared properties and
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
| `__autoload` | moot — removed by PHP itself, and by static resolution | **this ADR, § 6**; what replaces `spl_autoload_register` is [0061](0061-compile-time-autoload-and-program-discovery.md) |

- Every closed row above shares one argument already made in full by
  [0011](0011-functions-and-constants-are-class-members.md), [0012](0012-no-superglobals.md),
  [0013](0013-comparable-interface.md), [0014](0014-property-observer.md) and
  [0027](0027-callable-is-closures-only.md): a behavior triggered by a reserved method name is invisible
  from the class declaration and unreadable by a checker or IDE without reimplementing PHP's dispatch rules.
  This ADR applies that same reasoning to the five remaining cases, plus one reason specific to each — most
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

**MWL has no destructors.** A method named `__destruct` cannot even be declared:
[ADR 0029](0029-identifier-casing-is-checked.md)'s method-casing rule has never allowed a leading
underscore, so the casing checker refuses the name outright before anything about destructors comes into
play — the same fate [ADR 0014](0014-property-observer.md) § 6 gives `__call` (that section was itself
corrected to say so; earlier drafts of both described the name as "compiling as an ordinary method the
runtime never invokes," which stopped being accurate once the casing checker existed to reject it first).
Either way there is no refcount-triggered cleanup hook, no scope-exit hook, nothing. Two independent
arguments for why destructors themselves have no place here, either one sufficient on its own:

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
  != null`. For a non-nullable typed property this is always `true` — such a property can never hold
  `null` — and for a nullable one it reflects whatever was last assigned. No magic method is consulted;
  accessing an undeclared `$obj->prop` is already [0014](0014-property-observer.md)'s hard error before
  `isset` even gets involved.
- **`unset($obj->prop)`**, and `unset(Class::$prop)` with it — **a compile-time diagnostic**
  (`E0413`), for every declared property, static or instance, regardless of
  nullability. PHP's `unset()` removes the property outright, leaving later access to fall through to
  `__get`/trigger a notice — an "uninitialized again" state that [0022](0022-definite-property-initialization.md)
  exists specifically to make impossible for a declared property. There is no way to honor both ADRs at
  once, and [0022](0022-definite-property-initialization.md) was Accepted first. A script that wants a
  nullable property back to its empty state writes `$obj->prop = null;` — an ordinary assignment, not a
  structural removal — which already does everything PHP's `unset()` was being used for in that case.

Which leaves **exactly one thing `unset()` does**: remove an entry from an array, `unset($holder[key])`,
where the holder is a local, a property or a static property. PHP's existing rules for that spelling keep
working exactly as they do today, including at depth (`unset($grid[$r][$c])`) — with one difference that is
[0007](0007-explicit-type-system.md) § 7 row 11 rather than this section's, since an intermediate level is
an ordinary element read: an absent one throws where PHP is silent, and a removal never vivifies the row it
is removing from.

Every **other** operand is `E0234`, and the two shapes worth naming are the two PHP programs actually
write. `unset($local)` is refused because [0007](0007-explicit-type-system.md) § 1 declares every binding
once, with a type, definitely assigned: there is no "undefined again" state to put a name back into, so the
construct has nothing left to mean — assign `null` where the declared type is nullable, or let the binding
go out of scope. `unset(rows()[$k])` is refused because § 5's copy-on-write separates the array before the
entry goes and the separated copy needs a slot to be written back into, which is the same thing PHP means
by refusing a temporary in a write context. Narrowing the operand this far is what leaves `mwl_ir`'s
lowering one shape to lower and no shape to panic on.

### 4. `__debugInfo` — rejected, no replacement

`Core\Debug::dump` ([0092](0092-one-diagnostic-record-three-renderings.md), which owns its surface and its
three renderings) always shows a class's real declared properties and their real current values,
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
[0023](0023-clone-serialize-and-cross-boundary-copy.md) already closed for exactly this reason. A class declaring `__set_state` never reaches the question of whether the runtime would invoke it: the name
itself is refused by [ADR 0029](0029-identifier-casing-is-checked.md)'s method-casing rule before any
resolution logic runs, same as every other double-underscore magic method this document and
[ADR 0014](0014-property-observer.md) § 6 cover.

### 6. `__autoload` — moot, closing note only

PHP itself removed `__autoload()` in 8.0 (replaced by `spl_autoload_register()`, itself not a magic method).
It is noted here only so a future reader does not wonder why it is missing from the table above: MWL
resolves every class/interface/enum reference statically at compile time
([implementation-plan.md](../implementation-plan.md) M2, `mwl-hir`'s `SymbolTable`), so there is no runtime
moment at which an unresolved class name could trigger a loader callback in the first place. This is a
consequence of the static-resolution architecture already being built, not a new decision.

What this section does *not* answer is what replaces the mechanism PHP actually uses,
`spl_autoload_register()` — how a name reaches its file at all without a hand-written `require` for every
declaration. That is a decision rather than a consequence, and it is
[ADR 0061](0061-compile-time-autoload-and-program-discovery.md)'s.

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
- `Core\Debug::dump` becomes exactly as predictable as every other closed mechanism in this project: what
  you declare is what a dump shows, never what a class chose to substitute.

**Negative**

- **A structural break from PHP**, one of the divergences [divergences.md](divergences.md) registers: PHP source relying on `__toString` (mechanical rename to `Stringable`/`toString()`
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

- **A declared `Disposable`/`Closeable` interface with scope-exit invocation.** No scope-exit-triggered call
  exists anywhere else in MWL; see *Revisiting*.
- **Keep `__destruct`, forbid throwing, skip it at request-heap teardown.** Still requires walking every live
  object whose refcount reaches zero mid-request — the per-object cost [0004](0004-memory-for-simplicity.md)'s
  wholesale drop avoids.
- **Keep ambient `__isset`/`__unset` for parity.** Nothing left to intercept once [0014](0014-property-observer.md)
  closed `__get`/`__set`'s fallback.
- **Let `unset()` reset a non-nullable property to a type default.** [0022](0022-definite-property-initialization.md)
  already rejected per-type silent defaults for exactly this reason.
- **A `DebugRepresentable`-style interface for the dump.** No unsafe default to close off — would add a
  customization surface with no motivating problem.
  [0092](0092-one-diagnostic-record-three-renderings.md) § 7 reaffirms this rather than reopening it: its
  record model is closed, which is this refusal expressed as a data type.
- **Keep PHP's open `var_export()`/`__set_state` format for `mwl convert`.** Same
  generated-code-as-data-format risk [0023](0023-clone-serialize-and-cross-boundary-copy.md) already closed
  for serialize.

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
  named `__destruct`, `__isset`, `__unset`, `__debugInfo`, or `__set_state` never reaches any special
  resolution path in the first place: [ADR 0029](0029-identifier-casing-is-checked.md)'s method-casing rule
  refuses the leading-underscore name itself, the same diagnostic any other double-underscore method name
  gets. `unset($obj->prop)` is refused for every declared property with a
  diagnostic naming [0022](0022-definite-property-initialization.md)'s guarantee as the reason.
- **M4**: a class implementing `Stringable` is accepted at every implicit-conversion site in § 1 and
  produces the value `toString()` returns; no method named `__destruct` could compile in the first place,
  so none is ever invoked when refcounts legitimately reach zero mid-request; `isset($obj->prop)` matches
  `$obj->prop != null` for a nullable property and is always `true` for a non-nullable one;
  `Core\Debug::dump` shows every declared property and its live value, with no `__debugInfo`/`__set_state`
  method able to exist to affect that.
- **M11**: the converter mechanically rewrites `__toString` to `Stringable`/`toString()` and
  `unset($obj->nullableProp)` to `$obj->nullableProp = null;`; it flags PHP source using `__destruct` or
  `__set_state` as a `TODO` needing a human decision, per *Consequences*' negative list.
