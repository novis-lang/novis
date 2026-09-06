Type variables belong to declarations the compiler owns. A user class gets one narrow door onto
them: it may **implement a compiler-owned generic interface at a concrete type** —
`implements Iterator<User>`, `implements Iterable<int>` — fixing `T` at the declaration site, where
the argument is recorded on the class's signature and checked against every member body.

That is substitution of one concrete type into a known interface, and nothing more. A class may not
declare a type variable of its own, there is no inference, no variance, and no type-parameter scope
inside the class body. The door exists so a collection can be iterated
(`rule:iteration/two-interfaces`) without opening user-defined generics.
