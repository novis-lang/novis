A property read and a method call through a union of classes compile when every class of the union
declares or inherits that member, the member is visible at the access site in every class, and every
class gives it the same type. `Box|Circle $s; echo $s->name;` compiles when both classes declare
`string $name`. This is `rule:types/unions-and-mixed`'s own sentence applied to a member: the
operation is valid for every member of the union, so the union permits it.

- **The same type** is identical, nullability and the `tainted` and `secret` qualifiers included. A
  method's **same signature** is the same parameters with their types, `inout`, defaults and
  variadic, the same return type and the same `static`. The result therefore has one type, and
  nothing after the access depends on which class was behind it.
- **A common parent is not needed and not used.** Two classes that inherit the member from one parent
  share it by the test above; two unrelated classes that each declare it share it the same way.
- **Every member of the union is a class.** An interface in the union keeps `E0477` for a call and
  `E0495` for a property, because an interface has no fixed offset, and the fixed-offset read is what
  the lowering below is. A nullable union follows the nullable rules a single class follows: `?->`
  reads it, and `->` is `E_NULLABLE_RECEIVER`.
- **A write** compiles only when the property is writable at the site in every class: the same type,
  not `readonly` in any class, and the same `set` visibility. A write checks the incoming value
  against one declared type, so a union whose classes agree on that type writes exactly what a write
  through each class would. A union where one class makes the property `readonly` or narrows its
  `set` visibility could not be written through one check, and stays an error.
- **A union of classes that do not share the member** is one new code, `E0881`, whose message names
  the class that lacks the member, the class where it is not visible, or the two types that differ.
  `E0495` and `E0477` stay for a receiver that can hold no object at all, and for a union with a
  non-class member (`rule:types/a-declared-type-answers-before-the-program-runs`).

**The lowering is a class test per class, then the access that class alone makes.** The checker records
each class of the union with its member, and the lowering tests the receiver with `ClassTest` for each
class but the last. Each branch lowers as the same access through that one class does: a fixed-offset
field load for a plain property, the hook for a hooked one, the class's own call for a method. The last
class is the fall-through, because the checker proved the receiver is one of the union's classes.
The field load does not check its receiver's class, so `bun nv soundness` reads a subclass's
redeclared property through a union both from a tested arm and from the fall-through.

- **Latency:** a union of `n` classes pays `n - 1` class tests and the access, with no name lookup
  and no error edge. An access through one class does not change.
- **Memory:** one list of classes per access site in the checker's table, per compiled program and
  never per request.

**Declined:**
- **The name-keyed slot fetch `rule:types/erased-member-access` uses.** It looks the name up on every
  read and adds an error edge for a case the checker already ruled out.
- **Reading through the nearest common parent.** It would accept only unions with such a parent, and a
  member two unrelated classes each declare is the case the `is` ladder this replaces is written for.
- **Interface arms.** A later change can add them through the dispatch a call on an interface already
  uses; refusing them now is the reversible choice.
