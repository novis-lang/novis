`new $cls(...)` over a `class<T>` types its arguments against **`T`'s** constructor, exactly as
`new static(...)` types them against the current class's. That is the only signature the site can see,
and the value may be any implementor of `T`.

So the site is refused, at the `new`, when any implementor of `T` declares a constructor incompatible
with `T`'s — the same compatibility test the override check already makes, and the diagnostic names
that subclass. The mismatch is found at the site that can cause it, not when the wrong subclass
arrives at run time.

It is checked at the `new` and not at the class declaration on purpose. A subclass never instantiated
through a class reference is nobody's problem, and refusing it at its declaration would make an
unrelated file's `new` the reason a class cannot be written. The cost is one hierarchy-wide question
asked per dynamic `new` site; `new Dog()` pays nothing.
