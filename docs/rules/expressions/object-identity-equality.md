`$a == $b` on two class instances asks whether they are the same object. It never walks properties.

There is **no `__equals`, no `equals()` protocol and no `Equatable` interface**. A per-class equality
hook would make `==` mean something different in every file, and an ambient, undeclared property walk
has unbounded cost in the object graph's size with no way for a class to opt out.

Two named ways to ask the other question already exist:

- **`$a->compareTo($b) == 0`**, where the class implements `Comparable`. A class with a meaningful
  notion of "same value" almost always has a meaningful order too, and this gets both from one
  declaration.
- **`Core\Test::assertEqualsDeep`** in a test, which walks properties, arrays and shapes and produces
  a diff. It is a test member on purpose: a structural walk is a debugging affordance, not something a
  request path should reach for by accident.

A class needing content equality in production and wanting no order writes an ordinary named method.
That reads worse than `==` by exactly one call, and it is visible at the call site.
