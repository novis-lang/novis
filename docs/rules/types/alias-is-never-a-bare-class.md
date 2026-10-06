A `TypeExpr` that is nothing but one bare `ClassName`, `EnumName`, `self`, `static` or `parent` atom —
with no union, intersection, array wrapper or `?` sugar around it — is refused (`E0307`).

```nvs
type Id = SomeClass;              // rejected
type Ids = array<SomeClass>;      // fine
type Result = SomeClass|NotFound; // fine
```

Allowing the bare form would be `use SomeClass as Id;` wearing the type grammar as a disguise: a
second name a *runtime* observer would plausibly expect to mean what `SomeClass` means everywhere,
which is exactly what `rule:statements/nothing-gets-a-second-name` closes. A `type` alias exists to
give a short name to a **shape** — a union, an intersection, a parameterised array, an object shape —
never to a single already-named class.
