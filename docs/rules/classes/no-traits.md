`trait`, a class-body `use Trait, ...;` and `insteadof` are not in the grammar. Each is a parse-time
diagnostic naming the replacement: an interface method with a body for shared behaviour
(`rule:classes/interface-default-methods`, `rule:classes/interface-private-methods`), and
`implements I by $field;` for shared state (`rule:classes/delegation-by-field`).

A trait bundles two unrelated things — sharing behaviour and sharing state — under one flattening
mechanism, and gives the reused code no type identity at all: a class using `Greets` is not a
`Greets`, cannot be tested for one (`rule:types/type-test`), and does not appear as a capability
under reflection.
An interface is the vehicle this language already uses for "declare a capability", and delegation is
the vehicle for "hold a collaborator". Splitting them means each half is a type the checker and the
IDE can see.

`insteadof` disappears with the mechanism it arbitrated: a collision is resolved by an ordinary
override calling the source it wants by name (`rule:classes/member-conflict-is-an-error`). What it
costs is that code using a trait is restructured by hand, and a trait's `static` property —
silently copied per consuming class — has no destination at all.

**Declined:**
- **A narrower trait, without the `as` clause and with `insteadof` kept.** The reused behaviour still
  has no type identity, properties are still copied per consuming class, and `insteadof` stays a
  second, trait-only conflict mechanism beside the override every other collision uses.
- **A `mixin` keyword that flattens like a trait and is a type like an interface.** It is a third
  construct for a job interfaces already do, which is more surface to teach for nothing an existing
  construct lacks (`rule:statements/nothing-gets-a-second-name`).
