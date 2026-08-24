# ADR 0035 — A condition is judged by PHP's full truthy table; every other `bool` position stays checked

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** the exact set of syntax positions that test a value's truthiness rather than requiring it
  already be `bool` — `if`/`elseif`'s condition, `while`/`do…while`'s condition, `for`'s middle clause, the
  ternary/elvis operator's condition, and `&&`/`||`/`!`'s operand(s); the runtime rule those positions use to
  turn an arbitrary value into a branch decision.
- **Amends:** none. [ADR 0007](0007-explicit-type-system.md) never stated a rule for these positions —
  `mwl-types`' `check_stmt` already passes `expected: None` for every condition, and nothing enforced or
  rejected any type there. This ADR is the first to say so on purpose, closing a gap ADR 0007 left open
  rather than reopening a decision it made.
- **Amended by:** 0090 — the operator pair `==`/`===` is now the one operator `==`; nothing else about
  this decision changed.
- **Relates to:** 0007, 0010, 0034

> **In short:** a condition never needs an explicit `as bool` or comparison to be written just to express
> "is this set/non-empty/non-zero." `if ($rows)`, `while ($line)`, `$s && $n`, and `!$user` all type-check
> for any operand type, and at runtime resolve exactly PHP's own truthy table: `null`, `false`, `0`, `0.0`,
> `""`, and the string `"0"` are falsy; an empty `array<T>` (of any `T`) is falsy; every other value —
> including every object, callable and enum case, and every array with at least one element
> regardless of its contents — is truthy. This is the **one** place MWL performs an implicit, PHP-shaped
> conversion on a value's declared type; everywhere else — assigning into a `bool`-typed parameter, property,
> return, or local; `==`/`match` — ADR 0007's rule is untouched: an explicit `as bool` or comparison is
> still required, and nothing narrower than a condition gets this exception.

## Context

- Priority 4 argues for it directly: ported PHP code leans on `if ($str)`, `if ($rows)`, `if ($err)`
  constantly. Requiring `$str != ''`, `count($rows) > 0`, or an `as bool` conversion at every such site is
  exactly the ceremony ADR 0007's own verbosity trade-off already worries about — compounded at every branch
  rather than at every declaration.
- Priority 1 doesn't actually require the strict reading: the risk ADR 0007 § 2 closes is a *declared type
  silently changing*. A truthiness test produces no value and changes no binding's type, so it can't produce
  a "wrong answer far from its cause." PHP's one real footgun here — `"0"` falsy but `"0.0"`/`"false"` truthy
  — is a string-legibility complaint, not a type-safety hole, and no worse in MWL than it already is in PHP.
- The rejected alternatives (below) either reintroduce the ceremony this decision removes, or draw a line
  inside PHP's truthy table with no principled edge — worse than either keeping the whole table or dropping
  it entirely.

## Decision

**Exactly six syntax positions test a value's truthiness instead of requiring it already be `bool`:**
`if`/`elseif`'s condition, `while`/`do…while`'s condition, `for`'s middle clause, `?:`'s (ternary and elvis)
condition, and `&&`/`||`/`!`'s operand(s). Every one of them accepts a value of **any** type — `mixed`, a
union, a scalar, an array, an object, an enum case, `callable` — with no diagnostic for not
already being `bool`.

### 1. The one exception to ADR 0007 § 2, named precisely

ADR 0007 § 2 states a declared type never changes except through `as` or a new binding. A condition's
truthiness test is not a conversion at all under that rule — it produces no value of a different type
that could be read back, assigned, or passed on. It answers exactly one question, "branch or don't," and
the answer itself is always freshly computed, never stored as a re-typed `$x`. That is what keeps this a
narrow, named carve-out rather than a hole in ADR 0007: nothing here lets a `string` flow into a `bool`
*binding* without `as bool`, and nothing here changes what `==`/`match` do.

### 2. The truthy table

Exactly PHP's own rule, applied to MWL's own type set:

| type | falsy | truthy |
|---|---|---|
| `null` | always | never |
| `bool` | `false` | `true` |
| `int` / `uint` | `0` | anything else |
| `float` | `0.0` (including `-0.0`; `NAN` is truthy) | anything else |
| `string` | `""` and exactly the one-character string `"0"` | every other string, including `"0.0"` and `"false"` |
| `array<T>` | empty, for any `T` | one or more elements, regardless of their content |
| class instance, `callable` | never | always |
| enum case | never | always — see *4* |
| `mixed` / a union | resolved dynamically per this table, dispatching on the value's runtime type | — |

`null`/`never`/`void` cannot occur as a condition's static type in the first place outside `mixed`
(ADR 0007 already keeps `void`/`never` out of value position); this table only needs to cover what can
actually reach a condition.

### 3. Where the value is statically known, the check costs nothing at runtime

When a condition's static type is a scalar, array, class, `callable`, or enum — not `mixed` or a
union — the compiler already knows which row of the table applies, and lowers straight to the matching
native test (`x != 0`, `len != 0`, `true`, …) with no dispatch. Only a `mixed`/union-typed condition pays for
a runtime helper that inspects the value's tag and applies the table dynamically — the same "the fast path
is the typed one" shape ADR 0007 § 6 already commits to for every other `mixed` operation.

### 4. Enum cases are always truthy, not judged by their backing value

[ADR 0010](0010-enums-are-a-value-type.md) makes an enum case a closed, named integer value rather than
PHP's class-like construct — but real PHP source never observes an enum case as falsy: PHP enum cases are
objects, and PHP has no mechanism to make an object falsy at all, so every ported `if ($status)` where
`$status` is an enum case has only ever meant "always true." Judging an enum case by its backing integer
instead (a case backed by `0` reads falsy) would be the "more internally consistent" choice on MWL's own
representation, but it would silently change the behavior of every such condition relative to both PHP and
MWL's own enum semantics as observed from outside `Core\Reflect`. Enum truthiness therefore follows the
object rule (always truthy), not the integer rule its backing type might suggest.

### 5. What stays unaffected

Nothing about assignment, parameters, properties, returns, `==`, or `match` changes. `bool $b = $s;`
is still `E_TYPE_MISMATCH` requiring `$s as bool` or a comparison; `match (true) { $s => ... }` still tests
each arm with the equality rule
([0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md)), not truthiness (`match` is not
in the position list above). `??`/`?->`
already test null-vs-not, an entirely separate axis from truthiness, and are likewise untouched.

## Consequences

**Positive**

- Every "is this set" condition ported from real PHP — `if ($rows)`, `while ($line)`, `if ($err)`,
  `$user && $user->active`, `!$items` — type-checks and runs with its original meaning, with zero rewriting
  needed by `mwl convert` (M11) for this shape specifically, unlike the mechanical rewrites ADR 0034 and
  every other rejected-construct ADR require.
- The exception is small and precisely bounded — six syntax positions, one table — rather than a general
  "anything convertible to bool converts implicitly" rule that would have no natural edge.
- A statically-typed condition costs nothing extra at runtime; only `mixed`/union conditions pay for the
  dynamic dispatch, symmetric with every other `mixed` cost in the language.

**Negative**

- **PHP's own truthy-table footguns travel unchanged**: `"0"` is falsy but `"0.0"` and `"false"` are
  truthy, and a non-empty array is truthy regardless of whether every element is itself falsy. Code review,
  not the compiler, is what catches a condition that meant something else — the same trade ADR 0007 § 6
  already accepts for `mixed` as a whole.
- **One more rule to hold in mind reading any condition**: "what is this expression's type, and is it one of
  the six positions" now matters for whether a value can appear bare or needs `as bool`. Mitigated by the
  position list being short, fixed, and syntactically obvious (a condition always looks like a condition).
- **An enum case's truthiness is arguably surprising given ADR 0010's own representation** — a case backed
  by `0` still reads truthy. Argued deliberately in *4* as matching every ported program's actual
  expectation over matching MWL's internal representation.

## Alternatives rejected

- **No implicit truthiness anywhere — every condition must already be `bool`.** The strict ADR 0007 reading
  with zero exceptions. Rejected: the ceremony this forces at every branch is the priority-4 cost this
  decision avoids, and priority 1 doesn't require it (*Context*).
- **A restricted subset** (only `null`/`bool` tested implicitly; other scalars still need `as bool`).
  Rejected: draws a line inside PHP's truthy table with no principled basis — `if ($rows)` is exactly as
  common as `if ($maybeNull)`.
- **Full truthy table for `if`/`while`/ternary only — `&&`/`||`/`!` still require `bool` operands.** Rejected:
  `$user && $user->active` is at least as common a ported idiom as a bare `if`, and splitting the position
  list this way means the same value is bare in one spot and rejected two tokens later.
- **Judging an enum case by its backing integer.** Rejected in *Decision § 4*: matches MWL's own
  representation but not what any real program has ever observed an enum case's truthiness to mean.

## Revisiting

- If real ported code turns out to rely on `"0.0"`/`"false"`-shaped strings being treated as falsy (a
  PHP-authoring mistake, not a real PHP semantic — PHP never did this either), that is a bug in the source
  being ported, not a reason to reopen this table.
- A lint flagging a condition whose static type makes it provably always-truthy or always-falsy (a class
  instance, a non-nullable `callable`) is a plausible follow-on, deferred as tooling rather than folded into
  this decision.

Verification, in the order it becomes possible:

- **M2** (now): `mwl-types`' `check_stmt` passes no expected type into a condition
  (`crates/mwl-types/src/locals.rs`'s `StmtKind::If`/`StmtKind::While` arms already call `check_expr(cond,
  None, …)`), and `crates/mwl-types/src/check.rs`'s `a_non_bool_condition_is_never_a_type_mismatch` locks in
  that a `string`/`int`/`array<T>` condition, and `&&`/`||`/`!` over mixed operand types, never produce
  `E_TYPE_MISMATCH`. `for`'s middle clause and the ternary/elvis condition share the same unconstrained
  `check_expr` call once statement/expression checking reaches them.
- **M3**: the first backend lowers a statically-typed condition straight to the matching native test with no
  helper call; a `mixed`/union-typed condition lowers to one runtime helper implementing the full table,
  with a conformance suite covering every row (including `"0"` vs `"0.0"`, `-0.0`, `NAN`, an empty vs.
  one-element array of any element type, and an enum case backed by `0`).
