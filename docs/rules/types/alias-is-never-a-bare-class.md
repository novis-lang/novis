A `TypeExpr` that is nothing but one bare `ClassName`, `EnumName`, `self`, `static` or `parent` atom —
with no union, intersection, array wrapper or `?` sugar around it — is refused (`E0307`).

```nvs
type Id = SomeClass;              // rejected
type Ids = array<SomeClass>;      // fine
type Result = SomeClass|NotFound; // fine
```

The check runs at resolution, not while parsing: `type Id = SomeClass;` parses like any other alias,
and the resolver is where it becomes a diagnostic.

Allowing the bare form would be `use SomeClass as Id;` wearing the type grammar as a disguise: a
second name a *runtime* observer would plausibly expect to mean what `SomeClass` means everywhere,
which is exactly what `rule:statements/nothing-gets-a-second-name` closes. A `type` alias exists to
give a short name to a **shape** — a union, an intersection, a parameterised array, an object shape —
never to a single already-named class.

**Declined:**
- **A bare-class alias as a shorthand for a long fully-qualified name.** A `use` of the class under its
  own short name already shortens it, and the alias would be the same renaming in other syntax.
