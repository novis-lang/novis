# ADR 0053 — `Iterable`/`Iterator` are the only iteration interfaces; generators lower to state machines

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** which of SPL's "operator interfaces" exist in MWL; what `foreach` accepts; whether `yield`
  exists and how it is compiled. Not in scope: the `Core\Arr` API, and array iteration itself, which does
  not go through an interface at all.
- **Amends:** [0007](0007-explicit-type-system.md) — the `foreach` key/value binding slot now has a second
  source besides `array<T>`, and § 1's type-variable restriction gains the narrow exception in § 2.
  [0028](0028-closing-the-remaining-magic-methods.md) — it closed the remaining magic *methods* and left
  SPL's magic *interfaces* undecided; § 3 closes `ArrayAccess` and `Countable` on the same reasoning.
- **Amended by:** none.
- **Relates to:** [0014](0014-property-observer.md) (§ 3 applies its rejection of implicit
  property-access dispatch one level over, to subscript dispatch),
  [0011](0011-functions-and-constants-are-class-members.md) (which is why `Countable` has nothing left to
  hook), [0025](0025-wasm-browser-target.md) (§ 4's lowering choice is what keeps its cross-target claim
  intact), [0023](0023-clone-serialize-and-cross-boundary-copy.md) (§ 6, a generator does not cross a
  boundary), [0031](0031-callable-is-the-only-closure-type.md) (a generator is not a closure and has no
  capture clause).

> **In short:** exactly two iteration interfaces exist. `Iterator<T>` is a single-pass cursor —
> `advance(): bool` then `current(): T` — and `Iterable<T>` is a thing that can produce a fresh one via
> `iterate(): Iterator<T>`. `foreach` accepts an `array<T>`, an `Iterable<T>`, or an `Iterator<T>`, and
> nothing else. **`ArrayAccess` and `Countable` do not exist**: the first is the implicit dispatch
> [ADR 0014](0014-property-observer.md) already rejected for properties, and the second has no global
> `count()` left to hook since [ADR 0011](0011-functions-and-constants-are-class-members.md) removed free
> functions. **Generators exist**, spelled `yield`, and are **lowered to an explicit state machine** rather
> than run on the coroutine substrate — so they work identically on every compile target, including the
> wasm browser target where coroutine suspension is unavailable. The price is that `yield` may appear only
> in the generator's own body, never in a function it calls.

## Context

- [ADR 0028](0028-closing-the-remaining-magic-methods.md) closed every remaining magic *method*, but SPL's
  interfaces — `Iterator`, `IteratorAggregate`, `ArrayAccess`, `Countable` — are a separate mechanism and
  were left open. They are magic in the same sense: each makes a syntactic form dispatch to a method.
- The three are not equivalent, and the difference is what decides them. `ArrayAccess` and `Countable` buy
  *notation* over an explicit `->get($k)` or `->count()` — three characters, at the cost of a call that
  does not look like one. `Iterator` buys a *capability*: streaming a database cursor or a multi-gigabyte
  file without materializing it. Nothing else in the language provides that.
- `Countable` is close to incoherent here. It exists to make the global `count($x)` dispatch to a method;
  ADR 0011 removed global functions, so the equivalent would be `Core\Arr::count($obj)` — an *array*
  function dispatching on an object argument, which is worse than what it replaces.
- Generators interact with two decisions that pull in opposite directions. The runtime already has stackful
  coroutines, so implementing `yield` on them is nearly free and permits `yield` from arbitrary call depth.
  But [ADR 0025](0025-wasm-browser-target.md) states that coroutine-based suspension is unavailable in the
  browser, and it claims every *language* feature is unchanged across targets — only runtime facilities
  differ. Coroutine-based generators would make that claim false for the first time.
- This ADR blocks work already in progress: M2's IR must model suspension points inside loop bodies, and
  retrofitting them once M3 builds on the IR is expensive — the same argument the plan already makes for
  [ADR 0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s probe ids.

## Decision

### 1. Two interfaces, defined once

```
interface Iterator<T> {
    public function advance(): bool;   // move to the next element; false once exhausted
    public function current(): T;      // the element advance() just moved to
}

interface Iterable<T> {
    public function iterate(): Iterator<T>;
}
```

`current()` called before the first `advance()`, or after one returns `false`, throws. The pair is the
`MoveNext`/`Current` shape rather than a single `next(): ?T`, because the latter cannot distinguish "the
sequence ended" from "the next element is `null`", and MWL has nullable types.

### 2. `Iterable` and `Iterator` are compiler-owned generic interfaces

M8's rule that type variables are available only to declarations the compiler owns is kept, with one
narrow extension: a user class may **implement a compiler-owned generic interface at a concrete type** —
`implements Iterator<User>` — fixing `T` at the declaration site. It may not declare type variables of its
own. This is materially less than user-defined generics: no inference, no variance, no type-parameter
scope inside the class body, just substitution of one concrete type into a known interface.

### 3. `foreach` accepts three things; `ArrayAccess` and `Countable` do not exist

`foreach` accepts an `array<T>`, an `Iterable<T>`, or an `Iterator<T>`. An `array<T>` is iterated directly
by the IR with no interface call and no allocation, exactly as today. An `Iterable<T>` has `iterate()`
called once, then the loop drives the returned cursor. An `Iterator<T>` is driven directly. Any other
operand is a compile error naming this ADR.

`ArrayAccess` is rejected. `$obj[$k]` becoming a method call is the same implicit dispatch
[ADR 0014](0014-property-observer.md) rejected for `$obj->prop`, one syntactic level over, and it reads as
an array while none of `Core\Arr` applies to it. A collection exposes `->get($k)` and `->set($k, $v)`.

`Countable` is rejected, per *Context*. A collection exposes `->count()`.

### 4. Generators exist and lower to a state machine

A function whose body contains `yield` is a generator. Its declared return type must be `Iterator<T>`,
and every `yield` operand must satisfy `T`. Calling it runs no user code: it allocates and returns the
state object, which implements `Iterator<T>`. The body runs one segment per `advance()`.

Lowering is an explicit state-machine transform — the body is split at each `yield` into resumption states,
and every local live across a `yield` is stored in the state object rather than on a stack. It is **not**
built on the coroutine substrate. Three reasons, in priority order:

1. **Every target behaves identically** ([ADR 0025](0025-wasm-browser-target.md)). Generators are a
   language feature, and the browser target keeps them. That ADR's unavailable list stays exactly as it
   is — `spawn worker`, coroutine-based suspension, `.mwlx` — and does not grow a fourth entry.
2. **No stack to allocate or grow.** A generator is an ordinary object; iterating ten thousand of them
   costs ten thousand small objects, not ten thousand stacks.
3. It keeps coroutine suspension a property of *I/O*, not of ordinary control flow, which is a smaller and
   more checkable claim.

The cost is stated plainly: **`yield` is lexically confined to the generator's own body.** A helper
function called from a generator cannot yield into it. Where PHP would delegate, MWL writes
`foreach ($inner as $v) { yield $v; }`.

### 5. One-way only

A generator is a lazy sequence and nothing more. There is no `yield from`, no `send()` into a generator, no
`throw()` into one, and no generator return value to retrieve. PHP's `yield from` is the second spelling of
the `foreach`-and-re-yield loop above — test 6 of [ADR 0051](0051-standard-library-tiers.md) § 2, and it
costs O(nesting depth) per element rather than O(1), which is a real if small loss recorded here rather
than hidden. `send()`/`throw()` make a generator a bidirectional coroutine, which is a different feature
wearing the same syntax; MWL already has coroutines, and they are not spelled `yield`.

### 6. A generator does not cross a boundary

A generator object is not serializable and does not cross a `spawn`/`spawn worker`/`spawn script`
boundary — attempting either is the same refusal
[ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) already applies to a value it cannot copy
soundly. Although the state-machine lowering makes a generator an ordinary object with ordinary fields,
those fields are a compiler-chosen encoding of a suspended program point, and there is no version of
resuming one in another isolate that is meaningful. `clone` on a generator is likewise refused.

## Consequences

- **M2's IR must model a suspension point inside a loop body**, and lowering must be able to split a
  function into resumption states. This is the reason the ADR lands now rather than at M8. The transform
  itself may arrive later, but the IR must not foreclose it.
- **Definite assignment ([ADR 0022](0022-definite-property-initialization.md)) is unaffected in principle
  and more work in practice**: a local live across a `yield` is definitely assigned along every path
  reaching that `yield`, exactly as before, but the checker must reason about resumption edges rather than
  a single linear body.
- **`Core\Db` result streaming, `Core\IO` line reading and `Core\Xml`'s reader all return `Iterator<T>`**,
  implemented natively in Rust rather than as MWL generators. Most application code therefore consumes
  lazy sequences without ever writing one.
- **Two identifiers become reserved interface names**, `Iterable` and `Iterator`, joining `Comparable`
  ([ADR 0013](0013-comparable-interface.md)), `PropertyObserver` ([ADR 0014](0014-property-observer.md))
  and `Stringable` ([ADR 0028](0028-closing-the-remaining-magic-methods.md)).
- **`mwl convert` (M11) has a mechanical path for PHP's `Iterator`** — a 5-method interface collapsing to
  2, with `rewind()` and `key()` dropped — and no path at all for `ArrayAccess`, `Countable`, `send()` or
  `yield from`, each of which becomes a diagnostic naming its replacement.

## Alternatives rejected

- **Generators on the coroutine substrate.** One suspension mechanism instead of two (priority 4), and
  `yield` from any call depth. Rejected because it makes a *language* feature target-dependent for the
  first time, weakening [ADR 0025](0025-wasm-browser-target.md)'s central claim to buy an expressiveness
  gain — yielding from a helper — that is rare in practice and always rewritable as an explicit loop.
- **No generators; `Iterator` implemented by hand.** Smallest surface, no IR impact. Rejected: every lazy
  pipeline in userland becomes an explicit state class, and the state those classes hold is exactly what
  the compiler would have generated correctly. The stdlib's native iterators cover the common cases, but
  they cannot cover a user's own transformation of one.
- **Keep `ArrayAccess`.** Better PHP familiarity and nicer-reading collection code. Rejected on
  consistency: MWL has already paid the migration cost of removing implicit property dispatch, and keeping
  implicit subscript dispatch would leave the language inconsistent about the same question. It also
  interacts badly with [ADR 0024](0024-taint-tracking-for-injection-sinks.md) — whether `$obj[$k]` yields a
  tainted value would depend on an implementation the call site cannot see.
- **`Iterator` with a single `next(): ?T`.** One method instead of two. Rejected: it makes `Iterator<?T>`
  unrepresentable, and silently ending a sequence at the first `null` element is precisely the class of
  quiet wrong answer MWL's type system exists to prevent.

## Verification

- **M2:** fixtures for `foreach` over each of the three accepted operand kinds and a rejection for a fourth;
  a fixture asserting `$obj[$k]` on a non-array is a diagnostic naming this ADR; a generator whose declared
  return type is not `Iterator<T>` is a diagnostic; a `yield` outside a generator body is a diagnostic; a
  local live across a `yield` but not assigned on one incoming path is still an ADR 0022 error.
- **M3/M4:** an IR snapshot test for a generator with a `yield` inside a loop, asserting the resumption
  states and the lifted locals; a runtime test that a generator consumed twice reports exhaustion rather
  than restarting.
- **M8:** `Core\Db`'s streaming result set is an `Iterator<T>` and iterating a large table holds one row
  at a time, asserted against the request's memory accounting rather than by inspection.
- **M14:** the browser target compiles and runs a generator, which is the property § 4 exists to preserve.
