A program declares no type parameter of its own: a type variable belongs only to a declaration the
compiler owns — a `Core` member, and a user class implementing a compiler-owned generic interface at a
concrete type (`rule:iteration/concrete-generic-implements`). A user-declared name written with type
arguments, as a type or at a call, is refused where it is written.

A built-in type takes type arguments only where its grammar writes them: `array<T>`, `string<K>`,
`class<T>`, `property<T>` and the reserved `Iterable<T>` and `Iterator<T>`. A `<` after any other
built-in type, such as `int<int>` or `iterable<int>`, is `E0441` at the `<`, and the list is dropped
so the rest of the signature still parses. After `as`, a `<` is a comparison: `$n as int < 10`
compares the converted value.

Application code does not reach for one. A typed collection is `array<T>` with `Core\Arr`, a result
that may fail is `?T` and a throw, and an envelope is a structurally checked shape such as
`{items: array<User>, total: uint}` (`rule:types/shape-type`). The boundaries where a caller's type
cannot be inferred already write it at the call (`rule:attributes/call-site-type-argument`). Library
code is the real consumer, and `Core` is already parametric.

**Declined:**
- **User-declared generic functions.** Nothing written so far needs one; the narrowest form, a written
  type argument on a user-declared method, would reuse the call-site list rather than add a kind of type.
- **Generic classes.** One needs bounds the moment it sorts, a variance rule beside the array's, and
  either erasure, which checks without the typed path's speed, or one compiled copy per instantiation,
  which fights the one-immutable-file-per-unit cache
  (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file`).
