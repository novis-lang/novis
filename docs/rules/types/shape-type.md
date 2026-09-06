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
- Plain `object` is the fully erased form, and every shape type is a subtype of it
  (`rule:types/object-top`).
- A shape is a type *expression*, so `rule:types/type-alias` names one for free.

**This is the one deliberate, tightly scoped exception to an otherwise fully nominal type system.**
Two unrelated named classes sharing field names and types are interchangeable wherever a shape type is
used — that is exactly what delivers "no shape needs declaring anywhere for two sides to agree" — and
it applies to this type family alone. Interface satisfaction stays as nominal as it was.

A field named by the shape and reached through it is proven present, so the read cannot fail; the
fetch is still name-keyed rather than a fixed offset, because two concrete objects satisfying one
shape may lay their fields out differently. A name the shape does not list is erased
(`rule:types/erased-member-access`).
