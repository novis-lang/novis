# ADR 0029 — Identifier casing is a hard compiler error: `PascalCase` types, `camelCase` members, `SCREAMING_SNAKE_CASE` constants

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** the casing every user-written identifier must use — class/interface/trait/enum names, enum
  cases, namespace segments, method names, property names (any visibility, static or instance), parameter
  names, local variable names, and class constant names. Does **not** cover whitespace, indentation, brace
  placement, or any other physical formatting — those stay explicitly out of scope, deferred to a possible
  future ADR (see *Revisiting*). Built-in type keywords (`int`, `uint`, `bytes`, …) and language keywords
  (`isset`, `require`, …) are reserved words the grammar already lowercases; this ADR does not add a rule
  for them, it only notes they already comply. `__construct` — the one magic-method spelling
  [ADR 0028](0028-closing-the-remaining-magic-methods.md) leaves standing — is the same kind of
  grammar-fixed reserved name: it is not a name a program chooses, so the method-casing rule below does not
  apply to it, and the double leading underscore that would otherwise trip § 2 is not a violation.
- **Amends:** none. No prior ADR states an identifier-casing rule; this is the first. Every existing ADR's
  code examples already happen to follow the convention this one now makes a compiler error — see *Context*.
- **Amended by:** [0030](0030-no-leading-underscores-constructor-spelling.md) — § 2's leading-underscore
  allowance for properties, parameters and locals is revoked outright (no exceptions, including
  `_cache`/`$_unused`); the `__construct` reserved-word exception this file's *Scope* and § 2 state above is
  also revoked, because MWL's constructor is renamed to `constructor` and needs no exception at all.
  [0032](0032-acronym-casing-rule-revoked.md) — § 1's "acronyms are one word, never kept all-caps" rule is
  revoked outright; the checker enforces only this file's table (the leading character's case, an
  alphanumeric rest), so `HTTPClient` is accepted on equal footing with `HttpClient`.
- **Relates to:** [0007](0007-explicit-type-system.md) (the type-atom keywords this ADR leaves alone),
  [0008](0008-static-and-global.md) (a `static` property is still a property — the same casing rule applies
  regardless of storage class), [0010](0010-enums-are-a-value-type.md) (its own examples already spell enum
  cases `Active`/`Banned` — this ADR formalizes that spelling, doesn't change it),
  [0011](0011-functions-and-constants-are-class-members.md) (every callable is a method and every constant a
  class constant — this ADR is what gives each of those two categories its casing rule),
  [0013](0013-comparable-interface.md)/[0014](0014-property-observer.md)/[0027](0027-callable-is-closures-only.md)/[0028](0028-closing-the-remaining-magic-methods.md)
  (the global interfaces these declare — `Comparable`, `PropertyObserver`, `Stringable` — already follow the
  "PascalCase, no role-marker prefix" rule this ADR states), [0015](0015-no-name-aliasing.md) (the same "one
  name, one spelling" ethos, applied to how a name looks rather than how many of them exist),
  [0016](0016-ide-integration.md) (`mwl-fmt`/`mwl-lsp` are the natural home for a rename quick-fix once this
  rule exists)

> **In short:** every user-written identifier's casing is checked at compile time and a mismatch is a hard
> error — not a lint, not a warning, and **there is no suppression mechanism**. Types (`class`, `interface`,
> `trait`, `enum`, an enum's own cases, and each namespace segment) are `PascalCase`. Methods are
> `camelCase`. Properties, parameters and local variables are `camelCase`, optionally with one leading
> underscore (`_cache`, `$_unused`). Class constants are `SCREAMING_SNAKE_CASE`. An acronym is treated as a
> single word (`HttpClient`, `parseXmlPayload`), never kept all-caps. There is exactly one accepted spelling
> per category, decided now while the standard library is still unwritten, because renaming it later is a
> breaking change with no deprecation path under [ADR 0015](0015-no-name-aliasing.md).

## Context

- PHP enforces only legal *characters*, never *case* — `php-src` itself mixes `snake_case` functions
  (`array_map`), `camelCase` methods (`DateTime::createFromFormat`), and inconsistently-cased class names,
  with no tooling to catch drift; userland PHP inherits the same freedom.
- [ADR 0011](0011-functions-and-constants-are-class-members.md) (every function/constant is a class member)
  already implied this question needed an answer: PHP could dodge "what casing does a method/constant take"
  by having free functions and global constants; MWL closed off that dodge.
- A formalization more than a new choice: every accepted ADR's examples already followed one consistent
  style by accident of single authorship (`PascalCase` classes, `Active`/`Banned` enum cases
  ([0010](0010-enums-are-a-value-type.md)), `MAX`/`DEFAULT_STATUS` constants, `Core\Str`-style namespaces) —
  an accident that stops holding the moment a second contributor or a converted PHP project writes code.

## Investigation — how other languages handle this

- **Go**: checks only first-letter case, because that bit *is* semantics (exported/unexported) — formatting
  otherwise stays tooling-only (`gofmt`).
- **Haskell/Elm/OCaml**: also first-letter-only, for parser disambiguation (constructor/type vs value), not
  style.
- **Rust**: the only mainstream language checking whole-identifier casing — but as a default-on **warning**,
  suppressible per item (`#[allow(...)]`), because FFI/macro/serialization names legitimately need to
  violate it.
- **Java/C#**: tooling-only (Checkstyle, Roslyn + `.editorconfig`), never compiler-enforced.
- **Python** (PEP 8): pure convention, zero enforcement.
- No mainstream precedent for "hard error, zero exceptions" — but it extends a pattern MWL already applies:
  [0011](0011-functions-and-constants-are-class-members.md), [0015](0015-no-name-aliasing.md),
  [0021](0021-single-file-inclusion-construct.md) and [0027](0027-callable-is-closures-only.md) each pick one
  accepted spelling and reject the rest outright rather than warn. Rust's warning-with-escape-hatch model is
  the road not taken (see *Alternatives rejected*).

## Decision

Every identifier below is checked against exactly one pattern. There is no configuration, no per-project
override, and no suppression annotation — the same "one canonical spelling" stance
[ADR 0015](0015-no-name-aliasing.md) already takes for names generally.

| Category | Convention | Pattern | Example |
|---|---|---|---|
| Class, interface, trait, enum | `PascalCase` | `^[A-Z][A-Za-z0-9]*$` | `HttpClient`, `Comparable`, `Status` |
| Enum case | `PascalCase` | `^[A-Z][A-Za-z0-9]*$` | `Active`, `Banned` |
| Namespace segment | `PascalCase` | `^[A-Z][A-Za-z0-9]*$` | `Core\Html\Markup` |
| Method (instance or `static`) | `camelCase` | `^[a-z][A-Za-z0-9]*$` | `getName`, `fromString` |
| Property (any visibility, instance or `static`) | `camelCase`, one optional leading `_` | `^_?[a-z][A-Za-z0-9]*$` | `userName`, `_cache` |
| Parameter | `camelCase`, one optional leading `_` | `^_?[a-z][A-Za-z0-9]*$` | `userId`, `_unused` |
| Local variable | `camelCase`, one optional leading `_` | `^_?[a-z][A-Za-z0-9]*$` | `$rowCount`, `$_tmp` |
| Class constant | `SCREAMING_SNAKE_CASE` | `^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$` | `MAX_RETRIES` |
| Built-in type keyword, language keyword | lowercase | *(reserved word — grammar-fixed, not checked)* | `int`, `uint`, `bytes`, `isset`, `require` |
| `__construct` | *(reserved word — grammar-fixed, not checked)* | *(reserved word — grammar-fixed, not checked)* | `__construct` |

### 1. Acronyms are one word, never kept all-caps

`HttpClient`, `IoStream`, `XmlParser`, `parseXmlPayload`, `httpStatus` — an acronym gets exactly the same
single capitalized (or lowercased) first letter as any other word in the identifier. PHP's own historical
`DOMDocument`/`SimpleXMLElement`-style all-caps acronyms are rejected. This is a compiler rule, not a style
suggestion, so it has to be checkable with no external knowledge: "is `HTTP` an acronym the compiler should
recognize" would require a maintained dictionary (and still breaks the moment two recognized acronyms are
adjacent — `HTTPXMLParser`). Treating every acronym as an ordinary word needs no dictionary at all.

### 2. A single leading underscore is allowed only on properties, parameters and locals

`_cache`, `$_unused`, `_id` are accepted; `__cache` (two or more leading underscores) is rejected outright,
and a leading underscore is never accepted on a class/interface/trait/enum/method/namespace-segment/constant
name. This exists because a leading underscore for "private field," "intentionally unused parameter," or
"scratch/temporary local" is idiomatic across enough languages (PHP itself included, informally) that
banning it outright would fight a habit for no gain — but it stays a strictly optional *prefix* to an
otherwise ordinary `camelCase` body, not a second casing style. Rejecting two-or-more leading underscores
is also a small extra guardrail against a name that merely *looks* like one of PHP's magic-method spellings
([ADR 0028](0028-closing-the-remaining-magic-methods.md) already closed the methods themselves; this stops a
property or variable from visually echoing them) — with the one necessary exception that `__construct`
itself, the single magic-method spelling ADR 0028 leaves standing, is a reserved grammar keyword rather than
a chosen method name, so this rule does not flag it.

### 3. There is no suppression mechanism

Unlike Rust's `#[allow(non_snake_case)]`, MWL has no attribute, pragma, or per-file opt-out for this check,
and this ADR introduces none. A name that must violate the convention — because it round-trips through
reflection-driven construction, mirrors an external wire format, or comes from `mwl convert` unmodified —
does not compile until it is renamed. See *Consequences* and *Alternatives rejected* for what this costs.

## Diagnostics

- A class/interface/trait/enum/enum-case/namespace-segment name not matching `PascalCase` → *`{name}` must
  be PascalCase, e.g. `{suggested}`*
- A method name not matching `camelCase` → *method names must be camelCase, e.g. `{suggested}`*
- A property/parameter/local name not matching `camelCase` (with at most one leading `_`) → *`{category}`
  names must be camelCase, e.g. `{suggested}`*
- A class constant not matching `SCREAMING_SNAKE_CASE` → *class constants must be SCREAMING_SNAKE_CASE, e.g.
  `{suggested}`*
- Two or more leading underscores on a property/parameter/local → *at most one leading underscore is
  allowed, e.g. `{name with one underscore}`*

Every diagnostic names a mechanically-derived suggestion (split on existing case/underscore boundaries,
re-join in the target convention) so the fix is always a one-line rename, never a judgment call.

## Consequences

**Positive**

- Every identifier's syntactic category is legible from its spelling alone, before a reader — or an IDE —
  resolves anything against a symbol table. This is the same value
  [ADR 0011](0011-functions-and-constants-are-class-members.md)/[0012](0012-no-superglobals.md)/[0015](0015-no-name-aliasing.md)
  already bought by closing off ambient/string-addressed names, applied here to *how a name reads* rather
  than *how it resolves*.
- No bikeshedding is possible, and no codebase can drift into two competing house styles the way PHP
  projects routinely do — directly serving this project's stated goal of one consistent style every
  contributor is compiler-forced into, not just style-guide-encouraged into.
- `mwl-fmt`/`mwl-lsp` ([ADR 0016](0016-ide-integration.md)) get a mechanical, always-correct rename
  quick-fix for free — the suggested name in every diagnostic above *is* the quick-fix.
- Decided now, while `Core`'s stdlib surface is still unwritten past a handful of ADR examples. Every method
  and constant the stdlib will ever expose gets named once, correctly, instead of needing a
  compatibility-breaking mass rename after userland code already depends on the wrong casing.
- The check needs no name resolution: a class declaration, a method declaration, a parameter, a local
  binding, and a constant declaration are already distinct AST node kinds as of M1's parser
  (`crates/mwl-syntax`). The diagnostic can be raised directly off each node's spelling, the same
  grammar-level placement Haskell/Elm use for their (narrower) case rule — see *Verification*.

**Negative**

- **A structural break from PHP**, joining the divergence list [ADR 0007](0007-explicit-type-system.md) § 7
  already carries: PHP source using `snake_case` functions/variables (idiomatic PHP style) or
  inconsistently-cased class names does not compile unmodified. Mechanical for `mwl convert`
  ([M11](../implementation-plan.md)) in the common case — rewrite each identifier to the target convention
  — except where a converted name collides with another after rewriting, which needs a human decision.
- **Zero exceptions, forever, by explicit choice.** Nothing generated, reflected into existence, or mirroring
  an external format (a database column, a wire-format field, a wasm import) gets a way to keep a
  non-conforming spelling — a wrapping/mapping layer is the only route, on every such boundary this project
  ever grows.
- A small ongoing cost on every future ADR and every stdlib method: the name has to be chosen correctly the
  first time, since [ADR 0015](0015-no-name-aliasing.md) means there's no cheap alias to paper over a
  mis-cased name discovered later.

## Alternatives rejected

- **Warn instead of hard-error (Rust's model).** A warning will, in practice, be ignored indefinitely,
  defeating the goal of being compiler-forced rather than nudged — and would be the only lint-severity check
  among this project's otherwise-hard PHP-dynamism diagnostics ([0011](0011-functions-and-constants-are-class-members.md),
  [0015](0015-no-name-aliasing.md), [0021](0021-single-file-inclusion-construct.md), [0027](0027-callable-is-closures-only.md)).
- **A suppression attribute/pragma for generated/interop code.** Would be MWL's first compiler-level
  suppression mechanism of any kind — a bigger precedent than the rule itself. Worth revisiting only if a
  concrete boundary forces the question (see *Revisiting*).
- **First-letter-only casing (Haskell/Elm-style).** That rule disambiguates constructor from value at parse
  time — a problem MWL doesn't have, since declaration keywords already say what a name is; doesn't deliver
  this ADR's full-consistency goal.
- **Keep acronyms all-caps** (`HTTPClient`). Needs a maintained dictionary and still breaks on adjacent
  acronyms (`HTTPXMLParser`); one-word treatment is unambiguous with no dictionary.
- **An `I`-prefix for interfaces** (`IComparable`). Would rename interfaces already spelled in accepted ADRs
  ([0013](0013-comparable-interface.md), [0028](0028-closing-the-remaining-magic-methods.md)) and introduces
  a role-marker convention used nowhere else.
- **`lowercase` namespace segments** (`core\str`). Would rename every `Core\...` reference already accepted.
- **Extending to whitespace/indentation rules.** Out of scope by this ADR's own framing (see *Revisiting*).

## Revisiting

- **Whitespace/indentation-as-syntax** (Python-style significant indentation, or merely a mandatory brace
  style) is a materially bigger decision than identifier casing and is deliberately not addressed here. If
  it's ever pursued, it gets its own ADR rather than an amendment to this one.
- **User-defined generic type parameters** have no naming convention here because they don't exist yet —
  [ADR 0007](0007-explicit-type-system.md) § 3's *Consequences* already defers user-defined generics as a
  question of its own. Whoever decides that ADR also decides how a type parameter is spelled.
- **The zero-suppression stance** is the one piece of this decision most likely to be revisited, and only
  under real pressure from a concrete boundary this project doesn't have yet — a wasm host-import surface,
  a database-mirroring layer, or a reflection-driven construction path that must reproduce an external
  name exactly. If that day comes, the question is narrow ("does *this one* boundary need an escape") rather
  than "should the check be a warning after all."

Verification, in the order it becomes possible:

- Every category in the table above is already a distinct AST node kind as of M1
  (`crates/mwl-syntax`) — a class declaration, a method declaration, a parameter, a property, a local
  binding, a constant declaration, an enum case and a namespace segment are never ambiguous with each
  other. This check needs no symbol resolution, so it is not gated on the rest of M2's checker
  (`mwl-types`/`mwl-hir`) the way most of that milestone's diagnostics are.
- A corpus entry per category in the table: one file each for a correctly-cased and a mis-cased class,
  interface, trait, enum, enum case, namespace segment, method, property, parameter, local variable and
  class constant, plus one entry for a two-or-more-leading-underscore name and one for an all-caps acronym,
  each asserting the exact diagnostic and suggested rename from *Diagnostics* above.
- A negative corpus entry: a class declaring `__construct` produces no casing diagnostic at all, confirming
  the reserved-word exception above is actually wired into the check and not just stated in prose.
