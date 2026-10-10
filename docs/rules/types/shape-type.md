```nvs
function move(object {x: int, y: int} $p): void { $p->x += 1; }
type Point = {x: int, y: int};
```

A shape type is **not** a class and **not** an interface. It is a compile-time-only structural
constraint, checked at each site a value flows into a binding of that type — assignment, call
argument, return: the source's already fully known type must have at least the named fields, each
satisfying the shape's declared type by ordinary assignability. No new comparison logic exists for it.

- **Width subtyping**: a source with extra fields still satisfies the shape, so an already-shaped
  value never needs re-wrapping because a caller cares about two of its five fields.
- **Field types** are ordinary assignability with one exception, and a shape gets no variance rule
  of its own. The exception is the `int`/`uint` → `float` widening (`rule:types/implicit-widening`):
  it does not reach a field of a value that already exists, at any depth of the field type. Such a
  value is shared, not copied, so its field cannot be converted. `{w: int}` does not satisfy
  `{w: float}` or `{w: ?float}`, `{w: array<int>}` does not satisfy `{w: array<float>}`, and a class
  with an `int $w` property does not satisfy `{w: float}` either; `{w: int|float}` accepts both,
  because `int` is one of its members.
- **An anonymous object placed at a declared shape** takes each field the declaration names at the
  declared type, when its value fits it: `{w: 2}` and `{w: $count}` at `{w: float}` are `{w: float}`,
  and the literal stores `2.0`. An array literal in a field is placed the same way, so `{xs: [1, 2]}`
  at `{xs: array<float>}` stores two floats (`rule:types/arrays`). A field the declaration does not
  name keeps the type of its value.
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

The optional key exists for request input. Without it, a field the client may omit has to be declared
`?T`, and "the client may leave this out" and "the client may send `null` here" collapse into one fact,
which is the ambiguity `rule:errors/ambiguous-input-refused` refuses everywhere else.

**Memory:** one `bool` per shape field, in the interned type descriptor. That is one per distinct type
in the program, and nothing per value or per request.

**Declined:**
- **A read of an optional field that answers `?T`.** It merges optionality into nullability at exactly
  the site a program acts on the difference, and gives an absent key a second answer beside the throw
  an absent array key gives.
- **Refusing a bare read of an optional field.** A refusal has to name a replacement, and the
  replacement is `$p->a ?? $default`, which the throwing read already admits.
- **A shaped `array<T>`, typed key by key.** `array<T>` is homogeneous and invariant with one interned
  descriptor per header, so a per-key array would be a second array family with its own variance.
  Converting into a shape at the boundary is the safer half of that trade.
- **The `interface` keyword for a shape.** An `interface` is a nominal, `implements`-declared,
  method-bearing contract, and a structural, property-only meaning would give one keyword two
  meanings (`rule:statements/nothing-gets-a-second-name`).
- **Exact-match shapes, with no width subtyping, or an option that refuses extra fields.** They are
  stricter for no safety gained, force re-wrapping a value that already has every field a caller
  reads, and let a client that adds a field break a server. An option choosing between the two
  behaviours is a flag `rule:core-api/shape-rules` refuses.
- **A shape that requires a method.** A shape is data only. Behaviour belongs to a declared class or
  interface.
