A required, positional, fixed-key shape parameter is declared as **one type carrying a non-empty list of
arms**, each arm an ordered field list: one arm for the ordinary case, two or more for a discriminated
union. Each field carries an optional default — no default makes the field required, a default makes it
omittable and is what an omitting call site materializes.

A shape is **only ever a whole parameter**. It is never a member of a union, never a field's or an option's
own type, and never inside a nullable, an array or a variadic tail. Making the union a property of the one
type rather than a composition of two is what keeps declaration order, because the type interner is
entitled to flatten, deduplicate and sort a union's members — and the flatten order of a shape's fields
*is* its ABI (`rule:core-api/shape-flattens-at-the-abi`). It is also the only arrangement in which every
arm is guaranteed to flatten at all.

In every other respect a shape parameter is ordinary: it sits at its own position, carries a name, and
carries a default entry only if it is itself optional. The options bag (`rule:core-api/options-bag`) keeps
its own spelling rather than being folded into this one, because the two differ in call-site rules — last,
unnamed, omittable in full — which are worth enforcing by a type rather than restating in a test.

**Declined:**
- **A union of two shapes, built from the union type that already exists.** The interner sorts a union's
  members, and a shape's field order is its ABI. Preserving order for unions whose members happen all to
  be shapes is a special case in the one place canonicalization has to be unconditional.
- **One field struct shared by bag options and shape fields, with an optional default.** A bag is
  optional because every option has a default, and the required-argument count depends on that. Sharing
  the struct turns that type-enforced property into a checked one, which is the wrong direction.
- **Folding the options bag into the shape parameter.** The bag's trailing position, namelessness and
  omittability would become flags on a general type, checked by tests instead of held by the enum. The
  two are one checked type (`rule:core-api/one-checked-shape-type`) and two call-site rules.
