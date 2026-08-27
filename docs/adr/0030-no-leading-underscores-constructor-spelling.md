# ADR 0030 — No leading underscores anywhere; the constructor is spelled `constructor`, not `__construct`

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** revokes [ADR 0029](0029-identifier-casing-is-checked.md) § 2's leading-underscore allowance for
  properties, parameters and local variables — no identifier in any category may start with `_`, ever, with
  no exception left standing. Names `constructor` as Novis's one reserved constructor-method spelling,
  replacing PHP's `__construct`, and removes the `__construct` casing exception ADR 0029 carried for it,
  because a plain `constructor` needs no exception at all.
- **Amends:** [0029](0029-identifier-casing-is-checked.md) § "Scope" and § 2 — the leading-underscore
  allowance for properties, parameters and locals is revoked outright, and the `__construct` reserved-word
  exception is revoked because it no longer has anything left to except.
  [0028](0028-closing-the-remaining-magic-methods.md) — adds the `__construct`/`constructor` disposition row
  its own magic-method index never carried: kept, not closed, but respelled.

> **In short:** no property, parameter, or local variable name may begin with `_` — the leading-underscore
> allowance [ADR 0029](0029-identifier-casing-is-checked.md) granted them is revoked, with no replacement.
> Novis's constructor is spelled `constructor`, an ordinary method name, not PHP's `__construct` — it keeps
> every semantic role `__construct` had, only the spelling changes. Because `constructor` already satisfies
> ADR 0029's plain `camelCase` method rule, this removes the one casing exception that ADR ever carried:
> the check now has exactly zero exceptions, of any kind, for any identifier.

## Context

- ADR 0029 shipped with a leading-underscore allowance for properties/parameters/locals (`_cache`,
  `$_unused`) plus a same-day `__construct` reserved-word exception, so the one surviving magic-method
  spelling wouldn't trip the check.
- That exception was itself the single-name carve-out ADR 0029's own *Alternatives rejected* already argued
  against generalizing — the cleaner fix is removing the reason the exception exists (rename the
  constructor), not narrowing its wording.
- With that gone, the leading-underscore allowance loses its own justification too: it existed only to
  avoid a pointless breaking change for a PHP habit (`_privateField`) that converts mechanically (drop one
  character) — comfort ADR 0029's zero-suppression stance already declined to buy elsewhere.

## Decision

1. No property, parameter, or local variable name may begin with `_`. These three categories now use the
   exact `^[a-z][A-Za-z0-9]*$` pattern ADR 0029 already applies to methods — there is no longer a separate
   leading-underscore clause to state for them at all. `nvs convert` drops a leading underscore mechanically
   when converting a `_foo`-style PHP name, the same class of mechanical rename ADR 0029 § *Consequences*
   already prices in for casing conversion generally.
2. Novis's constructor method is spelled `constructor`. It keeps every semantic role `__construct` had —
   automatically invoked by `new`, the site where [ADR 0022](0022-definite-property-initialization.md)'s
   definite-property-initialization obligation attaches, discharged for inherited properties via
   `parent::constructor(...)` — only the spelling changes. `constructor` is an ordinary lowercase-first
   word, so it already satisfies ADR 0029's method-casing rule outright: no exception, table row, or
   reserved-word carve-out is needed for it, unlike the one this ADR removes.
3. `__construct` is no longer recognized as a constructor at all, and it can't compile as an ordinary method
   name either — two leading underscores fail the (now exception-free) method/property/parameter/local
   pattern exactly the way a PHP-style `_private` name does. The compiler names the fix directly rather than
   leaving the author to guess (see *Diagnostics*).

## Diagnostics

- A method literally named `__construct` → *Novis's constructor is spelled `constructor`, not `__construct`*
  — a targeted diagnostic distinct from the generic mis-casing message, since the mechanical rename
  algorithm (strip underscores, recase) would otherwise suggest `construct`, losing the actual intent.
- A property, parameter, or local name starting with `_` → the same *`{category}` names must be camelCase*
  diagnostic ADR 0029 already defines, now firing on every leading-underscore name instead of accepting one.

## Consequences

**Positive**

- Zero exceptions, full stop: the "no suppression mechanism" stance ADR 0029 already declared now holds
  with no asterisk next to it anywhere in the identifier grammar.
- PHP-native contributors get one clear, named rename target for the single most common magic method they
  will type from muscle memory, rather than a silent parse failure.
- `nvs convert`'s mechanical rename set gains exactly one fixed pair (`__construct` → `constructor`) instead
  of carrying a magic-method-name-preserving special case indefinitely.

**Negative**

- Another structural break from PHP, already priced into ADR 0029's and [ADR 0007](0007-explicit-type-system.md)
  § 7's divergence lists.
- `_`-prefixed PHP fields, parameters and locals — a common "private-ish"/"intentionally unused" convention
  — need mechanical renaming during conversion. Cheap per ADR 0029's own cost analysis, but one more line in
  that list.

## Alternatives rejected

- **Keep the `__construct` exception, drop only the leading-underscore allowance.** Rejected: leaves exactly
  the single-name carve-out ADR 0029 argued against generalizing from. Removing the need for the exception
  is the cleaner fix, not narrowing its scope.
- **Keep the leading-underscore allowance, rename only the constructor.** Rejected: no reason for the
  allowance survives once `__construct` no longer needs it, and PHP's underscore-prefix idiom converts too
  mechanically to be worth an exception (see *Decision* § 1).
- **A different constructor spelling** (`init`, `new`, `__init__`). Rejected: `constructor` is the closest
  match to what a PHP, Java, or TypeScript background already expects, needs no acronym or invented
  vocabulary, and — unlike `new` or `init` — cannot collide with a legitimately and separately named
  ordinary method.

## Verification

- Corpus entries, once [ADR 0029](0029-identifier-casing-is-checked.md)'s checker lands (its own
  *Verification* section names the home for these): a property/parameter/local named with a leading `_` is
  now rejected — a flip from ADR 0029's original "accepted" corpus case to a rejected one; a method named
  `__construct` gets the targeted "spelled `constructor`" diagnostic, not the generic `camelCase` one; a
  class declaring `constructor` produces no diagnostic and is recognized as satisfying ADR 0022's
  per-constructor obligation.
- No code exists yet that depends on the old spelling or the underscore allowance in a load-bearing way —
  the casing checker itself isn't implemented. This ADR is otherwise a text/example correction pass across
  ADR 0013, ADR 0022, and `crates/nvs-syntax`'s one test that used `__construct`.
