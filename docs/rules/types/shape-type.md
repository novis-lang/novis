```php
function move(object {x: int, y: int} $p): void { $p->x += 1; }
type Point = {x: int, y: int};
```

A shape type is **not** a class and **not** an interface. It is a compile-time-only structural
constraint, checked at each site a value flows into a binding of that type — assignment, call
argument, return: the source's already fully known type must have at least the named fields, each
satisfying the shape's declared type by ordinary assignability. No new comparison logic exists for it.

- **Width subtyping**: a source with extra fields still satisfies the shape, so an already-shaped
  value never needs re-wrapping because a caller cares about two of its five fields.
- **Field types** are ordinary assignability; a shape gets no variance rule of its own.
- **An optional key** is written `{name?: T}`, and that is not nullability: `{a?: int}` accepts a value
  with no `a`, `{a: ?int}` demands an `a` that may hold `null`, and the two accept different values so
  they intern apart. A source missing an *optional* field satisfies the shape; missing a *required* one
  does not.
- Plain `object` is the fully erased form, and every shape type is a subtype of it
  (`rule:types/object-top`).
- A shape is a type *expression*, so `rule:types/type-alias` names one for free — and a **local
  declaration** is the one slot where naming it first is required rather than optional, because a
  statement-initial `{` opens a block before it is anything else. `{x: int} $point;` is `E0134`,
  naming `type Point = {x: int}; Point $point;`; a parameter, a return type, a property, a class
  constant and a `foreach` binding each take the bare shape.

**This is the one deliberate, tightly scoped exception to an otherwise fully nominal type system.**
Two unrelated named classes sharing field names and types are interchangeable wherever a shape type is
used — that is exactly what delivers "no shape needs declaring anywhere for two sides to agree" — and
it applies to this type family alone. Interface satisfaction stays as nominal as it was.

A **required** field named by the shape and reached through it is proven present, so the read cannot
fail; the fetch is still name-keyed rather than a fixed offset, because two concrete objects
satisfying one shape may lay their fields out differently. A name the shape does not list is erased
(`rule:types/erased-member-access`).

An **optional** field is the middle case: the shape proves its type and not its presence, so the read
answers the declared `T` and an absent key is that same rule's checked, catchable throw — the answer
an absent array key already gives, with `??` and `isset` as the spellings that ask without throwing.
The read is deliberately **not** widened to `?T`: optionality and nullability are separate questions
(`rule:core-api/required-optional-and-nullable`), and one language does not answer "the key may be
absent" two different ways in two containers.
