Neither `let` nor `is` may name a class, interface, trait, enum, constant, function or parameter. PHP
8.6 deprecates both as identifiers; Novis, with no published corpus, refuses them outright and both at
once, because the cost of reserving now is near zero while the cost of taking either back later is a
breaking rename. A converted program renames any `let` or `is` it used as a name, and the rewrite is
mechanical.

**The two are reserved for unrelated reasons, and only one of them still has no construct.** PHP's
*Deprecations for PHP 8.6* RFC gives each its own motivation: `let` for the block-scoping construct,
whose own RFC was declined, and `is` for the Pattern Matching RFC, by name.

- **`let` is the empty kind** — the family of `eval`, `goto` and `list`, where the spelling is held
  and nothing is behind it, so nothing a user wrote has to be renamed out from under a future
  decision.
- **`is` is not.** It is the type test, `$x is T` (`rule:types/type-test`), which takes the settled
  half of the RFC PHP reserved the word for and leaves the rest of it unclaimed
  (`rule:php-migration/is-takes-pattern-matchings-type-patterns`).

The diagnostics name the living spellings: `var` declares an inferred local
(`rule:types/var-inference`), `is` and `instanceof` test (`rule:types/narrowing`) and `as` converts
(`rule:expressions/nullable-conversion`). Like every reserved word, both match in lower case only
(`rule:classes/reserved-spellings-are-lower-case`).
