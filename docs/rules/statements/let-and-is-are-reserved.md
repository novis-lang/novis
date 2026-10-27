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

Either word is `E0247` in three places, all reported by one helper
(`crates/nvs-syntax/src/parser/mod.rs:781`): where an expression starts
(`crates/nvs-syntax/src/parser/expr.rs:1501`); as the name of every declaration — a class,
interface, trait, enum, enum case, constant, method or `type` alias — through `parse_decl_name`
(`crates/nvs-syntax/src/parser/decl.rs:366`); and as a parameter, `$let` or `$is`, a promoted
constructor parameter included, in `parse_param` (`crates/nvs-syntax/src/parser/expr.rs:2048`). The
name is kept after the report, so the parse goes on and nothing else is reported for it. A member
reference after `->` or `::` still admits both words, since it declares nothing and no declaration
can match it. That is how `Core\Router\Match::is` is called: `Core` declares it, which a program
cannot, and the editor's stub of it is the one declaration the parser reports and keeps.

**Declined:**
- **Reserving `is` alone and leaving `let` free.** Reserving the second word costs the same as the
  first, and freeing it again stays possible where taking it back is not.
