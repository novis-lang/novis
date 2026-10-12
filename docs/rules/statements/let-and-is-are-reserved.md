Neither `let` nor `is` may name a class, interface, trait, enum, constant, function or parameter.
Reserving a word costs nothing while no program uses it, and taking one back later is a breaking
rename, so both are reserved now.

**The two are reserved for unrelated reasons, and only one of them still has no construct.**

- **`let` is the empty kind** — the family of `eval`, `goto` and `list`, where the spelling is held
  and nothing is behind it, so nothing a user wrote has to be renamed out from under a future
  decision.
- **`is` is not.** It is the type test, `$x is T` (`rule:types/type-test`), and the only one there is.
- **`instanceof` is neither**, and is the third spelling this rule's diagnostics have to know about:
  it is a word Novis refuses where it is written, naming `is` as the rewrite
  (`rule:types/one-type-test`). It was never a name, so nothing is reserved by refusing it — the token
  exists only so the refusal can spell it.

The diagnostics name the living spellings: `var` declares an inferred local
(`rule:types/var-inference`), `is` tests (`rule:types/narrowing`) and `as` converts
(`rule:expressions/nullable-conversion`). Like every reserved word, both match in lower case only
(`rule:classes/reserved-spellings-are-lower-case`).

**Partly on disk.** Where an expression starts, either word is `E0247`
(`crates/nvs-syntax/src/parser/expr.rs:1501`, `crates/nvs-syntax/src/parser/mod.rs:781`). A
declaration's name is not checked: `parse_decl_name` accepts both
(`crates/nvs-syntax/src/parser/decl.rs:366`), so a method named `is` or `let` compiles, and
`tests/conformance/lang/mutual-recursion-across-two-classes.nvst:6` declares one.

**Declined:**
- **Reserving `is` alone and leaving `let` free.** Reserving the second word costs the same as the
  first, and freeing it again stays possible where taking it back is not.
