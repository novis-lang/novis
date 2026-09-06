A subclass *adds* properties, so `property<Animal>`'s names are all valid on a `Dog` while
`property<Dog>`'s are not all valid on an `Animal`. **`property<Animal>` therefore widens to
`property<Dog>`, and never back**: the argument is contravariant.

That is exactly opposite to `class<T>` (`rule:types/class-reference-variance`), and for the reason
that inverts it — a class reference is *produced* against its bound, while a key is *consumed* by a
receiver. Narrowing is written like every other narrowing, `as property<Animal>`, and is the run-time
check the conversion's second row already describes.

The rule at the site follows and adds nothing: `$obj->$key` requires `$obj`'s type to be a `T`, where
`T` is the key's argument. A key made against `Animal` reads a `Dog`; a key made against `Dog` does not
read an `Animal`.
