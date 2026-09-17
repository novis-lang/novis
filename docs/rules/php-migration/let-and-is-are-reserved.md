Neither `let` nor `is` may name a class, interface, trait, enum, constant, function or parameter. PHP
8.6 deprecates both as identifiers; Novis, with no published corpus, refuses them outright and both at
once, because the cost of reserving now is near zero while the cost of taking either back later is a
breaking rename. A converted program renames any `let` or `is` it used as a name, and the rewrite is
mechanical.

**The two are reserved for unrelated reasons, and only one of them still has no construct.**

- **`let` is the empty kind** — the family of `eval`, `goto` and `list`, where the spelling is held
  and nothing is behind it, so nothing a user wrote has to be renamed out from under a future
  decision. Its block-scoping RFC was declined in PHP and nothing replaces it here.
- **`is` is not.** It is the type test, `$x is T` (`rule:types/type-test`), and the only one there is.
- **`instanceof` is neither**, and is the third spelling this rule's diagnostics have to know about:
  it is a word Novis refuses where a PHP program writes it, naming `is` as the rewrite
  (`rule:php-migration/one-type-test`). It was never a name in either language, so nothing is
  reserved by refusing it — the token exists only so the refusal can spell it.

The diagnostics name the living spellings: `var` declares an inferred local
(`rule:types/var-inference`), `is` tests (`rule:types/narrowing`) and `as` converts
(`rule:expressions/nullable-conversion`). Like every reserved word, both match in lower case only
(`rule:classes/reserved-spellings-are-lower-case`).
