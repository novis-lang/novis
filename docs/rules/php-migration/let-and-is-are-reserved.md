`let` and `is` are reserved words, in the family whose construct does not exist — `eval`, `goto`,
`list` — so the spelling stays available and nothing a user wrote ever has to be renamed out from
under a future decision. PHP 8.6 deprecates both as identifiers to reserve them for future use;
Novis, with no published corpus, reserves them outright, and both at once, because the cost of
reserving now is the same and near zero while the cost of taking either back later is a breaking
rename.

The diagnostics name the living spellings: `var` declares an inferred local
(`rule:types/var-inference`), `instanceof` tests (`rule:types/narrowing`) and `as` converts
(`rule:expressions/nullable-conversion`). Like every reserved word, both match in lower case only
(`rule:classes/reserved-spellings-are-lower-case`). A converted program renames any `let` or `is` it
used as a name; the rewrite is mechanical.
