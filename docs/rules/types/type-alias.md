`type Name = TypeExpr;` declares a compile-time-only synonym for a type expression, written either at
file and namespace scope alongside `use` and `namespace` or as a member of a class, interface or enum
body (`rule:types/class-scoped-alias`) — never inside a method body, a block or a closure body, where
it is `E0233` by name like any other declaration written where control flow can reach it.

```php
type UserId = uint;
type Result = User|NotFoundError;
type Matrix = array<array<float>>;
type Point  = {x: int, y: int};
```

`TypeExpr` is any production of the type grammar (`rule:types/grammar`) but one
(`rule:types/alias-is-never-a-bare-class`). The name is `PascalCase` like a class's, at both
sites and by the same check (`rule:core-api/identifier-casing`), because an alias stands where a class
name stands and reads as the type it names. The alias name is then lexically valid anywhere a class
name is, resolved by the same contextual lookup that already tells `self`/`static`/`parent` and an
enum's name apart from a class's, and reached through ordinary `use`/FQN resolution — nothing is
auto-imported.

An alias is **fully transparent, never nominal**: `UserId` and `uint` are the same type everywhere, in
both directions, needing no `as`, because after the checker resolves the alias there is only one type
there. It is not a newtype, and it has **zero runtime footprint** — nothing downstream of the checker
ever sees the alias name, not codegen, not the value layout, not `Core\Reflect::typeOf`, not an
isolate boundary crossing. That is what keeps it clear of `rule:statements/nothing-gets-a-second-name`:
it creates no name a runtime observer can see.

Aliases are resolved eagerly and a cycle is a diagnostic — `type A = B; type B = A;` is rejected at
check time rather than left to loop or bottom out at `mixed`. They are non-parametric:
`type Rows<T> = …` is out of scope while user-defined generics are.
