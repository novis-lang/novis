- **Widening an operand's *representation* in `nvs-ir` moves a refcount decision you did not edit.**
  `lower_array_key`'s `bool` meant "aliases storage someone else owns" while its call sites read
  `false` as "a fresh buffer owed a release", which coincided only while every key was a `Ty::Str`;
  an unrendered `int` subscript turned the *unchanged* `if !key_aliasing { emit_release }` into a
  release of a plain integer, and only the IR snapshots showed it. Return the operand's `Ty` and
  guard on `ty.is_refcounted()`, and re-read every consumer of a widened operand for a decision
  phrased as the *negation* of the old invariant. [until: reviewed 2026-09-06]
