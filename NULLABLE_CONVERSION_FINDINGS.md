# Findings — a non-throwing conversion, `expr as ?T`

**Status: investigation, not a decision.** No ADR exists yet and nothing here is binding. Delete this file
when ADR 0066 lands or the idea is dropped — it must not outlive one of those two events, or it becomes a
second home for facts ADR 0066 would own.

## What is proposed

`$s as ?int` yields `?int`: the converted value where `$s as int` would succeed, `null` where it would
throw. Failure stops being control flow you catch and becomes a value you test.

**The syntax already parses.** `as` takes a type, and `?int` is a legal type in ADR 0007 § 3's grammar.
[Spec § 3.4](docs/spec/00-overview.md) fixes `as` as final and defers its "throwing behaviour" to
ADR 0007 § 2 — whose conversion table has **no nullable rows at all**. So this is not new syntax. It is an
unspecified corner of syntax that already exists, and today's compiler would have to reject or define it
either way.

## Why it fits MWL rather than fighting it

[ADR 0022 § 1](docs/adr/0022-definite-property-initialization.md) is titled "`?T` already covers 'may
legitimately hold no value' — this ADR does not add a second way." That is already the house rule; `as ?T`
applies it to a conversion result.

The load-bearing argument is [ADR 0063](docs/adr/0063-core-api-conventions.md)'s, not ergonomics. R4 makes
failure throw and R5 **bans** `try…`/`…OrNull`/`…Safe` member names, so today the only non-throwing path is
check-then-convert. That costs **two parses of the same bytes** on the request path, and requires the two to
agree forever — a hazard closed by hand in [spec § 13](docs/spec/01-core-library.md) by defining
`Validate::isInteger` as "true exactly when `as int` succeeds." An operator that carries the optional
variant for *every* type makes that class of pairing unnecessary, uniformly. It **strengthens** R4/R5
instead of eroding them: `Enum::from()`/`tryFrom()`, which ADR 0063 collapses to one member, gets a
principled second form that no member has to spell.

## Resolved during investigation

These need no decision — each follows from a rule already in force.

**1. Which failures become `null`, and which stay compile errors.** Drive it off ADR 0007 § 2's conversion
table, which is already the single source of truth: if a row exists for `S → T`, then `as ?T` is legal and
yields `null` exactly where that row throws. If no row exists — `array<int> as ?int` — it stays the compile
error it is today. Without this, `as ?T` degrades into a universal escape hatch that erases genuine type
errors.

**2. A conversion that cannot fail must reject `as ?T`.** `decimal → string` is total, so `as ?string`
there would produce a `?string` that is never null — a lie in the type, and a pointless null check
downstream. Make it a compile error naming `as T`, on R17 grounds (no operation reachable two ways).

**3. Class types are out of scope.** `$obj as ?SomeClass` is Swift's and Kotlin's downcast, but MWL already
answers class membership with `instanceof` plus ADR 0007 § 6 narrowing. Adding a second spelling is exactly
what R17 refuses. Restricting `as ?T` to § 2's scalar table keeps the feature small and the boundary
obvious.

**4. Qualifiers are unaffected.** `tainted string as ?int` is `?tainted int`.
[ADR 0024](docs/adr/0024-taint-tracking-for-injection-sinks.md)'s propagation and
[ADR 0033](docs/adr/0033-secret-qualifier-for-confidential-values.md)'s are orthogonal to the result's
nullability, and `as ?T` must never launder — it is a conversion, not one of the sink-named launderers.

## The one open decision: may the operand be nullable?

Everything else is mechanical. This is not.

**Option A — operand must be non-nullable.** `?string as ?int` is a compile error; handle absence first.

```
var $raw = Core\Request::query('id');   // ?tainted string
if ($raw === null) { … }                 // absent — its own branch
var $id = $raw as ?uint;                 // invalid — a different branch
```

Keeps "missing" and "invalid" distinct, which is the distinction PHP destroys and that ADR 0007 § 7's
divergence list exists to defend. Note it does **not** make conflation impossible — `($raw ?? '') as ?uint
?? 1` still collapses both — it makes conflation *visible*, costing three tokens and a reader's attention.
That is arguably correct pricing rather than a cost.

**Option B — `null` passes through to `null`.** `Core\Request::query('id') as ?uint ?? 1` works as one
line. Maximum ergonomics, and what Swift and Kotlin actually do. The cost is that at that site a missing
parameter and `"abc"` are indistinguishable — the same collapse `(int)$x > 0` performs, arrived at
deliberately instead of accidentally, but arrived at.

The trade is ergonomics against a distinction the rest of the language spends real surface defending.

## Two hazards found while investigating

**A bare truthy condition re-creates the PHP bug.**
[ADR 0035](docs/adr/0035-truthy-boolean-context.md) lets any type sit in an `if`, and both `null` and `0`
are falsy. So `if ($s as ?int)` is false for *invalid* and for *valid zero* alike — precisely the
`(int)$x > 0` defect, reintroduced in a shorter spelling. It cannot be forbidden without carving an
exception into ADR 0035, so it is a lint candidate for `mwl check` and must be named explicitly in the ADR.
Whichever option wins, `!== null` is the spelling to teach.

**It hands `mwl convert` a tempting wrong default.** With `as ?T` available, PHP's `(int)$x` has an obvious
mechanical target in `$x as ?int ?? 0` — which quietly preserves the silent-zero behaviour ADR 0007 § 7
diverges from on purpose (and still is not faithful: `(int)"12abc"` is `12` in PHP, `null` here, since `as`
requires the whole string). M11 should keep emitting a diagnostic rather than taking it.

## Knock-on edits if this is adopted

- **`Core\Validate::isInteger`/`isFloat`/`isBoolean` likely stop existing** — each becomes
  `as ?T !== null`, and R17 forbids both. That would supersede the sentence just added to spec § 13, which
  is fine; confirm first whether their subject is `string` or `mixed`, since a `mixed` subject is not
  equivalent and would survive.
- **Spec § 3.4** gains the nullable-target semantics; ADR 0007 § 2's table gains a column or a paragraph.
- **ADR 0063** gains a note that R5's ban on `try…` names is now *load-bearing in both directions* — the
  operator supplies what the banned names would have.

## Prior art

| Language | Spelling | Semantics |
|---|---|---|
| Swift | `x as? Int` | `Int?`, nil on failure; `as` / `as?` / `as!` is the full three-way split |
| Kotlin | `x as? Int` | safe cast, null on failure |
| C# | `x as T` | **null** on failure — the throwing form is the `(T)x` cast |
| Rust | `s.parse::<i32>().ok()` | `Option<i32>` |
| Go | `v, ok := x.(int)` | comma-ok |

Swift and Kotlin spell it `as?`. Nothing here needs inventing.

## Recommendation

Adopt, under **Option A**, scoped to ADR 0007 § 2's scalar table. It costs one line at the boundary and
buys back a distinction the language spends heavily to protect elsewhere; Option B's one-liner is reachable
anyway when someone means it, just visibly. Performance improves (one parse, not two), memory is unchanged
(`?T` is already representable), and `Core` likely gets smaller by three members.
